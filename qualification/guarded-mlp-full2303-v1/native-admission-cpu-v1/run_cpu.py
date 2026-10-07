"""One bounded GPU-hidden run of 25 Full2303 pure data admission tests."""
import ast
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-native-admission-cpu-v228-v1')
OUT = ROOT / 'evidence'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
SOURCES = {
    'common.py': (3172, '445c602dd0d237beb2ebef62446d1d2b34283367b68529736a890640ae185256'),
    'compare_full.py': (13208, 'f6ba7ed47d93a5b03562cf22c730ebebd64ddc01afd1910acc0c51f8b3c875b1'),
    'diagnostics.py': (5084, '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf'),
    'full_announcement.py': (3246, '66e97079ac1df7c56db159c86aede157907fdb817464311d96c3333c803a0621'),
    'full_reference.py': (11342, '195ea81ab55e0523ca9b6a41680c654f7e99d7fb949b7d032a8f897d3620e471'),
    'reference.py': (11388, '466028fe6061de915bd852328030e9ea81a0b0874e4305563ecc78f0073b1d7b'),
    'test_full.py': (28057, '5acd7134ec8c4216a85a92d8629eb364e12d5af26fa8baf620ba87d19e56923f'),
    'validate_full.py': (22139, 'b640f4500e97c2c9ce276998d1c276f9ebc4c305748f449cfdf45b36ad8bb0f7'),
}
ROSTERS = {
    'FullTests': (
        'test_all144_pages_and_last_page_boundaries',
        'test_all_original_records_order_count_and_canonical_bytes',
        'test_authentic_prompt_then_only_previous_own_output',
        'test_bank_retirement_generation_and_bool_refused',
        'test_both_stream_budgets_and_compact_retention_accounting',
        'test_close_exact256_and_retirement_flags',
        'test_complete2303_own256_and_four_captures',
        'test_control_guard_and_cross_forward_frontiers',
        'test_decoded_file_pin_skip_policy_and_raw_bytes',
        'test_fresh_full_marker_and_owned_natural_retirement',
        'test_last_output_is_not_consumed_and_no_forward2304',
        'test_readiness_ar4_and_teacher_forced_profiles_refused',
        'test_selected_capture_roster_paths_hashes_and_extent',
        'test_selected_payload_finite_lowest_tie_and_zero',
        'test_stderr_authority_false_flags_and_strict_json',
        'test_unselected_original_hash_and_chain_changes_refused',
    ),
    'BehaviorTests': (
        'test_comparison_never_accepts_short_bool_or_fitted_token_vectors',
        'test_decoded_byte_mismatch_fails_without_normalization',
        'test_exact256_ids_and_raw_decoded_bytes_pass_only',
        'test_full_admission_joins_owned_stdout_command_and_retirement_before_mismatch',
        'test_generated_mismatch_fails_including_ties_first_and_last',
        'test_native_admission_original_pin_is_required_before_reference',
        'test_pinned_reference_manifest_refuses_replacement_before_projection',
        'test_reference_history_repeat_payload_and_output_drift',
        'test_reference_two_actual_own_histories_and_matrix_capture',
    ),
}
NAMES = tuple(sorted('test_full.' + cls + '.' + name
    for cls, names in ROSTERS.items() for name in names))
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
WHOLE_SECONDS, LEAF_SECONDS, CLEANUP_SECONDS = 180, 120, 50


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def load_supervisor():
    path = ROOT / 'supervisor.py'; before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 1 << 20, 'ordinary bounded supervisor')
    with path.open('rb') as stream:
        raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size
            and hashlib.sha256(raw).hexdigest() == SUPERVISOR_SHA, 'frozen supervisor bytes')
    module = types.ModuleType('readiness40_checker_owned'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def interrupted(number, _frame):
    raise RuntimeError('controller signal ' + str(number))


def main():
    start = time.monotonic(); deadline = start + WHOLE_SECONDS
    def arm_deadline():
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'whole CPU deadline exhausted; retain failed prefix only')
        signal.setitimer(signal.ITIMER_REAL, remaining)
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B run_cpu.py only')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT and not os.path.lexists(OUT),
            'fresh exact output namespace')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged CPU host')
    require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', *SOURCES}, 'closed ten-file input')
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice level')
    if priority == 0: os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = load_supervisor()
    h.OUT = OUT; h.AS_LIMIT = 512 << 20; h.FILE_LIMIT = 16 << 20
    h.STREAM_LIMIT = 4 << 20; h.CPU_LIMIT = 120; h.CACHE_LIMIT = 4 << 20
    h.CLEANUP_RESERVE = CLEANUP_SECONDS
    require(h.shutil.disk_usage(ROOT).free >= h.START_FREE, 'initial40GiB free floor')
    paths = [ROOT / name for name in sorted({'run_cpu.py', 'supervisor.py', *SOURCES})]
    def sources():
        require(all(p.resolve(strict=True) == p for p in paths), 'canonical CPU inputs')
        return {p.name: h.pin(p) for p in paths}
    h.sources = sources
    before = sources()
    for name, (size, sha) in SOURCES.items():
        require((before[name]['bytes'], before[name]['sha256']) == (size, sha), 'exact Full2303 pure source/test bytes')
    tree = ast.parse((ROOT / 'test_full.py').read_bytes())
    classes = {node.name: tuple(sorted(method.name for method in node.body
        if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')))
        for node in tree.body if isinstance(node, ast.ClassDef)}
    require(classes == ROSTERS, 'closed two-class source-level test roster')
    python = Path('/usr/bin/python3').resolve(strict=True)
    prlimit = Path('/usr/bin/prlimit').resolve(strict=True)
    tools = {'python': h.pin(python), 'prlimit': h.pin(prlimit)}
    environment = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        PYTHONPATH='', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    script = ('import sys,unittest;sys.path.insert(0,' + repr(str(ROOT)) + ');'
              'suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(name) '
              'for name in ("test_full",));'
              'result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())')
    argv = [str(python), '-I', '-B', '-c', script]
    OUT.mkdir(mode=0o700); phases = []; errors = []; failure = None; census = None; after = None
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for number in handlers: signal.signal(number, interrupted)
    try:
        h.save('sources-before.json', before)
        h.run('admission-tests', argv, environment, phases, deadline, before,
              seconds=LEAF_SECONDS, cwd=ROOT)
        stdout = (OUT / 'admission-tests.stdout').read_bytes()
        stderr = (OUT / 'admission-tests.stderr').read_text()
        require(stdout == b'', 'unexpected admission test stdout')
        expression = r'^(test_[A-Za-z0-9_]+) \((test_full\.FullTests|test_full\.BehaviorTests)\.\1\) \.\.\. ok$'
        found = [prefix + '.' + name for name, prefix in re.findall(expression, stderr, re.M)]
        require(tuple(sorted(found)) == NAMES and len(found) == len(set(found)) == 25,
                'exact twenty-five named passing tests')
        tail = re.sub(expression, '', stderr, flags=re.M)
        tail = '\n'.join(line for line in tail.splitlines() if line)
        require(re.fullmatch(r'-{70}\nRan 25 tests in [0-9]+\.[0-9]+s\nOK', tail),
                'exact unittest summary, no skips/errors')
        census = dict(names=list(NAMES), passed=25, failed=0, errors=0, skipped=0)
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    finally:
        for number in handlers:
            signal.signal(number, interrupted if number == signal.SIGALRM else signal.SIG_IGN)
        arm_deadline()
        try:
            after = sources(); require(after == before, 'CPU source drift')
            h.save('sources-after.json', after)
            require(Path('/usr/bin/python3').resolve(strict=True) == python
                    and Path('/usr/bin/prlimit').resolve(strict=True) == prlimit
                    and h.pin(python) == tools['python'] and h.pin(prlimit) == tools['prlimit'], 'CPU tool drift')
            require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', 'evidence', *SOURCES},
                    'unexpected bytecode/cache/input-root write')
            require(len(phases) == 1 and all(row['reaped'] and row['process_group_absent'] for row in phases),
                    'sole child completely reaped')
            require(time.monotonic() < deadline, 'whole CPU bound')
        except BaseException as error:
            if time.monotonic() >= deadline:
                raise
            errors.append(type(error).__name__ + ': ' + str(error))
    arm_deadline()
    raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    try:
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'final raw phase pin join')
            require(json.loads((OUT / 'admission-tests.result.json').read_bytes()) == row, 'original phase result join')
            started = json.loads((OUT / 'admission-tests.started.json').read_bytes())
            require(started == dict(pid=row['pid'], pgid=row['pgid'], argv=row['argv']), 'original child registration join')
        require(set(raw) <= {'sources-before.json', 'sources-after.json', 'admission-tests.command.json',
            'admission-tests.started.json', 'admission-tests.result.json', 'admission-tests.stdout', 'admission-tests.stderr'},
            'closed original raw prefix')
    except BaseException as error:
        if time.monotonic() >= deadline:
            raise
        errors.append('raw reconciliation: ' + repr(error))
    failure = failure or ('postcheck failed' if errors else None)
    value = dict(schema='ferric-guarded-mlp-full2303-native-admission-cpu-v1', passed=failure is None and census is not None,
        failure=failure, postcheck_errors=errors, controller=before['run_cpu.py'], supervisor=before['supervisor.py'],
        sources_before=before, sources_after=after, source_unchanged=after == before,
        tool_pins=tools, environment=environment, phases=phases, tests=census, raw=raw,
        limits=dict(whole_seconds=WHOLE_SECONDS, test_seconds=LEAF_SECONDS, cleanup_reserve_seconds=CLEANUP_SECONDS,
            address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10),
        elapsed_seconds=time.monotonic() - start, synthetic_data_tests_only=True,
        gpu_execution=False, native_parent_execution=False, model_execution=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False, production_authority=False)
    arm_deadline()
    h.save('complete.json' if value['passed'] else 'failed.json', value)
    print(json.dumps({key: value[key] for key in ('passed', 'failure', 'postcheck_errors', 'tests')}, sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items(): signal.signal(number, handler)
    return 0 if value['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
