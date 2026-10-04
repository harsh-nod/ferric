"""Bounded CPU-only tests or historical residual replay on MI350."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import sys
import time
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-residual-capture-replay-v1'
EXPECTED = {
    'run.py': '88f6e66fccb2ea1ef0d6079858a4b6af1d0b609fa50ec3ea02cb21329300021a',
    'test_run.py': '39efc6facf38da8cc5b0efb1a39df79a8369f6c121fdd7c8f953a5845c8a0a98',
    'README.md': 'fcf594296e6ae7804415dca4c55759ff4ba38042491fdc3c914a910183db6263',
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical source')
    raw = path.read_bytes()
    require(len(raw) <= 16 << 20, 'bounded source/result')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def snapshot():
    require({p.name for p in P.iterdir()} == set(EXPECTED), 'exact package census')
    result = {name: pin(P / name) for name in EXPECTED}
    require(all(result[name]['sha256'] == sha for name, sha in EXPECTED.items()), 'reviewed source hashes')
    return result


def save(path, value):
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    return pin(path)


def main():
    require(len(sys.argv) == 3 and sys.argv[1] in ('tests', 'replay')
            and re.fullmatch(r'residual-capture-(pure|replay)-v228-v[1-9][0-9]*', sys.argv[2]),
            'MODE FRESH_LABEL')
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
            and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ, 'isolated Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded MI350 CPU identity')
    require(all(os.environ.get(k) == '' for k in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no GPU visibility')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        limits = resource.getrlimit(kind)
        cap = min([cap] + [n for n in limits if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    before = snapshot(); controller = pin(Path(__file__).resolve())
    out = E / sys.argv[2]
    require(not os.path.lexists(out), 'fresh output')
    out.mkdir(mode=0o700)
    source_before = save(out / 'sources-before.json', before)
    start = time.monotonic()
    sys.path.insert(0, str(P))
    if sys.argv[1] == 'tests':
        import test_run as T
        require(Path(T.__file__).resolve() == P / 'test_run.py', 'actual test module')
        suite = unittest.defaultTestLoader.loadTestsFromModule(T)
        require(suite.countTestCases() == 14, 'exact authored test census')
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
        with (out / 'tests.log').open('x') as log:
            log.write(stream.getvalue())
        passed = result.wasSuccessful() and result.testsRun == 14 and not result.skipped
        details = dict(tests=result.testsRun, errors=len(result.errors), failures=len(result.failures),
                       skipped=len(result.skipped), transcript=pin(out / 'tests.log'))
        print(stream.getvalue(), end='', flush=True)
    else:
        import run as R
        require(Path(R.__file__).resolve() == P / 'run.py', 'actual replay module')
        result = R.replay(E, E / 'p227-prefix-layer-replay-v1/layer_validation.py',
            E / 'p228-independent-layer-reference-v1',
            Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target/model-00001-of-00005.safetensors'),
            E.parent / 'finite-two-forward-v223/p224-prompt-v1/prompt.u32le')
        passed = result['passed'] is True
        details = dict(historical_result=save(out / 'residuals.json', result))
    after = snapshot()
    require(before == after and pin(Path(__file__).resolve()) == controller, 'unchanged executed sources')
    value = dict(schema='ferric-p228-residual-capture-cpu-v1', mode=sys.argv[1], passed=passed,
        controller=controller, sources_before=source_before, sources_after=save(out / 'sources-after.json', after),
        source_postchecks_passed=True, elapsed_seconds=time.monotonic() - start, details=details,
        gpu_execution=False, new_v7_image_checked=False, full_layer_acceptance=False,
        full_model_acceptance=False, production_authority=False, performance_claim=False)
    receipt = save(out / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, receipt=receipt)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
