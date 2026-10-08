"""One supervised CPU-only admission of original diagnostic products on MI350."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-currentness-duration-live-admission-cpu-v228-v1')
OUT = ROOT / 'evidence'
SOURCES = {
    'live_admission.py': (5451, 'fd8c84dc601262c73c398d3abdf34f8db3ca1b7f0b212470d8e473de28f37b25'),
    'run_model_gpu.py': (54215, '2803e5783d9afaf2b58c704b2bf7dddc053ee44aca29a5c1fbf1a845c887daba'),
    'library_audit.py': (25872, 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'),
    'supervisor.py': (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def main():
    start = time.monotonic()
    deadline = start + 180
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -I -B only')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact CPU root/host/uid')
    names = {'run_cpu.py', *SOURCES}
    require({p.name for p in ROOT.iterdir()} == names and not os.path.lexists(OUT), 'fresh closed root')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (bound, bound))
    helper_bodies = {}
    for name, (size, sha) in SOURCES.items():
        path = ROOT / name
        require(path.resolve(strict=True) == path and path.is_file(), 'ordinary canonical helper')
        raw = path.read_bytes()
        require(len(raw) == size and hashlib.sha256(raw).hexdigest() == sha, 'exact reviewed helper: ' + name)
        helper_bodies[name] = raw
    h = types.ModuleType('qualified_live_cpu_supervisor')
    h.__file__ = str(ROOT / 'supervisor.py')
    exec(compile(helper_bodies['supervisor.py'], h.__file__, 'exec'), h.__dict__)
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h.OUT = OUT
    h.AS_LIMIT, h.FILE_LIMIT, h.STREAM_LIMIT = 512 << 20, 16 << 20, 4 << 20
    h.CPU_LIMIT, h.CLEANUP_RESERVE, h.CACHE_LIMIT = 60, 50, 4 << 20
    require(h.shutil.disk_usage(ROOT).free >= h.START_FREE, 'initial40GiB floor')
    def sources():
        return {name: h.pin(ROOT / name) for name in sorted(names)}
    h.sources = sources
    before = sources()
    require(all((before[name]['bytes'], before[name]['sha256']) == expected
                for name, expected in SOURCES.items()), 'loaded helper/source map identity')
    python = Path('/usr/bin/python3').resolve(strict=True)
    prlimit = Path('/usr/bin/prlimit').resolve(strict=True)
    tools = dict(python=h.pin(python), prlimit=h.pin(prlimit))
    environment = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        PYTHONPATH='', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    OUT.mkdir(mode=0o700)
    phases, errors = [], []
    failure = result = after = None
    def interrupted(number, _frame):
        raise RuntimeError('CPU controller signal ' + str(number))
    handlers = {n: signal.getsignal(n) for n in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP)}
    for number in handlers:
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        h.save('sources-before.json', before)
        h.run('live-admission', [str(python), '-I', '-B', str(ROOT / 'live_admission.py')],
              environment, phases, deadline, before, seconds=60, cwd=ROOT)
        require((OUT / 'live-admission.stderr').read_bytes() == b'', 'clean live admission stderr')
        result = json.loads((OUT / 'live-admission.stdout').read_bytes())
        require(result['schema'] == 'ferric-currentness-duration-live-cpu-admission-v1'
                and result['passed'] is True and result['original_products_admitted'] is True
                and all(result[k] is False for k in ('synthetic_fixture', 'native_launch_admitted',
                    'native_parent_execution', 'gpu_execution', 'model_execution',
                    'numerical_acceptance', 'performance_claim')), 'actual bounded original admission result')
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    finally:
        for number in handlers:
            signal.signal(number, interrupted if number == signal.SIGALRM else signal.SIG_IGN)
        signal.setitimer(signal.ITIMER_REAL, max(.001, deadline - time.monotonic()))
        try:
            after = sources()
            h.save('sources-after.json', after)
            require(after == before, 'source drift')
            require(h.pin(python) == tools['python'] and h.pin(prlimit) == tools['prlimit']
                    and Path('/usr/bin/python3').resolve(strict=True) == python
                    and Path('/usr/bin/prlimit').resolve(strict=True) == prlimit, 'tool drift')
            require({p.name for p in ROOT.iterdir()} == names | {'evidence'}, 'unexpected root write')
            require(len(phases) == 1 and all(p['reaped'] and p['process_group_absent'] for p in phases),
                    'sole owned phase retired')
            row = phases[0]
            require(json.loads((OUT / 'live-admission.result.json').read_bytes()) == row, 'original phase result')
            require(json.loads((OUT / 'live-admission.started.json').read_bytes()) ==
                    dict(pid=row['pid'], pgid=row['pgid'], argv=row['argv']), 'original child registration')
            for key in ('command', 'stdout', 'stderr'):
                require(h.pin(Path(row[key]['path'])) == row[key], 'raw phase pin drift')
            require(time.monotonic() < deadline, 'whole CPU bound')
        except BaseException as error:
            errors.append(type(error).__name__ + ': ' + str(error))
    raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    require(set(raw) <= {'sources-before.json', 'sources-after.json', 'live-admission.command.json',
        'live-admission.started.json', 'live-admission.result.json', 'live-admission.stdout',
        'live-admission.stderr'}, 'closed raw prefix')
    value = dict(schema='ferric-currentness-duration-live-admission-cpu-v1',
        passed=failure is None and not errors and result is not None,
        failure=failure, postcheck_errors=errors, controller=before['run_cpu.py'],
        supervisor=before['supervisor.py'], sources_before=before, sources_after=after,
        source_unchanged=before == after, tool_pins=tools, phases=phases, raw=raw, admission=result,
        environment=environment, elapsed_seconds=time.monotonic() - start,
        limits=dict(whole_seconds=180, leaf_seconds=60, cleanup_seconds=50,
                    address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10),
        data_only=True, gpu_execution=False, native_parent_execution=False, model_execution=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    h.save('complete.json' if value['passed'] else 'failed.json', value)
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items():
        signal.signal(number, handler)
    print(json.dumps({k: value[k] for k in ('passed', 'failure', 'postcheck_errors')}, sort_keys=True))
    return 0 if value['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
