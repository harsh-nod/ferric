#!/usr/bin/env python3
"""Explicit, bounded synthetic probes for Ferric's six engineering TP kernels."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import stat
import struct
import subprocess
import time


LIMIT = 4 * 1024 * 1024
PREFIX = "ferric_qwen3_tp_"
GUARD = b"\xD3\x6A\x95\x2C" * 16


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(value):
    return hashlib.sha256(value).hexdigest()


def identity(info):
    return (info.st_dev, info.st_ino, info.st_mode, info.st_uid, info.st_gid,
            info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def bf16(bits, count):
    return struct.pack("<H", bits) * count


def buffer(name, data, element_bytes, expected=None):
    return {
        "name": name,
        "data": bytes(data),
        "element_bytes": element_bytes,
        "expected": bytes(data if expected is None else expected),
        "access": "read" if expected is None else "write",
    }


def cases():
    # Target8B/TP8: Q512, KV128, intermediate1536, replicated hidden4096.
    column_row = bf16(0x3F80, 1) + bf16(0, 4095)
    partial_row = bf16(0x3F80, 1) + bf16(0x3B80, 1) + bf16(0, 510)
    partial = struct.pack("<f", 1.0 + 1.0 / 256.0) * 4096
    query = b"".join(bf16(0x3E80 + (index % 4) * 128, 1) for index in range(512))
    key = bf16(0xBF00, 128)
    val = bf16(0x3F40, 128)
    sentinel = bf16(0x3555, 128)
    yield {
        "name": "column_projection", "symbol": PREFIX + "gemv_bf16_f32_bf16_v1",
        "buffers": [buffer("a", bf16(0x3F80, 4096), 2),
                    buffer("weights", column_row * 512, 2),
                    buffer("output", bf16(0x7F80, 512), 2, bf16(0x3F80, 512))],
        "scalars": [512, 4096, 1, 8, 1], "groups": 8,
    }
    yield {
        "name": "fp32_partial_before_rounding", "symbol": PREFIX + "gemv_partial_bf16_f32_v1",
        "buffers": [buffer("a", bf16(0x3F80, 512), 2),
                    buffer("weights", partial_row * 4096, 2),
                    buffer("output", b"\xA5" * len(partial), 4, partial)],
        "scalars": [4096, 512, 1, 8, 1], "groups": 64,
    }
    yield {
        "name": "swiglu_zero", "symbol": PREFIX + "swiglu_bf16_f32_v1",
        "buffers": [buffer("gate", bf16(0, 1536), 2),
                    buffer("up", bf16(0x3F80, 1536), 2),
                    buffer("output", bf16(0x7F80, 1536), 2, bf16(0, 1536))],
        "scalars": [1, 8], "groups": 24,
    }
    yield {
        "name": "rope_position_zero", "symbol": PREFIX + "rope_v1",
        "buffers": [buffer("query", query, 2), buffer("key", key, 2),
                    buffer("cos", struct.pack("<f", 1.0) * 64, 4),
                    buffer("sin", struct.pack("<f", 0.0) * 64, 4),
                    buffer("rotated_query", bf16(0x7F80, 512), 2, query),
                    buffer("rotated_key", bf16(0x7F80, 128), 2, key)],
        "scalars": [0, 1, 8], "groups": 1,
    }
    yield {
        "name": "kv_append_preserves_other_rows", "symbol": PREFIX + "kv_append_v1",
        "buffers": [buffer("key", key, 2), buffer("value", val, 2),
                    buffer("key_cache", sentinel * 3, 2, sentinel + key + sentinel),
                    buffer("value_cache", sentinel * 3, 2, sentinel + val + sentinel)],
        "scalars": [1, 3, 1, 8], "groups": 1,
    }
    yield {
        "name": "attention_ignores_stale_nan_suffix", "symbol": PREFIX + "gqa_decode_bf16_f32_v1",
        "buffers": [buffer("query", bf16(0, 512), 2),
                    buffer("key_cache", bf16(0, 256) + bf16(0x7FC0, 128), 2),
                    buffer("value_cache", bf16(0x3E80, 128) + bf16(0x3F40, 128) + bf16(0x7FC0, 128), 2),
                    buffer("output", bf16(0x7F80, 512), 2, bf16(0x3F00, 512))],
        "scalars": [2, 3, 1, 8], "groups": 4,
    }


def held_file(path, expected_hash, limit, machine):
    require(len(expected_hash) == 64 and all(c in "0123456789abcdef" for c in expected_hash), "bad expected digest")
    require(path.is_absolute() and path.resolve(strict=True) == path, "input path must be canonical")
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit, "invalid held file")
        parts = []
        while part := os.read(fd, 1024 * 1024):
            parts.append(part)
        data = b"".join(parts)
        require(identity(os.fstat(fd)) == identity(before) and digest(data) == expected_hash, "held input identity drifted")
        require(data[:7] == b"\x7fELF\x02\x01\x01" and len(data) >= 64, "expected 64-bit little-endian ELF")
        require(struct.unpack_from("<H", data, 18)[0] == machine, "ELF machine drifted")
        return fd, before, data
    except BaseException:
        os.close(fd)
        raise


def unique_json(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


class Worker:
    def __init__(self, fd, identity, device, directory):
        self.binary = fd
        self.identity = identity
        self.timeout = 60.0
        self.trace = (directory / "protocol.jsonl").open("x", encoding="utf-8")
        self.stderr = (directory / "worker.stderr").open("xb")
        try:
            self.process = subprocess.Popen(
                [f"/proc/self/fd/{fd}", "--device-unique-id", str(device),
                 "--allow-unauthenticated-machine-code"],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
                pass_fds=(fd,), close_fds=True, bufsize=0,
            )
            os.set_blocking(self.process.stdin.fileno(), False)
            os.set_blocking(self.process.stdout.fileno(), False)
            self.start = self.process_start()
            self.check_process()
            ready, payload = self.receive()
            require(ready == {"op": "ready", "protocol": 1, "target": "gfx950:xnack-",
                              "device_unique_id": device, "authority": "none"} and not payload,
                    "worker ready identity drifted")
        except BaseException:
            if hasattr(self, "process"):
                self.abort()
            else:
                self.trace.close()
                self.stderr.close()
            raise

    def process_start(self):
        text = Path(f"/proc/{self.process.pid}/stat").read_text(encoding="ascii")
        return int(text.rsplit(")", 1)[1].split()[19])

    def check_process(self):
        require(self.process.poll() is None and self.process_start() == self.start, "worker PID identity drifted")
        info = os.stat(f"/proc/{self.process.pid}/exe")
        require((info.st_dev, info.st_ino) == (self.identity.st_dev, self.identity.st_ino), "worker executable identity drifted")

    def transfer(self, payload=None, length=0):
        writing = payload is not None
        fd = self.process.stdin.fileno() if writing else self.process.stdout.fileno()
        remaining = memoryview(payload) if writing else length
        output = bytearray()
        deadline = time.monotonic() + self.timeout
        while remaining:
            seconds = deadline - time.monotonic()
            require(seconds > 0, "bounded worker IPC timed out")
            readable, writable, _ = select.select([] if writing else [fd], [fd] if writing else [], [], seconds)
            require(writable if writing else readable, "bounded worker IPC timed out")
            try:
                if writing:
                    count = os.write(fd, remaining)
                    require(count > 0, "worker input closed")
                    remaining = remaining[count:]
                else:
                    part = os.read(fd, remaining)
                    require(part, "worker output truncated")
                    output.extend(part)
                    remaining -= len(part)
            except BlockingIOError:
                continue
        return bytes(output)

    def receive(self):
        length = struct.unpack("<I", self.transfer(length=4))[0]
        require(0 < length <= 65536, "worker header limit")
        header = json.loads(self.transfer(length=length), object_pairs_hook=unique_json)
        require(isinstance(header, dict), "worker header is not an object")
        size = header.get("payload_bytes", 0) if header.get("op") == "read" else 0
        require(type(size) is int and 0 <= size <= LIMIT, "worker payload limit")
        payload = self.transfer(length=size)
        self.trace.write(json.dumps({"response": header, "payload_sha256": digest(payload)}) + "\n")
        self.trace.flush()
        require(header.get("op") != "error", f"worker rejected command: {header}")
        return header, payload

    def command(self, command, payload=b"", expected=None):
        self.check_process()
        raw = json.dumps(command, separators=(",", ":")).encode("utf-8")
        require(0 < len(raw) <= 65536, "command header limit")
        require(command.get("payload_bytes", 0) == len(payload), "command payload mismatch")
        self.trace.write(json.dumps({"command": command, "payload_sha256": digest(payload)}) + "\n")
        self.trace.flush()
        self.transfer(struct.pack("<I", len(raw)) + raw + payload)
        response, data = self.receive()
        require(response.get("op") == expected, "unexpected response kind")
        return response, data

    def allocate(self, data):
        result, payload = self.command({"op": "allocate", "bytes": len(data)}, expected="allocated")
        require(result.get("bytes") == len(data) and not payload, "allocation response drifted")
        identifier = result["buffer"]
        require(type(identifier) is int and identifier > 0, "invalid allocation ID")
        for start in range(0, len(data), LIMIT):
            chunk = data[start:start + LIMIT]
            self.command({"op": "write", "buffer": identifier, "offset": start,
                          "payload_bytes": len(chunk)}, chunk, "written")
        return identifier

    def read(self, identifier, length):
        output = bytearray()
        for start in range(0, length, LIMIT):
            count = min(LIMIT, length - start)
            header, chunk = self.command({"op": "read", "buffer": identifier,
                                          "offset": start, "bytes": count}, expected="read")
            require(header.get("payload_bytes") == count and len(chunk) == count, "short device read")
            output.extend(chunk)
        return bytes(output)

    def finish(self):
        self.check_process()
        self.command({"op": "close"}, expected="closed")
        require(self.process.wait(timeout=10) == 0, "worker teardown failed")
        self.trace.close()
        self.stderr.close()

    def abort(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        self.trace.close()
        self.stderr.close()


def probe(worker, case, artifact, artifact_hash):
    loaded, payload = worker.command({"op": "load_kernel", "payload_bytes": len(artifact),
                                     "object_sha256": list(bytes.fromhex(artifact_hash)),
                                     "symbol": case["symbol"]}, artifact, "loaded_kernel")
    require(not payload, "load response payload")
    metadata = loaded["metadata"]
    require(metadata["symbol"] == case["symbol"] and metadata["object_sha256"] == list(bytes.fromhex(artifact_hash)), "loaded artifact identity")
    require(metadata["wavefront_size"] == 64 and metadata["private_segment_bytes"] == 0
            and metadata["group_segment_bytes"] == 0 and metadata["kernarg_alignment"] == 8,
            "unexpected resource or ABI requirements")
    args = metadata["explicit_arguments"]
    require(len(args) == len(case["buffers"]) * 2 + len(case["scalars"]), "explicit argument count")
    kernarg_bytes = metadata["kernarg_bytes"]
    require(type(kernarg_bytes) is int and 256 <= kernarg_bytes <= 65536,
            "kernarg byte limit")
    encoded = bytearray(kernarg_bytes)
    pointers = []
    allocated = []
    offset = 0
    for index, record in enumerate(case["buffers"]):
        pointer, count = args[index * 2:index * 2 + 2]
        require(pointer["offset"] == offset and pointer["bytes"] == 8 and pointer["global_buffer"]
                and pointer["pointee_alignment"] == record["element_bytes"]
                and pointer["access"] == record["access"], "buffer metadata differs from exact source ABI")
        require(count["offset"] == offset + 8 and count["bytes"] == 8 and not count["global_buffer"], "slice length metadata")
        identifier = worker.allocate(GUARD + record["data"] + GUARD)
        require(identifier not in allocated, "allocation ID reused")
        allocated.append(identifier)
        struct.pack_into("<Q", encoded, offset + 8, len(record["data"]) // record["element_bytes"])
        pointers.append({"kernarg_offset": offset, "buffer": identifier, "buffer_offset": len(GUARD),
                         "extent_bytes": len(record["data"]), "access": record["access"]})
        offset += 16
    for argument, value in zip(args[len(case["buffers"]) * 2:], case["scalars"], strict=True):
        require(argument["offset"] == offset and argument["bytes"] == 4 and not argument["global_buffer"], "scalar metadata")
        struct.pack_into("<I", encoded, offset, value)
        offset += 4
    implicit = metadata["implicit_argument_offset"]
    require(implicit == (offset + 7) // 8 * 8 and metadata["implicit_argument_bytes"] == 256
            and len(encoded) == implicit + 256, "hidden argument layout drifted")
    result, payload = worker.command({"op": "dispatch", "kernel": loaded["kernel"],
                                     "payload_bytes": len(encoded), "workgroup": [64, 1, 1],
                                     "grid": [case["groups"] * 64, 1, 1], "pointers": pointers,
                                     "timeout_ms": 30000}, bytes(encoded), "dispatched")
    require(not payload and result["elapsed_ns"] > 0, "dispatch completion response")
    checks = []
    for identifier, record in zip(allocated, case["buffers"], strict=True):
        guarded = worker.read(identifier, len(record["data"]) + 2 * len(GUARD))
        require(guarded[:len(GUARD)] == GUARD and guarded[-len(GUARD):] == GUARD,
                f"{case['name']}:{record['name']} slice guard changed")
        actual = guarded[len(GUARD):-len(GUARD)]
        require(actual == record["expected"], f"{case['name']}:{record['name']} numerical/input-immutability mismatch")
        checks.append({"name": record["name"], "access": record["access"], "bytes": len(actual),
                       "sha256": digest(actual), "guard_bytes_each_side": len(GUARD), "guards_unchanged": True})
        worker.command({"op": "free", "buffer": identifier}, expected="freed")
    return {"name": case["name"], "symbol": case["symbol"], "elapsed_ns": result["elapsed_ns"],
            "metadata": metadata, "checks": checks}


def self_test():
    fixtures = list(cases())
    require(len(fixtures) == 6 and len({case["symbol"] for case in fixtures}) == 6, "probe roster")
    for case in fixtures:
        for record in case["buffers"]:
            require(0 < len(record["data"]) <= LIMIT and len(record["data"]) == len(record["expected"]), "fixture extent")
            require(len(record["data"]) % record["element_bytes"] == 0, "fixture element units")
    require(struct.unpack("<f", fixtures[1]["buffers"][2]["expected"][:4])[0] == 1.00390625, "partial fixture lost precision")
    require(fixtures[5]["buffers"][2]["data"][-2:] == bf16(0x7FC0, 1), "missing stale NaN suffix")
    print("PASS: six bounded synthetic fixtures; no worker launched")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--worker-sha256")
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--artifact-sha256")
    parser.add_argument("--device-unique-id", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.self_test:
        require(not args.run, "self-test cannot execute GPU work")
        self_test()
        return
    require(args.run and all(value is not None for value in [args.worker, args.worker_sha256,
            args.artifact, args.artifact_sha256, args.device_unique_id, args.output]), "explicit run, identities, device and output are required")
    require(0 < args.device_unique_id <= (1 << 64) - 1, "device unique ID range")
    require(args.output.is_absolute(), "output must be absolute")
    args.output.mkdir(mode=0o700)
    source_identity = digest(Path(__file__).read_bytes())
    worker_fd, worker_stat, _ = held_file(args.worker, args.worker_sha256, 512 * 1024 * 1024, 62)
    artifact_fd = None
    worker = None
    try:
        artifact_fd, artifact_stat, artifact = held_file(args.artifact, args.artifact_sha256, 64 * 1024 * 1024, 224)
        worker = Worker(worker_fd, worker_stat, args.device_unique_id, args.output)
        results = [probe(worker, case, artifact, args.artifact_sha256) for case in cases()]
        worker.finish()
        require(identity(os.fstat(worker_fd)) == identity(worker_stat)
                and identity(os.fstat(artifact_fd)) == identity(artifact_stat), "held identity changed during run")
        require(source_identity == digest(Path(__file__).read_bytes()), "probe source changed during run")
        report = {"schema": "FerricTpSyntheticKernelProbeV1", "authority": "none",
                  "model_inference": False, "source_sha256": source_identity,
                  "timing_scope": "worker synchronous dispatch wall-clock nanoseconds",
                  "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
                  "device_unique_id": args.device_unique_id, "worker_pid": worker.process.pid,
                  "worker_start_ticks": worker.start, "clean_teardown": True, "results": results}
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2)
            output.write("\n")
        print("PASS: six real GPU synthetic probes, immutable inputs and clean teardown; no model inference claim")
    except BaseException:
        if worker is not None:
            worker.abort()
        raise
    finally:
        if artifact_fd is not None:
            os.close(artifact_fd)
        os.close(worker_fd)


if __name__ == "__main__":
    main()
