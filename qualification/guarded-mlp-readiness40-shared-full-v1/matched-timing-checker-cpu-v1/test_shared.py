"""Synthetic SharedFull policy and parity gates; no native execution."""
import copy
import json
import unittest

import test_readiness as R
import validate_shared as S


def record(summary):
    seq = summary['bootstrap']['sequence']
    return dict(schema='FerricReadiness40Position5SharedFullPolicyV1',
        session=seq['scope']['session'], device_ids=seq['device_ids'], child_pid=seq['scope']['child_identity'],
        worker_sha256=summary['request']['base']['worker']['sha256'], profile_sha256=summary['profile_sha256'],
        registration_sha256=seq['registration'], transcript_sha256=summary['transcript_sha256'],
        completed_forwards=40, generated_tokens=[], capture_positions=[0, 5, 16, 39],
        shared_full_currentness=True, cache_kernel_admission=False, operational_currentness=False,
        legacy_profile=False, host_observer=False, paired_hidden_reads=False, paired_terminal=False,
        native_closed=True, full_long_workload=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)


def install(value, raw):
    summary, _, bodies, _ = value
    path = summary['files']['child_stderr']['path']
    summary['files']['bytes_before_summary'] += len(raw) - len(bodies[path])
    bodies[path] = raw
    summary['files']['child_stderr'] = R.pin(path, raw)


def check(value, policy=None, wrapper_edit=None):
    summary, request, bodies, prompt = value
    raw = R.summary_bytes(summary)
    wrapper = dict(schema='FerricReadiness40Position5SharedFullObservationV1', observation=summary,
        currentness_policy=record(summary) if policy is None else policy)
    if wrapper_edit: wrapper_edit(wrapper)
    return S.validate(R.wire(wrapper), raw, request, R.DIRECTORY, lambda p: bodies[p['path']], prompt)


def parity_fixture(value, checked):
    current = copy.deepcopy(value[0])
    baseline = json.loads(R.wire(current).replace(b'/synthetic/readiness/native', b'/synthetic/control/native'))
    baseline['request']['base']['session'] = [9] * 32
    bodies = dict(value[2])
    for path, raw in value[2].items():
        bodies[path.replace('/synthetic/readiness/native', '/synthetic/control/native')] = raw
    admitted = copy.deepcopy(checked)
    admitted.pop('shared_full_policy')
    return current, baseline, bodies, admitted


class SharedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ordinary = R.fixture()
        cls.base = copy.deepcopy(cls.ordinary)
        install(cls.base, R.wire(record(cls.base[0])) + b'\n')

    def test_complete_policy_keeps_original_stderr_and_ordinary_contract(self):
        value = copy.deepcopy(self.base)
        checked, shared = check(value)
        self.assertEqual(checked['completed_forwards'], 40)
        self.assertEqual(shared['policy_record'], record(value[0]))
        self.assertEqual(shared['file']['bytes'], len(value[2][value[0]['files']['child_stderr']['path']]))
        self.assertTrue(shared['no_policy_bytes_discarded'])
        self.assertFalse(shared['numerical_acceptance'])

    def test_default_empty_stderr_and_shared_opt_in_are_not_interchangeable(self):
        self.assertEqual(R.check(copy.deepcopy(self.ordinary))['completed_forwards'], 40)
        with self.assertRaises(ValueError): R.check(copy.deepcopy(self.base))
        with self.assertRaises(ValueError): check(copy.deepcopy(self.ordinary))

    def test_canonical_single_record_no_suffix_duplicates_or_reordering(self):
        policy = record(self.base[0]); raw = R.wire(policy) + b'\n'
        candidates = (b'', raw[:-1], raw + b'\n', raw + raw, b' ' + raw,
            json.dumps(policy, indent=2).encode() + b'\n',
            R.wire(dict(reversed(list(policy.items())))) + b'\n',
            raw.replace(b'{', b'{"native_closed":true,', 1), b' ' * 4097)
        for candidate in candidates:
            with self.subTest(raw=candidate[:40]):
                self.assertNotEqual(candidate, raw)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    S.validate_record(candidate, self.base[0])

    def test_every_policy_flag_is_closed_and_bool_is_not_integer(self):
        policy = record(self.base[0])
        for field, expected in policy.items():
            if type(expected) is not bool: continue
            for wrong in (not expected, int(expected)):
                with self.subTest(field=field, wrong=wrong):
                    changed = dict(policy); changed[field] = wrong
                    self.assertNotEqual(R.wire(changed), R.wire(policy))
                    with self.assertRaises(ValueError):
                        S.validate_record(R.wire(changed) + b'\n', self.base[0])

    def test_identity_scope_hash_and_device_order_are_joined(self):
        policy = record(self.base[0])
        for field in ('session', 'worker_sha256', 'profile_sha256', 'registration_sha256', 'transcript_sha256'):
            with self.subTest(field=field):
                changed = copy.deepcopy(policy); changed[field][0] ^= 1
                with self.assertRaises(ValueError): S.validate_record(R.wire(changed) + b'\n', self.base[0])
        for field, wrong in (('child_pid', 102), ('child_pid', True), ('device_ids', list(reversed(policy['device_ids'])))):
            with self.subTest(field=field, wrong=wrong):
                changed = dict(policy); changed[field] = wrong
                with self.assertRaises(ValueError): S.validate_record(R.wire(changed) + b'\n', self.base[0])

    def test_exact_extent_captures_schema_and_no_extra_fields(self):
        policy = record(self.base[0])
        for field, wrong in (('completed_forwards', 39), ('generated_tokens', [3]),
                ('capture_positions', [0, 15, 16, 39]), ('schema', 'FerricReadiness40Position5ParentHostTimingV1'),
                ('unexpected', False)):
            with self.subTest(field=field):
                changed = dict(policy); changed[field] = wrong
                with self.assertRaises(ValueError): S.validate_record(R.wire(changed) + b'\n', self.base[0])
        changed = dict(policy); del changed['paired_terminal']
        with self.assertRaises(ValueError): S.validate_record(R.wire(changed) + b'\n', self.base[0])

    def test_real_close_and_position5_only_precede_policy_admission(self):
        raw = R.wire(record(self.base[0])) + b'\n'
        for field in ('native_closed', 'child_exit_zero', 'process_group_absent'):
            with self.subTest(field=field):
                summary = copy.deepcopy(self.base[0]); summary[field] = False
                with self.assertRaises(ValueError): S.validate_record(raw, summary)
        for profile in ('readiness40', 'full2303', 'autoregressive'):
            with self.subTest(profile=profile):
                summary = copy.deepcopy(self.base[0]); summary['bootstrap']['sequence']['profile'] = profile
                with self.assertRaises(ValueError): S.validate_record(raw, summary)

    def test_wrapper_cannot_relabel_ordinary_timing_or_different_policy(self):
        for schema in ('FerricReadiness40Position5TimedObservationV1', 'FerricGuardedMlpReadiness40Position5ObservationV1'):
            with self.subTest(schema=schema):
                with self.assertRaises(ValueError):
                    check(copy.deepcopy(self.base), wrapper_edit=lambda w: w.__setitem__('schema', schema))
        policy = record(self.base[0]); policy['shared_full_currentness'] = False
        with self.assertRaises(ValueError): check(copy.deepcopy(self.base), policy=policy)
        with self.assertRaises(ValueError):
            check(copy.deepcopy(self.base), wrapper_edit=lambda w: w.__setitem__('extra', False))

    def test_pinned_original_bytes_and_all40_chain_still_required(self):
        value = copy.deepcopy(self.base)
        path = value[0]['files']['child_stderr']['path']; value[2][path] += b'\n'
        with self.assertRaises(ValueError): check(value)
        value = copy.deepcopy(self.base)
        R.change_frame(value, 9, lambda f: f['completion'].__setitem__('output_token', 4))
        with self.assertRaises(ValueError): check(value)
        value = copy.deepcopy(self.base); value[0]['files']['bytes_before_summary'] -= 1
        with self.assertRaises(ValueError): check(value)

    def test_parity_compares_all40_records_and_all_four_payloads(self):
        value = copy.deepcopy(self.base); checked, _ = check(value)
        current, baseline, bodies, old_checked = parity_fixture(value, checked)
        read = lambda p: bodies[p['path']]
        got = S.compare_same_side(R.wire(current), R.wire(baseline), checked, old_checked, read)
        self.assertEqual(len(got['records']), 40); self.assertEqual(len(got['captures']), 4)
        path = current['files']['frames']['path']
        rows = [json.loads(line) for line in bodies[path].splitlines()]
        rows[11]['completion']['output_token'] += 1
        bodies[path] = b''.join(R.wire(row) + b'\n' for row in rows)
        current['files']['frames'] = R.pin(path, bodies[path])
        with self.assertRaises(ValueError): S.compare_same_side(R.wire(current), R.wire(baseline), checked, old_checked, read)

    def test_parity_ignores_only_independently_admitted_control_timer_bytes(self):
        value = copy.deepcopy(self.base); checked, _ = check(value)
        current, baseline, bodies, old_checked = parity_fixture(value, checked)
        capture = current['files']['captures'][0]; path = capture['file']['path']
        raw = bytearray(bodies[path]); raw[0] ^= 1
        bodies[path] = bytes(raw); capture['file'] = R.pin(path, bodies[path])
        read = lambda p: bodies[p['path']]
        S.compare_same_side(R.wire(current), R.wire(baseline), checked, old_checked, read)
        raw[S.CONTROL_BYTES] ^= 1; bodies[path] = bytes(raw); capture['file'] = R.pin(path, bodies[path])
        with self.assertRaises(ValueError): S.compare_same_side(R.wire(current), R.wire(baseline), checked, old_checked, read)

    def test_parity_refuses_same_session_model_drift_or_numerical_authority(self):
        value = copy.deepcopy(self.base); checked, _ = check(value)
        for mode in ('session', 'model', 'authority'):
            with self.subTest(mode=mode):
                current, baseline, bodies, old_checked = parity_fixture(value, checked)
                if mode == 'session': baseline['request']['base']['session'] = current['request']['base']['session']
                elif mode == 'model': baseline['request']['base']['source'] = '/different'
                else: old_checked['numerical_acceptance'] = True
                with self.assertRaises(ValueError):
                    S.compare_same_side(R.wire(current), R.wire(baseline), checked, old_checked, lambda p: bodies[p['path']])
