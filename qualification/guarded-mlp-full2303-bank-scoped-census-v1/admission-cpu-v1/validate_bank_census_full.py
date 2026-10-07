"""Original Full bank/census records; no launch, feasibility or numerical authority."""
import hashlib
import struct
from pathlib import Path

import validate_full as V
import validate_scoped_full as L
from validate_full import (CAPTURES, CONTROL_BYTES, FORWARDS, MAX_RETAINED_BYTES,
    PAYLOAD_BYTES, bootstrap, completion, control, encoded, keys, octets, ordered,
    parse, part, payload, request_paths, request_row, require, rust_pin, same, uint)

SCHEMA = 'ferric-full2303-bank-scoped-census-case-data-v1'
WRAPPER = 'FerricFull2303BankScopedCensusObservationV1'
FIELDS = ('schema execution_profile session device_ids child_pid worker_sha256 profile_sha256 '
    'registration_sha256 transcript_sha256 completed_forwards prompt_positions generated_token_count '
    'generated_tokens_sha256 capture_positions first_scoped_position layers_per_forward counts '
    'scoped_warm_currentness scoped_bank_rearm scoped_capacity_census full_entry_exit_per_scoped_bank '
    'full_entry_exit_per_scoped_layer participant_local_between_boundaries scope_includes_prefix_mlp_hidden '
    'allocation_preflights_outside_windows census_counters_are_layer_subset temporal_equivalent_to_full '
    'default_group_policy_unchanged shared_full_currentness cache_kernel_admission operational_currentness '
    'host_observer paired_hidden_reads paired_terminal native_closed full_long_workload '
    'numerical_acceptance performance_claim production_authority')
BANK_FIELDS = ('ordinary_initial_banks scoped_rearms final_generations full_discoveries '
               'local_checkpoints before_calls after_calls generation_probes')
SCOPED_LAYERS = 2301 * 36
SCOPED_BANKS = 2301


def bank_counts(value):
    keys(value, BANK_FIELDS)
    require(uint(value['ordinary_initial_banks'], 0xffffffff) == 2
            and uint(value['scoped_rearms'], 0xffffffff) == SCOPED_BANKS,
            'two initial banks and all2301 scoped rearms')
    generations = value['final_generations']
    require(type(generations) is list and len(generations) == 2
            and [uint(n) for n in generations] == [1152, 1151], 'both final bank generations')
    full = uint(value['full_discoveries'])
    local = uint(value['local_checkpoints'])
    before, after = uint(value['before_calls']), uint(value['after_calls'])
    probes = uint(value['generation_probes'])
    calls = uint(2 * local + 4 * SCOPED_BANKS)
    expected_probes = uint(2 * local + 3 * SCOPED_BANKS)
    require(full == 2 * SCOPED_BANKS and local >= SCOPED_BANKS
            and before == after == calls and probes == expected_probes,
            'checked Full bank-window counters only')
    return dict(ordinary_initial_banks=2, scoped_rearms=SCOPED_BANKS,
                final_generations=generations, full_discoveries=full,
                local_checkpoints=local, before_calls=before, after_calls=after,
                generation_probes=probes)


def census_counts(value):
    keys(value, 'warm_layers preflights rank_checkpoints owner_counts')
    require(uint(value['warm_layers'], 0xffffffff) == SCOPED_LAYERS
            and uint(value['preflights'], 0xffffffff) == 2 * SCOPED_LAYERS
            and uint(value['rank_checkpoints'], 0xffffffff) == 16 * SCOPED_LAYERS,
            'exact warm census subset')
    owners = value['owner_counts']
    require(type(owners) is list and len(owners) == 2
            and all(0 < uint(n, 2048) for n in owners),
            'two individual bounded reject-only owner assertions')
    return dict(warm_layers=SCOPED_LAYERS, preflights=2 * SCOPED_LAYERS,
                rank_checkpoints=16 * SCOPED_LAYERS, owner_counts=owners)


def counts(value):
    keys(value, 'layers banks census')
    layers = L.counts(value['layers'])
    banks = bank_counts(value['banks'])
    census = census_counts(value['census'])
    other = dict(layers)
    subset = census['rank_checkpoints']
    for field in ('local_checkpoints', 'before_calls', 'after_calls'):
        other[field] = uint(other[field] - subset)
    other['generation_probes'] = uint(other['generation_probes'] - uint(2 * subset))
    L.counts(other)
    return dict(layers=layers, banks=banks, census=census)


