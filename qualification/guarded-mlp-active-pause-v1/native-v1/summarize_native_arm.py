"""Fixed one-arm original-data summary; no native launch or comparison."""
from decimal import Decimal
import hashlib
import io
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import resource
import shutil
import signal
import stat
import sys
import tarfile
import types

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
W = E / "guarded-mlp-active-pause-native-workflow-source-v228-v1"
REDUCER = E / "guarded-mlp-layer-phases-native-source-v228-v1/report_layer.py"
REDUCER_PIN = dict(bytes=32087, sha256="357e3ee2391b23bc8527a0d9bd515e1834422a09bbc5f51b38cf6c5c4198bb14")
ARMS = {
    "control": ("guarded-mlp-active-pause-control-native-evidence-v228-v1.tar.gz",
                "guarded-mlp-active-pause-control-native-retained-v228-v1",
                "guarded-mlp-readiness40-tail-layer-active-pause-control-tools-v228-v1",
                "guarded-mlp-readiness40-tail-layer-active-pause-control-gpu-v228-v1",
                "ferric-guarded-mlp-readiness40-tail-layer-active-pause-control", False),
    "candidate": ("guarded-mlp-active-pause-native-evidence-v228-v1.tar.gz",
                  "guarded-mlp-active-pause-native-retained-v228-v1",
                  "guarded-mlp-readiness40-tail-layer-active-pause-tools-v228-v1",
                  "guarded-mlp-readiness40-tail-layer-active-pause-gpu-v228-v1",
                  "ferric-guarded-mlp-readiness40-tail-layer-active-pause", True),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path, cap=16 << 20):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= cap, "bounded ordinary original")
    with path.open("rb") as stream:
        raw = stream.read(cap + 1)
        opened = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(opened) == stamp(path.lstat())
            and len(raw) == before.st_size, "original changed")
    return raw


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 3
            and sys.argv[1] in ARMS and re.fullmatch("[0-9a-f]{64}", sys.argv[2]),
            "python3 -B summarize_native_arm.py control|candidate ACTUAL_ARCHIVE_SHA")
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == "smci350-rck-g03-b19-03"
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
            and all(os.environ.get(k) == "" for k in
                    ("HIP_VISIBLE_DEVICES", "ROCR_VISIBLE_DEVICES", "CUDA_VISIBLE_DEVICES")),
            "exact unprivileged CPU8,9 nice10 hidden-GPU host")
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
    resource.setrlimit(resource.RLIMIT_FSIZE, (4 << 20, 4 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    signal.alarm(90)
    require(shutil.disk_usage(E).free >= 40 << 30, "initial40GiB floor")
    arm = sys.argv[1]
    archive_name, retained_name, tools_name, native_name, schema, changed = ARMS[arm]
    output = W / "native-summary-v1" / arm
    require(not os.path.lexists(output), "fresh exclusive arm output")
    source = read(Path(__file__).resolve(), 1 << 20)
    reducer_raw = read(REDUCER, 1 << 20)
    require(pin(reducer_raw) == REDUCER_PIN, "qualified reducer source")
    # Its authenticated module body defines pure reducers; __main__ is never entered.
    reducer = types.ModuleType("_qualified_layer_report")
    reducer.__file__ = str(REDUCER)
    exec(compile(reducer_raw, str(REDUCER), "exec"), reducer.__dict__)
    raw = read(E / archive_name)
    require(pin(raw)["sha256"] == sys.argv[2], "caller-observed original archive")
    bodies, expanded = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
        for member in archive:
            name, parts = member.name, PurePosixPath(member.name).parts
            require(member.isfile() and not member.issym() and not member.islnk()
                    and parts and not name.startswith("/") and "\\" not in name
                    and PurePosixPath(name).as_posix() == name
                    and all(p not in ("", ".", "..") for p in parts)
                    and name not in bodies and 0 <= member.size <= 8 << 20, "safe unique original member")
            expanded += member.size
            require(len(bodies) < 170 and expanded <= 16 << 20, "bounded170 archive")
            stream = archive.extractfile(member)
            require(stream is not None, "ordinary archive stream")
            body = stream.read(member.size + 1)
            require(len(body) == member.size, "complete original member")
            bodies[name] = body
    manifest = reducer.decode(bodies["manifest.json"])
    require(len(bodies) == 170 and len(manifest["files"]) == 169
            and set(manifest["files"]) == set(bodies) - {"manifest.json"}
            and all(pin(bodies[n]) == p for n, p in manifest["files"].items()), "all169 original pins")
    retained = E / retained_name
    require({p.relative_to(retained).as_posix() for p in retained.rglob("*") if not p.is_dir()}
            == set(bodies) and all(read(retained / n) == b for n, b in bodies.items()), "all170 retained bytes")
    receipt_path = E / tools_name / "retain-stdout.json"
    receipt_raw = read(receipt_path)
    receipt, observation = reducer.decode(receipt_raw), manifest["observation"]
    require(manifest["schema"] == schema + "-retention-v1" and manifest["source_root"] == str(E / native_name)
            and receipt["destination"] == str(retained) and receipt["members"] == 170
            and receipt["expanded_bytes"] == expanded
            and receipt["archive"] == dict(path=str(E / archive_name), **pin(raw))
            and {k: v for k, v in receipt.items() if k not in
                 ("archive", "destination", "members", "expanded_bytes")} == observation, "actual retained receipt join")
    case = observation["cases"]["tail_layer"]
    require(observation["passed"] is True and observation["polling_pause_changed"] is changed
            and observation["original_owned_lineage_revalidated"] is True
            and observation["semantic_and_payload_parity_revalidated"] is True
            and case["present"] is True and case["original_passed"] is True
            and case["retained_success_revalidated"] is True and case["original_raw_files"] == 73,
            "separately revalidated healthy original case")
    terminal_raw = bodies["tail_layer/complete.json"]
    terminal = reducer.decode(terminal_raw)
    require(manifest["outcomes"] == {"tail_layer": {"name": "complete.json", "sha256": pin(terminal_raw)["sha256"]}}
            and case["original_terminal"] == dict(name="complete.json", **pin(terminal_raw))
            and terminal["schema"] == schema + "-gpu-v1" and terminal["polling_pause_changed"] is changed
            and terminal["passed"] is True and terminal["errors"] == [] and terminal["native_attempts"] == 1
            and terminal["generated_tokens_requested"] == 0 and terminal["prompt_positions_requested"] == 40
            and case["matched_timing"] == terminal["matched_timing"], "original arm and terminal")
    require(len(terminal["phases"]) == 11 and len(terminal["raw"]) == 73, "original owned phase/raw extent")
    for row in terminal["phases"]:
        require(row["exit_code"] == 0 and row["owned_groups_absent"] is True
                and row["owned_processes_reaped"] is True and row["cleanup_signalled"] is False
                and row["reason"] is None, "original natural retirement")
    for name, meta in terminal["raw"].items():
        require(meta == dict(path=str(E / native_name / "tail_layer" / name),
                             **pin(bodies["tail_layer/" + name])), "terminal original raw join")
    require(all(value["performance_claim"] is False and value["numerical_acceptance"] is False
                for value in (observation, case, terminal)), "no inherited performance or numerical authority")
    parent_rows = [row for row in terminal["phases"] if row["label"] == "parent"]
    require(len(parent_rows) == 1, "one measured parent leaf")
    parent_raw = bodies["tail_layer/parent/result.json"]
    parent_leaf = reducer.decode(parent_raw)
    require(parent_leaf == {k: v for k, v in parent_rows[0].items() if k != "label"}, "original parent result join")
    usage = parent_leaf["children_rusage"]
    fields = {"ru_utime", "ru_stime", "ru_minflt", "ru_majflt", "ru_nvcsw", "ru_nivcsw"}
    require(set(usage) == {"who", "before", "after", "delta", "error"}
            and usage["who"] == "RUSAGE_CHILDREN" and usage["error"] is None, "successful original usage samples")
    for key in ("before", "after", "delta"):
        require(set(usage[key]) == fields, "exact six resource fields")
        for field, value in usage[key].items():
            require((type(value) is float and math.isfinite(value) if field in ("ru_utime", "ru_stime")
                     else type(value) is int) and value >= 0, "finite nonnegative typed resource field")
    require(all(usage["after"][k] >= usage["before"][k]
                and usage["delta"][k] == usage["after"][k] - usage["before"][k] for k in fields),
            "exact original cumulative resource deltas")
    rows, line_pins = reducer.worker_records(bodies, case["matched_timing"])
    parent = reducer.parent_timeline(bodies, case["matched_timing"], rows)
    subsets, nested = reducer.reduce_rows(rows)
    layers = reducer.reduce_layers(rows)
    report = dict(schema="ferric-active-pause-native-arm-host-report-v1", arm=arm, polling_pause_changed=changed,
        provenance=dict(archive=dict(path=str(E / archive_name), **pin(raw)), terminal=pin(terminal_raw),
            retention_receipt=dict(path=str(receipt_path), **pin(receipt_raw)), retained_root=str(retained),
            reporter=pin(source), reducer=REDUCER_PIN, retainer=pin(bodies["layer_retention_tool.py"]),
            parent_result=pin(parent_raw), records=line_pins),
        phase_order=list(reducer.PHASES), layer_stage_order=list(reducer.LAYER_STAGES),
        paired_mlp_stage_order=list(reducer.PAIRED_STAGES), forwards=rows, subsets=subsets,
        warm_nested=nested, warm_layers=layers, parent=parent,
        whole_parent_resource_usage=dict(scope="owned parent leaf and reaped descendants, before spawn through cleanup",
            children_rusage=usage, parent_leaf_wall_seconds=parent_leaf["elapsed_seconds"],
            user_plus_system_seconds_decimal=str(sum(Decimal(str(usage["delta"][k])) for k in ("ru_utime", "ru_stime"))),
            warm_only=False, peak_memory_measured=False, peak_rss_delta_used=False),
        prompt_forwards=40, generated_tokens=0, native_rerun=False, gpu_timing=False,
        overlap_claim=False, matched_comparison=False, performance_claim=False, optimization_ceiling_claim=False,
        numerical_acceptance=False, generated_token_correctness=False, full2303_acceptance=False,
        production_authority=False)
    outputs = {"report.json": (json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n").encode(),
               "host-phases.svg": reducer.svg(report)}
    require(all(len(b) <= 4 << 20 for b in outputs.values()), "two bounded outputs")
    require(read(Path(__file__).resolve(), 1 << 20) == source and read(REDUCER, 1 << 20) == reducer_raw
            and read(receipt_path) == receipt_raw and read(E / archive_name) == raw, "original pre/post source custody")
    require(shutil.disk_usage(E).free >= 38 << 30, "prewrite38GiB floor")
    output.parent.mkdir(mode=0o700, exist_ok=True)
    require(output.parent.resolve(strict=True) == output.parent, "ordinary fixed summary parent")
    output.mkdir(mode=0o700)
    for name, body in outputs.items():
        with (output / name).open("xb") as stream:
            stream.write(body)
        require(read(output / name, 4 << 20) == body, "output rehash")
    require(shutil.disk_usage(E).free >= 38 << 30, "final38GiB floor")
    signal.alarm(0)
    print(json.dumps(dict(directory=str(output), files={n: pin(b) for n, b in outputs.items()}), sort_keys=True))


if __name__ == "__main__":
    main()
