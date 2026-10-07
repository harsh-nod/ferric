"""Readiness40Position5 data checks; no independent full-model numerical authority."""
import hashlib
import json
import struct

CONTROL_BYTES = 242824
PAYLOAD_BYTES = 606976
LOGIT_BYTES = 303872
CAPTURES = (0, 5, 16, 39)
IDS = [16366993098680759275, 10838076764495710945]

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


def encoded(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()


def same(a, b):
    return json.dumps(a, sort_keys=True, separators=(',', ':'), allow_nan=False) == json.dumps(
        b, sort_keys=True, separators=(',', ':'), allow_nan=False)


def ordered(value, names):
    keys(value, names)
    return {name: value[name] for name in names.split()}


def checked_part(value, extent=None, maximum=32 << 20):
    result = ordered(value, 'bytes sha256')
    require(0 < uint(result['bytes'], maximum) and octets(result['sha256']) != bytes(32), 'nonempty part')
    require(extent is None or result['bytes'] == extent, 'exact part extent')
    return result


def scope(value):
    out = ordered(value, 'bundle_id model_id session pool_identity group_id child_identity')
    for name in ('bundle_id', 'model_id', 'session'):
        require(octets(out[name]) != bytes(32), 'nonzero scope digest')
    require(uint(out['pool_identity']) > 0 and uint(out['child_identity'], 0xffffffff) > 0, 'scope identity')
    uint(out['group_id'])
    return out


def bootstrap(value, request, pid, prompt):
    outer = ordered(value, 'schema sequence child_deadline_ms')
    require(outer['schema'] == 'FerricGuardedMlpReadiness40Position5BootstrapV1'
            and uint(outer['child_deadline_ms']) == request['base']['child_deadline_ms'] == 3600000,
            'readiness bootstrap/deadline')
    b = ordered(outer['sequence'], 'protocol profile device_ids scope registration begin timeout_ms '
                'prompt_tokens prefix_image mlp_image projection_image guarded_image')
    config = request['base']
    keys(config, 'schema source worker images expected_bundle_id expected_model_id device_ids session '
         'prompt evidence_directory dispatch_timeout_ms child_deadline_ms')
    require(config['schema'] == 'FerricFiniteLongRequestV1', 'closed base request schema')
    require(uint(b['protocol']) == 1 and b['profile'] == 'readiness40_position5'
            and same(b['device_ids'], IDS) and same(b['device_ids'], config['device_ids'])
            and uint(b['timeout_ms']) == config['dispatch_timeout_ms'] == 10000
            and same(b['prompt_tokens'], prompt) and len(prompt) == 2048, 'closed full-prompt readiness profile')
    for token in prompt:
        uint(token, 151935)
    b['scope'] = scope(b['scope'])
    s = b['scope']
    require(s['child_identity'] == pid and same(s['session'], config['session'])
            and same(s['model_id'], config['expected_model_id'])
            and same(s['bundle_id'], config['expected_bundle_id']), 'actual bootstrap scope')
    octets(b['registration'])
    begin = ordered(b['begin'], 'scope registration source_program uploads prefix_image mlp_image residual_image tail_image')
    begin['scope'] = scope(begin['scope'])
    require(same(begin['scope'], s), 'Begin scope')
    for name in ('registration', 'source_program', 'uploads'):
        begin[name] = checked_part(begin[name], maximum=4 << 20)
    require(same(begin['registration']['sha256'], b['registration']), 'registration hash')
    for name in ('prefix', 'mlp', 'residual', 'tail'):
        pin = config['images'][name]
        begin[name + '_image'] = checked_part(begin[name + '_image'])
        require(same(begin[name + '_image'], {k: pin[k] for k in ('bytes', 'sha256')}), 'setup image join')
    b['begin'] = begin
    for name, key in [('prefix_image', 'prefix_image'), ('mlp_image', 'tiles_image'),
                      ('projection_image', 'projection_image'), ('guarded_image', 'guarded_image')]:
        b[name] = checked_part(b[name])
        require(same(b[name], {k: request[key][k] for k in ('bytes', 'sha256')}), 'selected image join')
    parts = [begin[n] for n in ('registration', 'source_program', 'uploads', 'prefix_image',
                               'mlp_image', 'residual_image', 'tail_image')]
    parts += [b[n] for n in ('prefix_image', 'mlp_image', 'projection_image', 'guarded_image')]
    require(sum(p['bytes'] for p in parts) <= (64 << 20) - 2 * (65536 + 4) - 41 * (8192 + 4),
            'unchanged combined input bound')
    outer['sequence'] = b
    return outer, hashlib.sha256(b'ferric-prefix284-combined2208-closed-long-v2\0' + encoded(b)).digest()


def request_row(value, b, digest, position):
    r = ordered(value, 'protocol id device_ids session registration profile_sha256 command')
    require(uint(r['protocol']) == 1 and uint(r['id']) == position + 1
            and same(r['device_ids'], b['device_ids']) and same(r['session'], b['scope']['session'])
            and same(r['registration'], b['registration']) and octets(r['profile_sha256']) == digest,
            'request identity/order')
    if position == 40:
        r['command'] = ordered(r['command'], 'op')
        require(r['command']['op'] == 'close', 'exact Close command')
    else:
        c = ordered(r['command'], 'op generation token cache_metadata rotary_bits')
        require(c['op'] == 'forward' and uint(c['generation']) == position + 1
                and uint(c['token'], 151935) == b['prompt_tokens'][position], 'authentic prompt-only request')
        pages, rotary = c['cache_metadata'], c['rotary_bits']
        require(type(pages) is list and len(pages) == 145 and uint(pages[0]) == position
                and sorted(uint(v, 143) for v in pages[1:]) == list(range(144)), 'full144 page permutation')
        require(type(rotary) is list and len(rotary) == 128
                and all(uint(v, 0xffffffff) & 0x7f800000 != 0x7f800000 for v in rotary), 'finite128 rotary bits')
        r['command'] = c
    require(len(encoded(r)) <= 8192, 'request record bound')
    return r


def frontiers(value):
    require(type(value) is list and len(value) == 2, 'two frontier ranks')
    for row in value:
        require(type(row) is list and len(row) == 2, 'write/read pair')
        require(uint(row[0]) > 0 and uint(row[1]) <= row[0], 'frontier bounds')
    return value


def completion(value, position):
    c = ordered(value, 'generation position input_token output_token bank control observation logits captured '
                'first_frontiers final_frontiers chain')
    require(uint(c['generation']) == position + 1 and uint(c['position']) == position,
            'completion position/generation')
    uint(c['input_token'], 151935); uint(c['output_token'], 151935)
    bank = ordered(c['bank'], 'bank local_generation retired_forward logical_page page_offset')
    expected = dict(bank=position % 2, local_generation=position // 2 + 1,
                    retired_forward=position - 1 if position >= 2 else None,
                    logical_page=position // 16, page_offset=position % 16)
    require(same(bank, expected), 'bank retirement/page step')
    c['bank'] = bank
    for name, extent in [('control', CONTROL_BYTES), ('observation', PAYLOAD_BYTES), ('logits', LOGIT_BYTES)]:
        c[name] = checked_part(c[name], extent)
    require(c['captured'] is (position in CAPTURES), 'fixed four-capture schedule')
    first, last = frontiers(c['first_frontiers']), frontiers(c['final_frontiers'])
    require(all(last[r][0] > first[r][0] and last[r][1] >= first[r][1] for r in range(2)), 'forward frontier summary')
    octets(c['chain'])
    return c


def control(raw, generation, previous):
    require(len(raw) == CONTROL_BYTES and 1 <= uint(generation, 40), 'readiness control extent/generation')
    offset = 16; current = [list(row) for row in previous]; first = None
    for layer in range(36):
        for _ in range(2):
            prefix(struct.unpack_from('<284I', raw, offset)); offset += 1136
        for _ in range(2):
            mlp(struct.unpack_from('<548I', raw, offset)); offset += 2192
        for _ in range(2):
            require(struct.unpack_from('<4I', raw, offset) == ((generation - 1) // 2 + 1, 0, 1, 0),
                    'actual local guard generation/valid/error'); offset += 16
        offset += 24
        for rank in range(2):
            write, read = struct.unpack_from('<QQ', raw, offset); offset += 16
            require(read <= write and write > current[rank][0] and read >= current[rank][1],
                    'actual queue frontiers')
            current[rank] = [write, read]
        if layer == 0:
            first = [row[:] for row in current]
    require(offset + 24 == len(raw), 'control trailing bytes')
    return first, current


def validate(raw, request, directory, read, prompt):
    require(type(raw) is bytes and 0 < len(raw) <= 128 << 10, 'bounded original parent summary')
    o = parse(raw)
    keys(o, 'schema request child_pid bootstrap profile_sha256 registration_sha256 source_program_sha256 '
         'upload_manifest_sha256 setup_commands completed_forwards prompt_positions_executed generated_tokens '
         'page_permutation transcript_sha256 request_stream_bytes response_stream_bytes files close '
         'child_exit_zero process_group_absent native_closed gpu_execution full_long_workload '
         'numerical_acceptance performance_claim production_authority')
    require(o['schema'] == 'FerricGuardedMlpReadiness40Position5ObservationV1' and same(o['request'], request)
            and uint(o['child_pid'], 0x7fffffff) > 0 and uint(o['setup_commands']) > 0
            and uint(o['completed_forwards']) == uint(o['prompt_positions_executed']) == 40
            and o['generated_tokens'] == []
            and all(o[k] is True for k in ('child_exit_zero', 'process_group_absent', 'native_closed', 'gpu_execution'))
            and all(o[k] is False for k in ('full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')),
            'closed readiness40 observation/no generated tokens')
    keys(request, 'schema base tiles_image prefix_image projection_image guarded_image')
    require(request['schema'] == 'FerricGuardedMlpReadiness40Position5RequestV1', 'readiness request schema')
    outer, digest = bootstrap(o['bootstrap'], request, o['child_pid'], prompt)
    b = outer['sequence']; begin = b['begin']
    require(octets(o['profile_sha256']) == digest and same(o['registration_sha256'], b['registration'])
            and same(o['source_program_sha256'], begin['source_program']['sha256'])
            and same(o['upload_manifest_sha256'], begin['uploads']['sha256']), 'profile/setup joins')
    pages = o['page_permutation']
    require(type(pages) is list and len(pages) == 144
            and sorted(uint(v, 143) for v in pages) == list(range(144)), 'retained full page permutation')
    f = o['files']; keys(f, 'frames captures child_stderr rows bytes_before_summary summary_bytes total_bytes supervisor_metadata_allowance')
    require(uint(f['rows']) == 40 and type(f['captures']) is list and len(f['captures']) == 4,
            'closed retained file roster')
    total = 0
    def body(value, name, maximum):
        nonlocal total
        pin = rust_pin(value)
        require(pin['path'] == str(directory / name) and pin['bytes'] <= maximum, 'exact ordinary retained path/extent')
        data = read(pin)
        require(type(data) is bytes and len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], 'retained body hash')
        total += len(data)
        return data
    framed = body(f['frames'], 'frames.ndjson', 40 * 8193)
    require(framed.endswith(b'\n') and len(framed.splitlines()) == 40, 'forty complete canonical records')
    capture_bodies = {}
    for position, row in zip(CAPTURES, f['captures']):
        keys(row, 'position file')
        require(uint(row['position']) == position, 'ordered selected capture positions')
        capture_bodies[position] = body(row['file'], 'capture-%d.bin' % position, CONTROL_BYTES + PAYLOAD_BYTES)
    stderr = body(f['child_stderr'], 'child-stderr.bin', 2 << 20)
    require(stderr == b'', 'unrequested observer/capture/diagnostic child stderr')
    chain = hashlib.sha256(b'ferric-guarded-mlp-long-transcript-v2\0' + digest).digest()
    previous = [[0, 0], [0, 0]]; outputs = []; selected = []; request_bytes = 0; response_bytes = 0
    for position, line in enumerate(framed.splitlines()):
        require(0 < len(line) <= 8192, 'frame record bound')
        frame = ordered(parse(line), 'schema profile request completion')
        require(frame['schema'] == 'FerricGuardedMlpLongResponseV2' and frame['profile'] == 'readiness40_position5', 'frame profile')
        r = request_row(frame['request'], b, digest, position); c = completion(frame['completion'], position)
        require(same(r['command']['cache_metadata'], [position] + pages)
                and c['input_token'] == r['command']['token'], 'stable KV/input token history')
        require(all(c['first_frontiers'][i][0] > previous[i][0]
                    and c['first_frontiers'][i][1] >= previous[i][1] for i in range(2)), 'cross-forward queue ordering')
        if position in CAPTURES:
            data = capture_bodies[position]
            require(len(data) == CONTROL_BYTES + PAYLOAD_BYTES, 'selected capture exact extent')
            controls, payload_raw = data[:CONTROL_BYTES], data[CONTROL_BYTES:]
            first, last = control(controls, position + 1, previous)
            winner, parts = payload(payload_raw)
            require(same(part(controls), c['control']) and same(parts['total'], c['observation'])
                    and same(parts['logits'], c['logits']) and winner == c['output_token']
                    and same(first, c['first_frontiers']) and same(last, c['final_frontiers']), 'selected original bodies/completion')
            selected.append(dict(position=position, output_token=winner, local_generation=position // 2 + 1,
                                 capture=part(data), layer_hidden=parts['layer_hidden'], logits=parts['logits']))
        saved = c['chain']; c['chain'] = [0] * 32
        chain = hashlib.sha256(chain + encoded(r) + encoded(c)).digest()
        require(octets(saved) == chain, 'full40 chained original completion')
        c['chain'] = saved; frame['request'] = r; frame['completion'] = c
        require(encoded(frame) == line, 'canonical Rust field order/frame bytes')
        request_bytes += 4 + len(encoded(r)); response_bytes += 4 + len(line)
        if position in CAPTURES:
            response_bytes += CONTROL_BYTES + PAYLOAD_BYTES
        previous = c['final_frontiers']; outputs.append(c['output_token'])
    closed = ordered(o['close'], 'schema request completed_forwards generated_tokens transcript_sha256 native_closed '
                     'numerical_acceptance performance_claim production_authority')
    closed['request'] = request_row(closed['request'], b, digest, 40)
    require(closed['schema'] == 'FerricGuardedMlpReadiness40Position5ClosedV1' and uint(closed['completed_forwards']) == 40
            and closed['generated_tokens'] == [] and closed['native_closed'] is True
            and all(closed[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority'))
            and octets(closed['transcript_sha256']) == octets(o['transcript_sha256']) == chain, 'exact healthy Close')
    setup = dict(protocol=1, id=1, device_ids=b['device_ids'], session=b['scope']['session'],
                 command=dict(op='begin', **begin))
    request_bytes += 4 + len(encoded(outer)) + sum(b[n]['bytes'] for n in ('prefix_image', 'mlp_image', 'projection_image', 'guarded_image'))
    request_bytes += 4 + len(encoded(setup)) + sum(begin[n]['bytes'] for n in ('registration', 'source_program', 'uploads',
                                      'prefix_image', 'mlp_image', 'residual_image', 'tail_image'))
    request_bytes += 4 + len(encoded(closed['request'])); response_bytes += 4 + len(encoded(closed))
    require(uint(o['request_stream_bytes'], 64 << 20) == request_bytes
            and uint(o['response_stream_bytes'], 64 << 20) == response_bytes, 'exact separate readiness framing budgets')
    require(uint(f['bytes_before_summary']) == total and uint(f['summary_bytes'], 128 << 10) == len(raw)
            and uint(f['total_bytes']) == total + len(raw) and uint(f['supervisor_metadata_allowance']) == 512 << 10
            and f['total_bytes'] + f['supervisor_metadata_allowance'] <= 32 << 20, 'bounded exact compact retention')
    return dict(schema='ferric-guarded-mlp-readiness40-position5-data-observation-v1', child_pid=o['child_pid'],
                completed_forwards=40, prompt_positions_executed=40, generated_tokens=[], captures=selected,
                observed_argmax_tokens=outputs, final_queue_frontiers=previous, profile_sha256=digest.hex(),
                transcript_sha256=chain.hex(), all40_transcript_checked=True, selected_payloads_independently_checked=4,
                unselected_payloads_independently_checked=False, full_long_workload=False,
                numerical_acceptance=False, performance_claim=False, production_authority=False)
