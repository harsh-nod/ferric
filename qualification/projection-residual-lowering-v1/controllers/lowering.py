"""One CPU-qualified candidate through the retained checked generic compiler."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import selectors
import shutil
import signal
import subprocess
import sys
import time
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
OLD = R / 'evidence/tp2-paired-mfma-v1'
HELPER = R / 'evidence/tp2-graph-host-combined-v1/release-courier-v2/release_runner_v2.py'
HELPER_SHA = 'bbaa0151eccee402dc5b63e878b6e0ceabb65fdbc90c6c84ad1b1592ee1a5d62'
COMMAND_SHA = '84b76a8781ddc545b1271faaba5803df79a5fe365221f1f579894b2dc4e14ab9'
PLAN_SHA = 'ad4c079b6f2091e662b485a5a3ac1b6b9aeb1ebbcf5ab460ea065ff2ed37785f'
RECEIPT_SHA = '509e61855ad6e38a5a91bd5f4ed5fb3a8f935312eab497020c85162a4b061c24'
TOOLS_SHA = '4ff14e82acb6087252fda40df1cdea27035001c483a4dac0e4425ea165288bc7'
OBSERVATION_SHA = 'ad4fcf901dcd1eae8df31bfe479b0fd7ad9fdaff2ce51c9a1cde77e5a0b98294'
OLD_CONTENT = 'cccf099cfde7d537f316a9b75b0872536ca7c4bb632c57a064548b987286f9b8'
CPU_RUN_SHA = '6ad7d210014606a278a30be4aff1665719300d67f9790e24a0cc95fa29f2af29'
SOURCE_SHA = '08973f7fd8851ad0c3ea628006454f91f2af87e9db156778b6e3de2ea5de865f'
CRATE = 'ferric_qwen3_tp_projection_residual_kernels_device_v1'
DIRECTORY = 'qwen3-tp-projection-residual-kernels-v1'
KERNEL = 'ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1'
SYSROOT_SHA = '37a931ebb3cdb986a7586e5af23ee6b6ddf5ab63d5d3bf93a51fafd44f21a90c'
VENDOR_SHA = '8c28a70a7b372ee635acf6053e0724be62e6df65baf1b35f8585aadc3c4ffe38'
MAX_JSON = 16 << 20


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def pin(h, path):
    require(path.is_absolute() and path.is_file() and not path.is_symlink(),
            'ordinary absolute input: ' + str(path))
    return dict(path=str(path), bytes=path.stat().st_size, sha256=h.sha(path))


def record(h, ledger, path, expected=None):
    value = pin(h, path)
    require(expected is None or value['sha256'] == expected, 'input identity: ' + str(path))
    require(value['path'] not in ledger or ledger[value['path']] == value, 'conflicting input pin')
    ledger[value['path']] = value
    return value


def document(h, ledger, path, expected=None):
    value = record(h, ledger, path, expected)
    require(value['bytes'] <= MAX_JSON, 'bounded JSON')
    raw = path.read_bytes()
    require(len(raw) == value['bytes'] and hashlib.sha256(raw).hexdigest() == value['sha256'],
            'unchanged JSON read')
    return json.loads(raw, object_pairs_hook=pairs)


def snapshot(h, root):
    require(root.resolve(strict=True) == root and root.is_dir(), 'canonical source directory')
    value = {}
    for path in sorted(root.rglob('*')):
        require(not path.is_symlink(), 'source tree contains a symlink')
        if path.is_file():
            require(len(value) < 12000, 'bounded source inventory')
            item = pin(h, path)
            value[str(path.relative_to(root))] = {key: item[key] for key in ('bytes', 'sha256')}
    return value


def protected_targets(roots):
    values = {}
    for root in roots:
        require(root.resolve(strict=True) == root and root.is_dir(), 'retained target directory')
        rows = {}
        for directory, dirs, files in os.walk(root, followlinks=False):
            for name in sorted(dirs + files):
                path = Path(directory) / name
                st = path.lstat()
                rows[str(path.relative_to(root))] = [st.st_mode, st.st_ino, st.st_size,
                    st.st_mtime_ns, st.st_ctime_ns, os.readlink(path) if path.is_symlink() else None]
        values[str(root)] = rows
    return values


def cpu_sources(h, ledger, path, expected):
    require(path.parent.parent == E and path.name == 'complete.json'
            and re.fullmatch(r'projection-residual-cpu-v228-v[1-9][0-9]*', path.parent.name),
            'actual candidate CPU namespace')
    value = document(h, ledger, path, expected)
    require(value['schema'] == 'ferric-p228-projection-residual-cpu-result-v1'
            and value['passed'] is True and value['error'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['gpu_execution'] is False and value['compiler_hsaco_reproduced'] is False
            and value['controller']['sha256'] == CPU_RUN_SHA
            and value['source_manifest']['sha256'] == SOURCE_SHA,
            'successful exact CPU source qualification, not an HSACO result')
    for item in value['raw'].values():
        require(Path(item['path']).parent == path.parent
                and record(h, ledger, Path(item['path'])) == item, 'CPU raw identity')
    for item in document(h, ledger, path.parent / 'inputs.json').values():
        require(record(h, ledger, Path(item['path'])) == item, 'CPU input/tool identity')
    phases = {'rustfmt', 'rustfmt-check', 'candidate-metadata', 'control-metadata',
              'candidate-build-tests', 'control-build-tests'}
    phases |= {role + '-' + kind + '-' + step for role in ('candidate', 'control')
               for kind in ('lib', 'contract') for step in ('list', 'ignored', 'tests')}
    require(set(value['phases']) == phases and len(phases) == 18, 'all eighteen actual CPU phases')
    for name in phases:
        result = document(h, ledger, path.parent / (name + '-result.json'))
        require(result == value['phases'][name] and result['exit_code'] == 0
                and result['reason'] is None and result['group_absent'] is True,
                'actual CPU natural/reaped result')
        for stream in ('stdout', 'stderr'):
            require(ledger[str(path.parent / (name + '-' + stream))]['sha256']
                    == result[stream + '_sha256'], 'CPU phase stream join')
    source_manifest = document(h, ledger, Path(value['source_manifest']['path']), SOURCE_SHA)
    expected_names = {'lib': source_manifest['test_census']['arithmetic'],
                      'contract': source_manifest['test_census']['source_contract']}
    total = 0
    for role in ('candidate', 'control'):
        for kind in ('lib', 'contract'):
            stem = role + '-' + kind
            names = sorted(re.findall(r'^([A-Za-z0-9_:]+): test$',
                (path.parent / (stem + '-list-stdout')).read_text(), re.MULTILINE))
            require(names and len(names) == len(set(names)), 'actual named CPU inventory')
            if role == 'candidate':
                require(names == sorted(expected_names[kind]), 'exact qualified candidate tests')
            text = (path.parent / (stem + '-tests-stdout')).read_text()
            actual = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ok$', text, re.MULTILINE)
            require(sorted(actual) == names and len(actual) == len(names)
                    and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
                    == [(str(len(names)), '0', '0')]
                    and value['tests'][role][kind] == dict(passed=len(names), ignored=0, names=names),
                    'all CPU tests actually passed')
            total += len(names)
    require(total == value['test_count'] and sum(len(v) for v in expected_names.values()) == 17,
            'actual aggregate and seventeen candidate tests')
    root = path.parent / 'sources'
    require(value['formatted_source_root'] == str(root), 'exact formatted source root')
    before = document(h, ledger, path.parent / 'sources-before.json')
    require(document(h, ledger, path.parent / 'sources-after.json') == before
            and snapshot(h, root) == before, 'exact CPU-qualified formatted source and lock')
    require(document(h, ledger, path.parent / 'dependencies-before.json')
            == document(h, ledger, path.parent / 'dependencies-after.json'), 'CPU dependency postcheck')
    previous = document(h, ledger, path.parent / 'old-targets-after.json')
    require(previous == document(h, ledger, path.parent / 'old-targets-before.json'),
            'CPU prior target custody')
    roots = [Path(name) for name in previous] + [path.parent / 'target']
    require(all(p.is_relative_to(E) for p in roots), 'task-owned protected targets')
    return value, root, before, roots


def scratch_size(root):
    # The compiler creates and retires private descendants; tolerate only that race.
    total = 0
    def walk_error(error):
        if not isinstance(error, FileNotFoundError):
            raise error
    for directory, _, files in os.walk(root, followlinks=False, onerror=walk_error):
        try:
            total += Path(directory).lstat().st_size
        except FileNotFoundError:
            continue
        for name in files:
            try:
                total += (Path(directory) / name).lstat().st_size
            except FileNotFoundError:
                continue
    return total


def configurations(roots):
    paths = {parent / '.cargo' / name for root in roots for parent in (root, *root.parents)
             for name in ('config', 'config.toml')}
    require(not any(os.path.lexists(path) for path in paths), 'no inherited Cargo configuration')
    return {str(path): None for path in sorted(paths)}


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and len(sys.argv) == 4,
            'usage: python3 -B run.py CPU_COMPLETE CPU_SHA projection-residual-lowering-v228-vN')
    cpu, cpu_sha, label = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    require(re.fullmatch(r'[0-9a-f]{64}', cpu_sha)
            and re.fullmatch(r'projection-residual-lowering-v228-v[1-9][0-9]*', label), 'closed invocation')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'same task-owned two-core build host')
    require(shutil.disk_usage(R).free >= 40 << 30, '40 GiB setup floor')
    raw = HELPER.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'unchanged ownership helper')
    h = types.ModuleType('retained_projection_compile_owner')
    h.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), h.__dict__)
    ledger = {}
    record(h, ledger, HELPER, HELPER_SHA)
    record(h, ledger, Path(__file__).resolve())
    qualified, source, source_map, roots = cpu_sources(h, ledger, cpu, cpu_sha)
    protected = protected_targets(roots)
    command_record = document(h, ledger, OLD / 'device-control-paired-on-v1/command.json', COMMAND_SHA)
    plan = document(h, ledger, OLD / 'device-courier-v1/device-compile-proposal-v1.json', PLAN_SHA)
    old_result = document(h, ledger, OLD / 'device-control-paired-on-v1/receipt.json', RECEIPT_SHA)
    require(old_result['passed'] is True and old_result['exit_code'] == 0
            and old_result['errors'] == [] and old_result['owned_group_empty'] is True
            and old_result['proposal_sha256'] == PLAN_SHA and old_result['gpu_execution'] is False,
            'actual successful historical generic route')
    arm = plan['arms'][0]
    require(arm['name'] == 'control' and command_record == dict(argv=arm['argv'],
            cwd=arm['cwd'], environment=arm['environment']), 'retained actual command/recipe equality')
    old_tools = document(h, ledger, OLD / 'device-control-paired-on-v1/tools.json', TOOLS_SHA)
    old_observation = document(h, ledger, OLD / 'device-control-paired-on-v1/fe2o3-engineering-v1'
        / OLD_CONTENT / 'observation.json', OBSERVATION_SHA)
    require(len(plan['input_manifests']) == 7, 'original seven closure manifests')
    for item in plan['input_manifests']:
        manifest = Path(item['path'])
        require(old_tools[str(manifest)] == item['sha256'], 'manifest in actual old closure')
        record(h, ledger, manifest, item['sha256'])
        base = manifest.parent if manifest.name == 'SHA256SUMS' else R
        for line in manifest.read_text().splitlines():
            digest, name = line.split(maxsplit=1)
            require(re.fullmatch(r'[0-9a-f]{64}', digest), 'closure digest syntax')
            path = Path(name.removeprefix('*'))
            path = path if path.is_absolute() else base / path
            require(old_tools[str(path)] == digest, 'exact original tool/provider member')
            record(h, ledger, path, digest)
    argv = list(command_record['argv'])
    record(h, ledger, Path(argv[0]), plan['cargo_fe2o3_sha256'])
    for index, flag in enumerate(argv):
        if flag.endswith('-sha256'):
            path = Path(argv[argv.index(flag.removesuffix('-sha256')) + 1])
            record(h, ledger, path, argv[index + 1])
    require(argv[-6:] == ['--manifest-path', str(Path(arm['cwd']) / 'Cargo.toml'), '--lib',
                         '--no-default-features', '--features', 'gfx950,mfma,mfma-paired-prefetch'],
            'exact historical selection tail')
    out = E / label
    require(not os.path.lexists(out), 'fresh exclusive lowering output')
    out.mkdir(mode=0o700)
    scratch = out / 'scratch'
    scratch.mkdir(mode=0o700)
    selected = source / DIRECTORY
    configs = configurations((selected, scratch))
    for flag, value in (('--crate', CRATE), ('--output-root', str(out / 'fe2o3-engineering-v1')),
                        ('--manifest-path', str(selected / 'Cargo.toml')), ('--features', 'gfx950')):
        require(argv.count(flag) == 1, 'unique argument substitution')
        argv[argv.index(flag) + 1] = value
    env = dict(command_record['environment'], HOME=str(out), TMPDIR=str(scratch))
    require(set(env) == {'PATH', 'HOME', 'LANG', 'LC_ALL', 'TZ', 'TMPDIR', 'CARGO_BUILD_JOBS', 'RUST_TEST_THREADS'},
            'unchanged cleared outer environment')
    os.umask(0o077)
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    h.write(out / 'command.json', dict(argv=argv, cwd=str(selected), environment=env,
        affinity=[8, 9], nice=10, deadline_seconds=600, cleanup_seconds=5,
        address_space_bytes=12 << 30, scratch_cap_bytes=6 << 30, stream_cap_bytes=64 << 20,
        gpu_execution=False, explicit_inner_cargo_job_limit=False))
    h.write(out / 'inputs.json', ledger)
    h.write(out / 'sources-before.json', source_map)
    h.write(out / 'targets-before.json', protected)
    h.write(out / 'configurations.json', configs)
    child, audit, natural_exit = None, None, None
    natural_group_absent = False
    errors, artifact = [], None
    lengths = dict(stdout=0, stderr=0)
    minimum, peak_scratch = shutil.disk_usage(R).free, 0
    handlers = {number: signal.signal(number, h.interrupted) for number in h.SIGNALS}
    started = time.monotonic()
    def limits():
        for kind, bound in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30),
                            (resource.RLIMIT_FSIZE, 1 << 30)):
            resource.setrlimit(kind, (bound, bound))
    with (out / 'compile.stdout').open('xb') as stdout, (out / 'compile.stderr').open('xb') as stderr:
        try:
            require(shutil.disk_usage(R).free >= 40 << 30, '40 GiB launch floor')
            child = subprocess.Popen(argv, cwd=selected, env=env, stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True, preexec_fn=limits)
            h.write(out / 'owned-process.json', dict(pid=child.pid, pgid=child.pid))
            with selectors.DefaultSelector() as selector:
                for pipe, name, stream in ((child.stdout, 'stdout', stdout), (child.stderr, 'stderr', stderr)):
                    os.set_blocking(pipe.fileno(), False)
                    selector.register(pipe, selectors.EVENT_READ, (name, stream))
                next_size = started
                while selector.get_map() or child.poll() is None:
                    minimum = min(minimum, shutil.disk_usage(R).free)
                    require(minimum >= 38 << 30, '38 GiB live floor')
                    now = time.monotonic()
                    require(now - started < 600, '600 second total compiler deadline')
                    if now >= next_size:
                        peak_scratch = max(peak_scratch, scratch_size(scratch))
                        require(peak_scratch <= 6 << 30, '6 GiB actual private scratch bound')
                        next_size = now + 2
                    for key, _ in selector.select(timeout=0.1):
                        data = os.read(key.fd, 64 << 10)
                        if not data:
                            selector.unregister(key.fileobj)
                            continue
                        name, stream = key.data
                        remaining = (64 << 20) - lengths[name]
                        stream.write(data[:remaining])
                        lengths[name] += min(len(data), remaining)
                        require(len(data) <= remaining, name + ' 64 MiB stream bound')
                natural_exit = child.poll()
                natural_group_absent = not h.group_exists(child.pid)
                require(natural_group_absent, 'compiler left live descendants before cleanup')
        except BaseException as error:
            errors.append(type(error).__name__ + ': ' + str(error))
        finally:
            if child is not None:
                audit = h.reap(child, 5)
                child.stdout.close()
                child.stderr.close()
            for number, handler in handlers.items():
                signal.signal(number, handler)
    h.write(out / 'owned-process-audit.json', audit)
    try:
        require(child is not None and natural_exit == 0 and natural_group_absent and child.returncode == 0
                and audit and audit['empty'], 'natural zero exit and owned group reaped')
        lines = (out / 'compile.stdout').read_text().splitlines()
        require(len(lines) == 1, 'one compiler output-directory record')
        folder = Path(lines[0])
        require(folder.parent == out / 'fe2o3-engineering-v1'
                and re.fullmatch(r'[0-9a-f]{64}', folder.name)
                and folder.resolve(strict=True) == folder
                and sorted(p.name for p in folder.iterdir()) == ['observation.hsaco', 'observation.json'],
                'fresh exact generic output tree')
        observation_pin = pin(h, folder / 'observation.json')
        require(observation_pin['bytes'] <= 1 << 20, 'bounded observation')
        observation = json.loads((folder / 'observation.json').read_bytes(), object_pairs_hook=pairs)
        image = pin(h, folder / 'observation.hsaco')
        require(observation['schema'] == 'EngineeringHsacoObservationV1'
                and observation['namespace'] == 'fe2o3-engineering-v1' and observation['authority'] == 'none'
                and observation['crate_name'] == CRATE and observation['target'] == 'gfx950:xnack-'
                and observation['code_object_version'] == 6 and observation['providers'] == []
                and observation['grants'] == dict(publication=False, load=False, launch=False)
                and observation['options'] == dict(optimization='O2', strip_debug=True, verify_each=True,
                    timeout_seconds=600, maximum_output_bytes=64 << 20)
                and observation['execution']['exact_output_replay'] is True
                and observation['tools'] == old_observation['tools']
                and observation['hsaco']['kernel_names'] == [KERNEL]
                and observation['hsaco']['identity'] == dict(sha256=image['sha256'], byte_len=image['bytes'])
                and 0 < image['bytes'] <= 64 << 20
                and observation['tools']['rustc_lib_tree_sha256'] == SYSROOT_SHA
                and observation['tools']['cargo_vendor']['tree_sha256'] == VENDOR_SHA,
                'checked engineering observation, exact kernel and pinned compiler closures')
        manifest_bytes = (folder / 'observation.json').read_bytes()
        image_bytes = (folder / 'observation.hsaco').read_bytes()
        content = hashlib.sha256(b'FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0'
            + len(manifest_bytes).to_bytes(8, 'little') + manifest_bytes
            + len(image_bytes).to_bytes(8, 'little') + image_bytes).hexdigest()
        require(content == folder.name, 'actual content-addressed output identity')
        artifact = dict(observation=observation_pin, image=image, value=observation)
    except BaseException as error:
        errors.append('output: ' + type(error).__name__ + ': ' + str(error))
    post_errors = []
    for name, function in (
        ('sources', lambda: require(snapshot(h, source) == source_map, 'qualified source drift')),
        ('targets', lambda: require(protected_targets(roots) == protected, 'prior/CPU target mutation')),
        ('configurations', lambda: require(configurations((selected, scratch)) == configs,
                                          'inherited configuration drift')),
        ('inputs', lambda: require(all(pin(h, Path(path)) == value for path, value in ledger.items()),
                                   'source/tool/provider/controller input drift')),
    ):
        try:
            function()
        except BaseException as error:
            post_errors.append(name + ': ' + type(error).__name__ + ': ' + str(error))
    passed = not errors and not post_errors and artifact is not None
    result = dict(schema='ferric-p228-projection-residual-lowering-result-v1', passed=passed,
        errors=errors, postcheck_errors=post_errors, cpu_complete=ledger[str(cpu)],
        source_root=str(source), source_unchanged=not post_errors, controller=ledger[str(Path(__file__).resolve())],
        natural_exit_code=natural_exit, exit_code=None if child is None else child.returncode,
        natural_group_absent_before_cleanup=natural_group_absent,
        owned_group_empty=bool(audit and audit['empty']), elapsed_seconds=time.monotonic() - started,
        minimum_free_bytes=minimum, peak_private_scratch_bytes=peak_scratch,
        stream_bytes=lengths, artifact=artifact, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False, load_authority=False, launch_authority=False,
        retained_handoff_or_llvm=False, automatic_retries=0,
        raw={p.name: pin(h, p) for p in out.iterdir() if p.is_file()})
    h.write(out / ('complete.json' if passed else 'failed.json'), result)
    print(json.dumps(dict(passed=passed, output=str(out), errors=errors, postcheck_errors=post_errors)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
