"""Root-owned preparation and exact-plan review for one independent GPU capture."""
import hashlib
import json
import os
from pathlib import Path
import resource
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-layer0-framework-launch-v1'
OUT = E / 'layer0-framework-inputs-v228-v1'
OWNER = E / 'layer0-framework-launch-v228-v1'
CAPTURE = E / 'layer0-framework-capture-v228-v1'
PREFIX = E.parent / 'qwen3-long-reference-env-v1/venv/lib/python3.10/site-packages'
SOURCES = {
    'modeling_qwen3': ('transformers/models/qwen3/modeling_qwen3.py',
        '704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2'),
    'activation': ('torch/nn/modules/activation.py',
        '89c774e9d0d6fc06a3d8cbf06036f32e0fc7fbd18b80436b6d462c398d4d1a8b'),
    'sdpa': ('transformers/integrations/sdpa_attention.py',
        '5a2d607459531430503dd3cb1adc8778885037c1d0be7e6ca0e82a1a852f4d8b'),
    'torch_functional': ('torch/nn/functional.py',
        '27493186ee22f811b553e31d9c804d4d46716d1be62d034d731537f66f27ef19'),
}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(path, expected=None):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical regular input')
    raw = path.read_bytes()
    require(len(raw) <= 32 << 20, 'bounded input')
    value = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(expected is None or value['sha256'] == expected, 'actual selected source bytes')
    return value


