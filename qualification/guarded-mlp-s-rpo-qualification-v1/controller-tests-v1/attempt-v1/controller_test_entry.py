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
    'run_cpu.py': (53331, '083abf39c6a9d24ff379c238a7f41c56dfb71757a2678a08f65ca935fd2fc866'),
    'qualification_helpers.py': (4103, 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'),
    'test_run.py': (5500, 'a20d03a985724c5f2e9c34210b5b76143633a84ff5b6a3967c4f1ae19bda9e9b'),
}
EXPECTED = (
    'test_run.SourceContractTests.test_five_closed_cohorts_have_43_distinct_names',
    'test_run.SourceContractTests.test_fixture_context_derivation_preserves_actual_session_literal',
    'test_run.SourceContractTests.test_insertion_refuses_deletion',
    'test_run.SourceContractTests.test_insertion_refuses_empty_and_missing_destination',
    'test_run.SourceContractTests.test_insertion_refuses_missing_or_duplicate_context',
    'test_run.SourceContractTests.test_insertion_refuses_unknown_or_repeated_destination',
    'test_run.SourceContractTests.test_metadata_does_not_replace_embedded_text_or_prefix_sibling',
    'test_run.SourceContractTests.test_metadata_refuses_overlapping_or_duplicate_mapping',
    'test_run.SourceContractTests.test_metadata_refuses_wrong_source_or_external_destination',
    'test_run.SourceContractTests.test_metadata_relocates_paths_and_structured_path_ids',
    'test_run.SourceContractTests.test_two_insertion_hunks_preserve_all_other_bytes',
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
            'aliased or nonordinary controller-test source')
    require(before.st_size == FILES[name][0], 'controller-test source extent differs')
    body = path.read_bytes()
    require(stamp(before) == stamp(path.lstat()), 'controller-test source changed while reading')
    require((len(body), hashlib.sha256(body).hexdigest()) == FILES[name], 'controller-test source pin differs')
    return body


def load_exact(name, body):
    require(name not in sys.modules, 'controller-test module already present')
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
    load_exact('qualification_helpers', bodies['qualification_helpers.py'])
    load_exact('run_cpu', bodies['run_cpu.py'])
    module = load_exact('test_run', bodies['test_run.py'])
    suite = unittest.defaultTestLoader.loadTestsFromModule(module)
    names = tuple(sorted(test_names(suite)))
    require(names == EXPECTED and len(names) == len(set(names)) == 11, 'exact11 synthetic test inventory required')
    result = unittest.TextTestRunner(verbosity=2, resultclass=NamedResult).run(suite)
    unchanged = all(source_body(name) == body for name, body in bodies.items())
    passed = (result.wasSuccessful() and result.testsRun == 11
              and tuple(sorted(result.passing_names)) == EXPECTED
              and not result.skipped and not result.expectedFailures
              and not result.unexpectedSuccesses and unchanged)
    print(json.dumps(dict(
        schema='ferric-s-rpo-controller-tests-v1', passed=passed,
        names=list(names), passing_names=sorted(result.passing_names), tests_run=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
        source_unchanged=unchanged,
        sources={name: dict(bytes=size, sha256=digest) for name, (size, digest) in FILES.items()},
        synthetic_controller_only=True, compiler_qualification=False, gpu_execution=False,
    ), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
