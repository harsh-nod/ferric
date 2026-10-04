"""Publish retained genuine framework captures; no imports of executable helpers."""
import argparse
import ast
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
Q = Path('/home/harsh/ferric-p227-integration/qualification/layer0-framework-capture-v1')
OWNER = 'layer0-framework-launch-v228-v1'
CAPTURE = 'layer0-framework-capture-v228-v1'
INPUTS = 'layer0-framework-inputs-v228-v1'
SOURCES = {
    'capture': ('p228-layer0-framework-capture-v1', 'run.py', 'test_run.py',
        '646ffc536f50610a4c3e790a4cb8fa945e5e1501bc2466c59ba8f6d5e92b8d32',
        'd4f85754e435f8559cf19078ea0aa320cbe8223d324ef906b072ad61ef0e4ad6'),
    'launcher': ('p228-layer0-framework-launch-v1', 'launch.py', 'test_launch.py',
        '81d7e77ebd4ed1be20842a477c482fb80d67e6ad71f3c2aead18e7a7ce19e23c',
        '7d325bccddf90eba0c6c25e28d4b7ac38460c1c78787c5441db14b08ba4498a7'),
}
FALSE = ('candidate_gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority')
CHECKED = {}
COPIES = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')


def parse(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def body(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local retained file')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 8 << 20,
            'bounded unaliased evidence body')
    raw = path.read_bytes()
    after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    value = dict(bytes=len(raw), sha256=sha(raw))
    require(stamp(before) == stamp(after) and len(raw) == before.st_size
            and CHECKED.setdefault(path, value) == value, 'stable retained evidence')
    return raw


def pin(path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals':
        relative = Path(*relative.parts[1:])
    raw = body(path)
    return dict(path=str(E / relative), bytes=len(raw), sha256=sha(raw))


def verify(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}
        and type(value['bytes']) is int and 0 <= value['bytes'] <= 8 << 20
        and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'closed evidence pin')
    path = Path(value['path'])
    require(path.is_relative_to(E) and '..' not in path.parts and str(path) == value['path'], 'retained namespace')
    relative = path.relative_to(E)
    proposal = relative.parts[0].startswith('p228-layer0-framework-') or relative.parts[0] == 'p224-rearm-framework'
    local = L / ('proposals' if proposal else '') / relative
    require(pin(local) == value, 'exact original body identity')
    return local


def document(path, expected=None):
    raw = body(path)
    require(expected is None or sha(raw) == expected, 'explicit actual receipt SHA')
    return parse(raw)


def add(name, path):
    require(name not in COPIES and not Path(name).is_absolute() and '..' not in Path(name).parts,
            'unique public path')
    raw = body(path)
    require(not raw.startswith(b'\x7fELF') and path.suffix not in ('.bf16', '.bin', '.safetensors'),
            'no tensor, executable or model body publication')
    raw.decode('utf-8')
    COPIES[name] = (raw, pin(path))


def source_constant(path, name):
    rows = [node.value for node in ast.parse(body(path)).body if isinstance(node, ast.Assign)
            and any(isinstance(t, ast.Name) and t.id == name for t in node.targets)]
    require(len(rows) == 1, 'one literal source constant')
    return ast.literal_eval(rows[0])


def test_names(path):
    return sorted(node.name for node in ast.walk(ast.parse(body(path)))
                  if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))


def tests_log(raw, names):
    text = raw.decode('utf-8')
    actual = re.findall(r'^(test_\w+) \([^\r\n]+\) \.\.\. ok$', text, re.MULTILINE)
    require(sorted(actual) == names and len(actual) == len(set(actual))
        and re.search(r'^Ran ' + str(len(names)) + r' tests in [^\n]+\n\nOK\s*$', text, re.MULTILINE),
        'all actual named tests passed without skips')


