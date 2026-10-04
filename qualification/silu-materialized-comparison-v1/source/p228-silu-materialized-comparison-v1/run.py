"""Bounded CPU-only comparison of genuine retained SiLU candidate captures."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import types
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-silu-materialized-comparison-v1'
PRIOR_RUN = E / 'p228-projection-residual-comparison-run-v1/run.py'
PRIOR_RUN_SHA = '08bb62113ff319662dea2b65d1c04b77b023c9397495e205b582c2bafb32cd0c'
PRIOR = E / 'projection-residual-comparison-v228-v1/complete.json'
PRIOR_SHA = 'affe711d0dcac8e95609396c74e0f546ca09d9d6a4abf798eab5bfc72a7e7551'
GPU = E / 'p228-silu-materialized-capture-gpu-v1'
GPU_SHA = '71cae69a25ac82c53bdf2976ab53b9d9af615c5af4b8337c2e386beb44a8a18b'
SILU = E / 'p228-silu-materialization-diagnostic-v1/diagnostic.py'
SILU_SHA = '19ac2cf361d5653b4438b77d5a7f5501961ffe1dcabb2606fbf681b5175ab3f8'
PURE_SCHEMA = 'ferric-p228-silu-materialized-comparison-pure-v1'
RESULT_SCHEMA = 'ferric-p228-silu-materialized-comparison-observation-v1'
FILES = {'silu.py', 'test_silu.py', 'run.py', 'README.md'}
TESTS = {
    'test_reused_arithmetic_and_residual_oracle_are_exact_pinned_sources',
    'test_complete_synthetic_path_retains_all_diagnostics_without_acceptance',
    'test_only_selected_mlp_image_and_scoped_identifiers_may_change',
    'test_old_image_wrong_residual_worker_or_reused_session_are_refused',
    'test_every_pre_swiglu_stage_is_equal_not_only_prefix',
    'test_logical_kv_row_can_move_between_authenticated_physical_pages',
    'test_changed_activation_mismatches_remain_visible_and_conditional',
    'test_changed_down_and_final_hidden_use_actual_candidate_residual',
    'test_incorrect_residual_output_is_reported_not_reclassified_as_success',
    'test_nonfinite_capture_partial_is_rejected_before_metrics',
    'test_fp32_partial_changes_distinguish_signed_zero_and_subnormals',
    'test_partial_extent_and_nonfinite_inputs_are_refused',
}


def require(ok, why):
    if not ok:
        raise ValueError(why)


def bootstrap(mode):
    require(not sys.flags.optimize and sys.dont_write_bytecode
            and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
            'ordinary isolated -B Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
            and all(os.environ.get(k) == '' for k in
                    ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
            'ASROCK bounded CPU-only environment')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 180 if mode == 'pure' else 300),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        values = [v for v in (*resource.getrlimit(kind), cap) if v != resource.RLIM_INFINITY]
        resource.setrlimit(kind, (min(values), min(values)))
    require(PRIOR_RUN.resolve(strict=True) == PRIOR_RUN, 'canonical previous controller')
    with os.fdopen(os.open(PRIOR_RUN, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and len(raw) == before.st_size <= 1 << 20
            and stamp(before) == stamp(after) == stamp(PRIOR_RUN.lstat())
            and hashlib.sha256(raw).hexdigest() == PRIOR_RUN_SHA, 'exact retained controller')
    old = types.ModuleType('retained_projection_runner'); old.__file__ = str(PRIOR_RUN)
    exec(compile(raw, str(PRIOR_RUN), 'exec'), old.__dict__)
    return old, old.bootstrap()


@contextlib.contextmanager
def aliases(values):
    absent = object(); previous = {name: sys.modules.get(name, absent) for name in values}
    try:
        sys.modules.update(values)
        yield
    finally:
        for name, value in previous.items():
            if value is absent:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = value


def package(J, reader, directory, digest, schema, names):
    pin = J.actual(directory / 'manifest.json', digest)[0]
    value = reader.doc(pin)
    require(value['schema'] == schema and len(value['files']) == len(names)
            and {row['path'] for row in value['files']} == names, 'closed frozen source package')
    rows = {row['path']: dict(row, path=str(directory / row['path'])) for row in value['files']}
    for record in rows.values():
        reader(record)
    return pin, value, rows


def setup(J, package_sha, additions=None):
    prior_pin, prior_raw = J.actual(PRIOR, PRIOR_SHA)
    prior = J.parse(prior_raw)
    require(prior['schema'] == 'ferric-p228-projection-residual-comparison-observation-v1'
            and prior['completed'] is True and prior['source_postchecks_passed'] is True
            and prior['native_structural_replayed'] is True
            and prior['candidate_owned_leaves_replayed'] == 7 and prior['candidate_audits_replayed'] == 6
            and prior['framework_owned_leaf_results_rechecked'] == 21
            and prior['numerical_acceptance'] is False, 'actual previously qualified corrected comparison')
    mapping = {row['original']['path']: row['retained'] for row in prior['consumed']
               if row['original']['path'] != row['retained']['path']}
    addition_pin = None
    if additions is not None:
        addition_pin, raw = J.actual(Path(additions[0]), additions[1])
        value = J.parse(raw); require(type(value) is dict, 'explicit transport additions')
        for path, record in value.items():
            require(path not in mapping or mapping[path] == record, 'immutable previous transport mapping')
            mapping[path] = record
    reader = J.Reader(mapping)
    for pin in (prior_pin, prior['controller'], J.actual(PRIOR_RUN, PRIOR_RUN_SHA)[0],
                J.actual(J.__file__)[0]):
        reader(pin)
    if addition_pin is not None:
        reader(addition_pin)
    manifest, _, sources = package(J, reader, PACKAGE, package_sha,
        'ferric-p228-silu-materialized-comparison-package-v1', FILES)
    dependencies = dict(prior['comparison_sources'])
    require(len(dependencies) == 8, 'unchanged exact residual/reference dependency roster')
    dependencies['p228-silu-materialization-diagnostic-v1/diagnostic.py'] = J.actual(SILU, SILU_SHA)[0]
    for pin in dependencies.values():
        reader(pin)
    for pin in sources.values():
        reader(pin)
    source_map = dict(dependencies)
    source_map.update({PACKAGE.name + '/' + key: pin for key, pin in sources.items()})
    source_map['previous-controller'] = prior['controller']
    source_map['previous-reader-controller'] = J.actual(J.__file__)[0]
    get = lambda key: dependencies[key]
    C = J.module(reader, get('p228-layer0-current-diagnostic-v1/compare.py'), 'silu_genuine_compare')
    K = J.module(reader, get('p228-layer0-current-diagnostic-v1/current.py'), 'silu_current', {'compare': C})
    B = J.module(reader, get('p228-output-residual-boundary-v1/boundary.py'), 'silu_boundary')
    M = J.module(reader, get('p228-projection-residual-comparison-v1/comparison.py'), 'silu_residual',
                 {'current': K, 'compare': C, 'boundary': B})
    S = J.module(reader, dependencies['p228-silu-materialization-diagnostic-v1/diagnostic.py'], 'silu_control')
    diagnostics = C.load_diagnostics(reader, get('p228-layer0-current-diagnostic-v1/diagnostics.py'))
    A = J.module(reader, sources['silu.py'], 'silu_candidate', {'comparison': M, 'diagnostic': S})
    return dict(J=J, reader=reader, prior_pin=prior_pin, prior=prior, manifest=manifest, sources=sources,
                dependencies=dependencies, source_map=source_map, additions=addition_pin,
                C=C, K=K, B=B, M=M, S=S, A=A, diagnostics=diagnostics)


def output(J, directory, name, value):
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(raw) <= 16 << 20, 'bounded diagnostic output')
    with (directory / name).open('xb') as stream:
        stream.write(raw)
    return J.actual(directory / name)[0]


def pure(c, out):
    J, reader, deps = c['J'], c['reader'], c['dependencies']
    modules = {'comparison': c['M'], 'current': c['K'], 'compare': c['C'],
               'boundary': c['B'], 'diagnostics': c['diagnostics'], 'silu': c['A']}
    T = J.module(reader, deps['p228-layer0-current-diagnostic-v1/test_current.py'], 'silu_fixture', modules)
    P = J.module(reader, deps['p228-projection-residual-comparison-v1/test_comparison.py'],
                 'silu_residual_fixture', dict(modules, test_current=T))
    test = J.module(reader, c['sources']['test_silu.py'], 'test_silu', dict(modules, test_comparison=P))
    suite = unittest.defaultTestLoader.loadTestsFromModule(test)
    names = sorted(t.id() for group in suite for t in group)
    require({name.rsplit('.', 1)[-1] for name in names} == TESTS and len(names) == 12, 'actual twelve-name inventory')
    before = output(J, out, 'sources-before.json', c['source_map'])
    transcript = io.StringIO()
    result = unittest.TextTestRunner(stream=transcript, verbosity=2).run(suite)
    raw = transcript.getvalue().encode(); require(len(raw) <= 1 << 20, 'bounded pure transcript')
    with (out / 'tests.log').open('xb') as stream:
        stream.write(raw)
    reader.recheck()
    after = output(J, out, 'sources-after.json', c['source_map'])
    passed = result.wasSuccessful() and result.testsRun == 12 and not result.skipped
    report = dict(schema=PURE_SCHEMA, passed=passed, tests=result.testsRun, errors=len(result.errors),
        failures=len(result.failures), skipped=len(result.skipped), names=names, manifest=c['manifest'],
        controller=c['sources']['run.py'], sources_before=before, sources_after=after,
        transcript=J.actual(out / 'tests.log')[0], source_postchecks_passed=True,
        synthetic_tests_only=True, gpu_execution=False, native_process_launched=False,
        numerical_acceptance=False, full_model_correctness=False, performance_claim=False, production_authority=False)
    complete = output(J, out, 'complete.json', report)
    print(json.dumps(dict(complete=complete, passed=passed, tests=result.testsRun)), flush=True)
    return 0 if passed else 1


class RecordedPins:
    """Only the frozen intake's file-reader API; no runtime or platform methods."""
    def __init__(self, J, reader):
        self.J, self.reader = J, reader

    def read(self, path, digest=None, retain=True, maximum=16 << 20):
        original = str(path)
        previous = self.reader.consumed.get(original)
        if previous is not None:
            pin = previous['original']
            require(digest is None or digest == pin['sha256'], 'same original reader identity')
        else:
            mapped = self.reader.mapping.get(original)
            if mapped is None:
                pin = self.J.actual(Path(path), digest)[0]
            else:
                pin = dict(mapped, path=original)
                require(digest is None or digest == pin['sha256'], 'exact transported original identity')
        raw = self.reader(pin); require(len(raw) <= maximum, 'bounded intake body')
        return pin, raw

    def pin(self, path, digest=None):
        return self.read(path, digest)[0]


