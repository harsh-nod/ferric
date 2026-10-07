"""Summarize a retained successful timing probe; no model or GPU execution."""
import csv
import hashlib
import html
import json
from pathlib import Path
import sys


def require(ok, why):
    if not ok:
        raise ValueError(why)


def read(path):
    require(path.is_file() and not path.is_symlink(), 'ordinary input')
    require(path.stat().st_size <= 8 << 20, 'input size')
    with path.open('rb') as stream:
        body = stream.read((8 << 20) + 1)
    require(len(body) <= 8 << 20, 'input size')
    return body, json.loads(body)


def digest(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def u64(value):
    require(type(value) is int and 0 <= value < 1 << 64, 'exact u64')
    return value


def checked_timeline(t):
    require(len(t['forwards']) == 40, 'forty actual forwards')
    spans = [('Source preparation', t['source_preparation']),
             ('Spawn through setup', t['spawn_to_setup_seal'])]
    for position, row in enumerate(t['forwards']):
        require(u64(row['position']) == position and u64(row['generation']) == position + 1,
                'ordered positions')
        parts = [row[k] for k in ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit')]
        require(sum(u64(p['elapsed_ns']) for p in parts) == u64(row['elapsed_ns']), 'row extent')
        spans.extend(zip(('Prepare/write', 'Wait for frame', 'Validate/retain/commit'), parts))
    spans.extend([('Close and retirement', t['close_and_retirement']),
                  ('Postcheck and publication', t['postcheck_and_ordinary_publication'])])
    cursor = 0
    for _, span in spans:
        require(set(span) == {'start_ns', 'end_ns', 'elapsed_ns'}, 'closed span')
        a, b, n = (u64(span[k]) for k in ('start_ns', 'end_ns', 'elapsed_ns'))
        require(a == cursor and b >= a and b - a == n, 'disjoint span')
        cursor = b
    require(len(spans) == 124 and cursor == u64(t['total_ns']) and cursor > 0, 'total extent')
    return spans


def svg_chart(title, subtitle, labels, series, names, colors, width=1060, height=540):
    # Independent stacked rows keep small validation spans visible in the CSV.
    require(len(labels) == len(series) and all(len(r) == len(names) for r in series), 'chart shape')
    maximum = max(map(sum, series))
    require(maximum > 0, 'nonempty chart')
    left, right, top, bottom = 180, 80, 105, 65
    plot_width, plot_height = width - left - right, height - top - bottom
    step = plot_height / len(labels)
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
           '<rect width="100%" height="100%" fill="white"/>',
           '<g font-family="sans-serif" fill="#20252b">',
           f'<text x="24" y="30" font-size="21">{html.escape(title)}</text>',
           f'<text x="24" y="54" font-size="13">{html.escape(subtitle)}</text>']
    for tick in range(5):
        x = left + tick * plot_width / 4
        out.extend([f'<line x1="{x}" y1="{top - 5}" x2="{x}" y2="{height-bottom}" stroke="#dfe4e8"/>',
                    f'<text x="{x}" y="{height-bottom+20}" text-anchor="middle" font-size="12">{maximum*tick/4:.2f}</text>'])
    for i, (label, parts) in enumerate(zip(labels, series)):
        y = top + step * i
        out.append(f'<text x="{left-10}" y="{y+step*0.6}" text-anchor="end" font-size="12">{html.escape(label)}</text>')
        x = left
        for value, color in zip(parts, colors):
            size = value / maximum * plot_width
            out.append(f'<rect x="{x}" y="{y+step*0.12}" width="{size}" height="{step*0.76}" fill="{color}"/>')
            x += size
        out.append(f'<text x="{x+6}" y="{y+step*0.6}" font-size="11">{sum(parts):.3f}</text>')
    legend_x = 24
    for name, color in zip(names, colors):
        out.extend([f'<rect x="{legend_x}" y="74" width="11" height="11" fill="{color}"/>',
                    f'<text x="{legend_x+17}" y="84" font-size="12">{html.escape(name)}</text>'])
        legend_x += 35 + len(name) * 7
    out.extend([f'<text x="{left+plot_width/2}" y="{height-14}" text-anchor="middle" font-size="13">Parent wall time (seconds), not GPU kernel time</text>', '</g></svg>\n'])
    return '\n'.join(out)


def main():
    require(len(sys.argv) == 4, 'CAPSULE ACTUAL_TERMINAL_SHA FRESH_OUTPUT')
    capsule, sha, out = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
    raw, terminal = read(capsule / 'readiness/complete.json')
    require(digest(raw)['sha256'] == sha and terminal['passed'] is True and terminal['errors'] == [],
            'actual clean successful terminal')
    require(terminal['native_attempts'] == 1 and terminal['retries'] == 0 and len(terminal['phases']) == 11,
            'one completed native attempt')
    for phase in terminal['phases']:
        require(phase['exit_code'] == 0 and phase['reason'] is None
                and phase['owned_groups_absent'] is True and phase['owned_processes_reaped'] is True,
                'clean phase retirement')
    require(terminal['same_side_parity']['passed'] is True
            and terminal['same_side_parity']['all40_records_equal'] is True
            and terminal['same_side_parity']['all4_payloads_byte_equal'] is True, 'instrumentation parity')
    sidecar_raw, sidecar = read(capsule / 'readiness/native/host-timing.json')
    pin = terminal['timing']['file']
    require(digest(sidecar_raw) == {k: pin[k] for k in ('bytes', 'sha256')}, 'sidecar pin')
    t = sidecar['timeline']
    require(t == terminal['timing']['timeline'] and sidecar['generated_tokens'] == 0
            and sidecar['gpu_timing'] is False, 'unchanged parent-only diagnostic')
    spans = checked_timeline(t)
    categories = {}
    for name, span in spans:
        categories[name] = categories.get(name, 0) + span['elapsed_ns']
    forwards = [r['elapsed_ns'] for r in t['forwards']]
    result = dict(schema='ferric-readiness40-host-timing-analysis-v1', terminal=digest(raw), sidecar=digest(sidecar_raw),
                  analysis_source=digest(Path(__file__).read_bytes()), total_ns=t['total_ns'], categories_ns=categories,
                  forward_ns=dict(min=min(forwards), median_twice_ns=sum(sorted(forwards)[19:21]),
                                  max=max(forwards), total=sum(forwards)),
                  parent_host_measurement=True, gpu_timing=False, generated_tokens=0,
                  throughput_claim=False, full2303_feasibility=False, numerical_acceptance=False)
    require(not out.exists() and out.parent.is_dir(), 'fresh output directory')
    out.mkdir()
    (out / 'summary.json').write_text(json.dumps(result, sort_keys=True, indent=2) + '\n')
    with (out / 'forwards.csv').open('x', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow(['position', 'generation', 'prepare_write_ns', 'flush_to_frame_read_ns', 'validate_retain_commit_ns', 'elapsed_ns'])
        for row in t['forwards']:
            writer.writerow([row['position'], row['generation'], *[row[k]['elapsed_ns'] for k in
                ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit')], row['elapsed_ns']])
    rows = ['| Parent interval | Seconds | Share |', '| --- | ---: | ---: |']
    for name, ns in categories.items():
        rows.append(f'| {name} | {ns/1e9:.6f} | {100*ns/t["total_ns"]:.3f}% |')
    rows.append(f'| Total | {t["total_ns"]/1e9:.6f} | 100% |')
    (out / 'table.md').write_text('\n'.join(rows) + '\n')
    (out / 'overview.svg').write_text(svg_chart('Readiness40 parent time breakdown',
        'One completed probe; 40 prompt positions, zero generated tokens.', list(categories),
        [[ns/1e9] for ns in categories.values()], ['Measured interval'], ['#147d92']))
    keys = ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit')
    (out / 'forwards.svg').write_text(svg_chart('Readiness40 per-forward parent intervals',
        'Contiguous nonoverlapping spans. Wait includes worker execution, pipe wait and framing.',
        [str(r['position']) for r in t['forwards']], [[r[k]['elapsed_ns']/1e9 for k in keys] for r in t['forwards']],
        ['Prepare/write', 'Wait for frame', 'Validate/retain/commit'], ['#ab4f35', '#147d92', '#58616b'], height=1180))
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
