"""Local-only assembly from actual receipts and explicit root review decisions."""
import ast
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = 'p228-projection-residual-decode-gpu-v1'
CPU_LABEL = 'projection-residual-decode-cpu-v228-v1'
CPU_SHA = '1f4365064da1a0884385035d1f2d280e6bba66afc2b5bb4265bf7d60b1418c83'
CAPTURE_LABEL = 'prefix-projection-residual-capture-gpu-v228-v1'
CAPTURE_SHA = '4f25030567c470062fd50862876bc35e93d080f1778c0be4ca36f2b39af4199a'
TF4_SHA = '00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073'
NAMES = dict(parent='ferric-qwen3-finite-projection-residual-decode-engineering',
             worker='ferric-tp-peer-finite-engineering-worker-v1')
SEEN = {}
ORIGINALS = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed JSON fields')


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def parse(raw):
    return json.loads(raw, object_pairs_hook=unique,
        parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def fingerprint(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local input')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 32 << 20,
            'bounded regular unaliased input')
    with path.open('rb') as stream:
        raw = stream.read((32 << 20) + 1); opened = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_nlink)
    require(stamp(before) == stamp(opened) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input changed while reading')
    return raw, dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def local_document(name, digest):
    path = Path(name)
    require(path.is_relative_to(L) and re.fullmatch('[0-9a-f]{64}', digest), 'explicit retained local input digest')
    raw, pin = fingerprint(path)
    require(pin['sha256'] == digest and SEEN.setdefault(str(path), pin) == pin, 'actual supplied input hash')
    return parse(raw), pin


def body(pin):
    keys(pin, 'path bytes sha256')
    path = Path(pin['path'])
    require(path.is_absolute() and '..' not in path.parts and str(path) == pin['path']
        and path.is_relative_to(E) and type(pin['bytes']) is int and 0 <= pin['bytes'] <= 32 << 20
        and type(pin['sha256']) is str and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'original evidence FilePin')
    rel = path.relative_to(E)
    local = L / ('proposals' if rel.parts[0].startswith(('p227-', 'p228-')) else '') / rel
    raw, retained = fingerprint(local)
    require(all(retained[k] == pin[k] for k in ('bytes', 'sha256'))
        and SEEN.setdefault(str(local), retained) == retained
        and ORIGINALS.setdefault(pin['path'], dict(original=pin, retained=retained)) == dict(original=pin, retained=retained),
        'original/retained body identity')
    return raw


def doc(pin):
    return parse(body(pin))


def known(label, digest):
    path = L / label / 'complete.json'
    value, retained = local_document(str(path), digest)
    original = dict(retained, path=str(E / label / 'complete.json'))
    require(doc(original) == value, 'known receipt identity')
    return value, original


def constants(raw):
    tree = ast.parse(raw)
    result = {}
    for node in tree.body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in {'JOINT_CPU', 'BINARIES', 'INPUT_SCHEMA', 'REVIEW_SCHEMA', 'PACKAGE_SCHEMA', 'PURE_SCHEMA',
                        'PLAN_FIELDS', 'REVIEW_BINDINGS', 'REVIEW_FALSE', 'TOPICS', 'PACKAGE_FILES', 'PURE_TESTS',
                        'TEST_RUNNER_SHA', 'LOWERING', 'INSPECTION_SHA', 'IMAGE'}:
                value = node.value
                if isinstance(value, ast.Call) and isinstance(value.func, ast.Attribute) and value.func.attr == 'split':
                    require(not value.args and not value.keywords, 'literal split only')
                    result[name] = ast.literal_eval(value.func.value).split()
                else:
                    result[name] = ast.literal_eval(value)
    require(len(result) == 16, 'complete frozen intake literal contract')
    return result


def content(pin):
    return pin['bytes'], pin['sha256']


def pure(config):
    manifest_pin = config['supervisor_manifest']
    require(manifest_pin['path'] == str(E / PACKAGE / 'manifest.json'), 'new supervisor package namespace')
    manifest = doc(manifest_pin)
    files = {}
    for row in manifest['files']:
        keys(row, 'path bytes sha256')
        require(type(row['path']) is str and '/' not in row['path'] and row['path'] not in files, 'flat unique package member')
        files[row['path']] = body(dict(row, path=str(E / PACKAGE / row['path'])))
    K = constants(files['intake.py'])
    require(set(files) == K['PACKAGE_FILES'] and manifest['schema'] == K['PACKAGE_SCHEMA']
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == K['PURE_TESTS'] == 35,
        'exact source package and authored test census')
    record = config['supervisor_tests']; directory = Path(record['path']).parent
    require(directory.parent == E and Path(record['path']).name == 'complete.json'
        and re.fullmatch(r'projection-residual-decode-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'actual pure namespace')
    value = doc(record)
    require(value['schema'] == K['PURE_SCHEMA'] and value['passed'] is True
        and type(value['tests']) is int and value['tests'] == 35
        and all(type(value[k]) is int and value[k] == 0 for k in ('errors', 'failures', 'skipped'))
        and value['manifest_sha256'] == manifest_pin['sha256']
        and value['controller_sha256'] == K['TEST_RUNNER_SHA']
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
        and all(value[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'actual bounded pure qualification only')
    for key, name in [('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'), ('transcript', 'tests.log')]:
        require(value[key]['path'] == str(directory / name), 'actual pure evidence namespace')
    expected = {row['path']: dict(row, path=str(E / PACKAGE / row['path'])) for row in manifest['files']}
    require(doc(value['sources_before']) == doc(value['sources_after']) == expected
        and value['source_sha256'] == value['sources_before']['sha256'], 'tested sources before and after')
    body(value['transcript'])
    require(value['test_inventory_before'] == value['test_inventory_after']
        and set(value['test_inventory_before']) == set(manifest['test_census'])
        and all(len(names) == len(set(names)) == manifest['test_census'][name]
            for name, names in value['test_inventory_before'].items()), 'actual named pure census')
    return K, value


def decision(value, extra=''):
    keys(value, 'reviewed authority notes' + (' ' + extra if extra else ''))
    require(value['reviewed'] is True and value['authority'] == 'none', 'root-authored affirmative engineering review required')
    require(type(value['notes']) is str and 32 <= len(value['notes'].strip()) and len(value['notes'].encode()) <= 16384,
            'substantive supplied root notes')


def runtime(role, record, binary, cpu_pin, notes):
    decision(notes)
    directory = Path(record['path']).parent
    require(directory.parent == E and Path(record['path']).name == 'complete.json'
        and re.fullmatch('projection-residual-decode-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', directory.name),
        'new role runtime audit namespace')
    value = doc(record)
    require(value['schema'] == 'ferric-p227-prefix-runtime-audit-v1' and value['reviewed'] is False
        and value['authority'] == 'none' and value['gpu_execution'] is False
        and value['host'] == 'smci350-rck-g03-b19-03'
        and value['boot_id'] == '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a'
        and value['devices'] == [16366993098680759275, 10838076764495710945]
        and value['binary'] == binary and len(value['commands']) == len(value['owners']) == 2
        and value['source_pins'][cpu_pin['path']] == cpu_pin
        and value['source_pins'][binary['path']] == binary, 'actual current-host new-binary CPU-only audit')
    for index, name in enumerate(('readelf', 'ldd')):
        require(value['commands'][index]['path'] == str(directory / name / 'command.json')
            and value['owners'][index]['path'] == str(directory / name / 'owner.json'), 'direct audit command/owner')
        audit, owner, command = doc(value[name]), doc(value['owners'][index]), doc(value['commands'][index])
        outcome = owner['outcome']
        require(type(outcome['exit_code']) is int and outcome['exit_code'] == 0 and outcome['reason'] is None
            and outcome['cleanup_signalled'] is False and outcome['owned_groups_absent'] is True
            and outcome['owned_processes_reaped'] is True and owner['command'] == value['commands'][index]
            and owner['gpu_execution'] is False and outcome['deadline_seconds'] == 30, 'naturally exited and reaped audit')
        started = doc(owner['started']); identity = started['parent']
        require(identity['pid'] == identity['pgid'] == identity['sid'] and identity['uid'] == 9661
            and identity['ppid'] == started['supervisor_pid'] and identity['starttime'] > 0
            and any(row.get('identity') == identity and row.get('event') == 'owned' for row in outcome['lineage']),
            'actual owned audit process identity')
        argv = ['/usr/bin/readelf', '-d', binary['path']] if name == 'readelf' else ['/usr/bin/ldd', binary['path']]
        require(audit['exit_code'] == 0 and audit['argv'] == command['audit_argv'] == argv
            and command['argv'] == ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30', '--fsize=1048576', '--core=0', '--', *argv]
            and command['deadline_seconds'] == 30 and command['gpu_execution_requested'] is False
            and command['stream_cap_bytes'] == 1 << 20
            and all(command['env'][k] == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
            'unchanged bounded CPU-only readelf/ldd command')
        require(body(audit['stderr']) == b'', 'quiet audit stderr')
        text = body(audit['stdout']).decode()
        require(all(word not in text for word in ('not found', 'RPATH', 'RUNPATH')), 'no unresolved or embedded search-path libraries')
    for key in ('topology_before', 'topology_after', 'software_audit'):
        body(value[key])
    review = {key: value[key] for key in ('host', 'boot_id', 'binary', 'readelf', 'ldd', 'libraries')}
    review.update(schema='ferric-p227-prefix-parity-runtime-review-v1', reviewed=notes['reviewed'],
        authority=notes['authority'], notes=notes['notes'], production_authority=False, gpu_execution=False)
    return review


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary bytecode-free local assembler')
    require(len(sys.argv) == 5, 'CONFIG_PATH CONFIG_SHA ROOT_NOTES_PATH ROOT_NOTES_SHA')
    config, config_pin = local_document(sys.argv[1], sys.argv[2])
    notes, notes_pin = local_document(sys.argv[3], sys.argv[4])
    keys(config, 'schema input_label output_label session parent_audit worker_audit supervisor_tests supervisor_manifest')
    require(config['schema'] == 'ferric-p228-projection-residual-decode-assembly-inputs-v1'
        and re.fullmatch(r'prefix-projection-residual-decode-inputs-v228-v[1-9][0-9]{0,8}', config['input_label'])
        and re.fullmatch(r'prefix-projection-residual-decode-gpu-v228-v[1-9][0-9]{0,8}', config['output_label'])
        and type(config['session']) is str and re.fullmatch('[0-9a-f]{64}', config['session'])
        and config['session'] != '0' * 64, 'fresh input/output namespaces and explicit nonzero session')
    keys(notes, 'schema configuration parent worker decode')
    require(notes['schema'] == 'ferric-p228-projection-residual-decode-root-notes-v1'
        and notes['configuration'] == config, 'root review is for this exact input configuration')
    decision(notes['decode'], 'gpu_attempts review_topics')
    require(type(notes['decode']['gpu_attempts']) is int and notes['decode']['gpu_attempts'] == 1, 'one expressly reviewed attempt')
    out = L / config['input_label']
    require(not os.path.lexists(out), 'fresh assembly directory')
    K, test = pure(config)
    keys(notes['decode']['review_topics'], ' '.join(K['TOPICS']))
    for text in notes['decode']['review_topics'].values():
        require(type(text) is str and len(text.strip()) >= 32 and len(text.encode()) <= 16384, 'supplied substantive review topic')
    cpu, cpu_pin = known(CPU_LABEL, CPU_SHA)
    require(content(cpu_pin) == K['JOINT_CPU'] and cpu['schema'] == 'ferric-projection-residual-decode-cpu-result-v1'
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['source_unchanged'] is True and len(cpu['phases']) == 87 and len(cpu['binaries']) == 17,
        'actual successful new CPU generation')
    require(doc(cpu['raw']['sources-before.json']) == doc(cpu['raw']['sources-after.json']), 'actual unchanged compiled sources')
    binaries = {role: cpu['binaries'][name]['binary'] for role, name in NAMES.items()}
    for role, pin in binaries.items():
        require(content(pin) == K['BINARIES'][role]
            and pin['path'] == str(E / CPU_LABEL / 'target' / role / 'debug' / NAMES[role]), 'actual original executable identity')
        raw = body(pin)
        require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00', 'retained qualified x86_64 ELF')
    capture, capture_pin = known(CAPTURE_LABEL, CAPTURE_SHA)
    require(capture['passed'] is True and capture['failures'] == [] and capture['native_attempts'] == 1
        and capture['retries'] == 0 and capture['full_forward'] is False
        and cpu['prior_completion'] == capture['parent_cpu_complete'] == capture['worker_cpu_complete'], 'actual prior layer-only generation')
    old_plan = doc(capture['plan']); old_review = doc(old_plan['capture_review'])
    require(old_plan['baseline'] == capture['baseline'] and old_plan['request'] == capture['request'], 'prior plan joins')
    tf4 = doc(capture['baseline'])
    require(capture['baseline']['sha256'] == TF4_SHA and tf4['schema'] == 'ferric-p228-down2-clock-gpu-v1'
        and tf4['passed'] is True and tf4['failures'] == [] and tf4['native_attempts'] == 1 and tf4['retries'] == 0,
        'genuine prior TF4 input source, not candidate equality')
    tf4_plan = doc(tf4['plan']); original = doc(tf4['request'])
    require(tf4_plan['request'] == tf4['request'] and original['schema'] == 'FerricFinitePrefixDecodeDeviceClockRequestV2'
        and original['decode']['schema'] == 'FerricFinitePrefixDecodeRequestV1'
        and original['decode']['mode'] == 'teacher_forced', 'exact old wrapper and plain TF4 inner request')
    lower_pin, inspection_pin, image = (old_plan[k] for k in ('lowering_complete', 'inspection_complete', 'projection_image'))
    require(content(lower_pin) == K['LOWERING'] and inspection_pin['sha256'] == K['INSPECTION_SHA']
        and content(image) == K['IMAGE'], 'unchanged checked projection image identities')
    lower, inspection = doc(lower_pin), doc(inspection_pin)
    require(lower['passed'] is True and inspection['passed'] is True
        and lower['errors'] == [] and lower['postcheck_errors'] == []
        and inspection['error'] is None and inspection['postcheck_errors'] == []
        and lower['artifact']['image'] == inspection['image'] == image
        and inspection['lowering_complete'] == lower_pin and inspection['cpu_complete'] == lower['cpu_complete']
        and doc(lower['artifact']['observation']) == lower['artifact']['value'], 'actual checked/static receipt joins')
    body(image)
    projection_provenance = dict(lowering=lower_pin, inspection=inspection_pin, cpu_complete=lower['cpu_complete'],
        original_image=inspection['image'], observation=lower['artifact']['observation'], descriptor=inspection['inspection']['descriptor'])
    require(old_review['projection_provenance'] == projection_provenance
        and old_review['image'] == tf4['selected_runtime']['image']
        and old_review['down2_image'] == tf4_plan['down2_image'], 'original reviewed image provenance')
    rust_pin = lambda p: dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    decode = copy.deepcopy(original['decode'])
    decode.update(worker=rust_pin(binaries['worker']), session=list(bytes.fromhex(config['session'])),
                  evidence_directory=str(E / config['output_label'] / 'native'))
    require(decode['session'] != original['decode']['session']
        and decode['session'] != doc(capture['request'])['layer']['session'], 'fresh explicit session')
    request = dict(schema='FerricFiniteProjectionResidualDecodeRequestV1', decode=decode,
                   projection_residual_image=rust_pin(image))
    require(len(json.dumps(request, separators=(',', ':')).encode()) <= 16384, 'new bounded request')
    pending = {}
    def pack(name, value):
        raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
        require(name not in pending and len(raw) <= 256 << 10, 'bounded unique assembly output')
        pin = dict(path=str(E / config['input_label'] / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
        pending[name] = raw
        return pin
    plan = dict(schema=K['INPUT_SCHEMA'], output_label=config['output_label'], baseline=capture_pin,
        parent_cpu=cpu_pin, worker_cpu=cpu_pin, **binaries, request=pack('request.json', request),
        projection_image=image, lowering_complete=lower_pin, inspection_complete=inspection_pin,
        parent_runtime_review=pack('parent-runtime-review.json', runtime('parent', config['parent_audit'], binaries['parent'], cpu_pin, notes['parent'])),
        worker_runtime_review=pack('worker-runtime-review.json', runtime('worker', config['worker_audit'], binaries['worker'], cpu_pin, notes['worker'])),
        supervisor_tests=config['supervisor_tests'], supervisor_test_sources=test['sources_before'])
    review = {key: plan[key] for key in K['REVIEW_BINDINGS']}
    review.update(schema=K['REVIEW_SCHEMA'], **notes['decode'], output_label=config['output_label'],
        image=old_review['image'], down2_image=old_review['down2_image'], image_provenance=old_review['image_provenance'],
        down2_provenance=old_review['down2_provenance'], projection_provenance=projection_provenance,
        **{key: False for key in K['REVIEW_FALSE']})
    plan['decode_review'] = pack('decode-review.json', review)
    require(set(plan) == set(K['PLAN_FIELDS'].split()), 'exact frozen intake plan shape')
    plan_pin = pack('plan.json', plan)
    _, own = fingerprint(Path(__file__).resolve(strict=True)); SEEN[own['path']] = own
    for path, pin in SEEN.items():
        require(fingerprint(Path(path))[1] == pin, 'consumed input custody before writes')
    pack('assembly.json', dict(schema='ferric-p228-projection-residual-decode-root-assembly-v1',
        plan=plan_pin, configuration=config_pin, root_notes=notes_pin, assembler=own,
        inputs=list(ORIGINALS.values()), local_only_inputs=[p for p in SEEN.values()
            if p['path'] not in {v['retained']['path'] for v in ORIGINALS.values()}],
        outputs={name: dict(path=str(E / config['input_label'] / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            for name, raw in pending.items()}, explicit_root_decisions_copied=True, automatic_approval=False,
        prior_gpu_outputs_replayed_here=False, unchanged_prior_input_bodies_rehashed=False,
        runtime_library_bodies_rehashed_locally=False, all_transitive_inputs_rehashed=False,
        gpu_execution=False, numerical_acceptance=False, production_authority=False, performance_claim=False))
    require(len(pending) == 6, 'six-file assembly only')
    out.mkdir(mode=0o700)
    for name, raw in pending.items():
        with (out / name).open('xb') as stream:
            stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(plan_pin), flush=True)


if __name__ == '__main__':
    main()
