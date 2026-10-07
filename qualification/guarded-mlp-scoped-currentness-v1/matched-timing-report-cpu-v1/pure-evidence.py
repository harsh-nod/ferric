"""Closed data-only export/retention of seven scoped matched parent-wall report tests."""
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
Q = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-scoped-currentness-v1')
SUPERVISOR = (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
MODES = {
    'report': dict(count=7, modules={
        'test_paired_analysis': ('PairedAnalysisTests',)}, sources={
        'paired_analysis.py': (19730, '1bc8cf810a70add1f485bad831e005c30af69739856ab72f65a265ae416f8226'),
        'run_cpu.py': (11393, '5f1ff87faeedd7eade3869c5c71219a1d2d2160e0454f0c80a6f3e571133d683'),
        'supervisor.py': (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
        'test_paired_analysis.py': (10610, 'eb429d078cb34d1b08472d0ae0ed9180ae1000f5d4dd5ebeb7d792554272d186'),
        'validate_readiness.py': (19800, '0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a'),
        'validate_timing.py': (10104, '27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f'),
    }),
}
RAW = {'sources-before.json', 'sources-after.json'} | {
    'report-tests.' + suffix for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
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
    root = E / ('guarded-mlp-readiness40-scoped-matched-timing-' + mode + '-cpu-v228-v1')
    name = 'guarded-mlp-readiness40-scoped-matched-timing-' + mode + '-evidence-v228-v1.tar.gz'
    return root, name, Q / ('matched-timing-' + mode + '-cpu-v1')


def terminal(bodies):
    names = [n for n in ('complete.json', 'failed.json') if 'evidence/' + n in bodies]
    require(len(names) == 1, 'one original outcome, never both or invented')
    return names[0]


def validate(mode, bodies, terminal_sha):
    contract = MODES[mode]
    require(all(value is not None for value in contract['sources'].values()),
            'actual report/controller source bindings required')
    root, _, _ = locations(mode)
    name = terminal(bodies)
    raw_terminal = bodies['evidence/' + name]
    require(pin(raw_terminal)['sha256'] == terminal_sha, 'observed original terminal SHA')
    c = parse(raw_terminal)
    require(type(c['passed']) is bool and c['schema'] == 'ferric-guarded-mlp-readiness40-scoped-matched-timing-report-cpu-v1'
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
    phase_names = {'report-tests.' + s for s in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
    if phase is not None:
        require(phase['label'] == 'report-tests'
                and type(phase['exit_code']) in (int, type(None))
                and all(type(phase[k]) is bool for k in ('natural_exit', 'reaped', 'process_group_absent',
                    'forced_cleanup', 'timed_out')), 'original phase state, not success inference')
        for key, suffix in (('command', '.command.json'), ('stdout', '.stdout'), ('stderr', '.stderr')):
            raw_name = 'report-tests' + suffix
            if raw_name in c['raw']:
                require(phase[key] == c['raw'][raw_name], 'original phase stream/command pin')
        if 'report-tests.result.json' in c['raw']:
            require(parse(bodies['evidence/report-tests.result.json']) == phase, 'original result join')
        if 'report-tests.started.json' in c['raw']:
            require(parse(bodies['evidence/report-tests.started.json']) ==
                    dict(pid=phase['pid'], pgid=phase['pgid'], argv=phase['argv']), 'original owned registration')
    elif 'report-tests.result.json' in c['raw']:
        require(False, 'result body without recorded owned phase')
    if 'report-tests.command.json' in c['raw']:
        command = parse(bodies['evidence/report-tests.command.json'])
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
    require(len(names) == len(set(names)) == contract['count'], 'fixed7 source-declared test names')
    # Only an actually admitted census may be described as passed, even if a later postcheck failed.
    if c['tests'] is not None:
        require(phase is not None and phase['exit_code'] == 0 and phase['natural_exit'] is True
                and phase['reaped'] is True and phase['process_group_absent'] is True
                and phase['forced_cleanup'] is False and phase['timed_out'] is False
                and phase['exception'] is None and phase['storage_failure'] is None
                and phase['observed_signals'] == [] and phase_names <= set(c['raw']), 'census requires original clean child')
        require(bodies['evidence/report-tests.stdout'] == b'', 'empty test stdout')
        expression = r'^(test_[A-Za-z0-9_]+) \((test_paired_analysis\.PairedAnalysisTests)\.\1\) \.\.\. ok$'
        stderr = bodies['evidence/report-tests.stderr'].decode()
        observed = [prefix + '.' + test for test, prefix in re.findall(expression, stderr, re.M)]
        require(len(observed) == len(set(observed)) == contract['count']
                and sorted(observed) == names, 'all exact named tests passed')
        tail = '\n'.join(line for line in re.sub(expression, '', stderr, flags=re.M).splitlines() if line)
        require(re.fullmatch(r'-{70}\nRan 7 tests in [0-9]+\.[0-9]+s\nOK', tail), 'original unittest complete summary')
        require(c['tests'] == dict(names=names, passed=7, failed=0, errors=0, skipped=0), 'exact admitted test census')
    if c['passed']:
        require(c['postcheck_errors'] == [] and c['tests'] is not None and after == c['sources_before']
                and set(c['raw']) == RAW and len(bodies) == 15, 'successful14 originals plus helper')
    require(c['limits'] == dict(whole_seconds=180, test_seconds=120, cleanup_reserve_seconds=50,
            address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10), 'unchanged qualification bounds')
    require(c['synthetic_data_tests_only'] is True and all(c[k] is False for k in
            ('actual_pair_report_rendered', 'gpu_execution', 'native_parent_execution', 'model_execution', 'numerical_acceptance',
             'full_model_acceptance', 'performance_claim', 'production_authority')), 'pure CPU scope')
    require(set(c['tool_pins']) == {'python', 'prlimit'}, 'closed original tool metadata')
    return c, {name: pin(body) for name, body in sorted(bodies.items())}


def export_manifest(mode, c, bodies, pins):
    return dict(schema='ferric-readiness40-scoped-matched-timing-report-pure-cpu-export-v1', mode=mode, files=pins,
        original_files=len(bodies) - 1, selected_files=len(bodies), terminal_name=terminal(bodies),
        terminal=pin(bodies['evidence/' + terminal(bodies)]), passed=c['passed'],
        admitted_test_count=c['tests']['passed'] if c['tests'] is not None else None,
        original_failure=c['failure'], original_postcheck_errors=c['postcheck_errors'],
        synthetic_only=True, actual_pair_report_rendered=False, new_project_execution=False, native_execution=False,
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
    require(len(packaged) <= 16 and sum(map(len, packaged.values())) <= TOTAL_LIMIT, 'bounded sixteen-member prefix')
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
            require(total <= TOTAL_LIMIT and len(bodies) < 16, 'bounded original archive prefix')
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
    report = dict(schema='ferric-readiness40-scoped-matched-timing-report-pure-cpu-retention-v1', mode=mode,
        archive=pin(raw), files={name: pin(body) for name, body in sorted(bodies.items())},
        original_files=len(bodies) - 2, passed=c['passed'], admitted_tests=manifest['admitted_test_count'],
        original_terminal_unchanged=True, external_tools_rehashed_locally=False,
        actual_pair_report_rendered=False, new_project_execution=False, native_execution=False, numerical_acceptance=False, performance_claim=False)
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
