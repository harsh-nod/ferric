#!/usr/bin/env bash
set -euo pipefail
umask 077

# Engineering observation only. This script neither admits nor launches code.
if [[ $# != 2 ]]; then
    printf 'usage: %s WORK_ROOT FRESH_OUTPUT_DIRECTORY\n' "$0" >&2
    exit 2
fi
root=$(realpath -- "$1")
out=$(realpath -m -- "$2")
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
package="$repo/device/gfx950-static-publication-v1"
# A diff plus a status listing cannot reconstruct untracked provider contents.
# Development diagnostics use separate scratch invocations; retained native
# evidence starts only from committed, clean compiler and fixture sources.
for checkout in "$root/fe2o3" "$repo"; do
    if [[ -n $(git -C "$checkout" status --porcelain=v1 --untracked-files=all) ]]; then
        printf 'native evidence requires clean committed source: %s\n' "$checkout" >&2
        exit 1
    fi
done
nightly="$root/toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu"
compiler=$(realpath -- "${FE2O3_COMPILER_BIN:?set FE2O3_COMPILER_BIN to a frozen clean-source V31 snapshot}")
compiler_head=$(git -C "$root/fe2o3" rev-parse HEAD)
test -f "$compiler/compiler-source-head.txt"
test -f "$compiler/compiler-source-status.txt"
test ! -s "$compiler/compiler-source-status.txt"
test "$(<"$compiler/compiler-source-head.txt")" = "$compiler_head"
test -f "$compiler/SHA256SUMS"
test -f "$compiler/loader-providers.sha256"
test -f "$compiler.provenance.sha256"
require_manifest_entry() {
    local manifest=$1 member=$2 expected
    expected=$(sha256sum -- "$member")
    if [[ $(grep -Fxc -- "$expected" "$manifest") != 1 ]]; then
        printf 'missing or duplicate selected-path manifest entry: %s\n' "$member" >&2
        return 1
    fi
}
for member in compiler-source-head.txt compiler-source-status.txt SHA256SUMS loader-providers.sha256; do
    require_manifest_entry "$compiler.provenance.sha256" "$compiler/$member"
done
for member in cargo-fe2o3 fe2o3-rustc-extract librustc_codegen_fe2o3.so; do
    require_manifest_entry "$compiler/SHA256SUMS" "$compiler/$member"
done
sha256sum --check --strict "$compiler.provenance.sha256"
sha256sum --check --strict "$compiler/SHA256SUMS"
sha256sum --check --strict "$compiler/loader-providers.sha256"
unpack="${FE2O3_HANDOFF_UNPACKER:-$root/evidence/unpack-handoff}"
rocm="${ROCM_PATH:-/opt/rocm-7.2.0}"
llvm="$rocm/lib/llvm/bin"
providers="$rocm/lib/llvm/lib/clang/22/lib/amdgcn/bitcode"
for executable in "$nightly/bin/cargo" "$nightly/bin/rustc" \
    "$compiler/fe2o3-rustc-extract" "$unpack" \
    "$llvm/llvm-link" "$llvm/opt" "$llvm/llc" "$llvm/ld.lld" "$llvm/llvm-readobj"; do
    test -x "$executable"
done
test -f "$compiler/librustc_codegen_fe2o3.so"
test -f "$package/Cargo.lock"
provider_files=(
    "$providers/ocml.bc"
    "$providers/ockl.bc"
    "$providers/oclc_daz_opt_off.bc"
    "$providers/oclc_unsafe_math_off.bc"
    "$providers/oclc_finite_only_off.bc"
    "$providers/oclc_correctly_rounded_sqrt_on.bc"
    "$providers/oclc_wavefrontsize64_on.bc"
    "$providers/oclc_isa_version_950.bc"
    "$providers/oclc_abi_version_600.bc"
)
for provider in "${provider_files[@]}"; do test -f "$provider"; done
mkdir -- "$out"
env -i HOME="$HOME" PATH="$nightly/bin:/usr/local/bin:/usr/bin:/bin" \
    CARGO_HOME="$root/toolchain/cargo" RUSTUP_HOME="$root/toolchain/rustup" \
    RUSTC="$nightly/bin/rustc" RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= \
    "$nightly/bin/cargo" metadata --locked --offline \
    --manifest-path "$package/Cargo.toml" --format-version=1 \
    > "$out/package-metadata.json"
# Also reject Cargo patch/config overrides of the two resolved providers.
jq -e --arg declared "git+https://github.com/harsh-nod/fe2o3?rev=$compiler_head" \
    --arg resolved "git+https://github.com/harsh-nod/fe2o3?rev=$compiler_head#$compiler_head" '
    .resolve.root as $root |
    [.packages[] | select(.id == $root)] as $roots |
    [.packages[] | select(.name == "fe2o3-device" or .name == "fe2o3-host")] as $providers |
    ($roots | length) == 1 and
    ($roots[0].dependencies | length) == 2 and
    ([$roots[0].dependencies[].name] | sort) == ["fe2o3-device", "fe2o3-host"] and
    all($roots[0].dependencies[]; .source == $declared and (.path // null) == null) and
    ([$providers[].name] | sort) == ["fe2o3-device", "fe2o3-host"] and
    all($providers[]; .source == $resolved)
' "$out/package-metadata.json" > "$out/dependency-pins-checked.txt"
artifact="$out/static-publication-v1"
inputs=(
    "$0" "$package/Cargo.toml" "$package/Cargo.lock" "$package/src/lib.rs"
    "$compiler/fe2o3-rustc-extract" "$compiler/librustc_codegen_fe2o3.so"
    "$compiler/compiler-source-head.txt" "$compiler/compiler-source-status.txt"
    "$compiler/SHA256SUMS" "$compiler/loader-providers.sha256" "$compiler.provenance.sha256"
    "$nightly/bin/cargo" "$nightly/bin/rustc" "$unpack"
    "$llvm/llvm-link" "$llvm/opt" "$llvm/llc" "$llvm/ld.lld" "$llvm/llvm-readobj"
    "${provider_files[@]}"
)
sha256sum "${inputs[@]}" > "$out/inputs.sha256"
git -C "$root/fe2o3" rev-parse HEAD > "$out/compiler-source-head.txt"
test "$(<"$out/compiler-source-head.txt")" = "$compiler_head"
git -C "$root/fe2o3" diff --binary HEAD > "$out/compiler-source-diff.patch"
git -C "$root/fe2o3" status --porcelain=v1 > "$out/compiler-source-status.txt"
git -C "$repo" rev-parse HEAD > "$out/ferric-source-head.txt"
git -C "$repo" status --porcelain=v1 --untracked-files=all > "$out/ferric-source-status.txt"
test ! -s "$out/compiler-source-status.txt"
test ! -s "$out/ferric-source-status.txt"
flags='-Zalways-encode-mir -Zinline-mir=yes -Zmir-enable-passes=-JumpThreading -Copt-level=3 -Ctarget-cpu=gfx950 -Ctarget-feature=-xnack,+wavefrontsize64,-wavefrontsize32'
printf '%s\n' "$flags" > "$out/rustflags.txt"
env -i HOME="$HOME" PATH="$nightly/bin:/usr/local/bin:/usr/bin:/bin" \
    CARGO_HOME="$root/toolchain/cargo" RUSTUP_HOME="$root/toolchain/rustup" \
    CARGO_TARGET_DIR="$out/cargo-target" CARGO_BUILD_JOBS=2 \
    LD_LIBRARY_PATH="$compiler:$nightly/lib" \
    RUSTC="$nightly/bin/rustc" RUSTC_WORKSPACE_WRAPPER= \
    RUSTC_WRAPPER="$compiler/fe2o3-rustc-extract" \
    FE2O3_EXTRACT_CRATE_V1=ferric_gfx950_static_publication_v1 \
    FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1="$artifact.handoff" \
    RUSTFLAGS="$flags" \
    "$nightly/bin/cargo" rustc --locked --offline --lib --manifest-path "$package/Cargo.toml" \
    -Zbuild-std=core --target amdgcn-amd-amdhsa \
    2>&1 | tee "$out/extraction.log"
"$unpack" "$artifact.handoff" "$artifact.ll" | tee "$out/unpack.log"
# Decode the unchanged checked compiler handoff; never patch its LLVM output.
"$llvm/llvm-link" --only-needed "$artifact.ll" "${provider_files[@]}" -o "$artifact.linked.bc"
"$llvm/opt" '-passes=default<O3>' "$artifact.linked.bc" -o "$artifact.optimized.bc"
"$llvm/llc" -O3 -mtriple=amdgcn-amd-amdhsa -mcpu=gfx950 \
    -mattr=-xnack,+wavefrontsize64,-wavefrontsize32 --amdhsa-code-object-version=6 \
    -filetype=obj "$artifact.optimized.bc" -o "$artifact.o"
"$llvm/ld.lld" -shared --no-undefined -z noexecstack "$artifact.o" -o "$artifact.hsaco"
"$llvm/llvm-readobj" --file-headers --notes --symbols "$artifact.hsaco" > "$artifact.metadata.txt"
sha256sum -c "$out/inputs.sha256" > "$out/inputs-rechecked.txt"
sha256sum --check --strict "$compiler.provenance.sha256" > "$out/compiler-provenance-postcheck.txt"
sha256sum --check --strict "$compiler/SHA256SUMS" > "$out/compiler-binaries-postcheck.txt"
sha256sum --check --strict "$compiler/loader-providers.sha256" > "$out/compiler-loader-postcheck.txt"
git -C "$root/fe2o3" diff --binary HEAD > "$out/compiler-source-diff-after.patch"
git -C "$root/fe2o3" status --porcelain=v1 > "$out/compiler-source-status-after.txt"
git -C "$root/fe2o3" rev-parse HEAD > "$out/compiler-source-head-after.txt"
cmp "$out/compiler-source-diff.patch" "$out/compiler-source-diff-after.patch"
cmp "$out/compiler-source-status.txt" "$out/compiler-source-status-after.txt"
cmp "$out/compiler-source-head.txt" "$out/compiler-source-head-after.txt"
git -C "$repo" rev-parse HEAD > "$out/ferric-source-head-after.txt"
git -C "$repo" status --porcelain=v1 --untracked-files=all > "$out/ferric-source-status-after.txt"
cmp "$out/ferric-source-head.txt" "$out/ferric-source-head-after.txt"
cmp "$out/ferric-source-status.txt" "$out/ferric-source-status-after.txt"
sha256sum "$artifact.handoff" "$artifact.ll" "$artifact.linked.bc" \
    "$artifact.optimized.bc" "$artifact.o" "$artifact.hsaco" \
    "$artifact.metadata.txt" > "$out/artifacts.sha256"
printf 'Built inert engineering artifact: %s\n' "$artifact.hsaco"
printf 'GPU execution, numerical correctness, and production authority are not implied.\n'
