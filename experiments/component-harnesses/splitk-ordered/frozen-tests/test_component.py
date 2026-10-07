"""Pure CPU transport/packing fixtures; never spawn an engineering worker."""

from pathlib import Path
import struct
import tempfile
import types
import unittest
from unittest import mock

import component as c


IMAGE, IMAGE_HASH = b"image-fixture", c.digest(b"image-fixture")


def metadata(symbol):
    count = 2 if symbol == c.MERGE else 3
    arguments = []
    for index in range(count):
        arguments.extend([
            {"offset": index * 16, "bytes": 8, "global_buffer": True,
             "pointee_alignment": None, "access": None},
            {"offset": index * 16 + 8, "bytes": 8, "global_buffer": False},
        ])
    scalar_count = 0 if symbol == c.MERGE else 5
    arguments.extend({"offset": count * 16 + index * 4, "bytes": 4, "global_buffer": False}
                     for index in range(scalar_count))
    explicit = count * 16 + scalar_count * 4
    implicit = (explicit + 7) // 8 * 8
    return {"symbol": symbol, "object_sha256": list(bytes.fromhex(IMAGE_HASH)),
            "wavefront_size": 64, "private_segment_bytes": 0, "group_segment_bytes": 0,
            "kernarg_alignment": 8, "explicit_arguments": arguments,
            "kernarg_bytes": implicit + 256, "implicit_argument_offset": implicit,
            "implicit_argument_bytes": 256}


def fixture():
    return types.SimpleNamespace(input_bf16=b"\x80\x3f" * 4,
        weights_nk_bf16=b"\x00\x3f" * 8, weights_kn_bf16=b"\x00\x3f" * 8,
        partials_f32=struct.pack("<f", 0.25) * 8,
        output_f32=struct.pack("<f", 2.0))


class FakeWorker:
    def __init__(self, value):
        self.fixture, self.buffers, self.kernels, self.log = value, {}, {}, []
        self.next_id, self.change_metadata, self.corrupt = 1, None, None

    def allocate(self, data):
        identifier = self.next_id
        self.next_id += 1
        self.buffers[identifier] = bytearray(data)
        self.log.append("allocate")
        return identifier

    def read(self, identifier, length):
        self.log.append("read")
        return bytes(self.buffers[identifier][:length])

    def command(self, command, payload=b"", expected=None):
        operation = command["op"]
        self.log.append(operation)
        if operation == "load_kernel":
            identifier = len(self.kernels) + 1
            self.kernels[identifier] = command["symbol"]
            value = metadata(command["symbol"])
            if self.change_metadata:
                self.change_metadata(value)
            return {"op": expected, "kernel": identifier, "metadata": value}, b""
        if operation == "write":
            start = command["offset"]
            self.buffers[command["buffer"]][start:start + len(payload)] = payload
        elif operation == "free":
            del self.buffers[command["buffer"]]
        elif operation == "dispatch":
            symbol = self.kernels[command["kernel"]]
            self.log[-1] = "dispatch:" + symbol
            target = command["pointers"][-1]
            data = self.fixture.partials_f32 if symbol == c.PARTIAL else self.fixture.output_f32
            start = target["buffer_offset"]
            self.buffers[target["buffer"]][start:start + len(data)] = data
            if self.corrupt == "guard":
                self.buffers[target["buffer"]][0] ^= 1
            elif self.corrupt == "output":
                self.buffers[target["buffer"]][start] ^= 1
            return {"op": expected, "elapsed_ns": 10}, b""
        return {"op": expected}, b""


