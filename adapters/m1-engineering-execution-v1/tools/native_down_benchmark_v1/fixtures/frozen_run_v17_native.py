#!/usr/bin/env python3
"""Bounded controller-ingress V17 probe, not HTTP or sustained qualification."""

import argparse
import collections
import hashlib
import json
import math
import os
from pathlib import Path
import select
import selectors
import signal
import stat
import subprocess
import time

MODES = ("baseline", "query-hoist-v14", "wave-rmsnorm-v15", "combined")
EVENT = "FerricQwen3TpLiveEventV1"
COMMAND = "FerricQwen3TpLiveCommandV1"
PROFILE = "wave-target-v17-live-v1"
MAX_STREAM = 8 * 1024 * 1024
MAX_LINE = 1024 * 1024
FLAGS = {"--live-stdin", "--allow-unauthenticated-machine-code", "--runtime-cache-admission",
         "--runtime-operational", "--queue-rollover", "--disable-prefix-cache", "--prune-output-head"}
VALUES = {"--source", "--target-artifact", "--target-head-artifact", "--argmax-artifact",
          "--attention-artifact", "--rmsnorm-artifact", "--worker", "--worker-sha256",
          "--device-unique-id", "--submission", "--context", "--pages", "--max-batches",
          "--layer-projection"}
ARTIFACTS = {"--target-artifact": None, "--target-head-artifact": "fp32_head_artifact",
             "--argmax-artifact": "argmax_artifact", "--attention-artifact": "attention_artifact",
             "--rmsnorm-artifact": "rmsnorm_artifact"}
