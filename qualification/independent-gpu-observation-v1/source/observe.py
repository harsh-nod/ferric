"""No-launch replay of independent native captures and conditional references."""
import hashlib
from pathlib import Path
import re
import sys
import types

import validation as V
import child_evidence as C

PLAN_SCHEMA = 'ferric-p228-independent-profile-observation-inputs-v1'
RESULT_SCHEMA = 'ferric-p228-independent-profile-conditional-observation-v1'
INSPECTION_SCHEMA = 'fe2o3-qwen-prefix-tiles-independent-profiles-inspection-v1'
OBSERVATION_SCHEMA = 'fe2o3-qwen-prefix-tiles-independent-profiles-observation-v1'
INSPECT = '--inspect-independent-profiles-v1'
EXECUTE = '--execute-reviewed-engineering-prefix-independent-profiles-v1'
PROFILES = ('baseline_v5', 'tiles_v6')
STREAM_CAP = 8 << 20
LEAF_SECONDS = 180
EXTRA_FIELDS = {'success', 'native_execution_attempted', 'paired_comparison_performed',
                'immutable_input_readbacks_match', 'profiles', 'captures'}
INSPECTION_FIELDS = V.BASE_FIELDS | EXTRA_FIELDS
OBSERVATION_FIELDS = INSPECTION_FIELDS | {'timing_boundary'}
CHANGED_FIELDS = {'schema', 'success', 'opened_device', 'native_execution_attempted',
                  'completed_and_closed', 'immutable_input_readbacks_match', 'profiles', 'captures'}
NUMERICAL_SHA = '4b72aeac6167006151b258536cc834f39e7a55da15ec10744df2af438bf3b46d'
VALIDATION_SHA = 'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1'
CHILD_EVIDENCE_SHA = '2cbb74ada0950d1767c8b526008e9d9c7efb48d84649e1aea8f3d6661309edd3'
FALSE_FIELDS = ('independent_numerical_acceptance', 'full_prefix_acceptance',
    'full_model_correctness', 'performance_claim', 'production_authority',
    'runtime_premises_discharged', 'scheduler_progress_guaranteed',
    'all_workgroups_participated', 'paired_comparison_performed', 'bitwise_match',
    'deployment_verified', 'platform_audits_verified', 'arithmetic_prerequisites_verified',
    'gpu_execution_verified', 'historical_kv_numerics_checked', 'untouched_kv_bytes_rechecked')


def inspection(value, requested, request_pin, baseline, case):
    V.keys(value, INSPECTION_FIELDS)
    V.base(value, requested, request_pin, baseline, case)
    V.require(value['schema'] == INSPECTION_SCHEMA and all(value[key] is False for key in
        ('success', 'opened_device', 'native_execution_attempted', 'completed_and_closed',
         'immutable_input_readbacks_match', 'paired_comparison_performed', 'bitwise_match'))
        and value['profiles'] == [] and value['captures'] == [], 'inert independent inspection')
    return value


