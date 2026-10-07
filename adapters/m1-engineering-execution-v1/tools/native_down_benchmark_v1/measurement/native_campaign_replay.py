#!/usr/bin/env python3
"""Read-only retained-cell loader for the fixed, shared-input V14 campaign."""
import importlib.util
from pathlib import Path
import stat
import os


_SPEC = importlib.util.spec_from_file_location("v14_retained_cell", Path(__file__).with_name("native_token_cell.py"))
cell = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(cell)
ledger, require = cell.ledger, cell.require
CELL_ORDER = [("counter-A", "A", "counters"), ("counter-B", "B", "counters")] + [
    (f"block{block}-{position}", position[0], "latency")
    for block in range(1, 4) for position in ("A1", "B1", "B2", "A2")]
LIMIT = 32 * 1024**2


def private_directory(path):
    path = Path(path)
    item = path.lstat()
    require(path.is_absolute() and path.resolve(strict=True) == path and stat.S_ISDIR(item.st_mode)
            and item.st_uid == os.getuid() and stat.S_IMODE(item.st_mode) == 0o700,
            "canonical owned mode0700 evidence directory required")
    return path


def read_json(path):
    raw = cell.retained_bytes(path, LIMIT)
    return raw, ledger.parse(raw)


def same(left, right, message):
    require(ledger.canonical(left) == ledger.canonical(right), message)


def outer_argv(argv, stage, cell_id, plan_binding):
    require(type(argv) is list and argv[:4] == ["/usr/bin/python3", "-I", "-B", str(stage / "run_stage.py")],
            "outer receipt is not the bound V14 Python driver")
    flags = argv[4:]
    require(flags.count("--execute") == 1, "one outer execute mode required")
    flags = [flag for flag in flags if flag != "--execute"]
    require(len(flags) == 6, "closed outer execute argv required")
    options = {}
    for index in range(0, 6, 2):
        key, value = flags[index:index + 2]
        require(key in ("--plan", "--plan-sha256", "--cell-id") and key not in options,
                "unknown or duplicate outer execute selector")
        options[key] = value
    same(options, {"--plan": plan_binding["path"], "--plan-sha256": plan_binding["sha256"], "--cell-id": cell_id},
         "outer command must bind this exact campaign and cell")


def validate_outer(outer, completion, result_raw, completion_raw):
    cell.exact(outer, {"status": 0, "cleanup_ok": True, "child_reaped": True, "errors": [],
                       "term_sent": False, "kill_sent": False, "termination_reason": "completed"},
               "final outer supervisor failed")
    require(outer.get("postflight", {}).get("accepted") is True, "final outer postflight failed")
    cell.exact(completion, {"accepted": True, "model_stable": True, "input_files_stable": True},
               "accepted stable-input completion required")
    require(outer.get("completion_sha256") == ledger.sha(completion_raw)
            and completion.get("cell_result_sha256") == ledger.sha(result_raw),
            "outer completion chain does not bind actual raw retained files")


