"""Synthetic admission tests only; these never enter main or launch a compiler."""
import copy
from pathlib import Path
import unittest

import run as subject


def plan():
    return dict(schema='ferric-p228-projection-ordered-segment-cpu-inputs-v1',
        proposals={'fe2o3': {}, 'ferric': {}},
        added_tests={name: ['some_module::test_case'] for name in
            ('runtime', 'worker', 'parent_library', 'parent_binary')},
        parent_binary='ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering',
        runtime_full_suite_cpu_reviewed=True)


def row(path):
    return dict(path=path, source='candidate/' + path, before=None,
        after=dict(bytes=12, sha256='1' * 64))


class ControllerPolicyTests(unittest.TestCase):
    def test_closed_plan_accepts_exact_reviewed_schema(self):
        subject.plan_shape(plan())

    def test_plan_rejects_extra_or_missing_fields(self):
        for change in ('extra', 'missing'):
            value = plan()
            if change == 'extra': value['runtime_gpu_tests_allowed'] = True
            else: del value['schema']
            with self.assertRaises(RuntimeError): subject.plan_shape(value)

    def test_full_suite_requires_literal_cpu_review(self):
        for flag in (False, 1, None, 'true'):
            value = plan(); value['runtime_full_suite_cpu_reviewed'] = flag
            with self.assertRaises(RuntimeError): subject.plan_shape(value)

    def test_old_or_unbounded_parent_selector_refused(self):
        for name in (subject.OLD_BIN, '/tmp/worker', 'ferric-qwen3-finite-evil;command-engineering'):
            value = plan(); value['parent_binary'] = name
            with self.assertRaises(RuntimeError): subject.plan_shape(value)

    def test_added_names_are_nonempty_sorted_unique_and_bounded(self):
        for names in ([], ['z', 'a'], ['a', 'a'], ['a;echo'], ['x'] * 181):
            value = plan(); value['added_tests']['runtime'] = names
            with self.assertRaises(RuntimeError): subject.plan_shape(value)

    def test_test_and_project_groups_cannot_expand(self):
        for key in ('proposals', 'added_tests'):
            value = plan(); value[key]['extra'] = []
            with self.assertRaises(RuntimeError): subject.plan_shape(value)

    def test_runtime_overlay_is_scoped_to_rust_sources(self):
        value = {'files': [row('crates/fe2o3-kfd/src/engineering_gfx950_peer.rs')]}
        self.assertEqual(subject.overlay_rows('fe2o3', value), value['files'])
        for name in ('Cargo.toml', 'crates/fe2o3-kfd/build.rs', 'crates/fe2o3-kfd/src/run.py',
                     'crates/fe2o3-kfd/src/../Cargo.toml', '/tmp/x.rs'):
            with self.assertRaises(RuntimeError): subject.overlay_rows('fe2o3', {'files': [row(name)]})

    def test_ferric_overlay_allows_only_adapter_rust_and_parent_manifest(self):
        for name in (subject.PARENT + '/Cargo.toml', subject.WORKER + '/src/main.rs'):
            subject.overlay_rows('ferric', {'files': [row(name)]})
        for name in (subject.WORKER + '/Cargo.toml', subject.PARENT + '/Cargo.lock', 'src/main.rs'):
            with self.assertRaises(RuntimeError): subject.overlay_rows('ferric', {'files': [row(name)]})

    def test_overlay_refuses_duplicate_empty_and_oversized_rosters(self):
        item = row('crates/fe2o3-kfd/src/new.rs')
        for rows in ([], [item, item], [item] * 65):
            with self.assertRaises(RuntimeError): subject.overlay_rows('fe2o3', {'files': rows})

    def test_proposal_generation_keeps_runtime_v1_and_only_ferric_v2(self):
        self.assertEqual(subject.proposal_directory('fe2o3'), 'p228-projection-residual-mlp-ordered-runtime-v1')
        self.assertEqual(subject.proposal_directory('ferric'), 'p228-projection-residual-mlp-ordered-ferric-v2')
        for project in ('runtime', '', None):
            with self.assertRaises(RuntimeError): subject.proposal_directory(project)
        self.assertEqual(subject.FAILED_V1, subject.E / 'projection-ordered-segment-cpu-v228-v1')
        self.assertEqual(subject.PACKAGE, subject.E / 'p228-projection-ordered-segment-cpu-v2')

    def test_output_never_reuses_failed_v1_or_noncanonical_label(self):
        for label in ('projection-ordered-segment-cpu-v228-v2', 'projection-ordered-segment-cpu-v228-v10'):
            subject.output_label(label)
        for label in ('projection-ordered-segment-cpu-v228-v1', 'projection-ordered-segment-cpu-v228-v0',
                      'projection-ordered-segment-cpu-v228-v02', '../projection-ordered-segment-cpu-v228-v2'):
            with self.assertRaises(RuntimeError): subject.output_label(label)

    def test_overlay_preimage_and_body_pins_are_closed(self):
        for pin in ({'bytes': 0, 'sha256': '1' * 64}, {'bytes': 1, 'sha256': 'bad'},
                    {'bytes': True, 'sha256': '1' * 64}, {'bytes': 1 << 21, 'sha256': '1' * 64},
                    {'bytes': 1, 'sha256': '1' * 64, 'path': '/tmp/other'}):
            for key in ('before', 'after'):
                item = row('crates/fe2o3-kfd/src/new.rs'); item[key] = pin
                with self.assertRaises(RuntimeError): subject.overlay_rows('fe2o3', {'files': [item]})

    def test_overlay_body_must_follow_candidate_namespace(self):
        item = row('crates/fe2o3-kfd/src/new.rs'); item['source'] = 'baseline/' + item['path']
        with self.assertRaises(RuntimeError): subject.overlay_rows('fe2o3', {'files': [item]})

    def test_exact_extension_keeps_every_old_test(self):
        subject.extend({'old', 'new'}, {'old'}, ['new'])
        for actual, added in (({'new'}, ['new']), ({'old', 'new', 'extra'}, ['new']),
                              ({'old'}, ['old']), ({'old'}, ['new'])):
            with self.assertRaises(RuntimeError): subject.extend(actual, {'old'}, added)

    def test_cargo_change_is_only_the_distinct_parent_binary(self):
        binary = plan()['parent_binary']
        before = dict(bin=[dict(name=subject.OLD_BIN, path='src/bin/old.rs')], dependencies={'serde': '1'})
        after = copy.deepcopy(before)
        addition = dict(name=binary, path='src/bin/' + binary + '.rs', **{'required-features': ['tp-batch-engineering']})
        after['bin'].append(addition); subject.cargo_delta(before, after, binary)
        for change in ('dependency', 'duplicate', 'feature'):
            bad = copy.deepcopy(after)
            if change == 'dependency': bad['dependencies']['serde'] = '2'
            elif change == 'duplicate': bad['bin'].append(addition)
            else: bad['bin'][-1]['required-features'] = []
            with self.assertRaises(RuntimeError): subject.cargo_delta(before, bad, binary)

    def test_relocation_preserves_other_recipe_values(self):
        out = Path('/fresh/qualified')
        value = {'argv': [str(subject.OLD / 'sources/file.rs'), '--offline'], 'timeout': 1200, 'enabled': False}
        self.assertEqual(subject.relocated(value, out),
            {'argv': ['/fresh/qualified/sources/file.rs', '--offline'], 'timeout': 1200, 'enabled': False})
        self.assertEqual(value['argv'][0], str(subject.OLD / 'sources/file.rs'))

    def test_ignored_inventory_keeps_names_and_not_passes(self):
        raw = 'test a::x ... ignored, retained artifact\ntest b::y ... ok\ntest c::z ... ignored\n'
        self.assertEqual(subject.ignored(raw), {'a::x', 'c::z'})


if __name__ == '__main__':
    unittest.main()
