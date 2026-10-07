"""Present one pinned numerical diagnostic receipt; do not recompute or accept it."""
import argparse
import csv
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys

INPUT_PIN = {'bytes': 94179, 'sha256': '50dcd3b01a95164186e517005719de46bc321396089762cf40e9013be14c0494'}
POSITIONS = (0, 15, 16, 39)
ROLES = tuple('layer%d-hidden' % i for i in range(36)) + ('final-norm', 'logits')
METRICS = ('elements', 'exact_words', 'max_abs_error', 'max_bf16_steps', 'relative_l2', 'rmse', 'zero_reference_norm')
FALSE_SCOPE = ('numerical_acceptance', 'full_model_acceptance', 'full_long_workload', 'performance_claim', 'production_authority', 'gpu_execution', 'model_execution')
CAP = 1 << 20


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(rows):
        value = {}
        for key, row in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = row
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def fields(value, names):
    require(type(value) is dict and set(value) == set(names), 'closed object fields')


def integer(value, low, high):
    require(type(value) is int and low <= value <= high, 'bounded integer, not bool')


def finite(value):
    require(type(value) in (int, float) and math.isfinite(value) and value >= 0, 'finite nonnegative scalar')


def logits_pin(value):
    fields(value, ('bytes', 'sha256'))
    require(type(value['bytes']) is int and value['bytes'] == 303872
            and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'recorded logits pin')


def validate(d):
    fields(d, ('schema', 'acceptance_threshold', 'argmax_diagnostics', 'argmax_matches',
               'argmax_mismatches', 'argmax_positions', 'bundle_id', 'candidate_generated_tokens',
               'full_prompt_sha256', 'model_id', 'own_kv_caches', 'reference_generated_tokens',
               'selected', 'selected_positions', 'tensor_rows', *FALSE_SCOPE))
    require(d['schema'] == 'ferric-readiness40-actual-reference-diagnostic-v1'
            and d['selected_positions'] == list(POSITIONS)
            and all(type(x) is int for x in d['selected_positions'])
            and d['acceptance_threshold'] is None and d['own_kv_caches'] is True
            and all(d[k] is False for k in FALSE_SCOPE), 'diagnostic-only scope')
    for key, value in (('tensor_rows', 152), ('argmax_positions', 40),
                       ('candidate_generated_tokens', 0), ('reference_generated_tokens', 0)):
        integer(d[key], value, value)
    require(d['full_prompt_sha256'] == '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02'
            and d['model_id'] == 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
            and d['bundle_id'] == '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b', 'pinned workload identity')
    points = d['argmax_diagnostics']
    require(type(points) is list and len(points) == 40, 'all40 diagnostic records')
    for position, row in enumerate(points):
        fields(row, ('position', 'input_token', 'candidate_output_token', 'reference_output_token',
                     'output_token_equal', 'selected_capture', 'candidate_argmax_recomputed',
                     'reference_argmax_recomputed', 'candidate_logits', 'reference_logits', 'unselected_argmax_scope'))
        integer(row['position'], position, position)
        for key in ('input_token', 'candidate_output_token', 'reference_output_token'):
            integer(row[key], 0, 151935)
        selected = position in POSITIONS
        require(row['output_token_equal'] is (row['candidate_output_token'] == row['reference_output_token'])
                and all(row[key] is selected for key in ('selected_capture', 'candidate_argmax_recomputed', 'reference_argmax_recomputed'))
                and row['unselected_argmax_scope'] == (None if selected else
                    'authenticated original records; logits body not retained'), 'argmax evidence scope')
        logits_pin(row['candidate_logits'])
        logits_pin(row['reference_logits'])
    mismatch_keys = ('position', 'input_token', 'candidate_output_token', 'reference_output_token',
                     'selected_capture', 'candidate_argmax_recomputed', 'reference_argmax_recomputed')
    mismatches = [{k: row[k] for k in mismatch_keys} for row in points if not row['output_token_equal']]
    integer(d['argmax_matches'], 0, 40)
    require(d['argmax_matches'] == 40 - len(mismatches)
            and encoded(d['argmax_mismatches']) == encoded(mismatches), 'complete mismatch ledger')
    selected = d['selected']
    fields(selected, ('schema', 'acceptance_threshold', 'comparisons', 'input_history', 'tensor_rows',
                      'receipt_authentication', 'numerical_acceptance', 'full_model_acceptance', 'full_long_workload', 'performance_claim'))
    require(selected['schema'] == 'ferric-readiness40-selected-tensor-diagnostic-v1'
            and selected['acceptance_threshold'] is None
            and selected['input_history'] == 'authentic first40 prompt IDs; independent own KV; no generated feedback'
            and all(selected[k] is False for k in ('receipt_authentication', 'numerical_acceptance',
                                                   'full_model_acceptance', 'full_long_workload', 'performance_claim')), 'selected scope')
    integer(selected['tensor_rows'], 152, 152)
    comparisons = selected['comparisons']
    require(type(comparisons) is list and len(comparisons) == 4, 'four selected captures')
    for position, row in zip(POSITIONS, comparisons):
        fields(row, ('position', 'input_token', 'candidate_output_token', 'reference_output_token', 'output_token_equal', 'tensors'))
        integer(row['position'], position, position)
        require(all(encoded(row[k]) == encoded(points[position][k]) for k in
                    ('input_token', 'candidate_output_token', 'reference_output_token', 'output_token_equal')), 'selected identity joins')
        fields(row['tensors'], ROLES)
        for role in ROLES:
            metric = row['tensors'][role]
            fields(metric, METRICS)
            count = 151936 if role == 'logits' else 4096
            integer(metric['elements'], count, count)
            integer(metric['exact_words'], 0, count)
            integer(metric['max_bf16_steps'], 0, 65535)
            finite(metric['max_abs_error'])
            finite(metric['rmse'])
            require(type(metric['zero_reference_norm']) is bool, 'zero-norm flag')
            relative = metric['relative_l2']
            if relative is None:
                require(metric['zero_reference_norm'] and metric['rmse'] > 0, 'undefined relative norm')
            else:
                finite(relative)
                finite(relative * 100)
                require(not metric['zero_reference_norm'] or relative == metric['rmse'] == 0, 'zero-reference consistency')
            if role.startswith('layer'):
                require(relative is not None, 'hidden plot requires finite relative error')
    return d


