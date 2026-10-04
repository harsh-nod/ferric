"""One owned SiLU-materialized compiler probe using the preserved checked tool generation."""
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import sys
import time
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-silu-materialized-lowering-v1'
OUT = E / 'row-silu-materialized-checked-probe-v228-v1'
OWNER = E / 'silu-materialized-checked-probe-owner-v228-v1'
SOURCE = E / 'p228-silu-materialized-kernel-v1'
PRIOR = E / 'row-down2-checked-probe-v228-v1'
BASE = E / 'row-source-v225-v9'
DEVICE = 'device/qwen3-tp-wave-rmsnorm-kernels-v15'
SYMBOL = 'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2'
REPLAY = 'production_pipeline::conditional_formal_tests::actual_mlp_v39_reaches_formal_archive_and_outer_lineage_replay'
JOIN = 'linux::wave_mlp_tiles_v2::tests::wave_emission_actual_retained_v1_passes_full_inert_join'
CAPTURES = ('mlp-tiles-semantic.bin', 'mlp-tiles-neutral-kir.bin', 'mlp-tiles-target-kir.bin', 'mlp-tiles.handoff-v3')
STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join',
          'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
FALSE_FLAGS = ('gpu_execution', 'production_authority', 'launch_authority', 'numerical_acceptance',
               'performance_claim', 'runtime_requirements_discharged', 'full_model_acceptance')
FILES = {'run.py', 'contracts.py', 'test_run.py', 'README.md'}
HELPERS = {
    'driver': ('p227-prefix-tiles-source-pipeline-v5/run_row_facts_v2.py',
               '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'),
    'owned': ('p227-prefix-tiles-cohort-v9/frozen_owned.py',
              'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'),
    'inventory': ('p227-prefix-tiles-cohort-v9/native_bounded.py',
                  '48d61daaf67b145f3cf2928c1abadcbcf1295831ee8a1ccf3fd6a442f74cfac0'),
    'probe': ('p228-reciprocal-checked-probe-v7/run.py',
              '1304ab84f9571a313e04d53c0884a3700cb4aa4b25e6744623e9d6468a380235'),
    'outer': ('p228-reciprocal-outer-v6/run.py',
              'd4612f5efbe409d86aadb8e2f653f2d9743abbc4a8b4f709d349ff99bf98bfa2'),
    'down_cpu': ('p228-down2-cpu-v1/run.py',
                 'a48976422c3eba35308d3f210022bcd2b736dcd9928f7b4f40d9f417293f9f24'),
}
RECEIPTS = {
    'template': ('p228-reciprocal-checked-probe-v7/recipe.json',
                 'f500b1c3d3218b53e9ba1df92d6f048a41f05873c9539f16bc69b20f85575a52'),
    'inputs': ('reciprocal-checked-probe-inputs-v228-v7.json',
               '08a85adc9d529ad91c019da4a428ccf69dddb10623d43e4ea8242f7390fec879'),
    'prior_cpu': ('down2-cpu-v228-v1/complete.json',
            '96ef991246b90f9f02b509b3302df0215309bb172f9cae4340c21eba27bda563'),
    'overlay': ('p228-silu-materialized-kernel-v1/source-manifest.json',
                'ca0bafcdd0297d2d849770791cf04686b33f048402f0fb5cc451183217acb276'),
    'prior_lowering': ('row-down2-checked-probe-v228-v1/complete.json',
                       '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e'),
    'baseline': ('row-source-v225-v9/complete.json',
                 'd3bd29c67375e1693d6e4170448a9a2813b1057a4007f0f2583c4b6ec3567891'),
}
OLD_TARGETS = (
    R / 'evidence/wave-attention-v214/target-nightly',
    R / 'evidence/wave-attention-v214/target-native-nightly-v225',
    E / 'idempotent-compiler-cpu-v228-v2/target',
    E / 'ordinary-induction-compiler-cpu-v228-v1/target',
    E / 'row-reciprocal-checked-probe-v228-v7/target',
    E / 'independent-native-profile-cpu-v228-v1/target',
    E / 'gfx950-clock-recorder-cpu-v228-v1/target',
    E / 'gfx950-clock-parent-cpu-v228-v1/target',
    E / 'down2-cpu-v228-v1/target',
)


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def guard(optimized, env):
    require(not optimized and 'PYTHONOPTIMIZE' not in env, 'optimized Python refused before helpers')


