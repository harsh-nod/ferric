#!/bin/sh
set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH='' cd -- "$script_dir/../../../.." && pwd)
expected_fe2o3_commit=310ce7b8c46b4a4add1d7523e20463ecf59d8733
expected_fe2o3_tree=53b61c8dc266a04525155e64b5af06f352a49d6a

for tool in cargo cp git mkdir mktemp patch python3 rm; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'FAIL: behavioral harness requires %s\n' "$tool" >&2
        exit 1
    }
done

scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-worker-v3-behavior.XXXXXX")
trap 'chmod -R u+w "$scratch" 2>/dev/null || true; rm -rf "$scratch"' EXIT HUP INT TERM
chmod 700 "$scratch"

metadata="$scratch/ferric-metadata.json"
(cd "$repo" && cargo metadata --locked --format-version 1) >"$metadata"
fe2o3_root=$(python3 -I - "$metadata" <<'PY'
import json
import pathlib
import sys

metadata = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
matches = [
    pathlib.Path(package["manifest_path"]).parents[2]
    for package in metadata["packages"]
    if package["name"] == "fe2o3-artifact-transaction"
]
if len(matches) != 1:
    raise SystemExit("pinned fe2o3 source did not resolve exactly once")
print(matches[0])
PY
)
[ "$(git -C "$fe2o3_root" rev-parse HEAD)" = "$expected_fe2o3_commit" ] || {
    printf 'FAIL: behavioral harness fe2o3 commit drifted\n' >&2
    exit 1
}
[ "$(git -C "$fe2o3_root" rev-parse 'HEAD^{tree}')" = "$expected_fe2o3_tree" ] || {
    printf 'FAIL: behavioral harness fe2o3 tree drifted\n' >&2
    exit 1
}

cp -a "$fe2o3_root" "$scratch/fe2o3"
cp -a "$repo" "$scratch/ferric"
chmod -R u+w "$scratch/fe2o3" "$scratch/ferric"
for fixture_patch in \
    fe2o3-root-manifest.patch \
    cargo-fe2o3-manifest.patch \
    fixture-hsaco.patch \
    fixture-publication.patch \
    fixture-wrapper.patch \
    collector-test.patch
do
    patch -d "$scratch/fe2o3" -p1 --fuzz=0 --batch --forward \
        <"$script_dir/patches/$fixture_patch"
done

target_dir=${FERRIC_BEHAVIOR_TARGET_DIR:-$scratch/target}
mkdir -p "$target_dir"
(
    cd "$scratch/fe2o3"
    export CARGO_TARGET_DIR="$target_dir"
    cargo generate-lockfile
    cargo test --locked --ignore-rust-version -p cargo-fe2o3 \
        --features worker-v3-envelope-integration-test-only \
        --test worker_v3_load_envelope_v2 \
        ferric_all12_collector_accepts_exact_live_inputs_and_rejects_hostile_state \
        -- --nocapture
)

printf '%s\n' \
    'PASS: synthetic test-only all-12 collector harness exercised retained recovery, full binding rejection, lock retention, and sealed stale-current rejection' \
    'NONCLAIM: synthetic fixtures grant no artifact, promotion, CURRENT, load, launch, hardware, or production authority'
