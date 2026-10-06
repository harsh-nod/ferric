"""Bounded data-only retention of an actual terminal atomic-load alias CPU attempt."""
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import signal
import stat
import sys
import tarfile


ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-atomic-load-alias-cpu-v228-v1')
ARCHIVE = ROOT.parent / 'guarded-mlp-atomic-load-alias-cpu-evidence-v228-v1.tar.gz'
CONTROLLER_SHA = 'd3dfe30c4dd2fd216d6d5527fae0ec573083d7c032651bfb368c7ab7156342d4'
HELPER_SHA = 'ba127f1057546c0ce6e57fb78832c3e774c1108a4f7c5a2421f4aad7f51108be'
BASE_HELPER_SHA = 'ba127f1057546c0ce6e57fb78832c3e774c1108a4f7c5a2421f4aad7f51108be'
PROPOSAL_SHA = 'aedc4e4595c843c87586414da9fe120a5228e3655469e73ad878f0ff7a4d5a6d'
INPUT_SHA = '7e15a68f3a7e0e6d521db06906af4f8bbbf9b538ebd05c071763de6a0a8740d4'  # Actual assembled input.
TERMINAL_SHA = '18925ee6216977c561523f84a1899db379fcaaff99a050cc750ef32befebb708'  # Observed complete terminal.
BASE_COMPLETE_SHA = 'bd1fa4827483cdf6ca6a0470aacd6826ddab007e8f129611b9725d3e0cdf8445'
BASE_SOURCES_SHA = '37355332ab519512f28ebe58f5d9f55f7c9774f8d14796db2519a5b5168f25ba'
BASE_INPUT_SHA = '3c60b907c38c1b61d645ec303b6099843e28e8b46148920f7486a364f0faba12'
BASE_FAILURE_ROOT = ROOT.parent / 'guarded-mlp-combined-state-lowering-v228-v3'
BASE_FAILURE_SHA = 'd072928318581d8f492d296dfa4cc4a1f42865fbd3a7d95fdac01dddac7e5ab0'
BASE_FAILURE_STDERR_SHA = '5d3adcd162d465f76cca68e5590494692be075905070ae9a9795d11e60fe9b75'
BASE_FAILURE_DIAGNOSTIC = (
    b'fe2o3 rustc extraction: production compilation compiler-module handoff failed: '
    b'compiler descriptor construction failed: production descriptor evidence has an internal '
    b'formal alias obligation not discharged by Rust ownership mismatch')
MAX_FILE, MAX_TOTAL, MAX_MEMBERS = 64 << 20, 64 << 20, 1024


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def pin(path, limit=MAX_FILE):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input required')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= limit, 'bounded ordinary file required')
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file identity changed before read')
        body = stream.read(limit + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file identity changed during read')
    require(len(body) == before.st_size and stamp(path.lstat()) == stamp(before), 'file changed after read')
    return body, dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


KERNEL_IR_TARGET_NAMES = [
    "fe2o3_kernel_ir",
    "amdgpu_diagnostics",
    "canonical_kir_v6",
    "cast_verification",
    "checked_binary_v6",
    "control_flow_bounds",
    "device_constants_v2",
    "effect_extraction",
    "float_operations",
    "formal_memory_obligations",
    "frozen_wire_compatibility",
    "g4_synchronization",
    "inert_v12",
    "inline_assembly",
    "integer_switch",
    "interprocedural_effects",
    "launch_kernel_v2",
    "matrix_operations",
    "memory_intrinsics_v10",
    "memory_safety_v2",
    "operand_visitation",
    "pointer_access_cast_v11",
    "region_effect_analysis",
    "region_effect_model",
    "scalar_ops_v2",
    "semantic_operations",
    "standard_atomics",
    "synchronization_v2",
    "terminator_operand_visitation",
    "v12_effect_consumer_closure",
    "vector_v12",
    "verification",
    "verification_contract_v12",
    "wave_operations",
    "wave_operations_v9",
    "wire",
    "wire_preflight_order",
    "wire_v5",
    "wire_v6",
    "wire_v7",
    "write_only_v9",
    "formal_atomic_load_alias_v1"
]
DESCRIPTOR_ATOMIC_TESTS = [
    "compiler_descriptor::tests::atomic_slice_descriptor_does_not_discharge_aliases_from_shared_rust_ownership",
    "compiler_descriptor::tests::atomic_slice_descriptor_requires_exact_readwrite_u32_global_slice"
]


def inventory(body):
    text = body.decode('utf-8')
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in text, 'ambiguous libtest inventory')
    return sorted(names)


