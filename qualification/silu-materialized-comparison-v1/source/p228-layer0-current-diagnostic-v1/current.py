"""Data-only current layer-zero diagnostics; caller authenticates native ownership."""
from pathlib import Path
import struct

import compare as C

COMPARE_SHA = '1598e22a3460a9ed2c350fb5d2b5fec6b8c37648b53bb2f1fc714c1fb3506d3f'
CURRENT_HIDDEN_SHA = 'faa56202578a3d3497bbe779137736439957e473775bd6f677dbce466a9d6979'
CAPTURE_BYTES = 9670656
STAGES = tuple((name, count, 4 if name in ('output-partial', 'down-partial') else 2)
               for name, count in C.HISTORICAL_LAYOUT)
FILES = ('request.json', 'candidate-registration.json', 'candidate-program.json',
         'candidate-uploads.json', 'candidate-bootstrap.json', 'candidate-request-1.json',
         'candidate-response-1.json', 'candidate-request-2.json', 'candidate-response-2.json',
         'candidate-capture.bin', 'candidate-stderr.bin')
ORDER = ('norm', 'qkv', 'query', 'key-current', 'value-current', 'attention',
         'first-residual', 'mlp-norm', 'gate', 'up', 'activation-product', 'final-hidden')


def wire_pin(value):
    C.require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}, 'wire FilePin')
    return dict(value, sha256=C.wire(value['sha256']))


def identity(value):
    pin = wire_pin(value)
    return pin['bytes'], pin['sha256']


def part(raw):
    return dict(bytes=len(raw), sha256=list(bytes.fromhex(C.sha(raw))))


def native_files(summary, read):
    directory = Path(summary['request']['evidence_directory'])
    C.require(directory.is_absolute() and '..' not in directory.parts,
              'absolute original candidate evidence directory')
    records = summary['files']
    C.require(type(records) is list and len(records) == 11
              and [p['name'] for p in records] == list(FILES), 'exact candidate-only evidence roster')
    bodies, pins = {}, {}
    for value in records:
        C.require(set(value) == {'name', 'bytes', 'sha256'}, 'closed native evidence File')
        name = value['name']
        pin = dict(path=str(directory / name), bytes=value['bytes'], sha256=C.wire(value['sha256']))
        maximum = (CAPTURE_BYTES if name == 'candidate-capture.bin' else
                   2 << 20 if name == 'candidate-stderr.bin' else
                   4 << 20 if name in FILES[1:4] else 65536)
        if name == 'candidate-stderr.bin' and pin['bytes'] == 0:
            raw = read(pin)
            C.require(type(raw) is bytes and raw == b'' and C.sha(raw) == pin['sha256'], 'empty closed child stderr pin')
        else:
            raw = C.checked_read(read, pin, maximum)
        bodies[name], pins[name] = raw, pin
    C.require(C.document(bodies['request.json']) == summary['request'], 'summary request is actual retained request')
    C.require(C.document(bodies['candidate-bootstrap.json']) == summary['run']['bootstrap'], 'actual retained bootstrap')
    C.require(C.document(bodies['candidate-response-2.json']) == summary['run']['close'], 'actual retained Close')
    return bodies, pins


def current_request(summary, tf4_request):
    request, old = summary['request'], tf4_request['decode']
    C.require(request['schema'] == 'FerricFinitePrefixLayerCaptureRequestV1'
              and tf4_request['schema'] == 'FerricFinitePrefixDecodeDeviceClockRequestV2', 'distinct actual native modes')
    for key in ('source', 'expected_model_id', 'expected_bundle_id', 'device_ids', 'dispatch_timeout_ms'):
        C.require(request[key] == old[key], 'same current native source/model/device: ' + key)
    C.require(C.wire(request['expected_model_id']) == C.MODEL
              and C.wire(request['expected_bundle_id']) == C.BUNDLE, 'same authentic Qwen3 model and bundle')
    C.require(identity(request['worker']) == identity(old['worker']), 'same actually qualified worker bytes')
    C.require(set(request['images']) == set(old['images']) == {'prefix', 'mlp', 'residual', 'tail'}, 'original image roster')
    for name in request['images']:
        C.require(identity(request['images'][name]) == identity(old['images'][name]), 'same original bootstrap image')
    C.require(identity(request['prefix_tiles_image']) == identity(old['prefix_image']) == (53560, C.V7)
              and identity(request['mlp_tiles_image']) == identity(old['tiles_image']) == (33112, C.DOWN2),
              'actual explicit V7 prefix and Down2 MLP; not original bootstrap images')
    C.require(set(request['prompt']) == set(old['prompt']) == {'manifest', 'text', 'tokens'}, 'closed genuine prompt roster')
    for name in request['prompt']:
        C.require(identity(request['prompt'][name]) == identity(old['prompt'][name]), 'same original prompt bytes')
    # Session, child PID and profile digest are scoped to each execution, never
    # compared across independently owned runs.
    C.require(C.wire(request['session']) != '0' * 64, 'fresh scoped session identity')
    return request


