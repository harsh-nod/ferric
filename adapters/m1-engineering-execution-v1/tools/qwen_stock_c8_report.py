"""Separate disclosed C8 cell; does not modify C1 or quality admission."""
import math

from qwen_stock_c1_report import DISCLOSURE, distribution, token_hash, window_interval


def overlap(records):
    events = []
    for record in records:
        start, end = record.get('started_ns'), record.get('completed_ns')
        if type(start) is not int or type(end) is not int or end <= start:
            raise ValueError('invalid actual request interval')
        events.extend([(start, 1), (end, -1)])
    active = peak = 0
    for _, delta in sorted(events):
        active += delta
        peak = max(peak, active)
    return peak


def summarize(run, reference_tokens, c1_tokens):
    reference_hash, c1_hash = token_hash(reference_tokens), token_hash(c1_tokens)
    if (run.get('schema') != 'FerricCompetitiveTokenStreamingRunV1' or
            run.get('engine') != 'vllm' or run.get('concurrency') != 8 or
            run.get('arrival_policy') != 'bounded-closed-loop-windows'):
        raise ValueError('expected raw C8 closed-loop stock report')
    workload = run['workload']
    requests = workload['requests']
    ids = [r['id'] for r in requests]
    if (workload.get('schema') != 'FerricCompetitiveTokenWorkloadV1' or
            len(requests) != 8 or len(set(ids)) != 8 or
            any(len(r['prompt']) != 128 or r['prompt'] != requests[0]['prompt'] or
                r['max_tokens'] != 256 for r in requests)):
        raise ValueError('expected eight unique IDs and identical raw128/256 requests')
    counts, windows, parity, measured = {}, [], [], []
    elapsed = 0
    previous_end = None
    for phase, planned in [('warmups', 10), ('samples', 30)]:
        observed = run[phase]
        if len(observed) > planned:
            raise ValueError('excess windows')
        successes = recorded = 0
        for index, window in enumerate(observed):
            records = window['requests']
            if window['index'] != index or len(records) != 8 or {r['id'] for r in records} != set(ids):
                raise ValueError('invalid C8 window records')
            start, end = window_interval(window, previous_end)
            previous_end = end
            peak = overlap(records)
            duration = end - start
            if phase == 'samples':
                elapsed += duration
            windows.append({'phase': phase, 'index': index, 'actual_peak_outstanding': peak,
                            'requests': len(records), 'eight_actual_outstanding': peak == 8})
            for record in records:
                recorded += 1
                item = {'phase': phase, 'window': index, 'id': record['id'],
                        'success': record['success'], 'error': record.get('error'),
                        'reference_equal': False, 'retained_C1_stock_equal': False}
                if record['success']:
                    if any(type(record['usage'].get(k)) is not int or record['usage'].get(k) != v
                           for k, v in {'prompt_tokens': 128, 'completion_tokens': 256, 'total_tokens': 384}.items()):
                        raise ValueError('token accounting mismatch')
                    digest = token_hash(record['token_ids'])
                    item.update(token_u32le_sha256=digest, reference_equal=digest == reference_hash,
                                retained_C1_stock_equal=digest == c1_hash)
                    for key in ['ttft_ns', 'tpot_ns', 'e2e_ns']:
                        v = record[key]
                        if type(v) not in (int, float) or not math.isfinite(v) or v < 0:
                            raise ValueError('invalid successful timing')
                    successes += 1
                    if phase == 'samples':
                        measured.append(record)
                parity.append(item)
        counts[phase] = {'planned_windows': planned, 'recorded_windows': len(observed),
                         'missing_windows': planned - len(observed), 'planned_requests': planned * 8,
                         'recorded_requests': recorded, 'successful_requests': successes,
                         'failed_requests': recorded - successes, 'missing_requests': planned * 8 - recorded}
    complete = (run.get('completed') is True and run.get('run_error') is None and all(c['successful_requests'] == c['planned_requests']
                for c in counts.values()) and all(w['eight_actual_outstanding'] for w in windows))
    return {'schema': 'FerricDisclosedStockC8SummaryV1', 'authority': 'none',
            'disclosure': DISCLOSURE, 'ferric_comparison': False, 'performance_qualified': False,
            'tuned': False, 'independent_reference_equivalence': False,
            'reference_token_sha256': reference_hash, 'retained_C1_stock_token_sha256': c1_hash,
            'complete_declared_baseline': complete, 'counts': counts,
            'overlap': windows, 'overlap_semantics': 'actual client request-start to DONE intervals; not proof of eight GPU sequences executing simultaneously',
            'parity': parity, 'run_error': run.get('run_error'),
            **{name: distribution([r[key] / 1e6 for r in measured])
               for key, name in [('ttft_ns', 'ttft_ms'), ('tpot_ns', 'tpot_ms'), ('e2e_ns', 'e2e_ms')]},
            'successful_measured_requests': len(measured), 'successful_output_tokens': len(measured) * 256,
            'recorded_measurement_window_seconds': elapsed / 1e9,
            'closed_loop_output_tokens_per_second': len(measured) * 256 / (elapsed / 1e9) if elapsed else None,
            'rate_semantics': 'successful measured tokens / sum recorded measured window durations including failed windows; finite closed-loop, not steady-state/peak throughput',
            'warmup_windows_excluded': True, 'token_itl_available': False,
            'ttft_semantics': run.get('ttft_semantics'), 'tpot_semantics': run.get('tpot_semantics')}
