"""Build matching finalizer tools from the CPU-qualified compiler source copy."""
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
COPY = CPU / 'source/fe2o3'
TARGET = CPU / 'target'
OUT = E / 'rpo-finalizer-tools-v228-v2'
OWNER = E / 'rpo-finalizer-tools-owner-v228-v2'
BASE = E / 'p228-partial-move-rpo-cpu-v2/run.py'
BASE_SHA = '6cb1dc984761bfb44bf03246c08623faba225433cad70d745a606893ab5125ba'
CPU_PACKAGE = dict(path=str(E / 'p228-partial-move-rpo-cpu-v2/manifest.json'), bytes=1917,
    sha256='9fe67c55e38e4713c692abb4e145cd58bcea81ecf162879de57568b81d729603')
# Only actual future completion digests supplied by the root CLI populate these.
CPU_PIN = None
OWNER_PIN = None
PATCHES = {"rpo": dict(path=str(E / 'p228-partial-move-rpo-v2/source-manifest.json'), bytes=5887,
    sha256='939b76eb28df8d7e2b53ae0b4f5034257185f12f077d20e581d9880b006a252c')}
QUALIFIED_GENERATION = {
    "completion": {
        "path": "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/ordinary-induction-compiler-cpu-v228-v1/complete.json",
        "bytes": 369402,
        "sha256": "c45aa83b01bf76fdfd9962b06884611658dab607b001ba67fecc66db9fd5ba97"
    },
    "sources": {
        "path": "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/ordinary-induction-compiler-cpu-v228-v1/sources-before.json",
        "bytes": 3779269,
        "sha256": "5aad4c8ba6520c323310424ce343de2ae5088e7f8c1c0b1fd67853b5fd3bd9f9"
    },
    "owner": {
        "path": "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/ordinary-induction-compiler-cpu-owner-v228-v1/complete.json",
        "bytes": 57318,
        "sha256": "85de66610631612ed2473f9d5fbb80ce89ea7ffc6064c1784834a6751d623609"
    }
}
REQUIRED_TESTS = {
    "compiler": [],
    "pliron": [
        "production::semantic_ssa::partial_moves::rpo_tests::call_destination_only_reinitializes_normal_edge_not_unwind",
        "production::semantic_ssa::partial_moves::rpo_tests::diamond_maybe_moved_rejection_matches_fifo_exactly",
        "production::semantic_ssa::partial_moves::rpo_tests::exact_storage_and_work_boundaries_still_reject_one_below",
        "production::semantic_ssa::partial_moves::rpo_tests::initialization_enqueue_and_skipped_slots_are_metered",
        "production::semantic_ssa::partial_moves::rpo_tests::loop_and_irreducible_fixed_points_match_fifo",
        "production::semantic_ssa::partial_moves::rpo_tests::malformed_priority_maps_and_unreachable_pushes_refuse",
        "production::semantic_ssa::partial_moves::rpo_tests::parallel_edges_and_unreachable_malformed_blocks_match_fifo",
        "production::semantic_ssa::partial_moves::rpo_tests::pointer_carrier_write_keeps_move_rejection_across_join",
        "production::semantic_ssa::partial_moves::rpo_tests::priority_storage_is_added_without_discounting_previous_reservations",
        "production::semantic_ssa::partial_moves::rpo_tests::production_plan_reconstruction_is_deterministic_with_new_accounting",
        "production::semantic_ssa::partial_moves::rpo_tests::rpo_backedge_requeues_an_earlier_rank",
        "production::semantic_ssa::partial_moves::rpo_tests::rpo_priority_is_deterministic_and_deduplicates_pending_blocks",
        "production::semantic_ssa::partial_moves::rpo_tests::scheduler_work_overflow_remains_fail_closed",
        "production::semantic_ssa::partial_moves::rpo_tests::skewed_diamond_reduces_repeated_tombstone_charges",
        "production::semantic_ssa::partial_moves::rpo_tests::union_missing_context_and_unsupported_moves_match_fifo",
        "production::semantic_ssa::partial_moves::rpo_tests::whole_tombstone_widening_and_storage_live_do_not_refund",
        "production::semantic_ssa::partial_moves::rpo_tests::zero_projected_moves_keep_the_empty_certificate_and_old_auxiliary_cost"
    ]
}
PRIOR = E / 'ordinary-induction-finalizer-tools-v228-v1'
PRIOR_PIN = dict(path=str(PRIOR / 'complete.json'), bytes=45313,
    sha256='874a6d2026029619a02283ee9650d682690941c14904d9fa59c101b58b89d6bc')
