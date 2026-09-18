#!/usr/bin/env python3
"""Sanitize one accepted timestamp cohort and reproduce its offline tables/plots."""
import argparse
from collections import Counter, defaultdict
import csv
import hashlib
from html import escape
import io
import json
import math
import os
from pathlib import Path
import re
import stat

ANALYSIS_SHA = 'bb73534910b1a87ee9b2c64bd28cf6c5ca8f55991455ace85bf2d581be59f61e'
PLAN_SHA = 'a7288b925c2fd05e622b509b9d6bd6aa8c3e3648a576a6d3ba4ccbe36bdbd08e'
CAPTURE_SHA = '6c9e89737c956dbedc61de96d830f23c6c970b7a0663f04043cc16e35833d6a8'
ROSTER_SHA = 'cf22112055614845533ea4c76f38ea76be2f66f6b8bed58111443c9b5e5b80d7'
PRIVATE_PINS = {
    'analysis-private.json': ANALYSIS_SHA,
    'all-packets-private.csv': 'b412a669cccba394612e7d5a597a2b3c8f0dde27c561de103738567bd208a719',
    'all-forwards.csv': '3a72f5fc5ef9d47a526ead84c634fdcc4a37d30907621b5165cf068a035bc82d',
}
OPERATIONS = [
    ('input_norm', 'normalization'), ('q_projection', 'QKV projections'),
    ('k_projection', 'QKV projections'), ('v_projection', 'QKV projections'),
    ('q_norm', 'normalization'), ('k_norm', 'normalization'), ('rope', 'rotary position'),
    ('kv_append', 'KV cache write'), ('gqa', 'attention'),
    ('o_projection', 'attention output projection'), ('attention_residual', 'residual add'),
    ('post_attention_norm', 'normalization'), ('gate_projection', 'MLP gate/up projections'),
    ('up_projection', 'MLP gate/up projections'), ('swiglu', 'activation'),
    ('down_projection', 'MLP down projection'), ('mlp_residual', 'residual add'),
]
COLORS = dict(zip(
    ['normalization', 'MLP down projection', 'QKV projections', 'KV cache write',
     'MLP gate/up projections', 'attention output projection', 'attention', 'rotary position',
     'argmax', 'vocabulary projection', 'residual add', 'activation', 'embedding'],
    ['#bd3f49', '#3769a5', '#20969e', '#a5791c', '#7956a5', '#5587bc', '#25804a',
     '#b76692', '#555d64', '#536579', '#668b79', '#91784c', '#858585'], strict=True))
PACKET_FIELDS = ['generation', 'phase', 'slot', 'layer', 'operation', 'family',
                 'loaded_kernel_ordinal', 'kernel_symbol', 'kernarg_bytes', 'grid_x', 'workgroup_x',
                 'start_system_ticks_from_forward_start', 'end_system_ticks_from_forward_start',
                 'system_frequency_hz', 'duration_ns_floor']
