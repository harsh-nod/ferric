"""Publish recorded AR4 diagnostics, not a tensor recomputation or native launch."""
import ast
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/projection-ar4-framework-evidence-v228-v1')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/rope-indexed-ar4-comparison-v1'
PACKAGE = 'p228-rope-indexed-ar4-comparison-v1'
SELF = L / 'proposals/p228-rope-indexed-ar4-comparison-publication-v1/publish.py'
HELPER = F / 'qualification/layer0-native-capture-v1/tools/publish.py'
HELPER_SHA = '3181b2f41a51ee94c331b137f2807580c326f7f6dc53c429163d8465309e2cb8'
MANIFEST_SHA = '1227cc775f78a8fd34449dffedb9a7fe0353d30c1faa8d23b154d617b3ea97e9'
COMPARISON = dict(path=str(E / 'rope-indexed-ar4-comparison-v228-v1/complete.json'), bytes=288269,
    sha256='9bf4d8e3504268c56656ccaf26d082d84156ac9f060cc7904293afa91d474a8c')
NATIVE = dict(path=str(E / 'prefix-rope-indexed-ar4-gpu-v228-v1/complete.json'), bytes=1128265,
    sha256='30119e94b7ef116a8845b2b8819122f039dd1a7688b28b9e7494de3f7d408749')
BASELINE_SHA = '15938580d218f855883a589c819d532bbf941a02c4677cb29928f7bf7106d1cb'
REFERENCE_SHA = '00952244362ad51d241d179f741ae5ae61ff8acfcb3fd160b9dc64ce3b5f699e'
OWNER_SHA = '94403d351120c0eb756f5660e333682ca0f47c4c6c1ac0f3cf6e9c10db896771'
MEMBERS = {'README.md', 'comparison.py', 'run.py', 'diagnostics.py', 'test_comparison.py'}
HELPERS = {'stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'decode_validation.py': '0eb96d4ac5e10e7f6ac10b018692c56f969286949681be336cb6040d56f01422'}
COPIES = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def helper():
    raw = HELPER.read_bytes(); info = HELPER.lstat()
    require(HELPER.resolve(strict=True) == HELPER and stat.S_ISREG(info.st_mode)
        and info.st_nlink == 1 and len(raw) == info.st_size <= 1 << 20
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact existing data-only publication reader')
    value = types.ModuleType('retained_publication_data'); value.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), value.__dict__); value.body(HELPER)
    return value


def pin(H, path, original=None):
    raw = H.body(path)
    return dict(path=str(original or path), bytes=len(raw), sha256=H.digest(raw))


def local(H, original):
    record = H.normalize(original); relative = Path(record['path']).relative_to(E)
    if relative.parts[0] in ('projection-ar4-framework-launch-v228-v1', 'projection-ar4-framework-reference-v228-v1'):
        path = W / relative
    elif relative.parts[0].startswith(('p225-', 'p227-', 'p228-')):
        path = L / 'proposals' / relative
    else:
        path = L / relative
    require(pin(H, path, record['path']) == record, 'exact locally retained original body: ' + record['path'])
    return path


def copy(H, path, name, original=None, expected=None):
    record = pin(H, path, original)
    require(expected is None or record['sha256'] == expected, 'caller-authenticated copy')
    require(name not in COPIES and not Path(name).is_absolute() and '..' not in Path(name).parts, 'distinct relative output')
    COPIES[name] = (H.body(path), record)
    return record


