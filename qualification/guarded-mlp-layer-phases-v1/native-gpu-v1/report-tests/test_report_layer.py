"""Pure reducer fixtures, not native capsules or a substitute for strict admission."""
import hashlib
import json
import unittest
from unittest.mock import patch

import report_layer as R


def encoded(value):
    return (json.dumps(value, separators=(",", ":"), allow_nan=False) + "\n").encode()


def metric():
    return dict(layers=36, phase_ns=[10, 20, 30, 20, 10, 10], layer_body_ns=100,
                paired_mlp_phase_ns=[2] * 7, paired_mlp_body_ns=14)


def fixture():
    # Minimal authenticated-reducer shapes with genuinely contained synthetic times.
    # Full policy/numerical/custody admission belongs to the unchanged strict retainer.
    policy = dict(schema="FerricReadiness40Position5BankScopedWarmCensusTailPolicyV4",
        session=[1] * 32, worker_sha256=[2] * 32, transcript_sha256=[3] * 32,
        native_closed=True, completed_forwards=40, generated_tokens=[], capture_positions=[0, 5, 16, 39])
    counts = {"bank": [726, 2, 726, 725], "layers": [10, 2, 10, 3], "tail": [8, 2, 8, 3]}
    policy["counts"] = {
        group: dict(zip(("before_calls", "full_discoveries", "after_calls", "generation_probes"),
                        (38 * n for n in counts[scope])))
        for scope, group in zip(R.SCOPES, ("banks", "layers", "tails"))}
    old_rows, new_rows = [], []
    for p in range(40):
        measured, layer = None, None
        phases = [1] * 9
        if p >= 2:
            measured = {
                scope: {key: dict(calls=calls, elapsed_ns=1) for key, calls in zip(R.CALLBACKS, counts[scope])}
                for scope in R.SCOPES}
            measured["bank_guarded_body_ns"] = 6
            layer = metric()
            phases = [1, 1, 1, 8, 120, 8, 1, 1, 1]
        old_rows.append(dict(position=p, measured=measured))
        new_rows.append(dict(position=p, phase_ns=phases, forward_body_ns=sum(phases), layer_metrics=layer))
    callbacks = dict(schema="FerricReadiness40TailCurrentnessDurationsV1", instrumented=True,
        policy_sha256=list(hashlib.sha256(encoded(policy)).digest()),
        **{k: policy[k] for k in ("session", "worker_sha256", "transcript_sha256")},
        forwards=old_rows, host_elapsed_nanoseconds=True, bank_guarded_body_includes_callbacks=True,
        numerical_acceptance=False, performance_claim=False, execution_authority=False)
    forward = dict(schema="FerricReadiness40ForwardLayerDurationsV2", instrumented=True,
        policy_sha256=callbacks["policy_sha256"],
        currentness_record_sha256=list(hashlib.sha256(encoded(callbacks)).digest()),
        **{k: policy[k] for k in ("session", "worker_sha256", "transcript_sha256")},
        phase_order=list(R.PHASES), layer_stage_order=list(R.LAYER_STAGES),
        paired_mlp_stage_order=list(R.PAIRED_STAGES), forwards=new_rows, host_elapsed_nanoseconds=True,
        disjoint_phases=True, currentness_durations_nested=True, layer_durations_nested=True,
        paired_mlp_durations_nested=True, gpu_timing=False, numerical_acceptance=False,
        performance_claim=False, execution_authority=False)
    return policy, callbacks, forward


def admitted(records, raw=None):
    policy, callbacks, forward = records
    wire = b"".join(encoded(v) for v in records) if raw is None else raw
    bodies = {"tail_layer/native/child-stderr.bin": wire}
    checked = dict(policy=dict(policy_record=policy, diagnostic_record=callbacks,
                               forward_record=forward, file=R.pin(wire)),
                   ordinary=dict(currentness_duration_diagnostic=callbacks, forward_phase_diagnostic=forward))
    return bodies, checked


