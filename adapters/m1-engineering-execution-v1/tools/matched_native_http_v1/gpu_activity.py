#!/usr/bin/env python3
"""Read-only, fail-closed GPU descriptor attribution across mount namespaces.

An empty descriptor scan is not proof of device idleness. Callers must retain
their utilization, resource and scheduler checks. No discovered PID is signaled.
HTTP startup permits empty or exact-owned GPU descriptors; it never admits timing.
"""
import argparse
import copy
import errno
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
MAX_CENSUS_ROUNDS = 3
MAX_PROCESS_ATTEMPTS = 3


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


class ProcessStillPresent(ValueError):
    """Departure proof found a process that still exists; not proof of safety."""


def confirm_departure(proc, pid, *, pidfd_opener=None, directory_stat=os.lstat):
    """Confirm a fully departed PID, not a missing FD or an unreadable live PID."""
    if pidfd_opener is None:
        pidfd_opener = getattr(os, "pidfd_open", None)
    require(pidfd_opener is not None, "pidfd_open is required for departure evidence")
    for _ in range(2):
        try:
            directory_stat(proc / str(pid))
        except FileNotFoundError as error:
            require(error.errno == errno.ENOENT, "exact absent proc directory required")
        else:
            raise ProcessStillPresent("missing proc member belongs to a still-present PID")
        try:
            descriptor = pidfd_opener(pid, 0)
        except ProcessLookupError as error:
            require(error.errno == errno.ESRCH, "exact pidfd ESRCH evidence required")
        else:
            os.close(descriptor)
            raise ProcessStillPresent("PID still exists according to pidfd_open")
    return {"pid": pid, "method": "proc-directory-absent-and-pidfd-esrch",
            "proc_directory_absent_checks": 2, "pidfd_esrch_checks": 2}


def error_evidence(error):
    return {"type": type(error).__name__, "message": str(error)[:4096],
            "errno": getattr(error, "errno", None),
            "filename": str(error.filename)[:4096] if getattr(error, "filename", None) is not None else None,
            "filename2": str(error.filename2)[:4096] if getattr(error, "filename2", None) is not None else None}


def changed_identity_fields(initial, final):
    if not isinstance(initial, dict) or not isinstance(final, dict):
        return ["<identity_type>"]
    return sorted(key for key in set(initial) | set(final) if initial.get(key) != final.get(key))


def refusal_process_observation(proc, pid):
    """Best-effort state evidence after refusal; never used to grant admission."""
    result = {"pid": pid, "scope": "post-refusal observation; not admission evidence"}
    directory = proc / str(pid)
    try:
        info = directory.lstat()
        result["directory"] = {"device": info.st_dev, "inode": info.st_ino,
                               "uid": info.st_uid, "mode": info.st_mode}
    except (OSError, ValueError, UnicodeError) as error:
        result["directory_error"] = error_evidence(error)
    try:
        raw = read_text(directory / "stat", limit=4096)
        prefix, sep, tail = raw.rpartition(") ")
        fields = tail.split()
        require(sep and prefix.startswith(str(pid) + " (") and len(fields) >= 20
                and fields[1].isdigit() and fields[19].isdigit(), "invalid diagnostic proc stat")
        result["stat"] = {"comm": prefix[len(str(pid)) + 2:], "state": fields[0],
                          "parent_pid": int(fields[1]), "start_time_ticks": int(fields[19])}
    except (OSError, ValueError, UnicodeError) as error:
        result["stat_error"] = error_evidence(error)
    return result