def observation(value, inspected, requested, request_pin, baseline, case, captures):
    """Validate complete captures separately; never compare either profile's bits."""
    V.keys(value, OBSERVATION_FIELDS)
    inspection(inspected, requested, request_pin, baseline, case)
    V.base(value, requested, request_pin, baseline, case)
    V.require(value['schema'] == OBSERVATION_SCHEMA and all(value[key] is True for key in
        ('success', 'opened_device', 'native_execution_attempted', 'completed_and_closed',
         'immutable_input_readbacks_match')) and value['paired_comparison_performed'] is False
        and value['bitwise_match'] is False and value['timing_boundary'] == V.TIMING,
        'closed independent native observation, not parity success')
    for key in INSPECTION_FIELDS - CHANGED_FIELDS:
        V.require(value[key] == inspected[key], 'inspection/execution identity changed')
    V.require(type(value['profiles']) is list and len(value['profiles']) == 2,
              'two ordered native profiles')
    owners = []
    for index, name in enumerate(PROFILES):
        row = value['profiles'][index]
        V.keys(row, 'profile states host_dispatch_elapsed_ns closed')
        V.require(row['profile'] == name and row['closed'] is True
            and type(row['states']) is list and len(row['states']) == 2
            and type(row['host_dispatch_elapsed_ns']) is list
            and len(row['host_dispatch_elapsed_ns']) == 2, 'ordered closed profile/ranks')
        owners.append([V.terminal(words, index) for words in row['states']])
        for elapsed in row['host_dispatch_elapsed_ns']:
            V.uint(elapsed)
    V.require(type(value['captures']) is list and len(value['captures']) == 2,
              'two complete capture pairs')
    expected_paths, stage_hashes = set(), []
    _, position = V.case_scope(case)
    slot = ((position // 16 * 5 + 7) % 144) * 16 + position % 16
    for index, label in enumerate(('baseline-v5', 'tiles-v6')):
        pair = value['captures'][index]
        V.require(type(pair) is list and len(pair) == 2, 'two ordered full rank captures')
        rows = []
        for rank, record in enumerate(pair):
            V.pin(record, V.CAPTURE_BYTES)
            path = str(Path(requested['capture_directory']) / (label + '-rank' + str(rank) + '.bin'))
            V.require(record['path'] == path and record['bytes'] == V.CAPTURE_BYTES,
                      'profile/rank capture path and extent')
            expected_paths.add(path)
            raw = captures.get(path)
            V.require(type(raw) is bytes and len(raw) == V.CAPTURE_BYTES
                      and V.sha(raw) == record['sha256'], 'entire capture pin')
            hashes, offset = [], 0
            for stage, extent in enumerate(V.EXTENTS[7:]):
                body = raw[offset:offset + extent]
                offset += extent
                if stage in (3, 4):
                    V.finite(body[slot * 1024:(slot + 1) * 1024], 2)
                else:
                    V.finite(body, 4 if stage == 6 else 2)
                hashes.append(V.sha(body))
            rows.append(hashes)
        stage_hashes.append(rows)
    V.require(type(captures) is dict and set(captures) == expected_paths, 'exact four capture bodies')
    return dict(capture_bytes=V.CASE_CAPTURE_BYTES, stage_sha256=stage_hashes,
        observed_owners=owners, all_typed_terminal_words_checked=True,
        native_close_reported=True, full_capture_bytes_hashed=True,
        paired_comparison_performed=False, bitwise_match=False,
        independent_numerical_acceptance=False, performance_claim=False, production_authority=False)


def numerical_plans(observed, requested, case, output_weights):
    V.require(type(output_weights) is list and len(output_weights) == 2, 'two original output-weight pins')
    for rank, record in enumerate(output_weights):
        V.pin(record, V.EXTENTS[6])
        V.require(record['bytes'] == V.EXTENTS[6]
            and record['sha256'] == observed['input_sha256'][rank][6], 'native O weight input identity')
    V.require(output_weights[0]['path'] != output_weights[1]['path'], 'distinct rank-local output weights')
    return [dict(schema='ferric-p228-prefix-profile-numerical-inputs-v1', profile=profile,
        case=case, baseline_request=dict(requested['baseline_request']), ranks=[dict(rank=rank,
            capture=dict(observed['captures'][index][rank]),
            input_sha256=list(observed['input_sha256'][rank]),
            output_weights=dict(output_weights[rank])) for rank in range(2)])
        for index, profile in enumerate(PROFILES)]


def plan_shape(plan, evidence_root):
    V.keys(plan, 'schema case case_directory request binary inspection_result native_result output_weights')
    V.require(plan['schema'] == PLAN_SCHEMA, 'explicit no-launch independent observation input')
    V.case_scope(plan['case'])
    V.require(type(plan['case_directory']) is str, 'case directory string')
    directory = Path(plan['case_directory'])
    V.require(str(directory) == plan['case_directory']
        and directory.is_absolute() and '..' not in directory.parts and directory.name == plan['case']
        and directory.parent.parent == evidence_root
        and re.fullmatch(r'prefix-independent-profile-gpu-v228-v[1-9][0-9]{0,8}', directory.parent.name),
        'new independently labelled retained case, not historical parity provenance')
    for name in ('request', 'binary', 'inspection_result', 'native_result'):
        V.pin(plan[name], 64 << 20 if name == 'binary' else 64 << 10)
    for leaf in ('inspection', 'native'):
        V.require(plan[leaf + '_result']['path'] == str(directory / leaf / 'result.json'),
                  'owned leaf result location')
    return directory


def replay_leaf(P, pins, result, binary, request, directory, mode, gpu):
    """Same natural-exit and exact command gates as frozen run_case.replay_leaf."""
    V.require(type(result['exit_code']) is int and result['exit_code'] == 0
        and result['reason'] is None and result['owned_groups_absent'] is True
        and result['owned_processes_reaped'] is True and result['cleanup_signalled'] is False,
        'natural owned leaf exit, absent groups and reaped processes')
    for name, filename in (('command', 'command.json'), ('started', 'started.json'),
                           ('stdout', 'stdout'), ('stderr', 'stderr')):
        V.pin(result[name], STREAM_CAP, nonempty=name != 'stderr')
        V.require(result[name]['path'] == str(directory / filename), 'exact retained leaf member')
    command = P.document(pins, result['command'])
    expected = dict(argv=[binary['path'], mode, request['path'], request['sha256']],
        env=P.ENV, cwd=str(P.R), deadline_seconds=LEAF_SECONDS, affinity=[8, 9], nice=10,
        address_space_bytes=12 << 30, stream_cap_bytes=STREAM_CAP, gpu_execution_requested=gpu)
    V.require(command == expected, 'unchanged bounded leaf envelope and explicit new selector')
    started = P.document(pins, result['started'])
    V.require(started['command_sha256'] == result['command']['sha256']
        and result['gpu_execution_requested'] is gpu and result['stderr']['bytes'] == 0,
        'original command/start/mode/empty stderr')
    P.read(pins, result['stderr'])
    return V.parse(P.read(pins, result['stdout'], True, 64 << 10))


def capture_bytes(P, pins, requested, observed):
    """Keep run_case's complete capture census; never truncate the KV buffers."""
    V.require(type(observed.get('captures')) is list and len(observed['captures']) == 2,
              'two retained native capture pairs')
    directory, result = Path(requested['capture_directory']), {}
    for index, label in enumerate(('baseline-v5', 'tiles-v6')):
        pair = observed['captures'][index]
        V.require(type(pair) is list and len(pair) == 2, 'two capture ranks')
        for rank, record in enumerate(pair):
            V.pin(record, V.CAPTURE_BYTES)
            path = directory / (label + '-rank' + str(rank) + '.bin')
            V.require(record['path'] == str(path) and record['bytes'] == V.CAPTURE_BYTES,
                      'closed capture filename/extent')
            result[str(path)] = P.read(pins, record, True, V.CAPTURE_BYTES)
    V.require(directory.is_dir() and not directory.is_symlink()
        and {str(path) for path in directory.iterdir()} == set(result), 'exact four retained capture files')
    return result


def load_numerical(profile_root, stage_root, sidecar_root):
    path = Path(profile_root) / 'compare_profile.py'
    V.require(path.is_absolute() and path.resolve(strict=True) == path and path.is_file(),
              'canonical immutable numerical module')
    with path.open('rb') as stream:
        raw = stream.read((1 << 20) + 1)
    V.require(len(raw) <= 1 << 20 and hashlib.sha256(raw).hexdigest() == NUMERICAL_SHA,
              'unchanged numerical module, not an alternate tolerance policy')
    module = types.ModuleType('independent_profile_numerical')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.source_pin = dict(path=str(path), bytes=len(raw), sha256=NUMERICAL_SHA)
    return module, module.helpers(stage_root, sidecar_root)


def numerical_result(result, plan, stage_hashes, numerical):
    V.require(result['schema'] == 'ferric-p228-prefix-profile-conditional-numerical-v1'
        and result['profile'] == plan['profile'] and result['case'] == plan['case']
        and result['conditional_operator_checks_passed'] is True
        and result['paired_comparison_performed'] is False
        and all(result[key] is False for key in numerical.FALSE_FIELDS),
        'unchanged reference returned conditional checks only')
    V.require(type(result['rows']) is list and len(result['rows']) == 2, 'two actual numerical ranks')
    for rank, row in enumerate(result['rows']):
        V.require(type(row['rank']) is int and row['rank'] == rank
            and row['profile'] == plan['profile'] and row['capture'] == plan['ranks'][rank]['capture']
            and row['stage_sha256'] == stage_hashes[rank]
            and row['conditioning']['output_weights'] == plan['ranks'][rank]['output_weights'],
            'numerical result joins this profile, capture and weight shard')


class NumericalCustodyError(RuntimeError):
    """A requested input could not be authenticated; not a numerical rejection."""


class CustodyReader:
    def __init__(self, P, pins, read):
        self.P, self.pins, self.read = P, pins, read
        self.records = {}

    def __call__(self, record, maximum=64 << 10):
        try:
            V.pin(record, maximum)
            previous = self.records.setdefault(record['path'], dict(record))
            V.require(previous == record, 'conflicting requested numerical input identity')
            # The original Reader registers only after a successful read. Keep
            # the attempted pin and fail fatally even on its first file error.
            self.P.read(self.pins, record, maximum=maximum)
            raw = self.read(record, maximum)
            V.require(type(raw) is bytes and len(raw) == record['bytes']
                      and V.sha(raw) == record['sha256'], 'numerical Reader returned exact pinned bytes')
            return raw
        except NumericalCustodyError:
            raise
        except Exception as failure:
            raise NumericalCustodyError('numerical input custody: ' + type(failure).__name__
                                        + ': ' + str(failure)) from failure


def compare_plans(plans, stage_hashes, P, pins, read, numerical, frozen):
    results, guarded = [], CustodyReader(P, pins, read)
    for index, expected in enumerate(plans):
        result, error = None, None
        try:
            result = numerical.compare_profile(expected, guarded, frozen)
        except NumericalCustodyError:
            raise
        except (AssertionError, RuntimeError, ValueError) as failure:
            error = (type(failure).__name__ + ': ' + str(failure))[:2048]
        if result is not None:
            numerical_result(result, expected, stage_hashes[index], numerical)
            for record in result['input_pins'].values():
                P.read(pins, record, maximum=max(record['bytes'], 64 << 10))
        for record in read.records.values():
            P.read(pins, record, maximum=max(record['bytes'], 64 << 10))
        for record in guarded.records.values():
            P.read(pins, record, maximum=max(record['bytes'], 64 << 10))
        # Keep the profiles independent even when one reference rejects. A
        # custody recheck failure is fatal, never a numerical rejection to skip.
        read.recheck()
        pins.recheck()
        results.append(dict(profile=expected['profile'], conditional_operator_checks_passed=result is not None,
                            error=error, checked=result))
    return results


def compare_retained(plan, P, pins, read, profile_root, stage_root, sidecar_root):
    """No launch or publication. P/pins are the frozen custody API; read is Reader.

    A future versioned controller must supply reviewed deployment/platform inputs
    and preserve its three pre/post audits. This function confers neither gate.
    """
    V.require(not sys.flags.optimize, 'reference assertions must remain enabled')
    for module, digest in ((V, VALIDATION_SHA), (C, CHILD_EVIDENCE_SHA)):
        path = Path(module.__file__)
        P.read(pins, dict(path=str(path), bytes=path.stat().st_size, sha256=digest), maximum=64 << 10)
    directory = plan_shape(plan, P.E)
    requested = P.document(pins, plan['request'], 64 << 10)
    baseline = P.document(pins, requested['baseline_request'], 64 << 10)
    V.request(requested, baseline, plan['case'])
    V.require(requested['capture_directory'] == str(directory / 'captures'), 'same closed case capture directory')
    for kind, record in zip(V.KINDS, requested['reviews']):
        V.review(P.document(pins, record, 64 << 10), kind, requested, baseline, plan['case'])
    P.read(pins, plan['binary'], maximum=64 << 20)
    inspect_result = P.document(pins, plan['inspection_result'], 64 << 10)
    native_result = P.document(pins, plan['native_result'], 64 << 10)
    inspected = replay_leaf(P, pins, inspect_result, plan['binary'], plan['request'],
                            directory / 'inspection', INSPECT, False)
    inspection(inspected, requested, plan['request'], baseline, plan['case'])
    observed = replay_leaf(P, pins, native_result, plan['binary'], plan['request'],
                           directory / 'native', EXECUTE, True)
    checked = observation(observed, inspected, requested, plan['request'], baseline, plan['case'],
                          capture_bytes(P, pins, requested, observed))
    children = C.validate(P, pins, directory, plan['request'], plan['binary'], observed, native_result)
    child_files = C.records(P, pins, directory, True)
    V.require(set(child_files) == set(C.NAMES), 'all ten unchanged child sidecars retained')
    plans = numerical_plans(observed, requested, plan['case'], plan['output_weights'])
    # Neither a finite value nor successful lifecycle replay grants arithmetic
    # qualification. Each unchanged reference independently checks its profile.
    pins.recheck()
    read.recheck()
    numerical, frozen = load_numerical(profile_root, stage_root, sidecar_root)
    P.read(pins, numerical.source_pin, maximum=1 << 20)
    results = compare_plans(plans, checked['stage_sha256'], P, pins, read, numerical, frozen)
    retained_children = C.records(P, pins, directory, True)
    V.require(retained_children == child_files, 'child sidecar identities changed')
    capture_bytes(P, pins, requested, observed)
    read.recheck()
    pins.recheck()
    return dict(schema=RESULT_SCHEMA, authority='none', case=plan['case'],
        request=dict(plan['request']), binary=dict(plan['binary']),
        inspection_result=dict(plan['inspection_result']), native_result=dict(plan['native_result']),
        closed_capture_checks=checked, profile_children=children, retained_profile_files=child_files,
        conditional_operator_checks_passed=all(row['conditional_operator_checks_passed'] for row in results),
        profiles=results,
        native_attempts_replayed=1, profile_attempts_replayed=2, retries=0,
        reference_module_sha256=NUMERICAL_SHA, original_input_pins=dict(pins.records),
        **{name: False for name in FALSE_FIELDS})
