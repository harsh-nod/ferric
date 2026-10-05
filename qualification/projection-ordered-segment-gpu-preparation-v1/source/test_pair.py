"""Closed paired-input and attribution tests; no native subprocess is launched."""
import copy
import hashlib
from pathlib import Path
import unittest

import intake as I
import pair as P
import test_host_validation as F


def pin(name):
    return dict(path='/evidence/' + name, bytes=1, sha256='ab' * 32)


def contexts():
    result = []
    for route in ('default', 'shared'):
        plan = {key: pin(key) for key in I.PLAN_FIELDS.split()}
        plan.update(schema=I.INPUT_SCHEMA, route=route,
            output_label='prefix-projection-ar4-shared-host-' + route + '-gpu-v228-v1',
            parent=pin(route + '-parent'), request=pin(route + '-request'),
            decode_review=pin(route + '-review'), parent_runtime_review=pin(route + '-runtime-review'))
        request = dict(schema='FerricFiniteProjectionResidualDecodeRequestV1',
            projection_residual_image=pin('projection'),
            decode=dict(mode='autoregressive', session=[1 if route == 'default' else 2] * 32,
                evidence_directory='/evidence/' + route + '/native', worker=pin('worker'),
                prompt={'seed': 9112}, expected_model_id=18446744073709551615,
                prefix_image=pin('prefix'), tiles_image=pin('mlp'), device_ids=[1, 2]))
        result.append(dict(route=route, plan=plan, request=request, out=Path('/evidence/' + plan['output_label'])))
    return result


def observations():
    f = F.fixture(); arms = []
    for route in ('default', 'shared'):
        shared = route == 'shared'
        host = {key: copy.deepcopy(f['report'][key]) for key in
                ('snapshots', 'intervals', 'forward_host_ns', 'serialization_host_ns', 'close_host_ns')}
        host['schema'] = 'ferric-p228-projection-ar4-' + ('shared-host' if shared else 'host') + '-counters-v1'
        for row in host['snapshots']: row['shared_full_currentness'] = shared
        if shared:
            host.update(policy='shared-full', configuration_time_in_snapshots=False, configuration_host_ns=1234)
        arms.append(dict(schema='ferric-p228-projection-ar4-shared-host-gpu-v1', route=route, passed=True,
            failures=[], native_attempts=1, retries=0, captured_payloads=4, captured_tensor_rows=152,
            own_output_trajectory_checked=True, performance_claim=False, parent_cpu_complete=pin('cpu'),
            worker_cpu_complete=pin('cpu'), worker=pin('worker'), projection_image=pin('projection'),
            prefix_image=pin('prefix'), mlp_image=pin('mlp'), host_checked=host))
    return arms + [copy.deepcopy(f['observed']), copy.deepcopy(f['observed'])]