CPU_PHASES = {'metadata', 'compiler-build', 'rustfmt', 'rustfmt-check'} | {
    short + '-' + suffix for short in ('pliron', 'compiler')
    for suffix in ('build-tests', 'list', 'ignored-list', 'tests')
}
TOOL_PHASES = {'metadata', 'finalizer-build', 'finalizer-build-tests',
               'finalizer-list', 'finalizer-ignored-list', 'finalizer-tests'}
EXTRA_OLD_TARGETS = (E / 'ordinary-induction-compiler-cpu-v228-v1/target',
    E / 'idempotent-compiler-cpu-v228-v2/target', E / 'row-reciprocal-checked-probe-v228-v6/target',
    E / 'row-reciprocal-checked-probe-v228-v7/target')

FINALIZER = 'finite_join_engineering_hsaco_v1'
METADATA = 'finite_join_request_metadata_v1'
ACTUAL_JOIN = ('linux::wave_qkv_attention_output_tiles_v6::tests::'
               'wave_emission_actual_retained_v6_passes_full_inert_join')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load_base():
    require(BASE.resolve(strict=True) == BASE and BASE.is_file(), 'canonical CPU runner')
    raw = BASE.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == BASE_SHA, 'qualified CPU runner hash')
    value = types.ModuleType('qualified_cpu_runner')
    value.__file__ = str(BASE)
    exec(compile(raw, str(BASE), 'exec'), value.__dict__)
    return value


def validate_cpu(cpu, owner):
    require(cpu['schema'] == 'fe2o3-p228-rpo-compiler-cpu-result-v1'
            and owner['schema'] == 'fe2o3-p228-rpo-compiler-owned-result-v1', 'CPU schemas')
    require(cpu['passed'] is True and cpu['fresh_compiler_built'] is True
            and owner['passed'] is True and owner['completion'] == CPU_PIN, 'CPU completion join')
    for value in (cpu, owner):
        require(value['error'] is None and value['postcheck_errors'] == [], 'CPU postchecks')
    require(set(cpu['phases']) == CPU_PHASES and all(row['exit_code'] == 0 and row['reason'] is None
            and row['group_absent'] is True for row in cpu['phases'].values()), 'all CPU phases')
    require(set(cpu['artifacts']) == {'compiler-tests', 'pliron-tests', 'fe2o3-rustc-extract',
                                     'librustc_codegen_fe2o3.so', 'librustc_codegen_fe2o3.rlib'}, 'compiler products')
    require(all(cpu[key] is False for key in ('checked_lowering', 'fresh_hsaco_emitted', 'gpu_execution',
                'production_authority', 'numerical_acceptance', 'performance_claim', 'full_model_acceptance')),
            'CPU-only prerequisite boundary')

    require(cpu['package'] == CPU_PACKAGE, 'exact V2 CPU package')
    require(cpu['patch'] == PATCHES['rpo'] and owner['patch'] == PATCHES['rpo'], 'exact additive RPO compiler overlay')
    require(cpu['qualified_generation'] == QUALIFIED_GENERATION
            and owner['qualified_generation'] == QUALIFIED_GENERATION, 'qualified source generation')
    require(cpu['required_test_names'] == REQUIRED_TESTS and set(cpu['tests']) == set(REQUIRED_TESTS),
            'exact required compiler tests')
    for short, passed, skipped in (('pliron', 1504, 1), ('compiler', 1196, 24)):
        row = cpu['tests'][short]
        names, ignored = row['names'], row['ignored_names']
        require(type(row['passed']) is int and row['passed'] == passed
                and type(row['ignored']) is int and row['ignored'] == skipped
                and names == sorted(set(names)) and ignored == sorted(set(ignored))
                and len(names) == passed + skipped and len(ignored) == skipped
                and set(ignored) <= set(names), 'exact qualified CPU test census')
        require(set(REQUIRED_TESTS[short]) <= set(names) - set(ignored),
                'new and retained compiler tests must actually pass')


