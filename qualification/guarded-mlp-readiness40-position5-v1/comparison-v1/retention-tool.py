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
ROOT = E / 'guarded-mlp-readiness40-position5-comparison-v228-v1'
SOURCE = dict(bytes=1932, sha256='d0872e23a2ef39dc0a857adda0166c01cc5b61f980bdd6363d5d04d5d821238d')
RUNNER = dict(bytes=20762, sha256='ffd32c787344001250aa9fe2e88452ad83cf1a4c633eca648c240bfc639b1884')
NEW_NATIVE_VERIFIER = dict(bytes=25681, sha256='432fb0d7716757209b4cd9d7a15c3851c0c6579d916f9f9e0ca9d6a3abd215b3')
ARCHIVE_ROLES = {
    'guarded-mlp-readiness40-gpu-evidence-v228-v1.tar.gz': 'NATIVE_ARCHIVE',
    'guarded-mlp-readiness40-checker-evidence-v228-v1.tar.gz': 'CHECKER_ARCHIVE',
    'guarded-mlp-readiness40-reference-evidence-v228-v2.tar.gz': 'REFERENCE_ARCHIVE',
    'guarded-mlp-readiness40-position5-gpu-evidence-v228-v1.tar.gz': 'NEW_NATIVE_ARCHIVE',
    'guarded-mlp-readiness40-position5-checker-evidence-v228-v1.tar.gz': 'NEW_CHECKER_ARCHIVE',
    'guarded-mlp-readiness40-position5-reference-evidence-v228-v1.tar.gz': 'NEW_REFERENCE_ARCHIVE',
    'guarded-mlp-readiness40-position5-diagnostic-evidence-v228-v1.tar.gz': 'DIAGNOSTIC_ARCHIVE',
}
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


def literal(node):
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == 'dict':
        require(not node.args and all(k.arg is not None for k in node.keywords), 'plain data dict literal')
        return {k.arg: literal(k.value) for k in node.keywords}
    if isinstance(node, ast.Dict):
        return {literal(k): literal(v) for k, v in zip(node.keys, node.values)}
    return ast.literal_eval(node)


def literals(raw):
    result = {}
    for node in ast.parse(raw).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            try: result[node.targets[0].id] = literal(node.value)
            except (ValueError, TypeError): pass
    return result


def compact_pin(value):
    require(type(value) is dict and set(value) == {'bytes', 'sha256'}
            and type(value['bytes']) is int and 0 <= value['bytes'] <= 128 << 20
            and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']),
            'observed ordinary pin')
    return value


