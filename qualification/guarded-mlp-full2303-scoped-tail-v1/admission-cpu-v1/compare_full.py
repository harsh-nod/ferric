"""Pure Full2303 behavior gate over pinned originals; no model or GPU execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import resource
import signal
import stat
import struct

import full_announcement as A
import full_reference as R
import validate_full as V

REFERENCE_MANIFEST = dict(bytes=22493, sha256='5a0564e78e6b4ab769af2c56eb7865e36fca4afc18bda7196ce1321b47ea3233')
REFERENCE_OWNER = dict(bytes=4910, sha256='e862ba7fc32df915c22867063f5cba48e9f776fbf36d2639cba786043bfe685d')
REFERENCE_INNER = dict(bytes=572152, sha256='876678fa2815a109546c2ff01db33fe80c2da66d9d00e0d8bb53e108ed2ed006')
INPUT_NAMES = ('summary.json', 'request.json', 'parent-result.json', 'parent-started.json', 'parent-command.json', 'parent-stderr')
CAP = 64 << 20


def compact(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def checked(raw, pin, maximum=CAP):
    V.keys(pin, 'bytes sha256')
    V.require(type(raw) is bytes and V.uint(pin['bytes'], maximum) == len(raw)
              and type(pin['sha256']) is str and compact(raw) == pin, 'original compact body pin')
    return raw


def reference_data(bodies):
    """Called after fixed-manifest authentication; repeated pure history/payload checks."""
    owner = V.parse(bodies['complete.json'])
    inner = V.parse(bodies['output/complete.json'])
    V.require(owner['passed'] is True and owner['errors'] == owner['postcheck_errors'] == []
              and owner['container_removed'] is True and type(owner['framework_attempts']) is int
              and owner['framework_attempts'] == 1 and type(owner['retries']) is int and owner['retries'] == 0,
              'original independent reference owner success')
    V.require(inner['schema'] == 'ferric-full2303-framework-reference-v1' and inner['passed'] is True
              and inner['error'] is None and inner['postcheck_errors'] == []
              and inner['repeat_gate_passed'] is True and inner['framework_execution'] is True
              and inner['native_execution'] is False and inner['native_intermediates_used'] is False
              and inner['candidate_receipt'] is None and inner['decoded_special_token_policy'] == 'skip'
              and inner['selected_positions'] == list(V.CAPTURES)
              and all(inner[k] is False for k in ('numerical_acceptance', 'performance_claim',
                  'production_authority', 'full_model_acceptance')),
              'independent reference policy/no candidate inputs')
    tokens_raw = bodies['inputs/prompt.u32le']
    V.require(len(tokens_raw) == 8192, 'authentic prompt token body')
    tokens = list(struct.unpack('<2048I', tokens_raw))
    manifest = V.parse(bodies['inputs/prompt-manifest.json'])
    V.require(V.same(tokens, manifest['input_token_ids']) and V.same(tokens, inner['full_prompt_tokens'])
              and V.same(tokens, inner['input_tokens']), 'same complete authentic prompt')
    passes = [V.parse(bodies['output/pass%d.json' % p]) for p in (1, 2)]
    V.require(V.same(passes, inner['passes']), 'original pass documents/terminal')
    payloads = [{p: bodies['output/pass%d-pos%d.bf16' % (i, p)] for p in V.CAPTURES} for i in (1, 2)]
    outputs = [{n: bodies['output/pass%d.%s' % (i, n)] for n in
                ('tokens.u32le', 'decoded.bin', 'decoded-preserve-special.bin')} for i in (1, 2)]
    V.require(R.repeat_gate(passes, tokens, payloads, outputs), 'two exact genuine own-history reference passes')
    R.evidence_bound(passes, payloads, outputs)
    return dict(owner=owner, inner=inner, tokens=tokens, passes=passes, outputs=outputs,
                prompt_pins={n: compact(bodies['inputs/' + p]) for n, p in
                    [('manifest', 'prompt-manifest.json'), ('text', 'prompt.txt'), ('tokens', 'prompt.u32le')]})


def authenticate_reference(read):
    raw = checked(read('manifest.json', 1 << 20), REFERENCE_MANIFEST, 1 << 20)
    manifest = V.parse(raw)
    pins = manifest['files']
    V.require(type(pins) is dict and len(pins) == 130, 'fixed reference original roster')
    bodies = {}; total = 0
    for name, pin in sorted(pins.items()):
        path = PurePosixPath(name)
        V.require(type(name) is str and not path.is_absolute() and '..' not in path.parts
                  and str(path) == name, 'reference member path')
        total += V.uint(pin['bytes'], 8 << 20)
        V.require(total <= CAP, 'reference aggregate bound')
        bodies[name] = checked(read(name, 8 << 20), pin, 8 << 20)
    checked(bodies['complete.json'], REFERENCE_OWNER)
    checked(bodies['output/complete.json'], REFERENCE_INNER)
    result = reference_data(bodies)
    result['original_files_rehashed'] = len(bodies)
    return result


def compare_choices(native, decoded, reference_tokens, reference_decoded):
    """No tie exemption, text normalization, or teacher-forced substitution."""
    for values in (native, reference_tokens):
        V.require(type(values) is list and len(values) == 256
                  and all(type(t) is int and 0 <= t < 151936 for t in values), 'exact256 finite vocabulary IDs')
    V.require(type(decoded) is bytes and type(reference_decoded) is bytes
              and len(decoded) <= 128 << 10 and len(reference_decoded) <= 128 << 10, 'raw decoded byte bounds')
    mismatch = [dict(index=i, position=2047 + i, native=a, reference=b)
                for i, (a, b) in enumerate(zip(native, reference_tokens)) if a != b]
    byte_equal = decoded == reference_decoded
    first_byte = next((i for i in range(min(len(decoded), len(reference_decoded)))
                       if decoded[i] != reference_decoded[i]), None)
    if first_byte is None and not byte_equal:
        first_byte = min(len(decoded), len(reference_decoded))
    return dict(passed=not mismatch and byte_equal, generated_ids_equal=not mismatch,
                generated_ids_checked=256, generated_id_mismatches=mismatch,
                first_generated_id_mismatch=mismatch[0] if mismatch else None,
                decoded_bytes_equal=byte_equal, first_decoded_byte_mismatch=first_byte,
                native_decoded=compact(decoded), reference_decoded=compact(reference_decoded),
                decoded_special_token_policy='skip', reference_tie_exemption=False,
                full_tensor_numerical_acceptance=False, numerical_acceptance=False,
                performance_claim=False, production_authority=False)


def admit(native_bodies, pins, native_read, reference_read):
    """Native admission pins are supplied from an observed supervised attempt, never guessed."""
    V.require(set(native_bodies) == set(pins) == set(INPUT_NAMES), 'six observed native admission originals')
    for name in INPUT_NAMES:
        maximum = 128 << 10 if name == 'summary.json' else 65536 if name == 'request.json' else 8 << 20
        checked(native_bodies[name], pins[name], maximum)
    reference = authenticate_reference(reference_read)
    request = V.parse(native_bodies['request.json'])
    base = request['base']
    for name, pin in reference['prompt_pins'].items():
        actual = V.rust_pin(base['prompt'][name])
        V.require({k: actual[k] for k in ('bytes', 'sha256')} == pin, 'native authentic reference prompt pin')
    V.require(V.octets(base['expected_model_id']).hex() == reference['inner']['model_id']
              and V.octets(base['expected_bundle_id']).hex() == reference['inner']['bundle_id'],
              'same actual target model and bundle')
    directory = Path(base['evidence_directory'])
    native = V.validate(native_bodies['summary.json'], request, directory, native_read, reference['tokens'])
    result = V.parse(native_bodies['parent-result.json']); started = V.parse(native_bodies['parent-started.json'])
    for role, name in [('stderr', 'parent-stderr'), ('stdout', 'summary.json'),
                       ('started', 'parent-started.json'), ('command', 'parent-command.json')]:
        V.require(result[role] == dict(path=result[role]['path'], **compact(native_bodies[name])),
                  'original owned parent raw body join')
    command = V.parse(native_bodies['parent-command.json'])
    argv = command['argv']
    V.require(type(argv) is list and len(argv) == 5 and all(type(v) is str for v in argv)
              and Path(argv[0]).name == 'ferric-qwen3-guarded-mlp-full2303-engineering'
              and Path(argv[0]).is_absolute() and Path(argv[2]).is_absolute()
              and argv[1] == '--request' and argv[3:] ==
                  ['--observe-guarded-full2303', '--allow-unauthenticated-machine-code']
              and command['gpu_execution_requested'] is True and result['gpu_execution_requested'] is True,
              'actual explicit Full2303 parent command')
    custody = A.validate_lineage(native_bodies['parent-stderr'], result, started, native['child_pid'])
    native['process_retirement_independently_checked'] = True
    summary = V.parse(native_bodies['summary.json'])
    decoded_pin = V.rust_pin(summary['files']['decoded_output'])
    decoded = native_read(decoded_pin)
    checked(decoded, {k: decoded_pin[k] for k in ('bytes', 'sha256')}, 128 << 10)
    behavioral = compare_choices(native['generated_tokens'], decoded,
        reference['passes'][0]['generated_tokens'], reference['outputs'][0]['decoded.bin'])
    return dict(schema='ferric-full2303-native-behavior-admission-v1', passed=behavioral['passed'],
                native=native, custody=custody, comparison=behavioral,
                native_admission_pins=pins, reference_manifest=REFERENCE_MANIFEST,
                reference_owner=REFERENCE_OWNER, reference_inner=REFERENCE_INNER,
                reference_original_files_rehashed=reference['original_files_rehashed'],
                reference_repeat_rechecked=True, independent_own_histories=True,
                generated_id_and_decoded_byte_gate_passed=behavioral['passed'],
                unselected_native_payloads_recomputed=False, full_tensor_numerical_acceptance=False,
                cpu_and_elf_qualification_checked=False, external_model_rehashed=False,
                native_launch_admitted=False, native_execution=False, numerical_acceptance=False,
                performance_claim=False, production_authority=False)


def read_file(root, name, maximum):
    relative = PurePosixPath(name)
    V.require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == name, 'relative data path')
    path = root / name
    for parent in (path, *path.parents):
        V.require(not parent.is_symlink(), 'no symlink data component')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        before = os.fstat(fd)
        V.require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= maximum, 'bounded regular original')
        with os.fdopen(fd, 'rb', closefd=False) as stream:
            raw = stream.read(maximum + 1)
        after = os.fstat(fd)
        V.require(len(raw) == before.st_size and (before.st_dev, before.st_ino, before.st_size,
                  before.st_mtime_ns, before.st_ctime_ns) == (after.st_dev, after.st_ino, after.st_size,
                  after.st_mtime_ns, after.st_ctime_ns), 'stable original read')
        return raw
    finally:
        os.close(fd)


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
        raw = read_file(root, name, maximum); reads.append((root, name, maximum, compact(raw)))
        return raw
    pins = V.parse(read(args.pins.parent, args.pins.name, 8192))
    originals = {n: read(args.native, n, 8 << 20) for n in INPUT_NAMES}
    summary = V.parse(originals['summary.json'])
    logical = Path(summary['request']['base']['evidence_directory'])
    allowed = {'frames.ndjson', 'child-stderr.bin', 'generated-text.bin'} | {'capture-%d.bin' % p for p in V.CAPTURES}
    def native_read(pin):
        path = Path(pin['path'])
        V.require(path.parent == logical and path.name in allowed, 'closed native retained originals')
        return read(args.native / 'native', path.name, min(pin['bytes'], 32 << 20))
    report = admit(originals, pins, native_read, lambda n, m: read(args.reference, n, m))
    for root, name, maximum, pin in reads:
        checked(read_file(root, name, maximum), pin)
    report['input_posthashes_checked'] = len(reads)
    data = json.dumps(report, indent=2, sort_keys=True, allow_nan=False).encode() + b'\n'
    V.require(len(data) <= 256 << 10, 'bounded comparison report')
    with args.output.open('xb') as output:
        output.write(data); output.flush(); os.fsync(output.fileno())
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
