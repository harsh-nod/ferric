"""Strict policy selection and exact host-wall ratios; no GPU-time inference."""
import copy
import hashlib
from math import gcd
import xml.etree.ElementTree as ET

import analyze as A

STAGES = ('prefix', 'paired', 'hidden', 'other')
COLORS = ('#168977', '#ba4567', '#397db5', '#c6922b')
PAYLOAD_BYTES = 606976


def require(ok, reason):
    if not ok: raise ValueError(reason)


def ratio(numerator, denominator):
    A.uint(numerator); A.uint(denominator)
    if denominator == 0:
        return None
    common = gcd(numerator, denominator)
    return dict(numerator=numerator // common, denominator=denominator // common)


def decimal(numerator, denominator, places=3):
    require(type(numerator) is int and type(denominator) is int and denominator > 0
            and type(places) is int and 0 <= places <= 9, 'bounded exact decimal arguments')
    sign = '-' if numerator < 0 else ''
    scale = 10 ** places
    quotient, remainder = divmod(abs(numerator) * scale, denominator)
    quotient += int(remainder * 2 > denominator or (remainder * 2 == denominator and quotient % 2 == 1))
    whole, fraction = divmod(quotient, scale)
    return sign + str(whole) + ('.' + str(fraction).zfill(places) if places else '')


def account(report, checked, shared):
    require(type(shared) is bool, 'explicit Boolean policy selector')
    schema = 'FerricGuardedMlpSharedFullHostObservationV1' if shared else 'FerricGuardedMlpHostObservationV1'
    checked_schema = 'ferric-guarded-mlp-model-host-' + ('shared-currentness' if shared else 'observation') + '-checked-v1'
    require(report['schema'] == schema and checked['schema'] == checked_schema, 'closed report mode')
    require(type(report['snapshots']) is list and len(report['snapshots']) == 587
            and all(s['shared_full_currentness'] is shared for s in report['snapshots']), 'one exact policy throughout')
    if shared:
        require(checked['shared_full_currentness'] is True, 'checked shared policy')
    else:
        require('shared_full_currentness' not in checked, 'original conservative checked schema')
    # Policy is admitted above. Only copies are mapped to the old accounting API;
    # the frozen arithmetic consumes identical counter/interval definitions.
    normalized, checked_copy = copy.deepcopy(report), copy.deepcopy(checked)
    normalized['schema'] = 'FerricGuardedMlpHostObservationV1'
    checked_copy['schema'] = 'ferric-guarded-mlp-model-host-observation-checked-v1'
    checked_copy.pop('shared_full_currentness', None)
    for snapshot in normalized['snapshots']: snapshot['shared_full_currentness'] = False
    result = A.analyze(normalized, checked_copy)
    result['observed_shared_full_currentness'] = shared
    result['observed_report_schema'] = schema
    result['policy_normalization_only_for_unchanged_accounting'] = True
    return result


def compare(baseline, candidate):
    for case in (baseline, candidate):
        require(type(case['payloads']) is list and len(case['payloads']) == 4
                and all(type(v) is bytes and len(v) == PAYLOAD_BYTES for v in case['payloads']), 'four exact payload extents')
        require(type(case['history']) is list and len(case['history']) == 4
                and all(type(v) is int and 0 <= v < 151936 for v in case['history']), 'four genuine token inputs')
    frames = []
    for index, (old, new) in enumerate(zip(baseline['payloads'], candidate['payloads'])):
        same_bytes = old == new
        same_history = baseline['history'][:index + 1] == candidate['history'][:index + 1]
        require(same_bytes and same_history, 'payload/history mismatch: no timing comparison')
        frames.append(dict(position=index, bytes=PAYLOAD_BYTES, sha256=hashlib.sha256(old).hexdigest(),
                           byte_equal=True, same_history=True))
    old = account(baseline['report'], baseline['checked'], False)
    new = account(candidate['report'], candidate['checked'], True)
    rows = []
    for left, right in zip(old['forwards'], new['forwards']):
        require(all(left[k] == right[k] for k in ('position', 'first_interval', 'interval_count', 'category_interval_counts')),
                'identical interval definitions')
        values = {}
        for key in (*STAGES, 'bracket'):
            a = left['bracket_host_ns'] if key == 'bracket' else left['disjoint_wall_ns'][key]
            b = right['bracket_host_ns'] if key == 'bracket' else right['disjoint_wall_ns'][key]
            values[key] = dict(conservative_ns=a, shared_ns=b, shared_minus_conservative_ns=b - a,
                               conservative_over_shared=ratio(a, b))
        rows.append(dict(position=left['position'], wall=values))
    return dict(schema='ferric-guarded-mlp-shared-host-comparison-v1', baseline=old, candidate=new,
        frames=frames, rows=rows, same_payloads=True, same_histories=True,
        host_wall_comparison=True, observations_per_mode=1, controlled_repeated_benchmark=False,
        ratio_is_observed_host_wall_only=True, causal_improvement_established=False,
        nested_counters_added_to_elapsed=False, gpu_execution=False, gpu_time=False, gpu_overlap=False,
        throughput=False, speedup_claim=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)


def markdown(result):
    lines = ['# Observed Host-Wall Comparison', '',
        'One observation per mode, not a controlled repeated benchmark. All four payloads and genuine histories are identical.',
        'Ratios divide conservative host wall by shared-full host wall; they are not device speedups or throughput.', '',
        '| Forward | Conservative Bracket (s) | Shared-Full Bracket (s) | Shared Minus Conservative (s) | Observed Ratio |',
        '| ---: | ---: | ---: | ---: | ---: |']
    for row in result['rows']:
        w = row['wall']['bracket']; r = w['conservative_over_shared']
        ratio_text = 'undefined (zero shared wall)' if r is None else decimal(r['numerator'], r['denominator'])
        values = [row['position'], A.seconds(w['conservative_ns']), A.seconds(w['shared_ns']),
                  decimal(w['shared_minus_conservative_ns'], 1000000000, 9), ratio_text]
        lines.append('| ' + ' | '.join(map(str, values)) + ' |')
    lines += ['', 'The separate per-mode tables retain inclusive rank/shared counters without adding them to wall time.',
        'The chart uses only disjoint prefix, paired, hidden-read and other snapshot intervals. Close is outside these brackets.',
        'No GPU timing, overlap, tokens/second, numerical acceptance, or production authority is claimed.', '']
    return '\n'.join(lines)


def svg(result):
    require(result['same_payloads'] is result['same_histories'] is True, 'plot equivalence gate')
    cases = (result['baseline'], result['candidate'])
    require(all(len(c['forwards']) == 4 for c in cases), 'four plot groups')
    maximum = max(A.uint(row['bracket_host_ns']) for c in cases for row in c['forwards'])
    require(maximum <= 4300 * 1000000000, 'native whole-wall chart bound')
    axis_ns = max(1000000000, ((maximum + 999999999) // 1000000000) * 1000000000)
    namespace = 'http://www.w3.org/2000/svg'
    ET.register_namespace('', namespace)
    root = ET.Element('{%s}svg' % namespace, dict(width='1180', height='720', viewBox='0 0 1180 720',
                                                role='img', **{'aria-labelledby': 'title description'}))
    def element(tag, attributes=None, text=None, parent=root):
        node = ET.SubElement(parent, '{%s}%s' % (namespace, tag), attributes or {})
        node.text = text
        return node
    element('title', {'id': 'title'}, 'Observed host wall by forward')
    element('desc', {'id': 'description'}, 'Four grouped stacked host-wall bars per mode. One observation per mode; not GPU time or a controlled benchmark.')
    element('rect', dict(width='1180', height='720', fill='#ffffff'))
    def text(x, y, value, size=14, anchor='start', color='#24292f'):
        return element('text', dict(x=str(x), y=str(y), fill=color, **{'font-family': 'sans-serif',
            'font-size': str(size), 'text-anchor': anchor}), value)
    text(50, 42, 'Observed Host Wall by Forward', 26)
    text(50, 69, 'Disjoint snapshot intervals; one conservative case and one shared-full case', 15)
    text(50, 95, 'Not GPU time, throughput, overlap, or a controlled repeated benchmark', 13, color='#525b65')
    labels = ('Prefix', 'Paired', 'Hidden Read', 'Other')
    for i, (label, color) in enumerate(zip(labels, COLORS)):
        x = 50 + 158 * i
        element('rect', dict(x=str(x), y='119', width='16', height='16', fill=color))
        text(x + 24, 132, label)
    left, top, bottom, height = 100, 177, 567, 390
    for tick in range(6):
        y = bottom - height * tick // 5
        element('line', dict(x1=str(left), y1=str(y), x2='1110', y2=str(y), stroke='#d9dee3', **{'stroke-width': '1'}))
        text(left - 12, y + 4, decimal(axis_ns * tick, 5 * 1000000000, 2), 12, 'end')
    text(50, top - 12, 'Host Seconds', 12)
    for forward in range(4):
        for mode, case in enumerate(cases):
            row = case['forwards'][forward]
            require(row['category_interval_counts'] == dict(prefix=36, paired=36, hidden=36, other=37)
                    and A.total(row['disjoint_wall_ns'].values()) == row['bracket_host_ns'], 'plot partition reconciliation')
            x = 140 + forward * 245 + mode * 84
            accumulated = 0
            group = element('g', {'id': 'forward_%d_%s' % (forward, ('conservative', 'shared')[mode])})
            for name, color in zip(STAGES, COLORS):
                value = A.uint(row['disjoint_wall_ns'][name]); accumulated += value
                rect = element('rect', dict(x=str(x), y=decimal(bottom * axis_ns - accumulated * height, axis_ns),
                    width='58', height=decimal(value * height, axis_ns), fill=color,
                    **{'data-stage': name, 'data-nanoseconds': str(value)}), parent=group)
                element('title', text=name + ': ' + A.seconds(value) + ' host seconds', parent=rect)
            text(x + 29, bottom + 24, ('Conservative', 'Shared Full')[mode], 11, 'middle')
            text(x + 29, top - 12, decimal(row['bracket_host_ns'], 1000000000, 3) + ' s', 12, 'middle')
        text(211 + forward * 245, bottom + 53, 'Forward %d' % forward, 15, 'middle')
    text(50, 659, 'All four output payloads are byte-identical; autoregressive input token histories match exactly.', 13)
    text(50, 684, 'Inclusive currentness/admission counters are reported separately and are not summed into these bars.', 13)
    return ET.tostring(root, encoding='utf-8', xml_declaration=True) + b'\n'
