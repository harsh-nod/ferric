"""Exercise frozen observer policies in-process, without native or GPU execution."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import sys
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-independent-gpu-observation-v1'
OUT = E / 'independent-observer-pure-v228-v1'


def pin(path):
    assert path.resolve(strict=True) == path and path.is_file()
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def save(path, body):
    with path.open('x', encoding='ascii') as stream:
        json.dump(body, stream, indent=2, sort_keys=True)
        stream.write('\n')


def main():
    if sys.flags.optimize or 'PYTHONOPTIMIZE' in os.environ:
        raise RuntimeError('unoptimized Python required')
    assert len(sys.argv) == 2 and len(sys.argv[1]) == 64 and sys.dont_write_bytecode
    digest = sys.argv[1]
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        value = min([cap] + [word for word in old if word != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))

    def verify():
        assert pin(P / 'manifest.json')['sha256'] == digest
        value = json.loads((P / 'manifest.json').read_bytes())
        assert value['schema'] == 'ferric-p228-independent-gpu-observation-contracts-v1'
        assert value['pure_tests'] == 81 and len(value['files']) == 18
        assert len({row['path'] for row in value['files']}) == 18
        result = {}
        for row in value['files']:
            path = P / row['path']
            assert path.resolve(strict=True).is_relative_to(P)
            result[row['path']] = pin(path)
            assert {key: result[row['path']][key] for key in ('bytes', 'sha256')} == {
                key: row[key] for key in ('bytes', 'sha256')}
        return result

    before = verify()
    OUT.mkdir(mode=0o700)
    save(OUT / 'sources-before.json', before)
    sys.path.insert(0, str(P))
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
        unittest.defaultTestLoader.discover(str(P), pattern='test_*.py'))
    assert verify() == before
    (OUT / 'tests.log').write_text(stream.getvalue(), encoding='utf-8')
    passed = result.wasSuccessful() and result.testsRun == 81 and not result.skipped
    body = dict(passed=passed, tests=result.testsRun, manifest_sha256=digest,
        source_sha256=pin(OUT / 'sources-before.json')['sha256'],
        controller_sha256=pin(Path(__file__).resolve())['sha256'],
        gpu_execution=False, numerical_acceptance=False)
    output = OUT / ('complete.json' if passed else 'failed.json')
    save(output, body)
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, output=pin(output))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
