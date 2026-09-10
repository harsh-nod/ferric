#!/usr/bin/env python3
"""Host-only probe protocol rejection and cleanup checks; no GPU is opened."""

import importlib.util
import io
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
from types import SimpleNamespace
import unittest


spec = importlib.util.spec_from_file_location("probe", Path(__file__).with_name("probe.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def receiver(data):
    worker = probe.Worker.__new__(probe.Worker)
    stream = io.BytesIO(data)
    worker.trace = io.StringIO()

    def transfer(payload=None, length=0):
        result = stream.read(length)
        probe.require(len(result) == length, "worker output truncated")
        return result

    worker.transfer = transfer
    return worker


def frame(header, payload=b""):
    data = json.dumps(header).encode()
    return struct.pack("<I", len(data)) + data + payload


class ProbeTests(unittest.TestCase):
    def test_fixtures(self):
        probe.self_test()

    def test_optional_pointer_metadata_never_overrides_source_abi(self):
        record = {"element_bytes": 2, "access": "read"}
        pointer = {"offset": 16, "bytes": 8, "global_buffer": True,
                   "pointee_alignment": None, "access": None}
        self.assertTrue(probe.pointer_matches_source(pointer, 16, record))
        self.assertTrue(probe.pointer_matches_source(
            {**pointer, "pointee_alignment": 2, "access": "read"}, 16, record))
        for key, value in [("offset", 0), ("bytes", 4), ("global_buffer", False),
                           ("pointee_alignment", 4), ("access", "write")]:
            self.assertFalse(probe.pointer_matches_source({**pointer, key: value}, 16, record))

    def test_header_bounds_and_duplicate_keys(self):
        for size in [0, 65537, (1 << 32) - 1]:
            with self.assertRaisesRegex(RuntimeError, "header limit"):
                receiver(struct.pack("<I", size)).receive()
        raw = b'{"op":"read","op":"closed"}'
        with self.assertRaisesRegex(RuntimeError, "duplicate JSON"):
            receiver(struct.pack("<I", len(raw)) + raw).receive()

    def test_payload_bounds_and_truncation(self):
        for size in [-1, True, probe.LIMIT + 1]:
            with self.assertRaisesRegex(RuntimeError, "payload limit"):
                receiver(frame({"op": "read", "payload_bytes": size})).receive()
        with self.assertRaisesRegex(RuntimeError, "truncated"):
            receiver(frame({"op": "read", "payload_bytes": 4}, b"abc")).receive()

    def test_mismatched_response(self):
        worker = receiver(b"")
        worker.check_process = lambda: None
        worker.transfer = lambda payload=None, length=0: b""
        worker.receive = lambda: ({"op": "allocated"}, b"")
        with self.assertRaisesRegex(RuntimeError, "unexpected response"):
            worker.command({"op": "close"}, expected="closed")

    def test_kernarg_limit_precedes_allocation(self):
        case = next(probe.cases())
        for size in [-1, True, 65537, 1 << 62]:
            metadata = {
                "symbol": case["symbol"], "object_sha256": [0] * 32,
                "wavefront_size": 64, "private_segment_bytes": 0,
                "group_segment_bytes": 0, "kernarg_alignment": 8,
                "explicit_arguments": [{}] * 11, "kernarg_bytes": size,
            }
            worker = SimpleNamespace(command=lambda *args, **kwargs: (
                {"metadata": metadata}, b""))
            with self.assertRaisesRegex(RuntimeError, "kernarg byte limit"):
                probe.probe(worker, case, b"artifact", "0" * 64)

    def test_closed_real_pipe(self):
        read_fd, write_fd = os.pipe()
        os.close(write_fd)
        with os.fdopen(read_fd, "rb", buffering=0) as output:
            worker = probe.Worker.__new__(probe.Worker)
            worker.process = SimpleNamespace(stdout=output)
            worker.timeout = 0.5
            with self.assertRaisesRegex(RuntimeError, "truncated"):
                worker.transfer(length=4)

    def test_abort_reaps_owned_child(self):
        worker = probe.Worker.__new__(probe.Worker)
        worker.trace = io.StringIO()
        worker.stderr = io.BytesIO()
        worker.process = subprocess.Popen([sys.executable, "-I", "-B", "-c",
                                           "import time; time.sleep(30)"],
                                          stdin=subprocess.DEVNULL,
                                          stdout=subprocess.DEVNULL,
                                          stderr=subprocess.DEVNULL)
        try:
            worker.abort()
            self.assertIsNotNone(worker.process.returncode)
            self.assertTrue(worker.trace.closed and worker.stderr.closed)
        finally:
            if worker.process.poll() is None:
                worker.process.kill()
                worker.process.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
