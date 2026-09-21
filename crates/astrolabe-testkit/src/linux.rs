//! Linux ownership and read-only process access. One owner per controller process.
//!
//! The controller must launch every concurrent child through this owner. Orphans
//! are adopted with subreaping; pre-existing children and their descendants are
//! excluded. External test supervision remains responsible for killed owners.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::process::CommandExt,
    },
    process::{Child, Command},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

static OWNER_ACTIVE: AtomicBool = AtomicBool::new(false);
const MAX_READ: usize = 1024 * 1024;
const MAX_CAPTURE: usize = 16 * MAX_READ;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Identity {
    pub pid: i32,
    pub start_ticks: u64,
}
impl Identity {
    pub fn current() -> Result<Self> {
        Ok(stat(std::process::id() as i32)?.identity)
    }
}
#[derive(Clone)]
struct Stat {
    identity: Identity,
    parent: i32,
    state: char,
}
fn stat_path(path: impl AsRef<std::path::Path>, pid: i32) -> Result<Stat> {
    let text = fs::read_to_string(path)?;
    let (_, rest) = text.rsplit_once(')').context("invalid process stat")?;
    let fields: Vec<_> = rest.split_whitespace().collect();
    ensure!(fields.len() > 19, "truncated process stat");
    Ok(Stat {
        identity: Identity {
            pid,
            start_ticks: fields[19].parse()?,
        },
        parent: fields[1].parse()?,
        state: fields[0].chars().next().context("missing state")?,
    })
}
fn stat(pid: i32) -> Result<Stat> {
    stat_path(format!("/proc/{pid}/stat"), pid)
}
fn table() -> Result<BTreeMap<i32, Stat>> {
    let mut rows = BTreeMap::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse().ok()) {
            match stat(pid) {
                Ok(row) => {
                    rows.insert(pid, row);
                }
                Err(error)
                    if error.downcast_ref::<std::io::Error>().is_some_and(|e| {
                        matches!(
                            e.kind(),
                            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
                        )
                    }) => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(rows)
}

#[derive(Clone)]
pub struct Process {
    identity: Identity,
    descriptor: Arc<OwnedFd>,
}
impl Process {
    fn pin(identity: Identity) -> Result<Self> {
        // SAFETY: pidfd_open has no pointer arguments; OwnedFd takes the returned fd once.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, identity.pid, 0) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let descriptor = Arc::new(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        let process = Self {
            identity,
            descriptor,
        };
        // A very short command may already be a zombie. Pin its identity so the
        // owner can retrieve its real exit status; memory/signal calls still
        // reject dead processes through check().
        ensure!(
            stat(identity.pid)?.identity == identity,
            "stale process identity"
        );
        Ok(process)
    }
    pub fn identity(&self) -> Identity {
        self.identity
    }
    pub fn check(&self) -> Result<()> {
        let row = stat(self.identity.pid).context("owned process has exited")?;
        ensure!(
            row.identity == self.identity && !matches!(row.state, 'Z' | 'X'),
            "stale process identity"
        );
        Ok(())
    }
    fn signal(&self, signal: i32) -> Result<()> {
        self.check()?;
        // SAFETY: the pidfd pins this exact process, and a null siginfo requests a normal signal.
        let code = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.descriptor.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if code < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    pub(crate) fn thread_states(&self) -> Result<BTreeMap<i32, char>> {
        self.check()?;
        let mut states = BTreeMap::new();
        for entry in fs::read_dir(format!("/proc/{}/task", self.identity.pid))? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            let tid = entry
                .file_name()
                .to_str()
                .context("invalid thread name")?
                .parse()?;
            match stat_path(entry.path().join("stat"), tid) {
                Ok(row) => {
                    states.insert(tid, row.state);
                }
                Err(error)
                    if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) => {}
                Err(error) => return Err(error),
            }
        }
        self.check()?;
        ensure!(!states.is_empty(), "no process threads");
        Ok(states)
    }
    fn readable(&self, address: usize, size: usize) -> Result<()> {
        ensure!(size > 0 && size <= MAX_READ, "read size outside 1..1048576");
        let end = address.checked_add(size).context("address overflow")?;
        let mut position = address;
        for line in fs::read_to_string(format!("/proc/{}/maps", self.identity.pid))?.lines() {
            let mut fields = line.split_whitespace();
            let (lo, hi) = fields
                .next()
                .context("missing map bounds")?
                .split_once('-')
                .context("map bounds")?;
            let lo = usize::from_str_radix(lo, 16)?;
            let hi = usize::from_str_radix(hi, 16)?;
            if lo <= position && position < hi {
                ensure!(
                    fields.next().context("map permissions")?.starts_with('r'),
                    "unreadable page"
                );
                position = end.min(hi);
                if position == end {
                    return Ok(());
                }
            }
        }
        bail!("range contains unmapped pages")
    }
    /// Running reads are explicitly non-atomic. Gameplay assertions must use a
    /// separately verified publication boundary, never just this method.
    pub fn read_running(&self, address: usize, size: usize) -> Result<Vec<u8>> {
        self.check()?;
        self.readable(address, size)?;
        let mut bytes = vec![0; size];
        let local = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: size,
        };
        let remote = libc::iovec {
            iov_base: address as *mut libc::c_void,
            iov_len: size,
        };
        // SAFETY: local points at a live writable Vec of size bytes. The kernel
        // validates the remote range; it is never dereferenced in this process.
        let count = unsafe { libc::process_vm_readv(self.identity.pid, &local, 1, &remote, 1, 0) };
        if count < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        ensure!(
            count as usize == size,
            "partial memory transfer {count}/{size}"
        );
        self.check()?;
        Ok(bytes)
    }
    pub(crate) fn write_instrumentation(&self, address: usize, bytes: &[u8]) -> Result<()> {
        self.check()?;
        ensure!(
            !bytes.is_empty() && bytes.len() <= MAX_READ,
            "invalid write size"
        );
        let local = libc::iovec {
            iov_base: bytes.as_ptr() as *mut libc::c_void,
            iov_len: bytes.len(),
        };
        let remote = libc::iovec {
            iov_base: address as *mut libc::c_void,
            iov_len: bytes.len(),
        };
        // SAFETY: local points at initialized source bytes. The kernel checks
        // the remote mapping; the instrumentation owner bounds destinations.
        let count = unsafe { libc::process_vm_writev(self.identity.pid, &local, 1, &remote, 1, 0) };
        if count < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        ensure!(
            count as usize == bytes.len(),
            "partial instrumentation write"
        );
        self.check()
    }
    /// Fail closed when instrumentation restoration cannot be established.
    /// The pidfd pins the exact owned process even if its numeric PID is reused.
    pub(crate) fn terminate_instrumentation(&self) -> Result<()> {
        // SAFETY: the pidfd pins the owned identity and SIGKILL has no user
        // pointer payload. ESRCH means that same process already exited.
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.descriptor.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error.into());
            }
        }
        Ok(())
    }
    pub(crate) fn resume_instrumentation(&self) -> Result<()> {
        self.signal(libc::SIGCONT)
    }
    pub fn pause(&self, timeout: Duration) -> Result<Pause<'_>> {
        ensure!(
            !timeout.is_zero() && timeout <= Duration::from_secs(2),
            "pause deadline outside 0..2 seconds"
        );
        let before = self.thread_states()?;
        ensure!(
            !before.values().any(|s| *s == 't'),
            "debugger-owned thread stop is unsupported"
        );
        let already_stopped = before.values().all(|s| *s == 'T');
        ensure!(
            already_stopped || !before.values().any(|s| *s == 'T'),
            "mixed pre-existing stop state"
        );
        // Establish restoration before sending SIGSTOP or waiting for all threads.
        let mut guard = Pause {
            process: self,
            resume: !already_stopped,
            threads: before.len(),
        };
        if !already_stopped {
            self.signal(libc::SIGSTOP)?;
        }
        let deadline = Instant::now() + timeout;
        loop {
            let states = self.thread_states()?;
            if states.values().all(|s| *s == 'T') && self.thread_states()? == states {
                guard.threads = states.len();
                return Ok(guard);
            }
            ensure!(
                Instant::now() < deadline,
                "could not stop every owned process thread"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }
}

