"""Synthetic policy checks only; no Cargo, subprocess, native or remote calls."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('indexed_join_cpu', Path(__file__).with_name('run.py'))
M = importlib.util.module_from_spec(spec)
spec.loader.exec_module(M)


class IndexedTests(unittest.TestCase):
    def proposal(self):
        return dict(schema='ferric-p228-kir-indexed-formal-join-source-v1', base_cpu={'actual': 'cpu'},
            base_source_snapshot={'actual': 'source'}, tentative=True, diagnostic_site_measured=False,
            compiled=False, tests_executed=False, changes_live_join=False, changes_canonical_bytes=False,
            changes_authority=False, work_limit=M.WORK_LIMIT, storage_limit=M.STORAGE_LIMIT,
            added_test_count=20, added_tests=[M.TEST_PREFIX + str(i) for i in range(20)])

    def successful_stdout(self):
        return (f'\nrunning 1 test\ntest {M.SELECTOR} ... ok\n\n'
                'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 204 filtered out; finished in 0.01s\n')

    def result(self, **values):
        return dict(dict(exit_code=0, reason=None, group_absent=True), **values)

    def test_actual_join_accepts_real_stdout_shape_only(self):
        result = M.actual_join(self.successful_stdout(), '', self.result())
        self.assertEqual(result['test'], M.SELECTOR)
        self.assertEqual(result['passed'], 1)
        self.assertTrue(result['actual_capture_join_passed'])
        self.assertFalse(result['fresh_hsaco_emitted'])

    def test_actual_join_requires_natural0_and_absent_group(self):
        for values in (dict(exit_code=101), dict(exit_code=False), dict(reason='timeout'), dict(group_absent=False)):
            with self.subTest(values=values), self.assertRaises(RuntimeError):
                M.actual_join(self.successful_stdout(), '', self.result(**values))

    def test_join_rejects_substituted_name_summary_and_duplicate(self):
        good = self.successful_stdout()
        for text in (good.replace(M.SELECTOR, M.SELECTOR + '_other'), good.replace('204 filtered', '203 filtered'),
                     good.replace('1 passed', '0 passed'), good + f'test {M.SELECTOR} ... ok\n'):
            with self.subTest(text=text), self.assertRaises(RuntimeError): M.actual_join(text, '', self.result())

    def test_join_cannot_relabel_prior_refusal_or_diagnostic(self):
        for stderr in ('CanonicalKernelIrWorkLimitV1 { actual: 1084825160, limit: 1073741824 }',
                       'kir-join-work-v1 stage=outer-middle-before'):
            with self.subTest(stderr=stderr), self.assertRaises(RuntimeError):
                M.actual_join(self.successful_stdout(), stderr, self.result())

    def test_lower_inventory_is_actual_dynamic_not_guessed_count(self):
        proposal = self.proposal()
        for old_count in (1, 7, 31):
            names = sorted(proposal['added_tests'] + ['old_test_' + str(i) for i in range(old_count)])
            result = M.lower_inventory(names, ['old_test_0'], proposal)
            self.assertEqual(result['names'], names)
            self.assertEqual(result['added_names'], sorted(proposal['added_tests']))
            self.assertFalse(result['prior_compiled_library_inventory_available'])

    def test_added_tests_must_all_be_present_unique_and_nonignored(self):
        proposal = self.proposal(); names = sorted(proposal['added_tests'])
        for selected, ignored in ((names[:-1], []), (names + [names[-1]], []),
                                  (names, [names[0]]), (sorted(names + [M.TEST_PREFIX + 'unexpected']), [])):
            with self.subTest(selected=selected, ignored=ignored), self.assertRaises(RuntimeError):
                M.lower_inventory(selected, ignored, proposal)

    def test_ignored_inventory_cannot_hide_unlisted_names(self):
        proposal = self.proposal(); names = sorted(proposal['added_tests'] + ['old'])
        for ignored in (['missing'], ['old', 'old']):
            with self.assertRaises(RuntimeError): M.lower_inventory(names, ignored, proposal)

    def source_fixture(self):
        before = {name: dict(bytes=1, sha256='1' * 64) for name in M.FILES - M.NEW_FILES}
        before['Cargo.toml'] = dict(bytes=2, sha256='2' * 64)
        rows = [dict(path=name, source='draft/' + name, before=before.get(name),
                     after=dict(bytes=3, sha256='3' * 64)) for name in sorted(M.FILES)]
        after = dict(before); after.update({row['path']: row['after'] for row in rows})
        return before, after, dict(files=rows)

    def test_source_adds_exact_two_files_and_formats_only_four(self):
        before, after, proposal = self.source_fixture()
        M.transition(before, after, proposal)
        for name in M.FILES: after[name] = dict(bytes=4, sha256='4' * 64)
        M.transition(before, after, proposal, formatted=True)
        after['Cargo.toml'] = dict(bytes=5, sha256='5' * 64)
        with self.assertRaises(RuntimeError): M.transition(before, after, proposal, formatted=True)

    def test_wrong_preimage_absent_claim_or_extra_file_refused(self):
        before, after, proposal = self.source_fixture()
        changed = copy.deepcopy(proposal)
        next(row for row in changed['files'] if row['before'] is not None)['before']['bytes'] += 1
        with self.assertRaises(RuntimeError): M.transition(before, after, changed)
        changed = copy.deepcopy(proposal)
        next(row for row in changed['files'] if row['before'] is not None)['before'] = None
        with self.assertRaises(RuntimeError): M.transition(before, after, changed)
        with self.assertRaises(RuntimeError): M.transition(before, dict(after, extra={}), proposal)

    def dependency_fixture(self):
        old = {str(M.ORIGINAL / 'crates/old.rs'): {'old': True}, '/registry/input': {'pin': 1}}
        sources = {str(M.COPY / name): {'name': name} for name in {'crates/old.rs', 'Cargo.toml'} | M.NEW_FILES}
        current = {name: row for name, row in sources.items() if not name.endswith('/Cargo.toml')}
        current['/registry/input'] = {'pin': 1}
        return old, current, sources

    def test_dependencies_add_only_two_declared_files(self):
        old, current, sources = self.dependency_fixture()
        M.dependency_transition(old, current, sources)
        for key in (str(M.COPY / next(iter(M.NEW_FILES))), str(M.COPY / 'crates/old.rs')):
            changed = dict(current); del changed[key]
            with self.assertRaises(RuntimeError): M.dependency_transition(old, changed, sources)
        with self.assertRaises(RuntimeError): M.dependency_transition(old, {**current, str(M.COPY / 'extra.rs'): {}}, sources)

    def test_dependencies_preserve_external_and_local_bodies(self):
        old, current, sources = self.dependency_fixture()
        for key in ('/registry/input', str(M.COPY / 'crates/old.rs')):
            with self.assertRaises(RuntimeError): M.dependency_transition(old, {**current, key: {}}, sources)

    def environment_fixture(self):
        env = dict(CARGO_TARGET_DIR='new-target', TMPDIR='new-tmp', LD_LIBRARY_PATH='new-deps', GPU='hidden')
        prefix = 'FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_'
        old = dict(env, **{prefix + 'PATH': M.HANDOFF['path'], prefix + 'BYTES': str(M.HANDOFF['bytes']), prefix + 'SHA256': M.HANDOFF['sha256']})
        old.update(CARGO_TARGET_DIR='old-target', TMPDIR='old-tmp', LD_LIBRARY_PATH='old-deps')
        return env, dict(env=old, argv=['old-ELF', '--exact', M.SELECTOR, '--ignored', '--show-output', '--test-threads=1'])

    def test_retained_environment_has_no_instrumentation_flag(self):
        env, previous = self.environment_fixture()
        selected = M.selected_environment(previous, env)
        self.assertNotIn(M.DIAGNOSTIC_ENV, selected)
        self.assertEqual(selected['GPU'], 'hidden')
        for extra in ({M.DIAGNOSTIC_ENV: '1'}, {'GPU': 'visible'}):
            with self.assertRaises(RuntimeError): M.selected_environment(previous, dict(env, **extra))

    def test_handoff_and_ignored_selector_are_immutable(self):
        env, previous = self.environment_fixture()
        changed = copy.deepcopy(previous); changed['argv'][2] += '_other'
        with self.assertRaises(RuntimeError): M.selected_environment(changed, env)
        changed = copy.deepcopy(previous); changed['env']['FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_BYTES'] = '1'
        with self.assertRaises(RuntimeError): M.selected_environment(changed, env)

    def test_proposal_preserves_source_generation_and_limits(self):
        proposal = self.proposal(); cpu, source = proposal['base_cpu'], proposal['base_source_snapshot']
        M.proposal_contract(proposal, cpu, source)
        for key, value in (('base_cpu', {}), ('base_source_snapshot', {}), ('compiled', True),
                           ('changes_authority', True), ('work_limit', M.WORK_LIMIT + 1), ('storage_limit', 1)):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                M.proposal_contract(dict(proposal, **{key: value}), cpu, source)

    def test_proposal_requires_twenty_distinct_declared_names(self):
        proposal = self.proposal(); cpu, source = proposal['base_cpu'], proposal['base_source_snapshot']
        for names in (proposal['added_tests'][:-1], [proposal['added_tests'][0]] * 20,
                      ['unrelated'] + proposal['added_tests'][1:]):
            with self.assertRaises(RuntimeError): M.proposal_contract(dict(proposal, added_tests=names), cpu, source)

    def test_exact_thirteen_phase_boundary_and_metadata_relocation(self):
        self.assertEqual(len(M.PHASES), 13)
        self.assertIn('lower-indexed-tests', M.PHASES)
        self.assertIn('actual-inert-join', M.PHASES)
        self.assertNotIn('actual-inert-join-diagnostic', M.PHASES)
        self.assertNotEqual(M.TARGET, M.CPU / 'target')
        self.assertFalse(M.TARGET.is_relative_to(M.DIAGNOSTIC))
        self.assertEqual(M.relocate(dict(root=str(M.ORIGINAL), external='/registry/input', count=3)),
                         dict(root=str(M.COPY), external='/registry/input', count=3))


if __name__ == '__main__':
    unittest.main(verbosity=2)