def load_driver():
    path, digest = E / HELPERS['driver'][0], HELPERS['driver'][1]
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical retained driver')
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == digest, 'retained coordinator digest')
    module = types.ModuleType('silu_retained_driver')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def rewrite(value, old, new):
    if isinstance(value, str):
        if value == str(old):
            return str(new)
        if value.startswith(str(old) + '/'):
            return str(new) + value[len(str(old)):].replace('/prefix-tiles', '/mlp-tiles')
        return value
    if isinstance(value, list):
        return [rewrite(item, old, new) for item in value]
    if isinstance(value, dict):
        return {key: rewrite(item, old, new) for key, item in value.items()}
    return value


def make_recipe(template, fixture):
    require(tuple(row['name'] for row in template['commands']) == STAGES, 'retained nine-stage template')
    commands = rewrite(copy.deepcopy(template['commands']), template['fresh_output'], OUT)
    by_name = {row['name']: row for row in commands}
    replay = by_name['actual-replay']
    replay['argv'][2] = REPLAY
    for suffix in ('', '_SHA256'):
        old = 'FE2O3_PREFIX_TILE_V6_SEMANTIC_CAPTURE' + suffix
        require(old in replay['env'], 'retained replay environment shape')
        replay['env']['FE2O3_MLP_TILE_V2_SEMANTIC_CAPTURE' + suffix] = replay['env'].pop(old)
    join = by_name['actual-inert-join']
    join['argv'][2] = JOIN
    for suffix in ('PATH', 'SHA256', 'BYTES'):
        old = 'FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_' + suffix
        join['env']['FE2O3_WAVE_MLP_TILE_ENGINEERING_V2_' + suffix] = join['env'].pop(old)
    by_name['emit']['argv'][1] = '--wave-mlp-tiles-v2'
    result = {key: copy.deepcopy(template[key]) for key in
              ('toolchain_and_retained_tool_pins', 'compiler_generation', 'retained_modules')}
    result.update(schema='ferric-p228-silu-materialized-lowering-recipe-v1', fresh_output=str(OUT),
                  fresh_target=str(OUT / 'target'), fixture=fixture, commands=commands, symbol=SYMBOL)
    require(all(row['cache_cap_bytes'] == 6 << 30 and row['affinity'] == [8, 9]
                and row['nice'] == 10 and row['expected_exit'] == 0 and row['gpu_execution'] is False
                and row['env']['CARGO_TARGET_DIR'] == result['fresh_target'] for row in commands), 'unchanged leaf bounds')
    return result


RUST_FILES = {'src/lib.rs', 'src/wave_numerics_v1.rs', 'src/mlp_numerics_v1.rs',
              'src/mlp_tile_numerics_v2.rs', 'src/mlp_silu_materialized_numerics_v1.rs'}
FIXTURE_FILES = RUST_FILES | {'Cargo.toml', 'Cargo.lock'}
TARGET_COUNTS = {'mlp_tiles_numerics_v2': 4, 'mlp_down_two_row_v1': 10,
                 'mlp_claimed_numerics_v1': 8, 'mlp_silu_materialized_v1': 16}
CLAIMED_TESTS = {
    'both_projection_handlers_preserve_all_rows_and_only_lane_zero_writes',
    'rejected_projection_and_down_still_execute_every_collective',
    'down_keeps_fp32_low_bits_and_meets_independent_fp64_bound',
    'specialized_down_matches_every_active_queued_iteration_and_overflow_status',
    'norm_preserves_two_bf16_rounds_and_all_lane_coverage',
    'norm_rejects_collective_invalid_math_and_local_write_failure',
    'swiglu_full_coverage_stable_expression_and_zero_signs',
    'swiglu_invalid_inputs_exp_and_writes_finish_all_components',
}
CPU_PHASES = ('rustfmt', 'rustfmt-check', 'metadata', 'build-tests') + tuple(
    name + suffix for name in TARGET_COUNTS for suffix in ('-list', '-ignored-list', ''))
CPU_RAW = {name + suffix for name in CPU_PHASES
           for suffix in ('-command.json', '-started.json', '-result.json', '-stdout', '-stderr')} | {
    'sources-unformatted.json', 'sources-before.json', 'sources-after.json',
    'dependencies-before.json', 'dependencies-after.json', 'inputs.json'}
CPU_RUNNER = dict(path=str(E / 'p228-silu-materialized-cpu-v1/run.py'), bytes=20305,
                  sha256='c083d3fc80405d10767d1a0d231965e240bdee9953978bf8142e2ad3935c1f82')


def cpu_argument(path, digest):
    path = Path(path)
    require(path.parent.parent == E and path.name == 'complete.json'
            and re.fullmatch(r'silu-materialized-cpu-v228-v[1-9][0-9]*', path.parent.name)
            and type(digest) is str and re.fullmatch('[0-9a-f]{64}', digest), 'actual CPU completion path and SHA')
    return path


