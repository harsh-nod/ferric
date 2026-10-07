"""Validate the exact retained da6b/a009 CPU checkpoint, without native admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat

CATALOG_SHA = '4af074ac6a1867aeb73d4f64e965514d8b5e11798ccb86cf51adf5ceea39d3a2'
PROFILE = 'FerricCpuFourCore42GiBEmitterV1'
LIMITS = {'duration_seconds':1200, 'individual_file_bytes':536870912, 'kill_wait_seconds':3,
    'log_bytes':33554432, 'memory_available_bytes':137438953472, 'observed_rss_bytes':8589934592,
    'root_free_bytes':23622320128, 'shm_free_bytes':34359738368, 'stage_bytes':45097156608,
    'stage_reserve_bytes':536870912, 'term_grace_seconds':10}
ROLES = {
    'runtime':('prepare','format','clippy','tests','default','worker'),
    'model':('prepare','library','worker','profile','clippy','source-policy','default','union','release-live','release-counters'),
    'actual':('image','capture-control','capture-candidate','abi-control','abi-candidate','profile-actual','full-live','full-counters'),
}
FIELDS = {'status','result','stdout','stderr','inner','role_stdout','role_stderr','helper'}
TEST_COUNTS = {
    'runtime-tests':[[748,0,3,0,0],[9,0,0,0,0]], 'runtime-default':[[477,0,1,0,0]],
    'model-library':[[11,0,1,0,540]], 'model-worker':[[6,0,0,0,168]],
    'model-profile':[[4,0,1,0,169]], 'model-source-policy':[[58,0,0,0,0]],
    'actual-image':[[1,0,0,0,551]],
    'actual-capture-control':[[1,0,0,0,551]], 'actual-capture-candidate':[[1,0,0,0,551]],
    'actual-abi-control':[[1,0,0,0,173]], 'actual-abi-candidate':[[1,0,0,0,173]],
    'actual-profile-actual':[[1,0,0,0,173]],
    'actual-full-live':[[156,0,18,0,0]], 'actual-full-counters':[[156,0,18,0,0]],
}

def require(value, message):
    if not value:
        raise ValueError(message)

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()

def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=lambda _: require(False,'nonfinite JSON'))

def file_bytes(path, maximum=64 * 1024**2):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical evidence path')
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= maximum, 'bounded regular evidence')
        with os.fdopen(descriptor, 'rb', closefd=False) as source:
            raw = source.read(maximum + 1)
        after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    require(len(raw) == before.st_size and all(getattr(before,key) == getattr(after,key) == getattr(path.lstat(),key)
        for key in ('st_dev','st_ino','st_size','st_mtime_ns','st_ctime_ns')), 'evidence changed while reading')
    return raw

def catalog():
    raw = file_bytes(Path(__file__).with_name('cpu-checkpoint-a009.json').resolve(), 2 * 1024**2)
    require(CATALOG_SHA is not None and digest(raw) == CATALOG_SHA, 'reviewed a009 catalog required')
    value = decode(raw)
    require(set(value) == {'schema','runtime_revision','phases','custody','native_qualified'}
        and value['schema'] == 'FerricGateUpAuditedCpuCatalogA009'
        and value['runtime_revision'] == 'da6b561c5a3f12acc5b0e6da74c808273e728710'
        and value['native_qualified'] is False, 'closed CPU-only catalog')
    require(set(value['phases']) == {group + '-' + role for group,roles in ROLES.items() for role in roles}, 'complete 24-role catalog')
    return value

def bindings(expected):
    return {
        'schema':'FerricGateUpCurrentCpuBindingA009',
        'catalog_sha256':CATALOG_SHA,
        'phases':{name:{key:{field:item[field] for field in ('path','sha256')}
            for key,item in row['files'].items()} for name,row in expected['phases'].items()},
        'custody':{path:{'path':path,'sha256':sha} for path,sha in expected['custody'].items()},
    }

def bound_bytes(item, expected_sha, read_raw, maximum=64 * 1024**2):
    require(type(item) is dict and set(item) == {'path','sha256'} and type(item['path']) is str
        and Path(item['path']).is_absolute() and item['sha256'] == expected_sha
        and re.fullmatch('[0-9a-f]{64}', expected_sha), 'closed exact file binding')
    raw = read_raw(item['path'], maximum)
    require(type(raw) is bytes and len(raw) <= maximum and digest(raw) == expected_sha, 'bound evidence hash differs')
    return raw

def exact(actual, expected, label):
    require(type(actual) is dict and all(type(actual.get(k)) is type(v) and actual[k] == v for k,v in expected.items()), label)

def validate_outer(result, spec):
    exact(result, {'status':0,'reason':'completed','returncode':0,'cleanup_ok':True,'child_reaped':True,
        'errors':[],'term_sent':False,'kill_sent':False,'log_limit_exceeded':False,'profile':PROFILE,
        'cpus':[0,1,2,3],'nice':19,'build_jobs':4,'rust_test_threads':1,'argv':spec['outer_argv']}, 'unclean or mismatched outer phase')
    require(result.get('limits') == LIMITS and all(type(v) is int for v in result['limits'].values()), 'fixed G42 resource limits')
    require(all(type(v) is int for v in result['cpus']), 'integer CPU placement')
    env = {'CARGO_BUILD_JOBS':'4','RUST_TEST_THREADS':'1','FERRIC_CPU_PROFILE':PROFILE,
        **{k:'-1' for k in ('CUDA_VISIBLE_DEVICES','HIP_VISIBLE_DEVICES','HSA_VISIBLE_DEVICES','ROCR_VISIBLE_DEVICES')}}
    exact(result.get('launch_environment'), env, 'GPU visibility must remain masked')
    require(type(result.get('peak_observed_rss_bytes')) is int and 0 <= result['peak_observed_rss_bytes'] <= LIMITS['observed_rss_bytes'], 'bounded observed RSS')
    for name in ('admission','final_resources'):
        resources = result.get(name, {})
        require(all(type(resources.get(k)) is int and resources[k] >= LIMITS[k]
            for k in ('memory_available_bytes','root_free_bytes','shm_free_bytes')), 'CPU host resource floor')
        require(type(resources.get('stage_bytes')) is int and 0 <= resources['stage_bytes'] <= LIMITS['stage_bytes'] - LIMITS['stage_reserve_bytes'], 'CPU stage ceiling')

def validate_inner(name, inner, spec, streams):
    excluded = {'source_before','source_after','stage_before_bytes','stage_after_bytes'}
    fixed = {k:v for k,v in inner.items() if k not in excluded}
    require(encoded(fixed) == encoded(spec['inner_fixed']), 'exact inner role, command, source and ELF identities')
    exact(inner, {'accepted':True,'returncode':0,'native_executed':False}, 'successful CPU-only inner phase')
    require({k:digest(encoded(inner[k])) for k in ('source_before','source_after') if k in inner} == spec['source_rosters'], 'exact qualified source rosters')
    allowance = inner['planning_increment_bytes']
    ceiling = LIMITS['stage_bytes'] - LIMITS['stage_reserve_bytes']
    require(type(allowance) is int and allowance > 0 and type(inner.get('stage_before_bytes')) is int
        and type(inner.get('stage_after_bytes')) is int and 0 <= inner['stage_before_bytes'] <= ceiling - allowance
        and 0 <= inner['stage_after_bytes'] <= ceiling
        and inner['stage_after_bytes'] - inner['stage_before_bytes'] <= allowance, 'bounded inner planning increment')
    omit = {'source_before','source_after'} if spec['group'] == 'model' else ({'source_after'} if spec['group'] == 'runtime' else set())
    lines = streams['stdout'].splitlines()
    require(lines and encoded(decode(lines[-1])) == encoded({k:v for k,v in inner.items() if k not in omit}), 'outer stdout must bind projected inner receipt')
    counts = [[int(x) for x in row] for row in re.findall(rb'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', streams['role_stdout'], re.M)]
    require(counts == TEST_COUNTS.get(name, []) == spec['test_counts'], 'exact raw executed test counts')
    require(digest(streams['helper']) == inner['helper_sha256'], 'exact qualifier helper')

def validate(value, read_raw=file_bytes):
    expected = catalog()
    require(type(value) is dict and set(value) == {'schema','catalog_sha256','phases','custody'}
        and value['schema'] == 'FerricGateUpCurrentCpuBindingA009' and value['catalog_sha256'] == CATALOG_SHA, 'closed a009 checkpoint binding')
    require(type(value['phases']) is dict and set(value['phases']) == set(expected['phases']), 'complete CPU phase roster')
    require(type(value['custody']) is dict and set(value['custody']) == set(expected['custody']), 'complete external artifact custody')
    for name, spec in expected['phases'].items():
        phase = value['phases'][name]
        require(type(phase) is dict and set(phase) == FIELDS == set(spec['files']), 'closed phase raw evidence fields')
        streams = {key:bound_bytes(phase[key], row['sha256'], read_raw, LIMITS['log_bytes']) for key,row in spec['files'].items()}
        require(all(len(streams[key]) == row['bytes'] for key,row in spec['files'].items()), 'exact retained evidence lengths')
        require(streams['status'] == b'0\n', 'raw zero exit status')
        validate_outer(decode(streams['result']), spec)
        validate_inner(name, decode(streams['inner']), spec, streams)
    for path, sha in expected['custody'].items():
        bound_bytes(value['custody'][path], sha, read_raw)
    return {'accepted':True,'phases':24,'custody_files':len(expected['custody']),
        'native_executed':False,'native_launch_admitted':False,'performance_qualified':False}
