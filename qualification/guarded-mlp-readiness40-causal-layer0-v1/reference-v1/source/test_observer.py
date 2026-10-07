"""Pure byte-layout/observer tests; imports neither Torch nor model code."""
import json
import types
import unittest
import observer as O

def fixture(position):
    values={}
    for name, shape in O.shapes(position).items():
        size=2
        for n in shape: size*=n
        values[name]=bytes(size)
    return values

class ObserverTests(unittest.TestCase):
    def test_six_complete_shapes_and_closed_integer_position(self):
        previous=None
        for p in range(6):
            values=fixture(p); O.validate_values(values,p,previous); previous=values
        for p in (-1,6,True,1.0):
            with self.assertRaises(ValueError): O.shapes(p)

    def test_cache_layout_is_token_then_four_rank_heads_not_head_major(self):
        raw=b''.join((head*10+token).to_bytes(2,'little')*128 for head in range(8) for token in range(3))
        for rank in (0,1):
            expected=b''.join((head*10+token).to_bytes(2,'little')*128
                              for token in range(3) for head in range(rank*4,rank*4+4))
            self.assertEqual(O.cache_rows(raw,2,rank),expected)
        for rank in (-1,2,True):
            with self.assertRaises(ValueError): O.cache_rows(raw,2,rank)

    def test_changed_completed_cache_and_wrong_appended_row_refuse(self):
        previous=fixture(0); values=fixture(1)
        raw=bytearray(values['cache-key']); raw[0]=1; values['cache-key']=bytes(raw)
        with self.assertRaisesRegex(ValueError,'completed causal cache changed'):
            O.validate_values(values,1,previous)
        values=fixture(1); raw=bytearray(values['cache-value']); raw[256]=1; values['cache-value']=bytes(raw)
        with self.assertRaisesRegex(ValueError,'current K/V appended'):
            O.validate_values(values,1,previous)

    def test_roster_nonfinite_and_producer_consumer_mismatch_refuse(self):
        for edit in (lambda v:v.pop('silu'),lambda v:v.update(unknown=b''),
                     lambda v:v.update(gate=b'\xc1\x7f'+v['gate'][2:]),
                     lambda v:v.update(**{'q-input':b'\x01\0'+v['q-input'][2:]})):
            values=fixture(0); edit(values)
            with self.assertRaises(ValueError): O.validate_values(values,0)

    def test_rotary_returns_identical_original_objects_without_recomputation(self):
        observer=O.Observe.__new__(O.Observe)
        result=(object(),object()); calls=[]; captured=[]
        def original(*args,**kwargs): calls.append((args,kwargs)); return result
        observer.original=original; observer.active=True; observer.calls=0; observer.selected=0
        observer.add=lambda name,value:captured.append((name,value))
        self.assertIs(observer.rotary('q',mask='x'),result)
        self.assertEqual(calls,[(('q',),{'mask':'x'})]); self.assertEqual(observer.calls,1)
        self.assertIs(captured[0][1],result[0]); self.assertIs(captured[1][1],result[1])
        with self.assertRaises(ValueError): observer.rotary()

    def test_restoration_removes_hooks_and_rejects_callable_replacement(self):
        observer=O.Observe.__new__(O.Observe); removed=[]
        original=object(); wrapper=object()
        observer.module=types.SimpleNamespace(apply_rotary_pos_emb=wrapper)
        observer.original=original; observer.wrapper=wrapper; observer.installed=True
        observer.handles=[types.SimpleNamespace(remove=lambda:removed.append(1))]
        observer.close(); self.assertIs(observer.module.apply_rotary_pos_emb,original); self.assertEqual(removed,[1])
        observer.installed=True; observer.module.apply_rotary_pos_emb=object()
        with self.assertRaises(ValueError): observer.close()
        self.assertIs(observer.module.apply_rotary_pos_emb,original)

    def test_binary_roster_has_six_real_positions_and_no_native_data_authority(self):
        values=[fixture(p) for p in range(6)]; tokens=list(range(40))
        raw=O.encode(values,tokens,1); self.assertEqual(raw[:8],b'FREF061\0')
        n=int.from_bytes(raw[8:12],'little'); size=int.from_bytes(raw[12:16],'little')
        head=json.loads(raw[16:16+n]); self.assertEqual(len(raw),16+n+size)
        self.assertEqual([r['position'] for r in head['captures']],list(range(6)))
        self.assertEqual([r['input_token'] for r in head['captures']],list(range(6)))
        self.assertEqual(head['payload'],O.pin(raw[16+n:])); self.assertFalse(head['native_intermediates_used'])
        self.assertFalse(head['numerical_acceptance']); self.assertFalse(head['performance_claim'])
        with self.assertRaises(ValueError): O.encode(values[:-1],tokens,1)
        with self.assertRaises(ValueError): O.encode(values,tokens,True)

if __name__=='__main__':
    unittest.main()
