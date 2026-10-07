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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-currentness-duration-v1')
Q = Path('/home/harsh/ferric-p227-integration/qualification')
LOCAL_FERRIC = Q.parent
LOCAL_BASE = Q / 'guarded-mlp-full2303-scoped-tail-v1/cpu-v1'
LOCAL_RUNTIME = Path('/home/harsh/fe2o3-p228-runtime')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-scoped-tail-cpu-v228-v1'
ROOT = E / 'guarded-mlp-currentness-duration-default-cpu-v228-v2'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-currentness-duration-default-input-v228-v2.tar.gz'
CONTROLLER_PIN = {"bytes":52230,"sha256":"d3c1a5bb0dde2815a33e91b8e6330a8d3b5ea5d4cca01b6e38bdb11e860fdce5"}
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=41316, sha256='4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a')
CACHE_PIN = {'bytes': 12799, 'sha256': '42fd7dc4bdb782e0719d81f2dddc54d356718005eb9f1cdc2fb12313aea5b226'}
CACHE_MANIFEST = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
PROPOSALS = {'runtime-proposal.json': (14821, '85cd4a641b0e7429f3eeb8aebe310e56a64487b467ca5e81431002c4a7ffc8aa'), 'worker-proposal.json': (25694, 'c6224930fef73622d36b2634f5ef88846db8c1b55e9a7e3fdd550247e5024e52')}
SELECTOR_SCHEMA = 'ferric-readiness40-tail-currentness-duration-worker-parent-source-v1'
SELECTOR_ROWS = 12
WORKER_FILES = 236
TOTAL_SOURCES = 1069
TOTAL_MEMBERS = 37
BASE_COMPLETE = {'bytes': 2498847, 'sha256': 'f25bd6d9f582a4e632c05e385961c260c0c95d2a119bac5af4927af6dc3b61ed'}
BASE_SOURCES = {'bytes': 435848, 'sha256': 'eda7f5f696a5b93d048923cd5f8bd83b51a5ca9cb77efab8322da97f4286e928'}
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = 'currentness-duration-default-coupled-v2'