FORWARD_FIELDS = ['generation', 'phase', 'packet_count', 'sum_packet_intervals_ns',
                  'packet_interval_span_ns_floor', 'instrumented_host_batch_elapsed_ns',
                  'system_frequency_hz', 'payload_sha256']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def bounded(path, limit=16 * 1024**2):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit, 'bounded regular input')
        result = bytearray()
        while len(result) <= limit:
            part = os.read(fd, min(1024**2, limit + 1 - len(result)))
            if not part:
                break
            result.extend(part)
        after = os.fstat(fd)
        fields = ('st_dev', 'st_ino', 'st_mode', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
        require(len(result) == before.st_size and all(getattr(before, key) == getattr(after, key) for key in fields), 'stable input')
        return bytes(result)
    finally:
        os.close(fd)


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def decode(raw):
    return json.loads(raw, object_pairs_hook=unique, parse_constant=lambda _: require(False, 'nonfinite JSON'))


def pinned(path, expected):
    raw = bounded(path, 38 * 1024**2)
    require(digest(raw) == expected, 'accepted input identity: ' + Path(path).name)
    return raw


def phase(generation):
    require(type(generation) is int and 1 <= generation <= 36, 'generation 1..36')
    return 'prompt' if generation <= 5 else 'decode'


def stage(slot):
    require(type(slot) is int and 0 <= slot < 616, 'slot 0..615')
    special = {0: ('', 'embedding', 'embedding'), 613: ('', 'final_norm', 'normalization'),
               614: ('', 'vocabulary_head', 'vocabulary projection'), 615: ('', 'argmax', 'argmax')}
    if slot in special:
        return special[slot]
    layer, offset = divmod(slot - 1, 17)
    return str(layer), *OPERATIONS[offset]


def read_csv(raw):
    return list(csv.DictReader(io.StringIO(raw.decode('ascii'))))


def integer(value):
    require(type(value) is str and re.fullmatch(r'0|[1-9][0-9]*', value) is not None, 'canonical nonnegative integer')
    result = int(value)
    require(result < 2**64, 'u64 interval field')
    return result


def checked_data(packet_rows, forward_rows, roster):
    require(len(packet_rows) == 22176 and len(forward_rows) == 36, 'complete accepted cohort')
    require(len(roster) == 19 and [row['loaded_kernel_ordinal'] for row in roster] == list(range(1, 20)), 'actual 19-root load order')
    by_id = {row['loaded_kernel_ordinal']: row for row in roster}
    packets, forwards = [], []
    textual = {'phase', 'layer', 'operation', 'family', 'kernel_symbol'}
    for index, source in enumerate(packet_rows):
        require(list(source) == PACKET_FIELDS, 'closed packet CSV schema')
        row = {key: value if key in textual else integer(value) for key, value in source.items()}
        generation, slot = divmod(index, 616)
        require((row['generation'], row['slot'], row['phase']) == (generation + 1, slot, phase(generation + 1)), 'complete ordered packet roster')
        require((row['layer'], row['operation'], row['family']) == stage(slot), 'exact source schedule roles')
        require(row['loaded_kernel_ordinal'] in by_id, 'loaded root ordinal')
        root = by_id[row['loaded_kernel_ordinal']]
        require(row['kernel_symbol'] == root['kernel_symbol'] and row['kernarg_bytes'] == root['kernarg_bytes'], 'actual root ABI')
        require(row['workgroup_x'] == 64 and row['grid_x'] > 0 and row['grid_x'] % 64 == 0, 'packet geometry')
        start, end = row['start_system_ticks_from_forward_start'], row['end_system_ticks_from_forward_start']
        require(start <= end and row['system_frequency_hz'] == 10**9 and row['duration_ns_floor'] > 0, 'positive correlated interval')
        require(abs((end - start) - row['duration_ns_floor']) <= 1, 'separate integer floors')
        packets.append(row)
    for index, source in enumerate(forward_rows):
        require(list(source) == FORWARD_FIELDS, 'closed forward CSV schema')
        row = {key: value if key in ('phase', 'payload_sha256') else integer(value) for key, value in source.items()}
        group = packets[index * 616:(index + 1) * 616]
        require((row['generation'], row['phase'], row['packet_count']) == (index + 1, phase(index + 1), 616), 'forward roster')
        require(row['system_frequency_hz'] == 10**9 and row['instrumented_host_batch_elapsed_ns'] > 0, 'separate clock fields')
        require(re.fullmatch('[0-9a-f]{64}', row['payload_sha256']) is not None, 'frame payload identity')
        require(min(p['start_system_ticks_from_forward_start'] for p in group) == 0, 'forward-local origin')
        require(row['packet_interval_span_ns_floor'] == max(p['end_system_ticks_from_forward_start'] for p in group), 'actual packet span')
        require(row['sum_packet_intervals_ns'] == sum(p['duration_ns_floor'] for p in group), 'all packet intervals summed')
        require(all(a['start_system_ticks_from_forward_start'] <= b['start_system_ticks_from_forward_start'] for a, b in zip(group, group[1:])), 'source-ordered sequential trace')
        forwards.append(row)
    return packets, forwards


def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n')


def write_csv(path, rows):
    with path.open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)


