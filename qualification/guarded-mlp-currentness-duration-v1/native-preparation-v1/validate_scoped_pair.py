"""Pure same-product Default/scoped pair join, not CPU or process authority."""
from pathlib import Path
import re

import validate_readiness as V
import validate_matched as M
import validate_scoped as C
import validate_timing as T
from validate_readiness import keys, parse, require, rust_pin, same


def compare_pair(default_raw, scoped_raw, default_checked, scoped_checked, admissions, read):
    keys(admissions, 'default scoped')
    for mode, raw, checked in (('default', default_raw, default_checked), ('scoped', scoped_raw, scoped_checked)):
        require(checked['schema'] == (M.SCHEMA if mode == 'default' else C.SCHEMA)
                and checked['mode'] == ('default' if mode == 'default' else 'scoped_timed')
                and checked['policy']['name'] == ('default_full' if mode == 'default' else 'scoped_warm')
                and checked['parent_host_measurement'] is True
                and all(checked[k] is False for k in ('gpu_timing', 'nested_control_timers_included',
                    'full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')),
                'separately admitted timed modes with false authority')
        root = parse(raw)['request']['base']['evidence_directory']
        require(same(checked['summary'], M.summary_pin(raw, root)), 'pair exact admitted original summary')
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
    require(same(admissions['default'], admissions['scoped']), 'same parent and worker ELF/CPU pins for both cases')
    default, scoped = parse(default_raw), parse(scoped_raw)
    require(default['request']['base']['evidence_directory'] != scoped['request']['base']['evidence_directory']
            and default['child_pid'] != scoped['child_pid'], 'distinct fresh case directories and workers')
    parity = C.compare_same_side(scoped_raw, default_raw, scoped_checked, default_checked['ordinary'], read)
    return dict(schema='ferric-readiness40-scoped-matched-timed-parity-v1', passed=True,
        same_elf_cpu_metadata=admissions['default'], parity=parity,
        default_timing=default_checked['timing'], scoped_timing=scoped_checked['timing'],
        scoped_policy=scoped_checked['policy'], temporal_equivalent_to_full=False,
        shared_full_currentness=False,
        independent_numerical_reference=False, cpu_qualification_checked=False,
        outer_owned_lineage_checked=False, fresh_processes_independently_checked=False,
        gpu_timing=False, full_long_workload=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
