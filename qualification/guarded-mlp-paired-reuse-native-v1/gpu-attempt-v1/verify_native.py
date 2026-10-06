"""Independent exact-dyadic reference for two consecutive paired generations."""
import copy
import functools
import hashlib
import json
from pathlib import Path
import re
import sys

TEST = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::native::paired::reuse::native_paired_guarded_mlp_reuse_v1'
HASHES = ['25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25',
          'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589',
          'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66']
WIDTH, INNER = 4096, 6144
MARKER = b'FERRIC_NATIVE_PAIRED_REUSE_MLP_V1='


def require(value, why):
    if not value:
        raise ValueError(why)


def strict(body):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def constant(value):
        raise ValueError('nonfinite JSON: ' + value)
    return json.loads(body, object_pairs_hook=pairs, parse_constant=constant)


def sha(body):
    return hashlib.sha256(body).hexdigest()


def integer(value, low, high):
    return type(value) is int and low <= value <= high


def dyadic(units, fraction_bits, mantissa_bits, byte_count):
    """Encode an exactly representable signed dyadic, with no host float."""
    require(type(units) is int, 'integer dyadic')
    if units == 0:
        return bytes(byte_count)
    magnitude = abs(units)
    top = magnitude.bit_length() - 1
    exponent = top - fraction_bits + 127
    require(1 <= exponent <= 254 and top <= mantissa_bits, 'normal exact dyadic range')
    significand = magnitude << (mantissa_bits - top)
    bits = ((units < 0) << (mantissa_bits + 8)) | (exponent << mantissa_bits)
    bits |= significand - (1 << mantissa_bits)
    return bits.to_bytes(byte_count, 'little')


def b16(units, fraction_bits=0):
    return dyadic(units, fraction_bits, 7, 2)


def f32(units, fraction_bits=0):
    return dyadic(units, fraction_bits, 23, 4)


