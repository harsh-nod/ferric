"""Verify retained result joins and publish exact machine-generated evidence."""
import hashlib
import json
from pathlib import Path

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/'
Q = Path('/home/harsh/ferric-p227-integration/qualification/independent-decode-observation-v1/tf4')
GPU = 'prefix-independent-decode-tf4-shared-full-currentness-gpu-v228-v1'
DIAG = 'prefix-independent-decode-tf4-framework-diagnostic-v228-v1'
OLD = 'group-fence-tf4-shared-full-currentness-gpu-v228-v1'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(relative):
    raw = (L / relative).read_bytes()
    return dict(path=E + relative, bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def checked(record):
    require(record['path'].startswith(E), 'retained evidence root')
    relative = record['path'][len(E):]
    require('..' not in Path(relative).parts and pin(relative) == record, 'exact retained pin')
    return (L / relative).read_bytes()


def doc(relative, sha):
    record = pin(relative)
    require(record['sha256'] == sha, 'actual terminal root receipt')
    return json.loads(checked(record)), record


def main():
    gpu, gpu_pin = doc(GPU + '/complete.json',
        'db417b2f7d0728577a711aa212564d0f3df783ec33a133fadaf35356ea9c159b')
    diag, diag_pin = doc(DIAG + '/complete.json',
        '3564507b9166628a22ae9967e41bdcab2f2ae349e6d468e72684bda9ff1b7945')
    old, old_pin = doc(OLD + '/complete.json',
        'ebecff7a6e459cbb6fa8b4c750bc9c524adb417405e2827608175ba0a2c66a5f')
    require(gpu['passed'] is True and gpu['failures'] == [] and gpu['native_attempts'] == 1
        and gpu['retries'] == 0 and diag['passed'] is True and diag['observation'] == gpu_pin,
        'actual successful one-attempt run and separate diagnostic')
    require(diag['summary']['compared_tensor_rows'] == 152
        and diag['acceptance_threshold'] is None and diag['numerical_acceptance'] is False,
        'all152 diagnostics without an invented acceptance threshold')
    retained = {gpu_pin['path']: gpu_pin}
    def walk(value):
        if type(value) is dict:
            if set(value) == {'path', 'bytes', 'sha256'} and type(value['path']) is str:
                if value['path'].startswith(E + GPU + '/'):
                    checked(value); retained[value['path']] = value
            for item in value.values():
                walk(item)
        elif type(value) is list:
            for item in value:
                walk(item)
    walk(gpu)
    actual = {E + str(p.relative_to(L)) for p in (L / GPU).rglob('*') if p.is_file()}
    require(set(retained) == actual, 'all case files retained and rehashed')
    for side in ('before', 'after'):
        require(len(gpu[side + '_audits']) == 3, 'actual six audits')
    for leaf in gpu['leaves'].values():
        owner = json.loads(checked(leaf['result']))
        require(owner['exit_code'] == 0 and owner['reason'] is None
            and owner['cleanup_signalled'] is False and owner['owned_groups_absent'] is True
            and owner['owned_processes_reaped'] is True, 'natural exit and clean reap')
    for name in ('sources_before', 'sources_after'):
        checked(diag[name])
    require(checked(diag['sources_before']) == checked(diag['sources_after']), 'diagnostic source postchecks')
    historical, rows = [], []
    for index, row in enumerate(diag['independent_framework']):
        require(row['position'] == index and row['same_input_history'] is True and len(row['tensors']) == 38,
            'four same-history framework comparisons')
        a, b = old['retained_native'][f'observation-{index}.bin'], gpu['retained_native'][f'observation-{index}.bin']
        equal = checked(a) == checked(b)
        historical.append(dict(position=index, historical=a, candidate=b, payload_byte_equal=equal))
        logits = next(t for t in row['tensors'] if t['name'] == 'logits')
        rows.append(dict(position=index, input_token=row['candidate_input'],
            reference_output=row['reference_output'], candidate_output=row['candidate_output'],
            output_equal=row['output_equal'], logits_relative_l2=logits['relative_l2'],
            logits_max_abs_error=logits['max_abs_error'], logits_exact_words=logits['exact_words'],
            logits_elements=logits['elements']))
    result = dict(schema='ferric-p228-independent-tf4-public-result-v1', authority='none',
        gpu_observation=gpu_pin, framework_diagnostic=diag_pin,
        selected_runtime=gpu['selected_runtime'], summary=diag['summary'], per_position=rows,
        historical_native_observation=old_pin, postrun_historical_payload_checks=historical,
        case_files_rehashed=len(retained), case_bytes_rehashed=sum(p['bytes'] for p in retained.values()),
        native_attempts=1, retries=0, natural_owned_leaf_exits=len(gpu['leaves']), device_audits=6,
        forward_host_ns=gpu['checked']['host_observation']['forward_host_ns'],
        numerical_acceptance=False, independent_tensor_acceptance=False, full_model_acceptance=False,
        sustained_2048_256=False, gpu_time=False, performance_claim=False, production_authority=False)
    require(not Q.exists(), 'fresh public result directory')
    Q.mkdir()
    for relative, name in ((GPU + '/complete.json', 'gpu-complete.json'),
                           (GPU + '/observation.json', 'observation.json'),
                           (GPU + '/native-host-policy-v2.json', 'host-observation.json'),
                           (DIAG + '/complete.json', 'framework-diagnostic.json'),
                           (DIAG + '/sources-before.json', 'diagnostic-sources-before.json'),
                           (DIAG + '/sources-after.json', 'diagnostic-sources-after.json'),
                           ('prefix-independent-decode-tf4-inputs-v228-v2/plan.json', 'plan.json'),
                           ('prefix-independent-decode-tf4-inputs-v228-v2/request.json', 'request.json'),
                           ('prefix-independent-decode-tf4-inputs-v228-v2/decode-review.json', 'decode-review.json')):
        with (Q / name).open('xb') as stream:
            stream.write((L / relative).read_bytes())
    with (Q / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True, allow_nan=False); stream.write('\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
