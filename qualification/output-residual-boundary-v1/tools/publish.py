"""Publish the actual conditional boundary replay; no imports of tested code."""
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
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/output-residual-boundary-v1'
COMPLETE_SHA = '39c12232b3504eccbca898383b70244093fe930b9c37656cbec1515cd30eaf11'
PRIOR_SHA = '758c2479a3f5a5f78e6ff088adb70a31ca5fb19c86e5a25cf718c3e1e46f4dab'
PRIOR_PUBLIC_SHA = '276497eb91193efb46903fb86b4d30fd4470523b8aa99baf097ebd3cdaeacd93'
PURE_SHA = '20a16b03d188e2fb4e68d1d44a901ba6e0a73863b5a2402f152c7521f2e02516'
RUN_SHA = '9cfc3124f1acdff6146792bafecfd944c747ec36f335f4b8d25fc37d24183a87'
PREPARE_SHA = '21fb3e76bb1d969ec8017c00063a12062fcb78e89453ac525d22a185f876c988'
ASSEMBLY_SHA = 'cf85a1eeb909b6f9b9330f19b016fbbddd57eee61ac88748949abb7f4bef455a'
WRAPPER_SHA = 'a00ba58cca7f4ac43d0be56f573be6c21587d57dde48a5c8d0b228a96e7bc791'
SOURCES = {
    'boundary.py': 'a5fac7b587739999623167d81c163fc18ebe9a2baf986da1b192b597e7fb0b9d',
    'test_boundary.py': '2088ec73ed3763647bebc3cf295364b2a029d2c59ca3937d95df1430932d39fb',
    'residual_oracle.py': '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3',
}
COUNTS = [4096, 4096, 2886, 2888, 2888, 4094, 4096, 4093]
NAMES = ['native-formula-vs-native-rank0', 'native-formula-vs-native-rank1',
    'native-formula-vs-framework-residual', 'materialized-formula-vs-native-rank0',
    'materialized-formula-vs-native-rank1', 'materialized-formula-vs-framework-residual',
    'framework-add-vs-framework-residual', 'rounded-rank-sum-vs-framework-projection']
FALSE = ('prior_comparison_reexecuted', 'prior_ownership_checks_reexecuted',
    'all_transitive_admission_inputs_replayed', 'new_native_execution', 'framework_execution',
    'gpu_execution', 'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')
DIAGNOSTIC_FALSE = ('input_provenance_verified', 'native_embedding_equality_verified',
    'upstream_partial_numerics_checked', 'projection_gemm_equivalence_proven', 'causal_explanation_proven',
    'genuine_framework_chain_rerun', 'arithmetic_prerequisites_verified', 'runtime_premises_discharged',
    'numerical_acceptance', 'full_layer_numerics_accepted', 'full_model_acceptance',
    'gpu_execution', 'performance_claim', 'production_authority')
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
            require(key not in value, 'duplicate JSON key'); value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def body(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local input')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 16 << 20,
            'bounded regular retained bytes')
    raw = path.read_bytes(); after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_nlink, s.st_mtime_ns, s.st_ctime_ns)
    pin = dict(bytes=len(raw), sha256=sha(raw))
    require(stamp(before) == stamp(after) and len(raw) == before.st_size
        and CHECKED.setdefault(path, pin) == pin, 'unchanged retained file')
    return raw


def local(original):
    path = Path(original)
    require(path.is_absolute() and str(path) == original and '..' not in path.parts, 'original absolute path')
    if path.is_relative_to(L): return path
    prompt = R / 'evidence/finite-two-forward-v223/p224-prompt-v1'
    if path.parent == prompt and path.name in ('prompt.u32le', 'prompt-manifest.json'):
        return L / 'p223-framework-reference-inputs' / path.name
    if path == R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py': return L / 'run_p217_mi350.py'
    require(path.is_relative_to(E), 'closed retained evidence namespace')
    relative = path.relative_to(E)
    return L / ('proposals' if relative.parts[0].startswith('p228-') else '') / relative


