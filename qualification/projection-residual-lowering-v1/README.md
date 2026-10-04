# BF16 Projection Residual: Checked gfx950 Lowering

The [CPU-qualified Rust candidate](../projection-residual-cpu-v1/README.md)
passed the retained generic `cargo-fe2o3 engineering hsaco` pipeline on
`mi350-2` on 2026-10-04. This is compilation and static inspection evidence,
not GPU execution or independent model numerical acceptance. The current
runtime and V18 image remain unchanged.

## Actual Compilation

The exact formatted source and lock used by the 31 passing CPU tests were
compiled with the pinned FAA fe2o3 compiler and LLVM22 worker, targeting
`gfx950:xnack-`, code-object version 6. Checked compiler execution and
exact-output replay passed. The process exited naturally, its owned group
was empty, and source, tool, provider and prior-target postchecks passed.
Compilation took 342.44 seconds; this is not a kernel timing measurement.

The emitted image is 10,864 bytes, with SHA-256:

```text
25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25
```

The actual lowering completion SHA-256 is
`4f46acb4eaaadefc2c1426146424bda91a7e455ab3b57b23c935401e9c4d79bb`.
The compiler observation still grants no load, launch or production authority.
No GPU was opened by compilation or inspection.

## Emitted ABI

Three bounded inspection commands passed: checked descriptor inspection,
ELF notes/symbols, and gfx950 disassembly. All three exited naturally with
unchanged input/source/tool postchecks.

| Property | Actual Result |
| --- | ---: |
| Exported kernels | 1 |
| Explicit arguments | 22 |
| Explicit argument bytes | 168 |
| Reserved hidden argument bytes | 256 |
| Executable kernarg bytes / alignment | 424 / 8 |
| Required workgroup | 64 x 1 x 1 |
| Maximum workgroup grid | 64 x 1 x 1 |
| Wave size | 64 |
| VGPRs / SGPRs | 6 / 36 |
| VGPR / SGPR spills | 0 / 0 |
| Shared / private segment bytes | 0 / 0 |
| Dynamic stack | false |

The sole export is
`ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1`. Ten pointer/length
pairs occupy bytes 0-159; `rows` and `world` occupy bytes 160 and 164.
The thirteen declared hidden arguments were checked individually, not just
through their total reserved extent.

## Static Arithmetic Review

Manual review of the retained [disassembly](inspection/disassembly-stdout)
found the intended finite-input data path:

| Address | Operation |
| --- | --- |
| `0x1f44` | FP32 `+0 + rank0` |
| `0x1f98` | FP32 addition of rank1 |
| `0x1fc8`, `0x1fd4` | BF16 ties-to-even bias: retained bit 16 plus `0x7fff` |
| `0x1fe4`-`0x1ffc` | Rounded projection finite check before residual load |
| `0x2010` | BF16 residual load |
| `0x201c`, `0x202c` | Rounded projection mask and residual widening |
| `0x2030` | FP32 projection/residual addition |
| `0x2044`, `0x2058` | Final BF16 rounding sequence |
| `0x20a4` | Guarded BF16 output store |

The integer mask makes the first rounding observable before the residual
addition; neither rank partial is independently narrowed. These data-flow
notes are not a formal hardware-arithmetic proof or a GPU correctness result.
The automated inspection deliberately makes no arithmetic-acceptance claim.

## Inspection History

The first inspection's three tools succeeded, but its checker required the
wrong LLVM target-string spelling. Actual LLVM22 metadata says
`amdgcn-amd-amdhsa--gfx950:xnack-`. The second controller changes only that
exact string comparison and passes. Both runs inspected the same unchanged
image and produced byte-identical descriptor, ELF and disassembly outputs.
The [failed first receipt](history/inspection-v1/failed.json) is preserved;
it is not relabeled as success, and the kernel was not recompiled for the retry.

## Evidence and Next Gate

[Publication ledger](result.json), [compiler completion](lowering/complete.json),
[compiler observation](lowering/observation.json), and
[inspection completion](inspection/complete.json) retain the actual results.
The binary and large immutable-input/target inventories remain outside Git
with their hashes recorded. The generic command does not retain LLVM IR.

Next is CPU qualification of the separately named parent/worker route,
followed by genuine candidate GPU capture and independent layer-zero
comparison. The earlier conditional replay still has two residual differences
against the framework. No full-model numerical acceptance, throughput gain,
overlap claim, or 700 tokens/s result follows from this checkpoint.
