# Four-Forward Raw Device Timing

Successor controller for the separately qualified CPU257 parent and CPU609
worker. Actual test and GPU results are separate retained receipts; this source
document does not assert that either has passed for this controller generation.

## Admission Fix

V1 passed its 44 synthetic tests, but actual admission failed before creating
the case directory or launching a native process. The baseline replay had
restored its temporary Python module aliases before a later legacy-validator
load, causing `ModuleNotFoundError: host_comparison`.

V2 authenticates the legacy package and loads all five required dependency
aliases in order for that load only. It restores absent, existing and explicit
`None` bindings on success and failure. No search path fallback or validator
bypass is added. Three new synthetic regressions exercise actual Python import
binding with fixture modules, dependency/intake failures, and manifest
disagreement; the original 44 tests remain. Native binaries and runtime audits
are unchanged. Frozen V1 evidence is preserved rather than rewritten.

## Scope

The candidate uses the new explicit `--observe-device-ticks` parent, the closed
`FerricFinitePrefixDecodeDeviceRequestV1` request wrapper, and the separately
qualified raw-routing worker. It keeps the exact V7 code object, model inputs,
teacher-forced four-forward workload and shared-full-currentness policy of the
authenticated CPU553 GPU baseline. Session and output directories must be fresh.

The historical HostV2 reader is used only to replay that historical baseline.
Candidate admission never passes through a HostV2 compatibility shim. The new
device reader checks the actual parent wrapper and original native summary,
then joins all 1,172 packet rows to the same actual four Control/capture pairs.
The five selected object digests, nine entry/stage roles, rank, queue epoch,
group incarnation, packet IDs and signal generations must match exactly. Final
per-rank dispatch counts must be 592 and 580. The tail uses the original tail
object; the copy uses the original residual object.

Every Control still passes the frozen full typed-state validator. Its 293 host
intervals per forward must equal the corresponding rows. Start/end ticks must
be nonzero/ordered, but are never converted to nanoseconds or compared across
devices. A paired prefix row is the whole fused dispatch, not an invented
breakdown of internal normalization, QKV, attention and output stages.

Successful observation additionally requires all four complete 606,976-byte
payloads, the exact token trajectory and all 152 named tensor slices to match
the authenticated non-instrumented baseline. This is instrumentation invariance,
not new independent mathematical or full-model acceptance. A mismatch retains
failure evidence and does not trigger a retry.

## Preserved Lifecycle

`run.py` is derived from the frozen state-bank supervisor. Its process ownership,
pidfd tracking, cleanup/reap logic, resources, file retention and six audit
paths are unchanged. There is at most one native attempt. Three idle platform
and process audits precede it, and all three post-audits are attempted from
`finally`, including after parser, output, ownership or native failures.

The limits remain 4,000 seconds for the native leaf, 30 seconds per audit and
4,300 seconds for the case, reserving the original post-audit window. Native
address space remains 32 GiB, audit address space 12 GiB, streams 8 MiB, and the
whole case 64 MiB/256 entries. CPU affinity is 8 and 9 with leaf niceness 10;
disk floors remain 40 GiB initially and 38 GiB during execution. The new sidecar
is bounded to 2 MiB; the existing aggregate native evidence cap remains 8 MiB.

Only successful owned execution, recorded consuming Close, natural child and
parent reaping, all audits, immutable input checks, complete raw rows and exact
output invariance can produce `complete.json`. Partial native and sidecar bytes
are retained on failure but grant no authority. Raw ticks have no calibrated
time, cross-device alignment, overlap, performance or 700 tokens/s claim.

## Integration

Root owns the exact helper copies, package manifest, bounded pure runner,
qualification pins, artifact transport and runtime/engineering reviews. The
intake deliberately refuses absent future pins; it does not create reviews or
invent deployment authority. See `INTAKE.md` for the closed input contract.

`device_validation.py` needs the unchanged `layer_validation.py` and
`host_validation.py` helpers from the baseline package. The intake also uses
the unchanged custody helper and exact retained baseline replay helpers.
No tested controller is rewritten or given permissive fallback behavior.

The tests are synthetic: supervisor mocks never spawn a process, and
raw row fixtures do not establish GPU provenance. The raw tests explicitly
mock the existing terminal-state validator while testing the new timing-field
extraction. Actual package tests, intake replay, runtime audits and one bounded
GPU run remain separate required gates.