TARGET_MODEL_ID = "f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a"
# Exact b968/c189 file pins from Ferric's canonical source contract.
TARGET_FILES = {
    "config.json": (728, "f7c4eadfbbf522470667b797a3c89be2524832d2d599797248dc304fff447c30"),
    "tokenizer.json": (11422654, "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4"),
    "tokenizer_config.json": (9732, "d5d09f07b48c3086c508b30d1c9114bd1189145b74e982a265350c923acd8101"),
    "model.safetensors.index.json": (32878, "f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc"),
    "model-00001-of-00005.safetensors": (3996250744, "31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f"),
    "model-00002-of-00005.safetensors": (3993160032, "5991236cea6fe21f3d43cab0f0e84448734fbbe0789816202989f2ddc9d18282"),
    "model-00003-of-00005.safetensors": (3959604768, "c5185c4794be2d8a9784d5753c9922db38df478ce11f9ed0b415b7304d896836"),
    "model-00004-of-00005.safetensors": (3187841392, "b5ee7de71fbf17db3d5704e0c8f2bc7d005ca9e1d7ca2aeb19827b0cfcaa917a"),
    "model-00005-of-00005.safetensors": (1244659840, "20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff"),
}
DRAFT_FILES = {
    "config.json": (726, "660db3b73d788119c04535e48cf9be5f55bc3100841a718637ae695b442f27dd"),
    "model.safetensors": (1503300328, "f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b"),
    "tokenizer.json": TARGET_FILES["tokenizer.json"],
    "tokenizer_config.json": TARGET_FILES["tokenizer_config.json"],
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def integer(value, label):
    require(type(value) is int and value >= 0, label + " must be a nonnegative integer")
    return value


def sha(value):
    require(isinstance(value, str) and len(value) == 64
            and all(c in "0123456789abcdef" for c in value), "invalid SHA256")
    return value


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key: " + key)
        result[key] = value
    return result


def decode(raw):
    return json.loads(raw, object_pairs_hook=unique_object,
                      parse_constant=lambda value: require(False, "nonfinite JSON: " + value))


def identity(path, expected=None, maximum=512 * 1024 * 1024):
    path = Path(path)
    require(path.is_absolute() and not path.is_symlink(), "absolute nonsymlink input required")
    with path.open("rb") as handle:
        before = os.fstat(handle.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= maximum,
                "nonempty bounded regular input required: " + str(path))
        digest = hashlib.file_digest(handle, "sha256").hexdigest()
        after = os.fstat(handle.fileno())
    keys = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    require(all(getattr(before, k) == getattr(after, k) == getattr(path.stat(), k) for k in keys),
            "input changed while hashing: " + str(path))
    if expected is not None:
        require(digest == sha(expected), "input SHA256 mismatch: " + str(path))
    return {"path": str(path), "bytes": before.st_size, "sha256": digest}


def bound_json(binding):
    require(set(binding) == {"path", "sha256"}, "exact path/SHA256 binding required")
    receipt = identity(binding["path"], binding["sha256"], MAX_LINE)
    value = decode(Path(receipt["path"]).read_bytes())
    identity(receipt["path"], receipt["sha256"], MAX_LINE)
    return value


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n")


def arguments(values):
    require(isinstance(values, list) and all(isinstance(v, str) and v for v in values),
            "common_args must be nonempty strings")
    result = {}
    iterator = iter(values)
    for flag in iterator:
        require(flag in FLAGS | VALUES and flag not in result, "unknown/duplicate argument: " + flag)
        result[flag] = True if flag in FLAGS else next(iterator, None)
        require(result[flag] is not None, "missing argument value: " + flag)
    require(set(result) == FLAGS | VALUES, "complete closed V17 common arguments required")
    for key, expected in {"--submission": "ordered", "--context": "8192", "--pages": "512",
                          "--layer-projection": "c1-wave"}.items():
        require(result[key] == expected, "closed profile mismatch: " + key)
    require(result["--device-unique-id"].isdigit() and int(result["--device-unique-id"]) > 0,
            "positive device identity required")
    require(result["--max-batches"].isdigit() and 540 <= int(result["--max-batches"]) <= 1000000,
            "max-batches must accommodate all four sequential 128/128 requests")
    for key in {"--source", "--worker"} | set(ARTIFACTS):
        require(Path(result[key]).is_absolute(), "absolute input path required: " + key)
    sha(result["--worker-sha256"])
    return result


def workload_reference(plan):
    workload = bound_json(plan["workload"])
    reference = bound_json(plan["reference"])
    target = bound_json(plan["target_manifest"])
    require(workload.get("schema") == "FerricCompetitiveWorkloadV1"
            and workload.get("model") == "Qwen/Qwen3-8B"
            and len(workload.get("requests", [])) == 1, "one matched Qwen request required")
    request = workload["requests"][0]
    require(type(request.get("max_tokens")) is int and request["max_tokens"] == 128
            and isinstance(request.get("prompt"), str) and 0 < len(request["prompt"].encode()) <= 8192,
            "frozen 128-output bounded prompt required")
    require(reference.get("schema") == "FerricMatched128ReferenceV1"
            and reference.get("policy") == "exact-greedy-token-ids-and-decoded-utf8-v1"
            and reference.get("workload_sha256") == plan["workload"]["sha256"]
            and reference.get("target_manifest_sha256") == plan["target_manifest"]["sha256"]
            and isinstance(reference.get("independent_producer"), str)
            and reference["independent_producer"].lower() not in ("", "ferric"),
            "independent reference binding mismatch")
    sha(reference.get("producer_evidence_sha256"))
    for key in ("prompt_token_ids", "generated_token_ids"):
        tokens = reference.get(key)
        require(isinstance(tokens, list) and len(tokens) == 128
                and all(type(x) is int and 0 <= x < 151936 for x in tokens), "128 reference IDs required")
    decoded = bytes.fromhex(reference["generated_utf8_hex"])
    require(0 < len(decoded) <= 32768 and decoded.decode("utf-8").encode() == decoded,
            "bounded reference UTF8 required")
    require(target.get("schema") == "FerricCanonicalTargetFileHashesV1"
            and len(target.get("files", [])) == 9, "canonical nine-file target manifest required")
    names = set()
    for file in target["files"]:
        name = file.get("name")
        require(name in TARGET_FILES and name not in names, "invalid/duplicate canonical target entry")
        require((file.get("bytes"), file.get("sha256")) == TARGET_FILES[name], "target manifest pin mismatch")
        names.add(name)
    return request, reference


def model_identities(options):
    root = Path(options["--source"])
    require(root.is_dir() and not root.is_symlink()
            and {p.name for p in root.iterdir()} == {"target", "draft"}, "exact canonical model root required")
    result = {}
    inodes = set()
    for role, files in (("target", TARGET_FILES), ("draft", DRAFT_FILES)):
        directory = root / role
        require(directory.is_dir() and not directory.is_symlink()
                and {p.name for p in directory.iterdir()} == set(files), "exact model file roster required")
        for name, (size, digest) in files.items():
            path = directory / name
            before = path.stat()
            require(before.st_size == size and before.st_nlink == 1
                    and (before.st_dev, before.st_ino) not in inodes, "model extent/alias mismatch")
            inodes.add((before.st_dev, before.st_ino))
            result[role + "/" + name] = identity(path, digest, 4 * 1024 * 1024 * 1024)
    return result


def input_identities(plan, options):
    result = {"controller": identity(plan["controller"]["path"], plan["controller"]["sha256"]),
              "worker": identity(options["--worker"], options["--worker-sha256"])}
    for option in ARTIFACTS:
        root = Path(options[option])
        require(root.is_dir() and not root.is_symlink()
                and {p.name for p in root.iterdir()} == {"observation.json", "observation.hsaco"},
                "exact two-file artifact directory required")
        result[option] = {name: identity(root / name, maximum=64 * 1024 * 1024)
                          for name in ("observation.json", "observation.hsaco")}
    return result


def check_setup(value, mode, options, inputs):
    attention = "query-hoist-v14" if mode in ("query-hoist-v14", "combined") else "wave"
    norm = "wave-v15" if mode in ("wave-rmsnorm-v15", "combined") else "baseline"
    expected = {"schema": "FerricQwen3TpBatchSetupV2", "authority": "none",
                "live_profile": PROFILE, "wave_target_mode": mode, "layer_projection": "c1-wave",
                "attention_mode": attention, "rmsnorm_mode": norm, "tensor_parallel": 1,
                "model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
                "head_precision": "fp32-v8", "argmax_mode": "wave-v11", "submission": "ordered",
                "runtime_ordered_batches": True, "prefix_cache": False, "context_tokens": 8192,
                "physical_pages": 512, "prefill_chunk": 16, "batch_tokens": 32,
                "performance_qualified": False, "serving_qualified": False,
                "benchmark_admitted": False, "serving_admitted": False,
                "controller_sha256": inputs["controller"]["sha256"],
                "worker_sha256": inputs["worker"]["sha256"],
                "running_worker_sha256": [inputs["worker"]["sha256"]],
                "device_unique_ids": [int(options["--device-unique-id"])]}
    expected["target_model_id"] = TARGET_MODEL_ID
    for key, expected_value in expected.items():
        require(type(value.get(key)) is type(expected_value) and value[key] == expected_value,
                "setup mismatch: " + key)
    for option, field in ARTIFACTS.items():
        artifact = value if field is None else value.get(field, {})
        require(artifact.get("artifact_hsaco_id") == inputs[option]["observation.hsaco"]["sha256"]
                and artifact.get("artifact_manifest_id") == inputs[option]["observation.json"]["sha256"],
                "setup artifact mismatch: " + option)
        sha(artifact.get("artifact_handoff_id"))
    for key in ("model_bundle_id", "target_model_id"):
        sha(value.get(key))
    profile = value.get("performance_profile", {})
    for key, expected_value in {"wave_target_mode": mode, "attention": attention,
                                "rmsnorm_mode": norm, "layer_projection": "c1-wave",
                                "runtime_profiling": False, "dispatch_sequences": False,
                                "runtime_cache_admission": True, "runtime_operational": True,
                                "queue_rollover": True}.items():
        require(type(profile.get(key)) is type(expected_value) and profile[key] == expected_value,
                "performance profile mismatch: " + key)
    return {key: value[key] for key in ("model_bundle_id", "target_model_id")}


class Controller:
    def __init__(self, argv, output, deadline):
        self.output = Path(output)
        self.deadline = deadline
        self.queue = collections.deque()
        self.pending = bytearray()
        self.counts = {"stdout": 0, "stderr": 0}
        self.logs = {name: (self.output / (name + ".raw")).open("xb")
                     for name in ("stdin", "stdout", "stderr")}
        self.received = (self.output / "received.jsonl").open("x")
        self.proc = None
        self.reaped = False
        self.clean_exit = False
        self.selector = selectors.DefaultSelector()
        try:
            self.proc = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                         stderr=subprocess.PIPE, bufsize=0, start_new_session=True)
            save(self.output / "process.json", {"pid": self.proc.pid, "owned_pgid": self.proc.pid,
                                               "argv": argv, "start_new_session": True})
            for name in ("stdin", "stdout", "stderr"):
                os.set_blocking(getattr(self.proc, name).fileno(), False)
            for name in ("stdout", "stderr"):
                self.selector.register(getattr(self.proc, name), selectors.EVENT_READ, name)
        except BaseException:
            self.close()
            raise

    def send(self, value, deadline):
        raw = (json.dumps(value, separators=(",", ":"), allow_nan=False) + "\n").encode()
        require(len(raw) <= 32768, "bounded command required")
        self.logs["stdin"].write(raw)
        self.logs["stdin"].flush()
        view = memoryview(raw)
        while view:
            remaining = min(deadline, self.deadline) - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("controller stdin deadline")
            _, writable, _ = select.select([], [self.proc.stdin], [], remaining)
            if writable:
                try:
                    view = view[os.write(self.proc.stdin.fileno(), view):]
                except BlockingIOError:
                    continue

    def pump(self, deadline):
        remaining = min(deadline, self.deadline) - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("controller event deadline")
        for key, _ in self.selector.select(min(remaining, 0.2)):
            raw = os.read(key.fileobj.fileno(), 65536)
            if not raw:
                self.selector.unregister(key.fileobj)
                continue
            name = key.data
            self.counts[name] += len(raw)
            require(self.counts[name] <= MAX_STREAM, name + " stream bound exceeded")
            self.logs[name].write(raw)
            self.logs[name].flush()
            if name == "stdout":
                self.pending.extend(raw)
                while b"\n" in self.pending:
                    line, _, suffix = self.pending.partition(b"\n")
                    require(len(line) <= MAX_LINE and line, "invalid bounded JSONL record")
                    self.pending = bytearray(suffix)
                    value = decode(line)
                    require(isinstance(value, dict), "object event required")
                    self.queue.append(value)
                    self.received.write(json.dumps({"received_monotonic_ns": time.monotonic_ns(),
                                                     "record": value}) + "\n")
                    self.received.flush()
                require(len(self.pending) <= MAX_LINE, "unterminated stdout line bound exceeded")

    def next(self, deadline):
        while not self.queue:
            if not self.selector.get_map():
                require(not self.pending, "unterminated stdout record")
                raise ValueError("controller EOF before expected record")
            self.pump(deadline)
        return self.queue.popleft()

    def finish(self, deadline):
        self.proc.stdin.close()
        while self.selector.get_map():
            self.pump(deadline)
        require(not self.pending and not self.queue, "trailing stdout after closed record")
        while True:
            status = os.waitid(os.P_PID, self.proc.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if status is not None:
                require(status.si_code == os.CLD_EXITED and status.si_status == 0,
                        "controller exit is not zero")
                members = self.owned_group_members()
                save(self.output / "group-before-cleanup.json", members)
                require([row["pid"] for row in members] == [self.proc.pid],
                        "clean controller exit left owned descendants")
                self.clean_exit = True
                return
            if time.monotonic() >= min(deadline, self.deadline):
                raise TimeoutError("controller exit deadline")
            time.sleep(0.01)

    def close(self):
        cleanup = {"owned_pgid": None, "term_sent": False, "kill_sent": False, "errors": []}
        # Teardown must finish even if the outer wrapper repeats its stop signal.
        previous_handlers = {sig: signal.signal(sig, signal.SIG_IGN)
                             for sig in (signal.SIGINT, signal.SIGTERM)}
        try:
            return self.close_owned(cleanup)
        finally:
            for sig, handler in previous_handlers.items():
                signal.signal(sig, handler)

    def close_owned(self, cleanup):
        if self.proc is not None:
            cleanup["owned_pgid"] = self.proc.pid
            # Keep the child unreaped until signaling is over, reserving its PID/PGID.
            # No poll()/wait() may run before this loop: numeric IDs could be reused.
            require(not self.reaped, "cannot signal a reaped controller group")
            for sig, label in ((signal.SIGTERM, "term_sent"), (signal.SIGKILL, "kill_sent")):
                try:
                    os.killpg(self.proc.pid, sig)
                    cleanup[label] = True
                except ProcessLookupError:
                    break
                except OSError as error:
                    cleanup["errors"].append("group signal: " + str(error))
                    break
                if sig == signal.SIGTERM and not self.clean_exit:
                    time.sleep(2)
            until = time.monotonic() + 2
            while time.monotonic() < until:
                try:
                    members = self.owned_group_members()
                except (OSError, ValueError, IndexError) as error:
                    cleanup["errors"].append("group inspection: " + str(error))
                    break
                if all(row["pid"] == self.proc.pid for row in members):
                    break
                time.sleep(0.05)
            try:
                self.proc.wait(timeout=3)
                self.reaped = True
            except subprocess.TimeoutExpired:
                cleanup["reap_failed"] = True
                cleanup["errors"].append("bounded controller reap failed")
            cleanup["returncode"] = self.proc.returncode
            try:
                cleanup["owned_group_absent"] = not self.group_alive()
            except OSError as error:
                cleanup["owned_group_absent"] = False
                cleanup["errors"].append("final group inspection: " + str(error))
            for name in ("stdout", "stderr"):
                stream = getattr(self.proc, name)
                if not stream.closed:
                    os.set_blocking(stream.fileno(), False)
                while not stream.closed and self.counts[name] < MAX_STREAM:
                    try:
                        raw = os.read(stream.fileno(), min(65536, MAX_STREAM - self.counts[name]))
                    except BlockingIOError:
                        break
                    if not raw:
                        break
                    self.logs[name].write(raw)
                    self.counts[name] += len(raw)
            for name in ("stdin", "stdout", "stderr"):
                getattr(self.proc, name).close()
        self.selector.close()
        for handle in self.logs.values():
            handle.close()
        self.received.close()
        cleanup["cleanup_ok"] = not cleanup["errors"] and cleanup.get("owned_group_absent", True)
        save(self.output / "cleanup.json", cleanup)
        return cleanup

    def group_alive(self):
        try:
            os.killpg(self.proc.pid, 0)
            return True
        except ProcessLookupError:
            return False

    def owned_group_members(self):
        require(not self.reaped, "group inspection requires unreaped leader")
        members = []
        inspected = 0
        deadline = time.monotonic() + 2
        # Read only bounded /proc stat records, never command lines or process memory.
        # The unreaped session leader prevents PGID reuse throughout this census.
        with os.scandir("/proc") as entries:
            for entry in entries:
                if not entry.name.isdigit():
                    continue
                inspected += 1
                require(inspected <= 32768 and time.monotonic() < deadline, "bounded process census exceeded")
                try:
                    with open(entry.path + "/stat", "rb") as handle:
                        fields = handle.read(4096).rsplit(b")", 1)[1].split()
                except (FileNotFoundError, ProcessLookupError):
                    continue
                if int(fields[2]) == self.proc.pid:
                    require(int(fields[3]) == self.proc.pid, "owned group session drift")
                    members.append({"pid": int(entry.name), "state": fields[0].decode("ascii")})
        require(any(row["pid"] == self.proc.pid for row in members), "unreaped session leader disappeared")
        return sorted(members, key=lambda row: row["pid"])


class Events:
    def __init__(self):
        self.emission = 0
        self.batches = 0
        self.dispatches = 0
        self.last_batch_end = 0

    def validate(self, event):
        require(event.get("schema") == EVENT and event.get("authority") == "none", "wrong live event schema")
        stamp = integer(event.get("emission_started_ns"), "emission_started_ns")
        require(stamp >= self.emission, "nonmonotonic live emission")
        self.emission = stamp
        require(event.get("event") not in ("rejected", "cancel", "stopped"), "request rejected/cancelled/stopped")
        if event.get("event") == "batch":
            self.batches += 1
            require(integer(event.get("tick"), "tick") == self.batches - 1
                    and integer(event.get("batch_id"), "batch ID") == self.batches,
                    "batch sequence drift")
            start = integer(event.get("started_ns"), "batch start")
            end = integer(event.get("completed_ns"), "batch completion")
            require(self.last_batch_end <= start <= end <= stamp, "invalid batch timing")
            self.last_batch_end = end
            rows = integer(event.get("rows"), "batch rows")
            outputs = integer(event.get("outputs"), "batch outputs")
            require(1 <= rows <= 32 and 0 <= outputs <= 1 and outputs <= rows
                    and event.get("output_head_rows") == outputs, "batch output geometry drift")
            count = 616 if outputs else 613
            require(event.get("rank_dispatch_counts") == [count], "batch dispatch count drift")
            self.dispatches += count


def collect_request(controller, events, request_id, name, reference, deadline):
    queued = admitted = None
    tokens, stamps = [], []
    arrival_floor = events.emission
    batches = rows = 0
    pending_token = None
    while True:
        event = controller.next(deadline)
        events.validate(event)
        kind = event.get("event")
        if kind == "batch":
            require(admitted is not None and batches < 135, "batch outside admitted request")
            expected_rows = 16 if batches < 8 else 1
            expected_outputs = 0 if batches < 7 else 1
            require(event["rows"] == expected_rows and event["outputs"] == expected_outputs
                    and event["started_ns"] >= admitted, "128/128 physical batch geometry mismatch")
            require((pending_token == event["completed_ns"]) if expected_outputs else pending_token is None,
                    "token completion does not match producing batch")
            pending_token = None
            rows += event["rows"]
            batches += 1
            continue
        require(event.get("request_id") == request_id and event.get("name") == name,
                "foreign request event")
        if kind == "queued":
            require(queued is None and admitted is None and not tokens, "duplicate/reordered queued event")
            queued = integer(event.get("arrival_ns"), "arrival")
            require(arrival_floor <= queued <= integer(event.get("queued_ns"), "queued") <= events.emission
                    and event.get("prompt_tokens") == 128, "queued metadata mismatch")
        elif kind == "admission":
            require(queued is not None and admitted is None and not tokens, "reordered admission")
            admitted = integer(event.get("admitted_ns"), "admitted")
            require(event.get("arrival_ns") == queued and queued <= admitted <= events.emission
                    and event.get("queue_wait_ns") == admitted - queued
                    and event.get("prompt_tokens") == reference["prompt_token_ids"]
                    and event.get("prompt_token_count") == 128
                    and event.get("cached_tokens") == 0 and event.get("cached_pages") == 0,
                    "admission/reference mismatch")
        elif kind == "token":
            require(admitted is not None and len(tokens) < 128 and pending_token is None,
                    "token before admission, beyond limit, or without previous batch")
            stamp = integer(event.get("completed_ns"), "token completion")
            require((stamps[-1] if stamps else admitted) <= stamp <= events.emission
                    and integer(event.get("index"), "token index") == len(tokens)
                    and integer(event.get("token"), "token ID") == reference["generated_token_ids"][len(tokens)]
                    and event.get("finished") is (len(tokens) == 127), "token/timestamp/reference mismatch")
            tokens.append(event["token"])
            stamps.append(stamp)
            pending_token = stamp
        elif kind == "request":
            require(admitted is not None and len(tokens) == 128 and batches == 135 and rows == 255
                    and pending_token is None, "incomplete request or physical batch stream")
            require(event.get("admitted") is True and event.get("state") == "Completed"
                    and event.get("cancelled_ns") is None and event.get("cached_prefix_tokens") == 0
                    and event.get("arrival_ns") == queued
                    and event.get("prompt_tokens") == reference["prompt_token_ids"]
                    and event.get("prompt_token_count") == 128
                    and event.get("generated_tokens") == tokens
                    and event.get("output_timestamps_ns") == stamps, "completed request mismatch")
            raw = event.get("generated_utf8_bytes")
            require(isinstance(raw, list) and all(type(x) is int and 0 <= x <= 255 for x in raw), "invalid decoded bytes")
            require(bytes(raw).hex() == reference["generated_utf8_hex"]
                    and event.get("generated_text") == bytes(raw).decode("utf-8"), "decoded output mismatch")
            intervals = [b - a for a, b in zip(stamps, stamps[1:])]
            ttft, tpot = stamps[0] - queued, (stamps[-1] - stamps[0]) // 127
            require(event.get("ttft_ns") == ttft and event.get("tpot_ns") == tpot
                    and event.get("decode_intervals_ns") == intervals and stamps[-1] > queued,
                    "reported latency differs from token timeline")
            return {"request_id": request_id, "name": name, "arrival_ns": queued,
                    "completed_ns": stamps[-1], "ttft_ns": ttft, "tpot_ns": tpot,
                    "output_tokens": 128, "exact_reference_match": True,
                    "ingress_output_tokens_per_second": 128e9 / (stamps[-1] - queued)}
        else:
            raise ValueError("unexpected request event: " + str(kind))


def run_arm(argv, output, mode, request, reference, measured, timeouts, setup_check):
    output = Path(output)
    output.mkdir(mode=0o700)
    deadline = time.monotonic() + timeouts["arm_seconds"]
    controller = None
    result = {"mode": mode, "accepted": False, "requests": []}
    try:
        controller = Controller(argv, output, deadline)
        phase = min(deadline, time.monotonic() + timeouts["setup_seconds"])
        setup = controller.next(phase)
        result["model_identity"] = setup_check(setup)
        events = Events()
        ready = controller.next(phase)
        events.validate(ready)
        require(ready.get("event") == "ready" and ready.get("clock") == "monotonic_ns_since_live_start"
                and ready.get("eos_policy") == "fixed output count"
                and ready.get("context_tokens") == 8192 and ready.get("physical_pages") == 512,
                "exact ready envelope required")
        for index in range(measured + 1):
            request_id = index + 1
            name = "warmup-0" if index == 0 else "measured-" + str(index)
            phase = min(deadline, time.monotonic() + timeouts["request_seconds"])
            controller.send({"schema": COMMAND, "op": "submit", "request_id": request_id,
                             "name": name, "prompt": request["prompt"], "new_tokens": 128}, phase)
            record = collect_request(controller, events, request_id, name, reference, phase)
            record["warmup"] = index == 0
            result["requests"].append(record)
            save(output / "partial-result.json", result)
        phase = min(deadline, time.monotonic() + 15)
        controller.send({"schema": COMMAND, "op": "drain"}, phase)
        draining = controller.next(phase)
        events.validate(draining)
        require(draining.get("event") == "draining" and draining.get("reason") == "command", "drain missing")
        stopped = controller.next(phase)
        require(stopped.get("schema") == EVENT and stopped.get("authority") == "none"
                and stopped.get("event") == "stopped" and stopped.get("reason") == "drained"
                and stopped.get("batches") == events.batches
                and integer(stopped.get("emission_started_ns"), "stopped emission") >= events.emission,
                "clean stopped event missing")
        closed = controller.next(phase)
        require(closed.get("schema") == "FerricQwen3TpBatchClosedV2"
                and closed.get("live_profile") == PROFILE and closed.get("wave_target_mode") == mode
                and closed.get("authority") == "none" and closed.get("all_workers_exited") is True
                and closed.get("execution_completed") is True
                and closed.get("rank_dispatch_counts") == [events.dispatches], "clean closed record missing")
        controller.finish(phase)
        samples = result["requests"][1:]
        span = samples[-1]["completed_ns"] - samples[0]["arrival_ns"]
        require(span > 0, "positive measured span required")
        result.update(accepted=True, mean_ttft_ms=sum(x["ttft_ns"] for x in samples) / measured / 1e6,
                      mean_tpot_ms=sum(x["tpot_ns"] for x in samples) / measured / 1e6,
                      measured_ingress_span_output_tokens_per_second=128 * measured * 1e9 / span,
                      measured_span_ns=span, batches=events.batches, dispatches=events.dispatches)
        return result
    except BaseException as error:
        result["error"] = type(error).__name__ + ": " + str(error)
        raise
    finally:
        cleanup = controller.close() if controller is not None else {}
        if not cleanup.get("cleanup_ok", True):
            result["accepted"] = False
            result["cleanup_error"] = cleanup
        save(output / "result.json", result)
        require(cleanup.get("cleanup_ok", True), "owned process cleanup failed")


def run_plan(plan, output):
    require(plan.get("schema") == "FerricWaveTargetV17NativePlanV1", "wrong plan schema")
    require(type(plan.get("measured_requests")) is int and 1 <= plan["measured_requests"] <= 3,
            "one to three measured requests required")
    require(type(plan.get("warmup_requests")) is int and plan["warmup_requests"] == 1,
            "exactly one warmup required")
    require(plan.get("modes") == list(MODES), "fixed four-arm roster/order required")
    options = arguments(plan["common_args"])
    request, reference = workload_reference(plan)
    timeouts = plan["timeouts"]
    require(set(timeouts) == {"setup_seconds", "request_seconds", "arm_seconds"}, "exact timeout fields required")
    for key, maximum in (("setup_seconds", 600), ("request_seconds", 180), ("arm_seconds", 1800)):
        value = timeouts[key]
        require(type(value) in (int, float) and math.isfinite(value) and 0 < value <= maximum, "bounded timeout required")
    output = Path(output)
    require(output.is_absolute(), "absolute new output directory required")
    output.mkdir(mode=0o700)
    save(output / "plan.json", plan)
    inputs = input_identities(plan, options)
    save(output / "inputs.json", inputs)
    model_inputs = model_identities(options)
    save(output / "model-inputs.json", model_inputs)
    report = {"schema": "FerricWaveTargetV17NativeProbeV1", "authority": "none", "accepted": False,
              "http": False, "sustained": False, "performance_qualified": False,
              "metric_clock": "controller monotonic ingress arrival to committed token completion",
              "arrival_policy": "identical sequential completion-driven commands; not a fixed wall-clock arrival trace",
              "warmup_requests_per_arm": 1, "measured_requests_per_arm": plan["measured_requests"], "arms": []}
    try:
        for mode in MODES:
            require(input_identities(plan, options) == inputs, "input identity drift before arm")
            workload_reference(plan)
            argv = [inputs["controller"]["path"], *plan["common_args"], "--wave-target-mode", mode]
            arm = run_arm(argv, output / mode, mode, request, reference, plan["measured_requests"], timeouts,
                          lambda setup, mode=mode: check_setup(setup, mode, options, inputs))
            require(not report["arms"] or arm["model_identity"] == report["arms"][0]["model_identity"],
                    "model identity drift between arms")
            report["arms"].append(arm)
            require(input_identities(plan, options) == inputs, "input identity drift after arm")
        require(model_identities(options) == model_inputs, "model bytes changed during four-arm probe")
        report["accepted"] = True
        return report
    except BaseException as error:
        report["error"] = type(error).__name__ + ": " + str(error)
        raise
    finally:
        save(output / "report.json", report)


def interrupted(number, _frame):
    raise InterruptedError("runner signal " + str(number))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", required=True)
    parser.add_argument("--plan-sha256", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--execute", action="store_true", help="launch only under the integration admission wrapper")
    args = parser.parse_args()
    require(args.execute, "explicit --execute required; this runner does not perform GPU admission")
    plan = bound_json({"path": args.plan, "sha256": args.plan_sha256})
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    run_plan(plan, args.output_dir)


if __name__ == "__main__":
    main()
