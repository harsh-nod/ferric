#!/usr/bin/env python3
"""Host-test fixture only. It never loads a model, artifact, worker or GPU."""
import json
import os
from pathlib import Path
import socket
import signal
import subprocess
import sys
import time


def main():
    config = json.loads(Path(sys.argv[sys.argv.index("--benchmark-control") + 1]).read_bytes())
    identity = config["identity"]
    mode = os.environ.get("FERRIC_REPLICA_TEST_MODE", "ok") if identity["replica_id"] == "replica-00" else "ok"
    if mode == "early_exit":
        return 7
    if mode == "no_ready":
        time.sleep(60)
    if mode == "descendant":
        child = subprocess.Popen([sys.executable, "-c", "import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(60)"])
        Path(sys.argv[sys.argv.index("--benchmark-control") + 1]).with_name("descendant.pid").write_text(str(child.pid))
    now = lambda: time.clock_gettime_ns(time.CLOCK_MONOTONIC_RAW)
    stream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    stream.connect(config["socket_path"])
    reader = stream.makefile("rb")

    def send(schema, **extra):
        stream.sendall(json.dumps({"schema": schema, "authority": "none", "identity": identity,
                                   "pid": os.getpid(), **extra}).encode() + b"\n")

    if mode == "wrong_nonce":
        identity["nonce"] = "f" * 64
    if mode == "oversized":
        stream.sendall(b"x" * 5000)
        time.sleep(60)
    ready = now()
    send("FerricReplicaReadyV1", ready_ns=ready)
    if mode == "duplicate_ready":
        send("FerricReplicaReadyV1", ready_ns=ready)
    start = json.loads(reader.readline())
    received = now()
    epoch = start["epoch_ns"]
    while now() < epoch:
        time.sleep(0.0001)
    if mode == "late_start":
        time.sleep(0.2)
    started = now()
    send("FerricReplicaStartedV1", ready_ns=float(ready) if mode == "float_ready" else ready, start_received_ns=received,
         epoch_ns=float(epoch) if mode == "float_start_epoch" else epoch, started_ns=started, lateness_ns=started - epoch)
    if mode in ("no_close", "descendant"):
        time.sleep(60)
    send("FerricReplicaClosedV1", epoch_ns=float(epoch) if mode == "float_close_epoch" else epoch + (1 if mode == "wrong_close" else 0), closed_ns=now())
    ack = json.loads(reader.readline())
    assert ack["schema"] == "FerricReplicaCloseAckV1" and ack["epoch_ns"] == epoch
    if mode == "trailing_partial":
        stream.sendall(b"{")
    if mode == "duplicate_after_close":
        time.sleep(0.02)
        send("FerricReplicaClosedV1", epoch_ns=epoch, closed_ns=now())
    print(json.dumps({"fixture": "host-control-only", "model_inference": False}))
    reader.close()
    stream.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
