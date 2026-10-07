# Explicit V7 Head Comparisons

`compare_tp_head_precision_v7.py` is a separate, source-pinned adapter checker.
It does not change the frozen BF16 comparator, timing summary or performance
ledger. Two explicit profiles are supported:

| Profile | Head arithmetic | Additional workspace payload |
|---|---|---:|
| `bf16-v7-control` | Original BF16 head; v7 artifact loaded | 0 bytes |
| `fp32-v7` | Generic FP32 head and argmax | 9,723,904 bytes |

Both require top-level Setup `head_precision`, an exact `fp32_head_artifact`
object containing externally pinned HSACO/manifest/handoff IDs, and
`fp32_head_workspace_bytes`. Neither profile can masquerade as legacy BF16.
The wrapper permits only TP1, row/chunk budgets16, baseline or MFMA projection,
baseline attention, and no sequences, wide metadata, capture or replica fields.
All other runtime and pruning policies must exactly match the explicit expected
seven-key profile. Normal dispatch counts remain checked by the frozen checker.

The expectation schema is `FerricQwen3TpHeadPrecisionExpectationV1`. It requires
the five usual controller/worker/base-artifact hashes, workload/reference hashes,
all eight physical GPU IDs, prefix-cache/pruning booleans, seven-key performance
profile, collective (`null` for the legacy host mode), head precision and the
three v7 artifact IDs. No hash or profile is inferred from an untrusted trace.

```bash
python3 -B adapters/m1-engineering-execution-v1/tools/compare_tp_head_precision_v7.py \
  --run-dir /absolute/archived/run --workload /absolute/workload.json \
  --reference /absolute/reference.json --expect /absolute/prelaunch-expectation.json \
  --output /absolute/new-v7-comparison.json
```

After validating the exact extra metadata, the wrapper removes only those three
Setup fields from an in-memory copy for the unchanged `1be4f2b3...` semantic and
fixed-reference checker. Raw bytes and raw hashes remain unchanged. Unknown
fields, failed references, nonzero exits, incomplete close, mismatched identities
or non-idle GPUs reject before any successful comparison is written. Output is
exclusive and uses `FerricQwen3TpHeadPrecisionComparisonV1`, not the legacy schema.

Freeze three matched cases with identical controller, worker, base/v7 artifacts,
workload/reference and policy: baseline projection with BF16-v7 control; baseline
projection with FP32-v7; MFMA projection with FP32-v7. The first pair isolates the
precision change including its workspace; the second isolates MFMA under fixed
FP32 head precision. All cases must pass exact token/reference checks before
timing interpretation. Do not automatically feed these reports into the old BF16
ledger, pool unrelated request identities, or claim steady-state serving from the
short fixed logical-tick workload. Host wall latencies are not GPU durations.
