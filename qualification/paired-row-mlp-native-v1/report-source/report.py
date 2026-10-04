"""Bounded retained-data comparison; never launch code or calibrate raw GPU clocks."""
import argparse
from collections import defaultdict
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PLOT = E / 'p228-device-clock-report-v1/report.py'
PLOT_SHA = '2dff8e703bde0e2370ada6190d635e023f4929ec502a70d8a536979d8a7898d3'
BASELINE = (902162, 'e34189597dc7db7a7c325040f5381932e84390c1a2cf32055830858cb9278ddb')
DOWN2 = (33112, '65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449')
FALSE = ('calibrated_nanoseconds', 'cross_device_clock_alignment', 'overlap_claim',
         'performance_claim', 'numerical_acceptance', 'full_model_acceptance', 'production_authority',
         'clock_domain_validated')
STAGES = ('embedding', 'copy', 'prefix', 'post-attention-residual', 'mlp',
          'post-mlp-residual', 'final-norm', 'head', 'argmax')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON member')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _value: require(False, 'nonfinite JSON'))


def digest(value):
    if type(value) is str:
        require(re.fullmatch('[0-9a-f]{64}', value), 'SHA256 spelling')
        return value
    require(type(value) is list and len(value) == 32
            and all(type(item) is int and 0 <= item <= 255 for item in value), 'wire SHA256 bytes')
    return bytes(value).hex()


def uint(value):
    require(type(value) is int and 0 <= value < 1 << 64, 'unsigned integer, not float/bool')
    return value


def read(path, sha=None, expected=None):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path and path.is_file(), 'canonical input')
    require(path.stat().st_size <= 8 << 20, 'bounded input')
    raw = path.read_bytes()
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(len(raw) <= 8 << 20 and (sha is None or pin['sha256'] == sha)
            and (expected is None or pin == expected), 'actual retained pin')
    return pin, raw


