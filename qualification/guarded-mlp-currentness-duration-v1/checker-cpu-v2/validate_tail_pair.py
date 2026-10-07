"""Pure same-binary CensusV3 versus TailV4 join; no native launch authority."""
import hashlib
from pathlib import Path
import re

import validate_matched as M
import validate_census as N
import validate_tail as U
import validate_timing as T
from validate_readiness import keys, parse, require, rust_pin, same


def compare_pair(census_raw, tail_raw, census_checked, tail_checked, admissions, read):
    keys(admissions, 'census tail')
    for mode, raw, checked, module, selected, policy_name, policy_key in (
            ('census', census_raw, census_checked, N, 'census_timed', 'bank_scoped_census', 'bank_scoped_census_policy'),
            ('tail', tail_raw, tail_checked, U, 'tail_timed', 'bank_scoped_census_tail', 'bank_scoped_census_tail_policy')):
        require(checked['schema'] == module.SCHEMA and checked['mode'] == selected
                and checked['policy']['name'] == policy_name
                and checked['policy']['no_policy_bytes_discarded'] is True
                and checked['policy']['original_stderr_empty'] is False
                and checked['policy']['temporal_equivalent_to_full'] is False
                and checked['policy']['shared_full_currentness'] is False
                and checked['parent_host_measurement'] is True
                and checked['allocation_preflights_changed'] is True
                and checked['census_counters_are_layer_subset'] is True
                and all(checked[k] is False for k in ('gpu_timing', 'nested_control_timers_included',
                    'full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')),
                'separately admitted CensusV3 control and TailV4 candidate')
        summary = parse(raw); root = summary['request']['base']['evidence_directory']
        require(same(checked['summary'], M.summary_pin(raw, root)), 'pair exact admitted original summary')
        pin = rust_pin(summary['files']['child_stderr'])
        require(same(checked['policy']['file'], pin), 'pair exact original policy file')
        original = read(pin)
        require(type(original) is bytes and len(original) == pin['bytes']
                and hashlib.sha256(original).hexdigest() == pin['sha256'], 'pair original policy bytes')
        record = module.validate_record(original, summary)
        require(same(record, checked['policy']['policy_record'])
                and same(record, checked['ordinary'][policy_key])
                and 'shared_full_policy' not in checked['ordinary'],
                'pair independently rechecks both unsanitized policies')
        keys(admissions[mode], 'worker worker_cpu parent parent_cpu')
        for product in admissions[mode].values():
            keys(product, 'path bytes sha256')
            require(type(product['path']) is str and Path(product['path']).is_absolute()
                    and type(product['bytes']) is int and product['bytes'] > 0
                    and type(product['sha256']) is str and re.fullmatch('[0-9a-f]{64}', product['sha256']),
                    'closed external CPU/product pin metadata')
        require(same(rust_pin(summary['request']['base']['worker']), admissions[mode]['worker']),
                'each original request joins selected worker ELF')
        if mode == 'tail':
            require(checked['scoped_tail'] is True and checked['tail_counters_are_independent'] is True,
                    'candidate tail windows have separate counters, not layer or census subsets')
        T.timeline(checked['timing']['timeline'])
    require(same(admissions['census'], admissions['tail']), 'same parent and worker ELF/CPU pins for both cases')
    census, tail = parse(census_raw), parse(tail_raw)
    require(census['request']['base']['evidence_directory'] != tail['request']['base']['evidence_directory']
            and census['child_pid'] != tail['child_pid'], 'distinct fresh case directories and workers')
    parity = U.compare_same_side(tail_raw, census_raw, tail_checked, census_checked['ordinary'], read)
    return dict(schema='ferric-readiness40-bank-scoped-census-tail-matched-timed-parity-v4', passed=True,
        same_elf_cpu_metadata=admissions['census'], parity=parity,
        census_timing=census_checked['timing'], tail_timing=tail_checked['timing'],
        census_policy=census_checked['policy'], tail_policy=tail_checked['policy'],
        allocation_preflights_changed=True, census_counters_are_layer_subset=True,
        scoped_tail=True, tail_counters_are_independent=True, temporal_equivalent_to_full=False,
        shared_full_currentness=False, independent_numerical_reference=False,
        cpu_qualification_checked=False, outer_owned_lineage_checked=False,
        fresh_processes_independently_checked=False, gpu_timing=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False)
