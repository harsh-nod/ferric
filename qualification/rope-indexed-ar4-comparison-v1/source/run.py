"""Bounded MI350 CPU-only retained-data comparison; no subprocess or model imports."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-rope-indexed-ar4-comparison-v1'
GPU_PACKAGE = E / 'p228-rope-indexed-ar4-gpu-v1'
MEMBERS = {'run.py', 'comparison.py', 'diagnostics.py', 'test_comparison.py', 'README.md'}
HELPERS = {
    'stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'decode_validation.py': '0eb96d4ac5e10e7f6ac10b018692c56f969286949681be336cb6040d56f01422',
}
DIAGNOSTICS_SHA = '645f11391b2255b7b93a8e7f0372114ad9700a0037e718bb48677d2234b926e7'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON field')
            value[key] = item
        return value
    def invalid(_):
        raise ValueError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def actual(path, expected=None):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical ordinary data path')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_size <= 4 << 20,
            'bounded ordinary data file')
        raw = stream.read((4 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_nlink)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
        'stable complete data read')
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(expected is None or pin['sha256'] == expected, 'caller-authenticated actual SHA')
    return pin, raw


def filepin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}
        and type(value['path']) is str and Path(value['path']).is_absolute()
        and type(value['bytes']) is int and 0 <= value['bytes'] <= 4 << 20
        and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'closed bounded FilePin')


class Reader:
    def __init__(self, transport):
        require(type(transport) is dict and len(transport) <= 64, 'explicit bounded original-path transport')
        for original, retained in transport.items():
            require(type(original) is str and Path(original).is_absolute(), 'absolute original identity')
            filepin(retained)
        self.transport, self.consumed = transport, {}

    def read(self, original):
        filepin(original)
        retained = self.transport.get(original['path'], original)
        require((retained['bytes'], retained['sha256']) == (original['bytes'], original['sha256']),
            'transport preserves original extent and SHA')
        observed, raw = actual(retained['path'], retained['sha256'])
        require(observed == retained, 'retained extent matches caller pin')
        row = dict(original=original, retained=retained)
        require(original['path'] not in self.consumed or self.consumed[original['path']] == row,
            'one identity per consumed original path')
        self.consumed[original['path']] = row
        require(len(self.consumed) <= 128 and sum(v['original']['bytes'] for v in self.consumed.values()) <= 64 << 20,
            'bounded complete input closure')
        return raw

    def recheck(self):
        for row in list(self.consumed.values()):
            self.read(row['original'])


def module(name, path, raw, aliases=None):
    missing = object()
    saved = {key: sys.modules.get(key, missing) for key in aliases or {}}
    value = types.ModuleType(name); value.__file__ = str(path)
    try:
        sys.modules.update(aliases or {})
        exec(compile(raw, str(path), 'exec'), value.__dict__)
    finally:
        for key, previous in saved.items():
            if previous is missing:
                sys.modules.pop(key, None)
            else:
                sys.modules[key] = previous
    return value


def enforce():
    require(not sys.flags.optimize and sys.dont_write_bytecode
        and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary isolated Python -B')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'MI350 CPU8/9 nice10, no native execution')
    require(all(os.environ.get(key) == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES',
        'CUDA_VISIBLE_DEVICES')), 'GPUs hidden from CPU-only process')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 300),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = min(v for v in (cap, soft, hard) if v != resource.RLIM_INFINITY)
        resource.setrlimit(kind, (bound, bound))
    def expired(_signal, _frame):
        raise TimeoutError('bounded 360-second CPU comparison')
    signal.signal(signal.SIGALRM, expired); signal.alarm(360)
    os.umask(0o077)


def main(args):
    require(len(args) == 4, 'INPUTS_PATH INPUTS_SHA PACKAGE_MANIFEST_SHA FRESH_OUTPUT_LABEL')
    enforce()
    require(Path(__file__).resolve(strict=True) == PACKAGE / 'run.py', 'selected controller path')
    input_pin, raw = actual(args[0], args[1]); config = parse(raw)
    require(set(config) == {'schema', 'native_complete', 'transport'}
        and config['schema'] == 'ferric-p228-rope-indexed-ar4-comparison-inputs-v1', 'closed comparison input')
    reader = Reader(config['transport']); reader.read(input_pin)
    filepin(config['native_complete'])
    native_path = Path(config['native_complete']['path'])
    require(native_path.parent.parent == E and native_path.name == 'complete.json'
        and re.fullmatch(r'prefix-rope-indexed-ar4-gpu-v228-v[1-9][0-9]{0,8}', native_path.parent.name),
        'actual future native completion, never a guessed or relabelled predecessor')
    require(re.fullmatch(r'rope-indexed-ar4-comparison-v228-v[1-9][0-9]{0,8}', args[3]), 'fresh result namespace')
    out = E / args[3]
    require(not os.path.lexists(out) and out.parent.resolve(strict=True) == E, 'exclusive output')
    manifest_pin, raw = actual(PACKAGE / 'manifest.json', args[2]); reader.read(manifest_pin)
    manifest = parse(raw)
    require(manifest['schema'] == 'ferric-p228-rope-indexed-ar4-comparison-package-v1'
        and len(manifest['files']) == len(MEMBERS) and {row['path'] for row in manifest['files']} == MEMBERS,
        'closed actual five-body package')
    sources, bodies = {}, {}
    for row in manifest['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'closed source row')
        pin = dict(row, path=str(PACKAGE / row['path']))
        sources[row['path']] = pin; bodies[row['path']] = reader.read(pin)
    require(sources['diagnostics.py']['sha256'] == DIAGNOSTICS_SHA, 'unchanged genuine AR4 metric implementation')
    for name, sha in HELPERS.items():
        pin, _ = actual(GPU_PACKAGE / name, sha)
        sources['validator/' + name] = pin; bodies[name] = reader.read(pin)
    comparison = module('retained_rope_ar4_comparison', PACKAGE / 'comparison.py', bodies['comparison.py'])
    diagnostics = module('retained_ar4_diagnostics', PACKAGE / 'diagnostics.py', bodies['diagnostics.py'])
    stage = module('retained_ar4_stage', GPU_PACKAGE / 'stage_core.py', bodies['stage_core.py'])
    smoke = module('retained_ar4_smoke', GPU_PACKAGE / 'smoke_validation.py', bodies['smoke_validation.py'], {'stage_core': stage})
    validator = module('retained_ar4_validator', GPU_PACKAGE / 'decode_validation.py', bodies['decode_validation.py'],
        {'stage_core': stage, 'smoke_validation': smoke})
    out.mkdir(mode=0o700)
    result, errors = None, []
    try:
        result = comparison.compare_retained(reader.read, config['native_complete'], validator, diagnostics)
        require(set(reader.transport) <= set(reader.consumed), 'every declared transport body actually consumed')
    except Exception as error:
        errors.append(type(error).__name__ + ': ' + str(error))
    postchecked = False
    try:
        reader.recheck(); postchecked = True
    except Exception as error:
        errors.append('postcheck: ' + type(error).__name__ + ': ' + str(error))
    value = dict(schema='ferric-p228-rope-indexed-ar4-comparison-observation-v1',
        completed=not errors and result is not None, errors=errors, inputs=input_pin,
        controller=sources['run.py'], source_manifest=manifest_pin, sources=sources,
        consumed=list(reader.consumed.values()), input_source_postchecks_passed=postchecked,
        comparison=result, data_only=True, gpu_execution=False, model_execution=False,
        native_or_framework_controllers_imported=False, full_owner_audits_replayed=False,
        numerical_acceptance=False, acceptance_threshold=None, full_model_correctness=False,
        sustained_2048_256=False, performance_claim=False, production_authority=False)
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(raw) <= 16 << 20, 'bounded actual result')
    path = out / ('complete.json' if value['completed'] else 'failure.json')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(completed=value['completed'], result=actual(path)[0],
        candidate_tensor_rows=result['candidate_tensor_rows'] if result else None), sort_keys=True), flush=True)
    signal.alarm(0)
    return 0 if value['completed'] else 1


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
