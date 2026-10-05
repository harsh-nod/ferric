"""Bounded original-path CPU883 runtime courier; no executable or tested-code runs."""
import hashlib
import json
import os
from pathlib import Path
import resource
import stat
import sys
import tarfile
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CASE = E / 'projection-ar4-shared-host-cpu-v228-v1'
ARCHIVE = E / 'projection-ar4-shared-host-runtime-v228-v1.tar.gz'
SELF = E / 'p228-projection-ar4-shared-host-courier-v1/run.py'
COMPLETE = dict(path=str(CASE / 'complete.json'), bytes=466118,
    sha256='deb2aeacded08c336d1fb5a1638cd2accc2b65ec65ffa3e90177bd5e61146d74')
CONTROLLER = E / 'p228-projection-ar4-shared-host-cpu-v1/run.py'
CONTROLLER_SHA = '1f9fc61f35cc2d8367592ecf29ff7ea48e6d6250556a72940abe3acb028a41c5'
PROPOSAL = E / 'p228-projection-ar4-shared-host-v1/source-manifest.json'
PROPOSAL_SHA = 'faaece5ab1afea85b7b6f03b02d772847889565a0ae70922180d6f62a1770461'
BASE_SHA = '1d173129c684afe5bcdc009ec939f8e4f0371bad8a4edecdf62b4548cb04f920'
SOURCE_SHA = 'ed98abb445b2dc3209fcd575bfb25e817b86b7bce0dbdb5f299984067149f3b4'
PLAIN = 'ferric-qwen3-finite-projection-residual-decode-engineering'
DEFAULT = 'ferric-qwen3-finite-projection-residual-decode-host-engineering'
SHARED = 'ferric-qwen3-finite-projection-residual-decode-shared-host-engineering'
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
ELFS = {
    PLAIN: (13925384, 'afab676b199de3bb89c49e66862316609389eea50e0df2870841b1ca97ac1345'),
    DEFAULT: (13947904, 'a5f8323b87f475f28dea91120742204e9e84b231257a42699967c08b35ea4ace'),
    SHARED: (13948728, '8214d2f3c2237d243c5ad97c109fdda3558d22caff3ed17fa0805b54eae27da5'),
    WORKER: (5176608, '3d36a6a53a2ab952a57b7e4784c8b2607348c45420b77b9a34e9483bef9218a8'),
}
RAW = ('sources-base.json', 'sources-unformatted.json', 'sources-before.json',
       'sources-after.json', 'worker-build-stdout', 'parent-builds-stdout')
START = time.monotonic()


def require(ok, message):
    if not ok:
        raise RuntimeError(message)
    if time.monotonic() - START > 300:
        raise RuntimeError('bounded data-only courier wall time')


