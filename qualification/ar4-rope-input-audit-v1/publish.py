"""Publish retained coefficient-audit metadata; no tested code or math execution."""
import argparse
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
F = Path('/home/harsh/ferric-p227-integration')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/projection-ar4-framework-evidence-v228-v1')
P = L / 'proposals/p228-ar4-rope-input-audit-v1'
SELF = L / 'proposals/p228-ar4-rope-input-audit-publication-v1/publish.py'
Q = F / 'qualification/ar4-rope-input-audit-v1'
SOURCE = {
    'run.py': (10211, '9a45f4dd519b49ba949a247b90da09bbaa8c98b025d92a9c86f10d0500ba8df6'),
    'test_run.py': (4419, '810a4becf62bfaf42ebe9f83bd56007ed82e779b9974f1397a1bb40e1def9365'),
    'README.md': (5340, 'dd1622b92129eab6f4b27a293f0389838485f4d898d7e95770471985bf6577bc'),
}
INPUTS = {
    'rope-framework-reference-v228-v1/complete.json': (330074, 'f5c47aabf7a307a19f9fe54fa37187f8548ceab4ac92eeeac2638fc9cc4589b4'),
    'projection-ar4-framework-reference-v228-v1/reference.json': (201239, '00952244362ad51d241d179f741ae5ae61ff8acfcb3fd160b9dc64ce3b5f699e'),
    'projection-ar4-framework-inputs-v228-v1/native-case/complete.json': (1054985, '15938580d218f855883a589c819d532bbf941a02c4677cb29928f7bf7106d1cb'),
    'projection-ar4-framework-inputs-v228-v1/native/request-0.json': (1865, 'dfdfef97a872fcb7622998489f219b61cff5a9d4dbe4c27c1955b7ab0c6066ce'),
    'projection-ar4-framework-inputs-v228-v1/native/request-1.json': (2400, '04cd581b9b4c7b75f1bb988e0f6c9f8e5bff828b412347ebbdfad9d665429c00'),
    'projection-ar4-framework-inputs-v228-v1/native/request-2.json': (2404, 'bba7e200de72e40866022407e2781769a5c48b75b0021e0947bdf850386e75c4'),
    'projection-ar4-framework-inputs-v228-v1/native/request-3.json': (2405, '0acf40630457e988af9f6e003def15f8f5c3562756616deaa03cac0964b3533e'),
}
FRAMEWORK_PUBLIC_SHA = '23567420a73dc63a393ab54da8606b24ceedd4afee866aab9967fd3196c21075'
FALSE = ('gpu_execution', 'model_loaded', 'tests_executed', 'native_or_framework_lifecycle_replayed',
    'raw_f32_table_equality_required', 'actual_gpu_trigonometry_validated', 'actual_long_position_tables_validated',
    'new_rope_image_validated', 'numerical_acceptance', 'full_model_correctness', 'production_authority',
    'performance_claim', 'sustained_2048_256')
OBSERVATION = 'Primary-agent observation of actual SSH tool output; not a remote supervisor receipt'
READS = {}


def require(ok, why):
    if not ok:
        raise ValueError(why)


def extent(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def pin(path, raw):
    return dict(path=str(path), **extent(raw))


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, expected=None):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical retained path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 2 << 20,
            'bounded ordinary retained file')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    actual = pin(path, raw)
    require(stamp(before) == stamp(path.lstat()) and len(raw) == before.st_size, 'stable read')
    require(expected is None or (len(raw), actual['sha256']) == expected, 'exact retained identity: ' + str(path))
    require(str(path) not in READS or READS[str(path)] == actual, 'input changed during publication')
    READS[str(path)] = actual
    return raw


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode('ascii')


