"""One admitted diagnostic archive's serial host attribution; never GPU timing."""
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
import tarfile
import time
import types

VERIFIER = dict(bytes=46945, sha256='5cec01470378f1ed5d9efd009f3d66bc8349fe52705b54b39d9ba997d1bf2be9')
CASE = 'tail_duration'
GROUPS = ('bank', 'layers', 'tail')
CATEGORIES = ('before', 'discover', 'after', 'root_generation')
PARENT_CATEGORIES = ('Source preparation', 'Spawn through setup', 'Prepare/write',
    'Wait for frame', 'Validate/retain/commit', 'Close and retirement', 'Postcheck and publication')
PARTITION = ('bank_callbacks_ns', 'bank_other_ns', 'layer_callbacks_ns',
             'tail_callbacks_ns', 'unattributed_parent_ns')
LABELS = ('Bank callbacks', 'Other bank guarded body', 'Layer callbacks',
          'Tail callbacks', 'Other parent-forward time')
COLORS = ('#147d92', '#94c5cc', '#b44b3e', '#667547', '#c7cbd1')
TOTAL_CAP, BODY_CAP, MEMBERS = 136 << 20, 8 << 20, 601
WHOLE_NS = 3_600_000_000_000


def require(ok, why):
    if not ok:
        raise ValueError(why)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def decimal_ratio(numerator, denominator, scale=1):
    require(type(numerator) is int and type(denominator) is int and denominator >= 0,
            'integer ratio operands')
    if denominator == 0:
        return None
    with localcontext() as context:
        context.prec = 80
        return format((Decimal(numerator) * Decimal(scale) / Decimal(denominator)).quantize(
            Decimal('0.000001'), rounding=ROUND_HALF_EVEN), 'f')


def csv_bytes(header, rows):
    stream = io.StringIO(newline=''); writer = csv.writer(stream, lineterminator='\n')
    writer.writerow(header); writer.writerows(rows)
    return stream.getvalue().encode()


def uint(value, maximum=(1 << 64) - 1):
    require(type(value) is int and 0 <= value <= maximum, 'exact nonnegative integer bound')
    return value


def checked_sum(values):
    total = 0
    for value in values:
        total = uint(total + uint(value))
    return total


