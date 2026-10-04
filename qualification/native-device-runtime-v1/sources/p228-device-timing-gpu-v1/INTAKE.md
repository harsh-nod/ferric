# Device Timing Intake

Draft only. No imports, tests, builds, SSH or GPU execution were performed by the
author. `test_intake.py` contains 14 synthetic policy tests, not actual admission
or numerical evidence. Root owns freeze, qualification, reviews and execution.

## Inputs and Scope

`intake.context(plan_pin)` accepts one closed `ferric-p228-device-timing-inputs-v1`
plan. `PLAN_FIELDS` in `intake.py` is the exact field list. This first experiment
is TF4 with `shared-full-currentness` only, and an exclusive output label:

```text
prefix-device-timing-tf4-shared-full-currentness-gpu-v228-vN
```

The request is exactly `{schema, decode}`, with schema
`FerricFinitePrefixDecodeDeviceRequestV1`. The supervisor launches the newly
qualified `ferric-qwen3-finite-prefix-decode-device-engineering` parent with:

```text
--request REQUEST --allow-unauthenticated-machine-code --observe-device-ticks
```

This is an explicit engineering observation. No new deployment qualification is
invented, no historical CPU633 parent is admitted as the new parent, and no old
HostV2 request/parser is applied to the candidate. Old HostV2 validation is used
only when replaying the authentic CPU553 baseline.

## Existing Evidence Reuse

Root must provide these exact, unchanged local package copies:

| Local filename | Original source | SHA256 |
|---|---|---|
| `run_row_facts_v2.py` | Frozen custody helper | `244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820` |
| `baseline_comparison.py` | `p228-state-bank-runtime-comparison-v1/run.py` | `413e8bb4ea5637cb3ca67003433b62bf7cb90832c4f88101d2c9ea9fa7bcc512` |
| `baseline_diagnostic.py` | `p228-independent-decode-diagnostic-v1/run.py` | `259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930` |

`layer_validation.py` is the unchanged helper used by the prior observer. The
baseline loader obtains the original frozen `p228-state-bank-batch-observation-v1`
package through its existing pinned reader, including its import dependencies.
That package and its earlier prerequisite packages remain required on MI350.

Only the actual baseline receipt is admitted:

```text
E/prefix-state-bank-batch-tf4-shared-full-currentness-gpu-v228-v1/complete.json
SHA256 9917a07aba38ecdf4ca1c9289774c9b9c040bdf5bf850157126c87ca1ab7595c
```

Its owned leaves, six process-audit records, worker/parent identities, native
files, HostV2 report and structural observation are replayed by the existing
tested comparator, not merely trusted from a summary. The unchanged bank intake
also replays the six V7 standalone GPU/numerical prerequisites and their recorded
assumptions. These conditional checks do not establish full-model correctness.

The new plan must retain the baseline's exact `image_deployment`,
`standalone_prepared`, `standalone_cases` and `numericals`. Only worker, fresh
session and evidence-directory fields may differ in the decode request. The
parent is a separate newly qualified binary outside that inner request.

## Qualified CPU Artifacts

CPU609 worker completion is bound to 183804 bytes and SHA256
`407990eeac9332f4807ff71fcc7c884a0f14a92dffd44f97fd19ea7fa0b7bbf7`.
Its worker is 4872432 bytes with SHA256
`eda6a2450636d5fdc8ceca0f39b84769cdbcf47a74b73bb66310eb5cc8233d03`.

`PARENT_CPU = None` intentionally refuses admission until root supplies the
actual passing parent completion extent/digest. Expected parent coverage is
257/0 with 41 phases; these are admission requirements, not a claim that the
pending qualification passed. The worker has actual 609/4 and 25 phases.

For each CPU generation, root must transport the exact completion plus its
`sources-before.json`, `sources-after.json` and selected Cargo build stdout at
their retained FilePin paths. Intake verifies the exact root-verified completion,
successful natural phase records, unchanged source maps, actual Cargo selected
artifact occurrence, and build stdout digest. It does not rerun the compiler or
read original build targets, dependency trees or toolchains on MI350.

Parent aliases and worker must have identical worker subtree rosters and contents,
except only `README.md`, updated by the CPU609 documentation publication after
its build. Every Rust source, test, Cargo file, config or other member remains in
the exact join, including the raw device Report and all relevant wire modules. The qualified fe2o3 source
commit must also match. `plan.parent` and `plan.worker` are explicit transported
FilePins under E; their ELF bytes must equal the original Cargo-selected artifacts.
Original and transported paths remain separate in the returned context.

Existing `P.runtime_review` authenticates actual selected executables, libraries,
tool outputs and device review. `decode_review` binds both CPU receipts, all
selected runtime pins, baseline, request and six-case prerequisites. It requires
root-authored substantive review notes, one attempt and false numerical,
production, performance, calibration, common-clock and overlap claims. The
intake never changes `reviewed` or authors an affirmative review.

## Supervisor Interface

The returned context preserves `D/V/E/R`, `read/save/owned_success/BODY`,
`pins`, `owned`, `O`, `P`, `standalone`, `topology`, `platform`, `environment`,
`runtime_reviews`, `C/H`, `plan`, `plan_pin`, `request`, `policy`, `out`,
`runtime/selected_runtime`, `image_provenance`, and `supervisor_manifest`.

`baseline` contains authenticated `observed`, `files`, `records`, `receipt`,
`receipt_pin`, `request`, and `runtime`. `parent_cpu` and `worker_cpu` contain
the actual CPU completions; `parent_artifact` and `worker_artifact` retain both
the original Cargo record and original binary FilePin.

`guard(c)` retains the existing immutable pin, six-case prerequisite and runtime
library alias rechecks. Runtime owns the bounded supervisor, six fresh audits,
actual sidecar/control/capture/Close/PID validation and 152-tensor invariance.
Nothing here converts raw ticks to time or infers overlap or speedup.

## Before Execution

Root must fill `PARENT_CPU`, `PACKAGE_FILES`, `PURE_TESTS`, and `TEST_RUNNER_SHA`
from actual evidence and freeze the resulting package. The exact pure receipt
schema is `ferric-p228-device-timing-pure-v1`; its existing before/after snapshot
and transcript contract is preserved. The expected pure output namespace is
`E/device-timing-gpu-pure-v228-vN` and the pinned runner path is
`E/run_device_timing_gpu_pure_p228_v1.py`. Actual test totals come from the full
assembled package's test inventory, not the 14 intake tests alone.
