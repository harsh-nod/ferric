"""Synthetic matched-timing admission tests; no native or timing outcome."""
import copy
import hashlib
import json
from pathlib import Path
import unittest

import test_readiness as R
import test_shared as H
import test_timing as U
import validate_matched as M


def rebuild(value, rows):
    summary, request, bodies, _ = value
    seq = summary['bootstrap']['sequence']; base = request['base']
    profile = hashlib.sha256(b'ferric-prefix284-combined2208-closed-long-v2\0' + R.wire(seq)).digest()
    summary['profile_sha256'] = list(profile)
    chain = hashlib.sha256(b'ferric-guarded-mlp-long-transcript-v2\0' + profile).digest()
    sent = received = 0
    for row in rows:
        row['request']['session'] = base['session']
        row['request']['profile_sha256'] = list(profile)
        row['completion']['chain'] = [0] * 32
        chain = hashlib.sha256(chain + R.wire(row['request']) + R.wire(row['completion'])).digest()
        row['completion']['chain'] = list(chain)
        sent += 4 + len(R.wire(row['request'])); received += 4 + len(R.wire(row))
        if row['completion']['captured']:
            received += M.V.CONTROL_BYTES + M.V.PAYLOAD_BYTES
    closed = summary['close']
    closed['request']['session'] = base['session']; closed['request']['profile_sha256'] = list(profile)
    closed['transcript_sha256'] = summary['transcript_sha256'] = list(chain)
    setup = dict(protocol=1, id=1, device_ids=base['device_ids'], session=base['session'],
                 command=dict(op='begin', **seq['begin']))
    sent += 4 + len(R.wire(summary['bootstrap'])) + sum(seq[n]['bytes'] for n in
        ('prefix_image', 'mlp_image', 'projection_image', 'guarded_image'))
    sent += 4 + len(R.wire(setup)) + sum(seq['begin'][n]['bytes'] for n in
        ('registration', 'source_program', 'uploads', 'prefix_image', 'mlp_image', 'residual_image', 'tail_image'))
    sent += 4 + len(R.wire(closed['request'])); received += 4 + len(R.wire(closed))
    summary['request_stream_bytes'] = sent; summary['response_stream_bytes'] = received
    raw = b''.join(R.wire(row) + b'\n' for row in rows)
    path = summary['files']['frames']['path']; bodies[path] = raw
    summary['files']['frames'] = R.pin(path, raw)
    summary['files']['bytes_before_summary'] = sum(len(bodies[p]) for p in [
        summary['files']['frames']['path'], summary['files']['child_stderr']['path'],
        *[c['file']['path'] for c in summary['files']['captures']]])


def fixture(mode):
    original = R.fixture()
    root = Path('/synthetic/' + mode + '/native')
    summary = json.loads(R.wire(original[0]).replace(str(R.DIRECTORY).encode(), str(root).encode()))
    request = summary['request']
    bodies = {p.replace(str(R.DIRECTORY), str(root)): b for p, b in original[2].items()}
    value = (summary, request, bodies, original[3])
    pid = 201 if mode == 'default' else 202
    session = [4 if mode == 'default' else 5] * 32
    request['base']['session'] = session
    seq = summary['bootstrap']['sequence']
    for scope in (seq['scope'], seq['begin']['scope']):
        scope['session'] = session; scope['child_identity'] = pid
    summary['child_pid'] = pid
    rows = [json.loads(b) for b in bodies[summary['files']['frames']['path']].splitlines()]
    rebuild(value, rows)
    if mode == 'shared': H.install(value, R.wire(H.record(summary)) + b'\n')
    return make_case(mode, value, root)


