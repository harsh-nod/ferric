import contextlib
import errno
import json
from pathlib import Path
import socket
import threading
import time
import unittest
from unittest import mock

import competitive_benchmark as bench
import test_serve_ferric as serving_fixture


SSE_HEADERS = b'HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n'
ITEM = {'id': 'deadline-test', 'prompt': 'x', 'max_tokens': 2}


def frame(text, finish=None, usage=None):
    value = {'choices': [{'index': 0, 'text': text, 'finish_reason': finish}]}
    if usage is not None:
        value['usage'] = usage
    return b'data: ' + json.dumps(value).encode() + b'\n\n'


COMPLETE = (frame('a') + frame('b', 'length', {'prompt_tokens': 1, 'completion_tokens': 2})
            + b'data: [DONE]\n\n')


class RawServer:
    """One owned, bounded local fixture; never a model server or daemon thread."""

    def __init__(self, script, family=socket.AF_INET, read_request=True):
        self.script = script
        self.read_request = read_request
        self.stop = threading.Event()
        self.accepted = threading.Event()
        self.dripping = threading.Event()
        self.disconnected = threading.Event()
        self.errors = []
        self.request = b''
        self.listener = socket.socket(family, socket.SOCK_STREAM)
        try:
            self.listener.bind(('::1' if family == socket.AF_INET6 else '127.0.0.1', 0))
            self.listener.listen(1)
            self.listener.settimeout(.1)
        except BaseException:
            self.listener.close()
            raise
        self.port = self.listener.getsockname()[1]
        host = '[::1]' if family == socket.AF_INET6 else '127.0.0.1'
        self.url = f'http://{host}:{self.port}/v1/completions'
        self.thread = threading.Thread(target=self.run, name='deadline-test-server')

    def run(self):
        try:
            while not self.stop.is_set():
                try:
                    connection, _ = self.listener.accept()
                    break
                except socket.timeout:
                    continue
            else:
                return
            with connection:
                self.accepted.set()
                connection.settimeout(.1)
                limit = time.monotonic() + 2
                if self.read_request:
                    while time.monotonic() < limit and not self.stop.is_set():
                        try:
                            data = connection.recv(8192)
                        except socket.timeout:
                            continue
                        if not data:
                            return
                        self.request += data
                        if len(self.request) > 1024 * 1024:
                            raise AssertionError('fixture request exceeded bound')
                        head, separator, body = self.request.partition(b'\r\n\r\n')
                        if separator:
                            lengths = [line.split(b':', 1)[1].strip() for line in head.split(b'\r\n')
                                       if line.lower().startswith(b'content-length:')]
                            if len(lengths) == 1 and len(body) >= int(lengths[0]):
                                break
                    else:
                        return
                self.script(self, connection)
        except (BrokenPipeError, ConnectionResetError):
            self.disconnected.set()
        except OSError as error:
            if not self.stop.is_set():
                self.errors.append(error)
        except BaseException as error:
            self.errors.append(error)

    def drip(self, connection, prefix, byte=b' '):
        connection.sendall(prefix)
        self.dripping.set()
        limit = time.monotonic() + 2
        while time.monotonic() < limit and not self.stop.wait(.01):
            connection.sendall(byte)

    def wait_for_close(self, connection):
        limit = time.monotonic() + 2
        while time.monotonic() < limit and not self.stop.is_set():
            try:
                if not connection.recv(8192):
                    self.disconnected.set()
                    return
            except socket.timeout:
                continue

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *args):
        self.stop.set()
        self.listener.close()
        self.thread.join(3)
        if self.thread.is_alive():
            raise AssertionError('fixture thread did not join')
        if self.errors:
            raise AssertionError(self.errors)


@contextlib.contextmanager
def tracked_sockets():
    original = bench.DeadlineSocket
    sockets = []

    def create(*args, **kwargs):
        result = original(*args, **kwargs)
        sockets.append(result)
        return result

    with mock.patch.object(bench, 'DeadlineSocket', side_effect=create):
        yield sockets
    for transport in sockets:
        if transport.fileno() != -1 or transport.selector.get_map() is not None:
            raise AssertionError('request socket or selector leaked')
        if transport.reader is not None and not transport.reader.closed:
            raise AssertionError('HTTP reader leaked')


