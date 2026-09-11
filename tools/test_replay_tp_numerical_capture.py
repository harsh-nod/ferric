import hashlib
import json
import os
from pathlib import Path
import struct
import tempfile
import unittest

import replay_tp_numerical_capture as replay


class ReplayTests(unittest.TestCase):
    def test_manifest_custody_and_sampled_actual_shape(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            def payload(name, data, width):
                (root/name).write_bytes(data)
                return {"file":name,"bytes":len(data),"sha256":hashlib.sha256(data).hexdigest(),"element_bytes":width}
            left=payload("input",bytes(4096*2),2)
            weight=payload("weight",bytes(1024*4096*2),2)
            output=payload("output",bytes(1024*2),2)
            manifest={"schema":"FerricTpNumericalCaptureV1","complete":True,"performance_qualified":False,
                      "model_parity_qualified":False,"maximum_total_bytes":replay.MAX_BYTES,
                      "identity":{"tensor_parallel":1,"projection":"baseline"},"execution_rows":[{"slot":0}],
                      "projection":{"rows":1,"n":1024,"k":4096,"input":left,"weights_nk":weight,
                                    "actual_weights":weight,"actual_weight_layout":"nk","output":output},
                      "head":{"request_rows":[]},"payload_bytes_before_manifest":sum(item["bytes"] for item in (left,weight,output))}
            raw=json.dumps(manifest).encode()
            (root/"manifest.json").write_bytes(raw)
            expected=hashlib.sha256(raw).hexdigest()
            loaded,payloads,actual,total=replay.load_capture(root,expected)
            self.assertEqual(actual,expected)
            self.assertEqual(total,len(raw)+manifest["payload_bytes_before_manifest"])
            result=replay.analyze(loaded,payloads)
            self.assertTrue(result["all_sampled_outputs_match_serial"])
            self.assertEqual(len(result["projection_samples"]),8)
            (root/"incomplete").write_bytes(b"x")
            with self.assertRaises(ValueError):
                replay.load_capture(root,expected)
            (root/"incomplete").unlink()
            (root/"input").write_bytes(b"x"*len(payloads["input"]))
            with self.assertRaises(ValueError):
                replay.load_capture(root,expected)

    def test_exact_transpose_and_mutation(self):
        for n, k in [(1, 1), (3, 5), (16, 17), (33, 31)]:
            original = list(range(n * k))
            actual = [original[column*k+inner] for inner in range(k) for column in range(n)]
            self.assertTrue(replay.full_transpose_equal(original, actual, n, k))
            actual[-1] ^= 1
            self.assertFalse(replay.full_transpose_equal(original, actual, n, k))

    def test_scalar_accumulation_and_ties_even_are_explicit(self):
        one, negative = 0x3f80, 0xbf80
        self.assertEqual(replay.sampled_dot([one, one], [one, negative]), (0.0, 0.0))
        for bits, expected in [(0x3f808000, 0x3f80), (0x3f818000, 0x3f82), (0xbf808000, 0xbf80), (0x80000000, 0x8000)]:
            self.assertEqual(replay.narrow(struct.unpack("<f", struct.pack("<I", bits))[0]), expected)
        with self.assertRaises(ValueError):
            replay.sampled_dot([0x7f80], [one])

    def test_file_digest_extent_symlink_and_json_duplicates(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "value"
            path.write_bytes(b"abc")
            os.utime(path, (1, 2))
            digest = hashlib.sha256(b"abc").hexdigest()
            self.assertEqual(replay.read_file(path, 3, digest), (b"abc", digest))
            for maximum, wrong in [(2, digest), (3, "0"*64)]:
                with self.assertRaises(ValueError):
                    replay.read_file(path, maximum, wrong)
            link = Path(root) / "link"
            link.symlink_to(path)
            with self.assertRaises(OSError):
                replay.read_file(link, 3, digest)
        with self.assertRaises(ValueError):
            json.loads('{"x":1,"x":2}', object_pairs_hook=replay.no_duplicates)


if __name__ == "__main__":
    unittest.main()
