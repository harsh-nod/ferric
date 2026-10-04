"""Data-only comparison of actual AR4 rotary inputs with retained CPU RoPE tables."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
INPUTS = E / 'projection-ar4-framework-inputs-v228-v1'
ROPE = E / 'rope-framework-reference-v228-v1/complete.json'
REFERENCE = E / 'projection-ar4-framework-reference-v228-v1/reference.json'
NATIVE = INPUTS / 'native-case/complete.json'
FIXED = {
    str(ROPE): (330074, 'f5c47aabf7a307a19f9fe54fa37187f8548ceab4ac92eeeac2638fc9cc4589b4'),
    str(REFERENCE): (201239, '00952244362ad51d241d179f741ae5ae61ff8acfcb3fd160b9dc64ce3b5f699e'),
    str(NATIVE): (1054985, '15938580d218f855883a589c819d532bbf941a02c4677cb29928f7bf7106d1cb'),
    str(INPUTS / 'native/request-0.json'): (1865, 'dfdfef97a872fcb7622998489f219b61cff5a9d4dbe4c27c1955b7ab0c6066ce'),
    str(INPUTS / 'native/request-1.json'): (2400, '04cd581b9b4c7b75f1bb988e0f6c9f8e5bff828b412347ebbdfad9d665429c00'),
    str(INPUTS / 'native/request-2.json'): (2404, 'bba7e200de72e40866022407e2781769a5c48b75b0021e0947bdf850386e75c4'),
    str(INPUTS / 'native/request-3.json'): (2405, '0acf40630457e988af9f6e003def15f8f5c3562756616deaa03cac0964b3533e'),
}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def extent(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, expected):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical retained path')
    before = path.stat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 2 << 20,
            'bounded ordinary retained body')
    raw = path.read_bytes()
    after = path.stat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) and extent(raw) == dict(bytes=expected[0], sha256=expected[1]),
            'exact unchanged input body')
    return raw


def finite_word(value, width):
    require(type(value) is int and 0 <= value < 1 << width, 'exact unsigned word, not bool')
    mask = 0x7f800000 if width == 32 else 0x7f80
    require(value & mask != mask, 'finite coefficient')
    return value


def bf16_rne(bits):
    bits = finite_word(bits, 32)
    # Integer ties-to-even narrowing, retaining the sign bit including negative zero.
    return finite_word((bits + 0x7fff + ((bits >> 16) & 1)) >> 16, 16)


def vector(value, width, length):
    require(type(value) is list and len(value) == length, 'coefficient vector extent')
    return [finite_word(word, width) for word in value]


def compare(tables, requests):
    require(type(tables) is list and type(requests) is list and len(requests) == 4, 'four actual requests')
    selected = {row['position']: row for row in tables if type(row['position']) is int and row['position'] in range(4)}
    require(set(selected) == set(range(4)) and sum(row['position'] in range(4) for row in tables) == 4,
            'exactly one retained table per selected position')
    result = []
    for position, request in enumerate(requests):
        command = request['command']
        require(request['protocol'] == 1 and type(request['id']) is int and request['id'] == position + 1
                and command['op'] == 'forward' and type(command['generation']) is int
                and command['generation'] == position + 1 and type(command['cache_metadata'][0]) is int
                and command['cache_metadata'][0] == position, 'actual request position/generation')
        actual = vector(command['rotary_bits'], 32, 128)
        table = selected[position]
        values = {}
        for role, width in (('framework_cos_bf16_bits', 16), ('framework_sin_bf16_bits', 16),
                            ('f64_model_cos_f32_bits', 32), ('f64_model_sin_f32_bits', 32),
                            ('framework_companion_cos_f32_bits', 32), ('framework_companion_sin_f32_bits', 32)):
            values[role] = vector(table[role], width, 128)
            require(values[role][:64] == values[role][64:], 'framework split-half coefficient duplication')
        expected = values['framework_cos_bf16_bits'][:64] + values['framework_sin_bf16_bits'][:64]
        model = values['f64_model_cos_f32_bits'][:64] + values['f64_model_sin_f32_bits'][:64]
        companion = values['framework_companion_cos_f32_bits'][:64] + values['framework_companion_sin_f32_bits'][:64]
        rounded = [bf16_rne(word) for word in actual]
        different = [i for i in range(128) if rounded[i] != expected[i]]
        result.append(dict(position=position, coefficient_words=128, bf16_exact_words=128 - len(different),
            bf16_different_indices=different,
            raw_f32_equal_to_retained_libm_model=sum(a == b for a, b in zip(actual, model)),
            raw_f32_equal_to_framework_companion=sum(a == b for a, b in zip(actual, companion))))
    return result


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary Python -B')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('label')
    parser.add_argument('source_sha256')
    parser.add_argument('test_sha256')
    args = parser.parse_args()
    require(re.fullmatch(r'ar4-rope-input-audit-v228-v[1-9][0-9]*', args.label)
            and all(re.fullmatch('[0-9a-f]{64}', x) for x in (args.source_sha256, args.test_sha256)), 'fresh label/source pins')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
            and all(os.environ.get(name) == '' for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES',
                                                         'CUDA_VISIBLE_DEVICES')), 'CPU-only owned ASROCK bounds')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        resource.setrlimit(kind, (min(cap, hard) if hard != resource.RLIM_INFINITY else cap,) * 2)
    folder = Path(__file__).resolve().parent
    sources = {str(folder / name): (folder.joinpath(name).stat().st_size, digest)
               for name, digest in (('run.py', args.source_sha256), ('test_run.py', args.test_sha256))}
    checked = {path: read(path, expected) for path, expected in {**FIXED, **sources}.items()}
    rope, reference, native = [parse(checked[str(path)]) for path in (ROPE, REFERENCE, NATIVE)]
    require(rope['schema'] == 'ferric-p228-rope-framework-reference-v1' and rope['completed'] is True
            and rope['theta'] == 1000000 and rope['head_dim'] == 128 and rope['gpu_execution'] is False
            and reference['schema'] == 'ferric-p228-projection-ar4-framework-reference-v1'
            and reference['status'] == 'PASS' and reference['native_complete']['sha256'] == FIXED[str(NATIVE)][1]
            and reference['native_checked'] == native['checked'] and native['passed'] is True
            and native['failures'] == [], 'authenticated prior reference/native metadata')
    requests, pins = [], []
    for position in range(4):
        path = INPUTS / f'native/request-{position}.json'
        pin = dict(path=str(path), **extent(checked[str(path)]))
        original = native['retained_native'][f'request-{position}.json']
        require(dict(original=original, retained=pin) in reference['native_consumed'], 'original/transport request join')
        requests.append(parse(checked[str(path)]))
        pins.append(dict(original=original, retained=pin))
    rows = compare(rope['tables'], requests)
    for position, request in enumerate(requests):
        require(request['command']['token'] == native['checked']['input_tokens'][position], 'actual native token metadata')
    result = dict(schema='ferric-p228-ar4-rope-input-audit-v1', completed=True,
        coefficient_words=512, bf16_exact_words=sum(row['bf16_exact_words'] for row in rows),
        all_bf16_coefficients_equal=all(row['bf16_exact_words'] == 128 for row in rows), rows=rows,
        requests=pins, inputs=[dict(path=path, **extent(raw)) for path, raw in checked.items()],
        source_postchecks_passed=True, arithmetic='integer FP32 to BF16 round-to-nearest-even only',
        positions=[0, 1, 2, 3], gpu_execution=False, model_loaded=False, tests_executed=False,
        native_or_framework_lifecycle_replayed=False, raw_f32_table_equality_required=False,
        actual_gpu_trigonometry_validated=False, actual_long_position_tables_validated=False,
        new_rope_image_validated=False, numerical_acceptance=False, full_model_correctness=False,
        production_authority=False, performance_claim=False, sustained_2048_256=False,
        caveat='Position-zero residual differences remain; four-position coefficient equality does not identify every error source.')
    for path, raw in checked.items():
        require(read(path, (len(raw), extent(raw)['sha256'])) == raw, 'all input/source postchecks')
    out = E / args.label
    require(not os.path.lexists(out), 'fresh result directory')
    out.mkdir(mode=0o700)
    raw = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    with (out / 'complete.json').open('xb') as stream:
        stream.write(raw)
    require(read(out / 'complete.json', (len(raw), extent(raw)['sha256'])) == raw, 'actual written result')
    print(json.dumps(dict(complete=dict(path=str(out / 'complete.json'), **extent(raw)),
                         bf16_exact_words=result['bf16_exact_words'])), flush=True)


if __name__ == '__main__':
    main()
