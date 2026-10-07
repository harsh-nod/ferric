"""Owned ten-test gate and actual pinned numerical report; no GPU execution."""
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
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-numerical-report-cpu-v228-v1')
OUT = ROOT / 'evidence'
REPORT = ROOT / 'report'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
SOURCES = {
    'render.py': (18398, '8b8129b76199d1e903e5fc434f604ce8a942a25822863d800f36e00fa6d6aab3'),
    'test_render.py': (10389, '0e80570701615a796cf5332b4edc37eb76c64bddedef07853de1500256078873'),
    'comparison.json': (94179, '50dcd3b01a95164186e517005719de46bc321396089762cf40e9013be14c0494'),
}
NAMES = tuple(sorted((
    'test_all40_argmax_and_uncaptured_mismatch_survive',
    'test_csv_preserves_all152_metrics_in_numeric_layer_order',
    'test_exact_input_pin_duplicate_keys_and_nonfinite_json_refused',
    'test_reader_refuses_symlink_and_oversized_file',
    'test_rejects_argmax_scope_history_and_mismatch_loss',
    'test_rejects_authority_identity_and_extra_fields',
    'test_rejects_metric_shape_bool_negative_and_nonfinite',
    'test_rejects_missing_reordered_or_duplicated_capture_roles',
    'test_svg_four_distinct_series_stable_geometry_and_percent_axis',
    'test_zero_curve_has_finite_nonclipped_axis',
)))
REPORT_FILES = {'tensor-metrics.csv', 'argmax-diagnostics.csv', 'summary.md', 'hidden-relative-l2.svg'}
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
WHOLE_SECONDS, LEAF_SECONDS, CLEANUP_SECONDS = 240, 120, 50


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
    module = types.ModuleType('numerical_report_owned'); module.__file__ = str(path)
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
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT and not os.path.lexists(OUT) and not os.path.lexists(REPORT),
            'fresh exact output namespace')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged CPU host')
    require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', *SOURCES}, 'closed five-file input')
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
    h.OUT = OUT; h.TARGET = REPORT; h.TMP = ROOT / 'tmp'; h.AS_LIMIT = 512 << 20; h.FILE_LIMIT = 16 << 20
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
        require((before[name]['bytes'], before[name]['sha256']) == (size, sha), 'exact renderer and actual comparison inputs')
    tree = ast.parse((ROOT / 'test_render.py').read_bytes())
    classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
    require(len(classes) == 1 and classes[0].name == 'ReportTests'
            and tuple(sorted(node.name for node in classes[0].body if isinstance(node, ast.FunctionDef)
                and node.name.startswith('test_'))) == NAMES, 'source-level ten-test roster')
    python = Path('/usr/bin/python3').resolve(strict=True)
    prlimit = Path('/usr/bin/prlimit').resolve(strict=True)
    tools = {'python': h.pin(python), 'prlimit': h.pin(prlimit)}
    environment = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        PYTHONPATH='', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    script = ('import sys,unittest;sys.path.insert(0,' + repr(str(ROOT)) + ');'
              'unittest.main(module="test_render",argv=["renderer-tests"],verbosity=2)')
    argv = [str(python), '-I', '-B', '-c', script]
    OUT.mkdir(mode=0o700); phases = []; errors = []; failure = None; census = None; after = None
    report = None; output_pins = {}
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for number in handlers: signal.signal(number, interrupted)
    try:
        h.save('sources-before.json', before)
        arm_deadline()
        h.run('renderer-tests', argv, environment, phases, deadline, before,
              seconds=LEAF_SECONDS, cwd=ROOT)
        arm_deadline()
        stdout = (OUT / 'renderer-tests.stdout').read_bytes()
        stderr = (OUT / 'renderer-tests.stderr').read_text()
        require(stdout == b'', 'unexpected synthetic test stdout')
        expression = r'^(test_[A-Za-z0-9_]+) \(test_render\.ReportTests\.\1\) \.\.\. ok$'
        found = re.findall(expression, stderr, re.M)
        require(tuple(sorted(found)) == NAMES and len(found) == len(set(found)) == 10, 'exact ten named passing tests')
        tail = re.sub(expression, '', stderr, flags=re.M)
        tail = '\n'.join(line for line in tail.splitlines() if line)
        require(re.fullmatch(r'-{70}\nRan 10 tests in [0-9]+\.[0-9]+s\nOK', tail), 'exact unittest summary, no skips/errors')
        census = dict(names=list(NAMES), passed=10, failed=0, errors=0, skipped=0)
        h.run('render', [str(python), '-I', '-B', str(ROOT / 'render.py'), '--input',
              str(ROOT / 'comparison.json'), '--output', str(REPORT)], environment,
              phases, deadline, before, seconds=60, cwd=ROOT)
        arm_deadline()
        require((OUT / 'render.stderr').read_bytes() == b'', 'unexpected actual render stderr')
        require(REPORT.resolve(strict=True) == REPORT
                and {p.name for p in REPORT.iterdir()} == REPORT_FILES | {'complete.json'}, 'closed five report files')
        output_pins = {p.name: h.pin(p) for p in sorted(REPORT.iterdir())}
        require(all(p.resolve(strict=True) == p for p in REPORT.iterdir())
                and sum(row['bytes'] for row in output_pins.values()) <= 1 << 20, 'ordinary bounded report bodies')
        report = json.loads((REPORT / 'complete.json').read_bytes())
        compact = lambda row: {key: row[key] for key in ('bytes', 'sha256')}
        require(set(report) == {'schema', 'passed', 'input', 'renderer', 'files', 'tensor_rows', 'argmax_rows',
                    'original_metrics_preserved', 'metrics_recomputed', 'numerical_acceptance',
                    'full_model_acceptance', 'full_long_workload', 'performance_claim', 'gpu_execution', 'model_execution'}
                and report['schema'] == 'ferric-readiness40-numerical-report-v1'
                and report['passed'] is True and type(report['tensor_rows']) is int and report['tensor_rows'] == 152
                and type(report['argmax_rows']) is int and report['argmax_rows'] == 40
                and report['original_metrics_preserved'] is True
                and all(report[k] is False for k in ('metrics_recomputed', 'numerical_acceptance',
                    'full_model_acceptance', 'full_long_workload', 'performance_claim', 'gpu_execution', 'model_execution'))
                and report['input'] == compact(before['comparison.json'])
                and report['renderer'] == compact(before['render.py'])
                and report['files'] == {name: compact(output_pins[name]) for name in REPORT_FILES}
                and json.loads((OUT / 'render.stdout').read_bytes()) == report, 'actual report stdout/body/pin/scope joins')
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
            require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', 'evidence', *SOURCES} | ({'report'} if REPORT.exists() else set()),
                    'unexpected bytecode/cache/input-root write')
            require([row['label'] for row in phases] == ['renderer-tests', 'render'][:len(phases)]
                    and len(phases) <= 2 and all(row['reaped'] and row['process_group_absent'] for row in phases),
                    'ordered owned children completely reaped')
            require(time.monotonic() < deadline, 'whole CPU bound')
            if REPORT.exists():
                current_outputs = {p.name: h.pin(p) for p in sorted(REPORT.iterdir())}
                require(set(current_outputs) <= REPORT_FILES | {'complete.json'}
                        and sum(row['bytes'] for row in current_outputs.values()) <= 1 << 20,
                        'closed bounded original report prefix')
                require(not output_pins or current_outputs == output_pins, 'report changed after capture')
                output_pins = current_outputs
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
            require(json.loads((OUT / (row['label'] + '.result.json')).read_bytes()) == row, 'original phase result join')
            started = json.loads((OUT / (row['label'] + '.started.json')).read_bytes())
            require(started == dict(pid=row['pid'], pgid=row['pgid'], argv=row['argv']), 'original child registration join')
        expected_raw = {'sources-before.json', 'sources-after.json'} | {
            label + suffix for label in ('renderer-tests', 'render')
            for suffix in ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
        require(set(raw) <= expected_raw, 'closed original raw prefix')
        if failure is None:
            require(set(raw) == expected_raw and len(phases) == 2 and census is not None and report is not None
                    and all(row['exit_code'] == 0 and row['natural_exit'] is True and row['reaped'] is True
                        and row['process_group_absent'] is True and row['forced_cleanup'] is False
                        and row['timed_out'] is False and row['exception'] is None for row in phases),
                    'two clean natural leaves and twelve original raw bodies')
    except BaseException as error:
        if time.monotonic() >= deadline:
            raise
        errors.append('raw reconciliation: ' + repr(error))
    failure = failure or ('postcheck failed' if errors else None)
    value = dict(schema='ferric-readiness40-numerical-report-cpu-v1', passed=failure is None and census is not None and report is not None,
        failure=failure, postcheck_errors=errors, controller=before['run_cpu.py'], supervisor=before['supervisor.py'],
        sources_before=before, sources_after=after, source_unchanged=after == before,
        tool_pins=tools, environment=environment, phases=phases, tests=census, raw=raw,
        report=report, report_files=output_pins, actual_comparison_input=before['comparison.json'],
        limits=dict(whole_seconds=WHOLE_SECONDS, test_seconds=LEAF_SECONDS, render_seconds=60, cleanup_reserve_seconds=CLEANUP_SECONDS,
            address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10),
        elapsed_seconds=time.monotonic() - start, synthetic_test_gate=True, actual_receipt_rendered=report is not None,
        metrics_recomputed=False, visual_inspection_performed=False, full_long_workload=False,
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
