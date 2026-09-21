"""Prove nextest deadlines and outer cleanup together using disposable Rust tests."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

from supervisor import supervise


class NextestCleanupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parent.parent
        cls.tool = os.environ.get('ASTROLABE_NEXTEST') or shutil.which('cargo-nextest') or str(root/'output/test-tools/nextest-0.9.145/cargo-nextest')
        cls.temp = tempfile.TemporaryDirectory(prefix='astrolabe-nextest-negative-')
        cls.root = Path(cls.temp.name)
        (cls.root/'src').mkdir()
        (cls.root/'Cargo.toml').write_text('[package]\nname="cleanup-fixture"\nversion="0.1.0"\nedition="2024"\n[workspace]\n')
        (cls.root/'src/lib.rs').write_text('''
#[test]
fn deadline() { std::thread::sleep(std::time::Duration::from_secs(30)); }
#[test]
fn escaped_child() {
    std::process::Command::new("setsid")
        .arg(std::env::var("ASTROLABE_FAKE_PYTHON").unwrap())
        .args(["-c", "import time; time.sleep(30)"])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null()).spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
}
''')
        (cls.root/'nextest.toml').write_text('[profile.probe]\nslow-timeout={period="200ms", terminate-after=1, grace-period="100ms"}\n')
        cls.environment = os.environ.copy()
        cls.environment['ASTROLABE_FAKE_PYTHON'] = sys.executable
        cls.environment['CARGO_TARGET_DIR'] = str(cls.root/'target')
        result = supervise([cls.tool, 'nextest', 'list', '--offline'], cwd=cls.root,
                           environment=cls.environment, log_path=cls.root/'build.log',
                           started=time.monotonic(), timeout=15, echo=False)
        if result.exit_code:
            cls.temp.cleanup()
            raise RuntimeError(f'nextest fixture build failed: {result}')

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def run_case(self, name):
        return supervise([self.tool, 'nextest', 'run', '--offline', '--locked',
                          '--config-file', str(self.root/'nextest.toml'), '--profile', 'probe',
                          '-E', f'test(={name})'], cwd=self.root, environment=self.environment,
                         log_path=self.root/(name+'.log'), started=time.monotonic(), timeout=8, echo=False)

    def test_nextest_per_test_deadline_is_enforced(self):
        result = self.run_case('deadline')
        self.assertNotEqual(result.child_exit_code, 0)
        self.assertFalse(result.timed_out, 'outer deadline hid a broken nextest timeout')
        self.assertFalse(result.remaining_processes)
        self.assertIn('TIMEOUT', (self.root/'deadline.log').read_text())

    def test_nextest_success_cannot_hide_escaped_descendant(self):
        result = self.run_case('escaped_child')
        self.assertEqual(result.child_exit_code, 0)
        self.assertEqual(result.exit_code, 125)
        self.assertTrue(result.leaked_children)
        self.assertFalse(result.remaining_processes)


if __name__ == '__main__':
    unittest.main()
