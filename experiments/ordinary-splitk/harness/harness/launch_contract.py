"""Default-off same-image ordinary V19 split-K experimental admission."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat

_E = importlib.util.spec_from_file_location('splitk_experimental_evidence',
    Path(__file__).with_name('experimental_evidence.py'))
experimental = importlib.util.module_from_spec(_E)
_E.loader.exec_module(experimental)
_S = importlib.util.spec_from_file_location('splitk_selection',
    Path(__file__).with_name('measurement') / 'splitk_selection.py')
selection = importlib.util.module_from_spec(_S)
_S.loader.exec_module(selection)

HOST = 'smci350-rck-g03-b19-03'
UID = 9661
DEVICE = 16366993098680759275
RUNTIME_MAIN = experimental.RUNTIME
SPLITK_QUALIFIED = True
HARNESS_HELPER_SHA = 'c3e5b5350dc1abaa7a7e76d292878356ad40735facabbfae72a631686a3f68c6'
HARNESS_TEST_COUNTS = {'launch-tests': 60, 'measurement-tests': 214}
OLD_PLAN_SHA = 'd91216856d50fdb22ea92ad67766af44b92120af3c5afec500617991af35516d'
ROSTER_SHA = '3e7aed02029016339bba6c3efbe4504b12ba78bf0150521157bd46898ead9ade'
V19_PINS = ('36fcfee3a886fb79719351708c38c0ae9af9617d5b9d723aca103242f825456c',
            '951619f87213020aaf0eebf794a37ef523400fac1f23ce9fd90e1b78d4b3e1da')
V19_SUFFIX = 'native-inputs/v19/fe2o3-engineering-v1/0f1dc95268aca76a3aeaf69eda200875d317b5d4dbf26af7f9dcf96a4a9f581a'
SPLITK_SUFFIX = 'native-inputs/splitk/fe2o3-engineering-v1/efe280b299a95408cd5326305190c1ae7471d7fd0da62226aa2fc528153556b7'
COUNTER_PINS = {
    'run_counter_diagnostic.py': '9758475fcf059295c27199fbdb929ed0bc71cd5e56db4e28ade9bfc2c25f8e93',
    'process_evidence.py': 'e8d70b045e3c4b2d8a025df31d0e61193dceaa35d97b83b1d541ae1f1d72bc2d',
}
OWN_SOURCES = ('launch_contract.py', 'experimental_evidence.py', 'prepare_stage.py', 'run_stage.py',
    'test_native_launch.py', 'test_splitk_selection.py', 'test_cpu_binding.py',
    'fixtures/frozen_run_v17_native.py', 'fixtures/frozen_run_v25_native.py', 'fixtures/retained_cpu.json')
MEASUREMENT_SOURCES = ('native_token_cell.py', 'abba_ledger.py', 'gpu_activity.py', 'native_campaign_replay.py',
    'native_lifecycle.py', 'splitk_selection.py', 'test_abba_ledger.py', 'test_gpu_activity.py',
    'test_native_campaign_replay.py', 'test_native_lifecycle.py', 'test_native_token_cell.py')
PHASES = experimental.PHASES


def phase_receipt_fields(name):
    require(name in PHASES, 'closed experimental CPU role')
    return experimental.fields(name)


MODEL_BUNDLE = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
MODEL_TARGET = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
IMAGE_FIELDS = {'--target-artifact': None, '--target-head-artifact': 'fp32_head_artifact',
    '--argmax-artifact': 'argmax_artifact', '--attention-artifact': 'attention_artifact',
    '--rmsnorm-artifact': 'rmsnorm_artifact', '--split-attention-artifact': 'split_attention_artifact',
    '--prefill-kv-artifact': 'prefill_kv_artifact', '--gemv-artifact': 'gemv_artifact',
    '--ordered64-kv-copy-artifact': 'kv_copy_artifact', '--splitk-down-artifact': 'splitk_down'}
EXTRA_IMAGES = ('--split-attention-artifact', '--prefill-kv-artifact', '--gemv-artifact',
                '--ordered64-kv-copy-artifact')
CELL_ORDER = (('counter-A', 'A', 'correctness'), ('counter-B', 'B', 'correctness')) + tuple(
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
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(descriptor, 'rb') as source:
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
            and re.fullmatch(r'ferric-native-splitk-down-[a-zA-Z0-9_-]{1,48}', stage.name),
            'fresh bounded split-K tmpfs stage name')
    return stage


def validate_build(build, *, admission=True):
    if admission:
        require(SPLITK_QUALIFIED is True and type(HARNESS_HELPER_SHA) is str
                and type(HARNESS_TEST_COUNTS) is dict,
                'split-K native admission is disabled pending exact harness qualification')
    require(type(build) is dict and set(build) == {'schema', 'runtime_main', 'runtime_source',
        'controller_source', 'worker', 'controller', 'cpu_qualification', 'experimental_retention', 'splitk'}
        and build['schema'] == 'FerricSplitKExperimentalBuildV1' and build['runtime_main'] == RUNTIME_MAIN,
        'closed exact experimental build binding')
    wanted = {'runtime_source': experimental.RUNTIME_SOURCE, 'controller_source': experimental.CLIENT_SOURCE,
              'worker': experimental.WORKER, 'controller': experimental.CONTROLLER,
              'experimental_retention': experimental.RETENTION}
    for name, digest in wanted.items():
        binding(build[name])
        require(build[name]['sha256'] == digest, 'exact source/release/custody identity: ' + name)
    binding(build['cpu_qualification'])
    validate_splitk(build['splitk'])


def validate_splitk(value):
    require(type(value) is dict and set(value) == {'image', 'roster'}, 'closed split-K artifact binding')
    image, roster = value['image'], value['roster']
    require(type(image) is dict and set(image) == {'path', 'hsaco', 'manifest', 'handoff'}
            and type(image['path']) is str and Path(image['path']).is_absolute(), 'exact image directory')
    require(type(roster) is dict and set(roster) == {'path', 'sha256', 'value'}, 'closed compiler roster binding')
    binding({key: roster[key] for key in ('path', 'sha256')})
    selection.expected_metadata({'arm': 'A', 'splitk': value})


def validate_cpu(cpu, build, read_bound=bound, read_raw=read):
    return experimental.validate(_ContractApi(), cpu, build, read_bound, read_raw)


class _ContractApi:
    # Dynamic source loaders do not all install their module in sys.modules.
    def __getattr__(self, key):
        return globals()[key]


def remap_common(old, stage, worker_sha, requests, arm):
    require(requests in (1, 6), 'one correctness/counter or six latency requests')
    require(arm in ('A', 'B'), 'closed down arm required')
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
    require(arm in ('A', 'B') and mode in ('correctness', 'latency'), 'closed cell selector')
    controller = {'path': str(stage / 'controller'), 'sha256': build['controller']['sha256']}
    argv = [controller['path']]
    splitk = build['splitk']
    for name, value in (('--splitk-down-mode', selection.ARMS[arm]),
        ('--splitk-down-artifact', splitk['image']['path']), ('--splitk-down-roster', splitk['roster']['path']),
        ('--splitk-down-roster-sha256', splitk['roster']['sha256']),
        ('--splitk-down-hsaco-sha256', selection.IMAGE['hsaco']),
        ('--splitk-down-manifest-sha256', selection.IMAGE['manifest']),
        ('--splitk-down-handoff-sha256', selection.IMAGE['handoff'])):
        argv += [name, value]
    argv += list(common)
    argv += ['--wave-target-mode', 'combined']
    for key in EXTRA_IMAGES:
        argv += [key, images[key]['path']]
    argv += ['--prefill-kv-mode', 'parallel-prefill16-v27', '--split-attention-mode', 'split8-v21',
             '--c1-packet-mode', 'packed64-v29', '--gemv-mode', 'baseline', '--ordered64-kv-copy-mode', 'parallel-c1-v19']
    setup = {'model_bundle_id': MODEL_BUNDLE, 'target_model_id': MODEL_TARGET}
    profile, closed = {}, {}
    for option, field in IMAGE_FIELDS.items():
        if option == '--splitk-down-artifact':
            continue
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
    return {'schema': 'FerricSplitKModelCellPlanV1', 'arm': arm, 'mode': mode, 'argv': argv,
        'controller': controller, 'worker': {'path': str(stage / 'worker-candidate'), 'sha256': build['worker']['sha256']},
        'splitk': splitk, 'experimental_retention': build['experimental_retention'],
        'device_unique_id': DEVICE, 'prompt': request['prompt'], 'reference': reference,
        'setup_expected': setup, 'profile_expected': profile, 'closed_expected': closed,
        'timeouts': {'setup_seconds': 600, 'request_seconds': 180, 'cell_seconds': 1200}}


def campaign_cells(stage, old, images, observations, build, request, reference):
    cells = []
    for cell_id, arm, mode in CELL_ORDER:
        common = remap_common(old, stage, build['worker']['sha256'], 6 if mode == 'latency' else 1, arm)
        cells.append({'cell_id': cell_id, 'output': str(stage / 'cells' / cell_id), 'common_args': common,
                      'spec': cell_spec(stage, arm, mode, common, images, observations, build, request, reference)})
    return cells


def comparison_plan(cells, inputs, sources, cell):
    profiles = {arm: cell.profile_for_spec(next(row['spec'] for row in cells
        if row['spec']['arm'] == arm and row['spec']['mode'] == 'latency')) for arm in ('A', 'B')}
    plan = {'schema': 'FerricSplitKModelChangePlanV1', 'change_id': 'ordinary-v19-down-wave-vs-unpaired-splitk8',
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
        empty_log = item['bytes'] == 0 and len(Path(name).parts) == 3 and Path(name).parts[0] == 'evidence' \
            and Path(name).parts[1] in PHASES and Path(name).name in ('stdout', 'stderr')
        raw, _ = read(path, item['sha256'], empty=empty_log)
        observed = path.lstat()
        require(len(raw) == item['bytes'] and observed.st_uid == os.getuid() and observed.st_nlink == 1
                and stat.S_IMODE(observed.st_mode) == item['mode'] in (0o600, 0o700), 'staged input identity')
        total += len(raw)
    require(total < 2 * 1024**3 - 512 * 1024**2, 'staged inputs leave bounded output headroom')


def validate_cpu_stage(stage, files, cpu):
    items = [item for name, phase in cpu['phases'].items() for field, item in phase.items()
             if field in phase_receipt_fields(name)]
    items += [cpu['experimental_retention'], *cpu['runtime_evidence'].values()]
    for item in items:
        binding(item)
        path = Path(item['path'])
        require(path.is_relative_to(stage), 'CPU receipt outside immutable stage')
        relative_path = str(relative(str(path.relative_to(stage))))
        require(relative_path in files and files[relative_path]['sha256'] == item['sha256'],
                'CPU receipt missing from closed staged file roster')


def validate_plan(plan, stage, cell_id='counter-A'):
    require(type(plan) is dict and set(plan) == {'schema', 'stage', 'cells', 'images', 'inputs',
            'files', 'build', 'cpu', 'sources', 'engineering_only', 'comparison'}
            and plan['schema'] == 'FerricSplitKModelCampaignPlanV1' and plan['stage'] == str(stage)
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
    for name in ('worker', 'controller', 'runtime_source', 'controller_source', 'experimental_retention'):
        item = build[name]
        path = Path(item['path'])
        require(path.is_relative_to(stage) and files_member(plan['files'], stage, item),
                'build artifact outside closed staged roster')
    splitk = build['splitk']
    require(bound({key: splitk['roster'][key] for key in ('path', 'sha256')}) == splitk['roster']['value']
            and files_member(plan['files'], stage, splitk['roster']), 'exact staged compiler roster')
    require(plan['images']['--splitk-down-artifact'] == {'path': splitk['image']['path'],
        'manifest_sha256': selection.IMAGE['manifest'], 'hsaco_sha256': selection.IMAGE['hsaco']},
        'split-K image in exact same image roster')
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


def files_member(files, stage, item):
    path = Path(item['path'])
    return path.is_relative_to(stage) and files.get(str(path.relative_to(stage)), {}).get('sha256') == item['sha256']