def full_suite_outcomes(body, expected_names, expected_ignored):
    """Replay a full default suite against separately authenticated inventories."""
    for names in (expected_names, expected_ignored):
        require(isinstance(names, (list, tuple))
                and all(isinstance(name, str) and re.fullmatch(r'[A-Za-z0-9_:]+', name)
                        for name in names)
                and len(names) == len(set(names)), 'invalid expected libtest roster')
    require(expected_names and set(expected_ignored) <= set(expected_names),
            'ignored roster must be a subset of the full inventory')
    text = body.decode('utf-8')
    pattern = re.compile(r'test ([A-Za-z0-9_:]+)(?: - should panic)? '
                         r'\.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?')
    progress_pattern = re.compile(
        r'test ([A-Za-z0-9_:]+) has been running for over 60 seconds')
    expected, ignored = set(expected_names), set(expected_ignored)
    named, completed, progress_names = [], set(), set()
    summary_seen = False
    for line in text.splitlines():
        if line.startswith('test result:'):
            summary_seen = True
            continue
        if not line.startswith('test '):
            continue
        progress = progress_pattern.fullmatch(line)
        if progress is not None:
            name = progress[1]
            require(name in expected and name not in ignored
                    and name not in completed and name not in progress_names
                    and not summary_seen, 'invalid named libtest progress notice')
            progress_names.add(name)
            continue
        match = pattern.fullmatch(line)
        require(match is not None, 'malformed named libtest result')
        named.append((match[1], match[2]))
        completed.add(match[1])
    require(len(named) == len(expected_names)
            and sorted(name for name, _ in named) == sorted(expected_names),
            'full inventory must appear exactly once')
    ignored = set(expected_ignored)
    require(all(status == ('ignored' if name in ignored else 'ok') for name, status in named),
            'full-suite status differs from the authenticated ignored roster')
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                           r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', text, re.M)
    require(sum(line.startswith('test result:') for line in text.splitlines()) == 1
            and summaries == [('ok', str(len(expected_names) - len(ignored)), '0',
                               str(len(ignored)), '0', '0')],
            'full-suite summary must match every named outcome without filtering')
    return dict(names=sorted(expected_names), passed=len(expected_names) - len(ignored),
                failed=0, ignored=len(ignored), filtered_out=0,
                ignored_names=sorted(ignored), named_outcomes=dict(sorted(named)))



def focused_outcomes(body, selected, total):
    text = body.decode('utf-8')
    named = re.findall(r'^test ([A-Za-z0-9_:]+)(?: - should panic)? \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                           r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', text, re.M)
    require(selected and len(named) == len(selected)
            and sorted(name for name, _ in named) == sorted(selected)
            and all(status == 'ok' for _, status in named)
            and summaries == [('ok', str(len(selected)), '0', '0', '0', str(total - len(selected)))],
            'selected named tests must all pass exactly once, with no hidden ignored cases')
    return dict(names=sorted(selected), passed=len(selected), failed=0, ignored=0,
                filtered_out=total-len(selected))


