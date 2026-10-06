"""Integer-only, independently implemented verifier for stable R2 observations."""
import copy
import hashlib
import json
from pathlib import Path
import re
import sys

IMAGE_SHA = 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'
TEST = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::native::native_stable_guarded_r2_v1'
MARKER = 'FERRIC_NATIVE_GUARDED_R2_V1='
WIDTH = 4096
ANCHORS = (
    (0, 0, 0, 0), (0x80000000, 0x80000000, 0x8000, 0),
    (0x3f800000, 0xbf800000, 0x8000, 0), (0x4b800000, 0xcb800000, 0x3f80, 0x3f80),
    (0x3f800000, 0x3b800000, 0xbf80, 0), (0x3f800000, 0x3c400000, 0xbf80, 0x3c80),
    (0xbf800000, 0xbb800000, 0x3f80, 0), (0xbf800000, 0xbc400000, 0x3f80, 0xbc80),
    (0x3f804000, 0xbf800000, 0, 0x3b00), (0x3b800000, 0x3b800000, 0x3f80, 0x3f81),
    (0x4b800000, 0x3f800000, 0xcb80, 0), (0x3f800000, 0, 0x3b80, 0x3f80),
    (0x3f810000, 0, 0x3b80, 0x3f82), (0xbf800000, 0, 0xbb80, 0xbf80),
    (0xbf810000, 0, 0xbb80, 0xbf82), (0x00800000, 0x80800000, 1, 1),
    (0x00008000, 0, 0, 0), (0x00008001, 0, 0, 1),
    (0x00018000, 0, 0, 2), (0x80008001, 0, 0, 0x8001),
    (0x80008000, 0, 0x8000, 0x8000), (0x7f7f0000, 0, 0, 0x7f7f),
    (0xff7f0000, 0, 0, 0xff7f), (0x7f7f0000, 0xff7f0000, 0x3f81, 0x3f81),
    (0x007f8000, 0, 0, 0x0080), (0x807f8000, 0, 0, 0x8080),
    (0x3f807fff, 0, 0, 0x3f80), (0x3f808001, 0, 0, 0x3f81),
    (0xbf807fff, 0, 0, 0xbf80), (0xbf808001, 0, 0, 0xbf81),
    (0x3f817fff, 0, 0, 0x3f81), (0x3f818001, 0, 0, 0x3f82),
)


def require(value, message):
    if not value:
        raise ValueError(message)