def rows(value):
    positions, tensors = value['positions'], value['tensors']
    require(len(positions) == 4 and len(tensors) == value['total_tensor_slots'] == 152, 'four positions and152 tensor slots')
    history = {'baseline': True, 'candidate': True}
    for pos, row in enumerate(positions):
        require(row['position'] == pos and all(type(row[k]) is int and 0 <= row[k] < 151936
            for k in ('framework_input_token', 'framework_output_token', 'baseline_input_token',
                      'baseline_output_token', 'candidate_input_token', 'candidate_output_token')), 'actual token fields')
        for role in ('framework', 'baseline', 'candidate'):
            expected = 9112 if pos == 0 else positions[pos - 1][role + '_output_token']
            require(row[role + '_input_token'] == expected, 'each genuine own-output recurrence')
        for role in history:
            history[role] &= row[role + '_input_token'] == row['framework_input_token']
            require(row[role + '_same_history'] is history[role], 'history comparability never recovers')
        require(row['before_after_comparable'] is all(history.values())
            and row['needs_conditional_reference'] is (not history['candidate']), 'same-reference before/after scope')
        for index, name in enumerate([*(f'layer{n}-hidden' for n in range(36)), 'final-norm', 'logits']):
            item = tensors[pos * 38 + index]
            require(item['position'] == pos and item['tensor'] == name
                and item['same_history_before_after'] is all(history.values())
                and item['needs_conditional_reference'] is (not history['candidate']), 'closed ordered tensor row')
            for role in history:
                metric = item[role]
                require((metric is None) is (not history[role]), 'no tensor metric for incomparable history')
                if metric is None:
                    continue
                count = 151936 if index == 37 else 4096
                require(metric['elements'] == count and type(metric['exact_words']) is int
                    and 0 <= metric['exact_words'] <= count and type(metric['zero_reference_norm']) is bool
                    and type(metric['max_bf16_steps']) is int and metric['max_bf16_steps'] >= 0, 'metric geometry')
                for key in ('max_abs_error', 'rmse', 'relative_l2'):
                    number = metric[key]
                    require((key == 'relative_l2' and number is None and metric['zero_reference_norm'])
                        or (type(number) in (int, float) and math.isfinite(number) and number >= 0), 'finite recorded error')
            expected = None
            if all(history.values()):
                expected = {key: item['candidate'][key] - item['baseline'][key]
                    if item['candidate'][key] is not None and item['baseline'][key] is not None else None
                    for key in ('max_abs_error', 'relative_l2', 'rmse', 'exact_words', 'max_bf16_steps')}
            require(item['candidate_minus_baseline'] == expected, 'reported metric subtraction only, no payload recomputation')
    for key, predicate in (('candidate_tensor_rows', lambda r: r['candidate'] is not None),
                           ('baseline_tensor_rows', lambda r: r['baseline'] is not None),
                           ('comparable_delta_rows', lambda r: r['same_history_before_after'])):
        require(value[key] == sum(predicate(row) for row in tensors), 'actual comparable census')
    require(value['all_four_output_tokens_equal'] is all(row['framework_output_token'] == row['candidate_output_token']
        for row in positions), 'token equality summary is observational')
    return dict(positions=positions, tensors=tensors, candidate_tensor_rows=value['candidate_tensor_rows'],
        comparable_delta_rows=value['comparable_delta_rows'], numerical_acceptance=False, acceptance_threshold=None)


def markdown(table):
    lines = ['# Recorded AR4 Framework Comparison', '',
        'Recorded CPU diagnostics only. A dash means incomparable input history, not zero error.', '',
        '| Position | Tensor | Old max abs | New max abs | Old relative L2 | New relative L2 | New exact words |',
        '| ---: | --- | ---: | ---: | ---: | ---: | ---: |']
    def metric(row, role, key):
        value = row[role]
        return '-' if value is None or value[key] is None else format(value[key], '.10g')
    for row in table['tensors']:
        lines.append('| %d | %s | %s | %s | %s | %s | %s |' % (row['position'], row['tensor'],
            metric(row, 'baseline', 'max_abs_error'), metric(row, 'candidate', 'max_abs_error'),
            metric(row, 'baseline', 'relative_l2'), metric(row, 'candidate', 'relative_l2'),
            metric(row, 'candidate', 'exact_words')))
    return ('\n'.join(lines) + '\n').encode()


