"""Synthetic policy tests only; no compiler, ELF, subprocess, SSH or GPU calls."""
import ast
import copy
import json
from pathlib import Path
import types
import unittest

import contracts
import run


class PolicyTests(unittest.TestCase):
    def template(self):
        old = run.E / 'row-reciprocal-checked-probe-v228-v7'
        commands = []
        for name in run.STAGES:
            env = dict(CARGO_TARGET_DIR=str(old / 'target'), LD_LIBRARY_PATH='/retained/deps:/nightly/lib',
                       HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
            argv = ['/retained/tool', '--exact', 'old_test']
            if name == 'checked-lowering':
                env['FE2O3_DIAGNOSTIC_SEMANTIC_MIR_PATH_V1'] = str(old / 'prefix-tiles-semantic.bin')
            if name == 'actual-replay':
                env.update(FE2O3_PREFIX_TILE_V6_SEMANTIC_CAPTURE=str(old / 'prefix-tiles-semantic.bin'),
                           FE2O3_PREFIX_TILE_V6_SEMANTIC_CAPTURE_SHA256='@candidate.semantic.sha256')
            if name == 'actual-inert-join':
                env.update(FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_PATH=str(old / 'prefix-tiles.handoff-v3'),
                           FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_SHA256='@candidate.handoff.sha256',
                           FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_BYTES='@candidate.handoff.bytes')
            if name == 'extract-retained':
                argv = [str(run.E / 'row-prefix-tiles-source-v227-v5/extract-retained'),
                        str(old / 'prefix-tiles.handoff-v3'), '@candidate.handoff.bytes', str(old / 'extracted')]
            commands.append(dict(name=name, argv=argv, env=env, cache_cap_bytes=6 << 30,
                                 affinity=[8, 9], nice=10, expected_exit=0, gpu_execution=False,
                                 deadline_seconds=1800 if name in ('checked-lowering', 'actual-replay') else 900))
        return dict(commands=commands, fresh_output=str(old), toolchain_and_retained_tool_pins=[{'unchanged': 1}],
                    compiler_generation={'unchanged': 2}, retained_modules=[{'unchanged': 3}])

    def test_guard_accepts_ordinary_python(self):
        run.guard(0, {})

    def test_guard_refuses_optimized_or_inherited_optimization(self):
        for optimized, env in ((1, {}), (0, {'PYTHONOPTIMIZE': ''}), (0, {'PYTHONOPTIMIZE': '0'})):
            with self.subTest(optimized=optimized, env=env), self.assertRaises(RuntimeError):
                run.guard(optimized, env)

    def test_relocation_changes_only_candidate_output_not_retained_prefix_tools(self):
        old = Path('/old/row-prefix-tiles')
        value = ['/retained/row-prefix-tiles-source/extract-retained', str(old / 'prefix-tiles-semantic.bin'), str(old)]
        self.assertEqual(run.rewrite(value, old, Path('/fresh')), [value[0], '/fresh/mlp-tiles-semantic.bin', '/fresh'])

    def test_recipe_selects_actual_mlp_replay_finalizer_and_capture_names(self):
        recipe = run.make_recipe(self.template(), [])
        commands = {row['name']: row for row in recipe['commands']}
        self.assertEqual(commands['actual-replay']['argv'][2], run.REPLAY)
        self.assertEqual(commands['actual-inert-join']['argv'][2], run.JOIN)
        self.assertEqual(commands['emit']['argv'][1], '--wave-mlp-tiles-v2')
        env = commands['actual-replay']['env']
        self.assertEqual(env['FE2O3_MLP_TILE_V2_SEMANTIC_CAPTURE'], str(run.OUT / 'mlp-tiles-semantic.bin'))
        self.assertNotIn('FE2O3_PREFIX_TILE_V6_SEMANTIC_CAPTURE', env)

    def test_recipe_preserves_tool_generation_resources_and_template_bytes(self):
        template = self.template()
        before = copy.deepcopy(template)
        recipe = run.make_recipe(template, [])
        self.assertEqual(template, before)
        for key in ('toolchain_and_retained_tool_pins', 'compiler_generation', 'retained_modules'):
            self.assertEqual(recipe[key], before[key])
        for old, new in zip(before['commands'], recipe['commands']):
            for key in ('deadline_seconds', 'cache_cap_bytes', 'affinity', 'nice', 'gpu_execution', 'expected_exit'):
                self.assertEqual(old[key], new[key])
            self.assertEqual(new['env']['LD_LIBRARY_PATH'], old['env']['LD_LIBRARY_PATH'])

    def test_recipe_rejects_missing_reordered_or_duplicate_stage(self):
        for kind in range(3):
            template = self.template()
            if kind == 0:
                template['commands'].pop()
            elif kind == 1:
                template['commands'].reverse()
            else:
                template['commands'][1] = copy.deepcopy(template['commands'][0])
            with self.subTest(kind=kind), self.assertRaises(RuntimeError):
                run.make_recipe(template, [])

    def test_recipe_rejects_widened_or_gpu_bounds(self):
        for key, value in (('cache_cap_bytes', 8 << 30), ('gpu_execution', True), ('nice', 0), ('affinity', [0, 1])):
            template = self.template()
            template['commands'][0][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                run.make_recipe(template, [])

    def test_one_exact_ignored_callback_must_actually_pass(self):
        raw = ('test ' + run.REPLAY + ' ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n').encode()
        contracts.exact_test(raw, run.REPLAY)
        for bad in (raw.replace(b'1 passed', b'0 passed'), raw.replace(b'0 ignored', b'1 ignored'),
                    raw.replace(run.REPLAY.encode(), b'wrong'), raw + raw):
            with self.subTest(raw=bad), self.assertRaises(RuntimeError):
                contracts.exact_test(bad, run.REPLAY)

    def metadata(self):
        image = dict(sha256='1' * 64, bytes=1234)
        value = dict(authority='none', object_sha256=image['sha256'], object_bytes=str(image['bytes']),
                     entry_symbol_hex=run.SYMBOL.encode().hex(), target='gfx950:xnack-', code_object_version='6',
                     explicit_argument_bytes='88', kernarg_segment_bytes='344', kernarg_alignment='8',
                     workgroup='64,1,1', max_grid_workgroups='64,1,1', descriptor_sha256='2' * 64,
                     canonical_code_object_digest='3' * 64, descriptor_symbol_hex='6162')
        return image, value

    def encode(self, header, fields):
        return (header + '\n' + ''.join(key + ' ' + value + '\n' for key, value in fields.items())).encode()

    def test_metadata_preserves_eleven_root_abi(self):
        image, value = self.metadata()
        self.assertEqual(contracts.metadata(self.encode('fe2o3-finite-join-request-metadata-v1', value), image), value)

    def test_metadata_rejects_changed_image_geometry_abi_or_authority(self):
        image, original = self.metadata()
        for key, value in (('object_sha256', '9' * 64), ('explicit_argument_bytes', '120'),
                           ('kernarg_segment_bytes', '376'), ('workgroup', '128,1,1'),
                           ('max_grid_workgroups', '65,1,1'), ('authority', 'launch')):
            fields = dict(original, **{key: value})
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                contracts.metadata(self.encode('fe2o3-finite-join-request-metadata-v1', fields), image)

    def test_metadata_refuses_duplicate_unknown_and_wrong_header(self):
        image, value = self.metadata()
        raw = self.encode('fe2o3-finite-join-request-metadata-v1', value)
        for bad in (raw + b'authority none\n', raw + b'extra value\n', raw.replace(b'metadata-v1', b'metadata-v2')):
            with self.subTest(raw=bad), self.assertRaises(RuntimeError):
                contracts.metadata(bad, image)

    def emission(self):
        pin = dict(sha256='4' * 64, bytes=100)
        outputs = {name: dict(pin) for name in ('source.handoff-v3', 'compiler.handoff-v2', 'artifact.hsaco')}
        config = {f'file{i}.bc': '5' * 64 for i in range(9)}
        fields = dict(status='PASS', authority='none', generic_proofs='unsupported', runtime_requirements='required_undischarged',
            unresolved_requirement_count='8', external_provider_count='0', builtin_device_library_provider='gfx950-ocml-rocm-7.2.1-v1',
            builtin_device_library_import_count='1', builtin_device_library_file_count='9', worker_timeout_seconds_per_pass='120',
            worker_stdout_cap=str(40 << 20), worker_stderr_cap=str(64 << 10), hsaco_cap=str(32 << 20),
            worker_build=json.dumps('worker'), llvm_build=json.dumps('llvm'), observer_compile_replay_finalize_wall_ms='42',
            builtin_device_library_manifest='6' * 64)
        for key in ('outer_v3_content', 'outer_v3_domain_identity', 'capsule_content', 'pair_binding_content',
                    'nested_v2_content', 'worker_executable', 'bootstrap_request', 'bootstrap_response',
                    'replay_request', 'replay_response', 'finalized_hsaco_content'):
            fields[key] = pin['sha256'] + ' 100'
        raw = self.encode('wave_mlp_tile_engineering_emission_v2', fields)
        raw += b'builtin_device_library_import __ocml_exp_f32\n'
        raw += ''.join('builtin_device_library_file ' + name + ' ' + digest + '\n' for name, digest in config.items()).encode()
        return raw, outputs, pin, config

    def test_emission_binds_exact_artifacts_and_keeps_requirements_open(self):
        raw, outputs, worker, config = self.emission()
        found = contracts.emission(raw, outputs, worker, 'worker', 'llvm', config)
        self.assertEqual(found['unresolved_requirement_count'], '8')
        self.assertEqual(found['authority'], 'none')

    def test_emission_refuses_budget_or_authority_relaxation(self):
        raw, outputs, worker, config = self.emission()
        for old, new in ((b'authority none', b'authority launch'), (b'worker_timeout_seconds_per_pass 120', b'worker_timeout_seconds_per_pass 999'),
                         (b'unresolved_requirement_count 8', b'unresolved_requirement_count 0')):
            with self.subTest(old=old), self.assertRaises(RuntimeError):
                contracts.emission(raw.replace(old, new), outputs, worker, 'worker', 'llvm', config)

    def test_emission_refuses_import_provider_or_image_drift(self):
        raw, outputs, worker, config = self.emission()
        bad_outputs = copy.deepcopy(outputs)
        bad_outputs['artifact.hsaco']['bytes'] += 1
        for body, images, files in ((raw.replace(b'__ocml_exp_f32', b'__ocml_sqrt_f32'), outputs, config),
                                   (raw, bad_outputs, config), (raw, outputs, dict(config, extra='0' * 64))):
            with self.subTest(body=body), self.assertRaises(RuntimeError):
                contracts.emission(body, images, worker, 'worker', 'llvm', files)

    def test_cpu_gate_refuses_failed_or_wrong_total_before_any_io(self):
        value = dict(schema='ferric-p228-silu-materialized-cpu-result-v1', passed=True, error=None, postcheck_errors=[],
                     source_unchanged=True, tests_passed=38, tests_ignored=0, cpu_arithmetic_only=True)
        for key, wrong in (('passed', False), ('tests_passed', 37), ('tests_ignored', 1), ('source_unchanged', False)):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                run.cpu_gate(types.SimpleNamespace(), None, dict(value, **{key: wrong}), None, None, None, None, None)

    def test_fresh_target_is_disjoint_from_preserved_targets(self):
        target = run.OUT / 'target'
        self.assertTrue(all(not target.is_relative_to(old) and not old.is_relative_to(target) for old in run.OLD_TARGETS))

    def test_both_completion_scopes_recheck_source_map_bytes_and_stamps(self):
        tree = ast.parse(Path(run.__file__).read_text())
        for scope in ('body', 'run'):
            with self.subTest(scope=scope):
                function = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == scope)
                rows = [node for node in ast.walk(function) if isinstance(node, ast.Tuple)
                        and len(node.elts) == 3 and isinstance(node.elts[0], ast.Constant)
                        and node.elts[0].value == 'source_maps']
                self.assertEqual(len(rows), 1)
                pins = [{'path': '/retained/map', 'sha256': '1' * 64}]
                original = ({'source.rs': '2' * 64}, [{'pin': pins[0], 'stamp': [1, 2, 3]}])
                current = [copy.deepcopy(original)]
                calls = []

                def observe(value):
                    self.assertIs(value, pins)
                    calls.append(value)
                    if isinstance(current[0], Exception):
                        raise current[0]
                    return current[0]

                context = dict(merged=original[0], source_records=original[1], inputs={'source_maps': pins})
                name, expected, operation = eval(compile(ast.Expression(rows[0]), '<postcheck>', 'eval'),
                                                {'c': context, 'p': types.SimpleNamespace(source_maps=observe)})
                self.assertEqual(name, 'source_maps')
                self.assertEqual(operation(), expected)
                current[0] = copy.deepcopy(original)
                current[0][1][0]['stamp'][-1] += 1
                self.assertNotEqual(operation(), expected)
                current[0] = RuntimeError('changed or deleted map bytes')
                with self.assertRaisesRegex(RuntimeError, 'changed or deleted'):
                    operation()
                self.assertEqual(len(calls), 3)

    def fixture(self):
        return [dict(destination=name, source=dict(path='/qualified/' + name,
                    bytes=10, sha256='1' * 64)) for name in sorted(run.FIXTURE_FILES)]

    def test_actual_cpu_path_is_a_closed_fresh_namespace(self):
        path = run.E / 'silu-materialized-cpu-v228-v1/complete.json'
        self.assertEqual(run.cpu_argument(str(path), 'a' * 64), path)
        self.assertEqual(run.cpu_argument(path, 'a' * 64), path)

    def test_actual_cpu_path_refuses_old_alias_escape_and_wrong_basename(self):
        for suffix in ('down2-cpu-v228-v1/complete.json', 'silu-materialized-cpu-v228-v0/complete.json',
                       'silu-materialized-cpu-v228-v1/failed.json', 'x/../silu-materialized-cpu-v228-v1/complete.json'):
            with self.subTest(suffix=suffix), self.assertRaises(RuntimeError):
                run.cpu_argument(run.E / suffix, 'a' * 64)
        with self.assertRaises(RuntimeError):
            run.cpu_argument('/tmp/silu-materialized-cpu-v228-v1/complete.json', 'a' * 64)

    def test_actual_cpu_digest_is_required_without_guessed_default(self):
        for digest in (None, True, '', 'a' * 63, 'A' * 64, '0' * 65):
            with self.subTest(digest=digest), self.assertRaises(RuntimeError):
                run.cpu_argument(run.E / 'silu-materialized-cpu-v228-v1/complete.json', digest)

    def test_fixture_is_exactly_five_rust_and_seven_total_files(self):
        rows = self.fixture()
        self.assertEqual(run.fixture_contract(list(reversed(rows))), rows)
        self.assertEqual(len([row for row in rows if row['destination'].endswith('.rs')]), 5)
        self.assertEqual(run.RUST_FILES, {'src/lib.rs', 'src/wave_numerics_v1.rs', 'src/mlp_numerics_v1.rs',
                                        'src/mlp_tile_numerics_v2.rs', 'src/mlp_silu_materialized_numerics_v1.rs'})

    def test_fixture_refuses_missing_extra_duplicate_and_test_source(self):
        original = self.fixture()
        malformed = [original[:-1], original + [original[0]], original[:-1] + [original[0]]]
        replaced = copy.deepcopy(original)
        replaced[0]['destination'] = 'tests/mlp_silu_materialized_v1.rs'
        malformed.append(replaced)
        for rows in malformed:
            with self.subTest(rows=rows), self.assertRaises(RuntimeError):
                run.fixture_contract(rows)

    def test_fixture_refuses_reusing_one_body_path_for_two_includes(self):
        rows = self.fixture()
        rows[0]['source'] = rows[1]['source']
        with self.assertRaises(RuntimeError):
            run.fixture_contract(rows)

    def test_pin_exact_joins_extent_and_digest_not_only_path(self):
        record = dict(path='/qualified/source.rs', bytes=10, sha256='a' * 64)
        pins = types.SimpleNamespace(pin=lambda path, sha: dict(record))
        self.assertEqual(run.pin_exact(pins, record), record)
        for changed in (dict(record, bytes=11), dict(record, sha256='b' * 64),
                        dict(record, bytes=True), dict(record, authority='launch')):
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                run.pin_exact(pins, changed)

    def test_cpu_cohort_retains_all_sixteen_phases_and_eighty_six_raw_records(self):
        self.assertEqual(len(run.CPU_PHASES), 16)
        self.assertEqual(len(set(run.CPU_PHASES)), 16)
        self.assertEqual(len(run.CPU_RAW), 86)
        self.assertEqual(list(run.CPU_PHASES[:4]), ['rustfmt', 'rustfmt-check', 'metadata', 'build-tests'])
        for name in run.TARGET_COUNTS:
            for suffix in ('-list', '-ignored-list', ''):
                self.assertIn(name + suffix, run.CPU_PHASES)
                self.assertIn(name + suffix + '-command.json', run.CPU_RAW)
        self.assertEqual(sum(run.TARGET_COUNTS.values()), 38)
        self.assertEqual(len(run.CLAIMED_TESTS), 8)

    def transition(self):
        old = {'src/old' + str(i) + '.rs': dict(bytes=10, sha256='a' * 64) for i in range(31)}
        additions = ['src/finite_mlp_tiles_silu_materialized_v1.rs',
                     'src/mlp_silu_materialized_numerics_v1.rs', 'tests/mlp_silu_materialized_v1.rs']
        overlay = dict(files=[dict(path=run.DEVICE + '/' + name, after=dict(bytes=11, sha256='b' * 64))
                              for name in additions])
        unformatted = {**copy.deepcopy(old), **{name: dict(bytes=11, sha256='b' * 64) for name in additions}}
        before = dict(fixture={**copy.deepcopy(old), **{name: dict(bytes=12, sha256='c' * 64) for name in additions}})
        return before, copy.deepcopy(before), unformatted, old, overlay

    def test_cpu_source_transition_allows_only_formatting_the_three_additions(self):
        run.source_transition(*self.transition())

    def test_cpu_source_transition_rejects_changed_old_missing_new_and_post_drift(self):
        for kind in ('old', 'missing', 'drift', 'unformatted'):
            before, after, unformatted, prior, overlay = self.transition()
            if kind == 'old':
                before['fixture']['src/old0.rs']['sha256'] = 'd' * 64
                after = copy.deepcopy(before)
            elif kind == 'missing':
                before['fixture'].pop('tests/mlp_silu_materialized_v1.rs')
                after = copy.deepcopy(before)
            elif kind == 'drift':
                after['fixture']['src/mlp_silu_materialized_numerics_v1.rs']['sha256'] = 'd' * 64
            else:
                unformatted['src/mlp_silu_materialized_numerics_v1.rs']['bytes'] += 1
            with self.subTest(kind=kind), self.assertRaises(RuntimeError):
                run.source_transition(before, after, unformatted, prior, overlay)


if __name__ == '__main__':
    unittest.main()
