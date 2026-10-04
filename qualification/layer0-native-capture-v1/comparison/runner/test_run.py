"""Synthetic adapter policy tests; no GPU, process, or numerical execution."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import run as M


class Memory:
    def __init__(self):
        self.bodies = {}

    def add(self, path, value, raw=False):
        body = value if raw else json.dumps(value, sort_keys=True).encode()
        pin = dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
        self.bodies[str(path)] = body
        return pin

    def __call__(self, pin):
        body = self.bodies[pin['path']]
        M.require(len(body) == pin['bytes'] and hashlib.sha256(body).hexdigest() == pin['sha256'], 'synthetic pin')
        return body

    def doc(self, pin):
        return M.parse(self(pin))


def leaf(reader, directory, name, parent_binary, request, environment):
    gpu = name == 'parent'
    pid = 100 + M.LEAVES.index(name)
    identity = dict(pid=pid, pgid=pid, sid=pid, uid=9661, ppid=90, starttime=1000 + pid)
    where = directory / name
    command = reader.add(where / 'command.json', dict(
        argv=([parent_binary['path'], '--request', request['path'], '--capture-layer-zero',
            '--allow-unauthenticated-machine-code'] if gpu else M.SMI_ARGV),
        env=environment, cwd=str(M.R), deadline_seconds=4000 if gpu else 30,
        affinity=[8, 9], nice=10, address_space_bytes=(32 if gpu else 12) << 30,
        file_cap_bytes=64 << 20, stream_cap_bytes=8 << 20, gpu_execution_requested=gpu))
    started = reader.add(where / 'started.json', dict(parent=identity, supervisor_pid=90,
        command_sha256=command['sha256']))
    stdout = reader.add(where / 'stdout', b'' if gpu else json.dumps([
        dict(gpu=rank, process_list=[dict(process_info='No running processes detected')]) for rank in range(2)]).encode(), True)
    stderr = reader.add(where / 'stderr', b'', True)
    value = dict(command=command, started=started, stdout=stdout, stderr=stderr, exit_code=0,
        reason=None, cleanup_signalled=False, owned_groups_absent=True, owned_processes_reaped=True,
        lineage=[dict(event='owned', identity=identity)], gpu_execution_requested=gpu)
    result = reader.add(where / 'result.json', value)
    return dict(result=result, retained_files={'command.json': command, 'started.json': started,
        'stdout': stdout, 'stderr': stderr, 'result.json': result})


def fixture():
    reader = Memory()
    directory = M.E / 'prefix-layer0-native-capture-gpu-v228-v1'
    parent = dict(path=str(M.E / 'parent'), bytes=13480088,
        sha256='e7fb0571cfb8114ef86987525a6b882c4fdc84329022db5eabfdb131f3e3e12d')
    worker = dict(path=str(M.E / 'worker'), bytes=5018856,
        sha256='d2af909e7fef88af5b20f4ae8f2a5779a7aee93a187169e97a939bec7d3a0fed')
    request = reader.add(M.E / 'input/request.json', dict(schema='synthetic-request'))
    env = dict(CUDA_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='')
    topology_identity = [dict(unique_id=uid, device_path='/sys/devices/' + bdf)
        for uid, bdf in zip(M.DEVICES, M.SMI_ARGV[3:5])]
    platform = reader.add(M.E / 'platform.json', dict(schema='ferric-p227-prefix-parity-platform-review-v1',
        reviewed=True, authority='none', devices=M.DEVICES, production_authority=False,
        runtime_premises_discharged=False, host='mi350-history', boot_id='old-boot',
        topology_identity=topology_identity))
    plan = dict(schema='ferric-p228-layer0-native-capture-inputs-v1', output_label=directory.name,
        parent=parent, worker=worker, request=request, baseline=M.BASELINE,
        parent_cpu=dict(path=str(M.E / 'parent-cpu.json'), bytes=326552,
            sha256='78c12f822e95d50a1239f56411c5b8da9411a3fbda7510fbbf38d5644e1dffbb'),
        worker_cpu=dict(path=str(M.E / 'worker-cpu.json'), bytes=213924,
            sha256='41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d'))
    for role in ('parent', 'worker'):
        plan[role + '_runtime_review'] = reader.add(M.E / (role + '-review.json'), dict(
            reviewed=True, authority='none', binary=plan[role], host='mi350-history', boot_id='old-boot',
            production_authority=False))
    plan['capture_review'] = reader.add(M.E / 'capture-review.json', dict(reviewed=True, authority='none',
        request=request, baseline=M.BASELINE, production_authority=False, numerical_acceptance=False))
    outer = dict(schema='ferric-p228-layer0-native-capture-gpu-v1', passed=True, failures=[],
        native_attempts=1, retries=0, gpu_execution_requested=True, current_tf4_hidden_equal=True, captured_arrays=28,
        controller=dict(path=str(M.GPU / 'run.py'), bytes=1, sha256=M.GPU_RUN_SHA),
        supervisor_manifest=dict(path=str(M.GPU / 'manifest.json'), bytes=1, sha256=M.GPU_MANIFEST_SHA),
        plan=reader.add(M.E / 'input/plan.json', plan), baseline=M.BASELINE,
        parent=parent, worker=worker, request=request, parent_cpu_complete=plan['parent_cpu'],
        worker_cpu_complete=plan['worker_cpu'], input_pins={platform['path']: platform}, standalone_input_pins={},
        **{key: False for key in M.FALSE})
    for key in ('parent_runtime_review', 'worker_runtime_review', 'capture_review'):
        outer[key] = plan[key]
    outer['leaves'] = {name: leaf(reader, directory, name, parent, request, env) for name in M.LEAVES}
    for side in ('before', 'after'):
        outer[side + '_audits'] = []
        for n in range(3):
            name = side + '-' + str(n)
            sample = dict(devices=[dict(d, gpu_busy=0, memory_busy=0, vram_used=4096) for d in topology_identity])
            sample_pin = reader.add(directory / (name + '-topology.json'), sample)
            outer[side + '_audits'].append(dict(topology=sample_pin, process_result=outer['leaves'][name]['result']))
    return reader, directory, outer, platform, env


def replace_leaf(reader, outer, name, mutate, component=None):
    entry = outer['leaves'][name]
    value = reader.doc(entry['result'])
    if component:
        target = reader.doc(value[component]); mutate(target)
        record = reader.add(value[component]['path'], target)
        value[component] = record
        entry['retained_files'][{'command': 'command.json', 'started': 'started.json'}[component]] = record
    else:
        mutate(value)
    entry['result'] = reader.add(entry['result']['path'], value)
    entry['retained_files']['result.json'] = entry['result']


class AdapterTests(unittest.TestCase):
    def test_reader_retains_original_transport_identity_and_empty_body(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(M, 'E', Path(temp)):
            path = Path(temp) / 'empty'; path.write_bytes(b'')
            retained, _ = M.actual(path)
            original = dict(retained, path='/original/empty')
            reader = M.Reader({original['path']: retained})
            self.assertEqual(reader(original), b'')
            self.assertEqual(reader.consumed[original['path']], dict(original=original, retained=retained))
            reader.recheck()

    def test_reader_refuses_changed_bytes_extent_and_postread_drift(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(M, 'E', Path(temp)):
            path = Path(temp) / 'body'; path.write_bytes(b'a')
            pin, _ = M.actual(path); reader = M.Reader({})
            with self.assertRaises(ValueError): reader(dict(pin, bytes=2))
            reader(pin); path.write_bytes(b'b')
            with self.assertRaises(ValueError): reader.recheck()
            with self.assertRaises(ValueError): reader(pin)

    def test_reader_refuses_symlink_and_changed_transport_hash(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(M, 'E', Path(temp)):
            path = Path(temp) / 'body'; path.write_bytes(b'a')
            link = Path(temp) / 'link'; link.symlink_to(path)
            with self.assertRaises(ValueError): M.actual(link)
            pin, _ = M.actual(path)
            reader = M.Reader({'/original': pin})
            with self.assertRaises(ValueError): reader(dict(pin, path='/original', sha256='0' * 64))

    def test_duplicate_nonfinite_json_and_invalid_filepins_refuse(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError): M.parse(raw)
        for row in (dict(path='/a/../b', bytes=0, sha256='0' * 64),
                    dict(path='/a', bytes=True, sha256='0' * 64),
                    dict(path='/a', bytes=0, sha256='X' * 64)):
            with self.assertRaises(ValueError): M.filepin(row)

    def test_module_alias_restores_absent_object_and_none_on_failure(self):
        reader = Memory(); key = '_comparison_test_alias'
        absent = object(); prior = sys.modules.get(key, absent)
        try:
            for old in (absent, None, SimpleNamespace()):
                if old is absent: sys.modules.pop(key, None)
                else: sys.modules[key] = old
                for fail in (False, True):
                    pin = reader.add('/module.py', b'raise ValueError("fixture")' if fail else b'value=1', True)
                    if fail:
                        with self.assertRaises(ValueError): M.module(reader, pin, 'fixture', {key: SimpleNamespace()})
                    else:
                        self.assertEqual(M.module(reader, pin, 'fixture', {key: SimpleNamespace()}).value, 1)
                    self.assertIs(sys.modules.get(key, absent), old)
        finally:
            if prior is absent: sys.modules.pop(key, None)
            else: sys.modules[key] = prior

    def test_all_seven_authentic_shaped_leaf_records_replay(self):
        reader, directory, outer, _, env = fixture()
        values = [M.owned_leaf(reader, outer, directory, name, env) for name in M.LEAVES]
        self.assertEqual(len(values), 7)
        self.assertEqual(sum(value['gpu_execution_requested'] for value, _ in values), 1)

    def test_owned_result_cleanup_exit_and_reap_refusals(self):
        for key, wrong in (('exit_code', 1), ('exit_code', False), ('reason', 'timeout'),
                           ('cleanup_signalled', True), ('owned_groups_absent', False),
                           ('owned_processes_reaped', False)):
            reader, directory, outer, _, env = fixture()
            replace_leaf(reader, outer, 'parent', lambda value: value.update({key: wrong}))
            with self.assertRaises(ValueError): M.owned_leaf(reader, outer, directory, 'parent', env)

    def test_leaf_roster_paths_stream_and_result_substitution_refuse(self):
        for mutation in ('missing', 'path', 'stream', 'result'):
            reader, directory, outer, _, env = fixture(); entry = outer['leaves']['parent']
            if mutation == 'missing': del entry['retained_files']['stderr']
            elif mutation == 'path': entry['retained_files']['stderr']['path'] += '-other'
            elif mutation == 'stream': reader.bodies[entry['retained_files']['stdout']['path']] = b'changed'
            else: entry['result'] = outer['leaves']['before-0']['result']
            with self.assertRaises((ValueError, KeyError)):
                M.owned_leaf(reader, outer, directory, 'parent', env)

    def test_original_selector_limits_and_gpu_class_are_closed(self):
        for key, value in (('argv', ['/other']), ('deadline_seconds', 4001), ('affinity', [0, 1]),
                           ('nice', 0), ('address_space_bytes', 64 << 30), ('file_cap_bytes', 128 << 20)):
            reader, directory, outer, _, env = fixture()
            replace_leaf(reader, outer, 'parent', lambda item: item.update({key: value}), 'command')
            with self.assertRaises(ValueError): M.owned_leaf(reader, outer, directory, 'parent', env)

    def test_started_command_and_owned_parent_identity_are_bound(self):
        for mutation in ('command', 'owner', 'uid'):
            reader, directory, outer, _, env = fixture()
            def change(item):
                if mutation == 'command': item['command_sha256'] = '0' * 64
                elif mutation == 'owner': item['supervisor_pid'] += 1
                else: item['parent']['uid'] = 0
            replace_leaf(reader, outer, 'parent', change, 'started')
            with self.assertRaises(ValueError): M.owned_leaf(reader, outer, directory, 'parent', env)

    def test_six_audits_call_original_data_only_helpers_and_join_leaves(self):
        reader, directory, outer, platform, env = fixture()
        leaves = {name: M.owned_leaf(reader, outer, directory, name, env) for name in M.LEAVES}
        audit = SimpleNamespace(empty_processes=Mock()); topology = SimpleNamespace(require_idle=Mock())
        M.audit_samples(reader, outer, directory, leaves, reader.doc(platform), audit, topology)
        self.assertEqual(audit.empty_processes.call_count, 6)
        self.assertEqual(topology.require_idle.call_count, 2)
        outer['after_audits'][0]['process_result'] = outer['leaves']['before-0']['result']
        with self.assertRaises(ValueError):
            M.audit_samples(reader, outer, directory, leaves, reader.doc(platform), audit, topology)

    def test_wrong_topology_busy_and_original_audit_rejection_propagate(self):
        for mutation in ('busy', 'device', 'process'):
            reader, directory, outer, platform, env = fixture()
            leaves = {name: M.owned_leaf(reader, outer, directory, name, env) for name in M.LEAVES}
            audit = SimpleNamespace(empty_processes=Mock()); topology = SimpleNamespace(require_idle=Mock())
            if mutation == 'process': audit.empty_processes.side_effect = RuntimeError('original parser refused')
            else:
                row = outer['before_audits'][0]; sample = reader.doc(row['topology'])
                sample['devices'][0]['gpu_busy' if mutation == 'busy' else 'unique_id'] = 1
                row['topology'] = reader.add(row['topology']['path'], sample)
            with self.assertRaises((ValueError, RuntimeError)):
                M.audit_samples(reader, outer, directory, leaves, reader.doc(platform), audit, topology)

    def test_pure_receipt_requires_actual_count_source_snapshots_and_transcript(self):
        reader = Memory(); sources = {'run.py': reader.add('/run.py', b'pass', True)}
        before = reader.add('/before', sources); after = reader.add('/after', sources)
        value = dict(schema='pure', passed=True, tests=14, errors=0, failures=0, skipped=0,
            source_postchecks_passed=True, sources_before=before, sources_after=after,
            transcript=reader.add('/tests.log', b'actual fixture only', True), controller=sources['run.py'])
        pin = reader.add('/pure', value); M.pure(reader, pin, 'pure', 14, sources)
        for key, wrong in (('tests', 13), ('passed', False), ('skipped', 1), ('source_postchecks_passed', False)):
            mutated = dict(value, **{key: wrong})
            with self.assertRaises(ValueError): M.pure(reader, reader.add('/pure', mutated), 'pure', 14, sources)
        pin = reader.add('/pure', value)
        with self.assertRaises(ValueError): M.pure(reader, pin, 'pure', 14, {})

    def test_full_native_adapter_routes_retained_bytes_through_capture_validator(self):
        baseline_reader = Memory()
        capture = baseline_reader.add(M.E / 'baseline/observation.bin', bytes(8192), True)
        baseline = baseline_reader.add(M.E / 'baseline/complete.json',
            dict(retained_native={'observation-0.bin': capture}))
        source = b'synthetic source placeholder'
        digest = hashlib.sha256(source).hexdigest()
        with patch.object(M, 'BASELINE', baseline), patch.object(M, 'GPU_RUN_SHA', digest), \
                patch.object(M, 'GPU_MANIFEST_SHA', digest):
            reader, directory, outer, platform, _ = fixture()
            reader.bodies.update(baseline_reader.bodies)
            for key in ('controller', 'supervisor_manifest'):
                outer[key] = reader.add(outer[key]['path'], source, True)
            names = ('request.json', 'candidate-registration.json', 'candidate-program.json',
                'candidate-uploads.json', 'candidate-bootstrap.json', 'candidate-request-1.json',
                'candidate-response-1.json', 'candidate-request-2.json', 'candidate-response-2.json',
                'candidate-capture.bin', 'candidate-stderr.bin')
            records = {name: reader.add(directory / 'native' / name, b'', True) for name in names}
            summary = b'{"synthetic":true}'
            records['summary.json'] = reader.add(directory / 'native/summary.json', summary, True)
            outer['retained_native'] = records
            checked = dict(closed_child_pids=[199], synthetic=True)
            outer['checked'] = checked
            outer['observation'] = reader.add(directory / 'observation.json', checked)
            entry = outer['leaves']['parent']
            parent = reader.doc(entry['result'])
            parent['stdout'] = reader.add(parent['stdout']['path'], summary + b'\n', True)
            entry['retained_files']['stdout'] = parent['stdout']
            entry['result'] = reader.add(entry['result']['path'], parent)
            entry['retained_files']['result.json'] = entry['result']
            root = reader.doc(parent['started'])['parent']
            outer['owned_children'] = dict(parent=root, workers=[dict(pid=199, identity=None,
                outer_pidfd_observed=False)], parent_asserted_close_and_reap=True,
                outer_groups_absent=True, synthesized_child_identity=False)
            pin = reader.add(directory / 'complete.json', outer)
            CV = SimpleNamespace(BODY=set(names), validate=Mock(return_value=checked), child_marker=Mock())
            audit = SimpleNamespace(empty_processes=Mock()); topology = SimpleNamespace(require_idle=Mock())
            observed, observed_summary = M.native(reader, pin, platform, CV, audit, topology)
            self.assertEqual(observed, outer)
            self.assertEqual(observed_summary, records['summary.json'])
            CV.validate.assert_called_once_with(summary, {name: b'' for name in names},
                reader.doc(outer['request']), bytes(8192))
            CV.child_marker.assert_called_once_with(b'', 199)
            self.assertFalse(observed['owned_children']['workers'][0]['outer_pidfd_observed'])
            outer['checked'] = dict(checked, synthetic=False)
            with self.assertRaises(ValueError):
                M.native(reader, reader.add(pin['path'], outer), platform, CV, audit, topology)

    def test_outer_requires_success_no_retries_all_seven_and_diagnostic_scope(self):
        for key, wrong in (('passed', False), ('failures', ['failed']), ('native_attempts', 2),
                           ('retries', 1), ('captured_arrays', 27), ('numerical_acceptance', True),
                           ('performance_claim', True), ('leaves', {})):
            reader, directory, outer, platform, _ = fixture()
            outer[key] = wrong; pin = reader.add(directory / 'complete.json', outer)
            with self.assertRaises(ValueError): M.native(reader, pin, platform, None, None, None)


if __name__ == '__main__':
    unittest.main(verbosity=2)
