import copy
import hashlib
from pathlib import Path
import tempfile
import unittest

from qualification_helpers import full_suite_outcomes, select_backend_rlib


class FullSuiteTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / 'stdout'
        self.names = ['suite::ordinary', 'suite::panic', 'suite::external']
        self.ignored = ['suite::external']
        self.body = (
            '\nrunning 3 tests\n'
            'test suite::ordinary ... ok\n'
            'test suite::panic - should panic ... ok\n'
            'test suite::external ... ignored, needs separately retained artifacts\n\n'
            'test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; '
            '0 filtered out; finished in 0.01s\n')

    def check_body(self, body, names=None, ignored=None):
        self.path.write_text(body)
        return full_suite_outcomes(self.path, self.names if names is None else names,
                                   self.ignored if ignored is None else ignored)

    def test_exact_ignored_names_and_should_panic_are_retained(self):
        result = self.check_body(self.body)
        self.assertEqual(result['named_outcomes'], {
            'suite::ordinary': 'ok', 'suite::panic': 'ok', 'suite::external': 'ignored'})
        self.assertEqual(result['ignored_names'], self.ignored)
        self.assertEqual((result['passed'], result['ignored'], result['filtered_out']), (2, 1, 0))

    def test_no_ignored_suite_is_supported(self):
        body = self.body.replace('ignored, needs separately retained artifacts', 'ok')
        body = body.replace('2 passed', '3 passed').replace('1 ignored', '0 ignored')
        self.assertEqual(self.check_body(body, ignored=[])['passed'], 3)

    def test_failed_or_changed_ignored_status_refuses(self):
        for body in [self.body.replace('ordinary ... ok', 'ordinary ... FAILED'),
                     self.body.replace('ordinary ... ok', 'ordinary ... ignored'),
                     self.body.replace('external ... ignored, needs separately retained artifacts',
                                       'external ... ok')]:
            with self.subTest(body=body), self.assertRaises(RuntimeError):
                self.check_body(body)

    def test_swapped_ignored_identity_refuses_even_with_same_counts(self):
        body = self.body.replace('ordinary ... ok', 'ordinary ... ignored')
        body = body.replace('external ... ignored, needs separately retained artifacts', 'external ... ok')
        with self.assertRaises(RuntimeError):
            self.check_body(body)

    def test_duplicate_missing_and_unexpected_names_refuse(self):
        for body in [self.body.replace('test suite::panic - should panic ... ok\n', ''),
                     self.body + 'test suite::ordinary ... ok\n',
                     self.body.replace('suite::ordinary', 'suite::unexpected'),
                     self.body.replace('suite::panic - should panic', 'suite::ordinary')]:
            with self.subTest(body=body), self.assertRaises(RuntimeError):
                self.check_body(body)

    def test_forged_multiple_and_missing_summary_refuse(self):
        for body in [self.body.replace('2 passed', '3 passed'),
                     self.body.replace('0 filtered out', '1 filtered out'),
                     self.body.replace('0 measured', '1 measured'),
                     self.body + self.body.splitlines()[-1] + '\n',
                     self.body[:self.body.index('test result:')]]:
            with self.subTest(body=body), self.assertRaises(RuntimeError):
                self.check_body(body)

    def test_malformed_named_line_is_not_silently_ignored(self):
        for extra in ['test suite::unknown ... maybe\n', 'test suite::ordinary -- should panic ... ok\n']:
            with self.subTest(extra=extra), self.assertRaises(RuntimeError):
                self.check_body(self.body + extra)

    def test_invalid_expected_rosters_refuse(self):
        for names, ignored in [(self.names + self.names[:1], self.ignored),
                               (self.names, self.ignored * 2),
                               (self.names, ['suite::absent']), ([], []),
                               (['invalid name'], []), (self.names, [True])]:
            with self.subTest(names=names, ignored=ignored), self.assertRaises(RuntimeError):
                self.check_body(self.body, names, ignored)


class BackendRlibTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.source = self.root / 'fe2o3'
        self.target = self.root / 'target'
        self.path = self.target / 'debug' / 'deps' / 'librustc_codegen_fe2o3.rlib'
        self.path.parent.mkdir(parents=True)
        self.path.write_bytes(b'!<arch>\nfixture only')
        package = self.source / 'crates' / 'rustc-codegen-fe2o3'
        self.row = dict(reason='compiler-artifact', manifest_path=str(package / 'Cargo.toml'),
                        profile={'test': False}, executable=None,
                        target=dict(name='rustc_codegen_fe2o3', kind=['rlib', 'dylib'],
                                    crate_types=['rlib', 'dylib'], src_path=str(package / 'src/lib.rs')),
                        filenames=[str(self.path), str(self.target / 'debug/librustc_codegen_fe2o3.so')])
        self.pinned = []

    def pin(self, path):
        self.pinned.append(path)
        body = path.read_bytes()
        return dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())

    def select(self, rows=None):
        return select_backend_rlib([self.row] if rows is None else rows,
                                   self.source, self.target, self.pin)

    def test_exact_non_test_backend_archive_selects_and_pins(self):
        result = self.select()
        self.assertEqual(result['cargo_artifact'], self.row)
        self.assertEqual(result['pin']['path'], str(self.path))
        self.assertEqual(self.pinned, [self.path])

    def test_other_package_target_kind_source_and_profile_refuse(self):
        variants = []
        for key, value in [('manifest_path', '/other/Cargo.toml'), ('profile', {'test': True}),
                           ('executable', str(self.path))]:
            row = copy.deepcopy(self.row); row[key] = value; variants.append(row)
        for key, value in [('name', 'other'), ('kind', ['rlib']), ('crate_types', ['dylib']),
                           ('kind', ['rlib', 'dylib', 'rlib']),
                           ('crate_types', ['rlib', 'dylib', 'dylib']),
                           ('src_path', '/other/lib.rs')]:
            row = copy.deepcopy(self.row); row['target'][key] = value; variants.append(row)
        for row in variants:
            with self.subTest(row=row), self.assertRaises(RuntimeError):
                self.select([row])
        self.assertEqual(self.pinned, [])

    def test_duplicate_rows_and_archive_filenames_refuse(self):
        with self.assertRaises(RuntimeError):
            self.select([self.row, copy.deepcopy(self.row)])
        self.row['filenames'].append(str(self.path))
        with self.assertRaises(RuntimeError):
            self.select()

    def test_missing_archive_and_wrong_basename_refuse(self):
        for names in [[self.row['filenames'][1]], [str(self.path.with_name('other.rlib'))]]:
            row = copy.deepcopy(self.row); row['filenames'] = names
            with self.subTest(names=names), self.assertRaises(RuntimeError):
                self.select([row])

    def test_elf_thin_archive_and_short_magic_refuse(self):
        for magic in [b'\x7fELFxxxx', b'!<thin>\n', b'!<arch>']:
            self.path.write_bytes(magic)
            with self.subTest(magic=magic), self.assertRaises(RuntimeError):
                self.select()
        self.assertEqual(self.pinned, [])

    def test_outside_target_refuses_before_pin(self):
        outside = self.root / self.path.name
        outside.write_bytes(b'!<arch>\n')
        self.row['filenames'] = [str(outside)]
        with self.assertRaises(RuntimeError):
            self.select()
        self.assertEqual(self.pinned, [])

    def test_file_alias_refuses(self):
        body = self.path.with_suffix('.body')
        self.path.rename(body)
        self.path.symlink_to(body)
        with self.assertRaises(RuntimeError):
            self.select()

    def test_parent_alias_refuses(self):
        alias = self.target / 'alias'
        alias.symlink_to(self.path.parent, target_is_directory=True)
        self.row['filenames'] = [str(alias / self.path.name)]
        with self.assertRaises(RuntimeError):
            self.select()


if __name__ == '__main__':
    unittest.main()
