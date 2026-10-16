# Ordinary Host Device Binding Enrollment (2026-10-16)

## Scope

This enrolls four current **non-authoritative host fallback namespaces**, not
native artifacts or proof identities. Package manifests, lockfiles, compiler
pins, kernel bodies and compilation flags are unchanged. The other four CI
family wrappers are parser-corpus coverage only, not new canonical enrollments.

The observed profile is a host library check using
`nightly-2026-04-03`, target `x86_64-unknown-linux-gnu`, edition 2024,
`embed-bitcode=no`, `debuginfo=2`. The aggregate has default `gfx942` feature
configuration; the three family wrappers have no feature cfg. This is not a
GPU-target build and does not establish parity for other test, release,
non-default-feature or `gfx950` configurations.

## Independent Provenance

The ordinary binding-only CLI is fe2o3 commit
`6f6a67bb2f6de70a1c5533bbcde1b09449c37359`, source tree
`5674a7078440e445010d3b5d58c8bc4a34b6fe94`. Its optimized release executable
SHA-256 is
`35e3ca3b503e61add7f5c23220c4c9c15682f394b8dae5787297ec6dd8114393`.
It was installed from the exact clean local source checkout with locked
dependencies, rather than CI's Git transport; no replacement compiler or
algorithm was used.

Two provenance paths agree: actual canonical verbose invocations and their
retained metadata artifacts, and a separate standard-library implementation
of the reviewed source recipe using the same exact manifest bytes and argv.
The recipe is defined in that compiler revision's:

- `crates/fe2o3-rustc-invocation/src/portable_metadata_v1.rs:294`
- `crates/cargo-fe2o3/src/binding_check_wrapper.rs:216`
- `crates/reserved-fe2o3-symbols/src/lib.rs:783`

Portable metadata hashes package name/version, manifest SHA-256, crate name,
sorted/deduplicated cfg and crate types, and ordered selected edition, target
and codegen fields. The crate binding then hashes the crate name and one
portable metadata token. Raw Cargo metadata remains required but is not the
portable identity input. Rust body text and build-script bytes are not hashed
by this host namespace recipe. Equal bindings therefore do not establish
source equivalence, semantic correctness, or native artifact identity.

| Module | Manifest SHA-256 |
| --- | --- |
| Aggregate | `15ec25d54250b897afa65e4fee9c84f156f8f0790ceb540a8c0e38018bbc8857` |
| Prefill | `63edbf873c8c3df36f7c1b1a8ece9d994b9bf59bafe6f7e5e7bf86bf3caa97df` |
| Paged decode | `5520a4cd860add39e92fd97c552d9a4d49987109e5552a41bcb430994fa56e21` |
| RoPE/KV | `4f344287ed7ee6b0a3085793e8da2d479727f36d4a03775df864ce9fc8ac98c5` |

| Module | Enrolled Ordinary Host Binding |
| --- | --- |
| Aggregate | `423280f43dabf0a3eb2162b5b0779fa050960ec0b57cd7d6813ea967ee8273b4` |
| Prefill | `fd68d93d9da0c6de96bfa13dbe58b79bbd93fbe0c3b5a13bc3553600bfcae872` |
| Paged decode | `53f449de98c3bcc8dd84748dbe59306f3108b8761e8ae6e41c68c611bf90c45a` |
| RoPE/KV | `af7701f7bcd0493b21be9948d5978ee2efae438ba1d4c6ef06c4f1921857cc3f` |

The aggregate's original canonical receipt (3937) and three family receipts
(3974) retain exact argv, runtime/tool hashes, full transcripts and `.rmeta`
hashes. Their machine-readable inventory SHA-256 values are respectively
`c1a62cb9fff4f955a988602706b6db02ace4a30ee00f16ea761d4d384e519164`
and `767d3b940028a81633a2f1729f8d3a456dd5b0163826b3bc16ad1a3715e0d18b`.
Independent recomputation audit 3985 has SHA-256
`fcb97e521bf9617107ed1d465eb1b1447d1445fc9aee1a0aec18eb6a9220eef1`.
These are retained operator receipts, not publicly hosted downloadable
artifacts; the commands below support reproduction.

## Reproduction

Use the verified ordinary CLI and the real pinned runtime library directory.
Do not set a compiler override for managed commands.

```bash
export PATH="$FE2O3_PREFIX/bin:$HOME/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin:/usr/bin:/bin"
export LD_LIBRARY_PATH="$HOME/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/lib"
unset RUSTC RUSTC_BOOTSTRAP RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
export CARGO_BUILD_JOBS=2 CARGO_TERM_COLOR=never
set -o pipefail
cd device/qwen3-all-kernels-v1  # repeat for prefill, paged-decode and rope-kv
"$FE2O3_PREFIX/bin/cargo-fe2o3" fe2o3 check --locked --lib -vv 2>&1 |
  tee "$EVIDENCE/binding.log"
python3 -I -B ../../proofs/check-device-binding.py . "$EVIDENCE/binding.log"
"$FE2O3_PREFIX/bin/cargo-fe2o3" fe2o3 check --locked --all-targets
"$FE2O3_PREFIX/bin/cargo-fe2o3" fe2o3 test --locked --all-targets
```

Use a fresh admitted private target directory for each canonical transcript;
a cached no-op transcript is not evidence of one canonical invocation.
The checker still requires exactly one canonical invocation, genuine metadata
embedding and strict compiler-derived versus explicit-fallback equality.
Actual keyed and legacy single-placeholder split emitters are recognized only
through complete closed script grammars, with all eight CI build scripts in
the positive corpus and hostile expansion/format/guard controls retained.

## Historical and Protected Boundaries

The old aggregate `383b85...` and family fallback IDs remain historical.
Original failed checks, receipts, source snapshots and their explanations
must not be overwritten. In particular,
`experiments/native-gate-up-a009/manifest.json`, native source-roster hashes,
source-pin adapter prebindings, proof registries and signed publication/runtime
records are not refreshed by this enrollment.

Aggregate `build.rs` belongs to the protected production source roster.
Changing its fallback changes that source closure even though the portable
host namespace omits those bytes. An old native/proof receipt cannot qualify
the new source merely because a host check passes. Future native use requires
separate current receipt-bound compilation, publication authentication,
runtime custody and appropriate validation.

This enrollment grants no protected compiler, verifier, native/GPU execution,
numerical proof, model quality, performance, Qwen or M1 authority.
Independent review and integration acceptance remain separate requirements.
