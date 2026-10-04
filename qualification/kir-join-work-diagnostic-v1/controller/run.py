"""Fresh instrumented finalizer tests and one expected retained-work refusal."""
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
OUT = E / 'kir-join-work-diagnostic-cpu-v228-v1'
OWNER = E / 'kir-join-work-diagnostic-cpu-owner-v228-v1'
COPY, TARGET = OUT / 'source/fe2o3', OUT / 'target'
PACKAGE = E / 'p228-kir-join-work-diagnostic-cpu-v1'
PATCH = E / 'p228-kir-join-work-diagnostic-v1'
BASE = E / 'p228-partial-move-rpo-finalizer-tools-v2/run.py'
BASE_SHA = 'cd5fb1f8254e3eb3a2c8df4881432f3c73fe1bb18183cee73e7a0f3c8ad30b25'
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
FILES = {'crates/fe2o3-hsaco-finalize/examples/finite_join_engineering_hsaco_v1/wave_qkv_attention_output_tiles_v6.rs',
    'crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1/wave_formal_evidence_v1.rs',
    'crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1/wave_formal_evidence_join_v1.rs'}
PHASES = {'rustfmt', 'rustfmt-check', 'metadata', 'finalizer-build-tests', 'finalizer-list',
          'finalizer-ignored-list', 'finalizer-tests', 'actual-inert-join-diagnostic'}
SELECTOR = 'linux::wave_qkv_attention_output_tiles_v6::tests::wave_emission_actual_retained_v6_passes_full_inert_join'
HANDOFF = dict(path=str(ROW / 'prefix-tiles.handoff-v3'), bytes=4078537,
    sha256='ad4b31ee88efa54702dc8dff13e1e3c331c296734f7cdcfa52e8249ed9fa37fc')
DIAGNOSTIC_ENV = 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1'
WORK_ACTUAL, WORK_LIMIT = 1084825160, 1073741824
STORAGE_LIMIT = 128 << 20
STAGES = ('outer-middle-before', 'outer-middle-after', 'outer-formal-before',
    'formal-archive-before', 'formal-archive-after', 'formal-receipt-before', 'formal-receipt-after',
    'formal-kir-before', 'formal-kir-after', 'formal-middle-before', 'formal-middle-after',
    'formal-layout-before', 'layout-bytes-before', 'layout-bytes-after', 'join-shape',
    'layout-join-before', 'layout-join-after', 'formal-layout-after', 'outer-formal-after',
    'ocml-before', 'ocml-after')
BULK = {'formal-archive', 'formal-receipt', 'layout-bytes', 'layout-join', 'ocml'}


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
    require(len(proposal['files']) == 3 and {row['path'] for row in proposal['files']} == FILES,
            'only three instrumentation bodies')
    expected = dict(before)
    for row in proposal['files']:
        require(set(row) == {'path', 'source', 'before', 'after'} and row['source'] == 'draft/' + row['path']
                and row['before'] is not None and before[row['path']] == row['before'], 'qualified source preimage')
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
    require(old_local and {str(Path(name).relative_to(COPY)) for name in local} == old_local,
            'exact prior local dependency roster, distinct from full source tree')
    require(all(row == sources[name] for name, row in local.items()), 'local dependencies match copied source pins/stamps')
    require({name: row for name, row in current.items() if name not in local}
            == {name: row for name, row in previous.items() if not Path(name).is_relative_to(ORIGINAL)},
            'unchanged external dependency/provider/toolchain bodies')


def proposal_contract(proposal, cpu_pin, source_pin):
    require(proposal['schema'] == 'ferric-p228-kir-join-work-diagnostic-source-v1'
            and proposal['base_cpu'] == cpu_pin and proposal['base_source_snapshot'] == source_pin,
            'exact diagnostic source generation')
    require(proposal['diagnostic_only'] is True and proposal['compiled'] is False
            and proposal['tests_executed'] is False and proposal['added_tests'] == []
            and proposal['work_limit'] == WORK_LIMIT and proposal['storage_limit'] == STORAGE_LIMIT
            and proposal['handoff'] == HANDOFF and proposal['diagnostic_environment'] == {DIAGNOSTIC_ENV: '1'}
            and proposal['expected_refusal'] == dict(selector=SELECTOR, natural_exit_code=101,
                actual_work=WORK_ACTUAL, work_limit=WORK_LIMIT), 'unchanged diagnostic scope, limits and refusal')


