"""Retain original bounded Position-5 framework evidence; never run a model."""
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
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-readiness40-position5-reference-v228-v1'
NAME = 'ferric-readiness40-position5-reference-v228-v1'
IMAGE = 'sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba'
MODEL = '/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target'
SOURCE = dict(bytes=4576, sha256='47278b4c53f95a2b47c7d594a897b9ff29e055de4d2c9e151d6c592920225d26')
SELECTED = (0, 5, 16, 39)
NORMAL = ('image', 'name-before', 'cpu-tests', 'before-0', 'before-1', 'before-2',
          'create', 'created', 'execute', 'exited', 'cleanup-inspect', 'final-inspect',
          'remove', 'name-after', 'after-0', 'after-1', 'after-2')
LABELS = set(NORMAL) | {'recover', 'stop'}
LEAF_FILES = {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
OUTPUTS = {'pass%d-pos%d.bf16' % (n, p) for n in (1, 2) for p in SELECTED}
OUTPUTS |= {'pass1.json', 'pass2.json'} | {'implementation-' + n + '.py' for n in
    ('modeling_qwen3', 'activation', 'sdpa', 'torch_functional')}
MAX_FILE, MAX_TOTAL, MAX_MEMBERS = 8 << 20, 100 << 20, 140


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row, metadata=False):
    require(type(row) is dict and type(row.get('bytes')) is int
            and 0 <= row['bytes'] <= ((1 << 40) if metadata else MAX_TOTAL)
            and type(row.get('sha256')) is str
            and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'ordinary byte pin')
    return {key: row[key] for key in ('bytes', 'sha256')}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(rows):
        value = {}
        for key, row in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = row
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def relative(name):
    return type(name) is str and name not in ('', '.') and not Path(name).is_absolute() \
        and Path(name).as_posix() == name and '..' not in Path(name).parts


def read(path, limit=MAX_FILE):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical ordinary input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= limit,
                'bounded single-link regular input')
        raw = stream.read(limit + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input changed while reading')
    return raw


def files(root):
    require(root.resolve(strict=True) == root, 'canonical tree root')
    found = []
    for directory, dirs, names in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'directory alias')
        for name in names:
            found.append(str((Path(directory) / name).relative_to(root)))
            require(len(found) <= MAX_MEMBERS, 'tree file bound')
    return sorted(found)


def pure_reference(bodies):
    # Only these exact authenticated stdlib-only modules are evaluated.
    for name in ('common', 'diagnostics', 'reference'):
        require(name not in sys.modules, 'clean pure helper namespace')
        module = types.ModuleType(name)
        module.__file__ = '/retained/source/' + name + '.py'
        sys.modules[name] = module
        exec(compile(bodies['source/' + name + '.py'], module.__file__, 'exec'), module.__dict__)
    return sys.modules['reference']


def source_literals(raw):
    tree = ast.parse(raw)
    result = {}
    for node in tree.body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id in ('CPU_ENV', 'TEST_NAMES'):
                    if target.id == 'TEST_NAMES':
                        result[target.id] = ast.literal_eval(node.value)
                    else:
                        require(isinstance(node.value, ast.Call) and node.value.func.id == 'dict'
                                and not node.value.args, 'literal CPU environment')
                        result[target.id] = {k.arg: ast.literal_eval(k.value) for k in node.value.keywords}
        elif isinstance(node, ast.FunctionDef) and node.name == 'container_environment':
            call = node.body[0].value
            require(isinstance(call, ast.Call) and call.func.id == 'dict' and not call.args,
                    'literal container environment')
            result['container_env'] = {k.arg: ast.literal_eval(k.value) for k in call.keywords}
    require(set(result) == {'CPU_ENV', 'TEST_NAMES', 'container_env'}, 'closed source literals')
    return result


