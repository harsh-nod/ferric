"""Closed component-only custody contract; no model or fence-campaign claims."""
import ast
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import re
import stat
import sys
import tarfile

D = Path(__file__).resolve().parent
CUSTODY_SHA = '66dc65822584035bfdd385f20f09578388b2dc655e6d5524aa71749422f32d14'
raw = (D / 'frozen/launch_contract.py').read_bytes()
if len(raw) > 1024**2 or hashlib.sha256(raw).hexdigest() != CUSTODY_SHA:
    raise ValueError('frozen custody helper differs')
spec = importlib.util.spec_from_file_location('component_frozen_custody', D / 'frozen/launch_contract.py')
custody = importlib.util.module_from_spec(spec)
exec(compile(raw, str(D / 'frozen/launch_contract.py'), 'exec'), custody.__dict__)
require, sha, encoded, decode = custody.require, custody.sha, custody.encoded, custody.decode
read, binding, bound, relative = custody.read, custody.binding, custody.bound, custody.relative
module = custody.module
HOST, UID, DEVICE = custody.HOST, custody.UID, custody.DEVICE
OUTPUT_NAMES = ('component',)
EMPTY_STDOUT = {'qualification/' + role + '/stdout' for role in ('component-tests', 'supervisor-tests')}
PINS = {
    'frozen/launch_contract.py': CUSTODY_SHA,
    'frozen/run_stage.py': '36d8eef5890c13c3a0ed6d1a99d327796ca9277bb3265cab0f5b21d3fe66fbed',
    'frozen/native_supervisor.py': '0c2a7cacbffbe74827ade03fd09f05a927cb9ba051ff475744be8d79f3a96436',
    'frozen/profile_v17.py': '37e1aeda338331733617ead881192b20ed769e498e1e991e271295bdb1174158',
    'frozen/process_evidence.py': 'e8d70b045e3c4b2d8a025df31d0e61193dceaa35d97b83b1d541ae1f1d72bc2d',
    'frozen/probe.py': 'a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b',
    'measurement/native_lifecycle.py': '0b4ad2aa60bfe50d962f304ae5794ad66ec507ee32b1a453ac1aa5e9a3128ac3',
    'measurement/gpu_activity.py': 'c08462f1c740759af8a9119f699955ed9789abddaac264b48bd35ef005e10730',
    'component.py': 'e9837407d7c169bd17e508648726d489204bc06a117bd2d43e4c987d4c6f1bbe',
    'fixtures.py': '30bfd7d03ecbc2d990fd79da2cad1d88cfecb2886abfa4cb81b63a613bb547d9',
    'frozen-tests/test_component.py': 'c7b659b4423237218d4a4a1db23b8e7760979003760510e323e4eb5f063b6ec9',
    'frozen-tests/test_fixtures.py': 'd38b89f7641c59b812084221f615b650f528af57bf96ca6d9ca7fbb5a4aa7f06',
}
OWN_SOURCES = ('component_contract.py', 'component_admission.py', 'component_entry.py',
               'supervise_component.py', 'test_supervisor.py', 'ordered_dispatch.py',
               'ordered_component.py', 'runtime_binding.py', 'prepare_ordered_bundle.py',
               'test_ordered_dispatch.py', 'test_ordered_component.py', 'test_ordered_bundle.py',
               'qualify_source.py')
MODES = ('latency', 'counters', 'ticks')
G36_PROFILE = 'FerricCpuFourCore36GiBEmitterV1'
QUALIFIER_SHA = 'b84d2f83eaffd7adf4f9aa760f88dd44d7fe4b2a58c23ea8a57d7beb3cfd0297'
CPU_STAGE = Path('/tmp/ferric-v16-emitter-b95a642-r1')
CPU_SOURCE = CPU_STAGE / 'component-ordered-adapter-a001/source'
CPU_HELPER = CPU_STAGE / 'inputs/component-ordered-cpu-a001/qualify_source.py'
G36_LIMITS = {
    'duration_seconds': 1200, 'individual_file_bytes': 536870912,
    'kill_wait_seconds': 3, 'log_bytes': 33554432,
    'memory_available_bytes': 137438953472, 'observed_rss_bytes': 8589934592,
    'root_free_bytes': 23622320128, 'shm_free_bytes': 34359738368,
    'stage_bytes': 38654705664, 'stage_reserve_bytes': 536870912,
    'term_grace_seconds': 10,
}
SYMBOLS = {
    'v5': ('ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5',
           'ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5'),
    'candidate': ('ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1',
                  'ferric_qwen3_c1_down_splitk8_merge_f32_r1'),
}


