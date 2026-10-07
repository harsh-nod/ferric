"""Pure admission of original bank-scoped-warm policy and optional parent timing bytes."""
import hashlib
from pathlib import Path

import validate_readiness as V
import validate_shared as S
import validate_matched as M
import validate_scoped as C
from validate_readiness import keys, octets, parse, require, rust_pin, same, uint

SCHEMA = 'ferric-readiness40-bank-scoped-warm-case-data-v2'
WRAPPERS = {'bank_timed': 'FerricReadiness40Position5BankScopedWarmTimedObservationV2'}
FIELDS = ('schema execution_profile session device_ids child_pid worker_sha256 profile_sha256 '
    'registration_sha256 transcript_sha256 completed_forwards generated_tokens capture_positions counts '
    'scoped_warm_currentness scoped_bank_rearm full_entry_exit_per_scoped_bank allocation_preflights_outside_windows full_entry_exit_per_scoped_layer participant_local_between_boundaries '
    'scope_includes_prefix_mlp_hidden temporal_equivalent_to_full default_group_policy_unchanged '
    'shared_full_currentness cache_kernel_admission operational_currentness host_observer '
    'paired_hidden_reads paired_terminal native_closed full_long_workload numerical_acceptance '
    'performance_claim production_authority')
BANK_FIELDS = ('ordinary_initial_banks scoped_rearms scoped_rearms_by_forward final_generations '
    'full_discoveries local_checkpoints before_calls after_calls generation_probes')
U64_MAX = (1 << 64) - 1


def counts(value):
    keys(value, 'layers banks')
    layers = C.counts(value['layers'])
    bank = value['banks']
    keys(bank, BANK_FIELDS)
    require(uint(bank['ordinary_initial_banks']) == 2 and uint(bank['scoped_rearms']) == 38
            and same(bank['scoped_rearms_by_forward'], [0, 0] + [1] * 38)
            and same(bank['final_generations'], [20, 20]),
            'two ordinary initial banks then exact38 ordered warm rearms')
    full = uint(bank['full_discoveries'], U64_MAX)
    local = uint(bank['local_checkpoints'], U64_MAX)
    before = uint(bank['before_calls'], U64_MAX)
    after = uint(bank['after_calls'], U64_MAX)
    probes = uint(bank['generation_probes'], U64_MAX)
    calls = uint(4 * 38 + 2 * local, U64_MAX)
    expected_probes = uint(3 * 38 + 2 * local, U64_MAX)
    require(full == 76 and local >= 38 and before == after == calls and probes == expected_probes,
            'separate observed scoped-bank window counter relations')
    banks = dict(ordinary_initial_banks=2, scoped_rearms=38, scoped_rearms_by_forward=[0, 0] + [1] * 38,
        final_generations=[20, 20], full_discoveries=full, local_checkpoints=local,
        before_calls=before, after_calls=after, generation_probes=probes)
    return dict(layers=layers, banks=banks)


def validate_record(raw, summary):
    require(type(raw) is bytes and 0 < len(raw) <= 4096 and raw.endswith(b'\n'),
            'one bounded original bank-scoped-warm policy record')
    value = parse(raw)
    keys(value, FIELDS)
    b = summary['bootstrap']['sequence']
    require(b['profile'] == 'readiness40_position5'
            and summary['schema'] == 'FerricGuardedMlpReadiness40Position5ObservationV1'
            and summary['request']['schema'] == 'FerricGuardedMlpReadiness40Position5RequestV1'
            and all(summary[k] is True for k in ('native_closed', 'child_exit_zero', 'process_group_absent')),
            'scoped policy only after original Position5 healthy Close')
    expected = dict(schema='FerricReadiness40Position5BankScopedWarmPolicyV2',
        execution_profile='Readiness40Position5BankScopedWarmCurrentnessV2',
        session=b['scope']['session'], device_ids=b['device_ids'], child_pid=b['scope']['child_identity'],
        worker_sha256=summary['request']['base']['worker']['sha256'],
        profile_sha256=summary['profile_sha256'], registration_sha256=b['registration'],
        transcript_sha256=summary['transcript_sha256'], completed_forwards=40, generated_tokens=[],
        capture_positions=[0, 5, 16, 39], counts=counts(value['counts']),
        scoped_warm_currentness=True, scoped_bank_rearm=True, full_entry_exit_per_scoped_bank=True,
        allocation_preflights_outside_windows=True, full_entry_exit_per_scoped_layer=True,
        participant_local_between_boundaries=True, scope_includes_prefix_mlp_hidden=True,
        temporal_equivalent_to_full=False, default_group_policy_unchanged=True,
        shared_full_currentness=False, cache_kernel_admission=False, operational_currentness=False,
        host_observer=False, paired_hidden_reads=False, paired_terminal=False, native_closed=True,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False)
    require(same(value, expected) and V.encoded(expected) + b'\n' == raw,
            'canonical scoped policy order/flags/identity/worker/transcript/framing')
    require(uint(value['child_pid'], 0x7fffffff) == summary['child_pid']
            and uint(summary['completed_forwards']) == 40 and summary['generated_tokens'] == []
            and all(octets(value[k]) != bytes(32) for k in
                ('profile_sha256', 'worker_sha256', 'transcript_sha256')),
            'complete scoped identity without zero hashes')
    return value


