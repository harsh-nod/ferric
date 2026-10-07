"""Source-only selector for one packet-tick diagnostic, never a latency comparison.

Native admission remains disabled until actual dedicated packet qualification.
Current55c runtime evidence and new baseline G36 controller roles stay separate.
"""
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import stat
import tarfile

HOST = 'smci350-rck-g03-b19-03'
UID = 9661
DEVICE = 16366993098680759275
RUNTIME_MAIN = '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
WORKER_SHA = 'af246e5b872b639b90906336493629eb2bbfa90ff57ffc4ffd859f8b963d9b30'
OLD_PLAN_SHA = 'd91216856d50fdb22ea92ad67766af44b92120af3c5afec500617991af35516d'
ROSTER_SHA = '3e7aed02029016339bba6c3efbe4504b12ba78bf0150521157bd46898ead9ade'
COUNTER_PINS = {
    'run_counter_diagnostic.py': '9758475fcf059295c27199fbdb929ed0bc71cd5e56db4e28ade9bfc2c25f8e93',
    'process_evidence.py': 'e8d70b045e3c4b2d8a025df31d0e61193dceaa35d97b83b1d541ae1f1d72bc2d',
}
OWN_SOURCES = ('launch_contract.py', 'prepare_stage.py', 'run_stage.py',
               'test_native_launch.py', 'test_cpu_binding.py', 'controller_binding.py', 'runtime_binding.py',
               'controller_qualifier.py', 'v19_qualifier.py', 'fixtures/source-files.json', 'fixtures/source-modes.json')
MEASUREMENT_SOURCES = ('native_token_cell.py', 'abba_ledger.py', 'gpu_activity.py',
    'native_campaign_replay.py', 'native_lifecycle.py', 'packet_ticks.py',
    'test_gpu_activity.py', 'test_native_lifecycle.py')
EXTERNAL_SOURCES = ('bind_build.py', 'test_packet_ticks.py', 'test_packet_v19_cell.py', 'prepare_qualification.py')
V19_PINS = ('36fcfee3a886fb79719351708c38c0ae9af9617d5b9d723aca103242f825456c',
            '951619f87213020aaf0eebf794a37ef523400fac1f23ce9fd90e1b78d4b3e1da')
V19_SUFFIX = 'native-inputs/v19/fe2o3-engineering-v1/0f1dc95268aca76a3aeaf69eda200875d317b5d4dbf26af7f9dcf96a4a9f581a'
RUNTIME_SOURCE_SHA = 'a9772e55f9a140d0749a621c07ea2c9cb19c36b2416656278dd8d2a2095321d3'
BASE_CONTROLLER_SOURCE_SHA = '6089abea6ebc3363557c8e5fb8b4e2e488fa49b4f30f99029dfcc16b7c5bddd7'
PACKET_PROFILE = 'FerricCpuFourCore36GiBEmitterV1'
CPU_LIMITS = {'duration_seconds': 1200, 'individual_file_bytes': 536870912,
    'kill_wait_seconds': 3, 'log_bytes': 33554432, 'memory_available_bytes': 137438953472,
    'observed_rss_bytes': 8589934592, 'root_free_bytes': 23622320128,
    'shm_free_bytes': 34359738368, 'stage_bytes': 38654705664,
    'stage_reserve_bytes': 536870912, 'term_grace_seconds': 10}
CPU_PROFILES = {PACKET_PROFILE: CPU_LIMITS}
GUARD_SOURCE_PINS = {PACKET_PROFILE: {
    'guard': 'fba93769c349a1bca0ea72ff2dbc0ed9b077be712dc8f370775d050df96af796',
    'environment': 'a4f373e86b56bfc692b56d726280fc1f7efbfa521db96e936dd92787d7b2c7b8'}}
