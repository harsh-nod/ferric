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

Parent qualification and the matched same-ELF native control/candidate run
are pending. The candidate must preserve all four complete control payloads,
the own-output recurrence, exact allocation/call census and healthy Close.
Host timing brackets are not GPU timings or tokens/s. Full-model numerical
acceptance and all issue #42 M0-M7 milestones remain open.
