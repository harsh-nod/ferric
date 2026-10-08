"""Synthetic scoped-policy/ordinary-parity tests; no native or performance outcome."""
import copy
import json
import unittest

import test_readiness as R
import test_shared as H
import test_matched as K
import validate_scoped as C


def record(summary):
    seq = summary['bootstrap']['sequence']
    return dict(schema='FerricReadiness40Position5ScopedWarmPolicyV1',
        execution_profile='Readiness40Position5ScopedWarmCurrentnessV1',
        session=seq['scope']['session'], device_ids=seq['device_ids'], child_pid=seq['scope']['child_identity'],
        worker_sha256=summary['request']['base']['worker']['sha256'], profile_sha256=summary['profile_sha256'],
        registration_sha256=seq['registration'], transcript_sha256=summary['transcript_sha256'],
        completed_forwards=40, generated_tokens=[], capture_positions=[0, 5, 16, 39],
        counts=dict(ordinary_layers=72, scoped_layers=1368, scoped_layers_by_forward=[0, 0] + [36] * 38,
            full_discoveries=2736, local_checkpoints=6840, before_calls=15048,
            after_calls=15048, generation_probes=17784),
        scoped_warm_currentness=True, full_entry_exit_per_scoped_layer=True,
        participant_local_between_boundaries=True, scope_includes_prefix_mlp_hidden=True,
        temporal_equivalent_to_full=False, default_group_policy_unchanged=True,
        shared_full_currentness=False, cache_kernel_admission=False, operational_currentness=False,
        host_observer=False, paired_hidden_reads=False, paired_terminal=False, native_closed=True,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False, production_authority=False)


def make_case(mode, value, root):
    H.install(value, R.wire(record(value[0])) + b'\n')
    case = K.make_case('shared', value, root)
    case['mode'] = mode
    case['wrapper']['schema'] = C.WRAPPERS[mode]
    case['wrapper']['currentness_policy'] = record(value[0])
    if mode == 'scoped':
        del case['wrapper']['host_timing']
    return case


def fixture(mode):
    old = K.fixture('shared')
    return make_case(mode, old['value'], old['root'])


def check(case):
    _, request, bodies, prompt = case['value']
    return C.validate(case['mode'], R.wire(case['wrapper']), case['summary'], request,
        case['root'], lambda pin: bodies[pin['path']], prompt)


def parity(case, baseline):
    scoped, ordinary = check(case), K.check(baseline)['ordinary']
    bodies = {**case['value'][2], **baseline['value'][2]}
    return C.compare_same_side(case['summary'], baseline['summary'], scoped, ordinary,
        lambda pin: bodies[pin['path']])


class ScopedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scoped = fixture('scoped')
        cls.timed = fixture('scoped_timed')
        cls.default = K.fixture('default')
        cls.shared = K.fixture('shared')

    def test_ordinary_and_timed_scoped_keep_original_policy_bytes_without_shared_label(self):
        for original in (self.scoped, self.timed):
            case = copy.deepcopy(original); bodies = copy.deepcopy(case['value'][2])
            got = check(case)
            self.assertEqual(bodies, case['value'][2])
            self.assertEqual(got['ordinary']['scoped_warm_policy'], record(case['value'][0]))
            self.assertNotIn('shared_full_policy', got['ordinary'])
            self.assertFalse(got['policy']['shared_full_currentness'])
            self.assertFalse(got['policy']['temporal_equivalent_to_full'])
            self.assertFalse(got['numerical_acceptance'] or got['performance_claim'])
            self.assertFalse(got['outer_owned_lineage_checked'] or got['cpu_qualification_checked'])
            if case['mode'] == 'scoped_timed':
                self.assertEqual(got['timing']['disjoint_spans'], 124)
                self.assertEqual(got['timing']['timeline']['total_ns'], 868)
            else:
                self.assertIsNone(got['timing'])

    def test_default_shared_and_scoped_policy_routes_are_not_interchangeable(self):
        for original in (self.scoped, self.timed):
            case = copy.deepcopy(original); value = case['value']
            with self.assertRaises(ValueError):
                C.V.validate(case['summary'], value[1], case['root'],
                    lambda pin: value[2][pin['path']], value[3])
            with self.assertRaises(ValueError):
                C.S.validate_record(R.wire(record(value[0])) + b'\n', value[0])
        for original in (self.default, self.shared):
            case = copy.deepcopy(original); case['mode'] = 'scoped'
            case['wrapper'] = dict(schema=C.WRAPPERS['scoped'], observation=case['value'][0],
                currentness_policy=record(case['value'][0]))
            with self.assertRaises(ValueError): check(case)
        self.assertEqual(K.check(copy.deepcopy(self.default))['policy']['name'], 'default_full')
        self.assertEqual(K.check(copy.deepcopy(self.shared))['policy']['name'], 'shared_full')

    def test_policy_requires_single_canonical_ordered_bounded_record(self):
        value = record(self.scoped['value'][0]); raw = R.wire(value) + b'\n'
        wrong = (b'', raw[:-1], raw + b'\n', raw + raw, b' ' + raw, b' ' * 4097,
            json.dumps(value, indent=2).encode() + b'\n',
            R.wire(dict(reversed(list(value.items())))) + b'\n',
            raw.replace(b'{', b'{"native_closed":true,', 1))
        for candidate in wrong:
            with self.subTest(prefix=candidate[:30]), self.assertRaises(ValueError):
                C.validate_record(candidate, self.scoped['value'][0])
        value['counts'] = dict(reversed(list(value['counts'].items())))
        with self.assertRaises(ValueError):
            C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])

    def test_counter_scalars_are_exact_unsigned_integers_not_bool_float_or_overflow(self):
        for field in C.COUNT_FIELDS.split():
            for wrong in (True, 1.0, -1, 1 << 64):
                value = record(self.scoped['value'][0])
                if field == 'scoped_layers_by_forward':
                    value['counts'][field][2] = wrong
                else:
                    value['counts'][field] = wrong
                with self.subTest(field=field, wrong=wrong), self.assertRaises(ValueError):
                    C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])

    def test_counter_census_and_checked_relations_refuse_missing_warm_calls(self):
        changes = {'ordinary_layers': 73, 'scoped_layers': 1367, 'full_discoveries': 2735,
            'local_checkpoints': 1367, 'before_calls': 1, 'after_calls': 15049,
            'generation_probes': 17785, 'scoped_layers_by_forward': [0, 36] + [36] * 38}
        for field, wrong in changes.items():
            value = record(self.scoped['value'][0]); value['counts'][field] = wrong
            with self.subTest(field=field), self.assertRaises(ValueError):
                C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])
        for local in ((1 << 63), (1 << 64) - 1):
            value = record(self.scoped['value'][0]); value['counts']['local_checkpoints'] = local
            with self.assertRaises(ValueError):
                C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])

    def test_large_valid_poll_dependent_counters_preserve_all_integer_bits(self):
        value = record(self.scoped['value'][0]); local = (1 << 53) + 3
        value['counts'].update(local_checkpoints=local, before_calls=local + 5472,
            after_calls=local + 5472, generation_probes=2 * local + 4104)
        got = C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])
        self.assertEqual(got['counts']['local_checkpoints'], local)
        self.assertIs(type(got['counts']['generation_probes']), int)

    def test_every_policy_flag_and_execution_identity_is_closed(self):
        original = record(self.scoped['value'][0])
        for field, expected in original.items():
            if type(expected) is not bool: continue
            for wrong in (not expected, int(expected)):
                value = copy.deepcopy(original); value[field] = wrong
                with self.subTest(field=field, wrong=wrong), self.assertRaises(ValueError):
                    C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])
        for field, wrong in (('schema', 'FerricReadiness40Position5SharedFullPolicyV1'),
                ('execution_profile', 'SharedFull'), ('completed_forwards', 39),
                ('generated_tokens', [2]), ('capture_positions', [0, 15, 16, 39]), ('extra', False)):
            value = copy.deepcopy(original); value[field] = wrong
            with self.subTest(field=field), self.assertRaises(ValueError):
                C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])

    def test_record_joins_bootstrap_session_devices_worker_profile_and_transcript(self):
        for field in ('session', 'worker_sha256', 'profile_sha256', 'registration_sha256', 'transcript_sha256'):
            value = record(self.scoped['value'][0]); value[field] = list(value[field]); value[field][0] ^= 1
            with self.subTest(field=field), self.assertRaises(ValueError):
                C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])
        for field, wrong in (('child_pid', True), ('child_pid', 999), ('device_ids', [2, 1])):
            value = record(self.scoped['value'][0]); value[field] = wrong
            with self.subTest(field=field), self.assertRaises(ValueError):
                C.validate_record(R.wire(value) + b'\n', self.scoped['value'][0])

    def test_real_close_and_position5_are_required_not_full_or_old_readiness(self):
        for field in ('native_closed', 'child_exit_zero', 'process_group_absent'):
            summary = copy.deepcopy(self.scoped['value'][0]); summary[field] = False
            with self.subTest(field=field), self.assertRaises(ValueError):
                C.validate_record(R.wire(record(summary)) + b'\n', summary)
        for profile in ('readiness40', 'full2303', 'autoregressive'):
            summary = copy.deepcopy(self.scoped['value'][0]); summary['bootstrap']['sequence']['profile'] = profile
            with self.subTest(profile=profile), self.assertRaises(ValueError):
                C.validate_record(R.wire(record(summary)) + b'\n', summary)

    def test_explicit_wrapper_and_mode_cannot_relabel_ordinary_or_shared(self):
        for mode in (None, True, 'default', 'shared', 'full2303'):
            case = copy.deepcopy(self.scoped); case['mode'] = mode
            with self.subTest(mode=mode), self.assertRaises(ValueError): check(case)
        for change in ('schema', 'observation', 'policy', 'extra', 'timing'):
            case = copy.deepcopy(self.scoped)
            if change == 'schema': case['wrapper']['schema'] = C.WRAPPERS['scoped_timed']
            elif change == 'observation': case['wrapper']['observation'] = {}
            elif change == 'policy': case['wrapper']['currentness_policy']['counts']['scoped_layers'] -= 1
            elif change == 'extra': case['wrapper']['extra'] = False
            else: case['wrapper']['host_timing'] = {}
            with self.subTest(change=change), self.assertRaises(ValueError): check(case)

    def test_original_pinned_stderr_all40_frames_four_captures_and_retention_stay_required(self):
        for change in ('stderr', 'frame', 'capture', 'accounting'):
            case = copy.deepcopy(self.scoped); value = case['value']
            check(case)
            if change == 'stderr': value[2][value[0]['files']['child_stderr']['path']] += b'\n'
            elif change == 'frame':
                path = value[0]['files']['frames']['path']
                rows = [json.loads(line) for line in value[2][path].splitlines()]
                rows[11]['completion']['output_token'] = 4
                raw = b''.join(R.wire(row) + b'\n' for row in rows)
                value[0]['files']['bytes_before_summary'] += len(raw) - len(value[2][path])
                value[2][path] = raw
                value[0]['files']['frames'] = R.pin(path, raw)
                case = make_case('scoped', value, case['root'])
            elif change == 'capture': value[2][value[0]['files']['captures'][3]['file']['path']] = b''
            else:
                value[0]['files']['bytes_before_summary'] -= 1
                case['summary'] = R.summary_bytes(value[0])
            expected = 'full40 chained original completion' if change == 'frame' else ''
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, expected): check(case)

    def test_timed_route_preserves_all124_spans_exact_pins_caps_and_false_authority(self):
        for change in ('integer', 'disjoint', 'pin', 'total', 'authority', 'close'):
            case = copy.deepcopy(self.timed)
            if change == 'integer': case['report']['timeline']['total_ns'] = True
            elif change == 'disjoint': case['report']['timeline']['forwards'][2]['prepare_write']['start_ns'] += 1
            elif change == 'pin': case['report']['ordinary_complete']['sha256'][0] ^= 1
            elif change == 'authority': case['report']['gpu_timing'] = True
            elif change == 'close': case['report']['native_closed'] = False
            K.encode(case)
            if change == 'total': case['wrapper']['host_timing']['retained_bytes_with_timing'] = 32 << 20
            with self.subTest(change=change), self.assertRaises(ValueError): check(case)

    def test_parity_checks_every_semantic_record_and_all_four_complete_payloads(self):
        got = parity(copy.deepcopy(self.scoped), copy.deepcopy(self.default))
        self.assertTrue(got['all40_records_equal'] and got['all4_payloads_byte_equal'])
        self.assertEqual(len(got['records']), 40); self.assertEqual(len(got['captures']), 4)
        self.assertFalse(got['controls_byte_equality_claimed'] or got['numerical_acceptance'])
        for change in ('unselected-output', 'selected-payload'):
            case = copy.deepcopy(self.scoped); value = case['value']; summary, _, bodies, _ = value
            rows = [json.loads(line) for line in bodies[summary['files']['frames']['path']].splitlines()]
            if change == 'unselected-output': rows[11]['completion']['output_token'] = 4
            else:
                capture = summary['files']['captures'][1]; path = capture['file']['path']
                raw = bytearray(bodies[path]); raw[C.V.CONTROL_BYTES] ^= 1; bodies[path] = bytes(raw)
                capture['file'] = R.pin(path, bodies[path])
                rows[5]['completion']['observation'] = R.part(bodies[path][C.V.CONTROL_BYTES:])
            K.rebuild(value, rows)
            case = make_case('scoped', value, case['root'])
            check(case)
            with self.subTest(change=change), self.assertRaises(ValueError):
                parity(case, copy.deepcopy(self.default))

    def test_parity_rejects_relabelled_policy_or_another_admitted_summary(self):
        case, baseline = copy.deepcopy(self.scoped), copy.deepcopy(self.default)
        a, b = check(case), K.check(baseline)['ordinary']
        bodies = {**case['value'][2], **baseline['value'][2]}
        for change in ('policy-name', 'legacy-slot', 'summary-pin', 'original-stderr'):
            current = copy.deepcopy(a); readset = dict(bodies)
            if change == 'policy-name': current['policy']['name'] = 'shared_full'
            elif change == 'legacy-slot': current['ordinary']['shared_full_policy'] = current['ordinary']['scoped_warm_policy']
            elif change == 'summary-pin': current['summary']['sha256'] = '0' * 64
            else: readset[current['policy']['file']['path']] += b'\n'
            with self.subTest(change=change), self.assertRaises(ValueError):
                C.compare_same_side(case['summary'], baseline['summary'], current, b, lambda pin: readset[pin['path']])
