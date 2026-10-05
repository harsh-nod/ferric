"""Render three non-stacked host-diagnostic plots from caller-pinned analysis data."""
import hashlib
import json
import math
from pathlib import Path
import sys
import xml.etree.ElementTree as ET


def main():
    if len(sys.argv) != 4:
        raise ValueError('ANALYSIS_JSON SHA256 FRESH_SVG')
    source, output = Path(sys.argv[1]), Path(sys.argv[3])
    raw = source.read_bytes()
    if len(raw) > 1 << 20 or hashlib.sha256(raw).hexdigest() != sys.argv[2]:
        raise ValueError('bounded caller-pinned analysis')
    value = json.loads(raw)
    if (value['schema'] != 'ferric-p228-projection-ordered-segment-pair-analysis-v1'
            or value['fixed_order'] != ['shared', 'ordered']
            or value['gpu_time'] is not False or value['performance_claim'] is not False
            or value['per_kernel_publish_wait_poll_comparable'] is not False
            or value['separate_residual_mlp_timings_available'] is not False):
        raise ValueError('single-pair host diagnostic only')
    roles = ('shared', 'ordered')
    timings = {role: value['timing'][role]['forward_host_ns'] for role in roles}
    counters = {role: [row['full_sum_ns'] for row in value['intervals']
        if row['route'] == role and 1 <= row['interval'] <= 4] for role in roles}
    segments = {'ordered': value['ordered_segment_sum_ns']}
    for series in (timings, counters, segments):
        if any(len(rows) != 4 or any(type(n) is not int or n < 0 for n in rows)
               for rows in series.values()):
            raise ValueError('four finite nonnegative observations per route')
    ET.register_namespace('', 'http://www.w3.org/2000/svg')
    root = ET.Element('svg', xmlns='http://www.w3.org/2000/svg',
        width='1100', height='1030', viewBox='0 0 1100 1030', role='img')
    ET.SubElement(root, 'title').text = 'Four-forward shared-full versus ordered host observations'
    ET.SubElement(root, 'desc').text = ('One fixed-order Qwen3-8B BF16 TP2 pair on MI350. '
        'Forward host wall time and inclusive full-currentness counter sums are separate panels. Ordered combined residual-to-MLP segments form a third, non-additive panel. '
        'Not GPU timing, a sustained benchmark, or independent numerical acceptance.')
    ET.SubElement(root, 'rect', width='1100', height='1030', fill='#ffffff')

    def text(x, y, label, size=14, anchor='start', weight='400'):
        ET.SubElement(root, 'text', x=str(x), y=str(y), fill='#202522',
            attrib={'font-family': 'Arial, sans-serif', 'font-size': str(size),
                    'text-anchor': anchor, 'font-weight': weight}).text = label

    text(40, 36, 'Qwen3-8B: shared-full versus ordered', 24, weight='600')
    text(40, 61, 'BF16, TP2, MI350 | one fixed-order four-forward diagnostic', 15)
    colors = {'shared': '#646c76', 'ordered': '#16826b'}
    for index, role in enumerate(roles):
        x = 700 + index * 175
        ET.SubElement(root, 'rect', x=str(x), y='82', width='16', height='16', fill=colors[role])
        text(x + 24, 96, 'Shared-full' if role == 'shared' else 'Ordered')
    for top, label, series in ((125, 'Forward host wall time (seconds)', timings),
                               (395, 'Full-currentness counter sum (seconds)', counters),
                               (665, 'Ordered combined segment sum, 36 layers (host seconds)', segments)):
        text(40, top, label, 17, weight='600')
        peak = max(n / 1e9 for rows in series.values() for n in rows)
        ceiling = max(1, math.ceil(peak / 5)) * 5
        bottom, height = top + 200, 160
        for tick in range(6):
            y = bottom - tick * height / 5
            ET.SubElement(root, 'line', x1='95', x2='1050', y1=str(y), y2=str(y), stroke='#e0e4e1')
            text(80, y + 5, str(ceiling * tick / 5), anchor='end')
        for position in range(4):
            center = 215 + position * 235
            text(center, bottom + 24, 'Position ' + str(position), anchor='middle')
            for index, role in enumerate(series):
                seconds = series[role][position] / 1e9
                bar = seconds / ceiling * height
                x = center - 41 + index * 44
                ET.SubElement(root, 'rect', x=str(x), y=str(bottom - bar),
                    width='38', height=str(bar), fill=colors[role])
                text(x + 19, bottom - bar - 8, f'{seconds:.3f}', 12, anchor='middle')
    text(40, 927, 'Counter sum = rank-full + group-full + publication-full; nested read/write/dispatch timers are excluded.', 13)
    text(40, 950, 'Counter intervals and forward boundaries differ. Bars are not an additive latency partition or GPU timings.', 13)
    text(40, 973, 'Shared first, ordered second; warmed caches and first-use arena costs confound this one pair. No 2,048/256 claim.', 13)
    text(40, 996, 'Combined segment includes preflight/staging/fences/waits/checks. Publish/wait/poll are not comparable per-kernel timers.', 13)
    text(40, 1019, 'The ordered segment panel has no legacy counterpart; do not add it to forward wall or full-currentness counters.', 13)
    if source.read_bytes() != raw:
        raise ValueError('analysis changed during rendering')
    encoded = ET.tostring(root, encoding='utf-8', xml_declaration=True)
    with output.open('xb') as stream:
        stream.write(encoded)
    print(json.dumps({'path': str(output), 'bytes': len(encoded),
        'sha256': hashlib.sha256(encoded).hexdigest(), 'analysis_sha256': sys.argv[2]}))


if __name__ == '__main__':
    main()
