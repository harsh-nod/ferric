import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import types
import unittest


def module(name):
    spec = importlib.util.spec_from_file_location("replay_test_" + name, Path(__file__).with_name(name + ".py"))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


m = module("native_campaign_replay")
fixtures = module("test_native_token_cell")


def pretty(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()


def write_json(path, value):
    raw = pretty(value)
    path.write_bytes(raw)
    return m.ledger.sha(raw)


def cpu_cost(before, after, ticks):
    m.require(type(ticks) is int and ticks > 0, "fake CPU frequency required")
    roles = {}
    for role in ("controller", "worker"):
        delta = after["cpu"][role]["process"]["utime_ticks"] - before["cpu"][role]["process"]["utime_ticks"]
        m.require(delta >= 0, "fake CPU counters decreased")
        roles[role] = {"cpu_seconds": delta / ticks, "cpu_seconds_per_token": delta / ticks / 128}
    return {"roles": roles, "clock_ticks_per_second": ticks, "scope": "fake initial scope"}


EVIDENCE = types.SimpleNamespace(StreamParity=fixtures.Parity, cpu_cost=cpu_cost)
LEGACY = types.SimpleNamespace(Events=fixtures.Events)


def staged_spec(stage, mode, arm):
    spec, setup, closed = fixtures.fixture(mode, arm)
    for key in ("controller", "worker"):
        old = spec[key]["path"]
        destination = stage / ("worker-candidate" if key == "worker"
                               else ("controller-counters-" if mode == "counters" else "controller-live-") + arm)
        spec[key]["path"] = str(destination)
        spec["argv"] = [str(destination) if value == old else value for value in spec["argv"]]
    setup["controller_sha256"] = spec["controller"]["sha256"]
    return spec, setup, closed


def stage_fixture(stage):
    (stage / "cells").mkdir(mode=0o700)
    plan = {"schema": "FerricNativeGateUpCampaignPlanR1", "stage": str(stage), "cells": []}
    for cell_id, arm, mode in m.CELL_ORDER:
        spec, _, _ = staged_spec(stage, mode, arm)
        plan["cells"].append({"cell_id": cell_id, "spec": spec, "output": str(stage / "cells" / cell_id),
                              "common_args": []})
    binding = {"path": str(stage / "plan.json"), "sha256": write_json(stage / "plan.json", plan)}
    return plan, binding


def cell_fixture(stage, plan, binding, cell_id):
    entry = next(row for row in plan["cells"] if row["cell_id"] == cell_id)
    spec, setup, closed = staged_spec(stage, entry["spec"]["mode"], entry["spec"]["arm"])
    output = stage / "cells" / cell_id
    output.mkdir(mode=0o700)
    raw_dir = output / "cell-results"
    raw_dir.mkdir(mode=0o700)
    stderr = b""
    if spec["mode"] == "counters":
        stderr = b"\n".join(m.ledger.canonical(value) for value in fixtures.counter_lines(spec, setup)) + b"\n"
    controller = fixtures.Controller(fixtures.transcript(spec, setup, closed), raw_dir, stderr)
    result = m.cell.consume(controller, spec, runner=fixtures.Runner, legacy=LEGACY, evidence=EVIDENCE, deadline=0)
    result.update(schema="FerricNativeGateUpTokenCellResultR1", spec_sha256=m.ledger.digest(spec), accepted=True,
                  arm=spec["arm"], mode=spec["mode"], raw_replay_passed=True,
                  instrumented=spec["mode"] == "counters", latency_admitted=spec["mode"] == "latency",
                  started_ns=100, finished_ns=200, cleanup=controller.close(),
                  raw_sha256={name: m.ledger.sha((raw_dir / name).read_bytes())
                              for name in ("stdin.raw", "stdout.raw", "stderr.raw")})
    if spec["mode"] == "counters":
        result["mechanism"] = m.cell.counter_replay(stderr, spec, setup)
    placement = {"worker": {"process_id": 42, "parent_pid": 41}, "controller": {"process_id": 41, "parent_pid": 40}}
    endpoints = []
    for ticks in (10, 20):
        endpoints.append({"cpu": {role: {"process": {**value, "utime_ticks": ticks}} for role, value in placement.items()},
                          "executables": {role: {"opened_executable": {"sha256": spec[role]["sha256"]}}
                                          for role in ("worker", "controller")}})
    cost = cpu_cost(*endpoints, 100)
    count, _ = m.cell.shape(spec)
    for value in cost["roles"].values():
        value["cpu_seconds_per_token"] = value["cpu_seconds"] / (count * 128)
    cost.update(scope="All requests including excluded warmups; excludes setup/teardown; not GPU time", output_tokens=count * 128)
    write_json(output / "process-endpoints.json", {"placements": [placement, placement], "endpoints": endpoints, "cpu_cost": cost})
    completion = {"accepted": True, "cell_id": cell_id, "model_stable": True, "input_files_stable": True,
                  "cell_result_sha256": write_json(raw_dir / "result.json", result), "cpu_cost": cost}
    outer = {"status": 0, "cleanup_ok": True, "child_reaped": True, "errors": [], "term_sent": False,
             "kill_sent": False, "termination_reason": "completed", "postflight": {"accepted": True},
             "completion_sha256": write_json(output / "completion.json", completion),
             "argv": ["/usr/bin/python3", "-I", "-B", str(stage / "run_stage.py"), "--execute", "--plan", binding["path"],
                      "--plan-sha256", binding["sha256"], "--cell-id", cell_id]}
    write_json(output / "launch-supervisor.json", outer)
    (output / "launch.status").write_bytes(b"0\n")
    return entry, output


class RetainedReplayTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.stage = Path(self.temporary.name)
        self.plan, self.binding = stage_fixture(self.stage)
        self.entry, self.output = cell_fixture(self.stage, self.plan, self.binding, "block1-A1")

    def load(self, entry=None, output=None):
        return m.load_retained_cell(entry or self.entry, output or self.output, plan_binding=self.binding,
                                    runner=fixtures.Runner, legacy=LEGACY, evidence=EVIDENCE)

    def mutate(self, path, action):
        value = json.loads(path.read_bytes())
        action(value)
        write_json(path, value)

    def rebind_inner(self):
        completion_path = self.output / "completion.json"
        self.mutate(completion_path, lambda value: value.update(cell_result_sha256=m.ledger.sha((self.output / "cell-results/result.json").read_bytes())))
        self.mutate(self.output / "launch-supervisor.json",
                    lambda value: value.update(completion_sha256=m.ledger.sha(completion_path.read_bytes())))

    def test_valid_latency_transcript_replayed(self):
        result = self.load()
        self.assertEqual(result["cell_id"], "block1-A1")
        self.assertTrue(result["result"]["latency_admitted"])
        self.assertEqual(result["result"]["exact_output_tokens_checked"], 768)
        self.assertIn("process_endpoints_sha256", result["custody"])

    def test_two_field_entry_projection_supported(self):
        value = self.load(entry={"cell_id": self.entry["cell_id"], "spec": self.entry["spec"]})
        self.assertTrue(value["result"]["accepted"])

    def test_actual_nonzero_outer_status_rejected(self):
        (self.output / "launch.status").write_bytes(b"125\n")
        with self.assertRaisesRegex(ValueError, "exit status"):
            self.load()

    def test_outer_failure_rejected_even_if_status_file_zero(self):
        self.mutate(self.output / "launch-supervisor.json", lambda value: value.update(status=125))
        with self.assertRaisesRegex(ValueError, "outer supervisor"):
            self.load()

    def test_outer_plan_hash_must_match_this_campaign(self):
        self.mutate(self.output / "launch-supervisor.json", lambda value: value["argv"].__setitem__(-3, "f" * 64))
        with self.assertRaisesRegex(ValueError, "exact campaign"):
            self.load()

    def test_outer_cell_id_must_match_this_cell(self):
        self.mutate(self.output / "launch-supervisor.json", lambda value: value["argv"].__setitem__(-1, "block1-B1"))
        with self.assertRaisesRegex(ValueError, "exact campaign"):
            self.load()

    def test_completion_cell_id_must_match_this_cell(self):
        self.mutate(self.output / "completion.json", lambda value: value.update(cell_id="block1-B1"))
        self.mutate(self.output / "launch-supervisor.json",
                    lambda value: value.update(completion_sha256=m.ledger.sha((self.output / "completion.json").read_bytes())))
        with self.assertRaisesRegex(ValueError, "different campaign cell"):
            self.load()

    def test_unknown_outer_argv_rejected(self):
        self.mutate(self.output / "launch-supervisor.json", lambda value: value["argv"].extend(["--other", "value"]))
        with self.assertRaisesRegex(ValueError, "closed outer"):
            self.load()

    def test_raw_stdout_hash_mismatch_rejected(self):
        with (self.output / "cell-results/stdout.raw").open("ab") as target:
            target.write(b"\n")
        with self.assertRaisesRegex(ValueError, "transcript hashes"):
            self.load()

    def test_forged_inner_summary_cannot_survive_raw_replay(self):
        self.mutate(self.output / "cell-results/result.json", lambda value: value["summary"].update(window_ns=1))
        self.rebind_inner()
        with self.assertRaisesRegex(ValueError, "raw replay differs"):
            self.load()

    def test_nonempty_latency_stderr_rejected(self):
        path = self.output / "cell-results/stderr.raw"
        path.write_bytes(b"unexpected diagnostics\n")
        self.mutate(self.output / "cell-results/result.json",
                    lambda value: value["raw_sha256"].update({"stderr.raw": m.ledger.sha(path.read_bytes())}))
        self.rebind_inner()
        with self.assertRaisesRegex(ValueError, "counter data"):
            self.load()

    def test_unknown_entry_field_rejected(self):
        with self.assertRaisesRegex(ValueError, "closed retained"):
            self.load(entry={**self.entry, "ignored": "not allowed"})

    def test_plan_bytes_changed_rejected(self):
        self.mutate(self.stage / "plan.json", lambda value: value.update(extra="changed"))
        with self.assertRaisesRegex(ValueError, "plan hash"):
            self.load()

    def test_plan_roster_order_changed_rejected(self):
        self.plan["cells"][0], self.plan["cells"][1] = self.plan["cells"][1], self.plan["cells"][0]
        self.binding["sha256"] = write_json(self.stage / "plan.json", self.plan)
        with self.assertRaisesRegex(ValueError, "roster/order"):
            self.load()

    def test_cpu_ticks_recomputed_not_trusted(self):
        self.mutate(self.output / "process-endpoints.json", lambda value: value["endpoints"][1]["cpu"]["worker"]["process"].update(utime_ticks=99))
        with self.assertRaisesRegex(ValueError, "CPU costs"):
            self.load()

    def test_cpu_binary_bound_to_measured_arm(self):
        self.mutate(self.output / "process-endpoints.json",
                    lambda value: value["endpoints"][0]["executables"]["worker"]["opened_executable"].update(sha256="9" * 64))
        with self.assertRaisesRegex(ValueError, "CPU endpoint binary"):
            self.load()

    def test_different_cpu_worker_rejected(self):
        self.mutate(self.output / "process-endpoints.json", lambda value: [row["worker"].update(process_id=99) for row in value["placements"]])
        with self.assertRaisesRegex(ValueError, "this cell's worker"):
            self.load()

    def test_symlink_evidence_rejected(self):
        path = self.output / "launch.status"
        alternate = self.output / "other-status"
        path.rename(alternate)
        path.symlink_to(alternate)
        with self.assertRaisesRegex(ValueError, "regular"):
            self.load()

    def test_counter_pair_qualifies_before_latency(self):
        entries = []
        for cell_id in ("counter-A", "counter-B"):
            entry, output = cell_fixture(self.stage, self.plan, self.binding, cell_id)
            entries.append(self.load(entry, output))
        result = m.validate_counter_pair(entries)
        self.assertTrue(result["accepted"])
        self.assertFalse(result["latency_admitted"])
        self.assertEqual(result["deltas"]["A"]["publications"], 131)
        self.assertEqual(result["deltas"]["B"]["publications"], 131)

    def test_counter_pair_order_is_fixed(self):
        with self.assertRaisesRegex(ValueError, "counter pair"):
            m.validate_counter_pair([{"cell_id": "counter-B"}, {"cell_id": "counter-A"}])


if __name__ == "__main__":
    unittest.main()
