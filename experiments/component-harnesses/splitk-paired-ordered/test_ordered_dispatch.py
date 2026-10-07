"""Source-only fixtures; qualification must run remotely under G36."""

import copy
import io
import json
import struct
import unittest

import ordered_dispatch as adapter


def plan(kernel=7, payload=bytes(32)):
    return {"symbol": "fixture", "metadata": {}, "encoded": payload, "command": {
        "op": "dispatch", "kernel": kernel, "payload_bytes": len(payload),
        "workgroup": [64, 1, 1], "grid": [128, 1, 1], "timeout_ms": 30000,
        "pointers": [{"kernarg_offset": 0, "buffer": 4, "buffer_offset": 64,
                      "extent_bytes": 128, "access": "read"}],
    }}


def completed(count=1):
    return {"op": "dispatch_ordered_batch64_completed", "completed_dispatches": count,
            "elapsed_ns": 100}


def profiled(kernels=(7, 9), frontier=0):
    return {"op": "dispatch_ordered_batch64_profiled_completed", "device_unique_id": 31,
            "queue_epoch": 0, "elapsed_ns": 100, "timestamps": [
                {"packet_id": frontier + index, "kernel": kernel,
                 "start_tick": 10 + index, "end_tick": 30 + index}
                for index, kernel in enumerate(kernels)]}


def counters(count=1, mode="counters"):
    before = dict.fromkeys(adapter.COUNTERS, 100)
    after = dict(before)
    for name, value in {"commands": 2, "command_ns": 800, "dispatches": count,
                        "full_currentness_checks": 2 if mode == "ticks" else 0,
                        "operational_currentness_checks": 5, "completion_polls": 1,
                        "dispatch_prepare_ns": 200, "dispatch_publish_ns": 300,
                        "dispatch_wait_ns": 400}.items():
        after[name] += value
    return before, after


def snapshot_response(values):
    return {"op": "performance_snapshot", "counters": values}


class FakeWorker:
    def __init__(self, responses):
        self.responses = list(responses)
        self.trace, self.transfers, self.commands = io.StringIO(), [], []
        self.checks = 0

    def check_process(self):
        self.checks += 1

    def transfer(self, frame):
        self.transfers.append(frame)

    def receive(self):
        result = self.responses.pop(0)
        if isinstance(result, BaseException):
            raise result
        return result if type(result) is tuple else (result, b"")

    def command(self, command, payload=b"", expected=None):
        if command.get("payload_bytes", 0) != len(payload):
            raise ValueError("frozen top-level payload check")
        self.commands.append(command)
        response, data = self.receive()
        if response["op"] != expected:
            raise ValueError("unexpected response")
        return response, data


