"""Read-only independent audit of the retained historical O replay.

Only standard-library data parsing and exact integer arithmetic are used.
No project module, compiler, GPU, model framework, or subprocess is invoked.
"""

import hashlib
import io
import json
from pathlib import Path
import re
import stat
import struct
import tarfile
from fractions import Fraction


LOCAL = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
WINDOWS = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
PROPOSAL = LOCAL / 'proposals/p228-o-projection-exact-replay-v1'
REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/'
NATIVE = 'prefix-silu-materialized-capture-gpu-v228-v1/'
REFERENCE = 'layer0-framework-capture-v228-v1/'
PINS = {
    'report': (WINDOWS / 'o-projection-exact-replay-complete-v1.json', 10946336,
               '869541d4852c6cc1af45164f2a2521b075db30c62ed0487309cf0dd16abaa515'),
    'tests': (WINDOWS / 'o-projection-tests-complete-v1.json', 4605,
              '701e09f47946c2974c0f79eca0d14e5b1cb21e94ee197955dc12ee796ea075d9'),
    'source': (PROPOSAL / 'source-manifest.json', 4340,
               '6655e7b7bd7a7f9b6f2e10aae1b4a5e854a0cf2b39481b237f420366f69a6827'),
    'native_archive': (LOCAL / 'prefix-silu-materialized-capture-gpu-v228-v1-retained.tar.gz',
                       427580, '1403889c2fc9bf87c5ebe8cb7fa89e66bbc58af1d07991c78862adf4edad6ef4'),
    'reference_archive': (LOCAL / 'layer0-framework-observation-v228-v1.tar.gz',
                          533661, '61c033c43e0234881aea3d6d28ebff0ffb87354fef0166d1e20b712a805dc813'),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(body):
    return hashlib.sha256(body).hexdigest()


def read(path, size, sha):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size == size <= 16 << 20, 'ordinary bounded input')
    body = path.read_bytes()
    after = path.lstat()
    stamp = lambda value: (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
                           value.st_size, value.st_mtime_ns, value.st_ctime_ns)
    require(stamp(before) == stamp(after) and len(body) == size and digest(body) == sha,
            'stable authenticated input: ' + str(path))
    return body


def archive(body):
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as stream:
        members = stream.getmembers()
        require(len(members) <= 1024 and len({row.name for row in members}) == len(members),
                'bounded unique archive roster')
        require(all(row.isfile() and not Path(row.name).is_absolute()
                    and '..' not in Path(row.name).parts and 0 <= row.size <= 16 << 20
                    for row in members) and sum(row.size for row in members) <= 32 << 20,
                'ordinary bounded archive bodies')
        return {row.name: stream.extractfile(row).read() for row in members}


def units(bits, fraction_bits):
    """Decode a finite binary32/BF16 encoding in exact 2^-266 units."""
    sign = -1 if bits >> (fraction_bits + 8) else 1
    exponent = (bits >> fraction_bits) & 255
    significand = bits & ((1 << fraction_bits) - 1)
    require(exponent < 255, 'finite scalar encoding')
    if exponent:
        significand += 1 << fraction_bits
    shift = exponent + 139 - fraction_bits if exponent else 140 - fraction_bits
    return sign * (significand << shift)


def nearest(total, fraction_bits):
    """Independent nearest-even search over finite positive encodings.

    This deliberately does not reproduce the replay's bit-length/shift rounding.
    The actual report lies strictly inside the finite representable range.
    """
    sign = 1 << (fraction_bits + 8) if total < 0 else 0
    magnitude = abs(total)
    low, high = 0, (255 << fraction_bits) - 1
    require(magnitude < units(high, fraction_bits), 'interior finite rounding case')
    while low < high:
        middle = (low + high + 1) // 2
        if units(middle, fraction_bits) <= magnitude:
            low = middle
        else:
            high = middle - 1
    return sign | min((low, low + 1),
                      key=lambda word: (abs(units(word, fraction_bits) - magnitude), word & 1))