def load_retained_cell(entry, cell_root, *, plan_binding, runner, legacy, evidence):
    """Load/replay one completed cell; performs no launch or process discovery.

    entry is either the complete predeclared campaign row or its exact
    {cell_id, spec} projection. plan_binding names the existing shared plan.json
    and its externally retained raw SHA-256. Evidence must still be at the
    original canonical shared-stage paths; arbitrary relocation is not inferred.
    """
    require(type(entry) is dict and set(entry) in ({"cell_id", "spec"}, {"cell_id", "spec", "output", "common_args"}),
            "closed retained campaign entry required")
    cell_id, spec = entry["cell_id"], entry["spec"]
    require(cell_id in [row[0] for row in CELL_ORDER], "predeclared fixed cell ID required")
    count, _ = cell.shape(spec)
    output = private_directory(cell_root)
    require(output.name == cell_id and output.parent.name == "cells", "fixed per-cell output layout required")
    private_directory(output.parent)
    stage = private_directory(output.parent.parent)
    raw_dir = private_directory(output / "cell-results")
    require(type(plan_binding) is dict and set(plan_binding) == {"path", "sha256"}
            and plan_binding["path"] == str(stage / "plan.json"), "exact shared campaign plan binding required")
    ledger.hash_string(plan_binding["sha256"])
    plan_raw, plan = read_json(Path(plan_binding["path"]))
    require(ledger.sha(plan_raw) == plan_binding["sha256"], "campaign plan hash changed")
    require(plan.get("schema") == "FerricNativeDownCampaignPlanR1" and plan.get("stage") == str(stage)
            and type(plan.get("cells")) is list and len(plan["cells"]) == 14, "complete shared-input campaign required")
    for row, (expected_id, arm, mode) in zip(plan["cells"], CELL_ORDER):
        require(row.get("cell_id") == expected_id and row.get("output") == str(stage / "cells" / expected_id)
                and row.get("spec", {}).get("arm") == arm and row["spec"].get("mode") == mode,
                "campaign roster/order changed")
    declared = next(row for row in plan["cells"] if row["cell_id"] == cell_id)
    same(entry, declared if "output" in entry else {"cell_id": declared["cell_id"], "spec": declared["spec"]},
         "retained entry differs from immutable campaign plan")
    status = cell.retained_bytes(output / "launch.status", 16)
    require(status == b"0\n", "actual outer exit status must be zero")
    result_raw, result = read_json(raw_dir / "result.json")
    completion_raw, completion = read_json(output / "completion.json")
    outer_raw, outer = read_json(output / "launch-supervisor.json")
    validate_outer(outer, completion, result_raw, completion_raw)
    require(completion.get("cell_id") == cell_id, "completion belongs to a different campaign cell")
    outer_argv(outer.get("argv"), stage, cell_id, plan_binding)
    cell.exact(result, {"schema": "FerricNativeDownTokenCellResultR1", "spec_sha256": ledger.digest(spec),
                        "accepted": True, "arm": spec["arm"], "mode": spec["mode"],
                        "raw_replay_passed": True, "instrumented": spec["mode"] == "counters",
                        "latency_admitted": spec["mode"] == "latency"}, "retained cell identity/status differs")
    cleanup = result.get("cleanup")
    cell.exact(cleanup, {"cleanup_ok": True, "child_reaped": True, "owned_descendants_absent": True,
                         "returncode": 0, "errors": [], "term_sent": False, "kill_sent": False},
               "inner controller cleanup failed")
    raw = {name: cell.retained_bytes(raw_dir / name, runner.MAX_STREAM)
           for name in ("stdin.raw", "stdout.raw", "stderr.raw")}
    same(result.get("raw_sha256"), {name: ledger.sha(value) for name, value in raw.items()},
         "actual raw transcript hashes differ")
    replay = cell.consume(cell.Replay(raw_dir, runner, count), spec, runner=runner, legacy=legacy,
                          evidence=evidence, deadline=0)
    for key, value in replay.items():
        same(result.get(key), value, "independent native raw replay differs: " + key)
    if spec["mode"] == "counters":
        same(result.get("mechanism"), cell.counter_replay(raw["stderr.raw"], spec, replay["setup"]),
             "counter raw replay differs")
    else:
        require(not raw["stderr.raw"] and "mechanism" not in result, "counter data cannot enter a latency cell")
    endpoint_raw, endpoints = read_json(output / "process-endpoints.json")
    require(type(endpoints.get("placements")) is list and len(endpoints["placements"]) == 2
            and type(endpoints.get("endpoints")) is list and len(endpoints["endpoints"]) == 2,
            "paired process placement/CPU endpoint evidence required")
    same(endpoints["placements"][0], endpoints["placements"][1], "process placement changed")
    placement = endpoints["placements"][0]
    require(placement.get("worker", {}).get("process_id") == replay["setup"]["worker_pids"][0]
            and placement["worker"].get("parent_pid") == placement.get("controller", {}).get("process_id"),
            "CPU endpoints must describe this cell's worker and controller")
    for endpoint in endpoints["endpoints"]:
        for role in ("worker", "controller"):
            process = endpoint.get("cpu", {}).get(role, {}).get("process", {})
            require(all(process.get(key) == value for key, value in placement[role].items()),
                    "CPU endpoint process identity differs from placement")
            executable = endpoint.get("executables", {}).get(role, {}).get("opened_executable", {})
            require(executable.get("sha256") == spec[role]["sha256"], "CPU endpoint binary differs from measured arm")
    cost = endpoints.get("cpu_cost", {})
    recomputed = evidence.cpu_cost(*endpoints["endpoints"], cost.get("clock_ticks_per_second"))
    for value in recomputed["roles"].values():
        value["cpu_seconds_per_token"] = value["cpu_seconds"] / (count * 128)
    recomputed["scope"] = "All requests including excluded warmups; excludes setup/teardown; not GPU time"
    recomputed["output_tokens"] = count * 128
    same(cost, recomputed, "CPU costs do not replay from raw endpoints")
    same(completion.get("cpu_cost"), recomputed, "completion CPU cost binding differs")
    # Re-read retained inputs after replay, so mutable files cannot race the audit.
    for path, before in ((Path(plan_binding["path"]), plan_raw), (raw_dir / "result.json", result_raw),
                         (output / "completion.json", completion_raw), (output / "launch-supervisor.json", outer_raw),
                         (output / "process-endpoints.json", endpoint_raw)):
        require(cell.retained_bytes(path, LIMIT) == before, "retained evidence changed during replay")
    require(cell.retained_bytes(output / "launch.status", 16) == status, "outer status changed during replay")
    for name, before in raw.items():
        require(cell.retained_bytes(raw_dir / name, runner.MAX_STREAM) == before, "raw transcript changed during replay")
    return {"cell_id": cell_id, "spec": spec, "result": result, "outer": outer, "completion": completion,
            "custody": {"plan": plan_binding, "outer_sha256": ledger.sha(outer_raw),
                        "completion_sha256": ledger.sha(completion_raw), "result_sha256": ledger.sha(result_raw),
                        "process_endpoints_sha256": ledger.sha(endpoint_raw)}}