class OrderedDispatchTests(unittest.TestCase):
    def test_single_batch_wire(self):
        source = plan()
        command, payload, frame = adapter.encode_batch([source], "latency")
        size = struct.unpack("<I", frame[:4])[0]
        self.assertEqual(json.loads(frame[4:4 + size]), command)
        self.assertEqual(frame[4 + size:], source["encoded"])
        self.assertEqual(payload, source["encoded"])
        self.assertEqual(set(command), {"op", "dispatches", "timeout_ms"})
        self.assertNotIn("payload_bytes", command)
        self.assertNotIn("timeout_ms", command["dispatches"][0])

    def test_split_payload_order_and_source_unchanged(self):
        plans = [plan(7, bytes(16) + b"a" * 16), plan(9, bytes(16) + b"b" * 16)]
        original = copy.deepcopy(plans)
        command, payload, _ = adapter.encode_batch(plans, "ticks")
        self.assertEqual(command["op"], "dispatch_ordered_batch64_profiled")
        self.assertEqual([item["kernel"] for item in command["dispatches"]], [7, 9])
        self.assertEqual(payload, plans[0]["encoded"] + plans[1]["encoded"])
        self.assertEqual(plans, original)

    def test_closed_command_and_plan_schemas(self):
        mutations = [lambda p: p.update(extra=1),
                     lambda p: p["command"].update(extra=1),
                     lambda p: p["command"].pop("kernel"),
                     lambda p: p["command"].update(op="dispatch_ordered_batch64")]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                item = plan()
                mutate(item)
                with self.assertRaises(ValueError):
                    adapter.encode_batch([item], "latency")

    def test_unsigned_bool_and_geometry_rejection(self):
        for field, value in (("kernel", True), ("kernel", 0), ("kernel", 1 << 64),
                             ("payload_bytes", True), ("workgroup", [True, 1, 1]),
                             ("workgroup", [1 << 16, 1, 1]), ("grid", [0, 1, 1]),
                             ("grid", [1 << 32, 1, 1]), ("grid", [64, 1]),
                             ("timeout_ms", True), ("timeout_ms", 30001)):
            with self.subTest(field=field, value=value):
                item = plan()
                item["command"][field] = value
                with self.assertRaises(ValueError):
                    adapter.encode_batch([item], "latency")

    def test_payload_bounds_and_identity(self):
        for payload in (b"short", bytearray(32), bytes(65537)):
            with self.subTest(length=len(payload)):
                item = plan()
                item["encoded"] = payload
                with self.assertRaises(ValueError):
                    adapter.encode_batch([item], "latency")

    def test_batch_count_and_mode_bounds(self):
        for plans, mode in (([], "latency"), ([plan()] * 65, "latency"),
                            ([plan()], "ordinary"), ((plan(),), "latency")):
            with self.subTest(mode=mode, count=len(plans)):
                with self.assertRaises(ValueError):
                    adapter.encode_batch(plans, mode)

    def test_pointer_closed_bounds_ranges_and_access(self):
        for field, value in (("extra", 1), ("kernarg_offset", 1),
                             ("kernarg_offset", 32), ("buffer", False),
                             ("buffer_offset", -1), ("buffer_offset", adapter.U64_MAX),
                             ("extent_bytes", 1 << 64), ("access", "execute")):
            with self.subTest(field=field, value=value):
                item = plan()
                item["command"]["pointers"][0][field] = value
                with self.assertRaises(ValueError):
                    adapter.encode_batch([item], "latency")

    def test_pointer_duplicate_count_and_prepatched_rejected(self):
        for count in (2, 257):
            item = plan()
            item["command"]["pointers"] *= count
            with self.assertRaises(ValueError):
                adapter.encode_batch([item], "latency")
        item = plan(payload=b"x" + bytes(31))
        with self.assertRaises(ValueError):
            adapter.encode_batch([item], "latency")

    def test_four_mib_transfer_boundary(self):
        item = plan(payload=bytes(65536))
        _, payload, _ = adapter.encode_batch([item] * 64, "latency")
        self.assertEqual(len(payload), 4 * 1024 * 1024)

    def test_header_bound(self):
        item = plan(payload=bytes(4096))
        item["command"]["pointers"] = [
            {**item["command"]["pointers"][0], "kernarg_offset": index * 8}
            for index in range(256)]
        with self.assertRaisesRegex(ValueError, "header bound"):
            adapter.encode_batch([item] * 64, "latency")

    def test_completed_response_identity(self):
        result = adapter.validate_response(completed(2), b"", [7, 9], "latency", 31, 0, 0)
        self.assertEqual(result, {"ordered_worker_elapsed_ns": 100})
        for field, value in (("op", "dispatched"), ("completed_dispatches", True),
                             ("completed_dispatches", 1), ("elapsed_ns", 0),
                             ("elapsed_ns", True), ("extra", 1)):
            response = completed(2)
            response[field] = value
            with self.assertRaises(ValueError):
                adapter.validate_response(response, b"", [7, 9], "latency", 31, 0, 0)
        with self.assertRaises(ValueError):
            adapter.validate_response(completed(), b"x", [7], "latency", 31, 0, 0)

    def test_profiled_overlap_is_allowed_without_conversion_or_sum(self):
        result = adapter.validate_response(profiled(), b"", [7, 9], "ticks", 31, 0, 0)
        self.assertEqual([item["interval_ticks"] for item in result["packet_timestamps"]], [20, 20])
        self.assertEqual(result["group_envelope_ticks"], 21)
        self.assertEqual(result["tick_unit"], "raw_device_ticks_frequency_unspecified")
        self.assertNotIn("gpu_ns", result)
        self.assertNotIn("summed_ticks", result)

    def test_profiled_queue_cardinality_and_opcode(self):
        for field, value in (("device_unique_id", 32), ("device_unique_id", True),
                             ("queue_epoch", 1), ("queue_epoch", False),
                             ("op", "dispatch_ordered_batch64_completed"),
                             ("timestamps", []), ("elapsed_ns", -1), ("extra", 1)):
            response = profiled()
            response[field] = value
            with self.assertRaises(ValueError):
                adapter.validate_response(response, b"", [7, 9], "ticks", 31, 0, 0)

    def test_profiled_packet_order_kernel_and_ticks(self):
        for field, value in (("packet_id", 0), ("packet_id", True), ("kernel", 7),
                             ("start_tick", 0), ("end_tick", 11),
                             ("end_tick", 1 << 64), ("extra", 1)):
            response = profiled()
            response["timestamps"][1][field] = value
            with self.assertRaises(ValueError):
                adapter.validate_response(response, b"", [7, 9], "ticks", 31, 0, 0)
        response = profiled()
        response["timestamps"].reverse()
        with self.assertRaises(ValueError):
            adapter.validate_response(response, b"", [7, 9], "ticks", 31, 0, 0)

    def test_profiled_frontier_and_overflow(self):
        adapter.validate_response(profiled(frontier=17), b"", [7, 9], "ticks", 31, 0, 17)
        with self.assertRaises(ValueError):
            adapter.validate_response(profiled(), b"", [7, 9], "ticks", 31, 0, 17)
        with self.assertRaises(ValueError):
            adapter.validate_response(completed(), b"", [7], "latency", 31, 0, adapter.U64_MAX)

    def test_counter_snapshot_closed_schema(self):
        before, _ = counters()
        worker = FakeWorker([snapshot_response(before)])
        self.assertEqual(adapter.snapshot(worker), before)
        for values in ({**before, "extra": 0}, {**before, "commands": True}):
            with self.assertRaises(ValueError):
                adapter.snapshot(FakeWorker([snapshot_response(values)]))
        with self.assertRaises(ValueError):
            adapter.snapshot(FakeWorker([(snapshot_response(before), b"x")]))

    def test_counter_scope_two_commands_and_not_additive(self):
        before, after = counters(2)
        delta = adapter.counter_delta(before, after, 2, "counters", True)
        self.assertEqual(delta["commands"], 2)
        self.assertEqual(delta["dispatches"], 2)
        self.assertGreater(sum(delta[name] for name in (
            "dispatch_prepare_ns", "dispatch_publish_ns", "dispatch_wait_ns")),
            delta["command_ns"])
        after["commands"] -= 1
        with self.assertRaises(ValueError):
            adapter.counter_delta(before, after, 2, "counters", True)

    def test_counter_mutations_and_regression(self):
        for field, value in (("commands", 99), ("dispatches", 102), ("reads", 101),
                             ("read_ns", 101), ("writes", 101), ("kernel_admissions", 101),
                             ("kernel_admission_ns", 101), ("completion_polls", 100),
                             ("operational_currentness_checks", 104),
                             ("full_currentness_checks", 101), ("command_ns", True)):
            before, after = counters()
            after[field] = value
            with self.assertRaises(ValueError):
                adapter.counter_delta(before, after, 1, "counters", True)

    def test_timestamp_full_checks_and_warmup_exception(self):
        before, after = counters(mode="ticks")
        adapter.counter_delta(before, after, 1, "ticks", True)
        after["full_currentness_checks"] += 3
        adapter.counter_delta(before, after, 1, "ticks", False)
        with self.assertRaises(ValueError):
            adapter.counter_delta(before, after, 1, "ticks", True)

    def test_latency_session_single_transfer_no_snapshots(self):
        worker = FakeWorker([{"op": "performance_configured"}, completed(2)])
        session = adapter.OrderedSession(worker, "latency", 31)
        session.configure()
        result = session.dispatch([plan(), plan(9)], post_warmup=True,
                                  clock=iter((10, 60)).__next__)
        self.assertEqual(result["host_dispatch_wall_ns"], 50)
        self.assertEqual(len(worker.transfers), 1)
        self.assertEqual(worker.checks, 1)
        self.assertEqual(session.frontier, 2)
        self.assertEqual(worker.commands, [{"op": "configure_performance",
            "cache_kernel_admission": True, "operational_currentness": True, "profile": False}])
        self.assertNotIn("counter_delta", result)

    def test_counter_session_separate_brackets(self):
        before, after = counters()
        worker = FakeWorker([{"op": "performance_configured"}, snapshot_response(before),
                             completed(), snapshot_response(after)])
        session = adapter.OrderedSession(worker, "counters", 31)
        session.configure()
        result = session.dispatch([plan()], post_warmup=True, clock=iter((10, 60)).__next__)
        self.assertEqual(result["counter_delta"]["commands"], 2)
        self.assertEqual([item["op"] for item in worker.commands], [
            "configure_performance", "performance_snapshot", "performance_snapshot"])
        self.assertTrue(worker.commands[0]["profile"])

    def test_tick_session_advances_only_validated_frontier(self):
        before, after = counters(2, "ticks")
        worker = FakeWorker([{"op": "performance_configured"}, snapshot_response(before),
                             profiled(), snapshot_response(after), snapshot_response(after),
                             profiled()])
        session = adapter.OrderedSession(worker, "ticks", 31)
        session.configure()
        session.dispatch([plan(), plan(9)], post_warmup=True, clock=iter((10, 60)).__next__)
        with self.assertRaises(ValueError):
            session.dispatch([plan(), plan(9)], post_warmup=True, clock=iter((70, 90)).__next__)
        self.assertTrue(session.failed)
        self.assertEqual(session.frontier, 2)

    def test_configuration_and_terminal_native_failure(self):
        worker = FakeWorker([{"op": "performance_configured"}, ValueError("native failure")])
        session = adapter.OrderedSession(worker, "latency", 31)
        with self.assertRaises(ValueError):
            session.dispatch([plan()], post_warmup=True)
        session.configure()
        with self.assertRaises(ValueError):
            session.configure()
        with self.assertRaisesRegex(ValueError, "native failure"):
            session.dispatch([plan()], post_warmup=True, clock=iter((10, 60)).__next__)
        self.assertEqual(session.frontier, 0)
        with self.assertRaises(ValueError):
            session.dispatch([plan()], post_warmup=True)
        self.assertEqual(len(worker.transfers), 1)

    def test_preflight_and_clock_failure_do_not_commit_frontier(self):
        worker = FakeWorker([{"op": "performance_configured"}])
        session = adapter.OrderedSession(worker, "latency", 31)
        session.configure()
        with self.assertRaises(ValueError):
            session.dispatch([], post_warmup=True, clock=iter((10, 60)).__next__)
        self.assertEqual(worker.transfers, [])
        worker = FakeWorker([{"op": "performance_configured"}, completed()])
        session = adapter.OrderedSession(worker, "latency", 31)
        session.configure()
        with self.assertRaises(ValueError):
            session.dispatch([plan()], post_warmup=True, clock=iter((10, 10)).__next__)
        self.assertEqual(session.frontier, 0)
        self.assertTrue(session.failed)


if __name__ == "__main__":
    unittest.main()
