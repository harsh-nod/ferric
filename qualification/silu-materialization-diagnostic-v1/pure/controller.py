"""Bounded synthetic qualification of the captured-BF16 product diagnostic."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import sys
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-silu-materialization-diagnostic-v1'
OUT = E / 'silu-materialization-pure-v228-v1'
MANIFEST_SHA = '9cedc8f71cd4eea8f7559ebdfdc854402be32b3ffe06ccfb097564b294452dc6'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file() and path.stat().st_size < 16 << 20, 'canonical bounded file')
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def save(name, value):
    with (OUT / name).open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode
            and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME', 'SILU_RESIDUAL_ORACLE')),
            'ordinary isolated interpreter')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded ASROCK CPU identity')
    require(all(os.environ.get(k) == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    for kind, cap in [(resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)]:
        resource.setrlimit(kind, (cap, cap))
    controller = pin(Path(__file__).resolve())
    require(len(sys.argv) == 2 and controller['sha256'] == sys.argv[1], 'root-selected controller')
    require(pin(P / 'manifest.json')['sha256'] == MANIFEST_SHA, 'root-selected package')
    manifest = json.loads((P / 'manifest.json').read_bytes())
    before = {name: pin(P / name) for name in [*manifest['files'], 'manifest.json']}
    require(all({k: before[name][k] for k in ('bytes', 'sha256')} == row for name, row in manifest['files'].items()), 'selected source bodies')
    OUT.mkdir(mode=0o700)
    save('sources-before.json', before)
    sys.path.insert(0, str(P))
    import test_diagnostic as tests
    import diagnostic
    require(Path(tests.__file__).resolve() == P / 'test_diagnostic.py'
            and Path(diagnostic.__file__).resolve() == P / 'diagnostic.py', 'selected imports')
    oracle = pin(tests.ORACLE_PATH)
    require(oracle['sha256'] == tests.ORACLE_SHA, 'selected oracle')
    suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
    names = [case.id() for group in suite for case in group]
    require(len(names) == len(set(names)) == 18 and {n.rsplit('.', 1)[-1] for n in names}
            == set(manifest['test_census']['test_diagnostic.py']), 'exact authored test census')
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after = {name: pin(P / name) for name in before}
    require(after == before and pin(tests.ORACLE_PATH) == oracle and pin(Path(__file__).resolve()) == controller, 'source postchecks')
    save('sources-after.json', after)
    with (OUT / 'tests.log').open('x') as log:
        log.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == 18 and not result.skipped
    value = dict(schema='ferric-silu-materialization-pure-v1', passed=passed, tests=result.testsRun,
        names=names, failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        controller=controller, oracle=oracle, sources_before=pin(OUT / 'sources-before.json'),
        sources_after=pin(OUT / 'sources-after.json'), transcript=pin(OUT / 'tests.log'),
        source_postchecks_passed=True, synthetic_only=True, gpu_execution=False,
        numerical_acceptance=False, performance_claim=False)
    save('complete.json' if passed else 'failed.json', value)
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, tests=result.testsRun)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
