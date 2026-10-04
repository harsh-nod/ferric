"""Additive paired bank-worker proof; inherited CPU522 evidence stays immutable."""
from collections import Counter
from pathlib import Path
import re

import resident_state_portable as S
import policy_portable as P
import layer_validation as V

SCHEMA = 'ferric-p228-state-bank-batch-deployment-v1'
REVIEW_SCHEMA = 'ferric-p228-state-bank-batch-cpu-review-v1'
# These identify the frozen source package, not a predicted CPU or GPU result.
PACKAGE_SHA = 'f882275f45f49bd3b999422f668c7ccfee3c1b47d173a98c17a19e5887551b49'
RUNNER_SHA = 'd5c2237645915cd965aeb46fadf70130486abfc894fffafc9fc3513f277925da'
OVERLAY_SHA = '73b74e8fe343ecdad622e8e1a4967b9bb096f21ead6992f82d8dd02b797ef8cb'
RUNTIME_ROOT = 'crates/fe2o3-kfd/src/'
FERRIC_ROOT = 'adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/prefix_tiles_decode_v6/'
RUNTIME_DIR = 'p228-state-bank-batch-runtime-v2'
FERRIC_DIR = 'p228-state-bank-batch-ferric-v2'
RUNTIME_FILES = ('lib.rs', 'engineering_gfx950.rs', 'engineering_gfx950_peer.rs',
    'engineering_gfx950_peer_wave_mlp_tiles_state_v2.rs',
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_state_v6.rs',
    'engineering_gfx950_peer_state_bank_v1.rs', 'engineering_gfx950_peer_state_bank_v1_tests.rs')
FERRIC_FILES = ('reuse.rs', 'tests.rs', 'bank_batch_tests.rs')
BANK_SELECTOR = 'engineering_gfx950::peer::state_bank_v1::tests::'
BANK_TESTS = {BANK_SELECTOR + name for name in (
    'bank_mixed_order_whole_validation_then_reads_and_two_fresh_checks',
    'bank_bounds_one_and144_accept_empty_and145_refuse_before_backend',
    'bank_duplicates_refuse_before_any_load_and_quarantine',
    'bank_first_middle_last_validation_read_and_both_fence_failures_return_no_vector',
    'bank_repeated_calls_take_new_fences_and_later_failure_quarantines',
    'bank_public_bounds_error_poison_and_public_single_fences_remain_guarded',
    'bank_native_routing_accepts_completed_lifetime_but_refuses_missing_owner_storage',
    'bank_native_rejects_unconstructed_lifetimes_without_storage_access',
    'bank_native_prefix_foreign_stale_kind_extent_and_mapping_checks_precede_storage',
    'bank_native_mlp_foreign_stale_kind_extent_and_mapping_checks_precede_storage')}
WORKER_SELECTOR = 'state_roster::prefix_tiles_decode_v6::reuse::bank_batch_tests::'
WORKER_TESTS = {WORKER_SELECTOR + name for name in (
    'bank_gather_selects_exact36_layers_and_prefix_then_mlp_rank_order',
    'bank_gather_rejects_wrong_roster_or_bank_before_any_state_access',
    'bank_gather_stops_on_each_failed_entry_without_returning_partial_requests',
    'bank_gather_owner_refusal_keeps_last_rank_from_entering_runtime_batch',
    'bank_decode_retains_every_word_in_both_typed_rank_arrays',
    'bank_decode_refuses_missing_extra_and_every_wrong_typed_slot',
    'bank_ledger_uses_two_snapshots_and_preserves_outer_fences_and144_rearms',
    'bank_ledger_fresh_generation_still_performs_both_scans_without_stores',
    'bank_ledger_each_failed_snapshot_is_terminal_without_publishing_generation',
    'bank_ledger_rejects_missing_or_extra_layers_in_either_scan',
    'bank_ledger_last_state_corruption_never_escapes_either_complete_scan')}
EXTRA_FILTERS = (
    ('state-bank', BANK_SELECTOR, 10),
    ('memory-prefix', 'memory_linux::wave_qkv_attention_output_tiles_v6::tests::', 4),
    ('memory-mlp', 'memory_linux::wave_mlp_tiles_v2::tests::', 4),
    ('memory-rearm', 'memory_linux::state_rearm_tests::', 2),
)
FILTERS = S.FILTERS + EXTRA_FILTERS
FILES = {'bank_batch_portable.py', 'export_bank_batch.py', 'audit_bank_batch_runtime.py',
         'test_bank_batch_portable.py'}


