"""Bind actual v5 receipts; retain/reap the extraction launcher's owned tree."""
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
HELPER_SHA = '67bcd967e3bca571ee600e6cf266b24abe8f5dd3bb51533155db525679e5e2db'
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
STREAM_CAP = 64 * 1024**2
FILE_CAP = 1024**3
DEADLINE = 600


def require(value, message):
    if not value:
        raise RuntimeError(message)


def identity(value):
    return value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns


def read_file(path, retain=False, maximum=FILE_CAP):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical file path')
    digest, size, data = hashlib.sha256(), 0, bytearray()
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= maximum, 'regular file bound')
        while True:
            block = stream.read(1024**2)
            if not block:
                break
            size += len(block)
            require(size <= maximum, 'growing file bound')
            digest.update(block)
            if retain:
                data.extend(block)
        after = os.fstat(stream.fileno())
    final = path.lstat()
    require(stat.S_ISREG(final.st_mode) and identity(before) == identity(after) == identity(final)
        and size == before.st_size, 'file changed during read')
    return dict(path=str(path), bytes=size, sha256=digest.hexdigest()), bytes(data)


def parse(data):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON field')
            result[key] = value
        return result
    return json.loads(data, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON constant'))


class Pins:
    def __init__(self):
        self.records = {}

    def read(self, path, expected=None, retain=False, maximum=FILE_CAP):
        record, data = read_file(path, retain, maximum)
        require(expected is None or record['sha256'] == expected, 'expected file SHA256')
        previous = self.records.setdefault(record['path'], record)
        require(previous == record, 'input changed between reads')
        return record, data

    def pin(self, path, expected=None):
        return self.read(path, expected)[0]

    def json(self, path, expected=None):
        record, data = self.read(path, expected, retain=True, maximum=STREAM_CAP)
        return parse(data), record

    def recheck(self):
        for record in self.records.values():
            require(read_file(record['path'])[0] == record, 'pinned input changed')


def load_module(pins, path, expected, name):
    _, data = pins.read(path, expected, retain=True, maximum=STREAM_CAP)
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(data, str(path), 'exec'), module.__dict__)
    return module


def package(pins, name, expected):
    directory = E / name
    manifest, _ = pins.json(directory / 'manifest.json', expected)
    seen = set()
    for row in manifest['files']:
        relative = Path(row['path'])
        require(not relative.is_absolute() and '..' not in relative.parts
            and relative.parts and row['path'] not in seen, 'package member path')
        seen.add(row['path'])
        require(pins.pin(directory / relative, row['sha256'])['bytes'] == row['bytes'],
            'package member bytes')
    return directory


def select_metadata(build, pins):
    require(build['passed'] is True and build['group'] == 'build', 'completed build cohort')
    candidates = [row for row in build['binaries']
        if row['target'] == 'finite_join_request_metadata_v1' and row['test'] is False]
    require(len(candidates) == 1, 'one actual non-test metadata helper')
    row = candidates[0]
    record = pins.pin(Path(row['path']), row['sha256'])
    require(record['bytes'] == row['bytes'], 'metadata helper size')
    return record


def stop_signal(number, _frame):
    raise RuntimeError('wrapper received signal ' + str(number))


def progress(value):
    try:
        print(json.dumps(value), flush=True)
    except OSError:
        pass


def run_coordinator(out, argv, cwd, environment, owned, save, deadline=DEADLINE):
    # This module creates exactly one child. The frozen helper requires an
    # initially childless single-threaded subreaper before admitting adoptees.
    owned.subreaper()
    tracker, child, outcome, reason = owned.OwnedProcesses(), None, None, None
    start = time.monotonic()
    handlers = {number: signal.getsignal(number) for number in (signal.SIGTERM, signal.SIGINT)}
    try:
        for number in handlers:
            signal.signal(number, stop_signal)
        with (out / 'stdout').open('xb') as stdout, (out / 'stderr').open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, start_new_session=True)
            tracker.attach(child)
            save(out / 'started.json', dict(parent=tracker.root, supervisor_pid=os.getpid()))
            progress(dict(status='RUNNING', pid=child.pid, logs=str(out)))
            while True:
                tracker.discover()
                require(time.monotonic() - start < deadline, 'coordinator deadline')
                require(max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size)
                    <= STREAM_CAP, 'coordinator stream cap')
                # Observe without reaping: the reserved leader PID and pidfds
                # remain valid while separately-sessioned tools are discovered.
                if os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT):
                    break
                time.sleep(.2)
    except BaseException as error:
        reason = type(error).__name__ + ': ' + str(error)
    finally:
        try:
            if child is not None:
                for number in handlers:
                    signal.signal(number, signal.SIG_IGN)
                try:
                    require(tracker.root is not None and owned.token(tracker.root) in tracker.records,
                        'parent pidfd initialization incomplete')
                    outcome = tracker.cleanup()
                except BaseException as error:
                    reason = reason or ('primary cleanup failed: ' + repr(error))
                    outcome = owned.emergency_reap(child, tracker.events)
                if outcome['cleanup_signalled']:
                    reason = reason or 'live owned processes required cleanup'
        finally:
            try:
                tracker.close_fds()
            finally:
                for number, handler in handlers.items():
                    signal.signal(number, handler)
    require(outcome is not None, reason or 'coordinator was not created')
    require(outcome['owned_groups_absent'] is True and outcome['owned_processes_reaped'] is True,
        'owned-tree cleanup incomplete')
    outcome.update(reason=reason, elapsed_seconds=time.monotonic() - start, deadline_seconds=deadline)
    return outcome


