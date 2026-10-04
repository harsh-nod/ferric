"""Pure build-receipt policy tests; no processes or GPU execution."""
import copy
import json
from pathlib import Path
import unittest

import run


class BuildTests(unittest.TestCase):
    def metadata(self):
        return dict(target_directory=str(run.OUT / 'target'), packages=[
            dict(name=name, manifest_path=str(run.COPY / 'crates' / name / 'Cargo.toml'), source=None)
            for name in ('fe2o3-pliron', 'rustc-codegen-fe2o3')])

    def test_metadata_accepts_exact_relocated_local_packages(self):
        self.assertEqual(len(run.metadata_paths(self.metadata(), run.COPY)), 2)

    def test_metadata_rejects_old_target_or_escaped_local_package(self):
        for field in ('target', 'local'):
            value = self.metadata()
            if field == 'target':
                value['target_directory'] = str(run.F / 'target')
            else:
                value['packages'][0]['manifest_path'] = str(run.F / 'crates/fe2o3-pliron/Cargo.toml')
            with self.assertRaises(RuntimeError):
                run.metadata_paths(value, run.COPY)

    def test_metadata_rejects_missing_or_duplicate_changed_package(self):
        for rows in (self.metadata()['packages'][:1], self.metadata()['packages'] * 2):
            with self.assertRaises(RuntimeError):
                run.metadata_paths(dict(self.metadata(), packages=rows), run.COPY)

    def test_inventory_requires_nonempty_unique_actual_names(self):
        self.assertEqual(run.inventory('b: test\na: test\n'), ['a', 'b'])
        for text in ('', '0 tests, 0 benchmarks\n', 'a: test\na: test\n'):
            with self.assertRaises(RuntimeError):
                run.inventory(text)

    def test_results_bind_every_test_name_and_ignored_name(self):
        text = 'test a ... ok\ntest b ... ignored, actual hardware\ntest result: ok. 1 passed; 0 failed; 1 ignored;\n'
        result = run.test_results(text, ['a', 'b'], ['b'])
        self.assertEqual(result['passed'], 1)
        for names, ignored in ((['a'], []), (['a', 'b'], []), (['x', 'b'], ['b'])):
            with self.assertRaises(RuntimeError):
                run.test_results(text, names, ignored)

    def test_results_reject_substituted_failed_or_duplicate_summary(self):
        text = 'test a ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n'
        for changed in (text.replace('1 passed', '0 passed'), text.replace('a ... ok', 'a ... FAILED'),
                        text + 'test result: ok. 1 passed; 0 failed; 0 ignored;\n'):
            with self.assertRaises(RuntimeError):
                run.test_results(changed, ['a'], [])

    def test_should_panic_success_retains_exact_inventory_name(self):
        text = 'test expected_panic - should panic ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n'
        self.assertEqual(run.test_results(text, ['expected_panic'], [])['passed'], 1)
        for changed in (text.replace('... ok', '... FAILED'),
                        text.replace('expected_panic', 'another_test'),
                        text.replace(' - should panic', ' - unrecognized annotation')):
            with self.assertRaises(RuntimeError):
                run.test_results(changed, ['expected_panic'], [])

    def artifact(self, tests=True):
        return dict(reason='compiler-artifact', target=dict(name='fe2o3_pliron'), profile=dict(test=tests),
                    manifest_path=str(run.COPY / 'crates/fe2o3-pliron/Cargo.toml'),
                    executable=str(run.OUT / 'target/debug/deps/test'), filenames=[])

    def test_cargo_artifact_must_have_matching_manifest_profile_and_fresh_path(self):
        row = self.artifact()
        paths = run.built_artifacts(json.dumps(row), 'fe2o3-pliron', 'fe2o3_pliron', True, run.OUT / 'target')
        self.assertEqual(paths, [row['executable']])
        for key, value in [('manifest_path', str(run.F / 'crates/fe2o3-pliron/Cargo.toml')),
                           ('executable', '/old/test'), ('profile', dict(test=False))]:
            with self.assertRaises(RuntimeError):
                run.built_artifacts(json.dumps(dict(row, **{key: value})), 'fe2o3-pliron',
                                    'fe2o3_pliron', True, run.OUT / 'target')

    def test_cargo_library_artifacts_and_duplicate_selection(self):
        row = self.artifact(False)
        row.update(executable=None, filenames=[str(run.OUT / 'target/debug/libexample.rlib'),
                                               str(run.OUT / 'target/debug/libexample.so')])
        self.assertEqual(len(run.built_artifacts(json.dumps(row), 'fe2o3-pliron',
                            'fe2o3_pliron', False, run.OUT / 'target')), 2)
        with self.assertRaises(RuntimeError):
            run.built_artifacts(json.dumps(row) + '\n' + json.dumps(row), 'fe2o3-pliron',
                                'fe2o3_pliron', False, run.OUT / 'target')


