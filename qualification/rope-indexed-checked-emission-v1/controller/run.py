"""Linked checked emission: retained RPO producer, qualified indexed consumer."""
import argparse
import copy
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
PACKAGE = E / 'p228-rope-indexed-checked-emission-v1'
OUT = E / 'rope-indexed-checked-emission-v228-v1'
OWNER = E / 'rope-indexed-checked-emission-owner-v228-v1'
QUALIFIED = E / 'kir-indexed-formal-join-cpu-v228-v2'
SOURCE, TARGET = QUALIFIED / 'source/fe2o3', QUALIFIED / 'target'
QUALIFIER = E / 'p228-kir-indexed-formal-join-cpu-v2/run.py'
QUALIFIER_SHA = '02621c9ccbad3f9843e0c0250f9a3546de2c2bee8ac01b502c681c8b7576b6b6'
QUALIFIER_PACKAGE_SHA = '6420388f8444a923dd19363e658809bb369d5096ad670d683c1f7ef75e5cc349'
QUALIFIER_SOURCE_SHA = 'd9d235a2f0bb2ae185c41f38d8406124c8233d3259e53751ccf47a17755b5dc2'
QUALIFIED_SHA = '4548c7ee32206f6fc94bf54b5c50e13c500d2f4381e58092094893d9d80d4bfa'
QUALIFIED_OWNER = E / 'kir-indexed-formal-join-cpu-owner-v228-v2'
QUALIFIED_OWNER_SHA = '21205fbed2b56d5a27b4c1fbb2f4f3d5f0e810b822a518e3f875e3083553954e'
SOURCE_SHA = 'f16b193aa6a505ab2b28d98334e86b60ac5bcb6cfbd09d885cecd33d5cc27854'
TEST_ELF_SHA = 'e4d5bd3825956cd8b2f58f408fcd48c029e34fc24dc31cc727ea08cf044e8912'
PRODUCER = E / 'row-rope-materialized-rpo-checked-probe-v228-v1'
PRODUCER_OWNER = E / 'rope-materialized-rpo-checked-probe-owner-v228-v1'
PRODUCER_SHA = '1fbbbd9a8889e1b33b724b3f8911c4ecc1074fffe8fc29377f6840686375ba1f'
PRODUCER_OWNER_SHA = '8fb2f02cde719c08fd41535d59d39980d8c6328f27cf952770c481f5b48edbbf'
LOWERING = E / 'p228-rope-materialized-rpo-lowering-v1/run.py'
LOWERING_SHA = '73d869feaf08d0bf84189f4b9aca2ba56a098888d2319416cad0a13f45d05dae'
LOWERING_PACKAGE_SHA = '6ed02271fa204a9d2f00f81751425bd26860456ff753c360b079167b1d4d87b0'
CONTRACT_SHA = '694be9d199cd92b2d11ff4fe8401ad3728497bd8846b5d6924311330f3d9de1a'
ROPE_CPU = E / 'rope-materialized-cpu-v228-v3/complete.json'
ROPE_CPU_SHA = '8dcb4f2b326ab909c52039273515f44eb4f88cd91a967c8816f866d5514c864a'
TOOLS_SHA = '39eab92ede34991b08c168e827c7111c13533627cbf2efd3d248c6742a92660b'
TOOLS_OWNER_SHA = 'e919fb520672be2a909fd0ec8301bc9923210680f818e411b7db459ffc3059dd'
FINALIZER, METADATA = 'finite_join_engineering_hsaco_v1', 'finite_join_request_metadata_v1'
PRODUCER_PHASES = ('fixture-metadata', 'checked-lowering', 'actual-replay')
CONSUMER_PHASES = ('actual-inert-join', 'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
PHASES = ('metadata', 'tool-build') + CONSUMER_PHASES
FILES = {'run.py', 'contracts.py', 'test_run.py', 'README.md'}
FALSE_FLAGS = ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_compiler_built', 'full_compiler_cohort_requalified',
    'gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority',
    'launch_authority', 'runtime_requirements_discharged', 'full_model_acceptance')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load(path, sha, name):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical reviewed helper')
    raw = path.read_bytes()
    require(len(raw) < 1 << 20 and hashlib.sha256(raw).hexdigest() == sha, 'reviewed helper digest')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def qualified_gate(value, owner, pin, h):
    require(value['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-result-v2'
        and owner['schema'] == 'ferric-p228-kir-indexed-formal-join-owned-result-v2'
        and owner['completion'] == pin and value['package'] == owner['package']
        and value['package']['sha256'] == QUALIFIER_PACKAGE_SHA
        and value['proposal'] == owner['proposal'] and value['proposal']['sha256'] == QUALIFIER_SOURCE_SHA,
        'exact independently qualified consumer')
    for row in (value, owner):
        require(row['passed'] is True and row['error'] is None and row['postcheck_errors'] == []
            and row['actual_capture_join_passed'] is True and row['fresh_hsaco_emitted'] is False
            and row['gpu_execution'] is row['production_authority'] is False, 'qualified join, not prior emission')
    require(value['staged_join_qualification_completed'] is True and value['source_unchanged'] is True
        and value['limits_changed'] is False and value['fresh_compiler_built'] is False
        and value['full_compiler_cohort_requalified'] is False
        and set(value['phases']) == h.PHASES and len(value['raw']) == 75,
        'thirteen actual bounded consumer phases')
    require(set(value['artifacts']) == {'lower-tests', 'finalizer-tests'}
        and value['artifacts']['finalizer-tests']['sha256'] == TEST_ELF_SHA
        and all(Path(pin['path']).is_relative_to(TARGET) for pin in value['artifacts'].values()),
        'actual qualified test executables')
    require(value['raw']['sources-before.json']['sha256'] == SOURCE_SHA
        and all(value['raw']['sources-before.json'][key] == value['raw']['sources-after.json'][key]
                for key in ('bytes', 'sha256')) and len(value['formatted_sources']) == 5,
        'exact tested formatted source snapshot')
    for name, passed, ignored in (('lower', 785, 0), ('indexed_subset_repeat', 20, 0), ('finalizer', 190, 15)):
        require(value['tests'][name]['passed'] == passed and value['tests'][name]['ignored'] == ignored,
            'actual named consumer cohort')
    require(value['retained_handoff'] == h.HANDOFF and value['actual_join']['actual_capture_join_passed'] is True,
        'same retained producer handoff already joined')


def producer_gate(value, owner, c, lower):
    require(value['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-result-v1'
        and owner['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-owned-result-v1'
        and value['passed'] is owner['passed'] is False and value['postcheck_errors'] == owner['postcheck_errors'] == []
        and value['error'] == 'AssertionError: actual-inert-join' and owner['completion'] is None,
        'original failed aggregate is preserved, not relabeled')
    require(tuple(row['name'] for row in value['commands']) == PRODUCER_PHASES
        and set(value['artifacts']) == set(lower.CAPTURES)
        and value['compiler_generation'] == owner['compiler_generation'] == c['generation']
        and value['package_manifest'] == owner['package_manifest'] == c['manifest']
        and value['candidate_cpu'] == owner['candidate_cpu'] == c['cpu_pin']
        and value['source_manifest'] == owner['source_manifest'] == c['source_manifest'],
        'actual three-stage producer and source lineage')
    for key in ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted', *lower.FALSE_FLAGS):
        require(value[key] is False, 'original failed aggregate flags unchanged')
    result = owner['owned']
    require(type(result['exit_code']) is int and result['exit_code'] == 1 and result['reason'] is None
        and result['cleanup_signalled'] is False and result['owned_groups_absent'] is True
        and result['owned_processes_reaped'] is True, 'terminal natural failed producer owner')


def consumer_command(template, producer_target, roles):
    command = copy.deepcopy(template)
    name = command['name']
    require(name in CONSUMER_PHASES and command['expected_exit'] == 0
        and command['affinity'] == [8, 9] and command['nice'] == 10
        and command['cache_cap_bytes'] == 6 << 30 and command['gpu_execution'] is False,
        'unchanged bounded consumer command')
    replacements = {str(PRODUCER / name): str(OUT / name) for name in ('emitted', 'extracted')}
    command['argv'] = [next((new + item[len(old):] for old, new in replacements.items()
        if item == old or item.startswith(old + '/')), item) for item in command['argv']]
    role = {'actual-inert-join': 'finalizer-tests', 'emit': 'finalizer', 'descriptor-metadata': 'metadata'}.get(name)
    if role:
        require(role in roles, 'selected consumer tool is actually available')
        command['argv'][0] = roles[role]['path']
    env = command['env']
    require(env['CARGO_TARGET_DIR'] == str(PRODUCER / 'target') and env['TMPDIR'] == str(PRODUCER / 'tmp')
        and env['LD_LIBRARY_PATH'].split(':')[0] == str(producer_target / 'debug/deps')
        and len(env['LD_LIBRARY_PATH'].split(':')) == 2
        and 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1' not in env, 'exact historical target/loader/temp, no diagnostic flag')
    env['CARGO_TARGET_DIR'], env['TMPDIR'] = str(TARGET), str(OUT / 'tmp')
    env['LD_LIBRARY_PATH'] = str(TARGET / 'debug/deps') + ':' + env['LD_LIBRARY_PATH'].split(':')[1]
    return command


def protected_targets(paths):
    result = tuple(dict.fromkeys(Path(path) for path in paths if Path(path) != TARGET))
    require(result and all(not TARGET.is_relative_to(path) and not path.is_relative_to(TARGET) for path in result),
        'only the exact qualified consumer target is extended')
    return result


def context(package_sha):
    h = load(QUALIFIER, QUALIFIER_SHA, 'qualified_indexed_consumer')
    hc = h.context(QUALIFIER_PACKAGE_SHA, QUALIFIER_SOURCE_SHA)
    p, outer, pins = hc['modules']['probe'], hc['modules']['outer'], hc['pins']
    pins.pin(QUALIFIER, QUALIFIER_SHA)
    lower = load(LOWERING, LOWERING_SHA, 'retained_rope_producer')
    lc = lower.context(LOWERING_PACKAGE_SHA, ROPE_CPU, ROPE_CPU_SHA, TOOLS_SHA, TOOLS_OWNER_SHA)
    pins.pin(LOWERING, LOWERING_SHA)
    manifest, package = pins.json(PACKAGE / 'manifest.json', package_sha)
    require(manifest['schema'] == 'ferric-p228-rope-indexed-checked-emission-package-v1'
        and len(manifest['files']) == 4 and {row['path'] for row in manifest['files']} == FILES
        and {path.name for path in PACKAGE.iterdir()} == FILES | {'manifest.json'}, 'closed four-file continuation package')
    for row in manifest['files']: outer.pin_exact(pins, dict(row, path=str(PACKAGE / row['path'])))
    require(pins.records[str(PACKAGE / 'contracts.py')]['sha256'] == CONTRACT_SHA, 'unchanged prefix emission contracts')
    contracts = load(PACKAGE / 'contracts.py', CONTRACT_SHA, 'linked_prefix_contracts')
    value, qualified_pin = pins.json(QUALIFIED / 'complete.json', QUALIFIED_SHA)
    owner, qualified_owner_pin = pins.json(QUALIFIED_OWNER / 'complete.json', QUALIFIED_OWNER_SHA)
    qualified_gate(value, owner, qualified_pin, h)
    outer.check_outcome(owner['owned'])
    require(value['compiler_cpu'] == hc['v'].CPU_PIN and value['original_source_baseline'] == hc['baseline_pin']
        and value['original_source_baseline_owner'] == hc['baseline_owner_pin'], 'original source/control lineage')
    for record in [*value['raw'].values(), *value['artifacts'].values(), *value['formatted_sources'].values()]:
        outer.pin_exact(pins, record, QUALIFIED)
    read = lambda name: p.read(value['raw'][name]['path'], value['raw'][name], retain=True)[2].decode()
    for name, phase in value['phases'].items():
        require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
            and phase['group_absent'] is True and p.parse(read(name + '-result.json')) == phase
            and all(phase[key + '_sha256'] == value['raw'][name + '-' + key]['sha256'] for key in ('stdout', 'stderr')),
            'actual successful consumer phase and streams')
    base = hc['base']
    names = base.inventory(read('lower-list-stdout'))
    ignored = sorted(re.findall(r'^([^\r\n]+): test$', read('lower-ignored-list-stdout'), re.M))
    require(h.lower_inventory(names, ignored, hc['proposal'], hc['baseline_names']) == value['lower_inventory']
        and base.test_results(read('lower-tests-stdout'), names, ignored) == value['tests']['lower']
        and base.test_results(read('lower-indexed-tests-stdout'), sorted(hc['proposal']['added_tests']), [])
            == value['tests']['indexed_subset_repeat'], 'actual full785 and explicit20 repeat')
    fnames = base.inventory(read('finalizer-list-stdout'))
    fignored = sorted(re.findall(r'^([^\r\n]+): test$', read('finalizer-ignored-list-stdout'), re.M))
    hc['v'].finalizer_inventory(fnames, fignored, hc['tools']['tests'])
    require(base.test_results(read('finalizer-tests-stdout'), fnames, fignored) == value['tests']['finalizer']
        and h.actual_join(read('actual-inert-join-stdout'), read('actual-inert-join-stderr'), value['phases']['actual-inert-join'])
            == value['actual_join'], 'actual190/15 and retained inert-join consumer')
    prior, producer_pin = pins.json(PRODUCER / 'failed.json', PRODUCER_SHA)
    prior_owner, producer_owner_pin = pins.json(PRODUCER_OWNER / 'failed.json', PRODUCER_OWNER_SHA)
    producer_gate(prior, prior_owner, lc, lower)
    lp = lc['p']
    recipe, recipe_pin = pins.json(PRODUCER / 'recipe.json')
    require(recipe == lc['recipe'], 'exact prior recipe rederived from authenticated source/generation')
    for record in prior['artifacts'].values(): outer.pin_exact(pins, record, PRODUCER)
    require(prior['artifacts']['prefix-tiles.handoff-v3'] == h.HANDOFF, 'unchanged producer/consumer handoff')
    substitutions = {'@candidate.semantic.sha256': prior['artifacts']['prefix-tiles-semantic.bin']['sha256'],
        '@candidate.handoff.sha256': h.HANDOFF['sha256'], '@candidate.handoff.bytes': h.HANDOFF['bytes']}
    for template, previous in zip(recipe['commands'][:3], prior['commands']):
        record, streams = lp.completed_leaf(PRODUCER, lp.substitute(template, substitutions))
        require(record == previous, 'exact retained successful producer leaf')
        for pin in record.values():
            if isinstance(pin, dict): outer.pin_exact(pins, pin, PRODUCER)
        if previous['name'] == 'checked-lowering':
            require(all(('stage=' + stage + ' status=complete').encode() in streams['stderr'][1]
                for stage in ('pre-ranked', 'neutral', 'target-before-llvm')), 'actual checked compiler stages')
        if previous['name'] == 'actual-replay':
            contracts.exact_test(streams['stdout'][1], template['argv'][2])
            require(all(marker in streams['stdout'][1] + streams['stderr'][1] for marker in (
                b'stage=recipe status=complete roots=15', b'stage=ranked status=complete',
                b'stage=formal status=complete roots=15 unresolved=8')), 'actual fifteen-root producer replay')
    sources = p.parse(read('sources-before.json'))
    dependencies = p.parse(read('dependencies-before.json'))
    configurations = p.parse(read('configurations-before.json'))
    p.config_paths = lambda: sorted(configurations)
    require(len(sources) == 5785 and p.snapshot(p.tree(SOURCE)) == sources
        and p.snapshot(sorted(dependencies), byte_cap=4 << 30) == dependencies
        and p.configurations(configurations) == configurations, 'current exact tested consumer source/dependencies/configuration')
    for row in recipe['fixture']:
        outer.pin_exact(pins, dict(row['source'], path=str(PRODUCER / 'fixture' / row['destination'])))
    old_paths = protected_targets((*outer.OLD_TARGETS, *hc['v'].EXTRA_OLD_TARGETS, *lc['old_targets'],
        h.DIAGNOSTIC / 'target', h.PREVIOUS / 'target', h.BASELINE / 'target', PRODUCER / 'target'))
    return dict(h=h, hc=hc, lower=lower, lc=lc, p=p, lp=lp, outer=outer, pins=pins, base=base,
        contracts=contracts, package=package, value=value, qualified_pin=qualified_pin,
        qualified_owner_pin=qualified_owner_pin, producer=prior, producer_pin=producer_pin,
        producer_owner_pin=producer_owner_pin, recipe=recipe, recipe_pin=recipe_pin,
        substitutions=substitutions, sources=sources, dependencies=dependencies,
        configurations=configurations, old_paths=old_paths)


def child(c):
    p, lp, base, h = c['p'], c['lp'], c['base'], c['h']
    n = c['hc']['modules']['bounded']
    require(not os.path.lexists(OUT), 'fresh continuation output')
    OUT.mkdir(mode=0o700)
    n.F, n.T, n.D = SOURCE, TARGET, OUT
    n.setup()
    env = n.environment()
    previous = p.parse(p.read(QUALIFIED / 'finalizer-build-tests-command.json',
        c['value']['raw']['finalizer-build-tests-command.json'], retain=True)[2])
    expected_env = dict(previous['env'], TMPDIR=str(OUT / 'tmp'))
    require(env == expected_env, 'same qualified build environment, only fresh temporary output')
    before = lp.snapshot(c['lc']['immutable'], c['lc']['expected'], byte_cap=4 << 30)
    p.save(OUT / 'before.json', dict(producer_inputs=before, consumer_sources=c['sources'],
        consumer_dependencies=c['dependencies'], configurations=c['configurations']))
    p.save(OUT / 'inputs.json', dict(consumer=c['qualified_pin'], consumer_owner=c['qualified_owner_pin'],
        producer=c['producer_pin'], producer_owner=c['producer_owner_pin'], producer_recipe=c['recipe_pin'],
        package=c['package'], retained_handoff=h.HANDOFF))
    phases, commands, tools, artifacts = {}, [], {}, {}
    joined, error, errors = None, None, []
    substitutions = dict(c['substitutions'])
    base.COPY, base.OUT = SOURCE, QUALIFIED

    def run(command):
        command = lp.substitute(command, substitutions)
        name = command['name']; n.F = Path(command['cwd'])
        try:
            n.run(OUT, name, command['argv'], env=command['env'], deadline=command['deadline_seconds'])
        finally:
            path = OUT / (name + '-result.json')
            if path.is_file(): phases[name] = p.parse(p.read(path, retain=True)[2])
        record, streams = lp.completed_leaf(OUT, command)
        commands.append(record)
        return command, streams['stdout'][1], streams['stderr'][1]

    def build_command(name, argv, deadline):
        return dict(name=name, argv=argv, env=env, cwd=str(SOURCE), tools=previous['tools'],
            deadline_seconds=deadline, cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10,
            expected_exit=0, gpu_execution=False)

    try:
        cargo = str(n.N / 'bin/cargo')
        _, stdout, _ = run(build_command('metadata', [cargo, 'metadata', '--offline', '--locked',
            '--manifest-path', str(SOURCE / 'Cargo.toml'), '--format-version', '1'], 120))
        require(p.parse(stdout) == p.parse(p.read(QUALIFIED / 'metadata-stdout',
            c['value']['raw']['metadata-stdout'], retain=True)[2]), 'same qualified Cargo graph/source/target')
        roots = base.metadata_paths(p.parse(stdout), SOURCE)
        require(p.snapshot(sorted({name for root in roots for name in p.tree(root, exclusions=('.git', 'target'))}),
            byte_cap=4 << 30) == c['dependencies'], 'same full consumer dependency source roster')
        _, stdout, _ = run(build_command('tool-build', [cargo, 'build', '--offline', '--locked', '--jobs', '2',
            '--manifest-path', str(SOURCE / 'Cargo.toml'), '-p', 'fe2o3-hsaco-finalize',
            '--example', FINALIZER, '--example', METADATA, '--message-format=json'], 1800))
        for role, name in (('finalizer', FINALIZER), ('metadata', METADATA)):
            paths = base.built_artifacts(stdout.decode(), 'fe2o3-hsaco-finalize', name, False, TARGET)
            require(len(paths) == 1, 'one actual newly built consumer tool')
            tools[role] = p.read(paths[0])[0]
        roles = dict(tools, **{'finalizer-tests': c['value']['artifacts']['finalizer-tests']})
        templates = {row['name']: row for row in c['recipe']['commands']}
        for name in CONSUMER_PHASES:
            command, stdout, stderr = run(consumer_command(templates[name], Path(c['producer']['compiler_generation']['target']), roles))
            if name == 'actual-inert-join':
                joined = h.actual_join(stdout.decode(), stderr.decode(), phases[name])
            elif name == 'emit':
                caps = {'source.handoff-v3': 16 << 20, 'compiler.handoff-v2': 16 << 20,
                    'artifact.hsaco': 32 << 20, 'receipt.txt': 64 << 10}
                require({path.name for path in (OUT / 'emitted').iterdir()} == set(caps), 'closed actual emission roster')
                emitted = {name: p.read(OUT / 'emitted' / name, cap=cap)[0] for name, cap in caps.items()}
                require(all(pin['bytes'] > 0 for pin in emitted.values()) and all(
                    emitted['source.handoff-v3'][key] == h.HANDOFF[key] for key in ('bytes', 'sha256')),
                    'new emission consumes exact old checked handoff')
                worker = next(pin for pin in c['recipe']['toolchain_and_retained_tool_pins'] if pin['path'] == command['argv'][5])
                config = {Path(pin['path']).name: pin['sha256'] for pin in c['recipe']['toolchain_and_retained_tool_pins']
                    if '/gfx950-reviewed-libs/' in pin['path']}
                c['contracts'].emission(p.read(OUT / 'emitted/receipt.txt', cap=64 << 10, retain=True)[2],
                    emitted, worker, command['argv'][8], command['argv'][9], config)
                artifacts.update({'emitted/' + name: pin for name, pin in emitted.items()})
                substitutions.update({'@candidate.image.sha256': emitted['artifact.hsaco']['sha256'],
                    '@candidate.image.bytes': emitted['artifact.hsaco']['bytes']})
            elif name == 'extract-retained':
                require({path.name for path in (OUT / 'extracted').iterdir()} == {'formal.archive', 'module.ll'}, 'closed inert extraction')
                for name in ('formal.archive', 'module.ll'):
                    artifacts['extracted/' + name] = p.read(OUT / 'extracted' / name, cap=16 << 20)[0]
                p.read(OUT / 'extracted/module.ll', cap=16 << 20, retain=True)[2].decode('utf-8')
                require(b'authority none' in stdout and b'gpu_execution false' in stdout, 'no extraction authority')
            elif name == 'descriptor-metadata':
                c['contracts'].metadata(stdout, artifacts['emitted/artifact.hsaco'])
            elif name == 'elf-notes':
                require(all(marker in stdout for marker in (b'.group_segment_fixed_size: 512',
                    b'.private_segment_fixed_size: 0', b'.wavefront_size: 64')), 'same prefix resource requirements')
            elif name == 'disassembly':
                require(c['lower'].SYMBOL.encode() in stdout and b's_endpgm' in stdout, 'actual prefix symbol and ISA')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        checks = [('pins', None, c['pins'].recheck), ('producer-pins', None, c['lc']['pins'].recheck),
            ('consumer-sources', c['sources'], lambda: p.snapshot(p.tree(SOURCE))),
            ('consumer-dependencies', c['dependencies'], lambda: p.snapshot(sorted(c['dependencies']), byte_cap=4 << 30)),
            ('configurations', c['configurations'], lambda: p.configurations(c['configurations'])),
            ('producer-inputs', before, lambda: lp.snapshot(c['lc']['immutable'], byte_cap=4 << 30)),
            ('source-maps', (c['lc']['merged'], c['lc']['source_records']), lambda: lp.source_maps(c['lc']['inputs']['source_maps'])),
            ('provider', c['lc']['provider'], lambda: lp.provider(c['lc']['merged'])),
            ('producer-configurations', c['lc']['configs'], lambda: lp.configurations(c['lc']['inputs']['cargo_configurations']))]
        after, errors = c['outer'].postchecks(checks)
        p.save(OUT / 'after.json', after)
        try:
            for pin in (*tools.values(), *artifacts.values(), *c['value']['artifacts'].values()):
                p.read(pin['path'], pin, cap=128 << 20)
        except BaseException as failure:
            errors.append('selected products: ' + repr(failure))
    passed = error is None and not errors and tuple(row['name'] for row in commands) == PHASES and joined is not None
    value = dict(schema='ferric-p228-rope-indexed-checked-emission-result-v1', passed=passed,
        error=error, postcheck_errors=errors, package=c['package'], producer=c['producer_pin'],
        producer_owner=c['producer_owner_pin'], producer_recipe=c['recipe_pin'],
        producer_generation=c['producer']['compiler_generation'], producer_commands=c['producer']['commands'],
        producer_captures=c['producer']['artifacts'], consumer=c['qualified_pin'], consumer_owner=c['qualified_owner_pin'],
        consumer_source=c['value']['raw']['sources-before.json'], consumer_proposal=c['value']['proposal'],
        consumer_tests=c['value']['tests'], preserved_consumer_test_artifacts=c['value']['artifacts'],
        retained_handoff=h.HANDOFF, phases=phases, commands=commands, tools=tools, artifacts=artifacts,
        actual_join=joined, fresh_consumer_tools_built=set(tools) == {'finalizer', 'metadata'},
        fresh_actual_inert_join_passed=joined is not None, fresh_hsaco_emitted=passed,
        retained_checked_producer_replay_authenticated=True, producer_consumer_generations_distinct=True,
        unresolved_runtime_requirements=8 if passed else None, source_unchanged=not errors,
        raw={path.name: p.read(path)[0] for path in OUT.iterdir() if path.is_file()},
        **{key: False for key in FALSE_FLAGS})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python -B')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest_sha256'); parser.add_argument('--child', action='store_true')
    args = parser.parse_args()
    require(re.fullmatch('[0-9a-f]{64}', args.manifest_sha256) and Path(__file__).resolve() == PACKAGE / 'run.py'
        and os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'fixed reviewed CPU envelope')
    for kind, cap in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        limit = min([cap] + [value for value in old if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    c = context(args.manifest_sha256)
    if args.child:
        signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt('owned termination')))
        return child(c)
    p, outer, pins = c['p'], c['outer'], c['pins']
    require(not os.path.lexists(OWNER) and not os.path.lexists(OUT), 'fresh continuation owner/output')
    before = outer.inventory(c['hc']['modules']['inventory'], c['old_paths'])
    os.umask(0o077); OWNER.mkdir(mode=0o700)
    p.save(OWNER / 'old-targets-before.json', before)
    argv = ['/usr/bin/python3', '-B', str(PACKAGE / 'run.py'), args.manifest_sha256, '--child']
    env = c['hc']['modules']['bounded'].environment()
    p.save(OWNER / 'command.json', dict(argv=argv, env=env, deadline_seconds=10800, gpu_execution=False))
    outcome, completion, error = None, None, None
    try:
        outcome = c['hc']['modules']['driver'].run_coordinator(OWNER, argv, SOURCE, env,
            c['hc']['modules']['owned'], p.save, deadline=10800)
        p.save(OWNER / 'owned-result.json', outcome); outer.check_outcome(outcome)
        value, completion = pins.json(OUT / 'complete.json')
        require(value['schema'] == 'ferric-p228-rope-indexed-checked-emission-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['package'] == c['package'] and value['consumer'] == c['qualified_pin']
            and value['producer'] == c['producer_pin'] and value['fresh_hsaco_emitted'] is True
            and value['fresh_actual_inert_join_passed'] is True and value['fresh_consumer_tools_built'] is True
            and tuple(row['name'] for row in value['commands']) == PHASES
            and all(value[key] is False for key in FALSE_FLAGS), 'actual linked continuation, not fresh compiler qualification')
        for record in [*value['raw'].values(), *value['artifacts'].values()]: outer.pin_exact(pins, record, OUT)
        for record in value['tools'].values(): outer.pin_exact(pins, record, TARGET)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        after, errors = outer.postchecks([('pins', None, pins.recheck),
            ('producer-pins', None, c['lc']['pins'].recheck),
            ('consumer-source', c['sources'], lambda: p.snapshot(p.tree(SOURCE))),
            ('old-targets', before, lambda: outer.inventory(c['hc']['modules']['inventory'], c['old_paths']))])
        p.save(OWNER / 'after.json', after)
    passed = error is None and not errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-rope-indexed-checked-emission-owned-result-v1', passed=passed,
        error=error, postcheck_errors=errors, owned=outcome, completion=completion,
        package=c['package'], consumer=c['qualified_pin'], consumer_owner=c['qualified_owner_pin'],
        producer=c['producer_pin'], producer_owner=c['producer_owner_pin'],
        fresh_hsaco_emitted=passed, **{key: False for key in FALSE_FLAGS}))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
