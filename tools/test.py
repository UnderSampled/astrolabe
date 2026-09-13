#!/usr/bin/env python3
"""Run one complete Cargo suite with a shared deadline and descendant cleanup."""
import argparse
import ctypes
from dataclasses import dataclass
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import resource
import selectors
import signal
import subprocess
import sys
import tempfile
import time

MAX_TEST_SECONDS = 570.0
MAX_TOTAL_SECONDS = 590.0  # Leave ten seconds before the user's absolute limit.


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


def session_members(session):
    result = {}
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        try:
            # comm may contain spaces or parentheses; fields follow its last ')'.
            fields = (path / "stat").read_text(errors="replace").rsplit(")", 1)[1].split()
            if int(fields[3]) == session:
                result[int(path.name)] = (int(fields[2]), fields[0])
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            pass
    return result


def reap_adopted():
    while True:
        try:
            pid, _status = os.waitpid(-1, os.WNOHANG)
            if pid == 0:
                break
        except ChildProcessError:
            break


def terminate_session(child, deadline):
    """Kill every process group in Cargo's session, including private test groups."""
    sent_term = False
    kill_at = time.monotonic() + 2.0
    while True:
        child.poll()  # Reap Cargo before waitpid(-1) can consume its status.
        if child.returncode is not None:
            reap_adopted()
        members = session_members(child.pid)
        active = {pid: value for pid, value in members.items() if value[1] != "Z"}
        if not active:
            if child.returncode is None:
                try:
                    child.wait(timeout=max(0.01, min(0.2, deadline - time.monotonic())))
                except subprocess.TimeoutExpired:
                    pass
            if child.returncode is not None:
                reap_adopted()
            return []
        now = time.monotonic()
        if now >= deadline:
            return sorted(active)
        if not sent_term or now >= kill_at:
            sig = signal.SIGKILL if now >= kill_at else signal.SIGTERM
            for group in {value[0] for value in active.values()}:
                if group == os.getpgrp():
                    raise RuntimeError("refusing to signal the supervisor's process group")
                try:
                    os.killpg(group, sig)
                except ProcessLookupError:
                    pass
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


def supervise(command, *, cwd, environment, log_path, started, timeout, echo=True):
    """One invocation, one deadline. Used by main and real process-tree checks."""
    enable_subreaper()
    deadline = started + timeout
    cleanup_deadline = started + min(MAX_TOTAL_SECONDS, timeout + 15.0)
    timed_out = False
    leaked = False
    stop_signal = None
    remaining = []
    child = None
    with open(log_path, "xb", buffering=0) as log, selectors.DefaultSelector() as selector:
        def output(data):
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
            os.set_blocking(child.stdout.fileno(), False)
            selector.register(child.stdout, selectors.EVENT_READ)
            while child.poll() is None:
                left = deadline - time.monotonic()
                if left <= 0:
                    timed_out = True
                    break
                drain(min(0.1, left))
            if time.monotonic() >= deadline:
                timed_out = True
            if not timed_out:
                # A passing Cargo process must not leave a renderer/worker alive.
                until = min(deadline, time.monotonic() + 0.15)
                while time.monotonic() < until:
                    reap_adopted()
                    members = session_members(child.pid)
                    if not any(state != "Z" for _group, state in members.values()):
                        break
                    drain(0.02)
                leaked = any(state != "Z" for _group, state in session_members(child.pid).values())
        except Interrupted as exc:
            stop_signal = exc.signum
        finally:
            if child is not None:
                # A second interrupt must not bypass child cleanup.
                previous = {sig: signal.signal(sig, signal.SIG_IGN) for sig in (signal.SIGINT, signal.SIGTERM)}
                try:
                    remaining = terminate_session(child, cleanup_deadline)
                    while selector.get_map() and selector.select(0):
                        drain(0)
                    child.stdout.close()
                finally:
                    for sig, handler in previous.items():
                        signal.signal(sig, handler)
    child_code = child.returncode if child is not None else None
    code = 124 if timed_out else (128 + stop_signal if stop_signal else
           (125 if leaked or remaining else (child_code if child_code is not None else 125)))
    if code < 0:
        code = 128 - code
    return Result(code, child_code, time.monotonic() - started, timed_out, leaked, remaining)


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


