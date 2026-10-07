"""Exact M32 component custody; no model or same-compiler isolation claims."""
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
NATIVE_ENABLED = True
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
EMPTY_STDOUT = {'qualification/harness/stdout'}
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
    'frozen/component_fixtures.py': '30bfd7d03ecbc2d990fd79da2cad1d88cfecb2886abfa4cb81b63a613bb547d9',
}
OWN_SOURCES = ('component_contract.py', 'component_admission.py', 'component_entry.py',
               'supervise_component.py', 'test_supervisor.py', 'ordered_dispatch.py',
               'm32_component.py', 'm32_provenance.py', 'runtime_binding.py', 'prepare_m32_bundle.py',
               'fixtures.py', 'slice_views.py', 'test_component_inputs.py', 'test_m32_component.py',
               'test_ordered_dispatch.py', 'test_m32_bundle.py', 'test_m32_provenance.py',
               'qualify_source.py')
MODES = ('latency',)
CPU_PROFILE = 'FerricCpuFourCore40GiBEmitterV1'
QUALIFIER_SHA = 'db542f327119cb57475924ce3d24b09eb87147c68227748ed409100aa686c508'
CPU_STAGE = Path('/tmp/ferric-v16-emitter-b95a642-r1')
CPU_SOURCE = CPU_STAGE / 'prefill-m32-harness-a001/source'
CPU_HELPER = CPU_STAGE / 'inputs/prefill-m32-harness-a001/qualify_source.py'
CPU_LIMITS = {
    'duration_seconds': 1200, 'individual_file_bytes': 536870912,
    'kill_wait_seconds': 3, 'log_bytes': 33554432,
    'memory_available_bytes': 137438953472, 'observed_rss_bytes': 8589934592,
    'root_free_bytes': 23622320128, 'shm_free_bytes': 34359738368,
    'stage_bytes': 42949672960, 'stage_reserve_bytes': 536870912,
    'term_grace_seconds': 10,
}
SYMBOLS = {
    'v5': ('ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5',),
    'm2': ('ferric_qwen3_prefill32_m2_gate_up_bf16_r1',),
}
IMAGE_PINS = {
    'v5': '98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502',
    'm2': 'bb164d23d55dec43e55b8d8cce936929a3cc749a29486bdcfff8bfe34ac7d5dc',
}


def stage_name(stage):
    stage = Path(stage)
    require(stage.is_absolute() and stage.parent == Path('/dev/shm')
            and re.fullmatch(r'ferric-prefill-m32-component-latency-[a-zA-Z0-9_-]{1,32}', stage.name),
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
    profiles = dict(custody.CPU_PROFILES) | {CPU_PROFILE: CPU_LIMITS}
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


def qualified_test_lines(archive, expected):
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
    names = tuple(sorted(name for name in expected if '/' not in name and name.startswith('test_')))
    selected = []
    for name in names:
        for cls in ast.parse(files[name], filename=name).body:
            if isinstance(cls, ast.ClassDef):
                for method in cls.body:
                    if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'):
                        selected.append((Path(name).stem, cls.name, method.name))
    require(1 <= len(selected) <= 128 and len(selected) == len(set(selected)), 'bounded distinct selected tests')
    return [method + ' (' + '.'.join((module, cls, method)) + ') ... ok'
            for module, cls, method in sorted(selected)]


def validate_phase(phase, plan):
    fields = {'status', 'result', 'stdout', 'stderr', 'source_archive', 'helper', 'inner'}
    require(type(phase) is dict and set(phase) == fields, 'exact fresh harness qualification')
    for key in fields:
        staged_binding(phase[key], plan)
        require(phase[key]['path'] == str(Path(plan['stage']) / 'qualification/harness' / key),
                'fixed qualification custody location')
    require(read(phase['status']['path'], phase['status']['sha256'], 16)[0] == b'0\n', 'raw CPU status must pass')
    result, inner = bound(phase['result']), bound(phase['inner'])
    clean_cpu(result)
    require(result['profile'] == CPU_PROFILE and result['cwd'] == str(CPU_STAGE),
            'exact G40 full harness test profile')
    require(phase['helper']['sha256'] == QUALIFIER_SHA, 'exact charged harness qualifier')
    read(phase['helper']['path'], QUALIFIER_SHA, 1024**2)
    command = ['/bin/bash', str(CPU_STAGE / 'owner/cpu-env-40g-emitter.sh'),
               '/usr/bin/python3', '-I', '-B', str(CPU_HELPER), phase['source_archive']['sha256']]
    require(result['argv'] == command, 'exact guarded full-closure invocation')
    archive = read(phase['source_archive']['path'], phase['source_archive']['sha256'], 8 * 1024**2)[0]
    tests = qualified_test_lines(archive, plan['sources'])
    argv = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
            '-s', str(CPU_SOURCE), '-p', 'test_*.py', '-v']
    require(inner.get('schema') == 'FerricM32HarnessQualificationV1'
            and inner.get('accepted') is True and type(inner.get('returncode')) is int
            and inner['returncode'] == 0 and inner.get('argv') == argv
            and inner.get('source_root') == str(CPU_SOURCE)
            and inner.get('source_archive_sha256') == phase['source_archive']['sha256']
            and inner.get('helper_sha256') == QUALIFIER_SHA
            and inner.get('source_before') == inner.get('source_after') == plan['sources']
            and type(inner.get('tests_expected')) is int and inner['tests_expected'] == len(tests)
            and inner.get('native_executed') is False and inner.get('native_qualified') is False,
            'exact successful harness source and test closure')
    before, after = inner.get('stage_before_bytes'), inner.get('stage_after_bytes')
    cap = CPU_LIMITS['stage_bytes'] - CPU_LIMITS['stage_reserve_bytes']
    require(type(before) is int and type(after) is int
            and inner.get('planning_increment_bytes') == 64 * 1024**2
            and 0 <= before <= cap - 64 * 1024**2 and 0 <= after <= cap
            and after - before <= 64 * 1024**2, 'charged fixed G40 qualification growth')
    require(read(phase['stdout']['path'], phase['stdout']['sha256'], 1024**2, empty=True)[0] == b'',
            'unittest stdout must be empty')
    stderr = read(phase['stderr']['path'], phase['stderr']['sha256'], 1024**2)[0].decode('utf-8')
    prefix = '\n'.join(tests) + '\n\n' + '-' * 70 + '\n'
    require(stderr.startswith(prefix) and re.fullmatch(r'Ran ' + str(len(tests)) +
            r' tests in [0-9]+\.[0-9]+s\n\nOK\n', stderr[len(prefix):]) is not None,
            'complete exact successful unittest roster and summary')


