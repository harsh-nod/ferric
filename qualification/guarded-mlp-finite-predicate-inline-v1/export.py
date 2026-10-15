"""Export exact compiler qualification and paired lowering evidence, without GPU claims."""
import difflib
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-finite-predicate-inline-publication-v228-v1'
CPU = E / 'guarded-mlp-finite-predicate-inline-cpu-v228-v3'
BASE = E / 'guarded-mlp-coverage-inline-cpu-v228-v1'
PAIR = E / 'guarded-mlp-early-stop-compile-v228-v18'
CPU_SHA = '41d0425beff3ebef1c98910bc904402452c83cddbb8c72229cb9c05e42787a1a'
LOWERER_PASSED = 125
LOWERER_UNIT_PASSED = 769
LOWERER_UNIT_IGNORED = 0


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 16 << 20, 'bounded canonical original')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(path.lstat()) and len(raw) == before.st_size, 'original changed')
    return raw


def put(name, raw):
    path = ROOT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as stream:
        stream.write(raw)
    require(read(path) == raw, 'export differs')


def clean(phase, code):
    require(phase['exit_code'] == code and phase['natural_exit'] is True
            and phase['reaped'] is True and phase['process_group_absent'] is True
            and phase['timed_out'] is False and phase['forced_cleanup'] is False
            and phase['exception'] is None and phase['storage_failure'] is None,
            'clean owned retirement required')


