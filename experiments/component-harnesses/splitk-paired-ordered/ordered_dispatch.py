"""Unqualified, source-only adapter for the frozen component Worker transport."""

import hashlib
import json
import struct
import time


COUNTERS = frozenset((
    "commands", "command_ns", "full_currentness_checks", "full_currentness_ns",
    "operational_currentness_checks", "operational_currentness_ns",
    "kernel_admissions", "kernel_admission_ns", "dispatches",
    "dispatch_prepare_ns", "dispatch_publish_ns", "dispatch_wait_ns",
    "completion_polls", "reads", "read_bytes", "read_ns", "writes",
    "write_bytes", "write_ns",
))
DISPATCH_FIELDS = frozenset((
    "kernel", "payload_bytes", "workgroup", "grid", "pointers",
))
POINTER_FIELDS = frozenset((
    "kernarg_offset", "buffer", "buffer_offset", "extent_bytes", "access",
))
MODES = ("latency", "counters", "ticks")
U64_MAX = (1 << 64) - 1


def require(condition, message):
    if not condition:
        raise ValueError(message)


def uint(value, bits=64, minimum=0):
    require(type(value) is int and minimum <= value < 1 << bits,
            "bounded unsigned integer required")
    return value


def exact(value, fields):
    require(type(value) is dict and value.keys() == fields, "closed object schema")


def vector(value, bits):
    require(type(value) is list and len(value) == 3, "three geometry dimensions")
    for dimension in value:
        uint(dimension, bits, 1)


def encode_batch(plans, mode):
    require(mode in MODES, "immutable campaign mode")
    require(type(plans) is list and 1 <= len(plans) <= 64, "ordered64 count")
    dispatches, payloads = [], []
    for plan in plans:
        exact(plan, {"symbol", "metadata", "command", "encoded"})
        command, payload = plan["command"], plan["encoded"]
        exact(command, DISPATCH_FIELDS | {"op", "timeout_ms"})
        require(command["op"] == "dispatch"
                and type(command["timeout_ms"]) is int
                and command["timeout_ms"] == 30000, "frozen dispatch deadline")
        uint(command["kernel"], minimum=1)
        size = uint(command["payload_bytes"], 32)
        require(type(payload) is bytes and len(payload) == size <= 65536,
                "exact bounded kernarg payload")
        vector(command["workgroup"], 16)
        vector(command["grid"], 32)
        pointers = command["pointers"]
        require(type(pointers) is list and len(pointers) <= 256, "pointer bound")
        offsets = set()
        for pointer in pointers:
            exact(pointer, POINTER_FIELDS)
            offset = uint(pointer["kernarg_offset"], 32)
            require(offset % 8 == 0 and offset + 8 <= size and offset not in offsets,
                    "unique aligned kernarg pointer")
            require(payload[offset:offset + 8] == bytes(8), "unpatched source pointer")
            offsets.add(offset)
            uint(pointer["buffer"], minimum=1)
            start = uint(pointer["buffer_offset"])
            extent = uint(pointer["extent_bytes"])
            require(start + extent <= U64_MAX, "pointer range overflow")
            require(pointer["access"] in ("read", "write", "read_write"), "pointer access")
        dispatches.append({name: command[name] for name in sorted(DISPATCH_FIELDS)})
        payloads.append(payload)
    require(sum(map(len, payloads)) <= 4 * 1024 * 1024, "transfer bound")
    opcode = ("dispatch_ordered_batch64_profiled" if mode == "ticks"
              else "dispatch_ordered_batch64")
    command = {"op": opcode, "dispatches": dispatches, "timeout_ms": 30000}
    raw = json.dumps(command, separators=(",", ":")).encode("ascii")
    require(0 < len(raw) <= 65536, "header bound")
    payload = b"".join(payloads)
    return command, payload, struct.pack("<I", len(raw)) + raw + payload


def snapshot(worker):
    response, payload = worker.command({"op": "performance_snapshot"},
                                       expected="performance_snapshot")
    exact(response, {"op", "counters"})
    require(response["op"] == "performance_snapshot" and payload == b"",
            "snapshot response identity")
    counters = response["counters"]
    exact(counters, COUNTERS)
    for value in counters.values():
        uint(value)
    return dict(counters)


def counter_delta(before, after, count, mode, post_warmup):
    require(mode in ("counters", "ticks") and type(post_warmup) is bool,
            "counter cohort identity")
    uint(count, minimum=1)
    exact(before, COUNTERS)
    exact(after, COUNTERS)
    delta = {}
    for name in sorted(COUNTERS):
        old, new = uint(before[name]), uint(after[name])
        require(new >= old, "counter regression")
        delta[name] = new - old
    # Snapshot clones before its own command is recorded: previous snapshot + batch.
    require(delta["commands"] == 2 and delta["dispatches"] == count,
            "snapshot bracket command/dispatch scope")
    for name in ("reads", "read_bytes", "read_ns", "writes", "write_bytes",
                 "write_ns", "kernel_admissions", "kernel_admission_ns"):
        require(delta[name] == 0, "unexpected bracket activity: " + name)
    require(delta["completion_polls"] >= 1, "completed batch poll")
    if post_warmup:
        require(delta["full_currentness_checks"] == (2 if mode == "ticks" else 0),
                "post-warmup full-currentness scope")
        require(delta["operational_currentness_checks"] >= 5,
                "post-warmup operational-currentness scope")
    return delta


