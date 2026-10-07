#!/usr/bin/env python3
"""Read-only, fail-closed GPU descriptor attribution across mount namespaces.

An empty descriptor scan is not proof of device idleness. Callers must retain
their utilization, resource and scheduler checks. No discovered PID is signaled.
"""
import argparse
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import time


MAX_PIDS = 32768
MAX_FDS = 131072
MAX_TOTAL_FDS = 1000000
MAX_SECONDS = 15


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_text(path, limit=65536):
    with path.open("rb") as source:
        raw = source.read(limit + 1)
    require(len(raw) <= limit, "proc evidence exceeds byte bound")
    return raw.decode("utf-8", errors="strict")


def identity(proc, pid):
    directory = proc / str(pid)
    raw = read_text(directory / "stat")
    prefix, sep, tail = raw.rpartition(") ")
    fields = tail.split()
    require(sep and prefix.startswith(str(pid) + " (") and len(fields) >= 20
            and fields[19].isdigit(), "invalid PID start identity")
    status = read_text(directory / "status")
    uid = re.findall(r"^Uid:\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s*$", status, re.M)
    require(len(uid) == 1, "exact UID evidence required")
    cgroup = read_text(directory / "cgroup")
    require(cgroup and all(re.fullmatch(r"\d+:[^\n:]*:/[^\n]*", line) for line in cgroup.splitlines()),
            "invalid cgroup evidence")
    namespace = (directory / "ns/pid").stat()
    return {"pid": pid, "start_time_ticks": int(fields[19]), "uids": [int(v) for v in uid[0]],
            "proc_uid": directory.stat().st_uid,
            "pid_namespace": [namespace.st_dev, namespace.st_ino], "cgroup": cgroup}


def pids(proc):
    result = set()
    with os.scandir(proc) as entries:
        for entry in entries:
            if entry.name.isdigit():
                require(len(result) < MAX_PIDS, "PID census exceeds bound")
                result.add(int(entry.name))
    return result


def device_identities(paths):
    result = {}
    for path in paths:
        item = Path(path).stat()
        require(stat.S_ISCHR(item.st_mode), "character GPU device required")
        pair = (os.major(item.st_rdev), os.minor(item.st_rdev))
        result.setdefault(pair, []).append(str(path))
    require(result, "at least one GPU character device required")
    return result


def scan(proc, devices, allowed, phase, *, clock=time.monotonic, identity_reader=identity,
         pid_reader=pids, descriptor_stat=os.stat):
    require(phase in ("preflight", "active", "postflight"), "invalid lifecycle phase")
    started = clock()
    row = {"schema": "FerricDeviceDescriptorSampleV1", "method": "proc-fd-rdev-all-pids",
           "phase": phase, "started_monotonic_ns": time.monotonic_ns(),
           "complete": False, "accepted": False, "errors": [], "device_users": [],
           "owned_users": [], "foreign_users": [], "descriptors_scanned": 0,
           "devices": [{"major": key[0], "minor": key[1], "paths": value} for key, value in sorted(devices.items())],
           "caveat": "descriptor sample only; not continuous isolation or proof of idle GPU"}
    try:
        before = pid_reader(proc)
        row["pids_scanned"] = len(before)
        for pid in sorted(before):
            require(clock() - started < MAX_SECONDS, "descriptor scan deadline exceeded")
            initial = identity_reader(proc, pid)
            hits = []
            count = 0
            with os.scandir(proc / str(pid) / "fd") as entries:
                for entry in entries:
                    require(entry.name.isdigit(), "non-numeric proc descriptor")
                    count += 1
                    row["descriptors_scanned"] += 1
                    require(count <= MAX_FDS and row["descriptors_scanned"] <= MAX_TOTAL_FDS,
                            "descriptor scan count exceeds bound")
                    require(clock() - started < MAX_SECONDS, "descriptor scan deadline exceeded")
                    # stat follows the proc FD link into its owning mount namespace.
                    item = descriptor_stat(entry.path)
                    if stat.S_ISCHR(item.st_mode):
                        pair = (os.major(item.st_rdev), os.minor(item.st_rdev))
                        if pair in devices:
                            hits.append({"fd": int(entry.name), "major": pair[0], "minor": pair[1]})
            require(identity_reader(proc, pid) == initial, "PID identity changed during scan")
            if hits:
                user = {"identity": initial, "descriptors": sorted(hits, key=lambda v: v["fd"])}
                row["device_users"].append(user)
                if allowed.get(pid) == initial:
                    row["owned_users"].append(user)
                else:
                    row["foreign_users"].append(user)
        require(pid_reader(proc) == before, "PID census changed during scan; retain and retry as a new sample")
        row["complete"] = True
        require(not row["foreign_users"], "foreign or unbound GPU descriptor owner")
        if phase == "active":
            require(row["owned_users"], "no positive owned GPU descriptor attribution")
        else:
            require(not row["device_users"], "GPU descriptors remain at lifecycle endpoint")
        row["accepted"] = True
    except (OSError, ValueError, UnicodeError) as error:
        row["errors"].append(type(error).__name__ + ": " + str(error)[:4096])
    row["completed_monotonic_ns"] = time.monotonic_ns()
    return row


