"""Local retained-data assembly only; no SSH, launch, tests, or future hashes."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
R = E.parents[1]
CASE = 'prefix-layer0-native-capture-gpu-v228-v1'
BASELINE = 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1'
PACKAGE = 'p228-layer0-native-capture-gpu-v1'
MANIFEST_SHA = '957115945b1e1a12a6ee6b7fd815a98a33b9d7f1bdc99b77293cb5556b560535'
LEAVES = ('before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
RAW = {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
SEEN = {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def fingerprint(path):
    require(path.resolve(strict=True) == path and stat.S_ISREG(path.lstat().st_mode), 'canonical retained file')
    before = path.stat()
    require(before.st_size <= 16 << 20, 'bounded retained file')
    raw = path.read_bytes(); after = path.stat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) and len(raw) == before.st_size, 'stable retained bytes')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest()), raw


def resolve(original):
    path = Path(original)
    require(path.is_absolute() and str(path) == original and '..' not in path.parts, 'original path')
    if path.is_relative_to(E):
        relative = path.relative_to(E)
        return L / ('proposals' if relative.parts[0].startswith('p228-') else '') / relative
    require(path == R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py', 'explicit non-E helper only')
    return L / 'run_p217_mi350.py'


def body(pin):
    require(set(pin) == {'path', 'bytes', 'sha256'}, 'original FilePin')
    local = resolve(pin['path']); actual, raw = fingerprint(local)
    require((actual['bytes'], actual['sha256']) == (pin['bytes'], pin['sha256']), 'retained original identity')
    require(SEEN.setdefault(str(local), actual) == actual, 'consistent local source')
    return raw


def doc(pin):
    return json.loads(body(pin))


def known(relative, sha):
    original = E / relative
    actual, raw = fingerprint(resolve(str(original)))
    require(actual['sha256'] == sha, 'caller-authenticated actual receipt')
    pin = dict(actual, path=str(original)); body(pin)
    return json.loads(raw), pin


def main(args):
    require(len(args) == 3 and not sys.flags.optimize and sys.dont_write_bytecode
        and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary -B NATIVE_SHA RUNNER_PURE_SHA VERSION')
    require(all(re.fullmatch('[0-9a-f]{64}', x) for x in args[:2])
        and re.fullmatch('[1-9][0-9]{0,8}', args[2]), 'actual digests and fresh version')
    label = 'layer0-current-comparison-inputs-v228-v' + args[2]
    output = L / label
    require(not os.path.lexists(output), 'fresh local transport staging directory')
    native, native_pin = known(CASE + '/complete.json', args[0])
    require(native['schema'] == 'ferric-p228-layer0-native-capture-gpu-v1' and native['passed'] is True
        and native['failures'] == [] and native['native_attempts'] == 1 and native['retries'] == 0,
        'actual naturally completed native case required')
    require(set(native['leaves']) == set(LEAVES), 'seven actual owned leaves')
    records = {}
    def add(pin):
        body(pin)
        require(records.setdefault(pin['path'], pin) == pin, 'consistent selected transport record')
    add(native_pin)
    case_paths = {native_pin['path']}
    for name in LEAVES:
        row = native['leaves'][name]
        require(set(row) == {'result', 'retained_files'} and set(row['retained_files']) == RAW
            and row['result'] == row['retained_files']['result.json'], 'exact owned leaf roster')
        for name_on_disk, pin in row['retained_files'].items():
            require(pin['path'] == str(E / CASE / name / name_on_disk), 'original leaf namespace')
            add(pin); case_paths.add(pin['path'])
    for audit in native['before_audits'] + native['after_audits']:
        add(audit['topology']); case_paths.add(audit['topology']['path'])
    for name, pin in native['retained_native'].items():
        require(pin['path'] == str(E / CASE / 'native' / name), 'original native body namespace')
        add(pin); case_paths.add(pin['path'])
    add(native['observation']); case_paths.add(native['observation']['path'])
    actual_case_paths = {str(E / CASE / path.relative_to(L / CASE))
        for path in (L / CASE).rglob('*') if path.is_file() or path.is_symlink()}
    require(actual_case_paths == case_paths, 'whole retained native case has no unaccounted file')
    for key in ('plan', 'request', 'capture_review', 'parent_runtime_review', 'worker_runtime_review'):
        add(native[key])
    platform_path = str(E / 'prefix-independent-root-inputs-v228-v1/platform-review.json')
    ledger = dict(native['input_pins'])
    for key, pin in native['standalone_input_pins'].items():
        require(key not in ledger or ledger[key] == pin, 'consistent admission ledgers')
        ledger[key] = pin
    platform = ledger[platform_path]; add(platform)
    require(doc(platform)['schema'] == 'ferric-p227-prefix-parity-platform-review-v1', 'original platform record')
    baseline, baseline_pin = known(BASELINE + '/complete.json',
        '00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073')
    require(native['baseline'] == baseline_pin, 'same previously qualified current native baseline')
    for pin in (baseline_pin, baseline['request'], baseline['retained_native']['observation-0.bin']): add(pin)
    pure, pure_pin = known('layer0-current-comparison-pure-v228-v1/complete.json', args[1])
    require(pure['schema'] == 'ferric-p228-layer0-current-comparison-pure-v1'
        and pure['passed'] is True and pure['tests'] == 15
        and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['source_postchecks_passed'] is True, 'actual new runner test receipt')
    require(doc(pure['sources_before']) == doc(pure['sources_after']), 'runner tested sources unchanged')
    body(pure['transcript'])
    manifest, manifest_pin = known(PACKAGE + '/manifest.json', MANIFEST_SHA)
    require(native['supervisor_manifest'] == manifest_pin and len(manifest['files']) == 10,
        'actual frozen capture supervisor package')
    direct = [manifest_pin] + [dict(row, path=str(E / PACKAGE / row['path'])) for row in manifest['files']]
    for original, digest in (
        (str(E / 'p228-independent-gpu-observation-v1/validation.py'),
         'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1'),
        (str(R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py'),
         '6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a')):
        seen, _ = fingerprint(resolve(original))
        require(seen['sha256'] == digest, 'unchanged offline audit helper')
        direct.append(dict(seen, path=original))
    for pin in direct: body(pin)
    for path, pin in SEEN.items(): require(fingerprint(Path(path))[0] == pin, 'all local inputs unchanged')
    mapping, copied = {}, []
    output.mkdir(mode=0o700); (output / 'payloads').mkdir(mode=0o700)
    for index, original in enumerate(sorted(records)):
        pin = records[original]
        target = output / 'payloads' / ('%03d-%s' % (index, Path(original).name))
        raw = body(pin)
        with target.open('xb') as stream: stream.write(raw)
        staged, _ = fingerprint(target)
        retained = dict(staged, path=str(E / label / target.relative_to(output)))
        mapping[original] = retained
        copied.append(dict(original=pin, local=staged, retained=retained))
    plan = dict(schema='ferric-p228-layer0-current-comparison-inputs-v1', native_outer=native_pin,
        platform=platform, transport=mapping, runner_tests=pure_pin,
        output_label='layer0-current-comparison-v228-v' + args[2])
    def save(name, value):
        raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
        require(len(raw) <= 1 << 20, 'small transfer manifest')
        with (output / name).open('xb') as stream: stream.write(raw)
        return dict(fingerprint(output / name)[0], path=str(E / label / name))
    plan_pin = save('plan.json', plan)
    inventory = save('transfer.json', dict(schema='ferric-p228-layer0-current-comparison-transfer-v1',
        plan=plan_pin, native_outer=native_pin, case_files=len(case_paths), mapped=copied,
        install_at_original_path_if_absent_or_identical=[dict(original=pin, local=SEEN[str(resolve(pin['path']))])
            for pin in direct], runner_tests_already_on_asrock=pure_pin,
        gpu_execution=False, process_execution=False, numerical_acceptance=False, performance_claim=False))
    for path, pin in SEEN.items(): require(fingerprint(Path(path))[0] == pin, 'unchanged source after staging')
    for row in copied: require(fingerprint(Path(row['local']['path']))[0] == row['local'], 'staged bytes unchanged')
    print(json.dumps(dict(plan=plan_pin, transfer=inventory, staged_directory=str(output),
        mapped_files=len(copied), required_original_path_files=len(direct))), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])
