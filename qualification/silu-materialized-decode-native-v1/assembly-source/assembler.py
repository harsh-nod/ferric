"""Data-only SiLU TF4 assembly; decisions and actual pure evidence come from root."""
import ast
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
HELPER = L / 'proposals/p228-projection-residual-decode-input-assembly-v1/prepare.py'
HELPER_SHA = 'd31308f7d05e485c21520561e0653ef9cf2403aface96243f9198ac8a2e97952'
PACKAGE = 'p228-silu-materialized-decode-gpu-v1'
PACKAGE_SHA = '47afc73e3333ff0705002a16b1a296780d22c119676bba142f07c2eb9fe80c31'
CAPTURE_LABEL = 'prefix-silu-materialized-capture-gpu-v228-v1'
CAPTURE_SHA = '67a235280b48cb5f6a2c51bcca534c182c843857c4c4a98e86718ed368ef12e4'
COMPARISON_LABEL = 'silu-materialized-comparison-v228-v1'
COMPARISON_SHA = '828f8fdb4fc4194b5d7d5c65135222b472d2ec76080667fae8f0b8febba169aa'


def helper():
    if sys.flags.optimize or not sys.dont_write_bytecode or 'PYTHONOPTIMIZE' in os.environ:
        raise RuntimeError('ordinary bytecode-free local assembler required')
    before = HELPER.lstat()
    if HELPER.resolve(strict=True) != HELPER or not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
        raise RuntimeError('canonical retained data helper required')
    with HELPER.open('rb') as stream:
        raw = stream.read((64 << 10) + 1); opened = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_nlink)
    if not (len(raw) == before.st_size <= 64 << 10 and stamp(before) == stamp(opened) == stamp(HELPER.lstat())
            and hashlib.sha256(raw).hexdigest() == HELPER_SHA):
        raise RuntimeError('authenticated original data helper changed')
    H = types.ModuleType('silu_decode_data_helper'); H.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), H.__dict__)
    H.SEEN[str(HELPER)] = dict(path=str(HELPER), bytes=len(raw), sha256=HELPER_SHA)
    return H