def frozen_source_gate():
    V.require(all(type(value) is str and re.fullmatch('[0-9a-f]{64}', value)
                  for value in (PACKAGE_SHA, RUNNER_SHA, OVERLAY_SHA)),
              'root must bind the actual frozen CPU package, runner and overlay')


def checked_package(I, pins):
    manifest, record = I.package_record(pins)
    directory = Path(I.__file__).resolve().parent
    V.require(directory == Path(__file__).resolve().parent
        and FILES <= {row['path'] for row in manifest['files']},
        'successor intake must authenticate every bank deployment helper')
    return manifest, record


def recipes(directory, tools):
    rows, worker = S.recipes(directory, tools)
    runtime = rows[1][1][:-4]
    V.require(rows[1][1][-4:] == ['--', '--list', '--format', 'terse'], 'inherited listing recipe')
    extra = [(name, [*runtime, selector, '--', '--test-threads=2'], 1200)
             for name, selector, _ in EXTRA_FILTERS]
    return rows[:-3] + extra + rows[-3:], worker


def unique_pins(rows):
    result = {}
    for row in rows:
        P.pin(row)
        V.require(result.setdefault(row['path'], row) == row, 'conflicting original input identity')
    return result


def exact_inputs(actual, expected):
    a, b = unique_pins(actual), unique_pins(expected)
    V.require(len(actual) == len(expected) == 151 and len(a) == len(b) == 144
        and a == b and Counter(row['path'] for row in actual) == Counter(row['path'] for row in expected),
        'closed151 input entries with144 identities and exact inherited multiplicity')
    return a


def overlay_shape(value, evidence):
    V.keys(value, 'schema runtime_preimages ferric_source_manifest files')
    V.require(value['schema'] == 'ferric-p228-state-bank-batch-cpu-overlay-v1'
        and P.pin(value['runtime_preimages'])['path'] == str(evidence / RUNTIME_DIR / 'preimages.json')
        and P.pin(value['ferric_source_manifest'])['path'] == str(evidence / FERRIC_DIR / 'source-pins.json'),
        'exact formatted paired proposal namespace')
    destinations = {('fe2o3', RUNTIME_ROOT + name): RUNTIME_DIR + '/source/' + name for name in RUNTIME_FILES}
    destinations.update({('ferric', FERRIC_ROOT + name): FERRIC_DIR + '/draft/' + FERRIC_ROOT + name
                         for name in FERRIC_FILES})
    V.require(type(value['files']) is list and len(value['files']) == 10, 'ten paired source bodies')
    seen, added = set(), set()
    for row in value['files']:
        V.keys(row, 'project path source before after')
        key = row['project'], row['path']
        V.require(key in destinations and key not in seen and row['source'] == destinations[key],
                  'exact distinct source/destination, no path traversal')
        seen.add(key)
        for pin in (row['before'], row['after']):
            if pin is not None:
                V.keys(pin, 'bytes sha256')
                P.pin(dict(pin, path='/content-only'))
        V.require(row['after'] is not None, 'source body cannot be removed')
        if row['before'] is None:
            added.add(key)
    expected_new = {('fe2o3', RUNTIME_ROOT + name) for name in RUNTIME_FILES[-2:]}
    expected_new.add(('ferric', FERRIC_ROOT + 'bank_batch_tests.rs'))
    V.require(added == expected_new, 'only the two runtime and one Ferric new files')
    return value


def apply_source_map(prior, rows):
    V.require(type(prior) is dict and len(prior) == 6928, 'actual qualified CPU522 source inventory')
    result, changed, added = dict(prior), set(), set()
    for row in rows:
        key = row['project'] + '/' + row['path']
        V.require(key not in changed and result.get(key) == row['before'], 'exact preceding byte preimage')
        changed.add(key)
        if row['before'] is None:
            V.require(key not in result, 'new source path must be absent')
            added.add(key)
        else:
            V.require(row['before'] != row['after'], 'each declared replacement must change bytes')
        result[key] = row['after']
    V.require(len(result) == 6931 and len(changed) == 10 and len(added) == 3
        and {key for key in prior if prior[key] != result[key]} == changed - added,
        'only seven replacements and three additions to the qualified source')
    return result


