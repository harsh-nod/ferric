"""Publish retained SiLU compiler text evidence; no compiler, test or native execution."""
import ast
import hashlib
import json
from pathlib import Path
import re
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
OUT = F / 'qualification/silu-materialized-lowering-v1'
DATA = F / 'qualification/paired-row-mlp-lowering-v1/publication/publish.py'
DATA_SHA = 'ead92c10bbdc6f16ce71041de322e4e29c63c3885a478088be392d874ee95766'
ROW = 'row-silu-materialized-checked-probe-v228-v1'
OWNER = 'silu-materialized-checked-probe-owner-v228-v1'
PACKAGE = 'p228-silu-materialized-lowering-v1'
PACKAGE_SHA = '24ff0ec4b86f2e60b7945b06ede8bcdec4fbd99131d33c3eabdb7c22dfb1e8b3'
GPU_PACKAGE = 'p228-silu-materialized-capture-gpu-v1'
GPU_PACKAGE_SHA = '71cae69a25ac82c53bdf2976ab53b9d9af615c5af4b8337c2e386beb44a8a18b'
CPU_SHA = 'e139729d8fa95715ef8017679ee21f1175ba302b681785789e3dc84137508a30'
INSPECT_SHA = 'b82f3a59be986c1f60ec88daf38affb56c3c0c8aa3fb2e80471450c5b4f34127'
PURE27 = 'silu-materialized-lowering-pure-v228-v1'
PURE36 = 'silu-materialized-capture-gpu-pure-v228-v1'
WRAPPER27 = '80819ec522b16560add09fccefc04d98477a829538722671abe4e0cbc3ace853'
WRAPPER36 = '13c47f9f53496c88357aa05052b5429fb0b9bf2ee0501dfcc4dfd7f89e301e84'
ALIASES = {str(E / 'run_silu_materialized_capture_gpu_pure_p228_v1.py'):
           L / 'proposals/p228-silu-materialized-capture-harness-v1/run_silu_materialized_capture_gpu_pure_p228_v1.py'}
LOCAL = {}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def local_body(path):
    require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink()
            and path.stat().st_size <= 128 << 10, 'canonical bounded local source')
    raw = path.read_bytes()
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(str(path) not in LOCAL or LOCAL[str(path)] == pin, 'local source changed')
    LOCAL[str(path)] = pin
    return pin, raw


def load_data():
    pin, raw = local_body(DATA)
    require(pin['sha256'] == DATA_SHA, 'published retained-data helper')
    module = types.ModuleType('silu_publication_retained_data')
    module.__file__ = str(DATA)
    exec(compile(raw, str(DATA), 'exec'), module.__dict__)
    original_local = module.local

    def retained(path):
        if str(path) not in ALIASES:
            return original_local(path)
        result = ALIASES[str(path)]
        require(result.resolve(strict=True) == result and result.is_file()
                and not result.is_symlink(), 'canonical explicit wrapper transport')
        return result

    module.local = retained
    return module


def actual(H, ledger, relative, digest, destination):
    require(re.fullmatch('[0-9a-f]{64}', digest), 'actual CLI receipt SHA')
    raw = H.local(E / H.relative(relative)).read_bytes()
    require(len(raw) <= 64 << 20 and hashlib.sha256(raw).hexdigest() == digest, 'actual completion bytes')
    pin = dict(path=str(E / relative), **H.content(raw))
    return ledger.document(pin, destination), pin


def package(H, ledger, name, digest, schema, count, destination):
    manifest, pin = actual(H, ledger, name + '/manifest.json', digest, destination + '/manifest.json')
    require(manifest['schema'] == schema and len(manifest['files']) == count
            and len({row['path'] for row in manifest['files']}) == count, 'closed frozen package')
    sources = {}
    for row in manifest['files']:
        relative = H.relative(row['path'])
        source = dict(row, path=str(E / name / relative))
        ledger.take(source, destination + '/' + row['path'])
        sources[row['path']] = source
    return manifest, pin, sources


