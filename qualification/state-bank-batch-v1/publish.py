"""Export exact CPU records and paired source, never inferred GPU acceptance."""
import hashlib
import json
from pathlib import Path
import re
import sys

L = Path(__file__).resolve().parent
P = L / 'proposals/p228-state-bank-batch-cpu-v1'
C = L / 'state-bank-batch-cpu-v228-v1'
PURE = L / 'state-bank-batch-pure-v228-v1'
E = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/'
ROOTS = {'fe2o3': Path('/home/harsh/fe2o3-p228-runtime'),
         'ferric': Path('/home/harsh/ferric-p227-integration')}
Q = ROOTS['ferric'] / 'qualification/state-bank-batch-v1'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical file')
    body = path.read_bytes()
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def checked(path, expected):
    require(pin(path) == {key: expected[key] for key in ('bytes', 'sha256')},
            'exact retained file: ' + str(path))
    return path.read_bytes()


def encode(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def main():
    require(not sys.flags.optimize and len(sys.argv) == 2
            and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'ordinary Python ACTUAL_RECEIPT_SHA')
    require(pin(C / 'complete.json')['sha256'] == sys.argv[1], 'actual completion')
    value = json.loads((C / 'complete.json').read_bytes())
    require(value['schema'] == 'ferric-p228-state-bank-batch-cpu-result-v1'
            and value['passed'] is True and value['error'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['tests_passed'] == 553 and value['tests_ignored'] == 4
            and value['empty_initial_target'] is True, 'successful CPU qualification')
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority'):
        require(value[key] is False, 'CPU-only scope')
    require(len(value['phases']) == 21 and len(value['raw']) == 109, 'closed command/raw census')
    for row in value['phases'].values():
        require(row['exit_code'] == 0 and row['reason'] is None and row['group_absent'] is True,
                'natural successful owned command')
    records = {'cpu/complete.json': (C / 'complete.json').read_bytes()}
    for name, row in value['raw'].items():
        require(Path(name).name == name and row['path'] == E + C.name + '/' + name, 'raw identity')
        records['cpu/' + name] = checked(C / name, row)
    require(records['cpu/sources-before.json'] == records['cpu/sources-after.json'], 'source postcheck')
    expected = json.loads(records['cpu/sources-after.json'])
    actual = {str(p.relative_to(C / 'sources')): pin(p)
              for p in (C / 'sources').rglob('*') if p.is_file()}
    require(len(actual) == 6931 and actual == expected, 'all retained compiled sources')
    records['controller/manifest.json'] = checked(P / 'manifest.json', value['package_manifest'])
    require(value['package_manifest']['sha256'] ==
            'f882275f45f49bd3b999422f668c7ccfee3c1b47d173a98c17a19e5887551b49', 'reviewed frozen package')
    manifest = json.loads(records['controller/manifest.json'])
    for row in manifest['files']:
        require(Path(row['path']).name == row['path'], 'flat source')
        records['controller/' + row['path']] = checked(P / row['path'], row)
    overlay = json.loads(checked(P / 'overlay.json', value['overlay']))
    for key in ('runtime_preimages', 'ferric_source_manifest'):
        row = overlay[key]
        require(row['path'].startswith(E), 'source manifest root')
        records['implementation/' + key + '.json'] = checked(L / 'proposals' / row['path'][len(E):], row)
    require(len(overlay['files']) == 10, 'paired source delta')
    for row in overlay['files']:
        key = row['project'] + '/' + row['path']
        require(expected[key] == row['after'], 'actually compiled source')
        body = checked(L / 'proposals' / row['source'], row['after'])
        require(checked(ROOTS[row['project']] / row['path'], row['after']) == body, 'integrated byte identity')
        records['implementation/' + key] = body
    worker = value['binaries']['ferric-tp-peer-finite-engineering-worker-v1']['binary']
    checked(L / 'state-bank-batch-worker-v228-v1', worker)
    require(pin(PURE / 'complete.json')['sha256'] ==
            '9c35b195b915bd4957d7513e5e2364b477ccc6a4a2285c92cf8791f7a22a0696', 'actual pure result')
    pure = json.loads((PURE / 'complete.json').read_bytes())
    require(pure['passed'] is True and pure['tests'] == 14 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['manifest_sha256'] == value['package_manifest']['sha256'], 'exact tested controller')
    records['pure/complete.json'] = (PURE / 'complete.json').read_bytes()
    for key in ('sources_before', 'sources_after', 'transcript'):
        row = pure[key]
        records['pure/' + Path(row['path']).name] = checked(PURE / Path(row['path']).name, row)
    runner = L / 'run_state_bank_batch_pure_p228_v1.py'
    require(pin(runner)['sha256'] == pure['controller_sha256'], 'actual pure runner')
    records['run_pure.py'] = runner.read_bytes()
    records['publish.py'] = Path(__file__).read_bytes()
    result = dict(schema='ferric-p228-state-bank-batch-public-cpu-v1', passed=True,
        cpu_receipt=dict(path=E + C.name + '/complete.json', **pin(C / 'complete.json')),
        tests_passed=553, tests_ignored=4, runtime_tests=144, worker_tests=409,
        new_tests=21, mapped_atomic_tests=10, controller_tests=14,
        bounded_commands=21, raw_files_rehashed=109, source_files=6931, worker=worker,
        parent_rebuilt=False, expected_group_checks_removed_per_forward=572,
        expected_reduction_measured=False, gpu_execution=False, numerical_acceptance=False,
        sustained_2048_256=False, performance_claim=False, production_authority=False)
    records['result.json'] = encode(result)
    require(not Q.exists(), 'fresh publication directory')
    Q.mkdir()
    for name, body in records.items():
        path = Q / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body)
    print(json.dumps(dict(published_files=len(records), **result), indent=2))


if __name__ == '__main__':
    main()
