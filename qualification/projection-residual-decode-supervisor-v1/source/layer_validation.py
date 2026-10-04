"""Closed layer0 evidence checks, not an independent arithmetic/model verifier."""
import hashlib
import json
import struct

CAPTURE_BYTES = 9670656
LIMIT = 64 << 20
PROFILES = ('baseline22_mlp548', 'prefix284_mlp548')
STAGES = (('norm', 8192, 2), ('qkv', 6144, 2), ('query', 4096, 2),
    ('key-cache', 2359296, 2), ('value-cache', 2359296, 2), ('attention', 4096, 2),
    ('output-partial', 16384, 4), ('first-residual', 8192, 2), ('mlp-norm', 8192, 2),
    ('gate', 12288, 2), ('up', 12288, 2), ('activation', 12288, 2),
    ('down-partial', 16384, 4), ('final-hidden', 8192, 2))
LABELS = ('baseline', 'candidate')
BODY = {'request.json', 'parity.json'} | {label + '-' + name for label in LABELS for name in (
    'registration.json', 'program.json', 'uploads.json', 'bootstrap.json', 'request-1.json',
    'response-1.json', 'request-2.json', 'response-2.json', 'capture.bin', 'stderr.bin')}
REG_PROFILE = 'qwen3-8b-tp2-finite-prefix-v5-mlp-v1-two-forward-context2304-v1'
REG_KEYS = ('profile bundle_id model_id session pool_identity group_id child_identity layers globals '
            'auxiliary scratch pending_buffers state_slots source_program_bytes source_program_sha256')
SCOPE = 'bundle_id model_id session pool_identity group_id child_identity'
FALSE = ('numerical_acceptance', 'performance_claim', 'production_authority')


def require(ok, message):
    if not ok: raise RuntimeError(message)


def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed fields: ' + names)


def uint(value, maximum=(1 << 64) - 1):
    require(type(value) is int and 0 <= value <= maximum, 'unsigned integer')
    return value


def array(value, count, maximum):
    require(type(value) is list and len(value) == count, 'exact array extent')
    for word in value: uint(word, maximum)
    return value


def digest(value):
    array(value, 32, 255); return bytes(value)


def sha(raw):
    return hashlib.sha256(raw).digest()


def parse(raw):
    def pairs(rows):
        result = {}
        for k, v in rows:
            require(k not in result, 'duplicate JSON key'); result[k] = v
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON value'))


def part(raw):
    return dict(bytes=len(raw), sha256=list(sha(raw)))


def finite(raw, width):
    require(type(raw) is bytes and len(raw) % width == 0, 'finite scalar byte extent')
    mask, code = (0x7f80, '<H') if width == 2 else (0x7f800000, '<I')
    require(all(word & mask != mask for (word,) in struct.iter_unpack(code, raw)), 'finite captures')


def control(value, profile):
    keys(value, 'prefix mlp embedding_ns paired_ns')
    array(value['embedding_ns'], 2, (1 << 64) - 1)
    require(type(value['paired_ns']) is list and len(value['paired_ns']) == 4, 'four paired timings')
    for row in value['paired_ns']: array(row, 2, (1 << 64) - 1)
    for kind in ('prefix', 'mlp'):
        require(type(value[kind]) is list and len(value[kind]) == 2, 'two ranked terminal states')
        for state in value[kind]:
            if kind == 'prefix' and profile == PROFILES[0]:
                array(state, 22, 0xffffffff)
                require(state[:4] == [1, 0, 65535, 65535] and state[5] == 0
                    and state[6:] == [64] * 16
                    and all((state[4] >> (2 * task)) & 3 in (1, 2) for task in range(16)),
                    'exact Prefix22 terminal words')
                continue
            prefix = kind == 'prefix'
            count, start, end = (284, 24, 154) if prefix else (548, 32, 290)
            array(state, count, 0xffffffff)
            for index, actual in enumerate(state):
                if start <= index < end:
                    require(1 <= actual <= 64, 'dynamic owner word'); continue
                if index == 0: expected = 1
                elif index in (1, 2): expected = 0
                elif index == 3: expected = 31
                elif 4 <= index < 14:
                    expected = ([1, 48, 1, 16, 64] if prefix else [1, 96, 96, 1, 64])[(index - 4) % 5]
                elif prefix and index < 24:
                    expected = [0xffffffff] * 4 + [3]; expected = expected[(index - 14) % 5]
                elif not prefix and index < 32: expected = 3 if index in (22, 31) else 0xffffffff
                else: expected = 64
                require(actual == expected, 'every fixed terminal word')


