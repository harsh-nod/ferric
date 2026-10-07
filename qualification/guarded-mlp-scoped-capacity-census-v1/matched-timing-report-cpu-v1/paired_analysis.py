"""Report one retained matched pair's disjoint parent wall spans; no GPU or model work."""
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
import sys
import tarfile
import time
import types

VERIFIER = {"bytes":48238,"sha256":"9979b224e7e8225c8a76963bc9b498a42f16f063c24005c99bd1129debf81527"}
CATEGORIES = ('Source preparation', 'Spawn through setup', 'Prepare/write', 'Wait for frame',
              'Validate/retain/commit', 'Close and retirement', 'Postcheck and publication')
COLORS = ('#147d92', '#b44b3e')
TOTAL_CAP, BODY_CAP, MEMBERS = 136 << 20, 8 << 20, 601


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


def compare_ns(bank, census):
    require(type(bank) is int and type(census) is int and 0 <= bank < 1 << 64
            and 0 <= census < 1 << 64, 'exact u64 durations')
    return dict(bank_ns=bank, census_ns=census, difference_ns=census - bank,
        census_over_bank=decimal_ratio(census, bank),
        change_percent=decimal_ratio(census - bank, bank, 100))


def summarize(observation, check_timeline):
    """Consume only the successful result of the exact retained-pair verifier."""
    require(observation['passed'] is True and observation['selected_original_bodies'] == 242
            and observation['raw_files'] == 147 and observation['baseline_raw_files'] == 70
            and observation['baseline_data_revalidated'] is True
            and observation['both_original_owned_lineages_revalidated'] is True
            and observation['matched_semantic_and_payload_parity_revalidated'] is True
            and observation['parent_host_measurement'] is True
            and observation['original_policy_bytes_retained'] is True
            and observation['census_scoped_case_retained'] is True
            and observation['bank_scoped_rearm_case_retained'] is True
            and observation['allocation_preflights_changed'] is True
            and observation['census_counters_are_layer_subset'] is True
            and observation['shared_full_currentness_requested'] is False
            and observation['currentness_temporal_equivalence_claim'] is False
            and all(observation[k] is False for k in ('gpu_timing', 'native_rerun', 'gpu_execution',
                'model_execution', 'numerical_reference_replayed', 'numerical_acceptance',
                'full_long_workload', 'performance_claim', 'production_authority')),
            'successful authenticated pair with census data-only authority')
    pair = observation['matched_pair']; cases = observation['cases']
    require(set(cases) == {'bank', 'census'} and pair['passed'] is True
            and pair['schema'] == 'ferric-readiness40-bank-scoped-census-matched-timed-parity-v3'
            and pair['temporal_equivalent_to_full'] is False
            and pair['shared_full_currentness'] is False
            and pair['allocation_preflights_changed'] is True
            and pair['census_counters_are_layer_subset'] is True
            and set(pair['same_elf_cpu_metadata']) == {'parent', 'parent_cpu', 'worker', 'worker_cpu'},
            'explicit matched case and CPU/product roster')
    require({name: {key: value[key] for key in ('bytes', 'sha256')}
             for name, value in pair['same_elf_cpu_metadata'].items()} == observation['cpu_and_elf_metadata'],
            'same actual qualified parent/worker metadata')
    parity = pair['parity']
    require(parity['passed'] is True and len(parity['records']) == 40 and len(parity['captures']) == 4
            and all(parity[k] is True for k in ('all40_records_equal', 'all40_observation_pins_equal',
                'all40_logit_pins_equal', 'all4_payloads_byte_equal', 'controls_individually_validated'))
            and all(parity[k] is False for k in ('controls_byte_equality_claimed',
                'independent_numerical_reference', 'numerical_acceptance', 'performance_claim', 'production_authority')),
            'all records and complete selected payload parity, not nested timer equality')
    timelines = {}; totals = {}; positions = []; categories = []
    for mode in ('bank', 'census'):
        case = cases[mode]; checked = case['matched_timing']
        require(case['present'] is True and case['original_passed'] is True
                and case['retained_success_revalidated'] is True
                and case['readiness_completed_forwards'] == 40
                and case['semantic_and_payload_parity_revalidated'] is True
                and checked['schema'] == ('ferric-readiness40-bank-scoped-warm-case-data-v2' if mode == 'bank'
                    else 'ferric-readiness40-bank-scoped-census-case-data-v3')
                and checked['mode'] == ('bank_timed' if mode == 'bank' else 'census_timed')
                and checked['policy']['name'] == ('bank_scoped_warm' if mode == 'bank' else 'bank_scoped_census'),
                'separately admitted explicit timing modes')
        require(checked['policy'] == pair[mode + '_policy']
                and checked['policy']['no_policy_bytes_discarded'] is True
                and checked['policy']['original_stderr_empty'] is False
                and checked['policy']['temporal_equivalent_to_full'] is False
                and checked['policy']['shared_full_currentness'] is False,
                'both original policy identities, not SharedFull or temporal equivalence')
        if mode == 'census':
            require(checked['allocation_preflights_changed'] is True
                    and checked['census_counters_are_layer_subset'] is True,
                    'explicit candidate census subset, not a relabelled V2 case')
        timing = checked['timing']
        require(timing == pair[mode + '_timing'] and timing['disjoint_spans'] == 124
                and timing['parent_host_measurement'] is True
                and all(timing[k] is False for k in ('gpu_timing', 'nested_control_timers_included',
                    'sidecar_publication_timed', 'full_long_workload', 'numerical_acceptance',
                    'performance_claim', 'production_authority')), 'original parent-only timeline')
        timeline = check_timeline(timing['timeline'])
        timelines[mode] = timeline
        totals[mode] = [timeline['source_preparation']['elapsed_ns'], timeline['spawn_to_setup_seal']['elapsed_ns']]
        totals[mode] += [sum(row[key]['elapsed_ns'] for row in timeline['forwards'])
                         for key in ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit')]
        totals[mode] += [timeline['close_and_retirement']['elapsed_ns'],
                         timeline['postcheck_and_ordinary_publication']['elapsed_ns']]
        require(sum(totals[mode]) == timeline['total_ns'], 'disjoint categories exhaust original total')
    for index, name in enumerate(CATEGORIES):
        categories.append(dict(category=name, **compare_ns(totals['bank'][index], totals['census'][index])))
    for position, (bank, census) in enumerate(zip(timelines['bank']['forwards'], timelines['census']['forwards'])):
        require(bank['position'] == census['position'] == position
                and bank['generation'] == census['generation'] == position + 1, 'paired actual position order')
        positions.append(dict(position=position, generation=position + 1,
            captured=position in (0, 5, 16, 39),
            **{key: compare_ns(bank[key]['elapsed_ns'], census[key]['elapsed_ns'])
               for key in ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit')},
            total=compare_ns(bank['elapsed_ns'], census['elapsed_ns'])))
    groups = []
    metrics = ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit', 'total')
    for name, first, end in (('First use (0-1)', 0, 2), ('Warm (2-39)', 2, 40), ('All forwards', 0, 40)):
        groups.append(dict(group=name, first_position=first, last_position=end-1, forwards=end-first,
            **{key: compare_ns(sum(row[key]['bank_ns'] for row in positions[first:end]),
                              sum(row[key]['census_ns'] for row in positions[first:end])) for key in metrics}))
    for mode in ('bank', 'census'):
        require(groups[0]['total'][mode+'_ns'] + groups[1]['total'][mode+'_ns']
                == groups[2]['total'][mode+'_ns']
                == sum(totals[mode][2:5]), 'first-use plus warm equals disjoint all-forward spans')
    return dict(schema='ferric-readiness40-bank-scoped-census-matched-parent-wall-analysis-v1',
        categories=categories, positions=positions, forward_groups=groups,
        total=compare_ns(timelines['bank']['total_ns'], timelines['census']['total_ns']),
        terminals={mode: cases[mode]['original_terminal'] for mode in cases},
        sidecars={mode: cases[mode]['matched_timing']['timing']['file'] for mode in cases},
        same_elf_cpu_metadata=pair['same_elf_cpu_metadata'], checker_cpu=observation['checker_cpu'],
        exact_integer_nanoseconds=True, decimal_rounding='six_places_round_half_even',
        one_ordered_pair=True, parent_host_measurement=True, generated_tokens=0,
        currentness_policies=dict(bank='bank_scoped_warm', census='bank_scoped_census'),
        policy_records={mode: cases[mode]['matched_timing']['policy']['policy_record'] for mode in cases},
        allocation_preflights_changed=True, census_counters_are_layer_subset=True, temporal_equivalent_to_full=False,
        shared_full_currentness=False, original_policy_bytes_retained=True,
        disjoint_spans_per_case=124, nested_timers_added=False, gpu_timing=False,
        gpu_overlap_claim=False, tokens_per_second_claim=False, ttft_claim=False,
        repeated_benchmark=False, general_speedup_claim=False, full2303_feasibility=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)


def csv_bytes(header, rows):
    stream = io.StringIO(newline=''); writer = csv.writer(stream, lineterminator='\n')
    writer.writerow(header); writer.writerows(rows)
    return stream.getvalue().encode()


def wait_svg(summary):
    width, height = 1060, 540
    left, right, top, bottom = 86, 34, 108, 76
    plot_width, plot_height = width - left - right, height - top - bottom
    rows = summary['positions']
    maximum = max(1, max(row['flush_to_frame_read'][mode + '_ns'] for row in rows for mode in ('bank', 'census')))
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="white"/>', '<g font-family="sans-serif" fill="#20252b">',
        '<text x="24" y="30" font-size="21">Matched Readiness40 parent frame waits</text>',
        '<text x="24" y="53" font-size="13">One ordered Bank V2 / Census V3 pair; same ELF pair and full payload parity required.</text>',
        '<text x="24" y="73" font-size="12">Wait includes worker execution, pipe wait and framing. Not GPU time or throughput.</text>']
    for index, (mode, label) in enumerate((('bank', 'Bank V2'), ('census', 'Census V3'))):
        x = 700 + index * 140
        out += [f'<line x1="{x}" y1="90" x2="{x+24}" y2="90" stroke="{COLORS[index]}" stroke-width="3"/>',
                f'<text x="{x+31}" y="94" font-size="12">{label}</text>']
    warm_x = left + 1.5 * plot_width / 39
    out.append(f'<rect x="{warm_x:.3f}" y="{top}" width="{width-right-warm_x:.3f}" height="{plot_height}" fill="#f3f5f7"/>')
    for tick in range(5):
        y = top + plot_height - tick * plot_height / 4
        out += [f'<line x1="{left}" y1="{y}" x2="{width-right}" y2="{y}" stroke="#dfe4e8"/>',
                f'<text x="{left-10}" y="{y+4}" text-anchor="end" font-size="12">{maximum*tick/4/1e9:.3f}</text>']
    for position in (0, 5, 16, 39):
        x = left + position * plot_width / 39
        out.append(f'<line x1="{x}" y1="{top}" x2="{x}" y2="{top+plot_height}" stroke="#d1d7dc" stroke-dasharray="3 4"/>')
    for index, mode in enumerate(('bank', 'census')):
        points = [(left + row['position'] * plot_width / 39,
                   top + plot_height * (1 - row['flush_to_frame_read'][mode + '_ns'] / maximum)) for row in rows]
        out.append(f'<polyline fill="none" stroke="{COLORS[index]}" stroke-width="2" points="' +
                   ' '.join(f'{x:.3f},{y:.3f}' for x, y in points) + '"/>')
        for row, (x, y) in zip(rows, points):
            if row['captured']:
                out.append(f'<circle cx="{x:.3f}" cy="{y:.3f}" r="3" fill="{COLORS[index]}"/>')
    for position in (0, 5, 10, 16, 20, 25, 30, 35, 39):
        x = left + position * plot_width / 39
        out.append(f'<text x="{x}" y="{top+plot_height+22}" text-anchor="middle" font-size="12">{position}</text>')
    out += [f'<text x="{left+plot_width/2}" y="{height-20}" text-anchor="middle" font-size="13">Prompt position (shaded: warm 2-39; markers: captures 0, 5, 16, 39)</text>',
            f'<text x="20" y="{top+plot_height/2}" transform="rotate(-90 20 {top+plot_height/2})" text-anchor="middle" font-size="13">Parent wall seconds</text>', '</g></svg>\n']
    return '\n'.join(out).encode()


def groups_svg(summary):
    width, height = 1060, 370
    rows = [('Total parent', summary['total'])]
    rows += [(row['group'], row['total']) for row in
             (summary['forward_groups'][2], summary['forward_groups'][0], summary['forward_groups'][1])]
    maximum = max(1, max(row[mode+'_ns'] for _, row in rows for mode in ('bank', 'census')))
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="white"/>', '<g font-family="sans-serif" fill="#20252b">',
        '<text x="24" y="28" font-size="20">Parent totals and forward groups</text>',
        '<text x="24" y="51" font-size="12">Rows overlap: all forwards = first use + warm. Total also includes preparation, Close and publication.</text>']
    for index, label in enumerate(('Bank V2 control', 'Census V3 candidate')):
        x = 230 + index*260
        out += [f'<rect x="{x}" y="68" width="24" height="10" fill="{COLORS[index]}"/>',
                f'<text x="{x+32}" y="78" font-size="12">{label}</text>']
    for index, (label, row) in enumerate(rows):
        y = 105 + index*55
        out.append(f'<text x="24" y="{y+17}" font-size="13">{label}</text>')
        for side, mode in enumerate(('bank', 'census')):
            ns = row[mode+'_ns']; span = ns*620/maximum
            out += [f'<rect x="215" y="{y+side*16}" width="{span:.3f}" height="11" fill="{COLORS[side]}"><title>{ns} ns</title></rect>',
                    f'<text x="860" y="{y+10+side*16}" font-size="12">{decimal_ratio(ns, 10**9)} s</text>']
    out += ['<text x="24" y="350" font-size="12">One ordered same-ELF pair. Parent wall measurement only; no GPU time, throughput or long-run claim.</text>',
            '</g></svg>\n']
    return '\n'.join(out).encode()


