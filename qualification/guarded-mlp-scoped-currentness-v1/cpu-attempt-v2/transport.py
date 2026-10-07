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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-guarded-full2303-v1')
Q = Path('/home/harsh/ferric-p227-integration/qualification')
LOCAL_FERRIC = Q.parent
LOCAL_WORKER = Q / 'guarded-mlp-readiness40-shared-full-v1/worker-cpu-v1'
LOCAL_RUNTIME = Q / 'guarded-mlp-terminal-pair-v1/cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness40-shared-full-worker-cpu-v228-v1'
ROOT = E / 'guarded-mlp-scoped-currentness-cpu-v228-v2'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-scoped-currentness-input-v228-v2.tar.gz'
CONTROLLER_PIN = dict(bytes=53673, sha256='1bcfbb8c6b1ba40a79c8603a45564123ae72ea7de196baa7e75f22c3d368dd5e')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=41316, sha256='4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a')
CACHE_PIN = dict(bytes=12789, sha256='35ffa957a6dff429572b9b4d9038e81a61712c1f8d28d678bbe36a58946d2c35')
CACHE_MANIFEST = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
PROPOSALS = {
    'runtime-proposal.json': (13373, '437948cd334c9f95dcf77de4e3217c92f4d8e6d4cabbcf5010aff182f53ae5b2'),
    'primitive-proposal.json': (3820, '9abcd59d4c3fd23d532bd1b9e86592a191e62960d8a901fbd31d8e61974bdec7'),
    'consumer-proposal.json': (1677, '6b4dffbe4032e52f6dddb80f255553c420e95a3fb7af84f2b1da311daf1168bb'),
    'selector-proposal.json': (15169, 'cfec625935042c416a329cfb0e02c1fc8836f12820967e1d3c0257ed2d6a8235'),
    'consumer-repair.json': (4565, 'c2be6fe83fba6c4bfef971bdf1e1535b88167ec8685f6faf6aa163abbdaa93e0'),
}
SELECTOR_SCHEMA = 'ferric-scoped-warm-readiness40-worker-selector-source-v1'
SELECTOR_ROWS = 9
WORKER_FILES = 209
TOTAL_SOURCES = 1033
TOTAL_MEMBERS = 55
BASE_COMPLETE = dict(bytes=1746409, sha256='4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337')
BASE_SOURCES = dict(bytes=428240, sha256='2566997ffb73fb40812688178f4494e48c1db4c8e7522717d54b9176994fd3e1')
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = 'scoped-currentness-coupled-v2'


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
    require(all(value is not None for value in (CONTROLLER_PIN, PROPOSALS['selector-proposal.json'],
            SELECTOR_SCHEMA, SELECTOR_ROWS, WORKER_FILES, TOTAL_SOURCES, TOTAL_MEMBERS)),
            'final reviewed selector/controller/composition bindings are required')
    require(pin(bodies['run_cpu.py']) == CONTROLLER_PIN
            and pin(bodies['worker_support.py']) == SUPPORT_PIN and pin(bodies['supervisor.py']) == SUPERVISOR_PIN
            and pin(bodies['cache.py']) == CACHE_PIN, 'four exact reviewed helper bodies')
    tree = ast.parse(bodies['run_cpu.py'])
    history = [node for node in tree.body if isinstance(node, ast.Assign)
               and any(isinstance(target, ast.Name) and target.id == 'HISTORY' for target in node.targets)]
    require(len(history) == 1, 'single literal lineage declaration')
    history = ast.literal_eval(history[0].value)
    require(len(history) == 10, 'nine current raw lineage bodies plus original legacy docs')
    for name, (size, digest) in history.items():
        require(pin(bodies['inputs/' + name]) == dict(bytes=size, sha256=digest), 'actual direct lineage pin')
    proposals = {}
    for name, (size, digest) in PROPOSALS.items():
        raw = bodies['inputs/' + name]
        require(pin(raw) == dict(bytes=size, sha256=digest), 'reviewed proposal manifest pin')
        proposals[name] = parse(raw)
    rp, pp, cp, sp = (proposals[name] for name in ('runtime-proposal.json',
        'primitive-proposal.json', 'consumer-proposal.json', 'selector-proposal.json'))
    require(rp['schema'] == 'ferric-scoped-currentness-routed-source-v1'
            and rp['primitive_manifest'] == pin(bodies['inputs/primitive-proposal.json'])
            and len(rp['files']) == 22 and rp['replacement_rows'] == 17 and rp['addition_rows'] == 5
            and cp['schema'] == 'ferric-scoped-warm-layer-worker-source-proposal-v1'
            and len(cp['files']) == 4
            and sp['schema'] == SELECTOR_SCHEMA and len(sp['files']) == SELECTOR_ROWS,
            'exact cumulative runtime, consumer and activated selector manifests')
    repaired = proposals['consumer-repair.json']
    require(repaired['schema'] == cp['schema']
            and compact({'x': repaired['predecessor_manifest']})['x'] == pin(bodies['inputs/consumer-proposal.json'])
            and repaired['source_generation'] == 'scoped-consumer-v2-module-path-repair'
            and repaired['declared_tests'] == cp['declared_tests'] == 7
            and all(repaired[key] is False for key in ('compiled', 'tested', 'native_execution',
                'canonical_changed', 'profile_enabled'))
            and len(repaired['files']) == len(cp['files']) == 4,
            'explicit repaired consumer with original selector lineage retained')
    repair = repaired['repair']
    require(repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/guarded_mlp_decode_v1/scoped_currentness_v1.rs',
        before_call='crate::native_forward::checked_layer_hidden_pair',
        after_call='crate::native_catalog::forward::checked_layer_hidden_pair',
        changed_occurrences=1, other_three_postimages_unchanged=True, tests_unchanged=True),
        'one declared registered-module path repair only')
    require(sorted(repaired['new_tests']['worker_library']) == sorted(
        name for names in sp['consumer_tests'].values() for name in names),
        'all seven original consumer names preserved')
    for old_row, new_row in zip(cp['files'], repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and (new_row['after'] != old_row['after'] if old_row['path'] == repair['path']
                     else new_row == old_row), 'one cumulative postimage transition')
    repaired_body = bodies['ferric/' + repair['path']]
    before_call, after_call = repair['before_call'].encode(), repair['after_call'].encode()
    old_pin = next(row['after'] for row in cp['files'] if row['path'] == repair['path'])
    require(repaired_body.count(after_call) == 1 and before_call not in repaired_body
            and pin(repaired_body.replace(after_call, before_call)) == old_pin,
            'byte-exact single-call repair reverses to original consumer postimage')
    require(compact({'x': sp['requires']['consumer']})['x'] == pin(bodies['inputs/consumer-proposal.json'])
            and [{key: row[key] for key in ('path', 'before', 'after')}
                 for row in sp['requires']['consumer']['files']] == cp['files'],
            'original selector requires the unchanged original consumer manifest')
    base = parse(bodies['inputs/worker-complete.json'])
    source_map = parse(bodies['inputs/worker-sources.json'])
    require(pin(bodies['inputs/worker-complete.json']) == BASE_COMPLETE
            and pin(bodies['inputs/worker-sources.json']) == BASE_SOURCES
            and base['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-worker-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == source_map
            and base['source_unchanged'] is True and len(source_map) == 1022
            and base['gpu_execution'] is False, 'actual immediate qualified source base')
    old = {name: row for name, row in compact(source_map).items() if name.startswith(('fe2o3/', WORKER_PREFIX))}
    require(len(old) == 1020 and sum(name.startswith('fe2o3/') for name in old) == 815
            and sum(name.startswith(WORKER_PREFIX) for name in old) == 205
            and all(relative(name) and closed_pin(row) for name, row in old.items()),
            'closed 815-runtime/205-worker actual source set')
    files, overlay = dict(old), set()
    primitive = {row['path']: row for row in pp['files']}
    for source, prefix in ((rp, 'fe2o3/'), (repaired, 'ferric/'), (sp, 'ferric/')):
        paths = [row['path'] for row in source['files']]
        require(len(paths) == len(set(paths)), 'unique overlay rows')
        for row in source['files']:
            name, path = prefix + row['path'], row['path']
            require(relative(path) and path.endswith('.rs')
                    and (path.startswith('crates/fe2o3-kfd/') if prefix == 'fe2o3/'
                         else name.startswith(WORKER_PREFIX))
                    and files.get(name) == row['before'] and closed_pin(row['after']),
                    'exact ordered source preimage/absence')
            if source is rp and path in primitive:
                require(row['primitive_postimage'] == primitive[path]['after']
                        and row['before'] == primitive[path]['before'], 'primitive intermediate source join')
            files[name] = row['after']
            overlay.add(name)
    for name in overlay:
        require(pin(bodies[name]) == files[name], 'final composed source body')
    for name in ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py'):
        files[name] = pin(bodies[name])
    require(len(files) == TOTAL_SOURCES and sum(row['bytes'] for row in files.values()) <= 64 << 20
            and sum(name.startswith('fe2o3/') for name in files) == 820
            and sum(name.startswith(WORKER_PREFIX) for name in files) == WORKER_FILES,
            'closed complete composed source map')
    expected = overlay | {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py',
        *('inputs/' + name for name in history), *('inputs/' + name for name in PROPOSALS)}
    require(set(bodies) == expected and len(bodies) + 1 == TOTAL_MEMBERS, 'exact overlay archive body set')
    inputs = dict(schema='ferric-guarded-mlp-scoped-currentness-cpu-input-v1', source_generation=GENERATION,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        lineage={name.removeprefix('inputs/'): pin(body) for name, body in sorted(bodies.items()) if name.startswith('inputs/')},
        overlay=sorted(overlay), cache_manifest=CACHE_MANIFEST)
    return inputs, old

def pack():
    require(all(value is not None for value in (CONTROLLER_PIN, PROPOSALS['selector-proposal.json'],
            SELECTOR_SCHEMA, SELECTOR_ROWS, WORKER_FILES, TOTAL_SOURCES, TOTAL_MEMBERS)),
            'pending selector/controller/composition bindings')
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'input-manifest.json'), 'fresh package outputs')
    source_paths = {
        'run_cpu.py': P / 'run_cpu.py', 'cache.py': P / 'cache.py',
        'supervisor.py': LOCAL_WORKER / 'supervisor.py', 'worker_support.py': LOCAL_WORKER / 'run_cpu.py',
        'inputs/runtime-proposal.json': PROPOSAL_ROOT / 'scoped-currentness-routed-v1/source-manifest.json',
        'inputs/primitive-proposal.json': PROPOSAL_ROOT / 'scoped-currentness-primitive-v1/source-manifest.json',
        'inputs/consumer-proposal.json': PROPOSAL_ROOT / 'scoped-worker-v1/source-manifest.json',
        'inputs/consumer-repair.json': PROPOSAL_ROOT / 'scoped-worker-v2/source-manifest.json',
        'inputs/selector-proposal.json': PROPOSAL_ROOT / 'scoped-worker-selector-v1/source-manifest.json',
    }
    for role, base in (('worker', LOCAL_WORKER), ('runtime', LOCAL_RUNTIME)):
        for label, original in (('complete.json', 'complete.json'), ('sources.json', 'sources-after.json'),
                                ('tests.stdout', 'worker-tests.stdout' if role == 'worker' else 'kfd-tests.stdout'),
                                ('list.stdout', 'worker-list.stdout' if role == 'worker' else 'kfd-list.stdout')):
            source_paths['inputs/' + role + '-' + label] = base / 'evidence' / original
    source_paths['inputs/runtime-docs.stdout'] = LOCAL_RUNTIME / 'evidence/interface-doc-tests.stdout'
    source_paths['inputs/legacy-docs.stdout'] = Q / 'guarded-mlp-model-interface-v1/cpu-attempt-v1/evidence/interface-doc-tests.stdout'
    for name, prefix, directory in (
            ('runtime-proposal.json', 'fe2o3/', PROPOSAL_ROOT / 'scoped-currentness-routed-v1/runtime'),
            ('consumer-repair.json', 'ferric/', PROPOSAL_ROOT / 'scoped-worker-v2/ferric'),
            ('selector-proposal.json', 'ferric/', PROPOSAL_ROOT / 'scoped-worker-selector-v1/ferric')):
        proposal = parse(read(source_paths['inputs/' + name]))
        for row in proposal['files']:
            source_paths[prefix + row['path']] = directory / row['path']
    bodies = {name: read(path) for name, path in source_paths.items()}
    inputs, old = contract(bodies)
    require(pin(read(LOCAL_WORKER / 'cargo-cache-manifest.json')) == CACHE_MANIFEST, 'actual union cache package manifest')
    for name, expected in old.items():
        if name.startswith(WORKER_PREFIX):
            require(pin(read(LOCAL_FERRIC / name.removeprefix('ferric/'))) == expected,
                    'all 205 canonical worker bodies join the actual qualified map')
    require(all(read(path) == bodies[name] for name, path in source_paths.items()), 'package input posthash')
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
                         members=TOTAL_MEMBERS, source_files=TOTAL_SOURCES, runtime_files=820, worker_files=WORKER_FILES), sort_keys=True))


def stage(archive_sha, input_sha):
    require(all(value is not None for value in (CONTROLLER_PIN, PROPOSALS['selector-proposal.json'],
            SELECTOR_SCHEMA, SELECTOR_ROWS, WORKER_FILES, TOTAL_SOURCES, TOTAL_MEMBERS)),
            'pending selector/controller/composition bindings')
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
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json'], 'live immutable qualified base receipts')
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
            and read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json'], 'base posthash, no source changes')
    receipt = dict(schema='ferric-guarded-mlp-scoped-currentness-stage-v1', passed=True, root=str(ROOT),
        archive=pin(archive_raw), input=pin(raw_input), source_files=TOTAL_SOURCES, runtime_files=820, worker_files=WORKER_FILES,
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
