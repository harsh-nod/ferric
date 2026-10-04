"""Rehash the actual completed AR4 observation and publish scoped diagnostics."""
import hashlib
import json
from pathlib import Path

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/'
Q = Path('/home/harsh/ferric-p227-integration/qualification/independent-decode-observation-v1/ar4')
GPU = 'prefix-independent-decode-ar4-shared-full-currentness-gpu-v228-v1'
DIAG = 'prefix-independent-decode-ar4-framework-diagnostic-v228-v1'
INPUT = 'prefix-independent-decode-ar4-inputs-v228-v1'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def checked(record):
    require(record['path'].startswith(E), 'retained evidence root')
    relative = record['path'][len(E):]
    require('..' not in Path(relative).parts, 'canonical relative path')
    raw = (L / relative).read_bytes()
    require(len(raw) == record['bytes'] and hashlib.sha256(raw).hexdigest() == record['sha256'],
            'actual retained bytes and digest')
    return raw


def document(relative, size, sha):
    pin = dict(path=E + relative, bytes=size, sha256=sha)
    return json.loads(checked(pin)), pin


def main():
    gpu, gpu_pin = document(GPU + '/complete.json', 996232,
        'd95058a58f72eff8b46d2d88fae2ec2ef31bdfb2e1f46ead93a1f50d376bc306')
    diag, diag_pin = document(DIAG + '/complete.json', 154238,
        'fd05c7160e6c2ecd2d05dc05675d5a74fa1a200b0f625d74fbd25ebbb2a7ced1')
    require(gpu['passed'] is True and gpu['failures'] == [] and gpu['native_attempts'] == 1
            and gpu['retries'] == 0 and diag['passed'] is True and diag['observation'] == gpu_pin,
            'successful one-attempt run and separate diagnostic')
    require(gpu['mode'] == diag['mode'] == 'autoregressive'
            and diag['acceptance_threshold'] is None and diag['numerical_acceptance'] is False,
            'autoregressive diagnostic without new acceptance threshold')
    retained = {gpu_pin['path']: gpu_pin}
    def walk(value):
        if type(value) is dict:
            if set(value) == {'path', 'bytes', 'sha256'} and value['path'].startswith(E + GPU + '/'):
                checked(value); retained[value['path']] = value
            for item in value.values():
                walk(item)
        elif type(value) is list:
            for item in value:
                walk(item)
    walk(gpu)
    require(set(retained) == {E + str(p.relative_to(L)) for p in (L / GPU).rglob('*') if p.is_file()},
            'entire actual GPU evidence tree retained and rehashed')
    require(len(gpu['leaves']) == 7 and len(gpu['before_audits']) == len(gpu['after_audits']) == 3,
            'one native leaf and six audits')
    for leaf in gpu['leaves'].values():
        owner = json.loads(checked(leaf['result']))
        require(owner['exit_code'] == 0 and owner['reason'] is None
                and owner['cleanup_signalled'] is False and owner['owned_groups_absent'] is True
                and owner['owned_processes_reaped'] is True, 'natural clean owned exits')
    require(checked(diag['sources_before']) == checked(diag['sources_after']), 'diagnostic source postchecks')
    rows = []
    for position, row in enumerate(diag['independent_framework']):
        require(row['position'] == position and row['same_input_history'] is True
                and len(row['tensors']) == 38, 'actual comparable histories at all four positions')
        logits = next(t for t in row['tensors'] if t['name'] == 'logits')
        rows.append(dict(position=position, input_token=row['candidate_input'],
            candidate_output=row['candidate_output'], reference_output=row['reference_output'],
            same_input_history=row['same_input_history'], output_equal=row['output_equal'],
            logits_relative_l2=logits['relative_l2'], logits_max_abs_error=logits['max_abs_error'],
            logits_exact_words=logits['exact_words'], logits_elements=logits['elements']))
    result = dict(schema='ferric-p228-independent-ar4-public-result-v1', authority='none',
        gpu_observation=gpu_pin, framework_diagnostic=diag_pin, selected_runtime=gpu['selected_runtime'],
        summary=diag['summary'], per_position=rows, case_files_rehashed=len(retained),
        case_bytes_rehashed=sum(p['bytes'] for p in retained.values()), native_attempts=1, retries=0,
        natural_owned_leaf_exits=7, device_audits=6,
        forward_host_ns=gpu['checked']['host_observation']['forward_host_ns'],
        numerical_acceptance=False, independent_tensor_acceptance=False, full_model_acceptance=False,
        sustained_2048_256=False, gpu_time=False, performance_claim=False, production_authority=False)
    published_pins = dict(gpu['input_pins'])
    published_pins.update(diag['input_pins'])
    published_pins.update(retained)
    for record in (diag_pin, diag['sources_before'], diag['sources_after']):
        published_pins[record['path']] = record
    if Q.exists():
        require(Q.is_dir() and not Q.is_symlink(), 'canonical existing publication')
    else:
        Q.mkdir()
    def publish(name, raw):
        destination = Q / name
        if destination.exists():
            require(not destination.is_symlink() and destination.read_bytes() == raw,
                    'existing publication must be byte-identical')
        else:
            with destination.open('xb') as stream:
                stream.write(raw)
    for relative, name in ((GPU + '/complete.json', 'gpu-complete.json'),
            (GPU + '/observation.json', 'observation.json'),
            (GPU + '/native-host-policy-v2.json', 'host-observation.json'),
            (DIAG + '/complete.json', 'framework-diagnostic.json'),
            (DIAG + '/sources-before.json', 'diagnostic-sources-before.json'),
            (DIAG + '/sources-after.json', 'diagnostic-sources-after.json'),
            (INPUT + '/plan.json', 'plan.json'), (INPUT + '/request.json', 'request.json'),
            (INPUT + '/decode-review.json', 'decode-review.json')):
        publish(name, checked(published_pins[E + relative]))
    publish('result.json', (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode())
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
