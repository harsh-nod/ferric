# BF16 Finiteness Predicate Inlining

This is an engineering checkpoint for Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42),
following the [canonical call census](../guarded-mlp-semantic-census-v1/README.md).
The exact source change is CPU-qualified and independently exercised by
both original gfx950 worker compilation attempts. **Both compilations
still fail, and no HSACO or GPU/model result is claimed.** All M0-M7
milestones remain open.

## Narrow Source Change

The independently verified remaining helper was `Bf16::is_finite`.
Only an AMDGPU-specific inline attribute was added; its const signature,
body and host behavior remain unchanged:

```rust
#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
pub const fn is_finite(self) -> bool {
    (self.0 & 0x7f80) != 0x7f80
}
```

The reviewed provider source-closure identity is refreshed to:

```text
9d40f73a5afd4a4dbdc36d5dfd77378584255fcd4a8dae06d0fcf0958af62c81
```

Only two project files differ from the qualified coverage-inline parent:
[half.rs](source/crates/fe2o3-device/src/half.rs) and
[trusted_device_items.rs](source/crates/rustc-codegen-fe2o3/src/trusted_device_items.rs).
The latter updates the provider constant, its duplicated positive test
literal, and adds the new exact-source/old-identity refusal regression.
The [patch](correction.patch) contains the complete change.

No effect-summary predicate, helper ABI, lowering conversion, resource
limit, runtime path, or ordinary/conditional handoff policy was relaxed.
BF16 conversion terminals retain their existing out-of-line policy.

## Regressions

The new device test checks all 65,536 BF16 bit encodings against the
independent host `f32::is_finite` classifier, constructing the widened
f32 bits directly rather than calling the BF16 implementation. It checks
65,280 finite and 256 nonfinite encodings, const evaluation and unchanged
input bits. Signed zeros, subnormals, boundary normal values, positive
and negative infinities, and signaling/quiet NaN encodings are covered.

The new compiler test checks the exact unchanged predicate body and
AMDGPU-only attribute in the BF16 implementation, accepts only the
current reviewed source closure, and rejects the previous identity.
Existing source-authentication, BF16 conversion, compiler, lowering,
aliasing, effect and dual-target LLVM tests remain present.

## Actual CPU Qualification

SSH host mi350, CPU8/9, nice10, offline Cargo jobs2; owned service with
12 GiB memory maximum and swap disabled:

| Suite | Passed | Failed | Existing ignores |
| --- | ---: | ---: | ---: |
| Compiler library | 1,341 | 0 | 24 |
| Device library | 361 | 0 | 0 |
| Lowering integration | 125 | 0 | 0 |
| Lowerer library | 769 | 0 | 0 |
| Total | 2,596 | 0 | 24 |

All 19 owned phases finish naturally with status0, reaped leaders and
absent process groups. Qualification takes 273.000213 seconds; source,
dependencies, toolchain, Rust source ledger and selected artifacts pass
the final checks. The two edited files pass `rustfmt --check` with
sibling-module traversal disabled.

Two earlier qualification failures are retained, not overwritten:

| Generation | Failure | Actual scope |
| --- | --- | --- |
| V1 | Source-scope guard caught formatter changes to sibling modules | Zero compiler phases |
| V2 | Positive source-order test still expected the old literal source hash | 1,340 passed, 1 failed, 24 existing ignores |
| V3 | Two intended source files only; duplicated positive literal refreshed | Full qualification above |

V1's unexpected formatted files are excluded from V3. V2's negative
old-identity refusal already passed and remains unchanged in V3.
The archive retains failed receipts, inputs, raw streams and source
pre/postimages.

## Original Worker Compile Pair

Both arms use the qualified provider/backend and original checked route,
normalization, target, tools, immutable input snapshots, limits and
cleanup rules. Their source/tool postchecks pass and both child groups
are reaped and absent.

| Arm | Natural exit | Elapsed seconds | New refusal | HSACO |
| --- | ---: | ---: | --- | --- |
| Fixed rounds | 1 | 186.264686 | V2-only extraction drops conditional runtime premises | None |
| Early stop | 1 | 121.092426 | Uniform induction region lacks one unique header exit | None |

