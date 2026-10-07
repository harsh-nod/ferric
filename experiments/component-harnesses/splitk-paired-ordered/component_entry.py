"""Owned wrapper for the frozen component API, with paired external admission."""
import importlib.util
import json
import os
from pathlib import Path
import signal
import stat

D = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('component_contract', D / 'component_contract.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)


class SampleEndpoints:
    def __init__(self, check_alive, admit, endpoint, expected_calls=32):
        c.require(expected_calls == 32, 'fixed 16-cell before/after callback roster')
        self.check_alive, self.admit, self.endpoint = check_alive, admit, endpoint
        self.calls, self.admissions, self.endpoints = 0, [], []

    def __call__(self):
        self.check_alive()
        c.require(self.calls < 32, 'unexpected extra sample callback')
        if self.calls in (0, 31):
            self.admissions.append(self.admit())
            c.require(self.admissions[-1].get('accepted') is True, 'positive component admission required')
            self.endpoints.append(self.endpoint(bool(self.calls)))
        self.calls += 1

    def finish(self):
        c.require(self.calls == 32 and len(self.admissions) == len(self.endpoints) == 2,
                  'incomplete fixed schedule or missing paired endpoints')


def owned_placement(evidence, wrapper, worker):
    placements = {}
    for role, identity in (('controller', wrapper), ('worker', worker)):
        observed = evidence.process(identity['pid'])['process']
        c.require(observed['process_id'] == identity['pid']
                  and observed['parent_pid'] == identity['parent']
                  and observed['start_time_ticks'] == identity['start']
                  and observed['process_group'] == observed['session'] == wrapper['pid'],
                  'live component process identity and owned session')
        placements[role] = {key: observed[key] for key in evidence.STABLE_PROCESS}
    c.require(placements['controller']['nice'] == placements['worker']['nice'] == 0
              and placements['controller']['cpus_allowed_list'] == placements['worker']['cpus_allowed_list'],
              'default inherited worker placement required')
    return placements


def execute(stage, plan, loaded, admission, component, output, lifecycle, evidence):
    """This exercised entry retains every custody gate of frozen component.main."""
    os.umask(0o077)
    c.require(plan['device_unique_id'] == c.DEVICE, 'explicit component device')
    c.require(os.getpid() == os.getpgrp() == os.getsid(0)
              and signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, 'owned wrapper session required')
    result_dir = output / 'component-results'
    c.require(result_dir.is_absolute() and result_dir.parent.resolve(strict=True) == result_dir.parent,
              'canonical component output parent')
    result_dir.mkdir(mode=0o700)
    core = component.load_frozen(stage / 'frozen/probe.py', component.HELPER_SHA256, 'component_probe')
    c.require(component.LIFECYCLE_SHA256 == c.PINS['measurement/native_lifecycle.py'], 'exact lifecycle lineage')
    source_hashes = {name: c.read(stage / name, expected)[1] for name, expected in plan['sources'].items()}
    wrapper, parent = lifecycle.process(os.getpid()), lifecycle.process(os.getppid())
    component.entry_identity(wrapper, parent, os.environ, os.getppid(), os.getuid())
    parent_fd = os.pidfd_open(parent['pid'], 0)
    try:
        component.verify_supervisor(lifecycle, parent_fd, parent)
    except BaseException:
        os.close(parent_fd)
        raise
    admission_module = loaded
    held, worker, clean = [], None, False
    with lifecycle.handling_stop():
        try:
            admission_module.save_new(output / 'wrapper.json', admission.activity.identity(Path('/proc'), os.getpid()))
            for item, limit, machine in ((plan['worker'], 512 * 1024**2, 62),
                    (plan['images']['unpaired'], 64 * 1024**2, 224),
                    (plan['images']['paired'], 64 * 1024**2, 224)):
                held.append(core.held_file(Path(item['path']), item['sha256'], limit, machine))
            fixture = component.fixtures.make_fixture()
            component.verify_supervisor(lifecycle, parent_fd, parent)
            with lifecycle.deferred_stop():
                worker = core.Worker(held[0][0], held[0][1], c.DEVICE, result_dir)
            child = lifecycle.process(worker.process.pid)
            c.require(child['parent'] == wrapper['pid'] and child['uid'] == wrapper['uid']
                      and child['group'] == child['session'] == wrapper['pid'], 'owned same-session worker')
            identities = {role: {**item, 'bytes': Path(item['path']).stat().st_size}
                          for role, item in (('controller', plan['python']), ('worker', plan['worker']))}
            admitted = evidence.admitted_executables(identities)
            placements = owned_placement(evidence, wrapper, child)

            def alive():
                component.verify_supervisor(lifecycle, parent_fd, parent)
                c.require(lifecycle.process(worker.process.pid) == child, 'worker lifetime changed')

            endpoints = SampleEndpoints(alive,
                lambda: admission.check('active', {'worker_pids': [child['pid']]}),
                lambda after: evidence.endpoint(placements, admitted, after_request=after))
            with (result_dir / 'samples.jsonl').open('x', encoding='utf-8') as samples:
                def retain(row):
                    samples.write(json.dumps(row, sort_keys=True, allow_nan=False) + '\n')
                    samples.flush()
                driver, adapter = c.load_ordered(stage, plan)
                result = driver.run_component(component, adapter, worker, fixture,
                    (held[1][2], plan['images']['unpaired']['sha256']),
                    (held[2][2], plan['images']['paired']['sha256']),
                    plan['mode'], c.DEVICE, retain, endpoints)
            endpoints.finish()
            c.require(evidence.admitted_executables(identities) == admitted, 'admitted executable changed')
            cost = evidence.cpu_cost(*endpoints.endpoints, os.sysconf('SC_CLK_TCK'))
            for role in cost['roles'].values():
                role.pop('cpu_seconds_per_token')
            cost['scope'] = ('From before the first reset to after the last dispatch timing; includes warmups '
                             'and intervening resets/readbacks, excludes final sample readback and final input/free '
                             'checks; not GPU time, model TTFT, TPOT, or per-token CPU')
            admission_module.save_new(output / 'process-endpoints.json', {
                'placements': placements, 'endpoints': endpoints.endpoints,
                'admissions': endpoints.admissions, 'cpu_cost': cost})
            with lifecycle.deferred_stop():
                worker.finish()
                clean = True
            c.require(lifecycle.members(wrapper) == [wrapper], 'owned descendants remain')
            component.verify_supervisor(lifecycle, parent_fd, parent)
            c.require(all(core.identity(os.fstat(fd)) == core.identity(before) for fd, before, _ in held),
                      'held image/worker identity drifted')
            c.require(source_hashes == {name: c.read(stage / name, expected)[1]
                      for name, expected in plan['sources'].items()}, 'component source changed')
            c.verify_files(stage, plan['files'])
            component.save(result_dir / 'result.json', {
                'schema': 'FerricSupervisedPairedSplitKComponentV1', 'accepted': True, 'authority': 'none',
                'model_inference': False, 'model_performance_qualified': False,
                'cache_regime': 'hot-reused-buffer-diagnostic', 'model_streaming_bandwidth_inferred': False,
                'timing_scope': 'ordered host synchronous IPC wall time; not GPU-only',
                'candidate_timing_includes': ['partial', 'merge'], 'warmups_per_arm': 2, 'blocks': 3,
                'source_sha256': source_hashes, 'worker_sha256': plan['worker']['sha256'],
                'images': plan['images'], 'device_unique_id': c.DEVICE, 'wrapper_identity': wrapper,
                'supervisor_identity': parent, 'worker_identity': child, 'clean_teardown': True,
                'outer_admission_required': True, **result})
            admission_module.save_new(output / 'completion.json', {
                'schema': 'FerricPairedSplitKComponentCompletionV1', 'accepted': True, 'mode': plan['mode'],
                'component_result_sha256': c.read(result_dir / 'result.json')[1],
                'endpoints_sha256': c.read(output / 'process-endpoints.json')[1],
                'input_files_stable': True, 'cpu_cost': cost})
            return 0
        except BaseException as error:
            cleanup_error = None
            with lifecycle.deferred_stop(deliver=False):
                if worker is not None and not clean:
                    try:
                        worker.abort()
                    except BaseException as cleanup:
                        cleanup_error = type(cleanup).__name__ + ': ' + str(cleanup)
                component.save(result_dir / 'failure.json', {'error': type(error).__name__ + ': ' + str(error),
                    'cleanup_error': cleanup_error, 'outer_group_cleanup_required': True})
            raise
        finally:
            for descriptor, _, _ in held:
                os.close(descriptor)
            os.close(parent_fd)