class DeadlineTests(unittest.TestCase):
    def request(self, server, timeout=.18, **kwargs):
        return bench.request_one(server.url, 'Qwen/Qwen3-8B', ITEM, timeout, **kwargs)

    def test_headers_and_body_stalls_and_drips_obey_same_absolute_deadline(self):
        scripts = {
            'no-status': lambda server, conn: server.wait_for_close(conn),
            'partial-status': lambda server, conn: server.drip(conn, b'HTTP/1.1 200'),
            'partial-header': lambda server, conn: server.drip(conn, SSE_HEADERS + b'X-Drip: '),
            'no-body': lambda server, conn: (conn.sendall(SSE_HEADERS + b'\r\n'), server.wait_for_close(conn)),
            'partial-sse': lambda server, conn: server.drip(conn, SSE_HEADERS + b'\r\ndata: {'),
            'partial-chunk-header': lambda server, conn: server.drip(
                conn, SSE_HEADERS + b'Transfer-Encoding: chunked\r\n\r\n0'),
            'partial-chunk-body': lambda server, conn: server.drip(
                conn, SSE_HEADERS + b'Transfer-Encoding: chunked\r\n\r\n10000\r\ndata: {'),
        }
        for name, script in scripts.items():
            with self.subTest(name=name), RawServer(script) as server, tracked_sockets():
                intended = time.monotonic_ns()
                record = bench.arrival_request(server.url, 'Qwen/Qwen3-8B', ITEM, .18, intended)
                elapsed = (time.monotonic_ns() - intended) / 1e9
                self.assertFalse(record['success'])
                self.assertEqual(record['failure_kind'], 'deadline', record)
                self.assertIn('deadline', record['error'])
                self.assertGreaterEqual(elapsed, .18)
                # A scheduling margin, not a claim that shared Linux is real-time.
                self.assertLess(elapsed, .8)
                self.assertTrue(server.disconnected.wait(.5), 'peer did not observe owned socket close')

    def test_complete_partial_events_are_preserved_before_deadline(self):
        script = lambda server, conn: server.drip(conn, SSE_HEADERS + b'\r\n' + frame('a') + b'data: {')
        with RawServer(script) as server, tracked_sockets():
            record = self.request(server)
        self.assertFalse(record['success'])
        self.assertEqual(len(record['chunks']), 1)
        self.assertEqual(record['chunks'][0]['event']['choices'][0]['text'], 'a')

    def test_content_length_close_and_chunked_streams_keep_existing_metrics(self):
        chunked = b''.join(f'{len(part):x}\r\n'.encode() + part + b'\r\n'
                           for part in (COMPLETE[:17], COMPLETE[17:])) + b'0\r\n\r\n'
        bodies = [SSE_HEADERS + b'Connection: close\r\n\r\n' + COMPLETE,
                  SSE_HEADERS + f'Content-Length: {len(COMPLETE)}\r\n\r\n'.encode() + COMPLETE,
                  SSE_HEADERS + b'Transfer-Encoding: chunked\r\n\r\n' + chunked]
        for response in bodies:
            with self.subTest(response=response[:100]), RawServer(
                    lambda _server, conn: conn.sendall(response)) as server, tracked_sockets():
                record = self.request(server, timeout=2)
                self.assertTrue(record['success'], record)
                self.assertEqual(record['text'], 'ab')
                self.assertEqual(record['usage'], {'prompt_tokens': 1, 'completion_tokens': 2})
                self.assertIsNone(record['token_itl_ns'])
                self.assertEqual(len(record['chunks']), 2)

    def test_expired_request_does_not_allocate_or_connect(self):
        with mock.patch.object(bench, 'DeadlineSocket') as transport:
            record = bench.request_one('http://127.0.0.1:1/v1/completions', 'Qwen/Qwen3-8B',
                                       ITEM, 1, deadline_ns=time.monotonic_ns() - 1)
        self.assertFalse(record['success'])
        self.assertIn('before send', record['error'])
        transport.assert_not_called()

    def test_localhost_has_no_dns_and_redirect_is_not_followed(self):
        with RawServer(lambda _server, conn: conn.sendall(SSE_HEADERS + b'\r\n' + COMPLETE)) as server, \
                mock.patch.object(socket, 'getaddrinfo', side_effect=AssertionError('DNS used')), tracked_sockets():
            result = bench.request_one(server.url.replace('127.0.0.1', 'localhost'),
                                       'Qwen/Qwen3-8B', ITEM, 2)
            self.assertTrue(result['success'], result)
        for code in (302, 307, 429, 500):
            def respond(_server, conn):
                conn.sendall(f'HTTP/1.1 {code} Rejected\r\nLocation: http://example.com/\r\n'
                             'Content-Length: 0\r\n\r\n'.encode())
            with self.subTest(status=code), RawServer(respond) as server, tracked_sockets() as transports:
                result = self.request(server, timeout=2)
                self.assertFalse(result['success'])
                self.assertEqual(result['error'], 'non-success HTTP response')
                self.assertEqual(result['http_status'], code)
                self.assertEqual(len(transports), 1)

    def test_connection_and_write_readiness_use_deadline_without_retry(self):
        transport = bench.DeadlineSocket(socket.AF_INET, time.monotonic_ns() + 1000000000)
        try:
            with mock.patch.object(socket.socket, 'connect_ex', return_value=errno.EINPROGRESS), \
                    mock.patch.object(transport, 'wait_ready', side_effect=TimeoutError('deadline')) as ready:
                with self.assertRaisesRegex(TimeoutError, 'deadline'):
                    transport.connect_loopback(('127.0.0.1', 1))
                ready.assert_called_once_with(bench.selectors.EVENT_WRITE)
            with mock.patch.object(transport, 'wait_ready', side_effect=TimeoutError('deadline')) as ready:
                with self.assertRaisesRegex(TimeoutError, 'deadline'):
                    transport.sendall(b'bounded request')
                ready.assert_called_once_with(bench.selectors.EVENT_WRITE)
        finally:
            transport.dispose()

    def test_real_backpressured_write_stops_at_deadline(self):
        with RawServer(lambda server, _conn: server.stop.wait(2), read_request=False) as server:
            start = time.monotonic_ns()
            transport = bench.DeadlineSocket(socket.AF_INET, start + 180000000)
            try:
                transport.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 4096)
                transport.connect_loopback(('127.0.0.1', server.port))
                with self.assertRaisesRegex(TimeoutError, 'deadline'):
                    transport.sendall(b'x' * (4 * 1024 * 1024))
                self.assertLess((time.monotonic_ns() - start) / 1e9, .8)
            finally:
                transport.dispose()

    def test_cancellation_during_dripped_headers_joins_workers_and_closes_fds(self):
        baseline_threads = {thread.ident for thread in threading.enumerate()}
        fd_dir = Path('/proc/self/fd')
        baseline_fds = len(list(fd_dir.iterdir()))
        for _ in range(3):
            script = lambda server, conn: server.drip(conn, SSE_HEADERS + b'X-Drip: ')
            with RawServer(script) as server, tracked_sockets():
                start = time.monotonic()
                with self.assertRaisesRegex(RuntimeError, 'dispatcher interrupted'):
                    with bench.request_workers(1) as (executor, cancel):
                        future = executor.submit(self.request, server, timeout=10, cancel=cancel)
                        self.assertTrue(server.dripping.wait(2))
                        raise RuntimeError('dispatcher interrupted')
                self.assertLess(time.monotonic() - start, .8)
                self.assertTrue(future.done())
                self.assertEqual(future.result()['failure_kind'], 'client_cancelled')
        self.assertEqual(len(list(fd_dir.iterdir())), baseline_fds)
        self.assertEqual({thread.ident for thread in threading.enumerate()}, baseline_threads)

    def test_cancelled_arrival_does_not_connect_or_look_like_engine_timeout(self):
        cancel = threading.Event()
        cancel.set()
        with mock.patch.object(bench, 'request_one') as request:
            result = bench.arrival_request('unused', 'unused', ITEM, 2, time.monotonic_ns(), cancel=cancel)
        request.assert_not_called()
        self.assertEqual(result['failure_kind'], 'client_cancelled')
        self.assertIsNone(result['send_started_ns'])

    def test_cancelled_body_retains_complete_events_and_explicit_client_failure(self):
        first_consumed = threading.Event()
        original = bench.sse_events

        def observe(*args, **kwargs):
            for event in original(*args, **kwargs):
                yield event
                first_consumed.set()

        script = lambda server, conn: server.drip(conn, SSE_HEADERS + b'\r\n' + frame('a') + b'data: {')
        with RawServer(script) as server, tracked_sockets(), \
                mock.patch.object(bench, 'sse_events', side_effect=observe):
            with bench.request_workers(1) as (executor, cancel):
                future = executor.submit(bench.arrival_request, server.url, 'Qwen/Qwen3-8B', ITEM,
                                         10, time.monotonic_ns(), None, cancel)
                self.assertTrue(first_consumed.wait(2))
                cancel.set()
            record = future.result()
            self.assertFalse(record['success'])
            self.assertEqual(record['failure_kind'], 'client_cancelled')
            self.assertEqual(record['http_status'], 200)
            self.assertEqual(len(record['chunks']), 1)
            self.assertTrue(server.disconnected.wait(.5))

    def test_literal_ipv6_loopback(self):
        try:
            server = RawServer(lambda _server, conn: conn.sendall(SSE_HEADERS + b'\r\n' + COMPLETE),
                               family=socket.AF_INET6)
        except OSError as error:
            self.skipTest(f'IPv6 loopback unavailable: {error}')
        with server, tracked_sockets():
            result = self.request(server, timeout=2)
            self.assertTrue(result['success'], result)

    def test_common_client_accepts_adapter_utf8_and_fixed_usage(self):
        with serving_fixture.running() as (_, server, _), tracked_sockets():
            host, port = server.server_address
            result = bench.request_one(f'http://{host}:{port}/v1/completions', 'Qwen/Qwen3-8B',
                                       dict(ITEM, prompt='split'), 2)
            self.assertTrue(result['success'], result)
            self.assertEqual(result['text'], '\u00e9')
            self.assertEqual(result['usage'], {'prompt_tokens': 2, 'completion_tokens': 2, 'total_tokens': 4})

    def test_client_deadline_disconnect_propagates_adapter_cancellation(self):
        with serving_fixture.running(request_timeout=3) as (_, server, trace), tracked_sockets():
            host, port = server.server_address
            result = bench.arrival_request(f'http://{host}:{port}/v1/completions', 'Qwen/Qwen3-8B',
                                           dict(ITEM, prompt='hold'), .18, time.monotonic_ns())
            self.assertFalse(result['success'])
            self.assertEqual(result['failure_kind'], 'deadline')
            serving_fixture.wait_for(lambda: any(command['op'] == 'cancel'
                                                for command in serving_fixture.commands(trace)))


if __name__ == '__main__':
    unittest.main()
