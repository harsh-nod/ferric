"""Pure controller-policy checks; no compilation or native execution."""
import copy
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import run as M


def pin(path='/actual'):
    return dict(path=path, bytes=1, sha256='a' * 64)


def overlay():
    files = []
    for name in M.RUNTIME_FILES:
        path = M.RUNTIME_ROOT + name
        files.append(dict(path=path, source=M.SOURCE_DIR + '/draft/' + path,
            before=None if 'clock_correlation' in name else dict(bytes=1, sha256='b' * 64),
            after=dict(bytes=2, sha256='c' * 64)))
    return dict(schema='ferric-p228-gfx950-clock-cpu-overlay-v1',
        source_manifest=pin(str(M.SOURCE_MANIFEST)), files=files,
        added_runtime_tests=sorted([M.DEVICE_CLOCK + 'case' + str(i) for i in range(6)]
            + [M.PEER_CLOCK + 'case' + str(i) for i in range(8)]))


def sources():
    return dict(schema='ferric-p228-clean-worker-sources-v1', archives={project: dict(
        **pin(str(M.E / M.ARCHIVES[project])), commit=M.COMMITS[project], tree=M.TREES[project])
        for project in ('ferric', 'fe2o3')})


class GuardTests(unittest.TestCase):
    def test_optimized_python_or_environment_refused(self):
        M.optimization_guard(0, {})
        for optimized, env in ((1, {}), (2, {}), (0, {'PYTHONOPTIMIZE': ''}), (0, {'PYTHONOPTIMIZE': '0'})):
            with self.subTest(optimized=optimized, env=env), self.assertRaises(RuntimeError):
                M.optimization_guard(optimized, env)

    def test_guard_precedes_package_or_helper_load(self):
        with patch.object(M, 'optimization_guard', side_effect=RuntimeError('guard')), \
                patch.object(M, 'package_inputs') as package:
            with self.assertRaisesRegex(RuntimeError, 'guard'):
                M.main()
            package.assert_not_called()

    def test_pin_refuses_wrong_fields_bool_and_noncanonical_path(self):
        self.assertEqual(M.file_pin(pin()), pin())
        for key, value in (('bytes', True), ('bytes', -1), ('sha256', 'A' * 64),
                           ('path', 'relative'), ('path', '/absolute/../escape')):
            row = pin()
            row[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                M.file_pin(row)


class SourceTests(unittest.TestCase):
    def test_exact_archive_commits_trees_and_paths_required(self):
        value = sources()
        self.assertIs(M.source_shape(value), value)
        for project in ('ferric', 'fe2o3'):
            for key in ('commit', 'tree', 'path'):
                changed = copy.deepcopy(value)
                changed['archives'][project][key] = '/wrong' if key == 'path' else '0' * 40
                with self.subTest(project=project, key=key), self.assertRaises(RuntimeError):
                    M.source_shape(changed)

    def test_missing_extra_archive_or_field_refused(self):
        for mutation in ('missing', 'extra', 'field'):
            value = sources()
            if mutation == 'missing':
                value['archives'].pop('ferric')
            elif mutation == 'extra':
                value['archives']['other'] = copy.deepcopy(value['archives']['ferric'])
            else:
                value['unreviewed'] = True
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.source_shape(value)

    def test_exact_six_preimages_four_new_modules_accept(self):
        value = overlay()
        self.assertIs(M.overlay_shape(value), value)

    def test_missing_new_module_or_missing_preimage_refused(self):
        for index in range(10):
            value = overlay()
            row = value['files'][index]
            row['before'] = dict(bytes=1, sha256='d' * 64) if row['before'] is None else None
            with self.subTest(index=index), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_parent_worker_cargo_escape_or_other_runtime_path_refused(self):
        for path in ('adapters/tp-peer-finite-engineering-worker-v1/src/lib.rs',
                     'adapters/m1-engineering-execution-v1/src/lib.rs',
                     M.RUNTIME_ROOT + '../Cargo.toml', M.RUNTIME_ROOT + 'other.rs', '../outside.rs'):
            value = overlay()
            value['files'][0]['path'] = path
            with self.subTest(path=path), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_duplicate_extra_missing_or_old_source_generation_refused(self):
        for mutation in ('duplicate', 'row-extra', 'source', 'missing', 'extra'):
            value = overlay()
            if mutation == 'duplicate':
                value['files'][0] = copy.deepcopy(value['files'][1])
            elif mutation == 'row-extra':
                value['files'][0]['project'] = 'fe2o3'
            elif mutation == 'source':
                value['files'][0]['source'] = 'old/' + value['files'][0]['path']
            elif mutation == 'missing':
                value['files'].pop()
            else:
                value['files'].append(copy.deepcopy(value['files'][0]))
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_preimages_checked_before_any_replacement_read(self):
        row = overlay()['files'][0]
        with self.assertRaisesRegex(RuntimeError, 'runtime source preimage'):
            M.install_overlay(Path('/never'), {}, [row],
                lambda _: self.fail('body read before preimage'), SimpleNamespace())


class InventoryTests(unittest.TestCase):
    def test_additions_are_sorted_unique_six_device_eight_peer_cases(self):
        for mutation in ('unsorted', 'duplicate', 'missing', 'foreign', 'invalid'):
            value = overlay()
            names = value['added_runtime_tests']
            if mutation == 'unsorted':
                names.reverse()
            elif mutation == 'duplicate':
                names[0] = names[1]
            elif mutation == 'missing':
                names.pop()
            elif mutation == 'foreign':
                names[0] = 'another::case'
                names.sort()
            else:
                names[0] = 'not a Rust test'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_exact_old_inventory_plus_declared_new_names_required(self):
        M.runtime_extension({'old', 'new'}, {'old'}, {'new'})
        for actual, prior, added in (({'new'}, {'old'}, {'new'}),
                                     ({'old', 'new', 'extra'}, {'old'}, {'new'}),
                                     ({'old'}, {'old'}, {'old'})):
            with self.subTest(actual=actual), self.assertRaises(RuntimeError):
                M.runtime_extension(actual, prior, added)

    def test_currentness_filters_do_not_match_group_currentness(self):
        rows = [row for row in M.EXTRA_FILTERS if row[0].startswith('currentness-')]
        self.assertEqual(len(rows), 4)
        for _, selector, count in rows:
            self.assertEqual(count, 1)
            self.assertNotEqual(selector, 'currentness::tests::')
            self.assertNotIn(selector, 'device::gfx950::group_currentness::tests::example')

    def test_eight_additional_phases_cover_39_tests(self):
        self.assertEqual(len(M.EXTRA_FILTERS), 8)
        self.assertEqual(sum(row[2] for row in M.EXTRA_FILTERS), 39)
        self.assertEqual(len({row[0] for row in M.EXTRA_FILTERS}), 8)
        self.assertIn(('queue-core', 'queue::tests::', 17), M.EXTRA_FILTERS)


if __name__ == '__main__':
    unittest.main()
