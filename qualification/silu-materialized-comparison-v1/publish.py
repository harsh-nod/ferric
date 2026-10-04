"""Publish authenticated retained SiLU diagnostics; never rerun numerical/GPU work."""
import argparse
import ast
import hashlib
import math
import os
from pathlib import Path
import re
import stat
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/silu-materialized-comparison-v1'
HELPER = F / 'qualification/layer0-native-capture-v1/tools/publish.py'
HELPER_SHA = '3181b2f41a51ee94c331b137f2807580c326f7f6dc53c429163d8465309e2cb8'
PACKAGE = 'p228-silu-materialized-comparison-v1'
PACKAGE_SHA = '2678c7e43318eb56388154bea3f5a4e5f6aaac3b307b7bd9865e5642a97adba2'
PURE_SHA = '278f480dbce3fa04383ecda1366f11fd7ba51cef42592d0a48cebce20ee1479a'
PRIOR_SHA = 'affe711d0dcac8e95609396c74e0f546ca09d9d6a4abf798eab5bfc72a7e7551'
GPU_SHA = '71cae69a25ac82c53bdf2976ab53b9d9af615c5af4b8337c2e386beb44a8a18b'
ORDER = ('norm', 'qkv', 'query', 'key-current', 'value-current', 'attention',
         'first-residual', 'mlp-norm', 'gate', 'up', 'activation-product', 'final-hidden')
COUNTS = (4096, 3072, 2048, 512, 512, 2048, 4096, 4096, 6144, 6144, 6144, 4096)
FALSE = ('gpu_execution', 'native_process_launched', 'numerical_acceptance',
         'full_model_correctness', 'performance_claim', 'production_authority')
COPIES = {}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def helper():
    require(HELPER.resolve(strict=True) == HELPER, 'canonical published reader')
    before = HELPER.lstat(); raw = HELPER.read_bytes()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 1 << 20
            and before == HELPER.lstat() and len(raw) == before.st_size
            and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact existing data reader')
    H = types.ModuleType('retained_silu_publication'); H.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), H.__dict__); H.body(HELPER)
    return H


def pin(H, path):
    raw = H.body(path)
    return dict(path=str(path), bytes=len(raw), sha256=H.digest(raw))


def original(H, path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals':
        relative = Path(*relative.parts[1:])
    return dict(pin(H, path), path=str(E / relative))


def read(H, record):
    record = H.normalize(record); relative = Path(record['path']).relative_to(E)
    path = L / ('proposals' if relative.parts[0].startswith(('p227-', 'p228-')) else '') / relative
    require(original(H, path) == record, 'actual original/retained body: ' + record['path'])
    return H.body(path)


def doc(H, record):
    return H.parse(read(H, record))


def retain(H, record, name):
    require(name not in COPIES and '..' not in Path(name).parts and not Path(name).is_absolute(), 'unique relative copy')
    COPIES[name] = (read(H, record), record)


def metric(value, nullable=False):
    require((nullable and value is None) or (type(value) in (int, float) and math.isfinite(value) and value >= 0),
            'finite nonnegative reported metric')


def metadata_pin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}
            and type(value['path']) is str and '\0' not in value['path']
            and Path(value['path']).is_absolute() and str(Path(value['path'])) == value['path']
            and '..' not in Path(value['path']).parts
            and type(value['bytes']) is int and 0 <= value['bytes'] <= 64 << 20
            and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']),
            'closed original or transported FilePin metadata')
    return value


def bf16_row(row, count):
    require(type(row['elements']) is int and row['elements'] == count
            and type(row['exact_words']) is int and 0 <= row['exact_words'] <= count
            and type(row['byte_equal']) is bool and row['byte_equal'] is (row['exact_words'] == count)
            and row['byte_equal'] is (row['reference_sha256'] == row['candidate_sha256']), 'BF16 row metadata')
    for key in ('reference_sha256', 'candidate_sha256'):
        require(re.fullmatch('[0-9a-f]{64}', row[key]), 'actual tensor digest')
    for key in ('max_abs_error', 'rmse', 'relative_l2'):
        metric(row[key], key == 'relative_l2')


