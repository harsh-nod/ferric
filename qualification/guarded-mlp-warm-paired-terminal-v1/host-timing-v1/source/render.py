"""One-pair host-wall diagnostic; no GPU timing or benchmark inference."""
import csv
from decimal import Decimal, localcontext, ROUND_HALF_EVEN
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time
import types
import xml.etree.ElementTree as ET

RETENTION_PIN = dict(bytes=35074, sha256='9fc9b80e46cf3e54d85f9d3206653664e5235a91ae65ff6d758465858bff0605')
MODES = ('control', 'candidate')
U64_MAX = (1 << 64) - 1
SVG = 'http://www.w3.org/2000/svg'
MAX_FILE, MAX_TOTAL, MAX_FILES = 16 << 20, 64 << 20, 256


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def parse(raw):
    def pairs(rows):
        result = {}
        for name, value in rows:
            require(name not in result, 'duplicate JSON key')
            result[name] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def decimal_ratio(numerator, denominator=1_000_000, places=3):
    require(type(numerator) is int and type(denominator) is int and denominator > 0
            and type(places) is int and 0 <= places <= 9, 'integer ratio formatting')
    with localcontext() as context:
        context.prec = 80
        return format((Decimal(numerator) / Decimal(denominator)).quantize(
            Decimal(1).scaleb(-places), rounding=ROUND_HALF_EVEN), 'f')


def summarize(cases):
    require(type(cases) is dict and set(cases) == set(MODES), 'one complete control/candidate pair')
    observations = {}
    for mode in MODES:
        case = cases[mode]
        require(case['passed'] is True and case['errors'] == [] and case['case'] == mode
                and case['mode'] == 'ar4' and case['native_attempts'] == 1 and case['retries'] == 0
                and case['paired_terminal_requested'] is (mode == 'candidate')
                and case['default_full_currentness_requested'] is True
                and case['shared_full_currentness_requested'] is False
                and case['host_observation_requested'] is False
                and case['hidden_read_policy_changed'] is False,
                'successful explicit unchanged-policy cases')
        require(case['paired_terminal_dispatches'] == ([0, 0, 36, 36] if mode == 'candidate' else [0] * 4),
                'actual first-use/warm dispatch cadence')
        for claim in ('numerical_acceptance', 'performance_claim', 'production_authority'):
            require(case[claim] is False, 'diagnostic-only authority')
        ordinary = case['ordinary_comparison']
        require(ordinary['all_payloads_equal'] is True and ordinary['all_histories_equal'] is True,
                'ordinary payload/history parity')
        obs = case['observation']
        require(obs['host_timing_only'] is True and obs['gpu_latency_claim'] is False,
                'original host timer scope')
        times = obs['segment_host_ns']
        require(case['segment_host_ns'] == times
                and obs['paired_terminal_dispatches'] == case['paired_terminal_dispatches']
                and obs['local_bank_generations'] == [1, 1, 2, 2], 'original observed timing and reuse joins')
        require(type(times) is list and len(times) == 4
                and all(type(row) is list and len(row) == 36 for row in times)
                and all(type(v) is int and 0 <= v <= U64_MAX for row in times for v in row),
                'exact 4 by 36 unsigned host intervals')
        observations[mode] = obs
    control, candidate = cases['control'], cases['candidate']
    require(control['controller'] == candidate['controller']
            and control['admission'] == candidate['admission'], 'same controller and qualified ELF pair')
    require(candidate['same_elf_control_required'] is True
            and candidate['control_comparison']['all_payloads_equal'] is True
            and candidate['control_comparison']['all_histories_equal'] is True,
            'same-ELF control payload/history parity')
    require(observations['control']['input_tokens'] == observations['candidate']['input_tokens']
            and observations['control']['output_tokens'] == observations['candidate']['output_tokens']
            and all(type(observations['control'][name]) is list and len(observations['control'][name]) == 4
                    for name in ('input_tokens', 'output_tokens')),
            'exact four-forward token histories')
    layers = []
    for position in range(4):
        for layer in range(36):
            for mode in MODES:
                layers.append(dict(mode=mode, position=position,
                    phase='first-use' if position < 2 else 'warm', bank=position % 2,
                    bank_generation=position // 2 + 1, layer=layer,
                    segment_host_ns=observations[mode]['segment_host_ns'][position][layer]))
    summaries = []
    for label, positions in [(str(i), [i]) for i in range(4)] + [('first-use', [0, 1]), ('warm', [2, 3])]:
        sums = {mode: sum(observations[mode]['segment_host_ns'][p][l]
                         for p in positions for l in range(36)) for mode in MODES}
        summaries.append(dict(scope=label, positions=positions, intervals_per_mode=36 * len(positions),
            control_ns=sums['control'], candidate_ns=sums['candidate'],
            candidate_minus_control_ns=sums['candidate'] - sums['control'],
            relative_change_numerator=100 * (sums['candidate'] - sums['control']),
            relative_change_denominator=sums['control'] or None))
    return dict(schema='ferric-warm-paired-terminal-host-diagnostic-v1', layers=layers,
        summaries=summaries, runs_per_mode=1, correlated_forwards_per_run=4,
        full_forward_time=False, gpu_timing=False, throughput_claim=False,
        end_to_end_speedup_claim=False, controlled_benchmark=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)


