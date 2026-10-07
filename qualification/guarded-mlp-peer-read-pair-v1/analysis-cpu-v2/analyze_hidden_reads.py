"""Analyze already-qualified host observations; never launch a model or GPU."""
import hashlib
import copy
import json
from pathlib import Path
import statistics
import sys


ORDER = ("control-0", "paired-0", "paired-1", "control-1")
COUNTERS = ["commands", "command_ns", "full_currentness_checks", "full_currentness_ns",
            "operational_currentness_checks", "operational_currentness_ns", "kernel_admissions",
            "kernel_admission_ns", "dispatches", "dispatch_prepare_ns", "dispatch_publish_ns",
            "dispatch_wait_ns", "completion_polls", "reads", "read_bytes", "read_ns", "writes",
            "write_bytes", "write_ns"]
SHARED = ["group_full_checks", "group_full_ns", "publication_full_checks", "publication_full_ns"]


def require(ok, why):
    if not ok:
        raise ValueError(why)


def parse(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, "duplicate JSON field")
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, "nonfinite JSON"))


def uint(value):
    require(type(value) is int and 0 <= value < 1 << 64, "u64 counter")
    return value


def extent(raw, pin):
    require(type(pin) is dict and type(pin.get("bytes")) is int
            and pin["bytes"] == len(raw)
            and pin.get("sha256") == hashlib.sha256(raw).hexdigest(), "raw extent/hash")


def artifact(pin):
    require(type(pin) is dict and set(pin) == {"path", "bytes", "sha256"}, "artifact pin")
    require(type(pin["bytes"]) is int and pin["bytes"] > 0
            and type(pin["sha256"]) is str and len(pin["sha256"]) == 64
            and all(c in "0123456789abcdef" for c in pin["sha256"]), "artifact identity")
    return pin["bytes"], pin["sha256"]


def inspect_case(terminal, read_body, paired):
    require(terminal["passed"] is True and terminal["errors"] == []
            and terminal["native_attempts"] == 1 and terminal["retries"] == 0
            and terminal["gpu_execution_confirmed"] is True
            and terminal["host_observation_verified"] is True
            and terminal["shared_full_currentness_verified"] is True
            and terminal["paired_read_requested"] is paired,
            "successful, explicit paired-read case required")
    require(all(terminal[name] is False for name in
                ("performance_claim", "production_authority", "full_model_acceptance",
                 "numerical_acceptance", "full_long_workload")), "scope flags")
    phases = terminal["phases"]
    require(len(phases) == 11, "complete owned phase roster")
    for row in phases:
        require(type(row["exit_code"]) is int and row["exit_code"] == 0
                and row["reason"] is None and row["cleanup_signalled"] is False
                and row["owned_groups_absent"] is True
                and row["owned_processes_reaped"] is True, "natural owned cleanup")
    host = terminal["host_observation"]
    raw = read_body("host-observation.json")
    extent(raw, terminal["raw"]["host-observation.json"])
    require(parse(raw) == host, "checked host report/terminal join")
    raw = read_body("native/child-stderr.bin")
    extent(raw, host["source"])
    extent(raw, terminal["raw"]["native/child-stderr.bin"])
    require(host["counter_names"] == COUNTERS and host["shared_counter_names"] == SHARED
            and host["snapshots"] == 587 and host["intervals"] == 586
            and host["same_run_completions_joined"] is True
            and host["native_close_confirmed"] is True
            and host["inclusive_nested_host_scopes"] is True
            and host["shared_full_currentness"] is True
            and all(host[name] is False for name in
                    ("gpu_time", "gpu_overlap", "throughput", "numerical_acceptance",
                     "full_model_acceptance", "performance_claim", "production_authority")),
            "host report scope and counter semantics")
    comparison = terminal["instrumentation_comparison"]
    require(comparison["all_payloads_equal"] is True
            and comparison["all_histories_equal"] is True, "ordinary AR4 equality required")
    summary_raw = read_body("native/complete.json")
    extent(summary_raw, terminal["raw"]["native/complete.json"])
    summary = parse(summary_raw)
    immutable_request = copy.deepcopy(summary["request"])
    decode = immutable_request["decode"]
    require(decode["mode"] == "autoregressive", "AR4 comparison only")
    session, child = decode["session"], uint(summary["child_pid"])
    require(type(session) is list and len(session) == 32 and child > 0
            and all(type(v) is int and 0 <= v <= 255 for v in session)
            and any(session), "actual distinct session/child")
    for key in ("session", "evidence_directory"):
        del decode[key]
    payloads = []
    for position in range(4):
        name = "native/observation-%d.bin" % position
        body = read_body(name)
        extent(body, terminal["raw"][name])
        require(len(body) == 606976, "complete finite-model payload extent")
        payloads.append(hashlib.sha256(body).hexdigest())
    rows = host["forward_rows"]
    require(type(rows) is list and len(rows) == 4, "four measured forwards")
    totals = []
    for position, row in enumerate(rows):
        require(type(row["position"]) is int and row["position"] == position
                and len(row["layers"]) == 36, "ordered 36-layer forward")
        elapsed = 0
        for layer, entry in enumerate(row["layers"]):
            require(type(entry["layer"]) is int and entry["layer"] == layer, "layer order")
            hidden = entry["hidden_read"]
            require(set(hidden) == {"host_elapsed_ns", "ranks", "shared"}
                    and len(hidden["ranks"]) == 2 and len(hidden["shared"]) == 4,
                    "closed hidden-read interval")
            shared = list(map(uint, hidden["shared"]))
            require(shared[0] == (2 if paired else 4) and shared[2:] == [0, 0],
                    "expected fresh group fences, no publication")
            for rank in hidden["ranks"]:
                require(type(rank) is list and len(rank) == 19, "rank counter shape")
                rank = list(map(uint, rank))
                require(rank[2] == (0 if paired else 2)
                        and rank[4:13] == [0] * 9 and rank[13:15] == [1, 8192]
                        and rank[16:19] == [0, 0, 0] and rank[:2] == [0, 0],
                        "one complete rank read, no hidden dispatch or writes")
            elapsed += uint(hidden["host_elapsed_ns"])
        require(elapsed <= uint(row["bracket_host_ns"])
                and uint(row["forward_host_ns"]) <= row["bracket_host_ns"],
                "disjoint read intervals within forward bracket")
        totals.append(dict(position=position, hidden_read_ns=elapsed,
                           forward_host_ns=row["forward_host_ns"],
                           bracket_host_ns=row["bracket_host_ns"]))
    return dict(worker=artifact(terminal["admission"]["worker"]),
                parent=artifact(terminal["admission"]["parent"]),
                immutable_request=immutable_request, platform=terminal["platform"],
                session=session, child_pid=child,
                input_tokens=terminal["observation"]["input_tokens"],
                payloads=payloads, forwards=totals,
                hidden_read_total_ns=sum(row["hidden_read_ns"] for row in totals))


