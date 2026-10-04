"""One owned RoPE probe with the qualified RPO generation and unchanged V7 replay gates."""
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
PACKAGE = E / 'p228-rope-materialized-rpo-lowering-v1'
OUT = E / 'row-rope-materialized-rpo-checked-probe-v228-v1'
OWNER = E / 'rope-materialized-rpo-checked-probe-owner-v228-v1'
SOURCE = E / 'p228-rope-materialized-source-v2'
PRIOR = E / 'row-reciprocal-checked-probe-v228-v7'
SYMBOL = 'ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6'
CAPTURES = ('prefix-tiles-semantic.bin', 'prefix-tiles-neutral-kir.bin', 'prefix-tiles-target-kir.bin', 'prefix-tiles.handoff-v3')
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
    'rpo_cpu': ('p228-partial-move-rpo-cpu-v2/run.py',
                '6cb1dc984761bfb44bf03246c08623faba225433cad70d745a606893ab5125ba'),
    'rpo_tools': ('p228-partial-move-rpo-finalizer-tools-v2/run.py',
                  'cd5fb1f8254e3eb3a2c8df4881432f3c73fe1bb18183cee73e7a0f3c8ad30b25'),
}
RPO_CPU = E / 'rpo-compiler-cpu-v228-v2'
RPO_TOOLS = E / 'rpo-finalizer-tools-v228-v2'
RPO_CPU_PIN = dict(path=str(RPO_CPU / 'complete.json'), bytes=375781,
    sha256='56fc51fc326980e00156d550d0a7052f44bb481ded7b9948c06217653fb246c1')
RPO_OWNER_PIN = dict(path=str(E / 'rpo-compiler-cpu-owner-v228-v2/complete.json'), bytes=57128,
    sha256='afca99d8799f910ce607873c320b8f244be66f9ffdb3df4dce4545eb1552ea66')
TOOLS_PACKAGE = dict(path=str(E / 'p228-partial-move-rpo-finalizer-tools-v2/manifest.json'), bytes=1693,
    sha256='d014c66154afefa324bb0f5a49bacfb034229e840efb012acd02c950d5ce0178')
