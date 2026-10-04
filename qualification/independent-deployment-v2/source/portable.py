"""Data-only custody for the actual V7 image and independent native generation."""
import hashlib
import os
from pathlib import Path
import re
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PUBLIC = Path('/home/harsh/ferric-p227-integration/qualification')
SCHEMA = 'ferric-p228-independent-deployment-v1'
ROW = 'row-reciprocal-checked-probe-v228-v7'
OWNER = 'reciprocal-checked-probe-owner-v228-v6'
NATIVE = 'independent-native-profile-cpu-v228-v1'
NATIVE_OWNER = 'independent-native-profile-cpu-owner-v228-v1'
STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join',
          'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
NATIVE_STAGES = ('metadata', 'example-build-tests', 'example-list',
                 'example-ignored-list', 'example-tests', 'example-build')
NONCLAIMS = ('gpu_execution', 'numerical_acceptance', 'production_authority',
             'launch_authority', 'performance_claim', 'full_model_acceptance')
MAX_ARCHIVE = 64 << 20
MAX_MEMBER = 32 << 20
MAX_TOTAL = 256 << 20
MAX_RETAINED = 128 << 20


def require(value, message):
    if not value:
        raise RuntimeError(message)


def fp(path, size, digest):
    return dict(path=str(path), bytes=size, sha256=digest)


ARCHIVES = {
    'compiler': fp(E / 'reciprocal-checked-probe-evidence-v228-v7.tar.gz', 8938596,
        'cfe073463cf3512ba7a6fb7239d0b9cdebbd0346ae0b177d776e6d94412b23a7'),
    'native': fp(E / 'independent-native-profile-cpu-evidence-v228-v1.tar.gz', 33725202,
        'e097001bf737d7b85e752ee4e3580bc702a2df9521abbb366d1ab355045d16a2'),
}
QUALIFICATIONS = {
    'compiler': fp(PUBLIC / 'fe2o3-ordinary-induction-alias-v1/checked-probe-result.json', 32235,
        '128cddec9501f363eab39a00da06752cd43e84ab0680cd99321a8f70addb2366'),
    'native': fp(PUBLIC / 'independent-native-profiles-v1/cpu-result.json', 8569,
        'edc59f8559a2c1f9a7fa6bab91c1474be91d6fb82b956f00a93c8a7742f976bb'),
}
COMPLETIONS = {
    'compiler': fp(E / ROW / 'complete.json', 25107,
        '6ba25826b71e30f5106e1efc9e786c021c56f5bab397add20b365373685081a9'),
    'compiler_owner': fp(E / OWNER / 'complete.json', 24697,
        '8d55d89d06fb1a3a6966c25ed52e3f5274b6c69b6cef9fffcb2363929305cb62'),
    'native': fp(E / NATIVE / 'complete.json', 30587,
        '40fc164c535628fb3b1a4f7ee929e9717f57e57b18e7f45dcec2b87d3a8dcde1'),
    'native_owner': fp(E / NATIVE_OWNER / 'complete.json', 46773,
        '5d5594e1dc74f2e678ee2d2117cbd3c509382281dcfb1394b213b3ce25c24e89'),
}
IMAGE = fp(E / ROW / 'emitted/artifact.hsaco', 53560,
    '4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5')
BINARY = fp(E / NATIVE / 'target/debug/examples/gfx950-qwen-prefix-tiles-comparison-v6', 11759152,
    '017e0d3a5c79b1c64a4dccdb0251e53c8e7f8c17ab47e92bbe1309b40760c607')
FILENAMES = dict(compiler_archive='evidence/compiler.tar.gz', native_archive='evidence/native.tar.gz',
    compiler_qualification='qualification/compiler.json', native_qualification='qualification/native.json',
    image='image/prefix-tiles.hsaco', binary='gfx950-qwen-prefix-tiles-comparison-v6')