def test_names(ledger, sources):
    result = {}
    for name, pin in sources.items():
        if not (name.startswith('test_') and name.endswith('.py')):
            continue
        tree = ast.parse(ledger.take(pin))
        names = [name[:-3] + '.' + cls.name + '.' + method.name
                 for cls in tree.body if isinstance(cls, ast.ClassDef)
                 for method in cls.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')]
        require(len(names) == len(set(names)), 'unique authored test names')
        result[name] = sorted(names)
    return result


def pure(H, ledger, label, digest, sources, manifest_pin, count):
    prefix = 'pure' + str(count)
    value, receipt = actual(H, ledger, label + '/complete.json', digest, prefix + '/complete.json')
    schema = 'ferric-p228-silu-materialized-' + ('lowering' if count == 27 else 'capture-gpu') + '-pure-v1'
    require(value['schema'] == schema and value['passed'] is True and value['tests'] == count
            and value['failures'] == value['errors'] == value['skipped'] == 0
            and value['source_postchecks_passed'] is True, 'actual passing synthetic suite')
    require(all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance',
                'performance_claim', 'production_authority')), 'synthetic tests grant no GPU authority')
    expected = test_names(ledger, sources)
    names = sorted(name for rows in expected.values() for name in rows)
    require(len(names) == count, 'exact authored suite size')
    if count == 27:
        require(value['synthetic_only'] is True and value['rust_compilation'] is False
                and value['names'] == names and value['controller']['sha256'] == WRAPPER27, 'CPU-only lowering policy suite')
        records = value['raw']
        require(set(records) == {'sources-before.json', 'sources-after.json', 'tests.log'}, 'three raw policy outputs')
        controller = value['controller']
        expected_sources = dict(sources, **{'manifest.json': manifest_pin, 'controller': controller})
    else:
        require(value['synthetic_policy_tests_only'] is True and value['native_execution'] is False
                and value['full_model_acceptance'] is False
                and value['manifest_sha256'] == manifest_pin['sha256']
                and value['controller_sha256'] == WRAPPER36
                and value['test_inventory_before'] == value['test_inventory_after'] == expected,
                'CPU-only capture policy suite')
        records = {'sources-before.json': value['sources_before'],
                   'sources-after.json': value['sources_after'], 'tests.log': value['transcript']}
        require(value['source_sha256'] == records['sources-before.json']['sha256'], 'pure source snapshot join')
        raw = H.local(E / 'run_silu_materialized_capture_gpu_pure_p228_v1.py').read_bytes()
        controller = dict(path=str(E / 'run_silu_materialized_capture_gpu_pure_p228_v1.py'), **H.content(raw))
        require(controller['sha256'] == WRAPPER36, 'actual pure36 wrapper')
        expected_sources = sources
    ledger.take(controller, prefix + '/controller.py')
    for name, record in records.items():
        require(record['path'] == str(E / label / name), 'actual pure output namespace')
        ledger.take(record, prefix + '/' + name)
    require(ledger.document(records['sources-before.json']) == ledger.document(records['sources-after.json'])
            == expected_sources, 'exact tested source identities')
    transcript = ledger.take(records['tests.log']).decode()
    rows = re.findall(r'^(test_\w+) \(([A-Za-z0-9_.]+)\) \.\.\. ok$', transcript, re.M)
    actual_names = [scope if scope.endswith('.' + method) else scope + '.' + method for method, scope in rows]
    require(sorted(actual_names) == names and len(actual_names) == len(set(actual_names))
            and re.search(r'^Ran ' + str(count) + r' tests in .+s$', transcript, re.M)
            and transcript.rstrip().endswith('OK'), 'actual exact named pure transcript')
    return receipt