def commands(plan, cid, literal):
    create = ['/usr/bin/docker', 'create', '--pull=never', '--name', NAME,
        '--label', 'ferric.readiness.reference.owner=' + str(ROOT), '--network=none', '--read-only',
        '--user', '9661:9661', '--group-add', '993', '--device', '/dev/kfd',
        '--device', '/dev/dri/renderD128', '--cap-drop=ALL', '--security-opt', 'no-new-privileges',
        '--cpuset-cpus', '8,9', '--cpus', '2', '--memory', '64g', '--memory-swap', '64g',
        '--pids-limit', '256', '--shm-size', '1g', '--ulimit', 'core=0:0',
        '--ulimit', 'fsize=1073741824:1073741824', '--log-driver=none', '--workdir', '/source',
        '--entrypoint', '/usr/bin/timeout']
    for source, target, readonly in ((MODEL, '/model', True), (ROOT / 'source', '/source', True),
            (ROOT / 'inputs', '/inputs', True), (plan['overlay_root'], '/packages', True),
            (ROOT / 'output', '/output', False), (ROOT / 'scratch', '/scratch', False),
            (ROOT / 'scratch/tmp', '/tmp', False)):
        create += ['--mount', 'type=bind,src=%s,dst=%s%s' % (source, target, ',readonly' if readonly else '')]
    for key, value in literal['container_env'].items():
        create += ['--env', key + '=' + value]
    create += [IMAGE, '--signal=TERM', '--kill-after=15s', '900s', '/usr/bin/python3', '-I', '-B', '/source/run.py']
    before = ['/usr/bin/docker', 'ps', '-aq', '--no-trunc', '--filter', 'name=^/' + NAME + '$']
    result = {'image': ['/usr/bin/docker', 'image', 'inspect', '--format', '{{.Id}}', IMAGE],
        'name-before': before, 'recover': before,
        'name-after': ['/usr/bin/docker', 'ps', '-aq', '--filter', 'name=^/' + NAME + '$'],
        'cpu-tests': ['/usr/bin/python3', '-B', '-m', 'unittest', '-v', 'test_reference'], 'create': create}
    result.update({label: ['/usr/bin/amd-smi', 'process', '--json'] for label in LABELS if label.startswith(('before-', 'after-'))})
    if cid is not None:
        result.update({label: ['/usr/bin/docker', 'inspect', cid]
                       for label in ('created', 'exited', 'cleanup-inspect', 'final-inspect')})
        result.update(execute=['/usr/bin/docker', 'start', '-a', cid],
                      stop=['/usr/bin/docker', 'stop', '--time', '15', cid],
                      remove=['/usr/bin/docker', 'rm', cid])
    return result


