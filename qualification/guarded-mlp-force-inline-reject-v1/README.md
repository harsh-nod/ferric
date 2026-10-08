# Canonical Helper Diagnosis and Device Inlining

Engineering checkpoint for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42),
2026-10-08 UTC. All authoring, compilation and test execution occurred on
`ssh mi350`. This is a qualification candidate, not a promoted runtime or a
successful end-to-end megakernel. Neither compiler retry produced HSACO.
There is no new GPU execution, numerical result, overlap graph or speedup.
The BF16 Qwen3-8B single-request 2048/256, 700 tokens/s target and M0-M7 remain open.

## What Changed

The preceding [diagnostic capture checkpoint](../guarded-mlp-semantic-capture-v1/README.md)
retained the actual canonical MIR and source map, but could not yet identify
the rejected helper by its complete typed identity. This checkpoint adds:

- A [bounded canonical reader](source/reader/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs)
  using the existing `AdmittedInertSemanticMirV1` decoder and logical argument
  mapping. It authenticates input hashes, rejects malformed or noncanonical
  encodings and never opens display paths supplied by the source map.
- An [exact external source join](source-join.json) from both V8 captures to
  `MlpNormTileV2::reject`, bytes 29396..29457, lines 828:5 through 830:6 of the
  pinned provider. Its complete helper identity is
  `00b15eaa5a80fc7f25e19c1c333fcde91f768f51097ae1c2c4f27ced27c66af0`.
- One AMDGPU-only `rustc_force_inline` attribute on that method. The method
  still only sets `*self.valid = false`; no signature, body, guard or scheduler
  ordering changed. See the [device source](source/correction/crates/fe2o3-device/src/finite_join/wave_mlp_tiles_v2.rs)
  and [exact correction patch](correction.patch).
- A [64-lane rejection/retirement regression](source/correction/crates/fe2o3-device/src/finite_join/wave_mlp_tiles_v2_tests.rs)
  covering repeated rejection, refused subsequent writes, unchanged payloads,
  `INCOMPLETE_WRITES`, and no fabricated arrivals or DONE publication.
- The matching strict source-closure pin and old-closure refusal regression in
  [trusted device items](source/correction/crates/rustc-codegen-fe2o3/src/trusted_device_items.rs).

The compiler authenticates all 53 provider source files, including unit tests.
Their reviewed closure changes from
`037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675`
to `d6ac31898b9e4d594d3aa36d6c5ced378059a4ffff54881463f57c10eba0563c`.
This is an exact source-version update, not an alternate-identity allowlist.
The old closure is refused. Helper ABI, aliasing, ownership, source-authentication
and lowering predicates remain unchanged. The existing default-off diagnostic
and normalization options retain their previous behavior.

## CPU Qualification

| Qualification | Passed | Failed | Ignored | Clean phases | Elapsed seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| Canonical reader | 7 | 0 | 0 | 9 | 23.403153 |
| Correction: compiler library | 1,329 | 0 | 24 | included below | included below |
| Correction: device library | 357 | 0 | 0 | 11 total | 239.283054 total |

The compiler's 24 ignored cases are unchanged fixture-dependent tests, not
passing tests. The complete compiled compiler inventory is 1,353 names;
all default compiler and device unit tests ran without filtering. All 15
MLP tile tests ran, including the new regression. The 11 existing capture
tests and 14 helper controls are included in the 1,329, not additional passes.
This is not a whole-workspace or integration-test claim.

Original receipts and named outcomes:
[reader receipt](cpu/reader/complete.json), [reader tests](cpu/reader/reader-tests.stdout),
[compiler/device receipt](cpu/correction/complete.json),
[compiler tests](cpu/correction/compiler-tests.stdout),
[device tests](cpu/correction/device-tests.stdout).
All phases exited naturally, were reaped and left no owned process group.
Source, tool, dependency and rust-src postchecks passed. The run used CPU8/9,
nice10, Cargo jobs2 and hidden GPU devices. Existing resource limits and
deadlines were retained. A preliminary formatting check refused Rustfmt2021
changes to old test bodies; those bytes are retained in the archive. The
qualified source uses Rustfmt2024 with all old test bodies unchanged.

## Real Compiler Retries

Both V9 arms use the qualified backend `e5f3f31f...`, unchanged qualified
frontend `26bf333c...` and exact updated provider. The callback remains
`inline(never)`; the two fixtures differ only in `run_fixed_rounds` versus
`run`. The named normalization profile remains
`optimized-inline-hint-16384-v1`. No arithmetic or launch geometry changed.

| Arm | Natural exit | Compiler leaf seconds | Captured MIR bytes | HSACO |
| --- | ---: | ---: | ---: | --- |
| Fixed rounds | 1 | 118.836184 | 921,836 | none |
| Early STOP | 1 | 119.138156 | 922,915 | none |

The diagnostic moved from the mutable `MlpNormTileV2::reject` helper to a
shared-reference `MlpDownTileV2::input` helper. This is observed compiler
progress, not compile acceptance. Both attempts were naturally reaped with
unchanged inputs, empty output products and no timeout or forced cleanup.
See [fixed result](pair/fixed/result.json), [fixed stderr](pair/fixed/compile.stderr),
[early result](pair/early/result.json), and [early stderr](pair/early/compile.stderr).

The same qualified reader then decoded both new captures. The
[fixed source join](next-helper/fixed.json) and
[early source join](next-helper/early.json) bind complete helper identity
`20a4d4a655b95130262fa05b5f384876f36f6d09945ec40dea003e461afe0a2c`
to `MlpDownTileV2::input`, bytes 35027..35237, lines 999:5 through 1004:6.
Its argument is a shared immutable reference to an ordinary aggregate, which
the current helper ABI correctly refuses. These two reader executions are
diagnostic/source joins, not additional unit tests or GPU tests.

The next source correction is to apply the existing AMDGPU-only force-inline
convention consistently to the remaining 19 small methods in the five MLP
tile-view impls. Only Down input is the newly observed refusal; the other
methods are structurally similar ABI risks identified by source review.
Their signatures, existing guard asymmetries and bodies must remain exact.
That broader correction has not yet been authored or qualified in this packet.
After a successful checked compile, native numerical validation and the
sustained workload benchmark are still required.

## Reproducible Evidence

[Manifest](manifest.json), [reader patch](reader.patch), [correction patch](correction.patch),
[original evidence archive](originals.tar.gz), and [exporter source](export.py).
The archive contains 188 members: 187 original files and `originals.json`,
which records original absolute paths, byte lengths and SHA-256 identities.
It retains CPU commands/results, source preimages/postimages, compiler
diagnostics, fixtures, canonical captures, both generations of typed source
joins, and the refused preliminary formatting body. It does not contain
compiler ELF binaries or unrelated shared-host files.

Archive: 10,473,228 bytes,
`e094773913ea093f5c1987bb914b4dfafedcdbfa73b328dd14ddd3f795b7b56d`.
Expanded members total 70,211,801 bytes.
Reader receipt: `5999eb6b23579e9be205b7627cc97f98eb58b471c552ff355239285c84a13877`.
Correction receipt: `c9bc292d82f5018e04692e7ed7b866f7ac578d35863f210313bc6e5efd176294`.

The retained controllers deliberately require fresh, exact namespaces and
pinned parent artifacts. Reproduction needs the original toolchain and
dependency inputs recorded there; an edited or relocated run needs its own
reviewed manifest. Do not overwrite prior evidence or treat these diagnostic
receipts as production execution authority.
