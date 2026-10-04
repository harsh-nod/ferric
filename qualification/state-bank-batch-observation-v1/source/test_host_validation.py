"""Synthetic protocol data using the frozen Four fixture, never GPU evidence."""
import copy
import hashlib
import json
import sys
from types import SimpleNamespace
import unittest

import intake as I
import host_validation as H


def fixtures():
    pins = I.D.Pins(); C, helpers = I.comparator(pins)
    base = I.D.package(pins, *I.COMPARISON); old = sys.modules.get('compare')
    try:
        sys.modules['compare'] = C
        fixture = I.D.load_module(pins, base / 'test_compare.py',
            '26375e5a5db4298afbe688d4e1a53b4772f05fae3059e3589723eb838d276d4a', 'host_four_test_fixture')
    finally:
        if old is None: sys.modules.pop('compare', None)
        else: sys.modules['compare'] = old
    pins.recheck(); return C, helpers, fixture


def encode(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()


def example(mode='autoregressive', policy='baseline'):
    C, helpers, F = fixtures()
    value, bodies, owner, retain = F.retained_candidate()
    observed, files = F.fixture(mode); raw = F.serialize(observed)
    value['native_files'] = {n: retain('/task/decode/' + n, b)
        for n, b in dict(files, **{'complete.json': raw}).items()}
    # Actual request files may be pretty JSON; the wrapper hashes serde's Config projection.
    request = dict(schema=H.REQUEST_SCHEMA, policy=policy, decode=observed['request'])
    value['request'] = retain('/task/request.json', json.dumps(request, indent=2).encode())
    cache, shared = H.POLICIES[policy]
    command = C.doc(lambda p, _: bodies[p['path']], value['command'])
    command['argv'].append('--observe-host-policy'); value['command'] = retain(value['command']['path'], encode(command))
    started = C.doc(lambda p, _: bodies[p['path']], value['started'])
    started['command_sha256'] = value['command']['sha256']; value['started'] = retain(value['started']['path'], encode(started))
    old_stderr = bodies[value['stderr']['path']]
    if mode == 'teacher_forced': old_stderr = old_stderr.replace(b'Autoregressive', b'TeacherForced')
    value['stderr'] = retain(value['stderr']['path'], old_stderr)
    snapshots = []
    for i, phase in enumerate(H.PHASES):
        ranks = []
        for rank, device in enumerate(observed['request']['device_ids']):
            counters = [(rank + 1) * i * (j + 1) for j in range(19)]; counters[4:6] = [0, 0]
            ranks.append(dict(rank=rank, unique_id=device, queue_epoch=2 + rank,
                cache_kernel_admission=cache, raw_timestamp_queue=False, counters=counters))
        snapshots.append(dict(phase=phase, group_incarnation=5, shared_full_currentness=shared,
            ranks=ranks, shared=[i * j for j in (1, 2, 3, 4)]))
    intervals = [dict(host_elapsed_ns=1000, shared=[n - o for n, o in zip(now['shared'], old['shared'])],
        ranks=[[n - o for n, o in zip(now['ranks'][r]['counters'], old['ranks'][r]['counters'])] for r in range(2)])
        for old, now in zip(snapshots, snapshots[1:])]
    completions = [{k: v for k, v in frame['response']['event'].items() if k != 'status'}
                   for frame in observed['files']['frames']]
    report = dict(schema='FerricPrefixDecodeHostObservationV2', policy=policy, bootstrap=copy.deepcopy(observed['bootstrap']),
        worker_sha256=observed['request']['worker']['sha256'][:], child_pid=observed['child_pid'],
        profile_sha256=observed['profile_sha256'][:], snapshots=snapshots, intervals=intervals,
        forward_host_ns=[100, 200, 300, 400], close_host_ns=500, completions=completions,
        transcript_sha256=observed['transcript_sha256'][:], native_closed=True, inclusive_nested_host_scopes=True,
        gpu_time=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    wrapper = dict(schema='FerricFinitePrefixDecodeHostDiagnosticV2', policy=policy, parent_binary=value['parent'],
        request_projection_sha256=list(hashlib.sha256(b'{"schema":"FerricFinitePrefixDecodeHostPolicyRequestV2","policy":'
            + json.dumps(policy).encode('ascii') + b',"decode":' + H.member(raw, 'request') + b'}').digest()),
        observation=observed, host_sidecar=None, inclusive_nested_host_scopes=True,
        gpu_time=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    f = SimpleNamespace(C=C, helpers=helpers, F=F, value=value, bodies=bodies, owner=owner,
        retain=retain, observed=observed, request=request, policy=policy, raw=raw, report=report, wrapper=wrapper, command=command, started=started)
    refresh(f); return f


def refresh(f):
    f.value['host_sidecar'] = f.retain('/task/decode-host-policy-v2.json', encode(f.report))
    f.wrapper['host_sidecar'] = f.value['host_sidecar']
    f.value['stdout'] = f.retain('/task/stdout', encode(f.wrapper) + b'\n')
    f.owner.update({n: f.value[n] for n in ('command', 'started', 'stdout', 'stderr')})
    f.value['owner'] = f.retain('/task/result.json', encode(f.owner))


def validate(f):
    f.helpers['decode_validation'].validate(f.raw, {n: f.bodies[p['path']]
        for n, p in f.value['native_files'].items() if n != 'complete.json'})
    return H.validate(f.C, f.bodies[f.value['stdout']['path']], encode(f.report), f.value['host_sidecar'],
                      f.raw, f.observed, f.value['parent'], f.request)


class HostValidationTests(unittest.TestCase):
    def test_three_policies_two_modes_require_each_snapshot_to_match(self):
        for policy in H.POLICIES:
            for mode in ('teacher_forced', 'autoregressive'):
                f = example(mode, policy)
                self.assertEqual(validate(f)['policy'], policy)
                for index in range(7):
                    for kind in ('cache', 'shared', 'raw', 'operational'):
                        bad = copy.deepcopy(f.report)
                        snapshot = bad['snapshots'][index]
                        if kind == 'cache':
                            snapshot['ranks'][1]['cache_kernel_admission'] = not H.POLICIES[policy][0]
                        elif kind == 'shared':
                            snapshot['shared_full_currentness'] = not H.POLICIES[policy][1]
                        elif kind == 'raw':
                            snapshot['ranks'][0]['raw_timestamp_queue'] = True
                        else:
                            snapshot['ranks'][0]['counters'][4] = 1
                        with self.subTest(policy=policy, mode=mode, index=index, kind=kind), self.assertRaises(RuntimeError):
                            H.report(encode(bad), f.observed, policy)

    def test_v2_schema_and_explicit_policy_have_no_v1_or_combined_fallback(self):
        f = example()
        for name in ('both', 'auto', '', 'baseline ', 'operational-currentness', 'raw-timestamps'):
            bad = copy.deepcopy(f.request)
            bad['policy'] = name
            with self.assertRaises(RuntimeError):
                H.request(bad)
        for key in ('schema', 'policy', 'decode'):
            bad = copy.deepcopy(f.request)
            bad.pop(key)
            with self.assertRaises(RuntimeError):
                H.request(bad)
        bad = copy.deepcopy(f.report)
        bad['schema'] = 'FerricPrefixDecodeHostObservationV1'
        bad.pop('policy')
        with self.assertRaises(RuntimeError):
            H.report(encode(bad), f.observed, 'baseline')
        for other in H.POLICIES:
            if other != f.policy:
                with self.assertRaises(RuntimeError):
                    H.report(encode(f.report), f.observed, other)

    def test_full_request_digest_and_wrapper_policy_cannot_be_substituted(self):
        f = example(policy='immutable-admission-cache')
        for change in ('old-config-digest', 'wrapper-policy', 'external-policy', 'external-decode'):
            wrapper, request = copy.deepcopy(f.wrapper), copy.deepcopy(f.request)
            if change == 'old-config-digest':
                wrapper['request_projection_sha256'] = list(hashlib.sha256(H.member(f.raw, 'request')).digest())
            elif change == 'wrapper-policy':
                wrapper['policy'] = 'baseline'
            elif change == 'external-policy':
                request['policy'] = 'shared-full-currentness'
            else:
                request['decode']['session'][0] ^= 1
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                H.validate(f.C, encode(wrapper) + b'\n', encode(f.report), f.value['host_sidecar'],
                           f.raw, f.observed, f.value['parent'], request)

    def test_both_real_four_shapes_exact_metadata_and_nested_host_only_results(self):
        for mode in ('teacher_forced', 'autoregressive'):
            f = example(mode); checked = validate(f)
            self.assertEqual(checked['snapshots'], f.report['snapshots'])
            self.assertEqual(checked['counter_names'], list(H.COUNTERS)); self.assertEqual(len(H.COUNTERS), 19)
            self.assertEqual(len(checked['intervals']), 6); self.assertEqual(len(checked['shared_counter_names']), 4)
            self.assertFalse(checked['gpu_time']); self.assertFalse(checked['calibrated_device_time'])
            self.assertTrue(checked['inclusive_nested_host_scopes'])
            self.assertNotEqual(hashlib.sha256(f.bodies[f.value['request']['path']]).digest(),
                                bytes(f.wrapper['request_projection_sha256']))

    def test_counter_decrease_overflow_boolean_and_each_delta_word_refuse(self):
        f = example()
        for rank in range(2):
            for word in range(19):
                bad = copy.deepcopy(f.report); bad['intervals'][2]['ranks'][rank][word] += 1
                with self.subTest(rank=rank, word=word), self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)
        for value in (-1, 1 << 64, True):
            bad = copy.deepcopy(f.report); bad['snapshots'][3]['ranks'][0]['counters'][0] = value
            with self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)
        bad = copy.deepcopy(f.report); bad['snapshots'][3]['shared'][0] = 0
        with self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)

    def test_identity_phase_policy_queue_and_freshness_mutations_refuse(self):
        f = example()
        for key in ('phase', 'group', 'rank', 'device', 'epoch', 'shared', 'cache', 'raw', 'operational', 'fresh'):
            bad = copy.deepcopy(f.report); row = bad['snapshots'][2]
            if key == 'phase': row['phase'] = 'forward_3'
            elif key == 'group': row['group_incarnation'] += 1
            elif key == 'rank': row['ranks'][0]['rank'] = 1
            elif key == 'device': row['ranks'][0]['unique_id'] += 1
            elif key == 'epoch': row['ranks'][1]['queue_epoch'] += 1
            elif key == 'shared': row['shared_full_currentness'] = True
            elif key == 'cache': row['ranks'][1]['cache_kernel_admission'] = True
            elif key == 'raw': row['ranks'][1]['raw_timestamp_queue'] = True
            elif key == 'operational': row['ranks'][0]['counters'][4] = 1
            else: bad['snapshots'][0]['ranks'][0]['counters'][0] = 1
            with self.subTest(key=key), self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)

    def test_every_completion_profile_worker_transcript_close_binding_refuses(self):
        f = example()
        for i in range(4):
            for field in ('generation', 'input_token', 'output_token', 'control', 'capture', 'chain'):
                bad = copy.deepcopy(f.report)
                if type(bad['completions'][i][field]) is int: bad['completions'][i][field] += 1
                else: bad['completions'][i][field] = None
                with self.subTest(i=i, field=field), self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)
        for field in ('bootstrap', 'worker_sha256', 'child_pid', 'profile_sha256', 'transcript_sha256', 'native_closed'):
            bad = copy.deepcopy(f.report)
            if field == 'bootstrap': bad[field]['scope']['group_id'] = 33
            elif field == 'native_closed': bad[field] = False
            elif field == 'child_pid': bad[field] += 1
            else: bad[field] = [0] * 32
            with self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)

    def test_closed_json_scopes_claims_and_wall_bounds(self):
        f = example()
        for field in H.FALSE:
            bad = copy.deepcopy(f.report); bad[field] = True
            with self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)
        for name in ('snapshot', 'interval', 'rank', 'root', 'wall', 'close'):
            bad = copy.deepcopy(f.report)
            if name == 'snapshot': bad['snapshots'][0]['extra'] = 1
            elif name == 'interval': bad['intervals'][0]['extra'] = 1
            elif name == 'rank': bad['snapshots'][0]['ranks'][0]['extra'] = 1
            elif name == 'root': bad['extra'] = 1
            elif name == 'wall': bad['forward_host_ns'][0] = 1001
            else: bad['close_host_ns'] = True
            with self.assertRaises(RuntimeError): H.report(encode(bad), f.observed, f.policy)
        for raw in (b'', b' ' * 65537, encode(f.report).replace(b'"schema":', b'"schema":0,"schema":', 1)):
            with self.assertRaises(RuntimeError): H.report(raw, f.observed, f.policy)

    def test_wrapper_actual_parent_sidecar_projection_and_exact_native_bytes(self):
        f = example()
        for field in ('schema', 'parent_binary', 'host_sidecar', 'request_projection_sha256', 'observation', 'gpu_time'):
            bad = copy.deepcopy(f.wrapper)
            if field == 'schema': bad[field] = 'ordinary'
            elif field in ('parent_binary', 'host_sidecar'): bad[field]['sha256'] = '00' * 32
            elif field == 'request_projection_sha256': bad[field] = [0] * 32
            elif field == 'observation': bad[field]['setup_commands'] += 1
            else: bad[field] = True
            with self.assertRaises(RuntimeError):
                H.validate(f.C, encode(bad) + b'\n', encode(f.report), f.value['host_sidecar'], f.raw, f.observed, f.value['parent'], f.request)
        stdout = f.bodies[f.value['stdout']['path']]
        for raw in (stdout[:-1], stdout + b'\n', b'{}' + stdout, b'x' * 65537):
            with self.assertRaises((RuntimeError, ValueError)):
                H.validate(f.C, raw, encode(f.report), f.value['host_sidecar'], f.raw, f.observed, f.value['parent'], f.request)

    def test_decoder_member_exact_unicode_escaping_order_and_boolean_distinction(self):
        raw = b'{"other":{"request":1},"request":{"x":"a\\\"b","u":"\\u03bb"}}\n'
        self.assertEqual(H.member(raw, 'request'), b'{"x":"a\\\"b","u":"\\u03bb"}')
        self.assertFalse(H.same({'x': 1}, {'x': True}))
        for bad in (b'{"request":1,"request":2}', b'{}', b'{"request":NaN}'):
            with self.assertRaises((RuntimeError, ValueError)): H.member(bad, 'request')


if __name__ == '__main__': unittest.main()
