#!/usr/bin/env python3
"""Same-host, bounded replica cohort launcher. Completion is not qualification."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import socket
import stat
import struct
import subprocess
import time

import compare_tp_batch as reference

CLOCK = "CLOCK_MONOTONIC_RAW"
TARGET_BYTES = 16_381_470_720
PARTITIONED_GPU_BYTES = 13_891_534_848
PER_RANK_GPU_BYTES = 608_256
RANK_ZERO_GPU_BYTES = 2_489_327_616
FRAME_LIMIT = 4096
LAYOUTS = {"1xTP8": 8, "4xTP2": 2, "8xTP1": 1}
NAMES = [f"replica-request-{i:02}" for i in range(8)]
VALUE_OPTIONS = {"--batch-tokens", "--prefill-chunk", "--context", "--pages",
                 "--cache-ttl", "--max-batches", "--kernel-profile", "--projection", "--attention"}
FLAG_OPTIONS = {"--prune-output-head", "--runtime-cache-admission", "--runtime-operational",
                "--dispatch-sequences", "--queue-rollover"}
require = reference.require
integer = reference.integer
_interrupted = None


def request_abort(signum, _frame):
    global _interrupted
    _interrupted = signum


def check_interrupted():
    require(_interrupted is None, f"launcher interrupted by signal {_interrupted}")


def now_ns():
    return integer(time.clock_gettime_ns(time.CLOCK_MONOTONIC_RAW), label="raw clock")


def domain():
    namespace = os.stat("/proc/self/ns/time")
    return {"clock": CLOCK, "hostname": Path("/proc/sys/kernel/hostname").read_text().rstrip("\n"),
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().rstrip("\n"),
            "time_namespace_dev": namespace.st_dev, "time_namespace_ino": namespace.st_ino}


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode() + b"\n"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def write_new(path, data):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as output:
        output.write(data)


def partitions(devices, layout):
    require(layout in LAYOUTS and type(devices) is list and len(devices) == 8, "exact eight-card layout required")
    devices = [integer(value, 1, label="GPU identity") for value in devices]
    require(len(set(devices)) == 8, "duplicate GPU identity")
    world = LAYOUTS[layout]
    return [devices[i:i + world] for i in range(0, 8, world)]


def fixed_workload(reference_bytes):
    reference.load_reference(reference_bytes)
    return workload_spec()


def workload_spec():
    return {"schema": "FerricReplicaFixedWorkloadV1", "model_bundle_id": reference.BUNDLE,
            "reference_sha256": reference.REFERENCE_SHA256, "dtype": "BF16", "greedy": True,
            "prefix_cache": False, "arrival_offset_ns": 0, "cancellation": False,
            "requests": [{"name": name, "prompt": reference.PROMPT, "prompt_tokens": reference.PROMPT_IDS,
                          "output_tokens": reference.REFERENCE_IDS[:8],
                          "output_utf8_bytes": list("".join(reference.REFERENCE_PIECES[:8]).encode())}
                         for name in NAMES], "total_output_tokens": 64}


def controller_options(values):
    require(type(values) is list and len(values) <= 32 and all(type(v) is str for v in values),
            "bounded controller options array required")
    seen, i = set(), 0
    while i < len(values):
        flag = values[i]
        require(flag not in seen, "duplicate controller option")
        seen.add(flag)
        i += 1
        if flag in FLAG_OPTIONS:
            continue
        require(flag in VALUE_OPTIONS and i < len(values), "unsupported or controlled controller option")
        require(0 < len(values[i]) <= 128 and not values[i].startswith("--"), "missing option value")
        i += 1
    return values


def plan(devices, layout, reference_bytes, options):
    groups = partitions(devices, layout)
    workload = fixed_workload(reference_bytes)
    options = controller_options(options)
    replicas = []
    for replica, group in enumerate(groups):
        names = NAMES[replica::len(groups)]
        requests = {"schema": "FerricQwen3TpWorkloadV2", "requests": [
            {"name": name, "prompt": reference.PROMPT, "new_tokens": 8, "arrival_tick": 0} for name in names]}
        replicas.append({"replica_id": f"replica-{replica:02}", "device_unique_ids": group,
                         "request_names": names, "requests": requests, "requests_sha256": sha(encoded(requests))})
    return {"schema": "FerricReplicaPlanV1", "authority": "none", "layout": layout,
            "device_unique_ids": devices, "tensor_parallel": LAYOUTS[layout], "replicas": replicas,
            "workload": workload, "workload_sha256": sha(encoded(workload)), "controller_options": options,
            "collective": "host-staged-v1", "retained_target_bytes": len(groups) * TARGET_BYTES,
            "per_instance_base_gpu_weight_bytes": PARTITIONED_GPU_BYTES + LAYOUTS[layout] * PER_RANK_GPU_BYTES + RANK_ZERO_GPU_BYTES,
            "aggregate_base_gpu_weight_bytes": len(groups) * (PARTITIONED_GPU_BYTES + LAYOUTS[layout] * PER_RANK_GPU_BYTES + RANK_ZERO_GPU_BYTES),
            "host_headroom_rule": "three times retained target bytes plus explicit reserve",
            "per_instance_row_budget": integer(int(option_value(options, "--batch-tokens", "16")), 1, 32),
            "total_row_budget": len(groups) * integer(int(option_value(options, "--batch-tokens", "16")), 1, 32),
            "model_parity_qualified": False}


def option_value(values, option, default):
    return values[values.index(option) + 1] if option in values else default


def validate_plan(value):
    groups = partitions(value["device_unique_ids"], value["layout"])
    require(encoded(value["workload"]) == encoded(workload_spec())
            and value["workload_sha256"] == sha(encoded(value["workload"])), "fixed workload differs")
    require(type(value["replicas"]) is list and len(value["replicas"]) == len(groups), "missing replica")
    names = []
    for index, replica in enumerate(value["replicas"]):
        expected_names = NAMES[index::len(groups)]
        expected = {"schema": "FerricQwen3TpWorkloadV2", "requests": [
            {"name": name, "prompt": reference.PROMPT, "new_tokens": 8, "arrival_tick": 0}
            for name in expected_names]}
        require(replica["replica_id"] == f"replica-{index:02}" and replica["device_unique_ids"] == groups[index]
                and replica["request_names"] == expected_names and encoded(replica["requests"]) == encoded(expected)
                and replica["requests_sha256"] == sha(encoded(expected)), "replica assignment differs")
        names.extend(replica["request_names"])
    require(sorted(names) == NAMES and len(set(names)) == 8, "global requests missing or duplicated")
    controller_options(value["controller_options"])
    rows = integer(int(option_value(value["controller_options"], "--batch-tokens", "16")), 1, 32)
    require(value["retained_target_bytes"] == len(groups) * TARGET_BYTES
            and value["per_instance_row_budget"] == rows and value["total_row_budget"] == rows * len(groups)
            and value["collective"] == "host-staged-v1", "replica resource contract differs")
    gpu_bytes = PARTITIONED_GPU_BYTES + LAYOUTS[value["layout"]] * PER_RANK_GPU_BYTES + RANK_ZERO_GPU_BYTES
    require(value["per_instance_base_gpu_weight_bytes"] == gpu_bytes
            and value["aggregate_base_gpu_weight_bytes"] == gpu_bytes * len(groups), "GPU weight allocation differs")


def validate_settings(value):
    reference.fields(value, {"ready_timeout_ns", "run_timeout_ns", "start_lead_ns", "max_lateness_ns"}, "settings")
    integer(value["ready_timeout_ns"], 1_000_000, 3_600_000_000_000)
    integer(value["run_timeout_ns"], 1_000_000, 14_400_000_000_000)
    integer(value["start_lead_ns"], 10_000_000, 10_000_000_000)
    integer(value["max_lateness_ns"], 1, 1_000_000_000)


def memory_check(raw, retained, reserve):
    integer(reserve, 32 * 1024**3, label="host reserve")
    fields = {}
    for line in raw.decode("ascii").splitlines():
        key, *words = line.split()
        if key == "MemAvailable:":
            require(key not in fields and len(words) == 2 and words[1] == "kB", "invalid MemAvailable")
            fields[key] = integer(int(words[0]), label="available memory") * 1024
    require("MemAvailable:" in fields, "missing MemAvailable")
    required = 3 * retained + reserve
    require(fields["MemAvailable:"] >= required, "insufficient concurrent model host-memory headroom")
    return {"available_bytes": fields["MemAvailable:"], "required_bytes": required,
            "retained_target_bytes": retained, "reserve_bytes": reserve, "transient_multiplier": 3}


def held_executable(path, expected):
    reference.hash_value(expected, "executable hash")
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 256 * 1024**2
                and before.st_mode & 0o111, "invalid executable")
        digest = hashlib.sha256()
        count = 0
        while block := os.read(fd, 1024 * 1024):
            count += len(block)
            require(count <= 256 * 1024**2, "executable grew")
            digest.update(block)
        after = os.fstat(fd)
        identity = lambda value: (value.st_dev, value.st_ino, value.st_mode, value.st_size,
                                  value.st_mtime_ns, value.st_ctime_ns)
        require(identity(before) == identity(after) and count == before.st_size and digest.hexdigest() == expected,
                "executable identity differs")
        return fd
    except BaseException:
        os.close(fd)
        raise


def freeze_executable(source, output, expected):
    source_fd = held_executable(source, expected)
    try:
        os.lseek(source_fd, 0, os.SEEK_SET)
        destination = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
        with os.fdopen(destination, "wb") as frozen:
            size = 0
            while block := os.read(source_fd, 1024 * 1024):
                size += len(block)
                require(size <= 256 * 1024**2, "executable copy exceeds bound")
                frozen.write(block)
            frozen.flush()
            os.fchmod(frozen.fileno(), 0o500)
    finally:
        os.close(source_fd)
    return held_executable(output, expected)


def child_status(child):
    # WNOWAIT reserves the controller PID until every owned group is cleaned up.
    if child.returncode is not None:
        return child.returncode
    status = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    if status is None:
        return None
    return status.si_status if status.si_code == os.CLD_EXITED else -status.si_status


def process_identity(pid):
    with Path(f"/proc/{pid}/stat").open("rb") as source:
        data = source.read(16_385)
    require(0 < len(data) <= 16_384, "process stat byte bound exceeded")
    raw = data.decode("utf-8", errors="surrogateescape")
    fields = raw.rsplit(") ", 1)[1].split()
    return {"pid": pid, "state": fields[0], "pgid": int(fields[2]),
            "session": int(fields[3]), "start_ticks": int(fields[19])}


def live_owned_groups(children):
    groups = {child.pid for child in children if child.returncode is None}
    live = []
    with os.scandir("/proc") as entries:
        for count, entry in enumerate(entries):
            require(count < 200_000, "process inventory exceeds cleanup bound")
            if not entry.name.isdecimal():
                continue
            try:
                item = process_identity(int(entry.name))
            except (FileNotFoundError, ProcessLookupError):
                continue
            if item["pgid"] in groups:
                require(item["session"] == item["pgid"], "owned process group session differs")
                if item["state"] not in ("Z", "X"):
                    live.append(item)
    return live


def quiet_groups(children, seconds):
    deadline = time.monotonic() + seconds
    while True:
        live = live_owned_groups(children)
        if not live:
            return {"schema": "FerricReplicaGroupTerminationV1", "observed_ns": now_ns(),
                    "controller_pids": [child.pid for child in children], "live_group_members": [],
                    "scope": "owned process groups; descendants must not detach", "confirmed": True}
        require(time.monotonic() < deadline, f"owned group members still live: {live}")
        time.sleep(0.01)


def abort_and_reap(children):
    active = [child for child in children if child.returncode is None]
    for child in active:
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    until = time.monotonic() + 2
    try:
        while time.monotonic() < until and live_owned_groups(active):
            time.sleep(0.01)
    finally:
        for child in active:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    receipt = quiet_groups(active, 5)
    return [child.wait(timeout=5) for child in children], receipt


def snapshot(command, output, phase, devices):
    require(type(command) is list and 1 <= len(command) <= 16
            and all(type(v) is str and 0 < len(v) <= 4096 for v in command), "snapshot argv required")
    path = output / f"gpu-{phase}.json"
    stderr = output / f"gpu-{phase}.stderr"
    with path.open("xb") as out, stderr.open("xb") as err:
        child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
        deadline = time.monotonic() + 30
        try:
            while child_status(child) is None:
                check_interrupted()
                require(time.monotonic() < deadline and out.tell() <= 1_048_576 and err.tell() <= 1_048_576,
                        "snapshot exceeded bound")
                time.sleep(0.01)
            code = child_status(child)
            require(code == 0, "snapshot process failed")
            quiet_groups([child], 1)
            child.wait(timeout=5)
        except BaseException:
            abort_and_reap([child])
            raise
    require(reference.gpu_roster(reference.read_bounded(path, 1_048_576)) == devices,
            "global idle snapshot roster differs")


def frame(value):
    data = encoded(value)
    require(len(data) <= FRAME_LIMIT, "oversized control frame")
    return data


def peer_pid(stream):
    pid, uid, _ = struct.unpack("3i", stream.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    require(uid == os.getuid(), "control peer UID differs")
    return pid


class Cohort:
    def __init__(self, cohort_plan, output, settings):
        self.plan, self.output, self.settings = cohort_plan, output, settings
        self.socket_path = output / "control.sock"
        require(len(os.fsencode(self.socket_path)) <= 100, "Unix socket path exceeds bound")
        self.domain, self.nonce = domain(), os.urandom(32).hex()
        self.children, self.states, self.streams = [], {}, {}
        self.selector = selectors.DefaultSelector()
        self.epoch, self.first_spawn, self.all_ready = None, None, None
        self.events = (output / "control-events.jsonl").open("xb")
        self.listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.listener.bind(str(self.socket_path))
        os.chmod(self.socket_path, 0o600)
        self.listener.listen(8)
        self.listener.setblocking(False)
        self.selector.register(self.listener, selectors.EVENT_READ, None)

    def event(self, replica, direction, value, observed=None):
        self.events.write(encoded({"observed_ns": now_ns() if observed is None else observed,
                                   "replica_id": replica, "direction": direction, "frame": value}))
        self.events.flush()

    def identity(self, replica):
        return {"nonce": self.nonce, "replica_id": replica["replica_id"],
                "workload_sha256": self.plan["workload_sha256"],
                "requests_sha256": replica["requests_sha256"],
                "device_unique_ids": replica["device_unique_ids"], "clock_domain": self.domain}

    def launch(self, controller, controller_fd, worker, source, artifact):
        for replica in self.plan["replicas"]:
            check_interrupted()
            directory = self.output / replica["replica_id"]
            directory.mkdir(mode=0o700)
            requests = directory / "requests.json"
            write_new(requests, encoded(replica["requests"]))
            config = {"schema": "FerricReplicaControlConfigV1", "socket_path": str(self.socket_path),
                      "launcher_pid": os.getpid(), "identity": self.identity(replica),
                      "io_timeout_ms": self.settings["ready_timeout_ns"] // 1_000_000,
                      "min_start_lead_ns": 1_000_000, "max_start_lead_ns": 10_000_000_000,
                      "max_lateness_ns": self.settings["max_lateness_ns"]}
            config_path = directory / "control.json"
            write_new(config_path, encoded(config))
            argv = [str(controller), "--source", str(source), "--artifact", str(artifact),
                    "--worker", str(worker), "--devices", ",".join(map(str, replica["device_unique_ids"])),
                    "--requests", str(requests), "--benchmark-control", str(config_path),
                    "--disable-prefix-cache", "--allow-unauthenticated-machine-code",
                    "--collective", "host-staged-v1"] + self.plan["controller_options"]
            spawned = now_ns()
            if self.first_spawn is None:
                self.first_spawn = spawned
            with (directory / "stdout.jsonl").open("xb") as out, (directory / "stderr.log").open("xb") as err:
                child = subprocess.Popen(argv, executable=f"/proc/self/fd/{controller_fd}",
                                         pass_fds=(controller_fd,), stdout=out, stderr=err, start_new_session=True)
            self.children.append(child)
            self.states[child.pid] = {"replica": replica, "identity": config["identity"], "pid": child.pid,
                                      "spawned_ns": spawned, "argv": argv, "ready": None,
                                      "started": None, "closed": None, "eof_ns": None, "stream": None}
            process = process_identity(child.pid)
            require(process["pgid"] == child.pid and process["session"] == child.pid, "controller session differs")
            self.states[child.pid]["process_identity"] = process
            write_new(directory / "launch.json", encoded({"pid": child.pid, "spawned_ns": spawned,
                                                           "process_identity": process, "argv": argv}))
            check_interrupted()

    def send(self, state, schema, extra):
        value = {"schema": schema, "authority": "none", "identity": state["identity"], **extra}
        before = now_ns()
        state["stream"].settimeout(1)
        try:
            state["stream"].sendall(frame(value))
        finally:
            state["stream"].setblocking(False)
        self.event(state["replica"]["replica_id"], "send", value, before)
        return before

    def accept(self):
        stream, _ = self.listener.accept()
        try:
            pid = peer_pid(stream)
            require(pid in self.states and self.states[pid]["stream"] is None, "unknown or duplicate replica peer")
            stream.setblocking(False)
            self.streams[stream] = bytearray()
            self.states[pid]["stream"] = stream
            self.selector.register(stream, selectors.EVENT_READ, pid)
        except BaseException:
            stream.close()
            raise

    def receive(self, stream, pid):
        chunk = stream.recv(FRAME_LIMIT + 1)
        if not chunk:
            require(self.states[pid]["closed"] is not None and not self.streams[stream], "early replica EOF")
            self.states[pid]["eof_ns"] = now_ns()
            self.selector.unregister(stream)
            return
        pending = self.streams[stream]
        wire = self.output / self.states[pid]["replica"]["replica_id"] / "control-received.bin"
        require((wire.stat().st_size if wire.exists() else 0) + len(chunk) <= 4 * FRAME_LIMIT,
                "total control byte bound exceeded")
        with wire.open("ab") as raw:
            raw.write(chunk)
        pending.extend(chunk)
        while b"\n" in pending:
            end = pending.index(10) + 1
            require(end <= FRAME_LIMIT, "oversized control message")
            message = reference.json_value(bytes(pending[:end]))
            del pending[:end]
            self.handle(self.states[pid], message)
        require(len(pending) < FRAME_LIMIT, "unterminated oversized control message")

    def handle(self, state, message):
        observed = now_ns()
        require(type(message) is dict and encoded(message.get("identity")) == encoded(state["identity"])
                and message.get("authority") == "none" and message.get("pid") == state["pid"],
                "replica frame identity differs")
        integer(message["pid"], 1, label="replica PID")
        base = {"schema", "authority", "identity", "pid"}
        kind = message["schema"]
        if kind == "FerricReplicaReadyV1":
            reference.fields(message, base | {"ready_ns"}, "Ready")
            require(self.epoch is None and state["ready"] is None, "duplicate or late Ready")
            integer(message["ready_ns"], state["spawned_ns"], observed, "Ready timestamp")
            state["ready"] = message
        elif kind == "FerricReplicaStartedV1":
            reference.fields(message, base | {"ready_ns", "start_received_ns", "epoch_ns", "started_ns", "lateness_ns"}, "Started")
            integer(message["ready_ns"], label="echoed Ready timestamp")
            integer(message["epoch_ns"], label="echoed start epoch")
            require(self.epoch is not None and state["ready"] is not None and state["started"] is None
                    and message["epoch_ns"] == self.epoch and message["ready_ns"] == state["ready"]["ready_ns"],
                    "unexpected Started or epoch")
            integer(message["start_received_ns"], state["start_sent_ns"], self.epoch - 1, "Start receipt")
            integer(message["started_ns"], self.epoch, observed, "actual release")
            integer(message["lateness_ns"], 0, self.settings["max_lateness_ns"], "release lateness")
            require(message["lateness_ns"] == message["started_ns"] - self.epoch, "release lateness differs")
            state["started"] = message
        elif kind == "FerricReplicaClosedV1":
            reference.fields(message, base | {"epoch_ns", "closed_ns"}, "Closed")
            integer(message["epoch_ns"], label="echoed close epoch")
            require(state["started"] is not None and state["closed"] is None and message["epoch_ns"] == self.epoch,
                    "unexpected Closed or epoch")
            integer(message["closed_ns"], state["started"]["started_ns"], observed, "close timestamp")
            state["closed"] = message
        else:
            raise ValueError("unexpected replica control schema")
        self.event(state["replica"]["replica_id"], "receive", message, observed)
        if kind == "FerricReplicaClosedV1":
            self.send(state, "FerricReplicaCloseAckV1", {"epoch_ns": self.epoch})

    def supervise(self):
        require(len(self.children) == len(self.plan["replicas"]), "incomplete cohort launch")
        while True:
            check_interrupted()
            for key, _ in self.selector.select(0.01):
                if key.fileobj is self.listener:
                    self.accept()
                else:
                    self.receive(key.fileobj, key.data)
            for child in self.children:
                code = child_status(child)
                require(code is None or (code == 0 and self.states[child.pid]["closed"] is not None),
                        "replica exited before clean close")
                directory = self.output / self.states[child.pid]["replica"]["replica_id"]
                require((directory / "stdout.jsonl").stat().st_size <= 64 * 1024**2
                        and (directory / "stderr.log").stat().st_size <= 16 * 1024**2, "replica log bound exceeded")
            if self.epoch is None:
                require(now_ns() - self.first_spawn < self.settings["ready_timeout_ns"], "Ready deadline exceeded")
                if all(state["ready"] is not None for state in self.states.values()):
                    require(domain() == self.domain, "launcher clock domain changed")
                    self.all_ready = now_ns()
                    self.epoch = self.all_ready + self.settings["start_lead_ns"]
                    for state in self.states.values():
                        state["start_sent_ns"] = self.send(state, "FerricReplicaStartV1", {"epoch_ns": self.epoch})
            else:
                require(now_ns() <= self.epoch + self.settings["run_timeout_ns"], "cohort run deadline exceeded")
                if all(state["closed"] is not None and state["eof_ns"] is not None for state in self.states.values()) and all(
                        child_status(child) == 0 for child in self.children):
                    require(domain() == self.domain, "launcher close clock domain changed")
                    return

    def record(self):
        return {"clock_domain": self.domain, "nonce": self.nonce, "settings": self.settings,
                "first_spawn_ns": self.first_spawn, "all_ready_ns": self.all_ready, "epoch_ns": self.epoch,
                "replicas": [{key: value for key, value in state.items() if key != "stream"}
                             for state in self.states.values()]}

    def close(self):
        for stream in self.streams:
            stream.close()
        self.listener.close()
        self.selector.close()
        self.events.close()
        self.socket_path.unlink(missing_ok=True)


def run(cohort_plan, output, controller, controller_sha, worker, worker_sha, source, artifact,
        snapshot_command, settings, reserve, snapshot_fn=snapshot, memory_raw=None):
    global _interrupted
    metadata = output.lstat()
    require(output.is_absolute() and stat.S_ISDIR(metadata.st_mode) and metadata.st_mode & 0o777 == 0o700
            and metadata.st_uid == os.getuid(), "owned private absolute cohort output required")
    controller_fd, worker_fd, cohort = None, None, None
    result = {"schema": "FerricReplicaCohortV1", "authority": "none", "status": "failed",
              "model_parity_qualified": False, "controller_sha256": controller_sha,
              "worker_sha256": worker_sha, "snapshot_command": snapshot_command,
              "workload_sha256": cohort_plan["workload_sha256"], "error": None,
              "clock_domain": domain(), "gpu_snapshot_intervals": []}

    def take_snapshot(phase):
        receipt = {"phase": phase, "start_ns": now_ns(), "end_ns": None, "completed": False}
        result["gpu_snapshot_intervals"].append(receipt)
        try:
            require(domain() == result["clock_domain"], "snapshot clock domain changed")
            snapshot_fn(snapshot_command, output, phase, cohort_plan["device_unique_ids"])
            require(domain() == result["clock_domain"], "snapshot clock domain changed")
            receipt["completed"] = True
        finally:
            receipt["end_ns"] = now_ns()

    pre_taken = False
    _interrupted = None
    previous_signals = {sig: signal.signal(sig, request_abort) for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        validate_plan(cohort_plan)
        validate_settings(settings)
        require(source.is_absolute() and artifact.is_absolute(), "absolute source/artifact paths required")
        raw = Path("/proc/meminfo").read_bytes() if memory_raw is None else memory_raw
        write_new(output / "meminfo-before.txt", raw)
        write_new(output / "plan.json", encoded(cohort_plan))
        memory = memory_check(raw, cohort_plan["retained_target_bytes"], reserve)
        write_new(output / "memory-headroom.json", encoded(memory))
        executable_directory = output / "launch-artifacts"
        executable_directory.mkdir(mode=0o700)
        frozen_controller, frozen_worker = executable_directory / "controller", executable_directory / "worker"
        controller_fd = freeze_executable(controller, frozen_controller, controller_sha)
        worker_fd = freeze_executable(worker, frozen_worker, worker_sha)
        result["executables"] = {"controller": str(frozen_controller), "worker": str(frozen_worker),
                                 "source_controller": str(controller), "source_worker": str(worker)}
        pre_taken = True
        take_snapshot("before")
        check_interrupted()
        cohort = Cohort(cohort_plan, output, settings)
        cohort.launch(frozen_controller, controller_fd, frozen_worker, source, artifact)
        cohort.supervise()
        result["group_termination"] = quiet_groups(cohort.children, 1)
        check_interrupted()
        result["exit_codes"] = [child.wait(timeout=5) for child in cohort.children]
        result["final_reap_ns"] = now_ns()
        # Already reaped children must not enter process-group abort handling.
        cohort.children.clear()
        pre_taken = False
        take_snapshot("after")
        check_interrupted()
        result["status"] = "unvalidated-complete"
    except BaseException as failure:
        for sig in previous_signals:
            signal.signal(sig, signal.SIG_IGN)
        _interrupted = None
        result["error"] = str(failure)
        if cohort is not None:
            try:
                codes, receipt = abort_and_reap(cohort.children)
                if cohort.children:
                    result["exit_codes"], result["group_termination"] = codes, receipt
                    result["final_reap_ns"] = now_ns()
                    cohort.children.clear()
            except Exception as cleanup_error:
                result["teardown_uncertain"] = str(cleanup_error)
        if pre_taken:
            try:
                take_snapshot("after")
            except Exception as snapshot_error:
                result["post_snapshot_error"] = str(snapshot_error)
    finally:
        if cohort is not None:
            result["control"] = cohort.record()
            cohort.close()
        for fd in (controller_fd, worker_fd):
            if fd is not None:
                os.close(fd)
        try:
            write_new(output / "cohort.json", encoded(result))
        finally:
            for sig, handler in previous_signals.items():
                signal.signal(sig, handler)
            _interrupted = None
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--plan", action="store_true")
    mode.add_argument("--run", action="store_true")
    parser.add_argument("--layout", choices=tuple(LAYOUTS), required=True)
    parser.add_argument("--devices", required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--controller-options", type=Path)
    for key in ("controller", "worker", "source", "artifact", "snapshot-command"):
        parser.add_argument("--" + key, type=Path)
    parser.add_argument("--controller-sha256")
    parser.add_argument("--worker-sha256")
    parser.add_argument("--ready-timeout-seconds", type=int, default=900)
    parser.add_argument("--run-timeout-seconds", type=int, default=1800)
    parser.add_argument("--start-lead-ms", type=int, default=250)
    parser.add_argument("--max-lateness-ms", type=int, default=100)
    parser.add_argument("--host-memory-reserve-bytes", type=int, default=32 * 1024**3)
    args = parser.parse_args()
    require(args.output.is_absolute(), "absolute output required")
    options = [] if args.controller_options is None else reference.json_value(reference.read_bounded(args.controller_options, 16_384))
    cohort_plan = plan([int(value) for value in args.devices.split(",")], args.layout,
                       reference.read_bounded(args.reference, 1_048_576), options)
    args.output.mkdir(mode=0o700)
    if args.plan:
        write_new(args.output / "plan.json", encoded(cohort_plan))
        return 0
    require(all(getattr(args, key) is not None for key in ("controller", "worker", "source", "artifact",
            "snapshot_command", "controller_sha256", "worker_sha256")), "explicit run identities required")
    settings = {"ready_timeout_ns": integer(args.ready_timeout_seconds, 1, 3600) * 1_000_000_000,
                "run_timeout_ns": integer(args.run_timeout_seconds, 1, 14_400) * 1_000_000_000,
                "start_lead_ns": integer(args.start_lead_ms, 10, 10_000) * 1_000_000,
                "max_lateness_ns": integer(args.max_lateness_ms, 1, 1000) * 1_000_000}
    command = reference.json_value(reference.read_bounded(args.snapshot_command, 16_384))
    result = run(cohort_plan, args.output, args.controller, args.controller_sha256,
                 args.worker, args.worker_sha256, args.source, args.artifact, command,
                 settings, args.host_memory_reserve_bytes)
    return 0 if result["status"] == "unvalidated-complete" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        raise SystemExit(f"replica benchmark failed: {error}") from error
