"""Real failure-path checks; no original game or Godot required."""
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

from supervisor import supervise


class SupervisorTests(unittest.TestCase):
    def run_child(self, script, timeout=2, max_output_bytes=64 * 1024 * 1024):
        with tempfile.TemporaryDirectory() as directory:
            return supervise([sys.executable, '-c', script], cwd=directory,
                             environment=os.environ.copy(), log_path=Path(directory)/'log',
                             started=time.monotonic(), timeout=timeout, echo=False,
                             max_output_bytes=max_output_bytes)

    def test_preserves_failure_exit(self):
        result = self.run_child('raise SystemExit(7)')
        self.assertEqual(result.exit_code, 7)
        self.assertFalse(result.timed_out)
        self.assertFalse(result.remaining_processes)

    def test_failed_launch_is_reported_without_children(self):
        with tempfile.TemporaryDirectory() as directory:
            result = supervise([str(Path(directory)/'absent')], cwd=directory,
                               environment=os.environ.copy(), log_path=Path(directory)/'log',
                               started=time.monotonic(), timeout=2, echo=False)
        self.assertEqual(result.exit_code, 125)
        self.assertIsNotNone(result.error)
        self.assertFalse(result.remaining_processes)

    def test_unbounded_diagnostics_fail_and_clean_up(self):
        result = self.run_child('import os; data=b"x"*65536\nwhile True: os.write(1,data)', max_output_bytes=131072)
        self.assertEqual(result.exit_code, 125)
        self.assertIn('diagnostic output exceeded', result.error)
        self.assertFalse(result.remaining_processes)

    def test_deadline_kills_term_ignoring_private_session_descendants(self):
        result = self.run_child('''
import os, signal, time
signal.signal(signal.SIGTERM, signal.SIG_IGN)
if os.fork() == 0:
    os.setsid()
    if os.fork() != 0: os._exit(0)
while True: time.sleep(1)
''', timeout=0.2)
        self.assertEqual(result.exit_code, 124)
        self.assertTrue(result.timed_out)
        self.assertFalse(result.remaining_processes)
        self.assertLess(result.elapsed_seconds, 4)

    def test_success_with_escaped_worker_fails_and_reaps_worker(self):
        result = self.run_child('''
import os, time
if os.fork() == 0:
    os.setsid()
    time.sleep(30)
else:
    time.sleep(.05)
''')
        self.assertEqual(result.exit_code, 125)
        self.assertTrue(result.leaked_children)
        self.assertFalse(result.remaining_processes)

    def test_preexisting_unrelated_child_survives_cleanup(self):
        other = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)'])
        try:
            result = self.run_child('import time; time.sleep(30)', timeout=.1)
            self.assertEqual(result.exit_code, 124)
            self.assertIsNone(other.poll())
        finally:
            other.terminate()
            other.wait(timeout=2)

    def test_interrupt_releases_the_owned_tree(self):
        # A separate supervisor receives SIGTERM; its cleanup still runs.
        root = str(Path(__file__).resolve().parent)
        script = f'''
import os,sys,signal,time,tempfile,json
sys.path.insert(0,{root!r})
from supervisor import supervise,interrupted
from pathlib import Path
signal.signal(signal.SIGTERM,interrupted)
with tempfile.TemporaryDirectory() as d:
 r=supervise([sys.executable,'-c','import os,signal,time; os.kill(os.getppid(), signal.SIGTERM); time.sleep(30)'],cwd=d,environment=os.environ.copy(),log_path=Path(d)/'log',started=time.monotonic(),timeout=2,echo=False)
 assert r.exit_code == 143 and not r.remaining_processes, r
'''
        result = self.run_child(script, timeout=4)
        self.assertEqual(result.exit_code, 0)


if __name__ == '__main__':
    unittest.main()