MODE = 'default'


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
    require(CONTROLLER_PIN is not None and all(value is not None for value in PROPOSALS.values()),
            'reviewed controller and worker source bindings required')
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
    proposals = {}
    for name, (size, digest) in PROPOSALS.items():
        raw = bodies['inputs/' + name]
        require(pin(raw) == dict(bytes=size, sha256=digest), 'reviewed exact source proposal manifest')
        proposals[name] = parse(raw)
    runtime, worker = proposals['runtime-proposal.json'], proposals['worker-proposal.json']
    require(runtime['schema'] == 'ferric-currentness-duration-runtime-source-v1'
            and runtime['source_only'] is True
            and runtime['feature'] == 'engineering-currentness-duration-diagnostics'
            and runtime['feature_dependencies'] == ['engineering-gfx950']
            and len(runtime['files']) == 10
            and all(runtime['guarantees'][key] is False for key in (
                'canonical_changed', 'compiled', 'formatted', 'native_execution', 'tests_executed',
                'new_currentness_policy', 'new_execution_facade', 'cached_admission_enabled',
                'ambient_profile_enabled', 'performance_claim', 'full2303_feasibility')),
            'reviewed diagnostic runtime, no currentness authority')
    require(worker['schema'] == SELECTOR_SCHEMA and len(worker['files']) == SELECTOR_ROWS + 4
            and worker['features']['worker'] == dict(
                name='engineering-currentness-duration-diagnostics',
                dependencies=['fe2o3-kfd/engineering-currentness-duration-diagnostics'])
            and worker['features']['default_features_changed'] is False
            and worker['features']['locks_changed'] is False
            and all(worker['guarantees'][key] is False for key in (
                'canonical_changed', 'compiled', 'formatted', 'tests_executed',
                'currentness_policy_changed', 'new_execution_authority', 'new_runtime_facade',
                'new_selector', 'native_execution', 'numerical_acceptance', 'performance_claim')),
            'reviewed worker-parent manifest; parent rows excluded')
    worker_rows = [row for row in worker['files'] if row['repository'] == 'ferric'
                   and ('ferric/' + row['path']).startswith(WORKER_PREFIX)]
    require(len(worker_rows) == SELECTOR_ROWS, 'exact coupled worker subset')
    base = parse(bodies['inputs/baseline-complete.json'])
    source_map = parse(bodies['inputs/baseline-sources.json'])
    require(pin(bodies['inputs/baseline-complete.json']) == BASE_COMPLETE
            and pin(bodies['inputs/baseline-sources.json']) == BASE_SOURCES
            and base['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-cpu-v1'
            and base['source_generation'] == 'full2303-scoped-tail-coupled-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == source_map
            and base['source_unchanged'] is True and len(source_map) == 1063
            and base['gpu_execution'] is False
            and len(base['phases']) == 27 and all(row['exit_code'] == 0
                and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                for row in base['phases']), 'actual immediate Full Tail coupled base')
    old = {name: row for name, row in compact(source_map).items()
           if name.startswith(('fe2o3/', WORKER_PREFIX))}
    require(len(old) == 1059 and sum(name.startswith('fe2o3/') for name in old) == 827
            and sum(name.startswith(WORKER_PREFIX) for name in old) == 232
            and all(relative(name) and closed_pin(row) for name, row in old.items()),
            'closed actual 827-runtime/232-worker base')
    files, overlay = dict(old), set()
    for repository, rows, count, additions in (
            ('fe2o3', runtime['files'], 10, 2), ('ferric', worker_rows, SELECTOR_ROWS, 4)):
        require(len({row['path'] for row in rows}) == count
                and sum(row['before'] is None for row in rows) == additions,
                'unique exact runtime/worker overlays and additions')
        for row in rows:
            path, name = row['path'], repository + '/' + row['path']
            manifest = name in ('fe2o3/crates/fe2o3-kfd/Cargo.toml', WORKER_PREFIX + 'Cargo.toml')
            require(relative(path) and (path.endswith('.rs') or manifest)
                    and (name.startswith('fe2o3/crates/fe2o3-kfd/') if repository == 'fe2o3'
                         else name.startswith(WORKER_PREFIX))
                    and row['repository'] == repository and files.get(name) == row['before']
                    and closed_pin(row['after']), 'exact source preimage/absence, two crate manifests only')
            files[name] = row['after']
            overlay.add(name)
    for name in overlay:
        require(pin(bodies[name]) == files[name], 'final reviewed runtime/worker body')
    for name in ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py'):
        files[name] = pin(bodies[name])
    require(len(files) == TOTAL_SOURCES and sum(row['bytes'] for row in files.values()) <= 64 << 20
            and sum(name.startswith('fe2o3/') for name in files) == 829
            and sum(name.startswith(WORKER_PREFIX) for name in files) == WORKER_FILES,
            'closed complete source map, no parent overlays')
    expected = overlay | {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py',
        *('inputs/' + name for name in history), *('inputs/' + name for name in PROPOSALS)}
    require(set(bodies) == expected and len(bodies) + 1 == TOTAL_MEMBERS,
            'exact twenty-two-overlay, four-helper, ten-lineage package')
    inputs = dict(schema='ferric-guarded-mlp-currentness-duration-cpu-input-v1',
        source_generation=GENERATION, qualification_mode=MODE,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
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
        'inputs/worker-proposal.json': PROPOSAL_ROOT / 'worker-v2/source-manifest.json',
        'inputs/runtime-proposal.json': PROPOSAL_ROOT / 'runtime-v1/source-manifest.json',
        'inputs/baseline-complete.json': LOCAL_BASE / 'evidence/complete.json',
        'inputs/baseline-sources.json': LOCAL_BASE / 'evidence/sources-after.json',
        'inputs/runtime-tests.stdout': LOCAL_BASE / 'evidence/kfd-tests.stdout',
        'inputs/runtime-list.stdout': LOCAL_BASE / 'evidence/kfd-list.stdout',
        'inputs/worker-tests.stdout': LOCAL_BASE / 'evidence/worker-tests.stdout',
        'inputs/worker-list.stdout': LOCAL_BASE / 'evidence/worker-list.stdout',
        'inputs/runtime-docs.stdout': LOCAL_BASE / 'evidence/interface-doc-tests.stdout',
        'inputs/legacy-docs.stdout': Q / 'guarded-mlp-model-interface-v1/cpu-attempt-v1/evidence/interface-doc-tests.stdout',
    }
    runtime = parse(read(source_paths['inputs/runtime-proposal.json']))
    worker = parse(read(source_paths['inputs/worker-proposal.json']))
    worker_rows = [row for row in worker['files'] if row['repository'] == 'ferric'
                   and ('ferric/' + row['path']).startswith(WORKER_PREFIX)]
    proposal_rows = [('fe2o3', row) for row in runtime['files']] + [('ferric', row) for row in worker_rows]
    for repository, row in proposal_rows:
        folder = 'runtime-v1' if repository == 'fe2o3' else 'worker-v2'
        source_paths[repository + '/' + row['path']] = PROPOSAL_ROOT / folder / repository / row['path']
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
    require(len(canonical) == 823 + 232 + 2
            and not os.path.lexists(LOCAL_RUNTIME / 'Cargo.toml.original')
            and not os.path.lexists(LOCAL_RUNTIME / 'Cargo.lock.input'),
            'fixture Cargo bodies excluded and original aliases mapped explicitly')
    for repository, row in proposal_rows:
        if row['before'] is None:
            local = LOCAL_RUNTIME if repository == 'fe2o3' else LOCAL_FERRIC
            require(not os.path.lexists(local / row['path']), 'new source path remains absent')
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
                         members=TOTAL_MEMBERS, source_files=TOTAL_SOURCES, runtime_files=829, worker_files=WORKER_FILES), sort_keys=True))


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
    receipt = dict(schema='ferric-guarded-mlp-currentness-duration-stage-v1', passed=True, root=str(ROOT), qualification_mode=MODE,
        archive=pin(archive_raw), input=pin(raw_input), source_files=TOTAL_SOURCES, runtime_files=829, worker_files=WORKER_FILES,
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
