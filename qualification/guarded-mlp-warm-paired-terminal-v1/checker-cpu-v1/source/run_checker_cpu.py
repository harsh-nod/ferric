"""Run twenty synthetic warm-terminal tests; no subprocesses or GPU use."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import time
import types
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-warm-paired-terminal-checker-cpu-v228-v1'
SOURCE_ROOT = E / 'guarded-mlp-warm-paired-terminal-checker-source-v228-v1'
SOURCES = {'validate_observation': ('validate_observation.py', 17983, '7224877439ec966254d7604317f1f47b5d6345af698e47e332f6cb36c3abc19b'), 'run_model_gpu': ('run_model_gpu.py', 43046, 'a996fbca701ed968a49f4dc88a62c808be76b150311d79e7dd4d098ca804a13b'), 'test_census': ('test_census.py', 4375, 'd4f19ae2f4bcf9f68464a7b8951b82368183cec6db998a0ed55449d2349e7f64'), 'test_comparison': ('test_comparison.py', 3753, 'daead3eef98b1368b705b17a6894c2c060ef69ca154788fbd8f01cbf9e888093'), 'test_terminal': ('test_terminal.py', 8856, '65cd07e2542b67aa8bcee20b17b9bc8e9c70a3a7a8940ab9f27576cca0f1ec47')}
TEST_NAMES = ('test_census.CensusTests.test_actual_width_ids_and_closed_positive_record', 'test_census.CensusTests.test_bounds_and_nonfinite_json', 'test_census.CensusTests.test_close_and_claim_flags_are_strict', 'test_census.CensusTests.test_fresh_schema_teacher_forcing_or_changed_bootstrap_refuse', 'test_census.CensusTests.test_growth_rank_swap_or_non_integer_counts', 'test_census.CensusTests.test_missing_extra_or_duplicate_fields', 'test_census.CensusTests.test_missing_extra_or_reordered_samples', 'test_census.CensusTests.test_stale_profile_registration_session_and_devices', 'test_comparison.ComparisonTests.test_all_four_equal_payloads_and_histories', 'test_comparison.ComparisonTests.test_history_difference_is_not_hidden_by_equal_payloads', 'test_comparison.ComparisonTests.test_one_changed_byte_in_each_frame_is_reported', 'test_comparison.ComparisonTests.test_tampered_hash_or_wrong_extent_is_refused', 'test_terminal.TerminalTests.test_case_exception_restores_outer_handlers_and_remaining_timer', 'test_terminal.TerminalTests.test_control_requires_same_elf_policy_success_and_distinct_session', 'test_terminal.TerminalTests.test_distinct_profile_and_nested_census_preserve_legacy', 'test_terminal.TerminalTests.test_nested_close_plateau_and_cadence_flags_are_strict', 'test_terminal.TerminalTests.test_only_two_warm_generations_may_report_paired_dispatches', 'test_terminal.TerminalTests.test_profile_schema_and_unknown_fields_cannot_cross_modes', 'test_terminal.TerminalTests.test_segment_host_fields_use_fixed_u64_offsets_not_gpu_time', 'test_terminal.TerminalTests.test_tested_runner_binding_normalization_is_narrow_and_literal')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def read(path):
    require(path.resolve(strict=True) == path, 'canonical input')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                and before.st_uid == os.getuid() and before.st_size <= 1 << 20, 'ordinary bounded owned input')
        raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input changed while reading')
    return raw


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


class BoundedText(io.StringIO):
    def write(self, value):
        require(self.tell() + len(value) <= 65536, 'bounded test output')
        return super().write(value)


def save(name, raw):
    require(len(raw) <= 65536, 'bounded output')
    with (ROOT / name).open('xb') as stream:
        stream.write(raw)
    return dict(path=str(ROOT / name), **pin(raw))


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'isolated no-bytecode invocation')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'selected MI350 host and account')
    require(E.resolve(strict=True) == E and not os.path.lexists(ROOT), 'fresh comparison CPU namespace')
    for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        require(os.environ.get(key, '') == '', 'GPU-hidden synthetic environment')
        os.environ[key] = ''
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    os.nice(10)
    for kind, limit in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 30),
                        (resource.RLIMIT_FSIZE, 65536), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([limit] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    def interrupted(number, _frame):
        raise RuntimeError('comparison CPU signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(number, interrupted)
    started = time.monotonic()
    deadline = started + 30
    def wall_guard():
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'absolute comparison CPU wall deadline')
        signal.setitimer(signal.ITIMER_REAL, remaining)
    wall_guard()
    require(all(type(size) is int and size > 0 and type(sha) is str and len(sha) == 64
                for _, size, sha in SOURCES.values()), 'bind actual reviewed runner and test source pins first')
    ROOT.mkdir(mode=0o700)
    sources, modules, result, failure = {}, {}, None, None
    stdout, stderr = BoundedText(), BoundedText()
    controller = read(Path(__file__).resolve())
    try:
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            for name, (filename, size, sha) in SOURCES.items():
                path = SOURCE_ROOT / filename
                raw = read(path)
                require(pin(raw) == dict(bytes=size, sha256=sha), 'reviewed source pin')
                sources[str(path)] = pin(raw)
                module = types.ModuleType(name)
                module.__file__ = str(path)
                require(name not in sys.modules, 'fresh test module')
                sys.modules[name] = modules[name] = module
                exec(compile(raw, str(path), 'exec'), module.__dict__)
            suite = unittest.TestSuite()
            for module_name in ('test_census', 'test_comparison', 'test_terminal'):
                suite.addTests(unittest.defaultTestLoader.loadTestsFromModule(modules[module_name]))
            def flatten(value):
                for case in value:
                    if isinstance(case, unittest.TestSuite):
                        yield from flatten(case)
                    else:
                        yield case.id()
            names = sorted(flatten(suite))
            require(names == list(TEST_NAMES) and suite.countTestCases() == 20,
                    'exact twenty named synthetic tests')
            result = unittest.TextTestRunner(stream=stderr, verbosity=2).run(suite)
            require(result.testsRun == 20 and result.wasSuccessful() and not result.skipped
                    and not result.expectedFailures and not result.unexpectedSuccesses, 'all twenty tests pass')
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    # Do not continue retention after a swallowed unittest timeout.
    wall_guard()
    post_errors = []
    for path, expected in sources.items():
        wall_guard()
        try:
            require(pin(read(Path(path))) == expected, 'source drift')
        except BaseException as error:
            post_errors.append(str(error))
    wall_guard()
    try:
        require(read(Path(__file__).resolve()) == controller, 'controller drift')
    except BaseException as error:
        post_errors.append(str(error))
    failure = failure or ('postcheck failed' if post_errors else None)
    wall_guard()
    record = dict(schema='ferric-guarded-mlp-warm-paired-terminal-checker-cpu-v1', passed=failure is None,
        failure=failure, postcheck_errors=post_errors, controller=pin(controller), sources=sources,
        expected_test_names=list(TEST_NAMES),
        tests_run=result.testsRun if result is not None else 0,
        failures=len(result.failures) if result is not None else None,
        errors=len(result.errors) if result is not None else None,
        stdout=save('stdout', stdout.getvalue().encode()), stderr=save('stderr', stderr.getvalue().encode()),
        elapsed_seconds=time.monotonic() - started, synthetic_evidence_only=True,
        subprocesses_started=0, gpu_execution=False, numerical_acceptance=False, performance_claim=False)
    raw = (json.dumps(record, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    wall_guard()
    receipt = save('complete.json' if failure is None else 'failed.json', raw)
    wall_guard()
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, tests_run=record['tests_run'], receipt=receipt)))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