def validate_response(response, payload, kernels, mode, device, epoch, frontier):
    require(mode in MODES and type(kernels) is list and 1 <= len(kernels) <= 64,
            "response campaign identity")
    for kernel in kernels:
        uint(kernel, minimum=1)
    uint(device)
    uint(epoch)
    uint(frontier)
    require(frontier + len(kernels) <= U64_MAX, "packet frontier overflow")
    require(payload == b"", "unexpected response payload")
    if mode != "ticks":
        exact(response, {"op", "completed_dispatches", "elapsed_ns"})
        require(response["op"] == "dispatch_ordered_batch64_completed"
                and type(response["completed_dispatches"]) is int
                and response["completed_dispatches"] == len(kernels),
                "completed batch identity")
        return {"ordered_worker_elapsed_ns": uint(response["elapsed_ns"], minimum=1)}
    exact(response, {"op", "device_unique_id", "queue_epoch", "elapsed_ns", "timestamps"})
    require(response["op"] == "dispatch_ordered_batch64_profiled_completed"
            and uint(response["device_unique_id"]) == device
            and uint(response["queue_epoch"]) == epoch, "profiled queue identity")
    elapsed = uint(response["elapsed_ns"], minimum=1)
    stamps = response["timestamps"]
    require(type(stamps) is list and len(stamps) == len(kernels), "timestamp cardinality")
    retained = []
    for index, (stamp, kernel) in enumerate(zip(stamps, kernels, strict=True)):
        exact(stamp, {"packet_id", "kernel", "start_tick", "end_tick"})
        require(uint(stamp["packet_id"]) == frontier + index
                and uint(stamp["kernel"], minimum=1) == kernel, "timestamp packet identity")
        start, end = uint(stamp["start_tick"], minimum=1), uint(stamp["end_tick"], minimum=1)
        require(end > start, "positive raw timestamp interval")
        retained.append({**stamp, "interval_ticks": end - start})
    return {"ordered_worker_elapsed_ns": elapsed, "packet_timestamps": retained,
            "tick_unit": "raw_device_ticks_frequency_unspecified",
            "interval_scope": "end_tick_minus_start_tick_not_shader_only_not_wall_time",
            "group_envelope_ticks": max(item["end_tick"] for item in retained)
                                    - min(item["start_tick"] for item in retained)}


class OrderedSession:
    """Fresh-worker owner; caller must send no other dispatches or queue rotations."""

    def __init__(self, worker, mode, device):
        require(mode in MODES, "immutable campaign mode")
        self.worker, self.mode, self.device = worker, mode, uint(device)
        # Pinned 55c worker constructs its initial queue at epoch/frontier zero.
        self.epoch, self.frontier = 0, 0
        self.configured, self.failed = False, False

    def configure(self):
        require(not self.configured and not self.failed, "fresh session configuration")
        try:
            response, payload = self.worker.command({
                "op": "configure_performance", "cache_kernel_admission": True,
                "operational_currentness": True, "profile": self.mode != "latency",
            }, expected="performance_configured")
            exact(response, {"op"})
            require(response["op"] == "performance_configured" and payload == b"",
                    "configuration response identity")
            self.configured = True
        except BaseException:
            self.failed = True
            raise

    def dispatch(self, plans, *, post_warmup, clock=time.perf_counter_ns):
        require(self.configured and not self.failed, "configured live session required")
        try:
            require(type(post_warmup) is bool, "warmup phase identity")
            before = snapshot(self.worker) if self.mode != "latency" else None
            started = uint(clock())
            command, payload, frame = encode_batch(plans, self.mode)
            kernels = [item["kernel"] for item in command["dispatches"]]
            require(self.frontier + len(kernels) <= U64_MAX, "packet frontier overflow")
            self.worker.check_process()
            self.worker.trace.write(json.dumps({
                "command": command, "payload_sha256": hashlib.sha256(payload).hexdigest(),
            }) + "\n")
            self.worker.trace.flush()
            self.worker.transfer(frame)
            response, data = self.worker.receive()
            elapsed = uint(clock()) - started
            require(elapsed > 0, "positive host interval")
            result = validate_response(response, data, kernels, self.mode, self.device,
                                       self.epoch, self.frontier)
            if before is not None:
                after = snapshot(self.worker)
                result["counter_delta"] = counter_delta(
                    before, after, len(kernels), self.mode, post_warmup)
            self.frontier += len(kernels)
            return {"mode": self.mode, "dispatch_count": len(kernels),
                    "host_dispatch_wall_ns": elapsed, **result}
        except BaseException:
            # The supervisor retains teardown ownership; never retry/fallback here.
            self.failed = True
            raise