def pin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}, 'closed FilePin')
    path = Path(value['path'])
    require(type(value['path']) is str and path.is_absolute() and str(path) == value['path']
        and '..' not in path.parts and type(value['bytes']) is int and 0 <= value['bytes'] <= 1 << 30
        and type(value['sha256']) is str and re.fullmatch('[a-f0-9]{64}', value['sha256']), 'FilePin fields')
    return value


def body_pin(value):
    return {name: pin(value)[name] for name in ('bytes', 'sha256')}


def identity(value):
    return value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns


def directory_name(directory):
    require(directory.is_absolute() and '..' not in directory.parts
        and re.fullmatch(r'prefix-independent-deployment-v228-v[1-9][0-9]*', directory.name),
        'new independent deployment namespace')


def relocation(value, original, directory, filename):
    require(type(value) is dict and set(value) == {'original', 'transported'}, 'explicit relocation')
    require(pin(value['original']) == original and body_pin(value['transported']) == body_pin(original)
        and value['transported']['path'] == str(directory / filename), 'exact original/transported join')
    return value['transported']


class Archive:
    """Hash every member without extracting paths or executing archived code."""
    def __init__(self, D, pins, record, count):
        self.D, self.record, self.members, self.data = D, pin(record), {}, {}
        path = Path(record['path'])
        observed, _ = pins.read(path, record['sha256'], False, MAX_ARCHIVE)
        require(observed == record, 'actual transported archive extent')
        require(path.resolve(strict=True) == path and path.is_file(), 'canonical archive')
        total = retained = 0
        with path.open('rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode), 'regular archive')
            with tarfile.open(fileobj=stream, mode='r|gz') as archive:
                for member in archive:
                    relative = Path(member.name)
                    require(member.isfile() and not member.linkname and not relative.is_absolute()
                        and str(relative) == member.name and '..' not in relative.parts
                        and relative.parts and member.name not in self.members, 'safe unique regular archive member')
                    require(len(self.members) < count and type(member.size) is int
                        and 0 <= member.size <= MAX_MEMBER, 'bounded archive member')
                    total += member.size
                    require(total <= MAX_TOTAL, 'bounded expanded archive')
                    keep = ('/source/fe2o3/' not in member.name and '/target/' not in member.name
                        and (member.name.endswith(('.json', '-stdout', '-stderr', '/receipt.txt'))
                             or relative.name in ('stdout', 'stderr', 'tests.log')))
                    digest, size, chunks = hashlib.sha256(), 0, []
                    with archive.extractfile(member) as source:
                        while block := source.read(1 << 20):
                            size += len(block)
                            require(size <= member.size, 'member grew while reading')
                            digest.update(block)
                            if keep:
                                retained += len(block)
                                require(retained <= MAX_RETAINED, 'bounded retained evidence')
                                chunks.append(block)
                    require(size == member.size, 'complete member body')
                    self.members[member.name] = fp(E / relative, size, digest.hexdigest())
                    if keep:
                        self.data[member.name] = b''.join(chunks)
            after = os.fstat(stream.fileno())
        require(identity(before) == identity(after) == identity(path.lstat()), 'archive changed while scanning')
        require(len(self.members) == count, 'exact complete archive member census')
        require(pins.read(path, record['sha256'], False, MAX_ARCHIVE)[0] == record, 'archive final hash')

    def member(self, path):
        path = Path(path)
        require(path.is_relative_to(E), 'original evidence namespace')
        name = str(path.relative_to(E))
        require(name in self.members, 'required archived member: ' + name)
        return self.members[name]

    def check(self, record):
        require(self.member(pin(record)['path']) == record, 'archived original file identity')
        return record

    def raw(self, record):
        self.check(record)
        name = str(Path(record['path']).relative_to(E))
        require(name in self.data, 'only retained evidence text may be decoded')
        return self.data[name]

    def doc(self, record):
        return self.D.parse(self.raw(record))

    def named(self, path):
        return self.doc(self.member(path))


def terminal(owner):
    require(owner['passed'] is True and owner['error'] is None and owner['postcheck_errors'] == [],
            'successful original owner')
    outcome = owner['owned']
    require(type(outcome['exit_code']) is int and outcome['exit_code'] == 0 and outcome['reason'] is None
        and outcome['cleanup_signalled'] is False and outcome['owned_groups_absent'] is True
        and outcome['owned_processes_reaped'] is True, 'original natural exit and complete reap')


def phase(archive, root, name, records, expected):
    for key, suffix in (('command', 'command.json'), ('started', 'started.json'), ('result', 'result.json'),
                        ('stdout', 'stdout'), ('stderr', 'stderr')):
        require(records[key]['path'] == str(root / (name + '-' + suffix)), 'phase original path')
        archive.check(records[key])
    command, started, result = [archive.doc(records[key]) for key in ('command', 'started', 'result')]
    require(command == expected and command['cache_cap_bytes'] == 6 << 30
        and command['affinity'] == [8, 9] and command['nice'] == 10
        and command['gpu_execution'] is False and type(command['expected_exit']) is int
        and command['expected_exit'] == 0, 'exact original CPU phase command')
    require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
        and started['pid'] > 0 and started['pgid'] == started['pid'], 'original owned phase group')
    require(type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
        and result['group_absent'] is True and type(result['cache_bytes']) is int
        and 0 <= result['cache_bytes'] <= 6 << 30, 'original natural CPU phase outcome')
    for stream in ('stdout', 'stderr'):
        require(result[stream + '_sha256'] == records[stream]['sha256'], 'phase actual stream join')
    return result