def csv_body(rows, fields):
    stream = io.StringIO(newline='')
    writer = csv.DictWriter(stream, fieldnames=fields, lineterminator='\n', extrasaction='raise')
    writer.writeheader(); writer.writerows(rows)
    return stream.getvalue().encode()


def plot(data):
    root = ET.Element('{%s}svg' % SVG, width='1120', height='760', viewBox='0 0 1120 760', role='img')
    ET.SubElement(root, '{%s}title' % SVG).text = 'Paired-terminal segment host-wall diagnostic'
    ET.SubElement(root, '{%s}desc' % SVG).text = (
        'One serial control/candidate pair, same ELF pair. Positions zero and one are first use; '
        'positions two and three reuse retired arenas. Four correlated forwards, not a benchmark.')
    def node(tag, **attrs):
        return ET.SubElement(root, '{%s}%s' % (SVG, tag),
            {key.replace('_', '-'): str(value) for key, value in attrs.items()})
    def text(x, y, value, size=14, **attrs):
        node('text', x=x, y=y, fill='#20242a', font_family='sans-serif', font_size=size, **attrs).text = value
    node('rect', x=0, y=0, width=1120, height=760, fill='#ffffff')
    text(45, 34, 'Paired-Terminal Segment Host Wall', 25)
    text(45, 60, 'One serial pair; same qualified ELF pair; four correlated forwards per run', 15)
    colors = {'control': '#256a8a', 'candidate': '#bf5b48'}
    for i, mode in enumerate(MODES):
        x = 700 + i * 180
        node('line', x1=x, x2=x + 28, y1=90, y2=90, stroke=colors[mode], stroke_width=3)
        text(x + 38, 95, mode.capitalize())
    text(45, 94, 'Host milliseconds per guarded segment; identical zero-based scale', 14)
    maximum = max(1, max(row['segment_host_ns'] for row in data['layers']))
    unit = 10 ** max(0, len(str(maximum)) - 2)
    top = ((maximum + unit - 1) // unit) * unit
    for position in range(4):
        x = 85 + (position % 2) * 550; y = 350 + (position // 2) * 285
        text(x, y - 205, 'Position %d: %s, bank %d / generation %d' %
             (position, 'first use' if position < 2 else 'warm reuse', position % 2, position // 2 + 1), 15)
        for tick in range(5):
            yy = y - tick * 45
            node('line', x1=x, x2=x + 420, y1=yy, y2=yy, stroke='#dde2e5', stroke_width=1)
            text(x - 10, yy + 4, decimal_ratio(top * tick, 4_000_000, 2), 12, text_anchor='end')
        for layer in (0, 7, 14, 21, 28, 35):
            text(x + layer * 12, y + 19, str(layer), 12, text_anchor='middle')
        text(x + 210, y + 42, 'Layer', 13, text_anchor='middle')
        for mode in MODES:
            values = [row for row in data['layers'] if row['position'] == position and row['mode'] == mode]
            require([row['layer'] for row in values] == list(range(36)), 'closed plotting order')
            points = ' '.join('%s,%s' % (row['layer'] * 12 + x,
                format(y - row['segment_host_ns'] * 180 / top, '.6f')) for row in values)
            line = node('polyline', points=points, fill='none', stroke=colors[mode], stroke_width=2)
            ET.SubElement(line, '{%s}title' % SVG).text = '%s position %d: 36 original host intervals' % (mode, position)
    text(45, 712, 'Includes coordinator checks, publish, waits, retire and terminal checks. Not GPU or full-forward time.', 13)
    text(45, 735, 'One pair only: no controlled benchmark, end-to-end speedup, throughput, or numerical acceptance.', 13)
    return ET.tostring(root, encoding='utf-8', xml_declaration=True) + b'\n'


def markdown(data):
    lines = ['# Paired-Terminal Segment Host Diagnostic', '',
        'One serial control/candidate pair using the same qualified parent and worker ELFs. '
        'The four forwards within each run are correlated, not independent repetitions.', '',
        '| Scope | Segments / mode | Control sum (ms) | Candidate sum (ms) | Candidate - control (ms) | Relative change (%) |',
        '| --- | ---: | ---: | ---: | ---: | ---: |']
    for row in data['summaries']:
        denominator = row['relative_change_denominator']
        change = 'n/a' if denominator is None else decimal_ratio(row['relative_change_numerator'], denominator)
        lines.append('| %s | %d | %s | %s | %s | %s |' % (row['scope'], row['intervals_per_mode'],
            decimal_ratio(row['control_ns']), decimal_ratio(row['candidate_ns']),
            decimal_ratio(row['candidate_minus_control_ns']), change))
    lines += ['', 'Positions 0/1 use the original first-use terminal path in both cases. '
        'At positions 2/3 the candidate uses the paired-terminal path only after genuine retired-arena reuse; '
        'the control keeps the original terminal path. All other currentness and hidden-read policies remain unchanged.', '',
        'The original `segment_host_ns` timer spans coordinator preflight, owner consumption, ring reservation, '
        'publication, polling, retirement and terminal validation. Earlier setup/allocation, prefix work, later hidden '
        'readback and the complete forward are outside this field. No inclusive counters are added or subtracted.', '',
        'Integer nanoseconds are preserved in `layers.csv` and `summary.csv`. '
        '[Per-layer host-wall plot](segments.svg). Display decimals use round-to-nearest, ties-to-even; '
        'a zero control denominator is reported as unavailable.', '',
        'Both original receipts and every retained body must pass the existing pure retention verifier first. '
        'Four-payload byte equality and exact token-history equality are prerequisites, not independent model accuracy. '
        'This is one observed pair, not a controlled repeated benchmark or proof of causality. '
        'No GPU latency, tokens/second, end-to-end speedup, numerical acceptance or production authority is claimed.', '']
    return '\n'.join(lines).encode()


def outputs(data):
    summaries = [{k: (','.join(map(str, v)) if k == 'positions' else v)
                  for k, v in row.items()} for row in data['summaries']]
    return {'layers.csv': csv_body(data['layers'], list(data['layers'][0])),
        'summary.csv': csv_body(summaries, list(summaries[0])),
        'summary.md': markdown(data), 'segments.svg': plot(data),
        'comparison.json': (json.dumps(data, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()}


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 5,
            'python3 -B render.py CAPSULE CONTROL_SHA CANDIDATE_SHA FRESH_OUTPUT')
    require(RETENTION_PIN is not None and all(re.fullmatch('[0-9a-f]{64}', s) for s in sys.argv[2:4]),
            'frozen verifier and observed successful terminal pins')
    deadline = time.monotonic() + 60
    def guard(): require(time.monotonic() < deadline, 'whole rendering deadline')
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('render deadline')))
    signal.setitimer(signal.ITIMER_REAL, 60)
    for kind, limit in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_CPU, 50),
                        (resource.RLIMIT_FSIZE, 1 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        cap = min([limit] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    readset = {}
    def read(path):
        guard(); path = Path(path)
        require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical report input')
        stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                    and 0 <= before.st_size <= MAX_FILE, 'bounded ordinary report input')
            raw = stream.read(MAX_FILE + 1); after = os.fstat(stream.fileno())
        require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
                'stable report input')
        current = pin(raw)
        require(str(path) not in readset or readset[str(path)] == current, 'consistent input pin')
        readset[str(path)] = current
        require(len(readset) <= MAX_FILES and sum(v['bytes'] for v in readset.values()) <= MAX_TOTAL,
                'bounded full report readset')
        return raw
    root, output = Path(sys.argv[1]), Path(sys.argv[4])
    require(root.is_absolute() and root.resolve(strict=True) == root and root.is_dir(), 'retained capsule root')
    require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent
            and not output.is_relative_to(root) and not os.path.lexists(output), 'fresh external report output')
    own = read(Path(__file__).resolve()); manifest_raw = read(root / 'manifest.json')
    manifest = parse(manifest_raw); files = manifest['files']
    require(type(files) is dict and 1 <= len(files) < MAX_FILES - 3, 'closed capsule manifest')
    def roster():
        names = set()
        for parent, dirs, basenames in os.walk(root, followlinks=False,
                onerror=lambda error: (_ for _ in ()).throw(error)):
            guard(); require(not any((Path(parent) / n).is_symlink() for n in dirs), 'no capsule directory links')
            names.update(str((Path(parent) / n).relative_to(root)) for n in basenames)
            require(len(names) <= MAX_FILES, 'capsule file count')
        return names
    names = roster()
    require(names in (set(files) | {'manifest.json'}, set(files) | {'manifest.json', 'retention.json'}),
            'exact exported or locally retained capsule closure')
    bodies = {}
    for name, expected in files.items():
        require(type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute()
                and '..' not in Path(name).parts and '\\' not in name and name not in ('', '.'), 'relative body name')
        bodies[name] = read(root / name)
        require(pin(bodies[name]) == expected, 'all retained original pins')
    if 'retention.json' in names:
        read(root / 'retention.json')
    verifier_raw = bodies['retention_tool.py']
    require(pin(verifier_raw) == RETENTION_PIN, 'exact pure retention verifier')
    verifier = types.ModuleType('_warm_terminal_report_verifier')
    verifier.__file__ = str(root / 'retention_tool.py')
    exec(compile(verifier_raw, verifier.__file__, 'exec'), verifier.__dict__)
    outcomes = {mode: dict(name='complete.json', sha256=sha) for mode, sha in zip(MODES, sys.argv[2:4])}
    verified = verifier.verify(bodies, outcomes)
    require(verified['all_cases_passed'] is True, 'both authenticated successful cases')
    cases = {mode: parse(bodies[mode + '/complete.json']) for mode in MODES}
    for mode in MODES:
        require(pin(bodies[mode + '/complete.json'])['sha256'] == outcomes[mode]['sha256']
                and verified['cases'][mode]['passed'] is True
                and json.dumps(verified['cases'][mode]['observation'], sort_keys=True) ==
                    json.dumps(cases[mode]['observation'], sort_keys=True), 'verified original observation')
    data = summarize(cases); rendered = outputs(data)
    require(all(len(raw) <= 1 << 20 for raw in rendered.values()), 'bounded report files')
    for name, expected in list(readset.items()):
        require(pin(read(Path(name))) == expected, 'all original input posthashes')
    require(read(Path(__file__).resolve()) == own, 'renderer unchanged')
    require(roster() == names, 'retained input closure unchanged')
    guard(); output.mkdir(mode=0o700)
    for name, raw in rendered.items():
        guard()
        with (output / name).open('xb') as stream:
            stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        require((output / name).read_bytes() == raw, 'exact report output readback')
    receipt = dict(schema='ferric-warm-paired-terminal-host-render-v1', passed=True,
        input_pins=readset, postcheck_errors=[], terminals=outcomes,
        outputs={name:pin(raw) for name,raw in rendered.items()}, runs_per_mode=1,
        gpu_execution=False, project_execution=False, performance_claim=False, numerical_acceptance=False)
    raw = (json.dumps(receipt, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw) <= 1 << 20, 'bounded rendering receipt')
    with (output / 'complete.json').open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    guard(); signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=True, output=str(output), layers=len(data['layers']), summaries=len(data['summaries']))))


if __name__ == '__main__':
    main()
