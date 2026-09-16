#!/usr/bin/env python3
"""Check gfx950 finite-decode engineering observations, not release authority."""

from __future__ import annotations

import argparse
from fractions import Fraction
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import random
import re
import stat
import sys
from typing import Any


FORMAT = "ferric.gfx950-megakernel.measurements.v1"
REPORT_FORMAT = "ferric.gfx950-megakernel.report.v1"
NUMERICAL_FORMAT = "ferric.gfx950-megakernel.numerical.v1"
NONCLAIM = (
    "Checked engineering observations only. File hashes authenticate supplied "
    "bytes, not their truth or physical collection. This is not production, "
    "numerical-policy, machine-refinement, or SoTA qualification."
)
SCOPES = {"device-smoke", "single-layer", "full-decode"}
ROLES = {"ferric-command-batch", "megakernel", "ablation", "vllm", "sglang"}
PROVENANCE = {
    "ferric_source", "fe2o3_source", "model", "tokenizer", "weights",
    "numerical_policy", "environment", "workload", "tuning_budget",
}
WARMUPS = 10
SAMPLES = 30
BOOTSTRAP_ROUNDS = 2048
BOOTSTRAP_SEED = 0xF3_2026_0915
MAX_DOCUMENT_BYTES = 8 * 1024 * 1024
MAX_RUNS = 4096
MAX_VARIANTS = 16
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
NAME = re.compile(r"[a-z][a-z0-9-]{0,63}\Z")


