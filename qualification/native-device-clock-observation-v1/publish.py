"""Publish retained clock V2 evidence only; never import or execute GPU controllers."""
import argparse
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
Q = Path('/home/harsh/ferric-p227-integration/qualification/native-device-clock-observation-v1')
PACKAGE = 'p228-device-clock-gpu-v1'
SOURCE_PACKAGES = {PACKAGE, 'p228-device-clock-runtime-v1', 'p228-device-clock-report-v1'}
PACKAGE_SHA = '2f8387de7fca50496c5c1cb8f42df3bc294d0633d54a7086ef82f0abdcbbf94c'
RUNNER_SHA = '2f14376c47882c5185ffb0dc2763d5c6afdf85125f6852b89e1225a3c52b184d'
BASE = 'prefix-state-bank-batch-tf4-shared-full-currentness-gpu-v228-v1'
BASE_SHA = '9917a07aba38ecdf4ca1c9289774c9b9c040bdf5bf850157126c87ca1ab7595c'
CPU = {
    'parent': ('gfx950-clock-parent-cpu-v228-v1',
        'd2a118dd2a3bfac749b18de26883a661a1078a22ebf374853a11b081f43b1484',
        'ferric-qwen3-finite-prefix-decode-device-clock-engineering'),
    'worker': ('gfx950-clock-recorder-cpu-v228-v1',
        '41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d',
        'ferric-tp-peer-finite-engineering-worker-v1'),
}
FALSE = ('clock_domain_validated', 'calibrated_nanoseconds', 'cross_device_clock_alignment',
         'overlap_claim', 'numerical_acceptance', 'full_model_acceptance', 'performance_claim',
         'production_authority')
CHECKED = {}
SUPPLEMENTAL_PURE = (
    ('device-clock-runtime-audit-pure-v228-v1', '20f7e36c882b3aa5235d184179b41ec64791ea70040d44810fa6f61e62c366e2',
     'ferric-p228-device-clock-runtime-audit-pure-v1', 7, 'p228-device-clock-runtime-v1',
     {'audit_device_clock_runtime.py', 'test_audit_device_clock_runtime.py'}),
    ('device-clock-preparation-pure-v228-v1', '3810f66b74232a38c0d8f258e2c4e74369ffd1fd41a456df78efdbaaf68ff250',
     'ferric-p228-device-clock-preparation-pure-v1', 5, 'p228-device-clock-runtime-v1',
     {'audit_device_clock_runtime.py', 'prepare_device_clock_inputs.py', 'test_prepare_device_clock_inputs.py'}),
    ('device-clock-report-pure-v228-v1', '829e3744e4d9ebc610d66476623b851174091da56962ba14973ca02738a22bf7',
     'ferric-p228-device-clock-report-pure-v1', 5, 'p228-device-clock-report-v1',
     {'report.py', 'test_report.py'}),
)


def require(value, message):
    if not value:
        raise RuntimeError(message)


def pairs(rows):
    value = {}
    for key, item in rows:
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
    pin = dict(bytes=len(raw), sha256=digest(raw))
    require(path not in CHECKED or CHECKED[path] == pin, 'retained file drift')
    CHECKED[path] = pin
    return raw


def filepin(path, remote=None):
    raw = body(path)
    return dict(path=str(remote or E / path.relative_to(L)), bytes=len(raw), sha256=digest(raw))


def normalize(record):
    require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}, 'closed FilePin')
    pin = dict(record)
    path = Path(pin['path'])
    require(path.is_relative_to(E) and '..' not in path.parts and str(path) == pin['path']
            and type(pin['bytes']) is int and 0 <= pin['bytes'] <= 8 << 20, 'bounded original evidence path')
    if type(pin['sha256']) is list:
        require(len(pin['sha256']) == 32 and all(type(n) is int and 0 <= n <= 255 for n in pin['sha256']),
                'Rust digest bytes')
        pin['sha256'] = bytes(pin['sha256']).hex()
    require(type(pin['sha256']) is str and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'SHA256')
    return pin


def verify(record):
    pin = normalize(record)
    relative = Path(pin['path']).relative_to(E)
    local = L / relative
    if relative.parts[0] in SOURCE_PACKAGES:
        local = L / 'proposals' / relative
    require(filepin(local, pin['path']) == pin, 'retained original pin equality')
    return local


def document(path, sha=None):
    raw = body(path)
    require(sha is None or digest(raw) == sha, 'actual supplied result SHA')
    return parse(raw)