D = '/tmp/ferric-v16-emitter-b95a642-r1'
ENV_WRAPPERS = {PACKET_PROFILE: D + '/owner/cpu-env-36g-emitter.sh'}
CLIENT_MANIFEST = D + '/client-packet-baseline-55c-a004/source/adapters/m1-engineering-execution-v1/Cargo.toml'
HARNESS_ROOT = D + '/packet-v19-55c-harness-a003/source'
HARNESS_QUALIFIER = D + '/inputs/packet-v19-harness-a003/prepare_qualification.py'
PACKET_BINARY = 'ferric-qwen3-ordered64-baseline-packet-ticks'
PACKET_FEATURES = 'c1-ordered64,model-timestamps'
PACKET_QUALIFIER = D + '/inputs/packet-baseline-55c-a004/qualify_controller.py'
PACKET_QUALIFIER_SHA = '5d30ac68ac5c6cc0a046764e30accb74117cd077b934d5c83f3b3f617a5663b1'
V19_BINARY = 'ferric-qwen3-ordered64-packet-ticks'
V19_QUALIFIER = D + '/inputs/packet-v19-55c-a001/qualify_v19.py'
V19_QUALIFIER_SHA = '0660dc79d28000e78f972fe4dc4460f63271671875526fa43539c329de21fe7f'
V19_COMMANDS = {'v19-' + role: role for role in ('graph', 'parser', 'clippy', 'release', 'retention')}
CLIENT_COMMANDS = {'packet-' + role: role for role in
    ('stage', 'format', 'timestamps', 'graph', 'parser', 'source-policy', 'clippy', 'release', 'retention')}
RUST_TEST_ROLES = {'packet-timestamps', 'packet-graph', 'packet-parser', 'packet-source-policy',
                   'v19-graph', 'v19-parser'}
PYTHON_COMMANDS = {
    'launch-tests': (HARNESS_ROOT + '/harness', 'test_*.py'),
    'measurement-tests': (HARNESS_ROOT + '/harness/measurement', 'test_*.py'),
    'packet-replay-tests': (HARNESS_ROOT, 'test_packet*.py'),
}
PYTHON_COUNTS = {'launch-tests': 31, 'measurement-tests': 130, 'packet-replay-tests': 24}
PHASES = set(CLIENT_COMMANDS) | set(V19_COMMANDS) | set(PYTHON_COMMANDS)
CONTROLLERS = ('diagnostic',)
PACKET_TICKS_QUALIFIED = True


def phase_receipt_fields(name):
    require(name in PHASES, 'known packet CPU phase role required')
    if name in CLIENT_COMMANDS or name in V19_COMMANDS:
        return ('status', 'result', 'stdout', 'stderr', 'inner', 'helper')
    return ('status', 'result', 'stderr', 'inner', 'helper')


def expected_command(name, archive_sha=None):
    prefix = ['/bin/bash', ENV_WRAPPERS[PACKET_PROFILE], '/usr/bin/python3', '-I', '-B']
    if name in CLIENT_COMMANDS:
        return [*prefix, PACKET_QUALIFIER, CLIENT_COMMANDS[name]]
    if name in V19_COMMANDS:
        return [*prefix, V19_QUALIFIER, V19_COMMANDS[name]]
    require(name in PYTHON_COMMANDS, 'dedicated Python qualification role required')
    return [*prefix, HARNESS_QUALIFIER, name, '--archive-sha256', sha(archive_sha)]


MODEL_BUNDLE = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
MODEL_TARGET = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
IMAGE_FIELDS = {'--target-artifact': None, '--target-head-artifact': 'fp32_head_artifact',
    '--argmax-artifact': 'argmax_artifact', '--attention-artifact': 'attention_artifact',
    '--rmsnorm-artifact': 'rmsnorm_artifact', '--split-attention-artifact': 'split_attention_artifact',
    '--prefill-kv-artifact': 'prefill_kv_artifact', '--gemv-artifact': 'gemv_artifact',
    '--ordered64-kv-copy-artifact': 'kv_copy_artifact'}
