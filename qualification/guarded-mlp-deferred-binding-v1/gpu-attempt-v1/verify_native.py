"""Independent integer reference for exact residual/output two-bank reuse."""
import copy
import functools
import hashlib
import json
from pathlib import Path
import re
import sys

TEST = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::native::paired::interleaved::native_paired_guarded_mlp_exact_residual_v1'
HASHES = ['25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25',
          'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589',
          'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66']
WIDTH, INNER = 4096, 6144
MARKER = b'FERRIC_NATIVE_PAIRED_EXACT_RESIDUAL_MLP_V1='
SCALES = (-1, 1, -2, 2, -4, 4, -8, 8)
WRITABLE = (0, 5, 6, 7, 8, 9)
INITIAL_PREFIX = [1, 0, 1] + [0] * 545


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


def up8(rank, row, scale):
    magnitude = 1 << ((row // 3 + rank) % 2)
    return scale * (-magnitude if (5 * row + rank) % 3 == 0 else magnitude)


def activation(rank, row, scale):
    return gate(rank, row) * up8(rank, row, scale) // 8


def down_terms(rank, row):
    columns = [(13 * row + 37 * rank + 2049 * i) % INNER for i in range(3)]
    require(len(set(columns)) == 3, 'three distinct sparse columns')
    weights8 = [4 if (row + rank) % 2 == 0 else -4, 1, -2]
    return list(zip(columns, weights8))


def down8(rank, row, scale):
    return sum(activation(rank, column, scale) * weight for column, weight in down_terms(rank, row))


def matrix_sha(rank, role, scale):
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
            values = [(column, b16(up8(rank, row, scale) * sign(rank, column), 3))]
        for column, value in values:
            body[column * 2:column * 2 + 2] = value
        digest.update(body)
    return digest.hexdigest()


@functools.lru_cache(maxsize=8)
def reference(scale):
    require(type(scale) is int and scale in SCALES, 'closed reference scales')
    # R1 produces exactly +/-1; RMSNorm with epsilon 1e-6 rounds back to
    # BF16 +/-1. For Gate=8/16, exp(Gate)>512 (already true for the first six
    # Taylor terms at8), so Gate*512/513 < SiLU(Gate) < Gate. This interval
    # is strictly inside Gate's BF16 rounding cell. The selected image
    # materializes that BF16 SiLU before multiplying Up.
    rows = []
    for rank in range(2):
        partial = b''.join(f32(2 * (i % 7 - 3) if rank == 0 else (7 - 2 * (i % 7)), 4) for i in range(WIDTH))
        residual = b''.join(b16(16 * sign(rank, i) - 1, 4) for i in range(WIDTH))
        stages = []
        for index in range(10):
            if index in (2, 3, 4):
                stages.append(dict(root=index, bytes=50331648, sha256=matrix_sha(rank, index - 2, scale), le_hex=None))
                continue
            if index in (0, 5):
                body = b''.join(b16(sign(rank, i)) for i in range(WIDTH))
            elif index == 1:
                body = b16(1) * WIDTH
            elif index == 6:
                body = b''.join(b16(gate(rank, i)) for i in range(INNER))
            elif index == 7:
                body = b''.join(b16(up8(rank, i, scale), 3) for i in range(INNER))
            elif index == 8:
                body = b''.join(b16(activation(rank, i, scale)) for i in range(INNER))
            else:
                body = b''.join(f32(down8(rank, i, scale), 3) for i in range(WIDTH))
            stages.append(dict(root=index, bytes=len(body), sha256=sha(body), le_hex=body.hex()))
        output = b''.join(b16(down8(0, i, scale) + down8(1, i, scale) + 8 * sign(rank, i), 3) for i in range(WIDTH))
        rows.append(dict(rank=rank, stages=stages, partial_le_hex=partial.hex(), partial_sha256=sha(partial),
                         residual_before_le_hex=residual.hex(), residual_before_sha256=sha(residual),
                         output_le_hex=output.hex(), output_sha256=sha(output)))
    return rows


def terminal(prefix, guard, generation):
    require(type(prefix) is list and len(prefix) == 548 and all(integer(v, 0, 0xffffffff) for v in prefix),
            'genuine terminal prefix words')
    fixed = [1, 0, 0, 31] + [1, 96, 96, 1, 64] * 2 + [0xffffffff] * 8 + [3] + [0xffffffff] * 8 + [3]
    require(prefix[:32] == fixed and all(1 <= v <= 64 for v in prefix[32:290])
            and prefix[290:] == [64] * 258, 'terminal task claims/arrivals')
    require(guard == [generation, 0, 1, 0] and all(type(v) is int for v in guard), 'current Valid guard')


FINAL_HASHES = [
 ['b20e302f292f34269bcb8050df66f0a0f3c5e05cd784f5aef70f049473422f73','9bbbfec933d22db9ed77aa6fb3af82cf7bbe5fb51f0f2d58066fed2c91dbaad4'],
 ['6e6ecb4b75e6e5f09a212bbd4d4779fc2975c026f4398c2241230843cf4a33bd','9fde0e89ff2a46d8b366a4adc5dbe4bb1728106112b2b8fa563e4af6d944d3bd'],
 ['dc168a4cf10046bccf128222d385ad604427d5527fd61477c8ab65aaf38b0157','d67e525ebc822f08791ee7d44f34ca4e5d39a8bf8e93114bbafa0b53f54dac02'],
 ['403903da99ade382e932312fb9fcdaa5d5fe3e41bf05768f614667fd77b160bf','e7164a702f121bbaa1b9e940960177f2b7670bf279f641e6e827be6bdfba3e93'],
 ['0403c9a2c2f7eb3015fa56f8c8193c2ad3fec071adff624703c7ae06ec4aea33','c80e80be3a90c3cf3e0067983de7023be3407c84f652e25ee32d3fa31eaa68a6'],
 ['1ce5c1c95bacc944f878faadb899b26b4b44ac0a9db0599af82e8681d456edcb','b197b000825190db35241e80fb84462eb64b938fc3939f1298b2daab47dc41f5'],
 ['e4abf09c2e64dcd0a4dad294a64f44da0bd124085f0ff5bc8fb7a3fabfb54b88','45c1625266516424ffaef254271d0dfd07b41ea3966030b714af47554414dc49'],
 ['62d3a5411e727f0e514798c9992de1a9bcaadb627751925cb1deba6ea9bc9fca','9ba51ea44d9a0d5d4cfde87b36d943191730e0e2832993986766f37bc11c0eef'],
]
UP_HASHES = [
 ['3fced08dc5eb0444bad194ea4f5d12fc3b1a92682d661fa66da3f9f778ea8c72','97191553d52fdd9bdd6e770ecc4374d1ae41a16f96fb87a710d13b6494d1374f'],
 ['e8acc9025230b0b6e92839eeaa86f437a6400f7854796e6ee6ed4b87ef9d1cc7','2086136b0338d4a163c317960b009767f7f7cf4ce175062011c54dfa2dd41f52'],
 ['9171b5727cd208fb2a244636d107eff7a48a0a0cb8fe9f3b0cd43ad66620cc0f','8786800f14eb32fb807803cd264aebe628e677941902aa14ef0dcbfb48c24ad7'],
 ['4d8d301047aab087afdf5c54302051fd4848fac603ac85605688d3b1e1a47da5','1c1d077694c7f6a826ff26c6a770ee9887cb9c376d6f65193545df3180583dd9'],
 ['a4533c4ab7ca44f99e41411a84fad4515d9e124006c3c0c4941e6db4e7cfbe7e','65d5fd51892e13e7ca24f3263f239f2a4ebbb53a3fdef3f2b89265cd71430c22'],
 ['4d0c73076df0025f92118bcabd2180d3e22857c7deb57c596bd53d669747a8bb','8b5fe43a4d6c5b37a5f57edf4997722ea0905e4d195a39f316f16070e5a13496'],
 ['d74849799bdb08d8a97bb9b5983bc0ce0184890f768495a85729c7d9f2726da3','2db79e5ca535928334b679432fb512b40e5204b1d54395c9aad7bb4de8be9c58'],
 ['f739f4a4b7e631cec45ee4d06517abe659b0794f78f872d8b5748c9c09fd240c','69555e7c6621a33cf87206a21051e9af52c4e716beda891b93561ac92ce387f5'],
]
ZEROS = ((0,0),(0,0),(1228,1229),(820,819),(0,0),(0,0),(272,273),(411,410))


def word_sha(words):
    return sha(b''.join(v.to_bytes(4, 'little') for v in words))


def flags(request, request_sha):
    return dict(kernel_dispatches=64, barrier_packets=16, completion_signals=80,
        paired_coordinator_tested=True, owner_lifecycle_tested=True, rearm_tested=True,
        owners_reused=True, payloads_reused=True, fresh_arenas_per_segment=True,
        interleaving_tested=True, inactive_payload_checks=336,
        fixture='two-layer-two-bank-exact-residual-v1', performance_claim=False, canaries_tested=False,
        exact_residual_reuse_tested=True,
        schema='ferric-native-paired-guarded-mlp-exact-residual-observation-v1', request_sha256=request_sha,
        image_sha256=HASHES, device_unique_ids=request['device_unique_ids'], healthy_close=True,
        gpu_execution=True, full_model_acceptance=False, production_authority=False)


def state(generation, completed=False, prefixes=None):
    return dict(generation=generation, completed=completed,
                prefix_sha256=prefixes or [word_sha(INITIAL_PREFIX)] * 2,
                guards=[[generation, 0, int(completed), 0] for _ in range(2)])


def identities_from(initial):
    require(type(initial) is list and len(initial) == 4, 'four initial pairs')
    identities = [row['owners'] for row in initial]
    group, ids = None, set()
    for pair in identities:
        require(type(pair) is list and len(pair) == 2, 'two owner identities')
        for rank, owner in enumerate(pair):
            require(type(owner) is list and len(owner) == 4
                    and all(integer(v, 0, 2**64-1) for v in owner)
                    and owner[0] > 0 and owner[1] > 0 and owner[2:] == [rank, 2208],
                    'typed scalar owner identity')
            group = owner[0] if group is None else group
            require(owner[0] == group and owner[1] not in ids, 'one group eight distinct owners')
            ids.add(owner[1])
    return identities


def frontiers(actual, write, reads):
    require(type(actual) is list and len(actual) == 2, 'two current queue frontiers')
    for rank, observed in enumerate(actual):
        require(type(observed) is list and len(observed) == 2
                and all(integer(v, 0, 2**64-1) for v in observed)
                and observed[0] == write and reads[rank] <= observed[1] <= write,
                'actual monotone current queue frontier')
        reads[rank] = observed[1]


def owners(actual, expected, identities, write, reads):
    require(type(actual) is list and len(actual) == 4, 'four fresh pair observations')
    for pair, (row, wanted) in enumerate(zip(actual, expected)):
        require(set(row) == {'pair', 'owners', 'observed_queue_frontiers'} | set(wanted),
                'closed pair observation')
        require(type(row['pair']) is int and row['pair'] == pair, 'ordered pair index')
        require(row['owners'] == identities[pair]
                and all(type(v) is int for owner in row['owners'] for v in owner), 'unchanged typed owner identity')
        require(type(row['generation']) is int and type(row['completed']) is bool, 'typed pair phase/generation')
        require(all(type(v) is int for guard in row['guards'] for v in guard), 'typed observed guard words')
        for field, value in wanted.items():
            require(row[field] == value, 'unchanged/current observed pair field: ' + field)
        frontiers(row['observed_queue_frontiers'], write, reads)


def poison_sha(root):
    return sha((bytes.fromhex('0100c07f') * WIDTH) if root == 9 else
               (bytes.fromhex('c17f') * (WIDTH if root in (0, 5) else INNER)))


def inactive_expected(active, last_scale):
    rows = []
    for pair in range(4):
        if pair == active:
            continue
        scale = last_scale[pair]
        for rank in range(2):
            refs = None if scale is None else reference(scale)[rank]
            rows.append(dict(pair=pair, rank=rank, scale=scale,
                roots=[dict(root=root, sha256=poison_sha(root) if refs is None else refs['stages'][root]['sha256'])
                       for root in WRITABLE],
                output_sha256=reference(1)[rank]['residual_before_sha256'] if refs is None else refs['output_sha256']))
    return rows


def verify(value, request, request_sha):
    expected_flags = flags(request, request_sha)
    require(set(value) == set(expected_flags) | {'initial', 'events'}, 'closed interleaved observation')
    for key, expected in expected_flags.items():
        require(type(value[key]) is type(expected) and value[key] == expected, 'annotation: ' + key)
    require(all(integer(v, 1, 2**64-1) for v in value['device_unique_ids']), 'integer device identities')
    identities = identities_from(value['initial'])
    payloads = {}
    used_ids = {token[1] for pair in identities for token in pair}
    overwritten = 0
    states, reads, last_scale = [state(1) for _ in range(4)], [0, 0], [None] * 4
    owners(value['initial'], states, identities, 0, reads)
    events = value['events']
    require(type(events) is list and len(events) == 10, 'eight segments and two whole-bank rearms')
    cursor, write = 0, 0
    for forward in range(4):
        bank, generation = forward % 2, 1 + forward // 2
        if forward >= 2:
            event = events[cursor]; cursor += 1
            require(set(event) == {'kind', 'forward', 'bank', 'next_generation', 'before', 'after'}
                    and event['kind'] == 'rearm', 'closed rearm event')
            for field, expected in (('forward', forward), ('bank', bank), ('next_generation', generation)):
                require(type(event[field]) is int and event[field] == expected, 'rearm label: ' + field)
            owners(event['before'], states, identities, write, reads)
            for pair in range(2*bank, 2*bank+2):
                require(states[pair]['completed'] and states[pair]['generation'] + 1 == generation,
                        'rearm only completed selected bank and exact successor')
                states[pair] = state(generation)
            owners(event['after'], states, identities, write, reads)
        for layer in range(2):
            event = events[cursor]; cursor += 1
            scale, pair = SCALES[forward * 2 + layer], bank * 2 + layer
            require(set(event) == {'kind', 'forward', 'bank', 'layer', 'generation', 'scale', 'records',
                    'inactive_payloads', 'owners', 'observed_queue_frontiers', 'segment_host_ns'}
                    and event['kind'] == 'segment', 'closed segment event')
            for field, expected in (('forward', forward), ('bank', bank), ('layer', layer),
                                    ('generation', generation), ('scale', scale)):
                require(type(event[field]) is int and event[field] == expected, 'segment label: ' + field)
            require(not states[pair]['completed'] and states[pair]['generation'] == generation,
                    'segment consumes selected Ready pair')
            require(integer(event['segment_host_ns'], 0, request['timeout_ms'] * 1000000 - 1),
                    'bounded host segment interval, not GPU timing')
            write += 5
            frontiers(event['observed_queue_frontiers'], write, reads)
            records = event['records']
            require(type(records) is list and len(records) == 2, 'two active numerical records')
            prefixes = []
            for rank, (actual, expected) in enumerate(zip(records, reference(scale))):
                require(set(actual) == set(expected) | {'prefix', 'guard', 'residual_token', 'output_token'}, 'closed rank record')
                terminal(actual['prefix'], actual['guard'], generation)
                require(type(actual['rank']) is int, 'integer rank')
                for key in ('residual_token', 'output_token'):
                    token = actual[key]
                    require(type(token) is list and len(token) == 4
                            and all(integer(v, 0, 2**64-1) for v in token)
                            and token[0] == identities[0][0][0] and token[1] > 0
                            and token[2:] == [rank, 8192], 'typed exact-residual payload token')
                token = actual['residual_token']
                require(token == actual['output_token'], 'exact residual/output identity')
                key = (pair, rank)
                if key not in payloads:
                    require(token[1] not in used_ids, 'private payload distinct from other pairs and owners')
                    payloads[key] = token
                    used_ids.add(token[1])
                require(payloads[key] == token, 'immutable residual/output binding across reuse')
                for field, want in expected.items():
                    require(actual[field] == want, 'exact independent rank field: ' + field)
                before = bytes.fromhex(actual['residual_before_le_hex'])
                output = bytes.fromhex(actual['output_le_hex'])
                require(len(before) == len(output) == WIDTH * 2 and
                        all(before[i:i+2] != output[i:i+2] for i in range(0, len(output), 2)),
                        'every residual word overwritten')
                overwritten += WIDTH
                for stage in actual['stages']:
                    require(set(stage) == {'root','bytes','sha256','le_hex'}
                            and type(stage['root']) is int and type(stage['bytes']) is int, 'typed closed stage')
                prefixes.append(word_sha(actual['prefix']))
            states[pair] = state(generation, True, prefixes)
            last_scale[pair] = scale
            inactive = event['inactive_payloads']
            require(type(inactive) is list and len(inactive) == 6, 'six inactive rank payload observations')
            for actual, expected in zip(inactive, inactive_expected(pair, last_scale)):
                require(actual == expected and type(actual['pair']) is int and type(actual['rank']) is int
                        and (actual['scale'] is None or type(actual['scale']) is int)
                        and all(type(row['root']) is int for row in actual['roots']),
                        'exact inactive private payload hashes')
            owners(event['owners'], states, identities, write, reads)
    require(cursor == 10 and write == 40 and all(s['completed'] and s['generation'] == 2 for s in states),
            'four pairs completed local generation two')
    require(len(payloads) == 8 and overwritten == 65536, 'eight private reused payloads and complete overwrite witness')
    return dict(passed=True, computed_elements_checked=557056, final_output_elements_checked=65536,
        dense_matrix_hashes_checked=48, distinct_dense_matrix_hashes=20,
        terminal_owner_observations_checked=16, owner_readback_observations_checked=104,
        inactive_payload_hashes_checked=336, segments_checked=8, bank_rearms_checked=2,
        residual_alias_observations_checked=16, distinct_residual_output_allocations=8,
        overwritten_residual_elements_checked=overwritten)


def fixture_request():
    return dict(schema='ferric-native-paired-guarded-mlp-request-v1',
        device_unique_ids=[2**64-2, 2**64-1], images=['/tmp/r1','/tmp/mlp','/tmp/guard'],
        image_sha256=HASHES, timeout_ms=5000)


def owner_records(states, identities, write):
    return [dict(pair=pair, owners=copy.deepcopy(identities[pair]), **copy.deepcopy(s),
                 observed_queue_frontiers=[[write, write], [write, write]])
            for pair, s in enumerate(states)]


def synthetic():
    req = fixture_request()
    value = dict(flags(req, 'test'), events=[])
    identities = [[[2**64-1, 2**64-100+2*pair+rank, rank, 2208] for rank in range(2)] for pair in range(4)]
    states, last = [state(1) for _ in range(4)], [None] * 4
    value['initial'] = owner_records(states, identities, 0)
    prefix = [1,0,0,31] + [1,96,96,1,64] * 2 + [0xffffffff] * 8 + [3] + [0xffffffff] * 8 + [3] + [64] * 516
    write = 0
    for forward in range(4):
        bank, generation = forward % 2, 1 + forward // 2
        if forward >= 2:
            before = owner_records(states, identities, write)
            for pair in range(2*bank, 2*bank+2):
                states[pair] = state(generation)
            value['events'].append(dict(kind='rearm', forward=forward, bank=bank,
                next_generation=generation, before=before, after=owner_records(states, identities, write)))
        for layer in range(2):
            pair, scale = 2*bank+layer, SCALES[2*forward+layer]
            records = copy.deepcopy(reference(scale))
            for row in records:
                row.update(prefix=prefix.copy(), guard=[generation,0,1,0])
                token = [2**64-1, 2**64-200+2*pair+row['rank'], row['rank'], 8192]
                row.update(residual_token=token.copy(), output_token=token.copy())
            write += 5
            states[pair], last[pair] = state(generation, True, [word_sha(prefix)] * 2), scale
            value['events'].append(dict(kind='segment', forward=forward, bank=bank, layer=layer,
                generation=generation, scale=scale, records=records,
                inactive_payloads=inactive_expected(pair, last),
                owners=owner_records(states, identities, write),
                segment_host_ns=req['timeout_ms']*1000000-1,
                observed_queue_frontiers=[[write,write],[write,write]]))
    return req, value


def test_census(text):
    summaries = re.findall(r'^test result:.*$', text, re.M)
    require(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M) == [(TEST,'ok')]
            and len(summaries) == 1
            and re.fullmatch(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; '
                r'1058 filtered out; finished in [0-9]+(?:\.[0-9]+)?s', summaries[0]), 'one exact native test')


def selftest():
    require(b16(1).hex() == '803f' and b16(-1).hex() == '80bf' and f32(1,3).hex() == '0000003e',
            'integer dyadic anchors')
    req, good = synthetic()
    verify(good, req, 'test')
    for rank, expected in enumerate((
        ('b2f33b1e042755dc40eb5b704842fd98d73d8fa6b75735580f1af89fb37ed76a',
         '235e169d667ceff0bc517d233accb6e05a758ee4ad91ea7c7c542ccf520b1931'),
        ('7fcd049b82fdbfd37bf8d09df778c3a22f06581edbac7907155c96571a93402a',
         'e6bb049aaf15e8e7e906c052333ae1fd9a9a255ffd85bd616a781552af32193e'),
    )):
        row = reference(1)[rank]
        require((row['partial_sha256'], row['residual_before_sha256']) == expected, 'independent input anchors')
    for index, scale in enumerate(SCALES):
        require([r['output_sha256'] for r in reference(scale)] == FINAL_HASHES[index], 'independent final anchors')
        require([r['stages'][3]['sha256'] for r in reference(scale)] == UP_HASHES[index], 'independent matrix anchors')
        for rank, row in enumerate(reference(scale)):
            body = bytes.fromhex(row['output_le_hex'])
            require(sum(body[i:i+2] == b'\0\0' for i in range(0,len(body),2)) == ZEROS[index][rank]
                    and all(body[i:i+2] != b'\0\x80' for i in range(0,len(body),2)), 'positive zero anchors')
            for earlier in SCALES[:index]:
                old = bytes.fromhex(reference(earlier)[rank]['output_le_hex'])
                require(all(body[i:i+2] != old[i:i+2] for i in range(0,len(body),2)), 'every final word distinguishes scale')
    refusals = 0
    def refuse(value):
        nonlocal refusals
        try:
            verify(value, req, 'test')
        except ValueError:
            refusals += 1
        else:
            raise AssertionError('mutated observation accepted')
    def reject(path, replacement):
        value = copy.deepcopy(good)
        node = value
        for key in path[:-1]:
            node = node[key]
        node[path[-1]] = replacement
        refuse(value)
    for ei, event in enumerate(good['events']):
        base = ['events', ei]
        if event['kind'] == 'segment':
            for rank in range(2):
                row = base + ['records', rank]
                for root in range(10):
                    reject(row+['stages',root,'sha256'], '0'*64)
                for field, value in [('output_le_hex','00'), ('output_sha256','0'*64)]:
                    reject(row+[field], value)
                reject(row+['prefix',0], 0); reject(row+['guard',0], 0)
            for field, value in [('forward',event['forward']+1), ('bank',1-event['bank']),
                ('layer',1-event['layer']), ('generation',0), ('scale',0),
                ('observed_queue_frontiers',[[0,0],[0,0]]), ('segment_host_ns',5000000000)]:
                reject(base+[field], value)
            for pair in range(4):
                row = base+['owners',pair]
                old = event['owners'][pair]
                reject(row+['generation'],0); reject(row+['completed'],not old['completed'])
                reject(row+['prefix_sha256',0], '0'*64)
                reject(row+['guards',0,2], 1-old['guards'][0][2])
                for rank in range(2):
                    reject(row+['owners',rank,1],old['owners'][rank][1]+1)
            for ri in range(6):
                row = base+['inactive_payloads',ri]
                reject(row+['output_sha256'],'0'*64); reject(row+['roots',0,'sha256'],'0'*64)
        else:
            for side in ('before','after'):
                for pair in range(4):
                    row = base+[side,pair]
                    reject(row+['generation'],0)
                    reject(row+['completed'],not event[side][pair]['completed'])
    for pair in range(4):
        row = ['initial',pair]
        reject(row+['owners',0,1],good['initial'][pair]['owners'][0][1]+1)
        reject(row+['prefix_sha256',0],'0'*64); reject(row+['guards',0,2],1)
        reject(row+['generation'],0); reject(row+['completed'],True)
    for field, changed in [('kernel_dispatches',63),('performance_claim',True),('rearm_tested',False),
            ('owners_reused',False),('fresh_arenas_per_segment',False),('interleaving_tested',False)]:
        reject([field],changed)
    for events in (good['events'][:-1],list(reversed(good['events'])),[good['events'][0]]*10,
                   good['events']+[good['events'][-1]]):
        reject(['events'],events)
    reject(['events',0,'generation'],True)
    bad = copy.deepcopy(good); bad['unrecognized']=1; refuse(bad)
    for ei, event in enumerate(good['events']):
        if event['kind'] != 'segment':
            continue
        for rank, row in enumerate(event['records']):
            body = bytearray.fromhex(row['output_le_hex'])
            at = next((i for i in range(0,len(body),2) if body[i:i+2] == b'\0\0'), None)
            if at is not None:
                body[at:at+2] = b'\0\x80'
                bad = copy.deepcopy(good)
                bad['events'][ei]['records'][rank].update(output_le_hex=body.hex(),output_sha256=sha(body))
                refuse(bad)
    require(refusals == 640, 'preserved original mutation census')
    for ei, event in enumerate(good['events']):
        if event['kind'] != 'segment':
            continue
        for rank, row in enumerate(event['records']):
            base = ['events', ei, 'records', rank]
            for key in ('residual_token', 'output_token'):
                for field in range(4):
                    reject(base + [key, field], row[key][field] ^ 1)
            reject(base + ['residual_before_sha256'], '0' * 64)
            bad = copy.deepcopy(good)
            bad['events'][ei]['records'][rank].update(
                residual_before_le_hex=row['output_le_hex'], residual_before_sha256=row['output_sha256'])
            refuse(bad)
            old_residual = b''.join(b16(8 * sign(rank, i) - 1, 3) for i in range(WIDTH))
            bad = copy.deepcopy(good)
            bad['events'][ei]['records'][rank].update(
                residual_before_le_hex=old_residual.hex(), residual_before_sha256=sha(old_residual))
            refuse(bad)
            before = bytes.fromhex(row['residual_before_le_hex'])
            # Cover first/middle/last, signs and BF16 zero witnesses without
            # quadratic copying of a full multi-megabyte observation per word.
            for column in (0, 1, WIDTH // 2, WIDTH - 1):
                output = bytearray.fromhex(row['output_le_hex'])
                output[2*column:2*column+2] = before[2*column:2*column+2]
                bad = copy.deepcopy(good)
                bad['events'][ei]['records'][rank].update(output_le_hex=output.hex(), output_sha256=sha(output))
                refuse(bad)
            bad = copy.deepcopy(good)
            other = event['records'][1-rank]['residual_token']
            bad['events'][ei]['records'][rank].update(residual_token=other.copy(), output_token=other.copy())
            refuse(bad)
        old_partial = b''.join(f32(4 - i % 7, 3) for i in range(WIDTH))
        bad = copy.deepcopy(good)
        bad['events'][ei]['records'][1].update(partial_le_hex=old_partial.hex(), partial_sha256=sha(old_partial))
        refuse(bad)
    # Preserve same-rank shape while aliasing another pair, or drift only on reuse.
    for first, second in ((0, 1), (0, 2), (0, 3), (0, 5), (1, 6), (2, 8), (3, 9)):
        for rank in range(2):
            bad = copy.deepcopy(good)
            token = good['events'][first]['records'][rank]['residual_token'].copy()
            if first == 0 and second == 5 or first == 1 and second == 6 or first == 2 and second == 8 or first == 3 and second == 9:
                token[1] -= 100
            bad['events'][second]['records'][rank].update(residual_token=token.copy(), output_token=token.copy())
            refuse(bad)
    require(refusals == 918, 'closed exact-residual mutation census')
    for body in ('{"x":1,"x":2}','{"x":NaN}'):
        try: strict(body)
        except ValueError: pass
        else: raise AssertionError('non-strict JSON accepted')
    line = 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 0.00s\n'
    text = 'test '+TEST+' ... ok\n'+line
    test_census(text)
    for extra in ('test result: FAILED. 0 passed; 1 failed;\n',line):
        try: test_census(text+extra)
        except ValueError: pass
        else: raise AssertionError('contradictory test summary accepted')
    # Synthetic identities/timing use maximum widths; allow ample libtest framing.
    wire_upper = len(json.dumps(good,separators=(',',':')).encode()) + 16384
    require(wire_upper < 4 << 20, 'keep original 4 MiB native stream limit')
    print(json.dumps(dict(passed=True,gpu_execution=False,observation_mutation_refusals=refusals,
        fixed_output_hashes=16,fixed_up_matrix_hashes=16,strict_json_refusals=2,stdout_refusals=2,
        native_wire_upper_bytes=wire_upper)))


def main():
    if sys.argv[1:] == ['--self-test']:
        selftest()
        return
    require(len(sys.argv)==3, 'verify_native.py STDOUT REQUEST')
    raw, request_body = Path(sys.argv[1]).read_bytes(), Path(sys.argv[2]).read_bytes()
    require(len(raw)<4<<20 and len(request_body)<4096, 'bounded verifier inputs')
    req = strict(request_body)
    require(set(req)=={'schema','device_unique_ids','images','image_sha256','timeout_ms'}
        and req['schema']=='ferric-native-paired-guarded-mlp-request-v1' and req['image_sha256']==HASHES
        and type(req['device_unique_ids']) is list and len(req['device_unique_ids'])==2
        and all(integer(v,1,2**64-1) for v in req['device_unique_ids'])
        and len(set(req['device_unique_ids']))==2 and integer(req['timeout_ms'],1,5000), 'closed request')
    require(type(req['images']) is list and len(req['images'])==3
        and all(type(v) is str and Path(v).is_absolute() for v in req['images'])
        and len(set(req['images']))==3, 'three image paths')
    test_census(raw.decode())
    lines = [line[len(MARKER):] for line in raw.splitlines() if line.startswith(MARKER)]
    require(len(lines)==1, 'one interleaved observation')
    result = verify(strict(lines[0]),req,sha(request_body))
    result.update(schema='ferric-native-exact-residual-mlp-dyadic-verification-v1',
        stdout_sha256=sha(raw),request_sha256=sha(request_body),
        full_model_acceptance=False,performance_claim=False)
    print(json.dumps(result,sort_keys=True))


if __name__ == '__main__':
    main()
