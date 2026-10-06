"""Bounded data-only retention of an actual terminal ranked-CFG diagnostic CPU attempt."""
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


ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-ranked-cfg-diagnostic-cpu-v228-v2')
ARCHIVE = ROOT.parent / 'guarded-mlp-ranked-cfg-diagnostic-cpu-evidence-v228-v2.tar.gz'
CONTROLLER_SHA = '239025f3eb11d5b1833c9c4505f6906066220062cab4156bf811d2f45f0f97d5'
HELPER_SHA = 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'
PROPOSAL_SHA = 'aa9e4b5eec3b1c81c8fc1b888507440f12889388536db42bf4ed61856376a397'
INPUT_SHA = '791bd7a5fb6eeba7904c6db2dfe2c861d12ff98605ae3d5774d0f969491d3ce4'
BASE_COMPLETE_SHA = 'e30712ef17c7094efce9bd13bee2bb172823e94f4817c812cfb8b1b20cefb847'
BASE_SOURCES_SHA = 'fa4722605d557e1236809f610bc7af5ebb6e7f15a643ba736392be5b2177bb8a'
BASE_INPUT_SHA = '354e2de2cb6c9d19508030bc8b3d09cbe7a3f17e65a7262f42fd346052b8cc23'
MAX_FILE, MAX_TOTAL, MAX_MEMBERS = 64 << 20, 128 << 20, 500


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
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B export_evidence.py ACTUAL_INPUT_SHA')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID mismatch')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ARCHIVE), 'fresh exact archive required')
    signal.alarm(180)
    evidence = ROOT / 'evidence'
    terminals = [p for p in evidence.iterdir() if p.name in ('complete.json', 'failed.json')]
    require(len(terminals) == 1, 'one actual terminal receipt required')
    receipt_body, receipt_pin = pin(terminals[0])
    receipt = json.loads(receipt_body)
    require(receipt['schema'] == 'ferric-guarded-mlp-ranked-cfg-diagnostic-cpu-v1'
            and type(receipt['passed']) is bool
            and receipt['passed'] == (terminals[0].name == 'complete.json')
            and receipt['diagnostic_build'] is True
            and receipt['diagnostic_changes_admission'] is False, 'terminal outcome/schema mismatch')
    for key in ('gpu_execution', 'guarded_candidate_hsaco_emitted', 'full_model_acceptance',
                'numerical_acceptance', 'performance_claim'):
        require(receipt[key] is False, 'CPU retention cannot grant additional authority')
    input_body, input_pin = pin(ROOT / 'input-manifest.json')
    inputs = json.loads(input_body)
    require(input_pin['sha256'] == sys.argv[1] == INPUT_SHA and receipt['input_manifest'] == input_pin
            and inputs['schema'] == 'ferric-guarded-mlp-ranked-cfg-diagnostic-cpu-input-v1'
            and set(inputs) == {'schema', 'files', 'tool_pins', 'lineage',
                                'metadata_relocations', 'rust_src'}
            and receipt['source_lineage'] == inputs['lineage']
            and receipt['metadata_relocations'] == inputs['metadata_relocations']
            and receipt['tool_pins'] == inputs['tool_pins'], 'literal input/source-lineage join')
    if receipt['passed']:
        require(receipt['failure'] is None and receipt['postcheck_errors'] == []
                and receipt['source_unchanged'] is True and len(receipt['phases']) == 33
                and len(receipt['tests']) == 20 and receipt['tests_passed'] == 2814
                and receipt['tests_ignored'] == 25 and len(receipt['artifacts']) == 7,
                'complete receipt lacks closed successful qualification')
    else:
        require(receipt['failure'] is not None, 'failed receipt must retain failure reason')

    files, bodies = {}, {}
    def retain(name, path, expected=None):
        relative = PurePosixPath(name)
        require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == name
                and name not in files and len(files) < MAX_MEMBERS, 'closed unique archive member')
        body, actual = pin(path)
        require(expected is None or actual == expected, 'retained body pin differs: ' + name)
        require(sum(map(len, bodies.values())) + len(body) <= MAX_TOTAL, 'retention total byte cap')
        bodies[name], files[name] = body, actual
        return body

    retain(str(terminals[0].relative_to(ROOT)), terminals[0], receipt_pin)
    retain('input-manifest.json', ROOT / 'input-manifest.json', input_pin)
    for name, key, digest in (('run_cpu.py', 'controller', CONTROLLER_SHA),
                              ('qualification_helpers.py', 'helper', HELPER_SHA)):
        body = retain(name, ROOT / name, receipt[key])
        require(files[name]['sha256'] == digest
                and compact(files[name]) == inputs['files'][name], 'qualified controller/helper identity')
    raw = receipt['raw']
    require(type(raw) is dict and len(raw) <= 169
            and (not receipt['passed'] or len(raw) == 169)
            and {p.name for p in evidence.iterdir()} == set(raw) | {terminals[0].name},
            'exact closed terminal evidence directory')
    for name, row in sorted(raw.items()):
        require(Path(name).name == name and row['path'] == str(evidence / name), 'original raw path')
        retain('evidence/' + name, evidence / name, row)
    require(len(receipt['phases']) == len({row['label'] for row in receipt['phases']}) <= 33,
            'unique bounded actual phase roster')
    for row in receipt['phases']:
        label = row['label']
        require(row['reaped'] is True and row['process_group_absent'] is True
                and not os.path.exists('/proc/' + str(row['pid'])), 'owned leaf must be terminal/reaped')
        require(json.loads(bodies['evidence/' + label + '.result.json']) == row
                and raw[label + '.command.json'] == row['command']
                and raw[label + '.stdout'] == row['stdout']
                and raw[label + '.stderr'] == row['stderr'], 'actual phase/raw join')
        command = json.loads(bodies['evidence/' + label + '.command.json'])
        started = json.loads(bodies['evidence/' + label + '.started.json'])
        require(command['argv'] == row['argv'] == started['argv']
                and started['pid'] == row['pid'] and started['pgid'] == row['pgid'],
                'actual command/started ownership join')
        if receipt['passed']:
            require(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None,
                    'passing attempt contains unsuccessful leaf')

    lineage = inputs['lineage']
    direct = {'base_complete', 'base_sources', 'base_input', 'base_metadata',
              'base_dependencies', 'diagnostic_proposal'}
    require(set(lineage) == direct | {'base_streams', 'diagnostic_overlay'},
            'closed retained diagnostic lineage fields')
    selected_inputs = {input_pin['path']: input_pin, receipt['helper']['path']: receipt['helper']}
    for key in sorted(direct):
        row = lineage[key]
        retain('lineage/' + key + '.json', row['path'], row)
        selected_inputs[row['path']] = row
    require(set(lineage['base_streams']) == {short + '-' + suffix + '-stdout'
            for short in ('compiler', 'pliron') for suffix in ('list', 'ignored-list', 'tests')},
            'six qualified full-S raw streams required')
    for name, row in sorted(lineage['base_streams'].items()):
        retain('lineage/base-streams/' + name, row['path'], row)
        selected_inputs[row['path']] = row
    proposal = json.loads(bodies['lineage/diagnostic_proposal.json'])
    require(lineage['diagnostic_proposal']['sha256'] == PROPOSAL_SHA
            and proposal['schema'] == 'ferric-guarded-mlp-ranked-cfg-diagnostic-source-v1'
            and len(proposal['files']) == 4
            and sum(row['before'] is None for row in proposal['files'].values()) == 2
            and {'fe2o3/' + name: row for name, row in proposal['files'].items()}
                == lineage['diagnostic_overlay']
            and receipt['cfg_diagnostic_tests'] == sorted(proposal['test_names']),
            'exact frozen four-file/eight-name diagnostic proposal')
    require(lineage['base_complete']['sha256'] == BASE_COMPLETE_SHA
            and lineage['base_sources']['sha256'] == BASE_SOURCES_SHA
            and lineage['base_input']['sha256'] == BASE_INPUT_SHA
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources']),
            'actual qualified baseline identity')
    baseline = json.loads(bodies['lineage/base_complete.json'])
    require(baseline['passed'] is True and baseline['failure'] is None
            and baseline['postcheck_errors'] == [] and baseline['source_unchanged'] is True
            and compact(baseline['input_manifest']) == compact(lineage['base_input'])
            and compact(baseline['input_sources']) == compact(lineage['base_sources'])
            and compact(baseline['final_sources']) == compact(lineage['base_sources'])
            and compact(baseline['raw']['metadata.stdout']) == compact(lineage['base_metadata']),
            'qualified baseline receipt/readset joins')
    for name in ('dependencies-before.json', 'dependencies-after.json'):
        require(compact(baseline['raw'][name]) == compact(lineage['base_dependencies']),
                'qualified dependency before/after map join')
    for short in ('compiler', 'pliron'):
        for suffix in ('list', 'ignored-list', 'tests'):
            key = short + '-' + suffix + '-stdout'
            raw_name = (short + '-tests.stdout' if suffix == 'tests' else
                        short + '-lib' + ('-list.stdout' if suffix == 'list' else '-ignored.stdout'))
            require(compact(lineage['base_streams'][key]) == compact(baseline['raw'][raw_name]),
                    'qualified historical stream pin')
    retain('lineage/rust-src.json', inputs['rust_src']['path'], inputs['rust_src'])
    selected_inputs[inputs['rust_src']['path']] = inputs['rust_src']
    require(selected_inputs == receipt['lineage_input_pins'], 'complete exact admission readset retained')

    before = json.loads(bodies['evidence/sources-before.json'])
    require(files['evidence/sources-before.json'] == receipt['input_sources']
            and {name: compact(row) for name, row in before.items()} == inputs['files'],
            'actual input source snapshot join')
    final_pin = receipt['final_sources']
    require(final_pin is not None and files['evidence/sources-after.json'] == final_pin,
            'recorded final source snapshot required for source-body retention')
    final = json.loads(bodies['evidence/sources-after.json'])
    require(receipt['source_unchanged'] == (final == before), 'truthful source preservation flag')
    baseline_sources = json.loads(bodies['lineage/base_sources.json'])
    expected_sources = {name: compact(row) for name, row in baseline_sources.items()
                        if name.startswith('fe2o3/')}
    require(len(expected_sources) == 5793, 'exact qualified baseline source census')
    source_names = set(lineage['diagnostic_overlay'])
    require(len(source_names) == 4, 'exact four diagnostic source bodies')
    for name, row in lineage['diagnostic_overlay'].items():
        require(row['before'] == expected_sources.get(name)
                and (name in expected_sources) == (row['before'] is not None),
                'exact diagnostic preimage or absent addition')
        expected_sources[name] = row['after']
    require(len(expected_sources) == 5795
            and {name: compact(row) for name, row in before.items() if name.startswith('fe2o3/')}
                == expected_sources, 'actual complete diagnostic-overlay source join')
    for name in sorted(source_names):
        retain(name, ROOT / name, final[name])
    retain('export_evidence.py', Path(__file__).resolve())
    manifest = dict(schema='ferric-guarded-mlp-ranked-cfg-diagnostic-cpu-retention-v1',
                    files=files, receipt=receipt_pin, input_manifest=input_pin,
                    passed=receipt['passed'], failure=receipt['failure'],
                    postcheck_errors=receipt['postcheck_errors'],
                    source_unchanged=receipt['source_unchanged'], source_lineage=lineage,
                    diagnostic_source_files=sorted(source_names), diagnostic_build=True,
                    diagnostic_changes_admission=False,
                    exported_binary_bodies=False, actual_artifact_metadata=receipt['artifacts'],
                    gpu_execution=False, guarded_candidate_hsaco_emitted=False,
                    numerical_acceptance=False, full_model_acceptance=False, performance_claim=False)
    bodies['retention-manifest.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    require(len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'final bounded retention roster')
    with ARCHIVE.open('xb') as raw_archive:
        with tarfile.open(fileobj=raw_archive, mode='w:gz') as archive:
            for name, body in sorted(bodies.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(body), 0o600, 0
                archive.addfile(member, io.BytesIO(body))
    require(all(pin(Path(row['path']))[1] == row for row in files.values()), 'source/readset drift after archive')
    require({p.name for p in evidence.iterdir()} == set(receipt['raw']) | {terminals[0].name},
            'evidence roster drift after archive')
    print(json.dumps(dict(archive=pin(ARCHIVE, MAX_TOTAL)[1], files=len(bodies),
                          source_body_bytes=sum(map(len, bodies.values())),
                          passed=receipt['passed'], receipt=receipt_pin)))


if __name__ == '__main__':
    main()

