"""Render the retained numerical diagnostic; this is not a timing benchmark."""
import csv
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parent
SHA = 'c164a135580fca25142f1b258f5844057dbfd86cbf5e01986809753b95ed8bdc'


def main():
    raw = (ROOT.parent / 'gpu-v1/output/diagnostic.json').read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('diagnostic hash mismatch')
    diagnostic = json.loads(raw)
    if diagnostic['control_and_repeat_gate_passed'] is not True:
        raise ValueError('control/repeat gate did not pass')
    rows = diagnostic['comparisons']
    expected = [(rank, stage) for rank in (0, 1) for stage in ('gate', 'up', 'product')]
    if [(r['rank'], r['stage']) for r in rows] != expected:
        raise ValueError('unexpected comparison rows')

    svg = ET.Element('svg', xmlns='http://www.w3.org/2000/svg',
                     viewBox='0 0 900 550', role='img')
    ET.SubElement(svg, 'title').text = 'MLP numerical isolation on MI350 gfx950'
    ET.SubElement(svg, 'desc').text = (
        'Six 6144-word BF16 comparisons. Original versus matched input differs '
        'at 1621 versus 16 words. No kernel change or performance claim.')
    ET.SubElement(svg, 'rect', width='900', height='550', fill='white')

    def text(x, y, value, size=14, **attrs):
        node = ET.SubElement(svg, 'text', x=str(x), y=str(y), fill='#202124',
                             attrib={'font-family': 'sans-serif', 'font-size': str(size), **attrs})
        node.text = str(value)

    text(24, 32, 'MLP differences: original chain vs identical input', 22)
    text(24, 57, 'Layer 0, position 0; 6,144 BF16 words per row. Numerical diagnostic only.')
    for x, color, label in ((170, '#60666d', 'Original chain'), (380, '#087f75', 'Identical input')):
        ET.SubElement(svg, 'rect', x=str(x), y='79', width='15', height='15', fill=color)
        text(x + 23, 92, label)
    for tick in range(0, 401, 100):
        x = 170 + 1.5 * tick
        ET.SubElement(svg, 'line', x1=str(x), x2=str(x), y1='114', y2='480', stroke='#e0e0e0')
        text(x, 501, tick, **{'text-anchor': 'middle'})

    table = []
    for index, row in enumerate(rows):
        y = 139 + index * 58
        text(24, y + 8, f"{row['stage'].title()}, rank {row['rank']}")
        counts = []
        for offset, key, color in ((0, 'original_chain', '#60666d'), (20, 'matched_input', '#087f75')):
            metric = row[key]
            if metric['elements'] != 6144 or not 0 <= metric['exact_words'] <= 6144:
                raise ValueError('unexpected metric extent')
            count = metric['elements'] - metric['exact_words']
            counts.append(count)
            width = 1.5 * count
            ET.SubElement(svg, 'rect', x='170', y=str(y - 10 + offset),
                          width=str(width), height='14', fill=color)
            text(176 + width, y + 2 + offset, count, 13)
        table.append((row['stage'], row['rank'], *counts,
                      row['matched_input']['max_bf16_steps'], row['matched_input']['relative_l2']))
    text(470, 531, 'Different BF16 words (linear scale)', **{'text-anchor': 'middle'})
    ET.ElementTree(svg).write(ROOT / 'different-words.svg', encoding='utf-8', xml_declaration=True)
    with (ROOT / 'comparison.csv').open('w', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow(('stage', 'rank', 'original_different_words', 'matched_different_words',
                         'matched_max_bf16_steps', 'matched_relative_l2'))
        writer.writerows(table)


if __name__ == '__main__':
    main()
