"""Original Full-scoped bytes only; no launcher, feasibility or numerical authority."""
import hashlib
import struct
from pathlib import Path

import validate_full as V
from validate_full import encoded, keys, octets, parse, require, rust_pin, same, uint

SCHEMA = 'ferric-full2303-scoped-warm-case-data-v1'
WRAPPER = 'FerricFull2303ScopedWarmObservationV1'
FIELDS = ('schema execution_profile session device_ids child_pid worker_sha256 profile_sha256 '
    'registration_sha256 transcript_sha256 completed_forwards prompt_positions generated_token_count '
    'generated_tokens_sha256 capture_positions first_scoped_position layers_per_forward counts '
    'scoped_warm_currentness full_entry_exit_per_scoped_layer participant_local_between_boundaries '
    'scope_includes_prefix_mlp_hidden temporal_equivalent_to_full default_group_policy_unchanged '
    'shared_full_currentness cache_kernel_admission operational_currentness host_observer '
    'paired_hidden_reads paired_terminal native_closed full_long_workload numerical_acceptance '
    'performance_claim production_authority')
COUNT_FIELDS = ('ordinary_layers scoped_layers full_discoveries local_checkpoints before_calls '
                'after_calls generation_probes')
SCOPED_LAYERS = 2301 * 36
U64_MAX = (1 << 64) - 1


def counts(value):
    keys(value, COUNT_FIELDS)
    require(uint(value['ordinary_layers'], 0xffffffff) == 72
            and uint(value['scoped_layers'], 0xffffffff) == SCOPED_LAYERS,
            'two ordinary first uses and every remaining Full layer')
    full = uint(value['full_discoveries'])
    local = uint(value['local_checkpoints'])
    before, after = uint(value['before_calls']), uint(value['after_calls'])
    probes = uint(value['generation_probes'])
    lower = uint(local + 4 * SCOPED_LAYERS)
    upper = uint(2 * local + 4 * SCOPED_LAYERS)
    expected_probes = uint(2 * local + 3 * SCOPED_LAYERS)
    require(full == 2 * SCOPED_LAYERS and local >= SCOPED_LAYERS
            and before == after and lower <= before <= upper and probes == expected_probes,
            'actual scoped-window counters, not whole-route discoveries')
    return dict(ordinary_layers=72, scoped_layers=SCOPED_LAYERS, full_discoveries=full,
                local_checkpoints=local, before_calls=before, after_calls=after,
                generation_probes=probes)


def validate_record(raw, summary):
    require(type(raw) is bytes and 0 < len(raw) <= 4096 and raw.endswith(b'\n'),
            'one bounded original Full-scoped policy')
    value = parse(raw)
    keys(value, FIELDS)
    b = summary['bootstrap']['sequence']
    require(summary['schema'] == 'FerricGuardedMlpFull2303ObservationV1'
            and summary['request']['schema'] == 'FerricGuardedMlpFull2303RequestV1'
            and b['profile'] == 'full2303'
            and uint(summary['completed_forwards']) == 2303
            and all(summary[k] is True for k in ('native_closed', 'child_exit_zero', 'process_group_absent')),
            'Full-scoped policy follows original complete healthy Full Close')
    tokens = summary['generated_tokens']
    require(type(tokens) is list and len(tokens) == 256, 'own committed 256-output history')
    history = b''.join(struct.pack('<I', uint(t, 151935)) for t in tokens)
    expected = dict(schema='FerricFull2303ScopedWarmPolicyV1',
        execution_profile='Full2303ScopedWarmCurrentnessV1', session=b['scope']['session'],
        device_ids=b['device_ids'], child_pid=b['scope']['child_identity'],
        worker_sha256=summary['request']['base']['worker']['sha256'],
        profile_sha256=summary['profile_sha256'], registration_sha256=b['registration'],
        transcript_sha256=summary['transcript_sha256'], completed_forwards=2303,
        prompt_positions=2048, generated_token_count=256,
        generated_tokens_sha256=list(hashlib.sha256(history).digest()),
        capture_positions=[0, 2047, 2048, 2302], first_scoped_position=2, layers_per_forward=36,
        counts=counts(value['counts']), scoped_warm_currentness=True,
        full_entry_exit_per_scoped_layer=True, participant_local_between_boundaries=True,
        scope_includes_prefix_mlp_hidden=True, temporal_equivalent_to_full=False,
        default_group_policy_unchanged=True, shared_full_currentness=False,
        cache_kernel_admission=False, operational_currentness=False, host_observer=False,
        paired_hidden_reads=False, paired_terminal=False, native_closed=True,
        full_long_workload=True, numerical_acceptance=False, performance_claim=False,
        production_authority=False)
    require(same(value, expected) and encoded(expected) + b'\n' == raw,
            'canonical Full-scoped policy fields/order/identity/history/flags')
    require(uint(value['child_pid'], 0x7fffffff) == summary['child_pid']
            and all(octets(value[k]) != bytes(32) for k in
                ('profile_sha256', 'worker_sha256', 'transcript_sha256', 'generated_tokens_sha256')),
            'nonzero complete Full-scoped identity')
    return value


def validate(stdout, summary_raw, request, native_root, read, prompt):
    require(type(stdout) is bytes and 0 < len(stdout) <= 128 << 10
            and type(summary_raw) is bytes and 0 < len(summary_raw) <= 128 << 10,
            'bounded original Full-scoped wrapper and ordinary summary')
    wrapper, summary = parse(stdout), parse(summary_raw)
    keys(wrapper, 'schema observation currentness_policy')
    require(wrapper['schema'] == WRAPPER and same(wrapper['observation'], summary),
            'explicit wrapper preserves original Full observation')
    ordinary = V.validate(summary_raw, request, Path(native_root), read, prompt,
                          _policy_validator=validate_record)
    record = ordinary['scoped_warm_policy']
    require(same(wrapper['currentness_policy'], record), 'wrapper joins original unmodified stderr policy')
    return dict(schema=SCHEMA, ordinary=ordinary,
        policy=dict(name='full2303_scoped_warm', file=rust_pin(summary['files']['child_stderr']),
            policy_record=record, no_policy_bytes_discarded=True, original_stderr_empty=False,
            temporal_equivalent_to_full=False, shared_full_currentness=False,
            discovery_count_scope='scoped-layer-windows-only'),
        summary=dict(path=str(Path(native_root) / 'complete.json'), **V.part(summary_raw)),
        stdout=V.part(stdout), full_long_workload=True, generated_history_independently_checked=True,
        runtime_reexecuted=False, outer_owned_lineage_checked=False, cpu_qualification_checked=False,
        launch_feasibility_admitted=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
