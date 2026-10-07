"""Retain the original CPU-only comparison without importing or recomputing it."""
import ast
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-readiness40-comparison-v228-v2'
SOURCE = dict(bytes=3046, sha256='32c213315c2eb36c3851489cd23fb874e1ef7e80ab67913f51dd4fb807e03a8e')
RUNNER = dict(bytes=14920, sha256='c66e8d3d71464db71b361190195421bfd920ba96360a538f1123b5d46bb30009')
ARCHIVES = {
    'guarded-mlp-readiness40-gpu-evidence-v228-v1.tar.gz': dict(bytes=2210645,
        sha256='df5c824f18d26b3c1a6cde4e7eb6827f1472a846cbb5b97a0b693001270d3c3c'),
    'guarded-mlp-readiness40-checker-evidence-v228-v1.tar.gz': dict(bytes=27743,
        sha256='95cebdbd242abc0d5d8256cd7f7fa11ddc45f5aa18d36e4652baae7b549a8660'),
    'guarded-mlp-readiness40-reference-evidence-v228-v2.tar.gz': dict(bytes=3965567,
        sha256='cd9103d1e73c1af63960f24e9d501f5a6747ff82a505891f38d08657693cf5bc'),
}
OUTPUTS = {'complete.json', 'comparison-tests.stderr'}
SELECTED = [0, 15, 16, 39]
CAP = 16 << 20


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(rows):
        value = {}
        for key, row in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = row
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= CAP,
                'bounded single-link ordinary body')
        raw = stream.read(CAP + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'input drift')
    return raw


def literals(raw):
    result = {}
    for node in ast.parse(raw).body:
        if not isinstance(node, ast.Assign):
            continue
        for target in node.targets:
            if not isinstance(target, ast.Name):
                continue
            try:
                result[target.id] = ast.literal_eval(node.value)
            except (ValueError, TypeError):
                if isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Name) \
                        and node.value.func.id == 'dict' and not node.value.args:
                    result[target.id] = {k.arg: ast.literal_eval(k.value) for k in node.value.keywords}
    return result


