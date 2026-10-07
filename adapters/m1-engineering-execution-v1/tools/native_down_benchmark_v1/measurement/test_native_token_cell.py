import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location("native_token_cell", Path(__file__).with_name("native_token_cell.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


WIDTH_SPEC = importlib.util.spec_from_file_location("width_fixtures", Path(__file__).parent.parent / "test_prefill_width.py")
width = importlib.util.module_from_spec(WIDTH_SPEC)
WIDTH_SPEC.loader.exec_module(width)


def fixture(mode="latency", arm="A"):
    return width.fixture(arm, mode)


def counter_lines(spec, setup):
    return width.counters(spec, setup)


class Events:
    def __init__(self, _runner, _policy):
        self.batches = self.dispatches = self.emission = 0

    def validate(self, _value):
        pass


class Parity:
    def __init__(self, controller, _reference):
        self.controller = controller
        self.complete, self.tokens, self.decoded = True, 128, b"ab"

    def send(self, value, deadline):
        self.controller.send(value, deadline)

    def next(self, deadline):
        return self.controller.next(deadline)


def transcript(spec, setup, closed):
    count, warmups = m.shape(spec)
    mechanism = m.expected_mechanism(spec["arm"])
    values = [setup, {"schema": width.frozen.EVENT, "authority": "none", "event": "ready",
                     "emission_started_ns": 0, "clock": "monotonic_ns_since_live_start",
                     "eos_policy": "fixed output count", "context_tokens": 8192, "physical_pages": 512}]
    for index in range(count):
        events, _ = width.request_stream(spec["arm"])
        arrival = (index + 1) * 10_000_000_000
        ttft = 800_000_000
        tpot = 50_000_000 if spec["arm"] == "A" else 45_000_000
        chunks = mechanism["prefill_chunks"]
        terminal = 150 + (chunks - 1) * 100
        def stamp(value):
            if value <= 3:
                return arrival + value - 1
            if value < terminal:
                return arrival + 2 + (value - 3) * (ttft - 2) // (terminal - 3)
            return arrival + ttft + (value - terminal) * tpot // 100
        for event in events:
            event["request_id"] = index + 1
            event["name"] = "warmup-" + str(index) if index < warmups else "measured-" + str(index - warmups)
            for key in ("emission_started_ns", "arrival_ns", "queued_ns", "admitted_ns", "started_ns", "completed_ns"):
                if key in event:
                    event[key] = stamp(event[key])
            if event["event"] == "batch":
                event["tick"] += index * mechanism["model_batches"]
                event["batch_id"] += index * mechanism["model_batches"]
            if event["event"] == "request":
                event["output_timestamps_ns"] = [stamp(value) for value in event["output_timestamps_ns"]]
                event["decode_intervals_ns"] = [tpot] * 127
                event["ttft_ns"], event["tpot_ns"] = ttft, tpot
            values.append(event)
    final_stamp = values[-1]["emission_started_ns"]
    values.extend([{"schema": width.frozen.EVENT, "authority": "none", "event": "draining",
                    "reason": "command", "emission_started_ns": final_stamp + 1},
                   {"schema": width.frozen.EVENT, "authority": "none", "event": "stopped", "reason": "drained",
                    "batches": count * mechanism["model_batches"], "emission_started_ns": final_stamp + 2}, closed])
    return values


class Runner:
    COMMAND, EVENT, MAX_STREAM = width.frozen.COMMAND, width.frozen.EVENT, 8 * 1024**2
    Events = width.frozen.Events
    integer = staticmethod(width.frozen.integer)

    @staticmethod
    def decode(raw):
        return json.loads(raw)

    @staticmethod
    def identity(path, **_kwargs):
        return {"sha256": m.ledger.sha(Path(path).read_bytes())}

    @staticmethod
    def save(path, value):
        Path(path).write_bytes(m.ledger.canonical(value))


class Controller:
    def __init__(self, events, output=None, stderr=b""):
        self.events = copy.deepcopy(events)
        self.commands = []
        self.output = output
        if output is not None:
            for name in ("stdin.raw", "stdout.raw"):
                (output / name).write_bytes(b"")
            (output / "stderr.raw").write_bytes(stderr)

    def next(self, _deadline):
        value = self.events.pop(0)
        if self.output is not None:
            with (self.output / "stdout.raw").open("ab") as target:
                target.write(m.ledger.canonical(value) + b"\n")
        return value

    def send(self, value, _deadline):
        self.commands.append(value)
        if self.output is not None:
            with (self.output / "stdin.raw").open("ab") as target:
                target.write(m.ledger.canonical(value) + b"\n")

    def finish(self, _deadline):
        m.require(not self.events, "unexpected leftover fake events")

    def close(self):
        return {"cleanup_ok": True, "child_reaped": True, "owned_descendants_absent": True,
                "returncode": 0, "errors": [], "term_sent": False, "kill_sent": False}


def campaign_fixture():
    cells, counters = [], []
    for index, (mode, arm) in enumerate([("counters", "A"), ("counters", "B")]
                                      + [("latency", arm) for arm in ["A", "B", "B", "A"] * 3]):
        spec, setup, closed = fixture(mode, arm)
        events = transcript(spec, setup, closed)
        result = m.consume(Controller(events), spec, runner=Runner,
                           legacy=types.SimpleNamespace(Events=Events), evidence=types.SimpleNamespace(StreamParity=Parity), deadline=0)
        result.update(schema="FerricNativeDownTokenCellResultR1", spec_sha256=m.ledger.digest(spec),
                      accepted=True, raw_replay_passed=True, instrumented=mode == "counters", latency_admitted=mode == "latency",
                      started_ns=(index + 1) * 1000, finished_ns=(index + 1) * 1000 + 100,
                      raw_sha256={"stdin.raw": "1" * 64, "stdout.raw": m.ledger.digest({"index": index}), "stderr.raw": "2" * 64})
        if mode == "counters":
            raw = b"\n".join(m.ledger.canonical(v) for v in counter_lines(spec, setup)) + b"\n"
            result["mechanism"] = m.counter_replay(raw, spec, setup)
        entry = {"cell_id": "cell-" + str(index), "spec": spec, "result": result,
                 "completion": {"accepted": True, "model_stable": True, "input_files_stable": True},
                 "outer": {"status": 0, "cleanup_ok": True, "child_reaped": True, "errors": [],
                           "term_sent": False, "kill_sent": False, "termination_reason": "completed",
                           "postflight": {"accepted": True}}}
        rebind_outer(entry)
        (counters if mode == "counters" else cells).append(entry)
    profiles = {arm: m.profile_for_spec(fixture(arm=arm)[0]) for arm in ("A", "B")}
    plan = {"schema": "FerricNativeDownChangePlanR1", "change_id": "native-whole-program-r1", "gates": m.ledger.GATES,
            "reference_sha256": m.ledger.digest(fixture()[0]["reference"]),
            "workload_sha256": "f" * 64, "client_sha256": "0" * 64,
            "timing_semantics": "native-ingress-v1", "profiles": profiles,
            "allowed_profile_differences": m.ledger.differences(profiles["A"], profiles["B"]),
            "workload": {"input_tokens": 128, "output_tokens": 128, "context_tokens": 8192, "concurrency": 1,
                         "tensor_parallel": 1, "greedy": True, "prefix_caching": False, "speculation": False},
            "mechanism": {arm: m.expected_mechanism(arm) for arm in ("A", "B")}}
    return plan, cells, counters


def rebind_outer(entry):
    retained_hash = lambda value: m.ledger.sha((json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode())
    entry["completion"]["cell_result_sha256"] = retained_hash(entry["result"])
    entry["outer"]["completion_sha256"] = retained_hash(entry["completion"])


class NativeTests(unittest.TestCase):
    def test_latency_exact_six_requests(self):
        spec, setup, closed = fixture()
        controller = Controller(transcript(spec, setup, closed))
        result = m.consume(controller, spec, runner=Runner, legacy=types.SimpleNamespace(Events=Events),
                           evidence=types.SimpleNamespace(StreamParity=Parity), deadline=0)
        self.assertEqual(result["batches"], 786)
        self.assertEqual(result["dispatches"], 512418)
        self.assertEqual(result["exact_output_tokens_checked"], 768)
        self.assertEqual(len(result["summary"]["requests"]), 4)
        self.assertEqual(result["summary"]["warmups_excluded"], 2)
        self.assertEqual(len(controller.commands), 7)

    def test_correctness_single_request_is_not_latency_cohort(self):
        spec, setup, closed = fixture("correctness", "B")
        result = m.consume(Controller(transcript(spec, setup, closed)), spec, runner=Runner,
                           legacy=types.SimpleNamespace(Events=Events), evidence=types.SimpleNamespace(StreamParity=Parity), deadline=0)
        self.assertEqual(result["batches"], 131)
        self.assertEqual(result["dispatches"], 89975)
        self.assertEqual(result["exact_output_tokens_checked"], 128)

    def test_wrong_backend_positive_id_rejected(self):
        spec, setup, _ = fixture()
        setup["token_program"]["backend"] = "native-boundary-fences-v1"
        with self.assertRaisesRegex(ValueError, "backend"):
            m.check_setup(setup, spec)

    def test_missing_identity_check_rejected(self):
        spec, setup, _ = fixture()
        del setup["token_program"]["backend_identity_checked"]
        with self.assertRaisesRegex(ValueError, "backend"):
            m.check_setup(setup, spec)

    def test_packed_down_composition_rejected(self):
        spec, setup, _ = fixture()
        setup["packed_down_mode"] = "candidate"
        with self.assertRaisesRegex(ValueError, "unrelated"):
            m.check_setup(setup, spec)

    def test_parallel_kv_required(self):
        spec, setup, _ = fixture()
        setup["kv_copy_mode"] = "baseline"
        with self.assertRaisesRegex(ValueError, "composition"):
            m.check_setup(setup, spec)

    def test_controller_hash_checked(self):
        spec, setup, _ = fixture()
        setup["controller_sha256"] = "f" * 64
        with self.assertRaisesRegex(ValueError, "setup drift"):
            m.check_setup(setup, spec)

    def test_closed_count_checked(self):
        spec, setup, closed = fixture()
        closed["rank_dispatch_counts"] = [350844]
        with self.assertRaisesRegex(ValueError, "incomplete"):
            m.check_closed(closed, setup, 526266, spec)

    def test_one_request_limit_required_for_counter(self):
        spec, _, _ = fixture("counters")
        spec["argv"][spec["argv"].index("--max-batches") + 1] = "810"
        with self.assertRaisesRegex(ValueError, "max-batches"):
            m.shape(spec)

    def test_duplicate_worker_selector_rejected(self):
        spec, _, _ = fixture()
        spec["argv"].extend(["--worker", "/foreign"])
        with self.assertRaisesRegex(ValueError, "worker"):
            m.shape(spec)

    def test_timing_profile_cannot_enable_counter_selector(self):
        spec, _, _ = fixture()
        spec["argv"].extend(["--token-program-backend", "ordered64-groups-v1"])
        with self.assertRaisesRegex(ValueError, "dedicated"):
            m.shape(spec)

    def test_both_counter_mechanisms(self):
        for arm in ("A", "B"):
            spec, setup, _ = fixture("counters", arm)
            m.check_setup(setup, spec)
            raw = b"\n".join(m.ledger.canonical(v) for v in counter_lines(spec, setup)) + b"\n"
            result = m.counter_replay(raw, spec, setup)
            self.assertEqual(result["delta"]["publications"], 131)
            self.assertEqual(result["delta"]["retirement_signals"], 89972 if arm == "B" else 85400)
            self.assertFalse(result["latency_admitted"])

    def bad_counter(self, mutate, regex):
        spec, setup, _ = fixture("counters", "B")
        rows = counter_lines(spec, setup)
        mutate(rows)
        raw = b"\n".join(m.ledger.canonical(v) for v in rows) + b"\n"
        with self.assertRaisesRegex(ValueError, regex):
            m.counter_replay(raw, spec, setup)

    def test_counter_last_signal_only_cannot_qualify(self):
        self.bad_counter(lambda rows: rows[1]["counters"].update(retirement_signals=127), "mechanism")

    def test_counter_ordinary_grouping_cannot_masquerade(self):
        self.bad_counter(lambda rows: rows[1]["counters"].update(publications=1397, final_waits=1397), "mechanism")

    def test_counter_start_must_be_zero(self):
        self.bad_counter(lambda rows: rows[0]["counters"].update(staging_ns=1), "zero")

    def test_counter_scope_and_pid_are_bound(self):
        self.bad_counter(lambda rows: rows[1].update(process_id=99), "identity")

    def test_counter_unknown_field_rejected(self):
        self.bad_counter(lambda rows: rows[1]["counters"].update(extra=1), "exact nonnegative")

    def test_counter_missing_close_rejected(self):
        self.bad_counter(lambda rows: rows.pop(), "pair")

    def test_prefill_phase_shape_and_idle_transitions_are_exact(self):
        for key, field, value in (("prefill", "executions", 7), ("prefill", "dispatches_per_execution", 612),
                                  ("prefill", "dynamic_slots", 215), ("decode_c1", "executions", 128),
                                  ("decode_c1", "dispatches_per_execution", 613), ("decode_c1", "dynamic_slots", 216)):
            self.bad_counter(lambda rows: rows[1]["program_phases"][key].update({field: value}), "phase and idle")
        for key in ("registrations", "releases"):
            self.bad_counter(lambda rows: rows[1]["program_phases"].update({key: 1}), "phase and idle")

    def test_prefill_start_phase_rejects_boolean_zero(self):
        self.bad_counter(lambda rows: rows[0]["program_phases"]["prefill"].update(executions=False), "phase and idle")

    def test_prefill_program_dispatches_exclude_head_singletons(self):
        self.bad_counter(lambda rows: rows[1]["counters"].update(dispatches=87711, retirement_signals=87711), "mechanism")

    def test_prefill_schema_and_scope_cannot_be_decode_only(self):
        self.bad_counter(lambda rows: rows[1].update(schema="FerricTokenProgramCountersV1"), "identity/scope")
        self.bad_counter(lambda rows: rows[1].update(scope="successful token executions only; excludes registration, ordinary prefill and readback"), "identity/scope")

    def test_prefill_metadata_is_required_for_both_widths(self):
        for arm in ("A", "B"):
            spec, setup, _ = fixture(arm=arm)
            del setup["prefill_program"]
            with self.assertRaisesRegex(ValueError, "selected-width"):
                m.check_setup(setup, spec)

    def test_prefill_metadata_does_not_accept_boolean_for_integer(self):
        spec, setup, _ = fixture(arm="B")
        setup["prefill_program"]["output_rows"] = [[], [True]]
        with self.assertRaisesRegex(ValueError, "selected-width metadata"):
            m.check_setup(setup, spec)

    def test_boundary_fence_composition_is_rejected_for_all_modes(self):
        for mode in ("latency", "counters"):
            for arm in ("A", "B"):
                spec, _, _ = fixture(mode, arm)
                spec["argv"].extend(["--token-program-fence-mode", "system-boundaries-v1"])
                with self.assertRaisesRegex(ValueError, "boundary-fence composition"):
                    m.shape(spec)

    def test_counter_selector_is_forbidden_on_dedicated_prefill_binary(self):
        spec, _, _ = fixture("counters", "B")
        spec["argv"][1:1] = ["--token-program-backend", "native-whole-program-v1"]
        with self.assertRaisesRegex(ValueError, "dedicated"):
            m.shape(spec)

    def test_diagnostic_profile_cannot_be_latency_manifest(self):
        spec, _, _ = fixture("counters")
        with self.assertRaisesRegex(ValueError, "uninstrumented"):
            m.profile_for_spec(spec)

    def test_native_binary_and_backend_difference_is_explicit(self):
        a, _, _ = fixture(arm="A")
        b, _, _ = fixture(arm="B")
        differences = m.ledger.differences(m.profile_for_spec(a), m.profile_for_spec(b))
        self.assertEqual(differences, ["/configuration/argv"])

    def test_control_counter_selector_is_leading_closed_and_unique(self):
        for mutate in (
            lambda spec: spec["argv"].__setitem__(2, "auto"),
            lambda spec: spec["argv"].__setitem__(2, "ordered64-groups-v1"),
            lambda spec: spec["argv"].extend(spec["argv"][1:3]),
            lambda spec: spec["argv"].__setitem__(slice(1, 3), []),
            lambda spec: spec["argv"].extend(["--token-program-backend", "native-whole-program-v1"]),
        ):
            spec, _, _ = fixture("counters", "A")
            mutate(spec)
            with self.assertRaisesRegex(ValueError, "width|selector|dedicated"):
                m.shape(spec)

    def test_added_prefill_counter_bytes_need_not_equal_decode_only_control(self):
        plan, cells, counters = campaign_fixture()
        mechanism = counters[1]["result"]["mechanism"]
        mechanism["end"]["counters"]["kernarg_initialized_bytes"] -= 1
        mechanism["delta"]["kernarg_initialized_bytes"] = mechanism["end"]["counters"]["kernarg_initialized_bytes"]
        rebind_outer(counters[1])
        self.assertEqual(m.evaluate_campaign(plan, cells, counters)["status"], "experimental-gates-passed")

    def test_controller_is_fixed_within_each_arm(self):
        plan, cells, counters = campaign_fixture()
        entry = cells[1]
        entry["spec"]["controller"]["sha256"] = "7" * 64
        entry["result"]["setup"]["controller_sha256"] = "7" * 64
        entry["result"]["spec_sha256"] = m.ledger.digest(entry["spec"])
        rebind_outer(entry)
        with self.assertRaisesRegex(ValueError, "predeclared native profile"):
            m.evaluate_campaign(plan, cells, counters)

    def test_run_cell_replays_and_retains_raw_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec, setup, closed = fixture()
            for key in ("controller", "worker"):
                path = root / key
                path.write_bytes(key.encode())
                old_path, old_hash = spec[key]["path"], spec[key]["sha256"]
                spec[key] = {"path": str(path), "sha256": m.ledger.sha(key.encode())}
                spec["argv"] = [str(path) if v == old_path else spec[key]["sha256"] if v == old_hash else v for v in spec["argv"]]
            setup.update(controller_sha256=spec["controller"]["sha256"], worker_sha256=spec["worker"]["sha256"],
                         running_worker_sha256=[spec["worker"]["sha256"]])
            def factory(_runner):
                return lambda _argv, output, _deadline: Controller(transcript(spec, setup, closed), output)
            phases = []
            def admitted(phase, _setup):
                phases.append(phase)
                return {"accepted": True}
            with mock.patch.object(m.lifecycle, "controller_class", factory):
                result = m.run_cell(spec, root / "cell", runner=Runner, legacy=types.SimpleNamespace(Events=Events),
                                    counter=types.SimpleNamespace(clean_controller=factory),
                                    evidence=types.SimpleNamespace(StreamParity=Parity), admission=admitted)
            self.assertTrue(result["accepted"])
            self.assertTrue(result["latency_admitted"])
            self.assertEqual(phases, ["preflight", "active", "active", "postflight"])
            self.assertEqual(set(result["raw_sha256"]), {"stdin.raw", "stdout.raw", "stderr.raw"})
            self.assertTrue((root / "cell/result.json").exists())

    def test_native_campaign_all_gates(self):
        plan, cells, counters = campaign_fixture()
        result = m.evaluate_campaign(plan, cells, counters)
        self.assertEqual(result["status"], "experimental-gates-passed")
        self.assertEqual(result["median_ttft_gain_percent"], 0)
        self.assertAlmostEqual(result["median_tpot_gain_percent"], 10)
        self.assertEqual(result["arms"]["A"]["requests"], 24)
        self.assertIn("native ingress", result["latency_semantics"])
        self.assertEqual(result["promotion_scope"], "explicit-down-selector-native32-backend-only")
        self.assertFalse(result["ordered64_comparison_performed"])
        self.assertFalse(result["vendor_comparison_performed"])
        self.assertFalse(result["default_promotion"])

    def test_native_campaign_predeclared_profile_required(self):
        plan, cells, counters = campaign_fixture()
        plan["profiles"]["A"]["artifacts"]["controller"] = "9" * 64
        plan["allowed_profile_differences"] = m.ledger.differences(plan["profiles"]["A"], plan["profiles"]["B"])
        with self.assertRaisesRegex(ValueError, "predeclared native profile"):
            m.evaluate_campaign(plan, cells, counters)

    def test_native_campaign_no_raw_reuse(self):
        plan, cells, counters = campaign_fixture()
        cells[1]["result"]["raw_sha256"]["stdout.raw"] = cells[0]["result"]["raw_sha256"]["stdout.raw"]
        rebind_outer(cells[1])
        with self.assertRaisesRegex(ValueError, "reused"):
            m.evaluate_campaign(plan, cells, counters)

    def test_native_campaign_no_cell_overlap(self):
        plan, cells, counters = campaign_fixture()
        cells[1]["result"]["started_ns"] = cells[0]["result"]["started_ns"]
        rebind_outer(cells[1])
        with self.assertRaisesRegex(ValueError, "overlap"):
            m.evaluate_campaign(plan, cells, counters)

    def test_native_campaign_requires_counter_pair(self):
        plan, cells, counters = campaign_fixture()
        with self.assertRaisesRegex(ValueError, "two-arm"):
            m.evaluate_campaign(plan, cells, counters[:1])

    def test_native_campaign_no_http_relabel(self):
        plan, cells, counters = campaign_fixture()
        plan["timing_semantics"] = "http-text-chunk-v1"
        with self.assertRaisesRegex(ValueError, "native.*ingress|native timing"):
            m.evaluate_campaign(plan, cells, counters)

    def test_native_campaign_missing_replay_fails(self):
        plan, cells, counters = campaign_fixture()
        cells[0]["result"]["raw_replay_passed"] = False
        rebind_outer(cells[0])
        with self.assertRaisesRegex(ValueError, "raw-replayed"):
            m.evaluate_campaign(plan, cells, counters)

    def test_empty_owned_transcript_has_real_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stderr.raw"
            path.write_bytes(b"")
            self.assertEqual(m.ledger.sha(m.retained_bytes(path, 1024)), m.ledger.sha(b""))

    def test_symlink_transcript_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stderr.raw"
            target = Path(directory) / "other"
            target.write_bytes(b"")
            path.symlink_to(target)
            with self.assertRaisesRegex(ValueError, "regular"):
                m.retained_bytes(path, 1024)

    def test_outer_failure_cannot_promote_inner_success(self):
        plan, cells, counters = campaign_fixture()
        cells[0]["outer"].update(status=125, cleanup_ok=False, errors=["postflight resource loss"])
        with self.assertRaisesRegex(ValueError, "final outer"):
            m.evaluate_campaign(plan, cells, counters)

    def test_outer_cannot_belong_to_other_result(self):
        plan, cells, counters = campaign_fixture()
        cells[0]["completion"]["cell_result_sha256"] = "9" * 64
        with self.assertRaisesRegex(ValueError, "bind this exact"):
            m.evaluate_campaign(plan, cells, counters)


if __name__ == "__main__":
    unittest.main()
