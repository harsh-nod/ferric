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

mkdir -p "$repo/target"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-qualification.XXXXXX")
proof_target=$(mktemp -d "${TMPDIR:-/tmp}/ferric-proof-target.XXXXXX")
if [ -n "${FERRIC_RECEIPT_DIR:-}" ]; then
    receipt_dir=$FERRIC_RECEIPT_DIR
    [ ! -e "$receipt_dir" ] || fail "receipt destination already exists: $receipt_dir"
    mkdir -p "$receipt_dir"
else
    receipt_dir=$(mktemp -d "$repo/target/ferric-receipt.XXXXXX")
fi
trap 'chmod -R u+w "$scratch" "$proof_target" 2>/dev/null || true; rm -rf "$scratch" "$proof_target"' EXIT HUP INT TERM

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
live_source_before="$scratch/live-source-before.records"
snapshot_source="$scratch/snapshot-source.records"
python3 "$script_dir/source-closure.py" "$repo" "$live_source_before"
python3 "$script_dir/source-closure.py" "$source_snapshot" "$snapshot_source"
cmp -s "$live_source_before" "$snapshot_source" || fail 'source changed while creating the qualification snapshot'
chmod -R a-w "$source_snapshot"
qualified_repo=$source_snapshot
qualified_scripts="$qualified_repo/proofs"

python3 "$qualified_scripts/check-lock.py" "$qualified_repo/Cargo.lock"
metadata="$scratch/cargo-metadata.json"
(
    cd "$qualified_repo"
    cargo metadata --locked --no-deps --format-version 1
) >"$metadata"
generated_coverage="$scratch/VERIFIED_MODULES.generated"
python3 "$qualified_scripts/check-coverage.py" --generate "$qualified_repo" "$metadata" "$generated_coverage"
cmp -s "$qualified_scripts/VERIFIED_MODULES" "$generated_coverage" || {
    printf 'FAIL: proof coverage manifest is stale; regenerate with:\n' >&2
    printf '  python3 proofs/check-coverage.py --generate . METADATA proofs/VERIFIED_MODULES\n' >&2
    exit 1
}
python3 "$qualified_scripts/check-coverage.py" \
    "$qualified_repo" "$qualified_scripts/VERIFIED_MODULES" "$metadata"

verified_sources="$scratch/verified-sources"
sed -n 's/^verified=\([^|]*\)|.*/\1/p' "$qualified_scripts/VERIFIED_MODULES" | LC_ALL=C sort -u >"$verified_sources"
[ -s "$verified_sources" ] || fail 'verified-module manifest selected no sources'
set --
while IFS= read -r source; do
    case "$source" in
        ''|/*|*..*|*[!A-Za-z0-9._/-]*) fail "invalid verified source path: $source" ;;
    esac
    set -- "$@" "$qualified_repo/$source"
done <"$verified_sources"
python3 "$qualified_scripts/check-source.py" --verus-blocks "$@"

transcript="$scratch/proof-build.transcript"
set +e
(
    cd "$qualified_repo"
    VERUS_Z3_PATH="$verus_root/z3" timeout "$timeout_seconds" \
        "$verus_root/cargo-verus" build --workspace --locked --release \
        --target-dir "$proof_target" --fwd-verus-args-to roots -j 1 -- --no-cheating
) >"$transcript" 2>&1
build_status=$?
set -e
cat "$transcript"
[ "$build_status" -ne 124 ] || fail 'strict proof/release build timed out'
[ "$build_status" -eq 0 ] || fail "strict proof/release build failed with status $build_status"

counts="$scratch/proof-counts.txt"
python3 "$qualified_scripts/check-transcript.py" "$metadata" "$transcript" "$counts"

negative="$scratch/negative"
FERRIC_NEGATIVE_TIMEOUT_SECONDS="$timeout_seconds" \
    "$qualified_scripts/negative/run-same-source.sh" \
    "$qualified_repo" "$verus_root" "$negative"
"$qualified_scripts/negative/test-policy.sh" "$qualified_repo" "$metadata" "$negative"

post_closure_log="$scratch/verus-closure-post.transcript"
chmod -R u+w "$verus_root"
"$qualified_scripts/verify-verus-closure.sh" \
    "$verus_root" "$qualified_scripts/verus/VERUS_CLOSURE_MANIFEST" >"$post_closure_log"
cmp -s "$closure_log" "$post_closure_log" || fail 'authenticated Verus closure changed during qualification'
chmod -R u+w "$source_snapshot"
snapshot_source_after="$scratch/snapshot-source-after.records"
python3 "$qualified_scripts/source-closure.py" "$source_snapshot" "$snapshot_source_after"
cmp -s "$snapshot_source" "$snapshot_source_after" || fail 'read-only source snapshot changed during qualification'
live_source_after="$scratch/live-source-after.records"
python3 "$script_dir/source-closure.py" "$repo" "$live_source_after"
cmp -s "$live_source_before" "$live_source_after" || fail 'live source changed during qualification'

python3 "$qualified_scripts/record-qualification.py" \
    "$qualified_repo" "$verus_root" "$proof_target" "$metadata" "$transcript" "$counts" \
    "$closure_log" "$negative" "$snapshot_source" "$receipt_dir"
printf 'PASS: Ferric strict proof and release qualification completed\n'