def verify(bodies, terminal_sha):
    require(pin(bodies['output/complete.json'])['sha256'] == terminal_sha, 'actual comparison terminal SHA')
    result = parse(bodies['output/complete.json'])
    require(result['schema'] == 'ferric-readiness40-comparison-data-v1' and result['passed'] is True
            and result['error'] is None and result['postcheck_errors'] == []
            and result['acceptance_threshold'] is None and 0 <= result['elapsed_seconds'] < 180
            and all(result[key] is False for key in ('gpu_execution', 'model_execution', 'native_rerun',
                'numerical_acceptance', 'full_model_acceptance', 'full_long_workload',
                'performance_claim', 'production_authority')), 'successful data comparison, not model acceptance')
    require(pin(bodies['source/source-manifest.json']) == SOURCE
            and pin(bodies['source/run_bound.py']) == RUNNER, 'reviewed source and actual two-literal bound runner')
    manifest = parse(bodies['source/source-manifest.json'])
    source = dict(manifest['files'], **{'source-manifest.json': SOURCE, 'run_bound.py': RUNNER})
    require(len(source) == 11 and all(Path(name).name == name and pin(bodies['source/' + name]) == row
            for name, row in source.items()), 'eleven exact source bodies')
    require(set(bodies) == {'source/' + name for name in source} | {'output/' + name for name in OUTPUTS},
            'exact thirteen original source/output bodies')
    bound = literals(bodies['source/run_bound.py'])
    original = bodies['source/run.py'].decode()
    restored = bodies['source/run_bound.py'].decode()
    for key in ('REFERENCE_TERMINAL', 'REFERENCE_ARCHIVE'):
        restored, count = re.subn(r'^' + key + r' = .+$', key + ' = None', restored, count=1, flags=re.M)
        require(count == 1, 'one known bound literal')
    require(restored == original, 'sole actual terminal/archive binding reversal')
    module_rows = dict(bound['MODULE_PINS'], **{'reference_evidence.py': bound['REFERENCE_EVIDENCE_PIN'],
                                              'run_bound.py': RUNNER})
    require(len(module_rows) == 8 and all(source[name] == row for name, row in module_rows.items()),
            'eight exact executed module/controller inputs')
    expected_readset = {str(ROOT / name): row for name, row in module_rows.items()}
    expected_readset.update({str(E / name): row for name, row in ARCHIVES.items()})
    rows = result['inputs']
    require(len(rows) == len({row['path'] for row in rows}) == 11
            and {row['path']: compact(row) for row in rows} == expected_readset, 'original eleven-input posthash ledger')
    checks = result['checks']
    require(set(checks) == {'native', 'checker', 'reference', 'comparison_tests', 'diagnostics'}
            and checks['native']['original_passed'] is True
            and checks['native']['retained_success_revalidated'] is True
            and compact(checks['native']['original_terminal']) == bound['NATIVE_TERMINAL']
            and checks['checker']['terminal'] == bound['CHECKER_TERMINAL']
            and checks['checker']['tests'] == 16 and checks['checker']['original_bytes_rechecked'] is True
            and checks['reference']['passed'] is True and checks['reference']['repeat_gate_rechecked'] is True
            and checks['reference']['owner_terminal'] == bound['REFERENCE_TERMINAL']
            and checks['reference']['full_model_forward_calls'] == 80,
            'original independent native/checker/reference admission results')
    tests = checks['comparison_tests']
    require(tests == sorted(manifest['tests']) and len(tests) == len(set(tests)) == 8, 'eight exact comparison tests')
    text = bodies['output/comparison-tests.stderr'].decode()
    names = re.findall(r'^(test_[a-z0-9_]+) \(test_compare\.ComparisonTests(?:\.\1)?\) \.\.\. ok$', text, re.M)
    require(len(names) == len(set(names)) == 8
            and sorted('test_compare.ComparisonTests.' + name for name in names) == tests
            and re.search(r'\nRan 8 tests in [0-9.]+s\n\nOK\s*$', text), 'original raw eight-test census')
    diagnostic = checks['diagnostics']
    selected = diagnostic['selected']
    require(diagnostic['schema'] == 'ferric-readiness40-actual-reference-diagnostic-v1'
            and diagnostic['selected_positions'] == SELECTED and diagnostic['tensor_rows'] == 152
            and diagnostic['argmax_positions'] == 40 and diagnostic['candidate_generated_tokens'] == 0
            and diagnostic['reference_generated_tokens'] == 0 and diagnostic['own_kv_caches'] is True
            and diagnostic['full_prompt_sha256'] == '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02'
            and diagnostic['model_id'] == 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
            and diagnostic['bundle_id'] == '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
            and diagnostic['acceptance_threshold'] is selected['acceptance_threshold'] is None
            and all(diagnostic[key] is False for key in ('numerical_acceptance', 'full_model_acceptance',
                'full_long_workload', 'performance_claim', 'production_authority', 'gpu_execution', 'model_execution'))
            and selected['schema'] == 'ferric-readiness40-selected-tensor-diagnostic-v1'
            and selected['tensor_rows'] == 152 and selected['receipt_authentication'] is False
            and all(selected[key] is False for key in ('numerical_acceptance', 'full_model_acceptance',
                                                     'full_long_workload', 'performance_claim')), 'diagnostic-only identity and scope')
    argmax = diagnostic['argmax_diagnostics']
    require(len(argmax) == 40 and [row['position'] for row in argmax] == list(range(40)), 'complete40 argmax observations')
    for position, row in enumerate(argmax):
        is_selected = position in SELECTED
        require(all(type(row[k]) is int and 0 <= row[k] < 151936
                    for k in ('input_token', 'candidate_output_token', 'reference_output_token'))
                and row['output_token_equal'] is (row['candidate_output_token'] == row['reference_output_token'])
                and row['selected_capture'] is row['candidate_argmax_recomputed'] is row['reference_argmax_recomputed'] is is_selected
                and row['unselected_argmax_scope'] == (None if is_selected else
                    'authenticated original records; logits body not retained'), 'truthful selected versus declared argmax scope')
    keys = ('position', 'input_token', 'candidate_output_token', 'reference_output_token',
            'selected_capture', 'candidate_argmax_recomputed', 'reference_argmax_recomputed')
    require(diagnostic['argmax_matches'] == sum(row['output_token_equal'] for row in argmax)
            and diagnostic['argmax_mismatches'] == [{k: row[k] for k in keys} for row in argmax if not row['output_token_equal']],
            'all mismatches preserved, never an equality acceptance gate')
    comparisons = selected['comparisons']
    tensor_roles = {'layer%d-hidden' % i for i in range(36)} | {'final-norm', 'logits'}
    require(len(comparisons) == 4 and [row['position'] for row in comparisons] == SELECTED,
            'four selected positions only')
    exact_rows = 0
    for row in comparisons:
        point = argmax[row['position']]
        require(all(row[k] == point[k] for k in ('input_token', 'candidate_output_token',
                                                'reference_output_token', 'output_token_equal'))
                and set(row['tensors']) == tensor_roles, 'selected identities and38 tensor roles')
        for name, metric in row['tensors'].items():
            require(set(metric) == {'elements', 'exact_words', 'max_abs_error', 'max_bf16_steps',
                                   'relative_l2', 'rmse', 'zero_reference_norm'}
                    and metric['elements'] == (151936 if name == 'logits' else 4096)
                    and type(metric['exact_words']) is int and 0 <= metric['exact_words'] <= metric['elements']
                    and type(metric['max_bf16_steps']) is int and 0 <= metric['max_bf16_steps'] <= 65535
                    and type(metric['zero_reference_norm']) is bool
                    and all(type(metric[k]) in (int, float) and math.isfinite(metric[k]) and metric[k] >= 0
                            for k in ('max_abs_error', 'rmse'))
                    and (metric['relative_l2'] is None or (type(metric['relative_l2']) in (int, float)
                         and math.isfinite(metric['relative_l2']) and metric['relative_l2'] >= 0)), 'recorded metric scalar bounds only')
            exact_rows += metric['exact_words'] == metric['elements']
    return dict(terminal=pin(bodies['output/complete.json']), source=SOURCE, runner=RUNNER,
        original_files=13, source_files=11, output_files=2, authenticated_readset=expected_readset,
        comparison_tests=8, tensor_rows=152, exact_tensor_rows=exact_rows,
        argmax_positions=40, argmax_matches=diagnostic['argmax_matches'],
        metrics_recomputed=False, modules_imported=False, numerical_acceptance=False,
        performance_claim=False, gpu_execution=False, original_capsule_bodies_retained=False)


