"""Render only authenticated, observed AR4 allocation counts for documentation."""
import csv
import hashlib
import io
import json
from pathlib import Path
import xml.etree.ElementTree as ET

TERMINAL_SHA = '05d695b62b76c00a6598ff69a08d9d3290710ffdc189f63d3ef2b738d521afbd'
ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parent / 'gpu-attempt-v1/ar4/complete.json'
NS = 'http://www.w3.org/2000/svg'
ET.register_namespace('', NS)


def require(ok, message):
    if not ok:
        raise ValueError(message)


def main():
    raw = SOURCE.read_bytes()
    require(TERMINAL_SHA is not None and hashlib.sha256(raw).hexdigest() == TERMINAL_SHA,
            'actual native terminal required')
    result = json.loads(raw)
    require(result['schema'] == 'ferric-guarded-mlp-reusable-ar4-gpu-v1'
            and result['passed'] is True and result['errors'] == []
            and result['actual_arena_plateau_verified'] is True
            and result['gpu_execution_confirmed'] is True and result['native_attempts'] == 1
            and result['retries'] == 0 and result['performance_claim'] is False,
            'successful single native reuse observation, not a performance claim')
    counts = result['actual_allocation_counts']
    require(counts == result['observation']['actual_arena_census']['allocation_counts']
            == [[715, 711], [751, 747], [787, 783], [787, 783], [787, 783]],
            'observed five-sample allocation plateau')
    comparison = result['ordinary_ar4_comparison']
    require(comparison['all_payloads_equal'] is True and comparison['all_histories_equal'] is True
            and len(comparison['frames']) == 4, 'unchanged complete outputs')

    table = io.StringIO(newline='')
    writer = csv.writer(table, lineterminator='\n')
    writer.writerow(['completed_forwards', 'rank_0_allocations', 'rank_1_allocations',
                     'rank_0_added', 'rank_1_added'])
    for step, row in enumerate(counts):
        delta = [0, 0] if step == 0 else [row[i] - counts[step - 1][i] for i in range(2)]
        writer.writerow([step, *row, *delta])

    svg = ET.Element('{' + NS + '}svg', {'viewBox': '0 0 900 440', 'role': 'img',
        'aria-labelledby': 'title description', 'width': '900', 'height': '440'})
    def element(tag, parent=svg, text=None, **attrs):
        node = ET.SubElement(parent, '{' + NS + '}' + tag,
                             {key.replace('_', '-'): str(value) for key, value in attrs.items()})
        node.text = text
        return node
    element('title', text='Observed reusable arena allocation plateau', id='title')
    element('desc', text='MI350 four-step decode. Both ranks stop adding allocations after forward two. '
            'The vertical axis starts at 650 allocations. This is not a throughput benchmark.', id='description')
    element('rect', width=900, height=440, fill='#ffffff')
    group = element('g', font_family='sans-serif', font_size=14, fill='#20242b')
    element('text', group, text='MI350: live allocations across four decode steps',
            x=70, y=32, font_size=21, font_weight=600)
    element('text', group, text='Observed counts; unchanged BF16 outputs; no throughput claim', x=70, y=57)
    x = lambda step: 110 + step * 165
    y = lambda value: 345 - (value - 650) * 1.18
    for tick in (650, 700, 750, 800, 850):
        element('line', group, x1=85, y1=y(tick), x2=800, y2=y(tick), stroke='#d9dee4')
        element('text', group, text=str(tick), x=72, y=y(tick) + 5, text_anchor='end')
    for step in range(5):
        element('text', group, text='Setup' if step == 0 else 'Forward ' + str(step),
                x=x(step), y=370, text_anchor='middle')
    for rank, color in enumerate(('#1263b3', '#158049')):
        element('polyline', group, points=' '.join(f'{x(step)},{y(row[rank])}'
                for step, row in enumerate(counts)), fill='none', stroke=color, stroke_width=3)
        for step, row in enumerate(counts):
            element('circle', group, cx=x(step), cy=y(row[rank]), r=5,
                    fill=color, stroke='#ffffff', stroke_width=1)
            element('text', group, text=str(row[rank]), x=x(step),
                    y=y(row[rank]) + (-13 if rank == 0 else 24),
                    text_anchor='middle', fill=color, font_weight=600)
        element('line', group, x1=585 + rank * 100, y1=83, x2=608 + rank * 100,
                y2=83, stroke=color, stroke_width=3)
        element('text', group, text='Rank ' + str(rank), x=614 + rank * 100, y=88)
    element('text', group, text='Vertical axis begins at 650 allocations. No allocations added in forwards 3 and 4.',
            x=70, y=410, font_size=13)
    outputs = {'allocations.csv': table.getvalue().encode(),
               'allocations.svg': ET.tostring(svg, encoding='utf-8', xml_declaration=True) + b'\n'}
    require(all(not (ROOT / name).exists() for name in outputs), 'fresh plot outputs')
    require(SOURCE.read_bytes() == raw, 'native terminal unchanged')
    for name, body in outputs.items():
        with (ROOT / name).open('xb') as stream:
            stream.write(body)
    print(json.dumps({'source_sha256': TERMINAL_SHA, 'completed_forwards': 4,
                      'performance_claim': False, 'outputs': list(outputs)}, sort_keys=True))


if __name__ == '__main__':
    main()
