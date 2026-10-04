"""CPU522 resident-state worker delta; CPU633 and CPU475 stay separate cohorts."""
from pathlib import Path

import group_fence_portable as G
import policy_portable as P
import layer_validation as V

SCHEMA = 'ferric-p228-resident-state-deployment-v1'
REVIEW_SCHEMA = 'ferric-p228-resident-state-cpu-review-v1'
CPU_SHA = '33fb539a0823dcaa988d9091d7712f40cfb0ed656c87b6e958df98f8e35ccdf9'
WORKER_SHA = '79d2b50a080a39300d02648c2844a398907ac4f89a41abff01c720ac58d42430'
PACKAGE_SHA = 'dff56a535d112d608d04a79d16bb88f3047f67eadcffa94513aa6f07c062390e'
RUNNER_SHA = 'c16fce5d5e431f893bfccc5f1124ba20cbfca2544e26501e221c3ba452fdbedc'
OVERLAY_SHA = '22ed0df3ab7c5844f8bb4e26af5495365c82508c6c1a181d451b4ae01927b8c8'
EXTRA_FILTERS = (
    ('prefix-state', 'engineering_gfx950::peer::wave_qkv_attention_output_tiles_state_v6::tests::', 10),
    ('prefix-resident', 'engineering_gfx950::peer::wave_qkv_attention_output_tiles_v6::tests::', 10),
    ('mlp-state', 'engineering_gfx950::peer::wave_mlp_tiles_state_v2::tests::', 10),
    ('mlp-resident', 'engineering_gfx950::peer::wave_mlp_tiles_v2::tests::', 10),
    ('mlp-timestamps', 'engineering_gfx950::peer::wave_mlp_tiles_v2::timestamp_tests::', 7),
)
FILTERS = G.FILTERS + EXTRA_FILTERS
NEW_TESTS = {
    'engineering_gfx950::peer::wave_qkv_attention_output_tiles_state_v6::tests::v6_private_resident_read_checks_active_phase_and_owner_before_storage',
    'engineering_gfx950::peer::wave_qkv_attention_output_tiles_state_v6::tests::v6_private_resident_read_keeps_token_kind_extent_and_mapping_refusals',
    'engineering_gfx950::peer::wave_qkv_attention_output_tiles_v6::tests::resident_v6_native_adapter_routes_to_private_guard_not_public_observer',
    'engineering_gfx950::peer::wave_mlp_tiles_state_v2::tests::v2_private_resident_read_checks_active_phase_and_owner_before_storage',
    'engineering_gfx950::peer::wave_mlp_tiles_state_v2::tests::v2_private_resident_read_keeps_token_kind_extent_and_mapping_refusals',
    'engineering_gfx950::peer::wave_mlp_tiles_v2::tests::resident_v2_native_adapter_routes_to_private_guard_not_public_observer',
}


def recipes(directory, tools):
    rows, worker = G.recipes(directory, tools)
    runtime = rows[1][1][:-4]
    V.require(rows[1][1][-4:] == ['--', '--list', '--format', 'terse'], 'unchanged runtime listing recipe')
    extras = [(name, [*runtime, selector, '--', '--test-threads=2'], 1200)
              for name, selector, _ in EXTRA_FILTERS]
    return rows[:-3] + extras + rows[-3:], worker


def checked_inputs(cpu):
    inputs = {row['path']: row for row in cpu['inputs']}
    V.require(len(inputs) == len(cpu['inputs']) == 37, 'closed CPU522 input census')
    return inputs


