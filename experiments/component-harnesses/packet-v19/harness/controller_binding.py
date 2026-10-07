"""Current55c baseline controller evidence, independent of native GPU admission."""
import hashlib
import io
from pathlib import Path
import re
import stat
import tarfile

QUALIFIER_SHA = '5d30ac68ac5c6cc0a046764e30accb74117cd077b934d5c83f3b3f617a5663b1'
V19_QUALIFIER_SHA = '0660dc79d28000e78f972fe4dc4460f63271671875526fa43539c329de21fe7f'
RUNTIME_BINDER_SHA = '0a5853b0544ec658f1c433532a50b1d5bd87fbf1c06170ab53ae05c88caf5ece'
HELPERS = {'qualifier': QUALIFIER_SHA,
           'a005': '5f29d7dd42f913334e9467465a674ac94ec05aaa6f4e6c187d3927421de0c354',
           'g36': 'b84d2f83eaffd7adf4f9aa760f88dd44d7fe4b2a58c23ea8a57d7beb3cfd0297'}
ROLES = ('stage', 'format', 'timestamps', 'graph', 'parser', 'source-policy',
         'clippy', 'release', 'retention')
TEST_COUNTS = {'timestamps': 32, 'graph': 3, 'parser': 4, 'source-policy': 58}
ALLOWANCES = dict(zip(ROLES, (128, 16, 512, 32, 192, 640, 192, 512, 64)))
PROFILE = 'FerricCpuFourCore36GiBEmitterV1'
BASE_SHA = '6089abea6ebc3363557c8e5fb8b4e2e488fa49b4f30f99029dfcc16b7c5bddd7'
BASE_ROSTER_SHA = '62893e35d35f8e959f7bc9e37eb59694bacdeee99a1dec2f48114c65088073c8'
MODES_SHA = 'df0b500263af3bda000767756d9f2339e1267cd38068e8cd6aa7516c528ffbed'
EVIDENCE_FIELDS = set(HELPERS) | {'base_source', 'source_roster', 'source_modes'}


