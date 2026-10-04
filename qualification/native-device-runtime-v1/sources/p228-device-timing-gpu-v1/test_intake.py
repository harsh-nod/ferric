"""Synthetic intake policy tests; no native, CPU build or GPU invocation."""
import copy
import hashlib
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import intake as I


def pin(path='/example', body=b'body'):
    return dict(path=path, bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def plan():
    value = {name: pin('/' + name) for name in I.PLAN_FIELDS.split()}
    value.update(schema=I.INPUT_SCHEMA, policy='shared-full-currentness',
        output_label='prefix-device-timing-tf4-shared-full-currentness-gpu-v228-v1',
        standalone_cases=[pin('/case' + str(i)) for i in range(6)],
        numericals=[pin('/numerical' + str(i)) for i in range(6)])
    return value


def cpu(role):
    parent = role == 'parent'
    name = I.PARENT_NAME if parent else I.WORKER_NAME
    binary = pin('/qualified/target/debug/' + name)
    if not parent:
        binary.update(bytes=I.WORKER_BINARY[0], sha256=I.WORKER_BINARY[1])
    artifact = dict(reason='compiler-artifact', executable=binary['path'], filenames=[binary['path']],
        target=dict(name=name, kind=['bin'], crate_types=['bin']),
        features=['default', 'tp-batch-engineering'] if parent else [],
        profile=dict(opt_level='2', test=False, debug_assertions=True, overflow_checks=True))
    return dict(schema='ferric-p228-device-parent-cpu-result-v1' if parent
        else 'ferric-p228-device-routing-cpu-result-v1', passed=True, error=None, postcheck_errors=[],
        source_unchanged=True, empty_initial_target=True, tests_passed=257 if parent else 609,
        tests_ignored=0 if parent else 4, parent_rebuilt=parent, worker_rebuilt=not parent,
        compiler_hsaco_reproduced=False, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, timestamp_calibration=False, production_authority=False,
        phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(41 if parent else 25)},
        binaries={name: dict(artifact=artifact, binary=binary)})


def source():
    prefix = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/src/'
    return {prefix + name: dict(bytes=1, sha256='a' * 64) for name in (
        'prefix_decode_device_observation_v1.rs', 'prefix_decode_device_observation_v1_tests.rs',
        'finite_prefix_decode_wire_v1.rs', 'finite_setup_wire_v1.rs', 'finite_forward_wire_v1.rs')}


