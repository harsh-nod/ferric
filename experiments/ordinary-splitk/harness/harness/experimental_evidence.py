"""Closed historical runtime reuse and exact failed-lint experimental custody."""
import hashlib
import re

D = '/tmp/ferric-v16-emitter-b95a642-r1'
G32 = 'FerricCpuFourCore32GiBEmitterV1'
G36 = 'FerricCpuFourCore36GiBEmitterV1'
RUNTIME = '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
WORKER = 'af246e5b872b639b90906336493629eb2bbfa90ff57ffc4ffd859f8b963d9b30'
RUNTIME_SOURCE = 'a9772e55f9a140d0749a621c07ea2c9cb19c36b2416656278dd8d2a2095321d3'
CLIENT_SOURCE = '302b74237a3909d750913b4c08f779295b377a438371cb96c50e38829fe7bc6a'
CLIENT_ROSTER = 'd67bc45babb01294b24feee1468e1ff0f2691583d6adb0658542e1d88e133f0f'
CONTROLLER = 'a8ab66586d6fde3ff15642796f0614378f9f3a8c8b1dbb4b8bf02c1ed7157a30'
RUNTIME_BUILD = 'dd045f41453ac3b98cf3eaf25116658082b4c6583f4a896b902028b884c77af2'
RUNTIME_CPU = 'd236b4db3e370bccdd0745a4b6f133f14b05f73d20e7289d3000e1a5c781ac53'
RETENTION = '38db36507f6b5e3e8cf62ec9f25f61cb29d666d1cfa5a80935d9ff5295f9ebd8'
RETENTION_RESULT = '303b2a5c055cf4795ddb6b677c030bb0cd48667d3ed14aadd1e0d9a94ebbeddd'
RETENTION_HELPER = '626736be5a857d772276ae0c562a0d7e68f7c514154bb910f030c10e33bd89de'
DEFERRED = ['regression-token', 'default', 'fallback', 'union', 'union-cli', 'source-policy', 'clippy-union']
PASSED = {'focused': 15, 'cli': 5, 'regression-v19': 8, 'regression-packed': 17,
          'regression-attention': 17, 'regression-prefill16': 9, 'regression-prefill32': 14,
          'regression-ordered': 5, 'actual-image': 1, 'release': None}
RUNTIME_ROLES = {'runtime-tests', 'worker-tests', 'runtime-build', 'aql-tests'}
HARNESS_ROLES = {'launch-tests', 'measurement-tests'}
PHASES = RUNTIME_ROLES | set(PASSED) | {'failed-clippy', 'experimental-retention'} | HARNESS_ROLES
LIMITS = {'duration_seconds': 1200, 'individual_file_bytes': 536870912, 'kill_wait_seconds': 3,
    'log_bytes': 33554432, 'memory_available_bytes': 137438953472, 'observed_rss_bytes': 8589934592,
    'root_free_bytes': 23622320128, 'shm_free_bytes': 34359738368,
    'stage_reserve_bytes': 536870912, 'term_grace_seconds': 10}


def fields(role):
    if role in HARNESS_ROLES:
        return ('status', 'result', 'stdout', 'stderr', 'inner', 'helper')
    if role == 'runtime-build':
        return ('status', 'result')
    if role in ('runtime-tests', 'worker-tests'):
        return ('status', 'result', 'stdout')
    if role == 'aql-tests':
        return ('status', 'result', 'stdout', 'inner', 'helper')
    if role in PASSED:
        return ('status', 'result', 'stdout', 'stderr', 'inner')
    return ('status', 'result', 'stdout', 'stderr')


def expected_profile(role):
    return G32 if role in RUNTIME_ROLES - {'aql-tests'} else G36


def exact(c, value, wanted, label):
    c.require(type(value) is dict and all(type(value.get(key)) is type(item) and value[key] == item
              for key, item in wanted.items()), label)