def make_case(mode, value, root):
    summary, request, bodies, prompt = value
    raw = R.summary_bytes(summary)
    report = copy.deepcopy(U.timing_fixture()['report'])
    report.update(ordinary_complete=R.pin(root / 'complete.json', raw),
        profile_sha256=summary['profile_sha256'], transcript_sha256=summary['transcript_sha256'])
    wrapper = dict(schema=M.WRAPPERS[mode], observation=summary,
        host_timing=dict(complete=True, file=None, ordinary_retained_bytes=summary['files']['total_bytes'],
            retained_bytes_with_timing=0, supervisor_metadata_allowance=summary['files']['supervisor_metadata_allowance'],
            parent_host_measurement=True, gpu_timing=False, numerical_acceptance=False, performance_claim=False))
    if mode == 'shared': wrapper['currentness_policy'] = H.record(summary)
    case = dict(mode=mode, root=root, value=value, summary=raw, wrapper=wrapper, report=report)
    encode(case)
    return case


def encode(case):
    raw = R.wire(case['report']) + b'\n'
    path = case['root'] / 'host-timing.json'
    status = case['wrapper']['host_timing']; status['file'] = R.pin(path, raw)
    status['retained_bytes_with_timing'] = status['ordinary_retained_bytes'] + len(raw)
    case['value'][2][str(path)] = raw


def check(case):
    _, request, bodies, prompt = case['value']
    return M.validate(case['mode'], R.wire(case['wrapper']), case['summary'], request,
        case['root'], lambda p: bodies[p['path']], prompt)


def compare(default, shared, admissions=None):
    left, right = check(default), check(shared)
    bodies = {**default['value'][2], **shared['value'][2]}
    worker = M.rust_pin(default['value'][1]['base']['worker'])
    fixed = dict(worker=worker, worker_cpu=dict(path='/qualified/worker-complete.json', bytes=123, sha256='a' * 64),
        parent=dict(path='/qualified/parent', bytes=456, sha256='b' * 64),
        parent_cpu=dict(path='/qualified/parent-complete.json', bytes=789, sha256='c' * 64))
    if admissions is None: admissions = dict(default=copy.deepcopy(fixed), shared=copy.deepcopy(fixed))
    return M.compare_pair(default['summary'], shared['summary'], left, right, admissions,
        lambda p: bodies[p['path']])


class MatchedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.default = fixture('default'); cls.shared = fixture('shared')

    def test_both_original_modes_join_policy_timeline_and_false_authority(self):
        for mode in ('default', 'shared'):
            with self.subTest(mode=mode):
                case = copy.deepcopy(getattr(self, mode)); before = copy.deepcopy(case['value'][2])
                got = check(case)
                self.assertEqual(got['mode'], mode)
                self.assertEqual(got['timing']['disjoint_spans'], 124)
                self.assertEqual(got['timing']['timeline']['total_ns'], 868)
                self.assertEqual(got['ordinary']['completed_forwards'], 40)
                self.assertEqual(before, case['value'][2])
                self.assertTrue(got['policy']['no_policy_bytes_discarded'])
                self.assertFalse(got['performance_claim'] or got['numerical_acceptance'])
                self.assertFalse(got['outer_owned_lineage_checked'] or got['cpu_qualification_checked'])

    def test_explicit_mode_and_exact_wrapper_cannot_be_inferred_or_relabelled(self):
        for mode in (None, True, 'auto', 'full2303'):
            case = copy.deepcopy(self.default); case['mode'] = mode
            with self.subTest(mode=mode), self.assertRaises(ValueError): check(case)
        for original in (self.default, self.shared):
            for name in ('schema', 'extra'):
                case = copy.deepcopy(original); case['wrapper'][name] = 'not-the-selected-wrapper'
                with self.subTest(mode=case['mode'], field=name), self.assertRaises(ValueError): check(case)
        case = copy.deepcopy(self.shared); case['wrapper']['schema'] = M.WRAPPERS['default']
        with self.assertRaises(ValueError): check(case)

    def test_default_empty_stderr_and_shared_original_record_remain_separate(self):
        case = copy.deepcopy(self.shared)
        with self.assertRaises(ValueError):
            M.V.validate(case['summary'], case['value'][1], case['root'],
                lambda p: case['value'][2][p['path']], case['value'][3])
        case = copy.deepcopy(self.default); case['mode'] = 'shared'
        case['wrapper']['schema'] = M.WRAPPERS['shared']
        case['wrapper']['currentness_policy'] = H.record(case['value'][0])
        with self.assertRaises(ValueError): check(case)
        case = copy.deepcopy(self.shared); case['mode'] = 'default'
        case['wrapper']['schema'] = M.WRAPPERS['default']; del case['wrapper']['currentness_policy']
        with self.assertRaises(ValueError): check(case)

    def test_combined_policy_joins_original_worker_transcript_and_flags(self):
        for field in ('worker_sha256', 'session', 'transcript_sha256'):
            case = copy.deepcopy(self.shared); case['wrapper']['currentness_policy'][field][0] ^= 1
            with self.subTest(field=field), self.assertRaises(ValueError): check(case)
        for field in ('shared_full_currentness', 'native_closed', 'paired_terminal', 'host_observer'):
            case = copy.deepcopy(self.shared)
            case['wrapper']['currentness_policy'][field] = not case['wrapper']['currentness_policy'][field]
            with self.subTest(field=field), self.assertRaises(ValueError): check(case)

    def test_original_policy_missing_suffix_changed_bytes_and_budgets_refuse(self):
        for change in ('missing', 'suffix', 'truncated', 'accounting'):
            case = copy.deepcopy(self.shared); value = case['value']
            path = value[0]['files']['child_stderr']['path']
            if change == 'missing': del value[2][path]
            elif change == 'suffix': value[2][path] += b'\n'
            elif change == 'truncated': value[2][path] = value[2][path][:-1]
            else:
                value[0]['files']['bytes_before_summary'] -= 1
                case['summary'] = R.summary_bytes(value[0])
            with self.subTest(change=change), self.assertRaises((ValueError, KeyError)): check(case)

    def test_full_ordinary_frames_captures_and_close_precede_timing(self):
        for change in ('frame', 'capture', 'close'):
            case = copy.deepcopy(self.shared); value = case['value']
            if change == 'frame':
                path = value[0]['files']['frames']['path']; value[2][path] += b'{}\n'
            elif change == 'capture': value[2][value[0]['files']['captures'][2]['file']['path']] = b''
            else:
                value[0]['native_closed'] = False; case['summary'] = R.summary_bytes(value[0])
            with self.subTest(change=change), self.assertRaises(ValueError): check(case)

    def test_shared_sidecar_actual_path_hash_extent_and_ordinary_pin(self):
        for field, wrong in (('path', '/other/host-timing.json'), ('bytes', 65537), ('sha256', [0] * 32)):
            case = copy.deepcopy(self.shared); case['wrapper']['host_timing']['file'][field] = wrong
            with self.subTest(field=field), self.assertRaises((ValueError, KeyError)): check(case)
        case = copy.deepcopy(self.shared); case['report']['ordinary_complete']['sha256'][0] ^= 1; encode(case)
        with self.assertRaises(ValueError): check(case)

    def test_shared_timeline_strict_scalars_order_and_exact_disjoint_sum(self):
        for value in (True, 1.0, -1, 1 << 64):
            case = copy.deepcopy(self.shared); case['report']['timeline']['total_ns'] = value; encode(case)
            with self.subTest(value=value), self.assertRaises(ValueError): check(case)
        for field in ('position', 'generation', 'elapsed_ns'):
            case = copy.deepcopy(self.shared); case['report']['timeline']['forwards'][5][field] += 1; encode(case)
            with self.subTest(field=field), self.assertRaises(ValueError): check(case)
        case = copy.deepcopy(self.shared)
        case['report']['timeline']['forwards'][0]['prepare_write']['start_ns'] += 1; encode(case)
        with self.assertRaises(ValueError): check(case)

    def test_shared_timing_retention_close_and_false_authority_are_not_relaxed(self):
        for field in M.T.FALSE_FIELDS:
            case = copy.deepcopy(self.shared); case['report'][field] = True; encode(case)
            with self.subTest(field=field), self.assertRaises(ValueError): check(case)
        for field in ('native_closed', 'child_exit_zero', 'process_group_absent'):
            case = copy.deepcopy(self.shared); case['report'][field] = False; encode(case)
            with self.subTest(field=field), self.assertRaises(ValueError): check(case)
        case = copy.deepcopy(self.shared); case['wrapper']['host_timing']['retained_bytes_with_timing'] = 32 << 20
        with self.assertRaises(ValueError): check(case)

    def test_matched_pair_requires_same_all_four_cpu_and_product_pins(self):
        default, shared = copy.deepcopy(self.default), copy.deepcopy(self.shared)
        good = compare(default, shared)
        self.assertTrue(good['passed'] and good['parity']['all40_records_equal'] and good['parity']['all4_payloads_byte_equal'])
        self.assertFalse(good['performance_claim'] or good['cpu_qualification_checked'])
        for role in ('worker', 'worker_cpu', 'parent', 'parent_cpu'):
            pins = dict(default=copy.deepcopy(good['same_elf_cpu_metadata']), shared=copy.deepcopy(good['same_elf_cpu_metadata']))
            pins['shared'][role]['sha256'] = 'd' * 64
            with self.subTest(role=role), self.assertRaises(ValueError): compare(default, shared, pins)
        pins = dict(default=copy.deepcopy(good['same_elf_cpu_metadata']), shared=copy.deepcopy(good['same_elf_cpu_metadata']))
        pins['default']['parent']['bytes'] = pins['shared']['parent']['bytes'] = True
        with self.assertRaises(ValueError): compare(default, shared, pins)

    def test_pair_refuses_same_worker_identity_or_same_mode(self):
        default, shared = copy.deepcopy(self.default), copy.deepcopy(self.shared)
        a, b = check(default), check(shared)
        pins = compare(default, shared)['same_elf_cpu_metadata']
        admissions = dict(default=pins, shared=pins)
        read = lambda p: {**default['value'][2], **shared['value'][2]}[p['path']]
        b['mode'] = 'default'
        with self.assertRaises(ValueError): M.compare_pair(default['summary'], shared['summary'], a, b, admissions, read)
        b['mode'] = 'shared'
        raw = json.loads(shared['summary']); raw['child_pid'] = default['value'][0]['child_pid']
        changed = R.wire(raw); b['summary'] = M.summary_pin(changed, shared['root'])
        with self.assertRaises(ValueError): M.compare_pair(default['summary'], changed, a, b, admissions, read)

    def test_pair_checks_every_semantic_record_and_complete_selected_payload(self):
        for change in ('unselected-output', 'selected-payload'):
            default, shared = copy.deepcopy(self.default), copy.deepcopy(self.shared)
            value = shared['value']; summary, _, bodies, _ = value
            rows = [json.loads(b) for b in bodies[summary['files']['frames']['path']].splitlines()]
            if change == 'unselected-output': rows[11]['completion']['output_token'] = 4
            else:
                capture = summary['files']['captures'][1]; path = capture['file']['path']
                raw = bytearray(bodies[path]); raw[M.V.CONTROL_BYTES] ^= 1; bodies[path] = bytes(raw)
                capture['file'] = R.pin(path, bodies[path])
                rows[5]['completion']['observation'] = R.part(bodies[path][M.V.CONTROL_BYTES:])
            rebuild(value, rows)
            H.install(value, R.wire(H.record(summary)) + b'\n')
            shared = make_case('shared', value, shared['root'])
            check(shared)
            with self.subTest(change=change), self.assertRaises(ValueError): compare(default, shared)
