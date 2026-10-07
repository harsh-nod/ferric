"""Pure replay of one instrumented registered-decode window; no launch authority."""
import hashlib
import json

GENERAL = set('commands command_ns full_currentness_checks full_currentness_ns '
    'operational_currentness_checks operational_currentness_ns kernel_admissions '
    'kernel_admission_ns dispatches dispatch_prepare_ns dispatch_publish_ns dispatch_wait_ns '
    'completion_polls reads read_bytes read_ns writes write_bytes write_ns'.split())
TOKEN = set('executions dispatches publications final_waits retirement_signals staging_ns '
    'kernarg_initialized_bytes'.split())
SETUP = {'schema':'FerricNativeGateUpDecodeBreakdownR1', 'performance_qualified':False,
    'latency_sample_admitted':False, 'requests':1, 'input_tokens':128, 'output_tokens':128}
SCOPES = {
    'scope':'Host-wall only; general counters overlap. Not GPU time or pure IPC. Pdelta includes the before-snapshot command, excludes the after-snapshot command.',
    'excludes':'First decode metadata uploads, cold transport plan and registration; setup, prefill, endpoint snapshot roundtrips and teardown.',
    'residual_scope':'Includes upstream graph construction, scheduler/JSON, instrumentation and inter-token gaps; signed, not clamped.'}
FIELDS = set('schema authority performance_qualified latency_sample_admitted complete worker_pid '
    'device_unique_id program epoch packets_per_execute start_frontier end_frontier started_monotonic_ns '
    'finished_monotonic_ns registered_decode_span_host_ns before after general_counter_delta token_counter_delta '
    'planner_host_ns executions reads writes execute_host_ns readback_host_ns metadata_write_host_ns '
    'warm_transport_planner_host_ns execute_minus_worker_scopes_host_ns span_residual_host_ns scope excludes residual_scope'.split())
MAX_STDERR = 256 * 1024


def require(ok, message):
    if not ok:
        raise ValueError(message)


def exact(actual, expected):
    require(type(actual) is dict and all(type(actual.get(k)) is type(v) and actual[k] == v
        for k,v in expected.items()), 'exact diagnostic metadata')


def integer(value, positive=False):
    require(type(value) is int and int(positive) <= value <= 2**64-1, 'bounded u64')
    return value


def decode(raw):
    def pairs(items):
        result = {}
        for key,value in items:
            require(key not in result, 'duplicate key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite value'))


def counters(value, fields):
    require(type(value) is dict and set(value) == fields, 'exact counter roster')
    for number in value.values():
        integer(number)
    return value


def delta(before, after, fields):
    a, b = counters(before, fields), counters(after, fields)
    require(all(b[k] >= a[k] for k in fields), 'regressed counter')
    return {k:b[k]-a[k] for k in fields}


def rows(value, count, fields):
    require(type(value) is list and len(value) == count, 'exact observation count')
    for row in value:
        require(type(row) is dict and set(row) == fields, 'exact observation fields')
        for number in row.values():
            integer(number)
    return value


def clean_exit(value):
    exact(value, {'cleanup_ok':True, 'child_reaped':True, 'owned_descendants_absent':True,
        'returncode':0, 'errors':[], 'term_sent':False, 'kill_sent':False})