def bound_sources(bodies, manifest):
    require(RUNNER is not None and NEW_NATIVE_VERIFIER is not None, 'observed bound source pins pending')
    source = dict(manifest['files'], **{'source-manifest.json': SOURCE,
        'run_bound.py': RUNNER, 'new_native_evidence_bound.py': NEW_NATIVE_VERIFIER})
    require(len(source) == 13 and all(Path(name).name == name and
        pin(bodies['source/' + name]) == row for name, row in source.items()), 'thirteen exact source bodies')
    bound_raw, original_raw = bodies['source/run_bound.py'], bodies['source/run.py']
    bound, original = literals(bound_raw), literals(original_raw)
    restored = bound_raw.decode()
    for key in ('NEW_NATIVE_ARCHIVE', 'NEW_NATIVE_TERMINAL', 'NEW_CHECKER_ARCHIVE', 'DIAGNOSTIC_ARCHIVE'):
        compact_pin(bound[key])
        restored, count = re.subn(r'^' + key + r' = .+$', key + ' = None', restored, count=1, flags=re.M)
        require(count == 1, 'one observed runner binding')
    restored, count = re.subn(r"^    'new_native_evidence.py': .+,$",
        "    'new_native_evidence.py': None,", restored, count=1, flags=re.M)
    require(count == 1 and restored == original_raw.decode(), 'exact five-binding runner reversal')
    require(bound['MODULE_PINS']['new_native_evidence.py'] == NEW_NATIVE_VERIFIER,
            'executed native verifier exact actual source')
    template_raw = bodies['source/new_native_evidence.py']
    actual_raw = bodies['source/new_native_evidence_bound.py']
    template, actual = literals(template_raw), literals(actual_raw)
    for field in ('ROOT_PINS', 'ADMISSION'):
        require(set(template[field]) == set(actual[field]), 'same native verifier binding fields')
        for name, value in template[field].items():
            if value is None: compact_pin(actual[field][name])
            else: require(value == actual[field][name], 'unchanged native verifier fixed input')
    require(template['CHECKER'] is None and set(actual['CHECKER']) == {'path', 'bytes', 'sha256'},
            'actual checker path and pin')
    require(compact(actual['CHECKER']) == bound['NEW_CHECKER_TERMINAL'], 'same actual checker terminal')
    left, right = ast.parse(template_raw), ast.parse(actual_raw)
    fields = {'ROOT_PINS', 'ADMISSION', 'CHECKER'}
    replacements = {n.targets[0].id: n.value for n in left.body if isinstance(n, ast.Assign)
                    and len(n.targets) == 1 and isinstance(n.targets[0], ast.Name) and n.targets[0].id in fields}
    seen = set()
    for node in right.body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in fields:
                require(name not in seen, 'single native binding'); seen.add(name); node.value = replacements[name]
    require(seen == fields and ast.dump(left, include_attributes=False) == ast.dump(right, include_attributes=False),
            'native verifier only actual binding AST changes')
    return source, bound


