"""Render retained V2 raw clocks and dispatch ticks without calibration or alignment."""
import argparse
from collections import Counter, defaultdict
from fractions import Fraction
import hashlib
from html import escape
import json
import math
from pathlib import Path
import sys

STAGES = ('embedding', 'copy', 'prefix', 'post-attention-residual', 'mlp',
          'post-mlp-residual', 'final-norm', 'head', 'argmax')
FALSE = ('calibrated_nanoseconds', 'cross_device_clock_alignment', 'overlap_claim',
         'performance_claim', 'numerical_acceptance', 'full_model_acceptance', 'production_authority')
CLOCK_FALSE = ('clock_domain_validated', *FALSE)


def require(value, message):
    if not value:
        raise ValueError(message)


def pairs(rows):
    value = {}
    for key, item in rows:
        require(key not in value, 'duplicate JSON member')
        value[key] = item
    return value


def load(path, expected):
    with path.open('rb') as stream:
        raw = stream.read((8 << 20) + 1)
    require(0 < len(raw) <= 8 << 20 and hashlib.sha256(raw).hexdigest() == expected, 'exact bounded input')
    return json.loads(raw, object_pairs_hook=pairs)


def uint(value):
    require(type(value) is int and 0 <= value < 1 << 64, 'unsigned integer, not boolean/float')
    return value


