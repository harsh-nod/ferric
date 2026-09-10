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
FRAME_LIMIT = 4096
LAYOUTS = {"1xTP8": 8, "4xTP2": 2, "8xTP1": 1}
NAMES = [f"replica-request-{i:02}" for i in range(8)]
VALUE_OPTIONS = {"--batch-tokens", "--prefill-chunk", "--context", "--pages",
                 "--cache-ttl", "--max-batches", "--kernel-profile", "--projection", "--attention"}
FLAG_OPTIONS = {"--prune-output-head", "--runtime-cache-admission", "--runtime-operational",
                "--dispatch-sequences", "--queue-rollover"}
require = reference.require
integer = reference.integer


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
            "host_headroom_rule": "three times retained target bytes plus explicit reserve",
            "per_instance_row_budget": option_value(options, "--batch-tokens", "16"),
            "model_parity_qualified": False}


def option_value(values, option, default):
    return values[values.index(option) + 1] if option in values else default


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


def child_status(child):
    # WNOWAIT reserves the controller PID until every owned group is cleaned up.
    if child.returncode is not None:
        return child.returncode
    status = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    if status is None:
        return None
    return status.si_status if status.si_code == os.CLD_EXITED else -status.si_status


def abort_and_reap(children):
    active = [child for child in children if child.returncode is None]
    for child in active:
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    until = time.monotonic() + 2
    while time.monotonic() < until and any(child_status(child) is None for child in active):
        time.sleep(0.01)
    for child in active:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    return [child.wait(timeout=5) for child in children]


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
                require(time.monotonic() < deadline and out.tell() <= 1_048_576 and err.tell() <= 1_048_576,
                        "snapshot exceeded bound")
                time.sleep(0.01)
            code = child_status(child)
            require(code == 0, "snapshot process failed")
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
        self.domain, self.nonce = domain(), os.urandom(32).hex()
        self.children, self.states, self.streams = [], {}, {}
        self.selector = selectors.DefaultSelector()
        self.epoch, self.first_spawn, self.all_ready = None, None, None
        self.events = (output / "control-events.jsonl").open("xb")
        self.listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket_path = output / "control.sock"
        require(len(os.fsencode(self.socket_path)) <= 100, "Unix socket path exceeds bound")
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
                                      "started": None, "closed": None, "stream": None}
            write_new(directory / "launch.json", encoded({"pid": child.pid, "spawned_ns": spawned, "argv": argv}))

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
            self.selector.unregister(stream)
            return
        pending = self.streams[stream]
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
                if all(state["closed"] is not None for state in self.states.values()) and all(
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
    require(output.is_absolute() and output.stat().st_mode & 0o777 == 0o700, "private absolute cohort output required")
    raw = Path("/proc/meminfo").read_bytes() if memory_raw is None else memory_raw
    write_new(output / "meminfo-before.txt", raw)
    memory = memory_check(raw, cohort_plan["retained_target_bytes"], reserve)
    write_new(output / "memory-headroom.json", encoded(memory))
    write_new(output / "plan.json", encoded(cohort_plan))
    controller_fd, worker_fd, cohort = None, None, None
    result = {"schema": "FerricReplicaCohortV1", "authority": "none", "status": "failed",
              "model_parity_qualified": False, "controller_sha256": controller_sha,
              "worker_sha256": worker_sha, "snapshot_command": snapshot_command,
              "workload_sha256": cohort_plan["workload_sha256"], "error": None}
    pre_taken = False
    try:
        controller_fd = held_executable(controller, controller_sha)
        worker_fd = held_executable(worker, worker_sha)
        pre_taken = True
        snapshot_fn(snapshot_command, output, "before", cohort_plan["device_unique_ids"])
        cohort = Cohort(cohort_plan, output, settings)
        cohort.launch(controller, controller_fd, worker, source, artifact)
        cohort.supervise()
        result["exit_codes"] = [child.wait(timeout=5) for child in cohort.children]
        result["final_reap_ns"] = now_ns()
        # Already reaped children must not enter process-group abort handling.
        cohort.children.clear()
        pre_taken = False
        snapshot_fn(snapshot_command, output, "after", cohort_plan["device_unique_ids"])
        result["status"] = "unvalidated-complete"
    except BaseException as failure:
        result["error"] = str(failure)
        if cohort is not None:
            result["exit_codes"] = abort_and_reap(cohort.children)
            result["final_reap_ns"] = now_ns()
            cohort.children.clear()
        if pre_taken:
            try:
                snapshot_fn(snapshot_command, output, "after", cohort_plan["device_unique_ids"])
            except Exception as snapshot_error:
                result["post_snapshot_error"] = str(snapshot_error)
    finally:
        if cohort is not None:
            result["control"] = cohort.record()
            cohort.close()
        for fd in (controller_fd, worker_fd):
            if fd is not None:
                os.close(fd)
        write_new(output / "cohort.json", encoded(result))
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
