"""Authenticated conditional position-5 comparison; no native or model execution."""
import argparse
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
NATIVE_ARCHIVE = dict(bytes=2210645, sha256='df5c824f18d26b3c1a6cde4e7eb6827f1472a846cbb5b97a0b693001270d3c3c')
NATIVE_TERMINAL = dict(bytes=169839, sha256='65c85efb646a6980be5a036220e5cfc70acd811b656dc5c821c7f67eae3dd131')
CHECKER_ARCHIVE = dict(bytes=27743, sha256='95cebdbd242abc0d5d8256cd7f7fa11ddc45f5aa18d36e4652baae7b549a8660')
CHECKER_TERMINAL = dict(bytes=10645, sha256='f639b7100ee8ec82f5442f0ae9136ea5f367793803ea78b6ca502328e7c69856')
REFERENCE_SOURCE = dict(bytes=4695, sha256='97dcb5a8dd5e6c986cd8fa792670ab184d77dfeaeb2c742d4a837d07723297ca')
REFERENCE_TERMINAL = dict(bytes=4477, sha256='8dd2b1134cdd5cf6c0888eaac08c580e30890c8a0b47203aa0edc55a02e81759')
REFERENCE_ARCHIVE = dict(bytes=3965567, sha256='cd9103d1e73c1af63960f24e9d501f5a6747ff82a505891f38d08657693cf5bc')
NEW_REFERENCE_TERMINAL = dict(bytes=4508, sha256='36365edf95b99eeac27ec91973862959f8b354d876258e29467b19753bd802ae')
NEW_REFERENCE_ARCHIVE = dict(bytes=3928908, sha256='3747e54a97c0ca99590fd5a30aa0f59866a71c766cd74ac40872d4b2a8ae9a2f')
NEW_REFERENCE_SOURCE = dict(bytes=4576, sha256='47278b4c53f95a2b47c7d594a897b9ff29e055de4d2c9e151d6c592920225d26')
NEW_NATIVE_ARCHIVE = None
NEW_NATIVE_TERMINAL = None
NEW_CHECKER_ARCHIVE = None
DIAGNOSTIC_ARCHIVE = None
NEW_CHECKER_TERMINAL = dict(bytes=10980, sha256='7df4ccb245eae6f517182cc9cd04977ff2ec6180f5c16bb11e9269a5e4ae6468')
DIAGNOSTIC_TERMINAL = dict(bytes=10052, sha256='3219e672554998322798e4ff4e5625bc4ddf8b3fcbb6a33e42627bc7e31fc2e0')
MODULE_PINS = {
    'common.py': dict(bytes=3172, sha256='445c602dd0d237beb2ebef62446d1d2b34283367b68529736a890640ae185256'),
    'compare.py': dict(bytes=8676, sha256='f3e9d141379c7de13726adf83273955a72dcae521c3389778bf12060c217fd7a'),
    'diagnostics.py': dict(bytes=5084, sha256='38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf'),
    'native_evidence.py': dict(bytes=25540, sha256='f81ff9ad0a6dd7a1652701286590779fdaa77209455ce31f5929b6efca67dd73'),
    'new_native_evidence.py': None,
    'reference_evidence.py': dict(bytes=30149, sha256='98d246d10ac9f755437d94dad9b1a0783b0e307bfae3d7a579ac3c6a086334a4'),
    'new_reference_evidence.py': dict(bytes=30237, sha256='d7c9b3b45e1addedb5d8518a5f410eb660628b95a85ee13d2801c80ca76d7fa5'),
    'pure_evidence.py': dict(bytes=15079, sha256='71048139b433d903a37c2344491dfa098ac9e4326e1b8860f462ac80ab9d5145'),
    'reference.py': dict(bytes=11379, sha256='bdaa410f15b485de29c3f1f97c4de86fc18a7b158bad58ee36bfc4df394b8568'),
}
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


