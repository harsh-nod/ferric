# V14 Measurement Successor

This directory adds measurement and replay code, not a measured speedup or a
GPU-launch authorization bypass. Frozen V13 and V19 sources are unchanged.
The first four Python files passed 52 tests remotely under G28; subsequent
native-cell changes need their own remote test receipt. Do not test/build this
project locally.

## Native Integration

`native_token_cell.run_cell(spec, output, *, runner, legacy, counter, evidence,
admission)` runs one cell inside the integration lead's existing GPU supervisor.
There is deliberately no standalone, unguarded GPU-launch CLI.

- `runner` and `legacy` come from the pinned existing `counter.load(stage)`.
- `counter` is the recovered V19 `counter-support/run_counter_diagnostic.py`.
- `evidence` is the recovered V19 `counter-support/process_evidence.py`.
- `admission(phase, setup_or_none)` must retain the existing outer resource,
  placement, process-identity and device checks and return a receipt containing
  `accepted: true`. It is called for preflight, before/after requests, and
  postflight. A separate outer monitor remains necessary during requests.
- `output` must not exist. Failures retain `result.json` and raw transcripts;
  later cleanup never changes a failed attempt into a success.

The spec schema is `FerricNativeTokenCellPlanV1`. Required fields:

| Field | Meaning |
| --- | --- |
| `mode` | `correctness`, `counters`, or `latency` |
| `arm` | `A` = ordered64 groups, `B` = native whole program |
| `argv` | Complete bound controller argv, including exact V19/split8/prefill16 selectors |
| `controller`, `worker` | `{path, sha256}` absolute binary bindings |
| `device_unique_id` | Physical device's integer unique ID |
| `prompt`, `reference` | Frozen prompt and complete exact-token/byte reference JSON |
| `setup_expected` | Actual frozen model IDs and artifact metadata checked against setup |
| `profile_expected`, `closed_expected` | Frozen artifact/model metadata checked in these scopes |
| `timeouts` | `{setup_seconds:600, request_seconds:180, cell_seconds:1200}` |

The outer builder must bind and check all nine artifact identities in the
expected metadata. The helper does not invent hashes or infer omitted artifact
bindings. It additionally checks backend identity, graph composition, worker
hash, device, BF16/FP32-head policy, context, TP1 and disabled prefix caching.

Correctness and counters use one request, `--max-batches 135`, 87,711 total
dispatches. Counters use the separate counter executable, leading
`--token-program-backend ordered64-groups-v1|native-whole-program-v1`, and exactly
two stderr JSON snapshots. They never qualify as latency samples. The snapshots
must prove 127 executions, 82,804 dispatches/retired signals, and 1,397 versus
127 publications/waits. Staging time and initialized-byte counts are retained.

Latency uses two excluded warmups plus four measured requests, `--max-batches
810`, 526,266 dispatches, and no stderr instrumentation. Every output is checked
through existing `StreamParity`, and every raw command/event is replayed.

`profile_for_spec(latency_spec)` produces the exact profile manifest for the
predeclared plan. In the first native A/B, only `/artifacts/controller` and
`/configuration/backend` differ. All kernel, model, worker and argument inputs
must be identical. The current evaluator requires shared immutable input paths
across cells; it intentionally does not silently normalize unrelated stages.

`evaluate_campaign(plan, cells, counter_cells)` accepts 12 native latency entries
and two separate mechanism entries. Each entry contains `cell_id`, `spec`, and
the guarded driver's raw-replayed `result`, plus the final `outer` supervisor
receipt and `completion` receipt. These must have clean status, accepted final
postflight, stable inputs, and matching hashes from outer completion to the
exact retained inner result. An inner success followed by outer failure is not
admitted. Use this directly in the driver that
owns these results. Offline consumers must independently replay the bound raw
transcripts, not trust an externally supplied `raw_replay_passed` boolean.

## Plan And Ledger

`abba_ledger.GATES` is the fixed promotion contract:

