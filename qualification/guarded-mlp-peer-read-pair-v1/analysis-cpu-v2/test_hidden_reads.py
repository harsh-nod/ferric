"""Synthetic data checks only; these fixtures are not GPU or numerical evidence."""
import copy
import contextlib
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import analyze_hidden_reads as a


def fixture(paired):
    pin = lambda raw: dict(path="/synthetic/body", bytes=len(raw),
                           sha256=hashlib.sha256(raw).hexdigest())
    rank = [0] * 19
    rank[2], rank[13], rank[14] = (0 if paired else 2), 1, 8192
    hidden = dict(host_elapsed_ns=10 if paired else 20,
                  ranks=[rank[:], rank[:]], shared=[2 if paired else 4, 2, 0, 0])
    host_raw = b"synthetic closed report body"
    host = dict(counter_names=a.COUNTERS, shared_counter_names=a.SHARED,
                source=pin(host_raw), snapshots=587, intervals=586,
                same_run_completions_joined=True, native_close_confirmed=True,
                inclusive_nested_host_scopes=True, shared_full_currentness=True,
                forward_rows=[dict(position=p, forward_host_ns=900, bracket_host_ns=1000,
                    layers=[dict(layer=l, hidden_read=copy.deepcopy(hidden)) for l in range(36)])
                    for p in range(4)])
    for key in ("gpu_time", "gpu_overlap", "throughput", "numerical_acceptance",
                "full_model_acceptance", "performance_claim", "production_authority"):
        host[key] = False
    summary = dict(child_pid=100, request=dict(schema="synthetic", decode=dict(mode="autoregressive",
                   session=[1] * 32, evidence_directory="/synthetic", model="same")))
    bodies = {"host-observation.json": json.dumps(host).encode(),
              "native/child-stderr.bin": host_raw,
              "native/complete.json": json.dumps(summary).encode()}
    for p in range(4):
        bodies["native/observation-%d.bin" % p] = bytes([p]) * 606976
    terminal = dict(passed=True, errors=[], native_attempts=1, retries=0,
                    gpu_execution_confirmed=True, host_observation_verified=True,
                    shared_full_currentness_verified=True, paired_read_requested=paired,
                    phases=[dict(exit_code=0, reason=None, cleanup_signalled=False,
                                 owned_groups_absent=True, owned_processes_reaped=True)] * 11,
                    host_observation=host, raw={k: pin(v) for k, v in bodies.items()},
                    instrumentation_comparison=dict(all_payloads_equal=True, all_histories_equal=True),
                    admission=dict(worker=pin(b"worker"), parent=pin(b"parent")),
                    observation=dict(input_tokens=[9112, 67, 25, 576]),
                    platform=dict(host="synthetic", boot="same", ids=[16366993098680759275,
                                                                    10838076764495710945]))
    for key in ("performance_claim", "production_authority", "full_model_acceptance",
                "numerical_acceptance", "full_long_workload"):
        terminal[key] = False
    return terminal, bodies