def main():
    require(len(sys.argv) == 4 and sys.argv[1] in ('--prepare', '--approve'),
            '(--prepare|--approve) LAUNCHER_SHA (PURE_SHA|READY_SHA)')
    require(not sys.flags.optimize and sys.dont_write_bytecode
            and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
            'ordinary Python without ambient source overrides')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'fixed bounded ASROCK CPU identity')
    require(all(os.environ.get(k) == '' for k in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'CPU-only preparer')
    for kind, value in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (value, value))
    selected = pin(P / 'launch.py', sys.argv[2])
    module = types.ModuleType('root_layer0_launcher')
    module.__file__ = selected['path']
    exec(compile(Path(selected['path']).read_bytes(), selected['path'], 'exec'), module.__dict__)
    old_pin = pin(E / 'p224-rearm-framework/launcher/launch.py', module.OLD_SHA)
    old = module.load(old_pin, module.OLD_SHA, 'root_old_owned_helper')
    require(old.device()['boot_id'] == '87aff38a-9905-4667-a0a2-8f5f3d24cac4', 'reviewed boot unchanged')
    require(old.pin(old.PYTHON.resolve(strict=True))['sha256'] == 'a2f33a6e006989270f4340528eb61f8f97366e00a5d1b602ac8672ea44fc56ae',
            'actual retained interpreter')
    installed = {role: pin(PREFIX / path, digest) for role, (path, digest) in SOURCES.items()}
    if sys.argv[1] == '--prepare':
        require(not any(os.path.lexists(p) for p in (OUT, OWNER, CAPTURE)), 'fresh experiment namespace')
        pure_pin = pin(E / 'layer0-framework-capture-pure-v228-v1/complete.json', sys.argv[3])
        pure = module.parse(module.read(pure_pin))
        require(pure['passed'] is True and pure['tests'] == 16
                and pure['errors'] == pure['failures'] == pure['skipped'] == 0
                and pure['source_postchecks_passed'] is True, 'actual capture tests passed')
        old_plan_pin = pin(E / 'framework-rearm-v224-v1/plan.json',
                          'f6faae3087d9c1c321ce2bbb84ec907489d5eb4ee6f0e975c34081b8c0c512f3')
        old_plan = module.parse(module.read(old_plan_pin))
        package = {name: pin(E / 'p228-layer0-framework-capture-v1' / name, digest)
                   for name, digest in module.CAPTURE.items()}
        before = module.parse(module.read(pure['sources_before']))
        require(before == module.parse(module.read(pure['sources_after']))
                and all(before[name] == package[name] for name in ('run.py', 'test_run.py')),
                'tested capture source joins')
        topology = old.device()
        process = old.monitor_process()
        executable = pin(Path(old.MONITOR_PATH), old.MONITOR_SHA)
        st = Path(old.MONITOR_PATH).stat()
        monitor = dict(schema='ferric-p223-platform-monitor-v1', host=topology['host'],
            boot_id=topology['boot_id'], process=process, executable=executable,
            filesystem_uid=st.st_uid, filesystem_gid=st.st_gid, read_only_attestation_reviewed=True)
        old.monitor_document(monitor, topology)
        OUT.mkdir(mode=0o700)
        monitor_pin = old.save(OUT / 'monitor.json', monitor)
        reference = E / 'p224-rearm-framework/reference'
        inputs = dict(schema='ferric-p228-layer0-framework-launch-inputs-v1',
            launcher_sha256=selected['sha256'], owned_helper=old_pin, capture_package=package,
            reference_helper=pin(reference / 'framework_reference.py', module.BASE_SHA),
            reference_support=dict(diagnostics=old_plan['diagnostics'], policy=old_plan['policy'],
                                   long_reference=pin(reference / 'helpers/long_reference.py')),
            legacy_plan=old_plan['legacy_plan'], source_authentication=old_plan['source_authentication'],
            token_provenance=old_plan['token_provenance'], topology=topology,
            platform_monitor=monitor_pin, implementation_sources=installed,
            output_label=OWNER.name, capture_label=CAPTURE.name)
        # Root reviewed the installed hook routing, bounded supervisor and one-run scope.
        # All actual idle, live executable and ownership checks remain mandatory at launch.
        review = dict(schema='ferric-p228-layer0-framework-launch-review-v1', reviewed=True,
            inputs_projection_sha256=module.sha(module.encoded(inputs)), resources=module.LIMITS,
            gpu_execution_authorized=True, **{key: False for key in module.FALSE})
        inputs['execution_review'] = old.save(OUT / 'root-review.json', review)
        module.input_shape(inputs)
        module.review_inputs(inputs, review)
        print(json.dumps(old.save(OUT / 'inputs.json', inputs)), flush=True)
    else:
        ready_pin = pin(OWNER / 'ready.json', sys.argv[3])
        ready = module.parse(module.read(ready_pin))
        projection = module.parse(module.read(ready['capture_projection']))
        reference = module.parse(module.read(ready['reference_plan']))
        inputs = module.parse(module.read(pin(OUT / 'inputs.json')))
        module.review_inputs(inputs, module.parse(module.read(inputs['execution_review'])))
        require(ready['schema'] == 'ferric-p228-layer0-framework-launch-ready-v1'
                and ready['gpu_execution'] is False and ready['review_deadline_seconds'] == 300
                and ready['approval_path'] == str(OWNER / 'approval.json')
                and ready['projection_sha256'] == module.sha(module.encoded(projection)), 'actual ready identity')
        pid = ready['supervisor_pid']
        require(type(pid) is int and pid > 1 and Path('/proc', str(pid)).stat().st_uid == 9661,
                'live owned supervisor')
        argv = Path('/proc', str(pid), 'cmdline').read_bytes().split(b'\0')
        require(str(P / 'launch.py').encode() in argv and str(OUT / 'inputs.json').encode() in argv,
                'actual launcher process command')
        require(reference == module.reference_plan(inputs, old, CAPTURE, pid)
                and projection == dict(schema='ferric-p228-layer0-framework-capture-plan-v1',
                    harness_sha256=module.CAPTURE['run.py'], reference_helper=inputs['reference_helper'],
                    reference_plan=ready['reference_plan'], output_root=str(CAPTURE),
                    implementation_sources=installed), 'exact reviewed current-PID capture projection')
        review = dict(schema='ferric-p228-layer0-framework-execution-review-v1', reviewed=True,
            plan_projection_sha256=module.sha(module.encoded(projection)), resources=module.LIMITS,
            gpu_execution_authorized=True, **{key: False for key in module.FALSE})
        reviewed = old.save(OWNER / 'root-capture-review.json', review)
        pending = OWNER / 'approval.pending.json'
        old.save(pending, dict(schema='ferric-p228-layer0-framework-launch-approval-v1',
            projection_sha256=ready['projection_sha256'], execution_review=reviewed))
        require(not os.path.lexists(OWNER / 'approval.json'), 'no existing approval to replace')
        pending.rename(OWNER / 'approval.json')
        print(json.dumps(old.pin(OWNER / 'approval.json')), flush=True)
    require(pin(P / 'launch.py') == selected, 'launcher source unchanged')


if __name__ == '__main__':
    main()
