"""Additional-image layer0 capture; unchanged upstreams, no arithmetic acceptance."""
import layer_validation as V

PROFILE = 'prefix284_mlp548'
SCHEMA = 'FerricFiniteProjectionResidualLayerCaptureObservationV1'
NAMES = ('request.json', 'candidate-registration.json', 'candidate-program.json',
    'candidate-uploads.json', 'candidate-bootstrap.json', 'candidate-request-1.json',
    'candidate-response-1.json', 'candidate-request-2.json', 'candidate-response-2.json',
    'candidate-capture.bin', 'candidate-stderr.bin')
BODY = set(NAMES)
FALSE = ('paired_comparison_performed', 'numerical_acceptance', 'performance_claim',
         'production_authority', 'full_forward')
IMAGE_SHA = '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25'
ARITHMETIC = b'ordered-fp32-tp2-bf16-projection-then-bf16-residual-v1'


def projection_digest(outer):
    import hashlib
    return list(hashlib.sha256(b'ferric-projection-residual-layer-closed-v1\0'
        + ARITHMETIC + bytes(V.profile_digest(outer['layer']))
        + V.digest(outer['projection_residual_image']['sha256'])).digest())


def upstream_rows(capture, input_record):
    V.input_value(input_record)
    rows = V.capture(capture, input_record)
    start = input_record['cache_metadata'][1] * 16 * 512 * 2
    return rows, [rows[rank * 14 + index][start:start + 1024]
        if index in (3, 4) else rows[rank * 14 + index]
        for rank in range(2) for index in range(7)]


def child_marker(raw, pid):
    V.require(type(raw) is bytes and type(pid) is int and 0 < pid <= 0xffffffff,
              'actual one-child marker input')
    expected = (f'finite prefix layer candidate child pid={pid} pgid={pid}; '
                'setup not acknowledged\n').encode()
    V.require(raw == expected, 'exact single candidate child marker, no other parent stderr')


def validate(summary, files, request, baseline_capture, baseline_input):
    V.require(type(summary) is bytes and type(files) is dict and set(files) == BODY
        and all(type(value) is bytes for value in files.values()), 'exact eleven capture body buffers')
    V.require(len(summary) <= 65536 and len(summary) + sum(map(len, files.values())) <= V.LIMIT,
              'unchanged aggregate capture bound')
    _, baseline_upstream = upstream_rows(baseline_capture, baseline_input)
    V.keys(request, 'schema layer projection_residual_image')
    V.require(request['schema'] == 'FerricFiniteProjectionResidualLayerCaptureRequestV1'
        and request['layer']['schema'] == 'FerricFinitePrefixLayerCaptureRequestV1',
        'separate nested projection capture request')
    image = request['projection_residual_image']
    V.keys(image, 'path bytes sha256')
    V.require(V.uint(image['bytes']) == 10864 and V.digest(image['sha256']).hex() == IMAGE_SHA,
        'actual separately checked projection image')
    o = V.parse(summary)
    V.keys(o, 'schema request run stages files native_attempts retries completed_layers native_closed '
        'gpu_execution ' + ' '.join(FALSE))
    V.require(o['schema'] == SCHEMA and o['request'] == request
        and V.parse(files['request.json']) == request
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
    outer = V.parse(files['candidate-bootstrap.json'])
    V.keys(outer, 'schema layer projection_residual_image')
    V.require(outer == run['bootstrap'] and outer['schema'] == 'FerricProjectionResidualLayerBootstrapV1'
        and outer['projection_residual_image'] == {k: image[k] for k in ('bytes', 'sha256')},
        'actual new wrapper and additional image')
    b = outer['layer']
    V.bootstrap(b, PROFILE, request['layer'], files, 'candidate', pid)
    V.require(run['profile_sha256'] == projection_digest(outer), 'new arithmetic-bound scoped profile')
    for key in ('generation', 'token', 'rotary_bits'):
        V.require(b['input'][key] == baseline_input[key], 'same genuine pre-residual logical input')
    V.require(b['input']['cache_metadata'][0] == baseline_input['cache_metadata'][0],
        'same logical position without assuming identical physical page placement')
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
    rows, upstream = upstream_rows(files['candidate-capture.bin'], b['input'])
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
    V.require(upstream == baseline_upstream, 'all fourteen pre-residual arrays equal genuine prior capture')
    return dict(schema='ferric-p228-projection-residual-capture-checked-v1', captured_arrays=28,
        capture_bytes=V.CAPTURE_BYTES, closed_child_pids=[pid], stages=stages,
        layer=0, generation=1, position=0, token=9112, full_kv_checked=True,
        pre_residual_arrays_equal=True, pre_residual_array_count=14,
        kv_comparison='logical-position0-row-after-both-full-cache-untouched-checks',
        projection_residual_image_sha256=IMAGE_SHA, old_hidden_equality_required=False,
        current_tf4_hidden_equal=None, conditional_residual_checks_performed=False,
        native_closed=True, recorded_child_exit_zero=True, recorded_child_group_absent=True,
        paired_comparison_performed=False, numerical_acceptance=False,
        independent_framework_comparison_performed=False, full_model_correctness=False,
        performance_claim=False, production_authority=False, full_forward=False)
