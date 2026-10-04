"""Pure candidate TF4 records; no source, execution or numerical authority."""
import hashlib
import json
import struct
from stage_core import keys, parse, require, uint, words, number, sha, OBSERVATION_BYTES
from smoke_validation import absolute, identity, normalized_pin, part, response as base_response, MODEL_ID, BUNDLE_ID, PROMPT_PINS

TOKENS = [9112, 2190, 3772, 220]
CONTROL_BYTES = 241960
SUMMARY_LIMIT = 64 << 10
OWN_LIMIT = 8 << 20
FILES = {'child-stderr.bin'} | {f'{kind}-{position}.{suffix}' for position in range(4)
    for kind, suffix in [('request', 'json'), ('control', 'bin'), ('observation', 'bin')]}


def canonical(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()


def base_profile(bootstrap, request, child_pid, registration):
    keys(bootstrap, 'protocol profile device_ids scope registration begin timeout_ms mode input_tokens tiles_image prefix_image')
    scope = bootstrap['scope']
    keys(scope, 'bundle_id model_id session pool_identity group_id child_identity')
    require(uint(bootstrap['protocol']) == 1 and bootstrap['profile'] == 'prefix284_mlp548_four_forward_v1'
        and bootstrap['mode'] == request['mode'] in ('teacher_forced', 'autoregressive')
        and type(bootstrap['input_tokens']) is list
        and [uint(value, 151935) for value in bootstrap['input_tokens']] ==
            (TOKENS if request['mode'] == 'teacher_forced' else TOKENS[:1]),
        'closed four-forward profile, never old22 or long route')
    require(type(bootstrap['device_ids']) is list
        and [uint(value) for value in bootstrap['device_ids']] == request['device_ids']
        and uint(bootstrap['timeout_ms'], 10000) == request['dispatch_timeout_ms']
        and identity(bootstrap['registration']) == registration, 'profile request and source registration')
    require(identity(scope['bundle_id']) == identity(request['expected_bundle_id'])
        and identity(scope['model_id']) == identity(request['expected_model_id'])
        and identity(scope['session']) == identity(request['session'])
        and identity(scope['session']) != bytes(32) and registration != bytes(32)
        and uint(scope['child_identity'], (1 << 32) - 1) == child_pid
        and uint(scope['pool_identity']) > 0, 'profile original scope')
    group = uint(scope['group_id'])
    begin = bootstrap['begin']
    keys(begin, 'scope registration source_program uploads prefix_image mlp_image residual_image tail_image')
    require(begin['scope'] == scope and identity(begin['registration']['sha256']) == registration,
            'original Begin/registration scope')
    parts = []
    for name in ('registration', 'source_program', 'uploads'):
        item = begin[name]; keys(item, 'bytes sha256')
        require(0 < uint(item['bytes'], 4 << 20) and identity(item['sha256']) != bytes(32), 'Begin metadata part')
        parts.append(item)
    for name in ('prefix', 'mlp', 'residual', 'tail'):
        item = begin[name + '_image']; keys(item, 'bytes sha256')
        image = normalized_pin(request['images'][name], 64 << 20)
        require(item['bytes'] == image['bytes'] and identity(item['sha256']).hex() == image['sha256'],
                'original Begin image binding')
        parts.append(item)
    for name in ('tiles_image', 'prefix_image'):
        item = bootstrap[name]; keys(item, 'bytes sha256')
        image = normalized_pin(request[name], 32 << 20)
        require(uint(item['bytes'], 32 << 20) == image['bytes']
            and identity(item['sha256']).hex() == image['sha256'], 'actual distinct profile image')
        parts.append(item)
    require(all(item['bytes'] > 0 and identity(item['sha256']) != bytes(32) for item in parts)
        and sum(item['bytes'] for item in parts) <= (64 << 20) - 7 * ((64 << 10) + 4),
            'original Begin7 and both images share one input envelope')
    material = b'ferric-prefix284-mlp548-four-decode-v6\0'
    for name in ('bundle_id', 'model_id', 'session'):
        material += identity(scope[name])
    material += struct.pack('<QQI', scope['pool_identity'], group, child_pid)
    material += registration + identity(bootstrap['prefix_image']['sha256']) + identity(bootstrap['tiles_image']['sha256'])
    inputs = TOKENS if request['mode'] == 'teacher_forced' else [TOKENS[0], 0, 0, 0]
    material += struct.pack('<IBQQ4I', bootstrap['timeout_ms'], int(request['mode'] == 'autoregressive'),
                            *request['device_ids'], *inputs)
    return hashlib.sha256(material).digest()


BODY = FILES
REQUEST_SCHEMA = 'FerricFiniteProjectionResidualDecodeRequestV1'
BOOTSTRAP_SCHEMA = 'FerricProjectionResidualDecodeBootstrapV1'
OBSERVATION_SCHEMA = 'FerricFiniteProjectionResidualDecodeObservationV1'
ARITHMETIC = b'ordered-fp32-tp2-bf16-projection-then-bf16-residual-v1'


def profile(bootstrap, outer, child_pid, registration):
    keys(bootstrap, 'schema decode projection_residual_image')
    require(bootstrap['schema'] == BOOTSTRAP_SCHEMA and outer['decode']['mode'] == 'teacher_forced',
            'separate candidate teacher-forced bootstrap')
    base = base_profile(bootstrap['decode'], outer['decode'], child_pid, registration)
    candidate = normalized_pin(outer['projection_residual_image'], 32 << 20)
    selected = bootstrap['projection_residual_image']
    keys(selected, 'bytes sha256')
    require(0 < uint(selected['bytes'], 32 << 20) == candidate['bytes']
        and identity(selected['sha256']).hex() == candidate['sha256']
        and candidate['sha256'] != normalized_pin(outer['decode']['images']['residual'], 64 << 20)['sha256'],
        'additional actual candidate image distinct from original copy image')
    b = bootstrap['decode']; begin = b['begin']
    parts = [begin[name] for name in ('registration', 'source_program', 'uploads',
        'prefix_image', 'mlp_image', 'residual_image', 'tail_image')]
    parts += [b['tiles_image'], b['prefix_image'], selected]
    require(sum(uint(p['bytes'], 64 << 20) for p in parts) <= (64 << 20) - 7 * ((64 << 10) + 4),
            'additional image stays inside original aggregate stream bound')
    return hashlib.sha256(b'ferric-projection-residual-four-decode-v1\0' +
                          ARITHMETIC + base + identity(selected['sha256'])).digest()


def response(value, observed, position, closed=False):
    # Pass only the envelope fields the unchanged response checker consumes.
    scope = dict(request=observed['request']['decode'],
        registration_sha256=observed['registration_sha256'],
        profile_sha256=observed['profile_sha256'])
    base_response(value, scope, position, closed)


def child_marker(raw, pid):
    require(type(raw) is bytes and 0 < uint(pid, (1 << 32) - 1), 'actual child marker input')
    expected = (f'finite engineering owned child pid={pid} pgid={pid}; no native setup acknowledged\n'
        'finite explicit profile=projection-residual-prefix284-mlp548-four-forward-v1 mode=TeacherForced\n')
    expected += ''.join(f'finite prefix decode completed position={p} forwards={p + 1}\n' for p in range(4))
    require(raw == expected.encode(), 'one exact worker marker and all four completed positions')
    return pid


def control(raw):
    require(type(raw) is bytes and len(raw) == CONTROL_BYTES, 'distinct all36 typed284+548 Control extent')
    offset = 16
    for _layer in range(36):
        for _rank in range(2):
            value = struct.unpack_from('<284I', raw, offset)
            offset += 1136
            require(value[:4] == (1, 0, 0, 31)
                and value[4:9] == value[9:14] == (1, 48, 1, 16, 64)
                and value[14:19] == value[19:24] == ((1 << 32) - 1,) * 4 + (3,)
                and all(1 <= owner <= 64 for owner in value[24:154])
                and value[154:] == (64,) * 130, 'all284 prefix terminal words')
        for _rank in range(2):
            value = struct.unpack_from('<548I', raw, offset)
            offset += 2192
            require(value[:4] == (1, 0, 0, 31)
                and value[4:9] == value[9:14] == (1, 96, 96, 1, 64)
                and value[14:23] == value[23:32] == ((1 << 32) - 1,) * 8 + (3,)
                and all(1 <= owner <= 64 for owner in value[32:290])
                and value[290:] == (64,) * 258, 'all548 tiled terminal words')
        offset += 64
    require(offset + 24 == len(raw), 'typed state/timing Control census')
    return 144


def validate(summary_raw: bytes, files: dict[str, bytes], expected_request):
    """Check retained records only. External process/source/image custody is separate."""
    require(type(summary_raw) is bytes and 0 < len(summary_raw) <= SUMMARY_LIMIT, 'summary bound')
    require(type(files) is dict and set(files) == FILES
        and all(type(value) is bytes for value in files.values()), 'closed13 body-file census')
    observed = parse(summary_raw)
    keys(observed, 'schema request child_pid registration_sha256 source_program_sha256 upload_manifest_sha256 '
        'bootstrap profile_sha256 setup_commands completed_forwards input_tokens observed_output_tokens page_permutation '
        'transcript_sha256 request_stream_bytes response_stream_bytes files close child_exit_zero process_group_absent '
        'native_closed gpu_execution numerical_acceptance performance_claim production_authority full_long_workload '
        'paired_comparison_performed native_attempts retries')
    require(observed['schema'] == OBSERVATION_SCHEMA and uint(observed['completed_forwards']) == 4
        and type(observed['input_tokens']) is list and len(observed['input_tokens']) == 4
        and all(uint(v, 151935) >= 0 for v in observed['input_tokens']),
        'exact all36 four-forward complete scope')
    require(all(observed[name] is True for name in ('child_exit_zero', 'process_group_absent', 'native_closed', 'gpu_execution'))
        and all(observed[name] is False for name in ('numerical_acceptance', 'performance_claim', 'production_authority',
                                                  'full_long_workload')), 'recorded lifecycle and claim boundary')
    child_pid = uint(observed['child_pid'], (1 << 32) - 1)
    require(child_pid > 0, 'child PID')
    outer = observed['request']
    keys(outer, 'schema decode projection_residual_image')
    require(outer == expected_request and outer['schema'] == REQUEST_SCHEMA
        and len(canonical(outer)) <= 16384 and observed['paired_comparison_performed'] is False
        and uint(observed['native_attempts']) == 1 and uint(observed['retries']) == 0,
        'actual closed candidate request, one attempt and no parity authority')
    request = outer['decode']
    keys(request, 'schema source worker images expected_bundle_id expected_model_id device_ids session prompt mode tiles_image prefix_image '
        'evidence_directory dispatch_timeout_ms child_deadline_ms')
    require(request['schema'] == 'FerricFinitePrefixDecodeRequestV1' and request['mode'] == 'teacher_forced'
        and identity(request['expected_model_id']).hex() == MODEL_ID
        and identity(request['expected_bundle_id']).hex() == BUNDLE_ID, 'actual original model and four-forward request')
    require(len(canonical(request)) <= 16384 and len(canonical(request['evidence_directory'])) <= 512,
        'request/path bounds')
    absolute(request['source'])
    directory = absolute(request['evidence_directory'])
    require(type(request['device_ids']) is list and len(request['device_ids']) == 2
        and all(uint(v) > 0 for v in request['device_ids']) and len(set(request['device_ids'])) == 2, 'TP2 distinct devices')
    identity(request['session'])
    require(1 <= uint(request['dispatch_timeout_ms']) <= 10000
        and 1000 <= uint(request['child_deadline_ms']) <= 3600000, 'unchanged request timeouts')
    normalized_pin(request['worker'], 512 << 20)
    keys(request['images'], 'prefix mlp residual tail')
    for image in request['images'].values():
        normalized_pin(image, 64 << 20)
    normalized_pin(request['tiles_image'], 32 << 20)
    normalized_pin(request['prefix_image'], 32 << 20)
    keys(request['prompt'], 'manifest text tokens')
    for name, expected in PROMPT_PINS.items():
        pin = normalized_pin(request['prompt'][name], 32 << 10)
        require((pin['bytes'], pin['sha256']) == expected, 'authentic retained prompt provenance')
    registration = identity(observed['registration_sha256'])
    for name in ('source_program_sha256', 'upload_manifest_sha256', 'transcript_sha256'):
        identity(observed[name])
    profile_sha = profile(observed['bootstrap'], outer, child_pid, registration)
    require(identity(observed['profile_sha256']) == profile_sha, 'exact backend profile encoding')
    require(identity(observed['bootstrap']['decode']['begin']['source_program']['sha256']) ==
                identity(observed['source_program_sha256']) and
            identity(observed['bootstrap']['decode']['begin']['uploads']['sha256']) ==
                identity(observed['upload_manifest_sha256']), 'Begin source metadata digest joins')
    require(0 < uint(observed['setup_commands']) <= 16384, 'setup count')
    for name in ('request_stream_bytes', 'response_stream_bytes'):
        require(0 < uint(observed[name]) <= 64 << 20, 'forward framing bound')
    pages = observed['page_permutation']
    require(type(pages) is list and len(pages) == 144 and all(type(v) is int for v in pages)
        and sorted(pages) == list(range(144)), 'full stable page permutation')
    roster = observed['files']
    keys(roster, 'frames child_stderr bytes_before_summary summary_bytes total_bytes')
    require(type(roster['frames']) is list and len(roster['frames']) == 4, 'four captured frames')
    pins = []

    def checked(value, name, maximum, empty=False):
        pin = normalized_pin(value, maximum, empty)
        raw = files[name]
        require(pin['path'] == directory + '/' + name and pin['bytes'] == len(raw)
            and pin['sha256'] == sha(raw), 'exact retained byte/path join')
        pins.append(pin)
        return raw

    chain = hashlib.sha256(b'ferric-prefix284-mlp548-four-transcript-v1\0' + registration + profile_sha).digest()
    inputs, outputs, states = [], [], 0
    for position, frame in enumerate(roster['frames']):
        token = TOKENS[position] if request['mode'] == 'teacher_forced' else (TOKENS[0] if position == 0 else outputs[-1])
        keys(frame, 'response control observation request')
        forward_raw = checked(frame['request'], f'request-{position}.json', 16 << 10)
        forward = parse(forward_raw)
        keys(forward, 'protocol id device_ids session registration profile_sha256 command')
        require(uint(forward['protocol']) == 1 and uint(forward['id']) == position + 1
            and type(forward['device_ids']) is list
            and [uint(v) for v in forward['device_ids']] == request['device_ids']
            and identity(forward['session']) == identity(request['session'])
            and identity(forward['registration']) == registration
            and identity(forward['profile_sha256']) == profile_sha, 'actual request envelope')
        command = forward['command']
        keys(command, 'op generation token cache_metadata rotary_bits')
        require(command['op'] == 'forward' and uint(command['generation']) == position + 1
            and uint(command['token']) == token, 'TF4 prompt or AR4 own previous checked argmax')
        require(type(command['cache_metadata']) is list
            and [uint(v, (1 << 32) - 1) for v in command['cache_metadata']] == [position] + pages,
            'actual stable KV metadata')
        require(type(command['rotary_bits']) is list and len(command['rotary_bits']) == 128, 'actual rotary extent')
        rotary = [uint(v, (1 << 32) - 1) for v in command['rotary_bits']]
        require(all(v & 0x7f800000 != 0x7f800000 for v in rotary), 'finite actual rotary')
        if position == 0:
            require(rotary == [0x3f800000] * 64 + [0] * 64, 'position-zero rotary')
        response(frame['response'], observed, position)
        completion = frame['response']['event']
        keys(completion, 'status generation position input_token output_token control observation capture chain')
        require(completion['status'] == 'completed'
            and (uint(completion['generation']), uint(completion['position']), uint(completion['input_token'])) ==
                (position + 1, position, token) and uint(completion['output_token']) < 151936,
            'completion position/token')
        raw_control = checked(frame['control'], f'control-{position}.bin', CONTROL_BYTES)
        states += control(raw_control)
        part(completion['control'], raw_control)
        raw = checked(frame['observation'], f'observation-{position}.bin', OBSERVATION_BYTES)
        require(len(raw) == OBSERVATION_BYTES, 'full main extent')
        values = words(raw, 'bf16')
        logits = values[-151936:]
        winner = max(range(151936), key=lambda i: number(logits[i], 'bf16'))
        require(winner == completion['output_token'], 'own finite lowest-index argmax')
        inputs.append(token)
        outputs.append(winner)
        part(completion['observation'], raw)
        capture = completion['capture']
        keys(capture, 'layer_hidden final_normalized logits total')
        require(type(capture['layer_hidden']) is list and len(capture['layer_hidden']) == 36, 'all36 hidden captures')
        for layer, desc in enumerate(capture['layer_hidden']):
            part(desc, raw[layer * 8192:(layer + 1) * 8192])
        part(capture['final_normalized'], raw[36 * 8192:37 * 8192])
        part(capture['logits'], raw[37 * 8192:])
        part(capture['total'], raw)
        material = chain + struct.pack('<QIII', position + 1, position, token, winner)
        for name in ('control', 'observation'):
            material += struct.pack('<I', completion[name]['bytes']) + identity(completion[name]['sha256'])
        chain = hashlib.sha256(material).digest()
        require(identity(completion['chain']) == chain, 'profile-bound typed transcript')
    require(observed['input_tokens'] == inputs, 'reported actual own-output inputs')
    require(type(observed['observed_output_tokens']) is list
        and [uint(v, 151935) for v in observed['observed_output_tokens']] == outputs, 'reported own outputs')
    response(observed['close'], observed, 4, True)
    close = observed['close']['event']
    keys(close, 'status completed_forwards transcript_sha256')
    require(close['status'] == 'closed' and uint(close['completed_forwards']) == 4
        and identity(close['transcript_sha256']) == chain == identity(observed['transcript_sha256']), 'Close5 transcript')
    checked(roster['child_stderr'], 'child-stderr.bin', 2 << 20, True)
    total = sum(len(raw) for raw in files.values())
    require(uint(roster['bytes_before_summary']) == total and uint(roster['summary_bytes']) == len(summary_raw)
        and uint(roster['total_bytes']) == total + len(summary_raw) <= OWN_LIMIT, '14-file exact private accounting')
    return {'schema': 'ferric-p228-checked-projection-residual-decode-structure-v1', 'mode': request['mode'],
        'input_tokens': inputs, 'output_tokens': outputs, 'positions': [0, 1, 2, 3],
        'logical_generations': [1, 2, 3, 4], 'expected_banks': [0, 1, 0, 1],
        'bank_reuse_order_independently_observed': False,
        'terminal_state_count': states, 'prefix_terminal_state_count': 288, 'tiles_terminal_state_count': 288,
        'child_pid': child_pid, 'closed_child_pids': [child_pid],
        'captured_tensor_rows': 152, 'captured_payloads': 4,
        'projection_residual_image': normalized_pin(outer['projection_residual_image'], 32 << 20),
        'paired_comparison_performed': False, 'old_native_equality_required': False,
        'profile_sha256': profile_sha.hex(), 'registration_sha256': registration.hex(),
        'transcript_sha256': chain.hex(), 'summary_sha256': sha(summary_raw), 'retained_files': pins,
        'parent_close_and_exit_flags_checked': True, 'external_supervisor_lifecycle_verified': False,
        'source_image_payloads_rehashed': False, 'rotary_nonzero_positions_independently_recomputed': False,
        'numerical_acceptance': False, 'performance_claim': False, 'production_authority': False,
        'full_long_workload': False}
