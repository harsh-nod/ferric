"""CPU475 worker delta over the unchanged, separately verified CPU633 parent."""
from pathlib import Path
import re

import policy_portable as P
import layer_validation as V

SCHEMA = 'ferric-p228-group-fence-deployment-v3'
CPU_SHA = '0231c40bf9a5ff285e041f29b2c958e26972e73f6a4485072e153a0bd178171b'
BASE_SHA = '4870de9c93693d073f77a1acdf88d77f23a6c6af8440ba88402af2f7320c01db'
RUNNER_SHA = '64f041c525d16510a0a2d6d2290f9cf17914fe3eecca263504f4d01d5b2d988c'
OVERLAY_SHA = '9a8f1d1f4da5cc846891f4721f7b37783777e4c2a368c5bf1fc340cface77834'
WORKER_SHA = '3a16059a96c2050b654e4998b395b9672a19a593f08bf0c5df893d181a1f5294'
FILTERS = (
    ('context', 'engineering_gfx950::tests::', 14),
    ('peer', 'engineering_gfx950::peer::tests::', 15),
    ('device-group', 'device::gfx950::group_currentness::tests::', 7),
    ('publication', 'engineering_gfx950::peer::round::tests::', 16),
    ('observation', 'engineering_gfx950::peer::host_observation::tests::', 13),
    ('policy', 'engineering_gfx950::peer::performance::tests::', 5),
    ('ordered', 'engineering_gfx950::ordered_batch::tests::', 7),
)


def recipes(directory, tools):
    worker = directory / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    cargo = str(Path(tools['worker']['root']) / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(worker)]
    runtime = [cargo, 'test', *common, '-p', 'fe2o3-kfd', '--lib']
    ordinary = [cargo, 'test', *common, '--lib', '--test', 'shared_wire']
    rows = [('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path',
                          str(worker), '--format-version', '1'], 120),
            ('runtime-list', [*runtime, '--', '--list', '--format', 'terse'], 1200)]
    rows += [(name, [*runtime, selector, '--', '--test-threads=2'], 1200)
             for name, selector, _ in FILTERS]
    rows += [('worker-list', [*ordinary, '--', '--list', '--format', 'terse'], 1200),
             ('worker-tests', [*ordinary, '--', '--test-threads=2'], 1200),
             ('worker-build', [cargo, 'build', '--profile', 'test', *common,
                               '--bin', P.WORKER, '--message-format=json'], 1200)]
    return rows, worker


def source_delta(store, cpu, base_store, base_cpu):
    inputs = {row['path']: row for row in cpu['inputs']}
    V.require(len(inputs) == len(cpu['inputs']) == 25, 'closed CPU475 input census')
    for row in inputs.values():
        store.get(row)
    root = Path(base_cpu['source_inputs']['path']).parent
    runner = inputs.get(str(root / 'run_group_fence_cpu_p228_v2.py'))
    V.require(runner is not None and runner['sha256'] == RUNNER_SHA, 'reviewed CPU475 runner')
    expected = [runner, base_cpu['runner'], base_cpu['source_inputs'],
                base_cpu['extractor'], base_cpu['helper'], base_cpu['overlay']]
    source_inputs = store.doc(base_cpu['source_inputs'])
    V.require(source_inputs['archives'] == base_cpu['archives'], 'same two source archives')
    installed = P.git_source_map(store, source_inputs['archives'])
    expected += [{k: row[k] for k in ('path', 'bytes', 'sha256')}
                 for row in source_inputs['archives'].values()]
    overlay_pin = inputs.get(str(root / 'p228-group-fence-consolidation-v1/manifest.json'))
    V.require(overlay_pin is not None and overlay_pin['sha256'] == OVERLAY_SHA,
              'exact three-file group-fence proposal')
    expected.append(overlay_pin)
    changed = set()
    for project, record, count in (('ferric', base_cpu['overlay'], 13), ('fe2o3', overlay_pin, 3)):
        overlay = store.doc(record)
        V.require(overlay['baseline_head'] == source_inputs['archives'][project]['commit']
            and len(overlay['files']) == count, 'exact overlay baseline and cardinality')
        names = set()
        for row in overlay['files']:
            name = project + '/' + row['path']
            V.require(name not in names and installed.get(name) == row['before'],
                      'unique replacement with exact before-body')
            names.add(name)
            body = dict(row['after'], path=str(Path(record['path']).parent / 'draft' / row['path']))
            store.get(body)
            expected.append(body)
            installed[name] = row['after']
        if project == 'fe2o3':
            changed = names
    V.require(inputs == {row['path']: row for row in expected}, 'all and only reviewed CPU475 inputs')
    before = base_store.doc(base_cpu['raw']['sources-after.json'])
    V.require(set(before) == set(installed) and len(installed) == 6928
        and {name for name in before if before[name] != installed[name]} == changed,
        'only three runtime sources differ from CPU633')
    V.require(store.doc(cpu['raw']['sources-before.json']) == installed
        == store.doc(cpu['raw']['sources-after.json']), 'actual compiled sources unchanged')
    return runner, overlay_pin