def verify(bodies, terminal_name, terminal_sha):
    require(terminal_name in ('complete.json', 'failed.json')
            and pin(bodies[terminal_name])['sha256'] == terminal_sha, 'observed owner terminal hash')
    owner, plan = parse(bodies[terminal_name]), parse(bodies['launch-plan.json'])
    success = terminal_name == 'complete.json'
    require(owner['schema'] == 'ferric-readiness40-position5-reference-owned-v1'
            and owner['passed'] is success and compact(owner['plan']) == pin(bodies['launch-plan.json'])
            and owner['plan']['path'] == str(ROOT / 'launch-plan.json') and owner['retries'] == 0
            and type(owner['framework_attempts']) is int and owner['framework_attempts'] in (0, 1)
            and all(owner[k] is False for k in ('native_execution', 'numerical_acceptance',
                'full_model_acceptance', 'performance_claim', 'production_authority'))
            and owner['acceptance_threshold'] is None
            and (owner['whole_deadline_seconds'], owner['operational_cutoff_seconds'],
                 owner['retirement_reserve_seconds']) == (1800, 1200, 600), 'original owner scope')
    require(plan['schema'] == 'ferric-readiness40-position5-reference-launch-v1' and plan['image'] == IMAGE
            and plan['topology']['host'] == 'smci350-rck-g03-b19-03'
            and plan['topology']['unique_id'] == 16366993098680759275
            and type(plan['topology']['boot']) is str and plan['topology']['boot'], 'observed launch identity')
    manifest_raw = bodies['source/source-manifest.json']
    require(pin(manifest_raw) == SOURCE == compact(plan['source_manifest']), 'frozen source manifest')
    manifest = parse(manifest_raw)
    source_rows = dict(manifest['files'], **{'source-manifest.json': SOURCE})
    require(len(source_rows) == 13 and manifest['schema'] == 'ferric-readiness40-position5-reference-source-v1', 'source closure')
    for name, row in source_rows.items():
        require(Path(name).name == name and pin(bodies['source/' + name]) == row, 'exact source byte pin')
    require(owner['source_files'] == source_rows, 'owner source before/after ledger')
    contract = parse(bodies['source/inputs.json'])
    input_rows = {contract['locations'][role]: compact(row) for role, row in contract['files'].items()}
    input_rows['environment.json'] = compact(plan['environment'])
    require(len(input_rows) == 7 and owner['input_files'] == input_rows, 'owner input before/after ledger')
    for name, row in input_rows.items():
        require(Path(name).name == name and pin(bodies['inputs/' + name]) == row, 'actual immutable input pin')
    environment = parse(bodies['inputs/environment.json'])
    require(environment['schema'] == 'ferric-guarded-mlp-matched-input-environment-v1'
            and environment['image'] == IMAGE and environment['expected_arch'] == 'gfx950'
            and environment['framework_gpu_execution'] is False
            and compact(owner['overlay_manifest']) == compact(plan['overlay_manifest'])
            and owner['overlay_manifest']['path'] == plan['overlay_manifest']['path'], 'honest environment/package provenance')
    reference = pure_reference(bodies)
    try:
        original = {role: bodies['inputs/' + contract['locations'][role]] for role in contract['files']}
        tokens, _, _ = reference.prompt_inputs(contract, original)
        literal = source_literals(bodies['source/launch.py'])
        cid = None
        if 'evidence/create/stdout' in bodies:
            candidate = bodies['evidence/create/stdout'].decode().strip()
            if re.fullmatch('[0-9a-f]{64}', candidate):
                cid = candidate
        if cid is None and 'evidence/recover/stdout' in bodies:
            recovered = bodies['evidence/recover/stdout'].decode().split()
            require(len(recovered) <= 1, 'single recovery owner')
            if recovered:
                require(re.fullmatch('[0-9a-f]{64}', recovered[0]), 'exact recovered container ID')
                cid = recovered[0]
        recipes = commands(plan, cid, literal)
        labels = {name.split('/')[1] for name in bodies if name.startswith('evidence/')}
        require(labels <= LABELS, 'closed original evidence labels')
        checks, idle_count = {}, 0
        for label in sorted(labels):
            prefix = 'evidence/' + label + '/'
            names = {name.removeprefix(prefix) for name in bodies if name.startswith(prefix)}
            require(names <= LEAF_FILES and 'command.json' in names, 'bounded original leaf prefix')
            command = parse(bodies[prefix + 'command.json'])
            require(label in recipes and command == dict(argv=recipes[label], env=literal['CPU_ENV'],
                    seconds=120 if label == 'cpu-tests' else 930 if label == 'execute' else 30), 'exact owned command recipe')
            if success:
                require(names == LEAF_FILES, 'complete successful leaf bytes')
            if 'result.json' not in names:
                checks[label] = dict(result_present=False)
                continue
            result = parse(bodies[prefix + 'result.json'])
            for stream in ('stdout', 'stderr'):
                require(result[stream]['path'] == str(ROOT / prefix / stream)
                        and compact(result[stream]) == pin(bodies[prefix + stream]), 'original stream join')
            started = parse(bodies[prefix + 'started.json'])
            parent = started['parent']
            require(parent['uid'] == 9661 and parent['pid'] == parent['pgid'] == parent['sid']
                    and parent['ppid'] == started['supervisor_pid']
                    and any(row.get('event') == 'owned' and row.get('identity') == parent
                            and row.get('reason') == 'spawned-parent' for row in result['lineage']), 'owned process identity lineage')
            clean = (result['exit_code'] == 0 and result['cleanup_signalled'] is False
                     and result['owned_groups_absent'] is True and result['owned_processes_reaped'] is True
                     and result['reason'] is None and result['observed_signals'] == [])
            if success:
                require(clean, 'successful command natural/reaped/absent without signals')
            checks[label] = dict(result_present=True, clean=clean, exit_code=result['exit_code'])
            if label.startswith(('before-', 'after-')) and clean:
                rows = parse(bodies[prefix + 'stdout'])
                require(type(rows) is list and len(rows) == 8
                        and all(set(row) == {'gpu', 'process_list'} and type(row['gpu']) is int for row in rows)
                        and {row['gpu'] for row in rows} == set(range(8))
                        and all(row['process_list'] == [{'process_info': 'No running processes detected'}]
                                for row in rows), 'actual eight-GPU idle observation')
                idle_count += 1
        output_names = {name.removeprefix('output/') for name in bodies if name.startswith('output/')}
        require(output_names <= OUTPUTS | {'complete.json', 'failed.json'}
                and not {'complete.json', 'failed.json'} <= output_names, 'closed original output prefix')
        inner_name = next((name for name in ('complete.json', 'failed.json') if name in output_names), None)
        inner = parse(bodies['output/' + inner_name]) if inner_name else None
        input_count = 0
        if inner is not None:
            require(inner['schema'] == 'ferric-readiness40-position5-framework-reference-v1'
                    and inner['passed'] is (inner_name == 'complete.json')
                    and all(inner[k] is False for k in ('numerical_acceptance', 'full_model_acceptance',
                        'full_long_workload', 'performance_claim', 'production_authority', 'native_execution',
                        'native_intermediates_used')) and inner['candidate_receipt'] is None
                    and inner['acceptance_threshold'] is None and inner['generated_tokens'] == 0,
                    'original inner outcome and diagnostic-only authority')
            pins = {}
            for row in inner['output_pins']:
                path = Path(row['path'])
                require(path.parent == Path('/output') and path.name not in pins
                        and pin(bodies['output/' + path.name]) == compact(row), 'inner original output pin')
                pins[path.name] = compact(row)
            require(set(pins) == output_names - {inner_name}, 'all original inner output bytes')
            seen = set()
            for row in inner['input_pins']:
                path = row['path']
                require(path not in seen, 'unique inner authenticated readset')
                seen.add(path); input_count += 1
                name = path.lstrip('/') if path.startswith(('/source/', '/inputs/')) else None
                if name is not None:
                    require(name in bodies and pin(bodies[name]) == compact(row), 'retained inner source/input readset join')
                else:
                    choices = [p for p in environment['implementation_sources'].values() if p['path'] == path]
                    require(len(choices) == 1 and compact(row) == compact(choices[0]), 'only declared external implementation inputs')
            if 'environment' in inner:
                require(inner['environment'] == environment, 'actual package/runtime versions retained')
            if 'model_sources' in inner:
                require({name: compact(row, metadata=True) for name, row in inner['model_sources'].items()} == contract['model_files'],
                        'nine original model pins, metadata only in this capsule')
            for name, row in environment['implementation_sources'].items():
                output = 'implementation-' + name + '.py'
                if output in output_names:
                    require(pin(bodies['output/' + output]) == compact(row), 'actual implementation body identity')
        if success:
            require(labels == set(NORMAL) and idle_count == 6 and owner['framework_attempts'] == 1
                    and owner['errors'] == owner['postcheck_errors'] == owner['observed_signals'] == []
                    and owner['container_removed'] is True and 0 <= owner['elapsed_seconds'] < 1800,
                    'successful owner, full17 natural leaves, retired container and clean postchecks')
            require(bodies['evidence/image/stdout'].decode().strip() == IMAGE
                    and not bodies['evidence/name-before/stdout'].strip()
                    and not bodies['evidence/name-after/stdout'].strip() and cid is not None,
                    'cached image and exclusive name before/after')
            for label in ('created', 'exited', 'cleanup-inspect', 'final-inspect'):
                values = parse(bodies['evidence/' + label + '/stdout'])
                require(type(values) is list and len(values) == 1, 'one owned inspected container')
                info = values[0]
                require(info['Id'] == cid and info['Name'] == '/' + NAME and info['Config']['Image'] == IMAGE
                        and info['Config']['Labels']['ferric.readiness.reference.owner'] == str(ROOT)
                        and info['HostConfig']['NetworkMode'] == 'none' and info['State']['Running'] is False,
                        'actual container ownership and stopped-state identity')
                if label != 'created':
                    require(info['State']['Status'] == 'exited' and info['State']['ExitCode'] == 0
                            and info['State']['OOMKilled'] is False, 'natural non-OOM framework process exit')
                if label == 'final-inspect':
                    require(info['State'] == owner['final_container_state'], 'retained final container state')
            text = bodies['evidence/cpu-tests/stderr'].decode()
            names = re.findall(r'^(test_[a-z0-9_]+) \(test_reference\.ReferenceTests(?:\.\1)?\) \.\.\. ok$', text, re.M)
            require(len(names) == len(set(names)) == 20 and set(names) == set(literal['TEST_NAMES'])
                    and re.search(r'\nRan 20 tests in [0-9.]+s\n\nOK\s*$', text), 'exact20 passed original CPU tests')
            require(inner_name == 'complete.json' and owner['framework_report']['path'] == str(ROOT / 'output/complete.json')
                    and compact(owner['framework_report']) == pin(bodies['output/complete.json'])
                    and inner['passed'] is True and inner['error'] is None and inner['postcheck_errors'] == []
                    and inner['full_model_forward_calls'] == 80 and inner['prompt_tokens_authenticated'] == 2048
                    and inner['repeat_gate_passed'] is True and inner['selected_positions'] == list(SELECTED)
                    and inner['full_prompt_tokens'] == tokens and inner['input_tokens'] == tokens[:40]
                    and inner['gpu_device_admitted'] is True and inner['framework_execution'] is True
                    and inner['gpu_context_requested'] is True
                    and inner['elapsed_seconds'] < 900 and output_names == OUTPUTS | {'complete.json'},
                    'actual80-forward two-pass reference output closure')
            require(seen == {'/' + name for name in bodies if name.startswith(('source/', 'inputs/'))}
                    | {row['path'] for row in environment['implementation_sources'].values()}
                    and input_count == 24 and inner['policy'] == dict(sdpa='math-only',
                        deterministic_algorithms=True, bf16_reduced_precision_matmul_reduction=False,
                        sdpa_low_precision_reduction=False, rotary_fp32_preserved=True, autocast=False)
                    and inner['actual_gcn_arch'].split(':')[0] == 'gfx950'
                    and inner['model_id'] == contract['model_id'] and inner['bundle_id'] == contract['bundle_id'],
                    'complete24-input posthash ledger and explicit deterministic numerical policy')
            payloads = [{p: bodies['output/pass%d-pos%d.bf16' % (n, p)] for p in SELECTED} for n in (1, 2)]
            require(reference.repeat_gate(inner['passes'], tokens, payloads), 'two exact complete reference passes and eight payloads')
            for n, value in enumerate(inner['passes'], 1):
                require(parse(bodies['output/pass%d.json' % n]) == value, 'original saved pass identity')
            require(parse(bodies['evidence/execute/stdout']) == dict(path='/output/complete.json',
                    **pin(bodies['output/complete.json'])), 'original container terminal announcement')
        expected = {terminal_name, 'launch-plan.json'} | {'source/' + n for n in source_rows}
        expected |= {'inputs/' + n for n in input_rows} | {'output/' + n for n in output_names}
        expected |= {name for name in bodies if name.startswith('evidence/')}
        require(set(bodies) == expected, 'no extra retained source/input/raw/output bodies')
        if success:
            require(len(bodies) == 122, '122 original successful reference bodies')
        return dict(passed=success, owner_terminal=pin(bodies[terminal_name]), owner_name=terminal_name,
            original_files=len(bodies), raw_files=sum(n.startswith('evidence/') for n in bodies),
            source_files=13, input_files=7, output_files=len(output_names), phases=checks,
            retained_inner_readset_rows=input_count, repeat_gate_rechecked=success,
            full_model_forward_calls=inner.get('full_model_forward_calls') if inner else None,
            native_receipt_authenticated=False, numerical_acceptance=False, full_model_acceptance=False,
            performance_claim=False, model_and_overlay_bodies_retained=False,
            external_model_and_package_rehash_performed=False,
            external_posthash_authority='original authenticated owner/inner receipts only')
    finally:
        for name in ('reference', 'diagnostics', 'common'):
            sys.modules.pop(name, None)


