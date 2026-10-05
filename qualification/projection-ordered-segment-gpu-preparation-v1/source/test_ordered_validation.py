"""Synthetic complete AR4 captures; no subprocess, model, or GPU execution."""
import copy
import hashlib
import json
import struct
import unittest

import ordered_validation as V

def encode(value):
    return (json.dumps(value, separators=(',', ':'), allow_nan=False) + '\n').encode()


def part(raw):
    return dict(bytes=len(raw), sha256=list(hashlib.sha256(raw).digest()))


def control_fixture():
    result = bytearray(struct.pack('<2Q', 1, 2))
    for layer in range(36):
        for rank in range(2):
            owners = [1 + (layer + rank + i) % 64 for i in range(130)]
            prefix = [1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2
            prefix += ([0xffffffff] * 4 + [3]) * 2 + owners + [64] * 130
            result.extend(struct.pack('<284I', *prefix))
        for rank in range(2):
            owners = [1 + (layer + rank + i) % 64 for i in range(258)]
            mlp = [1, 0, 0, 31] + [1, 96, 96, 1, 64] * 2
            mlp += ([0xffffffff] * 8 + [3]) * 2 + owners + [64] * 258
            result.extend(struct.pack('<548I', *mlp))
        result.extend(struct.pack('<5Q', *range(5)))
    result.extend(struct.pack('<3Q', 3, 4, 5))
    return bytes(result)


def serialize(value):
    for _ in range(8):
        raw = encode(value)
        if value['files']['summary_bytes'] == len(raw): return raw
        value['files']['summary_bytes'] = len(raw)
        value['files']['total_bytes'] = value['files']['bytes_before_summary'] + len(raw)
    raise AssertionError('fixed summary extent')


def fixture(mode='autoregressive', winners=(7, 8, 9, 10), inputs=None, mutate_payload=None):
    inputs = (V.TOKENS[:] if mode == 'teacher_forced' else [9112, *winners[:3]]) if inputs is None else inputs
    simple = dict(path='/inputs/image', bytes=1, sha256=[1] * 32)
    request = dict(schema='FerricFinitePrefixDecodeRequestV1', source='/model', worker=copy.deepcopy(simple),
        images={name: copy.deepcopy(simple) for name in ('prefix', 'mlp', 'residual', 'tail')},
        expected_bundle_id=list(bytes.fromhex(V.BUNDLE_ID)), expected_model_id=list(bytes.fromhex(V.MODEL_ID)),
        device_ids=[11, 22], session=[3] * 32, prompt={name: dict(path='/prompt/' + name, bytes=size,
            sha256=list(bytes.fromhex(digest))) for name, (size, digest) in V.PROMPT_PINS.items()},
        mode=mode, tiles_image=copy.deepcopy(simple), prefix_image=copy.deepcopy(simple),
        evidence_directory='/task/decode', dispatch_timeout_ms=10000, child_deadline_ms=3600000)
    registration = bytes([4] * 32)
    scope = dict(bundle_id=request['expected_bundle_id'][:], model_id=request['expected_model_id'][:],
        session=[3] * 32, pool_identity=1, group_id=0, child_identity=17)
    begin = dict(scope=copy.deepcopy(scope), registration=dict(bytes=1, sha256=[4] * 32),
        source_program=dict(bytes=1, sha256=[5] * 32), uploads=dict(bytes=1, sha256=[6] * 32),
        **{name + '_image': dict(bytes=1, sha256=[1] * 32) for name in ('prefix', 'mlp', 'residual', 'tail')})
    boot = dict(protocol=1, profile='prefix284_mlp548_four_forward_v1', device_ids=[11, 22], scope=scope,
        registration=list(registration), begin=begin, timeout_ms=10000, mode=mode,
        input_tokens=V.TOKENS[:] if mode == 'teacher_forced' else [9112],
        tiles_image=dict(bytes=1, sha256=[1] * 32), prefix_image=dict(bytes=1, sha256=[1] * 32))
    material = b'ferric-prefix284-mlp548-four-decode-v6\0' + bytes(scope['bundle_id']) + bytes(scope['model_id'])
    material += bytes([3] * 32) + struct.pack('<QQI', 1, 0, 17) + registration + bytes([1] * 64)
    material += struct.pack('<IBQQ4I', 10000, int(mode == 'autoregressive'), 11, 22,
        *(V.TOKENS if mode == 'teacher_forced' else [9112, 0, 0, 0]))
    base_profile = hashlib.sha256(material).digest()
    selected = dict(bytes=2, sha256=[9] * 32)
    outer = dict(schema='FerricFiniteProjectionResidualMlpOrderedRequestV1', decode=request,
        projection_residual_image=dict(path='/inputs/projection', **selected))
    wrapper = dict(schema='FerricProjectionResidualMlpOrderedBootstrapV1', decode=boot,
        projection_residual_image=selected)
    profile = hashlib.sha256(b'ferric-projection-residual-four-decode-v1\0' +
        b'ordered-fp32-tp2-bf16-projection-then-bf16-residual-v1' + base_profile + bytes([9] * 32)).digest()
    profile = hashlib.sha256(b'ferric-projection-residual-mlp-ordered-ar4-shared-v1\0' + profile).digest()
    observed = dict(schema='FerricFiniteProjectionResidualMlpOrderedObservationV1', request=outer, child_pid=17,
        registration_sha256=list(registration), source_program_sha256=[5] * 32, upload_manifest_sha256=[6] * 32,
        bootstrap=wrapper, profile_sha256=list(profile), setup_commands=8192, completed_forwards=4,
        input_tokens=inputs[:], observed_output_tokens=list(winners), page_permutation=list(range(144)),
        transcript_sha256=[0] * 32, request_stream_bytes=16384, response_stream_bytes=4000000,
        files=dict(frames=[], child_stderr=None, bytes_before_summary=0, summary_bytes=0, total_bytes=0),
        close=None, child_exit_zero=True, process_group_absent=True, native_closed=True, gpu_execution=True,
        numerical_acceptance=False, performance_claim=False, production_authority=False, full_long_workload=False,
        paired_comparison_performed=False, native_attempts=1, retries=0)
    def response(position, event, closed=False):
        return dict(protocol=1, id=position + 1, device_ids=[11, 22], session=[3] * 32,
            registration=list(registration), profile_sha256=list(profile), event=event, native_closed=closed,
            gpu_execution=True, numerical_acceptance=False, performance_claim=False, production_authority=False)
    def pin(name, raw): return dict(path='/task/decode/' + name, **part(raw))
    control = control_fixture(); files = {'child-stderr.bin': b''}
    chain = hashlib.sha256(b'ferric-prefix284-mlp548-four-transcript-v1\0' + registration + profile).digest()
    for position, token in enumerate(inputs):
        output = winners[position]; data = bytearray(V.OBSERVATION_BYTES)
        struct.pack_into('<H', data, 37 * 8192 + 2 * output, 0x4000)
        if mutate_payload: mutate_payload(position, data)
        main = bytes(data)
        q = dict(protocol=1, id=position + 1, device_ids=[11, 22], session=[3] * 32,
            registration=list(registration), profile_sha256=list(profile), command=dict(op='forward',
            generation=position + 1, token=token, cache_metadata=[position] + list(range(144)),
            rotary_bits=[0x3f800000] * 64 + [0] * 64))
        raw_request = encode(q)
        event = dict(status='completed', generation=position + 1, position=position, input_token=token,
            output_token=output, control=part(control), observation=part(main), capture=dict(
                layer_hidden=[part(main[i * 8192:(i + 1) * 8192]) for i in range(36)],
                final_normalized=part(main[36 * 8192:37 * 8192]), logits=part(main[37 * 8192:]), total=part(main)), chain=None)
        material = chain + struct.pack('<QIII', position + 1, position, token, output)
        for name in ('control', 'observation'):
            material += struct.pack('<I', event[name]['bytes']) + bytes(event[name]['sha256'])
        chain = hashlib.sha256(material).digest(); event['chain'] = list(chain)
        observed['files']['frames'].append(dict(response=response(position, event),
            control=pin(f'control-{position}.bin', control), observation=pin(f'observation-{position}.bin', main),
            request=pin(f'request-{position}.json', raw_request)))
        files.update({f'control-{position}.bin': control, f'observation-{position}.bin': main,
                      f'request-{position}.json': raw_request})
    observed['transcript_sha256'] = list(chain)
    observed['close'] = response(4, dict(status='closed', completed_forwards=4, transcript_sha256=list(chain)), True)
    observed['files']['child_stderr'] = pin('child-stderr.bin', b'')
    observed['files']['bytes_before_summary'] = sum(map(len, files.values()))
    return observed, files


def refresh(value, files, name):
    raw = files[name]
    for frame in value['files']['frames']:
        for field in ('request', 'control', 'observation'):
            if frame[field]['path'].endswith('/' + name):
                frame[field].update(part(raw))
    value['files']['bytes_before_summary'] = sum(map(len, files.values()))
    value['files']['summary_bytes'] = 0


class ValidationTests(unittest.TestCase):
    def check(self, value, files):
        return V.validate(serialize(value), files, value['request'])

    def test_complete_ar4_binds_all_payloads_controls_and_new_profile(self):
        value, files = fixture()
        result = self.check(value, files)
        self.assertEqual((result['captured_tensor_rows'], result['captured_payloads']), (152, 4))
        self.assertEqual(result['closed_child_pids'], [17])
        self.assertEqual(result['terminal_state_count'], 576)
        self.assertEqual(result['input_tokens'], [9112, 7, 8, 9])
        self.assertEqual(result['output_tokens'], [7, 8, 9, 10])
        self.assertEqual(result['profile_sha256'], bytes(value['profile_sha256']).hex())
        self.assertIs(result['own_output_trajectory_checked'], True)
        self.assertIs(result['teacher_forced_token_parity_required'], False)
        for name in ('numerical_acceptance', 'performance_claim', 'production_authority',
                     'paired_comparison_performed', 'old_native_equality_required'):
            self.assertIs(result[name], False)

    def test_exact_outer_request_schema_ar_only_and_closed_fields(self):
        for mode in ('old-schema', 'tf', 'extra', 'different-request'):
            value, files = fixture()
            expected = copy.deepcopy(value['request'])
            if mode == 'old-schema': value['schema'] = 'FerricFinitePrefixDecodeObservationV1'
            elif mode == 'tf': value['request']['decode']['mode'] = 'teacher_forced'
            elif mode == 'extra': value['request']['clock'] = True
            else: expected['decode']['worker']['sha256'][0] ^= 1
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                V.validate(serialize(value), files, expected)

    def test_candidate_identity_extent_base_profile_and_original_copy_are_joined(self):
        for mode in ('hash', 'size', 'zero', 'copy', 'base', 'old-profile'):
            value, files = fixture(); outer = value['request']; b = value['bootstrap']
            if mode == 'hash': b['projection_residual_image']['sha256'][0] ^= 1
            elif mode == 'size': b['projection_residual_image']['bytes'] += 1
            elif mode == 'zero': b['projection_residual_image']['sha256'] = [0] * 32
            elif mode == 'copy':
                outer['projection_residual_image']['sha256'] = [1] * 32
                b['projection_residual_image']['sha256'] = [1] * 32
            elif mode == 'base': b['decode']['scope']['child_identity'] += 1
            else: value['profile_sha256'] = list(V.base_profile(b['decode'], outer['decode'], 17, bytes([4] * 32)))
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.check(value, files)

    def test_extra_image_is_charged_to_original_combined_input_budget(self):
        value, _ = fixture(); outer = value['request']; b = value['bootstrap']
        for key, size in [('tiles_image', 32 << 20), ('prefix_image', (32 << 20) - (1 << 20))]:
            outer['decode'][key]['bytes'] = size
            b['decode'][key]['bytes'] = size
        V.base_profile(b['decode'], outer['decode'], 17, bytes([4] * 32))
        outer['projection_residual_image']['bytes'] = b['projection_residual_image']['bytes'] = 2 << 20
        with self.assertRaises(ValueError): V.profile(b, outer, 17, bytes([4] * 32))

    def test_last_layer_last_rank_terminal_state_is_still_checked(self):
        value, files = fixture()
        raw = bytearray(files['control-3.bin'])
        offset = 16 + 35 * (2 * (284 + 548) * 4 + 40) + 2 * 284 * 4 + 548 * 4 + 547 * 4
        struct.pack_into('<I', raw, offset, 0)
        files['control-3.bin'] = bytes(raw); refresh(value, files, 'control-3.bin')
        value['files']['frames'][3]['response']['event']['control'] = part(bytes(raw))
        with self.assertRaises(ValueError): self.check(value, files)

    def test_nonfinite_payload_and_wrong_lowest_argmax_are_refused(self):
        for change in ('nan', 'argmax'):
            def mutate(position, raw):
                if position == 0:
                    struct.pack_into('<H', raw, 0 if change == 'nan' else 37 * 8192,
                                     0x7fc0 if change == 'nan' else 0x4040)
            value, files = fixture(mutate_payload=mutate)
            with self.subTest(change=change), self.assertRaises(ValueError): self.check(value, files)

    def test_every_actual_input_generation_page_and_profile_is_checked(self):
        for key in ('token', 'generation', 'cache_metadata', 'rotary_bits', 'profile_sha256'):
            value, files = fixture(); q = json.loads(files['request-2.json'])
            if key == 'profile_sha256': q[key][0] ^= 1
            elif key in ('cache_metadata', 'rotary_bits'):
                q['command'][key][0] = 0x7f800000 if key == 'rotary_bits' else 0
            else: q['command'][key] += 1
            files['request-2.json'] = encode(q); refresh(value, files, 'request-2.json')
            with self.subTest(key=key), self.assertRaises(ValueError): self.check(value, files)

    def test_chain_close_and_actual_reap_flags_cannot_be_faked(self):
        for key in ('chain', 'close-profile', 'close-count', 'child_exit_zero', 'process_group_absent', 'native_closed'):
            value, files = fixture()
            if key == 'chain': value['files']['frames'][3]['response']['event']['chain'][0] ^= 1
            elif key == 'close-profile': value['close']['profile_sha256'][0] ^= 1
            elif key == 'close-count': value['close']['event']['completed_forwards'] = 3
            else: value[key] = False
            with self.subTest(key=key), self.assertRaises(ValueError): self.check(value, files)

    def test_complete_body_roster_and_every_hashed_extent_remain_closed(self):
        for mode in ('missing', 'extra', 'body', 'path', 'size'):
            value, files = fixture()
            if mode == 'missing': del files['observation-3.bin']
            elif mode == 'extra': files['unexpected'] = b'x'
            elif mode == 'body': files['observation-0.bin'] += b'x'
            elif mode == 'path': value['files']['frames'][0]['control']['path'] = '/other/control'
            else: value['files']['frames'][0]['observation']['bytes'] -= 1
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.check(value, files)

    def test_claims_retries_and_final_private_accounting_refuse(self):
        for key in ('numerical_acceptance', 'performance_claim', 'production_authority',
                    'full_long_workload', 'paired_comparison_performed', 'retries', 'native_attempts', 'total'):
            value, files = fixture()
            if key == 'total':
                raw = serialize(value); value['files']['total_bytes'] += 1
                raw = encode(value)
            else:
                value[key] = 2 if key == 'native_attempts' else (1 if key == 'retries' else True)
                raw = serialize(value)
            with self.subTest(key=key), self.assertRaises(ValueError):
                V.validate(raw, files, value['request'])

    def test_duplicate_json_keys_and_boolean_counter_are_not_scalar_authority(self):
        value, files = fixture(); raw = serialize(value)
        with self.assertRaises(ValueError):
            V.validate(raw.replace(b'"child_pid":17', b'"child_pid":17,"child_pid":17'), files, value['request'])
        value['native_attempts'] = True
        with self.assertRaises(ValueError): self.check(value, files)

    def test_stderr_requires_one_worker_marker_and_all_four_terminal_lines(self):
        raw = (b'finite engineering owned child pid=17 pgid=17; no native setup acknowledged\n'
            b'finite explicit profile=projection-residual-mlp-ordered-ar4-shared-v1 mode=Autoregressive\n'
            + b''.join(f'finite prefix decode completed position={n} forwards={n + 1}\n'.encode() for n in range(4)))
        self.assertEqual(V.child_marker(raw, 17), 17)
        for bad in (raw + b'warning\n', raw[:-1], raw.replace(b'pid=17', b'pid=18'),
                    raw.replace(b'Autoregressive', b'TeacherForced')):
            with self.assertRaises(ValueError): V.child_marker(bad, 17)

    def test_closed_teacher_forced_transcript_cannot_be_admitted_as_ar(self):
        value, files = fixture(mode='teacher_forced')
        with self.assertRaises(ValueError): self.check(value, files)

    def test_wrong_seed_refuses_even_with_self_consistent_hashes_and_chain(self):
        for seed in (0, 785, 9113):
            value, files = fixture(inputs=[seed, 7, 8, 9])
            with self.subTest(seed=seed), self.assertRaises(ValueError): self.check(value, files)

    def test_every_predecessor_must_be_own_checked_output_not_tf_prompt(self):
        for position in (1, 2, 3):
            inputs = [9112, 7, 8, 9]
            inputs[position] = V.TOKENS[position]
            value, files = fixture(inputs=inputs)
            with self.subTest(position=position), self.assertRaises(ValueError): self.check(value, files)

    def test_zero_and_repeated_outputs_are_valid_recurrent_inputs(self):
        for winners in ((0, 0, 0, 0), (37, 37, 37, 37), (151935, 4, 0, 2190)):
            value, files = fixture(winners=winners)
            result = self.check(value, files)
            self.assertEqual(result['input_tokens'], [9112, *winners[:3]])
            self.assertEqual(result['output_tokens'], list(winners))

    def test_bootstrap_seed_and_mode_are_bound_into_ar_profile(self):
        for mutation in ('tf-mode', 'tf-seeds', 'wrong-seed', 'empty-seed'):
            value, files = fixture()
            bootstrap = value['bootstrap']['decode']
            if mutation == 'tf-mode': bootstrap['mode'] = 'teacher_forced'
            elif mutation == 'tf-seeds': bootstrap['input_tokens'] = V.TOKENS[:]
            elif mutation == 'wrong-seed': bootstrap['input_tokens'] = [785]
            else: bootstrap['input_tokens'] = []
            with self.subTest(mutation=mutation), self.assertRaises(ValueError): self.check(value, files)


class OrderedBoundaryTests(unittest.TestCase):
    def test_ordered_control_never_enters_legacy_decoder(self):
        import decode_validation as old
        import test_decode_validation as previous
        with self.assertRaises(ValueError): old.control(control_fixture())
        with self.assertRaises(ValueError): V.control(previous.control_fixture())

    def test_exact_single_segment_timing_without_fabricated_stage_pairs(self):
        rows = V.control_timings(control_fixture())
        self.assertEqual(len(rows), 36)
        self.assertEqual(rows[0], dict(prefix_ns=[0, 1], segment_host_ns=2, final_residual_ns=[3, 4]))
        self.assertNotIn('mlp_ns', rows[0])
        self.assertNotIn('first_residual_ns', rows[0])

    def test_legacy_profile_domain_refused_even_for_matching_image(self):
        value, files = fixture()
        import decode_validation as old
        original = copy.deepcopy(value['bootstrap']); original['schema'] = old.BOOTSTRAP_SCHEMA
        outer = copy.deepcopy(value['request']); outer['schema'] = old.REQUEST_SCHEMA
        legacy = old.profile(original, outer, value['child_pid'], bytes(value['registration_sha256']))
        self.assertNotEqual(bytes(value['profile_sha256']), legacy)
        value['profile_sha256'] = list(legacy)
        with self.assertRaises(ValueError): V.validate(serialize(value), files, value['request'])


if __name__ == '__main__':
    unittest.main()