def archive_state(raw, require):
    require(type(raw) is bytes and 0 < len(raw) <= 64 * 1024**2, 'bounded source archive')
    files, modes, total = {}, {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for ordinal, member in enumerate(archive, 1):
            relative = Path(member.name)
            require(ordinal <= 10000 and not relative.is_absolute()
                    and '..' not in relative.parts, 'safe bounded archive member')
            name = str(relative)
            require(member.name in (name, './' + name, name + '/', './' + name + '/'),
                    'canonical archive member')
            if member.isdir():
                continue
            require(member.isfile() and name != '.' and name not in files
                    and 0 <= member.size <= 32 * 1024**2, 'distinct regular source member')
            total += member.size
            require(total <= 96 * 1024**2, 'bounded source expansion')
            value = archive.extractfile(member).read(member.size + 1)
            require(len(value) == member.size, 'complete member bytes')
            files[name] = hashlib.sha256(value).hexdigest()
            modes[name] = stat.S_IMODE(member.mode)
    require(len(files) == 1433, 'complete 1433-file client source')
    return files, modes


def test_counts(c, name, raw):
    c.require(type(raw) is bytes and raw, 'actual raw test output required')
    if name.startswith(('packet-', 'v19-')) and name not in c.PYTHON_COMMANDS:
        role = name.split('-', 1)[1]
        c.require(role in TEST_COUNTS, 'counted Rust role required')
        rows = re.findall(rb'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',
                          raw, re.MULTILINE)
        c.require(len(rows) == 1, 'exact single Rust suite')
        passed, failed, ignored, measured, filtered = map(int, rows[0])
        c.require((passed, failed, ignored, measured) == (TEST_COUNTS[role], 0, 0, 0),
                  'actual exact expected count with no skipped or failed tests')
        if role == 'source-policy':
            c.require(filtered == 0, 'complete source-policy suite')
        return {'passed': passed, 'failed': 0, 'ignored': 0, 'measured': 0, 'filtered_out': filtered}
    c.require(name in c.PYTHON_COMMANDS, 'counted Python role required')
    rows = re.findall(rb'^Ran (\d+) tests? in [0-9.]+s$', raw, re.MULTILINE)
    c.require(len(rows) == 1 and int(rows[0]) == c.PYTHON_COUNTS[name]
              and re.search(rb'^OK\n?\Z', raw, re.MULTILINE),
              'complete unskipped Python fixture footer')
    return {'passed': int(rows[0]), 'failed': 0, 'ignored': 0}


def validate_harness_sources(c, cpu, read_raw):
    evidence = cpu['harness_evidence']
    c.require(type(evidence) is dict and set(evidence) == {'archive', 'helper'},
              'closed actual tested harness evidence')
    for item in evidence.values():
        c.binding(item)
    raw = read_raw(evidence['archive']['path'], evidence['archive']['sha256'], 1024**2)[0]
    wanted = {'harness/' + name: sha for name, sha in cpu['harness_sources'].items()}
    wanted.update(cpu['external_sources'])
    files, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for member in archive:
            c.require(member.name in wanted and member.name not in files and member.isfile()
                      and 0 < member.size <= 512 * 1024, 'closed bounded harness member')
            total += member.size
            c.require(total <= 2 * 1024**2, 'bounded harness source expansion')
            value = archive.extractfile(member).read(member.size + 1)
            c.require(len(value) == member.size and hashlib.sha256(value).hexdigest() == wanted[member.name],
                      'exact tested source content')
            files[member.name] = value
    c.require(set(files) == set(wanted), 'complete actual harness archive')
    original = files['prepare_qualification.py']
    c.require(original.count(b'ENABLED = False\n') == 1, 'one disabled archive qualifier')
    helper = read_raw(evidence['helper']['path'], evidence['helper']['sha256'], 1024**2)[0]
    c.require(helper == original.replace(b'ENABLED = False\n', b'ENABLED = True\n', 1),
              'external helper is enabled-only exact source copy')
    return wanted


def validate_harness_inner(c, name, inner, cpu, sources):
    directory, pattern = c.PYTHON_COMMANDS[name]
    argv = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
            '-s', directory, '-p', pattern, '-v']
    wanted = {'schema': 'FerricV19PacketHarnessCustodyV1', 'action': name, 'argv': argv,
              'source_before': sources, 'source_after': sources, 'source_root': c.HARNESS_ROOT,
              'archive_sha256': cpu['harness_evidence']['archive']['sha256'],
              'helper_sha256': cpu['harness_evidence']['helper']['sha256'],
              'authored_test_count': c.PYTHON_COUNTS[name], 'returncode': 0,
              'native_executed': False, 'native_qualified': False}
    c.require(type(inner) is dict and set(inner) == set(wanted) | {'allocation_before', 'allocation_after'}
              and all(type(inner[key]) is type(value) and inner[key] == value for key, value in wanted.items()),
              'exact wrapped Python source custody and command')
    for name in ('allocation_before', 'allocation_after'):
        value = inner[name]
        planned = 64 * 1024**2 if name == 'allocation_before' else 0
        c.require(type(value) is dict and set(value) == {'stage_allocated_bytes', 'stage_cap_bytes',
                  'stage_reserve_bytes', 'remaining_reserved_bytes', 'planned_increment_bytes'}
                  and all(type(item) is int for item in value.values())
                  and value['stage_cap_bytes'] == 38654705664 and value['stage_reserve_bytes'] == 536870912
                  and value['planned_increment_bytes'] == planned
                  and 0 <= value['stage_allocated_bytes'] <= 38117834752 - planned
                  and value['remaining_reserved_bytes'] == 38117834752 - value['stage_allocated_bytes'],
                  'fixed Python fixture envelope and reserve')
    c.require(inner['allocation_after']['stage_allocated_bytes'] - inner['allocation_before']['stage_allocated_bytes']
              <= 64 * 1024**2, 'observed Python growth bound')


def runtime_evidence(c, value, build, read_raw):
    runtime = c.module(Path(__file__).with_name('runtime_binding.py'), RUNTIME_BINDER_SHA)
    c.require(type(value) is dict and set(value) == set(runtime.PINS), 'exact current runtime evidence roster')
    c.require(build['worker']['sha256'] == runtime.WORKER_SHA
              and build['runtime_source']['sha256'] == runtime.PINS['source-archive'],
              'current qualified55c runtime artifacts')
    worker = read_raw(build['worker']['path'], runtime.WORKER_SHA, 32 * 1024**2)[0]
    c.require(len(worker) == runtime.WORKER_BYTES and worker.startswith(b'\x7fELF'), 'actual current worker ELF')
    for name, digest in runtime.PINS.items():
        item = c.binding(value[name])
        c.require(item['sha256'] == digest, 'exact existing runtime evidence hash')
        raw = read_raw(item['path'], digest, 64 * 1024**2)[0]
        if name.endswith('-status'):
            c.require(raw == b'0\n', 'current runtime raw status')
        if name.endswith('-result'):
            result = c.decode(raw)
            clean = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
                     'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
                     'log_limit_exceeded': False}
            c.require(all(type(result.get(key)) is type(wanted) and result[key] == wanted
                          for key, wanted in clean.items()), 'current runtime clean terminal result')
        if name == 'full-stdout':
            c.require(b'test result: ok. 734 passed; 0 failed; 3 ignored;' in raw
                      and b'test result: ok. 9 passed; 0 failed; 0 ignored;' in raw,
                      'actual current runtime and worker tests')


