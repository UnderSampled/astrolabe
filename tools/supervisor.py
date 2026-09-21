#!/usr/bin/env python3
"""Bounded Linux subprocess ownership shared by selected tests and engine jobs."""
import ctypes
from dataclasses import dataclass
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time



class Interrupted(Exception):
    def __init__(self, signum):
        self.signum = signum


def interrupted(signum, _frame):
    raise Interrupted(signum)


def enable_subreaper():
    if sys.platform != "linux" or not Path("/proc/self/stat").exists():
        raise RuntimeError("this suite supervisor requires Linux /proc for complete child-session cleanup")
    # Adopt orphaned test workers so their termination can be reaped here.
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise OSError(ctypes.get_errno(), "cannot enable test-child supervision")


def process_table():
    result = {}
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        try:
            fields = (path / "stat").read_text(errors="replace").rsplit(")", 1)[1].split()
            # parent, group, session, state, start identity (kernel clock ticks).
            result[int(path.name)] = (int(fields[1]), int(fields[2]), int(fields[3]),
                                      fields[0], int(fields[19]))
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            pass
    return result


class OwnedTree:
    """One sequential job and its adopted descendants, including setsid workers.

    The supervising process must launch no concurrent unrelated children while a
    job is active. Pre-existing children and their descendants are excluded.
    """
    def __init__(self):
        self.root = None
        self.known = {}
        self.baseline = {pid: row[4] for pid, row in process_table().items()}

    def members(self):
        table = process_table()
        live = {pid for pid, stamp in self.known.items()
                if pid in table and table[pid][4] == stamp}
        if self.root in table and self.known.get(self.root, table[self.root][4]) == table[self.root][4]:
            live.add(self.root)
        # Double-forked / private-session workers are adopted by the subreaper.
        live.update(pid for pid, row in table.items()
                    if row[0] == os.getpid() and self.baseline.get(pid) != row[4])
        while True:
            children = {pid for pid, row in table.items() if row[0] in live}
            if children <= live:
                break
            live.update(children)
        self.known.update({pid: table[pid][4] for pid in live})
        return {pid: (table[pid][1], table[pid][3]) for pid in live}

    def signal(self, pid, signum):
        # pidfd pins the process across the final identity check and signal.
        try:
            fd = os.pidfd_open(pid)
        except ProcessLookupError:
            return
        try:
            row = process_table().get(pid)
            if row and row[4] == self.known.get(pid):
                signal.pidfd_send_signal(fd, signum)
        except ProcessLookupError:
            pass
        finally:
            os.close(fd)


def reap_adopted(ownership, root):
    for pid in ownership.known:
        if pid == root:
            continue  # Popen owns the main child's exit status.
        try:
            os.waitpid(pid, os.WNOHANG)
        except (ChildProcessError, ProcessLookupError):
            pass


def terminate_session(child, deadline, ownership):
    """Terminate only owned process identities, including escaped sessions."""
    sent_term = False
    kill_at = time.monotonic() + 2.0
    while True:
        child.poll()  # Reap Cargo before waitpid(-1) can consume its status.
        if child.returncode is not None:
            reap_adopted(ownership, child.pid)
        members = ownership.members()
        active = {pid: value for pid, value in members.items() if value[1] != "Z"}
        if not active:
            if child.returncode is None:
                try:
                    child.wait(timeout=max(0.01, min(0.2, deadline - time.monotonic())))
                except subprocess.TimeoutExpired:
                    pass
            if child.returncode is not None:
                reap_adopted(ownership, child.pid)
            return []
        now = time.monotonic()
        if now >= deadline:
            return sorted(active)
        if not sent_term or now >= kill_at:
            sig = signal.SIGKILL if now >= kill_at else signal.SIGTERM
            for pid in active:
                if pid == os.getpid():
                    raise RuntimeError("refusing to signal the supervisor")
                ownership.signal(pid, sig)
            sent_term = True
        time.sleep(min(0.05, max(0.0, deadline - time.monotonic())))