def aggregates(packets, key):
    result = []
    for scope, count in [('all', 36), ('prompt', 5), ('decode', 31)]:
        groups = defaultdict(list)
        for row in packets:
            if scope == 'all' or row['phase'] == scope:
                groups[row[key]].append(row['duration_ns_floor'])
        for name, values in groups.items():
            require(len(values) % count == 0, 'normalized packet counts')
            result.append({'phase': scope, key: name, 'forwards': count, 'packets_per_forward': len(values) // count,
                           'packet_count': len(values), 'sum_packet_intervals_ns': sum(values),
                           'mean_sum_per_forward_ns': sum(values) / count,
                           'mean_packet_interval_ns': sum(values) / len(values)})
    return sorted(result, key=lambda row: (row['phase'], -row['sum_packet_intervals_ns'], row[key]))


def summary(forwards, operations):
    phases = []
    for name in ('prompt', 'decode', 'all'):
        rows = [row for row in forwards if name == 'all' or row['phase'] == name]
        item = {'phase': name, 'forwards': len(rows)}
        for field in ('packet_interval_span_ns_floor', 'sum_packet_intervals_ns', 'instrumented_host_batch_elapsed_ns'):
            values = [row[field] for row in rows]
            item[field] = {'sum': sum(values), 'mean': sum(values) / len(rows), 'min': min(values), 'max': max(values)}
        phases.append(item)
    norms = []
    for label, names in [('full_hidden_4096', ('input_norm', 'post_attention_norm', 'final_norm')), ('qk_width_128', ('q_norm', 'k_norm'))]:
        for scope in ('all', 'prompt', 'decode'):
            rows = [row for row in operations if row['phase'] == scope and row['operation'] in names]
            count = sum(row['packet_count'] for row in rows)
            total = sum(row['sum_packet_intervals_ns'] for row in rows)
            norms.append({'phase': scope, 'normalization_shape': label, 'packets_per_forward': sum(row['packets_per_forward'] for row in rows),
                          'mean_sum_per_forward_ns': total / rows[0]['forwards'], 'mean_packet_interval_ns': total / count})
    return {'schema': 'FerricPublicGpuPacketProfileSummaryV1', 'phases': phases, 'normalization_shapes': norms,
            'timeline_generation': 6, 'timeline_packets': 616, 'all_packets': 22176,
            'performance_qualified': False, 'gpu_overlap_measured': False, 'clock_accuracy_qualified': False}


def svg(width, height, title):
    return [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img">',
            f'<title>{escape(title)}</title>', '<rect width="100%" height="100%" fill="white"/>',
            '<style>text{font:13px Arial,sans-serif;fill:#263238}.title{font-size:21px;font-weight:600}.small{font-size:12px}.axis{stroke:#ccd2d5;stroke-width:1}</style>']


def write_svg(path, parts):
    path.write_text('\n'.join(parts + ['</svg>']) + '\n')


def family_plot(path, families):
    parts = svg(1160, 690, 'Absolute packet interval family means, same scale for prompt and decode')
    parts += ['<text class="title" x="30" y="34">Packet-processing intervals by operation family</text>',
              '<text x="30" y="60">Mean sum per forward (ms); 5 prompt forwards and 31 decode forwards from one instrumented run</text>']
    names = [row['family'] for row in families if row['phase'] == 'all']
    lookup = {(row['phase'], row['family']): row for row in families}
    for scope, left, color in [('prompt', 290, '#ad5b31'), ('decode', 730, '#217995')]:
        parts.append(f'<text x="{left}" y="93">{scope.title()} (common 0-60 ms scale)</text>')
        for tick in range(0, 61, 15):
            x = left + tick / 60 * 320
            parts.append(f'<path class="axis" d="M{x:.2f} 110V618"/><text x="{x:.2f}" y="637" text-anchor="middle">{tick}</text>')
        for index, name in enumerate(names):
            y = 126 + index * 38
            row = lookup[scope, name]; value = row['mean_sum_per_forward_ns'] / 10**6
            if scope == 'prompt':
                parts.append(f'<text x="30" y="{y + 5}">{escape(name)}</text>')
            parts.append(f'<rect data-family="{escape(name)}" data-phase="{scope}" x="{left}" y="{y - 9}" width="{value / 60 * 320:.4f}" height="19" fill="{color}"/>')
            parts.append(f'<text x="{left + 330}" y="{y + 5}">{value:.3f}</text>')
    parts.append('<text class="small" x="30" y="674">Sum-derived category means, not GPU busy percentages, overlap, or pure wave-body time.</text>')
    write_svg(path, parts)


