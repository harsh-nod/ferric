import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

import competitive_benchmark as bench


def event(text="", finish=None, usage=None, index=0):
    value = {"choices": [{"index": index, "text": text, "finish_reason": finish}]}
    if usage is not None:
        value["usage"] = usage
    return json.dumps(value).encode()


class StreamingTests(unittest.TestCase):
    def stream(self, count=4):
        return [(event(" one"), 110), (event(" two three four"), 140),
                (event(finish="length", usage={"prompt_tokens": 5, "completion_tokens": count}), 145),
                (b"[DONE]", 150)]

    def test_multi_token_chunk_is_not_an_itl_sample(self):
        result = bench.summarize_stream(self.stream(), 100, 4)
        self.assertEqual(result["ttft_ns"], 10)
        self.assertEqual(result["tpot_ns"], 10)
        self.assertEqual(result["chunk_intervals_ns"], [30])
        self.assertIsNone(result["token_itl_ns"])
        self.assertEqual(result["text"], " one two three four")

    def test_one_token_has_no_tpot(self):
        result = bench.summarize_stream([(event("x", "length", {
            "prompt_tokens": 1, "completion_tokens": 1}), 110), (b"[DONE]", 120)], 100, 1)
        self.assertIsNone(result["tpot_ns"])

    def test_missing_done_rejected(self):
        with self.assertRaisesRegex(ValueError, "incomplete"):
            bench.summarize_stream(self.stream()[:-1], 100, 4)

    def test_missing_usage_rejected(self):
        with self.assertRaisesRegex(ValueError, "usage"):
            bench.summarize_stream([(event("x", "length"), 110), (b"[DONE]", 120)], 100, 4)

    def test_wrong_output_length_rejected(self):
        with self.assertRaisesRegex(ValueError, "output limit"):
            bench.summarize_stream(self.stream(3), 100, 4)

    def test_nonmonotonic_clock_rejected(self):
        stream = self.stream()
        stream[1] = (stream[1][0], 109)
        with self.assertRaisesRegex(ValueError, "clock"):
            bench.summarize_stream(stream, 100, 4)

    def test_error_event_rejected(self):
        with self.assertRaisesRegex(ValueError, "server stream error"):
            bench.summarize_stream([(b'{"error":"unavailable"}', 110)], 100, 4)

    def test_wrong_choice_rejected(self):
        with self.assertRaisesRegex(ValueError, "index"):
            bench.summarize_stream([(event("x", index=1), 110)], 100, 4)

    def test_eos_stop_rejected_for_fixed_length_workload(self):
        with self.assertRaisesRegex(ValueError, "finish reason"):
            bench.summarize_stream([(event("x", finish="stop"), 110)], 100, 4)

    def test_bool_token_count_rejected(self):
        with self.assertRaisesRegex(ValueError, "token count"):
            bench.summarize_stream(self.stream(True), 100, 4)

    def test_inconsistent_total_usage_rejected(self):
        with self.assertRaisesRegex(ValueError, 'total usage'):
            bench.summarize_stream([(event('x', 'length', {'prompt_tokens': 1,
                                     'completion_tokens': 1, 'total_tokens': 3}), 110),
                                    (b'[DONE]', 120)], 100, 1)

    def test_sse_crlf_comments_and_multiline(self):
        raw = io.BytesIO(b': heartbeat\r\nevent: message\r\ndata: {"a":\r\ndata: 1}\r\n\r\n')
        self.assertEqual(list(bench.sse_events(raw, lambda: 10)), [(b'{"a":\n1}', 10)])

    def test_truncated_frame_rejected(self):
        with self.assertRaisesRegex(ValueError, "truncated"):
            list(bench.sse_events(io.BytesIO(b'data: {}\n')))

    def test_oversized_line_rejected(self):
        with self.assertRaisesRegex(ValueError, "oversized"):
            list(bench.sse_events(io.BytesIO(b'x' * (bench.LINE_LIMIT + 1))))

    def test_duplicate_json_and_nonfinite_rejected(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}', b'{"x":1e999}', b'[' * 2000):
            with self.assertRaises(ValueError):
                bench.json_value(raw)

    def test_heartbeat_cannot_extend_absolute_deadline(self):
        ticks = iter([1, 2, 3, 11])
        with self.assertRaisesRegex(ValueError, "deadline"):
            list(bench.sse_events(io.BytesIO(b': heartbeat\n: heartbeat\n'),
                                  lambda: next(ticks), deadline_ns=10))

    def test_partial_chunks_remain_available_after_rejection(self):
        chunks = []
        with self.assertRaisesRegex(ValueError, "incomplete"):
            bench.summarize_stream(self.stream()[:-1], 100, 4, chunks)
        self.assertEqual(len(chunks), 3)
        self.assertEqual(chunks[0]["event"]["choices"][0]["text"], " one")


class ContractTests(unittest.TestCase):
    def test_loopback_only(self):
        for address in ("http://127.0.0.1:18981/v1/completions", "http://[::1]:18982/v1/completions"):
            self.assertEqual(bench.endpoint(address), address)
        for address in ("https://example.org/v1/completions", "http://user@localhost:9/v1/completions",
                        "http://localhost/v1/completions", "http://localhost:9/v1/completions?q=x"):
            with self.assertRaises(ValueError):
                bench.endpoint(address)

    def test_workload_and_duplicate_ids(self):
        value = {"schema": bench.SCHEMA, "model": "Qwen/Qwen3-8B", "requests": [
            {"id": "r1", "prompt": "The capital of France is", "max_tokens": 32}]}
        self.assertEqual(bench.workload(value), value)
        value["requests"].append(value["requests"][0].copy())
        with self.assertRaisesRegex(ValueError, "duplicate"):
            bench.workload(value)

    def test_failures_remain_in_denominator_and_goodput_is_slo_filtered(self):
        ok = {"success": True, "ttft_ns": 10e6, "tpot_ns": 20e6,
              "e2e_ns": 50e6, "usage": {"completion_tokens": 4}}
        slow = {**ok, "ttft_ns": 200e6}
        result = bench.aggregate([ok, slow, {"success": False}], 0, 1000000000, 100, 25)
        self.assertEqual(result["requests"], 3)
        self.assertEqual(result["failed_requests"], 1)
        self.assertEqual(result["output_tokens_per_second"], 8)
        self.assertEqual(result["request_goodput_per_second"], 1)
        self.assertEqual(result["output_goodput_per_second"], 4)
        self.assertFalse(result["all_requests_succeeded"])

    def test_empty_success_metrics_are_null(self):
        result = bench.aggregate([{"success": False}], 0, 10, 100, 25)
        self.assertIsNone(result["ttft_ms"]["p99"])
        self.assertEqual(result["output_tokens_per_second"], 0)


class ArrivalTests(unittest.TestCase):
    def workload(self, count=4):
        return {"model": "Qwen/Qwen3-8B", "requests": [
            {"id": str(index), "prompt": "x", "max_tokens": 2} for index in range(count)]}

    def test_seeded_arrivals_are_frozen_and_bounded(self):
        self.assertEqual(bench.arrival_offsets(4, 'constant', 2, 0), [0, 500000000, 1000000000, 1500000000])
        values = bench.arrival_offsets(20, 'poisson', 10, 13)
        self.assertEqual(values, bench.arrival_offsets(20, 'poisson', 10, 13))
        self.assertNotEqual(values, bench.arrival_offsets(20, 'poisson', 10, 14))
        self.assertEqual(values, sorted(values))
        for rate in (0, None, float('nan'), 1000001, True):
            with self.assertRaises(ValueError):
                bench.arrival_offsets(4, 'constant', rate, 0)
        with self.assertRaisesRegex(ValueError, 'one hour'):
            bench.arrival_offsets(256, 'constant', .001, 0)

    def test_actual_send_delay_is_included_in_ttft_and_e2e(self):
        wire = {'id': 'r', 'success': True, 'started_ns': 130, 'first_text_ns': 150,
                'completed_ns': 190, 'ttft_ns': 20, 'e2e_ns': 60}
        with mock.patch.object(bench.time, 'monotonic_ns', return_value=120), \
                mock.patch.object(bench, 'request_one', return_value=wire) as request:
            record = bench.arrival_request('unused', 'unused', {'id': 'r'}, 1, 100)
        self.assertEqual(record['send_delay_ns'], 30)
        self.assertEqual(record['ttft_ns'], 50)
        self.assertEqual(record['e2e_ns'], 90)
        self.assertEqual(record['wire_ttft_ns'], 20)
        self.assertEqual(request.call_args.kwargs['deadline_ns'], 1000000100)

    def test_expired_arrival_is_not_sent(self):
        with mock.patch.object(bench.time, 'monotonic_ns', return_value=2000000100), \
                mock.patch.object(bench, 'request_one') as request:
            record = bench.arrival_request('unused', 'unused', {'id': 'r'}, 1, 100)
        self.assertEqual(record['failure_kind'], 'queue_timeout')
        request.assert_not_called()

    def test_dispatch_and_pending_are_bounded_without_rescheduling_arrivals(self):
        entered = threading.Event()
        release = threading.Event()
        submissions = []

        def request(_url, _model, item, _timeout, intended, _budget):
            submissions.append(item['id'])
            entered.set()
            self.assertTrue(release.wait(3))
            return bench.arrival_failure(item, intended, time.monotonic_ns(), 'request_error')

        observed = {}
        with mock.patch.object(bench, 'arrival_request', side_effect=request):
            runner = threading.Thread(target=lambda: observed.update(result=bench.run_open_loop(
                'unused', self.workload(4), 1, 1, 2, [0, 0, 0, 0])))
            runner.start()
            self.assertTrue(entered.wait(2))
            # The dispatcher admits the entire zero-offset cohort before waiting
            # for the blocked request. Only one worker can enter the fake client.
            release.set()
            runner.join(3)
            self.assertFalse(runner.is_alive())
        records, origin, _ = observed['result']
        self.assertEqual(submissions, ['0', '1'])
        self.assertEqual([record['failure_kind'] for record in records],
                         ['request_error', 'request_error', 'client_overload', 'client_overload'])
        self.assertEqual([record['intended_arrival_ns'] for record in records], [origin] * 4)

    def test_shared_response_budget_bounds_all_streams(self):
        budget = bench.ResponseBudget(12)
        list(bench.sse_events(io.BytesIO(b'data: x\n\n'), budget=budget))
        with self.assertRaisesRegex(ValueError, 'response-byte budget'):
            list(bench.sse_events(io.BytesIO(b'data: y\n\n'), budget=budget))
        self.assertTrue(budget.exhausted)
        self.assertLessEqual(budget.used, 12)
        with mock.patch.object(bench, 'request_one') as request:
            result = bench.arrival_request('unused', 'unused', {'id': 'r'}, 1, time.monotonic_ns(), budget)
        self.assertEqual(result['failure_kind'], 'client_budget')
        request.assert_not_called()

    def test_open_loop_cli_retains_failed_planned_windows_and_marks_v2(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workload = {'schema': bench.SCHEMA, **self.workload(1)}
            (root / 'workload.json').write_text(json.dumps(workload))
            (root / 'identity.json').write_text(json.dumps({'engine': 'ferric'}))
            record = bench.arrival_failure(workload['requests'][0], 1, 2, 'client_overload')
            argv = ['benchmark', '--endpoint', 'http://127.0.0.1:18980/v1/completions',
                    '--workload', str(root / 'workload.json'), '--identity', str(root / 'identity.json'),
                    '--engine', 'ferric', '--output', str(root / 'report.json'), '--warmups', '0',
                    '--samples', '2', '--ttft-slo-ms', '100', '--tpot-slo-ms', '100',
                    '--arrival', 'constant', '--arrival-rate', '1']
            with mock.patch.object(sys, 'argv', argv), \
                    mock.patch.object(bench, 'run_open_loop', return_value=([record], 1, 3)) as run, \
                    contextlib.redirect_stdout(io.StringIO()):
                bench.main()
            result = json.loads((root / 'report.json').read_bytes())
            self.assertEqual(run.call_count, 2)
            self.assertEqual(result['schema'], 'FerricCompetitiveStreamingRunV2')
            self.assertTrue(result['completed'])
            self.assertFalse(result['qualification'])
            self.assertEqual(len(result['samples']), 2)
            self.assertEqual(result['samples'][0]['metrics']['failure_counts']['client_overload'], 1)

    def test_closed_loop_cli_preserves_v1_default_and_send_semantics(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workload = {'schema': bench.SCHEMA, **self.workload(1)}
            (root / 'workload.json').write_text(json.dumps(workload))
            (root / 'identity.json').write_text(json.dumps({'engine': 'ferric'}))
            record = {'id': '0', 'success': True, 'ttft_ns': 10, 'tpot_ns': 10, 'e2e_ns': 30,
                      'usage': {'completion_tokens': 2}}
            argv = ['benchmark', '--endpoint', 'http://127.0.0.1:18980/v1/completions',
                    '--workload', str(root / 'workload.json'), '--identity', str(root / 'identity.json'),
                    '--engine', 'ferric', '--output', str(root / 'report.json'), '--warmups', '0',
                    '--samples', '1', '--ttft-slo-ms', '100', '--tpot-slo-ms', '100']
            with mock.patch.object(sys, 'argv', argv), \
                    mock.patch.object(bench, 'run_window', return_value=([record], 1, 100)), \
                    contextlib.redirect_stdout(io.StringIO()):
                bench.main()
            result = json.loads((root / 'report.json').read_bytes())
            self.assertEqual(result['schema'], 'FerricCompetitiveStreamingRunV1')
            self.assertEqual(result['ttft_semantics'], 'client-send-to-first-nonempty-text-chunk')
            self.assertNotIn('arrival_offsets_ns', result)


if __name__ == "__main__":
    unittest.main()
