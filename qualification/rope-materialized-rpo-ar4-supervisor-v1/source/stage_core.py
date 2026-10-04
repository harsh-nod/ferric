"""Bounded layer-zero diagnostics. No receipt, launch or acceptance authority."""
import hashlib
import json
import math
import struct

PAYLOAD_BYTES = 256136
OBSERVATION_BYTES = 606976
STAGES = (
    ('before_prefix', (('input','bf16',8192), ('cache_metadata','u32',580), ('rotary','f32',512))),
    ('after_prefix', (('input_normalized','bf16',8192), ('raw_qkv','bf16',6144), ('query','bf16',4096),
                      ('current_key','bf16',1024), ('current_value','bf16',1024), ('attention','bf16',4096),
                      ('output_partial','f32',16384))),
    ('after_first_residual', (('first_residual','bf16',8192),)),
    ('after_mlp', (('post_normalized','bf16',8192), ('gate','bf16',12288), ('up','bf16',12288),
                   ('activation','bf16',12288), ('down_partial','f32',16384))),
    ('after_final_residual', (('final_hidden','bf16',8192),)),
)
GROUPS = (
    ('input', ('input',)), ('input_norm', ('input_normalized',)), ('raw_qkv', ('raw_qkv',)),
    ('head_norm_rope_append', ('query','current_key','current_value')), ('attention', ('attention',)),
    ('output_projection', ('output_partial',)), ('first_residual', ('first_residual',)),
    ('post_norm', ('post_normalized',)), ('gate_up', ('gate','up')), ('swiglu', ('activation',)),
    ('down_projection', ('down_partial',)), ('final_residual', ('final_hidden',)),
)

def require(ok, message):
    if not ok:
        raise ValueError(message)

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def parse(raw):
    def pairs(items):
        result = {}
        for key,value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def invalid(_):
        raise ValueError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)

def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed JSON fields')

def uint(value, maximum=(1 << 64)-1):
    require(type(value) is int and 0 <= value <= maximum, 'unsigned integer')
    return value

def array_bytes(value, count):
    require(type(value) is list and len(value) == count and
            all(type(v) is int and 0 <= v <= 255 for v in value), 'exact byte array')
    return bytes(value)