def referenced(value, root):
    if isinstance(value, dict):
        if set(value) == {'path', 'bytes', 'sha256'} and Path(value['path']).is_relative_to(root):
            verify(value)
        for child in value.values():
            referenced(child, root)
    elif isinstance(value, list):
        for child in value:
            referenced(child, root)


def tree(directory):
    require(directory.resolve(strict=True) == directory and directory.is_dir(), 'canonical complete case')
    files, size, count = [], 0, 0
    for path in directory.rglob('*'):
        mode = path.lstat(); count += 1
        require(path.resolve(strict=True) == path and (stat.S_ISDIR(mode.st_mode) or stat.S_ISREG(mode.st_mode)),
                'no aliased or special case member')
        if path.is_file():
            size += len(body(path)); files.append(path)
        require(count <= 255 and size <= 64 << 20, 'original whole-case count and byte bounds')
    return sorted(files), size


def supplements(copies, plan, complete):
    added, pure_pins, audit_pins = {}, {}, {}
    def retain(path, name):
        require(name not in added or added[name] == path, 'same supplemental publication identity')
        body(path)
        added[name] = path

    final_path = L / 'prefix-device-clock-preparation-final-v228-v1.json'
    final = document(final_path, '180b874fbdd754e63619176a8ad10ae1b53d20caee8182991c933b9c5588977f')
    draft_path = L / 'prefix-device-clock-preparation-draft-v228-v1.json'
    draft = document(draft_path, 'b79b9c5e40eb8d7e79972417f7129f218bdd5cbddc137cf0a164751393231f7e')
    require(final['schema'] == draft['schema'] == 'ferric-p228-device-clock-preparation-v1'
        and final['output_label'] == plan['output_label'] and final['supervisor_tests'] == plan['supervisor_tests']
        and final['package_manifest'] == complete['supervisor_manifest'], 'actual preparation configuration joins')
    review_keys = {'parent_runtime_review', 'worker_runtime_review', 'decode_review'}
    require({key: value for key, value in final.items() if key not in review_keys}
        == {key: value for key, value in draft.items() if key not in review_keys}
        and all(draft[key] is None and final[key] == plan[key] for key in review_keys),
        'draft/final differ only by explicit root review bindings')
    for role in ('parent', 'worker'):
        record = final[role + '_audit']; path = verify(record); audit = document(path)
        root = E / ('device-clock-runtime-' + role + '-v228-v1')
        require(record['path'] == str(root / 'complete.json')
            and audit['schema'] == 'ferric-p227-prefix-runtime-audit-v1' and audit['reviewed'] is False
            and audit['authority'] == 'none' and audit['binary'] == complete['selected_runtime'][role]
            and all(audit[key] is False for key in ('gpu_execution', 'production_authority',
                'numerical_acceptance', 'runtime_premises_discharged')), 'actual role-specific non-native audit')
        files, _ = tree(path.parent)
        expected = {'complete.json', 'inputs.json', 'topology-before.json', 'topology-after.json', 'software.json'}
        expected |= {name + '/' + member for name in ('readelf', 'ldd')
                     for member in ('command.json', 'started.json', 'owner.json', 'audit.json', 'stdout', 'stderr')}
        require({str(item.relative_to(path.parent)) for item in files} == expected, 'closed small runtime audit tree')
        for item in files:
            if item.suffix == '.json': referenced(document(item), root)
            retain(item, 'runtime/' + role + '/' + str(item.relative_to(path.parent)))
        inputs = document(path.parent / 'inputs.json')
        require(set(inputs) == {'binary', 'source_pins', 'tools'}
            and all(inputs[key] == audit[key] for key in ('binary', 'tools')),
            'runtime input identities match completion')
        # The auditor writes inputs before ldd discovers and pins resolved libraries.
        expected_sources = dict(inputs['source_pins'])
        for library in audit['libraries']:
            require(set(library) == {'path', 'resolved'}
                and set(library['resolved']) == {'path', 'bytes', 'sha256'}, 'resolved library identity')
            library_pin = library['resolved']; name = library_pin['path']
            require(name not in expected_sources or expected_sources[name] == library_pin,
                'runtime libraries cannot replace an initial pin')
            expected_sources[name] = library_pin
        require(audit['source_pins'] == expected_sources,
            'runtime completion adds only exact discovered library pins')
        require(len(audit['commands']) == len(audit['owners']) == 2, 'both owned runtime leaves')
        for index, name in enumerate(('readelf', 'ldd')):
            command = document(verify(audit['commands'][index]))
            owner = document(verify(audit['owners'][index])); outcome = owner['outcome']
            actual = document(verify(audit[name])); verify(owner['started'])
            argv = ['/usr/bin/readelf', '-d', audit['binary']['path']] if name == 'readelf' else ['/usr/bin/ldd', audit['binary']['path']]
            require(command['audit_argv'] == actual['argv'] == argv
                and command['argv'] == ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30',
                    '--fsize=1048576', '--core=0', '--', *argv]
                and command['deadline_seconds'] == 30 and command['stream_cap_bytes'] == 1 << 20
                and command['gpu_execution_requested'] is False and owner['command'] == audit['commands'][index]
                and owner['gpu_execution'] is False, 'actual bounded non-native audit command')
            require(type(outcome['exit_code']) is int and outcome['exit_code'] == 0
                and outcome['reason'] is None and outcome['cleanup_signalled'] is False
                and outcome['owned_groups_absent'] is True and outcome['owned_processes_reaped'] is True
                and type(actual['exit_code']) is int and actual['exit_code'] == 0
                and body(verify(actual['stderr'])) == b'', 'natural audit exit and reap')
            verify(actual['stdout'])
        for key in ('topology_before', 'topology_after', 'software_audit'): verify(audit[key])
        review = document(verify(plan[role + '_runtime_review']))
        require(review['reviewed'] is True and all(review[key] == audit[key]
            for key in ('host', 'boot_id', 'binary', 'readelf', 'ldd', 'libraries')),
            'published root review binds this exact audit')
        audit_pins[role] = record

    for label, sha, schema, count, source_package, members in SUPPLEMENTAL_PURE:
        path = L / label / 'complete.json'; value = document(path, sha)
        require(value['schema'] == schema and value['passed'] is True and type(value['tests']) is int
            and value['tests'] == count and value['errors'] == value['failures'] == value['skipped'] == 0
            and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
            and all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance',
                'performance_claim', 'production_authority', 'runtime_audit_executed')), 'actual separate synthetic suite')
        before = document(verify(value['sources_before'])); after = document(verify(value['sources_after']))
        require(before == after and set(before) == members, 'exact tested supplemental source census')
        for name, record in before.items():
            require(record['path'] == str(E / source_package / name), 'actual supplemental source identity')
            target = 'report.py' if name == 'report.py' else ('test_report.py' if name == 'test_report.py' else 'runtime-source/' + name)
            if target == 'report.py':
                require(verify(record) == Path(__file__).resolve().with_name('report.py'), 'tested report body beside publisher')
            else: retain(verify(record), target)
        for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
            require(value[key]['path'] == str(E / label / name), 'actual supplemental pure output path')
            retain(verify(value[key]), 'supplemental-pure/' + label + '/' + name)
        log = body(verify(value['transcript'])).decode('utf-8')
        require(len(re.findall(r'^test_\w+ \([^\n]+\) \.\.\. ok$', log, re.MULTILINE)) == count
            and re.search(r'^Ran ' + str(count) + r' tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE),
            'actual supplemental unittest success transcript')
        controller = verify(value['controller'])
        require(controller.parent == L and controller.name.startswith('run_device_clock_'), 'root-owned pure wrapper')
        retain(controller, 'tools/' + controller.name)
        retain(path, 'supplemental-pure/' + label + '/complete.json')
        pure_pins[label] = filepin(path)
    writer = L / 'write_root_clock_reviews_p228_v1.py'
    require(digest(body(writer)) == '8f0931f6d99d452b7f25857dab2a6829ae4b92bccc0fa4e75c903f079395fd6d',
            'actual root review writer retained as source, never executed here')
    retain(writer, 'tools/' + writer.name)
    retain(final_path, 'preparation/' + final_path.name)
    retain(draft_path, 'preparation/' + draft_path.name)
    copies.extend((path, name) for name, path in sorted(added.items()))
    return dict(runtime_audits=audit_pins, pure_suites=pure_pins,
        root_review_writer=filepin(writer), preparation_configs=[filepin(draft_path), filepin(final_path)],
        external_libraries_rehashed=False, transported_executables_rehashed=False,
        all_transitive_audit_inputs_replayed=False, supplemental_files=len(added))


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only publication')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('case_label'); parser.add_argument('complete_sha256'); parser.add_argument('report_label')
    args = parser.parse_args()
    require(re.fullmatch(r'prefix-device-clock-tf4-shared-full-currentness-gpu-v228-v[1-9][0-9]{0,8}', args.case_label)
        and re.fullmatch('[0-9a-f]{64}', args.complete_sha256)
        and re.fullmatch(r'device-clock-report-v228-v[1-9][0-9]{0,8}', args.report_label), 'closed actual case/report inputs')
    directory = L / args.case_label
    complete = document(directory / 'complete.json', args.complete_sha256)
    require(complete['schema'] == 'ferric-p228-device-clock-gpu-v1' and complete['passed'] is True
        and complete['failures'] == [] and type(complete['native_attempts']) is int
        and complete['native_attempts'] == 1 and complete['retries'] == 0
        and complete['raw_completion_ticks'] is True and complete['raw_clock_counters'] is True
        and all(complete[key] is False for key in FALSE)
        and complete['gpu_time'] is False and complete['sustained_2048_256'] is False,
        'actual successful single-attempt raw-only clock capture')
    files, retained_bytes = tree(directory)
    for path in files:
        if path.suffix == '.json':
            referenced(document(path), E / args.case_label)
    leaves = {'parent', *(side + '-' + str(i) for side in ('before', 'after') for i in range(3))}
    require(set(complete['leaves']) == leaves and len(complete['before_audits']) == len(complete['after_audits']) == 3,
            'six audits and one native leaf')
    owned = {}
    for name in sorted(leaves):
        record = complete['leaves'][name]['result']
        require(record['path'] == str(E / args.case_label / name / 'result.json'), 'direct owned leaf')
        value = document(verify(record))
        require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
            and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
            and value['owned_processes_reaped'] is True, 'natural owned exit and reap')
        for key in ('command', 'started', 'stdout', 'stderr'):
            verify(value[key])
        owned[name] = value
    for side in ('before', 'after'):
        for index, audit in enumerate(complete[side + '_audits']):
            name = side + '-' + str(index)
            require(audit['process_result'] == complete['leaves'][name]['result'], 'audit owns exact process leaf')
            verify(audit['topology'])
    require(complete['baseline']['path'] == str(E / BASE / 'complete.json')
            and complete['baseline']['sha256'] == BASE_SHA, 'fixed real native comparison baseline')
    baseline = document(verify(complete['baseline']))
    require(baseline['passed'] is True and baseline['failures'] == [], 'actual prior native success')
    native = document(verify(complete['retained_native']['complete.json']))
    prior = document(verify(baseline['retained_native']['complete.json']))
    require(all(native[key] is True for key in ('child_exit_zero', 'process_group_absent', 'native_closed'))
        and native['completed_forwards'] == 4 and native['observed_output_tokens'] == prior['observed_output_tokens']
        and native['input_tokens'] == prior['input_tokens'], 'native closure and exact token trajectory')
    sidecar = document(verify(complete['device_sidecar']))
    require(sidecar['schema'] == 'FerricPrefixDecodeDeviceClockObservationV2'
        and sidecar['raw_clock_counters'] is True and len(sidecar['samples']) == 16
        and all(sidecar[key] is False for key in FALSE), 'actual raw clock V2 report')
    device = sidecar['raw']
    require(device['schema'] == 'FerricPrefixDecodeDeviceObservationV1' and device['native_closed'] is True
        and len(device['rows']) == 1172 and device['final_dispatches'] == [592, 580], 'embedded full raw dispatch census')
    checked = complete['checked']
    require(checked == document(verify(complete['observation']))
        and checked['all_payloads_tokens_and_tensors_equal'] is True and checked['clock_samples'] == 16
        and checked['compared_tensor_rows'] == 152 and checked['recorded_close_and_owner_reap_checked'] is True
        and len(checked['tensor_comparison']) == 4 and all(checked[key] is False for key in FALSE),
        'retained checked observation and no extra authority')
    intervals = []
    tensors = [(f'layer{i}-hidden', 4096) for i in range(36)] + [('final-norm', 4096), ('logits', 151936)]
    for position, comparison in enumerate(checked['tensor_comparison']):
        require(comparison['position'] == position and comparison['same_input_history'] is True
            and comparison['output_equal'] is True
            and comparison['candidate_input'] == comparison['reference_input'] == native['input_tokens'][position]
            and comparison['candidate_output'] == comparison['reference_output'] == native['observed_output_tokens'][position]
            and len(comparison['tensors']) == 38, 'all four comparable actual forwards')
        name = f'observation-{position}.bin'
        current = body(verify(complete['retained_native'][name]))
        previous = body(verify(baseline['retained_native'][name]))
        require(len(current) == len(previous) == 606976 and current == previous, 'complete payload equality')
        offset = 0
        for (name, elements), row in zip(tensors, comparison['tensors']):
            part = current[offset:offset + elements * 2]; offset += len(part)
            require(row['name'] == name and row['elements'] == row['exact_words'] == elements
                and row['byte_equal'] is True and row['first_mismatching_element'] is None
                and row['candidate_sha256'] == row['reference_sha256'] == digest(part), 'actual tensor body/hash join')
        require(offset == len(current), 'full 38-tensor payload coverage')
        control = body(verify(complete['retained_native'][f'control-{position}.bin']))
        require(len(control) == 241960, 'full Control extent')
        times = list(struct.unpack_from('<2Q', control)); offset = 16
        for _ in range(36):
            offset += 2 * (284 + 548) * 4
            times.extend(struct.unpack_from('<8Q', control, offset)); offset += 64
        times.extend(struct.unpack_from('<3Q', control, offset))
        require(offset + 24 == len(control) and len(times) == 293, 'all actual Control interval slots')
        intervals.extend(times)
    require(intervals == [row['host_elapsed_ns'] for row in device['rows']], 'all 1172 raw/Control host interval joins')
    plan_path = verify(complete['plan']); plan = document(plan_path)
    request = document(verify(plan['request']))
    require(request['schema'] == 'FerricFinitePrefixDecodeDeviceClockRequestV2'
            and request['decode'] == native['request'], 'new exact request projection')
    ignored = {'worker', 'session', 'evidence_directory'}
    require(set(native['request']) == set(prior['request'])
        and {k: v for k, v in native['request'].items() if k not in ignored}
            == {k: v for k, v in prior['request'].items() if k not in ignored}, 'unchanged complete TF4 workload')
    for role, (label, sha, name) in CPU.items():
        record = complete[role + '_cpu_complete']
        require(record == plan[role + '_cpu'] and record['path'] == str(E / label / 'complete.json')
                and record['sha256'] == sha, 'actual newly qualified CPU identity')
        cpu = document(verify(record)); original = cpu['binaries'][name]['binary']
        selected = complete['selected_runtime'][role]
        require(selected == plan[role] and all(selected[key] == original[key] for key in ('bytes', 'sha256')),
                'selected runtime matches actual CPU executable')
    pure_path = verify(plan['supervisor_tests']); pure = document(pure_path)
    require(pure['schema'] == 'ferric-p228-device-clock-pure-v1' and pure['passed'] is True
        and pure['tests'] == 61 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
        and pure['manifest_sha256'] == PACKAGE_SHA and pure['controller_sha256'] == RUNNER_SHA,
        'actual bounded clock61 result')
    require(complete['supervisor_manifest']['path'] == str(E / PACKAGE / 'manifest.json')
            and complete['supervisor_manifest']['sha256'] == PACKAGE_SHA, 'exact frozen controller package')
    manifest_path = verify(complete['supervisor_manifest']); manifest = document(manifest_path)
    before = document(verify(pure['sources_before'])); after = document(verify(pure['sources_after']))
    require(before == after and pure['sources_before'] == plan['supervisor_test_sources']
        and pure['source_sha256'] == pure['sources_before']['sha256']
        and len(manifest['files']) == len(before) == 15 and manifest['pure_tests'] == 61,
        'exact tested sources before and after')
    copies = [(manifest_path, 'source/manifest.json')]
    for row in manifest['files']:
        record = dict(row, path=str(E / PACKAGE / row['path']))
        require(before[row['path']] == record, 'manifest/source-map body join')
        copies.append((verify(record), 'source/' + row['path']))
    for key in ('sources_before', 'sources_after', 'transcript'):
        path = verify(pure[key]); copies.append((path, 'pure/' + path.name))
    copies.append((pure_path, 'pure/complete.json'))
    pure_runner = L / 'run_device_clock_gpu_pure_p228_v1.py'
    require(digest(body(pure_runner)) == RUNNER_SHA, 'exact pure runner body')
    copies.append((pure_runner, 'tools/' + pure_runner.name))
    report = L / args.report_label
    names = {'raw-ticks.json', 'raw-ticks.md', 'raw-tick-ranges.svg', 'raw-tick-heatmaps.svg'}
    require({p.name for p in report.iterdir()} == names, 'closed actual report output')
    stats = document(report / 'raw-ticks.json')
    report_program = Path(__file__).resolve().with_name('report.py')
    require(stats['schema'] == 'ferric-p228-device-clock-report-v1'
        and stats['complete_sha256'] == args.complete_sha256
        and stats['device_sidecar'] == complete['device_sidecar'] and stats['raw_rows'] == 1172
        and stats['clock_samples'] == 16 and stats['raw_clock_samples'] == sidecar['samples']
        and len(stats['same_device_counter_differences']) == 8
        and stats['report_program_sha256'] == digest(body(report_program))
        and all(stats[key] is False for key in FALSE), 'actual uncalibrated report/source join')
    for row in stats['same_device_counter_differences']:
        pre, post = sidecar['samples'][row['pre_sample']], sidecar['samples'][row['post_sample']]
        require(pre['rank'] == post['rank'] == row['rank'] and pre['position'] == post['position'] == row['position'],
                'same-device sample difference')
        for key in ('gpu_clock_counter', 'cpu_clock_counter', 'system_clock_counter'):
            require(row[key + '_signed_difference'] == post[key] - pre[key]
                and row[key + '_decreased'] is (post[key] < pre[key]), 'raw signed difference, no conversion')
    copies += [(path, 'capture/' + str(path.relative_to(directory))) for path in files if path.suffix != '.bin']
    input_directory = plan_path.parent
    require(re.fullmatch(r'prefix-device-clock-tf4-inputs-v228-v[1-9][0-9]{0,8}', input_directory.name),
            'closed actual input namespace')
    copies += [(path, 'inputs/' + path.name) for path in sorted(input_directory.iterdir())]
    for key in ('decode_review', 'parent_runtime_review', 'worker_runtime_review'):
        verify(plan[key])
    copies += [(report / name, 'plots/' + name) for name in sorted(names)]
    copies += [(report_program, 'report.py'), (Path(__file__).resolve(), 'publish.py')]
    supplemental = supplements(copies, plan, complete)
    payloads, ledger = {}, {}
    for source, name in copies:
        require(name not in payloads and '..' not in Path(name).parts and not Path(name).is_absolute(), 'unique publication member')
        payloads[name] = body(source); ledger[name] = dict(CHECKED[source])
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} == {'README.md'},
                'fresh publication; optional root-authored README only')
        body(Q / 'README.md')
    for path in list(CHECKED): body(path)
    result = dict(schema='ferric-p228-device-clock-public-result-v1', authority='none',
        gpu_observation=filepin(directory / 'complete.json'), device_sidecar=complete['device_sidecar'],
        selected_runtime=complete['selected_runtime'], pure=plan['supervisor_tests'],
        parent_cpu=complete['parent_cpu_complete'], worker_cpu=complete['worker_cpu_complete'],
        supervisor_manifest=complete['supervisor_manifest'], report_program_sha256=stats['report_program_sha256'],
        native_attempts=1, retries=0, device_audits=6, natural_owned_leaf_exits=7,
        clock_samples=16, raw_rows=1172, rank_packets=[592, 580], captured_payloads=4,
        compared_tensor_rows=152, all_payloads_tokens_and_tensors_equal=True,
        output_tokens=native['observed_output_tokens'], retained_tree=dict(files=len(files), bytes=retained_bytes),
        supplemental=supplemental,
        raw_clock_counters=True, raw_completion_ticks=True, binary_captures_in_git=False,
        all_input_pins_replayed=False, selected_executable_bodies_locally_rehashed=False,
        frozen_controller_reexecuted=False, independent_reference_recomputed=False,
        gpu_time=False, sustained_2048_256=False, retained=ledger, **{key: False for key in FALSE})
    Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in payloads.items():
        path = Q / name; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(raw)
        require(body(path) == raw, 'published byte equality')
    for path in list(CHECKED): body(path)
    raw = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    with (Q / 'result.json').open('xb') as stream: stream.write(raw)
    print(json.dumps(dict(published_files=len(ledger), retained_case_files=len(files),
        result_bytes=len(raw), result_sha256=digest(raw))), flush=True)


if __name__ == '__main__': main()