def checker(bodies, native):
    raw = bodies['evidence/complete.json']; require(pin(raw) == CHECKER_TERMINAL, 'actual16 checker terminal')
    value = parse(raw)
    require(value['passed'] is True and value['failure'] is None and value['postcheck_errors'] == []
            and value['source_unchanged'] is True and value['sources_before'] == value['sources_after']
            and value['synthetic_data_tests_only'] is True
            and value['tests']['passed'] == 16 and all(value['tests'][k] == 0 for k in ('failed', 'errors', 'skipped')),
            'actual sixteen-test gate, separate from comparison tests')
    require(len(value['phases']) == 1 and set(bodies) == {'evidence/complete.json'}
            | {Path(row['path']).name for row in value['sources_before'].values()}
            | {'evidence/' + name for name in value['raw']}, 'full checker input/raw closure')
    for row in value['sources_before'].values(): require(pin(bodies[Path(row['path']).name]) == {k: row[k] for k in ('bytes', 'sha256')}, 'checker input hash')
    for name, row in value['raw'].items(): require(pin(bodies['evidence/' + name]) == {k: row[k] for k in ('bytes', 'sha256')}, 'checker raw hash')
    require(bodies['validate_readiness.py'] == native['validate_readiness.py']
            and bodies['readiness_announcement.py'] == native['readiness_announcement.py'], 'tested native validation bodies')
    names = re.findall(r'^(test_[A-Za-z0-9_]+) \((test_readiness\.ReadinessTests)(?:\.[A-Za-z0-9_]+)?\) \.\.\. ok$',
                       bodies['evidence/readiness-tests.stderr'].decode(), re.M)
    require(sorted(cls + '.' + name for name, cls in names) == value['tests']['names'], 'actual named16 raw outcomes')
    return dict(terminal=CHECKER_TERMINAL, tests=16, original_bytes_rechecked=True)


def native_admission(path, expected, terminal, module_name, schema):
    bodies = archive(path, expected, 82, 66 << 20)
    manifest = parse(bodies.pop('manifest.json'))
    require(manifest['schema'] == schema and manifest['terminal_name'] == 'complete.json'
            and manifest['terminal_sha256'] == terminal['sha256']
            and set(bodies) == set(manifest['files'])
            and all(pin(body) == manifest['files'][name] for name, body in bodies.items())
            and pin(bodies['readiness/complete.json']) == terminal, 'native capsule closure')
    filename = 'new_native_evidence_bound.py' if module_name == 'new_native_evidence' else module_name + '.py'
    verifier = load(module_name, filename, MODULE_PINS[module_name + '.py'])
    require(pin(bodies['retention_tool.py']) == MODULE_PINS[module_name + '.py'], 'actual native verifier body')
    verifier.DEADLINE = DEADLINE
    checked = verifier.verify(bodies, 'complete.json', terminal['sha256'])
    require(checked['original_passed'] is True and checked['retained_success_revalidated'] is True
            and same(checked, manifest['observation']), 'full retained native admission')
    return bodies, checked


def reference_admission(path, expected, terminal, source, module_name, schema):
    bodies = archive(path, expected, 124, 100 << 20)
    manifest = parse(bodies.pop('manifest.json')); helper = bodies.pop('retention-tool.py')
    require(manifest['schema'] == schema and manifest['terminal'] == 'complete.json'
            and manifest['source_manifest'] == source
            and set(manifest['files']) == set(bodies) | {'retention-tool.py'}
            and pin(helper) == manifest['files']['retention-tool.py'] == MODULE_PINS[module_name + '.py']
            and all(pin(body) == manifest['files'][name] for name, body in bodies.items())
            and pin(bodies['complete.json']) == terminal, 'independent reference capsule closure')
    verifier = load(module_name, module_name + '.py', MODULE_PINS[module_name + '.py'])
    checked = verifier.verify(bodies, 'complete.json', terminal['sha256'])
    require(checked['passed'] is True and checked['repeat_gate_rechecked'] is True
            and same(checked, manifest['verification']), 'full independent repeated-reference admission')
    return bodies, checked


