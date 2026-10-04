"""Publish measured TF4 diagnostics by metadata replay, never tensor recomputation."""
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
Q = F / 'qualification/silu-materialized-decode-comparison-v1'
GPU_PUBLIC = F / 'qualification/silu-materialized-decode-native-v1/result.json'
HELPER = F / 'qualification/layer0-native-capture-v1/tools/publish.py'
HELPER_SHA = '3181b2f41a51ee94c331b137f2807580c326f7f6dc53c429163d8465309e2cb8'
ADAPTER = 'p228-silu-materialized-decode-comparison-v1'
HARNESS = ADAPTER
PURE = 'silu-materialized-decode-comparison-pure-v228-v1'
FILES = {
    (ADAPTER, 'comparison.py'): ('source/comparison.py', 'a48d1a455ec0b3f3cc8903977cc8ff6ddec33cecdf28effeab0442f5ce73266b'),
    (ADAPTER, 'test_comparison.py'): ('source/test_comparison.py', '113576ce1de037d5e82b79a7565c717b3ca12fcd29eaedfe2c1ce11cc7adc8ed'),
    (ADAPTER, 'README.md'): ('source/README.md', '5c778dca94b814edc7950a3acd3292d3fa115265adb2af86f7505e91a95452ed'),
    (HARNESS, 'pure.py'): ('harness/pure.py', '4b2b5d01f03dfdb05caea82e58dc126b208e46b922836fb5e25af8b67438f624'),
    (HARNESS, 'run.py'): ('harness/run.py', '98befeb397359bc6141155a927da6dc7078df533d9fd0fa25c0b03f8df3f61ef'),
}
FRAMEWORK = dict(path=str(E / 'framework-rearm-v224-v1/complete.json'), bytes=8288,
    sha256='cac5d79969c2e17a19630a581b5e21c594ea65b452806630ee87bf7855396036')
BASELINE = dict(path=str(E / 'projection-residual-decode-framework-comparison-v228-v1/complete.json'),
    bytes=206798, sha256='2a5b98ae6a3a2ba4a034dbea0662504988d388b12a5447eaa395f82e3a41dd0c')
LAYER = (1024009, '67a235280b48cb5f6a2c51bcca534c182c843857c4c4a98e86718ed368ef12e4')
LAYER_MATH = (1508927, '828f8fdb4fc4194b5d7d5c65135222b472d2ec76080667fae8f0b8febba169aa')
MLP = (33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589')
FALSE = ('numerical_acceptance', 'full_model_correctness', 'full_model_acceptance', 'performance_claim', 'production_authority')
COPIES = {}


def require(ok, message):
    if not ok: raise RuntimeError(message)


def helper():
    require(HELPER.resolve(strict=True) == HELPER, 'canonical published data helper')
    before = HELPER.lstat(); raw = HELPER.read_bytes()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 1 << 20
        and before == HELPER.lstat() and len(raw) == before.st_size
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'authenticated data reader')
    value = types.ModuleType('retained_publication_data'); value.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), value.__dict__); value.body(HELPER)
    return value


def pin(H, path):
    raw = H.body(path)
    return dict(path=str(path), bytes=len(raw), sha256=H.digest(raw))