def checked(pin):
    require(type(pin) is dict and set(pin) == {'path', 'bytes', 'sha256'} and type(pin['bytes']) is int
        and 0 <= pin['bytes'] <= 16 << 20 and type(pin['sha256']) is str
        and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'closed FilePin')
    path = local(pin['path']); raw = body(path)
    require(len(raw) == pin['bytes'] and sha(raw) == pin['sha256'], 'actual original/transported bytes')
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
            'ordinary Python; publish only the fixed actual completed replay')
    prior_public_path = F / 'qualification/layer0-native-capture-v1/comparison/result.json'
    prior_public_raw = body(prior_public_path)
    require(sha(prior_public_raw) == PRIOR_PUBLIC_SHA, 'unchanged published comparison ledger')
    prior_public = parse(prior_public_raw)
    complete_path = L / 'output-residual-boundary-v228-v1/complete.json'; raw = body(complete_path)
    require(len(raw) == 177419 and sha(raw) == COMPLETE_SHA, 'actual completed boundary replay')
    value = parse(raw)
    require(value['schema'] == 'ferric-p228-output-residual-boundary-observation-v1'
        and value['completed'] is True and value['source_postchecks_passed'] is True
        and value['prior_consumed_files_rehashed'] == 260 and len(value['consumed']) == 271
        and all(value[key] is False for key in FALSE), 'conditional CPU-only result scope')
    require(value['prior_comparison'] == prior_public['actual_comparison']
        and value['prior_comparison']['sha256'] == PRIOR_SHA, 'same actual prior comparison')
    prior = doc(value['prior_comparison']); prior_rows = {row['original']['path']: row for row in prior['consumed']}
    consumed = {}
    for pair in value['consumed']:
        require(type(pair) is dict and set(pair) == {'original', 'retained'}, 'read ledger pair')
        original, retained = pair['original'], pair['retained']
        require(original['path'] not in consumed and (original['bytes'], original['sha256'])
            == (retained['bytes'], retained['sha256']), 'unique and byte-preserving aliases')
        require(pair == prior_rows.get(original['path'], dict(original=original, retained=original)),
                'exact original transport map, no new aliases')
        require(checked(original)[1] == checked(retained)[1], 'both ledger identities rehashed')
        consumed[original['path']] = pair
    require(len(prior_rows) == 260 and set(prior_rows) <= set(consumed), 'all prior input pairs preserved')
    for source in (value['controller'], value['plan'], value['pure_tests'], value['prior_comparison'],
        value['native_outer'], value['native_summary'], value['native_capture'], value['framework_outer'],
        value['framework_capture'], *value['sources'].values()):
        require(consumed[source['path']]['original'] == source, 'selected consumed identity')
    plan = doc(value['plan'])
    require(plan == dict(schema='ferric-p228-output-residual-boundary-inputs-v1',
        comparison=value['prior_comparison'], boundary_tests=value['pure_tests'],
        output_label='output-residual-boundary-v228-v1'), 'actual replay plan')
    d = value['diagnostic']
    require(d['schema'] == 'ferric-p228-output-residual-boundary-diagnostic-v1' and d['authority'] == 'none'
        and d['elements'] == 4096 and d['ranks'] == 2 and d['conditional_replay_performed'] is True
        and d['oracle_sha256'] == SOURCES['residual_oracle.py'] and d['acceptance_threshold'] is None
        and all(d[key] is False for key in DIAGNOSTIC_FALSE), 'no numerical or causal authority')
    rows = d['comparisons']
    require([row['name'] for row in rows] == NAMES and [row['exact_words'] for row in rows] == COUNTS,
            'all eight actual formula comparison counts')
    for row, exact in zip(rows, COUNTS, strict=True):
        require(row['elements'] == 4096 and row['differing_words'] == 4096 - exact
            and row['byte_equal'] is (exact == 4096)
            and row['byte_equal'] is (row['expected_sha256'] == row['actual_sha256'])
            and row['first_differences_limit'] == 16 and len(row['first_differences']) == min(16, 4096 - exact),
            'actual equality and bounded difference summaries')
    native = doc(value['native_outer']); summary = doc(value['native_summary'])
    require(value['native_outer'] == prior['native_outer'] and value['framework_outer'] == prior['framework_outer']
        and native['retained_native']['summary.json'] == value['native_summary']
        and native['retained_native']['candidate-capture.bin'] == value['native_capture'], 'actual capture joins')
    capture = checked(value['native_capture'])[1]
    require(len(capture) == 9670656 and summary['stages'] == native['checked']['stages'], 'complete native manifest')
    slices = [row for row in summary['stages'] if row['stage'] in ('output-partial', 'first-residual')]
    require(value['native_slices'] == [dict(row, sha256=bytes(row['sha256']).hex()) for row in slices],
            'exact rank-ordered typed native slices')
    for row in value['native_slices']:
        part = capture[row['offset']:row['offset'] + row['bytes']]
        require(len(part) == row['bytes'] and sha(part) == row['sha256'], 'actual captured slice bytes')
        key = 'output_partials' if row['stage'] == 'output-partial' else 'native_first_residuals'
        require(d['conditioning'][key][row['rank']] == dict(bytes=len(part), sha256=sha(part)), 'conditional native input')
    framework = doc(value['framework_capture']); framework_outer = doc(value['framework_outer'])
    require(framework_outer['reference'] == value['framework_capture'], 'actual framework capture join')
    for name, key in (('embedding', 'framework_embedding'), ('o-projection', 'framework_o_projection'),
                      ('first-residual', 'framework_first_residual')):
        pins = [row['stages'][name]['pin'] for row in framework['passes']]
        require(len(pins) == 2 and pins == value['framework_inputs'][name], 'both genuine framework passes')
        a, b = [checked(source)[1] for source in pins]
        require(a == b and len(a) == 8192 and d['conditioning'][key] == dict(bytes=len(a), sha256=sha(a)),
                'actual conditional framework input bytes')
    pure = doc(value['pure_tests'])
    require(value['pure_tests']['sha256'] == PURE_SHA and value['pure_tests']['bytes'] == 1500
        and pure['schema'] == 'ferric-p228-output-residual-boundary-pure-v1' and pure['passed'] is True
        and pure['tests'] == 12 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True,
        'actual successful pure12 result')
    require(doc(pure['sources_before']) == doc(pure['sources_after']) == value['sources']
        and set(value['sources']) == set(SOURCES), 'unchanged tested source snapshot')
    copies = {'complete.json': (complete_path, pin(complete_path)),
        'inputs/plan.json': (checked(value['plan'])[0], value['plan'])}
    for name, source in value['sources'].items():
        require(source['sha256'] == SOURCES[name], 'exact boundary/test/oracle source')
        copies['source/' + name] = (checked(source)[0], source)
    names = sorted(node.name for node in ast.walk(ast.parse(checked(value['sources']['test_boundary.py'])[1]))
        if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    transcript = checked(pure['transcript'])[1].decode('utf-8')
    passed = re.findall(r'^(test_\w+) \([^\r\n]+\) \.\.\. ok$', transcript, re.MULTILINE)
    require(len(names) == len(set(names)) == 12 and sorted(passed) == names
        and re.search(r'^Ran 12 tests in [^\n]+\n\nOK\s*$', transcript, re.MULTILINE), 'all twelve named test passes')
    for name, source in (('complete.json', value['pure_tests']), ('sources-before.json', pure['sources_before']),
                        ('sources-after.json', pure['sources_after']), ('tests.log', pure['transcript'])):
        copies['pure/' + name] = (checked(source)[0], source)
    require(value['controller']['sha256'] == RUN_SHA and pure['controller']['sha256'] == WRAPPER_SHA,
            'actual replay runner and pure wrapper')
    copies['runner/run.py'] = (checked(value['controller'])[0], value['controller'])
    copies['tools/run_output_residual_boundary_pure_p228_v1.py'] = (checked(pure['controller'])[0], pure['controller'])
    prepare = L / 'proposals/p228-output-residual-boundary-run-v1/prepare.py'
    require(sha(body(prepare)) == PREPARE_SHA, 'actual assembly helper')
    copies['runner/prepare.py'] = (prepare, dict(path=str(prepare), **CHECKED[prepare]))
    assembly = L / 'output-residual-boundary-inputs-v228-v1/assembly.json'; raw = body(assembly)
    require(sha(raw) == ASSEMBLY_SHA, 'actual root assembly record')
    for source in parse(raw)['local_input_pins']: checked(source)
    copies['inputs/assembly.json'] = (assembly, dict(path=str(assembly), **CHECKED[assembly]))
    own = Path(__file__).resolve(strict=True); body(own)
    copies['tools/publish.py'] = (own, dict(path=str(own), **CHECKED[own]))
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical qualification parent')
    readme = None
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} == {'README.md'},
                'only root-authored README may preexist')
        raw = body(Q / 'README.md'); readme = dict(bytes=len(raw), sha256=sha(raw))
    for name, (source, _) in copies.items():
        require(not os.path.lexists(Q / name) and source.suffix not in ('.bin', '.bf16', '.safetensors'),
                'fresh text-only publication target')
        body(source).decode('utf-8')
    for source in list(CHECKED): body(source)
    Q.mkdir(mode=0o755, exist_ok=True); retained = {}
    for name, (source, original) in sorted(copies.items()):
        destination = Q / name; destination.parent.mkdir(parents=True, exist_ok=True)
        require(destination.parent.resolve(strict=True) == destination.parent, 'canonical output parent')
        raw = body(source)
        with destination.open('xb') as stream: stream.write(raw)
        require(body(destination) == raw, 'exact published bytes')
        retained[name] = dict(bytes=len(raw), sha256=sha(raw), source=original)
    for source in list(CHECKED): body(source)
    require(body(prior_public_path) == prior_public_raw, 'prior comparison ledger remains unchanged')
    result = dict(schema='ferric-p228-output-residual-boundary-publication-v1', completed=True, authority='none',
        actual_replay=pin(complete_path), prior_comparison=value['prior_comparison'],
        prior_publication=dict(path='../layer0-native-capture-v1/comparison/result.json',
            bytes=len(prior_public_raw), sha256=PRIOR_PUBLIC_SHA), original_transport_pairs_rehashed=271,
        pure_tests=12, conditional_replay_performed=True, comparisons=rows,
        retained=retained, root_authored_readme=readme, original_source_readmes_omitted=True,
        raw_tensor_bodies_published=False, arithmetic_reexecuted_by_publisher=False,
        **{key: False for key in set(FALSE) | set(DIAGNOSTIC_FALSE)})
    raw = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    with (Q / 'result.json').open('xb') as stream: stream.write(raw)
    print(json.dumps(dict(result=str(Q / 'result.json'), bytes=len(raw), sha256=sha(raw), files=len(retained))), flush=True)


if __name__ == '__main__':
    main()
