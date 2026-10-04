"""Closed genuine layer-zero capture and current-output join, not arithmetic acceptance."""
import layer_validation as V

PROFILE = 'prefix284_mlp548'
SCHEMA = 'FerricFinitePrefixLayerCaptureObservationV1'
NAMES = ('request.json', 'candidate-registration.json', 'candidate-program.json',
    'candidate-uploads.json', 'candidate-bootstrap.json', 'candidate-request-1.json',
    'candidate-response-1.json', 'candidate-request-2.json', 'candidate-response-2.json',
    'candidate-capture.bin', 'candidate-stderr.bin')
BODY = set(NAMES)
FALSE = ('paired_comparison_performed', 'numerical_acceptance', 'performance_claim',
         'production_authority', 'full_forward')
HIDDEN_SHA = 'faa56202578a3d3497bbe779137736439957e473775bd6f677dbce466a9d6979'


def child_marker(raw, pid):
    V.require(type(raw) is bytes and type(pid) is int and 0 < pid <= 0xffffffff,
              'actual one-child marker input')
    expected = (f'finite prefix layer candidate child pid={pid} pgid={pid}; '
                'setup not acknowledged\n').encode()
    V.require(raw == expected, 'exact single candidate child marker, no other parent stderr')


def validate(summary, files, request, expected_hidden):
    V.require(type(summary) is bytes and type(files) is dict and set(files) == BODY
        and all(type(value) is bytes for value in files.values()), 'exact eleven capture body buffers')
    V.require(len(summary) <= 65536 and len(summary) + sum(map(len, files.values())) <= V.LIMIT,
              'unchanged aggregate capture bound')
    V.require(type(expected_hidden) is bytes and len(expected_hidden) == 8192
        and V.sha(expected_hidden).hex() == HIDDEN_SHA, 'authenticated current TF4 layer-zero hidden slice')
    V.finite(expected_hidden, 2)
    o = V.parse(summary)
    V.keys(o, 'schema request run stages files native_attempts retries completed_layers native_closed '
        'gpu_execution ' + ' '.join(FALSE))
    V.require(o['schema'] == SCHEMA and o['request'] == request
        and V.parse(files['request.json']) == request
        and request['schema'] == 'FerricFinitePrefixLayerCaptureRequestV1'
        and V.uint(o['native_attempts']) == 1 and V.uint(o['retries']) == 0
        and V.uint(o['completed_layers']) == 1 and o['native_closed'] is True
        and o['gpu_execution'] is True and all(o[key] is False for key in FALSE),
        'candidate-only genuine layer observation scope')
    V.require(type(o['files']) is list and len(o['files']) == len(NAMES), 'exact ordered body manifest')
    for name, row in zip(NAMES, o['files']):
        V.keys(row, 'name bytes sha256')
        V.uint(row['bytes'], V.LIMIT); V.digest(row['sha256'])
        V.require(row == dict(name=name, **V.part(files[name])), 'actual ordered capture body digest')
    V.require(files['candidate-stderr.bin'] == b'', 'closed worker has no stderr')
    run = o['run']
    V.keys(run, 'child_pid bootstrap profile_sha256 setup_commands close child_exit_zero process_group_absent')
    pid = V.uint(run['child_pid'], 0xffffffff)
    V.require(pid > 0 and run['child_exit_zero'] is True and run['process_group_absent'] is True
        and V.uint(run['setup_commands']) > 0, 'one naturally closed and reaped worker assertion')
    b = V.parse(files['candidate-bootstrap.json'])
    V.require(b == run['bootstrap'], 'actual bootstrap bytes')
    V.bootstrap(b, PROFILE, request, files, 'candidate', pid)
    V.require(run['profile_sha256'] == V.profile_digest(b), 'actual scoped input/profile digest')
    for number, command in ((1, 'run'), (2, 'close')):
        q = V.parse(files[f'candidate-request-{number}.json'])
        r = V.parse(files[f'candidate-response-{number}.json'])
        V.require(q == dict(protocol=1, id=number, profile_sha256=run['profile_sha256'], command=command),
                  'exact Run1/Close2 request')
        V.keys(r, 'protocol id profile_sha256 profile native_closed completed_layers control capture '
            'gpu_execution numerical_acceptance performance_claim production_authority')
        V.require(V.uint(r['protocol']) == 1 and V.uint(r['id']) == number
            and r['profile_sha256'] == run['profile_sha256'] and r['profile'] == PROFILE
            and V.uint(r['completed_layers']) == 1 and r['gpu_execution'] is True
            and all(r[key] is False for key in V.FALSE), 'actual closed response scope')
        if number == 1:
            V.require(r['native_closed'] is False and r['control'] is None and r['capture'] is None,
                      'Run cannot release captures')
        else:
            V.require(r == run['close'] and r['native_closed'] is True
                and r['capture'] == V.part(files['candidate-capture.bin']), 'Close-only actual capture')
            V.control(r['control'], PROFILE)
    rows = V.capture(files['candidate-capture.bin'], b['input'])
    stages, offset = [], 0
    for index, raw in enumerate(rows):
        name, size, width = V.STAGES[index % 14]
        stages.append(dict(rank=index // 14, stage=name, offset=offset, bytes=size,
            elements=size // width, element_bytes=width, sha256=list(V.sha(raw))))
        offset += size
    V.require(type(o['stages']) is list and len(o['stages']) == 28, 'exact stage manifest extent')
    for row in o['stages']:
        V.keys(row, 'rank stage offset bytes elements element_bytes sha256')
        for key in ('rank', 'offset', 'bytes', 'elements', 'element_bytes'): V.uint(row[key])
        V.digest(row['sha256'])
    V.require(o['stages'] == stages and offset == V.CAPTURE_BYTES,
              'all twenty-eight actual stage partitions and hashes')
    V.require(rows[13] == rows[27] == expected_hidden, 'both current native final-hidden rows match actual TF4')
    return dict(schema='ferric-p228-layer0-native-capture-checked-v1', captured_arrays=28,
        capture_bytes=V.CAPTURE_BYTES, closed_child_pids=[pid], stages=stages,
        layer=0, generation=1, position=0, token=9112, full_kv_checked=True,
        current_tf4_hidden_equal=True, current_tf4_hidden_sha256=HIDDEN_SHA,
        native_closed=True, recorded_child_exit_zero=True, recorded_child_group_absent=True,
        paired_comparison_performed=False, numerical_acceptance=False,
        independent_framework_comparison_performed=False, full_model_correctness=False,
        performance_claim=False, production_authority=False, full_forward=False)
