#!/bin/sh
set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH='' cd -- "$script_dir/.." && pwd)

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

for tool in awk cargo cat chmod cmp cp dirname grep mkdir mktemp python3 rm rustc sed sha256sum sort timeout tr uname; do
    command -v "$tool" >/dev/null 2>&1 || fail "$tool is required by the qualification gate"
done

verus_root=${VERUS_ROOT:-}
[ -n "$verus_root" ] || fail 'VERUS_ROOT must name the extracted pinned Verus release'
verus_root=$(CDPATH='' cd -- "$verus_root" 2>/dev/null && pwd) || fail 'VERUS_ROOT is unavailable'
for member in cargo-verus verus rust_verify z3 version.txt; do
    [ -f "$verus_root/$member" ] || fail "Verus release is missing $member"
done
for member in cargo-verus verus rust_verify z3; do
    [ -x "$verus_root/$member" ] || fail "Verus release member is not executable: $member"
done

expected_version=$(sed -n '1p' "$script_dir/verus/VERUS_VERSION")
actual_version=$(VERUS_Z3_PATH="$verus_root/z3" "$verus_root/verus" --version | sed -n 's/^  Version: //p')
[ "$actual_version" = "$expected_version" ] || fail 'Verus version does not match the admitted release'
expected_verus=$(sed -n '1p' "$script_dir/verus/VERUS_SHA256")
actual_verus=$(sha256sum "$verus_root/verus" | awk '{ print $1 }')
[ "$actual_verus" = "$expected_verus" ] || fail 'Verus launcher digest drifted'

timeout_seconds=${FERRIC_PROOF_TIMEOUT_SECONDS:-600}
case "$timeout_seconds" in
    ''|*[!0-9]*) printf 'FAIL: invalid proof timeout\n' >&2; exit 2 ;;
esac
[ "$timeout_seconds" -ge 1 ] && [ "$timeout_seconds" -le 1200 ] || {
    printf 'FAIL: proof timeout must be 1 through 1200\n' >&2
    exit 2
}

rust_version=$(rustc --version)
case "$rust_version" in
    'rustc 1.97.1 '*) ;;
    *) fail "Rust 1.97.1 is required, found $rust_version" ;;
esac
cargo_version=$(cargo --version)
case "$cargo_version" in
    'cargo 1.97.1 '*) ;;
    *) fail "Cargo 1.97.1 is required, found $cargo_version" ;;
esac

mkdir -p "$repo/target"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-qualification.XXXXXX")
proof_target=$(mktemp -d "${TMPDIR:-/tmp}/ferric-proof-target.XXXXXX")
source_gate_target=$(mktemp -d "${TMPDIR:-/tmp}/ferric-source-gate-target.XXXXXX")
if [ -n "${FERRIC_RECEIPT_DIR:-}" ]; then
    receipt_dir=$FERRIC_RECEIPT_DIR
    [ ! -e "$receipt_dir" ] || fail "receipt destination already exists: $receipt_dir"
    mkdir -p "$receipt_dir"
else
    receipt_dir=$(mktemp -d "$repo/target/ferric-receipt.XXXXXX")
fi
trap 'chmod -R u+w "$scratch" "$proof_target" "$source_gate_target" 2>/dev/null || true; rm -rf "$scratch" "$proof_target" "$source_gate_target"' EXIT HUP INT TERM

# Exclude ambient Cargo wrappers, flags, and configuration from the artifact.
unset RUSTC RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_BUILD_TARGET CARGO_TARGET_DIR
export CARGO_HOME="$scratch/cargo-home"
export CARGO_TERM_COLOR=never
export GIT_CONFIG_NOSYSTEM=1
export GIT_CONFIG_GLOBAL=/dev/null
mkdir -p "$CARGO_HOME"

closure_log="$scratch/verus-closure.transcript"
if "$script_dir/verify-verus-closure.sh" \
    "$verus_root" "$script_dir/verus/VERUS_CLOSURE_MANIFEST" >"$closure_log" 2>&1; then
    cat "$closure_log"
else
    cat "$closure_log" >&2
    fail 'pinned Verus release closure authentication failed'
fi

authenticated_verus="$scratch/verus-release"
cp -a "$verus_root" "$authenticated_verus"
"$script_dir/verify-verus-closure.sh" \
    "$authenticated_verus" "$script_dir/verus/VERUS_CLOSURE_MANIFEST" >/dev/null