def reduce_durations(forwards, timeline, check_timeline):
    """The diagnostic is admitted first; these extra checks guard attribution."""
    timeline = check_timeline(timeline)
    require(type(forwards) is list and len(forwards) == 40, 'forty original diagnostic rows')
    positions = []
    callbacks = {group: {category: dict(calls=0, elapsed_ns=0) for category in CATEGORIES}
                 for group in GROUPS}
    for position, (row, parent) in enumerate(zip(forwards, timeline['forwards'])):
        require(type(row) is dict and set(row) == {'position', 'measured'}
                and uint(row['position']) == position == parent['position'], 'ordered diagnostic/parent join')
        result = dict(position=position, measured=position >= 2,
                      parent_forward_ns=uint(parent['elapsed_ns']))
        if position < 2:
            require(row['measured'] is None, 'first two ordinary forwards are unmeasured')
            result.update({key: None for key in (*PARTITION, 'bank_guarded_body_ns',
                'all_callbacks_ns', 'expanded_disjoint_ns')})
        else:
            measured = row['measured']
            require(type(measured) is dict and set(measured) ==
                    {'bank', 'layers', 'tail', 'bank_guarded_body_ns'}, 'closed measured forward')
            subtotals = {}
            for group, discoveries in zip(GROUPS, (2, 72, 2)):
                values = measured[group]
                require(type(values) is dict and set(values) == set(CATEGORIES), 'four serial callback categories')
                for category in CATEGORIES:
                    pair = values[category]
                    require(type(pair) is dict and set(pair) == {'calls', 'elapsed_ns'}
                            and uint(pair['calls']) > 0, 'closed positive callback count')
                    uint(pair['elapsed_ns'], WHOLE_NS)
                    for key in ('calls', 'elapsed_ns'):
                        callbacks[group][category][key] = checked_sum(
                            (callbacks[group][category][key], pair[key]))
                require(values['before']['calls'] == values['after']['calls']
                        and values['discover']['calls'] == discoveries, 'balanced exact scoped callback census')
                subtotals[group] = checked_sum(values[key]['elapsed_ns'] for key in CATEGORIES)
            require([measured['bank'][key]['calls'] for key in CATEGORIES] == [726, 2, 726, 725],
                    'fixed per-forward bank calls')
            bank = uint(measured['bank_guarded_body_ns'], WHOLE_NS)
            require(subtotals['bank'] <= bank, 'bank callbacks are contained, never additive to bank body')
            expanded = checked_sum((bank, subtotals['layers'], subtotals['tail']))
            require(expanded <= result['parent_forward_ns'], 'negative complete-parent-forward residual')
            result.update(bank_guarded_body_ns=bank, bank_callbacks_ns=subtotals['bank'],
                bank_other_ns=bank - subtotals['bank'], layer_callbacks_ns=subtotals['layers'],
                tail_callbacks_ns=subtotals['tail'], all_callbacks_ns=checked_sum(subtotals.values()),
                expanded_disjoint_ns=expanded,
                unattributed_parent_ns=result['parent_forward_ns'] - expanded)
            require(checked_sum(result[key] for key in PARTITION) == result['parent_forward_ns'],
                    'disjoint warm attribution, no callback/body double count')
        positions.append(result)
    warm = {key: checked_sum(row[key] for row in positions[2:]) for key in
            ('parent_forward_ns', *PARTITION, 'bank_guarded_body_ns', 'all_callbacks_ns', 'expanded_disjoint_ns')}
    require(checked_sum(warm[key] for key in PARTITION) == warm['parent_forward_ns'],
            'warm partition exhausts complete parent forwards')
    category_rows = [dict(group=group, category=category, **callbacks[group][category],
                         share_warm_parent_percent=decimal_ratio(
                             callbacks[group][category]['elapsed_ns'], warm['parent_forward_ns'], 100))
                     for group in GROUPS for category in CATEGORIES]
    totals = [timeline['source_preparation']['elapsed_ns'], timeline['spawn_to_setup_seal']['elapsed_ns']]
    totals += [checked_sum(row[key]['elapsed_ns'] for row in timeline['forwards'])
               for key in ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit')]
    totals += [timeline['close_and_retirement']['elapsed_ns'],
               timeline['postcheck_and_ordinary_publication']['elapsed_ns']]
    require(checked_sum(totals) == timeline['total_ns'], '124 parent spans alone exhaust total')
    groups = [dict(group=name, first_position=start, last_position=end - 1,
                   forwards=end - start, elapsed_ns=checked_sum(row['parent_forward_ns']
                       for row in positions[start:end]))
              for name, start, end in (('First use (unmeasured callbacks)', 0, 2), ('Warm', 2, 40), ('All forwards', 0, 40))]
    require(groups[0]['elapsed_ns'] + groups[1]['elapsed_ns'] == groups[2]['elapsed_ns']
            == checked_sum(totals[2:5]), 'forward groups are subsets, not extra parent categories')
    spans = []
    def append_span(region, position, span):
        spans.append(dict(index=len(spans), region=region, position=position, **span))
    for key in ('source_preparation', 'spawn_to_setup_seal'):
        append_span(key, None, timeline[key])
    for row in timeline['forwards']:
        for key in ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit'):
            append_span(key, row['position'], row[key])
    for key in ('close_and_retirement', 'postcheck_and_ordinary_publication'):
        append_span(key, None, timeline[key])
    require(len(spans) == 124, 'exact original parent span census')
    return dict(positions=positions, warm=warm, callbacks=category_rows, forward_groups=groups,
        parent_categories=[dict(category=name, elapsed_ns=value) for name, value in zip(PARENT_CATEGORIES, totals)],
        parent_spans=spans, parent_total_ns=timeline['total_ns'])


def summarize(observation, check_timeline):
    require(observation['passed'] is True and observation['selected_original_bodies'] == 166
            and observation['raw_files'] == 73 and observation['baseline_raw_files'] == 70
            and set(observation['cases']) == {CASE}
            and all(observation[key] is True for key in ('baseline_data_revalidated',
                'original_owned_lineage_revalidated', 'semantic_and_payload_parity_revalidated',
                'parent_host_measurement', 'original_policy_bytes_retained', 'instrumented',
                'currentness_duration_diagnostic', 'bank_guarded_body_includes_callbacks'))
            and all(observation[key] is False for key in ('gpu_timing', 'matched_speed_comparison',
                'native_rerun', 'gpu_execution', 'model_execution', 'numerical_reference_replayed',
                'numerical_acceptance', 'full_long_workload', 'performance_claim', 'production_authority')),
            'successful original diagnostic retention only')
    case = observation['cases'][CASE]
    require(case['present'] is True and case['original_passed'] is True
            and case['retained_success_revalidated'] is True and case['readiness_completed_forwards'] == 40
            and case['semantic_and_payload_parity_revalidated'] is True, 'successful original forty-forward case')
    checked = case['matched_timing']; policy = checked['policy']; timing = checked['timing']
    require(checked['schema'] == 'ferric-readiness40-tail-currentness-duration-case-v1'
            and checked['mode'] == CASE and checked['instrumented'] is True
            and policy['name'] == 'bank_scoped_census_tail_duration'
            and policy['no_policy_bytes_discarded'] is True and policy['original_stderr_empty'] is False
            and policy['diagnostic_record'] == checked['ordinary']['currentness_duration_diagnostic']
            and policy['policy_record'] == checked['ordinary']['bank_scoped_census_tail_policy'],
            'same authenticated whole stderr and both original records')
    require(timing['disjoint_spans'] == 124 and timing['parent_host_measurement'] is True
            and all(timing[key] is False for key in ('gpu_timing', 'nested_control_timers_included',
                'sidecar_publication_timed', 'full_long_workload', 'numerical_acceptance',
                'performance_claim', 'production_authority')), 'original disjoint parent clock partition')
    reduced = reduce_durations(policy['diagnostic_record']['forwards'], timing['timeline'], check_timeline)
    return dict(schema='ferric-readiness40-currentness-duration-host-attribution-v1', **reduced,
        terminal=case['original_terminal'], stderr=policy['file'], timing_file=timing['file'],
        policy_record=policy['policy_record'], diagnostic_record=policy['diagnostic_record'],
        cpu_and_elf_metadata=observation['cpu_and_elf_metadata'], checker_cpu=observation['checker_cpu'],
        instrumented=True, parent_host_measurement=True, exact_integer_nanoseconds=True,
        decimal_rounding='six_places_round_half_even', first_two_callbacks_unmeasured=True,
        bank_guarded_body_contains_bank_callbacks=True, callback_categories_serial=True,
        forward_groups_are_parent_subsets=True, gpu_timing=False, gpu_overlap_claim=False,
        matched_speed_comparison=False, tokens_per_second_claim=False, numerical_acceptance=False,
        full2303_feasibility=False, performance_claim=False, production_authority=False)


def attribution_svg(summary):
    width, height, left, top, chart = 1040, 370, 32, 112, 960
    warm = summary['warm']; total = warm['parent_forward_ns']
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="white"/>',
        '<g font-family="sans-serif" fill="#20252b">',
        '<text x="32" y="32" font-size="21">Instrumented serial host attribution: warm forwards 2-39</text>',
        '<text x="32" y="58" font-size="13">Complete parent-forward time; not GPU time, overlap, throughput or speedup.</text>',
        '<text x="32" y="80" font-size="12">Bank callbacks are part of the bank interval. First two ordinary forwards are unmeasured.</text>']
    cumulative = 0
    for key, label, color in zip(PARTITION, LABELS, COLORS):
        value = warm[key]
        x = Decimal(left) + Decimal(decimal_ratio(cumulative, total, chart) or '0')
        size = decimal_ratio(value, total, chart) or '0'
        out.append(f'<rect x="{x}" y="{top}" width="{size}" height="48" fill="{color}"/>')
        cumulative += value
    for index, (key, label, color) in enumerate(zip(PARTITION, LABELS, COLORS)):
        y = 195 + 27 * index
        out += [f'<rect x="32" y="{y-11}" width="13" height="13" fill="{color}"/>',
                f'<text x="54" y="{y}" font-size="13">{label}</text>',
                f'<text x="610" y="{y}" font-size="13">{decimal_ratio(warm[key], 1000000)} ms</text>']
    out += [f'<text x="32" y="350" font-size="12">Warm parent total: {decimal_ratio(total, 1000000)} ms. Other parent time includes unprofiled work and waits.</text>',
            '</g></svg>\n']
    return '\n'.join(out).encode()


