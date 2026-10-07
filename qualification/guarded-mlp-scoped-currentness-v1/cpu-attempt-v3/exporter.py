"""Export/retain exact CPU evidence as data; never import or execute retained code."""
import ast
from collections import Counter
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-scoped-currentness-cpu-v228-v3'
ARCHIVE = E / 'guarded-mlp-scoped-currentness-cpu-evidence-v228-v3.tar.gz'
INPUT = dict(bytes=215444, sha256='2b7581fd58ef2981bffb7ade357f2e15c7758e8c039a86887225810fb6ebdfa1')
SOURCE_ARCHIVE = dict(bytes=999136, sha256='f46639175510b779125a24f6477800e05c3622d66533d8296d3db4a6310a14df')
CONTROLLER = dict(bytes=55965, sha256='e8270de7cedfcba4fa9587aee41011f6f99d060437f3e2eac89e0027388e716f')
STAGER = dict(bytes=23799, sha256='043d46144d4273af001265052ebb913464017ec4751632c3c7ea2a53fb97ce2d')
CACHE = dict(bytes=26511, sha256='28ba8dcd2578688231204408baa0cfb7eb658c8de6cb0a0b77c123156d80ea00')
CACHE_ARCHIVE = dict(bytes=9440444, sha256='9cbb23c185664f48f56713ccc6d38bb4d6d5e154e5c7ff58098b30e68207de96')
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'runtime-metadata', 'default-check',
    'kfd-tests-build', 'kfd-list', 'kfd-ignored', 'kfd-tests', 'terminal-pair-tests', 'peer-read-pair-tests', 'arena-reuse-tests', 'arena-memory-tests',
    'retained-tests', 'mixed-bank-tests', 'scoped-checkpoint-tests', 'scoped-group-tests', 'scoped-layer-tests', 'interface-doc-list', 'interface-doc-tests', 'metadata',
    'worker-tests-build', 'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
HELPERS = ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py')
MANIFESTS = tuple('fe2o3/' + name for name in
    ('Cargo.toml', 'Cargo.toml.original', 'Cargo.lock', 'Cargo.lock.input', 'rust-toolchain.toml')) \
    + (WORKER + 'Cargo.toml', WORKER + 'Cargo.lock')
PRLIMIT = ['/usr/bin/prlimit', '--as=12884901888', '--cpu=1200', '--fsize=1073741824', '--core=0', '--']
MAX_MEMBERS, MAX_BODY = 256, 64 << 20


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(items):
        result = {}
        for name, row in items:
            require(name not in result, 'duplicate JSON key')
            result[name] = row
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def ordinary(name):
    return type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute() \
        and '..' not in Path(name).parts and name not in ('', '.')


def read(path, cap=MAX_BODY):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical evidence input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_size <= cap,
                'bounded single-link evidence body')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'evidence input drift')
    return raw


def files_below(root):
    require(root.is_dir() and root.resolve(strict=True) == root, 'ordinary source root')
    result = []
    for directory, dirs, names in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'directory alias')
        result.extend(Path(directory) / name for name in names)
    require(len(result) <= 20000, 'bounded file census')
    return result


def file_pin(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical live hash input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= 1 << 30, 'bounded ordinary live hash input')
        digest, size = hashlib.sha256(), 0
        while block := stream.read(1 << 20):
            size += len(block)
            require(size <= before.st_size, 'live hash input grew')
            digest.update(block)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and size == before.st_size, 'live hash input drift')
    return dict(bytes=size, sha256=digest.hexdigest())


def named(text):
    for name in ('payload_release_failure_after_event_destroy_is_process_terminal',
                 'unpublished_custody_cleanup_failure_is_process_terminal'):
        full = 'queue_linux::tests::' + name
        text, count = re.subn('^' + re.escape('test ' + full + ' ... \nrunning 1 test\nok')
            + r'(?=\n|\Z)', 'test ' + full + ' ... ok', text, flags=re.M)
        require(count <= 1, 'duplicate known nested child output')
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                           r'(\d+) measured; (\d+) filtered out; finished in [0-9.]+s', text, re.M)
    require(len(rows) == len({name for name, _ in rows}) and summaries, 'closed actual named outcomes')
    value = dict(named=[dict(name=name, outcome=status) for name, status in rows],
        summaries=[dict(status=status, passed=int(passed), failed=int(failed), ignored=int(ignored),
                        measured=int(measured), filtered_out=int(filtered))
                   for status, passed, failed, ignored, measured, filtered in summaries])
    for field, status in (('passed', 'ok'), ('failed', 'FAILED'), ('ignored', 'ignored')):
        value[field] = sum(row[field] for row in value['summaries'])
        require(value[field] == sum(s == status for _, s in rows), 'actual named/summary census')
    return value