def words(raw, scalar):
    width,code,exponent = {'bf16':(2,'H',0x7f80), 'f32':(4,'I',0x7f800000)}[scalar]
    require(type(raw) is bytes and len(raw) > 0 and len(raw) % width == 0, 'scalar extent')
    result = struct.unpack('<'+str(len(raw)//width)+code, raw)
    require(all(v & exponent != exponent for v in result), 'nonfinite captured/reference scalar')
    return result

def number(bits, scalar):
    return struct.unpack('<f',struct.pack('<I',bits << 16 if scalar == 'bf16' else bits))[0]

def metadata_offset(raw):
    require(len(raw) == 580, 'metadata extent')
    row = struct.unpack('<145I',raw)
    require(row[0] == 0 and sorted(row[1:]) == list(range(144)), 'position-zero full page permutation')
    return row[1]*16*512*2

def validate_capture(raw, observation=None, pages=None):
    require(type(raw) is bytes and 0 < len(raw) <= 2 << 20, 'bounded stage JSON')
    capture = parse(raw)
    keys(capture, 'schema generation position layer parts payload_bytes payload_sha256 payload '
         'full_cache_capture native_close_confirmed numerical_acceptance performance_claim production_authority')
    require(capture['schema'] == 'FerricFiniteLayerZeroCaptureV1' and
            (uint(capture['generation']),uint(capture['position']),uint(capture['layer'])) == (1,0,0),
            'closed generation-one/position-zero/layer-zero scope')
    require(all(capture[k] is False for k in ('full_cache_capture','native_close_confirmed',
            'numerical_acceptance','performance_claim','production_authority')), 'stage authority flags')
    require(uint(capture['payload_bytes']) == PAYLOAD_BYTES and type(capture['parts']) is list
            and len(capture['parts']) == 34, 'exact stage census')
    payload = array_bytes(capture['payload'], PAYLOAD_BYTES)
    require(array_bytes(capture['payload_sha256'],32).hex() == sha(payload), 'stage payload digest')
    rows,offset,index,cache_offsets = {},0,0,{}
    for boundary,specs in STAGES:
        for rank in range(2):
            for role,scalar,size in specs:
                part = capture['parts'][index]
                keys(part, 'boundary rank role scalar elements offset bytes source_byte_offset sha256')
                width = 2 if scalar == 'bf16' else 4
                require((part['boundary'],uint(part['rank']),part['role'],part['scalar'],uint(part['elements']),
                         uint(part['offset']),uint(part['bytes'])) ==
                        (boundary,rank,role,scalar,size//width,offset,size), 'exact stage part order/type/extent')
                data = payload[offset:offset+size]
                source_offset = cache_offsets[rank] if role in ('current_key','current_value') else 0
                require(uint(part['source_byte_offset']) == source_offset and
                        array_bytes(part['sha256'],32).hex() == sha(data), 'stage source offset/hash')
                if scalar == 'u32':
                    cache_offsets[rank] = metadata_offset(data)
                    if pages is not None:
                        require(list(struct.unpack('<145I',data))[1:] == pages, 'stage pages differ from parent')
                else:
                    words(data,scalar)
                rows[rank,role] = {'boundary':boundary,'scalar':scalar,'data':data,'part':part}
                offset += size
                index += 1
    require(index == 34 and offset == len(payload), 'stage complete coverage')
    if observation is not None:
        require(type(observation) is bytes and len(observation) == OBSERVATION_BYTES, 'main observation extent')
        require(all(rows[rank,'final_hidden']['data'] == observation[:8192] for rank in range(2)),
                'stage final rank rows differ from main layer-zero row')
    return rows

def metrics(actual_raw, reference_raw, scalar):
    actual,reference = words(actual_raw,scalar),words(reference_raw,scalar)
    require(len(actual) == len(reference), 'comparison geometry')
    values = [(number(a,scalar),number(b,scalar)) for a,b in zip(actual,reference)]
    absolute = [abs(a-b) for a,b in values]
    error_sq = math.fsum((a-b)**2 for a,b in values)
    norm_sq = math.fsum(b*b for _,b in values)
    sign = 1 << (15 if scalar == 'bf16' else 31)
    def ordered(bits):
        return -(bits & (sign-1)) if bits & sign else bits
    exact = sum(a == b for a,b in zip(actual,reference))
    return {'scalar':scalar,'words':len(actual),'exact_words':exact,'different_words':len(actual)-exact,
            'signed_zero_bit_differences':sum(a != b and (a & (sign-1)) == (b & (sign-1)) == 0
                                               for a,b in zip(actual,reference)),
            'max_ordered_scalar_steps':max(abs(ordered(a)-ordered(b)) for a,b in zip(actual,reference)),
            'max_absolute_error':max(absolute),'mean_absolute_error':math.fsum(absolute)/len(actual),
            'error_l2':math.sqrt(error_sq),'reference_l2':math.sqrt(norm_sq),
            'relative_l2':math.sqrt(error_sq/norm_sq) if norm_sq else None,'reference_norm_zero':norm_sq == 0,
            'actual_sha256':sha(actual_raw),'reference_sha256':sha(reference_raw),
            'first_differences':[{'index':i,'actual_bits':a,'reference_bits':b,'absolute_error':absolute[i]}
                                 for i,(a,b) in enumerate(zip(actual,reference)) if a != b][:8]}

def reference_name(rank, role):
    shared = {'input':'pos0-embedding','input_normalized':'pos0-layer0-input-norm',
              'first_residual':'pos0-layer0-attention-residual','post_normalized':'pos0-layer0-post-norm',
              'final_hidden':'pos0-layer0-hidden'}
    suffix = {'raw_qkv':'qkv','query':'query','current_key':'key','current_value':'value','attention':'attention',
              'output_partial':'output-partial','gate':'gate','up':'up','activation':'activation','down_partial':'down-partial'}
    return shared[role] if role in shared else 'pos0-layer0-rank%d-%s' % (rank,suffix[role])

def dependencies(rank, role):
    same = {'input_normalized':['input'],'raw_qkv':['input_normalized'],'query':['raw_qkv'],
            'current_key':['raw_qkv'],'current_value':['raw_qkv'],'attention':['query','current_key','current_value'],
            'output_partial':['attention'],'post_normalized':['first_residual'],'gate':['post_normalized'],
            'up':['post_normalized'],'activation':['gate','up'],'down_partial':['activation']}
    if role in ('first_residual','final_hidden'):
        partial = 'output_partial' if role == 'first_residual' else 'down_partial'
        residual = 'input' if role == 'first_residual' else 'first_residual'
        return [(rank,residual),(0,partial),(1,partial)]
    return [(rank,name) for name in same.get(role,[])]

def compare_stages(rows, reference):
    require(len(rows) == 34, 'validated part roster required')
    required = {reference_name(rank,role) for _,roles in GROUPS for role in roles for rank in range(2)}
    require(set(reference) == required and len(required) == 25, 'closed independent reference roster')
    comparisons,by_role,first = [],{},None
    for group,roles in GROUPS:
        mismatches = []
        for role in roles:
            for rank in range(2):
                actual = rows[rank,role]
                name = reference_name(rank,role)
                result = metrics(actual['data'],reference[name],actual['scalar'])
                result.update(rank=rank,role=role,boundary=actual['boundary'],dependency_group=group,reference_name=name)
                deps = dependencies(rank,role)
                result['captured_dependency_inputs_exact'] = all(by_role[key]['different_words'] == 0 for key in deps) if deps else None
                result['causal_attribution_proven'] = False
                comparisons.append(result)
                by_role[rank,role] = result
                if result['different_words']:
                    mismatches.append({'rank':rank,'role':role,'different_words':result['different_words']})
        if first is None and mismatches:
            first = {'dependency_group':group,'mismatches':mismatches}
    internal = []
    expected_rotary = struct.pack('<128I',*([0x3f800000]*64+[0]*64))
    for rank in range(2):
        value = rows[rank,'current_value']['data']
        expected_attention = b''.join(value[(head//4)*256:(head//4+1)*256] for head in range(16))
        internal.append({'rank':rank,'raw_v_equals_current_value':rows[rank,'raw_qkv']['data'][5120:] == value,
            'position_zero_attention_equals_own_gqa_value':rows[rank,'attention']['data'] == expected_attention,
            'position_zero_rotary_exact_one_zero':rows[rank,'rotary']['data'] == expected_rotary})
    return {'schema':'ferric-p224-layer0-stage-diagnostics-v1','generation':1,'position':0,'layer':0,'input_token':9112,
            'first_nonexact_captured_dependency_group':first,'comparisons':comparisons,'internal_checks':internal,
            'shared_rank_vectors_equal':{role:rows[0,role]['data'] == rows[1,role]['data']
                for role in ('input','input_normalized','first_residual','post_normalized','final_hidden')},
            'numerical_parts':30,'metadata_parts':4,'numerical_words':sum(row['words'] for row in comparisons),
            'exact_words':sum(row['exact_words'] for row in comparisons),
            'different_words':sum(row['different_words'] for row in comparisons),
            'earliest_group_is_not_execution_time_order':True,'causal_attribution_proven':False,
            'missing_internal_boundaries':['FP32 RMS intermediate','per-lane projection partials','head-norm pre-RoPE'],
            'candidate_intermediate_substitution':False,'numerical_acceptance':False,'acceptance_threshold':None,
            'full_model_correctness':False,'performance_claim':False,'production_authority':False}