def source_delta(store, cpu, prior_store, prior_cpu, prior_pin, base_cpu):
    frozen_source_gate()
    package_pin, overlay_pin = cpu['package_manifest'], cpu['overlay']
    root = Path(package_pin['path']).parent
    evidence = root.parent
    V.require(root.name == 'p228-state-bank-batch-cpu-v1'
        and package_pin['path'] == str(root / 'manifest.json') and package_pin['sha256'] == PACKAGE_SHA
        and overlay_pin['path'] == str(root / 'overlay.json') and overlay_pin['sha256'] == OVERLAY_SHA,
        'actual frozen CPU553 source package')
    package = store.doc(package_pin)
    V.require(package['schema'] == 'ferric-p228-state-bank-batch-cpu-package-v1'
        and len(package['files']) == 4 and {r['path'] for r in package['files']}
        == {'run.py', 'overlay.json', 'test_run.py', 'README.md'}, 'exact four-file CPU controller closure')
    members = []
    for row in package['files']:
        V.keys(row, 'path bytes sha256')
        pin = dict(row, path=str(root / row['path']))
        store.get(pin)
        members.append(pin)
    runner = next(row for row in members if Path(row['path']).name == 'run.py')
    V.require(runner['sha256'] == RUNNER_SHA and overlay_pin in members, 'frozen bounded compiler controller')
    overlay = overlay_shape(store.doc(overlay_pin), evidence)
    preimages = store.doc(overlay['runtime_preimages'])
    V.keys(preimages, 'schema base_commit files new_files')
    runtime_rows = [row for row in overlay['files'] if row['project'] == 'fe2o3']
    V.require(preimages['schema'] == 'ferric-p228-state-bank-batch-source-preimages-v1'
        and preimages['base_commit'] == '36734d3bc28ed038019d9a16da01c8533cdd83c9'
        and preimages['files'] == {row['path']: row['before']['sha256'] for row in runtime_rows if row['before']}
        and sorted(preimages['new_files']) == sorted(row['path'] for row in runtime_rows if row['before'] is None),
        'exact runtime preimages and new paths')
    ferric = store.doc(overlay['ferric_source_manifest'])
    ferric_rows = [row for row in overlay['files'] if row['project'] == 'ferric']
    V.require(ferric['schema'] == 'ferric-p228-state-bank-batch-source-proposal-v1'
        and ferric['tests_executed'] is False and type(ferric['new_tests_authored']) is int
        and ferric['new_tests_authored'] == 11 and len(ferric['files']) == 3
        and {r['path']: {k: r[k] for k in ('before', 'after')} for r in ferric['files']}
        == {r['path']: {k: r[k] for k in ('before', 'after')} for r in ferric_rows},
        'paired Ferric proposal is source-only, not its later qualification')
    old_inputs = unique_pins(prior_cpu['inputs'])
    inherited_paths = [evidence / 'p228-host-policy-cpu-v1/run.py',
        evidence / 'p228-resident-state-fence-cpu-v2/run.py',
        evidence / 'clean-worker-source-inputs-v228-v1.json', evidence / 'run_clean_worker_p228_v1.py',
        evidence.parent / 'wave-output-lowering-v216/bounded.py']
    expected = [package_pin, *members, *(old_inputs[str(path)] for path in inherited_paths),
        prior_pin, *prior_cpu['inputs'], *prior_cpu['raw'].values(),
        prior_cpu['binaries'][P.WORKER]['binary'], overlay['runtime_preimages'], overlay['ferric_source_manifest']]
    for row in overlay['files']:
        pin = dict(row['after'], path=str(evidence / row['source']))
        store.get(pin)
        expected.append(pin)
    expected.extend({k: row[k] for k in ('path', 'bytes', 'sha256')} for row in base_cpu['archives'].values())
    for path, pin in exact_inputs(cpu['inputs'], expected).items():
        target = prior_store if path in prior_store.aliases else store
        target.get(pin)
    V.require(cpu['prior_cpu_complete'] == prior_pin, 'actual qualified CPU522 predecessor')
    prior_source = prior_store.doc(prior_cpu['raw']['sources-after.json'])
    V.require(store.doc(cpu['raw']['sources-base.json']) == prior_store.doc(prior_cpu['raw']['sources-base.json'])
        and store.doc(cpu['raw']['sources-prior.json']) == prior_source,
        'same authenticated Git archives and complete CPU522 generation')
    installed = apply_source_map(prior_source, overlay['files'])
    V.require(store.doc(cpu['raw']['sources-before.json']) == installed
        == store.doc(cpu['raw']['sources-after.json']), 'actual compiled6931 sources unchanged')
    return runner, overlay_pin


