"""Data-only regressions for absolute ldd interpreter closure."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('peer_gpu_runtime_v3', Path(__file__).with_name('run_gpu.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)

LOADER = 'ld-linux-x86-64.so.2'
INTERP = '/lib64/' + LOADER
DIRECT = '/lib/x86_64-linux-gnu/' + LOADER
LIBC = '/lib/x86_64-linux-gnu/libc.so.6'
LIBGCC = '/lib/x86_64-linux-gnu/libgcc_s.so.1'


def pin(path, digit):
    return dict(path=path, bytes=100, sha256=digit * 64)


def dynamic(needed=('libgcc_s.so.1', 'libc.so.6', LOADER), interpreter=INTERP):
    return (''.join(' 0x1 (NEEDED) Shared library: [' + name + ']\n' for name in needed)
            + ' [Requesting program interpreter: ' + interpreter + ']\n').encode()


def ldd(direct=INTERP, named_loader=False):
    text = (' linux-vdso.so.1 (0x7fff0000)\n'
            ' libgcc_s.so.1 => ' + LIBGCC + ' (0x7aaa0000)\n'
            ' libc.so.6 => ' + LIBC + ' (0x7bbb0000)\n')
    if named_loader:
        text += ' ' + LOADER + ' => ' + DIRECT + ' (0x7ccc0000)\n'
    return (text + ' ' + direct + ' (0x7ccc0000)\n').encode()


class RuntimeLibraryTests(unittest.TestCase):
    def setUp(self):
        loader_pin = pin('/usr/lib/x86_64-linux-gnu/' + LOADER, '1')
        self.pins = {INTERP: loader_pin, DIRECT: loader_pin,
                     LIBC: pin('/usr/lib/x86_64-linux-gnu/libc.so.6', '2'),
                     LIBGCC: pin('/usr/lib/x86_64-linux-gnu/libgcc_s.so.1', '3')}
        self.reads = []

    def resolve(self, path):
        self.reads.append(path)
        return self.pins[path]

    def parse(self, dynamic_bytes=None, ldd_bytes=None):
        with patch.object(R, 'resolved_input', side_effect=self.resolve):
            return R.runtime_libraries(dynamic() if dynamic_bytes is None else dynamic_bytes,
                                       ldd() if ldd_bytes is None else ldd_bytes)

    def test_actual_three_needed_shape_resolves_absolute_loader(self):
        value = self.parse()
        self.assertEqual(set(value['libraries']), {'libgcc_s.so.1', 'libc.so.6', LOADER})
        self.assertEqual(value['libraries'][LOADER], dict(reported_path=INTERP, pin=self.pins[INTERP]))
        self.assertEqual(value['interpreter']['pin'], self.pins[INTERP])
        self.assertTrue(value['all_needed_resolved'])
        self.assertEqual(self.reads, [LIBGCC, LIBC, INTERP, INTERP])

    def test_old_two_needed_shape_does_not_invent_a_library(self):
        value = self.parse(dynamic(('libgcc_s.so.1', 'libc.so.6')))
        self.assertNotIn(LOADER, value['libraries'])
        self.assertEqual(value['interpreter']['pin'], self.pins[INTERP])

    def test_canonical_interpreter_alias_with_same_basename_is_valid(self):
        value = self.parse(ldd_bytes=ldd(DIRECT))
        self.assertEqual(value['libraries'][LOADER]['reported_path'], DIRECT)
        self.assertEqual(value['libraries'][LOADER]['pin'], self.pins[INTERP])

    def test_loader_cannot_satisfy_an_unresolved_other_needed_name(self):
        raw = ldd().replace((' libc.so.6 => ' + LIBC + ' (0x7bbb0000)\n').encode(), b'')
        with self.assertRaisesRegex(RuntimeError, 'every DT_NEEDED'):
            self.parse(ldd_bytes=raw)

    def test_direct_interpreter_must_have_exact_canonical_pin(self):
        self.pins[DIRECT] = pin('/different/' + LOADER, '4')
        with self.assertRaisesRegex(RuntimeError, 'ldd/readelf interpreter mismatch'):
            self.parse(ldd_bytes=ldd(DIRECT))

    def test_same_pin_under_wrong_basename_cannot_cover_needed_loader(self):
        other = '/lib64/not-the-needed-loader.so'
        self.pins[other] = self.pins[INTERP]
        with self.assertRaisesRegex(RuntimeError, 'basename mismatch'):
            self.parse(ldd_bytes=ldd(other))

    def test_already_named_loader_must_agree_with_interpreter(self):
        value = self.parse(ldd_bytes=ldd(named_loader=True))
        self.assertEqual(value['libraries'][LOADER]['reported_path'], DIRECT)
        self.pins[DIRECT] = pin('/different/' + LOADER, '4')
        with self.assertRaisesRegex(RuntimeError, 'named interpreter pin mismatch'):
            self.parse(ldd_bytes=ldd(named_loader=True))

    def test_duplicate_or_missing_interpreter_and_vdso_are_refused(self):
        variants = [ldd() + (' ' + INTERP + ' (0x123)\n').encode(),
                    ldd().replace((' ' + INTERP + ' (0x7ccc0000)\n').encode(), b''),
                    ldd() + b' linux-vdso.so.1 (0x456)\n',
                    ldd().replace(b' linux-vdso.so.1 (0x7fff0000)\n', b'')]
        for raw in variants:
            with self.subTest(raw=raw), self.assertRaisesRegex(RuntimeError, 'exactly one'):
                self.parse(ldd_bytes=raw)

    def test_duplicate_needed_or_named_library_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, 'readelf missing/duplicate'):
            self.parse(dynamic(('libc.so.6', 'libc.so.6')))
        with self.assertRaisesRegex(RuntimeError, 'duplicate ldd SONAME'):
            self.parse(ldd_bytes=ldd() + (' libc.so.6 => ' + LIBC + ' (0x123)\n').encode())

    def test_missing_multiple_or_relative_interpreter_is_refused(self):
        raws = [dynamic().replace((' [Requesting program interpreter: ' + INTERP + ']\n').encode(), b''),
                dynamic() + (' [Requesting program interpreter: ' + INTERP + ']\n').encode()]
        for raw in raws:
            with self.subTest(raw=raw), self.assertRaisesRegex(RuntimeError, 'readelf missing/duplicate'):
                self.parse(dynamic_bytes=raw)
        with self.assertRaisesRegex(RuntimeError, 'unrecognized ldd line'):
            self.parse(ldd_bytes=ldd().replace(INTERP.encode(), b'relative-loader.so'))

    def test_unresolved_empty_unknown_and_relative_library_lines_refuse(self):
        for raw in [b'', b'libc.so.6 => not found\n', b'not a dynamic executable\n',
                    ldd() + b'unknown output\n', ldd().replace(LIBC.encode(), b'libc.so.6')]:
            with self.subTest(raw=raw), self.assertRaises(RuntimeError):
                self.parse(ldd_bytes=raw)

    def test_unneeded_transitive_library_remains_pinned(self):
        path = '/lib/x86_64-linux-gnu/libm.so.6'
        self.pins[path] = pin('/usr/lib/x86_64-linux-gnu/libm.so.6', '5')
        value = self.parse(ldd_bytes=ldd() + (' libm.so.6 => ' + path + ' (0x789)\n').encode())
        self.assertEqual(value['libraries']['libm.so.6']['pin'], self.pins[path])
        self.assertIn(path, self.reads)


if __name__ == '__main__':
    unittest.main()
