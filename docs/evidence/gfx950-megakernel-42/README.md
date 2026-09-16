# Initial Implementation Evidence

Collected on SSH host `mi350-2` on 2026-09-15 for
[Ferric #42](https://github.com/harsh-nod/ferric/issues/42).
This directory records CPU checks, the initial rejected device/compiler probes,
and a subsequent passing read-only device bind. It contains no GPU
dispatch, model numerical result, timing sample, formal proof, or release
qualification. The epic's M1-M7 completion gates remain open.

Source bases: Ferric `5d3d93d3e7f08645273d274bc35efbc79133e686` and fe2o3
`e3c359fb1bf39ec21c4239ac37ce59b7a3a51db9`. The integrated test logs include the
changes in this PR; the issue links the exact resulting commits. Checksums in
`SHA256SUMS` cover the published logs, not a production source/artifact closure.
The device probe has hardware identifiers redacted to follow fe2o3's
contribution policy; it is not a byte-identical raw capture. Original probe
output remains in the caller-owned evidence directory on the test host.

## Passing Checks

- `ferric-baseline-tests.log`: unmodified public-main workspace, 39 CPU tests.
- `ferric-integrated-tests.log`: the unchanged qualified runtime workspace, 39 tests.
- `ferric-clippy.log`: strict workspace/all-target Clippy.
- `planner-tests.log` / `planner-clippy.log`: isolated engineering tool, 20 tests
  and one compile-fail doctest; no Verus coverage or runtime release authority.
- `qualification-tests.log`: synthetic measurement-validator unit tests.
- `kfd-default-tests.log`: default KFD regression tests.
- `kfd-engineering-tests.log`: engineering-feature KFD regression tests.
- `uapi-tests.log`: DRM and KFD UAPI layout tests.
- `drm-oracle.log` / `kfd-event-oracle.log`: exact installed-header C oracles.
- `manifest-source-hashes.log`: new platform manifest versus installed sources.
- `xcp-*.log`: follow-up typed XCP0 correlation, default/engineering tests,
  Clippy, 25 source hashes, and successful read-only probe. The probe copy is
  explicitly redacted; it is not a GPU dispatch result.

Rust 1.97.1 was used for Ferric; fe2o3 used its pinned
`nightly-2026-04-03` with `rustc-dev`/`rust-src`. All builds and tests ran in
owned directories on the remote host, with bounded Cargo parallelism. No
system installation, foreign process termination, GPU reset, or authority
override was performed. The compiler build needed its Rust shared-library
directory in `LD_LIBRARY_PATH`; initial loader setup failures are not compiler
behavior findings.

## Initial Rejected Runtime Probe

`read-only-probe.log` preserves the rejection with device IDs redacted:

```sh
cargo run -p fe2o3-kfd --features engineering-gfx950 \
  --example kfd-gfx950-device-identity -- "$KFD_DEVICE_UNIQUE_ID"
```

Set `KFD_DEVICE_UNIQUE_ID` from local topology without publishing its value.
The KFD topology reports an XCD/XCP identity; the DRM parent exposes a
different board identity. The common topology
contract rejects the unequal IDs before acquiring device execution authority.
The exact driver sources explain the two domains. A matching independent DRM
XCD-ID query is not exposed, but the existing ABI supports the typed
correspondence implemented by the follow-up below; no new driver API is needed
for that contracted relationship.
See upstream `docs/gfx950-mi350-2-engineering-admission-v1.md` for source review.

## Passing Read-Only XCP0 Probe

The upstream engineering observation-v2 profile uses an explicit XCP0-to-parent
render contract, gated by the exact platform/feature, SPX/NPS1, geometry,
firmware, canonical PCI ancestry, exact render number/BDF, unique endpoint, and
retained XCP evidence. Both UID domains remain in snapshot/currentness equality.
Default gfx942 and original gfx950 behavior are unchanged.

`xcp-read-only-probe-redacted.log` records the successful `--all` example on
the host inventory, including two currentness checks and descriptor cleanup
(four before and after). GPU ID, unique ID, and render minor are redacted from
this published copy; original output is retained privately on the host. All
execution authorities in the probe are false. This is neither production
execution qualification nor proof of UID equivalence or all-reset detection.

Follow-up tests: 422 default KFD tests passed with one ignored; 498 engineering
tests passed with one ignored; strict Clippy passed for both configurations.
The 25 pinned installed-source hashes match. See the corresponding logs.

The follow-up source is fe2o3
`3018ec8193f13c829db4e037641664401f7eb180`, based on public main
`55d9bfe5746daf29dde35caa2c909c0d55393cda`. The successful probe executable
SHA-256 is
`5d6deead4e675d1a9470d86d0098b990a07227e89e021a9268543f0af66b7504`.

## Rejected Compiler Probe

`atomic-rmw-gfx950-extraction.log` records ordinary production Rust extraction
of the existing `atomic-rmw` fixture for gfx950. The compiler reports:

```text
semantic-to-ranked projection incomplete: a call terminator before exact callable memory-effect summaries are available
```

To reproduce from fe2o3 after building `rustc-codegen-fe2o3`, use the pinned
nightly, its shared-library directory, and the built backend's `debug/deps`
directory in `LD_LIBRARY_PATH`:

```sh
RUSTC_WORKSPACE_WRAPPER="$compiler/fe2o3-rustc-extract" \
FE2O3_EXTRACT_CRATE_V1=fe2o3_production_extraction_fixture \
FE2O3_EXTRACT_AMDGPU_LLVM_PATH_V1="$scratch/atomic-rmw-gfx950.ll" \
CARGO_TARGET_AMDGCN_AMD_AMDHSA_RUSTFLAGS='-Zalways-encode-mir -Copt-level=3 -Ctarget-cpu=gfx950 -Ctarget-feature=-xnack,+wavefrontsize64,-wavefrontsize32' \
cargo check --locked -Zbuild-std=core \
  -p fe2o3-production-extraction-fixture --features atomic-rmw \
  --target amdgcn-amd-amdhsa --target-dir "$scratch/target"
```

This exits nonzero and produces no accepted atomic HSACO. Standard atomic RMW
semantic import alone is not the required complete checked source lowering.
The scheduler needs exact callable effects, scoped atomic load/store/CAS,
allocation coherence, and synchronization/resource contracts before it can be
implemented and exercised through the production path. No handwritten IR or
alternative launcher was used to bypass this rejection.