def validate(mode, stdout, summary_raw, request, native_root, read, prompt):
    require(type(mode) is str and mode in WRAPPERS, 'explicit bank-scoped timed admission')
    require(type(stdout) is bytes and 0 < len(stdout) <= 128 << 10
            and type(summary_raw) is bytes and 0 < len(summary_raw) <= 128 << 10,
            'bounded original scoped wrapper and ordinary summary')
    wrapper, summary = parse(stdout), parse(summary_raw)
    keys(wrapper, 'schema observation currentness_policy' + (' host_timing' if mode == 'bank_timed' else ''))
    require(wrapper['schema'] == WRAPPERS[mode] and same(wrapper['observation'], summary),
            'explicit scoped wrapper preserves original ordinary observation')
    checked = V.validate(summary_raw, request, Path(native_root), read, prompt,
                         policy_validator=validate_record)
    # V's frozen callback slot predates scoped mode. Rename only its private
    # result key, never stderr bytes or the actual parsed policy record.
    record = checked.pop('shared_full_policy')
    checked['bank_scoped_warm_policy'] = record
    require(same(wrapper['currentness_policy'], record), 'wrapper joins original scoped worker policy')
    timing = None
    if mode == 'bank_timed':
        # This qualified helper validates only the unchanged timing schema and
        # accounting after ordinary admission; it does not select SharedFull.
        timing = M.shared_timing(wrapper, summary_raw, checked, native_root, read)
    return dict(schema=SCHEMA, mode=mode, ordinary=checked,
        policy=dict(name='bank_scoped_warm', file=rust_pin(summary['files']['child_stderr']),
            policy_record=record, no_policy_bytes_discarded=True, original_stderr_empty=False,
            temporal_equivalent_to_full=False, shared_full_currentness=False),
        timing=timing, summary=M.summary_pin(summary_raw, native_root), stdout=V.part(stdout),
        parent_host_measurement=mode == 'bank_timed', gpu_timing=False,
        nested_control_timers_included=False, full_long_workload=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False,
        outer_owned_lineage_checked=False, cpu_qualification_checked=False)


def compare_same_side(current_raw, baseline_raw, current_checked, baseline_checked, read):
    require(current_checked['schema'] == SCHEMA and current_checked['mode'] in WRAPPERS
            and current_checked['policy']['name'] == 'bank_scoped_warm'
            and current_checked['policy']['no_policy_bytes_discarded'] is True
            and current_checked['policy']['temporal_equivalent_to_full'] is False
            and current_checked['policy']['shared_full_currentness'] is False
            and 'shared_full_policy' not in current_checked['ordinary']
            and all(current_checked[k] is False for k in ('gpu_timing', 'nested_control_timers_included',
                'full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')),
            'separately admitted scoped currentness, not SharedFull or numerical authority')
    current = parse(current_raw)
    require(same(current_checked['summary'], M.summary_pin(current_raw,
                current['request']['base']['evidence_directory']))
            and same(current_checked['policy']['file'], rust_pin(current['files']['child_stderr']))
            and same(current_checked['policy']['policy_record'], current_checked['ordinary']['bank_scoped_warm_policy']),
            'parity exact admitted original scoped summary and policy')
    pin = current_checked['policy']['file']; raw = read(pin)
    require(type(raw) is bytes and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'parity original scoped policy pin')
    require(same(validate_record(raw, current), current_checked['policy']['policy_record']),
            'parity rechecks original scoped record, never sanitized')
    parity = S.compare_same_side(current_raw, baseline_raw, current_checked['ordinary'], baseline_checked, read)
    parity['schema'] = 'ferric-readiness40-bank-scoped-warm-same-side-parity-v2'
    parity['bank_scoped_warm_policy'] = current_checked['policy']
    parity['temporal_equivalent_to_full'] = False
    parity['shared_full_currentness'] = False
    return parity
