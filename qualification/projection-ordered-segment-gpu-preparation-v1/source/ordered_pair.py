"""Two sequential owned arms; host-counter attribution only for matching histories."""
import os
from pathlib import Path
import re
import sys
import time

import intake as I
import run as M
import host_validation as H

V = I.V
PAIR_SCHEMA = 'ferric-p228-projection-ordered-segment-pair-inputs-v1'
RESULT_SCHEMA = 'ferric-p228-projection-ordered-segment-pair-v1'
PAIR_SECONDS = 2 * M.CASE_SECONDS + 120


def plan_shape(value):
    V.keys(value, 'schema output_label shared ordered')
    V.require(value['schema'] == PAIR_SCHEMA and type(value['output_label']) is str
        and re.fullmatch(r'prefix-projection-ordered-segment-pair-v228-v[1-9][0-9]{0,8}', value['output_label']),
        'closed fresh paired output namespace')
    for name in ('shared', 'ordered'):
        V.keys(value[name], 'path bytes sha256')
        H.pin(value[name])
    V.require(value['shared'] != value['ordered'], 'two distinct reviewed arm plans')


def same_inputs(shared, ordered):
    V.require(shared['route'] == 'shared' and ordered['route'] == 'ordered', 'ordered explicit paired policies')
    a, b = shared['plan'], ordered['plan']
    differing = {'route', 'output_label', 'parent', 'request', 'decode_review', 'parent_runtime_review'}
    V.require(set(a) == set(b) and all(H.same(a[k], b[k]) for k in set(a) - differing),
              'same CPU generation, worker, images, pure evidence and provenance')
    V.require(shared['out'] != ordered['out'] and a['request'] != b['request']
        and a['parent'] != b['parent'] and a['parent_runtime_review'] != b['parent_runtime_review'],
        'distinct reviewed parent routes and isolated outputs')
    left, right = shared['request'], ordered['request']
    V.require(set(left) == set(right) == {'schema', 'decode', 'projection_residual_image'}
        and H.same(left['projection_residual_image'], right['projection_residual_image'])
        and left['schema'] == 'FerricFiniteProjectionResidualDecodeRequestV1'
        and right['schema'] == 'FerricFiniteProjectionResidualMlpOrderedRequestV1', 'identical additive image and request family')
    left, right = left['decode'], right['decode']
    V.require(set(left) == set(right) and left['mode'] == right['mode'] == 'autoregressive'
        and left['session'] != right['session'] and left['evidence_directory'] != right['evidence_directory']
        and all(H.same(left[k], right[k]) for k in set(left) - {'session', 'evidence_directory'}),
        'same model, prompt, seed, worker, devices and bounds; fresh separate sessions')


def full_currentness(interval):
    V.array(interval['shared'], 4, H.U64)
    V.require(type(interval['ranks']) is list and len(interval['ranks']) == 2, 'two currentness ranks')
    for row in interval['ranks']:
        V.array(row, 19, H.U64)
    rank = sum(row[3] for row in interval['ranks'])
    group, publication = interval['shared'][1], interval['shared'][3]
    return dict(rank_full_ns=rank, group_full_ns=group, publication_full_ns=publication,
                component_sum_ns=rank + group + publication)