def validate_record(raw, summary):
    require(type(raw) is bytes and 0 < len(raw) <= 4096 and raw.endswith(b'\n'),
            'one bounded original Full bank/census policy')
    value = parse(raw)
    keys(value, FIELDS)
    b = summary['bootstrap']['sequence']
    require(summary['schema'] == 'FerricGuardedMlpFull2303ObservationV1'
            and summary['request']['schema'] == 'FerricGuardedMlpFull2303RequestV1'
            and b['profile'] == 'full2303'
            and uint(summary['completed_forwards']) == 2303
            and all(summary[k] is True for k in ('native_closed', 'child_exit_zero', 'process_group_absent')),
            'Full bank/census policy follows original complete healthy Full Close')
    tokens = summary['generated_tokens']
    require(type(tokens) is list and len(tokens) == 256, 'own committed 256-output history')
    history = b''.join(struct.pack('<I', uint(t, 151935)) for t in tokens)
    expected = dict(schema='FerricFull2303BankScopedCensusPolicyV1',
        execution_profile='Full2303BankScopedCensusCurrentnessV1', session=b['scope']['session'],
        device_ids=b['device_ids'], child_pid=b['scope']['child_identity'],
        worker_sha256=summary['request']['base']['worker']['sha256'],
        profile_sha256=summary['profile_sha256'], registration_sha256=b['registration'],
        transcript_sha256=summary['transcript_sha256'], completed_forwards=2303,
        prompt_positions=2048, generated_token_count=256,
        generated_tokens_sha256=list(hashlib.sha256(history).digest()),
        capture_positions=[0, 2047, 2048, 2302], first_scoped_position=2, layers_per_forward=36,
        counts=counts(value['counts']), scoped_warm_currentness=True,
        scoped_bank_rearm=True, scoped_capacity_census=True,
        full_entry_exit_per_scoped_bank=True, full_entry_exit_per_scoped_layer=True,
        participant_local_between_boundaries=True, scope_includes_prefix_mlp_hidden=True,
        allocation_preflights_outside_windows=False, census_counters_are_layer_subset=True,
        temporal_equivalent_to_full=False,
        default_group_policy_unchanged=True, shared_full_currentness=False,
        cache_kernel_admission=False, operational_currentness=False, host_observer=False,
        paired_hidden_reads=False, paired_terminal=False, native_closed=True,
        full_long_workload=True, numerical_acceptance=False, performance_claim=False,
        production_authority=False)
    require(same(value, expected) and encoded(expected) + b'\n' == raw,
            'canonical Full bank/census policy fields/order/identity/history/flags')
    require(uint(value['child_pid'], 0x7fffffff) == summary['child_pid']
            and all(octets(value[k]) != bytes(32) for k in
                ('profile_sha256', 'worker_sha256', 'transcript_sha256', 'generated_tokens_sha256')),
            'nonzero complete Full bank/census identity')
    return value



