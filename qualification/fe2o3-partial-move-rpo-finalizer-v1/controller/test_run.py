"""Pure prerequisite and routing checks; no processes or GPU access."""
import copy
import unittest
from types import SimpleNamespace
from unittest.mock import patch

import run


class ToolTests(unittest.TestCase):
    def setUp(self):
        self.cpu_pin = dict(path=str(run.CPU / 'complete.json'), bytes=1, sha256='ab' * 32)
        guard = patch.object(run, 'CPU_PIN', self.cpu_pin)
        guard.start(); self.addCleanup(guard.stop)

    def cpu(self):
        value = dict(schema='fe2o3-p228-rpo-compiler-cpu-result-v1', passed=True,
                     fresh_compiler_built=True, error=None, postcheck_errors=[],
                     phases={str(n): dict(exit_code=0, reason=None, group_absent=True) for n in run.CPU_PHASES},
                     artifacts={name: {} for name in ('compiler-tests', 'pliron-tests', 'fe2o3-rustc-extract',
                                'librustc_codegen_fe2o3.so', 'librustc_codegen_fe2o3.rlib')})
        value.update({key: False for key in ('checked_lowering', 'fresh_hsaco_emitted', 'gpu_execution',
                     'production_authority', 'numerical_acceptance', 'performance_claim', 'full_model_acceptance')})
        owner = dict(schema='fe2o3-p228-rpo-compiler-owned-result-v1', passed=True,
                     error=None, postcheck_errors=[], completion=copy.deepcopy(run.CPU_PIN))
        value.update(package=copy.deepcopy(run.CPU_PACKAGE), patch=copy.deepcopy(run.PATCHES['rpo']),
                     qualified_generation=copy.deepcopy(run.QUALIFIED_GENERATION),
                     required_test_names=copy.deepcopy(run.REQUIRED_TESTS), tests={})
        owner.update(patch=copy.deepcopy(run.PATCHES['rpo']),
                     qualified_generation=copy.deepcopy(run.QUALIFIED_GENERATION))
        for short, passed, ignored in (('pliron', 1504, 1), ('compiler', 1196, 24)):
            required = list(run.REQUIRED_TESTS[short])
            skipped = [short + '::synthetic_ignored_' + str(n) for n in range(ignored)]
            names = sorted(required + skipped + [
                short + '::synthetic_pass_' + str(n) for n in range(passed - len(required))])
            value['tests'][short] = dict(passed=passed, ignored=ignored, names=names,
                                        ignored_names=sorted(skipped))
        return value, owner

    def test_successful_cpu_and_owner_join(self):
        run.validate_cpu(*self.cpu())

    def test_rejects_failed_or_incomplete_cpu(self):
        for key, value in [('passed', False), ('fresh_compiler_built', False), ('error', 'failed'),
                           ('postcheck_errors', ['changed']), ('phases', {}), ('artifacts', {}),
                           ('package', {}), ('package', dict(run.CPU_PACKAGE, sha256='00' * 32))]:
            cpu, owner = self.cpu()
            cpu[key] = value
            with self.assertRaises(RuntimeError):
                run.validate_cpu(cpu, owner)

    def test_rejects_wrong_owner_or_schema(self):
        for key, value in [('passed', False), ('error', 'failed'), ('postcheck_errors', ['changed']),
                           ('schema', 'unrelated'), ('completion', {})]:
            cpu, owner = self.cpu()
            owner[key] = value
            with self.assertRaises(RuntimeError):
                run.validate_cpu(cpu, owner)

    def test_rejects_nonterminal_phase(self):
        for key, value in [('exit_code', 1), ('reason', 'deadline'), ('group_absent', False)]:
            cpu, owner = self.cpu()
            cpu['phases']['metadata'][key] = value
            with self.assertRaises(RuntimeError):
                run.validate_cpu(cpu, owner)

    def test_cpu_gate_cannot_confer_gpu_or_production_claim(self):
        for key in ('checked_lowering', 'fresh_hsaco_emitted', 'gpu_execution', 'production_authority',
                    'numerical_acceptance', 'performance_claim', 'full_model_acceptance'):
            cpu, owner = self.cpu()
            cpu[key] = True
            with self.assertRaises(RuntimeError):
                run.validate_cpu(cpu, owner)

    def test_rejects_same_count_substituted_phase(self):
        cpu, owner = self.cpu()
        cpu['phases']['unrelated'] = cpu['phases'].pop('metadata')
        with self.assertRaises(RuntimeError):
            run.validate_cpu(cpu, owner)

    def test_rejects_overlay_or_qualified_generation_drift(self):
        for side, field in (('cpu', 'patch'), ('owner', 'patch'),
                            ('cpu', 'qualified_generation'), ('owner', 'qualified_generation')):
            cpu, owner = self.cpu()
            (cpu if side == 'cpu' else owner)[field] = {}
            with self.assertRaises(RuntimeError):
                run.validate_cpu(cpu, owner)

    def test_required_tests_must_be_declared_counted_and_nonignored(self):
        for short in (name for name, tests in run.REQUIRED_TESTS.items() if tests):
            for mutation in ('declaration', 'ignored', 'missing', 'count', 'duplicate'):
                cpu, owner = self.cpu()
                row = cpu['tests'][short]
                required = run.REQUIRED_TESTS[short][0]
                if mutation == 'declaration':
                    cpu['required_test_names'][short].remove(required)
                elif mutation == 'ignored':
                    row['ignored_names'] = sorted([required] + row['ignored_names'][1:])
                elif mutation == 'missing':
                    row['names'].remove(required)
                    row['names'] = sorted(row['names'] + [short + '::substitute'])
                elif mutation == 'count':
                    row['passed'] -= 1
                else:
                    row['names'][-1] = row['names'][0]
                    row['names'].sort()
                with self.assertRaises(RuntimeError):
                    run.validate_cpu(cpu, owner)

    def test_finalizer_census_and_separate_actual_capture_gate(self):
        ignored = sorted([run.ACTUAL_JOIN] + ['ignored_' + str(n) for n in range(14)])
        names = sorted(ignored + ['pass_' + str(n) for n in range(190)])
        prior = dict(names=names, ignored_names=ignored, passed=190, ignored=15)
        run.finalizer_inventory(names, ignored, prior)
        for changed_names, changed_ignored in (
                (names[:-1], ignored),
                (names, ignored[:-1]),
                (sorted(names[:-1] + [names[0]]), ignored),
                (names, sorted([name for name in ignored if name != run.ACTUAL_JOIN] + ['pass_0']))):
            with self.assertRaises(RuntimeError):
                run.finalizer_inventory(changed_names, changed_ignored, prior)

    def test_all_old_targets_exclude_active_target(self):
        old = (run.R / 'evidence/wave-attention-v214/target-nightly',
               run.R / 'evidence/wave-attention-v214/target-native-nightly-v225')
        self.assertEqual(run.protected_targets(old), old + run.EXTRA_OLD_TARGETS)
        self.assertEqual(run.protected_targets(old + run.EXTRA_OLD_TARGETS),
                         old + run.EXTRA_OLD_TARGETS)
        for bad in (run.TARGET, run.TARGET.parent, run.TARGET / 'debug'):
            with self.assertRaises(RuntimeError):
                run.protected_targets(old + (bad,))

    def test_cpu_raw_replay_routes_both_suites_and_rejects_drift(self):
        # Fake parsers test routing only; the production parser is pinned in BASE.
        cpu, _ = self.cpu()
        cpu['raw'], data, calls = {}, {}, []
        for short, row in cpu['tests'].items():
            for suffix, body in (
                    ('list', '\n'.join(row['names'])),
                    ('ignored-list', '\n'.join(name + ': test' for name in row['ignored_names'])),
                    ('tests', short)):
                name = short + '-' + suffix + '-stdout'
                cpu['raw'][name] = dict(path=str(run.CPU / name))
                data[str(run.CPU / name)] = body

        def read(pin):
            calls.append(pin['path'])
            return data[pin['path']]

        def parsed(raw, names, ignored):
            row = cpu['tests'][raw]
            self.assertEqual(names, row['names'])
            self.assertEqual(ignored, row['ignored_names'])
            return copy.deepcopy(row)

        base = SimpleNamespace(inventory=lambda raw: raw.splitlines(), test_results=parsed)
        run.replay_cpu_tests(base, cpu, read)
        self.assertEqual(len(calls), 6)
        self.assertEqual(set(calls), set(data))
        base.test_results = lambda *args: {}
        with self.assertRaises(RuntimeError):
            run.replay_cpu_tests(base, cpu, read)
        cpu['raw']['pliron-list-stdout']['path'] = str(run.OUT / 'pliron-list-stdout')
        with self.assertRaises(RuntimeError):
            run.replay_cpu_tests(base, cpu, read)

    def metadata(self):
        return dict(target_directory=str(run.TARGET), packages=[dict(name='fe2o3-hsaco-finalize',
                    manifest_path=str(run.COPY / 'crates/fe2o3-hsaco-finalize/Cargo.toml'), source=None)])

    def test_same_source_target_metadata(self):
        self.assertEqual(run.metadata_paths(self.metadata()), [run.COPY / 'crates/fe2o3-hsaco-finalize'])

    def test_rejects_wrong_target_or_local_dependency_escape(self):
        for kind in ('target', 'dependency'):
            value = self.metadata()
            if kind == 'target':
                value['target_directory'] = str(run.OUT / 'target')
            else:
                value['packages'][0]['manifest_path'] = str(run.R / 'fe2o3/crates/fe2o3-hsaco-finalize/Cargo.toml')
            with self.assertRaises(RuntimeError):
                run.metadata_paths(value)

    def test_rejects_missing_or_duplicate_finalizer(self):
        for rows in ([], self.metadata()['packages'] * 2):
            with self.assertRaises(RuntimeError):
                run.metadata_paths(dict(self.metadata(), packages=rows))


    def test_same_count_finalizer_name_substitution_is_rejected(self):
        ignored = sorted([run.ACTUAL_JOIN] + ['ignored_' + str(n) for n in range(14)])
        names = sorted(ignored + ['pass_' + str(n) for n in range(190)])
        prior = dict(names=names, ignored_names=ignored, passed=190, ignored=15)
        changed = sorted([name for name in names if name != 'pass_0'] + ['substitute'])
        with self.assertRaises(RuntimeError):
            run.finalizer_inventory(changed, ignored, prior)

    def test_candidate_has_twelve_cpu_phases_and_six_finalizer_phases(self):
        self.assertEqual(len(run.CPU_PHASES), 12)
        self.assertEqual(len(run.TOOL_PHASES), 6)
        self.assertTrue({'rustfmt', 'rustfmt-check'} <= run.CPU_PHASES)
        cpu, owner = self.cpu()
        cpu['phases'].pop('rustfmt')
        with self.assertRaises(RuntimeError):
            run.validate_cpu(cpu, owner)


if __name__ == '__main__':
    unittest.main()