def main():
    require(not sys.flags.optimize and len(sys.argv) == 6,
            'OWNER_SHA INSPECTION_LABEL INSPECTION_SHA PURE27_SHA PURE36_SHA')
    owner_sha, inspection_label, inspection_sha, pure27_sha, pure36_sha = sys.argv[1:]
    require(re.fullmatch(r'silu-materialized-inspection-v228-v[1-9][0-9]*', inspection_label), 'actual inspection namespace')
    H = load_data()
    ledger = H.Ledger()
    own_pin, own_body = local_body(Path(__file__).resolve())
    local_copies = {'publication/publish.py': (own_pin, own_body),
                    'publication/retained_data.py': (LOCAL[str(DATA)], DATA.read_bytes())}
    owner, owner_pin = actual(H, ledger, OWNER + '/complete.json', owner_sha, 'owner/complete.json')
    lower_pin = owner['completion']
    require(lower_pin['path'] == str(E / ROW / 'complete.json'), 'exact actual inner completion')
    lower = ledger.document(lower_pin, 'lowering/complete.json')
    H.passed(owner, 'ferric-p228-silu-materialized-lowering-owned-result-v1')
    H.passed(lower, 'ferric-p228-silu-materialized-lowering-result-v1')
    require(all(owner[key] == lower[key] for key in ('package_manifest', 'candidate_cpu', 'compiler_generation',
                'source_manifest', 'prior_lowering')) and lower['unresolved_runtime_requirements'] == 8
            and all(lower[key] is True for key in ('fresh_checked_lowering', 'fresh_checked_replay',
                'fresh_hsaco_emitted', 'frontend_recipe_is_diagnostic')), 'fresh conditional checked generation')
    owned = owner['owned']
    require(owned['exit_code'] == 0 and owned['reason'] is None and owned['cleanup_signalled'] is False
            and owned['owned_groups_absent'] is True and owned['owned_processes_reaped'] is True, 'natural owner exit/reap')
    require(set(owner['raw']) == {'before.json', 'after.json', 'command.json', 'started.json',
                                  'owned-result.json', 'stdout', 'stderr'}, 'owner raw roster')
    for name, pin in owner['raw'].items():
        require(pin['path'] == str(E / OWNER / name), 'owner raw namespace')
        ledger.take(pin, None if name in ('before.json', 'after.json') else 'owner/' + name)
    require(ledger.document(owner['raw']['owned-result.json']) == owned, 'actual owned record')
    before, after = (ledger.document(owner['raw'][name]) for name in ('before.json', 'after.json'))
    require(all(before[key] == after[key] for key in ('files', 'old_targets', 'compiler_source_roster')),
            'recorded whole source/cache custody')
    manifest, package_pin, sources = package(H, ledger, PACKAGE, PACKAGE_SHA,
        'ferric-p228-silu-materialized-lowering-package-v1', 4, 'controller')
    require(package_pin == lower['package_manifest'], 'compiled controller package')
    _, gpu_pin, gpu_sources = package(H, ledger, GPU_PACKAGE, GPU_PACKAGE_SHA,
        'ferric-p228-silu-materialized-capture-gpu-package-v1', 11, 'capture-controller')
    inspection, inspection_pin = actual(H, ledger, inspection_label + '/complete.json', inspection_sha,
                                        'inspection/complete.json')
    require(inspection['schema'] == 'ferric-p228-silu-materialized-inspection-v1'
            and inspection['owner'] == owner_pin and inspection['completion'] == lower_pin
            and inspection['candidate_cpu'] == lower['candidate_cpu']
            and inspection['source_manifest'] == lower['source_manifest']
            and inspection['controller']['sha256'] == INSPECT_SHA
            and inspection['static_inspection_complete'] is True and inspection['root_manual_review_required'] is True,
            'actual matched literal inspection, still requires manual review')
    require(all(inspection[key] is False for key in ('control_flow_equivalence_proved', 'isa_arithmetic_equivalence_proved',
                'ocml_accuracy_proved', 'native_execution', 'gpu_execution', 'numerical_acceptance',
                'performance_claim', 'production_authority', 'transitive_compiler_sources_replayed', 'cargo_cache_exported')),
            'inspection claim limits')
    ledger.take(inspection['controller'], 'inspection/controller.py')
    inspection_inputs = {pin['path']: pin for pin in inspection['exported_inputs']}
    require(len(inspection_inputs) == len(inspection['exported_inputs']), 'unique inspected originals')
    for pin in inspection_inputs.values():
        ledger.take(pin)
    recipe_pin = inspection_inputs[str(E / ROW / 'recipe.json')]
    recipe = ledger.document(recipe_pin, 'lowering/recipe.json')
    require(recipe['compiler_generation'] == lower['compiler_generation']['prerequisites']
            and tuple(row['name'] for row in recipe['commands']) == tuple(row['name'] for row in lower['commands']) == H.STAGES,
            'actual nine-stage recipe and preserved generation')
    image = lower['artifacts']['emitted/artifact.hsaco']
    replacements = {'@candidate.semantic.sha256': lower['artifacts']['mlp-tiles-semantic.bin']['sha256'],
        '@candidate.handoff.sha256': lower['artifacts']['mlp-tiles.handoff-v3']['sha256'],
        '@candidate.handoff.bytes': lower['artifacts']['mlp-tiles.handoff-v3']['bytes'],
        '@candidate.image.sha256': image['sha256'], '@candidate.image.bytes': image['bytes']}
    phases = {}
    for row, template in zip(lower['commands'], recipe['commands']):
        name = row['name']
        for key in ('command', 'started', 'result', 'stdout', 'stderr'):
            suffix = '-' + key + ('.json' if key in ('command', 'started', 'result') else '')
            require(row[key]['path'] == str(E / ROW / (name + suffix)), 'exact phase record namespace')
            ledger.take(row[key])
        start = ledger.document(row['started'])
        require(set(start) == {'pid', 'pgid'} and type(start['pid']) is int
                and start['pid'] > 0 and start['pid'] == start['pgid'], 'owned phase process group')
        command, result = ledger.document(row['command']), ledger.document(row['result'])
        expected = H.substitute({key: value for key, value in template.items() if key not in ('name', 'cwd')}, replacements)
        require(0 < command['deadline_seconds'] <= expected['deadline_seconds'], 'retained or shortened deadline')
        expected['deadline_seconds'] = command['deadline_seconds']
        require(command == expected and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and all(result[key + '_sha256'] == row[key]['sha256']
                                                         for key in ('stdout', 'stderr')), 'exact natural phase and streams')
        if name in ('actual-replay', 'actual-inert-join'):
            H.rust_results(ledger.take(row['stdout']), [command['argv'][2]])
        ledger.take(row['stdout'], 'lowering/' + name + '-stdout')
        if name == 'checked-lowering':
            ledger.take(row['stderr'], 'lowering/checked-lowering-stderr')
        phases[name] = dict(result=result, command=row['command'], started=row['started'])
    for name, pin in lower['artifacts'].items():
        ledger.take(pin, 'lowering/' + name if name in ('extracted/module.ll', 'emitted/receipt.txt') else None)
    require(len(recipe['fixture']) == 7, 'seven-file actual fixture')
    for row in recipe['fixture']:
        relative = H.relative(row['destination'])
        pin = dict(row['source'], path=str(E / ROW / 'fixture' / relative))
        require(pin == inspection_inputs[pin['path']], 'fixture byte identity exported from actual generation')
        ledger.take(pin, 'fixture/' + row['destination'])
    cpu = ledger.document(lower['candidate_cpu'], 'cpu/complete.json')
    require(lower['candidate_cpu']['sha256'] == CPU_SHA and cpu['passed'] is True and cpu['error'] is None
            and cpu['postcheck_errors'] == [] and cpu['tests_passed'] == 38 and cpu['tests_ignored'] == 0
            and cpu['source_unchanged'] is True and cpu['overlay'] == lower['source_manifest'], 'actual CPU38 source prerequisite')
    ledger.take(cpu['runner'], 'cpu/controller.py')
    ledger.take(cpu['overlay'], 'source/source-manifest.json')
    for name, pin in cpu['formatted_sources'].items():
        ledger.take(pin, 'source/' + str(H.relative(name)))
    p27 = pure(H, ledger, PURE27, pure27_sha, sources, package_pin, 27)
    p36 = pure(H, ledger, PURE36, pure36_sha, gpu_sources, gpu_pin, 36)
    require(not (set(local_copies) & set(ledger.copies)), 'distinct publication files')
    copies = {**ledger.copies, **local_copies}
    require(sum(len(raw) for _, raw in copies.values()) <= 8 << 20, 'bounded text checkpoint')
    summary = dict(schema='ferric-p228-silu-materialized-lowering-publication-v1', passed=True,
        publication_helper=own_pin, data_helper=LOCAL[str(DATA)], owner=owner_pin, lowering=lower_pin,
        inspection=inspection_pin, cpu=lower['candidate_cpu'], pure27=p27, pure36=p36,
        cpu_tests=cpu['tests'], pure_tests={'lowering_policy': 27, 'capture_policy': 36},
        phases=phases, artifacts=lower['artifacts'], resources=inspection['resources'], llvm=inspection['llvm'],
        actual_artifact_bodies_rehashed=True, fixture_source_join_checked=True, native_execution=False,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False, production_authority=False,
        isa_review_accepted=False, runtime_requirements_discharged=False, full_model_acceptance=False,
        limits=['This publisher rehashes the explicit ledger, not all transitive compiler/tool inputs.',
                'Static inspection is literal SSA evidence, not a control-flow, final-ISA or OCML proof.',
                'Pure36 qualifies Python admission policy only; no native SiLU capture is asserted.',
                'Eight runtime obligations remain open; no runtime review is minted.',
                'Binary artifacts, source maps and caches stay outside Git; original artifact pins are retained.'],
        inputs=list(ledger.inputs.values()), local_inputs=list(LOCAL.values()),
        explicit_transport_aliases={key: str(path) for key, path in ALIASES.items()},
        published_files=[dict(path=name, source_pin=pin) for name, (pin, _) in sorted(copies.items())])
    ledger.recheck()
    for path in list(LOCAL):
        local_body(Path(path))
    require(not OUT.is_symlink(), 'no publication alias')
    if OUT.exists():
        require(OUT.resolve(strict=True) == OUT and OUT.is_dir()
                and {path.name for path in OUT.iterdir()} <= {'README.md'}, 'fresh checkpoint, root README only')
        if (OUT / 'README.md').exists():
            require((OUT / 'README.md').is_file() and not (OUT / 'README.md').is_symlink(), 'regular root README')
    OUT.mkdir(exist_ok=True)
    for name, (_, raw) in sorted(copies.items()):
        path = OUT / H.relative(name)
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    with (OUT / 'result.json').open('x') as stream:
        json.dump(summary, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    ledger.recheck()
    for path in list(LOCAL):
        local_body(Path(path))
    print(json.dumps(dict(output=str(OUT), files=len(copies), result=H.content((OUT / 'result.json').read_bytes()))))


if __name__ == '__main__':
    main()