def validate_sources(c, evidence, build, read_bound, read_raw):
    c.require(type(evidence) is dict and set(evidence) == EVIDENCE_FIELDS, 'closed controller source evidence')
    for value in evidence.values():
        c.binding(value)
    for name, digest in HELPERS.items():
        c.require(evidence[name]['sha256'] == digest, 'exact qualification helper dependency')
        read_raw(evidence[name]['path'], digest, 1024**2)
    qualifier = c.module(evidence['qualifier']['path'], QUALIFIER_SHA)
    c.require(tuple(qualifier.ROLES) == ROLES and qualifier.TEST_COUNTS == TEST_COUNTS
              and qualifier.ALLOWANCES == ALLOWANCES and qualifier.REVISION == c.RUNTIME_MAIN
              and qualifier.BASE_SHA == BASE_SHA and qualifier.BINARY == c.PACKET_BINARY,
              'exact fixed qualifier contract')
    c.require(evidence['base_source']['sha256'] == BASE_SHA
              and evidence['source_modes']['sha256'] == MODES_SHA, 'exact A005 base and modes')
    before, before_modes = archive_state(read_raw(evidence['base_source']['path'], BASE_SHA, 64 * 1024**2)[0], c.require)
    c.require(hashlib.sha256(c.encoded(before)).hexdigest() == BASE_ROSTER_SHA,
              'exact complete A005 base file roster')
    expected = dict(before)
    for name, (old, new) in qualifier.OVERLAYS.items():
        c.require(expected[name] == old, 'exact source overlay preimage')
        expected[name] = new
    source_raw = read_raw(build['controller_source']['path'], build['controller_source']['sha256'], 64 * 1024**2)[0]
    after, modes = archive_state(source_raw, c.require)
    c.require((after, modes) == (expected, before_modes), 'exact three-file source overlay and unchanged modes')
    c.require(read_bound(evidence['source_roster']) == after
              and hashlib.sha256(c.encoded(after)).hexdigest() == evidence['source_roster']['sha256']
              and read_bound(evidence['source_modes']) == modes, 'exact retained source and mode rosters')
    return qualifier, evidence['source_roster']['sha256'], len(source_raw)


def validate_inner(c, name, inner, qualifier, source_sha):
    role = name.removeprefix('packet-')
    c.require(role in ROLES and type(inner) is dict, 'closed controller role')
    wanted = {'schema': 'FerricBaselinePacket55cCpuRoleV1', 'role': role,
              'source': str(qualifier.S), 'source_files': 1433, 'source_roster_sha256': source_sha,
              'base_archive_sha256': BASE_SHA, 'runtime_revision': c.RUNTIME_MAIN,
              'helper_sha256': QUALIFIER_SHA, 'profile': PROFILE, 'returncode': 0,
              'source_verified_after': True, 'native_executed': False, 'latency_sample_admitted': False,
              'commands': qualifier.commands(role), 'planning_allowance_bytes': ALLOWANCES[role] * 1024**2}
    c.require(all(type(inner.get(key)) is type(value) and inner[key] == value for key, value in wanted.items()),
              'exact current controller inner qualification')
    c.require(inner.get('overlays') == {name: list(pair) for name, pair in qualifier.OVERLAYS.items()},
              'exact declared source overlays')
    allowed = set(wanted) | {'overlays', 'allocation_before', 'allocation_after', 'artifacts', 'scope'}
    c.require(set(inner) == allowed and type(inner['artifacts']) is dict
              and inner['scope'] == 'CPU qualification only; prior test roles bind actual raw passing counts',
              'closed controller qualification receipt')
    for key in ('allocation_before', 'allocation_after'):
        value = inner[key]
        c.require(type(value) is dict and set(value) == {'stage_allocated_bytes', 'stage_cap_bytes',
                  'stage_reserve_bytes', 'remaining_reserved_bytes', 'planned_increment_bytes'}
                  and all(type(item) is int for item in value.values())
                  and value['stage_cap_bytes'] == 38654705664
                  and value['stage_reserve_bytes'] == 536870912
                  and value['planned_increment_bytes'] == (ALLOWANCES[role] * 1024**2 if key == 'allocation_before' else 0)
                  and 0 <= value['stage_allocated_bytes'] <= 38117834752 - value['planned_increment_bytes']
                  and value['remaining_reserved_bytes'] == 38117834752 - value['stage_allocated_bytes'],
                  'exact fresh bounded per-role resource envelope')
    c.require(inner['allocation_after']['stage_allocated_bytes'] - inner['allocation_before']['stage_allocated_bytes']
              <= ALLOWANCES[role] * 1024**2, 'observed role growth bound')


