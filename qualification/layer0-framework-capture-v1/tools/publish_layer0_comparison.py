"""Publish an actual layer0 diagnostic supplement without rerunning its code."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import struct
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
Q = Path('/home/harsh/ferric-p227-integration/qualification/layer0-framework-capture-v1')
PACKAGE = 'p228-layer0-genuine-comparison-v1'
RESULT_SHA = '428658f631b531efe3317525645224e368882618fe72f6ad2f884308edb32b9f'
SOURCES = {
    'compare.py': '1598e22a3460a9ed2c350fb5d2b5fec6b8c37648b53bb2f1fc714c1fb3506d3f',
    'test_compare.py': 'd8b1071ddbb250965baafc605076bd76cb0bb68c1d30fff2cc0df57182c0c475',
    'diagnostics.py': '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf',
}
NATIVE = 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1'
TRANSPORT = 'layer0-comparison-native-inputs-v228-v1'
ALIASES = {str(E / NATIVE / 'complete.json'): str(E / TRANSPORT / 'complete.json'),
    str(E / NATIVE / 'native/observation-0.bin'): str(E / TRANSPORT / 'observation-0.bin'),
    str(E / 'prefix-down2-clock-tf4-inputs-v228-v1/request.json'): str(E / TRANSPORT / 'request.json')}
PROMPT = E.parent / 'finite-two-forward-v223/p224-prompt-v1'
CHECKED = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=lambda _: require(False, 'nonfinite JSON'))


def body(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local body')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 8 << 20,
            'bounded unaliased retained body')
    raw = path.read_bytes(); after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    value = dict(bytes=len(raw), sha256=sha(raw))
    require(stamp(before) == stamp(after) and len(raw) == before.st_size
            and CHECKED.setdefault(path, value) == value, 'unchanged local bytes')
    return raw


def checked(row):
    require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'}
        and type(row['bytes']) is int and 0 <= row['bytes'] <= 8 << 20
        and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'closed FilePin')
    path = Path(row['path'])
    require(path.is_absolute() and '..' not in path.parts and str(path) == row['path'], 'canonical pin spelling')
    if path.parent == PROMPT and path.name in ('prompt.u32le', 'prompt-manifest.json'):
        local = L / 'p223-framework-reference-inputs' / path.name
    else:
        require(path.is_relative_to(E), 'closed retained remote namespace')
        relative = path.relative_to(E)
        local = L / ('proposals' if relative.parts[0] == PACKAGE else '') / relative
    raw = body(local)
    require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'actual original or transported bytes')
    return local, raw


def document(row):
    return parse(checked(row)[1])


def local_pin(path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals': relative = Path(*relative.parts[1:])
    raw = body(path)
    return dict(path=str(E / relative), bytes=len(raw), sha256=sha(raw))


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('comparison_sha256')
    args = parser.parse_args()
    require(re.fullmatch('[0-9a-f]{64}', args.comparison_sha256), 'actual comparison digest')
    original_result = body(Q / 'result.json')
    require(sha(original_result) == RESULT_SHA, 'original published qualification unchanged')
    qualification = parse(original_result)
    path = L / 'layer0-genuine-comparison-v228-v1/complete.json'
    raw = body(path); require(sha(raw) == args.comparison_sha256, 'actual completed CPU comparison')
    result = parse(raw)
    require(result['schema'] == 'ferric-p228-layer0-genuine-comparison-observation-v1'
        and result['completed'] is True and result['source_postchecks_passed'] is True
        and result['framework_owned_leaf_results_checked'] == 21
        and all(result[k] is False for k in ('all_transitive_inputs_replayed', 'gpu_execution',
            'numerical_acceptance', 'full_model_correctness', 'performance_measured', 'production_authority')),
        'actual diagnostic scope without acceptance')
    require(result['framework_outer'] == qualification['owner_complete'], 'same published framework execution')
    consumed = {}
    require(len(result['consumed']) == 177, 'actual declared comparison read census')
    for pair in result['consumed']:
        require(set(pair) == {'original', 'retained'}, 'closed transport pair')
        original, retained = pair['original'], pair['retained']
        require(original['path'] not in consumed and retained['path'] == ALIASES.get(original['path'], original['path'])
            and (original['bytes'], original['sha256']) == (retained['bytes'], retained['sha256']),
            'only the three declared transport aliases, with identical bytes')
        require(checked(original)[1] == checked(retained)[1], 'both locally retained identities match')
        consumed[original['path']] = original
    require(set(ALIASES) <= set(consumed), 'all three native transports replayed')
    for row in [result['controller'], result['pure'], result['framework_outer'], *result['source_pins'].values()]:
        require(consumed.get(row['path']) == row, 'consumed prerequisite identity')
    require(set(result['source_pins']) == set(SOURCES)
        and all(result['source_pins'][name]['sha256'] == digest for name, digest in SOURCES.items()),
        'exact tested comparator generation')
    pure = document(result['pure'])
    require(pure['schema'] == 'ferric-p228-layer0-genuine-comparison-pure-v1' and pure['passed'] is True
        and pure['tests'] == 14 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
        and all(pure[k] is False for k in ('framework_execution', 'gpu_execution', 'numerical_acceptance',
            'performance_claim', 'production_authority', 'runtime_audit_executed')), 'actual fourteen pure tests')
    require(document(pure['sources_before']) == document(pure['sources_after']) == result['source_pins'],
            'source before/after joins')
    names = sorted(n.name for n in ast.walk(ast.parse(checked(result['source_pins']['test_compare.py'])[1]))
                   if isinstance(n, ast.FunctionDef) and n.name.startswith('test_'))
    transcript = checked(pure['transcript'])[1].decode('utf-8')
    actual = re.findall(r'^(test_\w+) \([^\r\n]+\) \.\.\. ok$', transcript, re.MULTILINE)
    require(len(names) == 14 and sorted(actual) == names and len(set(actual)) == 14
        and re.search(r'^Ran 14 tests in [^\n]+\n\nOK\s*$', transcript, re.MULTILINE), 'actual named passing tests')
    comparison = result['comparison']; inputs = comparison['inputs']
    require(comparison['schema'] == 'ferric-p228-layer0-genuine-primary-diagnostic-v1'
        and comparison['position'] == 0 and comparison['input_token'] == 9112
        and comparison['original_framework_matches_new'] is True and comparison['genuine_independent_framework_outputs'] is True
        and comparison['acceptance_threshold'] is None and all(comparison[k] is False for k in
            ('candidate_intermediate_inputs', 'conditional_replay_performed', 'receipt_authentication',
             'numerical_acceptance', 'full_model_correctness', 'production_authority', 'performance_measured', 'gpu_execution')),
        'genuine comparison remains diagnostic')
    for row in inputs.values(): require(consumed.get(row['path']) == row, 'actual comparison input identity')
    require(inputs['framework_capture'] == qualification['capture'], 'same published captured framework bytes')
    framework, original, native = (document(inputs[name]) for name in ('framework_capture', 'original_framework', 'current_native'))
    fresh = [checked(p['stages']['layer0-hidden']['pin'])[1] for p in framework['passes']]
    prior = [checked(p['cases'][0]['payload'])[1][:8192] for p in original['passes']]
    current = checked(native['retained_native']['observation-0.bin'])[1][:8192]
    require(len(fresh) == len(prior) == 2 and all(len(v) == 8192 for v in [*fresh, *prior, current])
        and fresh[0] == fresh[1] == prior[0] == prior[1] and current != fresh[0], 'actual genuine equality and native mismatch')
    for name, reference, candidate in (('original_framework_repeat', prior[0], fresh[0]),
                                       ('current_native', fresh[0], current)):
        row = comparison[name]
        exact = sum(a == b for a, b in zip(struct.iter_unpack('<H', reference), struct.iter_unpack('<H', candidate)))
        require(row['reference_sha256'] == sha(reference) and row['candidate_sha256'] == sha(candidate)
            and row['elements'] == 4096 and row['exact_words'] == exact and row['byte_equal'] is (reference == candidate),
            'actual compared slice hashes, equality and exact words')
    copies = {'comparison/complete.json': (path, local_pin(path))}
    for name, row in result['source_pins'].items(): copies['comparison-source/' + name] = (checked(row)[0], row)
    readme = L / 'proposals' / PACKAGE / 'README.md'
    copies['comparison-source/README.md'] = (readme, local_pin(readme))
    for name, row in [('complete.json', result['pure']), ('sources-before.json', pure['sources_before']),
                      ('sources-after.json', pure['sources_after']), ('tests.log', pure['transcript'])]:
        copies['comparison-tests/' + name] = (checked(row)[0], row)
    for row in (result['controller'], pure['controller']): copies['tools/' + Path(row['path']).name] = (checked(row)[0], row)
    copies['comparison/original-framework-reference.json'] = (checked(inputs['original_framework'])[0], inputs['original_framework'])
    own = Path(__file__).resolve(strict=True)
    copies['tools/publish_layer0_comparison.py'] = (own, local_pin(own))
    require(Q.resolve(strict=True) == Q and Q.is_dir() and not (Q / 'comparison-manifest.json').exists(), 'existing qualification, fresh supplement')
    for name, (source, _) in copies.items():
        require(not os.path.lexists(Q / name) and source.suffix not in ('.bin', '.bf16', '.safetensors'), 'new text-only supplement path')
        body(source).decode('utf-8')
    for source in list(CHECKED): body(source)
    ledger = []
    for name, (source, row) in sorted(copies.items()):
        target = Q / name; target.parent.mkdir(parents=True, exist_ok=True)
        require(target.parent.resolve(strict=True) == target.parent, 'no destination directory alias')
        raw = body(source)
        with target.open('xb') as stream: stream.write(raw)
        require(body(target) == raw, 'exact supplemental publication')
        ledger.append(dict(path=name, bytes=len(raw), sha256=sha(raw), source_pin=row))
    for source in list(CHECKED): body(source)
    require(body(Q / 'result.json') == original_result, 'original result and ledger never rewritten')
    manifest = dict(schema='ferric-p228-layer0-genuine-comparison-publication-v1',
        original_result=dict(path='result.json', bytes=len(original_result), sha256=RESULT_SHA),
        actual_comparison=local_pin(path), pure_tests=14, consumed_original_transport_pairs_rehashed=177,
        all_declared_comparison_bodies_rehashed=True, original_framework_matches_new=True,
        current_native_byte_equal=False, files=ledger, raw_tensor_bodies_published=False,
        diagnostics_reexecuted=False, all_transitive_inputs_replayed=False, numerical_acceptance=False,
        full_model_correctness=False, performance_claim=False, production_authority=False,
        limitations=['Metrics come from the authenticated CPU comparison; this publisher checks slices and exact-word counts.',
            'No fitted tolerance or numerical acceptance follows from the reported mismatch.',
            'Only the recorded consumed set is replayed, not all transitive execution prerequisites.'])
    target = Q / 'comparison-manifest.json'
    with target.open('xb') as stream: stream.write(encoded(manifest))
    print(json.dumps(dict(path=str(target), bytes=target.stat().st_size, sha256=sha(body(target)), files=len(ledger))), flush=True)


if __name__ == '__main__':
    main()