def main(args):
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    require(Path(__file__).resolve() == SELF and all(re.fullmatch('[0-9a-f]{64}', value)
            for value in (args.complete_sha256, args.pure_sha256)), 'exact publisher and actual SHA arguments')
    match = re.fullmatch(r'ar4-rope-input-audit-complete-v228-v([1-9][0-9]*)[.]json', args.complete.name)
    require(args.complete.parent == L and match, 'exact flat retained audit namespace')
    original_complete = E / ('ar4-rope-input-audit-v228-v' + match.group(1)) / 'complete.json'
    require(args.pure.parent == L and args.pure.suffix == '.json', 'local primary observation namespace')
    source = {name: read(P / name, expected) for name, expected in SOURCE.items()}
    complete_raw, pure_raw = read(args.complete), read(args.pure)
    require(extent(complete_raw)['sha256'] == args.complete_sha256
            and extent(pure_raw)['sha256'] == args.pure_sha256, 'root-authenticated actual outcomes')
    value, pure = parse(complete_raw), parse(pure_raw)
    require(value['schema'] == 'ferric-p228-ar4-rope-input-audit-v1' and value['completed'] is True
            and value['source_postchecks_passed'] is True and value['positions'] == [0, 1, 2, 3]
            and value['coefficient_words'] == 512 and all(value[key] is False for key in FALSE), 'actual scoped data replay')
    require(value['arithmetic'] == 'integer FP32 to BF16 round-to-nearest-even only'
            and value['caveat'] == 'Position-zero residual differences remain; four-position coefficient equality does not identify every error source.',
            'retained arithmetic and position-zero limitation')
    require(type(value['rows']) is list and len(value['rows']) == 4, 'four recorded rows')
    total = 0
    for position, row in enumerate(value['rows']):
        require(set(row) == {'position', 'coefficient_words', 'bf16_exact_words', 'bf16_different_indices',
            'raw_f32_equal_to_retained_libm_model', 'raw_f32_equal_to_framework_companion'}
            and type(row['position']) is int and row['position'] == position and row['coefficient_words'] == 128,
            'closed recorded comparison row')
        differences = row['bf16_different_indices']
        require(type(differences) is list and all(type(i) is int and 0 <= i < 128 for i in differences)
                and differences == sorted(set(differences)), 'recorded ordered mismatch indices')
        for key in ('bf16_exact_words', 'raw_f32_equal_to_retained_libm_model', 'raw_f32_equal_to_framework_companion'):
            require(type(row[key]) is int and 0 <= row[key] <= 128, 'bounded recorded word count')
        require(row['bf16_exact_words'] == 128 - len(differences), 'recorded mismatch/count consistency')
        total += row['bf16_exact_words']
    require(type(value['bf16_exact_words']) is int and value['bf16_exact_words'] == total
            and value['all_bf16_coefficients_equal'] is (total == 512), 'recorded summary consistency, not a fitted equality gate')
    expected = {str(E / name): dict(path=str(E / name), bytes=size, sha256=sha)
                for name, (size, sha) in INPUTS.items()}
    expected.update({str(E / P.name / name): pin(E / P.name / name, source[name]) for name in ('run.py', 'test_run.py')})
    require(type(value['inputs']) is list and len(value['inputs']) == len(expected)
            and {row['path']: row for row in value['inputs']} == expected, 'exact seven inputs and two executed sources')
    framework_public = F / 'qualification/projection-ar4-framework-v1/result.json'
    public_raw = read(framework_public)
    require(extent(public_raw)['sha256'] == FRAMEWORK_PUBLIC_SHA, 'published reference checkpoint')
    public = parse(public_raw)
    require(public['schema'] == 'ferric-p228-projection-ar4-framework-publication-v1', 'prior publication schema')
    retained, documents = [], {}
    for name, record in INPUTS.items():
        if name == 'rope-framework-reference-v228-v1/complete.json':
            path = F / 'qualification/rope-framework-reference-v1/complete.json'
        elif name == 'projection-ar4-framework-reference-v228-v1/reference.json':
            path = F / 'qualification/projection-ar4-framework-v1/reference/reference.json'
        else:
            path = W / name
        raw = read(path, record)
        original = expected[str(E / name)]
        if not name.startswith('rope-framework-reference-'):
            require(public['retained_files'][name] == original, 'original input in published retained ledger')
        retained.append(dict(original=original, retained=pin(path, raw)))
        documents[name] = parse(raw)
    reference = documents['projection-ar4-framework-reference-v228-v1/reference.json']
    native = documents['projection-ar4-framework-inputs-v228-v1/native-case/complete.json']
    require(reference['native_checked'] == native['checked'] and len(value['requests']) == 4, 'recorded native/reference join')
    for position, row in enumerate(value['requests']):
        name = 'request-' + str(position) + '.json'
        original = native['retained_native'][name]
        record = expected[str(E / 'projection-ar4-framework-inputs-v228-v1/native' / name)]
        require(row == dict(original=original, retained=record) and row in reference['native_consumed'],
                'actual original/transport request identity')
    require(pure['schema'] == 'ferric-p228-ar4-rope-input-audit-root-test-observation-v1'
            and pure['observation_kind'] == OBSERVATION
            and all(type(pure[key]) is int and pure[key] == n for key, n in
                (('exit_code', 0), ('tests', 11), ('failures', 0), ('errors', 0), ('skipped', 0)))
            and pure['source_sha256'] == {'run': SOURCE['run.py'][1], 'test': SOURCE['test_run.py'][1]},
            'primary eleven-test observation, not a remote supervisor receipt')
    require(all(pure[key] is False for key in ('remote_supervisor_receipt', 'model_execution',
            'gpu_execution', 'numerical_acceptance', 'performance_claim')), 'primary observation scope')
    names = sorted(node.name for node in ast.walk(ast.parse(source['test_run.py']))
                   if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    log = pure['test_output_excerpt']
    observed = re.findall(r'^(test_\w+) \([^()\n]*AuditTests(?:\.test_\w+)?\) \.\.\. ok$', log, re.M)
    require(len(names) == len(set(names)) == 11 and sorted(observed) == names
            and re.search(r'\nRan 11 tests in [0-9.]+s\n\nOK\n?\Z', log)
            and not re.search(r'\.\.\. (?:FAIL|ERROR|skipped)|^FAILED', log, re.M), 'actual named primary test transcript')
    self_raw = read(SELF)
    copies = {'complete.json': (pin(original_complete, complete_raw), complete_raw),
        'pure-root-observation.json': (pin(args.pure, pure_raw), pure_raw),
        'publish.py': (pin(SELF, self_raw), self_raw)}
    copies.update({'source/' + name: (pin(P / name if name == 'README.md' else E / P.name / name, raw), raw)
                   for name, raw in source.items()})
    result = dict(schema='ferric-p228-ar4-rope-input-audit-publication-v1', publication_passed=True,
        complete=copies['complete.json'][0], pure_observation=copies['pure-root-observation.json'][0],
        primary_test_observation_kind=OBSERVATION, pure_tests=11, publication_helper=pin(SELF, self_raw),
        prior_framework_publication=pin(framework_public, public_raw), input_bodies=retained,
        coefficient_words=512, bf16_exact_words=total, all_bf16_coefficients_equal=value['all_bf16_coefficients_equal'],
        rows=value['rows'], recorded_summary_consistency_checked=True, seven_input_bodies_rehashed=True,
        original_and_retained_complete=dict(original=pin(original_complete, complete_raw),
                                           retained=pin(args.complete, complete_raw)),
        coefficient_rounding_recomputed=False, tests_rerun=False, tested_modules_imported=False,
        model_or_gpu_rerun=False, transitive_input_bodies_rehashed=False, native_or_framework_lifecycle_replayed=False,
        actual_gpu_trigonometry_validated=False, actual_long_position_tables_validated=False,
        new_rope_image_validated=False, numerical_acceptance=False, full_model_correctness=False,
        production_authority=False, performance_claim=False, sustained_2048_256=False,
        caveat=value['caveat'], files={name: dict(original=original, **extent(raw))
            for name, (original, raw) in copies.items()})
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {path.name for path in Q.iterdir()} <= {'README.md'},
                'fresh output with optional root README only')
        if os.path.lexists(Q / 'README.md'):
            read(Q / 'README.md')
    for record in list(READS.values()):
        read(Path(record['path']), (record['bytes'], record['sha256']))
    Q.mkdir(exist_ok=True)
    outputs = {name: raw for name, (_, raw) in copies.items()}
    outputs['result.json'] = encoded(result)
    for name, raw in outputs.items():
        path = Q / name
        path.parent.mkdir(exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
        require(read(path, (len(raw), extent(raw)['sha256'])) == raw, 'published exact bytes')
    for record in list(READS.values()):
        read(Path(record['path']), (record['bytes'], record['sha256']))
    require({str(path.relative_to(Q)) for path in Q.rglob('*') if path.is_file()}
            == set(outputs) | ({'README.md'} if (Q / 'README.md').exists() else set()), 'closed published roster')
    print(json.dumps(dict(result=pin(Q / 'result.json', outputs['result.json']), copied_files=len(copies))))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('complete', type=Path); parser.add_argument('complete_sha256')
    parser.add_argument('pure', type=Path); parser.add_argument('pure_sha256')
    main(parser.parse_args())