def authenticate(raw):
    require(pin(raw) == INPUT_PIN, 'exact actual comparison receipt')
    receipt = parse(raw)
    fields(receipt, ('schema', 'passed', 'error', 'postcheck_errors', 'elapsed_seconds', 'limits',
                     'inputs', 'checks', 'acceptance_threshold', 'native_rerun', *FALSE_SCOPE))
    require(receipt['schema'] == 'ferric-readiness40-comparison-data-v1' and receipt['passed'] is True
            and receipt['error'] is None and receipt['postcheck_errors'] == []
            and receipt['acceptance_threshold'] is None and receipt['native_rerun'] is False
            and all(receipt[k] is False for k in FALSE_SCOPE), 'successful diagnostic receipt, not acceptance')
    fields(receipt['checks'], ('native', 'checker', 'reference', 'comparison_tests', 'diagnostics'))
    require(len(receipt['inputs']) == 11 and len(receipt['checks']['comparison_tests']) == 8,
            'actual input/test census')
    return validate(receipt['checks']['diagnostics'])


def csv_bytes(header, rows):
    stream = io.StringIO(newline='')
    writer = csv.writer(stream, lineterminator='\n')
    writer.writerow(header)
    writer.writerows(rows)
    return stream.getvalue().encode()


def svg(d):
    lines = [[row['tensors']['layer%d-hidden' % i]['relative_l2'] * 100 for i in range(36)]
             for row in d['selected']['comparisons']]
    peak = max(max(row) for row in lines)
    scale = 10 ** math.floor(math.log10(peak)) if peak else 1
    top = math.ceil(peak / scale * 5) / 5 * scale if peak else 1
    require(math.isfinite(top) and top > 0, 'finite chart scale')
    colors = ('#0072b2', '#d55e00', '#009e73', '#8b4ca5')
    dashes = ('none', '8 3', '3 3', '10 3 2 3')
    out = ['<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="540" viewBox="0 0 1000 540" role="img" aria-labelledby="title desc">',
           '<title id="title">Readiness40: hidden-state relative L2 error</title>',
           '<desc id="desc">Four selected prompt positions, 36 layers. Independent reference diagnostics; no numerical acceptance. Position 5 mismatch has no captured logits.</desc>',
           '<rect width="1000" height="540" fill="#ffffff"/>',
           '<g font-family="Arial, sans-serif" fill="#202124">',
           '<text x="90" y="34" font-size="22">Readiness40: hidden-state relative L2 error</text>',
           '<text x="90" y="57" font-size="13">Native candidate versus independent reference, separate KV histories</text>']
    for i, position in enumerate(POSITIONS):
        x = 90 + i * 210
        out += ['<line x1="%d" y1="86" x2="%d" y2="86" stroke="%s" stroke-width="3" stroke-dasharray="%s"/>' % (x, x + 30, colors[i], dashes[i]),
                '<text x="%d" y="90" font-size="14">Position %d</text>' % (x + 39, position)]
    for tick in range(6):
        y = 405 - tick * 58
        out += ['<line x1="90" y1="%d" x2="955" y2="%d" stroke="#dce1e6"/>' % (y, y),
                '<text x="79" y="%d" text-anchor="end" font-size="12">%.4g</text>' % (y + 4, top * tick / 5)]
    for layer in range(0, 36, 5):
        x = 90 + layer * 865 / 35
        out.append('<text x="%.3f" y="427" text-anchor="middle" font-size="12">%d</text>' % (x, layer))
    out += ['<path d="M90 115V405H955" fill="none" stroke="#34383c"/>',
            '<text x="522" y="449" text-anchor="middle" font-size="14">Layer index (0-35)</text>',
            '<text x="24" y="260" transform="rotate(-90 24 260)" text-anchor="middle" font-size="14">Relative L2 error (%)</text>']
    for i, row in enumerate(lines):
        points = ' '.join('%.3f,%.3f' % (90 + layer * 865 / 35, 405 - value / top * 290) for layer, value in enumerate(row))
        out.append('<polyline data-position="%d" points="%s" fill="none" stroke="%s" stroke-width="2.4" stroke-dasharray="%s"/>' % (POSITIONS[i], points, colors[i], dashes[i]))
    out += ['<text x="90" y="478" font-size="13">%d/40 argmax matches; position 5 was NOT captured. No numerical acceptance.</text>' % d['argmax_matches'],
            '<text x="90" y="499" font-size="12">Captures: 0, 15, 16, 39. No performance or overlap claim; no threshold is applied.</text>',
            '<text x="90" y="522" font-size="11">Comparison receipt SHA-256: %s</text>' % INPUT_PIN['sha256'],
            '</g></svg>\n']
    return '\n'.join(out).encode()


