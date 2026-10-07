"""Portable replay of the retained native-down CPU checkpoint; no native launch."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import re
import tarfile

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('down_ancestry', HERE/'cpu_binding_a009.py')
ancestry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ancestry)
require, decode, encoded, digest = ancestry.require, ancestry.decode, ancestry.encoded, ancestry.digest
D = '/tmp/ferric-v16-emitter-b95a642-r1'
O = 'native-down-model-a001/'
I = 'inputs/native-down-model-a001/'
ARCHIVE_SHA = 'bf0b4b958c23e741060b5bf36d021b4e44240faef5d126546015254d1638dee0'
RECEIPT_SHA = '2041ac2a3dee134c21047c3c2e11e6d6fc0021c7bab0e8345eb5a9a4cb18a0d0'
AUDIT_SHA = '43fd89c7066827c7c15d058e142fe1501dfeba6f6fc94a981ad5ffb4780f844b'
AUDITOR_SHA = '862d865ba892f5c3d03dc67f5cada527bf288ef0c135db856bed27d42e8a3718'
SOURCE_SHA = '9e53fdc9f6c9e0690afac86ccaeed83ce000ebaa772f0deb40a6c57df0fe0b88'
MANIFEST_SHA = 'e60eedae6a4f029cef093e0f0dae51757d5c380451221fa010451a558ffb9337'
RELEASES = {'live':'f1fca91663c1fa682ada28fd79ef3e119d7de9845dfb1d8fb0c860ca0ffada7b',
            'counters':'c64cfab44f1ba50d8150c646a3934000de7c19f5e97647902d72521644581476'}
QUALIFIERS = {'a002':'39bfba5a1b712af99a3144a0f879442462ab3391f67c6b1d2e25a673ab0a1b20',
              'a003':'1e3d272eb92d84c4d7afdf22c933960154aebf1f4a96bd9f9e1de96f029efb3f',
              'a004':'108c76088be750e40f7e21048b13a1786fe4cf63456c774f67e45596e9dd9ebe'}
ACTUAL_SHA = '531bddea1604ef9cbe9de0a8c5863e3e7747698cb03a6a6e951c056d4b6a0a6c'
TOTALS = {'library':[10,0,1,0,540], 'worker':[6,0,0,0,168], 'profile':[4,0,1,0,169],
          'source-policy':[58,0,0,0,0], 'full-live':[156,0,18,0,0], 'full-counters':[156,0,18,0,0]}
COHORTS = {'library':'728b818d158ba8122f48b36778bc1037bed2a030ab8595742fd389ba67f08eba',
           'worker':'e0a9dc0e1535f5a84df29da9cc7c43ac6ffbb6a03f2e87423b4aed712c286389',
           'profile':'f4fe8fb1ee627a2e0ec0e7dc70494c661b9377e8b19bf3673a514e4b283b29a7',
           'source-policy':'9947016b34edac19d4c71c1c6cf618ccf2d9240ce45a5355c7c3f6e8dbf6478f',
           'full-live':'3e2c397607d31be29d94d04072fe628b6193d0a7b68342d3400a05d37f963d90',
           'full-counters':'3e2c397607d31be29d94d04072fe628b6193d0a7b68342d3400a05d37f963d90'}
ACTUAL = {'image':'splitk_actual_component_image_admits_exact_physical_abi_and_resources',
          'capture-control':'capture_down_control652_actual_driver_graphs',
          'capture-candidate':'capture_down_native688_actual_driver_graphs',
          'abi-control':'actual_images_pack_down_control652_and_reuse180_slots',
          'abi-candidate':'actual_images_pack_down_native688_and_reuse180_slots',
          'profile-actual':'native_down_actual_image_and_roster_open_for_both_arms'}
ROLES = ('fmt','metadata',*TOTALS,'clippy','default','union','release-live','release-counters',
         *('actual-'+role for role in ACTUAL))
FILES = {
    'archive':('native-down-cpu-a002-retained/evidence.tar.gz',ARCHIVE_SHA),
    'receipt':('native-down-cpu-a002-retained/receipt.json',RECEIPT_SHA),
    'result':('results/pages-native-down-retain-a002/result.json','3375eb99f1851d9f41d85285f77c857a0ab5ef75bc9424ca9a9ca15eb688fcd4'),
    'status':('results/pages-native-down-retain-a002/exit.status','9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa'),
    'stdout':('results/pages-native-down-retain-a002/stdout','f03f4d7f6c9a5534472013d41acfe076d1104d473ede057fed07164d27879e71'),
    'stderr':('results/pages-native-down-retain-a002/stderr','e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'),
}
FOOTER = rb'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s$'


def bindings():
    return {'schema':'FerricNativeDownRetainedCpuBindingR1',
            'files':{key:{'path':D+'/'+path,'sha256':pin} for key,(path,pin) in FILES.items()}}


def ancestry_path(path):
    old = {'live':'60bf47551d1ebdfc91f49b529dab8211e3f860a07bdd05921e642b72d505e9a4',
           'counters':'fa2049edf8daed70ba268be1b6799f5a4ba42e106e27025f776509876f371231'}
    for kind,pin in old.items():
        if path == D+'/target-fence-client-a001/release/ferric-qwen3-prefill-width-native-'+kind:
            return D+'/gate-up-native-transport-a003/payload/objects/'+pin
    return path


def relocate_ancestry(value):
    for original,item in value['custody'].items():
        item['path'] = ancestry_path(original)
    return value


def iter_bindings(value):
    require(type(value) is dict and set(value) == {'schema','files'}
            and value['schema'] == 'FerricNativeDownRetainedCpuBindingR1'
            and type(value['files']) is dict and set(value['files']) == set(FILES), 'closed down CPU binding')
    yield from sorted(value['files'].items())


def validate_outer(row, argv, gib=44):
    profile = 'FerricCpuFourCore'+str(gib)+'GiBEmitterV1'
    limits = dict(ancestry.LIMITS,stage_bytes=gib*1024**3)
    ancestry.exact(row, {'status':0,'reason':'completed','returncode':0,'cleanup_ok':True,'child_reaped':True,
        'errors':[],'term_sent':False,'kill_sent':False,'log_limit_exceeded':False,'profile':profile,
        'cpus':[0,1,2,3],'nice':19,'build_jobs':4,'rust_test_threads':1,'argv':argv}, 'clean exact down CPU outer')
    require(row['limits'] == limits and all(type(v) is int for v in row['limits'].values()), 'fixed CPU limits')
    require(all(type(value) is int for value in row['cpus']), 'integer CPU placement')
    ancestry.exact(row['launch_environment'], {'CARGO_BUILD_JOBS':'4','RUST_TEST_THREADS':'1','FERRIC_CPU_PROFILE':profile,
        **{key:'-1' for key in ('CUDA_VISIBLE_DEVICES','HIP_VISIBLE_DEVICES','HSA_VISIBLE_DEVICES','ROCR_VISIBLE_DEVICES')}},
        'masked GPU visibility')
    require(type(row['peak_observed_rss_bytes']) is int and 0 <= row['peak_observed_rss_bytes'] <= limits['observed_rss_bytes'], 'bounded RSS')
    for field in ('admission','final_resources'):
        resource = row[field]
        require(all(type(resource[key]) is int and resource[key] >= limits[key]
                    for key in ('root_free_bytes','shm_free_bytes','memory_available_bytes')), 'host resource floors')
        require(type(resource['stage_bytes']) is int and 0 <= resource['stage_bytes'] <= limits['stage_bytes']-limits['stage_reserve_bytes'], 'stage reserve')


def archive_members(raw, roster):
    require(type(roster) is dict and 0 < len(roster) <= 2500, 'bounded archive roster')
    members, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw),mode='r:gz') as archive:
        for member in archive:
            name = member.name
            path = Path(name)
            require(member.isfile() and not member.issparse() and not path.is_absolute()
                    and '..' not in path.parts and str(path) == name and name not in members and name in roster,
                    'unique canonical regular archive member')
            row = roster[name]
            require(set(row) == {'sha256','bytes','mode'} and type(member.size) is int
                    and member.size == row['bytes'] and 0 <= member.size < 64*1024**2
                    and member.mode == row['mode'], 'exact bounded member metadata')
            total += member.size
            require(total < 256*1024**2, 'bounded uncompressed closure')
            content = archive.extractfile(member).read(member.size+1)
            require(len(content) == member.size and digest(content) == row['sha256'], 'member bytes')
            members[name] = content
    require(set(members) == set(roster), 'complete archive closure')
    return members


def replay(members):
    def get(name):
        require(name in members, 'missing retained evidence: '+name)
        return members[name]
    def original(path):
        require(type(path) is str and path.startswith(D+'/'), 'original private evidence namespace')
        return get(path[len(D)+1:])
    audit_raw = get(O+'exact-audit-a002.json')
    require(digest(audit_raw) == AUDIT_SHA, 'exact reviewed CPU audit')
    audit = decode(audit_raw)
    require(set(audit['phases']) == set(ROLES) and audit['accepted'] is True
            and audit['native_executed'] is False and audit['performance_qualified'] is False
            and audit['serving_qualified'] is False and audit['helper_sha256'] == AUDITOR_SHA,
            'complete CPU-only audited role roster')
    require(digest(get(I+'audit_cpu_a002.py')) == AUDITOR_SHA, 'exact frozen auditor')
    for path,row in audit['files'].items():
        raw = original(path)
        require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'audited member identity')
    manifest = decode(get(I+'source-a002.json'))
    require(digest(get(I+'source-a002.json')) == MANIFEST_SHA
            and digest(get(I+'source-a002.tar.gz')) == audit['source_archive_sha256'] == SOURCE_SHA,
            'authored source identity')
    formatted = audit['formatted_source']
    require(len(formatted) == 1441 and set(formatted) == set(manifest['files']), 'full formatted source roster')
    require({name[len(O+'source/'):]:digest(raw) for name,raw in members.items() if name.startswith(O+'source/')} == formatted,
            'retained formatted source closure')
    fmt = decode(get(O+'fmt-a002/receipt.json'))
    require(fmt['source_before'] == {name:row['sha256'] for name,row in manifest['files'].items()}
            and fmt['source_after'] == formatted, 'authorship through formatting')
    outer_root = 'results/pages-native-down-exact-audit-a002/'
    validate_outer(decode(get(outer_root+'result.json')),
        ['/bin/bash',D+'/owner/cpu-env-44g-emitter.sh','/usr/bin/python3','-I','-B',D+'/'+I+'audit_cpu_a002.py'])
    require(get(outer_root+'exit.status') == b'0\n' and decode(get(outer_root+'stdout')) ==
            {'accepted':True,'phases':19,'files':len(audit['files']),'audit_sha256':AUDIT_SHA,'native_executed':False},
            'raw audit result binding')
    for name in ROLES:
        actual = name.startswith('actual-')
        role = name.removeprefix('actual-')
        attempt = 'a002' if role in ('fmt','metadata') else 'a003' if role in ('library','worker','profile','clippy') else 'a004'
        gib = 42 if attempt == 'a002' else 44
        helper = 'qualify_actual.py' if actual else 'qualify-'+attempt+'.py'
        helper_sha = ACTUAL_SHA if actual else QUALIFIERS[attempt]
        directory = O+('actual-'+role if actual else role+'-'+attempt)+'/'
        outer_dir = 'results/pages-native-down-'+name+'-'+('a001' if actual else attempt)+'/'
        require(audit['phases'][name] == {'inner':D+'/'+directory+'receipt.json','outer':D+'/'+outer_dir+'result.json'},
                'closed successful phase attempt')
        inner = decode(get(directory+'receipt.json'))
        require(type(inner['returncode']) is int and inner['returncode'] == 0 and inner['accepted'] is True
                and inner['role'] == role and inner['source_archive_sha256'] == SOURCE_SHA
                and inner['native_executed'] is False and inner['helper_sha256'] == helper_sha
                and digest(get(I+helper)) == helper_sha, 'exact successful inner role/source/helper')
        validate_outer(decode(get(outer_dir+'result.json')),
            ['/bin/bash',D+'/owner/cpu-env-'+str(gib)+'g-emitter.sh','/usr/bin/python3','-I','-B',D+'/'+I+helper,role],gib)
        require(get(outer_dir+'exit.status') == b'0\n' and decode(get(outer_dir+'stdout').splitlines()[-1]) ==
                {key:value for key,value in inner.items() if key not in ('source_before','source_after','passing_tests')},
                'raw successful inner projection')
        for key in ('source_before','source_after'):
            if key in inner and not (role == 'fmt' and key == 'source_before'):
                require(inner[key] == formatted, 'unchanged qualified source')
        if name in TOTALS or actual:
            stdout = get(directory+'stdout')
            totals = [list(map(int,row)) for row in re.findall(FOOTER,stdout,re.M)]
            wanted = [1,0,0,0,550 if role in ('image','capture-control','capture-candidate') else 173] if actual else TOTALS[name]
            require(totals == [wanted] and inner['test_totals'] == wanted, 'exact raw test footer')
            if actual:
                require(inner['argv'] == [inner['test_binary']['path'],ACTUAL[role],'--ignored','--nocapture','--test-threads=1']
                        and len(re.findall(rb'^test [A-Za-z0-9_:]*'+ACTUAL[role].encode()+rb' \.\.\. ',stdout,re.M)) == 1,
                        'exact actual test')
            else:
                rows = re.findall(rb'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored)(?:, [^\n]*)?$',stdout,re.M)
                cohort = {key:sorted(n.decode() for n,status in rows if status == label)
                          for key,label in [('passed',b'ok'),('ignored',b'ignored')]}
                require(digest(json.dumps(cohort,sort_keys=True,separators=(',',':')).encode()) == COHORTS[name], 'exact test cohort')
        for key in ('test_binary','release_binary'):
            if key in inner:
                row = inner[key]
                raw = original(row['path'])
                require(raw.startswith(b'\x7fELF') and len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'qualified ELF')
        for path,pin in inner.get('inputs',{}).items():
            require(digest(original(path)) == pin, 'actual immutable input')
        for key in ('graph','roster'):
            if key in inner:
                require(digest(get(directory+key+'.json')) == inner[key]['sha256'], 'actual output')
        if role.startswith('release-'):
            kind = role.removeprefix('release-')
            require(inner['release_binary']['sha256'] == RELEASES[kind], 'new controller release identity')
        ceiling = gib*1024**3-512*1024**2
        before,after,allowance = (inner[key] for key in ('stage_before_bytes','stage_after_bytes','planning_increment_bytes'))
        require(all(type(value) is int for value in (before,after,allowance)) and allowance > 0
                and 0 <= before < ceiling-allowance and 0 <= after < ceiling and after-before <= allowance, 'phase growth')
    return {'accepted':True,'phases':19,'formatted_source_files':1441,'controller_source_role':'authored-input',
            'native_executed':False,'performance_qualified':False}


def validate(value, read_raw=ancestry.file_bytes):
    rows = dict(iter_bindings(value))
    raw = {key:ancestry.bound_bytes(rows[key],pin,read_raw,256*1024**2 if key == 'archive' else 2*1024**2)
           for key,(_,pin) in FILES.items()}
    receipt = decode(raw['receipt'])
    require(receipt['accepted'] is True and receipt['all_archive_members_verified'] is True
            and receipt['archive_sha256'] == ARCHIVE_SHA and receipt['archive_bytes'] == len(raw['archive'])
            and receipt['audit_sha256'] == AUDIT_SHA and receipt['native_executed'] is False
            and receipt['performance_qualified'] is False, 'exact retained CPU-only archive')
    validate_outer(decode(raw['result']), ['/bin/bash',D+'/owner/cpu-env-44g-emitter.sh',
        '/usr/bin/python3','-I','-B',D+'/'+I+'retain_cpu_a002.py'])
    require(raw['status'] == b'0\n' and decode(raw['stdout']) == {key:value for key,value in receipt.items() if key != 'files'},
            'clean retained outer custody')
    members = archive_members(raw['archive'],receipt['files'])
    require(digest(members[I+'retain_cpu_a002.py']) == '551f40dba756e3cd32c8464513e96d02c0545e22fa498d2673bbd10e608348f0',
            'exact retention helper')
    return replay(members)
