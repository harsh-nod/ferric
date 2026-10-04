"""Unchanged-RPO library control; records a bounded named failure without qualification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-kir-indexed-formal-join-baseline-cpu-v1'
OUT = E / 'kir-indexed-formal-join-baseline-cpu-v228-v1'
OWNER = E / 'kir-indexed-formal-join-baseline-cpu-owner-v228-v1'
COPY, TARGET = OUT / 'source/fe2o3', OUT / 'target'
QUALIFIER = E / 'p228-kir-indexed-formal-join-cpu-v1/run.py'
QUALIFIER_SHA = '3ad6ed9c801b561b50219fcee55b0dacec2785ca0153bffa057181f8335b6431'
QUALIFIER_PACKAGE_SHA = '38206925bd968e7e883ad9c7f1184e9bf987d68f0e655b7fd2f3c9dc0f43c1a8'
SOURCE_SHA = '6501750a4ac345cbd47e543954580775eab5b8df406bc302117906b5c1265560'
CANDIDATE = E / 'kir-indexed-formal-join-cpu-v228-v1'
CANDIDATE_OWNER = E / 'kir-indexed-formal-join-cpu-owner-v228-v1'
FAILED_SHA = '88d0fa37ebdbe9bda0501af8693f664c8dbfb5b909b586f2b930b16d37fb7b89'
FAILED_OWNER_SHA = 'b845ba6e230625237605601ced70efba5bc8427346076256205b6014587fc183'
FAILURE = 'production_semantic_kir_v1::wave_task_entry_parameter_tests::access_roots::retained_fields::retained_nested_enum_referent_scalar_move_invalidates_saved_references'
PHASES = {'metadata', 'lower-build-tests', 'lower-list', 'lower-ignored-list', 'lower-focused-test', 'lower-tests'}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def load_qualifier():
    require(QUALIFIER.resolve(strict=True) == QUALIFIER and QUALIFIER.is_file(), 'canonical retained qualifier')
    raw = QUALIFIER.read_bytes()
    require(len(raw) < 1 << 20 and hashlib.sha256(raw).hexdigest() == QUALIFIER_SHA, 'exact retained qualifier source')
    module = types.ModuleType('indexed_join_control_prerequisites')
    module.__file__ = str(QUALIFIER)
    exec(compile(raw, str(QUALIFIER), 'exec'), module.__dict__)
    return module


def observed_tests(stdout, stderr, names, filtered, result):
    require(names == sorted(set(names)) and names and type(result['exit_code']) is int
            and result['exit_code'] in (0, 101) and result['reason'] is None and result['group_absent'] is True,
            'only natural complete test observations')
    rows = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|FAILED|ignored(?:, [^\n]*)?)$', stdout, re.M)
    require(sorted(name for name, _ in rows) == names and all(status in ('ok', 'FAILED') for _, status in rows),
            'all expected names appear exactly once, without ignored tests')
    failed = sorted(name for name, status in rows if status == 'FAILED')
    passed = len(names) - len(failed)
    label = 'ok' if not failed else 'FAILED'
    require(re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', stdout)
            == [(label, str(passed), str(len(failed)), '0', '0', str(filtered))], 'exact actual named result summary')
    require(result['exit_code'] == (101 if failed else 0), 'natural exit matches actual test outcome')
    if failed:
        require(failed == [FAILURE] and re.findall(r'left: \((\d+), Some\((\d+)\), (\d+)\)\s+right: \((\d+), Some\((\d+)\), (\d+)\)', stdout + stderr)
                == [('8', '7', '5', '7', '4', '5')], 'only the exact observed RPO assertion can be recorded as expected negative evidence')
    return dict(passed=passed, failed=len(failed), ignored=0, names=names, failed_names=failed,
                filtered_out=filtered, exit_code=result['exit_code'], test_suite_passed=not failed)


def allow_observed_assertion(error, name, result):
    require(type(error) is AssertionError and error.args == (name,) and type(result['exit_code']) is int
            and result['exit_code'] == 101 and result['reason'] is None and result['group_absent'] is True,
            'only bounded.run final natural101 assertion is an observable test outcome')


def baseline_inventory(names, ignored, candidate, additions):
    require(len(candidate) == len(set(candidate)) == 785 and len(additions) == len(set(additions)) == 20
            and set(additions) <= set(candidate) and FAILURE in candidate and FAILURE not in additions,
            'authentic candidate inventory and exact twenty additions')
    expected = sorted(set(candidate) - set(additions))
    require(names == expected and len(names) == 765 and ignored == [], 'unchanged-source full765 inventory without hidden skips')
    return expected


def dependency_transition(previous, current, before, original):
    local = {str(Path(name).relative_to(original)) for name in previous if Path(name).is_relative_to(original)}
    now = {name: row for name, row in current.items() if Path(name).is_relative_to(COPY)}
    require({str(Path(name).relative_to(COPY)) for name in now} == local
            and all(row == before[name] for name, row in now.items()), 'unchanged original local dependency subset')
    require({name: row for name, row in previous.items() if not Path(name).is_relative_to(original)}
            == {name: row for name, row in current.items() if name not in now}, 'unchanged external dependencies')


def context(package_sha):
    h = load_qualifier()
    c = h.context(QUALIFIER_PACKAGE_SHA, SOURCE_SHA)
    pins, p, outer = c['pins'], c['modules']['probe'], c['modules']['outer']
    pins.pin(QUALIFIER, QUALIFIER_SHA)
    candidate, failed_pin = pins.json(CANDIDATE / 'failed.json', FAILED_SHA)
    owner, owner_pin = pins.json(CANDIDATE_OWNER / 'failed.json', FAILED_OWNER_SHA)
    require(candidate['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-result-v1'
            and candidate['passed'] is False and candidate['error'] == 'AssertionError: lower-tests'
            and candidate['postcheck_errors'] == [] and candidate['source_unchanged'] is True
            and candidate['proposal'] == c['proposal_pin'] and candidate['package'] == c['package_pin'],
            'exact preserved candidate failure and unchanged source')
    require(owner['schema'] == 'ferric-p228-kir-indexed-formal-join-owned-result-v1'
            and owner['passed'] is False and owner['completion'] is None and owner['postcheck_errors'] == []
            and owner['error'] == 'RuntimeError: natural successful owned-tree exit required'
            and owner['package'] == candidate['package'] and owner['proposal'] == candidate['proposal'],
            'candidate failure owner linkage')
    owned = owner['owned']
    require(type(owned['exit_code']) is int and owned['exit_code'] == 1 and owned['reason'] is None
            and owned['cleanup_signalled'] is False and owned['owned_groups_absent'] is True
            and owned['owned_processes_reaped'] is True, 'actual natural reaped failed candidate')
    expected_phases = {'rustfmt', 'rustfmt-check', 'metadata', 'lower-build-tests', 'lower-list', 'lower-ignored-list', 'lower-tests'}
    require(set(candidate['phases']) == expected_phases and len(candidate['raw']) == 45, 'actual attempted candidate phase/raw census')
    for name, outcome in candidate['phases'].items():
        require(type(outcome['exit_code']) is int and outcome['exit_code'] == (101 if name == 'lower-tests' else 0)
                and outcome['reason'] is None and outcome['group_absent'] is True, 'actual natural candidate outcomes')
    for record in [*candidate['raw'].values(), *candidate['artifacts'].values()]:
        outer.pin_exact(pins, record, CANDIDATE)
    roster = candidate['lower_inventory']
    require(roster['ignored_names'] == [] and roster['added_names'] == sorted(c['proposal']['added_tests']),
            'candidate ignored/addition inventory binding')
    observed = observed_tests(p.read(CANDIDATE / 'lower-tests-stdout', candidate['raw']['lower-tests-stdout'], retain=True)[2].decode(),
        p.read(CANDIDATE / 'lower-tests-stderr', candidate['raw']['lower-tests-stderr'], retain=True)[2].decode(),
        roster['names'], 0, candidate['phases']['lower-tests'])
    require(observed['passed'] == 784 and observed['failed_names'] == [FAILURE], 'actual full candidate negative result')
    manifest, package_pin = pins.json(PACKAGE / 'manifest.json', package_sha)
    require(manifest['schema'] == 'ferric-p228-kir-indexed-baseline-cpu-package-v1'
            and len(manifest['files']) == 3 and {row['path'] for row in manifest['files']} == {'run.py', 'test_run.py', 'README.md'},
            'closed baseline observer package')
    for row in manifest['files']:
        outer.pin_exact(pins, dict(row, path=str(PACKAGE / row['path'])))
    c.update(h=h, baseline_package=package_pin, candidate=candidate, candidate_pin=failed_pin,
             candidate_owner=owner_pin, candidate_observation=observed)
    return c


def child(c):
    h, base, m, pins = c['h'], c['base'], c['modules'], c['pins']
    p, n = m['probe'], m['bounded']
    require(not os.path.lexists(OUT), 'fresh unchanged-source baseline case')
    OUT.mkdir(mode=0o700)
    n.F, n.T, n.D = COPY, TARGET, OUT
    n.setup(); require(not any(TARGET.iterdir()), 'fresh empty baseline target')
    cpu = c['cpu']
    original = p.parse(p.read(h.CPU / 'sources-before.json', cpu['raw']['sources-before.json'], retain=True)[2])
    inputs = p.parse(p.read(h.CPU / 'inputs-before.json', cpu['raw']['inputs-before.json'], retain=True)[2])
    old_deps = p.parse(p.read(h.CPU / 'dependencies-before.json', cpu['raw']['dependencies-before.json'], retain=True)[2])
    require(len(original) == 5783 and p.snapshot(p.tree(h.ORIGINAL)) == original, 'exact unchanged original RPO source')
    require(p.snapshot(sorted(inputs['files']), byte_cap=4 << 30) == inputs['files']
            and p.snapshot(sorted(old_deps), byte_cap=4 << 30) == old_deps, 'original tool/dependency inputs')
    for path, row in original.items():
        dest = COPY / Path(path).relative_to(h.ORIGINAL)
        dest.parent.mkdir(parents=True, exist_ok=True)
        raw = p.read(path, row['pin'], retain=True)[2]
        with dest.open('xb') as stream: stream.write(raw)
        dest.chmod(0o700 if Path(path).stat().st_mode & 0o111 else 0o600)
    before = p.snapshot(p.tree(COPY))
    require(base.relative_sources(before, COPY) == base.relative_sources(original, h.ORIGINAL),
            'byte-exact original copy without candidate overlay or formatter')
    p.save(OUT / 'sources-before.json', before)
    config_names = set(inputs['configurations']) | set(p.config_paths())
    p.F = COPY; config_names.update(p.config_paths()); p.config_paths = lambda: sorted(config_names)
    configs = {name: None for name in config_names}; config_before = p.configurations(configs)
    require(all(config_before[name] == value for name, value in inputs['configurations'].items()), 'original Cargo configuration')
    p.save(OUT / 'configurations-before.json', config_before)
    env = n.environment(); (TARGET / 'tmp').mkdir(mode=0o700); env['TMPDIR'] = str(TARGET / 'tmp')
    require(h.DIAGNOSTIC_ENV not in env, 'no diagnostic flag')
    cargo = str(n.N / 'bin/cargo')
    base.COPY, base.OUT = COPY, OUT
    phases, artifacts, tests = {}, {}, {}
    roots, dependencies, error, post_errors = None, None, None, []

    def run(name, argv, deadline=1800, observe=False):
        try:
            n.run(OUT, name, argv, env=env, deadline=deadline)
        except AssertionError as failure:
            if not observe: raise
            result = p.parse(p.read(OUT / (name + '-result.json'), retain=True)[2])
            allow_observed_assertion(failure, name, result)
        finally:
            path = OUT / (name + '-result.json')
            if path.is_file(): phases[name] = p.parse(p.read(path, retain=True)[2])
        return p.read(OUT / (name + '-stdout'), cap=64 << 20, retain=True)[2].decode()

    def dependency_snapshot():
        return p.snapshot(sorted({name for root in roots for name in p.tree(root, exclusions=('.git', 'target'))}), byte_cap=4 << 30)

    try:
        metadata = p.parse(run('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(COPY / 'Cargo.toml'), '--format-version', '1'], 120))
        prior = p.parse(p.read(h.CPU / 'metadata-stdout', cpu['raw']['metadata-stdout'], retain=True)[2])
        h.OUT = OUT
        require(metadata == h.relocate(prior), 'exact relocated original metadata')
        roots = base.metadata_paths(metadata, COPY); dependencies = dependency_snapshot()
        dependency_transition(old_deps, dependencies, before, h.ORIGINAL)
        p.save(OUT / 'dependencies-before.json', dependencies)
        built = run('lower-build-tests', [cargo, 'test', '--offline', '--locked', '--jobs', '2', '--manifest-path', str(COPY / 'Cargo.toml'),
            '-p', 'fe2o3-lower-mir-kernel', '--lib', '--no-run', '--message-format=json'])
        paths = base.built_artifacts(built, 'fe2o3-lower-mir-kernel', 'fe2o3_lower_mir_kernel', True, TARGET)
        require(len(paths) == 1, 'one actual unchanged-source library test ELF')
        binary = paths[0]; artifacts['lower-tests'] = p.read(binary)[0]
        names = base.inventory(run('lower-list', [binary, '--list', '--format', 'terse'], 120))
        ignored = sorted(re.findall(r'^([^\r\n]+): test$', run('lower-ignored-list', [binary, '--ignored', '--list', '--format', 'terse'], 120), re.M))
        baseline_inventory(names, ignored, c['candidate']['lower_inventory']['names'], c['proposal']['added_tests'])
        for name, argv, selected, filtered in (
            ('lower-focused-test', [binary, '--exact', FAILURE, '--show-output', '--test-threads=1'], [FAILURE], len(names) - 1),
            ('lower-tests', [binary, '--test-threads=2'], names, 0),
        ):
            stdout = run(name, argv, observe=True)
            stderr = p.read(OUT / (name + '-stderr'), retain=True)[2].decode()
            tests[name] = observed_tests(stdout, stderr, selected, filtered, phases[name])
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        checks = [('original', original, lambda: p.snapshot(p.tree(h.ORIGINAL))),
            ('sources', before, lambda: p.snapshot(p.tree(COPY))),
            ('inputs', inputs['files'], lambda: p.snapshot(sorted(inputs['files']), byte_cap=4 << 30)),
            ('prior-dependencies', old_deps, lambda: p.snapshot(sorted(old_deps), byte_cap=4 << 30)),
            ('configurations', config_before, lambda: p.configurations(configs))]
        if dependencies is not None: checks.append(('dependencies', dependencies, dependency_snapshot))
        for name, expected, operation in checks:
            try:
                actual = operation(); p.save(OUT / (name + '-after.json'), actual)
                require(actual == expected, name + ' changed')
            except BaseException as failure:
                post_errors.append(name + ': ' + repr(failure))
        try:
            pins.recheck()
            for record in artifacts.values(): p.read(record['path'], record)
        except BaseException as failure:
            post_errors.append('pinned inputs/products: ' + repr(failure))
    passed = error is None and not post_errors and set(phases) == PHASES and len(tests) == 2
    repeated = passed and all(row['failed_names'] == [FAILURE] for row in tests.values())
    value = dict(schema='ferric-p228-kir-indexed-baseline-cpu-result-v1', passed=passed,
        observation_completed=passed, error=error, postcheck_errors=post_errors,
        package=c['baseline_package'], candidate_failure=c['candidate_pin'], candidate_owner=c['candidate_owner'],
        compiler_cpu=c['v'].CPU_PIN, source_generation=cpu['raw']['sources-before.json'], source_unchanged=not post_errors,
        phases=phases, tests=tests, candidate_observation=c['candidate_observation'], artifacts=artifacts,
        preexisting_rpo_failure_observed=repeated, baseline_library_passed=tests.get('lower-tests', {}).get('test_suite_passed'),
        candidate_qualified=False, library_qualified=False, source_overlay_applied=False, source_formatted=False,
        fresh_compiler_built=False, fresh_hsaco_emitted=False, actual_capture_join_passed=False,
        gpu_execution=False, numerical_acceptance=False, production_authority=False, performance_claim=False,
        raw={path.name: p.read(path)[0] for path in OUT.iterdir() if path.is_file()})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, observation_completed=passed, preexisting_rpo_failure_observed=repeated,
                         error=error, postcheck_errors=post_errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python -B')
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('manifest_sha256'); parser.add_argument('--child', action='store_true')
    args = parser.parse_args(); require(re.fullmatch('[0-9a-f]{64}', args.manifest_sha256), 'root-authenticated package SHA')
    require(Path(__file__).resolve().parent == PACKAGE and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'asrock-1w300-g2-2b' and os.sched_getaffinity(0) == {8, 9}
            and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'fixed owned CPU host/package')
    for kind, cap in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind); limit = min([cap] + [word for word in old if word != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    c = context(args.manifest_sha256)
    if args.child:
        signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt('owned termination')))
        return child(c)
    h, m = c['h'], c['modules']; p, outer, driver = (m[key] for key in ('probe', 'outer', 'driver'))
    require(not os.path.lexists(OWNER) and not os.path.lexists(OUT), 'fresh baseline owner/case')
    old_paths = tuple(dict.fromkeys((*outer.OLD_TARGETS, *c['v'].EXTRA_OLD_TARGETS,
        h.CPU / 'target', h.ROW / 'target', h.DIAGNOSTIC / 'target', CANDIDATE / 'target')))
    require(all(not TARGET.is_relative_to(path) and not path.is_relative_to(TARGET) for path in old_paths), 'fresh separate baseline target')
    previous = outer.inventory(m['inventory'], old_paths)
    os.umask(0o077); OWNER.mkdir(mode=0o700); p.save(OWNER / 'old-targets-before.json', previous)
    argv = ['/usr/bin/python3', '-B', str(Path(__file__).resolve()), args.manifest_sha256, '--child']
    env = m['bounded'].environment()
    p.save(OWNER / 'command.json', dict(argv=argv, env=env, deadline_seconds=10800, gpu_execution=False))
    outcome, completion, error = None, None, None
    try:
        outcome = driver.run_coordinator(OWNER, argv, h.ORIGINAL, env, m['owned'], p.save, deadline=10800)
        p.save(OWNER / 'owned-result.json', outcome); outer.check_outcome(outcome)
        value, completion = c['pins'].json(OUT / 'complete.json')
        require(value['passed'] is True and value['observation_completed'] is True and value['postcheck_errors'] == []
                and value['package'] == c['baseline_package'] and value['candidate_failure'] == c['candidate_pin']
                and value['candidate_qualified'] is False and value['library_qualified'] is False
                and set(value['phases']) == PHASES, 'completed observer without a qualification claim')
        for record in value['raw'].values(): outer.pin_exact(c['pins'], record, OUT)
        for record in value['artifacts'].values(): outer.pin_exact(c['pins'], record, TARGET)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        after, errors = outer.postchecks([('pins', None, c['pins'].recheck),
            ('old_targets', previous, lambda: outer.inventory(m['inventory'], old_paths))])
        p.save(OWNER / 'after.json', after)
    passed = error is None and not errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(schema='ferric-p228-kir-indexed-baseline-owned-result-v1',
        passed=passed, observation_completed=passed, error=error, postcheck_errors=errors, owned=outcome,
        completion=completion, package=c['baseline_package'], candidate_failure=c['candidate_pin'],
        candidate_qualified=False, library_qualified=False, gpu_execution=False, production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