def stage_name(stage):
    stage = Path(stage)
    require(stage.is_absolute() and stage.parent == Path('/dev/shm')
            and re.fullmatch(r'ferric-v16-splitk-ordered-(latency|counters|ticks)-[a-zA-Z0-9_-]{1,32}', stage.name),
            'explicit component-only tmpfs stage')
    return stage


def verify_files(stage, files):
    require(type(files) is dict and 1 <= len(files) <= 512, 'bounded exact staged input roster')
    ordinary = {}
    for name, item in files.items():
        if name not in EMPTY_STDOUT:
            ordinary[name] = item
            continue
        path = stage / relative(name)
        require(type(item) is dict and set(item) == {'sha256', 'bytes', 'mode'}
                and item['bytes'] == 0 and item['mode'] == 0o600
                and item['sha256'] == hashlib.sha256(b'').hexdigest(), 'only exact empty qualification stdout')
        raw, _ = read(path, item['sha256'], 1, empty=True)
        observed = path.lstat()
        require(not raw and observed.st_uid == os.getuid() and observed.st_nlink == 1
                and stat.S_IMODE(observed.st_mode) == 0o600, 'private owned zero-byte stdout')
    # No executable, source, image, or other receipt acquires an empty-file exception.
    custody.verify_files(stage, ordinary)


def clean_cpu(result):
    expected = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
        'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
        'log_limit_exceeded': False}
    require(all(type(result.get(key)) is type(value) and result[key] == value
                for key, value in expected.items()), 'clean guarded CPU qualification required')
    profiles = dict(custody.CPU_PROFILES) | {G36_PROFILE: G36_LIMITS}
    require(result.get('profile') in profiles
            and result.get('limits') == profiles[result['profile']]
            and result.get('cpus') == [0, 1, 2, 3] and result.get('nice') == 19
            and result.get('build_jobs') == 4 and result.get('rust_test_threads') == 1,
            'unchanged qualified CPU limits')


def staged_binding(value, plan):
    binding(value)
    stage = Path(plan['stage'])
    require(Path(value['path']).is_relative_to(stage), 'review inputs must be inside capped stage')
    name = str(Path(value['path']).relative_to(stage))
    require(name in plan['files'] and plan['files'][name]['sha256'] == value['sha256'],
            'review input missing from immutable file roster')
    return value


def phase_source_roster(role, sources):
    require(role in ('component-tests', 'supervisor-tests'), 'exact qualification role')
    if role == 'supervisor-tests':
        return dict(sources)
    return {name: sources[name] for name in ('component.py', 'fixtures.py')} | {
        name: sources['frozen-tests/' + name] for name in ('test_component.py', 'test_fixtures.py')}


