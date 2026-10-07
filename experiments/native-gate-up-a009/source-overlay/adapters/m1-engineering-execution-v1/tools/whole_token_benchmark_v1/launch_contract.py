"""Bounded source/binary custody and pure V14 launch-plan construction."""
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
# Historical main base for this comparison, not the current upstream tip.
RUNTIME_MAIN = '40695ecf5ffd7f3ec2bca8fd138a89cb675b57c6'
OLD_PLAN_SHA = 'd91216856d50fdb22ea92ad67766af44b92120af3c5afec500617991af35516d'
ROSTER_SHA = '3e7aed02029016339bba6c3efbe4504b12ba78bf0150521157bd46898ead9ade'
V19_PINS = ('36fcfee3a886fb79719351708c38c0ae9af9617d5b9d723aca103242f825456c',
            '951619f87213020aaf0eebf794a37ef523400fac1f23ce9fd90e1b78d4b3e1da')
V19_SUFFIX = 'native-inputs/v19/fe2o3-engineering-v1/0f1dc95268aca76a3aeaf69eda200875d317b5d4dbf26af7f9dcf96a4a9f581a'
COUNTER_PINS = {
    'run_counter_diagnostic.py': '9758475fcf059295c27199fbdb929ed0bc71cd5e56db4e28ade9bfc2c25f8e93',
    'process_evidence.py': 'e8d70b045e3c4b2d8a025df31d0e61193dceaa35d97b83b1d541ae1f1d72bc2d',
}
OWN_SOURCES = ('launch_contract.py', 'prepare_stage.py', 'run_stage.py', 'test_native_launch.py')
MEASUREMENT_SOURCES = ('native_token_cell.py', 'abba_ledger.py', 'gpu_activity.py', 'native_campaign_replay.py')
PHASES = {'runtime-tests', 'aql-tests', 'worker-tests', 'runtime-build', 'controller-tests',
          'controller-build', 'launch-tests', 'measurement-tests'}
CPU_LIMITS = {'duration_seconds': 1200, 'individual_file_bytes': 536870912, 'kill_wait_seconds': 3,
    'log_bytes': 33554432, 'memory_available_bytes': 137438953472, 'observed_rss_bytes': 8589934592,
    'root_free_bytes': 23622320128, 'shm_free_bytes': 34359738368, 'stage_bytes': 30064771072,
    'stage_reserve_bytes': 536870912, 'term_grace_seconds': 10}
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
            and re.fullmatch(r'ferric-v14-native-40695ec-[a-zA-Z0-9_-]{1,48}', stage.name),
            'fresh bounded V14 tmpfs stage name')
    return stage


def validate_build(build):
    require(type(build) is dict and set(build) == {'schema', 'runtime_main', 'runtime_source',
            'controller_source', 'worker', 'controllers', 'cpu_qualification'}
            and build['schema'] == 'FerricV14NativeBuildBindingV1'
            and build['runtime_main'] == RUNTIME_MAIN, 'runtime baseline-base/source build binding')
    for key in ('runtime_source', 'controller_source', 'worker', 'cpu_qualification'):
        binding(build[key])
    require(type(build['controllers']) is dict and set(build['controllers']) == {'A', 'B', 'counters'},
            'two explicit latency controllers and separate counters binary')
    for value in build['controllers'].values():
        binding(value)
    require(len({build['controllers'][key]['sha256'] for key in ('A', 'B', 'counters')}) == 3,
            'distinct explicit controller entry binaries required')