def replay(raw, spec, setup, counter_replay):
    require(type(raw) is bytes and 0 < len(raw) <= MAX_STDERR and raw.endswith(b'\n'),
        'bounded newline-terminated stderr')
    lines = raw.splitlines(keepends=True)
    require(len(lines) == 3 and all(line.endswith(b'\n') for line in lines),
        'exact start/breakdown/end records')
    exact(setup.get('decode_diagnostic'),SETUP)
    require(set(setup['decode_diagnostic']) == set(SETUP), 'dedicated diagnostic setup')
    mechanism = counter_replay(lines[0] + lines[2], spec, setup)
    value = decode(lines[1])
    require(type(value) is dict and set(value) == FIELDS, 'closed breakdown schema')
    exact(value,SCOPES)
    pid = setup['worker_pids'][0]
    packets = {'A':652, 'B':724}[spec['arm']]
    exact(value, {'schema':SETUP['schema'], 'authority':'none', 'complete':True,
        'performance_qualified':False, 'latency_sample_admitted':False,
        'worker_pid':pid, 'device_unique_id':spec['device_unique_id'],
        'packets_per_execute':packets, 'start_frontier':2599, 'end_frontier':2599+127*packets})
    integer(value['epoch'])
    for key in ('program', 'started_monotonic_ns', 'finished_monotonic_ns',
                'registered_decode_span_host_ns'):
        integer(value[key], True)
    begin, end = value['started_monotonic_ns'], value['finished_monotonic_ns']
    require(end > begin and end-begin >= value['registered_decode_span_host_ns'], 'decode span clock')
    for ordinal, phase in enumerate(('before', 'after')):
        endpoint = value[phase]
        require(type(endpoint) is dict and set(endpoint) == {'general','token','snapshot_roundtrip_host_ns'},
            'exact diagnostic endpoint')
        integer(endpoint['snapshot_roundtrip_host_ns'], True)
        general = endpoint['general']
        require(type(general) is dict and set(general) == {'schema','authority','performance_qualified',
            'scope','process_id','device_unique_id','rank','ordinal','counters'},'closed snapshot wrapper')
        exact(general, {'schema':'FerricRuntimeDiagnosticSnapshotV1', 'authority':'none',
            'performance_qualified':False, 'scope':'cumulative overlapping worker host-wall counters, not GPU timestamps',
            'process_id':pid, 'device_unique_id':spec['device_unique_id'], 'rank':0, 'ordinal':ordinal})
        counters(general['counters'], GENERAL)
        counters(endpoint['token'], TOKEN)
    before, after = value['before'], value['after']
    general = delta(before['general']['counters'], after['general']['counters'], GENERAL)
    token = delta(before['token'], after['token'], TOKEN)
    require(counters(value['general_counter_delta'], GENERAL) == general
        and counters(value['token_counter_delta'], TOKEN) == token, 'recomputed counter deltas')
    exact(before['token'], {'executions':4,'dispatches':2596,'publications':4,
        'final_waits':4,'retirement_signals':2596})
    require(after['token'] == mechanism['end']['counters'], 'final token endpoint differs from close')
    exact(general, {'dispatches':127*packets,'reads':127,'read_bytes':508,'writes':630})
    exact(token, {'executions':127,'publications':127,'final_waits':127,
        'dispatches':127*packets,'retirement_signals':127*packets})
    execution = rows(value['executions'],127,
        {'ordinal','frontier','host_ns','started_monotonic_ns','finished_monotonic_ns'})
    reads = rows(value['reads'],127,{'bytes','host_ns','finished_monotonic_ns'})
    writes = rows(value['writes'],630,{'bytes','host_ns','finished_monotonic_ns'})
    planners = rows(value['planner_host_ns'],127,{'host_ns'})
    previous = begin
    for index,(run,read) in enumerate(zip(execution, reads)):
        exact(run, {'ordinal':index,'frontier':2599+(index+1)*packets})
        exact(read, {'bytes':4})
        require(previous <= run['started_monotonic_ns'] < run['finished_monotonic_ns']
            <= read['finished_monotonic_ns'] <= end, 'execute/read chronological order')
        require(run['finished_monotonic_ns']-run['started_monotonic_ns'] >= run['host_ns'],
            'execute host duration exceeds clock interval')
        require(read['host_ns'] <= read['finished_monotonic_ns']-run['finished_monotonic_ns'],
            'read duration exceeds available interval')
        if index:
            require(planners[index]['host_ns'] <= run['started_monotonic_ns']-writes[index*5-1]['finished_monotonic_ns'],
                'warm planner duration exceeds available interval')
        previous = read['finished_monotonic_ns']
        if index < 126:
            for write,size in zip(writes[index*5:(index+1)*5],(4,2048,256,256,4)):
                exact(write,{'bytes':size})
                require(previous <= write['finished_monotonic_ns'] <= execution[index+1]['started_monotonic_ns'],
                    'five metadata writes between validated token steps')
                require(write['host_ns'] <= write['finished_monotonic_ns']-previous,
                    'write duration exceeds available interval')
                previous = write['finished_monotonic_ns']
    require(sum(row['bytes'] for row in writes) == general['write_bytes'], 'write byte total')
    totals = {'execute_host_ns':sum(r['host_ns'] for r in execution),
        'readback_host_ns':sum(r['host_ns'] for r in reads),
        'metadata_write_host_ns':sum(r['host_ns'] for r in writes),
        'warm_transport_planner_host_ns':sum(r['host_ns'] for r in planners[1:])}
    exact(value, totals)
    worker_ns = sum(general[k] for k in ('dispatch_prepare_ns','dispatch_publish_ns','dispatch_wait_ns')) + token['staging_ns']
    residuals = {'execute_minus_worker_scopes_host_ns':totals['execute_host_ns']-worker_ns,
        'span_residual_host_ns':value['registered_decode_span_host_ns']-sum(totals.values())}
    exact(value, residuals)
    offset, bindings = 0, []
    for line in lines:
        bindings.append({'offset':offset,'bytes':len(line),'sha256':hashlib.sha256(line).hexdigest()})
        offset += len(line)
    return {'schema':'FerricNativeDecodeReplayR1','accepted':True,'mechanism':mechanism,
        'breakdown':value,'stderr_sha256':hashlib.sha256(raw).hexdigest(),'line_bindings':bindings,
        'performance_qualified':False,'latency_sample_admitted':False,
        'scope':'One cold instrumented request per arm; host wall time only, not TPOT or GPU time.'}
