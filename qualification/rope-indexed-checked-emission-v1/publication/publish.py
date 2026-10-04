"""Publish authenticated linked-emission evidence without rerunning tested code."""
import argparse
import ast
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import re
import tarfile
import types

HERE = Path(__file__).resolve().parent
L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
F = Path('/home/harsh/ferric-p227-integration')
OUT = F / 'qualification/rope-indexed-checked-emission-v1'
EXPORT_SHA = '245e40c5b5b0b0f9db6af46fda85b9a5360ef660ba81d37ddd277452e2dfb75e'
COMMON = F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py'
PURE = L / 'rope-indexed-emission-root-test-observation-v228-v1.json'
PURE_SHA = 'ac42c3917005cfce719f632a16108a3c95506d384ca0319bee1f1d2ac066bd3e'
PRIOR = {
    'producer': (F / 'qualification/rope-materialized-rpo-lowering-v1/result.json',
        '42a977b693db1f547a922a1bc49c3a67a55fdaeea6eb20dbc355ee6b676fb87f'),
    'consumer': (F / 'qualification/kir-indexed-formal-join-v2/result.json',
        '575b82a829f51b684202fd16e739d61178d18e69a1e16610e31f632897def6c5')}
SYMBOL = 'ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6'


def exporter():
    path = HERE / 'export.py'
    if path.resolve(strict=True) != path or path.stat().st_size > 64 << 10:
        raise ValueError('canonical bounded exporter')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != EXPORT_SHA:
        raise ValueError('reviewed data exporter identity')
    module = types.ModuleType('linked_emission_courier_data')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def verify(X, D, value, owner, producer, consumer, recipe, read):
    D.terminal(X, value, owner)
    doc = lambda path: X.parse(read(path))
    X.require(owner['completion'] == dict(path=str(D.CASE / 'complete.json'), **X.extent(read(D.CASE / 'complete.json'))),
        'actual owner completion')
    X.require(tuple(row['name'] for row in producer['commands']) == D.OLD_PHASES
        and producer['commands'] == value['producer_commands'] and producer['artifacts'] == value['producer_captures']
        and producer['compiler_generation'] == value['producer_generation'] and producer['passed'] is False
        and producer['error'] == 'AssertionError: actual-inert-join' and producer['postcheck_errors'] == [],
        'old failed producer not relabeled')
    X.require(consumer['tests'] == value['consumer_tests'] and consumer['artifacts'] == value['preserved_consumer_test_artifacts']
        and consumer['raw']['sources-before.json'] == value['consumer_source']
        and value['consumer_source']['sha256'] == D.SOURCE_SHA and consumer['proposal'] == value['consumer_proposal']
        and consumer['retained_handoff'] == value['retained_handoff'], 'separate actual consumer generation')
    X.require(value['actual_join'] == dict(test=D.SELECTOR, passed=1, ignored=0, filtered_out=204,
        exit_code=0, actual_capture_join_passed=True, fresh_hsaco_emitted=False), 'new actual inert join')
    reference = doc(D.CONSUMER / 'finalizer-build-tests-command.json')
    env = dict(reference['env'], TMPDIR=str(D.CASE / 'tmp'))
    cargo = next(item for item in reference['argv'] if item.endswith('/bin/cargo'))
    source, target = D.CONSUMER / 'source/fe2o3', D.CONSUMER / 'target'
    recipes = {
        'metadata': dict(argv=[cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(source / 'Cargo.toml'),
            '--format-version', '1'], env=env, deadline_seconds=120, tools=reference['tools']),
        'tool-build': dict(argv=[cargo, 'build', '--offline', '--locked', '--jobs', '2', '--manifest-path',
            str(source / 'Cargo.toml'), '-p', 'fe2o3-hsaco-finalize', '--example', D.FINALIZER,
            '--example', D.METADATA, '--message-format=json'], env=env, deadline_seconds=1800, tools=reference['tools'])}
    substitutions = {'@candidate.handoff.sha256': value['retained_handoff']['sha256'],
        '@candidate.handoff.bytes': str(value['retained_handoff']['bytes']),
        '@candidate.image.sha256': value['artifacts']['emitted/artifact.hsaco']['sha256'],
        '@candidate.image.bytes': str(value['artifacts']['emitted/artifact.hsaco']['bytes'])}
    roles = {'actual-inert-join': value['preserved_consumer_test_artifacts']['finalizer-tests'],
        'emit': value['tools']['finalizer'], 'descriptor-metadata': value['tools']['metadata']}
    for template in recipe['commands'][3:]:
        command = copy.deepcopy(template); name = command['name']
        for old, new in ((str(D.PRODUCER / 'emitted'), str(D.CASE / 'emitted')),
                         (str(D.PRODUCER / 'extracted'), str(D.CASE / 'extracted'))):
            command['argv'] = [new + item[len(old):] if item == old or item.startswith(old + '/') else item
                for item in command['argv']]
        if name in roles: command['argv'][0] = roles[name]['path']
        command['env']['CARGO_TARGET_DIR'], command['env']['TMPDIR'] = str(target), str(D.CASE / 'tmp')
        command['env']['LD_LIBRARY_PATH'] = str(target / 'debug/deps') + ':' + command['env']['LD_LIBRARY_PATH'].split(':')[1]
        command['argv'] = [substitutions.get(item, item) for item in command['argv']]
        command['env'] = {key: substitutions.get(item, item) for key, item in command['env'].items()}
        recipes[name] = command
    X.require(tuple(recipes) == D.PHASES and doc(D.CASE / 'metadata-stdout') == doc(D.CONSUMER / 'metadata-stdout'),
        'eight exact recipes and unchanged qualified Cargo graph')
    owned = owner['owned']
    X.require(doc(D.OWNER / 'owned-result.json') == owned, 'actual owned result body')
    started = doc(D.OWNER / 'started.json')
    X.require(started['parent']['uid'] == 9661 and started['parent']['pid'] == started['parent']['pgid']
        == started['parent']['sid'] and any(row.get('reason') == 'spawned-parent'
        and row.get('identity') == started['parent'] for row in owned['lineage']), 'owned parent identity')
    command = doc(D.OWNER / 'command.json')
    X.require(command['argv'] == ['/usr/bin/python3', '-B', str(D.PACKAGE / 'run.py'), D.PACKAGE_SHA, '--child']
        and command['deadline_seconds'] == 10800 and command['gpu_execution'] is False, 'outer invocation')
    pids = []
    for row in value['commands']:
        name = row['name']; expected = recipes[name]
        command, started, result = [doc(D.CASE / (name + '-' + suffix + '.json')) for suffix in ('command', 'started', 'result')]
        X.require(all(command[key] == expected[key] for key in ('argv', 'env', 'tools', 'deadline_seconds'))
            and command['cache_cap_bytes'] == 6 << 30 and command['affinity'] == [8, 9]
            and command['nice'] == 10 and command['expected_exit'] == 0 and command['gpu_execution'] is False
            and 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1' not in command['env'], 'original bounded CPU-only command: ' + name)
        X.require(result == value['phases'][name] and type(result['exit_code']) is int and result['exit_code'] == 0
            and result['reason'] is None and result['group_absent'] is True, 'natural fresh phase outcome')
        pid = started['pid']; pids.append(pid)
        X.require(type(pid) is int and pid > 0 and started['pgid'] == pid and pid in owned['owned_groups']
            and any(r.get('event') == 'owned' and r.get('identity', {}).get('pid') == pid
                and r['identity']['pgid'] == pid and r['identity']['uid'] == 9661 for r in owned['lineage']), 'owned leaf identity')
        for key, suffix in zip(('command', 'started', 'result', 'stdout', 'stderr'), X.SUFFIXES):
            X.require(row[key] == value['raw'][name + '-' + suffix]
                == dict(path=str(D.CASE / (name + '-' + suffix)), **X.extent(read(D.CASE / (name + '-' + suffix)))),
                'fresh phase original pin')
        X.require(all(result[key + '_sha256'] == row[key]['sha256'] for key in ('stdout', 'stderr')), 'output hashes')
    X.require(len(set(pids)) == 8, 'eight distinct owned leaf leaders')
    for old in producer['commands']:
        result = doc(Path(old['result']['path']))
        X.require(type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
            and result['group_absent'] is True and all(result[k + '_sha256'] == old[k]['sha256'] for k in ('stdout', 'stderr')),
            'retained successful historical leaf')
    build = [X.parse(line) for line in read(D.CASE / 'tool-build-stdout').splitlines() if line.strip()]
    for key, name in (('finalizer', D.FINALIZER), ('metadata', D.METADATA)):
        rows = [row for row in build if row.get('reason') == 'compiler-artifact' and row.get('target', {}).get('name') == name
            and row.get('profile', {}).get('test') is False
            and row.get('manifest_path') == str(source / 'crates/fe2o3-hsaco-finalize/Cargo.toml')]
        pin = X.pin(value['tools'][key])
        X.require(len(rows) == 1 and rows[0]['executable'] == pin['path'] == str(target / 'debug/examples' / name),
            'actual consumer Cargo tool identity')
    stdout = read(D.CASE / 'actual-inert-join-stdout').decode()
    X.require(re.findall(r'^test (\S+) \.\.\. (\S+)$', stdout, re.M) == [(D.SELECTOR, 'ok')]
        and re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 204 filtered out;', stdout),
        'one actually executed retained inert-join test')
    lines = read(D.CASE / 'descriptor-metadata-stdout').decode('ascii').splitlines()
    X.require(lines.pop(0) == 'fe2o3-finite-join-request-metadata-v1', 'descriptor schema')
    metadata = {}
    for line in lines:
        key, item = line.split(' ', 1); X.require(key not in metadata, 'unique descriptor field'); metadata[key] = item
    image = value['artifacts']['emitted/artifact.hsaco']
    expected = dict(authority='none', object_sha256=image['sha256'], object_bytes=str(image['bytes']),
        entry_symbol_hex=SYMBOL.encode().hex(), target='gfx950:xnack-', code_object_version='6',
        explicit_argument_bytes='120', kernarg_segment_bytes='376', kernarg_alignment='8', workgroup='64,1,1', max_grid_workgroups='64,1,1')
    X.require(set(metadata) == set(expected) | {'descriptor_sha256', 'canonical_code_object_digest', 'descriptor_symbol_hex'}
        and all(metadata[k] == v for k, v in expected.items()), 'same concrete prefix ABI')
    for key in ('descriptor_sha256', 'canonical_code_object_digest'): X.digest(metadata[key])
    X.require(re.fullmatch('(?:[0-9a-f]{2})+', metadata['descriptor_symbol_hex']), 'descriptor symbol encoding')
    notes = read(D.CASE / 'elf-notes-stdout'); isa = read(D.CASE / 'disassembly-stdout')
    X.require(all(marker in notes for marker in (b'.group_segment_fixed_size: 512', b'.private_segment_fixed_size: 0', b'.wavefront_size: 64'))
        and SYMBOL.encode() in isa and b's_endpgm' in isa, 'retained resource and ISA inspection')
    read(D.CASE / 'extracted/module.ll').decode('utf-8')
    X.require(all(value['artifacts']['emitted/source.handoff-v3'][key] == value['retained_handoff'][key]
        for key in ('bytes', 'sha256')), 'same source handoff bytes')
    receipt = read(D.CASE / 'emitted/receipt.txt').decode('utf-8')
    X.require(receipt.startswith('wave_qkv_attention_output_tile_engineering_emission_v6\n')
        and all(line in receipt.splitlines() for line in ('status PASS', 'authority none',
            'runtime_requirements required_undischarged', 'unresolved_requirement_count 8',
            'finalized_hsaco_content ' + image['sha256'] + ' ' + str(image['bytes']))), 'recorded bounded engineering emission')
    return metadata


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path); parser.add_argument('archive', type=Path)
    parser.add_argument('archive_sha256'); parser.add_argument('complete_sha256'); parser.add_argument('owner_sha256')
    args = parser.parse_args()
    D = exporter(); X = D.common(COMMON)
    root = args.retained_root.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical retained courier root')
    manifest_raw = X.read(root / 'export-manifest.json'); manifest = X.parse(manifest_raw)
    records, omitted = manifest['files'], manifest['remotely_rehashed_omitted_pins']
    X.require(manifest['schema'] == 'ferric-p228-rope-indexed-emission-export-v1' and manifest['original_root'] == str(D.E)
        and manifest['complete_sha256'] == X.digest(args.complete_sha256) == D.INNER[1]
        and manifest['owner_sha256'] == X.digest(args.owner_sha256) == D.OUTER[1]
        and manifest['exporter']['sha256'] == EXPORT_SHA and manifest['data_helper']['sha256'] == D.COMMON_SHA
        and all(manifest[k] is False for k in ('transitive_source_bodies_rehashed', 'tool_executables_exported',
            'full_source_maps_exported', 'owner_inventories_exported')) and len(records) <= 112 and len(omitted) <= 20,
        'actual reviewed finite courier')
    for name, pin in {**records, **omitted}.items():
        X.require(X.pin(pin)['path'] == str(D.E / X.relative(name)), 'original courier identity')
    X.require(not set(records) & set(omitted), 'retained and omitted identities disjoint')
    def read(path, expected=None):
        name = str(Path(path).relative_to(D.E)); pin = records[name]
        X.require(expected is None or {k: expected[k] for k in ('bytes', 'sha256')}
            == {k: pin[k] for k in ('bytes', 'sha256')}, 'requested original identity')
        return X.read(root / X.relative(name), pin, cap=128 << 20)
    for pin in records.values(): read(pin['path'], pin)
    archive_raw = X.read(args.archive.absolute(), cap=96 << 20)
    X.require(X.extent(archive_raw)['sha256'] == X.digest(args.archive_sha256), 'root-authenticated actual archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
        members = archive.getmembers()
        X.require(len(members) == len(records) + 1 and {m.name for m in members} == set(records) | {'export-manifest.json'}
            and all(m.isfile() and str(X.relative(m.name)) == m.name for m in members), 'closed ordinary archive')
        for member in members:
            expected = X.extent(manifest_raw) if member.name == 'export-manifest.json' else records[member.name]
            X.require(member.size == expected['bytes'] and X.extent(archive.extractfile(member).read())
                == {k: expected[k] for k in ('bytes', 'sha256')}, 'archive/retained byte identity')
    value = X.parse(read(D.CASE / 'complete.json', dict(zip(('bytes', 'sha256'), D.INNER))))
    owner = X.parse(read(D.OWNER / 'complete.json', dict(zip(('bytes', 'sha256'), D.OUTER))))
    producer, consumer = (X.parse(read(value[k]['path'], value[k])) for k in ('producer', 'consumer'))
    recipe = X.parse(read(value['producer_recipe']['path'], value['producer_recipe']))
    package = X.parse(read(value['package']['path'], value['package']))
    X.require(value['package']['sha256'] == D.PACKAGE_SHA and value['consumer_proposal']['sha256'] == D.PROPOSAL_SHA,
        'executed controller and consumer source proposals')
    for row in package['files']: read(D.PACKAGE / X.relative(row['path']), row)
    metadata = verify(X, D, value, owner, producer, consumer, recipe, read)
    for pin in (*value['tools'].values(), *value['preserved_consumer_test_artifacts'].values(),
                *value['producer_captures'].values(), value['consumer_source'],
                value['raw']['before.json'], value['raw']['after.json']):
        X.require(omitted[str(Path(pin['path']).relative_to(D.E))] == pin, 'explicit omitted-body identities')
    prior_raw = {key: X.read(path) for key, (path, _) in PRIOR.items()}
    for key, (_, digest) in PRIOR.items(): X.require(X.extent(prior_raw[key])['sha256'] == digest, 'immutable published prerequisite')
    old, current = (X.parse(prior_raw[key]) for key in ('producer', 'consumer'))
    X.require(old['lower'] == value['producer'] and old['owner'] == value['producer_owner'] and old['lowering_passed'] is False
        and old['compiler_generation'] == value['producer_generation']
        and current['cpu'] == value['consumer'] and current['owner'] == value['consumer_owner']
        and current['selected_test_artifacts'] == value['preserved_consumer_test_artifacts']
        and current['tests'] == {name: {key: result[key] for key in ('passed', 'ignored')}
            for name, result in value['consumer_tests'].items()}, 'separate immutable producer/consumer publications')
    pure_raw = X.read(PURE); pure = X.parse(pure_raw)
    X.require(X.extent(pure_raw)['sha256'] == PURE_SHA and pure['schema'] == 'ferric-primary-agent-test-observation-v1'
        and pure['remote_supervisor_receipt'] is False and pure['terminal_tool_result']['exit_code'] == 0
        and pure['tests_passed'] == 16 and pure['tests_failed'] == 0 and pure['source_hashes_unchanged'] is True,
        'actual primary pure16 observation, not a remote supervisor receipt')
    tree = ast.parse(read(D.PACKAGE / 'test_run.py'))
    names = sorted('test_run.' + cls.name + '.' + method.name for cls in tree.body if isinstance(cls, ast.ClassDef)
        for method in cls.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'))
    log = pure['terminal_tool_result']['output'].replace('\r\n', '\n')
    observed = sorted(cls + '.' + method for method, cls in re.findall(r'^(test_\w+) \((test_run[.]\w+)(?:[.]test_\w+)?\) \.\.\. ok$', log, re.M))
    X.require(len(names) == 16 and names == observed == sorted(cls + '.' + method
        for cls, methods in package['test_census'].items() for method in methods)
        and re.search(r'\nRan 16 tests in [0-9.]+s\n\nOK\n', log), 'actual complete named pure log')
    expected_hashes = {str(D.PACKAGE / row['path']): row['sha256'] for row in package['files']}
    expected_hashes[str(D.PACKAGE / 'manifest.json')] = D.PACKAGE_SHA
    for key in ('prehash_tool_result', 'posthash_tool_result'):
        raw = pure[key]; X.require(raw['exit_code'] == 0, 'actual source hash observation')
        found = {path: digest for digest, path in re.findall(r'^([0-9a-f]{64})  (\S+)$', raw['output'].replace('\r\n', '\n'), re.M)}
        X.require(found == expected_hashes, 'all five before/after source hashes')
    copies = {}
    def copy_body(name, raw, original):
        X.relative(name); X.require(name not in copies, 'unique publication member'); copies[name] = (raw, original)
    for pin in records.values():
        path = Path(pin['path']); name = None
        if path.is_relative_to(D.CASE):
            relative = str(path.relative_to(D.CASE))
            name = 'complete.json' if relative == 'complete.json' else ('artifacts/' + relative if relative in D.ARTIFACTS else 'raw/' + relative)
        elif path.is_relative_to(D.OWNER):
            name = 'owner-complete.json' if path.name == 'complete.json' else 'owner/' + path.name
        elif path.is_relative_to(D.PACKAGE): name = 'controller/' + str(path.relative_to(D.PACKAGE))
        elif path.is_relative_to(D.PRODUCER / 'fixture'): name = 'fixture/' + str(path.relative_to(D.PRODUCER / 'fixture'))
        elif path.is_relative_to(D.PRODUCER): name = 'producer/' + str(path.relative_to(D.PRODUCER))
        elif path.is_relative_to(D.PRODUCER_OWNER): name = 'producer/owner-failed.json'
        elif path.is_relative_to(D.CONSUMER): name = 'consumer/' + str(path.relative_to(D.CONSUMER))
        elif path.is_relative_to(D.CONSUMER_OWNER): name = 'consumer/owner-complete.json'
        elif pin == value['consumer_proposal']: name = 'consumer/source-proposal.json'
        elif pin == producer['candidate_cpu']: name = 'producer/arithmetic-cpu.json'
        elif pin == producer['source_manifest']: name = 'producer/arithmetic-source-proposal.json'
        X.require(name is not None, 'closed publication mapping')
        copy_body(name, read(path, pin), pin)
    copy_body('pure-root-observation.json', pure_raw, dict(path=str(PURE), **X.extent(pure_raw)))
    for name in ('export.py', 'publish.py'):
        raw = X.read(HERE / name); copy_body('publication/' + name, raw, dict(path=str(HERE / name), **X.extent(raw)))
    ledger = [dict(path=name, original=original, **X.extent(raw)) for name, (raw, original) in sorted(copies.items())]
    summary = dict(schema='ferric-p228-rope-indexed-emission-publication-v1', publication_passed=True,
        complete=owner['completion'], owner=dict(path=str(D.OWNER / 'complete.json'), **X.extent(read(D.OWNER / 'complete.json'))),
        producer=value['producer'], producer_owner=value['producer_owner'], producer_generation=value['producer_generation'],
        producer_commands=value['producer_commands'], producer_captures=value['producer_captures'], producer_aggregate_passed=False,
        consumer=value['consumer'], consumer_owner=value['consumer_owner'], consumer_source=value['consumer_source'],
        consumer_proposal=value['consumer_proposal'], consumer_tests=value['consumer_tests'], consumer_tools=value['tools'],
        preserved_consumer_test_artifacts=value['preserved_consumer_test_artifacts'], retained_handoff=value['retained_handoff'],
        phases=value['phases'], actual_join=value['actual_join'], artifacts=value['artifacts'], descriptor=metadata,
        retained_checked_producer_replay_authenticated=True, producer_consumer_generations_distinct=True,
        fresh_consumer_tools_built=True, fresh_actual_inert_join_passed=True, fresh_hsaco_emitted=True,
        unresolved_runtime_requirements=8, source_unchanged=True,
        owned_outcome={key: owner['owned'][key] for key in ('exit_code', 'reason', 'cleanup_signalled',
            'owned_groups_absent', 'owned_processes_reaped', 'elapsed_seconds', 'deadline_seconds')},
        elapsed_time_scope='host-owned-controller diagnostic, not GPU timing or performance',
        prerequisite_publications={key: dict(path=str(path), **X.extent(prior_raw[key])) for key, (path, _) in PRIOR.items()},
        primary_pure_observation=dict(path=str(PURE), **X.extent(pure_raw)), controller_policy_tests=16,
        archive=dict(path=str(args.archive.absolute()), **X.extent(archive_raw)), retained_files=records,
        remotely_rehashed_omitted_pins=omitted, copied=ledger, executable_bodies_rehashed_locally=False,
        full_source_maps_rehashed_locally=False, owner_inventory_bodies_rehashed_locally=False,
        transitive_source_or_tool_bodies_replayed=False, tested_controller_imported=False,
        limitations=['Historical producer successes are authenticated, not freshly executed by the consumer continuation.',
            'Full source/dependency and old-target maps plus tool/test executables were remotely rehashed but are not couriered or locally replayed.',
            'The publisher checks raw records, command/product joins and artifact bytes; it does not rerun formal joins, compiler passes or emission.',
            'Primary pure16 evidence is a retained SSH tool observation, not a remote supervisor receipt.',
            'No GPU execution, numerical acceptance, performance claim, production/launch authority or full model/long-workload claim.'],
        **{key: False for key in D.FALSE})
    for pin in records.values(): read(pin['path'], pin)
    X.require(X.read(PURE) == pure_raw and X.read(root / 'export-manifest.json') == manifest_raw
        and X.read(args.archive.absolute(), cap=96 << 20) == archive_raw, 'retained input postchecks')
    for key, (path, _) in PRIOR.items(): X.require(X.read(path) == prior_raw[key], 'prior publication unchanged')
    for name, (raw, original) in copies.items():
        if name.startswith('publication/'): X.require(X.read(Path(original['path'])) == raw, 'publication source unchanged')
    if os.path.lexists(OUT):
        X.require(OUT.resolve(strict=True) == OUT and OUT.is_dir()
            and all(p.name == 'README.md' and p.is_file() and not p.is_symlink() for p in OUT.iterdir()), 'root README only may preexist')
    else: OUT.mkdir()
    for name, (raw, _) in sorted(copies.items()):
        path = OUT / name; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(raw)
        X.require(X.read(path, cap=128 << 20) == raw, 'published exact bytes')
    with (OUT / 'result.json').open('xb') as stream:
        stream.write((json.dumps(summary, indent=2, sort_keys=True) + '\n').encode('ascii'))
    print(json.dumps(dict(output=str(OUT), files=len(copies), result=X.extent(X.read(OUT / 'result.json')))))


if __name__ == '__main__':
    main()