def pure(role, expected):
    package, main, tests, main_sha, test_sha = SOURCES[role]
    folder = L / ('layer0-framework-' + ('launch' if role == 'launcher' else 'capture') + '-pure-v228-v1')
    value = document(folder / 'complete.json', expected)
    count = 18 if role == 'launcher' else 16
    require(value['schema'] == 'ferric-p228-layer0-framework-' + ('launch' if role == 'launcher' else 'capture') + '-pure-v1'
        and value['passed'] is True and value['tests'] == count
        and value['errors'] == value['failures'] == value['skipped'] == 0
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
        and all(value[k] is False for k in ('framework_execution', 'gpu_execution', 'numerical_acceptance',
            'performance_claim', 'production_authority', 'runtime_audit_executed')), 'actual pure-only qualification')
    before = document(verify(value['sources_before']))
    require(before == document(verify(value['sources_after'])) and set(before) == {main, tests}
        and before[main]['sha256'] == main_sha and before[tests]['sha256'] == test_sha, 'tested exact source generation')
    for name, row in before.items():
        require(Path(row['path']) == E / package / name, 'actual tested source location')
        add('source/' + role + '/' + name, verify(row))
    names = test_names(verify(before[tests])); require(len(names) == count, 'source test census')
    tests_log(body(verify(value['transcript'])), names)
    for name in ('complete.json', 'sources-before.json', 'sources-after.json', 'tests.log'):
        add('tests/' + role + '/' + name, folder / name)
    add('tools/' + Path(value['controller']['path']).name, verify(value['controller']))
    add('source/' + role + '/README.md', L / 'proposals' / package / 'README.md')
    return value, names


def leaf(directory, name, supervisor):
    root = directory / name
    value = document(root / 'result.json')
    require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['failure'] is None,
            'actual natural zero exit, not a killed or failed leaf')
    command = document(verify(value['command']))
    started = document(root / 'started.json')
    require(value['command'] == pin(root / 'command.json')
        and started['command_sha256'] == value['command']['sha256']
        and type(started['pid']) is int and started['pid'] > 1 and started['pgid'] == started['pid']
        and started['supervisor_pid'] == supervisor, 'command/start/direct-owner join')
    for kind in ('stdout', 'stderr'):
        require(value[kind] == pin(root / kind), 'actual leaf stream pin')
        verify(value[kind])
    if 'group_absent' in value:
        require(value['group_absent'] is True and value['performance_measured'] is False
            and value['started'] == pin(root / 'started.json')
            and command['affinity'] == [8, 9] and command['nice'] == 10
            and command['group_rss_cap_bytes'] == 64 << 30
            and command['initial_free_bytes'] == 40 << 30 and command['ongoing_free_bytes'] == 38 << 30
            and command['output_cap_bytes'] == 32 << 20 and command['cache_cap_bytes'] == 1 << 30,
            'unchanged ordinary owned boundary and reap')
    else:
        require(value['observed_utility_tree_absent'] is True and value['read_only'] is True
            and value['platform_monitor_signalled'] is False and value['gpu_execution'] is False
            and command['read_only'] is True and command['gpu_execution'] is False
            and command['platform_monitor_signalled'] is False and command['seconds'] == 10,
            'read-only utility naturally exited and observed tree absent')
    return value, command


