"""Data-only draft/finalize assembly; no controller imports, launches or auto-review."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

import audit_device_clock_runtime as A

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = A.E
PURE_RUNNER_SHA = '2f14376c47882c5185ffb0dc2763d5c6afdf85125f6852b89e1225a3c52b184d'
BASELINE = ('prefix-state-bank-batch-tf4-shared-full-currentness-gpu-v228-v1',
    '9917a07aba38ecdf4ca1c9289774c9b9c040bdf5bf850157126c87ca1ab7595c')
CPU = {
    'parent': ('gfx950-clock-parent-cpu-v228-v1', 316284,
        'd2a118dd2a3bfac749b18de26883a661a1078a22ebf374853a11b081f43b1484',
        'ferric-qwen3-finite-prefix-decode-device-clock-engineering', 13771584,
        '8d8c8786b159ad959a6fa9ec6635915dfe2fde8214ce52a9254bf463c64aa2ca'),
    'worker': ('gfx950-clock-recorder-cpu-v228-v1', 213924,
        '41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d',
        'ferric-tp-peer-finite-engineering-worker-v1', 5018856,
        'd2af909e7fef88af5b20f4ae8f2a5779a7aee93a187169e97a939bec7d3a0fed'),
}
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
BINDINGS = ('baseline parent_cpu worker_cpu parent worker image_deployment standalone_prepared '
    'standalone_cases numericals request parent_runtime_review worker_runtime_review').split()
FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
    'timestamp_calibration', 'clock_domain_validated', 'cross_device_clock_alignment', 'overlap_claim')
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8',
    TZ='UTC', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
    HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
require = A.require


def parse(raw):
    def unique(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def invalid(value):
        raise RuntimeError('nonfinite JSON number: ' + value)
    return json.loads(raw, object_pairs_hook=unique, parse_constant=invalid)


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')


def filepin(path, raw):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


class Inputs:
    def __init__(self):
        self.records = {}

    def body(self, path, expected=None):
        require(path.is_relative_to(L) and path.resolve(strict=True) == path, 'canonical retained local input')
        before = path.stat()
        require(stat.S_ISREG(before.st_mode) and before.st_size <= 8 << 20, 'bounded regular input')
        raw = path.read_bytes(); after = path.stat()
        stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        require(stamp(before) == stamp(after) and len(raw) == before.st_size, 'stable retained input')
        record = filepin(path, raw)
        require(expected is None or record['sha256'] == expected, 'explicit retained input SHA')
        prior = self.records.setdefault(path, record)
        require(prior == record, 'conflicting retained input reads')
        return raw

    def read(self, record):
        require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}
            and type(record['bytes']) is int and 0 <= record['bytes'] <= 8 << 20
            and type(record['path']) is str and type(record['sha256']) is str
            and re.fullmatch('[0-9a-f]{64}', record['sha256']), 'bounded FilePin')
        remote = Path(record['path'])
        require(str(remote) == record['path'] and remote.is_relative_to(E)
            and '..' not in remote.parts, 'session evidence input only')
        relative = remote.relative_to(E)
        local = L / relative
        if relative.parts[0] == A.GPU_PACKAGE[0]:
            local = L / 'proposals' / relative
        raw = self.body(local, record['sha256'])
        require(len(raw) == record['bytes'], 'retained extent')
        return raw

    def doc(self, record):
        return parse(self.read(record))

    def known(self, relative, digest):
        local = L / relative
        raw = self.body(local, digest)
        return parse(raw), filepin(E / relative, raw)

    def recheck(self):
        for path, pin in list(self.records.items()):
            raw = self.body(path, pin['sha256'])
            require(len(raw) == pin['bytes'], 'retained input postcheck extent')


def config_shape(value, mode):
    require(type(value) is dict and set(value) == set(('schema input_label output_label package_manifest '
        'supervisor_tests parent_audit worker_audit parent_runtime_review worker_runtime_review decode_review').split())
        and value['schema'] == 'ferric-p228-device-clock-preparation-v1', 'closed preparation configuration')
    for key, pattern in (('input_label', r'prefix-device-clock-tf4-inputs-v228-v[1-9][0-9]{0,8}'),
            ('output_label', r'prefix-device-clock-tf4-shared-full-currentness-gpu-v228-v[1-9][0-9]{0,8}')):
        require(type(value[key]) is str and re.fullmatch(pattern, value[key]), 'closed fresh label')
    for key in ('parent_runtime_review', 'worker_runtime_review', 'decode_review'):
        require((value[key] is None) if mode == 'draft' else type(value[key]) is dict,
                'draft has no approvals; finalization requires separately authored root reviews')


def pure(inputs, config):
    pin = config['package_manifest']
    require(pin['path'] == str(E / A.GPU_PACKAGE[0] / 'manifest.json')
        and pin['sha256'] == A.GPU_PACKAGE[1], 'actual frozen clock package')
    manifest = inputs.doc(pin)
    require(manifest['schema'] == 'ferric-p228-device-clock-package-v1'
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == 61
        and len(manifest['files']) == 15, 'frozen fifteen-file clock source package')
    expected = {row['path']: dict(row, path=str(E / A.GPU_PACKAGE[0] / row['path']))
                for row in manifest['files']}
    require(len(expected) == 15 and all(Path(name).name == name for name in expected), 'flat unique frozen members')
    for record in expected.values(): inputs.read(record)
    value = inputs.doc(config['supervisor_tests'])
    require(value['schema'] == 'ferric-p228-device-clock-pure-v1' and value['passed'] is True
        and type(value['tests']) is int and value['tests'] == 61
        and all(type(value[key]) is int and value[key] == 0 for key in ('errors', 'failures', 'skipped'))
        and value['manifest_sha256'] == pin['sha256'] and value['controller_sha256'] == PURE_RUNNER_SHA
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
        and all(value[key] is False for key in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'actual bounded61 pure result')
    directory = Path(config['supervisor_tests']['path']).parent
    require(directory.parent == E and Path(config['supervisor_tests']['path']).name == 'complete.json'
        and re.fullmatch(r'device-clock-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'actual new pure namespace')
    for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                      ('transcript', 'tests.log')):
        require(value[key]['path'] == str(directory / name), 'actual pure output paths')
        inputs.read(value[key])
    require(inputs.doc(value['sources_before']) == inputs.doc(value['sources_after']) == expected
        and value['source_sha256'] == value['sources_before']['sha256'], 'actual unchanged tested package')
    inputs.body(L / 'run_device_clock_gpu_pure_p228_v1.py', PURE_RUNNER_SHA)
    return value


def cpu(inputs, role):
    label, size, digest, name, binary_size, binary_sha = CPU[role]
    value, pin = inputs.known(label + '/complete.json', digest)
    require(pin['bytes'] == size and value['passed'] is True and value['error'] is None
        and value['postcheck_errors'] == [] and value['source_unchanged'] is True, 'actual successful selected CPU')
    selected = value['binaries'][name]
    require(selected['binary']['bytes'] == binary_size and selected['binary']['sha256'] == binary_sha,
            'actual CPU-selected executable identity')
    require(inputs.doc(value['raw']['sources-before.json']) == inputs.doc(value['raw']['sources-after.json']),
            'CPU sources unchanged')
    phase = 'parent-builds' if role == 'parent' else 'worker-build'
    raw = inputs.read(value['raw'][phase + '-stdout'])
    require(hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256']
        and sum(parse(line) == selected['artifact'] for line in raw.splitlines() if line.strip()) == 1,
        'selected executable appears once in authentic Cargo output')
    return pin, selected['binary']


def audit(inputs, record, role, original):
    directory = Path(record['path']).parent
    require(directory.parent == E and Path(record['path']).name == 'complete.json'
        and re.fullmatch('device-clock-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', directory.name),
        'new role-specific actual runtime audit')
    value = inputs.doc(record)
    require(value['schema'] == 'ferric-p227-prefix-runtime-audit-v1' and value['reviewed'] is False
        and value['authority'] == 'none' and value['host'] == 'smci350-rck-g03-b19-03'
        and value['devices'] == [16366993098680759275, 10838076764495710945]
        and all(value[key] is False for key in ('gpu_execution', 'production_authority',
            'numerical_acceptance', 'runtime_premises_discharged')), 'unapproved non-native audit only')
    require(value['binary'] == dict(original, path=str(E / 'device-clock-runtime-v228-v1' / Path(original['path']).name)),
            'audited transported executable is the actual qualified role')
    require(len(value['commands']) == len(value['owners']) == 2, 'both owned library audit leaves')
    for index, name in enumerate(('readelf', 'ldd')):
        command = inputs.doc(value['commands'][index]); owner = inputs.doc(value['owners'][index])
        actual = inputs.doc(value[name]); started = inputs.doc(owner['started']); outcome = owner['outcome']
        argv = ['/usr/bin/readelf', '-d', value['binary']['path']] if name == 'readelf' else ['/usr/bin/ldd', value['binary']['path']]
        require(command['audit_argv'] == actual['argv'] == argv
            and command['argv'] == ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30', '--fsize=1048576', '--core=0', '--', *argv]
            and command['env'] == ENV and command['cwd'] == str(E.parents[1])
            and type(command['deadline_seconds']) is int and command['deadline_seconds'] == 30
            and command['stream_cap_bytes'] == 1 << 20 and command['gpu_execution_requested'] is False,
            'unchanged exact bounded runtime audit command')
        require(owner['command'] == value['commands'][index] and owner['gpu_execution'] is False
            and type(outcome['exit_code']) is int and outcome['exit_code'] == 0
            and outcome['reason'] is None and outcome['cleanup_signalled'] is False
            and outcome['owned_groups_absent'] is True and outcome['owned_processes_reaped'] is True
            and type(actual['exit_code']) is int and actual['exit_code'] == 0,
            'both audits naturally exit and reap, never killed-as-success')
        require(type(started) is dict and bool(started), 'actual owned start record')
        require(inputs.read(actual['stderr']) == b'', 'empty audit stderr')
        text = inputs.read(actual['stdout']).decode('utf-8')
        require('not found' not in text and 'RPATH' not in text and 'RUNPATH' not in text, 'reviewable resolved ELF dependencies')
    for key in ('topology_before', 'topology_after', 'software_audit'): inputs.read(value[key])
    return value


def runtime_template(value):
    review = {key: value[key] for key in ('host', 'boot_id', 'binary', 'readelf', 'ldd', 'libraries')}
    review.update(schema='ferric-p227-prefix-parity-runtime-review-v1', authority='none',
        reviewed=False, production_authority=False, gpu_execution=False, notes='')
    return review


def reviewed(value, template, engineering=False):
    expected = copy.deepcopy(template)
    require(type(value) is dict and set(value) == set(expected), 'closed separately authored root review')
    expected['reviewed'] = True
    expected['notes'] = value['notes']
    texts = [value['notes']]
    if engineering:
        require(type(value['review_topics']) is dict and set(value['review_topics']) == set(TOPICS), 'six root review topics')
        expected['review_topics'] = value['review_topics']; texts.extend(value['review_topics'].values())
    require(encoded(value) == encoded(expected) and value['reviewed'] is True,
            'root review may change only approval and substantive notes')
    for text in texts:
        require(type(text) is str and 32 <= len(text.strip()) and len(text.encode()) <= 16384
            and 'TODO' not in text and 'ROOT REVIEW REQUIRED' not in text, 'substantive actual root review notes')
    return value


def derive(inputs, config, mode):
    config_shape(config, mode)
    baseline, baseline_pin = inputs.known(BASELINE[0] + '/complete.json', BASELINE[1])
    require(baseline['passed'] is True and baseline['failures'] == [], 'actual baseline capture')
    old_plan = inputs.doc(baseline['plan']); old_request = inputs.doc(old_plan['request'])
    old_review = inputs.doc(old_plan['decode_review'])
    actual_pure = pure(inputs, config)
    plan = dict(schema='ferric-p228-device-clock-inputs-v1', output_label=config['output_label'],
        policy='shared-full-currentness', baseline=baseline_pin, supervisor_tests=config['supervisor_tests'],
        supervisor_test_sources=actual_pure['sources_before'])
    for key in ('image_deployment', 'standalone_prepared', 'standalone_cases', 'numericals'):
        require(old_plan[key] == baseline[key], 'unchanged baseline prerequisite')
        plan[key] = old_plan[key]
    templates, audits = {}, {}
    for role in ('parent', 'worker'):
        plan[role + '_cpu'], original = cpu(inputs, role)
        audits[role] = audit(inputs, config[role + '_audit'], role, original)
        plan[role] = audits[role]['binary']; templates[role] = runtime_template(audits[role])
        if mode == 'draft':
            plan[role + '_runtime_review'] = filepin(E / config['input_label'] / (role + '-runtime-review.draft.json'), encoded(templates[role]))
        else:
            reviewed(inputs.doc(config[role + '_runtime_review']), templates[role])
            plan[role + '_runtime_review'] = config[role + '_runtime_review']
    require(audits['parent']['boot_id'] == audits['worker']['boot_id'], 'same currently audited boot')
    decode = copy.deepcopy(old_request['decode'])
    decode.update(worker=dict(plan['worker'], sha256=list(bytes.fromhex(plan['worker']['sha256']))),
        session=list(hashlib.sha256(config['output_label'].encode('ascii')).digest()),
        evidence_directory=str(E / config['output_label'] / 'native'))
    require(decode['mode'] == 'teacher_forced' and set(decode) == set(old_request['decode'])
        and {key for key in decode if decode[key] != old_request['decode'][key]}
            == {'worker', 'session', 'evidence_directory'}, 'only worker/session/evidence decode delta')
    request = encoded(dict(schema='FerricFinitePrefixDecodeDeviceClockRequestV2', decode=decode))
    plan['request'] = filepin(E / config['input_label'] / 'request.json', request)
    review = {key: plan[key] for key in BINDINGS}
    review.update(schema='ferric-p228-device-clock-engineering-review-v1', reviewed=False, authority='none',
        policy=plan['policy'], image=baseline['selected_runtime']['image'],
        historical_runtime=baseline['selected_runtime'], image_provenance=old_review['image_provenance'],
        gpu_attempts=1, notes='', review_topics={key: '' for key in TOPICS}, **{key: False for key in FALSE})
    return plan, request, templates, review


def save(directory, name, raw):
    require(len(raw) <= 2 << 20, 'bounded preparation output')
    with (directory / name).open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return filepin(E / directory.name / name, raw)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python before retained reads')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('draft', 'finalize'))
    parser.add_argument('config'); parser.add_argument('config_sha')
    args = parser.parse_args()
    inputs = Inputs(); config = parse(inputs.body(Path(args.config), args.config_sha))
    plan, request, templates, review = derive(inputs, config, args.mode)
    out = L / config['input_label']
    if args.mode == 'draft':
        inputs.recheck(); out.mkdir(mode=0o700)
        records = {'request': save(out, 'request.json', request)}
        for role in ('parent', 'worker'):
            records[role + '_runtime_review_draft'] = save(out, role + '-runtime-review.draft.json', encoded(templates[role]))
        plan['decode_review'] = save(out, 'decode-review.draft.json', encoded(review))
        records['plan_draft'] = save(out, 'plan.draft.json', encoded(plan))
        print(json.dumps(dict(files=records, reviewed=False, gpu_execution=False), sort_keys=True))
    else:
        require(out.resolve(strict=True) == out and inputs.read(plan['request']) == request, 'retained unchanged draft request')
        reviewed(inputs.doc(config['decode_review']), review, engineering=True)
        plan['decode_review'] = config['decode_review']
        inputs.recheck()
        record = save(out, 'plan.json', encoded(plan))
        print(json.dumps(dict(plan=record, root_review_bytes_unchanged=True, gpu_execution=False), sort_keys=True))


if __name__ == '__main__': main()
