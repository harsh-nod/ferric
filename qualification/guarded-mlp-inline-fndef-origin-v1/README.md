# Bounded Inlined Static-Callback Source Authentication

This qualification candidate addresses the fully inlined `FnDef` callback
source-origin boundary observed while compiling the Qwen MLP worker.
It is not production admission, a working GPU image, a numerical result,
or a performance claim. All Ferric issue #42 M0-M7 milestones remain open.

## Results

All authoring, builds and tests in this packet ran on `ssh mi350`.
The private backend qualification completed eleven clean phases in
226.9686833950691 seconds.

| Check | Actual result |
| --- | --- |
| Full compiler library | 1,336 passed, 24 inherited ignores, 0 failed |
| Full device library | 359 passed, 0 ignored, 0 failed |
| New real-rustc regression tests | Five passed, including 19 negative conditions |
| Fixed-round V12 compilation | Natural exit 1; no HSACO; 119.29362021898851 seconds |
| Early-stop V12 compilation | Natural exit 1; no HSACO; 119.82043263502419 seconds |
| New GPU execution or model output validation | Not run |
| gfx942 compilation and native validation of this candidate | Not run |

The two compile times are whole probe elapsed times, not kernel timings.
The compiler leaves retired naturally in 118.366556754 and 118.880195303
seconds. Both were reaped, their process groups were absent, and there were
no timeouts, forced cleanup, storage failures or source/tool postcheck errors.

The previous callback-origin and indirect-enum helper boundaries no longer
stop these attempts. Both now fail during pre-ranked Kernel IR materialization:

- Fixed: function 2, block 307, `defined call argument type changed`.
- Early: function 2, block 159, `defined call argument type changed`.

The rejecting check compares the lowered Kernel IR binding type with the
callee's planned parameter type. The preceding semantic type-ID comparison
is a separate check. The exact failing callee, argument and lowered types
have not yet been independently joined from these captures; do not infer
a specific cause or a safe conversion from this diagnostic alone.

## Source Correction

[The patch](correction.patch) changes exactly two compiler files. The device
provider's 53-file source closure is unchanged at
`91b3c53fa4235b478427f4569d1d0b77ccbf996ac509dba8e58fc36aa828f950`.

[collector.rs](source/crates/rustc-codegen-fe2o3/src/collector.rs)
retains the original resolver's exact static `FnDef`, `FnMut::call_mut`,
argument-tuple and fully monomorphized instance checks. A callback that remains
in the executable closure still follows the existing path.

For a fully inlined callback that has disappeared from that closure, the new
fallback requires its actual Rust compiler source-scope tree to establish:

1. The exact body owner and normalized shim instance.
2. A real inlined `Item` with the exact normalized target instance.
3. That the matching shim is its immediate inlined ancestor.
4. Agreement between the complete lexical ancestry and cached inlined ancestry.
5. Termination at the actual outermost source scope, without invalid indices,
   detached ancestry or cycles.
6. Sufficient work remaining in one closure-wide bounded scan/walk budget.

The source-only graph edge still passes the existing unsafe-signature,
unsafe-HIR and reviewed-provider checks. The fallback does not invent an
executable function, change executable call edges, or admit indirect enum or
pointer helper arguments. It extends source-origin authentication; it is
not accurately described as unchanged admission.

[The regression tests](source/crates/rustc-codegen-fe2o3/src/collector/static_fndef_source_origin_v1_tests.rs)
use actual optimized rustc MIR. They verify the safe fully inlined case,
retained unsafe-body rejection, generic owner/target normalization, exact and
shared work bounds, and 19 rejection conditions. Those conditions include
wrong owners, wrong instantiations, missing origins, invalid scope indices,
lexical/cached cycles, sibling shims and a jointly detached lexical/cached
chain. The seven existing tests and fixture prefix remain unchanged.

## Evidence

The [manifest](manifest.json), [source review](source-review.json) and
[originals archive](originals.tar.gz) retain the exact source preimages and
postimages, patch, controllers, input manifests, original commands, full raw
CPU results, compiler-product identities, and both failed compile attempts
with canonical semantic captures and source maps.

The archive contains 112 members, including its own manifest; 111 entries
are individually joined to retained files. Every member was read back.
It is 7,697,630 bytes with SHA256
`0f1935dafe0645efd7968bee63f36138f2454cdb03a4ddbc26594febe15ca229`.

| Record | SHA256 |
| --- | --- |
| CPU qualification | `3d9f2fcecf68c702014ac9fce127fd35a20ab1fdde1231dcfcedee1dee910c29` |
| Final source map | `101fe1a4d3d04e5212e52d968017821df53f2572d2ea5c5fdb71c933f5ee049e` |
| Qualified backend ELF | `f9209a6d96291048b711c05e76e3f6325f849f0f059cecff92e5d72228bcafdd` |
| Fixed compile result | `8eeda87c5e60b46c2473ce59caacabe50ce5834b2020696f93a305d50094d9df` |
| Early compile result | `747d0f1e63ccf9cae6769260da1112878e8255407f5f1f8ddb5d36fad86ecb21` |

Directly readable results:
[CPU terminal](cpu/complete.json),
[compiler tests](cpu/compiler-tests.stdout),
[device tests](cpu/device-tests.stdout),
[fixed result](pair/fixed/result.json),
[fixed stderr](pair/fixed/compile.stderr),
[early result](pair/early/result.json),
[early stderr](pair/early/compile.stderr).

The new compile fixtures differ from V11 only in dependency-root relocation
and `execute_task` changing from `inline(never)` to `inline(always)`.
The fixed/early fixtures differ only in the worker entry point.
Original lock semantics, frontend/tools, optimization profile and runtime
source are unchanged.

CPU qualification retained CPUs 8/9, nice 10, explicit Cargo jobs 2, the
3,600-second whole/1,800-second leaf bounds and 50-second cleanup reserve.
Compile attempts retained the 600-second leaf and 50-second cleanup reserve,
12 GiB address-space cap, 6 GiB cache cap, 64 MiB stream cap, 1 GiB file cap
and 40/38 GiB initial/live free-space floors. The compile frontend clears
its inner Cargo environment; the outer jobs-2 setting is not an explicit
inner compilation job limit.

The archived commands reference occupied, immutable evidence directories.
Reproduction needs a new owned output root and explicitly rebound source/tool
pins; do not rerun them over the retained results. The exporter itself ran
with a 180-second wall limit, 120-second CPU cap and 2 GiB address-space cap.

## Next Gate

Diagnose and correct the exact lowered call-type mismatch, qualify the
correction, and retry both arms. Successful target-specific compilation must
then be followed by independently checked artifact and runtime contracts,
component output checks, lifecycle/reuse checks and complete model outputs.

The prior GPU device completion-retirement hold remains unresolved.
Runtime coordination has neither an owning-lane recovery record nor a
separately qualified TP2 set. No process-absence observation is treated as
recovery, and no GPU run is authorized by this CPU packet.

Separate real-source compilation, hardware execution and independent outputs
on both gfx942 and gfx950 remain required by the
[model sequence contract](../../docs/DUAL_TARGET_MEGAKERNEL_MODEL_SEQUENCE_V1.md).
Qwen3-8B remains BF16, single-request, target-only, 2,048 prompt tokens and
256 generated tokens. This packet supplies no token/s measurement.
