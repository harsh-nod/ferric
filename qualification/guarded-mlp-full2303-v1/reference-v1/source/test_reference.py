import copy
import hashlib
from pathlib import Path
import struct
import unittest

import common
import launch
import full_reference as R
import long_reference as old

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
    rows = R.D.split_payload(raw)
    summary = R.summarize(rows['logits'])
    generated = [0] * 256
    cases = [dict(step=i, position=2047 + i, input_token=tokens[-1] if i == 0 else 0,
        input_length=2048 if i == 0 else 1, cache_before=0 if i == 0 else 2047 + i,
        cache_after=2048 + i, greedy=copy.deepcopy(summary)) for i in range(256)]
    captures = []
    for position in R.SELECTED:
        step = max(0, position - 2047)
        kv = dict(bytes=2 * 8 * (2048 + step) * 128, sha256='c' * 64)
        captures.append(dict(position=position, forward_call=step, generated_index=None if position == 0 else step,
            input_token=tokens[position] if position < 2048 else 0, cache_length_at_call=2048 + step,
            greedy=copy.deepcopy(summary), payload=common.compact(raw),
            tensors={n: common.compact(b) for n, b in rows.items()},
            cache_hashes=[dict(key=kv.copy(), value=kv.copy()) for _ in range(36)]))
    outputs = R.output_bodies(1, generated, lambda ids, skip: bytes(ids))
    value = dict(ordinal=1, fresh_cache=True, framework_calls=256, positions_processed=2303,
        generated_tokens=generated, prompt_tokens=common.compact(struct.pack('<2048I', *tokens)),
        cases=cases, captures=captures, logit_stream=dict(bytes=77791232, sha256='a' * 64),
        logit_pin_chain_sha256=hashlib.sha256(b''.join(
            bytes.fromhex(c['greedy']['logits']['sha256']) for c in cases)).hexdigest(),
        output_pins={n: common.compact(b) for n, b in outputs.items()})
    return tokens, [value, dict(copy.deepcopy(value), ordinal=2)], [
        {p: raw for p in R.SELECTED} for _ in (1, 2)], [outputs, dict(outputs)]