def main():
    require(os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.getuid() == os.geteuid() == 9661, 'MI350 owner')
    os.sched_setaffinity(0, {8, 9})
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    members, originals, arms = {}, {}, {}

    def add(name, path):
        raw = read(path)
        require(name not in members and len(members) < 255, 'unique bounded roster')
        require(sum(map(len, members.values())) + len(raw) <= 256 << 20, 'expanded archive bound')
        members[name] = raw
        originals[name] = dict(path=str(path), **pin(raw))
        return raw

    raw = read(CPU / 'evidence/complete.json')
    require(pin(raw)['sha256'] == CPU_SHA, 'exact completed CPU qualification')
    cpu = json.loads(raw)
    require(cpu['passed'] is True and cpu['failure'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and len(cpu['phases']) == 19
            and cpu['input_sources'] == cpu['final_sources'], 'complete unchanged CPU scope')
    require(cpu['admission_changed'] is False and cpu['source_origin_authentication_extended'] is False
            and cpu['rejection_diagnostic_extended'] is False
            and cpu['semantic_predicates_changed'] is False and cpu['lowered_call_transport_extended'] is False
            and cpu['helper_abi_changed'] is False and cpu['reviewed_provider_source_pin_changed'] is True
            and cpu['worker_coverage_gpu_inline_policy_changed'] is False
            and cpu['bf16_finite_gpu_inline_policy_changed'] is True
            and cpu['bf16_finite_body_changed'] is False
            and cpu['worker_coverage_body_changed'] is False
            and cpu['execution_scope']['allowed_cpus'] == [8, 9]
            and cpu['execution_scope']['memory_max'] == str(12 << 30)
            and cpu['execution_scope']['memory_swap_max'] == '0',
            'BF16-finite-inline policy and unchanged acceptance/body with bounded CPU scope')
    for key, count, ignored in (('compiler', 1341, 24), ('device', 361, 0),
                                ('lowerer', LOWERER_PASSED, 0),
                                ('lowerer-unit', LOWERER_UNIT_PASSED, LOWERER_UNIT_IGNORED)):
        require(cpu['tests'][key]['passed'] == count and cpu['tests'][key]['ignored'] == ignored
                and cpu['tests'][key]['failed'] == 0, 'full suite exact outcomes')
    for phase in cpu['phases']:
        clean(phase, 0)
    for name, row in cpu['raw'].items():
        require(pin(read(CPU / 'evidence' / name)) == compact(row), 'raw CPU join')
    for path in sorted((CPU / 'evidence').iterdir()):
        if path.is_file():
            add('cpu/evidence/' + path.name, path)
    for name in ('run_cpu.py', 'prepare_inputs.py', 'supervisor.py', 'qualification_helpers.py',
                 'input-manifest.json'):
        retained = add('cpu/' + name, CPU / name)
        expected = cpu['input_manifest'] if name == 'input-manifest.json' else cpu['final_sources'][name]
        require(pin(retained) == compact(expected), 'exact CPU auxiliary join')
    base_raw = add('source-base-complete.json', BASE / 'evidence/complete.json')
    require(pin(base_raw) == compact(cpu['base_cpu_complete']), 'exact parent terminal join')
    patch = []
    require(len(cpu['source_delta']) == 2, 'exact BF16 policy, regression and provider authentication sources')
    for name, delta in sorted(cpu['source_delta'].items()):
        old = add('source-before/' + name, BASE / name)
        new = add('source-after/' + name, CPU / name)
        require(pin(old) == delta['before'] and pin(new) == delta['after'], 'exact source pre/postimages')
        relative = name.removeprefix('fe2o3/')
        put('source/' + relative, new)
        patch.extend(difflib.unified_diff(old.decode().splitlines(keepends=True),
                                         new.decode().splitlines(keepends=True),
                                         fromfile='a/' + relative, tofile='b/' + relative))
    patch_raw = ''.join(patch).encode()
    put('correction.patch', patch_raw)
    add('correction.patch', ROOT / 'correction.patch')
    put('cpu/complete.json', raw)
    for name in ('compiler-tests.stdout', 'device-tests.stdout', 'lowerer-tests.stdout', 'lowerer-unit-tests.stdout'):
        put('cpu/' + name, read(CPU / 'evidence' / name))
    probe_raw = add('pair/probe.py', PAIR / 'probe.py')
    for arm in ('fixed', 'early'):
        fixture = PAIR / arm
        run = PAIR / (arm + '-run')
        evidence = run / 'evidence'
        result_raw = read(evidence / 'result.json')
        result = json.loads(result_raw)
        require(result['arm'] == arm and result['postcheck_errors'] == []
                and len(result['phases']) == 1, 'terminal paired compilation')
        clean(result['phases'][0], 0 if result['compile_accepted'] else 1)
        require(read(evidence / 'sources-before.json') == read(evidence / 'sources-after.json'),
                'paired source mutation')
        sources = json.loads(read(evidence / 'sources-after.json'))
        require(pin(probe_raw) == compact(sources[str(PAIR / 'probe.py')]), 'paired controller join')
        for path in sorted(fixture.rglob('*')):
            if path.is_file():
                retained = add('pair/' + arm + '/' + str(path.relative_to(fixture)), path)
                require(pin(retained) == compact(sources[str(path)]), 'exact paired fixture join')
        for key in ('command', 'stdout', 'stderr'):
            expected = result['phases'][0][key]
            path = evidence / Path(expected['path']).name
            require(str(path) == expected['path'] and pin(read(path)) == compact(expected),
                    'paired phase raw pin join')
        inputs = json.loads(read(evidence / 'inputs.json'))
        require(inputs['backend_cpu_receipt']['sha256'] == CPU_SHA, 'qualified compiler join')
        for path in sorted(evidence.iterdir()):
            if path.is_file():
                add('pair/' + arm + '-run/evidence/' + path.name, path)
        for name, row in result['diagnostic_files'].items():
            diagnostic = add('pair/' + arm + '-run/diagnostic/' + name, Path(row['path']))
            require(pin(diagnostic) == compact(row), 'exact diagnostic')
        products = {}
        for path in sorted((run / 'fe2o3-engineering-v1').rglob('*')):
            if path.is_file():
                name = str(path.relative_to(run))
                products[name] = pin(add('pair/' + arm + '-run/' + name, path))
        require(not result['compile_accepted'] or bool(products), 'accepted compile needs products')
        put('pair/' + arm + '/result.json', result_raw)
        put('pair/' + arm + '/compile.stderr', read(evidence / 'compile.stderr'))
        arms[arm] = dict(result=pin(result_raw), accepted=result['compile_accepted'],
                         elapsed_seconds=result['elapsed_seconds'], products=products,
                         diagnostics=result['diagnostic_files'])
    census_root = E / 'guarded-mlp-finite-predicate-census-v228-v1'
    census_raw = read(census_root / 'evidence/complete.json')
    census = json.loads(census_raw)
    require(census['passed'] and census['failure'] is None and not census['postcheck_errors']
            and census['inputs_before'] == census['inputs_after']
            and len(census['phases']) == 2 and not census['gpu_execution']
            and not census['compiler_worker_replayed'] and not census['production_authority'],
            'qualified read-only retained census')
    for phase in census['phases']:
        clean(phase, 0)
        for key in ('pid', 'pgid'):
            require(not Path('/proc/' + str(phase[key])).exists(), 'census child retired')
    require(not Path('/proc/' + str(census['pid'])).exists(), 'census controller retired')
    for path in sorted((census_root / 'evidence').iterdir()):
        if path.is_file():
            add('retained-census/evidence/' + path.name, path)
    add('retained-census/run.py', census_root / 'run.py')
    for index, (path, expected) in enumerate(sorted(census['inputs_after'].items())):
        existing = [row for row in originals.values() if row['path'] == path]
        if existing:
            require(all(compact(row) == compact(expected) for row in existing),
                    'shared census original input join')
        else:
            body = add('retained-census/inputs/' + str(index) + '/' + Path(path).name, Path(path))
            require(pin(body) == compact(expected), 'retained census exact input')
    put('retained-census/complete.json', census_raw)
    for arm in ('fixed', 'early'):
        body = read(census_root / ('evidence/census-' + arm + '.stdout'))
        require(json.loads(body) == census['reports'][arm], 'retained census report join')
        put('retained-census/' + arm + '.json', body)
    failed_results = {}
    for version, sha, expected_phases in (
            ('v1', '7d16ab976d8dabff80cccd92472f3b1e918775ad71eacee741dff9ff2d6ff279', 0),
            ('v2', '06490fa99a0d9aa922450148073d4373434780b0bf31d2327e54e642e2f6e9eb', 6)):
        failed_root = E / ('guarded-mlp-finite-predicate-inline-cpu-v228-' + version)
        prefix = 'failed-' + version + '/'
        failed_raw = add(prefix + 'evidence/failed.json', failed_root / 'evidence/failed.json')
        require(pin(failed_raw)['sha256'] == sha, 'preserved original failed qualification')
        failed = json.loads(failed_raw)
        require(failed['passed'] is False and failed['source_unchanged']
                and failed['postcheck_errors'] == [] and len(failed['phases']) == expected_phases,
                'original failed qualification scope')
        for phase in failed['phases']:
            clean(phase, phase['exit_code'])
            for key in ('pid', 'pgid'):
                require(not Path('/proc/' + str(phase[key])).exists(), 'failed child still present')
        require(not Path('/proc/' + str(failed['execution_scope']['pid'])).exists(),
                'failed controller still present')
        for name, row in failed['raw'].items():
            require('/' not in name, 'raw basename')
            body = add(prefix + 'evidence/' + name, failed_root / 'evidence' / name)
            require(pin(body) == compact(row), 'failed raw join')
        for name in ('run_cpu.py', 'prepare_inputs.py', 'supervisor.py', 'qualification_helpers.py',
                     'input-manifest.json'):
            body = add(prefix + name, failed_root / name)
            expected = failed['input_manifest'] if name == 'input-manifest.json' else failed['final_sources'][name]
            require(pin(body) == compact(expected), 'failed input/source join')
        for name, delta in failed['source_delta'].items():
            before = add(prefix + 'source-before/' + name, BASE / name)
            after = add(prefix + 'source-after/' + name, failed_root / name)
            require(pin(before) == delta['before'] and pin(after) == delta['after'],
                    'failed source delta pre/postimages')
        failed_results[version] = dict(receipt=pin(failed_raw), phases=expected_phases,
                                      failure=failed['failure'])
    for phase in cpu['phases']:
        for key in ('pid', 'pgid'):
            require(not Path('/proc/' + str(phase[key])).exists(), 'qualified child still present')
    require(not Path('/proc/' + str(cpu['execution_scope']['pid'])).exists(),
            'qualified controller still present')
    add('export.py', ROOT / 'export.py')
    members['originals.json'] = (json.dumps(dict(schema='ferric-finite-predicate-inline-originals-v1',
                                                originals=originals), indent=2, sort_keys=True) + '\n').encode()
    require(len(members) <= 256 and sum(map(len, members.values())) <= 256 << 20,
            'complete archive bounds include manifest')
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, body in sorted(members.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(body), 0o600, 0
            archive.addfile(info, io.BytesIO(body))
    compressed = gzip.compress(buffer.getvalue(), mtime=0)
    put('originals.tar.gz', compressed)
    with tarfile.open(fileobj=io.BytesIO(read(ROOT / 'originals.tar.gz')), mode='r:gz') as archive:
        entries = archive.getmembers()
        require(len(entries) == len(members) and [entry.name for entry in entries] == sorted(members),
                'exact closed archive roster')
        for entry in entries:
            require(entry.isfile() and entry.size == len(members[entry.name])
                    and archive.extractfile(entry).read() == members[entry.name], 'archive byte readback')
    manifest = dict(schema='ferric-finite-predicate-inline-publication-v1',
                    cpu=dict(receipt=pin(raw), phases=19,
                             tests={'compiler': [1341, 24], 'device': [361, 0], 'lowerer': [LOWERER_PASSED, 0],
                                    'lowerer-unit': [LOWERER_UNIT_PASSED, LOWERER_UNIT_IGNORED]},
                             elapsed_seconds=cpu['elapsed_seconds'], artifacts=cpu['artifacts'],
                             execution_scope=cpu['execution_scope']),
                    pair=arms, retained_census=pin(census_raw), failed_qualifications=failed_results,
                    diagnostic_patch=pin(patch_raw), archive=pin(compressed),
                    members=len(members), originals=len(originals), expanded_bytes=sum(map(len, members.values())),
                    gpu_execution=False, numerical_acceptance=False, performance_claim=False,
                    production_authority=False, full_model_acceptance=False)
    put('manifest.json', (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode())
    print(json.dumps(dict(manifest=pin(read(ROOT / 'manifest.json')), archive=pin(compressed),
                          members=len(members), originals=len(originals)), sort_keys=True))


if __name__ == '__main__':
    main()