def gate_admission(path, expected, terminal, mode, verifier):
    bodies = archive(path, expected, 15, 4 << 20)
    manifest = parse(bodies.pop('manifest.json'))
    require(pin(bodies['evidence/complete.json']) == terminal
            and pin(bodies['pure-evidence.py']) == MODULE_PINS['pure_evidence.py'], 'actual gate terminal/helper')
    value, pins = verifier.validate(mode, bodies, terminal['sha256'])
    require(same(manifest, dict(schema='ferric-readiness40-position5-pure-cpu-export-v1',
        mode=mode, files=pins, original_files=13, selected_files=14, terminal=terminal,
        actual_test_count=verifier.MODES[mode]['count'], synthetic_only=True,
        new_project_execution=False, native_execution=False, numerical_acceptance=False,
        performance_claim=False)), 'complete pure-gate manifest')
    return bodies, dict(terminal=terminal, tests=value['tests'], original_bytes_rechecked=True)


def execute(args):
    require(all(value is not None for value in [NEW_NATIVE_ARCHIVE, NEW_NATIVE_TERMINAL,
        NEW_CHECKER_ARCHIVE, DIAGNOSTIC_ARCHIVE, *MODULE_PINS.values()]),
        'actual new native, gate archive and bound verifier pins remain pending')
    old_n, old_nc = native_admission(args.native_archive, NATIVE_ARCHIVE, NATIVE_TERMINAL,
        'native_evidence', 'ferric-guarded-mlp-readiness40-retention-v1')
    new_n, new_nc = native_admission(args.new_native_archive, NEW_NATIVE_ARCHIVE, NEW_NATIVE_TERMINAL,
        'new_native_evidence', 'ferric-guarded-mlp-readiness40-position5-retention-v1')
    old_gate = checker(archive(args.checker_archive, CHECKER_ARCHIVE, 13, 1 << 20), old_n)
    old_r, old_rc = reference_admission(args.reference_archive, REFERENCE_ARCHIVE, REFERENCE_TERMINAL,
        REFERENCE_SOURCE, 'reference_evidence', 'ferric-readiness40-reference-retention-v1')
    new_r, new_rc = reference_admission(args.new_reference_archive, NEW_REFERENCE_ARCHIVE, NEW_REFERENCE_TERMINAL,
        NEW_REFERENCE_SOURCE, 'new_reference_evidence', 'ferric-readiness40-position5-reference-retention-v1')
    gates = load('pure_evidence', 'pure_evidence.py', MODULE_PINS['pure_evidence.py'])
    new_g, new_gc = gate_admission(args.new_checker_archive, NEW_CHECKER_ARCHIVE, NEW_CHECKER_TERMINAL, 'checker', gates)
    diagnostic_g, diagnostic_gc = gate_admission(args.diagnostic_archive, DIAGNOSTIC_ARCHIVE, DIAGNOSTIC_TERMINAL, 'diagnostic', gates)
    require(new_g['validate_readiness.py'] == new_n['validate_readiness.py']
            and new_g['readiness_announcement.py'] == new_n['readiness_announcement.py'],
            'new native admission uses the actual seventeen-test checker bodies')
    require({k: parse(new_n['readiness/complete.json'])['checker_cpu'][k]
             for k in ('bytes', 'sha256')} == NEW_CHECKER_TERMINAL,
            'new native admission names the same actual checker terminal')
    require(pin(diagnostic_g['compare.py']) == MODULE_PINS['compare.py']
            and pin(diagnostic_g['diagnostics.py']) == MODULE_PINS['diagnostics.py'],
            'actual seven-test gate authenticates the unchanged diagnostic functions')
    for filename in ('common.py', 'diagnostics.py', 'reference.py'):
        require(pin(old_r['source/' + filename]) == MODULE_PINS[filename], 'original reference helper source')
        if filename != 'reference.py':
            require(new_r['source/' + filename] == old_r['source/' + filename], 'common reference helper bytes')
        load(filename.removesuffix('.py'), filename, MODULE_PINS[filename])
    reference = sys.modules['reference']
    reports, tokens_raw, tokens, contracts, model_pins = [], [], [], [], []
    for bodies in (old_r, new_r):
        contract = parse(bodies['source/inputs.json'])
        role_bodies = {role: bodies['inputs/' + contract['locations'][role]] for role in contract['files']}
        ids, prompt, model = reference.prompt_inputs(contract, role_bodies)
        report = parse(bodies['output/complete.json'])
        require({name: {k: row[k] for k in ('bytes', 'sha256')} for name, row in report['model_sources'].items()} == model,
                'actual model hashes equal authenticated source bundle')
        reports.append(report); tokens_raw.append(role_bodies['prompt_tokens']); tokens.append(ids)
        contracts.append(contract); model_pins.append(model)
    require(tokens_raw[0] == tokens_raw[1] and same(tokens[0], tokens[1])
            and same(model_pins[0], model_pins[1])
            and same({k: v for k, v in contracts[0].items() if k != 'schema'},
                     {k: v for k, v in contracts[1].items() if k != 'schema'}),
            'full prompt/model/bundle and original input contracts unchanged')
    for bodies, profile in ((old_n, 'readiness40'), (new_n, 'readiness40_position5')):
        sequence = parse(bodies['readiness/native/complete.json'])['bootstrap']['sequence']
        require(sequence['profile'] == profile and same(sequence['prompt_tokens'], tokens[0]),
                'both native bootstraps bind the full authentic 2048-token prompt')
        for field in ('model_id', 'bundle_id'):
            require(bytes(sequence['scope'][field]).hex() == contracts[0][field], 'native model/bundle identity')
    comparator = load('compare', 'compare.py', MODULE_PINS['compare.py'])
    native_payloads = []
    reference_payloads = []
    for bodies, selected in ((old_n, comparator.OLD), (new_n, comparator.NEW)):
        native_payloads.append({p: bodies['readiness/native/capture-%d.bin' % p][242824:] for p in selected})
    for bodies, selected in ((old_r, comparator.OLD), (new_r, comparator.NEW)):
        reference_payloads.append([{p: bodies['output/pass%d-pos%d.bf16' % (ordinal, p)] for p in selected}
                                   for ordinal in (1, 2)])
    tick()
    diagnostic = comparator.compare(tokens_raw[0], old_n['readiness/native/frames.ndjson'],
        new_n['readiness/native/frames.ndjson'], reports[0]['passes'], reports[1]['passes'],
        *native_payloads, *reference_payloads)
    return dict(original_native=old_nc, diagnostic_native=new_nc, original_checker=old_gate,
        diagnostic_checker=new_gc, diagnostic_function_gate=diagnostic_gc,
        original_reference=old_rc, diagnostic_reference=new_rc,
        full_original_receipts_authenticated=True, both_native_structural_admissions_rechecked=True,
        full_prompt_and_model_bundle_joined=True, diagnostic=diagnostic)


def save(path, raw):
    tick(); require(len(raw) <= BODY_CAP, 'bounded output')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return pin(raw)


def main():
    global DEADLINE
    require(__debug__ and sys.flags.isolated == 1 and sys.dont_write_bytecode, 'python3 -I -B run.py')
    parser = argparse.ArgumentParser()
    for name in ('native-archive', 'checker-archive', 'reference-archive', 'new-native-archive',
                 'new-checker-archive', 'new-reference-archive', 'diagnostic-archive', 'output'):
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
        receipt = dict(schema='ferric-readiness40-position5-comparison-data-v1', passed=passed, error=error,
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
