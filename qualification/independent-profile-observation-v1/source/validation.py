"""Closed six-case, full-capture V5/V6 comparison. No execution authority."""
import hashlib
import json
from pathlib import PurePosixPath
import re
import struct

CASES = ('genuine-pos0', 'genuine-pos4', 'patterned-pos15', 'patterned-pos16',
         'patterned-pos2047', 'patterned-pos2048')
DEVICES = [16366993098680759275, 10838076764495710945]
KINDS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
STAGES = ('normalized', 'qkv', 'query', 'key_cache', 'value_cache', 'attention', 'output_partial')
EXTENTS = [8192, 8192, 25165824, 512, 512, 580, 16777216,
           8192, 6144, 4096, 2359296, 2359296, 4096, 16384]
CAPTURE_BYTES = sum(EXTENTS[7:])
CASE_CAPTURE_BYTES = 4 * CAPTURE_BYTES
SYMBOL = 'ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6'
FALSE_FIELDS = ('independent_numerical_acceptance', 'full_model_correctness',
                'performance_claim', 'production_authority', 'runtime_premises_discharged',
                'scheduler_progress_guaranteed', 'all_workgroups_participated')
FORMAT = 'little-endian concatenation of six BF16 buffers then one FP32 buffer; whole KV caches included'
TIMING = 'host currentness/kernarg/publish through GPU completion and idle; not device-only or a benchmark'
BASE_FIELDS = set(('schema authority request_sha256 baseline_request baseline_image_sha256 tiles '
                  'devices history_kind position reviews opened_device completed_and_closed bitwise_match '
                  'input_sha256 initial_output_sha256 data_root_bytes v5_grid v6_grid workgroup '
                  'stage_order capture_bytes_per_rank capture_format').split()) | set(FALSE_FIELDS)


def require(value, message):
    if not value:
        raise RuntimeError(message)


def keys(value, expected):
    require(type(value) is dict and set(value) == set(expected.split() if isinstance(expected, str)
                                                     else expected), 'closed JSON fields')


def uint(value, maximum=(1 << 64) - 1):
    require(type(value) is int and 0 <= value <= maximum, 'unsigned integer')
    return value


def digest(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'SHA256 spelling')
    return value


def sha(data):
    return hashlib.sha256(data).hexdigest()


def parse(raw):
    require(type(raw) is bytes and 0 < len(raw) <= 64 << 10, 'bounded native JSON')
    def pairs(rows):
        out = {}
        for key, value in rows:
            require(key not in out, 'duplicate JSON key')
            out[key] = value
        return out
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def empty_processes(raw):
    rows = parse(raw)
    require(type(rows) is list and len(rows) == 2, 'both selected GPU process rows')
    for rank, row in enumerate(rows):
        keys(row, 'gpu process_list')
        require(type(row['gpu']) is int and row['gpu'] == rank
            and row['process_list'] == [{'process_info': 'No running processes detected'}],
            'exact empty selected-device process roster; no monitor exception')
    return rows


def pin(record, maximum=64 << 20, nonempty=True):
    keys(record, 'path bytes sha256')
    require(type(record['path']) is str, 'pin path string')
    path = PurePosixPath(record['path'])
    require(path.is_absolute() and '..' not in path.parts and str(path) == record['path'],
            'canonical absolute pin spelling')
    require(uint(record['bytes'], maximum) > 0 or not nonempty, 'nonempty pinned input')
    digest(record['sha256'])
    return record


def artifact(value):
    keys(value, 'object descriptor_sha256 canonical_code_object_digest entry_symbol descriptor_symbol')
    pin(value['object'])
    digest(value['descriptor_sha256']); digest(value['canonical_code_object_digest'])
    require(value['entry_symbol'] == SYMBOL and value['descriptor_symbol'] == SYMBOL + '.kd',
            'distinct exact V6 symbol')


def case_scope(case):
    require(case in CASES, 'closed case roster')
    history, position = case.split('-pos')
    return history, int(position)


def request(value, baseline, case):
    keys(value, 'schema baseline_request tiles reviews timeout_ms capture_directory')
    require(value['schema'] == 'fe2o3-qwen-prefix-tiles-comparison-request-v6', 'request schema')
    pin(value['baseline_request'], 64 << 10); artifact(value['tiles'])
    require(type(value['reviews']) is list and len(value['reviews']) == 6, 'six actual reviews')
    for row in value['reviews']: pin(row, 64 << 10)
    require(len({row['path'] for row in value['reviews']}) == 6, 'distinct review paths')
    require(1 <= uint(value['timeout_ms'], 10000) <= baseline['timeout_ms'], 'bounded dispatch timeout')
    require(type(value['capture_directory']) is str, 'capture directory string')
    directory = PurePosixPath(value['capture_directory'])
    require(directory.is_absolute() and '..' not in directory.parts
            and str(directory) == value['capture_directory'], 'canonical capture directory spelling')
    history, position = case_scope(case)
    require(baseline['schema'] == 'fe2o3-qwen-resident-prefix-tp2-request-v1'
            and baseline['history_kind'] == history and type(baseline['position']) is int
            and baseline['position'] == position, 'actual selected paired baseline')
    require(baseline['devices'] == [dict(rank=rank, unique_id=uid) for rank, uid in enumerate(DEVICES)],
            'exact ordered paired devices')
    pin(baseline['producer']); pin(baseline['consumer']); pin(baseline['pair_reference'])
    require(type(baseline['prefix_requests']) is list and len(baseline['prefix_requests']) == 2,
            'two actual rank-local source requests')
    for row in baseline['prefix_requests']: pin(row, 64 << 10)