def replay_cpu_tests(base, cpu, read):
    for short in ('pliron', 'compiler'):
        def raw(suffix):
            pin = cpu['raw'][short + '-' + suffix + '-stdout']
            require(pin['path'] == str(CPU / (short + '-' + suffix + '-stdout')),
                    'qualified CPU raw test path')
            return read(pin)
        names = base.inventory(raw('list'))
        ignored = sorted(re.findall(r'^([^\r\n]+): test$', raw('ignored-list'), re.M))
        require(base.test_results(raw('tests'), names, ignored) == cpu['tests'][short],
                'actual retained CPU results match qualification')


def finalizer_inventory(names, ignored, prior):
    require(names == prior['names'] and ignored == prior['ignored_names']
            and prior['passed'] == 190 and prior['ignored'] == 15, 'exact actual predecessor test identities')
    require(names == sorted(set(names)) and ignored == sorted(set(ignored))
            and len(names) == 205 and len(ignored) == 15 and set(ignored) <= set(names),
            'exact 190 default tests and 15 ignored tests')
    require(ACTUAL_JOIN in names and ACTUAL_JOIN in ignored,
            'actual-capture join must remain a separate gate')


def protected_targets(old_targets):
    targets = tuple(dict.fromkeys((*old_targets, *EXTRA_OLD_TARGETS)))
    require(all(not TARGET.is_relative_to(path) and not path.is_relative_to(TARGET)
                for path in targets), 'active qualified target is not an unchanged old target')
    return targets


def metadata_paths(value):
    require(value['target_directory'] == str(TARGET), 'same qualified target')
    rows = value['packages']
    require(0 < len(rows) <= 1024, 'bounded dependency inventory')
    for row in rows:
        path = Path(row['manifest_path'])
        require(path.name == 'Cargo.toml' and path.is_relative_to(R), 'owned dependency root')
        require(row['source'] is not None or path.is_relative_to(COPY), 'local package escaped copied source')
    require([row['manifest_path'] for row in rows if row['name'] == 'fe2o3-hsaco-finalize']
            == [str(COPY / 'crates/fe2o3-hsaco-finalize/Cargo.toml')], 'one copied finalizer package')
    return sorted({Path(row['manifest_path']).parent for row in rows})



def replay_finalizer(base, p):
    value = p.parse(p.read(PRIOR_PIN['path'], PRIOR_PIN, retain=True)[2])
    require(value['schema'] == 'fe2o3-p228-ordinary-induction-finalizer-tools-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and set(value['phases']) == TOOL_PHASES and all(
                row['exit_code'] == 0 and row['reason'] is None and row['group_absent'] is True
                for row in value['phases'].values()), 'actual successful six-phase finalizer baseline')
    pins = [PRIOR_PIN]
    for name, pin in value['raw'].items():
        require(pin['path'] == str(PRIOR / name), 'original finalizer raw path')
        p.read(pin['path'], pin)
        pins.append(pin)
    for name, row in value['phases'].items():
        require(p.parse(p.read(PRIOR / (name + '-result.json'), retain=True)[2]) == row,
                'prior finalizer phase result')
        require(all(value['raw'][name + '-' + stream]['sha256'] == row[stream + '_sha256']
                    for stream in ('stdout', 'stderr')), 'prior finalizer phase streams')
    read = lambda suffix: p.read(PRIOR / ('finalizer-' + suffix + '-stdout'), retain=True)[2].decode()
    names = base.inventory(read('list'))
    ignored = sorted(re.findall(r'^([^\r\n]+): test$', read('ignored-list'), re.M))
    finalizer_inventory(names, ignored, value['tests'])
    require(base.test_results(read('tests'), names, ignored) == value['tests'], 'prior full finalizer outcomes')
    return value['tests'], pins