def validate_counter_pair(entries):
    """Gate the first latency launch on separately replayed A/B mechanism cells."""
    require(type(entries) is list and len(entries) == 2
            and [entry.get("cell_id") for entry in entries] == ["counter-A", "counter-B"], "exact ordered counter pair required")
    normalized = None
    deltas = {}
    controller_sha = None
    for entry, arm in zip(entries, ("A", "B")):
        spec, result = entry["spec"], entry["result"]
        require(spec["arm"] == arm and spec["mode"] == "counters" and result.get("accepted") is True
                and result.get("latency_admitted") is False and result.get("raw_replay_passed") is True,
                "successful separately replayed mechanism cell required")
        cell.shape(spec)
        require(controller_sha is None or controller_sha == spec['controller']['sha256'],
                'counter pair must use one exact width-selector binary')
        controller_sha = spec['controller']['sha256']
        comparable = {**spec, "arm": "<prefill-mode>", "argv": cell.common_argv(spec),
                      "controller": {"path": "<arm-controller>", "sha256": "<arm-sha256>"}}
        same(comparable, normalized if normalized is not None else comparable, "counter pair composition changed")
        normalized = comparable
        mechanism = result["mechanism"]
        raw = b"\n".join(ledger.canonical(mechanism[key]) for key in ("start", "end")) + b"\n"
        same(mechanism, cell.counter_replay(raw, spec, result["setup"]), "counter pair mechanism replay changed")
        deltas[arm] = mechanism["delta"]
    return {"schema": "FerricNativeDownMechanismPairR1", "accepted": True, "latency_admitted": False, "deltas": deltas}
