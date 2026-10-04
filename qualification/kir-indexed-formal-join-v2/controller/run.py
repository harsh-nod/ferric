"""Fresh indexed-join library/finalizer tests and the retained actual inert join."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
CPU = E / 'rpo-compiler-cpu-v228-v2'
ORIGINAL = CPU / 'source/fe2o3'
TOOLS = E / 'rpo-finalizer-tools-v228-v2'
ROW = E / 'row-rope-materialized-rpo-checked-probe-v228-v1'
OUT = E / 'kir-indexed-formal-join-cpu-v228-v2'
OWNER = E / 'kir-indexed-formal-join-cpu-owner-v228-v2'
COPY, TARGET = OUT / 'source/fe2o3', OUT / 'target'
PACKAGE = E / 'p228-kir-indexed-formal-join-cpu-v2'
PATCH = E / 'p228-kir-indexed-formal-join-v2'
BASE = E / 'p228-partial-move-rpo-finalizer-tools-v2/run.py'
BASE_SHA = 'cd5fb1f8254e3eb3a2c8df4881432f3c73fe1bb18183cee73e7a0f3c8ad30b25'
DIAGNOSTIC = E / 'kir-join-work-diagnostic-cpu-v228-v1'
DIAGNOSTIC_SHA = '71ea46649382eed953ec5075524d831169ea92031b6ccaf1dbbeeb99937d2faf'
DIAGNOSTIC_OWNER = E / 'kir-join-work-diagnostic-cpu-owner-v228-v1'
DIAGNOSTIC_OWNER_SHA = '47477678c6db2dc800d80906453e23e5fe0409f378fcd0512f5be0630212e349'
SOURCE_SHA = 'd9d235a2f0bb2ae185c41f38d8406124c8233d3259e53751ccf47a17755b5dc2'
BASELINE_SHA = 'a95ee1e3fb32cd50745a0bba5274e5688b72e7ee3f26de25ddbe0cd936e667ee'
BASELINE_OWNER_SHA = '87ae6c49d7ec72a35d6e13339cc501323d10d5e78ce212c77b336ff8a675012c'
BASELINE = E / 'kir-indexed-formal-join-baseline-cpu-v228-v1'
BASELINE_OWNER = E / 'kir-indexed-formal-join-baseline-cpu-owner-v228-v1'
BASELINE_READER = E / 'p228-kir-indexed-formal-join-baseline-cpu-v1/run.py'
BASELINE_READER_SHA = '4311311fc1aec260cf189d7d20e05ede46360c266e8842eca23b5e26cf060344'
BASELINE_PACKAGE_SHA = 'a6ecb1fe4ef7fda4d397f656590c73b188f0441c32322522d03a6c85441074d8'
PREVIOUS = E / 'kir-indexed-formal-join-cpu-v228-v1'
PREVIOUS_PIN = dict(path=str(PREVIOUS / 'failed.json'), bytes=142699,
    sha256='88d0fa37ebdbe9bda0501af8693f664c8dbfb5b909b586f2b930b16d37fb7b89')
PREVIOUS_SOURCE = dict(path=str(E / 'p228-kir-indexed-formal-join-v1/source-manifest.json'),
    bytes=5277, sha256='6501750a4ac345cbd47e543954580775eab5b8df406bc302117906b5c1265560')
FAILURE = 'production_semantic_kir_v1::wave_task_entry_parameter_tests::access_roots::retained_fields::retained_nested_enum_referent_scalar_move_invalidates_saved_references'
FIXED = {
    str(CPU / 'complete.json'): (375781, '56fc51fc326980e00156d550d0a7052f44bb481ded7b9948c06217653fb246c1'),
    str(E / 'rpo-compiler-cpu-owner-v228-v2/complete.json'): (57128, 'afca99d8799f910ce607873c320b8f244be66f9ffdb3df4dce4545eb1552ea66'),
    str(TOOLS / 'complete.json'): (44858, '39eab92ede34991b08c168e827c7111c13533627cbf2efd3d248c6742a92660b'),
    str(E / 'rpo-finalizer-tools-owner-v228-v2/complete.json'): (11103, 'e919fb520672be2a909fd0ec8301bc9923210680f818e411b7db459ffc3059dd'),
    str(ROW / 'failed.json'): (21555, '1fbbbd9a8889e1b33b724b3f8911c4ecc1074fffe8fc29377f6840686375ba1f'),
    str(E / 'rope-materialized-rpo-checked-probe-owner-v228-v1/failed.json'): (28117, '8fb2f02cde719c08fd41535d59d39980d8c6328f27cf952770c481f5b48edbbf'),
    str(ROW / 'actual-inert-join-command.json'): (2791, 'fc96a2d19b1fcd008d1e04b6e3e3fe59d73c87a2aebccca08a7c3457de082223'),
    str(ROW / 'actual-inert-join-result.json'): (308, '1110d8ee0c91463eec4f59dfeaa34bc15e8bdbe17e1136b4f888da260a730cc1'),
    str(ROW / 'actual-inert-join-stdout'): (990, '9e49bc58a17e52b611688438ed8f1eb56618cdc3839f689f733c28f216f7b840'),
}
SOURCE_DIR = 'crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1/'
NEW_FILES = {SOURCE_DIR + name for name in ('wave_formal_evidence_index_v1.rs', 'wave_formal_evidence_index_v1_tests.rs')}
FILES = NEW_FILES | {SOURCE_DIR + name for name in ('wave_formal_evidence_join_v1.rs',
    'wave_formal_evidence_v1_tests.rs', 'retained_nested_enum_transport_v1_tests.rs')}
TEST_PREFIX = 'production_semantic_kir_v1::wave_formal_evidence_v1::tests::indexed_formal_join_'
PHASES = {'rustfmt', 'rustfmt-check', 'metadata', 'finalizer-build-tests', 'finalizer-list',
          'finalizer-ignored-list', 'finalizer-tests', 'actual-inert-join'} | {
          'lower-' + suffix for suffix in ('build-tests', 'list', 'ignored-list', 'tests', 'indexed-tests')}
SELECTOR = 'linux::wave_qkv_attention_output_tiles_v6::tests::wave_emission_actual_retained_v6_passes_full_inert_join'
HANDOFF = dict(path=str(ROW / 'prefix-tiles.handoff-v3'), bytes=4078537,
    sha256='ad4b31ee88efa54702dc8dff13e1e3c331c296734f7cdcfa52e8249ed9fa37fc')
DIAGNOSTIC_ENV = 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1'
WORK_ACTUAL, WORK_LIMIT = 1084825160, 1073741824
STORAGE_LIMIT = 128 << 20


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def load(path, digest, name):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical helper')
    raw = path.read_bytes()
    require(len(raw) < 1 << 20 and hashlib.sha256(raw).hexdigest() == digest, 'authenticated helper')
    result = types.ModuleType(name)
    result.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), result.__dict__)
    return result


def transition(before, after, proposal, formatted=False):
    require(len(proposal['files']) == 5 and {row['path'] for row in proposal['files']} == FILES,
            'only five indexed-join and expectation bodies')
    expected = dict(before)
    for row in proposal['files']:
        require(set(row) == {'path', 'source', 'before', 'after'} and row['source'] == 'draft/' + row['path']
                and (row['before'] is None and row['path'] in NEW_FILES and row['path'] not in before
                     or row['before'] is not None and row['path'] not in NEW_FILES and before[row['path']] == row['before']),
                'qualified source preimage or declared absent new body')
        expected[row['path']] = row['after']
    require(set(after) == set(expected) and all(after[name] == record for name, record in expected.items()
            if not formatted or name not in FILES), 'no source change outside exact overlay/format scope')


def relocate(value):
    if isinstance(value, str):
        return value.replace(str(CPU), str(OUT))
    if isinstance(value, list):
        return [relocate(item) for item in value]
    if isinstance(value, dict):
        return {key: relocate(item) for key, item in value.items()}
    return value


def dependency_transition(previous, current, sources):
    old_local = {str(Path(name).relative_to(ORIGINAL)) for name in previous if Path(name).is_relative_to(ORIGINAL)}
    local = {name: row for name, row in current.items() if Path(name).is_relative_to(COPY)}
    require(old_local and not old_local & NEW_FILES
            and {str(Path(name).relative_to(COPY)) for name in local} == old_local | NEW_FILES,
            'exact prior local dependency roster plus two declared new source files')
    require(all(row == sources[name] for name, row in local.items()), 'local dependencies match copied source pins/stamps')
    require({name: row for name, row in current.items() if name not in local}
            == {name: row for name, row in previous.items() if not Path(name).is_relative_to(ORIGINAL)},
            'unchanged external dependency/provider/toolchain bodies')


def prerequisite_pins_ready(source_sha, baseline_sha, owner_sha):
    require(all(type(value) is str and re.fullmatch('[0-9a-f]{64}', value)
                for value in (source_sha, baseline_sha, owner_sha)), 'V2 remains unexecutable until actual baseline/source pins are bound')


def proposal_contract(proposal, cpu_pin, source_pin, baseline_pin):
    require(proposal['schema'] == 'ferric-p228-kir-indexed-formal-join-source-v2'
            and proposal['base_cpu'] == cpu_pin and proposal['base_source_snapshot'] == source_pin,
            'exact original RPO source generation')
    require(proposal['tentative'] is True and proposal['diagnostic_site_measured'] is True
            and proposal['compiled'] is False and proposal['tests_executed'] is False
            and all(proposal[key] is False for key in ('changes_live_join', 'changes_canonical_bytes', 'changes_authority'))
            and proposal['work_limit'] == WORK_LIMIT and proposal['storage_limit'] == STORAGE_LIMIT,
            'frozen source proposal provenance and unchanged limits/authority')
    require(proposal['frozen'] is True and proposal['baseline_control_status'] == 'measured_original_RPO_control'
            and proposal['baseline_control'] == baseline_pin and proposal['baseline_failure_reproduced'] is True
            and proposal['previous_source_proposal'] == PREVIOUS_SOURCE and proposal['previous_indexed_attempt'] == PREVIOUS_PIN
            and proposal['changed_existing_test'] == dict(name=FAILURE, old_expected_location=[7, 4, 5],
                proposed_expected_location=[8, 7, 5], alias_only_proof_refusal_unchanged=True),
            'expectation correction requires the actual original-source control')
    names = proposal['added_tests']
    require(proposal['added_test_count'] == 20 and len(names) == len(set(names)) == 20
            and all(type(name) is str and name.startswith(TEST_PREFIX) for name in names),
            'exact twenty declared lower-library tests')


def lower_inventory(names, ignored, proposal, baseline_names):
    added = set(proposal['added_tests'])
    require(names == sorted(set(names)) and ignored == sorted(set(ignored)) and set(ignored) <= set(names)
            and {name for name in names if name.startswith(TEST_PREFIX)} == added
            and len(baseline_names) == len(set(baseline_names)) == 765 and FAILURE in baseline_names
            and not added & set(baseline_names) and names == sorted(set(baseline_names) | added)
            and ignored == [] and len(names) == 785, 'exact measured original765 plus twenty nonignored additions')
    return dict(names=names, ignored_names=ignored, added_names=sorted(added),
                prior_compiled_library_inventory_available=True)


def baseline_contract(value, owner, pin, cpu_pin, source_pin):
    require(value['schema'] == 'ferric-p228-kir-indexed-baseline-cpu-result-v1'
            and owner['schema'] == 'ferric-p228-kir-indexed-baseline-owned-result-v1'
            and owner['completion'] == pin and owner['package'] == value['package']
            and value['package']['path'] == str(BASELINE_READER.parent / 'manifest.json')
            and value['package']['sha256'] == BASELINE_PACKAGE_SHA
            and value['compiler_cpu'] == cpu_pin and value['source_generation'] == source_pin
            and value['candidate_failure'] == owner['candidate_failure'] == PREVIOUS_PIN,
            'actual baseline/source/failed-candidate generation joins')
    for row in (value, owner):
        require(row['passed'] is True and row['observation_completed'] is True and row['error'] is None
                and row['postcheck_errors'] == [] and row['candidate_qualified'] is False
                and row['library_qualified'] is False, 'completed negative observer is not library qualification')
    require(value['preexisting_rpo_failure_observed'] is True and value['baseline_library_passed'] is False
            and value['source_unchanged'] is True and value['source_overlay_applied'] is False
            and value['source_formatted'] is False, 'actual unchanged-source failure independently reproduced')
    focused, full = (value['tests'][key] for key in ('lower-focused-test', 'lower-tests'))
    require(set(value['tests']) == {'lower-focused-test', 'lower-tests'}
            and focused['names'] == focused['failed_names'] == [FAILURE] and focused['passed'] == 0
            and focused['failed'] == 1 and focused['filtered_out'] == 764
            and full['failed_names'] == [FAILURE] and full['passed'] == 764 and full['failed'] == 1
            and len(full['names']) == len(set(full['names'])) == 765 and full['filtered_out'] == 0
            and all(row['exit_code'] == 101 and row['ignored'] == 0 and row['test_suite_passed'] is False
                    for row in (focused, full)), 'exact measured focus/full765 negative outcomes')
    return full['names']


def actual_join(stdout, stderr, result):
    require(type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
            and result['group_absent'] is True, 'natural successful actual retained inert join')
    require(re.findall(r'^test (\S+) \.\.\. (\S+)$', stdout, re.M) == [(SELECTOR, 'ok')]
            and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', stdout)
                == [('1', '0', '0', '0', '204')], 'exact one actual ignored finalizer test now passes')
    require('kir-join-work-v1' not in stdout + stderr and 'CanonicalKernelIrWorkLimitV1' not in stdout + stderr,
            'no diagnostic instrumentation or old refusal accepted as success')
    return dict(test=SELECTOR, passed=1, ignored=0, filtered_out=204, exit_code=0,
                actual_capture_join_passed=True, fresh_hsaco_emitted=False)


def selected_environment(previous, env):
    prefix = 'FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_'
    inputs = {prefix + 'PATH': HANDOFF['path'], prefix + 'BYTES': str(HANDOFF['bytes']), prefix + 'SHA256': HANDOFF['sha256']}
    require(previous['argv'][1:] == ['--exact', SELECTOR, '--ignored', '--show-output', '--test-threads=1']
            and all(previous['env'][key] == value for key, value in inputs.items()), 'exact previous ignored test and handoff')
    expected = dict(previous['env'])
    for key in ('CARGO_TARGET_DIR', 'TMPDIR', 'LD_LIBRARY_PATH'):
        expected[key] = env[key]
    require(expected == dict(env, **inputs) and DIAGNOSTIC_ENV not in previous['env'] and DIAGNOSTIC_ENV not in env,
            'only fresh target/loader/temp change; no diagnostic instrumentation')
    return dict(env, **inputs)


def context(package_sha, source_sha):
    prerequisite_pins_ready(SOURCE_SHA, BASELINE_SHA, BASELINE_OWNER_SHA)
    require(source_sha == SOURCE_SHA, 'root-bound final V2 source manifest')
    v = load(BASE, BASE_SHA, 'qualified_finalizer_tools')
    base = v.load_base()
    modules = base.load_helpers()
    p, outer, driver = (modules[key] for key in ('probe', 'outer', 'driver'))
    pins = driver.Pins()
    fixed = {path: dict(path=path, bytes=size, sha256=sha) for path, (size, sha) in FIXED.items()}
    for record in fixed.values():
        outer.pin_exact(pins, record)
    cpu = p.parse(p.read(CPU / 'complete.json', retain=True)[2])
    owner = p.parse(p.read(E / 'rpo-compiler-cpu-owner-v228-v2/complete.json', retain=True)[2])
    tools = p.parse(p.read(TOOLS / 'complete.json', retain=True)[2])
    tool_owner = p.parse(p.read(E / 'rpo-finalizer-tools-owner-v228-v2/complete.json', retain=True)[2])
    v.CPU_PIN, v.OWNER_PIN = fixed[str(CPU / 'complete.json')], fixed[str(E / 'rpo-compiler-cpu-owner-v228-v2/complete.json')]
    v.validate_cpu(cpu, owner)
    outer.check_outcome(owner['owned']); outer.check_outcome(tool_owner['owned'])
    require(tools['schema'] == 'fe2o3-p228-rpo-finalizer-tools-result-v1'
            and tool_owner['schema'] == 'fe2o3-p228-rpo-finalizer-tools-owned-result-v1', 'actual finalizer schemas')
    for value in (tools, tool_owner):
        require(value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
                and value['compiler_cpu'] == v.CPU_PIN and value['compiler_owner'] == v.OWNER_PIN,
                'successful finalizer in exact qualified source generation')
    require(tool_owner['completion'] == fixed[str(TOOLS / 'complete.json')]
            and tools['compiler_artifacts'] == cpu['artifacts'] and tools['source_snapshot'] == cpu['raw']['sources-before.json'],
            'finalizer products preserve actual compiler source')
    for value in (cpu, tools):
        for record in [*value['raw'].values(), *value['artifacts'].values(), value['package']]:
            outer.pin_exact(pins, record)
    for name in ('librustc_codegen_fe2o3.so', 'librustc_codegen_fe2o3.rlib'):
        outer.pin_exact(pins, dict(cpu['artifacts'][name], path=str(CPU / 'target/debug/deps' / name)))
    read_old = lambda suffix: p.read(TOOLS / ('finalizer-' + suffix + '-stdout'), retain=True)[2].decode()
    names = base.inventory(read_old('list'))
    ignored = sorted(re.findall(r'^([^\r\n]+): test$', read_old('ignored-list'), re.M))
    v.finalizer_inventory(names, ignored, tools['tests'])
    require(base.test_results(read_old('tests'), names, ignored) == tools['tests'], 'actual full190/15 baseline transcript')
    lower = p.parse(p.read(ROW / 'failed.json', retain=True)[2])
    require(lower['passed'] is False and lower['postcheck_errors'] == [] and lower['artifacts']['prefix-tiles.handoff-v3'] == HANDOFF,
            'actual retained failed attempt and immutable handoff')
    for record in lower['artifacts'].values():
        outer.pin_exact(pins, record)
    diagnostic, diagnostic_pin = pins.json(DIAGNOSTIC / 'complete.json', DIAGNOSTIC_SHA)
    diagnostic_owner, diagnostic_owner_pin = pins.json(DIAGNOSTIC_OWNER / 'complete.json', DIAGNOSTIC_OWNER_SHA)
    require(diagnostic['schema'] == 'ferric-p228-kir-join-work-diagnostic-cpu-result-v1'
            and diagnostic_owner['schema'] == 'ferric-p228-kir-join-work-diagnostic-owned-result-v1'
            and diagnostic_owner['completion'] == diagnostic_pin
            and diagnostic['retained_handoff'] == HANDOFF and diagnostic['compiler_cpu'] == v.CPU_PIN,
            'actual separately measured diagnostic generation')
    for value in (diagnostic, diagnostic_owner):
        require(value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
                and value['actual_capture_join_passed'] is False, 'actual successful diagnosis is not join acceptance')
    outer.check_outcome(diagnostic_owner['owned'])
    observed = diagnostic['expected_refusal']
    require(observed['expected_refusal_observed'] is True
            and observed['error'] == dict(actual=WORK_ACTUAL, limit=WORK_LIMIT)
            and observed['last_observed_stage'] == 'layout-join-before'
            and observed['markers'][-1]['work'] + observed['markers'][-1]['charge'] == WORK_ACTUAL,
            'actual measured layout bulk refusal justifies this staged candidate')
    for record in diagnostic['artifacts'].values():
        outer.pin_exact(pins, record, DIAGNOSTIC / 'target')
    baseline, baseline_pin = pins.json(BASELINE / 'complete.json', BASELINE_SHA)
    baseline_owner, baseline_owner_pin = pins.json(BASELINE_OWNER / 'complete.json', BASELINE_OWNER_SHA)
    baseline_names = baseline_contract(baseline, baseline_owner, baseline_pin, v.CPU_PIN, cpu['raw']['sources-before.json'])
    outer.check_outcome(baseline_owner['owned'])
    outer.pin_exact(pins, PREVIOUS_PIN)
    outer.pin_exact(pins, PREVIOUS_SOURCE)
    previous = p.parse(p.read(PREVIOUS_PIN['path'], PREVIOUS_PIN, retain=True)[2])
    previous_source = p.parse(p.read(PREVIOUS_SOURCE['path'], PREVIOUS_SOURCE, retain=True)[2])
    require(previous['lower_inventory']['names'] == sorted(set(baseline_names) | set(previous_source['added_tests']))
            and previous['lower_inventory']['ignored_names'] == [] and previous['postcheck_errors'] == [],
            'new roster derives from both actual original and failed candidate inventories')
    for record in previous['artifacts'].values(): outer.pin_exact(pins, record, PREVIOUS / 'target')
    baseline_reader = load(BASELINE_READER, BASELINE_READER_SHA, 'qualified_original_rpo_control')
    pins.pin(BASELINE_READER, BASELINE_READER_SHA)
    outer.pin_exact(pins, baseline['package'])
    require(set(baseline['phases']) == baseline_reader.PHASES and len(baseline['raw']) == 39,
            'six actually completed baseline phases and full raw census')
    for name, phase in baseline['phases'].items():
        require(type(phase['exit_code']) is int and phase['exit_code'] == (101 if name in baseline['tests'] else 0)
                and phase['reason'] is None and phase['group_absent'] is True, 'actual baseline leaf status')
    for record in [*baseline['raw'].values(), *baseline['artifacts'].values()]: outer.pin_exact(pins, record, BASELINE)
    for name, expected in baseline['tests'].items():
        stdout = p.read(BASELINE / (name + '-stdout'), baseline['raw'][name + '-stdout'], retain=True)[2].decode()
        stderr = p.read(BASELINE / (name + '-stderr'), baseline['raw'][name + '-stderr'], retain=True)[2].decode()
        require(baseline_reader.observed_tests(stdout, stderr, expected['names'], expected['filtered_out'], baseline['phases'][name]) == expected,
                'replayed exact original-source named assertion, not a predicted baseline')
    manifest, package_pin = pins.json(PACKAGE / 'manifest.json', package_sha)
    require(manifest['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-package-v2'
            and len(manifest['files']) == 3 and {row['path'] for row in manifest['files']} == {'run.py', 'test_run.py', 'README.md'},
            'closed three-file runner package')
    for row in manifest['files']:
        outer.pin_exact(pins, dict(row, path=str(PACKAGE / row['path'])))
    require(source_sha == SOURCE_SHA, 'reviewed frozen indexed source manifest')
    proposal, proposal_pin = pins.json(PATCH / 'source-manifest.json', source_sha)
    proposal_contract(proposal, v.CPU_PIN, cpu['raw']['sources-before.json'], baseline_pin)
    require(len(proposal['files']) == 5 and {row['path'] for row in proposal['files']} == FILES, 'closed V2 indexed source roster')
    old_rows = {row['path']: row for row in previous_source['files']}
    require(proposal['added_tests'] == previous_source['added_tests']
            and all(row == old_rows[row['path']] for row in proposal['files'] if row['path'] in old_rows)
            and proposal['diagnostic'] == diagnostic_pin, 'four indexed bodies/test names unchanged from failed V1')
    for row in proposal['files']:
        require(row['source'] == 'draft/' + row['path'], 'exact overlay source path')
        outer.pin_exact(pins, dict(row['after'], path=str(PATCH / row['source'])))
    pins.pin(BASE, BASE_SHA); pins.pin(v.BASE, v.BASE_SHA)
    pins.pin(base.FORMATTER, base.FORMATTER_SHA)
    for name, sha in base.HELPERS.values():
        pins.pin(E / name, sha)
    return dict(v=v, base=base, modules=modules, pins=pins, cpu=cpu, tools=tools,
        proposal=proposal, proposal_pin=proposal_pin, package_pin=package_pin,
        diagnostic_pin=diagnostic_pin, diagnostic_owner_pin=diagnostic_owner_pin,
        diagnostic_artifacts=diagnostic['artifacts'],
        baseline_pin=baseline_pin, baseline_owner_pin=baseline_owner_pin, baseline_names=baseline_names,
        baseline_artifacts=baseline['artifacts'], previous_artifacts=previous['artifacts'],
        previous_command=p.parse(p.read(ROW / 'actual-inert-join-command.json', retain=True)[2]))


def child(c):
    base, v, m, pins = c['base'], c['v'], c['modules'], c['pins']
    p, n = m['probe'], m['bounded']
    require(not os.path.lexists(OUT), 'fresh indexed-join case')
    OUT.mkdir(mode=0o700)
    n.F, n.T, n.D = COPY, TARGET, OUT
    n.setup()
    require(not any(TARGET.iterdir()), 'fresh empty target')
    cpu, proposal = c['cpu'], c['proposal']
    original = p.parse(p.read(CPU / 'sources-before.json', cpu['raw']['sources-before.json'], retain=True)[2])
    inputs = p.parse(p.read(CPU / 'inputs-before.json', cpu['raw']['inputs-before.json'], retain=True)[2])
    dependencies_old = p.parse(p.read(CPU / 'dependencies-before.json', cpu['raw']['dependencies-before.json'], retain=True)[2])
    require(p.snapshot(p.tree(ORIGINAL)) == original and len(original) == 5783, 'exact qualified RPO source before copy')
    require(p.snapshot(sorted(inputs['files']), byte_cap=4 << 30) == inputs['files']
            and p.snapshot(sorted(dependencies_old), byte_cap=4 << 30) == dependencies_old, 'qualified compiler inputs/dependencies')
    before_relative = base.relative_sources(original, ORIGINAL)
    for path, row in original.items():
        target = COPY / Path(path).relative_to(ORIGINAL)
        target.parent.mkdir(parents=True, exist_ok=True)
        raw = p.read(path, row['pin'], retain=True)[2]
        with target.open('xb') as stream: stream.write(raw)
        target.chmod(0o700 if Path(path).stat().st_mode & 0o111 else 0o600)
    require(base.relative_sources(p.snapshot(p.tree(COPY)), COPY) == before_relative, 'exact fresh source copy')
    for row in proposal['files']:
        require(before_relative.get(row['path']) == row['before']
                and (row['path'] in NEW_FILES) == (row['before'] is None), 'exact original preimage or absent new body')
        raw = p.read(PATCH / row['source'], dict(row['after'], path=str(PATCH / row['source'])), retain=True)[2]
        with (COPY / row['path']).open('wb') as stream: stream.write(raw)
    unformatted = p.snapshot(p.tree(COPY))
    transition(before_relative, base.relative_sources(unformatted, COPY), proposal)
    require(len(unformatted) == 5785, 'exact two-source addition to original qualified tree')
    p.save(OUT / 'sources-unformatted.json', unformatted)
    config_names = set(inputs['configurations']) | set(p.config_paths())
    p.F = COPY
    config_names.update(p.config_paths())
    p.config_paths = lambda: sorted(config_names)
    configurations = {name: None for name in config_names}
    config_before = p.configurations(configurations)
    require(all(config_before[name] == value for name, value in inputs['configurations'].items()), 'unchanged prior Cargo configuration')
    p.save(OUT / 'configurations-before.json', config_before)
    env = n.environment()
    (TARGET / 'tmp').mkdir(mode=0o700)
    env['TMPDIR'] = str(TARGET / 'tmp')
    selected_env = selected_environment(c['previous_command'], env)
    cargo = str(n.N / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(COPY / 'Cargo.toml')]
    base.COPY, base.OUT = COPY, OUT
    phases, artifacts, results, observed_join, lower_roster = {}, {}, {}, None, None
    before, dependencies, roots, formatted = unformatted, None, None, {}
    error, post_errors = None, []

    def run(name, argv, deadline=1800, selected=env, expected_exit=0):
        try:
            n.run(OUT, name, argv, env=selected, deadline=deadline, expected_exit=expected_exit)
        finally:
            path = OUT / (name + '-result.json')
            if path.is_file(): phases[name] = p.parse(p.read(path, retain=True)[2])
        return p.read(OUT / (name + '-stdout'), cap=64 << 20, retain=True)[2].decode()

    def dependency_snapshot():
        return p.snapshot(sorted({name for root in roots for name in p.tree(root, exclusions=('.git', 'target'))}), byte_cap=4 << 30)

    try:
        fmt = [str(base.FORMATTER), '--edition', '2024', '--config', 'skip_children=true', *[str(COPY / name) for name in sorted(FILES)]]
        run('rustfmt', fmt, 60)
        before = p.snapshot(p.tree(COPY))
        transition(before_relative, base.relative_sources(before, COPY), proposal, formatted=True)
        p.save(OUT / 'sources-before.json', before)
        formatted = {name: p.read(COPY / name)[0] for name in sorted(FILES)}
        run('rustfmt-check', [fmt[0], '--check', *fmt[1:]], 60)
        metadata = p.parse(run('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(COPY / 'Cargo.toml'), '--format-version', '1'], 120))
        previous = p.parse(p.read(CPU / 'metadata-stdout', cpu['raw']['metadata-stdout'], retain=True)[2])
        require(metadata == relocate(previous), 'exact relocated qualified Cargo metadata')
        roots = base.metadata_paths(metadata, COPY)
        dependencies = dependency_snapshot()
        dependency_transition(dependencies_old, dependencies, before)
        p.save(OUT / 'dependencies-before.json', dependencies)
        built = run('lower-build-tests', [cargo, 'test', *common, '-p', 'fe2o3-lower-mir-kernel', '--lib', '--no-run', '--message-format=json'])
        paths = base.built_artifacts(built, 'fe2o3-lower-mir-kernel', 'fe2o3_lower_mir_kernel', True, TARGET)
        require(len(paths) == 1, 'one actual lower-library test executable')
        artifacts['lower-tests'] = p.read(paths[0])[0]
        lower_binary = paths[0]
        lower_names = base.inventory(run('lower-list', [lower_binary, '--list', '--format', 'terse'], 120))
        lower_ignored = sorted(re.findall(r'^([^\r\n]+): test$', run('lower-ignored-list', [lower_binary, '--ignored', '--list', '--format', 'terse'], 120), re.M))
        lower_roster = lower_inventory(lower_names, lower_ignored, proposal, c['baseline_names'])
        results['lower'] = base.test_results(run('lower-tests', [lower_binary, '--test-threads=2']), lower_names, lower_ignored)
        selected = run('lower-indexed-tests', [lower_binary, TEST_PREFIX, '--test-threads=2'])
        results['indexed_subset_repeat'] = base.test_results(selected, sorted(proposal['added_tests']), [])
        require(re.findall(r'(\d+) measured; (\d+) filtered out;', selected) == [('0', str(len(lower_names) - 20))],
                'twenty new tests execute again as an explicit subset, not additional unique tests')
        built = run('finalizer-build-tests', [cargo, 'test', *common, '-p', 'fe2o3-hsaco-finalize', '--example', v.FINALIZER, '--no-run', '--message-format=json'])
        paths = base.built_artifacts(built, 'fe2o3-hsaco-finalize', v.FINALIZER, True, TARGET)
        require(len(paths) == 1, 'one candidate finalizer-test executable')
        artifacts['finalizer-tests'] = p.read(paths[0])[0]
        binary = paths[0]
        names = base.inventory(run('finalizer-list', [binary, '--list', '--format', 'terse'], 120))
        ignored = sorted(re.findall(r'^([^\r\n]+): test$', run('finalizer-ignored-list', [binary, '--ignored', '--list', '--format', 'terse'], 120), re.M))
        v.finalizer_inventory(names, ignored, c['tools']['tests'])
        result_raw = run('finalizer-tests', [binary, '--test-threads=2'])
        results['finalizer'] = base.test_results(result_raw, names, ignored)
        require('kir-join-work-v1' not in result_raw + p.read(OUT / 'finalizer-tests-stderr', retain=True)[2].decode(), 'default suite has no diagnostic flag/output')
        observed = run('actual-inert-join', [binary, *c['previous_command']['argv'][1:]], 900, selected_env)
        observed_join = actual_join(observed, p.read(OUT / 'actual-inert-join-stderr', retain=True)[2].decode(), phases['actual-inert-join'])
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        checks = [('original', original, lambda: p.snapshot(p.tree(ORIGINAL))),
            ('sources', before, lambda: p.snapshot(p.tree(COPY))),
            ('inputs', inputs['files'], lambda: p.snapshot(sorted(inputs['files']), byte_cap=4 << 30)),
            ('prior-dependencies', dependencies_old, lambda: p.snapshot(sorted(dependencies_old), byte_cap=4 << 30)),
            ('configurations', config_before, lambda: p.configurations(configurations))]
        if dependencies is not None: checks.append(('dependencies', dependencies, dependency_snapshot))
        for name, expected, operation in checks:
            try:
                actual = operation(); p.save(OUT / (name + '-after.json'), actual)
                require(actual == expected, name + ' changed')
            except BaseException as failure:
                post_errors.append(name + ': ' + repr(failure))
        try:
            pins.recheck()
            for record in artifacts.values(): p.read(record['path'], record)
        except BaseException as failure:
            post_errors.append('pinned inputs/products: ' + repr(failure))
    passed = error is None and not post_errors and set(phases) == PHASES and observed_join is not None
    value = dict(schema='ferric-p228-kir-indexed-formal-join-cpu-result-v2', passed=passed,
        error=error, postcheck_errors=post_errors, staged_join_qualification_completed=passed,
        package=c['package_pin'], proposal=c['proposal_pin'], compiler_cpu=v.CPU_PIN,
        finalizer_tools=dict(path=str(TOOLS / 'complete.json'), **dict(zip(('bytes', 'sha256'), FIXED[str(TOOLS / 'complete.json')]))),
        source_generation=cpu['raw']['sources-before.json'], formatted_sources=formatted,
        phases=phases, tests=results, actual_join=observed_join, artifacts=artifacts,
        added_tests=proposal['added_tests'], lower_inventory=lower_roster, prior_compiled_library_inventory_available=True,
        original_source_baseline=c['baseline_pin'], original_source_baseline_owner=c['baseline_owner_pin'],
        previous_failed_candidate=PREVIOUS_PIN, corrected_existing_test=FAILURE,
        prior_diagnostic=c['diagnostic_pin'], prior_diagnostic_owner=c['diagnostic_owner_pin'],
        preserved_compiler_products=cpu['artifacts'], preserved_finalizer_products=c['tools']['artifacts'],
        preserved_diagnostic_products=c['diagnostic_artifacts'],
        preserved_baseline_products=c['baseline_artifacts'], preserved_failed_candidate_products=c['previous_artifacts'],
        retained_handoff=HANDOFF, source_unchanged=not post_errors, limits_changed=False,
        actual_capture_join_passed=observed_join is not None, fresh_compiler_built=False, fresh_hsaco_emitted=False,
        full_compiler_cohort_requalified=False,
        gpu_execution=False, numerical_acceptance=False, production_authority=False, performance_claim=False,
        raw={path.name: p.read(path)[0] for path in OUT.iterdir() if path.is_file()})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python -B')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest_sha256'); parser.add_argument('source_manifest_sha256')
    parser.add_argument('--child', action='store_true')
    args = parser.parse_args()
    require(all(re.fullmatch('[0-9a-f]{64}', value) for value in (args.manifest_sha256, args.source_manifest_sha256)), 'root-authenticated package/source digests')
    require(Path(__file__).resolve().parent == PACKAGE and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'asrock-1w300-g2-2b' and os.sched_getaffinity(0) == {8, 9}
            and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'fixed owned CPU host and package')
    for kind, cap in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        limit = min([cap] + [word for word in old if word != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    c = context(args.manifest_sha256, args.source_manifest_sha256)
    if args.child:
        signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt('owned termination')))
        return child(c)
    p, outer, driver = (c['modules'][key] for key in ('probe', 'outer', 'driver'))
    require(not os.path.lexists(OWNER) and not os.path.lexists(OUT), 'fresh indexed owner and case')
    old_paths = tuple(dict.fromkeys((*outer.OLD_TARGETS, *c['v'].EXTRA_OLD_TARGETS, CPU / 'target', ROW / 'target',
        DIAGNOSTIC / 'target', PREVIOUS / 'target', BASELINE / 'target')))
    require(all(not TARGET.is_relative_to(path) and not path.is_relative_to(TARGET) for path in old_paths), 'fresh target separate from protected targets')
    old_targets = outer.inventory(c['modules']['inventory'], old_paths)
    os.umask(0o077); OWNER.mkdir(mode=0o700)
    p.save(OWNER / 'old-targets-before.json', old_targets)
    argv = ['/usr/bin/python3', '-B', str(Path(__file__).resolve()), args.manifest_sha256, args.source_manifest_sha256, '--child']
    env = c['modules']['bounded'].environment()
    p.save(OWNER / 'command.json', dict(argv=argv, env=env, deadline_seconds=10800, gpu_execution=False))
    outcome, completion, error = None, None, None
    try:
        outcome = driver.run_coordinator(OWNER, argv, ORIGINAL, env, c['modules']['owned'], p.save, deadline=10800)
        p.save(OWNER / 'owned-result.json', outcome); outer.check_outcome(outcome)
        value, completion = c['pins'].json(OUT / 'complete.json')
        require(value['passed'] is True and value['staged_join_qualification_completed'] is True and value['postcheck_errors'] == []
                and value['package'] == c['package_pin'] and value['proposal'] == c['proposal_pin']
                and value['tests']['finalizer'] == c['tools']['tests'] and value['actual_capture_join_passed'] is True
                and value['actual_join']['actual_capture_join_passed'] is True and set(value['phases']) == PHASES,
                'actual library/finalizer regressions and retained join, never emission acceptance')
        for record in value['raw'].values(): outer.pin_exact(c['pins'], record, OUT)
        for record in value['artifacts'].values(): outer.pin_exact(c['pins'], record, TARGET)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        after, errors = outer.postchecks([('pins', None, c['pins'].recheck),
            ('old_targets', old_targets, lambda: outer.inventory(c['modules']['inventory'], old_paths))])
        p.save(OWNER / 'after.json', after)
    passed = error is None and not errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-kir-indexed-formal-join-owned-result-v2', passed=passed,
        error=error, postcheck_errors=errors, owned=outcome, completion=completion,
        package=c['package_pin'], proposal=c['proposal_pin'], gpu_execution=False,
        actual_capture_join_passed=passed, fresh_hsaco_emitted=False, production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