def validate_retention(c, inners, phases, build, source_bytes, read_raw):
    release = inners['packet-release']['artifacts']
    retained = inners['packet-retention']['artifacts']
    c.require(set(release) == {'controller'} and set(retained) == {'controller', 'source_archive', 'phases'},
              'closed actual release and retention artifacts')
    for artifact in (release['controller'], retained['controller'], retained['source_archive']):
        c.require(type(artifact) is dict and set(artifact) == {'path', 'sha256', 'size_bytes'}
                  and type(artifact['size_bytes']) is int and 0 < artifact['size_bytes'] <= 64 * 1024**2,
                  'bounded actual retained artifact')
        c.sha(artifact['sha256'])
    binary = build['controllers']['diagnostic']
    raw = read_raw(binary['path'], binary['sha256'], 32 * 1024**2)[0]
    c.require(raw.startswith(b'\x7fELF') and len(raw) == release['controller']['size_bytes']
              == retained['controller']['size_bytes'] and binary['sha256'] == release['controller']['sha256']
              == retained['controller']['sha256'], 'same actual dedicated release and retained ELF')
    c.require(release['controller']['path'] == c.D + '/target-fence-client-a001/release/' + c.PACKET_BINARY
              and retained['controller']['path'] == c.D + '/client-packet-baseline-55c-a004/retained/' + c.PACKET_BINARY
              and retained['source_archive']['path'] == c.D + '/client-packet-baseline-55c-a004/retained/ferric-packet-baseline-55c1a9b6-a004.tar.gz'
              and retained['source_archive']['sha256'] == build['controller_source']['sha256']
              and retained['source_archive']['size_bytes'] == source_bytes, 'retained source and artifact paths/identities')
    c.require(set(retained['phases']) == set(ROLES[:-1]), 'all prior roles retained without substitutions')
    for role, value in retained['phases'].items():
        phase = phases['packet-' + role]
        c.require(type(value) is dict and set(value) == {'inner', 'result', 'stdout_sha256', 'stderr_sha256'},
                  'closed final phase custody')
        for key in ('inner', 'result'):
            c.binding(value[key])
            c.require(value[key]['sha256'] == phase[key]['sha256'], 'same retained exact role receipt')
        for key in ('stdout', 'stderr'):
            c.require(value[key + '_sha256'] == phase[key]['sha256'], 'same retained exact raw output')


def validate_v19_evidence(c, evidence, source_sha, read_raw):
    c.require(type(evidence) is dict and set(evidence) == {'qualifier', 'baseline_controller'},
              'closed V19 helper and independently qualified baseline ELF')
    for value in evidence.values():
        c.binding(value)
    item = evidence['qualifier']
    c.require(item['sha256'] == V19_QUALIFIER_SHA, 'exact reviewed enabled V19 helper')
    read_raw(item['path'], V19_QUALIFIER_SHA, 1024**2)
    value = c.module(item['path'], V19_QUALIFIER_SHA)
    c.require(value.SOURCE_ROSTER_SHA == source_sha and value.BASE_HELPER_SHA == QUALIFIER_SHA
              and value.BINARY == c.V19_BINARY
              and value.ROLES == ('graph', 'parser', 'clippy', 'release', 'retention')
              and value.TEST_COUNTS == {'graph': 3, 'parser': 4}
              and value.ALLOWANCES == {'graph': 32, 'parser': 192, 'clippy': 192, 'release': 512, 'retention': 64},
              'same A004 source and closed incremental V19 qualification')
    return value


