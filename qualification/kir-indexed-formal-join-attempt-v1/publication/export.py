"""Data-only courier for the actual stopped indexed-join CPU attempt."""
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
CASE = E / 'kir-indexed-formal-join-cpu-v228-v1'
OWNER = E / 'kir-indexed-formal-join-cpu-owner-v228-v1'
CPU = E / 'rpo-compiler-cpu-v228-v2'
INNER = (142699, '88d0fa37ebdbe9bda0501af8693f664c8dbfb5b909b586f2b930b16d37fb7b89')
OUTER = (20777, 'b845ba6e230625237605601ced70efba5bc8427346076256205b6014587fc183')
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
SOURCE_SHA = '6501750a4ac345cbd47e543954580775eab5b8df406bc302117906b5c1265560'
PACKAGE_SHA = '38206925bd968e7e883ad9c7f1184e9bf987d68f0e655b7fd2f3c9dc0f43c1a8'
CONTROLLER = {'run.py': '3ad6ed9c801b561b50219fcee55b0dacec2785ca0153bffa057181f8335b6431',
    'test_run.py': '8a2edd440ce8c2280edba2376e7bd49115a1dab30acc08deb0fd2732b3886463',
    'README.md': '3a755b8f11a968fda8bb4a11ec7f850635b0c812e97fa0985132e463e3066d39'}
PHASES = ('rustfmt', 'rustfmt-check', 'metadata', 'lower-build-tests', 'lower-list',
          'lower-ignored-list', 'lower-tests')
UNATTEMPTED = ('lower-indexed-tests', 'finalizer-build-tests', 'finalizer-list',
               'finalizer-ignored-list', 'finalizer-tests', 'actual-inert-join')
SNAPSHOTS = {'sources-unformatted.json', 'configurations-before.json', 'sources-before.json',
    'dependencies-before.json', 'original-after.json', 'sources-after.json', 'inputs-after.json',
    'prior-dependencies-after.json', 'configurations-after.json', 'dependencies-after.json'}
OWNER_OMITTED = {'after.json', 'old-targets-before.json'}
FAILED_TEST = ('production_semantic_kir_v1::wave_task_entry_parameter_tests::access_roots::'
    'retained_fields::retained_nested_enum_referent_scalar_move_invalidates_saved_references')


def common(path):
    path = path.absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical data helper')
    raw = path.read_bytes()
    if len(raw) > 1 << 20 or hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact data helper')
    module = types.ModuleType('indexed_failure_publication_common')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return module


