"""Pure source/recipe refusals; these do not execute Cargo or authenticate a run."""
import copy
from pathlib import Path
import unittest
from unittest.mock import patch

import run as M


def fixture():
    value = dict(schema='ferric-p228-state-bank-batch-cpu-overlay-v1',
        runtime_preimages=dict(path=str(M.E / M.RUNTIME_DIR / 'preimages.json'), bytes=1, sha256='a' * 64),
        ferric_source_manifest=dict(path=str(M.E / M.FERRIC_DIR / 'source-pins.json'), bytes=1, sha256='b' * 64),
        files=[])
    for name in M.RUNTIME_FILES:
        value['files'].append(dict(project='fe2o3', path=M.RUNTIME_ROOT + name,
            source=M.RUNTIME_DIR + '/source/' + name,
            before=None if name in M.RUNTIME_FILES[-2:] else dict(bytes=1, sha256='c' * 64),
            after=dict(bytes=2, sha256='d' * 64)))
    for name in M.FERRIC_FILES:
        value['files'].append(dict(project='ferric', path=M.FERRIC_ROOT + name,
            source=M.FERRIC_DIR + '/draft/' + M.FERRIC_ROOT + name,
            before=None if name == 'bank_batch_tests.rs' else dict(bytes=1, sha256='e' * 64),
            after=dict(bytes=2, sha256='f' * 64)))
    return value


