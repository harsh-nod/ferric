"""Two GPU-hidden checker leaves; original two-record tests remain unchanged."""
import ast
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "evidence"
SUPERVISOR_SHA = "8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc"
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
WHOLE_SECONDS, LEAF_SECONDS, CLEANUP_SECONDS = 300, 120, 50

def require(ok, why):
    if not ok:
        raise RuntimeError(why)

def read(path, cap=1 << 20):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= cap, "ordinary bounded input")
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(path.lstat()) and len(raw) == before.st_size, "input changed")
    return raw

def compact(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())

def interrupted(number, _frame):
    raise RuntimeError("controller signal " + str(number))

def test_names(path):
    result = []
    for node in ast.parse(read(path)).body:
        if isinstance(node, ast.ClassDef):
            result.extend(path.stem + "." + node.name + "." + item.name for item in node.body
                          if isinstance(item, ast.FunctionDef) and item.name.startswith("test_"))
    return sorted(result)

def main():
    start = time.monotonic()
    deadline = start + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch("[0-9a-f]{64}", sys.argv[1]), "python3 -B run_cpu.py INPUT_SHA")
    raw = read(ROOT / "input-manifest.json")
    require(hashlib.sha256(raw).hexdigest() == sys.argv[1], "bound input manifest")
    spec = json.loads(raw)
    require(spec["schema"] == "ferric-forward-checker-input-v1" and str(ROOT) == spec["root"]
            and ROOT.parent == Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
            and re.fullmatch("guarded-mlp-readiness40-tail-forward-duration-checker-cpu-v228-v[0-9]+", ROOT.name)
            and ROOT.resolve(strict=True) == ROOT and not os.path.lexists(OUT), "fresh exact root")
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == "smci350-rck-g03-b19-03", "exact unprivileged host")
    require({p.name for p in ROOT.iterdir()} == {"input-manifest.json", *spec["sources"]},
            "closed input namespace")
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), "unexpected nice")
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 16 << 20),
                      (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, "subreaper setup")
    supervisor_raw = read(ROOT / "supervisor.py")
    require(hashlib.sha256(supervisor_raw).hexdigest() == SUPERVISOR_SHA, "frozen supervisor")
    h = types.ModuleType("forward_checker_owned")
    h.__file__ = str(ROOT / "supervisor.py")
    exec(compile(supervisor_raw, h.__file__, "exec"), h.__dict__)
    h.OUT = OUT
    h.AS_LIMIT = 512 << 20
    h.FILE_LIMIT = 16 << 20
    h.STREAM_LIMIT = 4 << 20
    h.CPU_LIMIT = 120
    h.CACHE_LIMIT = 4 << 20
    h.CLEANUP_RESERVE = CLEANUP_SECONDS
    require(h.shutil.disk_usage(ROOT).free >= h.START_FREE, "initial40GiB free floor")
    def sources():
        return {name: h.pin(ROOT / name) for name in sorted(spec["sources"])}
    h.sources = sources
    before = sources()
    require(all({k: before[n][k] for k in ("bytes", "sha256")} == p
                for n, p in spec["sources"].items()), "exact staged source bytes")
    require(set(spec["leaves"]) == {"legacy", "forward"}
            and len(spec["leaves"]["legacy"]["names"]) == 126
            and len(spec["leaves"]["forward"]["names"]) > 0, "old and new closed test scopes")
    for leaf in spec["leaves"].values():
        names = sorted(name for module in leaf["modules"] for name in test_names(ROOT / (module + ".py")))
        require(names == leaf["names"] and len(names) == len(set(names)), "closed AST test roster")
    python, prlimit = Path("/usr/bin/python3").resolve(strict=True), Path("/usr/bin/prlimit").resolve(strict=True)
    tools = dict(python=h.pin(python), prlimit=h.pin(prlimit))
    environment = dict(PATH="/usr/bin:/bin", HOME="/home/harmenon", LANG="C.UTF-8", LC_ALL="C.UTF-8", TZ="UTC",
        PYTHONPATH="", PYTHONDONTWRITEBYTECODE="1", PYTHONNOUSERSITE="1",
        HIP_VISIBLE_DEVICES="", ROCR_VISIBLE_DEVICES="", CUDA_VISIBLE_DEVICES="",
        OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1")
    OUT.mkdir(mode=0o700)
    phases, errors, results = [], [], {}
    failure = None
    after = None
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for number in handlers:
        signal.signal(number, interrupted)
    def arm_deadline():
        remaining = deadline - time.monotonic()
        require(remaining > 0, "whole checker deadline")
        signal.setitimer(signal.ITIMER_REAL, remaining)
    arm_deadline()
    try:
        h.save("sources-before.json", before)
        for label in ("legacy", "forward"):
            leaf = spec["leaves"][label]
            script = ("import sys,unittest;sys.path.insert(0," + repr(str(ROOT)) + ");"
                      "suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(n) for n in "
                      + repr(leaf["modules"]) + ");"
                      "result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())")
            h.run(label + "-tests", [str(python), "-I", "-B", "-c", script], environment,
                  phases, deadline, before, seconds=LEAF_SECONDS, cwd=ROOT)
            require(read(OUT / (label + "-tests.stdout"), 4 << 20) == b"", "unexpected test stdout")
            stderr = read(OUT / (label + "-tests.stderr"), 4 << 20).decode("utf-8")
            expression = r"^(test_[A-Za-z0-9_]+) \(([A-Za-z0-9_]+\.[A-Za-z0-9_]+)\.\1\) \.\.\. ok$"
            found = [prefix + "." + name for name, prefix in re.findall(expression, stderr, re.M)]
            require(sorted(found) == leaf["names"] and len(found) == len(set(found)), "exact named successes")
            tail = "\n".join(line for line in re.sub(expression, "", stderr, flags=re.M).splitlines() if line)
            require(re.fullmatch(r"-{70}\nRan " + str(len(found)) + r" tests in [0-9]+\.[0-9]+s\nOK", tail),
                    "exact unittest summary without skips")
            results[label] = dict(names=sorted(found), passed=len(found), failed=0, errors=0, skipped=0)
    except BaseException as error:
        failure = type(error).__name__ + ": " + str(error)
    finally:
        for number in handlers:
            signal.signal(number, interrupted if number == signal.SIGALRM else signal.SIG_IGN)
        arm_deadline()
        try:
            after = sources()
            require(after == before, "source drift")
            h.save("sources-after.json", after)
            require(read(ROOT / "input-manifest.json") == raw, "input manifest drift")
            require(Path("/usr/bin/python3").resolve(strict=True) == python
                    and Path("/usr/bin/prlimit").resolve(strict=True) == prlimit
                    and h.pin(python) == tools["python"] and h.pin(prlimit) == tools["prlimit"], "tool drift")
            require({p.name for p in ROOT.iterdir()} == {"input-manifest.json", "evidence", *spec["sources"]},
                    "unexpected bytecode or root writes")
            require(len(phases) == 2 and all(row["reaped"] and row["process_group_absent"] for row in phases),
                    "both checker children retired")
            require(time.monotonic() < deadline, "whole checker bound")
        except BaseException as error:
            errors.append(type(error).__name__ + ": " + str(error))
    raw_pins = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    try:
        for label, row in zip(("legacy", "forward"), phases):
            for key in ("command", "stdout", "stderr"):
                require(raw_pins[Path(row[key]["path"]).name] == row[key], "final raw pin join")
            require(json.loads(read(OUT / (label + "-tests.result.json"))) == row, "original result join")
            started = json.loads(read(OUT / (label + "-tests.started.json")))
            require(started == dict(pid=row["pid"], pgid=row["pgid"], argv=row["argv"]), "child identity join")
        allowed = {"sources-before.json", "sources-after.json"}
        allowed |= {label + "-tests." + suffix for label in ("legacy", "forward")
                    for suffix in ("command.json", "started.json", "result.json", "stdout", "stderr")}
        require(set(raw_pins) <= allowed, "closed original evidence prefix")
    except BaseException as error:
        errors.append("raw reconciliation: " + repr(error))
    failure = failure or ("postcheck failed" if errors else None)
    names = sorted(n for row in results.values() for n in row["names"])
    census = (dict(names=names, passed=len(names), failed=0, errors=0, skipped=0)
              if set(results) == {"legacy", "forward"} else None)
    value = dict(schema="ferric-guarded-mlp-readiness40-tail-forward-duration-checker-cpu-v1",
        passed=failure is None and set(results) == {"legacy", "forward"}, failure=failure,
        postcheck_errors=errors, input_manifest=dict(path=str(ROOT / "input-manifest.json"), **compact(raw)),
        controller=before["run_cpu.py"], supervisor=before["supervisor.py"],
        sources_before=before, sources_after=after, source_unchanged=after == before,
        tool_pins=tools, environment=environment, phases=phases, tests=census, leaf_tests=results, raw=raw_pins,
        limits=dict(whole_seconds=WHOLE_SECONDS, test_seconds=LEAF_SECONDS, cleanup_reserve_seconds=CLEANUP_SECONDS,
                    address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10),
        elapsed_seconds=time.monotonic() - start, synthetic_data_tests_only=True,
        gpu_execution=False, native_parent_execution=False, model_execution=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False, production_authority=False)
    arm_deadline()
    h.save("complete.json" if value["passed"] else "failed.json", value)
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items():
        signal.signal(number, handler)
    print(json.dumps({k: value[k] for k in ("passed", "failure", "postcheck_errors", "tests")}, sort_keys=True))
    return 0 if value["passed"] else 1

if __name__ == "__main__":
    raise SystemExit(main())