- Three ABBA blocks, two warmups and four measured requests per cell.
- At least 5% lower aggregate median TPOT, faster in five of six adjacent pairs,
  and positive median improvement in both AB and BA orders.
- No more than 5% regression in TTFT median/p95, TPOT p95 or finite output rate.
- Exact output IDs/bytes, dispatch/frontier checks, successful mechanism
  counters, no fallback/poison/timeout/leak, and clean resource/lifecycle checks.

Create and retain the immutable `FerricChangePlanV1` before launching, including
`change_id`, `gates`, fixed `workload`, exact A/B `profiles`, the complete
`allowed_profile_differences` list, reference/workload/client SHA-256 bindings,
per-arm `mechanism` counters, and `timing_semantics`. Use `native-ingress-v1` for
native cells and `http-text-chunk-v1` for HTTP. Do not relabel one as the other.
For native inline specs, `reference_sha256` is the canonical JSON digest of
`spec.reference`; retain the original raw reference-file hash separately in
outer input custody. HTTP plans bind the original reference-file bytes.

The HTTP evaluator replays existing `FerricCompetitiveStreamingRunV1` raw chunk
clocks, exact per-request output receipts, lifecycle evidence and separate
mechanism counters. It does not trust aggregate timing summaries. A complete
`FerricChangeAttemptV1` binds the canonical plan digest, reference, mechanism and
12 ordered cells. Each cell binds run/correctness/lifecycle JSON by relative path
and SHA-256; copied raw runs cannot count as new cells. The tests contain a full
synthetic schema fixture, never a live launch plan.

```text
python3 -I -B abba_ledger.py evaluate --plan plan.json --attempt attempt.json --output evaluation.json
python3 -I -B abba_ledger.py append --ledger improvements.jsonl --entry evaluation.json
```

The ledger uses append-only, fsynced, hash-linked JSONL with an exclusive file
lock. Preserve `pending`, `failed` and `inconclusive` outcomes as well as
`promotable` outcomes. A ledger status is a retained engineering assessment,
not an authorization or independent proof. Failed/incomplete attempts do not
contribute latency samples. A later attempt has a fresh ID and new raw files.

Outputs include TTFT/TPOT median, mean and p95; explicit TTFT and TPOT percentage
gains; all six adjacent-pair improvements; finite output rate and regression
checks. Percentiles use linear interpolation at `(n-1)*p`. These are finite
cohorts, not steady loaded throughput or statistical confidence claims.

## Container-Aware Descriptor Probe

`gpu_activity.py` follows `/proc/<pid>/fd/<fd>` and compares character-device
major/minor numbers, independent of the owning container's mount path. It scans
all visible PIDs and rejects incomplete access, PID identity changes, census
churn, unknown users and missing positive attribution during active work. It
performs no kill, pause, GPU launch or other privileged mutation.

```text
python3 -I -B gpu_activity.py --phase active --device /dev/kfd --owned-identities identities.json
python3 -I -B gpu_activity.py --phase active --device /dev/kfd --container-binding container.json
```

Owned identities include PID, start ticks, UIDs, proc UID, PID namespace inode
and cgroup. The container binding requires full container ID, image ID, exact
name and ownership-label key/value; read-only `docker inspect/top` establish host
PID membership before and after the scan. Never supply a freshly observed
unknown GPU user as an allowed identity merely to make a scan pass.

Root-level read access may be required. If it is unavailable, the result is a
refusal, not idle. Whole-sample retries must be bounded, keep every refusal and
never relabel skipped samples as accepted. An empty endpoint descriptor scan is
only that observation: keep existing GPU-utilization and resource checks.
Mapped-only device usage after closing all FDs and activity between samples are
not ruled out by this probe. No continuous-isolation claim is made.

## Remote CPU Tests

Stage a new immutable source archive, then run under the unchanged remote G28
guard:

```text
python3 -I -B -m unittest discover -s <staged-source> -p 'test_*.py' -v
```

Tests use synthetic records and fake process/descriptor/controller helpers.
They do not launch Docker, read live GPU processes or require GPU access.
