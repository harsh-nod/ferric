# Fixed Replica Cohort Comparison

`compare_replica_cohort.py` is a read-only verifier for the replica experiment,
not a launcher, serving benchmark, or authenticated execution attestation. It
does not import or execute the launcher. The existing four-request comparator
and performance ledger remain unchanged.

```sh
python3 -I -B compare_replica_cohort.py \
  --cohort-dir /archived/completed-cohort \
  --reference /pinned/reference.json \
  --expect /externally-reviewed/expectation.json \
  --output /new/cohort-comparison.json
```

Success writes a new report and exits zero. A failed check does not create a
report; existing outputs are never overwritten. No source executable is run.
Both original frozen executable copies must be retained under
`launch-artifacts/`; this checker rehashes them without executing them.

## External Expectation

The expectation is closed-schema. Freeze it outside the cohort before launching;
do not generate it from the untrusted completed trace or accept caller-selected
profile labels without independent review. Use the launcher's explicit nonce
option and preflight RAW clock domain when available. Original paths below are
recorded MI350 paths, not the relocated archive paths used by `--cohort-dir`.
Every field is required; there are no inferred identity pins.

```json
{
  "schema": "FerricReplicaExpectationV1",
  "layout": "1xTP8",
  "device_unique_ids": [100, 101, 102, 103, 104, 105, 106, 107],
  "controller_sha256": "bef12d573a77741cd0a3719d8b5aa65d1630085d98df5ba54b244b2e7a33893b",
  "worker_sha256": "189b918dcd3f7104767404c636eb08490a3b1714a6f78123ea3a1fa06f21babb",
  "artifact_hsaco_id": "af5019d3cfc4e860b33ebf0d97a82439f870a9e4e893c730118a97d8735c2d6a",
  "artifact_manifest_id": "99a7ed6f84ad1d863917c425fd51cd38105e3aee35824941b2c7f25f51e9641c",
  "artifact_handoff_id": "4f9a1cd4e3fda57243ba264160c29117b13aebb96a03de16b3932a0d8257eaa9",
  "reference_sha256": "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
  "cohort_path": "/REPLACE/original/cohort",
  "source_path": "/REPLACE/model",
  "artifact_path": "/REPLACE/artifact",
  "source_controller_path": "/REPLACE/controller",
  "source_worker_path": "/REPLACE/worker",
  "snapshot_command": ["/REPLACE/snapshot-command"],
  "clock_domain": {
    "clock": "CLOCK_MONOTONIC_RAW",
    "hostname": "REPLACE",
    "boot_id": "11111111-2222-3333-4444-555555555555",
    "time_namespace_dev": 1,
    "time_namespace_ino": 1
  },
  "nonce": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "settings": {
    "ready_timeout_ns": 3600000000000,
    "run_timeout_ns": 3600000000000,
    "start_lead_ns": 1000000000,
    "max_lateness_ns": 100000000
  },
  "host_reserve_bytes": 34359738368,
  "controller_options": ["--runtime-operational"],
  "kernel_profile": "v2",
  "performance_profile": {
    "runtime_cache_admission": false,
    "runtime_operational": true,
    "dispatch_sequences": false,
    "queue_rollover": false,
    "projection": "baseline",
    "attention": "baseline",
    "runtime_profiling": false
  },
  "output_head_pruning": false,
  "row_policy": {"scope": "per-instance", "row_budget": 16},
  "batch_tokens": 16,
  "prefill_chunk": 16,
  "context_tokens": 128,
  "physical_pages": 64,
  "cache_ttl_ticks": 1024,
  "max_batches": 240
}
```

Replace example physical IDs, paths, domain, nonce and timing policy with actual
preflight pins. The displayed executable pins identify one historical controller
and worker, not an instruction to substitute those binaries for later versions.
`controller_options` must match the recorded argv exactly; resolving the
allowlisted options must produce the separately pinned policy values. Defaults
are the controller's reviewed defaults. Unknown options and duplicate flags fail.
`runtime_profiling` is false because the replica launcher cannot enable it.

