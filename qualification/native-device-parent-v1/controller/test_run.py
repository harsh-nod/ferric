"""Synthetic controller-policy tests; these never launch Cargo or native code."""
import copy
from pathlib import Path
import types
import unittest

import run as M


FILTERS = (
    ('parent-client', 'tp_finite_client::', 138),
    ('parent-old-wire', 'finite_forward_wire_v1::', 5),
    ('parent-long-wire', 'finite_long_wire_v1::', 8),
    ('parent-smoke-wire', 'finite_rearm_smoke_wire_v1::', 8),
    ('parent-queued-mlp-wire', 'finite_queued_mlp_comparison_wire_v1::', 6),
    ('parent-projection-wire', 'finite_queued_projection_comparison_wire_v1::', 7),
    ('parent-tiles-wire', 'finite_mlp_tiles_comparison_wire_v1::', 9),
    ('parent-decode-wire', 'finite_tiles_decode_wire_v1::', 10),
    ('parent-prefix-layer-wire', 'finite_prefix_layer_wire_v1::', 8),
    ('parent-prefix-decode-wire', 'finite_prefix_decode_wire_v1::', 9),
    ('parent-host-data-v1', 'prefix_decode_host_observation_v1::', 7),
    ('parent-host-data-v2', 'prefix_decode_host_observation_v2::', 9),
)


def helper():
    return types.SimpleNamespace(FILTERS=FILTERS, PARENT_BINS=tuple('old-parent-' + str(i) for i in range(11)))


def pin():
    return dict(bytes=9, sha256='a' * 64)


def overlay():
    return dict(schema='ferric-p228-device-parent-cpu-overlay-v1',
        source_manifest=dict(path=str(M.SOURCE_MANIFEST), **pin()),
        files=[dict(path=name, source=M.SOURCE_DIR + '/draft/' + name,
                    before=None if name in M.NEW_DESTINATIONS else pin(), after=pin())
               for name in sorted(M.DESTINATIONS)],
        added_parent_tests=dict(lib=sorted([M.DEVICE_SELECTOR + 'direct', M.DATA_SELECTOR + 'shared']),
                                bin=['tests::requires_opt_in']))


def source():
    return dict(schema='ferric-p228-clean-worker-sources-v1', archives={
        name: dict(path=str(M.E / M.ARCHIVES[name]), commit=M.COMMITS[name], tree='b' * 40, **pin())
        for name in M.COMMITS})


def inventories():
    old = {selector + 'old_' + str(index) for _, selector, count in FILTERS for index in range(count)}
    extra = {M.DEVICE_SELECTOR + 'new_' + str(index) for index in range(9)}
    extra.update(M.DATA_SELECTOR + 'new_' + str(index) for index in range(12))
    return old, extra


def manifests():
    before = dict(package=dict(name='parent', autobins=False), bin=[dict(name='old', path='src/bin/old.rs')],
                  dependencies=dict(serde='1'), features={'tp-batch-engineering': []})
    after = copy.deepcopy(before)
    after['bin'].insert(0, dict(name=M.NEW_BIN, path='src/bin/' + M.NEW_BIN + '.rs',
                              **{'required-features': ['tp-batch-engineering']}))
    return before, after


