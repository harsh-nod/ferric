#!/usr/bin/env python3
"""Read-only privileged GPU sample for the private matched HTTP supervisor."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent


def require(ok, message):
    if not ok:
        raise ValueError(message)


def load(name, source, expected):
    path = ROOT / source
    raw = path.read_bytes()
    require(len(raw) <= 256 * 1024 and hashlib.sha256(raw).hexdigest() == expected,
            'privileged helper source changed')
    definition = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(definition)
    sys.modules[name] = module
    definition.loader.exec_module(module)
    return module


def executable(pid, expected):
    require(type(expected) is dict and set(expected) == {'path', 'dev', 'ino', 'size'},
            'closed executable inode binding required')
    path = Path('/proc') / str(pid) / 'exe'
    info = path.stat()
    return os.readlink(path) == expected['path'] and all(
        getattr(info, 'st_' + key) == expected[key] for key in ('dev', 'ino', 'size'))


class NativeDiscovery:
    """Pin controller/worker lifetimes on first observation, across both endpoints.

    The supervisor carries these anchors into every later sample. A missing
    previously seen process or replacement is a refusal, not a fresh owner.
    """
    def __init__(self, activity, lifecycle, probe, binding):
        require(type(binding) is dict and set(binding) == {
            'wrapper', 'controller_executable', 'worker_executable', 'controller', 'worker'},
            'closed native discovery binding required')
        self.activity, self.lifecycle, self.probe, self.binding = activity, lifecycle, probe, binding
        self.controller, self.worker = binding['controller'], binding['worker']

    def __call__(self):
        wrapper = self.binding['wrapper']
        rows = self.lifecycle.members(wrapper)
        candidates = [row for row in rows if row['pid'] != wrapper['pid']
                      and row['uid'] == wrapper['uid'] == 9661]
        controllers, workers = [], []
        for row in candidates:
            try:
                if executable(row['pid'], self.binding['controller_executable']):
                    controllers.append(row)
                if executable(row['pid'], self.binding['worker_executable']):
                    workers.append(row)
            except (FileNotFoundError, ProcessLookupError):
                require(not Path('/proc', str(row['pid'])).exists(), 'live unreadable candidate process')
        require(len(controllers) <= 1 and len(workers) <= 1, 'one selected controller/worker only')
        controller = controllers[0] if controllers else None
        require(self.controller is None or controller == self.controller, 'selected controller exited or changed')
        if controller is not None:
            require(controller['parent'] == wrapper['pid'], 'selected controller must be a direct wrapper child')
            self.controller = controller
        if not workers:
            require(self.worker is None, 'selected worker exited after ownership was bound')
            return {}, None
        require(controller is not None, 'selected worker without owned controller')
        identity = self.activity.identity(Path('/proc'), workers[0]['pid'])
        require(self.worker is None or identity == self.worker, 'selected worker lifetime changed')
        self.worker = identity
        return self.probe.NativeOwners(self.activity, self.lifecycle, {
            'wrapper': wrapper, 'controller': controller,
            'worker': {'identity': identity, 'executable': self.binding['worker_executable']}})()


class ContainerDiscovery:
    def __init__(self, activity, custody, binding, *, runner=subprocess.run, check=lambda: None):
        require(type(binding) is dict and set(binding) == {'intent', 'identity', 'runtime'},
                'closed owned-container discovery required')
        custody.validate_intent(binding['intent'])
        self.activity, self.custody = activity, custody
        self.intent, self.identity = binding['intent'], binding['identity']
        self.runtime = binding['runtime']
        self.runner, self.check = runner, check

    def command(self, argv, _label, *, timeout, limit):
        self.check()
        value = self.runner(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, timeout=timeout, check=False)
        require(len(value.stdout) + len(value.stderr) <= limit, 'bounded read-only Docker evidence')
        return value.returncode, value.stdout, value.stderr

    def read_identity(self, proc, pid):
        self.check()
        value = self.activity.identity(proc, pid)
        self.check()
        return value

    def __call__(self):
        self.check()
        rows = self.custody.list_owned(self.command, self.intent, 'probe-exact-container')
        if not rows:
            require(self.identity is None, 'previously owned container disappeared')
            return {}, None
        raw = self.activity.docker_json(['docker', 'inspect', rows[0]['ID']], runner=self.runner)
        values = json.loads(raw)
        self.identity = self.custody.observe(values, self.intent, self.identity)
        state = values[0].get('State', {})
        if not state.get('Running'):
            require(self.runtime is None and state.get('Status') == 'created' and state.get('Pid') == 0,
                    'owned container stopped or failed during observation')
            return {}, None
        binding = {key: self.identity[key] for key in ('id', 'name', 'image', 'label_key', 'label_value')}
        allowed, receipt = self.activity.container_members(binding, Path('/proc'),
            runner=self.runner, identity_reader=self.read_identity)
        require(self.runtime is None or self.runtime == receipt['state'],
                'owned container restarted between samples')
        self.runtime = receipt['state']
        return allowed, receipt


def startup_container(activity, attribution, probe, supplier, devices, budget):
    attempts = []
    result = None
    anchored = {}
    departure_proofs = []
    departure_checks = []
    reconciliation_errors = []
    endpoint_index = 0
    def owners():
        nonlocal endpoint_index
        budget.check()
        allowed, receipt = supplier()
        require(not budget.retired_pids.intersection(allowed), 'departed startup PID reappeared')
        require(all(allowed[pid] == row for pid, row in anchored.items() if pid in allowed)
                and (endpoint_index == 1 or set(anchored) <= set(allowed)),
                'startup owner disappeared or changed across reconciliation')
        anchored.update(allowed)
        endpoint_index += 1
        budget.check()
        return allowed, receipt
    for ordinal in range(3):
        endpoint_index = 0
        result = probe.observe(activity, attribution, 'startup', owners, devices, budget=budget)
        attempts.append(result)
        if result['accepted']:
            try:
                for pid in sorted(budget.retired_pids):
                    check = {'pid': pid, 'terminal': True, 'accepted': False}
                    departure_checks.append(check)
                    budget.check()
                    proof = activity.confirm_departure(Path('/proc'), pid)
                    check['proof'] = proof
                    require(proof == {'pid': pid, 'method': 'proc-directory-absent-and-pidfd-esrch',
                            'proc_directory_absent_checks': 2, 'pidfd_esrch_checks': 2},
                            'departed startup PID reappeared before final acceptance')
                    budget.check()
                    check['accepted'] = True
                    departure_proofs.append({'attempt': ordinal, 'terminal': True, 'proof': proof})
            except (OSError, ValueError, KeyError, TypeError) as error:
                check['error'] = type(error).__name__ + ': ' + str(error)
                result = {**result, 'accepted': False,
                    'errors': ['startup final departure check: ' + type(error).__name__ + ': ' + str(error)]}
            break
        try:
            try:
                probe.startup_birth(result, identity=supplier.identity, runtime=supplier.runtime)
            except (OSError, ValueError, KeyError, TypeError):
                proofs = probe.startup_departures(result, identity=supplier.identity,
                    runtime=supplier.runtime, budget=budget, evidence=departure_checks)
                for item in proofs:
                    pid = item['identity']['pid']
                    require(anchored.get(pid) == item['identity'], 'departure proof anchor changed')
                    budget.retired_pids.add(pid)
                    del anchored[pid]
                    departure_proofs.append({'attempt': ordinal, 'terminal': False, **item})
            budget.check()
        except (OSError, ValueError, KeyError, TypeError) as error:
            reconciliation_errors.append({'attempt': ordinal,
                'error': type(error).__name__ + ': ' + str(error)})
            break
        if ordinal == 2:
            break
    # Preserve rejected records unchanged and keep the final accepted sample
    # independently inspectable; no rejected sample becomes admitted evidence.
    final = dict(result)
    final['startup_reconciliation'] = {
        'schema': 'FerricNativeHttpStartupReconciliationV1',
        'policy': 'no-gpu-exact-container-birth-or-proven-noninit-exit-three-samples-v2',
        'sample_limit': 3, 'shared_seconds': activity.MAX_SECONDS,
        'attempts': attempts, 'accepted_attempt': len(attempts) - 1 if final['accepted'] else None,
        'departure_proofs': departure_proofs,
        'departure_checks': departure_checks, 'reconciliation_errors': reconciliation_errors,
        'timing_admitted': False}
    return final


def perform(value, modules):
    require(type(value) is dict and set(value) == {'schema', 'phase', 'devices', 'owner', 'sources'},
            'closed read-only GPU probe input')
    require(value['schema'] == 'FerricNativeHttpGpuProbeInputV1', 'probe input schema')
    activity, attribution, probe, lifecycle, custody = modules
    owner = value['owner']
    require(type(owner) is dict and set(owner) == {'kind', 'binding'}, 'closed owner selection')
    if owner['kind'] == 'empty':
        require(owner['binding'] is None, 'empty owner has no binding')
        supplier = lambda: ({}, None)
    elif owner['kind'] == 'native':
        supplier = NativeDiscovery(activity, lifecycle, probe, owner['binding'])
    else:
        require(owner['kind'] == 'container', 'unknown owner kind')
        if value['phase'] == 'startup':
            budget = probe.StartupBudget(activity)
            supplier = ContainerDiscovery(activity, custody, owner['binding'],
                runner=budget.run, check=budget.check)
        else:
            supplier = ContainerDiscovery(activity, custody, owner['binding'])
    if owner['kind'] == 'container' and value['phase'] == 'startup':
        result = startup_container(activity, attribution, probe, supplier, value['devices'], budget)
    else:
        options = {'native_owned_fd_rescan': True} if owner['kind'] == 'native' and value['phase'] == 'active' else {}
        result = probe.observe(activity, attribution, value['phase'], supplier, value['devices'], **options)
    if owner['kind'] == 'native':
        result['retained_owner'] = {'controller': supplier.controller, 'worker': supplier.worker}
    elif owner['kind'] == 'container':
        result['retained_owner'] = {'identity': supplier.identity, 'runtime': supplier.runtime}
    else:
        result['retained_owner'] = None
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--input-sha256', required=True)
    args = parser.parse_args()
    require(os.geteuid() == 0, 'privileged visibility is mandatory')
    require(args.input.is_absolute() and not args.input.is_symlink(), 'canonical probe input')
    raw = args.input.read_bytes()
    require(0 < len(raw) <= 128 * 1024 and hashlib.sha256(raw).hexdigest() == args.input_sha256,
            'bounded immutable probe input')
    value = json.loads(raw)
    names = ('gpu_activity.py', 'gpu_attribution.py', 'gpu_probe.py',
             'frozen/native_lifecycle.py', 'container_custody.py')
    require(set(value['sources']) == set(names), 'closed privileged helper source roster')
    modules = [load('http_probe_cli_' + str(index), name, value['sources'][name])
               for index, name in enumerate(names)]
    result = perform(value, modules)
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    raise SystemExit(0 if result['accepted'] else 125)


if __name__ == '__main__':
    main()
