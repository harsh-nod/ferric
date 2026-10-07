"""Retain original exact-head data only; never evaluate arithmetic or model code."""
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
import sys
import tarfile
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-readiness40-position5-head-exact-v228-v1'
MODEL = Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target')
DEST = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-readiness40-position5-v1/head-exact-v1')
ARCHIVE = E / 'guarded-mlp-readiness40-position5-head-exact-evidence-v228-v1.tar.gz'
SOURCE_MANIFEST = dict(bytes=1246, sha256='6f3ea976217c46a273046e73e13000de66bb171e63f00b978a07e3de78542df4')
MAX_FILE, MAX_TOTAL = 4 << 20, 8 << 20
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
    # Interpret only the closed data literals, never import or execute run.py.
    values = {}
    def literal(node):
        if isinstance(node, ast.Name):
            require(node.id in ('SHARD', 'INDEX') and node.id in values, 'closed constant name')
            return values[node.id]
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
    wanted = {'SHARD', 'INDEX', 'INPUTS', 'MODEL_PINS', 'TEST_NAMES'}
    for node in ast.parse(raw).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in wanted:
                require(name not in values, 'duplicate source declaration')
                values[name] = literal(node.value)
    require(set(values) == wanted and len(values['INPUTS']) == 8 and len(values['TEST_NAMES']) == 12,
            'closed authenticated declaration roster')
    return values


def verify(bodies, terminal_name, terminal_sha):
    guard()
    require(terminal_name in ('complete.json', 'failed.json') and re.fullmatch('[0-9a-f]{64}', terminal_sha),
            'observed terminal name/SHA')
    require(pin(bodies['source-manifest.json']) == SOURCE_MANIFEST, 'frozen original source manifest')
    source = parse(bodies['source-manifest.json'])
    require(source['schema'] == 'ferric-position5-exact-head-source-v1'
            and set(source['files']) == {'head.py', 'test_head.py', 'run.py', 'README.md'}, 'four original sources')
    require(all(pin(bodies[name]) == expected for name, expected in source['files'].items()), 'source byte pins')
    constants = declarations(bodies['run.py'])
    inputs, model_pins = constants['INPUTS'], constants['MODEL_PINS']
    require(source['test_names'] == constants['TEST_NAMES'], 'source test census')
    require(all(pin(bodies['inputs/' + name]) == expected for name, expected in inputs.items()), 'eight original data pins')
    original_names = set(source['files']) | {'source-manifest.json'} | {'inputs/' + name for name in inputs}
    original_names |= {'output/' + terminal_name, 'output/tests.stderr'}
    require(set(bodies) == original_names and len(bodies) == 15
            and sum(map(len, bodies.values())) <= MAX_TOTAL, 'closed original13 plus2 output bodies')
    raw = bodies['output/' + terminal_name]
    require(pin(raw)['sha256'] == terminal_sha, 'observed original terminal hash')
    result = parse(raw)
    require(result['schema'] == 'ferric-position5-exact-head-cpu-v1' and type(result['passed']) is bool
            and result['passed'] is (terminal_name == 'complete.json'), 'original status/name preserved')
    require(all(result[key] is False for key in ('gpu_execution', 'model_execution', 'native_rerun',
            'mfma_emulation', 'numerical_acceptance', 'full_model_acceptance', 'performance_claim',
            'production_authority', 'full_capsule_revalidated', 'native_weight_upload_revalidated'))
            and result['reused_prior_native_and_reference_admission'] is True, 'data-only scope/nonclaims')
    require(result['limits'] == dict(wall_seconds=180, cpu_seconds=120, address_space_bytes=512 << 20,
                                  affinity=[8, 9], nice=10), 'actual bounded CPU resource metadata')
    require(type(result['elapsed_seconds']) in (int, float) and 0 <= result['elapsed_seconds'] < 180,
            'actual bounded elapsed interval')
    expected = {str(ROOT / name): pin(body) for name, body in bodies.items() if not name.startswith('output/')}
    expected[str(MODEL / constants['INDEX'])] = model_pins[constants['INDEX']]
    require(set(result['inputs']) <= set(expected), 'no unexpected input/readset authority')
    for path, row in result['inputs'].items():
        require(row['path'] == path and compact(row) == expected[path], 'each retained input/posthash metadata join')
    require(all(path in {str(ROOT / n) for n in source['files']} and row == expected[path]
                for path, row in result['source_pins'].items()), 'reported source pins')
    tests = result['tests']
    require(type(tests) is dict and tests['names'] == constants['TEST_NAMES']
            and tests['stderr'] == pin(bodies['output/tests.stderr']), 'original named-test log pin')
    if result['passed']:
        require(result['error'] is None and result['postcheck_errors'] == []
                and set(result['inputs']) == set(expected)
                and result['source_pins'] == {str(ROOT / n): v for n, v in source['files'].items()},
                'clean complete source/input postchecks')
        require(tests['passed'] == 12 and tests['failures'] == tests['errors'] == tests['skipped'] == 0,
                'all12 original synthetic tests')
        lines = bodies['output/tests.stderr'].decode().splitlines()
        rows = []
        for line in lines:
            match = re.fullmatch(r'(test_\w+) \(test_head\.HeadTests(?:\.(test_\w+))?\) \.\.\. ok', line)
            if match:
                require(match[2] in (None, match[1]), 'unittest repeated method mismatch')
                rows.append(match[1])
        require(sorted(rows) == constants['TEST_NAMES'] and len(rows) == len(set(rows)) == 12
                and sum(bool(re.fullmatch(r'Ran 12 tests in [0-9.]+s', line)) for line in lines) == 1
                and lines.count('OK') == 1, 'exact original unittest name/outcome census')
        model = result['model_shard']
        require(result['original_full_head_shard_rehashed'] is True
                and model['path'] == str(MODEL / constants['SHARD'])
                and compact(model) == model_pins[constants['SHARD']]
                and model['tensor_file_offset'] == 128 and compact(model['header'])['bytes'] == 120
                and compact(model['tensor'])['bytes'] == 1244659712
                and set(model['rows']) == {'2', '9112'}, 'original full shard and tensor metadata')
        require(len(model['stable_stat']) == 9 and all(type(n) is int for n in model['stable_stat'])
                and stat.S_ISREG(model['stable_stat'][2]) and model['stable_stat'][3] >= 1
                and model['stable_stat'][6] == model_pins[constants['SHARD']]['bytes'], 'original stable shard stamp')
        for token, offset in (('2', 16512), ('9112', 74645632)):
            require(model['rows'][token]['file_offset'] == offset and compact(model['rows'][token])['bytes'] == 8192,
                    'original bounded row metadata')
        diagnostic = result['diagnostic']
        require(diagnostic['schema'] == 'ferric-position5-exact-head-diagnostic-v1'
                and diagnostic['position'] == 5 and diagnostic['tokens'] == [2, 9112]
                and diagnostic['dot_count'] == 4 and diagnostic['terms_per_dot'] == 4096
                and diagnostic['exact_product_unit'] == '2^-266'
                and all(diagnostic[k] is False for k in ('mfma_emulation', 'framework_accumulation_emulation',
                    'tolerance_applied', 'numerical_acceptance', 'full_model_acceptance', 'performance_claim')),
                'exact diagnostic scope only')
        require(diagnostic['row_pins'] == {token: compact(row) for token, row in model['rows'].items()}, 'row metadata joins')
        native = bodies['inputs/native-capture-5.bin'][242824:]
        reference = bodies['inputs/reference-pass1-pos5.bf16']
        require(len(native) == len(reference) == 606976
                and reference == bodies['inputs/reference-pass2-pos5.bf16'], 'original selected payload/repeat')
        for name, payload in (('native', native), ('reference', reference)):
            require(diagnostic['inputs'][name]['input_pins'] == dict(payload=pin(payload),
                final_norm=pin(payload[294912:303104]), logits=pin(payload[303104:])), 'actual payload slice pins')
    else:
        require(result['error'] is not None or result['postcheck_errors'], 'honest failed terminal')
    guard()
    return dict(original_passed=result['passed'], terminal=pin(raw), original_members=15,
                source_members=5, input_members=8, output_members=2,
                original_receipt_and_body_joins_verified=True, arithmetic_reexecuted=False,
                external_checkpoint_rehashed=False, checkpoint_bodies_retained=False,
                numerical_acceptance=False, performance_claim=False)