chmod -R a-w "$authenticated_verus"
verus_root=$authenticated_verus

source_snapshot="$scratch/source"
mkdir -p "$source_snapshot"
cp -a "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/rust-toolchain.toml" "$source_snapshot/"
cp -a "$repo/crates" "$repo/proofs" "$source_snapshot/"
cp -a "$repo/docs" "$source_snapshot/"
mkdir -p "$source_snapshot/.github/workflows"
cp -a "$repo/.github/workflows/verus.yml" "$source_snapshot/.github/workflows/"
live_source_before="$scratch/live-source-before.records"
snapshot_source="$scratch/snapshot-source.records"
python3 -I "$script_dir/source-closure.py" "$repo" "$live_source_before"
python3 -I "$script_dir/source-closure.py" "$source_snapshot" "$snapshot_source"
cmp -s "$live_source_before" "$snapshot_source" || fail 'source changed while creating the qualification snapshot'
chmod -R a-w "$source_snapshot"
qualified_repo=$source_snapshot
qualified_scripts="$qualified_repo/proofs"

python3 -I "$qualified_scripts/check-lock.py" \
    "$qualified_repo/Cargo.lock" "$qualified_scripts/source-gate/Cargo.lock"
(
    cd "$qualified_repo"
    cargo build --manifest-path proofs/source-gate/Cargo.toml --locked --release \
        --target-dir "$source_gate_target"
)
source_gate="$source_gate_target/release/ferric-source-gate"
[ -x "$source_gate" ] || fail 'compiler-rooted source gate was not built'
source_gate_digest=$(sha256sum "$source_gate" | awk '{ print $1 }')
source_gate_metadata="$scratch/source-gate-cargo-metadata.json"
(
    cd "$qualified_repo"
    CARGO_TARGET_DIR="$source_gate_target" \
        cargo metadata --manifest-path proofs/source-gate/Cargo.toml --locked --format-version 1
) >"$source_gate_metadata"
chmod -R a-w "$source_gate_target"
generated_source_gate_tcb="$scratch/SOURCE_GATE_DEPENDENCY_TCB.generated"
"$source_gate" --dependency-tcb "$source_gate_metadata" "$generated_source_gate_tcb"
cmp -s "$qualified_scripts/source-gate/DEPENDENCY_TCB" "$generated_source_gate_tcb" || \
    fail 'source-gate dependency or build-script TCB drifted'
metadata="$scratch/cargo-metadata.json"
(
    cd "$qualified_repo"
    CARGO_TARGET_DIR="$proof_target" cargo metadata --locked --no-deps --format-version 1
) >"$metadata"
generated_coverage="$scratch/VERIFIED_MODULES.generated"
"$source_gate" --generate "$qualified_repo" "$metadata" "$generated_coverage"
cmp -s "$qualified_scripts/VERIFIED_MODULES" "$generated_coverage" || {
    printf 'FAIL: proof coverage manifest is stale; regenerate with:\n' >&2
    printf '  proofs/source-gate/target/release/ferric-source-gate --generate . METADATA proofs/VERIFIED_MODULES\n' >&2
    exit 1
}
"$source_gate" "$qualified_repo" "$qualified_scripts/VERIFIED_MODULES" "$metadata"