def substitute(command, bindings):
    def value(item):
        if type(item) is str and item.startswith('@candidate.'):
            require(item in bindings, 'candidate artifact binding required')
            return str(bindings[item])
        return item
    result = dict(command)
    result['argv'] = [value(item) for item in command['argv']]
    result['env'] = {key: value(item) for key, item in command['env'].items()}
    require(all(type(item) is str for item in result['argv'] + list(result['env'].values())),
            'original command argv and environment are strings')
    return result


def verify_compiler(archive, qualification):
    c, owner = archive.doc(COMPLETIONS['compiler']), archive.doc(COMPLETIONS['compiler_owner'])
    require(qualification['schema'] == 'FerricReciprocalCheckedProbeQualificationV1'
        and qualification['passed'] is True and c['schema'] == 'ferric-p228-reciprocal-checked-probe-result-v7'
        and owner['schema'] == 'ferric-p228-reciprocal-owned-probe-result-v6', 'actual V7 generation')
    terminal(owner)
    require(owner['inner_completion'] == COMPLETIONS['compiler'] and c['probe_completed'] is True
        and c['error'] is None and c['postcheck_error'] is None, 'whole V7 completion')
    for key in ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted'):
        require(c[key] is True and qualification[key] is True, 'completed fresh checked stage')
    for key in NONCLAIMS:
        require(c[key] is False and qualification[key] is False, 'no compiler runtime authority')
    require(c['unresolved_runtime_requirements'] == qualification['unresolved_runtime_requirements'] == 8
        and c['frontend_recipe_is_diagnostic'] is True and c['runtime_requirements_discharged'] is False
        and c['isa_review_accepted'] is False and c['old_cpu_cohorts_are_candidate_qualification'] is False
        and qualification['old_cpu_cohorts_are_candidate_qualification'] is False, 'unchanged engineering proof boundary')
    require(qualification['receipts']['probe'] == body_pin(COMPLETIONS['compiler'])
        and qualification['receipts']['owner'] == body_pin(COMPLETIONS['compiler_owner'])
        and qualification['receipts']['retained_archive'] == body_pin(ARCHIVES['compiler']), 'compiler summary receipt joins')
    for key, public_key in (('recipe', 'recipe'), ('inputs', 'inputs'), ('compiler_generation', 'compiler_generation'),
        ('candidate_cpu_receipt', 'candidate_cpu'), ('candidate_cpu_sources', 'candidate_cpu_sources'),
        ('artifacts', 'artifacts'), ('commands', 'phase_records')):
        require(c[key] == qualification[public_key], 'root-verified compiler summary join: ' + key)
    require(owner['compiler_generation'] == c['compiler_generation']
        and owner['candidate_cpu_receipt'] == c['candidate_cpu_receipt']
        and owner['candidate_cpu_sources'] == c['candidate_cpu_sources'], 'outer compiler/candidate provenance')
    for record in c['artifacts'].values():
        archive.check(record)
    require(len(c['artifacts']) == 10 and c['artifacts']['emitted/artifact.hsaco'] == IMAGE, 'actual V7 image')
    recipe = archive.doc(c['recipe'])
    inputs = archive.doc(c['inputs'])
    require(recipe['candidate_cpu_receipt'] == c['candidate_cpu_receipt']
        and recipe['fresh_output'] == str(E / ROW)
        and inputs['schema'] == 'ferric-p228-reciprocal-checked-probe-inputs-v7', 'actual candidate recipe/input generation')
    bindings = {'@candidate.semantic.sha256': c['artifacts']['prefix-tiles-semantic.bin']['sha256'],
        '@candidate.handoff.sha256': c['artifacts']['prefix-tiles.handoff-v3']['sha256'],
        '@candidate.handoff.bytes': c['artifacts']['prefix-tiles.handoff-v3']['bytes'],
        '@candidate.image.sha256': IMAGE['sha256'], '@candidate.image.bytes': IMAGE['bytes']}
    require(tuple(row['name'] for row in c['commands']) == STAGES
        and tuple(row['name'] for row in recipe['commands']) == STAGES, 'all nine actual compiler phases')
    for row, template in zip(c['commands'], recipe['commands']):
        expected = {key: value for key, value in substitute(template, bindings).items() if key not in ('name', 'cwd')}
        require(phase(archive, E / ROW, row['name'], row, expected) == qualification['phases'][row['name']],
                'compiler phase qualification join')
    for name in ('actual-replay', 'actual-inert-join'):
        row = next(row for row in c['commands'] if row['name'] == name)
        selector = archive.doc(row['command'])['argv'][2]
        text = archive.raw(row['stdout']).decode('utf-8')
        require(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ok$', text, re.M) == [selector]
            and 'test result: ok. 1 passed; 0 failed; 0 ignored;' in text
            and row['stderr']['bytes'] == 0, 'actual named ignored callback passed')
    replay = archive.raw(archive.member(E / ROW / 'actual-replay-stdout'))
    require(b'stage=outer-lineage-replay status=complete formal_bytes=858689' in replay, 'complete actual lineage replay')
    for record in owner['raw'].values():
        archive.check(record)
    require(archive.named(E / OWNER / 'owned-result.json') == owner['owned'], 'original owned compiler outcome')
    before, after = [archive.named(E / ROW / (side + '.json')) for side in ('before', 'after')]
    require(before['inputs'] == c['inputs'] and before['recipe'] == c['recipe'], 'original compiler input snapshot join')
    for key in ('files', 'fixture', 'configurations'):
        require(before[key] == after[key], 'compiler postcheck: ' + key)
    for row in before['fixture'].values():
        archive.check(row['pin'])
    require(set(before['fixture']) == {str(E / ROW / 'fixture' / row['destination']) for row in recipe['fixture']},
            'closed nine-file actual compiler fixture')
    for row in recipe['fixture']:
        require(body_pin(before['fixture'][str(E / ROW / 'fixture' / row['destination'])]['pin'])
            == body_pin(row['source']), 'compiled fixture and candidate source body join')
    own_before, own_after = [archive.named(E / OWNER / (side + '.json')) for side in ('before', 'after')]
    for key in ('sources_and_tools', 'source_maps', 'configurations', 'old_targets', 'compiler_source_roster'):
        require(own_before[key] == own_after[key], 'compiler owner postcheck: ' + key)
    require(len(own_before['compiler_source_roster']) == 5781 and len(own_before['old_targets']) == 5,
            'original compiler source and old-target rosters')
    require(body_pin(archive.member(E / ROW / 'package-sources-before.json'))
        == body_pin(archive.member(E / ROW / 'package-sources-after.json')), 'compiler dependencies/sysroot postcheck')
    metadata = archive.raw(archive.member(E / ROW / 'descriptor-metadata-stdout')).decode('ascii').splitlines()
    require(metadata[0] == 'fe2o3-finite-join-request-metadata-v1', 'actual metadata format')
    fields = [line.split(' ', 1) for line in metadata[1:]]
    require(all(len(row) == 2 for row in fields) and len(dict(fields)) == len(fields)
        and dict(fields) == qualification['descriptor_metadata'], 'actual metadata/public summary equality')
    return c


