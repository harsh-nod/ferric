"""CPU-only replay of retained receipts and negative evidence-binding tests."""
import copy
import importlib.util
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('current_cpu_binding', HERE / 'cpu_binding_a009.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
spec = importlib.util.spec_from_file_location('down_ancestry_paths', HERE / 'cpu_binding_down_r1.py')
down_paths = importlib.util.module_from_spec(spec)
spec.loader.exec_module(down_paths)

class CurrentCpuBindingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.expected = c.catalog()
        path = HERE.parent / 'cpu-binding-fixtures-a009.tar.gz'
        cls.assert_pin = 'ed156a7c5b0def359003e57f6ec82c638a2480172837ae8e5d49748d9af2b154'
        if c.digest(path.read_bytes()) != cls.assert_pin:
            raise ValueError('exact retained fixture archive required')
        cls.original = {}
        with tarfile.open(path) as archive:
            for member in archive:
                if not member.isfile() or member.name in cls.original or member.size > c.LIMITS['log_bytes']:
                    raise ValueError('closed regular fixture archive')
                cls.original[member.name] = archive.extractfile(member).read()
        if set(cls.original) != {name + '/' + key for name in cls.expected['phases'] for key in c.FIELDS}:
            raise ValueError('complete 192-file receipt fixture')

    def setUp(self):
        self.value = down_paths.relocate_ancestry(c.bindings(self.expected))
        self.raw = {self.value['phases'][name][key]['path']:self.original[name + '/' + key]
            for name in self.expected['phases'] for key in c.FIELDS}

    def read(self, path, maximum):
        return self.raw[path] if path in self.raw else c.file_bytes(path, maximum)

    def streams(self, name):
        return {key:self.raw[item['path']] for key,item in self.value['phases'][name].items()}

    def inner(self, name):
        return c.decode(self.streams(name)['inner'])

    def validate_inner(self, name, inner=None, streams=None):
        c.validate_inner(name, self.inner(name) if inner is None else inner,
            self.expected['phases'][name], self.streams(name) if streams is None else streams)

    def test_complete_real_checkpoint_and_artifact_custody(self):
        result = c.validate(self.value, self.read)
        self.assertEqual(result['phases'], 24)
        self.assertEqual(result['custody_files'], 42)
        self.assertFalse(result['native_launch_admitted'])

    def test_all_receipt_groups_match_their_stdout_projection(self):
        for name in self.expected['phases']:
            with self.subTest(name=name):
                self.validate_inner(name)

    def test_relocated_evidence_preserves_original_inner_paths(self):
        relocated = {}
        for name, phase in self.value['phases'].items():
            for key, item in phase.items():
                path = '/retained/' + name + '/' + key
                relocated[path] = self.raw[item['path']]
                item['path'] = path
        for original,item in self.value['custody'].items():
            item['path'] = '/retained/custody/' + item['sha256']
            relocated[item['path']] = original
        def reader(path, maximum):
            value = relocated[path]
            return value if isinstance(value,bytes) else c.file_bytes(down_paths.ancestry_path(value),maximum)
        self.assertTrue(c.validate(self.value, reader)['accepted'])

    def test_every_required_phase_is_mandatory(self):
        for name in list(self.value['phases']):
            row = self.value['phases'].pop(name)
            with self.assertRaisesRegex(ValueError, 'complete CPU phase roster'):
                c.validate(self.value, self.read)
            self.value['phases'][name] = row

    def test_every_external_custody_entry_is_mandatory(self):
        for path in list(self.value['custody']):
            row = self.value['custody'].pop(path)
            with self.assertRaisesRegex(ValueError, 'complete external artifact custody'):
                c.validate(self.value, self.read)
            self.value['custody'][path] = row

    def test_obsolete_catalog_and_extra_keys_refuse(self):
        for change in (lambda v:v.update(catalog_sha256='0'*64),lambda v:v.update(extra=True),
            lambda v:v.update(schema='FerricNativeGateUpCpuQualificationR1')):
            value = copy.deepcopy(self.value)
            change(value)
            with self.assertRaises(ValueError):
                c.validate(value,self.read)

    def test_replaced_role_evidence_refuses(self):
        self.value['phases']['actual-abi-candidate'] = self.value['phases']['actual-abi-control']
        with self.assertRaises(ValueError):
            c.validate(self.value,self.read)

    def test_raw_hash_cannot_be_relabelled(self):
        item = self.value['phases']['actual-abi-candidate']['inner']
        self.raw[item['path']] += b'\n'
        item['sha256'] = c.digest(self.raw[item['path']])
        with self.assertRaises(ValueError):
            c.validate(self.value,self.read)

    def test_helper_mutation_refuses(self):
        item = self.value['phases']['actual-abi-candidate']['helper']
        self.raw[item['path']] += b'\n'
        with self.assertRaises(ValueError):
            c.validate(self.value,self.read)

    def test_omitted_raw_log_refuses(self):
        del self.value['phases']['actual-abi-candidate']['role_stdout']
        with self.assertRaisesRegex(ValueError,'raw evidence fields'):
            c.validate(self.value,self.read)

    def test_stale_revision_source_and_preparation_refuse(self):
        cases = [('runtime-tests','revision'),('model-library','source_archive_sha256'),
            ('actual-abi-candidate','source_preparation_sha256')]
        for name,key in cases:
            row = self.inner(name)
            row[key] = '0' * len(row[key])
            with self.assertRaises(ValueError):
                self.validate_inner(name,row)

    def test_swapped_test_or_release_elf_refuses(self):
        for name,key in (('model-library','test_binary'),('model-release-live','release_binary'),('runtime-worker','worker')):
            row = self.inner(name)
            row[key]['sha256'] = '0'*64
            with self.assertRaises(ValueError):
                self.validate_inner(name,row)

    def test_list_only_or_wrong_binary_command_refuses(self):
        for name in ('model-library','actual-full-live','actual-abi-candidate'):
            for change in (lambda a:a.append('--list'),lambda a:a.__setitem__(0,'/bin/true')):
                row = self.inner(name)
                change(row['argv'])
                with self.assertRaises(ValueError):
                    self.validate_inner(name,row)

    def test_success_booleans_are_not_return_codes(self):
        row = self.inner('model-library')
        row['returncode'] = False
        with self.assertRaises(ValueError):
            self.validate_inner('model-library',row)

    def test_missing_or_changed_source_roster_refuses(self):
        for key in ('source_before','source_after'):
            row = self.inner('model-library')
            row.pop(key)
            with self.assertRaises(ValueError):
                self.validate_inner('model-library',row)

    def test_outer_stdout_cannot_omit_non_roster_fields(self):
        for name in ('model-library','runtime-tests','actual-abi-candidate'):
            streams = self.streams(name)
            row = c.decode(streams['stdout'].splitlines()[-1])
            del row['argv']
            streams['stdout'] = json.dumps(row,sort_keys=True).encode() + b'\n'
            with self.assertRaisesRegex(ValueError,'projected inner'):
                self.validate_inner(name,streams=streams)

    def test_actual_stdout_cannot_use_model_projection(self):
        name = 'actual-abi-candidate'
        streams = self.streams(name)
        row = self.inner(name)
        row.pop('inputs')
        streams['stdout'] = json.dumps(row,sort_keys=True).encode() + b'\n'
        with self.assertRaisesRegex(ValueError,'projected inner'):
            self.validate_inner(name,streams=streams)

    def test_receipt_json_cannot_substitute_for_test_stdout(self):
        for name in c.TEST_COUNTS:
            streams = self.streams(name)
            streams['role_stdout'] = streams['stdout']
            with self.assertRaisesRegex(ValueError,'executed test counts'):
                self.validate_inner(name,streams=streams)

    def test_missing_duplicate_or_zero_test_footer_refuses(self):
        for raw in (b'',b'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'):
            streams = self.streams('actual-full-live')
            streams['role_stdout'] = raw
            with self.assertRaises(ValueError):
                self.validate_inner('actual-full-live',streams=streams)
        streams = self.streams('actual-full-live')
        streams['role_stdout'] *= 2
        with self.assertRaises(ValueError):
            self.validate_inner('actual-full-live',streams=streams)

    def test_cargo_checks_are_not_reported_as_tests(self):
        for name in ('model-union','model-default','model-clippy'):
            streams = self.streams(name)
            streams['role_stdout'] += b'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
            with self.assertRaises(ValueError):
                self.validate_inner(name,streams=streams)

    def test_inner_growth_and_reserve_cannot_be_relaxed(self):
        for key,value in (('stage_before_bytes',c.LIMITS['stage_bytes']),('stage_after_bytes',c.LIMITS['stage_bytes']),
            ('stage_before_bytes',False),('planning_increment_bytes',1)):
            row = self.inner('model-library')
            row[key] = value
            with self.assertRaises(ValueError):
                self.validate_inner('model-library',row)

    def test_outer_failure_or_signals_refuse(self):
        spec = self.expected['phases']['model-library']
        for key,value in (('status',False),('returncode',1),('cleanup_ok',False),('child_reaped',False),
            ('errors',['changed']),('term_sent',True),('kill_sent',True),('reason','timeout')):
            row = c.decode(self.streams('model-library')['result'])
            row[key] = value
            with self.assertRaises(ValueError):
                c.validate_outer(row,spec)

    def test_cpu_limits_visibility_and_command_are_fixed(self):
        spec = self.expected['phases']['model-library']
        for change in (lambda r:r['limits'].update(stage_bytes=43*1024**3),
            lambda r:r['launch_environment'].update(HIP_VISIBLE_DEVICES='0'),
            lambda r:r['argv'].append('--list'),lambda r:r.update(cpus=[0,1,2,3,4]),
            lambda r:r.update(nice=0),lambda r:r.update(build_jobs=8),
            lambda r:r.update(peak_observed_rss_bytes=9*1024**3),
            lambda r:r['final_resources'].update(root_free_bytes=0)):
            row = c.decode(self.streams('model-library')['result'])
            change(row)
            with self.assertRaises(ValueError):
                c.validate_outer(row,spec)

    def test_duplicate_and_nonfinite_json_refuse(self):
        for raw in (b'{"a":1,"a":2}',b'{"a":NaN}'):
            with self.assertRaises(ValueError):
                c.decode(raw)

    def test_noncanonical_symlink_and_oversize_files_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'regular'
            path.write_bytes(b'1234')
            (root / 'link').symlink_to(path)
            for candidate,maximum in ((root/'link',4),(path,3),(root,100)):
                with self.assertRaises(ValueError):
                    c.file_bytes(candidate,maximum)
            self.assertEqual(c.file_bytes(path,4),b'1234')

    def test_unreviewed_catalog_cannot_validate(self):
        with patch.object(c,'CATALOG_SHA',None):
            with self.assertRaises(ValueError):
                c.validate(self.value,self.read)

if __name__ == '__main__':
    unittest.main()
