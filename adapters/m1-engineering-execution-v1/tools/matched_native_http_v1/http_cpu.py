"""Closed remote CPU qualification for this private HTTP harness, not a GPU claim."""
import hashlib
import json
from pathlib import Path
import re

PROFILE = 'FerricCpuFourCore36GiBEmitterV1'
GUARD_SHA = 'fba93769c349a1bca0ea72ff2dbc0ed9b077be712dc8f370775d050df96af796'
ENV_SHA = 'a4f373e86b56bfc692b56d726280fc1f7efbfa521db96e936dd92787d7b2c7b8'
PROFILES = {
    'FerricCpuFourCore56GiBEmitterV1': (
        '7011bfe58dd5d602b421342c22ce23bdc5a1484e1d7d2da3375f9382e52f1902',
        '716d6ed7fb4796a3a80825fcdc403609ac089157a00eb7f49da67e0c5d960384',
        60129542144, 'cpu-env-56g-emitter.sh'),
    'FerricCpuFourCore44GiBEmitterV1': (
        '9018aa3b902ec4341f478ed25466fc5b9becc1d2a7ed72f3f831bf8810e12bb0',
        '13faafaf05ba850fdcb11bc4a79950d7d92523cb45db1756a1c5f751a2e1cc7a',
        47244640256, 'cpu-env-44g-emitter.sh'),
    'FerricCpuFourCore42GiBEmitterV1': (
        '4cff431e7d81a014b89cd391c5c89b40260b35d8d8a13fb667d11db2f560a55c',
        'cc797b77ac431b613658d136df2bba173cffb7ffd786634c6701767456264c9c',
        45097156608, 'cpu-env-42g-emitter.sh'),
    PROFILE: (GUARD_SHA, ENV_SHA, 38654705664, 'cpu-env-36g-emitter.sh'),
    'FerricCpuFourCore32GiBEmitterV1': (
        'a98cd103286f37bd90021f0e8fb0bd754b92281039b7e7bafeb88b64aec7f656',
        '66619d25c2cf529d5bb19c970c08d3804456ff960d441680eefaa016c30552ce',
        34359738368, 'cpu-env-32g-emitter.sh'),
    'FerricCpuFourCore30GiBEmitterV1': (
        '609e7e3f65904f57ece0ce82cdcf31af1ad641d73d5547b6cc6c85900cacb363',
        'b17576065fea6c72ca6306ff5667bcf75e108f4749b6856a2a831818e252e3f2',
        32212254720, 'cpu-env-30g-emitter.sh'),
}
LIMITS = {'duration_seconds': 1200, 'individual_file_bytes': 536870912, 'kill_wait_seconds': 3,
    'log_bytes': 33554432, 'memory_available_bytes': 137438953472, 'observed_rss_bytes': 8589934592,
    'root_free_bytes': 23622320128, 'shm_free_bytes': 34359738368, 'stage_bytes': 38654705664,
    'stage_reserve_bytes': 536870912, 'term_grace_seconds': 10}
TEST_FILES = ('test_http_lifecycle.py', 'test_selected_native.py', 'test_container_custody.py',
    'test_gpu_probe.py', 'test_owned_command.py', 'test_gpu_attribution.py',
    'test_http_runner.py', 'test_probe_cli.py', 'test_http_cpu.py',
    'test_gpu_activity.py', 'test_startup_scanner.py', 'test_startup_reconciliation.py',
    'test_startup_departure.py', 'test_native_fd_rescan.py', 'test_width_selection.py',
    'test_gate_up_selection.py', 'test_down_selection.py', 'test_current_down_selection.py')
EXPECTED_TESTS = 316


def require(ok, message):
    if not ok:
        raise ValueError(message)


