"""Pack the small overlay; stage by authenticating and copying the actual CPU base."""
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
Q = Path('/home/harsh/ferric-p227-integration/qualification')
LOCAL_FERRIC = Q.parent
LOCAL_WORKER = Q / 'guarded-mlp-readiness40-v1/worker-cpu-v1'
LOCAL_RUNTIME = Q / 'guarded-mlp-peer-read-pair-v1/runtime-cpu-v2'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness-cli-worker-cpu-v228-v1'
ROOT = E / 'guarded-mlp-terminal-pair-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-terminal-pair-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=39953, sha256='5e760e54201fa1d3d700414fa2e57e741c861a940bb634e7d55c1f1a3f068a2d')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=37922, sha256='640f8c694d3b4cbd1795e060916d548dfbc82b1fc944713637bd36dfad5ac6ce')
CACHE_PIN = dict(bytes=12784, sha256='afcffb1f6cc252915258da39fcc433c8cbfe0ca7df03f7e83c45eb9258730a56')
CACHE_MANIFEST = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
PROPOSAL_SHA = '80959e1cecf23ec059231620e3ed8e7169c05f8b41d4cef208ca439dfd38f6a5'
BASE_COMPLETE = dict(bytes=1675803, sha256='2b785c1b8384358f11d547be93614660171c94108aba5e50cdb8db7e4b462b77')
BASE_SOURCES = dict(bytes=412739, sha256='8fd9a73ff5954e82a45505e9e3cdd0b072565f2560403f58e6474093fa14ef3c')
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'


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
    require(CONTROLLER_PIN is not None and pin(bodies['run_cpu.py']) == CONTROLLER_PIN
            and pin(bodies['worker_support.py']) == SUPPORT_PIN and pin(bodies['supervisor.py']) == SUPERVISOR_PIN
            and pin(bodies['cache.py']) == CACHE_PIN, 'four exact reviewed controller/helper bodies')
    tree = ast.parse(bodies['run_cpu.py'])
    history = [node for node in tree.body if isinstance(node, ast.Assign)
               and any(isinstance(target, ast.Name) and target.id == 'HISTORY' for target in node.targets)]
    require(len(history) == 1, 'single literal lineage declaration')
    history = ast.literal_eval(history[0].value)
    require(len(history) == 10, 'nine current raw lineage bodies and one legacy doc body')
    for name, (size, digest) in history.items():
        require(pin(bodies['inputs/' + name]) == dict(bytes=size, sha256=digest), 'qualified lineage byte pin')
    proposal_raw = bodies['inputs/runtime-proposal.json']
    require(pin(proposal_raw)['sha256'] == PROPOSAL_SHA, 'frozen reviewed runtime source manifest')
    proposal = parse(proposal_raw)
    require(proposal['schema'] == 'ferric-guarded-mlp-terminal-pair-source-v1'
            and len(proposal['files']) == 6 and len({row['path'] for row in proposal['files']}) == 6
            and sum(row['before'] is None for row in proposal['files']) == 2, 'four replacements and two additions')
    base = parse(bodies['inputs/worker-complete.json'])
    source_map = parse(bodies['inputs/worker-sources.json'])
    require(pin(bodies['inputs/worker-complete.json']) == BASE_COMPLETE
            and pin(bodies['inputs/worker-sources.json']) == BASE_SOURCES
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == source_map and base['source_unchanged'] is True
            and len(source_map) == 1010 and base['gpu_execution'] is False, 'actual immediate qualified source base')
    old = {name: row for name, row in compact(source_map).items() if name.startswith(('fe2o3/', WORKER_PREFIX))}
    require(len(old) == 1008 and sum(name.startswith('fe2o3/') for name in old) == 813
            and all(relative(name) and closed_pin(row) for name, row in old.items()), 'closed qualified base source set')
    files = dict(old)
    overlay = []
    for row in proposal['files']:
        name = 'fe2o3/' + row['path']
        require(relative(row['path']) and row['path'].startswith('crates/fe2o3-kfd/')
                and old.get(name) == row['before'] and pin(bodies[name]) == row['after'], 'exact runtime source delta')
        files[name] = row['after']
        overlay.append(name)
    for name in ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py'):
        files[name] = pin(bodies[name])
    require(len(files) == 1014 and sum(row['bytes'] for row in files.values()) <= 64 << 20, 'closed full source map')
    expected_members = set(overlay) | {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py',
        'inputs/runtime-proposal.json', *('inputs/' + name for name in history)}
    require(set(bodies) == expected_members and len(bodies) == 21, 'closed small overlay archive body set')
    inputs = dict(schema='ferric-guarded-mlp-terminal-pair-cpu-input-v1', source_generation=GENERATION,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        lineage={name.removeprefix('inputs/'): pin(body) for name, body in sorted(bodies.items()) if name.startswith('inputs/')},
        overlay=sorted(overlay), cache_manifest=CACHE_MANIFEST)
    return inputs, old


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'input-manifest.json'), 'fresh package outputs')
    source_paths = {
        'run_cpu.py': P / 'run_cpu.py', 'cache.py': P / 'cache.py',
        'supervisor.py': LOCAL_WORKER / 'supervisor.py', 'worker_support.py': LOCAL_WORKER / 'run_cpu.py',
        'inputs/runtime-proposal.json': P.parent / 'source-manifest.json',
    }
    for role, base in (('worker', LOCAL_WORKER), ('runtime', LOCAL_RUNTIME)):
        for label, original in (('complete.json', 'complete.json'), ('sources.json', 'sources-after.json'),
                                ('tests.stdout', 'worker-tests.stdout' if role == 'worker' else 'kfd-tests.stdout'),
                                ('list.stdout', 'worker-list.stdout' if role == 'worker' else 'kfd-list.stdout')):
            source_paths['inputs/' + role + '-' + label] = base / 'evidence' / original
    source_paths['inputs/runtime-docs.stdout'] = LOCAL_RUNTIME / 'evidence/interface-doc-tests.stdout'
    source_paths['inputs/legacy-docs.stdout'] = Q / 'guarded-mlp-model-interface-v1/cpu-attempt-v1/evidence/interface-doc-tests.stdout'
    proposal = parse(read(source_paths['inputs/runtime-proposal.json']))
    for row in proposal['files']:
        source_paths['fe2o3/' + row['path']] = P.parent / 'runtime' / row['path']
    bodies = {name: read(path) for name, path in source_paths.items()}
    inputs, old = contract(bodies)
    require(pin(read(LOCAL_WORKER / 'cargo-cache-manifest.json')) == CACHE_MANIFEST, 'actual union cache package manifest')
    for name, expected in old.items():
        if name.startswith(WORKER_PREFIX):
            require(pin(read(LOCAL_FERRIC / name.removeprefix('ferric/'))) == expected,
                    'all 195 canonical worker bodies join the actual qualified map')
    require(all(read(path) == bodies[name] for name, path in source_paths.items()), 'package input posthash')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 22 and sum(map(len, bodies.values())) <= 16 << 20, 'bounded source transport')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(raw_input),
                         members=22, source_files=1014, runtime_files=815, worker_files=195), sort_keys=True))


def stage(archive_sha, input_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE and not os.path.lexists(ROOT),
            'fresh exclusive qualification namespace')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual archive hash')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 22
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
    require(all(pin(read(ROOT / name)) == row for name, row in expected['files'].items()), 'all1014 staged sources')
    require(all(pin(read(BASE / name)) == row for name, row in old.items())
            and read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json'], 'base posthash, no source changes')
    receipt = dict(schema='ferric-guarded-mlp-terminal-pair-stage-v1', passed=True, root=str(ROOT),
        archive=pin(archive_raw), input=pin(raw_input), source_files=1014, runtime_files=815, worker_files=195,
        copied_unchanged_files=1004, overlay_files=6, helpers=4, project_execution=False,
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