def render(summary):
    callbacks = summary['callbacks']; positions = summary['positions']; spans = summary['parent_spans']
    header = ('position', 'measured', 'parent_forward_ns', *PARTITION, 'bank_guarded_body_ns',
              'all_callbacks_ns', 'expanded_disjoint_ns')
    lines = ['# Instrumented Readiness40 Host Attribution', '',
        'One admitted diagnostic run; integer nanoseconds below are host elapsed time, not GPU time or speedup.', '',
        '## Warm Serial Partition', '', '| Component | Nanoseconds | Milliseconds |', '| --- | ---: | ---: |']
    for key, label in zip(PARTITION, LABELS):
        value = summary['warm'][key]
        lines.append(f'| {label} | {value} | {decimal_ratio(value, 1000000)} |')
    lines += [f"| Complete warm parent forwards | {summary['warm']['parent_forward_ns']} | {decimal_ratio(summary['warm']['parent_forward_ns'], 1000000)} |", '',
        'Expanded disjoint attribution is B + Clayers + Ctail. Cbank is already inside B;',
        'the displayed split is Cbank + (B - Cbank) + Clayers + Ctail + residual.',
        'Residual is relative to each complete parent forward, never only the flush-to-frame-read span.', '',
        '## Callback Categories', '', '| Scope | Callback | Calls | Nanoseconds |', '| --- | --- | ---: | ---: |']
    lines += [f"| {r['group']} | {r['category']} | {r['calls']} | {r['elapsed_ns']} |" for r in callbacks]
    lines += ['', 'Callback categories are serial subtotals, not additions to the partition above.',
        'Positions 0 and 1 have no callback measurements; blank CSV cells mean unmeasured, not zero.', '',
        '## Parent Disjoint Spans', '', '| Category | Nanoseconds |', '| --- | ---: |']
    lines += [f"| {r['category']} | {r['elapsed_ns']} |" for r in summary['parent_categories']]
    lines += [f"| Total (124 disjoint spans) | {summary['parent_total_ns']} |", '',
        'Only these parent categories sum to the total. The following forward groups are subsets:', '',
        '| Forward subset | Forwards | Nanoseconds |', '| --- | ---: | ---: |']
    lines += [f"| {r['group']} | {r['forwards']} | {r['elapsed_ns']} |" for r in summary['forward_groups']]
    lines += ['', 'Instrumentation costs and observer effects are included. This is not an independent numerical reference,',
        'a matched comparison, a GPU overlap measurement, or Full2303 feasibility evidence.', '']
    return {'summary.json': encoded(summary), 'table.md': '\n'.join(lines).encode(),
        'callbacks.csv': csv_bytes(('group', 'category', 'calls', 'elapsed_ns', 'share_warm_parent_percent'),
            ([r[k] for k in ('group', 'category', 'calls', 'elapsed_ns', 'share_warm_parent_percent')] for r in callbacks)),
        'forwards.csv': csv_bytes(header, ([r[k] for k in header] for r in positions)),
        'parent-spans.csv': csv_bytes(('index', 'region', 'position', 'start_ns', 'end_ns', 'elapsed_ns'),
            ([r[k] for k in ('index', 'region', 'position', 'start_ns', 'end_ns', 'elapsed_ns')] for r in spans)),
        'attribution.svg': attribution_svg(summary)}


