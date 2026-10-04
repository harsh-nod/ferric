"""Local-only assembly from actual receipts and explicit root review decisions."""
import ast
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = 'p228-silu-materialized-capture-gpu-v1'
CPU_LABEL = 'projection-residual-runtime-cpu-v228-v1'
CPU_SHA = 'bf1a12f78981d9ff9b8157e1dec6dca300752b238680e16380b98e9d1260bafb'
CAPTURE_LABEL = 'prefix-projection-residual-capture-gpu-v228-v1'
CAPTURE_SHA = '4f25030567c470062fd50862876bc35e93d080f1778c0be4ca36f2b39af4199a'
TF4_SHA = '00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073'
NAMES = dict(parent='ferric-qwen3-finite-projection-residual-layer-capture-engineering',
             worker='ferric-tp-peer-finite-engineering-worker-v1')
SEEN = {}
ORIGINALS = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed JSON fields')


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def parse(raw):
    return json.loads(raw, object_pairs_hook=unique,
        parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def fingerprint(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local input')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 32 << 20,
            'bounded regular unaliased input')
    with path.open('rb') as stream:
        raw = stream.read((32 << 20) + 1); opened = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_nlink)
    require(stamp(before) == stamp(opened) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input changed while reading')
    return raw, dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def local_document(name, digest):
    path = Path(name)
    require(path.is_relative_to(L) and re.fullmatch('[0-9a-f]{64}', digest), 'explicit retained local input digest')
    raw, pin = fingerprint(path)
    require(pin['sha256'] == digest and SEEN.setdefault(str(path), pin) == pin, 'actual supplied input hash')
    return parse(raw), pin


def body(pin):
    keys(pin, 'path bytes sha256')
    path = Path(pin['path'])
    require(path.is_absolute() and '..' not in path.parts and str(path) == pin['path']
        and path.is_relative_to(E) and type(pin['bytes']) is int and 0 <= pin['bytes'] <= 32 << 20
        and type(pin['sha256']) is str and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'original evidence FilePin')
    rel = path.relative_to(E)
    local = L / ('proposals' if rel.parts[0].startswith(('p227-', 'p228-')) else '') / rel
    if rel == Path('run_silu_materialized_capture_gpu_pure_p228_v1.py'):
        local = L / 'proposals/p228-silu-materialized-capture-harness-v1' / rel
    raw, retained = fingerprint(local)
    require(all(retained[k] == pin[k] for k in ('bytes', 'sha256'))
        and SEEN.setdefault(str(local), retained) == retained
        and ORIGINALS.setdefault(pin['path'], dict(original=pin, retained=retained)) == dict(original=pin, retained=retained),
        'original/retained body identity')
    return raw


def doc(pin):
    return parse(body(pin))


def known(label, digest):
    path = L / label / 'complete.json'
    value, retained = local_document(str(path), digest)
    original = dict(retained, path=str(E / label / 'complete.json'))
    require(doc(original) == value, 'known receipt identity')
    return value, original


PACKAGE_SHA = '71cae69a25ac82c53bdf2976ab53b9d9af615c5af4b8337c2e386beb44a8a18b'
DEVICES = [16366993098680759275, 10838076764495710945]


def constants(raw):
    names = {'MLP_CPU', 'BASELINE', 'INPUT_SCHEMA', 'REVIEW_SCHEMA', 'PACKAGE_SCHEMA', 'PURE_SCHEMA',
             'PLAN_FIELDS', 'REVIEW_FALSE', 'TOPICS', 'PACKAGE_FILES', 'PURE_TESTS', 'TEST_RUNNER_SHA'}
    result = {}
    for node in ast.parse(raw).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in names:
                result[name] = ast.literal_eval(node.value)
    require(set(result) == names, 'complete frozen intake literal contract')
    result['REVIEW_BINDINGS'] = [key for key in result['PLAN_FIELDS'].split()
        if key not in ('schema', 'output_label', 'capture_review', 'supervisor_tests', 'supervisor_test_sources')]
    return result


def content(pin):
    return pin['bytes'], pin['sha256']

def pure(config):
    manifest_pin = config['supervisor_manifest']
    require(manifest_pin['path'] == str(E / PACKAGE / 'manifest.json'), 'new supervisor package namespace')
    require(manifest_pin['sha256'] == PACKAGE_SHA, 'exact frozen capture package')
    manifest = doc(manifest_pin)
    files = {}
    for row in manifest['files']:
        keys(row, 'path bytes sha256')
        require(type(row['path']) is str and '/' not in row['path'] and row['path'] not in files, 'flat unique package member')
        files[row['path']] = body(dict(row, path=str(E / PACKAGE / row['path'])))
    K = constants(files['intake.py'])
    require(set(files) == K['PACKAGE_FILES'] and manifest['schema'] == K['PACKAGE_SCHEMA']
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == K['PURE_TESTS'] == 36,
        'exact source package and authored test census')
    record = config['supervisor_tests']; directory = Path(record['path']).parent
    require(directory.parent == E and Path(record['path']).name == 'complete.json'
        and re.fullmatch(r'silu-materialized-capture-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'actual pure namespace')
    value = doc(record)
    require(value['schema'] == K['PURE_SCHEMA'] and value['passed'] is True
        and type(value['tests']) is int and value['tests'] == 36
        and all(type(value[k]) is int and value[k] == 0 for k in ('errors', 'failures', 'skipped'))
        and value['manifest_sha256'] == manifest_pin['sha256']
        and value['controller_sha256'] == K['TEST_RUNNER_SHA']
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
        and all(value[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'actual bounded pure qualification only')
    for key, name in [('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'), ('transcript', 'tests.log')]:
        require(value[key]['path'] == str(directory / name), 'actual pure evidence namespace')
    expected = {row['path']: dict(row, path=str(E / PACKAGE / row['path'])) for row in manifest['files']}
    require(doc(value['sources_before']) == doc(value['sources_after']) == expected
        and value['source_sha256'] == value['sources_before']['sha256'], 'tested sources before and after')
    body(value['transcript'])
    require(value['test_inventory_before'] == value['test_inventory_after']
        and set(value['test_inventory_before']) == set(manifest['test_census'])
        and all(len(names) == len(set(names)) == manifest['test_census'][name]
            for name, names in value['test_inventory_before'].items()), 'actual named pure census')
    body(dict(path=str(E / 'run_silu_materialized_capture_gpu_pure_p228_v1.py'),
        bytes=7493, sha256=K['TEST_RUNNER_SHA']))
    return K, value


def decision(value, extra=''):
    keys(value, 'reviewed authority notes' + (' ' + extra if extra else ''))
    require(value['reviewed'] is True and value['authority'] == 'none', 'root-authored affirmative engineering review required')
    require(type(value['notes']) is str and 32 <= len(value['notes'].strip()) and len(value['notes'].encode()) <= 16384,
            'substantive supplied root notes')


def runtime(role, record, review_pin, binary, cpu_pin, cpu):
    review = doc(review_pin)
    decision({key: review[key] for key in ('reviewed', 'authority', 'notes')})
    directory = Path(record['path']).parent
    require(directory.parent == E and Path(record['path']).name == 'complete.json'
        and re.fullmatch('projection-residual-capture-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', directory.name),
        'new role runtime audit namespace')
    value = doc(record)
    require(value['schema'] == 'ferric-p227-prefix-runtime-audit-v1' and value['reviewed'] is False
        and value['authority'] == 'none' and value['gpu_execution'] is False
        and value['host'] == 'smci350-rck-g03-b19-03'
        and value['boot_id'] == '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a'
        and value['devices'] == [16366993098680759275, 10838076764495710945]
        and value['binary'] == binary and len(value['commands']) == len(value['owners']) == 2
        and value['source_pins'][cpu_pin['path']] == cpu_pin
        and value['source_pins'][binary['path']] == binary, 'actual current-host new-binary CPU-only audit')
    for index, name in enumerate(('readelf', 'ldd')):
        require(value['commands'][index]['path'] == str(directory / name / 'command.json')
            and value['owners'][index]['path'] == str(directory / name / 'owner.json'), 'direct audit command/owner')
        audit, owner, command = doc(value[name]), doc(value['owners'][index]), doc(value['commands'][index])
        outcome = owner['outcome']
        require(type(outcome['exit_code']) is int and outcome['exit_code'] == 0 and outcome['reason'] is None
            and outcome['cleanup_signalled'] is False and outcome['owned_groups_absent'] is True
            and outcome['owned_processes_reaped'] is True and owner['command'] == value['commands'][index]
            and owner['gpu_execution'] is False and outcome['deadline_seconds'] == 30, 'naturally exited and reaped audit')
        started = doc(owner['started']); identity = started['parent']
        require(identity['pid'] == identity['pgid'] == identity['sid'] and identity['uid'] == 9661
            and identity['ppid'] == started['supervisor_pid'] and identity['starttime'] > 0
            and any(row.get('identity') == identity and row.get('event') == 'owned' for row in outcome['lineage']),
            'actual owned audit process identity')
        argv = ['/usr/bin/readelf', '-d', binary['path']] if name == 'readelf' else ['/usr/bin/ldd', binary['path']]
        require(audit['exit_code'] == 0 and audit['argv'] == command['audit_argv'] == argv
            and command['argv'] == ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30', '--fsize=1048576', '--core=0', '--', *argv]
            and command['deadline_seconds'] == 30 and command['gpu_execution_requested'] is False
            and command['stream_cap_bytes'] == 1 << 20
            and all(command['env'][k] == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
            'unchanged bounded CPU-only readelf/ldd command')
        require(body(audit['stderr']) == b'', 'quiet audit stderr')
        text = body(audit['stdout']).decode()
        require(all(word not in text for word in ('not found', 'RPATH', 'RUNPATH')), 'no unresolved or embedded search-path libraries')
    for key in ('topology_before', 'topology_after', 'software_audit'):
        body(value[key])
    require(review['schema'] == 'ferric-p227-prefix-parity-runtime-review-v1'
        and review['production_authority'] is False and review['gpu_execution'] is False
        and all(review[key] == value[key] for key in ('host','boot_id','binary','readelf','ldd','libraries')),
        'existing reviewed runtime body matches actual audit; do not mint review')
    for pin in (cpu['controller'], cpu['raw']['sources-before.json'], cpu['raw']['sources-after.json'],
                cpu['raw']['parent-builds-stdout' if role == 'parent' else 'worker-build-stdout']):
        require(value['source_pins'][pin['path']] == pin, 'runtime audit binds selected CPU source/build records')
        body(pin)
    return body(review_pin)


def owned(value):
    require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
        and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
        and value['owned_processes_reaped'] is True, 'natural closed owned process')


def capture_leaves(value):
    root = E / CAPTURE_LABEL
    names = {'parent', *[side + '-' + str(n) for side in ('before','after') for n in range(3)]}
    require(set(value['leaves']) == names, 'original seven owned leaves')
    for name, leaf in value['leaves'].items():
        files = leaf['retained_files']
        require(set(files) == {'command.json','started.json','stdout','stderr','result.json'}
            and files['result.json'] == leaf['result'], 'complete original owned leaf')
        for filename, pin in files.items():
            require(pin['path'] == str(root / name / filename), 'original capture leaf namespace')
            body(pin)
        result = doc(leaf['result']); owned(result)
        for key in ('command','started','stdout','stderr'):
            filename = key + '.json' if key in ('command','started') else key
            require(result[key] == files[filename], 'original owner stream joins')
    empty = [dict(gpu=n, process_list=[dict(process_info='No running processes detected')]) for n in (0,1)]
    for side in ('before','after'):
        rows = value[side + '_audits']
        require(len(rows) == 3, 'three original idle audits per side')
        for index, row in enumerate(rows):
            name = side + '-' + str(index)
            require(row['process_result'] == value['leaves'][name]['result']
                and row['topology']['path'] == str(root / (name + '-topology.json')), 'original audit joins')
            result = doc(row['process_result'])
            require(result['gpu_execution_requested'] is False and body(result['stderr']) == b''
                and doc(result['stdout']) == empty, 'original exact-empty process records')
            sample = doc(row['topology'])
            require([d['unique_id'] for d in sample['devices']] == DEVICES
                and all(d['gpu_busy'] == d['memory_busy'] == 0 for d in sample['devices']),
                'recorded selected devices idle; not a fresh audit')
    require(value['checked']['native_closed'] is True and value['checked']['recorded_child_exit_zero'] is True
        and value['checked']['recorded_child_group_absent'] is True, 'original native Close/reap recorded')


def new_mlp(config, K, old_review):
    cpu, lower, owner = (doc(config[key]) for key in ('mlp_cpu','mlp_lowering_complete','mlp_lowering_owner'))
    require(content(config['mlp_cpu']) == K['MLP_CPU']
        and cpu['schema'] == 'ferric-p228-silu-materialized-cpu-result-v1'
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['source_unchanged'] is True and cpu['tests_passed'] == 38 and cpu['tests_ignored'] == 0,
        'actual CPU38 source generation')
    for key,label in (('mlp_lowering_complete','row-silu-materialized-checked-probe-v228-v'),
                      ('mlp_lowering_owner','silu-materialized-checked-probe-owner-v228-v')):
        path = Path(config[key]['path'])
        require(path.parent.parent == E and path.name == 'complete.json'
            and re.fullmatch(re.escape(label) + r'[1-9][0-9]{0,8}', path.parent.name), 'actual checked completion namespace')
    require(lower['schema'] == 'ferric-p228-silu-materialized-lowering-result-v1'
        and owner['schema'] == 'ferric-p228-silu-materialized-lowering-owned-result-v1', 'new checked lowering schemas')
    for value in (lower,owner):
        require(value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['candidate_cpu'] == config['mlp_cpu'] and value['source_manifest'] == cpu['overlay']
            and value['prior_lowering'] == cpu['prior_lowering'] == old_review['down2_provenance']['lowering'],
            'actual CPU/source/checked generation')
        require(all(value[k] is False for k in ('gpu_execution','numerical_acceptance','performance_claim','production_authority')),
                'no inherited arithmetic authority')
    require(owner['completion'] == config['mlp_lowering_complete']
        and lower['package_manifest'] == owner['package_manifest']
        and lower['compiler_generation'] == owner['compiler_generation'] == old_review['down2_provenance']['compiler_generation']
        and all(lower[key] is True for key in ('fresh_checked_lowering','fresh_checked_replay','fresh_hsaco_emitted'))
        and lower['frontend_recipe_is_diagnostic'] is True and lower['unresolved_runtime_requirements'] == 8,
        'actual nine-stage generation retains unresolved obligations')
    owned(owner['owned'])
    require(cpu['prior_cpu'] == old_review['down2_provenance']['candidate_cpu'], 'same CPU14 predecessor')
    body(cpu['runner']); body(cpu['overlay']); body(lower['package_manifest'])
    before = doc(cpu['raw']['sources-before.json'])
    require(before == doc(cpu['raw']['sources-after.json'])
        and len(cpu['formatted_sources']) == 3 and len(cpu['lowering_sources']) == 5, 'actual unchanged compiled source census')
    for pin in [*cpu['formatted_sources'].values(), *cpu['lowering_sources'].values()]:
        rel = str(Path(pin['path']).relative_to(Path(cpu['fixture'])))
        require(content(pin) == (before['fixture'][rel]['bytes'],before['fixture'][rel]['sha256']), 'source snapshot join')
        body(pin)
    stages = ('fixture-metadata','checked-lowering','actual-replay','actual-inert-join','emit',
              'extract-retained','descriptor-metadata','elf-notes','disassembly')
    require(tuple(row['name'] for row in lower['commands']) == stages and len(lower['artifacts']) == 10,
            'all nine retained checked phases and ten products')
    descriptor = None
    for row in lower['commands']:
        for key in ('command','started','result','stdout','stderr'): body(row[key])
        result = doc(row['result'])
        require(type(result['exit_code']) is int and result['exit_code'] == 0
            and result['reason'] is None and result['group_absent'] is True
            and all(result[k+'_sha256'] == row[k]['sha256'] for k in ('stdout','stderr')), 'natural lowering phase')
        if row['name'] == 'descriptor-metadata':
            lines = body(row['stdout']).decode('ascii').splitlines()
            require(lines.pop(0) == 'fe2o3-finite-join-request-metadata-v1', 'MLP descriptor header')
            descriptor = unique(line.split(' ',1) for line in lines)
        elif row['name'] == 'elf-notes':
            require(all(marker in body(row['stdout']) for marker in (b'.group_segment_fixed_size: 512',
                b'.private_segment_fixed_size: 0',b'.wavefront_size: 64')), 'unchanged launch resource markers')
    image = lower['artifacts']['emitted/artifact.hsaco']
    require(content(config['mlp_image']) == content(image) and content(image) != content(old_review['down2_image']),
            'actual new emitted MLP content')
    expected = dict(authority='none',object_sha256=image['sha256'],object_bytes=str(image['bytes']),
        entry_symbol_hex='ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2'.encode().hex(),target='gfx950:xnack-',
        code_object_version='6',explicit_argument_bytes='88',kernarg_segment_bytes='344',
        kernarg_alignment='8',workgroup='64,1,1',max_grid_workgroups='64,1,1')
    require(set(descriptor) == set(expected) | {'descriptor_sha256','canonical_code_object_digest','descriptor_symbol_hex'}
        and all(descriptor[k] == v for k,v in expected.items()), 'unchanged MLP ABI')
    for key in ('descriptor_sha256','canonical_code_object_digest'):
        require(re.fullmatch('[0-9a-f]{64}',descriptor[key]), 'descriptor digest')
    require(re.fullmatch('(?:[0-9a-f]{2})+',descriptor['descriptor_symbol_hex']), 'descriptor symbol')
    for pin in lower['artifacts'].values(): body(pin)
    body(config['mlp_image'])
    for key in ('command.json','started.json','owned-result.json','stdout','stderr'): body(owner['raw'][key])
    require(doc(owner['raw']['owned-result.json']) == owner['owned'], 'owner outcome body join')
    return dict(cpu=config['mlp_cpu'],lowering=config['mlp_lowering_complete'],owner=config['mlp_lowering_owner'],
        source_manifest=cpu['overlay'],original_image=image,descriptor=descriptor,
        numerical_acceptance=False,runtime_requirements_discharged=False)


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary bytecode-free local assembler')
    require(len(sys.argv) == 5, 'CONFIG_PATH CONFIG_SHA ROOT_NOTES_PATH ROOT_NOTES_SHA')
    config, config_pin = local_document(sys.argv[1],sys.argv[2])
    notes, notes_pin = local_document(sys.argv[3],sys.argv[4])
    keys(config,'schema input_label output_label session parent_audit worker_audit supervisor_tests supervisor_manifest '
        'mlp_cpu mlp_lowering_complete mlp_lowering_owner mlp_image')
    require(config['schema'] == 'ferric-p228-silu-materialized-capture-assembly-inputs-v1'
        and re.fullmatch(r'prefix-silu-materialized-capture-inputs-v228-v[1-9][0-9]{0,8}',config['input_label'])
        and re.fullmatch(r'prefix-silu-materialized-capture-gpu-v228-v[1-9][0-9]{0,8}',config['output_label'])
        and type(config['session']) is str and re.fullmatch('[0-9a-f]{64}',config['session'])
        and config['session'] != '0'*64, 'fresh explicit namespaces/session')
    keys(notes,'schema configuration capture')
    require(notes['schema'] == 'ferric-p228-silu-materialized-capture-root-notes-v1'
        and notes['configuration'] == config, 'explicit root review is for this exact configuration')
    decision(notes['capture'],'gpu_attempts review_topics')
    require(type(notes['capture']['gpu_attempts']) is int and notes['capture']['gpu_attempts'] == 1,
            'one explicit engineering attempt')
    out = L / config['input_label']
    require(not os.path.lexists(out),'fresh input directory')
    K,test = pure(config)
    keys(notes['capture']['review_topics'],' '.join(K['TOPICS']))
    require(all(type(text) is str and 32 <= len(text.strip()) and len(text.encode()) <= 16384
        for text in notes['capture']['review_topics'].values()), 'substantive supplied review topics')
    cpu,cpu_pin = known(CPU_LABEL,CPU_SHA)
    require(cpu['schema'] == 'ferric-p228-projection-residual-runtime-cpu-result-v1'
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['source_unchanged'] is True and cpu['tests_passed'] == 988 and cpu['tests_ignored'] == 4
        and len(cpu['phases']) == 84,'unchanged qualified CPU988 executables')
    require(doc(cpu['raw']['sources-before.json']) == doc(cpu['raw']['sources-after.json']), 'unchanged compiled generation')
    capture,capture_pin = known(CAPTURE_LABEL,CAPTURE_SHA)
    require(content(capture_pin) == K['BASELINE'] and capture['passed'] is True and capture['failures'] == []
        and capture['schema'] == 'ferric-p228-projection-residual-capture-gpu-v1'
        and capture['native_attempts'] == 1 and capture['retries'] == 0 and capture['full_forward'] is False
        and capture['parent_cpu_complete'] == capture['worker_cpu_complete'] == cpu_pin,'actual corrected residual layer capture')
    capture_leaves(capture)
    old_plan = doc(capture['plan']); old_review = doc(old_plan['capture_review'])
    require(old_plan['baseline'] == capture['baseline'] and old_plan['request'] == capture['request']
        and old_plan['parent_cpu'] == old_plan['worker_cpu'] == cpu_pin, 'original plan source')
    binaries = {role:cpu['binaries'][name]['binary'] for role,name in NAMES.items()}
    for role,pin in binaries.items():
        require(pin == capture['selected_runtime'][role] == old_plan[role]
            and pin['path'] == str(E / CPU_LABEL / 'target' / role / 'debug' / NAMES[role]),'original exact selected ELF')
        raw = body(pin)
        require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00','retained x86_64 ELF')
    tf4 = doc(capture['baseline'])
    require(capture['baseline']['sha256'] == TF4_SHA and tf4['passed'] is True and tf4['failures'] == []
        and tf4['native_attempts'] == 1 and tf4['retries'] == 0, 'original genuine TF4 input source')
    previous = doc(capture['request'])
    require(previous['schema'] == 'FerricFiniteProjectionResidualLayerCaptureRequestV1'
        and previous['layer']['schema'] == 'FerricFinitePrefixLayerCaptureRequestV1'
        and previous['projection_residual_image']['sha256'] == list(bytes.fromhex(old_plan['projection_image']['sha256']))
        and old_plan['projection_image']['sha256'] == '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25',
        'unchanged native route and corrected residual')
    mlp = new_mlp(config,K,old_review)
    request = copy.deepcopy(previous)
    request['layer'].update(mlp_tiles_image=dict(config['mlp_image'],sha256=list(bytes.fromhex(config['mlp_image']['sha256']))),
        session=list(bytes.fromhex(config['session'])),evidence_directory=str(E / config['output_label'] / 'native'))
    require(request['layer']['session'] != previous['layer']['session'],'new explicit session')
    require(len(json.dumps(request,separators=(',',':')).encode()) <= 16384,'bounded native request')
    pending = {}
    def pack(name,value,raw=False):
        data = value if raw else (json.dumps(value,indent=2,sort_keys=True,allow_nan=False)+'\n').encode('ascii')
        require(name not in pending and len(data) <= 256 << 10,'bounded unique assembly output')
        pending[name] = data
        return dict(path=str(E / config['input_label'] / name),bytes=len(data),sha256=hashlib.sha256(data).hexdigest())
    plan = dict(schema=K['INPUT_SCHEMA'],output_label=config['output_label'],baseline=old_plan['baseline'],
        baseline_capture=capture_pin,parent_cpu=cpu_pin,worker_cpu=cpu_pin,**binaries,
        request=pack('request.json',request),projection_image=old_plan['projection_image'],
        lowering_complete=old_plan['lowering_complete'],inspection_complete=old_plan['inspection_complete'],
        **{key:config[key] for key in ('mlp_image','mlp_cpu','mlp_lowering_complete','mlp_lowering_owner')},
        supervisor_tests=config['supervisor_tests'],supervisor_test_sources=test['sources_before'])
    for role in ('parent','worker'):
        review_pin = old_plan[role+'_runtime_review']
        require(review_pin == capture[role+'_runtime_review'],'original reviewed runtime identity')
        data = runtime(role,config[role+'_audit'],review_pin,binaries[role],cpu_pin,cpu)
        plan[role+'_runtime_review'] = pack(role+'-runtime-review.json',data,raw=True)
    review = {key:plan[key] for key in K['REVIEW_BINDINGS']}
    review.update(schema=K['REVIEW_SCHEMA'],**notes['capture'],output_label=config['output_label'],
        image=old_review['image'],image_provenance=old_review['image_provenance'],
        projection_provenance=old_review['projection_provenance'],prior_down2_provenance=old_review['down2_provenance'],
        mlp_provenance=mlp,**{key:False for key in K['REVIEW_FALSE']})
    plan['capture_review'] = pack('capture-review.json',review)
    require(set(plan) == set(K['PLAN_FIELDS'].split()),'exact frozen intake plan')
    plan_pin = pack('plan.json',plan)
    _,own = fingerprint(Path(__file__).resolve()); SEEN[own['path']] = own
    for path,pin in SEEN.items(): require(fingerprint(Path(path))[1] == pin,'consumed inputs unchanged before writes')
    new_transfer = {path:row['original'] for path,row in ORIGINALS.items()
        if Path(path).relative_to(E).parts[0].startswith(('silu-materialized-','row-silu-materialized-',
            'p228-silu-materialized-','run_silu_materialized_capture_gpu_pure_'))}
    outputs = {name:dict(path=str(E/config['input_label']/name),bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest())
               for name,raw in pending.items()}
    pack('assembly.json',dict(schema='ferric-p228-silu-materialized-capture-root-assembly-v1',
        plan=plan_pin,configuration=config_pin,root_notes=notes_pin,assembler=own,
        inputs=list(ORIGINALS.values()),local_only_inputs=[p for p in SEEN.values()
            if p['path'] not in {v['retained']['path'] for v in ORIGINALS.values()}],
        new_transfer=list(new_transfer.values()),new_transfer_count=len(new_transfer),outputs=outputs,
        existing_baseline_dependencies_still_required=True,existing_runtime_reviews_copied_verbatim=True,
        explicit_root_decisions_copied=True,automatic_approval=False,
        prior_gpu_outputs_replayed_here=False,unchanged_prior_input_bodies_rehashed=False,
        runtime_library_bodies_rehashed_locally=False,all_transitive_inputs_rehashed=False,
        fresh_runtime_audit_performed=False,gpu_execution=False,numerical_acceptance=False,
        production_authority=False,performance_claim=False))
    require(len(pending) == 6,'six-file assembly only')
    out.mkdir(mode=0o700)
    for name,raw in pending.items():
        with (out/name).open('xb') as stream:
            stream.write(raw);stream.flush();os.fsync(stream.fileno())
    for path,pin in SEEN.items(): require(fingerprint(Path(path))[1] == pin,'consumed input postchecks')
    print(json.dumps(plan_pin),flush=True)


if __name__ == '__main__':
    main()
