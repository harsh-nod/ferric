"""CPU-only comparison of actual completed framework and Ferric observations."""
import hashlib
import json
import os
from pathlib import Path
import resource
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-layer0-genuine-comparison-v1'
OUT = E / 'layer0-genuine-comparison-v228-v1'
NATIVE = E / 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1'
TRANSPORT = E / 'layer0-comparison-native-inputs-v228-v1'
MAPPING = {
    str(NATIVE / 'complete.json'): TRANSPORT / 'complete.json',
    str(NATIVE / 'native/observation-0.bin'): TRANSPORT / 'observation-0.bin',
    str(E / 'prefix-down2-clock-tf4-inputs-v228-v1/request.json'): TRANSPORT / 'request.json',
}
SOURCES = {
    'compare.py': '1598e22a3460a9ed2c350fb5d2b5fec6b8c37648b53bb2f1fc714c1fb3506d3f',
    'test_compare.py': 'd8b1071ddbb250965baafc605076bd76cb0bb68c1d30fff2cc0df57182c0c475',
    'diagnostics.py': '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf',
}
CONSUMED = {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(path, expected=None):
    require(path.resolve(strict=True) == path and path.is_file()
            and path.stat().st_size <= 16 << 20, 'canonical bounded input')
    raw = path.read_bytes()
    record = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(expected is None or record['sha256'] == expected, 'actual input digest')
    return record


def read(record):
    path = MAPPING.get(record['path'], Path(record['path']))
    actual = pin(path, record['sha256'])
    require(actual['bytes'] == record['bytes'], 'actual transported extent')
    raw = path.read_bytes()
    require(len(raw) == actual['bytes'] and hashlib.sha256(raw).hexdigest() == actual['sha256'], 'stable read')
    row = dict(original=record, retained=actual)
    require(CONSUMED.setdefault(record['path'], row) == row, 'consistent transported identity')
    return raw


def document(record):
    return json.loads(read(record))


def natural(record, utility=False):
    value = document(record)
    require(value['exit_code'] == 0 and value['failure'] is None
            and value['observed_utility_tree_absent' if utility else 'group_absent'] is True,
            'naturally successful reaped child')
    for name in ('command', 'stdout', 'stderr'):
        read(value[name])
    if 'started' in value:
        read(value['started'])


def main():
    require(len(sys.argv) == 2 and not sys.flags.optimize and sys.dont_write_bytecode,
            'ordinary -B invocation with actual PURE_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded ASROCK CPU identity')
    require(all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME'))
            and all(os.environ.get(k) == '' for k in
                    ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'CPU-only clean environment')
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (limit, limit))
    own = pin(Path(__file__).resolve(strict=True))
    read(own)
    source_pins = {name: pin(P / name, digest) for name, digest in SOURCES.items()}
    for row in source_pins.values():
        read(row)
    pure_pin = pin(E / 'layer0-genuine-comparison-pure-v228-v1/complete.json', sys.argv[1])
    pure = document(pure_pin)
    require(pure['passed'] is True and pure['tests'] == 14
            and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True, 'actual comparison tests')
    require(document(pure['sources_before']) == document(pure['sources_after']) == source_pins,
            'actual tested sources unchanged')
    read(pure['transcript'])
    module = types.ModuleType('root_genuine_comparison')
    module.__file__ = source_pins['compare.py']['path']
    exec(compile(read(source_pins['compare.py']), module.__file__, 'exec'), module.__dict__)
    diagnostics = module.load_diagnostics(read, source_pins['diagnostics.py'])
    outer_pin = pin(E / 'layer0-framework-launch-v228-v1/complete.json',
                    '46fd9acbca798f05bc737a651e6fba54e65c78688e2d425a8287eef402065edc')
    outer = document(outer_pin)
    require(outer['passed'] is True and outer['failures'] == [] and outer['native_attempts'] == 1
            and outer['retries'] == 0 and len(outer['before_audits']) == len(outer['after_audits']) == 3,
            'actual closed framework experiment')
    for name in ('cpu_result', 'inspection_result', 'execution_result'):
        natural(outer[name])
    for audit in outer['before_audits'] + outer['after_audits']:
        for name in ('live_path_result', 'live_sha_result', 'smi_result'):
            natural(audit[name], name != 'smi_result')
        require(audit['sample']['gpu_busy_percent'] == 0, 'actual idle audit')
    original_pin = pin(E / 'framework-rearm-v224-v1/reference/reference.json',
        '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416')
    native_pin = dict(path=str(NATIVE / 'complete.json'), bytes=963187,
        sha256='00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073')
    result, _ = module.compare_retained(outer['reference'], original_pin, native_pin, read, diagnostics)
    for value in list(CONSUMED.values()):
        require(pin(Path(value['retained']['path'])) == value['retained'], 'all consumed bytes unchanged')
    OUT.mkdir(mode=0o700)
    report = dict(schema='ferric-p228-layer0-genuine-comparison-observation-v1', completed=True,
        controller=own, comparison=result, source_pins=source_pins, pure=pure_pin,
        framework_outer=outer_pin, framework_owned_leaf_results_checked=21,
        consumed=list(CONSUMED.values()), source_postchecks_passed=True,
        all_transitive_inputs_replayed=False, gpu_execution=False, numerical_acceptance=False,
        full_model_correctness=False, performance_measured=False, production_authority=False)
    path = OUT / 'complete.json'
    with path.open('xb') as stream:
        stream.write((json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + '\n').encode())
    print(json.dumps(dict(complete=pin(path), comparison=result)), flush=True)


if __name__ == '__main__':
    main()