def validate_cpu(cpu, build, read_bound=bound, read_raw=read):
    require(type(cpu) is dict and set(cpu) == {'schema', 'runtime_source_sha256', 'controller_source_sha256',
            'worker_sha256', 'controllers', 'phases', 'harness_sources'} and cpu['schema'] == 'FerricV14CpuQualificationV1',
            'closed engineering CPU qualification')
    require(cpu['runtime_source_sha256'] == build['runtime_source']['sha256']
            and cpu['controller_source_sha256'] == build['controller_source']['sha256']
            and cpu['worker_sha256'] == build['worker']['sha256']
            and cpu['controllers'] == {key: value['sha256'] for key, value in build['controllers'].items()},
            'CPU qualification must bind exact current sources and binaries')
    require(type(cpu['phases']) is dict and set(cpu['phases']) == PHASES, 'complete CPU phase roster')
    require(type(cpu['harness_sources']) is dict and set(cpu['harness_sources']) ==
            set(OWN_SOURCES) | {'measurement/' + name for name in MEASUREMENT_SOURCES}, 'tested source roster')
    for digest in cpu['harness_sources'].values():
        sha(digest)
    for name, phase in cpu['phases'].items():
        require(type(phase) is dict and set(phase) == {'status', 'result'}, 'exact CPU phase receipts')
        binding(phase['status'])
        require(read_raw(phase['status']['path'], phase['status']['sha256'], 16)[0] == b'0\n',
                'nonzero raw CPU status: ' + name)
        result = read_bound(phase['result'])
        expected = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
            'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
            'log_limit_exceeded': False, 'profile': 'FerricCpuFourCore28GiBBuildV1'}
        require(all(type(result.get(key)) is type(value) and result[key] == value for key, value in expected.items()),
                'failed or unclean guarded CPU phase: ' + name)
        require(result.get('limits') == CPU_LIMITS and all(type(value) is int for value in result['limits'].values())
                and result.get('cpus') == [0, 1, 2, 3] and all(type(value) is int for value in result['cpus'])
                and type(result.get('nice')) is int and result['nice'] == 19
                and type(result.get('build_jobs')) is int and result['build_jobs'] == 4
                and type(result.get('rust_test_threads')) is int and result['rust_test_threads'] == 1,
                'CPU resource limits or execution placement changed: ' + name)
        environment = result.get('launch_environment', {})
        require(all(environment.get(key) == value for key, value in {
            'CARGO_BUILD_JOBS': '4', 'RUST_TEST_THREADS': '1', 'FERRIC_CPU_PROFILE': 'FerricCpuFourCore28GiBBuildV1',
            'CUDA_VISIBLE_DEVICES': '-1', 'HIP_VISIBLE_DEVICES': '-1', 'HSA_VISIBLE_DEVICES': '-1',
            'ROCR_VISIBLE_DEVICES': '-1'}.items()), 'CPU guard must mask GPU visibility')
        require(type(result.get('peak_observed_rss_bytes')) is int
                and 0 <= result['peak_observed_rss_bytes'] <= CPU_LIMITS['observed_rss_bytes'], 'CPU RSS bound')
        for key in ('admission', 'final_resources'):
            resources = result.get(key, {})
            require(all(type(resources.get(field)) is int and resources[field] >= CPU_LIMITS[field]
                for field in ('memory_available_bytes', 'root_free_bytes', 'shm_free_bytes'))
                and type(resources.get('stage_bytes')) is int
                and 0 <= resources['stage_bytes'] <= CPU_LIMITS['stage_bytes'] - CPU_LIMITS['stage_reserve_bytes'],
                'CPU host or stage headroom changed')
        require(type(result.get('argv')) is list and result['argv'] and all(type(word) is str and word for word in result['argv']),
                'actual bounded CPU command required')


def remap_common(old, stage, worker_sha, requests):
    require(requests in (1, 6), 'one correctness/counter or six latency requests')
    old_stage = Path(old['stage'])
    result = [str(stage / Path(word).relative_to(old_stage))
              if word.startswith(str(old_stage) + '/') else word for word in old['common_args']]
    for key, value in (('--max-batches', str(requests * 135)), ('--worker-sha256', sha(worker_sha))):
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
    require(actual in ('135', '810'), 'closed cell batch budget')
    projected[index] = '540'
    options = runner.arguments(projected)
    options['--max-batches'] = actual
    return options


def cell_spec(stage, arm, mode, common, images, observations, build, request, reference):
    require(arm in ('A', 'B') and mode in ('correctness', 'counters', 'latency'), 'closed cell selector')
    controller_key = 'counters' if mode == 'counters' else arm
    controller = {'path': str(stage / ('controller-' + controller_key)),
                  'sha256': build['controllers'][controller_key]['sha256']}
    argv = [controller['path']]
    if mode == 'counters':
        argv += ['--token-program-backend', 'ordered64-groups-v1' if arm == 'A' else 'native-whole-program-v1']
    argv += list(common)
    argv += ['--wave-target-mode', 'combined']
    for key in EXTRA_IMAGES:
        argv += [key, images[key]['path']]
    argv += ['--prefill-kv-mode', 'parallel-prefill16-v27', '--split-attention-mode', 'split8-v21',
             '--c1-packet-mode', 'packed64-v29', '--gemv-mode', 'baseline', '--ordered64-kv-copy-mode', 'parallel-c1-v19']
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
                'partitions': 8, 'split_packets_no_head': 649, 'split_packets_with_head': 652})
    return {'schema': 'FerricNativeTokenCellPlanV1', 'arm': arm, 'mode': mode, 'argv': argv,
        'controller': controller, 'worker': {'path': str(stage / 'worker-candidate'), 'sha256': build['worker']['sha256']},
        'device_unique_id': DEVICE, 'prompt': request['prompt'], 'reference': reference,
        'setup_expected': setup, 'profile_expected': profile, 'closed_expected': closed,
        'timeouts': {'setup_seconds': 600, 'request_seconds': 180, 'cell_seconds': 1200}}