def primary_tests(H, path, sha, manifest):
    require(path == L / 'rope-indexed-ar4-comparison-root-test-observation-v228-v1.json', 'actual primary observation path')
    record = copy(H, path, 'tests/primary-observation.json', expected=sha)
    value = H.parse(H.body(path))
    require(value['schema'] == 'ferric-primary-agent-test-observation-v1'
        and value['remote_supervisor_receipt'] is False and value['tests_passed'] == 18
        and value['tests_failed'] == 0 and value['source_hashes_unchanged'] is True
        and all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim')),
        'actual primary SSH observation, not a remote supervisor receipt')
    expected = {str(E / PACKAGE / row['path']): row['sha256'] for row in manifest['files']}
    expected[str(E / PACKAGE / 'manifest.json')] = MANIFEST_SHA
    for key in ('prehash_tool_result', 'posthash_tool_result'):
        tool = value[key]
        require(tool['exit_code'] == 0, 'successful primary hash command')
        matches = re.findall(r'^([0-9a-f]{64})  (/[^\r\n]+)\r?$', tool['output'], re.M)
        require(len(matches) == 6 and {path: digest for digest, path in matches} == expected,
            'all six actual pre/post package hashes, including manifest')
    source = H.body(L / 'proposals' / PACKAGE / 'test_comparison.py')
    names = sorted('test_comparison.' + cls.name + '.' + method.name
        for cls in ast.parse(source).body if isinstance(cls, ast.ClassDef)
        for method in cls.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'))
    require(len(names) == len(set(names)) == manifest['authored_tests']['count'] == 18
        and sorted(name.rsplit('.', 1)[1] for name in names) == sorted(manifest['authored_tests']['names']),
        'exact authored eighteen-test inventory')
    terminal = value['terminal_tool_result']
    require(terminal['exit_code'] == 0, 'observed primary natural terminal zero')
    log = value['initial_tool_result']['output'] + terminal['output']
    observed = []
    for method, qualified in re.findall(r'^(test_[^\s]+) \(([^()\n]+)\) \.\.\. ok$', log, re.M):
        observed.append(qualified if qualified.endswith('.' + method) else qualified + '.' + method)
    require(sorted(observed) == names and len(observed) == 18
        and re.search(r'\nRan 18 tests in [0-9.]+s\n\nOK\n?\Z', log), 'actual full named passing transcript')
    return dict(observation=record, passed=18, failed=0, named_tests=names,
        source_hashes_before_after_equal=True, remote_supervisor_receipt=False)


def main(args):
    require(len(args) == 4 and not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ,
        'LOCAL_COMPARISON_COMPLETE ACTUAL_SHA PRIMARY_TEST_OBSERVATION ACTUAL_TEST_SHA')
    H = helper()
    require(Path(__file__).resolve(strict=True) == SELF, 'selected publisher source')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'},
            'fresh qualification directory with optional root README only')
        if (Q / 'README.md').exists(): H.body(Q / 'README.md')
    complete_path = Path(args[0])
    require(complete_path == L / 'rope-indexed-ar4-comparison-complete-v228-v1.json'
        or complete_path == L / 'rope-indexed-ar4-comparison-v228-v1/complete.json', 'actual retained comparison namespace')
    complete = copy(H, complete_path, 'raw/complete.json', COMPARISON['path'], args[1])
    require(complete == COMPARISON, 'actual observed comparison identity')
    value = H.parse(H.body(complete_path))
    require(value['schema'] == 'ferric-p228-rope-indexed-ar4-comparison-observation-v1'
        and value['completed'] is True and value['errors'] == [] and value['input_source_postchecks_passed'] is True
        and value['data_only'] is True and all(value[key] is False for key in ('gpu_execution', 'model_execution',
            'native_or_framework_controllers_imported', 'full_owner_audits_replayed', 'numerical_acceptance',
            'full_model_correctness', 'sustained_2048_256', 'performance_claim', 'production_authority'))
        and value['acceptance_threshold'] is None, 'actual completed data-only comparison, no acceptance')
    package = L / 'proposals' / PACKAGE
    manifest_pin = copy(H, package / 'manifest.json', 'source/manifest.json', E / PACKAGE / 'manifest.json', MANIFEST_SHA)
    manifest = H.parse(H.body(package / 'manifest.json'))
    require(value['source_manifest'] == manifest_pin and {row['path'] for row in manifest['files']} == MEMBERS
        and len(manifest['files']) == 5, 'frozen five-source package')
    sources = {}
    for row in manifest['files']:
        record = copy(H, package / row['path'], 'source/' + row['path'], E / PACKAGE / row['path'], row['sha256'])
        require(record['bytes'] == row['bytes'], 'source extent')
        sources[row['path']] = record
    for name, sha in HELPERS.items():
        record = pin(H, L / 'proposals/p228-rope-indexed-ar4-gpu-v1' / name, E / 'p228-rope-indexed-ar4-gpu-v1' / name)
        require(record['sha256'] == sha, 'unchanged data validator')
        sources['validator/' + name] = record
    require(value['sources'] == sources and value['controller'] == sources['run.py'], 'all actual source joins')
    checked, consumed = {}, value['consumed']
    require(type(consumed) is list and 1 <= len(consumed) <= 128, 'bounded consumed data ledger')
    for row in consumed:
        require(set(row) == {'original', 'retained'}, 'preserved original and transport identity')
        original, retained = H.normalize(row['original']), H.normalize(row['retained'])
        require((original['bytes'], original['sha256']) == (retained['bytes'], retained['sha256'])
            and original['path'] not in checked, 'unique transport-preserved input')
        path = local(H, original)
        checked[original['path']] = dict(original=original, comparison_retained=retained, local=pin(H, path))
    result = value['comparison']
    require(result['native_complete'] == NATIVE and result['baseline_complete']['sha256'] == BASELINE_SHA
        and result['framework_reference']['sha256'] == REFERENCE_SHA and result['framework_owner']['sha256'] == OWNER_SHA
        and result['retained_framework_payloads_rehashed'] == result['retained_native_payloads_rehashed'] == 8
        and result['native_structure_replayed'] is True and all(result[key] is False for key in
            ('full_native_owner_audits_replayed', 'full_framework_owner_audits_replayed', 'model_or_image_bodies_rehashed',
             'source_image_admission_replayed', 'gpu_execution', 'model_execution', 'full_model_correctness',
             'performance_claim', 'production_authority', 'reference_reused_conditionally',
             'conditional_reference_generated', 'causal_attribution', 'numerical_acceptance')),
        'pinned actual native and genuine reference with explicit limits')
    for record in (NATIVE, result['baseline_complete'], result['framework_reference'], result['framework_owner'],
                   manifest_pin, value['inputs'], *sources.values()):
        require(checked[record['path']]['original'] == record, 'all direct source/data prerequisites actually consumed')
    table = rows(result)
    inputs = copy(H, local(H, value['inputs']), 'raw/inputs.json', value['inputs']['path'])
    tests = primary_tests(H, Path(args[2]), args[3], manifest)
    publisher = copy(H, SELF, 'tools/publish.py')
    generated = {'table.json': (json.dumps(table, indent=2, sort_keys=True, allow_nan=False) + '\n').encode(),
        'table.md': markdown(table)}
    for path in list(H.CHECKED): H.body(path)
    Q.mkdir(exist_ok=True)
    ledger = {}
    for name, (raw, original) in COPIES.items():
        path = Q / name; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(raw)
        require(H.body(path) == raw, 'copied output bytes')
        ledger[name] = dict(original=original, bytes=len(raw), sha256=H.digest(raw))
    for name, raw in generated.items():
        with (Q / name).open('xb') as stream: stream.write(raw)
        require(H.body(Q / name) == raw, 'generated metadata bytes')
        ledger[name] = dict(derived_from=complete, bytes=len(raw), sha256=H.digest(raw))
    summary = dict(schema='ferric-p228-rope-indexed-ar4-comparison-publication-v1', comparison=complete,
        inputs=inputs, package=manifest_pin, primary_tests=tests, publisher=publisher, files=ledger,
        data_reader=pin(H, HELPER),
        consumed_original_bodies_rehashed_locally=checked, native_complete=NATIVE,
        framework_reference=result['framework_reference'], baseline_complete=result['baseline_complete'],
        positions=table['positions'], candidate_tensor_rows=table['candidate_tensor_rows'],
        comparable_delta_rows=table['comparable_delta_rows'], all_four_output_tokens_equal=result['all_four_output_tokens_equal'],
        source_postchecks_passed=True, raw_payloads_published=False, tensor_metrics_recomputed=False,
        recorded_metric_delta_subtractions_checked=True, test_or_gpu_execution=False,
        full_owner_audits_replayed=False, numerical_acceptance=False, acceptance_threshold=None,
        full_model_correctness=False, performance_claim=False, production_authority=False,
        limitations=['Recorded actual CPU metrics; this publisher does not recompute tensor arithmetic.',
            'Native/framework receipts are pinned; full process/audit or source-image admission is not replayed.',
            'No later-position comparison is valid after input-history divergence without a genuine conditional reference.',
            'Primary test output observation is not a remote process-supervisor receipt.',
            'Private BF16 buffers are rehashed from retained originals but are not copied into Git.'])
    for path in list(H.CHECKED): H.body(path)
    raw = (json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    with (Q / 'result.json').open('xb') as stream: stream.write(raw)
    expected = set(ledger) | {'result.json'} | ({'README.md'} if (Q / 'README.md').exists() else set())
    require({str(p.relative_to(Q)) for p in Q.rglob('*') if p.is_file()} == expected, 'closed publication tree')
    print(json.dumps(dict(result=pin(H, Q / 'result.json'), copied=len(COPIES), generated=len(generated)), sort_keys=True))


if __name__ == '__main__':
    main(sys.argv[1:])
