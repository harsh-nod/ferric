"""Run capture-only fixtures on MI350 through the existing owned CPU supervisor."""
import ast
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import stat
import sys
import time
import types

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-handoff-capture-tests-v228-v1')
KNOWN = {
    'lowering.py': (106801, '4559d99f7843fe9d7d7220105971e8818bf531f6b4a16a6fcbf1cd106921d68d'),
    'previous_lowering.py': (97963, '637857bc4a5178bae425135271a3c537f14269f368d62d3dc82107fe5c13f33e'),
    'test_capture.py': (15319, '76c795d81072f785d5774424bb2a22f2da6cb36529d1f923fb456020671a0adf'),
    'supervisor.py': (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
}
TOOLS = {
    'python': dict(path='/usr/bin/python3.12', bytes=8020928, sha256='e1efa562c2cc2e35521a5c9c9b9939921001ff8ca9708a13ef15ace68cc2ccd7'),
    'prlimit': dict(path='/usr/bin/prlimit', bytes=27536, sha256='17064f67e650d6152a6902b013aab496b54c87587c8eea6f5023aafee6154069'),
}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def read(path, limit=16 << 20):
    path = Path(path)
    before = path.lstat()
    require(path.is_absolute() and path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= limit, 'ordinary bounded source/tool required')
    body = path.read_bytes()
    after = path.lstat()
    fields = ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns', 'st_mode')
    require(tuple(getattr(before, k) for k in fields) == tuple(getattr(after, k) for k in fields)
            and len(body) == before.st_size, 'source/tool drift')
    return body


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    started = time.monotonic()
    deadline = started + 180
    require(__debug__ and sys.flags.isolated and sys.flags.no_site and sys.dont_write_bytecode
            and len(sys.argv) == 2, 'isolated interpreter and input digest required')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and Path(__file__).resolve() == ROOT / 'run_tests.py', 'host/UID/controller mismatch')
    names = set(KNOWN) | {'run_tests.py'}
    require({p.name for p in ROOT.iterdir()} == names | {'input-manifest.json'}, 'fresh source-only namespace required')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB storage floor')
    bodies = {name: read(ROOT / name, 1 << 20) for name in names}
    before = {name: pin(body) for name, body in bodies.items()}
    for name, expected in KNOWN.items():
        require((before[name]['bytes'], before[name]['sha256']) == expected, 'source pin: ' + name)
    input_bytes = read(ROOT / 'input-manifest.json', 1 << 20)
    require(pin(input_bytes)['sha256'] == sys.argv[1], 'input manifest digest')
    require(json.loads(input_bytes) == dict(schema='ferric-handoff-capture-tests-input-v1', files=before, tools=TOOLS),
            'closed source/tool manifest')
    require(Path(sys.executable).resolve() == Path(TOOLS['python']['path']), 'Python path')
    for row in TOOLS.values():
        require(pin(read(Path(row['path']))) == {k: row[k] for k in ('bytes', 'sha256')}, 'tool digest')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    require(os.getpriority(os.PRIO_PROCESS, 0) in (0, 10), 'nice level')
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = types.ModuleType('owned_capture_fixture_supervisor')
    h.__file__ = str(ROOT / 'supervisor.py')
    exec(compile(bodies['supervisor.py'], h.__file__, 'exec'), h.__dict__)
    h.ROOT = ROOT
    h.OUT, h.TARGET, h.TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
    require(h.CLEANUP_RESERVE == 50 and h.AS_LIMIT == 12 << 30 and h.FILE_LIMIT == 1 << 30
            and h.CPU_LIMIT == 1200 and h.CACHE_LIMIT == 6 << 30 and h.STREAM_LIMIT == 64 << 20
            and h.LIVE_FREE == 38 << 30, 'unchanged supervisor limits')
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    for directory in (h.OUT, h.TARGET, h.TMP):
        directory.mkdir(mode=0o700)
    phases, postchecks, failure, observation = [], [], None, None
    h.save('sources-before.json', before)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - 50 - time.monotonic()))
    try:
        env = dict(PATH='/usr/bin:/bin', HOME=str(h.TMP), TMPDIR=str(h.TMP), LANG='C', LC_ALL='C',
                   ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        h.run('capture-tests', [TOOLS['python']['path'], '-I', '-S', '-B', str(ROOT / 'test_capture.py'),
                               str(ROOT / 'lowering.py'), str(ROOT / 'previous_lowering.py')],
              env, phases, deadline, None, 120, h.TMP)
        observation = json.loads(read(h.OUT / 'capture-tests.stdout', 1 << 20))
        tree = ast.parse(bodies['test_capture.py'])
        expected = sorted('__main__.' + cls.name + '.' + test.name
                          for cls in tree.body if isinstance(cls, ast.ClassDef) and cls.name.endswith('Tests')
                          for test in cls.body if isinstance(test, ast.FunctionDef) and test.name.startswith('test_'))
        require(len(expected) == len(set(expected)) == 29 and observation == dict(
            schema='ferric-handoff-capture-tests-v1', passed=True, names=expected, passing_names=expected,
            tests_run=29, failures=0, errors=0, skipped=0, expected_failures=0, unexpected_successes=0,
            source_unchanged=True, synthetic_capture_only=True, compiler_qualification=False, gpu_execution=False),
            'exact passing capture fixture roster')
        require(len(phases) == 1, 'one fixture leaf')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    after = None
    for label, action in (
        ('sources', lambda: {name: pin(read(ROOT / name, 1 << 20)) for name in names} == before),
        ('input', lambda: read(ROOT / 'input-manifest.json', 1 << 20) == input_bytes),
        ('tools', lambda: all(pin(read(Path(row['path']))) == {k: row[k] for k in ('bytes', 'sha256')} for row in TOOLS.values())),
        ('scratch cleanup', lambda: not list(h.TMP.iterdir())),
    ):
        try:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - 5 - time.monotonic()))
            require(action(), label + ' drift')
        except BaseException as error:
            postchecks.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    try:
        after = {name: pin(read(ROOT / name, 1 << 20)) for name in names}
        h.save('sources-after.json', after)
    except BaseException as error:
        postchecks.append('source retention: ' + repr(error))
    if time.monotonic() >= deadline:
        postchecks.append('whole deadline exceeded')
    result = dict(schema='ferric-handoff-capture-tests-cpu-v1', passed=failure is None and not postchecks,
                  failure=failure, postcheck_errors=postchecks, phases=phases, child_observation=observation,
                  sources_before=before, sources_after=after, input_manifest=pin(input_bytes), tools=TOOLS,
                  elapsed_seconds=time.monotonic() - started, synthetic_capture_only=True,
                  compiler_qualification=False, gpu_execution=False, numerical_acceptance=False,
                  production_authority=False, load_authority=False, launch_authority=False,
                  raw={p.name: h.pin(p) for p in sorted(h.OUT.iterdir()) if p.is_file()})
    receipt = h.save('complete.json' if result['passed'] else 'failed.json', result)
    print(json.dumps(receipt), flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
