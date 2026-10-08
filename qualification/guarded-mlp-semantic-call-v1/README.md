# Canonical Defined-Call Diagnosis

This is a CPU-qualified diagnostic checkpoint for Ferric issue #42, not a
lowering fix, an HSACO result, or GPU/model acceptance. It follows the
[bounded callback-origin correction](../guarded-mlp-inline-fndef-origin-v1/README.md).
All issue #42 milestones remain open.

## What Changed

The existing bounded canonical semantic inspector now accepts:

```text
fe2o3-semantic-capture-inspect MIR MAP MIR_SHA MAP_SHA --call FUNCTION BLOCK
```

The original helper-identity invocation remains unchanged. The new mode
decodes the canonical capture, joins the exact source-map identity, and
reports the defined callee, source argument identities and scalar types,
Copy/Move/constant form, local and projection identities, ABI arguments,
and source spans. It refuses a missing function/block, a non-call terminator,
or a callable without a defined body. Indices must be canonical decimal u32.

Inputs retain the 16 MiB MIR and 1 MiB map limits. Call output is bounded
to 1 MiB, with at most 256 arguments and 64 projections per place. Map paths
are labels, not paths the reader opens. The separate source join uses only
predetermined, hash-pinned provider and fixture files.

The reader does not observe transient Kernel IR bindings, change admission,
or authorize execution. See the [source](source/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs)
and [patch](reader.patch).

## Actual Qualification

MI350 host CPU8/9, nice10, offline Cargo jobs2:

| Check | Result |
| --- | --- |
| Reader tests | 14 passed, 0 failed, 0 ignored |
| Owned phases | 11 natural zero exits, reaped, process groups absent |
| Elapsed | 22.996975 seconds |
| Historical helper-mode JSON and stdout | Byte-identical to the seven-test parent |
| Source and dependencies | Unchanged across the run |
| V12 fixed/early inspection | Both passed against the original failed captures |
| Source-file join | Both passed; independently rehashed and reconstructed |

The seven additional tests cover real canonical Copy/Move/constant arguments,
initialized tuple projection, missing/non-call selections, strict indices,
non-defined call refusal, changed inputs/source joins, and exact output and
projection bounds. These are diagnostic tests, not a new full compiler run.
The original compiler attempts remain failures.

Raw [test output](cpu/reader-tests.stdout), [CPU terminal](cpu/complete.json),
[manifest](manifest.json), and [complete original archive](originals.tar.gz)
are retained. The archive contains 100 members, 99 pinned originals, and
57,993,488 expanded bytes. SHA-256:

```text
CPU terminal: 862acab9993408c8303ba047219b2fca5c2c8a4d9c940807b22569b8f3257a7b
Reader ELF:   17859ee1f2cd56ef74e0b20a00fa55844738665e22e21e6963e362c2acf2893f
Source join:  3585842d8321ae9172aa42ba72e93a63d791f4409970324357973f8ae5af3e78
Archive:      2e3700c0e64b0ef45dbcba6ecec9444c4c658ffcbddc66423892a76d3cdc6652
```

## What The Failure Actually Identifies

Both original V12 errors select caller function2, callee function1, callable1.
The fixed block is307 and the early block is159. The exact provider body
defines `complete_coverage(token: u32, lane: usize, written: usize, valid: bool)`
at lines380-394 and calls it at line798.

| Source argument | Operand | Semantic scalar | Fixed local | Early local |
| --- | --- | --- | --- | --- |
| token | Copy | unsigned32 | 84 | 1558 |
| lane | Copy | unsigned64 | 1206 | 1312 |
| written | Move | unsigned64 | 1468 | 598 |
| valid | Move | Bool | 927 | 653 |

All four source-semantic types match their expected parameter types. None
has a source-place projection. This narrows the failure to the later lowered
call binding/signature comparison; it does not establish which argument or
Kernel IR type differs.

The exact reports are [fixed](inspections/call-fixed.json),
[early](inspections/call-early.json), and the
[independently audited source join](source-join/result.json).
The lane originates from a thread index, making Index/U64 transport a
specific hypothesis, not an observed cause. The next step is a bounded
lowerer error payload and a genuine Index-valued helper-call regression,
then the original compile pair. Any conversion must preserve exact types,
ownership and existing refusal cases.

## Remaining Gates

There is no new HSACO, GPU execution, independent model output, throughput,
or 700 tokens/s claim. The unresolved device completion hold remains in
force; neither process disappearance nor a diagnostic pass releases it.
Native work needs recovery evidence or a separately qualified device set.
Separate gfx942 and gfx950 native/model/lifecycle evidence remains required.
