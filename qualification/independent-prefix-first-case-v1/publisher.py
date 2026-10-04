"""Publish the first actual independent-profile case; byte audit, no reference rerun."""
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
CASE = 'genuine-pos0'
GPU_DIRECTORY = E / 'prefix-independent-profile-gpu-v228-v1' / CASE
GPU = dict(path=str(GPU_DIRECTORY / 'complete.json'), bytes=259746,
    sha256='41d4674c3f6665e24a38d08074a6d91be5118d012f51bce87298d8b69eccd76f')
CPU = dict(path=str(E / 'prefix-independent-numerical-v228-v1/complete.json'), bytes=487421,
    sha256='6185d809002e61f53edb1a47f3fbf58cfa815e23daae70c108a7d12dfeeec62e')
OUT = Path('/home/harsh/ferric-p227-integration/qualification/independent-prefix-first-case-v1')
README = L / 'proposals/public-independent-first-case-v1/README.md'
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


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and len(sys.argv) == 1,
            'unoptimized fixed-input publisher')
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

    gpu_raw, cpu_raw = retained(GPU), retained(CPU)
    gpu, cpu = parse(gpu_raw), parse(cpu_raw)
    require(gpu['schema'] == 'ferric-p228-independent-gpu-observation-v1'
        and gpu['passed'] is True and gpu['case'] == CASE and type(gpu['case_index']) is int
        and gpu['case_index'] == 0 and type(gpu['native_attempts']) is int and gpu['native_attempts'] == 1
        and gpu['retries'] == 0 and gpu['failures'] == [], 'actual first closed GPU case')
    require(cpu['schema'] == 'ferric-p228-independent-profile-numerical-cli-result-v1'
        and cpu['case'] == CASE and cpu['observation'] == GPU
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
                if path.is_relative_to(GPU_DIRECTORY):
                    previous = case_pins.setdefault(value['path'], value)
                    require(previous == value, 'unique consistent case artifact identity')
            else:
                for nested in value.values():
                    case_records(nested)
        elif type(value) is list:
            for nested in value:
                case_records(nested)

    case_records(GPU); case_records(gpu); case_records(cpu)
    expected_names = {str(Path(path).relative_to(GPU_DIRECTORY)) for path in case_pins}
    require(len(expected_names) == 61 and census(L / GPU_DIRECTORY.relative_to(E)) == expected_names,
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
            require(record['path'] == str(GPU_DIRECTORY / name / filename), 'leaf-owned member path')
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
                and audit['topology']['path'] == str(GPU_DIRECTORY / (name + '-topology.json')), 'exact audit identity')
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
        and native['immutable_input_readbacks_match'] is True and native['position'] == 0
        and native['history_kind'] == 'genuine' and native['request_sha256'] == gpu['request']['sha256']
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
                require(len(words) == 284 and len(owners) == 130 and set(owners) == set(range(64))
                    and gpu['checked']['observed_owners'][profile_index][rank] ==
                        dict(owners=owners, useful_workgroups=64), 'observed useful candidate workgroups, not a progress theorem')
                observed_candidate_owners.append(64)
    require(len(captures) == len(gpu['retained_captures']) == 4, 'four full captures')

    conditional = cpu['conditional']
    require(conditional['schema'] == 'ferric-p228-independent-profile-conditional-observation-v1'
        and conditional['conditional_operator_checks_passed'] is True and conditional['case'] == CASE
        and conditional['request'] == gpu['request'] and conditional['binary'] == gpu['binary']
        and conditional['inspection_result'] == gpu['leaves']['inspection']['result']
        and conditional['native_result'] == gpu['leaves']['native']['result']
        and conditional['closed_capture_checks'] == gpu['checked']
        and conditional['profile_children'] == children
        and conditional['retained_profile_files'] == gpu['retained_profile_files'], 'CPU replay joins the actual GPU case')
    inputs_raw, review_raw = retained(cpu['inputs']), retained(cpu['arithmetic_review'])
    inputs, review = parse(inputs_raw), parse(review_raw)
    require(inputs['schema'] == 'ferric-p228-independent-profile-numerical-cli-inputs-v1'
        and inputs['prepared'] == gpu['prepared'] and inputs['observation'] == GPU
        and inputs['arithmetic_review'] == cpu['arithmetic_review']
        and review['case'] == CASE and review['request'] == gpu['request']
        and review['tiles_image_sha256'] == gpu['object']['sha256']
        and review['compiler_complete'] == gpu['compiler_complete']
        and review['candidate_cpu_receipt'] == gpu['candidate_cpu_receipt']
        and review['arithmetic_evidence'] == gpu['arithmetic_evidence'], 'actual inputs and case-scoped arithmetic assumptions')
    require(cpu['assumptions'] == dict(prefix=review['prefix_assumptions'],
        attention_output=review['attention_output_assumptions'], remaining_limitations=review['remaining_limitations']),
        'unchanged recorded mathematical assumptions and limitations')
    rows = []
    require([item['profile'] for item in conditional['profiles']] == list(PROFILES), 'two independent numerical profiles')
    for profile_result in conditional['profiles']:
        profile, checked = profile_result['profile'], profile_result['checked']
        require(profile_result['conditional_operator_checks_passed'] is True and profile_result['error'] is None
            and checked['conditional_operator_checks_passed'] is True and checked['profile'] == profile
            and checked['case'] == CASE and [row['rank'] for row in checked['rows']] == [0, 1]
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
            require(attention == dict(exact=2048, tolerated=0, max_abs_error=0.0, max_bf16_steps=0)
                and output['elements'] == 4096 and type(output['max_bound_ratio']) is float
                and math.isfinite(output['max_bound_ratio']) and 0 <= output['max_bound_ratio'] <= 1,
                'exact attention and output partial inside unchanged bound')
            rows.append(dict(profile=profile, rank=rank, capture=row['capture'],
                normalization=prefix['normalization'], qkv_projection=prefix['qkv_projection'],
                headnorm_rope_append=prefix['headnorm_rope_append'], attention=attention, output_partial=output))
    require(len(rows) == 4 and sum(row['attention']['exact'] for row in rows) == 8192, 'complete four-row numerical census')
    for record in case_pins.values():
        require(cpu['input_pins'].get(record['path']) == record, 'remote CPU replay authenticated each retained GPU artifact')
    retained(gpu['controller']); retained(cpu['controller'])

    summary = dict(schema='FerricIndependentPrefixFirstCaseQualificationV1', date='2026-10-03',
        case=CASE, history_kind='genuine', position=0, host='smci350-rck-g03-b19-03', devices=DEVICES,
        actual_gpu_observation_passed=True, actual_remote_conditional_reference_checks_passed=True,
        local_byte_audit_passed=True, local_reference_rerun=False, gpu_observation=GPU, numerical_receipt=CPU,
        numerical_inputs=cpu['inputs'], arithmetic_review=cpu['arithmetic_review'],
        image=gpu['object'], binary=gpu['binary'], compiler_complete=gpu['compiler_complete'], native_complete=gpu['native_complete'],
        gpu_case_artifacts=case_pins, gpu_case_files=61, retained_capture_bytes=19030016,
        owned_leaves=8, pre_audits=3, post_audits=3, child_sidecars=10, native_attempts=1,
        profile_attempts=2, retries=0, useful_candidate_workgroups_per_rank=observed_candidate_owners,
        owner_counts_are_observations_not_scheduler_guarantees=True, conditional_rows=rows,
        normalization_violations=0, qkv_projection_violations=0, attention_exact_bf16_elements=8192,
        attention_tolerated_elements=0, output_max_bound_ratio=max(row['output_partial']['max_bound_ratio'] for row in rows),
        output_max_abs_error_upper=max(row['output_partial']['max_abs_error_upper'] for row in rows),
        assumptions=cpu['assumptions'], limitations=cpu['limitations'] + [
            'Only genuine-pos0 is qualified here; the other five selected cases are not included.',
            'This local publisher rehashes retained artifacts and joins reports; it does not rerun the independent mathematical references.',
            'The actual mathematical checks ran on MI350 in a separate CPU leaf after the GPU case and its post-audits.',
            'Capture bodies remain in session evidence and are not included in Git.',
            'Observed use of all 64 candidate workgroups per rank does not prove scheduler fairness, universal progress or performance.',
        ], performance_claim=False, production_authority=False, full_model_acceptance=False,
        full_prefix_acceptance=False, universal_arithmetic_premises_discharged=False,
        independent_numerical_acceptance=False, paired_comparison_performed=False)
    outputs = {'gpu-observation.json': gpu_raw, 'numerical-observation.json': cpu_raw,
        'numerical-inputs.json': inputs_raw, 'arithmetic-review.json': review_raw,
        'README.md': read(README), 'publisher.py': read(Path(__file__).resolve()),
        'result.json': (json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')}
    for path, raw in local_bytes.items():
        require(read(path) == raw, 'retained bytes changed before publication')
    require(OUT.parent.resolve(strict=True) == OUT.parent, 'canonical existing qualification parent')
    if os.path.lexists(OUT):
        require(census(OUT) == set(outputs), 'existing publication census differs')
        for name, raw in outputs.items():
            require(read(OUT / name) == raw, 'existing publication differs')
    else:
        OUT.mkdir(mode=0o755)
        for name, raw in sorted(outputs.items()):
            with (OUT / name).open('xb') as stream:
                stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        require(census(OUT) == set(outputs), 'seven published records, no capture bodies')
        for name, raw in outputs.items():
            require(read(OUT / name) == raw, 'published artifact changed')
    print(json.dumps(dict(output=str(OUT), files=len(outputs), result=identity(outputs['result.json']),
        output_max_bound_ratio=summary['output_max_bound_ratio'])))


if __name__ == '__main__':
    main()
