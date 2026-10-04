"""Data-only courier for a completed unchanged-RPO negative baseline control."""
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
CASE = E / 'kir-indexed-formal-join-baseline-cpu-v228-v1'
OWNER = E / 'kir-indexed-formal-join-baseline-cpu-owner-v228-v1'
CPU = E / 'rpo-compiler-cpu-v228-v2'
PACKAGE = E / 'p228-kir-indexed-formal-join-baseline-cpu-v1'
INNER = (244380, 'a95ee1e3fb32cd50745a0bba5274e5688b72e7ee3f26de25ddbe0cd936e667ee')
OUTER = (19845, '87ae6c49d7ec72a35d6e13339cc501323d10d5e78ce212c77b336ff8a675012c')
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
PACKAGE_SHA = 'a6ecb1fe4ef7fda4d397f656590c73b188f0441c32322522d03a6c85441074d8'
CONTROLLER = {'run.py': '4311311fc1aec260cf189d7d20e05ede46360c266e8842eca23b5e26cf060344',
    'test_run.py': 'e3ffa3ba7b067a4a552951a73d5d66ad7af30e82d48f4b3f3595108a84e2cabc',
    'README.md': '4f0215ed72580387ba914397c4624f8cc2f9c8a4828641ec20821b4b6ea33604'}
PHASES = ('metadata', 'lower-build-tests', 'lower-list', 'lower-ignored-list',
          'lower-focused-test', 'lower-tests')
SNAPSHOTS = {'configurations-before.json', 'sources-before.json', 'dependencies-before.json',
    'original-after.json', 'sources-after.json', 'inputs-after.json',
    'prior-dependencies-after.json', 'configurations-after.json', 'dependencies-after.json'}
OWNER_OMITTED = {'after.json', 'old-targets-before.json'}
FAILURE = ('production_semantic_kir_v1::wave_task_entry_parameter_tests::access_roots::'
    'retained_fields::retained_nested_enum_referent_scalar_move_invalidates_saved_references')
FAILURE_PIN = dict(path=str(E / 'kir-indexed-formal-join-cpu-v228-v1/failed.json'),
    bytes=142699, sha256='88d0fa37ebdbe9bda0501af8693f664c8dbfb5b909b586f2b930b16d37fb7b89')
FAILURE_OWNER_PIN = dict(path=str(E / 'kir-indexed-formal-join-cpu-owner-v228-v1/failed.json'),
    bytes=20777, sha256='b845ba6e230625237605601ced70efba5bc8427346076256205b6014587fc183')
CPU_PIN = dict(path=str(CPU / 'complete.json'), bytes=375781,
    sha256='56fc51fc326980e00156d550d0a7052f44bb481ded7b9948c06217653fb246c1')
SOURCE_PIN = dict(path=str(CPU / 'sources-before.json'), bytes=3607137,
    sha256='77fbdd63c370665ea80bf70e8dd8f4a45533bcde8e5ba6ffd670ab497ebc6665')


def common(path):
    path = path.absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical data helper')
    raw = path.read_bytes()
    if len(raw) > 1 << 20 or hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact data helper')
    module = types.ModuleType('indexed_baseline_publication_common')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return module


def terminal(X, value, owner, completion):
    X.require(value['schema'] == 'ferric-p228-kir-indexed-baseline-cpu-result-v1'
        and owner['schema'] == 'ferric-p228-kir-indexed-baseline-owned-result-v1'
        and value['passed'] is owner['passed'] is True
        and value['observation_completed'] is owner['observation_completed'] is True
        and value['error'] is owner['error'] is None
        and value['postcheck_errors'] == owner['postcheck_errors'] == []
        and owner['completion'] == completion and value['source_unchanged'] is True,
        'completed observer and clean postchecks, not library qualification')
    X.require(value['preexisting_rpo_failure_observed'] is True and value['baseline_library_passed'] is False
        and set(value['phases']) == set(PHASES) and set(value['tests']) == {'lower-focused-test', 'lower-tests'}
        and owner['package'] == value['package']
        and owner['candidate_failure'] == value['candidate_failure'] == FAILURE_PIN
        and value['candidate_owner'] == FAILURE_OWNER_PIN and value['compiler_cpu'] == CPU_PIN
        and value['source_generation'] == SOURCE_PIN, 'exact negative-control lineage; stop on other outcomes')
    for key in ('candidate_qualified', 'library_qualified', 'source_overlay_applied', 'source_formatted',
                'fresh_compiler_built', 'fresh_hsaco_emitted', 'actual_capture_join_passed',
                'gpu_execution', 'numerical_acceptance', 'production_authority', 'performance_claim'):
        X.require(value[key] is False, 'no qualification or altered baseline source: ' + key)
    X.require(all(owner[key] is False for key in ('candidate_qualified', 'library_qualified',
        'gpu_execution', 'production_authority')), 'CPU-only observer owner')
    result = owner['owned']
    X.require(type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
        and result['cleanup_signalled'] is False and result['owned_groups_absent'] is True
        and result['owned_processes_reaped'] is True, 'natural successful observer process')