class Intake(unittest.TestCase):
    def test_closed_tf4_input_and_six_distinct_prerequisites(self):
        value = plan(); I.input_shape(value)
        for change in ('extra', 'policy', 'mode', 'duplicates', 'missing'):
            bad = copy.deepcopy(value)
            if change == 'extra': bad['deployment'] = pin()
            elif change == 'policy': bad['policy'] = 'baseline'
            elif change == 'mode': bad['output_label'] = value['output_label'].replace('tf4', 'ar4')
            elif change == 'duplicates': bad['standalone_cases'][1] = bad['standalone_cases'][0]
            else: del bad['parent_cpu']
            with self.assertRaises((RuntimeError, KeyError)): I.input_shape(bad)

    def test_device_request_has_no_host_policy_fallback(self):
        request = dict(schema='FerricFinitePrefixDecodeDeviceRequestV1', decode=dict(mode='teacher_forced'))
        self.assertEqual(I.device_request(request), request['decode'])
        for field, value in (('schema', 'FerricFinitePrefixDecodeHostPolicyRequestV2'),
                             ('policy', 'shared-full-currentness'), ('decode', dict(mode='autoregressive'))):
            bad = dict(request, **{field: value})
            with self.assertRaises(RuntimeError): I.device_request(bad)

    def test_qualified_worker_uses_actual_cpu609_artifact(self):
        value = cpu('worker')
        self.assertEqual(I.content(I.qualified_cpu(value, 'worker')['binary']), I.WORKER_BINARY)

    def test_qualified_parent_requires_new_binary_and_feature(self):
        value = cpu('parent'); I.qualified_cpu(value, 'parent')
        bad = copy.deepcopy(value); bad['binaries'][I.PARENT_NAME]['artifact']['features'] = []
        with self.assertRaises(RuntimeError): I.qualified_cpu(bad, 'parent')
        bad = copy.deepcopy(value); bad['binaries'] = {'old-host-parent': value['binaries'][I.PARENT_NAME]}
        with self.assertRaises(KeyError): I.qualified_cpu(bad, 'parent')

    def test_failed_partial_or_authoritative_cpu_receipt_rejected(self):
        for role in ('worker', 'parent'):
            for key, bad_value in (('passed', False), ('error', 'failure'), ('postcheck_errors', ['changed']),
                    ('source_unchanged', False), ('tests_passed', True), ('tests_ignored', 1),
                    ('gpu_execution', True), ('numerical_acceptance', True), ('performance_claim', True),
                    ('timestamp_calibration', True), ('parent_rebuilt', role != 'parent')):
                bad = cpu(role); bad[key] = bad_value
                with self.assertRaises(RuntimeError): I.qualified_cpu(bad, role)

    def test_missing_unreaped_or_failed_cpu_phase_rejected(self):
        for change in ('missing', 'exit', 'reason', 'group'):
            value = cpu('worker')
            if change == 'missing': del value['phases']['0']
            elif change == 'exit': value['phases']['0']['exit_code'] = False
            elif change == 'reason': value['phases']['0']['reason'] = 'deadline'
            else: value['phases']['0']['group_absent'] = False
            with self.assertRaises(RuntimeError): I.qualified_cpu(value, 'worker')

    def test_artifact_test_profile_disabled_assertions_or_wrong_identity_rejected(self):
        for key, bad_value in (('test', True), ('opt_level', '0'),
                               ('debug_assertions', False), ('overflow_checks', False)):
            value = cpu('worker'); value['binaries'][I.WORKER_NAME]['artifact']['profile'][key] = bad_value
            with self.assertRaises(RuntimeError): I.qualified_cpu(value, 'worker')
        value = cpu('worker'); value['binaries'][I.WORKER_NAME]['binary']['sha256'] = 'f' * 64
        with self.assertRaises(RuntimeError): I.qualified_cpu(value, 'worker')
        value = cpu('worker'); value['binaries'][I.WORKER_NAME]['artifact']['executable'] = '/other'
        with self.assertRaises(RuntimeError): I.qualified_cpu(value, 'worker')

    def test_pending_parent_receipt_fails_before_file_io(self):
        with patch.object(I, 'PARENT_CPU', None), patch.object(I, 'doc') as load:
            with self.assertRaises(RuntimeError): I.cpu_evidence(None, pin(), 'parent')
            load.assert_not_called()

    def test_shared_source_equality_ignores_unrelated_parent_files(self):
        worker = source()
        readme = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/README.md'
        worker[readme] = dict(bytes=8073, sha256='a' * 64)
        parent = dict(worker, **{'ferric/adapters/m1/src/new.rs': dict(bytes=2, sha256='b' * 64)})
        parent[readme] = dict(bytes=9000, sha256='b' * 64)
        I.shared_source(parent, worker)

    def test_shared_source_change_missing_member_or_unlisted_worker_addition_rejected(self):
        worker = source()
        for change in ('changed', 'missing', 'extra'):
            parent = copy.deepcopy(worker)
            if change == 'changed': parent[next(iter(parent))]['sha256'] = 'f' * 64
            elif change == 'missing': del parent[next(iter(parent))]
            else: parent['ferric/adapters/tp-peer-finite-engineering-worker-v1/src/extra.rs'] = dict(bytes=1, sha256='f' * 64)
            with self.assertRaises(RuntimeError): I.shared_source(parent, worker)

    def test_deployed_elf_matches_qualified_original_not_its_build_host_path(self):
        raw = b'\x7fELF\x02\x01' + bytes(12) + b'\x3e\x00'
        original = pin('/old/target/debug/' + I.PARENT_NAME, raw)
        deployed = dict(original, path=str(I.E / 'device-runtime-v228-v1' / I.PARENT_NAME))
        with patch.object(I, 'read', return_value=raw):
            self.assertEqual(I.deployed_binary(None, deployed, original), deployed)
            with self.assertRaises(RuntimeError): I.deployed_binary(None, dict(deployed, sha256='a' * 64), original)
            with self.assertRaises(RuntimeError): I.deployed_binary(None, dict(deployed, path='/tmp/' + I.PARENT_NAME), original)
        with patch.object(I, 'read', return_value=b'not an ELF'):
            with self.assertRaises(RuntimeError): I.deployed_binary(None, deployed, original)

    def test_old_baseline_digest_cannot_be_relabelled_as_new_case(self):
        value = pin(str(I.E / I.BASELINE_LABEL / 'complete.json'))
        with self.assertRaises(RuntimeError): I.baseline(None, value)

    def test_same_workload_allows_only_worker_fresh_session_and_output(self):
        original = dict(worker='old', session=[1], evidence_directory='/old', mode='teacher_forced', images={'tail': 1})
        new = dict(original, worker='new', session=[2], evidence_directory='/new')
        host = SimpleNamespace(same=lambda a, b: a == b)
        I.same_workload(new, dict(request=original), host)
        for bad in (dict(new, session=[1]), dict(new, mode='autoregressive'), dict(new, images={'tail': 2})):
            with self.assertRaises(RuntimeError): I.same_workload(bad, dict(request=original), host)

    def test_explicit_engineering_review_never_mints_numerical_or_timing_authority(self):
        p = plan(); runtime = {key: p[key] for key in ('parent', 'worker')}; runtime['image'] = pin('/image')
        prior = dict(runtime=dict(parent=pin('/old-parent'), worker=pin('/old-worker'), image=runtime['image']))
        s = dict(provenance={'recorded': 'V7'})
        value = {key: p[key] for key in I.REVIEW_BINDINGS}
        value.update(schema=I.REVIEW_SCHEMA, reviewed=True, authority='none', policy=p['policy'],
            image=runtime['image'], historical_runtime=prior['runtime'], image_provenance=s['provenance'],
            gpu_attempts=1, notes='Root records the exact scoped engineering assumptions.',
            review_topics={key: 'Root reviews the actual selected evidence for this topic.' for key in I.TOPICS})
        value.update({key: False for key in I.REVIEW_FALSE})
        I.engineering_review(value, p, runtime, prior, s)
        for key, changed in [('reviewed', False), ('gpu_attempts', 2),
                             *[(key, True) for key in I.REVIEW_FALSE]]:
            with self.assertRaises(RuntimeError): I.engineering_review(dict(value, **{key: changed}), p, runtime, prior, s)


if __name__ == '__main__':
    unittest.main()