class PolicyTests(unittest.TestCase):
    def test_unoptimized_environment_only_is_admitted(self):
        M.optimization_guard(0, {})
        for optimized, env in ((1, {}), (2, {}), (0, {'PYTHONOPTIMIZE': ''}), (0, {'PYTHONOPTIMIZE': '0'})):
            with self.subTest(optimized=optimized, env=env), self.assertRaises(RuntimeError):
                M.optimization_guard(optimized, env)

    def test_main_rejects_optimized_before_package_or_helper_access(self):
        with patch.object(M, 'optimization_guard', side_effect=RuntimeError('guard')) as guard:
            with patch.object(M, 'package_inputs') as package:
                with self.assertRaisesRegex(RuntimeError, 'guard'):
                    M.main()
                guard.assert_called_once()
                package.assert_not_called()

    def test_closed_overlay_accepts_exact_seven_plus_three(self):
        value = fixture()
        self.assertIs(M.overlay_shape(value), value)
        self.assertEqual(sum(row['before'] is None for row in value['files']), 3)

    def test_pending_source_pins_or_after_hashes_fail_closed(self):
        for key in ('runtime_preimages', 'ferric_source_manifest'):
            value = fixture()
            value[key] = None
            with self.subTest(key=key), self.assertRaises(AssertionError):
                M.overlay_shape(value)
        value = fixture()
        value['files'][-1]['after'] = None
        with self.assertRaises(AssertionError):
            M.overlay_shape(value)

    def test_overlay_rejects_unknown_fields_and_missing_or_duplicate_destinations(self):
        variants = []
        value = fixture()
        value['extra'] = False
        variants.append(value)
        value = fixture()
        value['files'].pop()
        variants.append(value)
        value = fixture()
        value['files'][-1] = copy.deepcopy(value['files'][0])
        variants.append(value)
        for value in variants:
            with self.subTest(value=value), self.assertRaises(AssertionError):
                M.overlay_shape(value)

    def test_overlay_rejects_path_escape_wrong_project_and_wrong_source_generation(self):
        for key, replacement in (('path', '../escape.rs'), ('project', 'other'),
                                 ('source', M.RUNTIME_DIR.replace('-v2', '-v1') + '/source/lib.rs')):
            value = fixture()
            value['files'][0][key] = replacement
            with self.subTest(key=key), self.assertRaises(AssertionError):
                M.overlay_shape(value)

    def test_overlay_requires_exact_three_absent_preimages(self):
        value = fixture()
        value['files'][0]['before'] = None
        with self.assertRaises(AssertionError):
            M.overlay_shape(value)
        value = fixture()
        value['files'][-1]['before'] = dict(bytes=1, sha256='0' * 64)
        with self.assertRaises(AssertionError):
            M.overlay_shape(value)

    def test_pin_schema_refuses_bool_bytes_bad_digest_or_relative_path(self):
        for key, replacement in (('bytes', True), ('bytes', -1), ('sha256', 'A' * 64), ('path', 'relative')):
            pin = dict(path='/actual', bytes=1, sha256='0' * 64)
            pin[key] = replacement
            with self.subTest(key=key, value=replacement), self.assertRaises(AssertionError):
                M.shape_pin(pin)

    def test_exact_test_extension_preserves_every_historical_name(self):
        M.exact_extension({'old', 'new'}, {'old'}, {'new'})
        for actual, prior, added in (({'new'}, {'old'}, {'new'}),
                                     ({'old', 'new', 'extra'}, {'old'}, {'new'}),
                                     ({'old'}, {'old'}, {'old'})):
            with self.subTest(actual=actual), self.assertRaises(AssertionError):
                M.exact_extension(actual, prior, added)

    def test_named_source_rosters_have_exact_new_counts(self):
        self.assertEqual(len(M.BANK_TESTS), 10)
        self.assertEqual(len(M.WORKER_TESTS), 11)
        self.assertTrue(all(name.startswith(M.BANK_SELECTOR) for name in M.BANK_TESTS))
        self.assertTrue(all(name.startswith(M.WORKER_SELECTOR) for name in M.WORKER_TESTS))
        self.assertEqual([count for _, _, count in M.MEMORY_FILTERS], [4, 4, 2])

    def test_recipe_preserves_old_phases_and_adds_only_four_runtime_selectors(self):
        old = tuple(('old-' + str(i), 'old::' + str(i) + '::', 1) for i in range(12))
        extra = (('state-bank', M.BANK_SELECTOR, 10), *M.MEMORY_FILTERS)
        before = M.recipes('/cargo', Path('/fresh/worker/Cargo.toml'), old)
        after = M.recipes('/cargo', Path('/fresh/worker/Cargo.toml'), (*old, *extra))
        self.assertEqual(len(before), 17)
        self.assertEqual(len(after), 21)
        names = {name for name, _, _ in before}
        self.assertEqual([row for row in after if row[0] in names], before)
        self.assertEqual({name for name, _, _ in after} - names, {name for name, _, _ in extra})

    def test_recipe_retains_default_build_thread_and_deadline_semantics(self):
        rows = M.recipes('/cargo', Path('/fresh/worker/Cargo.toml'), (('state-bank', M.BANK_SELECTOR, 10),))
        for name, argv, deadline in rows:
            self.assertEqual(deadline, 120 if name == 'metadata' else 1200)
            self.assertIn('--offline', argv)
            self.assertIn('--locked', argv)
            self.assertNotIn('--release', argv)
            if name != 'metadata':
                self.assertEqual(argv[argv.index('--jobs') + 1], '2')
            if name == 'worker-build':
                self.assertEqual(argv[argv.index('--profile') + 1], 'test')
            if name in ('state-bank', 'worker-tests'):
                self.assertEqual(argv[-1], '--test-threads=2')

    def test_ignored_names_keep_all_four_historical_ignores_distinct(self):
        raw = 'test a ... ok\ntest b ... ignored\ntest c ... ignored, hardware\n'
        self.assertEqual(M.ignored_names(raw), {'b', 'c'})

    def test_preimage_failure_precedes_replacement_read_or_write(self):
        class NeverRead:
            def pin(self, _):
                raise RuntimeError('unexpected filesystem access')
        rows = [dict(path='first.rs', before=dict(bytes=1, sha256='a' * 64), after=dict(bytes=2, sha256='b' * 64))]
        with self.assertRaisesRegex(AssertionError, 'overlay preimage'):
            M.apply_overlay(Path('/never'), {}, 'fe2o3', rows,
                lambda _: self.fail('replacement was read before preimage join'), NeverRead())


if __name__ == '__main__':
    unittest.main()