class ComponentTests(unittest.TestCase):
    def prepared(self):
        value = fixture()
        worker, records = FakeWorker(value), c.records_for(value)
        c.allocate_shared(worker, records)
        plans = c.component_plans(worker, records, (IMAGE, IMAGE_HASH), (IMAGE, IMAGE_HASH))
        return value, worker, records, plans

    def test_entry_binds_direct_supervisor_lifetime_and_owned_session(self):
        wrapper = {"pid": 20, "parent": 10, "group": 20, "session": 20, "start": 200, "uid": 1000}
        parent = {"pid": 10, "parent": 5, "group": 5, "session": 5, "start": 100, "uid": 1000}
        env = {"FERRIC_COMPONENT_SUPERVISOR_PID": "10", "FERRIC_COMPONENT_SUPERVISOR_START_TICKS": "100"}
        c.entry_identity(wrapper, parent, env, 10, 1000)
        cases = [({}, parent, wrapper, 10),
                 ({**env, "FERRIC_COMPONENT_SUPERVISOR_PID": "010"}, parent, wrapper, 10),
                 ({**env, "FERRIC_COMPONENT_SUPERVISOR_START_TICKS": "101"}, parent, wrapper, 10),
                 (env, {**parent, "uid": 0}, wrapper, 10),
                 (env, parent, {**wrapper, "group": 10}, 10),
                 (env, parent, {**wrapper, "session": 10}, 10),
                 (env, parent, wrapper, 11)]
        for actual_env, actual_parent, actual_wrapper, ppid in cases:
            with self.assertRaises(ValueError):
                c.entry_identity(actual_wrapper, actual_parent, actual_env, ppid, 1000)

    def test_supervisor_pidfd_readiness_or_identity_drift_refuses(self):
        parent = {"pid": 10, "parent": 5, "group": 5, "session": 5, "start": 100, "uid": 1000}
        lifecycle = types.SimpleNamespace(process=lambda _pid: dict(parent))
        with mock.patch.object(c.os, "getppid", return_value=10), \
                mock.patch.object(c.select, "select", return_value=([], [], [])):
            c.verify_supervisor(lifecycle, 7, parent)
        with mock.patch.object(c.os, "getppid", return_value=10), \
                mock.patch.object(c.select, "select", return_value=([7], [], [])):
            with self.assertRaisesRegex(ValueError, "supervisor exited"):
                c.verify_supervisor(lifecycle, 7, parent)
        with mock.patch.object(c.os, "getppid", return_value=10):
            lifecycle.process = lambda _pid: {**parent, "start": 101}
            with self.assertRaisesRegex(ValueError, "identity changed"):
                c.verify_supervisor(lifecycle, 7, parent)

    def test_shared_allocations_and_exact_abi_launches(self):
        _, worker, records, plans = self.prepared()
        self.assertEqual(worker.log.count("allocate"), 5)
        self.assertEqual([plans[name][0]["command"]["grid"][0] // 64
                          for name in ("wave", "mfma", "splitk")], [4096, 256, 2048])
        self.assertEqual(plans["splitk"][1]["command"]["grid"], [4096, 1, 1])
        self.assertEqual(plans["wave"][0]["command"]["pointers"][1]["buffer"], records["nk"]["id"])
        self.assertEqual(plans["mfma"][0]["command"]["pointers"][1]["buffer"], records["kn"]["id"])
        self.assertEqual(plans["splitk"][0]["command"]["pointers"][2]["buffer"],
                         plans["splitk"][1]["command"]["pointers"][0]["buffer"])
        for arm in ("wave", "mfma", "splitk"):
            encoded = plans[arm][0]["encoded"]
            self.assertEqual(struct.unpack_from("<IIIII", encoded, 48), (1, 4096, 12288, 1, 2))
            self.assertEqual(len(encoded), 328)
        self.assertEqual(len(plans["splitk"][1]["encoded"]), 288)

    def test_fixed_three_arm_warmup_then_paired_abba(self):
        rows = list(c.schedule())
        self.assertEqual(len(rows), 30)
        self.assertEqual([row["arm"] for row in rows[:6]], ["wave", "mfma", "splitk"] * 2)
        for block in range(3):
            part = rows[6 + block * 8:14 + block * 8]
            self.assertEqual([row["arm"] for row in part],
                             ["wave", "splitk", "splitk", "wave", "mfma", "splitk", "splitk", "mfma"])
            self.assertEqual([row["position"] for row in part], [0, 1, 2, 3] * 2)
        for blocks, warmups in ((0, 0), (9, 0), (True, 1), (1, -1), (1, 5)):
            with self.assertRaises(ValueError):
                list(c.schedule(blocks, warmups))

    def test_timer_contains_both_candidate_dispatches_and_no_reset_or_check(self):
        value, worker, records, plans = self.prepared()
        times = iter(range(100, 2000, 100))
        def clock():
            worker.log.append("clock")
            return next(times)
        samples = c.execute_samples(worker, records, plans, value, clock, blocks=1, warmups=0)
        self.assertEqual([row["dispatch_count"] for row in samples], [1, 2, 2, 1] * 2)
        self.assertEqual([row["host_dispatch_wall_ns"] for row in samples], [100] * 8)
        clocks = [index for index, item in enumerate(worker.log) if item == "clock"]
        for index in range(0, len(clocks), 2):
            timed = worker.log[clocks[index] + 1:clocks[index + 1]]
            self.assertTrue(all(item.startswith("dispatch:") for item in timed))
            if len(timed) == 2:
                self.assertEqual(timed, ["dispatch:" + c.PARTIAL, "dispatch:" + c.MERGE])
        self.assertEqual(worker.log.count("allocate"), 5)

    def test_guards_and_exact_results_refuse_corruption(self):
        for corruption in ("guard", "output"):
            value, worker, records, plans = self.prepared()
            worker.corrupt = corruption
            retained = []
            with self.assertRaises(ValueError):
                c.execute_samples(worker, records, plans, value, blocks=1, warmups=0,
                                  retain=retained.append)
            self.assertEqual(len(retained), 1)
            self.assertEqual(retained[0]["event"], "completed_unchecked")

    def test_inputs_are_checked_without_timing_or_allocation(self):
        _, worker, records, _ = self.prepared()
        self.assertEqual(set(c.check_inputs(worker, records)), {"a", "nk", "kn"})
        worker.buffers[records["kn"]["id"]][len(c.GUARD)] ^= 1
        with self.assertRaises(ValueError):
            c.check_inputs(worker, records)

    def test_resource_metadata_and_late_abi_fields_fail_before_dispatch(self):
        mutations = [
            lambda m: m.update(symbol="wrong"), lambda m: m.update(wavefront_size=32),
            lambda m: m.update(private_segment_bytes=8), lambda m: m.update(group_segment_bytes=8),
            lambda m: m.update(implicit_argument_offset=80),
            lambda m: m["explicit_arguments"][-1].update(offset=65),
            lambda m: m["explicit_arguments"][0].update(access="write"),
        ]
        for mutation in mutations:
            value = fixture()
            worker, records = FakeWorker(value), c.records_for(value)
            c.allocate_shared(worker, records)
            worker.change_metadata = mutation
            with self.assertRaises(ValueError):
                c.component_plans(worker, records, (IMAGE, IMAGE_HASH), (IMAGE, IMAGE_HASH))
            self.assertFalse(any(item.startswith("dispatch:") for item in worker.log))

    def test_success_checks_shared_inputs_and_frees_once(self):
        worker = FakeWorker(fixture())
        result = c.run_component(worker, worker.fixture, (IMAGE, IMAGE_HASH), (IMAGE, IMAGE_HASH))
        self.assertEqual(result["inputs_before"], result["inputs_after"])
        self.assertEqual(worker.log.count("allocate"), 5)
        self.assertEqual(worker.log.count("free"), 5)
        self.assertFalse(worker.buffers)
        self.assertEqual(len(result["samples"]), 30)

    def test_helper_hash_checked_before_execution_and_symlinks_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source = root / "helper.py"
            source.write_text("raise AssertionError('must not execute')\n")
            with self.assertRaises(ValueError):
                c.load_frozen(source, "0" * 64, "bad_helper")
            link = root / "link.py"
            link.symlink_to(source)
            with self.assertRaises(ValueError):
                c.load_frozen(link, c.digest(source.read_bytes()), "bad_link")


if __name__ == "__main__":
    unittest.main()
