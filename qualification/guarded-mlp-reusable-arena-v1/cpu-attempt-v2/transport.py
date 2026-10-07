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
LOCAL_WORKER = Q / 'guarded-mlp-model-host-shared-currentness-v1/worker-cpu-v1'
LOCAL_RUNTIME = Q / 'guarded-mlp-model-interface-v1/cpu-attempt-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-host-shared-currentness-worker-cpu-v228-v1'
ROOT = E / 'guarded-mlp-reusable-arena-cpu-v228-v2'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-reusable-arena-input-v228-v2.tar.gz'
CONTROLLER_PIN = dict(bytes=36752, sha256='e9a390747017cb408ce1bbec955d9d0e347786c87467a0aa4facd32aaf4e3581')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=37166, sha256='fa63ab2dac6557177a8f75d8bfa17d48feff728ca66943fbadc4ed6f4d313ebd')
CACHE_PIN = dict(bytes=12785, sha256='dcdb3041320b454b0dab9e0ed5fd3fe2695fb524925686af6a8d90cf56131f69')
CACHE_MANIFEST = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
PROPOSAL_SHA = '4c08a6338231a98310ff63ea34d0ff89775c670fecc5069bbfac3eb5e85e8015'
BASE_COMPLETE = dict(bytes=1649375, sha256='0e7f73d69fd2d3efef901f62ee90fac54cfdddbae60661314d222f25aa2ae82a')
BASE_SOURCES = dict(bytes=415508, sha256='f898efd3843b2bb01625124514c9d5f437d7c3bbac5f5f61ec789bb455f04ad4')
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
    require(len(history) == 11, 'nine qualified raw lineage bodies and two actual failure bodies')
    for name, (size, digest) in history.items():
        require(pin(bodies['inputs/' + name]) == dict(bytes=size, sha256=digest), 'qualified lineage byte pin')
    proposal_raw = bodies['inputs/runtime-proposal.json']
    require(pin(proposal_raw)['sha256'] == PROPOSAL_SHA, 'frozen reviewed runtime source manifest')
    proposal = parse(proposal_raw)
    require(proposal['schema'] == 'ferric-guarded-mlp-paired-arena-reuse-source-v1'
            and len(proposal['files']) == 14 and len({row['path'] for row in proposal['files']}) == 14
            and sum(row['before'] is None for row in proposal['files']) == 4, 'ten replacements and four additions')
    base = parse(bodies['inputs/worker-complete.json'])
    source_map = parse(bodies['inputs/worker-sources.json'])
    require(pin(bodies['inputs/worker-complete.json']) == BASE_COMPLETE
            and pin(bodies['inputs/worker-sources.json']) == BASE_SOURCES
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == source_map and base['source_unchanged'] is True
            and len(source_map) == 993 and base['gpu_execution'] is False, 'actual immediate qualified source base')
    old = {name: row for name, row in compact(source_map).items() if name.startswith(('fe2o3/', WORKER_PREFIX))}
    require(len(old) == 991 and sum(name.startswith('fe2o3/') for name in old) == 807
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
    require(len(files) == 999 and sum(row['bytes'] for row in files.values()) <= 64 << 20, 'closed full source map')
    expected_members = set(overlay) | {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py',
        'inputs/runtime-proposal.json', *('inputs/' + name for name in history)}
    require(set(bodies) == expected_members and len(bodies) == 30, 'closed small overlay archive body set')
    inputs = dict(schema='ferric-guarded-mlp-reusable-arena-cpu-input-v1', source_generation=GENERATION,
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
        'inputs/runtime-proposal.json': P.parent / 'runtime-source-manifest.json',
    }
    for role, base in (('worker', LOCAL_WORKER), ('runtime', LOCAL_RUNTIME)):
        for label, original in (('complete.json', 'complete.json'), ('sources.json', 'sources-after.json'),
                                ('tests.stdout', 'worker-tests.stdout' if role == 'worker' else 'kfd-tests.stdout'),
                                ('list.stdout', 'worker-list.stdout' if role == 'worker' else 'kfd-list.stdout')):
            source_paths['inputs/' + role + '-' + label] = base / 'evidence' / original
    source_paths['inputs/runtime-docs.stdout'] = LOCAL_RUNTIME / 'evidence/interface-doc-tests.stdout'
    source_paths['inputs/attempt-v1-failed.json'] = W / 'failed.json'
    source_paths['inputs/attempt-v1-docs.stdout'] = W / 'interface-doc-tests.stdout'
    proposal = parse(read(source_paths['inputs/runtime-proposal.json']))
    for row in proposal['files']:
        source_paths['fe2o3/' + row['path']] = P.parent / 'runtime' / row['path']
    bodies = {name: read(path) for name, path in source_paths.items()}
    inputs, old = contract(bodies)
    require(pin(read(P / 'cache-manifest.json')) == CACHE_MANIFEST, 'actual union cache package manifest')
    for name, expected in old.items():
        if name.startswith(WORKER_PREFIX):
            require(pin(read(LOCAL_WORKER / name)) == expected, 'all 184 unchanged worker bodies')
    require(all(read(path) == bodies[name] for name, path in source_paths.items()), 'package input posthash')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 31 and sum(map(len, bodies.values())) <= 16 << 20, 'bounded source transport')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(raw_input),
                         members=31, source_files=999, runtime_files=811, worker_files=184), sort_keys=True))


def stage(archive_sha, input_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE and not os.path.lexists(ROOT),
            'fresh exclusive qualification namespace')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual archive hash')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 31
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
    require(all(pin(read(ROOT / name)) == row for name, row in expected['files'].items()), 'all999 staged sources')
    require(all(pin(read(BASE / name)) == row for name, row in old.items())
            and read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json'], 'base posthash, no source changes')
    receipt = dict(schema='ferric-guarded-mlp-reusable-arena-stage-v1', passed=True, root=str(ROOT),
        archive=pin(archive_raw), input=pin(raw_input), source_files=999, runtime_files=811, worker_files=184,
        copied_unchanged_files=981, overlay_files=14, helpers=4, project_execution=False,
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