def input_value(value):
    keys(value, 'generation token cache_metadata rotary_bits')
    require(uint(value['generation']) == 1 and uint(value['token'], 151935) == 9112,
            'authentic first token/generation')
    metadata = array(value['cache_metadata'], 145, 0xffffffff)
    require(metadata[0] == 0 and sorted(metadata[1:]) == list(range(144)), 'first position/full bank permutation')
    array(value['rotary_bits'], 128, 0xffffffff)
    require(value['rotary_bits'] == [0x3f800000] * 64 + [0] * 64, 'position-zero exact rotary')


def profile_digest(b):
    registration = digest(b['begin']['registration']['sha256']); i = b['input']; s = b['begin']['scope']
    h = hashlib.sha256(b'ferric-prefix-layer-input-v6\0' + registration)
    h.update(struct.pack('<QI', i['generation'], i['token']))
    for word in i['cache_metadata'] + i['rotary_bits']: h.update(struct.pack('<I', word))
    p = hashlib.sha256(b'ferric-prefix-layer-closed-v6\0')
    for name in ('bundle_id', 'model_id', 'session'): p.update(digest(s[name]))
    p.update(struct.pack('<QQI', s['pool_identity'], s['group_id'], s['child_identity']))
    p.update(registration); p.update(bytes([b['prefix_image'] is not None]))
    p.update(digest(b['prefix_image']['sha256']) if b['prefix_image'] else bytes(32))
    p.update(digest(b['mlp_image']['sha256'])); p.update(h.digest())
    p.update(struct.pack('<IQQ', b['timeout_ms'], *b['device_ids']))
    return list(p.digest())


def source(raw_registration, raw_program, raw_uploads, b, pid, request):
    r, p, uploads = map(parse, (raw_registration, raw_program, raw_uploads))
    keys(r, REG_KEYS); keys(p, 'schema profile group_id token_buffer result_buffer metadata steps')
    require(r['profile'] == p['profile'] == REG_PROFILE and r['child_identity'] == pid
        and uint(r['pool_identity']) > 0 and uint(r['group_id']) >= 0
        and r['source_program_bytes'] == len(raw_program)
        and digest(r['source_program_sha256']) == sha(raw_program), 'own source program identity')
    for key, expected in (('bundle_id', request['expected_bundle_id']), ('model_id', request['expected_model_id']),
                          ('session', request['session'])):
        require(r[key] == expected and digest(r[key]) != bytes(32), 'own requested registration scope')
    require(b['begin']['scope'] == {name: r[name] for name in SCOPE.split()}, 'own bootstrap scope')
    for name, count in (('layers', 72), ('globals', 3), ('auxiliary', 28), ('scratch', 18),
                        ('pending_buffers', 150), ('state_slots', 288)):
        require(type(r[name]) is list and len(r[name]) == count, 'retained source roster')
    require(p['schema'] == 'ferric-finite-source-grammar-v1' and p['group_id'] == r['group_id']
        and type(p['steps']) is list and len(p['steps']) == 1013 and len(p['metadata']) == 2,
        'closed actual source snapshot')
    for m in p['metadata']: keys(m, 'positions page_table cos sin')
    for step in p['steps']:
        if step.get('kind') == 'rank':
            keys(step, 'kind rank dispatch'); require(uint(step['rank'], 1) <= 1, 'source rank')
            dispatches = [step['dispatch']]
        else:
            keys(step, 'kind rows group_id epoch layer model operation producers consumers')
            require(step['kind'] == 'collective' and step['rows'] == 1 and step['group_id'] == r['group_id']
                and step['model'] == 'target8b' and uint(step['layer'], 35) <= 35
                and step['operation'] in ('attention_output_sum', 'feed_forward_down_sum')
                and len(step['producers']) == len(step['consumers']) == 2, 'own collective scope')
            uint(step['epoch']); dispatches = step['producers'] + step['consumers']
        for d in dispatches:
            keys(d, 'symbol grid_workgroups workgroup_size arguments')
            require(type(d['symbol']) is str and type(d['arguments']) is list, 'ordered dispatch')
            uint(d['grid_workgroups'], 0xffffffff); uint(d['workgroup_size'], 0xffffffff)
            for a in d['arguments']:
                if a.get('kind') == 'buffer':
                    keys(a, 'kind source_id offset elements element_bytes access')
                    require(a['access'] in ('read', 'write', 'read_write'), 'source access')
                    for k in ('source_id', 'offset', 'elements', 'element_bytes'): uint(a[k])
                else:
                    keys(a, 'kind value'); require(a['kind'] in ('u32', 'f32_bits'), 'source literal kind')
                    uint(a['value'], 0xffffffff)
    # Only own-checked lifecycle fields are removed for equality, never for authority.
    logical_registration = {k: v for k, v in r.items() if k not in
        ('session', 'pool_identity', 'group_id', 'child_identity', 'source_program_bytes', 'source_program_sha256')}
    logical_program = dict(p); logical_program.pop('group_id')
    logical_program['steps'] = [{k: v for k, v in step.items() if k != 'group_id'} for step in p['steps']]
    return logical_registration, logical_program, uploads