EXTRA_IMAGES = ('--split-attention-artifact', '--prefill-kv-artifact', '--gemv-artifact',
                '--ordered64-kv-copy-artifact')
CELL_ORDER = (('counter-A', 'A', 'counters'),)


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
    name = 'v14_' + Path(path).stem
    spec = importlib.util.spec_from_file_location(name, path,
        loader=importlib.machinery.SourceFileLoader(name, str(path)))
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
            and re.fullmatch(r'ferric-packet-v19-55c-[a-zA-Z0-9_-]{1,48}', stage.name),
            'fresh bounded packet-tick tmpfs stage name')
    return stage


def validate_build(build):
    require(PACKET_TICKS_QUALIFIED, 'source-only packet-tick binding: actual controller and CPU evidence not qualified')
    validate_build_inputs(build)


def validate_build_inputs(build):
    require(type(build) is dict and set(build) == {'schema', 'runtime_main', 'runtime_source',
            'controller_source', 'worker', 'controllers', 'cpu_qualification'}
            and build['schema'] == 'FerricV19Packet55cBuildBindingV1'
            and build['runtime_main'] == RUNTIME_MAIN, 'runtime baseline-base/source build binding')
    for key in ('runtime_source', 'controller_source', 'worker', 'cpu_qualification'):
        binding(build[key])
    require(build['worker']['sha256'] == WORKER_SHA
            and build['runtime_source']['sha256'] == RUNTIME_SOURCE_SHA,
            'qualified published55c worker and exact source required')
    require(type(build['controllers']) is dict and set(build['controllers']) == set(CONTROLLERS),
            'one separate diagnostic binary required')
    for value in build['controllers'].values():
        binding(value)
    require(len({item['sha256'] for item in build['controllers'].values()}) == 1,
            'one explicit diagnostic binary required')


def validate_guard_sources(value, read_raw=read):
    require(type(value) is dict and set(value) == set(GUARD_SOURCE_PINS),
            'separate published-runtime and packet guard source rosters required')
    for profile, sources in value.items():
        expected = GUARD_SOURCE_PINS[profile]
        require(type(sources) is dict and set(sources) == set(expected),
                'exact guard/entry/environment roster: ' + profile)
        for name, item in sources.items():
            binding(item)
            require(item['sha256'] == expected[name], 'guard source differs: ' + profile + '/' + name)
            read_raw(item['path'], item['sha256'], 1024**2)


def source_archive_map(raw):
    require(type(raw) is bytes and 0 < len(raw) <= 64 * 1024**2, 'bounded controller source archive')
    files, total = {}, 0
    try:
        with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
            for ordinal, member in enumerate(archive, 1):
                require(ordinal <= 10000, 'source archive member bound')
                path = Path(member.name)
                require(not path.is_absolute() and '..' not in path.parts,
                        'relative controller source member required')
                name = str(path)
                require(member.name in (name, './' + name, name + '/', './' + name + '/'),
                        'canonical controller source member required')
                if member.isdir():
                    continue
                require(member.isfile() and name != '.' and name not in files,
                        'distinct regular controller source member required')
                require(0 <= member.size <= 32 * 1024**2, 'source file expansion bound')
                total += member.size
                require(total <= 256 * 1024**2, 'source archive expansion bound')
                source = archive.extractfile(member)
                require(source is not None, 'regular source member content required')
                with source:
                    data = source.read(member.size + 1)
                require(len(data) == member.size, 'source archive member size changed')
                files[name] = hashlib.sha256(data).hexdigest()
    except (tarfile.TarError, OSError, EOFError) as error:
        raise ValueError('invalid controller source archive') from error
    require(files, 'nonempty controller source file roster')
    return files


def controller_binding_module(expected):
    return module(Path(__file__).with_name('controller_binding.py'),
                  sha(expected))


