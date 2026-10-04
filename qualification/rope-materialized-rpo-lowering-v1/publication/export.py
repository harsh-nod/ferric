"""Bounded data-only courier for one terminal RoPE/RPO lowering attempt."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import sys
import tarfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROW = E / 'row-rope-materialized-rpo-checked-probe-v228-v1'
OWNER = E / 'rope-materialized-rpo-checked-probe-owner-v228-v1'
PACKAGE = E / 'p228-rope-materialized-rpo-lowering-v1'
PACKAGE_SHA = '6ed02271fa204a9d2f00f81751425bd26860456ff753c360b079167b1d4d87b0'
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
SOURCE_SHA = '7f1f447852f01bad9b6dedb9b40a7969961d45861592465548e6401615318d23'
CPU_SHA = '8dcb4f2b326ab909c52039273515f44eb4f88cd91a967c8816f866d5514c864a'
TOOLS_SHA = '39eab92ede34991b08c168e827c7111c13533627cbf2efd3d248c6742a92660b'
TOOLS_OWNER_SHA = 'e919fb520672be2a909fd0ec8301bc9923210680f818e411b7db459ffc3059dd'
TEMPLATE = E / 'p228-reciprocal-checked-probe-v7/recipe.json'
TEMPLATE_SHA = 'f500b1c3d3218b53e9ba1df92d6f048a41f05873c9539f16bc69b20f85575a52'
STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join',
          'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
SUFFIXES = ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')
FALSE = ('gpu_execution', 'production_authority', 'launch_authority', 'numerical_acceptance',
         'performance_claim', 'runtime_requirements_discharged', 'full_model_acceptance')
SNAPSHOTS = {'recipe.json', 'before.json', 'after.json',
             'package-sources-before.json', 'package-sources-after.json'}
OWNER_FILES = {'before.json', 'after.json', 'command.json', 'started.json',
               'owned-result.json', 'stdout', 'stderr'}
RUST = {'src/lib.rs', 'src/wave_numerics_v1.rs', 'src/head_rope_numerics_v3.rs',
        'src/prefix_reciprocal_numerics_v1.rs', 'src/attention_online.rs',
        'src/output_projection_numerics_v5.rs', 'src/prefix_tiles_numerics_v6.rs',
        'src/prefix_rope_materialized_numerics_v1.rs'}
ARTIFACTS = {'prefix-tiles-semantic.bin', 'prefix-tiles-neutral-kir.bin',
             'prefix-tiles-target-kir.bin', 'prefix-tiles.handoff-v3',
             'emitted/source.handoff-v3', 'emitted/compiler.handoff-v2',
             'emitted/artifact.hsaco', 'emitted/receipt.txt',
             'extracted/formal.archive', 'extracted/module.ll'}


def module(path, digest, name):
    path = Path(path).absolute()
    if path.resolve(strict=True) != path or not path.is_file() or path.stat().st_size > 64 << 10:
        raise ValueError('canonical bounded data helper')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError('data helper digest')
    value = types.ModuleType(name)
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def roster(X, reader, exists, row_path, row_sha, owner_path, owner_sha):
    row_path, owner_path = Path(row_path), Path(owner_path)
    X.require(row_path.parent == ROW and owner_path.parent == OWNER
              and row_path.name in ('complete.json', 'failed.json')
              and owner_path.name == row_path.name, 'explicit same-outcome terminal namespaces')
    X.digest(row_sha); X.digest(owner_sha)
    records = {}

    def add(path, expected=None):
        path = Path(path)
        name = str(X.relative(str(path.relative_to(E))))
        X.require('/target/' not in '/' + name + '/' and '/tmp/' not in '/' + name + '/',
                  'no cache or temporary tree')
        raw = reader(path, expected)
        record = dict(path=str(path), **X.extent(raw))
        X.require(name not in records or records[name] == record, 'conflicting original identity')
        records[name] = record
        return raw

    def doc(path, expected=None):
        return X.parse(add(path, expected))

    lower, owner = doc(row_path), doc(owner_path)
    X.require(records[str(row_path.relative_to(E))]['sha256'] == row_sha
              and records[str(owner_path.relative_to(E))]['sha256'] == owner_sha, 'actual terminal hashes')
    passed = row_path.name == 'complete.json'
    X.require(lower['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-result-v1'
              and owner['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-owned-result-v1'
              and lower['passed'] is owner['passed'] is passed
              and lower['postcheck_errors'] == owner['postcheck_errors'] == [], 'terminal schemas and clean postchecks')
    X.require(all(v[k] is False for v in (lower, owner) for k in FALSE), 'no downstream authority')
    X.require(all(owner[k] == lower[k] for k in ('package_manifest', 'candidate_cpu',
              'source_manifest', 'prior_lowering', 'compiler_generation')), 'owner/inner generation joins')
    X.require(all(lower[k] is passed for k in ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted'))
              and lower['frontend_recipe_is_diagnostic'] is True
              and lower['unresolved_runtime_requirements'] == (8 if passed else None), 'checked outcome scope')
    if passed:
        X.require(lower['error'] is owner['error'] is None
                  and owner['completion'] == records[str(row_path.relative_to(E))], 'successful owner completion')
    else:
        X.require(type(lower['error']) is str and lower['error'] and type(owner['error']) is str
                  and owner['error'] and owner['completion'] is None, 'retained failed completion')
    X.require(set(owner['raw']) == OWNER_FILES, 'seven owned raw records')
    for name, pin in owner['raw'].items():
        X.require(X.pin(pin)['path'] == str(OWNER / name), 'owned raw identity')
        add(OWNER / name, pin)
    owned = doc(OWNER / 'owned-result.json')
    X.require(owner['owned'] == owned and owned['exit_code'] == (0 if passed else 1)
              and owned['reason'] is None and owned['cleanup_signalled'] is False
              and owned['owned_groups_absent'] is True and owned['owned_processes_reaped'] is True,
              'natural terminal owner, no forced cleanup')
    for root in (ROW, OWNER):
        before, after = doc(root / 'before.json'), doc(root / 'after.json')
        X.require(all(after[k] == v for k, v in before.items()), 'recorded source/tool/cache postchecks')
    for name in ('package-sources-before.json', 'package-sources-after.json'):
        add(ROW / name)
    X.require(doc(ROW / 'package-sources-before.json') == doc(ROW / 'package-sources-after.json'),
              'unchanged recorded package dependencies')
    recipe = doc(ROW / 'recipe.json')
    X.require(recipe['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-recipe-v1'
              and tuple(r['name'] for r in recipe['commands']) == STAGES
              and recipe['fresh_output'] == str(ROW) and recipe['fresh_target'] == str(ROW / 'target'),
              'closed nine-stage recipe')
    completed = [r['name'] for r in lower['commands']]
    X.require(completed == list(STAGES[:len(completed)]) and len(completed) <= 9
              and (not passed or len(completed) == 9), 'ordered completed stage prefix')
    attempted = []
    for stage in STAGES:
        present = [exists(ROW / (stage + '-' + suffix)) for suffix in SUFFIXES]
        X.require(not any(present) or all(present), 'partial raw stage tree refused')
        if any(present):
            attempted.append(stage)
            for suffix in SUFFIXES:
                add(ROW / (stage + '-' + suffix))
    X.require(attempted == list(STAGES[:len(attempted)]) and len(completed) <= len(attempted) <= len(completed) + 1,
              'one ordered attempt, including any unindexed failing leaf')
    X.require(attempted and (not passed or attempted == list(STAGES)), 'actual attempted phase inventory')
    for row in lower['commands']:
        for key, suffix in zip(('command', 'started', 'result', 'stdout', 'stderr'), SUFFIXES):
            X.require(X.pin(row[key])['path'] == str(ROW / (row['name'] + '-' + suffix)), 'indexed leaf identity')
            add(Path(row[key]['path']), row[key])
    X.require(set(lower['artifacts']) <= ARTIFACTS and (not passed or set(lower['artifacts']) == ARTIFACTS),
              'closed indexed artifact family')
    discovered = {}
    for name in sorted(ARTIFACTS):
        path = ROW / name
        if exists(path):
            add(path, lower['artifacts'].get(name))
            discovered[name] = records[str(path.relative_to(E))]
    X.require(set(lower['artifacts']) <= set(discovered), 'no missing recorded artifact')
    for name, pin in lower['artifacts'].items():
        X.require(X.pin(pin)['path'] == str(ROW / name), 'artifact original namespace')
    X.require(lower['package_manifest']['path'] == str(PACKAGE / 'manifest.json')
              and lower['package_manifest']['sha256'] == PACKAGE_SHA, 'frozen four-file controller')
    package = doc(PACKAGE / 'manifest.json', lower['package_manifest'])
    X.require(package['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-package-v1'
              and len(package['files']) == 4 and {r['path'] for r in package['files']}
              == {'run.py', 'contracts.py', 'test_run.py', 'README.md'}, 'closed lowering package')
    for row in package['files']:
        add(PACKAGE / X.relative(row['path']), row)
    X.require(lower['candidate_cpu']['sha256'] == CPU_SHA and lower['source_manifest']['sha256'] == SOURCE_SHA,
              'actual V2 source and CPU prerequisite')
    cpu = doc(Path(lower['candidate_cpu']['path']), X.pin(lower['candidate_cpu']))
    X.success(cpu, 'ferric-p228-rope-materialized-cpu-result-v1')
    X.require(cpu['tests_passed'] == 33 and cpu['tests_ignored'] == 1 and cpu['source_unchanged'] is True
              and cpu['overlay'] == lower['source_manifest'], 'unchanged qualified arithmetic fixture')
    doc(Path(lower['source_manifest']['path']), X.pin(lower['source_manifest']))
    doc(Path(lower['prior_lowering']['path']), X.pin(lower['prior_lowering']))
    fixture = {**cpu['lowering_sources'], **cpu['lowering_fixture_pins']}
    X.require(set(fixture) == RUST | {'Cargo.toml', 'Cargo.lock'} and len(recipe['fixture']) == 10
              and {r['destination']: r['source'] for r in recipe['fixture']} == fixture, 'eight Rust / ten-file fixture')
    for name, pin in fixture.items():
        add(Path(X.pin(pin)['path']), pin)
        add(ROW / 'fixture' / X.relative(name), pin)
    generation = lower['compiler_generation']
    expected = {'cpu': X.CPU_SHA, 'cpu_owner': X.OWNER_SHA, 'tools': TOOLS_SHA, 'tools_owner': TOOLS_OWNER_SHA}
    X.require(set(generation['prerequisites']) == set(expected), 'four RPO generation receipts')
    for name, digest in expected.items():
        pin = X.pin(generation['prerequisites'][name])
        X.require(pin['sha256'] == digest, 'actual RPO prerequisite: ' + name)
        doc(Path(pin['path']), pin)
    add(TEMPLATE)
    X.require(records[str(TEMPLATE.relative_to(E))]['sha256'] == TEMPLATE_SHA, 'unchanged original V7 recipe')
    X.require(len(records) <= 160 and sum(r['bytes'] for r in records.values()) <= 256 << 20,
              'bounded evidence-only courier')
    return dict(lower=lower, owner=owner, cpu=cpu, recipe=recipe, passed=passed,
                attempted=attempted, discovered_artifacts=discovered, records=records)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('common_helper', type=Path)
    p.add_argument('row_terminal', type=Path); p.add_argument('row_sha256')
    p.add_argument('owner_terminal', type=Path); p.add_argument('owner_sha256')
    p.add_argument('archive', type=Path)
    args = p.parse_args()
    X = module(args.common_helper, COMMON_SHA, 'rope_rpo_export_data')
    X.require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary -B Python')
    out = args.archive.absolute()
    X.require(out.parent == E and re.fullmatch(r'rope-rpo-lowering-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
              and not os.path.lexists(out), 'fresh bounded archive namespace')
    for kind, cap in ((resource.RLIMIT_AS, 1 << 30), (resource.RLIMIT_CPU, 180),
                      (resource.RLIMIT_FSIZE, 96 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    reader = lambda path, expected=None: X.read(path, expected, cap=128 << 20)
    c = roster(X, reader, os.path.lexists, args.row_terminal, args.row_sha256, args.owner_terminal, args.owner_sha256)
    records = c['records']
    manifest = dict(schema='ferric-p228-rope-rpo-lowering-export-v1', original_root=str(E), files=records,
        row_terminal=str(args.row_terminal), row_sha256=args.row_sha256,
        owner_terminal=str(args.owner_terminal), owner_sha256=args.owner_sha256,
        lowering_passed=c['passed'], attempted_stages=c['attempted'],
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(reader(Path(__file__).resolve()))),
        data_helper=dict(path=str(args.common_helper.absolute()), **X.extent(reader(args.common_helper.absolute()))),
        target_cache_exported=False, transitive_source_tree_exported=False, gpu_execution=False)
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, record in sorted(records.items()):
            raw = reader(Path(record['path']), record)
            member = tarfile.TarInfo(name)
            member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, sort_keys=True, indent=2) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json')
        member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for record in records.values():
        reader(Path(record['path']), record)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(reader(out))), files=len(records),
        uncompressed_bytes=sum(r['bytes'] for r in records.values()), lowering_passed=c['passed'])))


if __name__ == '__main__':
    main()
