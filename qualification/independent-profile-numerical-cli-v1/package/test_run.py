"""Authored pure wiring tests; no observers, references, subprocesses or GPU run."""
import copy
import importlib.util
from pathlib import Path
import types
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('numerical_cli_draft', Path(__file__).with_name('run.py'))
M = importlib.util.module_from_spec(spec)
spec.loader.exec_module(M)


def pin(name, sha='a'):
    return dict(path='/synthetic/' + name, bytes=10, sha256=sha * 64)


def fixtures():
    S = types.SimpleNamespace(POLICY_SHA='1' * 64, PREREQUISITES=['prefix-assumption'])
    N = types.SimpleNamespace(ATTENTION_POLICY='2' * 64,
        HELPERS={'output_reference.py': '3' * 64}, PREREQUISITES=['attention-assumption'])
    evidence = {name: pin(name) for name in ('source_entry', 'reciprocal_source', 'llvm', 'isa', 'elf_notes')}
    requested = dict(tiles={'object': pin('image', 'b')}, reviews=[pin('source-review'),
        pin('formal-review'), pin('isa-review'), pin('coherence'), pin('lifecycle'), pin('devices')])
    replay = dict(receipt=dict(case='patterned-pos16', native_attempts=1, retries=0, failures=[],
            before_audits=[{}, {}, {}], after_audits=[{}, {}, {}]),
        receipt_pin=pin('gpu-complete'), request=requested, baseline=dict(producer=pin('baseline', 'c')),
        request_pin=pin('request'), binary=pin('binary'),
        case_directory=M.E / 'prefix-independent-profile-gpu-v228-v1/patterned-pos16',
        inspection_result=pin('inspection-result'), native_result=pin('native-result'))
    c = dict(inputs=dict(artifact_review=pin('artifact-review')),
        verified=dict(compiler_receipt=pin('compiler-complete'), candidate_cpu_receipt=pin('candidate-cpu'),
            arithmetic_evidence=evidence))
    review = dict(schema=M.REVIEW_SCHEMA, authority='none', reviewed=True, case='patterned-pos16',
        request=replay['request_pin'], baseline_image_sha256='c' * 64, tiles_image_sha256='b' * 64,
        artifact_review=c['inputs']['artifact_review'], source_lineage_review=requested['reviews'][0],
        isa_review=requested['reviews'][2], compiler_complete=c['verified']['compiler_receipt'],
        candidate_cpu_receipt=c['verified']['candidate_cpu_receipt'], arithmetic_evidence=evidence,
        prefix_policy_sha256=S.POLICY_SHA, attention_policy_sha256=N.ATTENTION_POLICY,
        output_reference_sha256=N.HELPERS['output_reference.py'], prefix_assumptions=S.PREREQUISITES,
        attention_output_assumptions=N.PREREQUISITES,
        notes='Synthetic explicitly scoped arithmetic assumptions review, not a universal proof.',
        remaining_limitations=['Universal sqrt and attention divide premises remain assumptions.'],
        **{key: False for key in M.REVIEW_FALSE})
    return c, replay, review, (S, object(), object(), N, None, None, None, None)


