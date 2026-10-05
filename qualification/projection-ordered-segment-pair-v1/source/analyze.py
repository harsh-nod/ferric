"""Read authenticated retained pair data; report host observations, not GPU timing."""
import csv
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import struct
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
CPU_SHA = '6ac67053d9b7d5d15772b5e4071a09933f013b6efa27394c31eb2b8279af884f'
PACKAGE_SHA = '054b11e3eae9389dad6dc42f216b2ccfc81115436340cab572fda7f93d8d4d69'
PHASES = ('fresh_enabled', 'setup_sealed', 'forward_0', 'forward_1', 'forward_2', 'forward_3', 'before_close')
READS = {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def read(path, expected=None, maximum=8 << 20):
    require(path.resolve(strict=True) == path, 'canonical retained file')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 < before.st_size <= maximum, 'bounded ordinary body')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened identity')
        raw = stream.read(maximum + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'stable opened body')
    # Reading may update atime; compare identity and mutation fields only.
    require(stamp(path.lstat()) == stamp(before) and len(raw) == before.st_size, 'stable retained body')
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(expected is None or (pin['bytes'], pin['sha256']) == (expected['bytes'], expected['sha256']), 'actual supplied pin')
    require(READS.setdefault(str(path), pin) == pin, 'unchanged repeated read')
    return raw, pin