def tree(root):
    names = set()
    for parent, dirs, files in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(parent) / n).is_symlink() for n in dirs), 'no directory aliases')
        names.update(str((Path(parent) / n).relative_to(root)) for n in files)
    return names


def export(terminal_name, terminal_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'actual export host')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ARCHIVE), 'fresh evidence archive')
    names = tree(ROOT)
    require(len(names) == 15 and all(ordinary(n) for n in names), 'original15-member root')
    bodies = {n: read(ROOT / n) for n in names}
    checks = verify(bodies, terminal_name, terminal_sha)
    bodies['retention-tool.py'] = read(Path(__file__).resolve())
    manifest = dict(schema='ferric-position5-exact-head-evidence-v1', terminal_name=terminal_name,
        terminal_sha256=terminal_sha, checks=checks, files={n: pin(b) for n, b in sorted(bodies.items())})
    bodies['manifest.json'] = encoded(manifest)
    require(len(bodies) == 17 and sum(map(len, bodies.values())) <= MAX_TOTAL, '17 bounded retained members')
    require(tree(ROOT) == names and all(read(ROOT / n) == bodies[n] for n in names), 'all15 original body posthashes')
    require(read(Path(__file__).resolve()) == bodies['retention-tool.py'], 'retainer source posthash')
    guard()
    with ARCHIVE.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive:
            for name, raw in sorted(bodies.items()):
                guard()
                row = tarfile.TarInfo(name); row.size = len(raw); row.mode = 0o600
                archive.addfile(row, io.BytesIO(raw))
    print(json.dumps(dict(archive=dict(path=str(ARCHIVE), **pin(read(ARCHIVE, MAX_TOTAL))), checks=checks)))


def retain(path, archive_sha, terminal_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha), 'observed archive SHA')
    raw = read(Path(path), MAX_TOTAL)
    require(pin(raw)['sha256'] == archive_sha, 'exact exported archive')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        members = archive.getmembers()
        require(len(members) == len({m.name for m in members}) == 17
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers and 0 <= m.size <= MAX_FILE for m in members)
                and sum(m.size for m in members) <= MAX_TOTAL, 'closed bounded ordinary archive')
        bodies = {m.name: archive.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete member reads')
    manifest = parse(bodies['manifest.json'])
    require(manifest['schema'] == 'ferric-position5-exact-head-evidence-v1'
            and manifest['terminal_sha256'] == terminal_sha
            and set(manifest['files']) == set(bodies) - {'manifest.json'}
            and all(pin(bodies[n]) == value for n, value in manifest['files'].items()), 'all16 retained member pins')
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
    require(tree(DEST) == set(bodies) and all(read(DEST / n) == body for n, body in bodies.items()), 'verbatim copy posthashes')
    require(read(Path(path), MAX_TOTAL) == raw, 'archive transport posthash')
    print(json.dumps(dict(destination=str(DEST), members=17, checks=checks)))


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
