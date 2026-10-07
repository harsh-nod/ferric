import copy
from pathlib import Path
import struct
import unittest

import common
import launch
import reference as R


def inputs():
    tokens = [100 + i % 1000 for i in range(2048)]
    model_files = {n: dict(bytes=1, sha256='a' * 64) for n in
                   ['config.json', 'tokenizer.json', 'tokenizer_config.json', 'model.safetensors.index.json']
                   + ['model-%05d-of-00005.safetensors' % i for i in range(1, 6)]}
    text = b'authentic fixture text'
    workload = common.encoded(dict(schema='FerricQwen3TpWorkloadV2', requests=[
        dict(name='qwen3-8b-2048-256', prompt=text.decode(), new_tokens=256, arrival_tick=0)]))
    binary = struct.pack('<2048I', *tokens)
    prompt = dict(schema='FerricQwen3LongPromptV1', revision='b968826d9c46dd6066d109eabc6255188de91218',
        input_tokens=2048, output_tokens=256, add_special_tokens=False, chat_template=None,
        round_trip_verified=True, generated_reference=False, input_token_ids=tokens,
        tokenizer_sources={n: model_files[n] for n in ('tokenizer.json', 'tokenizer_config.json')},
        files=[dict(path=n, **common.compact(raw)) for n, raw in
               [('prompt.txt', text), ('prompt.u32le', binary), ('workload.json', workload)]])
    model = common.encoded(dict(schema='ferric-guarded-mlp-matched-input-framework-v1',
        passed=True, error=None, postcheck_errors=[], model_sources=model_files))
    owner = common.encoded(dict(schema='ferric-guarded-mlp-matched-input-owned-v1',
        passed=True, errors=[], postcheck_errors=[], container_removed=True,
        framework_attempts=1, retries=0, framework_report=common.compact(model)))
    bodies = dict(prompt_manifest=common.encoded(prompt), prompt_text=text, prompt_tokens=binary,
                  workload=workload, qualified_owner=owner, qualified_model_record=model)
    contract = dict(files={n: common.compact(raw) for n, raw in bodies.items()}, model_files=model_files,
        model_id='f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a',
        bundle_id='6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b',
        candidate_receipt=None, candidate_captures=None)
    return contract, bodies


def passes():
    tokens = [100 + i % 1000 for i in range(2048)]
    raw = bytes(R.PAYLOAD_BYTES)
    rows = {n: common.compact(b) for n, b in R.D.split_payload(raw).items()}
    cases = []
    for position in range(40):
        selected = position in R.SELECTED
        kv = dict(bytes=2 * 8 * (position + 1) * 128, sha256='b' * 64)
        cases.append(dict(position=position, generation=position + 1, input_token=tokens[position],
            predicted_token=0, cache_length=position + 1, logits=rows['logits'].copy(),
            payload=common.compact(raw) if selected else None, tensors=copy.deepcopy(rows) if selected else None,
            cache_hashes=[dict(key=kv.copy(), value=kv.copy()) for _ in range(36)] if selected else None))
    result = [dict(ordinal=i, fresh_cache=True, cases=copy.deepcopy(cases)) for i in (1, 2)]
    payloads = [{position: raw for position in R.SELECTED} for _ in (1, 2)]
    return tokens, result, payloads


def raw_tensor(value, shape):
    R.require(shape == (1, 1, 4096), 'shape')
    R.D.words(value, 4096)
    return value


