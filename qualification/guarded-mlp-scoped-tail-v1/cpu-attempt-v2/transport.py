"""Coupled CPU transport; final controller binding is required before staging."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import tarfile

P = Path(__file__).resolve().parent
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-scoped-tail-v1')
Q = Path('/home/harsh/ferric-p227-integration/qualification')
LOCAL_FERRIC = Q.parent
LOCAL_BASE = Q / 'guarded-mlp-scoped-tail-v1/cpu-v1'
LOCAL_RUNTIME = Path('/home/harsh/fe2o3-p228-runtime')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-scoped-tail-cpu-v228-v1'
ROOT = E / 'guarded-mlp-scoped-tail-cpu-v228-v2'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-scoped-tail-input-v228-v2.tar.gz'
CONTROLLER_PIN = {'bytes': 45123, 'sha256': 'e52b6b4d0a6a3321599840550764dbf5bb4ed760f7c4586701f744a5e540a3cf'}
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=41316, sha256='4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a')
CACHE_PIN = {"bytes":12782,"sha256":"3c7241c071d87dcff7ce6a4f58720f6f83705a8629734fe543f9b8f6dd2c335d"}
CACHE_MANIFEST = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
PROPOSALS = {"worker-proposal.json":[27229,"96463bf9a70d6020e28c2d44260f589c38e7f61229f8c8e77f28242f1893ca71"],"runtime-proposal.json":[10858,"71da4afd43eb9f7de8fa1cac47cf8e9b2f0ececc86cfb98877b94678d500df83"],"worker-repair.json":[4324,"8e5ffe3f2b4d87b9967ef993e31385c42461dbdd92a49c1f63a2578e1b8e6128"]}
SELECTOR_SCHEMA = "ferric-readiness40-bank-scoped-census-tail-worker-source-v4"
SELECTOR_ROWS = 13
WORKER_FILES = 228
TOTAL_SOURCES = 1059
TOTAL_MEMBERS = 17
BASE_COMPLETE = {"bytes":2449865,"sha256":"a2e3d99f94b5b01223f42f3d8917c2cf6a1fc25f9227d4c341bc2b9a7cc9285c"}
BASE_SOURCES = {"bytes":424416,"sha256":"a2a74ed023e9b2aac224621d37f328d097838c317c901a361c1496064e162ec4"}
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = 'scoped-tail-coupled-v2'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(rows):
        value = {}
        for name, row in rows:
            require(name not in value, 'duplicate JSON key')
            value[name] = row
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 64 << 20,
                'bounded ordinary input')
        raw = stream.read((64 << 20) + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'input drift')
    return raw


def relative(name):
    return type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute() \
        and '..' not in Path(name).parts and name not in ('', '.')


def compact(rows):
    return {name: {key: row[key] for key in ('bytes', 'sha256')} for name, row in rows.items()}


def closed_pin(row):
    return type(row) is dict and set(row) == {'bytes', 'sha256'} and type(row['bytes']) is int \
        and 0 <= row['bytes'] <= 16 << 20 and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256'])


def contract(bodies):
    require(CONTROLLER_PIN is not None, 'reviewed controller binding required')
    require(pin(bodies['run_cpu.py']) == CONTROLLER_PIN
            and pin(bodies['worker_support.py']) == SUPPORT_PIN and pin(bodies['supervisor.py']) == SUPERVISOR_PIN
            and pin(bodies['cache.py']) == CACHE_PIN, 'four exact reviewed helper bodies')
    tree = ast.parse(bodies['run_cpu.py'])
    history = [node for node in tree.body if isinstance(node, ast.Assign)
               and any(isinstance(target, ast.Name) and target.id == 'HISTORY' for target in node.targets)]
    require(len(history) == 1, 'single literal lineage declaration')
    history = ast.literal_eval(history[0].value)
    require(len(history) == 8, 'seven current raw lineage bodies plus original legacy docs')
    for name, (size, digest) in history.items():
        require(pin(bodies['inputs/' + name]) == dict(bytes=size, sha256=digest), 'actual direct lineage pin')
    for name, (size, digest) in PROPOSALS.items():
        require(pin(bodies['inputs/' + name]) == dict(bytes=size, sha256=digest),
                'exact original source proposal or repair')
    base = parse(bodies['inputs/baseline-complete.json'])
    source_map = parse(bodies['inputs/baseline-sources.json'])
    require(pin(bodies['inputs/baseline-complete.json']) == BASE_COMPLETE
            and pin(bodies['inputs/baseline-sources.json']) == BASE_SOURCES
            and base['schema'] == 'ferric-guarded-mlp-scoped-tail-cpu-v1'
            and base['source_generation'] == 'scoped-tail-coupled-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == source_map
            and base['source_unchanged'] is True and len(source_map) == 1059
            and base['gpu_execution'] is False and base['runtime_source_changed'] is True
            and base['scoped_tail_runtime_source_added'] is True
            and len(base['phases']) == 27 and all(row['exit_code'] == 0
                and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                for row in base['phases']), 'actual immediate qualified Tail V1 base')
    old = {name: row for name, row in compact(source_map).items()
           if name.startswith(('fe2o3/', WORKER_PREFIX))}
    require(len(old) == 1055 and sum(name.startswith('fe2o3/') for name in old) == 827
            and sum(name.startswith(WORKER_PREFIX) for name in old) == 228
            and all(relative(name) and closed_pin(row) for name, row in old.items()),
            'closed actual827-runtime228-worker base')
    proposal = parse(bodies['inputs/worker-proposal.json'])
    runtime = parse(bodies['inputs/runtime-proposal.json'])
    repair = parse(bodies['inputs/worker-repair.json'])
    require(proposal['schema'] == SELECTOR_SCHEMA and len(proposal['files']) == SELECTOR_ROWS
            and pin(bodies['inputs/worker-proposal.json']) == compact({'x': base['readset']['worker-proposal.json']})['x']
            and pin(bodies['inputs/runtime-proposal.json']) == compact({'x': base['readset']['runtime-proposal.json']})['x']
            and compact({'x': proposal['requires']['runtime']})['x'] == pin(bodies['inputs/runtime-proposal.json'])
            and compact({'x': proposal['requires']['interface']})['x'] == compact({'x': runtime['interface']})['x'],
            'original source proposals retained as actual baseline lineage')
    require(repair['schema'] == 'ferric-scoped-tail-shared-test-repair-v2'
            and repair['source_generation'] == 'scoped-tail-shared-test-repair-v2'
            and repair['base']['terminal'] == BASE_COMPLETE and repair['base']['sources'] == BASE_SOURCES
            and repair['requires']['worker-v1/source-manifest.json'] == pin(bodies['inputs/worker-proposal.json'])
            and repair['requires']['runtime-v1/source-manifest.json'] == pin(bodies['inputs/runtime-proposal.json'])
            and all(repair[key] is False for key in ('production_changed', 'runtime_changed',
                'test_assertions_changed', 'tests_executed', 'canonical_changed', 'native_execution',
                'numerical_acceptance', 'performance_claim'))
            and repair['new_test_names'] == [] and len(repair['files']) == 1,
            'one unexecuted test-only repair')
    row = repair['files'][0]
    name = 'ferric/' + row['path']
    require(row['repository'] == 'ferric'
            and name == WORKER_PREFIX + 'src/guarded_mlp_long_sequence_v2_tests.rs'
            and old[name] == row['before'] and closed_pin(row['after']),
            'exact sole test-file preimage')
    raw = bodies[name]
    prior, after = (repair['replacement'][key].encode() for key in ('before', 'after'))
    require(raw.count(after) == 1 and prior not in raw and pin(raw) == row['after']
            and pin(raw.replace(after, prior, 1)) == row['before'],
            'exact repair reversal to actual qualified bytes')
    names = re.findall(r'#\[test\]\s*fn\s+(\w+)\s*\(', raw.decode())
    require(names == repair['unchanged_test_names'] and len(names) == len(set(names)) == 9,
            'unchanged nine shared test names')
    files, overlay = dict(old), {name}
    files[name] = row['after']
    for name in ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py'):
        files[name] = pin(bodies[name])
    require(len(files) == TOTAL_SOURCES and sum(row['bytes'] for row in files.values()) <= 64 << 20
            and sum(name.startswith('fe2o3/') for name in files) == 827
            and sum(name.startswith(WORKER_PREFIX) for name in files) == WORKER_FILES,
            'closed complete composed runtime and worker map')
    expected = overlay | {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py',
        *('inputs/' + name for name in history), *('inputs/' + name for name in PROPOSALS)}
    require(set(bodies) == expected and len(bodies) + 1 == TOTAL_MEMBERS,
            'exact one-overlay, four-helper, eleven-lineage package')
    inputs = dict(schema='ferric-guarded-mlp-scoped-tail-cpu-input-v1',
        source_generation=GENERATION, files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        lineage={name.removeprefix('inputs/'): pin(body) for name, body in sorted(bodies.items())
                 if name.startswith('inputs/')}, overlay=sorted(overlay), cache_manifest=CACHE_MANIFEST)
    return inputs, old

def pack():
    require(CONTROLLER_PIN is not None, 'reviewed controller binding required')
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'input-manifest.json'), 'fresh package outputs')
    source_paths = {
        'run_cpu.py': P / 'run_cpu.py', 'cache.py': P / 'cache.py',
        'supervisor.py': LOCAL_BASE / 'supervisor.py',
        'worker_support.py': LOCAL_BASE / 'worker_support.py',
        'inputs/worker-proposal.json': PROPOSAL_ROOT / 'worker-v1/source-manifest.json',
        'inputs/runtime-proposal.json': PROPOSAL_ROOT / 'runtime-v1/source-manifest.json',
        'inputs/worker-repair.json': PROPOSAL_ROOT / 'worker-fix-v2/source-manifest.json',
        'inputs/baseline-complete.json': LOCAL_BASE / 'evidence/complete.json',
        'inputs/baseline-sources.json': LOCAL_BASE / 'evidence/sources-after.json',
        'inputs/runtime-tests.stdout': LOCAL_BASE / 'evidence/kfd-tests.stdout',
        'inputs/runtime-list.stdout': LOCAL_BASE / 'evidence/kfd-list.stdout',
        'inputs/worker-tests.stdout': LOCAL_BASE / 'evidence/worker-tests.stdout',
        'inputs/worker-list.stdout': LOCAL_BASE / 'evidence/worker-list.stdout',
        'inputs/runtime-docs.stdout': LOCAL_BASE / 'evidence/interface-doc-tests.stdout',
        'inputs/legacy-docs.stdout': LOCAL_BASE / 'inputs/legacy-docs.stdout',
    }
    repair = parse(read(source_paths['inputs/worker-repair.json']))
    for row in repair['files']:
        source_paths['ferric/' + row['path']] = PROPOSAL_ROOT / 'worker-fix-v2/ferric' / row['path']
    bodies = {name: read(path) for name, path in source_paths.items()}
    inputs, old = contract(bodies)
    require(pin(read(LOCAL_BASE / 'cargo-cache-manifest.json')) == CACHE_MANIFEST,
            'actual immutable two-lock cache manifest')
    canonical = {}
    for name, expected in old.items():
        if name.startswith(WORKER_PREFIX):
            path = LOCAL_FERRIC / name.removeprefix('ferric/')
        elif name in ('fe2o3/Cargo.toml', 'fe2o3/Cargo.lock'):
            continue
        elif name == 'fe2o3/Cargo.toml.original':
            path = LOCAL_RUNTIME / 'Cargo.toml'
        elif name == 'fe2o3/Cargo.lock.input':
            path = LOCAL_RUNTIME / 'Cargo.lock'
        else:
            path = LOCAL_RUNTIME / name.removeprefix('fe2o3/')
        require(pin(read(path)) == expected, 'actual canonical source/original-Cargo identity')
        canonical[path] = expected
    require(len(canonical) == 823 + 228 + 2
            and not os.path.lexists(LOCAL_RUNTIME / 'Cargo.toml.original')
            and not os.path.lexists(LOCAL_RUNTIME / 'Cargo.lock.input'),
            'fixture Cargo bodies excluded and original aliases mapped explicitly')
    require(all(read(path) == bodies[name] for name, path in source_paths.items())
            and all(pin(read(path)) == expected for path, expected in canonical.items()), 'package input posthash')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == TOTAL_MEMBERS and sum(map(len, bodies.values())) <= 16 << 20, 'bounded source transport')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(raw_input),
                         members=TOTAL_MEMBERS, source_files=TOTAL_SOURCES, runtime_files=827, worker_files=WORKER_FILES), sort_keys=True))


def stage(archive_sha, input_sha):
    require(CONTROLLER_PIN is not None, 'reviewed controller binding required')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE and not os.path.lexists(ROOT),
            'fresh exclusive qualification namespace')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual archive hash')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == TOTAL_MEMBERS
                and all(m.isfile() and relative(m.name) and not m.pax_headers and 0 <= m.size <= 4 << 20 for m in members)
                and sum(m.size for m in members) <= 16 << 20, 'unique ordinary bounded USTAR input')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'source member bytes')
    raw_input = bodies.pop('input-manifest.json')
    require(pin(raw_input)['sha256'] == input_sha, 'actual input hash')
    expected, old = contract(bodies)
    require(parse(raw_input) == expected, 'entire source input contract reconstructed')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/baseline-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/baseline-sources.json'], 'live immutable qualified base receipts')
    base_paths = []
    for subtree in ('fe2o3', 'ferric'):
        for directory, dirs, names in os.walk(BASE / subtree, followlinks=False,
                onerror=lambda error: (_ for _ in ()).throw(error)):
            require(all(not (Path(directory) / name).is_symlink() and name not in ('.git', 'target') for name in dirs),
                    'no source directory alias or cache')
            base_paths.extend(Path(directory) / name for name in names)
    require({str(path.relative_to(BASE)) for path in base_paths} == set(old), 'exact live qualified source roster')
    for name, row in old.items():
        require(pin(read(BASE / name)) == row, 'qualified source preimage: ' + name)
    ROOT.mkdir(mode=0o700)
    def write(name, raw):
        require(relative(name), 'ordinary destination')
        path = ROOT / name
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        require(path.parent.resolve(strict=True) == path.parent, 'ordinary destination parent')
        with path.open('xb') as stream:
            stream.write(raw)
    for name, row in old.items():
        if name not in bodies:
            raw = read(BASE / name)
            require(pin(raw) == row, 'source stable while copied')
            write(name, raw)
    for name, raw in sorted(bodies.items()):
        write(name, raw)
    write('input-manifest.json', raw_input)
    require(all(pin(read(ROOT / name)) == row for name, row in expected['files'].items()), 'entire staged source map')
    require(all(pin(read(BASE / name)) == row for name, row in old.items())
            and read(BASE / 'evidence/complete.json') == bodies['inputs/baseline-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/baseline-sources.json'], 'base posthash, no source changes')
    receipt = dict(schema='ferric-guarded-mlp-scoped-tail-stage-v1', passed=True, root=str(ROOT),
        archive=pin(archive_raw), input=pin(raw_input), source_files=TOTAL_SOURCES, runtime_files=827, worker_files=WORKER_FILES,
        copied_unchanged_files=sum(name not in bodies for name in old),
        overlay_files=len(expected['overlay']), helpers=4, project_execution=False,
        canonical_changed=False, shared_cache_changed=False, lockfiles_changed=False,
        controller=pin(read(Path(__file__).resolve())))
    write('stage-complete.json', encoded(receipt))
    print(json.dumps(receipt, sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B transport.py pack | stage ARCHIVE_SHA INPUT_SHA')
    os.umask(0o077)
    def interrupted(number, _frame):
        raise RuntimeError('transport signal ' + str(number))
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        if sys.argv[1:] == ['pack']:
            pack()
        else:
            require(len(sys.argv) == 4 and sys.argv[1] == 'stage'
                    and all(re.fullmatch('[0-9a-f]{64}', value) for value in sys.argv[2:]), 'closed transport CLI')
            stage(*sys.argv[2:])
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
