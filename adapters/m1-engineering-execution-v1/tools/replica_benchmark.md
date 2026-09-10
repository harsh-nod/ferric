# Same-Host Replica Cohorts

`replica_benchmark.py` is a launch/control foundation, not a performance or model
qualification tool. A completed launch reports `unvalidated-complete` and
`model_parity_qualified: false`. A separate verifier must admit every controller,
worker, artifact, workload, token, clock, and teardown record before computing
rates. No GPU run is part of its host-only tests.

## Fixed Allocation Experiment

The launcher takes an ordered roster of exactly eight distinct physical device
IDs. `1xTP8`, `4xTP2`, and `8xTP1` partition that entire roster without sharing
cards. All layouts execute the same eight globally named requests, assigned
round-robin to replicas. Each request has the frozen five-token prompt
`The capital of France is` and eight greedy BF16 output tokens. All requests
arrive at zero, prefix caching is disabled, and cancellation is absent. The
global workload hash and total 64 output tokens are layout-independent.

The initial allocation experiment fixes `host-staged-v1` collectives across all
layouts. Optional kernel/runtime flags are recorded verbatim; results with
different flags are different experiments. Each replica retains its configured
row budget, so aggregate row capacity is recorded separately. This is not a
claim of equal aggregate scheduler capacity.

Host retained target weights are 16,381,470,720 bytes per controller. Required
host headroom is three times the aggregate target bytes plus an explicit
reserve of at least 32 GiB. The launcher retains the raw `MemAvailable` snapshot
and its calculation. Expected base GPU weights per replica are
`13,891,534,848 + TP * 608,256 + 2,489,327,616` bytes. Aggregate GPU weights are
recorded separately from host target bytes; transpose layouts and temporary
workspaces are not included in that base formula. Actual Setup allocation
counters must be checked by the cohort verifier.

## Controller Hook

`src/bin/tp_benchmark_control.rs` is an optional module for the existing batch
CLI. Legacy runs do not use this module or change their clock/schema. Controlled
runs must perform these steps:

1. Parse the bounded requests bytes once and hash those exact parsed bytes.
2. Call `ControlConfig::open(control_path, requests_path, devices)` and
   `verify_loaded_requests_sha256(parsed_bytes_sha256)` before model setup.
3. After all setup, call `ready_and_wait()` once to obtain `BenchmarkClock`.
4. Emit `clock.metadata()` conditionally in the Setup `replica_benchmark` field.
   Use checked `clock.elapsed_ns()` for controlled arrival, first-token, token,
   and completion timestamps. Any clock error aborts before publishing JSON.
5. Close all runtime resources successfully, then call consuming `finish()`.
   Emit its receipt conditionally in the Close `replica_benchmark` field.

The safe `rustix::time::clock_gettime(MonotonicRaw)` clock is bound to hostname,
kernel boot ID, and the time namespace device/inode. All peers must be in the
same domain. The private 0700 output tree contains a mode0600 Unix socket and
configs; both peers check UID and controller/launcher PID credentials. Each
strict newline JSON frame is at most 4096 bytes. Socket reads, setup, release,
lateness, log sizes, and run durations are bounded. The helper performs no
unsafe descriptor conversion and grants no GPU authority.

Wire sequence: `FerricReplicaReadyV1`, `FerricReplicaStartV1`,
`FerricReplicaStartedV1`, `FerricReplicaClosedV1`, `FerricReplicaCloseAckV1`, EOF.
Every frame repeats the exact nonce, replica ID, global workload hash,
per-replica parsed-request hash, ordered devices, and clock domain. Start uses
one future absolute RAW-clock epoch after every replica is Ready. Started
records receipt time, actual release, and measured lateness. All echoed times
are strict integers. Completion requires exactly one valid close, empty frame
buffer, EOF, exit zero, and no live owned group members before controller reap.

## Invocation

Use `--plan` for an allocation manifest without launching anything:

```sh
python3 tools/replica_benchmark.py --plan --layout 4xTP2 \
  --devices "$EXACT_EIGHT_DECIMAL_DEVICE_IDS" --reference "$FROZEN_REFERENCE" \
  --output /absolute/new-private-directory
```

`--run` additionally requires explicit `--controller`, `--controller-sha256`,
`--worker`, `--worker-sha256`, `--source`, `--artifact`, and `--snapshot-command`.
The snapshot command is a bounded JSON argv array, never shell syntax; its JSON
output must pass the existing exact eight-card globally idle roster validator.
One snapshot is taken before the cohort and one after teardown, not once per
replica. Optional controller options are a bounded JSON string array provided
by `--controller-options`; controlled assignment, cache, workload, and collective
flags cannot be overridden. Output directories must be new absolute paths;
the Unix socket path must fit the explicit 100-byte bound.

Controller and worker executables are hashed, copied into private
`launch-artifacts`, made read/execute-only, rehashed, and held open. The
controller is executed through its held descriptor; its worker path points to
the retained copy. The verifier must still compare Setup's running worker hash,
model bundle, source/artifact closure and kernel pins with external expectations.
The raw control receive bytes, validated send/receive events, requests, configs,
launch argv/process identities, stdout/stderr, global snapshots, memory receipt,
and cohort record are retained. No rates are calculated by the launcher.

## Cleanup and Emergency Contract

SIGINT and SIGTERM request the normal abort path. The handler only sets a flag,
so it cannot interrupt subprocess creation before ownership is recorded. A
second interrupt is ignored during cleanup. Failed setup, protocol, process,
clock, output, or close validation terminates all owned process groups, then
escalates to SIGKILL. Controllers remain unreaped with `WNOWAIT` until a bounded
`/proc` inventory confirms no live member of each owned group. Zombie/exited
descendants are terminated, not executing. The launcher then reaps controllers
and records the ordering in a group-termination receipt. If cleanup cannot be
confirmed, it records `teardown_uncertain` and never reports completion.

Workers must inherit their controller's process group and must not daemonize,
detach, or change sessions. This launcher is not a cgroup service manager.
SIGKILL of the launcher and machine loss cannot be caught. An external owner
must retain the cohort directory and use the recorded boot/domain, controller
PID, PGID/session, and `/proc` start ticks to establish identity before any
emergency cleanup. Never blindly signal stale recorded PIDs. Verify all owned
descendants have terminated and restore global GPU idle state before another
cohort. Failed/interrupted cohorts cannot contribute performance numbers.

## Host Verification

The Rust integration wrapper tests exact identities, request hashes, clock and
frame bounds, private file/socket checks, future release, and close receipts.
Python tests use only an explicit fake controller and synthetic idle/memory
snapshots. They test all three layouts, immutable executable pins, full eight
child release, malformed/duplicate/float/trailing frames, timeout and exit
failures, SIGTERM, descendant cleanup, and failed post-snapshot custody.
These tests do not open a GPU or load a model. Build and test them only on the
designated remote build host, with bounded jobs and private temporary storage.
