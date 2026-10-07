"""Independent synthetic Full2303 framing/retention fixtures; no native execution."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import unittest

import validate_full as V
import compare_full as C
import full_reference as R
from unittest import mock
import full_announcement as A

DIRECTORY = Path('/synthetic/full/native')
POSITIONS = (0, 2047, 2048, 2302)


def wire(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()


def part(data):
    return {'bytes': len(data), 'sha256': list(hashlib.sha256(data).digest())}


def pin(path, data):
    return {'path': str(path), **part(data)}


def controls(position):
    prefix = [1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2
    prefix += ([0xffffffff] * 4 + [3]) * 2 + [1] * 130 + [64] * 130
    mlp = [1, 0, 0, 31] + [1, 96, 96, 1, 64] * 2
    mlp += ([0xffffffff] * 8 + [3]) * 2 + [1] * 258 + [64] * 258
    body = bytearray(struct.pack('<2Q', 7, 9))
    for layer in range(36):
        body.extend(struct.pack('<568I', *(prefix * 2)))
        body.extend(struct.pack('<1096I', *(mlp * 2)))
        body.extend(struct.pack('<8I', *([position // 2 + 1, 0, 1, 0] * 2)))
        body.extend(struct.pack('<3Q', 1, 2, 3))
        for rank in range(2):
            write = position * 1000 + (layer + 1) * 10 + rank
            body.extend(struct.pack('<2Q', write, write - 2))
    body.extend(struct.pack('<3Q', 4, 5, 6))
    return bytes(body)


def fixture():
    prompt = list(range(100, 2148))
    scope = dict(bundle_id=[1] * 32, model_id=[2] * 32, session=[3] * 32,
                 pool_identity=4, group_id=5, child_identity=101)
    image = pin('/synthetic/image', b'qualified image fixture')
    images = {name: copy.deepcopy(image) for name in ('prefix', 'mlp', 'residual', 'tail')}
    base = dict(schema='FerricFiniteLongRequestV1', source='/synthetic/model',
        worker=pin('/synthetic/worker', b'elf fixture'), images=images,
        expected_bundle_id=scope['bundle_id'], expected_model_id=scope['model_id'],
        device_ids=[16366993098680759275, 10838076764495710945], session=scope['session'],
        prompt={name: pin('/synthetic/' + name, b'prompt fixture') for name in ('manifest', 'text', 'tokens')},
        evidence_directory=str(DIRECTORY), dispatch_timeout_ms=10000, child_deadline_ms=3600000)
    request = dict(schema='FerricGuardedMlpFull2303RequestV1', base=base,
        tiles_image=copy.deepcopy(image), prefix_image=copy.deepcopy(image),
        projection_image=copy.deepcopy(image), guarded_image=copy.deepcopy(image))
    request['guarded_image']['sha256'] = list(bytes.fromhex('de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'))
    request['projection_image']['sha256'] = [77] * 32
    begin = dict(scope=copy.deepcopy(scope), registration=part(b'registration'),
        source_program=part(b'program'), uploads=part(b'uploads'))
    for name in ('prefix', 'mlp', 'residual', 'tail'):
        begin[name + '_image'] = {k: images[name][k] for k in ('bytes', 'sha256')}
    seq = dict(protocol=1, profile='full2303', device_ids=base['device_ids'], scope=scope,
        registration=begin['registration']['sha256'], begin=begin, timeout_ms=10000, prompt_tokens=prompt)
    for name, key in [('prefix_image', 'prefix_image'), ('mlp_image', 'tiles_image'),
                      ('projection_image', 'projection_image'), ('guarded_image', 'guarded_image')]:
        seq[name] = {k: request[key][k] for k in ('bytes', 'sha256')}
    outer = dict(schema='FerricGuardedMlpFull2303BootstrapV1', sequence=seq, child_deadline_ms=3600000)
    profile = hashlib.sha256(b'ferric-prefix284-combined2208-closed-long-v2\0' + wire(seq)).digest()
    chain = hashlib.sha256(b'ferric-guarded-mlp-long-transcript-v2\0' + profile).digest()
    pages = list(reversed(range(144)))
    payload = bytearray(606976)
    struct.pack_into('<H', payload, 37 * 8192 + 3 * 2, 0x3f80)
    struct.pack_into('<H', payload, 37 * 8192 + 7 * 2, 0x3f80)
    payload = bytes(payload)
    def command_request(position, command):
        return dict(protocol=1, id=position + 1, device_ids=base['device_ids'], session=scope['session'],
            registration=begin['registration']['sha256'], profile_sha256=list(profile), command=command)
    frames, bodies, captures = [], {}, []
    requests = responses = 0
    payload_part, logits_part = part(payload), part(payload[37 * 8192:])
    for position in range(2303):
        r = command_request(position, dict(op='forward', generation=position + 1,
            token=prompt[position] if position < 2048 else 3, cache_metadata=[position] + pages, rotary_bits=[0x3f800000, 0] * 64))
        control = controls(position) if position in POSITIONS else None
        c = dict(generation=position + 1, position=position, input_token=prompt[position] if position < 2048 else 3, output_token=3,
            bank=dict(bank=position % 2, local_generation=position // 2 + 1,
                retired_forward=position - 1 if position >= 2 else None,
                logical_page=position // 16, page_offset=position % 16),
            control=part(control) if control is not None else dict(bytes=242824, sha256=[77] * 32),
            observation=payload_part, logits=logits_part,
            captured=position in POSITIONS,
            first_frontiers=[[position * 1000 + 10 + r, position * 1000 + 8 + r] for r in range(2)],
            final_frontiers=[[position * 1000 + 360 + r, position * 1000 + 358 + r] for r in range(2)],
            chain=[0] * 32)
        chain = hashlib.sha256(chain + wire(r) + wire(c)).digest()
        c['chain'] = list(chain)
        frame = dict(schema='FerricGuardedMlpLongResponseV2', profile='full2303', request=r, completion=c)
        frames.append(frame)
        requests += 4 + len(wire(r)); responses += 4 + len(wire(frame))
        if position in POSITIONS:
            name = 'capture-%d.bin' % position
            bodies[str(DIRECTORY / name)] = control + payload
            captures.append(dict(position=position, file=pin(DIRECTORY / name, control + payload)))
            responses += len(control) + len(payload)
    closed = dict(schema='FerricGuardedMlpFull2303ClosedV1',
        request=command_request(2303, dict(op='close')), completed_forwards=2303, generated_tokens=[3] * 256,
        transcript_sha256=list(chain), native_closed=True, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
    setup = dict(protocol=1, id=1, device_ids=base['device_ids'], session=scope['session'],
                 command=dict(op='begin', **begin))
    requests += 4 + len(wire(outer)) + sum(seq[n]['bytes'] for n in
        ('prefix_image', 'mlp_image', 'projection_image', 'guarded_image'))
    requests += 4 + len(wire(setup)) + sum(begin[n]['bytes'] for n in
        ('registration', 'source_program', 'uploads', 'prefix_image', 'mlp_image', 'residual_image', 'tail_image'))
    requests += 4 + len(wire(closed['request'])); responses += 4 + len(wire(closed))
    framed = b''.join(wire(frame) + b'\n' for frame in frames)
    bodies[str(DIRECTORY / 'frames.ndjson')] = framed
    bodies[str(DIRECTORY / 'child-stderr.bin')] = b''
    bodies[str(DIRECTORY / 'generated-text.bin')] = b'\xff synthetic raw decoded \x00'
    total = sum(map(len, bodies.values()))
    files = dict(frames=pin(DIRECTORY / 'frames.ndjson', framed), captures=captures,
        child_stderr=pin(DIRECTORY / 'child-stderr.bin', b''),
        decoded_output=pin(DIRECTORY / 'generated-text.bin', bodies[str(DIRECTORY / 'generated-text.bin')]), rows=2303,
        bytes_before_summary=total, summary_bytes=0, total_bytes=total, supervisor_metadata_allowance=524288)
    summary = dict(schema='FerricGuardedMlpFull2303ObservationV1', request=request, child_pid=101,
        bootstrap=outer, profile_sha256=list(profile), registration_sha256=seq['registration'],
        source_program_sha256=begin['source_program']['sha256'], upload_manifest_sha256=begin['uploads']['sha256'],
        setup_commands=100, completed_forwards=2303, prompt_positions_executed=2048,
        decode_positions_executed=255, decoded_special_token_policy='skip', generated_tokens=[3] * 256,
        page_permutation=pages, transcript_sha256=list(chain), request_stream_bytes=requests,
        response_stream_bytes=responses, files=files, close=closed,
        child_exit_zero=True, process_group_absent=True, native_closed=True, gpu_execution=True,
        full_long_workload=True, numerical_acceptance=False, performance_claim=False, production_authority=False)
    return summary, request, bodies, prompt


def summary_bytes(summary):
    for _ in range(8):
        raw = wire(summary) + b'\n'
        if summary['files']['summary_bytes'] == len(raw):
            return raw
        summary['files']['summary_bytes'] = len(raw)
        summary['files']['total_bytes'] = summary['files']['bytes_before_summary'] + len(raw)
    raise AssertionError('summary size fixed point')


def check(value):
    summary, request, bodies, prompt = value
    return V.validate(summary_bytes(summary), request, DIRECTORY, lambda p: bodies[p['path']], prompt)


def change_frame(value, index, edit):
    summary, _, bodies, _ = value
    path = str(DIRECTORY / 'frames.ndjson')
    frames = [json.loads(line) for line in bodies[path].splitlines()]
    before = wire(frames[index]); edit(frames[index])
    assert wire(frames[index]) != before
    raw = b''.join(wire(row) + b'\n' for row in frames)
    summary['files']['bytes_before_summary'] += len(raw) - len(bodies[path])
    bodies[path] = raw; summary['files']['frames'] = pin(path, raw)


def lineage():
    parent = dict(pid=100, pgid=100, sid=100, ppid=99, starttime=1000, state='S', uid=9661)
    worker = dict(pid=101, pgid=101, sid=100, ppid=100, starttime=1001, state='R', uid=9661)
    native = dict(exit_code=0, reason=None, cleanup_signalled=False, owned_groups_absent=True,
        owned_processes_reaped=True, lineage=[dict(event='owned', reason='spawned-parent', identity=parent),
        dict(event='owned', reason='ancestry', identity=worker)], owned_groups=[100, 101],
        command=dict(sha256='a' * 64))
    started = dict(parent=parent, supervisor_pid=99, command_sha256='a' * 64)
    return native, started


def reference_fixture():
    tokens = list(range(100, 2148))
    raw = bytes(606976)
    parts = R.D.split_payload(raw)
    greedy = R.summarize(parts['logits'])
    generated = [0] * 256
    cases = [dict(step=i, position=2047 + i, input_token=tokens[-1] if i == 0 else 0,
                  input_length=2048 if i == 0 else 1, cache_before=0 if i == 0 else 2047 + i,
                  cache_after=2048 + i, greedy=copy.deepcopy(greedy)) for i in range(256)]
    captures = []
    for p in POSITIONS:
        step = max(0, p - 2047)
        kv = dict(bytes=2 * 8 * (2048 + step) * 128, sha256='c' * 64)
        captures.append(dict(position=p, forward_call=step, generated_index=None if p == 0 else step,
            input_token=tokens[p] if p < 2048 else 0, cache_length_at_call=2048 + step,
            greedy=copy.deepcopy(greedy), payload=C.compact(raw),
            tensors={n: C.compact(b) for n, b in parts.items()},
            cache_hashes=[dict(key=kv.copy(), value=kv.copy()) for _ in range(36)]))
    outputs = {'tokens.u32le': struct.pack('<256I', *generated), 'decoded.bin': b'\xff raw\x00',
               'decoded-preserve-special.bin': b'\xff raw\x00<special>'}
    value = dict(ordinal=1, fresh_cache=True, framework_calls=256, positions_processed=2303,
        generated_tokens=generated, prompt_tokens=C.compact(struct.pack('<2048I', *tokens)),
        cases=cases, captures=captures, logit_stream=dict(bytes=77791232, sha256='a' * 64),
        logit_pin_chain_sha256=hashlib.sha256(b''.join(bytes.fromhex(c['greedy']['logits']['sha256'])
            for c in cases)).hexdigest(), output_pins={n: C.compact(b) for n, b in outputs.items()})
    passes = [value, dict(copy.deepcopy(value), ordinal=2)]
    owner = dict(passed=True, errors=[], postcheck_errors=[], container_removed=True, framework_attempts=1, retries=0)
    inner = dict(schema='ferric-full2303-framework-reference-v1', passed=True, error=None,
        postcheck_errors=[], repeat_gate_passed=True, framework_execution=True, native_execution=False,
        native_intermediates_used=False, candidate_receipt=None, decoded_special_token_policy='skip',
        selected_positions=list(POSITIONS), numerical_acceptance=False, performance_claim=False,
        production_authority=False, full_model_acceptance=False, full_prompt_tokens=tokens,
        input_tokens=tokens, passes=passes)
    bodies = {'complete.json': wire(owner), 'output/complete.json': wire(inner),
              'inputs/prompt.u32le': struct.pack('<2048I', *tokens), 'inputs/prompt.txt': b'synthetic',
              'inputs/prompt-manifest.json': wire(dict(input_token_ids=tokens))}
    for i in (1, 2):
        bodies['output/pass%d.json' % i] = wire(passes[i - 1])
        for p in POSITIONS: bodies['output/pass%d-pos%d.bf16' % (i, p)] = raw
        for name, data in outputs.items(): bodies['output/pass%d.%s' % (i, name)] = data
    return bodies


class FullTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = fixture()

    def rejected(self, edit):
        value = copy.deepcopy(self.base)
        edit(value)
        with self.assertRaises((ValueError, KeyError, TypeError)):
            check(value)

    def test_complete2303_own256_and_four_captures(self):
        got = check(copy.deepcopy(self.base))
        self.assertEqual((got['completed_forwards'], got['prompt_positions_executed'], got['decode_positions_executed']), (2303, 2048, 255))
        self.assertEqual(got['generated_tokens'], [3] * 256)
        self.assertEqual([r['position'] for r in got['captures']], list(POSITIONS))
        self.assertEqual([r['local_generation'] for r in got['captures']], [1, 1024, 1025, 1152])
        self.assertEqual(len(got['observed_argmax_tokens']), 2303)
        self.assertFalse(got['unselected_payloads_independently_checked'])
        self.assertFalse(got['numerical_acceptance'])
        self.assertFalse(got['process_retirement_independently_checked'])

    def test_readiness_ar4_and_teacher_forced_profiles_refused(self):
        for profile in ('readiness40', 'readiness40_position5', 'autoregressive', 'teacher_forced'):
            with self.subTest(profile=profile):
                self.rejected(lambda v: v[0]['bootstrap']['sequence'].__setitem__('profile', profile))

    def test_authentic_prompt_then_only_previous_own_output(self):
        for pos, wrong in [(1, 3), (2047, 3), (2048, 2147), (2200, 7)]:
            with self.subTest(position=pos):
                self.rejected(lambda v: change_frame(v, pos, lambda f: f['request']['command'].__setitem__('token', wrong)))
        self.rejected(lambda v: v[3].__setitem__(0, 99))

    def test_last_output_is_not_consumed_and_no_forward2304(self):
        seq = self.base[0]['bootstrap']['sequence']; digest = bytes(self.base[0]['profile_sha256'])
        rows = [json.loads(line) for line in self.base[2][str(DIRECTORY / 'frames.ndjson')].splitlines()]
        last = rows[-1]['request']
        self.assertEqual(V.request_row(last, seq, digest, 2302, 3)['command']['token'], 3)
        with self.assertRaises(ValueError): V.request_row(last, seq, digest, 2302, 7)
        close = self.base[0]['close']['request']
        self.assertEqual(V.request_row(close, seq, digest, 2303, 999)['command'], {'op': 'close'})
        self.rejected(lambda v: v[0].__setitem__('generated_tokens', [3] * 257))

    def test_all144_pages_and_last_page_boundaries(self):
        self.rejected(lambda v: v[0]['page_permutation'].__setitem__(0, 0))
        for pos, field, bad in [(2048, 'logical_page', 127), (2302, 'page_offset', 15)]:
            with self.subTest(position=pos):
                self.rejected(lambda v: change_frame(v, pos, lambda f: f['completion']['bank'].__setitem__(field, bad)))
        self.rejected(lambda v: change_frame(v, 2302, lambda f: f['request']['command']['cache_metadata'].pop()))

    def test_bank_retirement_generation_and_bool_refused(self):
        for name, wrong in [('bank', 1), ('local_generation', 1151), ('retired_forward', 2300), ('page_offset', True)]:
            with self.subTest(field=name):
                self.rejected(lambda v: change_frame(v, 2302, lambda f: f['completion']['bank'].__setitem__(name, wrong)))

    def test_all_original_records_order_count_and_canonical_bytes(self):
        def edit(v, mode):
            path = str(DIRECTORY / 'frames.ndjson'); rows = v[2][path].splitlines()
            if mode == 'swap': rows[2047], rows[2048] = rows[2048], rows[2047]
            elif mode == 'missing': rows.pop()
            elif mode == 'extra': rows.append(rows[-1])
            else: rows[1] += b' '
            data = b'\n'.join(rows) + b'\n'; v[2][path] = data; v[0]['files']['frames'] = pin(path, data)
        for mode in ('swap', 'missing', 'extra', 'noncanonical'):
            with self.subTest(mode=mode): self.rejected(lambda v: edit(v, mode))

    def test_unselected_original_hash_and_chain_changes_refused(self):
        for field in ('control', 'observation', 'logits'):
            with self.subTest(field=field):
                self.rejected(lambda v: change_frame(v, 2250, lambda f: f['completion'][field]['sha256'].__setitem__(0,
                    f['completion'][field]['sha256'][0] ^ 1)))
        self.rejected(lambda v: change_frame(v, 2100, lambda f: f['completion']['chain'].__setitem__(0,
            f['completion']['chain'][0] ^ 1)))

    def test_selected_capture_roster_paths_hashes_and_extent(self):
        self.rejected(lambda v: v[0]['files']['captures'][0]['file'].__setitem__('path', '/other/capture-0.bin'))
        self.rejected(lambda v: v[0]['files']['captures'].reverse())
        self.rejected(lambda v: v[0]['files']['captures'][1].__setitem__('position', 5))
        self.rejected(lambda v: v[2].__setitem__(str(DIRECTORY / 'capture-2302.bin'), b''))

    def test_control_guard_and_cross_forward_frontiers(self):
        raw = bytearray(controls(2302)); struct.pack_into('<I', raw, 16 + 2272 + 4384, 1151)
        with self.assertRaises(ValueError): V.control(bytes(raw), 2303, [[0, 0], [0, 0]])
        self.rejected(lambda v: change_frame(v, 2000, lambda f: f['completion']['first_frontiers'][0].__setitem__(0, 1)))

    def test_selected_payload_finite_lowest_tie_and_zero(self):
        raw = self.base[2][str(DIRECTORY / 'capture-0.bin')][242824:]
        self.assertEqual(V.payload(raw)[0], 3)
        zeros = bytearray(606976); struct.pack_into('<H', zeros, 37 * 8192, 0x8000)
        self.assertEqual(V.payload(bytes(zeros))[0], 0)
        for bits in (0x7f80, 0xff80, 0x7fc0):
            broken = bytearray(raw); struct.pack_into('<H', broken, 0, bits)
            with self.subTest(bits=bits), self.assertRaises(ValueError): V.payload(bytes(broken))
        self.rejected(lambda v: change_frame(v, 2047, lambda f: f['completion'].__setitem__('output_token', 7)))

    def test_close_exact256_and_retirement_flags(self):
        self.rejected(lambda v: v[0]['close']['generated_tokens'].__setitem__(255, 7))
        self.rejected(lambda v: v[0]['close'].__setitem__('native_closed', False))
        self.rejected(lambda v: v[0]['close']['request'].__setitem__('id', 2303))
        self.rejected(lambda v: v[0].__setitem__('process_group_absent', False))

    def test_both_stream_budgets_and_compact_retention_accounting(self):
        for name in ('request_stream_bytes', 'response_stream_bytes'):
            with self.subTest(field=name): self.rejected(lambda v: v[0].__setitem__(name, v[0][name] + 1))
        self.rejected(lambda v: v[0]['files'].__setitem__('supervisor_metadata_allowance', 0))
        self.rejected(lambda v: v[0]['files'].__setitem__('bytes_before_summary', 1))
        self.assertLess(V.MAX_RETAINED_BYTES, 32 << 20)

    def test_decoded_file_pin_skip_policy_and_raw_bytes(self):
        self.rejected(lambda v: v[0].__setitem__('decoded_special_token_policy', 'preserve'))
        self.rejected(lambda v: v[2].__setitem__(str(DIRECTORY / 'generated-text.bin'), b'wrong'))
        self.assertEqual(check(copy.deepcopy(self.base))['decoded_output'], part(b'\xff synthetic raw decoded \x00'))

    def test_stderr_authority_false_flags_and_strict_json(self):
        self.rejected(lambda v: v[0].__setitem__('numerical_acceptance', True))
        self.rejected(lambda v: v[0].__setitem__('full_long_workload', False))
        self.rejected(lambda v: v[0].__setitem__('completed_forwards', True))
        self.rejected(lambda v: v[1]['base'].__setitem__('child_deadline_ms', 3600000.0))
        self.rejected(lambda v: v[1]['base'].__setitem__('dispatch_timeout_ms', 10000.0))
        self.rejected(lambda v: v[0].__setitem__('extra', False))
        def stderr(v):
            path = str(DIRECTORY / 'child-stderr.bin'); v[2][path] = b'policy\n'
            v[0]['files']['child_stderr'] = pin(path, v[2][path])
        self.rejected(stderr)
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError): V.parse(raw)

    def test_fresh_full_marker_and_owned_natural_retirement(self):
        native, started = lineage()
        marker = b'finite guarded full2303 owned child pid=101 pgid=101; setup pending\n'
        self.assertTrue(A.validate_lineage(marker, native, started, 101)['exact_two_owned_identities'])
        for key, bad in [('cleanup_signalled', True), ('lineage', []), ('owned_groups', [100]),
                         ('exit_code', True), ('owned_processes_reaped', False)]:
            value = copy.deepcopy(native); value[key] = bad
            with self.subTest(field=key), self.assertRaises(RuntimeError):
                A.validate_lineage(marker, value, started, 101)
        for raw in (marker + marker, marker.replace(b'full2303', b'readiness'), b''):
            with self.assertRaises(RuntimeError): A.announcements(raw)


class BehaviorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reference = reference_fixture()

    def test_reference_two_actual_own_histories_and_matrix_capture(self):
        got = C.reference_data(self.reference)
        self.assertEqual(got['passes'][0]['cases'][-1]['cache_after'], 2303)
        self.assertIsNone(got['passes'][0]['captures'][0]['generated_index'])
        self.assertEqual(got['passes'][0]['captures'][0]['cache_length_at_call'], 2048)

    def test_reference_history_repeat_payload_and_output_drift(self):
        for mode in ('feedback', 'repeat', 'payload', 'decoded'):
            b = dict(self.reference)
            if mode in ('feedback', 'repeat'):
                inner = json.loads(b['output/complete.json'])
                if mode == 'feedback': inner['passes'][0]['cases'][1]['input_token'] = 999
                else: inner['passes'][1]['logit_stream']['sha256'] = 'b' * 64
                b['output/complete.json'] = wire(inner)
                for i in (1, 2): b['output/pass%d.json' % i] = wire(inner['passes'][i - 1])
            elif mode == 'payload': b['output/pass1-pos2302.bf16'] += b'x'
            else: b['output/pass2.decoded.bin'] += b'x'
            with self.subTest(mode=mode), self.assertRaises(ValueError): C.reference_data(b)

    def test_exact256_ids_and_raw_decoded_bytes_pass_only(self):
        got = C.compare_choices([3] * 256, b'\xff\x00x', [3] * 256, b'\xff\x00x')
        self.assertTrue(got['passed']); self.assertEqual(got['generated_ids_checked'], 256)
        self.assertFalse(got['numerical_acceptance']); self.assertFalse(got['full_tensor_numerical_acceptance'])

    def test_generated_mismatch_fails_including_ties_first_and_last(self):
        for index in (0, 5, 255):
            candidate = [3] * 256; candidate[index] = 7
            got = C.compare_choices(candidate, b'same', [3] * 256, b'same')
            self.assertFalse(got['passed']); self.assertFalse(got['reference_tie_exemption'])
            self.assertEqual(got['first_generated_id_mismatch']['index'], index)

    def test_decoded_byte_mismatch_fails_without_normalization(self):
        for left, right, offset in [(b'A\r\n', b'A\n', 1), (b'\xff', b'\xfe', 0), (b'x', b'x\x00', 1)]:
            got = C.compare_choices([3] * 256, left, [3] * 256, right)
            self.assertFalse(got['passed']); self.assertTrue(got['generated_ids_equal'])
            self.assertEqual(got['first_decoded_byte_mismatch'], offset)

    def test_comparison_never_accepts_short_bool_or_fitted_token_vectors(self):
        for vector in ([3] * 255, [3] * 257, [True] + [3] * 255, [151936] + [3] * 255):
            with self.assertRaises(ValueError): C.compare_choices(vector, b'', [3] * 256, b'')

    def test_pinned_reference_manifest_refuses_replacement_before_projection(self):
        with mock.patch.object(C, 'reference_data', side_effect=AssertionError('must not project')):
            with self.assertRaises(ValueError): C.authenticate_reference(lambda n, m: b'{}')

    def test_native_admission_original_pin_is_required_before_reference(self):
        originals = {n: b'{}' for n in C.INPUT_NAMES}
        pins = {n: C.compact(b) for n, b in originals.items()}
        pins['summary.json']['sha256'] = '0' * 64
        with mock.patch.object(C, 'authenticate_reference', side_effect=AssertionError('must not admit')):
            with self.assertRaises(ValueError): C.admit(originals, pins, lambda p: b'', lambda n, m: b'')

    def test_full_admission_joins_owned_stdout_command_and_retirement_before_mismatch(self):
        value = fixture(); summary, request, bodies, prompt = value
        raw = summary_bytes(summary)
        marker = b'finite guarded full2303 owned child pid=101 pgid=101; setup pending\n'
        result, started = lineage()
        command = wire(dict(argv=['/task/ferric-qwen3-guarded-mlp-full2303-engineering', '--request',
            '/task/request.json', '--observe-guarded-full2303', '--allow-unauthenticated-machine-code'],
            gpu_execution_requested=True))
        started['command_sha256'] = C.compact(command)['sha256']
        result['gpu_execution_requested'] = True
        original = {'summary.json': raw, 'request.json': wire(request), 'parent-command.json': command,
                    'parent-started.json': wire(started), 'parent-stderr': marker}
        for role, name in [('stdout', 'summary.json'), ('stderr', 'parent-stderr'),
                           ('started', 'parent-started.json'), ('command', 'parent-command.json')]:
            result[role] = dict(path='/task/' + name, **C.compact(original[name]))
        original['parent-result.json'] = wire(result)
        reference = C.reference_data(self.reference)
        reference['inner']['model_id'] = bytes([2] * 32).hex()
        reference['inner']['bundle_id'] = bytes([1] * 32).hex()
        reference['prompt_pins'] = {n: {k: V.rust_pin(p)[k] for k in ('bytes', 'sha256')}
                                    for n, p in request['base']['prompt'].items()}
        reference['original_files_rehashed'] = 130
        pins = {n: C.compact(b) for n, b in original.items()}
        with mock.patch.object(C, 'authenticate_reference', return_value=reference):
            got = C.admit(original, pins, lambda p: bodies[p['path']], lambda n, m: b'')
            self.assertFalse(got['passed'])
            self.assertEqual(len(got['comparison']['generated_id_mismatches']), 256)
            self.assertTrue(got['native']['process_retirement_independently_checked'])
            changed = dict(original); bad_result = copy.deepcopy(result)
            bad_result['stdout']['sha256'] = '0' * 64
            changed['parent-result.json'] = wire(bad_result)
            with self.assertRaises(ValueError):
                C.admit(changed, {n: C.compact(b) for n, b in changed.items()},
                        lambda p: bodies[p['path']], lambda n, m: b'')


if __name__ == '__main__':
    unittest.main()
