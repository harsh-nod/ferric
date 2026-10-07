"""CPU-only loopback and fake-child tests; no model, compiler, or GPU invocation."""

import contextlib
import dataclasses
import http.client
import json
import socket
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path

import serve_ferric as serving


FAKE = r'''
import json, sys, time
trace = sys.argv[1]
def emit(event):
    print(json.dumps({'schema':'FerricQwen3TpLiveEventV1','authority':'none', **event}), flush=True)
print(json.dumps({'schema':'FerricQwen3TpBatchSetupV2','authority':'none','model':'Qwen/Qwen3-8B'}), flush=True)
emit({'event':'ready','eos_policy':'fixed output count'})
for line in sys.stdin:
    command = json.loads(line)
    with open(trace, 'a') as output:
        output.write(json.dumps(command)+'\n')
    op = command['op']
    if op == 'shutdown':
        print(json.dumps({'schema':'FerricQwen3TpBatchClosedV2','authority':'none','all_workers_exited':True}), flush=True)
        break
    if op == 'cancel':
        emit({'event':'cancel','request_id':command['request_id'],'status':'cancelled_active'})
        continue
    if op != 'submit':
        continue
    rid = command['request_id']
    emit({'event':'queued','request_id':rid})
    if command['prompt'] == 'reject':
        emit({'event':'rejected','request_id':rid,'reason':'overloaded'})
        continue
    emit({'event':'admission','request_id':rid,'prompt_token_count':2})
    if command['prompt'] == 'hold':
        continue
    count = command['new_tokens']
    pieces = [[0xc3],[0xa9]] if command['prompt'] == 'split' else [[120] for _ in range(count)]
    tokens = list(range(count))
    raw = sum(pieces, [])
    for index, piece in enumerate(pieces):
        emit({'event':'token','request_id':rid,'index':index,'token':tokens[index],'decoded_bytes':piece})
    emit({'event':'request','request_id':rid,'state':'Completed','prompt_token_count':2,
          'generated_tokens':tokens,'generated_utf8_bytes':([0] if command['prompt'] == 'bad' else raw)})
'''


def payload(prompt="hello", count=2):
    return {"model": "Qwen/Qwen3-8B", "prompt": prompt, "max_tokens": count,
            "temperature": 0, "seed": 0, "ignore_eos": True, "stream": True,
            "stream_options": {"include_usage": True}, "n": 1}


def wait_for(predicate, seconds=3):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.01)
    raise AssertionError("condition did not become true")


@contextlib.contextmanager
def running(request_timeout=1):
    with tempfile.TemporaryDirectory(prefix="ferric-http-test-") as directory:
        trace = Path(directory) / "commands.jsonl"
        config = serving.Config((sys.executable, "-u", "-c", FAKE, str(trace)), max_inflight=2,
                                ready_timeout_seconds=3, request_timeout_seconds=request_timeout,
                                shutdown_timeout_seconds=2)
        backend = serving.Backend(config)
        server = None
        thread = None
        try:
            backend.wait_ready()
            server = serving.Server(0, backend)
            thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.02})
            thread.start()
            yield backend, server, trace
        finally:
            if server:
                server.shutdown()
            backend.close()
            if server:
                server.server_close()
            if thread:
                thread.join(timeout=3)
                assert not thread.is_alive()
            assert backend.process.poll() is not None
            assert all(not thread.is_alive() for thread in backend.threads)


def commands(trace):
    if not trace.exists():
        return []
    return [json.loads(line) for line in trace.read_text().splitlines()]


def request(server, value, path="/v1/completions"):
    connection = http.client.HTTPConnection(*server.server_address, timeout=5)
    try:
        connection.request("POST", path, json.dumps(value), {"Content-Type": "application/json"})
        response = connection.getresponse()
        return response.status, response.read().decode()
    finally:
        connection.close()


