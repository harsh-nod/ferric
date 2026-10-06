#!/usr/bin/env python3
"""Data-only all36 TF4/AR4 intake. No process launch or numerical tolerance."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import sys

MODES = {'teacher_forced': 'tf4', 'autoregressive': 'ar4'}
BASELINES = {
    'tf4': (19195, 'eb346b18136d0919de310ea2dce8c812878bb3ef16309197bdcec531b496a609'),
    'ar4': (19193, 'a4d54954d15eed20be041a1aec981f6ed1fd097ee971fe9de43283b907b2ce09')}
REFERENCES = {
    'tf4': (8288, 'cac5d79969c2e17a19630a581b5e21c594ea65b452806630ee87bf7855396036',
            128641, '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416'),
    'ar4': (8347, '385dc945b9bcd7e6c93d24cddde9afea24c7ed93c2708f5453aeac9dd9e2a69f',
            129032, '3f45657f510fd5b0afc609e8cf9249c3218e91a4a696667f8e1fcad474fa797f')}
HELPERS = {
    'stage_core': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'baseline_tf4': '031d2b897d4cd341610fb577ad88c421866edce841468103a166dae87b7ff67a',
    'baseline_ar4': 'b7ec1d8066a671acced6628fd82a1af6ecfad4afdb97e0481bbbcd2a57d5a6cf',
    'diagnostics': '645f11391b2255b7b93a8e7f0372114ad9700a0037e718bb48677d2234b926e7'}
FALSE = ('numerical_acceptance', 'performance_claim', 'production_authority', 'full_long_workload')
ROOT = '/home/harmenon/ferric-asrock-42'
DECODE_SHA = 'db9531757aed606a84c986cb4db736bde398d5fb67ffac74561116d9daaaceae'


def require(ok, message):
    if not ok: raise ValueError(message)


def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed fields: ' + names)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def document(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key'); result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def pin(value):
    keys(value, 'path bytes sha256')
    path = value['path']; size = value['bytes']; digest = value['sha256']
    require(type(path) is str and path.startswith('/') and str(Path(path)) == path
        and '..' not in Path(path).parts and '\0' not in path, 'canonical absolute FilePin syntax')
    require(type(size) is int and 0 <= size <= 512 << 20, 'bounded FilePin extent')
    if type(digest) is list:
        require(len(digest) == 32 and all(type(v) is int and 0 <= v <= 255 for v in digest), 'Rust SHA256')
        digest = bytes(digest).hex()
    require(type(digest) is str and re.fullmatch('[0-9a-f]{64}', digest), 'SHA256 syntax')
    return dict(path=path, bytes=size, sha256=digest)


def get(read, record, maximum=8 << 20):
    record = pin(record); require(record['bytes'] <= maximum, 'record bound')
    raw = read(record, maximum)
    require(type(raw) is bytes and len(raw) == record['bytes'] and sha(raw) == record['sha256'], 'exact input bytes')
    return raw


def doc(read, record, maximum=8 << 20):
    return document(get(read, record, maximum))


def helpers():
    require(not sys.flags.optimize, 'helper assertions require unoptimized Python')
    directory = Path(__file__).resolve().parent
    names = [*HELPERS, 'decode_validation']; previous = {n: sys.modules.get(n) for n in names}
    result = {}
    try:
        for name in names:
            path = directory / ('helpers' if name in HELPERS else '') / (name + '.py')
            raw = path.read_bytes()
            require(sha(raw) == (HELPERS[name] if name in HELPERS else DECODE_SHA), 'frozen helper bytes')
            spec = importlib.util.spec_from_file_location(name, path)
            module = importlib.util.module_from_spec(spec); sys.modules[name] = module
            exec(compile(raw, str(path), 'exec'), module.__dict__); result[name] = module
    finally:
        for name, prior in previous.items():
            if prior is None: sys.modules.pop(name, None)
            else: sys.modules[name] = prior
    return result


def clean_owner(value):
    require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
        and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
        and value['owned_processes_reaped'] is True, 'clean natural exit and owned cleanup')


def lineage(owner, started, child_pid):
    parent = started['parent']
    require(parent['pid'] == parent['pgid'] == parent['sid'] and parent['uid'] == 9661
        and parent['pid'] > 1 and parent['ppid'] == started['supervisor_pid'], 'owned model parent identity')
    rows = [r['identity'] for r in owner['lineage'] if r.get('event') == 'owned']
    require(parent in rows and child_pid != parent['pid'] and 1 <= len(rows) <= 2
        and len({(r['pid'], r['starttime']) for r in rows}) == len(rows)
        and {r['pid'] for r in rows} <= {parent['pid'], child_pid}, 'only model parent and known worker')
    require(type(owner['owned_groups']) is list and len(set(owner['owned_groups'])) == len(owner['owned_groups'])
        and set(owner['owned_groups']) == {r['pgid'] for r in rows}, 'actual owned group roster')
    found = [r for r in rows if r['pid'] == child_pid]
    if found:
        row = found[0]
        require(row['ppid'] == parent['pid'] and row['pid'] == row['pgid']
            and row['sid'] == parent['sid'] and row['uid'] == parent['uid']
            and row['starttime'] >= parent['starttime'], 'worker inherited session and own process group')
    return dict(parent=parent, child=found[0] if found else None, child_pid=child_pid,
        outer_pidfd_observed=bool(found), parent_asserted_close_and_reap=True)


def native_files(read, records, validator):
    require(type(records) is dict and set(records) == validator.FILES | {'complete.json'}, 'exact native14 roster')
    require(sum(pin(v)['bytes'] for v in records.values()) <= 8 << 20, 'native8MiB aggregate')
    summary = get(read, records['complete.json'], 65536)
    files = {name: get(read, records[name], 2 << 20) for name in validator.FILES}
    o = document(summary); directory = o['request']['evidence_directory']
    require(all(pin(value)['path'] == directory + '/' + name for name, value in records.items()),
            'native exact path roster')
    return summary, files, o


def parent_stderr(raw, child_pid, mode):
    require(type(child_pid) is int and 0 < child_pid <= 0xffffffff and mode in MODES,
            'actual child PID and input mode')
    debug_mode = 'TeacherForced' if mode == 'teacher_forced' else 'Autoregressive'
    lines = [f'finite engineering owned child pid={child_pid} pgid={child_pid}; no native setup acknowledged',
             f'finite explicit profile=prefix284-mlp548-four-forward-v1 mode={debug_mode}']
    lines += [f'finite prefix decode completed position={p} forwards={p + 1}' for p in range(4)]
    require(type(raw) is bytes and raw == ('\n'.join(lines) + '\n').encode('ascii'),
            'exact six actual parent progress lines, no additional stderr')


def candidate(read, value, h):
    keys(value, 'native_files request parent owner command started stdout stderr')
    v = h['decode_validation']; raw, files, o = native_files(read, value['native_files'], v)
    checked = v.validate(raw, files)
    require(doc(read, value['request'], 16384) == o['request'], 'actual external request bytes')
    get(read, value['parent'], 128 << 20)
    owner = doc(read, value['owner']); clean_owner(owner)
    require(owner['gpu_execution_requested'] is True, 'actual model leaf')
    for name in ('command', 'started', 'stdout', 'stderr'):
        require(pin(owner[name]) == pin(value[name]), 'owner raw record join')
    command = doc(read, value['command']); start = doc(read, value['started'])
    keys(command, 'argv env cwd deadline_seconds affinity nice address_space_bytes file_cap_bytes '
                  'stream_cap_bytes gpu_execution_requested')
    require(command['argv'] == [pin(value['parent'])['path'], '--request', pin(value['request'])['path'],
                               '--allow-unauthenticated-machine-code']
        and command['cwd'] == ROOT and command['deadline_seconds'] == 4000
        and command['address_space_bytes'] == 32 << 30 and command['file_cap_bytes'] == 64 << 20
        and command['stream_cap_bytes'] == 8 << 20 and command['affinity'] == [8, 9]
        and command['nice'] == 10 and command['gpu_execution_requested'] is True, 'existing owned model envelope')
    require(start['command_sha256'] == pin(value['command'])['sha256'], 'started command identity')
    require(get(read, value['stdout']) in (raw, raw + b'\n'), 'exact sole summary stdout')
    parent_stderr(get(read, value['stderr']), o['child_pid'], o['request']['mode'])
    return o, files, checked, lineage(owner, start, o['child_pid'])


def baseline(read, record, mode, h):
    tag = MODES[mode]; expected = BASELINES[tag]
    require((pin(record)['bytes'], pin(record)['sha256']) == expected, 'actual historical native baseline receipt')
    complete = doc(read, record)
    require(complete['schema'] == f'ferric-p225-tiles-{tag}-gpu-supervisor-complete-v1'
        and complete['status'] == f'CLOSED_TILES_{tag.upper()}_OBSERVATION'
        and complete['gpu_execution'] is True and all(complete[n] is False for n in FALSE), 'baseline limited scope')
    v = h['baseline_' + tag]; raw, files, o = native_files(read, complete['native_evidence_pins'], v)
    require(pin(complete['native_summary']) == pin(complete['native_evidence_pins']['complete.json']), 'baseline summary join')
    checked = getattr(v, 'validate_tiles_' + tag)(raw, files)
    require(checked == complete['structural'], 'baseline retained pure validator replay')
    owner = doc(read, complete['result']); clean_owner(owner)
    for name in ('stdout', 'stderr'):
        require(pin(owner[name]) == pin(complete[name]), 'baseline owner stream join')
    require(get(read, complete['stdout']) == raw, 'baseline exact summary stdout')
    get(read, complete['stderr']); doc(read, complete['command'])
    start = doc(read, complete['started']); lineage(owner, start, o['child_pid'])
    return o, files


def records(observed):
    return [{key: frame['response']['event'][key] for key in
             ('generation', 'position', 'input_token', 'output_token')} for frame in observed['files']['frames']]


def reference(read, record, mode, h):
    tag = MODES[mode]; expected = REFERENCES[tag]
    require((pin(record)['bytes'], pin(record)['sha256']) == expected[:2], 'actual independent reference owner receipt')
    complete = doc(read, record)
    require(complete['passed'] is True and complete['owned_group_reaped'] is True
        and complete['reference_process_exit_observed'] is True and complete['candidate_gpu_execution'] is False
        and complete['numerical_acceptance'] is False and complete['production_authority'] is False,
            'reference independent completed scope')
    require((pin(complete['reference'])['bytes'], pin(complete['reference'])['sha256']) == expected[2:],
            'actual independent two-pass report')
    owner = doc(read, complete['execution_result'])
    require(type(owner['exit_code']) is int and owner['exit_code'] == 0 and owner['failure'] is None
        and owner['group_absent'] is True, 'reference process exit')
    for name in ('command', 'started', 'stderr'): get(read, owner[name])
    require(pin(doc(read, owner['stdout'])) == pin(complete['reference']), 'reference result stdout pin')
    report = doc(read, complete['reference'])
    require(report['status'] == 'PASS' and report['mode'] == mode and report['repeat_passes_byte_equal'] is True
        and report['model_id'] == h['decode_validation'].MODEL_ID
        and report['bundle_id'] == h['decode_validation'].BUNDLE_ID
        and report['numerical_acceptance'] is False and report['acceptance_threshold'] is None,
            'unchanged independent reference arithmetic, no new tolerance')
    passes = report['passes']; require(type(passes) is list and len(passes) == 2, 'two reference passes')
    retained, first_records, caches = [], [], []
    diag = h['diagnostics']
    for ordinal, item in enumerate(passes, 1):
        keys(item, 'ordinal fresh_cache cases')
        require(item['ordinal'] == ordinal and item['fresh_cache'] is True and len(item['cases']) == 4,
                'four positions in each fresh reference cache')
        prior = 9112
        for position, case in enumerate(item['cases']):
            keys(case, 'record payload tensors cache_sha256')
            raw = get(read, case['payload'], 606976); rows = diag.validate_case(case['record'], raw, position)
            require(case['record']['input_token'] == (h['decode_validation'].TOKENS[position]
                    if mode == 'teacher_forced' else prior), 'reference own trajectory')
            prior = case['record']['output_token']
            require(case['tensors'] == {name: dict(bytes=len(body), sha256=sha(body)) for name, body in rows.items()},
                    'all38 reference tensor descriptors')
            require(type(case['cache_sha256']) is list and len(case['cache_sha256']) == 36, 'all36 reference KV hashes')
            for pair in case['cache_sha256']:
                keys(pair, 'key value')
                require(all(type(v) is str and re.fullmatch('[0-9a-f]{64}', v) for v in pair.values()), 'KV digest syntax')
            if ordinal == 1:
                retained.append(raw); first_records.append(case['record']); caches.append(case['cache_sha256'])
            else:
                require(raw == retained[position] and case['record'] == first_records[position]
                    and case['cache_sha256'] == caches[position], 'actual two-pass tensor, trajectory and cache equality')
    return first_records, retained


def compare_rows(left_records, left, right_records, right, diag):
    require(len(left_records) == len(left) == len(right_records) == len(right) == 4, 'four full payloads')
    result, same_history = [], True
    for position in range(4):
        l, r = left_records[position], right_records[position]
        a = diag.validate_case(l, left[position], position); b = diag.validate_case(r, right[position], position)
        same_history = same_history and l['input_token'] == r['input_token']
        rows = []
        if same_history:
            for name in a:
                first = next((i // 2 for i in range(0, len(a[name]), 2) if a[name][i:i+2] != b[name][i:i+2]), None)
                rows.append(dict(name=name, byte_equal=a[name] == b[name], first_mismatching_element=first,
                    reference_sha256=sha(a[name]), candidate_sha256=sha(b[name]), **diag.compare_tensor(a[name], b[name])))
        result.append(dict(position=position, same_input_history=same_history,
            reference_input=l['input_token'], candidate_input=r['input_token'], reference_output=l['output_token'],
            candidate_output=r['output_token'], output_equal=l['output_token'] == r['output_token'],
            tensors=rows if same_history else None))
    return result


def compare(plan, read, frozen=None):
    require(not sys.flags.optimize, 'unoptimized validation required')
    keys(plan, 'schema mode candidate baseline reference')
    require(plan['schema'] == 'ferric-p227-prefix-decode-comparison-inputs-v2' and plan['mode'] in MODES, 'closed comparison plan')
    h = helpers() if frozen is None else frozen
    observed, files, structural, ownership = candidate(read, plan['candidate'], h)
    require(observed['request']['mode'] == plan['mode'], 'candidate selected mode')
    previous, baseline_files = baseline(read, plan['baseline'], plan['mode'], h)
    # Scope/PID/program bytes can differ across fresh owners; actual model/input/image identities cannot.
    for name in ('expected_model_id', 'expected_bundle_id'):
        require(observed['request'][name] == previous['request'][name], 'same authentic model/prompt contract')
    for name in ('manifest', 'text', 'tokens'):
        a, b = pin(observed['request']['prompt'][name]), pin(previous['request']['prompt'][name])
        require((a['bytes'], a['sha256']) == (b['bytes'], b['sha256']), 'same authentic prompt bytes')
    for name in ('prefix', 'mlp', 'residual', 'tail'):
        a, b = pin(observed['request']['images'][name]), pin(previous['request']['images'][name])
        require((a['bytes'], a['sha256']) == (b['bytes'], b['sha256']), 'unchanged original setup image bytes')
    a, b = pin(observed['request']['tiles_image']), pin(previous['request']['tiles_image'])
    require((a['bytes'], a['sha256']) == (b['bytes'], b['sha256']), 'same MLP548 image bytes')
    for position in range(4):
        current = document(files[f'request-{position}.json'])['command']
        old = document(baseline_files[f'request-{position}.json'])['command']
        require(current['cache_metadata'] == old['cache_metadata'] and current['rotary_bits'] == old['rotary_bits'],
                'same KV mapping/position and full rotary input bits')
    rows = [files[f'observation-{i}.bin'] for i in range(4)]
    comparison = compare_rows(records(previous), [baseline_files[f'observation-{i}.bin'] for i in range(4)],
                              records(observed), rows, h['diagnostics'])
    ref_records, ref_rows = reference(read, plan['reference'], plan['mode'], h)
    independent = compare_rows(ref_records, ref_rows, records(observed), rows, h['diagnostics'])
    exact = all(row['same_input_history'] and all(t['byte_equal'] for t in row['tensors']) for row in comparison)
    return dict(schema='ferric-p227-prefix-decode-comparison-v2', mode=plan['mode'],
        status='EXACT_NATIVE_BASELINE_PARITY' if exact else 'NATIVE_BASELINE_MISMATCH',
        full152_tensor_rows_bitwise_equal=exact, structural=structural, owned_record_checks=ownership,
        native_baseline=comparison, independent_framework=independent,
        inputs=plan, gpu_launched=False, recorded_close_and_owner_reap_checked=True,
        current_source_binary_image_authority_verified=False, current_platform_idle_audits_verified=False,
        independent_tensor_acceptance=False, independent_tensor_threshold=None,
        full_model_acceptance=False, sustained_2048_256=False, performance_claim=False, production_authority=False)


class Reader:
    def __init__(self): self.records = {}

    def __call__(self, record, maximum):
        record = pin(record); path = Path(record['path'])
        require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical retained file')
        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode) and before.st_size == record['bytes'], 'regular exact extent')
            raw = stream.read(maximum + 1); after = os.fstat(stream.fileno())
        require(all(getattr(before, k) == getattr(after, k) == getattr(path.stat(), k)
            for k in ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')), 'retained file changed while read')
        require(len(raw) == record['bytes'] and sha(raw) == record['sha256'], 'retained file digest')
        require(record['path'] not in self.records or self.records[record['path']] == record, 'contradictory same-path pins')
        self.records[record['path']] = record; return raw

    def recheck(self):
        for record in list(self.records.values()): self(record, max(1, record['bytes']))


def main():
    require(len(sys.argv) == 4, 'compare.py PLAN_PATH PLAN_SHA NEW_RESULT_PATH')
    path, expected, output = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
    reader = Reader(); record = dict(path=str(path), bytes=path.stat().st_size, sha256=expected)
    plan = doc(reader, record, 1 << 20); value = compare(plan, reader)
    value['plan'] = record; value['input_pins'] = dict(reader.records)
    reader.recheck()
    require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent
            and not os.path.lexists(output), 'new canonical result path')
    raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw) <= 4 << 20, 'comparison output bound')
    with output.open('xb') as stream: stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(path=str(output), bytes=len(raw), sha256=sha(raw)), sort_keys=True))


if __name__ == '__main__': main()
