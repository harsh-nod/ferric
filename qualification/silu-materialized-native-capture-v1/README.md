# BF16 SiLU Materialization: Native Layer Capture

The [checked SiLU image](../silu-materialized-lowering-v1/README.md) executed on
MI350 in one engineering layer-zero capture. Both TP ranks completed, all 28
arrays were retained, and all 22 pre-SwiGLU arrays matched the previous
corrected-residual capture byte-for-byte. The parent and worker exited cleanly;
seven owned process leaves and six idle-device audits passed without a retry or
forced cleanup.

This is a real GPU execution and structural checkpoint, not full-model numerical
acceptance or a throughput result. It uses token 9112 at position zero. It is not
the 2,048-token prompt / 256-token sustained workload.

## Isolated Change

The request selects the new MLP image, SHA-256
`b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589`.
The CPU988-qualified parent and worker, V7 prefix, corrected projection-residual
image, original bootstrap images and native protocol remain unchanged. Only the
MLP image and fresh session/output identifiers change.

The new kernel rounds FP32 SiLU to BF16 before multiplying by up. The exponential,
division, two-row Down implementation, task geometry and synchronization are
unchanged. Manual LLVM and ISA inspection preceded the GPU attempt; the image
retains the previous register/LDS footprint and reports no spills/private storage.

Each rank's norm, QKV, query, logical key/value row, attention, output partial,
first residual, MLP norm, gate and up match the baseline. The full caches are also
checked for untouched-region preservation. Activation, Down partial and final
hidden values are allowed to change; equality to the old hidden output is not an
acceptance condition.

## Retained Evidence

[result.json](result.json) and [complete.json](complete.json) bind the exact run,
28 tensor ranges, binaries, image, raw records and source hashes. The request,
root review, verbatim runtime reviews, assembly inputs, native JSON records,
seven process records and six device snapshots are included. Binary tensor
payloads, ELF executables and HSACO remain outside Git with their exact pins.

The publisher rechecks all captured bodies and the structural validator; it does
not rerun the kernel, replay every transitive compiler input, or rehash system
library bodies locally. The separate 36-test policy suite is retained in [pure](pure).

The observed outer wall time was 347.227 seconds, including model loading,
validation and capture. It is not kernel latency, inter-token latency or a
tokens/s measurement. The [independent comparison](../silu-materialized-comparison-v1/README.md)
records the actual numerical differences separately. Production admission,
full-model acceptance and the 700 tokens/s target remain open.
