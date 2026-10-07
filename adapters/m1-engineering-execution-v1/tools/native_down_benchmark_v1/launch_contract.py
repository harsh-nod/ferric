"""Gate/up native campaign contract with current CPU and harness evidence gates."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat

HOST = 'smci350-rck-g03-b19-03'
UID = 9661
DEVICE = 16366993098680759275
RUNTIME_MAIN = 'da6b561c5a3f12acc5b0e6da74c808273e728710'
WIDTH_QUALIFIED = True
WORKER_SHA = '74d764e8797ba0413e38b762765d07faf74aa6a0af5daf60c0a1a56adc64f20a'
DOWN_IDS = {
    'artifact_hsaco_id': '1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae',
    'artifact_manifest_id': '2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54',
    'artifact_handoff_id': 'cd23060449750a973f05742c0d5a043cb637491743e2ac56b56e847cc2c4dbdf',
}
DOWN_SUFFIX = ('native-inputs/down-splitk8-r1/fe2o3-engineering-v1/'
                  'efe280b299a95408cd5326305190c1ae7471d7fd0da62226aa2fc528153556b7')
DOWN_ROSTER = 'native-inputs/down-splitk8-r1/roster.json'
QUALIFIED_ARTIFACTS = {
    'runtime_source': 'f44d31f140938f918f159f47fb6c81975063c34526221029a4bc1ff10c72a462',
    'controller_source': '9e53fdc9f6c9e0690afac86ccaeed83ce000ebaa772f0deb40a6c57df0fe0b88',
    'worker': WORKER_SHA,
    'controllers': {
        'live-A': 'f1fca91663c1fa682ada28fd79ef3e119d7de9845dfb1d8fb0c860ca0ffada7b',
        'live-B': 'f1fca91663c1fa682ada28fd79ef3e119d7de9845dfb1d8fb0c860ca0ffada7b',
        'counters-A': 'c64cfab44f1ba50d8150c646a3934000de7c19f5e97647902d72521644581476',
        'counters-B': 'c64cfab44f1ba50d8150c646a3934000de7c19f5e97647902d72521644581476',
    },
}
OLD_PLAN_SHA = 'd91216856d50fdb22ea92ad67766af44b92120af3c5afec500617991af35516d'
ROSTER_SHA = '3e7aed02029016339bba6c3efbe4504b12ba78bf0150521157bd46898ead9ade'
V19_PINS = ('36fcfee3a886fb79719351708c38c0ae9af9617d5b9d723aca103242f825456c',
            '951619f87213020aaf0eebf794a37ef523400fac1f23ce9fd90e1b78d4b3e1da')
V19_SUFFIX = 'native-inputs/v19/fe2o3-engineering-v1/0f1dc95268aca76a3aeaf69eda200875d317b5d4dbf26af7f9dcf96a4a9f581a'
COUNTER_PINS = {
    'run_counter_diagnostic.py': '9758475fcf059295c27199fbdb929ed0bc71cd5e56db4e28ade9bfc2c25f8e93',
    'process_evidence.py': 'e8d70b045e3c4b2d8a025df31d0e61193dceaa35d97b83b1d541ae1f1d72bc2d',
}
OWN_SOURCES = ('launch_contract.py', 'prepare_stage.py', 'run_stage.py', 'test_native_launch.py',
    'test_prefill_width.py', 'test_cpu_binding.py', 'fixtures/frozen_run_v17_native.py',
    'cpu_binding_a009.py', 'native_cpu_contract_a009.py', 'cpu-checkpoint-a009.json', 'test_cpu_binding_a009.py',
    'cpu_binding_down_r1.py', 'native_cpu_contract_down_r1.py', 'test_cpu_binding_down_r1.py')
MEASUREMENT_SOURCES = ('native_token_cell.py', 'abba_ledger.py', 'gpu_activity.py',
    'native_campaign_replay.py', 'native_lifecycle.py', 'test_native_token_cell.py',
    'test_abba_ledger.py', 'test_gpu_activity.py', 'test_native_campaign_replay.py', 'test_native_lifecycle.py')
CONTROLLERS = ('live-A', 'live-B', 'counters-A', 'counters-B')


def cpu_contract():
    path = Path(__file__).with_name('native_cpu_contract_down_r1.py')
    return module(path, read(path)[1])


def iter_cpu_bindings(cpu):
    return cpu_contract().iter_bindings(cpu)


MODEL_BUNDLE = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
MODEL_TARGET = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
IMAGE_FIELDS = {'--target-artifact': None, '--target-head-artifact': 'fp32_head_artifact',
    '--argmax-artifact': 'argmax_artifact', '--attention-artifact': 'attention_artifact',
    '--rmsnorm-artifact': 'rmsnorm_artifact', '--split-attention-artifact': 'split_attention_artifact',
    '--prefill-kv-artifact': 'prefill_kv_artifact', '--gemv-artifact': 'gemv_artifact',
    '--ordered64-kv-copy-artifact': 'kv_copy_artifact'}
EXTRA_IMAGES = ('--split-attention-artifact', '--prefill-kv-artifact', '--gemv-artifact',
                '--ordered64-kv-copy-artifact')
CELL_ORDER = (('counter-A', 'A', 'counters'), ('counter-B', 'B', 'counters')) + tuple(
    ('block' + str(block) + '-' + suffix, arm, 'latency')
    for block in range(1, 4) for suffix, arm in (('A1', 'A'), ('B1', 'B'), ('B2', 'B'), ('A2', 'A')))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'exact SHA256 required')
    return value


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def decode(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, expected=None, maximum=512 * 1024**2, empty=False):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    with path.open('rb') as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode) and (empty or before.st_size > 0)
                and before.st_size <= maximum, 'bounded regular input: ' + str(path))
        raw = source.read(maximum + 1)
        after = os.fstat(source.fileno())
    require(len(raw) == before.st_size and all(getattr(before, key) == getattr(after, key)
        == getattr(path.lstat(), key) for key in ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')),
        'input changed while reading')
    digest = hashlib.sha256(raw).hexdigest()
    require(expected is None or digest == sha(expected), 'input hash mismatch: ' + str(path))
    return raw, digest


def binding(value):
    require(type(value) is dict and set(value) == {'path', 'sha256'}
            and type(value['path']) is str and Path(value['path']).is_absolute(), 'closed file binding')
    sha(value['sha256'])
    return value


def bound(value, maximum=32 * 1024**2):
    binding(value)
    return decode(read(value['path'], value['sha256'], maximum)[0])


def module(path, expected):
    raw, _ = read(path, expected, 1024**2)
    spec = importlib.util.spec_from_file_location('v14_' + Path(path).stem, path)
    value = importlib.util.module_from_spec(spec)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def relative(value):
    path = Path(value)
    require(type(value) is str and not path.is_absolute() and path.parts and '..' not in path.parts
            and str(path) == value, 'canonical private relative path')
    return path


def stage_name(stage):
    stage = Path(stage)
    require(stage.is_absolute() and stage.parent == Path('/dev/shm')
            and re.fullmatch(r'ferric-native-down-[a-zA-Z0-9_-]{1,48}', stage.name),
            'fresh bounded prefill tmpfs stage name')
    return stage


def validate_build(build):
    require(WIDTH_QUALIFIED is True and type(QUALIFIED_ARTIFACTS) is dict
            and WORKER_SHA is not None,
            'native width release/images/CPU qualification is pending; admission disabled')
    require(type(build) is dict and set(build) == {'schema', 'runtime_main', 'runtime_source',
            'controller_source', 'worker', 'controllers', 'cpu_qualification'}
            and build['schema'] == 'FerricNativeDownBuildBindingR1'
            and build['runtime_main'] == RUNTIME_MAIN, 'runtime baseline-base/source build binding')
    for key in ('runtime_source', 'controller_source', 'worker', 'cpu_qualification'):
        binding(build[key])
    require(build['worker']['sha256'] == WORKER_SHA, 'qualified published slots512 worker required')
    require({name: build[name]['sha256'] for name in ('runtime_source', 'controller_source', 'worker')}
            == {name: QUALIFIED_ARTIFACTS[name] for name in ('runtime_source', 'controller_source', 'worker')},
            'reviewed source/release artifact identities required')
    require(type(build['controllers']) is dict and set(build['controllers']) == set(CONTROLLERS),
            'closed A/B aliases for two same-source width-selector binaries required')
    for value in build['controllers'].values():
        binding(value)
    require(build['controllers']['live-A']['sha256'] == build['controllers']['live-B']['sha256']
            and build['controllers']['counters-A']['sha256'] == build['controllers']['counters-B']['sha256']
            and build['controllers']['live-A']['sha256'] != build['controllers']['counters-A']['sha256'],
            'A and B must use identical live/counter binaries, distinct between instrumentation modes')
    require({key: item['sha256'] for key, item in build['controllers'].items()} == QUALIFIED_ARTIFACTS['controllers'],
            'reviewed controller release identities required')


def validate_cpu(cpu, build, read_bound=bound, read_raw=read):
    sources = set(OWN_SOURCES) | {'measurement/' + name for name in MEASUREMENT_SOURCES}
    cpu_contract().validate(cpu, build,
        lambda path, maximum: read_raw(path, None, maximum, True)[0], sources, QUALIFIED_ARTIFACTS)


def remap_common(old, stage, worker_sha, requests, arm):
    require(requests in (1, 6), 'one correctness/counter or six latency requests')
    require(arm in ('A', 'B'), 'closed down arm required')
    old_stage = Path(old['stage'])
    result = [str(stage / Path(word).relative_to(old_stage))
              if word.startswith(str(old_stage) + '/') else word for word in old['common_args']]
    for key, value in (('--max-batches', str(requests * 131)), ('--worker-sha256', sha(worker_sha))):
        require(result.count(key) == 1, 'one frozen selector: ' + key)
        result[result.index(key) + 1] = value
    require(result.count('--worker') == 1 and result[result.index('--worker') + 1] == str(stage / 'worker-candidate'),
            'owned worker path')
    return result


def parse_common(values, runner):
    require(values.count('--max-batches') == 1, 'one batch budget')
    projected = list(values)
    index = projected.index('--max-batches') + 1
    actual = projected[index]
    require(actual in ('131', '786'), 'closed cell batch budget')
    projected[index] = '540'
    options = runner.arguments(projected)
    options['--max-batches'] = actual
    return options


def down_metadata(down, roster):
    return {'schema': 'FerricNativeDownSelectionR1', 'authority': 'none',
        'artifact_path': down['path'], 'artifact': dict(DOWN_IDS),
        'compiler_roster': {**down['roster'], 'value': roster},
        'loaded_image_count': 10, 'additional_weight_bytes': 0, 'scratch_bytes': 131072,
        'weights': 'existing authenticated KN BF16 down maps',
        'scratch_layout': 'eight FP32 partial rows of 4096 elements',
        'prefill_unchanged': True, 'prefill_rows': 32, 'prefill_dispatches': 649,
        'decode_dynamic_slots': 180, 'decode_command_family': 'legacy256-v1',
        'decode_positions': [128,255], 'roles': [2], 'selected_rows': 1,
        'same_image_set_in_both_arms': True, 'same_scratch_in_both_arms': True,
        'native_qualified': False, 'performance_qualified': False, 'serving_qualified': False}


def validate_down(stage, down, files):
    require(type(down) is dict and set(down) == {'path', 'ids', 'roster'}
            and down['path'] == str(stage / DOWN_SUFFIX)
            and down['ids'] == DOWN_IDS, 'closed candidate image binding')
    binding(down['roster'])
    require(down['roster']['path'] == str(stage / DOWN_ROSTER), 'fixed external candidate roster path')
    root = Path(down['path'])
    require(root.parent.name == 'fe2o3-engineering-v1'
            and {path.name for path in root.iterdir()} == {'observation.json', 'observation.hsaco'},
            'candidate content namespace and exact two-file image directory')
    inputs = [(str(stage / DOWN_SUFFIX / name), DOWN_IDS[key]) for name,key in
              (('observation.hsaco','artifact_hsaco_id'), ('observation.json','artifact_manifest_id'))]
    inputs.append((down['roster']['path'], down['roster']['sha256']))
    raw = {}
    for path, pin in inputs:
        name = str(Path(path).relative_to(stage))
        require(name in files and files[name]['sha256'] == pin, 'candidate input absent from closed file roster')
        raw[Path(path).name] = read(path, pin)[0]
    observation = decode(raw['observation.json'])
    content = hashlib.sha256(b'FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0')
    for name in ('observation.json', 'observation.hsaco'):
        content.update(len(raw[name]).to_bytes(8, 'little'))
        content.update(raw[name])
    require(root.name == content.hexdigest(), 'candidate content-addressed directory identity')
    require(observation['compiler_handoff']['sha256'] == DOWN_IDS['artifact_handoff_id'], 'candidate compiler handoff changed')
    return down_metadata(down, decode(raw['roster.json']))


def cell_spec(stage, arm, mode, common, images, observations, build, request, reference, down_expected):
    require(arm in ('A', 'B') and mode in ('correctness', 'counters', 'latency'), 'closed cell selector')
    controller_key = ('counters' if mode == 'counters' else 'live') + '-' + arm
    controller = {'path': str(stage / ('controller-' + controller_key)),
                  'sha256': build['controllers'][controller_key]['sha256']}
    argv = [controller['path'], '--native-prefill-rows', '32']
    argv += list(common)
    argv += ['--wave-target-mode', 'combined']
    for key in EXTRA_IMAGES:
        argv += [key, images[key]['path']]
    argv += ['--prefill-kv-mode', 'parallel-prefill16-v27', '--split-attention-mode', 'split8-v21',
             '--c1-packet-mode', 'packed64-v29', '--gemv-mode', 'baseline', '--ordered64-kv-copy-mode', 'parallel-c1-v19']
    argv += ['--native-down', 'control' if arm == 'A' else 'splitk8',
             '--down-artifact', down_expected['artifact_path'],
             '--down-roster', down_expected['compiler_roster']['path'],
             '--down-roster-sha256', down_expected['compiler_roster']['sha256'],
             '--down-hsaco-sha256', DOWN_IDS['artifact_hsaco_id'],
             '--down-manifest-sha256', DOWN_IDS['artifact_manifest_id'],
             '--down-handoff-sha256', DOWN_IDS['artifact_handoff_id']]
    setup = {'model_bundle_id': MODEL_BUNDLE, 'target_model_id': MODEL_TARGET}
    profile, closed = {}, {}
    for option, field in IMAGE_FIELDS.items():
        image = images[option]
        metadata = {'artifact_manifest_id': image['manifest_sha256'], 'artifact_hsaco_id': image['hsaco_sha256'],
                    'artifact_handoff_id': sha(observations[option]['compiler_handoff']['sha256'])}
        if field is None:
            setup.update(metadata)
        else:
            setup[field] = metadata
        if option in EXTRA_IMAGES:
            for scope in (setup, profile, closed):
                scope[field] = metadata
                scope[field + '_path'] = image['path']
    for scope in (setup, profile):
        for key in ('--attention-artifact', '--rmsnorm-artifact'):
            scope[IMAGE_FIELDS[key] + '_path'] = images[key]['path']
    for scope in (setup, profile, closed):
        scope.update(requested_gemv_mode='baseline', gemv_mode='baseline', split_attention_workspace_bytes=133120,
            split_attention_policy={'physical_rows': 1, 'actual_context_min': 128, 'actual_context_max': 256,
                'fallback': 'query-hoist-v14', 'fallback_packets_no_head': 613, 'fallback_packets_with_head': 616,
                'partitions': 8, 'split_packets_no_head': 649, 'split_packets_with_head': 652,
                'packet_counts_scope': 'baseline FFN only; selected composition counts are in native_down'})
    return {'schema': 'FerricNativeDownTokenCellPlanR1', 'arm': arm, 'mode': mode, 'argv': argv,
        'controller': controller, 'worker': {'path': str(stage / 'worker-candidate'), 'sha256': build['worker']['sha256']},
        'device_unique_id': DEVICE, 'prompt': request['prompt'], 'reference': reference,
        'down_expected': down_expected,
        'setup_expected': setup, 'profile_expected': profile, 'closed_expected': closed,
        'timeouts': {'setup_seconds': 600, 'request_seconds': 180, 'cell_seconds': 1200}}


def campaign_cells(stage, old, images, observations, build, request, reference, down_expected):
    cells = []
    for cell_id, arm, mode in CELL_ORDER:
        common = remap_common(old, stage, build['worker']['sha256'], 6 if mode == 'latency' else 1, arm)
        cells.append({'cell_id': cell_id, 'output': str(stage / 'cells' / cell_id), 'common_args': common,
                      'spec': cell_spec(stage, arm, mode, common, images, observations, build, request, reference, down_expected)})
    return cells


def comparison_plan(cells, inputs, sources, cell):
    profiles = {arm: cell.profile_for_spec(next(row['spec'] for row in cells
        if row['spec']['arm'] == arm and row['spec']['mode'] == 'latency')) for arm in ('A', 'B')}
    plan = {'schema': 'FerricNativeDownChangePlanR1', 'change_id': 'native-down-control-vs-splitk8',
        'gates': dict(cell.ledger.GATES), 'workload': {'input_tokens': 128, 'output_tokens': 128,
            'context_tokens': 8192, 'concurrency': 1, 'tensor_parallel': 1,
            'greedy': True, 'prefix_caching': False, 'speculation': False},
        'reference_sha256': cell.ledger.digest(cells[0]['spec']['reference']),
        'workload_sha256': inputs['workload']['sha256'],
        'client_sha256': sources['measurement/native_token_cell.py'], 'profiles': profiles,
        'allowed_profile_differences': cell.ledger.differences(profiles['A'], profiles['B']),
        'mechanism': {arm: cell.expected_mechanism(arm) for arm in ('A', 'B')},
        'timing_semantics': 'native-ingress-v1'}
    return cell.ledger.validate_plan(plan)


def selected_cell(plan, cell_id):
    require(cell_id in [row[0] for row in CELL_ORDER], 'predeclared campaign cell ID required')
    matches = [row for row in plan['cells'] if row['cell_id'] == cell_id]
    require(len(matches) == 1, 'one exact campaign cell')
    return matches[0]


def verify_files(stage, files):
    require(type(files) is dict and 1 <= len(files) <= 512, 'bounded exact staged input roster')
    total = 0
    for name, item in files.items():
        path = stage / relative(name)
        require(type(item) is dict and set(item) == {'sha256', 'bytes', 'mode'}, 'closed staged identity')
        raw, _ = read(path, item['sha256'], empty=True)
        observed = path.lstat()
        require(len(raw) == item['bytes'] and observed.st_uid == os.getuid() and observed.st_nlink == 1
                and stat.S_IMODE(observed.st_mode) == item['mode'] in (0o600, 0o700), 'staged input identity')
        total += len(raw)
    require(total < 2 * 1024**3 - 512 * 1024**2, 'staged inputs leave bounded output headroom')


def validate_cpu_stage(stage, files, cpu):
    for _, item in iter_cpu_bindings(cpu):
        binding(item)
        path = Path(item['path'])
        require(path.is_relative_to(stage), 'CPU receipt outside immutable stage')
        relative_path = str(relative(str(path.relative_to(stage))))
        require(relative_path in files and files[relative_path]['sha256'] == item['sha256'],
                'CPU receipt missing from closed staged file roster')


def validate_plan(plan, stage, cell_id='counter-A'):
    require(type(plan) is dict and set(plan) == {'schema', 'stage', 'cells', 'images', 'inputs',
            'files', 'build', 'cpu', 'sources', 'engineering_only', 'comparison', 'down'}
            and plan['schema'] == 'FerricNativeDownCampaignPlanR1' and plan['stage'] == str(stage)
            and plan['engineering_only'] is True, 'closed native campaign plan')
    stage_name(stage)
    verify_files(stage, plan['files'])
    for name, digest in plan['sources'].items():
        require(plan['files'][name]['sha256'] == sha(digest), 'source roster binding')
    build = bound(plan['build'])
    validate_build(build)
    cpu = bound(plan['cpu'])
    require(build['cpu_qualification'] == plan['cpu'], 'build must bind staged CPU evidence')
    validate_cpu_stage(stage, plan['files'], cpu)
    validate_cpu(cpu, build)
    require(all(plan['sources'].get(name) == digest for name, digest in cpu['harness_sources'].items()),
            'launched sources differ from CPU qualification')
    counter = module(stage / 'counter-support/run_counter_diagnostic.py', COUNTER_PINS['run_counter_diagnostic.py'])
    evidence = module(stage / 'counter-support/process_evidence.py', COUNTER_PINS['process_evidence.py'])
    loaded = counter.load(stage)
    _, _, legacy, _, _, runner, supervisor, _ = loaded
    require((supervisor.HOST, supervisor.UID, int(supervisor.DEVICE_UNIQUE_ID)) == (HOST, UID, DEVICE)
            and supervisor.ROOT_FREE_BYTES == 64 * 1024**3 and supervisor.MEMORY_AVAILABLE_BYTES == 128 * 1024**3
            and supervisor.STAGE_BYTES == 2 * 1024**3, 'unchanged frozen resource policy')
    request, reference = runner.workload_reference(plan['inputs'])
    observations = {key: decode(read(Path(image['path']) / 'observation.json', image['manifest_sha256'])[0])
                    for key, image in plan['images'].items()}
    down_expected = validate_down(stage, plan['down'], plan['files'])
    cell = module(stage / 'measurement/native_token_cell.py', plan['sources']['measurement/native_token_cell.py'])
    require(type(plan['cells']) is list and len(plan['cells']) == len(CELL_ORDER), 'exact fourteen-cell campaign')
    for row, (expected_id, arm, mode) in zip(plan['cells'], CELL_ORDER):
        require(type(row) is dict and set(row) == {'cell_id', 'output', 'common_args', 'spec'}
                and row['cell_id'] == expected_id and row['output'] == str(stage / 'cells' / expected_id),
                'predeclared cell order and fresh output root')
        options = parse_common(row['common_args'], runner)
        expected = cell_spec(stage, arm, mode, row['common_args'], plan['images'], observations, build, request, reference, down_expected)
        require(row['spec'] == expected and options['--device-unique-id'] == str(DEVICE), 'cell reconstruction differs')
        cell.shape(expected)
    require(plan['comparison'] == comparison_plan(plan['cells'], plan['inputs'], plan['sources'], cell),
            'predeclared comparison differs from actual cell composition')
    selected = selected_cell(plan, cell_id)
    options = parse_common(selected['common_args'], runner)
    input_plan = {**plan['inputs'], 'controller': selected['spec']['controller'], 'images': plan['images']}
    identities = legacy.inputs(input_plan, options, runner)
    return loaded, counter, evidence, cell, options, identities
