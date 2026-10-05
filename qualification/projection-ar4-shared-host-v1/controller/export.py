"""Export actual shared-host CPU evidence, omitting remotely verified ELF bodies."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PROPOSAL = E / 'p228-projection-ar4-shared-host-v1/source-manifest.json'
PROPOSAL_SHA = 'faaece5ab1afea85b7b6f03b02d772847889565a0ae70922180d6f62a1770461'
CONTROLLER_SHA = '1f9fc61f35cc2d8367592ecf29ff7ea48e6d6250556a72940abe3acb028a41c5'
PARENT = 'ferric-qwen3-finite-projection-residual-decode'
BINARIES = {PARENT + suffix for suffix in ('-engineering', '-host-engineering', '-shared-host-engineering')}
BINARIES.add('ferric-tp-peer-finite-engineering-worker-v1')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def stamp(value):
    return tuple(getattr(value, key) for key in
        ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns'))


def pin(path):
    require(path.resolve(strict=True) == path and path.is_relative_to(E), 'canonical E member')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == 9661
            and 0 <= before.st_size <= 128 << 20, 'bounded owned regular file')
    digest = hashlib.sha256()
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened identity')
        while block := stream.read(1 << 20):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'stable opened body')
    require(stamp(path.lstat()) == stamp(before), 'stable named body')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
            and len(sys.argv) == 3, 'ordinary python -B COMPLETE_PATH ACTUAL_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'owned ASROCK CPU8,9 nice10')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 180),
                      (resource.RLIMIT_FSIZE, 256 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (cap, cap))
    receipt, expected_sha = Path(sys.argv[1]), sys.argv[2]
    case = receipt.parent
    require(receipt.name == 'complete.json' and case.parent == E
            and re.fullmatch(r'projection-ar4-shared-host-cpu-v228-v[1-9][0-9]*', case.name)
            and re.fullmatch('[0-9a-f]{64}', expected_sha), 'closed actual receipt namespace')
    receipt_pin = pin(receipt)
    require(receipt_pin['sha256'] == expected_sha, 'caller-authenticated actual completion')
    value = json.loads(receipt.read_bytes())
    require(value['schema'] == 'ferric-p228-projection-ar4-shared-host-cpu-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['source_unchanged'] is True and value['tests_passed'] == 883 and value['tests_ignored'] == 4
            and len(value['phases']) == 63 and len(value['raw']) == 322, 'actual complete CPU883 cohort')
    require(all(type(v['exit_code']) is int and v['exit_code'] == 0 and v['reason'] is None
                and v['group_absent'] is True for v in value['phases'].values()), '63 natural leaves')
    require(sum(v['passed'] for v in value['tests'].values()) == 883
            and sum(v['ignored'] for v in value['tests'].values()) == 4, 'named test census')
    fixed = {receipt: receipt_pin, PROPOSAL: pin(PROPOSAL)}
    fixed[Path(__file__).resolve(strict=True)] = pin(Path(__file__).resolve(strict=True))
    require(fixed[PROPOSAL] == value['proposal'] and fixed[PROPOSAL]['sha256'] == PROPOSAL_SHA,
            'exact reviewed shared-host source proposal')
    controller = E / 'p228-projection-ar4-shared-host-cpu-v1/run.py'
    fixed[controller] = pin(controller)
    require(fixed[controller] == value['controller'] and fixed[controller]['sha256'] == CONTROLLER_SHA,
            'actual selected CPU controller')
    selected = {receipt: receipt_pin}
    for name, record in value['raw'].items():
        require(Path(name).name == name and record['path'] == str(case / name)
                and pin(case / name) == record, 'all 322 exact raw records')
        selected[case / name] = record
    require(all(json.loads((case / (name + '-result.json')).read_bytes()) == phase
                for name, phase in value['phases'].items()), 'retained natural result bodies')
    tested = json.loads((case / 'sources-after.json').read_bytes())
    require(tested == json.loads((case / 'sources-before.json').read_bytes()) and len(tested) == 7003,
            'complete formatted source before/after equality')
    require(set(value['binaries']) == BINARIES, 'four selected production executable roles')
    for name, artifact in value['binaries'].items():
        record = artifact['binary']; path = Path(record['path'])
        role = 'worker' if name == 'ferric-tp-peer-finite-engineering-worker-v1' else 'parent'
        require(path == case / 'target' / role / 'debug' / name and pin(path) == record
                and os.access(path, os.X_OK), 'all four actual ELF bodies, omitted from archive')
        fixed[path] = record
    rows = json.loads(PROPOSAL.read_bytes())['files']
    require(len(rows) == len({row['path'] for row in rows}) == 14, 'fourteen formatted overlay bodies')
    for row in rows:
        path = case / 'sources/ferric' / row['path']; record = pin(path)
        require(path.is_relative_to(case / 'sources/ferric') and '..' not in Path(row['path']).parts
                and {key: record[key] for key in ('bytes', 'sha256')} == tested['ferric/' + row['path']],
                'actual qualified formatted overlay')
        selected[path] = record
    require(len(selected) == 337 and sum(row['bytes'] for row in selected.values()) <= 256 << 20,
            'bounded metadata/source-only courier')
    archive = E / (case.name + '-retained.tar.gz')
    with tarfile.open(archive, 'x:gz') as output:
        for path, record in sorted(selected.items()):
            require(pin(path) == record, 'pre-export body equality')
            output.add(path, arcname=str(path.relative_to(E)), recursive=False)
    require(all(pin(path) == record for path, record in (selected | fixed).items()), 'all input postchecks')
    print(json.dumps(dict(archive=pin(archive), members=len(selected),
        bytes=sum(row['bytes'] for row in selected.values()), verified_elf_bodies=4, exported_elf_bodies=0)), flush=True)


if __name__ == '__main__':
    main()
