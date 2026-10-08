"""Pure matched scoped-layer V1 versus bank-plus-layer V2 data join; no launch authority."""
import hashlib
from pathlib import Path
import re

import validate_readiness as V
import validate_matched as M
import validate_scoped as C
import validate_bank_scoped as B
import validate_timing as T
from validate_readiness import keys, parse, require, rust_pin, same


def compare_pair(scoped_raw, bank_raw, scoped_checked, bank_checked, admissions, read):
    keys(admissions, 'scoped bank')
    for mode, raw, checked, module, selected, policy_name, policy_key in (
            ('scoped', scoped_raw, scoped_checked, C, 'scoped_timed', 'scoped_warm', 'scoped_warm_policy'),
            ('bank', bank_raw, bank_checked, B, 'bank_timed', 'bank_scoped_warm', 'bank_scoped_warm_policy')):
        require(checked['schema'] == module.SCHEMA and checked['mode'] == selected
                and checked['policy']['name'] == policy_name
                and checked['policy']['no_policy_bytes_discarded'] is True
                and checked['policy']['original_stderr_empty'] is False
                and checked['policy']['temporal_equivalent_to_full'] is False
                and checked['policy']['shared_full_currentness'] is False
                and checked['parent_host_measurement'] is True
                and all(checked[k] is False for k in ('gpu_timing', 'nested_control_timers_included',
                    'full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')),
                'separately admitted timed layer-only and bank-plus-layer modes')
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
        T.timeline(checked['timing']['timeline'])
    require(same(admissions['scoped'], admissions['bank']), 'same parent and worker ELF/CPU pins for both cases')
    scoped, bank = parse(scoped_raw), parse(bank_raw)
    require(scoped['request']['base']['evidence_directory'] != bank['request']['base']['evidence_directory']
            and scoped['child_pid'] != bank['child_pid'], 'distinct fresh case directories and workers')
    parity = B.compare_same_side(bank_raw, scoped_raw, bank_checked, scoped_checked['ordinary'], read)
    return dict(schema='ferric-readiness40-bank-scoped-matched-timed-parity-v2', passed=True,
        same_elf_cpu_metadata=admissions['scoped'], parity=parity,
        scoped_timing=scoped_checked['timing'], bank_timing=bank_checked['timing'],
        scoped_policy=scoped_checked['policy'], bank_policy=bank_checked['policy'],
        allocation_preflights_changed=False, temporal_equivalent_to_full=False,
        shared_full_currentness=False, independent_numerical_reference=False,
        cpu_qualification_checked=False, outer_owned_lineage_checked=False,
        fresh_processes_independently_checked=False, gpu_timing=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False)
