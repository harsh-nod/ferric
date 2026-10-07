"""Bind audited CPU artifacts and freshly tested harness sources to a native plan."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('gate_up_current_cpu', HERE / 'cpu_binding_a009.py')
current = importlib.util.module_from_spec(spec)
spec.loader.exec_module(current)
spec = importlib.util.spec_from_file_location('down_current_cpu', HERE / 'cpu_binding_down_r1.py')
down = importlib.util.module_from_spec(spec)
spec.loader.exec_module(down)
require, encoded, decode = current.require, current.encoded, current.decode
D = '/tmp/ferric-v16-emitter-b95a642-r1'
INPUT = D + '/inputs/native-down-harness-a001'
HELPER = INPUT + '/qualify_harness.py'
HELPER_SHA = 'bff697ea3763909174e65cb0b267be1485392cf4c40902a7ce792b978f435faa'
COUNTS = {'binding':26,'contract':128,'measurement':227}
ROLES = set(COUNTS) | {'actual'}
BASE_FIELDS = ('status','result','stdout','stderr','inner','helper')

def fields(role):
    require(role in ROLES, 'closed harness role')
    return BASE_FIELDS + (() if role == 'actual' else ('role_stdout','role_stderr'))

def checkpoint_identity(value):
    require(type(value) is dict and set(value) == {'schema','catalog_sha256','phases','custody'}, 'closed checkpoint identity')
    return {'schema':value['schema'],'catalog_sha256':value['catalog_sha256'],
        'phases':{name:{key:row['sha256'] for key,row in phase.items()} for name,phase in value['phases'].items()},
        'custody':{name:row['sha256'] for name,row in value['custody'].items()}}

def iter_bindings(cpu):
    for name,item in down.iter_bindings(cpu['controller_checkpoint']):
        yield 'down/' + name, item
    for name,phase in sorted(cpu['checkpoint']['phases'].items()):
        for key in sorted(phase):
            yield 'checkpoint/' + name + '/' + key, phase[key]
    for index,(name,item) in enumerate(sorted(cpu['checkpoint']['custody'].items())):
        yield 'custody/' + str(index), item
    harness = cpu['harness']
    for name in ('manifest','original_actual_binding'):
        yield 'harness/' + name, harness[name]
    for role,phase in sorted(harness['roles'].items()):
        for key in sorted(phase):
            yield 'harness/' + role + '/' + key, phase[key]

def validate_harness(cpu, read_raw):
    harness = cpu['harness']
    require(type(harness) is dict and set(harness) == {'manifest','roles','original_actual_binding'}, 'closed fresh harness proof')
    require(type(harness['roles']) is dict and set(harness['roles']) == ROLES, 'complete fresh harness roles')
    def raw(item):
        return current.bound_bytes(item,item['sha256'],read_raw,current.LIMITS['log_bytes'])
    manifest_raw = raw(harness['manifest'])
    manifest = decode(manifest_raw)
    require(manifest == cpu['harness_sources'], 'exact full tested source manifest')
    manifest_sha = harness['manifest']['sha256']
    original = raw(harness['original_actual_binding'])
    require(checkpoint_identity(decode(original)) == checkpoint_identity(cpu['checkpoint']), 'relocated checkpoint differs from actually validated artifacts')
    for role,phase in harness['roles'].items():
        require(type(phase) is dict and set(phase) == set(fields(role)), 'complete harness raw evidence fields')
        streams = {key:raw(item) for key,item in phase.items()}
        require(streams['status'] == b'0\n' and HELPER_SHA is not None
            and hashlib.sha256(streams['helper']).hexdigest() == HELPER_SHA, 'successful exact harness qualifier')
        argv = ['/bin/bash',D+'/owner/cpu-env-44g-emitter.sh','/usr/bin/python3','-I','-B',HELPER,role,manifest_sha]
        down.validate_outer(decode(streams['result']),argv)
        inner = decode(streams['inner'])
        expected = {'schema':'FerricNativeDownHarnessQualificationR1','accepted':True,'role':role,
            'manifest_sha256':manifest_sha,'source_sha256':manifest,'helper_sha256':HELPER_SHA,
            'returncode':0,'native_executed':False,'native_launch_admitted':False,
            'planning_increment_bytes':32*1024**2}
        extra = {'stage_before_bytes','stage_after_bytes'} | ({'validation','binding_sha256'} if role == 'actual' else {'argv','tests_passed'})
        require(type(inner) is dict and set(inner) == set(expected) | extra, 'closed harness inner receipt')
        current.exact(inner,expected,'exact successful harness source and role')
        lines = streams['stdout'].splitlines()
        require(lines and encoded(decode(lines[-1])) == encoded(inner), 'harness outer stdout binds inner')
        ceiling = 44*1024**3 - current.LIMITS['stage_reserve_bytes']
        require(type(inner['stage_before_bytes']) is int and type(inner['stage_after_bytes']) is int
            and 0 <= inner['stage_before_bytes'] <= ceiling - 32*1024**2
            and 0 <= inner['stage_after_bytes'] <= ceiling
            and inner['stage_after_bytes'] - inner['stage_before_bytes'] <= 32*1024**2, 'bounded harness growth')
        if role == 'actual':
            require(inner['binding_sha256'] == harness['original_actual_binding']['sha256']
                and encoded(inner['validation']) == encoded({'accepted':True,'ancestry_phases':24,'ancestry_custody_files':42,'controller_phases':19,
                    'native_executed':False,'native_launch_admitted':False,'performance_qualified':False}), 'actual complete checkpoint validation')
        else:
            pattern = 'test_cpu_binding_a009.py' if role == 'binding' else 'test_*.py'
            directory = INPUT + '/harness' + ('/measurement' if role == 'measurement' else '')
            command = ['/usr/bin/python3','-I','-B','-m','unittest','discover','-v','-s',directory,'-p',pattern]
            require(inner['argv'] == command and type(inner['tests_passed']) is int
                and inner['tests_passed'] == COUNTS[role], 'exact executed harness command and count')
            log = streams['role_stderr']
            require(re.findall(rb'^Ran (\d+) tests in [0-9.]+s$',log,re.M) == [str(COUNTS[role]).encode()]
                and re.search(rb'^OK$',log,re.M), 'raw unittest success and total')
    return manifest

def validate(cpu, build, read_raw, sources, artifacts):
    require(type(cpu) is dict and set(cpu) == {'schema','runtime_source_sha256','controller_source_sha256',
        'worker_sha256','controllers','checkpoint','controller_checkpoint','controller_source_role','harness','harness_sources'}
        and cpu['schema'] == 'FerricNativeDownCpuQualificationR1'
        and cpu['controller_source_role'] == 'authored-input', 'closed current CPU qualification')
    for name in ('runtime_source','controller_source','worker'):
        require(cpu[name+'_sha256'] == build[name]['sha256'] == artifacts[name], 'CPU qualification exact build artifact')
    require(cpu['controllers'] == {name:item['sha256'] for name,item in build['controllers'].items()} == artifacts['controllers'], 'exact A/B controller aliases')
    require(type(cpu['harness_sources']) is dict and set(cpu['harness_sources']) == set(sources), 'complete tested harness source roster')
    require(all(type(value) is str and re.fullmatch('[0-9a-f]{64}',value) for value in cpu['harness_sources'].values()), 'exact source hashes')
    validate_harness(cpu,read_raw)
    current.validate(cpu['checkpoint'],read_raw)
    down.validate(cpu['controller_checkpoint'],read_raw)
    require(cpu['controller_source_sha256'] == down.SOURCE_SHA
            and all(cpu['controllers'][mode+'-'+arm] == down.RELEASES[mode]
                    for mode in ('live','counters') for arm in ('A','B')), 'exact independently qualified down releases')