def sign(rank, column):
    return -1 if (13 * column + column // 64 + 7 * rank) % 5 < 2 else 1


def gate(rank, row):
    return 8 << ((row + rank) % 2)


def up8(rank, row, generation):
    magnitude = 1 << ((row // 3 + rank) % 2)
    return generation * (-magnitude if (5 * row + rank) % 3 == 0 else magnitude)


def activation(rank, row, generation):
    return gate(rank, row) * up8(rank, row, generation) // 8


def down_terms(rank, row):
    columns = [(13 * row + 37 * rank + 2049 * i) % INNER for i in range(3)]
    require(len(set(columns)) == 3, 'three distinct sparse columns')
    weights8 = [4 if (row + rank) % 2 == 0 else -4, 1, -2]
    return list(zip(columns, weights8))


def down8(rank, row, generation):
    return sum(activation(rank, column, generation) * weight for column, weight in down_terms(rank, row))


def matrix_sha(rank, role, generation):
    """Hash the full dense representation while retaining only one row."""
    digest = hashlib.sha256()
    width, rows = (INNER, WIDTH) if role == 2 else (WIDTH, INNER)
    for row in range(rows):
        body = bytearray(width * 2)
        if role == 2:
            values = [(column, b16(weight, 3)) for column, weight in down_terms(rank, row)]
        elif role == 0:
            column = (17 * row + 29 * rank) % WIDTH
            values = [(column, b16(gate(rank, row) * sign(rank, column)))]
        else:
            column = (31 * row + 43 * rank + 11) % WIDTH
            values = [(column, b16(up8(rank, row, generation) * sign(rank, column), 3))]
        for column, value in values:
            body[column * 2:column * 2 + 2] = value
        digest.update(body)
    return digest.hexdigest()


@functools.lru_cache(maxsize=2)
def reference(generation):
    require(type(generation) is int and generation in (1, 2), 'closed reference generations')
    # R1 produces exactly +/-1; RMSNorm with epsilon 1e-6 rounds back to
    # BF16 +/-1. For Gate=8/16, exp(Gate)>512 (already true for the first six
    # Taylor terms at8), so Gate*512/513 < SiLU(Gate) < Gate. This interval
    # is strictly inside Gate's BF16 rounding cell. The selected image
    # materializes that BF16 SiLU before multiplying Up.
    rows = []
    for rank in range(2):
        partial = b''.join(f32((i % 7 - 3) if rank == 0 else (4 - i % 7), 3) for i in range(WIDTH))
        residual = b''.join(b16(8 * sign(rank, i) - 1, 3) for i in range(WIDTH))
        stages = []
        for index in range(10):
            if index in (2, 3, 4):
                stages.append(dict(root=index, bytes=50331648, sha256=matrix_sha(rank, index - 2, generation), le_hex=None))
                continue
            if index in (0, 5):
                body = b''.join(b16(sign(rank, i)) for i in range(WIDTH))
            elif index == 1:
                body = b16(1) * WIDTH
            elif index == 6:
                body = b''.join(b16(gate(rank, i)) for i in range(INNER))
            elif index == 7:
                body = b''.join(b16(up8(rank, i, generation), 3) for i in range(INNER))
            elif index == 8:
                body = b''.join(b16(activation(rank, i, generation)) for i in range(INNER))
            else:
                body = b''.join(f32(down8(rank, i, generation), 3) for i in range(WIDTH))
            stages.append(dict(root=index, bytes=len(body), sha256=sha(body), le_hex=body.hex()))
        output = b''.join(b16(down8(0, i, generation) + down8(1, i, generation) + 8 * sign(rank, i), 3) for i in range(WIDTH))
        rows.append(dict(rank=rank, stages=stages, partial_le_hex=partial.hex(), partial_sha256=sha(partial),
                         residual_le_hex=residual.hex(), residual_sha256=sha(residual),
                         output_le_hex=output.hex(), output_sha256=sha(output)))
    return rows


def terminal(prefix, guard, generation):
    require(type(prefix) is list and len(prefix) == 548 and all(integer(v, 0, 0xffffffff) for v in prefix),
            'genuine terminal prefix words')
    fixed = [1, 0, 0, 31] + [1, 96, 96, 1, 64] * 2 + [0xffffffff] * 8 + [3] + [0xffffffff] * 8 + [3]
    require(prefix[:32] == fixed and all(1 <= v <= 64 for v in prefix[32:290])
            and prefix[290:] == [64] * 258, 'terminal task claims/arrivals')
    require(guard == [generation, 0, 1, 0] and all(type(v) is int for v in guard), 'current Valid guard')


def verify(value, request, request_sha):
    expected_flags = dict(kernel_dispatches=16, barrier_packets=4, completion_signals=20,
        paired_coordinator_tested=True, owner_lifecycle_tested=True, rearm_tested=True,
        owners_reused=True, payloads_reused=True, fresh_arenas_per_generation=True,
        fixture='nonzero-sparse-rows-double-up-v1', performance_claim=False, canaries_tested=False,
        schema='ferric-native-paired-guarded-mlp-reuse-observation-v1', request_sha256=request_sha,
        image_sha256=HASHES, device_unique_ids=request['device_unique_ids'], healthy_close=True,
        gpu_execution=True, full_model_acceptance=False, production_authority=False)
    require(set(value) == set(expected_flags) | {'generations'},
            'closed paired observation')
    for key, expected in expected_flags.items():
        require(type(value[key]) is type(expected) and value[key] == expected, 'annotation: ' + key)
    require(all(integer(v, 1, 2**64 - 1) for v in value['device_unique_ids']), 'integer observed device identities')
    generations = value['generations']
    require(type(generations) is list and len(generations) == 2, 'exactly two generations')
    previous_reads = [0, 0]
    for generation, observed in enumerate(generations, 1):
        require(set(observed) == {'records', 'generation', 'observed_queue_frontiers', 'segment_host_ns'},
                'closed generation record')
        require(type(observed['generation']) is int and observed['generation'] == generation,
                'ordered consecutive generations')
        require(integer(observed['segment_host_ns'], 0, request['timeout_ms'] * 1000000 - 1), 'bounded host segment interval')
        frontiers = observed['observed_queue_frontiers']
        require(type(frontiers) is list and len(frontiers) == 2, 'two queue observations')
        for rank, frontier in enumerate(frontiers):
            require(type(frontier) is list and len(frontier) == 2
                    and all(integer(v, 0, 2**64 - 1) for v in frontier)
                    and frontier[0] == 5 * generation
                    and previous_reads[rank] <= frontier[1] <= frontier[0], 'actual monotone queue frontiers')
            previous_reads[rank] = frontier[1]
        require(type(observed['records']) is list and len(observed['records']) == 2, 'two rank records')
        for actual, expected in zip(observed['records'], reference(generation)):
            require(set(actual) == set(expected) | {'prefix', 'guard'}, 'closed rank record')
            terminal(actual['prefix'], actual['guard'], generation)
            require(type(actual['rank']) is int, 'integer rank')
            for key, want in expected.items():
                require(actual[key] == want, 'exact reconstructed rank field: ' + key)
            for stage in actual['stages']:
                require(set(stage) == {'root', 'bytes', 'sha256', 'le_hex'}
                        and type(stage['root']) is int and type(stage['bytes']) is int, 'closed typed stage')
    for rank in range(2):
        first, second = (g['records'][rank] for g in generations)
        for index in (0, 1, 2, 4, 5, 6):
            require(first['stages'][index] == second['stages'][index], 'unchanged input/Gate/Down matrix')
        require(first['stages'][3]['sha256'] != second['stages'][3]['sha256'], 'changed Up matrix')
        a, b = (bytes.fromhex(row['output_le_hex']) for row in (first, second))
        require(all(a[i:i+2] != b[i:i+2] for i in range(0, len(a), 2)), 'every final word changed')
        require(sum(b[i:i+2] == b'\0\0' for i in range(0, len(b), 2)) == (820, 819)[rank],
                'exact positive-zero cancellation count')
    return dict(passed=True, computed_elements_checked=139264, final_output_elements_checked=16384,
                dense_matrix_hashes_checked=12, distinct_dense_matrix_hashes=8,
                terminal_owner_observations_checked=4, generations_checked=2)


def fixture_request():
    return dict(schema='ferric-native-paired-guarded-mlp-request-v1', device_unique_ids=[11, 22],
                images=['/tmp/r1', '/tmp/mlp', '/tmp/guard'], image_sha256=HASHES, timeout_ms=5000)


def synthetic():
    req = fixture_request()
    value = dict(kernel_dispatches=16, barrier_packets=4, completion_signals=20,
        paired_coordinator_tested=True, owner_lifecycle_tested=True, rearm_tested=True,
        owners_reused=True, payloads_reused=True, fresh_arenas_per_generation=True,
        fixture='nonzero-sparse-rows-double-up-v1', performance_claim=False, canaries_tested=False,
        schema='ferric-native-paired-guarded-mlp-reuse-observation-v1', request_sha256='test',
        image_sha256=HASHES, device_unique_ids=req['device_unique_ids'], healthy_close=True,
        gpu_execution=True, full_model_acceptance=False, production_authority=False, generations=[])
    prefix = [1, 0, 0, 31] + [1, 96, 96, 1, 64] * 2 + [0xffffffff] * 8 + [3] + [0xffffffff] * 8 + [3] + [64] * 516
    for generation in (1, 2):
        records = copy.deepcopy(reference(generation))
        for row in records:
            row.update(prefix=prefix.copy(), guard=[generation, 0, 1, 0])
        value['generations'].append(dict(generation=generation, records=records,
            segment_host_ns=1, observed_queue_frontiers=[[5*generation, 5*generation-2]]*2))
    return req, value


def test_census(text):
    summaries = re.findall(r'^test result:.*$', text, re.M)
    require(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M) == [(TEST, 'ok')]
            and len(summaries) == 1
            and re.fullmatch(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; '
                             r'1026 filtered out; finished in [0-9]+(?:\.[0-9]+)?s', summaries[0]),
            'one selected paired native test passed')


def selftest():
    require(b16(1).hex() == '803f' and b16(-1).hex() == '80bf' and f32(1, 3).hex() == '0000003e', 'dyadic anchors')
    req, good = synthetic()
    verify(good, req, 'test')
    require([r['output_sha256'] for g in (1, 2) for r in reference(g)] == [
        '6e6ecb4b75e6e5f09a212bbd4d4779fc2975c026f4398c2241230843cf4a33bd',
        '9fde0e89ff2a46d8b366a4adc5dbe4bb1728106112b2b8fa563e4af6d944d3bd',
        '403903da99ade382e932312fb9fcdaa5d5fe3e41bf05768f614667fd77b160bf',
        'e7164a702f121bbaa1b9e940960177f2b7670bf279f641e6e827be6bdfba3e93'], 'independent output hash anchors')
    require([r['stages'][3]['sha256'] for r in reference(2)] == [
        '4d8d301047aab087afdf5c54302051fd4848fac603ac85605688d3b1e1a47da5',
        '1c1d077694c7f6a826ff26c6a770ee9887cb9c376d6f65193545df3180583dd9'], 'independent Up matrix anchors')
    mutations = []
    for g in range(2):
        for rank in range(2):
            for index in range(10):
                value = copy.deepcopy(good)
                value['generations'][g]['records'][rank]['stages'][index]['sha256'] = '0' * 64
                mutations.append(value)
            for field in ('partial_le_hex', 'residual_le_hex', 'output_le_hex'):
                value = copy.deepcopy(good); value['generations'][g]['records'][rank][field] = '00'; mutations.append(value)
            for field, index in (('prefix', 0), ('prefix', 32), ('prefix', 547), ('guard', 0), ('guard', 2)):
                value = copy.deepcopy(good); value['generations'][g]['records'][rank][field][index] = 0; mutations.append(value)
    for field, changed in [('kernel_dispatches', 18), ('performance_claim', True), ('rearm_tested', False),
                           ('owners_reused', False), ('payloads_reused', False), ('fresh_arenas_per_generation', False)]:
        value = copy.deepcopy(good); value[field] = changed; mutations.append(value)
    for g in range(2):
        for field, changed in [('observed_queue_frontiers', [[5*(g+1), 5*(g+1)+1], [5*(g+1), 0]]),
                               ('segment_host_ns', 5000000000), ('generation', g+2)]:
            value = copy.deepcopy(good); value['generations'][g][field] = changed; mutations.append(value)
    for generations in (good['generations'][:1], list(reversed(good['generations'])),
                        [good['generations'][0]]*2, good['generations']+[good['generations'][1]]):
        value = copy.deepcopy(good); value['generations'] = copy.deepcopy(generations); mutations.append(value)
    for rank in range(2):
        for field in ('stages', 'output_le_hex', 'guard'):
            value = copy.deepcopy(good)
            if field == 'stages':
                value['generations'][1]['records'][rank]['stages'][3] = copy.deepcopy(good['generations'][0]['records'][rank]['stages'][3])
            else:
                value['generations'][1]['records'][rank][field] = copy.deepcopy(good['generations'][0]['records'][rank][field])
            mutations.append(value)
        value = copy.deepcopy(good)
        row = value['generations'][1]['records'][rank]
        body = bytearray.fromhex(row['output_le_hex'])
        at = next(i for i in range(0, len(body), 2) if body[i:i+2] == b'\0\0')
        body[at:at+2] = b'\0\x80'
        row['output_le_hex'], row['output_sha256'] = body.hex(), sha(body)
        mutations.append(value)
    value = copy.deepcopy(good); value['generations'][1]['observed_queue_frontiers'][0][1] = 2; mutations.append(value)
    value = copy.deepcopy(good); value['generations'][0]['generation'] = True; mutations.append(value)
    value = copy.deepcopy(good); value['unrecognized'] = 1; mutations.append(value)
    require(len(mutations) == 99, 'closed observation mutation census')
    for value in mutations:
        try:
            verify(value, req, 'test')
        except ValueError:
            pass
        else:
            raise AssertionError('mutation accepted')
    for body in ('{"x":1,"x":2}', '{"x":NaN}'):
        try:
            strict(body)
        except ValueError:
            pass
        else:
            raise AssertionError('non-strict JSON accepted')
    good_stdout = ('test ' + TEST + ' ... ok\n'
                   'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1026 filtered out; finished in 0.00s\n')
    test_census(good_stdout)
    for extra in ('test result: FAILED. 0 passed; 1 failed;\n', good_stdout.splitlines()[1] + '\n'):
        try:
            test_census(good_stdout + extra)
        except ValueError:
            pass
        else:
            raise AssertionError('contradictory summary accepted')
    print(json.dumps(dict(passed=True, gpu_execution=False, observation_mutation_refusals=len(mutations),
                          fixed_output_hashes=4, fixed_up_matrix_hashes=2, strict_json_refusals=2, stdout_refusals=2)))


def main():
    if sys.argv[1:] == ['--self-test']:
        selftest()
        return
    require(len(sys.argv) == 3, 'verify_native.py STDOUT REQUEST')
    raw = Path(sys.argv[1]).read_bytes()
    request_body = Path(sys.argv[2]).read_bytes()
    require(len(raw) < 4 << 20 and len(request_body) < 4096, 'bounded verifier inputs')
    req = strict(request_body)
    require(set(req) == {'schema', 'device_unique_ids', 'images', 'image_sha256', 'timeout_ms'}
            and req['schema'] == 'ferric-native-paired-guarded-mlp-request-v1' and req['image_sha256'] == HASHES
            and type(req['device_unique_ids']) is list and len(req['device_unique_ids']) == 2
            and all(integer(v, 1, 2**64 - 1) for v in req['device_unique_ids'])
            and len(set(req['device_unique_ids'])) == 2 and integer(req['timeout_ms'], 1, 5000), 'closed request')
    require(type(req['images']) is list and len(req['images']) == 3
            and all(type(v) is str and Path(v).is_absolute() for v in req['images'])
            and len(set(req['images'])) == 3, 'three image paths')
    text = raw.decode()
    test_census(text)
    lines = [line[len(MARKER):] for line in raw.splitlines() if line.startswith(MARKER)]
    require(len(lines) == 1, 'one paired observation')
    result = verify(strict(lines[0]), req, sha(request_body))
    result.update(schema='ferric-native-paired-reuse-mlp-dyadic-verification-v1', stdout_sha256=sha(raw),
                  request_sha256=sha(request_body), full_model_acceptance=False, performance_claim=False)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
