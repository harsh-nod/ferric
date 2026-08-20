#!/bin/sh
set -eu

usage() {
    printf 'usage: %s REPO METADATA_JSON OUTPUT_DIR\n' "$0" >&2
    exit 2
}

[ "$#" -eq 3 ] || usage
repo=$(CDPATH='' cd -- "$1" && pwd)
metadata=$2
output=$3
[ -f "$metadata" ] || {
    printf 'FAIL: Cargo metadata is unavailable: %s\n' "$metadata" >&2
    exit 1
}
mkdir -p "$output"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-policy-negative.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

python3 - "$repo/proofs/VERIFIED_MODULES" "$scratch/missing-module.manifest" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
output = [source[0]]
removed = False
skip = False
for line in source[1:]:
    if line.startswith("module="):
        if not removed:
            removed = True
            skip = True
            continue
        skip = False
    if not skip:
        output.append(line)
if not removed:
    raise SystemExit("coverage manifest contains no removable module")
Path(sys.argv[2]).write_text("\n".join(output) + "\n", encoding="utf-8")
PY
set +e
printf 'FIXTURE=removed-manifest-module\n' >"$output/coverage-missing-module.transcript"
python3 "$repo/proofs/check-coverage.py" \
    "$repo" "$scratch/missing-module.manifest" "$metadata" \
    >>"$output/coverage-missing-module.transcript" 2>&1
status=$?
set -e
if [ "$status" -eq 0 ] || ! grep -F 'source closure drifted' \
    "$output/coverage-missing-module.transcript" >/dev/null; then
    printf 'FAIL: missing coverage module was not rejected\n' >&2
    exit 1
fi

copy="$scratch/source-copy"
mkdir -p "$copy"
cp -a "$repo/crates" "$copy/"
chmod -R u+w "$copy"
python3 - "$metadata" "$repo" "$copy" "$scratch/copied-metadata.json" <<'PY'
import json
from pathlib import Path
import sys

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
old = str(Path(sys.argv[2]).resolve())
new = str(Path(sys.argv[3]).resolve())
for package in metadata["packages"]:
    for target in package["targets"]:
        source = target["src_path"]
        if source.startswith(old + "/crates/"):
            target["src_path"] = new + source[len(old):]
Path(sys.argv[4]).write_text(json.dumps(metadata), encoding="utf-8")
PY
printf 'pub fn escaped_production_body() {}\n' >"$copy/crates/ferric-spec/src/unlisted_production.rs"
set +e
printf 'FIXTURE=created-unlisted-production-source\n' >"$output/coverage-unlisted-source.transcript"
python3 "$repo/proofs/check-coverage.py" \
    "$copy" "$repo/proofs/VERIFIED_MODULES" "$scratch/copied-metadata.json" \
    >>"$output/coverage-unlisted-source.transcript" 2>&1
status=$?
set -e
if [ "$status" -eq 0 ] || ! grep -F 'source closure drifted' \
    "$output/coverage-unlisted-source.transcript" >/dev/null; then
    printf 'FAIL: unlisted production source was not rejected\n' >&2
    exit 1
fi

scanner_source="$scratch/identity.rs"
cp "$repo/crates/ferric-spec/src/identity.rs" "$scanner_source"
chmod u+w "$scanner_source"
python3 - "$scanner_source" 'external' <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
source = path.read_text(encoding="utf-8")
anchor = "    pub fn is_present(&self)"
replacement = "    #[verifier :: external]\n" + anchor
if source.count(anchor) != 1:
    raise SystemExit("external scanner mutation anchor drifted")
path.write_text(source.replace(anchor, replacement), encoding="utf-8")
PY
set +e
printf 'FIXTURE=inserted-verifier-external-attribute\n' >"$output/scanner-external.transcript"
python3 "$repo/proofs/check-source.py" --verus-blocks "$scanner_source" \
    >>"$output/scanner-external.transcript" 2>&1
status=$?
set -e
if [ "$status" -eq 0 ] || ! grep -F "forbidden verifier attribute 'external'" \
    "$output/scanner-external.transcript" >/dev/null; then
    printf 'FAIL: verifier::external scanner mutation was not rejected\n' >&2
    exit 1
fi

cp "$repo/crates/ferric-spec/src/identity.rs" "$scanner_source"
chmod u+w "$scanner_source"
python3 - "$scanner_source" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
source = path.read_text(encoding="utf-8")
anchor = "    pub fn is_present(&self)"
replacement = "    #[verifier::trusted]\n" + anchor
if source.count(anchor) != 1:
    raise SystemExit("trusted scanner mutation anchor drifted")
path.write_text(source.replace(anchor, replacement), encoding="utf-8")
PY
set +e
printf 'FIXTURE=inserted-verifier-trusted-attribute\n' >"$output/scanner-trusted.transcript"
python3 "$repo/proofs/check-source.py" --verus-blocks "$scanner_source" \
    >>"$output/scanner-trusted.transcript" 2>&1
status=$?
set -e
if [ "$status" -eq 0 ] || ! grep -F "forbidden verifier attribute 'trusted'" \
    "$output/scanner-trusted.transcript" >/dev/null; then
    printf 'FAIL: verifier::trusted scanner mutation was not rejected\n' >&2
    exit 1
fi

boundary_log="$output/scanner-unverified-boundary.transcript"
{
    grep -F 'module=ferric-spec|crates/ferric-spec/src/configuration.rs' \
        "$repo/proofs/VERIFIED_MODULES"
    if grep -E '^verified=crates/ferric-spec/src/configuration[.]rs[|]' \
        "$repo/proofs/VERIFIED_MODULES"; then
        printf 'FAIL: configuration.rs unexpectedly enters the admitted Verus surface\n' >&2
        exit 1
    fi
    verified_sources="$scratch/verified-sources"
    sed -n 's/^verified=\([^|]*\)|.*/\1/p' "$repo/proofs/VERIFIED_MODULES" \
        | LC_ALL=C sort -u >"$verified_sources"
    if grep -F 'crates/ferric-spec/src/configuration.rs' "$verified_sources"; then
        printf 'FAIL: unverified-only source entered the Verus-block scanner set\n' >&2
        exit 1
    fi
    set --
    while IFS= read -r source; do
        set -- "$@" "$repo/$source"
    done <"$verified_sources"
    python3 "$repo/proofs/check-source.py" --verus-blocks "$@"
    printf 'PASS: unverified-only modules remain closure-accounted outside the proof scanner\n'
} >"$boundary_log" 2>&1

printf 'PASS: omitted module and unlisted production source fail coverage\n'
printf 'PASS: external and trusted verifier attributes fail lexical admission\n'
printf 'PASS: unverified-only module boundary is explicit and scanner-safe\n'