def validate(value, sources, *, bound, raw, digest, root):
    require(type(value) is dict and set(value) == {'schema', 'driver_sources', 'test_sources',
        'guard', 'environment', 'phase', 'source_root', 'servers_launched'}
        and value['schema'] == 'FerricNativeHttpCpuQualificationV1'
        and value['servers_launched'] is False, 'closed CPU-only qualification')
    require(value['driver_sources'] == sources and set(value['test_sources']) == set(TEST_FILES),
            'complete exact source qualification')
    for name, expected in value['test_sources'].items():
        require(digest(root / name) == expected, 'qualified fixture bytes changed')
    phase = value['phase']
    require(type(phase) is dict and set(phase) == {'status', 'result', 'stderr', 'argv_sha256'},
            'closed raw CPU phase fields')
    require(raw(phase['status'], 16) == b'0\n', 'CPU phase must succeed')
    result = bound(phase['result'])
    profile = result.get('profile')
    require(profile in PROFILES, 'unrecognized CPU profile')
    guard_sha, env_sha, stage_bytes, wrapper = PROFILES[profile]
    limits = {**LIMITS, 'stage_bytes': stage_bytes}
    for key, expected in (('guard', guard_sha), ('environment', env_sha)):
        require(value[key]['sha256'] == expected and digest(value[key]['path']) == expected,
                'actual qualified CPU guard/environment changed')
    wanted = {'profile': profile, 'status': 0, 'reason': 'completed', 'returncode': 0,
        'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False,
        'kill_sent': False, 'log_limit_exceeded': False, 'build_jobs': 4,
        'rust_test_threads': 1, 'nice': 19, 'cpus': [0, 1, 2, 3], 'limits': limits}
    require(all(type(result.get(key)) is type(item) and result[key] == item for key, item in wanted.items()),
            'clean exact-profile CPU result required')
    environment = result.get('launch_environment', {})
    require(all(environment.get(key) == item for key, item in {
        'CARGO_BUILD_JOBS': '4', 'RUST_TEST_THREADS': '1', 'FERRIC_CPU_PROFILE': profile,
        'CUDA_VISIBLE_DEVICES': '-1', 'HIP_VISIBLE_DEVICES': '-1', 'HSA_VISIBLE_DEVICES': '-1',
        'ROCR_VISIBLE_DEVICES': '-1'}.items()), 'CPU qualification must mask all GPU visibility')
    require(type(result.get('peak_observed_rss_bytes')) is int
            and 0 <= result['peak_observed_rss_bytes'] <= limits['observed_rss_bytes'], 'observed RSS bound')
    for key in ('admission', 'final_resources'):
        resources = result.get(key, {})
        require(all(type(resources.get(name)) is int and resources[name] >= limits[name]
                for name in ('memory_available_bytes', 'root_free_bytes', 'shm_free_bytes'))
                and type(resources.get('stage_bytes')) is int
                and 0 <= resources['stage_bytes'] <= limits['stage_bytes'] - limits['stage_reserve_bytes'],
                'CPU host/stage floors required')
    argv = result.get('argv')
    require(type(argv) is list and len(argv) >= 2 and all(type(word) is str and word for word in argv),
            'literal retained CPU command required')
    source_root = value['source_root']
    require(type(source_root) is str and Path(source_root).is_absolute(), 'original tested source root required')
    expected = ['/bin/bash', result['argv'][1], '/usr/bin/python3', '-I', '-B', '-m',
                'unittest', 'discover', '-s', source_root, '-p', 'test_*.py', '-v']
    require(argv == expected and Path(argv[1]).name == wrapper,
            'exact unfiltered remote unittest discovery command required')
    canonical = json.dumps(argv, sort_keys=True, separators=(',', ':'), allow_nan=False).encode() + b'\n'
    require(hashlib.sha256(canonical).hexdigest() == phase['argv_sha256'], 'actual CPU argv binding')
    output = raw(phase['stderr'], LIMITS['log_bytes'])
    require(re.findall(rb'^Ran (\d+) tests in [0-9.]+s$', output, re.MULTILINE) == [str(EXPECTED_TESTS).encode()]
            and re.findall(rb'^OK$', output, re.MULTILINE) == [b'OK']
            and b'FAILED' not in output and b' skipped' not in output, 'all exact fixtures must execute and pass')
    return {'schema': 'FerricNativeHttpCpuAdmissionV1', 'accepted': True,
            'tests': EXPECTED_TESTS, 'servers_launched': False, 'gpu_validation_claimed': False}
