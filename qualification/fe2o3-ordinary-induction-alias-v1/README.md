# Ordinary Induction Temporary

This is a CPU-tested compiler candidate with a checked engineering HSACO,
not a production admission or GPU numerical result. The [patch](compiler.patch) extends the exact previously qualified
[idempotent compiler generation](../fe2o3-partial-move-idempotent-v1/README.md).
[Source pins](source-pins.json) identify its required preimage and both changed
files. Apply/reverse validation reproduced the exact intended bodies.

## Why This Change

The retained reciprocal V3 semantic capture fits the existing storage and
1,024-block limits, but its counter update has this form:

```text
temporary = Add(counter, 1)
counter = Move(temporary)
```

The existing induction recognizer accepts a direct addition or field zero of
an authenticated checked-add result, but rejects this ordinary temporary.
Changing the Rust counter to checked `+=` passes CPU arithmetic tests yet
exceeds the unchanged semantic-SSA storage budget. This candidate instead
recognizes the V3 form without changing its source arithmetic or raising caps.

## Admission And Proof

The added route requires a distinct, exactly typed temporary with one static
definition, no address escape, and an ordinary `Add` producer immediately
before its copy/move in the same latch block. It rejects cross-block producers,
intervening statements, copy chains, redefinitions and checked/unchecked
temporary producers. Existing metered operand resolution is reused.

The existing topology, two-definition induction, uniform bound and positive
step checks remain mandatory. For a live loop body, the existing unsigned
proof bounds the largest update by `(bound.max - 1) + step` and requires that
it fit the exact source type. Tests exercise the accepted one-step limit and
overflowing two-step update for both u32 and u64.

The proof record includes the producer block, statement and temporary local.
Reconciliation recomputes and compares them, the update kind and the step.
This route grants **no overflow-assertion elision**. Existing direct checked
and unchecked routes, work budgets and partial-move accounting are unchanged.

## Executed Validation

The [fresh `mi350-2` CPU result](cpu-result.json) records:

- 1,487 Pliron tests passed; one existing test ignored.
- 1,196 compiler tests passed; 24 existing tests ignored.
- All eight new induction regressions and six previous idempotent regressions ran.
- All 17 controller policy tests and the four-phase patch round trip passed.
- Fresh extractor, backend and both test binaries built.

Before the new overlay, all 5,780 source entries matched the prior qualified
generation by relative path, size and hash. The new tree differs only in the
ranked projector and added test file. All ten owned commands exited naturally;
source, tool, dependency, configuration and four prior-target postchecks passed.
The owned tree was reaped without forced cleanup. All 5,873 archive members,
including 5,781 source files and five selected products, were rehashed locally.

The [matching finalizer tools](finalizer-result.json) subsequently built from
this exact source generation and passed 190 default tests, with 15 ignored.
All 14 controller policy tests passed. The controller also replayed both
compiler suites' retained listings and results before building the tools.
Both overlays, the prior source generation and all five compiler products
remained pinned. All six phases exited naturally and source, dependency,
tool/configuration and four old-target postchecks passed. The 56 archive
members, three new tools, five unchanged compiler products and full source
snapshot were rehashed locally. These are CPU results, not GPU performance.

## Checked Image

The real V3 source has now [passed the checked-lowering leaf](checked-lowering-checkpoint.json)
on `mi350-2`, naturally exiting zero in 1,771.339 seconds, within its unchanged
1,800-second deadline. It produced semantic MIR, neutral and target KIR V11,
gfx950 LLVM and a 4,022,416-byte inert compiler handoff. This clears the earlier
induction-latch refusal for this source/compiler pair.

That record remains an **interim snapshot**. The subsequent
[terminal nine-phase probe](checked-probe-result.json) passed, including the
separately selected actual-capture replay and inert-join tests. Replay completed
in 1,401.55 seconds; default-suite ignored counts are unchanged. All nine phases
exited naturally, source/dependency/tool and five prior-target postchecks passed,
and the owned processes were reaped without forced cleanup. All 92 retained
archive members were rehashed locally.

The compiler and finalizer produced a 53,560-byte gfx950 HSACO, SHA-256
`4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5`.
Its descriptor specifies a 64-thread workgroup and at most 64 workgroups.
Metadata reports 512 bytes of LDS, 106 SGPRs with 16 spills, 132 VGPRs and no
VGPR spills. These are compiler resource reports, not measured occupancy or
performance. LLVM, formal archive, descriptor metadata and ISA are retained.

## Remaining Gates

The emission receipt still reports eight undischarged runtime requirements,
unsupported generic proofs and no production authority. Fresh actual-image
arithmetic/ISA reviews, deployment and MI350 numerical validation remain
required. No new GPU execution, full-model certificate or 700 tokens/s result
is claimed. Prior-target checks compare stat inventories, not proof of no
transient writes.