def validate_review(review, plan):
    require(type(review) is dict and set(review) == {'schema', 'engineering_only', 'worker_sha256',
            'source_hashes', 'cpu_phase', 'mode', 'runtime_evidence', 'image_evidence',
            'control_compiler_matches_candidate'}
            and review['schema'] == 'FerricM32ComponentReviewedInputsV1'
            and review['engineering_only'] is True
            and review['control_compiler_matches_candidate'] is False, 'explicit unmatched-compiler engineering review')
    require(review['mode'] == plan['mode'] == 'latency', 'uninstrumented component campaign')
    require(review['worker_sha256'] == plan['worker']['sha256']
            and review['source_hashes'] == plan['sources'], 'reviewed source/worker custody')
    validate_phase(review['cpu_phase'], plan)
    for name, field in (('runtime_binding.py', 'runtime_evidence'), ('m32_provenance.py', 'image_evidence')):
        validator = module(Path(plan['stage']) / name, plan['sources'][name])
        validator.validate_review(review[field], plan, read=read, staged_binding=staged_binding,
                                  decode=decode, require=require)


def validate_plan(plan, stage):
    require(NATIVE_ENABLED, 'M32 component is not qualified/enabled')
    require(type(plan) is dict and set(plan) == {'schema', 'stage', 'engineering_only',
            'device_unique_id', 'sources', 'files', 'worker', 'images', 'python', 'review', 'mode'}
            and plan['schema'] == 'FerricM32ComponentLaunchPlanV1'
            and plan['stage'] == str(stage_name(stage)) and plan['engineering_only'] is True
            and plan['device_unique_id'] == DEVICE, 'closed component launch plan')
    require(plan['mode'] in MODES and stage.name.startswith('ferric-prefill-m32-component-' + plan['mode'] + '-'),
            'mode-specific create-only native namespace')
    require(type(plan['sources']) is dict and set(plan['sources']) == set(PINS) | set(OWN_SOURCES),
            'closed executable source roster')
    require(all(plan['sources'][name] == value for name, value in PINS.items()), 'frozen helper lineage')
    verify_files(stage, plan['files'])
    for name, digest in plan['sources'].items():
        require(plan['files'][name]['sha256'] == sha(digest), 'source file not bound')
    require(set(plan['images']) == set(IMAGE_PINS), 'exact image pair')
    for name, item in [('worker-candidate', plan['worker']),
                       ('images/v5.hsaco', plan['images']['v5']),
                       ('images/m2.hsaco', plan['images']['m2'])]:
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


def load_local(stage, names):
    # Restore preexisting test/import bindings, including on partial-load failure.
    absent = object()
    saved = {key: sys.modules.get(key, absent) for key in names}
    loaded = {}
    try:
        for key, (name, expected) in names.items():
            path = stage / name
            raw, _ = read(path, expected, 1024**2)
            spec = importlib.util.spec_from_file_location(key, path)
            value = importlib.util.module_from_spec(spec)
            sys.modules[key] = value
            exec(compile(raw, str(path), 'exec'), value.__dict__)
            loaded[key] = value
        return loaded
    finally:
        for key, previous in saved.items():
            if previous is absent:
                sys.modules.pop(key, None)
            else:
                sys.modules[key] = previous


def load_component(stage):
    values = load_local(stage, {
        'fixtures': ('frozen/component_fixtures.py', PINS['frozen/component_fixtures.py']),
        'component': ('component.py', PINS['component.py']),
    })
    return values['component']


def load_ordered(stage, plan):
    values = load_local(stage, {Path(name).stem: (name, plan['sources'][name]) for name in
        ('fixtures.py', 'slice_views.py', 'm32_component.py', 'ordered_dispatch.py')})
    return values['m32_component'], values['ordered_dispatch']