def relative_sources(values, root):
    result = {}
    for name, row in values.items():
        path = Path(name)
        require(path.is_relative_to(root) and row['pin']['path'] == name, 'source snapshot original namespace')
        result[str(path.relative_to(root))] = body_pin(row['pin'])
    return result


def overlay_sources(previous, overlay):
    previous = dict(previous)
    for row in overlay['required_preimages']:
        require(previous.get(row['path']) == {key: row[key] for key in ('bytes', 'sha256')}, 'overlay exact preimage')
    for name in overlay['required_absent_paths']:
        require(name not in previous, 'overlay required absence')
    for row in overlay['files']:
        if row['path'].startswith('source/'):
            previous[str(Path(overlay['source_directory']) / Path(row['path']).name)] = {
                key: row[key] for key in ('bytes', 'sha256')}
    return previous


def native_test_result(listed, result, required):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', listed, re.M)
    actual = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored(?:, [^\n]*)?)$', result, re.M)
    require(len(names) == len(set(names)) == 99 and len(actual) == 99 and {row[0] for row in actual} == set(names)
        and sum(row[1] == 'ok' for row in actual) == 98 and sum(row[1].startswith('ignored') for row in actual) == 1
        and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', result) == [('98', '0', '1')],
        'actual complete named native test result')
    require(len(required) == len(set(required)) == 14 and all((name, 'ok') in actual for name in required),
            'all fourteen new native regressions actually passed')