def comparison(shared, ordered, shared_summary, ordered_summary, payload_equal):
    for expected, arm in (('shared', shared), ('ordered', ordered)):
        V.require(arm['schema'] == ('ferric-p228-projection-ar4-shared-host-gpu-v1' if expected == 'shared' else
            'ferric-p228-projection-ordered-segment-gpu-v1')
            and arm['route'] == expected and arm['passed'] is True and arm['failures'] == []
            and arm['native_attempts'] == 1 and arm['retries'] == 0
            and arm['captured_payloads'] == 4 and arm['captured_tensor_rows'] == 152
            and arm['own_output_trajectory_checked'] is True and arm['performance_claim'] is False,
            'two actually successful owned AR4 arms')
    V.require(shared['parent_cpu_complete'] == ordered['parent_cpu_complete']
        and shared['worker_cpu_complete'] == ordered['worker_cpu_complete']
        and shared['worker'] == ordered['worker']
        and all(shared[key] == ordered[key] for key in ('projection_image', 'prefix_image', 'mlp_image')),
        'actual same qualified generation and images')
    events = [[frame['response']['event'] for frame in summary['files']['frames']]
              for summary in (shared_summary, ordered_summary)]
    V.require(all(len(rows) == 4 and all(row['status'] == 'completed' for row in rows) for rows in events),
              'four actual completion events per arm')
    histories = [[(row['position'], row['generation'], row['input_token']) for row in rows] for rows in events]
    V.require(histories[0] == histories[1], 'host attribution requires identical actual own-output histories')
    V.require(type(payload_equal) is list and len(payload_equal) == 4
        and all(type(flag) is bool for flag in payload_equal), 'four actual byte-repeatability observations')
    hosts = [shared['host_checked'], ordered['host_checked']]
    V.require(hosts[0]['schema'] == 'ferric-p228-projection-ar4-shared-host-counters-v1'
        and hosts[1]['schema'] == 'ferric-p228-projection-ordered-segment-host-counters-v1'
        and hosts[0]['policy'] == 'shared-full' and hosts[1]['policy'] == 'ordered-shared-full'
        and hosts[0]['configuration_time_in_snapshots'] is False and hosts[1]['configuration_time_in_snapshots'] is False,
        'separate legacy and ordered host schemas')
    V.uint(hosts[0]['configuration_host_ns']); V.uint(hosts[1]['configuration_host_ns'])
    for policy, host in zip((True, True), hosts):
        V.require(len(host['snapshots']) == 7 and len(host['intervals']) == 6
            and all(row['shared_full_currentness'] is policy for row in host['snapshots']),
            'all seven snapshots retain the selected policy')
    intervals = []
    for index, (left, right) in enumerate(zip(hosts[0]['intervals'], hosts[1]['intervals'])):
        a, b = full_currentness(left), full_currentness(right)
        intervals.append(dict(from_phase=H.PHASES[index], to_phase=H.PHASES[index + 1],
            shared=a, ordered=b, component_sum_delta_ns=b['component_sum_ns'] - a['component_sum_ns'],
            shared_interval_host_ns=left['host_elapsed_ns'], ordered_interval_host_ns=right['host_elapsed_ns']))
    return dict(primary_latency_scope='same-generation enclosing forward host wall, fixed shared-first order',
        per_kernel_publish_wait_poll_comparable=False, ordered_wait_spans_overlap=True,
        first_ordered_segment_includes_arena_initialization=True, controls_byte_comparison_performed=False,
        intermediate_host_fence_removed=True, same_generation=True, same_images=True, actual_input_histories=histories[0],
        output_tokens=[[row['output_token'] for row in rows] for rows in events],
        payloads_byte_equal=payload_equal, all_payloads_byte_equal=all(payload_equal),
        payload_equality_scope='same-generation repeatability, not independent numerical accuracy',
        intervals=intervals, configuration_host_ns=dict(shared=hosts[0]['configuration_host_ns'], ordered=hosts[1]['configuration_host_ns']),
        configuration_time_in_snapshots=False,
        forward_host_ns=dict(shared=hosts[0]['forward_host_ns'], ordered=hosts[1]['forward_host_ns']),
        serialization_host_ns=dict(shared=hosts[0]['serialization_host_ns'], ordered=hosts[1]['serialization_host_ns']),
        close_host_ns=dict(shared=hosts[0]['close_host_ns'], ordered=hosts[1]['close_host_ns']),
        numerator='rank_full_ns + group_full_ns + publication_full_ns',
        inclusive_host_scopes=True, components_are_disjoint_latency=False,
        gpu_time=False, numerical_acceptance=False, performance_claim=False, production_authority=False)