def pin_exact(pins, record):
    require(set(record) == {'path', 'bytes', 'sha256'} and type(record['bytes']) is int
            and record['bytes'] >= 0 and type(record['sha256']) is str
            and re.fullmatch('[0-9a-f]{64}', record['sha256']), 'exact FilePin')
    require(pins.pin(Path(record['path']), record['sha256']) == record, 'retained exact pin')
    return record


def fixture_contract(rows):
    require(len(rows) == 7 and {row['destination'] for row in rows} == FIXTURE_FILES,
            'closed five-Rust/seven-total fixture')
    require(len({row['source']['path'] for row in rows}) == 7, 'distinct source identities')
    return sorted(rows, key=lambda row: row['destination'])


def source_transition(before, after, unformatted, prior, overlay):
    additions = {row['path'].removeprefix(DEVICE + '/'): row['after'] for row in overlay['files']}
    require(len(additions) == 3 and not (set(additions) & set(prior)), 'only three additive CPU files')
    require(len(prior) == 31 and unformatted == {**prior, **additions}, 'exact prior CPU fixture plus SiLU')
    require(before == after and len(before['fixture']) == 34
            and set(before['fixture']) == set(unformatted)
            and all(before['fixture'][name] == value for name, value in prior.items()),
            'formatter and tests preserve every original CPU fixture body')