def phase_custody(c, observed, roles, phases, prefix):
    c.require(type(observed) is dict and set(observed) == set(roles), 'complete exact reused phase roster')
    for role, value in observed.items():
        phase = phases[prefix + role]
        c.require(type(value) is dict and set(value) == {'inner', 'result', 'stdout_sha256', 'stderr_sha256'},
                  'closed reused phase custody')
        for name in ('inner', 'result'):
            c.binding(value[name])
            c.require(value[name]['sha256'] == phase[name]['sha256'], 'exact reused raw role receipt')
        for name in ('stdout', 'stderr'):
            c.require(value[name + '_sha256'] == phase[name]['sha256'], 'exact reused raw output')


def validate_v19_inner(c, name, inner, qualifier, base, phases):
    role = name.removeprefix('v19-')
    c.require(role in qualifier.ROLES, 'known V19 role')
    wanted = {'schema': 'FerricV19Packet55cCpuRoleV1', 'role': role, 'source': str(base.S),
        'source_files': 1433, 'source_roster_sha256': qualifier.SOURCE_ROSTER_SHA,
        'runtime_revision': c.RUNTIME_MAIN, 'helper_sha256': V19_QUALIFIER_SHA,
        'base_helper_sha256': QUALIFIER_SHA, 'profile': PROFILE,
        'commands': qualifier.commands(base, role), 'returncode': 0, 'source_verified_after': True,
        'planning_allowance_bytes': qualifier.ALLOWANCES[role] * 1024**2,
        'reused_test_roles': {'timestamps': 32, 'source-policy': 58},
        'native_executed': False, 'latency_sample_admitted': False,
        'scope': 'same A004 source and features; V19 kernel composition, not native publication timing'}
    c.require(type(inner) is dict and set(inner) == set(wanted) | {
        'allocation_before', 'allocation_after', 'artifacts', 'baseline_phases'}
        and all(type(inner[key]) is type(value) and inner[key] == value for key, value in wanted.items())
        and type(inner['artifacts']) is dict, 'exact V19 source/features/shared-test qualification')
    phase_custody(c, inner['baseline_phases'], ROLES, phases, 'packet-')
    allowance = qualifier.ALLOWANCES[role] * 1024**2
    for key in ('allocation_before', 'allocation_after'):
        value = inner[key]
        planned = allowance if key == 'allocation_before' else 0
        c.require(type(value) is dict and set(value) == {'stage_allocated_bytes', 'stage_cap_bytes',
                  'stage_reserve_bytes', 'remaining_reserved_bytes', 'planned_increment_bytes'}
                  and all(type(item) is int for item in value.values())
                  and value['stage_cap_bytes'] == 38654705664 and value['stage_reserve_bytes'] == 536870912
                  and value['planned_increment_bytes'] == planned
                  and 0 <= value['stage_allocated_bytes'] <= 38117834752 - planned
                  and value['remaining_reserved_bytes'] == 38117834752 - value['stage_allocated_bytes'],
                  'exact V19 incremental envelope and reserve')
    c.require(inner['allocation_after']['stage_allocated_bytes'] - inner['allocation_before']['stage_allocated_bytes']
              <= allowance, 'observed V19 incremental growth')


def validate_v19_retention(c, inners, phases, build, read_raw):
    released = inners['v19-release']['artifacts']
    retained = inners['v19-retention']['artifacts']
    c.require(set(released) == {'controller'} and set(retained) == {'controller', 'source_archive', 'phases'},
              'closed V19 release and final artifacts')
    binary = build['controllers']['diagnostic']
    raw = read_raw(binary['path'], binary['sha256'], 32 * 1024**2)[0]
    c.require(raw.startswith(b'\x7fELF'), 'actual V19 retained ELF')
    for artifact, path in ((released['controller'], c.D + '/target-fence-client-a001/release/' + c.V19_BINARY),
                           (retained['controller'], c.D + '/client-packet-v19-55c-a001/retained/' + c.V19_BINARY)):
        c.require(type(artifact) is dict and set(artifact) == {'path', 'sha256', 'size_bytes'}
                  and type(artifact['size_bytes']) is int and artifact == {
                      'path': path, 'sha256': binary['sha256'], 'size_bytes': len(raw)},
                  'same actual V19 release and retained ELF')
    c.require(retained['source_archive'] == inners['packet-retention']['artifacts']['source_archive']
              and retained['source_archive']['sha256'] == build['controller_source']['sha256'],
              'reuse exact immutable A004 full source archive, no relabeling')
    phase_custody(c, retained['phases'], ('graph', 'parser', 'clippy', 'release'), phases, 'v19-')