class PolicyTests(unittest.TestCase):
    def test_optimized_python_is_rejected_before_helpers(self):
        M.optimization_guard(0, {})
        for optimized, env in [(1, {}), (0, {'PYTHONOPTIMIZE': ''}), (0, {'PYTHONOPTIMIZE': '0'})]:
            with self.assertRaises(RuntimeError):
                M.optimization_guard(optimized, env)

    def test_pins_refuse_boolean_sizes_relative_paths_and_bad_digests(self):
        self.assertEqual(M.content_pin(pin()), pin())
        for value in [dict(bytes=True, sha256='a' * 64), dict(bytes=1, sha256='a' * 63),
                      dict(bytes=-1, sha256='a' * 64), dict(bytes=1, sha256='a' * 64, extra=1)]:
            with self.assertRaises(RuntimeError):
                M.content_pin(value)
        for path in ['relative', '/a/../b', '/a//b']:
            with self.assertRaises(RuntimeError):
                M.file_pin(dict(path=path, **pin()))

    def test_overlay_accepts_exact_seven_parent_members(self):
        value = overlay()
        self.assertEqual(M.overlay_shape(value), value)
        self.assertEqual(sum(row['before'] is None for row in value['files']), 3)

    def test_overlay_refuses_worker_lockfile_duplicate_and_missing_members(self):
        for destination in ['adapters/tp-peer-finite-engineering-worker-v1/src/lib.rs',
                            M.PARENT + 'Cargo.lock', overlay()['files'][1]['path']]:
            value = overlay()
            value['files'][0]['path'] = destination
            with self.assertRaises(RuntimeError):
                M.overlay_shape(value)
        value = overlay()
        value['files'].pop()
        with self.assertRaises(RuntimeError):
            M.overlay_shape(value)

    def test_overlay_refuses_unknown_generation_and_wrong_preimage_kind(self):
        value = overlay()
        value['files'][0]['source'] = 'unreviewed/draft/' + value['files'][0]['path']
        with self.assertRaises(RuntimeError):
            M.overlay_shape(value)
        for row_index in (0, 1):
            value = overlay()
            row = value['files'][row_index]
            row['before'] = pin() if row['before'] is None else None
            with self.assertRaises(RuntimeError):
                M.overlay_shape(value)

    def test_overlay_requires_both_new_modules_and_one_named_bin(self):
        for additions in [dict(lib=[M.DATA_SELECTOR + 'only'], bin=['tests::one']),
                          dict(lib=[M.DEVICE_SELECTOR + 'only'], bin=['tests::one']),
                          dict(lib=['unknown::test'], bin=['tests::one']),
                          dict(lib=overlay()['added_parent_tests']['lib'], bin=['tests::one', 'tests::two'])]:
            value = overlay()
            value['added_parent_tests'] = additions
            with self.assertRaises(RuntimeError):
                M.overlay_shape(value)
        with self.assertRaises(RuntimeError):
            M.named_tests(['tests::same', 'tests::same'])

    def test_source_manifest_requires_exact_pushed_pair(self):
        self.assertEqual(M.source_shape(source()), source())
        for key, replacement in [('commit', 'c' * 40), ('tree', None), ('path', str(M.E / 'other.tar.gz'))]:
            value = source()
            value['archives']['ferric'][key] = replacement
            with self.assertRaises(RuntimeError):
                M.source_shape(value)

    def test_manifest_allows_only_one_new_opt_in_bin(self):
        before, after = manifests()
        M.manifest_delta(before, after)
        self.assertEqual(before['bin'], [dict(name='old', path='src/bin/old.rs')])

    def test_manifest_refuses_dependency_feature_and_duplicate_changes(self):
        for change in ('dependencies', 'features', 'duplicate', 'optin'):
            before, after = manifests()
            if change in ('dependencies', 'features'):
                after[change]['unreviewed'] = '1'
            elif change == 'duplicate':
                after['bin'].append(copy.deepcopy(after['bin'][0]))
            else:
                after['bin'][0]['required-features'] = []
            with self.assertRaises(RuntimeError):
                M.manifest_delta(before, after)

    def test_stable_environment_keeps_bootstrap_and_only_declared_profile_overrides(self):
        stable = types.SimpleNamespace(env=lambda: dict(RUSTC_BOOTSTRAP='fe2o3_device,fe2o3_macros',
            RUSTUP_TOOLCHAIN='1.97.1-x86_64-unknown-linux-gnu', HIP_VISIBLE_DEVICES=''))
        out = M.E / 'device-parent-cpu-v228-v1'
        value = M.environment(stable, out)
        self.assertEqual(stable.T, out / 'target')
        self.assertEqual(value['TMPDIR'], str(out / 'tmp'))
        self.assertEqual(value['HIP_VISIBLE_DEVICES'], '')
        self.assertEqual({k: v for k, v in value.items() if k.startswith('CARGO_PROFILE_')}, {
            'CARGO_PROFILE_TEST_OPT_LEVEL': '2', 'CARGO_PROFILE_DEV_OPT_LEVEL': '2',
            'CARGO_PROFILE_TEST_DEBUG': '0', 'CARGO_PROFILE_DEV_DEBUG': '0'})

    def test_recipe_preserves_38_old_phases_and_adds_only_three(self):
        h = helper()
        stable = types.SimpleNamespace(N=Path('/tools/stable'))
        old = {name: (argv, deadline) for name, argv, deadline in M.recipes(h, stable, M.PRIOR, False)}
        new = {name: (argv, deadline) for name, argv, deadline in M.recipes(h, stable, M.PRIOR, True)}
        self.assertEqual(set(new) - set(old), {'parent-device-data', M.NEW_BIN + '-list', M.NEW_BIN + '-tests'})
        for name in set(old) - {'parent-builds'}:
            self.assertEqual(new[name], old[name])
        expected = list(old['parent-builds'][0])
        expected[-1:-1] = ['--bin', M.NEW_BIN]
        self.assertEqual(new['parent-builds'], (expected, 1200))

    def test_default_check_and_all_commands_keep_offline_locked_parent_target(self):
        rows = M.recipes(helper(), types.SimpleNamespace(N=Path('/tools/stable')), M.PRIOR, True)
        for name, argv, deadline in rows:
            self.assertIn('--offline', argv)
            self.assertIn('--locked', argv)
            self.assertEqual(deadline, 120 if name == 'parent-metadata' else 1200)
            self.assertEqual(argv[argv.index('--manifest-path') + 1], str(M.PRIOR / 'sources/ferric' / M.PARENT / 'Cargo.toml'))
        argv = next(argv for name, argv, _ in rows if name == 'parent-default-check')
        self.assertIn('--lib', argv)
        self.assertNotIn('--features', argv)

    def test_nested_device_tests_count_once_and_shared_schema_is_distinct(self):
        old, extra = inventories()
        selected = M.extended_selections(helper(), old | extra, old, extra)
        self.assertEqual(len(selected['parent-client']), 147)
        self.assertEqual(len(selected['parent-device-data']), 12)
        self.assertEqual(sum(map(len, selected.values())), 245)
        self.assertEqual(len(set().union(*selected.values())), 245)
        self.assertEqual(235 + len(extra) + 1, 257)

    def test_full_inventory_refuses_removed_or_undeclared_or_already_old_names(self):
        old, extra = inventories()
        for actual, additions in [(old | extra | {'unknown::test'}, extra),
                                  ((old | extra) - {next(iter(old))}, extra),
                                  (old | extra, extra | {next(iter(old))})]:
            with self.assertRaises(RuntimeError):
                M.extended_selections(helper(), actual, old, additions)

    def test_added_names_must_all_be_executed(self):
        old, extra = inventories()
        extra.add('not_selected::unrun')
        with self.assertRaises(RuntimeError):
            M.extended_selections(helper(), old | extra, old, extra)

    def test_metadata_relocates_only_local_roots_and_preserves_dependencies(self):
        prior = dict(package_count=2, local={'parent': str(M.PRIOR / 'sources/ferric' / M.PARENT / 'Cargo.toml')},
                     external=[dict(name='dependency', manifest='/registry/Cargo.toml', manifest_sha256='a' * 64)])
        source_root = M.E / 'device-parent-cpu-v228-v1/sources'
        actual = dict(package_count=2, local={'parent': str(source_root / 'ferric' / M.PARENT / 'Cargo.toml')},
                      external=copy.deepcopy(prior['external']))
        M.metadata_generation(actual, prior, source_root)
        for key, value in [('package_count', 3), ('local', prior['local']), ('external', [])]:
            modified = copy.deepcopy(actual)
            modified[key] = value
            with self.assertRaises(RuntimeError):
                M.metadata_generation(modified, prior, source_root)


if __name__ == '__main__':
    unittest.main()
