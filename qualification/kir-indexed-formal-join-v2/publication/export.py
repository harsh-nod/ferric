"""Small data-only courier for an actually completed indexed inert-join qualification."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import tarfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CASE = E / 'kir-indexed-formal-join-cpu-v228-v2'
OWNER = E / 'kir-indexed-formal-join-cpu-owner-v228-v2'
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
SOURCE_SHA = 'd9d235a2f0bb2ae185c41f38d8406124c8233d3259e53751ccf47a17755b5dc2'
PACKAGE_SHA = '6420388f8444a923dd19363e658809bb369d5096ad670d683c1f7ef75e5cc349'
CONTROLLER = {'run.py': '02621c9ccbad3f9843e0c0250f9a3546de2c2bee8ac01b502c681c8b7576b6b6',
    'test_run.py': '3903034ed45209d6fbc12206484425cc7a2a133945de9ecbf7263581888699b0',
    'README.md': '4138183916783659167324b6dde08ad68a2a8b8575e379fff88b3ba4bafd3f6d'}
PHASES = {'rustfmt', 'rustfmt-check', 'metadata', 'finalizer-build-tests', 'finalizer-list',
    'finalizer-ignored-list', 'finalizer-tests', 'actual-inert-join'} | {
    'lower-' + suffix for suffix in ('build-tests', 'list', 'ignored-list', 'tests', 'indexed-tests')}
SNAPSHOTS = {'sources-unformatted.json', 'configurations-before.json', 'sources-before.json',
    'dependencies-before.json', 'original-after.json', 'sources-after.json', 'inputs-after.json',
    'prior-dependencies-after.json', 'configurations-after.json', 'dependencies-after.json'}
OWNER_OMITTED = {'after.json', 'old-targets-before.json'}
SELECTOR = 'linux::wave_qkv_attention_output_tiles_v6::tests::wave_emission_actual_retained_v6_passes_full_inert_join'


def common(path):
    path = path.absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical data helper')
    raw = path.read_bytes()
    if len(raw) > 1 << 20 or hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact data helper')
    module = types.ModuleType('indexed_v2_publication_common')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return module


INNER = (300367, '4548c7ee32206f6fc94bf54b5c50e13c500d2f4381e58092094893d9d80d4bfa')
OUTER = (35109, '21205fbed2b56d5a27b4c1fbb2f4f3d5f0e810b822a518e3f875e3083553954e')
BASELINE = dict(path=str(E / 'kir-indexed-formal-join-baseline-cpu-v228-v1/complete.json'),
    bytes=244380, sha256='a95ee1e3fb32cd50745a0bba5274e5688b72e7ee3f26de25ddbe0cd936e667ee')
BASELINE_OWNER = dict(path=str(E / 'kir-indexed-formal-join-baseline-cpu-owner-v228-v1/complete.json'),
    bytes=19845, sha256='87ae6c49d7ec72a35d6e13339cc501323d10d5e78ce212c77b336ff8a675012c')
CORRECTED = 'crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1/retained_nested_enum_transport_v1_tests.rs'


def source_maps(X, value, proposal, reader):
    def get(name):
        pin = value['raw'][name]
        return X.parse(reader(Path(pin['path']), pin))
    def relative(rows, root):
        result = {}
        for name, item in rows.items():
            pin = X.pin(item['pin'])
            X.require(pin['path'] == name, 'snapshot key identity')
            result[str(Path(name).relative_to(root))] = {key: pin[key] for key in ('bytes', 'sha256')}
        return result
    before, after = get('sources-before.json'), get('sources-after.json')
    X.require(before == after and len(before) == 5785, 'unchanged full V2 source map')
    original = relative(get('original-after.json'), E / 'rpo-compiler-cpu-v228-v2/source/fe2o3')
    unformatted = relative(get('sources-unformatted.json'), CASE / 'source/fe2o3')
    formatted = relative(before, CASE / 'source/fe2o3')
    expected = dict(original); changed = {row['path'] for row in proposal['files']}
    X.require(len(original) == 5783, 'original source census')
    for row in proposal['files']:
        X.require(original.get(row['path']) == row['before'], 'five exact original source preimages')
        expected[row['path']] = row['after']
    X.require(unformatted == expected and set(formatted) == set(expected)
        and all(formatted[name] == pin for name, pin in expected.items() if name not in changed),
        'only five declared bodies formatted and two source additions')
    for name, pin in value['formatted_sources'].items():
        X.require(formatted[name] == {key: pin[key] for key in ('bytes', 'sha256')}, 'compiled formatted source pin')
    for first, second in (('configurations-before.json', 'configurations-after.json'),
                          ('dependencies-before.json', 'dependencies-after.json')):
        X.require(get(first) == get(second), 'unchanged ' + first)
    X.require(all(value['raw']['original-after.json'][key] == value['source_generation'][key]
                  for key in ('bytes', 'sha256')), 'prior RPO source generation unchanged')
    return dict(original_files=5783, candidate_files=5785, overlay_files=5, additions=2,
        source_transition_checked_remotely=True, source_and_dependency_postchecks_equal=True)


def roster(X, reader, complete_sha, owner_sha):
    X.require(X.digest(complete_sha) == INNER[1] and X.digest(owner_sha) == OUTER[1],
        'exact actual V2 terminals, not unexecuted V1 success')
    records, omitted = {}, {}

    def add(path, expected=None, retained=True, cap=32 << 20):
        path = Path(path)
        name = str(X.relative(str(path.relative_to(E))))
        if retained:
            X.require('/target/' not in '/' + name + '/' and path.suffix not in ('.so', '.rlib', '.rmeta'),
                      'no build product body in archive')
        raw = reader(path, expected, cap=cap)
        pin = dict(path=str(path), **X.extent(raw))
        target = records if retained else omitted
        X.require(name not in target or target[name] == pin, 'one exact original identity')
        target[name] = pin
        return raw

    raw = add(CASE / 'complete.json', dict(zip(('bytes', 'sha256'), INNER)))
    outer = add(OWNER / 'complete.json', dict(zip(('bytes', 'sha256'), OUTER)))
    X.require(X.extent(raw)['sha256'] == X.digest(complete_sha)
              and X.extent(outer)['sha256'] == X.digest(owner_sha), 'root-supplied actual terminal pins')
    value, owner = X.parse(raw), X.parse(outer)
    X.success(value, 'ferric-p228-kir-indexed-formal-join-cpu-result-v2')
    X.success(owner, 'ferric-p228-kir-indexed-formal-join-owned-result-v2')
    X.require(owner['completion'] == records[str((CASE / 'complete.json').relative_to(E))]
              and owner['package'] == value['package'] and owner['proposal'] == value['proposal']
              and owner['actual_capture_join_passed'] is True, 'actual owner/source/join binding')
    X.require(value['staged_join_qualification_completed'] is True and value['source_unchanged'] is True
              and value['actual_capture_join_passed'] is True
              and all(value[key] is False for key in ('limits_changed', 'fresh_compiler_built',
                  'fresh_hsaco_emitted', 'full_compiler_cohort_requalified', 'gpu_execution',
                  'numerical_acceptance', 'production_authority', 'performance_claim'))
              and value['prior_compiled_library_inventory_available'] is True, 'staged CPU inert-join scope only')
    outcome = owner['owned']
    X.require(type(outcome['exit_code']) is int and outcome['exit_code'] == 0 and outcome['reason'] is None
              and outcome['cleanup_signalled'] is False and outcome['owned_groups_absent'] is True
              and outcome['owned_processes_reaped'] is True, 'natural successful owned process tree')
    raw_names = {phase + '-' + suffix for phase in PHASES for suffix in X.SUFFIXES}
    X.require(set(value['phases']) == PHASES and set(value['raw']) == raw_names | SNAPSHOTS,
              'thirteen actual leaves and exact snapshot roster')
    for name, pin in value['raw'].items():
        X.require(X.pin(pin)['path'] == str(CASE / name), 'exact original raw namespace')
        add(pin['path'], pin, name not in SNAPSHOTS)
    for name, phase in value['phases'].items():
        X.require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
                  and phase['group_absent'] is True, 'every phase naturally succeeds')
        X.require(X.parse(reader(CASE / (name + '-result.json'), value['raw'][name + '-result.json'])) == phase
                  and all(value['raw'][name + '-' + key]['sha256'] == phase[key + '_sha256']
                          for key in ('stdout', 'stderr')), 'actual phase/raw/stream binding')
    for name in sorted(X.OWNER_FILES):
        add(OWNER / name, retained=name not in OWNER_OMITTED)
    X.require(X.parse(reader(OWNER / 'owned-result.json')) == outcome, 'actual outer owned result body')
    X.require(value['package']['path'] == str(E / 'p228-kir-indexed-formal-join-cpu-v2/manifest.json')
              and value['package']['sha256'] == PACKAGE_SHA, 'frozen controller package')
    package = X.parse(add(value['package']['path'], value['package']))
    X.require(package['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-package-v2'
              and len(package['files']) == 3 and {row['path'] for row in package['files']} == set(CONTROLLER),
              'closed three-file controller package')
    for row in package['files']:
        X.require(row['sha256'] == CONTROLLER[row['path']], 'qualified controller source identity')
        add(Path(value['package']['path']).parent / X.relative(row['path']), row)
    X.require(value['proposal']['path'] == str(E / 'p228-kir-indexed-formal-join-v2/source-manifest.json')
              and value['proposal']['sha256'] == SOURCE_SHA, 'reviewed indexed source proposal')
    proposal = X.parse(add(value['proposal']['path'], value['proposal']))
    X.require(proposal['schema'] == 'ferric-p228-kir-indexed-formal-join-source-v2'
              and len(proposal['files']) == 5 and sum(row['before'] is None for row in proposal['files']) == 2
              and set(value['formatted_sources']) == {row['path'] for row in proposal['files']},
              'five selected V2 bodies with two additions')
    X.require(value['added_tests'] == proposal['added_tests'] and len(set(value['added_tests'])) == 20,
              'actual twenty-name source test contract')
    tests, inventory = value['tests'], value['lower_inventory']
    lower, subset, finalizer = (tests[key] for key in ('lower', 'indexed_subset_repeat', 'finalizer'))
    X.require(set(tests) == {'lower', 'indexed_subset_repeat', 'finalizer'}
              and inventory['names'] == lower['names'] and inventory['ignored_names'] == lower['ignored_names']
              and inventory['added_names'] == subset['names'] == sorted(value['added_tests'])
              and subset['passed'] == 20 and subset['ignored'] == 0 and subset['ignored_names'] == []
              and set(subset['names']) <= set(lower['names']) and not set(subset['names']) & set(lower['ignored_names'])
              and lower['passed'] + lower['ignored'] == len(lower['names'])
              and finalizer['passed'] == 190 and finalizer['ignored'] == 15 and len(finalizer['names']) == 205,
              'dynamic full library and distinct repeated subset, unchanged finalizer counts')
    X.require(value['actual_join'] == dict(test=SELECTOR, passed=1, ignored=0, filtered_out=204,
              exit_code=0, actual_capture_join_passed=True, fresh_hsaco_emitted=False), 'actual separate retained join result')
    X.require(value['original_source_baseline'] == proposal['baseline_control'] == BASELINE
        and value['original_source_baseline_owner'] == BASELINE_OWNER
        and value['corrected_existing_test'] == proposal['changed_existing_test']['name']
        and proposal['baseline_failure_reproduced'] is True
        and proposal['work_limit'] == 1 << 30 and proposal['storage_limit'] == 128 << 20
        and all(proposal[key] is False for key in ('changes_live_join', 'changes_canonical_bytes', 'changes_authority')),
        'measured baseline and unchanged proof/resource boundaries')
    for row in proposal['files']:
        X.require(row['source'] == 'draft/' + row['path'], 'exact proposal source mapping')
        add(Path(value['proposal']['path']).parent / X.relative(row['source']), row['after'])
        pin = X.pin(value['formatted_sources'][row['path']])
        X.require(pin['path'] == str(CASE / 'source/fe2o3' / X.relative(row['path'])), 'actual formatted source mapping')
        add(pin['path'], pin)
    add(Path(value['proposal']['path']).parent / 'README.md', dict(bytes=3294,
        sha256='35765ba5161fd6a19b53f9d3357af18c7d3767a2ec9f9459ed1f6dd6bd2622bb'))
    X.require(set(value['artifacts']) == {'lower-tests', 'finalizer-tests'}, 'two actual selected test executables')
    for pin in value['artifacts'].values():
        X.require(Path(X.pin(pin)['path']).is_relative_to(CASE / 'target'), 'actual new target product')
        add(pin['path'], pin, retained=False, cap=128 << 20)
    X.require(len(records) == 88 and len(omitted) == 14
              and sum(pin['bytes'] for pin in records.values()) <= 32 << 20, 'bounded exact small courier')
    sources = source_maps(X, value, proposal, reader)
    return value, owner, records, omitted, sources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('complete_sha256'); parser.add_argument('owner_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper)
    out = args.archive.absolute()
    X.require(out.parent == E and re.fullmatch('kir-indexed-v2-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
              and not os.path.lexists(out), 'fresh bounded archive')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 30 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    _, _, records, omitted, sources = roster(X, X.read, args.complete_sha256, args.owner_sha256)
    manifest = dict(schema='ferric-p228-kir-indexed-v2-export-v1', original_root=str(E),
        complete_sha256=args.complete_sha256, owner_sha256=args.owner_sha256, files=records,
        remotely_rehashed_omitted_pins=omitted, remote_source_checks=sources, snapshot_bodies_exported=False,
        owner_inventory_bodies_exported=False, artifact_bodies_exported=False, full_source_tree_exported=False,
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(X.read(Path(__file__).resolve()))))
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, pin in sorted(records.items()):
            raw = X.read(Path(pin['path']), pin)
            member = tarfile.TarInfo(name); member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json'); member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for pin in (*records.values(), *omitted.values()):
        path = Path(pin['path'])
        X.read(path, pin, cap=(128 if path.is_relative_to(CASE / 'target') else 32) << 20)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out))), files=len(records),
        remotely_rehashed_not_exported=len(omitted), artifact_bodies_exported=False)))


if __name__ == '__main__':
    main()