def review(value, kind, requested, baseline, case):
    keys(value, 'schema authority kind baseline_request_sha256 baseline_image_sha256 tiles_image_sha256 '
         'descriptor_sha256 canonical_code_object_digest devices history_kind position workgroup grid '
         'reviewed runtime_premises_discharged production_authority notes')
    history, position = case_scope(case)
    expected = dict(schema='fe2o3-qwen-prefix-tiles-comparison-review-v6', authority='none', kind=kind,
        baseline_request_sha256=requested['baseline_request']['sha256'],
        baseline_image_sha256=baseline['producer']['sha256'],
        tiles_image_sha256=requested['tiles']['object']['sha256'],
        descriptor_sha256=requested['tiles']['descriptor_sha256'],
        canonical_code_object_digest=requested['tiles']['canonical_code_object_digest'],
        devices=DEVICES, history_kind=history, position=position, workgroup=[64, 1, 1], grid=[4096, 1, 1])
    require(kind in KINDS and all(value[key] == item for key, item in expected.items()),
            'actual case/image/device scoped review')
    require(type(value['position']) is int and value['reviewed'] is True
            and value['runtime_premises_discharged'] is False and value['production_authority'] is False,
            'engineering review without authority')
    require(type(value['notes']) is str and value['notes'].strip()
            and len(value['notes'].encode()) <= 16384, 'substantive bounded operator notes')


def base(value, requested, requested_pin, baseline, case):
    history, position = case_scope(case)
    expected = dict(authority='none', request_sha256=requested_pin['sha256'],
        baseline_request=requested['baseline_request'], baseline_image_sha256=baseline['producer']['sha256'],
        tiles=requested['tiles'], devices=DEVICES, history_kind=history, position=position,
        reviews=requested['reviews'], data_root_bytes=EXTENTS, v5_grid=[128, 1, 1],
        v6_grid=[4096, 1, 1], workgroup=[64, 1, 1], stage_order=list(STAGES),
        capture_bytes_per_rank=CAPTURE_BYTES, capture_format=FORMAT)
    require(all(value.get(key) == item for key, item in expected.items()), 'exact inspected case/artifact')
    for name in FALSE_FIELDS: require(value.get(name) is False, 'observation has no extended authority')
    require(type(value['position']) is int and type(value['capture_bytes_per_rank']) is int,
            'integer report geometry')
    for name in ('input_sha256', 'initial_output_sha256'):
        require(type(value[name]) is list and len(value[name]) == 2, 'two input hash ranks')
        for rows in value[name]:
            require(type(rows) is list and len(rows) == 7, 'seven ordered buffer hashes')
            for item in rows: digest(item)
    for index in (0, 4, 5):
        require(value['input_sha256'][0][index] == value['input_sha256'][1][index],
                'paired original hidden/rotary/metadata')
    metadata = struct.pack('<145I', position, *[(page * 5 + 7) % 144 for page in range(144)])
    require(value['input_sha256'][0][5] == sha(metadata), 'exact retained fixture page permutation')
    for name in ('data_root_bytes', 'v5_grid', 'v6_grid', 'workgroup'):
        require(all(type(word) is int for word in value[name]), 'integer geometry, not booleans')


def inspection(value, requested, requested_pin, baseline, case):
    keys(value, BASE_FIELDS)
    base(value, requested, requested_pin, baseline, case)
    require(value['schema'] == 'fe2o3-qwen-prefix-tiles-comparison-inspection-v6'
            and all(value[name] is False for name in ('opened_device', 'completed_and_closed', 'bitwise_match')),
            'closed inert inspection')
    return value


def terminal(words, profile):
    require(type(words) is list and len(words) == (22 if profile == 0 else 284), 'exact typed state extent')
    for word in words: uint(word, (1 << 32) - 1)
    if profile == 0:
        owners = [(words[4] >> (2 * i)) & 3 for i in range(16)]
        require(words[:4] == [1, 0, 65535, 65535] and words[5] == 0
                and words[6:] == [64] * 16 and all(x in (1, 2) for x in owners), 'all22 terminal words')
    else:
        owners = words[24:154]
        require(words[:4] == [1, 0, 0, 31] and words[4:9] == words[9:14] == [1, 48, 1, 16, 64]
                and words[14:19] == words[19:24] == [4294967295] * 4 + [3]
                and all(1 <= x <= 64 for x in owners) and words[154:] == [64] * 130,
                'all284 terminal words')
    return dict(owners=[owner - 1 for owner in owners], useful_workgroups=len(set(owners)))


