"""Run the fixed parser regression suite using the existing owned supervisor."""
import ast
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import sys
import time
import types

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-libtest-progress-parser-tests-v228-v1')
KNOWN = {
    'qualification_helpers.py': (4777, 'ba127f1057546c0ce6e57fb78832c3e774c1108a4f7c5a2421f4aad7f51108be'),
    'test_qualification_helpers.py': (10026, '1990d2dd7d03b6814ae96d88436d82e0e4241c730bfd405e3793e740f872005d'),
    'supervisor.py': (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
    'baseline-complete.json': (1344193, '439d4b5bf10a1fe9f44d02fa1cbd28b03eec9601bb161806909b2f073beb9018'),
    'actual-failed.json': (900004, '67f0792e4d90b33f939fa1df7af45024d27d0cdb184ff82b7762b89f0910a9dd'),
    'actual-compiler-tests.stdout': (164031, '77d8c1c15022e2addd6b1e2411ea1f6464b4e151528cd619941eea64f7a7d3d1'),
}


def pin(path):
    assert path.resolve(strict=True) == path and path.is_file() and path.stat().st_size < 64 << 20
    body = path.read_bytes()
    return dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def main():
    start = time.monotonic()
    deadline = start + 180
    assert __debug__ and sys.flags.isolated and sys.flags.no_site and sys.dont_write_bytecode
    assert Path(__file__).resolve() == ROOT / 'run_parser_tests.py'
    assert len(sys.argv) == 2 and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1])
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert ROOT.resolve() == ROOT and ROOT.stat().st_uid == 9661
    names = set(KNOWN) | {'run_parser_tests.py'}
    assert {p.name for p in ROOT.iterdir()} == names | {'input-manifest.json'}
    assert shutil.disk_usage(ROOT).free >= 40 << 30
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    assert priority in (0, 10)
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30),
                      (resource.RLIMIT_FSIZE, 1 << 30)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    assert ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - 50 - time.monotonic()))
    ip = pin(ROOT / 'input-manifest.json')
    assert ip['sha256'] == sys.argv[1]
    inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
    assert set(inputs) == {'schema', 'files', 'tool_pins'}
    assert inputs['schema'] == 'ferric-libtest-progress-parser-tests-input-v1'
    assert set(inputs['files']) == names and set(inputs['tool_pins']) == {'python', 'prlimit'}
    before = {name: pin(ROOT / name) for name in sorted(names)}
    assert {name: {k: row[k] for k in ('bytes', 'sha256')} for name, row in before.items()} == inputs['files']
    assert all((before[n]['bytes'], before[n]['sha256']) == row for n, row in KNOWN.items())
    assert all(pin(Path(row['path'])) == row for row in inputs['tool_pins'].values())
    python = inputs['tool_pins']['python']['path']
    assert Path(python) == Path(sys.executable).resolve() and inputs['tool_pins']['prlimit']['path'] == '/usr/bin/prlimit'
    tree = ast.parse((ROOT / 'test_qualification_helpers.py').read_bytes())
    expected = sorted('__main__.' + cls.name + '.' + fn.name for cls in tree.body
                      if isinstance(cls, ast.ClassDef) for fn in cls.body
                      if isinstance(fn, ast.FunctionDef) and fn.name.startswith('test_'))
    assert len(expected) == len(set(expected)) == 17
    h = types.ModuleType('owned_parser_supervisor')
    h.__file__ = str(ROOT / 'supervisor.py')
    exec(compile((ROOT / 'supervisor.py').read_bytes(), h.__file__, 'exec'), h.__dict__)
    h.ROOT = ROOT
    h.OUT, h.TARGET, h.TMP = (ROOT / n for n in ('evidence', 'target', 'tmp'))
    assert h.CLEANUP_RESERVE == 50 and h.AS_LIMIT == 12 << 30 and h.FILE_LIMIT == 1 << 30
    assert h.CPU_LIMIT == 1200 and h.CACHE_LIMIT == 6 << 30 and h.STREAM_LIMIT == 64 << 20 and h.LIVE_FREE == 38 << 30
    h.OUT.mkdir(mode=0o700)
    phases, failure, postchecks, actual = [], None, [], []
    after = None
    try:
        h.TARGET.mkdir(mode=0o700)
        h.TMP.mkdir(mode=0o700)
        h.save('sources-before.json', before)
        env = dict(HOME='/home/harmenon', PATH=str(Path(python).parent) + ':/usr/bin:/bin',
                   LANG='C', LC_ALL='C', TMPDIR=str(h.TMP),
                   ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        h.run('parser-tests', [python, '-I', '-S', '-B', str(ROOT / 'test_qualification_helpers.py')],
              env, phases, deadline, None, 120, ROOT)
        stderr = (h.OUT / 'parser-tests.stderr').read_text()
        actual = re.findall(r'^test_\S+ \(([^()]+)\) \.\.\. (ok|FAIL|ERROR|skipped[^\n]*)$', stderr, re.M)
        assert actual == [(name, 'ok') for name in expected]
        assert len(re.findall(r'^Ran 17 tests in [0-9.]+s$', stderr, re.M)) == 1
        assert stderr.endswith('\nOK\n') and (h.OUT / 'parser-tests.stdout').read_bytes() == b''
        assert len(phases) == 1
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    try:
        signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic() - 5))
        after = {name: pin(ROOT / name) for name in sorted(names)}
        h.save('sources-after.json', after)
        assert after == before and pin(ROOT / 'input-manifest.json') == ip
        assert all(pin(Path(row['path'])) == row for row in inputs['tool_pins'].values())
    except BaseException as error:
        postchecks.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    failure = failure or ('postcheck failed' if postchecks else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    result = dict(schema='ferric-libtest-progress-parser-tests-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=postchecks, phases=phases, named_outcomes=actual,
                  input_manifest=ip, source_inputs=before, source_unchanged=after == before,
                  tool_pins=inputs['tool_pins'], controller=before['run_parser_tests.py'],
                  supervisor=before['supervisor.py'], raw={p.name: pin(p) for p in sorted(h.OUT.iterdir())},
                  tests_passed=len(actual) if failure is None else None,
                  elapsed_seconds=time.monotonic() - start, compiler_qualification=False, gpu_execution=False,
                  limits=dict(whole_seconds=180, leaf_seconds=120, cleanup_reserve_seconds=50,
                              address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT, cpu_seconds_per_leaf=h.CPU_LIMIT,
                              scratch_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT, initial_free_bytes=40 << 30,
                              live_free_bytes=h.LIVE_FREE, affinity=[8, 9], nice=10))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
