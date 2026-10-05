"""Read authenticated retained pair data; report host observations, not GPU timing."""
import csv
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
CPU_SHA = 'deb2aeacded08c336d1fb5a1638cd2accc2b65ec65ffa3e90177bd5e61146d74'
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


def analyze(pair, directories):
    require(pair['schema'] == 'ferric-p228-projection-ar4-shared-host-pair-v1' and pair['passed'] is True
        and pair['failures'] == [] and pair['paired_comparison_performed'] is True and pair['retries'] == 0
        and pair['fixed_order'] == ['default', 'shared'] and pair['each_arm_max_attempts'] == 1, 'actual successful single pair')
    require(all(pair[k] is False for k in ('independent_numerical_acceptance', 'gpu_time', 'performance_claim', 'production_authority')), 'diagnostic scope')
    arms, events, payloads, rows, timing = {}, {}, {}, [], {}
    for role, directory in zip(('default', 'shared'), directories):
        require(directory.parent == W and directory.resolve(strict=True) == directory
            and re.fullmatch('prefix-projection-ar4-shared-host-' + role + r'-gpu-v228-v[1-9][0-9]{0,8}', directory.name), 'retained W arm directory')
        require(Path(pair['arms'][role]['path']) == E / directory.name / 'complete.json', 'arm receipt location')
        arm = doc(retained(directory, pair['arms'][role])); arms[role] = arm
        require(arm['schema'] == 'ferric-p228-projection-ar4-shared-host-gpu-v1' and arm['route'] == role
            and arm['passed'] is True and arm['failures'] == [] and arm['native_attempts'] == 1 and arm['retries'] == 0
            and arm['captured_payloads'] == 4 and arm['captured_tensor_rows'] == 152
            and arm['own_output_trajectory_checked'] is True, 'actual successful AR4 arm')
        require(arm['parent_cpu_complete'] == arm['worker_cpu_complete']
            and arm['parent_cpu_complete']['sha256'] == CPU_SHA, 'actual shared CPU883 generation')
        summary = doc(retained(directory, arm['retained_native']['complete.json'], 65536))
        events[role] = [f['response']['event'] for f in summary['files']['frames']]
        require(len(events[role]) == 4 and all(e['status'] == 'completed' for e in events[role])
            and all(events[role][i]['input_token'] == events[role][i - 1]['output_token'] for i in range(1, 4)), 'four own-output events')
        payloads[role] = [retained(directory, arm['retained_native'][f'observation-{i}.bin'], 606976) for i in range(4)]
        require(all(len(raw) == 606976 for raw in payloads[role]), 'four full 152-tensor payloads')
        host = arm['host_checked']; sidecar = doc(retained(directory, arm['host_sidecar'], 65536))
        config = None
        if role == 'shared':
            require(sidecar['schema'] == 'FerricProjectionResidualDecodeSharedHostEnvelopeV1' and sidecar['policy'] == 'shared-full', 'shared envelope')
            config = uint(sidecar['configuration_host_ns']); sidecar = sidecar['observation']
            require(host['configuration_host_ns'] == config and host['configuration_time_in_snapshots'] is False, 'configuration outside snapshots')
        for key in ('snapshots', 'intervals', 'forward_host_ns', 'serialization_host_ns', 'close_host_ns'):
            require(host[key] == sidecar[key], 'authenticated sidecar/counter join')
        require(len(host['snapshots']) == 7 and len(host['intervals']) == 6
            and len(host['forward_host_ns']) == len(host['serialization_host_ns']) == 4
            and all(s['phase'] == PHASES[i] and s['shared_full_currentness'] is (role == 'shared') for i, s in enumerate(host['snapshots'])), 'closed selected-policy intervals')
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
        timing[role] = dict(setup_interval_host_ns=rows[-6]['interval_host_ns'], forward_host_ns=host['forward_host_ns'],
            serialization_host_ns=[uint(v) for v in host['serialization_host_ns']], close_host_ns=uint(host['close_host_ns']),
            configuration_host_ns=config, final_serialization_interval_host_ns=rows[-1]['interval_host_ns'])
    a, b = arms['default'], arms['shared']; comparison = pair['comparison']
    require(all(a[k] == b[k] for k in ('parent_cpu_complete', 'worker_cpu_complete', 'worker', 'prefix_image', 'mlp_image', 'projection_image')), 'same actual generation/worker/images')
    histories = {r: [[e[k] for k in ('position', 'generation', 'input_token')] for e in events[r]] for r in events}
    require(histories['default'] == histories['shared'] == comparison['actual_input_histories'], 'same actual histories')
    equal = [left == right for left, right in zip(payloads['default'], payloads['shared'])]
    require(equal == comparison['payloads_byte_equal'] and all(equal) == comparison['all_payloads_byte_equal'], 'actual byte-repeatability replay')
    require(comparison['output_tokens'] == [[e['output_token'] for e in events[r]] for r in ('default', 'shared')], 'recorded output tokens')
    for row in rows:
        saved = comparison['intervals'][row['interval']]; role = row['route']
        require(all(saved[role][k] == row[k] for k in ('rank_full_ns', 'group_full_ns', 'publication_full_ns'))
            and saved[role]['component_sum_ns'] == row['full_sum_ns']
            and saved[role + '_interval_host_ns'] == row['interval_host_ns'], 'recorded pair numerator and interval')
    require(all(comparison[k][r] == timing[r][k] for r in timing for k in
        ('forward_host_ns', 'serialization_host_ns', 'close_host_ns', 'configuration_host_ns')), 'recorded separate host timings')
    return dict(schema='ferric-p228-projection-ar4-shared-host-pair-analysis-v1', intervals=rows, timing=timing,
        histories=histories['default'], output_tokens={r: [e['output_token'] for e in events[r]] for r in events},
        payloads_byte_equal=equal, payload_sha256={r: [hashlib.sha256(raw).hexdigest() for raw in payloads[r]] for r in payloads},
        single_pair=True, fixed_order=['default', 'shared'], warmed_cache_order_confound=True,
        counter_interval_is_not_exact_forward_scope=True, timing_categories_are_not_additive=True,
        lifecycle_and_full_admission_replayed=False, independent_numerical_acceptance=False,
        gpu_time=False, performance_claim=False, sustained_700_tokens_per_second_claim=False, production_authority=False)


