"""CPU-only down checkpoint replay and targeted refusal tests."""
import copy
import importlib.util
import io
from pathlib import Path
import tarfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('test_down_checkpoint',HERE/'cpu_binding_down_r1.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)


class DownCpuBindingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binding = c.bindings()
        cls.raw = {key:c.ancestry.file_bytes(row['path'],256*1024**2)
                   for key,row in cls.binding['files'].items()}
        cls.receipt = c.decode(cls.raw['receipt'])
        cls.members = c.archive_members(cls.raw['archive'],cls.receipt['files'])

    def read(self,path,maximum):
        for key,row in self.binding['files'].items():
            if path == row['path']:
                return self.raw[key]
        raise ValueError('unbound fixture path')

    def mutation(self,path,raw):
        members = dict(self.members)
        members[path] = raw
        audit = c.decode(members[c.O+'exact-audit-a002.json'])
        audit['files'][c.D+'/'+path] = {'sha256':c.digest(raw),'bytes':len(raw)}
        members[c.O+'exact-audit-a002.json'] = c.encoded(audit)
        pin = c.digest(members[c.O+'exact-audit-a002.json'])
        outer = 'results/pages-native-down-exact-audit-a002/stdout'
        value = c.decode(members[outer])
        value['audit_sha256'] = pin
        members[outer] = c.encoded(value)
        return members,pin

    def reject_member(self,path,raw):
        members,pin = self.mutation(path,raw)
        with patch.object(c,'AUDIT_SHA',pin), self.assertRaises(ValueError):
            c.replay(members)

    def small_archive(self,changes=None):
        target = io.BytesIO()
        roster = {'input':{'sha256':c.digest(b'input'),'bytes':5,'mode':0o600}}
        with tarfile.open(fileobj=target,mode='w:gz') as archive:
            row = tarfile.TarInfo('input')
            row.size,row.mode = 5,0o600
            if changes:
                changes(row,archive)
            archive.addfile(row,io.BytesIO(b'input'))
        return target.getvalue(),roster

    def test_complete_real_down_checkpoint(self):
        for role in ('image','capture-control','capture-candidate','abi-control','abi-candidate'):
            raw = self.members[c.O+'actual-'+role+'/stdout']
            self.assertIn(c.ACTUAL[role].encode()+b' ... ',raw)
            self.assertNotIn(c.ACTUAL[role].encode()+b' ... ok\n',raw)
        result = c.validate(self.binding,self.read)
        self.assertEqual(result['phases'],19)
        self.assertEqual(result['formatted_source_files'],1441)
        self.assertFalse(result['performance_qualified'])

    def test_portable_bindings_preserve_original_inner_paths(self):
        value = copy.deepcopy(self.binding)
        raw = {}
        for key,row in value['files'].items():
            row['path'] = '/retained/'+key
            raw[row['path']] = self.raw[key]
        self.assertTrue(c.validate(value,lambda path,maximum:raw[path])['accepted'])

    def test_every_binding_is_required(self):
        for key in c.FILES:
            value = copy.deepcopy(self.binding)
            del value['files'][key]
            with self.assertRaises(ValueError):
                c.validate(value,self.read)

    def test_old_or_relabelled_archive_refuses(self):
        value = copy.deepcopy(self.binding)
        value['files']['archive']['sha256'] = c.SOURCE_SHA
        with self.assertRaises(ValueError):
            c.validate(value,self.read)

    def test_replaced_receipt_refuses(self):
        value = copy.deepcopy(self.binding)
        value['files']['receipt']['sha256'] = '0'*64
        with self.assertRaises(ValueError):
            c.validate(value,self.read)

    def test_archive_path_escapes_refuse(self):
        for name in ('../input','/input','a/../input'):
            raw,roster = self.small_archive(lambda row,archive:setattr(row,'name',name))
            with self.assertRaises(ValueError):
                c.archive_members(raw,roster)

    def test_archive_links_refuse(self):
        def link(row,archive):
            row.type,row.linkname,row.size = tarfile.SYMTYPE,'input',0
        raw,roster = self.small_archive(link)
        with self.assertRaises(ValueError):
            c.archive_members(raw,roster)

    def test_archive_duplicate_members_refuse(self):
        raw,roster = self.small_archive(lambda row,archive:archive.addfile(row,io.BytesIO(b'input')))
        with self.assertRaises(ValueError):
            c.archive_members(raw,roster)

    def test_archive_missing_members_refuse(self):
        raw,roster = self.small_archive()
        roster['missing'] = roster['input']
        with self.assertRaises(ValueError):
            c.archive_members(raw,roster)

    def test_archive_sizes_modes_and_hashes_refuse(self):
        for key,value in (('bytes',6),('mode',0o700),('sha256','0'*64)):
            raw,roster = self.small_archive()
            roster['input'][key] = value
            with self.assertRaises(ValueError):
                c.archive_members(raw,roster)

    def test_unclean_outer_refuses(self):
        path = 'results/pages-native-down-full-live-a004/result.json'
        for key,value in (('status',False),('returncode',-15),('term_sent',True),('kill_sent',True),
                          ('cleanup_ok',False),('child_reaped',False),('errors',['failure'])):
            row = c.decode(self.members[path])
            row[key] = value
            with self.assertRaises(ValueError):
                c.validate_outer(row,row['argv'])

    def test_profile_label_cannot_relax_limits(self):
        original = c.decode(self.members['results/pages-native-down-full-live-a004/result.json'])
        for change in (lambda row:row['limits'].update(stage_bytes=48*1024**3),
                       lambda row:row.update(cpus=[0,1,2,3,4]),
                       lambda row:row.update(cpus=[False,True,2.0,3.0]),
                       lambda row:row['launch_environment'].update(HIP_VISIBLE_DEVICES='0'),
                       lambda row:row['final_resources'].update(root_free_bytes=0)):
            row = copy.deepcopy(original)
            change(row)
            with self.assertRaises(ValueError):
                c.validate_outer(row,row['argv'])

    def test_role_helper_and_release_substitutions_refuse(self):
        for role,key,value in (('full-counters','role','full-live'),('full-live','helper_sha256',c.QUALIFIERS['a003']),
                               ('release-live','source_archive_sha256','0'*64)):
            path = c.O+role+'-a004/receipt.json'
            inner = c.decode(self.members[path])
            inner[key] = value
            self.reject_member(path,c.encoded(inner))
        path = c.O+'release-live-a004/receipt.json'
        inner = c.decode(self.members[path])
        inner['release_binary']['sha256'] = '60bf47551d1ebdfc91f49b529dab8211e3f860a07bdd05921e642b72d505e9a4'
        self.reject_member(path,c.encoded(inner))

    def test_actual_raw_footer_and_named_test_refuse(self):
        path = c.O+'actual-abi-candidate/stdout'
        raw = self.members[path]
        for altered in (raw.replace(b'1 passed;',b'0 passed;'),raw+raw,
                        raw.replace(c.ACTUAL['abi-candidate'].encode(),b'wrong_test')):
            self.reject_member(path,altered)

    def test_formatted_source_and_graph_mutations_refuse(self):
        for path in (c.O+'source/adapters/m1-engineering-execution-v1/src/tp_execution/batched/splitk_down_r1.rs',
                     c.O+'actual-capture-candidate/graph.json'):
            self.reject_member(path,self.members[path]+b'\n')


if __name__ == '__main__':
    unittest.main()