def render(d):
    validate(d)
    tensors = [(row['position'], role, row['input_token'], row['candidate_output_token'], row['reference_output_token'],
                *(row['tensors'][role][key] for key in METRICS)) for row in d['selected']['comparisons'] for role in ROLES]
    argmax_header = ('position', 'input_token', 'candidate_output_token', 'reference_output_token', 'output_token_equal',
                     'selected_capture', 'candidate_argmax_recomputed', 'reference_argmax_recomputed', 'unselected_argmax_scope')
    argmax_rows = [tuple(row[key] for key in argmax_header) + (row['candidate_logits']['sha256'], row['reference_logits']['sha256'])
                   for row in d['argmax_diagnostics']]
    exact = sum(row['tensors'][role]['exact_words'] == row['tensors'][role]['elements'] for row in d['selected']['comparisons'] for role in ROLES)
    md = ['# Readiness40 Numerical Diagnostics', '',
          'Forty authentic prompt-fed positions, zero generated tokens, independent own KV caches.',
          'This is not the full 2,048-input/256-output workload and does not establish numerical acceptance.', '',
          '**%d/40 argmax matches; %d/152 selected tensors bit exact.**' % (d['argmax_matches'], exact), '',
          '| Mismatch position | Input token | Native argmax | Reference argmax | Tensor capture |',
          '| --- | --- | --- | --- | --- |']
    for row in d['argmax_mismatches']:
        md.append('| %d | %d | %d | %d | %s |' % (row['position'], row['input_token'], row['candidate_output_token'],
                  row['reference_output_token'], 'Selected' if row['selected_capture'] else 'NOT captured'))
    md += ['', 'Position 5 has only authenticated original argmax records. Its logits, tensor errors, and margin were not captured.', '',
           '| Selected position | Native / reference argmax | Logits relative L2 | Logits exact words | Logits max absolute error |',
           '| --- | --- | --- | --- | --- |']
    for row in d['selected']['comparisons']:
        m = row['tensors']['logits']
        md.append('| %d | %d / %d | %s | %d / %d | %s |' % (row['position'], row['candidate_output_token'],
                  row['reference_output_token'], repr(m['relative_l2']), m['exact_words'], m['elements'], repr(m['max_abs_error'])))
    md += ['', '![Hidden-state relative L2 error across 36 layers](hidden-relative-l2.svg)', '',
           'The plot displays the recorded relative-L2 values multiplied by 100, with a linear axis. The CSV preserves the original metric values, without rounding or a new threshold.',
           'BF16-step distance is an encoding-order diagnostic, not a numerical acceptance rule.', '',
           '[All 152 tensor rows](tensor-metrics.csv) | [All 40 argmax records](argmax-diagnostics.csv)', '',
           'No performance, overlap, full-model acceptance, or production claim. No model execution or metric recomputation is performed by this renderer.', '',
           'Input: %d bytes, SHA-256 `%s`.' % (INPUT_PIN['bytes'], INPUT_PIN['sha256']), '']
    return {'tensor-metrics.csv': csv_bytes(('position', 'tensor', 'input_token', 'candidate_output_token', 'reference_output_token', *METRICS), tensors),
            'argmax-diagnostics.csv': csv_bytes((*argmax_header, 'candidate_logits_sha256', 'reference_logits_sha256'), argmax_rows),
            'summary.md': '\n'.join(md).encode(), 'hidden-relative-l2.svg': svg(d)}


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical regular input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= CAP, 'bounded ordinary file')
        raw = stream.read(CAP + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'input changed')
    return raw


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(args.output.is_absolute() and args.output.parent.resolve(strict=True) == args.output.parent
            and not os.path.lexists(args.output), 'fresh output directory')
    raw = read(args.input)
    source = read(Path(__file__).resolve())
    outputs = render(authenticate(raw))
    require(len(outputs) == 4 and sum(map(len, outputs.values())) < CAP, 'bounded four report outputs')
    args.output.mkdir(mode=0o700)
    for name, body in outputs.items():
        with (args.output / name).open('xb') as stream:
            stream.write(body)
            stream.flush()
            os.fsync(stream.fileno())
        require(read(args.output / name) == body, 'report output readback')
    require(read(args.input) == raw and read(Path(__file__).resolve()) == source, 'input/source posthash')
    result = dict(schema='ferric-readiness40-numerical-report-v1', passed=True, input=INPUT_PIN,
                  renderer=pin(source), files={name: pin(body) for name, body in outputs.items()},
                  tensor_rows=152, argmax_rows=40, original_metrics_preserved=True, metrics_recomputed=False,
                  numerical_acceptance=False, full_model_acceptance=False, full_long_workload=False,
                  performance_claim=False, gpu_execution=False, model_execution=False)
    with (args.output / 'complete.json').open('xb') as stream:
        stream.write(encoded(result))
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'use python3 -I -B render.py')
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 128 << 20), (resource.RLIMIT_FSIZE, CAP), (resource.RLIMIT_CORE, 0), (resource.RLIMIT_CPU, 20)):
        old = resource.getrlimit(kind)
        value = min([cap] + [x for x in old if x != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    def interrupted(number, _frame):
        raise RuntimeError('report signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 30)
    try:
        main()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