def diagnostic_result(stdout, stderr, result):
    require(type(result['exit_code']) is int and result['exit_code'] == 101 and result['reason'] is None
            and result['group_absent'] is True, 'natural expected-refusal leaf only')
    require(re.findall(r'^test (\S+) \.\.\. (\S+)$', stdout, re.M) == [(SELECTOR, 'FAILED')]
            and re.findall(r'test result: FAILED\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', stdout)
                == [('0', '1', '0', '0', '204')], 'exact one historical ignored test was executed and failed')
    both = stdout + '\n' + stderr
    errors = re.findall(r'CanonicalKernelIrWorkLimitV1 \{ actual: (\d+), limit: (\d+) \}', both)
    require(errors == [(str(WORK_ACTUAL), str(WORK_LIMIT))], 'unchanged actual canonical-work refusal')
    markers = []
    for line in both.splitlines():
        if 'kir-join-work-v1' not in line:
            continue
        counter = re.fullmatch(r'kir-join-work-v1 stage=([a-z-]+) work=(\d+) remaining=(\d+) storage=(\d+) peak=(\d+) charge=(\d+)', line)
        shape = re.fullmatch(r'kir-join-work-v1 stage=join-shape rows=(\d+) blocks=(\d+) nodes=(\d+) charge=(\d+)', line)
        require(counter or shape, 'closed integer diagnostic marker')
        if counter:
            stage, *words = counter.groups()
            row = dict(stage=stage, **dict(zip(('work', 'remaining', 'storage', 'peak', 'charge'), map(int, words))))
            require(stage in STAGES and stage != 'join-shape' and row['work'] + row['remaining'] == WORK_LIMIT
                    and row['storage'] <= row['peak'] <= STORAGE_LIMIT, 'unchanged counter budget and peak semantics')
            require(stage.endswith('-before') and stage[:-7] in BULK or row['charge'] == 0,
                    'only actual bulk-before markers carry a pending charge')
        else:
            row = dict(stage='join-shape', **dict(zip(('rows', 'blocks', 'nodes', 'charge'), map(int, shape.groups()))))
            require(row['charge'] == (row['rows'] + row['blocks'] + 1) * row['nodes'] * 128,
                    'recorded unchanged bulk-work formula')
        markers.append(row)
    require(0 < len(markers) <= len(STAGES) and tuple(row['stage'] for row in markers) == STAGES[:len(markers)],
            'nonempty exact stage prefix without duplicates or inferred failure site')
    counters = [row for row in markers if row['stage'] != 'join-shape']
    require(all(left['work'] <= right['work'] and left['peak'] <= right['peak']
                for left, right in zip(counters, counters[1:])), 'monotone accepted work and observed peak')
    for left, right in zip(markers, markers[1:]):
        if left['stage'].endswith('-before') and left['stage'][:-7] in BULK:
            require(right['stage'] == left['stage'][:-7] + '-after'
                    and right['work'] - left['work'] == left['charge'], 'exact accepted bulk charge delta')
        if left['stage'] == 'join-shape':
            require(right['charge'] == left['charge'], 'shape charge matches the actual next join charge')
    last = markers[-1]
    if last['stage'].endswith('-before') and last['stage'][:-7] in BULK:
        require(last['charge'] > 0 and last['work'] + last['charge'] == WORK_ACTUAL,
                'observed refused bulk charge matches original attempted work')
    return dict(expected_refusal_observed=True, test=SELECTOR, exit_code=101,
        error=dict(actual=WORK_ACTUAL, limit=WORK_LIMIT), markers=markers,
        last_observed_stage=markers[-1]['stage'], failure_phase_inferred=False,
        total_required_work_known=False, actual_capture_join_passed=False)


def selected_environment(previous, env):
    prefix = 'FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_'
    inputs = {prefix + 'PATH': HANDOFF['path'], prefix + 'BYTES': str(HANDOFF['bytes']), prefix + 'SHA256': HANDOFF['sha256']}
    require(previous['argv'][1:] == ['--exact', SELECTOR, '--ignored', '--show-output', '--test-threads=1']
            and all(previous['env'][key] == value for key, value in inputs.items()), 'exact previous ignored test and handoff')
    expected = dict(previous['env'])
    for key in ('CARGO_TARGET_DIR', 'TMPDIR', 'LD_LIBRARY_PATH'):
        expected[key] = env[key]
    require(expected == dict(env, **inputs) and DIAGNOSTIC_ENV not in previous['env'],
            'only fresh target/loader/temp and diagnostic flag change')
    return dict(env, **inputs, **{DIAGNOSTIC_ENV: '1'})


def context(package_sha, source_sha):
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
    manifest, package_pin = pins.json(PACKAGE / 'manifest.json', package_sha)
    require(manifest['schema'] == 'ferric-p228-kir-join-work-diagnostic-cpu-package-v1'
            and len(manifest['files']) == 3 and {row['path'] for row in manifest['files']} == {'run.py', 'test_run.py', 'README.md'},
            'closed three-file runner package')
    for row in manifest['files']:
        outer.pin_exact(pins, dict(row, path=str(PACKAGE / row['path'])))
    proposal, proposal_pin = pins.json(PATCH / 'source-manifest.json', source_sha)
    proposal_contract(proposal, v.CPU_PIN, cpu['raw']['sources-before.json'])
    require(len(proposal['files']) == 3 and {row['path'] for row in proposal['files']} == FILES, 'closed diagnostic source roster')
    for row in proposal['files']:
        require(row['source'] == 'draft/' + row['path'], 'exact overlay source path')
        outer.pin_exact(pins, dict(row['after'], path=str(PATCH / row['source'])))
    pins.pin(BASE, BASE_SHA); pins.pin(v.BASE, v.BASE_SHA)
    pins.pin(base.FORMATTER, base.FORMATTER_SHA)
    for name, sha in base.HELPERS.values():
        pins.pin(E / name, sha)
    return dict(v=v, base=base, modules=modules, pins=pins, cpu=cpu, tools=tools,
        proposal=proposal, proposal_pin=proposal_pin, package_pin=package_pin,
        previous_command=p.parse(p.read(ROW / 'actual-inert-join-command.json', retain=True)[2]))