class HiddenReadTests(unittest.TestCase):
    def capsule(self, root):
        manifest = dict(schema="ferric-peer-read-supervised-abba-v1",
                        controller=dict(path="/synthetic/controller.py", bytes=1, sha256="a" * 64),
                        runs=[])
        for i, name in enumerate(a.ORDER):
            terminal, bodies = fixture(name.startswith("paired"))
            summary = json.loads(bodies["native/complete.json"])
            summary["child_pid"] += i
            summary["request"]["decode"]["session"] = [i + 1] * 32
            bodies["native/complete.json"] = json.dumps(summary).encode()
            terminal["raw"]["native/complete.json"].update(
                bytes=len(bodies["native/complete.json"]),
                sha256=hashlib.sha256(bodies["native/complete.json"]).hexdigest())
            bodies["complete.json"] = json.dumps(terminal).encode()
            for relative, body in bodies.items():
                path = root / name / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(body)
            raw = bodies["complete.json"]
            manifest["runs"].append(dict(name=name,
                terminal=dict(path=str(root / name / "complete.json"), bytes=len(raw),
                              sha256=hashlib.sha256(raw).hexdigest()),
                started_monotonic_ns=2 * i + 1, finished_monotonic_ns=2 * i + 2))
        return manifest

    def invoke(self, root, manifest, digest=None):
        raw = json.dumps(manifest).encode()
        (root / "comparison-input.json").write_bytes(raw)
        args = ["analyze_hidden_reads.py", str(root), digest or hashlib.sha256(raw).hexdigest()]
        output = io.StringIO()
        with mock.patch.object(a.sys, "argv", args), contextlib.redirect_stdout(output):
            a.main()
        return json.loads(output.getvalue())

    def test_cli_authenticates_complete_four_run_capsule(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as temporary:
            root = Path(temporary)
            report = self.invoke(root, self.capsule(root))
            self.assertEqual(report["observed_hidden_read_ratio"], 2)
            self.assertEqual(report["independent_runs_per_mode"], 2)
            self.assertFalse(report["sustained_tokens_per_second"])

    def test_cli_rejects_manifest_pin_terminal_pin_order_and_chronology(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as temporary:
            root = Path(temporary)
            manifest = self.capsule(root)
            with self.assertRaisesRegex(ValueError, "authenticated comparison manifest"):
                self.invoke(root, manifest, "0" * 64)
            for mutation in ("pin", "order", "overlap", "missing"):
                candidate = copy.deepcopy(manifest)
                if mutation == "pin":
                    candidate["runs"][0]["terminal"]["sha256"] = "0" * 64
                elif mutation == "order":
                    candidate["runs"].reverse()
                elif mutation == "overlap":
                    candidate["runs"][1]["started_monotonic_ns"] = 1
                else:
                    candidate["runs"].pop()
                with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                    self.invoke(root, candidate)

    def test_cli_rejects_corrupt_missing_or_symlinked_payload(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as temporary:
            root = Path(temporary)
            manifest = self.capsule(root)
            path = root / "paired-0/native/observation-3.bin"
            original = path.read_bytes()
            path.write_bytes(b"x" + original[1:])
            with self.assertRaises(ValueError):
                self.invoke(root, manifest)
            path.unlink()
            with self.assertRaises(FileNotFoundError):
                self.invoke(root, manifest)
            target = root / "payload.bin"
            target.write_bytes(original)
            path.symlink_to(target)
            with self.assertRaisesRegex(ValueError, "bounded ordinary capsule body"):
                self.invoke(root, manifest)

    def case(self, paired):
        terminal, bodies = fixture(paired)
        return a.inspect_case(terminal, bodies.__getitem__, paired)

    def cases(self):
        cases, chronology = {}, []
        for i, name in enumerate(a.ORDER):
            row = self.case(name.startswith("paired"))
            row.update(session=[i + 1] * 32, child_pid=100 + i)
            cases[name] = row
            chronology.append(dict(name=name, terminal=dict(path="/synthetic/" + name,
                                   bytes=1, sha256="a" * 64),
                                   started_monotonic_ns=2 * i + 1, finished_monotonic_ns=2 * i + 2))
        return cases, chronology

    def test_scoped_abba_analysis_has_no_end_to_end_claim(self):
        cases, chronology = self.cases()
        report = a.compare(cases, chronology)
        self.assertEqual(report["observed_hidden_read_ratio"], 2)
        self.assertEqual(report["observed_hidden_read_reduction_fraction"], .5)
        self.assertFalse(report["end_to_end_speedup"])
        self.assertFalse(report["sustained_tokens_per_second"])
        self.assertEqual(report["runs"]["control-0"]["platform"]["ids"][0], 16366993098680759275)

    def test_payload_corruption_or_missing_frame_is_refused(self):
        for missing in (False, True):
            terminal, bodies = fixture(False)
            name = "native/observation-3.bin"
            if missing:
                del bodies[name]
            else:
                bodies[name] = b"x" + bodies[name][1:]
            with self.assertRaises((ValueError, KeyError)):
                a.inspect_case(terminal, bodies.__getitem__, False)

    def test_mode_counter_and_cleanup_drift_are_refused(self):
        for mutation in range(5):
            terminal, bodies = fixture(True)
            if mutation == 0:
                terminal["paired_read_requested"] = False
            elif mutation == 1:
                terminal["phases"][0]["cleanup_signalled"] = True
            elif mutation == 2:
                terminal["host_observation"]["forward_rows"][0]["layers"][0]["hidden_read"]["shared"][0] = 4
            elif mutation == 3:
                terminal["host_observation"]["forward_rows"][0]["layers"][0]["hidden_read"]["ranks"][0][13] = 2
            else:
                terminal["host_observation"]["forward_rows"][0]["position"] = False
            raw = json.dumps(terminal["host_observation"]).encode()
            bodies["host-observation.json"] = raw
            terminal["raw"]["host-observation.json"].update(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            with self.assertRaises(ValueError):
                a.inspect_case(terminal, bodies.__getitem__, True)

    def test_comparison_requires_same_inputs_histories_payloads_and_executables(self):
        for field in ("worker", "parent", "immutable_request", "platform", "input_tokens", "payloads"):
            cases, chronology = self.cases()
            cases["paired-0"][field] = "different"
            with self.assertRaises(ValueError):
                a.compare(cases, chronology)

    def test_partial_reordered_or_nonpositive_comparison_is_refused(self):
        cases, chronology = self.cases()
        with self.assertRaises(ValueError):
            a.compare(dict(reversed(list(cases.items()))), chronology)
        with self.assertRaises(ValueError):
            a.compare({name: cases[name] for name in a.ORDER[:2]}, chronology)
        cases["paired-0"]["hidden_read_total_ns"] = 0
        with self.assertRaises(ValueError):
            a.compare(cases, chronology)
        for field in ("session", "child_pid"):
            cases, chronology = self.cases()
            cases["paired-0"][field] = cases["control-0"][field]
            with self.assertRaises(ValueError):
                a.compare(cases, chronology)
        cases, chronology = self.cases()
        chronology[1]["started_monotonic_ns"] = 1
        with self.assertRaises(ValueError):
            a.compare(cases, chronology)


if __name__ == "__main__":
    unittest.main()