class ContractTests(unittest.TestCase):
    def test_only_explicit_supported_generation_contract_is_accepted(self):
        value = payload()
        self.assertEqual(serving.validate_request(json.dumps(value).encode(), value["model"]), ("hello", 2))
        for key, invalid in [("prompt", []), ("prompt", ""), ("prompt", "a" * 32769),
                             ("prompt", "\ud800"), ("temperature", 1), ("temperature", False),
                             ("seed", 1), ("n", 2), ("ignore_eos", False), ("stream", False),
                             ("max_tokens", 0), ("max_tokens", True), ("max_tokens", 8193),
                             ("stream_options", {}), ("top_p", 1), ("model", "other")]:
            with self.subTest(key=key, invalid=str(invalid)[:40]):
                with self.assertRaises(serving.RequestError):
                    serving.validate_request(json.dumps(value | {key: invalid}).encode(), value["model"])

    def test_duplicate_nonfinite_and_deep_json_are_rejected(self):
        for body in [b'{"prompt":"a","prompt":"b"}', b'{"temperature":NaN}',
                     b"[" * 2000 + b"]" * 2000]:
            with self.assertRaises(serving.RequestError):
                serving.validate_request(body, "Qwen/Qwen3-8B")

    def test_config_requires_literal_absolute_argv_and_bounds(self):
        for argv in [(), ("python3",), (1,), ("/bin/echo", "bad\0arg")]:
            with self.assertRaises(ValueError):
                serving.Config(argv)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "config.json"
            path.write_text(json.dumps({"schema": "FerricLiveHttpConfigV1", "argv": ["/bin/echo", "$(false)"]}))
            self.assertEqual(serving.Config.load(path).argv, ("/bin/echo", "$(false)"))
            path.write_text(json.dumps({"schema": "FerricLiveHttpConfigV1", "argv": ["/bin/echo"], "shell": True}))
            with self.assertRaises(ValueError):
                serving.Config.load(path)
        for value in [0, 65, True]:
            with self.assertRaises(ValueError):
                serving.Config(("/bin/echo",), max_inflight=value)
        for value in [0, float("nan"), float("inf"), 3601]:
            with self.assertRaises(ValueError):
                serving.Config(("/bin/echo",), request_timeout_seconds=value)


class LoopbackTests(unittest.TestCase):
    def test_health_and_streamed_utf8_bytes_usage_and_final_match(self):
        with running() as (backend, server, trace):
            self.assertEqual(server.server_address[0], "127.0.0.1")
            connection = http.client.HTTPConnection(*server.server_address, timeout=5)
            connection.request("GET", "/health")
            response = connection.getresponse()
            self.assertEqual(response.status, 200)
            self.assertTrue(json.loads(response.read())["ready"])
            connection.close()
            status, body = request(server, payload("split"))
            self.assertEqual(status, 200)
            self.assertIn("data: [DONE]", body)
            chunks = [json.loads(line[6:]) for line in body.splitlines()
                      if line.startswith("data: ") and line != "data: [DONE]"]
            text = "".join(choice["text"] for chunk in chunks for choice in chunk["choices"])
            self.assertEqual(text, "\u00e9")
            self.assertEqual(chunks[-2]["choices"][0]["finish_reason"], "length")
            self.assertEqual(chunks[-1]["usage"], {"prompt_tokens": 2, "completion_tokens": 2, "total_tokens": 4})
            self.assertEqual(len(commands(trace)), 1)
            self.assertFalse(backend.pending)

    def test_sustained_requests_keep_one_child_and_monotone_ids(self):
        with running() as (backend, server, trace):
            pid = backend.process.pid
            for _ in range(40):
                status, body = request(server, payload(count=1))
                self.assertEqual(status, 200)
                self.assertIn("data: [DONE]", body)
                self.assertEqual(backend.process.pid, pid)
            submits = [item for item in commands(trace) if item["op"] == "submit"]
            self.assertEqual([item["request_id"] for item in submits], list(range(1, 41)))

    def test_unsupported_requests_never_reach_child(self):
        with running() as (_, server, trace):
            status, _ = request(server, payload() | {"stream": False})
            self.assertEqual(status, 400)
            status, _ = request(server, payload(), "/v1/chat/completions")
            self.assertEqual(status, 404)
            self.assertFalse(commands(trace))
    def test_backend_overload_is_explicit_http_429(self):
        with running(request_timeout=3) as (backend, server, trace):
            first = backend.submit("hold", 2)
            second = backend.submit("hold", 2)
            status, body = request(server, payload())
            self.assertEqual(status, 429)
            self.assertIn("capacity", body)
            backend.release(first, cancel=True)
            backend.release(second, cancel=True)
            wait_for(lambda: len([c for c in commands(trace) if c["op"] == "cancel"]) == 2)

    def test_backend_rejection_is_explicit_http_429(self):
        with running() as (_, server, _):
            status, body = request(server, payload("reject"))
            self.assertEqual(status, 429)
            self.assertIn("overloaded", body)

    def test_client_disconnect_cancels_before_first_token(self):
        with running(request_timeout=3) as (_, server, trace):
            stream = socket.create_connection(server.server_address, timeout=3)
            body = json.dumps(payload("hold")).encode()
            stream.sendall(b"POST /v1/completions HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: "
                           + str(len(body)).encode() + b"\r\n\r\n" + body)
            wait_for(lambda: any(c["op"] == "submit" for c in commands(trace)))
            stream.close()
            wait_for(lambda: any(c["op"] == "cancel" for c in commands(trace)))

    def test_request_deadline_sends_cancellation(self):
        with running(request_timeout=0.15) as (_, server, trace):
            status, body = request(server, payload("hold"))
            self.assertEqual(status, 504)
            self.assertIn("deadline", body)
            wait_for(lambda: any(c["op"] == "cancel" for c in commands(trace)))

    def test_final_byte_mismatch_never_emits_success_or_done(self):
        with running() as (_, server, _):
            status, body = request(server, payload("bad"))
            self.assertEqual(status, 200)
            self.assertIn("final completion does not match", body)
            self.assertNotIn("data: [DONE]", body)
            self.assertNotIn('"finish_reason":"length"', body)

    def test_slow_consumer_queue_is_bounded_and_cancels(self):
        with running() as (backend, _, trace):
            pending = backend.submit("hold", 1)
            wait_for(lambda: any(c["op"] == "submit" for c in commands(trace)))
            for _ in range(serving.EVENT_QUEUE + 1):
                backend._event({"schema": serving.EVENT_SCHEMA, "authority": "none", "event": "queued",
                                "request_id": pending.request_id})
            self.assertLessEqual(pending.events.qsize(), serving.EVENT_QUEUE)
            self.assertEqual(pending.events.get(timeout=1)["event"], "adapter_error")
            wait_for(lambda: any(c["op"] == "cancel" for c in commands(trace)))

    def test_bounded_body_and_duplicate_lengths_are_rejected(self):
        with running() as (_, server, trace):
            for headers, expected in [(f"Content-Length: {serving.MAX_BODY + 1}\r\n", b" 413 "),
                                      ("Content-Length: 2\r\nContent-Length: 2\r\n", b" 400 "),
                                      ("Content-Length: 2\r\nTransfer-Encoding: chunked\r\n", b" 400 ")]:
                stream = socket.create_connection(server.server_address, timeout=3)
                stream.sendall(("POST /v1/completions HTTP/1.1\r\nHost: localhost\r\n" + headers + "\r\n{}").encode())
                self.assertIn(expected, stream.recv(4096).split(b"\r\n")[0])
                stream.close()
            self.assertFalse(commands(trace))


