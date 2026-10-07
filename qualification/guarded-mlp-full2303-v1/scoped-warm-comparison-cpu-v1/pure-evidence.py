"""Closed data-only retention of the Full2303/scoped behavior comparison CPU gate."""
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
Q = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-full2303-v1')
SUPERVISOR = (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
MODES = {
    'comparison': dict(count=42, modules={'test_full': ('FullTests', 'BehaviorTests'), 'test_scoped_full': ('ScopedFullTests',), 'test_compare_scoped_full': ('ScopedBehaviorTests',)}, sources={
        'common.py': (3172, '445c602dd0d237beb2ebef62446d1d2b34283367b68529736a890640ae185256'),
        'compare_full.py': (13208, 'f6ba7ed47d93a5b03562cf22c730ebebd64ddc01afd1910acc0c51f8b3c875b1'),
        'compare_scoped_full.py': (6552, '6b22d458206759a0938893dbb61f4491316abc2bb1661a413dff5325a92f15de'),
        'diagnostics.py': (5084, '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf'),
        'full_announcement.py': (3246, '66e97079ac1df7c56db159c86aede157907fdb817464311d96c3333c803a0621'),
        'full_reference.py': (11342, '195ea81ab55e0523ca9b6a41680c654f7e99d7fb949b7d032a8f897d3620e471'),
        'reference.py': (11388, '466028fe6061de915bd852328030e9ea81a0b0874e4305563ecc78f0073b1d7b'),
        'run_cpu.py': (14891, 'e48ad8b5d005c1a202917b11eb5d368cbbd5887528e96aecd2be80eb55baee3c'),
        'supervisor.py': (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
        'test_compare_scoped_full.py': (14248, '2f07034c65804053fcb04eb1f8cfbf0bd7fd58c319244d34be5ff707fed1d925'),
        'test_full.py': (28057, '5acd7134ec8c4216a85a92d8629eb364e12d5af26fa8baf620ba87d19e56923f'),
        'test_scoped_full.py': (10154, '74b98271ac65c1597dee0b786a2b7645623c99f65486ce7e3ecebfbe59b6384d'),
        'validate_full.py': (22566, 'e5c3f144d09fda62fface222659197c519dc260afb68751dfaad6503912176c1'),
        'validate_scoped_full.py': (6696, '3e2eba79e769b12437958f510bd4255507ac834208529957a6544ddc5fbeeef9'),
    }),
}
RAW = {'sources-before.json', 'sources-after.json'} | {
    'admission-tests.' + suffix for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
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
    root = E / ('guarded-mlp-full2303-scoped-warm-' + mode + '-cpu-v228-v1')
    name = 'guarded-mlp-full2303-scoped-warm-' + mode + '-evidence-v228-v1.tar.gz'
    return root, name, Q / ('scoped-warm-' + mode + '-cpu-v1')


def terminal(bodies):
    names = [n for n in ('complete.json', 'failed.json') if 'evidence/' + n in bodies]
    require(len(names) == 1, 'one original outcome, never both or invented')
    return names[0]


def validate(mode, bodies, terminal_sha):
    contract = MODES[mode]
    root, _, _ = locations(mode)
    name = terminal(bodies)
    raw_terminal = bodies['evidence/' + name]
    require(pin(raw_terminal)['sha256'] == terminal_sha, 'observed original terminal SHA')
    c = parse(raw_terminal)
    require(type(c['passed']) is bool and c['schema'] == 'ferric-guarded-mlp-full2303-scoped-warm-comparison-cpu-v1'
            and (name == 'complete.json') == c['passed']
            and (c['failure'] is None) == c['passed']
            and (c['passed'] or type(c['failure']) is str)
            and type(c['postcheck_errors']) is list
            and all(type(x) is str for x in c['postcheck_errors']), 'honest original CPU outcome')
    require(type(c['raw']) is dict and set(c['raw']) <= RAW, 'closed original raw prefix')
    original = set(contract['sources']) | {'evidence/' + n for n in set(c['raw']) | {name}}
    require(set(bodies) == original | {'pure-evidence.py'}, 'exact original prefix and helper')
    require(set(c['sources_before']) == set(contract['sources']), 'original source roster')
    for source, expected in contract['sources'].items():
        require((len(bodies[source]), pin(bodies[source])['sha256']) == expected
                and compact(c['sources_before'][source]) == pin(bodies[source])
                and c['sources_before'][source]['path'] == str(root / source), 'fixed source-before join')
    after = c['sources_after']
    require(after is None or after == c['sources_before'], 'no changed source body relabeled as qualified')
    require(c['source_unchanged'] is (after == c['sources_before']), 'original source postcheck metadata')
    require(c['controller'] == c['sources_before']['run_cpu.py']
            and c['supervisor'] == c['sources_before']['supervisor.py'], 'harness source joins')
    for raw_name, row in c['raw'].items():
        require(pin(bodies['evidence/' + raw_name]) == compact(row)
                and row['path'] == str(root / 'evidence' / raw_name), 'raw pin/path join')
    for label in ('before', 'after'):
        raw_name = 'sources-' + label + '.json'
        if raw_name in c['raw']:
            require(parse(bodies['evidence/' + raw_name]) == c['sources_' + label], 'original source map body')
    require(type(c['phases']) is list and len(c['phases']) <= 1, 'at most one original owned child')
    phase = c['phases'][0] if c['phases'] else None
    phase_names = {'admission-tests.' + s for s in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
    if phase is not None:
        require(phase['label'] == 'admission-tests'
                and type(phase['exit_code']) in (int, type(None))
                and all(type(phase[k]) is bool for k in ('natural_exit', 'reaped', 'process_group_absent',
                    'forced_cleanup', 'timed_out')), 'original phase state, not success inference')
        for key, suffix in (('command', '.command.json'), ('stdout', '.stdout'), ('stderr', '.stderr')):
            raw_name = 'admission-tests' + suffix
            if raw_name in c['raw']:
                require(phase[key] == c['raw'][raw_name], 'original phase stream/command pin')
        if 'admission-tests.result.json' in c['raw']:
            require(parse(bodies['evidence/admission-tests.result.json']) == phase, 'original result join')
        if 'admission-tests.started.json' in c['raw']:
            require(parse(bodies['evidence/admission-tests.started.json']) ==
                    dict(pid=phase['pid'], pgid=phase['pgid'], argv=phase['argv']), 'original owned registration')
    elif 'admission-tests.result.json' in c['raw']:
        require(False, 'result body without recorded owned phase')
    if 'admission-tests.command.json' in c['raw']:
        command = parse(bodies['evidence/admission-tests.command.json'])
        require(command['cwd'] == str(root) and command['env'] == c['environment']
                and (phase is None or command['argv'] == phase['argv']), 'original command/environment')
    for field in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        require(c['environment'][field] == '', 'pure CPU GPU visibility')
    names = []
    for module, expected_classes in contract['modules'].items():
        tree = ast.parse(bodies[module + '.py'])
        classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
        require({node.name for node in classes} == set(expected_classes)
                and len(classes) == len(expected_classes), 'exact synthetic classes')
        names += [module + '.' + cls.name + '.' + node.name for cls in classes for node in cls.body
                  if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')]
    names.sort()
    require(len(names) == len(set(names)) == contract['count'], 'fixed42 source-declared test names')
    # Only an actually admitted census may be described as passed, even if a later postcheck failed.
    if c['tests'] is not None:
        require(phase is not None and phase['exit_code'] == 0 and phase['natural_exit'] is True
                and phase['reaped'] is True and phase['process_group_absent'] is True
                and phase['forced_cleanup'] is False and phase['timed_out'] is False
                and phase['exception'] is None and phase['storage_failure'] is None
                and phase['observed_signals'] == [] and phase_names <= set(c['raw']), 'census requires original clean child')
        require(bodies['evidence/admission-tests.stdout'] == b'', 'empty test stdout')
        expression = r'^(test_[A-Za-z0-9_]+) \((test_full\.FullTests|test_full\.BehaviorTests|test_scoped_full\.ScopedFullTests|test_compare_scoped_full\.ScopedBehaviorTests)\.\1\) \.\.\. ok$'
        stderr = bodies['evidence/admission-tests.stderr'].decode()
        observed = [prefix + '.' + test for test, prefix in re.findall(expression, stderr, re.M)]
        require(len(observed) == len(set(observed)) == contract['count']
                and sorted(observed) == names, 'all exact named tests passed')
        tail = '\n'.join(line for line in re.sub(expression, '', stderr, flags=re.M).splitlines() if line)
        require(re.fullmatch(r'-{70}\nRan 42 tests in [0-9]+\.[0-9]+s\nOK', tail), 'original unittest complete summary')
        require(c['tests'] == dict(names=names, passed=42, failed=0, errors=0, skipped=0), 'exact admitted test census')
    if c['passed']:
        require(c['postcheck_errors'] == [] and c['tests'] is not None and after == c['sources_before']
                and set(c['raw']) == RAW and len(bodies) == 23, 'successful22 originals plus helper')
    require(c['limits'] == dict(whole_seconds=180, test_seconds=120, cleanup_reserve_seconds=50,
            address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10), 'unchanged qualification bounds')
    require(c['synthetic_data_tests_only'] is True and all(c[k] is False for k in
            ('gpu_execution', 'native_parent_execution', 'model_execution', 'numerical_acceptance',
             'full_model_acceptance', 'performance_claim', 'production_authority')), 'pure CPU scope')
    require(set(c['tool_pins']) == {'python', 'prlimit'}, 'closed original tool metadata')
    return c, {name: pin(body) for name, body in sorted(bodies.items())}


def export_manifest(mode, c, bodies, pins):
    return dict(schema='ferric-full2303-scoped-warm-comparison-pure-cpu-export-v1', mode=mode, files=pins,
        original_files=len(bodies) - 1, selected_files=len(bodies), terminal_name=terminal(bodies),
        terminal=pin(bodies['evidence/' + terminal(bodies)]), passed=c['passed'],
        admitted_test_count=c['tests']['passed'] if c['tests'] is not None else None,
        original_failure=c['failure'], original_postcheck_errors=c['postcheck_errors'],
        synthetic_only=True, new_project_execution=False, native_execution=False,
        numerical_acceptance=False, performance_claim=False,
        original_tools_rehashed_at_export=True, external_tools_rehashed_locally=False)


def export(mode, terminal_sha):
    root, archive_name, _ = locations(mode)
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'actual CPU host')
    output = E / archive_name
    require(not os.path.lexists(output) and root.resolve(strict=True) == root, 'fresh output and canonical root')
    require({p.name for p in root.iterdir()} == set(MODES[mode]['sources']) | {'evidence'}, 'closed live source root')
    observed = {p.name for p in (root / 'evidence').iterdir()}
    outcomes = observed & {'complete.json', 'failed.json'}
    require(len(outcomes) == 1 and observed - outcomes <= RAW, 'one original outcome and closed raw prefix')
    paths = {name: root / name for name in MODES[mode]['sources']}
    paths.update({'evidence/' + name: root / 'evidence' / name for name in observed})
    paths['pure-evidence.py'] = Path(__file__).resolve()
    bodies = {name: read(path) for name, path in paths.items()}
    c, pins = validate(mode, bodies, terminal_sha)
    for row in c['tool_pins'].values():
        require(pin(read(Path(row['path']), 16 << 20)) == compact(row), 'original Python/prlimit live tool pin')
    manifest = export_manifest(mode, c, bodies, pins)
    packaged = dict(bodies, **{'manifest.json': encoded(manifest)})
    require(len(packaged) <= 24 and sum(map(len, packaged.values())) <= TOTAL_LIMIT, 'bounded twenty-four-member prefix')
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(packaged.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(raw), 0o644, 0
                tar.addfile(member, io.BytesIO(raw))
        stream.flush(); os.fsync(stream.fileno())
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'all selected body posthashes')
    for row in c['tool_pins'].values():
        require(pin(read(Path(row['path']), 16 << 20)) == compact(row), 'original tool posthash')
    print(json.dumps(dict(archive=dict(path=str(output), **pin(read(output, TOTAL_LIMIT))),
        members=len(packaged), original_files=len(bodies) - 1, passed=c['passed'],
        admitted_tests=manifest['admitted_test_count']), sort_keys=True))


def retain(mode, archive_sha, terminal_sha):
    _, archive_name, dest = locations(mode)
    archive = W / archive_name
    raw = read(archive, TOTAL_LIMIT)
    require(pin(raw)['sha256'] == archive_sha, 'observed archive SHA')
    allowed = set(MODES[mode]['sources']) | {'evidence/' + n for n in RAW | {'complete.json', 'failed.json'}} | {'pure-evidence.py', 'manifest.json'}
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for member in tar:
            require(member.name in allowed and member.name not in bodies and member.isfile()
                    and not member.pax_headers and 0 <= member.size <= FILE_LIMIT, 'closed ordinary member')
            total += member.size
            require(total <= TOTAL_LIMIT and len(bodies) < 24, 'bounded original archive prefix')
            bodies[member.name] = tar.extractfile(member).read(member.size + 1)
            require(len(bodies[member.name]) == member.size, 'complete original body')
    require('manifest.json' in bodies, 'original manifest present')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    c, pins = validate(mode, bodies, terminal_sha)
    require(bodies['pure-evidence.py'] == read(Path(__file__).resolve())
            and manifest == export_manifest(mode, c, bodies, pins), 'exact helper/export manifest')
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
        require(read(target) == body, 'retained original readback')
    require(read(archive, TOTAL_LIMIT) == raw, 'archive unchanged after publication')
    report = dict(schema='ferric-full2303-scoped-warm-comparison-pure-cpu-retention-v1', mode=mode,
        archive=pin(raw), files={name: pin(body) for name, body in sorted(bodies.items())},
        original_files=len(bodies) - 2, passed=c['passed'], admitted_tests=manifest['admitted_test_count'],
        original_terminal_unchanged=True, external_tools_rehashed_locally=False,
        new_project_execution=False, native_execution=False, numerical_acceptance=False, performance_claim=False)
    with (dest / 'retention.json').open('xb') as stream:
        stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(dest), original_files=len(bodies) - 2,
        passed=c['passed'], admitted_tests=manifest['admitted_test_count'])))


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
