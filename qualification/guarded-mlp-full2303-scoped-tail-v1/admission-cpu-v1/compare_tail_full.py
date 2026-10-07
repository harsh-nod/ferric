"""Pinned Full bank/census/tail behavior comparison; data admission, never a native launcher."""
import argparse
import json
import os
from pathlib import Path
import resource
import signal

import compare_full as C
import full_announcement as A
import validate_full as V
import validate_tail_full as S

SCHEMA = 'ferric-full2303-bank-scoped-census-tail-behavior-admission-v1'
INPUT_NAMES = ('wrapper.json', 'summary.json', 'request.json', 'parent-result.json',
               'parent-started.json', 'parent-command.json', 'parent-stderr')
SELECTOR = '--observe-guarded-full2303-bank-scoped-census-tail-v1'


def admit(native_bodies, pins, native_read, reference_read):
    """Seven observed originals; independent histories are compared without substitution."""
    V.require(set(native_bodies) == set(pins) == set(INPUT_NAMES),
              'seven observed Full bank/census/tail native admission originals')
    for name in INPUT_NAMES:
        maximum = (128 << 10) if name in ('wrapper.json', 'summary.json') else (
            65536 if name == 'request.json' else 8 << 20)
        C.checked(native_bodies[name], pins[name], maximum)
    reference = C.authenticate_reference(reference_read)
    request = V.parse(native_bodies['request.json'])
    base = request['base']
    for name, pin in reference['prompt_pins'].items():
        actual = V.rust_pin(base['prompt'][name])
        V.require({k: actual[k] for k in ('bytes', 'sha256')} == pin,
                  'native authentic reference prompt pin')
    V.require(V.octets(base['expected_model_id']).hex() == reference['inner']['model_id']
              and V.octets(base['expected_bundle_id']).hex() == reference['inner']['bundle_id'],
              'same actual target model and bundle')
    tail = S.validate(native_bodies['wrapper.json'], native_bodies['summary.json'],
                        request, Path(base['evidence_directory']), native_read, reference['tokens'])
    native = tail['ordinary']
    result = V.parse(native_bodies['parent-result.json'])
    started = V.parse(native_bodies['parent-started.json'])
    for role, name in [('stderr', 'parent-stderr'), ('stdout', 'wrapper.json'),
                       ('started', 'parent-started.json'), ('command', 'parent-command.json')]:
        V.require(result[role] == dict(path=result[role]['path'], **C.compact(native_bodies[name])),
                  'original owned parent raw body join')
    command = V.parse(native_bodies['parent-command.json'])
    argv = command['argv']
    V.require(type(argv) is list and len(argv) == 5 and all(type(v) is str for v in argv)
              and Path(argv[0]).name == 'ferric-qwen3-guarded-mlp-full2303-engineering'
              and Path(argv[0]).is_absolute() and Path(argv[2]).is_absolute()
              and argv[1] == '--request'
              and argv[3:] == [SELECTOR, '--allow-unauthenticated-machine-code']
              and command['gpu_execution_requested'] is True and result['gpu_execution_requested'] is True,
              'actual explicit Full bank/census/tail parent command')
    custody = A.validate_lineage(native_bodies['parent-stderr'], result, started, native['child_pid'])
    native['process_retirement_independently_checked'] = True
    tail['outer_owned_lineage_checked'] = True
    summary = V.parse(native_bodies['summary.json'])
    decoded_pin = V.rust_pin(summary['files']['decoded_output'])
    decoded = native_read(decoded_pin)
    C.checked(decoded, {k: decoded_pin[k] for k in ('bytes', 'sha256')}, 128 << 10)
    behavioral = C.compare_choices(native['generated_tokens'], decoded,
        reference['passes'][0]['generated_tokens'], reference['outputs'][0]['decoded.bin'])
    return dict(schema=SCHEMA, passed=behavioral['passed'], tail=tail, custody=custody,
        comparison=behavioral, native_admission_pins=pins,
        reference_manifest=C.REFERENCE_MANIFEST, reference_owner=C.REFERENCE_OWNER,
        reference_inner=C.REFERENCE_INNER,
        reference_original_files_rehashed=reference['original_files_rehashed'],
        reference_repeat_rechecked=True, independent_own_histories=True,
        original_policy_bytes_preserved=True, temporal_equivalent_to_full=False,
        generated_id_and_decoded_byte_gate_passed=behavioral['passed'],
        unselected_native_payloads_recomputed=False, full_tensor_numerical_acceptance=False,
        cpu_and_elf_qualification_checked=False, external_model_rehashed=False,
        native_launch_admitted=False, native_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--pins', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 120))
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('120s data-only deadline')))
    signal.alarm(120)
    reads = []
    def read(root, name, maximum):
        raw = C.read_file(root, name, maximum)
        reads.append((root, name, maximum, C.compact(raw)))
        return raw
    pins = V.parse(read(args.pins.parent, args.pins.name, 8192))
    originals = {n: read(args.native, n, 8 << 20) for n in INPUT_NAMES}
    summary = V.parse(originals['summary.json'])
    logical = Path(summary['request']['base']['evidence_directory'])
    allowed = {'frames.ndjson', 'child-stderr.bin', 'generated-text.bin'} | {
        'capture-%d.bin' % p for p in V.CAPTURES}
    def native_read(pin):
        path = Path(pin['path'])
        V.require(path.parent == logical and path.name in allowed, 'closed native retained originals')
        return read(args.native / 'native', path.name, min(pin['bytes'], 32 << 20))
    report = admit(originals, pins, native_read, lambda n, m: read(args.reference, n, m))
    for root, name, maximum, pin in reads:
        C.checked(C.read_file(root, name, maximum), pin)
    report['input_posthashes_checked'] = len(reads)
    data = json.dumps(report, indent=2, sort_keys=True, allow_nan=False).encode() + b'\n'
    V.require(len(data) <= 256 << 10, 'bounded comparison report')
    with args.output.open('xb') as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
