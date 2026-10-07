# Warm Paired-Terminal AR4

This explicit engineering route connects the already qualified
[paired-terminal runtime API](../guarded-mlp-terminal-pair-v1/README.md) to
Ferric's reusable four-forward autoregressive model worker. It changes
terminal validation cadence, not the GPU kernels, arithmetic, model weights,
global currentness policy or hidden-state read policy. It is not the sustained
2,048-prompt/256-generated workload or a performance claim.

## Selection and Ownership

The worker flag is
`--engineering-native-guarded-mlp-reusable-ar4-paired-terminal-v1`; the parent
flag is `--observe-guarded-reusable-paired-terminal`. The route has distinct
request/bootstrap/observation schemas and a separate profile hash domain.
Existing entries retain their behavior. Capture, host observation, shared
currentness, paired hidden reads and readiness modes cannot be combined with
this route.

| Forward positions | Bank-local generation | Required prior generation | Arena allocation per rank/layer | Runtime call |
| --- | ---: | ---: | ---: | --- |
| 0, 1 | 1 | 0 | 1 | Existing terminal API |
| 2, 3 | 2 | 1 | 0 | Paired-terminal API |

Selection checks the actual reusable roster's bank, slot and generation
custody. The runtime independently checks private retirement proofs and
preserves queue, signal, ownership, fault and deadline validation. Errors
poison the sequence without retry through the legacy API. An unwind leaves
the in-flight operation uncommitted and prevents successful Close.

Only successful native Close makes the census publishable. Expected paired
dispatch counts are `[0,0,36,36]`; expected allocation samples are
`[715,711], [751,747], [787,783], [787,783], [787,783]`. The existing 4 KiB
stderr cap is unchanged.

## Qualification History

All builds and tests below ran on SSH host `mi350`, not the integration
machine. The original failed receipts are preserved rather than rewritten.

| Attempt | Actual result | Change needed |
| --- | --- | --- |
| [Worker V1](worker-cpu-attempt-v1/evidence/failed.json) | Rustfmt failed before tests | Rename the new test parameter `gen`, reserved in Rust 2024 |
| [Worker V2](worker-cpu-attempt-v2/evidence/failed.json) | Raw output: 672 passed, one failed, four ignored; qualification refused | Use the existing stdin lock in the new CLI branch; the real-executable EOF test caught nested-lock blocking |
| [Worker V3](worker-cpu-attempt-v3/evidence/complete.json) | 673 passed, four unchanged ignores; all nine phases clean | Both corrections, unchanged tests and deadlines |
| [Parent V3](parent-cpu-attempt-v3/evidence/complete.json) | 438 selected passes across 51 scopes and 60 clean phases | Not the entire 913-test library inventory |
| [Comparison checker](checker-cpu-v1/complete.json) | 20 synthetic tests passed | No GPU execution |

V2's original structured admitted-test field is empty because the gate
refused the failed suite; that does not mean no tests executed. The separate
retention report records the observed raw counts without changing admission.

The eleven qualified worker replacements are integrated. All 195 worker
source bodies match the tested map in
[the integration record](worker-source-integration.json). Its
`patch_applied=false` field describes the original plan-generation step;
integration and full-map verification occurred afterward. The worker's five
products include the same actual CLI ELF before and after its executable
tests. No dependency lock or runtime source was changed by this worker patch.

The three qualified parent changes are also integrated. The
[parent integration record](parent-source-integration.json) reconciles all
1,237 composed Ferric source bodies. It explicitly records ten formatter
differences between the worker inputs compiled by the parent and the separately
qualified worker postimages. The retained parent capsule includes every raw
test/command/result body and all six original product pins.

## Native Result

The matched same-ELF [control](native-pair-v1/control/complete.json) and
[candidate](native-pair-v1/candidate/complete.json) both pass on `mi350` in
one attempt each. Each completes eleven supervised phases, all four full-model
forwards, healthy Close and clean owned-process/device postchecks. Six idle
observations per run cover all eight devices. The candidate authenticates
the complete successful control before starting its own native execution.

All four 606,976-byte candidate payloads equal the control and the original
ordinary AR4 payloads, including all 36 layer outputs, final normalization
and logits. Both follow `9112 -> 67 -> 25 -> 576 -> 2701` with genuine own-output
recurrence. Control counts are `[0,0,0,0]`; candidate counts are `[0,0,36,36]`.
Both have the exact allocation plateau listed above. This is implementation
parity, not an independent accuracy reference.

The [original evidence capsule](native-pair-v1/manifest.json) retains 177
members, including all 156 raw case bodies, both original terminals, plans,
requests, helpers and four ordinary-reference payloads. No GPU execution is
repeated by retention, and neither original receipt is rewritten.

The [host-timing tables and per-layer plot](host-timing-v1/README.md) now pass
seven fixture tests and actual rendering on MI350. Warm segment sums fall
from 11,675.363 to 8,700.097 ms (-25.483%) in this pair; first-use sums rise
0.386%. These are one-pair host observations, not GPU timings, a controlled
repeated benchmark, end-to-end speedup or tokens/s. Full-model numerical
acceptance and all issue #42 M0-M7 milestones remain open.
