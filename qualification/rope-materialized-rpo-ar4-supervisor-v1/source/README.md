# RPO/RoPE Prefix AR4 Observation Draft

Source-only successor to the qualified CPU1037 AR4 supervisor. No test, native
execution, new image, numerical acceptance or performance result is claimed by
this package. The initial 65 tests are authored and unexecuted.

The explicit selected prefix image may change only after successful checked
RPO lowering. The retained CPU1037 parent/worker, original bootstrap/copy images,
SiLU MLP image `b0d1766f...` and projection residual image `25338bea...` remain
unchanged. The retained AR4 validator follows the candidate's own argmax chain:
it does not require teacher-forced token equality or the old AR4 trajectory.
No Rust rebuild is required by this selector change.

## Admission

The mandatory root-supplied plan pins are `prefix_cpu`,
`prefix_lowering_complete`, `prefix_lowering_owner` and `prefix_image`, alongside
all existing AR4 plan inputs. The command remains `run.py PLAN_PATH PLAN_SHA`.
No future completion or image digest is embedded in this draft.

The prefix CPU receipt must retain the actual 33 passing tests, one ordinary
ignored exhaustive case subsequently executed explicitly, 11 successful phases,
61 raw records and the V2 source's fifteen-file tested fixture. The lowered
eight Rust bodies remain joined to its formatted source snapshots.

The nine-stage lowering and naturally reaped owner must agree on the tested
source and exact frozen `p228-rope-materialized-rpo-lowering-v1` manifest
`6ed02271...`. Its compiler predecessor must equal the actual ordinary V7
generation. The new compiler identities are the already qualified RPO CPU
`56fc51fc...`, owner `afca99d8...`, tools `39eab92e...`, and owner `e919fb52...`.
Eight product identities, selected roles, backend alias, packages, source patch
and three equal compiled-source snapshots are joined as retained provenance.
Those compiler binaries and original build-host source/targets are not opened
or executed on MI350; `compiler_binary_bodies_replayed` is explicitly false.

All nine command/result/stream records and ten emitted/captured artifacts are
rehashed. Existing metadata checks require the prefix symbol, 120 explicit and
376 total kernarg bytes, WG64, Wave64, LDS512 and zero private bytes. The actual
LLVM/ISA/resource/float-mode review remains root-owned; these shape checks do not
prove GPU denormal equivalence or independently discharge arithmetic premises.

## Preserved Runtime Boundary

`decode_validation.py`, its seventeen tests, and four helper bodies are exact
copies of the frozen AR4 package. The supervisor differs only in its namespace
and retained prefix provenance. Seven owned leaves, six audits, one attempt,
four payloads/152 tensor rows, state/Control/profile checks, Close and child reap
remain unchanged. The root's substantive engineering review and current selected
ELF/runtime/device reviews remain required. The old AR4 native observation may
be a separate diagnostic baseline; byte equality is not an admission gate.

This is full-forward AR4 capture, not a layer-zero internal-stage capture. Its
nonzero positions exercise rotation on the actual own-output input chain.
An independent reference must distinguish genuine framework autoregression from
framework evaluation conditionally fed candidate-generated tokens. Neither
trajectory nor tensor differences are reclassified as numerical acceptance.

Native RoPE table generation is unchanged. Equality to framework-generated
tables is not asserted; BF16 arithmetic materialization and table construction
remain separate questions. Existing no-authority, no-clock-calibration,
no-overlap and no-throughput flags stay false.

## Root Execution

After review, freeze the exact sixteen package members and the 65-test census,
then use the packaged wrapper also copied byte-for-byte to:

`E/run_rope_materialized_rpo_ar4_gpu_pure_p228_v1.py`

Invoke with ordinary `python3 -B`, MI350 UID9661, CPU8/9, nice10 and all three GPU
visibility variables empty, passing `MANIFEST_SHA FRESH_PURE_LABEL`. The wrapper
enforces 2 GiB AS, 120 CPU seconds and 16 MiB output, and rehashes every package
member before and after. Its SHA is bound in intake only after wrapper bytes are
stable; no self-hash cycle. Test census: run11, validator17, intake16, SiLU8,
prefix/RPO13. No wrapper or test has been run by the author.

The native supervisor uses the inherited CPU8/9/nice0 controller and inherited
bounded native/audit leaves. A fresh label and separately reviewed plan are
mandatory. Root owns transport, actual qualification, engineering review and
the single eventual hardware attempt.
