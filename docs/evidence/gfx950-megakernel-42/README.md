# Initial Implementation Evidence

Collected on SSH host `mi350-2` on 2026-09-15 for
[Ferric #42](https://github.com/harsh-nod/ferric/issues/42).
This directory records CPU checks and two rejected probes. It contains no GPU
dispatch, model numerical result, timing sample, formal proof, or release
qualification. The epic's M1-M7 completion gates remain open.

Source bases: Ferric `5d3d93d3e7f08645273d274bc35efbc79133e686` and fe2o3
`e3c359fb1bf39ec21c4239ac37ce59b7a3a51db9`. The integrated test logs include the
changes in this PR; the issue links the exact resulting commits. Checksums in
`SHA256SUMS` cover these raw logs, not a production source/artifact closure.

## Passing Checks

- `ferric-baseline-tests.log`: unmodified public-main workspace, 39 CPU tests.
- `ferric-integrated-tests.log`: workspace plus the declared planner.
- `ferric-clippy.log`: strict workspace/all-target Clippy.
- `qualification-tests.log`: synthetic measurement-validator unit tests.
- `kfd-default-tests.log`: default KFD regression tests.
- `kfd-engineering-tests.log`: engineering-feature KFD regression tests.
- `uapi-tests.log`: DRM and KFD UAPI layout tests.
- `drm-oracle.log` / `kfd-event-oracle.log`: exact installed-header C oracles.
- `manifest-source-hashes.log`: new platform manifest versus installed sources.

Rust 1.97.1 was used for Ferric; fe2o3 used its pinned
`nightly-2026-04-03` with `rustc-dev`/`rust-src`. All builds and tests ran in
owned directories on the remote host, with bounded Cargo parallelism. No
system installation, foreign process termination, GPU reset, or authority
override was performed. The compiler build needed its Rust shared-library
directory in `LD_LIBRARY_PATH`; initial loader setup failures are not compiler
behavior findings.

## Rejected Runtime Probe

`read-only-probe.log` is the actual output of:

```sh
cargo run -p fe2o3-kfd --features engineering-gfx950 \
  --example kfd-gfx950-device-identity -- 10294934887855545115
```

The KFD topology reports XCD/XCP identity `0x8edef4bc5067db1b`; the DRM
parent exposes board identity `0xd0121ff00d36192e`. The common topology
contract rejects the unequal IDs before acquiring device execution authority.
The exact driver sources explain the two domains, but the reviewed public ABI
does not provide a corresponding independent DRM XCD-ID query. A separate
XCP-aware correlation contract is needed; this patch does not waive the check.
See upstream `docs/gfx950-mi350-2-engineering-admission-v1.md` for source review.

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
