"""Retain actual ordered-segment CPU metadata/source; never export ELF bodies."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import tarfile

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-projection-ordered-segment-cpu-v2'
CONTROLLER_SHA = 'c5fd9ef33af9f6bdae2adac608f69a7d88a969e1385210f8c65c371a86077d6f'
OLD = E / 'projection-ar4-shared-host-cpu-v228-v1'
OLD_SHA = 'deb2aeacded08c336d1fb5a1638cd2accc2b65ec65ffa3e90177bd5e61146d74'
SOURCE_SHA = 'ed98abb445b2dc3209fcd575bfb25e817b86b7bce0dbdb5f299984067149f3b4'
RUNTIME = E / 'gfx950-clock-cpu-v228-v1'
RUNTIME_SHA = '458732e9e0c67501f33584ff1c6c041f93c0b9021884b504c4a7dfb206610ad3'
PARENT = 'ferric-qwen3-finite-projection-residual-decode'
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
OLD_BINARIES = {PARENT + suffix for suffix in ('-engineering', '-host-engineering', '-shared-host-engineering')} | {WORKER}
MAPS = {'sources-base.json', 'sources-unformatted.json', 'sources-before.json',
        'sources-after.json', 'old-targets-before.json', 'old-targets-after.json', 'configurations.json'}
TAILS = ('-command.json', '-started.json', '-result.json', '-stdout', '-stderr')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def stamp(value):
    return tuple(getattr(value, key) for key in
        ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns'))


def read(path, retain=False, executable=False):
    require(path.resolve(strict=True) == path and path.is_relative_to(R), 'canonical task-root body')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == 9661 and before.st_nlink > 0
            and 0 <= before.st_size <= 128 << 20, 'bounded owned ordinary body')
    require(not retain or before.st_size <= 16 << 20, 'bounded retained metadata')
    digest, chunks, header = hashlib.sha256(), [], b''
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened identity')
        while block := stream.read(1 << 20):
            digest.update(block)
            if not header:
                header = block[:20]
            if retain:
                chunks.append(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'stable opened body')
    require(stamp(path.lstat()) == stamp(before), 'stable named body')
    require(not executable or (before.st_mode & 0o111 and header[:6] == b'\x7fELF\x02\x01'
            and header[18:20] == b'\x3e\x00'), 'actual executable x86-64 ELF')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest()), b''.join(chunks)


def names(raw):
    found = re.findall(r'^([A-Za-z0-9_:]+): test$', raw, re.M)
    require(found and len(found) == len(set(found)), 'unique compiled test inventory')
    return set(found)


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
            and len(sys.argv) == 3, 'ordinary python -B COMPLETE_PATH ACTUAL_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'owned ASROCK CPU8,9 nice10')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 180),
                      (resource.RLIMIT_FSIZE, 256 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (cap, cap))
    receipt, expected_sha = Path(sys.argv[1]), sys.argv[2]
    case = receipt.parent
    require(receipt.name == 'complete.json' and case.parent == E
            and re.fullmatch(r'projection-ordered-segment-cpu-v228-v(?:[2-9]|[1-9][0-9]+)', case.name)
            and re.fullmatch('[0-9a-f]{64}', expected_sha), 'closed actual completion namespace')
    fixed = {}

    def checked(path, expected=None, retain=False, executable=False):
        record, body = read(path, retain, executable)
        require(expected is None or record == expected, 'exact actual body pin: ' + str(path))
        require(path not in fixed or fixed[path] == record, 'repeated path identity')
        fixed[path] = record
        return body

    def document(path, expected=None):
        return json.loads(checked(path, expected, True), object_pairs_hook=pairs)

    value = document(receipt)
    require(fixed[receipt]['sha256'] == expected_sha, 'caller-authenticated actual completion')
    require(value['schema'] == 'ferric-p228-projection-ordered-segment-cpu-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['source_unchanged'] is True and value['empty_initial_target'] is True
            and value['complete_runtime_library_suite_selected'] is True
            and all(value[key] is False for key in ('compiler_requalified', 'gpu_execution',
                'numerical_acceptance', 'performance_claim', 'production_authority')),
            'actual successful bounded CPU observation only')
    checked(PACKAGE / 'run.py', value['controller'])
    require(value['controller']['sha256'] == CONTROLLER_SHA, 'frozen actual controller')
    plan = document(PACKAGE / 'inputs.json', value['plan'])
    require(plan['schema'] == 'ferric-p228-projection-ordered-segment-cpu-inputs-v1'
            and plan['runtime_full_suite_cpu_reviewed'] is True
            and set(plan['proposals']) == {'fe2o3', 'ferric'}
            and re.fullmatch(PARENT + r'-[a-z-]+-engineering', plan['parent_binary'])
            and plan['added_tests'] == value['declared_test_additions'], 'actual root-selected paired plan')
    additions = plan['added_tests']
    require(set(additions) == {'runtime', 'worker', 'parent_library', 'parent_binary'}
            and all(type(v) is list and 0 < len(v) <= 180 and v == sorted(set(v)) for v in additions.values()),
            'four exact additive named-test scopes')
    prior = document(OLD / 'complete.json', value['prior_completion'])
    require(fixed[OLD / 'complete.json']['sha256'] == OLD_SHA and prior['passed'] is True,
            'actual CPU883 predecessor')
    require(0 < len(value['inputs']) <= 4096 and sum(p['bytes'] for p in value['inputs']) <= 1 << 30,
            'bounded exact direct input/tool closure')
    for record in value['inputs']:
        checked(Path(record['path']), record)
    selected = {receipt: fixed[receipt]}
    require(0 < len(value['phases']) <= 150, 'bounded actual phase roster')
    require(set(value['raw']) == MAPS | {name + tail for name in value['phases'] for tail in TAILS},
            'complete phase/map raw roster')
    for name, record in value['raw'].items():
        require(Path(name).name == name and record['path'] == str(case / name), 'closed raw identity')
        checked(case / name, record)
        selected[case / name] = record
    for name, phase in value['phases'].items():
        require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
                and phase['group_absent'] is True and document(case / (name + '-result.json')) == phase,
                'all actual natural successful reaped leaves')
        require(all(phase[stream + '_sha256'] == value['raw'][name + '-' + stream]['sha256']
                    for stream in ('stdout', 'stderr')), 'exact phase streams')
    require(document(case / 'old-targets-before.json') == document(case / 'old-targets-after.json')
            and str(E / 'projection-ordered-segment-cpu-v228-v1/target') in document(case / 'old-targets-before.json'),
            'old target inventory unchanged')
    base = document(case / 'sources-base.json')
    old_sources = document(OLD / 'sources-after.json', prior['raw']['sources-after.json'])
    require(base == old_sources and len(base) == 7003
            and fixed[OLD / 'sources-after.json']['sha256'] == SOURCE_SHA, 'exact original source map')
    expected, overlays = dict(base), {}
    for project, record in plan['proposals'].items():
        folder = E / ('p228-projection-residual-mlp-ordered-runtime-v1' if project == 'fe2o3'
            else 'p228-projection-residual-mlp-ordered-ferric-v2')
        proposal = document(folder / 'source-manifest.json', record)
        rows = proposal['files']
        require(0 < len(rows) <= 64 and len({row['path'] for row in rows}) == len(rows), 'unique bounded source overlay')
        for row in rows:
            relative = Path(row['path']); key = project + '/' + row['path']
            require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == row['path']
                    and row['source'] == 'candidate/' + row['path'], 'canonical overlay member')
            require((project == 'fe2o3' and row['path'].startswith('crates/fe2o3-kfd/src/') and relative.suffix == '.rs')
                    or (project == 'ferric' and row['path'].startswith(('adapters/m1-engineering-execution-v1/',
                        'adapters/tp-peer-finite-engineering-worker-v1/'))
                        and (relative.suffix == '.rs' or row['path'] == 'adapters/m1-engineering-execution-v1/Cargo.toml')),
                    'closed runtime/Ferric source scope')
            require((key not in expected) if row['before'] is None else expected.get(key) == row['before'],
                    'exact source preimage')
            checked(folder / row['source'], dict(path=str(folder / row['source']), **row['after']))
            expected[key] = row['after']; overlays[key] = row
    require(document(case / 'sources-unformatted.json') == expected, 'only reviewed pre-format source changes')
    tested = document(case / 'sources-after.json')
    require(tested == document(case / 'sources-before.json') and set(tested) == set(expected)
            and all(tested[k] == v for k, v in expected.items() if k not in overlays or not k.endswith('.rs')),
            'source postchecks and formatter-only Rust scope')
    for key in overlays:
        path = case / 'sources' / key
        record = dict(path=str(path), **tested[key]); checked(path, record); selected[path] = record
    runtime_prior = document(RUNTIME / 'complete.json')
    require(fixed[RUNTIME / 'complete.json']['sha256'] == RUNTIME_SHA and runtime_prior['passed'] is True,
            'actual original runtime inventory')
    old_runtime_names = names(checked(RUNTIME / 'runtime-list-stdout', runtime_prior['raw']['runtime-list-stdout'], True).decode())
    runtime_names = names(checked(case / 'runtime-list-stdout', retain=True).decode())
    require(runtime_names == old_runtime_names | set(additions['runtime'])
            and not old_runtime_names.intersection(additions['runtime']), 'full additive runtime inventory')
    for name, test in value['tests'].items():
        raw = checked(case / (name + '-stdout'), retain=True).decode()
        rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored)(?:,.*)?$', raw, re.M)
        require(len(rows) == len({v[0] for v in rows}) and {v[0] for v in rows} == set(test['names'])
                and sum(v[1] == 'ok' for v in rows) == test['passed']
                and sum(v[1] == 'ignored' for v in rows) == test['ignored'], 'all named executed outcomes')
        summaries = [list(map(int, row)) for row in re.findall(
            r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw)]
        require(summaries == test['summaries'] and all(row[1] == 0 for row in summaries), 'actual zero-failure summaries')
    require(set(value['tests']['runtime-tests']['names']) == runtime_names, 'runtime list/run identity')
    runtime_ignored = {name for name, status in re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored)(?:,.*)?$',
        checked(case / 'runtime-tests-stdout', retain=True).decode(), re.M) if status == 'ignored'}
    require({v.rsplit('::', 1)[-1] for v in runtime_ignored} == {
        'retained_fixed_image_passes_same_engine_intake', 'actual_multiwave_image_passes_distinct_same_engine_intake',
        'real_gfx950_kernel_rejects_before_fixed_dispatch_data_preparation'} and len(runtime_ignored) == 3,
        'three unchanged historical runtime fixture ignores')
    old_worker = checked(OLD / 'worker-tests-stdout', prior['raw']['worker-tests-stdout'], True).decode()
    worker = checked(case / 'worker-tests-stdout', retain=True).decode()
    ignore_names = lambda raw: set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored', raw, re.M))
    require(ignore_names(worker) == ignore_names(old_worker) and len(ignore_names(worker)) == 4,
            'four unchanged historical worker ignores')
    expected_passed = 883 + len(runtime_names) - 3 + sum(len(additions[k]) for k in ('worker', 'parent_library', 'parent_binary'))
    require(value['tests_passed'] == sum(v['passed'] for v in value['tests'].values()) == expected_passed
            and value['tests_ignored'] == sum(v['ignored'] for v in value['tests'].values()) == 7,
            'derived named totals, not predicted qualification')
    require(plan['parent_binary'] not in OLD_BINARIES
            and set(value['binaries']) == OLD_BINARIES | {plan['parent_binary']}, 'five production executable roles')
    for name, artifact in value['binaries'].items():
        role = 'worker' if name == WORKER else 'parent'
        path = case / 'target' / role / 'debug' / name
        checked(path, artifact['binary'], executable=True)
        stream = checked(case / ('worker-build-stdout' if role == 'worker' else 'parent-builds-stdout'), retain=True).decode()
        objects = [json.loads(line, object_pairs_hook=pairs) for line in stream.splitlines() if line.startswith('{')]
        require(objects.count(artifact['artifact']) == 1 and artifact['artifact']['executable'] == str(path)
                and artifact['artifact']['target']['name'] == name and artifact['artifact']['profile']['test'] is False,
                'actual selected Cargo executable')
    checked(Path(__file__).resolve(strict=True))
    require(len(selected) == 1 + len(value['raw']) + len(overlays)
            and sum(row['bytes'] for row in selected.values()) <= 256 << 20, 'bounded exact courier roster')
    archive = E / (case.name + '-retained.tar.gz')
    with tarfile.open(archive, 'x:gz', dereference=True) as output:
        for path, record in sorted(selected.items()):
            checked(path, record)
            output.add(path, arcname=str(path.relative_to(E)), recursive=False)
    with tarfile.open(archive, 'r:gz') as retained:
        members = retained.getmembers()
        require(len(members) == len(selected) and len({m.name for m in members}) == len(members), 'exact tar census')
        for member in members:
            record = selected.get(E / member.name)
            require(member.isfile() and record is not None and member.size == record['bytes']
                    and str(Path(member.name)) == member.name and '..' not in Path(member.name).parts,
                    'ordinary original-path tar body')
            digest = hashlib.sha256()
            with retained.extractfile(member) as stream:
                while block := stream.read(1 << 20):
                    digest.update(block)
            require(digest.hexdigest() == record['sha256'], 'actual archive member hash')
    require(all(read(path)[0] == record for path, record in fixed.items()), 'all consumed body postchecks')
    print(json.dumps(dict(archive=read(archive)[0], members=len(selected),
        bytes=sum(row['bytes'] for row in selected.values()), formatted_overlay_bodies=len(overlays),
        phases=len(value['phases']), tests_passed=value['tests_passed'], tests_ignored=7,
        verified_elf_bodies=5, exported_elf_bodies=0, compiler_requalified=False,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False)), flush=True)


if __name__ == '__main__':
    main()
