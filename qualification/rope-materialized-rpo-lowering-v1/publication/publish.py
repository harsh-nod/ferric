"""Replay retained lowering metadata; never execute a compiler, finalizer or GPU."""
import argparse
import json
import os
from pathlib import Path
import re
import sys
import tarfile

import export as A

F = Path('/home/harsh/ferric-p227-integration')
OUT = F / 'qualification/rope-materialized-rpo-lowering-v1'
COMMON = F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py'
LEAF = F / 'qualification/paired-row-mlp-lowering-v1/publication/publish.py'
LEAF_SHA = 'ead92c10bbdc6f16ce71041de322e4e29c63c3885a478088be392d874ee95766'
EXPORT_SHA = '3f307cd839654edbcfd9e661b5e595d2e1fe228ad476df8eb84117e3868d9457'
CONTRACT_SHA = '694be9d199cd92b2d11ff4fe8401ad3728497bd8846b5d6924311330f3d9de1a'
PUBLIC = {
    'rope-materialized-cpu-v2': 'b780882b2700365bb25e581eee1c8d41abdda828d4f0552f80cceb6ca7ecc1bb',
    'fe2o3-partial-move-rpo-v1': 'b5f03a8741ec1795db7070a3428d0620794847ffc000551ad7dc2307368600fe',
    'fe2o3-partial-move-rpo-finalizer-v1': '3f17b6b102b256b38892a00fb8dec699f1a8b98bb4d6fec40c50da524550d2b0',
}


def replace(value, old, new):
    if isinstance(value, str):
        return new + value[len(old):] if value == old or value.startswith(old + '/') else value
    if isinstance(value, list):
        return [replace(v, old, new) for v in value]
    if isinstance(value, dict):
        return {k: replace(v, old, new) for k, v in value.items()}
    return value


