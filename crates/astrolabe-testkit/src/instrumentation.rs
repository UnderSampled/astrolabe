//! Dedicated memory for bounded native probes. Allocation is performed in the
//! owned child with a one-thread ptrace syscall stop, before callbacks are
//! installed. Game fields are never used as scratch storage.
use crate::linux::Process;
use anyhow::{Context, Result, bail, ensure};

const MAX_SCRATCH: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Abi {
    I386,
    X86_64,
}

/// A private anonymous mapping in an owned process. Explicit close is required
/// for successful runs; Drop is only a best-effort failure fallback.
pub struct Scratch<'a> {
    process: &'a Process,
    address: usize,
    size: usize,
    closed: bool,
}

impl<'a> Scratch<'a> {
    pub fn allocate(process: &'a Process, size: usize) -> Result<Self> {
        ensure!(
            (1..=MAX_SCRATCH).contains(&size),
            "scratch size outside 1..65536"
        );
        let size = size.next_multiple_of(4096);
        let abi = abi(process)?;
        let address = remote_syscall(
            process,
            abi,
            match abi {
                Abi::I386 => 192,
                Abi::X86_64 => libc::SYS_mmap as u64,
            },
            [
                0,
                size as u64,
                (libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC) as u64,
                (libc::MAP_PRIVATE | libc::MAP_ANONYMOUS) as u64,
                u64::MAX,
                0,
            ],
        )? as usize;
        ensure!(
            address != 0 && address != usize::MAX,
            "remote mmap returned invalid address"
        );
        if abi == Abi::I386 {
            ensure!(
                address <= u32::MAX as usize,
                "32-bit mmap returned wide address"
            );
        }
        let scratch = Self {
            process,
            address,
            size,
            closed: false,
        };
        scratch.write(0, &vec![0; size])?;
        Ok(scratch)
    }
    pub fn address(&self) -> usize {
        self.address
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn write(&self, offset: usize, bytes: &[u8]) -> Result<()> {
        let end = offset
            .checked_add(bytes.len())
            .context("scratch write overflow")?;
        ensure!(end <= self.size, "scratch write exceeds mapping");
        self.process
            .write_instrumentation(self.address + offset, bytes)
    }
    pub fn read(&self, offset: usize, size: usize) -> Result<Vec<u8>> {
        let end = offset.checked_add(size).context("scratch read overflow")?;
        ensure!(size > 0 && end <= self.size, "scratch read exceeds mapping");
        self.process.read_running(self.address + offset, size)
    }
    pub fn close(mut self) -> Result<()> {
        self.unmap()?;
        self.closed = true;
        Ok(())
    }
    fn unmap(&self) -> Result<()> {
        let abi = abi(self.process)?;
        let result = remote_syscall(
            self.process,
            abi,
            match abi {
                Abi::I386 => 91,
                Abi::X86_64 => libc::SYS_munmap as u64,
            },
            [self.address as u64, self.size as u64, 0, 0, 0, 0],
        )?;
        ensure!(result == 0, "remote munmap returned {result:#x}");
        Ok(())
    }
}
impl Drop for Scratch<'_> {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.unmap();
        }
    }
}

fn abi(process: &Process) -> Result<Abi> {
    let bytes = std::fs::read(format!("/proc/{}/exe", process.identity().pid))?;
    ensure!(
        bytes.get(..4) == Some(b"\x7fELF"),
        "owned process is not ELF"
    );
    match bytes.get(4) {
        Some(1) => Ok(Abi::I386),
        Some(2) => Ok(Abi::X86_64),
        _ => bail!("unsupported ELF class"),
    }
}

fn ptrace(request: libc::c_uint, pid: i32, address: usize, data: usize) -> Result<libc::c_long> {
    // SAFETY: ptrace arguments are scalar values for the exact owned PID.
    let value = unsafe {
        libc::ptrace(
            request,
            pid,
            address as *mut libc::c_void,
            data as *mut libc::c_void,
        )
    };
    if value == -1 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(value)
}
fn wait_stopped(pid: i32) -> Result<()> {
    let mut status = 0;
    // SAFETY: status is writable and pid is the attached owned thread.
    ensure!(
        unsafe { libc::waitpid(pid, &mut status, libc::__WALL) } == pid,
        "ptrace wait failed"
    );
    ensure!(libc::WIFSTOPPED(status), "owned thread did not ptrace-stop");
    Ok(())
}