def audit(value, label, directory, leaves, monitor):
    require(value['exclusive_reservation'] is False and value['process'] == monitor['process'], 'same reviewed monitor')
    sample = value['sample']
    require(sample['gpu_busy_percent'] == 0
        and sample['mem_info_vram_total'] - sample['mem_info_vram_used'] >= 48 << 30, 'idle/free-memory sample')
    pid = str(monitor['process']['pid'])
    for field, suffix, operation in (('live_path_result', 'live-path', 'readlink'),
                                    ('live_sha_result', 'live-sha', 'sha256sum')):
        name = label + '-' + suffix; result, command = leaves[name]
        require(value[field] == pin(directory / name / 'result.json'), 'audit utility identity')
        require(command['argv'] == ['/usr/bin/sudo', '-n', '--', '/usr/bin/timeout', '--foreground',
            '--kill-after=1', '5', '/usr/bin/' + operation, '--', '/proc/' + pid + '/exe'], 'fixed read-only operation')
        expected = (monitor['executable']['path'] + '\n' if operation == 'readlink' else
                    monitor['executable']['sha256'] + '  /proc/' + pid + '/exe\n').encode()
        require(body(verify(result['stdout'])) == expected, 'actual monitor executable identity output')
    name = label + '-process'; result, command = leaves[name]
    require(value['smi_result'] == pin(directory / name / 'result.json')
        and command['argv'] == ['/usr/bin/amd-smi', 'process', '--json'], 'actual audit command')
    smi = document(verify(result['stdout']))
    require(type(smi) is list and len(smi) == 1 and set(smi[0]) == {'gpu', 'process_list'}
        and type(smi[0]['gpu']) is int and smi[0]['gpu'] == 0 and len(smi[0]['process_list']) == 1,
        'one selected device and one admitted monitor only')
    rows = smi[0]['process_list']; require(set(rows[0]) == {'process_info'}, 'closed SMI process')
    info = rows[0]['process_info']
    require(set(info) == {'name', 'pid', 'memory_usage', 'mem_usage', 'usage', 'cu_occupancy', 'evicted_time'}
        and info['name'] == 'N/A' and info['pid'] == monitor['process']['pid']
        and type(info['cu_occupancy']) is int and info['cu_occupancy'] == 0
        and set(info['memory_usage']) == {'gtt_mem', 'cpu_mem', 'vram_mem'}
        and set(info['usage']) == {'gfx', 'enc'}, 'closed zero-activity monitor')
    for row, unit in [(v, 'B') for v in info['memory_usage'].values()] + [(info['mem_usage'], 'B')] + \
                     [(v, 'ns') for v in info['usage'].values()] + [(info['evicted_time'], 'ms')]:
        require(set(row) == {'value', 'unit'} and type(row['value']) is int
            and row['value'] == 0 and row['unit'] == unit, 'zero reported monitor allocation/activity')


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('complete_sha256'); parser.add_argument('capture_pure_sha256'); parser.add_argument('launch_pure_sha256')
    args = parser.parse_args()
    require(all(re.fullmatch('[0-9a-f]{64}', s) for s in vars(args).values()), 'explicit actual receipt digests')
    owner, capture = L / OWNER, L / CAPTURE
    complete = document(owner / 'complete.json', args.complete_sha256)
    require(complete['schema'] == 'ferric-p228-layer0-framework-launch-complete-v1'
        and complete['passed'] is True and complete['failures'] == [] and complete['native_attempts'] == 1
        and complete['retries'] == 0 and complete['gpu_execution'] is True
        and all(complete[k] is False for k in FALSE), 'actual one-attempt independent framework execution')
    capture_pure, capture_tests = pure('capture', args.capture_pure_sha256)
    launch_pure, _ = pure('launcher', args.launch_pure_sha256)
    inputs = document(owner / 'inputs.json')
    require(complete['input_pins'][0] == pin(L / INPUTS / 'inputs.json')
        and inputs == document(L / INPUTS / 'inputs.json') and inputs['topology'] == complete['topology'],
        'root inputs and actual topology')
    for role, (package, main_name, test_name, main_sha, test_sha) in SOURCES.items():
        require(pin(L / 'proposals' / package / main_name)['sha256'] == main_sha
            and pin(L / 'proposals' / package / test_name)['sha256'] == test_sha, 'qualified source bodies')
    require(inputs['launcher_sha256'] == SOURCES['launcher'][3], 'actual executed launcher generation')
    for name, row in inputs['capture_package'].items():
        require(row == pin(L / 'proposals' / SOURCES['capture'][0] / name), 'executed capture source generation')
    for row in [inputs['owned_helper'], inputs['reference_helper'], *inputs['reference_support'].values()]:
        path = verify(row)
        add('source/retained-helpers/' + str(Path(row['path']).relative_to(E / 'p224-rearm-framework')), path)
    for path in sorted((L / INPUTS).iterdir()):
        require(path.name in ('inputs.json', 'monitor.json', 'root-review.json'), 'closed root input roster')
        add('inputs/' + path.name, path)
    monitor = document(verify(inputs['platform_monitor']))
    require(monitor['host'] == complete['topology']['host'] and monitor['boot_id'] == complete['topology']['boot_id']
        and monitor['read_only_attestation_reviewed'] is True, 'reviewed platform identity')
    plan = document(verify(complete['plan']))
    require(complete['plan'] == pin(owner / 'capture-plan.json')
        and plan == document(capture / 'input-plan.json') and plan['harness_sha256'] == SOURCES['capture'][3]
        and plan['output_root'] == str(E / CAPTURE), 'actual capture plan and source')
    reference = document(verify(plan['reference_plan'])); supervisor = reference['supervisor_pid']
    ready = document(owner / 'ready.json'); projection = document(owner / 'capture-projection.json')
    require(ready['supervisor_pid'] == supervisor and ready['reference_plan'] == plan['reference_plan']
        and ready['capture_projection'] == pin(owner / 'capture-projection.json')
        and projection == {k: v for k, v in plan.items() if k != 'execution_review'}
        and ready['projection_sha256'] == sha(encoded(projection)), 'actual PID and closed plan projection')
    review = document(verify(plan['execution_review'])); approval = document(owner / 'approval.json')
    require(review['schema'] == 'ferric-p228-layer0-framework-execution-review-v1'
        and review['reviewed'] is True and review['gpu_execution_authorized'] is True
        and review['plan_projection_sha256'] == ready['projection_sha256']
        and all(review[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority'))
        and approval['execution_review'] == plan['execution_review']
        and approval['projection_sha256'] == ready['projection_sha256'], 'separate root approval, no invented authority')
    root_review = document(verify(inputs['execution_review']))
    require(root_review['inputs_projection_sha256'] == sha(encoded({k: v for k, v in inputs.items() if k != 'execution_review'}))
        and root_review['reviewed'] is True and root_review['gpu_execution_authorized'] is True
        and root_review['resources'] == review['resources'] == reference['resources'], 'root launch review identity')
    labels = [side + '-' + str(i) for side in ('before', 'after') for i in range(3)] + ['immediate']
    names = {'cpu-tests', 'pre-list', 'inspect', 'execute'} | {label + '-' + suffix for label in labels
        for suffix in ('live-path', 'live-sha', 'process')}
    leaves = {name: leaf(owner, name, supervisor) for name in sorted(names)}
    require(len(leaves) == 25, 'four ordinary work leaves and seven three-leaf audits')
    for field, name in (('cpu_result', 'cpu-tests'), ('inspection_result', 'inspect'), ('execution_result', 'execute')):
        require(complete[field] == pin(owner / name / 'result.json'), 'actual primary leaf identity')
    tests_log(body(owner / 'cpu-tests/stderr'), capture_tests)
    for side in ('before', 'after'):
        require(len(complete[side + '_audits']) == 3, 'all three audits')
        for index, value in enumerate(complete[side + '_audits']):
            audit(value, side + '-' + str(index), owner, leaves, monitor)
    audit(document(owner / 'immediate-idle.json'), 'immediate', owner, leaves, monitor)
    for name, mode, seconds in (('inspect', '--inspect', 60), ('execute', '--run-reviewed-layer0-capture', 900)):
        value, command = leaves[name]
        require(command['argv'] == [reference['python_executable'], '-I', '-B', str(E / SOURCES['capture'][0] / 'run.py'),
            mode, complete['plan']['path'], complete['plan']['sha256']] and command['seconds'] == seconds,
            'exact qualified inspection/execution command')
    require(parse(body(owner / 'execute/stdout')) == complete['reference'], 'actual child capture receipt')
    inspection = document(owner / 'inspect/stdout')
    require(inspection['schema'] == 'ferric-p228-layer0-framework-capture-inspection-v1'
        and inspection['gpu_opened'] is False and inspection['installed_callables_loaded'] is False, 'no-GPU inspection')
    report = document(verify(complete['reference']))
    require(complete['reference'] == pin(capture / 'capture.json')
        and report['schema'] == 'ferric-p228-layer0-framework-capture-v1' and report['status'] == 'PASS'
        and report['genuine_framework_chain'] is True and report['repeat_passes_byte_equal'] is True
        and report['position'] == 0 and report['input_token'] == 9112 and report['captured_stages_per_pass'] == 33
        and report['gpu_execution'] is True and report['acceptance_threshold'] is None
        and all(report[k] is False for k in ('candidate_gpu_execution', 'candidate_intermediate_inputs',
            'conditional_replay_performed', 'numerical_acceptance', 'full_model_correctness',
            'production_authority', 'performance_measured')) and complete['plan'] in report['input_pins'],
        'genuine capture only, no numerical or full-model acceptance')
    source = L / 'proposals' / SOURCES['capture'][0] / 'run.py'
    shapes, joins = source_constant(source, 'SHAPES'), source_constant(source, 'JOINS')
    require(len(shapes) == 33 and len(report['passes']) == 2, 'actual complete stage census')
    previous, tensors = None, []
    for ordinal, record in enumerate(report['passes'], 1):
        require(record['ordinal'] == ordinal and record['position'] == 0 and record['input_token'] == 9112
            and record['fresh_cache'] is True and set(record['stages']) == set(shapes), 'fresh genuine pass identity')
        values = {}
        for name, shape in shapes.items():
            row = record['stages'][name]; count = 1
            for n in shape: count *= n
            require(set(row) == {'dtype', 'shape', 'pin'} and row['dtype'] == 'bfloat16'
                and row['shape'] == list(shape) and row['pin']['path'] == str(E / CAPTURE / f'pass{ordinal}-{name}.bf16')
                and row['pin']['bytes'] == 2 * count, 'genuine typed tensor extent and path')
            raw = body(verify(row['pin']))
            require(all(word & 0x7f80 != 0x7f80 for (word,) in struct.iter_unpack('<H', raw)), 'finite BF16 words')
            values[name] = raw
            tensors.append(dict(pass_ordinal=ordinal, stage=name, dtype=row['dtype'], shape=row['shape'],
                elements=count, source_pin=row['pin'], published_raw=False))
        require(all(values[a] == values[b] for a, b in joins), 'actual producer/consumer joins')
        require(previous is None or previous == values, 'both actual complete stage maps repeat exactly')
        previous = values
    for name, row in report['retained_implementation_sources'].items():
        raw = body(verify(row)); installed = plan['implementation_sources'][name]
        require(row['bytes'] == installed['bytes'] and row['sha256'] == installed['sha256']
            and report['runtime']['implementation_sources'][name] == installed, 'actual installed/retained source bytes')
    root_names = {'ready.json', 'approval.json', 'capture-plan.json', 'inputs.json', 'python-identity.json',
        'immediate-idle.json', 'root-capture-review.json', 'complete.json', 'reference-plan.json', 'capture-projection.json'}
    expected = root_names | {name + '/' + item for name in names
        for item in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
    found = {str(p.relative_to(owner)) for p in owner.rglob('*') if p.is_file()}
    require(found == expected, 'closed retained owner file census')
    for name in sorted(found): add('owner/' + name, owner / name)
    add('capture/capture.json', capture / 'capture.json')
    add('capture/input-plan.json', capture / 'input-plan.json')
    add('tools/prepare_layer0_framework_p228_v1.py', L / 'prepare_layer0_framework_p228_v1.py')
    add('tools/publish.py', Path(__file__).resolve(strict=True))
    ledger = dict(schema='ferric-p228-layer0-framework-tensor-ledger-v1', passes=2, stages_per_pass=33,
        actual_repeated_bytes_equal=True, producer_consumer_joins_checked=True, finite_bf16_checked=True,
        tensors=tensors, raw_tensor_bodies_published=False, numerical_acceptance=False)
    summary = dict(schema='ferric-p228-layer0-framework-qualification-v1', status='PASS',
        owner_complete=pin(owner / 'complete.json'), capture=complete['reference'],
        pure_tests=dict(capture=16, launcher=18, in_owner_capture_rerun=16),
        pure_receipts=dict(capture=pin(L / 'layer0-framework-capture-pure-v228-v1/complete.json'),
            launcher=pin(L / 'layer0-framework-launch-pure-v228-v1/complete.json')),
        owned_leaves=25, pre_audits=3, immediate_audits=1, post_audits=3, native_attempts=1, retries=0,
        passes=2, captured_stages_per_pass=33, repeated_stage_bytes_equal=True, finite_bf16_checked=True,
        position=0, input_token=9112, model=report['model'], model_id=report['model_id'], bundle_id=report['bundle_id'],
        runtime=report['runtime'], topology=complete['topology'], typed_tensor_ledger='tensor-ledger.json',
        capture_elapsed_seconds_includes_setup=leaves['execute'][0]['elapsed_seconds'],
        peak_owned_child_rss_bytes=leaves['execute'][0]['peak_group_rss_bytes'],
        natural_owned_child_and_utility_exits_verified=True, top_level_ssh_reaping_independently_verified=False,
        all_transitive_inputs_rehashed=False, installed_binaries_or_model_weights_rehashed=False,
        raw_tensor_bodies_published=False, third_party_implementation_bodies_published=False,
        genuine_framework_gpu_execution=True, candidate_gpu_execution=False, numerical_acceptance=False,
        full_model_correctness=False, acceptance_threshold=None, performance_claim=False,
        production_authority=False, sustained_2048_256=False, target_700_tokens_per_second=False,
        limitations=['This verifies retained records, not a rerun of their GPU execution.',
            'Original input/model/library identity assertions remain attributed to the qualified capture.',
            'The primary observed the outer command terminal; this helper verifies only recorded owned child lifetimes.',
            'Repeated framework outputs alone do not accept Ferric numerical results or measure decode throughput.'])
    for path in list(CHECKED): body(path)
    if Q.exists():
        require(Q.resolve(strict=True) == Q and Q.is_dir() and not Q.is_symlink(), 'canonical destination')
        require({p.name for p in Q.iterdir()} <= {'README.md'}, 'fresh destination except optional root README')
        if (Q / 'README.md').exists(): body(Q / 'README.md')
    else:
        require(not Q.is_symlink(), 'no destination alias'); Q.mkdir()
    published = []
    COPIES['tensor-ledger.json'] = (encoded(ledger), None)
    for name, (raw, source_pin) in sorted(COPIES.items()):
        target = Q / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream: stream.write(raw)
        require(body(target) == raw, 'exact published body')
        published.append(dict(path=name, bytes=len(raw), sha256=sha(raw), source_pin=source_pin))
    for path in list(CHECKED): body(path)
    summary['files'] = published
    summary['checked_local_files'] = len(CHECKED)
    with (Q / 'result.json').open('xb') as stream: stream.write(encoded(summary))
    print(json.dumps(dict(path=str(Q / 'result.json'), bytes=(Q / 'result.json').stat().st_size,
                         sha256=sha(body(Q / 'result.json')), files=len(published))), flush=True)


if __name__ == '__main__':
    main()
