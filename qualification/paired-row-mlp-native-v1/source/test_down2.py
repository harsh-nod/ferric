"""Synthetic Down2-only image substitution and exact-output policy tests."""
import copy
import unittest

import down2 as D


def pin(name, value):
    return dict(path='/e/' + name, bytes=value[0], sha256=value[1])


def requests():
    old = dict(mode='teacher_forced', session=[1] * 32, evidence_directory='/e/old/native',
               tiles_image=pin('old.hsaco', (30000, 'a' * 64)), worker=pin('worker', (10, 'b' * 64)),
               prefix_image=pin('prefix.hsaco', (10, 'c' * 64)),
               images=dict(mlp=pin('bootstrap.hsaco', (20, 'd' * 64))),
               prompt={'tokens': [1, 2, 3, 4]}, device_ids=[123, 456], expected_model_id=[9] * 32)
    image = pin('down2.hsaco', D.IMAGE)
    new = copy.deepcopy(old)
    new.update(tiles_image=image, session=[2] * 32, evidence_directory='/e/new/native')
    return old, new, image


def lowered():
    return dict(schema='ferric-p228-down2-lowering-result-v1', passed=True, error=None, postcheck_errors=[],
                fresh_checked_lowering=True, fresh_checked_replay=True, fresh_hsaco_emitted=True,
                unresolved_runtime_requirements=8, commands=[{'name': name} for name in D.STAGES],
                artifacts={'emitted/artifact.hsaco': pin('down2.hsaco', D.IMAGE)},
                **{key: False for key in D.LOWERING_FALSE})


class Comparison:
    rows = None

    @staticmethod
    def document(value):
        return value

    @staticmethod
    def records(value):
        return value['tokens']

    @classmethod
    def compare_rows(cls, _old, _old_payload, _new, _new_payload, _helper):
        return copy.deepcopy(cls.rows)


def comparison():
    old, new, image = requests()
    prior, actual = {}, {}
    for i in range(4):
        request = dict(command={'position': i, 'token': i + 1}, device_ids=[123, 456], id=i, protocol=1)
        prior[f'request-{i}.json'] = request
        actual[f'request-{i}.json'] = copy.deepcopy(request)
        prior[f'observation-{i}.bin'] = bytes(606976)
        actual[f'observation-{i}.bin'] = bytes(606976)
    baseline = dict(observed=dict(request=old, tokens=[67, 198, 25, 16]), files=prior)
    observed = dict(request=new, tokens=[67, 198, 25, 16])
    Comparison.rows = [dict(same_input_history=True, tensors=[{'byte_equal': True} for _ in range(38)]) for _ in range(4)]
    return baseline, observed, actual, image