pub struct Pause<'a> {
    process: &'a Process,
    resume: bool,
    threads: usize,
}
impl Pause<'_> {
    pub fn preserved_existing_stop(&self) -> bool {
        !self.resume
    }
    pub fn stopped_threads(&self) -> usize {
        self.threads
    }
    pub fn capture(&self) -> Result<Capture<'_>> {
        // SAFETY: sysconf queries the host page size and takes no pointers.
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        ensure!(
            page_size > 0 && (page_size as usize).is_power_of_two(),
            "invalid host page size"
        );
        Ok(Capture {
            guard: self,
            ranges: BTreeMap::new(),
            pages: BTreeMap::new(),
            page_size: page_size as usize,
            bytes: 0,
            started: Instant::now(),
        })
    }
}
impl Drop for Pause<'_> {
    fn drop(&mut self) {
        if self.resume {
            let _ = self.process.signal(libc::SIGCONT);
        }
    }
}

pub struct Capture<'a> {
    guard: &'a Pause<'a>,
    ranges: BTreeMap<usize, Vec<u8>>,
    pages: BTreeMap<usize, Vec<u8>>,
    page_size: usize,
    bytes: usize,
    started: Instant,
}
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub process: Identity,
    pub consistency: Consistency,
    pub capture_micros: u128,
    pub stopped_threads: usize,
    pub page_reads: usize,
    pub copied_source_bytes: usize,
    ranges: BTreeMap<usize, Vec<u8>>,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum Consistency {
    StoppedBytes,
}
impl Capture<'_> {
    pub fn read(&mut self, address: usize, size: usize) -> Result<Vec<u8>> {
        if let Ok(bytes) = range(&self.ranges, address, size) {
            return Ok(bytes.to_vec());
        }
        ensure!((1..=MAX_READ).contains(&size), "invalid capture range size");
        ensure!(
            self.ranges.len() < 16384,
            "capture exceeds bounded record count"
        );
        let end = address
            .checked_add(size)
            .context("capture range overflow")?;
        self.guard.process.check()?;
        let mut bytes = Vec::with_capacity(size);
        let mut position = address;
        while position < end {
            let page = position & !(self.page_size - 1);
            let page_end = page
                .checked_add(self.page_size)
                .context("page range overflow")?;
            if !self.pages.contains_key(&page) {
                ensure!(
                    self.bytes + (self.pages.len() + 1) * self.page_size + size <= MAX_CAPTURE,
                    "capture exceeds 16 MiB of retained records and page cache"
                );
                let data = self.guard.process.read_running(page, self.page_size)?;
                self.pages.insert(page, data);
            }
            let stop = end.min(page_end);
            bytes.extend_from_slice(&self.pages[&page][position - page..stop - page]);
            position = stop;
        }
        ensure!(
            self.bytes + self.pages.len() * self.page_size + size <= MAX_CAPTURE,
            "capture exceeds 16 MiB of retained records and page cache"
        );
        self.bytes += size;
        self.ranges.insert(address, bytes.clone());
        Ok(bytes)
    }
    pub fn finish(self) -> Result<Snapshot> {
        ensure!(
            self.guard
                .process
                .thread_states()?
                .values()
                .all(|s| *s == 'T'),
            "capture lost stopped state"
        );
        Ok(Snapshot {
            process: self.guard.process.identity,
            consistency: Consistency::StoppedBytes,
            capture_micros: self.started.elapsed().as_micros(),
            stopped_threads: self.guard.threads,
            page_reads: self.pages.len(),
            copied_source_bytes: self.pages.len() * self.page_size,
            // Page prefetch is transient. Preserve only explicitly requested
            // records so neighboring bytes cannot be fetched after resume.
            ranges: self.ranges,
        })
    }
}
fn range(ranges: &BTreeMap<usize, Vec<u8>>, address: usize, size: usize) -> Result<&[u8]> {
    ensure!(size > 0 && size <= MAX_READ, "invalid capture range size");
    let end = address
        .checked_add(size)
        .context("capture range overflow")?;
    for (&start, bytes) in ranges.range(..=address).rev() {
        if start
            .checked_add(bytes.len())
            .is_some_and(|stop| end <= stop)
        {
            return Ok(&bytes[address - start..end - start]);
        }
    }
    bail!("range was not part of the stopped-byte capture")
}
impl Snapshot {
    /// Missing pages stay missing after resume; this object has no process reader.
    pub fn bytes(&self, address: usize, size: usize) -> Result<&[u8]> {
        range(&self.ranges, address, size)
    }
    pub fn require_completed_update(&self) -> Result<()> {
        bail!("stopped-byte capture has no completed-update publication evidence")
    }
}