def cpu_evidence(store, complete, base_store, base_cpu):
    cpu = store.doc(complete)
    V.keys(cpu, 'schema passed error postcheck_errors inputs metadata phases tests binaries tests_passed '
        'tests_ignored empty_initial_target external_cargo_cache_reused gpu_execution numerical_acceptance '
        'performance_claim production_authority raw')
    V.require(cpu['schema'] == 'ferric-p228-group-fence-cpu-result-v1' and cpu['passed'] is True
        and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['empty_initial_target'] is True and cpu['external_cargo_cache_reused'] is True
        and type(cpu['tests_passed']) is int and cpu['tests_passed'] == 475
        and type(cpu['tests_ignored']) is int and cpu['tests_ignored'] == 4
        and all(cpu[k] is False for k in ('gpu_execution', 'numerical_acceptance',
                                        'performance_claim', 'production_authority')),
        'distinct actual CPU475 receipt, never a joint parent build')
    directory = Path(complete['path']).parent
    V.require(directory.name == 'group-fence-cpu-v228-v2', 'reviewed actual CPU output')
    raw = cpu['raw']
    V.require(type(raw) is dict and len(raw) == 62, 'closed CPU475 raw census')
    for name, record in raw.items():
        V.require(Path(name).name == name and record['path'] == str(directory / name), 'actual raw location')
        store.get(record)
    runner, overlay = source_delta(store, cpu, base_store, base_cpu)
    tools = {'worker': base_cpu['toolchains']['worker']}
    expected, worker = recipes(directory, tools)
    V.require(set(cpu['phases']) == {row[0] for row in expected}, 'all twelve actual CPU phases')
    outputs = {}
    for name, argv, deadline in expected:
        command, started, result = (store.doc(raw[name + suffix]) for suffix in
                                   ('-command.json', '-started.json', '-result.json'))
        V.keys(command, 'argv env tools deadline_seconds cache_cap_bytes affinity nice gpu_execution expected_exit')
        V.keys(started, 'pid pgid')
        V.keys(result, 'exit_code reason elapsed_seconds group_absent cache_bytes stdout_sha256 stderr_sha256')
        V.require(command == dict(argv=argv, env=P.cpu_environment(directory, tools, 'worker'),
            tools=P.TOOL_PINS['worker'], deadline_seconds=deadline, cache_cap_bytes=6 << 30,
            affinity=[8, 9], nice=10, gpu_execution=False, expected_exit=0),
            'exact CPU475 bounded command and compiler environment')
        V.require(type(started['pid']) is int and started['pid'] > 0 and started['pgid'] == started['pid']
            and type(result['exit_code']) is int and result['exit_code'] == 0
            and result['reason'] is None and result['group_absent'] is True
            and V.uint(result['cache_bytes'], 6 << 30) <= 6 << 30
            and result['stdout_sha256'] == raw[name + '-stdout']['sha256']
            and result['stderr_sha256'] == raw[name + '-stderr']['sha256']
            and result == cpu['phases'][name], 'natural CPU exit and actual retained stream joins')
        outputs[name] = store.get(raw[name + '-stdout'])
    names = {'sources-before.json', 'sources-after.json'}
    names.update(name + suffix for name, *_ in expected
                 for suffix in ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json'))
    V.require(set(raw) == names, 'all and only CPU475 raw leaves')
    runtime_names, selected, tests = P.test_inventory(outputs['runtime-list']), set(), {}
    for name, selector, count in FILTERS:
        subset = {n for n in runtime_names if selector in n}
        V.require(len(subset) == count and not subset.intersection(selected), 'disjoint runtime tests')
        selected.update(subset)
        tests[name] = P.test_results(outputs[name], subset, [[count, 0, 0]])
    worker_names = P.test_inventory(outputs['worker-list'])
    V.require(len(selected) == 77 and len(worker_names) == 402, 'actual runtime and worker inventories')
    tests['worker'] = P.test_results(outputs['worker-tests'], worker_names, [[385, 0, 4], [13, 0, 0]])
    V.require(tests == cpu['tests'] and sum(t['passed'] for t in tests.values()) == 475
        and sum(t['ignored'] for t in tests.values()) == 4, 'recomputed CPU475 outcomes')
    metadata = V.parse(outputs['metadata'])
    recorded = cpu['metadata']
    local = {p['name']: p['manifest_path'] for p in metadata['packages'] if p['source'] is None}
    expected_local = {n: str(directory / 'sources/fe2o3/crates' / n / 'Cargo.toml') for n in P.RUNTIME_CRATES}
    expected_local[P.WORKER] = str(worker)
    V.require(metadata['target_directory'] == str(directory / 'target')
        and len(metadata['packages']) == len({p['id'] for p in metadata['packages']})
        == recorded['package_count'] == 39 and local == recorded['local'] == expected_local,
        'fresh exact worker metadata graph')
    external = [dict(name=p['name'], version=p['version'], source=p['source'], manifest=p['manifest_path'])
                for p in metadata['packages'] if p['source'] is not None]
    V.require(external == [{k: row[k] for k in ('name', 'version', 'source', 'manifest')}
                           for row in recorded['external']]
        and recorded['external'] == base_cpu['metadata']['worker']['external'],
        'unchanged locked registry manifests, not live build-host reads')
    package = next(p for p in metadata['packages'] if p['name'] == 'fe2o3-kfd')
    features = next(n['features'] for n in metadata['resolve']['nodes'] if n['id'] == package['id'])
    V.require('engineering-gfx950' in features and 'live-validation' not in features, 'engineering-only feature graph')
    rows = [V.parse(line) for line in outputs['worker-build'].splitlines() if line.startswith(b'{')]
    V.require([r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True], 'actual Cargo completion')
    artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
    V.require(len(artifacts) == 1 and set(cpu['binaries']) == {P.WORKER}, 'one newly built worker only')
    artifact, emitted = artifacts[0], cpu['binaries'][P.WORKER]
    V.require(artifact == emitted['artifact'] and artifact['manifest_path'] == str(worker)
        and artifact['target']['name'] == P.WORKER and artifact['target']['kind'] == ['bin']
        and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
        and artifact['executable'] == str(directory / 'target/debug' / P.WORKER)
        and emitted['binary']['path'] == artifact['executable']
        and emitted['binary']['sha256'] == WORKER_SHA, 'CPU475 worker Cargo artifact identity')
    V.require(store.get(emitted['binary'])[:6] == b'\x7fELF\x02\x01', 'actual worker ELF64')
    return cpu, runner, overlay


def verify(D, pins, value, directory, historical):
    V.keys(value, 'schema base_deployment worker_cpu worker_cpu_review aliases runtime')
    V.require(value['schema'] == SCHEMA and value['base_deployment']['sha256'] == BASE_SHA
        and value['worker_cpu']['sha256'] == CPU_SHA, 'separate frozen base and candidate cohorts')
    original, raw = pins.read(Path(value['base_deployment']['path']), BASE_SHA, True, 16 << 20)
    V.require(original == value['base_deployment'], 'base deployment extent')
    base = V.parse(raw)
    base_dir = Path(original['path']).parent
    old_runtime = P.verify(D, pins, base, base_dir, historical)
    base_store = P.Store(D, pins, base['aliases'], base_dir)
    base_cpu = base_store.doc(base['cpu']['complete'])
    store = P.Store(D, pins, value['aliases'], directory)
    cpu, runner, overlay = cpu_evidence(store, value['worker_cpu'], base_store, base_cpu)
    review = store.doc(value['worker_cpu_review'], 65536)
    V.keys(review, 'schema reviewed authority complete runner overlay base_deployment notes '
        'owned_leaf_results_reviewed source_and_toolchain_reviewed production_authority performance_claim')
    V.require(review['schema'] == 'ferric-p228-group-fence-cpu-review-v1'
        and review['reviewed'] is True and review['authority'] == 'none'
        and review['complete'] == value['worker_cpu'] and review['runner'] == runner
        and review['overlay'] == overlay and review['base_deployment'] == original
        and type(review['notes']) is str and 0 < len(review['notes'].strip()) <= 16384
        and review['owned_leaf_results_reviewed'] is True and review['source_and_toolchain_reviewed'] is True
        and review['production_authority'] is False and review['performance_claim'] is False,
        'root-reviewed distinct worker custody')
    V.keys(value['runtime'], 'parent worker image')
    V.require(value['runtime']['parent'] == old_runtime['parent']
        and value['runtime']['image'] == old_runtime['image'], 'unchanged parent path and image')
    deployed = P.pin(value['runtime']['worker'])
    original_worker = cpu['binaries'][P.WORKER]['binary']
    V.require(deployed == dict(original_worker, path=str(directory / 'bin' / P.WORKER)), 'new worker relocation')
    actual, raw = pins.read(Path(deployed['path']), WORKER_SHA, True, 16 << 20)
    V.require(actual == deployed and raw[:6] == b'\x7fELF\x02\x01', 'actual new relocated worker ELF64')
    V.require(store.used == set(store.aliases), 'no unreviewed candidate aliases')
    V.require({p.name for p in directory.iterdir()} <= {'objects', 'bin', 'complete.json'}
        and {p.name for p in (directory / 'objects').iterdir()}
        == {row['original']['sha256'] for row in store.aliases.values()}
        and {p.name for p in (directory / 'bin').iterdir()} == {P.WORKER}, 'closed worker delta directories')
    pins.recheck()
    return dict(value['runtime'])