def source_delta(store, cpu, prior_store, prior_cpu, base_cpu):
    inputs = checked_inputs(cpu)
    for row in inputs.values():
        store.get(row)
    package_pin, overlay_pin = cpu['package_manifest'], cpu['state_overlay']
    root = Path(package_pin['path']).parent
    V.require(root.name == 'p228-resident-state-fence-cpu-v2'
        and package_pin['path'] == str(root / 'manifest.json') and package_pin['sha256'] == PACKAGE_SHA
        and overlay_pin['path'] == str(root / 'overlay.json') and overlay_pin['sha256'] == OVERLAY_SHA,
        'actual frozen CPU522 package and state overlay')
    package = store.doc(package_pin)
    V.require(package['schema'] == 'ferric-p228-resident-state-fence-cpu-package-v1'
        and len(package['files']) == 3
        and {row['path'] for row in package['files']} == {'run.py', 'overlay.json', 'README.md'},
        'closed actual CPU package')
    package_files = []
    for row in package['files']:
        V.keys(row, 'path bytes sha256')
        record = dict(row, path=str(root / row['path']))
        store.get(record)
        package_files.append(record)
    runner = next(row for row in package_files if Path(row['path']).name == 'run.py')
    V.require(runner['sha256'] == RUNNER_SHA and overlay_pin in package_files, 'actual CPU522 runner')
    overlay = store.doc(overlay_pin)
    V.keys(overlay, 'schema baseline_head source_directory preimages files')
    V.require(overlay['schema'] == 'ferric-p228-resident-state-fence-cpu-overlay-v1'
        and overlay['baseline_head'] == '725ecc6a500ff49e7dfaefb38b027f6bcc223ebf'
        and overlay['source_directory'] == 'p228-resident-state-fence-consolidation-v2'
        and len(overlay['files']) == 8, 'reviewed eight-file state delta')
    source_root = root.parent / overlay['source_directory']
    preimage_pin = dict(overlay['preimages'], path=str(source_root / 'preimages.json'))
    preimages = store.doc(preimage_pin)
    V.keys(preimages, 'schema base_commit files')
    V.require(preimages['schema'] == 'ferric-p228-resident-state-fence-source-preimages-v1'
        and preimages['base_commit'] == overlay['baseline_head'], 'state proposal source generation')
    inherited = [row for row in prior_cpu['inputs']
                 if Path(row['path']).name != 'run_group_fence_cpu_p228_v2.py']
    V.require(len(inherited) == 24, 'all historical inputs except the historical controller')
    expected = [*inherited, package_pin, *package_files, preimage_pin]
    installed = dict(prior_store.doc(prior_cpu['raw']['sources-after.json']))
    original = dict(installed)
    changed = set()
    for row in overlay['files']:
        V.keys(row, 'path before after')
        path = Path(row['path'])
        V.require(not path.is_absolute() and '..' not in path.parts and path.parts[0] == 'crates',
                  'relative runtime source path')
        name = 'fe2o3/' + row['path']
        V.require(name not in changed and row['before'] is not None
            and installed.get(name) == row['before'], 'exact CPU475 byte preimage, not a Git-label substitution')
        changed.add(name)
        record = dict(row['after'], path=str(source_root / 'source' / path))
        store.get(record)
        expected.append(record)
        installed[name] = row['after']
    V.require(preimages['files'] == {row['path']: row['before']['sha256'] for row in overlay['files']},
              'complete preimage roster')
    V.require(inputs == {row['path']: row for row in expected} and len(expected) == 37,
              'all and only inherited plus state-delta inputs')
    archives = base_cpu['archives']
    V.require(store.doc(cpu['raw']['sources-base.json']) == P.git_source_map(store, archives),
              'original exact two Git archives')
    V.require(len(installed) == 6928 and set(original) == set(installed)
        and {name for name in installed if original[name] != installed[name]} == changed,
        'only eight runtime bodies differ from CPU475')
    V.require(store.doc(cpu['raw']['sources-before.json']) == installed
        == store.doc(cpu['raw']['sources-after.json']), 'compiled source generation unchanged')
    return runner, overlay_pin


def receipt_shape(cpu):
    V.keys(cpu, 'schema passed error postcheck_errors inputs package_manifest state_overlay source_unchanged '
        'metadata phases tests binaries tests_passed tests_ignored empty_initial_target '
        'external_cargo_cache_reused gpu_execution numerical_acceptance performance_claim production_authority raw')
    V.require(cpu['schema'] == 'ferric-p228-resident-state-fence-cpu-result-v1' and cpu['passed'] is True
        and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
        and cpu['empty_initial_target'] is True and cpu['external_cargo_cache_reused'] is True
        and type(cpu['tests_passed']) is int and cpu['tests_passed'] == 522
        and type(cpu['tests_ignored']) is int and cpu['tests_ignored'] == 4
        and all(cpu[k] is False for k in ('gpu_execution', 'numerical_acceptance',
                                        'performance_claim', 'production_authority')),
        'distinct actual CPU522 result with no runtime authority')