pub struct Owner {
    baseline: BTreeMap<i32, Identity>,
    owned: BTreeMap<Identity, Process>,
    children: Vec<Child>,
    closed: bool,
}
impl Owner {
    pub fn new() -> Result<Self> {
        ensure!(
            !OWNER_ACTIVE.swap(true, Ordering::SeqCst),
            "one process owner per controller is permitted"
        );
        let result = (|| {
            // SAFETY: PR_SET_CHILD_SUBREAPER accepts an integer flag and no pointers.
            ensure!(
                unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } == 0,
                "cannot enable child subreaping"
            );
            Ok(Self {
                baseline: table()?
                    .into_iter()
                    .map(|(pid, s)| (pid, s.identity))
                    .collect(),
                owned: BTreeMap::new(),
                children: vec![],
                closed: false,
            })
        })();
        if result.is_err() {
            OWNER_ACTIVE.store(false, Ordering::SeqCst);
        }
        result
    }
    pub fn spawn(&mut self, command: &mut Command) -> Result<Process> {
        ensure!(!self.closed, "process owner is closed");
        // SAFETY: setsid is async-signal-safe and only changes the new child.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        let child = command.spawn()?;
        let pid = child.id() as i32;
        self.children.push(child);
        let identity = stat(pid)?.identity;
        let process = Process::pin(identity)?;
        self.owned.insert(identity, process.clone());
        Ok(process)
    }
    pub fn processes(&mut self) -> Result<Vec<Process>> {
        self.refresh()?;
        Ok(self
            .owned
            .values()
            .filter(|p| p.check().is_ok())
            .cloned()
            .collect())
    }
    pub fn try_wait(&mut self, process: &Process) -> Result<Option<std::process::ExitStatus>> {
        ensure!(
            self.owned.contains_key(&process.identity),
            "process belongs to another owner"
        );
        self.children
            .iter_mut()
            .find(|child| child.id() as i32 == process.identity.pid)
            .context("only directly launched children have an owned exit status")?
            .try_wait()
            .map_err(Into::into)
    }
    fn refresh(&mut self) -> Result<()> {
        for child in &mut self.children {
            let _ = child.try_wait()?;
        }
        let table = table()?;
        let mut members: BTreeMap<i32, Identity> = self
            .owned
            .keys()
            .filter(|id| table.get(&id.pid).is_some_and(|s| s.identity == **id))
            .map(|id| (id.pid, *id))
            .collect();
        for (&pid, row) in &table {
            if row.parent == std::process::id() as i32
                && self.baseline.get(&pid) != Some(&row.identity)
            {
                members.insert(pid, row.identity);
            }
        }
        loop {
            let old = members.len();
            for (&pid, row) in &table {
                if members.contains_key(&row.parent) {
                    members.insert(pid, row.identity);
                }
            }
            if members.len() == old {
                break;
            }
        }
        for identity in members.values() {
            if self.owned.contains_key(identity) {
                continue;
            }
            if let Ok(process) = Process::pin(*identity) {
                self.owned.insert(*identity, process);
            } else if table[&identity.pid].state == 'Z' {
                // SAFETY: WNOHANG never blocks; only reap our adopted, exact child.
                unsafe {
                    libc::waitpid(identity.pid, std::ptr::null_mut(), libc::WNOHANG);
                }
            }
        }
        Ok(())
    }
    pub fn close(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        let started = Instant::now();
        loop {
            self.refresh()?;
            for process in self.owned.values() {
                if process.check().is_ok() {
                    let _ = process.signal(if started.elapsed() >= Duration::from_secs(2) {
                        libc::SIGKILL
                    } else {
                        libc::SIGTERM
                    });
                    let _ = process.signal(libc::SIGCONT);
                } else if !self
                    .children
                    .iter()
                    .any(|c| c.id() as i32 == process.identity.pid)
                    && stat(process.identity.pid)
                        .is_ok_and(|s| s.identity == process.identity && s.state == 'Z')
                {
                    // SAFETY: only our exact adopted children can be reaped.
                    unsafe {
                        libc::waitpid(process.identity.pid, std::ptr::null_mut(), libc::WNOHANG);
                    }
                }
            }
            if self.owned.values().all(|p| {
                stat(p.identity.pid).is_err()
                    || stat(p.identity.pid).is_ok_and(|s| s.identity != p.identity)
            }) {
                self.closed = true;
                return Ok(());
            }
            ensure!(
                started.elapsed() < Duration::from_secs(4),
                "owned process cleanup deadline exceeded"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.close();
        OWNER_ACTIVE.store(false, Ordering::SeqCst);
    }
}

impl astrolabe::openspace::level::live::ReadMemory for Capture<'_> {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        Capture::read(self, address as usize, size)
    }
}

impl astrolabe::openspace::level::live::ReadMemory for Snapshot {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        self.bytes(address as usize, size).map(<[u8]>::to_vec)
    }
}
