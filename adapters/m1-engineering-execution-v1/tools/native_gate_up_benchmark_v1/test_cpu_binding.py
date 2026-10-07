"""Fixtures joining real checkpoint receipts to synthetic native-harness evidence."""
import copy
import hashlib
import json
from pathlib import Path
import tarfile
from unittest.mock import patch

_RAW = None


class Evidence:
    def __init__(self, c):
        global _RAW
        self.c, self.raw = c, {}
        self.contract = c.cpu_contract()
        self.current = self.contract.current
        expected = self.current.catalog()
        self.expected = copy.deepcopy(expected)
        if _RAW is None:
            with tarfile.open(Path(__file__).resolve().parent.parent / 'cpu-binding-fixtures-a009.tar.gz') as packed:
                _RAW = {m.name:packed.extractfile(m).read() for m in packed if m.isfile()}
        checkpoint = self.current.bindings(expected)
        for name, phase in checkpoint['phases'].items():
            for key, item in phase.items():
                path = '/input/checkpoint/' + name + '/' + key
                self.raw[path] = _RAW[name + '/' + key]
                item['path'] = path
        # Custody payload hashing is exercised with real ELFs by the separate
        # current-checkpoint suite; staging fixtures use bounded synthetic bytes.
        for index,(original,item) in enumerate(checkpoint['custody'].items()):
            raw = ('synthetic custody ' + original).encode()
            digest = hashlib.sha256(raw).hexdigest()
            path = '/input/custody/' + str(index)
            item.update(path=path,sha256=digest)
            self.expected['custody'][original] = digest
            self.raw[path] = raw
        self.current.catalog = lambda:self.expected
        self.build = {'schema':'FerricNativeGateUpBuildBindingR1','runtime_main':c.RUNTIME_MAIN,
            **{name:{'path':'/input/' + name,'sha256':c.QUALIFIED_ARTIFACTS[name]}
               for name in ('runtime_source','controller_source','worker')},
            'controllers':{name:{'path':'/input/'+name,'sha256':digest} for name,digest in c.QUALIFIED_ARTIFACTS['controllers'].items()},
            'cpu_qualification':{'path':'/input/cpu','sha256':'a'*64}}
        source_names = set(c.OWN_SOURCES) | {'measurement/' + n for n in c.MEASUREMENT_SOURCES}
        sources = {name:'a'*64 for name in source_names}
        harness = {'manifest':self.put('/input/harness/manifest',c.encoded(sources)),
            'original_actual_binding':self.put('/input/harness/original',c.encoded(checkpoint)), 'roles':{}}
        helper = b'synthetic harness qualifier'
        self.contract.HELPER_SHA = hashlib.sha256(helper).hexdigest()
        for role in sorted(self.contract.ROLES):
            prefix = '/input/harness/' + role + '/'
            outer = self.current.decode(_RAW['model-library/result'])
            outer['argv'] = ['/bin/bash',self.contract.D+'/owner/cpu-env-42g-emitter.sh','/usr/bin/python3','-I','-B',self.contract.HELPER,role,harness['manifest']['sha256']]
            inner = {'schema':'FerricGateUpCurrentHarnessQualificationA008','accepted':True,'role':role,
                'manifest_sha256':harness['manifest']['sha256'],'source_sha256':sources,
                'helper_sha256':self.contract.HELPER_SHA,'returncode':0,'native_executed':False,'native_launch_admitted':False,
                'planning_increment_bytes':32*1024**2,'stage_before_bytes':1024,'stage_after_bytes':2048}
            if role == 'actual':
                inner.update(binding_sha256=harness['original_actual_binding']['sha256'],
                    validation={'accepted':True,'phases':24,'custody_files':42,'native_executed':False,'native_launch_admitted':False,'performance_qualified':False})
            else:
                directory = self.contract.INPUT + '/harness' + ('/measurement' if role == 'measurement' else '')
                pattern = 'test_cpu_binding_a009.py' if role == 'binding' else 'test_*.py'
                inner.update(argv=['/usr/bin/python3','-I','-B','-m','unittest','discover','-v','-s',directory,'-p',pattern],
                    tests_passed=self.contract.COUNTS[role])
            values = {'status':b'0\n','result':c.encoded(outer),'stdout':json.dumps(inner,sort_keys=True).encode()+b'\n',
                'stderr':b'','inner':c.encoded(inner),'helper':helper}
            if role != 'actual':
                values.update(role_stdout=b'',role_stderr=('Ran %d tests in 0.001s\n\nOK\n' % self.contract.COUNTS[role]).encode())
            harness['roles'][role] = {key:self.put(prefix+key,raw) for key,raw in values.items()}
        self.cpu = {'schema':'FerricNativeGateUpCpuQualificationR2',
            **{name+'_sha256':self.build[name]['sha256'] for name in ('runtime_source','controller_source','worker')},
            'controllers':{name:item['sha256'] for name,item in self.build['controllers'].items()},
            'checkpoint':checkpoint,'harness':harness,'harness_sources':sources}

    def put(self,path,raw):
        self.raw[path] = raw
        return {'path':path,'sha256':hashlib.sha256(raw).hexdigest()}

    def read(self,path,expected=None,maximum=512*1024**2,empty=False):
        raw = self.raw[str(path)]
        digest = hashlib.sha256(raw).hexdigest()
        self.c.require((empty or raw) and len(raw) <= maximum and (expected is None or expected == digest),'fixture hash and size')
        return raw,digest

    def bound(self,item):
        return self.c.decode(self.read(item['path'],item['sha256'])[0])

    def install(self,testcase):
        for name,value in (('WIDTH_QUALIFIED',True),('cpu_contract',lambda:self.contract)):
            mock = patch.object(self.c,name,value)
            mock.start()
            testcase.addCleanup(mock.stop)
        return self

    def validate(self):
        self.c.validate_cpu(self.cpu,self.build,self.bound,self.read)