class Policy(unittest.TestCase):
    def test_exact_down2_tiles_image_only_is_allowed(self):
        old, new, image = requests()
        D.workload(new, old, image, lambda x: x, lambda a, b: a == b)

    def test_bootstrap_mlp_image_is_not_the_replacement_slot(self):
        old, new, image = requests()
        new['images']['mlp'] = image
        with self.assertRaises(RuntimeError):
            D.workload(new, old, image, lambda x: x, lambda a, b: a == b)

    def test_worker_prefix_prompt_device_and_model_changes_are_refused(self):
        for key in ('worker', 'prefix_image', 'prompt', 'device_ids', 'expected_model_id'):
            old, new, image = requests()
            new[key] = None
            with self.assertRaises(RuntimeError):
                D.workload(new, old, image, lambda x: x, lambda a, b: a == b)

    def test_wrong_image_extent_hash_or_path_is_refused(self):
        for key, value in (('bytes', 1), ('sha256', 'f' * 64), ('path', '/elsewhere/image')):
            old, new, image = requests()
            new['tiles_image'] = {**image, key: value}
            with self.assertRaises(RuntimeError):
                D.workload(new, old, image, lambda x: x, lambda a, b: a == b)

    def test_reused_session_or_already_down2_baseline_is_refused(self):
        for same_session in (False, True):
            old, new, image = requests()
            if same_session:
                new['session'] = old['session']
            else:
                old['tiles_image'] = image
            with self.assertRaises(RuntimeError):
                D.workload(new, old, image, lambda x: x, lambda a, b: a == b)

    def test_complete_inert_lowering_shape_keeps_all_eight_requirements(self):
        D.lowering_shape(lowered())

    def test_incomplete_lowering_or_any_authority_claim_is_refused(self):
        for key, value in (('passed', False), ('fresh_checked_replay', False),
                           ('unresolved_runtime_requirements', 0), ('postcheck_errors', ['drift']),
                           *[(key, True) for key in D.LOWERING_FALSE]):
            actual = lowered()
            actual[key] = value
            with self.assertRaises(RuntimeError):
                D.lowering_shape(actual)

    def test_changed_compiler_phase_or_artifact_is_refused(self):
        for stage in (False, True):
            actual = lowered()
            if stage:
                actual['commands'][3]['name'] = 'different-test'
            else:
                actual['artifacts']['emitted/artifact.hsaco']['sha256'] = 'f' * 64
            with self.assertRaises(RuntimeError):
                D.lowering_shape(actual)

    def test_clock_baseline_shape_requires_original_lineage_six_audits_and_no_claims(self):
        prior = pin('original-complete.json', (10, 'a' * 64))
        false = ('performance_claim', 'production_authority', 'numerical_acceptance', 'clock_domain_validated')
        value = dict(schema='ferric-p228-device-clock-gpu-v1', passed=True, failures=[],
                     native_attempts=1, retries=0, baseline=prior, policy='shared-full-currentness',
                     before_audits=[{}] * 3, after_audits=[{}] * 3, **{key: False for key in false})
        D.clock_shape(value, prior, false)
        for key, replacement in (('baseline', {}), ('passed', False), ('native_attempts', 2),
                                 ('retries', 1), ('after_audits', [{}] * 2),
                                 *[(key, True) for key in false]):
            with self.assertRaises(RuntimeError):
                D.clock_shape({**value, key: replacement}, prior, false)

    def test_separate_image_review_binds_actual_image_request_runtime_and_all_topics(self):
        names = ('down2_lowering', 'down2_image', 'clock_baseline', 'request')
        plan = {name: pin(name, (10, 'a' * 64)) for name in names}
        runtime = dict(parent=pin('parent', (20, 'b' * 64)), worker=pin('worker', (30, 'c' * 64)))
        provenance = {'source_handoff': pin('source', (40, 'd' * 64))}
        clock = dict(receipt_pin=plan['clock_baseline'])
        false = ('production_authority', 'runtime_premises_discharged', 'performance_claim', 'numerical_acceptance')
        value = dict(plan, schema='ferric-p228-down2-image-engineering-review-v1', reviewed=True,
                     authority='none', gpu_attempts=1, unresolved_runtime_requirements=8,
                     provenance=provenance, **runtime, **{key: False for key in false},
                     notes='Root separately reviews the actual emitted Down2 image.',
                     review_topics={name: 'Substantive review of the actual supplied evidence.' for name in D.REVIEW_TOPICS})
        D.review(value, plan, provenance, clock, runtime, false)
        for key, replacement in (('down2_image', {}), ('request', {}), ('parent', {}),
                                 ('provenance', {}), ('reviewed', False), ('review_topics', {}),
                                 ('unresolved_runtime_requirements', 0), *[(key, True) for key in false]):
            with self.assertRaises(RuntimeError):
                D.review({**value, key: replacement}, plan, provenance, clock, runtime, false)

    def test_exact_four_payloads_and_152_tensor_rows_are_required(self):
        baseline, observed, files, image = comparison()
        rows = D.invariance(Comparison, baseline, observed, files, {'diagnostics': None}, image,
                            lambda x: x, lambda a, b: a == b)
        self.assertEqual(sum(len(row['tensors']) for row in rows), 152)

    def test_changed_payload_is_refused_even_if_diagnostic_claims_equal(self):
        baseline, observed, files, image = comparison()
        files['observation-2.bin'] = bytes(606975) + b'\x01'
        with self.assertRaises(RuntimeError):
            D.invariance(Comparison, baseline, observed, files, {'diagnostics': None}, image,
                         lambda x: x, lambda a, b: a == b)

    def test_changed_token_or_full_forward_command_is_refused(self):
        for token in (False, True):
            baseline, observed, files, image = comparison()
            if token:
                observed['tokens'][2] += 1
            else:
                files['request-2.json']['command']['position'] = 7
            with self.assertRaises(RuntimeError):
                D.invariance(Comparison, baseline, observed, files, {'diagnostics': None}, image,
                             lambda x: x, lambda a, b: a == b)

    def test_missing_or_unequal_diagnostic_tensor_is_refused(self):
        for omitted in (False, True):
            baseline, observed, files, image = comparison()
            if omitted:
                Comparison.rows[0]['tensors'].pop()
            else:
                Comparison.rows[0]['tensors'][0]['byte_equal'] = False
            with self.assertRaises(RuntimeError):
                D.invariance(Comparison, baseline, observed, files, {'diagnostics': None}, image,
                             lambda x: x, lambda a, b: a == b)


if __name__ == '__main__':
    unittest.main(verbosity=2)