def report_fixture(records=None):
    bodies, checked = admitted(fixture() if records is None else records)
    rows, _ = R.worker_records(bodies, checked)
    cursor = 0

    def span(elapsed):
        nonlocal cursor
        value = dict(start_ns=cursor, end_ns=cursor + elapsed, elapsed_ns=elapsed)
        cursor += elapsed
        return value

    timeline = dict(source_preparation=span(3), spawn_to_setup_seal=span(5), forwards=[])
    for row in rows:
        body = row["forward_body_ns"]
        values = {k: span(n) for k, n in zip(R.PARENT, (3, body - 2, 4))}
        timeline["forwards"].append(dict(position=row["position"], generation=row["position"] + 1,
                                         **values, elapsed_ns=body + 5))
    timeline["close_and_retirement"] = span(7)
    timeline["postcheck_and_ordinary_publication"] = span(11)
    timeline["total_ns"] = cursor
    checked["timing"] = dict(timeline=timeline, disjoint_spans=124)
    bodies["tail_layer/native/host-timing.json"] = encoded(dict(timeline=timeline))
    parent = R.parent_timeline(bodies, checked, rows)
    subsets, nested = R.reduce_rows(rows)
    return dict(forwards=rows, parent=parent, subsets=subsets, warm_nested=nested,
                warm_layers=R.reduce_layers(rows))


