"""Synthetic CPU-only controls, never evidence of GPU execution."""
import copy
from pathlib import Path
import tempfile
import unittest
from xml.etree import ElementTree as ET

from common import CHECK, nearest, pinned
import plots
from trace import validate_trace

CAPTURE = "a" * 64
ARTIFACT = "b" * 64


def synthetic_trace(device=False):
    return {"schema": "FerricDecodeTraceV1", "authority": "none", "capture_sha256": CAPTURE,
            "artifact_sha256": ARTIFACT, "dropped_events": 0,
            "collector": {"kind": "device-timestamp-buffer" if device else "host-raw-spans",
                          "source_sha256": "c" * 64, "clock_evidence_sha256": "d" * 64, "overhead": "unmeasured"},
            "clocks": [{"id": "synthetic-test-clock", "kind": "device-counter" if device else "controller-monotonic-raw",
                        "ticks_per_second": 10**9, "counter_bits": 64, "unwrapped": True, "max_drift_ppm": 0,
                        "anchors": [{"tick": 0, "host_offset_ns": 0, "max_error_ns": 1},
                                    {"tick": 1000, "host_offset_ns": 1000, "max_error_ns": 1}] if device else []}],
            "events": [{"id": "synthetic-test-0", "label": "SYNTHETIC CPU TEST", "lane": "lane-a", "clock": "synthetic-test-clock",
                        "run": 0, "token": 0, "begin_tick": 100, "end_tick": 400},
                       {"id": "synthetic-test-1", "label": "SYNTHETIC CPU TEST", "lane": "lane-b", "clock": "synthetic-test-clock",
                        "run": 0, "token": 1, "begin_tick": 200, "end_tick": 500}]}


class TraceTests(unittest.TestCase):
    def checked(self, value):
        return validate_trace(value, CAPTURE, ARTIFACT, 1, 2)

    def test_exact_host_lanes_not_gpu_claim(self):
        result = self.checked(synthetic_trace())
        self.assertEqual(result["domains"][0]["overlapping_lane_nanoseconds"], 200)
        self.assertEqual(result["domains"][0]["maximum_distinct_lanes"], 2)
        self.assertIn("not GPU", result["domains"][0]["interpretation"])
        self.assertFalse(result["cross_domain_overlap_computed"])

    def test_nested_same_lane_not_counted_twice(self):
        value = synthetic_trace()
        value["events"][1]["lane"] = "lane-a"
        result = self.checked(value)
        self.assertEqual(result["domains"][0]["overlapping_lane_nanoseconds"], 0)

    def test_device_calibration_and_uncertainty_retained(self):
        result = self.checked(synthetic_trace(True))
        self.assertEqual(result["events"][0]["begin_ns"], 100)
        self.assertGreaterEqual(result["events"][0]["max_error_ns"], 1)
        self.assertIn("not authenticated", result["domains"][0]["interpretation"])

    def test_device_clock_mutations_rejected(self):
        original = synthetic_trace(True)
        for edit in (
            lambda x: x["clocks"][0].update(anchors=[]),
            lambda x: x["clocks"][0].update(counter_bits=32),
            lambda x: x["clocks"][0].update(counter_bits=64.0),
            lambda x: x["clocks"][0].update(unwrapped=False),
            lambda x: x["clocks"][0].update(ticks_per_second=10**8),
            lambda x: x["clocks"][0]["anchors"][1].update(tick=0),
            lambda x: x["clocks"][0]["anchors"][1].update(host_offset_ns=0),
            lambda x: x["events"][0].update(begin_tick=1001, end_tick=1002),
            lambda x: x["collector"].update(kind="host-raw-spans"),
        ):
            value = copy.deepcopy(original)
            edit(value)
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.checked(value)

    def test_capture_shape_identity_and_event_mutations_rejected(self):
        for edit in (
            lambda x: x.update(capture_sha256="f" * 64),
            lambda x: x.update(artifact_sha256="f" * 64),
            lambda x: x.update(dropped_events=1),
            lambda x: x.update(authority="authenticated"),
            lambda x: x.update(events=[]),
            lambda x: x["collector"].update(kind="synthetic"),
            lambda x: x["collector"].update(overhead="zero"),
            lambda x: x["events"][0].update(begin_tick=True),
            lambda x: x["events"][0].update(end_tick=100),
            lambda x: x["events"][0].update(clock="absent"),
            lambda x: x["events"][0].update(run=1),
            lambda x: x["events"][0].update(token=2),
            lambda x: x["events"][1].update(id="synthetic-test-0"),
            lambda x: x["events"][0].update(duration_only_ns=50),
        ):
            value = synthetic_trace()
            edit(value)
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.checked(value)

    def test_domains_never_cross_joined(self):
        value = synthetic_trace()
        extra = copy.deepcopy(value["clocks"][0])
        extra["id"] = "second-test-domain"
        value["clocks"].append(extra)
        value["events"][1]["clock"] = extra["id"]
        result = self.checked(value)
        self.assertEqual([d["overlapping_lane_nanoseconds"] for d in result["domains"]], [0, 0])

    def test_svg_only_from_validated_events_and_escaped_labels(self):
        value = synthetic_trace()
        value["events"][0]["label"] = "SYNTHETIC TEST <script>"
        trace = self.checked(value)
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / "synthetic-test.svg"
            plots.timeline(path, trace)
            root = ET.parse(path).getroot()
            self.assertEqual(root.tag, "{http://www.w3.org/2000/svg}svg")
            self.assertNotIn(b"<script>", path.read_bytes())
            with self.assertRaises(FileExistsError):
                plots.timeline(path, trace)


class InputTests(unittest.TestCase):
    def test_strict_json_duplicate_and_nan_reject(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError):
                CHECK.json_value(raw)

    def test_pinned_file_drift_and_symlink_reject(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "raw").write_bytes(b"synthetic test")
            good = {"path": "raw", "sha256": CHECK.sha256(b"synthetic test")}
            self.assertEqual(pinned(good, root)[1], b"synthetic test")
            (root / "raw").write_bytes(b"changed")
            with self.assertRaises(ValueError):
                pinned(good, root)
            (root / "link").symlink_to(root / "raw")
            with self.assertRaises(OSError):
                pinned({**good, "path": "link"}, root)

    def test_nearest_rank_not_interpolated(self):
        self.assertEqual(nearest([10, 20, 30, 40], 50), 20)
        self.assertEqual(nearest([10, 20, 30, 40], 99), 40)


if __name__ == "__main__":
    unittest.main()
