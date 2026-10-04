"""Pure routing/refusal fixtures, not GPU or mathematical qualification."""
import copy
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import intake as I


def pin(name, digest='ab' * 32):
    return dict(path=str(I.E / name), bytes=1, sha256=digest)


def plan():
    value = {key: pin(key) for key in I.PLAN_FIELDS.split()}
    value.update(schema=I.INPUT_SCHEMA, output_label='prefix-independent-decode-tf4-baseline-gpu-v228-v1',
        policy='baseline', standalone_cases=[pin('case-' + str(i)) for i in range(6)],
        numericals=[pin('numerical-' + str(i)) for i in range(6)])
    return value


def selection():
    old = {key: pin('old-' + key) for key in ('parent', 'worker', 'image')}
    image = pin('new-image', 'cd' * 32)
    s = dict(verified=dict(image=image), prepared=dict(object=image), provenance=dict(compiler=pin('compiler')))
    return old, s


def review_fixture():
    p = plan(); old, s = selection(); runtime = I.select_runtime(old, s)
    review = dict(schema=I.REVIEW_SCHEMA, reviewed=True, authority='none', policy=p['policy'],
        historical_runtime=old, image_provenance=s['provenance'], gpu_attempts=1,
        notes='Explicit synthetic scoped engineering review fixture.',
        review_topics={key: 'Explicit synthetic scoped review for ' + key + '.' for key in I.TOPICS},
        **runtime, **{key: p[key] for key in I.REVIEW_BINDINGS}, **{key: False for key in I.REVIEW_FALSE})
    return p, old, s, runtime, review


class IntakeTests(unittest.TestCase):
    def test_closed_plan_and_each_explicit_policy(self):
        for policy in I.HC.H.POLICIES:
            value = plan(); value['policy'] = policy
            value['output_label'] = f'prefix-independent-decode-tf4-{policy}-gpu-v228-v1'
            I.input_shape(value)

    def test_old_schema_layer_prerequisite_and_extra_acceptance_refuse(self):
        for key in ('schema', 'layer_receipt', 'numerical_acceptance'):
            value = plan()
            if key == 'schema': value[key] = 'ferric-p228-group-fence-observation-inputs-v3'
            else: value[key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): I.input_shape(value)

    def test_missing_duplicate_or_unordered_scope_is_not_six_unique_inputs(self):
        for key in ('standalone_cases', 'numericals'):
            for change in ('short', 'duplicate'):
                value = plan()
                if change == 'short': value[key].pop()
                else: value[key][-1] = value[key][0]
                with self.subTest(key=key, change=change), self.assertRaises(RuntimeError): I.input_shape(value)

    def test_selection_preserves_both_historical_binary_pins_without_mutation(self):
        old, s = selection(); before = copy.deepcopy(old)
        result = I.select_runtime(old, s)
        self.assertEqual(old, before)
        self.assertEqual(result['parent'], old['parent']); self.assertEqual(result['worker'], old['worker'])
        self.assertEqual(result['image'], s['verified']['image'])
        result['parent']['path'] = '/changed'
        self.assertEqual(old, before)

    def test_selection_rejects_unchanged_or_inconsistent_image(self):
        for change in ('same', 'prepared'):
            old, s = selection()
            if change == 'same': s['verified']['image'] = s['prepared']['object'] = old['image']
            else: s['prepared']['object'] = old['image']
            with self.subTest(change=change), self.assertRaises(RuntimeError): I.select_runtime(old, s)

    def test_root_review_binds_two_deployments_and_all_six_cases(self):
        p, old, s, runtime, review = review_fixture()
        I.engineering_review(review, p, runtime, old, s)
        for key in I.REVIEW_BINDINGS:
            bad = copy.deepcopy(review); bad[key] = None
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.engineering_review(bad, p, runtime, old, s)

    def test_review_cannot_grant_authority_or_boolean_attempt_count(self):
        p, old, s, runtime, review = review_fixture()
        for key in (*I.REVIEW_FALSE, 'gpu_attempts', 'schema', 'authority'):
            bad = copy.deepcopy(review)
            bad[key] = ('old' if key in ('schema', 'authority') else True)
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.engineering_review(bad, p, runtime, old, s)

    def test_all_six_topic_notes_and_provenance_are_required(self):
        p, old, s, runtime, review = review_fixture()
        for key in ('review_topics', 'image_provenance', 'historical_runtime', 'parent', 'worker', 'image'):
            bad = copy.deepcopy(review); bad[key] = {}
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.engineering_review(bad, p, runtime, old, s)

    def test_unfrozen_package_refuses_without_reading_manifest(self):
        with patch.object(I, 'PACKAGE_FILES', None), patch.object(I.D, 'Pins') as pins:
            with self.assertRaises(RuntimeError): I.package_record(pins)
            pins.json.assert_not_called()

    def test_numerical_old_or_wrong_cohort_refuses_before_document_read(self):
        row = pin('prefix-independent-numerical-v228-v1/complete.json', 'ff' * 32)
        with patch.object(I, 'doc') as read, self.assertRaises(RuntimeError):
            I.numerical_receipt(None, row, 0, None, None, None, None, None, None)
        read.assert_not_called()

    def test_ledger_requires_exact_keys_and_rehashes_every_pin(self):
        row = pin('source')
        with patch.object(I, 'read') as read:
            I.ledger(None, {row['path']: row})
            read.assert_called_once_with(None, row, 64 << 20, False)
        for value in ({}, {'/different': row}):
            with self.assertRaises(RuntimeError): I.ledger(None, value)

    def test_native_routing_calls_only_new_observation_helper(self):
        p = plan(); old, s = selection(); runtime = I.select_runtime(old, s)
        c = dict(plan=p, runtime=runtime, selected_runtime=runtime, request=dict(mode='teacher_forced'),
            policy='baseline', C=object(), H=object(), pins=object())
        raw = {key: pin(key) for key in ('command', 'started', 'stdout', 'stderr')}
        with patch.object(I.OBS, 'observe', return_value={'synthetic': True}) as observe, \
             patch.object(I.HC, 'compare', side_effect=AssertionError('old parity called')) as parity:
            result = I.compare_native(c, {'capture': pin('capture')}, pin('sidecar'), pin('owner'), raw)
        self.assertEqual(result, {'synthetic': True}); parity.assert_not_called()
        self.assertEqual(observe.call_args.args[1]['schema'], I.OBS.INPUT_SCHEMA)
        self.assertEqual(observe.call_args.args[1]['prefix_image'], runtime['image'])
        self.assertNotIn('baseline', observe.call_args.args[1])

    def test_guard_rechecks_both_input_ledgers(self):
        pins = SimpleNamespace(recheck=unittest.mock.Mock())
        parent = SimpleNamespace(guard=unittest.mock.Mock())
        standalone = object()
        I.guard(dict(pins=pins, P=parent, standalone=standalone, runtime_reviews={}))
        pins.recheck.assert_called_once_with(); parent.guard.assert_called_once_with(standalone)