# Exact Full validator body except this private name, fixed callback and result slot.
def _validate_full(raw, request, directory, read, prompt, *, _policy_validator=None):
    if _policy_validator is not None:
        require(_policy_validator is validate_record, 'only fixed Full bank/census policy admission')
    require(type(raw) is bytes and 0 < len(raw) <= 128 << 10, 'bounded original parent summary')
    o = parse(raw)
    require(encoded(o) + b'\n' == raw, 'original compact parent summary serialization')
    keys(o, 'schema request child_pid bootstrap profile_sha256 registration_sha256 source_program_sha256 '
         'upload_manifest_sha256 setup_commands completed_forwards prompt_positions_executed decode_positions_executed '
         'decoded_special_token_policy generated_tokens '
         'page_permutation transcript_sha256 request_stream_bytes response_stream_bytes files close '
         'child_exit_zero process_group_absent native_closed gpu_execution full_long_workload '
         'numerical_acceptance performance_claim production_authority')
    require(o['schema'] == 'FerricGuardedMlpFull2303ObservationV1' and same(o['request'], request)
            and uint(o['child_pid'], 0x7fffffff) > 0 and uint(o['setup_commands']) > 0
            and uint(o['completed_forwards']) == FORWARDS and uint(o['prompt_positions_executed']) == 2048
            and uint(o['decode_positions_executed']) == 255 and o['decoded_special_token_policy'] == 'skip'
            and type(o['generated_tokens']) is list and len(o['generated_tokens']) == 256
            and all(type(t) is int and 0 <= t < 151936 for t in o['generated_tokens'])
            and all(o[k] is True for k in ('child_exit_zero', 'process_group_absent', 'native_closed', 'gpu_execution', 'full_long_workload'))
            and all(o[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority')),
            'closed Full2303 observation and own generated tokens')
    keys(request, 'schema base tiles_image prefix_image projection_image guarded_image')
    require(request['schema'] == 'FerricGuardedMlpFull2303RequestV1', 'readiness request schema')
    outer, digest = bootstrap(o['bootstrap'], request, o['child_pid'], prompt)
    request_paths(request, directory)
    b = outer['sequence']; begin = b['begin']
    require(octets(o['profile_sha256']) == digest and same(o['registration_sha256'], b['registration'])
            and same(o['source_program_sha256'], begin['source_program']['sha256'])
            and same(o['upload_manifest_sha256'], begin['uploads']['sha256']), 'profile/setup joins')
    pages = o['page_permutation']
    require(type(pages) is list and len(pages) == 144
            and sorted(uint(v, 143) for v in pages) == list(range(144)), 'retained full page permutation')
    f = o['files']; keys(f, 'frames captures child_stderr decoded_output rows bytes_before_summary summary_bytes total_bytes supervisor_metadata_allowance')
    require(uint(f['rows']) == FORWARDS and type(f['captures']) is list and len(f['captures']) == 4,
            'closed retained file roster')
    total = 0
    def body(value, name, maximum):
        nonlocal total
        pin = rust_pin(value)
        require(pin['path'] == str(directory / name) and pin['bytes'] <= maximum, 'exact ordinary retained path/extent')
        data = read(pin)
        require(type(data) is bytes and len(data) == pin['bytes'] and hashlib.sha256(data).hexdigest() == pin['sha256'], 'retained body hash')
        total += len(data)
        return data
    framed = body(f['frames'], 'frames.ndjson', FORWARDS * 8193)
    require(framed.endswith(b'\n') and len(framed.splitlines()) == FORWARDS, '2303 complete canonical records')
    capture_bodies = {}
    for position, row in zip(CAPTURES, f['captures']):
        keys(row, 'position file')
        require(uint(row['position']) == position, 'ordered selected capture positions')
        capture_bodies[position] = body(row['file'], 'capture-%d.bin' % position, CONTROL_BYTES + PAYLOAD_BYTES)
    stderr = body(f['child_stderr'], 'child-stderr.bin', (2 << 20) if _policy_validator is None else 4096)
    if _policy_validator is None:
        require(stderr == b'', 'unrequested observer/capture/diagnostic child stderr')
    decoded = body(f['decoded_output'], 'generated-text.bin', 128 << 10)
    chain = hashlib.sha256(b'ferric-guarded-mlp-long-transcript-v2\0' + digest).digest()
    previous = [[0, 0], [0, 0]]; outputs = []; selected = []; request_bytes = 0; response_bytes = 0
    for position, line in enumerate(framed.splitlines()):
        require(0 < len(line) <= 8192, 'frame record bound')
        frame = ordered(parse(line), 'schema profile request completion')
        require(frame['schema'] == 'FerricGuardedMlpLongResponseV2' and frame['profile'] == 'full2303', 'frame profile')
        r = request_row(frame['request'], b, digest, position, outputs[-1] if outputs else None); c = completion(frame['completion'], position)
        require(same(r['command']['cache_metadata'], [position] + pages)
                and c['input_token'] == r['command']['token'], 'stable KV/input token history')
        require(all(c['first_frontiers'][i][0] > previous[i][0]
                    and c['first_frontiers'][i][1] >= previous[i][1] for i in range(2)), 'cross-forward queue ordering')
        if position in CAPTURES:
            data = capture_bodies[position]
            require(len(data) == CONTROL_BYTES + PAYLOAD_BYTES, 'selected capture exact extent')
            controls, payload_raw = data[:CONTROL_BYTES], data[CONTROL_BYTES:]
            first, last = control(controls, position + 1, previous)
            winner, parts = payload(payload_raw)
            require(same(part(controls), c['control']) and same(parts['total'], c['observation'])
                    and same(parts['logits'], c['logits']) and winner == c['output_token']
                    and same(first, c['first_frontiers']) and same(last, c['final_frontiers']), 'selected original bodies/completion')
            selected.append(dict(position=position, output_token=winner, local_generation=position // 2 + 1,
                                 capture=part(data), layer_hidden=parts['layer_hidden'], logits=parts['logits']))
        saved = c['chain']; c['chain'] = [0] * 32
        chain = hashlib.sha256(chain + encoded(r) + encoded(c)).digest()
        require(octets(saved) == chain, 'full2303 chained original completion')
        c['chain'] = saved; frame['request'] = r; frame['completion'] = c
        require(encoded(frame) == line, 'canonical Rust field order/frame bytes')
        request_bytes += 4 + len(encoded(r)); response_bytes += 4 + len(line)
        if position in CAPTURES:
            response_bytes += CONTROL_BYTES + PAYLOAD_BYTES
        previous = c['final_frontiers']; outputs.append(c['output_token'])
    closed = ordered(o['close'], 'schema request completed_forwards generated_tokens transcript_sha256 native_closed '
                     'numerical_acceptance performance_claim production_authority')
    closed['request'] = request_row(closed['request'], b, digest, FORWARDS)
    require(closed['schema'] == 'FerricGuardedMlpFull2303ClosedV1' and uint(closed['completed_forwards']) == FORWARDS
            and same(closed['generated_tokens'], outputs[2047:])
            and same(o['generated_tokens'], outputs[2047:]) and closed['native_closed'] is True
            and all(closed[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority'))
            and octets(closed['transcript_sha256']) == octets(o['transcript_sha256']) == chain, 'exact healthy Close')
    setup = dict(protocol=1, id=1, device_ids=b['device_ids'], session=b['scope']['session'],
                 command=dict(op='begin', **begin))
    request_bytes += 4 + len(encoded(outer)) + sum(b[n]['bytes'] for n in ('prefix_image', 'mlp_image', 'projection_image', 'guarded_image'))
    request_bytes += 4 + len(encoded(setup)) + sum(begin[n]['bytes'] for n in ('registration', 'source_program', 'uploads',
                                      'prefix_image', 'mlp_image', 'residual_image', 'tail_image'))
    request_bytes += 4 + len(encoded(closed['request'])); response_bytes += 4 + len(encoded(closed))
    require(uint(o['request_stream_bytes'], 64 << 20) == request_bytes
            and uint(o['response_stream_bytes'], 64 << 20) == response_bytes, 'exact separate Full2303 framing budgets')
    require(uint(f['bytes_before_summary']) == total and uint(f['summary_bytes'], 128 << 10) == len(raw)
            and uint(f['total_bytes']) == total + len(raw) and uint(f['supervisor_metadata_allowance']) == 512 << 10
            and f['total_bytes'] + f['supervisor_metadata_allowance'] <= min(32 << 20, MAX_RETAINED_BYTES), 'bounded exact compact retention')
    checked = dict(schema='ferric-guarded-mlp-full2303-data-observation-v1', child_pid=o['child_pid'],
                completed_forwards=FORWARDS, prompt_positions_executed=2048, decode_positions_executed=255,
                generated_tokens=outputs[2047:], decoded_output=part(decoded), captures=selected,
                observed_argmax_tokens=outputs, final_queue_frontiers=previous, profile_sha256=digest.hex(),
                transcript_sha256=chain.hex(), all2303_transcript_checked=True, selected_payloads_independently_checked=4,
                unselected_payloads_independently_checked=False, full_long_workload=True,
                native_execution_reperformed=False, process_retirement_independently_checked=False,
                numerical_acceptance=False, performance_claim=False, production_authority=False)
    if _policy_validator is not None:
        checked['bank_scoped_census_policy'] = _policy_validator(stderr, o)
    return checked


def validate(stdout, summary_raw, request, native_root, read, prompt):
    require(type(stdout) is bytes and 0 < len(stdout) <= 128 << 10
            and type(summary_raw) is bytes and 0 < len(summary_raw) <= 128 << 10,
            'bounded original Full bank/census wrapper and ordinary summary')
    wrapper, summary = parse(stdout), parse(summary_raw)
    keys(wrapper, 'schema observation currentness_policy')
    require(wrapper['schema'] == WRAPPER and same(wrapper['observation'], summary),
            'explicit wrapper preserves original Full observation')
    ordinary = _validate_full(summary_raw, request, Path(native_root), read, prompt,
                          _policy_validator=validate_record)
    record = ordinary['bank_scoped_census_policy']
    require(same(wrapper['currentness_policy'], record), 'wrapper joins original unmodified stderr policy')
    return dict(schema=SCHEMA, ordinary=ordinary,
        policy=dict(name='full2303_bank_scoped_census', file=rust_pin(summary['files']['child_stderr']),
            policy_record=record, no_policy_bytes_discarded=True, original_stderr_empty=False,
            temporal_equivalent_to_full=False, shared_full_currentness=False,
            discovery_count_scope='separate-layer-and-bank-windows-census-is-layer-subset',
            census_counters_are_layer_subset=True, allocation_preflights_changed=True,
            owner_counts_runtime_recomputed=False),
        summary=dict(path=str(Path(native_root) / 'complete.json'), **V.part(summary_raw)),
        stdout=V.part(stdout), full_long_workload=True, generated_history_independently_checked=True,
        runtime_reexecuted=False, outer_owned_lineage_checked=False, cpu_qualification_checked=False,
        launch_feasibility_admitted=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