def main():
    require(not sys.flags.optimize and len(sys.argv) == 2
        and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'pipeline SHA256 argument')
    pins = Pins()
    p = load_module(pins, E / 'install_row_consumer_p225.py', HELPER_SHA, 'row_consumer')
    owned = load_module(pins, Path(__file__).resolve().with_name('frozen_owned.py'), OWNED_SHA, 'row_owned')
    p.n.setup()
    before = p.snapshot()
    source, _ = pins.json(E / 'row-native-tests-v225-v2/sources-after.json',
        '728804f859809b379c048298bd78ba2c296af882514a32296d56947db290060c')
    require(before == source, 'source snapshot')
    pipeline, pipeline_pin = pins.json(E / 'row-pipeline-v225-v5/complete.json', sys.argv[1])
    require(pipeline['passed'] is True and pipeline['gpu_execution'] is False, 'completed CPU pipeline')
    require(pipeline['stages'] == ['row-authentication-v225-v5', 'row-source-v225-v5',
        'row-replay-v225-v5', 'row-emit-v225-v5'], 'exact actual v5 stages')
    pins.json(E / 'row-pipeline-plan-v225-v5.json', pipeline['plan_sha256'])
    extraction = package(pins, 'p225-row-extraction-facts',
        '36fc8cd112554cb2206ad74469e50fb6faab5f4e29b6df800adc98b43c20a57b')
    inspector = package(pins, 'p225-row-artifact-inspection',
        '7ea2629b6d1c3fc1e24b6316f241aefc47400b5c362d1087a55d0365786ba493')
    test_pins = []
    for label, count, digest in [
        ('row-extraction-controller-tests-v225-v1', 8, '36fc8cd112554cb2206ad74469e50fb6faab5f4e29b6df800adc98b43c20a57b'),
        ('row-artifact-inspector-tests-v225-v1', 10, '7ea2629b6d1c3fc1e24b6316f241aefc47400b5c362d1087a55d0365786ba493')]:
        record, record_pin = pins.json(E / label / 'complete.json')
        require(record['passed'] is True and record['tests'] == count
            and record['manifest_sha256'] == digest, 'controller test cohort')
        test_pins.append(record_pin)
    build_label = pipeline['cohorts']['build']
    require(type(build_label) is str and re.fullmatch('row-build-v225-v[1-9][0-9]*', build_label),
        'build cohort label')
    build, build_pin = pins.json(E / build_label / 'complete.json')
    metadata = select_metadata(build, pins)
    llvm = Path('/opt/rocm-7.3.0/lib/llvm/bin')
    facts = dict(schema='ferric-p225-artifact-inspection-inputs-v1',
        source_complete=pins.pin(E / 'row-source-v225-v5/complete.json'),
        replay_complete=pins.pin(E / 'row-replay-v225-v5/complete.json'),
        emission_complete=pins.pin(E / 'row-emit-v225-v5/complete.json'), build_complete=build_pin,
        provider_manifest=pins.pin(p.w.R / 'evidence/wave-worker-probe-fix-v195/build/fe2o3-gfx950-ocml-provider.txt'),
        tools=dict(metadata_helper=metadata,
            readelf=pins.pin(llvm / 'llvm-readobj', '8cb6079bc349196a3f3af7589912ca48f716194970ccd0e6c6a36b09e0b21259'),
            objdump=pins.pin(llvm / 'llvm-objdump', 'be832bf5ddf82b3feb8a6e24f453e2602bd01a406f9f5bde47b225c4989d1a83')),
        controllers={name: pins.pin(inspector / name) for name in ('inspect_artifact.py', 'policy.py', 'common.py')})
    plan = dict(schema='ferric-p225-extraction-facts-inputs-v1', output_label='row-retained-facts-v225-v1',
        facts_output_label='row-artifact-facts-v225-v1',
        orchestration_common=pins.pin(E / 'row-orchestration-v225/common.py'),
        extractor_source=pins.pin(extraction / 'extract_retained.rs'), inspector_inputs=facts)
    out = E / 'row-retained-facts-driver-v225-v2'
    out.mkdir()
    p.b.save(out / 'inputs.json', plan)
    argv = ['/usr/bin/python3', '-B', str(extraction / 'run.py'),
        str(out / 'inputs.json'), pins.pin(out / 'inputs.json')['sha256']]
    p.b.save(out / 'command.json', dict(argv=argv, pipeline=pipeline_pin, tests=test_pins,
        controller=pins.pin(Path(__file__).resolve()), owned_helper=pins.pin(Path(owned.__file__)),
        deadline_seconds=DEADLINE, gpu_execution=False))
    pins.recheck()
    require(p.snapshot() == before, 'source changed before launch')
    outcome = run_coordinator(out, argv, p.w.R / 'fe2o3', p.n.environment(), owned, p.b.save)
    post_error = None
    try:
        pins.recheck()
        require(p.snapshot() == before, 'source changed after launch')
    except BaseException as error:
        post_error = type(error).__name__ + ': ' + str(error)
    stdout, stderr = read_file(out / 'stdout', maximum=STREAM_CAP)[0], read_file(out / 'stderr', maximum=STREAM_CAP)[0]
    passed = outcome['exit_code'] == 0 and outcome['reason'] is None and post_error is None
    p.b.save(out / 'result.json', dict(schema='ferric-p225-row-facts-driver-v2', passed=passed,
        **outcome, postcheck_error=post_error, inputs_unchanged=post_error is None,
        stdout=stdout, stderr=stderr, input_pins=pins.records,
        gpu_execution=False, numerical_acceptance=False, production_authority=False))
    progress(dict(passed=passed, result=str(out / 'result.json')))
    raise SystemExit(0 if passed else 1)


if __name__ == '__main__':
    main()
