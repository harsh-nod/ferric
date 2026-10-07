"""Remote-only CPU fixtures for the separate native decode capture."""
import copy
import contextlib
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import types
import unittest


def load(name):
    path = Path(__file__).with_name(name + '.py')
    module = importlib.util.module_from_spec(importlib.util.spec_from_file_location(name,path))
    module.__spec__.loader.exec_module(module)
    return module


d = load('decode_capture')
driver = load('run_decode')
prepare = load('prepare_capture')


def fixture(arm='A'):
    packets = {'A':652,'B':724}[arm]
    spec = {'arm':arm,'device_unique_id':42}
    setup = {'worker_pids':[99],'decode_diagnostic':dict(d.SETUP)}
    general = dict.fromkeys(d.GENERAL,0)
    token = dict.fromkeys(d.TOKEN,0)
    token.update(executions=4,dispatches=2596,publications=4,final_waits=4,retirement_signals=2596)
    def endpoint(ordinal):
        return {'general':{'schema':'FerricRuntimeDiagnosticSnapshotV1','authority':'none',
            'performance_qualified':False,'scope':'cumulative overlapping worker host-wall counters, not GPU timestamps',
            'process_id':99,'device_unique_id':42,'rank':0,'ordinal':ordinal,'counters':general.copy()},
            'token':token.copy(),'snapshot_roundtrip_host_ns':100}
    value = {'schema':d.SETUP['schema'],'authority':'none','performance_qualified':False,
        'latency_sample_admitted':False,'complete':True,'worker_pid':99,'device_unique_id':42,
        'program':1,'epoch':0,'packets_per_execute':packets,'start_frontier':2599,'end_frontier':2599+127*packets,
        'started_monotonic_ns':1000000000000000,'finished_monotonic_ns':1000000000012700,
        'registered_decode_span_host_ns':12700,'before':endpoint(0),'after':endpoint(1),
        'general_counter_delta':general.copy(),'token_counter_delta':dict.fromkeys(d.TOKEN,0),
        'planner_host_ns':[{'host_ns':5} for _ in range(127)],'executions':[],'reads':[],'writes':[],**d.SCOPES}
    value['general_counter_delta'].update(dispatches=127*packets,reads=127,read_bytes=508,writes=630,write_bytes=323568)
    value['token_counter_delta'].update(executions=127,dispatches=127*packets,publications=127,final_waits=127,
        retirement_signals=127*packets,staging_ns=1,kernarg_initialized_bytes=100)
    for kind,fields in (('general',d.GENERAL),('token',d.TOKEN)):
        a = value['before'][kind]['counters'] if kind == 'general' else value['before'][kind]
        b = {k:a[k]+value[kind+'_counter_delta'][k] for k in fields}
        if kind == 'general':
            value['after'][kind]['counters'] = b
        else:
            value['after'][kind] = b
    for i in range(127):
        start = value['started_monotonic_ns']+i*100
        value['executions'].append({'ordinal':i,'frontier':2599+(i+1)*packets,'host_ns':40,
            'started_monotonic_ns':start,'finished_monotonic_ns':start+40})
        value['reads'].append({'bytes':4,'host_ns':5,'finished_monotonic_ns':start+50})
        if i < 126:
            value['writes'].extend({'bytes':size,'host_ns':2,'finished_monotonic_ns':start+55+index*5}
                for index,size in enumerate((4,2048,256,256,4)))
    totals(value)
    def legacy(raw,_spec,_setup):
        assert raw == b'{"start":true}\n{"end":true}\n'
        return {'end':{'counters':value['after']['token'].copy()}}
    return value,spec,setup,legacy


def totals(value):
    for name,rows in (('execute_host_ns',value['executions']),('readback_host_ns',value['reads']),
        ('metadata_write_host_ns',value['writes']),('warm_transport_planner_host_ns',value['planner_host_ns'][1:])):
        value[name] = sum(row['host_ns'] for row in rows)
    general = value['general_counter_delta']
    value['execute_minus_worker_scopes_host_ns'] = value['execute_host_ns']-sum(general[k] for k in
        ('dispatch_prepare_ns','dispatch_publish_ns','dispatch_wait_ns'))-value['token_counter_delta']['staging_ns']
    value['span_residual_host_ns'] = value['registered_decode_span_host_ns']-sum(value[k] for k in
        ('execute_host_ns','readback_host_ns','metadata_write_host_ns','warm_transport_planner_host_ns'))