def export(terminal_name, terminal_sha, archive):
    require(archive.is_absolute() and archive.parent.resolve(strict=True) == archive.parent
            and not archive.is_relative_to(ROOT) and not os.path.lexists(archive), 'fresh external archive destination')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ROOT / ('failed.json' if terminal_name == 'complete.json' else 'complete.json')),
            'one immutable original owner terminal')
    names = [terminal_name, 'launch-plan.json']
    for tree in ('source', 'inputs', 'evidence', 'output'):
        names += [tree + '/' + name for name in files(ROOT / tree)]
    require(len(names) == len(set(names)) <= MAX_MEMBERS - 2, 'bounded unique original files')
    bodies = {name: read(ROOT / name) for name in names}
    require(sum(map(len, bodies.values())) <= MAX_TOTAL, 'aggregate original bound')
    checked = verify(bodies, terminal_name, terminal_sha)
    self_raw = read(Path(__file__).resolve())
    selected = dict(bodies, **{'retention-tool.py': self_raw})
    manifest = dict(schema='ferric-readiness40-position5-reference-retention-v1', root=str(ROOT),
        files={name: pin(body) for name, body in sorted(selected.items())}, verification=checked,
        terminal=terminal_name, source_manifest=SOURCE, runtime_execution=False, gpu_execution=False)
    selected['manifest.json'] = encoded(manifest)
    require(all(read(ROOT / name) == body for name, body in bodies.items())
            and read(Path(__file__).resolve()) == self_raw, 'entire original readset posthash')
    partial = archive.with_name(archive.name + '.partial')
    require(not os.path.lexists(partial), 'fresh partial archive')
    with partial.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, body in sorted(selected.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o600, 0
                tar.addfile(info, io.BytesIO(body))
        stream.flush(); os.fsync(stream.fileno())
    require(all(read(ROOT / name) == body for name, body in bodies.items()), 'original evidence stable through export')
    raw = read(partial, MAX_TOTAL)
    os.link(partial, archive); partial.unlink()
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(raw)), members=len(selected),
                         expanded_bytes=sum(map(len, selected.values())), verification=checked), sort_keys=True))


