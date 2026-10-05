"""Shared-envelope refusals; legacy default-full tests remain separately selected."""
import copy
import hashlib
from pathlib import Path
import unittest

import host_validation as H
import test_host_validation as F


def fixture():
    value = F.fixture()
    value['report']['schema'] = 'FerricProjectionResidualDecodeSharedHostObservationV1'
    for row in value['report']['snapshots']:
        row['shared_full_currentness'] = True
    return value


def envelope(f, configuration=7000):
    return dict(schema='FerricProjectionResidualDecodeSharedHostEnvelopeV1', policy='shared-full',
                configuration_host_ns=configuration, observation=f['report'])


def wrapper(f):
    raw = F.encode(envelope(f))
    path = Path(f['observed']['request']['decode']['evidence_directory'])
    sidecar = F.pin(str(path.with_name(path.name + '-projection-shared-host-observation.json')), raw)
    rust = lambda p: dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    value = dict(schema='FerricFiniteProjectionResidualDecodeSharedHostDiagnosticV1',
        parent_binary=rust(f['parent']), request_projection_sha256=list(hashlib.sha256(H.member(f['summary'], 'request')).digest()),
        observation=f['observed'], host_sidecar=rust(sidecar), inclusive_nested_host_scopes=True,
        gpu_time=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    return value, raw, sidecar


class SharedHostTests(unittest.TestCase):
    def test_shared_envelope_configuration_and_exact_native_bytes(self):
        f = fixture(); value, raw, sidecar = wrapper(f)
        result = H.validate_shared(F.encode(value) + b'\n', raw, sidecar, f['summary'], f['observed'], f['parent'])
        self.assertEqual(result['policy'], 'shared-full')
        self.assertEqual(result['configuration_host_ns'], 7000)
        self.assertFalse(result['configuration_time_in_snapshots'])
        self.assertEqual(result['forward_host_ns'], [100, 200, 300, 400])
        self.assertTrue(all(s['shared_full_currentness'] for s in result['snapshots']))

    def test_legacy_default_and_shared_decoders_cannot_cross_admit(self):
        f = fixture()
        with self.assertRaises(RuntimeError): H.report(F.encode(f['report']), f['observed'])
        with self.assertRaises(RuntimeError): H.report(F.encode(envelope(f)), f['observed'])
        old = F.fixture()
        with self.assertRaises(RuntimeError): H.shared_report(F.encode(old['report']), old['observed'])
        bad = envelope(old)
        with self.assertRaises(RuntimeError): H.shared_report(F.encode(bad), old['observed'])

    def test_every_snapshot_requires_shared_true(self):
        f = fixture()
        for index in range(7):
            bad = copy.deepcopy(f)
            bad['report']['snapshots'][index]['shared_full_currentness'] = False
            with self.subTest(index=index), self.assertRaises(RuntimeError):
                H.shared_report(F.encode(envelope(bad)), bad['observed'])

    def test_configuration_is_u64_and_envelope_is_closed(self):
        f = fixture()
        for bad in (-1, True, 1 << 64, '7'):
            with self.subTest(value=bad), self.assertRaises(RuntimeError):
                H.shared_report(F.encode(envelope(f, bad)), f['observed'])
        raw = envelope(f); raw['unknown'] = 0
        with self.assertRaises(RuntimeError): H.shared_report(F.encode(raw), f['observed'])
        with self.assertRaises(RuntimeError): H.shared_report(b' ' * 65537, f['observed'])

    def test_policy_and_both_schemas_are_exact(self):
        f = fixture()
        for key, value in (('policy', 'default-full'), ('policy', True), ('schema', 'old')):
            bad = envelope(f); bad[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                H.shared_report(F.encode(bad), f['observed'])
        f['report']['schema'] = 'FerricProjectionResidualDecodeHostObservationV1'
        with self.assertRaises(RuntimeError): H.shared_report(F.encode(envelope(f)), f['observed'])

    def test_configuration_never_changes_zero_baseline_or_interval_accounting(self):
        f = fixture()
        report, configuration = H.shared_report(F.encode(envelope(f, H.U64)), f['observed'])
        self.assertEqual(configuration, H.U64)
        self.assertEqual(report['snapshots'][0]['shared'], [0] * 4)
        self.assertEqual(report['intervals'], f['report']['intervals'])
        self.assertEqual(report['forward_host_ns'], f['report']['forward_host_ns'])

    def test_shared_wrapper_path_pin_schema_and_projection_are_bound(self):
        f = fixture(); value, raw, sidecar = wrapper(f)
        for key in ('schema', 'host_sidecar', 'parent_binary', 'request_projection_sha256'):
            bad = copy.deepcopy(value)
            if key == 'schema': bad[key] = 'FerricFiniteProjectionResidualDecodeHostDiagnosticV1'
            elif key == 'request_projection_sha256': bad[key] = [0] * 32
            else: bad[key]['sha256'] = [0] * 32
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                H.validate_shared(F.encode(bad) + b'\n', raw, sidecar, f['summary'], f['observed'], f['parent'])
        with self.assertRaises(RuntimeError):
            H.validate(F.encode(value) + b'\n', raw, sidecar, f['summary'], f['observed'], f['parent'])

    def test_shared_retains_operational_cache_epoch_and_delta_refusals(self):
        f = fixture()
        for key in ('operational', 'cache', 'epoch', 'delta'):
            bad = copy.deepcopy(f); rank = bad['report']['snapshots'][3]['ranks'][0]
            if key == 'operational': rank['counters'][4] = 1
            elif key == 'cache': rank['cache_kernel_admission'] = True
            elif key == 'epoch': rank['queue_epoch'] += 1
            else: bad['report']['intervals'][1]['shared'][0] += 1
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                H.shared_report(F.encode(envelope(bad)), bad['observed'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
