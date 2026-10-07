"""Strict raw native-packet replay; no GPU clock conversion or shader-only claim."""
import hashlib
import json
import statistics

SCHEMA = 'Fe2o3NativePacketObservationV1'
MAX_STREAM = 8 * 1024**2
MAX_RECORD = 256 * 1024
FIELDS = {'schema', 'clock', 'device_unique_id', 'queue_epoch', 'first_write',
          'next_write', 'symbols', 'packets'}


def require(ok, why):
    if not ok:
        raise ValueError(why)


def integer(value):
    require(type(value) is int and 0 <= value < 2**64, 'u64, not bool')
    return value


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def validate(row):
    require(type(row) is dict and set(row) == FIELDS, 'closed packet schema')
    require(row['schema'] == SCHEMA and row['clock'] == 'raw_gpu_clock_ticks', 'raw clock identity')
    for name in ('device_unique_id', 'queue_epoch', 'first_write', 'next_write'):
        integer(row[name])
    packets, symbols = row['packets'], row['symbols']
    require(type(packets) is list and 1 <= len(packets) <= 1024, 'bounded packet roster')
    require(row['first_write'] + len(packets) == row['next_write'], 'exact packet frontier')
    require(type(symbols) is dict and 1 <= len(symbols) <= len(packets), 'bounded symbol roster')
    for key, name in symbols.items():
        require(type(key) is str and key.isascii() and key.isdecimal()
                and key == str(integer(int(key))), 'canonical kernel handle')
        require(type(name) is str and 0 < len(name.encode('utf-8')) <= 1024, 'bounded symbol')
    used = set()
    for packet in packets:
        require(type(packet) is list and len(packet) == 3, 'kernel/start/end tuple')
        kernel, start, end = map(integer, packet)
        require(str(kernel) in symbols and 0 < start < end, 'bound kernel and qualified raw ticks')
        used.add(str(kernel))
    require(used == set(symbols), 'exact used symbol roster')
    intervals = sorted((start, end) for _, start, end in packets)
    begin, end = intervals[0]
    union = 0
    for start, finish in intervals[1:]:
        if start > end:
            union += end - begin
            begin, end = start, finish
        else:
            end = max(end, finish)
    union += end - begin
    span = max(end for _, _, end in packets) - min(start for _, start, _ in packets)
    total = sum(end - start for _, start, end in packets)
    return {'window_ticks': span, 'packet_sum_ticks': total, 'packet_union_ticks': union,
            'uncovered_ticks': span - union, 'overlap_ticks': total - union,
            'nonmonotonic_start_pairs': sum(b[1] < a[1] for a, b in zip(packets, packets[1:])),
            'overlapping_adjacent_pairs': sum(b[1] < a[2] for a, b in zip(packets, packets[1:]))}


def split_stream(raw):
    require(type(raw) is bytes and 0 < len(raw) <= MAX_STREAM and raw.endswith(b'\n'),
            'bounded complete packet stream')
    packets, other = [], []
    for line in raw.splitlines(keepends=True):
        value = decode(line)
        require(type(value) is dict and line.endswith(b'\n'), 'complete JSON object line')
        if value.get('schema') == SCHEMA:
            require(len(line) <= MAX_RECORD, 'bounded packet record')
            validate(value)
            packets.append(value)
        else:
            other.append(line)
    require(len(packets) == 131, 'one complete prefill32/decode128 native request')
    return packets, b''.join(other)


def statistics_for(values):
    return {'sum': sum(values), 'mean': statistics.mean(values),
            'median': statistics.median(values), 'max': max(values)}


def summarize(records, waits, device):
    require(len(records) == len(waits) == 131, 'matching native executions')
    symbols = {}
    for row, wait in zip(records, waits):
        validate(row)
        require(row['device_unique_id'] == integer(device) and row['queue_epoch'] == wait['queue_epoch']
                and row['next_write'] == wait['next_write'] and len(row['packets']) == wait['dispatches'],
                'same native device, epoch, count and frontier')
        for handle, name in row['symbols'].items():
            require(handle not in symbols or symbols[handle] == name, 'stable kernel handle binding')
            symbols[handle] = name
    result = {'schema': 'FerricNativePacketAttributionV1', 'performance_qualified': False,
              'latency_sample_admitted': False, 'clock': 'raw_gpu_clock_ticks',
              'scope': 'Firmware packet-processing intervals, not shader-only durations. Uncovered ticks exclude both host-side execution boundaries.',
              'device_unique_id': device, 'records': len(records)}
    for phase, rows in (('prefill', records[:4]), ('decode', records[4:])):
        roster = [p[0] for p in rows[0]['packets']]
        require(all([p[0] for p in row['packets']] == roster for row in rows), 'stable phase packet roster')
        metrics = [validate(row) for row in rows]
        stats = {key: statistics_for([m[key] for m in metrics]) for key in metrics[0]}
        families = {}
        positions = []
        for index, kernel in enumerate(roster):
            name = symbols[str(kernel)]
            durations = [row['packets'][index][2] - row['packets'][index][1] for row in rows]
            position = {'index': index, 'kernel': kernel, 'symbol': name,
                        'duration_ticks': statistics_for(durations)}
            positions.append(position)
            family = families.setdefault(name, {'packets': 0, 'sum_ticks': 0})
            family['packets'] += len(rows)
            family['sum_ticks'] += sum(durations)
        result[phase] = {'executions': len(rows), 'intervals': stats, 'kernels': families,
                         'positions': positions,
                         'uncovered_fraction_of_window': stats['uncovered_ticks']['sum'] / stats['window_ticks']['sum']}
    return result


class Replay:
    def __init__(self, legacy, binding, wait):
        self.legacy = binding.Replay(legacy, wait)
        self.wait = wait
        self.SETUP = legacy.SETUP
        self.MAX_STDERR = MAX_STREAM
        self.exact = legacy.exact
        self.clean_exit = legacy.clean_exit

    def replay(self, raw, spec, setup, counter):
        records, wait_raw = split_stream(raw)
        previous = self.legacy.replay(wait_raw, spec, setup, counter)
        waits, _ = self.wait.split_stream(wait_raw)
        report = summarize(records, waits, 16366993098680759275)
        report.update(stderr_sha256=hashlib.sha256(raw).hexdigest(),
                      wait_stderr_sha256=hashlib.sha256(wait_raw).hexdigest())
        return {'schema': 'FerricNativePacketCaptureReplayV1', 'accepted': True,
                'legacy': previous, 'packets': report, 'performance_qualified': False,
                'latency_sample_admitted': False}
