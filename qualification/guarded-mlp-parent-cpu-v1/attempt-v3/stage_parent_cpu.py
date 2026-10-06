"""Bounded data-only staging of one explicitly bound ordinary source archive."""
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import sys
import tarfile

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-parent-cpu-v228-v3')
ARCHIVE_PIN = dict(bytes=6951208, sha256='2869a6034f29747d6365bf2f2c53ba86bd6992320291a7a04a81c25f3f867a2a')
INPUT_PIN = dict(bytes=252017, sha256='ca54d12476b990b6c604043fce376903b375c12ec0d235fe2d1390d5559aefc7')
MEMBERS = 1318


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    require(len(sys.argv) == 2 and ARCHIVE_PIN is not None and INPUT_PIN is not None
            and MEMBERS is not None, 'actual archive/input/member bindings required')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'source staging host/UID')
    path = Path(sys.argv[1])
    require(path.is_absolute() and path.resolve(strict=True) == path and path.is_file()
            and path.stat().st_size <= 96 << 20, 'ordinary bounded archive')
    require(ROOT.parent.resolve(strict=True) == ROOT.parent and not os.path.lexists(ROOT), 'fresh exact root')
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (128 << 20, 128 << 20))
    signal.alarm(120)
    os.umask(0o077)
    require(pin(path.read_bytes()) == ARCHIVE_PIN, 'actual archive identity')
    bodies = {}
    with tarfile.open(path, 'r:gz') as tar:
        for row in tar:
            name = row.name
            require(row.isfile() and row.pax_headers == {} and name not in bodies
                    and Path(name).as_posix() == name and not name.startswith('/')
                    and '..' not in Path(name).parts and row.size <= 16 << 20
                    and len(bodies) < 3000, 'ordinary unique source member')
            stream = tar.extractfile(row)
            require(stream is not None, 'source member body')
            body = stream.read(row.size + 1)
            require(len(body) == row.size, 'source member extent')
            bodies[name] = body
            require(sum(map(len, bodies.values())) <= 96 << 20, 'expanded source cap')
    require(len(bodies) == MEMBERS and pin(bodies['input-manifest.json']) == INPUT_PIN, 'closed actual input')
    value = json.loads(bodies['input-manifest.json'])
    require(value['schema'] == 'ferric-guarded-mlp-parent-cpu-input-v1', 'parent source schema')
    expected = dict(value['files'])
    expected.update({'inputs/' + n: p for n, p in value['lineage'].items()})
    expected['input-manifest.json'] = INPUT_PIN
    require({n: pin(b) for n, b in bodies.items()} == expected, 'complete archive body closure')
    ROOT.mkdir(mode=0o700)
    for name, body in sorted(bodies.items()):
        dest = ROOT / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        with dest.open('xb') as stream:
            stream.write(body)
        require(pin(dest.read_bytes()) == expected[name], 'staged source posthash')
    require(pin(path.read_bytes()) == ARCHIVE_PIN, 'archive posthash')
    result = dict(schema='ferric-guarded-mlp-parent-source-stage-v1', passed=True,
        archive=ARCHIVE_PIN, input_manifest=INPUT_PIN, members=MEMBERS,
        source_files=len(value['files']), root=str(ROOT), project_code_executed=False,
        cargo_execution=False, gpu_execution=False, qualification_passed=False)
    with (ROOT / 'stage.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