def bootstrap(b, profile, request, files, label, pid):
    keys(b, 'protocol profile device_ids timeout_ms begin input mlp_image prefix_image')
    require(b['protocol'] == 1 and b['profile'] == profile and b['device_ids'] == request['device_ids']
        and b['timeout_ms'] == request['dispatch_timeout_ms'], 'exact bootstrap route/devices/timeout')
    input_value(b['input']); begin = b['begin']
    keys(begin, 'scope registration source_program uploads prefix_image mlp_image residual_image tail_image')
    keys(begin['scope'], SCOPE)
    for name, suffix in (('registration', 'registration'), ('source_program', 'program'), ('uploads', 'uploads')):
        require(begin[name] == part(files[label + '-' + suffix + '.json']), 'source part actual bytes')
    for name in ('prefix', 'mlp', 'residual', 'tail'):
        pin = request['images'][name]
        require(begin[name + '_image'] == dict(bytes=pin['bytes'], sha256=pin['sha256']), 'original setup images')
    require(b['mlp_image'] == {k: request['mlp_tiles_image'][k] for k in ('bytes', 'sha256')}, 'same MLP548 image')
    require(b['prefix_image'] == (None if profile == PROFILES[0] else
        {k: request['prefix_tiles_image'][k] for k in ('bytes', 'sha256')}), 'distinct prefix image presence')
    return source(files[label + '-registration.json'], files[label + '-program.json'],
                  files[label + '-uploads.json'], b, pid, request)


def capture(raw, input_record):
    require(type(raw) is bytes and len(raw) == CAPTURE_BYTES, 'exact both-rank capture extent')
    offset, rows = 0, []
    for rank in range(2):
        for name, size, width in STAGES:
            row = raw[offset:offset + size]; offset += size; finite(row, width)
            if name in ('key-cache', 'value-cache'):
                start = input_record['cache_metadata'][1] * 16 * 512 * 2
                require(not any(row[:start]) and not any(row[start + 1024:]), 'full KV unchanged outside position0 slot')
            rows.append(row)
    require(offset == len(raw), 'closed capture partition'); return rows


