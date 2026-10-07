"""Small synthetic arithmetic/parity tests, not native or model qualification."""
import copy
from fractions import Fraction
import json
import struct
import unittest

import compare as C


def fixture():
    tokens = list(range(100, 2148))
    raw = bytearray(606976)
    struct.pack_into('<H', raw, 37 * 8192 + 2 * 2, 0x4000)
    struct.pack_into('<H', raw, 37 * 8192 + 9112 * 2, 0x3f80)
    raw = bytes(raw)
    logits = raw[37 * 8192:]
    def native(selected):
        rows = []
        for position in range(40):
            rows.append(dict(schema='FerricGuardedMlpLongResponseV2',
                profile='readiness40' if selected == C.OLD else 'readiness40_position5',
                request=dict(command=dict(token=tokens[position])),
                completion=dict(position=position,generation=position+1,input_token=tokens[position],
                    output_token=2,captured=position in selected,logits=C.pin(logits),observation=C.pin(raw))))
        return b''.join(json.dumps(r,separators=(',',':')).encode()+b'\n' for r in rows)
    def reference(selected):
        cases=[dict(position=p,generation=p+1,input_token=tokens[p],predicted_token=2,cache_length=p+1,
                    logits=C.pin(logits),payload=C.pin(raw) if p in selected else None) for p in range(40)]
        return [dict(ordinal=i,fresh_cache=True,cases=copy.deepcopy(cases)) for i in (1,2)]
    bodies=lambda selected:{p:raw for p in selected}
    return [struct.pack('<2048I',*tokens),native(C.OLD),native(C.NEW),reference(C.OLD),reference(C.NEW),
            bodies(C.OLD),bodies(C.NEW),[bodies(C.OLD),bodies(C.OLD)],[bodies(C.NEW),bodies(C.NEW)]]


class DiagnosticTests(unittest.TestCase):
    def test_complete_parity_reports_no_acceptance(self):
        result=C.compare(*fixture())
        self.assertEqual(result['common_payloads_equal'],[0,16,39])
        self.assertEqual(result['reference']['top_two'][0]['token'],2)
        self.assertEqual(result['reference']['targets'][1]['rank'],2)
        self.assertEqual(result['native']['top_two_margin_units_2_pow_minus133'],1<<133)
        self.assertIsNone(result['first_nonidentical_layer_hidden'])
        self.assertFalse(result['numerical_acceptance'])
        self.assertFalse(result['receipt_authentication'])

    def test_all40_logit_pins_must_agree_not_only_selected(self):
        args=fixture(); rows=[json.loads(x) for x in args[2].splitlines()]
        rows[31]['completion']['logits']['sha256']='a'*64
        args[2]=b''.join(json.dumps(r).encode()+b'\n' for r in rows)
        with self.assertRaisesRegex(ValueError,'all40 native'):C.compare(*args)

    def test_reference_all40_logits_and_repeats_must_agree(self):
        args=fixture();args[4][1]['cases'][31]['logits']['sha256']='a'*64
        with self.assertRaisesRegex(ValueError,'repeat gate'):C.compare(*args)
        args[4][0]['cases'][31]['logits']['sha256']='a'*64
        with self.assertRaisesRegex(ValueError,'all40 reference'):C.compare(*args)

    def test_common_payload_bytes_and_selection_are_not_assumed(self):
        args=fixture();args[6][16]=b'\x01\x00'+args[6][16][2:]
        with self.assertRaisesRegex(ValueError,'selected bytes'):C.compare(*args)
        args=fixture();args[6][15]=args[6].pop(5)
        with self.assertRaisesRegex(ValueError,'four native'):C.compare(*args)

    def test_authentic_history_and_boolean_ids_refused(self):
        args=fixture();rows=[json.loads(x) for x in args[2].splitlines()]
        rows[5]['completion']['input_token']=9112
        args[2]=b''.join(json.dumps(r).encode()+b'\n' for r in rows)
        with self.assertRaises(ValueError):C.compare(*args)
        with self.assertRaises(ValueError):C.uint(True,40)
        with self.assertRaises(ValueError):C.normalized_pin(dict(bytes=True,sha256='0'*64),1)

    def test_exact_bf16_integer_units_include_subnormal_and_signed_zero(self):
        for word,expected in [(0,0),(0x8000,0),(1,Fraction(1,1<<133)),(0x8001,-Fraction(1,1<<133)),
                              (0x3f80,1),(0xbf80,-1),(0x4000,2),(0x0080,Fraction(1,1<<126))]:
            self.assertEqual(Fraction(C.exact_units(word),1<<133),expected)
        for word in (0x7f80,0xff80,0x7fc0,True):
            with self.assertRaises(ValueError):C.exact_units(word)

    def test_top_two_ties_use_lowest_token_and_zero_margin(self):
        raw=bytearray(303872)
        for token in (9112,2):struct.pack_into('<H',raw,token*2,0x3f80)
        got=C.logit_diagnostic(bytes(raw))
        self.assertEqual([r['token'] for r in got['top_two']],[2,9112])
        self.assertEqual(got['top_two_margin_units_2_pow_minus133'],0)
        self.assertEqual(got['token2_minus_token9112_units_2_pow_minus133'],0)


if __name__=='__main__':unittest.main(verbosity=2)
