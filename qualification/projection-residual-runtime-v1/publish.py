"""Compact retained-byte CPU publication; no tested-controller or native execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
HELPER_SHA = 'af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9'
CONTROLLER_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
SOURCE_SHA = '4dfcbd11140098cb432bd6e44ee017fb631ca30bc2de1400ffa8c397f2bb02b4'
PRIOR = {
    'worker': ('gfx950-clock-recorder-cpu-v228-v1',
        '41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d'),
    'parent': ('layer0-native-capture-cpu-v228-v2',
        '78c12f822e95d50a1239f56411c5b8da9411a3fbda7510fbbf38d5644e1dffbb'),
}
PROPOSALS = {
    'worker': ('p228-projection-residual-worker-v1',
        'b3278f3bc86dcee8ef03b8d23d4456280d5b4b7502294549faf3ae4aa7f71049'),
    'parent': ('p228-projection-residual-parent-v1',
        'a415fef2075cfe00c7402b18cf87413a212f43bbdc1672c6e788bd77ebe9772d'),
}
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
PARENT = 'ferric-qwen3-finite-projection-residual-layer-capture-engineering'
WIRE = 'finite_projection_residual_layer_wire_v1::tests::'
DIRECT = 'tp_finite_client::prefix_layer::projection_capture::tests::'
FALSE = ('compiler_hsaco_reproduced', 'gpu_execution', 'numerical_acceptance',
         'performance_claim', 'timestamp_calibration', 'production_authority')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load_helper(repo):
    path = repo / 'qualification/native-device-routing-v1/publish.py'
    require(path.resolve(strict=True) == path and not path.is_symlink(), 'published data helper path')
    body = path.read_bytes()
    require(hashlib.sha256(body).hexdigest() == HELPER_SHA, 'published data helper identity')
    helper = types.ModuleType('joint_publication_data_helpers')
    helper.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), helper.__dict__)
    helper.verify(path, sha=HELPER_SHA)
    return helper


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary verifier')
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('evidence', 'repo', 'cpu', 'cpu-sha', 'worker-elf', 'parent-elf', 'publish-to'):
        parser.add_argument('--' + name, required=True)
    a = parser.parse_args()
    local, repo, directory, destination = map(Path, (a.evidence, a.repo, a.cpu, a.publish_to))
    H = load_helper(repo)
    own = H.verify(Path(__file__).resolve(strict=True))
    require(not destination.exists() or destination.resolve(strict=True) == destination,
            'canonical publication directory')
    if destination.exists():
        require(destination.is_dir() and {p.name for p in destination.iterdir()} <= {'README.md'},
                'fresh publication, optional root README only')
        if (destination / 'README.md').exists():
            H.verify(destination / 'README.md')
    else:
        require(destination.parent.resolve(strict=True) == destination.parent, 'canonical publication parent')
    cpu = H.document(directory / 'complete.json', sha=a.cpu_sha)
    complete = H.CHECKED[str(directory / 'complete.json')]
    require(cpu['schema'] == 'ferric-p228-projection-residual-runtime-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and cpu['empty_initial_target'] is True
            and cpu['external_cargo_cache_reused'] is True
            and cpu['parent_rebuilt'] is True and cpu['worker_rebuilt'] is True,
            'actual completed joint qualification')
    require(all(cpu[key] is False for key in FALSE), 'CPU-only claim boundary')
    remote = Path(H.filepin(cpu['raw']['sources-before.json'])['path']).parent
    require(remote.parent == E and re.fullmatch('projection-residual-runtime-cpu-v228-v[1-9][0-9]*', remote.name),
            'qualified output namespace')
    inputs = {}
    for pin in cpu['inputs']:
        H.filepin(pin)
        require(pin['path'] not in inputs or inputs[pin['path']] == pin, 'conflicting repeated input')
        inputs[pin['path']] = pin
    controller = local / 'proposals/p228-projection-residual-runtime-cpu-v1/run.py'
    controller_body = H.read(controller, H.filepin(cpu['controller']), sha=CONTROLLER_SHA)
    require(cpu['controller']['path'] == str(E / 'p228-projection-residual-runtime-cpu-v1/run.py'),
            'actual executed controller, not the unexecuted successor')
    source_pin = H.filepin(cpu['source_manifest'])
    source_manifest = H.document(local / 'projection-residual-runtime-source-inputs-v228-v1.json',
                                 source_pin, sha=SOURCE_SHA)
    require(source_manifest['schema'] == 'ferric-p228-projection-residual-runtime-sources-v1'
            and set(source_manifest['archives']) == {'ferric', 'fe2o3'}, 'paired source manifest')
    priors, proposals = {}, {}
    for role in ('worker', 'parent'):
        old_label, digest = PRIOR[role]
        priors[role] = H.document(local / old_label / 'complete.json', cpu['prior_' + role + '_cpu'], sha=digest)
        require(cpu['prior_' + role + '_cpu']['path'] == str(E / old_label / 'complete.json')
                and priors[role]['passed'] is True, 'actual prior qualification')
        label, digest = PROPOSALS[role]
        proposals[role] = H.document(local / 'proposals' / label / 'source-manifest.json',
                                     cpu['proposal_manifests'][role], sha=digest)
        require(cpu['proposal_manifests'][role]['path'] == str(E / label / 'source-manifest.json'),
                'exact original proposal identity')
    require(source_manifest['qualified_parent'] == cpu['prior_parent_cpu'], 'qualified source generation')
    for pin in [cpu['controller'], cpu['source_manifest'], *cpu['proposal_manifests'].values(),
                cpu['prior_parent_cpu'], cpu['prior_worker_cpu'], *source_manifest['archives'].values()]:
        require(inputs.get(pin['path']) == pin, 'direct receipt input join')

    phases = (set(priors['worker']['phases']) - {'metadata'}) | {'worker-metadata'}
    phases |= set(priors['parent']['phases']) | {'rustfmt', 'rustfmt-check',
        'parent-projection-residual-wire', PARENT + '-list', PARENT + '-tests'}
    require(len(phases) == 84 and set(cpu['phases']) == phases, 'all 84 owned leaves')
    maps = {'sources-base.json', 'sources-unformatted.json', 'sources-before.json', 'sources-after.json',
            'old-targets-before.json', 'old-targets-after.json', 'configurations.json'}
    expected_raw = maps | {name + suffix for name in phases for suffix in H.SUFFIXES}
    require(set(cpu['raw']) == expected_raw, 'complete 427-member retained raw roster')
    retained = {}
    for name, pin in cpu['raw'].items():
        require(H.filepin(pin)['path'] == str(remote / name), 'direct retained member')
        retained[name] = H.read(directory / name, pin)
    commands = {}
    for role, prior in priors.items():
        old_root = str(Path(prior['raw']['sources-before.json']['path']).parent)
        for name in prior['phases']:
            old_pin = prior['raw'][name + '-command.json']
            command = H.document(local / PRIOR[role][0] / (name + '-command.json'), old_pin)
            command = H.relocated(command, old_root, str(remote))
            command = H.relocated(command, str(remote / 'target'), str(remote / 'target' / role))
            command['env']['CARGO_TARGET_DIR'] = str(remote / 'target' / role)
            key = 'worker-metadata' if role == 'worker' and name == 'metadata' else name
            commands[key] = command
    commands['parent-builds']['argv'][-1:-1] = ['--bin', PARENT]
    wire_command = H.parse(json.dumps(commands['parent-lib-list']))
    wire_command['argv'] = wire_command['argv'][:wire_command['argv'].index('--lib') + 1] + [WIRE, '--', '--test-threads=2']
    commands['parent-projection-residual-wire'] = wire_command
    old_bin = 'ferric-qwen3-finite-prefix-layer-capture-engineering'
    for suffix in ('-list', '-tests'):
        command = H.parse(json.dumps(commands[old_bin + suffix]))
        command['argv'][command['argv'].index('--bin') + 1] = PARENT
        commands[PARENT + suffix] = command
    rows = [row for role in ('worker', 'parent') for row in proposals[role]['files']]
    require(len(rows) == len({row['path'] for row in rows}) == 22, 'paired source overlay census')
    formatter = next(pin for pin in inputs.values() if pin['path'].endswith('/bin/rustfmt'))
    require(formatter['sha256'] == '30de9e1efcd8f8fe7750e00d0c45ff8f4c480608ef1be5baf9ab6f1b4556e8f8',
            'qualified formatter input')
    rustfmt_argv = [formatter['path'], '--edition', '2024', '--config', 'skip_children=true',
                   *[str(remote / 'sources/ferric' / row['path']) for row in rows if row['path'].endswith('.rs')]]
    for name, argv in [('rustfmt', rustfmt_argv), ('rustfmt-check', [rustfmt_argv[0], '--check', *rustfmt_argv[1:]])]:
        command = H.parse(json.dumps(commands['parent-metadata']))
        command.update(argv=argv, deadline_seconds=60)
        commands[name] = command
    phase_summary = {}
    for name in sorted(phases):
        command = H.parse(retained[name + '-command.json'])
        require(command == commands[name], 'exact command/environment: ' + name)
        started = H.parse(retained[name + '-started.json'])
        result = H.parse(retained[name + '-result.json'])
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'owned group identity')
        require(result == cpu['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30,
                'natural successful reaped leaf')
        require(all(result[s + '_sha256'] == cpu['raw'][name + '-' + s]['sha256'] for s in ('stdout', 'stderr')),
                'stream identity')
        phase_summary[name] = dict(command=cpu['raw'][name + '-command.json'], result=cpu['raw'][name + '-result.json'])
    require(H.parse(retained['old-targets-before.json']) == H.parse(retained['old-targets-after.json']),
            'old target inventories unchanged')

    base = H.parse(retained['sources-base.json'])
    previous = H.document(local / PRIOR['parent'][0] / 'sources-after.json', priors['parent']['raw']['sources-after.json'])
    require(base == {key: value for key, value in previous.items() if not key.startswith('ferric/qualification/')},
            'entire qualified source generation, qualification subtree excluded only')
    unformatted = dict(base)
    for role in ('worker', 'parent'):
        for row in proposals[role]['files']:
            require(row['source'] == 'draft/' + row['path'] and '..' not in Path(row['path']).parts
                    and not Path(row['path']).is_absolute(), 'proposal path')
            key = 'ferric/' + row['path']
            require(unformatted.get(key) == row['before'], 'overlay preimage')
            body = local / 'proposals' / PROPOSALS[role][0] / row['source']
            H.verify(body, row['after'])
            require(inputs.get(str(E / PROPOSALS[role][0] / row['source'])) == dict(
                    path=str(E / PROPOSALS[role][0] / row['source']), **row['after']), 'compiled proposal input')
            unformatted[key] = row['after']
    require(unformatted == H.parse(retained['sources-unformatted.json']), 'exact unformatted overlay generation')
    formatted = H.parse(retained['sources-before.json'])
    allowed = {'ferric/' + row['path'] for row in rows if row['path'].endswith('.rs')}
    require(len(allowed) == 21 and set(formatted) == set(unformatted)
            and all(formatted[key] == value for key, value in unformatted.items() if key not in allowed)
            and formatted == H.parse(retained['sources-after.json']), 'bounded formatting and unchanged compiled source')
    integrated = []
    for row in rows:
        expected = formatted['ferric/' + row['path']]
        H.verify(directory / 'sources/ferric' / row['path'], expected)
        live = H.verify(repo / row['path'], expected)
        integrated.append(dict(path=row['path'], before=row['before'], unformatted=row['after'],
                               compiled=expected, integrated=live))

    runtime_names = H.inventory(retained['runtime-list-stdout'].decode())
    worker_names = H.inventory(retained['worker-list-stdout'].decode())
    parent_names = H.inventory(retained['parent-lib-list-stdout'].decode())
    for role, current in [('worker', worker_names), ('parent', parent_names)]:
        stem = 'worker-list' if role == 'worker' else 'parent-lib-list'
        prior_names = H.inventory(H.read(local / PRIOR[role][0] / (stem + '-stdout'), priors[role]['raw'][stem + '-stdout']).decode())
        additions = set(cpu['added_worker_tests'] if role == 'worker' else cpu['added_parent_tests']['lib'])
        require(not prior_names.intersection(additions) and current == prior_names | additions, 'exact full library extension')
    require(runtime_names == H.inventory(H.read(local / PRIOR['worker'][0] / 'runtime-list-stdout',
            priors['worker']['raw']['runtime-list-stdout']).decode()), 'unchanged runtime inventory')
    worker_additions = set(cpu['added_worker_tests'])
    direct = set(proposals['parent']['test_census']['library'])
    wire = {name for name in worker_additions if name.startswith(WIRE)}
    require(len(worker_additions) == 16 and {n.rsplit('::', 1)[-1] for n in worker_additions}
            == set(proposals['worker']['authored_tests']['names']) and len(wire) == 4 and len(direct) == 9
            and all(name.startswith(DIRECT) for name in direct)
            and set(cpu['added_parent_tests']['lib']) == direct | wire
            and cpu['added_parent_tests']['bin'] == proposals['parent']['test_census']['binary'], 'declared tested additions')
    require(set(cpu['tests']) == {'runtime', 'worker', 'parent'}, 'closed role test records')
    require(set(cpu['tests']['runtime']) == set(priors['worker']['tests']) - {'worker'}
            and set(cpu['tests']['parent']) == set(priors['parent']['tests']) | {'parent-projection-residual-wire', PARENT},
            'all historical and new test selections')
    summaries, total, ignored_total = {}, 0, 0
    for role, groups in [('runtime', cpu['tests']['runtime']), ('worker', {'worker': cpu['tests']['worker']}),
                         ('parent', cpu['tests']['parent'])]:
        seen = set()
        summaries[role] = {}
        for name, result in groups.items():
            binary = name in cpu['binaries']
            stem = 'worker-tests' if role == 'worker' else name + '-tests' if binary else name
            ignored = H.outcomes(retained[stem + '-stdout'].decode(), result)
            names = set(result['names'])
            if role == 'worker':
                old = priors['worker']['tests']['worker']
                old_ignored = H.outcomes(H.read(local / PRIOR['worker'][0] / 'worker-tests-stdout',
                    priors['worker']['raw']['worker-tests-stdout']).decode(), old)
                require(names == worker_names and ignored == old_ignored and not ignored.intersection(worker_additions),
                        'all worker regressions and additions, only prior ignores')
            else:
                require(not ignored, 'nonignored selected runtime and parent cases')
                if name == 'parent-projection-residual-wire': expected = wire
                elif name == PARENT: expected = set(cpu['added_parent_tests']['bin'])
                else:
                    old = priors['worker' if role == 'runtime' else 'parent']['tests'][name]
                    expected = set(old['names']) | (direct if name == 'parent-client' else set())
                require(names == expected, 'exact selected regression names')
                if binary:
                    require(H.inventory(retained[name + '-list-stdout'].decode()) == names, 'bin actual inventory')
                else:
                    require(names <= (runtime_names if role == 'runtime' else parent_names)
                            and not names.intersection(seen), 'disjoint selected library tests')
                    seen.update(names)
            summaries[role][name] = dict(passed=result['passed'], ignored=result['ignored'])
            total += result['passed']; ignored_total += result['ignored']
    require(total == cpu['tests_passed'] and ignored_total == cpu['tests_ignored'] == 4,
            'actual aggregate derived from every named result')

    require(set(cpu['binaries']) == set(priors['parent']['binaries']) | {WORKER, PARENT}, 'all runnable artifact records')
    for role, phase in [('worker', 'worker-build'), ('parent', 'parent-builds')]:
        stream = [H.parse(line) for line in retained[phase + '-stdout'].splitlines() if line.startswith(b'{')]
        require([r['success'] for r in stream if r.get('reason') == 'build-finished'] == [True], 'actual Cargo build completion')
        artifacts = [r for r in stream if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        wanted = {WORKER} if role == 'worker' else set(priors['parent']['binaries']) | {PARENT}
        require(len(artifacts) == len(wanted) and {r['target']['name'] for r in artifacts} == wanted, 'exact artifact census')
        source_dir = 'tp-peer-finite-engineering-worker-v1' if role == 'worker' else 'm1-engineering-execution-v1'
        for artifact in artifacts:
            name = artifact['target']['name']; row = cpu['binaries'][name]
            require(artifact == row['artifact'] and artifact['target']['kind'] == ['bin']
                    and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
                    and artifact['executable'] == row['binary']['path'] == str(remote / 'target' / role / 'debug' / name)
                    and artifact['manifest_path'] == str(remote / 'sources/ferric/adapters' / source_dir / 'Cargo.toml'),
                    'actual optimized source-target-artifact identity')
    executable_pins = {}
    for name, path in [(WORKER, Path(a.worker_elf)), (PARENT, Path(a.parent_elf))]:
        executable_pins[name] = H.verify(path, cpu['binaries'][name]['binary'])
        with path.open('rb') as stream:
            require(stream.read(4) == b'\x7fELF', 'retained selected executable is ELF')

    # Large raw records, maps and executables stay in retained evidence, not Git.
    snapshot = dict(H.CHECKED)
    for path, pin in snapshot.items():
        H.verify(Path(path), pin)
    summary = dict(schema='ferric-p228-projection-residual-runtime-publication-v1',
        cpu_receipt=dict(original=dict(path=str(remote / 'complete.json'), bytes=complete['bytes'], sha256=complete['sha256']), retained=complete),
        controller=cpu['controller'], publication_helper=own, source_manifest=cpu['source_manifest'],
        prior_worker_cpu=cpu['prior_worker_cpu'], prior_parent_cpu=cpu['prior_parent_cpu'],
        proposal_manifests=cpu['proposal_manifests'], source_maps={name: cpu['raw'][name] for name in sorted(maps)},
        formatted_source_files=integrated, selected_binaries={name: cpu['binaries'][name] for name in (WORKER, PARENT)},
        retained_selected_binaries=executable_pins, all_built_binary_pins={name: row['binary'] for name, row in cpu['binaries'].items()},
        tests=summaries, tests_passed=total, tests_ignored=ignored_total, added_worker_tests=cpu['added_worker_tests'],
        added_parent_tests=cpu['added_parent_tests'], owned_phases=phase_summary, raw_members_rehashed=len(retained),
        local_file_hash_checks=len(snapshot), all_phase_commands_reconstructed=True, all_test_outcomes_replayed=True,
        all_22_compiled_sources_equal_live=True, raw_records_copied=False, source_archives_rehashed=False,
        all_transitive_cpu_inputs_rehashed=False, external_library_bodies_rehashed=False,
        metadata_validation='Authenticated actual controller receipt; not independently replayed here.',
        parent_dependency_routing='Locked Git fe2o3, not the paired local runtime used by the worker.',
        qualification='Actual CPU qualification and local retained-byte audit only.', **{key: False for key in FALSE})
    destination.mkdir(mode=0o700, exist_ok=True)
    for name, body in [('controller.py', controller_body), ('publish.py', H.read(Path(__file__).resolve(strict=True), own)),
                       ('result.json', (json.dumps(summary, indent=2, sort_keys=True) + '\n').encode())]:
        with (destination / name).open('xb') as stream:
            stream.write(body)
    print(json.dumps(dict(result=H.fingerprint(destination / 'result.json'), tests_passed=total,
                          tests_ignored=ignored_total, files_written=3)), flush=True)


if __name__ == '__main__':
    main()
