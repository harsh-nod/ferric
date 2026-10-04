"""Synthetic prefix-only admission tests; no compiler or native execution."""
import copy
from pathlib import Path
import unittest
from unittest.mock import patch

import intake as I
import prefix_contracts as C
from test_intake import pin, request_fixture



def generation_fixture(predecessor):
    root = I.E / 'rpo-compiler-cpu-v228-v2'
    products = {name: pin(str(root / 'target/debug' / name)) for name in (
        'pliron-tests', 'compiler-tests', 'fe2o3-rustc-extract',
        'librustc_codegen_fe2o3.so', 'librustc_codegen_fe2o3.rlib')}
    raw = {name: pin(str(root / name)) for name in ('sources-before.json', 'sources-after.json')}
    qualified = dict(completion=predecessor['prerequisites']['cpu'], owner=predecessor['prerequisites']['cpu_owner'])
    common = dict(passed=True, error=None, postcheck_errors=[], gpu_execution=False, production_authority=False)
    cpu = dict(copy.deepcopy(common), schema='fe2o3-p228-rpo-compiler-cpu-result-v1',
        source_unchanged=True, artifacts=products, raw=raw, qualified_generation=qualified,
        tests=dict(pliron=dict(passed=1504, ignored=1), compiler=dict(passed=1196, ignored=24)),
        patch=pin('rpo-source-manifest'), package=pin('rpo-cpu-manifest'), required_test_names={'pliron': ['test']})
    owned = dict(exit_code=0, reason=None, cleanup_signalled=False,
        owned_groups_absent=True, owned_processes_reaped=True)
    owner = dict(copy.deepcopy(common), schema='fe2o3-p228-rpo-compiler-owned-result-v1',
        owned=copy.deepcopy(owned), completion=I.RPO_PREREQUISITES['cpu'])
    tool_common = dict(copy.deepcopy(common), compiler_cpu=I.RPO_PREREQUISITES['cpu'],
        compiler_owner=I.RPO_PREREQUISITES['cpu_owner'], qualified_generation=qualified, patches={'rpo': cpu['patch']})
    tools = dict(copy.deepcopy(tool_common), schema='fe2o3-p228-rpo-finalizer-tools-result-v1',
        artifacts={name: pin(str(root / 'target/debug/examples' / name))
            for name in ('finalizer', 'finalizer-tests', 'metadata')},
        compiler_artifacts=products, source_snapshot=raw['sources-before.json'],
        compiler_required_test_names=cpu['required_test_names'], package=pin('rpo-tools-manifest'),
        raw={'sources-after.json': pin('rpo-finalizer-tools-v228-v2/sources-after.json')},
        tests=dict(passed=190, ignored=15))
    tools_owner = dict(copy.deepcopy(tool_common), schema='fe2o3-p228-rpo-finalizer-tools-owned-result-v1',
        owned=copy.deepcopy(owned), completion=I.RPO_PREREQUISITES['tools'])
    roles = {name: products[name] for name in ('compiler-tests', 'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so')}
    roles.update(tools['artifacts'])
    generation = dict(source=str(root / 'source/fe2o3'), target=str(root / 'target'), roles=roles,
        prerequisites=copy.deepcopy(I.RPO_PREREQUISITES), compiler_products=products, patches=tools['patches'],
        qualified_generation=qualified, predecessor=predecessor, compiler_package=cpu['package'],
        finalizer_package=tools['package'], backend_dynamic_library=dict(products['librustc_codegen_fe2o3.so'],
            path=str(root / 'target/debug/deps/librustc_codegen_fe2o3.so')))
    docs = {I.RPO_PREREQUISITES[key]['path']: value for key, value in
        zip(('cpu', 'cpu_owner', 'tools', 'tools_owner'), (cpu, owner, tools, tools_owner))}
    for record in (*raw.values(), tools['raw']['sources-after.json']):
        docs[record['path']] = {'source': {'sha256': 'ab' * 32}}
    for record in (cpu['package'], cpu['patch'], tools['package']):
        docs[record['path']] = {'synthetic': True}
    return generation, docs


