"""Data-only reusable AR4 checks; no numerical or performance admission."""
import hashlib
import json
import struct

CONTROL_BYTES = 242824
PAYLOAD_BYTES = 606976
IDS = [16366993098680759275, 10838076764495710945]
SEED = [9112, 2190, 3772, 220]
FALSE = ('numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed object fields')


def uint(value, maximum=(1 << 64) - 1):
    require(type(value) is int and 0 <= value <= maximum, 'unsigned integer')
    return value


def octets(value, count=32):
    require(type(value) is list and len(value) == count, 'byte-array extent')
    return bytes(uint(v, 255) for v in value)


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def part(raw):
    return dict(bytes=len(raw), sha256=list(hashlib.sha256(raw).digest()))


def rust_pin(value):
    keys(value, 'path bytes sha256')
    return dict(path=value['path'], bytes=uint(value['bytes']), sha256=octets(value['sha256']).hex())


def prefix(p):
    require(p[:4] == (1, 0, 0, 31) and p[4:9] == p[9:14] == (1, 48, 1, 16, 64)
            and p[14:19] == p[19:24] == (0xffffffff,) * 4 + (3,)
            and all(1 <= v <= 64 for v in p[24:154]) and p[154:] == (64,) * 130,
            'Prefix284 terminal words')


def mlp(p):
    require(p[:4] == (1, 0, 0, 31) and p[4:9] == p[9:14] == (1, 96, 96, 1, 64)
            and p[14:23] == p[23:32] == (0xffffffff,) * 8 + (3,)
            and all(1 <= v <= 64 for v in p[32:290]) and p[290:] == (64,) * 258,
            'combined548 prefix terminal words')