def forward_plot(path, forwards):
    parts = svg(1120, 900, 'All 36 forwards, separate packet span, packet sum, and host elapsed panels')
    parts += ['<text class="title" x="75" y="33">All 36 instrumented forwards</text>',
              '<text x="75" y="58">Shaded generations 1-5: prompt. Generations 6-36: decode after the first generated token.</text>']
    panels = [('packet_interval_span_ns_floor', 'Packet interval span', '#217995'),
              ('sum_packet_intervals_ns', 'Sum of packet intervals', '#348356'),
              ('instrumented_host_batch_elapsed_ns', 'Separate host batch elapsed (not GPU time)', '#645e79')]
    for index, (field, label, color) in enumerate(panels):
        top = 115 + index * 245; height = 164
        values = [row[field] / 10**6 for row in forwards]
        low, high = math.floor(min(values)) - 1, math.ceil(max(values)) + 1
        parts.append(f'<text x="75" y="{top - 20}">{label}; displayed axis {low}-{high} ms (not zero-based)</text>')
        parts.append(f'<rect x="75" y="{top}" width="121.5" height="{height}" fill="#fff1df"/>')
        for tick in range(5):
            y = top + height * (1 - tick / 4); value = low + (high - low) * tick / 4
            parts.append(f'<path class="axis" d="M75 {y}H1020"/><text x="65" y="{y + 4}" text-anchor="end">{value:.2f}</text>')
        coordinates = [(75 + (row['generation'] - 1) * 27, top + height * (high - value) / (high - low)) for row, value in zip(forwards, values)]
        parts.append(f'<polyline data-series="{field}" points="{" ".join(f"{x:.3f},{y:.3f}" for x, y in coordinates)}" fill="none" stroke="{color}" stroke-width="1.4"/>')
        for row, (x, y), value in zip(forwards, coordinates, values):
            parts.append(f'<circle data-series="{field}" data-generation="{row["generation"]}" cx="{x:.3f}" cy="{y:.3f}" r="3" fill="{color}"><title>Generation {row["generation"]}: {value:.6f} ms</title></circle>')
        for generation in (1, 5, 6, 10, 15, 20, 25, 30, 36):
            parts.append(f'<text x="{75 + (generation - 1) * 27}" y="{top + height + 21}" text-anchor="middle">{generation}</text>')
    parts += ['<text class="small" x="75" y="850">Separate panels retain all 108 observations. Span and sum nearly coincide; neither is a GPU utilization measure.</text>',
              '<text class="small" x="75" y="874">Host Instant durations have no absolute alignment to these GPU-correlated system timestamps.</text>']
    write_svg(path, parts)


