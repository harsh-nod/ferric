"""Synthetic V2 clock records; reuse real raw validation, never invoke native code."""
import copy
import hashlib
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import device_validation as D
import test_raw_validation as T


def fixture():
    raw, observed, files, validator = T.fixture()
    samples = []
    for index in range(16):
        position, slot = divmod(index, 4)
        rank, post = slot % 2, slot >= 2
        samples.append(dict(generation=position + 1, position=position,
            endpoint='post' if post else 'pre', rank=rank,
            row_boundary=(position + int(post)) * D.PER_FORWARD,
            group_incarnation=raw['group_incarnation'], unique_id=11 + rank, queue_epoch=20 + rank,
            gpu_id=rank, gpu_clock_counter=index * 13, cpu_clock_counter=index * 17,
            system_clock_counter=index * 19, system_clock_frequency_hz=1000000000 + rank,
            host_started_ns=index * 10, host_finished_ns=index * 10 + 5))
    report = dict(schema='FerricPrefixDecodeDeviceClockObservationV2', raw=raw, samples=samples,
        raw_clock_counters=True, **{key: False for key in D.FALSE})
    return report, observed, files, validator


def wrapper_fixture():
    value, observed, files, validator = fixture()
    summary = T.encoded(observed) + b'\n'
    parent = dict(path='/clock-parent', bytes=10, sha256='a' * 64)
    raw = T.encoded(value)
    sidecar = dict(path='/case/native-device-clock-v2.json', bytes=len(raw),
                   sha256=hashlib.sha256(raw).hexdigest())
    external = dict(schema=D.REQUEST_SCHEMA, decode=observed['request'])
    wrapper = dict(schema='FerricFinitePrefixDecodeDeviceClockDiagnosticV2', parent_binary=parent,
        request_projection_sha256=list(hashlib.sha256(T.encoded(external)).digest()),
        observation=observed, device_sidecar=sidecar, raw_completion_ticks=True, raw_clock_counters=True,
        **{key: False for key in D.FALSE})
    args = (raw, sidecar, summary, observed, parent, external, files, validator)
    return wrapper, args


