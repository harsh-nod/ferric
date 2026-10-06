"""Closed synthetic test child; launch only through the owned bounded supervisor."""

import hashlib
import json
from pathlib import Path
import stat
import sys
import types
import unittest


ROOT = Path(__file__).resolve().parent
FILES = {
    'lowering.py': (49545, '8748302163bf68ec42cd83e6a0e041c225a635da8fd481d842d3902b0c2091d3'),
    'test_lowering.py': (26873, '5d32c31fe27e36744d31ea035d42b26d7cda6a368cfdfc94b6d35711b8dc3e7c'),
    'test_normalization.py': (6507, '6d4d91e24fa483e81d99ae1dd89a629eb3b7a17aaef94e8f46958123c292465f'),
}
EXPECTED = (
    'test_lowering.AuditBindingTests.test_actual_previous_driver_generation_is_immutable',
    'test_lowering.AuditBindingTests.test_capacity_limits_and_cohorts_cannot_drift',
    'test_lowering.AuditBindingTests.test_deployed_body_must_match_final_not_earlier_product',
    'test_lowering.AuditBindingTests.test_driver_failure_controller_phase_or_full_suite_claim_refused',
    'test_lowering.AuditBindingTests.test_driver_readset_or_final_body_drift_refused',
    'test_lowering.AuditBindingTests.test_each_of_six_other_tools_stays_exact',
    'test_lowering.AuditBindingTests.test_earlier_generation_cannot_supply_census_provenance',
    'test_lowering.AuditBindingTests.test_failed_or_earlier_producer_refused',
    'test_lowering.AuditBindingTests.test_loader_input_readset_drift_refused',
    'test_lowering.AuditBindingTests.test_membership_lineage_and_census_are_required',
    'test_lowering.AuditBindingTests.test_new_driver_replay_and_metadata_only_scope_are_required',
    'test_lowering.AuditBindingTests.test_new_filepin_and_final_producer_join',
    'test_lowering.AuditBindingTests.test_old_audit_schema_or_missing_producer_replay_refused',
    'test_lowering.AuditBindingTests.test_pending_bindings_refuse_before_any_helper_or_effect',
    'test_lowering.AuditBindingTests.test_unreviewed_loader_or_producer_controller_refused',
    'test_normalization.NormalizationTests.test_command_selects_exact_profile_before_cargo_delimiter',
    'test_normalization.NormalizationTests.test_output_accepts_exact_seven_field_options',
    'test_normalization.NormalizationTests.test_output_refuses_flags_or_worker_policy_drift',
    'test_normalization.NormalizationTests.test_output_refuses_old_missing_extra_or_minimal_options',
)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def source_body(name):
    path = ROOT / name
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode),
            'aliased or nonordinary lowering-test source')
    require(before.st_size == FILES[name][0], 'lowering-test source extent differs')
    body = path.read_bytes()
    require(stamp(before) == stamp(path.lstat()), 'lowering-test source changed while reading')
    require((len(body), hashlib.sha256(body).hexdigest()) == FILES[name], 'lowering-test source pin differs')
    return body


def load_exact(name, body):
    require(name not in sys.modules, 'lowering-test module already present')
    module = types.ModuleType(name)
    module.__file__ = str(ROOT / (name + '.py'))
    sys.modules[name] = module
    # Execute the bytes just authenticated, without discovery or a second file read.
    exec(compile(body, module.__file__, 'exec'), module.__dict__)
    return module


def test_names(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from test_names(test)
        else:
            yield test.id()


class NamedResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.passing_names = []

    def addSuccess(self, test):
        self.passing_names.append(test.id())
        super().addSuccess(test)


def main():
    require(__debug__ and sys.flags.isolated and sys.flags.no_site and sys.dont_write_bytecode
            and len(sys.argv) == 1, 'owned leaf requires python3 -I -S -B with no arguments')
    bodies = {name: source_body(name) for name in FILES}
    load_exact('lowering', bodies['lowering.py'])
    modules = [load_exact(name, bodies[name + '.py'])
               for name in ('test_lowering', 'test_normalization')]
    suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(module)
                               for module in modules)
    names = tuple(sorted(test_names(suite)))
    require(names == EXPECTED and len(names) == len(set(names)) == 19, 'exact19 synthetic test inventory required')
    result = unittest.TextTestRunner(verbosity=2, resultclass=NamedResult).run(suite)
    unchanged = all(source_body(name) == body for name, body in bodies.items())
    passed = (result.wasSuccessful() and result.testsRun == 19
              and tuple(sorted(result.passing_names)) == EXPECTED
              and not result.skipped and not result.expectedFailures
              and not result.unexpectedSuccesses and unchanged)
    print(json.dumps(dict(
        schema='ferric-indexed-atomic-dead-cast-census-lowering-tests-v1', passed=passed,
        names=list(names), passing_names=sorted(result.passing_names), tests_run=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
        source_unchanged=unchanged,
        sources={name: dict(bytes=size, sha256=digest) for name, (size, digest) in FILES.items()},
        synthetic_lowering_only=True, compiler_qualification=False, gpu_execution=False,
    ), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
