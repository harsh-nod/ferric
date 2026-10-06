"""Recheck immutable original TF4 data; never rerun or relabel its failed controller."""
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ORIGINAL = E / 'guarded-mlp-model-gpu-v228-v2'
CASE = ORIGINAL / 'tf4'
OUT = E / 'guarded-mlp-model-tf4-revalidation-v228-v2'
FAILED_SHA = '8ced1ef7f167348fbcb68ce56cca8fc8e2238273ebe18fd737127563779871ce'
RUNNER_SHA = '865d00d994a7bcb2ff62f3bef6a33ee133fad03766f611c4b6833f11c96d0c09'
VALIDATOR_SHA = '367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055'
AUDIT_SHA = 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'
ANNOUNCEMENT_SHA = '96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80'
PLAN_SHA = '51f8a860efe329940e6fc825121b91f6b2d1dbdd382b2f7cb231854a71cceba8'
PARSER_CPU = dict(path=str(E / 'guarded-mlp-model-announcement-cpu-v228-v1/evidence/complete.json'),
    bytes=8872, sha256='170ba440cdf33cd88fc55c2ddff709624fd819d3d461d54daad239a719efdf5a')
INPUTS = {}
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def read(path, expected=None, retain=True):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical revalidation input')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1 and before.st_size <= 64 << 20,
            'ordinary bounded revalidation input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_uid, s.st_gid, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    digest = hashlib.sha256(); parts = []
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(before) == stamp(os.fstat(stream.fileno())), 'input changed at open')
        while True:
            require(time.monotonic() < DEADLINE, 'revalidation whole deadline')
            chunk = stream.read(1 << 20)
            if not chunk: break
            digest.update(chunk)
            if retain: parts.append(chunk)
        require(stamp(before) == stamp(os.fstat(stream.fileno())) == stamp(path.lstat()), 'input changed during read')
    pin = dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())
    require(expected is None or pin == expected, 'revalidation input pin differs')
    require(str(path) not in INPUTS or INPUTS[str(path)] == pin, 'input drift')
    INPUTS[str(path)] = pin
    require(len(INPUTS) <= 256 and sum(p['bytes'] for p in INPUTS.values()) <= 128 << 20,
            'closed revalidation input extent')
    return (b''.join(parts) if retain else None), pin


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key'); value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda v: (_ for _ in ()).throw(ValueError(v)))


def doc(path, expected=None):
    return parse(read(path, expected)[0])


def load(path, sha, name):
    raw, pin = read(path)
    require(pin['sha256'] == sha, 'authenticated data helper')
    module = types.ModuleType(name); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def interrupted(number, _frame):
    raise RuntimeError('revalidation signal ' + str(number))


