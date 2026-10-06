"""Render the exact retained numerical diagnostic, without recomputing or accepting it."""
import csv
import hashlib
import io
import json
import math
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import time
import xml.etree.ElementTree as ET

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
REPORT = dict(path=str(E / 'guarded-mlp-model-numerical-comparison-v228-v1/complete.json'),
    bytes=389586, sha256='55b550cf19e3deae73384db0895407b7d16cd9c75d6e677aa3159eed922da482')
OUT = E / 'guarded-mlp-model-numerical-plots-v228-v1'
FALSE = ('gpu_execution', 'native_rerun', 'subprocess_execution', 'numerical_acceptance',
         'independent_tensor_acceptance', 'full_model_acceptance', 'full_model_correctness',
         'performance_claim', 'production_authority', 'full_long_workload', 'sustained_2048_256')
FIELDS = ('mode', 'position', 'input_token', 'reference_output', 'candidate_output', 'output_equal',
          'elements', 'exact_words', 'relative_l2', 'relative_l2_percent', 'max_abs_error', 'rmse',
          'max_bf16_steps', 'first_mismatching_element', 'reference_sha256', 'candidate_sha256')
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def read(path, expected=None):
    path = Path(path)
    require(time.monotonic() < DEADLINE and path.is_absolute() and path.resolve(strict=True) == path,
            'bounded canonical render input')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1 and before.st_size <= 1 << 20,
            'ordinary render input no larger than 1 MiB')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(before) == stamp(os.fstat(stream.fileno())), 'render input open identity')
        raw = stream.read((1 << 20) + 1)
        require(len(raw) == before.st_size and stamp(before) == stamp(os.fstat(stream.fileno()))
                == stamp(path.lstat()), 'render input changed during read')
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(expected is None or pin == expected, 'exact render input pin')
    return raw, pin


