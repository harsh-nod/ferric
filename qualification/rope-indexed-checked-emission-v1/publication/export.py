"""Data-only bounded courier for a successful linked RoPE emission."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import tarfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CASE = E / 'rope-indexed-checked-emission-v228-v1'
OWNER = E / 'rope-indexed-checked-emission-owner-v228-v1'
PACKAGE = E / 'p228-rope-indexed-checked-emission-v1'
PACKAGE_SHA = 'b46ad9dbfb7d4a9f922adc80a2862df47e3801e4b3acd9d455357cc959a15233'
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
PRODUCER = E / 'row-rope-materialized-rpo-checked-probe-v228-v1'
PRODUCER_OWNER = E / 'rope-materialized-rpo-checked-probe-owner-v228-v1'
PRODUCER_SHA = '1fbbbd9a8889e1b33b724b3f8911c4ecc1074fffe8fc29377f6840686375ba1f'
PRODUCER_OWNER_SHA = '8fb2f02cde719c08fd41535d59d39980d8c6328f27cf952770c481f5b48edbbf'
CONSUMER = E / 'kir-indexed-formal-join-cpu-v228-v2'
CONSUMER_OWNER = E / 'kir-indexed-formal-join-cpu-owner-v228-v2'
CONSUMER_SHA = '4548c7ee32206f6fc94bf54b5c50e13c500d2f4381e58092094893d9d80d4bfa'
CONSUMER_OWNER_SHA = '21205fbed2b56d5a27b4c1fbb2f4f3d5f0e810b822a518e3f875e3083553954e'
SOURCE_SHA = 'f16b193aa6a505ab2b28d98334e86b60ac5bcb6cfbd09d885cecd33d5cc27854'
PROPOSAL_SHA = 'd9d235a2f0bb2ae185c41f38d8406124c8233d3259e53751ccf47a17755b5dc2'
HANDOFF_SHA = 'ad4b31ee88efa54702dc8dff13e1e3c331c296734f7cdcfa52e8249ed9fa37fc'
OLD_PHASES = ('fixture-metadata', 'checked-lowering', 'actual-replay')
PHASES = ('metadata', 'tool-build', 'actual-inert-join', 'emit', 'extract-retained',
          'descriptor-metadata', 'elf-notes', 'disassembly')
ARTIFACTS = {'emitted/source.handoff-v3': 16 << 20, 'emitted/compiler.handoff-v2': 16 << 20,
    'emitted/artifact.hsaco': 32 << 20, 'emitted/receipt.txt': 64 << 10,
    'extracted/formal.archive': 16 << 20, 'extracted/module.ll': 16 << 20}
FALSE = ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_compiler_built',
    'full_compiler_cohort_requalified', 'gpu_execution', 'numerical_acceptance',
    'performance_claim', 'production_authority', 'launch_authority',
    'runtime_requirements_discharged', 'full_model_acceptance')
SELECTOR = 'linux::wave_qkv_attention_output_tiles_v6::tests::wave_emission_actual_retained_v6_passes_full_inert_join'
FINALIZER, METADATA = 'finite_join_engineering_hsaco_v1', 'finite_join_request_metadata_v1'
INNER = (197494, 'd45dd1a2ed5b88e767b14bbb8c7d2728211bb49aeaa14b1f8b4cc5a5b4f80c49')
OUTER = (31010, 'd9e4794a60a7d48daa953701ae764e51e99b70270310537c9f93d21afb8aa991')


def common(path):
    path = Path(path).absolute()
    if path.resolve(strict=True) != path or not path.is_file() or path.stat().st_size > 1 << 20:
        raise ValueError('canonical bounded data helper')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact common data helper')
    value = types.ModuleType('linked_emission_publication_data')
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    value.read(path, value.extent(raw))
    return value


def terminal(X, value, owner):
    X.success(value, 'ferric-p228-rope-indexed-checked-emission-result-v1')
    X.success(owner, 'ferric-p228-rope-indexed-checked-emission-owned-result-v1')
    X.require(all(v[k] is False for v in (value, owner) for k in FALSE)
        and value['fresh_consumer_tools_built'] is True and value['fresh_actual_inert_join_passed'] is True
        and value['fresh_hsaco_emitted'] is owner['fresh_hsaco_emitted'] is True
        and value['retained_checked_producer_replay_authenticated'] is True
        and value['producer_consumer_generations_distinct'] is True and value['source_unchanged'] is True
        and value['unresolved_runtime_requirements'] == 8, 'linked emission without fresh compiler/runtime authority')
    X.require(tuple(row['name'] for row in value['commands']) == PHASES and set(value['phases']) == set(PHASES)
        and set(value['tools']) == {'finalizer', 'metadata'} and set(value['artifacts']) == set(ARTIFACTS),
        'eight successful leaves, two consumer tools and six artifacts')
    for key in ('package', 'producer', 'producer_owner', 'consumer', 'consumer_owner'):
        X.require(value[key] == owner[key], 'owner/inner prerequisite identity')
    out = owner['owned']
    X.require(type(out['exit_code']) is int and out['exit_code'] == 0 and out['reason'] is None
        and out['cleanup_signalled'] is False and out['owned_groups_absent'] is True
        and out['owned_processes_reaped'] is True and out['deadline_seconds'] == 10800,
        'natural successful owned continuation')


def cargo_product(X, path, expected, cap=128 << 20):
    path = Path(path)
    allowed = {CONSUMER / 'target/debug/examples' / name for name in (
        FINALIZER, METADATA, 'finite_join_engineering_hsaco_v1-d165b82509d45fc9')}
    allowed.add(CONSUMER / 'target/debug/deps/fe2o3_lower_mir_kernel-a9319dfef712f0cf')
    X.require(path in allowed and X.pin(expected)['path'] == str(path)
        and path.resolve(strict=True) == path, 'only four pinned omitted Cargo executables')
    before = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size,
        s.st_uid, s.st_gid, s.st_mtime_ns, s.st_ctime_ns)
    X.require(stat.S_ISREG(before.st_mode) and before.st_uid == 9661 and before.st_nlink >= 1
        and before.st_mode & 0o111 and before.st_size <= cap, 'owned bounded executable, legitimate Cargo links')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as stream:
        X.require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened Cargo product identity')
        raw = stream.read(cap + 1)
        X.require(stamp(os.fstat(stream.fileno())) == stamp(before), 'stable open Cargo product')
    X.require(stamp(path.lstat()) == stamp(before) and len(raw) == before.st_size
        and X.extent(raw) == {key: expected[key] for key in ('bytes', 'sha256')}, 'stable exact Cargo product body')
    return raw


def roster(X, reader, complete_sha, owner_sha):
    X.require(X.digest(complete_sha) == INNER[1] and X.digest(owner_sha) == OUTER[1], 'actual successful terminal selection')
    records, omitted = {}, {}

    def add(path, expected=None, retain=True, cap=32 << 20):
        path = Path(path); name = str(X.relative(str(path.relative_to(E))))
        X.require(not retain or '/target/' not in '/' + name + '/', 'no tool/test executable in courier')
        raw = cargo_product(X, path, expected, cap) if not retain and path.is_relative_to(CONSUMER / 'target') else reader(path, expected, cap=cap)
        pin = dict(path=str(path), **X.extent(raw))
        collection = records if retain else omitted
        X.require(name not in collection or collection[name] == pin, 'one original identity')
        collection[name] = pin
        return raw

    def doc(path, expected=None, retain=True, cap=32 << 20):
        return X.parse(add(path, expected, retain, cap))

    value = doc(CASE / 'complete.json', dict(zip(('bytes', 'sha256'), INNER)))
    owner = doc(OWNER / 'complete.json', dict(zip(('bytes', 'sha256'), OUTER)))
    X.require(records[str((CASE / 'complete.json').relative_to(E))]['sha256'] == complete_sha
        and records[str((OWNER / 'complete.json').relative_to(E))]['sha256'] == owner_sha
        and owner['completion'] == records[str((CASE / 'complete.json').relative_to(E))], 'actual terminal digests')
    terminal(X, value, owner)
    raw_names = {name + '-' + suffix for name in PHASES for suffix in X.SUFFIXES}
    X.require(set(value['raw']) == raw_names | {'before.json', 'after.json', 'inputs.json'}, 'closed 43-member raw roster')
    for name, pin in value['raw'].items():
        X.require(X.pin(pin)['path'] == str(CASE / name), 'raw original namespace')
        add(pin['path'], pin, name not in ('before.json', 'after.json'), cap=128 << 20)
    for row in value['commands']:
        name = row['name']; phase = value['phases'][name]
        X.require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
            and phase['group_absent'] is True and doc(CASE / (name + '-result.json')) == phase,
            'every recorded phase naturally succeeds')
        for key, suffix in zip(('command', 'started', 'result', 'stdout', 'stderr'), X.SUFFIXES):
            X.require(row[key] == value['raw'][name + '-' + suffix], 'indexed/raw phase identity')
        X.require(all(phase[key + '_sha256'] == row[key]['sha256'] for key in ('stdout', 'stderr')),
            'phase output hashes')
    for name in sorted(X.OWNER_FILES):
        add(OWNER / name, retain=name not in ('after.json', 'old-targets-before.json'), cap=128 << 20)
    X.require(doc(OWNER / 'owned-result.json') == owner['owned'], 'same owned result body')
    before = doc(CASE / 'before.json', value['raw']['before.json'], False, 128 << 20)
    after = doc(CASE / 'after.json', value['raw']['after.json'], False, 128 << 20)
    for old, new in (('producer_inputs', 'producer-inputs'), ('consumer_sources', 'consumer-sources'),
                     ('consumer_dependencies', 'consumer-dependencies'), ('configurations', 'configurations')):
        X.require(before[old] == after[new], 'recorded unchanged input map: ' + old)
    owner_after = doc(OWNER / 'after.json', retain=False, cap=128 << 20)
    X.require(doc(OWNER / 'old-targets-before.json', retain=False, cap=128 << 20) == owner_after['old-targets']
        and owner_after['consumer-source'] == before['consumer_sources'], 'recorded old targets and consumer source preserved')
    X.require(value['package']['path'] == str(PACKAGE / 'manifest.json')
        and value['package']['sha256'] == PACKAGE_SHA, 'frozen emission package')
    package = doc(value['package']['path'], value['package'])
    X.require(package['schema'] == 'ferric-p228-rope-indexed-checked-emission-package-v1'
        and len(package['files']) == 4 and {row['path'] for row in package['files']}
        == {'run.py', 'contracts.py', 'test_run.py', 'README.md'} and package['pure_tests'] == 16,
        'closed four-source package and authored census')
    for row in package['files']: add(PACKAGE / X.relative(row['path']), row)
    for key, root, leaf, sha in (('producer', PRODUCER, 'failed.json', PRODUCER_SHA),
        ('producer_owner', PRODUCER_OWNER, 'failed.json', PRODUCER_OWNER_SHA),
        ('consumer', CONSUMER, 'complete.json', CONSUMER_SHA),
        ('consumer_owner', CONSUMER_OWNER, 'complete.json', CONSUMER_OWNER_SHA)):
        X.require(X.pin(value[key])['path'] == str(root / leaf) and value[key]['sha256'] == sha, 'actual ' + key)
        add(value[key]['path'], value[key])
    producer = doc(value['producer']['path'], value['producer'])
    old_owner = doc(value['producer_owner']['path'], value['producer_owner'])
    consumer = doc(value['consumer']['path'], value['consumer'])
    consumer_owner = doc(value['consumer_owner']['path'], value['consumer_owner'])
    X.require(producer['passed'] is old_owner['passed'] is False
        and producer['error'] == 'AssertionError: actual-inert-join'
        and producer['postcheck_errors'] == old_owner['postcheck_errors'] == []
        and producer['commands'] == value['producer_commands']
        and tuple(row['name'] for row in producer['commands']) == OLD_PHASES
        and producer['compiler_generation'] == value['producer_generation']
        and producer['artifacts'] == value['producer_captures'], 'unchanged failed producer and three historical successes')
    X.success(consumer, 'ferric-p228-kir-indexed-formal-join-cpu-result-v2')
    X.success(consumer_owner, 'ferric-p228-kir-indexed-formal-join-owned-result-v2')
    X.require(consumer_owner['completion'] == value['consumer']
        and consumer['raw']['sources-before.json'] == value['consumer_source']
        and value['consumer_source']['sha256'] == SOURCE_SHA
        and consumer['proposal'] == value['consumer_proposal'] and value['consumer_proposal']['sha256'] == PROPOSAL_SHA
        and consumer['tests'] == value['consumer_tests']
        and consumer['artifacts'] == value['preserved_consumer_test_artifacts'], 'qualified consumer generation and products')
    sources = doc(value['consumer_source']['path'], value['consumer_source'], False)
    X.require(len(sources) == 5785 and sources == before['consumer_sources'], 'exact recorded consumer source map')
    for name, key in (('dependencies-before.json', 'consumer_dependencies'), ('configurations-before.json', 'configurations')):
        X.require(doc(consumer['raw'][name]['path'], consumer['raw'][name], False) == before[key], 'qualified consumer map: ' + name)
    proposal = doc(value['consumer_proposal']['path'], value['consumer_proposal'])
    X.require(len(proposal['files']) == 5 and set(consumer['formatted_sources']) == {r['path'] for r in proposal['files']},
        'five qualified formatted consumer bodies')
    for name, pin in consumer['formatted_sources'].items():
        X.require(pin == sources[str(CONSUMER / 'source/fe2o3' / X.relative(name))]['pin'], 'formatted source snapshot identity')
        add(pin['path'], pin)
    for name in ('metadata-stdout', 'finalizer-build-tests-command.json'):
        add(consumer['raw'][name]['path'], consumer['raw'][name])
    for pin in (*value['tools'].values(), *value['preserved_consumer_test_artifacts'].values()):
        X.require(Path(X.pin(pin)['path']).is_relative_to(CONSUMER / 'target'), 'consumer target product namespace')
        add(pin['path'], pin, False, 128 << 20)
    recipe = doc(value['producer_recipe']['path'], value['producer_recipe'])
    X.require(value['producer_recipe']['path'] == str(PRODUCER / 'recipe.json')
        and tuple(row['name'] for row in recipe['commands']) == OLD_PHASES + PHASES[2:], 'original nine-stage recipe')
    for old in producer['commands']:
        for key, suffix in zip(('command', 'started', 'result', 'stdout', 'stderr'), X.SUFFIXES):
            X.require(X.pin(old[key])['path'] == str(PRODUCER / (old['name'] + '-' + suffix)), 'historical leaf path')
            add(old[key]['path'], old[key])
    for pin in producer['artifacts'].values(): add(pin['path'], X.pin(pin), False)
    X.require(value['retained_handoff'] == producer['artifacts']['prefix-tiles.handoff-v3']
        == consumer['retained_handoff'] and value['retained_handoff']['bytes'] == 4078537
        and value['retained_handoff']['sha256'] == HANDOFF_SHA, 'same producer/consumer handoff')
    cpu = doc(producer['candidate_cpu']['path'], producer['candidate_cpu'])
    doc(producer['source_manifest']['path'], producer['source_manifest'])
    fixture = {**cpu['lowering_sources'], **cpu['lowering_fixture_pins']}
    X.require(len(fixture) == 10 and {r['destination']: r['source'] for r in recipe['fixture']} == fixture,
        'original ten-file RoPE arithmetic fixture')
    for name, pin in fixture.items(): add(PRODUCER / 'fixture' / X.relative(name), pin)
    for name, pin in value['artifacts'].items():
        X.require(X.pin(pin)['path'] == str(CASE / X.relative(name)), 'new artifact namespace')
        add(pin['path'], pin, cap=ARTIFACTS[name])
    X.require(all(value['artifacts']['emitted/source.handoff-v3'][key] == value['retained_handoff'][key]
        for key in ('bytes', 'sha256')), 'emitted source handoff is the retained producer body')
    X.require(len(records) <= 112 and len(omitted) <= 20
        and sum(pin['bytes'] for pin in records.values()) <= 128 << 20, 'bounded source/metadata/artifact courier')
    return value, owner, producer, consumer, recipe, package, records, omitted


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('complete_sha256'); parser.add_argument('owner_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper)
    X.require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python -B')
    out = args.archive.absolute()
    X.require(out.parent == E and re.fullmatch(r'rope-indexed-emission-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
        and not os.path.lexists(out), 'fresh bounded archive namespace')
    for kind, cap in ((resource.RLIMIT_AS, 1 << 30), (resource.RLIMIT_CPU, 180),
                      (resource.RLIMIT_FSIZE, 96 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        resource.setrlimit(kind, (cap if hard == resource.RLIM_INFINITY else min(cap, hard),) * 2)
    *_, records, omitted = roster(X, X.read, args.complete_sha256, args.owner_sha256)
    manifest = dict(schema='ferric-p228-rope-indexed-emission-export-v1', original_root=str(E),
        complete_sha256=args.complete_sha256, owner_sha256=args.owner_sha256, files=records,
        remotely_rehashed_omitted_pins=omitted, transitive_source_bodies_rehashed=False,
        tool_executables_exported=False, full_source_maps_exported=False, owner_inventories_exported=False,
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(X.read(Path(__file__).resolve()))),
        data_helper=dict(path=str(args.common_helper.absolute()), **X.extent(X.read(args.common_helper.absolute()))))
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, pin in sorted(records.items()):
            raw = X.read(Path(pin['path']), pin, cap=128 << 20)
            member = tarfile.TarInfo(name); member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json'); member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for pin in (*records.values(), *omitted.values()):
        path = Path(pin['path'])
        if path.is_relative_to(CONSUMER / 'target'):
            cargo_product(X, path, pin)
        else:
            X.read(path, pin, cap=128 << 20)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out, cap=96 << 20))),
        files=len(records), omitted=len(omitted), retained_bytes=sum(pin['bytes'] for pin in records.values()))))


if __name__ == '__main__':
    main()
