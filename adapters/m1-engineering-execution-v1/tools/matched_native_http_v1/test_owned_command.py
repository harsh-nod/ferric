import importlib.util
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import unittest
from unittest.mock import patch


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


ROOT = Path(__file__).parent
m = load('http_owned_command', ROOT / 'owned_command.py')
life = load('http_command_life', ROOT / 'frozen/native_lifecycle.py')


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream)


class CommandTests(unittest.TestCase):
    def execute(self, code, **kwargs):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = None
            with patch.object(os, 'killpg', side_effect=AssertionError('never signal groups')):
                try:
                    value = m.run([sys.executable, '-I', '-B', '-c', code], root, 'fixture',
                        lifecycle=life, save=save, **kwargs)
                except BaseException as error:
                    value = error
            return value, json.loads((root / 'fixture.command.json').read_text())

    def test_success_reaps_once_without_signals_or_new_session(self):
        value, receipt = self.execute('print("done")')
        self.assertEqual(value, (0, b'done\n', b''))
        self.assertTrue(receipt['child_reaped'])
        self.assertTrue(receipt['accepted'])
        self.assertFalse(receipt['start_new_session'])
        self.assertEqual(receipt['signals'], [])

    def test_failure_retains_exit_status_without_signaling_reaped_child(self):
        value, receipt = self.execute('raise SystemExit(7)')
        self.assertIsInstance(value, ValueError)
        self.assertEqual(receipt['returncode'], 7)
        self.assertTrue(receipt['child_reaped'])
        self.assertFalse(receipt['accepted'])
        self.assertEqual(receipt['signals'], [])

    def test_timeout_uses_owned_pidfd_and_reaps(self):
        value, receipt = self.execute('import time; time.sleep(30)', timeout=0.1)
        self.assertIsInstance(value, ValueError)
        self.assertTrue(receipt['child_reaped'])
        self.assertEqual(receipt['signals'], [signal.SIGTERM])
        self.assertFalse(receipt['accepted'])

    def test_stop_during_constructor_still_reaps_assigned_child(self):
        original = m.subprocess.Popen
        def spawn(*args, **kwargs):
            child = original(*args, **kwargs)
            os.kill(os.getpid(), signal.SIGTERM)
            return child
        with life.handling_stop(), patch.object(m.subprocess, 'Popen', side_effect=spawn):
            value, receipt = self.execute('import time; time.sleep(30)')
        self.assertIsInstance(value, life.StopRequested)
        self.assertTrue(receipt['child_reaped'])
        self.assertFalse(receipt['accepted'])

    def test_pidfd_construction_failure_preserves_unreaped_child_authority(self):
        with patch.object(os, 'pidfd_open', side_effect=OSError('unavailable')):
            value, receipt = self.execute('import time; time.sleep(30)')
        self.assertIsInstance(value, OSError)
        self.assertTrue(receipt['child_reaped'])
        self.assertEqual(receipt['signals'], [signal.SIGTERM])

    def test_evidence_limit_refusal_is_not_success(self):
        value, receipt = self.execute('print("x" * 1024)', limit=32)
        self.assertIsInstance(value, ValueError)
        self.assertTrue(receipt['child_reaped'])
        self.assertFalse(receipt['accepted'])

    def test_unchecked_nonzero_result_is_retained_exactly(self):
        value, receipt = self.execute('raise SystemExit(3)', check=False)
        self.assertEqual(value, (3, b'', b''))
        self.assertEqual(receipt['returncode'], 3)
        self.assertEqual(receipt['signals'], [])


if __name__ == '__main__':
    unittest.main()