The previous incomplete defined-call local-effect refusal is absent from
both actual streams. This is progress to two later boundaries, not
permission to remove either check. See the original
[fixed receipt](pair/fixed/result.json), [fixed stderr](pair/fixed/compile.stderr),
[early receipt](pair/early/result.json), and [early stderr](pair/early/compile.stderr).
The compile timings are diagnostic wall times, not kernel performance.

## Retained Census Confirms Removal

A separate bounded service applies the previously qualified, hash-pinned
reader to the two newly retained canonical captures. Both reader phases
finish naturally with status0, and all inputs remain unchanged; neither
worker is recompiled during this inspection.

| Capture | Functions | Blocks | All calls | Defined helper calls |
| --- | ---: | ---: | ---: | ---: |
| V17 fixed, before | 2 | 690 | 134 | 11 |
| V17 early, before | 2 | 693 | 134 | 11 |
| V18 fixed, after | 1 | 688 | 123 | 0 |
| V18 early, after | 1 | 691 | 123 | 0 |

This establishes removal of the remaining defined helper in the
source-semantic captures. It does not claim a fabricated mapping from
Kernel IR error block numbers. Unmapped expansion provenance remains
explicitly unresolved, with execution and production authority false.

The [inspection receipt](retained-census/complete.json),
[fixed census](retained-census/fixed.json), and
[early census](retained-census/early.json) retain the exact reports.

## Next Boundary

The fixed-round attempt reaches the deliberate ordinary V2 extraction
guard in `production_pipeline.rs`. That route cannot discard conditional
runtime premises. The existing semantic V3 handoff method retains lineage
and requires an actual canonical invocation descriptor; a raw environment
toggle or invented descriptor/custody is not an acceptable repair.

The next bounded step should first exercise the existing actual-capture
MLP tile formal/archive/outer-lineage replay test against the newly retained,
exact-digest fixed capture, using the already qualified compiler test
binary and current bounds. Its explicit capture/version checks must stay
closed. That evidence can then guide a lossless V3 frontend/handoff route,
preserving the ordinary V2 refusal and all runtime proof gates.

The early-stop induction-region failure remains separate. It needs an
exact CFG/source diagnosis and genuine exit-structure regression before
any repair; no acceptance rule is weakened here.

## Evidence And Limits

[CPU receipt](cpu/complete.json), [compiler stdout](cpu/compiler-tests.stdout),
[device stdout](cpu/device-tests.stdout),
[integration stdout](cpu/lowerer-tests.stdout),
[unit stdout](cpu/lowerer-unit-tests.stdout), [manifest](manifest.json),
[data-only audit](publication-audit.json), and
[complete original evidence archive](originals.tar.gz) are retained.
The archive has 243 members, 242 pinned originals and 101,278,702 expanded
bytes. Each member is reread byte-for-byte during export. A separate
data-only audit rehashes all 5,812 source/controller inputs, selected
artifacts and all archive originals without executing project code.

```text
CPU:         41d0425beff3ebef1c98910bc904402452c83cddbb8c72229cb9c05e42787a1a
Backend:     de3bb7de38b816a4ccc9ad70c16da711a0f5141b1590ac00cd5f3927099492ee
Fixed:       65f856d211c78cdbc4322060fbfea200f78299b8504032ff4f55db8c4e897017
Early:       6044ec0b2749633d4f4ef70f116e2635e7288b0d0fc301f6c2178252cc34a9b1
Census:      663b439ade8177ef733de4ced54e1a0f556173f5db147d8138a6be6f13bfb0f0
Data audit:  2626b9a5fdfaf86878a7a9e1bf502ecef37db0f0d65b4941909e140f9210ee34
Archive:     65008a7eb7aeb5435beac87a56a3138568ca6fdba18552a282b91894f6e4c68d
```

No GPU API, probe, reset or launch was used. The unresolved device-completion
hold is not released by CPU passes or absent process IDs. No new native
numerical result, model decode, speedup, 700 tokens/s result or milestone
completion is inferred.
