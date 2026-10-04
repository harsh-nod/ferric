"""Synthetic controller contracts only; no compiler, subprocess or native calls."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('kir_diagnostic_cpu', Path(__file__).with_name('run.py'))
M = importlib.util.module_from_spec(spec)
spec.loader.exec_module(M)


class DiagnosticTests(unittest.TestCase):
    def counter(self, stage, work=100, charge=0, peak=32, storage=16):
        return (f'kir-join-work-v1 stage={stage} work={work} remaining={M.WORK_LIMIT - work} '
                f'storage={storage} peak={peak} charge={charge}')

    def prefix(self, count):
        rows, work = [], 100
        for stage in M.STAGES[:count]:
            if stage == 'join-shape':
                rows.append('kir-join-work-v1 stage=join-shape rows=2 blocks=1 nodes=3 charge=1536')
                continue
            stem = stage.rsplit('-', 1)[0]
            charge = 1536 if stage == 'layout-join-before' else 10 if stage.endswith('-before') and stem in M.BULK else 0
            rows.append(self.counter(stage, work, charge))
            if charge:
                work += charge
        return rows

    def observe(self, rows, **outcome):
        text = (f'test {M.SELECTOR} ... FAILED\n' + '\n'.join(rows)
                + f'\nCanonicalKernelIrWorkLimitV1 {{ actual: {M.WORK_ACTUAL}, limit: {M.WORK_LIMIT} }}\n'
                + 'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 204 filtered out;\n')
        result = dict(exit_code=101, reason=None, group_absent=True)
        result.update(outcome)
        return M.diagnostic_result(text, '', result)

    def test_any_nested_prefix_is_localization_not_inferred_failure(self):
        for count in (1, 2, 3, 5, 7, 8, 9, 10, 11, 12, 14, 15, 17, 18, 19, 21):
            result = self.observe(self.prefix(count))
            self.assertFalse(result['failure_phase_inferred'])
            self.assertFalse(result['actual_capture_join_passed'])
            self.assertEqual(result['last_observed_stage'], M.STAGES[count - 1])

    def test_refused_bulk_joins_actual_attempted_work(self):
        rows = self.prefix(4)
        rows[-1] = self.counter('formal-archive-before', charge=M.WORK_ACTUAL - 100)
        self.assertTrue(self.observe(rows)['expected_refusal_observed'])
        rows[-1] = self.counter('formal-archive-before', charge=M.WORK_ACTUAL - 101)
        with self.assertRaises(RuntimeError): self.observe(rows)

    def test_missing_duplicate_and_reordered_markers_refused(self):
        for rows in ([], self.prefix(2)[1:], [*self.prefix(1), *self.prefix(1)], self.prefix(3)[::-1]):
            with self.subTest(rows=rows), self.assertRaises(RuntimeError): self.observe(rows)

    def test_marker_shape_unknown_field_and_stage_refused(self):
        for row in (self.counter('unknown-before'), self.counter(M.STAGES[0]) + ' extra=1',
                    self.counter(M.STAGES[0]).replace('work=100', 'work=-1')):
            with self.subTest(row=row), self.assertRaises(RuntimeError): self.observe([row])

    def test_limits_storage_and_pending_charge_refused(self):
        for row in (self.counter(M.STAGES[0], peak=M.STORAGE_LIMIT + 1),
                    self.counter(M.STAGES[0], storage=33), self.counter(M.STAGES[0], charge=1),
                    self.counter(M.STAGES[0]).replace('remaining=1073741724', 'remaining=0')):
            with self.subTest(row=row), self.assertRaises(RuntimeError): self.observe([row])

    def test_work_and_peak_regression_refused(self):
        for row in (self.counter(M.STAGES[1], work=99), self.counter(M.STAGES[1], peak=31)):
            with self.subTest(row=row), self.assertRaises(RuntimeError): self.observe([self.counter(M.STAGES[0]), row])

    def test_successful_bulk_delta_checked(self):
        rows = self.prefix(5)
        self.observe(rows)
        rows[-1] = self.counter('formal-archive-after', work=111)
        with self.assertRaises(RuntimeError): self.observe(rows)

    def test_shape_formula_and_next_charge_checked(self):
        rows = self.prefix(17)
        self.observe(rows)
        for index, replacement in ((14, rows[14].replace('charge=1536', 'charge=1537')),
                                   (15, rows[15].replace('charge=1536', 'charge=1537'))):
            changed = list(rows); changed[index] = replacement
            with self.subTest(index=index), self.assertRaises(RuntimeError): self.observe(changed)

    def test_leaf_must_be_natural101_and_group_absent(self):
        for outcome in (dict(exit_code=0), dict(exit_code=True), dict(reason='timeout'), dict(group_absent=False)):
            with self.subTest(outcome=outcome), self.assertRaises(RuntimeError): self.observe(self.prefix(1), **outcome)
        with self.assertRaises(RuntimeError): M.diagnostic_result('unrelated failed test', '', dict(exit_code=101, reason=None, group_absent=True))

    def source_fixture(self):
        before = {name: dict(bytes=1, sha256='1' * 64) for name in M.FILES}
        before['Cargo.toml'] = dict(bytes=2, sha256='2' * 64)
        rows = [dict(path=name, source='draft/' + name, before=before[name], after=dict(bytes=3, sha256='3' * 64)) for name in sorted(M.FILES)]
        after = dict(before); after.update({row['path']: row['after'] for row in rows})
        return before, after, dict(files=rows)

    def test_source_overlay_and_formatter_only_three_bodies(self):
        before, after, proposal = self.source_fixture()
        M.transition(before, after, proposal)
        for name in M.FILES: after[name] = dict(bytes=4, sha256='4' * 64)
        M.transition(before, after, proposal, formatted=True)
        after['Cargo.toml'] = dict(bytes=4, sha256='4' * 64)
        with self.assertRaises(RuntimeError): M.transition(before, after, proposal, formatted=True)

    def test_wrong_source_preimage_and_new_member_refused(self):
        before, after, proposal = self.source_fixture()
        changed = copy.deepcopy(proposal); changed['files'][0]['before']['bytes'] += 1
        with self.assertRaises(RuntimeError): M.transition(before, after, changed)
        after['new.rs'] = dict(bytes=1, sha256='5' * 64)
        with self.assertRaises(RuntimeError): M.transition(before, after, proposal)

    def test_dependency_subset_preserves_full_tree_separately(self):
        old = {str(M.ORIGINAL / 'crates/a.rs'): {'old': True}, '/registry/input': {'pin': 1}}
        current = {str(M.COPY / 'crates/a.rs'): {'new': True}, '/registry/input': {'pin': 1}}
        sources = {str(M.COPY / 'crates/a.rs'): {'new': True}, str(M.COPY / 'Cargo.toml'): {'workspace': True}}
        M.dependency_transition(old, current, sources)
        for changed in ({**current, '/registry/input': {'pin': 2}}, {**current, str(M.COPY / 'extra.rs'): {}}):
            with self.assertRaises(RuntimeError): M.dependency_transition(old, changed, sources)
        with self.assertRaises(RuntimeError): M.dependency_transition(old, current, {**sources, str(M.COPY / 'crates/a.rs'): {}})

    def test_metadata_relocation_does_not_change_other_values(self):
        value = dict(root=str(M.ORIGINAL), package=['path+' + str(M.ORIGINAL) + '#test@1'], external='/registry/input', count=3)
        expected = dict(root=str(M.COPY), package=['path+' + str(M.COPY) + '#test@1'], external='/registry/input', count=3)
        self.assertEqual(M.relocate(value), expected)

    def environment_fixture(self):
        env = dict(CARGO_TARGET_DIR='new-target', TMPDIR='new-tmp', LD_LIBRARY_PATH='new-deps', GPU='hidden')
        prefix = 'FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_'
        old = dict(env, **{prefix + 'PATH': M.HANDOFF['path'], prefix + 'BYTES': str(M.HANDOFF['bytes']), prefix + 'SHA256': M.HANDOFF['sha256']})
        old.update(CARGO_TARGET_DIR='old-target', TMPDIR='old-tmp', LD_LIBRARY_PATH='old-deps')
        return env, dict(env=old, argv=['old-ELF', '--exact', M.SELECTOR, '--ignored', '--show-output', '--test-threads=1'])

    def test_environment_changes_only_owned_paths_and_opt_in(self):
        env, previous = self.environment_fixture()
        actual = M.selected_environment(previous, env)
        self.assertEqual(actual[M.DIAGNOSTIC_ENV], '1')
        self.assertEqual(actual['GPU'], 'hidden')
        with self.assertRaises(RuntimeError): M.selected_environment(previous, dict(env, GPU='visible'))

    def test_handoff_and_selector_cannot_change(self):
        env, previous = self.environment_fixture()
        changed = copy.deepcopy(previous); changed['argv'][2] += '_other'
        with self.assertRaises(RuntimeError): M.selected_environment(changed, env)
        changed = copy.deepcopy(previous); changed['env']['FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_BYTES'] = '1'
        with self.assertRaises(RuntimeError): M.selected_environment(changed, env)

    def test_proposal_requires_actual_generation_and_unchanged_scope(self):
        cpu, source = {'actual': 'cpu'}, {'actual': 'source'}
        proposal = dict(schema='ferric-p228-kir-join-work-diagnostic-source-v1', base_cpu=cpu,
            base_source_snapshot=source, diagnostic_only=True, compiled=False, tests_executed=False,
            added_tests=[], work_limit=M.WORK_LIMIT, storage_limit=M.STORAGE_LIMIT, handoff=M.HANDOFF,
            diagnostic_environment={M.DIAGNOSTIC_ENV: '1'}, expected_refusal=dict(selector=M.SELECTOR,
                natural_exit_code=101, actual_work=M.WORK_ACTUAL, work_limit=M.WORK_LIMIT))
        M.proposal_contract(proposal, cpu, source)
        for key, value in (('base_cpu', {}), ('base_source_snapshot', {}), ('compiled', True),
                           ('added_tests', ['new']), ('work_limit', M.WORK_LIMIT + 1), ('handoff', {})):
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.proposal_contract(dict(proposal, **{key: value}), cpu, source)


if __name__ == '__main__':
    unittest.main(verbosity=2)
