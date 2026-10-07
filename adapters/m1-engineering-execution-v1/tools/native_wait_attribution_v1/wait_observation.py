"""Replay CPU signal-observation bounds; never GPU timings or launch authority."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics

SCHEMA = 'Fe2o3NativeWaitObservationV1'
CLOCK = 'host_monotonic_ns_since_native_execution_entry'
MAX_STREAM = 1024 * 1024
MAX_RECORD = 4096
U64 = 2**64 - 1
NUMBERS = set(('queue_epoch next_write dispatches execution_return_ns signal_reads '
               'pending_reads completed_reads unexpected_reads max_inter_read_ns '
               'signal_read_ns pause_count pause_elapsed_ns max_pause_ns post_read_ns '
               'currentness_nested_ns retirement_signals_ns').split())
FIELDS = NUMBERS | set(('schema clock execution_succeeded observation_valid '
                       'publication_return_ns last_pending_read_ns first_completed_read_ns '
                       'completion_bracket_ns poll_ready_ns').split())


def require(value, message):
    if not value:
        raise ValueError(message)


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def integer(value):
    require(type(value) is int and 0 <= value <= U64, 'bounded unsigned integer')
    return value


def window(value, end):
    require(type(value) is list and len(value) == 2, 'two-element timing window')
    before, after = map(integer, value)
    require(before <= after <= end, 'timing window outside execution')
    return before, after


def validate(record):
    require(type(record) is dict and set(record) == FIELDS, 'closed wait-observation schema')
    require(record['schema'] == SCHEMA and record['clock'] == CLOCK, 'schema and clock')
    require(record['execution_succeeded'] is True and record['observation_valid'] is True,
            'failed or invalid execution cannot support successful attribution')
    for field in NUMBERS:
        integer(record[field])
    end = record['execution_return_ns']
    publication = integer(record['publication_return_ns'])
    ready = integer(record['poll_ready_ns'])
    first = window(record['first_completed_read_ns'], end)
    lower, upper = window(record['completion_bracket_ns'], end)
    require(publication <= first[0] <= first[1] <= ready <= end, 'publication/read/ready order')
    pending = record['pending_reads']
    require(record['completed_reads'] == 1 and record['unexpected_reads'] == 0
            and record['signal_reads'] == pending + 1, 'successful native poll counts')
    require(record['pause_count'] == pending, 'one native pause per pending read')
    known_read_ns = first[1] - first[0]
    if pending:
        last = window(record['last_pending_read_ns'], end)
        require(publication <= last[0] <= last[1] <= first[0], 'last pending read order')
        require(lower == last[0] and upper == first[1], 'raw signal completion bracket')
        require(upper - lower <= record['max_inter_read_ns'] <= end, 'inter-read upper bound')
        known_read_ns += last[1] - last[0]
    else:
        require(record['last_pending_read_ns'] is None and lower == 0 and upper == first[1],
                'first-read completion keeps conservative execution-entry lower bound')
        require(record['max_inter_read_ns'] == 0, 'one read has no inter-read interval')
    require(known_read_ns <= record['signal_read_ns'], 'known acquisition durations')
    require(record['currentness_nested_ns'] <= record['post_read_ns'], 'nested currentness')
    require(ready - first[1] <= record['post_read_ns'], 'final post-read validation interval')
    require(record['retirement_signals_ns'] <= end - ready, 'retirement after ready')
    pauses, total, maximum = pending, record['pause_elapsed_ns'], record['max_pause_ns']
    require(maximum <= total <= maximum * pauses, 'actual pause accounting')
    if not pauses:
        require(total == maximum == 0, 'no pauses means no pause duration')
    disjoint = sum(record[key] for key in ('signal_read_ns', 'pause_elapsed_ns',
                   'post_read_ns', 'retirement_signals_ns'))
    require(disjoint <= end, 'CPU observation buckets exceed native execution interval')
    return {
        'publication_to_signal_lower_ns': max(0, lower - publication),
        'publication_to_signal_upper_ns': max(0, upper - publication),
        'signal_observation_uncertainty_ns': upper - lower,
        'signal_to_execution_return_lower_ns': end - upper,
        'signal_to_execution_return_upper_ns': end - lower,
        'execution_return_ns': end,
        'pause_elapsed_ns': total,
        'post_read_ns': record['post_read_ns'],
        'currentness_nested_ns': record['currentness_nested_ns'],
        'retirement_signals_ns': record['retirement_signals_ns'],
        'signal_reads': record['signal_reads'],
        'max_inter_read_ns': record['max_inter_read_ns'],
    }


def split_stream(raw):
    """Preserve the legacy stream byte-for-byte for its independent existing replay."""
    require(type(raw) is bytes and 0 < len(raw) <= MAX_STREAM and raw.endswith(b'\n'),
            'bounded complete diagnostic stream')
    observations, legacy = [], []
    for line in raw.splitlines(keepends=True):
        require(line.endswith(b'\n'), 'complete line')
        value = decode(line)
        require(type(value) is dict, 'JSON object line')
        if value.get('schema') == SCHEMA:
            require(len(line) <= MAX_RECORD, 'bounded observation record')
            validate(value)
            observations.append(value)
        else:
            legacy.append(line)
    require(len(observations) == 131 and len(legacy) == 3,
            'one prefill32/decode128 diagnostic request with three legacy records')
    return observations, b''.join(legacy)


def describe(values):
    values = sorted(values)
    require(values, 'nonempty statistics')
    return {'sum': sum(values), 'mean': statistics.mean(values),
            'median': statistics.median(values), 'max': values[-1],
            'p95_nearest_rank': values[(95 * len(values) + 99) // 100 - 1]}


def summarize(records, epoch):
    integer(epoch)
    require(type(records) is list and len(records) == 131, 'exact program execution count')
    expected = [(649, 649 * (i + 1)) for i in range(4)]
    expected += [(652, 2599 + 652 * (i + 1)) for i in range(127)]
    rows = []
    for record, (dispatches, frontier) in zip(records, expected):
        row = validate(record)
        require(record['queue_epoch'] == epoch and record['dispatches'] == dispatches
                and record['next_write'] == frontier, 'exact baseline program geometry')
        rows.append(row)
    return {
        'schema': 'FerricNativeWaitAttributionV1',
        'record_geometry_validated': True,
        'performance_qualified': False,
        'latency_sample_admitted': False,
        'gpu_time_measured': False,
        'records': len(rows),
        'queue_epoch': epoch,
        'prefill': {key: describe([row[key] for row in rows[:4]]) for key in rows[0]},
        'decode': {key: describe([row[key] for row in rows[4:]]) for key in rows[0]},
        'scope': 'CPU monotonic observation bounds, not calibrated GPU execution or kernel shares.',
        'overlap': 'Pause and post-read work overlap device progress; never subtract them as recoverable latency. Currentness is nested inside post-read.',
        'excludes': 'Three singleton head dispatches, preparation before native entry, diagnostic serialization/output and remaining caller/worker return work.',
        'required_external_checks': 'Unchanged source and ELF identities, legacy counter/span replay, exact token parity and clean lifecycle must be verified separately.',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stderr', type=Path, required=True)
    parser.add_argument('--epoch', type=int, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    with args.stderr.open('rb') as source:
        raw = source.read(MAX_STREAM + 1)
    records, legacy = split_stream(raw)
    result = summarize(records, args.epoch)
    result['stderr_sha256'] = hashlib.sha256(raw).hexdigest()
    result['legacy_stderr_sha256'] = hashlib.sha256(legacy).hexdigest()
    with args.output.open('x') as target:
        json.dump(result, target, sort_keys=True, indent=2)
        target.write('\n')


if __name__ == '__main__':
    main()