def control(raw, generation, previous):
    require(len(raw) == CONTROL_BYTES and uint(generation, 4) >= 1, 'control extent/generation')
    offset = 16
    current = list(previous)
    guards = []
    for _ in range(36):
        for _rank in range(2):
            prefix(struct.unpack_from('<284I', raw, offset)); offset += 284 * 4
        for _rank in range(2):
            mlp(struct.unpack_from('<548I', raw, offset)); offset += 548 * 4
        layer_guards = []
        for _rank in range(2):
            value = struct.unpack_from('<4I', raw, offset); offset += 16
            require(value == ((generation - 1) // 2 + 1, 0, 1, 0), 'stale/rejected guarded generation')
            layer_guards.append(list(value))
        guards.append(layer_guards)
        offset += 24
        for rank in range(2):
            write, read = struct.unpack_from('<QQ', raw, offset); offset += 16
            require(read <= write and write > current[rank][0] and read >= current[rank][1],
                    'queue frontier ordering')
            current[rank] = (write, read)
    require(offset + 24 == len(raw), 'control trailing bytes')
    return current, guards


def payload(raw):
    require(len(raw) == PAYLOAD_BYTES, 'payload extent')
    words = struct.unpack('<303488H', raw)
    require(all(v & 0x7f80 != 0x7f80 for v in words), 'nonfinite payload')
    # BF16 bit ordering suffices; signed zeros compare equal as in IEEE f32.
    def order(v):
        return 0 if v & 0x7fff == 0 else (-int(v & 0x7fff) if v & 0x8000 else v)
    logits = words[37 * 4096:]
    winner = max(range(len(logits)), key=lambda i: order(logits[i]))
    capture = dict(layer_hidden=[part(raw[i * 8192:(i + 1) * 8192]) for i in range(36)],
                   final_normalized=part(raw[36 * 8192:37 * 8192]),
                   logits=part(raw[37 * 8192:]), total=part(raw))
    return winner, capture


def profile(bootstrap):
    require(bootstrap['schema'] == 'FerricGuardedMlpReusableAr4BootstrapV1',
            'explicit reusable bootstrap')
    b = bootstrap['decode']; s = b['scope']
    h = hashlib.sha256(b'ferric-prefix284-combined2208-four-decode-v1\0')
    for value in (s['bundle_id'], s['model_id'], s['session'], b['registration'],
                  b['prefix_image']['sha256'], b['tiles_image']['sha256'],
                  bootstrap['projection_image']['sha256'], bootstrap['guarded_image']['sha256']):
        h.update(octets(value))
    for value in (s['pool_identity'], s['group_id']):
        h.update(struct.pack('<Q', uint(value)))
    h.update(struct.pack('<II', uint(s['child_identity'], 0xffffffff), uint(b['timeout_ms'], 10000)))
    require(b['mode'] == 'autoregressive', 'reusable route requires autoregressive input')
    ar = b['mode'] == 'autoregressive'
    require(b['input_tokens'] == (SEED[:1] if ar else SEED), 'profile seeds')
    h.update(bytes([ar]))
    for value in b['device_ids']:
        h.update(struct.pack('<Q', uint(value)))
    for value in ([SEED[0], 0, 0, 0] if ar else SEED):
        h.update(struct.pack('<I', value))
    return hashlib.sha256(b'ferric-guarded-mlp-reusable-ar4-profile-v1\0' + h.digest()).digest()


def arena_census(raw, bootstrap):
    require(0 < len(raw) <= 4096, 'bounded reusable arena census')
    value = parse(raw)
    keys(value, 'schema profile_sha256 registration session device_ids allocation_counts '
                'completed_forwards native_closed performance_claim')
    decode = bootstrap['decode']
    require(value['schema'] == 'FerricGuardedMlpReusableAr4ArenaCensusV1'
            and octets(value['profile_sha256']) == profile(bootstrap)
            and octets(value['registration']) == octets(decode['registration'])
            and octets(value['session']) == octets(decode['scope']['session'])
            and value['device_ids'] == decode['device_ids'] == IDS
            and all(type(item) is int for item in value['device_ids'])
            and uint(value['completed_forwards']) == 4
            and value['native_closed'] is True and value['performance_claim'] is False,
            'reusable census profile, identity and healthy Close')
    counts = value['allocation_counts']
    require(type(counts) is list and len(counts) == 5, 'five actual census samples')
    for row in counts:
        require(type(row) is list and len(row) == 2, 'two-rank census extent')
        for count in row:
            uint(count, 2048)
    require(counts == [[715, 711], [751, 747], [787, 783], [787, 783], [787, 783]],
            'actual arena first uses and plateau')
    return value


def envelope(row, observation, ident):
    keys(row, 'schema protocol id device_ids session registration profile_sha256 event native_closed '
              'gpu_execution numerical_acceptance full_model_acceptance performance_claim production_authority')
    require(row['schema'] == 'FerricGuardedMlpDecodeObservationV1' and uint(row['protocol']) == 1
            and uint(row['id']) == ident and row['device_ids'] == IDS
            and all(type(v) is int for v in row['device_ids'])
            and row['session'] == observation['request']['decode']['session']
            and row['registration'] == observation['registration_sha256']
            and row['profile_sha256'] == observation['profile_sha256']
            and row['gpu_execution'] is True and all(row[k] is False for k in FALSE), 'response identity/claims')


def validate(raw, request, directory, read):
    require(0 < len(raw) <= 65536, 'summary bound')
    o = parse(raw)
    keys(o, 'schema request child_pid registration_sha256 source_program_sha256 upload_manifest_sha256 '
            'bootstrap profile_sha256 setup_commands completed_forwards input_tokens observed_output_tokens '
            'page_permutation transcript_sha256 request_stream_bytes response_stream_bytes files close '
            'child_exit_zero process_group_absent native_closed gpu_execution numerical_acceptance '
            'full_model_acceptance performance_claim production_authority full_long_workload native_attempts retries')
    require(o['schema'] == 'FerricFiniteGuardedMlpReusableAr4ObservationV1' and o['request'] == request
            and request['schema'] == 'FerricFiniteGuardedMlpReusableAr4RequestV1'
            and uint(o['child_pid'], 0xffffffff) > 0 and uint(o['setup_commands']) > 0
            and uint(o['completed_forwards']) == 4 and uint(o['native_attempts']) == 1 and uint(o['retries']) == 0
            and all(o[k] is True for k in ('child_exit_zero', 'process_group_absent', 'native_closed', 'gpu_execution'))
            and all(o[k] is False for k in (*FALSE, 'full_long_workload')), 'closed four-forward observation')
    b = o['bootstrap']; d = b['decode']; config = request['decode']
    require(b['schema'] == 'FerricGuardedMlpReusableAr4BootstrapV1'
            and d['scope']['child_identity'] == o['child_pid']
            and d['scope']['bundle_id'] == config['expected_bundle_id']
            and d['scope']['model_id'] == config['expected_model_id']
            and d['scope']['session'] == config['session']
            and d['device_ids'] == config['device_ids'] == IDS
            and d['mode'] == config['mode'] and d['timeout_ms'] == config['dispatch_timeout_ms']
            and d['registration'] == o['registration_sha256']
            and d['begin']['source_program']['sha256'] == o['source_program_sha256']
            and d['begin']['uploads']['sha256'] == o['upload_manifest_sha256'], 'bootstrap actual custody joins')
    for supplied, expected in [(b['projection_image'], request['projection_image']),
                               (b['guarded_image'], request['guarded_image']),
                               (d['prefix_image'], config['prefix_image']), (d['tiles_image'], config['tiles_image'])]:
        require(supplied == {k: expected[k] for k in ('bytes', 'sha256')}, 'bootstrap selected image')
    for name in ('prefix', 'mlp', 'residual', 'tail'):
        expected = config['images'][name]
        require(d['begin'][name + '_image'] == {k: expected[k] for k in ('bytes', 'sha256')}, 'setup image')
    require(profile(b) == octets(o['profile_sha256']), 'profile digest')
    pages = o['page_permutation']
    require(type(pages) is list and len(pages) == 144 and all(type(v) is int for v in pages)
            and sorted(pages) == list(range(144)), 'page permutation')
    files = o['files']; keys(files, 'frames child_stderr bytes_before_summary summary_bytes total_bytes')
    require(type(files['frames']) is list and len(files['frames']) == 4, 'four frames')
    require(type(o['input_tokens']) is list and type(o['observed_output_tokens']) is list
            and len(o['input_tokens']) == len(o['observed_output_tokens']) == 4, 'trajectory extent')
    for value in o['input_tokens'] + o['observed_output_tokens']:
        uint(value, 151935)
    chain = hashlib.sha256(b'ferric-prefix284-combined2208-four-transcript-v1\0'
                           + octets(o['registration_sha256']) + octets(o['profile_sha256'])).digest()
    total = 0; previous = [(0, 0), (0, 0)]; frontiers = []; outputs = []; guards = []
    def body(value, name, maximum):
        nonlocal total
        pin = rust_pin(value)
        require(pin['path'] == str(directory / name) and pin['bytes'] <= maximum, 'closed evidence body path/extent')
        data = read(pin)
        require(len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], 'body hash')
        total += len(data)
        return data
    for position, frame in enumerate(files['frames']):
        keys(frame, 'response control observation request')
        response = frame['response']; ident = position + 1; envelope(response, o, ident)
        require(response['native_closed'] is False, 'premature Close')
        c = response['event']; keys(c, 'status generation position input_token output_token control observation capture chain')
        require(c['status'] == 'completed' and uint(c['generation']) == ident
                and uint(c['position']) == position, 'forward order')
        control_raw = body(frame['control'], f'control-{position}.bin', CONTROL_BYTES)
        data = body(frame['observation'], f'observation-{position}.bin', PAYLOAD_BYTES)
        request_raw = body(frame['request'], f'request-{position}.json', 16384)
        r = parse(request_raw); keys(r, 'protocol id device_ids session registration profile_sha256 command')
        uint(r['protocol']); uint(r['id'])
        for name in ('protocol', 'id', 'device_ids', 'session', 'registration', 'profile_sha256'):
            require(r[name] == response[name], 'request/response identity')
        command = r['command']; keys(command, 'op generation token cache_metadata rotary_bits')
        uint(command['token'], 151935)
        require(type(command['cache_metadata']) is list and all(type(v) is int for v in command['cache_metadata']),
                'integer cache metadata')
        require(command['op'] == 'forward' and uint(command['generation']) == ident
                and command['cache_metadata'] == [position] + pages
                and len(command['rotary_bits']) == 128
                and all(uint(v, 0xffffffff) & 0x7f800000 != 0x7f800000 for v in command['rotary_bits']),
                'forward metadata')
        expected = SEED[position] if config['mode'] == 'teacher_forced' else (SEED[0] if not outputs else outputs[-1])
        require(uint(c['input_token'], 151935) == expected == command['token'] == o['input_tokens'][position],
                'TF/own-output AR input')
        previous, layer_guards = control(control_raw, ident, previous)
        winner, capture = payload(data)
        require(uint(c['output_token'], 151935) == winner == o['observed_output_tokens'][position]
                and c['capture'] == capture and c['control'] == part(control_raw)
                and c['observation'] == capture['total'], 'raw output/capture/argmax')
        encoded = struct.pack('<QIIII', ident, position, expected, winner, CONTROL_BYTES)
        chain = hashlib.sha256(chain + encoded + hashlib.sha256(control_raw).digest()
                               + struct.pack('<I', PAYLOAD_BYTES) + hashlib.sha256(data).digest()).digest()
        require(octets(c['chain']) == chain, 'transcript chain')
        outputs.append(winner); frontiers.append(previous); guards.append(layer_guards)
    census = arena_census(body(files['child_stderr'], 'child-stderr.bin', 4096), b)
    require(uint(files['bytes_before_summary']) == total and uint(files['summary_bytes']) == len(raw)
            and uint(files['total_bytes']) == total + len(raw) <= 8 << 20, 'exact parent evidence census')
    close = o['close']; envelope(close, o, 5)
    require(close['native_closed'] is True and close['event'] == dict(status='closed', completed_forwards=4,
            transcript_sha256=list(chain)) and octets(o['transcript_sha256']) == chain, 'healthy Close transcript')
    return dict(child_pid=o['child_pid'], mode=config['mode'], input_tokens=o['input_tokens'], output_tokens=outputs,
                local_bank_generations=[1, 1, 2, 2], layer_observations=144, rank_guard_observations=288,
                final_queue_frontiers=frontiers, guards=guards, retained_parent_files=14,
                captured_bf16_words=4 * 303488, actual_lowest_index_argmax_checked=True,
                actual_arena_census=census, actual_arena_plateau_verified=True,
                independent_full_model_reference=False, numerical_acceptance=False,
                full_model_acceptance=False, performance_claim=False, production_authority=False)
