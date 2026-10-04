"""One owned Down2 compiler probe using the preserved checked tool generation."""
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
PACKAGE = E / 'p228-down2-lowering-v2'
OUT = E / 'row-down2-checked-probe-v228-v1'
OWNER = E / 'down2-checked-probe-owner-v228-v1'
CPU = E / 'down2-cpu-v228-v1'
SOURCE = E / 'p228-mlp-down-two-row-source-v2'
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
    'cpu': ('down2-cpu-v228-v1/complete.json',
            '96ef991246b90f9f02b509b3302df0215309bb172f9cae4340c21eba27bda563'),
    'overlay': ('p228-mlp-down-two-row-source-v2/source-manifest.json',
                '26ac584bc95b4455d5a5d5a3d9503d9aa3de9250a5d22bba42ed8a5a6d811691'),
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
    module = types.ModuleType('down2_retained_driver')
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
    result.update(schema='ferric-p228-down2-lowering-recipe-v1', fresh_output=str(OUT),
                  fresh_target=str(OUT / 'target'), fixture=fixture, commands=commands, symbol=SYMBOL)
    require(all(row['cache_cap_bytes'] == 6 << 30 and row['affinity'] == [8, 9]
                and row['nice'] == 10 and row['expected_exit'] == 0 and row['gpu_execution'] is False
                and row['env']['CARGO_TARGET_DIR'] == result['fresh_target'] for row in commands), 'unchanged leaf bounds')
    return result


def cpu_gate(p, d, cpu, overlay, baseline, provider_paths, pins):
    require(cpu['schema'] == 'ferric-p228-down2-cpu-result-v1' and cpu['passed'] is True
            and cpu['error'] is None and cpu['postcheck_error'] is None and cpu['source_unchanged'] is True
            and cpu['tests_passed'] == 14 and cpu['tests_ignored'] == 0 and cpu['cpu_arithmetic_only'] is True,
            'actual CPU14 arithmetic/source qualification')
    require(all(cpu[key] is False for key in ('gpu_execution', 'compiler_hsaco_reproduced',
                'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority')),
            'CPU receipt grants no downstream acceptance')
    require(cpu['actual_provider_source_sha256'] == p.PROVIDER_SHA, 'same actual provider53')
    require(cpu['overlay'] == pins.records[str(SOURCE / 'source-manifest.json')], 'actual CPU overlay identity')
    changes, expected_tests = d.proposal(overlay)
    for name, record in cpu['raw'].items():
        require(record['path'] == str(CPU / name) and pins.pin(CPU / name, record['sha256']) == record, 'actual CPU raw bytes')
    require(len(cpu['phases']) == 8 and len(cpu['raw']) == 45, 'CPU cohort raw census')
    for name, value in cpu['phases'].items():
        actual, _ = pins.json(CPU / (name + '-result.json'))
        require(actual == value and value['exit_code'] == 0 and value['reason'] is None
                and value['group_absent'] is True, 'natural CPU phase outcome')
        require(all(value[key + '_sha256'] == cpu['raw'][name + '-' + key]['sha256']
                    for key in ('stdout', 'stderr')), 'CPU phase streams')
    require(set(cpu['tests']) == set(expected_tests), 'exact CPU test targets')
    for name, names in expected_tests.items():
        listed = (CPU / (name + '-list-stdout')).read_text()
        ignored = (CPU / (name + '-ignored-list-stdout')).read_text()
        require(d.check_inventory(listed, ignored, names) == names, 'actual CPU test inventory')
        result = d.results((CPU / (name + '-stdout')).read_text(), names, set())
        require(json.loads(json.dumps(result)) == cpu['tests'][name], 'actual CPU test outcomes')
    before, _ = pins.json(CPU / 'sources-before.json')
    after, _ = pins.json(CPU / 'sources-after.json')
    require(before == after and set(before['provider']) == set(provider_paths), 'unchanged CPU/provider source closure')
    for path, record in before['provider'].items():
        require(pins.pin(Path(path), record['sha256']) == record, 'CPU provider bytes remain selected')
    require(baseline['schema'] == 'ferric-p225-source-handoff-v1' and baseline['passed'] is True
            and baseline['pointer_roots'] == 11 and baseline['static_lds_bytes'] == 512
            and baseline['explicit_argument_bytes'] == 88 and baseline['executable_kernarg_bytes'] == 344,
            'original checked MLP source/ABI baseline')
    fixture = []
    replacements = {row['path'].split('/src/')[-1]: row for row in changes if '/src/' in row['path']}
    for name, digest in baseline['fixture_pins'].items():
        original = BASE / 'fixture' / name
        pins.pin(original, digest)
        if name == 'src/lib.rs':
            row = replacements['finite_mlp_tiles_v2.rs']
        elif name == 'src/mlp_tile_numerics_v2.rs':
            row = replacements['mlp_tile_numerics_v2.rs']
        else:
            row = None
        if row is not None:
            require(row['before']['sha256'] == digest, 'candidate replaces exact checked preimage')
            path = SOURCE / row['source']
            pin = pins.pin(path, row['after']['sha256'])
            require({key: pin[key] for key in ('bytes', 'sha256')} == row['after'], 'candidate source extent')
            cpu_name = 'src/' + row['path'].split('/src/')[-1]
            require(before['fixture'][cpu_name] == row['after'], 'exact CPU-tested candidate source')
        else:
            pin = pins.records[str(original)]
            if name.startswith('src/'):
                require(before['fixture'][name] == {key: pin[key] for key in ('bytes', 'sha256')}, 'unchanged numerical include')
        fixture.append(dict(destination=name, source=pin))
    require({row['destination'] for row in fixture} == {'Cargo.toml', 'Cargo.lock', 'src/lib.rs',
            'src/mlp_numerics_v1.rs', 'src/mlp_tile_numerics_v2.rs', 'src/wave_numerics_v1.rs'}, 'closed four-Rust-file fixture')
    return sorted(fixture, key=lambda row: row['destination'])


