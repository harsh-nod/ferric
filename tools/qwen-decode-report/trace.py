"""Import actual recorded spans; never infer scheduling from duration totals."""
from collections import defaultdict
from fractions import Fraction
from common import fields, hash_value, integer, require

MAX_EVENTS = 4096
MAX_LANES = 128
MAX_NS = 24 * 60 * 60 * 10**9


def text(value):
    require(type(value) is str and 0 < len(value) <= 96 and value.isascii()
            and all(32 <= ord(c) < 127 for c in value), "trace label")
    return value


def clock_spec(clock):
    fields(clock, {"id", "kind", "ticks_per_second", "counter_bits", "unwrapped", "max_drift_ppm", "anchors"}, "trace clock")
    text(clock["id"])
    require(clock["kind"] in ("controller-monotonic-raw", "device-counter"), "clock domain kind")
    hz = integer(clock["ticks_per_second"], 1, 10**12, "clock frequency")
    integer(clock["counter_bits"], 64, 64, "clock counter width")
    require(clock["unwrapped"] is True, "wrapped/truncated clock unsupported")
    drift = integer(clock["max_drift_ppm"], 0, 1000, "clock drift bound")
    anchors = clock["anchors"]
    require(type(anchors) is list and len(anchors) <= 64, "calibration bound")
    if clock["kind"] == "controller-monotonic-raw":
        require(hz == 10**9 and drift == 0 and not anchors, "host offsets must be original RAW nanoseconds")
        return lambda tick: (Fraction(tick), Fraction(0))
    require(len(anchors) >= 2, "device clock requires bracketing calibration")
    previous = None
    for anchor in anchors:
        fields(anchor, {"tick", "host_offset_ns", "max_error_ns"}, "calibration anchor")
        for key in anchor:
            integer(anchor[key], 0, (1 << 64) - 1, "calibration scalar")
        integer(anchor["host_offset_ns"], 0, MAX_NS, "controller-relative calibration")
        integer(anchor["max_error_ns"], 0, 10**9, "calibration uncertainty")
        if previous:
            require(anchor["tick"] > previous["tick"] and anchor["host_offset_ns"] > previous["host_offset_ns"], "calibration order")
        previous = anchor
    first = anchors[0]
    def mapping(tick):
        require(first["tick"] <= tick <= anchors[-1]["tick"], "event outside clock calibration")
        elapsed = Fraction((tick - first["tick"]) * 10**9, hz)
        error = max(a["max_error_ns"] for a in anchors) + abs(elapsed) * Fraction(drift, 10**6) + Fraction(10**9, hz)
        return first["host_offset_ns"] + elapsed, error
    for anchor in anchors:
        predicted, error = mapping(anchor["tick"])
        require(abs(predicted - anchor["host_offset_ns"]) <= error + anchor["max_error_ns"], "clock frequency/calibration disagreement")
    return mapping


def interval_union(intervals):
    merged = []
    for begin, end in sorted(intervals):
        if merged and begin <= merged[-1][1]:
            merged[-1][1] = max(end, merged[-1][1])
        else:
            merged.append([begin, end])
    return merged


def overlap(events):
    """Count distinct lanes, not nested spans; do not join different clocks."""
    lanes = defaultdict(list)
    for event in events:
        lanes[event["lane"]].append((event["begin_ns"], event["end_ns"]))
    points = defaultdict(int)
    for intervals in lanes.values():
        for begin, end in interval_union(intervals):
            points[begin] += 1
            points[end] -= 1
    active = peak = 0
    prior = None
    together = 0
    for instant, delta in sorted(points.items()):
        if prior is not None and active >= 2:
            together += instant - prior
        active += delta
        peak = max(peak, active)
        prior = instant
    return {"maximum_distinct_lanes": peak, "overlapping_lane_nanoseconds": together}


def validate_trace(value, capture_sha256, artifact_sha256, run_count, token_count):
    fields(value, {"schema", "authority", "capture_sha256", "artifact_sha256", "collector", "clocks", "events", "dropped_events"}, "trace")
    require(value["schema"] == "FerricDecodeTraceV1" and value["authority"] == "none", "trace schema/authority")
    require(value["capture_sha256"] == capture_sha256 and value["artifact_sha256"] == artifact_sha256, "trace capture/artifact substitution")
    require(type(value["dropped_events"]) is int and value["dropped_events"] == 0, "incomplete trace")
    collector = value["collector"]
    fields(collector, {"kind", "source_sha256", "clock_evidence_sha256", "overhead"}, "trace collector")
    require(collector["kind"] in ("host-raw-spans", "device-timestamp-buffer", "rocprofiler-sdk"), "not a real trace collector kind")
    hash_value(collector["source_sha256"], "collector source")
    hash_value(collector["clock_evidence_sha256"], "clock evidence")
    require(collector["overhead"] == "unmeasured", "overhead must be computed from a separate matched pair")
    require(type(value["clocks"]) is list and 1 <= len(value["clocks"]) <= 16, "clock count")
    clocks = {}
    for clock in value["clocks"]:
        require(clock["id"] not in clocks, "duplicate clock")
        clocks[clock["id"]] = (clock, clock_spec(clock))
    rows = value["events"]
    require(type(rows) is list and 1 <= len(rows) <= MAX_EVENTS, "trace event count")
    events, ids, lanes = [], set(), set()
    for row in rows:
        fields(row, {"id", "label", "lane", "clock", "run", "token", "begin_tick", "end_tick"}, "trace event")
        for key in ("id", "label", "lane", "clock"):
            text(row[key])
        require(row["id"] not in ids and row["clock"] in clocks, "duplicate event or unknown clock")
        ids.add(row["id"])
        lanes.add((row["clock"], row["lane"]))
        require(len(lanes) <= MAX_LANES, "trace lane bound")
        integer(row["run"], 0, run_count - 1, "trace run")
        integer(row["token"], 0, token_count - 1, "trace token")
        begin = integer(row["begin_tick"], 0, (1 << 64) - 1, "event begin")
        end = integer(row["end_tick"], begin + 1, (1 << 64) - 1, "event end")
        clock, mapping = clocks[row["clock"]]
        if clock["kind"] == "device-counter":
            require(collector["kind"] != "host-raw-spans", "host spans are not GPU events")
        left, le = mapping(begin)
        right, re = mapping(end)
        require(0 <= left < right <= MAX_NS, "trace exceeds bounded controller-relative interval")
        events.append({**row, "begin_ns": float(left), "end_ns": float(right), "max_error_ns": float(max(le, re))})
    domains = []
    for name, (clock, _) in clocks.items():
        selected = [row for row in events if row["clock"] == name]
        require(selected, "unused clock")
        domains.append({"clock": name, "kind": clock["kind"], **overlap(selected),
                        "interpretation": "recorded host-scope overlap, not GPU concurrency" if clock["kind"] == "controller-monotonic-raw"
                        else "producer-reported calibrated device spans; collector truth is not authenticated"})
    return {"events": events, "domains": domains, "cross_domain_overlap_computed": False,
            "collector": collector, "clock_claim": "structural calibration checks only, not hardware attestation"}
