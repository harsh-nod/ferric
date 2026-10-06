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
    'audit_tools.py': (28549, 'ee7ef8a28cb7742302d5b76567c061eaa9489ecf2cf6b94af796794280d68004'),
    'test_audit_tools.py': (9140, 'e984676e6b8b4b072e36208acc1865142137b57208141253b83ff901190a0d57'),
}
EXPECTED = (
    'test_audit_tools.ProducerAdmissionTests.test_final_two_products_only_and_rlib_metadata',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_any_of_five_legacy_tool_changes',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_controller_helper_or_proposal_generation_drift',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_extra_tools_or_old_backend_replacement',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_failed_incomplete_or_diagnostic_producer',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_missing_or_changed_source_map',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_missing_rlib_or_test_product',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_old_or_failed_final_cargo_observation',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_phase_or_test_census_failure',
    'test_audit_tools.ProducerAdmissionTests.test_rejects_product_outside_target_and_wrong_role',
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
            'aliased or nonordinary loader-test source')
    require(before.st_size == FILES[name][0], 'loader-test source extent differs')
    body = path.read_bytes()
    require(stamp(before) == stamp(path.lstat()), 'loader-test source changed while reading')
    require((len(body), hashlib.sha256(body).hexdigest()) == FILES[name], 'loader-test source pin differs')
    return body


def load_exact(name, body):
    require(name not in sys.modules, 'loader-test module already present')
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
    load_exact('audit_tools', bodies['audit_tools.py'])
    module = load_exact('test_audit_tools', bodies['test_audit_tools.py'])
    suite = unittest.defaultTestLoader.loadTestsFromModule(module)
    names = tuple(sorted(test_names(suite)))
    require(names == EXPECTED and len(names) == len(set(names)) == 10, 'exact10 synthetic test inventory required')
    result = unittest.TextTestRunner(verbosity=2, resultclass=NamedResult).run(suite)
    unchanged = all(source_body(name) == body for name, body in bodies.items())
    passed = (result.wasSuccessful() and result.testsRun == 10
              and tuple(sorted(result.passing_names)) == EXPECTED
              and not result.skipped and not result.expectedFailures
              and not result.unexpectedSuccesses and unchanged)
    print(json.dumps(dict(
        schema='ferric-s-rpo-loader-tests-v1', passed=passed,
        names=list(names), passing_names=sorted(result.passing_names), tests_run=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
        source_unchanged=unchanged,
        sources={name: dict(bytes=size, sha256=digest) for name, (size, digest) in FILES.items()},
        synthetic_loader_only=True, compiler_qualification=False, gpu_execution=False,
    ), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())