def docker_json(argv, runner=subprocess.run):
    # The only Docker operations in this tool are inspect and top.
    require(argv[:2] in (["docker", "inspect"], ["docker", "top"]), "read-only Docker command required")
    result = runner(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    timeout=5, check=False)
    require(result.returncode == 0 and len(result.stdout) <= 1024**2 and not result.stderr,
            "bounded Docker identity command failed")
    return result.stdout


def container_members(binding, proc, *, runner=subprocess.run, identity_reader=identity):
    require(type(binding) is dict and set(binding) == {"id", "name", "image", "label_key", "label_value"},
            "exact container binding required")
    require(re.fullmatch(r"[0-9a-f]{64}", binding["id"])
            and re.fullmatch(r"sha256:[0-9a-f]{64}", binding["image"]), "full immutable container/image IDs required")
    require(type(binding["name"]) is str and re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}", binding["name"]),
            "exact bounded container name required")
    require(type(binding["label_key"]) is str and binding["label_key"]
            and type(binding["label_value"]) is str and binding["label_value"], "ownership label required")

    def inspect():
        values = json.loads(docker_json(["docker", "inspect", binding["id"]], runner))
        require(type(values) is list and len(values) == 1, "one exact container required")
        value = values[0]
        state = value.get("State", {})
        require(value.get("Id") == binding["id"] and value.get("Name") == "/" + binding["name"]
                and value.get("Image") == binding["image"]
                and value.get("Config", {}).get("Labels", {}).get(binding["label_key"]) == binding["label_value"]
                and state.get("Running") is True and state.get("Paused") is False
                and state.get("OOMKilled") is False and type(state.get("Pid")) is int and state["Pid"] > 1,
                "owned running container identity changed")
        return {"id": value["Id"], "init_pid": state["Pid"], "started_at": state.get("StartedAt")}

    first = inspect()
    raw = docker_json(["docker", "top", binding["id"], "-eo", "pid"], runner)
    lines = raw.splitlines()
    require(lines and lines[0].strip() == b"PID" and 1 < len(lines) <= MAX_PIDS + 1,
            "bounded nonempty Docker host PID table required")
    fields = [line.strip() for line in lines[1:]]
    require(all(field.isdigit() and int(field) > 1 for field in fields)
            and len(set(fields)) == len(fields), "invalid container PID membership")
    members = {int(field): identity_reader(proc, int(field)) for field in fields}
    require(first["init_pid"] in members, "container init absent from membership")
    require(inspect() == first, "container restarted during attribution")
    return members, {"binding": binding, "state": first, "host_pids": sorted(members)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--device", action="append", required=True)
    parser.add_argument("--phase", choices=("preflight", "active", "postflight"), required=True)
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--container-binding", type=Path)
    source.add_argument("--owned-identities", type=Path)
    args = parser.parse_args()
    proc, allowed, receipt = Path("/proc"), {}, None
    try:
        devices = device_identities(args.device)
        if args.container_binding:
            binding = json.loads(read_text(args.container_binding))
            allowed, receipt = container_members(binding, proc)
        elif args.owned_identities:
            values = json.loads(read_text(args.owned_identities))
            require(type(values) is list and len(values) <= MAX_PIDS, "bounded exact identity list required")
            allowed = {value["pid"]: value for value in values}
            require(len(allowed) == len(values), "duplicate owned PID identity")
        row = scan(proc, devices, allowed, args.phase)
        if receipt is not None:
            after, after_receipt = container_members(binding, proc)
            require(after == allowed and after_receipt == receipt, "container membership changed during descriptor scan")
            row["container"] = receipt
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        row = {"schema": "FerricDeviceDescriptorSampleV1", "phase": args.phase,
               "method": "proc-fd-rdev-all-pids", "accepted": False, "complete": False,
               "errors": [type(error).__name__ + ": " + str(error)[:4096]]}
    print(json.dumps(row, sort_keys=True, allow_nan=False))
    return 0 if row["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