def replay_candidate(c, R, candidate_pin, platform_pin):
    J, reader = c['J'], c['reader']
    manifest_pin = J.actual(GPU / 'manifest.json', GPU_SHA)[0]
    manifest = reader.doc(manifest_pin)
    require(manifest['schema'] == 'ferric-p228-silu-materialized-capture-gpu-package-v1'
            and len(manifest['files']) == 11 and manifest['pure_tests'] == 36, 'frozen actual SiLU GPU package')
    sources = {row['path']: dict(row, path=str(GPU / row['path'])) for row in manifest['files']}
    require(len(sources) == 11, 'unique GPU source roster')
    for pin in sources.values():
        reader(pin)
    V = J.module(reader, sources['layer_validation.py'], 'silu_layer_validation')
    I = J.module(reader, sources['intake.py'], 'silu_capture_intake', {'layer_validation': V})
    CV = J.module(reader, sources['capture_validation.py'], 'silu_capture_validation', {'layer_validation': V})
    runner = J.module(reader, sources['run.py'], 'silu_capture_runner', {'intake': I, 'capture_validation': CV})
    contracts = J.module(reader, sources['mlp_contracts.py'], 'silu_mlp_contracts')
    require(set(sources) == I.PACKAGE_FILES, 'frozen closed GPU roster')
    value = reader.doc(candidate_pin); directory = Path(candidate_pin['path']).parent
    require(directory.parent == E and Path(candidate_pin['path']).name == 'complete.json'
            and re.fullmatch(r'prefix-silu-materialized-capture-gpu-v228-v[1-9][0-9]{0,8}', directory.name)
            and value['schema'] == 'ferric-p228-silu-materialized-capture-gpu-v1'
            and value['passed'] is True and value['failures'] == [] and value['native_attempts'] == 1
            and value['retries'] == 0 and value['gpu_execution_requested'] is True
            and value['captured_arrays'] == 28 and value['pre_swiglu_arrays_equal'] is True
            and value['pre_swiglu_array_count'] == 22 and value['old_hidden_equality_required'] is False
            and value['current_tf4_hidden_equal'] is None and value['conditional_residual_checks_performed'] is False
            and all(value[k] is False for k in J.FALSE), 'actual successful diagnostic-only SiLU capture')
    require(value['controller'] == sources['run.py'] and value['supervisor_manifest'] == manifest_pin,
            'exact source-bound native controller')
    plan = reader.doc(value['plan']); I.input_shape(plan)
    old = reader.doc(c['prior']['native_outer']); old_plan = reader.doc(old['plan'])
    require(plan['output_label'] == directory.name and plan['baseline'] == value['baseline'] == J.BASELINE
            and plan['baseline_capture'] == value['baseline_capture'] == c['prior']['native_outer'],
            'genuine corrected baseline and TF4 lineage')
    for key in ('parent', 'worker', 'request', 'projection_image', 'lowering_complete', 'inspection_complete',
                'mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner',
                'parent_runtime_review', 'worker_runtime_review', 'capture_review'):
        require(value[key] == plan[key], 'actual plan binding: ' + key)
    for key in ('baseline', 'parent_cpu', 'worker_cpu', 'projection_image', 'lowering_complete', 'inspection_complete'):
        require(plan[key] == old_plan[key], 'unchanged prior qualification: ' + key)
    require(value['selected_runtime'] == dict(parent=value['parent'], worker=value['worker'],
                                             image=old['selected_runtime']['image'])
            and all(I.content(value[role]) == I.content(old[role]) for role in ('parent', 'worker'))
            and value['parent_cpu_complete'] == plan['parent_cpu']
            and value['worker_cpu_complete'] == plan['worker_cpu'], 'same actually qualified executables')
    pins = RecordedPins(J, reader)
    I.supervisor_tests(pins, plan, manifest, manifest_pin)
    with aliases({'mlp_contracts': contracts}):
        mlp = I.mlp_evidence(pins, plan, reader.doc(J.BASELINE))
    platform = reader.doc(platform_pin)
    ledger = dict(value['input_pins']); ledger.update(value['standalone_input_pins'])
    require(ledger.get(platform_pin['path']) == platform_pin, 'authentic historical platform input')
    for role in ('parent', 'worker'):
        review = reader.doc(value[role + '_runtime_review'])
        require(review['reviewed'] is True and review['authority'] == 'none' and review['binary'] == value[role]
                and review['host'] == platform['host'] and review['boot_id'] == platform['boot_id']
                and review['production_authority'] is False, 'actual selected runtime review')
    review = reader.doc(value['capture_review'])
    require(review['schema'] == I.REVIEW_SCHEMA and review['reviewed'] is True and review['authority'] == 'none'
            and review['output_label'] == directory.name and review['gpu_attempts'] == 1
            and all(review[k] == plan[k] for k in I.REVIEW_BINDINGS)
            and all(review[k] is False for k in I.REVIEW_FALSE) and review['mlp_provenance'] == mlp,
            'actual engineering review and exact new checked MLP lineage')
    request = reader.doc(value['request'])
    I.request_shape(request, reader.doc(old['request']), value['selected_runtime'], directory,
                    plan['mlp_image'], c['K'].wire_pin)
    require(set(value['leaves']) == set(J.LEAVES), 'seven recorded owned leaves')
    env = reader.doc(value['leaves']['parent']['retained_files']['command.json'])['env']
    require(type(env) is dict and all(env.get(k) == '' for k in
            ('CUDA_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES')), 'KFD-only parent environment')
    leaves = {name: (R.parent_leaf(J, reader, value, directory, env) if name == 'parent' else
                    J.owned_leaf(reader, value, directory, name, env)) for name in J.LEAVES}
    require(len({start['supervisor_pid'] for _, start in leaves.values()}) == 1
            and len({(s['parent']['pid'], s['parent']['starttime']) for _, s in leaves.values()}) == 7,
            'one supervisor and seven unique process incarnations')
    audit = J.module(reader, J.actual(E / 'p228-independent-gpu-observation-v1/validation.py',
        'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1')[0], 'silu_audit')
    topology = J.module(reader, J.actual(J.R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py',
        '6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a')[0], 'silu_topology')
    J.audit_samples(reader, value, directory, leaves, platform, audit, topology)
    records = value['retained_native']
    require(set(records) == CV.BODY | {'summary.json'}, 'exact twelve retained native bodies')
    for name, pin in records.items():
        require(pin['path'] == str(directory / 'native' / name), 'case-contained native body')
    files = {name: reader(records[name]) for name in CV.BODY}; summary = reader(records['summary.json'])
    parent, _ = leaves['parent']
    require(reader(parent['stdout']) == summary + b'\n' and reader.doc(records['request.json']) == request,
            'actual parent stdout and request')
    require(value['baseline_capture_payload'] == old['retained_native']['candidate-capture.bin'],
            'actual corrected baseline payload')
    old_bootstrap = reader.doc(old['retained_native']['candidate-bootstrap.json'])
    checked = CV.validate(summary, files, request, reader(value['baseline_capture_payload']),
                          old_bootstrap['layer']['input'])
    require(checked == value['checked'] == reader.doc(value['observation']), 'actual 28-array structural replay')
    CV.child_marker(reader(parent['stderr']), checked['closed_child_pids'][0])
    require(runner.lineage(pins, parent, checked) == value['owned_children'], 'actual child Close/lineage join')
    return value, records['summary.json'], manifest_pin


