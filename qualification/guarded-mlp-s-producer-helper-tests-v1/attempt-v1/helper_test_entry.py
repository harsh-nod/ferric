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
    'qualification_helpers.py': (4103, 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'),
    'test_qualification_helpers.py': (8866, 'e0480bfe8e5278eea12551e7c950225a387c4d9b48f79cb2dbd146c93443b91a'),
}
EXPECTED = tuple(sorted(
    ['test_qualification_helpers.BackendRlibTests.' + name for name in (
        'test_duplicate_rows_and_archive_filenames_refuse',
        'test_elf_thin_archive_and_short_magic_refuse',
        'test_exact_non_test_backend_archive_selects_and_pins',
        'test_file_alias_refuses',
        'test_missing_archive_and_wrong_basename_refuse',
        'test_other_package_target_kind_source_and_profile_refuse',
        'test_outside_target_refuses_before_pin',
        'test_parent_alias_refuses',
    )] + ['test_qualification_helpers.FullSuiteTests.' + name for name in (
        'test_duplicate_missing_and_unexpected_names_refuse',
        'test_exact_ignored_names_and_should_panic_are_retained',
        'test_failed_or_changed_ignored_status_refuses',
        'test_forged_multiple_and_missing_summary_refuse',
        'test_invalid_expected_rosters_refuse',
        'test_malformed_named_line_is_not_silently_ignored',
        'test_no_ignored_suite_is_supported',
        'test_swapped_ignored_identity_refuses_even_with_same_counts',
    )]))


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
            'aliased or nonordinary helper source')
    require(before.st_size == FILES[name][0], 'helper source extent differs')
    body = path.read_bytes()
    require(stamp(before) == stamp(path.lstat()), 'helper source changed while reading')
    require((len(body), hashlib.sha256(body).hexdigest()) == FILES[name], 'helper source pin differs')
    return body


def load_exact(name, body):
    require(name not in sys.modules, 'helper module already present')
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
    module = load_exact('test_qualification_helpers', bodies['test_qualification_helpers.py'])
    suite = unittest.defaultTestLoader.loadTestsFromModule(module)
    names = tuple(sorted(test_names(suite)))
    require(names == EXPECTED and len(names) == len(set(names)) == 16, 'exact16 synthetic test inventory required')
    result = unittest.TextTestRunner(verbosity=2, resultclass=NamedResult).run(suite)
    unchanged = all(source_body(name) == body for name, body in bodies.items())
    passed = (result.wasSuccessful() and result.testsRun == 16
              and tuple(sorted(result.passing_names)) == EXPECTED
              and not result.skipped and not result.expectedFailures
              and not result.unexpectedSuccesses and unchanged)
    print(json.dumps(dict(
        schema='ferric-s-producer-helper-tests-v1', passed=passed,
        names=list(names), passing_names=sorted(result.passing_names), tests_run=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
        expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
        source_unchanged=unchanged,
        sources={name: dict(bytes=size, sha256=digest) for name, (size, digest) in FILES.items()},
        synthetic_helpers_only=True, compiler_qualification=False, gpu_execution=False,
    ), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
