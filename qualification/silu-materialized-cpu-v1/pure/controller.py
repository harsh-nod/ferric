"""Execute the frozen SiLU CPU controller's synthetic tests without a compiler."""
import hashlib
import json
import os
from pathlib import Path
import resource
import sys
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-silu-materialized-cpu-v1'
OUT = E / 'silu-materialized-cpu-pure-v228-v1'
MANIFEST_SHA = 'ad306aeb5ecc446ab8d4f0cf48a0f83e4b28626781743adf82332c140e7dac14'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical source')
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode, 'plain bytecode-free Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded ASROCK CPU owner')
    for kind, cap in [(resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)]:
        resource.setrlimit(kind, (cap, cap))
    require(pin(P / 'manifest.json')['sha256'] == MANIFEST_SHA, 'frozen package')
    manifest = json.loads((P / 'manifest.json').read_bytes())
    before = {name: pin(P / name) for name in ('manifest.json', 'run.py', 'test_run.py', 'README.md')}
    before['controller'] = pin(Path(__file__).resolve())
    for name, record in manifest['files'].items():
        require({k: before[name][k] for k in ('bytes', 'sha256')} == record, 'frozen source hash')
    OUT.mkdir(mode=0o700)
    with (OUT / 'sources-before.json').open('x') as stream:
        json.dump(before, stream, indent=2, sort_keys=True)
    suite = unittest.defaultTestLoader.discover(str(P), pattern='test_run.py')

    def names(value):
        return [name for child in value for name in (names(child) if isinstance(child, unittest.TestSuite) else [child.id()])]

    inventory = sorted(names(suite))
    require(inventory == sorted('test_run.PolicyTests.' + name for name in manifest['test_census']['test_run.py'])
            and len(inventory) == 12, 'twelve exact named tests')
    with (OUT / 'tests.log').open('x') as stream:
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after = {name: pin(P / name) for name in ('manifest.json', 'run.py', 'test_run.py', 'README.md')}
    after['controller'] = pin(Path(__file__).resolve())
    with (OUT / 'sources-after.json').open('x') as stream:
        json.dump(after, stream, indent=2, sort_keys=True)
    passed = result.wasSuccessful() and result.testsRun == 12 and not result.skipped and before == after
    receipt = dict(schema='ferric-p228-silu-materialized-cpu-pure-v1', passed=passed,
        tests=result.testsRun, failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        names=inventory, controller=before['controller'], source_postchecks_passed=before == after,
        raw={name: pin(OUT / name) for name in ('sources-before.json', 'sources-after.json', 'tests.log')},
        synthetic_only=True, rust_compilation=False, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
    with (OUT / ('complete.json' if passed else 'failed.json')).open('x') as stream:
        json.dump(receipt, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(receipt), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
