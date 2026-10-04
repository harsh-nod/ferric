# Genuine Layer-Zero Native Capture Controller

Prepared for root qualification. The separate capture-parent CPU run passed
289 Rust tests and all 46 phases on mi350-2. Its actual receipt and ELF are now
bound in intake. This controller's 40 synthetic tests and native GPU capture
have not yet run. Root owns qualification, input review, execution and retention.

## Narrow Successor

`run.py` derives from the actually used `p228-down2-clock-gpu-v1/run.py`.
`resources`, `inventory`, `child_limits` and `bounded` are unchanged. The same
pidfd-owned process tracker, discovery, reserved cleanup, emergency reap,
signal handling, stream/whole-tree limits and natural-exit requirements remain.
The three before and three finally-post audits use the existing authenticated
topology/SMI helpers. A failed pre-audit prevents native execution; an individual
post-audit failure does not skip the remaining post-audits. There are no retries.

Limits remain: 4,000 seconds native, 30 seconds per audit, 4,300 seconds per case,
8 MiB streams, 64 MiB case files, at most 256 filesystem entries, CPU affinity
8/9, child nice +10, 32 GiB native address space and 12 GiB audit address space.
Disk floors remain 40 GiB initially and 38 GiB during execution. Existing lower
process limits are never raised. The controller requires the same engineering
UID/affinity/nice contract. Root must inspect the actual current host, devices
and admitted monitor; historical identities are not substituted for that check.

Only the explicit parent route changes:

```
NEW_QUALIFIED_PARENT --request ABSOLUTE_REQUEST \
  --capture-layer-zero --allow-unauthenticated-machine-code
```

The new flat Rust request is `FerricFinitePrefixLayerCaptureRequestV1`. It keeps
the original model, bundle, prompt/tokenizer, bootstrap images, unchanged CPU669
worker, V7 prefix and Down2 MLP images. It computes one genuine token 9112 at
position zero, not token 785 and not an injected intermediate. The old paired
parent mode and its parity gate are neither invoked nor changed. The unchanged
one-layer worker is not relabeled as the TF4 host-policy execution path.

The parent CPU completion and selected Cargo executable are actual
root-reviewed inputs. No projected receipt or binary SHA is admitted. Intake
also requires separate root-authored engineering/runtime reviews and the new
package's actual bounded pure-test receipt. Those fields remain fail-closed
until root supplies them; this document grants no launch authority.

## Capture Boundary

`capture_validation.py` reuses the unchanged `layer_validation.py` primitives
for the source registration/program, scoped profile digest, input metadata,
typed Prefix284/MLP548 terminal states, finite values and complete physical KV
checks. It does not call the old paired `validate` entrypoint.

The native body roster is the exact ordered eleven names in `CV.NAMES`, plus
`summary.json`. The parent stdout must be that exact summary plus one newline.
Parent stderr must be the single candidate PID/PGID announcement emitted by the
existing `run_one`; no additional line is accepted. Child stderr is separately
retained and must be empty. Actual retained Run1/Close2 request/response bytes
are checked against the scoped bootstrap and the released capture digest.

All 28 arrays (14 per rank, 9,670,656 bytes total) are independently partitioned,
hashed and finite-checked; current-slot-only KV writes are checked across both
full physical caches. Each final-hidden row must match the authenticated first
8,192 bytes of the genuine position-zero current Down2 TF4 observation:

```
faa56202578a3d3497bbe779137736439957e473775bd6f677dbce466a9d6979
```

Intake must authenticate the complete source payload and its first token,
position, model, images and worker before deriving `expected_hidden`. Checking
the above digest alone is not a substitute for those joins.

The outer tracker may miss a short-lived worker between polling intervals.
When it observed that child, its actual PID/starttime/parent/private PGID and
inherited SID must agree; when it did not, the output explicitly records a
missing outer identity and the parent-asserted Close/reap, rather than inventing
pidfd evidence. No unknown third process is accepted. Parent owned cleanup and
post-audit quiescence remain required in either case.

## Output and Further Comparison

The outer result schema is `ferric-p228-layer0-native-capture-gpu-v1`.
Success means closed capture custody and the current native final-output join,
not independent numerical acceptance. It retains the original baseline and
source-payload slice identity, runtime/CPU/review/input pins, all seven owned
leaves, six audit records, all twelve native files and checked stage rows.
Failure retains available known native files without publishing completion.

Framework comparison belongs in a separate bounded data-only CPU step after
all post-audits. This controller does not import Torch or the numerical
comparer, loosen a tolerance, calibrate clocks or claim full-model correctness,
GPU time, overlap, sustained 2,048/256 performance, or 700 tokens/s.

## Authored Tests

`test_capture_validation.py` has twelve synthetic tests using the real frozen
wire/source/terminal/capture predicates: full successful shape; production
digest refusal for synthetic data; both final-hidden ranks; exact body roster;
old/authority modes; input/image/child scope; Run/Close and natural reap;
every terminal word; nonfinite/untouched KV; stage types and hashes; child
stderr/duplicate JSON; and the exact one-child marker.

`test_run.py` has eleven file-backed tests. They mock the native leaf and the
capture validator, not claim an actual native run. They cover single-attempt
routing/retention, failure classes, source drift, pre/post-audit failures,
closed/partial rosters, filesystem bounds, unchanged resource limits, owner
refusals and actual-versus-unobserved child lineage. Intake tests are separately
owned and documented in `INTAKE.md`; root will derive the final package census
from the frozen source and retain its actual result.
