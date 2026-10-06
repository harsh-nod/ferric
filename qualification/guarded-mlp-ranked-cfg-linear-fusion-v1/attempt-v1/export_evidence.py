"""Bounded data-only retention of an actual terminal ranked-CFG linear fusion CPU attempt."""
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


ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-ranked-cfg-linear-fusion-cpu-v228-v1')
ARCHIVE = ROOT.parent / 'guarded-mlp-ranked-cfg-linear-fusion-cpu-evidence-v228-v1.tar.gz'
CONTROLLER_SHA = '6a3c26eb3c77be02a5281f5a3a421d26611f773916a04cabf167ba4d94d1bf38'
HELPER_SHA = 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'
PROPOSAL_SHA = '3ecc592848d6fddf148684f539e16cf20a3761e1fa15821017a7e17819783cbe'
INPUT_SHA = 'f72c5a9b18960983c0398f9819cd9087049420e791ef11a420e4cc9ef254cece'
TERMINAL_SHA = '67207c7b09f58f3f3d3e64f763274badbd296668964cf772487d01646a126444'  # Actual failed first qualification.
BASE_COMPLETE_SHA = '439d4b5bf10a1fe9f44d02fa1cbd28b03eec9601bb161806909b2f073beb9018'
BASE_SOURCES_SHA = 'c00481ef4e62adf81ebce11b6a2a9ecb51823a709c28ccf87194c54c5fa7f420'
BASE_INPUT_SHA = 'eee83b9d3f67eeb383b164e14ba4c8e5e8ee218d14492c3dbbde017135fff2ba'
BASE_FAILURE_ROOT = ROOT.parent / 'guarded-mlp-ranked-cfg-compaction-lowering-v228-v1'
BASE_FAILURE_SHA = '1bf26b28162414dfb4afbf6dc5ba5a70a60d1c1fe1fd55624c251733840c560b'
BASE_FAILURE_STDERR_SHA = '9a309b8f23c66ad29e98a464bcb3ed8480766e2c7fd807415616dc99d6121a05'
BASE_FAILURE_DIAGNOSTIC = (
    b'fe2o3 rustc extraction: production compilation general kernel verification failed: '
    b'error[FE2O3-PRESERVE-028]: structural identity is unavailable; '
    b'error[FE2O3-PRESERVE-002]: error[FE2O3-PRESERVE-002]: '
    b'basic blocks count 1025 at function exceeds identity limit 1024; '
    b'help: split or simplify the function before preservation checking')