class WiringTests(unittest.TestCase):
    def test_completed_rejection_has_nonzero_exit_and_requires_explicit_boolean(self):
        self.assertEqual(M.conditional_exit_status(dict(conditional_operator_checks_passed=True)), 0)
        self.assertEqual(M.conditional_exit_status(dict(conditional_operator_checks_passed=False)), 1)
        for value in (0, 1, None, 'true'):
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                M.conditional_exit_status(dict(conditional_operator_checks_passed=value))

    def test_unfrozen_observer_refuses_before_any_host_probe(self):
        with patch.object(M, 'OBSERVER_MANIFEST_SHA', None), patch.object(M, 'envelope') as envelope:
            with self.assertRaisesRegex(RuntimeError, 'root must freeze'):
                M.main(['/synthetic/input', 'a' * 64, 'prefix-independent-numerical-v228-v1', 'b' * 64])
        envelope.assert_not_called()

    def test_frozen_observer_needs_actual_receipt_pin(self):
        with patch.object(M, 'OBSERVER_MANIFEST_SHA', 'a' * 64), patch.object(M, 'OBSERVER_TESTS', None):
            with self.assertRaises(RuntimeError):
                M.frozen_observer()
        with patch.object(M, 'OBSERVER_MANIFEST_SHA', 'a' * 64), patch.object(M, 'OBSERVER_TESTS', pin('pure')):
            M.frozen_observer()

    def test_pin_rejects_bool_extent_relative_path_and_extra_fields(self):
        M.filepin(pin('input'))
        for update in ({'bytes': True}, {'bytes': 0}, {'path': 'relative'},
            {'path': '/synthetic/../input'}, {'sha256': 'bad'}, {'extra': False}):
            with self.subTest(update=update), self.assertRaises(RuntimeError):
                M.filepin(dict(pin('input'), **update))

    def test_inputs_require_exact_scope_and_distinct_original_weight_files(self):
        value = dict(schema=M.INPUT_SCHEMA, prepared=pin('prepared'), observation=pin('observed'),
            arithmetic_review=pin('review'), output_weights=[pin('weight0'), pin('weight1')])
        M.input_shape(value)
        for update in ({'schema': 'old-parity'}, {'extra': True},
            {'output_weights': [pin('weight0'), pin('weight0')]}):
            with self.subTest(update=update), self.assertRaises(RuntimeError):
                M.input_shape(dict(value, **update))

    def test_review_binds_actual_v7_evidence_and_records_unproven_assumptions(self):
        c, replay, review, frozen = fixtures()
        self.assertIs(M.arithmetic_review(review, c, replay, frozen), review)
        self.assertFalse(review['arithmetic_prerequisites_verified'])
        self.assertFalse(review['runtime_premises_discharged'])

    def test_review_uses_baseline_producer_and_third_isa_review_not_formal(self):
        c, replay, review, frozen = fixtures()
        for update in ({'baseline_image_sha256': 'd' * 64},
            {'isa_review': replay['request']['reviews'][1]}):
            with self.subTest(update=update), self.assertRaises(RuntimeError):
                M.arithmetic_review(dict(review, **update), c, replay, frozen)

    def test_review_refuses_old_source_cpu_or_swapped_isa_pins(self):
        c, replay, review, frozen = fixtures()
        mutants = [dict(review, compiler_complete=pin('old-compiler')),
            dict(review, candidate_cpu_receipt=pin('old-cpu'))]
        changed = copy.deepcopy(review)
        changed['arithmetic_evidence']['isa'] = pin('old-isa')
        mutants.append(changed)
        for changed in mutants:
            with self.assertRaises(RuntimeError):
                M.arithmetic_review(changed, c, replay, frozen)

    def test_review_cannot_claim_universal_discharge_or_production(self):
        c, replay, review, frozen = fixtures()
        for key in M.REVIEW_FALSE:
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                M.arithmetic_review(dict(review, **{key: True}), c, replay, frozen)

    def test_review_cannot_widen_bounds_or_drop_a_prerequisite(self):
        c, replay, review, frozen = fixtures()
        for update in ({'prefix_assumptions': []}, {'attention_output_assumptions': []},
            {'prefix_policy_sha256': 'e' * 64}, {'attention_policy_sha256': 'e' * 64},
            {'output_reference_sha256': 'e' * 64}):
            with self.subTest(update=update), self.assertRaises(RuntimeError):
                M.arithmetic_review(dict(review, **update), c, replay, frozen)

    def test_review_requires_explicit_notes_limitations_and_reviewed_scope(self):
        c, replay, review, frozen = fixtures()
        for update in ({'reviewed': False}, {'authority': 'production'}, {'notes': ''},
            {'remaining_limitations': []}, {'remaining_limitations': ['']}, {'case': 'genuine-pos0'}):
            with self.subTest(update=update), self.assertRaises(RuntimeError):
                M.arithmetic_review(dict(review, **update), c, replay, frozen)

    def test_plan_uses_actual_case_paths_without_paired_parity(self):
        _, replay, _, _ = fixtures()
        weights = [pin('weight0'), pin('weight1')]
        value = M.numerical_plan(replay, weights)
        self.assertEqual(value['request'], replay['request_pin'])
        self.assertEqual(value['native_result'], replay['native_result'])
        self.assertEqual(value['case_directory'], str(replay['case_directory']))
        self.assertIs(value['output_weights'], weights)
        self.assertNotIn('bitwise_match', value)

    def comparison(self, mathematical_pass=True):
        c, replay, review, frozen = fixtures()
        reader = Mock(records={'/synthetic/read': pin('read')})
        frozen[3].Reader = Mock(return_value=reader)
        c['pins'] = Mock()
        c['P'] = types.SimpleNamespace(read=Mock())
        plan = M.numerical_plan(replay, [pin('weight0'), pin('weight1')])
        result = dict(schema='synthetic-adapter-result', case=plan['case'], request=plan['request'],
            binary=plan['binary'], inspection_result=plan['inspection_result'], native_result=plan['native_result'],
            profiles=[dict(profile='baseline_v5'), dict(profile='tiles_v6')],
            conditional_operator_checks_passed=mathematical_pass, production_authority=False)
        O = types.SimpleNamespace(RESULT_SCHEMA=result['schema'], PROFILES=('baseline_v5', 'tiles_v6'),
            FALSE_FIELDS=('production_authority',), compare_retained=Mock(return_value=result))
        return c, replay, review, frozen, plan, O, reader

    def test_comparison_delegates_unchanged_adapter_then_rechecks_both_ledgers(self):
        c, replay, review, frozen, plan, O, reader = self.comparison()
        guard = Mock()
        result = M.compare(c, replay, plan, review, O, frozen, guard)
        O.compare_retained.assert_called_once_with(plan, c['P'], c['pins'], reader,
            M.E / M.PROFILE, M.E / M.STAGE, M.E / M.SIDECAR)
        c['P'].read.assert_called_once()
        reader.recheck.assert_called_once()
        c['pins'].recheck.assert_called_once()
        self.assertEqual(guard.call_count, 2)
        self.assertTrue(result['conditional_operator_checks_passed'])

    def test_mathematical_rejection_stays_explicit_data_without_retry(self):
        c, replay, review, frozen, plan, O, _ = self.comparison(False)
        result = M.compare(c, replay, plan, review, O, frozen, Mock())
        self.assertFalse(result['conditional_operator_checks_passed'])
        self.assertEqual(O.compare_retained.call_count, 1)

    def test_missing_post_audit_or_native_failure_prevents_math(self):
        for update in ({'after_audits': [{}, {}]}, {'before_audits': []},
            {'failures': ['native error']}, {'native_attempts': 2}, {'retries': 1}):
            c, replay, review, frozen, plan, O, _ = self.comparison()
            replay['receipt'].update(update)
            with self.subTest(update=update), self.assertRaises(RuntimeError):
                M.compare(c, replay, plan, review, O, frozen, Mock())
            O.compare_retained.assert_not_called()

    def test_custody_failure_is_fatal_not_downgraded_to_numerical_rejection(self):
        c, replay, review, frozen, plan, O, reader = self.comparison()
        reader.recheck.side_effect = RuntimeError('changed original bytes')
        with self.assertRaisesRegex(RuntimeError, 'changed original bytes'):
            M.compare(c, replay, plan, review, O, frozen, Mock())

    def test_reference_authority_substitution_is_refused(self):
        c, replay, review, frozen, plan, O, _ = self.comparison()
        O.compare_retained.return_value['production_authority'] = True
        with self.assertRaisesRegex(RuntimeError, 'conditional adapter result'):
            M.compare(c, replay, plan, review, O, frozen, Mock())


if __name__ == '__main__':
    unittest.main()
