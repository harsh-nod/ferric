"""Retain original position-five R2 bodies only; never evaluate replay or model code."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import struct
import sys
import tarfile
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-causal-position5-r2-replay-v228-v1'
INPUT_ROOT = E / 'guarded-mlp-causal-qkv-exact-v228-v1/inputs'
DEST = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-readiness40-causal-layer0-v1/position5-r2-replay-v1')
ARCHIVE = E / 'guarded-mlp-causal-position5-r2-replay-evidence-v228-v1.tar.gz'
SOURCE_MANIFEST = dict(bytes=10034, sha256='2ef18416c88ddec78c3ffe05bb1017bc546987f0283813d8ddac378b363eafea')
SOURCES = {'head.py', 'qkv.py', 'exact_bf16.py', 'fp32_replay.py', 'capture.py',
    'r2.py', 'position5.py', 'test_fp32_replay.py', 'test_r2.py', 'test_position5.py',
    'arithmetic.rs', 'kernels.rs', 'run.py', 'README.md'}
OUTPUT_ORDER = ('tests.stderr', 'replay.json', 'derived-down.bf16', 'derived-ordered-sum.f32',
                'replayed-final-rank0.bf16', 'replayed-final-rank1.bf16')
OUTPUT_SIZES = {'derived-down.bf16': 8192, 'derived-ordered-sum.f32': 16384,
                'replayed-final-rank0.bf16': 8192, 'replayed-final-rank1.bf16': 8192}
MAX_FILE, MAX_TOTAL = 2 << 20, 16 << 20
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def guard():
    remaining = DEADLINE - time.monotonic()
    require(remaining > 0, '120-second retention deadline')
    signal.setitimer(signal.ITIMER_REAL, remaining)


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    require(type(row) is dict and type(row['bytes']) is int and 0 <= row['bytes'] <= 2 << 30
            and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'data/metadata pin')
    return {key: row[key] for key in ('bytes', 'sha256')}


def ordinary(name):
    return type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute() \
        and '..' not in Path(name).parts and name not in ('', '.')


def read(path, cap=MAX_FILE):
    guard()
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical ordinary body')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_uid, s.st_gid,
                       s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and 0 <= before.st_size <= cap, 'bounded regular body')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'body changed while reading')
    guard()
    return raw


def declarations(raw):
    # Interpret authenticated data literals, never import or execute run.py.
    def literal(node):
        if isinstance(node, ast.Dict):
            keys = [literal(k) for k in node.keys]
            require(len(set(keys)) == len(keys), 'duplicate literal key')
            return dict(zip(keys, (literal(v) for v in node.values)))
        if isinstance(node, ast.Call):
            require(isinstance(node.func, ast.Name) and node.func.id == 'dict' and not node.args
                    and all(k.arg is not None for k in node.keywords), 'closed dict constructor only')
            require(len({k.arg for k in node.keywords}) == len(node.keywords), 'duplicate dict keyword')
            return {k.arg: literal(k.value) for k in node.keywords}
        return ast.literal_eval(node)
    wanted, values = {'INPUTS', 'TEST_NAMES'}, {}
    for node in ast.parse(raw).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in wanted:
                require(name not in values, 'duplicate source declaration')
                values[name] = literal(node.value)
    require(set(values) == wanted and len(values['INPUTS']) == 8 and len(values['TEST_NAMES']) == 26
            and len(set(values['TEST_NAMES'])) == 26, 'closed authenticated declaration roster')
    return values


def uint(value, maximum):
    require(type(value) is int and 0 <= value <= maximum, 'strict bounded integer')
    return value


def envelope(raw, magic):
    require(raw[:8] == magic and len(raw) >= 16, 'original sidecar framing')
    header_bytes, payload_bytes = struct.unpack_from('<II', raw, 8)
    require(2 <= header_bytes <= 128 << 10 and 16 + header_bytes + payload_bytes == len(raw),
            'exact original sidecar extent')
    return parse(raw[16:16 + header_bytes]), raw[16 + header_bytes:]


def selected_parts(bodies, diagnostic):
    nh, np = envelope(bodies['inputs/native-sidecar.bin'], b'FCAP061\0')
    fh, fp = envelope(bodies['inputs/reference-sidecar1.bin'], b'FREF061\0')
    rh, rp = envelope(bodies['inputs/reference-sidecar2.bin'], b'FREF061\0')
    require(fp == rp and fh['captures'] == rh['captures'] and fh['ordinal'] == 1 and rh['ordinal'] == 2,
            'original repeated framework payloads')
    nc, fc = nh['captures'][5], fh['captures'][5]
    require(nc['position'] == fc['position'] == 5 and nc['generation'] == 6 and nc['layer'] == 0
            and fc['input_token'] == 271, 'original position-five capture scope')
    offset = sum(uint(c['payload_bytes'], len(np)) for c in nh['captures'][:5])
    np = np[offset:offset + uint(nc['payload_bytes'], len(np))]
    parts, selected = [], {}
    for side, records, payload in (('native', nc['parts'], np), ('framework', fc['parts'], fp)):
        for part in records:
            role = part['role'] if side == 'native' else part['name']
            if role not in (('first_residual', 'down_partial', 'final_hidden') if side == 'native'
                            else ('down-projection', 'mlp-output', 'first-residual', 'layer0-hidden')):
                continue
            start, size = uint(part['offset'], len(payload)), uint(part['bytes'], len(payload))
            require(size > 0 and start + size <= len(payload), 'original selected slice bound')
            raw = payload[start:start + size]
            row = dict(side=side, role=role, offset=start, **pin(raw))
            if side == 'native':
                row['rank'] = uint(part['rank'], 1)
                key = side, row['rank'], role
            else:
                key = side, role
            require(key not in selected, 'unique selected original')
            parts.append(row); selected[key] = raw
    require(parts == diagnostic['selected_parts'] and len(parts) == 10,
            'all ten reported selected pins match original sidecars')
    require(diagnostic['original_sidecars'] == dict(native=pin(bodies['inputs/native-sidecar.bin']),
            reference=pin(bodies['inputs/reference-sidecar1.bin']),
            repeat=pin(bodies['inputs/reference-sidecar2.bin'])), 'reported whole-sidecar pins')
    return selected


def verify_diagnostic(bodies, diagnostic):
    require(diagnostic['schema'] == 'ferric-causal-position5-r2-boundary-diagnostic-v1'
            and diagnostic['position'] == 5 and diagnostic['input_token'] == 271 and diagnostic['layer'] == 0
            and diagnostic['rows_per_rank'] == 4096 and diagnostic['native_words_checked'] == 8192
            and diagnostic['framework_words_checked'] == 4096, 'complete boundary diagnostic extent')
    require(all(diagnostic[k] is False for k in ('current_machine_instruction_order_independently_proven',
            'dedicated_down_dot_products_replayed', 'gate_up_or_silu_replayed',
            'mismatch_explains_position5_argmax', 'independent_model_reference', 'numerical_acceptance',
            'full_model_acceptance', 'semantic_bug_claimed', 'performance_claim', 'gpu_execution', 'model_execution'))
            and diagnostic['acceptance_threshold'] is None
            and all(diagnostic[k] is True for k in ('framework_inputs_not_substituted_for_native',
                'native_projection_is_derived_not_directly_captured', 'source_boundary_replayed')),
            'boundary-only nonclaims')
    selected = selected_parts(bodies, diagnostic)
    retained = diagnostic.get('retained_outputs')
    if retained is None:
        return False
    require(set(retained) == set(OUTPUT_ORDER) - {'tests.stderr'}
            and all(value == pin(bodies['output/' + name]) for name, value in retained.items()),
            'all five original derived output pins')
    value = parse(bodies['output/replay.json'])
    require(value['schema'] == 'ferric-causal-position5-r2-original-replay-rows-v1'
            and value['position'] == 5 and value['layer'] == 0, 'original all-row output scope')
    native, framework = value['native'], value['framework']
    require(native['schema'] == 'ferric-guarded-mlp-current-r2-replay-v1'
            and native['rows_per_rank'] == 4096 and native['observed_words'] == 8192
            and len(native['rows']) == 4096 and framework['rows'] == 4096
            and native['derived_outputs'] == {n: retained[n] for n in OUTPUT_SIZES},
            'original complete row and derived-array pin joins')
    require(all(native[k] is False for k in ('current_machine_instruction_order_independently_proven',
            'dedicated_down_dot_products_replayed', 'materialized_down_directly_captured',
            'model_weights_read', 'framework_execution', 'native_execution', 'gpu_execution',
            'independent_model_reference', 'numerical_acceptance', 'full_model_acceptance',
            'performance_claim', 'production_authority')) and native['acceptance_threshold'] is None
            and native['source_contract_replayed'] is True
            and framework['framework_inputs_not_substituted_for_native'] is True, 'unchanged replay scope')
    words = {}
    for key, raw in selected.items():
        count = 4096
        code = 'I' if key[-1] == 'down_partial' else 'H'
        require(len(raw) == count * (4 if code == 'I' else 2), 'selected scalar extent')
        words[key] = struct.unpack('<4096' + code, raw)
    outputs = {}
    for name, size in OUTPUT_SIZES.items():
        raw = bodies['output/' + name]
        require(len(raw) == size, 'derived scalar extent')
        outputs[name] = struct.unpack('<4096' + ('I' if size == 16384 else 'H'), raw)
    matched, mismatches, cross = [0, 0], [], [[], []]
    for index, row in enumerate(native['rows']):
        require(type(row['row']) is int and row['row'] == index, 'every original row once in order')
        expected_fields = {
            'down_partial_f32': [format(words['native', rank, 'down_partial'][index], '08x') for rank in (0, 1)],
            'first_residual_bf16': [format(words['native', rank, 'first_residual'][index], '04x') for rank in (0, 1)],
            'observed_final_bf16': [format(words['native', rank, 'final_hidden'][index], '04x') for rank in (0, 1)],
            'ordered_sum_f32': format(outputs['derived-ordered-sum.f32'][index], '08x'),
            'derived_down_bf16': format(outputs['derived-down.bf16'][index], '04x'),
            'replay_final_bf16': [format(outputs['replayed-final-rank%d.bf16' % rank][index], '04x') for rank in (0, 1)],
        }
        require(all(row[k] == v for k, v in expected_fields.items()), 'row original-input/derived-output encoding joins')
        matches = [row['replay_final_bf16'][rank] == row['observed_final_bf16'][rank] for rank in (0, 1)]
        require(type(row['matches']) is list and all(type(v) is bool for v in row['matches'])
                and row['matches'] == matches, 'original row match metadata')
        if not all(matches):
            mismatches.append(index)
        for rank in (0, 1):
            matched[rank] += matches[rank]
            if words['native', rank, 'final_hidden'][index] != words['framework', 'layer0-hidden'][index]:
                cross[rank].append(index)
    require(native['matched_words_per_rank'] == matched and native['matched_words'] == sum(matched)
            and native['mismatch_rows'] == mismatches
            and native['all_final_encodings_match'] is (not mismatches), 'original row census without replay')
    require(diagnostic['native_matched_words'] == sum(matched)
            and diagnostic['native_mismatch_rows'] == mismatches
            and diagnostic['native_boundary_exact'] is native['all_final_encodings_match']
            and diagnostic['framework_matched_words'] == uint(framework['matched_words'], 4096)
            and diagnostic['framework_boundary_exact'] is framework['all_encodings_match']
            and type(framework['all_encodings_match']) is bool
            and framework['all_encodings_match'] is (framework['matched_words'] == 4096)
            and diagnostic['all_observed_boundaries_exact'] is
                (native['all_final_encodings_match'] and framework['all_encodings_match'])
            and diagnostic['native_reference_different_rows'] == cross
            and diagnostic['order'] == native['order'], 'diagnostic/row report joins, not arithmetic reexecution')
    return True


def verify(bodies, terminal_name, terminal_sha):
    guard()
    require(terminal_name in ('complete.json', 'failed.json') and re.fullmatch('[0-9a-f]{64}', terminal_sha),
            'observed terminal name/SHA')
    require(pin(bodies['source-manifest.json']) == SOURCE_MANIFEST, 'frozen original source manifest')
    source = parse(bodies['source-manifest.json'])
    require(source['schema'] == 'ferric-causal-position5-r2-replay-source-v1'
            and set(source['files']) == SOURCES, 'fourteen original source bodies')
    require(all(pin(bodies[name]) == expected for name, expected in source['files'].items()), 'source byte pins')
    constants = declarations(bodies['run.py'])
    inputs = constants['INPUTS']
    require(source['test_names'] == constants['TEST_NAMES'], 'source test census')
    require(all(pin(bodies['inputs/' + name]) == expected for name, expected in inputs.items()),
            'eight original immutable data pins')
    fixed = SOURCES | {'source-manifest.json'} | {'inputs/' + name for name in inputs}
    outputs = {n.removeprefix('output/') for n in bodies if n.startswith('output/')}
    prefix = outputs - {terminal_name}
    require(prefix == set(OUTPUT_ORDER[:len(prefix)]) and terminal_name in outputs
            and set(bodies) == fixed | {'output/' + n for n in outputs}
            and 24 <= len(bodies) <= 30 and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'closed source/input set and actual stable output prefix')
    raw = bodies['output/' + terminal_name]
    require(pin(raw)['sha256'] == terminal_sha, 'observed original terminal hash')
    result = parse(raw)
    require(result['schema'] == 'ferric-causal-position5-r2-replay-cpu-v1' and type(result['passed']) is bool
            and result['passed'] is (terminal_name == 'complete.json'), 'original status/name preserved')
    require(all(result[k] is False for k in ('full_capsule_revalidated', 'original_model_shards_rehashed',
            'current_machine_instruction_order_independently_proven', 'gpu_execution', 'model_execution',
            'native_rerun', 'policy_changed', 'dedicated_down_dot_products_replayed',
            'mismatch_explains_position5_argmax', 'numerical_acceptance', 'full_model_acceptance',
            'semantic_bug_claimed', 'performance_claim', 'production_authority'))
            and result['reused_prior_native_reference_comparison_admission'] is True
            and result['captured_boundary_replay_only'] is True, 'data-only scope/nonclaims')
    require(result['limits'] == dict(wall_seconds=180, cpu_seconds=120, address_space_bytes=512 << 20,
                                  affinity=[8, 9], nice=10), 'actual bounded CPU resource metadata')
    require(type(result['elapsed_seconds']) in (int, float) and 0 <= result['elapsed_seconds'] < 180,
            'actual bounded elapsed interval')
    expected = {str(ROOT / n): pin(bodies[n]) for n in SOURCES | {'source-manifest.json'}}
    expected.update({str(INPUT_ROOT / n): expected for n, expected in inputs.items()})
    require(set(result['inputs']) <= set(expected), 'no unexpected input/readset authority')
    for path, row in result['inputs'].items():
        require(row == dict(path=path, **expected[path]), 'retained input/posthash metadata join')
    require(all(path in {str(ROOT / n) for n in SOURCES} and row == expected[path]
                for path, row in result['source_pins'].items()), 'reported source pins')
    tests = result['tests']
    if tests is not None:
        require(type(tests) is dict and tests['names'] == constants['TEST_NAMES']
                and tests['stderr'] == pin(bodies['output/tests.stderr']), 'original named-test stream pin')
        require(sum(uint(tests[k], 26) for k in ('passed', 'failures', 'errors')) == 26
                and uint(tests['skipped'], 26) <= tests['passed'], 'original observed test census')
    diagnostic_checked = False
    if result['diagnostic'] is not None:
        diagnostic_checked = verify_diagnostic(bodies, result['diagnostic'])
    if result['passed']:
        require(result['error'] is None and result['postcheck_errors'] == []
                and set(result['inputs']) == set(expected)
                and result['source_pins'] == {str(ROOT / n): v for n, v in source['files'].items()},
                'clean complete source/input postchecks')
        require(prefix == set(OUTPUT_ORDER) and tests is not None
                and tests['passed'] == 26 and tests['failures'] == tests['errors'] == tests['skipped'] == 0
                and diagnostic_checked and result['diagnostic']['all_observed_boundaries_exact'] is True,
                'complete declared fixture and boundary gates, not model acceptance')
        lines = bodies['output/tests.stderr'].decode().splitlines()
        rows = []
        pattern = r'(test_\w+) \(((?:test_fp32_replay\.Fp32ReplayTests|test_r2\.R2Tests|test_position5\.Position5Tests))(?:\.(test_\w+))?\) \.\.\. ok'
        for line in lines:
            match = re.fullmatch(pattern, line)
            if match:
                require(match[3] in (None, match[1]), 'unittest repeated method mismatch')
                rows.append(match[2] + '.' + match[1])
        require(sorted(rows) == constants['TEST_NAMES'] and len(rows) == len(set(rows)) == 26
                and sum(bool(re.fullmatch(r'Ran 26 tests in [0-9.]+s', line)) for line in lines) == 1
                and lines.count('OK') == 1, 'exact original unittest outcomes')
    else:
        require(result['error'] is not None or result['postcheck_errors'], 'honest failed terminal')
    guard()
    return dict(original_passed=result['passed'], terminal=pin(raw), original_members=len(bodies),
                source_members=15, input_members=8, output_members=len(outputs),
                original_receipt_and_body_joins_verified=True, complete_output_joins_verified=diagnostic_checked,
                original_test_census=tests, arithmetic_reexecuted=False, model_bodies_retained=False,
                checkpoint_bodies_rehashed=False, native_rerun=False, full_capsule_revalidated=False,
                numerical_acceptance=False, full_model_acceptance=False, performance_claim=False)


def tree(root):
    names = set()
    for parent, dirs, files in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(parent) / n).is_symlink() for n in dirs), 'no directory aliases')
        names.update(str((Path(parent) / n).relative_to(root)) for n in files)
    return names


def export(terminal_name, terminal_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'actual export host')
    require(ROOT.resolve(strict=True) == ROOT and INPUT_ROOT.resolve(strict=True) == INPUT_ROOT
            and not os.path.lexists(ARCHIVE), 'fresh evidence archive and original roots')
    names = tree(ROOT)
    require(16 <= len(names) <= 22 and all(ordinary(n) for n in names), 'bounded original source/output root')
    bodies = {n: read(ROOT / n) for n in names}
    constants = declarations(bodies['run.py'])
    require(tree(INPUT_ROOT) == set(constants['INPUTS']), 'exact eight original input aliases')
    bodies.update({'inputs/' + n: read(INPUT_ROOT / n) for n in constants['INPUTS']})
    checks = verify(bodies, terminal_name, terminal_sha)
    bodies['retention-tool.py'] = read(Path(__file__).resolve())
    manifest = dict(schema='ferric-causal-position5-r2-replay-evidence-v1', terminal_name=terminal_name,
        terminal_sha256=terminal_sha, checks=checks, files={n: pin(b) for n, b in sorted(bodies.items())})
    bodies['manifest.json'] = encoded(manifest)
    require(26 <= len(bodies) <= 32 and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'bounded original members plus helper and manifest')
    require(tree(ROOT) == names and all(read(ROOT / n) == bodies[n] for n in names),
            'all original source/output posthashes')
    require(tree(INPUT_ROOT) == set(constants['INPUTS'])
            and all(read(INPUT_ROOT / n) == bodies['inputs/' + n] for n in constants['INPUTS']),
            'all original read-only input alias posthashes')
    require(read(Path(__file__).resolve()) == bodies['retention-tool.py'], 'retainer source posthash')
    guard()
    with ARCHIVE.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive:
            for name, raw in sorted(bodies.items()):
                guard()
                row = tarfile.TarInfo(name); row.size = len(raw); row.mode = 0o600
                archive.addfile(row, io.BytesIO(raw))
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps(dict(archive=dict(path=str(ARCHIVE), **pin(read(ARCHIVE, MAX_TOTAL))), checks=checks)))


def retain(path, archive_sha, terminal_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha), 'observed archive SHA')
    raw = read(Path(path), MAX_TOTAL)
    require(pin(raw)['sha256'] == archive_sha, 'exact exported archive')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        members = archive.getmembers()
        require(26 <= len(members) == len({m.name for m in members}) <= 32
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers and 0 <= m.size <= MAX_FILE for m in members)
                and sum(m.size for m in members) <= MAX_TOTAL, 'closed bounded ordinary archive')
        bodies = {m.name: archive.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete member reads')
    manifest = parse(bodies['manifest.json'])
    require(manifest['schema'] == 'ferric-causal-position5-r2-replay-evidence-v1'
            and manifest['terminal_sha256'] == terminal_sha
            and set(manifest['files']) == set(bodies) - {'manifest.json'}
            and all(pin(bodies[n]) == value for n, value in manifest['files'].items()), 'all retained member pins')
    require(bodies['retention-tool.py'] == read(Path(__file__).resolve()), 'exact export/retention helper')
    original = {n: body for n, body in bodies.items() if n not in ('manifest.json', 'retention-tool.py')}
    checks = verify(original, manifest['terminal_name'], terminal_sha)
    require(checks == manifest['checks'], 'same checked original claims')
    require(not os.path.lexists(DEST), 'fresh canonical retention destination')
    DEST.mkdir(parents=True, mode=0o700)
    for name, body in sorted(bodies.items()):
        guard()
        dest = DEST / name; dest.parent.mkdir(parents=True, exist_ok=True)
        with dest.open('xb') as stream:
            stream.write(body)
    require(tree(DEST) == set(bodies) and all(read(DEST / n) == body for n, body in bodies.items()),
            'verbatim copy posthashes')
    require(read(Path(path), MAX_TOTAL) == raw, 'archive transport posthash')
    print(json.dumps(dict(destination=str(DEST), members=len(bodies), checks=checks)))


def main():
    global DEADLINE
    require(__debug__ and sys.dont_write_bytecode, 'unoptimized python -B required')
    DEADLINE = time.monotonic() + 120
    def stop(number, _frame):
        raise TimeoutError('retention signal %d' % number)
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, stop)
    resource.setrlimit(resource.RLIMIT_AS, (256 << 20, 256 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 120))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_TOTAL, MAX_TOTAL))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    guard()
    if len(sys.argv) == 4 and sys.argv[1] == 'export':
        export(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 5 and sys.argv[1] == 'retain':
        retain(sys.argv[2], sys.argv[3], sys.argv[4])
    else:
        raise ValueError('evidence.py export TERMINAL_NAME SHA | retain ARCHIVE ARCHIVE_SHA TERMINAL_SHA')
    guard()
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
