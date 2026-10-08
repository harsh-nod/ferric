"""Pure, mode-explicit admission of original timed Default/SharedFull evidence."""
import hashlib
from pathlib import Path
import re

import validate_readiness as V
import validate_shared as S
import validate_timing as T
from validate_readiness import keys, octets, parse, require, rust_pin, same

SCHEMA = 'ferric-readiness40-matched-timed-case-data-v1'
WRAPPERS = {
    'default': 'FerricReadiness40Position5TimedObservationV1',
    'shared': 'FerricReadiness40Position5SharedFullTimedObservationV1',
}


def summary_pin(raw, root):
    return dict(path=str(Path(root) / 'complete.json'), bytes=len(raw),
                sha256=hashlib.sha256(raw).hexdigest())


def shared_timing(wrapper, summary_raw, checked, native_root, read):
    # This is the existing timing admission with the actual combined wrapper.
    # No rewritten stdout, replacement stderr, or invented ordinary wrapper is used.
    summary = parse(summary_raw)
    require(checked['schema'] == T.OBSERVATION and checked['completed_forwards'] == 40
            and checked['generated_tokens'] == [] and checked['all40_transcript_checked'] is True
            and checked['selected_payloads_independently_checked'] == 4,
            'ordinary admission must precede timing admission')
    root = Path(native_root); original = summary_pin(summary_raw, root)
    status = wrapper['host_timing']
    keys(status, 'complete file ordinary_retained_bytes retained_bytes_with_timing supervisor_metadata_allowance parent_host_measurement gpu_timing numerical_acceptance performance_claim')
    require(status['complete'] is True and status['parent_host_measurement'] is True
            and all(status[k] is False for k in ('gpu_timing', 'numerical_acceptance', 'performance_claim')),
            'timing status complete with false authority')
    pin = rust_pin(status['file'])
    require(pin['path'] == str(root / 'host-timing.json') and 0 < pin['bytes'] <= 64 << 10,
            'closed timing sidecar path/extent')
    raw = read(pin)
    require(type(raw) is bytes and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'timing original bytes')
    report = parse(raw)
    keys(report, 'schema ordinary_complete profile_sha256 transcript_sha256 completed_forwards generated_tokens capture_positions timeline native_closed child_exit_zero process_group_absent parent_host_measurement ' + ' '.join(T.FALSE_FIELDS))
    require(report['schema'] == 'FerricReadiness40Position5ParentHostTimingV1'
            and rust_pin(report['ordinary_complete']) == original
            and octets(report['profile_sha256']) == octets(summary['profile_sha256'])
            and octets(report['transcript_sha256']) == octets(summary['transcript_sha256'])
            and T.u64(report['completed_forwards']) == 40 and T.u64(report['generated_tokens']) == 0
            and same(report['capture_positions'], [0, 5, 16, 39]), 'timing actual ordinary scope/hash join')
    require(all(report[k] is True and summary[k] is True
                for k in ('native_closed', 'child_exit_zero', 'process_group_absent'))
            and report['parent_host_measurement'] is True
            and all(report[k] is False for k in T.FALSE_FIELDS), 'closed timing scope and false authority')
    ordinary = T.u64(status['ordinary_retained_bytes']); reserve = T.u64(status['supervisor_metadata_allowance'])
    total = T.u64(status['retained_bytes_with_timing'])
    require(ordinary == T.u64(summary['files']['total_bytes'])
            and reserve == T.u64(summary['files']['supervisor_metadata_allowance'])
            and total == T.u64(ordinary + len(raw)) and T.u64(total + reserve) <= 32 << 20,
            'timing unchanged aggregate retention cap and exact original byte accounting')
    checked_timeline = T.timeline(report['timeline'])
    return dict(schema='ferric-readiness40-position5-parent-host-timing-data-v1',
        file=pin, ordinary_complete=original, timeline=checked_timeline, disjoint_spans=124,
        ordinary_retained_bytes=ordinary, retained_bytes_with_timing=total,
        supervisor_metadata_allowance=reserve, parent_host_measurement=True,
        gpu_timing=False, nested_control_timers_included=False, sidecar_publication_timed=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False)