def number(value):
    value = Fraction(value)
    require(value.denominator in (1, 2), 'integer/half-integer median or signed difference')
    sign = '-' if value < 0 else ''
    numerator = abs(value.numerator)
    return sign + (str(numerator) if value.denominator == 1 else str(numerator // 2) + '.5')


def median(values):
    require(values, 'nonempty median')
    values = sorted(values)
    return Fraction(values[(len(values) - 1) // 2] + values[len(values) // 2], 2)


def coordinate(index):
    require(type(index) is int and 0 <= index < 1172, 'bounded packet index')
    position, slot = divmod(index, 293)
    if slot == 0:
        return position, 'embedding', None, 0
    if slot == 1:
        return position, 'copy', None, 1
    if slot < 290:
        layer, local = divmod(slot - 2, 8)
        return position, ('prefix', 'post-attention-residual', 'mlp', 'post-mlp-residual')[local // 2], layer, local % 2
    return position, ('final-norm', 'head', 'argmax')[slot - 290], None, 0


def selected_rows(raw, request, selected):
    require(raw['schema'] == 'FerricPrefixDecodeDeviceObservationV1' and raw['native_closed'] is True
            and raw['raw_completion_ticks'] is True and raw['shared_full_currentness'] is True
            and raw['cache_kernel_admission'] is False and raw['operational_currentness'] is False
            and all(raw[key] is False for key in FALSE if key != 'clock_domain_validated'), 'closed raw-only observation')
    require(raw['final_dispatches'] == [592, 580] and len(raw['rows']) == 1172
            and len(raw['ranks']) == 2 and raw['bootstrap']['device_ids'] == request['device_ids'], 'exact ranked raw census')
    require(raw['bootstrap']['scope']['session'] == request['session']
            and digest(raw['worker_sha256']) == digest(request['worker']['sha256']), 'actual per-run identity')
    part = raw['bootstrap']['tiles_image']
    require((part['bytes'], digest(part['sha256'])) == selected
            and digest(raw['images']['mlp']) == selected[1], 'selected explicit MLP image in bootstrap and sidecar')
    ticks, host, packets = defaultdict(list), defaultdict(list), [0, 0]
    for index, row in enumerate(raw['rows']):
        position, stage, layer, rank = coordinate(index)
        owner = raw['ranks'][rank]
        require(row['position'] == position and row['generation'] == position + 1
                and row['stage'] == stage and row['layer'] == layer and row['rank'] == rank
                and owner['rank'] == rank and owner['unique_id'] == request['device_ids'][rank]
                and row['unique_id'] == owner['unique_id'] and row['queue_epoch'] == owner['queue_epoch']
                and row['group_incarnation'] == raw['group_incarnation']
                and row['packet_id'] == packets[rank] and row['signal_generation'] == packets[rank] + 1,
                'exact stage/layer/position/rank/packet identity')
        image = 'residual' if stage.endswith('-residual') else stage if stage in ('prefix', 'mlp', 'copy') else 'tail'
        require(digest(row['image_sha256']) == digest(raw['images'][image]), 'selected image on every dispatch')
        if stage == 'mlp':
            require(digest(row['image_sha256']) == selected[1]
                    and row['entry'] == 'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2', 'exact Down2/baseline MLP entry on all 288 rows')
        start, end = uint(row['start_tick']), uint(row['end_tick'])
        require(0 < start <= end, 'ordered raw endpoints; zero deltas remain allowed')
        ticks[rank, stage].append(end - start)
        host[rank, stage].append(uint(row['host_elapsed_ns']))
        packets[rank] += 1
    require(packets == [592, 580] and len(ticks[0, 'mlp']) == len(ticks[1, 'mlp']) == 144,
            'full actual dispatch and MLP-image census')
    return ticks, host


def case(path, sha, role, plot):
    complete_pin, raw = read(path, sha)
    complete = parse(raw)
    schema = 'ferric-p228-device-clock-gpu-v1' if role == 'baseline' else 'ferric-p228-down2-clock-gpu-v1'
    require(complete['schema'] == schema and complete['passed'] is True and complete['failures'] == []
            and complete['native_attempts'] == 1 and complete['retries'] == 0
            and len(complete['before_audits']) == len(complete['after_audits']) == 3
            and all(complete[key] is False for key in FALSE), 'actual closed successful engineering row')
    if role == 'baseline':
        require((complete_pin['bytes'], complete_pin['sha256']) == BASELINE, 'exact genuine clock baseline')
    checked = complete['checked']
    require(checked['raw_rows'] == 1172 and checked['clock_samples'] == 16 and checked['rank_packets'] == [592, 580]
            and checked['captured_payloads'] == 4 and checked['compared_tensor_rows'] == 152
            and complete['raw_clock_counters'] is True and checked['raw_clock_counters'] is True
            and checked['all_payloads_tokens_and_tensors_equal'] is True
            and checked['recorded_close_and_owner_reap_checked'] is True
            and checked['device_sidecar'] == complete['device_sidecar']
            and checked['request'] == complete['request']
            and all(checked[key] is False for key in FALSE), 'retained checked raw-clock/invariance result')
    pins = [complete_pin]
    def pinned(record):
        pin, raw = read(record['path'], expected=record)
        pins.append(pin)
        return parse(raw)
    external = pinned(complete['request'])
    require(external['schema'] == 'FerricFinitePrefixDecodeDeviceClockRequestV2', 'same explicit clock request')
    request = external['decode']
    selected = (request['tiles_image']['bytes'], digest(request['tiles_image']['sha256']))
    image = dict(request['tiles_image'], sha256=selected[1])
    image_pin, _ = read(image['path'], expected=image)
    pins.append(image_pin)
    if role == 'candidate':
        require(selected == DOWN2 and complete['down2_image'] == image, 'actual candidate image pin, not a guessed result SHA')
    clock = pinned(complete['device_sidecar'])
    require(clock['schema'] == 'FerricPrefixDecodeDeviceClockObservationV2'
            and clock['raw_clock_counters'] is True and all(clock[key] is False for key in FALSE), 'raw-only clock schema')
    native = clock['raw']
    require(digest(native['profile_sha256']) == checked['structural']['profile_sha256']
            and digest(native['transcript_sha256']) == checked['structural']['transcript_sha256']
            and native['child_pid'] == checked['structural']['child_pid'], 'actual checked native sidecar identity')
    ticks, host = selected_rows(native, request, selected)
    samples, differences = plot.clock_samples(clock, native)
    return dict(pin=complete_pin, complete=complete, request=request, selected_image=image,
                clock=clock, rows=native['rows'], ranks=native['ranks'], ticks=ticks, host=host,
                samples=samples, counter_differences=differences, pins=pins)


def same_cases(old, new):
    require(new['complete']['baseline'] == old['pin']
            and new['complete']['selected_runtime'] == old['complete']['selected_runtime'], 'actual baseline and unchanged native runtime')
    omitted = {'session', 'evidence_directory', 'tiles_image'}
    a, b = old['request'], new['request']
    require(set(a) == set(b) and {k: v for k, v in a.items() if k not in omitted} == {k: v for k, v in b.items() if k not in omitted}
            and a['session'] != b['session'] and a['mode'] == b['mode'] == 'teacher_forced', 'image-only TF4 workload comparison')
    require(set(old['ticks']) == set(new['ticks']) and old['selected_image'] != new['selected_image'], 'matching stage census and actual image change')


def summary(old, new):
    rows = []
    for rank in range(2):
        for stage in STAGES:
            key = rank, stage
            if key not in old['ticks']:
                continue
            require(len(old['ticks'][key]) == len(new['ticks'][key]), 'equal descriptive sample counts')
            value = dict(rank=rank, stage=stage, count=len(old['ticks'][key]))
            for metric in ('ticks', 'host'):
                a, b = old[metric][key], new[metric][key]
                value[metric] = dict(baseline_min=min(a), baseline_median=number(median(a)), baseline_max=max(a),
                    candidate_min=min(b), candidate_median=number(median(b)), candidate_max=max(b),
                    signed_median_difference=number(median(b) - median(a)),
                    baseline_zero_deltas=a.count(0), candidate_zero_deltas=b.count(0))
            rows.append(value)
    return rows


def markdown(rows):
    lines = ['# Down2 Raw-Clock Comparison', '',
        'One genuine baseline and one Down2 four-forward teacher-forced observation.',
        'These are descriptive samples, not a speedup, throughput result, calibrated GPU duration or overlap claim.',
        'Raw ticks have no established cross-run/device frequency or aligned origin. Signed differences are arithmetic only.', '',
        '## Per-Device Raw Ticks', '',
        '| Rank | Stage | Count each | Baseline median | Down2 median | Signed difference | Baseline zeros | Down2 zeros |',
        '| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |']
    for row in rows:
        value = row['ticks']
        lines.append('| {rank} | {stage} | {count} | {baseline_median} | {candidate_median} | {signed_median_difference} | {baseline_zero_deltas} | {candidate_zero_deltas} |'.format(**row, **value))
    lines += ['', '## Inclusive Host Intervals', '',
        'Host intervals are recorded nanoseconds, not GPU durations. They include currentness checks and host dispatch work.', '',
        '| Rank | Stage | Baseline median ns | Down2 median ns | Signed difference ns |',
        '| ---: | --- | ---: | ---: | ---: | ---: |']
    for row in rows:
        lines.append('| {rank} | {stage} | {baseline_median} | {candidate_median} | {signed_median_difference} |'.format(**row, **row['host']))
    lines += ['', '## Separate Plots', '',
        'Each plot/panel has its own labeled scale; no aligned device timeline is inferred.', '',
        '![Baseline raw tick ranges](baseline-ranges.svg)', '', '![Down2 raw tick ranges](down2-ranges.svg)', '',
        '![Baseline layer-position raw ticks](baseline-heatmaps.svg)', '', '![Down2 layer-position raw ticks](down2-heatmaps.svg)', '',
        'The accepted observer checked four whole payloads and 152 tensor rows for bitwise invariance.',
        'This renderer reads that pinned result and rechecks raw identities; it does not rerun independent numerical validation.',
        'No sustained 2,048/256 decode or 700 tokens/s result is implied.', '']
    return '\n'.join(lines)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary reporting Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('baseline', type=Path)
    parser.add_argument('baseline_sha256')
    parser.add_argument('candidate', type=Path)
    parser.add_argument('candidate_sha256')
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    for value in (args.baseline_sha256, args.candidate_sha256):
        digest(value)
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        ceiling = min([cap] + [value for value in old if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (ceiling, ceiling))
    source_pin, _ = read(Path(__file__).resolve())
    plot_pin, raw = read(PLOT, PLOT_SHA)
    plot = types.ModuleType('down2_retained_plot'); plot.__file__ = str(PLOT)
    exec(compile(raw, str(PLOT), 'exec'), plot.__dict__)
    old = case(args.baseline, args.baseline_sha256, 'baseline', plot)
    new = case(args.candidate, args.candidate_sha256, 'candidate', plot)
    same_cases(old, new)
    rows = summary(old, new)
    records = dict(schema='ferric-p228-down2-clock-report-v1', baseline=old['pin'], candidate=new['pin'],
        selected_baseline_image=old['selected_image'], selected_down2_image=new['selected_image'],
        stage_summaries=rows, raw_rows_per_case=1172, mlp_rows_per_case=288, clock_samples_per_case=16,
        program=source_pin, plotting_helper=plot_pin, descriptive_only=True, gpu_execution=False,
        speedup_claim=False, throughput_claim=False, independent_numerical_replay_performed=False,
        baseline_clock_differences=old['counter_differences'], candidate_clock_differences=new['counter_differences'],
        **{key: False for key in FALSE})
    output = {
        'comparison.json': json.dumps(records, indent=2, sort_keys=True, allow_nan=False) + '\n',
        'comparison.md': markdown(rows),
        'baseline-ranges.svg': plot.ranges(old['ticks'], old['ranks']),
        'down2-ranges.svg': plot.ranges(new['ticks'], new['ranks']),
        'baseline-heatmaps.svg': plot.heatmaps(old['rows']),
        'down2-heatmaps.svg': plot.heatmaps(new['rows']),
    }
    pins = [source_pin, plot_pin, *old['pins'], *new['pins']]
    for pin in pins:
        read(pin['path'], expected=pin)
    require(args.output.is_absolute() and args.output.parent.resolve(strict=True) == args.output.parent
            and not os.path.lexists(args.output), 'fresh canonical report output')
    args.output.mkdir(mode=0o700)
    for name, body in output.items():
        require(len(body.encode()) <= 8 << 20, 'bounded rendered output')
        with (args.output / name).open('x', encoding='utf-8') as stream:
            stream.write(body)
    for pin in pins:
        read(pin['path'], expected=pin)
    print(json.dumps(dict(stage_groups=len(rows), output=str(args.output), descriptive_only=True,
                          files={name: read(args.output / name)[0] for name in output})), flush=True)


if __name__ == '__main__':
    main()
