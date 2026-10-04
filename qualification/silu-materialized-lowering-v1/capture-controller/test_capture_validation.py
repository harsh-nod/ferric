"""Synthetic fixtures only; no model, parent process or GPU execution."""
import copy
import json
import unittest

import capture_validation as C
import layer_validation as V


def encoded(value):
    return json.dumps(value, separators=(',', ':'), sort_keys=True).encode()


def terminal(prefix):
    if prefix:
        return [1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2 + ([0xffffffff] * 4 + [3]) * 2 + [1] * 130 + [64] * 130
    return [1, 0, 0, 31] + [1, 96, 96, 1, 64] * 2 + ([0xffffffff] * 8 + [3]) * 2 + [1] * 258 + [64] * 258


def refresh(o, files):
    raw = files['candidate-capture.bin']; stages = []; offset = 0
    for rank in range(2):
        for name, size, width in V.STAGES:
            stages.append(dict(rank=rank, stage=name, offset=offset, bytes=size,
                elements=size // width, element_bytes=width, sha256=list(V.sha(raw[offset:offset+size]))))
            offset += size
    o['stages'] = stages
    close = V.parse(files['candidate-response-2.json']); close['capture'] = V.part(raw)
    files['candidate-response-2.json'] = encoded(close); o['run']['close'] = close
    o['files'] = [dict(name=name, **V.part(files[name])) for name in C.NAMES]


def fixture():
    image = dict(path='/synthetic/image', bytes=1, sha256=[1] * 32)
    request = dict(schema='FerricFinitePrefixLayerCaptureRequestV1', source='/synthetic/model', worker=image,
        images={name: dict(image) for name in ('prefix', 'mlp', 'residual', 'tail')},
        expected_bundle_id=[2] * 32, expected_model_id=[3] * 32, device_ids=[7, 9], session=[4] * 32,
        prompt={name: dict(image) for name in ('manifest', 'text', 'tokens')},
        prefix_tiles_image=image, mlp_tiles_image=image, evidence_directory='/synthetic/native',
        dispatch_timeout_ms=10000, child_deadline_ms=3600000)
    pid, group = 12347, 101
    dispatch = dict(symbol='synthetic', grid_workgroups=1, workgroup_size=64,
        arguments=[dict(kind='buffer', source_id=10, offset=0, elements=1, element_bytes=2, access='read')])
    program = dict(schema='ferric-finite-source-grammar-v1', profile=V.REG_PROFILE, group_id=group,
        token_buffer=1, result_buffer=2, metadata=[dict(positions=3, page_table=4, cos=5, sin=6)] * 2,
        steps=[dict(kind='rank', rank=0, dispatch=dispatch)] * 1013)
    program_raw = encoded(program)
    registration = dict(profile=V.REG_PROFILE, bundle_id=request['expected_bundle_id'],
        model_id=request['expected_model_id'], session=request['session'], pool_identity=201,
        group_id=group, child_identity=pid, source_program_bytes=len(program_raw),
        source_program_sha256=list(V.sha(program_raw)),
        **{name: [0] * count for name, count in [('layers',72), ('globals',3), ('auxiliary',28),
            ('scratch',18), ('pending_buffers',150), ('state_slots',288)]})
    files = {'request.json': encoded(request), 'candidate-program.json': program_raw,
             'candidate-registration.json': encoded(registration), 'candidate-uploads.json': b'{}'}
    begin = dict(scope={name: registration[name] for name in V.SCOPE.split()},
        **{name: V.part(files['candidate-' + suffix + '.json']) for name, suffix in
            [('registration','registration'), ('source_program','program'), ('uploads','uploads')]},
        **{name+'_image': {key:image[key] for key in ('bytes','sha256')} for name in ('prefix','mlp','residual','tail')})
    b = dict(protocol=1, profile=C.PROFILE, device_ids=request['device_ids'], timeout_ms=10000,
        begin=begin, input=dict(generation=1, token=9112, cache_metadata=[0,*range(144)],
            rotary_bits=[0x3f800000]*64+[0]*64),
        mlp_image={key:image[key] for key in ('bytes','sha256')},
        prefix_image={key:image[key] for key in ('bytes','sha256')})
    extra = dict(path='/synthetic/projection.hsaco', bytes=10864, sha256=list(bytes.fromhex(C.IMAGE_SHA)))
    request = dict(schema='FerricFiniteProjectionResidualLayerCaptureRequestV1',
                   layer=request, projection_residual_image=extra)
    outer = dict(schema='FerricProjectionResidualLayerBootstrapV1', layer=b,
                 projection_residual_image={k:extra[k] for k in ('bytes','sha256')})
    files['request.json'] = encoded(request)
    files['candidate-bootstrap.json'] = encoded(outer); profile = C.projection_digest(outer)
    files['candidate-capture.bin'] = bytes(V.CAPTURE_BYTES); files['candidate-stderr.bin'] = b''
    for number, command in ((1,'run'), (2,'close')):
        files[f'candidate-request-{number}.json'] = encoded(dict(protocol=1, id=number,
            profile_sha256=profile, command=command))
        response = dict(protocol=1, id=number, profile_sha256=profile, profile=C.PROFILE,
            native_closed=number == 2, completed_layers=1, control=None, capture=None,
            gpu_execution=True, **{name:False for name in V.FALSE})
        if number == 2:
            response.update(capture=V.part(files['candidate-capture.bin']),
                control=dict(prefix=[terminal(True)] * 2, mlp=[terminal(False)] * 2,
                    embedding_ns=[1,2], paired_ns=[[1,2]]*4))
        files[f'candidate-response-{number}.json'] = encoded(response)
    run = dict(child_pid=pid, bootstrap=outer, profile_sha256=profile, setup_commands=1,
        close=response, child_exit_zero=True, process_group_absent=True)
    o = dict(schema=C.SCHEMA, request=request, run=run, stages=[], files=[], native_attempts=1,
        retries=0, completed_layers=1, native_closed=True, gpu_execution=True,
        **{key:False for key in C.FALSE})
    refresh(o, files)
    return o, files, request, dict(body=bytes(V.CAPTURE_BYTES), input=copy.deepcopy(b['input']))


class CaptureTests(unittest.TestCase):
    def check(self, o, files, request, hidden):
        return C.validate(encoded(o), files, request, hidden['body'], hidden['input'])

    def test_complete_synthetic_capture_uses_real_wire_source_and_array_checks(self):
        result = self.check(*fixture())
        self.assertEqual((result['captured_arrays'], result['capture_bytes']), (28, V.CAPTURE_BYTES))
        self.assertEqual(result['closed_child_pids'], [12347])
        self.assertTrue(result['full_kv_checked']); self.assertTrue(result['pre_residual_arrays_equal'])
        self.assertTrue(result['pre_swiglu_arrays_equal'])
        self.assertEqual(result['pre_swiglu_array_count'],22)
        self.assertIsNone(result['current_tf4_hidden_equal'])
        for key in ('paired_comparison_performed', 'numerical_acceptance', 'full_model_correctness',
                    'independent_framework_comparison_performed', 'performance_claim', 'production_authority'):
            self.assertIs(result[key], False)

    def test_candidate_image_and_old_profile_fallback_refuse(self):
        o, files, request, hidden = fixture()
        for mode in ('image', 'profile', 'wrapper'):
            bad=copy.deepcopy(o); changed=dict(files); req=copy.deepcopy(request)
            if mode=='image': req['projection_residual_image']['sha256']=[0]*32
            elif mode=='profile': bad['run']['profile_sha256']=V.profile_digest(bad['run']['bootstrap']['layer'])
            else: bad['run']['bootstrap']['schema']='FerricPrefixLayerBootstrapV1'
            with self.subTest(mode=mode),self.assertRaises(RuntimeError):
                self.check(bad,changed,req,hidden)

    def test_all_pre_residual_stages_must_equal_but_downstream_may_change(self):
        o, files, request, hidden = fixture()
        for index in (*range(11), *range(14,25)):
            bad=copy.deepcopy(o); changed=dict(files); raw=bytearray(changed['candidate-capture.bin'])
            raw[bad['stages'][index]['offset']]=1
            changed['candidate-capture.bin']=bytes(raw); refresh(bad,changed)
            with self.subTest(index=index),self.assertRaises(RuntimeError): self.check(bad,changed,request,hidden)
        for index in (11, 12, 13, 25, 26, 27):
            bad=copy.deepcopy(o); changed=dict(files); raw=bytearray(changed['candidate-capture.bin'])
            raw[bad['stages'][index]['offset']]=1
            changed['candidate-capture.bin']=bytes(raw); refresh(bad,changed)
            result=self.check(bad,changed,request,hidden)
            self.assertIsNone(result['current_tf4_hidden_equal'])
            self.assertIs(result['conditional_residual_checks_performed'],False)

    def test_logical_kv_row_can_move_only_with_authentic_page_table(self):
        o,files,request,baseline=fixture()
        old=bytearray(baseline['body']); candidate=bytearray(old)
        for index in (3,4,17,18):
            offset=o['stages'][index]['offset']
            old[offset:offset+1024]=b'\x80\x3f'*512
            candidate[offset+16384:offset+16384+1024]=b'\x80\x3f'*512
        baseline['body']=bytes(old)
        b=o['run']['bootstrap']
        b['layer']['input']['cache_metadata'][1:3]=[1,0]
        files['candidate-bootstrap.json']=encoded(b)
        profile=C.projection_digest(b); o['run']['profile_sha256']=profile
        for number in (1,2):
            for kind in ('request','response'):
                name=f'candidate-{kind}-{number}.json'; row=V.parse(files[name])
                row['profile_sha256']=profile; files[name]=encoded(row)
        files['candidate-capture.bin']=bytes(candidate); refresh(o,files)
        self.assertTrue(self.check(o,files,request,baseline)['pre_residual_arrays_equal'])
        candidate[o['stages'][3]['offset']]=1
        files['candidate-capture.bin']=bytes(candidate); refresh(o,files)
        with self.assertRaises(RuntimeError): self.check(o,files,request,baseline)

    def test_missing_extra_or_reordered_body_manifest_refuses(self):
        o, files, request, hidden = fixture()
        for mode in ('missing', 'extra', 'order', 'digest'):
            bad = copy.deepcopy(o); changed = dict(files)
            if mode == 'missing': changed.pop('candidate-stderr.bin')
            elif mode == 'extra': changed['parity.json'] = b'{}'
            elif mode == 'order': bad['files'][0], bad['files'][1] = bad['files'][1], bad['files'][0]
            else: bad['files'][0]['sha256'] = [0]*32
            with self.subTest(mode=mode), self.assertRaises(RuntimeError): self.check(bad, changed, request, hidden)

    def test_old_paired_or_numerical_authority_cannot_be_admitted(self):
        o, files, request, hidden = fixture()
        for key, value in [('schema','FerricFinitePrefixLayerComparisonObservationV1'),
                ('bitwise_equal',True), ('runs',[]), ('native_attempts',True), ('retries',1),
                *[(key,True) for key in C.FALSE]]:
            with self.subTest(key=key), self.assertRaises(RuntimeError): self.check(dict(o,**{key:value}), files, request, hidden)

    def test_genuine_token_pages_rotary_image_and_child_scope_are_joined(self):
        o, files, request, hidden = fixture()
        for mode in ('token','position','rotary','image','child','scope'):
            bad = copy.deepcopy(o); changed = dict(files); outer = bad['run']['bootstrap']; b = outer['layer']
            if mode == 'token': b['input']['token'] = 785
            elif mode == 'position': b['input']['cache_metadata'][0] = 1
            elif mode == 'rotary': b['input']['rotary_bits'][0] = 0
            elif mode == 'image': b['prefix_image']['sha256'] = [9]*32
            elif mode == 'child': bad['run']['child_pid'] += 1
            else: b['begin']['scope']['session'] = [9]*32
            changed['candidate-bootstrap.json'] = encoded(outer); refresh(bad,changed)
            with self.subTest(mode=mode), self.assertRaises(RuntimeError): self.check(bad,changed,request,hidden)

    def test_run_close_order_terminal_states_and_reap_are_required(self):
        o, files, request, hidden = fixture()
        for mode in ('reap','group','commands','close','run-release','control'):
            bad=copy.deepcopy(o); changed=dict(files)
            if mode in ('reap','group','commands'):
                bad['run'][{'reap':'child_exit_zero','group':'process_group_absent','commands':'setup_commands'}[mode]] = False
            else:
                number = 1 if mode == 'run-release' else 2
                value=V.parse(changed[f'candidate-response-{number}.json'])
                if mode == 'close': value['native_closed'] = False
                elif mode == 'run-release': value['capture'] = V.part(changed['candidate-capture.bin'])
                else: value['control']['prefix'][1][24] = 65
                changed[f'candidate-response-{number}.json'] = encoded(value)
                if number == 2: bad['run']['close'] = value
                bad['files']=[dict(name=name,**V.part(changed[name])) for name in C.NAMES]
            with self.subTest(mode=mode), self.assertRaises(RuntimeError): self.check(bad,changed,request,hidden)

    def test_every_typed_terminal_state_refuses_corruption(self):
        value=dict(prefix=[terminal(True),terminal(True)],mlp=[terminal(False),terminal(False)],
            embedding_ns=[0,1],paired_ns=[[0,1]]*4)
        V.control(value,C.PROFILE)
        for kind in ('prefix','mlp'):
            for rank in range(2):
                for index in range(len(value[kind][rank])):
                    bad=copy.deepcopy(value); old=bad[kind][rank][index]; bad[kind][rank][index]=1 if old==0 else 0
                    with self.assertRaises(RuntimeError): V.control(bad,C.PROFILE)

    def test_nonfinite_capture_and_untouched_kv_corruption_refuse_with_honest_hashes(self):
        o,files,request,hidden=fixture()
        key_start=sum(size for _,size,_ in V.STAGES[:3])
        fp32_start=sum(size for _,size,_ in V.STAGES[:6])
        for offset,body in ((0,b'\x80\x7f'),(key_start+1024,b'\x01'),(fp32_start,b'\x00\x00\x80\x7f')):
            bad=copy.deepcopy(o); changed=dict(files); raw=bytearray(changed['candidate-capture.bin'])
            raw[offset:offset+len(body)]=body; changed['candidate-capture.bin']=bytes(raw); refresh(bad,changed)
            with self.assertRaises(RuntimeError): self.check(bad,changed,request,hidden)

    def test_stage_extent_offset_type_and_hash_cannot_be_asserted(self):
        o,files,request,hidden=fixture()
        for key,value in (('rank',False),('offset',True),('bytes',1),('element_bytes',4),('sha256',[0]*32)):
            bad=copy.deepcopy(o); bad['stages'][0][key]=value
            with self.assertRaises(RuntimeError): self.check(bad,files,request,hidden)

    def test_worker_stderr_and_duplicate_json_refuse(self):
        o,files,request,hidden=fixture(); files['candidate-stderr.bin']=b'warning\n'; refresh(o,files)
        with self.assertRaises(RuntimeError): self.check(o,files,request,hidden)
        with self.assertRaises(RuntimeError): V.parse(b'{"x":1,"x":2}')

    def test_exact_one_child_marker_only(self):
        raw=b'finite prefix layer candidate child pid=123 pgid=123; setup not acknowledged\n'
        C.child_marker(raw,123)
        for bad in (b'',raw+raw,raw.replace(b'candidate',b'baseline'),raw.replace(b'pgid=123',b'pgid=124'),raw+b'warning\n'):
            with self.assertRaises(RuntimeError): C.child_marker(bad,123)
        with self.assertRaises(RuntimeError): C.child_marker(raw,124)


if __name__ == '__main__': unittest.main()
