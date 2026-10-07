"""Authenticated causal stage diagnostics; data only, no model/native replay."""
import argparse
import ast
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
import time
import types

HERE = Path(__file__).resolve().parent
REFERENCE_ARCHIVE = dict(bytes=9927249, sha256='aab3837ab2d3d7fcff0d69424246272f3969c1f0a827a2bf853f770bc2d204bf')
REFERENCE_TERMINAL = dict(bytes=10632, sha256='2d4687d2fae7045a572054c5f46018c859e62a30f5b63c08a7f6060adbd7046c')
REFERENCE_SOURCE = dict(bytes=6553, sha256='294ecd2e93731a76c7c73fd02d17dd1e5157e89b56204e73b02c8903baef1ca9')
NATIVE_ARCHIVE = dict(bytes=5614887, sha256='6d346693b1024978d9c445d945ee2e01cace16187e5251932ef9ae52f3d06c0f')
NATIVE_TERMINAL = dict(bytes=278903, sha256='00aa0447a23f57c6d9bb2ed39cd4e5b2b9da692f15bceb33107ac3822f95516f')
COMPARATOR_CPU_ARCHIVE = dict(bytes=28080, sha256='4d0eff0a66f7d43cf2e6b2e1bbb2460f167bb7cea8b78c6633cfe2242e937bd4')
COMPARATOR_CPU_TERMINAL = dict(bytes=11202, sha256='fb9c982ac824eab32f8b1d35f9ee5ab54eb3eaba5feda38e7efb13aa8dc7e981')
MODULE_PINS = {
    'compare.py': dict(bytes=12434, sha256='066a545b3a531ce8c2e0bb66b81f659b15c3e1e58f5fe0416199c22949bb0efe'),
    'diagnostics.py': dict(bytes=5084, sha256='38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf'),
    'observer.py': dict(bytes=10055, sha256='a96312f8d4c25f49b4585bc6f8ccca583f5097403c36608969709799a5b13fe9'),
    'reference_evidence.py': dict(bytes=40208, sha256='145a23a2ad7fc566d9debed368fb1073cc0b444098f3b8e31e922ec007e40b49'),
    'native_evidence.py': dict(bytes=33276, sha256='22b4c2916b9f5a2398aad20e5db41546e73d9a0f37c6106c1f739c114412af54'),
}
CPU_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-causal-stage-comparison-cpu-v228-v1')
CPU_SOURCES = dict(MODULE_PINS)
for name in ('reference_evidence.py', 'native_evidence.py'):
    CPU_SOURCES.pop(name)
CPU_SOURCES.update({
    'run_cpu.py': dict(bytes=11592, sha256='0c4437bcde9cbfefc5d3c474e49fba950cb435f97d1046ede05be9ce86ed3a16'),
    'supervisor.py': dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
    'test_compare.py': dict(bytes=11894, sha256='9183e7849c31c42efeecad232639de243cff9397f0b0b0f76b43cd806d2aae3e'),
})
BODY_CAP, TOTAL_CAP, WHOLE_SECONDS = 8 << 20, 128 << 20, 180
READSET = {}
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def parse(raw):
    def pairs(rows):
        value = {}
        for key, row in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = row
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def same(a, b):
    return encoded(a) == encoded(b)


def tick():
    require(time.monotonic() < DEADLINE, 'whole comparison deadline')


def read(path, expected=None, cap=BODY_CAP):
    tick()
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_size <= cap,
                'bounded single-link ordinary input')
        raw = stream.read(cap + 1); after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input changed during read')
    row = dict(path=str(path), **pin(raw))
    if expected is not None: require(pin(raw) == expected, 'exact observed input pin')
    require(str(path) not in READSET or READSET[str(path)][0] == row, 'readset drift')
    READSET[str(path)] = (row, cap)
    require(len(READSET) <= 32 and sum(v[0]['bytes'] for v in READSET.values()) <= TOTAL_CAP, 'readset bounds')
    return raw


def archive(path, expected, count, maximum):
    require(expected is not None, 'actual archive pin remains pending')
    raw = read(path, expected, maximum)
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as stream:
        for row in stream:
            tick(); name = row.name
            require(row.isfile() and name not in bodies and name not in ('', '.')
                    and Path(name).as_posix() == name and not Path(name).is_absolute()
                    and '..' not in Path(name).parts and not row.pax_headers
                    and 0 <= row.size <= BODY_CAP and len(bodies) < count, 'closed ordinary tar member')
            total += row.size; require(total <= maximum, 'expanded archive cap')
            with stream.extractfile(row) as body:
                bodies[name] = body.read(row.size + 1)
            require(len(bodies[name]) == row.size, 'exact archive body extent')
    require(len(bodies) == count, 'exact successful archive member census')
    return bodies