MAX_FILE, MAX_TOTAL, MAX_MEMBERS = 64 << 20, 128 << 20, 500


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
    require(receipt['schema'] == 'ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-v1'
            and type(receipt['passed']) is bool
            and receipt['passed'] == (terminals[0].name == 'complete.json')
            and receipt['diagnostic_build'] is False
            and receipt['diagnostic_only'] is False
            and receipt['cfg_linear_fusion_optimization'] is True
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
            and receipt['compiler_scratch_added'] is True
            and receipt['semantic_predicates_changed'] is False
            and receipt['actual_failure_caller_identified'] is False
            and receipt['baseline_failure_caller_identified'] is True
            and receipt['baseline_failure_block_count_observed'] is True
            and receipt['baseline_identity_first_refusal_blocks'] == 1025
            and receipt['baseline_identity_block_limit'] == 1024
            and receipt['baseline_rendered_blocks'] == 1675
            and receipt['baseline_rendered_edges'] == 2230
            and receipt['baseline_edge_verdict_observed'] is False, 'terminal outcome/schema mismatch')
    for key in ('gpu_execution', 'guarded_candidate_hsaco_emitted', 'full_model_acceptance',
                'numerical_acceptance', 'performance_claim'):
        require(receipt[key] is False, 'CPU retention cannot grant additional authority')
    input_body, input_pin = pin(ROOT / 'input-manifest.json')
    inputs = json.loads(input_body)
    require(input_pin['sha256'] == sys.argv[1] == INPUT_SHA and receipt['input_manifest'] == input_pin
            and inputs['schema'] == 'ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-input-v1'
            and set(inputs) == {'schema', 'files', 'tool_pins', 'lineage',
                                'metadata_relocations', 'rust_src'}
            and receipt['source_lineage'] == inputs['lineage']
            and receipt['metadata_relocations'] == inputs['metadata_relocations']
            and receipt['tool_pins'] == inputs['tool_pins'], 'literal input/source-lineage join')
    if receipt['passed']:
        require(receipt['failure'] is None and receipt['postcheck_errors'] == []
                and receipt['source_unchanged'] is True and len(receipt['phases']) == 44
                and len(receipt['tests']) == 31
                and receipt['tests_passed'] == 2972
                and receipt['tests_ignored'] == 25 and len(receipt['artifacts']) == 7,
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
    require(type(raw) is dict and len(raw) <= 224
            and (not receipt['passed'] or len(raw) == 224)
            and {p.name for p in evidence.iterdir()} == set(raw) | {terminals[0].name},
            'exact closed terminal evidence directory')
    for name, row in sorted(raw.items()):
        require(Path(name).name == name and row['path'] == str(evidence / name), 'original raw path')
        retain('evidence/' + name, evidence / name, row)
    require(len(receipt['phases']) == len({row['label'] for row in receipt['phases']}) <= 44,
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
              'base_dependencies', 'cfg_linear_fusion_proposal', 'baseline_failure',
              'baseline_failure_stderr'}
    require(set(lineage) == direct | {'base_streams', 'cfg_linear_fusion_overlay'},
            'closed retained CFG linear fusion lineage fields')
    selected_inputs = {input_pin['path']: input_pin, receipt['helper']['path']: receipt['helper']}
    for key in sorted(direct):
        row = lineage[key]
        name = 'lineage/baseline_failure.stderr' if key == 'baseline_failure_stderr' else 'lineage/' + key + '.json'
        retain(name, row['path'], row)
        selected_inputs[row['path']] = row
    require(set(lineage['base_streams']) == {short + '-' + suffix + '-stdout'
            for short in ('compiler', 'pliron') for suffix in ('list', 'ignored-list', 'tests')},
            'six qualified CFG-compaction baseline full-suite streams required')
    for name, row in sorted(lineage['base_streams'].items()):
        retain('lineage/base-streams/' + name, row['path'], row)
        selected_inputs[row['path']] = row
    proposal = json.loads(bodies['lineage/cfg_linear_fusion_proposal.json'])
    require(lineage['cfg_linear_fusion_proposal']['sha256'] == PROPOSAL_SHA
            and proposal['schema'] == 'ferric-guarded-mlp-ranked-cfg-linear-fusion-source-v1'
            and len(proposal['files']) == 4
            and sum(row['before'] is None for row in proposal['files'].values()) == 2
            and {'fe2o3/' + name: row for name, row in proposal['files'].items()}
                == lineage['cfg_linear_fusion_overlay']
            and proposal['source_only'] is True
            and proposal['cfg_linear_fusion_optimization'] is True
            and proposal['cfg_compaction_optimization'] is True
            and proposal['inherited_cfg_compaction_optimization'] is True
            and proposal['admission_changed'] is True
            and proposal['compiler_scratch_added'] is True
            and proposal['graph_analysis_optimization'] is False
            and proposal['structural_limits_changed'] is False
            and proposal['resource_admission_may_change'] is True
            and proposal['semantic_predicates_changed'] is False
            and proposal['static_failure_site_identified'] is True
            and proposal['actual_failure_caller_identified'] is False
            and proposal['actual_failure_block_count_observed'] is False
            and proposal['baseline_failure_caller_identified'] is True
            and proposal['baseline_failure_block_count_observed'] is True
            and proposal['baseline_identity_first_refusal_blocks'] == 1025
            and proposal['baseline_identity_block_limit'] == 1024
            and proposal['baseline_rendered_blocks'] == 1675
            and proposal['baseline_rendered_edges'] == 2230
            and proposal['baseline_edge_verdict_observed'] is False
            and proposal['base_source_count'] == 5804 and proposal['source_count'] == 5806
            and proposal['source_file_additions'] == 2 and proposal['new_test_count'] == 9
            and proposal['actual_failure'] == compact(lineage['baseline_failure'])
            and proposal['actual_failure_stderr'] == compact(lineage['baseline_failure_stderr'])
            and receipt['cfg_linear_fusion_test_filter'] == proposal['filter']
            and receipt['cfg_linear_fusion_tests'] == proposal['test_names']
            and len(set(proposal['test_names'])) == len(proposal['test_names']) == 9,
            'exact frozen four-file/nine-test CFG linear fusion proposal')
    limits = receipt['capacity_limits']
    require(limits == dict(previous_blocks=1024, blocks=2048, facts=1024, edges=2048,
                           projection_graph_work=3145728, operations=65536,
                           ranked_work=8388608, ranked_storage=131072, findings=4096)
            and proposal['graph_work_limit'] == limits['projection_graph_work']
            and proposal['block_limit'] == limits['blocks']
            and proposal['edge_limit'] == limits['edges'],
            'inherited structural expansion with unchanged independent limits')
    require(lineage['baseline_failure'] == dict(path=str(BASE_FAILURE_ROOT / 'failed.json'),
                bytes=16503, sha256=BASE_FAILURE_SHA)
            and lineage['baseline_failure_stderr'] == dict(
                path=str(BASE_FAILURE_ROOT / 'compile.stderr'), bytes=164209,
                sha256=BASE_FAILURE_STDERR_SHA), 'exact measured baseline failure inputs')
    baseline_failure = json.loads(bodies['lineage/baseline_failure.json'])
    baseline_stderr = bodies['lineage/baseline_failure.stderr']
    require(baseline_failure['schema'] == 'ferric-guarded-mlp-ranked-cfg-compaction-lowering-result-v1'
            and baseline_failure['passed'] is False
            and baseline_failure['cfg_compaction_optimization'] is True
            and baseline_failure['diagnostic_build'] is False
            and baseline_failure['diagnostic_only'] is False
            and baseline_failure['compiler_scratch_added'] is False
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
            'one exact first-refused identity diagnostic without additional authority')
    # This is an inventory of the pinned rendered text, not an edge-admission verdict.
    rendered_parts = baseline_stderr.split(b'  = ranked PLIRON before rejected lowering:\n')
    require(len(rendered_parts) == 2, 'one retained ranked graph diagnostic required')
    rendered_end = rendered_parts[1].split(b'  = lowering stopped before target IR or artifact emission\n')
    require(len(rendered_end) == 2, 'one complete bounded ranked graph rendering required')
    rendered_graph = rendered_end[0]
    require(rendered_graph.startswith(b'    func @ferric_qwen3_mlp_state_guard_v1 {\n')
            and rendered_graph.endswith(b'    }\n')
            and rendered_graph.count(b'    func @') == 1,
            'rendered failure graph belongs to the exact guarded root')
    block_ids = re.findall(rb'^    \^bb([0-9]+):$', rendered_graph, re.MULTILINE)
    block_references = re.findall(rb'\^bb([0-9]+)', rendered_graph)
    require(block_ids == [str(index).encode() for index in range(1675)]
            and len(block_references) - len(block_ids) == 2230
            and set(block_references) == set(block_ids),
            '1675 declared blocks and 2230 raw block references, not a first-refusal total')
    require(lineage['base_complete']['sha256'] == BASE_COMPLETE_SHA
            and lineage['base_sources']['sha256'] == BASE_SOURCES_SHA
            and lineage['base_input']['sha256'] == BASE_INPUT_SHA,
            'actual qualified baseline identity')
    baseline = json.loads(bodies['lineage/base_complete.json'])
    require(baseline['schema'] == 'ferric-guarded-mlp-ranked-cfg-compaction-cpu-v1'
            and baseline['passed'] is True and baseline['failure'] is None
            and baseline['postcheck_errors'] == [] and baseline['source_unchanged'] is True
            and baseline['tests_passed'] == 2954 and baseline['tests_ignored'] == 25
            and len(baseline['phases']) == 43 and len(baseline['tests']) == 30
            and baseline['admission_changed'] is True and baseline['structural_capacity_expansion'] is False
            and baseline['inherited_capacity_expansion'] is True
            and baseline['graph_analysis_optimization'] is False
            and baseline['resource_admission_may_change'] is True
            and baseline['semantic_predicates_changed'] is False
            and baseline['diagnostic_build'] is False and baseline['diagnostic_only'] is False
            and baseline['cfg_compaction_optimization'] is True
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
            and baseline['compiler_scratch_added'] is False
            and baseline['inherited_graph_work_diagnostics'] is True
            and baseline['baseline_failure_caller_identified'] is True
            and baseline['baseline_failure_block_count_observed'] is True
            and baseline['baseline_projected_blocks'] == 2778
            and baseline['baseline_semantic_blocks'] == 1121
            and baseline['baseline_block_limit'] == 2048
            and baseline['actual_failure_caller_identified'] is False
            and baseline['inherited_graph_analysis_optimization'] is True
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources'])
            and proposal['base_input'] == compact(lineage['base_input'])
            and proposal['base_controller'] == compact(baseline['controller'])
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
    expected_sources = {name: compact(row) for name, row in baseline_sources.items()
                        if name.startswith('fe2o3/')}
    require(len(expected_sources) == 5804, 'exact qualified CFG-compaction baseline source census')
    source_names = set(lineage['cfg_linear_fusion_overlay'])
    require(len(source_names) == 4, 'exact four CFG linear fusion source bodies')
    for name, row in lineage['cfg_linear_fusion_overlay'].items():
        require(row['before'] == expected_sources.get(name)
                and (name in expected_sources) == (row['before'] is not None),
                'exact CFG linear fusion preimage')
        expected_sources[name] = row['after']
    require(len(expected_sources) == 5806
            and {name: compact(row) for name, row in before.items() if name.startswith('fe2o3/')}
                == expected_sources, 'actual complete CFG linear fusion overlay source join')
    for name in sorted(source_names):
        retain(name, ROOT / name, final[name])
    retain('export_evidence.py', Path(__file__).resolve())
    manifest = dict(schema='ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-retention-v1',
                    files=files, receipt=receipt_pin, input_manifest=input_pin,
                    passed=receipt['passed'], failure=receipt['failure'],
                    postcheck_errors=receipt['postcheck_errors'],
                    source_unchanged=receipt['source_unchanged'], source_lineage=lineage,
                    cfg_linear_fusion_source_files=sorted(source_names), diagnostic_build=False, diagnostic_only=False,
                    cfg_linear_fusion_optimization=True,
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
                    compiler_scratch_added=True,
                    semantic_predicates_changed=False,
                    actual_failure_caller_identified=False, baseline_failure_caller_identified=True,
                    baseline_failure_block_count_observed=True,
                    baseline_identity_first_refusal_blocks=1025, baseline_identity_block_limit=1024,
                    baseline_rendered_blocks=1675, baseline_rendered_edges=2230,
                    baseline_edge_verdict_observed=False,
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
    require(len(bodies) == len(raw) + 25
            and (not receipt['passed'] or len(bodies) == 249),
            'exact raw plus terminal/input/harness/lineage/four-source/export roster')
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