class ClockValidationTests(unittest.TestCase):
    def test_all16_samples_wrap_complete1172_raw_rows_and_actual_controls(self):
        value, observed, files, validator = fixture()
        actual = D.report(T.encoded(value), observed, files, validator)
        self.assertEqual(actual, value)
        self.assertEqual((len(actual['samples']), len(actual['raw']['rows'])), (16, 1172))
        self.assertEqual([sample['row_boundary'] for sample in actual['samples']],
            [0, 0, 293, 293, 293, 293, 586, 586, 586, 586, 879, 879, 879, 879, 1172, 1172])

    def test_raw_object_is_checked_directly_without_legacy_serialization(self):
        value, observed, files, validator = fixture()
        with patch.object(D.RV, 'report_value', wraps=D.RV.report_value) as checked:
            result = D.report(T.encoded(value), observed, files, validator)
        checked.assert_called_once()
        self.assertIs(checked.call_args.args[0], result['raw'])
        self.assertIs(checked.call_args.args[1], observed)
        self.assertIs(checked.call_args.args[2], files)
        self.assertIs(checked.call_args.args[3], validator)

    def test_missing_extra_or_reordered_samples_are_refused(self):
        value, observed, files, validator = fixture()
        for change in ('missing', 'extra', 'order'):
            bad = copy.deepcopy(value)
            if change == 'missing': bad['samples'].pop()
            elif change == 'extra': bad['samples'].append(bad['samples'][-1])
            else: bad['samples'][2], bad['samples'][3] = bad['samples'][3], bad['samples'][2]
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                D.report(T.encoded(bad), observed, files, validator)

    def test_each_endpoint_boundary_and_native_identity_is_bound(self):
        value, observed, files, validator = fixture()
        for index in (0, 7, 15):
            for key in ('generation', 'position', 'rank', 'row_boundary', 'group_incarnation',
                        'unique_id', 'queue_epoch', 'endpoint'):
                bad = copy.deepcopy(value)
                bad['samples'][index][key] = 'invalid' if key == 'endpoint' else bad['samples'][index][key] + 1
                with self.subTest(index=index, key=key), self.assertRaises(RuntimeError):
                    D.report(T.encoded(bad), observed, files, validator)

    def test_all_clock_integer_fields_refuse_boolean_float_negative_and_overflow(self):
        value, observed, files, validator = fixture()
        for key in set(value['samples'][0]) - {'endpoint'}:
            for invalid in (True, 0.0, -1, 1 << 64):
                bad = copy.deepcopy(value); bad['samples'][0][key] = invalid
                with self.subTest(key=key, invalid=invalid), self.assertRaises(RuntimeError):
                    D.report(T.encoded(bad), observed, files, validator)
        bad = copy.deepcopy(value); bad['samples'][0]['gpu_id'] = 1 << 32
        with self.assertRaises(RuntimeError): D.report(T.encoded(bad), observed, files, validator)

    def test_same_device_ids_frequencies_and_nonoverlapping_host_brackets(self):
        value, observed, files, validator = fixture()
        for index, key, changed in ((1, 'gpu_id', 0), (2, 'gpu_id', 8),
                (15, 'system_clock_frequency_hz', 1), (0, 'system_clock_frequency_hz', 0),
                (3, 'host_started_ns', 24), (15, 'host_finished_ns', 149)):
            bad = copy.deepcopy(value); bad['samples'][index][key] = changed
            with self.subTest(index=index, key=key), self.assertRaises(RuntimeError):
                D.report(T.encoded(bad), observed, files, validator)

    def test_zero_and_decreasing_raw_counters_do_not_imply_clock_validation(self):
        value, observed, files, validator = fixture()
        for index, sample in enumerate(value['samples']):
            sample.update(gpu_clock_counter=15 - index, cpu_clock_counter=0,
                          system_clock_counter=(1 << 64) - 1 if index == 0 else 0)
        result = D.report(T.encoded(value), observed, files, validator)
        self.assertTrue(result['raw_clock_counters'])
        for key in D.FALSE: self.assertIs(result[key], False)

    def test_closed_outer_schema_and_all_authority_flags(self):
        value, observed, files, validator = fixture()
        for key, invalid in (('schema', 'FerricPrefixDecodeDeviceObservationV1'), ('extra', False),
                ('raw_clock_counters', 1), *[(key, True) for key in D.FALSE],
                *[(key, 0) for key in D.FALSE]):
            with self.subTest(key=key, invalid=invalid), self.assertRaises(RuntimeError):
                D.report(T.encoded(dict(value, **{key: invalid})), observed, files, validator)
        bad = copy.deepcopy(value); bad['samples'][0]['extra'] = 0
        with self.assertRaises(RuntimeError): D.report(T.encoded(bad), observed, files, validator)

    def test_nested_raw_policy_control_and_completion_failures_are_not_hidden(self):
        value, observed, files, validator = fixture()
        for change in ('ticks', 'control', 'completion', 'policy', 'capture'):
            bad = copy.deepcopy(value)
            if change == 'ticks': bad['raw']['rows'][0]['start_tick'] = 0
            elif change == 'control': bad['raw']['rows'][-1]['host_elapsed_ns'] ^= 1
            elif change == 'completion': bad['raw']['completions'][-1]['generation'] = 1
            elif change == 'policy': bad['raw']['shared_full_currentness'] = False
            else: bad['raw']['transcript_sha256'] = [0] * 32
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                D.report(T.encoded(bad), observed, files, validator)
        with self.assertRaises(RuntimeError):
            D.report(T.encoded(value), observed, files, SimpleNamespace(control=lambda raw: 0))

    def test_duplicate_fields_size_limits_and_legacy_fallback_refused(self):
        value, observed, files, validator = fixture()
        for raw in (b'', b' ' * (D.MAX_BYTES + 1), T.encoded(value['raw']),
                    b'{"schema":0,' + T.encoded(value)[1:]):
            with self.assertRaises(RuntimeError): D.report(raw, observed, files, validator)
        for request in (dict(schema='FerricFinitePrefixDecodeDeviceRequestV1', decode={}),
                        dict(schema=D.REQUEST_SCHEMA, decode={}, policy='shared-full-currentness')):
            with self.assertRaises(RuntimeError): D.request(request)

    def test_clock_parent_wrapper_projection_path_and_claims(self):
        wrapper, args = wrapper_fixture(); C = SimpleNamespace(pin=lambda value: value)
        actual = D.validate(C, T.encoded(wrapper) + b'\n', *args)
        self.assertEqual(len(actual['samples']), 16)
        for key, invalid in (('schema', 'FerricFinitePrefixDecodeDeviceDiagnosticV1'),
                ('request_projection_sha256', [0] * 32), ('raw_clock_counters', False),
                ('parent_binary', dict(wrapper['parent_binary'], bytes=11)),
                ('device_sidecar', dict(wrapper['device_sidecar'], path='/wrong')),
                *[(key, True) for key in D.FALSE]):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                D.validate(C, T.encoded(dict(wrapper, **{key: invalid})) + b'\n', *args)
        bad = copy.deepcopy(wrapper); bad['observation']['child_pid'] += 1
        with self.assertRaises(RuntimeError): D.validate(C, T.encoded(bad) + b'\n', *args)

    def test_full_clock_sidecar_is_charged_to_original_aggregate_budget(self):
        wrapper, args = wrapper_fixture(); C = SimpleNamespace(pin=lambda value: value)
        raw, sidecar, _, observed, parent, external, files, validator = args
        for delta in (0, 1):
            changed = copy.deepcopy(observed)
            changed['files']['bytes_before_summary'] = (8 << 20) - 65536 - len(raw) + delta
            summary = T.encoded(changed) + b'\n'; w = dict(wrapper, observation=changed)
            call = lambda: D.validate(C, T.encoded(w) + b'\n', raw, sidecar, summary,
                                      changed, parent, external, files, validator)
            if delta == 0: call()
            else:
                with self.assertRaises(RuntimeError): call()


if __name__ == '__main__': unittest.main()
