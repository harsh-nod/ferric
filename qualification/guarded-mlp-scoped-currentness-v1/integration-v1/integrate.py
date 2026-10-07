"""Success-only data integration plan; never import retained code or apply a patch."""
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

RT = Path('/home/harsh/fe2o3-p228-runtime')
F = Path('/home/harsh/ferric-p227-integration')
REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-scoped-currentness-cpu-v228-v3'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
CONTROLLER = {'bytes': 55965, 'sha256': 'e8270de7cedfcba4fa9587aee41011f6f99d060437f3e2eac89e0027388e716f'}
SCHEMA = 'ferric-scoped-currentness-v3-qualified-source-integration-v1'
HELPERS = {'run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py'}
FIXTURES = {'fe2o3/Cargo.toml', 'fe2o3/Cargo.lock',
            'fe2o3/Cargo.toml.original', 'fe2o3/Cargo.lock.input'}
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'runtime-metadata', 'default-check',
          'kfd-tests-build', 'kfd-list', 'kfd-ignored', 'kfd-tests', 'terminal-pair-tests',
          'peer-read-pair-tests', 'arena-reuse-tests', 'arena-memory-tests', 'retained-tests',
          'mixed-bank-tests', 'scoped-checkpoint-tests', 'scoped-group-tests',
          'scoped-layer-tests', 'interface-doc-list', 'interface-doc-tests', 'metadata',
          'worker-tests-build', 'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')
MAX_ARCHIVE = 64 << 20
READSET = {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def compact(row):
    value = {key: row[key] for key in ('bytes', 'sha256')}
    require(type(value['bytes']) is int and value['bytes'] >= 0
            and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']),
            'valid compact pin')
    return value


def parse(raw):
    def pairs(items):
        out = {}
        for name, value in items:
            require(name not in out, 'duplicate JSON key')
            out[name] = value
        return out
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def ordinary(name):
    return type(name) is str and name not in ('', '.') and not Path(name).is_absolute() \
        and Path(name).as_posix() == name and '..' not in Path(name).parts


def read(path, cap=32 << 20):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical ordinary input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= cap, 'bounded regular input')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat())
            and len(raw) == before.st_size, 'input drift')
    actual = pin(raw)
    require(str(path) not in READSET or READSET[str(path)] == actual, 'readset changed')
    READSET[str(path)] = actual
    return raw


def posthash():
    for name, expected in list(READSET.items()):
        require(pin(read(Path(name), MAX_ARCHIVE)) == expected, 'posthash: ' + name)


def literal(tree, name):
    nodes = [n for n in tree.body if isinstance(n, ast.Assign)
             and any(isinstance(t, ast.Name) and t.id == name for t in n.targets)]
    require(len(nodes) == 1, 'unique pinned source literal')
    return ast.literal_eval(nodes[0].value)


def canonical(name):
    require(ordinary(name), 'ordinary source name')
    if name.startswith('fe2o3/'):
        return RT / name.removeprefix('fe2o3/')
    require(name.startswith(WORKER), 'closed Ferric worker boundary')
    return F / name.removeprefix('ferric/')


def named(raw):
    text = raw.decode()
    for name in ('payload_release_failure_after_event_destroy_is_process_terminal',
                 'unpublished_custody_cleanup_failure_is_process_terminal'):
        full = 'queue_linux::tests::' + name
        text, count = re.subn('^' + re.escape('test ' + full + ' ... \nrunning 1 test\nok')
                             + r'(?=\n|\Z)', 'test ' + full + ' ... ok', text, flags=re.M)
        require(count <= 1, 'known nested child output')
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)
    require(len(rows) == len(dict(rows)), 'unique named outcomes')
    return dict(rows)


