"""Retain only the authenticated clean parent V1 metadata failure; execute no project code."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-parent-cpu-v228-v1'
OUT = ROOT / 'evidence'
ARCHIVE = E / 'guarded-mlp-parent-cpu-failed-evidence-v228-v1.tar.gz'
TERMINAL = dict(bytes=1584613, sha256='5d2c84938fb488f821c8b6a24b2f2689dcac0a4edf833b7c1675ac5eeb1c17a0')
INPUT = dict(bytes=251879, sha256='63558dda298d84bc10ff85cebb8f30b5320c4f092098af6789499126247e0088')
SOURCE_ARCHIVE = dict(bytes=6949509, sha256='bc640cf70fb13e57f2ad5af7858bff4492e30e7d94833cf03728e0e17e739e65')
CACHE = dict(bytes=96016, sha256='e8c533b038db432d45a31d461651a50998d0b94f42ecfc678c5823d00ffa8c81')
HELPERS = {
    'run_cpu.py': dict(bytes=28934, sha256='c01d6093aac85c8f05874d8d419be1e9906dfd9acd56828e3801cce38f50ba15'),
    'qualification_support.py': dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'),
    'supervisor.py': dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
}
STAGERS = {
    'stage_parent_cpu.py': (E / 'stage_guarded_parent_cpu_v228_v1.py',
        '51e34b394a1888bd01539efc946758c0d1eda8b784abbdfe8985b9e1ccf1787e'),
    'stage_parent_cache.py': (E / 'stage_guarded_parent_cache_v228_v1.py',
        'f2fbf9463b00498bd99419043faf05fd46ef45b2710283e433a57e86059b98b3'),
}
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'metadata')
PRLIMIT = ['/usr/bin/prlimit', '--as=12884901888', '--cpu=1200', '--fsize=1073741824', '--core=0', '--']
FALSE_FIELDS = ('all_selected_parent_tests_executed', 'full_parent_library_suite_executed',
    'runtime_suite_rerun', 'worker_suite_rerun', 'gpu_execution', 'full_model_acceptance',
    'numerical_acceptance', 'performance_claim', 'production_authority')
MAX_BODY, MAX_TOTAL, MAX_MEMBERS = 16 << 20, 64 << 20, 180


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def stamp(row):
    return (row.st_dev, row.st_ino, row.st_mode, row.st_nlink,
            row.st_size, row.st_mtime_ns, row.st_ctime_ns)


def file_pin(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= 1 << 30, 'bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed before pin')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed during pin')
    require(stamp(path.lstat()) == stamp(before), 'file changed after pin')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def ordinary(name):
    return (type(name) is str and Path(name).as_posix() == name and name not in ('', '.')
            and not Path(name).is_absolute() and '..' not in Path(name).parts)


def paths_below(root):
    require(root.resolve(strict=True) == root and root.is_dir(), 'ordinary source root')
    paths = []
    for directory, dirs, names in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'directory alias')
        paths.extend(Path(directory) / name for name in names)
        require(len(paths) <= 3000, 'bounded source roster')
    return sorted(paths)


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and sys.argv[1] == TERMINAL['sha256'], 'python3 -B export_parent_cpu_failed_v1b.py TERMINAL_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'actual qualification host/UID')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ARCHIVE), 'fresh exact export')
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (128 << 20, 128 << 20))
    os.umask(0o077)
    bodies, observed = {}, {}

    def add(name, path, expected=None):
        require(ordinary(name) and name not in bodies, 'unique retained member')
        before = file_pin(path)
        require(before['bytes'] <= MAX_BODY and (expected is None or compact(before) == compact(expected)),
                'retained input pin: ' + name)
        body = path.read_bytes()
        require(pin(body) == compact(before) and file_pin(path) == before, 'retained body drift')
        bodies[name], observed[str(path)] = body, before
        require(len(bodies) < MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL,
                'retention bounds')
        return body

    def verify(path, expected):
        row = file_pin(path)
        require(row == expected, 'full observed pin drift: ' + str(path))
        observed[str(path)] = row

    result = json.loads(add('evidence/failed.json', OUT / 'failed.json', TERMINAL))
    require(not os.path.lexists(OUT / 'complete.json')
            and result['schema'] == 'ferric-guarded-mlp-parent-cpu-v1'
            and result['passed'] is False and result['postcheck_errors'] == []
            and result['failure'] == "RuntimeError('metadata did not finish naturally/reaped/successfully')"
            and result['source_unchanged'] is True and result['tests'] == {}
            and result['inventory'] == [] and result['artifacts'] == {} and result['metadata'] is None
            and all(result[key] is False for key in FALSE_FIELDS), 'exact failed metadata attempt')
    inputs = json.loads(add('input-manifest.json', ROOT / 'input-manifest.json', INPUT))
    require(set(inputs) == {'schema', 'files', 'lineage', 'parent_overlay', 'cache_manifest', 'git_revision'}
            and inputs['schema'] == 'ferric-guarded-mlp-parent-cpu-input-v1'
            and len(inputs['files']) == 1220 and len(inputs['lineage']) == 96
            and inputs['cache_manifest'] == CACHE and compact(result['input_manifest']) == INPUT,
            'closed actual input')
    for name, expected in HELPERS.items():
        add(name, ROOT / name, expected)
    require(compact(result['controller']) == HELPERS['run_cpu.py']
            and compact(result['supervisor']) == HELPERS['supervisor.py'], 'actual controller identity')
    for name, (path, digest) in STAGERS.items():
        require(pin(add(name, path))['sha256'] == digest, 'reviewed stager identity')
    stage = json.loads(add('stage.json', ROOT / 'stage.json'))
    require(stage['schema'] == 'ferric-guarded-mlp-parent-source-stage-v1' and stage['passed'] is True
            and stage['archive'] == SOURCE_ARCHIVE and stage['input_manifest'] == INPUT
            and stage['members'] == 1317 and stage['source_files'] == 1220
            and stage['root'] == str(ROOT) and all(stage[k] is False for k in
                ('project_code_executed', 'cargo_execution', 'gpu_execution', 'qualification_passed')),
            'actual source stage joins')
    require(set(result['readset']) == set(inputs['lineage']), 'closed96 readset')
    for name, expected in inputs['lineage'].items():
        require(ordinary(name) and '/' not in name, 'ordinary lineage basename')
        add('inputs/' + name, ROOT / 'inputs' / name, expected)
        verify(ROOT / 'inputs' / name, result['readset'][name])
    proposal = json.loads(bodies['inputs/parent-source-manifest.json'])
    worker = json.loads(bodies['inputs/worker-source.json'])
    committed = json.loads(bodies['inputs/committed-source.json'])
    overlay = {'ferric/' + name: row for name, row in proposal['files'].items()}
    overlay.update(worker['overlay'])
    require(len(proposal['files']) == 7 and len(worker['overlay']) == 19 and len(overlay) == 26
            and committed['overlay'] == overlay and len(committed['files']) == 1202
            and committed['revision'] == inputs['git_revision'] == result['git_revision']
            and committed['revision'].startswith('47b5165a'), 'committed47b plus26 reviewed source overlay')
    reconstructed = dict(committed['files'])
    for name, row in overlay.items():
        require(ordinary(name) and name.startswith('ferric/') and reconstructed.get(name) == row['before'],
                'reviewed source preimage')
        reconstructed[name] = row['after']
    require(reconstructed == {n: p for n, p in inputs['files'].items() if n.startswith('ferric/')},
            'complete preformat source reconstruction')
    before, formatted, final = (result[key] for key in ('preformat_sources', 'input_sources', 'final_sources'))
    require({n: compact(p) for n, p in before.items()} == inputs['files']
            and len(final) == 1220 and formatted == final and set(before) == set(final), 'source map joins')
    changed = sorted(n for n in before if before[n] != formatted[n])
    require(len(result['format_changed_paths']) == len(changed)
            and set(changed) == set(result['format_changed_paths']) and set(changed) <= set(inputs['parent_overlay'])
            and inputs['parent_overlay'] == sorted('ferric/' + n for n in proposal['files'] if n.endswith('.rs')),
            'only six selected parent sources may format')
    source_paths = paths_below(ROOT / 'ferric') + [ROOT / n for n in HELPERS]
    require({str(p.relative_to(ROOT)) for p in source_paths} == set(final), 'full current source roster')
    for name, expected in final.items():
        require(ordinary(name), 'ordinary source key')
        verify(ROOT / name, expected)
    for name in overlay:
        add(name, ROOT / name, final[name])
    cache = json.loads(add('cargo-cache-manifest.json', ROOT / 'cargo-cache-manifest.json', CACHE))
    cache_stage = json.loads(add('cargo-cache-stage-complete.json', ROOT / 'cargo-cache-stage-complete.json'))
    provenance = result['cache_provenance']
    verify(ROOT / 'cargo-cache-manifest.json', provenance['manifest'])
    verify(ROOT / 'cargo-cache-stage-complete.json', provenance['stage'])
    require(cache['schema'] == 'ferric-guarded-mlp-parent-cache-v1' and cache['cargo_execution'] is False
            and len(cache['files']) == 247 and len(cache['packages']) == 118 and len(cache['git_commits']) == 3
            and compact(final['ferric/adapters/m1-engineering-execution-v1/Cargo.lock']) == cache['lock']
            and {n: compact(p) for n, p in provenance['files'].items()} == cache['files']
            and cache_stage['schema'] == 'ferric-guarded-mlp-parent-cache-stage-v1'
            and cache_stage['passed'] is True and cache_stage['manifest'] == CACHE
            and cache_stage['cargo_home'] == str(ROOT / 'cargo-home')
            and cache_stage['lock'] == cache['lock'] and cache_stage['files'] == cache['files']
            and cache_stage['git_commits'] == cache['git_commits'] and cache_stage['packages'] == 118
            and cache_stage['cache_files'] == 247
            and all(cache_stage[k] is False for k in ('shared_cache_changed', 'lock_changed',
                'project_code_executed', 'crate_sources_extracted', 'git_sources_checked_out')),
            'exact original private-cache provenance')
    for name, row in provenance['files'].items():
        require(ordinary(name), 'ordinary immutable cache key')
        verify(ROOT / 'cargo-home' / name, row)
    for row in result['tool_pins'].values():
        verify(Path(row['path']), row)
    require(result['configurations'] and all(p is None for p in result['configurations'].values())
            and all(not os.path.lexists(p) for p in result['configurations']), 'no Cargo configuration drift')
    raw_names = {name + suffix for name in PHASES for suffix in
                 ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    raw_names |= {'sources-preformat.json', 'sources-before.json', 'sources-after.json'}
    require(set(result['raw']) == raw_names and {p.name for p in OUT.iterdir()} == raw_names | {'failed.json'},
            'closed23 raw evidence files')
    for name, expected in result['raw'].items():
        add('evidence/' + name, OUT / name, expected)
    for name, rows in (('sources-preformat.json', before), ('sources-before.json', formatted),
                       ('sources-after.json', final)):
        require(json.loads(bodies['evidence/' + name]) == rows, 'raw source snapshot join')
    require([p['label'] for p in result['phases']] == list(PHASES), 'exact four-phase failure prefix')
    for index, phase in enumerate(result['phases']):
        label = phase['label']
        require(phase['exit_code'] == (101 if index == 3 else 0)
                and all(phase[k] is True for k in ('natural_exit', 'reaped', 'process_group_absent'))
                and all(phase[k] is False for k in ('timed_out', 'forced_cleanup'))
                and all(phase[k] is None for k in ('exception', 'storage_failure'))
                and phase['adopted_reaped'] == [] and phase['observed_signals'] == []
                and phase['argv'][:6] == PRLIMIT, 'clean natural phase prefix')
        command = json.loads(bodies['evidence/' + label + '.command.json'])
        started = json.loads(bodies['evidence/' + label + '.started.json'])
        require(json.loads(bodies['evidence/' + label + '.result.json']) == phase
                and command['argv'] == phase['argv'] and started['pid'] == phase['pid']
                and started['pgid'] == phase['pgid'], 'raw process record joins')
        for key in ('command', 'stdout', 'stderr'):
            require(phase[key] == result['raw'][label + ('.command.json' if key == 'command' else '.' + key)],
                    'phase stream join')
    require(pin(bodies['evidence/metadata.stderr']) == dict(bytes=1268,
        sha256='f8c4aa562370668a9eec07db5c946ae11846b841572b302708b725b3804e8c4e')
        and bodies['evidence/metadata.stdout'] == b'', 'actual local Git refspec refusal')
    add('export_parent_cpu_failed_v1b.py', Path(__file__).resolve())
    require(len(bodies) == 156, 'closed selected evidence capsule')
    manifest = dict(schema='ferric-guarded-mlp-parent-cpu-failed-retention-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())}, terminal=TERMINAL,
        input=INPUT, source_archive=SOURCE_ARCHIVE, cache_manifest=CACHE,
        passed=False, failure=result['failure'], phases=4, raw_count=23, current_tests=0,
        source_rows=1220, overlay_bodies=26, lineage_bodies=96, immutable_cache_rows=247,
        full_source_bodies_retained=False, cache_package_bodies_retained=False,
        host_executable_bodies_retained=False, parent_cpu_qualified=False,
        gpu_execution=False, full_model_acceptance=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
    bodies['manifest.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    require(len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL, 'final capsule bounds')
    for path, expected in observed.items():
        require(file_pin(Path(path)) == expected, 'final source/input/cache/tool/retained-body posthash')
    created = False
    try:
        with ARCHIVE.open('xb') as stream:
            created = True
            with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
                for name, body in sorted(bodies.items()):
                    row = tarfile.TarInfo(name)
                    row.size, row.mode, row.mtime = len(body), 0o600, 0
                    tar.addfile(row, io.BytesIO(body))
        for path, expected in observed.items():
            require(file_pin(Path(path)) == expected, 'post-export input drift')
    except BaseException:
        if created:
            ARCHIVE.unlink(missing_ok=True)
        raise
    print(json.dumps(dict(archive=file_pin(ARCHIVE), terminal=TERMINAL, passed=False,
        members=len(bodies), pinned_members=len(manifest['files']), raw_count=23,
        expanded_bytes=sum(map(len, bodies.values())), qualification_passed=False), sort_keys=True))


if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('parent failure export interrupted: ' + str(signum))
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        main()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