def verify_native(archive, qualification, compiler):
    n, owner = archive.doc(COMPLETIONS['native']), archive.doc(COMPLETIONS['native_owner'])
    require(qualification['schema'] == 'FerricIndependentNativeProfilesCpuQualificationV1'
        and qualification['passed'] is True and n['schema'] == 'fe2o3-p228-independent-native-profile-cpu-result-v1'
        and owner['schema'] == 'fe2o3-p228-independent-native-profile-cpu-owned-result-v1', 'actual independent native generation')
    terminal(owner)
    require(n['passed'] is True and n['error'] is None and n['postcheck_errors'] == []
        and n['fresh_native_example_built'] is True and owner['completion'] == COMPLETIONS['native'], 'whole native completion')
    for key in ('gpu_execution', 'numerical_acceptance', 'production_authority', 'performance_claim',
                'full_model_acceptance', 'checked_lowering', 'fresh_hsaco_emitted', 'device_opened'):
        require(n[key] is False and qualification[key] is False, 'native qualification is CPU only')
    require(qualification['raw_completion'] == COMPLETIONS['native']
        and qualification['owner_completion'] == body_pin(COMPLETIONS['native_owner'])
        and qualification['retained_archive'] == body_pin(ARCHIVES['native']), 'native qualification receipt joins')
    for key, summary_key in (('artifacts', 'artifacts'), ('qualified_generation', 'compiler_generation'),
        ('overlay', 'overlay'), ('roundtrip', 'roundtrip'), ('package', 'package'),
        ('required_test_names', 'new_tests'), ('phases', 'phases')):
        require(n[key] == qualification[summary_key], 'native root-verified qualification join: ' + key)
    generation = compiler['compiler_generation']
    require(n['qualified_generation']['completion'] == generation['prerequisites']['cpu']
        and n['qualified_generation']['owner'] == generation['prerequisites']['cpu_owner']
        and owner['qualified_generation'] == n['qualified_generation']['completion'], 'same qualified compiler generation')
    for record in n['raw'].values():
        archive.check(record)
    for record in n['artifacts'].values():
        archive.check(record)
    require(set(n['artifacts']) == {'example', 'example-tests'} and n['artifacts']['example'] == BINARY,
            'new actual native binary, not historical paired executable')
    require(set(n['phases']) == set(NATIVE_STAGES), 'all six native CPU phases')
    for name in NATIVE_STAGES:
        records = {key: n['raw'][name + '-' + suffix] for key, suffix in
            (('command', 'command.json'), ('started', 'started.json'), ('result', 'result.json'),
             ('stdout', 'stdout'), ('stderr', 'stderr'))}
        # The immutable original completion pins this exact command; no build-host tool is opened.
        expected = archive.doc(records['command'])
        require(phase(archive, E / NATIVE, name, records, expected) == n['phases'][name], 'native phase receipt equality')
    for name in ('original', 'sources', 'inputs', 'fixtures', 'configurations', 'dependencies'):
        require(body_pin(archive.member(E / NATIVE / (name + '-before.json')))
            == body_pin(archive.member(E / NATIVE / (name + '-after.json'))), 'native postcheck: ' + name)
    require(archive.named(E / NATIVE_OWNER / 'owned-result.json') == owner['owned'], 'native owned result')
    old_targets = archive.named(E / NATIVE_OWNER / 'old-targets-before.json')
    require(len(old_targets) == 6 and old_targets == archive.named(E / NATIVE_OWNER / 'after.json')['old_targets'],
            'six original native old-target postchecks')
    source_pin = archive.member(E / NATIVE / 'sources-before.json')
    sources = archive.doc(source_pin)
    require(len(sources) == n['source_members_after'] == qualification['sources_rehashed'] == 5783,
            'full actual native source census')
    source_root = E / NATIVE / 'source/fe2o3'
    require({name for name in archive.members if name.startswith(str(source_root.relative_to(E)) + '/')}
        == {str(Path(name).relative_to(E)) for name in sources}, 'closed full native source archive roster')
    for row in sources.values():
        archive.check(row['pin'])
    original = archive.member(E / NATIVE / 'original-before.json')
    require(body_pin(original) == body_pin(n['qualified_generation']['sources']), 'exact previously qualified source snapshot')
    previous = relative_sources(archive.doc(original), Path(generation['source']))
    require(len(previous) == n['source_members_before'] == 5781, 'qualified source census before native overlay')
    overlay = archive.doc(n['overlay'])
    require(overlay['schema'] == 'fe2o3-p228-independent-native-profile-proposal-v1'
        and overlay['authored_tests'] == 14 and owner['overlay'] == n['overlay'], 'exact native source overlay')
    overlay_root = Path(n['overlay']['path']).parent
    for row in overlay['files']:
        archive.check(dict(row, path=str(overlay_root / row['path'])))
    previous = overlay_sources(previous, overlay)
    require(previous == relative_sources(sources, source_root), 'only the qualified three-file native overlay applied')
    require(owner['roundtrip'] == n['roundtrip'], 'native patch roundtrip pin')
    archive.check(n['roundtrip'])
    package = archive.doc(n['package'])
    for row in package['files']:
        archive.check(dict(row, path=str(Path(n['package']['path']).parent / row['path'])))
    listed = archive.raw(n['raw']['example-list-stdout']).decode()
    result = archive.raw(n['raw']['example-tests-stdout']).decode()
    native_test_result(listed, result, n['required_test_names'])
    return n


