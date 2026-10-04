# BF16 SiLU Materialization: Checked gfx950 Emission

The separately selected SiLU candidate completed all nine compiler stages on
ASROCK through `mi350-2`, following its [38-test CPU qualification](../silu-materialized-cpu-v1/README.md).
The compiler owner exited naturally, reaped its processes, and passed its source
and tool postchecks. This checkpoint does not execute or numerically accept the
new GPU image, and does not change a production route.

## Actual Compiler Result

The retained stages are fixture metadata, checked Rust lowering, actual compiler
replay, inert-lineage join, emission, archive extraction, descriptor metadata,
ELF notes and disassembly. The original checked fe2o3 path is preserved; no
hand-authored LLVM, ISA or HIP substitute was used.

The emitted gfx950:xnack- code object is 33,320 bytes, SHA-256
`b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589`.
Its selected symbol and launch ABI remain unchanged. Compilation took about
1,180 seconds including the owner checks; this is not GPU execution time.

| Reported Resource | Previous Down2 | SiLU Candidate |
| --- | ---: | ---: |
| SGPRs | 106 | 106 |
| VGPRs | 106 | 106 |
| SGPR / VGPR spills | 0 / 0 | 0 / 0 |
| LDS bytes | 512 | 512 |
| Private segment bytes | 0 | 0 |
| Wave / maximum workgroup size | 64 / 64 | 64 / 64 |
| Kernarg bytes / alignment | 344 / 8 | 344 / 8 |

## Arithmetic Inspection

The actual [LLVM](lowering/extracted/module.ll) contains the original gate
absolute value, negative exponential argument, sign-selected numerator and FP32
division. The resulting FP32 SiLU now feeds BF16 RNE narrowing and widening
before multiplication by widened up, followed by the existing final BF16 RNE.
The internal narrowing and widening helper definitions match the previous
Down2 emission byte-for-byte.

The [inspection record](inspection/complete.json) retains exact SSA dependencies,
line-numbered excerpts and actual resource fields. Its literal recognizer is
not a complete control-flow or final-ISA proof. Manual review remains a separate
requirement before an engineering GPU attempt; the publisher does not grant ISA
acceptance or runtime authority. OCML/framework equality is not established.

## Evidence and Remaining Gates

[result.json](result.json) binds the actual owner, lowering, CPU and inspection
receipts. The seven-file [compiled fixture](fixture), controller sources, selected
compiler streams, LLVM, ELF notes and disassembly are included. All phase command,
start, result and stream bodies were rehashed; bodies not copied into Git, binary
artifacts and large source snapshots remain retained with their exact pins.

The 27 lowering-policy tests and 36 capture-policy tests passed separately on
ASROCK, with no failures or skips. They test controllers, not additional GPU
arithmetic cases. The capture controller is retained here for reproducibility;
its presence is not a successful native capture.

Eight runtime obligations remain undischarged. Genuine GPU execution, independent
numerical comparison, full-model acceptance, sustained 2,048/256 decoding and the
700 tokens/s target remain open. No acceptance tolerance or performance claim
was introduced.
