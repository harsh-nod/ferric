"""Isolated component probe; only the outer native supervisor may launch it."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import stat
import struct
import time
import types

import fixtures


HELPER_SHA256 = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
LIFECYCLE_SHA256 = "0b4ad2aa60bfe50d962f304ae5794ad66ec507ee32b1a453ac1aa5e9a3128ac3"
GUARD = b"\xD3\x6A\x95\x2C" * 16
TAIL = b"\xA5" * (31 * fixtures.N * 4)
PARTIAL = "ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1"
MERGE = "ferric_qwen3_c1_down_splitk8_merge_f32_r1"
WARMUPS, BLOCKS = 2, 3


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def entry_identity(wrapper, parent, environment, actual_parent, uid):
    keys = ("FERRIC_COMPONENT_SUPERVISOR_PID", "FERRIC_COMPONENT_SUPERVISOR_START_TICKS")
    values = []
    for key in keys:
        text = environment.get(key)
        require(type(text) is str and text.isascii() and text.isdecimal()
                and str(int(text)) == text and int(text) > 0, "explicit supervisor identity required")
        values.append(int(text))
    require(wrapper["pid"] == wrapper["group"] == wrapper["session"]
            and wrapper["parent"] == actual_parent == values[0]
            and wrapper["uid"] == uid, "owned wrapper session and direct supervisor required")
    require(parent["pid"] == values[0] and parent["start"] == values[1]
            and parent["uid"] == uid, "supervisor lifetime/credentials changed")


def verify_supervisor(lifecycle, descriptor, parent):
    require(os.getppid() == parent["pid"] and lifecycle.process(parent["pid"]) == parent,
            "supervisor identity changed")
    require(not select.select([descriptor], [], [], 0)[0], "supervisor exited")


def load_frozen(path, expected, name):
    require(path.is_absolute() and path.resolve(strict=True) == path, "canonical helper path")
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 128 * 1024, "bounded helper")
        data = bytearray()
        while part := os.read(descriptor, 128 * 1024 - len(data) + 1):
            data.extend(part)
            require(len(data) <= 128 * 1024, "helper grew")
        after = os.fstat(descriptor)
        fields = ("st_dev", "st_ino", "st_mode", "st_uid", "st_size", "st_mtime_ns", "st_ctime_ns")
        require(all(getattr(before, key) == getattr(after, key) for key in fields)
                and digest(data) == expected, "helper identity mismatch")
    finally:
        os.close(descriptor)
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(bytes(data), str(path), "exec"), module.__dict__)
    return module


def records_for(fixture):
    values = {
        "a": (fixture.input_bf16, 2),
        "nk": (fixture.weights_nk_bf16, 2),
        "kn": (fixture.weights_kn_bf16, 2),
        "partials": (b"\xA5" * len(fixture.partials_f32), 4),
        "output": (b"\xA5" * (len(fixture.output_f32) + len(TAIL)), 4),
    }
    return {name: {"data": data, "element_bytes": width}
            for name, (data, width) in values.items()}


def allocate_shared(worker, records):
    identifiers = set()
    for record in records.values():
        identifier = worker.allocate(GUARD + record["data"] + GUARD)
        require(type(identifier) is int and identifier > 0 and identifier not in identifiers,
                "shared allocation identity reused")
        identifiers.add(identifier)
        record["id"] = identifier


def load_plan(worker, image, image_hash, symbol, buffers, scalars, groups):
    loaded, payload = worker.command({"op": "load_kernel", "payload_bytes": len(image),
        "object_sha256": list(bytes.fromhex(image_hash)), "symbol": symbol}, image, "loaded_kernel")
    require(not payload and type(loaded.get("kernel")) is int and loaded["kernel"] > 0,
            "loaded kernel identity")
    metadata = loaded["metadata"]
    require(metadata["symbol"] == symbol
            and metadata["object_sha256"] == list(bytes.fromhex(image_hash)), "loaded image identity")
    require(metadata["wavefront_size"] == 64 and metadata["private_segment_bytes"] == 0
            and metadata["group_segment_bytes"] == 0 and metadata["kernarg_alignment"] == 8,
            "resource or ABI gate requires the qualified emitted image")
    arguments = metadata["explicit_arguments"]
    require(len(arguments) == 2 * len(buffers) + len(scalars), "explicit argument roster")
    size = metadata["kernarg_bytes"]
    require(type(size) is int and 256 <= size <= 65536, "kernarg extent")
    encoded, pointers, offset = bytearray(size), [], 0
    for index, (record, access) in enumerate(buffers):
        pointer, count = arguments[2 * index:2 * index + 2]
        require(pointer["offset"] == offset and pointer["bytes"] == 8
                and pointer["global_buffer"] is True
                and pointer["pointee_alignment"] in (None, record["element_bytes"])
                and pointer["access"] in (None, access), "source pointer metadata")
        require(count["offset"] == offset + 8 and count["bytes"] == 8
                and count["global_buffer"] is False, "source slice length metadata")
        require(len(record["data"]) % record["element_bytes"] == 0, "slice element extent")
        struct.pack_into("<Q", encoded, offset + 8, len(record["data"]) // record["element_bytes"])
        pointers.append({"kernarg_offset": offset, "buffer": record["id"],
                         "buffer_offset": len(GUARD), "extent_bytes": len(record["data"]),
                         "access": access})
        offset += 16
    for field, scalar in zip(arguments[2 * len(buffers):], scalars, strict=True):
        require(field["offset"] == offset and field["bytes"] == 4
                and field["global_buffer"] is False and type(scalar) is int
                and 0 <= scalar < 1 << 32, "source scalar metadata")
        struct.pack_into("<I", encoded, offset, scalar)
        offset += 4
    require(metadata["implicit_argument_offset"] == (offset + 7) // 8 * 8
            and metadata["implicit_argument_bytes"] == 256
            and size == metadata["implicit_argument_offset"] + 256, "physical hidden ABI")
    require(type(groups) is int and 1 <= groups <= 4096, "bounded component launch")
    return {"symbol": symbol, "metadata": metadata, "encoded": bytes(encoded),
            "command": {"op": "dispatch", "kernel": loaded["kernel"], "payload_bytes": size,
                        "workgroup": [64, 1, 1], "grid": [groups * 64, 1, 1],
                        "pointers": pointers, "timeout_ms": 30000}}


def component_plans(worker, records, v5, candidate):
    scalars = (1, fixtures.N, fixtures.K, 1, 2)
    plans = {}
    for arm, weight in (("wave", "nk"), ("mfma", "kn")):
        control = fixtures.CONTROL_KERNELS[arm]
        plans[arm] = [load_plan(worker, *v5, control["symbol"],
            [(records["a"], "read"), (records[weight], "read"), (records["output"], "write")],
            scalars, control["groups"])]
    plans["splitk"] = [
        load_plan(worker, *candidate, PARTIAL,
            [(records["a"], "read"), (records["kn"], "read"), (records["partials"], "write")],
            scalars, 2048),
        load_plan(worker, *candidate, MERGE,
            [(records["partials"], "read"), (records["output"], "write")], (), 64),
    ]
    return plans


def reset_outputs(worker, records):
    for name in ("partials", "output"):
        record = records[name]
        header, payload = worker.command({"op": "write", "buffer": record["id"],
            "offset": len(GUARD), "payload_bytes": len(record["data"])}, record["data"], "written")
        require(not payload, "write response payload")


def checked_read(worker, record, expected, name):
    data = worker.read(record["id"], len(record["data"]) + 2 * len(GUARD))
    require(data[:len(GUARD)] == GUARD and data[-len(GUARD):] == GUARD,
            name + ": allocation guards changed")
    actual = data[len(GUARD):-len(GUARD)]
    fixtures.validate_output(actual, expected, name)
    return {"bytes": len(actual), "sha256": digest(actual), "guards_unchanged": True}


def check_inputs(worker, records):
    return {name: checked_read(worker, records[name], records[name]["data"], name)
            for name in ("a", "nk", "kn")}


def schedule(blocks=BLOCKS, warmups=WARMUPS):
    require(type(blocks) is int and 1 <= blocks <= 8
            and type(warmups) is int and 0 <= warmups <= 4, "bounded ABBA schedule")
    for index in range(warmups):
        for arm in ("wave", "mfma", "splitk"):
            yield {"phase": "warmup", "block": index, "comparison": None, "position": None, "arm": arm}
    for block in range(blocks):
        for control in ("wave", "mfma"):
            for position, arm in enumerate((control, "splitk", "splitk", control)):
                yield {"phase": "sample", "block": block, "comparison": control,
                       "position": position, "arm": arm}


def execute_samples(worker, records, plans, fixture, clock=time.perf_counter_ns,
                    blocks=BLOCKS, warmups=WARMUPS, retain=lambda _row: None,
                    check_alive=lambda: None):
    samples = []
    for cell in schedule(blocks, warmups):
        check_alive()
        reset_outputs(worker, records)
        elapsed = []
        started = clock()
        for plan in plans[cell["arm"]]:
            header, payload = worker.command(plan["command"], plan["encoded"], "dispatched")
            require(not payload and type(header.get("elapsed_ns")) is int
                    and header["elapsed_ns"] > 0, "completed synchronous dispatch required")
            elapsed.append(header["elapsed_ns"])
        duration = clock() - started
        require(type(duration) is int and duration > 0, "positive host sample duration")
        timing = {**cell, "host_dispatch_wall_ns": duration,
                  "worker_dispatch_wall_ns": elapsed, "dispatch_count": len(elapsed)}
        retain({"event": "completed_unchecked", **timing})
        check_alive()
        output = checked_read(worker, records["output"], fixture.output_f32 + TAIL, "output")
        partial_expected = (fixture.partials_f32 if cell["arm"] == "splitk"
                            else records["partials"]["data"])
        partials = checked_read(worker, records["partials"], partial_expected, "partials")
        row = {**timing, "output_check": output, "partial_check": partials}
        retain({"event": "verified_sample", **row})
        samples.append(row)
    return samples


def run_component(worker, fixture, v5, candidate, retain=lambda _row: None,
                  check_alive=lambda: None):
    records = records_for(fixture)
    allocate_shared(worker, records)
    plans = component_plans(worker, records, v5, candidate)
    before = check_inputs(worker, records)
    samples = execute_samples(worker, records, plans, fixture, retain=retain, check_alive=check_alive)
    after = check_inputs(worker, records)
    require(before == after, "input custody changed")
    for record in records.values():
        _, payload = worker.command({"op": "free", "buffer": record["id"]}, expected="freed")
        require(not payload, "free response payload")
    return {"inputs_before": before, "inputs_after": after, "samples": samples,
            "metadata": {arm: [plan["metadata"] for plan in entries] for arm, entries in plans.items()},
            "allocation_ids": {name: record["id"] for name, record in records.items()},
            "allocation_reuse": "one fixed shared set; no allocation within timed samples"}


def save(path, value):
    with path.open("x", encoding="utf-8") as output:
        json.dump(value, output, sort_keys=True, indent=2, allow_nan=False)
        output.write("\n")


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", action="store_true")
    for name in ("helper", "lifecycle", "worker", "v5-image", "candidate-image", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ("worker", "v5-image", "candidate-image"):
        parser.add_argument("--" + name + "-sha256", required=True)
    parser.add_argument("--device-unique-id", type=int, required=True)
    args = parser.parse_args()
    require(args.run and 0 < args.device_unique_id < 1 << 64, "explicit native run and device identity")
    require(os.getpid() == os.getpgrp() == os.getsid(0)
            and signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL,
            "outer supervisor must create an owned wrapper session")
    require(args.output.is_absolute() and args.output.parent.resolve(strict=True) == args.output.parent,
            "canonical output parent")
    args.output.mkdir(mode=0o700)
    core = load_frozen(args.helper, HELPER_SHA256, "ferric_component_core")
    lifecycle = load_frozen(args.lifecycle, LIFECYCLE_SHA256, "ferric_component_lifecycle")
    source_hashes = {path.name: digest(path.read_bytes()) for path in
                     (Path(__file__).resolve(), Path(fixtures.__file__).resolve())}
    wrapper = lifecycle.process(os.getpid())
    supervisor = lifecycle.process(os.getppid())
    entry_identity(wrapper, supervisor, os.environ, os.getppid(), os.getuid())
    supervisor_fd = os.pidfd_open(supervisor["pid"], 0)
    try:
        verify_supervisor(lifecycle, supervisor_fd, supervisor)
    except BaseException:
        os.close(supervisor_fd)
        raise
    held, worker, clean = [], None, False
    with lifecycle.handling_stop():
        try:
            for path, expected, limit, machine in (
                (args.worker, args.worker_sha256, 512 * 1024 * 1024, 62),
                (args.v5_image, args.v5_image_sha256, 64 * 1024 * 1024, 224),
                (args.candidate_image, args.candidate_image_sha256, 64 * 1024 * 1024, 224),
            ):
                held.append(core.held_file(path, expected, limit, machine))
            fixture = fixtures.make_fixture()
            verify_supervisor(lifecycle, supervisor_fd, supervisor)
            with lifecycle.deferred_stop():
                worker = core.Worker(held[0][0], held[0][1], args.device_unique_id, args.output)
            child = lifecycle.process(worker.process.pid)
            require(child["parent"] == wrapper["pid"] and child["uid"] == wrapper["uid"]
                    and child["group"] == child["session"] == wrapper["pid"], "owned same-session worker")
            with (args.output / "samples.jsonl").open("x", encoding="utf-8") as samples:
                def retain(row):
                    samples.write(json.dumps(row, sort_keys=True, allow_nan=False) + "\n")
                    samples.flush()
                result = run_component(worker, fixture, (held[1][2], args.v5_image_sha256),
                                       (held[2][2], args.candidate_image_sha256), retain,
                                       lambda: verify_supervisor(lifecycle, supervisor_fd, supervisor))
            with lifecycle.deferred_stop():
                worker.finish()
                clean = True
            require(lifecycle.members(wrapper) == [wrapper], "owned worker descendants remain")
            verify_supervisor(lifecycle, supervisor_fd, supervisor)
            require(all(core.identity(os.fstat(fd)) == core.identity(before) for fd, before, _ in held),
                    "held image/worker identity drifted")
            require(source_hashes == {path.name: digest(path.read_bytes()) for path in
                    (Path(__file__).resolve(), Path(fixtures.__file__).resolve())}, "probe source drifted")
            save(args.output / "result.json", {"schema": "FerricSplitKDownComponentV1",
                "authority": "none", "model_inference": False, "model_performance_qualified": False,
                "fixture": "dense-signed-dyadic-denominator16-v1", "cache_regime": "hot-reused-buffer-diagnostic",
                "model_streaming_bandwidth_inferred": False,
                "timing_scope": "host synchronous IPC dispatch wall time; not GPU-only",
                "candidate_timing_includes": ["partial", "merge"], "warmups_per_arm": WARMUPS,
                "blocks": BLOCKS, "helper_sha256": HELPER_SHA256, "lifecycle_sha256": LIFECYCLE_SHA256,
                "source_sha256": source_hashes, "worker_sha256": args.worker_sha256,
                "v5_image_sha256": args.v5_image_sha256, "candidate_image_sha256": args.candidate_image_sha256,
                "device_unique_id": args.device_unique_id, "wrapper_identity": wrapper,
                "supervisor_identity": supervisor,
                "worker_identity": child, "clean_teardown": True, "outer_admission_required": True, **result})
        except BaseException as error:
            cleanup_error = None
            with lifecycle.deferred_stop(deliver=False):
                if worker is not None and not clean:
                    try:
                        worker.abort()
                    except BaseException as cleanup:
                        cleanup_error = type(cleanup).__name__ + ": " + str(cleanup)
                save(args.output / "failure.json", {"error": type(error).__name__ + ": " + str(error),
                    "cleanup_error": cleanup_error, "outer_group_cleanup_required": True})
            raise
        finally:
            for descriptor, _, _ in held:
                os.close(descriptor)
            os.close(supervisor_fd)


if __name__ == "__main__":
    main()