class Rejected(ValueError):
    """The submitted observation does not satisfy the engineering contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise Rejected(message)


def obj(value: Any, fields: set[str], name: str) -> dict[str, Any]:
    require(isinstance(value, dict) and set(value) == fields, f"{name}: field roster")
    return value


def integer(value: Any, name: str, minimum: int = 0, maximum: int = 2**63 - 1) -> int:
    require(type(value) is int and minimum <= value <= maximum, f"{name}: integer range")
    return value


def digest(value: Any, name: str) -> str:
    require(isinstance(value, str) and SHA256.fullmatch(value) is not None, f"{name}: SHA-256")
    return value


def sequence(value: Any, name: str, maximum: int, minimum: int = 0) -> list[Any]:
    require(isinstance(value, list) and minimum <= len(value) <= maximum, f"{name}: list extent")
    return value


def unique_names(value: Any, name: str) -> list[str]:
    values = sequence(value, name, 64)
    require(all(isinstance(v, str) and NAME.fullmatch(v) for v in values), f"{name}: names")
    require(len(values) == len(set(values)), f"{name}: duplicates")
    return values


def canonical(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=True, allow_nan=False) + "\n").encode("ascii")


def reject_number(value: str) -> None:
    raise Rejected(f"non-integer JSON number: {value}")


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse(raw: bytes, name: str) -> Any:
    require(0 < len(raw) <= MAX_DOCUMENT_BYTES, f"{name}: byte extent")
    try:
        value = json.loads(raw, object_pairs_hook=unique_object,
                           parse_float=reject_number, parse_constant=reject_number)
        require(canonical(value) == raw, f"{name}: canonical ASCII JSON required")
    except (UnicodeError, json.JSONDecodeError, RecursionError, ValueError) as error:
        if isinstance(error, Rejected):
            raise
        raise Rejected(f"{name}: invalid JSON") from error
    return value


def stable_file(root: Path, relative: str, maximum: int) -> bytes:
    """Use descriptor-relative opens so a replaced parent cannot redirect a pin."""
    path = PurePosixPath(relative)
    require(relative == path.as_posix() and not path.is_absolute()
            and bool(path.parts) and all(p not in {".", ".."} for p in path.parts),
            "artifact path: canonical relative path required")
    directory = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for component in path.parts[:-1]:
            child = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                            dir_fd=directory)
            os.close(directory)
            directory = child
        descriptor = os.open(path.parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                             dir_fd=directory)
        try:
            before = os.fstat(descriptor)
            require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1,
                    "artifact: single-link regular file required")
            require(0 < before.st_size <= maximum, "artifact: byte extent")
            chunks: list[bytes] = []
            remaining = before.st_size
            while remaining:
                chunk = os.read(descriptor, min(remaining, 1024 * 1024))
                require(bool(chunk), "artifact truncated during read")
                chunks.append(chunk)
                remaining -= len(chunk)
            require(os.read(descriptor, 1) == b"", "artifact grew during read")
            after = os.fstat(descriptor)
            binding = os.stat(path.parts[-1], dir_fd=directory, follow_symlinks=False)
            fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns", "st_nlink")
            require(all(getattr(before, k) == getattr(after, k) == getattr(binding, k)
                        for k in fields), "artifact changed during read")
            return b"".join(chunks)
        finally:
            os.close(descriptor)
    finally:
        os.close(directory)


class Pins:
    def __init__(self, root: Path):
        self.root = root
        self.cache: dict[str, tuple[dict[str, Any], bytes]] = {}

    def read(self, value: Any, name: str) -> bytes:
        pin = obj(value, {"path", "sha256", "size_bytes"}, name)
        require(isinstance(pin["path"], str), f"{name}: path")
        digest(pin["sha256"], name)
        integer(pin["size_bytes"], name, 1, MAX_DOCUMENT_BYTES)
        if pin["path"] in self.cache:
            prior, raw = self.cache[pin["path"]]
            require(prior == pin, f"{name}: conflicting file pins")
            return raw
        raw = stable_file(self.root, pin["path"], MAX_DOCUMENT_BYTES)
        require(len(raw) == pin["size_bytes"], f"{name}: size mismatch")
        require(hashlib.sha256(raw).hexdigest() == pin["sha256"], f"{name}: digest mismatch")
        self.cache[pin["path"]] = (pin.copy(), raw)
        return raw


def median(values: list[int | Fraction]) -> Fraction:
    ordered = sorted(values)
    count = len(ordered)
    require(count > 0, "median: empty sample")
    return (Fraction(ordered[(count - 1) // 2]) + Fraction(ordered[count // 2])) / 2


def rational(value: int | Fraction) -> dict[str, int]:
    fraction = Fraction(value)
    return {"numerator": fraction.numerator, "denominator": fraction.denominator}


def spread(values: list[int]) -> Fraction:
    return Fraction(max(values) - min(values), 1) / median(values)


def spread_ppm(values: list[int]) -> int:
    return math.floor(spread(values) * 1_000_000)


def paired_interval(control: list[int], candidate: list[int]) -> tuple[Fraction, Fraction]:
    require(len(control) == len(candidate) and len(control) >= SAMPLES, "bootstrap: pairs")
    generator = random.Random(BOOTSTRAP_SEED)
    ratios: list[Fraction] = []
    for _ in range(BOOTSTRAP_ROUNDS):
        indices = [generator.randrange(len(control)) for _ in control]
        ratios.append(median([control[i] for i in indices]) / median([candidate[i] for i in indices]))
    ratios.sort()
    return ratios[(BOOTSTRAP_ROUNDS * 25) // 1000], ratios[(BOOTSTRAP_ROUNDS * 975) // 1000]


def comparison(control: list[int], candidate: list[int]) -> dict[str, Any]:
    low, high = paired_interval(control, candidate)
    ratio = median(control) / median(candidate)
    return {"median_speedup": rational(ratio), "paired_95_ci": [rational(low), rational(high)],
            "nonregression_095": low >= Fraction(95, 100),
            "engineering_faster_threshold_105": low > Fraction(105, 100)}


def numerical(value: Any, variant: dict[str, Any], document: dict[str, Any], pins: Pins) -> None:
    report = obj(parse(pins.read(value, "numerical report"), "numerical report"), {
        "format", "target", "scope", "passed", "comparison", "tests", "failed_tests",
        "executable_sha256", "workload_sha256", "model_sha256", "weights_sha256",
        "numerical_policy_sha256", "reference_implementation",
    }, "numerical report")
    require(report["format"] == NUMERICAL_FORMAT, "numerical format")
    require(report["target"] == document["target"] and report["scope"] == document["scope"],
            "numerical target/scope mismatch")
    require(report["passed"] is True and report["failed_tests"] == [], "numerical comparison failed")
    require(report["comparison"] == "independent-reference", "independent comparison required")
    integer(report["tests"], "numerical test count", 1)
    require(report["executable_sha256"] == variant["executable"]["sha256"], "numerical executable mismatch")
    for key in ("workload", "model", "weights", "numerical_policy"):
        require(report[key + "_sha256"] == document["provenance"][key]["sha256"],
                f"numerical {key} mismatch")
    pins.read(report["reference_implementation"], "independent reference implementation")


def validate(document: Any, root: Path) -> dict[str, Any]:
    document = obj(document, {"format", "scope", "target", "authority", "qualification",
                            "public_faster_claim", "case", "provenance", "variants", "runs",
                            "ablations", "interactions", "bound"}, "measurements")
    require(document["format"] == FORMAT, "measurement format")
    require(isinstance(document["scope"], str) and document["scope"] in SCOPES
            and document["target"] == "gfx950", "target/scope")
    require(document["authority"] == "engineering-observation-only"
            and document["qualification"] is False and document["public_faster_claim"] is False,
            "unsupported authority or qualification claim")
    case = obj(document["case"], {"model", "batch", "context", "precision", "accumulation",
                                   "metric", "output_tokens"}, "case")
    require(isinstance(case["model"], str) and case["model"] in {"Qwen/Qwen3-0.6B", "Qwen/Qwen3-8B"}, "model scope")
    require(type(case["batch"]) is int and case["batch"] in {1, 2, 4, 8}, "batch scope")
    require(type(case["context"]) is int and case["context"] in {1024, 4096, 8192}, "context scope")
    require(case["precision"] == "bf16" and case["accumulation"] == "fp32", "numerical scope")
    require(case["metric"] == "decode-step-latency-ns" and type(case["output_tokens"]) is int
            and case["output_tokens"] == 1, "measurement boundary")
    pins = Pins(root)
    provenance = obj(document["provenance"], PROVENANCE, "provenance")
    for name, pin in provenance.items():
        pins.read(pin, name)
    workload = obj(parse(pins.read(provenance["workload"], "workload"), "workload"),
                   {"case", "scope", "measurement_boundary", "prompt_trace", "seed"}, "workload")
    require(workload["case"] == case and workload["scope"] == document["scope"], "workload scope mismatch")
    require(workload["measurement_boundary"] == "submit-through-exact-completion",
            "kernel-only timing cannot substitute for decode-step timing")
    integer(workload["seed"], "workload seed")
    pins.read(workload["prompt_trace"], "prompt trace")
    environment = obj(parse(pins.read(provenance["environment"], "environment"), "environment"),
                      {"target", "device_uuid", "device_count", "host", "software", "hardware", "policy"},
                      "environment")
    require(environment["target"] == "gfx950" and type(environment["device_count"]) is int
            and environment["device_count"] == 1, "one gfx950 device required")
    for name in ("device_uuid", "host"):
        require(isinstance(environment[name], str) and 0 < len(environment[name]) <= 256,
                f"environment {name}")
    for name in ("software", "hardware", "policy"):
        pins.read(environment[name], f"environment {name}")
    variants = sequence(document["variants"], "variants", MAX_VARIANTS, 2)
    by_id: dict[str, dict[str, Any]] = {}
    for variant in variants:
        obj(variant, {"id", "role", "scope", "optimizations", "executable", "build_manifest",
                      "numerical_report", "environment_sha256", "workload_sha256", "tuning_budget_sha256"}, "variant")
        require(isinstance(variant["id"], str) and NAME.fullmatch(variant["id"]), "variant id")
        require(variant["id"] not in by_id, "duplicate variant id")
        require(isinstance(variant["role"], str) and variant["role"] in ROLES
                and variant["scope"] == document["scope"], "variant role/scope")
        unique_names(variant["optimizations"], "optimizations")
        for name in ("environment", "workload", "tuning_budget"):
            require(variant[name + "_sha256"] == provenance[name]["sha256"], f"variant {name} mismatch")
        pins.read(variant["executable"], "executable")
        pins.read(variant["build_manifest"], "build manifest")
        numerical(variant["numerical_report"], variant, document, pins)
        by_id[variant["id"]] = variant
    controls = [v["id"] for v in variants if v["role"] == "ferric-command-batch"]
    candidates = [v["id"] for v in variants if v["role"] == "megakernel"]
    require(len(controls) == 1 and len(candidates) == 1, "one command-batch control and one megakernel required")
    order = list(by_id)
    rows = sequence(document["runs"], "runs", MAX_RUNS, WARMUPS + SAMPLES)
    timings: dict[str, list[int]] = {key: [] for key in order}
    clocks: dict[str, list[int]] = {key: [] for key in order}
    temperatures: dict[str, list[int]] = {key: [] for key in order}
    counts = {"warmup": 0, "recorded": 0}
    recorded_started = False
    for row in rows:
        obj(row, {"phase", "ordinal", "engine_order", "values"}, "run")
        phase = row["phase"]
        require(isinstance(phase, str) and phase in counts, "run phase")
        integer(row["ordinal"], "run ordinal", 0, MAX_RUNS)
        require(row["ordinal"] == counts[phase], "missing or repeated run")
        require(not (phase == "warmup" and recorded_started), "warmup after recorded sample")
        recorded_started |= phase == "recorded"
        offset = row["ordinal"] % len(order)
        require(row["engine_order"] == order[offset:] + order[:offset], "engine order not rotated")
        obj(row["values"], set(order), "run values")
        for key, sample in row["values"].items():
            obj(sample, {"latency_ns", "clock_khz", "temperature_millicelsius", "passed",
                         "faults", "numerical_report_sha256", "executable_sha256"}, "sample")
            require(sample["passed"] is True and sample["faults"] == [], "failed/faulted sample retained: timing rejected")
            variant = by_id[key]
            for name in ("numerical_report", "executable"):
                require(sample[name + "_sha256"] == variant[name]["sha256"], f"sample {name} drift")
            integer(sample["latency_ns"], "latency", 1)
            integer(sample["clock_khz"], "clock", 1)
            integer(sample["temperature_millicelsius"], "temperature", 1, 200_000)
            clocks[key].append(sample["clock_khz"])
            temperatures[key].append(sample["temperature_millicelsius"])
            if phase == "recorded":
                timings[key].append(sample["latency_ns"])
        counts[phase] += 1
    require(counts["warmup"] >= WARMUPS and counts["recorded"] >= SAMPLES, "insufficient warmups/samples")
    summaries = {}
    for key in order:
        require(spread(timings[key]) <= Fraction(2, 100), f"{key}: latency variation exceeds 2%")
        require(spread(clocks[key]) <= Fraction(3, 100) and spread(temperatures[key]) <= Fraction(3, 100),
                f"{key}: thermal/clock drift exceeds 3%")
        sorted_times = sorted(timings[key])
        summaries[key] = {"median_ns": rational(median(sorted_times)),
                          "p90_ns": sorted_times[math.ceil(len(sorted_times) * 0.9) - 1],
                          "p99_ns": sorted_times[math.ceil(len(sorted_times) * 0.99) - 1],
                          "range_over_median_ppm": spread_ppm(sorted_times)}
    control, candidate = controls[0], candidates[0]
    comparisons = {key: comparison(timings[control], values) for key, values in timings.items() if key != control}
    ablations = []
    edges: set[tuple[str, str]] = set()
    for edge in sequence(document["ablations"], "ablations", 64):
        obj(edge, {"control", "candidate", "changed_optimization"}, "ablation")
        before, after, change = edge["control"], edge["candidate"], edge["changed_optimization"]
        require(isinstance(before, str) and isinstance(after, str)
                and before in by_id and after in by_id and before != after, "ablation endpoints")
        require((before, after) not in edges, "duplicate ablation")
        edges.add((before, after))
        prior = set(by_id[before]["optimizations"])
        following = set(by_id[after]["optimizations"])
        require(isinstance(change, str) and following - prior == {change} and prior <= following,
                "ablation must add exactly one named optimization")
        ablations.append({**edge, **comparison(timings[before], timings[after]),
                          "saved_median_ns": rational(median(timings[before]) - median(timings[after]))})
    interactions = []
    interaction_roster: set[tuple[str, str, str, str]] = set()
    for interaction in sequence(document["interactions"], "interactions", 32):
        obj(interaction, {"control", "a", "b", "combined"}, "interaction")
        keys = tuple(interaction[key] for key in ("control", "a", "b", "combined"))
        require(all(isinstance(key, str) and key in by_id for key in keys)
                and len(set(keys)) == 4, "interaction endpoints")
        require(keys not in interaction_roster, "duplicate interaction")
        interaction_roster.add(keys)
        base, a, b, combined = [set(by_id[key]["optimizations"]) for key in keys]
        require(base < a and base < b and len(a - base) == len(b - base) == 1
                and a != b and combined == a | b, "interaction requires matched two-factor variants")
        control_ns, a_ns, b_ns, combined_ns = [median(timings[key]) for key in keys]
        interactions.append({**interaction,
            "additional_saving_ns": rational(a_ns + b_ns - control_ns - combined_ns),
            "interpretation": "Positive means combined saving exceeds the sum of separate savings. Descriptive medians, not an interaction significance test."})
    bound_report = None
    if document["bound"] is not None:
        bound = obj(document["bound"], {"minimum_hbm_bytes", "operations", "sustained_bytes_per_second",
                    "sustained_flops_per_second", "critical_path_ns", "calibration", "derivation"}, "bound")
        for name in ("minimum_hbm_bytes", "operations", "critical_path_ns"):
            integer(bound[name], name)
        for name in ("sustained_bytes_per_second", "sustained_flops_per_second"):
            integer(bound[name], name, 1)
        require(bound["minimum_hbm_bytes"] > 0 or bound["operations"] > 0, "empty bound")
        pins.read(bound["calibration"], "bound calibration")
        pins.read(bound["derivation"], "bound derivation")
        floor = max(Fraction(bound["minimum_hbm_bytes"] * 10**9, bound["sustained_bytes_per_second"]),
                    Fraction(bound["operations"] * 10**9, bound["sustained_flops_per_second"]),
                    Fraction(bound["critical_path_ns"]))
        bound_report = {"modeled_floor_ns": rational(floor), "physical_lower_bound_proved": False,
                        "floor_over_median": {key: rational(floor / median(values)) for key, values in timings.items()},
                        "inconsistent_bound_requires_audit": any(floor > median(values) for values in timings.values()),
                        "calibration_sha256": bound["calibration"]["sha256"],
                        "derivation_sha256": bound["derivation"]["sha256"]}
    vendor_ids = [v["id"] for v in variants if v["role"] in {"vllm", "sglang"}]
    fastest_vendor = min(vendor_ids, key=lambda key: median(timings[key])) if vendor_ids else None
    return {"format": REPORT_FORMAT, "authority": "checked-engineering-measurements-only",
            "qualification": False, "public_faster_claim": False, "nonclaim": NONCLAIM,
            "target": "gfx950", "scope": document["scope"], "case": case,
            "input_sha256": hashlib.sha256(canonical(document)).hexdigest(),
            "provenance": provenance, "counts": counts,
            "bootstrap": {"rounds": BOOTSTRAP_ROUNDS, "seed": BOOTSTRAP_SEED,
                          "method": "paired-ratio-of-medians-python-random-randrange-percentiles-v1"},
            "variants": summaries, "relative_to_command_batch": comparisons,
            "fastest_measured_external_baseline": fastest_vendor,
            "candidate_vs_fastest_measured_external": comparison(timings[fastest_vendor], timings[candidate])
                if fastest_vendor else None,
            "ablations": ablations, "interactions": interactions,
            "ablation_interpretation": "Each delta is conditional on its control. Deltas are not independent or additive; omitted interactions are unmeasured.",
            "bound": bound_report}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence_root", type=Path)
    parser.add_argument("measurements", help="relative canonical JSON path within evidence_root")
    args = parser.parse_args()
    try:
        raw = stable_file(args.evidence_root, args.measurements, MAX_DOCUMENT_BYTES)
        report = validate(parse(raw, "measurements"), args.evidence_root)
    except (ValueError, OSError, TypeError, KeyError, OverflowError) as error:
        print(f"REJECTED: {error}", file=sys.stderr)
        return 1
    sys.stdout.buffer.write(canonical(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