class ReferenceTests(unittest.TestCase):
    def test_authentic_prompt_roles_and_full_count(self):
        contract, bodies = inputs()
        tokens, _, model = R.prompt_inputs(contract, bodies)
        self.assertEqual(len(tokens), 2048)
        self.assertEqual(len(model), 9)
        self.assertEqual([tokens[p] for p in R.SELECTED], [100, 105, 116, 139])

    def test_prompt_mutation_and_workload_join_refused(self):
        contract, bodies = inputs()
        for role in ('prompt_tokens', 'workload', 'prompt_text'):
            changed = dict(bodies)
            changed[role] += b'\x00'
            with self.subTest(role=role), self.assertRaises(ValueError):
                R.prompt_inputs(contract, changed)
        changed = dict(bodies)
        p = common.parse(changed['prompt_manifest'])
        p['input_tokens'] = 40
        changed['prompt_manifest'] = common.encoded(p)
        contract['files']['prompt_manifest'] = common.compact(changed['prompt_manifest'])
        with self.assertRaises(ValueError):
            R.prompt_inputs(contract, changed)

    def test_boolean_ids_and_wrong_model_lineage_refused(self):
        for field in ('bool', 'owner', 'model'):
            contract, bodies = inputs()
            if field == 'bool':
                p = common.parse(bodies['prompt_manifest']); p['input_token_ids'][0] = True
                bodies['prompt_manifest'] = common.encoded(p)
            if field == 'owner':
                p = common.parse(bodies['qualified_owner']); p['framework_report']['sha256'] = 'c' * 64
                bodies['qualified_owner'] = common.encoded(p)
            if field == 'model':
                contract['model_files']['config.json']['sha256'] = 'c' * 64
            contract['files'] = {n: common.compact(b) for n, b in bodies.items()}
            with self.subTest(field=field), self.assertRaises(ValueError):
                R.prompt_inputs(contract, bodies)

    def test_forty_selected_positions_and_repeat_gate(self):
        tokens, values, payloads = passes()
        self.assertTrue(R.repeat_gate(values, tokens, payloads))
        self.assertEqual(sum(c['payload'] is not None for c in values[0]['cases']), 4)
        self.assertEqual(sum(len(b) for p in payloads for b in p.values()), 4855808)
        self.assertEqual(values[0]['cases'][-1]['cache_length'], 40)

    def test_input_history_never_uses_previous_prediction(self):
        tokens, values, payloads = passes()
        self.assertNotEqual(tokens[1], values[0]['cases'][0]['predicted_token'])
        values[0]['cases'][1]['input_token'] = values[0]['cases'][0]['predicted_token']
        with self.assertRaises(ValueError):
            R.repeat_gate(values, tokens, payloads)

    def test_selected_byte_hash_or_role_mismatch_refused(self):
        tokens, values, payloads = passes()
        for field in ('bytes', 'role', 'pin'):
            changed, raw = copy.deepcopy(values), copy.deepcopy(payloads)
            if field == 'bytes':
                raw[0][5] = b'\x01\x00' + raw[0][5][2:]
            if field == 'role':
                changed[0]['cases'][5]['tensors']['silu'] = changed[0]['cases'][5]['tensors'].pop('layer0-hidden')
            if field == 'pin':
                changed[0]['cases'][5]['payload']['sha256'] = 'd' * 64
            with self.subTest(field=field), self.assertRaises(ValueError):
                R.repeat_gate(changed, tokens, raw)

    def test_capture_missing_duplicate_and_norm_join_refused(self):
        capture = R.Capture(raw_tensor)
        raw = bytes(8192)
        capture.layer(0)(None, (), (raw,))
        with self.assertRaises(ValueError):
            capture.layer(0)(None, (), raw)
        with self.assertRaises(ValueError):
            capture.payload(raw, bytes(303872))
        for i in range(1, 36): capture.layer(i)(None, (), raw)
        capture.norm_input(None, (b'\x01\x00' + raw[2:],))
        with self.assertRaises(ValueError):
            capture.payload(raw, bytes(303872))
        capture.norm = [raw]
        self.assertEqual(len(capture.payload(raw, bytes(303872))), 606976)

    def test_capture_hooks_removed_even_when_removal_fails(self):
        removed = []
        class Handle:
            def __init__(self, i): self.i = i
            def remove(self):
                removed.append(self.i)
                if self.i == 1: raise ValueError('remove failed')
        capture = R.Capture(raw_tensor)
        capture.handles = [Handle(i) for i in range(3)]
        with self.assertRaises(ValueError): capture.close()
        self.assertEqual(removed, [2, 1, 0])
        self.assertEqual(capture.handles, [])

    def test_nonfinite_shape_and_argmax_refused(self):
        tokens, values, payloads = passes()
        with self.assertRaises(ValueError): raw_tensor(bytes(8190), (1, 1, 4096))
        with self.assertRaises(ValueError): raw_tensor(b'\x80\x7f' + bytes(8190), (1, 1, 4096))
        values[0]['cases'][0]['predicted_token'] = 1
        with self.assertRaises(ValueError): R.repeat_gate(values, tokens, payloads)

    def test_repeat_payload_logit_and_cache_drift_refused(self):
        tokens, values, payloads = passes()
        for field in ('logits', 'cache', 'order'):
            changed = copy.deepcopy(values)
            if field == 'logits': changed[1]['cases'][1]['logits']['sha256'] = 'd' * 64
            if field == 'cache': changed[1]['cases'][5]['cache_hashes'][0]['key']['sha256'] = 'd' * 64
            if field == 'order': changed[1]['cases'][16]['cache_length'] = 16
            with self.subTest(field=field):
                if field == 'order':
                    with self.assertRaises(ValueError): R.repeat_gate(changed, tokens, payloads)
                else: self.assertFalse(R.repeat_gate(changed, tokens, payloads))

    def test_metric_byte_difference_does_not_create_acceptance(self):
        tokens, values, payloads = passes()
        candidates = {p: dict(position=p, input_token=tokens[p], output_token=0, payload=payloads[0][p])
                      for p in R.SELECTED}
        candidates[39]['payload'] = b'\x80\x3f' + candidates[39]['payload'][2:]
        result = R.compare_selected(values, payloads, candidates, tokens, tokens[:40])
        self.assertEqual(result['tensor_rows'], 152)
        self.assertEqual(result['comparisons'][-1]['tensors']['layer0-hidden']['exact_words'], 4095)
        self.assertFalse(result['receipt_authentication'])
        self.assertFalse(result['numerical_acceptance'])
        self.assertIsNone(result['acceptance_threshold'])
        candidates[39]['input_token'] = tokens[38]
        with self.assertRaises(ValueError): R.compare_selected(values, payloads, candidates, tokens, tokens[:40])
        candidates[39]['input_token'] = tokens[39]
        wrong_history = tokens[:40]; wrong_history[1] = 0
        with self.assertRaises(ValueError): R.compare_selected(values, payloads, candidates, tokens, wrong_history)

    def test_wrong_selection_and_trailing_bytes_refused(self):
        tokens, values, payloads = passes()
        changed = copy.deepcopy(payloads); changed[0][1] = changed[0].pop(5)
        with self.assertRaises(ValueError): R.repeat_gate(values, tokens, changed)
        changed = copy.deepcopy(payloads); changed[0][39] += b'\x00'
        with self.assertRaises(ValueError): R.repeat_gate(values, tokens, changed)
        with self.assertRaises(ValueError): common.parse(b'{"position":39}{}')
        with self.assertRaises(ValueError): common.parse(b'{"position":39,"position":0}')

    def test_container_named_user_offline_and_single_gpu(self):
        value = launch.container_environment()
        self.assertEqual((value['USER'], value['LOGNAME'], value['HOME']),
                         ('harmenon', 'harmenon', '/scratch/home'))
        self.assertEqual(value['TORCHINDUCTOR_CACHE_DIR'], '/scratch/inductor')
        self.assertEqual(value['HIP_VISIBLE_DEVICES'], '0')
        self.assertNotIn('ROCR_VISIBLE_DEVICES', value)
        self.assertNotIn('CUDA_VISIBLE_DEVICES', value)
        for name in ('HF_HUB_OFFLINE', 'TRANSFORMERS_OFFLINE', 'HF_DATASETS_OFFLINE'):
            self.assertEqual(value[name], '1')

    def test_container_exact_readonly_mounts_and_no_network(self):
        argv = launch.build_command(Path('/private/packages'))
        mounts = [argv[i + 1] for i, item in enumerate(argv) if item == '--mount']
        for target in ('/model', '/source', '/inputs', '/packages'):
            self.assertEqual(sum(',dst=' + target + ',readonly' in item for item in mounts), 1)
        self.assertIn('--network=none', argv)
        self.assertIn('--read-only', argv)
        self.assertIn('--pull=never', argv)
        self.assertEqual(argv[-4:], ['/usr/bin/python3', '-I', '-B', '/source/run.py'])
        self.assertEqual(argv.count('/dev/dri/renderD128'), 1)

    def test_wrong_container_owner_or_image_refused(self):
        base = dict(Name='/' + launch.NAME, Config=dict(Image=launch.IMAGE,
            Labels={'ferric.readiness.reference.owner': str(launch.ROOT)}), HostConfig=dict(NetworkMode='none'))
        self.assertEqual(launch.owned_container([base]), base)
        for field in ('owner', 'image', 'network', 'name'):
            changed = copy.deepcopy(base)
            if field == 'owner': changed['Config']['Labels']['ferric.readiness.reference.owner'] = '/other'
            if field == 'image': changed['Config']['Image'] = 'mutable:tag'
            if field == 'network': changed['HostConfig']['NetworkMode'] = 'host'
            if field == 'name': changed['Name'] = '/other'
            with self.subTest(field=field), self.assertRaises(ValueError):
                launch.owned_container([changed])

    def test_retirement_uses_separate_deadline(self):
        with self.assertRaises(ValueError):
            launch.leaf_guard(False, [], 1201, 1200, 30, 1200, lambda: None)
        launch.leaf_guard(True, [], 1201, 1200, 30, 1800, lambda: None)
        with self.assertRaises(ValueError):
            launch.leaf_guard(True, [], 1790, 1789, 30, 1800, lambda: None)

    def test_retirement_ignores_failed_workload_storage(self):
        def bad_storage(): raise ValueError('scratch full')
        with self.assertRaisesRegex(ValueError, 'scratch full'):
            launch.leaf_guard(False, [], 1, 0, 30, 1200, bad_storage)
        launch.leaf_guard(True, [], 1, 0, 30, 1800, bad_storage)

    def test_retirement_signal_deferred_and_recorded(self):
        state = launch.OwnerSignals(); state.retiring = True
        state(launch.signal.SIGTERM, None); state(launch.signal.SIGHUP, None)
        self.assertEqual(state.observed, [launch.signal.SIGTERM, launch.signal.SIGHUP])
        launch.leaf_guard(True, state.observed, 1, 0, 30, 1800, lambda: None)

    def test_work_signal_raises_between_cli_calls(self):
        state = launch.OwnerSignals()
        with self.assertRaises(InterruptedError): state(launch.signal.SIGTERM, None)
        self.assertEqual(state.observed, [launch.signal.SIGTERM])

    def test_unittest_census_accepts_both_supported_spellings(self):
        for modern in (False, True):
            raw = ''.join('%s (test_reference.ReferenceTests%s) ... ok\n' %
                (name, '.' + name if modern else '') for name in launch.TEST_NAMES)
            raw += '\nRan 20 tests in 0.001s\n\nOK\n'
            launch.test_census(raw.encode())
            with self.assertRaises(ValueError): launch.test_census(raw.replace('... ok', '... FAIL', 1).encode())


if __name__ == '__main__':
    unittest.main()
