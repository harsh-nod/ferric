"""Copy only CPU-qualified active-pause workflow sources; never prepare or launch."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import sys

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
D = E / "guarded-mlp-active-pause-native-workflow-source-v228-v1"
QUALIFIED = E / "guarded-mlp-readiness40-tail-layer-active-pause-native-admission-cpu-v228-v2"
ROOT = E / "guarded-mlp-readiness40-tail-layer-active-pause-gpu-v228-v1"
TOOLS = E / "guarded-mlp-readiness40-tail-layer-active-pause-tools-v228-v1"
INPUT = {'bytes': 235264, 'sha256': 'e5d7513f246b3b1032f80694102f74892f746ca890c1881e917e58852f852ee7'}
SUPERVISOR_SHA = "8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc"
RETAINER = {'bytes': 50407, 'sha256': 'da5a58d10a677562fb3de76ce90c4bad58b08188cd54f4095589840e482d8ec2'}
COUNTS = {'legacy-tests': 171, 'forward-cpu-tests': 29, 'forward-retention-tests': 12, 'forward-preparation-tests': 15, 'layer-cpu-tests': 50, 'layer-retention-tests': 25, 'layer-preparation-tests': 20, 'usage-tests': 14}
PRODUCTION = {
    "run_layer_model_gpu.py", "prepare_layer_model_inputs.py", "layer_cpu_admission.py",
    "validate_layer.py", "validate_forward.py", "frozen_owned.py", "library_audit.py",
    "readiness_announcement.py", "validate_readiness.py", "validate_shared.py",
    "validate_timing.py", "validate_matched.py", "validate_scoped.py",
    "validate_scoped_pair.py", "validate_bank_scoped.py", "validate_census.py",
    "validate_tail.py", "validate_duration.py"}
PREPARED = {"tail_layer-input.json", "tail_layer-request.json", "prepared-inputs.json"}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result
    def constant(value):
        raise RuntimeError("nonfinite JSON constant " + value)
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant)


def same(left, right):
    return encoded(left) == encoded(right)


def read(path, cap=16 << 20):
    before = path.lstat()
    require(path.is_absolute() and path.resolve(strict=True) == path
            and stat.S_ISREG(before.st_mode) and before.st_nlink == 1
            and before.st_size <= cap, "bounded ordinary source")
    with path.open("rb") as stream:
        raw = stream.read(cap + 1)
        opened = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_uid, s.st_gid,
                       s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(opened) == stamp(path.lstat())
            and len(raw) == before.st_size, "source changed during read")
    return raw


def relative(name):
    require(type(name) is str and name and not Path(name).is_absolute()
            and Path(name).as_posix() == name
            and all(p not in ("", ".", "..") for p in Path(name).parts),
            "canonical relative name")
    return name


def literals(raw, selected):
    result = {}
    for node in ast.parse(raw).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in selected:
                require(name not in result, "duplicate bound assignment")
                result[name] = ast.literal_eval(node.value)
    require(set(result) == set(selected), "complete literal binding set")
    return result


def names(raw, module):
    found = []
    for node in ast.parse(raw, filename=module).body:
        if isinstance(node, ast.ClassDef):
            found.extend(module + "." + node.name + "." + item.name for item in node.body
                         if isinstance(item, ast.FunctionDef) and item.name.startswith("test_"))
    require(len(found) == len(set(found)), "duplicate test definitions")
    return sorted(found)


def source_namespace(expected):
    files, directories = set(), set()
    wanted_dirs = {str(p.relative_to(QUALIFIED))
                   for n in expected for p in (QUALIFIED / n).parents
                   if QUALIFIED in p.parents}
    for path in QUALIFIED.rglob("*"):
        name = path.relative_to(QUALIFIED).as_posix()
        if Path(name).parts[0] in ("evidence", "tmp"):
            continue
        require(path.resolve(strict=True) == path, "canonical qualified namespace")
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode):
            directories.add(name)
        else:
            require(stat.S_ISREG(mode), "ordinary qualified source")
            files.add(name)
    require(files == set(expected) | {"input-manifest.json"} and directories == wanted_dirs,
            "closed qualified source namespace")


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch("[0-9a-f]{64}", sys.argv[1]),
            "python3 -B stage_layer_native.py COMPLETE_SHA")
    require(INPUT is not None, "reviewed successful retry input binding required")
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == "smci350-rck-g03-b19-03", "exact unprivileged host")
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            "CPU8/9 nice10")
    require(all(os.environ.get(k) == "" for k in
                ("HIP_VISIBLE_DEVICES", "ROCR_VISIBLE_DEVICES", "CUDA_VISIBLE_DEVICES")),
            "hidden GPUs")
    require(E.resolve(strict=True) == E and QUALIFIED.resolve(strict=True) == QUALIFIED
            and not os.path.lexists(ROOT) and not os.path.lexists(TOOLS)
            and shutil.disk_usage(E).free >= 40 << 30, "fresh outputs and initial40GiB floor")
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (cap, cap))
    signal.alarm(180)
    readset = {}

    def observed(path, expected=None, cap=16 << 20):
        raw = read(path, cap)
        actual = dict(path=str(path), **pin(raw))
        if expected is not None:
            require(same(actual, expected), "exact original bytes: " + str(path))
        if str(path) in readset:
            require(same(readset[str(path)], actual), "repeated read drift")
        readset[str(path)] = actual
        return raw

    input_path = QUALIFIED / "input-manifest.json"
    input_raw = observed(input_path, dict(path=str(input_path), **INPUT), 1 << 20)
    spec = parse(input_raw)
    require(spec["schema"] == "ferric-active-pause-native-admission-input-v1"
            and spec["root"] == str(QUALIFIED) and spec["gpu_execution"] is False
            and type(spec["source_files"]) is int and spec["source_files"] == 377
            and type(spec["total_tests"]) is int and spec["total_tests"] == 336
            and spec["original_fixture_files"] == 152 and spec["fixture_copies"] == 2
            and spec["leaf_order"] == list(COUNTS) and set(spec["leaves"]) == set(COUNTS)
            and len(spec["sources"]) == 377, "fixed retry source and test contract")
    source_namespace(spec["sources"])
    bodies = {}
    for name, expected in spec["sources"].items():
        relative(name)
        require(Path(name).parts[0] not in ("evidence", "tmp"), "source-only input")
        bodies[name] = observed(QUALIFIED / name, dict(path=str(QUALIFIED / name), **expected))
    require(sum(map(len, bodies.values())) <= 32 << 20, "bounded qualified source census")
    maps = {n: dict(path=str(QUALIFIED / n), **pin(raw)) for n, raw in sorted(bodies.items())}
    require(pin(bodies["supervisor.py"])["sha256"] == SUPERVISOR_SHA, "qualified supervisor")
    seal_raw = observed(Path(spec["source_seal"]["path"]), spec["source_seal"], 1 << 20)
    require(seal_raw == bodies["active-pause-source-seal.json"], "original and staged source seal")
    seal = parse(seal_raw)
    require(seal["schema"] == "ferric-active-pause-native-admission-source-seal-v1"
            and same(seal["leaves"], spec["leaves"]), "source seal test contract")
    for name, expected in seal["sources"].items():
        target = "run_cpu.py" if name == "layer_native_cpu_template.py" else name
        require(same(pin(bodies[target]), expected), "seal source postimage")
    observed(Path(spec["predecessor_input"]["path"]), spec["predecessor_input"], 1 << 20)
    terminal_path = QUALIFIED / "evidence/complete.json"
    terminal_raw = observed(terminal_path, cap=2 << 20)
    require(pin(terminal_raw)["sha256"] == sys.argv[1], "caller-observed complete receipt")
    terminal = parse(terminal_raw)
    require(terminal["schema"] == "ferric-active-pause-native-admission-cpu-v1"
            and terminal["passed"] is True and terminal["failure"] is None
            and terminal["postcheck_errors"] == [] and terminal["source_unchanged"] is True
            and same(terminal["input_manifest"], readset[str(input_path)])
            and same(terminal["sources_before"], maps) and same(terminal["sources_after"], maps)
            and same(terminal["controller"], maps["run_cpu.py"])
            and same(terminal["supervisor"], maps["supervisor.py"]), "successful original source closure")
    for key in ("gpu_execution", "model_shard_reads", "native_parent_execution", "model_execution",
                "numerical_acceptance", "full_model_acceptance", "performance_claim", "production_authority",
                "synthetic_data_tests_only"):
        require(terminal[key] is False, "no execution or acceptance authority: " + key)
    require(terminal["synthetic_and_actual_artifact_backed_tests"] is True
            and terminal["actual_cpu_artifact_reads"] is True
            and terminal["original_fixture_files"] == 152, "honest artifact-backed CPU scope")
    limits = dict(whole_seconds=1000, leaf_seconds={n: 120 for n in COUNTS},
        cleanup_reserve_seconds=50, address_space_bytes=512 << 20, file_bytes=16 << 20,
        stream_bytes=4 << 20, temporary_bytes=64 << 20, affinity=[8, 9], nice=10,
        cargo_jobs=2, initial_free_bytes=40 << 30, live_free_bytes=38 << 30)
    require(same(terminal["limits"], limits) and type(terminal["elapsed_seconds"]) in (int, float)
            and 0 <= terminal["elapsed_seconds"] <= 1000, "original bounded qualification")
    require([p["label"] for p in terminal["phases"]] == list(COUNTS)
            and set(terminal["leaf_tests"]) == set(COUNTS), "eight complete ordered leaves")
    expected_raw = {"sources-before.json", "sources-after.json"} | {
        label + suffix for label in COUNTS
        for suffix in (".command.json", ".started.json", ".result.json", ".stdout", ".stderr")}
    require(set(terminal["raw"]) == expected_raw
            and {p.name for p in (QUALIFIED / "evidence").iterdir()} == expected_raw | {"complete.json"},
            "42 original raw bodies and only the successful terminal")
    raw = {}
    for name in sorted(expected_raw):
        path = QUALIFIED / "evidence" / name
        require(terminal["raw"][name]["path"] == str(path), "raw path ownership")
        raw[name] = observed(path, terminal["raw"][name], 4 << 20)
    require(same(parse(raw["sources-before.json"]), maps)
            and same(parse(raw["sources-after.json"]), maps), "original before/after source maps")
    environment = dict(PATH="/usr/bin:/bin", HOME="/home/harmenon", LANG="C.UTF-8", LC_ALL="C.UTF-8",
        TZ="UTC", PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1", PYTHONNOUSERSITE="1",
        TMPDIR=str(QUALIFIED / "tmp"), HIP_VISIBLE_DEVICES="", ROCR_VISIBLE_DEVICES="",
        CUDA_VISIBLE_DEVICES="", CARGO_BUILD_JOBS="2", OMP_NUM_THREADS="1",
        OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1")
    require(same(terminal["environment"], environment) and set(terminal["tool_pins"]) == {"python", "prlimit"},
            "original hidden-GPU CPU environment")
    for p in terminal["tool_pins"].values():
        observed(Path(p["path"]), p)
    python, prlimit = (terminal["tool_pins"][k]["path"] for k in ("python", "prlimit"))
    require(python == str(Path("/usr/bin/python3").resolve(strict=True))
            and prlimit == str(Path("/usr/bin/prlimit").resolve(strict=True)), "original CPU tools")
    all_names = []
    for phase in terminal["phases"]:
        label, leaf = phase["label"], spec["leaves"][phase["label"]]
        directory = "legacy" if label == "legacy-tests" else "."
        require(leaf["directory"] == directory and leaf["count"] == COUNTS[label], "exact leaf count and root")
        prefix = "legacy/" if directory == "legacy" else ""
        actual_names = sorted(n for module in leaf["modules"]
                              for n in names(bodies[prefix + module + ".py"], module))
        require(actual_names == leaf["names"] and len(actual_names) == len(set(actual_names)) == COUNTS[label],
                "actual source AST names")
        all_names.extend(actual_names)
        require(type(phase["exit_code"]) is int and phase["exit_code"] == 0
                and all(phase[k] is True for k in ("natural_exit", "reaped", "process_group_absent"))
                and all(phase[k] is False for k in ("timed_out", "forced_cleanup"))
                and phase["observed_signals"] == phase["adopted_reaped"] == []
                and phase["exception"] is None and phase["storage_failure"] is None
                and type(phase["pid"]) is int and phase["pid"] > 1 and phase["pgid"] == phase["pid"]
                and type(phase["elapsed_ns"]) is int and 0 <= phase["elapsed_ns"] <= 120_000_000_000,
                "naturally retired original CPU leaf")
        cwd = str(QUALIFIED / directory)
        script = ("import sys,unittest;sys.path.insert(0," + repr(cwd) + ");"
                  "suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(n) for n in "
                  + repr(leaf["modules"]) + ");"
                  "result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())")
        argv = [prlimit, "--as=536870912", "--cpu=120", "--fsize=16777216", "--core=0", "--",
                python, "-I", "-B", "-c", script]
        require(phase["argv"] == argv, "exact qualified unittest command")
        require(same(parse(raw[label + ".command.json"]),
                     dict(argv=argv, cwd=cwd, env=environment, wall_timeout_seconds=120))
                and same(parse(raw[label + ".started.json"]),
                         dict(argv=argv, pid=phase["pid"], pgid=phase["pgid"]))
                and same(parse(raw[label + ".result.json"]), phase), "original command/start/result joins")
        for field, suffix in (("command", ".command.json"), ("stdout", ".stdout"), ("stderr", ".stderr")):
            require(same(phase[field], terminal["raw"][label + suffix]), "phase raw pin joins")
        require(raw[label + ".stdout"] == b"", "quiet unittest stdout")
        stderr = raw[label + ".stderr"].decode("utf-8")
        pattern = r"^(test_[A-Za-z0-9_]+) \(([A-Za-z0-9_]+\.[A-Za-z0-9_]+)\.\1\) \.\.\. ok$"
        found = re.findall(pattern, stderr, re.M)
        actual = sorted(prefix + "." + method for method, prefix in found)
        require(actual == actual_names, "all original named tests passed once")
        remainder = re.sub(pattern, "", stderr, flags=re.M)
        tail = [line for line in remainder.splitlines() if line]
        require(len(tail) == 3 and tail[0] == "-" * 70
                and re.fullmatch("Ran " + str(COUNTS[label]) + r" tests in [0-9]+\.[0-9]+s", tail[1])
                and tail[2] == "OK", "clean exact unittest summary")
        require(same(terminal["leaf_tests"][label],
                     dict(names=actual_names, passed=COUNTS[label], failed=0, errors=0, skipped=0)),
                "original leaf census")
    require(len(all_names) == len(set(all_names)) == 336
            and same(terminal["tests"], dict(names=sorted(all_names), passed=336, failed=0, errors=0, skipped=0)),
            "336 tests, including unchanged227, and no GPU result")
    require([terminal[k] for k in ("inherited_tests", "forward_cpu_tests", "forward_retention_tests",
            "forward_preparation_tests", "layer_cpu_tests", "layer_retention_tests", "layer_preparation_tests", "usage_tests")]
            == list(COUNTS.values()), "original named category counts")
    retainer_raw = bodies["layer_retention_tool.py"]
    require(same(pin(retainer_raw), RETAINER), "exact qualified retainer")
    contract = literals(retainer_raw, ("ROOT_PINS", "ADMISSION", "CHECKER"))
    require(set(contract["ROOT_PINS"]) == PRODUCTION | PREPARED
            and len(PRODUCTION) == 18 and all(contract["ROOT_PINS"][n] is None for n in PREPARED)
            and contract["CHECKER"] is None
            and set(contract["ADMISSION"]) == {"runtime_cpu", "runtime_sources", "worker_cpu", "worker_sources",
                                               "parent_cpu", "parent_sources", "worker", "parent"}
            and all(v is None for v in contract["ADMISSION"].values()), "unbound closed native retainer contract")
    for name in PRODUCTION:
        expected = contract["ROOT_PINS"][name]
        require(expected is None or same(expected, pin(bodies[name])), "retainer fixed source")
    runner = literals(bodies["run_layer_model_gpu.py"],
                      ("PLAN_SHA", "CPU_ADMISSION_SHA", "LAYER_SHA", "FORWARD_SHA"))
    preparer = literals(bodies["prepare_layer_model_inputs.py"], ("CPU_ADMISSION_SHA",))
    require(runner["PLAN_SHA"] == {"tail_layer": None}
            and runner["CPU_ADMISSION_SHA"] == preparer["CPU_ADMISSION_SHA"] == pin(bodies["layer_cpu_admission.py"])["sha256"]
            and runner["LAYER_SHA"] == pin(bodies["validate_layer.py"])["sha256"]
            and runner["FORWARD_SHA"] == pin(bodies["validate_forward.py"])["sha256"],
            "qualified unbound plan and exact helper bindings")
    self_path = Path(__file__).absolute()
    require(self_path == D / "stage_layer_native.py", "fixed data stager source")
    observed(self_path)

    def rehash():
        for path, expected in readset.items():
            require(same(dict(path=path, **pin(read(Path(path)))), expected), "qualified original drift")
        source_namespace(spec["sources"])

    def write(path, value):
        require(shutil.disk_usage(E).free >= 38 << 30, "live38GiB write floor")
        with path.open("xb") as stream:
            stream.write(value)
            stream.flush()
            os.fsync(stream.fileno())
        require(read(path) == value and shutil.disk_usage(E).free >= 38 << 30, "exact exclusive copy and final floor")

    rehash()
    ROOT.mkdir(mode=0o700)
    TOOLS.mkdir(mode=0o700)
    copies = {}
    for name in sorted(PRODUCTION):
        write(ROOT / name, bodies[name])
        copies[name] = dict(source=maps[name], destination=dict(path=str(ROOT / name), **pin(bodies[name])))
    tool_path = TOOLS / "layer_retention_tool.py"
    write(tool_path, retainer_raw)
    rehash()
    require({p.name for p in ROOT.iterdir()} == PRODUCTION
            and {p.name for p in TOOLS.iterdir()} == {"layer_retention_tool.py"},
            "only18 native scripts and external retainer")
    for name in PRODUCTION:
        require(pin(read(ROOT / name)) == pin(bodies[name]), "final deployed source")
    receipt = dict(schema="ferric-tail-layer-active-pause-native-deployment-v1", root=str(ROOT), tools_root=str(TOOLS),
        qualification=readset[str(terminal_path)], input_manifest=readset[str(input_path)],
        source_seal=spec["source_seal"], qualified_sources_before=terminal["sources_before"],
        qualified_sources_after=terminal["sources_after"], qualified_raw=terminal["raw"],
        tests=terminal["tests"], phases=terminal["phases"], readset=readset, copies=copies,
        retainer=dict(source=maps["layer_retention_tool.py"],
                      destination=dict(path=str(tool_path), **pin(retainer_raw))),
        source_files=377, production_scripts=18, retainer_scripts=1, prepared_json_files=0,
        prepared=False, plan_bound=False, native_execution=False, gpu_execution=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False,
        production_authority=False, original_process_retirement_only=True)
    receipt_raw = encoded(receipt)
    require(len(receipt_raw) <= 1 << 20, "bounded external deployment receipt")
    receipt_path = TOOLS / "deployment.json"
    write(receipt_path, receipt_raw)
    signal.alarm(0)
    print(json.dumps(dict(root=str(ROOT), tools_root=str(TOOLS),
        receipt=dict(path=str(receipt_path), **pin(receipt_raw)), scripts=18,
        prepared=False, native_execution=False), sort_keys=True))


if __name__ == "__main__":
    main()