def phase_evidence(store, cpu, directory, tools):
    raw = cpu['raw']
    expected, worker = recipes(directory, tools)
    names = {'sources-base.json', 'sources-before.json', 'sources-after.json'}
    names.update(name + suffix for name, *_ in expected
                 for suffix in ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json'))
    V.require(type(raw) is dict and len(raw) == 88 and set(raw) == names
        and set(cpu['phases']) == {row[0] for row in expected}, 'all seventeen actual CPU phases and 88 raw records')
    for name, record in raw.items():
        V.require(record['path'] == str(directory / name), 'actual raw location')
        store.get(record)
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
            'exact CPU522 bounded command and compiler environment')
        V.require(type(started['pid']) is int and started['pid'] > 0 and started['pgid'] == started['pid']
            and type(result['exit_code']) is int and result['exit_code'] == 0
            and result['reason'] is None and result['group_absent'] is True
            and V.uint(result['cache_bytes'], 6 << 30) <= 6 << 30
            and result['stdout_sha256'] == raw[name + '-stdout']['sha256']
            and result['stderr_sha256'] == raw[name + '-stderr']['sha256']
            and result == cpu['phases'][name], 'natural exit, reaped group and actual stream joins')
        outputs[name] = store.get(raw[name + '-stdout'])
    return outputs, worker


def test_evidence(outputs, cpu):
    runtime_names, selected, tests = P.test_inventory(outputs['runtime-list']), set(), {}
    for name, selector, count in FILTERS:
        subset = {n for n in runtime_names if selector in n}
        V.require(len(subset) == count and not subset.intersection(selected), 'disjoint actual runtime tests')
        selected.update(subset)
        tests[name] = P.test_results(outputs[name], subset, [[count, 0, 0]])
    worker_names = P.test_inventory(outputs['worker-list'])
    V.require(len(selected) == 124 and NEW_TESTS <= selected and len(worker_names) == 402,
              'runtime and worker inventories include every new refusal regression')
    tests['worker'] = P.test_results(outputs['worker-tests'], worker_names, [[385, 0, 4], [13, 0, 0]])
    V.require(tests == cpu['tests'] and sum(t['passed'] for t in tests.values()) == 522
        and sum(t['ignored'] for t in tests.values()) == 4, 'recomputed CPU522 outcomes')


def artifact_evidence(store, cpu, outputs, directory, worker, base_cpu):
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
    V.require('engineering-gfx950' in features and 'live-validation' not in features, 'engineering-only graph')
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
        and emitted['binary']['bytes'] == 4780024 and emitted['binary']['sha256'] == WORKER_SHA,
        'CPU522 worker Cargo artifact, never the old CPU475 worker')
    V.require(store.get(emitted['binary'])[:6] == b'\x7fELF\x02\x01', 'actual worker ELF64')


def cpu_evidence(store, complete, prior_store, prior_cpu, base_cpu):
    cpu = store.doc(complete)
    receipt_shape(cpu)
    directory = Path(complete['path']).parent
    V.require(directory.name == 'resident-state-fence-cpu-v228-v2', 'actual CPU522 output')
    runner, overlay = source_delta(store, cpu, prior_store, prior_cpu, base_cpu)
    outputs, worker = phase_evidence(store, cpu, directory, {'worker': base_cpu['toolchains']['worker']})
    test_evidence(outputs, cpu)
    artifact_evidence(store, cpu, outputs, directory, worker, base_cpu)
    return cpu, runner, overlay


def read_document(D, pins, record):
    P.pin(record)
    actual, raw = pins.read(Path(record['path']), record['sha256'], True, 16 << 20)
    V.require(actual == record, 'actual deployment extent')
    return D.parse(raw)