def context(manifest_sha):
    guard(sys.flags.optimize, os.environ)
    require(re.fullmatch('[0-9a-f]{64}', manifest_sha), 'frozen package manifest SHA')
    driver = load_driver()
    pins = driver.Pins()
    pins.pin(E / HELPERS['driver'][0], HELPERS['driver'][1])
    modules = {name: driver.load_module(pins, E / path, digest, 'down2_' + name)
               for name, (path, digest) in HELPERS.items() if name != 'driver'}
    p = modules['probe']
    manifest, manifest_pin = pins.json(PACKAGE / 'manifest.json', manifest_sha)
    require(manifest['schema'] == 'ferric-p228-down2-lowering-package-v2'
            and len(manifest['files']) == 4 and {row['path'] for row in manifest['files']} == FILES
            and {path.name for path in PACKAGE.iterdir()} == FILES | {'manifest.json'}, 'closed frozen package')
    for row in manifest['files']:
        require(pins.pin(PACKAGE / row['path'], row['sha256'])['bytes'] == row['bytes'], 'package member extent')
    require(Path(__file__).resolve() == PACKAGE / 'run.py', 'executed package identity')
    contracts = driver.load_module(pins, PACKAGE / 'contracts.py', pins.records[str(PACKAGE / 'contracts.py')]['sha256'], 'down2_contracts')
    values = {name: pins.json(E / path, digest)[0] for name, (path, digest) in RECEIPTS.items()}
    inputs = values['inputs']
    require(set(inputs) == {'schema', 'source_maps', 'cargo_configurations'}
            and inputs['schema'] == 'ferric-p228-reciprocal-checked-probe-inputs-v7', 'retained ROOT/PIPE input record')
    merged, source_records = p.source_maps(inputs['source_maps'])
    expected = {str(R / name): digest for name, digest in merged.items()}
    configs = p.configurations(inputs['cargo_configurations'])
    providers = p.provider(merged)
    fixture = cpu_gate(p, modules['down_cpu'], values['cpu'], values['overlay'], values['baseline'], providers, pins)
    recipe = make_recipe(values['template'], fixture)
    generation, generation_pins, compiler_sources = p.compiler_generation(recipe)
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
                cpu_pin=pins.records[str(CPU / 'complete.json')])


