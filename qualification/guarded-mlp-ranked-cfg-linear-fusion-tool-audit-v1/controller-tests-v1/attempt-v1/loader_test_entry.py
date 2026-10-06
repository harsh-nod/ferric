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
    'baseline-failed.json': (16503, '1bf26b28162414dfb4afbf6dc5ba5a70a60d1c1fe1fd55624c251733840c560b'),
    'baseline-compile.stderr': (164209, '9a309b8f23c66ad29e98a464bcb3ed8480766e2c7fd807415616dc99d6121a05'),
    'audit_tools.py': (96798, '7a1323ef8e14a474b4387b0438cae5eb00713c710947a657588c0a0c67e1dd96'),
    'test_audit_tools.py': (96433, 'c0afb4f640d3d8efd1889b4e027708250243186966cd90dd4765e38f258f3a34'),
}
EXPECTED = (
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_exact_cfg_linear_fusion_generation_preserves_non_deployed_provenance',
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_pending_cfg_linear_fusion_receipt_refuses_before_effects',
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_refuses_changed_independent_limit_or_inherited_cohort',
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_refuses_changed_non_overlay_body_or_preimage',
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_refuses_failed_admitting_or_wrong_generation',
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_refuses_lost_old_name_changed_ignore_or_cfg_linear_fusion_status',
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_refuses_old_product_and_any_unchanged_tool_substitution',
    'test_audit_tools.CfgLinearFusionAdmissionTests.test_refuses_phase_omission_reordering_and_cleanup',
    'test_audit_tools.DriverAdmissionTests.test_each_unchanged_tool_and_old_driver_are_refused',
    'test_audit_tools.DriverAdmissionTests.test_exact_focused_census_and_no_full_suite_claim',
    'test_audit_tools.DriverAdmissionTests.test_failed_unreviewed_or_wrong_source_generation_refused',
    'test_audit_tools.DriverAdmissionTests.test_missing_changed_or_relocated_source_map_refused',
    'test_audit_tools.DriverAdmissionTests.test_only_new_driver_and_test_metadata_are_admitted',
    'test_audit_tools.DriverAdmissionTests.test_pending_driver_binding_refuses_before_host_or_output_effects',
    'test_audit_tools.DriverAdmissionTests.test_phase_cleanup_and_wrong_final_build_refused',
    'test_audit_tools.DriverAdmissionTests.test_wrong_driver_or_test_cargo_role_refused',
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
    require(names == EXPECTED and len(names) == len(set(names)) == 26, 'exact26 synthetic test inventory required')
    result = unittest.TextTestRunner(verbosity=2, resultclass=NamedResult).run(suite)
    unchanged = all(source_body(name) == body for name, body in bodies.items())
    passed = (result.wasSuccessful() and result.testsRun == 26
              and tuple(sorted(result.passing_names)) == EXPECTED
              and not result.skipped and not result.expectedFailures
              and not result.unexpectedSuccesses and unchanged)
    print(json.dumps(dict(
        schema='ferric-ranked-cfg-linear-fusion-loader-tests-v1', passed=passed,
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