verified_sources="$scratch/verified-sources"
sed -n 's/^verified=[^|]*|\([^|]*\)|.*/\1/p' "$qualified_scripts/VERIFIED_MODULES" | LC_ALL=C sort -u >"$verified_sources"
[ -s "$verified_sources" ] || fail 'verified-module manifest selected no sources'
set --
while IFS= read -r source; do
    case "$source" in
        ''|/*|*..*|*[!A-Za-z0-9._/-]*) fail "invalid verified source path: $source" ;;
    esac
    set -- "$@" "$qualified_repo/$source"
done <"$verified_sources"
python3 -I "$qualified_scripts/check-source.py" --verus-blocks "$@"

transcript="$scratch/proof-build.transcript"
: >"$transcript"
counts="$scratch/proof-counts.txt"
: >"$counts"
artifact_stage="$scratch/qualified-artifacts"
mkdir -p "$artifact_stage/release"
expected_verus=$(sed -n '1p' "$qualified_scripts/verus/VERUS_VERSION")
sed -n 's/^package=\([^|]*\)|\([^|]*\)$/\1|\2/p' \
    "$qualified_scripts/VERIFIED_MODULES" >"$scratch/proof-packages"
[ -s "$scratch/proof-packages" ] || fail 'coverage manifest selected no proof packages'
while IFS='|' read -r package crate_name extra; do
    [ -n "$package" ] && [ -n "$crate_name" ] && [ -z "$extra" ] || \
        fail 'malformed proof package record'
    case "$package$crate_name" in
        *[!A-Za-z0-9_.-]*) fail 'unsafe proof package identity' ;;
    esac
    package_transcript="$scratch/proof-$package.transcript"
    set +e
    (
        cd "$qualified_repo"
        VERUS_Z3_PATH="$verus_root/z3" timeout "$timeout_seconds" \
            "$verus_root/cargo-verus" build -p "$package" --locked --release \
            --target-dir "$proof_target" --fwd-verus-args-to roots -j 1 -- \
            --no-cheating --output-json
    ) >"$package_transcript" 2>&1
    build_status=$?
    set -e
    {
        printf 'PACKAGE=%s\n' "$package"
        cat "$package_transcript"
    } >>"$transcript"
    cat "$package_transcript"
    [ "$build_status" -ne 124 ] || fail "$package strict proof/release build timed out"
    [ "$build_status" -eq 0 ] || \
        fail "$package strict proof/release build failed with status $build_status"
    package_counts="$scratch/proof-$package.counts"
    python3 -I "$qualified_scripts/check-transcript.py" \
        "$package" "$crate_name" "$qualified_scripts/VERIFIED_MODULES" \
        "$package_transcript" "$package_counts" "$expected_verus"
    cat "$package_counts" >>"$counts"
    artifact_name="lib$(printf '%s' "$crate_name" | tr '-' '_').rlib"
    [ -f "$proof_target/release/$artifact_name" ] || \
        fail "$package strict root artifact is missing: $artifact_name"
    cp "$proof_target/release/$artifact_name" "$artifact_stage/release/$artifact_name"
done <"$scratch/proof-packages"
[ "$(awk -F '|' '{ total += $2 } END { print total + 0 }' "$counts")" -gt 0 ] || \
    fail 'workspace qualification verified no proof obligations'

negative="$scratch/negative"
FERRIC_NEGATIVE_TIMEOUT_SECONDS="$timeout_seconds" \
    "$qualified_scripts/negative/run-same-source.sh" \
    "$qualified_repo" "$verus_root" "$negative"
"$qualified_scripts/negative/test-policy.sh" \
    "$qualified_repo" "$metadata" "$negative" "$source_gate"

[ "$(sha256sum "$source_gate" | awk '{ print $1 }')" = "$source_gate_digest" ] || \
    fail 'compiler-rooted source gate changed during qualification'
"$source_gate" "$qualified_repo" "$qualified_scripts/VERIFIED_MODULES" "$metadata"

post_closure_log="$scratch/verus-closure-post.transcript"
chmod -R u+w "$verus_root"
"$qualified_scripts/verify-verus-closure.sh" \
    "$verus_root" "$qualified_scripts/verus/VERUS_CLOSURE_MANIFEST" >"$post_closure_log"
cmp -s "$closure_log" "$post_closure_log" || fail 'authenticated Verus closure changed during qualification'
chmod -R u+w "$source_snapshot"
snapshot_source_after="$scratch/snapshot-source-after.records"
python3 -I "$qualified_scripts/source-closure.py" "$source_snapshot" "$snapshot_source_after"
cmp -s "$snapshot_source" "$snapshot_source_after" || fail 'read-only source snapshot changed during qualification'
live_source_after="$scratch/live-source-after.records"
python3 -I "$script_dir/source-closure.py" "$repo" "$live_source_after"
cmp -s "$live_source_before" "$live_source_after" || fail 'live source changed during qualification'

python3 -I "$qualified_scripts/record-qualification.py" \
    "$qualified_repo" "$verus_root" "$artifact_stage" "$metadata" "$transcript" "$counts" \
    "$closure_log" "$negative" "$snapshot_source" "$source_gate" "$receipt_dir"
printf 'PASS: Ferric strict proof and release qualification completed\n'
