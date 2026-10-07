"""Join two already-authenticated Readiness40 observations; diagnostics only."""
import struct

from common import compact, parse, require
import reference as R

CONTROL_BYTES = 242824


def compact_wire(value, extent):
    require(type(value) is dict and set(value) == {'bytes', 'sha256'}
            and type(value['bytes']) is int and value['bytes'] == extent
            and type(value['sha256']) is list and len(value['sha256']) == 32
            and all(type(n) is int and 0 <= n <= 255 for n in value['sha256']),
            'exact native wire pin')
    return dict(bytes=extent, sha256=bytes(value['sha256']).hex())


def compare(native, report, reference_payloads, contract, tokens):
    require(type(tokens) is list and len(tokens) == 2048
            and all(type(t) is int and 0 <= t < R.VOCABULARY for t in tokens), 'full authentic prompt')
    summary = parse(native['readiness/native/complete.json'])
    sequence = summary['bootstrap']['sequence']
    require(sequence['profile'] == 'readiness40' and sequence['prompt_tokens'] == tokens,
            'candidate full prompt/profile join')
    for key in ('model_id', 'bundle_id'):
        value = sequence['scope'][key]
        require(type(value) is list and len(value) == 32
                and all(type(n) is int and 0 <= n <= 255 for n in value)
                and bytes(value).hex() == contract[key] == report[key], 'same model/bundle identity')
    require(report['full_prompt_tokens'] == tokens and report['input_tokens'] == tokens[:40]
            and report['generated_tokens'] == 0 and report['native_intermediates_used'] is False
            and report['candidate_receipt'] is None, 'independent own-KV reference prompt history')
    lines = native['readiness/native/frames.ndjson'].splitlines()
    require(len(lines) == 40 and all(lines), 'exact candidate transcript length')
    frames = [parse(line) for line in lines]
    history, cases, outputs = [], {}, []
    for position, frame in enumerate(frames):
        request, value = frame['request'], frame['completion']
        require(frame['schema'] == 'FerricGuardedMlpLongResponseV2'
                and frame['profile'] == 'readiness40'
                and type(value['position']) is int and value['position'] == position
                and type(value['generation']) is int and value['generation'] == position + 1
                and type(value['input_token']) is int and value['input_token'] == tokens[position]
                and request['command']['op'] == 'forward'
                and type(request['command']['token']) is int
                and request['command']['token'] == tokens[position]
                and type(request['command']['generation']) is int
                and request['command']['generation'] == position + 1,
                'all40 exact native prompt positions, not generated feedback')
        R.uint(value['output_token'], R.VOCABULARY - 1)
        history.append(value['input_token'])
        selected = position in R.SELECTED
        require(value['captured'] is selected, 'four exact native capture selections')
        logit_pin = compact_wire(value['logits'], 2 * R.VOCABULARY)
        if selected:
            raw = native['readiness/native/capture-%d.bin' % position]
            require(len(raw) == CONTROL_BYTES + R.PAYLOAD_BYTES, 'exact control/payload boundary')
            payload = raw[CONTROL_BYTES:]
            require(compact(raw[:CONTROL_BYTES]) == compact_wire(value['control'], CONTROL_BYTES)
                    and compact(payload) == compact_wire(value['observation'], R.PAYLOAD_BYTES)
                    and compact(R.D.split_payload(payload)['logits']) == logit_pin,
                    'selected native control, payload and logits pin join')
            cases[position] = dict(position=position, input_token=tokens[position],
                output_token=value['output_token'], payload=payload)
        reference_case = report['passes'][0]['cases'][position]
        outputs.append(dict(position=position, input_token=tokens[position],
            candidate_output_token=value['output_token'],
            reference_output_token=reference_case['predicted_token'],
            output_token_equal=value['output_token'] == reference_case['predicted_token'],
            candidate_logits=logit_pin, reference_logits=reference_case['logits'],
            selected_capture=selected, candidate_argmax_recomputed=selected,
            reference_argmax_recomputed=selected,
            unselected_argmax_scope='authenticated original records; logits body not retained' if not selected else None))
    selected = R.compare_selected(report['passes'], reference_payloads, cases, tokens, history)
    require(selected['tensor_rows'] == 152 and len(selected['comparisons']) == 4,
            'exact four by38 tensor diagnostics')
    return dict(schema='ferric-readiness40-actual-reference-diagnostic-v1',
        selected=selected, argmax_diagnostics=outputs,
        argmax_matches=sum(row['output_token_equal'] for row in outputs),
        argmax_mismatches=[{k: row[k] for k in ('position', 'input_token',
            'candidate_output_token', 'reference_output_token', 'selected_capture',
            'candidate_argmax_recomputed', 'reference_argmax_recomputed')}
            for row in outputs if not row['output_token_equal']],
        argmax_positions=40, selected_positions=list(R.SELECTED), tensor_rows=152,
        full_prompt_sha256=compact(struct.pack('<2048I', *tokens))['sha256'],
        model_id=contract['model_id'], bundle_id=contract['bundle_id'],
        candidate_generated_tokens=0, reference_generated_tokens=0,
        own_kv_caches=True, numerical_acceptance=False, acceptance_threshold=None,
        full_model_acceptance=False, full_long_workload=False, performance_claim=False,
        production_authority=False, gpu_execution=False, model_execution=False)
