"""Bounded retained-data analysis on MI350; no subprocess or GPU launches."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import time
import types
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CASE = E / 'guarded-mlp-model-gpu-v228-v3/ar4'
OUT = E / 'guarded-mlp-model-host-timing-v228-v1'
VALIDATOR = E / 'guarded-mlp-model-numerical-comparison-input-v228-v1/validate_observation.py'
PINS = {
    'timing.py': 'b8dcf6ddb7e2e42f3b1c427a726d3b7e42e48667373b7472c11fc79881b5e5f7',
    'test_timing.py': '6cd550c2763ce30d66638dc15013a194991ae5c6db80a8bc7d6918140d5d5f81',
    str(VALIDATOR): '367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055',
    str(CASE / 'complete.json'): 'edf05cf2dd19a9934dab9762b328ea3e6160cf01c15606a3df3299ce967e4991',
    str(CASE / 'native/complete.json'): '5ebc12a40f6e1fff7b8b05510af0b94ca8c3fd15ac96ba119a372af1ff3c046a',
}
INPUTS = {}


def read(path, digest=None):
    path = Path(path)
    before = path.lstat()
    if path.resolve(strict=True) != path or not stat.S_ISREG(before.st_mode) or before.st_size > 1 << 20:
        raise ValueError('bounded canonical regular input required')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        if stamp(before) != stamp(os.fstat(stream.fileno())):
            raise ValueError('input changed at open')
        raw = stream.read((1 << 20) + 1)
        if len(raw) != before.st_size or stamp(before) != stamp(os.fstat(stream.fileno())) or stamp(before) != stamp(path.lstat()):
            raise ValueError('input changed during read')
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    if digest is not None and digest != pin['sha256']:
        raise ValueError('input hash mismatch')
    if str(path) in INPUTS and INPUTS[str(path)] != pin:
        raise ValueError('input changed across reads')
    INPUTS[str(path)] = pin
    if len(INPUTS) > 32 or sum(p['bytes'] for p in INPUTS.values()) > 8 << 20:
        raise ValueError('readset bound')
    return raw


def module(name, path, digest):
    raw = read(path, digest)
    value = types.ModuleType(name)
    value.__file__ = str(path)
    sys.modules[name] = value
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def main():
    if not __debug__ or not sys.dont_write_bytecode or len(sys.argv) != 1:
        raise ValueError('python -B run.py only')
    if os.getuid() != 9661 or os.uname().nodename != 'smci350-rck-g03-b19-03':
        raise ValueError('MI350 host required')
    os.sched_setaffinity(0, {8, 9})
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        os.environ[name] = ''
    for kind, maximum in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_CPU, 30),
                          (resource.RLIMIT_FSIZE, 1 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([maximum] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('30s deadline')))
    signal.alarm(30)
    OUT.mkdir(mode=0o700)
    started = time.monotonic()
    result = dict(passed=False, gpu_execution=False, throughput_measured=False,
                  overlap_measured=False, numerical_acceptance=False)
    error = None
    try:
        root = Path(__file__).resolve().parent
        read(Path(__file__).resolve())
        t = module('timing', root / 'timing.py', PINS['timing.py'])
        tests = module('test_timing', root / 'test_timing.py', PINS['test_timing.py'])
        log = io.StringIO()
        test = unittest.TextTestRunner(stream=log, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(tests))
        result['tests'] = dict(run=test.testsRun, failures=len(test.failures), errors=len(test.errors),
                               skipped=len(test.skipped), log=log.getvalue())
        if test.testsRun != 4 or not test.wasSuccessful() or test.skipped:
            raise ValueError('four required tests')
        v = module('validator', VALIDATOR, PINS[str(VALIDATOR)])
        terminal = v.parse(read(CASE / 'complete.json', PINS[str(CASE / 'complete.json')]))
        if terminal['passed'] is not True or terminal['errors'] != []:
            raise ValueError('successful actual outer controller required')
        raw = read(CASE / 'native/complete.json', PINS[str(CASE / 'native/complete.json')])
        native = v.parse(raw)
        def body(pin):
            data = read(pin['path'], pin['sha256'])
            if len(data) != pin['bytes']:
                raise ValueError('body extent')
            return data
        result['observation'] = v.validate(raw, native['request'], CASE / 'native', body)
        result['positions'] = [dict(position=i, **t.counters(body(v.rust_pin(frame['control']))))
                               for i, frame in enumerate(native['files']['frames'])]
        result['scope'] = 'host dispatch spans from retained controls; not GPU timestamps or complete forward latency'
        result['passed'] = True
    except Exception as exc:
        error = type(exc).__name__ + ': ' + str(exc)
    posterrors = []
    for pin in list(INPUTS.values()):
        try:
            read(pin['path'], pin['sha256'])
        except Exception as exc:
            posterrors.append(str(exc))
    result.update(error=error, postcheck_errors=posterrors, inputs=list(INPUTS.values()),
                  cpu_analysis_seconds=time.monotonic() - started)
    result['passed'] = result['passed'] and error is None and not posterrors
    with (OUT / ('complete.json' if result['passed'] else 'failed.json')).open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    signal.alarm(0)
    print(json.dumps(dict(passed=result['passed'], error=error, output=str(OUT))))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
