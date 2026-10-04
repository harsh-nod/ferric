"""Pure structural validation of native four-forward retained evidence.

The caller owns source/image/process/device admission. No filesystem or GPU I/O.
"""
import hashlib
import json
import posixpath
import struct

from stage_core import OBSERVATION_BYTES, array_bytes, keys, parse, require, sha, uint, validate_capture, words, number

TOKENS = [9112,2190,3772,220]
MODEL_ID = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
BUNDLE_ID = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
CONTROL_BYTES = 11848
PROMPT_PINS = {'manifest':(21318,'30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600'),
               'text':(11224,'a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a'),
               'tokens':(8192,'2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02')}

def identity(value):
    raw = array_bytes(value,32)
    require(raw != bytes(32), 'nonzero identity')
    return raw

def absolute(path):
    require(type(path) is str and path.startswith('/') and posixpath.normpath(path) == path
            and '\0' not in path, 'normalized absolute evidence path')
    return path

def normalized_pin(value, maximum, empty=False):
    keys(value,'path bytes sha256')
    size = uint(value['bytes'],maximum)
    require(empty or size > 0, 'nonempty pin')
    return {'path':absolute(value['path']),'bytes':size,'sha256':array_bytes(value['sha256'],32).hex()}

def part(value, raw):
    keys(value,'bytes sha256')
    require(uint(value['bytes'],2 << 20) == len(raw) and array_bytes(value['sha256'],32).hex() == sha(raw), 'wire part extent/hash')

def terminal(row,tasks):
    require(len(row) == tasks+6 and row[:4] == (1,0,(1 << tasks)-1,(1 << tasks)-1)
            and row[5] == 0 and row[6:] == (64,)*tasks, 'terminal state flags/arrivals')
    require(all(((row[4] >> (2*i)) & 3) in (1,2) for i in range(tasks)), 'terminal task owner pairs')
    require(tasks == 16 or row[4] >> (2*tasks) == 0, 'terminal unused owner bits')

def control(raw):
    require(type(raw) is bytes and len(raw) == CONTROL_BYTES, 'exact control extent')
    offset,count = 16,0
    for _layer in range(36):
        for tasks in (16,5):
            size = 4*(tasks+6)
            for _rank in range(2):
                terminal(struct.unpack_from('<'+str(tasks+6)+'I',raw,offset),tasks)
                offset += size
                count += 1
        offset += 64
    require(offset+24 == len(raw) and count == 144, 'control full census')
    return count

def response(value,observed,position,closed=False):
    keys(value,'protocol id device_ids session registration profile_sha256 event native_closed gpu_execution '
         'numerical_acceptance performance_claim production_authority')
    require(type(value['device_ids']) is list and len(value['device_ids']) == 2
            and all(uint(v) > 0 for v in value['device_ids']), 'response device scalar types')
    require(uint(value['protocol']) == 1 and uint(value['id']) == position+1
            and value['device_ids'] == observed['request']['device_ids']
            and identity(value['session']) == identity(observed['request']['session'])
            and identity(value['registration']) == identity(observed['registration_sha256'])
            and identity(value['profile_sha256']) == identity(observed['profile_sha256'])
            and value['native_closed'] is closed and value['gpu_execution'] is True, 'response scope/lifecycle')
    require(all(value[k] is False for k in ('numerical_acceptance','performance_claim','production_authority')), 'response authority')