def parse(raw):
    def pairs(items):
        result = {}
        for name, value in items:
            require(name not in result, 'duplicate JSON key')
            result[name] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def rows(report):
    require(report['schema'] == 'ferric-guarded-mlp-model-numerical-diagnostic-v1'
            and report['passed'] is True and report['status'] == 'DIAGNOSTIC_COMPLETE'
            and report['error'] is None and report['postcheck_errors'] == []
            and report['comparison_completed'] is True and report['input_posthashes_complete'] is True
            and report['independent_framework_reference_compared'] is True
            and report['original_tf4_failure_preserved'] is True
            and report['acceptance_threshold'] is None and all(report[key] is False for key in FALSE),
            'completed diagnostic without acceptance or performance authority')
    require(set(report['modes']) == {'tf4', 'ar4'} and len(report['inputs']) == 250
            and report['selftests']['run'] == 5 and all(report['selftests'][key] == 0 for key in
            ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes')),
            'actual diagnostic census')
    result = []
    tensor_count = exact_tensors = 0
    for mode in ('tf4', 'ar4'):
        item = report['modes'][mode]
        require(item['mode'] == ('teacher_forced' if mode == 'tf4' else 'autoregressive')
                and item['compared_positions'] == 4 and item['same_history_tensor_rows'] == 152
                and [position['position'] for position in item['positions']] == list(range(4)),
                'four positions per actual mode')
        for position in item['positions']:
            tensors = position['tensors']
            require(position['same_input_history'] is True
                    and position['reference_input'] == position['candidate_input']
                    and position['output_equal'] is True
                    and position['reference_output'] == position['candidate_output']
                    and len(tensors) == 38 and len({t['name'] for t in tensors}) == 38,
                    'only comparable actual positions and their 38 unique tensors')
            tensor_count += len(tensors)
            exact_tensors += sum(t['byte_equal'] is True for t in tensors)
            logits = [tensor for tensor in tensors if tensor['name'] == 'logits']
            require(len(logits) == 1, 'one actual logits tensor per position')
            logits = logits[0]
            require(logits['elements'] == 151936 and logits['zero_reference_norm'] is False
                    and logits['byte_equal'] is False and type(logits['exact_words']) is int
                    and 0 <= logits['exact_words'] <= logits['elements']
                    and all(type(logits[key]) in (int, float) and math.isfinite(logits[key])
                            and logits[key] >= 0 for key in ('relative_l2', 'max_abs_error', 'rmse')),
                    'finite recorded logits errors and exact-word counts')
            row = dict(mode=mode.upper(), position=position['position'], input_token=position['candidate_input'],
                reference_output=position['reference_output'], candidate_output=position['candidate_output'],
                output_equal=position['output_equal'], relative_l2_percent=100 * logits['relative_l2'])
            row.update({key: logits[key] for key in FIELDS if key in logits})
            require(set(row) == set(FIELDS), 'closed rendered logits columns')
            result.append(row)
    require(len(result) == 8 and tensor_count == 304 and exact_tensors == 0,
            'eight token matches; zero wholly bit-identical tensors among 304 comparisons')
    return result


def csv_bytes(data):
    stream = io.StringIO(newline='')
    writer = csv.DictWriter(stream, fieldnames=FIELDS, lineterminator='\n')
    writer.writeheader()
    writer.writerows(data)
    return stream.getvalue().encode('ascii')


def svg_bytes(data):
    ns = 'http://www.w3.org/2000/svg'
    ET.register_namespace('', ns)
    svg = ET.Element('{' + ns + '}svg', dict(viewBox='0 0 960 500', width='960', height='500', role='img'))
    svg.set('aria-labelledby', 'title description')

    def node(tag, **attributes):
        return ET.SubElement(svg, '{' + ns + '}' + tag,
                             {key.replace('_', '-'): str(value) for key, value in attributes.items()})

    def text(x, y, body, size=13, color='#41464d', anchor='start', weight='400'):
        item = node('text', x=x, y=y, font_family='Arial, sans-serif', font_size=size,
                    fill=color, text_anchor=anchor, font_weight=weight, letter_spacing='0')
        item.text = body

    node('title', id='title').text = 'Logits relative L2 error: TF4 and AR4'
    node('desc', id='description').text = (
        'Numerical diagnostic, not acceptance or performance. Eight recorded same-history comparisons '
        'against the pinned independent reference. All output tokens match, but no tensor is wholly bit-identical.')
    node('rect', x=0, y=0, width=960, height=500, fill='#ffffff')
    text(56, 38, 'Logits Relative L2 Error', size=23, color='#20252b', weight='600')
    text(56, 65, 'Numerical diagnostic, not acceptance or performance', size=15)
    colors = {'TF4': '#087f8c', 'AR4': '#78679e'}
    for ordinal, (mode, color) in enumerate(colors.items()):
        x = 670 + ordinal * 106
        node('rect', x=x, y=91, width=14, height=14, fill=color)
        text(x + 22, 103, mode, size=14)
    text(56, 105, 'Relative L2 error (%)', size=13)
    left, right, top, baseline = 82, 916, 132, 373
    maximum = math.ceil(max(row['relative_l2_percent'] for row in data) * 1.15 * 5) / 5
    require(maximum > 0, 'nonzero actual chart scale')
    for ordinal in range(5):
        value = maximum * ordinal / 4
        y = baseline - (baseline - top) * ordinal / 4
        node('line', x1=left, y1=f'{y:.3f}', x2=right, y2=f'{y:.3f}', stroke='#e2e5e8', stroke_width=1)
        text(left - 12, y + 4, f'{value:.2f}', anchor='end', size=12)
    by_key = {(row['position'], row['mode']): row for row in data}
    width, gap = 46, 10
    for position in range(4):
        center = left + (position + 0.5) * (right - left) / 4
        for index, mode in enumerate(('TF4', 'AR4')):
            row = by_key[position, mode]
            value = row['relative_l2_percent']
            height = value / maximum * (baseline - top)
            x = center - width - gap / 2 if index == 0 else center + gap / 2
            y = baseline - height
            bar = node('rect', x=f'{x:.3f}', y=f'{y:.3f}', width=width, height=f'{height:.3f}', fill=colors[mode])
            ET.SubElement(bar, '{' + ns + '}title').text = f'{mode}, position {position}: {value:.17g}%'
            text(x + width / 2, y - 8, f'{value:.3f}%', anchor='middle', size=12, color=colors[mode])
        text(center, baseline + 26, f'Position {position}', anchor='middle', size=13)
    text(56, 443, '8/8 output tokens match; 0/304 complete tensors are bit-identical.', size=14)
    text(56, 468, 'Four recorded forwards per mode. No numerical threshold, speedup, or sustained-decode claim.', size=12)
    return ET.tostring(svg, encoding='utf-8', xml_declaration=True) + b'\n'


def markdown_bytes(data):
    lines = ['# Guarded Model Numerical Diagnostic', '',
        '**Diagnostic only: not numerical acceptance or performance evidence.**', '',
        'All eight output tokens match the independent reference on matching input histories. '
        'None of the 304 complete compared tensors is bit-identical. '
        'The original failed TF4 controller receipt remains unchanged; its separate data revalidation authorized comparison.', '',
        '![Logits relative L2 error by position](logits-relative-l2.svg)', '',
        '| Mode | Position | Input | Output (Both) | Logits Relative L2 (%) | Max Absolute Error | RMSE | Exact Logits Words |',
        '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
    for row in data:
        lines.append(f"| {row['mode']} | {row['position']} | {row['input_token']} | {row['candidate_output']} | "
                     f"{row['relative_l2_percent']:.6f} | {row['max_abs_error']:.8g} | {row['rmse']:.8g} | "
                     f"{row['exact_words']:,}/{row['elements']:,} |")
    lines += ['', 'Relative L2 (%) is exactly `100 * recorded_relative_l2`; '
        'the recorded ratio is `||candidate - reference||_2 / ||reference||_2`. '
        'This renderer does not recompute tensors or change the comparison policy.', '',
        '[CSV with full recorded numeric values and tensor hashes](logits.csv). '
        'The chart and table round display values only.', '',
        'Source: the retained `ferric-guarded-mlp-model-numerical-diagnostic-v1` report, '
        f"{REPORT['bytes']:,} bytes, SHA-256 `{REPORT['sha256']}`. "
        'The renderer verifies that exact report before and after rendering. '
        'It does not independently rehash the 250 upstream inputs or repeat the model/reference executions.', '',
        'No acceptance threshold, full-model correctness, speedup, theoretical performance bound, '
        'or sustained 2,048-prompt/256-decode result is asserted.']
    return ('\n'.join(lines) + '\n').encode('ascii')


def write(name, raw):
    require(time.monotonic() < DEADLINE and len(raw) <= 1 << 20, 'bounded render output')
    with (OUT / name).open('xb') as stream:
        stream.write(raw)
    pin = dict(path=str(OUT / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    read(OUT / name, pin)
    return pin


def interrupted(number, _frame):
    raise RuntimeError('renderer interrupted by signal ' + str(number))


def main():
    global DEADLINE
    DEADLINE = time.monotonic() + 30
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B render.py only')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged data-only render host')
    require(OUT.parent.resolve(strict=True) == OUT.parent and not os.path.lexists(OUT), 'fresh render directory')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'renderer nice level')
    if priority == 0:
        os.nice(10)
    for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        os.environ[key] = ''
    for kind, maximum in ((resource.RLIMIT_AS, 128 << 20), (resource.RLIMIT_FSIZE, 1 << 20),
                          (resource.RLIMIT_CPU, 30), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        cap = min([maximum] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    handlers = {number: signal.getsignal(number) for number in
                (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)}
    for number in handlers:
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 30)
    try:
        _, source = read(Path(__file__).resolve())
        raw, report_pin = read(REPORT['path'], REPORT)
        data = rows(parse(raw))
        outputs = {'logits.csv': csv_bytes(data), 'logits-relative-l2.svg': svg_bytes(data),
                   'summary.md': markdown_bytes(data)}
        read(REPORT['path'], REPORT)
        read(source['path'], source)
        OUT.mkdir(mode=0o700)
        pins = {name: write(name, body) for name, body in outputs.items()}
        read(REPORT['path'], REPORT)
        read(source['path'], source)
        complete = dict(schema='ferric-guarded-mlp-model-numerical-plots-v1', passed=True,
            meaning='rendered the exact retained diagnostic; not numerical acceptance',
            report=report_pin, renderer=source, outputs=pins, rows=8, comparable_tensor_rows=304,
            completely_bit_equal_tensor_rows=0, output_tokens_equal=8, report_posthash_unchanged=True,
            upstream_inputs_rehashed=False, tensor_metrics_recomputed=False, acceptance_threshold=None,
            **{key: False for key in FALSE})
        write('complete.json', (json.dumps(complete, indent=2, sort_keys=True, allow_nan=False) + '\n').encode())
        print(json.dumps(dict(passed=True, rendered_files=3, numerical_acceptance=False), sort_keys=True))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for number, handler in handlers.items():
            signal.signal(number, handler)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