def validate_cpu(cpu, build, read_bound=bound, read_raw=read):
    import types
    api = types.SimpleNamespace(require=require, binding=binding, sha=sha, decode=decode,
        encoded=encoded, module=module, RUNTIME_MAIN=RUNTIME_MAIN, PACKET_BINARY=PACKET_BINARY,
        PYTHON_COMMANDS=PYTHON_COMMANDS, PYTHON_COUNTS=PYTHON_COUNTS, D=D, HARNESS_ROOT=HARNESS_ROOT,
        V19_BINARY=V19_BINARY)
    require(type(cpu) is dict and set(cpu) == {'schema', 'runtime_source_sha256',
            'controller_source_sha256', 'worker_sha256', 'controllers', 'phases',
            'harness_sources', 'external_sources', 'guard_sources', 'controller_evidence', 'runtime_evidence',
            'harness_evidence', 'v19_evidence'}
            and cpu['schema'] == 'FerricV19Packet55cCpuQualificationV1',
            'closed current baseline packet CPU qualification')
    require(cpu['runtime_source_sha256'] == build['runtime_source']['sha256'] == RUNTIME_SOURCE_SHA
            and cpu['controller_source_sha256'] == build['controller_source']['sha256']
            and cpu['worker_sha256'] == build['worker']['sha256'] == WORKER_SHA
            and cpu['controllers'] == {key: value['sha256'] for key, value in build['controllers'].items()},
            'exact source and binary CPU binding')
    require(type(cpu['phases']) is dict and set(cpu['phases']) == PHASES, 'complete current phase roster')
    require(type(cpu['harness_sources']) is dict and set(cpu['harness_sources']) ==
            set(OWN_SOURCES) | {'measurement/' + name for name in MEASUREMENT_SOURCES},
            'complete tested harness source roster')
    require(type(cpu['external_sources']) is dict and set(cpu['external_sources']) == set(EXTERNAL_SOURCES),
            'complete tested binder and sidecar fixture roster')
    for digest in (*cpu['harness_sources'].values(), *cpu['external_sources'].values()):
        sha(digest)
    for name in ('controller_binding.py', 'runtime_binding.py'):
        read(Path(__file__).with_name(name), cpu['harness_sources'][name], 1024**2)
    policy = controller_binding_module(cpu['harness_sources']['controller_binding.py'])
    validate_guard_sources(cpu['guard_sources'], read_raw)
    policy.runtime_evidence(api, cpu['runtime_evidence'], build, read_raw)
    qualifier, source_sha, source_bytes = policy.validate_sources(api, cpu['controller_evidence'],
                                                                 build, read_bound, read_raw)
    v19_qualifier = policy.validate_v19_evidence(api, cpu['v19_evidence'], source_sha, read_raw)
    harness_sources = policy.validate_harness_sources(api, cpu, read_raw)
    executed, inners = {}, {}
    for name, phase in cpu['phases'].items():
        require(type(phase) is dict and set(phase) == set(phase_receipt_fields(name)) | {'profile', 'argv_sha256'}
                and phase['profile'] == PACKET_PROFILE, 'exact role/profile/receipt mapping')
        for field in phase_receipt_fields(name):
            binding(phase[field])
        require(read_raw(phase['status']['path'], phase['status']['sha256'], 16)[0] == b'0\n',
                'zero raw role status')
        result = read_bound(phase['result'])
        wanted = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
            'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
            'log_limit_exceeded': False, 'profile': PACKET_PROFILE, 'cwd': D}
        require(all(type(result.get(key)) is type(value) and result[key] == value
                    for key, value in wanted.items()), 'clean exact G36 role result')
        require(result.get('limits') == CPU_LIMITS and all(type(value) is int for value in result['limits'].values())
                and result.get('cpus') == [0, 1, 2, 3] and all(type(value) is int for value in result['cpus'])
                and type(result.get('nice')) is int and result['nice'] == 19
                and type(result.get('build_jobs')) is int and result['build_jobs'] == 4
                and type(result.get('rust_test_threads')) is int and result['rust_test_threads'] == 1,
                'unchanged CPU limits and placement')
        environment = result.get('launch_environment', {})
        require(all(environment.get(key) == value for key, value in {
            'CARGO_BUILD_JOBS': '4', 'RUST_TEST_THREADS': '1', 'FERRIC_CPU_PROFILE': PACKET_PROFILE,
            'CUDA_VISIBLE_DEVICES': '-1', 'HIP_VISIBLE_DEVICES': '-1', 'HSA_VISIBLE_DEVICES': '-1',
            'ROCR_VISIBLE_DEVICES': '-1'}.items()), 'masked GPU visibility')
        require(type(result.get('peak_observed_rss_bytes')) is int
                and 0 <= result['peak_observed_rss_bytes'] <= CPU_LIMITS['observed_rss_bytes'], 'CPU RSS bound')
        for key in ('admission', 'final_resources'):
            resources = result.get(key, {})
            require(all(type(resources.get(field)) is int and resources[field] >= CPU_LIMITS[field]
                        for field in ('memory_available_bytes', 'root_free_bytes', 'shm_free_bytes'))
                    and type(resources.get('stage_bytes')) is int
                    and 0 <= resources['stage_bytes'] <= CPU_LIMITS['stage_bytes'] - CPU_LIMITS['stage_reserve_bytes'],
                    'unchanged host and stage floors')
        argv = result.get('argv')
        require(argv == expected_command(name, cpu['harness_evidence']['archive']['sha256'])
                and hashlib.sha256(encoded(argv)).hexdigest()
                == sha(phase['argv_sha256']), 'exact wrapped/direct role argv')
        if name in CLIENT_COMMANDS:
            require(phase['helper']['sha256'] == PACKET_QUALIFIER_SHA, 'exact wrapped controller qualifier')
            read_raw(phase['helper']['path'], PACKET_QUALIFIER_SHA, 1024**2)
            inner = inners[name] = read_bound(phase['inner'])
            policy.validate_inner(api, name, inner, qualifier, source_sha)
            for stream in ('stdout', 'stderr'):
                read_raw(phase[stream]['path'], phase[stream]['sha256'], CPU_LIMITS['log_bytes'], empty=True)
        elif name in V19_COMMANDS:
            require(phase['helper']['sha256'] == V19_QUALIFIER_SHA, 'exact V19 controller qualifier')
            read_raw(phase['helper']['path'], V19_QUALIFIER_SHA, 1024**2)
            inner = inners[name] = read_bound(phase['inner'])
            policy.validate_v19_inner(api, name, inner, v19_qualifier, qualifier, cpu['phases'])
            for stream in ('stdout', 'stderr'):
                read_raw(phase[stream]['path'], phase[stream]['sha256'], CPU_LIMITS['log_bytes'], empty=True)
        else:
            require(phase['helper']['sha256'] == cpu['harness_evidence']['helper']['sha256'],
                    'same qualified harness helper')
            read_raw(phase['helper']['path'], phase['helper']['sha256'], 1024**2)
            policy.validate_harness_inner(api, name, read_bound(phase['inner']), cpu, harness_sources)
        if name in RUST_TEST_ROLES or name in PYTHON_COMMANDS:
            stream = 'stdout' if name in RUST_TEST_ROLES else 'stderr'
            output = read_raw(phase[stream]['path'], phase[stream]['sha256'], CPU_LIMITS['log_bytes'])[0]
            executed[name] = policy.test_counts(api, name, output)
    baseline_build = {**build, 'controllers': {'diagnostic': cpu['v19_evidence']['baseline_controller']}}
    policy.validate_retention(api, inners, cpu['phases'], baseline_build, source_bytes, read_raw)
    policy.validate_v19_retention(api, inners, cpu['phases'], build, read_raw)
    return executed