def receipt_shape(cpu):
    V.keys(cpu, 'schema passed error postcheck_errors inputs package_manifest overlay prior_cpu_complete source_unchanged '
        'metadata phases tests binaries tests_passed tests_ignored empty_initial_target '
        'external_cargo_cache_reused gpu_execution numerical_acceptance performance_claim production_authority raw')
    V.require(cpu['schema'] == 'ferric-p228-state-bank-batch-cpu-result-v1' and cpu['passed'] is True
        and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
        and cpu['empty_initial_target'] is True and cpu['external_cargo_cache_reused'] is True
        and type(cpu['tests_passed']) is int and cpu['tests_passed'] == 553
        and type(cpu['tests_ignored']) is int and cpu['tests_ignored'] == 4
        and all(cpu[k] is False for k in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority')),
        'actual CPU553 success, without runtime or numerical authority')


def phase_evidence(store, cpu, directory, tools):
    raw = cpu['raw']
    expected, worker = recipes(directory, tools)
    names = {'sources-base.json', 'sources-prior.json', 'sources-before.json', 'sources-after.json'}
    names.update(name + suffix for name, *_ in expected
        for suffix in ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json'))
    V.require(len(expected) == 21 and type(raw) is dict and len(raw) == 109 and set(raw) == names
        and set(cpu['phases']) == {row[0] for row in expected}, 'all21 phases and109 actual raw records')
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
            'same bounded commands, compiler routing, offline cache and visibility')
        V.require(type(started['pid']) is int and started['pid'] > 0 and started['pgid'] == started['pid']
            and type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
            and result['group_absent'] is True and V.uint(result['cache_bytes'], 6 << 30) <= 6 << 30
            and result['stdout_sha256'] == raw[name + '-stdout']['sha256']
            and result['stderr_sha256'] == raw[name + '-stderr']['sha256']
            and result == cpu['phases'][name], 'natural exit, reaped group and retained stream identities')
        outputs[name] = store.get(raw[name + '-stdout'])
    return outputs, worker


def exact_extension(actual, prior, added):
    V.require(not prior.intersection(added) and actual == prior | added,
              'actual compiled inventory extends only by the exact new named tests')


def ignored_names(raw):
    return set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored(?:, [^\n]*)?$', raw.decode(), re.MULTILINE))


def test_evidence(outputs, cpu, prior_store, prior_cpu):
    prior_runtime = P.test_inventory(prior_store.get(prior_cpu['raw']['runtime-list-stdout']))
    runtime = P.test_inventory(outputs['runtime-list'])
    exact_extension(runtime, prior_runtime, BANK_TESTS)
    selected, tests = set(), {}
    for name, selector, count in FILTERS:
        subset = {n for n in runtime if selector in n}
        V.require(len(subset) == count and not subset.intersection(selected), 'disjoint runtime cohorts')
        V.require(subset == (BANK_TESTS if name == 'state-bank' else {n for n in prior_runtime if selector in n}),
                  'unchanged prior or exact new runtime tests')
        selected.update(subset)
        tests[name] = P.test_results(outputs[name], subset, [[count, 0, 0]])
    prior_worker = P.test_inventory(prior_store.get(prior_cpu['raw']['worker-list-stdout']))
    worker = P.test_inventory(outputs['worker-list'])
    exact_extension(worker, prior_worker, WORKER_TESTS)
    V.require(len(selected) == 144 and S.NEW_TESTS | BANK_TESTS <= selected and len(worker) == 413,
              'all old regressions,10 bank and11 Ferric tests compiled')
    tests['worker'] = P.test_results(outputs['worker-tests'], worker, [[396, 0, 4], [13, 0, 0]])
    prior_ignored = ignored_names(prior_store.get(prior_cpu['raw']['worker-tests-stdout']))
    V.require(ignored_names(outputs['worker-tests']) == prior_ignored and not WORKER_TESTS.intersection(prior_ignored),
              'same four ignored tests, none of the new tests ignored')
    V.require(tests == cpu['tests'] and sum(t['passed'] for t in tests.values()) == 553
        and sum(t['ignored'] for t in tests.values()) == 4, 'recomputed553 outcomes, not declared counts alone')


def artifact_evidence(store, cpu, outputs, directory, worker, prior_cpu):
    metadata = V.parse(outputs['metadata'])
    recorded = cpu['metadata']
    local = {p['name']: p['manifest_path'] for p in metadata['packages'] if p['source'] is None}
    expected_local = {n: str(directory / 'sources/fe2o3/crates' / n / 'Cargo.toml') for n in P.RUNTIME_CRATES}
    expected_local[P.WORKER] = str(worker)
    V.require(metadata['target_directory'] == str(directory / 'target')
        and len(metadata['packages']) == len({p['id'] for p in metadata['packages']}) == recorded['package_count'] == 39
        and local == recorded['local'] == expected_local, 'fresh unchanged39-package engineering graph')
    external = [dict(name=p['name'], version=p['version'], source=p['source'], manifest=p['manifest_path'])
                for p in metadata['packages'] if p['source'] is not None]
    V.require(external == [{k: r[k] for k in ('name', 'version', 'source', 'manifest')} for r in recorded['external']]
        and recorded['external'] == prior_cpu['metadata']['external'], 'unchanged authenticated dependency manifests')
    package = next(p for p in metadata['packages'] if p['name'] == 'fe2o3-kfd')
    features = next(n['features'] for n in metadata['resolve']['nodes'] if n['id'] == package['id'])
    V.require('engineering-gfx950' in features and 'live-validation' not in features, 'engineering-only graph')
    worker_artifact(store, cpu, outputs['worker-build'], directory, worker, prior_cpu)


def worker_artifact(store, cpu, raw, directory, worker, prior_cpu):
    rows = [V.parse(line) for line in raw.splitlines() if line.startswith(b'{')]
    V.require([r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True], 'actual Cargo completion')
    artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
    V.require(len(artifacts) == 1 and set(cpu['binaries']) == {P.WORKER}, 'exactly one actual new worker artifact')
    artifact, emitted = artifacts[0], cpu['binaries'][P.WORKER]
    binary = P.pin(emitted['binary'])
    V.require(artifact == emitted['artifact'] and artifact['manifest_path'] == str(worker)
        and artifact['target']['name'] == P.WORKER and artifact['target']['kind'] == ['bin']
        and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
        and artifact['executable'] == str(directory / 'target/debug' / P.WORKER)
        and binary['path'] == artifact['executable'] and 0 < binary['bytes'] <= 16 << 20
        and binary['sha256'] != prior_cpu['binaries'][P.WORKER]['binary']['sha256'],
        'derive the new binary pin only from the authentic Cargo result, not a placeholder')
    V.require(store.get(binary, 16 << 20)[:6] == b'\x7fELF\x02\x01', 'actual new worker ELF64LE')
    return binary


def cpu_evidence(store, complete, prior_store, prior_cpu, prior_pin, base_cpu):
    cpu = store.doc(complete)
    receipt_shape(cpu)
    directory = Path(complete['path']).parent
    V.require(re.fullmatch(r'state-bank-batch-cpu-v228-v[1-9][0-9]*', directory.name)
        and complete['path'] == str(directory / 'complete.json'), 'actual successful new CPU namespace')
    runner, overlay = source_delta(store, cpu, prior_store, prior_cpu, prior_pin, base_cpu)
    outputs, worker = phase_evidence(store, cpu, directory, {'worker': base_cpu['toolchains']['worker']})
    test_evidence(outputs, cpu, prior_store, prior_cpu)
    artifact_evidence(store, cpu, outputs, directory, worker, prior_cpu)
    return cpu, runner, overlay


def verify(D, pins, value, directory, historical):
    frozen_source_gate()
    V.keys(value, 'schema base_deployment prior_deployment worker_cpu worker_cpu_review aliases runtime')
    V.require(value['schema'] == SCHEMA and value['base_deployment']['sha256'] == S.G.BASE_SHA
        and re.fullmatch(r'state-bank-batch-deployment-v228-v[1-9][0-9]{0,8}', directory.name),
        'additive CPU553 deployment namespace')
    prior = S.read_document(D, pins, value['prior_deployment'])
    prior_dir = Path(value['prior_deployment']['path']).parent
    old_runtime = S.verify(D, pins, prior, prior_dir, historical)
    V.require(prior['base_deployment'] == value['base_deployment']
        and prior['worker_cpu']['sha256'] == S.CPU_SHA, 'unchanged CPU633 base and actual CPU522 prior')
    prior_store = P.Store(D, pins, prior['aliases'], prior_dir)
    prior_cpu = prior_store.doc(prior['worker_cpu'])
    base = S.read_document(D, pins, value['base_deployment'])
    base_store = P.Store(D, pins, base['aliases'], Path(value['base_deployment']['path']).parent)
    base_cpu = base_store.doc(base['cpu']['complete'])
    V.require(not set(value['aliases']).intersection(prior['aliases']), 'only new objects in the additive courier')
    store = P.Store(D, pins, value['aliases'], directory)
    cpu, runner, overlay = cpu_evidence(store, value['worker_cpu'], prior_store, prior_cpu, prior['worker_cpu'], base_cpu)
    review = store.doc(value['worker_cpu_review'], 65536)
    V.keys(review, 'schema reviewed authority complete runner overlay package_manifest base_deployment prior_deployment '
        'prior_cpu_complete runtime_preimages ferric_source_manifest notes owned_leaf_results_reviewed '
        'source_and_toolchain_reviewed production_authority performance_claim')
    pair = store.doc(overlay)
    V.require(review['schema'] == REVIEW_SCHEMA and review['reviewed'] is True and review['authority'] == 'none'
        and review['complete'] == value['worker_cpu'] and review['runner'] == runner and review['overlay'] == overlay
        and review['package_manifest'] == cpu['package_manifest'] and review['base_deployment'] == value['base_deployment']
        and review['prior_deployment'] == value['prior_deployment'] and review['prior_cpu_complete'] == prior['worker_cpu']
        and review['runtime_preimages'] == pair['runtime_preimages']
        and review['ferric_source_manifest'] == pair['ferric_source_manifest']
        and type(review['notes']) is str and 0 < len(review['notes'].strip()) <= 16384
        and review['owned_leaf_results_reviewed'] is True and review['source_and_toolchain_reviewed'] is True
        and review['production_authority'] is False and review['performance_claim'] is False,
        'actual root review binds both paired sources and real CPU result, no runtime authority')
    V.keys(value['runtime'], 'parent worker image')
    V.require(value['runtime']['parent'] == old_runtime['parent'] and value['runtime']['image'] == old_runtime['image'],
              'unchanged parent and historical image')
    deployed = P.pin(value['runtime']['worker'])
    original = cpu['binaries'][P.WORKER]['binary']
    V.require(deployed == dict(original, path=str(directory / 'bin' / P.WORKER))
        and deployed['sha256'] != old_runtime['worker']['sha256'], 'relocation of the actual new worker only')
    actual, raw = pins.read(Path(deployed['path']), deployed['sha256'], True, 16 << 20)
    V.require(actual == deployed and raw[:6] == b'\x7fELF\x02\x01', 'actual relocated ELF64LE')
    V.require(len(store.aliases) == 129 and store.used == set(store.aliases), 'all and only129 new artifact identities consumed')
    V.require({p.name for p in directory.iterdir()} <= {'objects', 'bin', 'complete.json'}
        and {p.name for p in (directory / 'objects').iterdir()} == {r['original']['sha256'] for r in store.aliases.values()}
        and {p.name for p in (directory / 'bin').iterdir()} == {P.WORKER}, 'closed additive directories')
    pins.recheck()
    return dict(value['runtime'])


def deployment(D, pins, record, historical):
    value = S.read_document(D, pins, record)
    return value, verify(D, pins, value, Path(record['path']).parent, historical)


def records(cpu_pin, cpu, review, prior_aliases):
    receipt_shape(cpu)
    inherited = unique_pins([row['original'] for row in prior_aliases.values()])
    inputs = unique_pins(cpu['inputs'])
    V.require(len(cpu['inputs']) == 151 and len(inputs) == 144, 'actual new input roster')
    fresh = []
    for path, pin in inputs.items():
        if path in inherited:
            V.require(pin == inherited[path], 'inherited original input identity')
        else:
            fresh.append(pin)
    V.require(len(fresh) == 17 and len(cpu['raw']) == 109 and set(cpu['binaries']) == {P.WORKER},
              '17new inputs,109raw records and one actual worker')
    result = unique_pins([cpu_pin, review, *fresh, *cpu['raw'].values(), cpu['binaries'][P.WORKER]['binary']])
    V.require(len(result) == 129 and not set(result).intersection(inherited), 'bounded delta-only courier census')
    return result