def cpu_gate(p, d, cpu, overlay, values, provider_paths, pins, cpu_path):
    require(cpu['schema'] == 'ferric-p228-silu-materialized-cpu-result-v1' and cpu['passed'] is True
            and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
            and cpu['tests_passed'] == 38 and cpu['tests_ignored'] == 0 and cpu['cpu_arithmetic_only'] is True,
            'actual CPU38 arithmetic/source qualification')
    require(all(cpu[key] is False for key in ('gpu_execution', 'compiler_hsaco_reproduced',
                'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority')),
            'CPU receipt grants no downstream acceptance')
    require(cpu['actual_provider_source_sha256'] == p.PROVIDER_SHA, 'same actual provider53')
    require(cpu['overlay'] == pins.records[str(SOURCE / 'source-manifest.json')]
            and cpu['prior_cpu'] == pins.records[str(E / RECEIPTS['prior_cpu'][0])]
            and cpu['prior_lowering'] == pins.records[str(PRIOR / 'complete.json')], 'exact CPU lineage')
    root = cpu_path.parent
    require(cpu['runner'] == CPU_RUNNER, 'reviewed actual CPU controller')
    pin_exact(pins, cpu['runner'])
    require(cpu['fixture'] == str(root / 'fixture'), 'isolated CPU fixture')
    require(set(cpu['phases']) == set(CPU_PHASES) and set(cpu['raw']) == CPU_RAW,
            'exact sixteen phases and eighty-six raw files')
    for name, record in cpu['raw'].items():
        require(Path(name).name == name and record['path'] == str(root / name), 'CPU raw namespace')
        pin_exact(pins, record)
    for name, value in cpu['phases'].items():
        actual, _ = pins.json(root / (name + '-result.json'))
        require(actual == value and value['exit_code'] == 0 and value['reason'] is None
                and value['group_absent'] is True, 'natural CPU phase outcome')
        require(all(value[key + '_sha256'] == cpu['raw'][name + '-' + key]['sha256']
                    for key in ('stdout', 'stderr')), 'CPU phase streams')
        command, _ = pins.json(root / (name + '-command.json'))
        require(command['affinity'] == [8, 9] and command['nice'] == 10
                and command['cache_cap_bytes'] == 6 << 30 and command['expected_exit'] == 0
                and command['gpu_execution'] is False
                and command['env']['CARGO_TARGET_DIR'] == str(root / 'target')
                and all(command['env'][key] == '' for key in
                        ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
                'unchanged isolated CPU phase bounds')
    require(set(cpu['binaries']) == set(TARGET_COUNTS), 'four compiled test artifacts')
    messages = [json.loads(line) for line in (root / 'build-tests-stdout').read_text().splitlines()
                if line.startswith('{')]
    require([row.get('success') for row in messages if row.get('reason') == 'build-finished'] == [True],
            'actual successful Cargo build')
    artifacts = [row for row in messages if row.get('reason') == 'compiler-artifact'
                 and row.get('target', {}).get('kind') == ['test']]
    require(len(artifacts) == 4 and {row['target']['name'] for row in artifacts} == set(TARGET_COUNTS),
            'closed compiled test artifact roster')
    for artifact in artifacts:
        name, profile = artifact['target']['name'], artifact['profile']
        binary = cpu['binaries'][name]
        require(binary['cargo'] == artifact and binary['binary']['path'] == artifact['executable']
                and artifact['manifest_path'] == str(root / 'fixture/Cargo.toml')
                and artifact['target']['src_path'] == str(root / 'fixture/tests' / (name + '.rs'))
                and Path(artifact['executable']).is_relative_to(root / 'target')
                and profile['test'] is True and profile['opt_level'] == '2'
                and profile['debug_assertions'] is True and profile['overflow_checks'] is True,
                'actual optimized checked test executable')
        pin_exact(pins, binary['binary'])
        for suffix, arguments in (('-list', ['--list', '--format', 'terse']),
                                  ('-ignored-list', ['--ignored', '--list', '--format', 'terse']),
                                  ('', ['--test-threads=1'])):
            command, _ = pins.json(root / (name + suffix + '-command.json'))
            require(command['argv'] == [artifact['executable'], *arguments], 'actual selected binary ran')
    require(set(cpu['tests']) == set(TARGET_COUNTS), 'exact four CPU test targets')
    for name, count in TARGET_COUNTS.items():
        names = set(cpu['tests'][name]['names'])
        require(len(names) == len(cpu['tests'][name]['names']) == count, 'exact CPU test inventory size')
        if name in values['prior_cpu']['tests']:
            require(names == set(values['prior_cpu']['tests'][name]['names']), 'preserved Down2 tests')
        if name == 'mlp_silu_materialized_v1':
            require(names == set(overlay['test_census'][name]), 'all sixteen SiLU tests')
        if name == 'mlp_claimed_numerics_v1':
            require(names == CLAIMED_TESTS, 'all eight retained claimed-numerics tests')
        listed = (root / (name + '-list-stdout')).read_text()
        ignored = (root / (name + '-ignored-list-stdout')).read_text()
        require(d.check_inventory(listed, ignored, names) == names, 'actual CPU inventory, none ignored')
        result = d.results((root / (name + '-stdout')).read_text(), names, set())
        require(json.loads(json.dumps(result)) == cpu['tests'][name], 'actual CPU test outcomes')
    before, _ = pins.json(root / 'sources-before.json')
    after, _ = pins.json(root / 'sources-after.json')
    unformatted, _ = pins.json(root / 'sources-unformatted.json')
    prior_before, _ = pins.json(E / 'down2-cpu-v228-v1/sources-before.json',
                              values['prior_cpu']['raw']['sources-before.json']['sha256'])
    source_transition(before, after, unformatted, prior_before['fixture'], overlay)
    for name, record in before['fixture'].items():
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'relative fixture source')
        pin_exact(pins, dict(path=str(root / 'fixture' / name), **record))
    require(before == after and set(before['provider']) == set(provider_paths), 'unchanged CPU/provider closure')
    for record in before['provider'].values():
        pin_exact(pins, record)
    inputs, _ = pins.json(root / 'inputs.json')
    require(list(inputs.values()) == cpu['input_pins'] or sorted(inputs.values(), key=lambda row: row['path'])
            == sorted(cpu['input_pins'], key=lambda row: row['path']), 'CPU input pin ledger')
    for path, record in inputs.items():
        require(path == record['path'], 'CPU input identity')
        pin_exact(pins, record)
    for row in overlay['files']:
        require(row['source'] == 'draft/' + row['path'] and row['before'] is None, 'additive source proposal')
        original = pins.pin(SOURCE / row['source'], row['after']['sha256'])
        require({key: original[key] for key in ('bytes', 'sha256')} == row['after'], 'authored source identity')
    require(set(cpu['formatted_sources']) == {row['path'] for row in overlay['files']}, 'three formatted CPU sources')
    for name, record in cpu['formatted_sources'].items():
        require(record['path'] == str(root / 'fixture' / name.removeprefix(DEVICE + '/')), 'formatted source stays in CPU fixture')
        require({key: record[key] for key in ('bytes', 'sha256')}
                == before['fixture'][name.removeprefix(DEVICE + '/')], 'formatted source was tested unchanged')
        pin_exact(pins, record)
    baseline = values['baseline']
    require(baseline['schema'] == 'ferric-p225-source-handoff-v1' and baseline['passed'] is True
            and baseline['pointer_roots'] == 11 and baseline['static_lds_bytes'] == 512
            and baseline['explicit_argument_bytes'] == 88 and baseline['executable_kernarg_bytes'] == 344,
            'original checked MLP source/ABI baseline')
    prior = values['prior_lowering']
    require(prior['schema'] == 'ferric-p228-down2-lowering-result-v1' and prior['passed'] is True
            and prior['error'] is None and prior['postcheck_errors'] == []
            and tuple(row['name'] for row in prior['commands']) == STAGES
            and all(prior[key] is False for key in FALSE_FLAGS)
            and prior['candidate_cpu'] == cpu['prior_cpu'], 'actual checked Down2 generation')
    require(set(cpu['lowering_sources']) == RUST_FILES, 'five CPU-tested Rust lowering inputs')
    fixture = []
    for name, record in cpu['lowering_sources'].items():
        pin_exact(pins, record)
        source_name = 'src/finite_mlp_tiles_silu_materialized_v1.rs' if name == 'src/lib.rs' else name
        require(record['path'] == str(root / 'fixture' / source_name)
                and {key: record[key] for key in ('bytes', 'sha256')} == before['fixture'][source_name],
                'lowering source belongs to the unchanged tested CPU fixture')
        if name == 'src/lib.rs':
            require(record == cpu['formatted_sources'][DEVICE + '/src/finite_mlp_tiles_silu_materialized_v1.rs'],
                    'candidate entry is the CPU-tested formatted entry')
        elif name == 'src/mlp_silu_materialized_numerics_v1.rs':
            require(record == cpu['formatted_sources'][DEVICE + '/' + name], 'CPU-tested SiLU include')
        else:
            require({key: record[key] for key in ('bytes', 'sha256')} == overlay['baseline_fixture']['source_pins'][name],
                    'unchanged checked Down2 include')
        fixture.append(dict(destination=name, source=record))
    for name in ('Cargo.toml', 'Cargo.lock'):
        fixture.append(dict(destination=name, source=pins.pin(BASE / 'fixture' / name, baseline['fixture_pins'][name])))
    return fixture_contract(fixture)


