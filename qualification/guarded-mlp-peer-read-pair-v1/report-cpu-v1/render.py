"""Render authenticated paired-read host diagnostics; no native execution or metric replay."""
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import statistics
import sys
import time
import xml.etree.ElementTree as ET

ORDER = ('control-0', 'paired-0', 'paired-1', 'control-1')
COUNTERS = ['commands', 'command_ns', 'full_currentness_checks', 'full_currentness_ns',
    'operational_currentness_checks', 'operational_currentness_ns', 'kernel_admissions',
    'kernel_admission_ns', 'dispatches', 'dispatch_prepare_ns', 'dispatch_publish_ns',
    'dispatch_wait_ns', 'completion_polls', 'reads', 'read_bytes', 'read_ns', 'writes',
    'write_bytes', 'write_ns']
SHARED = ['group_full_checks', 'group_full_ns', 'publication_full_checks', 'publication_full_ns']
FALSE_FLAGS = ('gpu_time', 'gpu_overlap', 'end_to_end_speedup', 'sustained_tokens_per_second',
               'numerical_acceptance', 'full_model_acceptance', 'production_authority')
SVG = 'http://www.w3.org/2000/svg'
ET.register_namespace('', SVG)


def require(ok, message):
    if not ok:
        raise ValueError(message)


def parse(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON field')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def uint(value):
    require(type(value) is int and 0 <= value < 1 << 64, 'u64 scalar')
    return value


def total(values):
    return uint(sum(uint(value) for value in values))


def pin(raw, path):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def artifact(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}
            and type(value['path']) is str and Path(value['path']).is_absolute()
            and type(value['bytes']) is int and value['bytes'] > 0
            and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']),
            'closed artifact pin')
    return value['bytes'], value['sha256']