def export(terminal_sha, archive):
    require(archive.is_absolute() and archive.parent.resolve(strict=True) == archive.parent
            and not archive.is_relative_to(ROOT) and not os.path.lexists(archive), 'fresh external archive')
    manifest_raw = read(ROOT / 'source-manifest.json')
    require(pin(manifest_raw) == SOURCE, 'frozen source roster')
    source_names = set(parse(manifest_raw)['files']) | {'source-manifest.json', 'run_bound.py'}
    require({p.name for p in ROOT.iterdir()} == source_names | {'output'}
            and {p.name for p in (ROOT / 'output').iterdir()} == OUTPUTS, 'closed remote source/output root')
    paths = {'source/' + name: ROOT / name for name in source_names}
    paths.update({'output/' + name: ROOT / 'output' / name for name in OUTPUTS})
    bodies = {name: read(path) for name, path in paths.items()}
    checked = verify(bodies, terminal_sha)
    readset = {str(path): pin(bodies[name]) for name, path in paths.items()}
    for path, row in checked['authenticated_readset'].items():
        require(pin(read(Path(path))) == row, 'actual11 input rehash before export')
        readset[path] = row
    tool = read(Path(__file__).resolve())
    bodies['retention-tool.py'] = tool
    manifest = encoded(dict(schema='ferric-readiness40-comparison-retention-v1', root=str(ROOT),
        verification=checked, files={name: pin(body) for name, body in sorted(bodies.items())},
        full_recorded_readset_rehashed=True, runtime_execution=False, gpu_execution=False))
    bodies['manifest.json'] = manifest
    require(len(bodies) == 15 and sum(map(len, bodies.values())) <= CAP, 'closed15 bounded retention bodies')
    require(all(pin(read(Path(path))) == row for path, row in readset.items())
            and read(Path(__file__).resolve()) == tool, 'source/output/all11 inputs posthash')
    partial = archive.with_name(archive.name + '.partial')
    require(not os.path.lexists(partial), 'fresh partial archive')
    with partial.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, body in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o600, 0
                tar.addfile(info, io.BytesIO(body))
        stream.flush(); os.fsync(stream.fileno())
    require(all(pin(read(Path(path))) == row for path, row in readset.items()), 'readset stable through export')
    raw = read(partial)
    os.link(partial, archive); partial.unlink()
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(raw)), members=15,
                         expanded_bytes=sum(map(len, bodies.values())), verification=checked), sort_keys=True))