def observed_tests(X, value, listing, ignored, focused, full):
    names = sorted(re.findall(r'^([^\r\n]+): test$', listing, re.M))
    X.require(len(names) == len(set(names)) == 765 and ignored == '' and FAILURE in names,
        'exact unchanged-source library inventory without hidden ignores')
    prior = value['candidate_observation']
    X.require(prior['passed'] == 784 and prior['failed'] == 1 and prior['ignored'] == 0
        and prior['failed_names'] == [FAILURE] and prior['exit_code'] == 101
        and prior['test_suite_passed'] is False and prior['filtered_out'] == 0
        and len(prior['names']) == len(set(prior['names'])) == 785 and set(names) < set(prior['names']),
        'recorded prior failed inventory includes the unchanged-source cohort')
    summaries = {}
    for phase, log, expected, filtered in (('lower-focused-test', focused, [FAILURE], 764),
                                          ('lower-tests', full, names, 0)):
        rows = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|FAILED)$', log, re.M)
        X.require(sorted(name for name, _ in rows) == expected
            and [name for name, status in rows if status == 'FAILED'] == [FAILURE]
            and re.findall(r'test result: FAILED\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log)
                == [(str(len(expected) - 1), '1', '0', '0', str(filtered))], 'exact named negative result')
        X.require(re.findall(r'left: \((\d+), Some\((\d+)\), (\d+)\)\s+right: \((\d+), Some\((\d+)\), (\d+)\)', log)
            == [('8', '7', '5', '7', '4', '5')], 'same exact location assertion, not inferred equivalence')
        observed = dict(passed=len(expected) - 1, failed=1, ignored=0, names=expected,
            failed_names=[FAILURE], filtered_out=filtered, exit_code=101, test_suite_passed=False)
        X.require(value['tests'][phase] == observed, 'receipt and actual named transcript agree')
        summaries[phase] = {key: item for key, item in observed.items() if key != 'names'}
    return dict(suites=summaries, full_names=names, observed_location=[8, 7, 5],
        expected_location=[7, 4, 5], reproduced_without_indexed_overlay=True,
        precise_cause_proven=False, error_order_soundness_proven=False,
        second_alias_only_subcase_completed=False)


def source_maps(X, value, reader):
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
    original = get('original-after.json')
    X.require(before == after and len(before) == len(original) == 5783
        and relative(before, CASE / 'source/fe2o3') == relative(original, CPU / 'source/fe2o3'),
        'byte-exact unchanged original RPO source, no overlay or formatter')
    X.require(all(value['raw']['original-after.json'][key] == SOURCE_PIN[key] for key in ('bytes', 'sha256')),
        'original source generation unchanged')
    for first, second in (('configurations-before.json', 'configurations-after.json'),
                          ('dependencies-before.json', 'dependencies-after.json')):
        X.require(get(first) == get(second), 'unchanged ' + first)
    return dict(original_files=5783, baseline_files=5783, overlay_files=0, formatted_files=0,
        source_transition_checked_remotely=True, source_and_dependency_postchecks_equal=True)


def roster(X, reader, inner_sha, owner_sha):
    X.require(X.digest(inner_sha) == INNER[1] and X.digest(owner_sha) == OUTER[1],
        'actual retained baseline terminal digests')
    records, omitted = {}, {}
    def add(path, expected=None, retained=True, cap=32 << 20):
        path = Path(path)
        name = str(X.relative(str(path.relative_to(E))))
        if retained:
            X.require('/target/' not in '/' + name + '/' and path.suffix not in ('.so', '.rlib', '.rmeta'), 'no product body')
        raw = reader(path, expected, cap=cap)
        pin = dict(path=str(path), **X.extent(raw))
        target = records if retained else omitted
        X.require(name not in target or target[name] == pin, 'unique original body identity')
        target[name] = pin
        return raw
    value = X.parse(add(CASE / 'complete.json', dict(zip(('bytes', 'sha256'), INNER))))
    owner = X.parse(add(OWNER / 'complete.json', dict(zip(('bytes', 'sha256'), OUTER))))
    complete = records[str((CASE / 'complete.json').relative_to(E))]
    X.require(complete['sha256'] == inner_sha
        and records[str((OWNER / 'complete.json').relative_to(E))]['sha256'] == owner_sha,
        'root-authenticated actual terminal receipts')
    terminal(X, value, owner, complete)
    leaves = {phase + '-' + suffix for phase in PHASES for suffix in X.SUFFIXES}
    X.require(set(value['raw']) == leaves | SNAPSHOTS, 'thirty leaf bodies plus nine snapshot pins')
    for name, pin in value['raw'].items():
        X.require(X.pin(pin)['path'] == str(CASE / name), 'raw original namespace')
        add(pin['path'], pin, name not in SNAPSHOTS)
    for name in sorted(X.OWNER_FILES):
        add(OWNER / name, retained=name not in OWNER_OMITTED)
    X.require(X.parse(reader(OWNER / 'owned-result.json')) == owner['owned'], 'actual owned result body')
    for phase, result in value['phases'].items():
        X.require(type(result['exit_code']) is int
            and result['exit_code'] == (101 if phase in ('lower-focused-test', 'lower-tests') else 0)
            and result['reason'] is None and result['group_absent'] is True
            and X.parse(reader(CASE / (phase + '-result.json'), value['raw'][phase + '-result.json'])) == result
            and all(value['raw'][phase + '-' + key]['sha256'] == result[key + '_sha256'] for key in ('stdout', 'stderr')),
            'natural leaf results and streams, negative tests not qualification')
    X.require(value['package']['path'] == str(PACKAGE / 'manifest.json')
        and value['package']['sha256'] == PACKAGE_SHA, 'frozen baseline controller package')
    package = X.parse(add(value['package']['path'], value['package']))
    X.require(package['schema'] == 'ferric-p228-kir-indexed-baseline-cpu-package-v1'
        and len(package['files']) == 3 and {row['path']: row['sha256'] for row in package['files']} == CONTROLLER,
        'three original controller sources')
    for row in package['files']:
        add(PACKAGE / X.relative(row['path']), row)
    tests = observed_tests(X, value, *(reader(CASE / (name + '-stdout'), value['raw'][name + '-stdout']).decode()
        for name in ('lower-list', 'lower-ignored-list', 'lower-focused-test', 'lower-tests')))
    sources = source_maps(X, value, reader)
    X.require(set(value['artifacts']) == {'lower-tests'}, 'only original lower-library test artifact')
    pin = X.pin(value['artifacts']['lower-tests'])
    X.require(Path(pin['path']).is_relative_to(CASE / 'target/debug/deps'), 'test ELF original target')
    add(pin['path'], pin, retained=False, cap=128 << 20)
    X.require(len(records) == 41 and len(omitted) == 12
        and sum(pin['bytes'] for pin in records.values()) <= 16 << 20, 'closed small baseline courier')
    return records, omitted, tests, sources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('inner_sha256'); parser.add_argument('owner_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args(); X = common(args.common_helper)
    out = args.archive.absolute()
    X.require(out.parent == E and re.fullmatch('kir-indexed-baseline-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
        and not os.path.lexists(out), 'fresh bounded archive')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 20 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    records, omitted, tests, sources = roster(X, X.read, args.inner_sha256, args.owner_sha256)
    manifest = dict(schema='ferric-p228-kir-indexed-baseline-export-v1', original_root=str(E),
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
        remotely_rehashed_not_exported=len(omitted), observer_completed=True, library_qualified=False)))


if __name__ == '__main__':
    main()