def verify(bodies, terminal_name, terminal_sha):
    require(all(value is not None for value in (INPUT, SOURCE_ARCHIVE, CONTROLLER, STAGER)),
            'actual reviewed input/archive/controller/stager bindings required')
    raw_terminal = bodies['evidence/' + terminal_name]
    require(pin(raw_terminal)['sha256'] == terminal_sha, 'observed terminal hash')
    result = parse(raw_terminal)
    require(result['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-v1'
            and type(result['passed']) is bool and terminal_name == ('complete.json' if result['passed'] else 'failed.json')
            and result['inherited_terminal_pair_runtime_preserved'] is True
            and all(result[key] is True for key in ('explicit_per_dispatch_opt_in',
                'legacy_terminal_dispatch_preserved', 'fresh_terminal_opt_in_refused',
                'inherited_peer_read_pair_runtime_preserved', 'opt_in_retired_arena_reuse_preserved',
                'opt_in_terminal_currentness_cadence_changed', 'method_local_currentness_cadence_changed',
                'scoped_currentness_runtime_source_added', 'scoped_currentness_worker_source_added',
                'explicit_scoped_readiness_selector_source_added', 'worker_source_changed'))
            and all(result[key] is False for key in ('default_fresh_arena_policy_changed',
                'global_currentness_policy_changed', 'arena_policy_changed', 'ordinary_read_api_changed',
                'public_runtime_limits_changed', 'scoped_currentness_native_execution', 'currentness_temporal_equivalence_claim',
                'lockfiles_changed', 'shared_cache_changed', 'full_model_long_request_enabled',
                'gpu_execution', 'gpu_qualified', 'numerical_acceptance', 'performance_claim', 'production_authority',
                'all_crate_doctests_executed')), 'CPU-only result authority')
    require(result['postcheck_errors'] == [], 'supported stable source/dependency postchecks')
    formatted = result['input_sources'] is not None
    if formatted:
        require(result['source_unchanged'] is True and result['input_sources'] == result['final_sources'],
                'observed postformat source stability')
    else:
        require(not result['passed'] and result['source_unchanged'] is False
                and result['format_changed_paths'] == [], 'original failed preformat terminal')
    inputs = parse(bodies['input-manifest.json'])
    require(pin(bodies['input-manifest.json']) == INPUT == compact(result['input_manifest'])
            and pin(bodies['run_cpu.py']) == CONTROLLER == compact(result['controller'])
            and pin(bodies['transport.py']) == STAGER, 'exact qualification controller/input/stager')
    require(inputs['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-input-v1'
            and inputs['source_generation'] == result['source_generation']
            and result['tool_pins'] == inputs['tool_pins']
            and len(inputs['files']) == len(result['final_sources']) == 1033, 'closed source generation')
    final, before = result['final_sources'], result['preformat_sources']
    require({name: compact(row) for name, row in before.items()} == inputs['files']
            and set(final) == set(before) and all(ordinary(name) and row['path'] == str(ROOT / name)
                for name, row in final.items()), 'source map path/identity closure')
    changed = [name for name in final if final[name] != before[name]]
    require(set(changed) <= set(inputs['overlay']), 'no source drift outside declared format overlay')
    if formatted:
        require(len(result['format_changed_paths']) == len(set(result['format_changed_paths']))
                and set(changed) == set(result['format_changed_paths']), 'original formatter result census')
    tree = ast.parse(bodies['run_cpu.py'])
    literal = lambda key: ast.literal_eval(next(node.value for node in tree.body if isinstance(node, ast.Assign)
        and any(isinstance(target, ast.Name) and target.id == key for target in node.targets)))
    history = literal('HISTORY')
    proposal_pins = {name: literal(key) for name, key in (
        ('runtime-proposal.json', 'RUNTIME_PROPOSAL_SHA'),
        ('primitive-proposal.json', 'PRIMITIVE_PROPOSAL_SHA'),
        ('consumer-proposal.json', 'CONSUMER_PROPOSAL_SHA'),
        ('selector-proposal.json', 'SELECTOR_PROPOSAL_SHA'),
        ('consumer-repair.json', 'CONSUMER_REPAIR_SHA'),
        ('selector-repair.json', 'SELECTOR_REPAIR_SHA'))}
    require(set(inputs['lineage']) == set(history) | set(proposal_pins)
            and set(result['readset']) == set(inputs['lineage']) and len(inputs['lineage']) == 16,
            'closed sixteen direct lineage records')
    for name, expected in inputs['lineage'].items():
        require(pin(bodies['inputs/' + name]) == expected == compact(result['readset'][name]), 'original lineage bytes')
        if name in history:
            require(expected == dict(bytes=history[name][0], sha256=history[name][1]), 'literal actual predecessor')
        else:
            require(expected['sha256'] == proposal_pins[name], 'literal frozen source proposal')
    rp, pp, cp, sp = (parse(bodies['inputs/' + name]) for name in ('runtime-proposal.json',
        'primitive-proposal.json', 'consumer-proposal.json', 'selector-proposal.json'))
    require(len(rp['files']) == 22 and rp['primitive_manifest'] == pin(bodies['inputs/primitive-proposal.json'])
            and len(pp['files']) == 6 and len(cp['files']) == 4 and len(sp['files']) == 9,
            'cumulative runtime, consumer and selector source chain')
    runtime_names = list(literal('RUNTIME_NEW_NAMES'))
    worker_names = literal('WORKER_NEW_NAMES')
    require(len(runtime_names) == len(set(runtime_names)) == 24
            and {name.rsplit('::', 1)[-1] for name in runtime_names} == set(rp['new_tests'])
            and worker_names == sp['composed_new_tests']
            and len(set(name for names in worker_names.values() for name in names)) == 22
            and sum(map(len, worker_names.values())) == 22, 'exact declared additive runtime/worker names')
    require(compact(sp['requires']['runtime']) == pin(bodies['inputs/runtime-proposal.json'])
            and compact(sp['requires']['consumer']) == pin(bodies['inputs/consumer-proposal.json'])
            and compact(sp['base']['worker_receipt']) == pin(bodies['inputs/worker-complete.json'])
            and compact(sp['base']['worker_sources']) == pin(bodies['inputs/worker-sources.json']),
            'selector actual baseline and required source manifests')
    repaired = parse(bodies['inputs/consumer-repair.json'])
    require(repaired['schema'] == cp['schema']
            and compact(repaired['predecessor_manifest']) == pin(bodies['inputs/consumer-proposal.json'])
            and repaired['source_generation'] == 'scoped-consumer-v2-module-path-repair'
            and repaired['declared_tests'] == cp['declared_tests'] == 7
            and all(repaired[key] is False for key in ('compiled', 'tested', 'native_execution',
                'canonical_changed', 'profile_enabled'))
            and len(repaired['files']) == len(cp['files']) == 4,
            'explicit repaired consumer with original selector lineage retained')
    repair = repaired['repair']
    require(repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/guarded_mlp_decode_v1/scoped_currentness_v1.rs',
        before_call='crate::native_forward::checked_layer_hidden_pair',
        after_call='crate::native_catalog::forward::checked_layer_hidden_pair',
        changed_occurrences=1, other_three_postimages_unchanged=True, tests_unchanged=True),
        'one declared registered-module path repair only')
    require(sorted(repaired['new_tests']['worker_library']) == sorted(
        name for names in sp['consumer_tests'].values() for name in names),
        'all seven original consumer names preserved')
    for old_row, new_row in zip(cp['files'], repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and (new_row['after'] != old_row['after'] if old_row['path'] == repair['path']
                     else new_row == old_row), 'one cumulative postimage transition')
    selector_repaired = parse(bodies['inputs/selector-repair.json'])
    require(compact(selector_repaired['predecessor_manifest']) == pin(bodies['inputs/selector-proposal.json'])
            and selector_repaired['source_generation'] == 'scoped-selector-v2-owned-reader-signature-repair'
            and all(selector_repaired[key] == value for key, value in sp.items()
                    if key not in ('files', 'artifacts'))
            and len(selector_repaired['files']) == len(sp['files']) == 9,
            'original selector contract and all test names preserved through test-only repair')
    selector_repair = selector_repaired['repair']
    require(selector_repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1_tests.rs',
        before_reader='&mut &[u8]', after_reader='&mut io::Cursor<Vec<u8>>', changed_occurrences=2,
        changed_test='scoped_warm_publication_deadline_requires_both_pre_and_post_write_to_be_inside',
        other_eight_postimages_unchanged=True, production_bodies_unchanged=True,
        test_names_unchanged=True, original_consumer_requirement_unchanged=True),
        'only the two explicit owned-reader test signatures may change')
    for old_row, new_row in zip(sp['files'], selector_repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and (new_row['after'] != old_row['after'] if old_row['path'] == selector_repair['path']
                     else new_row == old_row), 'one cumulative selector test postimage transition')
    previous_worker = parse(bodies['inputs/worker-complete.json'])
    previous_runtime = parse(bodies['inputs/runtime-complete.json'])
    prior_map = parse(bodies['inputs/worker-sources.json'])
    runtime_map = parse(bodies['inputs/runtime-sources.json'])
    require(previous_worker['passed'] is True and previous_runtime['passed'] is True
            and previous_worker['final_sources'] == previous_worker['input_sources'] == prior_map
            and previous_runtime['final_sources'] == previous_runtime['input_sources'] == runtime_map
            and {n: compact(v) for n, v in prior_map.items() if n.startswith('fe2o3/')}
                == {n: compact(v) for n, v in runtime_map.items() if n.startswith('fe2o3/')}
            and previous_worker['tool_pins'] == previous_runtime['tool_pins'] == inputs['tool_pins'],
            'actual independently qualified worker/runtime source and tool joins')
    expected = {name: compact(row) for name, row in prior_map.items() if name.startswith(('fe2o3/', WORKER))}
    require(len(expected) == 1020 and sum(n.startswith(WORKER) for n in expected) == 205,
            'actual 815-runtime/205-worker source base')
    overlay = set()
    primitive = {row['path']: row for row in pp['files']}
    for proposal, prefix in ((rp, 'fe2o3/'), (repaired, 'ferric/'), (selector_repaired, 'ferric/')):
        for row in proposal['files']:
            name = prefix + row['path']
            require(ordinary(row['path']) and name not in overlay and expected.get(name) == row['before'],
                    'qualified exact source preimage/absence')
            if proposal is rp and row['path'] in primitive:
                require(row['primitive_postimage'] == primitive[row['path']]['after']
                        and row['before'] == primitive[row['path']]['before'], 'primitive intermediate source join')
            expected[name] = row['after']
            overlay.add(name)
    require(len(overlay) == 35 and inputs['overlay'] == sorted(overlay)
            and sum(n.startswith('fe2o3/') for n in expected) == 820
            and sum(n.startswith(WORKER) for n in expected) == 209, 'closed routed source composition')
    expected.update({name: pin(bodies[name]) for name in HELPERS})
    require(expected == inputs['files'], 'no unlisted source changes')
    selected = set(inputs['overlay']) | set(HELPERS) | set(MANIFESTS)
    require(len(selected) == 46 and all(pin(bodies[name]) == compact(final[name]) for name in selected),
            'actual formatted postimages and exact helper/manifest bodies')
    require(pin(bodies['supervisor.py']) == compact(result['supervisor'])
            and pin(bodies['worker_support.py']) == compact(result['support'])
            and pin(bodies['cache.py']) == compact(result['cache_helper']), 'retained executed helpers')
    stage = parse(bodies['stage-complete.json'])
    require(stage['schema'] == 'ferric-guarded-mlp-scoped-currentness-stage-v1' and stage['passed'] is True
            and stage['controller'] == STAGER and stage['input'] == INPUT and stage['root'] == str(ROOT)
            and stage['archive'] == SOURCE_ARCHIVE
            and all(stage[key] is False for key in ('project_execution', 'canonical_changed', 'shared_cache_changed', 'lockfiles_changed'))
            and (stage['source_files'], stage['runtime_files'], stage['worker_files'], stage['overlay_files']) == (1033, 820, 209, 35),
            'actual source staging join')
    cache = parse(bodies['cargo-cache-manifest.json'])
    cache_stage = parse(bodies['cargo-cache-stage-complete.json'])
    require(pin(bodies['cargo-cache-manifest.json']) == CACHE == inputs['cache_manifest']
            and compact(result['cache_provenance']['manifest']) == CACHE
            and pin(bodies['cargo-cache-stage-complete.json']) == compact(result['cache_provenance']['stage'])
            and cache_stage['manifest'] == CACHE and cache_stage['passed'] is True
            and cache_stage['archive'] == result['cache_provenance']['archive'] == CACHE_ARCHIVE
            and cache_stage['files'] == cache['files'] and len(cache['files']) == 73
            and len(cache['locked_packages']) == 39 and cache_stage['packages'] == 39
            and cache_stage['controller'] == pin(bodies['cache.py'])
            and {name: compact(row) for name, row in result['cache_provenance']['files'].items()} == cache['files'],
            'actual private two-lock cache provenance')
    require(all(cache_stage[key] is False for key in ('shared_cache_changed', 'lock_changed',
            'project_code_executed', 'crate_sources_extracted')), 'private cache no-execution scope')
    require(cache_stage['locks'] == cache['locks'] == result['cache_provenance']['locks']
            and cache['locks'] == dict(runtime=pin(bodies['fe2o3/Cargo.lock']),
                                      worker=pin(bodies[WORKER + 'Cargo.lock']))
            and cache_stage['cargo_home'] == str(ROOT / 'cargo-home'), 'both exact original locks and private cache location')
    phases = result['phases']
    require(0 < len(phases) <= len(PHASES) and [row['label'] for row in phases] == list(PHASES[:len(phases)]), 'exact phase prefix')
    raw = result['raw']
    require(all(ordinary(name) and '/' not in name and pin(bodies['evidence/' + name]) == compact(row)
                and row['path'] == str(ROOT / 'evidence' / name) for name, row in raw.items()), 'original raw bodies')
    fixed_raw = {'sources-preformat.json', 'sources-after.json', 'dependencies-after.json',
                 'doc-parser-regression.json'}
    if formatted:
        fixed_raw.add('sources-before.json')
    if 'runtime-dependencies-before.json' in raw:
        fixed_raw.add('runtime-dependencies-before.json')
    if 'dependencies-before.json' in raw:
        fixed_raw.add('dependencies-before.json')
    expected_raw = fixed_raw | {row['label'] + suffix for row in phases
        for suffix in ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    require(set(raw) == expected_raw, 'no omitted or invented raw records')
    require(parse(bodies['evidence/sources-preformat.json']) == before
            and parse(bodies['evidence/sources-after.json']) == final, 'actual preformat/final source maps')
    if formatted:
        require(parse(bodies['evidence/sources-before.json']) == result['input_sources'], 'actual postformat source map')
    regression = parse(bodies['evidence/doc-parser-regression.json'])
    require(regression == result['doc_parser_tests'] and regression['passed'] == 8 and regression['failed'] == 0
            and regression['project_execution'] is False and regression['gpu_execution'] is False
            and regression['cases'] == [dict(name=name, outcome='ok') for name in (
                'actual_two_sections', 'historical_one_section', 'missing_named_result', 'duplicate_named_result',
                'wrong_section_subtotal', 'failed_named_result', 'duplicate_section_summary', 'missing_final_summary')],
            'eight actually executed data-only doc parser regressions')
    for index, row in enumerate(phases):
        label = row['label']
        command, started, saved = (parse(bodies['evidence/' + label + suffix])
                                  for suffix in ('.command.json', '.started.json', '.result.json'))
        require(saved == row and command['argv'] == started['argv'] == row['argv']
                and row['argv'][:6] == PRLIMIT and started['pid'] == started['pgid'] == row['pid'] == row['pgid']
                and row['reaped'] is True and row['process_group_absent'] is True
                and all(type(row[key]) is bool for key in ('natural_exit', 'forced_cleanup', 'timed_out')),
                'original owned phase is fully retired')
        clean = row['natural_exit'] is True and row['forced_cleanup'] is False and row['timed_out'] is False \
            and row['exception'] is None and row['storage_failure'] is None and row['observed_signals'] == []
        if result['passed'] or index < len(phases) - 1:
            require(clean and row['exit_code'] == 0, 'successful clean preceding phase')
        else:
            require(type(row['exit_code']) is int and type(row['observed_signals']) is list
                    and (row['exception'] is None or type(row['exception']) is str)
                    and (row['storage_failure'] is None or type(row['storage_failure']) is str),
                    'last failed leaf preserves original exception/storage/signal/exit metadata')
        require(command['cwd'] in (str(ROOT / 'fe2o3'), str(ROOT / WORKER))
                and 0 < command['wall_timeout_seconds'] <= 1800
                and all(command['env'][key] == '' for key in ('ROCR_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
                and command['env']['CARGO_HOME'] == str(ROOT / 'cargo-home')
                and command['env']['CARGO_NET_OFFLINE'] == 'true', 'bounded CPU-only private dependency environment')
        require(all(row[key] == raw[label + suffix] for key, suffix in
                    (('command', '.command.json'), ('stdout', '.stdout'), ('stderr', '.stderr'))), 'phase stream joins')
    for label, value in result['tests'].items():
        if label != 'interface-doc-tests':
            require(named(bodies['evidence/' + label + '.stdout'].decode()) == value, 'raw named test results')
    for role in ('runtime', 'worker'):
        require(named(bodies['inputs/' + role + '-tests.stdout'].decode()) == result['baseline_tests'][role], 'original baseline outcomes')
    for label, artifact in result['artifacts'].items():
        row = artifact['cargo_artifact']
        path = Path(artifact['pin']['path'])
        require(path.is_relative_to(ROOT / 'target') and row['executable'] == str(path)
                and row['filenames'].count(str(path)) == 1 and row['reason'] == 'compiler-artifact', 'selected actual product metadata')
        raw_label = 'worker-build' if label == 'worker' else 'worker-tests-build' if label.startswith('worker-') else 'kfd-tests-build'
        records = [parse(line) for line in bodies['evidence/' + raw_label + '.stdout'].splitlines() if line.startswith(b'{')]
        require(records.count(row) == 1, 'product belongs to original Cargo stream')
    cli = result['cli_executable_before_tests']
    if cli is not None:
        records = [parse(line) for line in bodies['evidence/worker-tests-build.stdout'].splitlines() if line.startswith(b'{')]
        row = cli['cargo_artifact']
        require(records.count(row) == 1 and row['profile']['test'] is False
                and row['target']['kind'] == ['bin'] and row['target']['name'] == 'ferric-tp-peer-finite-engineering-worker-v1'
                and row['target']['src_path'] == str(ROOT / WORKER / 'src/main.rs')
                and row['executable'] == cli['pin']['path'] == str(ROOT / 'target/debug/ferric-tp-peer-finite-engineering-worker-v1')
                and row['features'] == [], 'actual production ELF built before real CLI integration tests')
    if result['passed']:
        require(result['failure'] is None and formatted and len(phases) == 26 and len(raw) == 137 and len(result['artifacts']) == 11
                and result['full_runtime_tests_executed'] is True and result['full_worker_tests_executed'] is True
                and result['selected_facade_doctests_executed'] is True, 'complete conditional success extent')
        require(result['cli_executable_unchanged_across_tests'] is True
                and cli['pin'] == result['artifacts']['worker']['pin']
                and len({a['pin']['path'] for a in result['artifacts'].values()}) == 11
                and set(result['artifacts']) == {'kfd-lib', 'engineering-worker-test', 'guarded-facade-test',
                    'debug-trap-test', 'telemetry-env-test', 'telemetry-test', 'worker-lib', 'worker-bin-test',
                    'worker-readiness-test', 'worker-wire-test', 'worker'}, 'all11 products and pretest/posttest/final ELF join')
        runtime, worker = (result['tests'][label] for label in ('kfd-tests', 'worker-tests'))
        require((runtime['passed'], runtime['failed'], runtime['ignored']) == (1143, 0, 8)
                and (worker['passed'], worker['failed'], worker['ignored']) == (726, 0, 4),
                'full actual runtime and activated worker census')
        for role, current, new, total in (('runtime', runtime, runtime_names, 1151),
                ('worker', worker, [n for names in worker_names.values() for n in names], 730)):
            old = {row['name']: row['outcome'] for row in result['baseline_tests'][role]['named']}
            require(not set(old).intersection(new), 'only additive named test outcomes')
            expected_names = old | {name: 'ok' for name in new}
            require({row['name']: row['outcome'] for row in current['named']} == expected_names
                    and len(expected_names) == total and result['inventories'][role] == sorted(expected_names),
                    'all prior outcomes and new names, not counts alone')
            summaries = [dict(row) for row in result['baseline_tests'][role]['summaries']]
            additions = [24, 0, 0, 0, 0, 0] if role == 'runtime' else [21, 0, 1, 0]
            require(len(summaries) == len(additions), 'original per-target test summaries')
            for summary, extra in zip(summaries, additions):
                summary['passed'] += extra
            require(current['summaries'] == summaries, 'exact unchanged and additive target summaries')
        for label, count in (('terminal-pair-tests', 9), ('peer-read-pair-tests', 7), ('arena-reuse-tests', 7),
                             ('arena-memory-tests', 2), ('retained-tests', 20), ('mixed-bank-tests', 13),
                             ('scoped-checkpoint-tests', 16), ('scoped-group-tests', 1), ('scoped-layer-tests', 7)):
            value = result['tests'][label]
            require((value['passed'], value['failed'], value['ignored']) == (count, 0, 0),
                    'actual focused scope count')
        docs = result['tests']['interface-doc-tests']
        text = bodies['evidence/interface-doc-tests.stdout'].decode()
        rows = re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)
        require(rows == [tuple(row) for row in docs['named']] and len(rows) == docs['passed'] == 10
                and all(status == 'ok' for _, status in rows)
                and sum(name.endswith(' - compile fail') for name, _ in rows) == 9, 'ten actual selected docs')
        summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                               r'(\d+) measured; (\d+) filtered out;', text, re.M)
        require([list(row) for row in summaries] == docs['summaries']
                and sorted(row[1] for row in summaries) == ['1', '9']
                and all(row[0] == 'ok' and row[2:5] == ('0', '0', '0') for row in summaries), 'exact two successful doc sections')
    else:
        require(type(result['failure']) is str and result['failure'], 'failed terminal remains failed')
    expected_bodies = selected | {'input-manifest.json', 'transport.py', 'stage-complete.json',
        'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json', 'exporter.py', 'evidence/' + terminal_name}
    expected_bodies |= {'inputs/' + name for name in inputs['lineage']} | {'evidence/' + name for name in raw}
    require(set(bodies) == expected_bodies and len(bodies) <= MAX_MEMBERS
            and sum(map(len, bodies.values())) <= MAX_BODY, 'closed bounded retained selection')
    if result['passed']:
        require(len(bodies) == 206, '206 pinned success bodies plus one manifest')
    return result, inputs


def export(terminal_sha, stager_path):
    require(os.getuid() == os.geteuid() == 9661 and ROOT.resolve(strict=True) == ROOT
            and not os.path.lexists(ARCHIVE), 'fresh remote export')
    terminals = [name for name in ('complete.json', 'failed.json') if os.path.lexists(ROOT / 'evidence' / name)]
    require(len(terminals) == 1, 'single original terminal')
    terminal_name = terminals[0]
    original = read(ROOT / 'evidence' / terminal_name)
    require(pin(original)['sha256'] == terminal_sha, 'actual terminal SHA argument')
    result = parse(original)
    inputs = parse(read(ROOT / 'input-manifest.json'))
    bodies, readset = {}, {}
    def select(name, path, expected=None):
        require(name not in bodies and ordinary(name), 'unique selected evidence member')
        raw = read(path)
        require(expected is None or pin(raw) == compact(expected), 'selected body pin')
        bodies[name], readset[str(path)] = raw, pin(raw)
    select('evidence/' + terminal_name, ROOT / 'evidence' / terminal_name)
    for name, row in result['raw'].items():
        require(ordinary(name) and '/' not in name, 'raw basename')
        select('evidence/' + name, ROOT / 'evidence' / name, row)
    selected = set(inputs['overlay']) | set(HELPERS) | set(MANIFESTS)
    for name in selected:
        select(name, ROOT / name, result['final_sources'][name])
    for name, row in inputs['lineage'].items():
        select('inputs/' + name, ROOT / 'inputs' / name, row)
    for name in ('input-manifest.json', 'stage-complete.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json'):
        select(name, ROOT / name)
    select('transport.py', stager_path, STAGER)
    select('exporter.py', Path(__file__).resolve())
    result, inputs = verify(bodies, terminal_name, terminal_sha)
    final = result['final_sources']
    actual_sources = {str(path.relative_to(ROOT)) for subtree in ('fe2o3', 'ferric') for path in files_below(ROOT / subtree)} | set(HELPERS)
    require(actual_sources == set(final), 'full1033 live source closure')
    for name, row in final.items():
        path = ROOT / name
        require(pin(read(path)) == compact(row), 'full source hash: ' + name)
        readset[str(path)] = compact(row)
    require({path.name for path in (ROOT / 'evidence').iterdir()} == set(result['raw']) | {terminal_name}, 'exact original evidence directory')
    for row in result['tool_pins'].values():
        require(file_pin(Path(row['path'])) == compact(row), 'qualified tool unchanged before export')
        readset[row['path']] = compact(row)
    for row in result['cache_provenance']['files'].values():
        require(pin(read(Path(row['path']))) == compact(row), 'immutable private cache input')
        readset[row['path']] = compact(row)
    dependencies = parse(bodies['evidence/dependencies-after.json'])
    expected_dependencies = {}
    for name in ('runtime-dependencies-before.json', 'dependencies-before.json'):
        if 'evidence/' + name in bodies:
            for root, rows in parse(bodies['evidence/' + name]).items():
                require(root not in expected_dependencies or expected_dependencies[root] == rows, 'both locked dependency resolutions')
                expected_dependencies[root] = rows
    require(dependencies == expected_dependencies, 'complete final dependency snapshot')
    for directory, rows in dependencies.items():
        root = Path(directory)
        require(root.is_relative_to(ROOT / 'cargo-home/registry/src') and
                {str(path.relative_to(root)) for path in files_below(root)} == set(rows), 'exact private resolved package tree')
        for name, row in rows.items():
            path = root / name
            require(pin(read(path)) == compact(row), 'resolved dependency unchanged')
            readset[str(path)] = compact(row)
    artifact_presence = {}
    for name, artifact in result['artifacts'].items():
        path = Path(artifact['pin']['path'])
        artifact_presence[name] = os.path.lexists(path)
        if artifact_presence[name]:
            require(file_pin(path) == compact(artifact['pin']), 'actual selected ELF unchanged')
            readset[str(path)] = compact(artifact['pin'])
    require(not result['passed'] or all(artifact_presence.values()), 'all eleven successful products remain present')
    cli = result['cli_executable_before_tests']
    if cli is not None:
        require(file_pin(Path(cli['pin']['path'])) == compact(cli['pin']), 'pretest real executable still exact')
        readset[cli['pin']['path']] = compact(cli['pin'])
    require(all(file_pin(Path(path)) == row for path, row in readset.items()), 'entire export readset posthash')
    manifest = dict(schema='ferric-guarded-mlp-scoped-currentness-cpu-retained-v1', terminal_name=terminal_name,
        terminal=pin(original), passed=result['passed'], failure=result['failure'], phases=len(result['phases']),
        raw_count=len(result['raw']), source_map_rows=len(final), retained_runtime_overlay_files=22,
        retained_worker_overlay_files=13, postformat_snapshot_observed=result['input_sources'] is not None,
        observed_overlay_changes=sorted(n for n in final if final[n] != result['preformat_sources'][n]),
        artifact_presence=artifact_presence, full_live_source_and_dependency_posthash=True,
        host_executable_bodies_retained=False, crate_bodies_retained=False, gpu_execution=False,
        numerical_acceptance=False, performance_claim=False,
        files={name: pin(body) for name, body in sorted(bodies.items())})
    bodies['manifest.json'] = encoded(manifest)
    require(len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_BODY, 'capsule final bounds')
    with ARCHIVE.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, body in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o600, 0
                tar.addfile(info, io.BytesIO(body))
    print(json.dumps(dict(archive=dict(path=str(ARCHIVE), **pin(read(ARCHIVE))), members=len(bodies),
        expanded_bytes=sum(map(len, bodies.values())), passed=result['passed'], failure=result['failure']), sort_keys=True))


def retain(archive_path, archive_sha, terminal_sha, destination):
    require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
            and not os.path.lexists(destination), 'fresh retained destination')
    raw = read(archive_path)
    require(pin(raw)['sha256'] == archive_sha, 'observed exact archive hash')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(0 < len(members) <= MAX_MEMBERS and len({m.name for m in members}) == len(members)
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers and 0 <= m.size <= 32 << 20 for m in members)
                and sum(m.size for m in members) <= MAX_BODY, 'bounded regular unique USTAR capsule')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'capsule member bytes')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    require(manifest['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-retained-v1'
            and manifest['files'] == {name: pin(body) for name, body in bodies.items()}, 'exact manifest closure')
    require(bodies['exporter.py'] == read(Path(__file__).resolve()), 'same reviewed exporter/retainer source')
    result, _ = verify(bodies, manifest['terminal_name'], terminal_sha)
    require(manifest['terminal'] == pin(bodies['evidence/' + manifest['terminal_name']])
            and manifest['passed'] == result['passed'] and manifest['failure'] == result['failure']
            and manifest['phases'] == len(result['phases']) and manifest['raw_count'] == len(result['raw'])
            and manifest['source_map_rows'] == 1033 and manifest['retained_runtime_overlay_files'] == 22
            and manifest['retained_worker_overlay_files'] == 13
            and manifest['postformat_snapshot_observed'] is (result['input_sources'] is not None)
            and manifest['observed_overlay_changes'] == sorted(n for n in result['final_sources']
                if result['final_sources'][n] != result['preformat_sources'][n])
            and set(manifest['artifact_presence']) == set(result['artifacts'])
            and all(type(value) is bool for value in manifest['artifact_presence'].values())
            and (not result['passed'] or all(manifest['artifact_presence'].values()))
            and manifest['full_live_source_and_dependency_posthash'] is True
            and all(manifest[key] is False for key in ('host_executable_bodies_retained', 'crate_bodies_retained',
                'gpu_execution', 'numerical_acceptance', 'performance_claim')), 'truthful retention scope')
    require(read(archive_path) == raw, 'archive stable through local verification')
    bodies['manifest.json'] = manifest_raw
    destination.mkdir(parents=True, mode=0o700)
    require(destination.resolve(strict=True) == destination, 'ordinary fresh destination')
    for name, body in sorted(bodies.items()):
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with path.open('xb') as stream:
            stream.write(body)
        require(read(path) == body, 'retained bytes exact')
    print(json.dumps(dict(destination=str(destination), archive=pin(raw), members=len(bodies),
                         passed=result['passed'], failure=result['failure']), sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B evidence.py export TERMINAL_SHA STAGER_PATH | retain ARCHIVE ARCHIVE_SHA TERMINAL_SHA DEST')
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 768 << 20), (resource.RLIMIT_CORE, 0),
                      (resource.RLIMIT_FSIZE, MAX_BODY)):
        before = resource.getrlimit(kind)
        bound = min([cap] + [n for n in before if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (bound, bound))
    def interrupted(number, _frame):
        raise RuntimeError('evidence helper signal ' + str(number))
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        require(len(sys.argv) in (4, 6), 'closed evidence CLI')
        if sys.argv[1] == 'export':
            require(len(sys.argv) == 4 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'actual terminal SHA')
            export(sys.argv[2], Path(sys.argv[3]))
        else:
            require(len(sys.argv) == 6 and sys.argv[1] == 'retain'
                    and all(re.fullmatch('[0-9a-f]{64}', value) for value in sys.argv[3:5]), 'actual archive/terminal hashes')
            retain(Path(sys.argv[2]), sys.argv[3], sys.argv[4], Path(sys.argv[5]))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
