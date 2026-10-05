# Guarded MLP Segment Kernels

Experimental gfx950 device kernels for a fixed TP2 segment. This crate is not
connected to Ferric's inference path. CPU qualification, checked HSACO lowering,
native negative controls and reusable-arena validation are separate gates.

## Device Contract

`ferric_qwen3_mlp_state_guard_v1` uses one Wave64 workgroup. Lane zero reads all
548 atomic state words and checks the complete runtime MLP terminal predicate,
including owner ranges and arrival counts. Constant indices keep this within
the checked indexed-atomic lowering model. No completion flag substitutes for
full state validation.

The four atomic guard words are generation low/high halves, verdict, and reserved
zero. Verdict 0 is pending, 1 valid, 2 invalid. The validator writes the tag and
reserved word before publishing the verdict with Release.

`ferric_qwen3_tp2_guarded_projection_residual_bf16_v1` uses 64 Wave64 workgroups
for 4,096 elements. It acquires both guards and requires both exact nonzero
generation tags, valid verdicts and zero reserved words before any payload
access. A rejected guard returns normally without reading or writing payloads;
the host must reject the segment. Malformed invocation geometry/extents or a
zero requested generation may trap.

The arithmetic preserves the existing projection-residual kernel's FP32 rank
sum, intermediate BF16 materialization and final BF16 residual addition.

## Ownership and Validation

The runtime must provide genuine atomic storage, distinct nonaliasing buffers,
ordered producer/validator completion, unique generations and no concurrent
reset. These kernels do not authorize arena reuse, ring overwrite or peer release.

Dependencies select fe2o3 commit `097b4f796a283f554339cc0c0ef5c2c8c3858d2a`,
which supports typed atomic slices. This is a new dependency generation, not a
claim of identity with an older qualified compiler snapshot.

The [MI350 CPU qualification](../../qualification/guarded-mlp-segment-cpu-v1/README.md)
passed all 27 tests, plus seven harness regression tests. The kernel tests cover
staged arithmetic, every state word against an
independent range-based oracle, all valid owner values, exact load coverage,
guard publication order, both tag halves and no payload access on either invalid
guard. CPU success alone cannot establish device ordering or emitted control-flow
dominance. Actual ABI, LLVM/ISA, GPU results and model performance require their
own evidence.
