"""CPU-only replay of captured output/residual boundaries; no model reload."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-output-residual-boundary-v1'
HELPER = E / 'p228-layer0-current-comparison-v1/run.py'
HELPER_SHA = '9c3db491d4f8c20890b045474bd1ab5bd8565d42916a8eb7772c0eb07c44717e'
COMPARISON = dict(path=str(E / 'layer0-current-comparison-v228-v1/complete.json'), bytes=180610,
    sha256='758c2479a3f5a5f78e6ff088adb70a31ca5fb19c86e5a25cf718c3e1e46f4dab')
SOURCES = {
    'boundary.py': (PACKAGE / 'boundary.py', 'a5fac7b587739999623167d81c163fc18ebe9a2baf986da1b192b597e7fb0b9d'),
    'test_boundary.py': (PACKAGE / 'test_boundary.py', '2088ec73ed3763647bebc3cf295364b2a029d2c59ca3937d95df1430932d39fb'),
    'residual_oracle.py': (E / 'p228-independent-layer-reference-v1/helpers/residual_oracle.py',
        '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'),
}
PURE_RUNNER = E / 'run_output_residual_boundary_pure_p228_v1.py'
PURE_RUNNER_SHA = 'a00ba58cca7f4ac43d0be56f573be6c21587d57dde48a5c8d0b228a96e7bc791'
PURE = dict(path=str(E / 'output-residual-boundary-pure-v228-v1/complete.json'), bytes=1500,
    sha256='20a16b03d188e2fb4e68d1d44a901ba6e0a73863b5a2402f152c7521f2e02516')
STAGES = (('norm', 8192, 2), ('qkv', 6144, 2), ('query', 4096, 2),
    ('key-cache', 2359296, 2), ('value-cache', 2359296, 2), ('attention', 4096, 2),
    ('output-partial', 16384, 4), ('first-residual', 8192, 2), ('mlp-norm', 8192, 2),
    ('gate', 12288, 2), ('up', 12288, 2), ('activation', 12288, 2),
    ('down-partial', 16384, 4), ('final-hidden', 8192, 2))


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def helper():
    require(HELPER.resolve(strict=True) == HELPER, 'canonical existing data-reader source')
    with HELPER.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((64 << 10) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and len(raw) == before.st_size <= 64 << 10
        and stamp(before) == stamp(after) == stamp(HELPER.lstat()) and digest(raw) == HELPER_SHA,
        'unchanged existing retained-data reader')
    module = types.ModuleType('retained_boundary_data_reader'); module.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), module.__dict__)
    return module


def comparison_reader(H, comparison):
    require(comparison['schema'] == 'ferric-p228-layer0-current-comparison-observation-v1'
        and comparison['completed'] is True and comparison['source_postchecks_passed'] is True
        and comparison['native_structural_replayed'] is True
        and comparison['native_owned_leaves_replayed'] == 7 and comparison['native_audits_replayed'] == 6
        and comparison['framework_owned_leaf_results_rechecked'] == 21,
        'actual completed structural and independent comparison evidence')
    for key in ('all_transitive_admission_inputs_replayed', 'current_gpu_audits_performed', 'gpu_execution',
                'native_process_launched', 'numerical_acceptance', 'full_model_correctness',
                'performance_claim', 'production_authority'):
        require(comparison[key] is False, 'prior diagnostic-only scope')
    rows = comparison['consumed']; originals, mapping = {}, {}
    require(type(rows) is list and len(rows) == 260, 'actual finite consumed-input roster')
    for row in rows:
        require(type(row) is dict and set(row) == {'original', 'retained'}, 'original and transported identities')
        original, retained = H.filepin(row['original']), H.filepin(row['retained'])
        require(original['path'] not in originals and (original['bytes'], original['sha256'])
            == (retained['bytes'], retained['sha256']), 'unique originals and byte-preserving transport')
        originals[original['path']] = row
        if original['path'] != retained['path']:
            mapping[original['path']] = retained
    reader = H.Reader(mapping)
    for row in rows:
        reader(row['original'])
        require(reader.consumed[row['original']['path']] == row, 'actual recorded transport identity')
    return reader, originals


def prior_read(reader, originals, pin):
    require(pin['path'] in originals and originals[pin['path']]['original'] == pin,
            'slice input was consumed by actual prior comparison')
    return reader(pin)


def native_slices(summary, capture):
    require(summary['schema'] == 'FerricFinitePrefixLayerCaptureObservationV1'
        and summary['native_closed'] is True and summary['completed_layers'] == 1
        and len(capture) == 9670656 and len(summary['stages']) == 28, 'actual closed complete layer capture')
    indexed, slices, offset = {}, [], 0
    for index, (stage, size, width) in enumerate(STAGES * 2):
        raw = capture[offset:offset + size]
        expected = dict(rank=index // 14, stage=stage, offset=offset, bytes=size,
            elements=size // width, element_bytes=width, sha256=list(hashlib.sha256(raw).digest()))
        row = summary['stages'][index]
        require(type(row) is dict and set(row) == set(expected)
            and all(type(row[key]) is int for key in ('rank', 'offset', 'bytes', 'elements', 'element_bytes'))
            and type(row['sha256']) is list and all(type(value) is int for value in row['sha256'])
            and row == expected, 'exact native stage extent, order, rank and digest')
        if stage in ('output-partial', 'first-residual'):
            indexed[(index // 14, stage)] = raw
            slices.append(dict(row, sha256=digest(raw)))
        offset += size
    require(offset == len(capture), 'no omitted native bytes')
    return indexed, slices


def framework_slices(H, reader, originals, report):
    require(report['schema'] == 'ferric-p228-layer0-framework-capture-v1'
        and report['position'] == 0 and report['input_token'] == 9112
        and report['genuine_framework_chain'] is True and report['repeat_passes_byte_equal'] is True
        and report['captured_stages_per_pass'] == 33 and len(report['passes']) == 2,
        'actual two-pass genuine layer0 framework capture')
    selected, pins = {}, {}
    for name in ('embedding', 'o-projection', 'first-residual'):
        rows = [entry['stages'][name] for entry in report['passes']]
        for ordinal, (entry, row) in enumerate(zip(report['passes'], rows, strict=True), 1):
            require(entry['ordinal'] == ordinal and entry['fresh_cache'] is True
                and entry['position'] == 0 and entry['input_token'] == 9112 and len(entry['stages']) == 33
                and set(row) == {'dtype', 'shape', 'pin'} and row['dtype'] == 'bfloat16'
                and row['shape'] == [1, 1, 4096] and row['pin']['bytes'] == 8192,
                'genuine fixed-format BF16 framework row')
        first, second = [prior_read(reader, originals, row['pin']) for row in rows]
        require(first == second, 'both actual framework passes agree')
        selected[name] = first; pins[name] = [row['pin'] for row in rows]
    return selected, pins


def pure(H, reader, pin, sources):
    require(pin == PURE, 'actual root-retained pure12 completion')
    H.pure(reader, pin, 'ferric-p228-output-residual-boundary-pure-v1', 12, sources)
    value = reader.doc(pin)
    require(value['synthetic_policy_tests_only'] is True and value['controller']['path'] == str(PURE_RUNNER)
        and value['controller']['sha256'] == PURE_RUNNER_SHA, 'actual bounded pure12 source qualification')
    for key in ('compiler_execution', 'framework_execution', 'runtime_audit_executed', 'gpu_execution',
                'numerical_acceptance', 'production_authority', 'performance_claim'):
        require(value[key] is False, 'synthetic CPU tests confer no additional authority')
    source = reader(sources['test_boundary.py']).decode('utf-8')
    expected = {'test_boundary.' + cls.name + '.' + method.name for cls in ast.parse(source).body
        if isinstance(cls, ast.ClassDef) for method in cls.body
        if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')}
    log = reader(value['transcript']).decode('utf-8')
    rows = re.findall(r'^(test_\w+) \(([^\n]+)\) \.\.\. ok$', log, re.MULTILINE)
    actual = {label if label.endswith('.' + name) else label + '.' + name for name, label in rows}
    require(len(rows) == len(expected) == 12 and actual == expected
        and re.search(r'^Ran 12 tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE), 'all twelve actual named passes')


def main(args):
    require(len(args) == 2 and not sys.flags.optimize and sys.dont_write_bytecode
        and all(key not in os.environ for key in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary -B PLAN_PATH PLAN_SHA invocation')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
        and all(os.environ.get(key) == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
        'fixed GPU-hidden ASROCK CPU execution')
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        values = [limit] + [value for value in resource.getrlimit(kind) if value != resource.RLIM_INFINITY]
        resource.setrlimit(kind, (min(values), min(values)))
    H = helper()
    plan_pin, raw = H.actual(Path(args[0]), args[1]); plan = H.parse(raw)
    require(set(plan) == {'schema', 'comparison', 'boundary_tests', 'output_label'}
        and plan['schema'] == 'ferric-p228-output-residual-boundary-inputs-v1' and plan['comparison'] == COMPARISON
        and type(plan['output_label']) is str
        and re.fullmatch(r'output-residual-boundary-v228-v[1-9][0-9]{0,8}', plan['output_label']), 'closed actual-input plan')
    out = E / plan['output_label']; require(not os.path.lexists(out), 'fresh conditional replay output')
    receipt_pin, raw = H.actual(COMPARISON['path'], COMPARISON['sha256'])
    require(receipt_pin == COMPARISON, 'exact completed comparison extent')
    comparison = H.parse(raw); reader, originals = comparison_reader(H, comparison)
    reader(COMPARISON); reader(plan_pin)
    controller = H.actual(Path(__file__).resolve())[0]; reader(controller)
    reader(H.actual(HELPER, HELPER_SHA)[0])
    sources = {name: H.actual(path, sha)[0] for name, (path, sha) in SOURCES.items()}
    for pin in sources.values(): reader(pin)
    pure(H, reader, plan['boundary_tests'], sources)
    B = H.module(reader, sources['boundary.py'], 'retained_output_residual_boundary')
    native = H.parse(prior_read(reader, originals, comparison['native_outer']))
    summary_pin = comparison['comparison']['inputs']['native_capture_summary']
    require(summary_pin == native['retained_native']['summary.json'], 'actual owned native summary join')
    summary = H.parse(prior_read(reader, originals, summary_pin))
    require(summary['stages'] == native['checked']['stages'], 'previously structurally replayed stage manifest')
    capture_pin = native['retained_native']['candidate-capture.bin']
    capture = prior_read(reader, originals, capture_pin)
    indexed, slice_pins = native_slices(summary, capture)
    framework_outer = H.parse(prior_read(reader, originals, comparison['framework_outer']))
    framework_pin = comparison['comparison']['inputs']['framework_capture']
    require(framework_pin == framework_outer['reference'], 'actual closed framework capture join')
    framework = H.parse(prior_read(reader, originals, framework_pin))
    reference, framework_pins = framework_slices(H, reader, originals, framework)
    result = B.compare([indexed[(rank, 'output-partial')] for rank in range(2)], reference['embedding'],
        [indexed[(rank, 'first-residual')] for rank in range(2)], reference['o-projection'], reference['first-residual'])
    require(result['schema'] == B.SCHEMA and len(result['comparisons']) == 8
        and result['conditional_replay_performed'] is True and result['numerical_acceptance'] is False,
        'conditional eight-comparison diagnostic only')
    reader.recheck()
    output = dict(schema='ferric-p228-output-residual-boundary-observation-v1', completed=True,
        controller=controller, plan=plan_pin, prior_comparison=COMPARISON, prior_consumed_files_rehashed=len(originals),
        pure_tests=plan['boundary_tests'], sources=sources, native_outer=comparison['native_outer'],
        native_summary=summary_pin, native_capture=capture_pin, native_slices=slice_pins,
        framework_outer=comparison['framework_outer'], framework_capture=framework_pin,
        framework_inputs=framework_pins, diagnostic=result, consumed=list(reader.consumed.values()),
        source_postchecks_passed=True, prior_comparison_reexecuted=False, prior_ownership_checks_reexecuted=False,
        all_transitive_admission_inputs_replayed=False, new_native_execution=False,
        framework_execution=False, gpu_execution=False, numerical_acceptance=False,
        full_model_acceptance=False, performance_claim=False, production_authority=False)
    raw = (json.dumps(output, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(raw) <= 16 << 20, 'bounded data-only report')
    out.mkdir(mode=0o700)
    with (out / 'complete.json').open('xb') as stream: stream.write(raw)
    print(json.dumps(dict(complete=H.actual(out / 'complete.json')[0], conditional_replay=True,
        comparisons=[dict(name=row['name'], exact_words=row['exact_words']) for row in result['comparisons']])), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])