def validate(summary, files, request):
    require(type(files) is dict and set(files) == BODY and all(type(v) is bytes for v in files.values()),
            'exact22 retained body buffers')
    require(len(summary) <= 65536 and len(summary) + sum(map(len, files.values())) <= LIMIT, 'whole64MiB evidence')
    o = parse(summary)
    keys(o, 'schema request runs stages files bitwise_equal completed_layers_per_run native_closed '
            'gpu_execution numerical_acceptance performance_claim production_authority full_forward')
    require(o['schema'] == 'FerricFinitePrefixLayerComparisonObservationV1' and o['request'] == request
        and parse(files['request.json']) == request and o['completed_layers_per_run'] == 1
        and o['native_closed'] is True and o['gpu_execution'] is True and o['full_forward'] is False
        and all(o[k] is False for k in FALSE), 'closed layer-only observation scope')
    require(type(o['files']) is list and len(o['files']) == 22, 'exact file manifest extent')
    seen = set()
    for row in o['files']:
        keys(row, 'name bytes sha256'); name = row['name']
        require(name in files and name not in seen and row == dict(name=name, **part(files[name])), 'actual retained file digest')
        seen.add(name)
    require(len(o['runs']) == 2, 'two independently fresh routes')
    inputs, sources, rows, pids = [], [], [], []
    for index, (label, run) in enumerate(zip(LABELS, o['runs'])):
        keys(run, 'child_pid bootstrap profile_sha256 setup_commands close child_exit_zero process_group_absent')
        pid = uint(run['child_pid'], 0xffffffff); require(pid > 0 and pid not in pids, 'distinct actual child PID')
        pids.append(pid); require(run['child_exit_zero'] is True and run['process_group_absent'] is True
            and uint(run['setup_commands']) > 0, 'closed and reaped child assertion')
        b = parse(files[label + '-bootstrap.json']); require(b == run['bootstrap'], 'retained bootstrap exact')
        sources.append(bootstrap(b, PROFILES[index], request, files, label, pid)); inputs.append(b['input'])
        require(run['profile_sha256'] == profile_digest(b), 'actual profile hash')
        for number, command in ((1, 'run'), (2, 'close')):
            q = parse(files[f'{label}-request-{number}.json']); r = parse(files[f'{label}-response-{number}.json'])
            require(q == dict(protocol=1, id=number, profile_sha256=run['profile_sha256'], command=command),
                    'exact Run1/Close2 request')
            keys(r, 'protocol id profile_sha256 profile native_closed completed_layers control capture '
                    'gpu_execution numerical_acceptance performance_claim production_authority')
            require(r['protocol'] == 1 and r['id'] == number and r['profile_sha256'] == run['profile_sha256']
                and r['profile'] == PROFILES[index] and r['completed_layers'] == 1 and r['gpu_execution'] is True
                and all(r[k] is False for k in FALSE), 'exact response scope')
            if number == 1:
                require(r['native_closed'] is False and r['control'] is None and r['capture'] is None,
                        'Run does not release capture')
            else:
                require(r == run['close'] and r['native_closed'] is True
                    and r['capture'] == part(files[label + '-capture.bin']), 'Close-only actual capture')
                control(r['control'], PROFILES[index])
        rows.append(capture(files[label + '-capture.bin'], b['input']))
    require(inputs[0] == inputs[1] and sources[0] == sources[1], 'same logical source/input, exact epochs and ordered args')
    expected = []
    for i, (a, b) in enumerate(zip(*rows)):
        name, size, width = STAGES[i % 14]
        expected.append(dict(rank=i // 14, stage=name, elements=size // width, element_bytes=width,
            baseline_sha256=list(sha(a)), candidate_sha256=list(sha(b)),
            bit_mismatches=sum(a[n:n+width] != b[n:n+width] for n in range(0, size, width))))
    equal = all(row['bit_mismatches'] == 0 for row in expected)
    require(o['stages'] == parse(files['parity.json']) == expected and o['bitwise_equal'] is equal,
            'all28 freshly recomputed parity rows, no assertion-only match')
    return dict(bitwise_equal=equal, compared_arrays=28, capture_bytes_per_run=CAPTURE_BYTES,
        closed_child_pids=pids, stages=expected, layer=0, generation=1, position=0, token=9112,
        full_kv_checked=True, numerical_acceptance=False, full_model_correctness=False,
        performance_claim=False, production_authority=False)