Supported layouts partition exactly eight ordered physical IDs: `1xTP8`,
`4xTP2`, `8xTP1`. Each layout executes the same eight names and 64 frozen greedy
output tokens, with five prompt tokens per request, no cache or cancellation.
Replica assignment is the launcher's fixed round-robin global-name partition.
Only the independent-worker host-staged collective is accepted. Peer-process
profiles and arbitrary workloads require a separately reviewed contract.

The checker supports row/chunk budgets 1..16 for ordinary images and 1..32 for
explicit wide images. Chunk cannot exceed rows. Ordinary `v2` requires baseline
arithmetic; wave images cannot select MFMA/auto. Wide Setup fields are mandatory
only for wide profiles and otherwise forbidden. Context must fit all 12 consumed
positions; page capacity must fit every request simultaneously. The simulated
schedule validates exact row and output order, rotating decode/prefill cursors,
page counters, dispatch totals and all request retirements. Independent actual
coordinator tests cover the intended 8/2/1 local-request experiments at row/chunk
budgets 16 and 32. Other supported budgets have host-synthetic cross-checks only.

`row_policy.scope = per-instance` pins the budget per controller. `fixed-total`
pins the sum across replicas, which must divide into the actual identical
per-instance budgets. Keep comparisons between these policies separate. The
report always includes both per-instance and aggregate row budgets.

## Metrics

Each report describes exactly one cohort, not a distribution of repetitions:

- Global output rate: `64 * 1e9 / last_output_epoch_offset_ns`. The denominator
  starts at the common release T0, includes start lateness and ends at the last
  output across all replicas. Do not sum per-replica or per-request rates.
- Secondary admission-window rate: `64 * 1e9 / (last_output - first_admission)`.
  This is separately named and must not replace the common-release primary rate.
- Admission TTFT per global request identity: `first_output - actual_admission`.
- Release-to-first-token: `first_output - T0`; offsets already use T0 as zero.
- Actual-start-to-first-token: `first_output_offset - replica_lateness`.
- TPOT per request: the mean of its seven exact consecutive output intervals.
  The controller's integer-floor mean is checked; the report preserves the
  fractional mean in seconds. Never treat different requests as repetitions.
- Barrier setup: `all_ready - first_spawn`; whole cohort: `final_reap - first_spawn`.
  Controller setup/whole times and release-to-all-closed are also kept separate.

The report verifies all 64 output tokens and exact UTF-8, actual loaded payload
counters, externally pinned Setup and running-worker hashes, control frame
identity/clock/epoch/order, exact EOF, all-zero controller exits, empty owned
process-group termination and final reap. Both global snapshots must contain
all eight physical cards idle, with the before snapshot preceding first spawn
and the after snapshot following the final reap. There are no per-instance idle
snapshots while neighboring replicas are active.

Actual weight payload is distinct from rounded allocation footprint:

| Layout | Host Target Bytes | Base GPU Weight Bytes |
| --- | ---: | ---: |
| 1xTP8 | 16,381,470,720 | 16,385,728,512 |
| 4xTP2 | 65,525,882,880 | 65,528,315,904 |
| 8xTP1 | 131,051,765,760 | 131,051,765,760 |

MFMA/auto add 15,136,194,560 transposed bytes per instance, including when auto's
eventual one-row path uses wave arithmetic. Base counters exclude transposes,
KV, activations, scratch, allocator overhead and page rounding. Required host
headroom is three times retained target bytes plus an explicit reserve of at
least 32 GiB, checked against the retained preflight `MemAvailable` receipt.

The process receipt assumes descendants remain in the owned process groups;
it is not a cgroup-wide process proof. File and in-process hashes plus paths bind
the reviewed experiment, not an authenticated loader or source authorization.
Do not call a passing fixed cohort steady-state serving, an M1 protected-path
qualification, a stable p95 tail, or a vLLM/SGLang comparison.

## Host Validation

Run only on the designated build host in an owned bounded stage:

```sh
python3 -I -B -m unittest discover -s . -p test_compare_replica_cohort.py -v
```

Synthetic fixtures do not execute GPU code. Their positive full-cohort tests
mock only the frozen reference loader, while a separate negative test proves
the real entry point rejects an incorrect reference hash. Real cohort
comparisons must use the exact frozen reference file without any mocking.