def verify_test_census(receipt, bodies, baseline):
    """Join successful recorded scopes to raw bytes; never promote an unrecorded failed scope."""
    wrapper = ['/usr/bin/prlimit', '--as=12884901888', '--cpu=1200',
               '--fsize=1073741824', '--core=0', '--']
    roles = ['kernel-ir-' + name for name in KERNEL_IR_TARGET_NAMES]
    old_roles = ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')
    require(receipt['kernel_ir_target_names'] == sorted(KERNEL_IR_TARGET_NAMES)
            and receipt['kernel_ir_historical_raw_census_available'] is False
            and receipt['focused_repeats_are_not_unique_tests'] is True
            and receipt['historical_product_pins_postchecked'] is False,
            'fresh kernel-IR census and repeated-test semantics')
    expected_phases = ['rustc-version', 'alias-rustfmt-check', 'metadata', 'kernel-ir-build-tests']
    expected_phases += [role + suffix for role in roles for suffix in ('-list', '-ignored', '-tests')]
    expected_phases += ['atomic-load-alias']
    for phase in baseline['phases'][2:]:
        expected_phases.append(phase['label'])
        if phase['label'] == 'compiler-tests':
            expected_phases += ['descriptor-atomic-negative-0', 'descriptor-atomic-negative-1']
    require(len(expected_phases) == len(set(expected_phases)) == 176
            and [row['label'] for row in receipt['phases']]
                == expected_phases[:len(receipt['phases'])], 'exact ordered actual phase prefix')
    phases = {row['label']: row for row in receipt['phases']}
    inventories, ignored = receipt['test_inventories'], receipt['ignored_inventories']
    require(set(inventories) == set(ignored) <= set(roles) | set(old_roles),
            'closed full/ignored inventory roles')
    for role, names in inventories.items():
        require(names and names == inventory(bodies['evidence/' + role + '-list.stdout'])
                and ignored[role] == inventory(bodies['evidence/' + role + '-ignored.stdout'])
                and set(ignored[role]) <= set(names), 'literal fresh compiled inventory')
        if role in old_roles:
            require(names == baseline['test_inventories'][role]
                    and ignored[role] == baseline['ignored_inventories'][role],
                    'unchanged qualified historical inventories')
    alias_role = 'kernel-ir-formal_atomic_load_alias_v1'
    if alias_role in inventories:
        require(inventories[alias_role] == receipt['atomic_load_alias_tests']
                and ignored[alias_role] == [], 'nine exact fresh nonignored alias cases')
    tests = receipt['tests']
    expected_scopes = set(baseline['tests']) | set(roles) | {
        'atomic-load-alias', 'descriptor-atomic-negative-0', 'descriptor-atomic-negative-1'}
    require(len(expected_scopes) == 77 and set(tests) <= expected_scopes,
            'closed actual successful scope prefix')
    artifacts = receipt['artifacts']
    require(set(artifacts) <= set(roles) | set(old_roles) | {'backend', 'backend-rlib', 'extractor'},
            'closed artifact metadata role set')
    for label, result in tests.items():
        phase_label = label + '-tests' if label in roles or label in ('compiler', 'pliron') else label
        phase = phases[phase_label]
        require(type(phase['argv']) is list and len(phase['argv']) > len(wrapper)
                and phase['argv'][:len(wrapper)] == wrapper,
                'exact qualification resource-limit wrapper: ' + phase_label)
        require(phase['exit_code'] == 0 and phase['natural_exit'] is True
                and phase['reaped'] is True and phase['process_group_absent'] is True
                and phase['forced_cleanup'] is False and phase['timed_out'] is False
                and phase['exception'] is None and phase['storage_failure'] is None,
                'recorded successful scope requires a clean natural leaf')
        body = bodies['evidence/' + phase_label + '.stdout']
        if label in roles or label in ('compiler', 'pliron'):
            role = label if label in roles else label + '-lib'
            actual = full_suite_outcomes(body, inventories[role], ignored[role])
            expected_argv = [artifacts[role]['pin']['path'], '--test-threads=2']
        elif label == 'atomic-load-alias':
            role = alias_role
            actual = focused_outcomes(body, receipt['atomic_load_alias_tests'], len(inventories[role]))
            expected_argv = [artifacts[role]['pin']['path'], 'atomic_load_alias_', '--test-threads=1']
        elif label.startswith('descriptor-atomic-negative-'):
            role = 'compiler-lib'
            name = DESCRIPTOR_ATOMIC_TESTS[int(label.rsplit('-', 1)[1])]
            require(name in inventories[role] and name not in ignored[role],
                    'unchanged existing descriptor negative remains nonignored')
            actual = focused_outcomes(body, [name], len(inventories[role]))
            expected_argv = [artifacts[role]['pin']['path'], name, '--exact', '--test-threads=1']
        else:
            prior_phase = next(row for row in baseline['phases'] if row['label'] == label)
            require(type(prior_phase['argv']) is list and len(prior_phase['argv']) > len(wrapper)
                    and prior_phase['argv'][:len(wrapper)] == wrapper,
                    'exact historical resource-limit wrapper: ' + label)
            role = next(role for role in old_roles
                        if baseline['artifacts'][role]['pin']['path'] == prior_phase['argv'][len(wrapper)])
            actual = focused_outcomes(body, baseline['tests'][label]['names'], len(inventories[role]))
            expected_argv = [artifacts[role]['pin']['path'], *prior_phase['argv'][len(wrapper) + 1:]]
        require(result == actual and phase['argv'] == wrapper + expected_argv,
                'exact raw named outcomes and executable recipe: ' + label)
    require(receipt['tests_passed'] == sum(row['passed'] for row in tests.values())
            and receipt['tests_ignored'] == sum(row['ignored'] for row in tests.values())
            and receipt['kernel_ir_tests_passed'] == sum(tests[role]['passed'] for role in roles if role in tests)
            and receipt['kernel_ir_tests_ignored'] == sum(tests[role]['ignored'] for role in roles if role in tests),
            'actual dynamic kernel-IR and whole-run outcome sums')
    require(receipt['limits'] == baseline['limits'], 'unchanged bounded qualification process limits')
    if receipt['passed']:
        require(set(tests) == expected_scopes and len(phases) == 176
                and set(inventories) == set(roles) | set(old_roles)
                and len(artifacts) == 49 and len(receipt['raw']) == 884
                and receipt['tests_passed'] == 3001 + receipt['kernel_ir_tests_passed']
                and receipt['tests_ignored'] == 25 + receipt['kernel_ir_tests_ignored'],
                'complete dynamic 176-phase/77-scope/49-product census')