def verify(D, pins, value, directory, historical):
    V.keys(value, 'schema base_deployment prior_deployment worker_cpu worker_cpu_review aliases runtime')
    V.require(value['schema'] == SCHEMA and value['base_deployment']['sha256'] == G.BASE_SHA
        and value['worker_cpu']['sha256'] == CPU_SHA and value['worker_cpu']['bytes'] == 120723,
        'explicit CPU633, CPU475 and CPU522 cohorts')
    prior = read_document(D, pins, value['prior_deployment'])
    prior_dir = Path(value['prior_deployment']['path']).parent
    old_runtime = G.verify(D, pins, prior, prior_dir, historical)
    V.require(prior['base_deployment'] == value['base_deployment'], 'unchanged CPU633 deployment')
    base = read_document(D, pins, value['base_deployment'])
    base_store = P.Store(D, pins, base['aliases'], Path(value['base_deployment']['path']).parent)
    base_cpu = base_store.doc(base['cpu']['complete'])
    prior_store = P.Store(D, pins, prior['aliases'], prior_dir)
    prior_cpu = prior_store.doc(prior['worker_cpu'])
    store = P.Store(D, pins, value['aliases'], directory)
    cpu, runner, overlay = cpu_evidence(store, value['worker_cpu'], prior_store, prior_cpu, base_cpu)
    review = store.doc(value['worker_cpu_review'], 65536)
    V.keys(review, 'schema reviewed authority complete runner overlay package_manifest base_deployment '
        'prior_deployment notes owned_leaf_results_reviewed source_and_toolchain_reviewed '
        'production_authority performance_claim')
    V.require(review['schema'] == REVIEW_SCHEMA and review['reviewed'] is True and review['authority'] == 'none'
        and review['complete'] == value['worker_cpu'] and review['runner'] == runner
        and review['overlay'] == overlay and review['package_manifest'] == cpu['package_manifest']
        and review['base_deployment'] == value['base_deployment']
        and review['prior_deployment'] == value['prior_deployment']
        and type(review['notes']) is str and 0 < len(review['notes'].strip()) <= 16384
        and review['owned_leaf_results_reviewed'] is True and review['source_and_toolchain_reviewed'] is True
        and review['production_authority'] is False and review['performance_claim'] is False,
        'actual root review of new worker custody, no new runtime authority')
    V.keys(value['runtime'], 'parent worker image')
    V.require(value['runtime']['parent'] == old_runtime['parent']
        and value['runtime']['image'] == old_runtime['image'], 'unchanged parent and historical image')
    deployed = P.pin(value['runtime']['worker'])
    original_worker = cpu['binaries'][P.WORKER]['binary']
    V.require(deployed == dict(original_worker, path=str(directory / 'bin' / P.WORKER))
        and deployed['sha256'] != old_runtime['worker']['sha256'], 'new worker relocation only')
    actual, raw = pins.read(Path(deployed['path']), WORKER_SHA, True, 16 << 20)
    V.require(actual == deployed and raw[:6] == b'\x7fELF\x02\x01', 'actual relocated worker ELF64')
    V.require(store.used == set(store.aliases), 'no unreviewed CPU522 aliases')
    V.require({p.name for p in directory.iterdir()} <= {'objects', 'bin', 'complete.json'}
        and {p.name for p in (directory / 'objects').iterdir()}
        == {row['original']['sha256'] for row in store.aliases.values()}
        and {p.name for p in (directory / 'bin').iterdir()} == {P.WORKER}, 'closed worker delta directories')
    pins.recheck()
    return dict(value['runtime'])


def deployment(D, pins, record, historical):
    value = read_document(D, pins, record)
    return value, verify(D, pins, value, Path(record['path']).parent, historical)


def records(cpu_pin, cpu, review):
    result = {}
    for row in [cpu_pin, review, *cpu['inputs'], *cpu['raw'].values(),
                *(row['binary'] for row in cpu['binaries'].values())]:
        P.pin(row)
        V.require(result.setdefault(row['path'], row) == row, 'no conflicting original identities')
    V.require(len(result) == 128, 'actual 37-input, 88-raw, one-worker courier census')
    return result
