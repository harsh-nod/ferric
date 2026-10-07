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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-scoped-bank-rearm-v1')
Q = Path('/home/harsh/ferric-p227-integration/qualification')
LOCAL_FERRIC = Q.parent
LOCAL_BASE = Q / 'guarded-mlp-full2303-scoped-currentness-v1/cpu-v1'
LOCAL_RUNTIME = Path('/home/harsh/fe2o3-p228-runtime')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-scoped-currentness-cpu-v228-v1'
ROOT = E / 'guarded-mlp-scoped-bank-rearm-cpu-v228-v2'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-scoped-bank-rearm-input-v228-v2.tar.gz'
CONTROLLER_PIN = dict(bytes=53065, sha256='80f65503058f9950cdb9bd76f8016f413080c701f86c2b344b5a80f5ee62d9a7')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=41316, sha256='4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a')
CACHE_PIN = {"bytes":12788,"sha256":"6cf8676e08bf275b32b8cabc1b03cdb8716577ded12899151759fa40e0fdd884"}
CACHE_MANIFEST = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
PROPOSALS = {'runtime-proposal.json': (10029, '2798cf16b04d0c77c5a1df155513a99c226907c1db46558e4d882da9ddf2e7ff'), 'worker-proposal.json': (15970, 'bb28743219931b14202c725650ff45841e3857a3cc5112390896dfc163c6626c'), 'worker-repair.json': (17360, '70f9d4e91c0c2d22bdc647f6cac67aa889fd7e9972c21050caa19cb5d4845b53')}
SELECTOR_SCHEMA = 'ferric-readiness40-bank-scoped-warm-worker-source-v2'
SELECTOR_ROWS = 11
WORKER_FILES = 216
TOTAL_SOURCES = 1042
TOTAL_MEMBERS = 39
BASE_COMPLETE = {'bytes': 2402339, 'sha256': 'a44623c8586dd5ba00eb55f0d549eb11b01c7c53b96f363749246eef1037d2a1'}
BASE_SOURCES = {'bytes': 430941, 'sha256': 'c3dbb35ea085d4166e4b5ee58b29732ebfe2f14c63599f693ceefdc5a8959d37'}
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = 'scoped-bank-rearm-coupled-v2'


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
    original_worker = proposal
    repaired_raw = bodies['inputs/worker-repair.json']
    require(pin(repaired_raw) == {"bytes":17360,"sha256":"70f9d4e91c0c2d22bdc647f6cac67aa889fd7e9972c21050caa19cb5d4845b53"}, 'literal reviewed worker repair manifest')
    proposal = parse(repaired_raw)
    repair = proposal['repair']
    repair_path = "adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1_tests.rs"
    require(proposal['source_generation'] == 'bank-scoped-warm-worker-v2-local-unsafe-test-allowance'
            and {k: proposal['predecessor_manifest'][k] for k in ('bytes', 'sha256')}
                == dict(bytes=15970, sha256='bb28743219931b14202c725650ff45841e3857a3cc5112390896dfc163c6626c')
            and {k: v for k, v in original_worker.items() if k not in ('files', 'readme', 'patch')}
                == {k: v for k, v in proposal.items() if k not in
                    ('files', 'readme', 'patch', 'source_generation', 'predecessor_manifest', 'repair')}
            and repair['path'] == repair_path
            and repair['before'] == {"bytes":19939,"sha256":"27c422af23b93adc05c3a8619015c9a0af0506e9f20d2b2d8ff319e85e460e33"}
            and repair['after'] == {"bytes":20118,"sha256":"8b97a5446abdab7071d02587cbe9fcea071ff8c383f5e435ca202fdc8fb56545"}
            and all(repair[key] is False for key in ('production_changed', 'test_names_changed',
                'assertions_changed', 'crate_or_module_lint_changed', 'observed_worker_attempt_passed')),
            'original worker proposal retained with sole reviewed test-local repair')
    old_rows = {row['path']: row for row in original_worker['files']}
    new_rows = {row['path']: row for row in proposal['files']}
    require(len(old_rows) == len(new_rows) == len(original_worker['files']) == len(proposal['files']) == 11
            and set(old_rows) == set(new_rows)
            and all(new_rows[name] == (dict(row, after=repair['after']) if name == repair_path else row)
                    for name, row in old_rows.items())
            and old_rows[repair_path]['after'] == repair['before'],
            'eleven unchanged preimages and exact one-test postimage transition')
    repaired_body = bodies['ferric/' + repair_path]
    require(pin(repaired_body) == repair['after'], 'actual preformat repaired test bytes')
    annotation = b"#[test]\n#[allow(unsafe_code)]\nfn bank_scoped_cli_refuses_policy_combinations_before_input_and_keeps_deadline() {"
    original_annotation = b"#[test]\nfn bank_scoped_cli_refuses_policy_combinations_before_input_and_keeps_deadline() {"
    safety = b"    // SAFETY: the owned empty Cursor returns EOF before bootstrap admission,\n    // setup preparation, native group opening, or any machine-code execution.\n"
    require(repaired_body.count(annotation) == repaired_body.count(safety) == 1
            and pin(repaired_body.replace(annotation, original_annotation).replace(safety, b''))
                == repair['before'], 'byte-exact reversal of function-local unsafe test allowance')
    require(proposal['schema'] == SELECTOR_SCHEMA and len(proposal['files']) == SELECTOR_ROWS
            and all(proposal[key] is False for key in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'currentness_temporal_equivalence_claim', 'performance_claim',
                'numerical_acceptance', 'full_launch_admitted', 'allocation_preflights_changed'))
            and compact({'x': proposal['base']['worker_receipt']})['x'] == BASE_COMPLETE
            and compact({'x': proposal['base']['worker_sources']})['x'] == BASE_SOURCES
            and proposal['conditional'] == dict(worker_files=216, worker_passed=760,
                worker_ignored=4, worker_inventory=764, coupled_runtime_files=822,
                coupled_source_files_including_four_helpers=1042),
            'exact reviewed bank-scoped worker contract')
    base = parse(bodies['inputs/baseline-complete.json'])
    source_map = parse(bodies['inputs/baseline-sources.json'])
    require(pin(bodies['inputs/baseline-complete.json']) == BASE_COMPLETE
            and pin(bodies['inputs/baseline-sources.json']) == BASE_SOURCES
            and base['schema'] == 'ferric-guarded-mlp-full2303-scoped-currentness-cpu-v1'
            and base['source_generation'] == 'full2303-scoped-currentness-coupled-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == source_map
            and base['source_unchanged'] is True and len(source_map) == 1036
            and base['gpu_execution'] is False
            and len(base['phases']) == 26 and all(row['exit_code'] == 0
                and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                for row in base['phases']), 'actual immediate qualified coupled base')
    old = {name: row for name, row in compact(source_map).items()
           if name.startswith(('fe2o3/', WORKER_PREFIX))}
    require(len(old) == 1032 and sum(name.startswith('fe2o3/') for name in old) == 820
            and sum(name.startswith(WORKER_PREFIX) for name in old) == 212
            and all(relative(name) and closed_pin(row) for name, row in old.items()),
            'closed actual 820-runtime/212-worker base')
    files, overlay = dict(old), set()
    require(len({row['path'] for row in proposal['files']}) == SELECTOR_ROWS
            and sum(row['before'] is None for row in proposal['files']) == 4,
            'eleven worker rows with four additions')
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
    require(runtime['schema'] == 'ferric-scoped-bank-rearm-runtime-source-v1'
            and runtime['status'] == 'source-only-uncompiled-unexecuted'
            and runtime['baseline']['runtime_map_rows'] == 820
            and runtime['baseline']['runtime_passed'] == 1143 and runtime['baseline']['runtime_ignored'] == 8
            and runtime['baseline']['terminal'] == dict(bytes=2364691,
                sha256='335cf93cc109390f3b7590f7dbe35ed852adca223a042894dcc18418a738ffbd')
            and runtime['baseline']['sources'] == compact({'x': base['readset']['baseline-sources.json']})['x']
            and base['runtime_source_changed'] is False
            and runtime['conditional'] == dict(replacement_rows=10, added_rows=2, runtime_map_rows=822,
                runtime_passed=1152, runtime_ignored=8, runtime_inventory=1160, mixed_bank_scope_passed=22)
            and runtime['scope'] == dict(ordinary_rearm_unchanged=True, fixed_roster_no_user_callback=True,
                shader_execution_in_scope=False, global_policy_changed=False, worker_selector_added=False,
                temporal_equivalence_claim=False, performance_claim=False, native_execution=False)
            and compact({'x': proposal['requires']['runtime_manifest']})['x'] == pin(runtime_raw)
            and compact({'x': proposal['requires']['runtime_interface']})['x'] == runtime['interface'],
            'explicit reviewed runtime dependency and qualified base lineage')
    require(len(runtime['files']) == len({row['path'] for row in runtime['files']}) == 12
            and sum(row['before'] is None for row in runtime['files']) == 2,
            'twelve runtime rows with two additions')
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
            and sum(name.startswith('fe2o3/') for name in files) == 822
            and sum(name.startswith(WORKER_PREFIX) for name in files) == WORKER_FILES,
            'closed complete composed runtime and worker map')
    expected = overlay | {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py',
        *('inputs/' + name for name in history), *('inputs/' + name for name in PROPOSALS)}
    require(set(bodies) == expected and len(bodies) + 1 == TOTAL_MEMBERS,
            'exact twenty-three-overlay, four-helper, ten-lineage package')
    inputs = dict(schema='ferric-guarded-mlp-scoped-bank-rearm-cpu-input-v1',
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
        'inputs/worker-repair.json': PROPOSAL_ROOT / 'worker-v2/source-manifest.json',
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
    proposal = parse(read(source_paths['inputs/worker-repair.json']))
    for row in proposal['files']:
        source_paths['ferric/' + row['path']] = PROPOSAL_ROOT / 'worker-v2/ferric' / row['path']
    runtime = parse(read(source_paths['inputs/runtime-proposal.json']))
    for row in runtime['files']:
        source_paths['fe2o3/' + row['path']] = PROPOSAL_ROOT / 'runtime-v1/runtime' / row['path']
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
    require(len(canonical) == 816 + 212 + 2
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
                         members=TOTAL_MEMBERS, source_files=TOTAL_SOURCES, runtime_files=822, worker_files=WORKER_FILES), sort_keys=True))


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
    receipt = dict(schema='ferric-guarded-mlp-scoped-bank-rearm-stage-v1', passed=True, root=str(ROOT),
        archive=pin(archive_raw), input=pin(raw_input), source_files=TOTAL_SOURCES, runtime_files=822, worker_files=WORKER_FILES,
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
