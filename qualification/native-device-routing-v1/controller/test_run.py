"""Unexecuted CPU policy tests; fixtures never compile or run native code."""
import copy
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import run as M


def pin(path='/actual'):
    return dict(path=path, bytes=1, sha256='a' * 64)


def overlay():
    path = M.WORKER_ROOT + 'native_prefix_device_recorder_v1.rs'
    return dict(schema='ferric-p228-device-routing-cpu-overlay-v1', source_manifest=pin(str(M.SOURCE_MANIFEST)),
        files=[dict(path=path, source=M.SOURCE_DIR + '/draft/' + path,
                    before=None, after=dict(bytes=2, sha256='b' * 64))],
        added_worker_tests=sorted(['prefix_decode_device_observation_v1::tests::schema',
                                  'native_prefix_device_recorder_v1::tests::recorder']))


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

    def test_main_guard_precedes_any_package_or_helper_load(self):
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
    def test_actual_pushed_commits_and_trees_are_required(self):
        value = sources()
        self.assertIs(M.source_shape(value), value)
        for project in ('ferric', 'fe2o3'):
            for key in ('commit', 'tree', 'path'):
                changed = copy.deepcopy(value)
                changed['archives'][project][key] = '/wrong' if key == 'path' else '0' * 40
                with self.subTest(project=project, key=key), self.assertRaises(RuntimeError):
                    M.source_shape(changed)

    def test_missing_extra_archive_or_manifest_field_refused(self):
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

    def test_worker_only_overlay_accepts_new_and_existing_rust(self):
        value = overlay()
        self.assertIs(M.overlay_shape(value), value)
        value['files'][0]['before'] = dict(bytes=1, sha256='c' * 64)
        self.assertIs(M.overlay_shape(value), value)

    def test_pending_manifest_empty_sources_and_empty_tests_refuse(self):
        for key, pending in (('source_manifest', None), ('files', []), ('added_worker_tests', [])):
            value = overlay()
            value[key] = pending
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_runtime_parent_cargo_and_escaping_source_overlays_refuse(self):
        for path in ('crates/fe2o3-kfd/src/lib.rs', 'adapters/m1-engineering-execution-v1/src/lib.rs',
                     M.WORKER_ROOT + '../Cargo.toml', M.WORKER_ROOT + 'config.json', '../outside.rs'):
            value = overlay()
            value['files'][0]['path'] = path
            with self.subTest(path=path), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_duplicate_unknown_or_wrong_source_generation_refuses(self):
        for mutation in ('duplicate', 'row-extra', 'source', 'too-many'):
            value = overlay()
            if mutation == 'duplicate':
                value['files'].append(copy.deepcopy(value['files'][0]))
            elif mutation == 'row-extra':
                value['files'][0]['project'] = 'fe2o3'
            elif mutation == 'source':
                value['files'][0]['source'] = 'old/' + value['files'][0]['path']
            else:
                value['files'] *= 33
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_preimages_are_joined_before_any_replacement_read(self):
        row = overlay()['files'][0]
        row['before'] = dict(bytes=1, sha256='f' * 64)
        with self.assertRaisesRegex(RuntimeError, 'worker source preimage'):
            M.install_overlay(Path('/never'), {}, [row],
                lambda _: self.fail('body read before preimage'), SimpleNamespace())


class InventoryTests(unittest.TestCase):
    def test_frozen_additions_require_sorted_unique_named_schema_and_recorder(self):
        for mutation in ('unsorted', 'duplicate', 'schema', 'recorder', 'invalid'):
            value = overlay()
            if mutation == 'unsorted':
                value['added_worker_tests'].reverse()
            elif mutation == 'duplicate':
                value['added_worker_tests'].append(value['added_worker_tests'][0])
            elif mutation == 'invalid':
                value['added_worker_tests'].append('not a Rust test')
            else:
                prefix = 'prefix_decode_' if mutation == 'schema' else 'native_prefix_'
                value['added_worker_tests'] = [name for name in value['added_worker_tests'] if not name.startswith(prefix)]
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_worker_count_comes_from_frozen_additions_not_a_new_constant(self):
        for count in (1, 19, 23, 31):
            added = {'new::test' + str(index) for index in range(count)}
            with self.subTest(count=count):
                self.assertEqual(M.worker_extension({'old'} | added, {'old'}, added),
                                 [(396 + count, 0, 4), (13, 0, 0)])

    def test_missing_old_unlisted_new_or_duplicate_baseline_addition_refuses(self):
        for actual, prior, added in (({'new'}, {'old'}, {'new'}),
                                     ({'old', 'new', 'extra'}, {'old'}, {'new'}),
                                     ({'old'}, {'old'}, {'old'})):
            with self.subTest(actual=actual), self.assertRaises(RuntimeError):
                M.worker_extension(actual, prior, added)


if __name__ == '__main__':
    unittest.main()