def verify(bodies, terminal_name, terminal_sha):
    require(terminal_name in ('complete.json', 'failed.json'), 'closed original terminal name')
    require(pin(bodies['output/' + terminal_name])['sha256'] == terminal_sha, 'actual terminal SHA')
    result = parse(bodies['output/' + terminal_name])
    require(result['schema'] == 'ferric-readiness40-position5-comparison-data-v1'
            and type(result['passed']) is bool and terminal_name == ('complete.json' if result['passed'] else 'failed.json')
            and type(result['postcheck_errors']) is list and result['acceptance_threshold'] is None
            and type(result['elapsed_seconds']) in (int, float) and math.isfinite(result['elapsed_seconds'])
            and 0 <= result['elapsed_seconds'] < 180 and all(result[key] is False for key in
                ('gpu_execution', 'model_execution', 'native_rerun', 'numerical_acceptance',
                 'full_model_acceptance', 'full_long_workload', 'performance_claim', 'production_authority')),
            'original bounded diagnostic scope')
    require(pin(bodies['source/source-manifest.json']) == SOURCE, 'frozen source manifest')
    source, bound = bound_sources(bodies, parse(bodies['source/source-manifest.json']))
    require(set(bodies) == {'source/' + name for name in source} | {'output/' + terminal_name},
            'fourteen original source/output bodies')
    module_rows = {('new_native_evidence_bound.py' if name == 'new_native_evidence.py' else name): row
                   for name, row in bound['MODULE_PINS'].items()}
    module_rows['run_bound.py'] = RUNNER
    require(len(module_rows) == 10 and all(source[name] == row for name, row in module_rows.items()),
            'ten exact executed source bodies')
    expected = {str(ROOT / name): row for name, row in module_rows.items()}
    expected.update({str(E / name): compact_pin(bound[key]) for name, key in ARCHIVE_ROLES.items()})
    rows = result['inputs']
    observed = {row['path']: compact(row) for row in rows}
    require(len(rows) == len(observed) and set(observed) <= set(expected)
            and all(row == expected[name] for name, row in observed.items()), 'original input readset pins')
    if not result['passed']:
        require(result['error'] is not None or result['postcheck_errors'], 'truthful failed diagnostic prefix')
        return dict(terminal=pin(bodies['output/' + terminal_name]), terminal_name=terminal_name,
            original_passed=False, original_files=14, source_files=13, output_files=1,
            authenticated_readset=observed, success_checks_revalidated=False,
            modules_imported=False, metrics_recomputed=False, numerical_acceptance=False,
            performance_claim=False, gpu_execution=False)
    require(result['error'] is None and result['postcheck_errors'] == [] and observed == expected,
            'successful complete seventeen-input posthash ledger')
    c = result['checks']
    require(set(c) == {'original_native', 'diagnostic_native', 'original_checker',
        'diagnostic_checker', 'diagnostic_function_gate', 'original_reference', 'diagnostic_reference',
        'full_original_receipts_authenticated', 'both_native_structural_admissions_rechecked',
        'full_prompt_and_model_bundle_joined', 'diagnostic'} and all(c[k] is True for k in
        ('full_original_receipts_authenticated', 'both_native_structural_admissions_rechecked',
         'full_prompt_and_model_bundle_joined')), 'complete upstream authentication')
    for key, terminal in (('original_native', 'NATIVE_TERMINAL'), ('diagnostic_native', 'NEW_NATIVE_TERMINAL')):
        require(c[key]['original_passed'] is True and c[key]['retained_success_revalidated'] is True
                and compact(c[key]['original_terminal']) == bound[terminal], 'both original native admissions')
    for key, terminal in (('original_reference', 'REFERENCE_TERMINAL'), ('diagnostic_reference', 'NEW_REFERENCE_TERMINAL')):
        require(c[key]['passed'] is True and c[key]['repeat_gate_rechecked'] is True
                and c[key]['owner_terminal'] == bound[terminal] and c[key]['full_model_forward_calls'] == 80,
                'both independently repeated reference admissions')
    require(c['original_checker'] == dict(terminal=bound['CHECKER_TERMINAL'], tests=16,
                                         original_bytes_rechecked=True), 'original sixteen-test gate')
    for key, terminal, count in (('diagnostic_checker', 'NEW_CHECKER_TERMINAL', 17),
                                ('diagnostic_function_gate', 'DIAGNOSTIC_TERMINAL', 7)):
        gate = c[key]
        require(gate['terminal'] == bound[terminal] and gate['original_bytes_rechecked'] is True
                and gate['tests']['passed'] == count
                and len(gate['tests']['names']) == len(set(gate['tests']['names'])) == count
                and all(gate['tests'][k] == 0 for k in ('failed', 'errors', 'skipped')), 'actual separate pure gate')
    d = c['diagnostic']
    require(d['schema'] == 'ferric-readiness40-position5-conditional-diagnostic-v1'
            and d['position'] == 5 and type(d['input_token']) is int and 0 <= d['input_token'] < 151936
            and d['all40_native_pins_unchanged'] is True and d['all40_reference_pins_unchanged'] is True
            and d['common_payloads_equal'] == [0, 16, 39] and d['acceptance_threshold'] is None
            and all(d[k] is False for k in ('receipt_authentication', 'native_structural_admission',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim'))
            and d['prompt'] == dict(bytes=8192, sha256='2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02'),
            'conditional position-five parity and authority boundary')
    roles = {'layer%d-hidden' % i for i in range(36)} | {'final-norm', 'logits'}
    require(set(d['tensors']) == roles, 'thirty-eight exact tensor roles')
    for name, metric in d['tensors'].items():
        require(set(metric) == {'elements', 'exact_words', 'max_abs_error', 'max_bf16_steps',
                               'relative_l2', 'rmse', 'zero_reference_norm'}
            and metric['elements'] == (151936 if name == 'logits' else 4096)
            and type(metric['exact_words']) is int and 0 <= metric['exact_words'] <= metric['elements']
            and type(metric['max_bf16_steps']) is int and 0 <= metric['max_bf16_steps'] <= 65535
            and type(metric['zero_reference_norm']) is bool
            and all(type(metric[k]) in (int, float) and math.isfinite(metric[k]) and metric[k] >= 0
                    for k in ('max_abs_error', 'rmse'))
            and (metric['relative_l2'] is None or (type(metric['relative_l2']) in (int, float)
                 and math.isfinite(metric['relative_l2']) and metric['relative_l2'] >= 0)), 'recorded metric bounds')
    require(d['first_nonidentical_layer_hidden'] is None or
            (type(d['first_nonidentical_layer_hidden']) is int and 0 <= d['first_nonidentical_layer_hidden'] < 36),
            'bounded diagnostic first difference')
    for side in ('native', 'reference'):
        item = d[side]
        require(set(item) == {'top_two', 'top_two_margin_units_2_pow_minus133', 'tie_break', 'targets',
                             'token2_minus_token9112_units_2_pow_minus133'}
                and len(item['top_two']) == len(item['targets']) == 2
                and [r['token'] for r in item['targets']] == [2, 9112]
                and [r['rank'] for r in item['top_two']] == [1, 2], 'exact rank diagnostic roles')
        for row in item['top_two'] + item['targets']:
            require(set(row) == {'token', 'rank', 'bf16_word', 'value', 'signed_units_2_pow_minus133'}
                and type(row['token']) is int and 0 <= row['token'] < 151936
                and type(row['rank']) is int and 1 <= row['rank'] <= 151936
                and type(row['bf16_word']) is int and 0 <= row['bf16_word'] <= 65535
                and row['bf16_word'] & 0x7f80 != 0x7f80
                and type(row['value']) in (int, float) and math.isfinite(row['value'])
                and type(row['signed_units_2_pow_minus133']) is int, 'finite recorded logit diagnostic')
        a, b = item['top_two']
        require(a['token'] != b['token'] and
                (a['signed_units_2_pow_minus133'] > b['signed_units_2_pow_minus133'] or
                 (a['signed_units_2_pow_minus133'] == b['signed_units_2_pow_minus133'] and a['token'] < b['token']))
                and type(item['top_two_margin_units_2_pow_minus133']) is int
                and item['top_two_margin_units_2_pow_minus133'] ==
                    a['signed_units_2_pow_minus133'] - b['signed_units_2_pow_minus133']
                and type(item['token2_minus_token9112_units_2_pow_minus133']) is int
                and item['token2_minus_token9112_units_2_pow_minus133'] ==
                    item['targets'][0]['signed_units_2_pow_minus133'] - item['targets'][1]['signed_units_2_pow_minus133'],
                'recorded top-two and target margin consistency')
    return dict(terminal=pin(bodies['output/complete.json']), terminal_name=terminal_name,
        original_passed=True, original_files=14, source_files=13, output_files=1,
        authenticated_readset=observed, success_checks_revalidated=True, tensor_rows=38,
        own_history_records=40, common_capture_positions=[0, 16, 39],
        native_top_two=d['native']['top_two'], reference_top_two=d['reference']['top_two'],
        metrics_recomputed=False, modules_imported=False, numerical_acceptance=False,
        performance_claim=False, gpu_execution=False, original_capsule_bodies_retained=False)


def export(terminal_name, terminal_sha, archive):
    outputs = {terminal_name}
    require(archive.is_absolute() and archive.parent.resolve(strict=True) == archive.parent
            and not archive.is_relative_to(ROOT) and not os.path.lexists(archive), 'fresh external archive')
    manifest_raw = read(ROOT / 'source-manifest.json')
    require(pin(manifest_raw) == SOURCE, 'frozen source roster')
    source_names = set(parse(manifest_raw)['files']) | {'source-manifest.json', 'run_bound.py', 'new_native_evidence_bound.py'}
    require({p.name for p in ROOT.iterdir()} == source_names | {'output'}
            and {p.name for p in (ROOT / 'output').iterdir()} == outputs, 'closed remote source/output root')
    paths = {'source/' + name: ROOT / name for name in source_names}
    paths.update({'output/' + name: ROOT / 'output' / name for name in outputs})
    bodies = {name: read(path) for name, path in paths.items()}
    checked = verify(bodies, terminal_name, terminal_sha)
    readset = {str(path): pin(bodies[name]) for name, path in paths.items()}
    for path, row in checked['authenticated_readset'].items():
        require(pin(read(Path(path))) == row, 'actual recorded input rehash before export')
        readset[path] = row
    tool = read(Path(__file__).resolve())
    bodies['retention-tool.py'] = tool
    manifest = encoded(dict(schema='ferric-readiness40-position5-comparison-retention-v1', root=str(ROOT),
        verification=checked, files={name: pin(body) for name, body in sorted(bodies.items())},
        full_recorded_readset_rehashed=True, runtime_execution=False, gpu_execution=False))
    bodies['manifest.json'] = manifest
    require(len(bodies) == 16 and sum(map(len, bodies.values())) <= CAP, 'closed16 bounded retention bodies')
    require(all(pin(read(Path(path))) == row for path, row in readset.items())
            and read(Path(__file__).resolve()) == tool, 'source/output/all recorded inputs posthash')
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
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(raw)), members=16,
                         expanded_bytes=sum(map(len, bodies.values())), verification=checked), sort_keys=True))