def load(name, filename, expected):
    require(expected is not None, 'reviewed pure module binding remains pending')
    path = HERE / filename
    raw = read(path, expected)
    module = types.ModuleType(name); module.__file__ = str(path)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def native_admission(path):
    bodies = archive(path, NATIVE_ARCHIVE, 157, 72 << 20)
    manifest = parse(bodies.pop('manifest.json'))
    require(manifest['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-retention-v1'
            and manifest['terminal_name'] == 'complete.json'
            and manifest['terminal_sha256'] == NATIVE_TERMINAL['sha256']
            and set(bodies) == set(manifest['files'])
            and all(pin(raw) == manifest['files'][name] for name, raw in bodies.items())
            and pin(bodies['readiness/complete.json']) == NATIVE_TERMINAL, 'complete native capsule closure')
    verifier = load('native_evidence', 'native_evidence_bound.py', MODULE_PINS['native_evidence.py'])
    require(pin(bodies['retention_tool.py']) == MODULE_PINS['native_evidence.py']
            and manifest['source_root'] == str(verifier.REMOTE), 'actual reviewed V2 native retention body/root')
    verifier.DEADLINE = DEADLINE
    checked = verifier.verify(bodies, 'complete.json', NATIVE_TERMINAL['sha256'])
    require(checked['original_passed'] is True and checked['retained_success_revalidated'] is True
            and checked['causal_sidecar_revalidated'] is True
            and checked['instrumentation_parity_revalidated'] is True
            and checked['readiness_completed_forwards'] == 40
            and same(checked, manifest['observation']), 'native full transcript/capture/same-side admission')
    return bodies, checked


def reference_admission(path):
    bodies = archive(path, REFERENCE_ARCHIVE, 143, 100 << 20)
    manifest = parse(bodies.pop('manifest.json')); helper = bodies.pop('retention-tool.py')
    require(manifest['schema'] == 'ferric-readiness40-causal-layer0-reference-retention-v1'
            and manifest['terminal'] == 'complete.json' and manifest['source_manifest'] == REFERENCE_SOURCE
            and set(manifest['files']) == set(bodies) | {'retention-tool.py'}
            and pin(helper) == manifest['files']['retention-tool.py'] == MODULE_PINS['reference_evidence.py']
            and all(pin(raw) == manifest['files'][name] for name, raw in bodies.items())
            and pin(bodies['complete.json']) == REFERENCE_TERMINAL, 'complete repeated-reference capsule closure')
    verifier = load('reference_evidence', 'reference_evidence.py', MODULE_PINS['reference_evidence.py'])
    require(manifest['root'] == str(verifier.ROOT), 'actual reference namespace')
    checked = verifier.verify(bodies, 'complete.json', REFERENCE_TERMINAL['sha256'])
    require(checked['passed'] is True and checked['repeat_gate_rechecked'] is True
            and checked['causal_repeat_gate_rechecked'] is True
            and checked['same_side_reference_parity_rechecked'] is True
            and checked['historical_reference_files'] == 10
            and same(checked, manifest['verification']), 'independent full reference repeat and same-side admission')
    return bodies, checked, verifier


def comparator_gate(path):
    bodies = archive(path, COMPARATOR_CPU_ARCHIVE, 14, 2 << 20)
    require(pin(bodies['evidence/complete.json']) == COMPARATOR_CPU_TERMINAL, 'actual ten-test terminal')
    result = parse(bodies['evidence/complete.json'])
    require(result['schema'] == 'ferric-guarded-mlp-readiness40-causal-stage-comparison-cpu-v1'
            and result['passed'] is True and result['failure'] is None and result['postcheck_errors'] == []
            and result['source_unchanged'] is True and result['sources_before'] == result['sources_after']
            and result['synthetic_data_tests_only'] is True
            and all(result[key] is False for key in ('actual_capture_comparison_performed', 'receipt_authentication',
                'same_side_parity_verified_here', 'gpu_execution', 'native_parent_execution', 'model_execution',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')),
            'actual synthetic-only qualification')
    require(set(result['sources_before']) == set(CPU_SOURCES), 'six exact test inputs')
    for name, expected in CPU_SOURCES.items():
        row = result['sources_before'][name]
        require(row['path'] == str(CPU_ROOT / name) and {k: row[k] for k in ('bytes', 'sha256')} == expected
                and pin(bodies[name]) == expected, 'original test source body')
    raw_names = {'sources-before.json', 'sources-after.json'}
    raw_names |= {'readiness-tests.' + suffix for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
    require(set(result['raw']) == raw_names and set(bodies) == set(CPU_SOURCES)
            | {'evidence/' + n for n in raw_names | {'complete.json'}}, 'all seven original raw bodies')
    for name, row in result['raw'].items():
        require(row['path'] == str(CPU_ROOT / 'evidence' / name)
                and pin(bodies['evidence/' + name]) == {k: row[k] for k in ('bytes', 'sha256')}, 'test raw body pin')
    require(parse(bodies['evidence/sources-before.json']) == result['sources_before']
            and parse(bodies['evidence/sources-after.json']) == result['sources_after'], 'test source-map bodies')
    require(len(result['phases']) == 1, 'sole owned test leaf')
    phase = result['phases'][0]
    require(phase['label'] == 'readiness-tests' and phase['exit_code'] == 0 and phase['natural_exit'] is True
            and phase['reaped'] is True and phase['process_group_absent'] is True
            and phase['forced_cleanup'] is False and phase['timed_out'] is False
            and phase['exception'] is None and phase['storage_failure'] is None and phase['observed_signals'] == []
            and parse(bodies['evidence/readiness-tests.result.json']) == phase, 'natural clean test retirement')
    for name in ('command', 'stdout', 'stderr'):
        suffix = 'command.json' if name == 'command' else name
        require(phase[name] == result['raw']['readiness-tests.' + suffix], 'phase original stream/command pin')
    started = parse(bodies['evidence/readiness-tests.started.json'])
    require(started == dict(pid=phase['pid'], pgid=phase['pgid'], argv=phase['argv']), 'actual test registration')
    command = parse(bodies['evidence/readiness-tests.command.json'])
    require(command['argv'] == phase['argv'] and command['env'] == result['environment']
            and command['cwd'] == str(CPU_ROOT)
            and all(result['environment'][k] == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
            'actual GPU-hidden test command')
    tree = ast.parse(bodies['test_compare.py'])
    names = sorted('test_compare.' + cls.name + '.' + node.name for cls in tree.body if isinstance(cls, ast.ClassDef)
        for node in cls.body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    text = bodies['evidence/readiness-tests.stderr'].decode()
    expression = r'^(test_[A-Za-z0-9_]+) \((test_compare\.ComparisonTests)\.\1\) \.\.\. ok$'
    found = [prefix + '.' + name for name, prefix in re.findall(expression, text, re.M)]
    tail = '\n'.join(line for line in re.sub(expression, '', text, flags=re.M).splitlines() if line)
    require(len(names) == len(found) == len(set(found)) == 10 and sorted(found) == names
            and bodies['evidence/readiness-tests.stdout'] == b''
            and re.fullmatch(r'-{70}\nRan 10 tests in [0-9]+\.[0-9]+s\nOK', tail)
            and result['tests'] == dict(names=names, passed=10, failed=0, errors=0, skipped=0), 'all ten original named passes')
    return dict(terminal=COMPARATOR_CPU_TERMINAL, archive=COMPARATOR_CPU_ARCHIVE, tests=result['tests'],
                original_bytes_rechecked=True, tests_rerun=False)


def decoded_native(raw, tokens):
    require(raw[:8] == b'FCAP061\0', 'already admitted native envelope')
    header_size = int.from_bytes(raw[8:12], 'little')
    total = int.from_bytes(raw[12:16], 'little')
    require(total == 1598256 and len(raw) == 16 + header_size + total, 'native full payload')
    header = parse(raw[16:16 + header_size]); data = raw[16 + header_size:]
    records, base = [], 0
    require(len(header['captures']) == 6, 'six native snapshots')
    for position, capture in enumerate(header['captures']):
        require(capture['position'] == position and capture['generation'] == position + 1
                and capture['layer'] == 0, 'native decoded identity')
        length = capture['payload_bytes']; payload = data[base:base + length]; offset = 0
        require(pin(payload)['sha256'] == bytes(capture['payload_sha256']).hex(), 'native snapshot identity')
        ranks = [{}, {}]
        for part in capture['parts']:
            rank, role, size = part['rank'], part['role'], part['bytes']
            require(type(rank) is int and rank in (0, 1) and role not in ranks[rank]
                    and part['offset'] == offset, 'exact native decoded rank/role/offset')
            value = payload[offset:offset + size]
            require(len(value) == size and pin(value)['sha256'] == bytes(part['sha256']).hex(), 'original native stage bytes')
            ranks[rank][role] = value; offset += size
        require(offset == length, 'complete native snapshot')
        records.append(dict(position=position, input_token=tokens[position], ranks=ranks)); base += length
    require(base == total, 'all native payload bytes consumed')
    return records


def execute(args):
    require(all(value is not None for value in (NATIVE_ARCHIVE, NATIVE_TERMINAL,
        COMPARATOR_CPU_ARCHIVE, COMPARATOR_CPU_TERMINAL, *MODULE_PINS.values())),
        'actual native and comparator CPU outcome/source pins remain pending')
    gate = comparator_gate(args.comparator_cpu_archive)
    native, native_checked = native_admission(args.native_archive)
    reference, reference_checked, reference_verifier = reference_admission(args.reference_archive)
    native_summary = parse(native['readiness/native/complete.json'])
    reference_summary = parse(reference['output/complete.json'])
    contract = parse(reference['source/inputs.json'])
    tokens = reference_summary['full_prompt_tokens']
    sequence = native_summary['bootstrap']['sequence']
    require(len(tokens) == 2048 and all(type(t) is int and 0 <= t < 151936 for t in tokens)
            and same(sequence['prompt_tokens'], tokens) and sequence['profile'] == 'readiness40_position5'
            and reference_summary['input_tokens'] == tokens[:40], 'same complete authentic prompt and40 teacher-forced inputs')
    for key in ('model_id', 'bundle_id'):
        require(bytes(sequence['scope'][key]).hex() == reference_summary[key] == contract[key],
                'common authenticated model/bundle identity')
    base = parse(native['readiness-request.json'])['base']
    for field, role in (('manifest', 'prompt_manifest'), ('text', 'prompt_text'), ('tokens', 'prompt_tokens')):
        value = base['prompt'][field]
        require(dict(bytes=value['bytes'], sha256=bytes(value['sha256']).hex()) ==
                {key: contract['files'][role][key] for key in ('bytes', 'sha256')},
                'same authentic prompt original pin')
    require({name: {k: row[k] for k in ('bytes', 'sha256')} for name, row in reference_summary['model_sources'].items()}
            == contract['model_files'], 'actual reference weights/config identities')
    require(reference_summary['generated_tokens'] == 0 and native_summary['generated_tokens'] == []
            and native_summary['completed_forwards'] == reference_summary['passes'][0]['cases'][-1]['position'] + 1 == 40
            and reference_summary['policy'] == dict(sdpa='math-only', deterministic_algorithms=True,
                bf16_reduced_precision_matmul_reduction=False, sdpa_low_precision_reduction=False,
                rotary_fp32_preserved=True, autocast=False), 'closed workload and retained reference numerical policy')
    require(pin(reference['source/observer.py']) == MODULE_PINS['observer.py']
            and pin(reference['source/diagnostics.py']) == MODULE_PINS['diagnostics.py'], 'same frozen pure mapping/metric helpers')
    load('diagnostics', 'diagnostics.py', MODULE_PINS['diagnostics.py'])
    load('observer', 'observer.py', MODULE_PINS['observer.py'])
    comparator = load('compare', 'compare.py', MODULE_PINS['compare.py'])
    raw_native = native['readiness/native/child-stderr.bin']
    n = decoded_native(raw_native, tokens)
    decoded = []
    for ordinal, record in enumerate(reference_summary['causal_layer_zero'], 1):
        raw = reference['output/' + record['file']]
        values = reference_verifier.causal_sidecar(raw, ordinal, tokens, record['stages'])
        decoded.append([dict(position=p, input_token=tokens[p], stages=v) for p, v in enumerate(values)])
    require(decoded[0] == decoded[1], 'same decoded reference stages in both fresh passes')
    frames = [parse(line) for line in native['readiness/native/frames.ndjson'].splitlines()]
    require(len(frames) == 40, 'all original native completion records')
    argmax = []
    for position, (frame, record) in enumerate(zip(frames, reference_summary['passes'][0]['cases'])):
        completion = frame['completion']
        require(completion['position'] == record['position'] == position
                and completion['input_token'] == record['input_token'] == tokens[position], 'same per-position original input')
        argmax.append(dict(position=position, input_token=tokens[position],
            native=completion['output_token'], reference=record['predicted_token'],
            equal=completion['output_token'] == record['predicted_token'],
            selected_full_payload=position in (0, 5, 16, 39), causal_layer0_captured=position < 6))
    tick()
    result = comparator.compare(n, decoded[0])
    require(len(result['comparable_rows']) == 204 and len(result['noncomparable_rows']) == 36
            and result['numerical_acceptance'] is False and result['partials_summed_or_rounded'] is False,
            'unchanged pure comparator scope')
    return dict(native=native_checked, reference=reference_checked, comparator_cpu=gate,
        full_original_receipts_authenticated=True, both_same_side_parity_gates_rechecked=True,
        full_prompt_model_bundle_joined=True, decoded_native_sidecar=pin(raw_native),
        native_image_policy='exact authenticated native source/image route retained in capsule',
        framework_policy=reference_summary['policy'], implementation_policy_equivalence_assumed=False,
        argmax_diagnostics=argmax, argmax_matches=sum(r['equal'] for r in argmax),
        diagnostic=result)


def save(path, raw):
    tick(); require(len(raw) <= BODY_CAP, 'bounded output')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return pin(raw)


def main():
    global DEADLINE
    require(__debug__ and sys.flags.isolated == 1 and sys.dont_write_bytecode, 'python3 -I -B run.py')
    parser = argparse.ArgumentParser()
    for name in ('native-archive', 'reference-archive', 'comparator-cpu-archive', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    require(args.output.is_absolute() and args.output.parent.resolve(strict=True) == args.output.parent
            and not os.path.lexists(args.output), 'fresh output namespace')
    started = time.monotonic(); DEADLINE = started + WHOLE_SECONDS
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0); require(priority in (0, 10), 'nice level')
    if priority == 0: os.nice(10)
    for key in ('ROCR_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'): os.environ[key] = ''
    for kind, cap in ((resource.RLIMIT_AS, 1 << 30), (resource.RLIMIT_CORE, 0), (resource.RLIMIT_FSIZE, BODY_CAP)):
        old = resource.getrlimit(kind); value = min([cap] + [n for n in old if n != resource.RLIM_INFINITY]); resource.setrlimit(kind, (value, value))
    def interrupted(number, _frame): raise RuntimeError('comparison signal ' + str(number))
    original = {s: signal.getsignal(s) for s in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)}
    for sig in original: signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS - 10)
    args.output.mkdir(mode=0o700)
    error, checks, posterrors = None, None, []
    try:
        read(Path(__file__).resolve())
        checks = execute(args)
    except BaseException as failure:
        error = repr(failure)
    finally:
        for sig in original:
            if sig != signal.SIGALRM: signal.signal(sig, signal.SIG_IGN)
        signal.setitimer(signal.ITIMER_REAL, max(0.001, DEADLINE - time.monotonic()))
    try:
        for row, cap in list(READSET.values()):
            try: read(Path(row['path']), {k: row[k] for k in ('bytes', 'sha256')}, cap)
            except BaseException as failure: posterrors.append(dict(path=row['path'], error=repr(failure)))
        tick()
        passed = error is None and not posterrors
        receipt = dict(schema='ferric-readiness40-causal-layer0-comparison-data-v1', passed=passed, error=error,
            postcheck_errors=posterrors, checks=checks, inputs=[r for r, _ in READSET.values()],
            elapsed_seconds=time.monotonic() - started, limits=dict(seconds=180, address_space_bytes=1 << 30,
                input_bytes=TOTAL_CAP, output_file_bytes=BODY_CAP, affinity=[8, 9], nice=10),
            gpu_execution=False, model_execution=False, native_rerun=False, numerical_acceptance=False,
            acceptance_threshold=None, full_model_acceptance=False, full_long_workload=False,
            performance_claim=False, production_authority=False)
        save(args.output / ('complete.json' if passed else 'failed.json'), encoded(receipt))
        print(json.dumps(dict(passed=passed, error=error, output=str(args.output)), sort_keys=True))
        return 0 if passed else 1
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for sig, handler in original.items(): signal.signal(sig, handler)


if __name__ == '__main__':
    raise SystemExit(main())
