"""CPU-only retained native ownership replay followed by unchanged diagnostics."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
R = E.parents[1]
P = E / 'p228-layer0-current-diagnostic-v1'
GPU = E / 'p228-layer0-native-capture-gpu-v1'
GPU_MANIFEST_SHA = '957115945b1e1a12a6ee6b7fd815a98a33b9d7f1bdc99b77293cb5556b560535'
GPU_RUN_SHA = '3ba662f337f2456741805bf19930acbe267e0e589b4dcbb9a0d87f81aa5173c2'
CURRENT_PURE_SHA = '7af96a474c6d0d27b9f502cc8e6f7e70e4dd22e69d64e24ab131968259b5236c'
FRAMEWORK_SHA = '46fd9acbca798f05bc737a651e6fba54e65c78688e2d425a8287eef402065edc'
BASELINE = dict(path=str(E / 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1/complete.json'),
    bytes=963187, sha256='00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073')
SOURCES = {
    'compare.py': '1598e22a3460a9ed2c350fb5d2b5fec6b8c37648b53bb2f1fc714c1fb3506d3f',
    'current.py': 'eb103b681c54b0c880bc6e692363208dae837eb322845bab7e593bdea6330d7b',
    'diagnostics.py': '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf',
    'test_current.py': '55260e410cb601e9af38cb6e7c35c6c153c7a0f7d11269a40d581cad1c56b3b4',
}
LEAVES = ('before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
RAW = {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
SMI_ARGV = ['/opt/rocm/bin/amd-smi', 'process', '--gpu', '0000:05:00.0', '0000:15:00.0', '--json']
DEVICES = [16366993098680759275, 10838076764495710945]
FALSE = ('paired_comparison_performed', 'full_model_correctness', 'independent_numerical_acceptance',
    'numerical_acceptance', 'independent_tensor_acceptance', 'full_model_acceptance',
    'independent_framework_comparison_performed', 'full_forward', 'sustained_2048_256', 'gpu_time',
    'calibrated_nanoseconds', 'cross_device_clock_alignment', 'overlap_claim', 'performance_claim',
    'production_authority')


def require(value, message):
    if not value:
        raise ValueError(message)


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def filepin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}, 'exact FilePin')
    p = Path(value['path'])
    require(type(value['path']) is str and p.is_absolute() and str(p) == value['path']
        and '..' not in p.parts and type(value['bytes']) is int and 0 <= value['bytes'] <= 16 << 20
        and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'bounded FilePin')
    return value


def actual(path, digest=None):
    path = Path(path)
    require(path.resolve(strict=True) == path, 'canonical retained path')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= 16 << 20, 'bounded regular file')
        raw = stream.read((16 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
        'stable retained read')
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(digest is None or pin['sha256'] == digest, 'authenticated input digest')
    return pin, raw


class Reader:
    def __init__(self, mapping):
        require(type(mapping) is dict and len(mapping) <= 160, 'bounded explicit transport map')
        self.mapping, self.consumed = mapping, {}
        for original, retained in mapping.items():
            require(type(original) is str and Path(original).is_absolute()
                and str(Path(original)) == original and '..' not in Path(original).parts, 'original path')
            filepin(retained)
            require(Path(retained['path']).is_relative_to(E), 'transport remains under evidence root')

    def __call__(self, pin):
        filepin(pin)
        selected = self.mapping.get(pin['path'], pin)
        require((selected['bytes'], selected['sha256']) == (pin['bytes'], pin['sha256']),
            'transport cannot change original bytes')
        seen, raw = actual(selected['path'], pin['sha256'])
        require(seen == selected, 'transported extent and identity')
        row = dict(original=pin, retained=seen)
        require(self.consumed.setdefault(pin['path'], row) == row, 'consistent original identity')
        return raw

    def doc(self, pin):
        return parse(self(pin))

    def recheck(self):
        for row in self.consumed.values():
            require(actual(row['retained']['path'])[0] == row['retained'], 'all consumed bytes unchanged')


def module(reader, pin, name, aliases=None):
    missing = object()
    saved = {key: sys.modules.get(key, missing) for key in (aliases or {})}
    try:
        sys.modules.update(aliases or {})
        value = types.ModuleType(name)
        value.__file__ = pin['path']
        exec(compile(reader(pin), pin['path'], 'exec'), value.__dict__)
        return value
    finally:
        for key, value in saved.items():
            if value is missing:
                sys.modules.pop(key, None)
            else:
                sys.modules[key] = value


def pure(reader, pin, schema, count, sources):
    value = reader.doc(pin)
    require(value['schema'] == schema and value['passed'] is True and value['tests'] == count
        and value['errors'] == value['failures'] == value['skipped'] == 0
        and value['source_postchecks_passed'] is True, 'actual source-bound synthetic tests')
    require(reader.doc(value['sources_before']) == reader.doc(value['sources_after']) == sources,
        'tested source identities')
    reader(value['transcript'])
    reader(value['controller'])


def owned_leaf(reader, outer, directory, name, environment):
    entry = outer['leaves'][name]
    require(set(entry) == {'result', 'retained_files'} and set(entry['retained_files']) == RAW,
        'exact five owned leaf files')
    records = entry['retained_files']
    for member, pin in records.items():
        require(pin['path'] == str(directory / name / member), 'original owned leaf path')
        reader(pin)
    require(entry['result'] == records['result.json'], 'owned result pin')
    value = reader.doc(entry['result'])
    require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
        and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
        and value['owned_processes_reaped'] is True, 'natural zero and fully reaped ownership')
    for key, member in (('command', 'command.json'), ('started', 'started.json'),
                        ('stdout', 'stdout'), ('stderr', 'stderr')):
        require(value[key] == records[member], 'owned raw file join')
    gpu = name == 'parent'
    argv = ([outer['parent']['path'], '--request', outer['request']['path'],
        '--capture-layer-zero', '--allow-unauthenticated-machine-code'] if gpu else SMI_ARGV)
    command = reader.doc(value['command'])
    require(command == dict(argv=argv, env=environment, cwd=str(R), deadline_seconds=4000 if gpu else 30,
        affinity=[8, 9], nice=10, address_space_bytes=(32 if gpu else 12) << 30,
        file_cap_bytes=64 << 20, stream_cap_bytes=8 << 20, gpu_execution_requested=gpu)
        and value['gpu_execution_requested'] is gpu, 'recorded original command and fixed bounds')
    start = reader.doc(value['started'])
    require(set(start) == {'parent', 'supervisor_pid', 'command_sha256'}
        and start['command_sha256'] == value['command']['sha256'], 'started command digest')
    parent = start['parent']
    require(parent['pid'] == parent['pgid'] == parent['sid'] and parent['uid'] == 9661
        and parent['ppid'] == start['supervisor_pid']
        and parent in [row['identity'] for row in value['lineage'] if row.get('event') == 'owned'],
        'started identity present in actual owned lineage')
    if not gpu:
        require(reader(value['stderr']) == b'', 'process audit stderr empty')
    return value, start


def audit_samples(reader, outer, directory, leaves, platform, audit_validation, topology):
    require(platform['schema'] == 'ferric-p227-prefix-parity-platform-review-v1'
        and platform['reviewed'] is True and platform['authority'] == 'none'
        and platform['devices'] == DEVICES and platform['production_authority'] is False
        and platform['runtime_premises_discharged'] is False, 'original scoped platform review')
    require([Path(row['device_path']).name for row in platform['topology_identity']] == SMI_ARGV[3:5],
        'recorded process selection joins reviewed physical devices')
    for side in ('before', 'after'):
        rows = outer[side + '_audits']
        require(type(rows) is list and len(rows) == 3, 'three audits on each side')
        samples = []
        for index, row in enumerate(rows):
            name = side + '-' + str(index)
            require(set(row) == {'topology', 'process_result'}
                and row['process_result'] == outer['leaves'][name]['result']
                and row['topology']['path'] == str(directory / (name + '-topology.json')), 'audit event joins')
            sample = reader.doc(row['topology'])
            require(type(sample['devices']) is list and len(sample['devices']) == 2, 'two selected devices')
            identity = [{k: v for k, v in device.items() if k not in ('gpu_busy', 'memory_busy', 'vram_used')}
                for device in sample['devices']]
            require(identity == platform['topology_identity']
                and all(device['gpu_busy'] == device['memory_busy'] == 0 for device in sample['devices']),
                'recorded idle reviewed topology, not a fresh ASROCK audit')
            audit_validation.empty_processes(reader(leaves[name][0]['stdout']))
            samples.append(sample)
        topology.require_idle(samples)


def native(reader, pin, platform_pin, CV, audit_validation, topology):
    outer = reader.doc(pin)
    directory = Path(pin['path']).parent
    require(directory.parent == E and re.fullmatch(r'prefix-layer0-native-capture-gpu-v228-v[1-9][0-9]{0,8}',
        directory.name) and Path(pin['path']).name == 'complete.json', 'genuine capture outer namespace')
    require(outer['schema'] == 'ferric-p228-layer0-native-capture-gpu-v1' and outer['passed'] is True
        and outer['failures'] == [] and type(outer['native_attempts']) is int and outer['native_attempts'] == 1
        and type(outer['retries']) is int and outer['retries'] == 0 and outer['gpu_execution_requested'] is True
        and outer['current_tf4_hidden_equal'] is True and outer['captured_arrays'] == 28
        and all(outer[key] is False for key in FALSE), 'completed one-attempt diagnostic only')
    require(set(outer['leaves']) == set(LEAVES), 'exact seven owned leaves')
    require(outer['controller']['path'] == str(GPU / 'run.py')
        and outer['controller']['sha256'] == GPU_RUN_SHA
        and outer['supervisor_manifest']['path'] == str(GPU / 'manifest.json')
        and outer['supervisor_manifest']['sha256'] == GPU_MANIFEST_SHA, 'qualified original supervisor')
    reader(outer['controller']); reader(outer['supervisor_manifest'])
    plan = reader.doc(outer['plan'])
    require(plan['schema'] == 'ferric-p228-layer0-native-capture-inputs-v1'
        and plan['output_label'] == directory.name and outer['baseline'] == plan['baseline'] == BASELINE,
        'same actual Down2 baseline and original capture plan')
    for key in ('parent', 'worker', 'request', 'parent_runtime_review', 'worker_runtime_review', 'capture_review'):
        require(outer[key] == plan[key], 'original plan identity: ' + key)
    require(outer['parent_cpu_complete'] == plan['parent_cpu']
        and outer['worker_cpu_complete'] == plan['worker_cpu'], 'actual parent and worker qualification joins')
    require(outer['parent_cpu_complete']['sha256'] == '78c12f822e95d50a1239f56411c5b8da9411a3fbda7510fbbf38d5644e1dffbb'
        and outer['worker_cpu_complete']['sha256'] == '41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d'
        and outer['parent']['sha256'] == 'e7fb0571cfb8114ef86987525a6b882c4fdc84329022db5eabfdb131f3e3e12d'
        and outer['parent']['bytes'] == 13480088
        and outer['worker']['sha256'] == 'd2af909e7fef88af5b20f4ae8f2a5779a7aee93a187169e97a939bec7d3a0fed'
        and outer['worker']['bytes'] == 5018856, 'actually qualified parent and unchanged worker')
    ledger = dict(outer['input_pins'])
    ledger.update(outer['standalone_input_pins'])
    require(ledger.get(platform_pin['path']) == platform_pin, 'platform review was an original admission input')
    platform = reader.doc(platform_pin)
    for role in ('parent', 'worker'):
        review = reader.doc(outer[role + '_runtime_review'])
        require(review['reviewed'] is True and review['authority'] == 'none'
            and review['binary'] == outer[role] and review['host'] == platform['host']
            and review['boot_id'] == platform['boot_id'] and review['production_authority'] is False,
            'original selected runtime review, no new authority')
    review = reader.doc(outer['capture_review'])
    require(review['reviewed'] is True and review['authority'] == 'none'
        and review['request'] == outer['request'] and review['baseline'] == BASELINE
        and review['production_authority'] is False and review['numerical_acceptance'] is False,
        'original engineering-only capture review')
    first_command = reader.doc(outer['leaves']['parent']['retained_files']['command.json'])
    environment = first_command['env']
    require(type(environment) is dict and all(environment.get(k) == '' for k in
        ('CUDA_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES')), 'recorded KFD-only environment')
    leaves = {name: owned_leaf(reader, outer, directory, name, environment) for name in LEAVES}
    require(len({start['supervisor_pid'] for _, start in leaves.values()}) == 1
        and len({(start['parent']['pid'], start['parent']['starttime']) for _, start in leaves.values()}) == 7,
        'one supervisor and seven distinct owned process incarnations')
    audit_samples(reader, outer, directory, leaves, platform, audit_validation, topology)
    records = outer['retained_native']
    require(set(records) == CV.BODY | {'summary.json'}, 'exact native twelve-file roster')
    for name, record in records.items():
        require(record['path'] == str(directory / 'native' / name), 'native evidence path')
    body = {name: reader(records[name]) for name in CV.BODY}
    summary = reader(records['summary.json'])
    parent, started = leaves['parent']
    require(reader(parent['stdout']) == summary + b'\n', 'parent stdout equals retained summary')
    baseline = reader.doc(BASELINE)
    hidden = reader(baseline['retained_native']['observation-0.bin'])[:8192]
    checked = CV.validate(summary, body, reader.doc(outer['request']), hidden)
    require(checked == outer['checked'] == reader.doc(outer['observation']), 'actual structural replay')
    pid = checked['closed_child_pids'][0]
    CV.child_marker(reader(parent['stderr']), pid)
    root = started['parent']
    owned = [row['identity'] for row in parent['lineage'] if row.get('event') == 'owned']
    require(root in owned and pid != root['pid'] and 1 <= len(owned) <= 2
        and len({(row['pid'], row['starttime']) for row in owned}) == len(owned)
        and {row['pid'] for row in owned} <= {root['pid'], pid}, 'only parent and captured worker')
    found = [row for row in owned if row['pid'] == pid]
    if found:
        child = found[0]
        require(child['ppid'] == root['pid'] and child['pid'] == child['pgid'] and child['sid'] == root['sid']
            and child['uid'] == root['uid'] and child['starttime'] >= root['starttime'], 'actual worker identity')
    identities = dict(parent=root, workers=[dict(pid=pid, identity=found[0] if found else None,
        outer_pidfd_observed=bool(found))], parent_asserted_close_and_reap=True,
        outer_groups_absent=True, synthesized_child_identity=False)
    require(identities == outer['owned_children'], 'retained independent child lineage replay')
    return outer, records['summary.json']


def framework(reader):
    pin, _ = actual(E / 'layer0-framework-launch-v228-v1/complete.json', FRAMEWORK_SHA)
    value = reader.doc(pin)
    require(value['passed'] is True and value['failures'] == [] and value['native_attempts'] == 1
        and value['retries'] == 0 and len(value['before_audits']) == len(value['after_audits']) == 3,
        'actual successful independent framework experiment')
    def natural(record, utility=False):
        row = reader.doc(record)
        require(row['exit_code'] == 0 and row['failure'] is None
            and row['observed_utility_tree_absent' if utility else 'group_absent'] is True,
            'original framework owned result')
        for key in ('command', 'stdout', 'stderr'):
            reader(row[key])
        if 'started' in row:
            reader(row['started'])
    for key in ('cpu_result', 'inspection_result', 'execution_result'):
        natural(value[key])
    for audit in value['before_audits'] + value['after_audits']:
        for key in ('live_path_result', 'live_sha_result', 'smi_result'):
            natural(audit[key], key != 'smi_result')
        require(audit['sample']['gpu_busy_percent'] == 0, 'recorded framework idle audit')
    return pin, value['reference']


def main(args):
    require(len(args) == 2 and not sys.flags.optimize and sys.dont_write_bytecode
        and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary -B PLAN_PATH PLAN_SHA invocation')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
        and all(os.environ.get(k) == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
        'fixed ASROCK CPU-only execution')
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        finite = [x for x in (soft, hard, limit) if x != resource.RLIM_INFINITY]
        resource.setrlimit(kind, (min(finite), min(finite)))
    plan_pin, raw = actual(Path(args[0]), args[1]); plan = parse(raw)
    require(set(plan) == {'schema', 'native_outer', 'platform', 'transport', 'runner_tests', 'output_label'}
        and plan['schema'] == 'ferric-p228-layer0-current-comparison-inputs-v1'
        and type(plan['output_label']) is str
        and re.fullmatch(r'layer0-current-comparison-v228-v[1-9][0-9]{0,8}', plan['output_label']), 'closed comparison plan')
    out = E / plan['output_label']; require(not os.path.lexists(out), 'fresh comparison output')
    reader = Reader(plan['transport']); reader(plan_pin)
    own = actual(Path(__file__).resolve())[0]; reader(own)
    own_sources = {name: actual(Path(__file__).resolve().with_name(name))[0] for name in ('run.py', 'test_run.py')}
    for pin in own_sources.values(): reader(pin)
    pure(reader, plan['runner_tests'], 'ferric-p228-layer0-current-comparison-pure-v1', 15, own_sources)
    source_pins = {name: actual(P / name, digest)[0] for name, digest in SOURCES.items()}
    current_pure = actual(E / 'layer0-current-diagnostic-pure-v228-v1/complete.json', CURRENT_PURE_SHA)[0]
    pure(reader, current_pure, 'ferric-p228-layer0-current-diagnostic-pure-v1', 18, source_pins)
    for pin in source_pins.values(): reader(pin)
    C = module(reader, source_pins['compare.py'], 'retained_genuine_compare')
    current = module(reader, source_pins['current.py'], 'retained_current_compare', {'compare': C})
    diagnostics = C.load_diagnostics(reader, source_pins['diagnostics.py'])
    gpu_manifest = actual(GPU / 'manifest.json', GPU_MANIFEST_SHA)[0]
    manifest = reader.doc(gpu_manifest)
    gpu_sources = {row['path']: dict(row, path=str(GPU / row['path'])) for row in manifest['files']}
    require(manifest['schema'] == 'ferric-p228-layer0-native-capture-gpu-package-v1'
        and len(gpu_sources) == len(manifest['files']) == 10, 'frozen qualified capture package')
    for pin in gpu_sources.values(): reader(pin)
    V = module(reader, gpu_sources['layer_validation.py'], 'retained_layer_validation')
    CV = module(reader, gpu_sources['capture_validation.py'], 'retained_capture_validation', {'layer_validation': V})
    audit_pin = actual(E / 'p228-independent-gpu-observation-v1/validation.py',
        'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1')[0]
    topology_pin = actual(R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py',
        '6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a')[0]
    audit_validation = module(reader, audit_pin, 'retained_empty_process_validation')
    topology = module(reader, topology_pin, 'retained_topology_idle_check')
    outer, summary = native(reader, plan['native_outer'], plan['platform'], CV, audit_validation, topology)
    framework_outer, reference = framework(reader)
    original = actual(E / 'framework-rearm-v224-v1/reference/reference.json',
        '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416')[0]
    comparison = current.compare_retained(reference, original, BASELINE, summary, reader, diagnostics)
    require(comparison['comparable_rows'] == 24 and comparison['numerical_acceptance'] is False
        and comparison['acceptance_threshold'] is None and comparison['ordering_is_a_causal_proof'] is False,
        'unchanged diagnostic-only comparator result')
    reader.recheck()
    report = dict(schema='ferric-p228-layer0-current-comparison-observation-v1', completed=True,
        controller=own, plan=plan_pin, runner_tests=plan['runner_tests'], comparison_tests=current_pure,
        native_outer=plan['native_outer'], native_owned_leaves_replayed=7, native_audits_replayed=6,
        native_structural_replayed=True, framework_outer=framework_outer,
        framework_owned_leaf_results_rechecked=21, native_supervisor_manifest=gpu_manifest,
        source_pins=source_pins, comparison=comparison, consumed=list(reader.consumed.values()),
        source_postchecks_passed=True, all_transitive_admission_inputs_replayed=False,
        current_gpu_audits_performed=False, gpu_execution=False, native_process_launched=False,
        numerical_acceptance=False, full_model_correctness=False, performance_claim=False, production_authority=False)
    body = (json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(body) <= 16 << 20, 'bounded comparison report')
    out.mkdir(mode=0o700)
    with (out / 'complete.json').open('xb') as stream: stream.write(body)
    print(json.dumps(dict(complete=actual(out / 'complete.json')[0], comparable_rows=24,
        earliest_observable_divergence=comparison['earliest_observable_divergence'])), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])
