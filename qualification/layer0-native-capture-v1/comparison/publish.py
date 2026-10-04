"""Publish the completed current-stage diagnostic; never import or rerun it."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
R = E.parents[1]
Q = Path('/home/harsh/ferric-p227-integration/qualification/layer0-native-capture-v1')
OUT = Q / 'comparison'
BASE_SHA = '83e7ae6ea8ff8ae2df1e1e20a21cecade33f58c9800a74800b524c54790a9d5f'
COMPLETE_SHA = '758c2479a3f5a5f78e6ff088adb70a31ca5fb19c86e5a25cf718c3e1e46f4dab'
TRANSFER_SHA = '14086ce1ded18db9e8e243d92ef9a98c8e0709213778ff81fb14d79b7d528e0d'
PURES = {
    'current': ('7af96a474c6d0d27b9f502cc8e6f7e70e4dd22e69d64e24ab131968259b5236c', 18,
        'ferric-p228-layer0-current-diagnostic-pure-v1', 'test_current.py'),
    'runner': ('3bc87c2eee5a0cceca0aec4dda4a436b17838aace64d66bffce5f571db35e5b3', 15,
        'ferric-p228-layer0-current-comparison-pure-v1', 'test_run.py'),
}
CHECKED = {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def body(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local source')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 16 << 20,
        'bounded regular retained input')
    raw = path.read_bytes(); after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    pin = dict(bytes=len(raw), sha256=sha(raw))
    require(stamp(before) == stamp(after) and len(raw) == before.st_size
        and CHECKED.setdefault(path, pin) == pin, 'stable source bytes')
    return raw


def local(original):
    path = Path(original)
    require(path.is_absolute() and str(path) == original and '..' not in path.parts, 'original path')
    prompt = R / 'evidence/finite-two-forward-v223/p224-prompt-v1'
    if path.parent == prompt and path.name in ('prompt.u32le', 'prompt-manifest.json'):
        return L / 'p223-framework-reference-inputs' / path.name
    if path == R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py':
        return L / 'run_p217_mi350.py'
    require(path.is_relative_to(E), 'closed retained evidence namespace')
    relative = path.relative_to(E)
    return L / ('proposals' if relative.parts[0].startswith('p228-') else '') / relative


def checked(pin):
    require(type(pin) is dict and set(pin) == {'path', 'bytes', 'sha256'}
        and type(pin['bytes']) is int and 0 <= pin['bytes'] <= 16 << 20
        and type(pin['sha256']) is str and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'FilePin')
    path = local(pin['path']); raw = body(path)
    require(len(raw) == pin['bytes'] and sha(raw) == pin['sha256'], 'actual retained byte identity')
    return path, raw


def doc(pin):
    return parse(checked(pin)[1])


def pin(path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals': relative = Path(*relative.parts[1:])
    raw = body(path)
    return dict(path=str(E / relative), bytes=len(raw), sha256=sha(raw))


def main():
    require(len(sys.argv) == 1 and not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ,
        'ordinary Python required; only actual fixed completed evidence is published')
    require(Q.resolve(strict=True) == Q and Q.is_dir(), 'existing native qualification')
    original_result = body(Q / 'result.json')
    require(sha(original_result) == BASE_SHA, 'original published GPU result unchanged')
    qualification = parse(original_result)
    complete_path = L / 'layer0-current-comparison-v228-v1/complete.json'
    raw = body(complete_path)
    require(len(raw) == 180610 and sha(raw) == COMPLETE_SHA, 'actual completed CPU comparison')
    value = parse(raw)
    require(value['schema'] == 'ferric-p228-layer0-current-comparison-observation-v1'
        and value['completed'] is True and value['source_postchecks_passed'] is True
        and value['native_owned_leaves_replayed'] == 7 and value['native_audits_replayed'] == 6
        and value['native_structural_replayed'] is True and value['framework_owned_leaf_results_rechecked'] == 21
        and value['native_outer'] == qualification['gpu_observation']
        and all(value[k] is False for k in ('all_transitive_admission_inputs_replayed',
            'current_gpu_audits_performed', 'gpu_execution', 'native_process_launched',
            'numerical_acceptance', 'full_model_correctness', 'performance_claim', 'production_authority')),
        'actual comparison scope, not new GPU or numerical acceptance')
    plan = doc(value['plan'])
    transfer_path = L / 'layer0-current-comparison-inputs-v228-v1/transfer.json'
    transfer_raw = body(transfer_path)
    require(sha(transfer_raw) == TRANSFER_SHA, 'actual root transfer inventory')
    transfer = parse(transfer_raw)
    require(transfer['plan'] == value['plan'] and transfer['native_outer'] == value['native_outer']
        and len(transfer['mapped']) == len(plan['transport']) == 64, 'actual transported input plan')
    require({row['original']['path']: row['retained'] for row in transfer['mapped']} == plan['transport'],
        'full map joins actual transfer inventory')
    consumed = {}
    require(len(value['consumed']) == 260, 'actual consumed read census')
    for pair in value['consumed']:
        require(set(pair) == {'original', 'retained'}, 'original/transported pin pair')
        original, retained = pair['original'], pair['retained']
        require(original['path'] not in consumed and retained == plan['transport'].get(original['path'], original)
            and (original['bytes'], original['sha256']) == (retained['bytes'], retained['sha256']),
            'only declared byte-identical transport aliases')
        require(checked(original)[1] == checked(retained)[1], 'both identities independently rehashed locally')
        consumed[original['path']] = original
    require(set(plan['transport']) <= set(consumed), 'all staged inputs actually consumed')
    for row in (value['controller'], value['runner_tests'], value['comparison_tests'], value['plan'],
                value['native_outer'], value['framework_outer'], *value['source_pins'].values()):
        require(consumed.get(row['path']) == row, 'consumed prerequisite join')
    comparison = value['comparison']
    require(comparison['schema'] == 'ferric-p228-layer0-current-intermediate-diagnostic-v1'
        and comparison['comparable_rows'] == 24 and comparison['input_token'] == 9112 and comparison['position'] == 0
        and comparison['both_rank_final_hidden_match_current_tf4'] is True
        and comparison['current_tf4_final_hidden_sha256'] == qualification['current_tf4_hidden_sha256']
        and comparison['genuine_independent_framework_outputs'] is True
        and comparison['current_V7_Down2_intermediate_evidence'] is True
        and comparison['acceptance_threshold'] is None and comparison['fp32_partials_finite'] is True
        and len(comparison['excluded_fp32_partials']) == 4
        and all(comparison[k] is False for k in ('ordering_is_a_causal_proof', 'fp32_partials_compared_to_full_bf16',
            'candidate_intermediate_inputs', 'conditional_replay_performed', 'receipt_authentication',
            'numerical_acceptance', 'full_model_correctness', 'production_authority', 'performance_measured', 'gpu_execution')),
        'genuine cumulative-chain diagnostic without fitted tolerance')
    rows = comparison['comparisons']; order = comparison['observation_order']
    require(len(order) == 12 and len(rows) == 24
        and [(r['stage'], r['rank'], r['observable_order']) for r in rows]
            == [(stage, rank, index) for index, stage in enumerate(order) for rank in range(2)], 'all24 stage/rank rows')
    for row in rows:
        require(type(row['elements']) is int and type(row['exact_words']) is int
            and 0 <= row['exact_words'] <= row['elements']
            and row['byte_equal'] is (row['candidate_sha256'] == row['reference_sha256'])
            and row['byte_equal'] is (row['exact_words'] == row['elements']), 'actual equality summaries')
    first = next(row for row in rows if not row['byte_equal'])
    expected = dict(stage=first['stage'], observable_order=first['observable_order'],
        ranks=[row['rank'] for row in rows if row['observable_order'] == first['observable_order'] and not row['byte_equal']])
    require(comparison['earliest_observable_divergence'] == expected == dict(stage='qkv', observable_order=1, ranks=[0]),
        'actual earliest observed difference, not cause')
    copies = {'complete.json': (complete_path, pin(complete_path)),
        'inputs/plan.json': (checked(value['plan'])[0], value['plan']),
        'inputs/transfer.json': (transfer_path, pin(transfer_path))}
    for group, receipt_pin in (('current', value['comparison_tests']), ('runner', value['runner_tests'])):
        digest, count, schema, test_name = PURES[group]
        require(receipt_pin['sha256'] == digest, 'actual separate pure receipt')
        pure = doc(receipt_pin)
        require(pure['schema'] == schema and pure['passed'] is True and pure['tests'] == count
            and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True,
            'actual named synthetic policy suite')
        sources = doc(pure['sources_before'])
        require(sources == doc(pure['sources_after'])
            and set(sources) == (set(value['source_pins']) if group == 'current' else {'run.py', 'test_run.py'}),
            'tested source roster unchanged')
        if group == 'current': require(sources == value['source_pins'], 'actual comparator source generation')
        else: require(sources['run.py'] == value['controller'], 'actual executed runner generation')
        for name, row in sources.items():
            require(consumed.get(row['path']) == row, 'tested source consumed by actual run')
            copies[('source/' if group == 'current' else 'runner/') + name] = (checked(row)[0], row)
        names = sorted(n.name for n in ast.walk(ast.parse(checked(sources[test_name])[1]))
            if isinstance(n, ast.FunctionDef) and n.name.startswith('test_'))
        transcript = checked(pure['transcript'])[1].decode()
        passed = re.findall(r'^(test_\w+) \([^\r\n]+\) \.\.\. ok$', transcript, re.MULTILINE)
        require(len(names) == count and sorted(passed) == names and len(set(passed)) == count
            and re.search(r'^Ran ' + str(count) + r' tests in [^\n]+\n\nOK\s*$', transcript, re.MULTILINE),
            'actual individually named test successes')
        for name, row in (('complete.json', receipt_pin), ('sources-before.json', pure['sources_before']),
                          ('sources-after.json', pure['sources_after']), ('tests.log', pure['transcript'])):
            copies[('pure-comparator/' if group == 'current' else 'pure-adapter/') + name] = (checked(row)[0], row)
        row = pure['controller']; copies['tools/' + Path(row['path']).name] = (checked(row)[0], row)
    assembler = L / 'proposals/p228-layer0-current-comparison-inputs-v1/prepare.py'
    require(sha(body(assembler)) == '5e23c201292ea16f642e2ac5aa26ca807aca1e1457d05dbf4cb50c08f70a5897',
        'actual root assembly source')
    copies['tools/prepare_inputs.py'] = (assembler, dict(path=str(assembler), **CHECKED[assembler]))
    own = Path(__file__).resolve(strict=True); raw = body(own)
    copies['publish.py'] = (own, dict(path=str(own), bytes=len(raw), sha256=sha(raw)))
    readme = None
    if os.path.lexists(OUT):
        require(OUT.resolve(strict=True) == OUT and OUT.is_dir() and {p.name for p in OUT.iterdir()} == {'README.md'},
            'only separately root-authored README may preexist')
        raw = body(OUT / 'README.md'); readme = dict(bytes=len(raw), sha256=sha(raw))
    for target, (source, _) in copies.items():
        require(not os.path.lexists(OUT / target) and source.suffix not in ('.bin', '.bf16', '.safetensors'),
            'fresh text-only supplemental target')
        body(source).decode('utf-8')
    for source in list(CHECKED): body(source)
    OUT.mkdir(mode=0o700, exist_ok=True)
    retained = {}
    for name, (source, original) in sorted(copies.items()):
        destination = OUT / name; destination.parent.mkdir(parents=True, exist_ok=True)
        require(destination.parent.resolve(strict=True) == destination.parent, 'canonical output parent')
        raw = body(source)
        with destination.open('xb') as stream: stream.write(raw)
        require(body(destination) == raw, 'published bytes equal actual source')
        retained[name] = dict(bytes=len(raw), sha256=sha(raw), source=original)
    for source in list(CHECKED): body(source)
    require(body(Q / 'result.json') == original_result, 'original native result remains immutable')
    report = dict(schema='ferric-p228-layer0-current-comparison-publication-v1', completed=True,
        parent_result=dict(path='../result.json', bytes=len(original_result), sha256=BASE_SHA),
        actual_comparison=pin(complete_path), original_transport_pairs_rehashed=260,
        native_owned_leaves_replayed=7, native_audits_replayed=6, comparable_bf16_rows=24,
        excluded_fp32_partial_rows=4, pure_suites=dict(current=18, runner=15),
        earliest_observable_divergence=expected, retained=retained, root_authored_readme=readme,
        original_source_readmes_omitted=True, raw_tensor_bodies_published=False, diagnostics_reexecuted=False,
        all_transitive_admission_inputs_replayed=False, gpu_execution=False, numerical_acceptance=False,
        ordering_is_a_causal_proof=False, performance_claim=False, full_model_correctness=False, production_authority=False)
    raw = (json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    with (OUT / 'result.json').open('xb') as stream: stream.write(raw)
    print(json.dumps(dict(result=str(OUT / 'result.json'), bytes=len(raw), sha256=sha(raw), files=len(retained))), flush=True)


if __name__ == '__main__':
    main()