class ChildLifecycleTests(unittest.TestCase):
    def test_shutdown_interrupts_a_long_initial_ready_wait(self):
        backend = serving.Backend(serving.Config(
            (sys.executable, "-u", "-c", "import time; time.sleep(60)"),
            ready_timeout_seconds=60, shutdown_timeout_seconds=0.1))
        stopping = threading.Event()
        stopping.set()
        try:
            started = time.monotonic()
            with self.assertRaises(serving.RequestError):
                backend.wait_ready(stopping)
            self.assertLess(time.monotonic() - started, 2)
        finally:
            backend.close()
            self.assertIsNotNone(backend.process.poll())
            self.assertTrue(all(not thread.is_alive() for thread in backend.threads))

    def test_shutdown_reaps_an_owned_child_that_does_not_read_stdin(self):
        script = FAKE.split("for line in sys.stdin:")[0] + "\ntime.sleep(60)\n"
        backend = serving.Backend(serving.Config((sys.executable, "-u", "-c", script, "/unused"),
            ready_timeout_seconds=3, shutdown_timeout_seconds=0.1))
        try:
            backend.wait_ready()
        finally:
            started = time.monotonic()
            backend.close()
            self.assertLess(time.monotonic() - started, 5)
            self.assertIsNotNone(backend.process.poll())
            self.assertTrue(all(not thread.is_alive() for thread in backend.threads))

    def test_malformed_and_oversized_child_output_fail_readiness_and_reap(self):
        for line in ["'{}'", f"'x' * {serving.MAX_LINE + 1}"]:
            backend = serving.Backend(serving.Config(
                (sys.executable, "-u", "-c", f"import time; print({line}, flush=True); time.sleep(60)"),
                ready_timeout_seconds=3, shutdown_timeout_seconds=0.1))
            try:
                with self.assertRaises(serving.RequestError):
                    backend.wait_ready()
            finally:
                backend.close()
                self.assertIsNotNone(backend.process.poll())
                self.assertTrue(all(not thread.is_alive() for thread in backend.threads))


if __name__ == "__main__":
    unittest.main()