class LayerReportTests(unittest.TestCase):
    def test_exact_three_levels_and_warm_census(self):
        report = report_fixture()
        layer = report["warm_layers"]
        self.assertEqual((layer["measured_forwards"], layer["measured_layer_returns"]), (38, 1368))
        self.assertEqual(layer["layer_body_ns"], 3800)
        self.assertEqual(layer["paired_mlp_body_ns"], 532)
        self.assertEqual(layer["forward_layers_ns"], 4560)
        self.assertEqual(layer["forward_layers_outside_closed_ns"], 760)
        self.assertEqual(layer["closed_mlp_outside_paired_ns"], 608)
        self.assertEqual(report["subsets"]["first_use"]["body_ns"], 18)
        self.assertEqual(report["subsets"]["warm"]["body_ns"], 5396)
        self.assertEqual(layer["phase_percent"]["prefix"], "20.000000")

    def test_cold_metrics_absent_and_warm_metrics_required(self):
        for position, value in ((0, metric()), (2, None)):
            with self.subTest(position=position):
                records = fixture()
                records[2]["forwards"][position]["layer_metrics"] = value
                with self.assertRaises(RuntimeError):
                    R.worker_records(*admitted(records))

    def test_both_stage_orders_are_exact(self):
        for key in ("layer_stage_order", "paired_mlp_stage_order"):
            with self.subTest(key=key):
                records = fixture()
                records[2][key].reverse()
                with self.assertRaises(RuntimeError):
                    R.worker_records(*admitted(records))

    def test_closed_and_paired_sums_are_independently_checked(self):
        for field in ("layer_body_ns", "paired_mlp_body_ns"):
            with self.subTest(field=field):
                value = metric()
                value[field] += 1
                with self.assertRaises(RuntimeError):
                    R.layer_metrics(value, 120, 4)

    def test_each_same_process_containment_is_checked(self):
        paired = metric()
        paired["paired_mlp_phase_ns"] = [5] * 7
        paired["paired_mlp_body_ns"] = 35
        for value, enclosing, callbacks in ((paired, 120, 4), (metric(), 99, 4), (metric(), 120, 101)):
            with self.subTest(enclosing=enclosing, callbacks=callbacks):
                with self.assertRaises(RuntimeError):
                    R.layer_metrics(value, enclosing, callbacks)

    def test_exact_integer_extent_overflow_and_census(self):
        for value in (True, 1.0, -1, 1 << 64):
            with self.subTest(value=value):
                bad = metric()
                bad["phase_ns"][0] = value
                with self.assertRaises(RuntimeError):
                    R.layer_metrics(bad, 120, 4)
        for value in (35, 37, True):
            bad = metric()
            bad["layers"] = value
            with self.assertRaises(RuntimeError):
                R.layer_metrics(bad, 120, 4)
        with self.assertRaises(RuntimeError):
            R.total([R.HOUR_NS, 1], R.HOUR_NS)
        bad = metric()
        bad["paired_mlp_phase_ns"].pop()
        with self.assertRaises(RuntimeError):
            R.layer_metrics(bad, 120, 4)

    def test_original_lf_hash_canonical_and_whole_file_custody(self):
        records = fixture()
        records[2]["currentness_record_sha256"][0] ^= 1
        with self.assertRaises(RuntimeError):
            R.worker_records(*admitted(records))
        records = fixture()
        wire = b"".join(encoded(v) for v in records)
        for changed in (wire + b"{}\n", wire[:-1], wire.replace(b'{"schema":', b'{ "schema":', 1)):
            with self.subTest(length=len(changed)):
                with self.assertRaises(RuntimeError):
                    R.worker_records(*admitted(records, changed))
        bodies, checked = admitted(records)
        checked["policy"]["file"] = R.pin(wire[:-1])
        with self.assertRaises(RuntimeError):
            R.worker_records(bodies, checked)

    def test_signed_parent_differences_are_not_clamped(self):
        report = report_fixture()
        self.assertEqual(report["parent"]["disjoint_spans"], 124)
        self.assertEqual(report["forwards"][14]["parent_minus_body_ns"],
                         dict(flush_to_frame_read=-2, whole_forward=5))
        self.assertEqual(report["subsets"]["warm"]["signed_parent_minus_body_ns"],
                         dict(flush_to_frame_read=-76, whole_forward=190))

    def test_no_cross_level_or_callback_double_counting(self):
        report = report_fixture()
        layer = report["warm_layers"]
        self.assertEqual(sum(layer["phase_ns"].values()), layer["layer_body_ns"])
        self.assertEqual(sum(layer["paired_mlp_phase_ns"].values()), layer["paired_mlp_body_ns"])
        self.assertEqual(layer["layer_callbacks_ns"], 152)
        self.assertNotEqual(layer["layer_body_ns"] + layer["paired_mlp_body_ns"], layer["layer_body_ns"])
        self.assertEqual(report["subsets"]["warm"]["phase_ns"]["layers"], 4560)
        text = R.svg(report).decode()
        self.assertEqual(text.count("<rect x="), 9 + 6 + 7)
        self.assertIn("Separate scales; do not add the three levels.", text)
        self.assertIn("Already inside closed mlp_retired_seal", text)

    def test_zero_duration_nested_levels_remain_valid_and_render(self):
        records = fixture()
        for position in range(2, 40):
            records[2]["forwards"][position]["layer_metrics"] = dict(layers=36, phase_ns=[0] * 6,
                layer_body_ns=0, paired_mlp_phase_ns=[0] * 7, paired_mlp_body_ns=0)
            for part in records[1]["forwards"][position]["measured"]["layers"].values():
                part["elapsed_ns"] = 0
        records[2]["currentness_record_sha256"] = list(hashlib.sha256(encoded(records[1])).digest())
        report = report_fixture(records)
        self.assertEqual(report["warm_layers"]["layer_body_ns"], 0)
        self.assertEqual(set(report["warm_layers"]["phase_percent"].values()), {"0.000000"})
        self.assertNotIn("NaN", R.svg(report).decode())

    def test_decimal_half_even_display(self):
        self.assertEqual(R.display(5, 2, 0), "2")
        self.assertEqual(R.display(7, 2, 0), "4")
        self.assertEqual(R.display(-5, 2, 0), "-2")

    def test_unbound_original_inputs_refuse_before_io(self):
        with patch.object(R, "ARCHIVE", None), patch.object(R, "RECEIPT", None), patch.object(R, "TERMINAL", None):
            with self.assertRaisesRegex(RuntimeError, "observed archive"):
                R.archive_inputs()


if __name__ == "__main__":
    unittest.main()
