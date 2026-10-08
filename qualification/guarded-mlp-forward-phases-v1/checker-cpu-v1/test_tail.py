"""Synthetic Tail V4 mutations; no native execution or observed performance."""
import copy
import json
import unittest

import test_readiness as R
import test_shared as H
import test_matched as K
import test_census as J
import validate_census as N
import validate_tail as U
import validate_tail_pair as P


def record(summary):
    value = {}
    for key, item in J.record(summary).items():
        if key == 'schema':
            item = 'FerricReadiness40Position5BankScopedWarmCensusTailPolicyV4'
        elif key == 'execution_profile':
            item = 'Readiness40Position5BankScopedWarmCensusTailCurrentnessV4'
        elif key == 'counts':
            item['tails'] = dict(ordinary_tails=2, scoped_tails=38, dispatches=114, readbacks=114,
                readback_bytes=11858584, full_discoveries=76, local_checkpoints=1026,
                before_calls=1634, after_calls=1634, generation_probes=2166)
        if key == 'temporal_equivalent_to_full':
            value.update(scoped_tail=True, full_entry_exit_per_scoped_tail=True,
                         scope_includes_tail_dispatch_and_readback=True)
        value[key] = item
    return value


def make_case(value, root):
    H.install(value, R.wire(record(value[0])) + b'\n')
    case = K.make_case('shared', value, root)
    case['mode'] = 'tail_timed'
    case['wrapper']['schema'] = U.WRAPPERS['tail_timed']
    case['wrapper']['currentness_policy'] = record(value[0])
    return case


def fixture():
    original = K.fixture('shared')
    return make_case(original['value'], original['root'])


def check(case):
    _, request, bodies, prompt = case['value']
    return U.validate(case['mode'], R.wire(case['wrapper']), case['summary'], request,
                      case['root'], lambda pin: bodies[pin['path']], prompt)


def setup_pair():
    original = K.fixture('default')
    left = J.make_case(original['value'], original['root'])
    right = fixture()
    a, b = J.check(left), check(right)
    common = dict(worker=P.rust_pin(left['value'][1]['base']['worker']),
        worker_cpu=dict(path='/qualified/coupled.json', bytes=123, sha256='a' * 64),
        parent=dict(path='/qualified/parent', bytes=456, sha256='b' * 64),
        parent_cpu=dict(path='/qualified/parent.json', bytes=789, sha256='c' * 64))
    return left, right, a, b, dict(census=copy.deepcopy(common), tail=copy.deepcopy(common)), {
        **left['value'][2], **right['value'][2]}


def compare(value):
    left, right, a, b, admissions, bodies = value
    return P.compare_pair(left['summary'], right['summary'], a, b, admissions,
                          lambda pin: bodies[pin['path']])


class TailTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case = fixture()

    def test_tail_original_record_keeps_separate_counters_bytes_and124_spans(self):
        case = copy.deepcopy(self.case); before = copy.deepcopy(case['value'][2])
        got = check(case); counts = got['policy']['policy_record']['counts']
        self.assertEqual(before, case['value'][2])
        self.assertEqual(list(counts), ['layers', 'banks', 'census', 'tails'])
        self.assertEqual(counts['tails'], record(case['value'][0])['counts']['tails'])
        self.assertEqual(counts['layers'], J.record(case['value'][0])['counts']['layers'])
        self.assertEqual(counts['census']['rank_checkpoints'], 21888)
        self.assertEqual(got['timing']['disjoint_spans'], 124)
        self.assertEqual(got['ordinary']['completed_forwards'], 40)
        self.assertTrue(got['scoped_tail'] and got['tail_counters_are_independent'])
        self.assertNotIn('bank_scoped_census_policy', got['ordinary'])
        self.assertNotIn('shared_full_policy', got['ordinary'])
        self.assertFalse(got['gpu_timing'] or got['performance_claim'] or got['numerical_acceptance'])

    def test_tail_all_counter_fields_and_fixed_extent_relations_are_closed(self):
        summary = self.case['value'][0]
        for key in U.TAIL_FIELDS.split():
            value = record(summary); value['counts']['tails'][key] += 1
            with self.subTest(key=key), self.assertRaises(ValueError):
                U.validate_record(R.wire(value) + b'\n', summary)
        for key, wrong in [('ordinary_tails', 0), ('scoped_tails', 40), ('dispatches', 120),
                ('readbacks', 38), ('readback_bytes', 312068), ('full_discoveries', 80),
                ('local_checkpoints', 1025)]:
            value = record(summary); value['counts']['tails'][key] = wrong
            with self.subTest(key=key), self.assertRaises(ValueError):
                U.validate_record(R.wire(value) + b'\n', summary)
        value = record(summary); value['counts']['tails']['extra'] = 0
        with self.assertRaises(ValueError): U.validate_record(R.wire(value) + b'\n', summary)

    def test_tail_scalar_types_widths_and_checked_overflow_are_strict(self):
        summary = self.case['value'][0]
        for key in U.TAIL_FIELDS.split():
            for wrong in (True, 1.0, -1, 1 << 64):
                value = record(summary); value['counts']['tails'][key] = wrong
                with self.subTest(key=key, wrong=wrong), self.assertRaises(ValueError):
                    U.validate_record(R.wire(value) + b'\n', summary)
        for local in ((1 << 63), (1 << 64) - 1):
            value = record(summary)
            value['counts']['tails'].update(local_checkpoints=local, before_calls=local+608,
                after_calls=local+608, generation_probes=2*local+114)
            with self.assertRaises(ValueError): U.validate_record(R.wire(value) + b'\n', summary)

    def test_tail_variable_periodic_checks_and_large_integers_are_exact(self):
        summary = self.case['value'][0]
        for local in (1026, 1027, 1139, (1 << 53) + 3):
            value = record(summary)
            value['counts']['tails'].update(local_checkpoints=local, before_calls=local+608,
                after_calls=local+608, generation_probes=2*local+114)
            got = U.validate_record(R.wire(value) + b'\n', summary)
            self.assertEqual(got['counts']['tails']['local_checkpoints'], local)
            self.assertEqual(got['counts']['tails']['generation_probes'], 2*local+114)

    def test_tail_one_original_canonical_bounded_record_is_required(self):
        summary = self.case['value'][0]; value = record(summary); raw = R.wire(value) + b'\n'
        bodies = [b'', raw[:-1], raw + b'\n', raw + raw, b' ' * 4097,
            json.dumps(value, indent=2).encode() + b'\n',
            R.wire(dict(reversed(list(value.items())))) + b'\n',
            raw.replace(b'{', b'{"scoped_tail":true,', 1)]
        for body in bodies:
            with self.subTest(body=body[:24]), self.assertRaises(ValueError):
                U.validate_record(body, summary)
        value['counts']['tails'] = dict(reversed(list(value['counts']['tails'].items())))
        with self.assertRaises(ValueError): U.validate_record(R.wire(value) + b'\n', summary)

    def test_tail_and_old_policy_routes_cannot_admit_each_other(self):
        summary = self.case['value'][0]; raw = R.wire(record(summary)) + b'\n'
        for make, validator in ((J.record, N.validate_record),
                (J.L.record, J.L.B.validate_record), (J.C.record, J.C.C.validate_record),
                (H.record, H.S.validate_record)):
            with self.assertRaises(ValueError): U.validate_record(R.wire(make(summary)) + b'\n', summary)
            with self.assertRaises(ValueError): validator(raw, summary)
        case = copy.deepcopy(self.case); _, request, bodies, prompt = case['value']
        with self.assertRaises(ValueError):
            U.V.validate(case['summary'], request, case['root'], lambda pin: bodies[pin['path']], prompt)
        for mode in ('census_timed', 'bank_timed', 'scoped_timed', 'tail', 'default', 'full2303'):
            case = copy.deepcopy(self.case); case['mode'] = mode
            with self.subTest(mode=mode), self.assertRaises(ValueError): check(case)

    def test_tail_scope_flags_identities_close_and_profile_cannot_be_relabelled(self):
        summary = self.case['value'][0]; original = record(summary)
        for key, item in original.items():
            if type(item) is bool:
                for wrong in (not item, int(item)):
                    value = copy.deepcopy(original); value[key] = wrong
                    with self.subTest(key=key), self.assertRaises(ValueError):
                        U.validate_record(R.wire(value) + b'\n', summary)
        for key in ('session', 'worker_sha256', 'profile_sha256', 'registration_sha256', 'transcript_sha256'):
            value = copy.deepcopy(original); value[key][0] ^= 1
            with self.subTest(key=key), self.assertRaises(ValueError):
                U.validate_record(R.wire(value) + b'\n', summary)
        for key, wrong in [('child_pid', True), ('completed_forwards', 39), ('generated_tokens', [2]),
                ('capture_positions', [0,15,16,39]), ('execution_profile', 'Full2303')]:
            value = copy.deepcopy(original); value[key] = wrong
            with self.assertRaises(ValueError): U.validate_record(R.wire(value) + b'\n', summary)
        for key in ('native_closed', 'child_exit_zero', 'process_group_absent'):
            changed = copy.deepcopy(summary); changed[key] = False
            with self.assertRaises(ValueError): U.validate_record(R.wire(record(changed)) + b'\n', changed)
        changed = copy.deepcopy(summary); changed['bootstrap']['sequence']['profile'] = 'full2303'
        with self.assertRaises(ValueError): U.validate_record(R.wire(record(changed)) + b'\n', changed)

    def test_tail_counts_never_replace_or_relax_inherited_layer_bank_census(self):
        summary = self.case['value'][0]
        for group in ('layers', 'banks', 'census'):
            value = record(summary); value['counts'][group] = value['counts']['tails']
            with self.subTest(group=group), self.assertRaises(ValueError):
                U.validate_record(R.wire(value) + b'\n', summary)
        for group, key in [('layers','before_calls'), ('banks','generation_probes'), ('census','rank_checkpoints')]:
            value = record(summary); value['counts'][group][key] += 1
            with self.assertRaises(ValueError): U.validate_record(R.wire(value) + b'\n', summary)
        for owners in ([787,787], [783,787], [787,782], [True,783], [2048,2048]):
            value = record(summary); value['counts']['census']['owner_counts'] = owners
            with self.subTest(owners=owners), self.assertRaises(ValueError):
                U.validate_record(R.wire(value) + b'\n', summary)
        value = record(summary)
        for key in ('local_checkpoints','before_calls','after_calls'):
            value['counts']['layers'][key] += value['counts']['tails'][key]
        with self.assertRaises(ValueError): U.validate_record(R.wire(value) + b'\n', summary)

    def test_tail_original40_chain_capture_accounting_and_timing_are_required(self):
        for change in ('stderr', 'frame', 'capture', 'accounting', 'timing', 'scope'):
            case = copy.deepcopy(self.case); value = case['value']; check(case)
            if change == 'stderr': value[2][value[0]['files']['child_stderr']['path']] += b'\n'
            elif change == 'frame':
                path = value[0]['files']['frames']['path']
                rows = [json.loads(line) for line in value[2][path].splitlines()]
                rows[11]['completion']['output_token'] = 4
                raw = b''.join(R.wire(row)+b'\n' for row in rows)
                value[0]['files']['bytes_before_summary'] += len(raw)-len(value[2][path])
                value[2][path] = raw; value[0]['files']['frames'] = R.pin(path, raw)
                case = make_case(value, case['root'])
            elif change == 'capture': value[2][value[0]['files']['captures'][3]['file']['path']] = b''
            elif change == 'accounting':
                value[0]['files']['bytes_before_summary'] -= 1; case['summary'] = R.summary_bytes(value[0])
            elif change == 'timing':
                case['report']['timeline']['forwards'][2]['prepare_write']['start_ns'] += 1; K.encode(case)
            else: case['wrapper']['schema'] = N.WRAPPERS['census_timed']
            with self.subTest(change=change), self.assertRaisesRegex(ValueError,
                    'full40 chained original completion' if change == 'frame' else ''):
                check(case)

    def test_tail_wrapper_cannot_replace_policy_or_timing_and_bounds(self):
        for change in ('policy','ordinary','timeline','sidecar','stdout','summary'):
            case = copy.deepcopy(self.case)
            if change == 'policy': case['wrapper']['currentness_policy'] = J.record(case['value'][0])
            elif change == 'ordinary': case['wrapper']['observation']['completed_forwards'] = 39
            elif change == 'timeline':
                case['report']['timeline']['total_ns'] += 1; K.encode(case)
            elif change == 'sidecar': case['value'][2][case['wrapper']['host_timing']['file']['path']] += b' '
            elif change == 'stdout': case['wrapper']['extra'] = 'x' * (128 << 10)
            else: case['summary'] = b' ' * ((128 << 10) + 1)
            with self.subTest(change=change), self.assertRaises(ValueError): check(case)

    def test_tail_pair_preserves_original_v3_v4_policies_and_exact_payload_parity(self):
        value = setup_pair(); before = copy.deepcopy(value[-1]); got = compare(value)
        self.assertEqual(before, value[-1])
        self.assertTrue(got['parity']['all40_records_equal'] and got['parity']['all4_payloads_byte_equal'])
        self.assertEqual(got['census_policy']['name'], 'bank_scoped_census')
        self.assertEqual(got['tail_policy']['name'], 'bank_scoped_census_tail')
        self.assertEqual(got['census_timing']['disjoint_spans'], 124)
        self.assertEqual(got['tail_timing']['disjoint_spans'], 124)
        self.assertTrue(got['scoped_tail'] and got['tail_counters_are_independent'])
        self.assertFalse(got['cpu_qualification_checked'] or got['outer_owned_lineage_checked']
                         or got['temporal_equivalent_to_full'])

    def test_tail_pair_requires_all_four_same_cpu_product_identities(self):
        for role in ('worker', 'worker_cpu', 'parent', 'parent_cpu'):
            value = setup_pair(); value[4]['tail'][role]['sha256'] = 'd'*64
            with self.subTest(role=role), self.assertRaises(ValueError): compare(value)
        for field, wrong in [('path','relative'), ('bytes',True), ('sha256','x')]:
            value = setup_pair()
            value[4]['tail']['parent'][field] = wrong; value[4]['census']['parent'][field] = wrong
            with self.assertRaises(ValueError): compare(value)
        value = setup_pair(); value[3]['mode'] = 'census_timed'
        with self.assertRaises(ValueError): compare(value)
        value = setup_pair(); case = value[1]; summary = case['value'][0]
        pid = value[0]['value'][0]['child_pid']
        summary['child_pid'] = pid
        sequence = summary['bootstrap']['sequence']
        for scope in (sequence['scope'], sequence['begin']['scope']):
            scope['child_identity'] = pid
        rows = [json.loads(line) for line in case['value'][2][summary['files']['frames']['path']].splitlines()]
        K.rebuild(case['value'], rows)
        case = make_case(case['value'], case['root']); checked = check(case)
        bodies = {**value[0]['value'][2], **case['value'][2]}
        with self.assertRaisesRegex(ValueError, 'distinct fresh case directories and workers'):
            P.compare_pair(value[0]['summary'], case['summary'], value[2], checked,
                           value[4], lambda pin: bodies[pin['path']])

    def test_tail_pair_rechecks_both_original_files_and_policy_authority(self):
        for side in (2,3):
            for change in ('stderr','record','file','authority','name','timeline'):
                value = setup_pair(); checked = value[side]
                if change == 'stderr': value[-1][checked['policy']['file']['path']] += b'\n'
                elif change == 'record': checked['policy']['policy_record']['native_closed'] = False
                elif change == 'file': checked['policy']['file']['sha256'] = '0'*64
                elif change == 'authority': checked['performance_claim'] = True
                elif change == 'name': checked['policy']['name'] = 'default_full'
                else: checked['timing']['timeline']['total_ns'] += 1
                with self.subTest(side=side,change=change), self.assertRaises(ValueError): compare(value)

    def test_tail_pair_checks_all40_semantic_records_and_four_complete_payloads(self):
        for change in ('unselected-output','selected-payload'):
            value = setup_pair(); case = value[1]; summary, _, bodies, _ = case['value']
            path = summary['files']['frames']['path']
            rows = [json.loads(line) for line in bodies[path].splitlines()]
            if change == 'unselected-output': rows[11]['completion']['output_token'] = 4
            else:
                capture = summary['files']['captures'][1]; path = capture['file']['path']
                raw = bytearray(bodies[path]); raw[U.V.CONTROL_BYTES] ^= 1; bodies[path] = bytes(raw)
                capture['file'] = R.pin(path,bodies[path])
                rows[5]['completion']['observation'] = R.part(bodies[path][U.V.CONTROL_BYTES:])
            K.rebuild(case['value'], rows)
            case = make_case(case['value'],case['root']); checked = check(case)
            bodies = {**value[0]['value'][2], **case['value'][2]}
            with self.subTest(change=change), self.assertRaises(ValueError):
                P.compare_pair(value[0]['summary'],case['summary'],value[2],checked,value[4],
                               lambda pin:bodies[pin['path']])

    def test_tail_same_side_requires_independent_scope_flags_and_rehashes_policy(self):
        value = setup_pair()
        for key in ('scoped_tail','tail_counters_are_independent'):
            checked = copy.deepcopy(value[3]); checked[key] = False
            with self.subTest(key=key), self.assertRaises(ValueError):
                U.compare_same_side(value[1]['summary'],value[0]['summary'],checked,value[2]['ordinary'],
                                    lambda pin:value[-1][pin['path']])
        checked = copy.deepcopy(value[3]); checked['ordinary']['bank_scoped_census_tail_policy']['counts']['tails']['scoped_tails'] = 37
        with self.assertRaises(ValueError):
            U.compare_same_side(value[1]['summary'],value[0]['summary'],checked,value[2]['ordinary'],
                                lambda pin:value[-1][pin['path']])

    def test_tail_case_and_pair_nonclaims_cannot_be_promoted_to_native_acceptance(self):
        got = check(copy.deepcopy(self.case))
        for key in ('outer_owned_lineage_checked','cpu_qualification_checked','full_long_workload',
                    'numerical_acceptance','performance_claim','production_authority','gpu_timing'):
            self.assertIs(got[key], False)
        pair = compare(setup_pair())
        for key in ('fresh_processes_independently_checked','independent_numerical_reference',
                    'outer_owned_lineage_checked','cpu_qualification_checked','full_long_workload',
                    'numerical_acceptance','performance_claim','production_authority','gpu_timing'):
            self.assertIs(pair[key], False)


if __name__ == '__main__':
    unittest.main()