def main():
    started = time.monotonic()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--local", action="store_true", help="include every ignored local game/Godot test")
    parser.add_argument("--game", type=Path, help="game/disc root; sets ASTROLABE_GAME_DIR")
    parser.add_argument("--godot", help="Godot executable; sets ASTROLABE_GODOT")
    parser.add_argument("--offline", action="store_true", help="require already cached Cargo dependencies")
    parser.add_argument("--timeout", type=float, default=MAX_TEST_SECONDS,
                        help="whole build/test deadline in seconds, at most 570 (default: 570)")
    parser.add_argument("--log", type=Path, help="new log file (default: timestamped TestResults log)")
    args = parser.parse_args()
    if not 0 < args.timeout <= MAX_TEST_SECONDS:
        parser.error("--timeout must be greater than zero and at most 570 seconds")
    if args.game is not None and not args.game.is_dir():
        parser.error("--game must name an existing game/disc root directory")
    root = Path(__file__).resolve().parent.parent
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    log_path = (args.log or root / "TestResults" / f"{stamp}-{os.getpid()}.log").resolve()
    report_path = log_path.with_suffix(log_path.suffix + ".json")
    if log_path.exists() or report_path.exists():
        parser.error("log/report already exists; choose a new --log path")
    log_path.parent.mkdir(parents=True, exist_ok=True)
    command = ["cargo", "test", "--release", "--locked", "--no-fail-fast"]
    if args.offline:
        command.append("--offline")
    command += ["--", "--nocapture"]
    if args.local:
        command.append("--include-ignored")
    environment = os.environ.copy()
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    environment["CARGO_TERM_COLOR"] = "never"
    environment.setdefault("RUST_BACKTRACE", "1")
    if args.game:
        environment["ASTROLABE_GAME_DIR"] = str(args.game.resolve())
    if args.godot:
        environment["ASTROLABE_GODOT"] = str(Path(args.godot).resolve()) if os.sep in args.godot else args.godot
    temporary = Path(tempfile.mkdtemp(prefix="astrolabe-suite-"))
    environment["TMPDIR"] = str(temporary)
    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    print(f"Running one {'local-inclusive' if args.local else 'routine'} suite; build/test deadline {args.timeout:g}s.", flush=True)
    print(f"Log: {log_path}", flush=True)
    result = None
    error = None
    cleaned = False
    try:
        result = supervise(command, cwd=root, environment=environment, log_path=log_path,
                           started=started, timeout=args.timeout)
    except (OSError, RuntimeError, Interrupted) as exc:
        error = str(exc) or type(exc).__name__
    finally:
        for sig in handlers:
            signal.signal(sig, signal.SIG_IGN)
        cleanup_deadline = started + min(MAX_TOTAL_SECONDS, args.timeout + 20.0)
        cleaned = remove_temporary_tree(temporary, cleanup_deadline)
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
    elapsed = time.monotonic() - started
    code = result.exit_code if result is not None else 125
    if not cleaned or elapsed >= MAX_TOTAL_SECONDS:
        code = code or 125
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    report = {"status": "passed" if code == 0 else "failed", "exit_code": code,
              "elapsed_seconds": round(elapsed, 3), "build_test_deadline_seconds": args.timeout,
              "command": command, "local": args.local, "log": str(log_path),
              "child_cpu_seconds": round(usage.ru_utime + usage.ru_stime, 3),
              "peak_child_rss_kib": usage.ru_maxrss, "temporary_cleanup_complete": cleaned,
              "temporary_directory": None if cleaned else str(temporary), "error": error,
              "timed_out": bool(result and result.timed_out),
              "leaked_children": bool(result and result.leaked_children),
              "remaining_processes": result.remaining_processes if result else []}
    with open(report_path, "x") as file:
        json.dump(report, file, indent=2)
        file.write("\n")
    print(f"{report['status'].upper()}: {elapsed:.2f}s total, exit {code}; report: {report_path}", flush=True)
    if report["timed_out"]:
        print("The complete suite exceeded its shared deadline; this is a failed run.", file=sys.stderr)
    if report["leaked_children"] or report["remaining_processes"] or not cleaned:
        print("Child/temporary cleanup required intervention or remains incomplete; see the report.", file=sys.stderr)
    if error:
        print(error, file=sys.stderr)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
