#!/usr/bin/env python3
"""Loopback-only streaming completions adapter for Ferric's live engineering runner.

The JSON config pins an explicit argv, never a shell command. This adapter does
not confer protected execution authority or performance qualification. It only
supports raw-prompt, fixed-length greedy streaming completions.
"""

from __future__ import annotations

import argparse
import codecs
import dataclasses
import http.server
import json
import os
import queue
import select
import signal
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path


COMMAND_SCHEMA = "FerricQwen3TpLiveCommandV1"
EVENT_SCHEMA = "FerricQwen3TpLiveEventV1"
MAX_BODY = 256 * 1024
MAX_LINE = 2 * 1024 * 1024
MAX_PROMPT = 32 * 1024
EVENT_QUEUE = 64
COMMAND_QUEUE = 64


class RequestError(Exception):
    def __init__(self, status: int, message: str):
        super().__init__(message)
        self.status = status


def strict_json(data: bytes):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError(f"duplicate field: {key}")
            result[key] = value
        return result

    try:
        return json.loads(data, object_pairs_hook=pairs,
                          parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))
    except RecursionError as error:
        raise ValueError("JSON nesting exceeds decoder bound") from error


@dataclasses.dataclass(frozen=True)
class Config:
    argv: tuple[str, ...]
    model: str = "Qwen/Qwen3-8B"
    max_inflight: int = 32
    ready_timeout_seconds: float = 300
    request_timeout_seconds: float = 600
    shutdown_timeout_seconds: float = 30

    def __post_init__(self):
        if (not self.argv or len(self.argv) > 128
                or any(not isinstance(arg, str) or not arg or len(arg) > 4096 or "\0" in arg
                       for arg in self.argv) or not os.path.isabs(self.argv[0])):
            raise ValueError("argv requires an absolute executable and 1..128 bounded literal arguments")
        if not isinstance(self.model, str) or not 1 <= len(self.model) <= 256:
            raise ValueError("invalid model identity")
        if type(self.max_inflight) is not int or not 1 <= self.max_inflight <= 64:
            raise ValueError("max_inflight must be in 1..64")
        for value in (self.ready_timeout_seconds, self.request_timeout_seconds,
                      self.shutdown_timeout_seconds):
            if type(value) not in (int, float) or not 0 < value <= 3600:
                raise ValueError("timeouts must be finite and in (0,3600]")

    @classmethod
    def load(cls, path: Path):
        with path.open("rb") as stream:
            data = stream.read(65_537)
        if len(data) > 65_536:
            raise ValueError("config exceeds 64 KiB")
        value = strict_json(data)
        fields = {field.name for field in dataclasses.fields(cls)}
        if (not isinstance(value, dict) or value.pop("schema", None) != "FerricLiveHttpConfigV1"
                or set(value) - fields or not isinstance(value.get("argv"), list)):
            raise ValueError("wrong config schema or fields")
        return cls(**(value | {"argv": tuple(value["argv"])}))


@dataclasses.dataclass
class Pending:
    request_id: int
    max_tokens: int
    events: queue.Queue = dataclasses.field(default_factory=lambda: queue.Queue(EVENT_QUEUE))


