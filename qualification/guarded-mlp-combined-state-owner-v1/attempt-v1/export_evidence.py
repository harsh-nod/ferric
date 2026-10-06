"""Bounded data-only retention of an actual terminal combined-state owner CPU attempt."""
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import signal
import stat
import sys
import tarfile


E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-combined-state-owner-cpu-v228-v1'
BASE = E / 'peer-dependency-signal-completion-cpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-combined-state-owner-cpu-evidence-v228-v1.tar.gz'
TERMINAL_SHA = 'e1d8582b90344c8cd52925b88bd849b248ea242964a543d1c96b52f91f86d37e'  # Root-measured actual complete receipt.
INPUT_SHA = 'c740a1e926f408413783bf69e8aed2c6f22a46ce86670ad8e3f2ee19f403fd09'
CONTROLLER_SHA = 'ab32b20c622ac6d3f50c9191bfad8a4f38ba38d688ba961ab37c631f118216aa'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
PROPOSAL_SHA = '323ef39a6c6ec1d9c91c179a3e28e6d48bc5f56e99e786df52ff30b68f8dd311'
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
MAX_FILE, MAX_TOTAL, MAX_MEMBERS = 64 << 20, 128 << 20, 100
PHASES = ('rustc-version', 'metadata', 'default-check', 'kfd-tests-build', 'kfd-list',
          'kfd-ignored', 'kfd-tests', 'combined-owner-tests', 'combined-memory-tests')