def retain(archive, archive_sha, destination):
    require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
            and not os.path.lexists(destination), 'fresh local destination')
    raw = read(archive)
    require(pin(raw)['sha256'] == archive_sha, 'actual archive SHA')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 16
                and all(m.isfile() and not m.pax_headers and not Path(m.name).is_absolute()
                        and Path(m.name).as_posix() == m.name and '..' not in Path(m.name).parts
                        and m.name not in ('', '.') and 0 <= m.size <= CAP for m in members)
                and sum(m.size for m in members) <= CAP, 'closed regular USTAR capsule')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'exact member extents')
    manifest = parse(bodies['manifest.json'])
    require(manifest['schema'] == 'ferric-readiness40-position5-comparison-retention-v1' and manifest['root'] == str(ROOT)
            and manifest['files'] == {name: pin(body) for name, body in bodies.items() if name != 'manifest.json'}
            and manifest['full_recorded_readset_rehashed'] is True
            and manifest['runtime_execution'] is manifest['gpu_execution'] is False
            and bodies['retention-tool.py'] == read(Path(__file__).resolve()), 'exact archived pins and reviewed retainer')
    originals = {name: body for name, body in bodies.items() if name not in ('manifest.json', 'retention-tool.py')}
    checked = verify(originals, manifest['verification']['terminal_name'], manifest['verification']['terminal']['sha256'])
    require(checked == manifest['verification'] and read(archive) == raw, 'retained verification and archive posthash')
    destination.mkdir(mode=0o700)
    for name, body in sorted(bodies.items()):
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with path.open('xb') as stream:
            stream.write(body)
        require(read(path) == body, 'original byte preservation')
    print(json.dumps(dict(destination=str(destination), archive=pin(raw), members=16,
                         verification=checked, local_external_inputs_rehashed=False), sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B evidence.py export NAME TERMINAL_SHA ARCHIVE | retain ARCHIVE SHA DEST')
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
        if len(sys.argv) == 5 and sys.argv[1] == 'export':
            require(sys.argv[2] in ('complete.json', 'failed.json') and re.fullmatch('[0-9a-f]{64}', sys.argv[3]), 'actual terminal name/SHA')
            export(sys.argv[2], sys.argv[3], Path(sys.argv[4]))
        else:
            require(len(sys.argv) == 5 and sys.argv[1] == 'retain'
                    and re.fullmatch('[0-9a-f]{64}', sys.argv[3]), 'actual archive SHA')
            retain(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