def raw(value):
    return b'{"start":true}\n'+json.dumps(value,separators=(',',':')).encode()+b'\n{"end":true}\n'


class DecodeReplayTests(unittest.TestCase):
    def test_both_arms_epoch_zero_large_stderr(self):
        for arm in ('A','B'):
            value,spec,setup,legacy = fixture(arm)
            stream = raw(value)
            self.assertGreater(len(stream),65536)
            report = d.replay(stream,spec,setup,legacy)
            self.assertTrue(report['accepted'])
            self.assertFalse(report['latency_sample_admitted'])
            self.assertEqual(sum(row['bytes'] for row in report['line_bindings']),len(stream))
            self.assertEqual(report['stderr_sha256'],hashlib.sha256(stream).hexdigest())

    def test_closed_schema_and_scope(self):
        mutations = (lambda v:v.update(extra=1),lambda v:v.pop('scope'),lambda v:v.update(scope='GPU time'),
            lambda v:v.update(schema='FerricNativeGateUpDecodeIncompleteR1'),lambda v:v.update(complete=False))
        for mutate in mutations:
            value,spec,setup,legacy = fixture()
            mutate(value)
            with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)

    def test_wrong_identity_or_count(self):
        for key in ('worker_pid','device_unique_id','packets_per_execute','start_frontier','end_frontier'):
            value,spec,setup,legacy = fixture()
            value[key] += 1
            with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)

    def test_counter_roster_u64_and_delta(self):
        for key in ('general_counter_delta','token_counter_delta'):
            for bad in (True,-1,2**64,1.5):
                value,spec,setup,legacy = fixture()
                value[key]['dispatches'] = bad
                with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)
        value,spec,setup,legacy = fixture()
        value['after']['general']['counters']['reads'] = 126
        with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)

    def test_missing_observations(self):
        for key in ('executions','reads','writes','planner_host_ns'):
            value,spec,setup,legacy = fixture()
            value[key].pop()
            with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)

    def test_inconsistent_durations_even_with_recomputed_totals(self):
        for key in ('executions','reads','writes','planner_host_ns'):
            value,spec,setup,legacy = fixture()
            value[key][1]['host_ns'] = 2**63
            totals(value)
            with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)

    def test_write_geometry_and_clock_order(self):
        for mutate in (lambda v:v['writes'][1].update(bytes=4),
                       lambda v:v['writes'][1].update(finished_monotonic_ns=0),
                       lambda v:v['executions'][1].update(ordinal=2)):
            value,spec,setup,legacy = fixture()
            mutate(value)
            with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)

    def test_final_counter_endpoint_binding(self):
        value,spec,setup,_ = fixture()
        def mismatch(*_args):
            changed = value['after']['token'].copy()
            changed['staging_ns'] += 1
            return {'end':{'counters':changed}}
        with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,mismatch)

    def test_malformed_extra_incomplete_and_oversized(self):
        value,spec,setup,legacy = fixture()
        for stream in (raw(value)[:-1],raw(value)+b'{}\n',b'{}\n',b'not json\n',b'x'*d.MAX_STDERR+b'\n',
            raw(value).replace(b'"complete":true',b'"complete":true,"complete":true')):
            with self.assertRaises(ValueError): d.replay(stream,spec,setup,legacy)

    def test_booleans_are_not_integer_metadata(self):
        value,spec,setup,legacy = fixture()
        setup['decode_diagnostic']['requests'] = True
        with self.assertRaises(ValueError): d.replay(raw(value),spec,setup,legacy)

    def test_overlapping_worker_residual_may_be_negative(self):
        value,spec,setup,legacy = fixture()
        value['general_counter_delta']['dispatch_wait_ns'] = 999999
        value['after']['general']['counters']['dispatch_wait_ns'] = 999999
        totals(value)
        self.assertLess(d.replay(raw(value),spec,setup,legacy)['breakdown']['execute_minus_worker_scopes_host_ns'],0)

    def test_cleanup_is_mandatory(self):
        good = {'cleanup_ok':True,'child_reaped':True,'owned_descendants_absent':True,
            'returncode':0,'errors':[],'term_sent':False,'kill_sent':False}
        d.clean_exit(good)
        for key,value in (('returncode',1),('cleanup_ok',False),('child_reaped',False),
            ('owned_descendants_absent',False),('term_sent',True),('kill_sent',True),('errors',['failure'])):
            with self.assertRaises(ValueError): d.clean_exit({**good,key:value})

    def test_precise_spec_projection_preserves_parent(self):
        parent = {'cells':[]}
        for arm in ('A','B'):
            parent['cells'].append({'cell_id':'counter-'+arm,'output':'/old/cell','common_args':['--worker','/old/worker'],
                'spec':{'mode':'counters','worker':{'path':'/old/worker','sha256':'f'*64},'controller':{},
                    'argv':['/old/controller','--worker','/old/worker'],'setup_expected':{}}})
        before = copy.deepcopy(parent)
        selected = driver.derive(parent,Path('/dev/shm/ferric-native-gate-up-decode-test'),'A',d.SETUP)
        self.assertEqual(parent,before)
        self.assertEqual(selected['spec']['controller']['sha256'],driver.ELF_SHA)
        self.assertEqual(selected['spec']['worker']['sha256'],'f'*64)
        self.assertEqual(selected['spec']['setup_expected']['decode_diagnostic'],d.SETUP)

    def test_a_failure_prevents_b(self):
        for bad in ({'status':1},{'status':0,'cleanup_ok':False}):
            c = types.SimpleNamespace(read=lambda *_: (json.dumps(bad).encode(),'x'),decode=json.loads)
            with self.assertRaises(ValueError): driver.preceding(c,Path('/unused'),'B')
        driver.preceding(None,None,'A')

    def test_capture_failures_reap_and_preserve_receipt(self):
        for failure in ('token-parity','nonzero','cleanup','bad-stderr','postflight','success'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temporary:
                closed,receipts = [],{}
                value,spec,setup,legacy_counter = fixture()
                spec.update(argv=['/mock/controller'],timeouts={'cell_seconds':1200,'setup_seconds':600,'request_seconds':180})
                cleanup = {'cleanup_ok':failure != 'cleanup','child_reaped':True,'owned_descendants_absent':True,
                    'returncode':1 if failure == 'nonzero' else 0,'errors':[],'term_sent':False,'kill_sent':False}
                class Controller:
                    def __init__(self,*_args): pass
                    def close(self):
                        closed.append(True)
                        return cleanup
                def consume(_controller,_spec,**kwargs):
                    if 'before_requests' in kwargs:
                        kwargs['before_requests'](setup)
                        kwargs['before_request'](0)
                        if failure == 'token-parity': raise ValueError('wrong token')
                        kwargs['after_requests'](setup)
                    return {'setup':setup,'closed':{'clean':True},'exact_output_tokens_checked':128}
                def admission(phase,_setup):
                    if phase == 'postflight' and failure == 'postflight': raise ValueError('GPU appeared')
                    return {'accepted':True}
                cell = types.SimpleNamespace(ledger=types.SimpleNamespace(digest=lambda _: 'spec'),
                    lifecycle=types.SimpleNamespace(deferred_stop=lambda **_:contextlib.nullcontext(),
                        controller_class=lambda _:Controller),consume=consume,Replay=lambda *_:object(),
                    retained_bytes=lambda path,_:b'bad\n' if failure == 'bad-stderr' else raw(value),
                    counter_replay=legacy_counter)
                runner = types.SimpleNamespace(MAX_STREAM=8*1024**2,save=lambda path,row:receipts.update({str(path):copy.deepcopy(row)}))
                args = (spec,Path(temporary)/'result',runner,None,None,cell,d,admission)
                if failure == 'success':
                    self.assertTrue(driver.capture(*args)['accepted'])
                else:
                    with self.assertRaises(ValueError): driver.capture(*args)
                self.assertEqual(len(closed),1)
                self.assertEqual(len(receipts),1)
                self.assertEqual(next(iter(receipts.values()))['accepted'],failure == 'success')


if __name__ == '__main__':
    unittest.main(verbosity=2)