def admit(archive, archive_sha, terminal_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha)
            and re.fullmatch('[0-9a-f]{64}', terminal_sha), 'observed SHA arguments')
    raw = read(archive, MAX_ARCHIVE)
    require(pin(raw)['sha256'] == archive_sha, 'observed archive')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 207
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers
                        and 0 <= m.size <= 32 << 20 for m in members)
                and sum(m.size for m in members) <= MAX_ARCHIVE, 'closed bounded207 archive')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete archive bodies')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    require(manifest['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-retained-v1'
            and manifest['files'] == {n: pin(b) for n, b in bodies.items()}
            and manifest['terminal_name'] == 'complete.json'
            and manifest['passed'] is True and manifest['failure'] is None
            and manifest['full_live_source_and_dependency_posthash'] is True,
            'authenticated successful retention closure')
    result = parse(bodies['evidence/complete.json'])
    require(pin(bodies['evidence/complete.json']) == manifest['terminal']
            and manifest['terminal']['sha256'] == terminal_sha
            and result['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-v1'
            and result['source_generation'] == 'scoped-currentness-coupled-v3'
            and result['passed'] is True and result['failure'] is None
            and result['postcheck_errors'] == [] and result['source_unchanged'] is True
            and result['input_sources'] == result['final_sources']
            and all(result[k] is True for k in ('full_runtime_tests_executed', 'full_worker_tests_executed',
                                                'selected_facade_doctests_executed'))
            and all(result[k] is False for k in ('gpu_execution', 'gpu_qualified', 'numerical_acceptance',
                'performance_claim', 'production_authority', 'scoped_currentness_native_execution',
                'currentness_temporal_equivalence_claim')), 'success-only CPU source authority')
    require(pin(bodies['run_cpu.py']) == CONTROLLER == compact(result['controller']), 'frozen V3 controller')
    tree = ast.parse(bodies['run_cpu.py'])
    inputs = parse(bodies['input-manifest.json'])
    require(pin(bodies['input-manifest.json']) == compact(result['input_manifest'])
            and inputs['source_generation'] == result['source_generation']
            and inputs['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-input-v1',
            'actual input generation')
    history = literal(tree, 'HISTORY')
    props = {n: literal(tree, k) for n, k in (
        ('runtime-proposal.json', 'RUNTIME_PROPOSAL_SHA'),
        ('primitive-proposal.json', 'PRIMITIVE_PROPOSAL_SHA'),
        ('consumer-proposal.json', 'CONSUMER_PROPOSAL_SHA'),
        ('consumer-repair.json', 'CONSUMER_REPAIR_SHA'),
        ('selector-proposal.json', 'SELECTOR_PROPOSAL_SHA'),
        ('selector-repair.json', 'SELECTOR_REPAIR_SHA'))}
    require(set(inputs['lineage']) == set(result['readset']) == set(history) | set(props)
            and len(inputs['lineage']) == 16, 'sixteen direct lineage bodies')
    for name, expected in inputs['lineage'].items():
        require(pin(bodies['inputs/' + name]) == expected == compact(result['readset'][name]),
                'original lineage body')
        require(expected == {'bytes': history[name][0], 'sha256': history[name][1]} if name in history
                else expected['sha256'] == props[name], 'qualified predecessor/source proposal')
    rp, pp, cp, cr, sp, sr = [parse(bodies['inputs/' + n]) for n in (
        'runtime-proposal.json', 'primitive-proposal.json', 'consumer-proposal.json',
        'consumer-repair.json', 'selector-proposal.json', 'selector-repair.json')]
    require(rp['primitive_manifest'] == pin(bodies['inputs/primitive-proposal.json'])
            and compact(cr['predecessor_manifest']) == pin(bodies['inputs/consumer-proposal.json'])
            and compact(sr['predecessor_manifest']) == pin(bodies['inputs/selector-proposal.json'])
            and compact(sp['requires']['consumer']) == compact(sr['requires']['consumer'])
                == pin(bodies['inputs/consumer-proposal.json'])
            and compact(sp['requires']['runtime']) == pin(bodies['inputs/runtime-proposal.json']),
            'original dependencies and explicit cumulative repairs')
    base = parse(bodies['inputs/worker-sources.json'])
    runtime_base = parse(bodies['inputs/runtime-sources.json'])
    for role, current in (('worker', base), ('runtime', runtime_base)):
        old = parse(bodies['inputs/' + role + '-complete.json'])
        require(old['passed'] is True and old['input_sources'] == old['final_sources'] == current,
                'actual qualified baseline source map')
    base = {n: compact(v) for n, v in base.items() if n.startswith(('fe2o3/', WORKER))}
    require(len(base) == 1020 and sum(n.startswith(WORKER) for n in base) == 205
            and {n: v for n, v in base.items() if n.startswith('fe2o3/')}
                == {n: compact(v) for n, v in runtime_base.items() if n.startswith('fe2o3/')},
            '815 runtime plus205 worker baseline')
    before = {n: compact(v) for n, v in result['preformat_sources'].items()}
    final = {n: compact(v) for n, v in result['final_sources'].items()}
    expected, overlays = dict(base), {}
    for proposal, prefix, count in ((rp, 'fe2o3/', 22), (cr, 'ferric/', 4), (sr, 'ferric/', 9)):
        require(len(proposal['files']) == count, 'cumulative row census')
        for row in proposal['files']:
            name = prefix + row['path']
            require(ordinary(row['path']) and name.endswith('.rs') and name not in overlays
                    and expected.get(name) == row['before'], 'canonical preimage/absence chain')
            canonical(name)
            expected[name], overlays[name] = compact(row['after']), row
    expected.update({n: pin(bodies[n]) for n in HELPERS})
    require(expected == before == inputs['files'] and len(final) == len(before) == 1033
            and set(final) == set(before) and inputs['overlay'] == sorted(overlays)
            and len(overlays) == 35 and sum(r['before'] is None for r in overlays.values()) == 9
            and sum(n.startswith('fe2o3/') for n in final) == 820
            and sum(n.startswith(WORKER) for n in final) == 209, 'complete source composition')
    require(all(v['path'] == REMOTE + '/' + n for n, v in result['final_sources'].items()),
            'source paths bound to V3')
    changed = {n for n in before if before[n] != final[n]}
    require(changed <= set(overlays) and changed == set(result['format_changed_paths'])
            and len(result['format_changed_paths']) == len(changed), 'only allowed rustfmt changes')
    selected = set(overlays) | HELPERS | FIXTURES | {
        'fe2o3/rust-toolchain.toml', WORKER + 'Cargo.toml', WORKER + 'Cargo.lock'}
    require(len(selected) == 46 and all(pin(bodies[n]) == final[n] for n in selected),
            'actual formatted retained bodies')
    require(parse(bodies['evidence/sources-after.json']) == result['final_sources']
            and parse(bodies['evidence/sources-before.json']) == result['input_sources']
            and parse(bodies['evidence/sources-preformat.json']) == result['preformat_sources'],
            'original source snapshots')
    rawpins = result['raw']
    require(len(rawpins) == 137 and all(ordinary(n) and '/' not in n
            and pin(bodies['evidence/' + n]) == compact(v) for n, v in rawpins.items()), '137 original raw bodies')
    require(set(bodies) == selected | {'inputs/' + n for n in inputs['lineage']}
            | {'evidence/' + n for n in rawpins} | {'evidence/complete.json', 'input-manifest.json',
                'transport.py', 'stage-complete.json', 'cargo-cache-manifest.json',
                'cargo-cache-stage-complete.json', 'exporter.py'}, '206 exact selected bodies')
    require([p['label'] for p in result['phases']] == list(PHASES), 'all26 ordered phases')
    for phase in result['phases']:
        label = phase['label']
        saved, command, started = [parse(bodies['evidence/' + label + suffix])
                                   for suffix in ('.result.json', '.command.json', '.started.json')]
        require(saved == phase and phase['exit_code'] == 0 and phase['natural_exit'] is True
                and phase['reaped'] is True and phase['process_group_absent'] is True
                and phase['forced_cleanup'] is False and phase['timed_out'] is False
                and phase['exception'] is None and phase['storage_failure'] is None
                and phase['observed_signals'] == []
                and command['argv'] == started['argv'] == phase['argv']
                and started['pid'] == started['pgid'] == phase['pid'] == phase['pgid'],
                'clean original owned lifecycle')
    runtime_names = literal(tree, 'RUNTIME_NEW_NAMES')
    worker_names = literal(tree, 'WORKER_NEW_NAMES')
    require(len(runtime_names) == 24 and worker_names == sr['composed_new_tests']
            and sum(map(len, worker_names.values())) == 22, 'same declared additions')
    for role, label, additions, passed, ignored in (
        ('runtime', 'kfd-tests', runtime_names, 1143, 8),
        ('worker', 'worker-tests', [n for ns in worker_names.values() for n in ns], 726, 4)):
        old = named(bodies['inputs/' + role + '-tests.stdout'])
        actual = named(bodies['evidence/' + label + '.stdout'])
        value = result['tests'][label]
        require(not set(old).intersection(additions) and len(additions) == len(set(additions))
                and actual == old | {n: 'ok' for n in additions}
                and actual == {r['name']: r['outcome'] for r in value['named']}
                and result['inventories'][role] == sorted(actual)
                and (value['passed'], value['failed'], value['ignored']) == (passed, 0, ignored)
                and sum(v == 'ok' for v in actual.values()) == passed
                and sum(v == 'ignored' for v in actual.values()) == ignored, 'all old and new outcomes')
    require(result['tests']['interface-doc-tests']['passed'] == 10
            and result['doc_parser_tests']['passed'] == 8 and result['doc_parser_tests']['failed'] == 0,
            'docs and parser gates')
    artifacts = result['artifacts']
    roles = {'kfd-lib', 'engineering-worker-test', 'guarded-facade-test', 'debug-trap-test',
             'telemetry-env-test', 'telemetry-test', 'worker-lib', 'worker-bin-test',
             'worker-readiness-test', 'worker-wire-test', 'worker'}
    require(set(artifacts) == roles and len({v['pin']['path'] for v in artifacts.values()}) == 11
            and set(manifest['artifact_presence']) == roles
            and all(v is True for v in manifest['artifact_presence'].values())
            and result['cli_executable_unchanged_across_tests'] is True
            and result['cli_executable_before_tests']['pin'] == artifacts['worker']['pin'],
            'eleven preserved products and real CLI identity')
    for role, value in artifacts.items():
        label = 'worker-build' if role == 'worker' else 'worker-tests-build' if role.startswith('worker-') else 'kfd-tests-build'
        records = [parse(line) for line in bodies['evidence/' + label + '.stdout'].splitlines()
                   if line.startswith(b'{')]
        record = value['cargo_artifact']
        require(records.count(record) == 1 and record['executable'] == value['pin']['path']
                and record['reason'] == 'compiler-artifact', 'actual Cargo selected product')
    return bodies, result, inputs, base, final, overlays, pin(manifest_raw)


def patch(rows):
    out = ['*** Begin Patch']
    for path, before, after in rows:
        require(after.endswith(b'\n') and (before is None or before.endswith(b'\n')), 'newline-terminated Rust source')
        if before is None:
            out += ['*** Add File: ' + str(path)]
        else:
            out += ['*** Update File: ' + str(path), '@@']
            out += ['-' + line for line in before.decode()[:-1].split('\n')]
        out += ['+' + line for line in after.decode()[:-1].split('\n')]
    return ('\n'.join(out + ['*** End Patch']) + '\n').encode()


def emit(archive, archive_sha, terminal_sha, destination):
    require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
            and not os.path.lexists(destination)
            and not any(destination.is_relative_to(root) for root in (RT, F)), 'fresh output outside repositories')
    bodies, result, inputs, base, final, overlays, manifest = admit(archive, archive_sha, terminal_sha)
    require(RT.resolve(strict=True) == RT and F.resolve(strict=True) == F, 'canonical repository roots')
    expected = {n: v for n, v in final.items() if n not in FIXTURES | HELPERS}
    require(len(expected) == 1025 and sum(n.startswith('fe2o3/') for n in expected) == 816
            and sum(n.startswith(WORKER) for n in expected) == 209, 'full relevant source extent')
    verification, patches = {}, {'runtime.patch': [], 'worker.patch': []}
    for name, after in sorted(expected.items()):
        path, before = canonical(name), base.get(name)
        if before is None:
            require(name in overlays and not os.path.lexists(path), 'new canonical source absent')
            raw = None
        else:
            raw = read(path)
            require(pin(raw) == before, 'exact entire canonical preimage: ' + name)
        verification[str(path)] = {'before': before, 'after': after}
        if name in overlays:
            require(overlays[name]['before'] == before and pin(bodies[name]) == after, 'actual tested overlay')
            patches['runtime.patch' if name.startswith('fe2o3/') else 'worker.patch'].append((path, raw, bodies[name]))
        else:
            require(before == after, 'no unlisted canonical change')
    for alias, target in (('Cargo.toml.original', 'Cargo.toml'), ('Cargo.lock.input', 'Cargo.lock')):
        require(not os.path.lexists(RT / alias), 'qualification-only alias absent in canonical')
        identity = final['fe2o3/' + alias]
        require(identity == base['fe2o3/' + alias] == pin(bodies['fe2o3/' + alias])
                == pin(read(RT / target)), 'original canonical Cargo identity')
        verification[str(RT / target)] = {'before': identity, 'after': identity}
    require(len(verification) == 1027 and len(patches['runtime.patch']) == 22
            and len(patches['worker.patch']) == 13, '35 rows and complete1027 identity map')
    patch_bodies = {name: patch(rows) for name, rows in patches.items()}
    report = {'schema': SCHEMA, 'passed': True, 'mode': 'emit-only', 'archive': pin(read(archive, MAX_ARCHIVE)),
              'terminal': pin(bodies['evidence/complete.json']),
              'source_map': pin(bodies['evidence/sources-after.json']), 'generation': result['source_generation'],
              'source_rows': verification, 'overlay_paths': sorted(str(canonical(n)) for n in overlays),
              'fixture_exclusions': {n: final[n] for n in sorted(FIXTURES)},
              'aliases_must_remain_absent': [str(RT / n) for n in ('Cargo.toml.original', 'Cargo.lock.input')],
              'lineage': inputs['lineage'], 'manifest': manifest,
              'patches': {n: pin(b) for n, b in patch_bodies.items()},
              'emitter': pin(read(Path(__file__).resolve())),
              'runtime_nonfixture_files': 816, 'worker_files': 209, 'original_cargo_identities': 2,
              'canonical_changed': False, 'postcheck_completed': False, 'project_execution': False,
              'native_execution': False, 'numerical_acceptance': False,
              'currentness_temporal_equivalence_claim': False, 'performance_claim': False}
    posthash()
    require(all(not os.path.lexists(Path(p)) for p, r in verification.items() if r['before'] is None),
            'new paths remained absent through preparation')
    destination.mkdir(mode=0o700)
    for name, body in {**patch_bodies, 'integration.json': encoded(report)}.items():
        with (destination / name).open('xb') as stream:
            stream.write(body)
        require(pin(read(destination / name)) == pin(body), 'emitted bytes exact')
    print(json.dumps({'destination': str(destination), 'files': {p.name: pin(read(p))
          for p in sorted(destination.iterdir())}, 'canonical_changed': False}, sort_keys=True))


def check(map_path, map_sha):
    raw = read(map_path)
    require(pin(raw)['sha256'] == map_sha, 'observed integration map')
    report = parse(raw)
    require(report['schema'] == SCHEMA and report['mode'] == 'emit-only' and report['passed'] is True
            and report['generation'] == 'scoped-currentness-coupled-v3'
            and len(report['source_rows']) == 1027 and len(report['overlay_paths']) == 35
            and report['emitter'] == pin(read(Path(__file__).resolve()))
            and set(report['patches']) == {'runtime.patch', 'worker.patch'},
            'closed original integration plan')
    counts = {'runtime': 0, 'worker': 0, 'cargo': 0}
    for name, row in report['source_rows'].items():
        path = Path(name)
        if path in (RT / 'Cargo.toml', RT / 'Cargo.lock'):
            counts['cargo'] += 1
        elif path.is_relative_to(RT) and path not in (RT / 'Cargo.toml.original', RT / 'Cargo.lock.input'):
            counts['runtime'] += 1
        else:
            require(path.is_relative_to(F / WORKER.removeprefix('ferric/')), 'closed postcheck path')
            counts['worker'] += 1
        require(pin(read(path)) == compact(row['after']), 'full canonical postimage: ' + name)
    require(counts == {'runtime': 816, 'worker': 209, 'cargo': 2}
            and report['aliases_must_remain_absent'] == [str(RT / n)
                for n in ('Cargo.toml.original', 'Cargo.lock.input')]
            and all(not os.path.lexists(n) for n in report['aliases_must_remain_absent']),
            'complete relevant postcheck and no fixture aliases')
    for name, expected in report['patches'].items():
        require(name in ('runtime.patch', 'worker.patch')
                and pin(read(map_path.parent / name)) == expected, 'original emitted patch')
    posthash()
    print(json.dumps({'schema': SCHEMA, 'mode': 'read-only-postcheck', 'passed': True,
                      'integration_map': pin(raw), 'counts': counts,
                      'canonical_changed_by_this_command': False, 'native_execution': False}, sort_keys=True))


if __name__ == '__main__':
    signal.alarm(180)
    resource.setrlimit(resource.RLIMIT_AS, (768 << 20, 768 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (64 << 20, 64 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    if len(sys.argv) == 6 and sys.argv[1] == 'emit':
        emit(Path(sys.argv[2]), sys.argv[3], sys.argv[4], Path(sys.argv[5]))
    elif len(sys.argv) == 4 and sys.argv[1] == 'check':
        check(Path(sys.argv[2]), sys.argv[3])
    else:
        raise SystemExit('emit ARCHIVE ARCHIVE_SHA TERMINAL_SHA FRESH_OUT | check MAP MAP_SHA')
