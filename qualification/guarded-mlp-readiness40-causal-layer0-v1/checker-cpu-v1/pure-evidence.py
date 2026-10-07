"""Closed data-only export/retention of the causal synthetic CPU gate."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
Q = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-readiness40-causal-layer0-v1')
SUPERVISOR = (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
MODES = {
    'checker': dict(count=29, modules={'test_readiness': 'ReadinessTests', 'test_causal': 'CausalTests'}, sources={
        'run_cpu.py': (12813, '4ab5c205146ffaaf62fdcb3cb4adf51f8ff668c417b1a8269cfea29afa30e281'),
        'supervisor.py': SUPERVISOR,
        'validate_readiness.py': (19525, '57a7a8cfe5c1327df0f1c006b1c70fa6cdf860d441a057d3a6764669d89ccc40'),
        'readiness_announcement.py': (3246, 'b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1'),
        'test_readiness.py': (17802, '4b644462a71120ee45b3351757ecb0d5867c9453daadb6a10c07bdbcd38e8bb5'),
        'validate_causal.py': (30083, 'a86f17d88340f6466c570cb2e09a74bbc14d2fc9626cb4e9060632ca03c9cd87'),
        'test_causal.py': (14636, 'ca3c5d2942baed09517e50e648f408e8645650f1a189e9715dedc975d4ecfee4')}),
}
RAW = {'sources-before.json', 'sources-after.json'} | {
    'readiness-tests.' + suffix for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
FILE_LIMIT, TOTAL_LIMIT = 1 << 20, 4 << 20


def require(ok, why):
    if not ok:
        raise ValueError(why)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, cap=FILE_LIMIT):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= cap, 'bounded regular input')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'input drift')
    return raw


def locations(mode):
    require(mode in MODES, 'closed CPU gate mode')
    root = E / ('guarded-mlp-readiness40-causal-layer0-' + mode + '-cpu-v228-v1')
    name = 'guarded-mlp-readiness40-causal-layer0-' + mode + '-evidence-v228-v1.tar.gz'
    return root, name, Q / (mode + '-cpu-v1')


def validate(mode, bodies, terminal_sha):
    contract = MODES[mode]
    root, _, _ = locations(mode)
    original = set(contract['sources']) | {'evidence/' + n for n in RAW | {'complete.json'}}
    require(set(bodies) == original | {'pure-evidence.py'}, 'fifteen originals and exact helper')
    require(pin(bodies['evidence/complete.json'])['sha256'] == terminal_sha, 'observed terminal SHA')
    c = parse(bodies['evidence/complete.json'])
    require(c['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-' + mode + '-cpu-v1'
            and c['passed'] is True and c['failure'] is None and c['postcheck_errors'] == []
            and c['sources_before'] == c['sources_after'] and c['source_unchanged'] is True,
            'successful unchanged CPU gate')
    require(set(c['sources_after']) == set(contract['sources']) and set(c['raw']) == RAW,
            'original source/raw closure')
    for name, expected in contract['sources'].items():
        require((len(bodies[name]), pin(bodies[name])['sha256']) == expected
                and compact(c['sources_after'][name]) == pin(bodies[name])
                and c['sources_after'][name]['path'] == str(root / name), 'fixed original source join')
    require(c['controller'] == c['sources_after']['run_cpu.py']
            and c['supervisor'] == c['sources_after']['supervisor.py'], 'harness source joins')
    for name, row in c['raw'].items():
        require(pin(bodies['evidence/' + name]) == compact(row)
                and row['path'] == str(root / 'evidence' / name), 'original raw pin/path')
    for label in ('before', 'after'):
        require(parse(bodies['evidence/sources-' + label + '.json']) == c['sources_' + label], 'source map join')
    require(len(c['phases']) == 1, 'sole owned test child')
    phase = c['phases'][0]
    require(phase['label'] == 'readiness-tests' and phase['exit_code'] == 0
            and phase['natural_exit'] is True and phase['reaped'] is True
            and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
            and phase['timed_out'] is False and phase['exception'] is None
            and phase['storage_failure'] is None and phase['observed_signals'] == [], 'clean natural child')
    require(parse(bodies['evidence/readiness-tests.result.json']) == phase, 'original result join')
    require(parse(bodies['evidence/readiness-tests.started.json']) ==
            dict(pid=phase['pid'], pgid=phase['pgid'], argv=phase['argv']), 'owned process registration')
    command = parse(bodies['evidence/readiness-tests.command.json'])
    require(command['argv'] == phase['argv'] and command['cwd'] == str(root)
            and command['env'] == c['environment'], 'original command/working directory/environment')
    for field in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        require(c['environment'][field] == '', 'GPU hidden during pure CPU gate')
    for key, suffix in (('command', '.command.json'), ('stdout', '.stdout'), ('stderr', '.stderr')):
        require(phase[key] == c['raw']['readiness-tests' + suffix], 'phase original stream pin')
    require(bodies['evidence/readiness-tests.stdout'] == b'', 'empty test stdout')
    names = []
    for module, cls in contract['modules'].items():
        tree = ast.parse(bodies[module + '.py'])
        classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
        require(len(classes) == 1 and classes[0].name == cls, 'exact synthetic class')
        names += [module + '.' + cls + '.' + node.name for node in classes[0].body
                  if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    names.sort()
    expression = r'^(test_[A-Za-z0-9_]+) \((test_readiness\.ReadinessTests|test_causal\.CausalTests)\.\1\) \.\.\. ok$'
    stderr = bodies['evidence/readiness-tests.stderr'].decode()
    observed = [prefix + '.' + name for name, prefix in re.findall(expression, stderr, re.M)]
    require(len(names) == len(set(names)) == len(observed) == len(set(observed)) == contract['count']
            and sorted(observed) == names, 'all exact source-declared synthetic tests passed')
    tail = '\n'.join(line for line in re.sub(expression, '', stderr, flags=re.M).splitlines() if line)
    require(re.fullmatch(r'-{70}\nRan ' + str(contract['count']) + r' tests in [0-9]+\.[0-9]+s\nOK', tail),
            'complete natural unittest summary')
    require(c['tests'] == dict(names=names, passed=contract['count'], failed=0, errors=0, skipped=0),
            'exact terminal census')
    require(c['synthetic_data_tests_only'] is True and all(c[k] is False for k in
            ('gpu_execution', 'native_parent_execution', 'model_execution', 'numerical_acceptance',
             'full_model_acceptance', 'performance_claim', 'production_authority')), 'CPU-only diagnostic scope')
    return c, {name: pin(body) for name, body in sorted(bodies.items())}


def export(mode, terminal_sha):
    root, archive_name, _ = locations(mode)
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'actual CPU host')
    output = E / archive_name
    require(not os.path.lexists(output) and root.resolve(strict=True) == root, 'fresh output and canonical root')
    require({p.name for p in root.iterdir()} == set(MODES[mode]['sources']) | {'evidence'}
            and {p.name for p in (root / 'evidence').iterdir()} == RAW | {'complete.json'}, 'closed live source/evidence tree')
    paths = {name: root / name for name in MODES[mode]['sources']}
    paths.update({'evidence/' + name: root / 'evidence' / name for name in RAW | {'complete.json'}})
    paths['pure-evidence.py'] = Path(__file__).resolve()
    bodies = {name: read(path) for name, path in paths.items()}
    c, pins = validate(mode, bodies, terminal_sha)
    for row in c['tool_pins'].values():
        require(pin(read(Path(row['path']), 16 << 20)) == compact(row), 'original Python/prlimit live tool pin')
    manifest = dict(schema='ferric-readiness40-causal-layer0-pure-cpu-export-v1', mode=mode, files=pins,
        original_files=15, selected_files=16, terminal=pin(bodies['evidence/complete.json']),
        actual_test_count=MODES[mode]['count'], synthetic_only=True, new_project_execution=False,
        native_execution=False, numerical_acceptance=False, performance_claim=False)
    packaged = dict(bodies, **{'manifest.json': encoded(manifest)})
    require(len(packaged) == 17 and sum(map(len, packaged.values())) <= TOTAL_LIMIT, 'bounded seventeen members')
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(packaged.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(raw), 0o644, 0
                tar.addfile(member, io.BytesIO(raw))
        stream.flush(); os.fsync(stream.fileno())
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'all selected body posthashes')
    for row in c['tool_pins'].values():
        require(pin(read(Path(row['path']), 16 << 20)) == compact(row), 'original tools posthash')
    print(json.dumps(dict(archive=dict(path=str(output), **pin(read(output, TOTAL_LIMIT))), members=17,
                         original_files=15, tests=MODES[mode]['count']), sort_keys=True))


def retain(mode, archive_sha, terminal_sha):
    _, archive_name, dest = locations(mode)
    archive = W / archive_name
    raw = read(archive, TOTAL_LIMIT)
    require(pin(raw)['sha256'] == archive_sha, 'observed archive SHA')
    expected = set(MODES[mode]['sources']) | {'evidence/' + n for n in RAW | {'complete.json'}} | {'pure-evidence.py', 'manifest.json'}
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for member in tar:
            require(member.name in expected and member.name not in bodies and member.isfile()
                    and not member.pax_headers and 0 <= member.size <= FILE_LIMIT, 'closed ordinary member')
            total += member.size
            require(total <= TOTAL_LIMIT, 'bounded expanded archive')
            bodies[member.name] = tar.extractfile(member).read(member.size + 1)
            require(len(bodies[member.name]) == member.size, 'complete original body')
    require(set(bodies) == expected, 'exact seventeen-member archive')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    c, pins = validate(mode, bodies, terminal_sha)
    require(bodies['pure-evidence.py'] == read(Path(__file__).resolve()) and manifest == dict(
        schema='ferric-readiness40-causal-layer0-pure-cpu-export-v1', mode=mode, files=pins,
        original_files=15, selected_files=16, terminal=pin(bodies['evidence/complete.json']),
        actual_test_count=MODES[mode]['count'], synthetic_only=True, new_project_execution=False,
        native_execution=False, numerical_acceptance=False, performance_claim=False), 'exact helper/export manifest')
    require(read(archive, TOTAL_LIMIT) == raw and dest.parent.resolve(strict=True) == dest.parent
            and not os.path.lexists(dest), 'archive posthash and fresh retention')
    bodies['manifest.json'] = manifest_raw
    dest.mkdir(mode=0o755)
    for name, body in sorted(bodies.items()):
        target = dest / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
        target.chmod(0o644)
        require(read(target) == body, 'retained original body readback')
    require(read(archive, TOTAL_LIMIT) == raw, 'archive unchanged after publication')
    report = dict(schema='ferric-readiness40-causal-layer0-pure-cpu-retention-v1', mode=mode,
        archive=pin(raw), files={name: pin(body) for name, body in sorted(bodies.items())}, original_files=15,
        tests_passed=MODES[mode]['count'], original_terminal_unchanged=True, external_tools_rehashed_locally=False,
        new_project_execution=False, native_execution=False, numerical_acceptance=False, performance_claim=False)
    with (dest / 'retention.json').open('xb') as stream:
        stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(dest), original_files=15, tests=MODES[mode]['count'])))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) in (4, 5),
            'python3 -B pure-evidence.py export MODE TERMINAL_SHA | retain MODE ARCHIVE_SHA TERMINAL_SHA')
    require(sys.argv[2] in MODES and all(re.fullmatch('[0-9a-f]{64}', value) for value in sys.argv[3:]), 'closed mode and observed hashes')
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 128 << 20), (resource.RLIMIT_FSIZE, TOTAL_LIMIT), (resource.RLIMIT_CORE, 0)):
        limits = resource.getrlimit(kind)
        value = min([cap] + [n for n in limits if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    def interrupted(number, _frame):
        raise RuntimeError('pure evidence signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, interrupted)
    signal.alarm(30)
    try:
        if sys.argv[1] == 'export' and len(sys.argv) == 4:
            export(sys.argv[2], sys.argv[3])
        else:
            require(sys.argv[1] == 'retain' and len(sys.argv) == 5, 'closed retention invocation')
            retain(sys.argv[2], sys.argv[3], sys.argv[4])
    finally:
        signal.alarm(0)
