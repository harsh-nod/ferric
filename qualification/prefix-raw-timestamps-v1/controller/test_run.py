"""Pure runner-policy fixtures only; no Cargo, archive, or GPU execution."""
import copy
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import run as M


def pin(path='/actual'):
    return dict(path=path, bytes=1, sha256='a' * 64)


def overlay():
    return dict(schema='ferric-p228-prefix-raw-timestamps-cpu-overlay-v1',
        source_manifest=pin(str(M.SOURCE_MANIFEST)),
        runtime_preimages=pin(str(M.E / M.RUNTIME_DIR / 'preimages.json')),
        added_runtime_tests=sorted(M.NEW_TESTS), files=[dict(
            path=M.RUNTIME_ROOT + name, source=M.RUNTIME_DIR + '/source/' + name,
            before=None if name == M.RUNTIME_FILES[-1] else dict(bytes=1, sha256='b' * 64),
            after=dict(bytes=2, sha256='c' * 64)) for name in M.RUNTIME_FILES])


def sources():
    return dict(schema='ferric-p228-clean-worker-sources-v1', archives={name: dict(
        **pin(str(M.E / M.ARCHIVES[name])), commit=M.COMMITS[name], tree=M.TREES[name])
        for name in ('ferric', 'fe2o3')})


class GuardTests(unittest.TestCase):
    def test_optimized_or_inherited_pythonoptimize_refused(self):
        M.optimization_guard(0, {})
        for optimized, env in ((1, {}), (2, {}), (0, {'PYTHONOPTIMIZE': ''}),
                               (0, {'PYTHONOPTIMIZE': '0'})):
            with self.subTest(optimized=optimized, env=env), self.assertRaises(RuntimeError):
                M.optimization_guard(optimized, env)

    def test_main_guard_precedes_package_or_authenticated_helpers(self):
        with patch.object(M, 'optimization_guard', side_effect=RuntimeError('guard')) as guard, \
                patch.object(M, 'package_inputs') as package:
            with self.assertRaisesRegex(RuntimeError, 'guard'):
                M.main()
            guard.assert_called_once()
            package.assert_not_called()

    def test_pin_shape_rejects_bool_negative_digest_case_and_relative_path(self):
        self.assertEqual(M.shape_pin(pin()), pin())
        for key, value in (('bytes', True), ('bytes', -1), ('sha256', 'A' * 64), ('path', 'relative')):
            record = pin()
            record[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                M.shape_pin(record)


class SourceTests(unittest.TestCase):
    def test_clean_archive_pair_binds_both_actual_commits_and_trees(self):
        value = sources()
        self.assertIs(M.source_shape(value), value)
        self.assertEqual(value['archives']['ferric']['commit'], '44e308d72815405f9127473368671e9696c5ebaa')
        self.assertEqual(value['archives']['fe2o3']['commit'], '3d217aabc3c48e9c767fa28a05bd596987c29f9d')

    def test_old_generation_and_archive_path_substitution_refused(self):
        for project in ('ferric', 'fe2o3'):
            for key, value in (('commit', '0' * 40), ('tree', '1' * 40), ('path', '/old/archive.tar.gz')):
                record = sources()
                record['archives'][project][key] = value
                with self.subTest(project=project, key=key), self.assertRaises(RuntimeError):
                    M.source_shape(record)

    def test_source_schema_and_roster_are_closed(self):
        for mutation in ('schema', 'extra', 'missing', 'row-extra'):
            value = sources()
            if mutation == 'schema':
                value['schema'] = 'old'
            elif mutation == 'extra':
                value['overlay'] = []
            elif mutation == 'missing':
                value['archives'].pop('ferric')
            else:
                value['archives']['fe2o3']['ignored'] = False
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.source_shape(value)

    def test_exact_three_file_overlay_and_nine_authored_tests(self):
        value = overlay()
        self.assertIs(M.overlay_shape(value), value)
        self.assertEqual(len(M.NEW_TESTS), 9)
        self.assertEqual(sum(row['before'] is None for row in value['files']), 1)

    def test_pending_archive_or_source_pins_fail_closed(self):
        for key in ('source_manifest', 'runtime_preimages'):
            value = overlay()
            value[key] = None
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                M.overlay_shape(value)
        value = overlay()
        value['files'][0]['after'] = None
        with self.assertRaises(RuntimeError):
            M.overlay_shape(value)

    def test_overlay_rejects_missing_duplicate_or_foreign_destinations(self):
        for mutation in ('missing', 'duplicate', 'foreign', 'escape'):
            value = overlay()
            if mutation == 'missing':
                value['files'].pop()
            elif mutation == 'duplicate':
                value['files'][1] = copy.deepcopy(value['files'][0])
            elif mutation == 'foreign':
                value['files'][0]['path'] = 'adapters/worker/src/lib.rs'
            else:
                value['files'][0]['path'] = '../escape.rs'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_overlay_rejects_unformatted_generation_and_wrong_new_preimage(self):
        for mutation in ('source', 'preimage', 'new'):
            value = overlay()
            if mutation == 'source':
                value['files'][0]['source'] = value['files'][0]['source'].replace('-v2/', '-v1/')
            elif mutation == 'preimage':
                value['files'][0]['before'] = None
            else:
                value['files'][-1]['before'] = dict(bytes=1, sha256='0' * 64)
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_added_test_roster_rejects_missing_duplicate_or_renamed_tests(self):
        for mutation in ('missing', 'duplicate', 'renamed'):
            value = overlay()
            if mutation == 'missing':
                value['added_runtime_tests'].pop()
            elif mutation == 'duplicate':
                value['added_runtime_tests'][0] = value['added_runtime_tests'][1]
            else:
                value['added_runtime_tests'][0] += '_changed'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_all_preimages_are_checked_before_reading_replacement(self):
        row = dict(path='existing.rs', before=dict(bytes=1, sha256='a' * 64),
                   after=dict(bytes=2, sha256='b' * 64))
        with self.assertRaisesRegex(RuntimeError, 'overlay preimage'):
            M.install_overlay(Path('/never'), {}, [row],
                lambda _: self.fail('unexpected replacement read'), SimpleNamespace())


class InventoryTests(unittest.TestCase):
    def test_compiled_runtime_inventory_is_an_exact_extension(self):
        M.exact_extension({'old', 'new'}, {'old'}, {'new'})
        for actual, prior, added in (({'new'}, {'old'}, {'new'}),
                                     ({'old', 'new', 'extra'}, {'old'}, {'new'}),
                                     ({'old'}, {'old'}, {'old'})):
            with self.subTest(actual=actual), self.assertRaises(RuntimeError):
                M.exact_extension(actual, prior, added)

    def test_existing_filters_survive_and_only_prefix_resident_count_grows(self):
        base = SimpleNamespace(FILTERS=(('prefix-resident', M.PREFIX_RESIDENT, 10),
            ('mlp-timestamps', 'mlp::timestamp_tests::', 7), ('policy', 'policy::', 5)))
        state = SimpleNamespace(BANK_SELECTOR='bank::', MEMORY_FILTERS=(('memory', 'memory::', 10),))
        rows = M.filters_for(base, state)
        self.assertEqual(rows[:3], (('prefix-resident', M.PREFIX_RESIDENT, 12),
            ('mlp-timestamps', 'mlp::timestamp_tests::', 7), ('policy', 'policy::', 5)))
        self.assertEqual(rows[3:5], (('state-bank', 'bank::', 10), ('memory', 'memory::', 10)))
        self.assertEqual(rows[5:], M.EXTRA_FILTERS)

    def test_extra_coverage_keeps_raw_signal_and_queue_tests_distinct(self):
        self.assertEqual([row[0] for row in M.EXTRA_FILTERS], [
            'prefix-timestamps', 'raw-timestamps', 'memory-raw-timestamps', 'queue-raw-timestamps'])
        self.assertEqual([row[2] for row in M.EXTRA_FILTERS], [7, 8, 6, 2])
        self.assertEqual(sum(row[2] for row in M.EXTRA_FILTERS[1:]), 16)

    def test_new_test_names_cover_public_refusal_and_real_poison_join(self):
        self.assertIn(M.PREFIX_RESIDENT + 'resident_v6_raw_entry_invalid_deadline_poison_blocks_both_modes_and_rearm',
                      M.NEW_TESTS)
        self.assertIn(M.PREFIX_TIMESTAMPS + 'timestamp_prefix_join_failure_enters_existing_group_poison_path',
                      M.NEW_TESTS)
        self.assertEqual(sum(name.startswith(M.PREFIX_TIMESTAMPS) for name in M.NEW_TESTS), 7)


if __name__ == '__main__':
    unittest.main()
