"""Synthetic timing/parity boundary tests, separate from any native outcome."""
import copy
import hashlib
import json
import unittest

import test_readiness as R
import validate_timing as T


def timing_fixture():
    value = R.fixture()
    summary, request, bodies, tokens = value
    raw = R.summary_bytes(summary)
    checked = R.V.validate(raw, request, R.DIRECTORY, lambda p: bodies[p['path']], tokens)
    pin = dict(path=str(R.DIRECTORY / 'complete.json'), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    cursor = 0
    def span():
        nonlocal cursor
        start = cursor; cursor += 7
        return dict(start_ns=start, end_ns=cursor, elapsed_ns=7)
    timeline = dict(source_preparation=span(), spawn_to_setup_seal=span(), forwards=[])
    for position in range(40):
        timeline['forwards'].append(dict(position=position, generation=position + 1,
            prepare_write=span(), flush_to_frame_read=span(), validate_retain_commit=span(), elapsed_ns=21))
    timeline.update(close_and_retirement=span(), postcheck_and_ordinary_publication=span(), total_ns=cursor)
    report = dict(schema='FerricReadiness40Position5ParentHostTimingV1',
        ordinary_complete=R.pin(R.DIRECTORY / 'complete.json', raw),
        profile_sha256=summary['profile_sha256'], transcript_sha256=summary['transcript_sha256'],
        completed_forwards=40, generated_tokens=0, capture_positions=[0, 5, 16, 39], timeline=timeline,
        native_closed=True, child_exit_zero=True, process_group_absent=True, parent_host_measurement=True,
        **{name: False for name in T.FALSE_FIELDS})
    wrapper = dict(schema='FerricReadiness40Position5TimedObservationV1', observation=summary,
        host_timing=dict(complete=True, file=None, ordinary_retained_bytes=summary['files']['total_bytes'],
            retained_bytes_with_timing=0, supervisor_metadata_allowance=summary['files']['supervisor_metadata_allowance'],
            parent_host_measurement=True, gpu_timing=False, numerical_acceptance=False, performance_claim=False))
    return dict(wrapper=wrapper, report=report, summary=raw, pin=pin, checked=checked, bodies=bodies)


def encode_case(case):
    raw = R.wire(case['report']) + b'\n'
    status = case['wrapper']['host_timing']
    status['file'] = R.pin(R.DIRECTORY / 'host-timing.json', raw)
    status['retained_bytes_with_timing'] = status['ordinary_retained_bytes'] + len(raw)
    case['bodies'][status['file']['path']] = raw


def check(case):
    return T.validate(R.wire(case['wrapper']), case['summary'], case['pin'], case['checked'],
        R.DIRECTORY, lambda pin: case['bodies'][pin['path']])


def parity_fixture(case):
    current = json.loads(case['summary'])
    baseline = json.loads(case['summary'].replace(b'/synthetic/readiness/native', b'/synthetic/control/native'))
    baseline['request']['base']['session'] = [9] * 32
    bodies = dict(case['bodies'])
    for path, raw in case['bodies'].items():
        bodies[path.replace('/synthetic/readiness/native', '/synthetic/control/native')] = raw
    return current, baseline, bodies


class TimingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = timing_fixture()

    def rejected(self, edit, after=False):
        case = copy.deepcopy(self.base)
        before = R.wire(case['report']) + R.wire(case['wrapper'])
        if after: encode_case(case)
        edit(case)
        if not after: encode_case(case)
        self.assertNotEqual(before, R.wire(case['report']) + R.wire(case['wrapper']))
        with self.assertRaises((ValueError, KeyError, TypeError)):
            check(case)

    def test_complete_timeline_zero_spans_and_large_integer_precision(self):
        case = copy.deepcopy(self.base); encode_case(case)
        got = check(case)
        self.assertEqual(got['disjoint_spans'], 124)
        self.assertEqual(got['timeline']['total_ns'], 868)
        self.assertFalse(got['gpu_timing'])
        timeline = copy.deepcopy(got['timeline'])
        for row in timeline['forwards']:
            row['elapsed_ns'] = 0
            for key in ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit'):
                row[key] = dict(start_ns=0, end_ns=0, elapsed_ns=0)
        for key in ('source_preparation', 'spawn_to_setup_seal', 'close_and_retirement', 'postcheck_and_ordinary_publication'):
            timeline[key] = dict(start_ns=0, end_ns=0, elapsed_ns=0)
        timeline['total_ns'] = 0; T.timeline(timeline)
        timeline['postcheck_and_ordinary_publication'] = dict(start_ns=0, end_ns=(1 << 64) - 1, elapsed_ns=(1 << 64) - 1)
        timeline['total_ns'] = (1 << 64) - 1
        self.assertEqual(T.timeline(timeline)['total_ns'], (1 << 64) - 1)

    def test_strict_u64_refuses_bool_float_negative_and_overflow(self):
        for value in (True, 1.0, -1, 1 << 64):
            with self.subTest(value=value):
                self.rejected(lambda c: c['report']['timeline'].__setitem__('total_ns', value))

    def test_gaps_overlap_regression_and_duration_are_refused(self):
        for field, value in (('start_ns', 15), ('start_ns', 13), ('end_ns', 0), ('elapsed_ns', 8)):
            with self.subTest(field=field, value=value):
                self.rejected(lambda c: c['report']['timeline']['forwards'][0]['prepare_write'].__setitem__(field, value))

    def test_order_extent_and_forward_sum_are_closed(self):
        for field, value in (('position', 2), ('generation', 0), ('elapsed_ns', 22)):
            with self.subTest(field=field):
                self.rejected(lambda c: c['report']['timeline']['forwards'][0].__setitem__(field, value))
        self.rejected(lambda c: c['report']['timeline']['forwards'].pop())
        self.rejected(lambda c: c['report']['timeline'].__setitem__('extra', 0))

    def test_wrapper_and_ordinary_admission_are_not_interchangeable(self):
        self.rejected(lambda c: c['wrapper'].__setitem__('schema', 'FerricGuardedMlpReadiness40Position5ObservationV1'))
        self.rejected(lambda c: c['wrapper']['observation'].__setitem__('child_pid', 102))
        self.rejected(lambda c: c['wrapper'].__setitem__('extra', False))
        case = copy.deepcopy(self.base); encode_case(case)
        case['checked']['schema'] = 'unadmitted'
        with self.assertRaises(ValueError): check(case)

    def test_sidecar_actual_body_path_hash_and_size_are_joined(self):
        for field, value in (('path', '/wrong/host-timing.json'), ('bytes', 65537), ('sha256', [1] * 32)):
            with self.subTest(field=field):
                self.rejected(lambda c: c['wrapper']['host_timing']['file'].__setitem__(field, value), after=True)
        self.rejected(lambda c: c['report']['ordinary_complete'].__setitem__('sha256', [1] * 32))

    def test_report_hashes_scope_and_capture_set_are_joined(self):
        for field, value in (('profile_sha256', [1] * 32), ('transcript_sha256', [1] * 32),
                ('completed_forwards', 39), ('generated_tokens', 1), ('capture_positions', [0, 15, 16, 39])):
            with self.subTest(field=field): self.rejected(lambda c: c['report'].__setitem__(field, value))

    def test_close_and_false_authority_are_required(self):
        for field in ('native_closed', 'child_exit_zero', 'process_group_absent', 'parent_host_measurement'):
            with self.subTest(field=field): self.rejected(lambda c: c['report'].__setitem__(field, False))
        for field in T.FALSE_FIELDS:
            with self.subTest(field=field): self.rejected(lambda c: c['report'].__setitem__(field, True))
        self.rejected(lambda c: c['wrapper']['host_timing'].__setitem__('complete', False))

    def test_exact_retention_accounting_and_unchanged_bound(self):
        for field, value in (('ordinary_retained_bytes', 0), ('supervisor_metadata_allowance', 0),
                ('retained_bytes_with_timing', 32 << 20)):
            with self.subTest(field=field):
                self.rejected(lambda c: c['wrapper']['host_timing'].__setitem__(field, value), after=True)

    def test_parity_accepts_distinct_sessions_and_rejects_any_record_change(self):
        case = copy.deepcopy(self.base)
        current, baseline, bodies = parity_fixture(case)
        result = T.compare_same_side(R.wire(current), R.wire(baseline), case['checked'], case['checked'], lambda p: bodies[p['path']])
        self.assertEqual(len(result['records']), 40)
        self.assertTrue(result['all4_payloads_byte_equal'])
        path = current['files']['frames']['path']
        frames = [json.loads(line) for line in bodies[path].splitlines()]
        frames[9]['completion']['output_token'] += 1
        bodies[path] = b''.join(R.wire(v) + b'\n' for v in frames)
        current['files']['frames'] = R.pin(path, bodies[path])
        with self.assertRaises(ValueError):
            T.compare_same_side(R.wire(current), R.wire(baseline), case['checked'], case['checked'], lambda p: bodies[p['path']])

    def test_parity_compares_payload_not_control_timer_bytes(self):
        case = copy.deepcopy(self.base); current, baseline, bodies = parity_fixture(case)
        capture = current['files']['captures'][0]
        path = capture['file']['path']; raw = bytearray(bodies[path]); raw[0] ^= 1
        bodies[path] = bytes(raw); capture['file'] = R.pin(path, bodies[path])
        T.compare_same_side(R.wire(current), R.wire(baseline), case['checked'], case['checked'], lambda p: bodies[p['path']])
        raw[T.CONTROL_BYTES] ^= 1; bodies[path] = bytes(raw); capture['file'] = R.pin(path, bodies[path])
        with self.assertRaises(ValueError):
            T.compare_same_side(R.wire(current), R.wire(baseline), case['checked'], case['checked'], lambda p: bodies[p['path']])

    def test_parity_rejects_same_session_model_drift_and_authority(self):
        for mode in ('session', 'source', 'authority'):
            with self.subTest(mode=mode):
                case = copy.deepcopy(self.base); current, baseline, bodies = parity_fixture(case)
                if mode == 'session': baseline['request']['base']['session'] = current['request']['base']['session']
                elif mode == 'source': baseline['request']['base']['source'] = '/different'
                else: case['checked']['numerical_acceptance'] = True
                with self.assertRaises(ValueError):
                    T.compare_same_side(R.wire(current), R.wire(baseline), case['checked'], case['checked'], lambda p: bodies[p['path']])
