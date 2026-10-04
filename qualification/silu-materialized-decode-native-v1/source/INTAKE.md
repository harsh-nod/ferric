# Closed SiLU Four-Step Inputs

The input schema is `ferric-p228-silu-materialized-decode-inputs-v1`. The plan must
have exactly these keys:

```text
schema output_label baseline parent_cpu worker_cpu parent worker request
projection_image lowering_complete inspection_complete decode_review
parent_runtime_review worker_runtime_review supervisor_tests supervisor_test_sources
layer_comparison mlp_image mlp_cpu mlp_lowering_complete mlp_lowering_owner
```

Every field except `schema` and `output_label` is an exact `{path, bytes, sha256}`
FilePin. `baseline` is the actual SiLU one-layer capture `67a23528...`, not the
old TF4 observation or corrected-residual layer. `layer_comparison` is the actual
independent comparison `828f8fdb...`. Both remain supplied as explicit original
receipt pins. The known checksummed bytes are closed in the intake.

The four fields `mlp_image`, `mlp_cpu`, `mlp_lowering_complete` and
`mlp_lowering_owner` must equal the selection in that genuine layer capture.
The checked lowering retains eight unresolved runtime
requirements and grants no execution authority. The residual image/lowering/
inspection inputs must equal the earlier separately reviewed `25338bea...` image.
`parent_cpu` and `worker_cpu` both identify CPU1022; the old exact Cargo artifact,
source snapshots, profile, stream joins and deployed ELF checks remain unchanged.

`request` uses the existing `FerricFiniteProjectionResidualDecodeRequestV1` wrapper
around `FerricFinitePrefixDecodeRequestV1`. Only the selected `decode.tiles_image`
changes to SiLU. Preserve all original `decode.images`, prompt/source/model/
device values, V7 `prefix_image`, teacher-forced mode and deadlines. Use a fresh
session and the new case's `native` evidence directory. The separately selected
projection-residual image is unchanged. Do not replace `images.mlp` or
`images.residual` with the new selected images.

`decode_review` uses `ferric-p228-silu-materialized-decode-engineering-review-v1`.
Its exact fields are the old review fields plus `mlp_provenance` and all new plan
bindings. `down2_image`/`down2_provenance` remain historical ancestry; `mlp_image`
and `mlp_provenance` identify the actual selected SiLU image. Include substantive
root-authored source-lineage, formal, ISA, coherence, lifecycle and device notes.
The helper never generates authority or marks all runtime premises discharged.

Parent/worker runtime reviews must bind the unchanged selected CPU1022 ELF bodies
and the real current MI350 platform. Prior CPU988 layer reviews are replayed as
historical layer evidence, not substituted for the selected decode executables.

The package contains 14 files plus its manifest, including the pure wrapper.
The actual pure controller digest and 43-test census are pinned in intake. The pure output
schema is `ferric-p228-silu-materialized-decode-gpu-pure-v1`, with a fresh
`silu-materialized-decode-gpu-pure-v228-vN` label. The root wrapper path is
`E/run_silu_materialized_decode_gpu_pure_p228_v1.py`.

All original files must remain available under authenticated E paths, including
the frozen SiLU layer package, old corrected layer/Down2 evidence, actual 15-body
comparison source map, its pure receipt/snapshots/transcript, and original model
inputs. No model or executable is rebuilt. No fresh GPU result is assumed by this
document; root runs the proposal only after review and actual pure qualification.