def body(c):
    p, recipe = c['p'], c['recipe']
    started = time.monotonic()
    require(not os.path.lexists(OUT), 'fresh compiler row')
    OUT.mkdir(mode=0o700)
    (OUT / 'tmp').mkdir(mode=0o700)
    bounded = p.module(recipe['retained_modules'][0], 'down2_bounded')
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
    result = dict(schema='ferric-p228-down2-lowering-result-v1', passed=passed, error=error,
        postcheck_errors=errors, package_manifest=c['manifest'], candidate_cpu=c['cpu_pin'],
        compiler_generation=c['generation'], commands=commands, artifacts=artifacts,
        fresh_checked_lowering=passed, fresh_checked_replay=passed, fresh_hsaco_emitted=passed,
        frontend_recipe_is_diagnostic=True, unresolved_runtime_requirements=8 if passed else None,
        elapsed_host_seconds=time.monotonic() - started, **{key: False for key in FALSE_FLAGS})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), result)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def run(manifest_sha, child=False):
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
    c = context(manifest_sha)
    if child:
        return body(c)
    require(not os.path.lexists(OUT) and not os.path.lexists(OWNER), 'fresh owner and candidate output')
    p, outer, pins = c['p'], c['outer'], c['pins']
    before = dict(files=p.snapshot(c['immutable'], c['expected'], byte_cap=4 << 30),
                  old_targets=outer.inventory(c['inventory'], OLD_TARGETS),
                  compiler_source_roster=p.tree(Path(c['generation']['source'])))
    env = dict(c['recipe']['commands'][0]['env'])
    require(not any(key.startswith('PYTHON') for key in env) and all(env[key] == '' for key in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'closed CPU child environment')
    argv = ['/usr/bin/python3', '-B', str(PACKAGE / 'run.py'), manifest_sha, '--child']
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
        require(value['schema'] == 'ferric-p228-down2-lowering-result-v1' and value['passed'] is True
                and value['error'] is None and value['postcheck_errors'] == []
                and value['package_manifest'] == c['manifest'] and value['candidate_cpu'] == c['cpu_pin']
                and value['compiler_generation'] == c['generation']
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
        ('old_targets', before['old_targets'], lambda: outer.inventory(c['inventory'], OLD_TARGETS)),
        ('compiler_source_roster', before['compiler_source_roster'], lambda: p.tree(Path(c['generation']['source']))),
        ('configuration', c['configs'], lambda: p.configurations(c['inputs']['cargo_configurations']))])
    p.save(OWNER / 'after.json', after)
    passed = error is None and not errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-down2-lowering-owned-result-v1', passed=passed, error=error,
        postcheck_errors=errors, owned=outcome, completion=completion,
        package_manifest=c['manifest'], candidate_cpu=c['cpu_pin'], compiler_generation=c['generation'],
        raw={path.name: c['driver'].read_file(path)[0] for path in OWNER.iterdir() if path.is_file()},
        **{key: False for key in FALSE_FLAGS}))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    def terminate(_signal, _frame):
        raise KeyboardInterrupt('owned Down2 compiler probe terminated')
    signal.signal(signal.SIGTERM, terminate)
    require(len(sys.argv) in (2, 3) and (len(sys.argv) == 2 or sys.argv[2] == '--child'), 'MANIFEST_SHA [--child]')
    sys.exit(run(sys.argv[1], len(sys.argv) == 3))