def framework_prompt(report, request):
    records = report['input_pins']
    C.require(type(records) is list and records, 'actual framework input pin roster')
    for role in ('manifest', 'tokens'):
        selected = wire_pin(request['prompt'][role])
        C.require(any(type(pin) is dict and set(pin) == {'path', 'bytes', 'sha256'}
                      and (pin['bytes'], pin['sha256']) == (selected['bytes'], selected['sha256'])
                      for pin in records), 'framework/native authentic prompt bytes: ' + role)


def bootstrap_page(summary, bodies):
    record, request = summary['run'], summary['request']
    b = record['bootstrap']; scope = b['begin']['scope']; value = b['input']
    C.require(type(record['child_pid']) is int and record['child_pid'] > 0
              and record['child_exit_zero'] is True and record['process_group_absent'] is True
              and type(record['setup_commands']) is int and record['setup_commands'] > 0,
              'recorded single child natural close/reap; outer ownership still caller-authenticated')
    C.require(b['protocol'] == 1 and b['profile'] == 'prefix284_mlp548'
              and b['device_ids'] == request['device_ids'] and b['timeout_ms'] == request['dispatch_timeout_ms']
              and scope['child_identity'] == record['child_pid'] and scope['session'] == request['session']
              and scope['model_id'] == request['expected_model_id'] and scope['bundle_id'] == request['expected_bundle_id'],
              'own bootstrap child/session/model/profile')
    for role in ('prefix', 'mlp'):
        selected = request[role + '_tiles_image']
        C.require(b[role + '_image'] == dict(bytes=selected['bytes'], sha256=selected['sha256']), 'actual explicit image part')
    for role in ('prefix', 'mlp', 'residual', 'tail'):
        selected = request['images'][role]
        C.require(b['begin'][role + '_image'] == dict(bytes=selected['bytes'], sha256=selected['sha256']), 'actual original image part')
    for field, name in (('registration', FILES[1]), ('source_program', FILES[2]), ('uploads', FILES[3])):
        C.require(b['begin'][field] == part(bodies[name]), 'bootstrap authentic source body: ' + field)
    C.require(value['generation'] == 1 and value['token'] == 9112
              and value['rotary_bits'] == [0x3f800000] * 64 + [0] * 64, 'genuine first token/position-zero rotary')
    metadata = value['cache_metadata']
    C.require(type(metadata) is list and len(metadata) == 145 and metadata[0] == 0
              and all(type(v) is int for v in metadata) and sorted(metadata[1:]) == list(range(144)),
              'actual complete physical-page permutation; not an assumed identity map')
    profile = record['profile_sha256']
    C.require(C.wire(profile) != '0' * 64, 'own profile digest')
    for ordinal, command in ((1, 'run'), (2, 'close')):
        expected = dict(protocol=1, id=ordinal, profile_sha256=profile, command=command)
        C.require(C.document(bodies[f'candidate-request-{ordinal}.json']) == expected, 'actual own Run/Close order')
        response = C.document(bodies[f'candidate-response-{ordinal}.json'])
        C.require(response['protocol'] == 1 and response['id'] == ordinal and response['profile_sha256'] == profile
                  and response['profile'] == b['profile'] and response['completed_layers'] == 1
                  and response['native_closed'] is (ordinal == 2) and response['gpu_execution'] is True
                  and all(response[name] is False for name in ('numerical_acceptance', 'performance_claim', 'production_authority')),
                  'recorded native response scope and nonclaims')
        if ordinal == 1:
            C.require(response['capture'] is None and response['control'] is None, 'no pre-Close capture')
        else:
            C.require(response['capture'] == part(bodies['candidate-capture.bin']) and type(response['control']) is dict,
                      'actual closed payload; root replays terminal-control/runtime validation')
    return metadata[1]


