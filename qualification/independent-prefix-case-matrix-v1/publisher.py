"""Publish six retained independent-profile cases; byte audits, no reference rerun."""
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import sys

L = Path(__file__).resolve().parent
R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
CASES = (
    dict(case='genuine-pos0', history_kind='genuine', position=0,
        gpu=dict(bytes=259746, sha256='41d4674c3f6665e24a38d08074a6d91be5118d012f51bce87298d8b69eccd76f'),
        cpu=dict(bytes=487421, sha256='6185d809002e61f53edb1a47f3fbf58cfa815e23daae70c108a7d12dfeeec62e')),
    dict(case='genuine-pos4', history_kind='genuine', position=4,
        gpu=dict(bytes=286191, sha256='1f750946bbb066417efdc8c080e57f26135cb96ee2a0ae45c20e9cb743da95c7'),
        cpu=dict(bytes=541009, sha256='24b4a5ea414858e362ae0a6c2e6e1bc5cb1ad1d144c3e0ff8cac7289d232504a')),
    dict(case='patterned-pos15', history_kind='patterned', position=15,
        gpu=dict(bytes=313237, sha256='fc0933062e18fe002f4486ab9fdb468778de1005f887d42fffa28bede4a38afe'),
        cpu=dict(bytes=595440, sha256='eec4e9dd9d501775ec1d1c9dfd2fb778d666d91631256994402a9618fb97b9f7')),
    dict(case='patterned-pos16', history_kind='patterned', position=16,
        gpu=dict(bytes=340058, sha256='bbf1f86d90c9e45e6a970480f0cf7daaccd35a3e2893f35dbda0ff9337b8e64e'),
        cpu=dict(bytes=649694, sha256='c4a35d01f6ec4f838e3fdd4d2fbf6327000caf425049e9b419a1125690411c1c')),
    dict(case='patterned-pos2047', history_kind='patterned', position=2047,
        gpu=dict(bytes=367266, sha256='f3ba008cb8358021fb01f15a88ae7ab61490c8e49e2070c1df705d4e0ddf1bf4'),
        cpu=dict(bytes=704575, sha256='d3df53b2979917e1b72c03003c5407bcd1ddd43be65438942cc631c4588f0c66')),
    dict(case='patterned-pos2048', history_kind='patterned', position=2048,
        gpu=dict(bytes=394331, sha256='e69d9e7b4587e3a520b354dd40b013aa1663b521b02bed573b468085b634d6ff'),
        cpu=dict(bytes=759320, sha256='f4cf48160c02c7acbbcafc785f57234cd7cffe80ee8fc5c5c2cac170e959b0f5')),
)
OUT = Path('/home/harsh/ferric-p227-integration/qualification/independent-prefix-case-matrix-v1')
README = L / 'proposals/public-independent-case-matrix-v1/README.md'
POLICIES = dict(
    prefix_policy_sha256='c3f085ad8230f4dcac7a36dd2a72872f00bc454f0996467af90ea4fde69a0f8d',
    attention_policy_sha256='438b10d7cf2bdc7cd693024edcfa7f3f84929e4a852787d72964c43263cd0fc2',
    output_reference_sha256='34d86aa6e02f872b219cbb105c335633ca0dc8ed666e97a29376606cd97137cc',
)
ATTENTION_POLICY = dict(exact_position_zero=True, bf16_steps=1, cancellation_coefficient=5e-5,
    acceptance='exact at position 0; otherwise steps <= 1 OR abs_error <= 5e-5 * per_kv_head_max_abs_V',
    adaptive_tolerance=False)
PROFILES = ('baseline_v5', 'tiles_v6')
LABELS = ('baseline-v5', 'tiles-v6')
EXTENTS = (8192, 6144, 4096, 2359296, 2359296, 4096, 16384)
DEVICES = [16366993098680759275, 10838076764495710945]
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
    OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
    HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
SMI = ['/opt/rocm/bin/amd-smi', 'process', '--gpu', '0000:05:00.0', '0000:15:00.0', '--json']


def require(value, message):
    if not value:
        raise RuntimeError(message)