def stamp(s):
    return tuple(getattr(s, k) for k in ('st_dev', 'st_ino', 'st_mode', 'st_uid',
        'st_gid', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns'))


def read(path, retain=False, executable=False):
    require(path.is_relative_to(E) and path.resolve(strict=True) == path, 'canonical original E path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == 9661
        and before.st_nlink >= 1 and not before.st_mode & 0o7000
        and 0 < before.st_size <= 64 << 20, 'bounded ordinary owned input; Cargo hardlinks allowed')
    if executable:
        require(before.st_mode & 0o100 and os.access(path, os.X_OK), 'selected executable owner mode')
    digest, size, kept, header = hashlib.sha256(), 0, bytearray(), b''
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        require(stamp(before) == stamp(os.fstat(stream.fileno())), 'opened identity')
        while block := stream.read(1 << 20):
            size += len(block); require(size <= 64 << 20, 'growing file bound')
            if not header: header = block[:20]
            digest.update(block)
            if retain: kept.extend(block)
        require(stamp(before) == stamp(os.fstat(stream.fileno())), 'stable opened body')
    require(size == before.st_size and stamp(before) == stamp(path.lstat()), 'stable named input')
    if executable:
        require(len(header) == 20 and header[:6] == b'\x7fELF\x02\x01'
            and header[18:20] == b'\x3e\x00', 'ELF64 little-endian x86_64 product')
    return dict(path=str(path), bytes=size, sha256=digest.hexdigest()), bytes(kept)


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key'); value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def main():
    require(len(sys.argv) == 1 and not sys.flags.optimize and sys.dont_write_bytecode
        and 'PYTHONOPTIMIZE' not in os.environ and os.getuid() == os.geteuid() == 9661
        and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'root-selected bounded ASROCK CPU-only courier')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 180),
                      (resource.RLIMIT_FSIZE, 128 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (cap, cap))
    os.umask(0o077)
    require(Path(__file__).resolve(strict=True) == SELF and not os.path.lexists(ARCHIVE),
            'selected source and fresh exclusive archive')
    verified = {}; selected = {}; executables = {}

    def take(path, expected=None, retain=False, executable=False, export=True):
        record, raw = read(path, retain, executable)
        require(expected is None or record == expected, 'exact actual input extent and SHA')
        require(verified.setdefault(path, record) == record, 'stable repeated input')
        if export: selected[path] = record
        return record, raw

    self_pin, _ = take(SELF, export=False)
    _, raw = take(CASE / 'complete.json', COMPLETE, True); value = parse(raw)
    require(value['schema'] == 'ferric-p228-projection-ar4-shared-host-cpu-result-v1'
        and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['empty_initial_target'] is True
        and len(value['phases']) == 63 and len(value['raw']) == 322
        and value['tests_passed'] == 883 and value['tests_ignored'] == 4
        and value['tests']['worker-tests']['summaries'] == [[519, 0, 4], [13, 0, 0]],
        'actual completed scoped CPU883 receipt')
    require(all(p['exit_code'] == 0 and p['reason'] is None and p['group_absent'] is True
        for p in value['phases'].values()) and value['historical_runtime_tests_not_repeated'] == 208,
        'all63 natural leaves; no new runtime208 claim')
    require(all(value[k] is False for k in ('gpu_execution', 'numerical_acceptance',
        'performance_claim', 'production_authority', 'full_cpu1037_cohort_requalified', 'compiler_requalified')),
        'unchanged qualification scope')
    for path, key, digest in ((CONTROLLER, 'controller', CONTROLLER_SHA), (PROPOSAL, 'proposal', PROPOSAL_SHA)):
        record, raw = take(path, value[key], retain=True)
        require(record['sha256'] == digest, 'actual selected controller/source proposal')
        if key == 'proposal': proposal = parse(raw)
    bodies = {}
    for name in RAW:
        _, bodies[name] = take(CASE / name, value['raw'][name], True)
    maps = {k: parse(bodies['sources-' + k + '.json']) for k in ('base', 'unformatted', 'before', 'after')}
    require(value['raw']['sources-base.json']['sha256'] == BASE_SHA and len(maps['base']) == 6999
        and value['raw']['sources-before.json']['sha256'] == value['raw']['sources-after.json']['sha256'] == SOURCE_SHA
        and maps['before'] == maps['after'] and len(maps['after']) == 7003, 'actual exact qualified source maps')
    expected = dict(maps['base']); changed = set()
    require(proposal['base_cpu'] == value['prior_completion'] and len(proposal['files']) == 14, 'exact source generation')
    for row in proposal['files']:
        key = 'ferric/' + row['path']
        require((key not in expected) if row['before'] is None else expected.get(key) == row['before'], 'source preimage')
        expected[key] = row['after']
        if key.endswith('.rs'): changed.add(key)
    require(maps['unformatted'] == expected and len(changed) == 13 and set(expected) == set(maps['before'])
        and all(maps['before'][k] == v for k, v in expected.items() if k not in changed), 'only reviewed formatting scope')
    require(set(value['binaries']) == set(ELFS), 'all four actual artifact identities')
    for phase, names in (('worker-build', [WORKER]), ('parent-builds', [PLAIN, DEFAULT, SHARED])):
        raw = bodies[phase + '-stdout']; rows = [parse(line) for line in raw.splitlines() if line.strip()]
        artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        require(hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256']
            and [r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True]
            and len(artifacts) == len(names) and {r['target']['name'] for r in artifacts} == set(names), 'actual successful build stream')
        for name in names:
            entry = value['binaries'][name]; artifact = entry['artifact']; role = 'worker' if name == WORKER else 'parent'
            path = CASE / 'target' / role / 'debug' / name
            require(artifacts.count(artifact) == 1 and artifact['profile']['test'] is False
                and artifact['target']['kind'] == artifact['target']['crate_types'] == ['bin']
                and artifact['target']['name'] == name and artifact['executable'] == str(path)
                and artifact['filenames'] == [str(path)]
                and entry['binary'] == dict(path=str(path), bytes=ELFS[name][0], sha256=ELFS[name][1]), 'exact actual non-test product')
            record, _ = take(path, entry['binary'], executable=True, export=name != PLAIN)
            executables[name] = record
    require(len(selected) == 12 and sum(r['bytes'] for r in selected.values()) <= 64 << 20,
            'only three ELF + seven CPU + two controller/proposal bodies')
    with tarfile.open(ARCHIVE, 'x:gz', dereference=True) as output:
        for path, record in sorted(selected.items()):
            require(read(path)[0] == record, 'pre-archive body equality')
            output.add(path, arcname=str(path.relative_to(E)), recursive=False)
    expected_names = {str(path.relative_to(E)): record for path, record in selected.items()}
    with tarfile.open(ARCHIVE, 'r:gz') as source:
        rows = source.getmembers()
        require(len(rows) == 12 and {r.name for r in rows} == set(expected_names)
            and all(r.isfile() for r in rows), 'closed twelve ordinary archive members, no link records')
        for row in rows:
            expected = expected_names[row.name]; digest = hashlib.sha256(); size = 0
            with source.extractfile(row) as stream:
                while block := stream.read(1 << 20):
                    size += len(block); require(size <= expected['bytes'], 'archive member extent'); digest.update(block)
            require(size == row.size == expected['bytes'] and digest.hexdigest() == expected['sha256'], 'archive body replay')
            if str(E / row.name) in {p['path'] for n, p in executables.items() if n != PLAIN}:
                require(row.mode & 0o100, 'transport preserves owner executable bit')
    executable_paths = {Path(record['path']) for record in executables.values()}
    require(all(read(path, executable=path in executable_paths)[0] == record
        for path, record in verified.items()), 'all actual inputs and executable modes rechecked after archive')
    print(json.dumps(dict(schema='ferric-p228-projection-ar4-shared-host-runtime-courier-v1',
        archive=read(ARCHIVE)[0], members=12, original_pins=list(selected.values()), controller=self_pin,
        verified_executables=executables, exported_executables=[DEFAULT, SHARED, WORKER],
        omitted_plain_parent=executables[PLAIN], input_postchecks_passed=True, archive_bodies_rechecked=True,
        elf_audits_performed=False, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False), sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
