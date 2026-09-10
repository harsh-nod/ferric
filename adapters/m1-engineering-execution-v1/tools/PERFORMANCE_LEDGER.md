# Fixed-Workload Performance Ledger

These tools validate the existing four-request Qwen3-8B logical-tick workload.
They do not launch workers, execute GPU code, or establish steady-state serving
throughput. Run tools/tests on the designated remote build host, not locally.

## Comparator Extensions

Without new expectation flags, `compare_tp_batch.py` retains the exact old
setup/batch schemas, legacy collective label, and dispatch counts. Extra
fields and unrequested profile labels reject.

- `--expect-output-head-pruning on|off` requires the boolean setup field and
  exact `output_head_rows` on every batch. Pruned rows equal published outputs;
  unpruned rows equal original source rows. Source row order remains unchanged.
- `--expect-collective host-staged-v1|host-staged-reuse-v3|device-tp1-v3` requires
  exactly that label. Omit it for the original long-form host-staged label,
  even if another extension is enabled. Device TP1 is rejected for TP2/TP8.
- `--expect-profile JSON` requires the exact closed seven-key object below.
  Unknown keys, modes, integer-as-boolean substitutions, and self-selected
  profile changes reject.
- Additional artifact pins: `--expect-manifest-sha256`,
  `--expect-handoff-sha256`, `--expect-workload-sha256`, and
  `--expect-reference-sha256`. The frozen reference hash is always checked.
- `--expect-collective device-peer-serial-v4` is supported only for TP2/TP8.
  It requires all three `--expect-peer-artifact-sha256`,
  `--expect-peer-manifest-sha256`, and `--expect-peer-handoff-sha256` pins,
  matching the exact setup `peer_artifact` object. Both PID rosters retain one
  entry per rank but repeat one nonzero PID. These extra pins/fields reject in
  every other mode; legacy modes still require distinct worker PIDs.

```json
{
  "runtime_cache_admission": false,
  "runtime_operational": false,
  "dispatch_sequences": false,
  "queue_rollover": false,
  "projection": "baseline",
  "attention": "baseline",
  "runtime_profiling": false
}
```

Projection modes are `baseline`, `wave`, `mfma`, `auto`; attention modes are
`baseline`, `wave`. The comparator does not infer that a label proves a speedup.
Rank zero executes 541 base dispatches plus three when output-head rows are
nonzero. Each peer executes 540. `device-tp1-v3` adds 72 rank-zero dispatches.
Totals are recomputed batch by batch, rather than multiplying one fixed count.
`device-peer-serial-v4` uses 613 dispatches per peer rank, including its embedding
copy and 72 ordered residual reductions. Rank zero uses 616, or 613 when
pruning suppresses an empty output head. This is explicitly serial peer-device
transport, not overlapped collective execution.

## Wide Kernel Profiles

`--expect-wide-kernel-profile v5-wave32|v5-mfma32` explicitly opts into a
32-row-capable image. It requires setup fields `kernel_profile` matching that
expectation and integer `kernel_row_capacity` equal to 32, as well as the exact
seven-key `--expect-profile` object. Neither setup field is accepted without
this flag. `v5-wave32` permits only `baseline` or `wave` projection;
`v5-mfma32` also permits `mfma` and `auto`. Attention remains `baseline` or
`wave`. Artifact hashes must still be independently pinned.

For the same fixed four-request workload, wide profiles support batch-token
budgets and prefill chunks from 16 through 32, with the chunk no larger than
the batch budget and with 16-token pages. The checker
validates the exact corresponding admission, row, output, retirement,
generation, and cache timeline. In particular, processing the seed's 17-token
prompt in one batch changes its completion tick and subsequent slot reuse;
this is not accepted as an arbitrary reordered trace. Budgets below 16 and
different workloads remain unsupported.

The report distinguishes `kernel_row_capacity` from
`maximum_batch_rows_observed`. Capacity 32 does not imply a 32-row batch was
executed: this cache-on workload reaches at most 17 rows and its cache-off
version reaches at most 20. Legacy reports retain their original shape.

A wide ledger expectation must add `wide_kernel_profile` with the explicit
profile string. Omit this key for legacy images; `null` is not an opt-out.
Peer artifact pins and pruning expectations remain independently required
when those extensions are selected. Batch and chunk budgets remain strict
ledger equivalence fields: do not pool budget-16 and budget-32 runs in one
optimization comparison. Use separately matched controls and clearly label
any separate policy comparison.

## Manifest And Invocation