def guarded(c, role, phase, read_bound, read_raw):
    profile = expected_profile(role)
    c.require(phase['profile'] == profile, 'role-specific unchanged CPU profile')
    failure = role == 'failed-clippy'
    c.require(read_raw(phase['status']['path'], phase['status']['sha256'], 16)[0]
              == (b'1\n' if failure else b'0\n'), 'actual raw CPU exit differs')
    result = read_bound(phase['result'])
    exact(c, result, {'status': 1 if failure else 0, 'returncode': 1 if failure else 0,
        'reason': 'completed', 'cleanup_ok': True, 'child_reaped': True, 'errors': [],
        'term_sent': False, 'kill_sent': False, 'log_limit_exceeded': False, 'profile': profile,
        'cpus': [0, 1, 2, 3], 'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1},
        'unclean or misattributed CPU role: ' + role)
    limits = {**LIMITS, 'stage_bytes': (32 if profile == G32 else 36) * 1024**3}
    c.require(result.get('limits') == limits and all(type(v) is int for v in result['limits'].values()),
              'CPU cap/floors/reserve changed')
    exact(c, result.get('launch_environment'), {'CARGO_BUILD_JOBS': '4', 'RUST_TEST_THREADS': '1',
        'FERRIC_CPU_PROFILE': profile, 'CUDA_VISIBLE_DEVICES': '-1', 'HIP_VISIBLE_DEVICES': '-1',
        'HSA_VISIBLE_DEVICES': '-1', 'ROCR_VISIBLE_DEVICES': '-1'}, 'CPU visibility masking changed')
    c.require(type(result.get('peak_observed_rss_bytes')) is int
              and 0 <= result['peak_observed_rss_bytes'] <= limits['observed_rss_bytes'], 'CPU RSS bound')
    for key in ('admission', 'final_resources'):
        value = result.get(key, {})
        c.require(all(type(value.get(name)) is int and value[name] >= limits[name]
                      for name in ('memory_available_bytes', 'root_free_bytes', 'shm_free_bytes'))
                  and type(value.get('stage_bytes')) is int
                  and 0 <= value['stage_bytes'] <= limits['stage_bytes'] - limits['stage_reserve_bytes'],
                  'actual CPU resource endpoint failed')
    argv = result.get('argv')
    c.require(type(argv) is list and argv and all(type(x) is str and x for x in argv)
              and hashlib.sha256(c.encoded(argv)).hexdigest() == phase['argv_sha256'],
              'actual wrapped command binding differs')
    raw = {name: read_raw(phase[name]['path'], phase[name]['sha256'], limits['log_bytes'], empty=True)[0]
           for name in ('stdout', 'stderr') if name in phase}
    return result, raw


def validate(c, cpu, build, read_bound, read_raw):
    c.require(type(cpu) is dict and set(cpu) == {'schema', 'runtime_evidence', 'experimental_retention',
              'phases', 'harness_sources'} and cpu['schema'] == 'FerricSplitKExperimentalCpuV1',
              'closed experimental CPU evidence')
    borrowed = cpu['runtime_evidence']
    c.require(type(borrowed) is dict and set(borrowed) == {'build', 'cpu'}
              and borrowed['build']['sha256'] == RUNTIME_BUILD and borrowed['cpu']['sha256'] == RUNTIME_CPU,
              'only the exact accepted width runtime evidence can be reused')
    old_build, old_cpu = read_bound(borrowed['build']), read_bound(borrowed['cpu'])
    exact(c, old_build, {'runtime_main': RUNTIME}, 'borrowed published runtime identity')
    c.require(old_build['cpu_qualification']['sha256'] == RUNTIME_CPU
              and old_build['worker']['sha256'] == build['worker']['sha256'] == WORKER
              and old_build['runtime_source']['sha256'] == build['runtime_source']['sha256'] == RUNTIME_SOURCE,
              'borrowed runtime/worker/source bytes changed; width controller evidence is not reusable')
    c.require(cpu['experimental_retention'] == build['experimental_retention']
              and cpu['experimental_retention']['sha256'] == RETENTION, 'exact experimental custody root')
    retained = read_bound(cpu['experimental_retention'])
    exact(c, retained, {'schema': 'FerricSplitKModelExperimentalRetentionV1', 'actual_test_passes': 91,
        'source_files': 1440, 'source_roster_sha256': CLIENT_ROSTER, 'source_verified_before': True,
        'source_verified_after': True, 'runtime_revision': RUNTIME, 'profile': G36,
        'features': ['c1-ordered64'], 'full_matrix_qualified': False, 'production_qualified': False,
        'default_promotion': False, 'native_executed': False, 'latency_sample_admitted': False,
        'deferred_roles': DEFERRED, 'helper_sha256': RETENTION_HELPER}, 'experimental retention scope changed')
    c.require(retained['controller']['sha256'] == build['controller']['sha256'] == CONTROLLER
              and retained['controller']['size_bytes'] == 10455688
              and retained['source_archive']['sha256'] == build['controller_source']['sha256'] == CLIENT_SOURCE
              and set(retained['passed_roles']) == set(PASSED), 'exact released source/ELF and passed-role roster')
    exact(c, retained['clippy'], {'status': 'failed', 'binary_lint_coverage': 'incomplete',
        'findings': ['doc_markdown', 'format_push_string']}, 'failed Clippy must not become a pass')
    c.require(type(cpu['phases']) is dict and set(cpu['phases']) == PHASES, 'complete closed CPU evidence roster')
    expected_sources = set(c.OWN_SOURCES) | {'measurement/' + name for name in c.MEASUREMENT_SOURCES}
    c.require(type(cpu['harness_sources']) is dict and set(cpu['harness_sources']) == expected_sources,
              'exact tested harness sources')
    for digest in cpu['harness_sources'].values():
        c.sha(digest)
    for role, phase in cpu['phases'].items():
        c.require(type(phase) is dict and set(phase) == set(fields(role)) | {'profile', 'argv_sha256'},
                  'closed CPU role evidence: ' + role)
        for name in fields(role):
            c.binding(phase[name])
        result, raw = guarded(c, role, phase, read_bound, read_raw)
        if role in RUNTIME_ROLES:
            original = old_cpu['phases'][role]
            for name in set(fields(role)) & set(original):
                c.require(phase[name]['sha256'] == original[name]['sha256'], 'borrowed raw runtime receipt differs')
            c.require(result['argv'] and phase['argv_sha256'] == original['argv_sha256'],
                      'borrowed runtime command differs')
            if role == 'aql-tests':
                read_raw(phase['helper']['path'], phase['helper']['sha256'], 1024**2)
                inner = read_bound(phase['inner'])
                c.require(c.decode(raw['stdout'].splitlines()[-1]) == inner,
                          'fresh AQL outer stdout/inner binding')
        elif role in PASSED:
            expected = retained['passed_roles'][role]
            for name in fields(role):
                c.require(phase[name]['sha256'] == expected[name]['sha256'], 'exact retained split-K role differs')
            inner = read_bound(phase['inner'])
            c.require(c.decode(raw['stdout'].splitlines()[-1]) == inner,
                      'outer output does not bind exact successful inner receipt')
            exact(c, inner, {'role': role, 'returncode': 0, 'source_files': 1440,
                'source_roster_sha256': CLIENT_ROSTER, 'source_verified_after': True,
                'runtime_revision': RUNTIME, 'native_executed': False, 'profile': G36}, 'client source/role drift')
            if PASSED[role] is not None:
                counts = re.findall(rb'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;',
                                    raw['stdout'], re.MULTILINE)
                wanted = [(PASSED[role], 0, 1 if role == 'focused' else 0, 0)]
                c.require([tuple(int(x) for x in row) for row in counts] == wanted,
                          'exact actual split-K test cardinality')
        elif role == 'failed-clippy':
            names = {'status': 'exit.status', 'result': 'result.json', 'stdout': 'stdout', 'stderr': 'stderr'}
            c.require(all(phase[key]['sha256'] == retained['clippy']['evidence'][name]['sha256']
                          for key, name in names.items()), 'failed Clippy raw custody changed')
        elif role == 'experimental-retention':
            c.require(phase['result']['sha256'] == RETENTION_RESULT
                      and c.decode(raw['stdout']) == retained, 'retention outer/inner chain differs')
        else:
            c.require(type(c.HARNESS_TEST_COUNTS) is dict and role in c.HARNESS_TEST_COUNTS
                      and type(c.HARNESS_HELPER_SHA) is str,
                      'new harness CPU qualification is pending')
            helper = D + '/inputs/splitk-model-harness-a003/qualify.py'
            c.require(phase['helper']['sha256'] == c.HARNESS_HELPER_SHA
                      and result['argv'] == ['/bin/bash', D + '/owner/cpu-env-36g-emitter.sh',
                          '/usr/bin/python3', '-I', '-B', helper, role], 'exact new harness test invocation')
            read_raw(phase['helper']['path'], phase['helper']['sha256'], 1024**2)
            inner = read_bound(phase['inner'])
            c.require(c.decode(raw['stdout']) == inner, 'actual harness outer stdout/inner custody')
            root = D + '/splitk-model-harness-a003/harness'
            command = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover', '-s',
                       root if role == 'launch-tests' else root + '/measurement', '-p', 'test_*.py', '-v']
            exact(c, inner, {'schema': 'FerricSplitKHarnessCpuV1', 'role': role, 'profile': G36,
                'command': command, 'returncode': 0, 'helper_sha256': c.HARNESS_HELPER_SHA,
                'source_before': cpu['harness_sources'], 'source_after': cpu['harness_sources'],
                'tests': c.HARNESS_TEST_COUNTS[role], 'native_executed': False, 'accepted': True,
                'error': None}, 'tested harness source/role drift')
            stderr = inner.get('test_stderr', '')
            c.require(type(stderr) is str and re.search(r'\nRan ' + str(c.HARNESS_TEST_COUNTS[role])
                + r' tests? in [0-9.]+s\n\nOK\n\Z', stderr) is not None
                and inner.get('test_stdout') == '', 'actual full unittest footer, no failures or skips')
    return {'schema': 'FerricSplitKExperimentalScopeV1', 'runtime_only_reuse': True,
            'width_controller_passes_credited': 0, 'controller_test_passes': 91,
            'strict_clippy': 'failed', 'binary_lint_coverage': 'incomplete', 'deferred_roles': DEFERRED,
            'production_qualified': False, 'default_promotion': False}