fn remote_syscall(process: &Process, abi: Abi, number: u64, args: [u64; 6]) -> Result<u64> {
    process.check()?;
    let pid = process.identity().pid;
    ptrace(libc::PTRACE_ATTACH, pid, 0, 0)?;
    wait_stopped(pid)?;
    let result = (|| {
        // x86-64 kernels use this register layout for both native and compat tasks.
        let mut saved: libc::user_regs_struct = unsafe { std::mem::zeroed() };
        // SAFETY: saved is a correctly sized writable register structure.
        let got = unsafe {
            libc::ptrace(
                libc::PTRACE_GETREGS,
                pid,
                std::ptr::null_mut::<libc::c_void>(),
                &mut saved,
            )
        };
        ensure!(
            got == 0,
            "PTRACE_GETREGS failed: {}",
            std::io::Error::last_os_error()
        );
        let ip = saved.rip as usize;
        let original = ptrace(libc::PTRACE_PEEKTEXT, pid, ip, 0)? as libc::c_long;
        let mut patched = original.to_ne_bytes();
        match abi {
            Abi::I386 => patched[..3].copy_from_slice(&[0xcd, 0x80, 0xcc]),
            Abi::X86_64 => patched[..3].copy_from_slice(&[0x0f, 0x05, 0xcc]),
        }
        ptrace(
            libc::PTRACE_POKETEXT,
            pid,
            ip,
            libc::c_long::from_ne_bytes(patched) as usize,
        )?;
        let mut call = saved;
        call.rax = number;
        match abi {
            Abi::I386 => {
                call.rbx = args[0];
                call.rcx = args[1];
                call.rdx = args[2];
                call.rsi = args[3];
                call.rdi = args[4];
                call.rbp = args[5];
            }
            Abi::X86_64 => {
                call.rdi = args[0];
                call.rsi = args[1];
                call.rdx = args[2];
                call.r10 = args[3];
                call.r8 = args[4];
                call.r9 = args[5];
            }
        }
        // SAFETY: call is a correctly sized register structure for this stopped task.
        ensure!(
            unsafe {
                libc::ptrace(
                    libc::PTRACE_SETREGS,
                    pid,
                    std::ptr::null_mut::<libc::c_void>(),
                    &call,
                )
            } == 0,
            "PTRACE_SETREGS failed"
        );
        ptrace(libc::PTRACE_CONT, pid, 0, 0)?;
        wait_stopped(pid)?;
        let mut done: libc::user_regs_struct = unsafe { std::mem::zeroed() };
        ensure!(
            unsafe {
                libc::ptrace(
                    libc::PTRACE_GETREGS,
                    pid,
                    std::ptr::null_mut::<libc::c_void>(),
                    &mut done,
                )
            } == 0,
            "PTRACE_GETREGS result failed"
        );
        ptrace(libc::PTRACE_POKETEXT, pid, ip, original as usize)?;
        ensure!(
            unsafe {
                libc::ptrace(
                    libc::PTRACE_SETREGS,
                    pid,
                    std::ptr::null_mut::<libc::c_void>(),
                    &saved,
                )
            } == 0,
            "cannot restore registers"
        );
        let value = done.rax;
        let signed = match abi {
            Abi::I386 => (value as u32 as i32) as i64,
            Abi::X86_64 => value as i64,
        };
        ensure!(
            !(-4095..0).contains(&signed),
            "remote syscall failed with errno {}",
            -signed
        );
        Ok(match abi {
            Abi::I386 => value as u32 as u64,
            Abi::X86_64 => value,
        })
    })();
    // DETACH resumes the exact thread even on an error; process ownership still
    // provides outer cleanup if restoration itself failed.
    let detached = ptrace(libc::PTRACE_DETACH, pid, 0, 0);
    let value = result?;
    detached?;
    Ok(value)
}
