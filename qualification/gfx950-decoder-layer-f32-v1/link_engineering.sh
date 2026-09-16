#!/usr/bin/env bash
set -euo pipefail

# Engineering-only native linking of unchanged, upstream-decoded LLVM bytes.
root=/home/harmenon/ferric-gfx950-42
llvm=/opt/rocm-7.2.0/lib/llvm/bin
providers=/opt/rocm-7.2.0/lib/llvm/lib/clang/22/lib/amdgcn/bitcode
artifact="$root/evidence/decoder-layer-f32-v1"
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
input_files=(
    "$root/evidence/link-decoder-engineering.sh"
    "$root/ferric/device/gfx950-decoder-layer-f32-v1/src/lib.rs"
    "$root/ferric/device/gfx950-decoder-layer-f32-v1/Cargo.lock"
    "$root/evidence/decoder-layer-f32-v1-handoff-5.json"
    "$root/evidence/unpack-handoff.rs"
    "$root/evidence/unpack-handoff"
    "$artifact.ll"
    "$llvm/llvm-link"
    "$llvm/opt"
    "$llvm/llc"
    "$llvm/ld.lld"
    "$llvm/llvm-readobj"
    "${provider_files[@]}"
)
for suffix in linked.bc optimized.bc o hsaco metadata.txt
do
    test ! -e "$artifact.$suffix"
done
sha256sum "${input_files[@]}" > "$artifact.native-inputs.sha256"
"$llvm/llvm-link" --version
"$llvm/opt" --version
"$llvm/llc" --version
"$llvm/ld.lld" --version
set -x
"$llvm/llvm-link" --only-needed "$artifact.ll" "${provider_files[@]}" -o "$artifact.linked.bc"
"$llvm/opt" -passes='default<O3>' "$artifact.linked.bc" -o "$artifact.optimized.bc"
"$llvm/llc" -O3 -mtriple=amdgcn-amd-amdhsa -mcpu=gfx950 -mattr=-xnack,+wavefrontsize64,-wavefrontsize32 --amdhsa-code-object-version=6 -filetype=obj "$artifact.optimized.bc" -o "$artifact.o"
"$llvm/ld.lld" -shared --no-undefined -z noexecstack "$artifact.o" -o "$artifact.hsaco"
"$llvm/llvm-readobj" --file-headers --notes --symbols "$artifact.hsaco" > "$artifact.metadata.txt"
set +x
sha256sum -c "$artifact.native-inputs.sha256"
sha256sum "$artifact.ll" "$artifact.linked.bc" "$artifact.optimized.bc" "$artifact.o" "$artifact.hsaco" "$artifact.metadata.txt"