class RpoTests(unittest.TestCase):
    def proposal(self):
        names = sorted(run.FILES)
        return dict(schema='ferric-p228-partial-move-rpo-source-proposal-v2', status='untested-candidate',
            tests_authored=17, tests_executed=False, formatted=False, compiler_build_executed=False,
            checked_probe_executed=False, gpu_execution=False, production_authority=False, limits_changed=False,
            qualified_generation=dict(run.QUALIFIED_COMPLETE), qualified_source_snapshot=dict(run.QUALIFIED_SOURCES),
            files=[dict(path=name, source='draft/' + name,
                        before=None if name.endswith(('partial_move_rpo_v1_tests.rs', 'partial_move_fifo_oracle_v1.rs'))
                               else dict(bytes=1, sha256='ab' * 32),
                        after=dict(bytes=2, sha256='cd' * 32)) for name in names],
            retained_files=[], test_names=[run.PREFIX + 'case_' + str(i) for i in range(17)])

    def test_overlay_is_four_exact_bodies_and_seventeen_unique_tests(self):
        value = self.proposal()
        self.assertEqual(len(run.overlay_members(value)), 4)
        for kind in ('path', 'duplicate', 'before', 'tests', 'authority', 'qualified'):
            bad = copy.deepcopy(value)
            if kind == 'path': bad['files'][0]['source'] = '../outside'
            elif kind == 'duplicate': bad['files'][0] = bad['files'][1]
            elif kind == 'before':
                next(row for row in bad['files'] if row['before'] is None)['before'] = {'bytes': 1, 'sha256': 'aa'}
            elif kind == 'tests': bad['test_names'][-1] = bad['test_names'][0]
            elif kind == 'authority': bad['checked_probe_executed'] = True
            else: bad['qualified_generation']['sha256'] = '00' * 32
            with self.subTest(kind=kind), self.assertRaises(RuntimeError):
                run.overlay_members(bad)

    def test_source_transition_checks_both_existing_preimages_and_new_absence(self):
        proposal = self.proposal()
        before = {row['path']: row['before'] for row in proposal['files'] if row['before'] is not None}
        before['other.rs'] = dict(bytes=3, sha256='ef' * 32)
        after = dict(before, **{row['path']: row['after'] for row in proposal['files']})
        run.source_transition(before, after, proposal)
        for name in run.FILES:
            bad = dict(before); bad[name] = dict(bytes=99, sha256='00' * 32)
            with self.subTest(name=name), self.assertRaises(RuntimeError):
                run.source_transition(bad, after, proposal)

    def test_formatter_cannot_change_any_unselected_source_or_file_roster(self):
        proposal = self.proposal()
        before = {row['path']: row['before'] for row in proposal['files'] if row['before'] is not None}
        before['Cargo.lock'] = dict(bytes=3, sha256='ef' * 32)
        after = dict(before, **{row['path']: dict(bytes=7, sha256='de' * 32) for row in proposal['files']})
        run.source_transition(before, after, proposal, formatted=True)
        for bad in (dict(after, **{'Cargo.lock': {}}), dict(after, **{'extra.rs': {}}),
                    {name: pin for name, pin in after.items() if name != 'Cargo.lock'}):
            with self.assertRaises(RuntimeError):
                run.source_transition(before, bad, proposal, formatted=True)
        with self.assertRaises(RuntimeError):
            run.source_transition(before, after, proposal)

    def test_full_suites_preserve_all_old_names_ignores_and_add_only_rpo(self):
        proposal = self.proposal()
        previous = dict(tests={name: dict(names=['old', 'skipped'], ignored_names=['skipped'])
                               for name in ('pliron', 'compiler')})
        names = sorted(['old', 'skipped', *proposal['test_names']])
        self.assertEqual(run.required_tests('pliron', names, ['skipped'], previous, proposal),
                         sorted(proposal['test_names']))
        self.assertEqual(run.required_tests('compiler', ['old', 'skipped'], ['skipped'], previous, proposal), [])
        for changed, ignored in ((names[:-1], ['skipped']), (sorted(names + ['unexpected']), ['skipped']),
                                 (names, []), (names, ['skipped', proposal['test_names'][0]])):
            with self.assertRaises(RuntimeError):
                run.required_tests('pliron', changed, ignored, previous, proposal)

    def test_prior_and_candidate_phase_rosters_are_distinct_ten_and_twelve(self):
        self.assertEqual(len(run.BASE_PHASES), 10)
        self.assertEqual(len(run.PHASES), 12)
        self.assertEqual(run.PHASES - run.BASE_PHASES, {'rustfmt', 'rustfmt-check'})
        self.assertEqual(run.F, run.QUALIFIED_COPY)
        self.assertNotEqual(run.F, run.R / 'fe2o3')

    def test_relative_sources_rejects_foreign_paths_extent_mismatch_or_aliases(self):
        path = run.QUALIFIED_COPY / 'Cargo.toml'
        pin = dict(path=str(path), bytes=3, sha256='ab' * 32)
        snapshot = {str(path): dict(pin=pin, stamp=[1, 2, 3, 4, 5])}
        self.assertEqual(run.relative_sources(snapshot, run.QUALIFIED_COPY),
                         {'Cargo.toml': dict(bytes=3, sha256='ab' * 32)})
        for kind in ('path', 'extent', 'stamp', 'extra'):
            bad = copy.deepcopy(snapshot)
            if kind == 'path': bad[str(path)]['pin']['path'] = '/other/Cargo.toml'
            elif kind == 'extent': bad[str(path)]['pin']['bytes'] = 4
            elif kind == 'stamp': bad[str(path)]['stamp'].pop()
            else: bad[str(path)]['extra'] = True
            with self.subTest(kind=kind), self.assertRaises(RuntimeError):
                run.relative_sources(bad, run.QUALIFIED_COPY)

    def test_metadata_relocation_changes_only_qualified_namespace(self):
        old = dict(packages=[dict(manifest_path=str(run.QUALIFIED_COPY / 'Cargo.toml'), source=None),
                             dict(manifest_path=str(run.R / 'toolchain/cargo/registry/Cargo.toml'),
                                  source='registry+https://github.com/rust-lang/crates.io-index')],
                   target_directory=str(run.QUALIFIED / 'target'))
        new = run.relocate(old)
        self.assertEqual(new['packages'][0]['manifest_path'], str(run.COPY / 'Cargo.toml'))
        self.assertEqual(new['packages'][1], old['packages'][1])
        self.assertEqual(new['target_directory'], str(run.OUT / 'target'))


if __name__ == '__main__':
    unittest.main()
