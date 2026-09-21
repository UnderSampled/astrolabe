//! Bounded instrumentation of an owned x86 Linux process.
//!
//! Allocation single-steps an existing syscall instruction: original executable
//! bytes are never patched. Every other thread remains stopped. Any uncertainty
//! after tracing begins kills the exact pidfd-owned target before releasing stops.
use crate::linux::{Pause, Process};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    io::Read,
    time::{Duration, Instant},
};

const BUDGET: Duration = Duration::from_secs(2);
const MAX_SCRATCH: usize = 64 * 1024;
const MAX_THREADS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Abi {
    I386,
    X64,
}
#[derive(Clone, Copy)]
enum Call {
    Map(usize),
    Unmap(usize, usize),
}

/// Memory that is never borrowed from a game object. Mapping publication is
/// explicit; once exposed to callbacks it remains mapped for the process lifetime.
pub struct Scratch<'a> {
    process: &'a Process,
    address: usize,
    size: usize,
    callbacks: Vec<(usize, u32, u32)>,
    exposed: bool,
    closed: bool,
}
impl<'a> Scratch<'a> {
    pub fn allocate(process: &'a Process, size: usize) -> Result<Self> {
        ensure!(
            (1..=MAX_SCRATCH).contains(&size),
            "scratch size outside 1..65536"
        );
        let size = size.next_multiple_of(4096);
        let address = remote_call(process, Call::Map(size))? as usize;
        ensure!(
            address > 0
                && address
                    .checked_add(size)
                    .is_some_and(|end| end <= u32::MAX as usize),
            "scratch mapping is not a bounded 32-bit address"
        );
        Ok(Self {
            process,
            address,
            size,
            callbacks: vec![],
            exposed: false,
            closed: false,
        })
    }
    pub fn address(&self) -> usize {
        self.address
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn write(&self, offset: usize, bytes: &[u8]) -> Result<()> {
        ensure!(
            !self.exposed,
            "initialize scratch before callback publication"
        );
        ensure!(
            offset
                .checked_add(bytes.len())
                .is_some_and(|end| end <= self.size),
            "scratch write exceeds mapping"
        );
        self.process
            .write_instrumentation(self.address + offset, bytes)
    }
    /// Running reads are non-atomic; publication consistency belongs to the hook.
    pub fn read(&self, offset: usize, size: usize) -> Result<Vec<u8>> {
        ensure!(
            size > 0 && offset.checked_add(size).is_some_and(|end| end <= self.size),
            "scratch read exceeds mapping"
        );
        self.process.read_running(self.address + offset, size)
    }
    /// Replace one verified 32-bit callback while all process threads are stopped.
    /// The caller owns build/calling-convention validation of the generated code.
    pub fn install_callback(
        &mut self,
        slot: usize,
        expected: u32,
        code_offset: usize,
    ) -> Result<()> {
        ensure!(
            code_offset < self.size && slot.is_multiple_of(4),
            "invalid callback address"
        );
        ensure!(
            !self.callbacks.iter().any(|entry| entry.0 == slot),
            "callback already installed"
        );
        let _pause = self.process.pause(BUDGET)?;
        let actual = u32::from_le_bytes(self.process.read_running(slot, 4)?.as_slice().try_into()?);
        ensure!(
            actual == expected,
            "callback ownership changed before installation"
        );
        let replacement = (self.address + code_offset) as u32;
        // Establish lifetime ownership before a possibly partial remote write.
        self.exposed = true;
        self.callbacks.push((slot, expected, replacement));
        if let Err(error) = self
            .process
            .write_instrumentation(slot, &replacement.to_le_bytes())
        {
            let _ = self.process.terminate_instrumentation();
            return Err(error);
        }
        Ok(())
    }
    /// Restore callbacks only while their slots still contain our exact values.
    /// Exposed memory is retained: a stopped thread can have a return address into
    /// the mapping even when its current instruction pointer is outside it.
    pub fn close(mut self) -> Result<()> {
        if self.exposed {
            let _pause = self.process.pause(BUDGET)?;
            let result = (|| -> Result<()> {
                for &(slot, _, installed) in &self.callbacks {
                    ensure!(
                        self.process.read_running(slot, 4)? == installed.to_le_bytes(),
                        "callback ownership changed before restoration"
                    );
                }
                for &(slot, original, _) in &self.callbacks {
                    self.process
                        .write_instrumentation(slot, &original.to_le_bytes())?;
                }
                Ok(())
            })();
            if result.is_err() {
                let _ = self.process.terminate_instrumentation();
            }
            result?;
        } else {
            ensure!(
                remote_call(self.process, Call::Unmap(self.address, self.size))? == 0,
                "remote munmap failed"
            );
        }
        self.closed = true;
        Ok(())
    }
}
impl Drop for Scratch<'_> {
    fn drop(&mut self) {
        // Unattended callbacks must not continue, and Drop never guesses that
        // executing code is safe to unmap. Explicit close owns normal teardown.
        if !self.closed {
            let _ = self.process.terminate_instrumentation();
        }
    }
}