def context(manifest_sha, cpu_path, cpu_sha):
    guard(sys.flags.optimize, os.environ)
    require(re.fullmatch('[0-9a-f]{64}', manifest_sha), 'frozen package manifest SHA')
    cpu_path = cpu_argument(cpu_path, cpu_sha)
    driver = load_driver()
    pins = driver.Pins()
    pins.pin(E / HELPERS['driver'][0], HELPERS['driver'][1])
    modules = {name: driver.load_module(pins, E / path, digest, 'silu_' + name)
               for name, (path, digest) in HELPERS.items() if name != 'driver'}
    p = modules['probe']
    manifest, manifest_pin = pins.json(PACKAGE / 'manifest.json', manifest_sha)
    require(manifest['schema'] == 'ferric-p228-silu-materialized-lowering-package-v1'
            and len(manifest['files']) == 4 and {row['path'] for row in manifest['files']} == FILES
            and {path.name for path in PACKAGE.iterdir()} == FILES | {'manifest.json'}, 'closed frozen package')
    for row in manifest['files']:
        require(pins.pin(PACKAGE / row['path'], row['sha256'])['bytes'] == row['bytes'], 'package member extent')
    require(Path(__file__).resolve() == PACKAGE / 'run.py', 'executed package identity')
    contracts = driver.load_module(pins, PACKAGE / 'contracts.py', pins.records[str(PACKAGE / 'contracts.py')]['sha256'], 'silu_contracts')
    values = {name: pins.json(E / path, digest)[0] for name, (path, digest) in RECEIPTS.items()}
    cpu, cpu_pin = pins.json(cpu_path, cpu_sha)
    inputs = values['inputs']
    require(set(inputs) == {'schema', 'source_maps', 'cargo_configurations'}
            and inputs['schema'] == 'ferric-p228-reciprocal-checked-probe-inputs-v7', 'retained ROOT/PIPE input record')
    merged, source_records = p.source_maps(inputs['source_maps'])
    expected = {str(R / name): digest for name, digest in merged.items()}
    configs = p.configurations(inputs['cargo_configurations'])
    providers = p.provider(merged)
    fixture = cpu_gate(p, modules['down_cpu'], cpu, values['overlay'], values, providers, pins, cpu_path)
    recipe = make_recipe(values['template'], fixture)
    generation, generation_pins, compiler_sources = p.compiler_generation(recipe)
    require(generation == values['prior_lowering']['compiler_generation'], 'unchanged qualified compiler generation')
    expected.update(compiler_sources)
    for pin in generation_pins + recipe['toolchain_and_retained_tool_pins'] + recipe['retained_modules']:
        require(pins.pin(Path(pin['path']), pin['sha256']) == pin, 'retained exact tool/source pin')
    library_dirs = (Path(generation['target']) / 'debug/deps', p.N / 'lib',
                    p.N / 'lib/rustlib/x86_64-unknown-linux-gnu/lib')
    libraries = {str(path) for directory in library_dirs for path in directory.glob('*.so*')}
    require(libraries and all(directory.is_dir() for directory in library_dirs), 'actual dynamic-library closure')
    immutable = set(expected) | set(pins.records) | libraries
    return dict(driver=driver, pins=pins, p=p, **modules, contracts=contracts, recipe=recipe,
                manifest=manifest_pin, inputs=inputs, merged=merged, source_records=source_records,
                expected=expected, configs=configs, provider=providers, generation=generation,
                compiler_sources=compiler_sources, library_dirs=library_dirs, libraries=libraries, immutable=immutable,
                cpu_pin=cpu_pin, prior_lowering=pins.records[str(PRIOR / 'complete.json')],
                source_manifest=pins.records[str(SOURCE / 'source-manifest.json')],
                old_targets=OLD_TARGETS + (cpu_path.parent / 'target',))


