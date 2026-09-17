#!/usr/bin/env bash
set -euo pipefail
umask 077

if [[ $# != 2 ]]; then
  printf 'usage: %s WORK_ROOT FRESH_OUTPUT_DIRECTORY\n' "$0" >&2
  exit 2
fi
root=$(realpath -- "$1")
out=$(realpath -m -- "$2")
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
compiler="${FE2O3_COMPILER_BIN:-$root/evidence/compiler-atomic-slice-bin-v1}"
expected_compiler=3dfa5b3fdac1832bd7d8902e32f591d81300d1e3
nightly="$root/toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu"
unpack="${FE2O3_HANDOFF_UNPACKER:-$root/evidence/unpack-handoff}"
llvm="${ROCM_PATH:-/opt/rocm-7.2.0}/lib/llvm/bin"
providers="${ROCM_PATH:-/opt/rocm-7.2.0}/lib/llvm/lib/clang/22/lib/amdgcn/bitcode"

if [[ -n $(git -C "$repo" status --porcelain=v1 --untracked-files=all) ]]; then
  printf 'native evidence requires committed clean Ferric source\n' >&2
  exit 1
fi
# Use the previously frozen clean compiler build, not a concurrently edited
# compiler checkout or its live target directory. Recheck the whole manifest.
test "$(cat "$compiler/compiler-source-head.txt")" = "$expected_compiler"
test -f "$compiler/compiler-source-status.txt"
test ! -s "$compiler/compiler-source-status.txt"
sha256sum --check --strict "$compiler/provenance.sha256" >/dev/null
for executable in "$nightly/bin/cargo" "$nightly/bin/rustc" \
  "$compiler/fe2o3-rustc-extract" "$unpack" "$llvm/llvm-link" "$llvm/opt" \
  "$llvm/llc" "$llvm/ld.lld" "$llvm/llvm-readobj" "$llvm/llvm-objdump" "$llvm/llvm-dis"; do
  test -x "$executable"
done
test -f "$compiler/librustc_codegen_fe2o3.so"
provider_files=(
  "$providers/ocml.bc" "$providers/ockl.bc" "$providers/oclc_daz_opt_off.bc"
  "$providers/oclc_unsafe_math_off.bc" "$providers/oclc_finite_only_off.bc"
  "$providers/oclc_correctly_rounded_sqrt_on.bc" "$providers/oclc_wavefrontsize64_on.bc"
  "$providers/oclc_isa_version_950.bc" "$providers/oclc_abi_version_600.bc"
)
for provider in "${provider_files[@]}"; do test -f "$provider"; done
mkdir -- "$out"
producer="$repo/device/gfx950-qwen3-kproj-wave64-v1"
consumer="$repo/device/gfx950-qwen3-knorm-v1"
inputs=("$0" "$compiler/provenance.sha256" "$compiler/compiler-source-head.txt"
  "$compiler/compiler-source-status.txt" "$compiler/fe2o3-rustc-extract"
  "$compiler/librustc_codegen_fe2o3.so" "$nightly/bin/cargo" "$nightly/bin/rustc" "$unpack"
  "$llvm/llvm-link" "$llvm/opt" "$llvm/llc" "$llvm/ld.lld" "$llvm/llvm-readobj"
  "$llvm/llvm-objdump" "$llvm/llvm-dis" "${provider_files[@]}")
for package in "$producer" "$consumer"; do
  inputs+=("$package/Cargo.toml" "$package/Cargo.lock" "$package/src/lib.rs")
done
sha256sum "${inputs[@]}" > "$out/inputs.sha256"
git -C "$repo" rev-parse HEAD > "$out/ferric-source-head.txt"
flags='-Zalways-encode-mir -Zinline-mir=yes -Zmir-enable-passes=-JumpThreading -Copt-level=3 -Ctarget-cpu=gfx950 -Ctarget-feature=-xnack,+wavefrontsize64,-wavefrontsize32'
printf '%s\n' "$flags" > "$out/rustflags.txt"
build_stage() {
  local stage=$1 package=$2 crate=$3 artifact="$out/$1"
  env PATH="$nightly/bin:/usr/local/bin:/usr/bin:/bin" \
    CARGO_HOME="$root/toolchain/cargo" RUSTUP_HOME="$root/toolchain/rustup" \
    CARGO_TARGET_DIR="$out/cargo-target" CARGO_BUILD_JOBS=2 \
    LD_LIBRARY_PATH="$compiler:$nightly/lib" \
    RUSTC_WRAPPER="$compiler/fe2o3-rustc-extract" FE2O3_EXTRACT_CRATE_V1="$crate" \
    FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1="$artifact.handoff" RUSTFLAGS="$flags" \
    "$nightly/bin/cargo" rustc --locked --lib --manifest-path "$package/Cargo.toml" \
    -Zbuild-std=core --target amdgcn-amd-amdhsa 2>&1 | tee "$out/$stage-extraction.log"
  "$unpack" "$artifact.handoff" "$artifact.ll" | tee "$out/$stage-unpack.log"
  "$llvm/llvm-link" --only-needed "$artifact.ll" "${provider_files[@]}" -o "$artifact.linked.bc"
  "$llvm/opt" '-passes=default<O3>' "$artifact.linked.bc" -o "$artifact.optimized.bc"
  "$llvm/llvm-dis" "$artifact.optimized.bc" -o "$artifact.optimized.ll"
  "$llvm/llc" -O3 -mtriple=amdgcn-amd-amdhsa -mcpu=gfx950 \
    -mattr=-xnack,+wavefrontsize64,-wavefrontsize32 --amdhsa-code-object-version=6 \
    -filetype=obj "$artifact.optimized.bc" -o "$artifact.o"
  "$llvm/ld.lld" -shared --no-undefined -z noexecstack "$artifact.o" -o "$artifact.hsaco"
  "$llvm/llvm-readobj" --file-headers --notes --symbols "$artifact.hsaco" > "$artifact.metadata.txt"
  "$llvm/llvm-objdump" -d --mcpu=gfx950 "$artifact.hsaco" > "$artifact.isa.txt"
  sha256sum "$artifact.handoff" "$artifact.ll" "$artifact.linked.bc" "$artifact.optimized.bc" \
    "$artifact.optimized.ll" "$artifact.o" "$artifact.hsaco" "$artifact.metadata.txt" \
    "$artifact.isa.txt" > "$out/$stage-artifacts.sha256"
}
build_stage producer "$producer" ferric_gfx950_qwen3_kproj_wave64_v1
build_stage consumer "$consumer" ferric_gfx950_qwen3_knorm_v1
sha256sum --check --strict "$out/inputs.sha256" > "$out/inputs-rechecked.txt"
sha256sum --check --strict "$compiler/provenance.sha256" > "$out/compiler-rechecked.txt"
git -C "$repo" rev-parse HEAD > "$out/ferric-source-head-after.txt"
cmp "$out/ferric-source-head.txt" "$out/ferric-source-head-after.txt"
test -z "$(git -C "$repo" status --porcelain=v1 --untracked-files=all)"
printf 'Built two inert artifacts; no GPU execution or production authority is implied.\n'
