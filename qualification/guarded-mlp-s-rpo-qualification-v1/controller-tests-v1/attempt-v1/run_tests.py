"""One fixed controller-test leaf through the authenticated owned CPU supervisor."""

import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import sys
import time
import types


ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-s-rpo-controller-tests-v228-v1')
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 180, 120, 50
KNOWN = {
    'controller_test_entry.py': (5050, '0abd889b873f964d35a44dfdee3599cbafa942658394dd054a7dbebc16881a87'),
    'run_cpu.py': (53331, '083abf39c6a9d24ff379c238a7f41c56dfb71757a2678a08f65ca935fd2fc866'),
    'qualification_helpers.py': (4103, 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'),
    'test_run.py': (5500, 'a20d03a985724c5f2e9c34210b5b76143633a84ff5b6a3967c4f1ae19bda9e9b'),
    'supervisor.py': (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
}
EXPECTED = (
    'test_run.SourceContractTests.test_five_closed_cohorts_have_43_distinct_names',
    'test_run.SourceContractTests.test_fixture_context_derivation_preserves_actual_session_literal',
    'test_run.SourceContractTests.test_insertion_refuses_deletion',
    'test_run.SourceContractTests.test_insertion_refuses_empty_and_missing_destination',
    'test_run.SourceContractTests.test_insertion_refuses_missing_or_duplicate_context',
    'test_run.SourceContractTests.test_insertion_refuses_unknown_or_repeated_destination',
    'test_run.SourceContractTests.test_metadata_does_not_replace_embedded_text_or_prefix_sibling',
    'test_run.SourceContractTests.test_metadata_refuses_overlapping_or_duplicate_mapping',
    'test_run.SourceContractTests.test_metadata_refuses_wrong_source_or_external_destination',
    'test_run.SourceContractTests.test_metadata_relocates_paths_and_structured_path_ids',
    'test_run.SourceContractTests.test_two_insertion_hunks_preserve_all_other_bytes',
)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink, value.st_uid,
            value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def read_pin(path, retain=False, limit=1 << 30):
    path = Path(path)
    before = path.lstat()
    require(path.is_absolute() and path.resolve(strict=True) == path
            and stat.S_ISREG(before.st_mode) and before.st_size <= limit,
            'aliased, nonordinary or oversized input')
    digest, pieces, count = hashlib.sha256(), [], 0
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed before read')
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            count += len(chunk)
            require(count <= limit, 'input extent exceeded')
            digest.update(chunk)
            if retain:
                pieces.append(chunk)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during read')
    require(stamp(path.lstat()) == stamp(before) and count == before.st_size, 'input changed after read')
    return b''.join(pieces), dict(path=str(path), bytes=count, sha256=digest.hexdigest())


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_WALL
    require(__debug__ and sys.flags.isolated and sys.flags.no_site and sys.dont_write_bytecode
            and len(sys.argv) == 2 and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]),
            'python3 -I -S -B run_tests.py ACTUAL_INPUT_SHA required')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and os.getuid() == os.geteuid() == 9661 and ROOT.stat().st_uid == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'fresh host/UID/root mismatch')
    names = set(KNOWN) | {'run_tests.py'}
    require({path.name for path in ROOT.iterdir()} == names | {'input-manifest.json'},
            'fresh closed source-only namespace required')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, 'initial40GiB storage floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30),
                      (resource.RLIMIT_FSIZE, 1 << 30)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup failed')
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline-CLEANUP_RESERVE-time.monotonic()))
    input_body, input_pin = read_pin(ROOT / 'input-manifest.json', True, 64 << 10)
    require(input_pin['sha256'] == sys.argv[1], 'literal input pin mismatch')
    inputs = json.loads(input_body)
    require(set(inputs) == {'schema', 'files', 'tool_pins'}
            and inputs['schema'] == 'ferric-s-rpo-controller-test-input-v1'
            and type(inputs['files']) is dict and set(inputs['files']) == names
            and type(inputs['tool_pins']) is dict and set(inputs['tool_pins']) == {'python', 'prlimit'},
            'closed controller-test input fields required')
    bodies, before = {}, {}
    for name in sorted(names):
        bodies[name], before[name] = read_pin(ROOT / name, True, 1 << 20)
        row = inputs['files'][name]
        require(type(row) is dict and set(row) == {'bytes', 'sha256'}
                and type(row['bytes']) is int and row == {k: before[name][k] for k in row},
                'source pin differs: ' + name)
        if name in KNOWN:
            require((before[name]['bytes'], before[name]['sha256']) == KNOWN[name],
                    'frozen source generation differs: ' + name)
    tools = {}
    for role in ('python', 'prlimit'):
        row = inputs['tool_pins'][role]
        require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'}
                and type(row['path']) is str and type(row['bytes']) is int, 'tool pin fields')
        tools[role] = read_pin(Path(row['path']))[1]
        require(tools[role] == row, 'actual tool pin differs: ' + role)
    python = Path(tools['python']['path'])
    require(python == Path(sys.executable).resolve(strict=True)
            and tools['prlimit']['path'] == '/usr/bin/prlimit', 'actual Python/prlimit identity differs')
    module = types.ModuleType('p228_owned_controller_test_supervisor')
    module.__file__ = str(ROOT / 'supervisor.py')
    # Use only the authenticated supervisor bytes; its __main__ block is not entered.
    exec(compile(bodies['supervisor.py'], module.__file__, 'exec'), module.__dict__)
    h = module
    h.ROOT = ROOT
    h.OUT, h.TARGET, h.TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
    require(h.CLEANUP_RESERVE == CLEANUP_RESERVE and h.AS_LIMIT == 12 << 30
            and h.FILE_LIMIT == 1 << 30 and h.CPU_LIMIT == 1200
            and h.CACHE_LIMIT == 6 << 30 and h.STREAM_LIMIT == 64 << 20
            and h.LIVE_FREE == 38 << 30, 'frozen supervisor limit mismatch')
    h.OUT.mkdir(mode=0o700)
    phases, failure, postchecks, observation = [], None, [], None
    after = None
    try:
        h.TARGET.mkdir(mode=0o700)
        h.TMP.mkdir(mode=0o700)
        h.save('sources-before.json', before)
        env = dict(HOME='/home/harmenon', PATH=str(python.parent) + ':/usr/bin:/bin',
                   LANG='C', LC_ALL='C', TMPDIR=str(h.TMP),
                   ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        h.run('s-rpo-controller-tests', [str(python), '-I', '-S', '-B', str(ROOT/'controller_test_entry.py')],
              env, phases, hard_deadline, None, LEAF_WALL, ROOT)
        observation = json.loads((h.OUT/'s-rpo-controller-tests.stdout').read_bytes())
        require(set(observation) == {'schema', 'passed', 'names', 'passing_names', 'tests_run',
                    'failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes',
                    'source_unchanged', 'sources', 'synthetic_controller_only', 'compiler_qualification',
                    'gpu_execution'}, 'closed child observation fields required')
        require(observation['schema'] == 'ferric-s-rpo-controller-tests-v1'
                and observation['passed'] is True and observation['source_unchanged'] is True
                and observation['synthetic_controller_only'] is True
                and observation['compiler_qualification'] is False and observation['gpu_execution'] is False
                and observation['names'] == list(EXPECTED) and observation['passing_names'] == list(EXPECTED),
                'exact passing synthetic controller observation required')
        for key, expected in [('tests_run', 11), ('failures', 0), ('errors', 0), ('skipped', 0),
                              ('expected_failures', 0), ('unexpected_successes', 0)]:
            require(type(observation[key]) is int and observation[key] == expected, 'child counter differs: '+key)
        require(observation['sources'] == {name: inputs['files'][name] for name in
                                          ('run_cpu.py', 'qualification_helpers.py', 'test_run.py')},
                'child controller pins differ')
        require(len(phases) == 1, 'one fixed controller leaf required')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            remaining = hard_deadline - 5 - time.monotonic()
            require(remaining > 0, 'postcheck whole deadline')
            signal.setitimer(signal.ITIMER_REAL, remaining)
            action()
        except BaseException as error:
            postchecks.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal after
        after = {name: read_pin(ROOT/name, limit=1 << 20)[1] for name in sorted(names)}
        h.save('sources-after.json', after)
        require(after == before, 'controller-test source drift')
    check('sources', source_check)
    check('input', lambda: require(read_pin(ROOT/'input-manifest.json')[1] == input_pin, 'input drift'))
    check('tools', lambda: require({role: read_pin(Path(row['path']))[1] for role, row in tools.items()}
                                   == tools, 'Python/prlimit drift'))
    raw = {}
    def raw_check():
        for path in sorted(h.OUT.iterdir()):
            raw[path.name] = read_pin(path, limit=h.STREAM_LIMIT)[1]
    check('raw', raw_check)
    failure = failure or ('postcheck failed' if postchecks else None)
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-s-rpo-controller-test-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=postchecks, phases=phases,
                  child_observation=observation, input_manifest=input_pin,
                  source_inputs=before, source_unchanged=after==before, tool_pins=tools,
                  controller=before['run_tests.py'], supervisor=before['supervisor.py'], raw=raw,
                  elapsed_seconds=time.monotonic()-started,
                  limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, address_space_bytes=h.AS_LIMIT,
                              cpu_seconds_per_leaf=h.CPU_LIMIT, file_bytes=h.FILE_LIMIT,
                              scratch_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
                              initial_free_bytes=40 << 30, live_free_bytes=h.LIVE_FREE, affinity=[8, 9], nice=10),
                  synthetic_controller_only=True, compiler_qualification=False, gpu_execution=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline-time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