def main():
    require(all(type(value) is str and re.fullmatch(r'[0-9a-f]{64}', value)
                for value in (CONTROLLER_SHA, PROPOSAL_SHA, INPUT_SHA, TERMINAL_SHA)),
            'reviewed source and actual input/terminal pins are not bound')
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B export_evidence.py ACTUAL_INPUT_SHA')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID mismatch')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ARCHIVE), 'fresh exact archive required')
    signal.alarm(180)
    evidence = ROOT / 'evidence'
    terminals = [p for p in evidence.iterdir() if p.name in ('complete.json', 'failed.json')]
    require(len(terminals) == 1, 'one actual terminal receipt required')
    receipt_body, receipt_pin = pin(terminals[0])
    require(receipt_pin['sha256'] == TERMINAL_SHA, 'actual terminal receipt pin mismatch')
    receipt = json.loads(receipt_body)
    require(receipt['schema'] == 'ferric-guarded-mlp-atomic-load-alias-cpu-v1'
            and type(receipt['passed']) is bool
            and receipt['passed'] == (terminals[0].name == 'complete.json')
            and receipt['diagnostic_build'] is False
            and receipt['diagnostic_only'] is False
            and receipt['atomic_load_alias_refinement'] is True
            and receipt['alias_predicates_changed'] is True
            and receipt['compiler_scratch_lifetime_changed'] is True
            and receipt['receipt_serialization_changed'] is False
            and receipt['inherited_memory_bounds_dag_optimization'] is True
            and receipt['memory_bounds_dag_optimization'] is True
            and receipt['cfg_linear_fusion_optimization'] is True
            and receipt['inherited_cfg_linear_fusion_optimization'] is True
            and receipt['cfg_compaction_optimization'] is True
            and receipt['inherited_cfg_compaction_optimization'] is True
            and receipt['inherited_cfg_expansion_diagnostics'] is True
            and receipt['cfg_expansion_diagnostics'] is True
            and receipt['diagnostic_changes_admission'] is False
            and receipt['static_failure_site_identified'] is True
            and receipt['actual_failure_block_count_observed'] is False
            and receipt['graph_work_diagnostics'] is True
            and receipt['inherited_graph_work_diagnostics'] is True
            and receipt['cfg_diagnostics_retained'] is True
            and receipt['admission_changed'] is True
            and receipt['structural_capacity_expansion'] is False
            and receipt['inherited_capacity_expansion'] is True
            and receipt['structural_limits_changed'] is False
            and receipt['graph_analysis_optimization'] is False
            and receipt['inherited_graph_analysis_optimization'] is True
            and receipt['resource_admission_may_change'] is True
            and receipt['membership_lookup_optimization'] is True
            and receipt['inherited_membership_lookup_optimization'] is True
            and receipt['dead_cast_census_optimization'] is True
            and receipt['inherited_dead_cast_census_optimization'] is True
            and receipt['use_lookup_optimization'] is True
            and receipt['inherited_use_lookup_optimization'] is True
            and receipt['borrow_lookup_optimization'] is True
            and receipt['inherited_borrow_lookup_optimization'] is True
            and receipt['compiler_scratch_added'] is False
            and receipt['semantic_predicates_changed'] is True
            and receipt['actual_failure_caller_identified'] is False
            and receipt['baseline_failure_caller_identified'] is False
            and receipt['baseline_failure_block_count_observed'] is False
            and receipt['baseline_failure_root_observed'] is False
            and receipt['baseline_alias_pair_observed'] is False, 'terminal outcome/schema mismatch')
    verified = receipt['baseline_descriptor_alias_refusal_observed']
    require(type(verified) is bool and (not receipt['passed'] or verified),
            'truthful prior descriptor-refusal validation state')
    for key in ('gpu_execution', 'guarded_candidate_hsaco_emitted', 'full_model_acceptance',
                'numerical_acceptance', 'performance_claim'):
        require(receipt[key] is False, 'CPU retention cannot grant additional authority')
    input_body, input_pin = pin(ROOT / 'input-manifest.json')
    inputs = json.loads(input_body)
    require(input_pin['sha256'] == sys.argv[1] == INPUT_SHA and receipt['input_manifest'] == input_pin
            and inputs['schema'] == 'ferric-guarded-mlp-atomic-load-alias-cpu-input-v1'
            and set(inputs) == {'schema', 'files', 'tool_pins', 'lineage',
                                'metadata_relocations', 'rust_src'}
            and receipt['source_lineage'] == inputs['lineage']
            and receipt['metadata_relocations'] == inputs['metadata_relocations']
            and receipt['tool_pins'] == inputs['tool_pins'], 'literal input/source-lineage join')
    if receipt['passed']:
        require(receipt['failure'] is None and receipt['postcheck_errors'] == []
                and receipt['source_unchanged'] is True and len(receipt['phases']) == 176
                and len(receipt['tests']) == 77
                and receipt['tests_passed'] == 3001 + receipt['kernel_ir_tests_passed']
                and receipt['tests_ignored'] == 25 + receipt['kernel_ir_tests_ignored']
                and len(receipt['artifacts']) == 49,
                'complete receipt lacks closed successful qualification')
    else:
        require(receipt['failure'] is not None, 'failed receipt must retain failure reason')

    files, bodies = {}, {}
    def retain(name, path, expected=None):
        relative = PurePosixPath(name)
        require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == name
                and name not in files and len(files) < MAX_MEMBERS, 'closed unique archive member')
        body, actual = pin(path)
        require(expected is None or actual == expected, 'retained body pin differs: ' + name)
        require(sum(map(len, bodies.values())) + len(body) <= MAX_TOTAL, 'retention total byte cap')
        bodies[name], files[name] = body, actual
        return body

    retain(str(terminals[0].relative_to(ROOT)), terminals[0], receipt_pin)
    retain('input-manifest.json', ROOT / 'input-manifest.json', input_pin)
    for name, key, digest in (('run_cpu.py', 'controller', CONTROLLER_SHA),
                              ('qualification_helpers.py', 'helper', HELPER_SHA)):
        body = retain(name, ROOT / name, receipt[key])
        require(files[name]['sha256'] == digest
                and compact(files[name]) == inputs['files'][name], 'qualified controller/helper identity')
    raw = receipt['raw']
    require(type(raw) is dict and len(raw) <= 884
            and (not receipt['passed'] or len(raw) == 884)
            and {p.name for p in evidence.iterdir()} == set(raw) | {terminals[0].name},
            'exact closed terminal evidence directory')
    for name, row in sorted(raw.items()):
        require(Path(name).name == name and row['path'] == str(evidence / name), 'original raw path')
        retain('evidence/' + name, evidence / name, row)
    require(len(receipt['phases']) == len({row['label'] for row in receipt['phases']}) <= 176,
            'unique bounded actual phase roster')
    for row in receipt['phases']:
        label = row['label']
        require(row['reaped'] is True and row['process_group_absent'] is True
                and not os.path.exists('/proc/' + str(row['pid'])), 'owned leaf must be terminal/reaped')
        require(json.loads(bodies['evidence/' + label + '.result.json']) == row
                and raw[label + '.command.json'] == row['command']
                and raw[label + '.stdout'] == row['stdout']
                and raw[label + '.stderr'] == row['stderr'], 'actual phase/raw join')
        command = json.loads(bodies['evidence/' + label + '.command.json'])
        started = json.loads(bodies['evidence/' + label + '.started.json'])
        require(command['argv'] == row['argv'] == started['argv']
                and started['pid'] == row['pid'] and started['pgid'] == row['pgid'],
                'actual command/started ownership join')
        if receipt['passed']:
            require(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None,
                    'passing attempt contains unsuccessful leaf')

    lineage = inputs['lineage']
    direct = {'base_complete', 'base_sources', 'base_input', 'base_metadata',
              'base_dependencies', 'atomic_load_alias_proposal', 'baseline_failure',
              'baseline_failure_stderr'}
    require(set(lineage) == direct | {'base_streams', 'atomic_load_alias_overlay'},
            'closed retained atomic-load alias lineage fields')
    selected_inputs = {input_pin['path']: input_pin, receipt['helper']['path']: receipt['helper']}
    for key in sorted(direct):
        row = lineage[key]
        name = 'lineage/baseline_failure.stderr' if key == 'baseline_failure_stderr' else 'lineage/' + key + '.json'
        retain(name, row['path'], row)
        selected_inputs[row['path']] = row
    require(set(lineage['base_streams']) == {short + '-' + suffix + '-stdout'
            for short in ('compiler', 'pliron') for suffix in ('list', 'ignored-list', 'tests')},
            'six qualified memory-bounds DAG baseline full-suite streams required')
    for name, row in sorted(lineage['base_streams'].items()):
        retain('lineage/base-streams/' + name, row['path'], row)
        selected_inputs[row['path']] = row
    proposal = json.loads(bodies['lineage/atomic_load_alias_proposal.json'])
    baseline = json.loads(bodies['lineage/base_complete.json'])
    limits = receipt['capacity_limits']
    require(proposal['schema'] == 'ferric-guarded-mlp-atomic-load-alias-source-v1'
            and lineage['atomic_load_alias_proposal']['sha256'] == PROPOSAL_SHA
            and proposal['source_only'] is True
            and proposal['atomic_load_alias_refinement'] is True
            and proposal['alias_predicates_changed'] is True
            and proposal['semantic_predicates_changed'] is True
            and proposal['admission_changed'] is True
            and proposal['compiler_scratch_added'] is False
            and proposal['compiler_scratch_lifetime_changed'] is True
            and proposal['receipt_serialization_changed'] is False
            and proposal['graph_analysis_optimization'] is False
            and proposal['inherited_graph_analysis_optimization'] is True
            and proposal['structural_limits_changed'] is False
            and proposal['resource_admission_may_change'] is True
            and proposal['baseline_descriptor_alias_refusal_observed'] is True
            and proposal['baseline_failure_caller_identified'] is False
            and proposal['baseline_failure_block_count_observed'] is False
            and proposal['baseline_failure_root_observed'] is False
            and proposal['baseline_alias_pair_observed'] is False
            and proposal['base_source_count'] == 5808
            and proposal['source_count'] == 5809
            and proposal['source_file_additions'] == 1
            and proposal['new_test_count'] == 9
            and proposal['actual_failure'] == compact(lineage['baseline_failure'])
            and proposal['actual_failure_stderr'] == compact(lineage['baseline_failure_stderr'])
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources'])
            and proposal['base_input'] == compact(lineage['base_input'])
            and proposal['base_controller'] == compact(baseline['controller'])
            and len(proposal['files']) == 2
            and sum(row['before'] is None for row in proposal['files'].values()) == 1
            and {'fe2o3/' + name: row for name, row in proposal['files'].items()}
                == lineage['atomic_load_alias_overlay']
            and receipt['atomic_load_alias_test_filter'] == proposal['filter'] == 'atomic_load_alias_'
            and receipt['atomic_load_alias_tests'] == proposal['test_names']
            and proposal['test_target'] == 'formal_atomic_load_alias_v1'
            and receipt['descriptor_atomic_negative_tests'] == proposal['descriptor_regression_test_names']
                == DESCRIPTOR_ATOMIC_TESTS
            and len(set(proposal['test_names'])) == len(proposal['test_names']) == 9,
            'exact frozen two-file/nine-test atomic-load alias proposal')
    require(limits == dict(previous_blocks=1024, blocks=2048, facts=1024, edges=2048,
                           projection_graph_work=3145728, operations=65536,
                           ranked_work=8388608, ranked_storage=131072, findings=4096),
            'inherited structural expansion with unchanged independent limits')
    require(lineage['baseline_failure'] == dict(path=str(BASE_FAILURE_ROOT / 'failed.json'),
                bytes=19077, sha256=BASE_FAILURE_SHA)
            and lineage['baseline_failure_stderr'] == dict(
                path=str(BASE_FAILURE_ROOT / 'compile.stderr'), bytes=8480,
                sha256=BASE_FAILURE_STDERR_SHA), 'exact measured baseline failure inputs')
    baseline_failure = json.loads(bodies['lineage/baseline_failure.json'])
    baseline_stderr = bodies['lineage/baseline_failure.stderr']
    require(baseline_failure['schema'] == 'ferric-guarded-mlp-combined-state-lowering-result-v1'
            and baseline_failure['passed'] is False
            and baseline_failure['cfg_linear_fusion_optimization'] is True
            and baseline_failure['cfg_compaction_optimization'] is True
            and baseline_failure['diagnostic_build'] is False
            and baseline_failure['diagnostic_only'] is False
            and baseline_failure['compiler_scratch_added'] is True
            and baseline_failure['actual_failure_caller_identified'] is False
            and baseline_failure['actual_failure_block_count_observed'] is False
            and baseline_failure['failure'] == "RuntimeError('natural successful/reaped leaf required')"
            and baseline_failure['postcheck_errors'] == []
            and baseline_failure['source_unchanged'] is True
            and baseline_failure['input_byte_maps_rechecked'] is True
            and baseline_failure['automatic_retries'] == 0
            and baseline_failure['artifact'] is None
            and baseline_failure['retained_handoff_or_llvm'] is False
            and baseline_failure['capacity_limits'] == limits
            and baseline_failure['raw']['compile.stderr'] == lineage['baseline_failure_stderr']
            and compact(baseline_failure['raw']['source-before.json'])
                == compact(baseline_failure['raw']['source-after.json'])
            and len(baseline_failure['phases']) == 1,
            'actual clean failed guarded baseline and immutable source joins')
    baseline_phase = baseline_failure['phases'][0]
    require(baseline_phase['label'] == 'compile' and baseline_phase['exit_code'] == 1
            and baseline_phase['natural_exit'] is True and baseline_phase['reaped'] is True
            and baseline_phase['process_group_absent'] is True
            and baseline_phase['forced_cleanup'] is False and baseline_phase['timed_out'] is False
            and baseline_phase['exception'] is None and baseline_phase['observed_signals'] == []
            and baseline_phase['stderr'] == lineage['baseline_failure_stderr']
            and baseline_phase['stdout']['bytes'] == 0
            and baseline_phase['stdout'] == baseline_failure['raw']['compile.stdout'],
            'baseline compiler exited naturally with code one and retained exact stderr')
    require(all(baseline_failure[key] is False for key in (
                'production_authority', 'load_authority', 'launch_authority', 'gpu_execution',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim'))
            and [line for line in baseline_stderr.splitlines()
                 if line.startswith(b'fe2o3 rustc extraction:')] == [BASE_FAILURE_DIAGNOSTIC],
            'one exact descriptor alias refusal without measured root or pair authority')
    require(lineage['base_complete']['sha256'] == BASE_COMPLETE_SHA
            and lineage['base_sources']['sha256'] == BASE_SOURCES_SHA
            and lineage['base_input']['sha256'] == BASE_INPUT_SHA,
            'actual qualified baseline identity')
    baseline = json.loads(bodies['lineage/base_complete.json'])
    require(baseline['schema'] == 'ferric-guarded-mlp-memory-bounds-dag-cpu-v1'
            and baseline['passed'] is True and baseline['failure'] is None
            and baseline['postcheck_errors'] == [] and baseline['source_unchanged'] is True
            and baseline['tests_passed'] == 2990 and baseline['tests_ignored'] == 25
            and len(baseline['phases']) == 45 and len(baseline['tests']) == 32
            and baseline['admission_changed'] is True and baseline['structural_capacity_expansion'] is False
            and baseline['inherited_capacity_expansion'] is True
            and baseline['graph_analysis_optimization'] is True
            and baseline['resource_admission_may_change'] is True
            and baseline['semantic_predicates_changed'] is False
            and baseline['diagnostic_build'] is False and baseline['diagnostic_only'] is False
            and baseline['cfg_linear_fusion_optimization'] is True
            and baseline['cfg_compaction_optimization'] is True
            and baseline['inherited_cfg_compaction_optimization'] is True
            and baseline['inherited_cfg_expansion_diagnostics'] is True
            and baseline['cfg_expansion_diagnostics'] is True
            and baseline['diagnostic_changes_admission'] is False
            and baseline['static_failure_site_identified'] is True
            and baseline['actual_failure_block_count_observed'] is False
            and baseline['inherited_borrow_lookup_optimization'] is True
            and baseline['graph_work_diagnostics'] is True
            and baseline['membership_lookup_optimization'] is True
            and baseline['dead_cast_census_optimization'] is True
            and baseline['inherited_dead_cast_census_optimization'] is True
            and baseline['use_lookup_optimization'] is True
            and baseline['inherited_use_lookup_optimization'] is True
            and baseline['borrow_lookup_optimization'] is True
            and baseline['compiler_scratch_added'] is True
            and baseline['inherited_graph_work_diagnostics'] is True
            and baseline['baseline_failure_caller_identified'] is True
            and baseline['baseline_failure_block_count_observed'] is True
            and baseline['baseline_memory_bounds_preflight_refusal'] is True
            and baseline['baseline_rendered_blocks'] == 567
            and baseline['baseline_rendered_edges'] == 1122
            and baseline['baseline_rendered_operations'] == 2240
            and baseline['baseline_guard_candidates'] == 552
            and baseline['baseline_intersection_work_upper_bound'] == 9340170
            and baseline['baseline_memory_bounds_work_limit'] == limits['ranked_work']
            and baseline['baseline_runtime_work_exhaustion_observed'] is False
            and baseline['memory_bounds_dag_optimization'] is True
            and receipt['memory_bounds_dag_test_filter'] == baseline['memory_bounds_dag_test_filter']
            and receipt['memory_bounds_dag_tests'] == baseline['memory_bounds_dag_tests']
            and baseline['baseline_edge_verdict_observed'] is False
            and baseline['actual_failure_caller_identified'] is False
            and baseline['inherited_graph_analysis_optimization'] is True
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources'])
            and proposal['base_input'] == compact(lineage['base_input'])
            and proposal['base_controller'] == compact(baseline['controller'])
            and receipt['cfg_linear_fusion_test_filter'] == baseline['cfg_linear_fusion_test_filter']
            and receipt['cfg_linear_fusion_tests'] == baseline['cfg_linear_fusion_tests']
            and receipt['cfg_compaction_test_filter'] == baseline['cfg_compaction_test_filter']
            and receipt['cfg_compaction_tests'] == baseline['cfg_compaction_tests']
            and receipt['cfg_expansion_test_filter'] == baseline['cfg_expansion_test_filter']
            and receipt['cfg_expansion_tests'] == baseline['cfg_expansion_tests']
            and receipt['borrow_lookup_test_filter'] == baseline['borrow_lookup_test_filter']
            and receipt['borrow_lookup_tests'] == baseline['borrow_lookup_tests']
            and receipt['use_lookup_test_filter'] == baseline['use_lookup_test_filter']
            and receipt['use_lookup_tests'] == baseline['use_lookup_tests']
            and receipt['census_test_filter'] == baseline['census_test_filter']
            and receipt['census_tests'] == baseline['census_tests']
            and receipt['membership_test_filter'] == baseline['membership_test_filter']
            and receipt['membership_tests'] == baseline['membership_tests']
            and receipt['graph_work_test_filter'] == baseline['graph_work_test_filter']
            and receipt['graph_work_tests'] == baseline['graph_work_tests']
            and receipt['dag_test_filter'] == baseline['dag_test_filter']
            and receipt['dag_tests'] == baseline['dag_tests']
            and baseline['capacity_limits'] == limits
            and receipt['capacity_cohorts'] == baseline['capacity_cohorts']
            and receipt['cfg_diagnostic_tests'] == baseline['cfg_diagnostic_tests']
            and receipt['compiler_cohorts'] == baseline['compiler_cohorts']
            and compact(baseline['input_manifest']) == compact(lineage['base_input'])
            and compact(baseline['input_sources']) == compact(lineage['base_sources'])
            and compact(baseline['final_sources']) == compact(lineage['base_sources'])
            and compact(baseline['raw']['metadata.stdout']) == compact(lineage['base_metadata']),
            'qualified baseline receipt/readset joins')
    for name in ('dependencies-before.json', 'dependencies-after.json'):
        require(compact(baseline['raw'][name]) == compact(lineage['base_dependencies']),
                'qualified dependency before/after map join')
    for short in ('compiler', 'pliron'):
        for suffix in ('list', 'ignored-list', 'tests'):
            key = short + '-' + suffix + '-stdout'
            raw_name = (short + '-tests.stdout' if suffix == 'tests' else
                        short + '-lib' + ('-list.stdout' if suffix == 'list' else '-ignored.stdout'))
            require(compact(lineage['base_streams'][key]) == compact(baseline['raw'][raw_name]),
                    'qualified historical stream pin')
    retain('lineage/rust-src.json', inputs['rust_src']['path'], inputs['rust_src'])
    selected_inputs[inputs['rust_src']['path']] = inputs['rust_src']
    require(selected_inputs == receipt['lineage_input_pins'], 'complete exact admission readset retained')

    before = json.loads(bodies['evidence/sources-before.json'])
    require(files['evidence/sources-before.json'] == receipt['input_sources']
            and {name: compact(row) for name, row in before.items()} == inputs['files'],
            'actual input source snapshot join')
    final_pin = receipt['final_sources']
    require(final_pin is not None and files['evidence/sources-after.json'] == final_pin,
            'recorded final source snapshot required for source-body retention')
    final = json.loads(bodies['evidence/sources-after.json'])
    require(receipt['source_unchanged'] == (final == before), 'truthful source preservation flag')
    baseline_sources = json.loads(bodies['lineage/base_sources.json'])
    require(compact(before['qualification_helpers.py']) == dict(bytes=4777, sha256=HELPER_SHA)
            and compact(receipt['helper']) == dict(bytes=4777, sha256=HELPER_SHA)
            and compact(baseline_sources['qualification_helpers.py']) == dict(bytes=4777, sha256=BASE_HELPER_SHA)
            and compact(baseline['helper']) == dict(bytes=4777, sha256=BASE_HELPER_SHA),
            'current progress parser and immutable qualified baseline helper joins')
    expected_sources = {name: compact(row) for name, row in baseline_sources.items()
                        if name.startswith('fe2o3/')}
    require(len(expected_sources) == 5808, 'exact qualified memory-bounds DAG baseline source census')
    source_names = set(lineage['atomic_load_alias_overlay'])
    require(len(source_names) == 2, 'exact two atomic-load alias source bodies')
    for name, row in lineage['atomic_load_alias_overlay'].items():
        require(row['before'] == expected_sources.get(name)
                and (name in expected_sources) == (row['before'] is not None),
                'exact atomic-load alias preimage')
        expected_sources[name] = row['after']
    require(len(expected_sources) == 5809
            and {name: compact(row) for name, row in before.items() if name.startswith('fe2o3/')}
                == expected_sources, 'actual complete atomic-load alias overlay source join')
    verify_test_census(receipt, bodies, baseline)
    for name in sorted(source_names):
        retain(name, ROOT / name, final[name])
    retain('export_evidence.py', Path(__file__).resolve())
    manifest = dict(schema='ferric-guarded-mlp-atomic-load-alias-cpu-retention-v1',
                    files=files, receipt=receipt_pin, input_manifest=input_pin,
                    passed=receipt['passed'], failure=receipt['failure'],
                    postcheck_errors=receipt['postcheck_errors'],
                    source_unchanged=receipt['source_unchanged'], source_lineage=lineage,
                    atomic_load_alias_source_files=sorted(source_names), diagnostic_build=False, diagnostic_only=False,
                    atomic_load_alias_refinement=True, alias_predicates_changed=True,
                    atomic_load_alias_test_filter=receipt['atomic_load_alias_test_filter'],
                    atomic_load_alias_tests=receipt['atomic_load_alias_tests'],
                    descriptor_atomic_negative_tests=receipt['descriptor_atomic_negative_tests'],
                    kernel_ir_target_names=receipt['kernel_ir_target_names'],
                    kernel_ir_historical_raw_census_available=False,
                    kernel_ir_tests_passed=receipt['kernel_ir_tests_passed'],
                    kernel_ir_tests_ignored=receipt['kernel_ir_tests_ignored'],
                    inherited_memory_bounds_dag_optimization=True,
                    memory_bounds_dag_optimization=True,
                    memory_bounds_dag_test_filter=receipt['memory_bounds_dag_test_filter'],
                    memory_bounds_dag_tests=receipt['memory_bounds_dag_tests'],
                    cfg_linear_fusion_optimization=True, inherited_cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    cfg_compaction_test_filter=receipt['cfg_compaction_test_filter'],
                    cfg_compaction_tests=receipt['cfg_compaction_tests'],
                    cfg_linear_fusion_test_filter=receipt['cfg_linear_fusion_test_filter'],
                    cfg_linear_fusion_tests=receipt['cfg_linear_fusion_tests'],
                    inherited_cfg_expansion_diagnostics=True,
                    cfg_expansion_diagnostics=True, diagnostic_changes_admission=False,
                    cfg_expansion_test_filter=receipt['cfg_expansion_test_filter'],
                    cfg_expansion_tests=receipt['cfg_expansion_tests'],
                    static_failure_site_identified=True,
                    actual_failure_block_count_observed=False,
                    graph_work_diagnostics=True, inherited_graph_work_diagnostics=True,
                    cfg_diagnostics_retained=True, admission_changed=True,
                    structural_capacity_expansion=False, inherited_capacity_expansion=True,
                    structural_limits_changed=False, graph_analysis_optimization=False,
                    inherited_graph_analysis_optimization=True, resource_admission_may_change=True,
                    membership_lookup_optimization=True, inherited_membership_lookup_optimization=True,
                    dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                    use_lookup_optimization=True, inherited_use_lookup_optimization=True,
                    borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                    compiler_scratch_added=False, compiler_scratch_lifetime_changed=True,
                    receipt_serialization_changed=False, semantic_predicates_changed=True,
                    actual_failure_caller_identified=False, baseline_failure_caller_identified=False,
                    baseline_failure_block_count_observed=False,
                    baseline_descriptor_alias_refusal_observed=verified,
                    baseline_failure_root_observed=False, baseline_alias_pair_observed=False,
                    capacity_limits=limits,
                    capacity_cohorts=receipt['capacity_cohorts'],
                    dag_test_filter=receipt['dag_test_filter'], dag_tests=receipt['dag_tests'],
                    graph_work_test_filter=receipt['graph_work_test_filter'],
                    graph_work_tests=receipt['graph_work_tests'],
                    membership_test_filter=receipt['membership_test_filter'],
                    membership_tests=receipt['membership_tests'],
                    census_test_filter=receipt['census_test_filter'], census_tests=receipt['census_tests'],
                    use_lookup_test_filter=receipt['use_lookup_test_filter'],
                    use_lookup_tests=receipt['use_lookup_tests'],
                    borrow_lookup_test_filter=receipt['borrow_lookup_test_filter'],
                    borrow_lookup_tests=receipt['borrow_lookup_tests'],
                    exported_binary_bodies=False, actual_artifact_metadata=receipt['artifacts'],
                    gpu_execution=False, guarded_candidate_hsaco_emitted=False,
                    numerical_acceptance=False, full_model_acceptance=False, performance_claim=False)
    bodies['retention-manifest.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    require(len(bodies) == len(raw) + 23
            and (not receipt['passed'] or len(bodies) == 907),
            'exact raw plus terminal/input/harness/lineage/two-source/export roster')
    require(len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'final bounded retention roster')
    with ARCHIVE.open('xb') as raw_archive:
        with tarfile.open(fileobj=raw_archive, mode='w:gz') as archive:
            for name, body in sorted(bodies.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(body), 0o600, 0
                archive.addfile(member, io.BytesIO(body))
    require(all(pin(Path(row['path']))[1] == row for row in files.values()), 'source/readset drift after archive')
    require({p.name for p in evidence.iterdir()} == set(receipt['raw']) | {terminals[0].name},
            'evidence roster drift after archive')
    print(json.dumps(dict(archive=pin(ARCHIVE, MAX_TOTAL)[1], files=len(bodies),
                          source_body_bytes=sum(map(len, bodies.values())),
                          passed=receipt['passed'], receipt=receipt_pin)))


if __name__ == '__main__':
    main()