def retain(archive, archive_sha, destination):
    require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
            and not os.path.lexists(destination), 'fresh canonical local retention destination')
    raw = read(archive, MAX_TOTAL)
    require(pin(raw)['sha256'] == archive_sha, 'observed archive SHA')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(0 < len(members) == len({m.name for m in members}) <= MAX_MEMBERS
                and all(m.isfile() and relative(m.name) and not m.pax_headers and 0 <= m.size <= MAX_FILE for m in members)
                and sum(m.size for m in members) <= MAX_TOTAL, 'bounded regular unique USTAR closure')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'archive member byte extents')
    manifest = parse(bodies['manifest.json'])
    require(manifest['schema'] == 'ferric-readiness40-position5-reference-retention-v1'
            and manifest['root'] == str(ROOT) and manifest['source_manifest'] == SOURCE
            and manifest['runtime_execution'] is False and manifest['gpu_execution'] is False
            and set(bodies) == set(manifest['files']) | {'manifest.json'}
            and all(pin(bodies[name]) == row for name, row in manifest['files'].items())
            and bodies['retention-tool.py'] == read(Path(__file__).resolve()), 'archive exact pins and reviewed exporter identity')
    originals = {name: body for name, body in bodies.items() if name not in ('manifest.json', 'retention-tool.py')}
    checked = verify(originals, manifest['terminal'], manifest['verification']['owner_terminal']['sha256'])
    require(checked == manifest['verification'] and read(archive, MAX_TOTAL) == raw, 'independent data verification and archive posthash')
    destination.mkdir(mode=0o700)
    for name, body in sorted(bodies.items()):
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with path.open('xb') as stream:
            stream.write(body)
        require(read(path) == body, 'retained original byte identity')
    receipt = encoded(dict(schema='ferric-readiness40-position5-reference-local-retention-v1', archive=pin(raw),
        manifest=pin(bodies['manifest.json']), verification=checked, original_receipts_modified=False,
        project_execution=False, gpu_execution=False))
    with (destination / 'retention.json').open('xb') as stream:
        stream.write(receipt)
    print(receipt.decode(), end='')


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 5
            and sys.argv[1] in ('export', 'retain') and re.fullmatch('[0-9a-f]{64}', sys.argv[3]),
            'python3 -B evidence.py export OWNER_TERMINAL OBSERVED_SHA ARCHIVE | retain ARCHIVE OBSERVED_SHA FRESH_DIRECTORY')
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 768 << 20), (resource.RLIMIT_CORE, 0), (resource.RLIMIT_FSIZE, MAX_TOTAL)):
        before = resource.getrlimit(kind)
        bound = min([cap] + [n for n in before if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (bound, bound))
    def interrupted(number, _frame):
        raise RuntimeError('retention signal ' + str(number))
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        if sys.argv[1] == 'export':
            export(sys.argv[2], sys.argv[3], Path(sys.argv[4]))
        else:
            retain(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
