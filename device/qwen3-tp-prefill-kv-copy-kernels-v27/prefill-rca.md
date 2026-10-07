# Prefill RCA: Observations Versus Hypotheses

Reviewed active Ferric V17/V22 source read-only. Paths below are relative to
`/home/harsh/.codex-tmp/ferric-integration-v3`.

## No Redundant Prompt Head or Missing MFMA Selection

- `adapters/m1-engineering-execution-v1/src/bin/wave_argmax_live_contract.rs:8`
  fixes physical capacity32 but prompt chunk16. The shared V17/V22 runner uses
  these constants and configures pruning, MFMA, deviceTP1, FP32-v8 head and the
  C1-only Wave layer selector at `src/bin/wave_target_v17_runner.rs:554`.
- `src/tp_scheduler.rs:471` caps each request's prompt rows at that16-token
  chunk and marks only the final prompt token `PrefillFinal`.
- `src/tp_batch_runtime.rs:328` excludes every `PrefillIntermediate` row from
  `output_rows`. `src/tp_execution/batched.rs:1309` projects only those rows
  when pruning is on. `batched.rs:1895` returns before final norm/head/argmax
  if there are no published rows. The final chunk produces one head row.
- `src/tp_execution/projection.rs:168` selects Wave layers only for rows1;
  the actual16-row chunks stay MFMA. `row_profile.rs:183` selects the V5
  batch32 MFMA roots and `row_profile.rs:202` multiplies tile grids by
  `ceil(rows/16)`. This means a16-row chunk computes one16-row tile, NOT
  thirty-two padded rows. A32-row chunk computes two16-row tiles.
- `device/qwen3-tp-batch32-kernels-v5/src/projection.rs:331` and the partial
  root's branches at lines502/518 use the existing16x16 MFMA fragment
  loops. Changing the selector from GEMV to MFMA is not a prefill optimization:
  the multirow route already uses MFMA.

The128-token matched prompt executes seven613-packet headless batches plus
one616-packet final batch:4907 packets before the first token. The following
127 decode batches use616 each, yielding83139/request. V22 packing starts only
when `rows==1 && output_rows==[0]` at `batched.rs:1597`, so its improved decode
submission does not alter these eight prefill batches.

## Retained Host Evidence

Input evidence is `E/profile-retained-r1/host-timing.json`, SHA
`e5d9206333b83087dd9c95809ea3ee25b9c34e4e656b5dbc861df3e9128eaf1a`,
where E is `/home/harsh/.codex-tmp/ferric-matched-optimization-v5`.
The adjacent accepted report records one warmup and one instrumented diagnostic
request,270 physical batches,166278 completed dispatches. It explicitly says
host-wall latency, overlapping/non-additive scopes, not GPU duration, HTTP,
sustained throughput or a vendor comparison.

For the diagnostic request's eight prompt batches136..143, the retained spans
sum within each label to:

| Scope | Count | Host-wall milliseconds |
| --- | ---: | ---: |
| whole batch | 8 | 2282.386812 |
| collective_attention | 288 | 1889.415254 |
| collective_feed_forward | 288 | 386.139320 |
| metadata | 8 | 1.865165 |
| output_head | 1 | 2.256607 |

Do not add these overlapping rows. The per-operation construction scopes are
tiny because commands are queued; execution waits occur at the ordered-group
flush. Each attention group contains norm, Q/K/V projections, Q/K norm, RoPE,
KV append, GQA, output projection and residual, not just GQA. Thus these data
support investigating that group, not attributing1889ms to any single kernel.
The final head is not the leading observed prompt span. These diagnostic data
must not be substituted for the separate matched HTTP TTFT cohort.

## V27 Lead

`device/qwen3-tp-batch32-kernels-v5/src/rope_kv.rs:275` launches only one
workgroup; at line343 `grid_leader` leaves one active writer. It validates
every row's slot and earlier-row uniqueness, then serially copies all rows
and1024 components at line454 onward. In the16-row TP1 case this is16384
serial K copies and16384 serial V copies per layer, with raw volatile reads
and checked stores. The one-row case copies only1024 of each. This is a
specific source explanation for potential prefill scaling within the dominant
group, but its actual contribution has NOT been measured independently.

V27 preserves one append packet per layer and all bits while letting16384
lanes fill one separately owned, full physical page. Existing host reservations
already validate row/page/COW ownership, but the new whole-page view requires
its own explicit predicate: same sequence, aligned complete16-token page,
same destination physical page for every row, exclusive writable reservation,
and either exact natural or final-head rotation order. Do not broaden this to
arbitrary scatter or infer page authority from a scalar kernel argument.

A secondary bounded experiment is chunk32: existing kernels can process it,
reducing the prompt to four batches and2455 packets without increasing the
total MFMA row-tile count. It does not remove KV bytes copied, doubles the
within-chunk slot-comparison pairs across the prompt, and the current serial
append may limit its benefit. It should therefore remain a separate candidate,
not be mixed into the first V27 A/B or described as a predicted2x TTFT gain.

Required next observation: isolated V5 append versus V27, exact16-row full-page
inputs, both source permutations, identical warmed allocations and whole-copy
completion boundaries. Then exact full-model output IDs/text and matched TTFT
are required. V19's noisy C1 no-gain result neither establishes nor rules out
a prefill benefit; no speedup is assumed from this source change.