def finite(raw, width):
    require(width in (2, 4), 'BF16 or F32 capture words')
    mask = 0x7f80 if width == 2 else 0x7f800000
    code = '<H' if width == 2 else '<I'
    require(len(raw) % width == 0 and all(word[0] & mask != mask for word in struct.iter_unpack(code, raw)),
            'computed output contains nonfinite/unwritten bits')


def observation(value, inspected, requested, requested_pin, baseline, case, captures):
    keys(value, BASE_FIELDS | set('profiles captures stages immutable_input_readbacks_match timing_boundary'.split()))
    inspection(inspected, requested, requested_pin, baseline, case)
    base(value, requested, requested_pin, baseline, case)
    require(value['schema'] == 'fe2o3-qwen-prefix-tiles-comparison-observation-v6'
            and all(value[name] is True for name in ('opened_device', 'completed_and_closed', 'bitwise_match',
                                                    'immutable_input_readbacks_match'))
            and value['timing_boundary'] == TIMING, 'closed native comparison success')
    for key in BASE_FIELDS - {'schema', 'opened_device', 'completed_and_closed', 'bitwise_match'}:
        require(value[key] == inspected[key], 'inspection/execution input identity changed')
    require(type(value['profiles']) is list and len(value['profiles']) == 2, 'two ordered closed profiles')
    owners = []
    for index, name in enumerate(('baseline_v5', 'tiles_v6')):
        profile = value['profiles'][index]
        keys(profile, 'profile states host_dispatch_elapsed_ns closed')
        require(profile['profile'] == name and profile['closed'] is True, 'explicit native Close')
        require(type(profile['states']) is list and len(profile['states']) == 2
                and type(profile['host_dispatch_elapsed_ns']) is list
                and len(profile['host_dispatch_elapsed_ns']) == 2, 'paired states/timing')
        owners.append([terminal(words, index) for words in profile['states']])
        for elapsed in profile['host_dispatch_elapsed_ns']: uint(elapsed)
    require(type(value['captures']) is list and len(value['captures']) == 2, 'two capture profiles')
    directory = PurePosixPath(requested['capture_directory'])
    arrays = []
    expected_paths = set()
    for index, name in enumerate(('baseline-v5', 'tiles-v6')):
        require(type(value['captures'][index]) is list and len(value['captures'][index]) == 2, 'paired captures')
        ranks = []
        for rank, record in enumerate(value['captures'][index]):
            pin(record, CAPTURE_BYTES)
            path = str(directory / (name + '-rank' + str(rank) + '.bin'))
            require(record['path'] == path and record['bytes'] == CAPTURE_BYTES, 'exact capture path/extent')
            expected_paths.add(path)
            raw = captures.get(path)
            require(type(raw) is bytes and len(raw) == CAPTURE_BYTES and sha(raw) == record['sha256'],
                    'retained entire closed capture')
            offset, stages = 0, []
            for extent in EXTENTS[7:]:
                stages.append(raw[offset:offset + extent]); offset += extent
            ranks.append(stages)
        arrays.append(ranks)
    require(set(captures) == expected_paths, 'no substituted or extra captures')
    rows = []
    _, position = case_scope(case)
    slot = ((position // 16 * 5 + 7) % 144) * 16 + position % 16
    for rank in range(2):
        for stage, name in enumerate(STAGES):
            left, right = arrays[0][rank][stage], arrays[1][rank][stage]
            width = 4 if stage == 6 else 2
            if stage in (3, 4):
                start = slot * 1024
                finite(left[start:start + 1024], 2); finite(right[start:start + 1024], 2)
            else:
                finite(left, width); finite(right, width)
            mismatch = sum(left[i:i + width] != right[i:i + width] for i in range(0, len(left), width))
            rows.append(dict(rank=rank, stage=name, elements=len(left) // width, word_bytes=width,
                             mismatches=mismatch, baseline_sha256=sha(left), tiles_sha256=sha(right)))
    require(type(value['stages']) is list and len(value['stages']) == 14, 'fourteen stage rows')
    for row in value['stages']:
        keys(row, 'rank stage elements word_bytes mismatches baseline_sha256 tiles_sha256')
        for key in ('rank', 'elements', 'word_bytes', 'mismatches'): uint(row[key])
    require(value['stages'] == rows and all(row['mismatches'] == 0 for row in rows),
            'all fourteen complete stage bit comparisons, including whole KV caches')
    return dict(capture_bytes=CASE_CAPTURE_BYTES, compared_rows=rows, observed_owners=owners,
                all_typed_terminal_words_checked=True, native_close_reported=True,
                bitwise_match=True, independent_numerical_acceptance=False,
                full_model_correctness=False, performance_claim=False, production_authority=False)