def qualified_test_lines(role, archive, expected):
    files, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as packed:
        for count, item in enumerate(packed, 1):
            require(count <= 512 and (item.isdir() or item.isfile()), 'bounded regular source archive')
            name = item.name[2:] if item.name.startswith('./') else item.name
            if item.isdir():
                require(name in ('', '.') or (not Path(name).is_absolute() and '..' not in Path(name).parts),
                        'canonical source archive directory')
                continue
            relative(name)
            require(name not in files and item.size <= 1024**2, 'distinct bounded source archive member')
            total += item.size
            require(total <= 8 * 1024**2, 'bounded source archive expansion')
            raw = packed.extractfile(item).read(item.size + 1)
            require(len(raw) == item.size, 'complete source member')
            files[name] = raw
    require({name for name in files if name.endswith('.py')} == set(expected), 'exact tested Python source roster')
    require(all(hashlib.sha256(files[name]).hexdigest() == digest for name, digest in expected.items()),
            'tested source archive differs from launch source roster')
    names = (('test_component.py', 'test_fixtures.py') if role == 'component-tests' else
             tuple(sorted(name for name in expected if '/' not in name and name.startswith('test_'))))
    selected = []
    for name in names:
        for cls in ast.parse(files[name], filename=name).body:
            if isinstance(cls, ast.ClassDef):
                for method in cls.body:
                    if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'):
                        selected.append((Path(name).stem, cls.name, method.name))
    require(1 <= len(selected) <= 128 and len(selected) == len(set(selected)), 'bounded distinct selected tests')
    if role == 'component-tests':
        require(len(selected) == 17, 'frozen seventeen-test component suite')
    return [method + ' (' + '.'.join((module, cls, method)) + ') ... ok'
            for module, cls, method in sorted(selected)]


def validate_phase(role, phase, plan):
    fields = {'status', 'result', 'stdout', 'stderr', 'source_archive', 'source_directory', 'argv_sha256'}
    if role == 'supervisor-tests':
        fields |= {'helper', 'inner'}
    require(type(phase) is dict and set(phase) == fields, 'exact CPU qualification custody')
    for key in fields - {'source_directory', 'argv_sha256'}:
        staged_binding(phase[key], plan)
    require(phase['stdout']['path'] == str(Path(plan['stage']) / 'qualification' / role / 'stdout'),
            'one fixed role-specific empty stdout location')
    require(read(phase['status']['path'], phase['status']['sha256'], 16)[0] == b'0\n', 'raw CPU status must pass')
    result = bound(phase['result'])
    clean_cpu(result)
    cwd, source = Path(result['cwd']), Path(phase['source_directory'])
    require(cwd.is_absolute() and source.is_absolute() and '..' not in source.parts
            and source.is_relative_to(cwd) and source != cwd, 'tested private source directory')
    inner_argv = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
                  '-s', str(source), '-p', 'test_*.py', '-v']
    if role == 'supervisor-tests':
        require(result['profile'] == G36_PROFILE and cwd == CPU_STAGE and source == CPU_SOURCE,
                'new harness requires exact G36 profile and source namespace')
        require(phase['helper']['sha256'] == QUALIFIER_SHA
                and phase['helper']['path'] == str(Path(plan['stage']) / 'qualification/supervisor-tests/helper'),
                'exact enabled charged qualifier')
        read(phase['helper']['path'], QUALIFIER_SHA, 1024**2)
        expected = ['/bin/bash', str(cwd / 'owner/cpu-env-36g-emitter.sh'),
                    '/usr/bin/python3', '-I', '-B', str(CPU_HELPER), 'harness',
                    '--archive-sha256', phase['source_archive']['sha256']]
    else:
        expected = ['/bin/bash', str(cwd / 'owner/cpu-env.sh'), *inner_argv]
    require(result['argv'] == expected and hashlib.sha256(encoded(expected)).hexdigest() == sha(phase['argv_sha256']),
            'exact role-specific guarded unittest invocation')
    archive = read(phase['source_archive']['path'], phase['source_archive']['sha256'], 8 * 1024**2)[0]
    tests = qualified_test_lines(role, archive, phase_source_roster(role, plan['sources']))
    if role == 'supervisor-tests':
        inner = bound(phase['inner'])
        require(inner.get('schema') == 'FerricOrderedComponentSourceCustodyV1'
                and inner.get('action') == 'harness' and inner.get('argv') == inner_argv
                and type(inner.get('returncode')) is int and inner['returncode'] == 0
                and inner.get('source_root') == str(source)
                and inner.get('archive_sha256') == phase['source_archive']['sha256']
                and inner.get('helper_sha256') == QUALIFIER_SHA
                and inner.get('source_before') == inner.get('source_after') == plan['sources']
                and type(inner.get('authored_test_count')) is int
                and inner['authored_test_count'] == len(tests)
                and inner.get('native_executed') is False and inner.get('native_qualified') is False,
                'exact clean inner qualification and unchanged source')
        before, after = inner.get('allocation_before'), inner.get('allocation_after')
        for allocation, allowance in ((before, 64 * 1024**2), (after, 0)):
            require(type(allocation) is dict and set(allocation) == {'stage_allocated_bytes',
                    'stage_cap_bytes', 'stage_reserve_bytes', 'remaining_reserved_bytes', 'planned_increment_bytes'}
                    and all(type(value) is int for value in allocation.values())
                    and allocation['stage_cap_bytes'] == G36_LIMITS['stage_bytes']
                    and allocation['stage_reserve_bytes'] == G36_LIMITS['stage_reserve_bytes']
                    and allocation['planned_increment_bytes'] == allowance
                    and 0 <= allocation['stage_allocated_bytes'] <= G36_LIMITS['stage_bytes']
                        - G36_LIMITS['stage_reserve_bytes'] - allowance
                    and allocation['remaining_reserved_bytes'] == G36_LIMITS['stage_bytes']
                        - G36_LIMITS['stage_reserve_bytes'] - allocation['stage_allocated_bytes'],
                    'charged unchanged G36 test envelope')
        require(after['stage_allocated_bytes'] - before['stage_allocated_bytes'] <= 64 * 1024**2,
                'test stage growth bound')
    require(read(phase['stdout']['path'], phase['stdout']['sha256'], 1024**2, empty=True)[0] == b'',
            'unittest stdout must be empty')
    stderr = read(phase['stderr']['path'], phase['stderr']['sha256'], 1024**2)[0].decode('utf-8')
    prefix = '\n'.join(tests) + '\n\n' + '-' * 70 + '\n'
    require(stderr.startswith(prefix) and re.fullmatch(r'Ran ' + str(len(tests)) +
            r' tests in [0-9]+\.[0-9]+s\n\nOK\n', stderr[len(prefix):]) is not None,
            'complete exact successful unittest roster and summary')