class Backend:
    """One owned child/session, bounded transport, and generational request routing."""

    def __init__(self, config: Config):
        self.config = config
        self.lock = threading.Lock()
        self.pending: dict[int, Pending] = {}
        self.next_id = 1
        self.commands = queue.Queue(COMMAND_QUEUE)
        self.ready = threading.Event()
        self.stopping = threading.Event()
        self.failure: str | None = None
        self.setup: dict | None = None
        self.closed_receipt = False
        self.stderr_tail = bytearray()
        self.process = subprocess.Popen(config.argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, start_new_session=True, bufsize=65_536)
        self.threads = [threading.Thread(target=target, daemon=True, name=name)
                        for target, name in [(self._writer, "ferric-command-writer"),
                                             (self._reader, "ferric-event-reader"),
                                             (self._stderr, "ferric-stderr-reader")]]
        for thread in self.threads:
            thread.start()

    def wait_ready(self, stopping: threading.Event | None = None):
        deadline = time.monotonic() + self.config.ready_timeout_seconds
        while not self.ready.wait(0.1):
            if stopping is not None and stopping.is_set():
                raise RequestError(503, "adapter stopped during Ferric initialization")
            if time.monotonic() >= deadline:
                raise RequestError(503, "Ferric ready deadline exceeded")
        if self.failure or self.stopping.is_set():
            raise RequestError(503, self.failure or "Ferric is stopping")

    def healthy(self):
        return self.ready.is_set() and not self.failure and not self.stopping.is_set()

    @staticmethod
    def _wake(pending: Pending, message: str):
        while True:
            try:
                pending.events.get_nowait()
            except queue.Empty:
                break
        pending.events.put_nowait({"event": "adapter_error", "detail": message})

    def _fail(self, message: str):
        with self.lock:
            if self.failure is None:
                self.failure = message
            for pending in self.pending.values():
                self._wake(pending, self.failure)
            self.ready.set()

    def _enqueue(self, command: dict):
        command = {"schema": COMMAND_SCHEMA} | command
        self.commands.put_nowait(command)

    def submit(self, prompt: str, max_tokens: int) -> Pending:
        with self.lock:
            if not self.healthy():
                raise RequestError(503, self.failure or "Ferric is not ready")
            if len(self.pending) >= self.config.max_inflight:
                raise RequestError(429, "adapter request capacity exhausted")
            request_id = self.next_id
            self.next_id += 1
            if request_id >= 2**64:
                raise RequestError(503, "request identity exhausted")
            pending = Pending(request_id, max_tokens)
            try:
                self._enqueue({"op": "submit", "request_id": request_id, "name": f"http-{request_id}",
                               "prompt": prompt, "new_tokens": max_tokens})
            except queue.Full as error:
                raise RequestError(429, "adapter command queue exhausted") from error
            self.pending[request_id] = pending
            return pending

    def release(self, pending: Pending, cancel: bool):
        failed = False
        with self.lock:
            self.pending.pop(pending.request_id, None)
            if cancel and not self.stopping.is_set() and not self.failure:
                try:
                    self._enqueue({"op": "cancel", "request_id": pending.request_id})
                except queue.Full:
                    failed = True
        if failed:
            self._fail("cannot deliver bounded cancellation command")

    def _writer(self):
        try:
            assert self.process.stdin is not None
            fd = self.process.stdin.fileno()
            os.set_blocking(fd, False)
            while True:
                command = self.commands.get()
                if command is None:
                    return
                data = memoryview(json.dumps(command, ensure_ascii=True, separators=(",", ":")).encode() + b"\n")
                deadline = time.monotonic() + 10
                while data:
                    if time.monotonic() >= deadline:
                        raise TimeoutError("child stdin write deadline")
                    if not select.select([], [fd], [], 0.1)[1]:
                        continue
                    try:
                        data = data[os.write(fd, data):]
                    except BlockingIOError:
                        continue
        except (OSError, ValueError, TimeoutError) as error:
            self._fail(f"child command transport failed: {error}")

    def _event(self, event: dict):
        if not isinstance(event, dict) or event.get("authority") != "none":
            raise ValueError("unrecognized child event authority")
        schema = event.get("schema")
        if schema == "FerricQwen3TpBatchSetupV2":
            if self.setup is not None or event.get("model") != self.config.model:
                raise ValueError("duplicate setup or model mismatch")
            self.setup = event
            return
        if schema == "FerricQwen3TpBatchClosedV2":
            self.closed_receipt = event.get("all_workers_exited") is True
            return
        if schema != EVENT_SCHEMA:
            raise ValueError("unknown child event schema")
        kind = event.get("event")
        if kind == "ready":
            if self.ready.is_set() or self.setup is None or event.get("eos_policy") != "fixed output count":
                raise ValueError("invalid live readiness receipt")
            self.ready.set()
            return
        if kind in ("batch", "draining", "stopped"):
            return
        if kind not in ("queued", "admission", "token", "request", "rejected", "cancel"):
            raise ValueError("unknown child live event")
        request_id = event.get("request_id")
        if type(request_id) is not int or not 0 < request_id < 2**64:
            raise ValueError("unbound child request event")
        overloaded = None
        with self.lock:
            pending = self.pending.get(request_id)
            if pending is None:
                return  # A disconnected client already relinquished this ID.
            try:
                pending.events.put_nowait(event)
            except queue.Full:
                self._wake(pending, "client event queue exhausted")
                overloaded = pending
        if overloaded:
            self.release(overloaded, cancel=True)

    def _reader(self):
        try:
            assert self.process.stdout is not None
            while line := self.process.stdout.readline(MAX_LINE + 1):
                if len(line) > MAX_LINE or not line.endswith(b"\n"):
                    raise ValueError("child output frame exceeded its bound or was truncated")
                self._event(strict_json(line))
            if not self.stopping.is_set():
                self._fail("Ferric event stream closed")
        except (OSError, ValueError, TypeError) as error:
            self._fail(f"child event transport failed: {error}")

    def _stderr(self):
        assert self.process.stderr is not None
        while data := self.process.stderr.read(4096):
            self.stderr_tail.extend(data)
            del self.stderr_tail[:-65_536]

    def close(self):
        if self.stopping.is_set():
            return
        self.stopping.set()
        self._fail("adapter shutting down")
        while True:
            try:
                self.commands.get_nowait()
            except queue.Empty:
                break
        self._enqueue({"op": "shutdown"})
        self.commands.put_nowait(None)
        try:
            self.process.wait(timeout=self.config.shutdown_timeout_seconds)
        except subprocess.TimeoutExpired:
            self._signal_group(signal.SIGTERM)
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self._signal_group(signal.SIGKILL)
                self.process.wait(timeout=3)
        # A controller exit without its close receipt cannot establish that its
        # workers exited. The isolated process group belongs only to this child.
        if not self.closed_receipt:
            self._signal_group(signal.SIGKILL)
        for thread in self.threads:
            thread.join(timeout=12)
        for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
            if stream is not None:
                stream.close()

    def _signal_group(self, sig):
        try:
            os.killpg(self.process.pid, sig)
        except ProcessLookupError:
            pass


