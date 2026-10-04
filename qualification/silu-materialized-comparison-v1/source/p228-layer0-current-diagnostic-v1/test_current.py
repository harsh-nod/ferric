"""Synthetic data custody and layout tests, never an executed native result."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

import current as M
import compare as C
import diagnostics as D


def raw(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def fixture():
    files = {}
    def save(name, body):
        p = dict(path='/synthetic/' + name, bytes=len(body), sha256=C.sha(body))
        files[p['path']] = body
        return p
    def wp(p):
        return dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    def digest(text):
        return list(bytes.fromhex(text))
    zero = b'\0' * 8192
    shapes = {}
    for name, shape in C.SHAPES.items():
        count = 1
        for dimension in shape: count *= dimension
        body = (b'\x80\x3f' if name == 'rotary-cos' else b'\0\0') * count
        shapes[name] = dict(dtype='bfloat16', shape=list(shape), pin=save('framework-' + name, body))
    source = {'config.json': dict(bytes=1, sha256='a' * 64, stat=[1])}
    framework = dict(schema='ferric-p228-layer0-framework-capture-v1', status='PASS', position=0, input_token=9112,
        model_id=C.MODEL, bundle_id=C.BUNDLE, genuine_framework_chain=True, conditional_replay_performed=False,
        candidate_intermediate_inputs=False, candidate_gpu_execution=False, repeat_passes_byte_equal=True,
        captured_stages_per_pass=33, numerical_acceptance=False, acceptance_threshold=None, model_sources=source,
        passes=[dict(ordinal=i, position=0, input_token=9112, fresh_cache=True, stages=copy.deepcopy(shapes)) for i in (1, 2)])
    payload = save('tf4-payload', b'\0' * D.PAYLOAD_BYTES)
    case = dict(record=dict(generation=1, position=0, input_token=9112, output_token=0), payload=payload,
                tensors={'layer0-hidden': dict(bytes=8192, sha256=C.sha(zero))})
    original = dict(schema='ferric-p224-rearm-four-framework-reference-v1', status='PASS', model_id=C.MODEL,
        bundle_id=C.BUNDLE, mode='teacher_forced', selected_input_tokens=C.TOKENS, model_sources=copy.deepcopy(source),
        repeat_passes_byte_equal=True, candidate_intermediate_inputs=False,
        passes=[dict(ordinal=i, fresh_cache=True, cases=[case, {}, {}, {}]) for i in (1, 2)])
    tokens = C.TOKENS + [0] * 2044
    prompt = dict(tokens=wp(save('tokens', struct.pack('<2048I', *tokens))), text=wp(save('text', b'synthetic')),
        manifest=wp(save('manifest', raw(dict(schema='FerricQwen3LongPromptV1', input_token_ids=tokens,
            input_tokens=2048, add_special_tokens=False, chat_template=None)))))
    framework['input_pins'] = [M.wire_pin(prompt[role]) for role in ('manifest', 'tokens')]
    dummy = wp(save('bootstrap-image', b'image'))
    decode = dict(source='/synthetic/model', worker=dummy, images={name: dummy for name in ('prefix', 'mlp', 'residual', 'tail')},
        expected_model_id=digest(C.MODEL), expected_bundle_id=digest(C.BUNDLE), device_ids=[7, 9], session=[1] * 32,
        dispatch_timeout_ms=10000, child_deadline_ms=3600000, mode='teacher_forced', prompt=prompt,
        prefix_image=dict(path='/synthetic/v7', bytes=53560, sha256=digest(C.V7)),
        tiles_image=dict(path='/synthetic/down2', bytes=33112, sha256=digest(C.DOWN2)))
    tf4_request = dict(schema='FerricFinitePrefixDecodeDeviceClockRequestV2', decode=decode)
    tf4 = dict(schema='ferric-p228-down2-clock-gpu-v1', passed=True, failures=[], native_attempts=1, retries=0,
        request=save('tf4-request', raw(tf4_request)), retained_native={'observation-0.bin': payload},
        checked=dict(all_payloads_tokens_and_tensors_equal=True, recorded_close_and_owner_reap_checked=True,
            structural=dict(mode='teacher_forced', input_tokens=C.TOKENS, positions=[0, 1, 2, 3],
                            logical_generations=[1, 2, 3, 4], output_tokens=[0] * 4)))
    request = {key: copy.deepcopy(decode[key]) for key in ('source', 'worker', 'images', 'expected_model_id',
        'expected_bundle_id', 'device_ids', 'dispatch_timeout_ms', 'child_deadline_ms', 'prompt')}
    request.update(schema='FerricFinitePrefixLayerCaptureRequestV1', session=[2] * 32,
        prefix_tiles_image=decode['prefix_image'], mlp_tiles_image=decode['tiles_image'], evidence_directory='/synthetic/native')
    bodies = {'request.json': raw(request), 'candidate-registration.json': b'{"synthetic":1}',
        'candidate-program.json': b'{"synthetic":2}', 'candidate-uploads.json': b'{"synthetic":3}',
        'candidate-capture.bin': b'\0' * M.CAPTURE_BYTES, 'candidate-stderr.bin': b''}
    def imagepart(p):
        return dict(bytes=p['bytes'], sha256=p['sha256'])
    pages = list(range(144)); pages[0], pages[7] = pages[7], pages[0]
    bootstrap = dict(protocol=1, profile='prefix284_mlp548', device_ids=[7, 9], timeout_ms=10000,
        begin=dict(scope=dict(child_identity=123, session=request['session'], model_id=request['expected_model_id'],
            bundle_id=request['expected_bundle_id'], pool_identity=1, group_id=0),
            registration=M.part(bodies[M.FILES[1]]), source_program=M.part(bodies[M.FILES[2]]), uploads=M.part(bodies[M.FILES[3]]),
            **{role + '_image': imagepart(request['images'][role]) for role in request['images']}),
        prefix_image=imagepart(request['prefix_tiles_image']), mlp_image=imagepart(request['mlp_tiles_image']),
        input=dict(generation=1, token=9112, cache_metadata=[0] + pages, rotary_bits=[0x3f800000] * 64 + [0] * 64))
    bodies['candidate-bootstrap.json'] = raw(bootstrap)
    responses = []
    for ordinal, command in ((1, 'run'), (2, 'close')):
        bodies[f'candidate-request-{ordinal}.json'] = raw(dict(protocol=1, id=ordinal, profile_sha256=[4] * 32, command=command))
        response = dict(protocol=1, id=ordinal, profile_sha256=[4] * 32, profile=bootstrap['profile'],
            completed_layers=1, native_closed=ordinal == 2, gpu_execution=True, numerical_acceptance=False,
            performance_claim=False, production_authority=False,
            capture=M.part(bodies['candidate-capture.bin']) if ordinal == 2 else None,
            control={'synthetic': True} if ordinal == 2 else None)
        bodies[f'candidate-response-{ordinal}.json'] = raw(response)
        responses.append(response)
    stages, offset = [], 0
    for rank in range(2):
        for name, count, width in M.STAGES:
            stages.append(dict(rank=rank, stage=name, offset=offset, bytes=count, elements=count // width,
                               element_bytes=width, sha256=digest(C.sha(b'\0' * count))))
            offset += count
    summary = dict(schema='FerricFinitePrefixLayerCaptureObservationV1', request=request,
        run=dict(child_pid=123, bootstrap=bootstrap, profile_sha256=[4] * 32, setup_commands=1,
                 close=responses[1], child_exit_zero=True, process_group_absent=True), stages=stages,
        files=[dict(name=name, bytes=len(bodies[name]), sha256=digest(C.sha(bodies[name]))) for name in M.FILES],
        native_attempts=1, retries=0, completed_layers=1, native_closed=True, gpu_execution=True,
        paired_comparison_performed=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False, full_forward=False)
    for name, body in bodies.items(): files['/synthetic/native/' + name] = body
    pins = [save('framework.json', raw(framework)), save('original.json', raw(original)),
            save('tf4.json', raw(tf4)), save('summary.json', raw(summary))]
    return dict(files=files, pins=pins, framework=framework, original=original, tf4=tf4,
        tf4_request=tf4_request, summary=summary, bodies=bodies, hidden=zero)


class CurrentDiagnostic(unittest.TestCase):
    def stage(self, f, page=7):
        with patch.object(M, 'CURRENT_HIDDEN_SHA', C.sha(f['hidden'])):
            return M.stage_bytes(f['summary'], f['bodies']['candidate-capture.bin'], page, f['hidden'], D)

    def test_reused_source_bytes_are_unchanged(self):
        self.assertEqual(hashlib.sha256(Path(C.__file__).read_bytes()).hexdigest(), M.COMPARE_SHA)
        self.assertEqual(hashlib.sha256(Path(D.__file__).read_bytes()).hexdigest(), C.DIAGNOSTICS_SHA)

    def test_full_data_path_has_twenty_four_rows_and_no_acceptance(self):
        f = fixture()
        with patch.object(C, 'EMBEDDING_SHA', C.sha(f['hidden'])), patch.object(M, 'CURRENT_HIDDEN_SHA', C.sha(f['hidden'])):
            result = M.compare_retained(*f['pins'], lambda p: f['files'][p['path']], D)
        self.assertEqual(result['comparable_rows'], 24)
        self.assertEqual(len(result['excluded_fp32_partials']), 4)
        self.assertEqual(result['physical_page'], 7)
        self.assertIsNone(result['earliest_observable_divergence'])
        for key in ('numerical_acceptance', 'receipt_authentication', 'production_authority', 'full_model_correctness',
                    'performance_measured', 'gpu_execution', 'ordering_is_a_causal_proof', 'fp32_partials_compared_to_full_bf16'):
            self.assertFalse(result[key], key)
        self.assertIsNone(result['acceptance_threshold'])

    def test_scoped_sessions_may_differ_but_source_and_prompt_may_not(self):
        f = fixture()
        self.assertNotEqual(f['summary']['request']['session'], f['tf4_request']['decode']['session'])
        M.current_request(f['summary'], f['tf4_request'])
        for field in ('source', 'device_ids', 'expected_model_id'):
            bad = copy.deepcopy(f['summary']); bad['request'][field] = 'changed'
            with self.assertRaises(ValueError): M.current_request(bad, f['tf4_request'])
        bad = copy.deepcopy(f['summary']); bad['request']['prompt']['tokens']['sha256'] = [0] * 32
        with self.assertRaises(ValueError): M.current_request(bad, f['tf4_request'])

    def test_explicit_and_bootstrap_images_cannot_be_interchanged(self):
        f = fixture()
        for name in ('prefix_tiles_image', 'mlp_tiles_image', 'worker'):
            bad = copy.deepcopy(f['summary']); bad['request'][name]['sha256'] = [0] * 32
            with self.assertRaises(ValueError): M.current_request(bad, f['tf4_request'])
        bad = copy.deepcopy(f['summary']); bad['request']['prefix_tiles_image'] = bad['request']['images']['prefix']
        with self.assertRaises(ValueError): M.current_request(bad, f['tf4_request'])

    def test_native_file_roster_and_actual_capture_hash_are_checked(self):
        f = fixture(); read = lambda p: f['files'][p['path']]
        self.assertEqual(len(M.native_files(f['summary'], read)[0]), 11)
        for which in range(3):
            bad = copy.deepcopy(f['summary'])
            if which == 0: bad['files'].pop()
            elif which == 1: bad['files'][1]['name'] = 'baseline-registration.json'
            else: bad['files'][9]['sha256'] = [0] * 32
            with self.assertRaises(ValueError): M.native_files(bad, read)

    def test_body_request_bootstrap_and_close_join_to_summary(self):
        f = fixture()
        for field in ('request', 'bootstrap', 'close'):
            bad = copy.deepcopy(f['summary'])
            (bad if field == 'request' else bad['run'])[field]['extra'] = True
            with self.assertRaises(ValueError): M.native_files(bad, lambda p: f['files'][p['path']])

    def test_bootstrap_physical_page_and_genuine_input_are_not_guessed(self):
        f = fixture()
        self.assertEqual(M.bootstrap_page(f['summary'], f['bodies']), 7)
        for field, value in (('token', 785), ('generation', 2), ('rotary_bits', [0] * 128), ('cache_metadata', [0] * 145)):
            bad = copy.deepcopy(f['summary']); bad['run']['bootstrap']['input'][field] = value
            with self.assertRaises(ValueError): M.bootstrap_page(bad, f['bodies'])

    def test_bootstrap_own_child_source_body_and_scope_are_checked(self):
        f = fixture()
        for which in range(4):
            bad = copy.deepcopy(f['summary'])
            if which == 0: bad['run']['child_pid'] += 1
            elif which == 1: bad['run']['bootstrap']['begin']['registration']['sha256'] = [0] * 32
            elif which == 2: bad['run']['bootstrap']['begin']['scope']['session'] = [0] * 32
            else: bad['run']['process_group_absent'] = False
            with self.assertRaises(ValueError): M.bootstrap_page(bad, f['bodies'])

    def test_run_close_order_and_profile_are_bound_to_one_owner(self):
        f = fixture()
        for name in ('candidate-request-1.json', 'candidate-request-2.json', 'candidate-response-2.json'):
            bodies = dict(f['bodies']); doc = C.document(bodies[name]); doc['profile_sha256'] = [9] * 32; bodies[name] = raw(doc)
            with self.assertRaises(ValueError): M.bootstrap_page(f['summary'], bodies)

    def test_all_twenty_eight_stage_metadata_and_digests_are_exact(self):
        f = fixture(); self.assertEqual(len(self.stage(f)), 2)
        for field, value in (('offset', 1), ('bytes', 1), ('rank', 1), ('element_bytes', 4), ('sha256', [0] * 32)):
            bad = copy.deepcopy(f); bad['summary']['stages'][0][field] = value
            with self.assertRaises(ValueError): self.stage(bad)

    def test_full_capture_rejects_bf16_and_fp32_nonfinite_values(self):
        for stage in (0, 6, 12, 17, 26):
            f = fixture(); row = f['summary']['stages'][stage]; body = bytearray(f['bodies']['candidate-capture.bin'])
            invalid = struct.pack('<H', 0x7fc0) if row['element_bytes'] == 2 else struct.pack('<I', 0x7f800000)
            body[row['offset']:row['offset'] + len(invalid)] = invalid
            f['bodies']['candidate-capture.bin'] = bytes(body)
            row['sha256'] = list(bytes.fromhex(C.sha(body[row['offset']:row['offset'] + row['bytes']])))
            with self.assertRaises(ValueError): self.stage(f)

    def test_cache_only_current_slot_may_change_and_page_is_bounded(self):
        f = fixture(); row = f['summary']['stages'][3]; body = bytearray(f['bodies']['candidate-capture.bin'])
        body[row['offset'] + 7 * 16 * 1024] = 1
        f['bodies']['candidate-capture.bin'] = bytes(body)
        row['sha256'] = list(bytes.fromhex(C.sha(body[row['offset']:row['offset'] + row['bytes']])))
        self.stage(f)
        for page in (0, -1, 144, True):
            with self.assertRaises(ValueError): self.stage(f, page)

    def test_each_final_hidden_rank_must_match_authenticated_tf4_not_each_other_only(self):
        for index in (13, 27):
            f = fixture(); row = f['summary']['stages'][index]; body = bytearray(f['bodies']['candidate-capture.bin'])
            body[row['offset']] = 1; f['bodies']['candidate-capture.bin'] = bytes(body)
            row['sha256'] = list(bytes.fromhex(C.sha(body[row['offset']:row['offset'] + row['bytes']])))
            with self.assertRaises(ValueError): self.stage(f)
        f = fixture()
        with self.assertRaises(ValueError): M.stage_bytes(f['summary'], f['bodies']['candidate-capture.bin'], 7, f['hidden'], D)

    def test_rank_sharding_qkv_and_current_page_mapping(self):
        f = fixture(); parts = self.stage(f)
        framework = {name: f['files'][stage['pin']['path']] for name, stage in f['framework']['passes'][0]['stages'].items()}
        for name, per_rank in (('q-projection', 2048), ('k-projection', 512), ('v-projection', 512),
                               ('cache-key', 512), ('cache-value', 512)):
            framework[name] = b'\x80\x3f' * per_rank + b'\0\x40' * per_rank
        for rank in range(2):
            parts[rank]['qkv'] = framework['q-projection'][rank * 4096:(rank + 1) * 4096] + framework['k-projection'][rank * 1024:(rank + 1) * 1024] + framework['v-projection'][rank * 1024:(rank + 1) * 1024]
            for name, source in (('key-cache', 'cache-key'), ('value-cache', 'cache-value')):
                body = bytearray(parts[rank][name]); body[7 * 16 * 1024:7 * 16 * 1024 + 1024] = framework[source][rank * 1024:(rank + 1) * 1024]; parts[rank][name] = bytes(body)
        rows, earliest = M.comparisons(framework, parts, 7, D)
        self.assertIsNone(earliest)
        self.assertTrue(all(row['byte_equal'] for row in rows))
        wrong, _ = M.comparisons(framework, parts, 0, D)
        self.assertTrue(any(not row['byte_equal'] for row in wrong if row['stage'] == 'key-current'))

    def test_activation_is_product_not_silu_and_partials_are_excluded(self):
        f = fixture(); parts = self.stage(f)
        framework = {name: f['files'][stage['pin']['path']] for name, stage in f['framework']['passes'][0]['stages'].items()}
        framework['silu'] = b'\x80\x3f' * 12288
        rows, earliest = M.comparisons(framework, parts, 7, D)
        self.assertIsNone(earliest)
        self.assertEqual(len(rows), 24)
        self.assertTrue(all(row['stage'] not in ('output-partial', 'down-partial') for row in rows))

    def test_earliest_observed_stage_is_stage_order_not_rank_major_or_cause(self):
        f = fixture(); parts = self.stage(f)
        framework = {name: f['files'][stage['pin']['path']] for name, stage in f['framework']['passes'][0]['stages'].items()}
        parts[0]['gate'] = b'\x80\x3f' * 6144
        parts[1]['norm'] = b'\x80\x3f' * 4096
        rows, earliest = M.comparisons(framework, parts, 7, D)
        self.assertEqual(earliest, dict(stage='norm', observable_order=0, ranks=[1]))
        self.assertEqual([(r['observable_order'], r['rank']) for r in rows], sorted((r['observable_order'], r['rank']) for r in rows))

    def test_framework_repeat_source_and_no_conditional_input_gates_are_reused(self):
        for which in range(4):
            f = fixture()
            if which == 0: f['framework']['candidate_intermediate_inputs'] = True
            elif which == 1: f['framework']['model_sources']['config.json']['sha256'] = 'b' * 64
            elif which == 2: f['framework']['passes'][1]['stages']['q-norm']['dtype'] = 'float32'
            else: f['framework']['input_pins'][0]['sha256'] = 'b' * 64
            body = raw(f['framework']); f['files'][f['pins'][0]['path']] = body
            f['pins'][0].update(bytes=len(body), sha256=C.sha(body))
            with patch.object(C, 'EMBEDDING_SHA', C.sha(f['hidden'])), patch.object(M, 'CURRENT_HIDDEN_SHA', C.sha(f['hidden'])):
                with self.assertRaises(ValueError): M.compare_retained(*f['pins'], lambda p: f['files'][p['path']], D)

    def test_unqualified_summary_modes_nonclaims_and_changed_pins_are_refused(self):
        for field, value in (('paired_comparison_performed', True), ('native_attempts', 2), ('bitwise_equal', False), ('schema', 'old-paired')):
            f = fixture(); f['summary'][field] = value; body = raw(f['summary'])
            f['files'][f['pins'][3]['path']] = body; f['pins'][3].update(bytes=len(body), sha256=C.sha(body))
            with patch.object(C, 'EMBEDDING_SHA', C.sha(f['hidden'])), patch.object(M, 'CURRENT_HIDDEN_SHA', C.sha(f['hidden'])):
                with self.assertRaises(ValueError): M.compare_retained(*f['pins'], lambda p: f['files'][p['path']], D)
        f = fixture(); f['files'][f['pins'][3]['path']] = b'changed'
        with patch.object(C, 'EMBEDDING_SHA', C.sha(f['hidden'])), patch.object(M, 'CURRENT_HIDDEN_SHA', C.sha(f['hidden'])):
            with self.assertRaises(ValueError): M.compare_retained(*f['pins'], lambda p: f['files'][p['path']], D)


if __name__ == '__main__':
    unittest.main(verbosity=2)
