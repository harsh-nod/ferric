"""Publish an actual retained Down2 TF4 result; data only, no controller imports."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import struct
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/paired-row-mlp-native-v1'
PACKAGE = 'p228-down2-clock-gpu-v1'
PACKAGE_SHA = '09cb794c667a89f033128a588d6cf558e7306f0e1f7200bd341995113bc64b57'
RUNNER = 'run_down2_clock_gpu_pure_p228_v1.py'
RUNNER_SHA = '5995bcb2ee48f95b9f1c11e8eb9942ecae41e49032fc2fc56d9f9c909f33123b'
ASSEMBLER = 'prepare_root_down2_inputs_p228_v1.py'
ASSEMBLER_SHA = 'b545e3be8ea9ac9fddc78499023322bff38655b63d804bb1b83c491da8366774'
BASE = 'prefix-device-clock-tf4-shared-full-currentness-gpu-v228-v1'
BASE_SHA = 'e34189597dc7db7a7c325040f5381932e84390c1a2cf32055830858cb9278ddb'
LOWER_SHA = '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e'
IMAGE = (33112, '65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449')
FALSE = ('clock_domain_validated', 'calibrated_nanoseconds', 'cross_device_clock_alignment',
         'overlap_claim', 'numerical_acceptance', 'full_model_acceptance', 'performance_claim',
         'production_authority')
REVIEW_FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
    'timestamp_calibration', 'clock_domain_validated', 'cross_device_clock_alignment', 'overlap_claim')
CHECKED = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, 'duplicate JSON member')
        value[key] = item
    return value


def parse(raw):
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def body(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical retained file')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 8 << 20,
            'bounded unaliased regular evidence')
    raw = path.read_bytes()
    after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_nlink, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) and len(raw) == before.st_size, 'stable retained bytes')
    value = dict(bytes=len(raw), sha256=digest(raw))
    require(CHECKED.setdefault(path, value) == value, 'retained file drift')
    return raw


def normalize(record):
    require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}, 'closed FilePin')
    value = dict(record)
    path = Path(value['path'])
    require(path.is_relative_to(E) and '..' not in path.parts and str(path) == value['path']
        and type(value['bytes']) is int and 0 <= value['bytes'] <= 32 << 20, 'bounded evidence pin')
    if type(value['sha256']) is list:
        require(len(value['sha256']) == 32 and all(type(n) is int and 0 <= n <= 255 for n in value['sha256']),
                'Rust digest bytes')
        value['sha256'] = bytes(value['sha256']).hex()
    require(type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'SHA256')
    return value


def remote_pin(path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals':
        relative = Path(*relative.parts[1:])
    raw = body(path)
    return dict(path=str(E / relative), bytes=len(raw), sha256=digest(raw))


def verify(record):
    record = normalize(record)
    relative = Path(record['path']).relative_to(E)
    path = L / relative
    if relative.parts[0] in (PACKAGE, 'p228-down2-native-publication-v1'):
        path = L / 'proposals' / relative
    require(remote_pin(path) == record, 'actual retained pin equality')
    return path


def document(path, sha=None):
    raw = body(path)
    require(sha is None or digest(raw) == sha, 'explicit actual completion SHA')
    return parse(raw)


def referenced(value, root):
    if type(value) is dict:
        if set(value) == {'path', 'bytes', 'sha256'} and Path(value['path']).is_relative_to(root):
            verify(value)
        for item in value.values():
            referenced(item, root)
    elif type(value) is list:
        for item in value:
            referenced(item, root)


def tree(directory):
    require(directory.resolve(strict=True) == directory and directory.is_dir(), 'canonical case directory')
    files, size, count = [], 0, 0
    for path in directory.rglob('*'):
        mode = path.lstat(); count += 1
        require(path.resolve(strict=True) == path and (stat.S_ISREG(mode.st_mode) or stat.S_ISDIR(mode.st_mode)),
                'no special or aliased case members')
        if path.is_file():
            size += len(body(path)); files.append(path)
        require(count <= 255 and size <= 64 << 20, 'bounded retained case tree')
    return sorted(files), size


def natural(value):
    require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
        and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
        and value['owned_processes_reaped'] is True, 'natural zero exit and owned reap')


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('case_label'); parser.add_argument('complete_sha256')
    args = parser.parse_args()
    require(re.fullmatch(r'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v[1-9][0-9]{0,8}', args.case_label)
        and re.fullmatch('[0-9a-f]{64}', args.complete_sha256), 'actual case and terminal completion SHA')
    directory = L / args.case_label
    complete = document(directory / 'complete.json', args.complete_sha256)
    require(complete['schema'] == 'ferric-p228-down2-clock-gpu-v1' and complete['passed'] is True
        and complete['failures'] == [] and type(complete['native_attempts']) is int
        and complete['native_attempts'] == 1 and complete['retries'] == 0
        and complete['raw_completion_ticks'] is True and complete['raw_clock_counters'] is True
        and all(complete[key] is False for key in FALSE)
        and complete['gpu_time'] is False and complete['sustained_2048_256'] is False,
        'successful one-attempt raw-only Down2 capture')
    files, retained_bytes = tree(directory)
    for path in files:
        if path.suffix == '.json': referenced(document(path), E / args.case_label)
    leaves = {'parent', *(side + '-' + str(i) for side in ('before', 'after') for i in range(3))}
    require(set(complete['leaves']) == leaves and len(complete['before_audits']) == len(complete['after_audits']) == 3,
            'one native leaf and six audits')
    for name in sorted(leaves):
        leaf = complete['leaves'][name]; record = leaf['result']
        require(record['path'] == str(E / args.case_label / name / 'result.json'), 'case-contained owned leaf')
        value = document(verify(record)); natural(value)
        require(set(leaf['retained_files']) == {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'},
                'all five retained owned records')
        require(leaf['retained_files']['result.json'] == record, 'actual result identity')
        for key in ('command', 'started', 'stdout', 'stderr'):
            filename = key + ('.json' if key in ('command', 'started') else '')
            require(value[key] == leaf['retained_files'][filename]
                and value[key]['path'] == str(E / args.case_label / name / filename), 'owned raw identity')
            verify(value[key])
        command = document(verify(value['command'])); started = document(verify(value['started']))
        require(started['command_sha256'] == value['command']['sha256']
            and value['gpu_execution_requested'] is (name == 'parent')
            and command['gpu_execution_requested'] is (name == 'parent'), 'actual launch identity and role')
    for side in ('before', 'after'):
        for index, audit in enumerate(complete[side + '_audits']):
            name = side + '-' + str(index)
            require(audit['process_result'] == complete['leaves'][name]['result'], 'audit/owned leaf join')
            verify(audit['topology'])
    require(complete['baseline']['path'] == str(E / BASE / 'complete.json')
        and complete['baseline']['sha256'] == BASE_SHA, 'exact actual clock baseline')
    baseline = document(verify(complete['baseline']))
    require(baseline['passed'] is True and baseline['failures'] == []
        and complete['selected_runtime'] == baseline['selected_runtime'], 'same actual clock runtime')
    native = document(verify(complete['retained_native']['complete.json']))
    prior = document(verify(baseline['retained_native']['complete.json']))
    require(all(native[key] is True for key in ('child_exit_zero', 'process_group_absent', 'native_closed'))
        and native['completed_forwards'] == 4 and native['observed_output_tokens'] == prior['observed_output_tokens']
        and native['input_tokens'] == prior['input_tokens'], 'native closure and exact token trajectory')
    sidecar = document(verify(complete['device_sidecar']))
    require(sidecar['schema'] == 'FerricPrefixDecodeDeviceClockObservationV2'
        and sidecar['raw_clock_counters'] is True and len(sidecar['samples']) == 16
        and all(sidecar[key] is False for key in FALSE), 'raw-only sixteen-sample clock report')
    device = sidecar['raw']
    require(device['schema'] == 'FerricPrefixDecodeDeviceObservationV1' and device['native_closed'] is True
        and len(device['rows']) == 1172 and device['final_dispatches'] == [592, 580], 'complete raw dispatch census')
    checked = complete['checked']
    require(checked == document(verify(complete['observation']))
        and checked['schema'] == 'ferric-p228-down2-clock-observation-v1'
        and checked['baseline'] == complete['baseline'] and checked['all_payloads_tokens_and_tensors_equal'] is True
        and checked['clock_samples'] == 16 and checked['compared_tensor_rows'] == 152
        and checked['recorded_close_and_owner_reap_checked'] is True and len(checked['tensor_comparison']) == 4
        and all(checked[key] is False for key in FALSE), 'checked observation and false authority')
    control_summary, payload_pins = [], []
    tensors = [(f'layer{i}-hidden', 4096) for i in range(36)] + [('final-norm', 4096), ('logits', 151936)]
    for position, comparison in enumerate(checked['tensor_comparison']):
        require(comparison['position'] == position and comparison['same_input_history'] is True
            and comparison['output_equal'] is True
            and comparison['candidate_input'] == comparison['reference_input'] == native['input_tokens'][position]
            and comparison['candidate_output'] == comparison['reference_output'] == native['observed_output_tokens'][position]
            and len(comparison['tensors']) == 38, 'all four comparable native forwards')
        name = f'observation-{position}.bin'
        current = body(verify(complete['retained_native'][name]))
        previous = body(verify(baseline['retained_native'][name]))
        require(len(current) == len(previous) == 606976 and current == previous, 'complete payload equality')
        offset = 0
        for (tensor, elements), row in zip(tensors, comparison['tensors']):
            part = current[offset:offset + elements * 2]; offset += len(part)
            require(row['name'] == tensor and row['elements'] == row['exact_words'] == elements
                and row['byte_equal'] is True and row['first_mismatching_element'] is None
                and row['candidate_sha256'] == row['reference_sha256'] == digest(part), 'actual tensor bytes/hash')
        require(offset == len(current), 'all payload bytes covered by tensor slices')
        payload_pins.append(dict(position=position, candidate=complete['retained_native'][name],
                                 baseline=baseline['retained_native'][name], byte_equal=True))
        current_request = document(verify(complete['retained_native'][f'request-{position}.json']))
        old_request = document(verify(baseline['retained_native'][f'request-{position}.json']))
        for key in ('command', 'device_ids', 'id', 'protocol'):
            require(current_request[key] == old_request[key], 'unchanged full forward input')
        control_pin = complete['retained_native'][f'control-{position}.bin']
        control = body(verify(control_pin)); require(len(control) == 241960, 'full Control extent')
        times = list(struct.unpack_from('<2Q', control)); offset = 16
        for _ in range(36):
            offset += 2 * (284 + 548) * 4
            times.extend(struct.unpack_from('<8Q', control, offset)); offset += 64
        times.extend(struct.unpack_from('<3Q', control, offset))
        require(offset + 24 == len(control) and len(times) == 293, 'all native Control interval slots')
        require(times == [row['host_elapsed_ns'] for row in device['rows'][position * 293:(position + 1) * 293]],
                'all raw/Control host interval joins')
        control_summary.append(dict(position=position, control=control_pin, inclusive_host_ns=times))
    plan_path = verify(complete['plan']); plan = document(plan_path)
    request = document(verify(plan['request']))
    require(request['schema'] == 'FerricFinitePrefixDecodeDeviceClockRequestV2'
        and request['decode'] == native['request'] and plan['clock_baseline'] == complete['baseline']
        and plan['baseline'] == complete['legacy_baseline'] == baseline['baseline'], 'request and distinct baselines')
    ignored = {'tiles_image', 'session', 'evidence_directory'}
    require(set(native['request']) == set(prior['request'])
        and {k: v for k, v in native['request'].items() if k not in ignored}
            == {k: v for k, v in prior['request'].items() if k not in ignored}
        and native['request']['session'] != prior['request']['session'], 'only exact MLP image/session/output change')
    image = normalize(plan['down2_image'])
    require(normalize(native['request']['tiles_image']) == image
        and (image['bytes'], image['sha256']) == IMAGE and image == complete['down2_image'], 'actual selected Down2 image')
    verify(image)
    mlp_rows = [row for row in device['rows'] if row['stage'] == 'mlp']
    require(len(mlp_rows) == 288 and all(bytes(row['image_sha256']).hex() == IMAGE[1]
        and row['entry'] == 'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2' for row in mlp_rows), 'all actual MLP dispatch image joins')
    lower_path = verify(plan['down2_lowering']); lower = document(lower_path, LOWER_SHA)
    require(plan['down2_lowering'] == complete['down2_lowering'] and lower['passed'] is True
        and lower['unresolved_runtime_requirements'] == 8 and lower['runtime_requirements_discharged'] is False,
        'existing checked lowering stays engineering-only')
    public_lower = document(F / 'qualification/paired-row-mlp-lowering-v1/result.json')
    require(public_lower['passed'] is True and public_lower['lower_receipt'] == plan['down2_lowering'],
            'link to the actual previously published lowering cohort')
    image_review = document(verify(plan['down2_review']))
    decode_review = document(verify(plan['decode_review']))
    require(image_review['schema'] == 'ferric-p228-down2-image-engineering-review-v1'
        and image_review['provenance'] == complete['down2_provenance']
        and image_review['unresolved_runtime_requirements'] == 8
        and decode_review['historical_runtime'] == baseline['selected_runtime'], 'separate image and unchanged runtime review')
    for review in (image_review, decode_review):
        require(review['reviewed'] is True and review['authority'] == 'none' and review['gpu_attempts'] == 1
            and all(review[key] is False for key in REVIEW_FALSE), 'engineering review grants no acceptance')
        for key in ('down2_lowering', 'down2_image', 'clock_baseline', 'request', 'parent', 'worker'):
            require(review[key] == plan[key], 'exact reviewed input/runtime binding')
    old_plan = document(verify(baseline['plan']))
    for key in ('parent_cpu', 'worker_cpu', 'parent', 'worker', 'parent_runtime_review', 'worker_runtime_review',
                'image_deployment', 'standalone_prepared', 'standalone_cases', 'numericals'):
        require(plan[key] == old_plan[key], 'unchanged qualified prerequisite: ' + key)
    for role in ('parent', 'worker'):
        require(complete[role + '_cpu_complete'] == plan[role + '_cpu']
            and complete[role] == complete['selected_runtime'][role] == plan[role]
            and complete[role + '_runtime_review'] == plan[role + '_runtime_review'],
            'selected CPU/runtime/review identity joins')
    require(complete['request'] == plan['request'] and complete['decode_review'] == plan['decode_review']
        and complete['down2_review'] == plan['down2_review'], 'completed request and review joins')
    pure_path = verify(plan['supervisor_tests']); pure = document(pure_path)
    require(pure['schema'] == 'ferric-p228-down2-clock-pure-v1' and pure['passed'] is True
        and type(pure['tests']) is int and pure['tests'] == 75 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
        and pure['manifest_sha256'] == PACKAGE_SHA and pure['controller_sha256'] == RUNNER_SHA,
        'actual passing bounded 75-test suite')
    manifest_path = verify(complete['supervisor_manifest']); manifest = document(manifest_path, PACKAGE_SHA)
    require(complete['supervisor_manifest']['path'] == str(E / PACKAGE / 'manifest.json')
        and manifest['schema'] == 'ferric-p228-down2-clock-package-v1', 'exact executed package')
    before = document(verify(pure['sources_before'])); after = document(verify(pure['sources_after']))
    require(before == after and pure['sources_before'] == plan['supervisor_test_sources']
        and pure['source_sha256'] == pure['sources_before']['sha256']
        and len(manifest['files']) == len(before) == 17 and manifest['pure_tests'] == 75, 'tested source closure')
    copies = [(manifest_path, 'source/manifest.json')]; expected_tests = set()
    for member in manifest['files']:
        record = dict(member, path=str(E / PACKAGE / member['path']))
        require(before[member['path']] == record, 'manifest/tested source join')
        source = verify(record); copies.append((source, 'source/' + member['path']))
        if member['path'].startswith('test_'):
            syntax = ast.parse(body(source), filename=member['path'])
            names = {(method.name, member['path'][:-3] + '.' + cls.name) for cls in syntax.body
                if isinstance(cls, ast.ClassDef) for method in cls.body
                if isinstance(method, (ast.FunctionDef, ast.AsyncFunctionDef)) and method.name.startswith('test_')}
            require(len(names) == manifest['test_census'][member['path']], 'authored named test census')
            expected_tests.update(names)
    log = body(verify(pure['transcript'])).decode('utf-8')
    actual_rows = re.findall(r'^(test_\w+) \(([^\n]+)\) \.\.\. ok$', log, re.MULTILINE)
    actual_tests = {(name, label.rsplit('.' + name, 1)[0]) for name, label in actual_rows}
    require(len(actual_rows) == len(expected_tests) == 75 and actual_tests == expected_tests
        and re.search(r'^Ran 75 tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE), 'actual named passing test inventory')
    for key in ('sources_before', 'sources_after', 'transcript'):
        path = verify(pure[key]); copies.append((path, 'pure/' + path.name))
    copies.append((pure_path, 'pure/complete.json'))
    for name, sha in ((RUNNER, RUNNER_SHA), (ASSEMBLER, ASSEMBLER_SHA)):
        path = L / name; require(digest(body(path)) == sha, 'actual bounded runner/root review assembler')
        copies.append((path, 'tools/' + name))
    copies.append((Path(__file__).resolve(), 'tools/publish.py'))
    copies.extend((path, 'capture/' + str(path.relative_to(directory))) for path in files if path.suffix != '.bin')
    for key in ('request', 'decode_review', 'down2_review', 'parent_runtime_review', 'worker_runtime_review'):
        path = verify(plan[key]); copies.append((path, 'inputs/' + key.replace('_', '-') + '.json'))
    copies.append((plan_path, 'inputs/plan.json'))
    payloads, ledger = {}, {}
    for source, name in copies:
        require(name not in payloads and not Path(name).is_absolute() and '..' not in Path(name).parts, 'unique public member')
        payloads[name] = body(source)
        ledger[name] = dict(source=remote_pin(source), **CHECKED[source])
    summary = dict(schema='ferric-p228-down2-controls-summary-v1', raw_control_bodies_in_git=False,
        inclusive_host_intervals=True, gpu_time=False, forwards=control_summary)
    payloads['control-summary.json'] = json_bytes(summary)
    ledger['control-summary.json'] = dict(bytes=len(payloads['control-summary.json']),
        sha256=digest(payloads['control-summary.json']), derived_from=[row['control'] for row in control_summary])
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} == {'README.md'},
                'fresh publication with optional root README only')
        body(Q / 'README.md')
    for path in list(CHECKED): body(path)
    result = dict(schema='ferric-p228-paired-row-mlp-native-publication-v1', authority='none',
        gpu_observation=remote_pin(directory / 'complete.json'), baseline=complete['baseline'],
        legacy_baseline=complete['legacy_baseline'], down2_image=image, down2_lowering=plan['down2_lowering'],
        lowering_publication='../paired-row-mlp-lowering-v1/result.json',
        selected_runtime=complete['selected_runtime'], pure=plan['supervisor_tests'], pure_tests=75,
        supervisor_manifest=complete['supervisor_manifest'], native_attempts=1, retries=0,
        device_audits=6, natural_owned_leaf_exits=7, clock_samples=16, raw_rows=1172,
        mlp_dispatches=288, rank_packets=[592, 580], captured_payloads=4, compared_tensor_rows=152,
        all_payloads_tokens_and_tensors_equal=True, payload_comparisons=payload_pins,
        output_tokens=native['observed_output_tokens'], retained_case=dict(files=len(files), bytes=retained_bytes),
        raw_clock_counters=True, raw_completion_ticks=True, retained=ledger,
        binary_captures_in_git=False, all_transitive_inputs_replayed=False,
        selected_executable_bodies_locally_rehashed=False, frozen_controller_reexecuted=False,
        independent_reference_recomputed=False, runtime_requirements_discharged=False,
        top_level_observer_reaping_independently_verified=False, gpu_time=False, sustained_2048_256=False,
        **{key: False for key in FALSE})
    Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in payloads.items():
        path = Q / name; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(raw)
        require(body(path) == raw, 'published byte equality')
    for path in list(CHECKED): body(path)
    raw = json_bytes(result)
    with (Q / 'result.json').open('xb') as stream: stream.write(raw)
    print(json.dumps(dict(published_files=len(ledger), retained_case_files=len(files),
                         result_bytes=len(raw), result_sha256=digest(raw))), flush=True)


if __name__ == '__main__':
    main()
