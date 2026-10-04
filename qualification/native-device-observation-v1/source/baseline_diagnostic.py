"""Retained Four-forward tensor diagnostics; no launches or acceptance policy."""
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
OBSERVER = ('p228-independent-decode-observation-v1',
    '10125c91f9c83c55379c5769b2d1bfa099835744b10f292c9ebd5f152ae7e9c3')
HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
ALIASES = ('layer_validation', 'host_validation', 'host_comparison', 'observation',
           'policy_portable', 'group_fence_portable', 'intake', 'run')
LEAVES = ('before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
SMI_ARGV = ['/opt/rocm/bin/amd-smi', 'process', '--gpu', '0000:05:00.0', '0000:15:00.0', '--json']
FALSE = ('gpu_execution_requested', 'numerical_acceptance', 'independent_numerical_acceptance',
    'independent_tensor_acceptance',
    'full_model_acceptance', 'full_model_correctness', 'sustained_2048_256', 'performance_claim',
    'production_authority', 'current_source_binary_image_authority_verified',
    'current_platform_idle_audits_verified', 'top_level_observer_reaping_verified',
    'runtime_premises_discharged', 'arithmetic_prerequisites_verified')


def require(value, message):
    if not value:
        raise ValueError(message)


def bootstrap():
    path = E / OBSERVER[0] / 'run_row_facts_v2.py'
    require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical frozen custody helper')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact unchanged custody helper')
    module = types.ModuleType('independent_decode_diagnostic_custody'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def loaded(D, pins):
    directory = D.package(pins, *OBSERVER)
    manifest, manifest_pin = pins.json(directory / 'manifest.json', OBSERVER[1])
    hashes = {row['path']: row['sha256'] for row in manifest['files']}
    previous = {name: sys.modules.get(name) for name in ALIASES}
    modules = {}
    try:
        for name in ALIASES:
            modules[name] = D.load_module(pins, directory / (name + '.py'), hashes[name + '.py'],
                'independent_decode_diagnostic_' + name)
            sys.modules[name] = modules[name]
        I, M = modules['intake'], modules['run']
        C, H = I.comparator(pins)
    finally:
        for name, prior in previous.items():
            if prior is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = prior
    return I, M, C, H, manifest_pin


def replay_leaves(I, pins, receipt, directory):
    I.V.require(type(receipt['leaves']) is dict and set(receipt['leaves']) == set(LEAVES),
                'one native leaf and all six retained audit leaves')
    values = {}
    for name in LEAVES:
        row = receipt['leaves'][name]
        I.V.keys(row, 'result retained_files')
        retained = row['retained_files']
        I.V.require(type(retained) is dict
            and set(retained) == {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'},
            'complete owned leaf file roster')
        for filename, pin in retained.items():
            I.V.require(pin['path'] == str(directory / name / filename), 'owned leaf retained path')
            I.read(pins, pin, 8 << 20, False)
        I.V.require(row['result'] == retained['result.json'], 'leaf result pin join')
        value = I.doc(pins, row['result']); I.owned_success(value)
        for key, filename in (('command', 'command.json'), ('started', 'started.json'),
                              ('stdout', 'stdout'), ('stderr', 'stderr')):
            I.V.require(value[key] == retained[filename], 'owned leaf record identity')
        I.V.require(value['gpu_execution_requested'] is (name == 'parent'), 'native-only recorded GPU leaf')
        if name != 'parent':
            command = I.doc(pins, value['command']); started = I.doc(pins, value['started'])
            I.V.require(command['argv'] == SMI_ARGV and command['cwd'] == str(I.R)
                and command['deadline_seconds'] == 30 and command['affinity'] == [8, 9]
                and command['nice'] == 10 and command['address_space_bytes'] == 12 << 30
                and command['file_cap_bytes'] == 64 << 20 and command['stream_cap_bytes'] == 8 << 20
                and command['gpu_execution_requested'] is False
                and started['command_sha256'] == value['command']['sha256']
                and value['stderr']['bytes'] == 0, 'unchanged recorded process-audit envelope')
        values[name] = value
    for when in ('before', 'after'):
        rows = receipt[when + '_audits']
        I.V.require(type(rows) is list and len(rows) == 3, 'complete recorded audit triple')
        for index, row in enumerate(rows):
            I.V.keys(row, 'topology process_result')
            name = when + '-' + str(index)
            I.V.require(row['process_result'] == receipt['leaves'][name]['result']
                and row['topology']['path'] == str(directory / (name + '-topology.json')),
                'recorded topology/process audit join')
            I.doc(pins, row['topology'], 1 << 20)
    return values['parent']


def replay_observation(I, M, C, H, pins, receipt_pin, receipt, manifest_pin):
    V = I.V
    V.require(receipt['schema'] == 'ferric-p228-independent-decode-observation-v1'
        and receipt['passed'] is True and receipt['failures'] == []
        and type(receipt['native_attempts']) is int and receipt['native_attempts'] == 1
        and type(receipt['retries']) is int and receipt['retries'] == 0
        and receipt['gpu_execution_requested'] is True,
        'actual completed one-shot independent observation')
    for key in ('full_model_correctness', 'independent_numerical_acceptance', 'numerical_acceptance',
            'independent_tensor_acceptance', 'full_model_acceptance', 'native_baseline_comparison_performed',
            'independent_framework_comparison_performed', 'sustained_2048_256', 'gpu_time',
            'performance_claim', 'production_authority'):
        V.require(receipt[key] is False, 'observation did not claim numerical or performance authority')
    V.require(receipt['supervisor_manifest'] == manifest_pin
        and receipt['controller'] == pins.pin(E / OBSERVER[0] / 'run.py'), 'frozen actual observer generation')
    plan = I.doc(pins, receipt['plan'], 1 << 20); I.input_shape(plan)
    directory = E / plan['output_label']
    V.require(receipt_pin['path'] == str(directory / 'complete.json'), 'actual successful case namespace')
    for key in ('request', 'deployment', 'image_deployment', 'standalone_prepared', 'standalone_cases', 'numericals'):
        V.require(receipt[key] == plan[key], 'completion binds original observation plan: ' + key)
    request, policy = I.HC.H.request(I.doc(pins, plan['request'], 64 << 10))
    V.require(receipt['mode'] == request['mode'] and receipt['policy'] == policy == plan['policy'],
              'exact recorded mode and policy')
    runtime = receipt['selected_runtime']
    V.require(runtime['parent'] == receipt['parent'] and runtime['worker'] == receipt['worker']
        and C.pin(request['prefix_image']) == runtime['image'], 'recorded selected runtime and V7 override')
    parent = replay_leaves(I, pins, receipt, directory)
    candidate = dict(native_files=receipt['retained_native'], host_sidecar=receipt['host_sidecar'],
        request=plan['request'], parent=runtime['parent'], owner=receipt['leaves']['parent']['result'],
        **{key: parent[key] for key in ('command', 'started', 'stdout', 'stderr')})
    read = lambda pin, maximum: I.read(pins, pin, maximum)
    observed, files, structural, ownership, host = I.HC.candidate(C, read, candidate, H)
    observed_plan = dict(schema=I.OBS.INPUT_SCHEMA, mode=request['mode'], policy=policy,
        request=plan['request'], prefix_image=runtime['image'], candidate=candidate)
    # The helper replays recorded custody only; no new runtime review is constructed.
    checked = I.OBS.observe(C, observed_plan, read, H, I.HC)
    c = dict(request=request, policy=policy, plan=plan, selected_runtime=runtime)
    M.checked_observation(c, checked)
    V.require(receipt['observation']['path'] == str(directory / 'observation.json')
        and I.doc(pins, receipt['observation']) == receipt['checked'] == checked
        and receipt['owned_children'] == ownership and checked['structural'] == structural
        and checked['host_observation'] == host, 'independent native observation replay agrees exactly')
    return plan, observed, files, checked


def compare_retained(I, C, H, pins, plan, observed, files):
    read = lambda pin, maximum: I.read(pins, pin, maximum)
    records, payloads = C.reference(read, plan['comparison_reference'], observed['request']['mode'], H)
    rows = C.compare_rows(records, payloads, C.records(observed),
        [files[f'observation-{position}.bin'] for position in range(4)], H['diagnostics'])
    return rows


def summary(rows):
    require(type(rows) is list and len(rows) == 4, 'four diagnostic positions')
    comparable, incomparable, tensors = [], [], []
    diverged = False
    for position, row in enumerate(rows):
        require(row['position'] == position and type(row['same_input_history']) is bool,
                'exact ordered history diagnostics')
        if not row['same_input_history']:
            diverged = True
            require(row['tensors'] is None, 'different histories have no tensor comparison')
            incomparable.append(position)
        else:
            require(not diverged and type(row['tensors']) is list and len(row['tensors']) == 38,
                    'all38 tensors only before history divergence')
            comparable.append(position); tensors.extend(row['tensors'])
    return dict(captured_tensor_rows=152, comparable_positions=comparable, incomparable_positions=incomparable,
        compared_tensor_rows=len(tensors), byte_equal_tensor_rows=sum(row['byte_equal'] for row in tensors),
        differing_tensor_rows=sum(not row['byte_equal'] for row in tensors))


def source_pins(D, pins):
    base = Path(__file__).resolve().parent
    for name in ('run.py', 'test_run.py', 'README.md'):
        pins.pin(base / name)
    return {path: row for path, row in pins.records.items()
            if Path(path).suffix in ('.py', '.md') or Path(path).name == 'manifest.json'}


def execute(observation, expected, output):
    require(not sys.flags.optimize and not os.environ.get('PYTHONOPTIMIZE'), 'ordinary Python required')
    require(type(expected) is str and re.fullmatch('[0-9a-f]{64}', expected), 'root-supplied observation SHA256')
    output = Path(output)
    require(output.is_absolute() and not os.path.lexists(output)
        and output.parent.resolve(strict=True) == output.parent, 'fresh canonical diagnostic directory')
    D = bootstrap(); pins = D.Pins()
    I, M, C, H, manifest_pin = loaded(D, pins)
    # The supplied digest authenticates bytes before the actual extent becomes an input pin.
    receipt_pin, raw = pins.read(Path(observation), expected, retain=True, maximum=8 << 20)
    receipt = D.parse(raw)
    before = source_pins(D, pins)
    output.mkdir(mode=0o700)
    before_pin = I.save(output / 'sources-before.json', before)
    plan, observed, files, checked = replay_observation(I, M, C, H, pins, receipt_pin, receipt, manifest_pin)
    rows = compare_retained(I, C, H, pins, plan, observed, files)
    counts = summary(rows)
    pins.recheck()
    after = {path: pins.pin(path, row['sha256']) for path, row in before.items()}
    require(before == after, 'all loaded source bytes unchanged')
    after_pin = I.save(output / 'sources-after.json', after)
    result = dict(schema='ferric-p228-independent-decode-diagnostic-v1', authority='none',
        status='RETAINED_TENSOR_DIAGNOSTICS_ONLY', passed=True, observation=receipt_pin,
        observer_package=manifest_pin, plan=receipt['plan'], request=plan['request'],
        mode=receipt['mode'], policy=receipt['policy'], selected_runtime=receipt['selected_runtime'],
        reference=plan['comparison_reference'], structural=checked['structural'],
        owned_record_checks=checked['owned_record_checks'], independent_framework=rows, summary=counts,
        sources_before=before_pin, sources_after=after_pin, source_postchecks_passed=True,
        retained_native_and_close_replayed=True, recorded_six_audit_leaf_bytes_rehashed=True,
        independent_reference_compared=True, acceptance_threshold=None,
        input_pins=dict(pins.records), **{key: False for key in FALSE})
    return I.save(output / 'complete.json', result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--observation', required=True)
    parser.add_argument('--observation-sha', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    print(json.dumps(execute(args.observation, args.observation_sha, args.output), sort_keys=True))


if __name__ == '__main__':
    main()