def main():
    require(len(sys.argv) == 6 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'PAIR_COMPLETE SHA DEFAULT_W_DIR SHARED_W_DIR FRESH_OUTPUT')
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
    lines = ['# Single-Pair Host Observations', '', 'Default first; shared second. Warm-cache/order confounding is unresolved.',
        'Counter intervals include surrounding work, so their forward-wall ratios are not a disjoint latency partition.',
        'No GPU-time, independent numerical, sustained throughput, or production claim.', '', '| Route | Interval | Host s | Forward s | Full-currentness s | Full / interval |', '|---|---|---:|---:|---:|---:|']
    for r in result['intervals']:
        forward = '-' if r['forward_host_ns'] is None else f"{r['forward_host_ns'] / 1e9:.6f}"
        ratio = '-' if r['full_over_interval'] is None else f"{r['full_over_interval']:.6f}"
        lines.append(f"| {r['route']} | {r['from_phase']} -> {r['to_phase']} | {r['interval_host_ns'] / 1e9:.6f} | {forward} | {r['full_sum_ns'] / 1e9:.6f} | {ratio} |")
    lines += ['', 'Separate timing (nanoseconds; configuration is outside zero-based snapshots):', '```json', json.dumps(result['timing'], indent=2), '```', '', 'Payload repeatability: ' + json.dumps(result['payloads_byte_equal'])]
    with (out / 'report.md').open('x') as stream: stream.write('\n'.join(lines) + '\n')
    print(json.dumps(dict(output=str(out), payloads_byte_equal=result['payloads_byte_equal'], performance_claim=False)))


if __name__ == '__main__':
    main()
