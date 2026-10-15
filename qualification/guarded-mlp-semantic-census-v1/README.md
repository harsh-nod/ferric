# Bounded Canonical Call Census

This CPU diagnostic checkpoint advances Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42).
It is not a lowering fix, HSACO result, GPU acceptance, or model benchmark.
All M0-M7 milestones remain open.

## Interface And Bounds

The existing read-only canonical inspector adds one closed invocation:

```text
fe2o3-semantic-capture-inspect MIR MAP MIR_SHA MAP_SHA --census
```

It inventories functions and Call/TailCall sites in source-semantic order,
including full identities, callable indices, defined callee indices, and
source provenance. It does not expand every argument/type or observe
transient Kernel IR bindings.

The original helper and defined-call modes retain their behavior and
strict source-map checks. All four historical V8/V12 stdout streams are
byte-identical. No production admission predicate, provider, compiler
lowering, runtime path or GPU kernel changed.

| Bound | Maximum |
| --- | ---: |
| Canonical MIR input | 16 MiB |
| Source-map input | 1 MiB |
| Functions inventoried | 128 |
| Blocks scanned | 131,072 |
| Call/TailCall rows | 4,096 |
| Serialized output, excluding newline | 1 MiB minus 1 byte |

Counts are checked before retaining rows. Serialized byte accounting
includes the header, both arrays and row separators; inclusive limits and
one-byte-too-small refusal are tested. Source inputs retain canonical-path,
stable-file, exact-digest, strict-map and canonical-decoding checks.

## Incomplete Maps Are Explicit

The first census attempt passed all 18 tests but refused V16 because its
map omits some macro-expansion files. Its failed receipt and original
streams remain in the archive. A fresh generation reports these origins
as `FileAbsent`, retaining the full source identity, byte range and
coordinates, but assigning **no path or extent**. All source-content
verification and execution-authority flags remain false. Known mapped
origins still reject out-of-extent spans. This is partial diagnostic data,
not a repaired map or additional execution authority.

All four captures contain 58 missing origins across the complete census.
For the defined-function/call subset used below, 11 expansion origins per
arm remain unresolved. The source join does not invent content for them.

## Actual Qualification

On SSH host mi350, CPU8/9, nice10, offline Cargo jobs2, 12 GiB service memory
cap and swap disabled:

| Check | Actual result |
| --- | --- |
| Reader tests | 18 passed, 0 failed, 0 ignored |
| Owned phases | 15 natural zero exits, reaped, groups absent |
| Wall time | 24.666519 seconds |
| Closed inputs | 5,844, including 32 retained capture/terminal/stream files |
| Source and dependencies | Unchanged across qualification |
| Historical helper/call stdout | Four streams byte-identical |
| V16/V17 census invocations | All four passed |
| Independent source join | Exact compile snapshots, byte spans and coordinates passed |

The original 14 tests are retained. Four additional tests cover defined
and non-body call census, exact count/output limits, digest and source-map
identity/extent checks, explicit unresolved origins, closed CLI arguments,
and an actually admitted TailCall while the legacy call selector still
refuses TailCall.

See the [reader source](source/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs),
[patch](reader.patch), [test stdout](cpu/reader-tests.stdout),
[CPU receipt](cpu/complete.json), [manifest](manifest.json), and
[complete original evidence archive](originals.tar.gz).
The archive contains 148 members, 147 pinned originals and 94,142,186
expanded bytes. Every member was reread and compared byte-for-byte on
mi350.
An additional [data-only audit](publication-audit.json) independently
rehashed all 5,844 qualification inputs and all 147 archive originals,
checked every member and exported report, and confirmed the four historical
stdout comparisons. It did not execute project code or use a GPU.
The exported reader also passes `rustfmt --check` on mi350.

## What The Captures Establish

| Capture | Functions | Blocks | All calls | Defined calls |
| --- | ---: | ---: | ---: | ---: |
| V16 fixed | 3 | 693 | 135 | 12 |
| V16 early | 3 | 696 | 135 | 12 |
| V17 fixed | 2 | 690 | 134 | 11 |
| V17 early | 2 | 693 | 134 | 11 |

The V16 census independently identifies `complete_coverage` plus one
BF16 predicate helper. V17 no longer contains the coverage helper as a
defined function: all 11 remaining defined calls target the same helper.

That V17 helper has identity:

```text
c00e88a5081a31be7b195d3d172c060ebdf65ad43f6f420f2a476a9cdb427afb
```

Its actual source is `Bf16::is_finite`, `half.rs:209-211`, bytes6723-6803:

```rust
pub const fn is_finite(self) -> bool {
    (self.0 & 0x7f80) != 0x7f80
}
```

The source is independently SHA-256 pinned to
`0d7b2193ec67c34919181a1ecefff23d24ee9af45ec567c5a4f57772792f10af`
and joined to both original compile source snapshots. A source-file
identity in the semantic map is a compiler identity, **not** this content
hash. An initial preliminary `to_f32` name assumption was rejected by
the source-body check before publication; the accepted join uses only
the exact body above.

V17 semantic function0 calls helper1 at these independently inventoried
blocks:

| Arm | Source-semantic block indices |
| --- | --- |
| Fixed | 19, 64, 237, 244, 267, 346, 373, 380, 403, 486, 540 |
| Early | 44, 118, 119, 135, 166, 189, 296, 398, 494, 581, 679 |

These are census-derived semantic selectors, **not** selectors inferred
from the compiler's Kernel IR error numbers. The unchanged strict call
inspector cannot resolve their absent expansion-map records. Neither
this census nor the source join proves a KIR-to-semantic call mapping or
that this helper alone causes the later incomplete-effect refusal.

Reports: [V16 fixed](census/v16-fixed.json),
[V16 early](census/v16-early.json),
[V17 fixed](census/v17-fixed.json),
[V17 early](census/v17-early.json), and the
[independent source join](source-join/result.json).

## Next Bounded Repair

Test AMDGPU-only force-inlining of this exact `Bf16::is_finite` body,
with source-identity refresh and regressions retaining host behavior and
NaN/infinity/subnormal boundary behavior. Qualify the exact source and
compiler/device tests before a fresh fixed/early worker compile pair.
The candidate must clear the original refusal without weakening local
effect summaries or any validation predicate. It remains a hypothesis
until those original compilations run.

Both original V17 compiler attempts remain natural status1 failures with
no HSACO. No worker compilation or GPU job was submitted in this census
step. The unresolved device-completion hold remains closed; CPU diagnostics
and absent PIDs do not release it. No new numerical result, throughput,
700 tokens/s or milestone completion is claimed.

## Receipt Digests

```text
CPU:         60898b32e9d93cdae3ec5359177055213898dc27c0e14e5bf68b5d727376a5a7
Reader ELF:  601bfbed7f0c7e9707b2d453739127abfeebfaa98a984f1546a036247a4c1d67
Reader src:  d28f1efe055896f3c95a425217f7134136ccde572ce477ba8d0c094966bd464c
Source join: 8f9318ebc9136aab08eea1963b457e82ed243ef78e293e2822e0cc9e25c13a6e
Failed V1:   c789b43cbc42e30cef017fb810f2c5856c610d91492e429edacbb8636d4dadda
Archive:     4a3664fd69b15d953ac443bb72be64127e8dd1792ae36772627b79d06434e6d1
```
