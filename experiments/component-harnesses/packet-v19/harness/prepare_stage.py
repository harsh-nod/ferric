#!/usr/bin/env python3
"""Create-only single-request diagnostic stage; no launch or frozen-helper mutation."""
import argparse
import hashlib
import importlib.util
import os
from pathlib import Path
import shutil
import socket
import stat

D = Path(__file__).resolve().parent
definition = importlib.util.spec_from_file_location('v14_launch_contract', D / 'launch_contract.py')
c = importlib.util.module_from_spec(definition)
definition.loader.exec_module(c)


def create(stage, name, raw, files, executable=False):
    path = stage / c.relative(name)
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    c.require(path.parent.resolve(strict=True).is_relative_to(stage), 'private canonical destination')
    mode = 0o700 if executable else 0o600
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    with os.fdopen(descriptor, 'wb') as target:
        target.write(raw)
    digest = hashlib.sha256(raw).hexdigest()
    files[name] = {'sha256': digest, 'bytes': len(raw), 'mode': mode}
    return {'path': str(path), 'sha256': digest}


def stage_cpu_receipts(stage, cpu, files):
    for name, phase in cpu['phases'].items():
        for key in c.phase_receipt_fields(name):
            item = phase[key]
            phase[key] = create(stage, 'evidence/' + name + '/' + key,
                c.read(item['path'], item['sha256'], empty=key in ('stdout', 'stderr'))[0], files)
    for profile, sources in cpu['guard_sources'].items():
        for key, item in sources.items():
            sources[key] = create(stage, 'evidence/guards/' + profile + '/' + key,
                c.read(item['path'], item['sha256'])[0], files)
    for key, item in cpu['controller_evidence'].items():
        cpu['controller_evidence'][key] = create(stage, 'evidence/controller/' + key,
            c.read(item['path'], item['sha256'])[0], files)
    for key, item in cpu['runtime_evidence'].items():
        cpu['runtime_evidence'][key] = create(stage, 'evidence/runtime/' + key,
            c.read(item['path'], item['sha256'])[0], files)
    for key, item in cpu['harness_evidence'].items():
        cpu['harness_evidence'][key] = create(stage, 'evidence/harness/' + key,
            c.read(item['path'], item['sha256'])[0], files)
    for key, item in cpu['v19_evidence'].items():
        cpu['v19_evidence'][key] = create(stage, 'evidence/v19/' + key,
            c.read(item['path'], item['sha256'])[0], files)
    c.validate_cpu_stage(stage, files, cpu)


