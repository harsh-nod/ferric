"""Publish the normalized runtime candidate only after its own completed CPU run."""
import hashlib
import json
from pathlib import Path
import re
import sys

L = Path(__file__).resolve().parent
CASE = L / 'resident-state-fence-cpu-v228-v2'
PACKAGE = L / 'proposals/p228-resident-state-fence-cpu-v2'
SOURCE = L / 'proposals/p228-resident-state-fence-consolidation-v2'
LIVE = Path('/home/harsh/fe2o3-p228-runtime')
Q = Path('/home/harsh/ferric-p227-integration/qualification/resident-state-fence-consolidation-v1')
REMOTE_CASE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/resident-state-fence-cpu-v228-v2'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def checked(path, expected):
    raw = path.read_bytes()
    require(len(raw) == expected['bytes'] and hashlib.sha256(raw).hexdigest() == expected['sha256'],
            'actual pinned bytes: ' + str(path))
    return raw


def main():
    require(len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'actual completion SHA')
    raw = (CASE / 'complete.json').read_bytes()
    require(hashlib.sha256(raw).hexdigest() == sys.argv[1], 'actual terminal receipt')
    value = json.loads(raw)
    require(value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['tests_passed'] == 522 and value['tests_ignored'] == 4,
            'actual selected regression census')
    require(value['empty_initial_target'] is True and value['source_unchanged'] is True
            and value['gpu_execution'] is False and value['performance_claim'] is False
            and value['production_authority'] is False, 'CPU-only qualification scope')
    require(len(value['phases']) == 17, 'exact bounded command census')
    for phase in value['phases'].values():
        require(phase['exit_code'] == 0 and phase['reason'] is None and phase['group_absent'] is True,
                'natural successful bounded leaf')
    records = {'cpu/complete.json': raw}
    for name, expected in value['raw'].items():
        records['cpu/' + name] = checked(CASE / name, expected)
    require(records['cpu/sources-before.json'] == records['cpu/sources-after.json'], 'source postchecks')
    snapshot = json.loads(records['cpu/sources-after.json'])
    manifest = json.loads(checked(PACKAGE / 'manifest.json', value['package_manifest']))
    records['controller/manifest.json'] = (PACKAGE / 'manifest.json').read_bytes()
    for row in manifest['files']:
        records['controller/' + row['path']] = checked(PACKAGE / row['path'], row)
    overlay = json.loads(checked(PACKAGE / 'overlay.json', value['state_overlay']))
    records['implementation/preimages.json'] = checked(SOURCE / 'preimages.json', overlay['preimages'])
    for row in overlay['files']:
        require(snapshot['fe2o3/' + row['path']] == row['after'], 'actually compiled source row')
        body = checked(SOURCE / 'source' / row['path'], row['after'])
        require(checked(LIVE / row['path'], row['after']) == body, 'integrated source exact byte identity')
        records['implementation/' + row['path']] = body
    worker = value['binaries']['ferric-tp-peer-finite-engineering-worker-v1']['binary']
    checked(L / 'resident-state-fence-worker-v228-v2', worker)
    result = dict(schema='ferric-p228-resident-state-fence-public-cpu-v1', authority='none',
        passed=True, cpu_receipt=dict(path=REMOTE_CASE + '/complete.json', bytes=len(raw), sha256=sys.argv[1]),
        tests_passed=522, tests_ignored=4, new_tests=6, runtime_tests=124, worker_tests=398,
        bounded_commands=17, raw_files_rehashed=len(value['raw']), source_files=len(snapshot),
        worker=worker, parent_rebuilt=False, code_scope='eight fe2o3-kfd files',
        expected_group_checks_removed_per_forward=576, expected_reduction_measured=False,
        gpu_execution=False, gpu_numerical_validation=False, numerical_acceptance=False,
        sustained_2048_256=False, performance_claim=False, production_authority=False)
    records['result.json'] = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(not Q.exists(), 'fresh publication directory'); Q.mkdir()
    for name, body in records.items():
        path = Q / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