def terminal(X, value, owner):
    X.require(value['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-result-v1'
        and owner['schema'] == 'ferric-p228-kir-indexed-formal-join-owned-result-v1'
        and value['passed'] is False and owner['passed'] is False
        and value['error'] == 'AssertionError: lower-tests'
        and owner['error'] == 'RuntimeError: natural successful owned-tree exit required'
        and value['postcheck_errors'] == owner['postcheck_errors'] == []
        and owner['completion'] is None and value['source_unchanged'] is True,
        'actual failed qualification with clean postchecks, not expected-test success')
    X.require(owner['package'] == value['package'] and owner['proposal'] == value['proposal']
        and value['tests'] == {} and value['actual_join'] is None
        and set(value['phases']) == set(PHASES), 'same generation and stopped seven-phase boundary')
    for key in ('staged_join_qualification_completed', 'actual_capture_join_passed', 'limits_changed',
                'fresh_compiler_built', 'fresh_hsaco_emitted', 'full_compiler_cohort_requalified',
                'gpu_execution', 'numerical_acceptance', 'production_authority', 'performance_claim',
                'prior_compiled_library_inventory_available'):
        X.require(value[key] is False, 'no qualification or new image claim: ' + key)
    X.require(all(owner[key] is False for key in ('actual_capture_join_passed', 'fresh_hsaco_emitted',
        'gpu_execution', 'production_authority')), 'owner remains failed and CPU-only')
    outcome = owner['owned']
    X.require(type(outcome['exit_code']) is int and outcome['exit_code'] == 1
        and outcome['reason'] is None and outcome['cleanup_signalled'] is False
        and outcome['owned_groups_absent'] is True and outcome['owned_processes_reaped'] is True,
        'natural owned failure, all processes reaped')


def observed_tests(X, value, listing, ignored, log):
    names = sorted(re.findall(r'^([^\r\n]+): test$', listing, re.M))
    rows = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|FAILED)$', log, re.M)
    X.require(len(names) == len(set(names)) == 785 and ignored == ''
        and sorted(name for name, _ in rows) == names
        and [name for name, status in rows if status == 'FAILED'] == [FAILED_TEST]
        and re.findall(r'test result: FAILED\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log)
            == [('784', '1', '0', '0', '0')], '785 named outcomes, actual 784/1/0 failure')
    inventory = value['lower_inventory']
    added = sorted(value['added_tests'])
    X.require(inventory['names'] == names and inventory['ignored_names'] == []
        and inventory['added_names'] == added and len(added) == len(set(added)) == 20
        and all((name, 'ok') in rows for name in added), 'all twenty additions passed inside failed full suite')
    X.require(re.findall(r'^  left: (.+)$', log, re.M) == ['(8, Some(7), 5)']
        and re.findall(r'^ right: (.+)$', log, re.M) == ['(7, Some(4), 5)']
        and 'retained_field_scalar_moves_v1_tests.rs:173:13:' in log, 'actual location assertion, no inferred cause')
    return dict(passed=784, failed=1, ignored=0, names=names, failed_names=[FAILED_TEST],
        added_passed_names=added, observed_location=[8, 7, 5], expected_location=[7, 4, 5],
        baseline_cause_established=False, separate_indexed_subset_executed=False,
        second_alias_only_subcase_completed=False)


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
    X.require(before == after and len(before) == 5785, 'unchanged complete candidate source map')
    original = relative(get('original-after.json'), CPU / 'source/fe2o3')
    unformatted = relative(get('sources-unformatted.json'), CASE / 'source/fe2o3')
    formatted = relative(before, CASE / 'source/fe2o3')
    expected = dict(original)
    changed = {row['path'] for row in proposal['files']}
    X.require(len(original) == 5783, 'actual original source census')
    for row in proposal['files']:
        X.require(original.get(row['path']) == row['before'], 'source proposal exact preimage')
        expected[row['path']] = row['after']
    X.require(unformatted == expected and set(formatted) == set(expected)
        and all(formatted[name] == pin for name, pin in expected.items() if name not in changed),
        'only four formatted overlay bodies and two additions')
    for name, pin in value['formatted_sources'].items():
        X.require(formatted[name] == {key: pin[key] for key in ('bytes', 'sha256')}, 'formatted compiled pin')
    for first, second in (('configurations-before.json', 'configurations-after.json'),
                          ('dependencies-before.json', 'dependencies-after.json')):
        X.require(get(first) == get(second), 'unchanged ' + first)
    X.require(all(value['raw']['original-after.json'][key] == value['source_generation'][key]
                  for key in ('bytes', 'sha256')), 'original source map still matches prior RPO generation')
    return dict(original_files=5783, candidate_files=5785, overlay_files=4, additions=2,
        source_transition_checked_remotely=True, source_and_dependency_postchecks_equal=True)


def roster(X, reader, inner_sha, owner_sha):
    X.require(X.digest(inner_sha) == INNER[1] and X.digest(owner_sha) == OUTER[1], 'actual failed terminal digests')
    records, omitted = {}, {}
    def add(path, expected=None, retained=True, cap=32 << 20):
        path = Path(path)
        name = str(X.relative(str(path.relative_to(E))))
        if retained:
            X.require('/target/' not in '/' + name + '/' and path.suffix not in ('.so', '.rlib', '.rmeta'), 'no product body')
        raw = reader(path, expected, cap=cap)
        pin = dict(path=str(path), **X.extent(raw))
        target = records if retained else omitted
        X.require(name not in target or target[name] == pin, 'unique original file identity')
        target[name] = pin
        return raw
    value = X.parse(add(CASE / 'failed.json', dict(zip(('bytes', 'sha256'), INNER))))
    owner = X.parse(add(OWNER / 'failed.json', dict(zip(('bytes', 'sha256'), OUTER))))
    terminal(X, value, owner)
    raw_names = {phase + '-' + suffix for phase in PHASES for suffix in X.SUFFIXES}
    X.require(set(value['raw']) == raw_names | SNAPSHOTS, '35 raw leaf bodies plus ten snapshots')
    for name, pin in value['raw'].items():
        X.require(X.pin(pin)['path'] == str(CASE / name), 'original raw namespace')
        add(pin['path'], pin, name not in SNAPSHOTS)
    for phase, result in value['phases'].items():
        X.require(result['exit_code'] == (101 if phase == 'lower-tests' else 0)
            and result['reason'] is None and result['group_absent'] is True
            and X.parse(reader(CASE / (phase + '-result.json'), value['raw'][phase + '-result.json'])) == result
            and all(value['raw'][phase + '-' + key]['sha256'] == result[key + '_sha256']
                    for key in ('stdout', 'stderr')), 'natural actual phase/result/stream join')
    for name in sorted((X.OWNER_FILES - {'complete.json'}) | {'failed.json'}):
        add(OWNER / name, retained=name not in OWNER_OMITTED)
    X.require(X.parse(reader(OWNER / 'owned-result.json')) == owner['owned'], 'original owned result')
    X.require(value['package']['path'] == str(E / 'p228-kir-indexed-formal-join-cpu-v1/manifest.json')
        and value['package']['sha256'] == PACKAGE_SHA, 'frozen CPU controller')
    package = X.parse(add(value['package']['path'], value['package']))
    X.require(package['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-package-v1'
        and len(package['files']) == 3 and {row['path']: row['sha256'] for row in package['files']} == CONTROLLER,
        'three controller sources')
    for row in package['files']:
        add(Path(value['package']['path']).parent / X.relative(row['path']), row)
    X.require(value['proposal']['path'] == str(E / 'p228-kir-indexed-formal-join-v1/source-manifest.json')
        and value['proposal']['sha256'] == SOURCE_SHA, 'reviewed source proposal')
    proposal = X.parse(add(value['proposal']['path'], value['proposal']))
    X.require(proposal['schema'] == 'ferric-p228-kir-indexed-formal-join-source-v1'
        and len(proposal['files']) == 4 and sum(row['before'] is None for row in proposal['files']) == 2
        and set(value['formatted_sources']) == {row['path'] for row in proposal['files']}
        and value['added_tests'] == proposal['added_tests'], 'four sources and actual added-test contract')
    for row in proposal['files']:
        X.require(row['source'] == 'draft/' + row['path'], 'proposal source mapping')
        add(Path(value['proposal']['path']).parent / X.relative(row['source']), row['after'])
        pin = value['formatted_sources'][row['path']]
        X.require(pin['path'] == str(CASE / 'source/fe2o3' / X.relative(row['path'])), 'formatted source mapping')
        add(pin['path'], pin)
    add(Path(value['proposal']['path']).parent / 'README.md', dict(bytes=6033,
        sha256='54ec0e8f146735085f3cac58510446212633291634f73c528898559eb9156d90'))
    tests = observed_tests(X, value, *(reader(CASE / (name + '-stdout'), value['raw'][name + '-stdout']).decode()
        for name in ('lower-list', 'lower-ignored-list', 'lower-tests')))
    sources = source_maps(X, value, proposal, reader)
    X.require(set(value['artifacts']) == {'lower-tests'}, 'only lower-library ELF was built')
    pin = X.pin(value['artifacts']['lower-tests'])
    X.require(Path(pin['path']).is_relative_to(CASE / 'target/debug/deps'), 'actual lower-library artifact path')
    add(pin['path'], pin, retained=False, cap=128 << 20)
    X.require(len(records) == 56 and len(omitted) == 13
        and sum(pin['bytes'] for pin in records.values()) <= 32 << 20, 'closed small failure courier')
    return records, omitted, tests, sources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('inner_sha256'); parser.add_argument('owner_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper)
    out = args.archive.absolute()
    X.require(out.parent == E and re.fullmatch('kir-indexed-formal-join-failure-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
        and not os.path.lexists(out), 'fresh bounded archive')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 30 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    records, omitted, tests, sources = roster(X, X.read, args.inner_sha256, args.owner_sha256)
    manifest = dict(schema='ferric-p228-kir-indexed-formal-join-failure-export-v1', original_root=str(E),
        inner_sha256=args.inner_sha256, owner_sha256=args.owner_sha256, files=records,
        remotely_rehashed_omitted_pins=omitted, observed_tests=tests, remote_source_checks=sources,
        snapshot_bodies_exported=False, owner_inventory_bodies_exported=False,
        artifact_bodies_exported=False, full_source_tree_exported=False,
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
        X.read(Path(pin['path']), pin, cap=(128 if Path(pin['path']).is_relative_to(CASE / 'target') else 32) << 20)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out))), files=len(records),
        remotely_rehashed_not_exported=len(omitted), qualification_passed=False)))


if __name__ == '__main__':
    main()