def body(c):
    p, recipe = c['p'], c['recipe']
    started = time.monotonic()
    require(not os.path.lexists(OUT), 'fresh compiler row')
    OUT.mkdir(mode=0o700)
    (OUT / 'tmp').mkdir(mode=0o700)
    bounded = p.module(recipe['retained_modules'][0], 'silu_bounded')
    bounded.T = OUT / 'target'
    bounded.setup()
    require(not any(bounded.T.iterdir()), 'fresh empty Cargo target')
    fixture = OUT / 'fixture'
    (fixture / 'src').mkdir(parents=True, mode=0o700)
    for row in recipe['fixture']:
        raw = p.read(row['source']['path'], row['source'], cap=4 << 20, retain=True)[2]
        with (fixture / row['destination']).open('xb') as stream:
            stream.write(raw)
    before = p.snapshot(c['immutable'], c['expected'], byte_cap=4 << 30)
    body_before = p.fixture_snapshot(fixture, recipe['fixture'])
    p.save(OUT / 'recipe.json', recipe)
    p.save(OUT / 'before.json', dict(files=before, fixture=body_before, configurations=c['configs']))
    commands, artifacts, substitutions = [], {}, {}
    dependencies, dependency_paths, error, errors = None, None, None, []
    try:
        for template in recipe['commands']:
            require(time.monotonic() - started < 10500, 'whole inner deadline')
            command = p.substitute(template, substitutions)
            command['deadline_seconds'] = min(command['deadline_seconds'], max(1, int(10500 - (time.monotonic() - started))))
            bounded.run(OUT, command['name'], command['argv'], env=command['env'], deadline=command['deadline_seconds'])
            record, streams = p.completed_leaf(OUT, command)
            commands.append(record)
            stdout, stderr = streams['stdout'][1], streams['stderr'][1]
            name = command['name']
            if name == 'fixture-metadata':
                dependency_paths = p.package_paths(p.parse(stdout), fixture)
                dependencies = p.snapshot(dependency_paths)
                p.save(OUT / 'package-sources-before.json', dependencies)
            elif name == 'checked-lowering':
                for stage in ('pre-ranked', 'neutral', 'target-before-llvm'):
                    require(('stage=' + stage + ' status=complete').encode() in stderr, 'checked stage missing')
                for basename in CAPTURES:
                    artifacts[basename] = p.read(OUT / basename, cap=16 << 20)[0]
                    require(artifacts[basename]['bytes'] > 0, 'fresh nonempty compiler capture')
                substitutions.update({'@candidate.semantic.sha256': artifacts[CAPTURES[0]]['sha256'],
                    '@candidate.handoff.sha256': artifacts[CAPTURES[3]]['sha256'],
                    '@candidate.handoff.bytes': artifacts[CAPTURES[3]]['bytes']})
            elif name in ('actual-replay', 'actual-inert-join'):
                c['contracts'].exact_test(stdout, command['argv'][2])
                if name == 'actual-replay':
                    for marker in ('stage=recipe status=complete roots=11', 'stage=ranked status=complete',
                                   'stage=formal status=complete roots=11 unresolved=8'):
                        require(marker.encode() in stdout + stderr, 'actual MLP replay marker')
            elif name == 'emit':
                caps = {'source.handoff-v3': 16 << 20, 'compiler.handoff-v2': 16 << 20,
                        'artifact.hsaco': 32 << 20, 'receipt.txt': 64 << 10}
                require({path.name for path in (OUT / 'emitted').iterdir()} == set(caps), 'closed emission roster')
                emitted = {key: p.read(OUT / 'emitted' / key, cap=cap)[0] for key, cap in caps.items()}
                require(all(pin['bytes'] > 0 for pin in emitted.values()), 'nonempty finalizer outputs')
                require(all(emitted['source.handoff-v3'][key] == artifacts[CAPTURES[3]][key]
                            for key in ('bytes', 'sha256')), 'exact captured handoff finalized')
                worker = next(pin for pin in recipe['toolchain_and_retained_tool_pins'] if pin['path'] == command['argv'][5])
                config = {Path(pin['path']).name: pin['sha256'] for pin in recipe['toolchain_and_retained_tool_pins']
                          if '/gfx950-reviewed-libs/' in pin['path']}
                c['contracts'].emission(p.read(OUT / 'emitted/receipt.txt', cap=64 << 10, retain=True)[2],
                    emitted, worker, command['argv'][8], command['argv'][9], config)
                artifacts.update({'emitted/' + key: pin for key, pin in emitted.items()})
                substitutions.update({'@candidate.image.sha256': emitted['artifact.hsaco']['sha256'],
                                      '@candidate.image.bytes': emitted['artifact.hsaco']['bytes']})
            elif name == 'extract-retained':
                require({path.name for path in (OUT / 'extracted').iterdir()} == {'formal.archive', 'module.ll'}, 'exact extraction roster')
                for basename in ('formal.archive', 'module.ll'):
                    artifacts['extracted/' + basename] = p.read(OUT / 'extracted' / basename, cap=16 << 20)[0]
                p.read(OUT / 'extracted/module.ll', cap=16 << 20, retain=True)[2].decode('utf-8')
                require(b'authority none' in stdout and b'gpu_execution false' in stdout, 'inert extraction')
            elif name == 'descriptor-metadata':
                c['contracts'].metadata(stdout, artifacts['emitted/artifact.hsaco'])
            elif name == 'elf-notes':
                for marker in (b'.group_segment_fixed_size: 512', b'.private_segment_fixed_size: 0', b'.wavefront_size: 64'):
                    require(marker in stdout, 'unchanged launch resource requirement: ' + marker.decode())
            elif name == 'disassembly':
                require(SYMBOL.encode() in stdout and b's_endpgm' in stdout, 'actual MLP symbol and ISA')
            require(p.fixture_snapshot(fixture, recipe['fixture']) == body_before, 'candidate source changed')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        checks = [('inputs', None, c['pins'].recheck),
                  ('source_maps', (c['merged'], c['source_records']), lambda: p.source_maps(c['inputs']['source_maps'])),
                  ('files', before, lambda: p.snapshot(c['immutable'], byte_cap=4 << 30)),
                  ('fixture', body_before, lambda: p.fixture_snapshot(fixture, recipe['fixture'])),
                  ('configurations', c['configs'], lambda: p.configurations(c['inputs']['cargo_configurations'])),
                  ('provider', c['provider'], lambda: p.provider(c['merged'])),
                  ('libraries', sorted(c['libraries']), lambda: sorted(str(path) for directory in c['library_dirs'] for path in directory.glob('*.so*')))]
        after, errors = c['outer'].postchecks(checks)
        p.save(OUT / 'after.json', after)
        if dependencies is not None:
            try:
                actual_paths = p.package_paths(p.parse(p.read(OUT / 'fixture-metadata-stdout', cap=64 << 20, retain=True)[2]), fixture)
                require(actual_paths == dependency_paths, 'dependency/sysroot roster changed')
                actual = p.snapshot(actual_paths)
                p.save(OUT / 'package-sources-after.json', actual)
                require(actual == dependencies, 'dependency/sysroot source changed')
            except BaseException as failure:
                errors.append('dependencies: ' + repr(failure))
        try:
            for pin in artifacts.values():
                p.read(pin['path'], pin, cap=32 << 20)
        except BaseException as failure:
            errors.append('artifacts: ' + repr(failure))
    passed = error is None and not errors and tuple(row['name'] for row in commands) == STAGES
    result = dict(schema='ferric-p228-silu-materialized-lowering-result-v1', passed=passed, error=error,
        postcheck_errors=errors, package_manifest=c['manifest'], candidate_cpu=c['cpu_pin'],
        source_manifest=c['source_manifest'], prior_lowering=c['prior_lowering'],
        compiler_generation=c['generation'], commands=commands, artifacts=artifacts,
        fresh_checked_lowering=passed, fresh_checked_replay=passed, fresh_hsaco_emitted=passed,
        frontend_recipe_is_diagnostic=True, unresolved_runtime_requirements=8 if passed else None,
        elapsed_host_seconds=time.monotonic() - started, **{key: False for key in FALSE_FLAGS})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), result)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def run(manifest_sha, cpu_path, cpu_sha, child=False):
    guard(sys.flags.optimize, os.environ)
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'fixed CPU owner/envelope')
    limits = {}
    for name, kind, cap in (('address_space_bytes', resource.RLIMIT_AS, 12 << 30),
                           ('file_cap_bytes', resource.RLIMIT_FSIZE, 1 << 30), ('core_bytes', resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        actual = min([cap] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (actual, actual))
        limits[name] = actual
    os.umask(0o077)
    c = context(manifest_sha, cpu_path, cpu_sha)
    if child:
        return body(c)
    require(not os.path.lexists(OUT) and not os.path.lexists(OWNER), 'fresh owner and candidate output')
    p, outer, pins = c['p'], c['outer'], c['pins']
    before = dict(files=p.snapshot(c['immutable'], c['expected'], byte_cap=4 << 30),
                  old_targets=outer.inventory(c['inventory'], c['old_targets']),
                  compiler_source_roster=p.tree(Path(c['generation']['source'])))
    env = dict(c['recipe']['commands'][0]['env'])
    require(not any(key.startswith('PYTHON') for key in env) and all(env[key] == '' for key in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'closed CPU child environment')
    argv = ['/usr/bin/python3', '-B', str(PACKAGE / 'run.py'), manifest_sha, str(cpu_path), cpu_sha, '--child']
    OWNER.mkdir(mode=0o700)
    p.save(OWNER / 'before.json', before)
    p.save(OWNER / 'command.json', dict(argv=argv, env=env, cwd=str(p.F), deadline_seconds=10800,
                                      affinity=[8, 9], nice=10, gpu_execution=False, **limits))
    outcome, completion, error = None, None, None
    try:
        outcome = c['driver'].run_coordinator(OWNER, argv, p.F, env, c['owned'], p.save, deadline=10800)
        p.save(OWNER / 'owned-result.json', outcome)
        outer.check_outcome(outcome)
        value, completion = pins.json(OUT / 'complete.json')
        require(value['schema'] == 'ferric-p228-silu-materialized-lowering-result-v1' and value['passed'] is True
                and value['error'] is None and value['postcheck_errors'] == []
                and value['package_manifest'] == c['manifest'] and value['candidate_cpu'] == c['cpu_pin']
                and value['compiler_generation'] == c['generation']
                and value['source_manifest'] == c['source_manifest'] and value['prior_lowering'] == c['prior_lowering']
                and tuple(row['name'] for row in value['commands']) == STAGES
                and all(value[key] is False for key in FALSE_FLAGS), 'exact successful child completion')
        require(all(value[key] is True for key in ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted'))
                and value['frontend_recipe_is_diagnostic'] is True and value['unresolved_runtime_requirements'] == 8,
                'fresh checked stages without runtime authority')
        outer.retain_completion(pins, value, OUT)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    after, errors = outer.postchecks([
        ('pins', None, pins.recheck), ('files', before['files'], lambda: p.snapshot(c['immutable'], byte_cap=4 << 30)),
        ('source_maps', (c['merged'], c['source_records']), lambda: p.source_maps(c['inputs']['source_maps'])),
        ('old_targets', before['old_targets'], lambda: outer.inventory(c['inventory'], c['old_targets'])),
        ('compiler_source_roster', before['compiler_source_roster'], lambda: p.tree(Path(c['generation']['source']))),
        ('configuration', c['configs'], lambda: p.configurations(c['inputs']['cargo_configurations']))])
    p.save(OWNER / 'after.json', after)
    passed = error is None and not errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-silu-materialized-lowering-owned-result-v1', passed=passed, error=error,
        postcheck_errors=errors, owned=outcome, completion=completion,
        package_manifest=c['manifest'], candidate_cpu=c['cpu_pin'], compiler_generation=c['generation'],
        source_manifest=c['source_manifest'], prior_lowering=c['prior_lowering'],
        raw={path.name: c['driver'].read_file(path)[0] for path in OWNER.iterdir() if path.is_file()},
        **{key: False for key in FALSE_FLAGS}))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    def terminate(_signal, _frame):
        raise KeyboardInterrupt('owned SiLU-materialized compiler probe terminated')
    signal.signal(signal.SIGTERM, terminate)
    require(len(sys.argv) in (4, 5) and (len(sys.argv) == 4 or sys.argv[4] == '--child'),
            'MANIFEST_SHA ACTUAL_CPU_COMPLETE_PATH ACTUAL_CPU_SHA [--child]')
    sys.exit(run(sys.argv[1], sys.argv[2], sys.argv[3], len(sys.argv) == 5))