def evidence(D, compiler_archive, native_archive, compiler_summary, native_summary):
    c = verify_compiler(compiler_archive, compiler_summary)
    n = verify_native(native_archive, native_summary, c)
    return dict(metadata=compiler_summary['descriptor_metadata'], original_image=IMAGE, original_binary=BINARY,
        compiler_receipt=COMPLETIONS['compiler'], compiler_owner=COMPLETIONS['compiler_owner'],
        native_receipt=COMPLETIONS['native'], native_owner=COMPLETIONS['native_owner'],
        compiler_generation=c['compiler_generation'], candidate_cpu_receipt=c['candidate_cpu_receipt'],
        candidate_cpu_sources=c['candidate_cpu_sources'], native_compiler_generation=n['qualified_generation'],
        compiler_artifacts=c['artifacts'], compiler_phase_records=c['commands'],
        candidate_sources=compiler_archive.doc(c['recipe'])['fixture'],
        arithmetic_evidence={name: compiler_archive.member(E / ROW / relative) for name, relative in (
            ('source_entry', 'fixture/src/lib.rs'),
            ('reciprocal_source', 'fixture/src/prefix_reciprocal_numerics_v1.rs'),
            ('llvm', 'extracted/module.ll'), ('isa', 'disassembly-stdout'), ('elf_notes', 'elf-notes-stdout'))},
        native_overlay=n['overlay'], native_sources=dict(files=5783,
            before=n['raw']['sources-before.json'], after=n['raw']['sources-after.json']),
        authority='none', **{key: False for key in NONCLAIMS})