def fixture():
    plan = dict(prefix_cpu=pin('rope-materialized-cpu-v228-v7/complete.json'),
        prefix_emission_complete=pin('rope-indexed-checked-emission-v228-v1/complete.json'),
        prefix_emission_owner=pin('rope-indexed-checked-emission-owner-v228-v1/complete.json'),
        prefix_image=pin('transport/new-prefix.hsaco', 4, 'cd' * 32))
    original = dict(candidate_cpu_receipt=pin('original-cpu'), compiler_complete=pin('original-lower'),
                    compiler_generation=dict(generation='original V7', prerequisites=dict(cpu=pin('ordinary-cpu'), cpu_owner=pin('ordinary-owner'))))
    prior = dict(standalone=dict(provenance=original), runtime=dict(image=pin('original-image')))
    cpu_root = Path(plan['prefix_cpu']['path']).parent
    cpu = dict(schema='ferric-p228-rope-materialized-cpu-result-v1', passed=True, error=None,
        postcheck_errors=[], source_unchanged=True, cpu_arithmetic_only=True, tests_passed=33, tests_ignored=1,
        prior_cpu=original['candidate_cpu_receipt'], prior_lowering=original['compiler_complete'],
        runner=pin('p228-rope-materialized-cpu-v2/run.py'),
        overlay=pin('p228-rope-materialized-source-v2/source-manifest.json'), fixture=str(cpu_root / 'fixture'),
        phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(11)},
        raw={str(i): pin('unused-' + str(i)) for i in range(59)}, tests={})
    for name, passed, ignored in (('prefix_reciprocal_v1', 12, 1), ('rope_materialized_v1', 20, 0), ('exhaustive', 1, 0)):
        cpu['tests'][name] = dict(passed=passed, ignored=ignored,
            names=[name + '::' + str(i) for i in range(passed + ignored)])
    cpu['tests']['exhaustive']['names'] = ['exhaustive_all_significands_in_one_binade']
    for key in ('kernel_entry_host_compiled', 'gpu_execution', 'compiler_hsaco_reproduced',
                'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority'):
        cpu[key] = False
    source = dict(fixture={name: dict(bytes=1, sha256='ab' * 32) for name in I.PREFIX_RUST | I.PREFIX_CHANGED})
    source['fixture'].update({'synthetic-extra-' + str(i): dict(bytes=1, sha256='ab' * 32) for i in range(5)})
    cpu['formatted_sources'] = {name: pin(str(cpu_root / 'fixture' / name)) for name in I.PREFIX_CHANGED}
    cpu['lowering_sources'] = {name: pin(str(cpu_root / 'fixture' / name)) for name in I.PREFIX_RUST}
    generation, docs = generation_fixture(original['compiler_generation'])
    docs[plan['prefix_cpu']['path']] = cpu
    docs[cpu['overlay']['path']] = dict(schema='ferric-p228-rope-materialized-source-proposal-v2')
    for side in ('before', 'after'):
        record = pin(str(cpu_root / ('sources-' + side + '.json')))
        cpu['raw']['sources-' + side + '.json'] = record
        docs[record['path']] = copy.deepcopy(source)
    lower = dict(compiler_generation=generation)
    return plan, cpu, lower, {}, prior, docs, {}


def metadata(image):
    fields = dict(authority='none', object_sha256=image['sha256'], object_bytes=str(image['bytes']),
        entry_symbol_hex=C.SYMBOL.encode().hex(), target='gfx950:xnack-', code_object_version='6',
        explicit_argument_bytes='120', kernarg_segment_bytes='376', kernarg_alignment='8',
        workgroup='64,1,1', max_grid_workgroups='64,1,1', descriptor_sha256='01' * 32,
        canonical_code_object_digest='02' * 32, descriptor_symbol_hex='03')
    return ('fe2o3-finite-join-request-metadata-v1\n' + ''.join(k + ' ' + v + '\n' for k, v in fields.items())).encode()