def source_bytes(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical standalone reviewed verifier')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size == VERIFIER['bytes'], 'fixed verifier extent')
    stamp = lambda row: (row.st_dev, row.st_ino, row.st_mode, row.st_size, row.st_mtime_ns, row.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'verifier changed at open')
        raw = stream.read(VERIFIER['bytes'] + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'verifier changed during read')
    require(stamp(path.lstat()) == stamp(before) and pin(raw) == VERIFIER, 'exact final retention source')
    return raw


def admit_archive(raw, verifier):
    require(type(raw) is bytes and 0 < len(raw) <= TOTAL_CAP, 'bounded original archive bytes')
    bodies = {}; total = 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for member in archive:
            verifier.tick()
            require(member.isfile() and verifier.ordinary(member.name) and member.name not in bodies
                    and not member.pax_headers and 0 <= member.size <= BODY_CAP, 'ordinary unique bounded archive member')
            total += member.size
            require(len(bodies) < MEMBERS and total <= TOTAL_CAP, 'bounded original archive expansion')
            with archive.extractfile(member) as stream:
                body = stream.read(member.size + 1)
            require(len(body) == member.size, 'exact original member extent')
            bodies[member.name] = body
    require('manifest.json' in bodies, 'original archive manifest')
    manifest_raw = bodies.pop('manifest.json'); manifest = verifier.parse(manifest_raw)
    require(len(bodies) == 166 and set(manifest) == {'schema', 'source_root', 'files', 'observation', 'outcomes'}
            and manifest['schema'] == 'ferric-guarded-mlp-readiness40-tail-currentness-duration-retention-v1'
            and manifest['source_root'] == str(verifier.REMOTE)
            and manifest['files'] == {name: pin(body) for name, body in bodies.items()},
            'closed167 original archive, never commentary or a reconstructed outcome')
    observation = verifier.verify(bodies, manifest['outcomes'])
    require(verifier.same(observation, manifest['observation']) and observation['passed'] is True,
            'successful original outcome independently revalidated')
    modules = verifier.validation_modules(bodies)
    require(len(modules) == 9, 'fixed nine-module diagnostic verifier ABI')
    return summarize(observation, modules[2].timeline), manifest_raw


def main():
    require(__debug__ and sys.flags.isolated and sys.dont_write_bytecode and len(sys.argv) == 5,
            'python3 -I -B duration_report.py FINAL_VERIFIER ORIGINAL_ARCHIVE OBSERVED_SHA FRESH_OUTPUT')
    verifier_path, archive, sha, out = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4])
    require(re.fullmatch('[0-9a-f]{64}', sha), 'observed original archive SHA required')
    deadline = time.monotonic() + 180
    def stop(_number, _frame): raise RuntimeError('bounded data-only report deadline')
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, stop)
    signal.setitimer(signal.ITIMER_REAL, 180); os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 768 << 20), (resource.RLIMIT_FSIZE, 2 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (bound, bound))
    source = source_bytes(verifier_path)
    verifier = types.ModuleType('authenticated_duration_retention')
    verifier.__file__ = str(verifier_path)
    exec(compile(source, str(verifier_path), 'exec'), verifier.__dict__)
    verifier.DEADLINE = deadline
    raw = verifier.read(archive, limit=TOTAL_CAP)
    require(pin(raw)['sha256'] == sha, 'exact observed archive bytes')
    summary, manifest_raw = admit_archive(raw, verifier)
    outputs = render(summary)
    outputs['provenance.json'] = encoded(dict(archive=dict(path=str(archive), **pin(raw)),
        manifest=pin(manifest_raw), verifier=VERIFIER, report_source=pin(verifier.read(Path(__file__).resolve())),
        original_members=167, all_originals_rehashed=True, original_data_revalidated=True,
        native_rerun=False, gpu_execution=False, model_execution=False, numerical_acceptance=False, performance_claim=False))
    require(sum(map(len, outputs.values())) <= 2 << 20 and all(len(b) <= 1 << 20 for b in outputs.values()),
            'bounded complete report output')
    require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent
            and not os.path.lexists(out) and out not in (archive, verifier_path), 'fresh report output only')
    require(verifier.read(archive, limit=TOTAL_CAP) == raw and source_bytes(verifier_path) == source,
            'original archive and verifier final posthash')
    out.mkdir(mode=0o700)
    for name, body in outputs.items():
        with (out / name).open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
        require(verifier.read(out / name) == body, 'generated report readback')
    verifier.tick(); signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(output=str(out), files={n: pin(b) for n, b in outputs.items()},
        data_only=True, gpu_timing=False, matched_speed_comparison=False, performance_claim=False), sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