def main():
    global DEADLINE
    started_at = time.monotonic(); DEADLINE = started_at + 120
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B revalidate_tf4.py only')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged qualification host')
    require(OUT.parent.resolve(strict=True) == OUT.parent and not os.path.lexists(OUT), 'fresh separate revalidation output')
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0); require(priority in (0, 10), 'nice level')
    if priority == 0: os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 4 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        cap = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    handlers = {n: signal.getsignal(n) for n in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)}
    for number in handlers: signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 120)
    OUT.mkdir(mode=0o700)
    error = None; checked = None; lineage = None; original_pin = None
    try:
        read(Path(__file__).resolve())
        guard = load(Path(__file__).resolve().parent / 'guarded_announcement.py', ANNOUNCEMENT_SHA, 'guarded_marker')
        validator = load(ORIGINAL / 'validate_observation.py', VALIDATOR_SHA, 'original_validator')
        original_runner = load(ORIGINAL / 'run_model_gpu.py', RUNNER_SHA, 'original_runner')
        audit = load(ORIGINAL / 'library_audit.py', AUDIT_SHA, 'original_read_helper')
        audit.HARD_DEADLINE = DEADLINE
        cpu = doc(PARSER_CPU['path'], PARSER_CPU)
        require(cpu['schema'] == 'ferric-guarded-mlp-model-announcement-cpu-v1'
                and cpu['passed'] is True and cpu['failure'] is None and cpu['postcheck_errors'] == []
                and cpu['source_unchanged'] is True and cpu['sources_before'] == cpu['sources_after']
                and cpu['tests']['passed'] == 6 and cpu['tests']['failed'] == cpu['tests']['errors'] == cpu['tests']['skipped'] == 0
                and cpu['sources_before']['guarded_announcement.py']['sha256'] == ANNOUNCEMENT_SHA
                and len(cpu['phases']) == 1 and cpu['gpu_execution'] is False, 'actual six-test parser qualification')
        audit.natural(cpu['phases'][0])
        for pin in [*cpu['raw'].values(), *cpu['sources_before'].values()]: read(pin['path'], pin, retain=False)
        raw, original_pin = read(CASE / 'failed.json')
        require(original_pin['bytes'] == 91510 and original_pin['sha256'] == FAILED_SHA, 'exact original failed terminal')
        original = parse(raw)
        require(original['schema'] == 'ferric-guarded-mlp-model-gpu-v1' and original['passed'] is False
                and original['errors'] == ['RuntimeError: one actual worker announcement']
                and original['mode'] == 'tf4' and original['native_attempts'] == 1 and original['retries'] == 0,
                'only the observed parser failure is eligible')
        plan_raw, plan_pin = read(ORIGINAL / 'tf4-input.json', original['plan'])
        require(plan_pin['sha256'] == PLAN_SHA, 'actual original plan')
        plan = parse(plan_raw); request = doc(ORIGINAL / 'tf4-request.json', plan['request'])
        require(original_runner.cpu_admission(plan, audit) == original['admission'], 'actual CPU/ELF ancestry')
        for path, pin in original['readset'].items():
            require(path == pin['path'], 'original readset path'); read(path, pin, retain=False)
        labels = ['parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd']
        labels += ['before-' + str(i) for i in range(3)] + ['parent'] + ['after-' + str(i) for i in range(3)]
        require([r['label'] for r in original['phases']] == labels, 'exact eleven original leaves')
        expected = {label + '/' + name for label in labels
                    for name in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
        expected |= {'initial-topology.json'} | {label + '-topology.json' for label in labels if label.startswith(('before-', 'after-'))}
        native_names = {'complete.json', 'child-stderr.bin'} | {f'{stem}-{i}.{suffix}' for i in range(4)
                       for stem, suffix in [('request', 'json'), ('control', 'bin'), ('observation', 'bin')]}
        expected |= {'native/' + name for name in native_names}
        require(len(expected) == 76 and set(original['raw']) == expected, 'exact original raw roster')
        actual = set()
        for parent, dirs, files in os.walk(CASE, followlinks=False):
            for name in dirs + files:
                p = Path(parent) / name; mode = p.lstat().st_mode
                require(p.resolve(strict=True) == p and (stat.S_ISDIR(mode) or stat.S_ISREG(mode)), 'ordinary original tree')
            actual.update(str((Path(parent) / name).relative_to(CASE)) for name in files)
        require(actual == expected | {'failed.json'}, 'immutable original case closure')
        for name, pin in original['raw'].items():
            require(pin['path'] == str(CASE / name), 'raw path confinement'); read(CASE / name, pin, retain=False)
        for phase in original['phases']:
            label = phase['label']; original_runner.successful(phase)
            require(doc(CASE / label / 'result.json') == {k: v for k, v in phase.items() if k != 'label'}, 'original phase/body join')
            for key, filename in [('command', 'command.json'), ('started', 'started.json'), ('stdout', 'stdout'), ('stderr', 'stderr')]:
                require(phase[key] == original['raw'][label + '/' + filename], 'original phase pin join')
        initial = doc(CASE / 'initial-topology.json'); original_runner.validate_topology(initial)
        require(initial == original['platform'], 'original admitted platform')
        for label in labels:
            if label.startswith(('before-', 'after-')):
                require(doc(CASE / (label + '-topology.json')) == initial, 'recorded topology drift')
                audit.idle(read(CASE / label / 'stdout')[0])
        summary = read(CASE / 'native/complete.json')[0]
        require(read(CASE / 'parent/stdout')[0] == read(CASE / 'native/complete.json')[0], 'exact parent summary bytes')
        def native_body(pin):
            path = Path(pin['path'])
            require(path.parent == CASE / 'native' and path.name in native_names, 'native retained file scope')
            return read(path, pin)[0]
        checked = validator.validate(summary, request, CASE / 'native', native_body)
        require(checked == original['observation'], 'same independently checked TF4 observation')
        native = next(p for p in original['phases'] if p['label'] == 'parent')
        command = doc(CASE / 'parent/command.json')
        require(command['argv'] == [plan['parent']['path'], '--request', plan['request']['path'],
                '--observe-guarded-mlp-decode', '--allow-unauthenticated-machine-code']
                and command['env'] == original_runner.ENV and command['cwd'] == str(E), 'original native recipe')
        lineage = guard.validate_lineage(read(CASE / 'parent/stderr')[0], native,
                                        doc(CASE / 'parent/started.json'), checked['child_pid'])
        for pin in audit.INPUTS.values(): read(pin['path'], pin, retain=False)
        for pin in list(INPUTS.values()): read(pin['path'], pin, retain=False)
    except BaseException as caught:
        error = type(caught).__name__ + ': ' + str(caught)
    require(time.monotonic() < DEADLINE, 'deadline expired; no accepted revalidation')
    result = dict(schema='ferric-guarded-mlp-model-tf4-data-revalidation-v1', passed=error is None,
        error=error, original_terminal=original_pin, original_controller_passed=False,
        parser_cpu=PARSER_CPU,
        original_failure_preserved=True, native_rerun=False, gpu_execution=False,
        actual_original_observation_revalidated=error is None, observation=checked, owned_lineage=lineage,
        inputs=INPUTS, input_posthashes_complete=error is None, original_raw_files=76 if error is None else None,
        original_natural_phases=11 if error is None else None, original_idle_snapshots=6 if error is None else None,
        elapsed_seconds=time.monotonic() - started_at, numerical_acceptance=False,
        full_model_acceptance=False, independent_full_model_reference=False,
        production_authority=False, performance_claim=False, full_long_workload=False)
    output = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(output) <= 4 << 20, 'revalidation terminal bound')
    with (OUT / ('complete.json' if error is None else 'failed.json')).open('xb') as stream: stream.write(output)
    print(json.dumps(dict(passed=error is None, error=error, native_rerun=False), sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items(): signal.signal(number, handler)
    return 0 if error is None else 1


if __name__ == '__main__':
    raise SystemExit(main())