@dataclass
class Result:
    exit_code: int
    child_exit_code: int | None
    elapsed_seconds: float
    timed_out: bool
    leaked_children: bool
    remaining_processes: list
    execution_seconds: float
    cleanup_seconds: float
    error: str | None


def supervise(command, *, cwd, environment, log_path, started, timeout, echo=True,
              max_output_bytes=64 * 1024 * 1024):
    """One invocation, one deadline. Used by main and real process-tree checks."""
    enable_subreaper()
    deadline = started + timeout
    cleanup_deadline = deadline + 5.0
    timed_out = False
    leaked = False
    stop_signal = None
    remaining = []
    execution_error = None
    output_bytes = 0
    cleanup_started = None
    child = None
    ownership = OwnedTree()
    with open(log_path, "xb", buffering=0) as log, selectors.DefaultSelector() as selector:
        def output(data):
            nonlocal output_bytes
            output_bytes += len(data)
            if output_bytes > max_output_bytes:
                raise RuntimeError(f'job diagnostic output exceeded {max_output_bytes} bytes')
            log.write(data)
            if echo:
                sys.stdout.buffer.write(data)
                sys.stdout.buffer.flush()

        def drain(wait):
            for key, _mask in selector.select(wait):
                data = os.read(key.fd, 65536)
                if data:
                    output(data)
                else:
                    selector.unregister(key.fd)

        try:
            child = subprocess.Popen(command, cwd=cwd, env=environment,
                                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                     start_new_session=True, bufsize=0)
            ownership.root = child.pid
            os.set_blocking(child.stdout.fileno(), False)
            selector.register(child.stdout, selectors.EVENT_READ)
            next_scan = 0.0
            while child.poll() is None:
                left = deadline - time.monotonic()
                if left <= 0:
                    timed_out = True
                    break
                if time.monotonic() >= next_scan:
                    ownership.members()
                    next_scan = time.monotonic() + 0.05
                drain(min(0.1, left))
            if time.monotonic() >= deadline:
                timed_out = True
            if not timed_out:
                # A passing Cargo process must not leave a renderer/worker alive.
                until = min(deadline, time.monotonic() + 0.15)
                while time.monotonic() < until:
                    reap_adopted(ownership, child.pid)
                    members = ownership.members()
                    if not any(state != "Z" for _group, state in members.values()):
                        break
                    drain(0.02)
                leaked = any(state != "Z" for _group, state in ownership.members().values())
        except Interrupted as exc:
            stop_signal = exc.signum
        except (OSError, RuntimeError) as exc:
            execution_error = str(exc)
        finally:
            cleanup_started = time.monotonic()
            if child is not None:
                # A second interrupt must not bypass child cleanup.
                previous = {sig: signal.signal(sig, signal.SIG_IGN) for sig in (signal.SIGINT, signal.SIGTERM)}
                try:
                    remaining = terminate_session(child, cleanup_deadline, ownership)
                    while output_bytes <= max_output_bytes and selector.get_map() and selector.select(0):
                        drain(0)
                    child.stdout.close()
                finally:
                    for sig, handler in previous.items():
                        signal.signal(sig, handler)
    child_code = child.returncode if child is not None else None
    code = 124 if timed_out else (128 + stop_signal if stop_signal else
           (125 if execution_error or leaked or remaining else (child_code if child_code is not None else 125)))
    if code < 0:
        code = 128 - code
    finished = time.monotonic()
    return Result(code, child_code, finished - started, timed_out, leaked, remaining,
                  cleanup_started - started, finished - cleanup_started, execution_error)


def remove_temporary_tree(path, deadline):
    """Bound filesystem cleanup too; only remove this invocation's owned temp tree."""
    child = subprocess.Popen([sys.executable, "-B", "-c",
                              "import shutil,sys; shutil.rmtree(sys.argv[1])", str(path)],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        return child.wait(timeout=max(0.01, deadline - time.monotonic())) == 0
    except subprocess.TimeoutExpired:
        child.kill()
        try:
            child.wait(timeout=0.2)
        except subprocess.TimeoutExpired:
            pass
        return False