def child(c):
    base, v, m, pins = c['base'], c['v'], c['modules'], c['pins']
    p, n = m['probe'], m['bounded']
    require(not os.path.lexists(OUT), 'fresh diagnostic case')
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
        require(before_relative[row['path']] == row['before'], 'exact original diagnostic preimage')
        raw = p.read(PATCH / row['source'], dict(row['after'], path=str(PATCH / row['source'])), retain=True)[2]
        with (COPY / row['path']).open('wb') as stream: stream.write(raw)
    unformatted = p.snapshot(p.tree(COPY))
    transition(before_relative, base.relative_sources(unformatted, COPY), proposal)
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
    phases, artifacts, results, diagnostic = {}, {}, None, None
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
        built = run('finalizer-build-tests', [cargo, 'test', *common, '-p', 'fe2o3-hsaco-finalize', '--example', v.FINALIZER, '--no-run', '--message-format=json'])
        paths = base.built_artifacts(built, 'fe2o3-hsaco-finalize', v.FINALIZER, True, TARGET)
        require(len(paths) == 1, 'one instrumented finalizer-test executable')
        artifacts['finalizer-tests'] = p.read(paths[0])[0]
        binary = paths[0]
        names = base.inventory(run('finalizer-list', [binary, '--list', '--format', 'terse'], 120))
        ignored = sorted(re.findall(r'^([^\r\n]+): test$', run('finalizer-ignored-list', [binary, '--ignored', '--list', '--format', 'terse'], 120), re.M))
        v.finalizer_inventory(names, ignored, c['tools']['tests'])
        result_raw = run('finalizer-tests', [binary, '--test-threads=2'])
        results = base.test_results(result_raw, names, ignored)
        require('kir-join-work-v1' not in result_raw + p.read(OUT / 'finalizer-tests-stderr', retain=True)[2].decode(), 'default suite has no diagnostic flag/output')
        observed = run('actual-inert-join-diagnostic', [binary, *c['previous_command']['argv'][1:]], 900, selected_env, 101)
        diagnostic = diagnostic_result(observed, p.read(OUT / 'actual-inert-join-diagnostic-stderr', retain=True)[2].decode(), phases['actual-inert-join-diagnostic'])
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
    passed = error is None and not post_errors and set(phases) == PHASES and diagnostic is not None
    value = dict(schema='ferric-p228-kir-join-work-diagnostic-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, diagnostic_completed=passed,
        package=c['package_pin'], proposal=c['proposal_pin'], compiler_cpu=v.CPU_PIN,
        finalizer_tools=dict(path=str(TOOLS / 'complete.json'), **dict(zip(('bytes', 'sha256'), FIXED[str(TOOLS / 'complete.json')]))),
        source_generation=cpu['raw']['sources-before.json'], formatted_sources=formatted,
        phases=phases, tests=results, expected_refusal=diagnostic, artifacts=artifacts,
        preserved_compiler_products=cpu['artifacts'], preserved_finalizer_products=c['tools']['artifacts'],
        retained_handoff=HANDOFF, source_unchanged=not post_errors, limits_changed=False,
        actual_capture_join_passed=False, fresh_compiler_built=False, fresh_hsaco_emitted=False,
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
    require(not os.path.lexists(OWNER) and not os.path.lexists(OUT), 'fresh diagnostic owner and case')
    old_paths = tuple(dict.fromkeys((*outer.OLD_TARGETS, *c['v'].EXTRA_OLD_TARGETS, CPU / 'target', ROW / 'target')))
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
        require(value['passed'] is True and value['diagnostic_completed'] is True and value['postcheck_errors'] == []
                and value['package'] == c['package_pin'] and value['proposal'] == c['proposal_pin']
                and value['tests'] == c['tools']['tests'] and value['actual_capture_join_passed'] is False
                and value['expected_refusal']['expected_refusal_observed'] is True and set(value['phases']) == PHASES,
                'default regressions and expected refusal, never image acceptance')
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
        schema='ferric-p228-kir-join-work-diagnostic-owned-result-v1', passed=passed,
        error=error, postcheck_errors=errors, owned=outcome, completion=completion,
        package=c['package_pin'], proposal=c['proposal_pin'], gpu_execution=False,
        actual_capture_join_passed=False, fresh_hsaco_emitted=False, production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
