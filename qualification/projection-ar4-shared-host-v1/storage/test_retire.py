"""Synthetic scope tests only; never invoke plan/apply, quiescence or deletion."""
import importlib.util
from pathlib import Path
import stat
import types
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('performance_cache_adapter', Path(__file__).with_name('retire.py'))
M = importlib.util.module_from_spec(spec)
spec.loader.exec_module(M)


class ScopeTests(unittest.TestCase):
    def setUp(self):
        self.c = types.SimpleNamespace(disposable=Mock(return_value=True))
        self.st = types.SimpleNamespace(st_mode=stat.S_IFREG | 0o600, st_nlink=1)
        self.path = M.DEPS / 'libordinary-0123.rlib'

    def test_direct_cache_delegates_to_unchanged_classifier(self):
        for suffix in ('.rlib', '.rmeta', '.o'):
            path = self.path.with_suffix(suffix)
            self.assertTrue(M.eligible(self.c, path, self.st, set()))
            self.c.disposable.assert_called_with(path, self.st, set())

    def test_nested_and_sibling_scopes_never_reach_classifier(self):
        for path in (M.DEPS / 'nested/x.rmeta', M.TARGET / 'debug/x.rlib',
                     M.TARGET.with_name('other-target') / 'debug/deps/x.o'):
            self.assertFalse(M.eligible(self.c, path, self.st, set()))
        self.c.disposable.assert_not_called()

    def test_explicit_path_is_protected_independent_of_old_digest(self):
        value = [{'path': str(self.path), 'bytes': 1, 'sha256': '0' * 64},
                 {'path': str(self.path), 'bytes': 9, 'sha256': '1' * 64}]
        explicit = set(M.referenced_paths(value))
        self.assertEqual(explicit, {str(self.path)})
        self.assertFalse(M.eligible(self.c, self.path, self.st, explicit))
        self.c.disposable.assert_not_called()

    def test_old_absolute_key_maps_and_candidate_string_aliases_protected(self):
        other = M.DEPS / 'libpinned.rmeta'
        value = {str(self.path): {'sha256': 'a' * 64}, 'candidate': [str(other)]}
        self.assertEqual(set(M.referenced_paths(value)), {str(self.path), str(other)})

    def test_non_target_strings_do_not_broaden_scope(self):
        value = ['/tmp/x.rlib', 'relative.rmeta', str(M.TARGET) + '-other/debug/deps/x.o', 8, None]
        self.assertEqual(list(M.referenced_paths(value)), [])

    def test_backend_alias_names_stay_protected(self):
        for name in ('librustc_codegen_fe2o3.rlib', 'librustc_codegen_fe2o3-abcd.rmeta',
                     'RUSTC_CODEGEN_FE2O3-alias.o'):
            self.assertFalse(M.eligible(self.c, M.DEPS / name, self.st, set()))
        self.c.disposable.assert_not_called()

    def test_hardlinks_are_never_eligible(self):
        self.st.st_nlink = 2
        self.assertFalse(M.eligible(self.c, self.path, self.st, set()))
        self.c.disposable.assert_not_called()

    def test_executable_modes_are_never_eligible(self):
        self.st.st_mode |= 0o100
        self.assertFalse(M.eligible(self.c, self.path, self.st, set()))
        self.c.disposable.assert_not_called()

    def test_symlink_and_nonregular_entries_refuse(self):
        for mode in (stat.S_IFLNK | 0o777, stat.S_IFDIR | 0o700, stat.S_IFIFO | 0o600):
            self.st.st_mode = mode
            with self.assertRaises(RuntimeError):
                M.eligible(self.c, self.path, self.st, set())
        self.c.disposable.assert_not_called()

    def test_unsupported_suffixes_cannot_be_selected(self):
        for suffix in ('.so', '.hsaco', '.elf', '.d', '.json'):
            self.assertFalse(M.eligible(self.c, self.path.with_suffix(suffix), self.st, set()))
        self.c.disposable.assert_not_called()

    def test_classifier_rejection_is_not_overridden(self):
        self.c.disposable.return_value = False
        self.assertFalse(M.eligible(self.c, self.path, self.st, set()))
        self.c.disposable.assert_called_once()

    def test_nonterminal_manifest_census_selection_is_explicit(self):
        for name in ('source-manifest.json', 'candidate.json', 'retained-paths.json', 'inputs.json'):
            self.assertTrue(M.supplemental_name(name))
        for name in ('sources-before.json', 'manifest.py', 'stdout', 'module.ll'):
            self.assertFalse(M.supplemental_name(name))


if __name__ == '__main__':
    unittest.main(verbosity=2)