def original(H, path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals': relative = Path(*relative.parts[1:])
    return dict(pin(H, path), path=str(E / relative))


def read(H, record):
    record = H.normalize(record); relative = Path(record['path']).relative_to(E)
    path = L / ('proposals' if relative.parts[0].startswith(('p227-', 'p228-')) else '') / relative
    require(original(H, path) == record, 'original/retained metadata identity')
    return H.body(path)


def doc(H, record):
    return H.parse(read(H, record))


def copy(H, path, destination, expected=None):
    record = original(H, path); raw = H.body(path)
    require(expected is None or record['sha256'] == expected, 'selected published source hash')
    require(destination not in COPIES, 'unique publication destination')
    COPIES[destination] = (raw, record)
    return record


def literal(raw, name):
    values = [n.value for n in ast.parse(raw).body if isinstance(n, ast.Assign)
        and len(n.targets) == 1 and isinstance(n.targets[0], ast.Name) and n.targets[0].id == name]
    require(len(values) == 1, 'one authenticated literal: ' + name)
    value = values[0]
    if name == 'SOURCES':
        require(isinstance(value, ast.Dict), 'source roster dictionary')
        result = {}
        for key, item in zip(value.keys, value.values):
            if isinstance(item, ast.Name):
                require(item.id == 'J_SHA', 'only declared Reader hash alias')
                digest = literal(raw, 'J_SHA')
            else: digest = ast.literal_eval(item)
            key = ast.literal_eval(key); require(key not in result, 'unique source literal')
            result[key] = digest
        return result
    return ast.literal_eval(value)


def pure(H, sources, pure_sha):
    directory = L / PURE
    require({p.name for p in directory.iterdir()} == {'complete.json', 'sources-before.json', 'sources-after.json', 'tests.log'},
        'four actual pure records')
    pins = {name: copy(H, directory / name, 'tests/' + name, pure_sha if name == 'complete.json' else None)
        for name in ('complete.json', 'sources-before.json', 'sources-after.json', 'tests.log')}
    value = doc(H, pins['complete.json']); controller = sources[HARNESS + '/pure.py']
    require(value['schema'] == 'ferric-p228-silu-materialized-decode-comparison-pure-v1'
        and value['passed'] is True and type(value['tests']) is int and value['tests'] == 17
        and all(type(value[k]) is int and value[k] == 0 for k in ('errors', 'failures', 'skipped'))
        and value['controller'] == controller and value['source_postchecks_passed'] is True
        and value['synthetic_only'] is True and all(value[k] is False for k in
            ('gpu_execution', 'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')),
        'actual passing pure17, not numerical qualification')
    for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'), ('transcript', 'tests.log')):
        require(value[key] == pins[name], 'pure receipt/raw joins')
    expected = literal(read(H, controller), 'SOURCES'); require(len(expected) == 10, 'ten-source pinned closure')
    closure = {}
    for relative, sha in expected.items():
        path = L / 'proposals' / relative; record = original(H, path)
        require(record['sha256'] == sha, 'actual source closure body'); closure[relative] = record
    require(all(closure[ADAPTER + '/' + name] == sources[ADAPTER + '/' + name]
        for name in ('comparison.py', 'test_comparison.py', 'README.md')), 'adapter belongs to tested closure')
    tested = dict(closure, controller=controller)
    require(doc(H, pins['sources-before.json']) == doc(H, pins['sources-after.json']) == tested, 'tested complete source snapshots')
    raw = read(H, sources[ADAPTER + '/test_comparison.py'])
    names = sorted('projection_decode_comparison_tests.' + cls.name + '.' + fn.name
        for cls in ast.parse(raw).body if isinstance(cls, ast.ClassDef)
        for fn in cls.body if isinstance(fn, ast.FunctionDef) and fn.name.startswith('test_'))
    require(len(names) == len(set(names)) == 17 and sorted(value['names']) == names
        and {n.rsplit('.', 1)[1] for n in names} == literal(read(H, controller), 'TESTS'), 'actual full named inventory')
    log = read(H, pins['tests.log']).decode(); passed = []
    for method, qualified in re.findall(r'^(test_[^\s]+) \(([^()\n]+)\) \.\.\. ok$', log, re.M):
        passed.append(qualified if qualified.endswith('.' + method) else qualified + '.' + method)
    require(sorted(passed) == names and len(passed) == 17
        and re.search(r'\nRan 17 tests in [0-9.]+s\n\nOK\n?\Z', log), 'Python3.10/3.12 exact unittest log names')
    return pins['complete.json'], closure


def rows(H, value, gpu, reference, native):
    comparisons = value['comparisons']; growth = value['layer_error_trajectories']
    require(len(comparisons) == len(growth) == 4 and value['tensor_rows'] == 152, 'four38-tensor comparisons')
    names = ['layer%d-hidden' % n for n in range(36)] + ['final-norm', 'logits']
    equal = 0
    for position, row in enumerate(comparisons):
        case = reference['passes'][0]['cases'][position]; event = native['files']['frames'][position]['response']['event']
        require(row['position'] == position and row['same_input_history'] is True
            and row['reference_input'] == row['candidate_input'] == [9112, 2190, 3772, 220][position]
            and row['reference_output'] == case['record']['output_token']
            and row['candidate_output'] == event['output_token'] == gpu['structural']['output_tokens'][position]
            and row['output_equal'] is (row['reference_output'] == row['candidate_output'])
            and [t['name'] for t in row['tensors']] == names, 'actual TF4 token and tensor metadata')
        parts = event['capture']['layer_hidden'] + [event['capture']['final_normalized'], event['capture']['logits']]
        for index, tensor in enumerate(row['tensors']):
            count = 151936 if index == 37 else 4096; ref = case['tensors'][names[index]]; part = parts[index]
            require(type(tensor['elements']) is int and tensor['elements'] == count and ref['bytes'] == part['bytes'] == 2 * count
                and tensor['reference_sha256'] == ref['sha256']
                and tensor['candidate_sha256'] == bytes(part['sha256']).hex()
                and type(tensor['exact_words']) is int and 0 <= tensor['exact_words'] <= count
                and type(tensor['byte_equal']) is bool and tensor['byte_equal'] is (tensor['exact_words'] == count)
                and tensor['byte_equal'] is (tensor['reference_sha256'] == tensor['candidate_sha256']), 'recorded tensor identity/geometry')
            first = tensor['first_mismatching_element']
            require(first is None if tensor['byte_equal'] else type(first) is int and 0 <= first < count, 'first difference metadata')
            for name in ('max_abs_error', 'rmse', 'relative_l2'):
                metric = tensor[name]
                require((name == 'relative_l2' and metric is None) or
                    (type(metric) in (int, float) and math.isfinite(metric) and metric >= 0), 'finite reported metric')
            equal += tensor['byte_equal']
        trajectory = growth[position]
        require(trajectory['position'] == position and len(trajectory['layers']) == 36
            and trajectory['causal_attribution_proven'] is False and trajectory['monotonic_growth_required'] is False
            and trajectory['first_nonexact_hidden_layer'] == next((i for i, t in enumerate(row['tensors'][:36]) if not t['byte_equal']), None),
            'descriptive layer trajectory only')
        for layer, item in enumerate(trajectory['layers']):
            require(item['layer'] == layer and all(item[k] == row['tensors'][layer][k]
                for k in ('exact_words', 'elements', 'max_abs_error', 'relative_l2')), 'trajectory joins measured row')
    require(value['exact_tensor_rows'] == equal
        and value['reference_output_tokens'] == [r['reference_output'] for r in comparisons]
        and value['candidate_output_tokens'] == [r['candidate_output'] for r in comparisons]
        and value['output_tokens_equal'] is all(r['output_equal'] for r in comparisons), 'recorded summary counts, never fitted acceptance')


def baseline(H, value, comparison):
    require(value['baseline_comparison'] == BASELINE, 'actual previously measured TF4 baseline')
    old = doc(H, BASELINE)
    require(old['schema'] == 'ferric-p228-projection-residual-decode-framework-comparison-observation-v1'
        and old['completed'] is True and old['errors'] == [] and old['source_postchecks_passed'] is True
        and old['framework_outer'] == FRAMEWORK and all(old[key] is False for key in (*FALSE, 'gpu_execution')),
        'authentic baseline diagnostic only')
    recorded = comparison['recorded_baseline_metrics']
    require(recorded['receipt'] == BASELINE and recorded['comparisons'] == old['comparison']['comparisons']
        and recorded['reference_tensor_identities_equal'] is True
        and all(recorded[key] is False for key in ('metrics_recomputed', 'baseline_payloads_rehashed',
            'causal_attribution_proven', 'numerical_acceptance', 'performance_claim')),
        'baseline metrics copied exactly without new native or tensor replay')
    require(len(recorded['comparisons']) == len(comparison['comparisons']) == 4, 'four baseline/candidate positions')
    for left, right in zip(recorded['comparisons'], comparison['comparisons']):
        require(all(left[key] == right[key] for key in ('position', 'reference_input', 'candidate_input',
            'reference_output', 'same_input_history')) and left['same_input_history'] is True
            and len(left['tensors']) == len(right['tensors']) == 38, 'same genuine teacher-forced histories')
        for a, b in zip(left['tensors'], right['tensors']):
            require(all(a[key] == b[key] for key in ('name', 'elements', 'reference_sha256')),
                'all152 same reference tensor identities')
    return old['comparison']


def markdown(table):
    old, new = table['baseline'], table['candidate']
    lines = ['# Recorded Four-Step Framework Diagnostics', '',
        'Baseline: projection-residual correction. Candidate: the same correction plus BF16 SiLU materialization.',
        'These are retained diagnostic metrics, not a numerical-acceptance or performance result.', '',
        '| Position | Reference token | Baseline token | Candidate token |',
        '| --- | ---: | ---: | ---: |']
    for a, b in zip(old['comparisons'], new['comparisons']):
        lines.append('| %d | %d | %d | %d |' %
            (b['position'], b['reference_output'], a['candidate_output'], b['candidate_output']))
    lines.extend(['', '| Position | Tensor | Baseline exact | Candidate exact | Elements | Baseline max abs | Candidate max abs | Baseline relative L2 | Candidate relative L2 |',
        '| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |'])
    def number(value):
        return 'undefined' if value is None else format(value, '.17g')
    for a, b in zip(old['comparisons'], new['comparisons']):
        for x, y in zip(a['tensors'], b['tensors']):
            lines.append('| %d | %s | %d | %d | %d | %s | %s | %s | %s |' %
                (b['position'], y['name'], x['exact_words'], y['exact_words'], y['elements'],
                 number(x['max_abs_error']), number(y['max_abs_error']),
                 number(x['relative_l2']), number(y['relative_l2'])))
    lines.extend(['', 'Full-precision stored values, per-layer trajectories, and original receipt pins are in `table.json`.',
        'Baseline metrics are copied from their authenticated earlier comparison; its payloads are not replayed here.', ''])
    return '\n'.join(lines).encode('ascii')


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('complete_path'); parser.add_argument('complete_sha256')
    parser.add_argument('pure_sha256'); parser.add_argument('gpu_sha256'); parser.add_argument('gpu_public_sha256')
    args = parser.parse_args()
    path = Path(args.complete_path)
    require(path.name == 'complete.json' and path.parent.parent == L
        and re.fullmatch(r'silu-materialized-decode-framework-comparison-v228-v[1-9][0-9]{0,8}', path.parent.name)
        and all(re.fullmatch('[0-9a-f]{64}', sha) for sha in (args.complete_sha256, args.pure_sha256,
            args.gpu_sha256, args.gpu_public_sha256)), 'actual completion, pure and GPU publication digests')
    H = helper(); value = H.document(path, args.complete_sha256); receipt = original(H, path)
    sources = {package + '/' + name: copy(H, L / 'proposals' / package / name, target, sha)
        for (package, name), (target, sha) in FILES.items()}
    pure_pin, closure = pure(H, sources, args.pure_sha256)
    require(value['schema'] == 'ferric-p228-silu-materialized-decode-framework-comparison-observation-v1'
        and value['completed'] is True and value['errors'] == [] and value['controller'] == sources[HARNESS + '/run.py']
        and value['pure_complete'] == pure_pin and value['source_postchecks_passed'] is True
        and value['framework_outer'] == FRAMEWORK and value['framework_payloads_rehashed'] == 8
        and value['reference_arithmetic_unchanged'] is True and value['acceptance_threshold'] is None
        and value['baseline_metrics_recomputed'] is False and value['baseline_payloads_rehashed'] is False
        and all(value[k] is False for k in (*FALSE, 'gpu_execution', 'source_image_admission_replayed', 'current_platform_audit_performed')),
        'actual completed diagnostic, no numerical acceptance')
    expected = dict(closure, controller=sources[HARNESS + '/run.py'], pure_controller=sources[HARNESS + '/pure.py'])
    require(doc(H, value['sources_before']) == doc(H, value['sources_after']) == expected, 'actual evaluation source closure unchanged')
    require(value['native_outer']['sha256'] == args.gpu_sha256, 'explicit actual GPU completion digest')
    gpu = H.document(GPU_PUBLIC, args.gpu_public_sha256); outer = doc(H, value['native_outer'])
    require(gpu['schema'] == 'ferric-p228-silu-materialized-decode-native-publication-v1'
        and gpu['gpu_observation'] == value['native_outer'] and gpu['recorded_gpu_execution'] is True
        and gpu['structural_validator_replayed'] is True and gpu['native_attempts'] == 1 and gpu['retries'] == 0
        and gpu['captured_tensor_rows'] == 152 and gpu['captured_payloads'] == 4
        and all(gpu[k] is False for k in FALSE) and outer['passed'] is True
        and outer['checked'] == gpu['structural'] and outer['retained_native'] == gpu['retained_native'], 'already published actual GPU capture')
    comparison = value['comparison']
    require(comparison['schema'] == 'ferric-p228-silu-materialized-decode-framework-diagnostic-v1'
        and comparison['mode'] == 'teacher_forced' and comparison['candidate_complete'] == value['native_outer']
        and comparison['framework_complete'] == FRAMEWORK and comparison['projection_image'] == gpu['projection_image']
        and comparison['selected_mlp_image'] == outer['mlp_image'] == gpu['mlp_image']
        and (comparison['selected_mlp_image']['bytes'], comparison['selected_mlp_image']['sha256']) == MLP
        and comparison['layer_capture'] == outer['baseline']
        and (comparison['layer_capture']['bytes'], comparison['layer_capture']['sha256']) == LAYER
        and comparison['layer_comparison'] == outer['layer_comparison']
        and (comparison['layer_comparison']['bytes'], comparison['layer_comparison']['sha256']) == LAYER_MATH
        and comparison['layer_predecessor_receipts_rehashed'] is True and comparison['layer_tensor_bodies_replayed'] is False
        and comparison['structural'] == gpu['structural'] and comparison['acceptance_threshold'] is None
        and comparison['recorded_close_and_owner_reap_checked'] is True and comparison['recorded_six_audit_leaves_rehashed'] is True
        and all(comparison[k] is False for k in (*FALSE, 'gpu_launched', 'current_platform_idle_audits_verified',
            'source_binary_image_admission_replayed', 'independent_tensor_acceptance', 'conditional_residual_checks_performed',
            'candidate_intermediate_inputs', 'sustained_2048_256', 'causal_attribution_proven')), 'measured diagnostic scope')
    consumed = {}
    for pair in value['consumed']:
        require(set(pair) == {'original', 'retained'} and H.normalize(pair['original']) == H.normalize(pair['retained']),
            'actual runner used original paths without transport rewriting')
        record = pair['original']; require(record['path'] not in consumed, 'unique consumed metadata'); consumed[record['path']] = record
    required = [value['native_outer'], FRAMEWORK, value['controller'], pure_pin, BASELINE,
        comparison['layer_capture'], comparison['layer_comparison'], *expected.values(), *gpu['retained_native'].values()]
    for record in required: require(consumed.get(record['path']) == record, 'selected source/native consumed identity')
    reference_owner = doc(H, FRAMEWORK); reference = doc(H, reference_owner['reference'])
    require(reference_owner['passed'] is True and reference['repeat_passes_byte_equal'] is True
        and len(reference['passes']) == 2, 'actual repeated framework reference metadata')
    for repeat in reference['passes']:
        require(len(repeat['cases']) == 4, 'four framework positions')
        for case in repeat['cases']:
            require(consumed.get(case['payload']['path']) == case['payload'] and case['payload']['bytes'] == 606976,
                'all eight actual reference payloads recorded as consumed')
    native = doc(H, gpu['retained_native']['complete.json'])
    rows(H, comparison, gpu, reference, native)
    old = baseline(H, value, comparison)
    copy(H, path, 'complete.json', args.complete_sha256)
    for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json')):
        require(value[key]['path'] == str(E / path.parent.name / name), 'actual comparison source snapshot namespace')
        require(copy(H, path.parent / name, name) == value[key], 'copied actual source snapshot')
    publisher = pin(H, Path(__file__).resolve())
    COPIES['publish.py'] = (H.body(Path(__file__).resolve()), publisher)
    metrics = ('comparisons', 'layer_error_trajectories', 'tensor_rows', 'exact_tensor_rows',
        'output_tokens_equal', 'reference_output_tokens', 'candidate_output_tokens')
    table = dict(schema='ferric-p228-silu-materialized-decode-comparison-table-v1', receipt=receipt,
        native_outer=value['native_outer'], framework_outer=FRAMEWORK, baseline_receipt=BASELINE,
        candidate={k: comparison[k] for k in metrics}, baseline={k: old[k] for k in metrics},
        baseline_metrics_recomputed=False, baseline_payloads_rehashed=False,
        acceptance_threshold=None, numerical_acceptance=False, causal_attribution_proven=False)
    table_raw = H.json_bytes(table)
    table_md = markdown(table)
    result = dict(schema='ferric-p228-silu-materialized-decode-comparison-publication-v1', authority='none',
        comparison_receipt=receipt, pure_receipt=pure_pin, pure_tests=17, controller=value['controller'],
        gpu_publication=pin(H, GPU_PUBLIC), native_outer=value['native_outer'], framework_outer=FRAMEWORK,
        baseline_comparison=BASELINE, selected_mlp_image=comparison['selected_mlp_image'],
        layer_capture=comparison['layer_capture'], layer_comparison=comparison['layer_comparison'],
        sources_before=value['sources_before'], sources_after=value['sources_after'], source_closure=expected,
        consumed=list(value['consumed']), publisher=publisher, data_reader=pin(H, HELPER),
        files={name: dict(original=record, bytes=len(raw), sha256=H.digest(raw)) for name, (raw, record) in sorted(COPIES.items())},
        table=dict(path='table.json', bytes=len(table_raw), sha256=H.digest(table_raw)),
        table_markdown=dict(path='table.md', bytes=len(table_md), sha256=H.digest(table_md)),
        locally_rehashed=[dict(path=str(p), **record) for p, record in sorted(H.CHECKED.items())],
        metadata_validated=True, tensor_bodies_locally_replayed=False, all_consumed_bodies_locally_rehashed=False,
        numerical_comparison_reexecuted=False, tested_modules_imported=False, tests_rerun=False,
        baseline_metrics_recomputed=False, baseline_payloads_rehashed=False,
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False, full_model_correctness=False,
        performance_claim=False, production_authority=False, original_gpu_publication_unchanged=True,
        limitations=['Metrics are copied from the authenticated bounded comparison; tensor arithmetic is not rerun here.',
            'Consumed pins are identity metadata; this publisher does not read their tensor buffers.',
            'Existing GPU publication retains ownership/admission scope. No threshold or numerical acceptance is granted.'])
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'}, 'fresh output/root README only')
        if os.path.lexists(Q / 'README.md'): H.body(Q / 'README.md')
    for p in list(H.CHECKED): H.body(p)
    outputs = {name: raw for name, (raw, _) in COPIES.items()}
    outputs.update({'table.json': table_raw, 'table.md': table_md, 'result.json': H.json_bytes(result)}); Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in outputs.items():
        target = Q / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream: stream.write(raw)
        require(H.body(target) == raw, 'published exact body')
    for p in list(H.CHECKED): H.body(p)
    print(H.json_bytes(dict(result=pin(H, Q / 'result.json'), table=pin(H, Q / 'table.json'), files=len(outputs))).decode(), end='')


if __name__ == '__main__': main()