fn ptrace(request: libc::c_uint, tid: i32, address: usize, data: usize) -> Result<()> {
    // SAFETY: scalar ptrace parameters refer only to a seized owned thread.
    let result = unsafe {
        libc::ptrace(
            request,
            tid,
            address as *mut libc::c_void,
            data as *mut libc::c_void,
        )
    };
    ensure!(
        result != -1,
        "ptrace {request:#x} failed: {}",
        std::io::Error::last_os_error()
    );
    Ok(())
}
fn check_deadline(deadline: Instant) -> Result<()> {
    ensure!(
        Instant::now() < deadline,
        "instrumentation deadline expired"
    );
    Ok(())
}
fn wait_stop(tid: i32, deadline: Instant, interrupt: bool) -> Result<bool> {
    loop {
        check_deadline(deadline)?;
        let mut notice: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: notice is writable; WNOHANG bounds this poll. WNOWAIT retains
        // real exit status for Owner/Child, even if a signal terminates the target.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                tid as libc::id_t,
                &mut notice,
                libc::__WALL | libc::WNOHANG | libc::WNOWAIT | libc::WSTOPPED | libc::WEXITED,
            )
        };
        if result == 0 && unsafe { notice.si_pid() } == tid {
            ensure!(
                matches!(notice.si_code, libc::CLD_STOPPED | libc::CLD_TRAPPED),
                "instrumented thread exited"
            );
            // waitid reports the ptrace stop's signal plus event in si_status.
            let status = unsafe { notice.si_status() };
            let signal = status & 0xff;
            let event = status >> 8;
            if interrupt {
                ensure!(
                    event == libc::PTRACE_EVENT_STOP
                        && matches!(signal, libc::SIGSTOP | libc::SIGTRAP),
                    "unexpected signal/event while stopping instrumentation: {signal}/{event}"
                );
            } else {
                if event == libc::PTRACE_EVENT_STOP && signal == libc::SIGTRAP {
                    return Ok(false);
                }
                if event == 0 && signal == libc::SIGCONT {
                    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                    ptrace(
                        libc::PTRACE_GETSIGINFO,
                        tid,
                        0,
                        (&mut info as *mut libc::siginfo_t) as usize,
                    )?;
                    ensure!(
                        info.si_code == libc::SI_USER
                            && unsafe { info.si_pid() } == std::process::id() as i32,
                        "foreign continue signal during syscall"
                    );
                    return Ok(false);
                }
                ensure!(
                    event == 0 && signal == libc::SIGTRAP,
                    "unexpected syscall stop: {signal}/{event}"
                );
                let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                ptrace(
                    libc::PTRACE_GETSIGINFO,
                    tid,
                    0,
                    (&mut info as *mut libc::siginfo_t) as usize,
                )?;
                ensure!(
                    // Linux x86 reports TRAP_BRKPT for syscall single-step,
                    // unlike int3 (SI_KERNEL). See GDB nat/linux-ptrace.h.
                    matches!(info.si_code, libc::TRAP_TRACE | libc::TRAP_BRKPT),
                    "syscall stop was not a single-step trap: {}",
                    info.si_code
                );
            }
            return Ok(true);
        }
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EINTR) {
                return Err(error.into());
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn regs(tid: i32) -> Result<libc::user_regs_struct> {
    let mut value = unsafe { std::mem::zeroed() };
    ptrace(
        libc::PTRACE_GETREGS,
        tid,
        0,
        (&mut value as *mut libc::user_regs_struct) as usize,
    )?;
    Ok(value)
}
fn set_regs(tid: i32, value: &libc::user_regs_struct) -> Result<()> {
    ptrace(
        libc::PTRACE_SETREGS,
        tid,
        0,
        (value as *const libc::user_regs_struct) as usize,
    )
}

struct Trace<'a> {
    process: &'a Process,
    pause: Option<Pause<'a>>,
    tids: Vec<i32>,
    safe: bool,
}
impl<'a> Trace<'a> {
    fn stop(process: &'a Process, deadline: Instant) -> Result<Self> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let pause = process.pause(remaining)?;
        // Rejection is before tracing/mutation and preserves the caller's stop.
        ensure!(
            !pause.preserved_existing_stop(),
            "pre-existing group stop is unsupported for remote syscalls"
        );
        let mut trace = Self {
            process,
            pause: Some(pause),
            tids: vec![],
            safe: false,
        };
        let states = process.thread_states()?;
        ensure!(
            states.len() <= MAX_THREADS && states.values().all(|s| *s == 'T'),
            "thread group is not quiescent"
        );
        for &tid in states.keys() {
            check_deadline(deadline)?;
            ptrace(libc::PTRACE_SEIZE, tid, 0, libc::PTRACE_O_EXITKILL as usize)?;
            trace.tids.push(tid);
            fault("seize")?;
            ptrace(libc::PTRACE_INTERRUPT, tid, 0, 0)?;
            wait_stop(tid, deadline, true)?;
        }
        let stopped = process.thread_states()?;
        ensure!(
            stopped.keys().eq(states.keys()) && stopped.values().all(|s| *s == 't'),
            "thread set changed during instrumentation stop"
        );
        process.check()?;
        Ok(trace)
    }
    fn finish(mut self, deadline: Instant) -> Result<()> {
        check_deadline(deadline)?;
        fault("detach")?;
        while let Some(&tid) = self.tids.last() {
            ptrace(libc::PTRACE_DETACH, tid, 0, 0)?;
            self.tids.pop();
        }
        self.safe = true;
        Ok(())
    }
}
impl Drop for Trace<'_> {
    fn drop(&mut self) {
        if !self.safe {
            // Kill is submitted through the pinned pidfd before any detach or
            // SIGCONT. If even that fails, retain all stops rather than resume.
            if self.process.terminate_instrumentation().is_err() {
                if let Some(pause) = self.pause.take() {
                    std::mem::forget(pause);
                }
                return;
            }
            for &tid in &self.tids {
                let _ = ptrace(libc::PTRACE_DETACH, tid, 0, 0);
            }
            // Ptrace makes non-leader thread exits waitable by this tracer.
            // Drain those deaths so the owning Child can reap its group leader.
            let deadline = Instant::now() + Duration::from_millis(200);
            for &tid in &self.tids {
                if tid == self.process.identity().pid {
                    continue;
                }
                loop {
                    let mut status = 0;
                    // SAFETY: exact seized thread, nonblocking bounded wait.
                    let result =
                        unsafe { libc::waitpid(tid, &mut status, libc::__WALL | libc::WNOHANG) };
                    if result == -1
                        || (result == tid && (libc::WIFEXITED(status) || libc::WIFSIGNALED(status)))
                        || Instant::now() >= deadline
                    {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }
}

fn abi(process: &Process, registers: &libc::user_regs_struct) -> Result<Abi> {
    let mut header = [0; 20];
    fs::File::open(format!("/proc/{}/exe", process.identity().pid))?.read_exact(&mut header)?;
    ensure!(
        &header[..4] == b"\x7fELF" && header[5] == 1,
        "unsupported process executable"
    );
    let machine = u16::from_le_bytes([header[18], header[19]]);
    match (header[4], machine, registers.cs) {
        (1, 3, 0x23) | (2, 62, 0x23) => Ok(Abi::I386),
        (2, 62, 0x33) => Ok(Abi::X64),
        _ => bail!(
            "unsupported ELF/CPU execution mode: class={} machine={machine} cs={:#x}",
            header[4],
            registers.cs
        ),
    }
}
fn instruction(
    process: &Process,
    abi: Abi,
    deadline: Instant,
    excluded: Option<(usize, usize)>,
) -> Result<usize> {
    let opcode = match abi {
        Abi::I386 => [0xcd, 0x80],
        Abi::X64 => [0x0f, 0x05],
    };
    let maps = fs::read_to_string(format!("/proc/{}/maps", process.identity().pid))?;
    let mut scanned = 0usize;
    for line in maps.lines() {
        let mut fields = line.split_whitespace();
        let bounds = fields.next().context("mapping bounds missing")?;
        let permissions = fields.next().context("mapping permissions missing")?;
        if !permissions.starts_with('r') || permissions.as_bytes().get(2) != Some(&b'x') {
            continue;
        }
        let (lo, hi) = bounds.split_once('-').context("invalid mapping bounds")?;
        let mut start = usize::from_str_radix(lo, 16)?;
        let end = usize::from_str_radix(hi, 16)?;
        if excluded.is_some_and(|(lo, hi)| start < hi && lo < end) {
            continue;
        }
        if abi == Abi::I386 && end > u32::MAX as usize {
            continue;
        }
        while start < end {
            check_deadline(deadline)?;
            let size = (end - start).min(64 * 1024);
            scanned += size;
            ensure!(
                scanned <= 16 * 1024 * 1024,
                "syscall instruction scan exceeds 16 MiB"
            );
            let bytes = process.read_running(start, size)?;
            if let Some(offset) = bytes.windows(2).position(|pair| pair == opcode) {
                return Ok(start + offset);
            }
            if size < 2 {
                break;
            }
            start += size - 1;
        }
    }
    bail!("no existing executable syscall instruction for target mode")
}
fn remote_call(process: &Process, operation: Call) -> Result<u64> {
    let deadline = Instant::now() + BUDGET;
    let trace = Trace::stop(process, deadline)?;
    let tid = process.identity().pid;
    ensure!(
        trace.tids.contains(&tid),
        "process leader missing from stopped threads"
    );
    let saved = regs(tid)?;
    let abi = abi(process, &saved)?;
    let excluded = match operation {
        Call::Unmap(address, size) => Some((address, address + size)),
        Call::Map(_) => None,
    };
    let ip = instruction(process, abi, deadline, excluded)?;
    let (number, args) = match (abi, operation) {
        (Abi::I386, Call::Map(size)) => (192, [0, size as u64, 7, 0x22, u32::MAX as u64, 0]),
        (Abi::X64, Call::Map(size)) => (
            9,
            [
                0,
                size as u64,
                7,
                0x22 | libc::MAP_32BIT as u64,
                u64::MAX,
                0,
            ],
        ),
        (Abi::I386, Call::Unmap(address, size)) => (91, [address as u64, size as u64, 0, 0, 0, 0]),
        (Abi::X64, Call::Unmap(address, size)) => (11, [address as u64, size as u64, 0, 0, 0, 0]),
    };
    let mut call = saved;
    call.rip = ip as u64;
    call.rax = number;
    call.orig_rax = u64::MAX;
    match abi {
        Abi::I386 => {
            call.rbx = args[0];
            call.rcx = args[1];
            call.rdx = args[2];
            call.rsi = args[3];
            call.rdi = args[4];
            call.rbp = args[5];
        }
        Abi::X64 => {
            call.rdi = args[0];
            call.rsi = args[1];
            call.rdx = args[2];
            call.r10 = args[3];
            call.r8 = args[4];
            call.r9 = args[5];
        }
    }
    set_regs(tid, &call)?;
    fault("registers")?;
    // Release the group stop only after every thread is individually traced.
    // They remain in ptrace-stop; only this thread is allowed one instruction.
    process.resume_instrumentation()?;
    fault("continue")?;
    let mut stepped = false;
    for _ in 0..4 {
        ptrace(libc::PTRACE_SINGLESTEP, tid, 0, injected_signal())?;
        fault("step")?;
        if wait_stop(tid, injected_deadline(deadline), false)? {
            stepped = true;
            break;
        }
        ensure!(
            register_words(&regs(tid)?) == register_words(&call),
            "registers changed before syscall resume"
        );
    }
    ensure!(stepped, "too many group-stop continuation events");
    let done = regs(tid)?;
    fault("trap")?;
    ensure!(
        done.rip == ip as u64 + 2,
        "syscall stopped at unexpected instruction pointer"
    );
    fault("restore")?;
    set_regs(tid, &saved)?;
    let restored = regs(tid)?;
    // Verify every general register, including the syscall restart marker.
    ensure!(
        register_words(&restored) == register_words(&saved),
        "register restoration differs"
    );
    trace.finish(deadline)?;
    let value = match abi {
        Abi::I386 => done.rax as u32 as u64,
        Abi::X64 => done.rax,
    };
    let signed = match abi {
        Abi::I386 => value as u32 as i32 as i64,
        Abi::X64 => value as i64,
    };
    ensure!(
        !(-4095..0).contains(&signed),
        "remote syscall failed with errno {}",
        -signed
    );
    Ok(value)
}
fn register_words(value: &libc::user_regs_struct) -> [u64; 27] {
    [
        value.r15,
        value.r14,
        value.r13,
        value.r12,
        value.rbp,
        value.rbx,
        value.r11,
        value.r10,
        value.r9,
        value.r8,
        value.rax,
        value.rcx,
        value.rdx,
        value.rsi,
        value.rdi,
        value.orig_rax,
        value.rip,
        value.cs,
        value.eflags,
        value.rsp,
        value.ss,
        value.fs_base,
        value.gs_base,
        value.ds,
        value.es,
        value.fs,
        value.gs,
    ]
}

#[cfg(not(test))]
fn fault(_: &str) -> Result<()> {
    Ok(())
}
#[cfg(not(test))]
fn injected_signal() -> usize {
    0
}
#[cfg(test)]
fn injected_signal() -> usize {
    if FAULT.get() == Some("signal") {
        libc::SIGUSR1 as usize
    } else {
        0
    }
}
#[cfg(not(test))]
fn injected_deadline(deadline: Instant) -> Instant {
    deadline
}
#[cfg(test)]
fn injected_deadline(deadline: Instant) -> Instant {
    if FAULT.get() == Some("timeout") {
        Instant::now()
    } else {
        deadline
    }
}
#[cfg(test)]
thread_local! { static FAULT: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) }; }
#[cfg(test)]
fn fault(stage: &str) -> Result<()> {
    ensure!(
        FAULT.get() != Some(stage),
        "injected instrumentation failure at {stage}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux::Owner;
    use std::process::Command;

    #[test]
    fn mutation_failures_kill_exact_owned_child() -> Result<()> {
        for stage in [
            "seize",
            "registers",
            "continue",
            "step",
            "trap",
            "restore",
            "detach",
            "timeout",
            "signal",
        ] {
            let mut owner = Owner::new()?;
            let child = owner.spawn(Command::new("/usr/bin/sleep").arg("30"))?;
            std::thread::sleep(Duration::from_millis(20));
            FAULT.set(Some(stage));
            let started = Instant::now();
            let result = Scratch::allocate(&child, 4096);
            FAULT.set(None);
            ensure!(
                result.is_err(),
                "failure stage {stage} unexpectedly succeeded"
            );
            owner
                .close()
                .with_context(|| format!("cleanup after fault {stage}"))?;
            ensure!(
                started.elapsed() < Duration::from_secs(5),
                "fault {stage} exceeded budget"
            );
            ensure!(
                child.check().is_err(),
                "fault {stage} left owned child alive"
            );
        }
        let mut owner = Owner::new()?;
        let child = owner.spawn(Command::new("/usr/bin/sleep").arg("30"))?;
        std::thread::sleep(Duration::from_millis(20));
        let mut scratch = Scratch::allocate(&child, 4096)?;
        let slot = scratch.address();
        scratch.write(0, &0x12345678u32.to_le_bytes())?;
        scratch.write(64, &[0xc3])?;
        scratch.install_callback(slot, 0x12345678, 64)?;
        child.write_instrumentation(slot, &0x87654321u32.to_le_bytes())?;
        ensure!(
            scratch.close().is_err(),
            "foreign callback ownership accepted"
        );
        owner.close()?;
        ensure!(
            child.check().is_err(),
            "callback ownership failure left child alive"
        );
        Ok(())
    }
}