def source_and_pure(H, value, prior):
    manifest_pin = original(H, L / 'proposals' / PACKAGE / 'manifest.json')
    require(manifest_pin['sha256'] == PACKAGE_SHA and value['manifest'] == manifest_pin, 'frozen tested comparison package')
    manifest = doc(H, manifest_pin)
    require(manifest['schema'] == 'ferric-p228-silu-materialized-comparison-package-v1'
            and manifest['pure_tests'] == 12 and len(manifest['files']) == 4
            and {r['path'] for r in manifest['files']} == {'silu.py', 'test_silu.py', 'run.py', 'README.md'}, 'four sources/twelve tests')
    sources = {r['path']: dict(r, path=str(E / PACKAGE / r['path'])) for r in manifest['files']}
    require(value['controller'] == sources['run.py'], 'executed exact comparison controller')
    expected = dict(prior['comparison_sources'])
    expected['p228-silu-materialization-diagnostic-v1/diagnostic.py'] = original(H,
        L / 'proposals/p228-silu-materialization-diagnostic-v1/diagnostic.py')
    require(expected['p228-silu-materialization-diagnostic-v1/diagnostic.py']['sha256'] ==
        '19ac2cf361d5653b4438b77d5a7f5501961ffe1dcabb2606fbf681b5175ab3f8', 'unchanged exact SiLU control')
    expected.update({PACKAGE + '/' + name: p for name, p in sources.items()})
    expected['previous-controller'] = prior['controller']
    expected['previous-reader-controller'] = original(H, L / 'proposals/p228-layer0-current-comparison-v1/run.py')
    require(expected['previous-reader-controller']['sha256'] ==
        '9c3db491d4f8c20890b045474bd1ab5bd8565d42916a8eb7772c0eb07c44717e'
        and len(expected) == 15 and value['comparison_sources'] == expected, 'exact imported/test/controller closure')
    retain(H, manifest_pin, 'source/manifest.json')
    for name, record in expected.items():
        retain(H, record, 'source/' + (name if '/' in name else name + '.py'))
    pure_pin = value['comparison_tests']; tested = doc(H, pure_pin)
    require(pure_pin['sha256'] == PURE_SHA and tested['schema'] == 'ferric-p228-silu-materialized-comparison-pure-v1'
            and tested['passed'] is True and type(tested['tests']) is int and tested['tests'] == 12
            and all(type(tested[k]) is int and tested[k] == 0 for k in ('errors', 'failures', 'skipped'))
            and tested['controller'] == value['controller'] and tested['manifest'] == manifest_pin
            and tested['source_postchecks_passed'] is True and tested['synthetic_tests_only'] is True
            and all(tested[k] is False for k in FALSE), 'actual bounded twelve-test receipt')
    directory = Path(pure_pin['path']).parent
    require(directory == E / 'silu-materialized-comparison-pure-v228-v1' and Path(pure_pin['path']).name == 'complete.json',
            'actual pure output namespace')
    require(doc(H, tested['sources_before']) == doc(H, tested['sources_after']) == expected, 'actual unchanged test source snapshots')
    names = sorted('test_silu.' + cls.name + '.' + fn.name
        for cls in ast.parse(read(H, sources['test_silu.py'])).body if isinstance(cls, ast.ClassDef)
        for fn in cls.body if isinstance(fn, ast.FunctionDef) and fn.name.startswith('test_'))
    require(len(names) == len(set(names)) == 12 and sorted(tested['names']) == names, 'actual named test inventory')
    log = read(H, tested['transcript']).decode(); passed = []
    for method, qualified in re.findall(r'^(test_[^\s]+) \(([^()\n]+)\) \.\.\. ok$', log, re.M):
        passed.append(qualified if qualified.endswith('.' + method) else qualified + '.' + method)
    require(sorted(passed) == names and re.search(r'\nRan 12 tests in [0-9.]+s\n\nOK\n?\Z', log), 'authentic unittest successes')
    retain(H, pure_pin, 'pure/complete.json')
    for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'), ('transcript', 'tests.log')):
        require(tested[key]['path'] == str(directory / name), 'actual pure raw path')
        retain(H, tested[key], 'pure/' + name)
    return expected