def profile(bootstrap,request,child_pid):
    names = 'protocol profile device_ids scope timeout_ms prompt_tokens capture_layer0'
    keys(bootstrap,names)
    scope_names = 'bundle_id model_id session pool_identity group_id child_identity'
    scope = bootstrap['scope']
    keys(scope,scope_names)
    require(type(bootstrap['device_ids']) is list and len(bootstrap['device_ids']) == 2
            and all(uint(v) > 0 for v in bootstrap['device_ids']), 'bootstrap device scalar types')
    require(uint(bootstrap['protocol']) == 1 and bootstrap['profile'] == 'rearm_four_forward_v1'
            and bootstrap['device_ids'] == request['device_ids'] and uint(bootstrap['timeout_ms']) == request['dispatch_timeout_ms']
            and bootstrap['prompt_tokens'] == TOKENS and all(type(v) is int for v in bootstrap['prompt_tokens'])
            and bootstrap['capture_layer0'] is request['capture_layer0'], 'closed smoke bootstrap')
    require(identity(scope['bundle_id']) == identity(request['expected_bundle_id'])
            and identity(scope['model_id']) == identity(request['expected_model_id'])
            and identity(scope['session']) == identity(request['session'])
            and uint(scope['child_identity'],(1 << 32)-1) == child_pid
            and uint(scope['pool_identity']) > 0, 'bootstrap scope/child')
    uint(scope['group_id'])
    # Serialize the exact Rust declaration order, not the input JSON key order.
    canonical = {name:({key:scope[key] for key in scope_names.split()} if name == 'scope' else bootstrap[name])
                 for name in names.split()}
    return hashlib.sha256(b'ferric-rearm-four-forward-profile-v1\0' +
                          json.dumps(canonical,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()).digest()

def validate_smoke(summary_raw, files):
    """Return JSON-only checked facts from exact summary/file bytes; no reference required.

    files keys are exact relative native filenames. Source/image payloads and
    external process/topology evidence remain the supervising caller's checks.
    """
    require(type(summary_raw) is bytes and 0 < len(summary_raw) <= 64 << 10, 'bounded native summary')
    require(type(files) is dict and all(type(k) is str and type(v) is bytes for k,v in files.items()), 'byte-file mapping')
    observed = parse(summary_raw)
    keys(observed,'schema request child_pid registration_sha256 source_program_sha256 upload_manifest_sha256 '
         'bootstrap profile_sha256 setup_commands completed_forwards input_tokens observed_output_tokens page_permutation '
         'transcript_sha256 request_stream_bytes response_stream_bytes files close child_exit_zero process_group_absent '
         'native_closed gpu_execution numerical_acceptance performance_claim production_authority full_long_workload')
    require(observed['schema'] == 'FerricFiniteRearmSmokeObservationV1' and uint(observed['completed_forwards']) == 4
            and observed['input_tokens'] == TOKENS and all(type(v) is int for v in observed['input_tokens']), 'closed four-forward result')
    require(all(observed[k] is True for k in ('child_exit_zero','process_group_absent','native_closed','gpu_execution'))
            and all(observed[k] is False for k in ('numerical_acceptance','performance_claim','production_authority','full_long_workload')),
            'parent outcome/authority flags')
    child_pid = uint(observed['child_pid'],(1 << 32)-1)
    require(child_pid > 0, 'child PID')
    request = observed['request']
    keys(request,'schema source worker images expected_bundle_id expected_model_id device_ids session prompt capture_layer0 '
         'evidence_directory dispatch_timeout_ms child_deadline_ms')
    require(request['schema'] == 'FerricFiniteRearmSmokeRequestV1' and type(request['capture_layer0']) is bool
            and identity(request['expected_model_id']).hex() == MODEL_ID and identity(request['expected_bundle_id']).hex() == BUNDLE_ID,
            'candidate model/profile/capture scope')
    absolute(request['source'])
    directory = absolute(request['evidence_directory'])
    require(type(request['device_ids']) is list and len(request['device_ids']) == 2
            and all(uint(v) > 0 for v in request['device_ids']) and len(set(request['device_ids'])) == 2, 'two device identities')
    identity(request['session'])
    require(1 <= uint(request['dispatch_timeout_ms']) <= 10000 and 1000 <= uint(request['child_deadline_ms']) <= 3600000,
            'native timeout bounds')
    normalized_pin(request['worker'],512 << 20)
    keys(request['images'],'prefix mlp residual tail')
    for image in request['images'].values():
        normalized_pin(image,16 << 20)
    keys(request['prompt'],'manifest text tokens')
    for name,(size,digest) in PROMPT_PINS.items():
        pin = normalized_pin(request['prompt'][name],32 << 10)
        require((pin['bytes'],pin['sha256']) == (size,digest), 'original prompt provenance pins')
    registration = identity(observed['registration_sha256'])
    for name in ('source_program_sha256','upload_manifest_sha256','transcript_sha256'):
        identity(observed[name])
    profile_sha = profile(observed['bootstrap'],request,child_pid)
    require(identity(observed['profile_sha256']) == profile_sha, 'bootstrap/profile hash')
    require(0 < uint(observed['setup_commands']) <= 16384, 'setup command bound')
    require(0 < uint(observed['request_stream_bytes']) <= 64 << 20 and
            0 < uint(observed['response_stream_bytes']) <= 64 << 20, 'framed stream bounds')
    pages = observed['page_permutation']
    require(type(pages) is list and len(pages) == 144 and all(type(v) is int for v in pages)
            and sorted(pages) == list(range(144)), 'stable full page permutation')
    roster = observed['files']
    keys(roster,'frames child_stderr bytes_before_summary summary_bytes total_bytes')
    require(type(roster['frames']) is list and len(roster['frames']) == 4, 'four retained frames')
    expected_names = {'child-stderr.bin'} | {f'{kind}-{pos}.bin' for kind in ('control','observation') for pos in range(4)}
    if request['capture_layer0']:
        expected_names.add('layer0.json')
    require(set(files) == expected_names, 'exact native file roster')
    pins,outputs,state_count = [],[],0
    def checked(value,name,maximum,empty=False):
        pin = normalized_pin(value,maximum,empty)
        data = files[name]
        require(pin['path'] == directory+'/'+name and pin['bytes'] == len(data) and pin['sha256'] == sha(data), 'retained file join')
        pins.append(pin)
        return data
    chain = hashlib.sha256(b'ferric-rearm-four-forward-transcript-v1\0'+registration+profile_sha).digest()
    annex = None
    for position,frame in enumerate(roster['frames']):
        keys(frame,'response control observation layer0')
        response(frame['response'],observed,position)
        c = frame['response']['event']
        keys(c,'status generation position input_token output_token control observation capture stage_capture chain')
        require(c['status'] == 'completed' and uint(c['generation']) == position+1 and uint(c['position']) == position
                and uint(c['input_token']) == TOKENS[position] and uint(c['output_token']) < 151936, 'frame token/position identity')
        control_raw = checked(frame['control'],f'control-{position}.bin',CONTROL_BYTES)
        state_count += control(control_raw)
        part(c['control'],control_raw)
        data = checked(frame['observation'],f'observation-{position}.bin',OBSERVATION_BYTES)
        require(len(data) == OBSERVATION_BYTES, 'full main observation extent')
        all_words = words(data,'bf16')
        logits = all_words[-151936:]
        require(max(range(151936),key=lambda i:number(logits[i],'bf16')) == c['output_token'], 'own-logit lowest-index argmax')
        part(c['observation'],data)
        payload = c['capture']
        keys(payload,'layer_hidden final_normalized logits total')
        require(type(payload['layer_hidden']) is list and len(payload['layer_hidden']) == 36, 'main layer roster')
        for layer,descriptor in enumerate(payload['layer_hidden']):
            part(descriptor,data[layer*8192:(layer+1)*8192])
        part(payload['final_normalized'],data[36*8192:37*8192])
        part(payload['logits'],data[37*8192:])
        part(payload['total'],data)
        stage = b''
        if position == 0 and request['capture_layer0']:
            require(c['stage_capture'] is not None and frame['layer0'] is not None, 'requested annex missing')
            stage = checked(frame['layer0'],'layer0.json',2 << 20)
            part(c['stage_capture'],stage)
            rows = validate_capture(stage,data,pages)
            expected_rotary = struct.pack('<128I',*([0x3f800000]*64+[0]*64))
            require(all(rows[rank,'rotary']['data'] == expected_rotary for rank in range(2)), 'position-zero source rotary')
            annex = pins[-1]
        else:
            require(c['stage_capture'] is None and frame['layer0'] is None, 'unexpected stage annex')
        chained = chain+struct.pack('<QIII',c['generation'],c['position'],c['input_token'],c['output_token'])
        for name in ('control','observation'):
            chained += struct.pack('<I',c[name]['bytes'])+array_bytes(c[name]['sha256'],32)
        chained += b'\1'+struct.pack('<I',len(stage))+bytes.fromhex(sha(stage)) if stage else b'\0'
        chain = hashlib.sha256(chained).digest()
        require(array_bytes(c['chain'],32) == chain, 'complete transcript chain')
        outputs.append(c['output_token'])
    require(observed['observed_output_tokens'] == outputs and all(type(v) is int for v in observed['observed_output_tokens']), 'reported outputs')
    response(observed['close'],observed,4,True)
    end = observed['close']['event']
    keys(end,'status completed_forwards transcript_sha256')
    require(end['status'] == 'closed' and uint(end['completed_forwards']) == 4
            and array_bytes(end['transcript_sha256'],32) == chain == array_bytes(observed['transcript_sha256'],32), 'closed transcript')
    checked(roster['child_stderr'],'child-stderr.bin',2 << 20,True)
    count = sum(len(data) for data in files.values())
    require(uint(roster['bytes_before_summary']) == count and uint(roster['summary_bytes']) == len(summary_raw)
            and uint(roster['total_bytes']) == count+len(summary_raw) <= 8 << 20, 'exact native file byte census')
    require(state_count == 576, 'all four-forward terminal states')
    return {'schema':'ferric-p224-checked-rearm-smoke-structure-v1','input_tokens':TOKENS.copy(),'output_tokens':outputs,
            'terminal_state_count':state_count,'child_pid':child_pid,'device_ids':request['device_ids'],
            'session':identity(request['session']).hex(),'registration_sha256':registration.hex(),
            'profile_sha256':profile_sha.hex(),'source_program_sha256':identity(observed['source_program_sha256']).hex(),
            'transcript_sha256':chain.hex(),'summary_sha256':sha(summary_raw),'retained_files':pins,'annex':annex,
            'parent_close_and_exit_flags_checked':True,'external_supervisor_lifecycle_verified':False,
            'source_image_payloads_rehashed':False,'device_admission_verified':False,'numerical_acceptance':False,
            'production_authority':False,'performance_claim':False,'full_long_workload':False}