def validate_request(body: bytes, model: str):
    try:
        value = strict_json(body)
    except (ValueError, UnicodeError) as error:
        raise RequestError(400, f"invalid JSON: {error}") from error
    allowed = {"model", "prompt", "max_tokens", "temperature", "seed", "ignore_eos",
               "stream", "stream_options", "n"}
    if not isinstance(value, dict) or set(value) - allowed:
        raise RequestError(400, "unsupported completion fields")
    if value.get("model") != model:
        raise RequestError(404, "unknown model")
    prompt = value.get("prompt")
    try:
        prompt_bytes = len(prompt.encode("utf-8")) if isinstance(prompt, str) else 0
    except UnicodeError as error:
        raise RequestError(400, "prompt contains invalid Unicode") from error
    if not 0 < prompt_bytes <= MAX_PROMPT:
        raise RequestError(400, "prompt must be a raw nonempty string of at most 32 KiB")
    count = value.get("max_tokens")
    if type(count) is not int or not 1 <= count <= 8192:
        raise RequestError(400, "max_tokens must be in 1..8192 and fit the backend context")
    if (type(value.get("temperature", 0)) not in (int, float) or value.get("temperature", 0) != 0
            or type(value.get("seed", 0)) is not int or value.get("seed", 0) != 0
            or type(value.get("n", 1)) is not int or value.get("n", 1) != 1
            or value.get("ignore_eos") is not True or value.get("stream") is not True
            or value.get("stream_options") != {"include_usage": True}):
        raise RequestError(400, "only greedy seed0 n1 ignore_eos streaming with usage is supported")
    return prompt, count