def prepare(args):
    stage = c.stage_name(args.stage)
    c.require(socket.gethostname() == c.HOST and os.getuid() == c.UID, 'authorized staging host/UID')
    guard = c.module(D / 'run_stage.py', c.read(D / 'run_stage.py')[1])
    guard.tmpfs_parent()
    c.require(not os.path.lexists(stage), 'fresh stage required; existing stage is never removed')
    old_raw, _ = c.read(args.base_root / 'counter-plan.json', c.OLD_PLAN_SHA)
    old = c.decode(old_raw)
    build_raw, _ = c.read(args.build_manifest, args.build_manifest_sha256)
    build = c.decode(build_raw)
    c.validate_build(build)
    cpu = c.bound(build['cpu_qualification'])
    c.validate_cpu(cpu, build)
    files, sources = {}, {}
    os.mkdir(stage, 0o700)
    created = stage.lstat()
    try:
        supervisor = c.module(args.base_root / 'v25-native/native_supervisor.py',
            '0c2a7cacbffbe74827ade03fd09f05a927cb9ba051ff475744be8d79f3a96436')
        guard.resources(stage, supervisor, guard.stage_identity(stage))
        create(stage, 'evidence/original-build.json', build_raw, files)
        create(stage, 'evidence/original-cpu.json', c.read(build['cpu_qualification']['path'], build['cpu_qualification']['sha256'])[0], files)
        roster, _ = c.read(args.base_root / 'v25-native/inputs.sha256', c.ROSTER_SHA)
        for line in roster.decode('ascii').splitlines():
            digest, name = line.split('  ', 1)
            c.require(Path(name).name == name, 'flat frozen native helper')
            key = 'v25-native/' + name
            raw, _ = c.read(args.base_root / key, digest)
            create(stage, key, raw, files)
            sources[key] = digest
        create(stage, 'v25-native/inputs.sha256', roster, files)
        for name, digest in c.COUNTER_PINS.items():
            key = 'counter-support/' + name
            create(stage, key, c.read(args.v19_root / key, digest)[0], files)
            sources[key] = digest
        for name in c.OWN_SOURCES:
            raw, digest = c.read(D / name)
            c.require(cpu['harness_sources'][name] == digest, 'untested launcher source: ' + name)
            create(stage, name, raw, files)
            sources[name] = digest
        for name in c.MEASUREMENT_SOURCES:
            raw, digest = c.read(args.measurement_root / name)
            key = 'measurement/' + name
            c.require(cpu['harness_sources'][key] == digest, 'untested measurement source: ' + name)
            create(stage, key, raw, files)
            sources[key] = digest
        for name in c.EXTERNAL_SOURCES:
            raw, digest = c.read(D.parent / name, cpu['external_sources'][name])
            key = 'qualification/' + name
            create(stage, key, raw, files)
            sources[key] = digest
        old_stage = Path(old['stage'])
        inputs = {}
        for key in ('workload', 'reference', 'target_manifest', 'numerical_prerequisite',
                    'native_prerequisite', 'native_supervisor_prerequisite'):
            name = str(Path(old[key]['path']).relative_to(old_stage))
            inputs[key] = create(stage, name, c.read(args.base_root / name, old[key]['sha256'])[0], files)
        images = {}
        for option, image in old['images'].items():
            suffix = str(Path(image['path']).relative_to(old_stage))
            for name, field in (('observation.json', 'manifest_sha256'), ('observation.hsaco', 'hsaco_sha256')):
                create(stage, suffix + '/' + name, c.read(args.base_root / suffix / name, image[field])[0], files)
            images[option] = {**image, 'path': str(stage / suffix)}
        for name, digest in zip(('observation.json', 'observation.hsaco'), c.V19_PINS):
            create(stage, c.V19_SUFFIX + '/' + name, c.read(args.kv_image / name, digest)[0], files)
        images['--ordered64-kv-copy-artifact'] = {'path': str(stage / c.V19_SUFFIX),
            'manifest_sha256': c.V19_PINS[0], 'hsaco_sha256': c.V19_PINS[1]}
        c.require(set(images) == set(c.IMAGE_FIELDS), 'same exact nine-image roster')
        original_worker_sha = build['worker']['sha256']
        build['worker'] = create(stage, 'worker-candidate', c.read(build['worker']['path'], original_worker_sha)[0], files, True)
        for key in c.CONTROLLERS:
            item = build['controllers'][key]
            build['controllers'][key] = create(stage, 'controller-' + key, c.read(item['path'], item['sha256'])[0], files, True)
        for key in ('runtime_source', 'controller_source'):
            item = build[key]
            build[key] = create(stage, 'evidence/' + key + '.archive', c.read(item['path'], item['sha256'])[0], files)
        stage_cpu_receipts(stage, cpu, files)
        cpu_binding = create(stage, 'cpu.json', c.encoded(cpu), files)
        build['cpu_qualification'] = cpu_binding
        build_binding = create(stage, 'build.json', c.encoded(build), files)
        counter = c.module(stage / 'counter-support/run_counter_diagnostic.py', c.COUNTER_PINS['run_counter_diagnostic.py'])
        loaded = counter.load(stage)
        runner = loaded[5]
        request, reference = runner.workload_reference(inputs)
        loaded[2].validate_numerical_prerequisite(runner.bound_json(inputs['numerical_prerequisite']))
        loaded[1].load_v28(stage / 'v25-native').validate_prerequisite(
            runner.bound_json(inputs['native_prerequisite']), runner.bound_json(inputs['native_supervisor_prerequisite']), loaded[2])
        observations = {key: c.decode(c.read(Path(value['path']) / 'observation.json', value['manifest_sha256'])[0])
                        for key, value in images.items()}
        cells = c.campaign_cells(stage, old, images, observations, build, request, reference)
        cell = c.module(stage / 'measurement/native_token_cell.py', sources['measurement/native_token_cell.py'])
        plan = {'schema': 'FerricV19Packet55cPlanV1', 'stage': str(stage), 'cells': cells,
                'images': images, 'inputs': inputs, 'files': files,
                'sources': sources, 'build': build_binding, 'cpu': cpu_binding, 'engineering_only': True,
                'comparison': c.comparison_plan(cells, inputs, sources, cell)}
        c.validate_plan(plan, stage)
        guard.resources(stage, loaded[6], guard.stage_identity(stage))
        # plan.json is the external root hash; it cannot include its own digest.
        raw = c.encoded(plan)
        with (stage / 'plan.json').open('xb') as stream:
            stream.write(raw)
        return {'stage': str(stage), 'plan_sha256': hashlib.sha256(raw).hexdigest(), 'launched': False,
                'images': len(images), 'model_copied': False, 'cell_ids': [row['cell_id'] for row in cells]}
    except BaseException as error:
        failure = stage.with_name(stage.name + '.prepare-failure.json')
        with failure.open('xb') as stream:
            stream.write(c.encoded({'accepted': False, 'error': type(error).__name__ + ': ' + str(error),
                                   'stage': str(stage), 'launched': False}))
        current = stage.lstat()
        c.require((current.st_dev, current.st_ino, current.st_uid, stat.S_IMODE(current.st_mode)) ==
                  (created.st_dev, created.st_ino, c.UID, 0o700) and stage.resolve(strict=True) == stage,
                  'failed stage identity changed; preserve instead of removing')
        shutil.rmtree(stage)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('stage', 'base-root', 'v19-root', 'kv-image', 'measurement-root', 'build-manifest'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--build-manifest-sha256', required=True)
    args = parser.parse_args()
    os.umask(0o077)
    print(c.encoded(prepare(args)).decode(), end='')


if __name__ == '__main__':
    main()