def median(values):
    ordered = sorted(values)
    return Fraction(ordered[(len(ordered) - 1) // 2] + ordered[len(ordered) // 2], 2)


def number(value):
    value = Fraction(value)
    return str(value.numerator) if value.denominator == 1 else str(value.numerator // 2) + '.5'


def write(path, raw):
    with path.open('x', encoding='utf-8') as stream:
        stream.write(raw)


def svg(width, height, title):
    return [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}" role="img">', f'<title>{escape(title)}</title>',
            '<style>text{font-family:Arial,sans-serif;fill:#20242a;font-size:12px}</style>',
            f'<rect width="{width}" height="{height}" fill="white"/>']


def text(parts, x, y, label, size=12):
    parts.append(f'<text x="{x}" y="{y}" style="font-size:{size}px">{escape(str(label))}</text>')


def ranges(groups, ranks):
    parts = svg(1060, 730, 'Raw tick ranges and medians, separate device panels')
    text(parts, 20, 26, 'Raw tick ranges and medians', 20)
    text(parts, 20, 49, 'No nanosecond conversion or cross-device alignment. Position 0 and zero deltas are retained.')
    for rank in range(2):
        top = 88 + rank * 320
        values = [v for (r, _), items in groups.items() if r == rank for v in items]
        decades = max(1, math.ceil(math.log10(max(values) + 1)))
        xpos = lambda value: 250 + 735 * math.log10(float(value) + 1) / decades
        text(parts, 20, top, f'Rank {rank}: GPU {ranks[rank]["unique_id"]}; queue epoch {ranks[rank]["queue_epoch"]}', 14)
        for exponent in range(decades + 1):
            x = xpos(10 ** exponent - 1)
            parts.append(f'<path d="M{x:.2f},{top+12} V{top+250}" stroke="#e0e3e7"/>')
            text(parts, round(x - 7, 2), top + 270, '0' if exponent == 0 else f'10^{exponent}-1')
        for index, stage in enumerate(STAGES):
            y = top + 28 + index * 25
            items = groups.get((rank, stage), [])
            text(parts, 20, y + 4, stage)
            if not items:
                text(parts, 255, y + 4, 'not dispatched on this rank')
                continue
            low, high, middle = min(items), max(items), median(items)
            parts.append(f'<path d="M{xpos(low):.2f},{y} H{xpos(high):.2f}" stroke="#25816d" stroke-width="3"/>')
            parts.append(f'<circle cx="{xpos(middle):.2f}" cy="{y}" r="4" fill="#bc4b38">'
                         f'<title>{escape(stage)}: n={len(items)}, min={low}, median={number(middle)}, max={high}</title></circle>')
        text(parts, 250, top + 292, 'Axis: log10(1 + raw ticks); line = min/max, point = median')
    return '\n'.join(parts + ['</svg>', ''])


def heatmaps(rows):
    parts = svg(1080, 1210, 'Raw prefix and MLP tick deltas by layer and position')
    text(parts, 20, 27, 'Layer and position: raw dispatch ticks', 20)
    text(parts, 20, 50, 'Each panel has its own scale. No device alignment or calibrated duration is implied.')
    for rank in range(2):
        for column, stage in enumerate(('prefix', 'mlp')):
            left, top = 65 + column * 520, 104 + rank * 555
            cells = {(row['layer'], row['position']): row['end_tick'] - row['start_tick']
                     for row in rows if row['rank'] == rank and row['stage'] == stage}
            require(set(cells) == {(layer, position) for layer in range(36) for position in range(4)}, 'complete heatmap')
            low, high = min(cells.values()), max(cells.values())
            text(parts, left, top - 30, f'Rank {rank} / {stage}: {low} to {high} raw ticks', 14)
            for position in range(4):
                text(parts, left + position * 94 + 22, top - 9, f'pos {position}')
            for layer in range(36):
                text(parts, left - 34, top + layer * 13 + 11, layer)
                for position in range(4):
                    value = cells[layer, position]
                    weight = (value - low) / (high - low) if high != low else 0
                    rgb = tuple(round(a + weight * (b - a)) for a, b in zip((235, 243, 240), (26, 112, 99)))
                    fill = '#%02x%02x%02x' % rgb
                    parts.append(f'<rect x="{left+position*94}" y="{top+layer*13}" width="92" height="12" fill="{fill}">'
                                 f'<title>rank={rank}, {stage}, layer={layer}, position={position}, ticks={value}</title></rect>')
            text(parts, left, top + 490, 'Rows: layers 0-35; lighter = lower, darker = higher')
    return '\n'.join(parts + ['</svg>', ''])


def clock_samples(value, device):
    require(set(value) == {'schema', 'raw', 'samples', 'raw_clock_counters', *CLOCK_FALSE}
        and value['schema'] == 'FerricPrefixDecodeDeviceClockObservationV2'
        and value['raw_clock_counters'] is True
        and all(value[key] is False for key in CLOCK_FALSE), 'raw-only clock V2 report')
    samples = value['samples']
    require(type(samples) is list and len(samples) == 16, 'sixteen native samples')
    fields = {'generation', 'position', 'endpoint', 'rank', 'row_boundary', 'group_incarnation',
        'unique_id', 'queue_epoch', 'gpu_id', 'gpu_clock_counter', 'cpu_clock_counter',
        'system_clock_counter', 'system_clock_frequency_hz', 'host_started_ns', 'host_finished_ns'}
    prior_finished = 0
    for index, sample in enumerate(samples):
        require(type(sample) is dict and set(sample) == fields, 'closed native sample')
        for key in fields - {'endpoint'}:
            uint(sample[key])
        position, slot = divmod(index, 4)
        rank, post = slot % 2, slot >= 2
        require(sample['generation'] == position + 1 and sample['position'] == position
            and sample['rank'] == rank and sample['endpoint'] == ('post' if post else 'pre')
            and sample['row_boundary'] == (position + int(post)) * 293
            and sample['group_incarnation'] == device['group_incarnation']
            and sample['unique_id'] == device['ranks'][rank]['unique_id']
            and sample['queue_epoch'] == device['ranks'][rank]['queue_epoch']
            and sample['gpu_id'] < 1 << 32 and sample['system_clock_frequency_hz'] > 0,
            'actual sample endpoint and rank identity')
        require(prior_finished <= sample['host_started_ns'] <= sample['host_finished_ns'],
                'ordered process-local host brackets')
        if index == 1:
            require(sample['gpu_id'] != samples[0]['gpu_id'], 'distinct KFD device IDs')
        if index >= 2:
            require(sample['gpu_id'] == samples[rank]['gpu_id']
                and sample['system_clock_frequency_hz'] == samples[rank]['system_clock_frequency_hz'],
                'stable per-rank identity and system frequency')
        prior_finished = sample['host_finished_ns']
    deltas = []
    for position in range(4):
        for rank in range(2):
            pre, post = samples[position * 4 + rank], samples[position * 4 + 2 + rank]
            delta = dict(position=position, generation=position + 1, rank=rank,
                unique_id=pre['unique_id'], queue_epoch=pre['queue_epoch'], gpu_id=pre['gpu_id'],
                pre_sample=position * 4 + rank, post_sample=position * 4 + 2 + rank,
                host_bracket_span_ns=post['host_finished_ns'] - pre['host_started_ns'],
                pre_sample_host_span_ns=pre['host_finished_ns'] - pre['host_started_ns'],
                post_sample_host_span_ns=post['host_finished_ns'] - post['host_started_ns'])
            for key in ('gpu_clock_counter', 'cpu_clock_counter', 'system_clock_counter'):
                delta[key + '_signed_difference'] = post[key] - pre[key]
                delta[key + '_decreased'] = post[key] < pre[key]
            deltas.append(delta)
    return samples, deltas


def clock_markdown(samples, deltas):
    lines = ['', '## Raw Clock Samples', '',
        'These are sixteen actual KFD samples, taken sequentially by rank before and after each forward.',
        'GPU, CPU and system counters are distinct raw values. The system frequency is not a GPU tick frequency.',
        'Host brackets use one process-local Instant origin and include device currentness checks.',
        'They do not establish simultaneous sampling or a GPU dispatch clock-domain relationship.', '',
        '| Position | Rank | Boundary | Recorded rows | GPU counter | CPU counter | System counter | Host bracket ns |',
        '| ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |']
    for sample in samples:
        lines.append('| {position} | {rank} | {endpoint} | {row_boundary} | {gpu_clock_counter} | '
            '{cpu_clock_counter} | {system_clock_counter} | {host_started_ns}..{host_finished_ns} |'.format(**sample))
    for rank in range(2):
        lines += ['', 'Rank {}: KFD GPU ID {}, reported system-counter frequency {} Hz.'.format(
            rank, samples[rank]['gpu_id'], samples[rank]['system_clock_frequency_hz'])]
    lines += ['', '## Same-Device Counter Differences', '',
        'Signed post-minus-pre integer differences only, without wrap correction or unit conversion.',
        'A negative value is retained and flagged in JSON; it is not silently repaired or treated as a duration.',
        'The host span includes both sampling calls and all intervening host/device work, not GPU compute alone.', '',
        '| Position | Rank | GPU difference (raw) | CPU difference (raw) | System difference (raw) | Host bracket span ns |',
        '| ---: | ---: | ---: | ---: | ---: | ---: |']
    for row in deltas:
        lines.append('| {position} | {rank} | {gpu_clock_counter_signed_difference} | '
            '{cpu_clock_counter_signed_difference} | {system_clock_counter_signed_difference} | '
            '{host_bracket_span_ns} |'.format(**row))
    return lines


def main():
    require(not sys.flags.optimize, 'ordinary reporting Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('complete', type=Path)
    parser.add_argument('complete_sha256')
    parser.add_argument('sidecar', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    complete = load(args.complete, args.complete_sha256)
    require(complete['schema'] == 'ferric-p228-device-clock-gpu-v1' and complete['passed'] is True
        and complete['failures'] == [] and complete['native_attempts'] == 1 and complete['retries'] == 0,
        'actual successful one-attempt observation only')
    require(len(complete['before_audits']) == len(complete['after_audits']) == 3, 'six device audits')
    checked = complete['checked']
    require(checked['raw_rows'] == 1172 and checked['rank_packets'] == [592, 580]
        and checked['clock_samples'] == 16 and checked['raw_clock_counters'] is True
        and complete['raw_clock_counters'] is True
        and checked['captured_payloads'] == 4 and checked['compared_tensor_rows'] == 152
        and checked['all_payloads_tokens_and_tensors_equal'] is True
        and checked['recorded_close_and_owner_reap_checked'] is True
        and all(complete[key] is checked[key] is False for key in CLOCK_FALSE), 'checked coverage and raw-only scope')
    clock = load(args.sidecar, complete['device_sidecar']['sha256'])
    device = clock['raw']
    require(args.sidecar.stat().st_size == complete['device_sidecar']['bytes'] <= 2 << 20
        and device['schema'] == 'FerricPrefixDecodeDeviceObservationV1' and device['native_closed'] is True
        and device['raw_completion_ticks'] is True and all(device[key] is False for key in FALSE), 'authentic raw-only sidecar')
    samples, counter_deltas = clock_samples(clock, device)
    rows, ranks = device['rows'], device['ranks']
    require(len(rows) == 1172 and len(ranks) == 2 and device['final_dispatches'] == [592, 580], 'complete raw census')
    groups, host = defaultdict(list), defaultdict(list)
    for index, row in enumerate(rows):
        rank, position = uint(row['rank']), uint(row['position'])
        require(rank < 2 and position == index // 293 and row['generation'] == position + 1
            and row['stage'] in STAGES and row['unique_id'] == ranks[rank]['unique_id']
            and row['queue_epoch'] == ranks[rank]['queue_epoch'], 'same per-device generation')
        start, end = uint(row['start_tick']), uint(row['end_tick'])
        require(0 < start <= end, 'ordered nonzero raw tick endpoints')
        groups[rank, row['stage']].append(end - start)
        host[rank, row['stage']].append(uint(row['host_elapsed_ns']))
    require(Counter(row['rank'] for row in rows) == {0: 592, 1: 580}, 'per-rank counts')
    expected = {(rank, stage): 144 for rank in range(2)
                for stage in ('prefix', 'post-attention-residual', 'mlp', 'post-mlp-residual')}
    expected.update({(0, stage): 4 for stage in ('embedding', 'final-norm', 'head', 'argmax')})
    expected[1, 'copy'] = 4
    require({key: len(values) for key, values in groups.items()} == expected, 'exact 13-group stage census')
    summaries = []
    lines = ['# Raw Clock And Completion Tick Report', '',
        'One four-forward teacher-forced observation; not the 2,048/256 workload.',
        'Raw tick deltas are not nanoseconds, calibrated durations, cross-device coordinates or a throughput result.', '',
        '| Rank | Stage | Count | Min ticks | Median ticks | Max ticks | Zero deltas |',
        '| --- | --- | ---: | ---: | ---: | ---: | ---: |']
    for rank in range(2):
        for stage in STAGES:
            values = groups.get((rank, stage), [])
            if not values:
                continue
            summary = dict(rank=rank, unique_id=ranks[rank]['unique_id'], queue_epoch=ranks[rank]['queue_epoch'],
                stage=stage, count=len(values), min_ticks=min(values), median_ticks=number(median(values)),
                max_ticks=max(values), zero_deltas=values.count(0))
            summaries.append(summary)
            lines.append(f'| {rank} | {stage} | {len(values)} | {min(values)} | {number(median(values))} | {max(values)} | {values.count(0)} |')
    lines += ['', '![Per-device ranges and medians](raw-tick-ranges.svg)', '',
              '![Layer and position raw ticks](raw-tick-heatmaps.svg)', '', '## Separate Host Intervals', '',
              'These inclusive host intervals are recorded in nanoseconds and joined to the native Controls.',
              'They are not GPU durations and must not be subtracted from the tick values.', '',
              '| Rank | Stage | Min host ns | Median host ns | Max host ns |',
              '| --- | --- | ---: | ---: | ---: |']
    for rank, stage in groups:
        values = host[rank, stage]
        lines.append(f'| {rank} | {stage} | {min(values)} | {number(median(values))} | {max(values)} |')
    lines += clock_markdown(samples, counter_deltas)
    args.output.mkdir(exist_ok=False)
    write(args.output / 'raw-ticks.json', json.dumps(dict(complete_sha256=args.complete_sha256,
        schema='ferric-p228-device-clock-report-v1',
        device_sidecar=complete['device_sidecar'], raw_rows=len(rows), stages=summaries,
        clock_samples=len(samples), raw_clock_samples=samples, same_device_counter_differences=counter_deltas,
        report_program_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        raw_clock_counters=True, raw_completion_ticks=True,
        **{key: False for key in CLOCK_FALSE}), indent=2, sort_keys=True) + '\n')
    write(args.output / 'raw-ticks.md', '\n'.join(lines) + '\n')
    write(args.output / 'raw-tick-ranges.svg', ranges(groups, ranks))
    write(args.output / 'raw-tick-heatmaps.svg', heatmaps(rows))
    print(json.dumps(dict(rows=len(rows), clock_samples=len(samples), stage_groups=len(groups), zero_deltas=sum(row['end_tick'] == row['start_tick'] for row in rows))))


if __name__ == '__main__':
    main()