def controls(value, product_sha):
    require(set(value) == {'baseline', 'candidate'}, 'two genuine SiLU diagnostic controls')
    for label, control in value.items():
        require(control['schema'] == 'ferric-p228-silu-materialization-diagnostic-v1'
                and control['authority'] == 'none' and control['conditional_replay_performed'] is True
                and control['acceptance_threshold'] is None and len(control['ranks']) == 2
                and all(control[k] is False for k in ('exponential_evaluated', 'native_exp_error_measured',
                    'materialization_only_cause_proven', 'native_silu_intermediate_observed',
                    'different_gate_predictions_performed', 'numerical_acceptance', 'full_model_correctness',
                    'gpu_execution', 'performance_claim', 'production_authority')), 'conditional control scope')
        require(control['framework_product_control'] == dict(elements=12288, exact_words=12288, byte_equal=True,
                    predicted_sha256=product_sha, silu_input_equals_gate=True), 'genuine framework product control')
        for rank, row in enumerate(control['ranks']):
            groups = row['partitions']
            require(row['rank'] == rank and row['elements'] == 6144
                    and set(groups) == {'same_gate_same_up', 'same_gate_different_up',
                                        'different_gate_same_up', 'different_gate_different_up'}
                    and sorted(i for group in groups.values() for i in group) == list(range(6144))
                    and all(type(i) is int for group in groups.values() for i in group)
                    and row['partition_counts'] == {k: len(v) for k, v in groups.items()}, 'closed gate/up partitions')
            predicted = sorted(groups['same_gate_same_up'] + groups['same_gate_different_up'])
            require(row['prediction_indices'] == predicted and row['same_gate_elements'] == len(predicted)
                    and row['mismatch_indices'] == sorted(set(row['mismatch_indices']))
                    and set(row['mismatch_indices']) <= set(predicted)
                    and row['native_different_materialized'] == len(row['mismatch_indices'])
                    and row['native_equal_materialized'] == len(predicted) - len(row['mismatch_indices'])
                    and row['same_gate_same_up_native_different_framework_indices'] ==
                        sorted(set(row['mismatch_indices']) & set(groups['same_gate_same_up']))
                    and row['native_silu_intermediate_observed'] is False, 'conditional counts retain all mismatches')
    for left, right in zip(value['baseline']['ranks'], value['candidate']['ranks']):
        require(all(left[k] == right[k] for k in ('partitions', 'partition_counts', 'prediction_indices',
                    'same_gate_elements', 'predicted_bf16_sha256')), 'identical upstream gate/up conditioning')


def measured_rows(math_value, prior, native, baseline, framework):
    require(math_value['baseline_comparisons'] == prior['comparison']['comparisons'], 'baseline remains actual corrected capture')
    for name in ('comparisons', 'baseline_comparisons'):
        rows = math_value[name]
        require(len(rows) == 24 and {(r['stage'], r['rank']) for r in rows} ==
                {(stage, rank) for stage in ORDER for rank in (0, 1)}, 'complete two-rank/twelve-stage table')
        for row in rows:
            order = ORDER.index(row['stage']); bf16_row(row, COUNTS[order])
            require(row['observable_order'] == order, 'actual observation ordering')
    for new, old in zip(math_value['comparisons'], math_value['baseline_comparisons']):
        require((new['stage'], new['rank'], new['reference_sha256'], new['elements']) ==
                (old['stage'], old['rank'], old['reference_sha256'], old['elements']), 'same genuine framework reference')
    residuals = math_value['conditional_residual_comparisons']
    require(len(residuals) == 4 and {(r['stage'], r['rank']) for r in residuals} ==
            {('output', 0), ('output', 1), ('down', 0), ('down', 1)}
            and math_value['conditional_residual_words'] == 16384
            and math_value['conditional_residuals_exact'] is all(r['byte_equal'] for r in residuals), 'four conditional residual results')
    stages = {(r['stage'], r['rank']): r for r in native['stages']}
    old_stages = {(r['stage'], r['rank']): r for r in baseline['stages']}
    for row in residuals:
        stage = 'first-residual' if row['stage'] == 'output' else 'final-hidden'
        require(row['elements'] == 4096 and 0 <= row['exact_words'] <= 4096
                and row['differing_words'] == 4096 - row['exact_words']
                and row['byte_equal'] is (row['exact_words'] == 4096)
                and row['byte_equal'] is (row['actual_sha256'] == row['expected_sha256'])
                and row['actual_sha256'] == bytes(stages[stage, row['rank']]['sha256']).hex(), 'residual output hash/count')
    controls(math_value['silu_product_controls'], framework['passes'][0]['stages']['product']['pin']['sha256'])
    changes = math_value['changed_native_stages']
    require(len(changes) == 6 and {(r['stage'], r['rank']) for r in changes} ==
            {(stage, rank) for stage in ('activation', 'down-partial', 'final-hidden') for rank in (0, 1)},
            'six separately labeled native changes')
    for row in changes:
        key = row['stage'], row['rank']
        require(row['reference_sha256'] == bytes(old_stages[key]['sha256']).hex()
                and row['candidate_sha256'] == bytes(stages[key]['sha256']).hex(), 'actual old/new native stage hashes')
        if row['stage'] != 'down-partial':
            bf16_row(row, 6144 if row['stage'] == 'activation' else 4096)
        else:
            require(row['elements'] == 4096 and row['element_bytes'] == 4
                    and 0 <= row['exact_elements'] <= 4096 and row['independent_reference'] is False
                    and row['compared_to_full_framework_bf16'] is False
                    and row['byte_equal'] is (row['exact_elements'] == 4096)
                    and row['byte_equal'] is (row['reference_sha256'] == row['candidate_sha256']), 'native-only FP32 partial')
            metric(row['max_absolute_error']); metric(row['relative_l2_error'], True)


