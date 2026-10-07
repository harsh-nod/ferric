"""Independent synthetic Readiness40 framing/retention fixtures; no native execution."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import unittest

import validate_readiness as V
import readiness_announcement as A

DIRECTORY = Path('/synthetic/readiness/native')
POSITIONS = (0, 15, 16, 39)


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
    request = dict(schema='FerricGuardedMlpReadiness40RequestV1', base=base,
        tiles_image=copy.deepcopy(image), prefix_image=copy.deepcopy(image),
        projection_image=copy.deepcopy(image), guarded_image=copy.deepcopy(image))
    begin = dict(scope=copy.deepcopy(scope), registration=part(b'registration'),
        source_program=part(b'program'), uploads=part(b'uploads'))
    for name in ('prefix', 'mlp', 'residual', 'tail'):
        begin[name + '_image'] = {k: images[name][k] for k in ('bytes', 'sha256')}
    seq = dict(protocol=1, profile='readiness40', device_ids=base['device_ids'], scope=scope,
        registration=begin['registration']['sha256'], begin=begin, timeout_ms=10000, prompt_tokens=prompt)
    for name, key in [('prefix_image', 'prefix_image'), ('mlp_image', 'tiles_image'),
                      ('projection_image', 'projection_image'), ('guarded_image', 'guarded_image')]:
        seq[name] = {k: request[key][k] for k in ('bytes', 'sha256')}
    outer = dict(schema='FerricGuardedMlpReadiness40BootstrapV1', sequence=seq, child_deadline_ms=3600000)
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
    for position in range(40):
        r = command_request(position, dict(op='forward', generation=position + 1,
            token=prompt[position], cache_metadata=[position] + pages, rotary_bits=[0x3f800000, 0] * 64))
        control = controls(position)
        c = dict(generation=position + 1, position=position, input_token=prompt[position], output_token=3,
            bank=dict(bank=position % 2, local_generation=position // 2 + 1,
                retired_forward=position - 1 if position >= 2 else None,
                logical_page=position // 16, page_offset=position % 16),
            control=part(control), observation=part(payload), logits=part(payload[37 * 8192:]),
            captured=position in POSITIONS,
            first_frontiers=[[position * 1000 + 10 + r, position * 1000 + 8 + r] for r in range(2)],
            final_frontiers=[[position * 1000 + 360 + r, position * 1000 + 358 + r] for r in range(2)],
            chain=[0] * 32)
        chain = hashlib.sha256(chain + wire(r) + wire(c)).digest()
        c['chain'] = list(chain)
        frame = dict(schema='FerricGuardedMlpLongResponseV2', profile='readiness40', request=r, completion=c)
        frames.append(frame)
        requests += 4 + len(wire(r)); responses += 4 + len(wire(frame))
        if position in POSITIONS:
            name = 'capture-%d.bin' % position
            bodies[str(DIRECTORY / name)] = control + payload
            captures.append(dict(position=position, file=pin(DIRECTORY / name, control + payload)))
            responses += len(control) + len(payload)
    closed = dict(schema='FerricGuardedMlpReadiness40ClosedV1',
        request=command_request(40, dict(op='close')), completed_forwards=40, generated_tokens=[],
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
    total = sum(map(len, bodies.values()))
    files = dict(frames=pin(DIRECTORY / 'frames.ndjson', framed), captures=captures,
        child_stderr=pin(DIRECTORY / 'child-stderr.bin', b''), rows=40,
        bytes_before_summary=total, summary_bytes=0, total_bytes=total, supervisor_metadata_allowance=524288)
    summary = dict(schema='FerricGuardedMlpReadiness40ObservationV1', request=request, child_pid=101,
        bootstrap=outer, profile_sha256=list(profile), registration_sha256=seq['registration'],
        source_program_sha256=begin['source_program']['sha256'], upload_manifest_sha256=begin['uploads']['sha256'],
        setup_commands=100, completed_forwards=40, prompt_positions_executed=40, generated_tokens=[],
        page_permutation=pages, transcript_sha256=list(chain), request_stream_bytes=requests,
        response_stream_bytes=responses, files=files, close=closed,
        child_exit_zero=True, process_group_absent=True, native_closed=True, gpu_execution=True,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    return summary, request, bodies, prompt


def summary_bytes(summary):
    for _ in range(8):
        raw = json.dumps(summary, indent=2, allow_nan=False).encode() + b'\n'
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


class ReadinessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = fixture()

    def rejected(self, edit):
        value = copy.deepcopy(self.base)
        edit(value)
        with self.assertRaises((ValueError, KeyError, TypeError)):
            check(value)

    def test_complete40_original_chain_and_four_selected_payloads(self):
        got = check(copy.deepcopy(self.base))
        self.assertEqual(got['completed_forwards'], 40)
        self.assertEqual(got['generated_tokens'], [])
        self.assertEqual([r['position'] for r in got['captures']], [0, 15, 16, 39])
        self.assertEqual([r['local_generation'] for r in got['captures']], [1, 8, 9, 20])
        self.assertEqual(got['observed_argmax_tokens'], [3] * 40)
        self.assertEqual(got['selected_payloads_independently_checked'], 4)
        self.assertFalse(got['unselected_payloads_independently_checked'])
        self.assertFalse(got['numerical_acceptance'])

    def test_full2303_and_legacy_profiles_are_refused(self):
        for profile in ('full2303', 'autoregressive', 'teacher_forced'):
            with self.subTest(profile=profile):
                self.rejected(lambda v: v[0]['bootstrap']['sequence'].__setitem__('profile', profile))

    def test_authentic_prompt_only_not_argmax_feedback(self):
        self.rejected(lambda v: change_frame(v, 1, lambda f: f['request']['command'].__setitem__('token', 3)))
        self.rejected(lambda v: v[3].__setitem__(0, 99))

    def test_all144_pages_and_page_boundary_are_closed(self):
        self.rejected(lambda v: v[0]['page_permutation'].__setitem__(0, 0))
        self.rejected(lambda v: change_frame(v, 16, lambda f: f['completion']['bank'].__setitem__('logical_page', 0)))
        self.rejected(lambda v: change_frame(v, 39, lambda f: f['request']['command']['cache_metadata'].pop()))

    def test_bank_retirement_local_generation_and_bool_are_checked(self):
        for name, wrong in [('bank', 0), ('local_generation', 1), ('retired_forward', 0), ('page_offset', True)]:
            with self.subTest(field=name):
                self.rejected(lambda v: change_frame(v, 39, lambda f: f['completion']['bank'].__setitem__(name, wrong)))

    def test_frame_order_missing_and_extra_are_refused(self):
        def edit(v, mode):
            path = str(DIRECTORY / 'frames.ndjson'); rows = v[2][path].splitlines()
            if mode == 'swap': rows[14], rows[15] = rows[15], rows[14]
            elif mode == 'missing': rows.pop()
            else: rows.append(rows[-1])
            data = b'\n'.join(rows) + b'\n'
            v[2][path] = data; v[0]['files']['frames'] = pin(path, data)
        for mode in ('swap', 'missing', 'extra'):
            with self.subTest(mode=mode): self.rejected(lambda v: edit(v, mode))

    def test_unselected_hashes_and_chains_remain_authenticated(self):
        self.rejected(lambda v: change_frame(v, 31, lambda f: f['completion']['control']['sha256'].__setitem__(0,
            f['completion']['control']['sha256'][0] ^ 1)))
        self.rejected(lambda v: change_frame(v, 8, lambda f: f['completion']['chain'].__setitem__(0,
            f['completion']['chain'][0] ^ 1)))

    def test_capture_path_digest_order_and_truncation(self):
        self.rejected(lambda v: v[0]['files']['captures'][0]['file'].__setitem__('path', '/wrong/capture-0.bin'))
        self.rejected(lambda v: v[0]['files']['captures'].reverse())
        self.rejected(lambda v: v[0]['files']['captures'][0]['file']['sha256'].__setitem__(0,
            v[0]['files']['captures'][0]['file']['sha256'][0] ^ 1))
        self.rejected(lambda v: v[2].__setitem__(str(DIRECTORY / 'capture-39.bin'), b''))

    def test_selected_guard_and_queue_words_are_independently_checked(self):
        original = controls(39)
        stale = bytearray(original); struct.pack_into('<I', stale, 16 + 2272 + 4384, 1)
        with self.assertRaises(ValueError): V.control(bytes(stale), 40, [[0, 0], [0, 0]])
        changed = bytearray(original); struct.pack_into('<Q', changed, 16 + 2272 + 4384 + 32 + 24, 0)
        with self.assertRaises(ValueError): V.control(bytes(changed), 40, [[0, 0], [0, 0]])

    def test_payload_finite_lowest_tie_and_signed_zero(self):
        payload = self.base[2][str(DIRECTORY / 'capture-0.bin')][242824:]
        self.assertEqual(V.payload(payload)[0], 3)
        zeros = bytearray(606976); struct.pack_into('<H', zeros, 37 * 8192, 0x8000)
        self.assertEqual(V.payload(bytes(zeros))[0], 0)
        bad = bytearray(payload); struct.pack_into('<H', bad, 0, 0x7fc0)
        with self.assertRaises(ValueError): V.payload(bytes(bad))
        self.rejected(lambda v: change_frame(v, 0, lambda f: f['completion'].__setitem__('output_token', 7)))

    def test_close_generated_tokens_and_native_retirement(self):
        self.rejected(lambda v: v[0]['close'].__setitem__('generated_tokens', [3]))
        self.rejected(lambda v: v[0]['close'].__setitem__('native_closed', False))
        self.rejected(lambda v: v[0]['close']['request'].__setitem__('id', 40))
        self.rejected(lambda v: v[0].__setitem__('process_group_absent', False))

    def test_stream_and_retention_accounting_are_exact(self):
        for name in ('request_stream_bytes', 'response_stream_bytes'):
            with self.subTest(field=name): self.rejected(lambda v: v[0].__setitem__(name, v[0][name] + 1))
        self.rejected(lambda v: v[0]['files'].__setitem__('supervisor_metadata_allowance', 0))
        self.rejected(lambda v: v[0]['files'].__setitem__('bytes_before_summary', 1))

    def test_extra_fields_authority_and_observer_stderr_refused(self):
        self.rejected(lambda v: v[0].__setitem__('numerical_acceptance', True))
        self.rejected(lambda v: v[0].__setitem__('full_long_workload', True))
        self.rejected(lambda v: v[0].__setitem__('extra', False))
        def stderr(v):
            path = str(DIRECTORY / 'child-stderr.bin')
            v[2][path] = b'unsolicited observer\n'; v[0]['files']['child_stderr'] = pin(path, v[2][path])
        self.rejected(stderr)

    def test_strict_json_scalars_and_boolean_identity(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}'):
            with self.subTest(raw=raw):
                with self.assertRaises(ValueError): V.parse(raw)
        self.assertFalse(V.same({'x': True}, {'x': 1}))
        self.rejected(lambda v: v[0].__setitem__('completed_forwards', True))

    def test_readiness_marker_is_distinct_and_unique(self):
        good = b'finite guarded readiness owned child pid=101 pgid=101; setup pending\n'
        self.assertEqual(A.announcements(good), [101])
        for bad in (b'', good + good, good.replace(b'readiness ', b''),
                    good.replace(b'pgid=101', b'pgid=102'), good.replace(b'pid=101', b'pid=0101')):
            with self.subTest(raw=bad):
                with self.assertRaises(RuntimeError): A.announcements(bad)

    def test_marker_alone_never_substitutes_owned_lineage(self):
        native, started = lineage()
        marker = b'finite guarded readiness owned child pid=101 pgid=101; setup pending\n'
        self.assertTrue(A.validate_lineage(marker, native, started, 101)['exact_two_owned_identities'])
        for key, value in [('lineage', []), ('owned_groups', [100]), ('cleanup_signalled', True),
                           ('exit_code', True), ('owned_processes_reaped', False)]:
            with self.subTest(field=key):
                broken = copy.deepcopy(native); broken[key] = value
                with self.assertRaises(RuntimeError): A.validate_lineage(marker, broken, started, 101)


if __name__ == '__main__':
    unittest.main()