class RoPEAdmissionTests(unittest.TestCase):
    def test_cpu33_counts_and_unique_names_include_explicit_exhaustive(self):
        _, cpu, *_ = fixture()
        I.prefix_cpu_contract(cpu)
        for mode in ('count', 'ignored', 'phase-count', 'raw-count', 'names', 'exhaustive'):
            bad = copy.deepcopy(cpu)
            if mode == 'count': bad['tests_passed'] = 32
            elif mode == 'ignored': bad['tests_ignored'] = 0
            elif mode == 'phase-count': bad['phases'].pop('0')
            elif mode == 'raw-count': bad['raw'].pop('0')
            elif mode == 'names': bad['tests']['rope_materialized_v1']['names'][-1] = bad['tests']['rope_materialized_v1']['names'][0]
            else: bad['tests']['exhaustive']['ignored'] = 1
            with self.subTest(mode=mode), self.assertRaises(RuntimeError): I.prefix_cpu_contract(bad)

    def test_cpu_failure_or_native_authority_refuses(self):
        _, cpu, *_ = fixture()
        for mode in ('schema', 'failed', 'postcheck', 'boolean-exit', 'timeout', 'live-group', 'gpu', 'kernel', 'numerical'):
            bad = copy.deepcopy(cpu)
            if mode == 'schema': bad['schema'] = 'ferric-p228-rope-materialized-cpu-result-v2'
            elif mode == 'failed': bad['passed'] = False
            elif mode == 'postcheck': bad['postcheck_errors'] = ['source changed']
            elif mode == 'boolean-exit': bad['phases']['0']['exit_code'] = False
            elif mode == 'timeout': bad['phases']['0']['reason'] = 'timeout'
            elif mode == 'live-group': bad['phases']['0']['group_absent'] = False
            else: bad[{'gpu': 'gpu_execution', 'kernel': 'kernel_entry_host_compiled', 'numerical': 'numerical_acceptance'}[mode]] = True
            with self.subTest(mode=mode), self.assertRaises(RuntimeError): I.prefix_cpu_contract(bad)

    def test_only_prefix_changes_while_copy_silu_residual_and_workload_stay_fixed(self):
        request, prior, runtime, out, image, mlp = request_fixture()
        with patch.object(I, 'read', return_value=b''): I.request_check(None, request, prior, runtime, out, image, mlp)
        for mode in ('old-prefix', 'copy', 'silu', 'residual', 'input'):
            bad, selected = copy.deepcopy(request), copy.deepcopy(runtime)
            if mode == 'old-prefix':
                bad['decode']['prefix_image'] = prior['request']['prefix_image']
                selected['image'] = prior['request']['prefix_image']
            elif mode == 'copy': bad['decode']['images']['residual'] = pin('different-copy')
            elif mode == 'silu': bad['decode']['tiles_image'] = pin('different-silu')
            elif mode == 'residual': bad['projection_residual_image'] = pin('different-residual')
            else: bad['decode']['prompt']['tokens'] = pin('different-tokens')
            with patch.object(I, 'read', return_value=b''), self.subTest(mode=mode), self.assertRaises(RuntimeError):
                I.request_check(None, bad, prior, selected, out, image, mlp)

    def test_prefix_descriptor_keeps_existing_120_376_wave_launch_abi(self):
        image = pin('image', 4)
        raw = metadata(image)
        self.assertEqual(C.metadata(raw, image)['explicit_argument_bytes'], '120')
        for old, new in ((b'explicit_argument_bytes 120', b'explicit_argument_bytes 88'),
                         (b'kernarg_segment_bytes 376', b'kernarg_segment_bytes 344'),
                         (b'workgroup 64,1,1', b'workgroup 32,1,1'),
                         (b'authority none', b'authority qualified')):
            with self.subTest(old=old), self.assertRaises(RuntimeError): C.metadata(raw.replace(old, new), image)

    def test_rpo_receipt_and_ordinary_predecessor_pins_are_exact(self):
        _, _, lower, _, prior, docs, _ = fixture()
        generation = lower['compiler_generation']
        predecessor = prior['standalone']['provenance']['compiler_generation']
        with patch.object(I, 'doc', side_effect=lambda _, record, *args: docs[record['path']]):
            self.assertEqual(I.prefix_generation(None, generation, predecessor), generation)
            for mode in ('cpu-pin', 'tools-pin', 'predecessor', 'old-generation'):
                bad = copy.deepcopy(generation)
                if mode == 'cpu-pin': bad['prerequisites']['cpu']['sha256'] = 'ff' * 32
                elif mode == 'tools-pin': bad['prerequisites']['tools_owner']['sha256'] = 'ff' * 32
                elif mode == 'predecessor': bad['predecessor']['generation'] = 'other'
                else: bad = predecessor
                with self.subTest(mode=mode), self.assertRaises(RuntimeError):
                    I.prefix_generation(None, bad, predecessor)

    def test_rpo_selected_roles_alias_packages_and_sources_are_not_relabelled(self):
        _, _, lower, _, prior, docs, _ = fixture()
        generation = lower['compiler_generation']
        predecessor = prior['standalone']['provenance']['compiler_generation']
        for mode in ('role', 'alias', 'source', 'package', 'extra-role'):
            bad = copy.deepcopy(generation)
            if mode == 'role': bad['roles']['finalizer']['sha256'] = 'ff' * 32
            elif mode == 'alias': bad['backend_dynamic_library']['path'] += '.other'
            elif mode == 'source': bad['source'] += '-other'
            elif mode == 'package': bad['compiler_package']['sha256'] = 'ff' * 32
            else: bad['roles']['unknown'] = pin('extra')
            with patch.object(I, 'doc', side_effect=lambda _, record, *args: docs[record['path']]), \
                 self.subTest(mode=mode), self.assertRaises(RuntimeError):
                I.prefix_generation(None, bad, predecessor)

    def test_rpo_receipt_failures_source_drift_and_tool_rebinding_refuse(self):
        _, _, lower, _, prior, original_docs, _ = fixture()
        generation = lower['compiler_generation']
        predecessor = prior['standalone']['provenance']['compiler_generation']
        for mode in ('failed', 'count', 'owner', 'tools-owner', 'source-drift', 'artifact', 'lineage'):
            docs = copy.deepcopy(original_docs)
            cpu = docs[I.RPO_PREREQUISITES['cpu']['path']]
            tools = docs[I.RPO_PREREQUISITES['tools']['path']]
            if mode == 'failed': cpu['passed'] = False
            elif mode == 'count': tools['tests']['passed'] = 189
            elif mode == 'owner': docs[I.RPO_PREREQUISITES['cpu_owner']['path']]['owned']['cleanup_signalled'] = True
            elif mode == 'tools-owner': docs[I.RPO_PREREQUISITES['tools_owner']['path']]['completion'] = pin('wrong-complete')
            elif mode == 'source-drift': docs[tools['raw']['sources-after.json']['path']]['source']['sha256'] = 'ff' * 32
            elif mode == 'artifact': tools['compiler_artifacts'] = {}
            else: tools['compiler_cpu'] = pin('wrong-cpu')
            with patch.object(I, 'doc', side_effect=lambda _, record, *args: docs[record['path']]), \
                 self.subTest(mode=mode), self.assertRaises(RuntimeError):
                I.prefix_generation(None, generation, predecessor)


if __name__ == '__main__':
    unittest.main()
