"""Owned ASROCK layer-zero reference capture; no candidate or numerical acceptance."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OLD_SHA = '810eab5d9f674f7042b32ed9f81af691c0d8fbad5a028478ce758e116621ee19'
CAPTURE = {
    'run.py': '646ffc536f50610a4c3e790a4cb8fa945e5e1501bc2466c59ba8f6d5e92b8d32',
    'test_run.py': 'd4f85754e435f8559cf19078ea0aa320cbe8223d324ef906b072ad61ef0e4ad6',
    'README.md': 'e63ccb85b397b02435200083c93526905c57a4eeba82e5f81b5387fd0cf33563',
}
BASE_SHA = '613579c7c84ed9b6f94abf6864c9934b72331dd001bf6badb973ab4576f4db97'
LIMITS = dict(timeout_seconds=900, host_rss_cap_bytes=64 << 30,
              minimum_free_host_bytes=48 << 30, minimum_free_gpu_bytes=48 << 30,
              output_cap_bytes=32 << 20, cache_cap_bytes=1 << 30)
FALSE = ('numerical_acceptance', 'performance_claim', 'production_authority')
INPUT_FIELDS = {'schema', 'launcher_sha256', 'owned_helper', 'capture_package', 'reference_helper',
                'reference_support', 'legacy_plan', 'source_authentication', 'token_provenance',
                'topology', 'platform_monitor', 'implementation_sources', 'output_label',
                'capture_label', 'execution_review'}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(row, cap=4 << 20):
    require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'}, 'closed FilePin')
    path = Path(row['path'])
    require(type(row['bytes']) is int and 0 <= row['bytes'] <= cap
            and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256'])
            and path.is_absolute() and path.resolve(strict=True) == path, 'bounded canonical pin')
    before = path.stat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1, 'unaliased regular input')
    raw = path.read_bytes()
    after = path.stat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) and len(raw) == row['bytes']
            and sha(raw) == row['sha256'], 'stable authenticated input')
    return raw


def load(row, expected, name):
    require(row['sha256'] == expected, 'fixed helper implementation')
    raw = read(row)
    module = types.ModuleType(name)
    module.__file__ = row['path']
    exec(compile(raw, row['path'], 'exec'), module.__dict__)
    read(row)
    return module


def input_shape(value):
    require(type(value) is dict and set(value) == INPUT_FIELDS
            and value['schema'] == 'ferric-p228-layer0-framework-launch-inputs-v1', 'closed launch inputs')
    require(re.fullmatch(r'layer0-framework-launch-v228-v[1-9][0-9]{0,8}', value['output_label'])
            and re.fullmatch(r'layer0-framework-capture-v228-v[1-9][0-9]{0,8}', value['capture_label']),
            'fresh isolated launch and capture labels')
    require(type(value['capture_package']) is dict and set(value['capture_package']) == set(CAPTURE),
            'complete three-file capture package')
    parents = set()
    for name, row in value['capture_package'].items():
        require(row['sha256'] == CAPTURE[name] and Path(row['path']).name == name, 'exact capture source')
        parents.add(Path(row['path']).parent)
    require(len(parents) == 1 and next(iter(parents)).is_relative_to(E), 'one retained package directory')
    require(value['reference_helper']['sha256'] == BASE_SHA, 'original independent reference helper')
    require(set(value['reference_support']) == {'diagnostics', 'policy', 'long_reference'}, 'reference closure')
    root = Path(value['reference_helper']['path']).parent
    for key, name in (('diagnostics', 'diagnostics.py'), ('policy', 'policy.json'),
                      ('long_reference', 'helpers/long_reference.py')):
        require(value['reference_support'][key]['path'] == str(root / name), 'original local support path')
    require(set(value['implementation_sources']) == {'modeling_qwen3', 'activation', 'sdpa', 'torch_functional'},
            'four actual installed source pins')
    require(value['topology']['host'] == 'asrock-1w300-g2-2b'
            and value['topology']['card'] == 'card1'
            and re.fullmatch('[0-9a-f]{16}', value['topology']['uid']), 'explicit ASROCK device')


def review_inputs(value, review):
    require(type(review) is dict and set(review) == {'schema', 'reviewed', 'inputs_projection_sha256',
            'resources', 'gpu_execution_authorized', *FALSE}, 'closed root launch review')
    require(review['schema'] == 'ferric-p228-layer0-framework-launch-review-v1'
            and review['reviewed'] is True and review['gpu_execution_authorized'] is True
            and review['inputs_projection_sha256'] == sha(encoded({k: v for k, v in value.items()
                                                                  if k != 'execution_review'}))
            and review['resources'] == LIMITS and all(review[k] is False for k in FALSE),
            'separately authored exact root review')


def configure_monitor(old, value, topology):
    old.UID = topology['uid']
    if value is None:
        return
    require(type(value) is dict and set(value) == {'schema', 'host', 'boot_id', 'process', 'executable',
            'filesystem_uid', 'filesystem_gid', 'read_only_attestation_reviewed'}
            and value['schema'] == 'ferric-p223-platform-monitor-v1'
            and value['host'] == topology['host'] and value['boot_id'] == topology['boot_id']
            and value['read_only_attestation_reviewed'] is True, 'fresh reviewed monitor document')
    row = value['process']
    require(set(row) == {'pid', 'uid', 'ppid', 'pgid', 'session', 'start_ticks', 'comm', 'cmdline_hex'}
            and all(type(row[k]) is int for k in ('pid', 'uid', 'ppid', 'pgid', 'session', 'start_ticks'))
            and row['pid'] > 1 and row['start_ticks'] > 0 and row['uid'] == 0 and row['ppid'] == 1
            and row['pgid'] == row['session'] == row['pid'] and row['comm'] == 'gpuagent'
            and row['cmdline_hex'] == b'/usr/local/bin/gpuagent\0'.hex(), 'closed root-owned monitor identity')
    require(value['executable']['path'] == '/usr/local/bin/gpuagent'
            and all(type(value[k]) is int and value[k] >= 0 for k in ('filesystem_uid', 'filesystem_gid')),
            'fixed read-only monitor executable and owners')
    read(value['executable'], 128 << 20)
    old.MONITOR_PID = row['pid']
    old.MONITOR_PROCESS = dict(row)
    old.MONITOR_PATH = value['executable']['path']
    old.MONITOR_BYTES = value['executable']['bytes']
    old.MONITOR_SHA = value['executable']['sha256']
    # Only the admission identity changes; original proc, SMI and sudo/read/reap machinery remains.
    def document(actual, current):
        require(actual == value and current == topology, 'unchanged reviewed monitor and topology')
    old.monitor_document = document


def joined_inventory(old_inventory, owner, capture):
    result = old_inventory(owner)
    if not capture.exists():
        return result
    require(capture.resolve(strict=True) == capture and capture.is_dir(), 'owned capture directory identity')
    cache_root = capture / 'private-cache'
    require(not os.path.lexists(cache_root) or (cache_root.resolve(strict=True) == cache_root
            and cache_root.is_dir()), 'owned private cache directory identity')
    whole = old_inventory(capture)
    cache = old_inventory(capture / 'private-cache')
    ordinary = whole['total'] - cache['total']
    maximum = 0
    for path in capture.rglob('*'):
        if path.is_file() and not path.is_relative_to(capture / 'private-cache'):
            maximum = max(maximum, path.stat().st_size)
    result['total'] += whole['total']
    result['cache'] += cache['total']
    result['other'] += ordinary
    result['output'] += ordinary
    result['files'] += whole['files']
    result['maximum_file'] = max(result['maximum_file'], maximum)
    result['maximum_provider_file'] = max(result['maximum_provider_file'], cache['maximum_file'])
    require(result['files'] <= 32768, 'combined owned file roster cap')
    return result


def audit(old, label, monitor, topology, owner, env):
    require(old.device() == topology, 'fresh exact selected device')
    if monitor is not None:
        return old.monitor_audit(label, monitor, topology, owner, env)
    result, pin = old.bounded(['/usr/bin/amd-smi', 'process', '--json'], owner / (label + '-process'),
                              owner, env, 30)
    old.smi_idle(old.decode(old.checked(result['stdout'])))
    require(old.device() == topology, 'topology stable during no-monitor audit')
    return dict(smi_result=pin, sample=old.idle_sample(), exclusive_reservation=False)


def reference_plan(inputs, old, output, pid):
    legacy = parse(read(inputs['legacy_plan']))
    return dict(schema='ferric-p224-rearm-four-framework-reference-plan-v1',
        model_id='f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a',
        bundle_id='6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b',
        harness_sha256=BASE_SHA, diagnostics=inputs['reference_support']['diagnostics'],
        policy=inputs['reference_support']['policy'], legacy_plan=inputs['legacy_plan'],
        mode='teacher_forced', input_tokens=[9112, 2190, 3772, 220], token_provenance=inputs['token_provenance'],
        source_authentication=inputs['source_authentication'], host=inputs['topology']['host'],
        boot_id=inputs['topology']['boot_id'], gpu_unique_id=inputs['topology']['uid'],
        gpu_unique_id_file='/sys/class/drm/card1/device/unique_id', hip_visible_devices='0',
        python_executable=str(old.PYTHON), python_prefix=str(old.PYTHON.parent.parent),
        model_root=legacy['model_root'], output_root=str(output), supervisor_pid=pid,
        execution_review=inputs['execution_review'], resources=LIMITS)


def approval(old, owner, projection, capture_module, input_pins):
    path = owner / 'approval.json'
    deadline = time.monotonic() + 300
    while not os.path.lexists(path):
        require(time.monotonic() < deadline, 'root approval timeout; no GPU attempt')
        require(shutil.disk_usage(owner).free >= old.ONGOING_FREE, 'approval disk floor')
        old.storage_ok(old.inventory(owner))
        time.sleep(.2)
    require(time.monotonic() < deadline, 'root approval arrived after deadline; no GPU attempt')
    record = old.pin(path)
    value = parse(read(record))
    require(type(value) is dict and set(value) == {'schema', 'projection_sha256', 'execution_review'}
            and value['schema'] == 'ferric-p228-layer0-framework-launch-approval-v1'
            and value['projection_sha256'] == sha(encoded(projection)), 'exact root approval projection')
    review_pin = value['execution_review']
    review = parse(read(review_pin))
    plan = dict(projection, execution_review=review_pin)
    capture_module.plan_shape(plan)
    capture_module.reviewed(plan, review, LIMITS)
    input_pins.extend((record, review_pin))
    return plan


def validate_report(old, capture_module, report, directory, plan_pin):
    require(report['schema'] == 'ferric-p228-layer0-framework-capture-v1' and report['status'] == 'PASS'
            and report['repeat_passes_byte_equal'] is True and report['genuine_framework_chain'] is True
            and report['position'] == 0 and report['input_token'] == 9112
            and report['captured_stages_per_pass'] == 33 and len(report['passes']) == 2
            and plan_pin in report['input_pins'] and report['gpu_execution'] is True
            and all(report[k] is False for k in ('candidate_gpu_execution', 'candidate_intermediate_inputs',
                'conditional_replay_performed', 'numerical_acceptance', 'full_model_correctness',
                'production_authority', 'performance_measured')),
            'genuine repeat capture only, never independent candidate acceptance')
    previous = None
    for ordinal, run in enumerate(report['passes'], 1):
        require(run['ordinal'] == ordinal and run['position'] == 0 and run['input_token'] == 9112
                and run['fresh_cache'] is True and set(run['stages']) == set(capture_module.SHAPES),
                'two complete position-zero stage rosters')
        values = {}
        for name, row in run['stages'].items():
            require(set(row) == {'dtype', 'shape', 'pin'} and row['dtype'] == 'bfloat16'
                    and row['shape'] == list(capture_module.SHAPES[name])
                    and Path(row['pin']['path']).parent == directory
                    and row['pin']['bytes'] == capture_module.stage_bytes(name), 'exact captured stage pin')
            values[name] = old.checked(row['pin'])
        capture_module.validate_values(values)
        require(previous is None or previous == values, 'actual repeat bytes identical')
        previous = values
    for row in report['input_pins']:
        old.checked(row)


def completed_status(result):
    return (not result['failures'] and result['native_attempts'] == 1 and result['reference'] is not None
            and len(result['before_audits']) == len(result['after_audits']) == 3)


def post_audits(result, observe):
    for index in range(3):
        try:
            result['after_audits'].append(observe(index))
        except BaseException as error:
            result['failures'].append('post-audit: ' + type(error).__name__ + ': ' + str(error))


def main(input_path, expected):
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
            and os.getuid() == os.geteuid() == 9661, 'unprivileged ordinary -B controller')
    path = Path(input_path)
    raw = path.read_bytes()
    require(len(raw) <= 1 << 20 and sha(raw) == expected, 'explicit input digest')
    inputs = parse(raw)
    input_shape(inputs)
    old = load(inputs['owned_helper'], OLD_SHA, 'layer0_owned_p224')
    initial = old.pin(path)
    require(initial['sha256'] == expected, 'unchanged input')
    own = old.pin(Path(__file__).resolve(strict=True))
    require(own['sha256'] == inputs['launcher_sha256'], 'reviewed launcher source')
    pins = [initial, own, inputs['owned_helper'], inputs['reference_helper'], inputs['legacy_plan'],
            inputs['source_authentication'], inputs['execution_review'], *inputs['capture_package'].values(),
            *inputs['reference_support'].values(), *inputs['token_provenance'].values(),
            *inputs['implementation_sources'].values()]
    for row in pins:
        read(row)
    review_inputs(inputs, parse(read(inputs['execution_review'])))
    monitor = None if inputs['platform_monitor'] is None else parse(read(inputs['platform_monitor']))
    if monitor is not None:
        pins.extend((inputs['platform_monitor'], monitor['executable']))
    topology = inputs['topology']
    configure_monitor(old, monitor, topology)
    require(old.device() == topology and old.CPUS <= os.sched_getaffinity(0), 'fresh ASROCK device and CPUs')
    owner, output = E / inputs['output_label'], E / inputs['capture_label']
    require(not os.path.lexists(output) and shutil.disk_usage(E).free >= old.INITIAL_FREE,
            'fresh capture output and unchanged initial disk floor')
    owner = old.new_directory(owner)
    old.save(owner / 'inputs.json', inputs)
    original_inventory = old.inventory
    old.inventory = lambda current: joined_inventory(original_inventory, current, output)
    env = old.environment()
    for name in ('home', 'tmp'):
        (owner / name).mkdir(mode=0o700)
    env['HOME'], env['TMPDIR'] = str(owner / 'home'), str(owner / 'tmp')
    python = dict(path=str(old.PYTHON), link=os.readlink(old.PYTHON) if old.PYTHON.is_symlink() else None,
                  target=old.pin(old.PYTHON.resolve(strict=True), 128 << 20))
    old.save(owner / 'python-identity.json', python)
    smi = old.pin(Path('/usr/bin/amd-smi').resolve(strict=True), 128 << 20)
    tools = [old.pin(Path(name), 128 << 20) for name in old.READ_TOOLS] if monitor is not None else []
    source = Path(inputs['capture_package']['run.py']['path']).parent
    result = dict(schema='ferric-p228-layer0-framework-launch-complete-v1', passed=False, failures=[],
                  native_attempts=0, retries=0, reference=None, before_audits=[], after_audits=[],
                  candidate_gpu_execution=False, **{key: False for key in FALSE})
    try:
        tests, result['cpu_result'] = old.bounded(['/usr/bin/python3', '-B', '-m', 'unittest', 'discover', '-v',
            '-s', str(source), '-p', 'test_run.py'], owner / 'cpu-tests', owner, env, 120)
        transcript = old.checked(tests['stderr']).decode('utf-8')
        require(re.search(r'^Ran 16 tests in [^\n]+\n\nOK\s*$', transcript, re.MULTILINE)
                and len(re.findall(r'^test_\w+ .* \.\.\. ok$', transcript, re.MULTILINE)) == 16,
                'all sixteen pure capture tests pass')
        capture_module = load(inputs['capture_package']['run.py'], CAPTURE['run.py'], 'layer0_capture_policy')
        listing, _ = old.bounded(['/usr/bin/amd-smi', 'list', '--json'], owner / 'pre-list', owner, env, 30)
        old.parse_smi_list(old.decode(old.checked(listing['stdout'])), topology['bdf'])
        for index in range(3):
            result['before_audits'].append(audit(old, 'before-' + str(index), monitor, topology, owner, env))
            if index != 2:
                time.sleep(1)
        reference = reference_plan(inputs, old, output, os.getpid())
        reference_pin = old.save(owner / 'reference-plan.json', reference)
        projection = dict(schema='ferric-p228-layer0-framework-capture-plan-v1',
            harness_sha256=CAPTURE['run.py'], reference_helper=inputs['reference_helper'],
            reference_plan=reference_pin, output_root=str(output), implementation_sources=inputs['implementation_sources'])
        projection_pin = old.save(owner / 'capture-projection.json', projection)
        ready = old.save(owner / 'ready.json', dict(schema='ferric-p228-layer0-framework-launch-ready-v1',
            supervisor_pid=os.getpid(), reference_plan=reference_pin, capture_projection=projection_pin,
            projection_sha256=sha(encoded(projection)), approval_path=str(owner / 'approval.json'),
            review_deadline_seconds=300, gpu_execution=False))
        print(json.dumps(dict(ready=ready)), flush=True)
        plan = approval(old, owner, projection, capture_module, pins)
        plan_pin = old.save(owner / 'capture-plan.json', plan)
        result['plan'] = plan_pin
        argv = [str(old.PYTHON), '-I', '-B', str(source / 'run.py')]
        inspection, result['inspection_result'] = old.bounded([*argv, '--inspect', plan_pin['path'], plan_pin['sha256']],
            owner / 'inspect', owner, env, 60)
        inspected = old.decode(old.checked(inspection['stdout']))
        require(inspected['schema'] == 'ferric-p228-layer0-framework-capture-inspection-v1'
                and inspected['gpu_opened'] is False and inspected['installed_callables_loaded'] is False,
                'no-GPU inspection passed')
        for row in pins:
            read(row, 128 << 20)
        old.save(owner / 'immediate-idle.json', audit(old, 'immediate', monitor, topology, owner, env))
        result['native_attempts'] = 1
        execution, result['execution_result'] = old.bounded([*argv, '--run-reviewed-layer0-capture',
            plan_pin['path'], plan_pin['sha256']], owner / 'execute', owner, env, old.DEADLINE)
        reference_output = old.pin(output / 'capture.json')
        require(old.decode(old.checked(execution['stdout'])) == reference_output, 'actual capture stdout pin')
        validate_report(old, capture_module, old.decode(old.checked(reference_output)), output, plan_pin)
        result['reference'] = reference_output
    except BaseException as error:
        result['failures'].append(type(error).__name__ + ': ' + str(error))
    finally:
        post_audits(result, lambda index: audit(old, 'after-' + str(index), monitor, topology, owner, env))
        try:
            for row in pins:
                read(row, 128 << 20)
            require(old.device() == topology and old.pin(old.PYTHON.resolve(strict=True), 128 << 20) == python['target']
                    and (os.readlink(old.PYTHON) if old.PYTHON.is_symlink() else None) == python['link']
                    and old.pin(Path('/usr/bin/amd-smi').resolve(strict=True), 128 << 20) == smi,
                    'post-run topology, interpreter and SMI identity')
            for row in tools:
                old.checked(row, 128 << 20)
            old.storage_ok(old.inventory(owner))
        except BaseException as error:
            result['failures'].append('postcheck: ' + type(error).__name__ + ': ' + str(error))
    result['passed'] = completed_status(result)
    result['input_pins'], result['topology'] = pins, topology
    result['gpu_execution'] = result['native_attempts'] == 1
    complete = old.save(owner / 'complete.json', result)
    print(json.dumps(dict(complete=complete, passed=result['passed'])), flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    require(len(sys.argv) == 3, 'launch.py INPUTS SHA256')
    sys.exit(main(*sys.argv[1:]))