def pure(H, config):
    pin = config['supervisor_manifest']
    H.require(type(PACKAGE_SHA) is str and H.re.fullmatch('[0-9a-f]{64}', PACKAGE_SHA)
        and pin['path'] == str(E / PACKAGE / 'manifest.json') and pin['sha256'] == PACKAGE_SHA,
        'root-bound actual frozen successor manifest')
    manifest = H.doc(pin); files = {}
    for row in manifest['files']:
        H.keys(row, 'path bytes sha256')
        H.require(type(row['path']) is str and '/' not in row['path'] and row['path'] not in files,
                  'flat unique frozen package member')
        files[row['path']] = H.body(dict(row, path=str(E / PACKAGE / row['path'])))
    K = H.constants(files['intake.py'])
    wanted = {'SILU_CAPTURE', 'SILU_CPU', 'SILU_LOWERING', 'SILU_OWNER', 'SILU_IMAGE', 'LAYER_COMPARISON'}
    for node in ast.parse(files['intake.py']).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in wanted:
                K[name] = ast.literal_eval(node.value)
    H.require(wanted <= K.keys() and set(files) == K['PACKAGE_FILES'] and len(files) == 14
        and manifest['schema'] == K['PACKAGE_SCHEMA'] and type(manifest['pure_tests']) is int
        and manifest['pure_tests'] == K['PURE_TESTS'] == 43, 'frozen 14-file/43-test contract')
    record = config['supervisor_tests']; directory = Path(record['path']).parent
    H.require(directory.parent == E and Path(record['path']).name == 'complete.json'
        and H.re.fullmatch(r'silu-materialized-decode-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name),
        'explicit actual pure completion namespace')
    value = H.doc(record)
    H.require(value['schema'] == K['PURE_SCHEMA'] and value['passed'] is True
        and type(value['tests']) is int and value['tests'] == 43
        and all(type(value[k]) is int and value[k] == 0 for k in ('errors', 'failures', 'skipped'))
        and value['manifest_sha256'] == pin['sha256'] and value['controller_sha256'] == K['TEST_RUNNER_SHA']
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
        and all(value[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'actual passing pure policy receipt')
    for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'), ('transcript', 'tests.log')):
        H.require(value[key]['path'] == str(directory / name), 'actual pure evidence namespace')
    expected = {row['path']: dict(row, path=str(E / PACKAGE / row['path'])) for row in manifest['files']}
    H.require(H.doc(value['sources_before']) == H.doc(value['sources_after']) == expected
        and value['source_sha256'] == value['sources_before']['sha256'], 'tested exact sources before and after')
    H.body(value['transcript'])
    H.require(value['test_inventory_before'] == value['test_inventory_after']
        and set(value['test_inventory_before']) == set(manifest['test_census'])
        and sum(manifest['test_census'].values()) == 43
        and all(len(names) == len(set(names)) == manifest['test_census'][name]
            for name, names in value['test_inventory_before'].items()), 'actual named test inventory')
    return K, value


def main():
    H = helper()
    H.require(len(sys.argv) == 5, 'CONFIG_PATH CONFIG_SHA ROOT_NOTES_PATH ROOT_NOTES_SHA')
    config, config_pin = H.local_document(sys.argv[1], sys.argv[2])
    notes, notes_pin = H.local_document(sys.argv[3], sys.argv[4])
    H.keys(config, 'schema input_label output_label session parent_audit worker_audit supervisor_tests supervisor_manifest')
    H.require(config['schema'] == 'ferric-p228-silu-materialized-decode-assembly-inputs-v1'
        and H.re.fullmatch(r'prefix-silu-materialized-decode-inputs-v228-v[1-9][0-9]{0,8}', config['input_label'])
        and H.re.fullmatch(r'prefix-silu-materialized-decode-gpu-v228-v[1-9][0-9]{0,8}', config['output_label'])
        and type(config['session']) is str and H.re.fullmatch('[0-9a-f]{64}', config['session'])
        and config['session'] != '0' * 64, 'closed namespaces and explicit nonzero session')
    H.keys(notes, 'schema configuration parent worker decode')
    H.require(notes['schema'] == 'ferric-p228-silu-materialized-decode-root-notes-v1'
        and notes['configuration'] == config, 'supplied root decisions bind exact configuration')
    H.decision(notes['decode'], 'gpu_attempts review_topics')
    H.require(type(notes['decode']['gpu_attempts']) is int and notes['decode']['gpu_attempts'] == 1, 'one reviewed attempt')
    out = L / config['input_label']; H.require(not os.path.lexists(out), 'fresh six-file input directory')
    K, test = pure(H, config)
    H.keys(notes['decode']['review_topics'], ' '.join(K['TOPICS']))
    H.require(all(type(v) is str and len(v.strip()) >= 32 and len(v.encode()) <= 16384
        for v in notes['decode']['review_topics'].values()), 'substantive root-authored topics')
    cpu, cpu_pin = H.known(H.CPU_LABEL, H.CPU_SHA)
    H.require(H.content(cpu_pin) == K['JOINT_CPU'] and cpu['schema'] == 'ferric-projection-residual-decode-cpu-result-v1'
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['source_unchanged'] is True and len(cpu['phases']) == 87 and len(cpu['binaries']) == 17,
        'actual CPU1022 generation')
    H.require(H.doc(cpu['raw']['sources-before.json']) == H.doc(cpu['raw']['sources-after.json']), 'compiled source custody')
    binaries = {role: cpu['binaries'][name]['binary'] for role, name in H.NAMES.items()}
    for role, pin in binaries.items():
        H.require(H.content(pin) == K['BINARIES'][role]
            and pin['path'] == str(E / H.CPU_LABEL / 'target' / role / 'debug' / H.NAMES[role]), 'same selected CPU1022 ELF')
        raw = H.body(pin)
        H.require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00', 'actual retained x86_64 ELF')
    capture, capture_pin = H.known(CAPTURE_LABEL, CAPTURE_SHA)
    H.require(H.content(capture_pin) == K['SILU_CAPTURE'] and capture['schema'] == 'ferric-p228-silu-materialized-capture-gpu-v1'
        and capture['passed'] is True and capture['failures'] == [] and capture['native_attempts'] == 1
        and capture['retries'] == 0 and capture['captured_arrays'] == 28 and capture['pre_swiglu_array_count'] == 22
        and capture['pre_swiglu_arrays_equal'] is True and capture['numerical_acceptance'] is False
        and cpu['prior_completion'] == capture['parent_cpu_complete'] == capture['worker_cpu_complete'], 'actual SiLU layer and CPU988 predecessor')
    old_plan = H.doc(capture['plan']); old_review = H.doc(old_plan['capture_review'])
    H.require(old_plan['baseline'] == capture['baseline'] and old_plan['request'] == capture['request'], 'actual layer plan joins')
    comparison, comparison_pin = H.known(COMPARISON_LABEL, COMPARISON_SHA)
    H.require(H.content(comparison_pin) == K['LAYER_COMPARISON']
        and comparison['schema'] == 'ferric-p228-silu-materialized-comparison-observation-v1'
        and comparison['completed'] is True and comparison['source_postchecks_passed'] is True
        and comparison['native_outer'] == capture_pin and comparison['comparison']['conditional_residuals_exact'] is True
        and comparison['comparison']['comparable_rows'] == 24 and comparison['numerical_acceptance'] is False,
        'actual separate conditional layer comparison')
    tf4 = H.doc(capture['baseline']); tf4_plan = H.doc(tf4['plan']); original = H.doc(tf4['request'])
    H.require(capture['baseline']['sha256'] == H.TF4_SHA and tf4['passed'] is True and tf4['failures'] == []
        and tf4_plan['request'] == tf4['request'] and original['schema'] == 'FerricFinitePrefixDecodeDeviceClockRequestV2'
        and original['decode']['schema'] == 'FerricFinitePrefixDecodeRequestV1'
        and original['decode']['mode'] == 'teacher_forced', 'genuine unchanged TF4 input ancestry')
    lower_pin, inspection_pin, image = (old_plan[k] for k in ('lowering_complete', 'inspection_complete', 'projection_image'))
    H.require(H.content(lower_pin) == K['LOWERING'] and inspection_pin['sha256'] == K['INSPECTION_SHA']
        and H.content(image) == K['IMAGE'], 'unchanged projection image')
    lower, inspection = H.doc(lower_pin), H.doc(inspection_pin)
    H.require(lower['passed'] is True and inspection['passed'] is True and lower['errors'] == []
        and lower['postcheck_errors'] == inspection['postcheck_errors'] == [] and inspection['error'] is None
        and lower['artifact']['image'] == inspection['image'] == image and inspection['lowering_complete'] == lower_pin
        and inspection['cpu_complete'] == lower['cpu_complete']
        and H.doc(lower['artifact']['observation']) == lower['artifact']['value'], 'actual projection compile/inspection joins')
    H.body(image)
    projection = dict(lowering=lower_pin, inspection=inspection_pin, cpu_complete=lower['cpu_complete'],
        original_image=inspection['image'], observation=lower['artifact']['observation'], descriptor=inspection['inspection']['descriptor'])
    mlp_pins = {k: old_plan[k] for k in ('mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner')}
    for key, constant in (('mlp_image', 'SILU_IMAGE'), ('mlp_cpu', 'SILU_CPU'),
                          ('mlp_lowering_complete', 'SILU_LOWERING'), ('mlp_lowering_owner', 'SILU_OWNER')):
        H.require(H.content(mlp_pins[key]) == K[constant] and mlp_pins[key] == capture[key], 'actual selected SiLU pin: ' + key)
    m_cpu, m_lower, m_owner = (H.doc(mlp_pins[k]) for k in ('mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner'))
    H.require(m_cpu['passed'] is True and m_lower['passed'] is True and m_owner['passed'] is True
        and m_cpu['postcheck_errors'] == m_lower['postcheck_errors'] == m_owner['postcheck_errors'] == []
        and m_cpu['tests_passed'] == 38 and m_cpu['tests_ignored'] == 0
        and m_lower['candidate_cpu'] == m_owner['candidate_cpu'] == mlp_pins['mlp_cpu']
        and m_lower['source_manifest'] == m_owner['source_manifest'] == m_cpu['overlay']
        and m_owner['completion'] == mlp_pins['mlp_lowering_complete']
        and m_lower['artifacts']['emitted/artifact.hsaco'] == mlp_pins['mlp_image'], 'actual CPU38/owned lowering/image joins')
    H.body(mlp_pins['mlp_image'])
    mlp = old_review['mlp_provenance']
    H.require(mlp['cpu'] == mlp_pins['mlp_cpu'] and mlp['lowering'] == mlp_pins['mlp_lowering_complete']
        and mlp['owner'] == mlp_pins['mlp_lowering_owner'] and mlp['original_image'] == mlp_pins['mlp_image']
        and mlp['source_manifest'] == m_cpu['overlay'] and mlp['numerical_acceptance'] is False
        and mlp['runtime_requirements_discharged'] is False and old_review['projection_provenance'] == projection
        and old_review['image'] == tf4['selected_runtime']['image']
        and old_review['prior_down2_provenance'] == tf4['down2_provenance'], 'reviewed original image lineage, not new authority')
    rust_pin = lambda p: dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    decode = copy.deepcopy(original['decode'])
    decode.update(worker=rust_pin(binaries['worker']), tiles_image=rust_pin(mlp_pins['mlp_image']),
        session=list(bytes.fromhex(config['session'])), evidence_directory=str(E / config['output_label'] / 'native'))
    H.require(decode['session'] != original['decode']['session']
        and decode['session'] != H.doc(capture['request'])['layer']['session'], 'fresh explicit session')
    request = dict(schema='FerricFiniteProjectionResidualDecodeRequestV1', decode=decode, projection_residual_image=rust_pin(image))
    pending = {}
    def pack(name, value):
        raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
        H.require(name not in pending and len(raw) <= 256 << 10, 'bounded unique assembly output')
        pending[name] = raw
        return dict(path=str(E / config['input_label'] / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    plan = dict(schema=K['INPUT_SCHEMA'], output_label=config['output_label'], baseline=capture_pin,
        parent_cpu=cpu_pin, worker_cpu=cpu_pin, **binaries, request=pack('request.json', request),
        projection_image=image, lowering_complete=lower_pin, inspection_complete=inspection_pin,
        layer_comparison=comparison_pin, **mlp_pins,
        parent_runtime_review=pack('parent-runtime-review.json', H.runtime('parent', config['parent_audit'], binaries['parent'], cpu_pin, notes['parent'])),
        worker_runtime_review=pack('worker-runtime-review.json', H.runtime('worker', config['worker_audit'], binaries['worker'], cpu_pin, notes['worker'])),
        supervisor_tests=config['supervisor_tests'], supervisor_test_sources=test['sources_before'])
    review = {k: plan[k] for k in K['REVIEW_BINDINGS']}
    review.update(schema=K['REVIEW_SCHEMA'], **notes['decode'], output_label=config['output_label'],
        image=old_review['image'], down2_image=tf4_plan['down2_image'], image_provenance=old_review['image_provenance'],
        down2_provenance=tf4['down2_provenance'], projection_provenance=projection, mlp_provenance=mlp,
        **{k: False for k in K['REVIEW_FALSE']})
    plan['decode_review'] = pack('decode-review.json', review)
    H.require(set(plan) == set(K['PLAN_FIELDS'].split()), 'exact frozen intake plan shape')
    plan_pin = pack('plan.json', plan)
    _, own = H.fingerprint(Path(__file__).resolve(strict=True)); H.SEEN[own['path']] = own
    for path, pin in H.SEEN.items():
        H.require(H.fingerprint(Path(path))[1] == pin, 'all consumed local bytes unchanged before writes')
    pack('assembly.json', dict(schema='ferric-p228-silu-materialized-decode-root-assembly-v1',
        plan=plan_pin, configuration=config_pin, root_notes=notes_pin, assembler=own,
        replay_dependency=H.SEEN[str(HELPER)], inputs=list(H.ORIGINALS.values()),
        local_only_inputs=[p for p in H.SEEN.values() if p['path'] not in {v['retained']['path'] for v in H.ORIGINALS.values()}],
        outputs={name: dict(path=str(E / config['input_label'] / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            for name, raw in pending.items()}, explicit_root_decisions_copied=True, automatic_approval=False,
        prior_gpu_outputs_replayed_here=False, comparison_math_reexecuted=False,
        unchanged_prior_input_bodies_rehashed=False, runtime_library_bodies_rehashed_locally=False,
        all_transitive_inputs_rehashed=False, gpu_execution=False, numerical_acceptance=False,
        production_authority=False, performance_claim=False))
    H.require(len(pending) == 6, 'six-file assembly only')
    out.mkdir(mode=0o700)
    for name, raw in pending.items():
        with (out / name).open('xb') as stream:
            stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(plan_pin), flush=True)


if __name__ == '__main__':
    main()