def scan(proc, devices, allowed, phase, *, clock=time.monotonic, identity_reader=identity,
         pid_reader=pids, descriptor_stat=os.stat, departure_reader=confirm_departure,
         effective_uid_reader=os.geteuid, native_owned_fd_rescan=False):
    require(phase in ("preflight", "startup", "active", "postflight"), "invalid lifecycle phase")
    require(type(native_owned_fd_rescan) is bool, "explicit native FD rescan selection required")
    require(not native_owned_fd_rescan or (phase == "active" and len(allowed) == 1
            and effective_uid_reader() == 0), "native FD rescan requires one active root-visible owner")
    started = clock()
    row = {"schema": "FerricDeviceDescriptorSampleV2", "method": "proc-fd-rdev-finite-roster-v1",
           "sampling_policy": "initial-plus-one-birth-frontier-v1",
           "coverage_scope": "complete declared roster over interval; not atomic terminal host coverage",
           "root_visibility": effective_uid_reader() == 0,
           "phase": phase, "started_monotonic_ns": time.monotonic_ns(),
           "complete": False, "accepted": False, "errors": [], "device_users": [],
           "diagnostic_only": False, "native_launch_admitted": False, "failure_diagnostics": [],
           "owned_users": [], "foreign_users": [], "descriptors_scanned": 0, "pids_scanned": 0,
           "departed_processes": [], "census_reconciliation": [],
           "departure_policy": "fully-absent-pidfd-esrch-v1",
           "process_retry_policy": "nonowner-complete-rescan-fd-enoent-cgroup-or-recorded-credentials-v2",
           "process_attempt_events": [],
           "devices": [{"major": key[0], "minor": key[1], "paths": value} for key, value in sorted(devices.items())],
           "caveat": "finite-roster descriptor sample only; not continuous isolation or proof of idle GPU; "
                     "late births are unscanned and retained; paired external KFD/fuser endpoints required"}
    observed, inspected, departed, device_pids = set(), {}, set(), set()
    context = {}
    anchors, attempts, process_descriptors = {}, {}, {}
    completed_owned = {}
    if native_owned_fd_rescan:
        row.update(native_owned_fd_rescan=True, completed_owned_users=[],
            process_retry_policy="native-owned-complete-rescan-fd-enoent-v1")

    def deadline():
        require(clock() - started < MAX_SECONDS, "descriptor scan deadline exceeded")

    def census():
        context.clear()
        context.update(operation="pid_census", pid=None)
        deadline()
        require(len(row['census_reconciliation']) < MAX_CENSUS_ROUNDS, 'finite roster census bound exceeded')
        current = pid_reader(proc)
        deadline()
        require(type(current) is set and all(type(pid) is int and pid > 0 for pid in current),
                "exact positive PID census required")
        require(len(observed | current) <= MAX_PIDS, "PID census union exceeds bound")
        context["reappeared_pids"] = sorted(current & departed)
        require(not current & departed, "departed PID reappeared during scan")
        fresh = current - observed
        observed.update(current)
        row["pids_observed"] = len(observed)
        row["census_reconciliation"].append({"ordinal": len(row["census_reconciliation"]),
                                             "pid_count": len(current), "new_pids": sorted(fresh)})
        return current, fresh

    def record_departure(pid, initial, reason, missing_error=None):
        triggering_context = copy.deepcopy(context) if missing_error is not None else None
        context.clear()
        context.update(operation="departure_proof", pid=pid, reason=reason,
                       initial_identity=copy.deepcopy(initial),
                       allowed_identity=copy.deepcopy(allowed.get(pid)))
        if missing_error is not None:
            context["triggering_missing_proc_error"] = error_evidence(missing_error)
            context["triggering_context"] = triggering_context
        deadline()
        require(pid not in allowed, "allowlisted worker disappeared during scan")
        require(pid not in device_pids, "observed GPU descriptor owner disappeared during scan")
        proof = departure_reader(proc, pid)
        require(proof == {"pid": pid, "method": "proc-directory-absent-and-pidfd-esrch",
                          "proc_directory_absent_checks": 2, "pidfd_esrch_checks": 2},
                "exact positively departed PID evidence required")
        deadline()
        departed.add(pid)
        inspected.pop(pid, None)
        row["departed_processes"].append({"identity": initial, "reason": reason, "proof": proof})

    def interrupted(pid, reason, error=None):
        event = {"pid": pid, "attempt": attempts.get(pid, 0), "reason": reason,
                 "context": copy.deepcopy(context), "retry_requested": False,
                 "cumulative_descriptors": process_descriptors.get(pid, 0)}
        if error is not None:
            event["error"] = error_evidence(error)
        row["process_attempt_events"].append(event)
        return event

    def request_retry(pid, event):
        deadline()
        require(pid not in allowed, "allowlisted worker cannot be rescanned after an interruption")
        require(pid not in device_pids, "observed GPU descriptor owner cannot be rescanned after an interruption")
        require(pid in anchors, "complete initial process identity required before retry")
        require(attempts[pid] < MAX_PROCESS_ATTEMPTS, "per-process complete rescan attempt bound exceeded")
        event["retry_requested"] = True

    def restart_for_identity(pid, initial, final, where):
        fields = changed_identity_fields(initial, final)
        if not fields or not set(fields) <= {"cgroup", "uids", "proc_uid"}:
            return False
        credentials = bool(set(fields) & {"uids", "proc_uid"})
        event = interrupted(pid, ("credentials" if credentials else "cgroup") + "-changed-" + where)
        event.update(initial_identity=copy.deepcopy(initial), final_identity=copy.deepcopy(final),
                     changed_identity_fields=fields)
        require(not credentials or row["root_visibility"], 'credential rescan requires root visibility')
        request_retry(pid, event)
        # The lifetime and namespace anchor remains unchanged. Credential changes
        # become a new baseline only through this explicit retained restart.
        anchors[pid] = copy.deepcopy(final)
        return True

    def inspect_pid(pid):
        completed_owned.pop(pid, None)
        while True:
            context.clear()
            context.update(operation="identity_initial", pid=pid,
                           allowed_identity=copy.deepcopy(allowed.get(pid)))
            deadline()
            require(attempts.get(pid, 0) < MAX_PROCESS_ATTEMPTS,
                    "per-process complete rescan attempt bound exceeded")
            attempts[pid] = attempts.get(pid, 0) + 1
            row["pids_scanned"] = len(attempts)
            context["process_attempt"] = attempts[pid]
            initial, user = None, None
            try:
                initial = identity_reader(proc, pid)
                context.update(operation="allowed_identity_validation", initial_identity=copy.deepcopy(initial))
                if pid in allowed:
                    context["allowed_changed_identity_fields"] = changed_identity_fields(allowed[pid], initial)
                require(pid not in allowed or allowed[pid] == initial,
                        "allowlisted PID identity changed before descriptor scan")
                if pid in anchors:
                    context["anchor_identity"] = copy.deepcopy(anchors[pid])
                    context["anchor_changed_fields"] = changed_identity_fields(anchors[pid], initial)
                    require(not context["anchor_changed_fields"],
                            "PID identity changed before complete rescan")
                else:
                    anchors[pid] = copy.deepcopy(initial)
                context.update(operation="fd_directory_open", path=str(proc / str(pid) / "fd"))
                with os.scandir(proc / str(pid) / "fd") as entries:
                    for entry in entries:
                        require(entry.name.isdigit(), "non-numeric proc descriptor")
                        process_descriptors[pid] = process_descriptors.get(pid, 0) + 1
                        row["descriptors_scanned"] += 1
                        require(process_descriptors[pid] <= MAX_FDS
                                and row["descriptors_scanned"] <= MAX_TOTAL_FDS,
                                "descriptor scan count exceeds bound")
                        deadline()
                        context.update(operation="descriptor_stat", path=entry.path)
                        item = descriptor_stat(entry.path)
                        if stat.S_ISCHR(item.st_mode):
                            pair = (os.major(item.st_rdev), os.minor(item.st_rdev))
                            if pair in devices:
                                if user is None:
                                    user = {"identity": initial, "descriptors": []}
                                    row["device_users"].append(user)
                                    device_pids.add(pid)
                                    row["owned_users" if allowed.get(pid) == initial else "foreign_users"].append(user)
                                user["descriptors"].append({"fd": int(entry.name), "major": pair[0], "minor": pair[1]})
                context.update(operation="identity_after_descriptors")
                final = identity_reader(proc, pid)
                fields = changed_identity_fields(initial, final)
                context.update(final_identity=copy.deepcopy(final), changed_identity_fields=fields)
                if restart_for_identity(pid, initial, final, "during-descriptor-scan"):
                    continue
                require(final == initial, "PID identity changed during scan")
                if user is not None:
                    user["descriptors"].sort(key=lambda value: value["fd"])
                inspected[pid] = initial
                if user is not None and allowed.get(pid) == initial:
                    completed_owned[pid] = user
                if attempts[pid] > 1:
                    row["process_attempt_events"].append({"pid": pid, "attempt": attempts[pid],
                        "reason": "complete-stable-rescan", "identity": copy.deepcopy(initial),
                        "cumulative_descriptors": process_descriptors.get(pid, 0)})
                return
            except FileNotFoundError as error:
                event = interrupted(pid, "proc-entry-missing-during-process-attempt", error)
                origin = copy.deepcopy(context)
                exact_fd = (origin.get("operation") == "descriptor_stat"
                    and error.errno == errno.ENOENT and error.filename == origin.get("path")
                    and Path(error.filename).parent == proc / str(pid) / "fd"
                    and Path(error.filename).name.isdigit())
                if native_owned_fd_rescan and pid in allowed and exact_fd:
                    deadline()
                    require(not row["foreign_users"], "foreign GPU hit forbids native FD rescan")
                    require(initial == anchors.get(pid) == allowed[pid],
                            "exact anchored owner required before native FD rescan")
                    context.update(operation="native_fd_retry_identity", path=str(proc / str(pid)))
                    final = identity_reader(proc, pid)
                    context.update(final_identity=copy.deepcopy(final),
                        changed_identity_fields=changed_identity_fields(initial, final))
                    event["retry_identity"] = copy.deepcopy(final)
                    require(final == initial, "owned identity changed after missing FD")
                    deadline()
                    require(attempts[pid] < MAX_PROCESS_ATTEMPTS,
                            "per-process complete rescan attempt bound exceeded")
                    event.update(retry_requested=True, retry_policy="native-owned-complete-rescan-fd-enoent-v1")
                    # Restart the entire FD enumeration. Sticky observations from
                    # the interrupted attempt remain evidence, not completion.
                    continue
                try:
                    record_departure(pid, initial, "proc-entry-vanished-during-descriptor-scan", error)
                except ProcessStillPresent as present:
                    event["departure_refusal"] = error_evidence(present)
                    if not exact_fd:
                        raise
                    request_retry(pid, event)
                    continue
                event["departure_confirmed"] = True
                return

    try:
        _, initial_roster = census()
        for pid in sorted(initial_roster):
            inspect_pid(pid)
        current, frontier = census()
        roster = initial_roster | frontier
        row.update(roster_pids=sorted(roster), birth_frontier_pids=sorted(frontier),
                   roster_closed_monotonic_ns=time.monotonic_ns(), reconciliation_rounds=1)
        for pid in sorted(set(inspected) - current):
            record_departure(pid, inspected[pid], "absent-from-followup-census")
        for pid in sorted(frontier):
            inspect_pid(pid)
        # One lifetime reconciliation, not a whole-host fixed point. Every roster
        # member was scanned completely or has positive departure evidence.
        for pid in sorted(inspected):
            initial = inspected[pid]
            context.clear()
            context.update(operation="identity_reconciliation", pid=pid,
                           initial_identity=copy.deepcopy(initial),
                           allowed_identity=copy.deepcopy(allowed.get(pid)))
            deadline()
            try:
                final = identity_reader(proc, pid)
                context.update(final_identity=copy.deepcopy(final),
                               changed_identity_fields=changed_identity_fields(initial, final))
                if restart_for_identity(pid, initial, final, "during-reconciliation"):
                    inspect_pid(pid)
                else:
                    require(final == initial, "PID identity changed during reconciliation")
            except FileNotFoundError as error:
                record_departure(pid, initial, "proc-entry-vanished-during-identity-reconciliation", error)
        current, late = census()
        row.update(late_birth_pids=sorted(late), terminal_census_monotonic_ns=time.monotonic_ns(),
                   terminal_pid_count=len(current), terminal_roster_pids=sorted(current & roster))
        for pid in sorted(set(inspected) - current):
            record_departure(pid, inspected[pid], "absent-from-terminal-census")
        for pid in sorted(allowed):
            deadline()
            context.clear()
            context.update(operation='owned_terminal_identity', pid=pid,
                           allowed_identity=copy.deepcopy(allowed[pid]))
            require(pid in current and pid in inspected, "allowlisted worker absent at sample closure")
            final = identity_reader(proc, pid)
            context.update(final_identity=copy.deepcopy(final),
                           changed_identity_fields=changed_identity_fields(allowed[pid], final))
            require(final == allowed[pid] == inspected[pid], "allowlisted worker changed at terminal endpoint")
        require(set(inspected) | departed == roster, "declared roster coverage incomplete")
        deadline()
        if native_owned_fd_rescan:
            row["completed_owned_users"] = [completed_owned[pid] for pid in sorted(completed_owned)]
            require(row["completed_owned_users"], "fresh complete owned GPU descriptor scan required")
        row["complete"] = True
        require(not row["foreign_users"], "foreign or unbound GPU descriptor owner")
        require(set(allowed) <= set(inspected), "allowlisted worker absent at sample closure")
        if phase == "active":
            require(row["owned_users"], "no positive owned GPU descriptor attribution")
        elif phase != "startup":
            require(not row["device_users"], "GPU descriptors remain at lifecycle endpoint")
        row["accepted"] = True
    except (OSError, ValueError, UnicodeError) as error:
        row["errors"].append(type(error).__name__ + ": " + str(error)[:4096])
        diagnostic = {"context": copy.deepcopy(context), "error": error_evidence(error)}
        pid = context.get("pid")
        if type(pid) is int:
            if clock() - started < MAX_SECONDS:
                diagnostic["post_refusal_process"] = refusal_process_observation(proc, pid)
            else:
                diagnostic["post_refusal_process"] = {"pid": pid, "skipped": "original scan deadline elapsed"}
        row["failure_diagnostics"].append(diagnostic)
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
    parser.add_argument("--phase", choices=("preflight", "startup", "active", "postflight"), required=True)
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
        row = {"schema": "FerricDeviceDescriptorSampleV2", "phase": args.phase,
               "method": "proc-fd-rdev-finite-roster-v1", "accepted": False, "complete": False,
               "diagnostic_only": False, "native_launch_admitted": False,
               "errors": [type(error).__name__ + ": " + str(error)[:4096]]}
    print(json.dumps(row, sort_keys=True, allow_nan=False))
    return 0 if row["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