def decimal_ns(value, scale=1_000_000, places=3):
    """Integer decimal formatting, round half up; raw CSV integers remain authoritative."""
    value = uint(value)
    factor = 10 ** places
    rounded = (value * factor * 2 + scale) // (scale * 2)
    return str(rounded // factor) + '.' + str(rounded % factor).zfill(places)


def collect(analysis, terminals, hosts):
    require(set(analysis) == {'schema', 'run_order', 'runs', 'chronology', 'independent_runs_per_mode',
                'control_mean_hidden_read_ns', 'paired_mean_hidden_read_ns',
                'observed_hidden_read_reduction_fraction', 'observed_hidden_read_ratio',
                'per_forward_observations_are_not_independent_runs', 'supervisor_manifest_sha256',
                'supervisor_controller', *FALSE_FLAGS}, 'closed complete analysis')
    require(analysis['schema'] == 'ferric-hidden-pair-read-abba-analysis-v1'
            and analysis['run_order'] == list(ORDER)
            and set(analysis['runs']) == set(ORDER)
            and set(terminals) == set(hosts) == set(ORDER)
            and type(analysis['independent_runs_per_mode']) is int
            and analysis['independent_runs_per_mode'] == 2
            and analysis['per_forward_observations_are_not_independent_runs'] is True
            and all(analysis[name] is False for name in FALSE_FLAGS), 'complete host-only analysis')
    require(type(analysis['supervisor_manifest_sha256']) is str
            and re.fullmatch('[0-9a-f]{64}', analysis['supervisor_manifest_sha256']), 'manifest provenance')
    artifact(analysis['supervisor_controller'])
    chronology = analysis['chronology']
    require(type(chronology) is list and len(chronology) == 4, 'four supervised runs')
    previous, sessions, children = 0, set(), set()
    first, forwards, layers, runs = analysis['runs'][ORDER[0]], [], [], []
    for name, event in zip(ORDER, chronology):
        paired = name.startswith('paired-'); case = analysis['runs'][name]
        terminal, host = terminals[name], hosts[name]
        require(set(event) == {'name', 'terminal', 'started_monotonic_ns', 'finished_monotonic_ns'}
                and event['name'] == name, 'ordered chronology')
        begin, end = uint(event['started_monotonic_ns']), uint(event['finished_monotonic_ns'])
        require(previous <= begin < end, 'nonoverlapping actual chronology'); previous = end
        artifact(event['terminal'])
        require(terminal['schema'] == 'ferric-guarded-mlp-peer-read-pair-gpu-v2'
                and terminal['passed'] is True and terminal['errors'] == []
                and terminal['case'] == name and terminal['paired_read_requested'] is paired
                and type(terminal['native_attempts']) is int and terminal['native_attempts'] == 1
                and type(terminal['retries']) is int and terminal['retries'] == 0
                and terminal['gpu_execution_confirmed'] is True
                and terminal['host_observation_verified'] is True
                and terminal['shared_full_currentness_verified'] is True
                and terminal['host_observation'] == host
                and all(terminal[key] is False for key in ('performance_claim', 'production_authority',
                    'full_model_acceptance', 'numerical_acceptance', 'full_long_workload')),
                'successful explicit native case')
        require(terminal['instrumentation_comparison']['all_payloads_equal'] is True
                and terminal['instrumentation_comparison']['all_histories_equal'] is True,
                'ordinary payload and history equality prerequisite')
        require(len(terminal['phases']) == 11, 'complete owned phase roster')
        for phase in terminal['phases']:
            require(type(phase['exit_code']) is int and phase['exit_code'] == 0
                    and phase['reason'] is None and phase['cleanup_signalled'] is False
                    and phase['owned_groups_absent'] is True
                    and phase['owned_processes_reaped'] is True, 'clean native lifecycle')
        require(host['schema'] == 'ferric-guarded-mlp-peer-read-pair-host-checked-v2'
                and host['paired_read'] is paired
                and host['counter_names'] == COUNTERS and host['shared_counter_names'] == SHARED
                and type(host['snapshots']) is int and host['snapshots'] == 587
                and type(host['intervals']) is int and host['intervals'] == 586
                and host['same_run_completions_joined'] is True and host['native_close_confirmed'] is True
                and host['shared_full_currentness'] is True and host['inclusive_nested_host_scopes'] is True
                and all(host[key] is False for key in ('gpu_time', 'gpu_overlap', 'throughput',
                    'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')),
                'explicit checked host report semantics')
        require(all(case[key] == first[key] for key in ('worker', 'parent', 'immutable_request',
                    'platform', 'input_tokens', 'payloads')), 'same inputs, platform, histories and full payloads')
        require(case['worker'] == list(artifact(terminal['admission']['worker']))
                and case['parent'] == list(artifact(terminal['admission']['parent']))
                and case['platform'] == terminal['platform']
                and case['input_tokens'] == terminal['observation']['input_tokens'], 'analysis/native identity join')
        session = case['session']; child = uint(case['child_pid'])
        require(type(session) is list and len(session) == 32 and any(session)
                and all(type(v) is int and 0 <= v <= 255 for v in session)
                and tuple(session) not in sessions and child > 0 and child not in children
                and child == terminal['observation']['child_pid'], 'distinct actual sessions and children')
        sessions.add(tuple(session)); children.add(child)
        require(type(case['payloads']) is list and len(case['payloads']) == 4, 'four full payload identities')
        for position in range(4):
            extent, digest = artifact(terminal['raw']['native/observation-%d.bin' % position])
            require(extent == 606976 and case['payloads'][position] == digest, 'analyzed complete payload pin')
        require(type(case['forwards']) is list and len(case['forwards']) == 4
                and type(host['forward_rows']) is list and len(host['forward_rows']) == 4, 'four correlated forwards')
        run_values = []
        for position, (forward, measured) in enumerate(zip(case['forwards'], host['forward_rows'])):
            require(set(forward) == {'position', 'hidden_read_ns', 'forward_host_ns', 'bracket_host_ns'}
                    and type(forward['position']) is int and forward['position'] == position
                    and type(measured['position']) is int and measured['position'] == position
                    and len(measured['layers']) == 36, 'complete ordered layer roster')
            elapsed = []
            for index, layer in enumerate(measured['layers']):
                require(type(layer['layer']) is int and layer['layer'] == index, 'exact layer index')
                hidden = layer['hidden_read']
                require(set(hidden) == {'host_elapsed_ns', 'ranks', 'shared'}
                        and len(hidden['ranks']) == 2 and len(hidden['shared']) == 4, 'closed hidden interval')
                shared = [uint(v) for v in hidden['shared']]
                require(shared[0] == (2 if paired else 4) and shared[2:] == [0, 0], 'fresh full group fences')
                ranks = []
                for rank in hidden['ranks']:
                    require(type(rank) is list and len(rank) == 19, 'closed rank counters')
                    rank = [uint(v) for v in rank]
                    require(rank[2] == (0 if paired else 2) and rank[:2] == [0, 0]
                            and rank[4:13] == [0] * 9 and rank[13:15] == [1, 8192]
                            and rank[16:19] == [0, 0, 0], 'unchanged reads, no hidden dispatch/write')
                    ranks.append(rank)
                wall = uint(hidden['host_elapsed_ns']); elapsed.append(wall)
                layers.append(dict(run=name, position=position, layer=index, host_hidden_read_ns=wall,
                    group_full_checks=shared[0], rank0_full_checks=ranks[0][2], rank1_full_checks=ranks[1][2],
                    rank0_reads=ranks[0][13], rank1_reads=ranks[1][13],
                    rank0_read_bytes=ranks[0][14], rank1_read_bytes=ranks[1][14]))
            wall = total(elapsed); bracket = uint(forward['bracket_host_ns'])
            require(wall == uint(forward['hidden_read_ns']) <= bracket
                    and uint(forward['forward_host_ns']) <= bracket
                    and forward['forward_host_ns'] == measured['forward_host_ns']
                    and bracket == measured['bracket_host_ns'], 'exact outer hidden interval reconciliation')
            run_values.append(wall)
            forwards.append(dict(run=name, position=position, host_hidden_read_ns=wall))
        run_total = total(run_values)
        require(run_total == uint(case['hidden_read_total_ns']) and run_total > 0, 'exact positive run total')
        runs.append(dict(run=name, mode='paired' if paired else 'control', forwards_ns=run_values,
                         host_hidden_read_total_ns=run_total))
    require(len(layers) == 576 and len(forwards) == 16, 'complete rendered census')
    control = statistics.mean(runs[i]['host_hidden_read_total_ns'] for i in (0, 3))
    paired = statistics.mean(runs[i]['host_hidden_read_total_ns'] for i in (1, 2))
    for key, expected in (('control_mean_hidden_read_ns', control), ('paired_mean_hidden_read_ns', paired),
                         ('observed_hidden_read_reduction_fraction', 1 - paired / control),
                         ('observed_hidden_read_ratio', control / paired)):
        require(type(analysis[key]) in (int, float) and analysis[key] == expected,
                'reported aggregate diagnostic drift')
    return dict(runs=runs, forwards=forwards, layers=layers)


def csv_bytes(rows):
    require(bool(rows), 'nonempty CSV')
    stream = io.StringIO(newline=''); writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator='\n')
    writer.writeheader(); writer.writerows(rows)
    return stream.getvalue().encode()


