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
LOCAL_BASE = Q / 'guarded-mlp-full2303-bank-scoped-census-v1/cpu-v1'
LOCAL_RUNTIME = Path('/home/harsh/fe2o3-p228-runtime')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-bank-scoped-census-cpu-v228-v1'
ROOT = E / 'guarded-mlp-scoped-tail-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-scoped-tail-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=51583, sha256='ac62522a40eb3b2656ffb2726e58ef03cf3eda352924a3bb592a97dbdb7e0a80')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=41316, sha256='4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a')
CACHE_PIN = {"bytes":12782,"sha256":"6104a5f779b28014e8dc36a24f103d503760f6da2be4e55e528c092bebb906a3"}
CACHE_MANIFEST = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
PROPOSALS = {"runtime-proposal.json":[10858,"71da4afd43eb9f7de8fa1cac47cf8e9b2f0ececc86cfb98877b94678d500df83"],"worker-proposal.json":[27229,"96463bf9a70d6020e28c2d44260f589c38e7f61229f8c8e77f28242f1893ca71"]}
SELECTOR_SCHEMA = "ferric-readiness40-bank-scoped-census-tail-worker-source-v4"
SELECTOR_ROWS = 13
WORKER_FILES = 228
TOTAL_SOURCES = 1059
TOTAL_MEMBERS = 34
BASE_COMPLETE = {"bytes":2475794,"sha256":"a7fcea3f09cfcda84ba556f60b5f1578ed378a45fc84bb81fbfa6ed31273cd64"}
BASE_SOURCES = {"bytes":438675,"sha256":"5372e185504d11ed89d45cfb3d3e32672f93e8d1bbf3d04e44932b280300528c"}
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = 'scoped-tail-coupled-v1'


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
    size, digest = PROPOSALS['worker-proposal.json']
    raw = bodies['inputs/worker-proposal.json']
    require(pin(raw) == dict(bytes=size, sha256=digest), 'reviewed worker proposal manifest')
    proposal = parse(raw)
    require(proposal['schema'] == SELECTOR_SCHEMA and len(proposal['files']) == SELECTOR_ROWS
            and all(proposal[key] is False for key in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'currentness_temporal_equivalence_claim', 'performance_claim',
                'numerical_acceptance', 'full_launch_admitted'))
            and proposal['allocation_preflights_changed'] is False
            and proposal['existing_selectors_changed'] is False
            and proposal['first_two_forwards_unchanged'] is True
            and proposal['new_explicit_readiness40_tail_route'] is True
            and proposal['full2303_route_changed'] is False
            and compact({'x': proposal['base']['worker_receipt']})['x'] == BASE_COMPLETE
            and compact({'x': proposal['base']['worker_sources']})['x'] == BASE_SOURCES
            and proposal['conditional_qualification'] == {"worker_files":228,"passed":820,"ignored":4,"inventory":824,"new_library_tests":20,"new_readiness_cli_tests":1,"targets":{"library":{"passed":793,"ignored":4},"binary":{"passed":0,"ignored":0},"readiness_cli":{"passed":11,"ignored":0},"wire":{"passed":16,"ignored":0}},"coupled_sources":1059,"runtime_sources":827,"runtime_passed":1180,"runtime_ignored":8,"helper_sources":4,"phases":27,"elf_products":11,"qualified":False},
            'exact reviewed tail worker contract')
    base = parse(bodies['inputs/baseline-complete.json'])
    source_map = parse(bodies['inputs/baseline-sources.json'])
    require(pin(bodies['inputs/baseline-complete.json']) == BASE_COMPLETE
            and pin(bodies['inputs/baseline-sources.json']) == BASE_SOURCES
            and base['schema'] == 'ferric-guarded-mlp-full2303-bank-scoped-census-cpu-v1'
            and base['source_generation'] == 'full2303-bank-scoped-census-coupled-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == source_map
            and base['source_unchanged'] is True and len(source_map) == 1053
            and base['gpu_execution'] is False
            and len(base['phases']) == 26 and all(row['exit_code'] == 0
                and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                for row in base['phases']), 'actual immediate qualified coupled base')
    old = {name: row for name, row in compact(source_map).items()
           if name.startswith(('fe2o3/', WORKER_PREFIX))}
    require(len(old) == 1049 and sum(name.startswith('fe2o3/') for name in old) == 825
            and sum(name.startswith(WORKER_PREFIX) for name in old) == 224
            and all(relative(name) and closed_pin(row) for name, row in old.items()),
            'closed actual 825-runtime/224-worker base')
    files, overlay = dict(old), set()
    require(len({row['path'] for row in proposal['files']}) == SELECTOR_ROWS
            and sum(row['before'] is None for row in proposal['files']) == 4,
            'thirteen worker rows with four additions')
    for row in proposal['files']:
        name, path = 'ferric/' + row['path'], row['path']
        require(relative(path) and path.endswith('.rs') and name.startswith(WORKER_PREFIX)
                and row['repository'] == 'ferric' and files.get(name) == row['before']
                and closed_pin(row['after']), 'exact worker preimage/absence')
        files[name] = row['after']
        overlay.add(name)
    runtime_raw = bodies['inputs/runtime-proposal.json']
    size, digest = PROPOSALS['runtime-proposal.json']
    require(pin(runtime_raw) == dict(bytes=size, sha256=digest), 'reviewed runtime proposal manifest')
    runtime = parse(runtime_raw)
    require(runtime['schema'] == 'ferric-scoped-tail-runtime-source-v1'
            and runtime['status'] == 'source-only-uncompiled-unexecuted'
            and runtime['baseline']['runtime_map_rows'] == 825
            and runtime['baseline']['runtime_passed'] == 1165 and runtime['baseline']['runtime_ignored'] == 8
            and runtime['baseline']['terminal'] == compact({'x': base['readset']['baseline-complete.json']})['x']
            and runtime['baseline']['sources'] == compact({'x': base['readset']['baseline-sources.json']})['x']
            and runtime['baseline']['test_inventory'] == pin(bodies['inputs/runtime-list.stdout'])
            and runtime['conditional'] == {"runtime_map_rows":827,"canonical_runtime_bodies":825,"runtime_passed":1180,"runtime_ignored":8,"runtime_test_inventory":1188,"added_tests":15,"focus_prefix":"engineering_gfx950::peer::scoped_tail_v1::tests::","qualified":False}
            and all(value is False for value in runtime['claims'].values())
            and compact({'x': proposal['requires']['runtime']})['x'] == pin(runtime_raw)
            and compact({'x': proposal['requires']['interface']})['x'] == compact({'x': runtime['interface']})['x'],
            'reviewed tail runtime and unchanged qualified runtime ancestry')
    require(len(runtime['files']) == len({row['path'] for row in runtime['files']}) == 6
            and sum(row['before'] is None for row in runtime['files']) == 2,
            'six runtime rows with two additions')
    for row in runtime['files']:
        name, path = 'fe2o3/' + row['path'], row['path']
        require(relative(path) and path.startswith('crates/fe2o3-kfd/src/') and path.endswith('.rs')
                and row['repository'] == 'fe2o3' and files.get(name) == row['before']
                and closed_pin(row['after']), 'exact runtime preimage/absence')
        files[name] = row['after']
        overlay.add(name)
    for name in overlay:
        require(pin(bodies[name]) == files[name], 'final reviewed runtime or worker body')
    for name in ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py'):
        files[name] = pin(bodies[name])
    require(len(files) == TOTAL_SOURCES and sum(row['bytes'] for row in files.values()) <= 64 << 20
            and sum(name.startswith('fe2o3/') for name in files) == 827
            and sum(name.startswith(WORKER_PREFIX) for name in files) == WORKER_FILES,
            'closed complete composed runtime and worker map')
    expected = overlay | {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py',
        *('inputs/' + name for name in history), *('inputs/' + name for name in PROPOSALS)}
    require(set(bodies) == expected and len(bodies) + 1 == TOTAL_MEMBERS,
            'exact nineteen-overlay, four-helper, ten-lineage package')
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
        'inputs/baseline-complete.json': LOCAL_BASE / 'evidence/complete.json',
        'inputs/baseline-sources.json': LOCAL_BASE / 'evidence/sources-after.json',
        'inputs/runtime-tests.stdout': LOCAL_BASE / 'evidence/kfd-tests.stdout',
        'inputs/runtime-list.stdout': LOCAL_BASE / 'evidence/kfd-list.stdout',
        'inputs/worker-tests.stdout': LOCAL_BASE / 'evidence/worker-tests.stdout',
        'inputs/worker-list.stdout': LOCAL_BASE / 'evidence/worker-list.stdout',
        'inputs/runtime-docs.stdout': LOCAL_BASE / 'evidence/interface-doc-tests.stdout',
        'inputs/legacy-docs.stdout': LOCAL_BASE / 'inputs/legacy-docs.stdout',
    }
    proposal = parse(read(source_paths['inputs/worker-proposal.json']))
    for row in proposal['files']:
        source_paths['ferric/' + row['path']] = PROPOSAL_ROOT / 'worker-v1/ferric' / row['path']
    runtime = parse(read(source_paths['inputs/runtime-proposal.json']))
    for row in runtime['files']:
        source_paths['fe2o3/' + row['path']] = PROPOSAL_ROOT / 'runtime-v1/fe2o3' / row['path']
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
    require(len(canonical) == 821 + 224 + 2
            and not os.path.lexists(LOCAL_RUNTIME / 'Cargo.toml.original')
            and not os.path.lexists(LOCAL_RUNTIME / 'Cargo.lock.input'),
            'fixture Cargo bodies excluded and original aliases mapped explicitly')
    for row in proposal['files']:
        if row['before'] is None:
            require(not os.path.lexists(LOCAL_FERRIC / row['path']), 'new worker path remains absent')
    for row in runtime['files']:
        if row['before'] is None:
            require(not os.path.lexists(LOCAL_RUNTIME / row['path']), 'new runtime path remains absent')
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
