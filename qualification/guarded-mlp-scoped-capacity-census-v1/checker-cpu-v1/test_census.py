"""Synthetic V3 census mutations; no native execution or observed timing results."""
import copy
import json
import unittest

import test_readiness as R
import test_shared as H
import test_matched as K
import test_scoped as C
import test_bank_scoped as L
import validate_census as N
import validate_census_pair as P


def record(summary):
    value = {}
    for key, item in L.record(summary).items():
        if key == 'schema': item = 'FerricReadiness40Position5BankScopedWarmCensusPolicyV3'
        elif key == 'execution_profile': item = 'Readiness40Position5BankScopedWarmCensusCurrentnessV3'
        elif key == 'counts':
            for name, delta in (('local_checkpoints', 21888), ('before_calls', 21888),
                                ('after_calls', 21888), ('generation_probes', 43776)):
                item['layers'][name] += delta
            item['census'] = dict(warm_layers=1368, preflights=2736,
                                  rank_checkpoints=21888, owner_counts=[787, 783])
        elif key == 'allocation_preflights_outside_windows': item = False
        value[key] = item
        if key == 'allocation_preflights_outside_windows':
            value.update(scoped_capacity_census=True, allocation_preflights_changed=True)
    return value


def make_case(value, root):
    H.install(value, R.wire(record(value[0])) + b'\n')
    case = K.make_case('shared', value, root)
    case['mode'] = 'census_timed'
    case['wrapper']['schema'] = N.WRAPPERS['census_timed']
    case['wrapper']['currentness_policy'] = record(value[0])
    return case


def fixture():
    original = K.fixture('shared')
    return make_case(original['value'], original['root'])


def check(case):
    _, request, bodies, prompt = case['value']
    return N.validate(case['mode'], R.wire(case['wrapper']), case['summary'], request,
        case['root'], lambda pin: bodies[pin['path']], prompt)


def setup_pair():
    original = K.fixture('default')
    left = L.make_case(original['value'], original['root'])
    right = fixture()
    a, b = L.check(left), check(right)
    common = dict(worker=P.rust_pin(left['value'][1]['base']['worker']),
        worker_cpu=dict(path='/qualified/coupled.json', bytes=123, sha256='a' * 64),
        parent=dict(path='/qualified/parent', bytes=456, sha256='b' * 64),
        parent_cpu=dict(path='/qualified/parent.json', bytes=789, sha256='c' * 64))
    return left, right, a, b, dict(bank=copy.deepcopy(common), census=copy.deepcopy(common)), {
        **left['value'][2], **right['value'][2]}


def compare(value):
    left, right, a, b, admissions, bodies = value
    return P.compare_pair(left['summary'], right['summary'], a, b, admissions,
                          lambda pin: bodies[pin['path']])


class CensusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case = fixture()

    def test_census_original_bank_record_retains_separate_layers_banks_and124_spans(self):
        case = copy.deepcopy(self.case); before = copy.deepcopy(case['value'][2])
        got = check(case); counts = got['policy']['policy_record']['counts']
        self.assertEqual(before, case['value'][2])
        self.assertEqual(counts['layers']['full_discoveries'], 2736)
        self.assertEqual(counts['banks']['full_discoveries'], 76)
        self.assertEqual(counts['banks']['final_generations'], [20, 20])
        self.assertEqual(got['timing']['disjoint_spans'], 124)
        self.assertEqual(got['ordinary']['completed_forwards'], 40)
        self.assertEqual(counts['census']['owner_counts'], [787, 783])
        self.assertEqual(counts['layers']['local_checkpoints'], 6840 + 21888)
        self.assertEqual(N.C.counts(dict(counts['layers']))['local_checkpoints'], 28728)
        self.assertTrue(got['allocation_preflights_changed'] and got['census_counters_are_layer_subset'])
        self.assertNotIn('bank_scoped_warm_policy', got['ordinary'])
        self.assertNotIn('shared_full_policy', got['ordinary'])
        self.assertFalse(got['gpu_timing'] or got['performance_claim'] or got['numerical_acceptance'])

    def test_census_bank_policy_is_one_canonical_bounded_record_not_v1_or_shared(self):
        summary = self.case['value'][0]; value = record(summary); raw = R.wire(value) + b'\n'
        wrong = [b'', raw[:-1], raw + b'\n', raw + raw, b' ' * 4097,
            json.dumps(value, indent=2).encode() + b'\n',
            R.wire(dict(reversed(list(value.items())))) + b'\n',
            raw.replace(b'{', b'{"native_closed":true,', 1),
            R.wire(C.record(summary)) + b'\n', R.wire(H.record(summary)) + b'\n',
            R.wire(L.record(summary)) + b'\n']
        for body in wrong:
            with self.subTest(body=body[:24]), self.assertRaises(ValueError):
                N.validate_record(body, summary)
        for validator in (C.C.validate_record, H.S.validate_record, L.B.validate_record):
            with self.assertRaises(ValueError): validator(raw, summary)
        value['counts'] = dict(reversed(list(value['counts'].items())))
        with self.assertRaises(ValueError): N.validate_record(R.wire(value) + b'\n', summary)

    def test_census_bank_scalar_types_generations_and_every_counter_relation_are_strict(self):
        summary = self.case['value'][0]
        for key in L.B.BANK_FIELDS.split():
            for wrong in (True, 1.0, -1, 1 << 64):
                value = record(summary)
                if key in ('scoped_rearms_by_forward', 'final_generations'):
                    value['counts']['banks'][key][0] = wrong
                else: value['counts']['banks'][key] = wrong
                with self.subTest(key=key, wrong=wrong), self.assertRaises(ValueError):
                    N.validate_record(R.wire(value) + b'\n', summary)
        for key, wrong in dict(ordinary_initial_banks=1, scoped_rearms=37,
                scoped_rearms_by_forward=[0, 1] + [1] * 38, final_generations=[20, 19],
                full_discoveries=75, local_checkpoints=37, before_calls=531,
                after_calls=531, generation_probes=493).items():
            value = record(summary); value['counts']['banks'][key] = wrong
            with self.subTest(key=key), self.assertRaises(ValueError):
                N.validate_record(R.wire(value) + b'\n', summary)

    def test_census_layer_counts_cannot_be_replaced_by_bank_counts_or_weakened(self):
        summary = self.case['value'][0]
        for group in ('layers', 'banks'):
            value = record(summary); value['counts'][group] = value['counts']['banks' if group == 'layers' else 'layers']
            with self.assertRaises(ValueError): N.validate_record(R.wire(value) + b'\n', summary)
        for key in C.C.COUNT_FIELDS.split():
            value = record(summary)
            if key == 'scoped_layers_by_forward': value['counts']['layers'][key][1] = 36
            else: value['counts']['layers'][key] = True
            with self.subTest(key=key), self.assertRaises(ValueError):
                N.validate_record(R.wire(value) + b'\n', summary)

    def test_census_large_integer_counters_remain_exact_and_overflow_is_refused(self):
        summary = self.case['value'][0]; value = record(summary); local = (1 << 53) + 3
        value['counts']['banks'].update(local_checkpoints=local, before_calls=152+2*local,
            after_calls=152+2*local, generation_probes=114+2*local)
        got = N.validate_record(R.wire(value) + b'\n', summary)
        self.assertEqual(got['counts']['banks']['local_checkpoints'], local)
        for group in ('layers', 'banks'):
            for local in (1 << 63, (1 << 64)-1):
                value = record(summary); value['counts'][group]['local_checkpoints'] = local
                with self.subTest(group=group), self.assertRaises(ValueError):
                    N.validate_record(R.wire(value) + b'\n', summary)

    def test_census_all_policy_flags_identity_close_and_profile_remain_closed(self):
        summary = self.case['value'][0]; original = record(summary)
        for key, item in original.items():
            if type(item) is bool:
                for wrong in (not item, int(item)):
                    value = copy.deepcopy(original); value[key] = wrong
                    with self.subTest(key=key), self.assertRaises(ValueError):
                        N.validate_record(R.wire(value) + b'\n', summary)
        for key in ('session', 'worker_sha256', 'profile_sha256', 'registration_sha256', 'transcript_sha256'):
            value = copy.deepcopy(original); value[key][0] ^= 1
            with self.subTest(key=key), self.assertRaises(ValueError):
                N.validate_record(R.wire(value) + b'\n', summary)
        for key, wrong in [('child_pid', True), ('completed_forwards', 39), ('generated_tokens', [2]),
                ('capture_positions', [0,15,16,39]), ('extra', False)]:
            value = copy.deepcopy(original); value[key] = wrong
            with self.assertRaises(ValueError): N.validate_record(R.wire(value) + b'\n', summary)
        for key in ('native_closed', 'child_exit_zero', 'process_group_absent'):
            changed = copy.deepcopy(summary); changed[key] = False
            with self.assertRaises(ValueError): N.validate_record(R.wire(record(changed)) + b'\n', changed)
        changed = copy.deepcopy(summary); changed['bootstrap']['sequence']['profile'] = 'full2303'
        with self.assertRaises(ValueError): N.validate_record(R.wire(record(changed)) + b'\n', changed)

    def test_census_original40_chain_payloads_policy_accounting_and_timing_are_required(self):
        for change in ('stderr', 'frame', 'capture', 'accounting', 'timing', 'scope', 'mode'):
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
            elif change == 'scope': case['wrapper']['schema'] = C.C.WRAPPERS['scoped_timed']
            else: case['mode'] = 'scoped_timed'
            with self.subTest(change=change), self.assertRaisesRegex(ValueError,
                    'full40 chained original completion' if change == 'frame' else ''):
                check(case)

    def test_census_same_elf_bank_control_and_census_candidate_preserve_both_original_policies(self):
        value = setup_pair(); before = copy.deepcopy(value[-1]); result = compare(value)
        self.assertEqual(before, value[-1])
        self.assertTrue(result['parity']['all40_records_equal'] and result['parity']['all4_payloads_byte_equal'])
        self.assertEqual(result['bank_policy']['name'], 'bank_scoped_warm')
        self.assertEqual(result['census_policy']['name'], 'bank_scoped_census')
        self.assertEqual(result['bank_timing']['disjoint_spans'], 124)
        self.assertEqual(result['census_timing']['disjoint_spans'], 124)
        self.assertFalse(result['temporal_equivalent_to_full'])
        self.assertTrue(result['allocation_preflights_changed'] and result['census_counters_are_layer_subset'])
        self.assertFalse(result['cpu_qualification_checked'] or result['outer_owned_lineage_checked'])

    def test_census_pair_requires_same_all_cpu_product_pins_and_distinct_cases(self):
        for role in ('worker', 'worker_cpu', 'parent', 'parent_cpu'):
            value = setup_pair(); value[4]['census'][role]['sha256'] = 'd'*64
            with self.subTest(role=role), self.assertRaises(ValueError): compare(value)
        for field, wrong in [('path', 'relative'), ('bytes', True), ('sha256', 'x')]:
            value = setup_pair()
            value[4]['census']['parent'][field] = wrong; value[4]['bank']['parent'][field] = wrong
            with self.assertRaises(ValueError): compare(value)
        value = setup_pair(); value[3]['mode'] = 'bank_timed'
        with self.assertRaises(ValueError): compare(value)

    def test_census_pair_rechecks_both_original_policy_files_and_no_relabelled_authority(self):
        for side in (2,3):
            for change in ('stderr', 'record', 'file', 'authority', 'name', 'timeline'):
                value = setup_pair(); checked = value[side]
                if change == 'stderr': value[-1][checked['policy']['file']['path']] += b'\n'
                elif change == 'record': checked['policy']['policy_record']['native_closed'] = False
                elif change == 'file': checked['policy']['file']['sha256'] = '0'*64
                elif change == 'authority': checked['performance_claim'] = True
                elif change == 'name': checked['policy']['name'] = 'default_full'
                else: checked['timing']['timeline']['total_ns'] += 1
                with self.subTest(side=side,change=change), self.assertRaises(ValueError): compare(value)

    def test_census_pair_checks_every_semantic_record_and_complete_selected_payload(self):
        for change in ('unselected-output', 'selected-payload'):
            value = setup_pair(); case = value[1]; summary, _, bodies, _ = case['value']
            path = summary['files']['frames']['path']
            rows = [json.loads(line) for line in bodies[path].splitlines()]
            if change == 'unselected-output': rows[11]['completion']['output_token'] = 4
            else:
                capture = summary['files']['captures'][1]; path = capture['file']['path']
                raw = bytearray(bodies[path]); raw[N.V.CONTROL_BYTES] ^= 1; bodies[path] = bytes(raw)
                capture['file'] = R.pin(path,bodies[path])
                rows[5]['completion']['observation'] = R.part(bodies[path][N.V.CONTROL_BYTES:])
            K.rebuild(case['value'], rows)
            case = make_case(case['value'], case['root']); checked = check(case)
            bodies = {**value[0]['value'][2], **case['value'][2]}
            with self.subTest(change=change), self.assertRaises(ValueError):
                P.compare_pair(value[0]['summary'],case['summary'],value[2],checked,value[4],
                    lambda pin:bodies[pin['path']])

    def test_census_timing_byte_cap_and_native_close_cannot_be_upgraded_by_wrapper(self):
        for change in ('cap','close','generated','policy'):
            case = copy.deepcopy(self.case)
            if change=='cap': case['wrapper']['host_timing']['retained_bytes_with_timing']=32<<20
            elif change=='close': case['report']['native_closed']=False; K.encode(case)
            elif change=='generated': case['report']['generated_tokens']=1; K.encode(case)
            else: case['wrapper']['currentness_policy']['counts']['banks']['scoped_rearms']=39
            with self.subTest(change=change), self.assertRaises(ValueError): check(case)

    def test_census_exact_owner_assertion_is_ranked_non_authoritative_and_not_equalized(self):
        summary = self.case['value'][0]
        for owners in ([783, 787], [787, 787], [783, 783], [786, 783], [787, 784],
                       [0, 783], [2049, 783], [787], [787, 783, 0]):
            value = record(summary); value['counts']['census']['owner_counts'] = owners
            with self.subTest(owners=owners), self.assertRaises(ValueError):
                N.validate_record(R.wire(value) + b'\n', summary)
        value = record(summary)
        self.assertEqual(N.validate_record(R.wire(value) + b'\n', summary)['counts']['census']['owner_counts'],
                         [787, 783])

    def test_census_every_subset_scalar_and_nested_field_is_strict(self):
        summary = self.case['value'][0]
        for key in N.CENSUS_FIELDS.split():
            for bad in (True, 1.0, -1, 1 << 64):
                value = record(summary)
                if key == 'owner_counts': value['counts']['census'][key][0] = bad
                else: value['counts']['census'][key] = bad
                with self.subTest(key=key, bad=bad), self.assertRaises(ValueError):
                    N.validate_record(R.wire(value) + b'\n', summary)
        for group in ('layers', 'banks', 'census'):
            for kind in ('extra', 'missing', 'reordered'):
                value = record(summary); fields = value['counts'][group]
                if kind == 'extra': fields['extra'] = 0
                elif kind == 'missing': del fields[next(iter(fields))]
                else: value['counts'][group] = dict(reversed(list(fields.items())))
                with self.subTest(group=group, kind=kind), self.assertRaises(ValueError):
                    N.validate_record(R.wire(value) + b'\n', summary)
        for key in ('warm_layers', 'preflights', 'rank_checkpoints'):
            value = record(summary); value['counts']['census'][key] += 1
            with self.assertRaises(ValueError): N.validate_record(R.wire(value) + b'\n', summary)

    def test_census_subset_is_not_added_twice_and_remaining_layer_work_is_required(self):
        summary = self.case['value'][0]
        for local, before, probes in ((21888, 27360, 47880), (28728, 54720, 61560)):
            value = record(summary); layers = value['counts']['layers']
            layers.update(local_checkpoints=local, before_calls=before,
                          after_calls=before, generation_probes=probes)
            N.C.counts(layers)  # Complete totals alone are insufficient.
            with self.assertRaises(ValueError): N.validate_record(R.wire(value) + b'\n', summary)
        for key, bad in (('local_checkpoints', 21887), ('before_calls', 21887),
                         ('after_calls', 21887), ('generation_probes', 43775)):
            value = record(summary); value['counts']['layers'][key] = bad
            with self.subTest(key=key), self.assertRaises(ValueError):
                N.validate_record(R.wire(value) + b'\n', summary)
        value = record(summary); local = (1 << 53) + 3
        value['counts']['layers'].update(local_checkpoints=local + 21888,
            before_calls=5472 + 2 * local + 21888, after_calls=5472 + 2 * local + 21888,
            generation_probes=4104 + 2 * local + 43776)
        got = N.validate_record(R.wire(value) + b'\n', summary)
        self.assertEqual(got['counts']['layers']['local_checkpoints'], local + 21888)

    def test_census_pair_refuses_subset_relabel_and_each_forward_timeline_gap(self):
        for key in ('allocation_preflights_changed', 'census_counters_are_layer_subset'):
            value = setup_pair(); value[3][key] = False
            with self.subTest(key=key), self.assertRaises(ValueError): compare(value)
        original = setup_pair()
        for side in (2, 3):
            for position in range(40):
                value = list(original)
                value[side] = copy.deepcopy(original[side])
                value[side]['timing']['timeline']['forwards'][position]['prepare_write']['start_ns'] += 1
                with self.subTest(side=side, position=position), self.assertRaises(ValueError): compare(value)
