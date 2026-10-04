# Owned Layer-0 Framework Capture

Unexecuted launcher proposal. It runs only the separately pinned independent
framework capture, not a candidate kernel. Neither successful capture nor exact
repeat bytes imply numerical acceptance, full-model correctness or performance.
The 18 authored launcher tests are synthetic policy tests. The actual launcher
first executes all 16 capture-package pure tests before any framework execution.

## Reused Boundary

`launch.py` hash-loads the original P224 supervisor at SHA
`810eab5d9f674f7042b32ed9f81af691c0d8fbad5a028478ce758e116621ee19`.
It does not call the old main function. The original `bounded`, process-group
ownership/reaping, child resource settings, environment, read-only monitor
utilities and storage limits remain in use. Ordinary children are unprivileged,
use CPUs 8/9 and niceness +10, and have the original 64 GiB RSS/900-second native
deadline. Launch the outer process at ordinary niceness zero so the unchanged
child adjustment produces niceness 10. The original 40 GiB initial and 38 GiB
ongoing free-space floors, 32 MiB capture/ordinary-file cap and private cache/tmp
limits are unchanged.

The only storage adapter accounts the separately named capture directory along
with the owner directory. `private-cache` retains its provider classification;
the combined trees still use the original aggregate limits and file-count bound.
Output and cache directory symlinks are refused.

The old PID is never reused as authority. A separately reviewed monitor document
supplies current process/start identity and executable bytes. It is restricted
to the root-owned `/usr/local/bin/gpuagent` process form and the original strict
zero-allocation/activity SMI schema. Its live process, executable and topology
are rechecked by the original monitor machinery. `platform_monitor: null` means
no exception: the original empty-process SMI check must pass. No monitor is ever
signalled. Unknown additional GPU processes are rejected.

## Preparation

The root prepares a closed `ferric-p228-layer0-framework-launch-inputs-v1` JSON:

- `launcher_sha256`: actual frozen `launch.py` digest.
- `owned_helper`: the exact original P224 `launch.py` FilePin.
- `capture_package`: exact `run.py`, `test_run.py`, `README.md` FilePins from the
  stable `p228-layer0-framework-capture-v1` package, in one E directory.
- `reference_helper`: original `framework_reference.py` FilePin (`613579c7...`).
- `reference_support`: `diagnostics`, `policy`, `long_reference` FilePins at the
  original helper's local paths. No old reference is overwritten.
- `legacy_plan`, `source_authentication`, `token_provenance`: the retained inputs
  already required by the original independent reference policy.
- `topology`: the complete freshly read `old.device()` object, not a summary.
- `platform_monitor`: freshly reviewed monitor-document FilePin, or null.
- `implementation_sources`: actual installed `modeling_qwen3`, `activation`,
  `sdpa`, `torch_functional` FilePins required by the new capture.
- `output_label`: fresh `layer0-framework-launch-v228-vN`.
- `capture_label`: fresh `layer0-framework-capture-v228-vN`.
- `execution_review`: independently root-authored launch-review FilePin.

E is `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
The launch review has the closed fields `schema`, `reviewed`,
`inputs_projection_sha256`, `resources`, `gpu_execution_authorized`,
`numerical_acceptance`, `performance_claim`, `production_authority`. Schema is
`ferric-p228-layer0-framework-launch-review-v1`; the two authorization booleans
must be true and all three claim flags false. The digest is SHA256 of the inputs
without `execution_review`, serialized as sorted, indented JSON plus newline.
`resources` must equal `LIMITS` in the launcher. No code here authors this review.

Run under the root's owned outer process after separately qualifying this
launcher package:

```text
/usr/bin/python3 -B E/p228-layer0-framework-launch-v1/launch.py INPUTS INPUTS_SHA
```

The existing isolated interpreter remains at
`/home/harmenon/ferric-asrock-42/evidence/qwen3-long-reference-env-v1/venv/bin/python`.
The launcher authenticates its resolved executable, preserves the original venv
invocation path, and later runs it with `-I -B`. It does not install packages or
copy model weights.

## Exact Review Handshake

The child requires its actual direct supervisor PID. After pure tests and three
fresh pre-audits, the live launcher writes `reference-plan.json`,
`capture-projection.json` and `ready.json`, and prints the ready FilePin. It waits
at most 300 seconds. Complete installed-source/monitor review before starting
this window.

The root reads and authenticates those files, then independently authors the
existing capture review with schema
`ferric-p228-layer0-framework-execution-review-v1`. Its
`plan_projection_sha256` is the exact `ready.json` projection digest; resources
and claim flags follow the existing capture contract. This is separate from the
launch review to avoid a recursive hash dependency.

After saving that review, the root atomically creates `OWNER/approval.json`:

```json
{
  "schema": "ferric-p228-layer0-framework-launch-approval-v1",
  "projection_sha256": "ACTUAL_READY_PROJECTION_SHA",
  "execution_review": {"path": "ACTUAL_REVIEW_PATH", "bytes": 0, "sha256": "ACTUAL_SHA"}
}
```

The example extent is a placeholder, not an acceptable actual pin. The launcher
authenticates the review and refuses a changed projection, missing approval or
expired wait. It never constructs an approving review itself. It then performs
no-GPU `--inspect`, an immediate idle audit, and exactly one
`--run-reviewed-layer0-capture` invocation. There is no retry.

## Completion

Three post-audits run in `finally`, including after inspection, native or
comparison failure; failures are accumulated, not converted to success. The
original supervisor owns termination and reap of each launched child. A success
requires all six audits, natural successful children, complete 33-stage BF16
captures from each of two fresh-cache passes, producer/consumer joins, and exact
repeat bytes. All input/implementation/interpreter/SMI identities and storage
bounds are rechecked.

`complete.json` uses `ferric-p228-layer0-framework-launch-complete-v1` and retains
the actual capture FilePin as `reference`, the child plan, pure/inspection/native
owned result pins, audit arrays, input pins and topology. An explicit nonzero
exit accompanies failure. The capture's own lifecycle-pending flag is unchanged;
this outer receipt adds actual child lifecycle evidence. The primary's separate
owner still must observe and retain the outer launcher's terminal exit/reap.
