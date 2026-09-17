#!/usr/bin/env bash
set -euo pipefail

# Historical extraction-5 command. This creates an inert handoff, not authority.
# The recorded RUSTUP_HOME typo is preserved; pinned executables were on PATH.
root=/home/harmenon/ferric-gfx950-42
nightly="$root/toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu"
test ! -e "$root/evidence/decoder-layer-f32-v1-handoff-5.json"
env \
    PATH="$nightly/bin:/usr/local/bin:/usr/bin:/bin" \
    CARGO_HOME="$root/toolchain/cargo" \
    RUSTUP_HOME="$root/toolchain/rustup/toolchain" \
    CARGO_TARGET_DIR="$root/target-decoder" \
    CARGO_BUILD_JOBS=2 \
    LD_LIBRARY_PATH="$root/target-compiler/debug/deps:$nightly/lib" \
    RUSTC_WRAPPER="$root/target-compiler/debug/fe2o3-rustc-extract" \
    FE2O3_EXTRACT_CRATE_V1=ferric_gfx950_decoder_layer_f32_v1 \
    FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1="$root/evidence/decoder-layer-f32-v1-handoff-5.json" \
    RUSTFLAGS="-Zalways-encode-mir -Copt-level=3 -Ctarget-cpu=gfx950 -Ctarget-feature=-xnack,+wavefrontsize64,-wavefrontsize32" \
    "$nightly/bin/cargo" rustc --lib \
    --manifest-path "$root/ferric/device/gfx950-decoder-layer-f32-v1/Cargo.toml" \
    -Zbuild-std=core --target amdgcn-amd-amdhsa -- \
    -Zdump-mir=all -Zdump-mir-dir="$root/evidence/decoder-layer-mir-5" \
    2>&1 | tee "$root/evidence/decoder-layer-f32-v1-extraction-5.log"