def validate(mode, stdout, summary_raw, request, native_root, read, prompt):
    require(type(mode) is str and mode in WRAPPERS, 'explicit default or shared timing mode')
    require(type(stdout) is bytes and 0 < len(stdout) <= 128 << 10
            and type(summary_raw) is bytes and 0 < len(summary_raw) <= 128 << 10,
            'bounded original timing wrapper and ordinary summary')
    wrapper, summary = parse(stdout), parse(summary_raw)
    keys(wrapper, 'schema observation host_timing' + (' currentness_policy' if mode == 'shared' else ''))
    require(wrapper['schema'] == WRAPPERS[mode] and same(wrapper['observation'], summary),
            'explicit timed mode preserves original ordinary observation')
    if mode == 'default':
        checked = V.validate(summary_raw, request, Path(native_root), read, prompt)
        require('shared_full_policy' not in checked, 'default remains empty-stderr admission')
        timing = T.validate(stdout, summary_raw, summary_pin(summary_raw, native_root),
                            checked, native_root, read)
        policy = dict(name='default_full', file=rust_pin(summary['files']['child_stderr']),
                      original_stderr_empty=True, no_policy_bytes_discarded=True)
    else:
        checked = V.validate(summary_raw, request, Path(native_root), read, prompt,
                             policy_validator=S.validate_record)
        record = checked['shared_full_policy']
        require(same(wrapper['currentness_policy'], record), 'combined wrapper joins original worker policy')
        timing = shared_timing(wrapper, summary_raw, checked, native_root, read)
        policy = dict(name='shared_full', file=rust_pin(summary['files']['child_stderr']),
                      policy_record=record, original_stderr_empty=False, no_policy_bytes_discarded=True)
    return dict(schema=SCHEMA, mode=mode, ordinary=checked, policy=policy, timing=timing,
        summary=summary_pin(summary_raw, native_root), stdout=V.part(stdout),
        parent_host_measurement=True, gpu_timing=False, nested_control_timers_included=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False, outer_owned_lineage_checked=False, cpu_qualification_checked=False)


def compare_pair(default_raw, shared_raw, default_checked, shared_checked, admissions, read):
    keys(admissions, 'default shared')
    for mode, raw, checked in (('default', default_raw, default_checked), ('shared', shared_raw, shared_checked)):
        require(checked['schema'] == SCHEMA and checked['mode'] == mode
                and checked['policy']['name'] == ('default_full' if mode == 'default' else 'shared_full')
                and checked['parent_host_measurement'] is True
                and all(checked[k] is False for k in ('gpu_timing', 'nested_control_timers_included',
                    'full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')),
                'separately admitted timed modes with false authority')
        root = parse(raw)['request']['base']['evidence_directory']
        require(same(checked['summary'], summary_pin(raw, root)), 'pair exact admitted original summary')
        keys(admissions[mode], 'worker worker_cpu parent parent_cpu')
        for pin in admissions[mode].values():
            keys(pin, 'path bytes sha256')
            require(type(pin['path']) is str and Path(pin['path']).is_absolute()
                    and type(pin['bytes']) is int and pin['bytes'] > 0
                    and type(pin['sha256']) is str and re.fullmatch('[0-9a-f]{64}', pin['sha256']),
                    'closed external CPU/product pin metadata')
        require(same(rust_pin(parse(raw)['request']['base']['worker']), admissions[mode]['worker']),
                'each original request joins selected worker ELF')
        T.timeline(checked['timing']['timeline'])
    require(same(admissions['default'], admissions['shared']), 'same parent and worker ELF/CPU pins for both cases')
    default, shared = parse(default_raw), parse(shared_raw)
    require(default['request']['base']['evidence_directory'] != shared['request']['base']['evidence_directory']
            and default['child_pid'] != shared['child_pid'], 'distinct fresh case directories and workers')
    parity = S.compare_same_side(shared_raw, default_raw, shared_checked['ordinary'], default_checked['ordinary'], read)
    return dict(schema='ferric-readiness40-matched-timed-parity-v1', passed=True,
        same_elf_cpu_metadata=admissions['default'], parity=parity,
        default_timing=default_checked['timing'], shared_timing=shared_checked['timing'],
        independent_numerical_reference=False, cpu_qualification_checked=False,
        outer_owned_lineage_checked=False, fresh_processes_independently_checked=False,
        gpu_timing=False, full_long_workload=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