def doc(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key'); result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=lambda _: require(False, 'nonfinite JSON'))


def retained(directory, pin, maximum=8 << 20):
    require(set(pin) == {'path', 'bytes', 'sha256'} and type(pin['bytes']) is int
        and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'closed actual FilePin')
    original = Path(pin['path']); base = E / directory.name
    require(original.is_relative_to(base) and '..' not in original.parts, 'exact arm-local original path')
    return read(directory / original.relative_to(base), pin, maximum)[0]


def uint(value):
    require(type(value) is int and 0 <= value < 1 << 64, 'u64 host counter')
    return value


def validate_trajectory(events, input_tokens):
    require(type(events) is list and len(events) == 4, 'four captured events')
    # AR4 starts at the fixed first prompt token; prompt contains FilePins, not tokens.
    require(uint(events[0]['input_token']) == 9112
        and [uint(e['position']) for e in events] == list(range(4))
        and [uint(e['generation']) for e in events] == list(range(1, 5)),
        'actual AR4 seed/positions/generations')
    require(all(e['status'] == 'completed' for e in events)
        and all(uint(events[i]['input_token']) == uint(events[i - 1]['output_token']) for i in range(1, 4)),
        'four own-output events')
    require(type(input_tokens) is list and len(input_tokens) == 4
        and all(uint(v) < 151936 for v in input_tokens)
        and input_tokens == [e['input_token'] for e in events]
        and all(uint(e['output_token']) < 151936 for e in events),
        'actual captured input/output token join')


def analyze(pair, directories):
    require(pair['schema'] == 'ferric-p228-projection-ordered-segment-pair-v1' and pair['passed'] is True
        and pair['failures'] == [] and pair['paired_comparison_performed'] is True and pair['retries'] == 0
        and pair['fixed_order'] == ['shared', 'ordered'] and pair['each_arm_max_attempts'] == 1, 'actual successful single pair')
    require(all(pair[k] is False for k in ('independent_numerical_acceptance', 'gpu_time', 'performance_claim', 'production_authority')), 'diagnostic scope')
    arms, events, payloads, rows, timing, requests = {}, {}, {}, [], {}, {}
    segments = []
    for role, directory in zip(('shared', 'ordered'), directories):
        require(directory.is_relative_to(W) and directory.resolve(strict=True) == directory
            and re.fullmatch('prefix-projection-ordered-segment-' + role + r'-gpu-v228-v[1-9][0-9]{0,8}', directory.name), 'retained W arm directory')
        require(Path(pair['arms'][role]['path']) == E / directory.name / 'complete.json', 'arm receipt location')
        arm = doc(retained(directory, pair['arms'][role])); arms[role] = arm
        require(arm['schema'] == ('ferric-p228-projection-ar4-shared-host-gpu-v1' if role == 'shared' else
            'ferric-p228-projection-ordered-segment-gpu-v1') and arm['route'] == role
            and arm['passed'] is True and arm['failures'] == [] and arm['native_attempts'] == 1 and arm['retries'] == 0
            and arm['captured_payloads'] == 4 and arm['captured_tensor_rows'] == 152
            and arm['own_output_trajectory_checked'] is True, 'actual successful AR4 arm')
        require(arm['parent_cpu_complete'] == arm['worker_cpu_complete']
            and arm['parent_cpu_complete']['sha256'] == CPU_SHA
            and arm['supervisor_manifest']['sha256'] == PACKAGE_SHA, 'actual same CPU1848 and supervisor generation')
        summary = doc(retained(directory, arm['retained_native']['complete.json'], 65536))
        events[role] = [f['response']['event'] for f in summary['files']['frames']]
        requests[role] = summary['request']
        validate_trajectory(events[role], summary['input_tokens'])
        payloads[role] = [retained(directory, arm['retained_native'][f'observation-{i}.bin'], 606976) for i in range(4)]
        require(all(len(raw) == 606976 for raw in payloads[role]), 'four full 152-tensor payloads')
        host = arm['host_checked']; sidecar = doc(retained(directory, arm['host_sidecar'], 65536))
        expected_envelope = ('FerricProjectionResidualDecodeSharedHostEnvelopeV1' if role == 'shared' else
            'FerricProjectionResidualMlpOrderedHostEnvelopeV1')
        expected_policy = 'shared-full' if role == 'shared' else 'ordered-shared-full'
        require(sidecar['schema'] == expected_envelope and sidecar['policy'] == expected_policy
            and host['policy'] == expected_policy, 'distinct explicit host envelope')
        config = uint(sidecar['configuration_host_ns']); sidecar = sidecar['observation']
        require(host['configuration_host_ns'] == config and host['configuration_time_in_snapshots'] is False,
            'configuration outside snapshots')
        for key in ('snapshots', 'intervals', 'forward_host_ns', 'serialization_host_ns', 'close_host_ns'):
            require(host[key] == sidecar[key], 'authenticated sidecar/counter join')
        require(len(host['snapshots']) == 7 and len(host['intervals']) == 6
            and len(host['forward_host_ns']) == len(host['serialization_host_ns']) == 4
            and all(s['phase'] == PHASES[i] and s['shared_full_currentness'] is True for i, s in enumerate(host['snapshots'])), 'closed selected-policy intervals')
        for i, interval in enumerate(host['intervals']):
            rank = interval['ranks']; shared = interval['shared']; elapsed = uint(interval['host_elapsed_ns'])
            require(len(rank) == 2 and all(len(r) == 19 for r in rank) and len(shared) == 4, 'counter dimensions')
            require(all(uint(v) >= 0 for r in rank for v in r) and all(uint(v) >= 0 for v in shared), 'finite integer counters')
            full = sum(r[3] for r in rank) + shared[1] + shared[3]
            forward = uint(host['forward_host_ns'][i - 1]) if 1 <= i <= 4 else None
            rows.append(dict(route=role, interval=i, from_phase=PHASES[i], to_phase=PHASES[i + 1],
                interval_host_ns=elapsed, forward_host_ns=forward, rank_full_ns=sum(r[3] for r in rank),
                group_full_ns=shared[1], publication_full_ns=shared[3], full_sum_ns=full,
                rank_full_checks=sum(r[2] for r in rank), group_full_checks=shared[0], publication_full_checks=shared[2],
                full_over_interval=None if elapsed == 0 else full / elapsed,
                interval_full_over_forward=None if not forward else full / forward))
        if role == 'ordered':
            require(host['per_kernel_publish_wait_poll_comparable'] is False
                and host['ordered_wait_spans_overlap'] is True
                and host['segment_host_ns_in_control'] is True
                and arm['checked']['separate_residual_mlp_timings_available'] is False,
                'combined scope, no per-kernel timing invention')
            for position in range(4):
                raw = retained(directory, arm['retained_native'][f'control-{position}.bin'], 241096)
                require(len(raw) == 241096, 'distinct ordered Control extent')
                current = []
                for layer in range(36):
                    offset = 16 + layer * 6696 + 6656
                    values = struct.unpack_from('<5Q', raw, offset)
                    row = dict(prefix_ns=list(values[:2]), segment_host_ns=values[2],
                        final_residual_ns=list(values[3:]))
                    current.append(row)
                    segments.append(dict(position=position, layer=layer, **row))
                require(current == arm['checked']['ordered_layer_timings'][position],
                    'actual typed Control timings equal recorded structural validation')
                require(sum(row['segment_host_ns'] for row in current) <= host['forward_host_ns'][position],
                    'sequential combined segments within enclosing forward')
        timing[role] = dict(setup_interval_host_ns=rows[-6]['interval_host_ns'], forward_host_ns=host['forward_host_ns'],
            serialization_host_ns=[uint(v) for v in host['serialization_host_ns']], close_host_ns=uint(host['close_host_ns']),
            configuration_host_ns=config, final_serialization_interval_host_ns=rows[-1]['interval_host_ns'])
    a, b = arms['shared'], arms['ordered']; comparison = pair['comparison']
    require(requests['shared']['schema'] == 'FerricFiniteProjectionResidualDecodeRequestV1'
        and requests['ordered']['schema'] == 'FerricFiniteProjectionResidualMlpOrderedRequestV1'
        and requests['shared']['projection_residual_image'] == requests['ordered']['projection_residual_image'],
        'distinct route schema and identical additive image')
    left, right = requests['shared']['decode'], requests['ordered']['decode']
    require(set(left) == set(right) and left['mode'] == right['mode'] == 'autoregressive'
        and left['session'] != right['session'] and left['evidence_directory'] != right['evidence_directory']
        and all(left[k] == right[k] for k in set(left) - {'session', 'evidence_directory'}),
        'same model/prompt/devices/worker/bounds with lossless u64 identities')
    require(comparison['per_kernel_publish_wait_poll_comparable'] is False
        and comparison['ordered_wait_spans_overlap'] is True
        and comparison['first_ordered_segment_includes_arena_initialization'] is True
        and comparison['controls_byte_comparison_performed'] is False, 'recorded limited timing attribution')
    require(all(a[k] == b[k] for k in ('parent_cpu_complete', 'worker_cpu_complete', 'worker', 'prefix_image', 'mlp_image', 'projection_image')), 'same actual generation/worker/images')
    histories = {r: [[e[k] for k in ('position', 'generation', 'input_token')] for e in events[r]] for r in events}
    require(histories['shared'] == histories['ordered'] == comparison['actual_input_histories'], 'same actual histories')
    equal = [left == right for left, right in zip(payloads['shared'], payloads['ordered'])]
    require(equal == comparison['payloads_byte_equal'] and all(equal) == comparison['all_payloads_byte_equal'], 'actual byte-repeatability replay')
    require(comparison['output_tokens'] == [[e['output_token'] for e in events[r]] for r in ('shared', 'ordered')], 'recorded output tokens')
    for row in rows:
        saved = comparison['intervals'][row['interval']]; role = row['route']
        require(all(saved[role][k] == row[k] for k in ('rank_full_ns', 'group_full_ns', 'publication_full_ns'))
            and saved[role]['component_sum_ns'] == row['full_sum_ns']
            and saved[role + '_interval_host_ns'] == row['interval_host_ns'], 'recorded pair numerator and interval')
    require(all(comparison[k][r] == timing[r][k] for r in timing for k in
        ('forward_host_ns', 'serialization_host_ns', 'close_host_ns', 'configuration_host_ns')), 'recorded separate host timings')
    forward_ratios = [timing['shared']['forward_host_ns'][i] / timing['ordered']['forward_host_ns'][i]
        if timing['ordered']['forward_host_ns'][i] else None for i in range(4)]
    denominator = sum(timing['ordered']['forward_host_ns'])
    return dict(schema='ferric-p228-projection-ordered-segment-pair-analysis-v1', intervals=rows, timing=timing,
        ordered_segments=segments,
        ordered_segment_sum_ns=[sum(r['segment_host_ns'] for r in segments if r['position'] == i) for i in range(4)],
        shared_over_ordered_forward_ratio=forward_ratios,
        shared_over_ordered_forward_sum_ratio=(sum(timing['shared']['forward_host_ns']) / denominator if denominator else None),
        per_kernel_publish_wait_poll_comparable=False, ordered_wait_spans_overlap=True,
        separate_residual_mlp_timings_available=False, first_ordered_segment_includes_arena_initialization=True,
        intermediate_host_fence_removed=True, controls_byte_comparison_performed=False,
        histories=histories['shared'], output_tokens={r: [e['output_token'] for e in events[r]] for r in events},
        payloads_byte_equal=equal, payload_sha256={r: [hashlib.sha256(raw).hexdigest() for raw in payloads[r]] for r in payloads},
        single_pair=True, fixed_order=['shared', 'ordered'], warmed_cache_order_confound=True,
        counter_interval_is_not_exact_forward_scope=True, timing_categories_are_not_additive=True,
        lifecycle_and_full_admission_replayed=False, independent_numerical_acceptance=False,
        gpu_time=False, performance_claim=False, sustained_700_tokens_per_second_claim=False, production_authority=False)


def main():
    require(len(sys.argv) == 6 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'PAIR_COMPLETE SHA SHARED_W_DIR ORDERED_W_DIR FRESH_OUTPUT')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120), (resource.RLIMIT_FSIZE, 16 << 20)):
        old = resource.getrlimit(kind); cap = min([cap] + [n for n in old if n != resource.RLIM_INFINITY]); resource.setrlimit(kind, (cap, cap))
    _, controller = read(Path(__file__).resolve())
    raw, pin = read(Path(sys.argv[1])); require(pin['sha256'] == sys.argv[2], 'caller-authenticated actual pair SHA')
    result = analyze(doc(raw), [Path(p) for p in sys.argv[3:5]])
    result.update(pair=pin, controller=controller, consumed=list(READS.values()))
    for path, expected in list(READS.items()): read(Path(path), expected)
    out = Path(sys.argv[5]); require(not os.path.lexists(out) and out.parent.resolve(strict=True) == out.parent, 'fresh output')
    out.mkdir(mode=0o700)
    with (out / 'analysis.json').open('x') as stream: json.dump(result, stream, indent=2, sort_keys=True, allow_nan=False); stream.write('\n')
    with (out / 'intervals.csv').open('x', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(result['intervals'][0])); writer.writeheader(); writer.writerows(result['intervals'])
    lines = ['# Single-Pair Host Observations', '', 'Shared first; ordered second. Warm-cache/order and first-use arena confounding are unresolved.',
        'Counter intervals include surrounding work, so their forward-wall ratios are not a disjoint latency partition.',
        'No GPU-time, independent numerical, sustained throughput, or production claim.', '',
        'Primary scope: enclosing forward host wall time, not GPU makespan.', '',
        '| Position | Shared s | Ordered s | Shared / ordered | Ordered combined segments s |',
        '|---|---:|---:|---:|---:|']
    for i in range(4):
        ratio = result['shared_over_ordered_forward_ratio'][i]
        display = '-' if ratio is None else f'{ratio:.6f}'
        lines.append(f"| {i} | {result['timing']['shared']['forward_host_ns'][i] / 1e9:.6f} | {result['timing']['ordered']['forward_host_ns'][i] / 1e9:.6f} | {display} | {result['ordered_segment_sum_ns'][i] / 1e9:.6f} |")
    lines += ['', 'Ratio of four-forward sums: ' + str(result['shared_over_ordered_forward_sum_ratio']),
        '', '| Route | Interval | Host s | Forward s | Full-currentness s | Full / interval |',
        '|---|---|---:|---:|---:|---:|']
    for r in result['intervals']:
        forward = '-' if r['forward_host_ns'] is None else f"{r['forward_host_ns'] / 1e9:.6f}"
        ratio = '-' if r['full_over_interval'] is None else f"{r['full_over_interval']:.6f}"
        lines.append(f"| {r['route']} | {r['from_phase']} -> {r['to_phase']} | {r['interval_host_ns'] / 1e9:.6f} | {forward} | {r['full_sum_ns'] / 1e9:.6f} | {ratio} |")
    lines += ['', 'Ordered combined residual-to-MLP segments (host ns, not GPU or separate-kernel timing):',
        '| Position | Layer | Prefix rank 0 | Prefix rank 1 | Combined segment | Final residual rank 0 | Final residual rank 1 |',
        '|---|---|---:|---:|---:|---:|---:|']
    for row in result['ordered_segments']:
        lines.append(f"| {row['position']} | {row['layer']} | {row['prefix_ns'][0]} | {row['prefix_ns'][1]} | {row['segment_host_ns']} | {row['final_residual_ns'][0]} | {row['final_residual_ns'][1]} |")
    lines += ['', 'Combined segment includes preflight, staging, first-use arenas, fences, waits and terminal checks.',
        'Publish/wait/poll scopes differ; their changes are not per-kernel speedups. Rank waits overlap.',
        'The first ordered segment includes cold arena initialization. No GPU makespan or independent accuracy is measured.',
        '', 'Separate timing (nanoseconds; configuration is outside zero-based snapshots):', '```json', json.dumps(result['timing'], indent=2), '```', '', 'Payload repeatability: ' + json.dumps(result['payloads_byte_equal'])]
    with (out / 'report.md').open('x') as stream: stream.write('\n'.join(lines) + '\n')
    print(json.dumps(dict(output=str(out), payloads_byte_equal=result['payloads_byte_equal'], performance_claim=False)))


if __name__ == '__main__':
    main()
