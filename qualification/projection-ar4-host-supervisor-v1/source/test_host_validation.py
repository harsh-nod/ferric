"""Synthetic projection sidecar tests; the native AR4 fixture is independently checked."""
import copy
import hashlib
import json
from pathlib import Path
import unittest

import decode_validation as CV
import host_validation as H
import test_decode_validation as F


def encode(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False).encode()


def pin(path, raw):
    return dict(path=path, bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def fixture():
    observed, bodies = F.fixture(); summary = F.serialize(observed)
    snapshots = []
    for i, phase in enumerate(H.PHASES):
        ranks = []
        for rank, device in enumerate(observed['request']['decode']['device_ids']):
            counters = [(rank + 1) * i * (j + 1) for j in range(19)]; counters[4:6] = [0, 0]
            ranks.append(dict(rank=rank, unique_id=device, queue_epoch=rank + 2,
                cache_kernel_admission=False, raw_timestamp_queue=False, counters=counters))
        snapshots.append(dict(phase=phase, group_incarnation=5, shared_full_currentness=False,
                              ranks=ranks, shared=[i * j for j in (1, 2, 3, 4)]))
    intervals = [dict(host_elapsed_ns=1000 + index, shared=[n - p for n, p in zip(now['shared'], old['shared'])],
        ranks=[[n - p for n, p in zip(now['ranks'][r]['counters'], old['ranks'][r]['counters'])] for r in range(2)])
        for index, (old, now) in enumerate(zip(snapshots, snapshots[1:]))]
    report = dict(schema='FerricProjectionResidualDecodeHostObservationV1', bootstrap=copy.deepcopy(observed['bootstrap']),
        worker_sha256=observed['request']['decode']['worker']['sha256'][:], child_pid=observed['child_pid'],
        profile_sha256=observed['profile_sha256'][:], snapshots=snapshots, intervals=intervals,
        forward_host_ns=[100, 200, 300, 400], serialization_host_ns=[10, 20, 30, 40], close_host_ns=500,
        completions=[{k: v for k, v in frame['response']['event'].items() if k != 'status'}
                     for frame in observed['files']['frames']],
        transcript_sha256=observed['transcript_sha256'][:], native_closed=True, inclusive_nested_host_scopes=True,
        gpu_time=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    parent = pin('/task/observer-parent', b'parent')
    return dict(observed=observed, bodies=bodies, summary=summary, report=report, parent=parent)


def wrapper(f):
    raw = encode(f['report']); path = Path(f['observed']['request']['decode']['evidence_directory'])
    sidecar = pin(str(path.with_name(path.name + '-projection-host-observation.json')), raw)
    rust_pin = lambda p: dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    value = dict(schema='FerricFiniteProjectionResidualDecodeHostDiagnosticV1', parent_binary=rust_pin(f['parent']),
        request_projection_sha256=list(hashlib.sha256(H.member(f['summary'], 'request')).digest()),
        observation=f['observed'], host_sidecar=rust_pin(sidecar), inclusive_nested_host_scopes=True,
        gpu_time=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    return value, raw, sidecar


def validate(f):
    value, raw, sidecar = wrapper(f)
    return H.validate(encode(value) + b'\n', raw, sidecar, f['summary'], f['observed'], f['parent'])


class HostTests(unittest.TestCase):
    def test_projection_ar4_native_then_exact_sidecar(self):
        f = fixture(); checked = CV.validate(f['summary'], f['bodies'], f['observed']['request'])
        self.assertEqual(checked['captured_tensor_rows'], 152)
        host = validate(f)
        self.assertEqual(host['forward_host_ns'], [100, 200, 300, 400])
        self.assertEqual(host['serialization_host_ns'], [10, 20, 30, 40])
        self.assertEqual((len(host['counter_names']), len(host['shared_counter_names'])), (19, 4))
        self.assertFalse(host['gpu_time']); self.assertFalse(host['performance_claim'])

    def test_projection_schema_and_ar_mode_are_not_legacy_aliases(self):
        f = fixture()
        for schema in ('FerricPrefixDecodeHostObservationV1', 'FerricPrefixDecodeHostObservationV2'):
            bad = copy.deepcopy(f['report']); bad['schema'] = schema
            with self.assertRaises(RuntimeError): H.report(encode(bad), f['observed'])
        f['observed']['request']['decode']['mode'] = 'teacher_forced'
        with self.assertRaises(RuntimeError): H.report(encode(f['report']), f['observed'])

    def test_worker_projection_profile_completion_and_close_joins(self):
        f = fixture()
        for key in ('worker_sha256', 'profile_sha256', 'transcript_sha256', 'child_pid', 'native_closed', 'image', 'completion'):
            bad = copy.deepcopy(f['report'])
            if key == 'child_pid': bad[key] += 1
            elif key == 'native_closed': bad[key] = False
            elif key == 'image': bad['bootstrap']['projection_residual_image']['sha256'][0] ^= 1
            elif key == 'completion': bad['completions'][3]['output_token'] += 1
            else: bad[key] = [0] * 32
            with self.subTest(key=key), self.assertRaises(RuntimeError): H.report(encode(bad), f['observed'])

    def test_full_currentness_policy_and_rank_epoch_are_mandatory(self):
        f = fixture()
        for key in ('shared_full_currentness', 'cache_kernel_admission', 'raw_timestamp_queue', 'queue_epoch', 'operational'):
            bad = copy.deepcopy(f['report']); s = bad['snapshots'][3]
            if key == 'shared_full_currentness': s[key] = True
            elif key == 'operational': s['ranks'][0]['counters'][4] = 1
            elif key == 'queue_epoch': s['ranks'][0][key] += 1
            else: s['ranks'][0][key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): H.report(encode(bad), f['observed'])

    def test_deltas_u64s_fresh_snapshot_and_false_claims(self):
        f = fixture()
        for key in ('delta', 'fresh', 'negative', 'overflow', 'boolean', *H.FALSE):
            bad = copy.deepcopy(f['report'])
            if key == 'delta': bad['intervals'][1]['ranks'][0][1] += 1
            elif key == 'fresh': bad['snapshots'][0]['shared'][0] = 1
            elif key in ('negative', 'overflow', 'boolean'):
                bad['snapshots'][1]['ranks'][0]['counters'][0] = {'negative': -1, 'overflow': 1 << 64, 'boolean': True}[key]
            else: bad[key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): H.report(encode(bad), f['observed'])

    def test_serialization_uses_following_interval_not_forward_interval(self):
        f = fixture(); f['report']['serialization_host_ns'][0] = 1002
        H.report(encode(f['report']), f['observed'])
        f['report']['serialization_host_ns'][0] = 1003
        with self.assertRaises(RuntimeError): H.report(encode(f['report']), f['observed'])
        f = fixture(); f['report']['forward_host_ns'][0] = 1002
        with self.assertRaises(RuntimeError): H.report(encode(f['report']), f['observed'])

    def test_wrapper_exact_projection_sidecar_pin_and_native_bytes(self):
        f = fixture(); value, raw, sidecar = wrapper(f)
        self.assertEqual(H.pin(value['parent_binary']), f['parent'])
        self.assertEqual(H.pin(value['host_sidecar']), sidecar)
        for key in ('schema', 'parent_binary', 'host_sidecar', 'request_projection_sha256', 'observation', 'gpu_time'):
            bad = copy.deepcopy(value)
            if key == 'schema': bad[key] = 'FerricFinitePrefixDecodeHostDiagnosticV1'
            elif key in ('parent_binary', 'host_sidecar'): bad[key]['sha256'] = '00' * 32
            elif key == 'request_projection_sha256': bad[key] = [0] * 32
            elif key == 'observation': bad[key]['setup_commands'] += 1
            else: bad[key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                H.validate(encode(bad) + b'\n', raw, sidecar, f['summary'], f['observed'], f['parent'])
        with self.assertRaises(RuntimeError):
            H.validate(encode(value) + b'\n', raw + b' ', sidecar, f['summary'], f['observed'], f['parent'])

    def test_closed_json_and_exact_member_preserve_u64(self):
        raw = b'{"request":{"id":18446744073709551615},"nested":{"request":0}}\n'
        self.assertEqual(H.member(raw, 'request'), b'{"id":18446744073709551615}')
        self.assertFalse(H.same({'x': 1}, {'x': True}))
        f = fixture()
        for raw in (b'', b' ' * 65537, encode(f['report']).replace(b'"schema":', b'"schema":0,"schema":', 1)):
            with self.assertRaises((RuntimeError, ValueError)): H.report(raw, f['observed'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