def validate_review(review, plan):
    require(type(review) is dict and set(review) == {'schema', 'engineering_only', 'worker_sha256',
            'images', 'source_hashes', 'cpu_phases', 'mode', 'runtime_evidence'}
            and review['schema'] == 'FerricOrderedSplitKComponentReviewedInputsV1'
            and review['engineering_only'] is True, 'explicit component engineering review')
    require(review['mode'] in MODES and review['mode'] == plan['mode'], 'bound campaign mode')
    require(review['worker_sha256'] == plan['worker']['sha256']
            and review['source_hashes'] == plan['sources'], 'reviewed source/worker custody')
    require(type(review['images']) is dict and set(review['images']) == {'v5', 'candidate'},
            'two reviewed images')
    emitters = []
    for role, value in review['images'].items():
        require(type(value) is dict and set(value) == {'sha256', 'source_sha256', 'emitter_sha256',
                'isa_review', 'symbols'} and value['sha256'] == plan['images'][role]['sha256'],
                'image review bound to exact emitted bytes')
        sha(value['source_sha256'])
        emitters.append(sha(value['emitter_sha256']))
        isa = bound(staged_binding(value['isa_review'], plan))
        require(type(isa) is dict and isa.get('schema') == 'FerricSplitKSymbolIsaReviewV1'
                and isa.get('image_sha256') == value['sha256'] and isa.get('accepted') is True
                and isa.get('symbols') == list(SYMBOLS[role]), 'retained exact symbol-scoped ISA review')
        require(value['symbols'] == {symbol: {'wavefront_size': 64, 'private_segment_bytes': 0,
                'group_segment_bytes': 0} for symbol in SYMBOLS[role]}, 'unrelaxed descriptor resource gate')
    require(emitters[0] == emitters[1], 'same selected emitter for both controls and candidate')
    require(type(review['cpu_phases']) is dict and set(review['cpu_phases']) ==
            {'component-tests', 'supervisor-tests'}, 'closed component CPU qualification roster')
    for role, phase in review['cpu_phases'].items():
        validate_phase(role, phase, plan)
    runtime = module(Path(plan['stage']) / 'runtime_binding.py', plan['sources']['runtime_binding.py'])
    runtime.validate_review(review['runtime_evidence'], plan,
                            read=read, staged_binding=staged_binding, decode=decode, require=require)