def child(base, modules, package_pin):
    p, n, outer = modules['probe'], modules['bounded'], modules['outer']
    require(not os.path.lexists(OUT), 'fresh finalizer row')
    OUT.mkdir(mode=0o700)
    n.F, n.T, n.D = COPY, TARGET, OUT
    n.setup()
    cpu = p.parse(p.read(CPU_PIN['path'], CPU_PIN, retain=True)[2])
    owner = p.parse(p.read(OWNER_PIN['path'], OWNER_PIN, retain=True)[2])
    validate_cpu(cpu, owner)
    outer.check_outcome(owner['owned'])
    prior_tests, prior_pins = replay_finalizer(base, p)
    prior_cpu = p.parse(p.read(base.QUALIFIED_COMPLETE['path'], base.QUALIFIED_COMPLETE, retain=True)[2])
    proposal = p.parse(p.read(PATCHES['rpo']['path'], PATCHES['rpo'], retain=True)[2])
    for short in ('pliron', 'compiler'):
        base.required_tests(short, cpu['tests'][short]['names'], cpu['tests'][short]['ignored_names'], prior_cpu, proposal)
    for record in cpu['raw'].values():
        p.read(record['path'], record)
    replay_cpu_tests(base, cpu, lambda pin: p.read(pin['path'], pin, retain=True)[2].decode())
    before = p.parse(p.read(CPU / 'sources-before.json', cpu['raw']['sources-before.json'], retain=True)[2])
    dependencies = p.parse(p.read(CPU / 'dependencies-before.json',
                                  cpu['raw']['dependencies-before.json'], retain=True)[2])
    inputs = p.parse(p.read(CPU / 'inputs-before.json', cpu['raw']['inputs-before.json'], retain=True)[2])
    require(p.snapshot(p.tree(COPY)) == before, 'exact qualified compiler source copy')
    require(p.snapshot(sorted(dependencies), byte_cap=4 << 30) == dependencies, 'qualified dependencies')
    require(p.snapshot(sorted(inputs['files']), byte_cap=4 << 30) == inputs['files'], 'qualified tool inputs')
    config_names = sorted(inputs['configurations'])
    p.config_paths = lambda: config_names
    configurations = {name: None for name in config_names}
    require(p.configurations(configurations) == inputs['configurations'], 'qualified configurations')
    fixed_pins = [CPU_PIN, OWNER_PIN, CPU_PACKAGE, package_pin, *PATCHES.values(), *QUALIFIED_GENERATION.values(),
                  *cpu['raw'].values(), *cpu['artifacts'].values(), *prior_pins]
    fixed_paths = [record['path'] for record in fixed_pins] + [str(BASE), str(Path(__file__).resolve())]
    fixed = p.snapshot(fixed_paths, {record['path']: record['sha256'] for record in fixed_pins}, byte_cap=4 << 30)
    p.save(OUT / 'before.json', dict(files=fixed, sources=before, configurations=inputs['configurations']))
    env = n.environment()
    env['TMPDIR'] = str(TARGET / 'tmp')
    cargo = str(n.N / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(COPY / 'Cargo.toml')]
    phases, artifacts, results, roots = {}, {}, None, None
    error, post_errors = None, []

    def run(name, argv, deadline=1800):
        try:
            n.run(OUT, name, argv, env=env, deadline=deadline)
        finally:
            path = OUT / (name + '-result.json')
            if path.is_file(): phases[name] = p.parse(p.read(path, retain=True)[2])
        return p.read(OUT / (name + '-stdout'), cap=64 << 20, retain=True)[2].decode()

    def select(raw, name, tests):
        paths = base.built_artifacts(raw, 'fe2o3-hsaco-finalize', name, tests, TARGET)
        require(len(paths) == 1, 'one actual example executable')
        return p.read(paths[0])[0]

    def dependency_snapshot():
        return p.snapshot(sorted({path for root in roots for path in p.tree(root, exclusions=('.git', 'target'))}),
                          byte_cap=4 << 30)

    try:
        metadata = p.parse(run('metadata', [cargo, 'metadata', '--offline', '--locked',
                               '--manifest-path', str(COPY / 'Cargo.toml'), '--format-version', '1'], 120))
        require(metadata == p.parse(p.read(CPU / 'metadata-stdout', cpu['raw']['metadata-stdout'], retain=True)[2]),
                'same actual CPU dependency metadata')
        roots = metadata_paths(metadata)
        require(dependency_snapshot() == dependencies, 'same full dependency inventory')
        built = run('finalizer-build', [cargo, 'build', *common, '-p', 'fe2o3-hsaco-finalize',
                    '--example', FINALIZER, '--example', METADATA, '--message-format=json'])
        artifacts['finalizer'] = select(built, FINALIZER, False)
        artifacts['metadata'] = select(built, METADATA, False)
        built = run('finalizer-build-tests', [cargo, 'test', *common, '-p', 'fe2o3-hsaco-finalize',
                    '--example', FINALIZER, '--no-run', '--message-format=json'])
        artifacts['finalizer-tests'] = select(built, FINALIZER, True)
        binary = artifacts['finalizer-tests']['path']
        names = base.inventory(run('finalizer-list', [binary, '--list', '--format', 'terse'], 120))
        ignored_raw = run('finalizer-ignored-list', [binary, '--ignored', '--list', '--format', 'terse'], 120)
        ignored = sorted(re.findall(r'^([^\r\n]+): test$', ignored_raw, re.M))
        finalizer_inventory(names, ignored, prior_tests)
        results = base.test_results(run('finalizer-tests', [binary, '--test-threads=2']), names, ignored)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        checks = {
            'files': lambda: p.snapshot(fixed_paths, byte_cap=4 << 30),
            'sources': lambda: p.snapshot(p.tree(COPY)),
            'inputs': lambda: p.snapshot(sorted(inputs['files']), byte_cap=4 << 30),
            'configurations': lambda: p.configurations(configurations),
        }
        expected = dict(files=fixed, sources=before, inputs=inputs['files'], configurations=inputs['configurations'])
        if roots is not None:
            checks['dependencies'] = dependency_snapshot
            expected['dependencies'] = dependencies
        for name, operation in checks.items():
            try:
                actual = operation()
                p.save(OUT / (name + '-after.json'), actual)
                require(actual == expected[name], name + ' changed')
            except BaseException as failure:
                post_errors.append(name + ': ' + repr(failure))
    passed = error is None and not post_errors and set(phases) == TOOL_PHASES
    value = dict(schema='fe2o3-p228-rpo-finalizer-tools-result-v1', passed=passed,
                 error=error, postcheck_errors=post_errors, package=package_pin,
                 compiler_cpu=CPU_PIN, compiler_owner=OWNER_PIN, prior_finalizer=PRIOR_PIN,
                 patches=PATCHES, qualified_generation=QUALIFIED_GENERATION,
                 compiler_required_test_names=REQUIRED_TESTS,
                 compiler_artifacts=cpu['artifacts'], source_snapshot=cpu['raw']['sources-before.json'],
                 phases=phases, tests=results, artifacts=artifacts,
                 checked_lowering=False, actual_capture_join=False, fresh_hsaco_emitted=False,
                 gpu_execution=False, production_authority=False, numerical_acceptance=False,
                 performance_claim=False, full_model_acceptance=False,
                 raw={path.name: p.read(path)[0] for path in OUT.iterdir() if path.is_file()})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def main():
    global CPU_PIN, OWNER_PIN
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode, 'ordinary bytecode-free Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest_sha256')
    parser.add_argument('cpu_sha256')
    parser.add_argument('owner_sha256')
    parser.add_argument('--child', action='store_true')
    args = parser.parse_args()
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'fixed CPU envelope')
    base = load_base()
    modules = base.load_helpers()
    p, outer, driver = modules['probe'], modules['outer'], modules['driver']
    pins = driver.Pins()
    package = Path(__file__).resolve().parent
    require(package == E / 'p228-partial-move-rpo-finalizer-tools-v2', 'isolated finalizer package')
    _, CPU_PIN = pins.json(CPU / 'complete.json', args.cpu_sha256)
    _, OWNER_PIN = pins.json(E / 'rpo-compiler-cpu-owner-v228-v2/complete.json', args.owner_sha256)
    manifest, package_pin = pins.json(package / 'manifest.json', args.manifest_sha256)
    require(len(manifest['files']) == 3 and {row['path'] for row in manifest['files']}
            == {'run.py', 'test_run.py', 'README.md'}, 'closed finalizer package')
    for member in manifest['files']:
        require(Path(member['path']).name == member['path'], 'flat runner package')
        outer.pin_exact(pins, dict(member, path=str(package / member['path'])))
    pins.pin(BASE, BASE_SHA)
    for name, sha in base.HELPERS.values():
        pins.pin(E / name, sha)
    for record in (CPU_PIN, OWNER_PIN, CPU_PACKAGE, *PATCHES.values(), *QUALIFIED_GENERATION.values()):
        outer.pin_exact(pins, record)
    if args.child:
        signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt('owned termination')))
        return child(base, modules, package_pin)
    for kind, cap in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        limit = min([cap] + [n for n in old if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(not os.path.lexists(OWNER) and not os.path.lexists(OUT), 'fresh owned outputs')
    os.umask(0o077)
    OWNER.mkdir(mode=0o700)
    old_target_paths = protected_targets(outer.OLD_TARGETS)
    old_targets = outer.inventory(modules['inventory'], old_target_paths)
    p.save(OWNER / 'old-targets-before.json', old_targets)
    argv = ['/usr/bin/python3', '-B', str(Path(__file__).resolve()), args.manifest_sha256,
            args.cpu_sha256, args.owner_sha256, '--child']
    env = modules['bounded'].environment()
    p.save(OWNER / 'command.json', dict(argv=argv, env=env, deadline_seconds=10800, gpu_execution=False))
    outcome, error, completion = None, None, None
    try:
        outcome = driver.run_coordinator(OWNER, argv, COPY, env, modules['owned'], p.save, deadline=10800)
        p.save(OWNER / 'owned-result.json', outcome)
        outer.check_outcome(outcome)
        value, completion = pins.json(OUT / 'complete.json')
        require(value['passed'] is True and value['package'] == package_pin and value['compiler_cpu'] == CPU_PIN
                and value['compiler_owner'] == OWNER_PIN
                and value['patches'] == PATCHES and value['qualified_generation'] == QUALIFIED_GENERATION
                and value['compiler_required_test_names'] == REQUIRED_TESTS
                and value['gpu_execution'] is False and value['production_authority'] is False, 'tool completion join')
        for record in value['raw'].values():
            outer.pin_exact(pins, record, OUT)
        for record in [*value['artifacts'].values(), *value['compiler_artifacts'].values()]:
            outer.pin_exact(pins, record, TARGET)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        after, post_errors = outer.postchecks([
            ('pins', None, pins.recheck),
            ('old_targets', old_targets, lambda: outer.inventory(modules['inventory'], old_target_paths)),
        ])
        p.save(OWNER / 'after.json', after)
    passed = error is None and not post_errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(
        schema='fe2o3-p228-rpo-finalizer-tools-owned-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, owned=outcome, completion=completion,
        compiler_cpu=CPU_PIN, compiler_owner=OWNER_PIN,
        patches=PATCHES, qualified_generation=QUALIFIED_GENERATION,
        gpu_execution=False, production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