def deployment_header(value, directory):
    directory_name(directory)
    require(type(value) is dict and set(value) == {'schema', 'directory', 'archives', 'qualifications',
        'image', 'binary', 'authority', *NONCLAIMS} and value['schema'] == SCHEMA
        and value['directory'] == str(directory) and value['authority'] == 'none'
        and all(value[key] is False for key in NONCLAIMS), 'closed non-authoritative deployment')
    require(set(value['archives']) == set(value['qualifications']) == {'compiler', 'native'}, 'two evidence generations')


def verify(D, pins, value, directory):
    """Verify transported bytes only. Original absolute paths are provenance, not IO."""
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'unoptimized verifier required')
    directory = Path(directory)
    deployment_header(value, directory)
    require(directory.resolve(strict=True) == directory and directory.is_dir(), 'canonical deployment directory')
    archives, summaries = {}, {}
    for role, count in (('compiler', 92), ('native', 5856)):
        record = relocation(value['archives'][role], ARCHIVES[role], directory, FILENAMES[role + '_archive'])
        archives[role] = Archive(D, pins, record, count)
        record = relocation(value['qualifications'][role], QUALIFICATIONS[role], directory, FILENAMES[role + '_qualification'])
        actual, raw = pins.read(Path(record['path']), record['sha256'], True, 1 << 20)
        require(actual == record, 'pinned root-verified public qualification')
        summaries[role] = D.parse(raw)
    verified = evidence(D, archives['compiler'], archives['native'], summaries['compiler'], summaries['native'])
    for name, original in (('image', IMAGE), ('binary', BINARY)):
        record = relocation(value[name], original, directory, FILENAMES[name])
        require(pins.read(Path(record['path']), record['sha256'], False, MAX_MEMBER)[0] == record,
                'actual deployed ' + name)
        verified[name] = record
    verified.update(qualifications=value['qualifications'], archive_pins=value['archives'])
    expected = {str(directory / name) for name in FILENAMES.values()} | {str(directory / 'deployment.json')}
    observed = set()
    for root, dirs, files in os.walk(directory, followlinks=False):
        require(all(not (Path(root) / name).is_symlink() for name in [*dirs, *files]), 'no deployment aliases')
        for name in files:
            path = Path(root) / name
            require(path.is_file() and path.resolve(strict=True) == path, 'regular deployment member')
            observed.add(str(path))
    require(observed == expected, 'closed portable seven-file directory')
    require(D.parse(pins.read(directory / 'deployment.json', None, True, 1 << 20)[1]) == value,
            'manifest matches caller value')
    pins.recheck()
    return value, verified