def identity(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path):
    require(path.resolve(strict=True) == path, 'canonical local artifact: ' + str(path))
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno())
        raw = stream.read((8 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda row: (row.st_dev, row.st_ino, row.st_size, row.st_mtime_ns, row.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and len(raw) == before.st_size <= 8 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat()), 'stable bounded artifact')
    return raw


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def filepin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}, 'exact FilePin')
    path = Path(value['path'])
    require(path.is_absolute() and '..' not in path.parts and str(path) == value['path']
        and type(value['bytes']) is int and 0 <= value['bytes'] <= 1 << 30
        and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'canonical FilePin')
    return path


def census(directory):
    require(directory.resolve(strict=True) == directory and directory.is_dir(), 'canonical directory')
    result = set()
    for path in directory.rglob('*'):
        require(not path.is_symlink() and (path.is_dir() or path.is_file()), 'no aliases or special files')
        if path.is_file():
            result.add(str(path.relative_to(directory)))
    return result


def audit_case(index, specification):
    case = specification['case']
    history_kind, position = specification['history_kind'], specification['position']
    gpu_directory = E / 'prefix-independent-profile-gpu-v228-v1' / case
    gpu_pin = dict(path=str(gpu_directory / 'complete.json'), **specification['gpu'])
    cpu_pin = dict(path=str(E / ('prefix-independent-numerical-v228-v' + str(index + 1)) / 'complete.json'),
                   **specification['cpu'])
    local_bytes, case_pins = {}, {}

    def retained(record):
        original = filepin(record)
        require(original.is_relative_to(E), 'retained task evidence only')
        relative = original.relative_to(E)
        local = L / relative
        if relative.parts[0].startswith('p228-'):
            local = L / 'proposals' / relative
        raw = read(local)
        require(identity(raw) == {key: record[key] for key in ('bytes', 'sha256')}, 'exact retained identity: ' + str(local))
        if local in local_bytes:
            require(local_bytes[local] == raw, 'retained artifact changed')
        local_bytes[local] = raw
        return raw

    gpu_raw, cpu_raw = retained(gpu_pin), retained(cpu_pin)
    gpu, cpu = parse(gpu_raw), parse(cpu_raw)
    require(gpu['schema'] == 'ferric-p228-independent-gpu-observation-v1'
        and gpu['passed'] is True and gpu['case'] == case and type(gpu['case_index']) is int
        and gpu['case_index'] == index and type(gpu['native_attempts']) is int and gpu['native_attempts'] == 1
        and gpu['retries'] == 0 and gpu['failures'] == [], 'actual closed GPU case')
    require(cpu['schema'] == 'ferric-p228-independent-profile-numerical-cli-result-v1'
        and cpu['case'] == case and cpu['observation'] == gpu_pin
        and cpu['conditional_operator_checks_passed'] is True
        and cpu['native_attempts_replayed'] == 1 and cpu['profile_attempts_replayed'] == 2
        and cpu['retries'] == 0, 'actual separately completed independent CPU checks')
    for value, names in ((gpu, ('production_authority', 'performance_claim', 'independent_numerical_acceptance',
                               'full_model_correctness')),
                         (cpu, ('gpu_execution', 'production_authority', 'runtime_premises_discharged',
                            'arithmetic_prerequisites_verified', 'independent_numerical_acceptance',
                            'full_prefix_acceptance', 'full_model_acceptance', 'performance_claim',
                            'top_level_observer_reaping_verified', 'paired_comparison_performed'))):
        require(all(value[name] is False for name in names), 'preserved evidence boundaries')

    def case_records(value):
        if type(value) is dict:
            if set(value) == {'path', 'bytes', 'sha256'}:
                path = filepin(value)
                if path.is_relative_to(gpu_directory):
                    previous = case_pins.setdefault(value['path'], value)
                    require(previous == value, 'unique consistent case artifact identity')
            else:
                for nested in value.values():
                    case_records(nested)
        elif type(value) is list:
            for nested in value:
                case_records(nested)

    case_records(gpu_pin); case_records(gpu); case_records(cpu)
    expected_names = {str(Path(path).relative_to(gpu_directory)) for path in case_pins}
    require(len(expected_names) == 61 and census(L / gpu_directory.relative_to(E)) == expected_names,
            'complete actual sixty-one-file GPU case, no extra artifacts')
    case_data = {name: retained(record) for name, record in case_pins.items()}
    require(sum(record['bytes'] for record in case_pins.values()) <= 32 << 20, 'unchanged case evidence cap')
    def case_doc(record):
        require(case_pins.get(record['path']) == record, 'authenticated case JSON')
        return parse(case_data[record['path']])

    names = {'inspection', 'native', *('before-' + str(i) for i in range(3)), *('after-' + str(i) for i in range(3))}
    require(set(gpu['leaves']) == names, 'all eight owned leaves')
    leaves = {}
    for name, row in gpu['leaves'].items():
        files = row['retained_files']
        require(set(files) == {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
            and row['result'] == files['result.json'], 'exact retained leaf roster')
        result = case_doc(row['result']); command = case_doc(files['command.json']); started = case_doc(files['started.json'])
        for filename, record in files.items():
            require(record['path'] == str(gpu_directory / name / filename), 'leaf-owned member path')
            if filename != 'result.json':
                require(result[filename.removesuffix('.json')] == record, 'result/member join')
        require(type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
            and result['owned_groups_absent'] is True and result['owned_processes_reaped'] is True
            and result['cleanup_signalled'] is False and case_data[files['stderr']['path']] == b''
            and started['command_sha256'] == files['command.json']['sha256'], 'natural owned leaf completion')
        is_native = name == 'native'
        if name in ('inspection', 'native'):
            mode = ('--execute-reviewed-engineering-prefix-independent-profiles-v1' if is_native
                    else '--inspect-independent-profiles-v1')
            argv = [gpu['binary']['path'], mode, gpu['request']['path'], gpu['request']['sha256']]
            seconds = 180
        else:
            argv, seconds = SMI, 30
        require(command == dict(argv=argv, env=ENV, cwd=str(R), deadline_seconds=seconds,
            affinity=[8, 9], nice=10, address_space_bytes=12 << 30, stream_cap_bytes=8 << 20,
            gpu_execution_requested=is_native) and result['gpu_execution_requested'] is is_native,
            'original command and resource envelope')
        leaves[name] = result

    topologies = []
    for phase in ('before', 'after'):
        audits = gpu[phase + '_audits']
        require(type(audits) is list and len(audits) == 3, 'three ordered audits per side')
        for index, audit in enumerate(audits):
            name = phase + '-' + str(index)
            require(audit['process_result'] == gpu['leaves'][name]['result']
                and audit['topology']['path'] == str(gpu_directory / (name + '-topology.json')), 'exact audit identity')
            process = case_doc(leaves[name]['stdout'])
            require(process == [dict(gpu=i, process_list=[dict(process_info='No running processes detected')])
                                for i in range(2)], 'empty process roster for both selected GPUs')
            devices = case_doc(audit['topology'])['devices']
            require([row['unique_id'] for row in devices] == DEVICES
                and all(row['gpu_busy'] == row['memory_busy'] == 0 for row in devices), 'idle exact physical GPU pair')
            topologies.append([{key: value for key, value in row.items()
                if key not in ('gpu_busy', 'memory_busy', 'vram_used')} for row in devices])
    require(all(row == topologies[0] for row in topologies), 'same device topology in all six audits')

    native = case_doc(leaves['native']['stdout']); inspected = case_doc(leaves['inspection']['stdout'])
    require(native['schema'] == 'fe2o3-qwen-prefix-tiles-independent-profiles-observation-v1'
        and inspected['schema'] == 'fe2o3-qwen-prefix-tiles-independent-profiles-inspection-v1'
        and native['success'] is True and native['completed_and_closed'] is True
        and native['immutable_input_readbacks_match'] is True and native['position'] == position
        and native['history_kind'] == history_kind and native['request_sha256'] == gpu['request']['sha256']
        and native['paired_comparison_performed'] is False and native['bitwise_match'] is False,
        'new independent parent report, never paired-parity acceptance')
    require(len(gpu['retained_profile_files']) == 10 and len(native['captures']) == 2
        and [row['profile'] for row in native['profiles']] == list(PROFILES), 'two native profiles and ten child sidecars')
    children = gpu['profile_children']
    require(children['profile_attempts'] == 2 and children['retries'] == 0
        and children['shared_outer_process_group'] is True, 'shared owned profile process group')
    observed_candidate_owners = []
    captures = {}
    for profile_index, (label, profile) in enumerate(zip(LABELS, PROFILES)):
        child = children['profiles'][profile_index]
        require(child['profile'] == profile and native['profiles'][profile_index]['closed'] is True,
                'ordered naturally closed profile')
        records = {kind: gpu['retained_profile_files']['prefix-' + label + '-child-' + kind]
            for kind in ('command.json', 'started.json', 'stdout.bin', 'stderr.bin', 'result.json')}
        require(child['files'] == records, 'all five exact child sidecars')
        child_out, child_result = case_doc(records['stdout.bin']), case_doc(records['result.json'])
        require(child_result['passed'] is True and child_result['wait_returned'] is True
            and type(child_result['exit_code']) is int and child_result['exit_code'] == 0
            and child_result['signal'] is None and child_result['error'] is None
            and child_result['stdout'] == records['stdout.bin'] and child_result['stderr'] == records['stderr.bin']
            and case_data[records['stderr.bin']['path']] == b''
            and child_out['captures'] == native['captures'][profile_index]
            and child_out['states'] == native['profiles'][profile_index]['states']
            and child_out['progress']['closed'] is True and child_out['paired_comparison'] is False,
            'child wait/Close/capture identity')
        for rank, record in enumerate(native['captures'][profile_index]):
            filename = label + '-rank' + str(rank) + '.bin'
            require(record == gpu['retained_captures'][filename] and record['bytes'] == 4757504,
                    'entire rank capture identity and extent')
            raw, offset, stage_hashes = case_data[record['path']], 0, []
            for extent in EXTENTS:
                stage_hashes.append(hashlib.sha256(raw[offset:offset + extent]).hexdigest()); offset += extent
            require(offset == len(raw) and stage_hashes == gpu['checked']['stage_sha256'][profile_index][rank],
                    'all seven complete stage hashes')
            captures[(profile, rank)] = record
            if profile_index == 1:
                words = native['profiles'][profile_index]['states'][rank]
                owners = [word - 1 for word in words[24:154]]
                require(len(words) == 284 and len(owners) == 130
                    and all(type(owner) is int and 0 <= owner < 64 for owner in owners)
                    and gpu['checked']['observed_owners'][profile_index][rank] ==
                        dict(owners=owners, useful_workgroups=len(set(owners))),
                    'observed useful candidate workgroups, not a progress theorem')
                observed_candidate_owners.append(len(set(owners)))
    require(len(captures) == len(gpu['retained_captures']) == 4, 'four full captures')

    conditional = cpu['conditional']
    require(conditional['schema'] == 'ferric-p228-independent-profile-conditional-observation-v1'
        and conditional['conditional_operator_checks_passed'] is True and conditional['case'] == case
        and conditional['request'] == gpu['request'] and conditional['binary'] == gpu['binary']
        and conditional['inspection_result'] == gpu['leaves']['inspection']['result']
        and conditional['native_result'] == gpu['leaves']['native']['result']
        and conditional['closed_capture_checks'] == gpu['checked']
        and conditional['profile_children'] == children
        and conditional['retained_profile_files'] == gpu['retained_profile_files'], 'CPU replay joins the actual GPU case')
    inputs_raw, review_raw = retained(cpu['inputs']), retained(cpu['arithmetic_review'])
    inputs, review = parse(inputs_raw), parse(review_raw)
    require(inputs['schema'] == 'ferric-p228-independent-profile-numerical-cli-inputs-v1'
        and inputs['prepared'] == gpu['prepared'] and inputs['observation'] == gpu_pin
        and inputs['arithmetic_review'] == cpu['arithmetic_review']
        and review['case'] == case and review['request'] == gpu['request']
        and review['tiles_image_sha256'] == gpu['object']['sha256']
        and review['compiler_complete'] == gpu['compiler_complete']
        and review['candidate_cpu_receipt'] == gpu['candidate_cpu_receipt']
        and review['arithmetic_evidence'] == gpu['arithmetic_evidence'], 'actual inputs and case-scoped arithmetic assumptions')
    require(all(review[key] == digest for key, digest in POLICIES.items()), 'unchanged recorded numerical policies')
    require(cpu['assumptions'] == dict(prefix=review['prefix_assumptions'],
        attention_output=review['attention_output_assumptions'], remaining_limitations=review['remaining_limitations']),
        'unchanged recorded mathematical assumptions and limitations')
    rows = []
    require([item['profile'] for item in conditional['profiles']] == list(PROFILES), 'two independent numerical profiles')
    for profile_result in conditional['profiles']:
        profile, checked = profile_result['profile'], profile_result['checked']
        require(profile_result['conditional_operator_checks_passed'] is True and profile_result['error'] is None
            and checked['conditional_operator_checks_passed'] is True and checked['profile'] == profile
            and checked['case'] == case and [row['rank'] for row in checked['rows']] == [0, 1]
            and checked['prefix_policy_sha256'] == review['prefix_policy_sha256']
            and checked['attention_policy_sha256'] == review['attention_policy_sha256']
            and checked['output_reference_sha256'] == review['output_reference_sha256'], 'actual unchanged independent references')
        for row in checked['rows']:
            rank = row['rank']; prefix, attention, output = row['prefix'], row['attention'], row['output_partial']
            require(row['profile'] == profile and row['capture'] == captures[(profile, rank)]
                and row['stage_sha256'] == gpu['checked']['stage_sha256'][PROFILES.index(profile)][rank]
                and row['conditioning']['output_weights'] == inputs['output_weights'][rank]
                and prefix['normalization']['mismatches'] == prefix['qkv_projection']['mismatches'] == 0
                and prefix['conditional_on_actual_preceding_stage'] is True
                and prefix['headnorm_rope_append']['exact_current_value_append'] is True,
                'four actual profile/rank rows with zero norm/QKV violations')
            require(set(attention) == {'exact', 'tolerated', 'max_abs_error', 'max_bf16_steps'}
                and all(type(attention[key]) is int and attention[key] >= 0
                        for key in ('exact', 'tolerated', 'max_bf16_steps'))
                and attention['exact'] + attention['tolerated'] == 2048
                and type(attention['max_abs_error']) in (int, float)
                and math.isfinite(attention['max_abs_error']) and attention['max_abs_error'] >= 0,
                'actual fixed-policy attention result census, not a new tolerance')
            if position == 0:
                require(attention == dict(exact=2048, tolerated=0, max_abs_error=0.0, max_bf16_steps=0),
                        'position zero remains exactly rounded')
            require(output['elements'] == 4096 and type(output['max_bound_ratio']) in (int, float)
                and math.isfinite(output['max_bound_ratio']) and 0 <= output['max_bound_ratio'] <= 1
                and type(output['max_abs_error_upper']) in (int, float)
                and math.isfinite(output['max_abs_error_upper']) and output['max_abs_error_upper'] >= 0,
                'output partial inside unchanged bound')
            rows.append(dict(profile=profile, rank=rank, capture=row['capture'],
                normalization=prefix['normalization'], qkv_projection=prefix['qkv_projection'],
                headnorm_rope_append=prefix['headnorm_rope_append'], attention=attention, output_partial=output))
    require(len(rows) == 4 and sum(row['attention']['exact'] + row['attention']['tolerated'] for row in rows) == 8192,
            'complete four-row numerical census')
    for record in case_pins.values():
        require(cpu['input_pins'].get(record['path']) == record, 'remote CPU replay authenticated each retained GPU artifact')
    retained(gpu['controller']); retained(cpu['controller'])

    summary = dict(case=case, history_kind=history_kind, position=position,
        actual_gpu_observation_passed=True, actual_remote_conditional_reference_checks_passed=True,
        local_byte_audit_passed=True, local_reference_rerun=False, gpu_observation=gpu_pin, numerical_receipt=cpu_pin,
        numerical_inputs=cpu['inputs'], arithmetic_review=cpu['arithmetic_review'],
        image=gpu['object'], binary=gpu['binary'], compiler_complete=gpu['compiler_complete'],
        native_complete=gpu['native_complete'], prepared=gpu['prepared'], policies=POLICIES,
        gpu_case_artifact_census_sha256=hashlib.sha256(
            json.dumps(case_pins, sort_keys=True, separators=(',', ':')).encode('ascii')).hexdigest(),
        gpu_case_files=61, gpu_case_bytes=sum(record['bytes'] for record in case_pins.values()), retained_capture_bytes=19030016,
        owned_leaves=8, pre_audits=3, post_audits=3, child_sidecars=10, native_attempts=1,
        profile_attempts=2, retries=0, useful_candidate_workgroups_per_rank=observed_candidate_owners,
        owner_counts_are_observations_not_scheduler_guarantees=True,
        profile_rows=[dict(profile=row['profile'], rank=row['rank'], capture=row['capture'],
            normalization_violations=row['normalization']['mismatches'],
            qkv_projection_violations=row['qkv_projection']['mismatches'],
            attention=row['attention'], output_partial=row['output_partial']) for row in rows],
        normalization_violations=0, qkv_projection_violations=0,
        attention_exact_bf16_elements=sum(row['attention']['exact'] for row in rows),
        attention_tolerated_elements=sum(row['attention']['tolerated'] for row in rows),
        attention_max_bf16_steps=max(row['attention']['max_bf16_steps'] for row in rows),
        attention_max_abs_error=max(row['attention']['max_abs_error'] for row in rows),
        output_max_bound_ratio=max(row['output_partial']['max_bound_ratio'] for row in rows),
        output_max_abs_error_upper=max(row['output_partial']['max_abs_error_upper'] for row in rows),
        assumptions=cpu['assumptions'], limitations=cpu['limitations'])
    outputs = {'gpu-observation.json': gpu_raw, 'numerical-observation.json': cpu_raw,
               'numerical-inputs.json': inputs_raw, 'arithmetic-review.json': review_raw}
    for path, raw in local_bytes.items():
        require(read(path) == raw, 'retained case bytes changed after audit')
    return summary, outputs, local_bytes


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and len(sys.argv) == 1,
            'unoptimized fixed-input publisher')
    require([(row['case'], row['history_kind'], row['position']) for row in CASES] == [
        ('genuine-pos0', 'genuine', 0), ('genuine-pos4', 'genuine', 4),
        ('patterned-pos15', 'patterned', 15), ('patterned-pos16', 'patterned', 16),
        ('patterned-pos2047', 'patterned', 2047), ('patterned-pos2048', 'patterned', 2048)],
        'closed ordered six-case matrix')
    for row in CASES:
        for role in ('gpu', 'cpu'):
            record = row[role]
            require(type(record) is dict and set(record) == {'bytes', 'sha256'}
                and type(record['bytes']) is int and 0 < record['bytes'] <= 8 << 20
                and type(record['sha256']) is str and re.fullmatch('[0-9a-f]{64}', record['sha256']),
                'actual final receipt pin is still missing: ' + row['case'] + '/' + role)
    cases, outputs, retained_inputs = [], {}, {}
    for index, specification in enumerate(CASES):
        summary, records, retained_case = audit_case(index, specification)
        cases.append(summary)
        for name, raw in records.items():
            outputs['cases/' + specification['case'] + '/' + name] = raw
        for path, raw in retained_case.items():
            require(path not in retained_inputs or retained_inputs[path] == raw, 'unchanged shared matrix artifact')
            retained_inputs[path] = raw
    for key in ('image', 'binary', 'compiler_complete', 'native_complete', 'prepared', 'policies'):
        require(all(case[key] == cases[0][key] for case in cases), 'same qualified generation across six cases: ' + key)
    result = dict(schema='FerricIndependentPrefixCaseMatrixQualificationV1', date='2026-10-03',
        passed=True, host='smci350-rck-g03-b19-03', devices=DEVICES, cases=cases, cases_passed=6,
        gpu_case_files=366, retained_capture_bytes=114180096, owned_leaves=48, pre_audits=18, post_audits=18,
        child_sidecars=60, native_attempts=6, profile_attempts=12, retries=0,
        policies=POLICIES, recorded_attention_policy=ATTENTION_POLICY,
        normalization_violations=sum(case['normalization_violations'] for case in cases),
        qkv_projection_violations=sum(case['qkv_projection_violations'] for case in cases),
        attention_exact_bf16_elements=sum(case['attention_exact_bf16_elements'] for case in cases),
        attention_tolerated_elements=sum(case['attention_tolerated_elements'] for case in cases),
        attention_max_bf16_steps=max(case['attention_max_bf16_steps'] for case in cases),
        attention_max_abs_error=max(case['attention_max_abs_error'] for case in cases),
        output_max_bound_ratio=max(case['output_max_bound_ratio'] for case in cases),
        output_max_abs_error_upper=max(case['output_max_abs_error_upper'] for case in cases),
        actual_remote_conditional_reference_checks_passed=True, local_byte_audit_passed=True,
        local_reference_rerun=False, publisher_gpu_execution=False, independent_numerical_acceptance=False,
        full_prefix_acceptance=False, full_model_acceptance=False, production_authority=False,
        universal_arithmetic_premises_discharged=False, performance_claim=False, paired_comparison_performed=False,
        limitations=[
            'These are six selected prefix cases, not a complete model or sustained decode workload.',
            'Actual independent mathematical checks ran on MI350 after each GPU case and its post-audits.',
            'This publisher hashes retained bytes and joins the recorded results; it does not rerun mathematical references.',
            'Nonzero-position attention uses the unchanged preregistered one-step OR cancellation bound; a large BF16 step count can pass through the cancellation clause.',
            'Arithmetic checks remain conditional on actual preceding stages, supplied rotary values and each recorded assumption review.',
            'Observed workgroup use does not establish scheduler fairness, universal progress, or performance.',
            'Large captures remain in session evidence and are not committed. No 700 tokens/s claim is made.',
        ])
    table = ['| Case | Norm / QKV Violations | Attention Exact / Tolerated | Max BF16 Steps | Max O Bound Ratio | Useful Candidate WGs (Ranks 0 / 1) |',
             '| --- | ---: | ---: | ---: | ---: | ---: |']
    for case in cases:
        table.append('| ' + case['case'] + ' | ' + str(case['normalization_violations']) + ' / '
            + str(case['qkv_projection_violations']) + ' | ' + str(case['attention_exact_bf16_elements'])
            + ' / ' + str(case['attention_tolerated_elements']) + ' | ' + str(case['attention_max_bf16_steps'])
            + ' | ' + format(case['output_max_bound_ratio'], '.9g') + ' | '
            + ' / '.join(str(count) for count in case['useful_candidate_workgroups_per_rank']) + ' |')
    readme = read(README).decode('ascii')
    require(readme.count('<!-- ACTUAL_CASE_MATRIX -->') == 1, 'one actual-data table insertion point')
    outputs['README.md'] = readme.replace('<!-- ACTUAL_CASE_MATRIX -->', '\n'.join(table)).encode('ascii')
    outputs['publisher.py'] = read(Path(__file__).resolve())
    outputs['result.json'] = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    for path, raw in retained_inputs.items():
        require(read(path) == raw, 'retained matrix bytes changed before publication')
    require(OUT.parent.resolve(strict=True) == OUT.parent, 'canonical existing qualification parent')
    if os.path.lexists(OUT):
        require(census(OUT) == set(outputs), 'existing publication census differs')
        for name, raw in outputs.items():
            require(read(OUT / name) == raw, 'existing publication differs')
    else:
        OUT.mkdir(mode=0o755)
        for name, raw in sorted(outputs.items()):
            path = OUT / name
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as stream:
                stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        require(census(OUT) == set(outputs) and len(outputs) == 27, 'complete matrix publication, no capture bodies')
        for name, raw in outputs.items():
            require(read(OUT / name) == raw, 'published artifact changed')
    print(json.dumps(dict(output=str(OUT), files=len(outputs), result=identity(outputs['result.json']),
        cases_passed=6, output_max_bound_ratio=result['output_max_bound_ratio'])))


if __name__ == '__main__':
    main()