`performance_ledger.py --manifest MANIFEST.json --json-output LEDGER.json
--markdown-output LEDGER.md` creates new outputs only. Its manifest has exact
top-level fields `schema`, `baseline`, `warmup_policy`, and `variants`:

```json
{
  "schema": "FerricTpPerformanceLedgerManifestV1",
  "baseline": "baseline",
  "warmup_policy": "fresh-worker-no-warmup",
  "variants": [
    {
      "name": "baseline",
      "kind": "baseline",
      "expect": {
        "world": 8,
        "prefix_cache": true,
        "output_head_pruning": null,
        "collective": null,
        "performance_profile": null,
        "controller_sha256": "EXTERNALLY_REVIEWED_64_HEX_DIGEST",
        "worker_sha256": "EXTERNALLY_REVIEWED_64_HEX_DIGEST",
        "artifact_hsaco_id": "EXTERNALLY_REVIEWED_64_HEX_DIGEST",
        "artifact_manifest_id": "EXTERNALLY_REVIEWED_64_HEX_DIGEST",
        "artifact_handoff_id": "EXTERNALLY_REVIEWED_64_HEX_DIGEST",
        "workload_sha256": "EXTERNALLY_REVIEWED_64_HEX_DIGEST",
        "reference_sha256": "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094"
      },
      "runs": [
        {
          "id": "baseline-r1",
          "run_dir": "/absolute/canonical/run",
          "workload": "/absolute/canonical/workload.json",
          "reference": "/absolute/canonical/reference.json",
          "comparison": "/absolute/canonical/current-comparison.json",
          "comparison_sha256": "EXTERNALLY_REVIEWED_64_HEX_DIGEST"
        }
      ]
    }
  ]
}
```

`null` expectations mean the old field is absent or the exact legacy collective
label is required, not "accept any value." New binaries emitting false pruning
or an all-default performance object must explicitly expect those values.
Each nonbaseline variant is `standalone` or `cumulative`. Repetitions have unique
IDs and unique canonical run directories. The maximum is 20 repetitions per
variant, 32 variants, and 128 runs overall.

Only an expectation with `collective` equal to `device-peer-serial-v4` must add
`peer_artifact`, an exact object with `artifact_hsaco_id`,
`artifact_manifest_id`, and `artifact_handoff_id` externally reviewed digest
values. All three are retained in each comparison and ledger run's identities.
Missing, unknown, zero, or mismatched peer identities reject, as does this
object in an ordinary host or TP1 variant. The five base identities and other
workload requirements remain unchanged.

Every ledger run requires all five externally supplied component identities,
workload/reference digests, a pinned current correctness report, status zero,
and matching idle snapshots of all eight physical GPUs. The ledger reruns the
current comparator and requires exact canonical equality with the supplied
report, including comparator source hash and raw input hashes. Old correctness
reports must be regenerated using all five identity pins; a stale report cannot
be substituted merely by changing its manifest digest.

Each ledger compares one world/cache/workload configuration. Dtype, target,
model bundle, physical roster, context capacity, physical page count, page and
chunk sizes, cache TTL, batch budget, arrival policy, and warmup policy must
match. Component hashes intentionally may differ between optimization variants.
Use separate ledgers for TP1, TP2, TP8, or cache-on/cache-off experiments.

## Metric Definitions

- TTFT: first committed output minus actual admission, per request identity.
- TPOT: arithmetic mean of adjacent committed output gaps within one request
  and repetition. The ledger retains the fractional mean rather than rounding
  to the trace's integer-nanosecond mean. One output has no TPOT, not zero TPOT.
- Request output rate: its output count divided by admission-to-terminal time.
- Workload output rate: all eight outputs, including the cancelled request's
  one output, divided by first-admission-to-last-terminal time. Request rates
  are never summed. A completed request terminates at its last output; a
  cancelled request terminates at `cancelled_ns`.
- Setup and whole-run times are reported separately, not silently subtracted
  or included in the workload rate.

Percentiles use nearest rank `sorted[ceil(p*n)-1]` across repetitions of the
same request and metric. Different request latencies or individual decode gaps
are never pooled. With two or three repetitions, p95 is simply the worst
observation and does not establish a stable serving tail or SLO. Reports show
repetition counts and decode gaps per repetition alongside the results.
Statistics are computed independently per metric, not selected from a single
"median run"; reciprocal throughput/window percentiles need not refer to the
same repetition.

Ratios are variant/baseline for each mean, p50, and p95. Positive improvement
means lower latency or higher output rate. These are descriptive comparisons,
not significance tests, and fresh worker processes do not guarantee cold GPU
caches or exclusive host resources.
