"""Audit retained TF4 CPU evidence, install its exact source overlay, publish a compact ledger."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import tempfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
HELPER_SHA = 'af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9'
CONTROLLER_SHA = '6dac9b6fef7159d6906eab6582963f4931d7337b311ad5f0d30d1535455fe400'
PACKAGE = 'p228-projection-residual-decode-cpu-v1'
SOURCE_NAME = 'projection-residual-decode-cpu-inputs-v228-v2.json'
SOURCE_SHA = '012dfc4d04c4799aca2a90948bfb615656615eb24b9213599574d074355415b8'
OLD = 'projection-residual-runtime-cpu-v228-v1'
OLD_SHA = 'bf1a12f78981d9ff9b8157e1dec6dca300752b238680e16380b98e9d1260bafb'
OLD_CONTROLLER_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
BASE = 'da9f613224b81232776ebb37ad8a1b903dc0488a'
PROPOSALS = {
    'parent': ('p228-projection-residual-decode-parent-v1',
        '6deeeb84202536119ab594dd2c40c1e5d33f8dfbe210948f5dbb1ac30623148c'),
    'worker': ('p228-projection-residual-decode-worker-v1',
        'dcf3805d169131a52bb3f93aec920e45cde3d65d49cf3f6633e0e4f7651ac4f0'),
}
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
PARENT = 'ferric-qwen3-finite-projection-residual-decode-engineering'
OLD_BIN = 'ferric-qwen3-finite-projection-residual-layer-capture-engineering'
WIRE = 'finite_projection_residual_decode_wire_v1::tests::'
OLD_WIRE = 'finite_projection_residual_layer_wire_v1::tests::'
DIRECT = 'tp_finite_client::prefix_decode::projection::tests::'
FALSE = ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load_helper(repo):
    path = repo / 'qualification/native-device-routing-v1/publish.py'
    require(path.resolve(strict=True) == path and not path.is_symlink(), 'published data helper path')
    body = path.read_bytes()
    require(hashlib.sha256(body).hexdigest() == HELPER_SHA, 'published data helper identity')
    helper = types.ModuleType('decode_publication_data_helpers')
    helper.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), helper.__dict__)
    helper.verify(path, sha=HELPER_SHA)
    return helper


def live_preimage(H, path, expected):
    require(path.parent.resolve(strict=True) == path.parent, 'canonical source parent')
    if expected is None:
        require(not path.exists() and not path.is_symlink(), 'new source must be absent: ' + str(path))
        return None
    actual = H.fingerprint(path)
    require({k: actual[k] for k in ('bytes', 'sha256')} == expected, 'live source preimage: ' + str(path))
    return actual


def install(H, repo, staged):
    # Stage every complete body first; no existing source is truncated on disk exhaustion.
    temporary = []
    try:
        for row, body in staged:
            path = repo / row['path']
            live_preimage(H, path, row['before'])
            fd, name = tempfile.mkstemp(prefix='.' + path.name + '.decode-', dir=path.parent)
            temporary.append((Path(name), path, row))
            with os.fdopen(fd, 'wb') as stream:
                os.fchmod(stream.fileno(), stat.S_IMODE(path.stat().st_mode) if row['before'] else 0o644)
                stream.write(body)
                stream.flush()
                os.fsync(stream.fileno())
            actual = H.fingerprint(Path(name))
            require({k: actual[k] for k in ('bytes', 'sha256')} == row['compiled'], 'staged source bytes')
        for _, path, row in temporary:
            live_preimage(H, path, row['before'])
        for temp, path, row in temporary:
            live_preimage(H, path, row['before'])
            if row['before'] is None:
                os.link(temp, path)
                temp.unlink()
            else:
                os.replace(temp, path)
            fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(fd)
            finally:
                os.close(fd)
        return [dict(row, integrated=H.verify(repo / row['path'], row['compiled'])) for row, _ in staged]
    finally:
        for temp, _, _ in temporary:
            if temp.exists():
                temp.unlink()


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary verifier')
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('evidence', 'repo', 'cpu', 'cpu-sha', 'publish-to'):
        parser.add_argument('--' + name, required=True)
    a = parser.parse_args()
    local, repo, directory, destination = map(Path, (a.evidence, a.repo, a.cpu, a.publish_to))
    require(all(p.is_absolute() and p.resolve(strict=True) == p for p in (local, repo, directory)), 'canonical roots')
    require(destination == repo / 'qualification/projection-residual-decode-cpu-v1', 'closed publication destination')
    H = load_helper(repo)
    own = H.verify(Path(__file__).resolve(strict=True))
    if destination.exists():
        require(destination.resolve(strict=True) == destination and destination.is_dir()
                and {p.name for p in destination.iterdir()} <= {'README.md'}, 'fresh publication, optional README only')
        if (destination / 'README.md').exists():
            H.verify(destination / 'README.md')
    else:
        require(not destination.is_symlink() and destination.parent.resolve(strict=True) == destination.parent,
                'canonical publication parent')
    cpu = H.document(directory / 'complete.json', sha=a.cpu_sha)
    complete = H.CHECKED[str(directory / 'complete.json')]
    require(cpu['schema'] == 'ferric-projection-residual-decode-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and cpu['empty_initial_target'] is True
            and all(cpu[key] is False for key in FALSE), 'actual successful CPU-only qualification')
    remote = Path(H.filepin(cpu['raw']['sources-before.json'])['path']).parent
    require(remote.parent == E and re.fullmatch('projection-residual-decode-cpu-v228-v[1-9][0-9]*', remote.name),
            'qualified output namespace')
    inputs = {}
    for pin in cpu['inputs']:
        H.filepin(pin)
        require(pin['path'] not in inputs or inputs[pin['path']] == pin, 'conflicting repeated input')
        inputs[pin['path']] = pin
    controller = local / 'proposals' / PACKAGE / 'run.py'
    controller_body = H.read(controller, H.filepin(cpu['controller']), sha=CONTROLLER_SHA)
    require(cpu['controller']['path'] == str(E / PACKAGE / 'run.py'), 'actual executed controller')
    source_pin = inputs[str(E / SOURCE_NAME)]
    source = H.document(local / SOURCE_NAME, source_pin, sha=SOURCE_SHA)
    require(source['schema'] == 'ferric-projection-residual-decode-cpu-inputs-v1'
            and set(source['archives']) == {'ferric', 'fe2o3'} and set(source['proposals']) == set(PROPOSALS),
            'closed paired sources and overlay roles')
    prior = H.document(local / OLD / 'complete.json', cpu['prior_completion'], sha=OLD_SHA)
    require(cpu['prior_completion']['path'] == str(E / OLD / 'complete.json')
            and prior['passed'] is True and prior['error'] is None and not prior['postcheck_errors']
            and prior['tests_passed'] == 988 and prior['tests_ignored'] == 4, 'actual prior CPU988')
    prior_inputs = {v['path']: H.filepin(v) for v in prior['inputs']}
    initial = [cpu['controller'], source_pin]
    for path in (E / 'p228-host-policy-cpu-v1/run.py', E / 'run_clean_worker_p228_v1.py',
                 E.parent / 'wave-output-lowering-v216/bounded.py'):
        initial.append(prior_inputs[str(path)])
    require(prior['controller']['sha256'] == OLD_CONTROLLER_SHA, 'historical controller generation')
    initial += [prior['controller'], cpu['prior_completion']]
    old_raw = {}
    for name, pin in prior['raw'].items():
        require(H.filepin(pin)['path'] == str(E / OLD / name), 'direct prior raw member')
        old_raw[name] = H.read(local / OLD / name, pin)
    expected_inputs = initial + list(prior['raw'].values()) + list(source['archives'].values())
    proposals, rows = {}, []
    for role, pin in source['proposals'].items():
        label, digest = PROPOSALS[role]
        require(H.filepin(pin)['path'] == str(E / label / 'source-manifest.json'), 'proposal path')
        proposal = H.document(local / 'proposals' / label / 'source-manifest.json', pin, sha=digest)
        require(proposal['base_commit'] == BASE, 'original source base')
        proposals[role] = proposal
        expected_inputs.append(pin)
        prefix = 'adapters/' + ('tp-peer-finite-engineering-worker-v1' if role == 'worker' else 'm1-engineering-execution-v1') + '/'
        for row in proposal['files']:
            path = Path(row['path'])
            require(not path.is_absolute() and '..' not in path.parts and str(path) == row['path']
                    and row['source'] == 'draft/' + row['path'] and row['path'].startswith(prefix)
                    and (path.suffix == '.rs' or row['path'] == prefix + 'Cargo.toml'), 'closed overlay destination')
            H.verify(local / 'proposals' / label / row['source'], row['after'])
            expected_inputs.append(dict(path=str(E / label / row['source']), **row['after']))
            rows.append(row)
    require(len(rows) == len({row['path'] for row in rows}) == 21, 'disjoint 21-file overlay')

    commands = {}
    for name in prior['phases']:
        command = H.parse(old_raw[name + '-command.json'])
        command['argv'] = [v.replace(str(E / OLD), str(remote)) for v in command['argv']]
        command['env'] = {k: v.replace(str(E / OLD), str(remote)) for k, v in command['env'].items()}
        commands[name] = command
    fmt_files = [str(remote / 'sources/ferric' / row['path']) for row in rows if row['path'].endswith('.rs')]
    for name in ('rustfmt', 'rustfmt-check'):
        argv = commands[name]['argv']
        commands[name]['argv'] = argv[:argv.index('skip_children=true') + 1] + fmt_files
    formatter = prior_inputs[commands['rustfmt']['argv'][0]]
    require(formatter['sha256'] == '30de9e1efcd8f8fe7750e00d0c45ff8f4c480608ef1be5baf9ab6f1b4556e8f8', 'qualified formatter')
    expected_inputs.append(formatter)
    for suffix in ('-list', '-tests'):
        command = H.parse(json.dumps(commands[OLD_BIN + suffix]))
        command['argv'] = [v.replace(OLD_BIN, PARENT) for v in command['argv']]
        commands[PARENT + suffix] = command
    command = H.parse(json.dumps(commands['parent-projection-residual-wire']))
    command['argv'] = [WIRE if v == OLD_WIRE else v for v in command['argv']]
    commands['parent-projection-residual-decode-wire'] = command
    argv = commands['parent-builds']['argv']
    index = argv.index('--message-format=json')
    argv[index:index] = ['--bin', PARENT]
    tools_seen = set()
    for command in commands.values():
        for tool, digest in command['tools'].items():
            path = str(Path(command['env']['RUSTC']).parent / tool)
            if path not in tools_seen:
                require(prior_inputs[path]['sha256'] == digest, 'qualified tool pin')
                expected_inputs.append(prior_inputs[path]); tools_seen.add(path)
    require(cpu['inputs'] == expected_inputs, 'exact original input sequence')
    phases = set(commands)
    require(len(phases) == 87 and set(cpu['phases']) == phases, 'all 87 owned leaves')
    maps = {'sources-base.json', 'sources-unformatted.json', 'sources-before.json', 'sources-after.json',
            'old-targets-before.json', 'old-targets-after.json', 'configurations.json'}
    expected_raw = maps | {name + suffix for name in phases for suffix in H.SUFFIXES}
    require(set(cpu['raw']) == expected_raw, 'complete direct raw census')
    retained = {}
    for name, pin in cpu['raw'].items():
        require(H.filepin(pin)['path'] == str(remote / name), 'direct retained member')
        retained[name] = H.read(directory / name, pin)
    phase_summary = {}
    for name in sorted(phases):
        require(H.parse(retained[name + '-command.json']) == commands[name], 'exact command/environment: ' + name)
        started = H.parse(retained[name + '-started.json'])
        result = H.parse(retained[name + '-result.json'])
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'owned group identity')
        require(result == cpu['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30,
                'natural successful reaped leaf')
        require(all(result[s + '_sha256'] == cpu['raw'][name + '-' + s]['sha256'] for s in ('stdout', 'stderr')),
                'actual stdout/stderr hashes')
        phase_summary[name] = {kind: cpu['raw'][name + '-' + kind + '.json'] for kind in ('command', 'started', 'result')}
    require(H.parse(retained['old-targets-before.json']) == H.parse(retained['old-targets-after.json']), 'old target custody')

    base = H.parse(retained['sources-base.json'])
    previous = H.parse(old_raw['sources-after.json'])
    require(set(base) == set(previous), 'entire prior source file census')
    changed = {k: dict(before=previous[k], after=v) for k, v in base.items() if v != previous[k]}
    require(changed == source['documentation_changes'] and all(k.endswith('.md') for k in changed), 'declared documentation-only base delta')
    archive_sources, archive_summary = {}, {}
    for project, pin in source['archives'].items():
        path = Path(H.filepin(pin)['path'])
        require(path.parent == E, 'source archive namespace')
        H.verify(local / path.name, pin)
        files, archive_summary[project] = H.archive_map(local / path.name, project)
        require(not archive_sources.keys() & files.keys(), 'disjoint source archive roots')
        archive_sources.update(files)
    require(archive_sources == base, 'actual source archives reproduce full source base without extraction')
    unformatted = dict(base)
    for row in rows:
        key = 'ferric/' + row['path']
        require(unformatted.get(key) == row['before'], 'original source preimage')
        unformatted[key] = row['after']
    require(unformatted == H.parse(retained['sources-unformatted.json']), 'exact unformatted overlay')
    formatted = H.parse(retained['sources-before.json'])
    allowed = {'ferric/' + row['path'] for row in rows if row['path'].endswith('.rs')}
    require(len(allowed) == 20 and set(formatted) == set(unformatted)
            and all(formatted[k] == v for k, v in unformatted.items() if k not in allowed)
            and formatted == H.parse(retained['sources-after.json']), 'only declared Rust formatting; source unchanged through build')
    staged = []
    for row in rows:
        expected = formatted['ferric/' + row['path']]
        body = H.read(directory / 'sources/ferric' / row['path'], expected)
        live_preimage(H, repo / row['path'], row['before'])
        staged.append((dict(path=row['path'], before=row['before'], unformatted=row['after'], compiled=expected), body))

    inventory = {role: H.inventory(retained[stem + '-stdout'].decode()) for role, stem in
                 [('runtime', 'runtime-list'), ('worker', 'worker-list'), ('parent', 'parent-lib-list')]}
    old_inventory = {role: H.inventory(old_raw[stem + '-stdout'].decode()) for role, stem in
                     [('runtime', 'runtime-list'), ('worker', 'worker-list'), ('parent', 'parent-lib-list')]}
    additions = inventory['worker'] - old_inventory['worker']
    declared = proposals['worker']['authored_tests']
    require(inventory['runtime'] == old_inventory['runtime'] and old_inventory['worker'] <= inventory['worker']
            and declared['executed'] is False and len(additions) == declared['count'] == len(set(declared['names']))
            and {v.rsplit('::', 1)[-1] for v in additions} == set(declared['names']), 'exact unchanged runtime and declared worker additions')
    direct = set(proposals['parent']['test_census']['library'])
    new_bin = set(proposals['parent']['test_census']['binary'])
    wire = {name for name in additions if name.startswith(WIRE)}
    require(wire and direct and new_bin and all(v.startswith(DIRECT) for v in direct)
            and not old_inventory['parent'] & (direct | wire)
            and inventory['parent'] == old_inventory['parent'] | direct | wire, 'exact full parent library extension')
    expected_tests = {name: ('runtime', set(v['names'])) for name, v in prior['tests']['runtime'].items()}
    expected_tests['worker-tests'] = ('worker', inventory['worker'])
    for name, old in prior['tests']['parent'].items():
        if name in prior['binaries']:
            expected_tests[name + '-tests'] = ('parent-bin', set(old['names']))
        else:
            selector = commands[name]['argv'][commands[name]['argv'].index('--lib') + 1]
            names = {v for v in inventory['parent'] if selector in v}
            require(set(old['names']) <= names and names - set(old['names']) <= direct, 'retained parent selection')
            expected_tests[name] = ('parent', names)
    expected_tests['parent-projection-residual-decode-wire'] = ('parent', wire)
    expected_tests[PARENT + '-tests'] = ('parent-bin', new_bin)
    require(set(cpu['tests']) == set(expected_tests), 'closed flat actual test selections')
    old_ignored = H.outcomes(old_raw['worker-tests-stdout'].decode(), prior['tests']['worker'])
    totals, seen = {}, {'runtime': set(), 'parent': set()}
    for name, (role, names) in expected_tests.items():
        result = cpu['tests'][name]
        require(set(result['names']) == names, 'exact selected test names: ' + name)
        ignored = H.outcomes(retained[name + '-stdout'].decode(), result)
        require(ignored == (old_ignored if role == 'worker' else set()) and not ignored & additions, 'only historical worker ignores')
        if role == 'parent-bin':
            require(H.inventory(retained[name[:-6] + '-list-stdout'].decode()) == names, 'actual binary test listing')
        elif role != 'worker':
            require(names <= inventory[role] and not names & seen[role], 'disjoint selected library tests')
            seen[role].update(names)
        totals[name] = dict(passed=result['passed'], ignored=result['ignored'])
    require(direct | wire <= seen['parent'], 'all new parent tests executed')
    total = sum(v['passed'] for v in totals.values()); ignored_total = sum(v['ignored'] for v in totals.values())
    require(total == cpu['tests_passed'] and ignored_total == cpu['tests_ignored'] == prior['tests_ignored'], 'derived actual aggregate')

    require(set(cpu['binaries']) == set(prior['binaries']) | {PARENT} and len(cpu['binaries']) == 17, 'all 17 build artifact records')
    executable_pins = {}
    for role, phase in [('worker', 'worker-build'), ('parent', 'parent-builds')]:
        stream = [H.parse(line) for line in retained[phase + '-stdout'].splitlines() if line.startswith(b'{')]
        require([r['success'] for r in stream if r.get('reason') == 'build-finished'] == [True], 'Cargo build completion')
        artifacts = [r for r in stream if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        wanted = {WORKER} if role == 'worker' else set(cpu['binaries']) - {WORKER}
        require(len(artifacts) == len(wanted) and {r['target']['name'] for r in artifacts} == wanted, 'actual artifact census')
        source_dir = 'tp-peer-finite-engineering-worker-v1' if role == 'worker' else 'm1-engineering-execution-v1'
        for artifact in artifacts:
            name = artifact['target']['name']; row = cpu['binaries'][name]
            pin = H.filepin(row['binary'])
            require(artifact == row['artifact'] and artifact['target']['kind'] == ['bin']
                    and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
                    and artifact['executable'] == pin['path'] == str(remote / 'target' / role / 'debug' / name)
                    and artifact['manifest_path'] == str(remote / 'sources/ferric/adapters' / source_dir / 'Cargo.toml'),
                    'actual source-target-artifact identity')
            path = directory / 'target' / role / 'debug' / name
            executable_pins[name] = H.verify(path, pin)
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'retained executable is ELF')

    # All evidence and all live preimages must pass before any source or publication write.
    publisher_body = H.read(Path(__file__).resolve(strict=True), own)
    snapshot = dict(H.CHECKED)
    for path, pin in snapshot.items():
        H.verify(Path(path), pin)
    integrated = install(H, repo, staged)
    summary = dict(schema='ferric-p228-projection-residual-decode-cpu-publication-v1',
        cpu_receipt=dict(original=dict(path=str(remote / 'complete.json'), bytes=complete['bytes'], sha256=complete['sha256']), retained=complete),
        controller=cpu['controller'], publication_helper=own, source_manifest=source_pin, prior_cpu=cpu['prior_completion'],
        proposal_manifests=source['proposals'], source_maps={name: cpu['raw'][name] for name in sorted(maps)},
        source_archives=source['archives'], archive_census=archive_summary, formatted_source_files=integrated,
        selected_binaries={name: cpu['binaries'][name] for name in (WORKER, PARENT)},
        retained_binaries=executable_pins, all_built_binary_pins={name: row['binary'] for name, row in cpu['binaries'].items()},
        tests=totals, tests_passed=total, tests_ignored=ignored_total, added_worker_tests=sorted(additions),
        added_parent_tests=dict(library=sorted(direct | wire), binary=sorted(new_bin)), owned_phases=phase_summary,
        raw_members_rehashed=len(retained), prior_raw_members_rehashed=len(old_raw), local_file_hash_checks=len(snapshot),
        all_phase_commands_reconstructed=True, all_test_outcomes_replayed=True, all_21_compiled_sources_equal_live=True,
        live_preimages_checked=True, installation_atomicity='Per file, after staging all bodies; not a cross-file transaction.',
        unrelated_repo_paths_written=False, all_17_built_elf_bodies_rehashed=True, raw_records_copied=False,
        source_archives_rehashed=True, source_archive_maps_reconstructed=True, all_transitive_cpu_inputs_rehashed=False,
        external_library_bodies_rehashed=False, metadata_validation='Authenticated actual controller receipt; not independently replayed here.',
        parent_dependency_routing='Locked Git fe2o3, not the paired local runtime used by the worker.',
        qualification='Actual CPU tests/build and retained-byte source integration only; no native GPU run.',
        compiler_hsaco_reproduced=False, timestamp_calibration=False, full_model_acceptance=False,
        **{key: False for key in FALSE})
    destination.mkdir(mode=0o700, exist_ok=True)
    for name, body in [('controller.py', controller_body), ('publish.py', publisher_body),
                       ('result.json', (json.dumps(summary, indent=2, sort_keys=True) + '\n').encode())]:
        with (destination / name).open('xb') as stream:
            stream.write(body)
            stream.flush()
            os.fsync(stream.fileno())
    print(json.dumps(dict(result=H.fingerprint(destination / 'result.json'), tests_passed=total,
                          tests_ignored=ignored_total, source_files_installed=len(integrated), files_written=3)), flush=True)


if __name__ == '__main__':
    main()
