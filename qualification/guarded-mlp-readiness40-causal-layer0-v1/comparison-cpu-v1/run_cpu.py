"""One bounded, GPU-hidden run of ten causal stage-mapping data tests."""
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
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-causal-stage-comparison-cpu-v228-v1')
OUT = ROOT / 'evidence'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
SOURCES = {
    'compare.py': (12434, '066a545b3a531ce8c2e0bb66b81f659b15c3e1e58f5fe0416199c22949bb0efe'),
    'diagnostics.py': (5084, '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf'),
    'observer.py': (10055, 'a96312f8d4c25f49b4585bc6f8ccca583f5097403c36608969709799a5b13fe9'),
    'test_compare.py': (11894, '9183e7849c31c42efeecad232639de243cff9397f0b0b0f76b43cd806d2aae3e'),
}
ROSTERS = {
    'test_compare': ('ComparisonTests', (
        'test_closed204_rows_and_no_acceptance_or_partial_synthesis',
        'test_closed_scope_extents_finiteness_and_cache_ancestry',
        'test_earliest_stage_groups_parallel_qkv_and_preserves_record_order',
        'test_framework_head_major_cache_permutation_and_current_row',
        'test_frozen_metrics_keep_zero_norm_signed_zero_and_exact_word_counts',
        'test_product_is_composite_and_requires_both_gate_and_up',
        'test_qkv_rank_slices_and_gate_up_halves_are_not_interleaved',
        'test_same_captured_inputs_are_distinct_from_propagation',
        'test_tp_composite_dependencies_include_both_ranks',
        'test_unavailable_rotary_and_partial_rows_never_become_full_projections',
    )),
}
NAMES = tuple(sorted(module + "." + cls + "." + name
    for module, (cls, names) in ROSTERS.items() for name in names))
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
    require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', *SOURCES}, 'closed six-file input')
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
        require((before[name]['bytes'], before[name]['sha256']) == (size, sha), 'exact causal comparison and reused pure helper source')
    for module, (class_name, names) in ROSTERS.items():
        tree = ast.parse((ROOT / (module + '.py')).read_bytes())
        classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
        require(len(classes) == 1 and classes[0].name == class_name
                and tuple(sorted(node.name for node in classes[0].body if isinstance(node, ast.FunctionDef)
                    and node.name.startswith('test_'))) == names, 'closed source-level test rosters')
    python = Path('/usr/bin/python3').resolve(strict=True)
    prlimit = Path('/usr/bin/prlimit').resolve(strict=True)
    tools = {'python': h.pin(python), 'prlimit': h.pin(prlimit)}
    environment = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        PYTHONPATH='', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    script = ('import sys,unittest;sys.path.insert(0,' + repr(str(ROOT)) + ');'
              'suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(name) '
              'for name in ("test_compare",));'
              'result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())')
    argv = [str(python), '-I', '-B', '-c', script]
    OUT.mkdir(mode=0o700); phases = []; errors = []; failure = None; census = None; after = None
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for number in handlers: signal.signal(number, interrupted)
    try:
        h.save('sources-before.json', before)
        h.run('readiness-tests', argv, environment, phases, deadline, before,
              seconds=LEAF_SECONDS, cwd=ROOT)
        stdout = (OUT / 'readiness-tests.stdout').read_bytes()
        stderr = (OUT / 'readiness-tests.stderr').read_text()
        require(stdout == b'', 'unexpected readiness test stdout')
        expression = r'^(test_[A-Za-z0-9_]+) \((test_compare\.ComparisonTests)\.\1\) \.\.\. ok$'
        found = [prefix + '.' + name for name, prefix in re.findall(expression, stderr, re.M)]
        require(tuple(sorted(found)) == NAMES and len(found) == len(set(found)) == 10,
                'exact ten named passing tests')
        tail = re.sub(expression, '', stderr, flags=re.M)
        tail = '\n'.join(line for line in tail.splitlines() if line)
        require(re.fullmatch(r'-{70}\nRan 10 tests in [0-9]+\.[0-9]+s\nOK', tail),
                'exact unittest summary, no skips/errors')
        census = dict(names=list(NAMES), passed=10, failed=0, errors=0, skipped=0)
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
            require(json.loads((OUT / 'readiness-tests.result.json').read_bytes()) == row, 'original phase result join')
            started = json.loads((OUT / 'readiness-tests.started.json').read_bytes())
            require(started == dict(pid=row['pid'], pgid=row['pgid'], argv=row['argv']), 'original child registration join')
        require(set(raw) <= {'sources-before.json', 'sources-after.json', 'readiness-tests.command.json',
            'readiness-tests.started.json', 'readiness-tests.result.json', 'readiness-tests.stdout', 'readiness-tests.stderr'},
            'closed original raw prefix')
    except BaseException as error:
        if time.monotonic() >= deadline:
            raise
        errors.append('raw reconciliation: ' + repr(error))
    failure = failure or ('postcheck failed' if errors else None)
    value = dict(schema='ferric-guarded-mlp-readiness40-causal-stage-comparison-cpu-v1', passed=failure is None and census is not None,
        failure=failure, postcheck_errors=errors, controller=before['run_cpu.py'], supervisor=before['supervisor.py'],
        sources_before=before, sources_after=after, source_unchanged=after == before,
        tool_pins=tools, environment=environment, phases=phases, tests=census, raw=raw,
        limits=dict(whole_seconds=WHOLE_SECONDS, test_seconds=LEAF_SECONDS, cleanup_reserve_seconds=CLEANUP_SECONDS,
            address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10),
        elapsed_seconds=time.monotonic() - start, synthetic_data_tests_only=True, actual_capture_comparison_performed=False,
        receipt_authentication=False, same_side_parity_verified_here=False,
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
