"""Pure synthetic comparison checks; no native process or model validation."""
import copy
import hashlib
import json
import unittest

import run_model_gpu as runner
import validate_observation as validator


def fixture():
    values, frames, bodies = [], [], {}
    for index in range(4):
        raw = bytes([index + 1]) * validator.PAYLOAD_BYTES
        old = dict(path='/synthetic/ordinary-%d.bin' % index, bytes=len(raw),
                   sha256=hashlib.sha256(raw).hexdigest())
        current = dict(old, path='/synthetic/current-%d.bin' % index)
        frames.append(dict(observation=dict(current, sha256=list(bytes.fromhex(current['sha256'])))))
        values.append((old, raw)); bodies[current['path']] = raw
    summary = dict(files=dict(frames=frames), input_tokens=[9112, 67, 25, 576])
    prior = (dict(path='/synthetic/ordinary.json', bytes=1, sha256='a' * 64),
             list(summary['input_tokens']), values)
    return summary, bodies, prior


def compare(summary, bodies, prior):
    return runner.compare_payloads(json.dumps(summary).encode(),
                                   lambda pin: bodies[pin['path']], validator, prior)


class ComparisonTests(unittest.TestCase):
    def test_all_four_equal_payloads_and_histories(self):
        summary, bodies, prior = fixture()
        result = compare(summary, bodies, prior)
        self.assertEqual(len(result['frames']), 4)
        self.assertTrue(result['all_payloads_equal'])
        self.assertTrue(result['all_histories_equal'])
        self.assertFalse(result['observed_payload_difference'])
        self.assertFalse(result['independent_accuracy_reference'])
        self.assertFalse(result['numerical_acceptance'])
        self.assertFalse(result['performance_claim'])

    def test_one_changed_byte_in_each_frame_is_reported(self):
        original, original_bodies, prior = fixture()
        for index in range(4):
            with self.subTest(frame=index):
                summary, bodies = copy.deepcopy(original), dict(original_bodies)
                pin = summary['files']['frames'][index]['observation']
                raw = bytearray(bodies[pin['path']]); raw[-1] ^= 1
                bodies[pin['path']] = bytes(raw)
                pin['sha256'] = list(hashlib.sha256(raw).digest())
                result = compare(summary, bodies, prior)
                self.assertFalse(result['all_payloads_equal'])
                self.assertTrue(result['observed_payload_difference'])
                self.assertEqual([row['byte_equal'] for row in result['frames']],
                                 [position != index for position in range(4)])

    def test_history_difference_is_not_hidden_by_equal_payloads(self):
        summary, bodies, prior = fixture()
        summary['input_tokens'][2] += 1
        result = compare(summary, bodies, prior)
        self.assertTrue(result['all_payloads_equal'])
        self.assertFalse(result['all_histories_equal'])
        self.assertEqual([row['same_history'] for row in result['frames']], [True, True, False, False])

    def test_tampered_hash_or_wrong_extent_is_refused(self):
        original, original_bodies, prior = fixture()
        for mutation in ('hash', 'extent', 'body'):
            with self.subTest(mutation=mutation):
                summary, bodies = copy.deepcopy(original), dict(original_bodies)
                pin = summary['files']['frames'][3]['observation']
                if mutation == 'hash': pin['sha256'][0] ^= 1
                elif mutation == 'extent': pin['bytes'] -= 1
                else: bodies[pin['path']] = bodies[pin['path']][:-1]
                with self.assertRaises(RuntimeError): compare(summary, bodies, prior)


if __name__ == '__main__':
    unittest.main(verbosity=2)