TOP = ('Cargo.toml', 'Cargo.toml.original', 'Cargo.lock', 'Cargo.lock.input', 'rust-toolchain.toml')
LINEAGE = {
    'base_complete': (BASE / 'evidence/complete.json', 234095,
                      'b4d6edccd6232fc925305d8053b844c3bf072a3f2aade9a774f44d3e67db059f'),
    'base_sources': (BASE / 'evidence/sources-tested.json', 307490,
                     'e307d81cac3a96e7006a5aa1ffe0b62e2aab746ca5ebb735f3e7c5ecc8d623e1'),
    'base_stdout': (BASE / 'evidence/kfd-tests.stdout', 110290,
                    'e0d3b14fb03fe4aa56d7ffeb2fadbb4c39583befe4fe56364b17eff378837899'),
    'base_controller': (BASE / 'run_cpu.py', 27116,
                        'd66b641752986b09952dc73b28ee7984796359e39d1373046dfa2229ffa68406'),
    'rt_input': (E / 'guarded-mlp-segment-cpu-v228-v5/input-manifest.json', 1012525,
                 '9c3cf80f76658c778bc708997858860f8089355a693fab7fad3a4ad706aa1c87'),
}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def pin(path, limit=MAX_FILE):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input required')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= limit, 'bounded ordinary file required')
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file identity changed before read')
        body = stream.read(limit + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file identity changed during read')
    require(len(body) == before.st_size and stamp(path.lstat()) == stamp(before), 'file changed after read')
    return body, dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def main():
    require(type(TERMINAL_SHA) is str and re.fullmatch(r'[0-9a-f]{64}', TERMINAL_SHA),
            'actual terminal pin is not bound')
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and sys.argv[1] == INPUT_SHA, 'python3 -B export_evidence.py ACTUAL_INPUT_SHA')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID mismatch')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ARCHIVE), 'fresh exact archive required')
    signal.alarm(180)
    evidence = ROOT / 'evidence'
    terminals = [p for p in evidence.iterdir() if p.name in ('complete.json', 'failed.json')]
    require(len(terminals) == 1, 'one actual terminal receipt required')
    receipt_body, receipt_pin = pin(terminals[0])
    require(receipt_pin['sha256'] == TERMINAL_SHA, 'actual terminal receipt mismatch')
    receipt = json.loads(receipt_body)
    require(receipt['schema'] == 'ferric-guarded-mlp-combined-state-owner-cpu-v1'
            and type(receipt['passed']) is bool and receipt['passed'] == (terminals[0].name == 'complete.json')
            and receipt['source_generation'] == GENERATION and receipt['additive_private_owner'] is True,
            'terminal schema/outcome mismatch')
    for name in ('legacy_state_v2_changed', 'legacy_profiles_changed', 'gpu_execution', 'worker_integrated',
                 'coordinator_implemented', 'native_peer_ordering_qualified', 'production_authority',
                 'performance_claim', 'doctests_executed', 'live_validation_enabled'):
        require(receipt[name] is False, 'CPU retention cannot grant extra authority')
    require((receipt['passed'] and receipt['failure'] is None and receipt['postcheck_errors'] == [])
            or (not receipt['passed'] and receipt['failure'] is not None), 'failure truth')
    input_body, input_pin = pin(ROOT / 'input-manifest.json')
    inputs = json.loads(input_body)
    require(input_pin['bytes'] == 163029 and input_pin['sha256'] == INPUT_SHA
            and receipt['input_manifest'] == input_pin
            and set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins', 'source_lineage'}
            and inputs['schema'] == 'ferric-guarded-mlp-combined-state-owner-cpu-input-v1'
            and inputs['source_generation'] == GENERATION and len(inputs['files']) == 790,
            'actual input identity')
    if receipt['source_lineage'] is not None:
        require(receipt['source_lineage'] == inputs['source_lineage'], 'reported lineage differs')
    for name, row in receipt['tool_pins'].items():
        require(inputs['tool_pins'][name] == row, 'reported tool differs')
    files, bodies = {}, {}
    def retain(name, path, expected=None):
        relative = PurePosixPath(name)
        require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == name
                and name not in files and len(files) < MAX_MEMBERS, 'closed unique archive member')
        body, actual = pin(path)
        require(expected is None or actual == expected, 'retained body differs: ' + name)
        require(sum(map(len, bodies.values())) + len(body) <= MAX_TOTAL, 'retention byte cap')
        bodies[name], files[name] = body, actual
        return body
    retain('evidence/' + terminals[0].name, terminals[0], receipt_pin)
    retain('input-manifest.json', ROOT / 'input-manifest.json', input_pin)
    for name, key, digest in (('run_cpu.py', 'controller', CONTROLLER_SHA),
                              ('supervisor.py', 'supervisor', SUPERVISOR_SHA)):
        retain(name, ROOT / name, receipt[key])
        require(files[name]['sha256'] == digest and compact(files[name]) == inputs['files'][name],
                'known harness differs')
    raw = receipt['raw']
    allowed = {label + suffix for label in PHASES for suffix in
               ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    allowed |= {'sources-before.json', 'sources-after.json', 'dependencies-before.json', 'dependencies-after.json'}
    require(type(raw) is dict and set(raw) <= allowed and len(raw) <= 49
            and {p.name for p in evidence.iterdir()} == set(raw) | {terminals[0].name}, 'closed raw directory')
    for name, row in sorted(raw.items()):
        require(Path(name).name == name and row['path'] == str(evidence / name), 'original raw path')
        retain('evidence/' + name, evidence / name, row)
    phases = receipt['phases']
    require([row['label'] for row in phases] == list(PHASES[:len(phases)]), 'actual phase prefix differs')
    for row in phases:
        label = row['label']
        require(row['reaped'] is True and row['process_group_absent'] is True
                and not os.path.exists('/proc/' + str(row['pid'])), 'owned leaf not terminal/reaped')
        require(json.loads(bodies['evidence/' + label + '.result.json']) == row
                and raw[label + '.command.json'] == row['command']
                and raw[label + '.stdout'] == row['stdout'] and raw[label + '.stderr'] == row['stderr'],
                'phase raw join')
        command = json.loads(bodies['evidence/' + label + '.command.json'])
        started = json.loads(bodies['evidence/' + label + '.started.json'])
        require(command['argv'] == started['argv'] == row['argv'] and command['cwd'] == str(ROOT / 'fe2o3')
                and started['pid'] == row['pid'] and started['pgid'] == row['pgid'], 'command/ownership join')
        if receipt['passed']:
            require(row['exit_code'] == 0 and row['natural_exit'] is True and row['forced_cleanup'] is False
                    and row['timed_out'] is False and row['exception'] is None and row['storage_failure'] is None,
                    'success requires clean natural leaf')
    before, final = receipt['input_sources'], receipt['final_sources']
    for name, value in (('sources-before.json', before), ('sources-after.json', final)):
        if name in raw:
            require(json.loads(bodies['evidence/' + name]) == value, 'source map/receipt join')
    require(receipt['source_unchanged'] is (before is not None and final == before), 'source-change truth')
    if before is not None:
        require({name: compact(row) for name, row in before.items()} == inputs['files']
                and all(row['path'] == str(ROOT / name) for name, row in before.items()), 'full input map join')
    lineage = inputs['source_lineage']
    require(set(lineage) == set(LINEAGE) | {'owner_proposal', 'owner_overlay'}, 'closed lineage')
    proof = {}
    for name, (path, size, digest) in LINEAGE.items():
        require(lineage[name] == dict(path=str(path), bytes=size, sha256=digest), 'literal lineage body')
        proof[name] = retain('lineage/' + name + ('.stdout' if name == 'base_stdout' else '.py' if name == 'base_controller' else '.json'),
                             path, lineage[name])
    proposal_body = retain('inputs/owner-source.json', ROOT / 'inputs/owner-source.json', lineage['owner_proposal'])
    require(files['inputs/owner-source.json']['sha256'] == PROPOSAL_SHA, 'source authority pin')
    proposal = json.loads(proposal_body)
    base, base_sources = json.loads(proof['base_complete']), json.loads(proof['base_sources'])
    require(base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['tested_sources'] == lineage['base_sources']
            and base['controller'] == lineage['base_controller']
            and base['raw']['kfd-tests.stdout'] == lineage['base_stdout'], 'baseline proof join')
    require(len(proposal['files']) == 7 and sum(r['before'] is None for r in proposal['files'].values()) == 4
            and set(lineage['owner_overlay']) == set(proposal['files']), 'owner closed overlay')
    expected = {'fe2o3/' + name: compact(row) for name, row in base_sources.items() if name != 'run_cpu.py'}
    selected_sources = {'fe2o3/' + name for name in TOP}
    for name, row in proposal['files'].items():
        full = 'fe2o3/' + name
        require(expected.get(full) == row['before'] and lineage['owner_overlay'][name]
                == dict(path=str(ROOT / full), **row['after']), 'overlay pre/post lineage')
        expected[full] = row['after']
        selected_sources.add(full)
    require(len(expected) == 788 and {name: row for name, row in inputs['files'].items()
            if name.startswith('fe2o3/')} == expected, 'exact owner source closure')
    for name in sorted(selected_sources):
        expected_row = final.get(name) if final is not None else None
        retain(name, ROOT / name, expected_row)
        if receipt['passed']:
            require(compact(files[name]) == inputs['files'][name], 'success source postimage differs')
    declared_readset = {name: row for name, row in lineage.items() if name != 'owner_overlay'}
    declared_readset.update({'owner_overlay/' + name: row for name, row in lineage['owner_overlay'].items()})
    require(set(receipt['readset']) <= set(declared_readset)
            and all(declared_readset[name] == row for name, row in receipt['readset'].items()), 'reported readset join')
    expected_cohorts = {}
    for prefix, data in proposal['test_cohorts'].items():
        label = 'combined-owner-tests' if prefix.startswith('engineering_gfx950::') else 'combined-memory-tests'
        expected_cohorts[label] = data
    require(receipt['owner_cohorts'] == expected_cohorts, 'source/receipt focused names')
    if receipt['baseline_tests'] is not None:
        require(receipt['baseline_tests'] == base['tests']['kfd-tests'], 'actual baseline named census')
    require(set(receipt['tests']) <= {'kfd-tests', *expected_cohorts}, 'bounded completed test scopes')
    if receipt['passed']:
        require(before == final and before is not None and set(raw) == allowed
                and len(phases) == 9 and receipt['readset'] == declared_readset
                and receipt['tool_pins'] == inputs['tool_pins'] and receipt['full_kfd_tests_executed'] is True,
                'success closure incomplete')
        tests = receipt['tests']
        require(set(tests) == {'kfd-tests', *expected_cohorts}
                and (tests['kfd-tests']['passed'], tests['kfd-tests']['failed'], tests['kfd-tests']['ignored']) == (1008, 0, 3),
                'full KFD result differs')
        old = {r['name']: r['outcome'] for r in base['tests']['kfd-tests']['named']}
        wanted = dict(old)
        for label, cohort in expected_cohorts.items():
            wanted.update({name: 'ok' for name in cohort['names']})
            value = tests[label]
            require(value == dict(names=cohort['names'], passed=len(cohort['names']), failed=0,
                                  ignored=0, filtered_out=991 - len(cohort['names'])), 'focused full result differs')
        named = tests['kfd-tests']['named']
        require(len(named) == len(wanted) == 1011 and {r['name']: r['outcome'] for r in named} == wanted
                and receipt['inventory'] == sorted(wanted)
                and receipt['ignored'] == sorted(n for n, status in old.items() if status == 'ignored'),
                'all old/new/ignored names required')
        summaries = [dict(row) for row in base['tests']['kfd-tests']['summaries']]
        summaries[0]['passed'] += 14
        require(tests['kfd-tests']['summaries'] == summaries and len(receipt['artifacts']) == 5,
                'five target summaries/artifacts')
        require(json.loads(bodies['evidence/dependencies-before.json'])
                == json.loads(bodies['evidence/dependencies-after.json']), 'success dependency drift')
    retain('export_evidence.py', Path(__file__).resolve())
    manifest = dict(schema='ferric-guarded-mlp-combined-state-owner-cpu-retention-v1',
                    files=files, receipt=receipt_pin, input_manifest=input_pin,
                    passed=receipt['passed'], failure=receipt['failure'], postcheck_errors=receipt['postcheck_errors'],
                    source_unchanged=receipt['source_unchanged'], source_lineage=lineage,
                    selected_source_files=sorted(selected_sources), selected_source_scope='seven-owner-postimages-plus-five-workspace-inputs',
                    full_source_tree_retained=False, exported_binary_bodies=False,
                    actual_artifact_metadata=receipt['artifacts'], gpu_execution=False,
                    native_peer_ordering_qualified=False, worker_integrated=False, performance_claim=False)
    bodies['retention-manifest.json'] = (json.dumps(manifest, sort_keys=True, indent=2) + '\n').encode()
    require(len(bodies) == len(raw) + 24 and (not receipt['passed'] or len(bodies) == 73)
            and len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL, 'exact bounded capsule census')
    with ARCHIVE.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz') as archive:
            for name, body in sorted(bodies.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(body), 0o600, 0
                archive.addfile(member, io.BytesIO(body))
    require(all(pin(Path(row['path']))[1] == row for row in files.values()), 'retained readset drift after archive')
    require({p.name for p in evidence.iterdir()} == set(raw) | {terminals[0].name}, 'evidence roster drift')
    print(json.dumps(dict(archive=pin(ARCHIVE, MAX_TOTAL)[1], members=len(bodies), pins=len(files), raw=len(raw),
                          body_bytes=sum(map(len, bodies.values())), passed=receipt['passed'], receipt=receipt_pin)))


if __name__ == '__main__':
    main()
