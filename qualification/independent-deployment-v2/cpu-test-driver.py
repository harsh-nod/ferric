"""Run the frozen V2 deployment suite on MI350, without GPU or native launches."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import sys
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-independent-deployment-v2'
OUT = E / 'independent-deployment-pure-v228-v2'


def pin(path):
    assert path.resolve(strict=True) == path and path.is_file()
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    if sys.flags.optimize or 'PYTHONOPTIMIZE' in os.environ:
        raise RuntimeError('unoptimized Python required')
    assert len(sys.argv) == 2 and len(sys.argv[1]) == 64 and sys.dont_write_bytecode
    digest = sys.argv[1]
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert sorted(os.sched_getaffinity(0)) == [8, 9] and os.getpriority(os.PRIO_PROCESS, 0) == 10
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        value = min([cap] + [word for word in old if word != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))

    def verify():
        assert pin(P / 'manifest.json')['sha256'] == digest
        value = json.loads((P / 'manifest.json').read_bytes())
        assert value['schema'] == 'ferric-p228-independent-deployment-package-v1'
        assert value['pure_tests'] == 30 and len(value['files']) == 14
        assert len({row['path'] for row in value['files']}) == 14
        for row in value['files']:
            assert (P / row['path']).resolve(strict=True).is_relative_to(P)
            actual = pin(P / row['path'])
            assert {key: actual[key] for key in ('bytes', 'sha256')} == {
                key: row[key] for key in ('bytes', 'sha256')}

    verify()
    OUT.mkdir(mode=0o700)
    sys.path.insert(0, str(P))
    import test_portable
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromModule(test_portable))
    verify()
    (OUT / 'tests.log').write_text(stream.getvalue(), encoding='utf-8')
    passed = result.wasSuccessful() and result.testsRun == 30 and not result.skipped
    body = dict(schema='ferric-p228-independent-deployment-pure-v1', passed=passed,
        tests=result.testsRun, package_manifest=pin(P / 'manifest.json'),
        controller=pin(Path(__file__).resolve()), test_source=pin(P / 'test_portable.py'),
        transcript=pin(OUT / 'tests.log'), gpu_execution=False, numerical_acceptance=False)
    output = OUT / ('complete.json' if passed else 'failed.json')
    with output.open('x', encoding='ascii') as stream_out:
        json.dump(body, stream_out, indent=2, sort_keys=True)
        stream_out.write('\n')
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, output=pin(output))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