class PairTests(unittest.TestCase):
    def test_pair_plan_has_only_two_explicit_arm_pins_and_fresh_namespace(self):
        value = dict(schema=P.PAIR_SCHEMA, output_label='prefix-projection-ar4-shared-host-pair-v228-v1',
                     default=pin('default'), shared=pin('shared'))
        P.plan_shape(value)
        for change in ({'extra': True}, {'schema': I.INPUT_SCHEMA}, {'shared': value['default']},
                       {'output_label': 'old-case'}):
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                P.plan_shape(dict(value, **change))

    def test_same_inputs_preserve_lossless_model_identity_and_distinct_sessions(self):
        a, b = contexts(); P.same_inputs(a, b)
        self.assertEqual(a['request']['decode']['expected_model_id'], 18446744073709551615)
        for key in ('session', 'evidence_directory'):
            bad = copy.deepcopy(b); bad['request']['decode'][key] = a['request']['decode'][key]
            with self.subTest(key=key), self.assertRaises(RuntimeError): P.same_inputs(a, bad)

    def test_pair_rejects_cross_generation_images_worker_or_pure_evidence(self):
        a, b = contexts()
        for key in ('parent_cpu', 'worker_cpu', 'worker', 'prefix_image', 'projection_image', 'mlp_image', 'supervisor_tests'):
            bad = copy.deepcopy(b); bad['plan'][key] = pin('changed')
            with self.subTest(key=key), self.assertRaises(RuntimeError): P.same_inputs(a, bad)
        bad = copy.deepcopy(b); bad['route'] = 'default'
        with self.assertRaises(RuntimeError): P.same_inputs(a, bad)

    def test_same_model_prompt_devices_and_request_family_are_required(self):
        a, b = contexts()
        for key, changed in (('prompt', {'seed': 67}), ('device_ids', [2, 1]), ('mode', 'teacher_forced'),
                             ('expected_model_id', 18446744073709551614)):
            bad = copy.deepcopy(b); bad['request']['decode'][key] = changed
            with self.subTest(key=key), self.assertRaises(RuntimeError): P.same_inputs(a, bad)

    def test_numerator_contains_only_rank_group_and_publication_full_time(self):
        interval = dict(ranks=[[999] * 19, [999] * 19], shared=[999, 17, 999, 19])
        interval['ranks'][0][3] = 7; interval['ranks'][1][3] = 11
        result = P.full_currentness(interval)
        self.assertEqual(result, dict(rank_full_ns=18, group_full_ns=17, publication_full_ns=19, component_sum_ns=54))
        interval['ranks'][0][1] = 999999999
        self.assertEqual(P.full_currentness(interval)['component_sum_ns'], 54)

    def test_comparison_records_matching_histories_and_honest_repeatability(self):
        a, b, x, y = observations(); result = P.comparison(a, b, x, y, [True] * 4)
        self.assertEqual(len(result['intervals']), 6)
        self.assertTrue(result['all_payloads_byte_equal'])
        self.assertEqual(result['numerator'], 'rank_full_ns + group_full_ns + publication_full_ns')
        self.assertFalse(result['performance_claim']); self.assertFalse(result['numerical_acceptance'])
        self.assertFalse(result['components_are_disjoint_latency'])

    def test_different_actual_own_output_history_refuses_attribution(self):
        a, b, x, y = observations()
        y['files']['frames'][2]['response']['event']['input_token'] += 1
        with self.assertRaises(RuntimeError): P.comparison(a, b, x, y, [True] * 4)

    def test_actual_generation_or_image_mismatch_cannot_pass_plan_only(self):
        for key in ('parent_cpu_complete', 'worker_cpu_complete', 'worker', 'prefix_image', 'mlp_image', 'projection_image'):
            a, b, x, y = observations(); b[key] = pin('other')
            with self.subTest(key=key), self.assertRaises(RuntimeError): P.comparison(a, b, x, y, [True] * 4)

    def test_every_snapshot_and_distinct_host_schema_must_agree(self):
        for index in range(7):
            a, b, x, y = observations(); b['host_checked']['snapshots'][index]['shared_full_currentness'] = False
            with self.subTest(index=index), self.assertRaises(RuntimeError): P.comparison(a, b, x, y, [True] * 4)
        a, b, x, y = observations(); b['host_checked']['schema'] = a['host_checked']['schema']
        with self.assertRaises(RuntimeError): P.comparison(a, b, x, y, [True] * 4)

    def test_payload_inequality_is_not_independent_numerical_acceptance(self):
        a, b, x, y = observations(); result = P.comparison(a, b, x, y, [True, False, True, True])
        self.assertFalse(result['all_payloads_byte_equal'])
        self.assertFalse(result['numerical_acceptance'])
        self.assertIn('not independent', result['payload_equality_scope'])

    def test_payload_acquisition_retains_bytes_before_comparing(self):
        bodies, calls, arms = {}, [], []
        for role in ('default', 'shared'):
            retained = {}
            for index in range(4):
                name = 'observation-' + str(index) + '.bin'
                raw = bytes([index, 1 if role == 'shared' and index == 1 else 0])
                record = dict(path='/evidence/' + role + '/' + name, bytes=len(raw),
                              sha256=hashlib.sha256(raw).hexdigest())
                retained[name] = record
                bodies[record['path']] = (record, raw)
            arms.append(dict(retained_native=retained))

        class Pins:
            def read(self, path, expected_sha, retain, maximum):
                record, raw = bodies[str(path)]
                if record['sha256'] != expected_sha or len(raw) > maximum:
                    raise RuntimeError('bad synthetic pin')
                calls.append(retain)
                return record, raw if retain else b''

        self.assertEqual(P.payload_repeatability(Pins(), *arms), [True, False, True, True])
        self.assertEqual(calls, [True] * 8)

    def test_configuration_duration_is_separate_from_interval_numerator(self):
        a, b, x, y = observations(); b['host_checked']['configuration_host_ns'] = (1 << 64) - 1
        result = P.comparison(a, b, x, y, [True] * 4)
        self.assertEqual(result['configuration_host_ns'], {'default': None, 'shared': (1 << 64) - 1})
        self.assertTrue(all(row['component_sum_delta_ns'] == 0 for row in result['intervals']))
        self.assertFalse(result['configuration_time_in_snapshots'])

    def test_failed_arm_or_retry_cannot_become_paired_success(self):
        for key, value in (('passed', False), ('failures', ['error']), ('native_attempts', 2), ('retries', 1)):
            a, b, x, y = observations(); b[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): P.comparison(a, b, x, y, [True] * 4)


if __name__ == '__main__':
    unittest.main(verbosity=2)
