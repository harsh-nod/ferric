#!/usr/bin/env bash
set -euo pipefail
umask 077

# Source extraction and native linking only. This never opens GPU devices.
if [[ $# != 2 ]]; then
  printf 'usage: %s WORK_ROOT FRESH_OUTPUT_DIRECTORY\n' "$0" >&2
  exit 2
fi
root=$(realpath -- "$1")
out=$(realpath -m -- "$2")
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
package="$repo/device/gfx950-qwen3-kproj-v1"
nightly="$root/toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu"
compiler="${FE2O3_COMPILER_BIN:-$root/evidence/compiler-scalar-pipeline-bin}"
unpack="${FE2O3_HANDOFF_UNPACKER:-$root/evidence/unpack-handoff}"
rocm="${ROCM_PATH:-/opt/rocm-7.2.0}"
llvm="$rocm/lib/llvm/bin"
providers="$rocm/lib/llvm/lib/clang/22/lib/amdgcn/bitcode"
for executable in "$nightly/bin/cargo" "$nightly/bin/rustc" \
  "$compiler/fe2o3-rustc-extract" "$unpack" "$llvm/llvm-link" "$llvm/opt" \
  "$llvm/llc" "$llvm/ld.lld" "$llvm/llvm-readobj"; do
  test -x "$executable"
done
test -f "$compiler/librustc_codegen_fe2o3.so"
test -f "$package/Cargo.lock"
provider_files=(
  "$providers/ocml.bc" "$providers/ockl.bc" "$providers/oclc_daz_opt_off.bc"
  "$providers/oclc_unsafe_math_off.bc" "$providers/oclc_finite_only_off.bc"
  "$providers/oclc_correctly_rounded_sqrt_on.bc" "$providers/oclc_wavefrontsize64_on.bc"
  "$providers/oclc_isa_version_950.bc" "$providers/oclc_abi_version_600.bc"
)
for provider in "${provider_files[@]}"; do test -f "$provider"; done
mkdir -- "$out"
artifact="$out/qwen3-kproj-v1"
inputs=(
  "$0" "$package/Cargo.toml" "$package/Cargo.lock" "$package/src/lib.rs"
  "$compiler/fe2o3-rustc-extract" "$compiler/librustc_codegen_fe2o3.so"
  "$nightly/bin/cargo" "$nightly/bin/rustc" "$unpack"
  "$llvm/llvm-link" "$llvm/opt" "$llvm/llc" "$llvm/ld.lld" "$llvm/llvm-readobj"
  "${provider_files[@]}"
)
sha256sum "${inputs[@]}" > "$out/inputs.sha256"
flags='-Zalways-encode-mir -Zinline-mir=yes -Zmir-enable-passes=-JumpThreading -Copt-level=3 -Ctarget-cpu=gfx950 -Ctarget-feature=-xnack,+wavefrontsize64,-wavefrontsize32'
printf '%s\n' "$flags" > "$out/rustflags.txt"
env PATH="$nightly/bin:/usr/local/bin:/usr/bin:/bin" \
  CARGO_HOME="$root/toolchain/cargo" RUSTUP_HOME="$root/toolchain/rustup" \
  CARGO_TARGET_DIR="$out/cargo-target" CARGO_BUILD_JOBS=2 \
  LD_LIBRARY_PATH="$compiler:$nightly/lib" \
  RUSTC_WRAPPER="$compiler/fe2o3-rustc-extract" \
  FE2O3_EXTRACT_CRATE_V1=ferric_gfx950_qwen3_kproj_v1 \
  FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1="$artifact.handoff" \
  RUSTFLAGS="$flags" \
  "$nightly/bin/cargo" rustc --locked --lib --manifest-path "$package/Cargo.toml" \
  -Zbuild-std=core --target amdgcn-amd-amdhsa 2>&1 | tee "$out/extraction.log"
"$unpack" "$artifact.handoff" "$artifact.ll" | tee "$out/unpack.log"
# Decode and compile the unchanged compiler handoff, never handwritten LLVM.
"$llvm/llvm-link" --only-needed "$artifact.ll" "${provider_files[@]}" -o "$artifact.linked.bc"
"$llvm/opt" '-passes=default<O3>' "$artifact.linked.bc" -o "$artifact.optimized.bc"
"$llvm/llc" -O3 -mtriple=amdgcn-amd-amdhsa -mcpu=gfx950 \
  -mattr=-xnack,+wavefrontsize64,-wavefrontsize32 --amdhsa-code-object-version=6 \
  -filetype=obj "$artifact.optimized.bc" -o "$artifact.o"
"$llvm/ld.lld" -shared --no-undefined -z noexecstack "$artifact.o" -o "$artifact.hsaco"
"$llvm/llvm-readobj" --file-headers --notes --symbols "$artifact.hsaco" > "$artifact.metadata.txt"
sha256sum -c "$out/inputs.sha256" > "$out/inputs-rechecked.txt"
sha256sum "$artifact.handoff" "$artifact.ll" "$artifact.linked.bc" \
  "$artifact.optimized.bc" "$artifact.o" "$artifact.hsaco" "$artifact.metadata.txt" \
  > "$out/artifacts.sha256"
printf 'Built inert actual-weight GEMV baseline artifact: %s\n' "$artifact.hsaco"
printf 'No GPU execution, numerical result, optimized performance or production authority is implied.\n'
