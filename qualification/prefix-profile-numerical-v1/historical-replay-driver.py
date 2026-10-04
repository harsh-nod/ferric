"""CPU-only independent-profile replay of retained, older GPU captures."""
import hashlib
import json
import os
from pathlib import Path
import platform
import sys
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-prefix-profile-numerical-v1'
INPUTS = E / 'p228-profile-replay-inputs-v2'
OUT = E / 'prefix-profile-replay-v228-v2'
SOURCE_SHA = 'd99d27e9912b486ad90b212d08fb297d894033da9845330bea36f199ed4f0122'
INPUT_SHA = '2f463293acdd3fde9b0edd4e037a538bbe5867101d22665391cf68adc730143a'
CPU_SHA = '189bd2fdc8c946777a1216c3578316c1b318b69e8c92ac5ef05ca7c15575f992'


def pin(path):
    raw = path.read_bytes()
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def sources():
    manifest = P / 'source-manifest.json'
    assert pin(manifest)['sha256'] == SOURCE_SHA
    value = json.loads(manifest.read_bytes())
    for row in value['files']:
        path = E / row['path']
        assert path.resolve(strict=True) == path and not path.is_symlink()
        assert pin(path) == {key: row[key] for key in ('bytes', 'sha256')}


def write(name, value):
    path = OUT / name
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    return dict(path=str(path), **pin(path))


def main():
    assert not sys.flags.optimize and sys.dont_write_bytecode
    assert sorted(os.sched_getaffinity(0)) == [8, 9] and os.getpriority(os.PRIO_PROCESS, 0) == 10
    assert os.environ['OPENBLAS_NUM_THREADS'] == os.environ['OMP_NUM_THREADS'] == '1'
    sources()
    cpu_path = E / 'prefix-profile-tests-v228-v1/complete.json'
    assert pin(cpu_path)['sha256'] == CPU_SHA
    cpu = json.loads(cpu_path.read_bytes())
    assert cpu['passed'] is True and cpu['tests'] == 20
    assert pin(INPUTS / 'manifest.json')['sha256'] == INPUT_SHA
    manifest = json.loads((INPUTS / 'manifest.json').read_bytes())
    assert manifest['schema'] == 'ferric-prefix-profile-historical-replay-inputs-v1'
    assert manifest['new_gpu_execution'] is False
    sys.path.insert(0, str(P))
    import compare_profile as C
    frozen = C.helpers()
    V, N = frozen[1], frozen[3]
    roster, replicas, records = {}, {}, {}
    for row in manifest['replicas']:
        record = row['original']
        path = INPUTS / row['replica']
        assert path.resolve(strict=True).is_relative_to(INPUTS) and not path.is_symlink()
        assert roster.setdefault(record['path'], record) == record
        replicas[record['path']] = path
    for record in manifest['external_inputs']:
        assert roster.setdefault(record['path'], record) == record
        assert record['path'] not in replicas

    def read(record, maximum=64 << 10):
        V.pin(record, maximum)
        assert roster[record['path']] == record
        raw = N.raw_file(replicas.get(record['path'], Path(record['path'])), maximum)
        assert len(raw) == record['bytes'] and C.sha(raw) == record['sha256']
        assert records.setdefault(record['path'], record) == record
        return raw

    OUT.mkdir(mode=0o700)
    start, results, error = time.monotonic(), [], None
    try:
        # Precheck the complete retained-input closure, not only successful rows.
        for record in roster.values():
            read(record, max(record['bytes'], 64 << 10))
        assert [row['case'] for row in manifest['plans']] == ['patterned-pos15', 'patterned-pos16']
        for source in manifest['plans']:
            stage = N.document(V, read, source['stage'])
            sidecar = N.document(V, read, source['sidecar'])
            assert stage['schema'] == 'ferric-p227-prefix-stage-inputs-v1'
            assert sidecar['schema'] == 'ferric-p227-prefix-numerical-inputs-v1'
            for key in ('case', 'request', 'observation', 'inspection'):
                assert stage[key] == sidecar[key]
            assert stage['case'] == source['case']
            request = N.document(V, read, stage['request'])
            observation = N.document(V, read, stage['observation'])
            for index, profile in enumerate(C.PROFILES):
                plan = dict(schema=C.PLAN_SCHEMA, case=source['case'], profile=profile,
                            baseline_request=request['baseline_request'], ranks=[
                                dict(rank=rank, capture=observation['captures'][index][rank],
                                     input_sha256=observation['input_sha256'][rank],
                                     output_weights=sidecar['output_weights'][rank]) for rank in range(2)])
                plan_pin = write(source['case'] + '-' + profile + '-plan.json', plan)
                checked = C.compare_profile(plan, read, frozen)
                assert checked['conditional_operator_checks_passed'] is True
                assert all(checked[key] is False for key in C.FALSE_FIELDS)
                result_pin = write(source['case'] + '-' + profile + '-result.json', checked)
                results.append(dict(case=source['case'], profile=profile, plan=plan_pin, result=result_pin,
                                    rank_rows=len(checked['rows'])))
                print(json.dumps(dict(case=source['case'], profile=profile, passed=True)), flush=True)
    except Exception as exc:
        error = type(exc).__name__ + ': ' + str(exc)
    postcheck_errors = []
    try:
        sources()
        assert pin(INPUTS / 'manifest.json')['sha256'] == INPUT_SHA
        assert pin(cpu_path)['sha256'] == CPU_SHA
        for record in list(records.values()):
            read(record, max(record['bytes'], 64 << 10))
    except Exception as exc:
        postcheck_errors.append(type(exc).__name__ + ': ' + str(exc))
    passed = error is None and not postcheck_errors and len(results) == 4
    summary = dict(schema='ferric-prefix-profile-historical-replay-v1', passed=passed,
                   error=error, postcheck_errors=postcheck_errors, results=results,
                   source_manifest=pin(P / 'source-manifest.json'), input_manifest=pin(INPUTS / 'manifest.json'),
                   cpu_tests=pin(cpu_path), controller=pin(Path(__file__).resolve()),
                   host=platform.node(), python=platform.python_version(), numpy=frozen[2].np.__version__,
                   cpu_affinity=[8, 9], nice=10, elapsed_host_seconds=time.monotonic() - start,
                   inputs_rechecked=len(records), input_pins=records,
                   retained_historical_captures_only=True, independent_profiles=True,
                   paired_comparison_performed=False, new_gpu_execution=False,
                   gpu_execution_verified=False, new_kernel_qualified=False,
                   full_model_acceptance=False, numerical_acceptance=False,
                   performance_claim=False, production_authority=False)
    output = write('complete.json' if passed else 'failed.json', summary)
    print(json.dumps(dict(passed=passed, output=output)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