def timeline_plot(path, packets):
    rows = [row for row in packets if row['generation'] == 6]
    require(len(rows) == 616, 'first-decode timeline completeness')
    groups = [rows[:103]] + [rows[103 + i * 102:103 + (i + 1) * 102] for i in range(4)] + [rows[511:]]
    require(sum(map(len, groups)) == 616, 'wrapped sequential row accounting')
    max_ms = max((group[-1]['end_system_ticks_from_forward_start'] - group[0]['start_system_ticks_from_forward_start']) / 10**6 for group in groups)
    axis = math.ceil(max_ms / 5) * 5
    parts = svg(1180, 865, 'Actual first decode forward: all 616 sequential packet intervals, wrapped in time')
    parts += ['<text class="title" x="35" y="34">Actual first decode forward: 616 sequential AQL packets</text>',
              '<text x="35" y="60">Generation 6. One queue, wrapped in time: rows continue top to bottom; they are NOT parallel lanes.</text>',
              f'<text x="35" y="83">Every rectangle uses actual correlated endpoints. Shared row scale: 0-{axis} ms from that row\'s first packet.</text>']
    for row_index, group in enumerate(groups):
        top = 132 + row_index * 83
        origin = group[0]['start_system_ticks_from_forward_start']; end = group[-1]['end_system_ticks_from_forward_start']
        parts.append(f'<text x="35" y="{top - 13}">Slots {group[0]["slot"]}-{group[-1]["slot"]}: forward-relative {origin / 10**6:.3f}-{end / 10**6:.3f} ms</text>')
        for tick in range(0, axis + 1, 5):
            x = 35 + tick / axis * 1080
            parts.append(f'<path class="axis" d="M{x} {top}V{top + 27}"/><text class="small" x="{x}" y="{top + 45}" text-anchor="middle">{tick}</text>')
        for row in group:
            start = row['start_system_ticks_from_forward_start']; finish = row['end_system_ticks_from_forward_start']
            x = 35 + (start - origin) / 10**6 / axis * 1080
            width = (finish - start) / 10**6 / axis * 1080
            label = f'Slot {row["slot"]}, {row["operation"]}, {start / 10**6:.6f}-{finish / 10**6:.6f} ms; duration floor {row["duration_ns_floor"]} ns'
            parts.append(f'<rect data-slot="{row["slot"]}" data-generation="6" x="{x:.6f}" y="{top}" width="{width:.6f}" height="26" fill="{COLORS[row["family"]]}"><title>{escape(label)}</title></rect>')
    for index, (family, color) in enumerate(COLORS.items()):
        x = 35 + (index % 3) * 380; y = 672 + (index // 3) * 27
        parts.append(f'<rect x="{x}" y="{y - 11}" width="14" height="14" fill="{color}"/><text x="{x + 23}" y="{y}">{escape(family)}</text>')
    parts += ['<text class="small" x="35" y="824">Sequential execution is the source-defined ordered queue schedule, not an overlap experiment or a GPU busy/union measurement.</text>',
              '<text class="small" x="35" y="846">Packet-processing intervals include more than pure wave-body execution. No host/GPU absolute clock alignment is implied.</text>']
    write_svg(path, parts)


def table(path, rows, key):
    lookup = {(row['phase'], row[key]): row for row in rows}
    names = [row[key] for row in rows if row['phase'] == 'all']
    text = ['| Category | Packets/forward | All mean sum (ms) | Prompt (ms) | Decode (ms) |',
            '| --- | ---: | ---: | ---: | ---: |']
    for name in names:
        row = lookup['all', name]
        text.append(f'| {name} | {row["packets_per_forward"]} | {row["mean_sum_per_forward_ns"] / 10**6:.4f} | {lookup["prompt", name]["mean_sum_per_forward_ns"] / 10**6:.4f} | {lookup["decode", name]["mean_sum_per_forward_ns"] / 10**6:.4f} |')
    path.write_text('\n'.join(text) + '\n')


def render(input_directory, output):
    provenance_raw = bounded(input_directory / 'provenance.json')
    provenance = decode(provenance_raw)
    require(provenance['accepted_analysis_sha256'] == ANALYSIS_SHA and provenance['capture_manifest_sha256'] == CAPTURE_SHA, 'single accepted cohort')
    require(provenance['acceptance'] == {'forwards': 36, 'packets': 22176, 'reference_tokens': 32, 'postchecks_zero': 13, 'clean_footer': True}, 'accepted completeness')
    require(set(provenance['public_data_sha256']) == {'packets.csv', 'forwards.csv', 'kernel-roster.json'}, 'public input roster')
    raw = {name: pinned(input_directory / name, pin) for name, pin in provenance['public_data_sha256'].items()}
    packets, forwards = checked_data(read_csv(raw['packets.csv']), read_csv(raw['forwards.csv']), decode(raw['kernel-roster.json']))
    output.mkdir(parents=True, exist_ok=False)
    for name, data in raw.items():
        (output / name).write_bytes(data)
    (output / 'provenance.json').write_bytes(provenance_raw)
    families, operations = aggregates(packets, 'family'), aggregates(packets, 'operation')
    write_csv(output / 'families.csv', families); write_csv(output / 'operations.csv', operations)
    write_json(output / 'summary.json', summary(forwards, operations))
    family_plot(output / 'family-means.svg', families)
    forward_plot(output / 'forward-intervals.svg', forwards)
    timeline_plot(output / 'first-decode-timeline.svg', packets)
    table(output / 'family-table.md', families, 'family'); table(output / 'operation-table.md', operations, 'operation')
    hashes = ''.join(f'{digest(bounded(path))}  {path.name}\n' for path in sorted(output.iterdir()) if path.is_file())
    (output / 'SHA256SUMS').write_text(hashes)


def export(analysis, plan_path, capture_path, roster_path, output):
    original = {name: pinned(analysis / name, pin) for name, pin in PRIVATE_PINS.items()}
    accepted = decode(original['analysis-private.json'])
    require(accepted['passed'] is True and accepted['checked_packets'] == 22176 and accepted['checked_forwards'] == 36, 'accepted analysis')
    require(accepted['poststatuses_zero'] == 13 and accepted['reference_generated_tokens'] == 32, 'accepted numerical/postflight gates')
    plan = decode(pinned(plan_path, PLAN_SHA)); capture = decode(pinned(capture_path, CAPTURE_SHA))
    roster = decode(pinned(roster_path, ROSTER_SHA))
    packet_source = read_csv(original['all-packets-private.csv']); forward_source = read_csv(original['all-forwards.csv'])
    origins = {integer(row['generation']): integer(row['min_start_system_ticks']) for row in forward_source}
    packets = []
    for source in packet_source:
        row = {key: source[key] for key in PACKET_FIELDS if key in source}
        row['loaded_kernel_ordinal'] = source['kernel_id']
        origin = origins[integer(source['generation'])]
        row['start_system_ticks_from_forward_start'] = str(integer(source['start_system_ticks']) - origin)
        row['end_system_ticks_from_forward_start'] = str(integer(source['end_system_ticks']) - origin)
        packets.append({key: row[key] for key in PACKET_FIELDS})
    forwards = [{key: source[key] for key in FORWARD_FIELDS} for source in forward_source]
    public_roster = [{'loaded_kernel_ordinal': row['kernel_id'], 'kernel_symbol': row['kernel_symbol'],
                      'image_sha256': bytes(row['kernel_sha256']).hex(), 'kernarg_bytes': row['kernarg_bytes'],
                      'implicit_argument_offset': row['implicit_argument_offset'], 'implicit_argument_bytes': row['implicit_argument_bytes'],
                      'wavefront_size': row['wavefront_size']} for row in roster]
    checked_data(packets, forwards, public_roster)
    output.mkdir(parents=True, exist_ok=False)
    write_csv(output / 'packets.csv', packets); write_csv(output / 'forwards.csv', forwards)
    write_json(output / 'kernel-roster.json', public_roster)
    public_data = {name: digest(bounded(output / name)) for name in ('packets.csv', 'forwards.csv', 'kernel-roster.json')}
    provenance = {'schema': 'FerricPublicGpuPacketProfileProvenanceV1', 'accepted_analysis_sha256': ANALYSIS_SHA,
                  'capture_manifest_sha256': CAPTURE_SHA, 'plan_sha256': PLAN_SHA, 'raw_roster_sha256': ROSTER_SHA,
                  'acceptance': {'forwards': 36, 'packets': 22176, 'reference_tokens': 32, 'postchecks_zero': 13, 'clean_footer': True},
                  'source_input_sha256': {role: record['sha256'] for role, record in plan['inputs'].items()},
                  'raw_capture_sha256': capture, 'private_analysis_input_sha256': PRIVATE_PINS,
                  'public_data_sha256': public_data,
                  'sanitization': 'Closed CSV schemas; absolute GPU/system ticks removed; each forward has its own zero origin. No process IDs, device IDs, model contents, or host paths.',
                  'scope': 'Single instrumented run; AMD packet-processing intervals; not pure wave-body, busy/union, overlap, or throughput qualification.'}
    write_json(output / 'provenance.json', provenance)


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    commands = parser.add_subparsers(dest='mode', required=True)
    plotting = commands.add_parser('render'); plotting.add_argument('--input', type=Path, required=True); plotting.add_argument('--output', type=Path, required=True)
    exporting = commands.add_parser('export')
    for name in ('analysis', 'plan', 'capture-manifest', 'roster', 'output'):
        exporting.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    if args.mode == 'render':
        render(args.input, args.output)
    else:
        export(args.analysis, args.plan, args.capture_manifest, args.roster, args.output)


if __name__ == '__main__':
    main()