def recipe_check(X, doc, lower, recipe):
    template = doc(A.TEMPLATE)
    g = lower['compiler_generation']
    old = g['predecessor']
    prior = doc(Path(lower['prior_lowering']['path']))
    X.require(old == prior['compiler_generation'], 'exact original V7 compiler generation')
    cpu = doc(Path(g['prerequisites']['cpu']['path']))
    tools = doc(Path(g['prerequisites']['tools']['path']))
    X.success(cpu, 'fe2o3-p228-rpo-compiler-cpu-result-v1')
    X.success(tools, 'fe2o3-p228-rpo-finalizer-tools-result-v1')
    roles = {k: cpu['artifacts'][k] for k in ('compiler-tests', 'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so')}
    roles.update(tools['artifacts'])
    X.require(g['roles'] == roles and g['compiler_products'] == cpu['artifacts']
              and tools['compiler_artifacts'] == cpu['artifacts']
              and g['qualified_generation'] == cpu['qualified_generation'] == tools['qualified_generation']
              and g['patches'] == tools['patches'], 'joined actual RPO compiler/finalizer products and source')
    X.require(template['compiler_generation'] == old['prerequisites']
              and set(old['roles']) == set(roles), 'same six generation roles')
    commands = replace(template['commands'], template['fresh_output'], str(A.ROW))
    by_name = {v['name']: v for v in commands}
    by_name['checked-lowering']['env']['RUSTC_WORKSPACE_WRAPPER'] = roles['fe2o3-rustc-extract']['path']
    for stage, role in (('actual-replay', 'compiler-tests'), ('actual-inert-join', 'finalizer-tests'),
                        ('emit', 'finalizer'), ('descriptor-metadata', 'metadata')):
        X.require(by_name[stage]['argv'][0] == old['roles'][role]['path'], 'original selected generation role')
        by_name[stage]['argv'][0] = roles[role]['path']
    for row in commands:
        paths = row['env']['LD_LIBRARY_PATH'].split(':')
        X.require(len(paths) == 2 and paths[0] == str(Path(old['target']) / 'debug/deps'), 'original loader path')
        row['env']['LD_LIBRARY_PATH'] = str(Path(g['target']) / 'debug/deps') + ':' + paths[1]
    changed = {v['path']: roles[k] for k, v in old['roles'].items()}
    pins = [changed.get(v['path'], v) for v in template['toolchain_and_retained_tool_pins']]
    X.require(recipe['commands'] == commands and recipe['compiler_generation'] == g['prerequisites']
              and recipe['toolchain_and_retained_tool_pins'] == pins
              and recipe['retained_modules'] == template['retained_modules']
              and recipe['symbol'] == template['symbol'], 'only reviewed generation/output recipe substitutions')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('retained_root', type=Path); p.add_argument('archive', type=Path)
    p.add_argument('archive_sha256'); p.add_argument('row_terminal', type=Path)
    p.add_argument('row_sha256'); p.add_argument('owner_terminal', type=Path); p.add_argument('owner_sha256')
    args = p.parse_args()
    X = A.module(COMMON, A.COMMON_SHA, 'rope_rpo_publication_common')
    H = A.module(LEAF, LEAF_SHA, 'rope_rpo_leaf_data')
    X.require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary -B Python')
    checked, copies = {}, {}

    def local(path, expected=None, cap=128 << 20):
        raw = X.read(path, expected, cap=cap)
        pin = dict(path=str(path), **X.extent(raw))
        X.require(str(path) not in checked or checked[str(path)] == pin, 'source changed during publication')
        checked[str(path)] = pin
        return raw

    root = args.retained_root.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical retained root')
    export_path = Path(A.__file__).resolve()
    X.require(X.extent(local(export_path))['sha256'] == EXPORT_SHA, 'reviewed exporter')
    local(COMMON); local(LEAF)
    archive_pin = dict(path=str(args.archive.absolute()), **X.extent(local(args.archive.absolute())))
    X.require(archive_pin['sha256'] == X.digest(args.archive_sha256), 'actual courier archive SHA')
    manifest_raw = local(root / 'export-manifest.json')
    manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-rope-rpo-lowering-export-v1'
              and manifest['original_root'] == str(A.E) and manifest['exporter']['sha256'] == EXPORT_SHA
              and manifest['data_helper']['sha256'] == A.COMMON_SHA
              and manifest['row_terminal'] == str(args.row_terminal) and manifest['row_sha256'] == args.row_sha256
              and manifest['owner_terminal'] == str(args.owner_terminal) and manifest['owner_sha256'] == args.owner_sha256,
              'root-pinned actual courier identity')
    expected = manifest['files']
    X.require(type(expected) is dict and len(expected) <= 160, 'bounded retained roster')
    actual = set()
    for path in root.rglob('*'):
        X.require(not path.is_symlink() and (path.is_file() or path.is_dir()), 'ordinary retained tree')
        if path.is_file():
            actual.add(str(path.relative_to(root)))
    X.require(actual == set(expected) | {'export-manifest.json'}, 'closed courier tree')

    def read(original, pin=None):
        name = str(X.relative(str(Path(original).relative_to(A.E))))
        record = expected[name]
        X.require(X.pin(record)['path'] == str(original) and (pin is None or
                  all(pin[k] == record[k] for k in ('bytes', 'sha256'))), 'original-to-retained identity')
        return local(root / name, record)

    def doc(original):
        return X.parse(read(original))

    with tarfile.open(args.archive, 'r:gz') as archive:
        names = set()
        for item in archive:
            X.relative(item.name)
            X.require(item.isfile() and item.name not in names and item.size <= 128 << 20, 'ordinary unaliased archive')
            names.add(item.name)
            stream = archive.extractfile(item)
            raw = stream.read(item.size + 1)
            pin = X.extent(manifest_raw) if item.name == 'export-manifest.json' else expected[item.name]
            X.require(X.extent(raw) == {k: pin[k] for k in ('bytes', 'sha256')}, 'archive member identity')
        X.require(names == actual, 'exact archive/tree roster')
    c = A.roster(X, read, lambda path: str(path.relative_to(A.E)) in expected,
                 args.row_terminal, args.row_sha256, args.owner_terminal, args.owner_sha256)
    X.require(c['records'] == expected and manifest['lowering_passed'] is c['passed']
              and manifest['attempted_stages'] == c['attempted'], 'rederived original evidence roster')
    lower, owner, recipe = c['lower'], c['owner'], c['recipe']
    recipe_check(X, doc, lower, recipe)
    for name, sha in PUBLIC.items():
        raw = local(F / 'qualification' / name / 'result.json')
        X.require(X.extent(raw)['sha256'] == sha, 'published prerequisite checkpoint unchanged')
    command = doc(A.OWNER / 'command.json')
    X.require(command['argv'] == ['/usr/bin/python3', '-B', str(A.PACKAGE / 'run.py'), A.PACKAGE_SHA,
              lower['candidate_cpu']['path'], A.CPU_SHA, A.TOOLS_SHA, A.TOOLS_OWNER_SHA, '--child']
              and command['env'] == recipe['commands'][0]['env'] and command['affinity'] == [8, 9]
              and command['nice'] == 10 and command['deadline_seconds'] == 10800
              and 0 < command['address_space_bytes'] <= 12 << 30 and 0 < command['file_cap_bytes'] <= 1 << 30
              and command['core_bytes'] == 0 and command['gpu_execution'] is False, 'actual bounded owner command')
    start = doc(A.OWNER / 'started.json')
    parent = start['parent']
    X.require(parent['pid'] == parent['pgid'] == parent['sid'] and type(parent['pid']) is int
              and parent['pid'] > 0 and parent['ppid'] == start['supervisor_pid']
              and parent['uid'] == 9661 and parent['starttime'] > 0
              and any(v['event'] == 'owned' and v['identity'] == parent
                      for v in owner['owned']['lineage']), 'owned direct process group and observed identity')
    substitutions = {}
    for key, artifact, field in (('@candidate.semantic.sha256', 'prefix-tiles-semantic.bin', 'sha256'),
            ('@candidate.handoff.sha256', 'prefix-tiles.handoff-v3', 'sha256'),
            ('@candidate.handoff.bytes', 'prefix-tiles.handoff-v3', 'bytes'),
            ('@candidate.image.sha256', 'emitted/artifact.hsaco', 'sha256'),
            ('@candidate.image.bytes', 'emitted/artifact.hsaco', 'bytes')):
        if artifact in lower['artifacts']:
            substitutions[key] = lower['artifacts'][artifact][field]
    phases, streams = {}, {}
    for name, intended in zip(c['attempted'], recipe['commands']):
        command, start, result = (doc(A.ROW / (name + '-' + suffix))
                                  for suffix in ('command.json', 'started.json', 'result.json'))
        target = H.substitute({k: v for k, v in intended.items() if k not in ('name', 'cwd')}, substitutions)
        X.require(0 < command['deadline_seconds'] <= target['deadline_seconds'], 'bounded stage deadline')
        target['deadline_seconds'] = command['deadline_seconds']
        X.require(command == target and start['pid'] == start['pgid'] and type(start['pid']) is int
                  and start['pid'] > 0 and result['reason'] is None and result['group_absent'] is True
                  and type(result['exit_code']) is int and result['cache_bytes'] <= 6 << 30, 'actual recipe/natural leaf')
        X.require(result['exit_code'] == 0 or (not c['passed'] and name == c['attempted'][-1]
                  and name not in [v['name'] for v in lower['commands']]), 'only last unindexed attempted stage may fail')
        streams[name] = {key: read(A.ROW / (name + '-' + key)) for key in ('stdout', 'stderr')}
        X.require(all(X.extent(raw)['sha256'] == result[key + '_sha256'] for key, raw in streams[name].items()),
                  'actual stage stream hashes')
        phases[name] = result
    C = A.module(root / str((A.PACKAGE / 'contracts.py').relative_to(A.E)), CONTRACT_SHA, 'rope_rpo_contract_data')
    for name in ('actual-replay', 'actual-inert-join'):
        if name in phases and phases[name]['exit_code'] == 0:
            C.exact_test(streams[name]['stdout'], recipe['commands'][A.STAGES.index(name)]['argv'][2])
    for name, markers in (('checked-lowering', (b'stage=pre-ranked status=complete', b'stage=neutral status=complete',
            b'stage=target-before-llvm status=complete')), ('actual-replay', (b'stage=recipe status=complete roots=15',
            b'stage=ranked status=complete', b'stage=formal status=complete roots=15 unresolved=8'))):
        if name in phases and phases[name]['exit_code'] == 0:
            X.require(all(m in streams[name]['stdout'] + streams[name]['stderr'] for m in markers), 'actual checked/replay markers')
    metadata, emission = None, None
    if c['passed']:
        image = lower['artifacts']['emitted/artifact.hsaco']
        metadata = C.metadata(streams['descriptor-metadata']['stdout'], image)
        emit = recipe['commands'][A.STAGES.index('emit')]
        tools = recipe['toolchain_and_retained_tool_pins']
        worker = next(pin for pin in tools if pin['path'] == emit['argv'][5])
        libs = {Path(pin['path']).name: pin['sha256'] for pin in tools if '/gfx950-reviewed-libs/' in pin['path']}
        emitted = {k.removeprefix('emitted/'): v for k, v in lower['artifacts'].items() if k.startswith('emitted/')}
        emission = C.emission(read(A.ROW / 'emitted/receipt.txt'), emitted, worker, emit['argv'][8], emit['argv'][9], libs)
        X.require(all(emitted['source.handoff-v3'][k] == lower['artifacts']['prefix-tiles.handoff-v3'][k]
                      for k in ('bytes', 'sha256')), 'actual captured handoff emitted')
        X.require(b'authority none' in streams['extract-retained']['stdout']
                  and b'gpu_execution false' in streams['extract-retained']['stdout'], 'inert extraction')
        X.require(all(v in streams['elf-notes']['stdout'] for v in (b'.group_segment_fixed_size: 512',
                  b'.private_segment_fixed_size: 0', b'.wavefront_size: 64'))
                  and recipe['symbol'].encode() in streams['disassembly']['stdout']
                  and b's_endpgm' in streams['disassembly']['stdout'], 'actual symbol and resource metadata')

    def copy(original, destination):
        X.relative(destination)
        X.require(destination not in copies, 'unique publication destination')
        raw = read(original)
        copies[destination] = (dict(path=str(original), **X.extent(raw)), raw)

    copy(args.row_terminal, 'lowering/' + args.row_terminal.name)
    copy(args.owner_terminal, 'owner/' + args.owner_terminal.name)
    copy(A.ROW / 'recipe.json', 'lowering/recipe.json')
    for name in A.OWNER_FILES - {'before.json', 'after.json'}:
        copy(A.OWNER / name, 'owner/' + name)
    for name in c['attempted']:
        for suffix in A.SUFFIXES:
            copy(A.ROW / (name + '-' + suffix), 'raw/' + name + '-' + suffix)
    for row in recipe['fixture']:
        copy(A.ROW / 'fixture' / row['destination'], 'fixture/' + row['destination'])
    for name in ('manifest.json', 'run.py', 'test_run.py', 'contracts.py', 'README.md'):
        copy(A.PACKAGE / name, 'controller/' + name)
    for name in ('extracted/module.ll', 'emitted/receipt.txt'):
        if name in c['discovered_artifacts']:
            copy(A.ROW / name, 'lowering/' + name)
    for path, name in ((Path(__file__).resolve(), 'publication/publish.py'), (export_path, 'publication/export.py')):
        raw = local(path)
        copies[name] = (dict(path=str(path), **X.extent(raw)), raw)
    X.require(sum(len(raw) for _, raw in copies.values()) <= 32 << 20, 'bounded text/source publication')
    X.require(not os.path.lexists(OUT) or (OUT.resolve(strict=True) == OUT and OUT.is_dir()
              and {p.name for p in OUT.iterdir()} <= {'README.md'} and
              (not (OUT / 'README.md').exists() or (OUT / 'README.md').is_file())), 'fresh output, root README only')
    if (OUT / 'README.md').exists():
        local(OUT / 'README.md')
    for path, pin in checked.items():
        X.read(Path(path), pin, cap=128 << 20)
    OUT.mkdir(exist_ok=True)
    ledger = {}
    for name, (original, raw) in sorted(copies.items()):
        target = OUT / X.relative(name)
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(raw)
        X.read(target, X.extent(raw))
        ledger[name] = dict(original=original, **X.extent(raw))
    for path, pin in checked.items():
        X.read(Path(path), pin, cap=128 << 20)
    result = dict(schema='ferric-p228-rope-rpo-lowering-publication-v1', publication_passed=True,
        lowering_passed=c['passed'], lower=expected[str(args.row_terminal.relative_to(A.E))],
        owner=expected[str(args.owner_terminal.relative_to(A.E))], archive=archive_pin,
        source_manifest=lower['source_manifest'], candidate_cpu=lower['candidate_cpu'],
        compiler_generation=lower['compiler_generation'], prerequisite_publications=PUBLIC,
        completed_stages=[r['name'] for r in lower['commands']], attempted_stages=c['attempted'], phases=phases,
        lowering_error=lower['error'], owner_error=owner['error'], discovered_artifacts=c['discovered_artifacts'],
        qualified_hsaco=lower['artifacts'].get('emitted/artifact.hsaco') if c['passed'] else None,
        unresolved_runtime_requirements=lower['unresolved_runtime_requirements'], metadata=metadata, emission=emission,
        fixture_source_join_checked=True, local_courier_bodies_rehashed=True, binary_artifact_bodies_published=False,
        text_llvm_published='extracted/module.ll' in c['discovered_artifacts'],
        text_isa_published='disassembly' in c['attempted'], transitive_compiler_tool_inputs_rehashed=False,
        compiler_or_finalizer_rerun=False, tested_controller_imported=False,
        isa_review_accepted=False, arithmetic_order_review_complete=False,
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        runtime_requirements_discharged=False, launch_authority=False, production_authority=False, performance_claim=False,
        limitations=['Only recorded stages and retained bytes are replayed; no compiler, proof, finalizer or GPU is run.',
            'Generation product identities are joined to published qualification; product/dependency bodies are not rehashed here.',
            'A failed attempt may retain partial artifacts without qualifying any image or downstream use.',
            'Recorded storage refusals are diagnostics; limit+1 is not an estimate of total required storage.',
            'Text metadata and ISA are retained, not an independent arithmetic or ISA acceptance review.'],
        retained_files=expected, locally_rehashed_inputs=list(checked.values()), files=ledger)
    with (OUT / 'result.json').open('x') as stream:
        json.dump(result, stream, sort_keys=True, indent=2); stream.write('\n')
    print(json.dumps(dict(result=dict(path=str(OUT / 'result.json'), **X.extent(X.read(OUT / 'result.json'))),
                         files=len(ledger), lowering_passed=c['passed'])))


if __name__ == '__main__':
    main()
