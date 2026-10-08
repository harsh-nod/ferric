"""Export successful control330 workflow originals; no imported project helpers."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import sys
import tarfile

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
R = E / "guarded-mlp-readiness40-tail-layer-active-pause-control-native-admission-cpu-v228-v1"
D = E / "guarded-mlp-active-pause-control-native-workflow-source-v228-v1"
DEST = E / "guarded-mlp-active-pause-control-native-admission-evidence-v228-v1.tar.gz"
INPUT = dict(bytes=234625, sha256="023d24a450bb25ff898ed7c6fc746200dc1f5c871c28e7b59794ab1756d9a3e4")
SEAL = dict(bytes=39233, sha256="c5541a176046ee15049007b89d6b2ad90159626e4f70d23c449e705bb8a4e9f1")
CONTROLLER = dict(bytes=15026, sha256="b785ae085d41e6552f493e2c40516b3456cfa7197474d7a983d2ba535bd76290")
SUPERVISOR_SHA = "8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc"
COUNTS = {"legacy-tests": 171, "forward-cpu-tests": 29,
          "forward-retention-tests": 12, "forward-preparation-tests": 15,
          "layer-cpu-tests": 44, "layer-retention-tests": 25, "layer-preparation-tests": 20,
          "usage-tests": 14}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path, cap=32 << 20):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= cap, "bounded ordinary original")
    with path.open("rb") as stream:
        raw = stream.read(cap + 1)
        opened = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_uid, s.st_gid,
                       s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(opened) == stamp(path.lstat())
            and len(raw) == before.st_size, "original changed")
    return raw


def compact(value):
    return {k: value[k] for k in ("bytes", "sha256")}


def original(path, expected):
    raw = read(path)
    require(pin(raw) == compact(expected), "original body pin: " + str(path))
    if "path" in expected:
        require(expected["path"] == str(path), "original path identity")
    return raw


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch("[0-9a-f]{64}", sys.argv[1]), "python3 -B export_native_cpu.py COMPLETE_SHA")
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == "smci350-rck-g03-b19-03", "exact unprivileged host")
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            "CPU8,9 nice10")
    require(all(os.environ.get(k) == "" for k in
                ("HIP_VISIBLE_DEVICES", "ROCR_VISIBLE_DEVICES", "CUDA_VISIBLE_DEVICES")), "hidden GPUs")
    require(not os.path.lexists(DEST) and shutil.disk_usage(E).free >= 40 << 30,
            "exclusive archive and initial storage floor")
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 120))
    resource.setrlimit(resource.RLIMIT_FSIZE, (32 << 20, 32 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    signal.alarm(180)
    input_raw = original(R / "input-manifest.json", INPUT)
    spec = json.loads(input_raw)
    result_raw = read(R / "evidence/complete.json")
    require(pin(result_raw)["sha256"] == sys.argv[1], "caller-observed successful terminal")
    result = json.loads(result_raw)
    require(result["schema"] == "ferric-active-pause-control-native-admission-cpu-v1"
            and result["passed"] is True and result["failure"] is None
            and result["postcheck_errors"] == [], "successful original workflow only")
    require(spec["schema"] == "ferric-active-pause-control-native-admission-input-v1"
            and spec["root"] == str(R) and spec["leaf_order"] == list(COUNTS)
            and set(spec["leaves"]) == set(COUNTS) and spec["total_tests"] == 330
            and spec["source_files"] == len(spec["sources"]) == 377
            and spec["original_fixture_files"] == 152 and spec["fixture_copies"] == 2
            and spec["gpu_execution"] is False, "original staged closure")
    require(result["input_manifest"] == dict(path=str(R / "input-manifest.json"), **INPUT)
            and compact(result["controller"]) == CONTROLLER
            and result["controller"]["path"] == str(R / "run_cpu.py")
            and result["supervisor"]["sha256"] == SUPERVISOR_SHA, "original controller/input custody")
    require(result["source_unchanged"] is True
            and result["sources_before"] == result["sources_after"]
            and set(result["sources_after"]) == set(spec["sources"]), "whole source pre/post identity")
    require(result["synthetic_and_actual_artifact_backed_tests"] is True
            and result["actual_cpu_artifact_reads"] is True
            and result["synthetic_data_tests_only"] is False, "honest test scope")
    for field in ("model_shard_reads", "gpu_execution", "native_parent_execution", "model_execution",
                  "numerical_acceptance", "full_model_acceptance", "performance_claim", "production_authority"):
        require(result[field] is False, "no execution or acceptance authority")
    require(result["inherited_tests"] == 171 and result["forward_cpu_tests"] == 29
            and result["forward_retention_tests"] == 12 and result["forward_preparation_tests"] == 15
            and result["layer_cpu_tests"] == 44 and result["layer_retention_tests"] == 25
            and result["layer_preparation_tests"] == 20 and result["usage_tests"] == 14
            and result["original_fixture_files"] == 152, "test census labels")
    require(result["limits"] == dict(whole_seconds=1000, leaf_seconds={n: 120 for n in COUNTS},
        cleanup_reserve_seconds=50, address_space_bytes=512 << 20, file_bytes=16 << 20,
        stream_bytes=4 << 20, temporary_bytes=64 << 20, affinity=[8, 9], nice=10, cargo_jobs=2,
        initial_free_bytes=40 << 30, live_free_bytes=38 << 30)
        and type(result["elapsed_seconds"]) in (int, float)
        and 0 <= result["elapsed_seconds"] <= 1000, "bounded eight-leaf lifecycle")
    environment = dict(PATH="/usr/bin:/bin", HOME="/home/harmenon", LANG="C.UTF-8", LC_ALL="C.UTF-8",
        TZ="UTC", PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1", PYTHONNOUSERSITE="1", TMPDIR=str(R / "tmp"),
        HIP_VISIBLE_DEVICES="", ROCR_VISIBLE_DEVICES="", CUDA_VISIBLE_DEVICES="", CARGO_BUILD_JOBS="2",
        OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1")
    require(result["environment"] == environment, "original hidden-GPU environment")
    require(set(result["tool_pins"]) == {"python", "prlimit"}, "closed original tools")
    for name, alias in (("python", "/usr/bin/python3"), ("prlimit", "/usr/bin/prlimit")):
        tool = Path(alias).resolve(strict=True)
        original(tool, result["tool_pins"][name])
    bodies = {"input-manifest.json": input_raw, "evidence/complete.json": result_raw}
    for name, expected in spec["sources"].items():
        require(not Path(name).is_absolute() and Path(name).as_posix() == name
                and all(p not in ("", ".", "..") for p in Path(name).parts), "safe source-relative name")
        raw = original(R / name, result["sources_after"][name])
        require(pin(raw) == expected, "staged source pin")
        bodies[name] = raw
    require(pin(bodies["active-pause-source-seal.json"]) == SEAL
            and pin(bodies["run_cpu.py"]) == CONTROLLER, "fixed seal/controller")
    observed = set()
    for path in R.rglob("*"):
        rel = path.relative_to(R)
        if rel.parts[0] in ("evidence", "tmp"):
            continue
        require(path.resolve(strict=True) == path, "canonical staged tree")
        if not path.is_dir():
            require(stat.S_ISREG(path.lstat().st_mode), "ordinary staged file")
            observed.add(rel.as_posix())
    require(observed == set(spec["sources"]) | {"input-manifest.json"}
            and not any((R / "tmp").iterdir()), "closed originals and retired temporary files")
    allowed_raw = {"sources-before.json", "sources-after.json"} | {
        label + "." + suffix for label in COUNTS
        for suffix in ("command.json", "started.json", "result.json", "stdout", "stderr")}
    require(set(result["raw"]) == allowed_raw and len(allowed_raw) == 42, "exact42 raw originals")
    require({p.name for p in (R / "evidence").iterdir()} == allowed_raw | {"complete.json"},
            "no failed or unrelated evidence")
    for name, expected in result["raw"].items():
        bodies["evidence/" + name] = original(R / "evidence" / name, expected)
    for name in ("before", "after"):
        require(json.loads(bodies["evidence/sources-" + name + ".json"]) == result["sources_" + name],
                "original source snapshot join")
    require(len(result["phases"]) == 8 and [r["label"] for r in result["phases"]] == list(COUNTS)
            and set(result["leaf_tests"]) == set(COUNTS), "eight exact ordered leaves")
    all_names = []
    for row, (label, count) in zip(result["phases"], COUNTS.items()):
        require(type(row["pid"]) is int and row["pid"] > 0 and row["pgid"] == row["pid"]
                and row["exit_code"] == 0 and row["natural_exit"] is True
                and row["reaped"] is True and row["process_group_absent"] is True
                and row["forced_cleanup"] is False and row["timed_out"] is False
                and row["exception"] is None and row["storage_failure"] is None
                and row["observed_signals"] == [] and row["adopted_reaped"] == [],
                "clean natural owned retirement")
        leaf = spec["leaves"][label]
        require(leaf["directory"] == ("legacy" if label == "legacy-tests" else ".")
                and leaf["count"] == count, "private legacy/new test namespace")
        directory = R / leaf["directory"]
        script = ("import sys,unittest;sys.path.insert(0," + repr(str(directory)) + ");"
                  "suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(n) for n in "
                  + repr(leaf["modules"]) + ");"
                  "result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())")
        argv = ["/usr/bin/prlimit", "--as=536870912", "--cpu=120", "--fsize=16777216", "--core=0", "--",
                result["tool_pins"]["python"]["path"], "-I", "-B", "-c", script]
        command = json.loads(bodies["evidence/" + label + ".command.json"])
        require(set(command) == {"argv", "cwd", "env", "wall_timeout_seconds"}
                and command["argv"] == row["argv"] == argv and command["cwd"] == str(directory)
                and command["env"] == environment and type(command["wall_timeout_seconds"]) in (int, float)
                and 0 < command["wall_timeout_seconds"] <= 120, "original bounded command")
        require(json.loads(bodies["evidence/" + label + ".result.json"]) == row, "original phase result")
        require(json.loads(bodies["evidence/" + label + ".started.json"])
                == dict(pid=row["pid"], pgid=row["pgid"], argv=argv), "original child registration")
        for key, suffix in (("command", "command.json"), ("stdout", "stdout"), ("stderr", "stderr")):
            require(row[key] == result["raw"][label + "." + suffix], "phase/raw pin join")
        names = []
        for module in leaf["modules"]:
            rel = ("" if leaf["directory"] == "." else "legacy/") + module + ".py"
            for cls in ast.parse(bodies[rel]).body:
                if isinstance(cls, ast.ClassDef):
                    names.extend(module + "." + cls.name + "." + f.name for f in cls.body
                                 if isinstance(f, ast.FunctionDef) and f.name.startswith("test_"))
        require(sorted(names) == leaf["names"] and len(names) == len(set(names)) == count, "source-level names")
        require(bodies["evidence/" + label + ".stdout"] == b"", "original empty test stdout")
        stderr = bodies["evidence/" + label + ".stderr"].decode("utf-8")
        expression = r"^(test_[A-Za-z0-9_]+) \(([A-Za-z0-9_]+\.[A-Za-z0-9_]+)\.\1\) \.\.\. ok$"
        found = [prefix + "." + name for name, prefix in re.findall(expression, stderr, re.M)]
        require(sorted(found) == sorted(names) and len(found) == len(set(found)) == count, "raw named successes")
        tail = "\n".join(line for line in re.sub(expression, "", stderr, flags=re.M).splitlines() if line)
        require(re.fullmatch(r"-{70}\nRan " + str(count) + r" tests in [0-9]+\.[0-9]+s\nOK", tail),
                "raw unittest summary without skips")
        require(result["leaf_tests"][label] == dict(names=sorted(names), passed=count, failed=0, errors=0, skipped=0),
                "original leaf census")
        all_names.extend(names)
    require(len(all_names) == len(set(all_names)) == 330
            and result["tests"] == dict(names=sorted(all_names), passed=330, failed=0, errors=0, skipped=0),
            "exact330 combined names")
    require(len(bodies) == 421 and sum(map(len, bodies.values())) <= 32 << 20, "bounded421 original bodies")
    stager = D / "stage_layer_native_cpu.py"
    bodies["authoring/stage_layer_native_cpu.py"] = original(stager, spec["authoring_readset"][str(stager)])
    manifest = dict(schema="ferric-active-pause-control-native-admission-export-v1",
        files={name: pin(raw) for name, raw in sorted(bodies.items())}, terminal=pin(result_raw),
        passed_tests=330, legacy_tests=227, new_tests=103, source_files=377, raw_files=42,
        usage_tests=14, staged_but_unrun_attempts_credited=False,
        synthetic_and_actual_artifact_backed_tests=True, gpu_execution=False,
        native_parent_execution=False, model_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
    bodies["manifest.json"] = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
    with DEST.open("xb") as stream:
        with tarfile.open(fileobj=stream, mode="w:gz") as archive:
            for name, raw in sorted(bodies.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(raw), 0o644, 0
                archive.addfile(member, io.BytesIO(raw))
    require(shutil.disk_usage(E).free >= 38 << 30, "final live storage floor")
    signal.alarm(0)
    print(json.dumps(dict(archive=str(DEST), **pin(read(DEST)), members=len(bodies),
        terminal=pin(result_raw), elapsed_seconds=result["elapsed_seconds"]), sort_keys=True))


if __name__ == "__main__":
    main()

