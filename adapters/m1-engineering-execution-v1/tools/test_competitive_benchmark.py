import io
import json
import unittest

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
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}'):
            with self.assertRaises(ValueError):
                bench.json_value(raw)


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


if __name__ == "__main__":
    unittest.main()
