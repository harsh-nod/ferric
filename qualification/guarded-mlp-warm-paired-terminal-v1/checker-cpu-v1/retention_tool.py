"""Export/retain the original twenty-test result as data; never import or rerun it."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import resource
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-warm-paired-terminal-checker-cpu-v228-v1'
SOURCE = E / 'guarded-mlp-warm-paired-terminal-checker-source-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
DEST = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-warm-paired-terminal-v1/checker-cpu-v1')
BASENAME = 'guarded-mlp-warm-paired-terminal-checker-evidence-v228-v1.tar.gz'
TERMINAL = dict(bytes=4147, sha256='ca0ddb8f614a17e09c787d75f3a5d0d912456642be59b179cc57ea766fff6adc')
CONTROLLER = dict(bytes=9393, sha256='4137a9b689c0865ee7d8996bccd4a8a3fb50ad3b731b1342da2cb2fbbc8d912b')
SOURCES = {
    'run_model_gpu.py': dict(bytes=43046, sha256='a996fbca701ed968a49f4dc88a62c808be76b150311d79e7dd4d098ca804a13b'),
    'test_census.py': dict(bytes=4375, sha256='d4f19ae2f4bcf9f68464a7b8951b82368183cec6db998a0ed55449d2349e7f64'),
    'test_comparison.py': dict(bytes=3753, sha256='daead3eef98b1368b705b17a6894c2c060ef69ca154788fbd8f01cbf9e888093'),
    'test_terminal.py': dict(bytes=8856, sha256='65cd07e2542b67aa8bcee20b17b9bc8e9c70a3a7a8940ab9f27576cca0f1ec47'),
    'validate_observation.py': dict(bytes=17983, sha256='7224877439ec966254d7604317f1f47b5d6345af698e47e332f6cb36c3abc19b'),
}
MAX_FILE, MAX_TOTAL, MEMBERS = 1 << 20, 4 << 20, 11


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, limit=MAX_FILE):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= limit, 'ordinary bounded body')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'body changed before read')
        raw = stream.read(limit + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'body changed during read')
    require(stamp(path.lstat()) == stamp(before) and len(raw) == before.st_size, 'body changed after read')
    return raw


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def names_in(raw, module):
    names = []
    for node in ast.parse(raw).body:
        if isinstance(node, ast.ClassDef):
            names += [module + '.' + node.name + '.' + method.name for method in node.body
                      if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')]
    return names


def validate(bodies):
    expected_names = {'complete.json', 'stdout', 'stderr', 'retention_tool.py', 'source/run_checker_cpu.py'}
    expected_names |= {'source/' + name for name in SOURCES}
    require(set(bodies) == expected_names and len(bodies) == MEMBERS - 1, 'closed ten original/helper bodies')
    require(pin(bodies['complete.json']) == TERMINAL
            and pin(bodies['source/run_checker_cpu.py']) == CONTROLLER, 'actual result/controller pins')
    for name, expected in SOURCES.items():
        require(pin(bodies['source/' + name]) == expected, 'original tested source ' + name)
    c = parse(bodies['complete.json'])
    require(set(c) == set('controller elapsed_seconds errors expected_test_names failure failures gpu_execution '
            'numerical_acceptance passed performance_claim postcheck_errors schema sources stderr stdout '
            'subprocesses_started synthetic_evidence_only tests_run'.split()), 'closed actual checker result')
    require(c['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-checker-cpu-v1'
            and c['passed'] is True and c['failure'] is None and c['postcheck_errors'] == []
            and c['controller'] == CONTROLLER
            and c['sources'] == {str(SOURCE / name): value for name, value in SOURCES.items()}
            and type(c['tests_run']) is int and c['tests_run'] == 20
            and type(c['errors']) is int and c['errors'] == 0
            and type(c['failures']) is int and c['failures'] == 0
            and type(c['subprocesses_started']) is int and c['subprocesses_started'] == 0
            and c['synthetic_evidence_only'] is True
            and all(c[k] is False for k in ('gpu_execution', 'numerical_acceptance', 'performance_claim')),
            'actual20 pure successful tests with no child or GPU')
    tests = sorted(name for module in ('test_census', 'test_comparison', 'test_terminal')
                   for name in names_in(bodies['source/' + module + '.py'], module))
    require(len(tests) == len(set(tests)) == 20 and c['expected_test_names'] == tests, 'exact source-declared20 names')
    constants = {}
    for node in ast.parse(bodies['source/run_checker_cpu.py']).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            if node.targets[0].id in ('SOURCES', 'TEST_NAMES'):
                constants[node.targets[0].id] = ast.literal_eval(node.value)
    require(list(constants['TEST_NAMES']) == tests and
            {file: dict(bytes=size, sha256=sha) for file, size, sha in constants['SOURCES'].values()} == SOURCES,
            'actual checker source roster and explicit source pins')
    for name in ('stdout', 'stderr'):
        require(c[name] == dict(path=str(ROOT / name), **pin(bodies[name])), 'raw test stream pin')
    require(bodies['stdout'] == b'', 'exact empty test stdout')
    stderr = bodies['stderr'].decode('utf-8')
    rows = re.findall(r'^test_\w+ \((test_\w+\.\w+\.test_\w+)\) \.\.\. ok$', stderr, re.M)
    require(len(rows) == len(set(rows)) == 20 and sorted(rows) == tests
            and re.search(r'\nRan 20 tests in [0-9]+\.[0-9]+s\n\nOK\n\Z', stderr),
            'twenty original named passes and clean unittest summary')
    return dict(schema='ferric-warm-paired-terminal-checker-data-audit-v1',
        original_terminal=TERMINAL, original_controller=CONTROLLER,
        tests=tests, original_tests_passed=20, original_tests_failed=0,
        original_test_sources=6, original_output_files=3,
        original_test_run_repeated=False, new_project_execution=False,
        subprocesses_started=0, gpu_execution=False, numerical_acceptance=False, performance_claim=False)


def export():
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'original MI350 account')
    require(ROOT.resolve(strict=True) == ROOT and SOURCE.resolve(strict=True) == SOURCE, 'original roots')
    paths = {'source/' + name: SOURCE / name for name in SOURCES}
    paths['source/run_checker_cpu.py'] = SOURCE / 'run_checker_cpu.py'
    paths.update({name: ROOT / name for name in ('complete.json', 'stdout', 'stderr')})
    paths['retention_tool.py'] = Path(__file__).resolve()
    require({p.name for p in SOURCE.iterdir()} == set(SOURCES) | {'run_checker_cpu.py'}
            and {p.name for p in ROOT.iterdir()} == {'complete.json', 'stdout', 'stderr'}, 'all original source/result files')
    bodies = {name: read(path) for name, path in paths.items()}
    audit = validate(bodies)
    manifest = dict(schema='ferric-warm-paired-terminal-checker-export-v1',
        files={name: pin(raw) for name, raw in sorted(bodies.items())}, audit=audit,
        original_files=9, selected_files=10, members=MEMBERS,
        source_posthashes=True, test_rerun=False, gpu_execution=False, numerical_acceptance=False, performance_claim=False)
    archive_bodies = dict(bodies, **{'manifest.json': encoded(manifest)})
    require(sum(map(len, archive_bodies.values())) <= MAX_TOTAL, 'bounded expanded archive')
    output = E / BASENAME
    require(E.resolve(strict=True) == E and not os.path.lexists(output), 'fresh original evidence archive')
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(archive_bodies.items()):
                item = tarfile.TarInfo(name)
                item.size, item.mode, item.mtime = len(raw), 0o644, 0
                tar.addfile(item, io.BytesIO(raw))
        stream.flush(); os.fsync(stream.fileno())
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'all original source/result posthashes')
    print(json.dumps(dict(archive=dict(path=str(output), **pin(read(output, MAX_TOTAL))), members=MEMBERS,
        original_tests_passed=20, test_rerun=False, gpu_execution=False), sort_keys=True))


def retain(archive_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha), 'observed archive hash')
    require(DEST.parent.resolve(strict=True) == DEST.parent and not os.path.lexists(DEST), 'fresh canonical capsule')
    archive = W / BASENAME
    raw = read(archive, MAX_TOTAL)
    require(pin(raw)['sha256'] == archive_sha, 'observed original archive')
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for row in tar:
            name = PurePosixPath(row.name)
            require(row.isfile() and not row.pax_headers and str(name) == row.name
                    and not name.is_absolute() and '..' not in name.parts and '\\' not in row.name
                    and row.name not in bodies and len(bodies) < MEMBERS
                    and 0 <= row.size <= MAX_FILE, 'closed ordinary archive member')
            total += row.size
            require(total <= MAX_TOTAL, 'expanded archive limit')
            body = tar.extractfile(row).read(row.size + 1)
            require(len(body) == row.size, 'whole original archive body')
            bodies[row.name] = body
    require(len(bodies) == MEMBERS, 'exact eleven archive members')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    require(bodies['retention_tool.py'] == read(Path(__file__).resolve()), 'same reviewed export/retain tool')
    audit = validate(bodies)
    require(manifest == dict(schema='ferric-warm-paired-terminal-checker-export-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())}, audit=audit,
        original_files=9, selected_files=10, members=MEMBERS,
        source_posthashes=True, test_rerun=False, gpu_execution=False, numerical_acceptance=False, performance_claim=False),
        'entire original manifest and raw data join')
    require(read(archive, MAX_TOTAL) == raw, 'archive stable before retention')
    bodies['manifest.json'] = manifest_raw
    DEST.mkdir(mode=0o755)
    for name, body in sorted(bodies.items()):
        path = DEST / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
    require(all(read(DEST / name) == body for name, body in bodies.items())
            and read(archive, MAX_TOTAL) == raw, 'all retained original bytes')
    report = dict(schema='ferric-warm-paired-terminal-checker-retention-v1', archive=pin(raw),
        files={name: pin(body) for name, body in sorted(bodies.items())}, audit=audit,
        original_archive_members=MEMBERS, original_receipt_unchanged=True,
        external_inputs_rehashed_locally=False, project_execution=False, gpu_execution=False)
    with (DEST / 'retention.json').open('xb') as stream:
        stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(DEST), original_archive_members=MEMBERS, original_tests_passed=20)))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -I -B evidence.py export | retain ACTUAL_ARCHIVE_SHA')
    resource.setrlimit(resource.RLIMIT_AS, (128 << 20, 128 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_TOTAL, MAX_TOTAL))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    def interrupted(number, _frame):
        raise RuntimeError('pure evidence signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, interrupted)
    signal.alarm(30)
    try:
        if len(sys.argv) == 2 and sys.argv[1] == 'export':
            export()
        else:
            require(len(sys.argv) == 3 and sys.argv[1] == 'retain', 'closed data-only CLI')
            retain(sys.argv[2])
    finally:
        signal.alarm(0)