def payload_repeatability(pins, shared, ordered):
    equal = []
    for index in range(4):
        name = 'observation-' + str(index) + '.bin'
        left = I.read(pins, shared['retained_native'][name], 1 << 20, True)
        right = I.read(pins, ordered['retained_native'][name], 1 << 20, True)
        V.require(len(left) == len(right) == 606976, 'two complete payload bodies')
        equal.append(left == right)
    return equal


def execute(plan_pin):
    pins = I.D.Pins(); plan = I.doc(pins, plan_pin, 1 << 20); plan_shape(plan)
    contexts = {role: I.context(plan[role]) for role in ('shared', 'ordered')}
    same_inputs(contexts['shared'], contexts['ordered'])
    out = I.E / plan['output_label']
    V.require(not os.path.lexists(out) and out not in {c['out'] for c in contexts.values()}, 'fresh separate paired summary')
    M.resources(True); out.mkdir(mode=0o700)
    started = time.monotonic(); arms, result, failures = {}, None, []
    try:
        for role in ('shared', 'ordered'):
            V.require(time.monotonic() - started + M.CASE_SECONDS <= PAIR_SECONDS, 'bounded paired schedule')
            for context in contexts.values(): I.guard(context)
            code = M.execute(contexts[role])
            name = 'complete.json' if code == 0 else 'failure.json'
            record, _ = pins.read(contexts[role]['out'] / name, retain=True, maximum=8 << 20)
            arms[role] = record
            V.require(code == 0, 'paired arm failed; no retry or replacement')
        values = {role: I.doc(pins, record, 8 << 20) for role, record in arms.items()}
        summaries = {role: I.doc(pins, value['retained_native']['complete.json'], 65536)
                     for role, value in values.items()}
        equal = payload_repeatability(pins, values['shared'], values['ordered'])
        result = comparison(values['shared'], values['ordered'], summaries['shared'], summaries['ordered'], equal)
    except BaseException as error:
        failures.append(type(error).__name__ + ': ' + str(error))
    finally:
        try:
            for context in contexts.values(): I.guard(context)
            for record in [plan_pin, *plan.values()]:
                if type(record) is dict: I.read(pins, record, 8 << 20, False)
            M.resources(); M.inventory(out)
            V.require(time.monotonic() - started <= PAIR_SECONDS, 'paired wall bound')
        except BaseException as error:
            failures.append('postcheck: ' + repr(error))
    passed = not failures and result is not None and set(arms) == {'shared', 'ordered'}
    value = dict(schema=RESULT_SCHEMA, passed=passed, plan=plan_pin, arms=arms, comparison=result,
        failures=failures, fixed_order=['shared', 'ordered'], retries=0,
        each_arm_max_attempts=1, each_arm_max_seconds=M.CASE_SECONDS, pair_max_seconds=PAIR_SECONDS,
        input_pins=dict(pins.records), elapsed_seconds=time.monotonic() - started,
        paired_comparison_performed=passed, independent_numerical_acceptance=False,
        gpu_time=False, performance_claim=False, production_authority=False,
        fixed_order_warm_cache_and_single_pair_limitations=True)
    record = I.save(out / ('complete.json' if passed else 'failure.json'), value)
    I.D.progress(dict(passed=passed, receipt=record))
    return 0 if passed else 1


def main(args):
    V.require(len(args) == 2 and not sys.flags.optimize, 'PAIR_PLAN_PATH PAIR_PLAN_SHA')
    V.require(os.getuid() == os.geteuid() == 9661 and os.sched_getaffinity(0) == {8, 9}
        and os.getpriority(os.PRIO_PROCESS, 0) == 0, 'fixed numerical UID/affinity/controller nice')
    pins = I.D.Pins(); record, _ = pins.read(Path(args[0]), args[1], maximum=1 << 20)
    return execute(record)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
