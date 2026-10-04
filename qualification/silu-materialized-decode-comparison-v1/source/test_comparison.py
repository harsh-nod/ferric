"""Synthetic payload and recorded-custody tests; no subprocess/model/GPU use."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import sys
import types
import unittest
from unittest.mock import patch

import comparison as A

ROOT = Path(__file__).resolve().parent.parent
DEPENDENCIES = {
    'retained_compare': ('p227-prefix-decode-gpu-qualification-v2/compare.py',
        '8154580de7f5ad40fd4897ce264fc3d92c9a109808ea7ea5dab6d0486f01e622'),
    'diagnostics': ('p225-tiles-tf4-framework-comparison/helpers/diagnostics.py',
        '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf'),
    'stage_core': ('p228-silu-materialized-decode-gpu-v1/stage_core.py',
        '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2'),
    'smoke_validation': ('p228-silu-materialized-decode-gpu-v1/smoke_validation.py',
        '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26'),
    'decode_validation': ('p228-silu-materialized-decode-gpu-v1/decode_validation.py',
        'b4e4cee33a9c82dd58189ca359e5a64d7e1dd1780e974e215c85c40e0df479e0'),
    'tf4_fixture': ('p228-silu-materialized-decode-gpu-v1/test_decode_validation.py',
        '37a24160ea948a8d19e9f5abac34d5407c239be7311c3e72efbe9074a026ff82'),
}


def modules():
    absent = object(); old = {name: sys.modules.get(name, absent) for name in DEPENDENCIES}
    result = {}
    try:
        for name, (relative, digest) in DEPENDENCIES.items():
            path = ROOT / relative; raw = path.read_bytes()
            if hashlib.sha256(raw).hexdigest() != digest:
                raise ValueError('exact test dependency: ' + relative)
            module = types.ModuleType(name); module.__file__ = str(path)
            sys.modules[name] = module
            exec(compile(raw, str(path), 'exec'), module.__dict__); result[name] = module
    finally:
        for name, value in old.items():
            if value is absent: sys.modules.pop(name, None)
            else: sys.modules[name] = value
    return result


H = modules()
C, D, V, F = (H[name] for name in ('retained_compare', 'diagnostics', 'decode_validation', 'tf4_fixture'))


def payloads(winners=(7, 8, 9, 10)):
    records, rows = [], []
    for position, winner in enumerate(winners):
        raw = bytearray(b'\x80\x3f' * (D.PAYLOAD_BYTES // 2))
        struct.pack_into('<H', raw, 37 * 8192 + 2 * winner, 0x4000)
        records.append(dict(generation=position + 1, position=position,
            input_token=A.TOKENS[position], output_token=winner))
        rows.append(bytes(raw))
    return records, rows


class Memory:
    def __init__(self): self.bodies = {}
    def put(self, path, value):
        raw = value if type(value) is bytes else (json.dumps(value, sort_keys=True) + '\n').encode()
        self.bodies[path] = raw
        return dict(path=path, bytes=len(raw), sha256=C.sha(raw))
    def __call__(self, pin, maximum):
        raw = self.bodies[pin['path']]
        if len(raw) != pin['bytes'] or len(raw) > maximum or C.sha(raw) != pin['sha256']:
            raise ValueError('synthetic exact pin')
        return raw


def native_fixture():
    memory = Memory(); observed, files = F.fixture()
    request = observed['request']; request['decode']['evidence_directory'] = '/task/native'
    for frame in observed['files']['frames']:
        for key in ('request', 'control', 'observation'):
            frame[key]['path'] = frame[key]['path'].replace('/task/decode/', '/task/native/')
    observed['files']['child_stderr']['path'] = '/task/native/child-stderr.bin'
    summary = F.serialize(observed)
    checked = V.validate(summary, files, request)
    records = {name: memory.put('/task/native/' + name, raw) for name, raw in files.items()}
    records['complete.json'] = memory.put('/task/native/complete.json', summary)
    image = C.pin(request['projection_residual_image']); worker = C.pin(request['decode']['worker'])
    parent = dict(path='/artifacts/parent', bytes=1, sha256='ab' * 32)
    outer = dict(schema=A.GPU_SCHEMA, passed=True, failures=[], native_attempts=1, retries=0,
        full_forward=True, gpu_execution_requested=True, old_native_equality_required=False,
        **{name: False for name in A.FALSE}, parent=parent, worker=worker, projection_image=image,
        controller=memory.put('/package/run.py', b'synthetic-controller'),
        supervisor_manifest=memory.put('/package/manifest.json', b'{}'),
        request=memory.put('/inputs/request.json', request), retained_native=records,
        checked=checked, observation=memory.put('/task/observation.json', checked),
        captured_tensor_rows=152, captured_payloads=4, leaves={}, before_audits=[], after_audits=[])
    for key in ('lowering_complete', 'inspection_complete', 'parent_runtime_review',
                'worker_runtime_review', 'decode_review'):
        outer[key] = memory.put('/inputs/' + key + '.json', {})
    plan = {name: outer[name] for name in ('request', 'parent', 'worker', 'projection_image',
        'lowering_complete', 'inspection_complete', 'parent_runtime_review', 'worker_runtime_review', 'decode_review')}
    plan['output_label'] = 'task'; outer['plan'] = memory.put('/inputs/plan.json', plan)
    pid = dict(pid=120, pgid=120, sid=120, uid=9661, ppid=100, starttime=20)
    for name in sorted(A.LEAVES):
        gpu = name == 'parent'
        argv = ([parent['path'], '--request', outer['request']['path'],
            '--observe-projection-residual-decode', '--allow-unauthenticated-machine-code'] if gpu else ['/smi'])
        command = memory.put('/task/' + name + '/command.json', dict(argv=argv, deadline_seconds=4000,
            affinity=[8, 9], nice=10, address_space_bytes=32 << 30, file_cap_bytes=64 << 20,
            stream_cap_bytes=8 << 20, gpu_execution_requested=gpu))
        start = memory.put('/task/' + name + '/started.json',
            dict(parent=pid, supervisor_pid=100, command_sha256=command['sha256']))
        stderr = (b'finite engineering owned child pid=17 pgid=17; no native setup acknowledged\n'
            b'finite explicit profile=projection-residual-prefix284-mlp548-four-forward-v1 mode=TeacherForced\n'
            + b''.join(f'finite prefix decode completed position={n} forwards={n + 1}\n'.encode()
                       for n in range(4))) if gpu else b''
        value = dict(exit_code=0, reason=None, cleanup_signalled=False, owned_groups_absent=True,
            owned_processes_reaped=True, owned_groups=[120], lineage=[dict(event='owned', identity=pid)],
            command=command, started=start, stdout=memory.put('/task/' + name + '/stdout', summary if gpu else b'{}'),
            stderr=memory.put('/task/' + name + '/stderr', stderr), gpu_execution_requested=gpu)
        result = memory.put('/task/' + name + '/result.json', value)
        outer['leaves'][name] = dict(result=result, retained_files=dict(
            **{'command.json': command, 'started.json': start, 'result.json': result},
            stdout=value['stdout'], stderr=value['stderr']))
        if not gpu:
            outer[name.split('-')[0] + '_audits'].append(dict(process_result=result,
                topology=memory.put('/task/' + name + '-topology.json', {})))
    for key in ('before_audits', 'after_audits'):
        outer[key].sort(key=lambda row: row['topology']['path'])
    return memory, outer


def provenance_fixture():
    memory = Memory()
    image = dict(path='/images/silu', bytes=A.SILU_IMAGE[0], sha256=A.SILU_IMAGE[1])
    projection = dict(path='/images/residual', bytes=10864, sha256='25' * 32)
    outer = dict(mlp_image=image, projection_image=projection,
        mlp_cpu=memory.put('/source/cpu.json', {}), mlp_lowering_complete=memory.put('/source/lower.json', {}),
        mlp_lowering_owner=memory.put('/source/owner.json', {}))
    summary = memory.put('/layer/native/summary.json', {})
    layer = dict(schema='ferric-p228-silu-materialized-capture-gpu-v1',
        passed=True, failures=[], retained_native={'summary.json': summary},
        **outer, **{key: False for key in ('numerical_acceptance', 'full_model_correctness',
            'performance_claim', 'production_authority')})
    outer['baseline'] = memory.put('/layer/complete.json', layer)
    inputs = dict(candidate_capture=summary, selected_mlp_image=image, projection_image=projection,
        original_framework=dict(path='/framework/reference.json',
            bytes=A.FRAMEWORK_REPORT[0], sha256=A.FRAMEWORK_REPORT[1]))
    math = dict(schema='ferric-p228-silu-materialized-comparison-observation-v1',
        completed=True, source_postchecks_passed=True, native_outer=outer['baseline'],
        comparison={'inputs': inputs}, **{key: False for key in ('numerical_acceptance',
            'full_model_correctness', 'performance_claim', 'production_authority')})
    outer['layer_comparison'] = memory.put('/math/complete.json', math)
    plan = copy.deepcopy(outer)
    request = dict(decode={'tiles_image': image})
    return memory, outer, plan, request


def baseline_fixture():
    memory = Memory(); rows = payloads()
    comparison = A.compare_payloads(*rows, *rows, C, D)
    comparison['framework_complete'] = memory.put('/framework/complete.json', {})
    value = dict(schema='ferric-p228-projection-residual-decode-framework-comparison-observation-v1',
        completed=True, errors=[], source_postchecks_passed=True,
        framework_outer=comparison['framework_complete'], comparison=copy.deepcopy(comparison),
        **{key: False for key in ('gpu_execution', 'numerical_acceptance', 'full_model_correctness',
            'full_model_acceptance', 'performance_claim', 'production_authority')})
    return memory, value, comparison


class ComparisonTests(unittest.TestCase):
    def compare(self, left, right): return A.compare_payloads(*left, *right, C, D)
    def native(self, memory, outer):
        record = memory.put('/task/complete.json', outer)
        with patch.object(A, 'GPU_CONTROLLER_SHA', outer['controller']['sha256']), \
             patch.object(A, 'GPU_MANIFEST_SHA', outer['supervisor_manifest']['sha256']), \
             patch.object(A, 'silu_provenance', return_value={}):
            return A.candidate(memory, record, C, V)

    def test_all152_direct_rows_and_four_token_records(self):
        cases = payloads(); result = self.compare(cases, cases)
        self.assertEqual((result['tensor_rows'], result['exact_tensor_rows']), (152, 152))
        self.assertTrue(result['output_tokens_equal'])
        self.assertEqual(len(result['layer_error_trajectories']), 4)
        self.assertTrue(all(row['first_nonexact_hidden_layer'] is None for row in result['layer_error_trajectories']))

    def test_last_hidden_norm_and_logits_keep_exact_differences(self):
        original = payloads(); changed = copy.deepcopy(original)
        raw = bytearray(changed[1][3])
        for offset in (35 * 8192, 36 * 8192, 37 * 8192): struct.pack_into('<H', raw, offset, 0x3f81)
        changed[1][3] = bytes(raw)
        result = self.compare(original, changed)
        self.assertEqual(result['exact_tensor_rows'], 149)
        self.assertEqual(result['layer_error_trajectories'][3]['first_nonexact_hidden_layer'], 35)
        for row in result['comparisons'][3]['tensors'][-3:]:
            self.assertEqual(row['max_abs_error'], 1 / 128)
            self.assertEqual(row['exact_words'], row['elements'] - 1)

    def test_output_token_mismatch_is_diagnostic_not_rejection(self):
        result = self.compare(payloads(), payloads((11, 12, 13, 14)))
        self.assertFalse(result['output_tokens_equal']); self.assertEqual(result['tensor_rows'], 152)
        self.assertEqual(result['candidate_output_tokens'], [11, 12, 13, 14])

    def test_changed_tf_input_is_not_same_workload(self):
        left = payloads(); right = copy.deepcopy(left); right[0][2]['input_token'] = 7
        with self.assertRaises(ValueError): self.compare(left, right)

    def test_nonfinite_tensor_refuses(self):
        left = payloads(); right = copy.deepcopy(left); raw = bytearray(right[1][0])
        struct.pack_into('<H', raw, 8192, 0x7fc0); right[1][0] = bytes(raw)
        with self.assertRaises(ValueError): self.compare(left, right)

    def test_layer_error_can_shrink_without_causal_claim(self):
        left = payloads(); right = copy.deepcopy(left); raw = bytearray(right[1][0])
        struct.pack_into('<H', raw, 0, 0x4000); right[1][0] = bytes(raw)
        row = self.compare(left, right)['layer_error_trajectories'][0]
        self.assertEqual(row['layers'][1]['max_abs_change_from_previous_layer'], -1)
        self.assertFalse(row['causal_attribution_proven']); self.assertFalse(row['monotonic_growth_required'])

    def test_signed_zero_is_nonexact_with_zero_absolute_error(self):
        left = payloads(); right = copy.deepcopy(left)
        for cases, bits in ((left, 0), (right, 0x8000)):
            raw = bytearray(cases[1][0]); struct.pack_into('<H', raw, 0, bits); cases[1][0] = bytes(raw)
        row = self.compare(left, right)['comparisons'][0]['tensors'][0]
        self.assertFalse(row['byte_equal']); self.assertEqual(row['max_abs_error'], 0)

    def test_truncated_payload_and_wrong_own_argmax_refuse(self):
        left = payloads()
        for mode in ('truncated', 'argmax'):
            right = copy.deepcopy(left)
            if mode == 'truncated': right[1][0] = right[1][0][:-2]
            else: right[0][0]['output_token'] = 0
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.compare(left, right)

    def test_candidate_nested_schema_real_validator_and_seven_leaves(self):
        memory, outer = native_fixture(); _, observed, files, checked, owner = self.native(memory, outer)
        self.assertEqual(observed['request']['schema'], 'FerricFiniteProjectionResidualDecodeRequestV1')
        self.assertEqual((len(files), checked['captured_tensor_rows']), (13, 152))
        self.assertTrue(owner['parent_asserted_close_and_reap']); self.assertIsNone(owner['child'])

    def test_failed_or_authority_bearing_gpu_receipt_refuses(self):
        for key, value in (('passed', False), ('native_attempts', True), ('numerical_acceptance', True),
                           ('paired_comparison_performed', True), ('retries', 1)):
            memory, outer = native_fixture(); outer[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): self.native(memory, outer)

    def test_capture_digest_structural_result_and_request_cannot_drift(self):
        for mode in ('body', 'checked', 'path'):
            memory, outer = native_fixture()
            if mode == 'body': memory.bodies['/task/native/observation-3.bin'] += b'x'
            elif mode == 'checked': outer['checked']['input_tokens'][0] = 7
            else: outer['retained_native']['control-0.bin']['path'] = '/other/control-0.bin'
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.native(memory, outer)

    def test_failed_owner_wrong_selector_and_missing_audit_refuse(self):
        for mode in ('owner', 'selector', 'audit'):
            memory, outer = native_fixture()
            if mode == 'audit': outer['before_audits'].pop()
            else:
                item = outer['leaves']['parent']; value = C.doc(memory, item['result'])
                if mode == 'owner': value['cleanup_signalled'] = True
                else:
                    command = C.doc(memory, value['command']); command['argv'][3] = '--old-mode'
                    value['command'] = memory.put(value['command']['path'], command)
                    item['retained_files']['command.json'] = value['command']
                    start = C.doc(memory, value['started']); start['command_sha256'] = value['command']['sha256']
                    value['started'] = memory.put(value['started']['path'], start)
                    item['retained_files']['started.json'] = value['started']
                item['result'] = memory.put(item['result']['path'], value)
                item['retained_files']['result.json'] = item['result']
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.native(memory, outer)


    def provenance(self, memory, outer, plan, request):
        with patch.object(A, 'SILU_LAYER', A.content(C, outer['baseline'])), \
             patch.object(A, 'LAYER_COMPARISON', A.content(C, outer['layer_comparison'])):
            return A.silu_provenance(memory, outer, plan, request, C)

    def baseline(self, memory, value, comparison):
        record = memory.put('/baseline/complete.json', value)
        with patch.object(A, 'BASELINE_COMPARISON', A.content(C, record)):
            return A.baseline_metrics(memory, record, comparison, C)

    def test_silu_predecessors_join_without_equating_workers(self):
        memory, outer, plan, request = provenance_fixture()
        outer['worker'] = dict(path='/decode/worker', bytes=1, sha256='01' * 32)
        result = self.provenance(memory, outer, plan, request)
        self.assertEqual(result['mlp_image'], outer['mlp_image'])
        self.assertEqual(result['baseline'], outer['baseline'])

    def test_silu_image_or_predecessor_binding_drift_refuses(self):
        for mode in ('image', 'request', 'plan', 'framework', 'capture'):
            memory, outer, plan, request = provenance_fixture()
            if mode == 'image':
                outer['mlp_image']['sha256'] = '00' * 32
                plan['mlp_image'] = copy.deepcopy(outer['mlp_image'])
            elif mode == 'request': request['decode']['tiles_image'] = outer['projection_image']
            elif mode == 'plan': plan['mlp_cpu']['sha256'] = '00' * 32
            else:
                value = C.doc(memory, outer['layer_comparison'])
                field = 'original_framework' if mode == 'framework' else 'candidate_capture'
                value['comparison']['inputs'][field]['sha256'] = '00' * 32
                outer['layer_comparison'] = memory.put('/math/complete.json', value)
                plan['layer_comparison'] = outer['layer_comparison']
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                self.provenance(memory, outer, plan, request)

    def test_silu_predecessor_failure_or_authority_refuses(self):
        for key, changed in (('completed', False), ('source_postchecks_passed', False),
                             ('numerical_acceptance', True)):
            memory, outer, plan, request = provenance_fixture()
            value = C.doc(memory, outer['layer_comparison']); value[key] = changed
            outer['layer_comparison'] = memory.put('/math/complete.json', value)
            plan['layer_comparison'] = outer['layer_comparison']
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.provenance(memory, outer, plan, request)

    def test_recorded_baseline_requires_all_reference_identities(self):
        for mode in ('same', 'last-tensor', 'framework', 'input'):
            memory, value, comparison = baseline_fixture()
            if mode == 'last-tensor':
                value['comparison']['comparisons'][3]['tensors'][37]['reference_sha256'] = '00' * 32
            elif mode == 'framework': value['framework_outer'] = dict(path='/other', bytes=1, sha256='00' * 32)
            elif mode == 'input': value['comparison']['comparisons'][3]['candidate_input'] = 7
            if mode == 'same':
                result = self.baseline(memory, value, comparison)
                self.assertTrue(result['reference_tensor_identities_equal'])
                self.assertFalse(result['metrics_recomputed'])
                self.assertFalse(result['baseline_payloads_rehashed'])
            else:
                with self.subTest(mode=mode), self.assertRaises(ValueError):
                    self.baseline(memory, value, comparison)

    def test_recorded_baseline_token_difference_stays_diagnostic(self):
        memory, value, comparison = baseline_fixture()
        value['comparison']['comparisons'][0]['candidate_output'] = 100
        value['comparison']['comparisons'][0]['output_equal'] = False
        result = self.baseline(memory, value, comparison)
        self.assertEqual(result['comparisons'][0]['candidate_output'], 100)
        self.assertFalse(result['numerical_acceptance'])


if __name__ == '__main__': unittest.main()