def campaign_cells(stage, old, images, observations, build, request, reference):
    cells = []
    for cell_id, arm, mode in CELL_ORDER:
        common = remap_common(old, stage, build['worker']['sha256'], 6 if mode == 'latency' else 1)
        cells.append({'cell_id': cell_id, 'output': str(stage / 'cells' / cell_id), 'common_args': common,
                      'spec': cell_spec(stage, arm, mode, common, images, observations, build, request, reference)})
    return cells


def comparison_plan(cells, inputs, sources, cell):
    profiles = {arm: cell.profile_for_spec(next(row['spec'] for row in cells
        if row['spec']['arm'] == arm and row['spec']['mode'] == 'latency')) for arm in ('A', 'B')}
    plan = {'schema': 'FerricChangePlanV1', 'change_id': 'v14-native-full-token',
        'gates': dict(cell.ledger.GATES), 'workload': {'input_tokens': 128, 'output_tokens': 128,
            'context_tokens': 8192, 'concurrency': 1, 'tensor_parallel': 1,
            'greedy': True, 'prefix_caching': False, 'speculation': False},
        'reference_sha256': cell.ledger.digest(cells[0]['spec']['reference']),
        'workload_sha256': inputs['workload']['sha256'],
        'client_sha256': sources['measurement/native_token_cell.py'], 'profiles': profiles,
        'allowed_profile_differences': cell.ledger.differences(profiles['A'], profiles['B']),
        'mechanism': {arm: {'dispatches_per_decode_token': 652,
            'publications_per_decode_token': groups, 'waits_per_decode_token': groups,
            'retired_signals_per_decode_token': 652} for arm, groups in (('A', 11), ('B', 1))},
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
        raw, _ = read(path, item['sha256'])
        observed = path.lstat()
        require(len(raw) == item['bytes'] and observed.st_uid == os.getuid() and observed.st_nlink == 1
                and stat.S_IMODE(observed.st_mode) == item['mode'] in (0o600, 0o700), 'staged input identity')
        total += len(raw)
    require(total < 2 * 1024**3 - 512 * 1024**2, 'staged inputs leave bounded output headroom')


def validate_plan(plan, stage, cell_id='counter-A'):
    require(type(plan) is dict and set(plan) == {'schema', 'stage', 'cells', 'images', 'inputs',
            'files', 'build', 'cpu', 'sources', 'engineering_only', 'comparison'}
            and plan['schema'] == 'FerricV14NativeCampaignPlanV1' and plan['stage'] == str(stage)
            and plan['engineering_only'] is True, 'closed native campaign plan')
    stage_name(stage)
    verify_files(stage, plan['files'])
    for name, digest in plan['sources'].items():
        require(plan['files'][name]['sha256'] == sha(digest), 'source roster binding')
    build = bound(plan['build'])
    validate_build(build)
    validate_cpu(bound(plan['cpu']), build)
    require(all(plan['sources'].get(name) == digest for name, digest in bound(plan['cpu'])['harness_sources'].items()),
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
    cell = module(stage / 'measurement/native_token_cell.py', plan['sources']['measurement/native_token_cell.py'])
    require(type(plan['cells']) is list and len(plan['cells']) == len(CELL_ORDER), 'exact fourteen-cell campaign')
    for row, (expected_id, arm, mode) in zip(plan['cells'], CELL_ORDER):
        require(type(row) is dict and set(row) == {'cell_id', 'output', 'common_args', 'spec'}
                and row['cell_id'] == expected_id and row['output'] == str(stage / 'cells' / expected_id),
                'predeclared cell order and fresh output root')
        options = parse_common(row['common_args'], runner)
        expected = cell_spec(stage, arm, mode, row['common_args'], plan['images'], observations, build, request, reference)
        require(row['spec'] == expected and options['--device-unique-id'] == str(DEVICE), 'cell reconstruction differs')
        cell.shape(expected)
    require(plan['comparison'] == comparison_plan(plan['cells'], plan['inputs'], plan['sources'], cell),
            'predeclared comparison differs from actual cell composition')
    selected = selected_cell(plan, cell_id)
    options = parse_common(selected['common_args'], runner)
    input_plan = {**plan['inputs'], 'controller': selected['spec']['controller'], 'images': plan['images']}
    identities = legacy.inputs(input_plan, options, runner)
    return loaded, counter, evidence, cell, options, identities
