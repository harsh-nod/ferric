"""Retained CPU475/CPU522 TF4 invariance and host-only diagnostics; no launches."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
READER_SHA = '259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930'
OLD_SHA = 'db417b2f7d0728577a711aa212564d0f3df783ec33a133fadaf35356ea9c159b'
OLD_LABEL = 'prefix-independent-decode-tf4-shared-full-currentness-gpu-v228-v1'
PACKAGES = {
    'old': ('p228-independent-decode-observation-v1',
        '10125c91f9c83c55379c5769b2d1bfa099835744b10f292c9ebd5f152ae7e9c3',
        'ferric-p228-independent-decode-observation-v1'),
    'new': ('p228-resident-state-decode-observation-v1',
        '84432400d40b3f8f8225c8953c99d435f295f5bbafe69b521cca95975ac4f2c0',
        'ferric-p228-resident-state-decode-observation-v1'),
}
PARENT = (13618248, '830f90b17bde546fb0d236a127f4bff055b31b4f0b46e2381e22ec072cb9d4d0')
IMAGE = (53560, '4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5')
WORKERS = {
    'old': (4777488, '3a16059a96c2050b654e4998b395b9672a19a593f08bf0c5df893d181a1f5294'),
    'new': (4780024, '79d2b50a080a39300d02648c2844a398907ac4f89a41abff01c720ac58d42430'),
}
INVARIANT_COUNTS = ('commands', 'full_currentness_checks', 'operational_currentness_checks',
    'kernel_admissions', 'dispatches', 'reads', 'read_bytes', 'writes', 'write_bytes')
SHARED_NAMES = ['group_full_checks', 'group_full_ns', 'publication_full_checks', 'publication_full_ns']
OBS_FALSE = ('full_model_correctness', 'independent_numerical_acceptance', 'numerical_acceptance',
    'independent_tensor_acceptance', 'full_model_acceptance', 'native_baseline_comparison_performed',
    'independent_framework_comparison_performed', 'sustained_2048_256', 'gpu_time',
    'performance_claim', 'production_authority')
FALSE = ('gpu_execution_requested', 'gpu_time', 'calibrated_device_time', 'qualified_speedup',
    'performance_claim', 'numerical_acceptance', 'independent_numerical_acceptance',
    'full_model_acceptance', 'full_model_correctness', 'production_authority', 'sustained_2048_256',
    'current_platform_idle_audits_verified', 'runtime_premises_discharged',
    'top_level_observer_reaping_verified', 'current_source_binary_image_authority_verified')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def authenticated_reader(path):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path and not path.is_symlink(),
            'canonical frozen diagnostic reader')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and 0 < len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == READER_SHA, 'unchanged diagnostic reader bytes')
    module = types.ModuleType('_resident_comparison_reader')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def loaded(D, pins, reader, role):
    name, sha, _ = PACKAGES[role]
    directory = D.package(pins, name, sha)
    manifest, manifest_pin = pins.json(directory / 'manifest.json', sha)
    hashes = {row['path']: row['sha256'] for row in manifest['files']}
    aliases = list(reader.ALIASES)
    if role == 'new':
        aliases.insert(aliases.index('intake'), 'resident_state_portable')
    previous = {name: sys.modules.get(name) for name in aliases}
    modules = {}
    try:
        for alias in aliases:
            modules[alias] = D.load_module(pins, directory / (alias + '.py'),
                hashes[alias + '.py'], '_resident_comparison_' + role + '_' + alias)
            sys.modules[alias] = modules[alias]
        I, M = modules['intake'], modules['run']
        C, H = I.comparator(pins)
    finally:
        for alias, prior in previous.items():
            if prior is None:
                sys.modules.pop(alias, None)
            else:
                sys.modules[alias] = prior
    return I, M, C, H, manifest_pin


def replay(reader, D, pins, role, receipt_pin, receipt, modules):
    I, M, C, H, manifest_pin = modules
    require(receipt['schema'] == PACKAGES[role][2] and receipt['passed'] is True
        and receipt['failures'] == [] and type(receipt['native_attempts']) is int
        and receipt['native_attempts'] == 1 and type(receipt['retries']) is int
        and receipt['retries'] == 0 and receipt['gpu_execution_requested'] is True,
        'actual completed one-shot observation')
    require(all(receipt[key] is False for key in OBS_FALSE), 'structural-only observation authority')
    require(receipt['supervisor_manifest'] == manifest_pin
        and receipt['controller'] == pins.pin(E / PACKAGES[role][0] / 'run.py'),
        'exact actual observer generation')
    plan = I.doc(pins, receipt['plan'], 1 << 20)
    I.input_shape(plan)
    directory = E / plan['output_label']
    require(receipt_pin['path'] == str(directory / 'complete.json'), 'actual case namespace')
    for key in ('request', 'deployment', 'image_deployment', 'standalone_prepared', 'standalone_cases', 'numericals'):
        require(receipt[key] == plan[key], 'completion plan join: ' + key)
    if role == 'new':
        require(receipt['prior_deployment'] == plan['prior_deployment'], 'explicit prior deployment join')
    request, policy = I.HC.H.request(I.doc(pins, plan['request'], 64 << 10))
    require(receipt['mode'] == request['mode'] == 'teacher_forced'
        and receipt['policy'] == policy == plan['policy'] == 'shared-full-currentness',
        'same TF4 shared-full-currentness experiment')
    runtime = receipt['selected_runtime']
    require(set(runtime) == {'parent', 'worker', 'image'} and runtime['parent'] == receipt['parent']
        and runtime['worker'] == receipt['worker'] and C.pin(request['prefix_image']) == runtime['image']
        and C.pin(request['worker']) == runtime['worker'], 'actual parent/worker/V7 selected runtime')
    for key, expected in (('parent', PARENT), ('image', IMAGE), ('worker', WORKERS[role])):
        require((runtime[key]['bytes'], runtime[key]['sha256']) == expected, 'qualified ' + role + ' ' + key)
        I.read(pins, runtime[key], 128 << 20, False)
    require(request['evidence_directory'] == str(directory / 'native')
        and receipt['host_sidecar']['path'] == str(directory / 'native-host-policy-v2.json'),
        'case-contained native and sidecar paths')
    parent = reader.replay_leaves(I, pins, receipt, directory)
    candidate = dict(native_files=receipt['retained_native'], host_sidecar=receipt['host_sidecar'],
        request=plan['request'], parent=runtime['parent'], owner=receipt['leaves']['parent']['result'],
        **{key: parent[key] for key in ('command', 'started', 'stdout', 'stderr')})
    read = lambda pin, maximum: I.read(pins, pin, maximum)
    observed, files, structural, ownership, host = I.HC.candidate(C, read, candidate, H)
    checked = I.OBS.observe(C, dict(schema=I.OBS.INPUT_SCHEMA, mode=request['mode'], policy=policy,
        request=plan['request'], prefix_image=runtime['image'], candidate=candidate), read, H, I.HC)
    M.checked_observation(dict(request=request, policy=policy, plan=plan, selected_runtime=runtime), checked)
    require(receipt['observation']['path'] == str(directory / 'observation.json')
        and I.doc(pins, receipt['observation']) == receipt['checked'] == checked
        and receipt['owned_children'] == ownership and checked['structural'] == structural
        and checked['host_observation'] == host, 'actual observation and ownership replay')
    return dict(plan=plan, request=request, observed=observed, files=files, checked=checked,
        runtime=runtime, records=C.records(observed), receipt=receipt, receipt_pin=receipt_pin)


def same_workload(old, new, C, host):
    ignored = {'worker', 'session', 'evidence_directory'}
    require(set(old['request']) == set(new['request']) and ignored <= set(old['request']),
            'same complete request fields')
    require(host.same({k: v for k, v in old['request'].items() if k not in ignored},
                     {k: v for k, v in new['request'].items() if k not in ignored}),
            'only worker/session/output request differences permitted')
    require(old['request']['session'] != new['request']['session'], 'fresh candidate session')
    for name in ('parent', 'image'):
        require(old['runtime'][name] == new['runtime'][name], 'unchanged selected ' + name)
    require(new['receipt']['prior_deployment'] == old['receipt']['deployment'],
            'candidate descends from the actual CPU475 deployment')
    for name in ('parent_cpu_complete', 'parent_cpu_review', 'image_deployment',
                 'standalone_prepared', 'standalone_cases', 'numericals'):
        require(old['receipt'][name] == new['receipt'][name], 'unchanged prerequisite: ' + name)
    for position in range(4):
        left = C.document(old['files'][f'request-{position}.json'])
        right = C.document(new['files'][f'request-{position}.json'])
        # Profile identity includes the fresh session, registration and child PID.
        # Each frozen observer already replays that hash; bind requests within runs.
        for case, row in ((old, left), (new, right)):
            observed = case['observed']
            require(host.same(row['profile_sha256'], observed['profile_sha256'])
                and host.same(row['registration'], observed['bootstrap']['registration'])
                and host.same(row['session'], observed['bootstrap']['scope']['session'])
                and host.same(row['session'], case['request']['session']),
                'forward request bound to its own authenticated profile/session')
        require(host.same(left['command'], right['command']) and left['device_ids'] == right['device_ids']
            and left['id'] == right['id'] and left['protocol'] == right['protocol'],
            'same exact forward input contract')


def tensor_comparison(old, new, diagnostics):
    require(len(old['records']) == len(new['records']) == 4, 'four actual forward records')
    rows = []
    for position, (left_record, right_record) in enumerate(zip(old['records'], new['records'])):
        left = old['files'][f'observation-{position}.bin']
        right = new['files'][f'observation-{position}.bin']
        require(type(left) is bytes and type(right) is bytes and len(left) == len(right) == 606976,
                'all four full606976B observation payloads')
        a = diagnostics.validate_case(left_record, left, position)
        b = diagnostics.validate_case(right_record, right, position)
        require(len(a) == len(b) == 38 and set(a) == set(b), 'all38 actual tensors per forward')
        tensors = [dict(name=name, bytes=len(a[name]), old_sha256=hashlib.sha256(a[name]).hexdigest(),
            new_sha256=hashlib.sha256(b[name]).hexdigest(), byte_equal=a[name] == b[name]) for name in a]
        rows.append(dict(position=position, old_record=left_record, new_record=right_record,
            record_equal=left_record == right_record, payload_byte_equal=left == right,
            old_payload_sha256=hashlib.sha256(left).hexdigest(),
            new_payload_sha256=hashlib.sha256(right).hexdigest(), tensors=tensors))
    return rows


def host_comparison(old, new):
    for host in (old, new):
        require(host['policy'] == 'shared-full-currentness'
            and host['inclusive_nested_host_scopes'] is True and host['gpu_time'] is False
            and host['calibrated_device_time'] is False and host['native_closed'] is True,
            'recorded host-only closed policy counters')
        require(type(host['counter_names']) is list and len(host['counter_names']) == 19
            and len(set(host['counter_names'])) == 19
            and set(INVARIANT_COUNTS) <= set(host['counter_names'])
            and host['shared_counter_names'] == SHARED_NAMES and len(host['intervals']) == 6
            and len(host['forward_host_ns']) == 4, 'actual complete host counter layout')
    require(old['counter_names'] == new['counter_names'], 'identical rank counter order')
    rows = []
    for position in range(4):
        left, right = old['intervals'][position + 1], new['intervals'][position + 1]
        durations = [host['forward_host_ns'][position] for host in (old, new)]
        require(all(type(v) is int and v > 0 for v in durations), 'positive actual host durations')
        for interval in (left, right):
            require(len(interval['shared']) == 4 and len(interval['ranks']) == 2
                and all(len(rank) == 19 for rank in interval['ranks'])
                and all(type(v) is int and v >= 0 for v in interval['shared']
                    + interval['ranks'][0] + interval['ranks'][1]), 'actual unsigned counter deltas')
        ranks = []
        for rank in range(2):
            a, b = (dict(zip(old['counter_names'], interval['ranks'][rank])) for interval in (left, right))
            ranks.append(dict(rank=rank, old=a, new=b, invariant_counts_equal=all(a[k] == b[k]
                for k in INVARIANT_COUNTS), differing_invariant_counts=[k for k in INVARIANT_COUNTS if a[k] != b[k]]))
        a, b = (dict(zip(SHARED_NAMES, interval['shared'])) for interval in (left, right))
        removed = a['group_full_checks'] - b['group_full_checks']
        rows.append(dict(position=position, old_host_ns=durations[0], new_host_ns=durations[1],
            old_over_new_host_duration_ratio=durations[0] / durations[1],
            host_duration_reduction_ns=durations[0] - durations[1], old_shared=a, new_shared=b,
            removed_group_full_checks=removed, expected_removed_group_full_checks=576,
            expected_removal_observed=(removed == 576),
            publication_counts_equal=(a['publication_full_checks'] == b['publication_full_checks']),
            ranks=ranks))
    return rows


def summarize(tensors, counters):
    require(len(tensors) == len(counters) == 4 and sum(len(row['tensors']) for row in tensors) == 152,
            'complete152 tensor and four counter rows')
    exact = sum(item['byte_equal'] for row in tensors for item in row['tensors'])
    payloads = all(row['payload_byte_equal'] and row['record_equal'] for row in tensors)
    counts = all(row['expected_removal_observed'] and row['publication_counts_equal']
        and all(rank['invariant_counts_equal'] for rank in row['ranks']) for row in counters)
    return dict(compared_payloads=4, compared_tensor_rows=152, byte_equal_tensor_rows=exact,
        all_payloads_and_tokens_equal=payloads, all152_tensors_equal=(exact == 152),
        all_counter_invariants_and_expected_removals_observed=counts,
        observed_invariance_passed=payloads and exact == 152 and counts)


def table(rows):
    lines = ['# Host-Only Runtime Diagnostics', '',
        'One retained TF4 run per worker. Inclusive nested host timings, not GPU time or a qualified speedup.', '',
        '| Forward | CPU475 host ms | CPU522 host ms | Group checks old/new | Removed | Publication old/new |',
        '|---:|---:|---:|---:|---:|---:|']
    for row in rows:
        a, b = row['old_shared'], row['new_shared']
        lines.append(f"| {row['position']} | {row['old_host_ns'] / 1e6:.3f} | {row['new_host_ns'] / 1e6:.3f} | "
            f"{a['group_full_checks']}/{b['group_full_checks']} | {row['removed_group_full_checks']} | "
            f"{a['publication_full_checks']}/{b['publication_full_checks']} |")
    lines += ['', 'Expected removal is 576 group checks per forward; it is checked against actual counters.',
        'Nested timing scopes must not be summed. No framework-reference, model-acceptance or throughput claim.', '']
    return '\n'.join(lines)


def source_snapshot(pins):
    for name in ('run.py', 'test_run.py', 'README.md'):
        pins.pin(Path(__file__).resolve().parent / name)
    return {path: pin for path, pin in pins.records.items()
            if Path(path).suffix in ('.py', '.md', '.txt') or Path(path).name == 'manifest.json'}


def execute(args):
    require(not sys.flags.optimize and not os.environ.get('PYTHONOPTIMIZE'), 'ordinary Python required')
    require(type(args.candidate_sha) is str and re.fullmatch('[0-9a-f]{64}', args.candidate_sha)
        and args.candidate_sha != OLD_SHA, 'explicit new actual candidate SHA256')
    output = Path(args.output)
    require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent
        and not os.path.lexists(output), 'fresh canonical comparison output')
    reader = authenticated_reader(args.diagnostic_reader)
    D = reader.bootstrap()
    pins = D.Pins()
    pins.pin(args.diagnostic_reader, READER_SHA)
    modules = {role: loaded(D, pins, reader, role) for role in ('old', 'new')}
    receipts = {}
    for role, path, sha in (('old', args.old_complete, OLD_SHA),
                             ('new', args.candidate_complete, args.candidate_sha)):
        pin, raw = pins.read(Path(path), sha, retain=True, maximum=8 << 20)
        receipts[role] = (pin, D.parse(raw))
    before = source_snapshot(pins)
    output.mkdir(mode=0o700)
    I, _, C, H, _ = modules['new']
    before_pin = I.save(output / 'sources-before.json', before)
    cases = {role: replay(reader, D, pins, role, *receipts[role], modules[role]) for role in ('old', 'new')}
    same_workload(cases['old'], cases['new'], C, I.HC.H)
    tensors = tensor_comparison(cases['old'], cases['new'], H['diagnostics'])
    counters = host_comparison(cases['old']['checked']['host_observation'],
                               cases['new']['checked']['host_observation'])
    summary = summarize(tensors, counters)
    with (output / 'host-comparison.md').open('x', encoding='ascii') as stream:
        stream.write(table(counters))
    table_pin = pins.pin(output / 'host-comparison.md')
    pins.recheck()
    after = {path: pins.pin(path, pin['sha256']) for path, pin in before.items()}
    require(before == after, 'loaded source closure unchanged')
    after_pin = I.save(output / 'sources-after.json', after)
    result = dict(schema='ferric-p228-resident-state-runtime-comparison-v2', authority='none',
        status='RETAINED_HOST_ONLY_RUNTIME_DIAGNOSTICS', passed=summary['observed_invariance_passed'],
        comparison_completed=True, observations={role: receipts[role][0] for role in receipts},
        observer_packages={role: modules[role][4] for role in modules},
        selected_runtime={role: cases[role]['runtime'] for role in cases},
        tensor_comparison=tensors, host_comparison=counters, summary=summary, host_table=table_pin,
        invariant_rank_counters=list(INVARIANT_COUNTS), completion_polls_expected_invariant=False,
        same_workload_verified=True, retained_native_and_close_replayed=True,
        recorded_six_audit_leaf_bytes_rehashed_per_case=True, inclusive_nested_host_scopes=True,
        sources_before=before_pin, sources_after=after_pin, source_postchecks_passed=True,
        input_pins=dict(pins.records), **{key: False for key in FALSE})
    return I.save(output / 'complete.json', result), result['passed']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--diagnostic-reader', default=str(E / 'p228-independent-decode-diagnostic-v1/run.py'))
    parser.add_argument('--old-complete', default=str(E / OLD_LABEL / 'complete.json'))
    parser.add_argument('--candidate-complete', required=True)
    parser.add_argument('--candidate-sha', required=True)
    parser.add_argument('--output', required=True)
    pin, passed = execute(parser.parse_args())
    print(json.dumps(pin, sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
