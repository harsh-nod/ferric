"""CPU-only cache-inventory regression fixtures; no subprocesses or compilation."""
import errno
import importlib.util
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    'guarded_mlp_cpu_v3_scratch_tests', Path(__file__).with_name('run_cpu.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


class ScratchBytesTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.target = self.base / 'target'
        self.tmp = self.base / 'tmp'
        self.target.mkdir()
        self.tmp.mkdir()
        for name, path in (('TARGET', self.target), ('TMP', self.tmp)):
            replacement = patch.object(R, name, path)
            replacement.start()
            self.addCleanup(replacement.stop)

    def failing_walk(self, error):
        def walk(root, *, followlinks, onerror):
            self.assertFalse(followlinks)
            if root == self.target:
                onerror(error)
                yield str(root), [], ['retained']
        return walk

    def test_vanished_owned_descendant_does_not_discard_remaining_files(self):
        (self.target / 'retained').write_bytes(b'1234567')
        missing = FileNotFoundError(errno.ENOENT, 'vanished',
                                    str(self.target / 'debug' / 'removed'))
        with patch.object(R.os, 'walk', self.failing_walk(missing)):
            self.assertEqual(R.scratch_bytes(), 7)

    def test_permission_error_on_owned_descendant_is_not_suppressed(self):
        denied = PermissionError(errno.EACCES, 'denied', str(self.target / 'debug'))
        with patch.object(R.os, 'walk', self.failing_walk(denied)):
            with self.assertRaises(PermissionError) as caught:
                R.scratch_bytes()
        self.assertIs(caught.exception, denied)

    def test_missing_root_or_unowned_or_ambiguous_path_is_not_suppressed(self):
        paths = [str(self.target), str(self.base / 'elsewhere'),
                 str(self.target / '..' / 'elsewhere'), 'relative/removed', None]
        for path in paths:
            with self.subTest(path=path):
                missing = FileNotFoundError(errno.ENOENT, 'vanished', path)
                with patch.object(R.os, 'walk', self.failing_walk(missing)):
                    with self.assertRaises(FileNotFoundError) as caught:
                        R.scratch_bytes()
                self.assertIs(caught.exception, missing)

    def test_other_os_error_is_not_suppressed(self):
        failure = OSError(errno.EIO, 'I/O failure', str(self.target / 'debug'))
        with patch.object(R.os, 'walk', self.failing_walk(failure)):
            with self.assertRaises(OSError) as caught:
                R.scratch_bytes()
        self.assertIs(caught.exception, failure)

    def test_sizes_sum_both_owned_trees_and_exclude_outside_files(self):
        (self.target / 'debug').mkdir()
        (self.target / 'debug' / 'a').write_bytes(b'a' * 7)
        (self.target / 'b').write_bytes(b'b' * 3)
        (self.tmp / 'c').write_bytes(b'c' * 5)
        (self.base / 'outside').write_bytes(b'x' * 100)
        self.assertEqual(R.scratch_bytes(), 15)

    def test_existing_vanished_file_tolerance_remains(self):
        (self.target / 'retained').write_bytes(b'1234')
        def walk(root, *, followlinks, onerror):
            self.assertFalse(followlinks)
            if root == self.target:
                yield str(root), [], ['removed', 'retained']
        with patch.object(R.os, 'walk', walk):
            self.assertEqual(R.scratch_bytes(), 4)

    def test_exact_file_count_bound_and_one_over_remain_enforced(self):
        for count in (200000, 200001):
            with self.subTest(count=count):
                def walk(root, *, followlinks, onerror):
                    self.assertFalse(followlinks)
                    if root == self.target:
                        yield str(root), [], ['one'] * count
                with patch.object(R.os, 'walk', walk), patch.object(
                        Path, 'lstat', lambda _path: SimpleNamespace(st_size=1)):
                    if count == 200000:
                        self.assertEqual(R.scratch_bytes(), count)
                    else:
                        with self.assertRaisesRegex(RuntimeError, 'scratch inventory bound'):
                            R.scratch_bytes()


if __name__ == '__main__':
    unittest.main()