def render(observation, check_timeline):
    summary = summarize(observation, check_timeline)
    metrics = ('bank_ns', 'census_ns', 'difference_ns', 'census_over_bank', 'change_percent')
    categories = csv_bytes(['category', *metrics], [[row['category'], *[row[key] for key in metrics]]
                           for row in [*summary['categories'], dict(category='Total', **summary['total'])]])
    keys = ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit', 'total')
    positions = csv_bytes(['position', 'generation', 'captured', *[key + '_' + metric for key in keys for metric in metrics]],
        [[row['position'], row['generation'], row['captured'], *[row[key][metric] for key in keys for metric in metrics]]
         for row in summary['positions']])
    groups = csv_bytes(['group', 'first_position', 'last_position', 'forwards',
        *[key+'_'+metric for key in keys for metric in metrics]],
        [[row['group'], row['first_position'], row['last_position'], row['forwards'],
          *[row[key][metric] for key in keys for metric in metrics]] for row in summary['forward_groups']])
    table = ['# Matched Readiness40 Parent Wall Spans', '',
        'One ordered same-ELF pair with all 40 records and four complete payloads equal. '
        'These are disjoint parent wall spans, not GPU time, overlap, throughput or a repeated benchmark.', '',
        '| Parent interval | Bank V2 seconds | Census V3 seconds | Candidate / Control | Change |',
        '| --- | ---: | ---: | ---: | ---: |']
    for row in [*summary['categories'], dict(category='Total', **summary['total'])]:
        ratio = row['census_over_bank'] or 'n/a'
        change = (row['change_percent'] + '%') if row['change_percent'] is not None else 'n/a'
        table.append('| ' + ' | '.join([row['category'], decimal_ratio(row['bank_ns'], 10**9),
            decimal_ratio(row['census_ns'], 10**9), ratio, change]) + ' |')
    table += ['', '## Forward Groups', '',
        'These subsets are not extra time to add to the parent total. All forwards = first use + warm.', '',
        '| Forward interval | Bank V2 seconds | Census V3 seconds | Candidate / Control | Change |',
        '| --- | ---: | ---: | ---: | ---: |']
    for group in summary['forward_groups']:
        row = group['total']; ratio = row['census_over_bank'] or 'n/a'
        change = row['change_percent']+'%' if row['change_percent'] is not None else 'n/a'
        table.append('| ' + ' | '.join([group['group'], decimal_ratio(row['bank_ns'], 10**9),
            decimal_ratio(row['census_ns'], 10**9), ratio, change]) + ' |')
    table += ['', 'The frame-wait category includes worker execution, pipe waiting and framing. '
        'Nested control timers and sidecar publication are excluded. Zero-denominator ratios are n/a. '
        'Both cases scope bank rearm and layers; the candidate additionally scopes the two warm-layer allocation censuses. '
        'Neither policy claims temporal equivalence to repeated full currentness. '
        'The census counters are a subset of the candidate layer counters, not extra time or work to add. No numerical acceptance or Full2303 feasibility is inferred.', '']
    bodies = {'summary.json': encoded(summary), 'categories.csv': categories, 'positions.csv': positions,
              'forward-groups.csv': groups, 'table.md': '\n'.join(table).encode(),
              'waits.svg': wait_svg(summary), 'groups.svg': groups_svg(summary)}
    require(all(len(body) <= 1 << 20 for body in bodies.values()) and sum(map(len, bodies.values())) <= 2 << 20,
            'bounded report output')
    return bodies


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 5,
            'python3 -B paired_analysis.py RETAINED_CAPSULE ORIGINAL_ARCHIVE ACTUAL_ARCHIVE_SHA FRESH_OUTPUT')
    capsule, archive, sha, out = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4])
    require(re.fullmatch('[0-9a-f]{64}', sha), 'observed original archive SHA')
    deadline = time.monotonic() + 180
    def stop(_number, _frame): raise RuntimeError('data-only report deadline or signal')
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP): signal.signal(number, stop)
    signal.setitimer(signal.ITIMER_REAL, 180); os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 768 << 20), (resource.RLIMIT_FSIZE, 2 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        cap = min([cap] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    require(type(VERIFIER) is dict and set(VERIFIER) == {'bytes', 'sha256'}
            and type(VERIFIER['bytes']) is int and VERIFIER['bytes'] > 0
            and type(VERIFIER['sha256']) is str and re.fullmatch('[0-9a-f]{64}', VERIFIER['sha256']),
            'actual reviewed bound census retained-pair verifier pin is pending')
    path = capsule / 'retention_tool.py'
    require(path.is_absolute() and path.resolve(strict=True) == path and path.is_file()
            and path.stat().st_size == VERIFIER['bytes'], 'canonical pinned verifier file')
    source = path.read_bytes(); require(pin(source) == VERIFIER, 'exact reviewed retained verifier')
    verifier = types.ModuleType('matched_retained_data'); verifier.__file__ = str(path)
    exec(compile(source, str(path), 'exec'), verifier.__dict__); verifier.DEADLINE = deadline
    raw = verifier.read(archive, limit=TOTAL_CAP); require(pin(raw)['sha256'] == sha, 'observed original archive')
    bodies = {}; total = 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for member in tar:
            verifier.tick()
            require(member.isfile() and verifier.ordinary(member.name) and member.name not in bodies
                    and not member.pax_headers and 0 <= member.size <= BODY_CAP, 'ordinary original archive member')
            total += member.size
            require(len(bodies) < MEMBERS and total <= TOTAL_CAP, 'bounded original archive')
            body = tar.extractfile(member).read(member.size + 1)
            require(len(body) == member.size and verifier.read(capsule / member.name) == body,
                    'every retained original equals the observed archive member')
            bodies[member.name] = body
    require('manifest.json' in bodies, 'original manifest')
    manifest_raw = bodies.pop('manifest.json'); manifest = verifier.parse(manifest_raw)
    require(len(bodies) == 242 and set(manifest) == {'schema', 'source_root', 'files', 'observation', 'outcomes'}
            and manifest['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-matched-timing-retention-v1'
            and manifest['source_root'] == str(verifier.REMOTE)
            and manifest['files'] == {name: pin(body) for name, body in bodies.items()}, 'original closed member hashes')
    require(verifier.roster(capsule, MEMBERS + 32) <= set(bodies) | {'manifest.json', 'retention.json'},
            'no unrecorded retained evidence bodies')
    observation = verifier.verify(bodies, manifest['outcomes'])
    require(verifier.same(observation, manifest['observation']) and observation['passed'] is True,
            'both original outcomes revalidated before reporting')
    modules = verifier.validation_modules(bodies)
    require(len(modules) == 8, 'fixed V2/V3 validator module closure')
    timing = modules[2]
    outputs = render(observation, timing.timeline)
    outputs['provenance.json'] = encoded(dict(archive=pin(raw), manifest=pin(manifest_raw), verifier=VERIFIER,
        report_source=pin(Path(__file__).read_bytes()), original_members=243,
        all_originals_rehashed=True, pair_data_revalidated=True, native_rerun=False,
        gpu_execution=False, model_execution=False, numerical_acceptance=False, performance_claim=False))
    require(sum(map(len, outputs.values())) <= 2 << 20, 'bounded complete report output')
    require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent and not os.path.lexists(out)
            and out != capsule and capsule not in out.parents, 'fresh report outside original capsule')
    require(verifier.read(archive, limit=TOTAL_CAP) == raw, 'original archive final posthash')
    for name, body in dict(bodies, **{'manifest.json': manifest_raw}).items():
        require(verifier.read(capsule / name) == body, 'original retained member posthash')
    out.mkdir(mode=0o700)
    for name, body in outputs.items():
        require(len(body) <= 1 << 20, 'bounded report file')
        with (out / name).open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
        require(verifier.read(out / name) == body, 'report readback')
    verifier.tick(); signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(output=str(out), files={name: pin(body) for name, body in outputs.items()},
        parent_host_measurement=True, gpu_timing=False, performance_claim=False), sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
