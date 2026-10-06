"""Bounded existing-data stage diagnostic; no process, model, or GPU launch."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import struct
import sys
import time
import types
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OUT = E / 'guarded-mlp-model-stage-numerical-v228-v2'
NATIVE = dict(path=str(E / 'guarded-mlp-model-stage-capture-gpu-v228-v1/ar4/complete.json'), bytes=97034,
    sha256='324b40f7dcee931c856aec8b353bd35176e02570290aca5376c0075cfdf3be12')
CONTROLLER = dict(path=str(E / 'guarded-mlp-model-stage-capture-gpu-v228-v1/run_model_gpu.py'), bytes=34531,
    sha256='afa4d007153179533136ae7a5be4de860407522cb6b92ebcde12441b2eabe175')
ALIAS_ROOT = E / 'guarded-mlp-model-stage-framework-v228-v1'
ALIAS_SHA = 'f69fff0120cd8b25cf8350b246631a88e504de387b685c1b7c92f181d1d0bd3f'
FRAMEWORK = dict(path=str(E / 'layer0-framework-launch-v228-v1/complete.json'), bytes=17449,
    sha256='46fd9acbca798f05bc737a651e6fba54e65c78688e2d425a8287eef402065edc')
CAPTURE_FUNCTION_SHA = '2ad2129ecf20d530d7cc84b4951682c69a656e0519c8b6de54dc5f2e755c9f6e'
SOURCES = {'compare.py': '8154580de7f5ad40fd4897ce264fc3d92c9a109808ea7ea5dab6d0486f01e622', 'layer_compare.py': '1598e22a3460a9ed2c350fb5d2b5fec6b8c37648b53bb2f1fc714c1fb3506d3f', 'diagnostics.py': '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf', 'validate_observation.py': '367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055', 'guarded_announcement.py': '96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80', 'adapter.py': '5f562d3df722fbc378c69beec94c09d2c6f9493e70a62c622c391ac0a63314fa', 'test_adapter.py': 'ce8fdfa6533b3984ed5476074a23c13f859654422b7fcd83cd6eb3c357696bec'}
SOURCES['test_aliases.py'] = 'fdc4f6b83cf92381ac42cb1dc4ab527b23b410c15828a651cd7713d00330b2e4'
TESTS = ['test_complete_partition_preserves_dedicated_down_and_raw_pins',
         'test_embedding_and_all_rank_shards_use_unchanged_metrics',
         'test_failed_unverified_and_acceptance_receipts_refuse',
         'test_first_divergence_is_observation_order_not_causal_claim',
         'test_fp32_partials_are_never_cast_or_compared_to_full_bf16',
         'test_missing_reordered_extent_and_digest_mutations_refuse']
ALIAS_TESTS = ['test_alias_posthash_and_symlink_mutations_refuse',
               'test_original_pin_and_physical_path_remain_separate_without_fallback']
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)

def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid, value.st_gid,
            value.st_nlink, value.st_size, value.st_mtime_ns, value.st_ctime_ns)

class Reader:
    def __init__(self):
        self.pins = {}
        self.c = None
        self.aliases = {}
        self.physical_pins = {}

    def read_path(self, path, expected=None, maximum=64 << 20, retain=True):
        require(time.monotonic() < DEADLINE, 'whole diagnostic deadline')
        path = Path(path)
        original = str(path)
        alias = self.aliases.get(original)
        if alias is not None:
            require(expected == alias['original'], 'alias requires the unchanged original FilePin')
            path = ALIAS_ROOT / alias['relative']
        require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical regular input path')
        before = path.lstat()
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and 0 <= before.st_size <= maximum <= 128 << 20, 'bounded ordinary input')
        raw = []
        digest = hashlib.sha256()
        size = 0
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
        with os.fdopen(fd, 'rb') as stream:
            require(stamp(before) == stamp(os.fstat(stream.fileno())), 'input open identity')
            while True:
                require(time.monotonic() < DEADLINE, 'whole diagnostic read deadline')
                chunk = stream.read(1 << 20)
                if not chunk:
                    break
                size += len(chunk)
                require(size <= maximum, 'growing input bound')
                digest.update(chunk)
                if retain:
                    raw.append(chunk)
            require(size == before.st_size and stamp(before) == stamp(os.fstat(stream.fileno()))
                    == stamp(path.lstat()), 'input changed during read')
        pin = dict(path=original, bytes=size, sha256=digest.hexdigest())
        require(expected is None or pin == expected, 'actual input pin mismatch')
        require(original not in self.pins or self.pins[original] == pin, 'input changed across reads')
        self.pins[original] = pin
        if alias is not None:
            physical = dict(pin, path=str(path))
            require(original not in self.physical_pins or self.physical_pins[original] == physical,
                    'physical alias changed across reads')
            self.physical_pins[original] = physical
        require(len(self.pins) <= 384 and sum(row['bytes'] for row in self.pins.values()) <= 128 << 20,
                '384 inputs/128 MiB total readset bound')
        return b''.join(raw) if retain else None

    def read(self, pin, maximum=64 << 20, retain=True):
        pin = self.c.pin(pin)
        require(pin['bytes'] <= maximum, 'declared input bound')
        return self.read_path(pin['path'], pin, maximum, retain)

    def tree(self, root, expected):
        actual = set()
        require(root.resolve(strict=True) == root and stat.S_ISDIR(root.lstat().st_mode), 'canonical case directory')
        for directory, dirs, files in os.walk(root, followlinks=False,
                                            onerror=lambda error: (_ for _ in ()).throw(error)):
            require(time.monotonic() < DEADLINE, 'case inventory deadline')
            for name in dirs + files:
                path = Path(directory) / name
                value = path.lstat()
                require(path.resolve(strict=True) == path and
                        (stat.S_ISDIR(value.st_mode) or stat.S_ISREG(value.st_mode)), 'no special case members')
                if stat.S_ISREG(value.st_mode):
                    require(value.st_nlink == 1 and value.st_size <= 8 << 20, 'ordinary bounded case file')
            actual.update(str((Path(directory) / name).relative_to(root)) for name in files)
            require(len(actual) <= 79, 'bounded closed case inventory')
        require(actual == expected, 'exact original case members, no missing or extra file')

    def recheck(self):
        for row in list(self.pins.values()):
            self.read(row, retain=False)
        if self.aliases:
            self.tree(ALIAS_ROOT, {row['relative'] for row in self.aliases.values()} | {'framework-aliases.json'})

    def configure_aliases(self):
        raw = self.read_path(ALIAS_ROOT / 'framework-aliases.json', maximum=65536)
        require(hashlib.sha256(raw).hexdigest() == ALIAS_SHA, 'exact closed historical alias manifest')
        value = self.c.document(raw)
        require(set(value) == {'schema', 'root', 'files', 'historical_receipts_rewritten', 'gpu_execution'}
                and value['schema'] == 'ferric-guarded-mlp-stage-framework-aliases-v1'
                and value['root'] == str(ALIAS_ROOT) and value['historical_receipts_rewritten'] is False
                and value['gpu_execution'] is False and len(value['files']) == 73, 'closed alias scope')
        relative = set()
        for original, row in value['files'].items():
            require(set(row) == {'original', 'relative'} and self.c.pin(row['original'])['path'] == original,
                    'original pinned alias identity')
            name = Path(row['relative'])
            require(not name.is_absolute() and name.as_posix() == row['relative']
                    and '..' not in name.parts and name.parts[0] in ('owner', 'capture')
                    and row['relative'] not in relative, 'unique closed physical alias')
            relative.add(row['relative'])
        require(sum(row['original']['bytes'] for row in value['files'].values()) == 651037,
                'exact 73-body historical extent')
        self.tree(ALIAS_ROOT, relative | {'framework-aliases.json'})
        self.aliases = value['files']

def load(reader, name):
    path = Path(__file__).resolve().parent / name
    raw = reader.read_path(path, maximum=1 << 20)
    require(hashlib.sha256(raw).hexdigest() == SOURCES[name], 'frozen pure source identity')
    module = types.ModuleType('guarded_diagnostic_' + name.replace('.', '_'))
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module

def save(name, value):
    raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw) <= 4 << 20, 'diagnostic output bound')
    with (OUT / name).open('xb') as stream:
        stream.write(raw)
    return dict(path=str(OUT / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())

def interrupted(number, _frame):
    raise RuntimeError('diagnostic interrupted by signal ' + str(number))

def capture_function(reader, validator):
    raw = reader.read(CONTROLLER, 1 << 20)
    nodes = [node for node in ast.parse(raw).body
             if isinstance(node, ast.FunctionDef) and node.name == 'capture_admission']
    require(len(nodes) == 1 and hashlib.sha256(ast.dump(nodes[0], include_attributes=False).encode()).hexdigest()
            == CAPTURE_FUNCTION_SHA, 'unchanged reviewed pure capture admission AST')
    namespace = dict(hashlib=hashlib, struct=struct, IDS=validator.IDS, require=require)
    module = ast.Module(body=nodes, type_ignores=[])
    exec(compile(module, CONTROLLER['path'], 'exec'), namespace)
    return namespace['capture_admission']


def main():
    global DEADLINE
    started_at = time.monotonic()
    DEADLINE = started_at + 300
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B run.py only')
    require(NATIVE is not None and CONTROLLER is not None,
            'actual capture terminal/controller remain pending; no output created')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged data-only host')
    require(OUT.parent.resolve(strict=True) == OUT.parent and not os.path.lexists(OUT), 'fresh diagnostic output')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'diagnostic nice level')
    if priority == 0:
        os.nice(10)
    for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        os.environ[key] = ''
    for kind, maximum in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 4 << 20),
                          (resource.RLIMIT_CPU, 300), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        maximum = min([maximum] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (maximum, maximum))
    handlers = {number: signal.getsignal(number) for number in
                (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)}
    for number in handlers:
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 300)
    OUT.mkdir(mode=0o700)
    reader = Reader()
    error = None
    posterrors = []
    result = None
    tests = None
    try:
        reader.read_path(Path(__file__).resolve(), maximum=1 << 20)
        c = load(reader, 'compare.py')
        reader.c = c
        layer = load(reader, 'layer_compare.py')
        d = load(reader, 'diagnostics.py')
        v = load(reader, 'validate_observation.py')
        announcement = load(reader, 'guarded_announcement.py')
        adapter = load(reader, 'adapter.py')
        t = load(reader, 'test_adapter.py')
        t.C, t.L, t.D, t.A = c, layer, d, adapter
        aliases = load(reader, 'test_aliases.py')
        aliases.C, aliases.R, aliases.OUT = c, Reader, OUT
        require(unittest.defaultTestLoader.getTestCaseNames(t.AdapterTests) == TESTS,
                'exact six pure stage adapter regression names')
        require(unittest.defaultTestLoader.getTestCaseNames(aliases.AliasTests) == ALIAS_TESTS,
                'exact two physical-alias regression names')
        transcript = io.StringIO()
        suite = unittest.TestSuite([unittest.defaultTestLoader.loadTestsFromTestCase(t.AdapterTests),
                                   unittest.defaultTestLoader.loadTestsFromTestCase(aliases.AliasTests)])
        tested = unittest.TextTestRunner(stream=transcript, verbosity=2).run(
            suite)
        tests = dict(names=TESTS + ALIAS_TESTS, run=tested.testsRun, failures=len(tested.failures), errors=len(tested.errors),
            skipped=len(tested.skipped), expected_failures=len(tested.expectedFailures),
            unexpected_successes=len(tested.unexpectedSuccesses), transcript=transcript.getvalue())
        require(tested.wasSuccessful() and tests['run'] == 8 and all(tests[key] == 0 for key in
                ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes')),
                'eight stage/physical-alias regressions')
        reader.configure_aliases()
        capture = capture_function(reader, v)
        result = adapter.compare(reader, c, layer, d, v, announcement, NATIVE, CONTROLLER, capture, FRAMEWORK)
        require(set(reader.physical_pins) == set(reader.aliases), 'all73 original and physical aliases consumed')
    except BaseException as caught:
        error = type(caught).__name__ + ': ' + str(caught)
    try:
        reader.recheck()
    except BaseException as caught:
        posterrors.append(type(caught).__name__ + ': ' + str(caught))
    require(time.monotonic() < DEADLINE, 'whole deadline expired; no completed report accepted')
    passed = error is None and not posterrors
    report = dict(schema='ferric-guarded-mlp-model-stage-numerical-report-v1', passed=passed,
        status='DIAGNOSTIC_COMPLETE' if passed else 'DIAGNOSTIC_FAILED', error=error,
        postcheck_errors=posterrors, selftests=tests, diagnostic=result, sources=SOURCES,
        capture_function_ast_sha256=CAPTURE_FUNCTION_SHA, inputs=reader.pins,
        physical_alias_inputs=reader.physical_pins, original_framework_paths_preserved=True,
        historical_receipts_rewritten=False,
        input_posthashes_complete=not posterrors, elapsed_seconds=time.monotonic() - started_at,
        comparison_completed=passed, gpu_execution=False, subprocess_execution=False, native_rerun=False,
        source_model_shards_rehashed=False, numerical_acceptance=False, acceptance_threshold=None,
        full_model_acceptance=False, performance_claim=False, production_authority=False)
    save('complete.json' if passed else 'failed.json', report)
    print(json.dumps(dict(passed=passed, error=error, numerical_acceptance=False), sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items():
        signal.signal(number, handler)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