def main():
    pinned = {name: read(*pin) for name, pin in PINS.items()}
    report, tests, manifest = (json.loads(pinned[name]) for name in ('report', 'tests', 'source'))
    require(tests['names'] == manifest['test_names'] and tests['tests'] == 18
            and tests['errors'] == tests['failures'] == tests['skipped'] == 0
            and tests['passed'] is True and tests['gpu_execution'] is False
            and tests['source_and_input_postchecks_passed'] is True
            and tests['source_manifest_sha256'] == PINS['source'][2], 'actual closed 18-test result')
    require(len(re.findall(r'^test_.* \.\.\. ok$', tests['log'], re.M)) == 18
            and '\nRan 18 tests ' in tests['log'] and tests['log'].endswith('\nOK\n'),
            'actual named unittest log')
    native, reference = archive(pinned['native_archive']), archive(pinned['reference_archive'])
    consumed, missing = {}, []
    for entry in report['consumed']:
        pin = entry['original']
        relative = pin['path'].removeprefix(REMOTE)
        body = None
        if relative.startswith('o-projection-source-input-v228-v1/'):
            body = read(PROPOSAL / Path(relative).name, pin['bytes'], pin['sha256'])
        elif relative.startswith('row-reciprocal-checked-probe-v228-v7/'):
            body = read(LOCAL / relative, pin['bytes'], pin['sha256'])
        elif relative in native:
            body = native[relative]
        elif relative in reference:
            body = reference[relative]
        else:
            missing.append(pin)
        require(entry['retained']['bytes'] == pin['bytes']
                and entry['retained']['sha256'] == pin['sha256'], 'physical alias keeps exact identity')
        if body is not None:
            require(len(body) == pin['bytes'] and digest(body) == pin['sha256']
                    and relative not in consumed, 'consumed input identity/uniqueness')
            consumed[relative] = body
    require(len(consumed) == 30 and len(missing) == 2 and len(report['consumed']) == 32,
            'exact locally verified input census')
    ref = json.loads(reference[REFERENCE + 'capture.json'])
    require({Path(pin['path']).name for pin in missing} == {'config.json', 'model.safetensors.index.json'},
            'only model config/index bodies not locally reread')
    for pin in missing:
        old = ref['model_sources'][Path(pin['path']).name]
        require((pin['bytes'], pin['sha256']) == (old['bytes'], old['sha256']),
                'config/index lineage through authenticated framework capture')
    for role in ('attention-output', 'o-projection', 'embedding', 'first-residual'):
        require(reference[REFERENCE + 'pass1-' + role + '.bf16']
                == reference[REFERENCE + 'pass2-' + role + '.bf16'], 'genuine repeated framework payload')
    summary = json.loads(native[NATIVE + 'native/summary.json'])
    raw = native[NATIVE + 'native/candidate-capture.bin']
    selected, offset = {}, 0
    for stage in summary['stages']:
        require(stage['offset'] == offset, 'contiguous native capture')
        body = raw[offset:offset + stage['bytes']]
        offset += stage['bytes']
        require(hashlib.sha256(body).digest() == bytes(stage['sha256']), 'native stage digest')
        key = stage['rank'], stage['stage']
        require(key not in selected, 'unique native stage')
        selected[key] = body
    require(offset == len(raw) and len(selected) == 28, 'complete original native capture')
    require(selected[0, 'attention'] + selected[1, 'attention']
            == reference[REFERENCE + 'pass1-attention-output.bf16'], 'same immediate O operand')
    partials = [struct.unpack('<4096I', selected[rank, 'output-partial']) for rank in (0, 1)]
    residuals = [struct.unpack('<4096H', selected[rank, 'first-residual']) for rank in (0, 1)]
    expected = {name: struct.unpack('<4096H', reference[REFERENCE + 'pass1-' + name + '.bf16'])
                for name in ('embedding', 'o-projection', 'first-residual')}
    registration = json.loads(native[NATIVE + 'native/candidate-registration.json'])
    uploads = json.loads(native[NATIVE + 'native/candidate-uploads.json'])
    require(len(report['weight_upload_slices']) == 2, 'two rank weight slices')
    for rank, row in enumerate(report['weight_upload_slices']):
        layers = [value for value in registration['layers'] if value['layer'] == 0 and value['rank'] == rank]
        require(len(layers) == 1, 'one layer-zero registration per rank')
        roots = [value['buffer'] for value in layers[0]['weights'] if value['kind'] == 'output_projection']
        require(len(roots) == 1 and roots[0]['id'] == row['key']['id']
                and roots[0]['elements'] == 4096 * 2048 and roots[0]['element_bytes'] == 2,
                'registered rank-column weight extent')
        matches = [value for value in uploads['uploads'] if value['key'] == row['key']]
        require(len(matches) == 1 and matches[0]['bytes'] == row['bytes'] == 16777216
                and bytes(matches[0]['sha256']).hex() == row['sha256'], 'actual uploaded weight identity')
    request = json.loads(native[NATIVE + 'native/request.json'])
    require(bytes(request['layer']['prefix_tiles_image']['sha256']).hex()
            == manifest['historical_kernel_inputs']['emitted/artifact.hsaco']['sha256'], 'captured reviewed image')
    counts = dict(rows=len(report['rows']), rank_partials=2 * len(report['rows']),
                  native_partials_match_fixed_order=0, native_projection_matches_ideal=0,
                  framework_projection_matches_ideal=0, native_projection_matches_framework=0,
                  native_residual_matches_conditional_boundary=0, native_residual_matches_framework=0,
                  rounded_products=0, subnormal_products=0, subnormal_sums=0)
    rank_ratios, combined_ratios, different, residual_different, native_nonideal = [], [], [], [], []
    require(len(report['rows']) == 4096, 'all output rows')
    for index, row in enumerate(report['rows']):
        require(row['row'] == index and len(row['ranks']) == 2
                and row['shard_byte_offset'] == report['rows'][0]['shard_byte_offset'] + 8192 * index,
                'exact row order and stride')
        total = int(row['exact_sum_units'])
        require(total == sum(int(rank['exact_sum_units']) for rank in row['ranks']), 'exact rank sums combine')
        for rank, value in enumerate(row['ranks']):
            actual, replayed = int(value['native_f32'], 16), int(value['replay_f32'], 16)
            exact, absolute = int(value['exact_sum_units']), int(value['absolute_sum_units'])
            error = abs(units(actual, 23) - exact)
            require(value['rank'] == rank and actual == partials[rank][index]
                    and absolute >= abs(exact) and error == int(value['native_distance_units'])
                    and abs(units(replayed, 23) - exact) == int(value['replay_distance_units'])
                    and value['native_matches_fixed_order'] == (actual == replayed), 'rank encoding/distance joins')
            counts['native_partials_match_fixed_order'] += actual == replayed
            for name in ('rounded_products', 'subnormal_products', 'subnormal_sums'):
                require(value[name] == 0, 'actual replay reports no exceptional rounding case')
            require(error * ((1 << 24) - 38) <= 38 * absolute, 'conditional gamma38 integer inequality')
            if absolute:
                rank_ratios.append((Fraction(error * ((1 << 24) - 38), 38 * absolute), index, rank))
        sum_word = nearest(units(partials[0][index], 23) + units(partials[1][index], 23), 23)
        require(sum_word == int(row['native_tp_sum_f32'], 16) == int(row['replay_tp_sum_f32'], 16),
                'independently rounded TP sum')
        native_word, ideal = nearest(units(sum_word, 23), 7), nearest(total, 7)
        framework, embedding = expected['o-projection'][index], expected['embedding'][index]
        require((native_word, ideal, framework, embedding) == tuple(int(row[name], 16) for name in
                ('native_projection_bf16', 'ideal_bf16', 'framework_projection_bf16', 'embedding_bf16'))
                and int(row['replay_projection_bf16'], 16) == native_word, 'independent BF16 nearest encodings')
        require(row['native_matches_ideal'] == (native_word == ideal)
                and row['framework_matches_ideal'] == (framework == ideal)
                and row['native_matches_framework'] == (native_word == framework), 'comparison flags')
        for name, value in (('native_projection_matches_ideal', native_word == ideal),
                            ('framework_projection_matches_ideal', framework == ideal),
                            ('native_projection_matches_framework', native_word == framework)):
            counts[name] += value
        a, b = abs(units(native_word, 7) - total), abs(units(framework, 7) - total)
        require(a == int(row['native_distance_units']) and b == int(row['framework_distance_units'])
                and row['closer_to_exact'] == ('equal' if a == b else 'native' if a < b else 'framework'),
                'exact full-dot distances')
        if native_word != framework:
            different.append(index)
            midpoint = row['differing_projection_midpoint']
            numerator = 2 * total - units(native_word, 7) - units(framework, 7)
            require(int(midpoint['signed_offset_numerator']) == numerator
                    and int(midpoint['distance_numerator']) == abs(numerator)
                    and midpoint['scale_power_of_two'] == -267
                    and midpoint['left_even'] == (native_word & 1 == 0)
                    and midpoint['right_even'] == (framework & 1 == 0), 'exact midpoint and parity')
        else:
            require(row['differing_projection_midpoint'] is None, 'equal-value midpoint absent')
        if native_word != ideal:
            native_nonideal.append(index)
        output = nearest(units(nearest(units(native_word, 7) + units(embedding, 7), 23), 23), 7)
        reference_output = nearest(units(nearest(units(framework, 7) + units(embedding, 7), 23), 23), 7)
        require(reference_output == expected['first-residual'][index] == int(row['framework_residual_bf16'], 16)
                and output == int(row['conditional_residual_bf16'], 16) == int(row['replay_residual_bf16'], 16),
                'materialized projection/residual boundary')
        for rank in (0, 1):
            observed = residuals[rank][index]
            require(observed == int(row['native_residual_bf16'][rank], 16)
                    and row['native_residual_matches_conditional_boundary'][rank] == (observed == output)
                    and row['native_residual_matches_framework'][rank] == (observed == reference_output),
                    'direct captured residual joins')
            counts['native_residual_matches_conditional_boundary'] += observed == output
            counts['native_residual_matches_framework'] += observed == reference_output
        if any(residuals[rank][index] != reference_output for rank in (0, 1)):
            residual_different.append(index)
        absolute = sum(int(value['absolute_sum_units']) for value in row['ranks'])
        error = abs(units(sum_word, 23) - total)
        require(error * ((1 << 24) - 39) <= 39 * absolute, 'conditional gamma39 integer inequality')
        if absolute:
            combined_ratios.append((Fraction(error * ((1 << 24) - 39), 39 * absolute), index))
    require(counts == report['counts'] and different == report['disagreement_rows']
            and residual_different == report['residual_disagreement_rows'], 'recomputed aggregate counts')
    for key in ('current_guarded_internal_stages_observed', 'framework_accumulator_order_proven',
                'full_model_correctness', 'gpu_execution', 'model_loaded',
                'native_materialized_projection_directly_captured', 'native_residual_input_directly_captured',
                'numerical_acceptance', 'performance_claim', 'production_authority', 'semantic_bug_proven',
                'sustained_2048_256', 'tests_executed'):
        require(report[key] is False, 'retained scope/nonclaim: ' + key)
    for pin in PINS.values():
        read(*pin)
    rank_max, combined_max = max(rank_ratios), max(combined_ratios)
    result = dict(schema='ferric-p228-o-projection-independent-data-audit-v1', passed=True,
                  actual_report_sha256=PINS['report'][2], actual_tests_sha256=PINS['tests'][2],
                  locally_rehashed_consumed_bodies=30, inherited_manifest_only_bodies=2,
                  counts=counts, disagreement_rows=different, residual_disagreement_rows=residual_different,
                  native_nonideal_rows=native_nonideal,
                  conditional_gamma38=dict(rank_partials=8192, max_error_over_bound=str(rank_max[0]),
                                           approximate_ratio=float(rank_max[0]), row=rank_max[1], rank=rank_max[2]),
                  conditional_gamma39=dict(rows=4096, max_error_over_bound=str(combined_max[0]),
                                           approximate_ratio=float(combined_max[0]), row=combined_max[1]),
                  reported_exact_dots_independently_recomputed=False, full_model_shard_locally_rehashed=False,
                  fixed_order_full_dot_replay_reexecuted=False, project_imports=False,
                  numerical_acceptance=False, framework_accumulator_order_inferred=False,
                  gpu_execution=False, model_execution=False, performance_claim=False)
    print(json.dumps(result, sort_keys=True, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