def remap_common(old, stage, worker_sha, requests):
    require(requests == 1, 'one diagnostic request required')
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
    require(actual == '135', 'closed one-request batch budget')
    projected[index] = '540'
    options = runner.arguments(projected)
    options['--max-batches'] = actual
    return options


def cell_spec(stage, arm, mode, common, images, observations, build, request, reference):
    require(arm == 'A' and mode == 'counters', 'closed one-request diagnostic selector')
    controller_key = 'diagnostic'
    controller = {'path': str(stage / ('controller-' + controller_key)),
                  'sha256': build['controllers'][controller_key]['sha256']}
    argv = [controller['path']]
    argv += list(common)
    argv += ['--wave-target-mode', 'combined']
    for key in EXTRA_IMAGES:
        argv += [key, images[key]['path']]
    argv += ['--prefill-kv-mode', 'parallel-prefill16-v27', '--split-attention-mode', 'split8-v21',
             '--c1-packet-mode', 'packed64-v29', '--gemv-mode', 'baseline',
             '--ordered64-kv-copy-mode', 'parallel-c1-v19']
    sidecar = str(stage / 'cells' / 'counter-A' / 'cell-results' / 'packet-ticks.json')
    argv += ['--ordered64-packet-ticks', sidecar]
    setup = {'model_bundle_id': MODEL_BUNDLE, 'target_model_id': MODEL_TARGET}
    profile, closed, catalog = {}, {}, {}
    for option, field in IMAGE_FIELDS.items():
        image = images[option]
        hsaco = observations[option]['hsaco']
        require(hsaco['identity']['sha256'] == image['hsaco_sha256'], 'observed HSACO identity differs')
        names = hsaco['kernel_names']
        require(type(names) is list and names and len(names) == len(set(names)), 'exact image exports required')
        for name in names:
            require(type(name) is str and re.fullmatch('[A-Za-z0-9_]{1,256}', name)
                    and name not in catalog, 'loaded image exports must be disjoint')
            catalog[name] = sha(image['hsaco_sha256'])
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
    return {'schema': 'FerricPacketTicksCellPlanV1', 'arm': arm, 'mode': mode, 'argv': argv,
        'packet_sidecar': sidecar, 'kernel_catalog': catalog,
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
    require(len(cells) == 1, 'one diagnostic cell required')
    return {'schema': 'FerricPacketTicksDiagnosticPlanV1',
            'cell_spec_sha256': cell.ledger.digest(cells[0]['spec']), 'latency_admitted': False,
            'scope': 'one instrumented ordered64 request; uncalibrated non-shader-only packet ticks'}


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
        raw, _ = read(path, item['sha256'], empty=item['bytes'] == 0)
        observed = path.lstat()
        require(len(raw) == item['bytes'] and observed.st_uid == os.getuid() and observed.st_nlink == 1
                and stat.S_IMODE(observed.st_mode) == item['mode'] in (0o600, 0o700), 'staged input identity')
        total += len(raw)
    require(total < 2 * 1024**3 - 512 * 1024**2, 'staged inputs leave bounded output headroom')


def validate_cpu_stage(stage, files, cpu):
    items = [phase[field] for name, phase in cpu['phases'].items() for field in phase_receipt_fields(name)]
    items.extend(item for sources in cpu['guard_sources'].values() for item in sources.values())
    items.extend(cpu['controller_evidence'].values())
    items.extend(cpu['runtime_evidence'].values())
    items.extend(cpu['harness_evidence'].values())
    items.extend(cpu['v19_evidence'].values())
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
            and plan['schema'] == 'FerricV19Packet55cPlanV1' and plan['stage'] == str(stage)
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
    require(all(plan['sources'].get(name) == digest for name, digest in cpu['harness_sources'].items())
            and all(plan['sources'].get('qualification/' + name) == digest
                    for name, digest in cpu['external_sources'].items()),
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
    require(type(plan['cells']) is list and len(plan['cells']) == len(CELL_ORDER), 'exact one-cell diagnostic plan')
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