class Server(http.server.ThreadingHTTPServer):
    daemon_threads = False
    allow_reuse_address = False
    request_queue_size = 64

    def __init__(self, port: int, backend: Backend):
        self.backend = backend
        self.slots = threading.BoundedSemaphore(backend.config.max_inflight + 8)
        super().__init__(("127.0.0.1", port), Handler)
        self.timeout = 0.2

    def process_request(self, request, client_address):
        if not self.slots.acquire(blocking=False):
            request.close()
            return
        try:
            super().process_request(request, client_address)
        except BaseException:
            self.slots.release()
            raise

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            self.slots.release()


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "FerricLoopbackEngineering/1"

    def setup(self):
        super().setup()
        self.connection.settimeout(10)

    def log_message(self, *_args):
        pass

    def _json(self, status: int, value):
        data = json.dumps(value, separators=(",", ":")).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(data)
        self.close_connection = True

    def do_GET(self):
        if self.path != "/health":
            return self._json(404, {"error": {"message": "unknown endpoint"}})
        backend = self.server.backend
        return self._json(200 if backend.healthy() else 503,
                          {"ready": backend.healthy(), "authority": "none",
                           "model": backend.config.model})

    def _disconnected(self):
        if not select.select([self.connection], [], [], 0)[0]:
            return False
        try:
            return self.connection.recv(1, socket.MSG_PEEK | socket.MSG_DONTWAIT) == b""
        except BlockingIOError:
            return False

    def _sse(self, value):
        self.wfile.write(b"data: " + json.dumps(value, ensure_ascii=False,
                                               separators=(",", ":")).encode() + b"\n\n")
        self.wfile.flush()

    def do_POST(self):
        pending = None
        started = False
        completed = False
        backend = self.server.backend
        try:
            if self.path != "/v1/completions":
                raise RequestError(404, "only /v1/completions is supported")
            if self.headers.get("Origin") is not None:
                raise RequestError(403, "browser-origin requests are not supported")
            lengths = self.headers.get_all("Content-Length", [])
            if len(lengths) != 1 or self.headers.get("Transfer-Encoding") is not None:
                raise RequestError(400, "one Content-Length and no transfer encoding are required")
            try:
                length = int(lengths[0])
            except ValueError as error:
                raise RequestError(400, "invalid Content-Length") from error
            if not 0 < length <= MAX_BODY:
                raise RequestError(413, "request body exceeds bound")
            if self.headers.get_content_type() != "application/json":
                raise RequestError(415, "Content-Type must be application/json")
            body = self.rfile.read(length)
            if len(body) != length:
                raise RequestError(400, "truncated body")
            prompt, count = validate_request(body, backend.config.model)
            pending = backend.submit(prompt, count)
            deadline = time.monotonic() + backend.config.request_timeout_seconds
            decoder = codecs.getincrementaldecoder("utf-8")("replace")
            raw = bytearray()
            tokens = []
            prompt_count = None
            completion_id = f"ferric-{backend.process.pid}-{pending.request_id}"
            created = int(time.time())

            def chunk(text, finish=None, usage=None):
                return {"id": completion_id, "object": "text_completion", "created": created,
                        "model": backend.config.model, "choices": [{"index": 0, "text": text,
                        "logprobs": None, "finish_reason": finish}], "usage": usage}

            while not completed:
                if time.monotonic() >= deadline:
                    raise RequestError(504, "request deadline exceeded")
                if self._disconnected():
                    return
                try:
                    event = pending.events.get(timeout=0.1)
                except queue.Empty:
                    continue
                kind = event["event"]
                if kind == "adapter_error":
                    raise RequestError(503, event["detail"])
                if kind == "rejected":
                    raise RequestError(429 if event.get("reason") == "overloaded" else 400,
                                       f"Ferric rejected request: {event.get('reason')}")
                if kind == "admission":
                    prompt_count = event.get("prompt_token_count")
                    if type(prompt_count) is not int or prompt_count <= 0:
                        raise RequestError(502, "invalid prompt token count")
                elif kind == "token":
                    token = event.get("token")
                    data = event.get("decoded_bytes")
                    if (prompt_count is None or type(event.get("index")) is not int
                            or event.get("index") != len(tokens)
                            or type(token) is not int or not 0 <= token < 2**32
                            or not isinstance(data, list) or len(data) > 131_072
                            or any(type(byte) is not int or not 0 <= byte <= 255 for byte in data)
                            or len(tokens) >= count or len(raw) + len(data) > 131_072):
                        raise RequestError(502, "invalid token stream")
                    raw.extend(data)
                    tokens.append(token)
                    text = decoder.decode(bytes(data), final=False)
                    if not started:
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Cache-Control", "no-cache")
                        self.send_header("Connection", "close")
                        self.end_headers()
                        started = True
                    self._sse(chunk(text))
                elif kind == "request":
                    if (event.get("state") != "Completed" or len(tokens) != count
                            or not isinstance(event.get("generated_tokens"), list)
                            or any(type(token) is not int for token in event["generated_tokens"])
                            or not isinstance(event.get("generated_utf8_bytes"), list)
                            or any(type(byte) is not int for byte in event["generated_utf8_bytes"])
                            or event.get("generated_tokens") != tokens
                            or event.get("generated_utf8_bytes") != list(raw)
                            or type(event.get("prompt_token_count")) is not int
                            or event.get("prompt_token_count") != prompt_count or not started):
                        raise RequestError(502, "final completion does not match streamed tokens/bytes/counts")
                    tail = decoder.decode(b"", final=True)
                    if tail:
                        self._sse(chunk(tail))
                    self._sse(chunk("", "length"))
                    self._sse({"id": completion_id, "object": "text_completion", "created": created,
                               "model": backend.config.model, "choices": [], "usage": {
                                   "prompt_tokens": prompt_count, "completion_tokens": len(tokens),
                                   "total_tokens": prompt_count + len(tokens)}})
                    self.wfile.write(b"data: [DONE]\n\n")
                    self.wfile.flush()
                    completed = True
        except RequestError as error:
            value = {"error": {"message": str(error), "type": "ferric_engineering_error"}}
            try:
                if started:
                    self._sse(value)
                else:
                    self._json(error.status, value)
            except OSError:
                pass
        except OSError:
            pass  # A disconnected client relinquishes its request below.
        finally:
            self.close_connection = True
            if pending is not None:
                backend.release(pending, cancel=not completed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--port", required=True, type=int)
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        parser.error("port must be in 1..65535; bind is fixed to 127.0.0.1")
    backend = None
    server = None
    stopping = threading.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, lambda *_: stopping.set())
    try:
        backend = Backend(Config.load(args.config))
        backend.wait_ready(stopping)
        server = Server(args.port, backend)
        print(json.dumps({"schema": "FerricLiveHttpReadyV1", "authority": "none",
                          "url": f"http://127.0.0.1:{args.port}",
                          "pid": backend.process.pid, "setup": backend.setup}), flush=True)
        while not stopping.is_set() and backend.healthy():
            server.handle_request()
        return 0 if stopping.is_set() else 1
    except (ValueError, OSError, RequestError) as error:
        print(f"Ferric loopback adapter failed: {error}", file=sys.stderr)
        return 1
    finally:
        if backend is not None:
            backend.close()
        if server is not None:
            server.server_close()


if __name__ == "__main__":
    raise SystemExit(main())