def compare(c, R, out, args):
    J, reader = c['J'], c['reader']
    require(len(args) == 4, 'ACTUAL_GPU_COMPLETE ACTUAL_GPU_SHA ACTUAL_PURE_COMPLETE ACTUAL_PURE_SHA')
    pure_pin = J.actual(Path(args[2]), args[3])[0]; tested = reader.doc(pure_pin)
    require(tested['schema'] == PURE_SCHEMA and tested['passed'] is True and tested['tests'] == 12
            and tested['errors'] == tested['failures'] == tested['skipped'] == 0
            and tested['manifest'] == c['manifest'] and tested['controller'] == c['sources']['run.py']
            and tested['source_postchecks_passed'] is True
            and {name.rsplit('.', 1)[-1] for name in tested['names']} == TESTS
            and reader.doc(tested['sources_before']) == reader.doc(tested['sources_after']) == c['source_map'],
            'actual exact twelve-test source qualification')
    reader(tested['transcript'])
    # Rehash the already replayed corrected-baseline evidence. Its historical
    # ownership checks are reused, not represented as another native execution.
    for row in c['prior']['consumed']:
        reader(row['original'])
    prior_plan = reader.doc(c['prior']['old_plan'])
    candidate_pin = J.actual(Path(args[0]), args[1])[0]
    native, summary, gpu_manifest = replay_candidate(c, R, candidate_pin, prior_plan['platform'])
    framework_outer, reference = J.framework(reader)
    require(framework_outer == c['prior']['framework_outer'], 'same genuine 33-stage framework capture')
    original = J.actual(E / 'framework-rearm-v224-v1/reference/reference.json',
        '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416')[0]
    baseline = reader.doc(c['prior']['native_outer'])
    result = c['A'].compare_retained(reference, original, J.BASELINE,
        baseline['retained_native']['summary.json'], summary, reader, c['diagnostics'],
        candidate_mlp_image_pin=native['mlp_image'], projection_image_pin=native['projection_image'],
        candidate_worker_pin=native['worker'])
    require(result['comparable_rows'] == len(result['baseline_comparisons']) == 24
            and result['unchanged_pre_swiglu_count'] == 22
            and len(result['conditional_residual_comparisons']) == 4 and result['conditional_residual_words'] == 16384
            and result['framework_product_control_exact'] is True
            and result['numerical_acceptance'] is False and result['acceptance_threshold'] is None,
            'complete diagnostics without fitted acceptance')
    reader.recheck()
    report = dict(schema=RESULT_SCHEMA, completed=True, controller=c['sources']['run.py'],
        manifest=c['manifest'], comparison_tests=pure_pin, comparison_sources=c['source_map'],
        native_outer=candidate_pin, baseline_comparison=c['prior_pin'], baseline_outer=c['prior']['native_outer'],
        framework_outer=framework_outer, native_supervisor_manifest=gpu_manifest,
        transport_additions=c['additions'], candidate_owned_leaves_replayed=7, candidate_audits_replayed=6,
        baseline_previously_replayed_evidence_rehashed=True, framework_owned_leaf_results_rechecked=21,
        native_structural_replayed=True, source_postchecks_passed=True, comparison=result,
        consumed=list(reader.consumed.values()), all_transitive_admission_inputs_replayed=False,
        current_gpu_audits_performed=False, gpu_execution=False, native_process_launched=False,
        numerical_acceptance=False, full_model_correctness=False, production_authority=False, performance_claim=False)
    pin = output(J, out, 'complete.json', report)
    print(json.dumps(dict(complete=pin, comparable_rows=24,
        conditional_residuals_exact=result['conditional_residuals_exact'],
        earliest_observable_divergence=result['earliest_observable_divergence'])), flush=True)
    return 0


def main(args):
    require(len(args) >= 3 and args[0] in ('pure', 'compare'),
            'pure PACKAGE_SHA FRESH_LABEL | compare PACKAGE_SHA FRESH_LABEL ADDITIONS ADDITIONS_SHA GPU SHA PURE SHA')
    mode, digest, label = args[:3]
    require(re.fullmatch('[0-9a-f]{64}', digest)
            and re.fullmatch('silu-materialized-comparison-' + ('pure-' if mode == 'pure' else '')
                             + r'v228-v[1-9][0-9]{0,8}', label), 'closed manifest and fresh label')
    require(len(args) == (3 if mode == 'pure' else 9), 'exact mode arguments')
    R, J = bootstrap(mode)
    out = E / label; require(not os.path.lexists(out), 'fresh CPU-only output')
    c = setup(J, digest, args[3:5] if mode == 'compare' else None)
    out.mkdir(mode=0o700)
    return pure(c, out) if mode == 'pure' else compare(c, R, out, args[5:])


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