RECEIPTS = {
    'template': ('p228-reciprocal-checked-probe-v7/recipe.json',
                 'f500b1c3d3218b53e9ba1df92d6f048a41f05873c9539f16bc69b20f85575a52'),
    'inputs': ('reciprocal-checked-probe-inputs-v228-v7.json',
               '08a85adc9d529ad91c019da4a428ccf69dddb10623d43e4ea8242f7390fec879'),
    'prior_cpu': ('reciprocal-cpu-v228-v3/complete.json',
                  '6b02d358eca33d31d6839e9c28de38276b4f968cfaec93b7ddb97963a77956d3'),
    'overlay': ('p228-rope-materialized-source-v2/source-manifest.json',
                '7f1f447852f01bad9b6dedb9b40a7969961d45861592465548e6401615318d23'),
    'prior_lowering': ('row-reciprocal-checked-probe-v228-v7/complete.json',
                       '6ba25826b71e30f5106e1efc9e786c021c56f5bab397add20b365373685081a9'),
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
    module = types.ModuleType('rope_retained_driver')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def rewrite(value, old, new):
    if isinstance(value, str):
        if value == str(old):
            return str(new)
        if value.startswith(str(old) + '/'):
            return str(new) + value[len(str(old)):]
        return value
    if isinstance(value, list):
        return [rewrite(item, old, new) for item in value]
    if isinstance(value, dict):
        return {key: rewrite(item, old, new) for key, item in value.items()}
    return value


def make_recipe(template, fixture):
    require(tuple(row['name'] for row in template['commands']) == STAGES, 'retained nine-stage template')
    commands = rewrite(copy.deepcopy(template['commands']), template['fresh_output'], OUT)
    require(template['symbol'] == SYMBOL, 'retained exact prefix symbol')
    result = {key: copy.deepcopy(template[key]) for key in
              ('toolchain_and_retained_tool_pins', 'compiler_generation', 'retained_modules')}
    result.update(schema='ferric-p228-rope-materialized-rpo-lowering-recipe-v1', fresh_output=str(OUT),
                  fresh_target=str(OUT / 'target'), fixture=fixture, commands=commands, symbol=SYMBOL)
    require(all(row['cache_cap_bytes'] == 6 << 30 and row['affinity'] == [8, 9]
                and row['nice'] == 10 and row['expected_exit'] == 0 and row['gpu_execution'] is False
                and row['env']['CARGO_TARGET_DIR'] == result['fresh_target'] for row in commands), 'unchanged leaf bounds')
    return result


def generation_recipe(recipe, prior, current):
    result = copy.deepcopy(recipe)
    require(recipe['compiler_generation'] == prior['prerequisites'], 'original compiler prerequisite map')
    old, new = prior['roles'], current['roles']
    require(set(old) == set(new) == {'compiler-tests', 'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so',
            'finalizer', 'finalizer-tests', 'metadata'}, 'exact six generation roles')
    replacements = {pin['path']: new[name] for name, pin in old.items()}
    retained = recipe['toolchain_and_retained_tool_pins']
    require(len(replacements) == 6 and len({pin['path'] for pin in retained}) == len(retained)
            and all(pin in retained for pin in old.values()), 'exact prior tool role identities')
    result['toolchain_and_retained_tool_pins'] = [replacements.get(pin['path'], pin) for pin in retained]
    result['compiler_generation'] = current['prerequisites']
    commands = {row['name']: row for row in result['commands']}
    require(commands['checked-lowering']['env']['RUSTC_WORKSPACE_WRAPPER'] == old['fe2o3-rustc-extract']['path'],
            'original checked wrapper')
    commands['checked-lowering']['env']['RUSTC_WORKSPACE_WRAPPER'] = new['fe2o3-rustc-extract']['path']
    for phase, role in (('actual-replay', 'compiler-tests'), ('actual-inert-join', 'finalizer-tests'),
                        ('emit', 'finalizer'), ('descriptor-metadata', 'metadata')):
        require(commands[phase]['argv'][0] == old[role]['path'], 'original selected tool: ' + phase)
        commands[phase]['argv'][0] = new[role]['path']
    old_lib = str(Path(prior['target']) / 'debug/deps')
    new_lib = str(Path(current['target']) / 'debug/deps')
    for row in result['commands']:
        parts = row['env']['LD_LIBRARY_PATH'].split(':')
        require(len(parts) == 2 and parts[0] == old_lib, 'original compiler library search path')
        row['env']['LD_LIBRARY_PATH'] = new_lib + ':' + parts[1]
    require(len({pin['path'] for pin in result['toolchain_and_retained_tool_pins']}) == len(retained),
            'no generation tool alias collision')
    return result


def tools_gate(cpu, tools, owner, prerequisites, finalizer):
    require(tools['schema'] == 'fe2o3-p228-rpo-finalizer-tools-result-v1'
            and owner['schema'] == 'fe2o3-p228-rpo-finalizer-tools-owned-result-v1', 'RPO finalizer schemas')
    for value in (tools, owner):
        require(value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
                and value['gpu_execution'] is value['production_authority'] is False, 'successful CPU-only finalizer')
        require(value['compiler_cpu'] == prerequisites['cpu'] and value['compiler_owner'] == prerequisites['cpu_owner']
                and value['patches'] == finalizer.PATCHES
                and value['qualified_generation'] == cpu['qualified_generation'], 'same RPO generation')
    require(owner['completion'] == prerequisites['tools'] and tools['package'] == TOOLS_PACKAGE
            and tools['compiler_artifacts'] == cpu['artifacts']
            and tools['source_snapshot'] == cpu['raw']['sources-before.json']
            and tools['compiler_required_test_names'] == cpu['required_test_names']
            and tools['prior_finalizer'] == finalizer.PRIOR_PIN, 'exact qualified finalizer lineage')
    require(set(tools['phases']) == finalizer.TOOL_PHASES and all(
            type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
            and row['group_absent'] is True for row in tools['phases'].values()), 'six natural finalizer phases')
    require(set(tools['artifacts']) == {'finalizer', 'finalizer-tests', 'metadata'}
            and all(value[key] is False for value in (tools,) for key in (
                'checked_lowering', 'actual_capture_join', 'fresh_hsaco_emitted', 'numerical_acceptance',
                'performance_claim', 'full_model_acceptance')), 'tools do not qualify actual capture or downstream use')
    require(all(Path(pin['path']).is_relative_to(RPO_CPU / 'target')
                for pin in (*cpu['artifacts'].values(), *tools['artifacts'].values())), 'one actual RPO target')


def replay_generation_raw(pins, root, value):
    for name, pin in value['raw'].items():
        require(Path(name).name == name and pin['path'] == str(root / name), 'generation raw namespace')
        pin_exact(pins, pin)
    for name, row in value['phases'].items():
        actual, _ = pins.json(root / (name + '-result.json'))
        require(actual == row and all(row[key + '_sha256'] == value['raw'][name + '-' + key]['sha256']
                    for key in ('stdout', 'stderr')), 'generation phase/stream identity')


def rpo_generation(pins, modules, template, previous, tools_sha, owner_sha):
    require(all(type(sha) is str and re.fullmatch('[0-9a-f]{64}', sha) for sha in (tools_sha, owner_sha)),
            'actual finalizer completion and owner hashes required')
    p, base, finalizer, outer = (modules[name] for name in ('probe', 'rpo_cpu', 'rpo_tools', 'outer'))
    predecessor, old_pins, old_sources = p.compiler_generation(template)
    require(predecessor == previous['compiler_generation'], 'actual ordinary V7 compiler predecessor')
    cpu, _ = pins.json(RPO_CPU / 'complete.json', RPO_CPU_PIN['sha256'])
    owner, _ = pins.json(Path(RPO_OWNER_PIN['path']), RPO_OWNER_PIN['sha256'])
    pin_exact(pins, RPO_CPU_PIN); pin_exact(pins, RPO_OWNER_PIN)
    tools, tools_pin = pins.json(RPO_TOOLS / 'complete.json', tools_sha)
    tools_owner, owner_pin = pins.json(E / 'rpo-finalizer-tools-owner-v228-v2/complete.json', owner_sha)
    prerequisites = dict(cpu=RPO_CPU_PIN, cpu_owner=RPO_OWNER_PIN, tools=tools_pin, tools_owner=owner_pin)
    finalizer.CPU_PIN, finalizer.OWNER_PIN = RPO_CPU_PIN, RPO_OWNER_PIN
    finalizer.validate_cpu(cpu, owner)
    require(cpu['source_unchanged'] is True
            and cpu['qualified_generation']['completion'] == predecessor['prerequisites']['cpu']
            and cpu['qualified_generation']['owner'] == predecessor['prerequisites']['cpu_owner'], 'RPO exact predecessor')
    outer.check_outcome(owner['owned']); outer.check_outcome(tools_owner['owned'])
    tools_gate(cpu, tools, tools_owner, prerequisites, finalizer)
    for root, value in ((RPO_CPU, cpu), (RPO_TOOLS, tools)):
        replay_generation_raw(pins, root, value)
    read_text = lambda pin: p.read(pin['path'], pin, cap=64 << 20, retain=True)[2].decode()
    finalizer.replay_cpu_tests(base, cpu, read_text)
    old_cpu, _ = pins.json(Path(base.QUALIFIED_COMPLETE['path']), base.QUALIFIED_COMPLETE['sha256'])
    proposal, _ = pins.json(Path(finalizer.PATCHES['rpo']['path']), finalizer.PATCHES['rpo']['sha256'])
    for short in ('pliron', 'compiler'):
        base.required_tests(short, cpu['tests'][short]['names'], cpu['tests'][short]['ignored_names'], old_cpu, proposal)
    prior_tests, prior_pins = finalizer.replay_finalizer(base, p)
    names = base.inventory(read_text(tools['raw']['finalizer-list-stdout']))
    ignored = sorted(re.findall(r'^([^\r\n]+): test$', read_text(tools['raw']['finalizer-ignored-list-stdout']), re.M))
    finalizer.finalizer_inventory(names, ignored, prior_tests)
    require(base.test_results(read_text(tools['raw']['finalizer-tests-stdout']), names, ignored) == tools['tests'],
            'actual finalizer named outcomes')
    roles = {name: cpu['artifacts'][name] for name in ('compiler-tests', 'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so')}
    roles.update(tools['artifacts'])
    joined = dict(source=str(RPO_CPU / 'source/fe2o3'), target=str(RPO_CPU / 'target'), roles=roles,
        prerequisites=prerequisites, compiler_products=cpu['artifacts'], patches=finalizer.PATCHES,
        qualified_generation=cpu['qualified_generation'], predecessor=predecessor,
        compiler_package=finalizer.CPU_PACKAGE, finalizer_package=TOOLS_PACKAGE)
    joined['backend_dynamic_library'] = p.backend_library_pin(joined)
    source_pins = [cpu['raw']['sources-before.json'], cpu['raw']['sources-after.json'], tools['raw']['sources-after.json']]
    expected = p.compiler_source_snapshot(Path(joined['source']), source_pins)
    require(not set(expected) & set(old_sources), 'separate original and RPO compiler source copies')
    expected.update(old_sources)
    records = [*old_pins, *prior_pins, *prerequisites.values(), *source_pins, *roles.values(),
               *cpu['artifacts'].values(), *finalizer.PATCHES.values(), *finalizer.QUALIFIED_GENERATION.values(),
               finalizer.CPU_PACKAGE, TOOLS_PACKAGE, joined['backend_dynamic_library']]
    for record in records:
        pin_exact(pins, record)
    return joined, records, expected


BASE_RUST = {'src/lib.rs', 'src/wave_numerics_v1.rs', 'src/head_rope_numerics_v3.rs',
             'src/prefix_reciprocal_numerics_v1.rs', 'src/attention_online.rs',
             'src/output_projection_numerics_v5.rs', 'src/prefix_tiles_numerics_v6.rs'}
RUST_FILES = BASE_RUST | {'src/prefix_rope_materialized_numerics_v1.rs'}
FIXTURE_FILES = RUST_FILES | {'Cargo.toml', 'Cargo.lock'}
CHANGES = {'src/lib.rs', 'src/prefix_rope_materialized_numerics_v1.rs',
           'tests/rope_materialized_v1.rs', 'tests/fixtures/v7_lib.rs'}
FORMATTED = CHANGES - {'tests/fixtures/v7_lib.rs'}
TARGET_COUNTS = {'prefix_reciprocal_v1': 13, 'rope_materialized_v1': 20}
EXHAUSTIVE = 'exhaustive_all_significands_in_one_binade'
OLD_TEST_INPUTS = {'tests/prefix_reciprocal_v1.rs',
                   'tests/fixtures/prefix_reciprocal_fraction_v1.tsv'}
CPU_PHASES = ('rustfmt', 'rustfmt-check', 'metadata', 'build-tests') + tuple(
    name + suffix for name in TARGET_COUNTS for suffix in ('-list', '-ignored-list', '')) + ('exhaustive',)
CPU_RAW = {name + suffix for name in CPU_PHASES
           for suffix in ('-command.json', '-started.json', '-result.json', '-stdout', '-stderr')} | {
    'sources-unformatted.json', 'sources-before.json', 'sources-after.json',
    'dependencies-before.json', 'dependencies-after.json', 'inputs.json'}
CPU_RUNNER = dict(path=str(E / 'p228-rope-materialized-cpu-v2/run.py'), bytes=22030,
                  sha256='084d135a52c4274341e2063b4e46c732db07ea2a67e69f6cae6c9e575f089dd1')


def cpu_argument(path, digest):
    path = Path(path)
    require(path.parent.parent == E and path.name == 'complete.json'
            and re.fullmatch(r'rope-materialized-cpu-v228-v[1-9][0-9]*', path.parent.name)
            and type(digest) is str and re.fullmatch('[0-9a-f]{64}', digest), 'actual CPU completion path and SHA')
    return path


def pin_exact(pins, record):
    require(set(record) == {'path', 'bytes', 'sha256'} and type(record['bytes']) is int
            and record['bytes'] >= 0 and type(record['sha256']) is str
            and re.fullmatch('[0-9a-f]{64}', record['sha256']), 'exact FilePin')
    require(pins.pin(Path(record['path']), record['sha256']) == record, 'retained exact pin')
    return record


def fixture_contract(rows):
    require(len(rows) == 10 and {row['destination'] for row in rows} == FIXTURE_FILES,
            'closed eight-Rust/ten-total fixture')
    require(len({row['source']['path'] for row in rows}) == 10, 'distinct source identities')
    return sorted(rows, key=lambda row: row['destination'])


def source_transition(before, after, unformatted, prior, overlay, harness_names):
    base = overlay['baseline_fixture']['files']
    require(set(base) == BASE_RUST | {'Cargo.toml', 'Cargo.lock'}, 'exact original V7 fixture roster')
    rows = overlay['files']
    require(len(rows) == 4 and {row['path'] for row in rows} == CHANGES, 'four RoPE overlays')
    for row in rows:
        require(row['source'] == 'draft/' + row['path'] and row['before'] == base.get(row['path']),
                'exact V7 preimage or additive absence')
    expected = {name: base[name] for name in BASE_RUST}
    expected.update({name: prior[name] for name in set(harness_names) | OLD_TEST_INPUTS})
    expected.update({row['path']: row['after'] for row in rows})
    require(len(expected) == 15 and unformatted == expected, 'exact minimal CPU fixture before formatting')
    require(before == after and set(before['fixture']) == set(expected)
            and all(before['fixture'][name] == item for name, item in expected.items() if name not in FORMATTED),
            'only three candidate bodies formatted; every body unchanged during tests')
    require(before['fixture']['tests/fixtures/v7_lib.rs'] == base['src/lib.rs'], 'exact baseline entry fixture')


def cpu_gate(p, d, cpu, overlay, values, provider_paths, pins, cpu_path):
    require(cpu['schema'] == 'ferric-p228-rope-materialized-cpu-result-v1' and cpu['passed'] is True
            and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
            and cpu['tests_passed'] == 33 and cpu['tests_ignored'] == 1 and cpu['cpu_arithmetic_only'] is True,
            'actual CPU33 arithmetic/source qualification including explicit exhaustive run')
    require(all(cpu[key] is False for key in ('gpu_execution', 'compiler_hsaco_reproduced',
                'kernel_entry_host_compiled', 'full_model_acceptance', 'numerical_acceptance',
                'performance_claim', 'production_authority')), 'CPU receipt grants no checked-entry or downstream acceptance')
    require(cpu['actual_provider_source_sha256'] == p.PROVIDER_SHA, 'same actual provider53')
    require(cpu['overlay'] == pins.records[str(SOURCE / 'source-manifest.json')]
            and cpu['prior_cpu'] == pins.records[str(E / RECEIPTS['prior_cpu'][0])]
            and cpu['prior_lowering'] == pins.records[str(PRIOR / 'complete.json')], 'exact CPU lineage')
    root = cpu_path.parent
    require(cpu['runner'] == CPU_RUNNER, 'reviewed actual CPU controller')
    pin_exact(pins, cpu['runner'])
    require(cpu['fixture'] == str(root / 'fixture'), 'isolated CPU fixture')
    require(set(cpu['phases']) == set(CPU_PHASES) and set(cpu['raw']) == CPU_RAW,
            'exact eleven phases and sixty-one raw files')
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
    require(set(cpu['binaries']) == set(TARGET_COUNTS), 'two compiled test artifacts')
    messages = [json.loads(line) for line in (root / 'build-tests-stdout').read_text().splitlines()
                if line.startswith('{')]
    require([row.get('success') for row in messages if row.get('reason') == 'build-finished'] == [True],
            'actual successful Cargo build')
    artifacts = [row for row in messages if row.get('reason') == 'compiler-artifact'
                 and row.get('target', {}).get('kind') == ['test']]
    require(len(artifacts) == 2 and {row['target']['name'] for row in artifacts} == set(TARGET_COUNTS),
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
    require(set(cpu['tests']) == set(TARGET_COUNTS) | {'exhaustive'}, 'exact CPU test cohorts')
    for name, count in TARGET_COUNTS.items():
        names = set(cpu['tests'][name]['names'])
        require(len(names) == len(cpu['tests'][name]['names']) == count, 'exact CPU test inventory size')
        ignored = {EXHAUSTIVE} if name == 'prefix_reciprocal_v1' else set()
        expected_names = (set(values['prior_cpu']['tests']['arithmetic']['names']) if ignored
                          else set(overlay['test_census'][name]))
        require(names == expected_names, 'actual named reciprocal or RoPE cohort')
        require(d.inventory((root / (name + '-list-stdout')).read_text()) == names
                and d.inventory((root / (name + '-ignored-list-stdout')).read_text(), allow_empty=True) == ignored,
                'exact actual ordinary/ignored inventory')
        result = d.results((root / (name + '-stdout')).read_text(), names, ignored)
        require(json.loads(json.dumps(result)) == cpu['tests'][name], 'actual CPU test outcomes')
    command, _ = pins.json(root / 'exhaustive-command.json')
    require(command['argv'] == [cpu['binaries']['prefix_reciprocal_v1']['binary']['path'],
            '--ignored', '--exact', EXHAUSTIVE, '--test-threads=1'], 'actual separately selected exhaustive test')
    exhaustive = d.results((root / 'exhaustive-stdout').read_text(), {EXHAUSTIVE}, set())
    require(json.loads(json.dumps(exhaustive)) == cpu['tests']['exhaustive'], 'actual exhaustive outcome')
    before, _ = pins.json(root / 'sources-before.json')
    after, _ = pins.json(root / 'sources-after.json')
    unformatted, _ = pins.json(root / 'sources-unformatted.json')
    prior_root = E / Path(RECEIPTS['prior_cpu'][0]).parent
    prior_before, _ = pins.json(prior_root / 'sources-before.json',
                              values['prior_cpu']['raw']['sources-before.json']['sha256'])
    source_transition(before, after, unformatted, prior_before['fixture'], overlay, d.HARNESS_FILES)
    for name, record in before['fixture'].items():
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'relative fixture source')
        pin_exact(pins, dict(path=str(root / 'fixture' / name), **record))
    require(before == after and set(before['provider']) == set(provider_paths), 'unchanged CPU/provider closure')
    for record in before['provider'].values():
        pin_exact(pins, record)
    dep_before, _ = pins.json(root / 'dependencies-before.json')
    dep_after, _ = pins.json(root / 'dependencies-after.json')
    require(dep_before == dep_after and dep_before, 'actual CPU dependency postchecks')
    inputs, _ = pins.json(root / 'inputs.json')
    require(sorted(inputs.values(), key=lambda row: row['path'])
            == sorted(cpu['input_pins'], key=lambda row: row['path']), 'CPU input pin ledger')
    for path, record in inputs.items():
        require(path == record['path'], 'CPU input identity')
        pin_exact(pins, record)
    require(overlay['schema'] == 'ferric-p228-rope-materialized-source-proposal-v2'
            and overlay['authored_tests'] == 20, 'frozen source proposal')
    for row in overlay['files']:
        original = pins.pin(SOURCE / row['source'], row['after']['sha256'])
        require({key: original[key] for key in ('bytes', 'sha256')} == row['after'], 'authored source identity')
    require(set(cpu['formatted_sources']) == CHANGES, 'four tested overlay bodies')
    for name, record in cpu['formatted_sources'].items():
        require(record['path'] == str(root / 'fixture' / name)
                and {key: record[key] for key in ('bytes', 'sha256')} == before['fixture'][name],
                'formatted source belongs to tested unchanged fixture')
        pin_exact(pins, record)
    prior = values['prior_lowering']
    require(prior['schema'] == 'ferric-p228-reciprocal-checked-probe-result-v7'
            and prior['probe_completed'] is True and prior['error'] is None
            and prior['postcheck_error'] is None
            and tuple(row['name'] for row in prior['commands']) == STAGES
            and all(prior[key] is False for key in FALSE_FLAGS)
            and prior['candidate_cpu_receipt'] == cpu['prior_cpu'], 'actual checked V7 generation')
    require(set(cpu['lowering_sources']) == RUST_FILES, 'eight CPU-fixture Rust lowering inputs')
    fixture = []
    for name, record in cpu['lowering_sources'].items():
        pin_exact(pins, record)
        require(record['path'] == str(root / 'fixture' / name)
                and {key: record[key] for key in ('bytes', 'sha256')} == before['fixture'][name],
                'lowering source is retained in tested unchanged fixture')
        if name in CHANGES:
            require(record == cpu['formatted_sources'][name], 'formatted candidate entry/include identity')
        else:
            require({key: record[key] for key in ('bytes', 'sha256')}
                    == overlay['baseline_fixture']['files'][name], 'unchanged selected V7 include')
        fixture.append(dict(destination=name, source=record))
    require(set(cpu['lowering_fixture_pins']) == {'Cargo.toml', 'Cargo.lock'}, 'original checked Cargo pair')
    for name, record in cpu['lowering_fixture_pins'].items():
        require(record['path'] == str(PRIOR / 'fixture' / name)
                and {key: record[key] for key in ('bytes', 'sha256')}
                == overlay['baseline_fixture']['files'][name], 'unchanged checked Cargo file')
        fixture.append(dict(destination=name, source=pin_exact(pins, record)))
    return fixture_contract(fixture)


def context(manifest_sha, cpu_path, cpu_sha, tools_sha, tools_owner_sha):
    guard(sys.flags.optimize, os.environ)
    require(re.fullmatch('[0-9a-f]{64}', manifest_sha), 'frozen package manifest SHA')
    cpu_path = cpu_argument(cpu_path, cpu_sha)
    driver = load_driver()
    pins = driver.Pins()
    pins.pin(E / HELPERS['driver'][0], HELPERS['driver'][1])
    modules = {name: driver.load_module(pins, E / path, digest, 'rope_' + name)
               for name, (path, digest) in HELPERS.items() if name != 'driver'}
    p = modules['probe']
    manifest, manifest_pin = pins.json(PACKAGE / 'manifest.json', manifest_sha)
    require(manifest['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-package-v1'
            and len(manifest['files']) == 4 and {row['path'] for row in manifest['files']} == FILES
            and {path.name for path in PACKAGE.iterdir()} == FILES | {'manifest.json'}, 'closed frozen package')
    for row in manifest['files']:
        require(pins.pin(PACKAGE / row['path'], row['sha256'])['bytes'] == row['bytes'], 'package member extent')
    require(Path(__file__).resolve() == PACKAGE / 'run.py', 'executed package identity')
    contracts = driver.load_module(pins, PACKAGE / 'contracts.py', pins.records[str(PACKAGE / 'contracts.py')]['sha256'], 'rope_contracts')
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
    generation, generation_pins, compiler_sources = rpo_generation(
        pins, modules, values['template'], values['prior_lowering'], tools_sha, tools_owner_sha)
    recipe = generation_recipe(recipe, generation['predecessor'], generation)
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
                old_targets=OLD_TARGETS + (cpu_path.parent / 'target', RPO_CPU / 'target'))


def body(c):
    p, recipe = c['p'], c['recipe']
    started = time.monotonic()
    require(not os.path.lexists(OUT), 'fresh compiler row')
    OUT.mkdir(mode=0o700)
    (OUT / 'tmp').mkdir(mode=0o700)
    bounded = p.module(recipe['retained_modules'][0], 'rope_bounded')
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
                    for marker in ('stage=recipe status=complete roots=15', 'stage=ranked status=complete',
                                   'stage=formal status=complete roots=15 unresolved=8'):
                        require(marker.encode() in stdout + stderr, 'actual prefix replay marker')
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
                require(SYMBOL.encode() in stdout and b's_endpgm' in stdout, 'actual prefix symbol and ISA')
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
    result = dict(schema='ferric-p228-rope-materialized-rpo-lowering-result-v1', passed=passed, error=error,
        postcheck_errors=errors, package_manifest=c['manifest'], candidate_cpu=c['cpu_pin'],
        source_manifest=c['source_manifest'], prior_lowering=c['prior_lowering'],
        compiler_generation=c['generation'], commands=commands, artifacts=artifacts,
        fresh_checked_lowering=passed, fresh_checked_replay=passed, fresh_hsaco_emitted=passed,
        frontend_recipe_is_diagnostic=True, unresolved_runtime_requirements=8 if passed else None,
        elapsed_host_seconds=time.monotonic() - started, **{key: False for key in FALSE_FLAGS})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), result)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def run(manifest_sha, cpu_path, cpu_sha, tools_sha, tools_owner_sha, child=False):
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
    c = context(manifest_sha, cpu_path, cpu_sha, tools_sha, tools_owner_sha)
    if child:
        return body(c)
    require(not os.path.lexists(OUT) and not os.path.lexists(OWNER), 'fresh owner and candidate output')
    p, outer, pins = c['p'], c['outer'], c['pins']
    before = dict(files=p.snapshot(c['immutable'], c['expected'], byte_cap=4 << 30),
                  old_targets=outer.inventory(c['inventory'], c['old_targets']),
                  compiler_source_roster={source: p.tree(Path(source)) for source in
                    (c['generation']['source'], c['generation']['predecessor']['source'])})
    env = dict(c['recipe']['commands'][0]['env'])
    require(not any(key.startswith('PYTHON') for key in env) and all(env[key] == '' for key in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'closed CPU child environment')
    argv = ['/usr/bin/python3', '-B', str(PACKAGE / 'run.py'), manifest_sha, str(cpu_path), cpu_sha,
            tools_sha, tools_owner_sha, '--child']
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
        require(value['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-result-v1' and value['passed'] is True
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
        ('compiler_source_roster', before['compiler_source_roster'], lambda:
            {source: p.tree(Path(source)) for source in before['compiler_source_roster']}),
        ('configuration', c['configs'], lambda: p.configurations(c['inputs']['cargo_configurations']))])
    p.save(OWNER / 'after.json', after)
    passed = error is None and not errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-rope-materialized-rpo-lowering-owned-result-v1', passed=passed, error=error,
        postcheck_errors=errors, owned=outcome, completion=completion,
        package_manifest=c['manifest'], candidate_cpu=c['cpu_pin'], compiler_generation=c['generation'],
        source_manifest=c['source_manifest'], prior_lowering=c['prior_lowering'],
        raw={path.name: c['driver'].read_file(path)[0] for path in OWNER.iterdir() if path.is_file()},
        **{key: False for key in FALSE_FLAGS}))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    def terminate(_signal, _frame):
        raise KeyboardInterrupt('owned RoPE-materialized compiler probe terminated')
    signal.signal(signal.SIGTERM, terminate)
    require(len(sys.argv) in (6, 7) and (len(sys.argv) == 6 or sys.argv[6] == '--child'),
            'MANIFEST_SHA ROPE_CPU_PATH ROPE_CPU_SHA ACTUAL_TOOLS_SHA ACTUAL_TOOLS_OWNER_SHA [--child]')
    sys.exit(run(*sys.argv[1:6], child=len(sys.argv) == 7))
