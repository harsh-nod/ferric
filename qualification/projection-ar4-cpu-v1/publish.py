"""Verify actual AR4 CPU evidence; publish source copies and a separate integration patch."""
import argparse
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = 'p228-projection-ar4-cpu-v1'
CONTROLLER_SHA = 'a4f9fe5df6d6203003eaffd11ba0a7dee0586e16a5b0fc419599c8bab16b57b3'
INPUT_NAME = 'projection-ar4-cpu-inputs-v228-v1.json'
INPUT_SHA = 'f2cb8e4b233c6960139cd326c440f0682b575685c1647975781702e341750f0f'
PROPOSAL = 'p228-projection-ar4-runtime-v1'
PROPOSAL_SHA = '58ae10d372b552190a6c811e1a0e53591171a6d8682cafd3bca55279d2eb3d7f'
HELPER_SHA = 'af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9'
OLD = 'projection-residual-decode-cpu-v228-v1'
OLD_SHA = '1f4365064da1a0884385035d1f2d280e6bba66afc2b5bb4265bf7d60b1418c83'
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
PARENT = 'ferric-qwen3-finite-projection-residual-decode-engineering'
FALSE = ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority')
MAPS = {'sources-base.json', 'sources-unformatted.json', 'sources-before.json', 'sources-after.json',
        'old-targets-before.json', 'old-targets-after.json', 'configurations.json'}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def module(path, body, name):
    value = types.ModuleType(name)
    value.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), value.__dict__)
    return value


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary verifier')
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('evidence', 'repo', 'cpu', 'cpu-sha', 'publish-to'):
        parser.add_argument('--' + key, required=True)
    a = parser.parse_args()
    local, repo, directory, destination = map(Path, (a.evidence, a.repo, a.cpu, a.publish_to))
    require(all(p.is_absolute() and p.resolve(strict=True) == p for p in (local, repo, directory)), 'canonical roots')
    require(destination == repo / 'qualification/projection-ar4-cpu-v1', 'closed output scope')
    helper_path = repo / 'qualification/native-device-routing-v1/publish.py'
    require(helper_path.resolve(strict=True) == helper_path, 'canonical retained data helper')
    helper_body = helper_path.read_bytes()
    require(hashlib.sha256(helper_body).hexdigest() == HELPER_SHA, 'retained data helper')
    H = module(helper_path, helper_body, 'ar4_publication_data')
    H.verify(helper_path, sha=HELPER_SHA)
    own_path = Path(__file__).resolve(strict=True)
    own = H.verify(own_path)
    own_body = H.read(own_path, own)
    if destination.exists():
        require(destination.resolve(strict=True) == destination and destination.is_dir()
                and {p.name for p in destination.iterdir()} <= {'README.md'}, 'README-only destination')
        if (destination / 'README.md').exists():
            H.verify(destination / 'README.md')
    else:
        require(not destination.is_symlink() and destination.parent.resolve(strict=True) == destination.parent,
                'canonical fresh output parent')

    complete_body = H.read(directory / 'complete.json', sha=a.cpu_sha)
    complete = H.CHECKED[str(directory / 'complete.json')]
    cpu = H.parse(complete_body)
    require(cpu['schema'] == 'ferric-projection-ar4-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and cpu['empty_initial_target'] is True
            and all(cpu[k] is False for k in FALSE), 'actual successful CPU qualification')
    remote = Path(H.filepin(cpu['raw']['sources-before.json'])['path']).parent
    require(remote.parent == E and re.fullmatch(r'projection-ar4-cpu-v228-v[1-9][0-9]*', remote.name), 'CPU case namespace')
    controller_path = local / 'proposals' / PACKAGE / 'run.py'
    controller_body = H.read(controller_path, cpu['controller'], sha=CONTROLLER_SHA)
    require(cpu['controller']['path'] == str(E / PACKAGE / 'run.py'), 'executed CPU controller')
    C = module(controller_path, controller_body, 'ar4_cpu_receipt_policy')
    input_pins = {}
    for pin in cpu['inputs']:
        H.filepin(pin)
        require(pin['path'] not in input_pins or input_pins[pin['path']] == pin, 'nonconflicting consumed pins')
        input_pins[pin['path']] = pin
    source_pin = input_pins[str(E / INPUT_NAME)]
    source_path = local / 'proposals' / PACKAGE / 'inputs.json'
    source_body = H.read(source_path, source_pin, sha=INPUT_SHA)
    source = H.parse(source_body); C.input_shape(source)
    proposal_path = local / 'proposals' / PROPOSAL / 'source-manifest.json'
    proposal_body = H.read(proposal_path, source['proposal'], sha=PROPOSAL_SHA)
    proposal = H.parse(proposal_body); C.proposal_shape(proposal)
    require(cpu['proposal'] == source['proposal']
            and cpu['declared_test_additions'] == proposal['added_tests']
            and cpu['declared_test_renames'] == proposal['renamed_tests'], 'declared AR4 source/test delta')
    prior = H.document(local / OLD / 'complete.json', cpu['prior_completion'], sha=OLD_SHA)
    require(cpu['prior_completion']['path'] == str(E / OLD / 'complete.json'), 'actual CPU1022 predecessor')
    C.prior_contract(prior)
    old_inputs = {v['path']: H.filepin(v) for v in prior['inputs']}
    initial_paths = (C.HELPER, E / 'run_clean_worker_p228_v1.py', E.parent / 'wave-output-lowering-v216/bounded.py',
                     C.PRIOR_CONTROLLER)
    expected_inputs = [cpu['controller'], source_pin] + [old_inputs[str(p)] for p in initial_paths]
    require(prior['controller']['sha256'] == C.OLD_CONTROLLER_SHA, 'actual prior controller')
    expected_inputs += [prior['controller'], cpu['prior_completion']]
    old_raw = {}
    for name, pin in prior['raw'].items():
        require(H.filepin(pin)['path'] == str(E / OLD / name), 'direct prior raw member')
        old_raw[name] = H.read(local / OLD / name, pin)
    expected_inputs += list(prior['raw'].values()) + list(source['archives'].values()) + [source['proposal']]
    rows = proposal['files']
    for row in rows:
        path = local / 'proposals' / PROPOSAL / row['source']
        H.verify(path, row['after'])
        expected_inputs.append(dict(path=str(E / PROPOSAL / row['source']), **row['after']))

    recipes = {name: C.relocate(H.parse(old_raw[name + '-command.json']), E / OLD, remote) for name in prior['phases']}
    for name in ('rustfmt', 'rustfmt-check'):
        argv = recipes[name]['argv']
        recipes[name]['argv'] = argv[:argv.index('skip_children=true') + 1] + [str(remote / 'sources/ferric' / row['path']) for row in rows]
    formatter = old_inputs[recipes['rustfmt']['argv'][0]]
    require(formatter['sha256'] == '30de9e1efcd8f8fe7750e00d0c45ff8f4c480608ef1be5baf9ab6f1b4556e8f8', 'qualified formatter')
    expected_inputs.append(formatter)
    tools_seen = set()
    for recipe in recipes.values():
        for tool, digest in recipe['tools'].items():
            path = str(Path(recipe['env']['RUSTC']).parent / tool)
            require(old_inputs[path]['sha256'] == digest, 'unchanged recorded tool identity')
            if path not in tools_seen:
                expected_inputs.append(old_inputs[path]); tools_seen.add(path)
    require(cpu['inputs'] == expected_inputs, 'exact actual input order and identities')
    phases = set(recipes)
    require(len(phases) == 87 and set(cpu['phases']) == phases, 'all original 87 phases')
    expected_raw = MAPS | {name + suffix for name in phases for suffix in H.SUFFIXES}
    require(set(cpu['raw']) == expected_raw and len(expected_raw) == 442, 'complete actual raw roster')
    raw = {}
    for name, pin in cpu['raw'].items():
        require(H.filepin(pin)['path'] == str(remote / name), 'direct candidate raw member')
        raw[name] = H.read(directory / name, pin)
    for name in sorted(phases):
        require(H.parse(raw[name + '-command.json']) == recipes[name], 'exact relocated command: ' + name)
        started = H.parse(raw[name + '-started.json'])
        result = H.parse(raw[name + '-result.json'])
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'actual owned group')
        require(result == cpu['phases'][name] and type(result['exit_code']) is int
                and result['exit_code'] == 0 and result['reason'] is None and result['group_absent'] is True
                and 0 <= result['cache_bytes'] <= 6 << 30, 'natural successful bounded phase')
        require(all(result[k + '_sha256'] == cpu['raw'][name + '-' + k]['sha256'] for k in ('stdout', 'stderr')),
                'actual phase streams')
    require(H.parse(raw['old-targets-before.json']) == H.parse(raw['old-targets-after.json']), 'protected targets unchanged')
    for role in ('worker', 'parent'):
        require(cpu['metadata'][role] == C.relocate(prior['metadata'][role], E / OLD, remote), 'unchanged recorded metadata closure')

    base = H.parse(raw['sources-base.json'])
    C.source_base(base, H.parse(old_raw['sources-after.json']), source['documentation_changes'])
    archive_sources, archive_summary = {}, {}
    for project, pin in source['archives'].items():
        path = Path(H.filepin(pin)['path'])
        require(path.parent == E, 'source archive scope')
        H.verify(local / path.name, pin)
        files, archive_summary[project] = H.archive_map(local / path.name, project)
        require(not archive_sources.keys() & files.keys(), 'disjoint source archives')
        archive_sources.update(files)
    require(archive_sources == base, 'full archived source map')
    expected = dict(base)
    for row in rows:
        key = 'ferric/' + row['path']
        require(expected[key] == row['before'], 'actual source preimage')
        expected[key] = row['after']
    require(expected == H.parse(raw['sources-unformatted.json']), 'only reviewed nine-source overlay')
    formatted = H.parse(raw['sources-before.json'])
    allowed = {'ferric/' + row['path'] for row in rows}
    require(len(allowed) == 9 and set(formatted) == set(expected)
            and all(formatted[k] == v for k, v in expected.items() if k not in allowed)
            and formatted == H.parse(raw['sources-after.json']), 'formatter/source closure')
    staged, patch = [], ['*** Begin Patch\n']
    for row in rows:
        path = repo / row['path']
        require(path.parent.resolve(strict=True) == path.parent and not path.is_symlink(), 'canonical live preimage')
        before = H.read(path, row['before'])
        after_pin = formatted['ferric/' + row['path']]
        after = H.read(directory / 'sources/ferric' / row['path'], after_pin)
        require(before.endswith(b'\n') and after.endswith(b'\n'), 'complete newline-terminated Rust source')
        require(before != after, 'reviewed replacement differs from live preimage')
        diff = list(difflib.unified_diff(before.decode('utf-8').splitlines(keepends=True),
                    after.decode('utf-8').splitlines(keepends=True), n=3))
        require(len(diff) > 2, 'nonempty source integration patch')
        patch.append('*** Update File: ' + str(path) + '\n')
        patch.extend('@@\n' if line.startswith('@@ ') else line for line in diff[2:])
        staged.append((dict(path=row['path'], before=row['before'], unformatted=row['after'], compiled=after_pin), after))
    patch.append('*** End Patch\n')
    patch_body = ''.join(patch).encode('utf-8')

    inventories = {role: H.inventory(raw[name + '-stdout'].decode()) for role, name in
                   [('runtime', 'runtime-list'), ('worker', 'worker-list'), ('parent', 'parent-lib-list')]}
    old_inventories = {role: H.inventory(old_raw[name + '-stdout'].decode()) for role, name in
                       [('runtime', 'runtime-list'), ('worker', 'worker-list'), ('parent', 'parent-lib-list')]}
    require(inventories['runtime'] == old_inventories['runtime'], 'unchanged full runtime inventory')
    additions, renames = {}, {}
    for role in ('worker', 'parent'):
        additions[role], renames[role] = C.extended_inventory(old_inventories[role], inventories[role], proposal, role)
    require(set(cpu['tests']) == set(prior['tests']), 'unchanged test selections')
    old_ignored = H.outcomes(old_raw['worker-tests-stdout'].decode(), prior['tests']['worker-tests'])
    totals, seen = {}, {'runtime': set(), 'parent': set()}
    for name, previous in prior['tests'].items():
        observed = cpu['tests'][name]
        if name == 'worker-tests':
            role, names = 'worker', inventories['worker']
        elif name.startswith('ferric-'):
            role, names = 'parent-bin', set(previous['names'])
            require(H.inventory(raw[name.removesuffix('-tests') + '-list-stdout'].decode()) == names, 'unchanged binary test inventory')
        elif name.startswith('parent-'):
            role = 'parent'; argv = recipes[name]['argv']; selector = argv[argv.index('--lib') + 1]
            names = C.selected_names(set(previous['names']), inventories[role], additions[role], renames[role], selector)
        else:
            role, names = 'runtime', set(previous['names'])
        require(len(observed['names']) == len(names) and set(observed['names']) == names, 'exact named outcomes: ' + name)
        ignored = H.outcomes(raw[name + '-stdout'].decode(), observed)
        require(ignored == (old_ignored if role == 'worker' else set()), 'only historical ignores')
        if role in seen:
            require(names <= inventories[role] and not seen[role] & names, 'disjoint selected library tests')
            seen[role].update(names)
        totals[name] = dict(passed=observed['passed'], ignored=observed['ignored'])
    require(additions['parent'] <= seen['parent'] and len(old_ignored) == 4, 'all new parent executions; exact old ignores')
    passed = sum(v['passed'] for v in totals.values()); ignored = sum(v['ignored'] for v in totals.values())
    require(passed == cpu['tests_passed'] == prior['tests_passed'] + 15 == 1037
            and ignored == cpu['tests_ignored'] == 4, 'derived actual total, not projected success')

    require(set(cpu['binaries']) == set(prior['binaries']) and len(cpu['binaries']) == 17, 'same 17 selected products')
    elf_pins = {}
    for role, phase, subdir in [('worker', 'worker-build', C.WORKER), ('parent', 'parent-builds', C.PARENT)]:
        stream = [H.parse(line) for line in raw[phase + '-stdout'].splitlines() if line.startswith(b'{')]
        require([v['success'] for v in stream if v.get('reason') == 'build-finished'] == [True], 'Cargo build-finished')
        artifacts = [v for v in stream if v.get('reason') == 'compiler-artifact' and v.get('executable')]
        wanted = {WORKER} if role == 'worker' else set(cpu['binaries']) - {WORKER}
        require(len(artifacts) == len(wanted) and {v['target']['name'] for v in artifacts} == wanted, 'actual artifact census')
        for artifact in artifacts:
            name = artifact['target']['name']; row = cpu['binaries'][name]; pin = H.filepin(row['binary'])
            require(artifact == row['artifact'] and artifact['target']['kind'] == ['bin']
                    and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
                    and artifact['executable'] == pin['path'] == str(remote / 'target' / role / 'debug' / name)
                    and artifact['manifest_path'] == str(remote / 'sources/ferric' / subdir / 'Cargo.toml'), 'exact Cargo source/product selection')
            path = directory / 'target' / role / 'debug' / name
            elf_pins[name] = H.verify(path, pin)
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'retained selected ELF')

    payloads = {'complete.json': complete_body, 'controller.py': controller_body, 'publish.py': own_body,
                'inputs.json': source_body, 'source-manifest.json': proposal_body, 'integration.patch': patch_body}
    payloads.update({'candidate/' + row['path']: body for row, body in staged})
    payloads.update({'raw/' + name: body for name, body in raw.items() if name not in MAPS})
    snapshot = dict(H.CHECKED)
    for path, pin in snapshot.items():
        H.verify(Path(path), pin)
    summary = dict(schema='ferric-p228-projection-ar4-cpu-publication-v1',
        cpu_receipt=dict(original=dict(path=str(remote / 'complete.json'), bytes=complete['bytes'], sha256=complete['sha256']), retained=complete),
        controller=cpu['controller'], publication_helper=own, prior_cpu=cpu['prior_completion'],
        source_manifest=source_pin, proposal_manifest=cpu['proposal'], source_archives=source['archives'], archive_census=archive_summary,
        source_maps={name: cpu['raw'][name] for name in sorted(MAPS)}, formatted_source_files=[row for row, _ in staged],
        integration_patch=dict(bytes=len(patch_body), sha256=hashlib.sha256(patch_body).hexdigest()),
        tests=totals, tests_passed=passed, tests_ignored=ignored,
        added_tests={role: sorted(value) for role, value in additions.items()}, renamed_tests=renames,
        selected_binaries={name: cpu['binaries'][name] for name in (WORKER, PARENT)},
        all_built_binary_pins={name: row['binary'] for name, row in cpu['binaries'].items()}, retained_binaries=elf_pins,
        owned_phase_count=len(phases), raw_members_rehashed=len(raw), raw_phase_files_copied=len(raw) - len(MAPS),
        prior_raw_members_rehashed=len(old_raw), local_file_hash_checks=len(snapshot),
        all_phase_commands_reconstructed=True, all_test_outcomes_replayed=True, all_17_built_elf_bodies_rehashed=True,
        source_archives_rehashed=True, source_archive_maps_reconstructed=True, all_nine_live_preimages_checked=True,
        live_source_files_written=False, source_integration_performed=False, source_maps_copied=False,
        executable_bodies_copied=False, model_bodies_copied=False, all_transitive_cpu_inputs_rehashed=False,
        external_library_bodies_rehashed=False, metadata_validation='Actual controller metadata equals the relocated CPU1022 record; no tool rerun here.',
        parent_dependency_routing='Locked Git fe2o3; worker uses the paired local runtime.',
        qualification='Actual CPU tests and build only. Integration patch is retained but not applied by this publisher.',
        compiler_hsaco_reproduced=False, full_model_acceptance=False, **{key: False for key in FALSE})
    payloads['result.json'] = (json.dumps(summary, indent=2, sort_keys=True) + '\n').encode('utf-8')
    ledger = {name: dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest()) for name, body in sorted(payloads.items())}
    payloads['ledger.json'] = (json.dumps(dict(schema='ferric-p228-projection-ar4-cpu-ledger-v1', files=ledger), indent=2, sort_keys=True) + '\n').encode('utf-8')
    # No source installation. Every input and all output bodies passed before mkdir.
    destination.mkdir(mode=0o700, exist_ok=True)
    for name, body in payloads.items():
        path = destination / name
        path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(result=H.fingerprint(destination / 'result.json'), tests_passed=passed,
                          tests_ignored=ignored, source_files_written=0, files_published=len(payloads))), flush=True)


if __name__ == '__main__':
    main()