def plot(data, analysis_sha):
    require(type(analysis_sha) is str and re.fullmatch('[0-9a-f]{64}', analysis_sha), 'plot provenance')
    root = ET.Element('{%s}svg' % SVG, width='1180', height='720', viewBox='0 0 1180 720', role='img')
    ET.SubElement(root, '{%s}title' % SVG).text = 'Outer hidden-read host wall time for four serial AR4 runs'
    ET.SubElement(root, '{%s}desc' % SVG).text = 'Two independent runs per mode. Four colored forward observations within each run are correlated. Not GPU time or end-to-end speedup.'
    def node(tag, **attrs): return ET.SubElement(root, '{%s}%s' % (SVG, tag), {key.replace('_', '-'): str(value) for key, value in attrs.items()})
    def text(x, y, value, size=16, **attrs):
        node('text', x=x, y=y, fill='#20242a', font_family='sans-serif', font_size=size, **attrs).text = value
    node('rect', x=0, y=0, width=1180, height=720, fill='#ffffff')
    text(64, 43, 'Outer Hidden-Read Host Wall Time', 27)
    text(64, 72, 'Serial ABBA; two independent runs per mode; four correlated forwards per run', 16)
    text(64, 104, 'Sum of 36 disjoint layer intervals per forward, seconds', 15)
    maximum = max(row['host_hidden_read_total_ns'] for row in data['runs'])
    require(maximum > 0, 'positive plot scale')
    unit = 10 ** max(0, len(str(maximum)) - 2)
    top = ((maximum + unit - 1) // unit) * unit
    colors = ('#276a8a', '#299c8e', '#bd7833', '#a95c82')
    for tick in range(6):
        y = 530 - tick * 74
        node('line', x1=130, x2=1110, y1=y, y2=y, stroke='#dce1e4', stroke_width=1)
        text(112, y + 5, decimal_ns(top * tick // 5, 1_000_000_000, 3), 14, text_anchor='end')
    for index, row in enumerate(data['runs']):
        x, accumulated = 205 + index * 235, 0
        for position, value in enumerate(row['forwards_ns']):
            bottom = 530 - accumulated * 370 / top
            height = value * 370 / top
            rect = node('rect', x=x, y=format(bottom - height, '.6f'), width=104,
                        height=format(height, '.6f'), fill=colors[position])
            ET.SubElement(rect, '{%s}title' % SVG).text = '%s forward %d: %s ns' % (row['run'], position, value)
            accumulated += value
        text(x + 52, 518 - accumulated * 370 / top,
             decimal_ns(accumulated, 1_000_000_000, 3), 16, text_anchor='middle')
        text(x + 52, 558, row['run'], 16, text_anchor='middle')
    for position, color in enumerate(colors):
        x = 170 + position * 225
        node('rect', x=x, y=585, width=16, height=16, fill=color)
        text(x + 25, 598, 'Forward %d' % position, 15)
    text(64, 640, 'Host timing includes blocking and completion waits. No GPU time, overlap, tok/s, or end-to-end speedup.', 15)
    text(64, 669, 'All compared payloads and token histories must match; no numerical acceptance is inferred.', 15)
    text(64, 699, 'Analysis SHA-256: ' + analysis_sha, 12)
    return ET.tostring(root, encoding='utf-8', xml_declaration=True) + b'\n'


def markdown(data, analysis_sha):
    lines = ['# Paired-Read Host Interval Diagnostic', '',
        'Four independent serial runs, two per mode. Each row contains four correlated forward observations, not four independent samples.', '',
        '| Run | Forward 0 (ms) | Forward 1 (ms) | Forward 2 (ms) | Forward 3 (ms) | Total (ms) |',
        '| --- | ---: | ---: | ---: | ---: | ---: |']
    for row in data['runs']:
        lines.append('| ' + row['run'] + ' | ' + ' | '.join(decimal_ns(v) for v in
            [*row['forwards_ns'], row['host_hidden_read_total_ns']]) + ' |')
    lines += ['', 'Each forward sums 36 disjoint outer hidden-read wall intervals. Inclusive currentness/read timer counters are not added to elapsed time.', '',
        '| Mode | Hidden intervals | Group full checks / interval | Full checks / rank / interval | Reads / rank / interval | Bytes / rank / interval |',
        '| --- | ---: | ---: | ---: | ---: | ---: |']
    for mode in ('control', 'paired'):
        rows = [r for r in data['layers'] if r['run'].startswith(mode)]
        counts = {(r['group_full_checks'], r['rank0_full_checks'], r['rank1_full_checks'],
                   r['rank0_reads'], r['rank1_reads'], r['rank0_read_bytes'], r['rank1_read_bytes']) for r in rows}
        require(len(counts) == 1, 'uniform actual per-layer counts'); group, rank0, rank1, read0, read1, byte0, byte1 = next(iter(counts))
        require(rank0 == rank1 and read0 == read1 and byte0 == byte1, 'equal rank count summaries')
        lines.append('| %s | %d | %d | %d | %d | %d |' % (mode, len(rows), group, rank0, read0, byte0))
    lines += ['', '[Host wall plot](hidden-read-host-wall.svg). Exact integer observations are retained in `forward-intervals.csv` and all 576 per-layer rows in `layer-counts.csv`.', '',
        'These are host timings, including blocking and completion waits, not GPU execution time, overlap, tokens per second, or end-to-end speedup. Two runs per mode are a small diagnostic, not a controlled repeated benchmark. Payload/history equality is a prerequisite, not independent model accuracy or numerical acceptance.', '',
        'The renderer authenticates the analysis, four terminal receipts and four checked host bodies. It reconciles counts/totals but does not rerun the model, rehash payload bodies, revalidate the entire capsule, or infer a framework reference.', '',
        'Analysis SHA-256: `' + analysis_sha + '`.', '']
    return '\n'.join(lines).encode()


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 5,
            'python3 -B render.py ANALYSIS_JSON ANALYSIS_SHA CAPSULE_ROOT FRESH_OUTPUT')
    require(re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'explicit observed analysis SHA')
    started = time.monotonic(); deadline = started + 30
    def expired(_number, _frame): raise RuntimeError('data-only rendering deadline')
    signal.signal(signal.SIGALRM, expired); signal.setitimer(signal.ITIMER_REAL, 30)
    for kind, cap in ((resource.RLIMIT_AS, 128 << 20), (resource.RLIMIT_CPU, 30),
                      (resource.RLIMIT_FSIZE, 8 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (bound, bound))
    def guard(): require(time.monotonic() < deadline, 'whole rendering deadline')
    inputs, aggregate = {}, 0
    def read(path, expected=None, track=True):
        nonlocal aggregate
        guard(); path = Path(path)
        require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical render input')
        stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1 and 0 < before.st_size <= 8 << 20,
                    'bounded ordinary render input')
            body = stream.read((8 << 20) + 1); after = os.fstat(stream.fileno())
        require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(body) == before.st_size, 'render input drift')
        actual = pin(body, path)
        if expected is not None:
            require(artifact(actual) == artifact(expected), 'render input extent/hash join')
        if track:
            require(str(path) not in inputs or inputs[str(path)] == actual, 'conflicting input pins')
            if str(path) not in inputs: aggregate += len(body)
            inputs[str(path)] = actual
            require(len(inputs) <= 10 and aggregate <= 64 << 20, 'closed bounded renderer readset')
        return body
    controller = pin(read(Path(__file__).resolve()), Path(__file__).resolve())
    raw = read(Path(sys.argv[1])); require(hashlib.sha256(raw).hexdigest() == sys.argv[2], 'actual analysis pin')
    analysis = parse(raw); root = Path(sys.argv[3])
    require(root.is_absolute() and root.resolve(strict=True) == root and root.is_dir(), 'canonical original capsule')
    terminals, hosts = {}, {}
    require(type(analysis['chronology']) is list and len(analysis['chronology']) == 4, 'four actual cases')
    for name, event in zip(ORDER, analysis['chronology']):
        require(event['name'] == name, 'actual case order')
        terminal = parse(read(root / name / 'complete.json', event['terminal']))
        terminals[name] = terminal
        hosts[name] = parse(read(root / name / 'host-observation.json', terminal['raw']['host-observation.json']))
    data = collect(analysis, terminals, hosts)
    outputs = {'forward-intervals.csv': csv_bytes(data['forwards']),
        'layer-counts.csv': csv_bytes(data['layers']), 'hidden-read-host-wall.svg': plot(data, sys.argv[2]),
        'summary.md': markdown(data, sys.argv[2])}
    for path, expected in inputs.items(): read(path, expected, False)
    out = Path(sys.argv[4]); require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent
        and not os.path.lexists(out), 'fresh output directory')
    os.umask(0o077); out.mkdir(mode=0o700); written = {}
    for name, body in outputs.items():
        guard(); require(len(body) <= 8 << 20, 'bounded output')
        path = out / name
        with path.open('xb') as stream: stream.write(body); stream.flush(); os.fsync(stream.fileno())
        written[name] = pin(body, path)
    for path, expected in inputs.items(): read(path, expected, False)
    receipt = dict(schema='ferric-peer-read-host-report-render-v1', passed=True, controller=controller,
        analysis_sha256=sys.argv[2], inputs=inputs, outputs=written, input_posthashes_complete=True,
        independent_runs=4, independent_runs_per_mode=2, correlated_forwards_per_run=4,
        forward_rows=16, layer_rows=576, elapsed_seconds=time.monotonic() - started,
        rendering_only=True, whole_capsule_requalified=False, payload_bodies_rehashed=False,
        gpu_execution=False, model_execution=False, gpu_time=False, gpu_overlap=False,
        throughput_claim=False, end_to_end_speedup=False, performance_claim=False,
        numerical_acceptance=False, production_authority=False)
    guard()
    with (out / 'complete.json').open('x') as stream:
        json.dump(receipt, stream, indent=2, sort_keys=True, allow_nan=False); stream.write('\n')
        stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(passed=True, output=str(out), analysis_sha256=sys.argv[2]), sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