def stage_bytes(summary, raw, page, current_hidden, diagnostics):
    C.require(type(raw) is bytes and len(raw) == CAPTURE_BYTES and type(page) is int and 0 <= page < 144, 'capture extent/page')
    C.require(type(current_hidden) is bytes and len(current_hidden) == 8192
              and C.sha(current_hidden) == CURRENT_HIDDEN_SHA, 'authenticated current TF4 layer-zero slice')
    records = summary['stages']
    C.require(type(records) is list and len(records) == 28, 'exact two-rank stage roster')
    parts, offset = [], 0
    for rank in range(2):
        values = {}
        for ordinal, (name, count, width) in enumerate(STAGES):
            row = records[rank * 14 + ordinal]
            body = raw[offset:offset + count]
            expected = dict(rank=rank, stage=name, offset=offset, bytes=count, elements=count // width,
                            element_bytes=width, sha256=list(bytes.fromhex(C.sha(body))))
            C.require(row == expected, 'actual stage offset/shape/digest: ' + name)
            if width == 2:
                diagnostics.words(body, count // 2)
            else:
                C.require(all(v & 0x7f800000 != 0x7f800000 for (v,) in struct.iter_unpack('<I', body)), 'finite FP32 partial')
            if name in ('key-cache', 'value-cache'):
                start = page * 16 * 1024
                C.require(not any(body[:start]) and not any(body[start + 1024:]), 'untouched KV outside actual current slot')
            values[name] = body
            offset += count
        C.require(values['final-hidden'] == current_hidden, 'both rank final-hidden rows match actual current TF4 bytes')
        parts.append(values)
    C.require(offset == len(raw), 'exact capture partition')
    return parts


def comparisons(framework, parts, page, diagnostics):
    C.require(set(framework) == set(C.SHAPES) and len(parts) == 2, 'complete independent framework and native ranks')
    rows = []
    for rank, values in enumerate(parts):
        start = page * 16 * 1024
        qkv = (framework['q-projection'][rank * 4096:(rank + 1) * 4096]
               + framework['k-projection'][rank * 1024:(rank + 1) * 1024]
               + framework['v-projection'][rank * 1024:(rank + 1) * 1024])
        pairs = ((framework['input-norm'], values['norm']), (qkv, values['qkv']),
            (framework['rotary-q'][rank * 4096:(rank + 1) * 4096], values['query']),
            (framework['cache-key'][rank * 1024:(rank + 1) * 1024], values['key-cache'][start:start + 1024]),
            (framework['cache-value'][rank * 1024:(rank + 1) * 1024], values['value-cache'][start:start + 1024]),
            (framework['attention-output'][rank * 4096:(rank + 1) * 4096], values['attention']),
            (framework['first-residual'], values['first-residual']), (framework['post-norm'], values['mlp-norm']),
            (framework['gate'][rank * 12288:(rank + 1) * 12288], values['gate']),
            (framework['up'][rank * 12288:(rank + 1) * 12288], values['up']),
            (framework['product'][rank * 12288:(rank + 1) * 12288], values['activation']),
            (framework['layer0-hidden'], values['final-hidden']))
        for ordinal, (name, (reference, actual)) in enumerate(zip(ORDER, pairs)):
            rows.append(dict(rank=rank, observable_order=ordinal, **C.diagnostic(diagnostics, name, reference, actual)))
    rows.sort(key=lambda row: (row['observable_order'], row['rank']))
    different = [row for row in rows if not row['byte_equal']]
    earliest = None
    if different:
        first = different[0]
        earliest = dict(stage=first['stage'], observable_order=first['observable_order'],
                        ranks=[row['rank'] for row in different if row['observable_order'] == first['observable_order']])
    return rows, earliest


def compare_retained(framework_pin, original_framework_pin, current_tf4_pin, capture_summary_pin, read, diagnostics):
    """Caller verifies both successful, reaped outer executions before invoking."""
    primary, framework = C.compare_retained(framework_pin, original_framework_pin, current_tf4_pin, read, diagnostics)
    C.require(primary['original_framework_matches_new'] is True, 'new genuine framework reproduces original layer0')
    tf4 = C.document(C.checked_read(read, current_tf4_pin, 4 << 20))
    tf4_request = C.document(C.checked_read(read, tf4['request'], 1 << 20))
    current = C.current_hidden(tf4, tf4_request, read, diagnostics)
    summary = C.document(C.checked_read(read, capture_summary_pin, 65536))
    C.require(summary['schema'] == 'FerricFinitePrefixLayerCaptureObservationV1'
              and summary['native_attempts'] == 1 and summary['retries'] == 0 and summary['completed_layers'] == 1
              and summary['native_closed'] is True and summary['gpu_execution'] is True
              and all(summary[name] is False for name in ('paired_comparison_performed', 'numerical_acceptance',
                  'performance_claim', 'production_authority', 'full_forward'))
              and 'bitwise_equal' not in summary and 'runs' not in summary, 'independent one-owner native capture, not paired success')
    current_request(summary, tf4_request)
    framework_prompt(C.document(C.checked_read(read, framework_pin, 4 << 20)), summary['request'])
    bodies, pins = native_files(summary, read)
    page = bootstrap_page(summary, bodies)
    parts = stage_bytes(summary, bodies['candidate-capture.bin'], page, current, diagnostics)
    rows, earliest = comparisons(framework, parts, page, diagnostics)
    return dict(schema='ferric-p228-layer0-current-intermediate-diagnostic-v1', position=0, input_token=9112,
        inputs=dict(framework_capture=framework_pin, original_framework=original_framework_pin,
                    current_tf4=current_tf4_pin, native_capture_summary=capture_summary_pin), native_files=pins,
        physical_page=page, current_tf4_final_hidden_sha256=C.sha(current), both_rank_final_hidden_match_current_tf4=True,
        primary=primary, comparisons=rows, comparable_rows=len(rows), earliest_observable_divergence=earliest,
        observation_order=list(ORDER), ordering_is_a_causal_proof=False,
        excluded_fp32_partials=[row for row in summary['stages'] if row['element_bytes'] == 4],
        fp32_partials_finite=True, fp32_partials_compared_to_full_bf16=False,
        genuine_independent_framework_outputs=True, current_V7_Down2_intermediate_evidence=True,
        candidate_intermediate_inputs=False, conditional_replay_performed=False,
        cumulative_chain_differences_not_isolated_operator_errors=True, receipt_authentication=False,
        numerical_acceptance=False, acceptance_threshold=None, full_model_correctness=False,
        production_authority=False, performance_measured=False, gpu_execution=False)