class ReferenceTests(unittest.TestCase):
    def test_authentic_full_prompt_and_model_roles(self):
        contract, bodies = inputs()
        tokens, _, model = R.prompt_inputs(contract, bodies)
        self.assertEqual((len(tokens), len(model)), (2048, 9))
        for role in ('prompt_tokens', 'workload', 'prompt_text'):
            changed = dict(bodies); changed[role] += b'x'
            with self.subTest(role=role), self.assertRaises(ValueError):
                R.prompt_inputs(contract, changed)

    def test_exact_prefill_and_own_choice_recurrence(self):
        tokens = list(range(2048))
        generated = []
        for step in range(256):
            ids = R.input_ids(tokens, step, generated)
            self.assertEqual(ids, tokens if step == 0 else [generated[-1]])
            generated.append((step + 3000) % R.VOCABULARY)
        self.assertEqual(len(generated), 256)
        self.assertEqual(2048 + 255, R.POSITIONS)
        with self.assertRaises(ValueError): R.input_ids(tokens, 256, generated)

    def test_reference_or_prompt_feedback_is_refused(self):
        tokens, values, payloads, outputs = passes()
        values[0]['cases'][1]['input_token'] = tokens[1]
        with self.assertRaises(ValueError): R.repeat_gate(values, tokens, payloads, outputs)
        with self.assertRaises(ValueError): R.input_ids(tokens, 2, [4])
        with self.assertRaises(ValueError): R.input_ids(tokens, 1, [True])

    def test_tie_lowest_id_signed_zero_and_exact_margin(self):
        raw = bytearray(bytes(R.VOCABULARY * 2))
        raw[0:2] = struct.pack('<H', 0x8000)
        value = R.summarize(bytes(raw))
        self.assertEqual((value['token_id'], value['runner_up_token_id'], value['maximum_tie_count']), (0, 1, R.VOCABULARY))
        self.assertEqual(value['margin_units_2_neg133'], '0')
        raw[6:8] = struct.pack('<H', 1)
        value = R.summarize(bytes(raw))
        self.assertEqual((value['token_id'], value['runner_up_token_id'], value['margin_units_2_neg133']), (3, 0, '1'))
        R.validate_summary(value)

    def test_nonfinite_shape_and_invalid_token_refused(self):
        for bits in (0x7f80, 0xff80, 0x7fc1):
            with self.subTest(bits=bits), self.assertRaises(ValueError):
                R.summarize(struct.pack('<H', bits) + bytes(2 * R.VOCABULARY - 2))
        with self.assertRaises(ValueError): R.summarize(bytes(2 * R.VOCABULARY - 2))
        with self.assertRaises(ValueError): R.token(True)
        with self.assertRaises(ValueError): R.token(R.VOCABULARY)

    def test_two_full_passes_exact_lengths_and_last_output_unconsumed(self):
        tokens, values, payloads, outputs = passes()
        self.assertTrue(R.repeat_gate(values, tokens, payloads, outputs, lambda ids, skip: bytes(ids)))
        self.assertEqual(values[0]['cases'][-1]['cache_after'], 2303)
        self.assertEqual(values[0]['cases'][-1]['position'], 2302)
        for field, replacement in [('framework_calls', 2303), ('positions_processed', 2304), ('generated_tokens', [0] * 255)]:
            changed = copy.deepcopy(values); changed[0][field] = replacement
            with self.subTest(field=field), self.assertRaises(ValueError):
                R.repeat_gate(changed, tokens, payloads, outputs)

    def test_capture_matrix_slices_and_norm_hook_custody(self):
        zero, one = bytes(8192), struct.pack('<H', 0x3f80) + bytes(8190)
        tensor = {0: zero, 2047: one}
        capture = R.Capture(lambda value, p: value[p], (0, 2047))
        for layer in range(36): capture.layer(layer)(None, (), (tensor,))
        capture.norm_input(None, (tensor,))
        self.assertEqual(capture.payload(0, zero, bytes(303872))[:8192], zero)
        self.assertEqual(capture.payload(2047, one, bytes(303872))[:8192], one)
        with self.assertRaises(ValueError): capture.layer(0)(None, (), tensor)
        capture.norm[2047] = zero
        with self.assertRaises(ValueError): capture.payload(2047, one, bytes(303872))
        removed = []
        class Handle:
            def remove(self): removed.append(1); raise ValueError('remove failed')
        capture.handles = [Handle(), Handle()]
        with self.assertRaises(ValueError): capture.close()
        self.assertEqual(removed, [1, 1])
        self.assertEqual(capture.handles, [])

    def test_capture_selection_and_position0_not_output(self):
        tokens, values, payloads, outputs = passes()
        for change in ('selection', 'generated', 'cache', 'trailing'):
            v, p = copy.deepcopy(values), copy.deepcopy(payloads)
            if change == 'selection': p[0][5] = p[0].pop(2047)
            if change == 'generated': v[0]['captures'][0]['generated_index'] = 0
            if change == 'cache': v[0]['captures'][0]['cache_length_at_call'] = 1
            if change == 'trailing': p[0][2302] += b'x'
            with self.subTest(change=change), self.assertRaises(ValueError):
                R.repeat_gate(v, tokens, p, outputs)

    def test_raw_decode_skip_special_and_invalid_utf8_preserved(self):
        document = dict(model=dict(type='BPE', vocab={'A': 0, chr(255): 1}), decoder=dict(type='ByteLevel'),
                        added_tokens=[dict(id=2, content='<special>', special=True)])
        decode, special = old.raw_decoder(document)
        self.assertEqual(special, {2})
        self.assertEqual(decode([0, 1, 2], True), b'A' + bytes([255]))
        self.assertEqual(decode([0, 1, 2], False), b'A' + bytes([255]) + b'<special>')
        with self.assertRaises(UnicodeDecodeError): decode([1], True).decode('utf-8')
        with self.assertRaises(ValueError): decode([3], True)
        with self.assertRaises(ValueError): decode([True], True)

    def test_decoded_bytes_and_token_pin_mismatch_refused(self):
        tokens, values, payloads, outputs = passes()
        for name in outputs[0]:
            changed = copy.deepcopy(outputs); changed[0][name] += b'x'
            with self.subTest(name=name), self.assertRaises(ValueError):
                R.repeat_gate(values, tokens, payloads, changed)
        with self.assertRaises(ValueError):
            R.repeat_gate(values, tokens, payloads, outputs, lambda ids, skip: b'wrong bytes')

    def test_repeat_stream_row_cache_and_decoded_drift_not_accepted(self):
        tokens, values, payloads, outputs = passes()
        for change in ('stream', 'cache', 'decoded'):
            v, out = copy.deepcopy(values), copy.deepcopy(outputs)
            if change == 'stream': v[1]['logit_stream']['sha256'] = 'b' * 64
            if change == 'cache':
                for capture in v[1]['captures'][:2]:
                    capture['cache_hashes'][0]['key']['sha256'] = 'd' * 64
            if change == 'decoded':
                out[1]['decoded.bin'] += b'x'
                v[1]['output_pins']['decoded.bin'] = common.compact(out[1]['decoded.bin'])
            with self.subTest(change=change):
                self.assertFalse(R.repeat_gate(v, tokens, payloads, out))
        values[0]['cases'][4]['greedy']['logits']['sha256'] = 'e' * 64
        with self.assertRaises(ValueError): R.repeat_gate(values, tokens, payloads, outputs)

    def test_evidence_bounds_and_closed_metadata(self):
        tokens, values, payloads, outputs = passes()
        self.assertLess(R.evidence_bound(values, payloads, outputs), 12 << 20)
        with self.assertRaises(ValueError):
            R.output_bodies(1, [0] * 256, lambda ids, skip: bytes((128 << 10) + 1))
        values[0]['acceptance_threshold'] = 0.1
        with self.assertRaises(ValueError): R.repeat_gate(values, tokens, payloads, outputs)
        with self.assertRaises(ValueError): common.parse(b'{"x":1,"x":2}')
        with self.assertRaises(ValueError): common.parse(b'{"x":1}{}')

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