def compare(cases, chronology):
    require(type(cases) is dict and tuple(cases) == ORDER, "predeclared ABBA order")
    require(type(chronology) is list and len(chronology) == 4, "supervised run chronology")
    previous = 0
    for name, row in zip(ORDER, chronology):
        require(set(row) == {"name", "terminal", "started_monotonic_ns", "finished_monotonic_ns"}
                and row["name"] == name, "chronology roster")
        begin, end = uint(row["started_monotonic_ns"]), uint(row["finished_monotonic_ns"])
        require(previous <= begin < end, "non-overlapping actual execution order")
        artifact(row["terminal"])
        previous = end
    require(len({tuple(case["session"]) for case in cases.values()}) == 4
            and len({case["child_pid"] for case in cases.values()}) == 4,
            "four independent owned processes/sessions")
    first = cases[ORDER[0]]
    for case in cases.values():
        require(all(case[key] == first[key] for key in
                    ("worker", "parent", "immutable_request", "platform", "input_tokens", "payloads")),
                "same executables, model inputs, platform, histories and complete payloads")
    control = [cases[key]["hidden_read_total_ns"] for key in (ORDER[0], ORDER[3])]
    paired = [cases[key]["hidden_read_total_ns"] for key in (ORDER[1], ORDER[2])]
    require(all(type(value) is int and value > 0 for value in control + paired), "positive sample")
    base, candidate = statistics.mean(control), statistics.mean(paired)
    return dict(schema="ferric-hidden-pair-read-abba-analysis-v1", run_order=list(ORDER),
                runs=cases, chronology=chronology, independent_runs_per_mode=2,
                control_mean_hidden_read_ns=base, paired_mean_hidden_read_ns=candidate,
                observed_hidden_read_reduction_fraction=1 - candidate / base,
                observed_hidden_read_ratio=base / candidate,
                per_forward_observations_are_not_independent_runs=True,
                gpu_time=False, gpu_overlap=False, end_to_end_speedup=False,
                sustained_tokens_per_second=False, numerical_acceptance=False,
                full_model_acceptance=False, production_authority=False)


def main():
    require(len(sys.argv) == 3, "analyze_hidden_reads.py CAPSULE_ROOT MANIFEST_SHA256")
    root = Path(sys.argv[1]).resolve(strict=True)
    manifest_path = root / "comparison-input.json"
    require(manifest_path.resolve(strict=True) == manifest_path and manifest_path.is_file()
            and manifest_path.stat().st_size <= 65536, "bounded ordinary comparison manifest")
    manifest_raw = manifest_path.read_bytes()
    require(hashlib.sha256(manifest_raw).hexdigest() == sys.argv[2], "authenticated comparison manifest")
    manifest = parse(manifest_raw)
    require(set(manifest) == {"schema", "controller", "runs"}
            and manifest["schema"] == "ferric-peer-read-supervised-abba-v1", "supervised comparison manifest")
    artifact(manifest["controller"])
    require(type(manifest["runs"]) is list and len(manifest["runs"]) == 4, "four supervised runs")
    cases = {}
    for name, entry in zip(ORDER, manifest["runs"]):
        require(entry["name"] == name, "declared run order")
        directory = root / name
        def read_body(relative):
            path = directory / relative
            require(path.resolve(strict=True) == path and path.is_file()
                    and path.stat().st_size <= 8 << 20, "bounded ordinary capsule body")
            return path.read_bytes()
        raw = read_body("complete.json")
        extent(raw, entry["terminal"])
        cases[name] = inspect_case(parse(raw), read_body, name.startswith("paired"))
    report = compare(cases, manifest["runs"])
    report["supervisor_manifest_sha256"] = sys.argv[2]
    report["supervisor_controller"] = manifest["controller"]
    print(json.dumps(report, sort_keys=True, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