def markdown(value):
    fmt = lambda x: 'n/a' if x is None else format(x, '.8g')
    lines = ['# Retained SiLU Comparison', '', 'Diagnostic metrics only. No numerical acceptance or performance claim.', '',
        '| Stage | Rank | Baseline Exact | Candidate Exact | Words | Baseline Max Abs | Candidate Max Abs | Baseline Rel L2 | Candidate Rel L2 |',
        '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
    for old, new in zip(value['baseline_comparisons'], value['comparisons']):
        lines.append('| %s | %d | %d | %d | %d | %s | %s | %s | %s |' %
            (new['stage'], new['rank'], old['exact_words'], new['exact_words'], new['elements'],
             fmt(old['max_abs_error']), fmt(new['max_abs_error']), fmt(old['relative_l2']), fmt(new['relative_l2'])))
    lines.extend(['', '## Conditional Residuals', '', '| Stage | Rank | Exact Words | Words |', '| --- | ---: | ---: | ---: |'])
    for row in value['conditional_residual_comparisons']:
        lines.append('| %s | %d | %d | %d |' % (row['stage'], row['rank'], row['exact_words'], row['elements']))
    lines.extend(['', '## Same-Gate SiLU Controls', '',
        'Framework product control: 12,288 exact words. Predictions use genuine captured BF16 SiLU, not a reconstruction of native OCML exp.', '',
        '| Native Capture | Rank | Same-Gate Words | Equal Materialized | Different Materialized |', '| --- | ---: | ---: | ---: | ---: |'])
    for label, control in value['silu_product_controls'].items():
        for row in control['ranks']:
            lines.append('| %s | %d | %d | %d | %d |' % (label, row['rank'], row['same_gate_elements'],
                row['native_equal_materialized'], row['native_different_materialized']))
    lines.extend(['', '## Native Down Partials', '',
        'FP32 partial-to-partial changes only; neither side is a full framework projection oracle.', '',
        '| Rank | Exact FP32 Words | Words | Max Abs Change | Relative L2 Change |', '| ---: | ---: | ---: | ---: | ---: |'])
    for row in value['changed_native_stages']:
        if row['stage'] == 'down-partial':
            lines.append('| %d | %d | %d | %s | %s |' % (row['rank'], row['exact_elements'], row['elements'],
                fmt(row['max_absolute_error']), fmt(row['relative_l2_error'])))
    return ('\n'.join(lines) + '\n').encode()


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary publication Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('complete_path'); parser.add_argument('complete_sha256'); args = parser.parse_args()
    path = Path(args.complete_path)
    require(path.parent.parent == L and path.name == 'complete.json'
            and re.fullmatch(r'silu-materialized-comparison-v228-v[1-9][0-9]{0,8}', path.parent.name)
            and re.fullmatch('[0-9a-f]{64}', args.complete_sha256), 'actual local comparison path/SHA')
    H = helper(); value = H.document(path, args.complete_sha256); receipt = original(H, path)
    prior = doc(H, value['baseline_comparison'])
    require(value['baseline_comparison']['sha256'] == PRIOR_SHA
            and value['schema'] == 'ferric-p228-silu-materialized-comparison-observation-v1'
            and value['completed'] is True and value['source_postchecks_passed'] is True
            and value['candidate_owned_leaves_replayed'] == 7 and value['candidate_audits_replayed'] == 6
            and value['framework_owned_leaf_results_rechecked'] == 21 and value['native_structural_replayed'] is True
            and value['baseline_previously_replayed_evidence_rehashed'] is True
            and value['baseline_outer'] == prior['native_outer'] and value['framework_outer'] == prior['framework_outer']
            and all(value[k] is False for k in (*FALSE, 'all_transitive_admission_inputs_replayed', 'current_gpu_audits_performed')),
            'actual bounded diagnostic result and honest replay scope')
    sources = source_and_pure(H, value, prior)
    gpu, old = doc(H, value['native_outer']), doc(H, value['baseline_outer'])
    require(gpu['schema'] == 'ferric-p228-silu-materialized-capture-gpu-v1' and gpu['passed'] is True
            and gpu['failures'] == [] and gpu['native_attempts'] == 1 and gpu['retries'] == 0
            and gpu['supervisor_manifest'] == value['native_supervisor_manifest']
            and gpu['supervisor_manifest']['sha256'] == GPU_SHA and gpu['baseline_capture'] == value['baseline_outer']
            and gpu['captured_arrays'] == 28 and gpu['pre_swiglu_arrays_equal'] is True
            and gpu['pre_swiglu_array_count'] == 22 and gpu['old_hidden_equality_required'] is False
            and gpu['conditional_residual_checks_performed'] is False, 'actual new capture selection')
    m = value['comparison']; inputs = m['inputs']
    require(m['schema'] == 'ferric-p228-silu-materialized-comparison-v1' and m['authority'] == 'none'
            and m['input_token'] == 9112 and m['position'] == 0 and m['comparable_rows'] == 24
            and m['unchanged_pre_swiglu_count'] == len(m['unchanged_pre_swiglu_arrays']) == 22
            and all(r['byte_equal'] is True for r in m['unchanged_pre_swiglu_arrays'])
            and m['conditional_replay_performed'] is True and m['genuine_independent_framework_outputs'] is True
            and m['framework_product_control_exact'] is True and m['acceptance_threshold'] is None
            and all(m[k] is False for k in ('ordering_is_a_causal_proof', 'native_silu_intermediate_observed',
                'exponential_evaluated', 'native_exp_error_measured', 'materialization_only_cause_proven',
                'different_gate_predictions_performed', 'upstream_partial_numerics_checked', 'old_hidden_equality_required',
                'receipt_authentication', 'arithmetic_prerequisites_verified', 'runtime_premises_discharged',
                'numerical_acceptance', 'full_layer_numerics_accepted', 'full_model_correctness', 'production_authority',
                'performance_measured', 'gpu_execution', 'fp32_partials_compared_to_full_bf16')), 'diagnostic arithmetic scope')
    require(inputs['candidate_capture'] == gpu['retained_native']['summary.json']
            and inputs['corrected_baseline'] == old['retained_native']['summary.json']
            and inputs['selected_mlp_image'] == gpu['mlp_image'] and inputs['projection_image'] == gpu['projection_image']
            and inputs['worker'] == gpu['worker'] and inputs['current_tf4'] == gpu['baseline']
            and all(inputs[k] == prior['comparison']['inputs'][k] for k in ('framework_capture', 'original_framework', 'current_tf4'))
            and m['candidate_files'] == {k: p for k, p in gpu['retained_native'].items() if k != 'summary.json'}
            and m['baseline_files'] == {k: p for k, p in old['retained_native'].items() if k != 'summary.json'}, 'same selected actual inputs')
    consumed = {}
    for pair in value['consumed']:
        require(set(pair) == {'original', 'retained'}, 'explicit original/transport pair')
        # Historical prompt/topology identities also live outside E. They remain
        # metadata here; read/copy access above stays confined to retained E files.
        a, b = metadata_pin(pair['original']), metadata_pin(pair['retained'])
        require(a['bytes'] == b['bytes'] and a['sha256'] == b['sha256'] and a['path'] not in consumed,
                'unique content-preserving transport')
        consumed[a['path']] = a
    for record in [value['native_outer'], value['baseline_comparison'], value['baseline_outer'], value['manifest'],
                   value['comparison_tests'], *sources.values(), *m['candidate_files'].values(), *m['baseline_files'].values(),
                   *(p for k, p in inputs.items() if k != 'worker')]:
        require(consumed.get(record['path']) == record, 'selected actual consumed identity')
    framework = doc(H, inputs['framework_capture'])
    require(framework['repeat_passes_byte_equal'] is True and len(framework['passes']) == 2
            and framework['input_token'] == 9112 and framework['position'] == 0, 'genuine repeated framework metadata')
    for repeat in framework['passes']:
        require(len(repeat['stages']) == 33, 'full genuine stage census')
        for stage in repeat['stages'].values():
            require(consumed.get(stage['pin']['path']) == stage['pin'], 'all genuine stage payloads actually consumed')
    measured_rows(m, prior, doc(H, inputs['candidate_capture']), doc(H, inputs['corrected_baseline']), framework)
    retain(H, receipt, 'complete.json')
    publisher = pin(H, Path(__file__).resolve())
    COPIES['publish.py'] = (H.body(Path(__file__).resolve()), publisher)
    table = dict(schema='ferric-p228-silu-materialized-comparison-table-v1', receipt=receipt,
        **{k: m[k] for k in ('comparisons', 'baseline_comparisons', 'conditional_residual_comparisons',
            'conditional_residual_conditioning', 'conditional_residuals_exact', 'silu_product_controls',
            'changed_native_stages', 'earliest_observable_divergence', 'baseline_earliest_observable_divergence')},
        numerical_acceptance=False, acceptance_threshold=None, performance_claim=False, causal_attribution_proven=False)
    outputs = {name: raw for name, (raw, _) in COPIES.items()}
    outputs.update({'table.json': H.json_bytes(table), 'table.md': markdown(m)})
    result = dict(schema='ferric-p228-silu-materialized-comparison-publication-v1', authority='none',
        comparison_receipt=receipt, baseline_comparison=value['baseline_comparison'], pure_receipt=value['comparison_tests'],
        pure_tests=12, native_outer=value['native_outer'], baseline_outer=value['baseline_outer'], framework_outer=value['framework_outer'],
        native_supervisor_manifest=value['native_supervisor_manifest'], selected_mlp_image=inputs['selected_mlp_image'],
        projection_image=inputs['projection_image'], controller=value['controller'], source_closure=sources,
        publisher=publisher, data_reader=pin(H, HELPER), consumed=value['consumed'],
        files={name: dict(original=record, bytes=len(raw), sha256=H.digest(raw)) for name, (raw, record) in sorted(COPIES.items())},
        tables={name: dict(bytes=len(outputs[name]), sha256=H.digest(outputs[name])) for name in ('table.json', 'table.md')},
        locally_rehashed=[dict(path=str(p), **row) for p, row in sorted(H.CHECKED.items())],
        metadata_validated=True, numerical_comparison_reexecuted=False, tensor_buffers_locally_replayed=False,
        all_consumed_bodies_locally_rehashed=False, tested_modules_imported=False, tests_rerun=False,
        gpu_execution=False, numerical_acceptance=False, full_layer_numerics_accepted=False, full_model_correctness=False,
        production_authority=False, performance_claim=False, acceptance_threshold=None,
        limitations=['Measured metrics are copied from the exact bounded comparison, not recomputed by this publisher.',
            'Conditional residual and same-gate SiLU controls do not prove full GEMM, OCML, layer or model accuracy.',
            'Consumed tensor pins are retained identity metadata; this publisher does not reread every tensor buffer.',
            'The copied source README records author-time unexecuted status; actual pure and comparison receipts govern execution status.'])
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical qualification parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'}, 'fresh output/root README only')
        if os.path.lexists(Q / 'README.md'):
            H.body(Q / 'README.md')
    for path in list(H.CHECKED):
        H.body(path)
    outputs['result.json'] = H.json_bytes(result)
    Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in outputs.items():
        destination = Q / name; destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as stream:
            stream.write(raw)
        require(H.body(destination) == raw, 'byte-exact publication')
    for path in list(H.CHECKED):
        H.body(path)
    print(H.json_bytes(dict(result=pin(H, Q / 'result.json'), files=len(outputs))).decode(), end='')


if __name__ == '__main__':
    main()
