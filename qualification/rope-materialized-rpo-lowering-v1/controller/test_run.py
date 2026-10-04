"""Synthetic admission tests only; no compiler, subprocess, SSH or GPU calls."""
import ast
import copy
from pathlib import Path
import types
import unittest

import contracts
import run


class PolicyTests(unittest.TestCase):
    def template(self):
        old = run.PRIOR
        commands = []
        for name in run.STAGES:
            commands.append(dict(name=name, argv=['/retained/tool', '--exact', 'original_prefix_test'],
                env=dict(CARGO_TARGET_DIR=str(old / 'target'), LD_LIBRARY_PATH='/retained/deps:/nightly/lib',
                         CAPTURE=str(old / 'prefix-tiles-semantic.bin')),
                cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10, expected_exit=0,
                gpu_execution=False, deadline_seconds=1800 if name in ('checked-lowering', 'actual-replay') else 900))
        return dict(commands=commands, fresh_output=str(old), symbol=run.SYMBOL,
                    toolchain_and_retained_tool_pins=[{'unchanged': 1}],
                    compiler_generation={'unchanged': 2}, retained_modules=[{'unchanged': 3}])

    def test_guard_accepts_ordinary_python(self):
        run.guard(0, {})

    def test_guard_rejects_optimized_or_inherited_optimization(self):
        for optimized, env in ((1, {}), (0, {'PYTHONOPTIMIZE': ''}), (0, {'PYTHONOPTIMIZE': '0'})):
            with self.subTest(env=env), self.assertRaises(RuntimeError):
                run.guard(optimized, env)

    def test_actual_cpu_path_rejects_aliases_and_wrong_basename(self):
        path = run.E / 'rope-materialized-cpu-v228-v1/complete.json'
        self.assertEqual(run.cpu_argument(path, 'a' * 64), path)
        for suffix in ('silu-materialized-cpu-v228-v1/complete.json', 'rope-materialized-cpu-v228-v0/complete.json',
                       'rope-materialized-cpu-v228-v1/failed.json', 'x/../rope-materialized-cpu-v228-v1/complete.json'):
            with self.subTest(suffix=suffix), self.assertRaises(RuntimeError):
                run.cpu_argument(run.E / suffix, 'a' * 64)

    def test_relocation_does_not_rename_prefix_or_external_retained_tools(self):
        source = {'value': [str(run.PRIOR / 'prefix-tiles-semantic.bin'), str(run.PRIOR),
                            str(run.E / 'row-prefix-tiles-source-v227-v5/extract-retained')], 'limit': 1800}
        self.assertEqual(run.rewrite(source, run.PRIOR, run.OUT),
                         {'value': [str(run.OUT / 'prefix-tiles-semantic.bin'), str(run.OUT), source['value'][2]], 'limit': 1800})

    def test_recipe_retains_all_prefix_commands_env_and_generation(self):
        template = self.template()
        before = copy.deepcopy(template)
        recipe = run.make_recipe(template, [])
        self.assertEqual(template, before)
        self.assertEqual(recipe['commands'], run.rewrite(before['commands'], run.PRIOR, run.OUT))
        for key in ('toolchain_and_retained_tool_pins', 'compiler_generation', 'retained_modules', 'symbol'):
            self.assertEqual(recipe[key], before[key])
        self.assertEqual(tuple(row['name'] for row in recipe['commands']), run.STAGES)

    def test_recipe_refuses_changed_symbol_stage_or_bounds(self):
        changes = [('symbol', 'wrong'), ('stage', 'wrong'), ('cache_cap_bytes', 8 << 30),
                   ('gpu_execution', True), ('affinity', [0, 1]), ('nice', 0)]
        for key, value in changes:
            template = self.template()
            if key == 'symbol':
                template[key] = value
            elif key == 'stage':
                template['commands'][0]['name'] = value
            else:
                template['commands'][0][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                run.make_recipe(template, [])

    def test_lowering_fixture_is_exactly_eight_rust_plus_cargo_pair(self):
        rows = [dict(destination=name, source=dict(path='/actual/' + name, bytes=1, sha256='a' * 64))
                for name in sorted(run.FIXTURE_FILES)]
        self.assertEqual(len(run.fixture_contract(rows)), 10)
        for altered in (rows[:-1], rows + [rows[0]], [dict(row, source=rows[0]['source']) for row in rows]):
            with self.assertRaises(RuntimeError):
                run.fixture_contract(altered)

    def transition(self):
        old = dict(bytes=10, sha256='a' * 64)
        new = dict(bytes=11, sha256='b' * 64)
        base = {name: copy.deepcopy(old) for name in run.BASE_RUST | {'Cargo.toml', 'Cargo.lock'}}
        harness = {'Cargo.toml', 'Cargo.lock', 'src/fixture_lib.rs'}
        prior = {name: copy.deepcopy(old) for name in harness | run.OLD_TEST_INPUTS}
        rows = [dict(path=name, source='draft/' + name, before=base.get(name),
                     after=copy.deepcopy(old if name == 'tests/fixtures/v7_lib.rs' else new)) for name in sorted(run.CHANGES)]
        overlay = dict(baseline_fixture={'files': base}, files=rows)
        unformatted = {name: copy.deepcopy(base[name]) for name in run.BASE_RUST}
        unformatted.update(copy.deepcopy(prior))
        unformatted.update({row['path']: copy.deepcopy(row['after']) for row in rows})
        before = dict(fixture=copy.deepcopy(unformatted), provider={'old': 1})
        for name in run.FORMATTED:
            before['fixture'][name] = dict(bytes=12, sha256='c' * 64)
        return before, copy.deepcopy(before), unformatted, prior, overlay, harness

    def test_source_transition_allows_only_candidate_formatting(self):
        args = self.transition()
        run.source_transition(*args)
        self.assertEqual(len(args[0]['fixture']), 15)

    def test_source_transition_refuses_original_fixture_or_postcheck_drift(self):
        for change in ('original', 'baseline', 'preimage', 'after', 'extra'):
            args = list(self.transition())
            if change == 'original':
                args[0]['fixture']['src/head_rope_numerics_v3.rs']['bytes'] += 1
                args[1] = copy.deepcopy(args[0])
            elif change == 'baseline':
                args[0]['fixture']['tests/fixtures/v7_lib.rs']['bytes'] += 1
            elif change == 'preimage':
                args[4]['files'][0]['before'] = None
            elif change == 'after':
                args[1]['provider']['old'] = 2
            else:
                args[2]['src/unexpected.rs'] = dict(bytes=1, sha256='d' * 64)
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                run.source_transition(*args)

    def test_cpu_gate_rejects_wrong_outcome_or_cohort_before_io(self):
        value = dict(schema='ferric-p228-rope-materialized-cpu-result-v1', passed=True, error=None,
                     postcheck_errors=[], source_unchanged=True, tests_passed=33, tests_ignored=1,
                     cpu_arithmetic_only=True)
        for key, wrong in (('passed', False), ('tests_passed', 32), ('tests_ignored', 0), ('source_unchanged', False)):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                run.cpu_gate(None, None, dict(value, **{key: wrong}), None, None, None, None, None)

    def metadata(self):
        image = dict(sha256='1' * 64, bytes=1234)
        fields = dict(authority='none', object_sha256=image['sha256'], object_bytes='1234',
            entry_symbol_hex=run.SYMBOL.encode().hex(), target='gfx950:xnack-', code_object_version='6',
            explicit_argument_bytes='120', kernarg_segment_bytes='376', kernarg_alignment='8',
            workgroup='64,1,1', max_grid_workgroups='64,1,1', descriptor_sha256='2' * 64,
            canonical_code_object_digest='3' * 64, descriptor_symbol_hex='6162')
        return image, fields

    def encode(self, fields):
        return ('fe2o3-finite-join-request-metadata-v1\n' + ''.join(k + ' ' + v + '\n' for k, v in fields.items())).encode()

    def test_original_prefix_metadata_contract_preserves_fifteen_root_abi(self):
        image, fields = self.metadata()
        self.assertEqual(contracts.metadata(self.encode(fields), image), fields)

    def test_metadata_refuses_mlp_abi_or_new_authority(self):
        image, original = self.metadata()
        for key, value in (('explicit_argument_bytes', '88'), ('kernarg_segment_bytes', '344'),
                           ('workgroup', '128,1,1'), ('object_sha256', 'f' * 64), ('authority', 'launch')):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                contracts.metadata(self.encode(dict(original, **{key: value})), image)

    def test_exact_replay_must_be_one_executed_not_ignored_test(self):
        name = 'production_pipeline::conditional_formal_tests::actual_prefix'
        raw = ('test ' + name + ' ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n').encode()
        contracts.exact_test(raw, name)
        for changed in (raw + raw, raw.replace(b'1 passed', b'0 passed'), raw.replace(b'0 ignored', b'1 ignored')):
            with self.assertRaises(RuntimeError):
                contracts.exact_test(changed, name)

    def test_both_scopes_recheck_original_source_map_bytes_and_stamps(self):
        tree = ast.parse(Path(run.__file__).read_text())
        for scope in ('body', 'run'):
            function = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == scope)
            rows = [node for node in ast.walk(function) if isinstance(node, ast.Tuple) and len(node.elts) == 3
                    and isinstance(node.elts[0], ast.Constant) and node.elts[0].value == 'source_maps']
            self.assertEqual(len(rows), 1)
            records = [{'pin': 'actual', 'stamp': [1, 2, 3]}]
            observed = [({'source.rs': 'a' * 64}, copy.deepcopy(records))]
            context = dict(merged=observed[0][0], source_records=records, inputs={'source_maps': ['pin']})
            helper = types.SimpleNamespace(source_maps=lambda _: observed[0])
            _, expected, call = eval(compile(ast.Expression(rows[0]), '<postcheck>', 'eval'), {'c': context, 'p': helper})
            self.assertEqual(call(), expected)
            observed[0][1][0]['stamp'][2] += 1
            self.assertNotEqual(call(), expected)

    def generations(self):
        roles = ('compiler-tests', 'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so',
                 'finalizer', 'finalizer-tests', 'metadata')
        def generation(root, digit):
            return dict(target=root, prerequisites={k: dict(path=root + '/' + k, bytes=1, sha256=digit * 64)
                for k in ('cpu', 'cpu_owner', 'tools', 'tools_owner')},
                roles={k: dict(path=root + '/debug/' + k, bytes=2, sha256=digit * 64) for k in roles})
        old, new = generation('/old', 'a'), generation(str(run.RPO_CPU / 'target'), 'b')
        template = self.template()
        template['compiler_generation'] = copy.deepcopy(old['prerequisites'])
        template['toolchain_and_retained_tool_pins'] = list(old['roles'].values()) + [
            dict(path='/retained/extract-retained', bytes=3, sha256='c' * 64)]
        for row in template['commands']:
            row['env']['LD_LIBRARY_PATH'] = '/old/debug/deps:/nightly/lib'
            if row['name'] == 'checked-lowering':
                row['env']['RUSTC_WORKSPACE_WRAPPER'] = old['roles']['fe2o3-rustc-extract']['path']
        commands = {row['name']: row for row in template['commands']}
        for phase, role in (('actual-replay', 'compiler-tests'), ('actual-inert-join', 'finalizer-tests'),
                            ('emit', 'finalizer'), ('descriptor-metadata', 'metadata')):
            commands[phase]['argv'][0] = old['roles'][role]['path']
        return run.make_recipe(template, []), old, new

    def test_generation_recipe_replaces_only_roles_and_linker_search_path(self):
        recipe, old, new = self.generations()
        before = copy.deepcopy(recipe)
        actual = run.generation_recipe(recipe, old, new)
        self.assertEqual(recipe, before)
        self.assertEqual(actual['compiler_generation'], new['prerequisites'])
        self.assertEqual(actual['toolchain_and_retained_tool_pins'][-1], before['toolchain_and_retained_tool_pins'][-1])
        self.assertEqual(actual['retained_modules'], before['retained_modules'])
        restored = copy.deepcopy(actual)
        for row, prior in zip(restored['commands'], before['commands']):
            row['env']['LD_LIBRARY_PATH'] = prior['env']['LD_LIBRARY_PATH']
            if row['name'] == 'checked-lowering':
                self.assertEqual(row['env']['RUSTC_WORKSPACE_WRAPPER'], new['roles']['fe2o3-rustc-extract']['path'])
                row['env']['RUSTC_WORKSPACE_WRAPPER'] = prior['env']['RUSTC_WORKSPACE_WRAPPER']
            if row['name'] in ('actual-replay', 'actual-inert-join', 'emit', 'descriptor-metadata'):
                row['argv'][0] = prior['argv'][0]
        self.assertEqual(restored['commands'], before['commands'])
        self.assertEqual(len(actual['commands']), 9)

    def test_generation_recipe_refuses_mixed_tools_or_wrong_library_route(self):
        for change in ('prerequisite', 'role', 'pin', 'wrapper', 'replay', 'library', 'alias'):
            recipe, old, new = self.generations()
            commands = {row['name']: row for row in recipe['commands']}
            if change == 'prerequisite': recipe['compiler_generation']['cpu']['sha256'] = '0' * 64
            elif change == 'role': del new['roles']['metadata']
            elif change == 'pin': recipe['toolchain_and_retained_tool_pins'] = recipe['toolchain_and_retained_tool_pins'][1:]
            elif change == 'wrapper': commands['checked-lowering']['env']['RUSTC_WORKSPACE_WRAPPER'] = '/wrong'
            elif change == 'replay': commands['actual-replay']['argv'][0] = '/wrong'
            elif change == 'library': commands['emit']['env']['LD_LIBRARY_PATH'] = '/wrong:/nightly/lib'
            else: new['roles']['metadata'] = copy.deepcopy(new['roles']['finalizer'])
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                run.generation_recipe(recipe, old, new)

    def tool_values(self):
        _, _, new = self.generations()
        generation = new['prerequisites']
        finalizer = types.SimpleNamespace(PATCHES={'rpo': {'digest': 'source'}}, PRIOR_PIN={'digest': 'prior'},
            TOOL_PHASES={'metadata', 'finalizer-build', 'finalizer-build-tests', 'finalizer-list',
                         'finalizer-ignored-list', 'finalizer-tests'})
        cpu = dict(qualified_generation={'ordinary': 'actual'}, required_test_names={'pliron': ['new']},
            artifacts={name: dict(path=str(run.RPO_CPU / 'target' / name), bytes=1, sha256='a' * 64)
                for name in ('compiler-tests', 'pliron-tests', 'fe2o3-rustc-extract',
                             'librustc_codegen_fe2o3.so', 'librustc_codegen_fe2o3.rlib')},
            raw={'sources-before.json': {'digest': 'source'}})
        common = dict(passed=True, error=None, postcheck_errors=[], gpu_execution=False, production_authority=False,
            compiler_cpu=copy.deepcopy(generation['cpu']), compiler_owner=copy.deepcopy(generation['cpu_owner']),
            patches=copy.deepcopy(finalizer.PATCHES), qualified_generation=copy.deepcopy(cpu['qualified_generation']))
        owner = dict(copy.deepcopy(common), schema='fe2o3-p228-rpo-finalizer-tools-owned-result-v1',
            completion=copy.deepcopy(generation['tools']))
        tools = dict(copy.deepcopy(common), schema='fe2o3-p228-rpo-finalizer-tools-result-v1',
            package=copy.deepcopy(run.TOOLS_PACKAGE), compiler_artifacts=copy.deepcopy(cpu['artifacts']),
            source_snapshot=copy.deepcopy(cpu['raw']['sources-before.json']),
            compiler_required_test_names=copy.deepcopy(cpu['required_test_names']), prior_finalizer=finalizer.PRIOR_PIN,
            phases={name: dict(exit_code=0, reason=None, group_absent=True) for name in finalizer.TOOL_PHASES},
            artifacts={name: new['roles'][name] for name in ('finalizer', 'finalizer-tests', 'metadata')},
            checked_lowering=False, actual_capture_join=False, fresh_hsaco_emitted=False,
            numerical_acceptance=False, performance_claim=False, full_model_acceptance=False)
        return cpu, tools, owner, generation, finalizer

    def test_finalizer_gate_accepts_explicit_actual_generation_shape(self):
        run.tools_gate(*self.tool_values())

    def test_finalizer_gate_rejects_mixed_generation_or_completion(self):
        for key in ('compiler_cpu', 'compiler_owner', 'patches', 'qualified_generation', 'package',
                    'compiler_artifacts', 'source_snapshot', 'compiler_required_test_names', 'prior_finalizer'):
            args = list(self.tool_values()); args[1][key] = {}
            with self.subTest(key=key), self.assertRaises(RuntimeError): run.tools_gate(*args)
        args = list(self.tool_values()); args[2]['completion'] = {}
        with self.assertRaises(RuntimeError): run.tools_gate(*args)

    def test_finalizer_gate_refuses_failed_or_authoritative_tools(self):
        for key, wrong in (('passed', False), ('postcheck_errors', ['changed']), ('phases', {}),
                           ('gpu_execution', True), ('actual_capture_join', True), ('numerical_acceptance', True)):
            args = list(self.tool_values()); args[1][key] = wrong
            with self.subTest(key=key), self.assertRaises(RuntimeError): run.tools_gate(*args)
        args = list(self.tool_values()); args[1]['phases']['metadata']['group_absent'] = False
        with self.assertRaises(RuntimeError): run.tools_gate(*args)

    def test_finalizer_gate_rejects_tool_outside_actual_rpo_target(self):
        args = list(self.tool_values()); args[1]['artifacts']['metadata']['path'] = '/old/target/debug/metadata'
        with self.assertRaises(RuntimeError): run.tools_gate(*args)

    def test_future_finalizer_hashes_are_mandatory_before_loading(self):
        for sha, owner in ((None, 'a' * 64), ('a' * 64, None), ('', 'a' * 64), ('a' * 64, 'not-a-sha')):
            with self.subTest(sha=sha, owner=owner), self.assertRaises(RuntimeError):
                run.rpo_generation(None, None, None, None, sha, owner)


if __name__ == '__main__':
    unittest.main(verbosity=2)
