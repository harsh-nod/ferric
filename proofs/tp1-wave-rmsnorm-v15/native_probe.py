#!/usr/bin/env python3
"""Explicit finite V15 device checks; run under the shared-host lifecycle wrapper."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
from types import ModuleType

FIXTURE_SHA = "35368711baaaac27ce6612c120875a8afb33540b38db0daa7135e05863c02d90"
HELPER_SHA = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
TIMING_SCOPE = "worker dispatch latency retained but excluded from all comparisons"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def exact(actual, expected, message):
    require(type(actual) is type(expected), message)
    if type(expected) is dict:
        require(actual.keys() == expected.keys(), message)
        for key in expected:
            exact(actual[key], expected[key], message)
    elif type(expected) is list:
        require(len(actual) == len(expected), message)
        for left, right in zip(actual, expected, strict=True):
            exact(left, right, message)
    else:
        require(actual == expected, message)


def positive(value, maximum=(1 << 64) - 1):
    require(type(value) is int and 0 < value <= maximum, "positive bounded integer")
    return value


def load_fixture(path):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 65536, "fixture extent")
        with os.fdopen(os.dup(fd), "rb") as source:
            raw = source.read(65537)
        after = os.fstat(fd)
        fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
        require(all(getattr(before, key) == getattr(after, key) for key in fields)
                and len(raw) == before.st_size and hashlib.sha256(raw).hexdigest() == FIXTURE_SHA,
                "fixture identity drift")
    finally:
        os.close(fd)
    fixture = ModuleType("ferric_v15_analytical_fixtures")
    fixture.__file__ = str(path)
    exec(compile(raw, str(path), "exec", dont_inherit=True), fixture.__dict__)
    require(fixture.HELPER_SHA256 == HELPER_SHA, "fixture/core binding")
    return fixture


def validate_metadata(metadata, case, artifact_sha, core):
    require(type(metadata) is dict, "metadata object")
    arguments = metadata.get("explicit_arguments")
    require(type(arguments) is list and len(arguments) == 14, "five slices/four scalars")
    expected = []
    for index, buffer in enumerate(case["buffers"]):
        offset = index * 16
        pointer = arguments[index * 2]
        require(type(pointer) is dict and set(pointer) ==
                {"offset", "bytes", "global_buffer", "access", "pointee_alignment"}
                and core.pointer_matches_source(pointer, offset, buffer), "source pointer contract")
        require(pointer["access"] is None or type(pointer["access"]) is str,
                "pointer access type")
        require(pointer["pointee_alignment"] is None
                or type(pointer["pointee_alignment"]) is int, "pointer alignment type")
        expected.extend([
            dict(offset=offset, bytes=8, global_buffer=True, access=pointer["access"],
                 pointee_alignment=pointer["pointee_alignment"]),
            dict(offset=offset + 8, bytes=8, global_buffer=False, access=None, pointee_alignment=None),
        ])
    expected.extend(dict(offset=offset, bytes=4, global_buffer=False, access=None,
                         pointee_alignment=None) for offset in range(80, 96, 4))
    exact(metadata, dict(symbol=case["symbol"], object_sha256=list(bytes.fromhex(artifact_sha)),
        wavefront_size=64, private_segment_bytes=0, group_segment_bytes=0, kernarg_alignment=8,
        kernarg_bytes=352, implicit_argument_offset=96, implicit_argument_bytes=256,
        explicit_arguments=expected), "exact V15 ABI and resources")


def validate_result(result, case, artifact_sha, core):
    require(type(result) is dict and set(result) ==
            {"name", "symbol", "elapsed_ns", "metadata", "checks"}, "closed case result")
    exact(result["name"], case["name"], "ordered case name")
    exact(result["symbol"], case["symbol"], "V15 root")
    positive(result["elapsed_ns"])
    checks = [dict(name=buffer["name"], access=buffer["access"], bytes=len(buffer["expected"]),
                   sha256=hashlib.sha256(buffer["expected"]).hexdigest(),
                   guard_bytes_each_side=64, guards_unchanged=True) for buffer in case["buffers"]]
    exact(result["checks"], checks, "five full active buffers and ten guards")
    validate_metadata(result["metadata"], case, artifact_sha, core)


def validate_report(report, fixtures, core, artifact_sha, worker_sha, device, source_sha):
    require(type(report) is dict, "report object")
    rows = report.get("results")
    require(type(rows) is list and len(rows) == 8, "complete eight-case roster")
    pid = positive(report.get("worker_pid"), (1 << 31) - 1)
    start = positive(report.get("worker_start_ticks"))
    exact(report, dict(schema="FerricTp1WaveRmsnormProbeV15", authority="none", benchmark=False,
        performance_qualified=False, model_inference=False, model_parity_qualified=False,
        checks_pass=True, source_sha256=source_sha, fixture_sha256=FIXTURE_SHA, helper_sha256=HELPER_SHA,
        worker_sha256=worker_sha, artifact_sha256=artifact_sha, runtime_operational=True,
        device_unique_id=device, worker_pid=pid, worker_start_ticks=start, clean_teardown=True,
        timing_scope=TIMING_SCOPE, closed_roots=[fixtures.ROOT], active_inputs_finite=True,
        results=rows), "exact report identity and nonclaims")
    for spec, result in zip(fixtures.specifications(), rows, strict=True):
        case = fixtures.make_case(core, spec)
        fixtures.validate_case(core, case)
        validate_result(result, case, artifact_sha, core)
    return pid


def run_cases(worker, core, fixtures, artifact, artifact_sha):
    try:
        _, payload = worker.command({"op": "configure_performance", "cache_kernel_admission": False,
                                     "operational_currentness": True, "profile": False},
                                    expected="performance_configured")
        require(not payload, "configuration payload")
        results = []
        for spec in fixtures.specifications():
            case = fixtures.make_case(core, spec)
            fixtures.validate_case(core, case)
            result = core.probe(worker, case, artifact, artifact_sha)
            validate_result(result, case, artifact_sha, core)
            results.append(result)
            print(case["name"] + ": PASS", flush=True)
        worker.finish()
        return results
    except BaseException:
        worker.abort()
        raise


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--run", action="store_true", required=True)
    parser.add_argument("--operational", action="store_true", required=True)
    for name in ("fixtures", "helper", "worker", "artifact", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--worker-sha256", required=True)
    parser.add_argument("--artifact-sha256", required=True)
    parser.add_argument("--device-unique-id", type=int, required=True)
    args = parser.parse_args(argv)
    positive(args.device_unique_id)
    require(args.output.is_absolute(), "absolute fresh output path")
    fixtures = load_fixture(args.fixtures)
    core = fixtures.load_helper(args.helper)
    fixtures.self_test(core)
    args.output.mkdir(mode=0o700)
    source_sha = core.digest(Path(__file__).read_bytes())
    worker_fd, worker_stat, _ = core.held_file(args.worker, args.worker_sha256, 512 * 1024**2, 62)
    artifact_fd = None
    try:
        artifact_fd, artifact_stat, artifact = core.held_file(
            args.artifact, args.artifact_sha256, 64 * 1024**2, 224)
        worker = core.Worker(worker_fd, worker_stat, args.device_unique_id, args.output)
        results = run_cases(worker, core, fixtures, artifact, args.artifact_sha256)
        require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat)
                and core.digest(Path(__file__).read_bytes()) == source_sha
                and core.digest(args.fixtures.read_bytes()) == FIXTURE_SHA
                and core.digest(args.helper.read_bytes()) == HELPER_SHA, "retained input identity drift")
        report = dict(schema="FerricTp1WaveRmsnormProbeV15", authority="none", benchmark=False,
            performance_qualified=False, model_inference=False, model_parity_qualified=False,
            checks_pass=True, source_sha256=source_sha, fixture_sha256=FIXTURE_SHA, helper_sha256=HELPER_SHA,
            worker_sha256=args.worker_sha256, artifact_sha256=args.artifact_sha256,
            runtime_operational=True, device_unique_id=args.device_unique_id, worker_pid=worker.process.pid,
            worker_start_ticks=worker.start, clean_teardown=True, timing_scope=TIMING_SCOPE,
            closed_roots=[fixtures.ROOT], active_inputs_finite=True, results=results)
        validate_report(report, fixtures, core, args.artifact_sha256, args.worker_sha256,
                        args.device_unique_id, source_sha)
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2, allow_nan=False)
            output.write("\n")
    finally:
        if artifact_fd is not None:
            os.close(artifact_fd)
        os.close(worker_fd)


if __name__ == "__main__":
    main()