def retain(archive, archive_sha, destination):
    require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
            and not os.path.lexists(destination), 'fresh local destination')
    raw = read(archive)
    require(pin(raw)['sha256'] == archive_sha, 'actual archive SHA')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 15
                and all(m.isfile() and not m.pax_headers and not Path(m.name).is_absolute()
                        and Path(m.name).as_posix() == m.name and '..' not in Path(m.name).parts
                        and m.name not in ('', '.') and 0 <= m.size <= CAP for m in members)
                and sum(m.size for m in members) <= CAP, 'closed regular USTAR capsule')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'exact member extents')
    manifest = parse(bodies['manifest.json'])
    require(manifest['schema'] == 'ferric-readiness40-comparison-retention-v1' and manifest['root'] == str(ROOT)
            and manifest['files'] == {name: pin(body) for name, body in bodies.items() if name != 'manifest.json'}
            and manifest['full_recorded_readset_rehashed'] is True
            and manifest['runtime_execution'] is manifest['gpu_execution'] is False
            and bodies['retention-tool.py'] == read(Path(__file__).resolve()), 'exact archived pins and reviewed retainer')
    originals = {name: body for name, body in bodies.items() if name not in ('manifest.json', 'retention-tool.py')}
    checked = verify(originals, manifest['verification']['terminal']['sha256'])
    require(checked == manifest['verification'] and read(archive) == raw, 'retained verification and archive posthash')
    destination.mkdir(mode=0o700)
    for name, body in sorted(bodies.items()):
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with path.open('xb') as stream:
            stream.write(body)
        require(read(path) == body, 'original byte preservation')
    print(json.dumps(dict(destination=str(destination), archive=pin(raw), members=15,
                         verification=checked, local_external_inputs_rehashed=False), sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B evidence.py export TERMINAL_SHA ARCHIVE | retain ARCHIVE SHA DEST')
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_CORE, 0), (resource.RLIMIT_FSIZE, CAP)):
        old = resource.getrlimit(kind)
        value = min([cap] + [n for n in old if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    def interrupted(number, _frame):
        raise RuntimeError('retention signal ' + str(number))
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 60)
    try:
        if len(sys.argv) == 4 and sys.argv[1] == 'export':
            require(re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'actual terminal SHA')
            export(sys.argv[2], Path(sys.argv[3]))
        else:
            require(len(sys.argv) == 5 and sys.argv[1] == 'retain'
                    and re.fullmatch('[0-9a-f]{64}', sys.argv[3]), 'actual archive SHA')
            retain(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