def numerical_fixture(N, O, packages, controller):
    case = 'genuine-pos0'
    image = pin('image', 'cd' * 32)
    prepared = pin('prepared'); artifact = pin('artifact'); review_pin = pin('arithmetic-review')
    receipt_pin = pin('gpu-complete'); request_pin = pin('request')
    profile_captures = {f'{tag}-rank{rank}.bin': pin(f'{tag}-rank{rank}.bin')
                       for tag in ('baseline-v5', 'tiles-v6') for rank in range(2)}
    capture_checks = dict(stage_sha256=[[['01' * 32] * 7 for _ in range(2)] for _ in range(2)])
    receipt = dict(case=case, checked=capture_checks, profile_children={}, retained_profile_files={},
                   retained_captures=profile_captures)
    requested = dict(reviews=[pin('review-' + str(i)) for i in range(6)])
    baseline = dict(producer=pin('baseline-image'))
    replay = dict(receipt=receipt, receipt_pin=receipt_pin, request_pin=request_pin, request=requested,
        baseline=baseline, binary=pin('binary'), inspection_result=pin('inspect'), native_result=pin('native'))
    verified = dict(image=image, compiler_receipt=pin('compiler'), candidate_cpu_receipt=pin('reciprocal'),
                    arithmetic_evidence={name: pin(name) for name in ('source_entry', 'reciprocal_source', 'llvm', 'isa', 'elf_notes')})
    s = dict(prepared_pin=prepared, inputs=dict(artifact_review=artifact), verified=verified)
    inputs = dict(schema=N.INPUT_SCHEMA, prepared=prepared, observation=receipt_pin, arithmetic_review=review_pin,
                  output_weights=[pin('weights-0'), pin('weights-1')])
    review = dict(schema=N.REVIEW_SCHEMA, reviewed=True, authority='none', case=case, request=request_pin,
        artifact_review=artifact, source_lineage_review=requested['reviews'][0], isa_review=requested['reviews'][2],
        compiler_complete=verified['compiler_receipt'], candidate_cpu_receipt=verified['candidate_cpu_receipt'],
        arithmetic_evidence=verified['arithmetic_evidence'], tiles_image_sha256=image['sha256'],
        baseline_image_sha256=baseline['producer']['sha256'], prefix_assumptions=['synthetic-prefix-assumption'],
        attention_output_assumptions=['synthetic-attention-assumption'], remaining_limitations=['synthetic limitation'],
        **{key: False for key in N.REVIEW_FALSE})
    policies = {name: '02' * 32 for name in ('prefix_policy_sha256', 'attention_policy_sha256', 'output_reference_sha256')}
    review.update(policies)
    false = ('arithmetic_prerequisites_verified capture_provenance_verified full_model_acceptance '
        'full_prefix_acceptance gpu_execution_verified historical_kv_numerics_checked independent_numerical_acceptance '
        'paired_comparison_performed performance_claim production_authority runtime_premises_discharged '
        'untouched_kv_bytes_checked').split()
    profiles = []
    for profile_index, name in enumerate(O.PROFILES):
        tag = ('baseline-v5', 'tiles-v6')[profile_index]
        checked = dict(schema='ferric-p228-prefix-profile-conditional-numerical-v1', case=case, profile=name,
            authority='none', conditional_operator_checks_passed=True, current_kv_append_checked=True,
            full_capture_bytes_hashed=True, required_prefix_prerequisites=review['prefix_assumptions'],
            required_attention_output_prerequisites=review['attention_output_assumptions'], input_pins={},
            rows=[dict(rank=rank, profile=name, capture=profile_captures[f'{tag}-rank{rank}.bin'],
                stage_sha256=capture_checks['stage_sha256'][profile_index][rank],
                conditioning=dict(output_weights=inputs['output_weights'][rank])) for rank in range(2)],
            **policies, **{key: False for key in false})
        profiles.append(dict(profile=name, checked=checked, error=None, conditional_operator_checks_passed=True))
    conditional = dict(schema=O.RESULT_SCHEMA, authority='none', case=case, request=request_pin,
        binary=replay['binary'], inspection_result=replay['inspection_result'], native_result=replay['native_result'],
        closed_capture_checks=capture_checks, profile_children={}, retained_profile_files={},
        conditional_operator_checks_passed=True, profiles=profiles, original_input_pins={},
        **{key: False for key in O.FALSE_FIELDS})
    record = pin('prefix-independent-numerical-v228-v1/complete.json', I.NUMERICAL_SHAS[0])
    value = dict(schema=N.SCHEMA, authority='none', case=case, inputs=pin('numerical-inputs'), observation=receipt_pin,
        arithmetic_review=review_pin, packages=packages, controller=controller, conditional=conditional,
        conditional_operator_checks_passed=True, assumptions=dict(prefix=review['prefix_assumptions'],
            attention_output=review['attention_output_assumptions'], remaining_limitations=review['remaining_limitations']),
        native_attempts_replayed=1, profile_attempts_replayed=2, retries=0, input_pins={}, elapsed_host_seconds=1.0,
        limitations=N.LIMITATIONS, **{key: False for key in N.FALSE})
    documents = {record['path']: value, value['inputs']['path']: inputs, review_pin['path']: review}
    return SimpleNamespace(record=record, value=value, s=s, replay=replay, documents=documents)


class NumericalReceiptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.N, modules, cls.packages, cls.controller, _ = I.numerical_modules(I.D.Pins())
        cls.O = modules['observe']

    def fixture(self):
        return numerical_fixture(self.N, self.O, self.packages, self.controller)

    def verify(self, f, ledger_failure=None):
        with patch.object(I, 'doc', side_effect=lambda _, record, *args: f.documents[record['path']]), \
             patch.object(I, 'ledger', side_effect=ledger_failure) as ledger:
            result = I.numerical_receipt(None, f.record, 0, f.s, f.replay,
                                       self.N, self.O, self.packages, self.controller)
            self.assertEqual(ledger.call_count, 4)
            return result

    def test_synthetic_success_checks_both_profiles_and_four_ledgers(self):
        f = self.fixture()
        self.assertIs(self.verify(f), f.value)
        self.assertFalse(f.value['independent_numerical_acceptance'])

    def test_conditional_failure_or_widened_authority_refuses(self):
        for key in ('conditional_operator_checks_passed', 'independent_numerical_acceptance', 'paired_comparison_performed'):
            f = self.fixture(); f.value[key] = key != 'conditional_operator_checks_passed'
            with self.subTest(key=key), self.assertRaises(RuntimeError): self.verify(f)

    def test_wrong_profile_rank_or_capture_refuses(self):
        for change in ('profile', 'rank', 'capture', 'policy'):
            f = self.fixture(); row = f.value['conditional']['profiles'][1]['checked']
            if change == 'profile': row['profile'] = 'baseline_v5'
            elif change == 'rank': row['rows'][1]['rank'] = True
            elif change == 'capture': row['rows'][1]['capture'] = pin('other-capture')
            else: row['prefix_policy_sha256'] = '03' * 32
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.verify(f)

    def test_actual_image_or_observer_identity_cannot_be_substituted(self):
        for change in ('image', 'observer', 'compiler'):
            f = self.fixture()
            if change == 'image': f.s['verified']['image'] = pin('wrong-image', 'ff' * 32)
            elif change == 'observer': f.value['observation'] = pin('different-observer')
            else: f.s['verified']['compiler_receipt'] = pin('different-compiler')
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.verify(f)

    def test_custody_failure_is_fatal_not_a_conditional_result(self):
        with self.assertRaisesRegex(RuntimeError, 'synthetic custody failure'):
            self.verify(self.fixture(), RuntimeError('synthetic custody failure'))


if __name__ == '__main__':
    unittest.main()
