"""Small policy fixtures only; these tests do not qualify the actual archives."""
import copy
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import types
import unittest
from unittest.mock import patch

import portable as P
import export as X


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class Pins:
    def __init__(self):
        self.records = {}

    def read(self, path, expected=None, retain=False, maximum=1 << 30):
        P.require(path.resolve(strict=True) == path and path.is_file(), 'fixture canonical file')
        raw = path.read_bytes()
        P.require(len(raw) <= maximum and (expected is None or digest(raw) == expected), 'fixture pin')
        record = P.fp(path, len(raw), digest(raw))
        P.require(self.records.setdefault(str(path), record) == record, 'fixture stable pin')
        return record, raw if retain else b''

    def pin(self, path, expected=None):
        return self.read(path, expected)[0]

    def recheck(self):
        for record in list(self.records.values()):
            self.read(Path(record['path']), record['sha256'])


D = types.SimpleNamespace(parse=json.loads, Pins=Pins)

ACTUAL_COMMAND_HASHES = {
    'fixture-metadata': 'd40be588c6ca55400fcee6e6bbea7c2efc49e8e7385606c65bc25f11b03996bb',
    'checked-lowering': '7a7afbf4cae316fdab3c7e7c4469342fb1ee29faf4bb444bae215dc267383987',
    'actual-replay': '5fb4966d95f00126eef093b6466549ff019a748069277ccb8da07cd6dbc8cb6d',
    'actual-inert-join': '79f99463fe117cd9705a4911ac5546a1fdf46838d939c3af9c053bb1f7732200',
    'emit': '969cd8acf6939c97fe9ee45252407285d26903821e083fd28ff84025d967faab',
    'extract-retained': '83754b57e71a187a6c7d84ab9bb88491397a413e88009cf0b4d6d41a03923d20',
    'descriptor-metadata': '64e89affd84e28f392b95be766b4a82e50700d6ed9f9e892668692bf631b7393',
    'elf-notes': '4ca7748979883c05a66e83023e1ce070f4f4e5f3cffd7b5375d17dd4d7f6643b',
    'disassembly': '3bbfee0a717d630de1cd5dbe895b8ec619e6ef5a8ed49f39b427b0fa805a2666',
}
ACTUAL_BINDINGS = {
    '@candidate.semantic.sha256': '335d72d39460c835101d409bc684e1f34cc1e733b0ba30d8b524ea046daf5e2a',
    '@candidate.handoff.sha256': '6117bbcade7a0a66c83694e26ed0313c1848834e0ceeaa67824d1ac35cceb6b7',
    '@candidate.handoff.bytes': 4022416,
    '@candidate.image.sha256': '4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5',
    '@candidate.image.bytes': 53560,
}


class MemoryArchive:
    def __init__(self):
        self.values, self.records = {}, {}

    def add(self, path, value):
        raw = value if type(value) is bytes else json.dumps(value, sort_keys=True).encode()
        record = P.fp(path, len(raw), digest(raw))
        self.values[str(path)], self.records[str(path)] = raw, record
        return record

    def check(self, record):
        P.require(self.records[record['path']] == record, 'fixture original pin')

    def doc(self, record):
        self.check(record)
        return json.loads(self.values[record['path']])


class DeploymentPolicyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.addCleanup(self.temp.cleanup)

    def packed(self, rows, count=None):
        path = self.root / ('archive-' + str(len(list(self.root.iterdir()))) + '.tar.gz')
        with tarfile.open(path, 'w:gz') as archive:
            for name, body, kind in rows:
                member = tarfile.TarInfo(name)
                member.type = kind
                if kind == tarfile.SYMTYPE:
                    member.linkname = '/outside'
                member.size = len(body) if kind == tarfile.REGTYPE else 0
                archive.addfile(member, io.BytesIO(body) if kind == tarfile.REGTYPE else None)
        pins = Pins()
        return path, pins, pins.pin(path), len(rows) if count is None else count

    def scan(self, rows, count=None):
        path, pins, record, count = self.packed(rows, count)
        return P.Archive(D, pins, record, count), path, pins

    def phase_fixture(self):
        archive, root, name = MemoryArchive(), P.E / 'fixture', 'test'
        command = dict(argv=['/original/tool', '--test'], env={}, tools={}, deadline_seconds=120,
            cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10, gpu_execution=False, expected_exit=0)
        records = {}
        for key, suffix, value in (('command', 'command.json', command),
            ('started', 'started.json', dict(pid=10, pgid=10)),
            ('stdout', 'stdout', b'result\n'), ('stderr', 'stderr', b'')):
            records[key] = archive.add(root / (name + '-' + suffix), value)
        result = dict(exit_code=0, reason=None, elapsed_seconds=1.0, group_absent=True, cache_bytes=123,
            stdout_sha256=records['stdout']['sha256'], stderr_sha256=records['stderr']['sha256'])
        records['result'] = archive.add(root / (name + '-result.json'), result)
        return archive, root, name, records, command

    def header(self):
        directory = self.root / 'prefix-independent-deployment-v228-v1'
        value = dict(schema=P.SCHEMA, directory=str(directory), archives=dict(compiler={}, native={}),
            qualifications=dict(compiler={}, native={}), image={}, binary={}, authority='none',
            **{key: False for key in P.NONCLAIMS})
        return directory, value

    def actual_recipe(self):
        raw = (Path(__file__).parent / 'fixtures/recipe.json').read_bytes()
        self.assertEqual(digest(raw), 'f500b1c3d3218b53e9ba1df92d6f048a41f05873c9539f16bc69b20f85575a52')
        return json.loads(raw)

    def test_all_nine_authentic_recipe_commands_match_retained_actual_records(self):
        recipe = self.actual_recipe()
        self.assertEqual(tuple(row['name'] for row in recipe['commands']), P.STAGES)
        for template in recipe['commands']:
            name = template['name']
            raw = (Path(__file__).parent / ('fixtures/' + name + '-command.json')).read_bytes()
            self.assertEqual(digest(raw), ACTUAL_COMMAND_HASHES[name])
            expected = {key: value for key, value in P.substitute(template, ACTUAL_BINDINGS).items()
                        if key not in ('name', 'cwd')}
            self.assertEqual(expected, json.loads(raw), name)

    def test_authentic_byte_counts_are_argv_strings_without_changing_numeric_limits(self):
        templates = {row['name']: row for row in self.actual_recipe()['commands']}
        for name in ('emit', 'extract-retained', 'descriptor-metadata'):
            template = templates[name]
            original = copy.deepcopy(template)
            value = P.substitute(template, ACTUAL_BINDINGS)
            self.assertEqual(template, original)
            self.assertTrue(all(type(item) is str for item in value['argv'] + list(value['env'].values())))
            self.assertIs(type(value['deadline_seconds']), int)
            self.assertIs(type(value['cache_cap_bytes']), int)
        self.assertIn('4022416', P.substitute(templates['emit'], ACTUAL_BINDINGS)['argv'])
        self.assertIn('53560', P.substitute(templates['descriptor-metadata'], ACTUAL_BINDINGS)['argv'])

    def test_substitution_refuses_missing_artifact_and_nonstring_argv(self):
        for command, bindings in ((dict(argv=['@candidate.missing'], env={}), ACTUAL_BINDINGS),
            (dict(argv=[4022416], env={}), ACTUAL_BINDINGS),
            (dict(argv=['tool'], env={'VALUE': 1}), ACTUAL_BINDINGS)):
            with self.subTest(command=command), self.assertRaises(RuntimeError):
                P.substitute(command, bindings)

    def test_file_pin_rejects_boolean_size_and_extra_fields(self):
        for value in (dict(P.IMAGE, bytes=True), dict(P.IMAGE, extra=1)):
            with self.assertRaises(RuntimeError):
                P.pin(value)

    def test_file_pin_rejects_noncanonical_path_and_digest(self):
        for value in (dict(P.IMAGE, path='/root/../image'), dict(P.IMAGE, sha256='not-a-digest')):
            with self.assertRaises(RuntimeError):
                P.pin(value)

    def test_relocation_binds_both_original_and_transported_identity(self):
        directory, _ = self.header()
        self.assertEqual(P.FILENAMES['binary'], 'gfx950-qwen-prefix-tiles-comparison-v6')
        binary = dict(P.BINARY, path=str(directory / 'gfx950-qwen-prefix-tiles-comparison-v6'))
        self.assertEqual(P.relocation(dict(original=P.BINARY, transported=binary), P.BINARY,
            directory, P.FILENAMES['binary']), binary)
        target = dict(P.IMAGE, path=str(directory / P.FILENAMES['image']))
        row = dict(original=P.IMAGE, transported=target)
        self.assertEqual(P.relocation(row, P.IMAGE, directory, P.FILENAMES['image']), target)
        row['original'] = dict(P.IMAGE, path='/old/source-v5/artifact.hsaco')
        with self.assertRaises(RuntimeError):
            P.relocation(row, P.IMAGE, directory, P.FILENAMES['image'])

    def test_relocation_rejects_wrong_image_and_target_path(self):
        directory, _ = self.header()
        for target in (dict(P.IMAGE, sha256='0' * 64), dict(P.IMAGE, path='/elsewhere/artifact.hsaco')):
            with self.assertRaises(RuntimeError):
                P.relocation(dict(original=P.IMAGE, transported=target), P.IMAGE, directory, P.FILENAMES['image'])

    def test_deployment_requires_new_namespace(self):
        for name in ('prefix-parity-deployment-v227-v1', 'prefix-independent-deployment-v228-v0',
                     'prefix-independent-deployment-v228-v1-extra'):
            with self.assertRaises(RuntimeError):
                P.directory_name(self.root / name)

    def test_header_forbids_every_authority_claim(self):
        directory, value = self.header()
        P.deployment_header(value, directory)
        for key in P.NONCLAIMS:
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                P.deployment_header(dict(value, **{key: True}), directory)

    def test_header_closes_fields_and_qualification_roster(self):
        directory, value = self.header()
        for changed in (dict(value, historical_source_v5={}), dict(value, qualifications={'compiler': {}})):
            with self.assertRaises(RuntimeError):
                P.deployment_header(changed, directory)

    def test_archive_streams_original_identities_without_opening_them(self):
        archive, path, pins = self.scan([('row/value.json', b'{"value":1}', tarfile.REGTYPE)])
        original = archive.member(P.E / 'row/value.json')
        self.assertEqual(archive.doc(original), {'value': 1})
        self.assertEqual(set(pins.records), {str(path)})

    def test_archive_rejects_duplicate_names(self):
        with self.assertRaises(RuntimeError):
            self.scan([('row/a', b'1', tarfile.REGTYPE), ('row/a', b'2', tarfile.REGTYPE)])

    def test_archive_rejects_absolute_and_traversal_names(self):
        for name in ('/outside', '../outside', 'row/../outside', 'row//value'):
            with self.subTest(name=name), self.assertRaises(RuntimeError):
                self.scan([(name, b'1', tarfile.REGTYPE)])

    def test_archive_rejects_links(self):
        with self.assertRaises(RuntimeError):
            self.scan([('row/link', b'', tarfile.SYMTYPE)])

    def test_archive_requires_exact_member_census(self):
        with self.assertRaises(RuntimeError):
            self.scan([('row/a', b'1', tarfile.REGTYPE)], count=2)

    def test_archive_bounds_each_member(self):
        with patch.object(P, 'MAX_MEMBER', 1), self.assertRaises(RuntimeError):
            self.scan([('row/a', b'12', tarfile.REGTYPE)])

    def test_archive_bounds_expanded_bytes(self):
        with patch.object(P, 'MAX_TOTAL', 1), self.assertRaises(RuntimeError):
            self.scan([('row/a', b'1', tarfile.REGTYPE), ('row/b', b'2', tarfile.REGTYPE)])

    def test_archive_bounds_retained_text(self):
        with patch.object(P, 'MAX_RETAINED', 1), self.assertRaises(RuntimeError):
            self.scan([('row/a.json', b'{}', tarfile.REGTYPE)])

    def test_archive_rejects_wrong_compressed_pin(self):
        _, pins, record, count = self.packed([('row/a', b'1', tarfile.REGTYPE)])
        with self.assertRaises(RuntimeError):
            P.Archive(D, pins, dict(record, sha256='0' * 64), count)

    def test_archived_sources_are_hashed_but_not_retained_as_documents(self):
        archive, _, _ = self.scan([('row/source/fe2o3/fixture.json', b'{}', tarfile.REGTYPE)])
        original = archive.member(P.E / 'row/source/fe2o3/fixture.json')
        self.assertEqual(original['sha256'], digest(b'{}'))
        with self.assertRaises(RuntimeError):
            archive.doc(original)

    def test_phase_joins_natural_result_and_actual_streams(self):
        args = self.phase_fixture()
        self.assertEqual(P.phase(*args)['exit_code'], 0)

    def test_phase_rejects_changed_exact_command(self):
        args = list(self.phase_fixture())
        args[-1] = dict(args[-1], argv=['/other/tool'])
        with self.assertRaises(RuntimeError):
            P.phase(*args)

    def test_phase_rejects_wrong_stream_join(self):
        archive, root, name, records, command = self.phase_fixture()
        value = archive.doc(records['result'])
        value['stdout_sha256'] = '0' * 64
        records['result'] = archive.add(root / 'test-result.json', value)
        with self.assertRaises(RuntimeError):
            P.phase(archive, root, name, records, command)

    def test_phase_rejects_failure_or_unreaped_group(self):
        for changed in ({'exit_code': 1}, {'group_absent': False}, {'reason': 'timeout'}):
            archive, root, name, records, command = self.phase_fixture()
            records['result'] = archive.add(root / 'test-result.json', dict(archive.doc(records['result']), **changed))
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                P.phase(archive, root, name, records, command)

    def test_terminal_refuses_cleanup_or_uncertain_reap(self):
        owned = dict(exit_code=0, reason=None, cleanup_signalled=False,
                     owned_groups_absent=True, owned_processes_reaped=True)
        value = dict(passed=True, error=None, postcheck_errors=[], owned=owned)
        P.terminal(value)
        for changed in ({'cleanup_signalled': True}, {'owned_processes_reaped': False}):
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                P.terminal(dict(value, owned=dict(owned, **changed)))

    def test_overlay_requires_preimages_and_absence_preserves_unmodified_files(self):
        old, new = dict(bytes=1, sha256='1' * 64), dict(bytes=2, sha256='2' * 64)
        previous = {'host/execution.rs': old, 'host/process.rs': old}
        overlay = dict(source_directory='host', required_preimages=[dict(old, path='host/execution.rs')],
            required_absent_paths=['host/independent.rs'],
            files=[dict(new, path='source/execution.rs'), dict(new, path='source/independent.rs')])
        result = P.overlay_sources(previous, overlay)
        self.assertEqual(result, {'host/execution.rs': new, 'host/process.rs': old, 'host/independent.rs': new})
        self.assertEqual(previous['host/execution.rs'], old)
        for changed in (dict(previous, **{'host/execution.rs': new}), dict(previous, **{'host/independent.rs': old})):
            with self.assertRaises(RuntimeError):
                P.overlay_sources(changed, overlay)

    def test_relative_source_map_rejects_escaped_or_relabelled_source(self):
        root = P.E / 'source'
        value = {str(root / 'a.rs'): {'pin': P.fp(root / 'a.rs', 1, '1' * 64)}}
        self.assertEqual(set(P.relative_sources(value, root)), {'a.rs'})
        value[str(root / 'a.rs')]['pin']['path'] = str(root / 'different.rs')
        with self.assertRaises(RuntimeError):
            P.relative_sources(value, root)

    def test_native_inventory_requires_all_ninety_nine_names_and_fourteen_passes(self):
        names = ['suite::case_' + str(index) for index in range(99)]
        listed = ''.join(name + ': test\n' for name in names)
        result = ''.join('test ' + name + ' ... ' + ('ok' if index < 98 else 'ignored') + '\n'
            for index, name in enumerate(names)) + 'test result: ok. 98 passed; 0 failed; 1 ignored;\n'
        P.native_test_result(listed, result, names[:14])
        for bad_list, bad_result, required in ((listed + names[0] + ': test\n', result, names[:14]),
            (listed, result.replace('suite::case_0 ... ok', 'suite::other ... ok'), names[:14]),
            (listed, result, names[:13] + [names[-1]])):
            with self.assertRaises(RuntimeError):
                P.native_test_result(bad_list, bad_result, required)

    def test_export_copy_is_exclusive_and_hash_checked(self):
        src, dst = self.root / 'input', self.root / 'output'
        src.write_bytes(b'qualified')
        expected = P.fp('/original', 9, digest(b'qualified'))
        X.copy_file(src, dst, expected)
        self.assertEqual(dst.read_bytes(), b'qualified')
        with self.assertRaises(FileExistsError):
            X.copy_file(src, dst, expected)
        with self.assertRaises(RuntimeError):
            X.copy_file(src, self.root / 'bad', dict(expected, sha256='0' * 64))

    def test_export_extracts_only_the_selected_verified_member(self):
        archive, _, _ = self.scan([('row/artifact', b'kernel', tarfile.REGTYPE), ('row/other', b'other', tarfile.REGTYPE)])
        dst = self.root / 'artifact'
        X.copy_member(archive, archive.member(P.E / 'row/artifact'), dst)
        self.assertEqual(dst.read_bytes(), b'kernel')
        self.assertFalse((self.root / 'other').exists())


if __name__ == '__main__':
    unittest.main()