def validate_plan(plan, stage):
    require(type(plan) is dict and set(plan) == {'schema', 'stage', 'engineering_only',
            'device_unique_id', 'sources', 'files', 'worker', 'images', 'python', 'review', 'mode'}
            and plan['schema'] == 'FerricOrderedSplitKComponentLaunchPlanV1'
            and plan['stage'] == str(stage_name(stage)) and plan['engineering_only'] is True
            and plan['device_unique_id'] == DEVICE, 'closed component launch plan')
    require(plan['mode'] in MODES and stage.name.startswith('ferric-v16-splitk-ordered-' + plan['mode'] + '-'),
            'mode-specific create-only native namespace')
    require(type(plan['sources']) is dict and set(plan['sources']) == set(PINS) | set(OWN_SOURCES),
            'closed executable source roster')
    require(all(plan['sources'][name] == value for name, value in PINS.items()), 'frozen helper lineage')
    verify_files(stage, plan['files'])
    for name, digest in plan['sources'].items():
        require(plan['files'][name]['sha256'] == sha(digest), 'source file not bound')
    require(set(plan['images']) == {'v5', 'candidate'}, 'exact image pair')
    for name, item in [('worker-candidate', plan['worker']),
                       ('images/v5.hsaco', plan['images']['v5']),
                       ('images/candidate.hsaco', plan['images']['candidate'])]:
        binding(item)
        require(item['path'] == str(stage / name) and item['sha256'] == plan['files'][name]['sha256'],
                'fixed staged artifact identity')
    binding(plan['python'])
    require(Path('/usr/bin/python3').resolve(strict=True) == Path(plan['python']['path']),
            'system Python executable must be canonical and pinned')
    read(plan['python']['path'], plan['python']['sha256'])
    validate_review(bound(staged_binding(plan['review'], plan)), plan)
    supervisor = module(stage / 'frozen/native_supervisor.py', PINS['frozen/native_supervisor.py'])
    require((supervisor.HOST, supervisor.UID, int(supervisor.DEVICE_UNIQUE_ID)) == (HOST, UID, DEVICE)
            and supervisor.ROOT_FREE_BYTES == 64 * 1024**3
            and supervisor.MEMORY_AVAILABLE_BYTES == 128 * 1024**3
            and supervisor.STAGE_BYTES == 2 * 1024**3, 'unchanged native resource policy')
    profile = module(stage / 'frozen/profile_v17.py', PINS['frozen/profile_v17.py'])
    lifecycle = module(stage / 'measurement/native_lifecycle.py', PINS['measurement/native_lifecycle.py'])
    evidence = module(stage / 'frozen/process_evidence.py', PINS['frozen/process_evidence.py'])
    return supervisor, profile, lifecycle, evidence


def load_component(stage):
    # The frozen runner's sole local import is installed from its pinned bytes.
    require('fixtures' not in sys.modules, 'fixture module must not be preloaded')
    path = stage / 'fixtures.py'
    raw, _ = read(path, PINS['fixtures.py'], 1024**2)
    spec = importlib.util.spec_from_file_location('fixtures', path)
    fixtures = importlib.util.module_from_spec(spec)
    sys.modules['fixtures'] = fixtures
    try:
        exec(compile(raw, str(path), 'exec'), fixtures.__dict__)
        return module(stage / 'component.py', PINS['component.py'])
    finally:
        del sys.modules['fixtures']


def load_ordered(stage, plan):
    return tuple(module(stage / name, plan['sources'][name])
                 for name in ('ordered_component.py', 'ordered_dispatch.py'))