def strict_json(body):
    def object_pairs(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate JSON key: ' + key)
            result[key] = value
        return result
    return json.loads(body, object_pairs_hook=object_pairs,
                      parse_constant=lambda value: require(False, 'nonfinite JSON'))


def units32(bits):
    require(type(bits) is int and 0 <= bits < 1 << 32, 'FP32 bits')
    exponent, fraction = (bits >> 23) & 255, bits & 0x7fffff
    require(exponent != 255, 'nonfinite FP32')
    magnitude = fraction if exponent == 0 else ((1 << 23) | fraction) << (exponent - 1)
    return -magnitude if bits >> 31 else magnitude


def encode32(units, negative_zero=False):
    # Exact values use integer multiples of the smallest FP32 subnormal.
    if units == 0:
        return 0x80000000 if negative_zero else 0
    sign, magnitude = (0x80000000 if units < 0 else 0), abs(units)
    if magnitude < 1 << 23:
        return sign | magnitude
    shift = max(0, magnitude.bit_length() - 24)
    significand, remainder = divmod(magnitude, 1 << shift)
    if shift and (remainder > 1 << (shift - 1)
                  or (remainder == 1 << (shift - 1) and significand & 1)):
        significand += 1
    if significand == 1 << 24:
        significand >>= 1
        shift += 1
    exponent = shift + 1
    require(exponent < 255, 'FP32 overflow')
    return sign | exponent << 23 | (significand & 0x7fffff)


def add32(a, b):
    return encode32(units32(a) + units32(b), a == b == 0x80000000)


def narrow16(bits):
    units32(bits)
    high, low = bits >> 16, bits & 0xffff
    result = high + int(low > 0x8000 or (low == 0x8000 and high & 1))
    require(result & 0x7f80 != 0x7f80, 'BF16 overflow')
    return result


def expected_bits(a, b, residual):
    require(type(residual) is int and 0 <= residual < 1 << 16, 'BF16 bits')
    projection = narrow16(add32(add32(0, a), b))
    return narrow16(add32(projection << 16, residual << 16))


def corpus(ordinal):
    parts = [bytearray() for _ in range(4)]
    for i in range(WIDTH):
        if i % 64 < 32:
            a, b, residual, _ = ANCHORS[(i % 64 + 7 * (i // 64) + 11 * ordinal) % 32]
        else:
            a = encode32((((17 * i + 13 * ordinal) % 257) - 128) << 144)
            b = encode32((((29 * i + 7 * ordinal) % 251) - 125) << 143)
            residual = (0, 0x8000, 0x3f80, 0xbf80, 0x3f81, 0x0080)[i % 6]
        for part, value, width in zip(parts, (a, b, residual, expected_bits(a, b, residual)), (4, 4, 2, 2)):
            part.extend(value.to_bytes(width, 'little'))
    return tuple(bytes(part) for part in parts)


def digest(body):
    return hashlib.sha256(body).hexdigest()


def guarded(body):
    return (bytes((0xa5 + 29 * i) & 255 for i in range(64)) + body
            + bytes((0x5a + 17 * i) & 255 for i in range(64)))


def records():
    poison = b''.join((0x7fc0 | (1 + i % 63)).to_bytes(2, 'little') for i in range(WIDTH))
    expected = []
    for ordinal, generation in enumerate((1, 4294967299)):
        values = corpus(ordinal)
        valid_guard = [generation & 0xffffffff, generation >> 32, 1, 0]
        cases = [('valid', [valid_guard[:], valid_guard[:]], True)]
        for owner in range(2):
            for label, word, value in (('stale-low', 0, valid_guard[0] ^ 1),
                                       ('stale-high', 1, valid_guard[1] ^ 1),
                                       ('pending', 2, 0), ('failed', 2, 2), ('reserved', 3, 1)):
                guards = [valid_guard[:], valid_guard[:]]
                guards[owner][word] = value
                cases.append((f'rank{owner}-{label}', guards, False))
            for index in (0, 3, 4, 8, 14, 22, 31, 32, 547):
                guards = [valid_guard[:], valid_guard[:]]
                guards[owner][2] = 2
                cases.append((f'rank{owner}-prefix-{index}', guards, True))
        for case, guards, validators in cases:
            valid = case == 'valid'
            for rank in range(2):
                row = dict(case=case, generation=generation, rank=rank, valid=valid,
                           validators_executed=validators, guards=guards,
                           output_sha256=digest(guarded(values[3] if valid else poison)),
                           all_output_bytes_and_canaries_checked=True, non_target_output_unchanged=True)
                row.update(zip(('p0_le_hex', 'p1_le_hex', 'residual_le_hex', 'output_le_hex'),
                               (value.hex() if valid else None for value in values)))
                expected.append(row)
    return expected


def verify(observation, request_bytes):
    req = strict_json(request_bytes)
    require(set(req) == {'schema', 'device_unique_ids', 'image', 'image_sha256', 'timeout_ms'}
            and req['schema'] == 'ferric-native-stable-guarded-r2-request-v1'
            and req['image_sha256'] == IMAGE_SHA and type(req['timeout_ms']) is int
            and 1 <= req['timeout_ms'] <= 5000 and type(req['image']) is str
            and Path(req['image']).is_absolute(), 'closed request')
    ids = req['device_unique_ids']
    require(type(ids) is list and len(ids) == 2 and len(set(ids)) == 2
            and all(type(uid) is int and 0 < uid < 1 << 64 for uid in ids), 'exact device IDs')
    wanted = dict(schema='ferric-native-stable-guarded-r2-observation-v1',
                  request_sha256=digest(request_bytes), image_sha256=IMAGE_SHA,
                  device_unique_ids=ids, healthy_close=True, gpu_execution=True,
                  full_model_acceptance=False, production_authority=False, dispatches=192,
                  serial_host_ordered=True, paired_coordinator_tested=False,
                  owner_lifecycle_tested=False, performance_claim=False, records=records())
    # Canonical JSON equality also rejects bool-for-int substitutions and extra fields.
    require(json.dumps(observation, sort_keys=True, separators=(',', ':'), allow_nan=False)
            == json.dumps(wanted, sort_keys=True, separators=(',', ':'), allow_nan=False),
            'observation differs from independent full-bit/case/guard expectation')
    return dict(schema='ferric-native-stable-r2-integer-verification-v1', passed=True,
                arithmetic='integer multiples of 2^-149; staged FP32 and BF16 round-to-nearest-even',
                records=116, valid_records=4, invalid_records=112, validator_dispatches=76,
                r2_dispatches=116, total_dispatches=192, valid_output_elements_checked=16384,
                all_input_bits_reconstructed=True, all_output_bits_checked=True,
                performance_claim=False, full_model_acceptance=False)


def self_test():
    for a, b, r, expected in ANCHORS:
        require(expected_bits(a, b, r) == expected, 'fixed arithmetic anchor')
    bad = [(v, 0, 0) for v in (0x7f800000, 0xff800000, 0x7fc00001)]
    bad += [(0, v, 0) for v in (0x7f800000, 0xff800000, 0x7fc00001)]
    bad += [(0, 0, v) for v in (0x7f80, 0xff80, 0x7fc1)]
    bad += [(0x7f7fffff, 0, 0xff7f), (0x7f7fffff, 0x7f7fffff, 0),
            (0x7f7f0000, 0, 0x7f7f), (0x7f7f0000, 0, 0x7b00)]
    for args in bad:
        try:
            expected_bits(*args)
        except ValueError:
            continue
        raise ValueError('missing arithmetic refusal')
    require(add32(0x80000000, 0x80000000) == 0x80000000
            and add32(0, 0x80000000) == 0, 'signed zero addition')
    rows = records()
    require(len(rows) == 116 and sum(row['valid'] for row in rows) == 4
            and sum(row['validators_executed'] for row in rows) == 76, 'case census')
    req = dict(schema='ferric-native-stable-guarded-r2-request-v1', device_unique_ids=[11, 22],
               image='/fixture/image.hsaco', image_sha256=IMAGE_SHA, timeout_ms=5000)
    request_bytes = json.dumps(req, sort_keys=True).encode()
    fixture = dict(schema='ferric-native-stable-guarded-r2-observation-v1',
                   request_sha256=digest(request_bytes), image_sha256=IMAGE_SHA, device_unique_ids=[11, 22],
                   healthy_close=True, gpu_execution=True, full_model_acceptance=False,
                   production_authority=False, dispatches=192, serial_host_ordered=True,
                   paired_coordinator_tested=False, owner_lifecycle_tested=False,
                   performance_claim=False, records=rows)
    verify(fixture, request_bytes)
    mutations = (
        lambda v: v.update(extra=True),
        lambda v: v.pop('healthy_close'),
        lambda v: v['records'].pop(),
        lambda v: v['records'].reverse(),
        lambda v: v['records'][0].update(generation=True),
        lambda v: v['records'][0].update(generation=1.0),
        lambda v: v['records'][0].update(output_le_hex='00' * 8192),
        lambda v: v['records'][0].update(p0_le_hex='00' * 16384),
        lambda v: v['records'][0]['guards'][0].__setitem__(0, 2),
        lambda v: v['records'][0].update(validators_executed=False),
        lambda v: v['records'][2].update(output_sha256='0' * 64),
        lambda v: v['records'][0].update(non_target_output_unchanged=False),
        lambda v: v.update(device_unique_ids=[22, 11]),
        lambda v: v['records'][2].update(output_le_hex='00'),
    )
    for mutate in mutations:
        bad_fixture = copy.deepcopy(fixture)
        mutate(bad_fixture)
        try:
            verify(bad_fixture, request_bytes)
        except ValueError:
            continue
        raise ValueError('missing observation mutation refusal')
    for malformed in ('{"a":1,"a":2}', '{"x":NaN}'):
        try:
            strict_json(malformed)
        except ValueError:
            continue
        raise ValueError('missing strict JSON refusal')
    print(json.dumps(dict(schema='ferric-native-stable-r2-integer-selftest-v1', passed=True,
                          fixed_arithmetic_anchors=32, arithmetic_refusals=13,
                          signed_zero_checks=2, case_records=116, positive_observation_fixtures=1,
                          observation_mutation_refusals=14, json_refusals=2, gpu_execution=False)))


def main():
    if sys.argv[1:] == ['--self-test']:
        self_test()
        return
    require(len(sys.argv) == 3, 'verify_native.py STDOUT REQUEST')
    stdout, request = Path(sys.argv[1]), Path(sys.argv[2])
    require(stdout.stat().st_size <= 4 << 20 and request.stat().st_size <= 4096, 'bounded verifier inputs')
    body, request_bytes = stdout.read_bytes(), request.read_bytes()
    lines = body.decode().splitlines()
    require([line for line in lines if line.startswith('running ')] == ['running 1 test']
            and [line for line in lines if line.startswith('test ') and not line.startswith('test result:')]
            == ['test ' + TEST + ' ... ok'], 'exact selected native test')
    matches = [line for line in lines if MARKER in line]
    require(len(matches) == 1 and matches[0].startswith(MARKER), 'exact native observation marker')
    summaries = re.findall(r'^test result: (.*)$', body.decode(), re.M)
    require(len(summaries) == 1 and re.fullmatch(
        r'ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 995 filtered out; finished in [0-9.]+s',
        summaries[0]), 'exact isolated ignored-test outcome')
    result = verify(strict_json(matches[0][len(MARKER):]), request_bytes)
    result.update(stdout_sha256=digest(body), request_sha256=digest(request_bytes))
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
