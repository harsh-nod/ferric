#!/bin/sh
set -eu

usage() {
    printf 'usage: %s REPO PROPERTY_BINDER SOURCE_GATE OUTPUT_DIR\n' "$0" >&2
    exit 2
}

[ "$#" -eq 4 ] || usage
repo=$(CDPATH='' cd -- "$1" && pwd)
binder=$2
source_gate=$3
output=$4
[ -x "$binder" ] || { printf 'FAIL: property binder is unavailable\n' >&2; exit 1; }
[ -x "$source_gate" ] || { printf 'FAIL: source gate is unavailable\n' >&2; exit 1; }
mkdir -p "$output"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-property-policy.XXXXXX")
trap 'chmod -R u+w "$scratch" 2>/dev/null || true; rm -rf "$scratch"' EXIT HUP INT TERM

expect_rejected() {
    name=$1
    expected=$2
    shift 2
    transcript="$output/$name.transcript"
    printf 'FIXTURE=%s\n' "$name" >"$transcript"
    set +e
    "$@" >>"$transcript" 2>&1
    status=$?
    set -e
    if [ "$status" -eq 0 ] || ! grep -F "$expected" "$transcript" >/dev/null; then
        printf 'FAIL: %s was not rejected with %s\n' "$name" "$expected" >&2
        cat "$transcript" >&2
        exit 1
    fi
}

manifest_copy() {
    name=$1
    fixture="$scratch/$name"
    mkdir -p "$fixture/docs" "$fixture/proofs"
    cp "$repo/docs/M0_PROPERTY_CONTRACT.md" "$fixture/docs/"
    cp "$repo/proofs/M0_PROPERTIES.json" "$fixture/proofs/"
    printf '%s\n' "$fixture"
}

omission=$(manifest_copy property-omission)
python3 -I - "$omission/proofs/M0_PROPERTIES.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
value["properties"].pop()
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
expect_rejected property-omission 'M0 property roster has 14 records, expected 15' \
    "$binder" --manifest-check "$omission"

upgrade=$(manifest_copy property-status-upgrade)
python3 -I - "$upgrade/proofs/M0_PROPERTIES.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
record = next(item for item in value["properties"] if item["name"] == "m0.machine_refinement")
record["required_status"] = "Proved"
record["compiler_paths_resolved"] = True
record["compiler_path_prefixes"] = ["ferric_engine::fixture::machine_refinement"]
record["required_mutations"] = ["fixture"]
record["unsupported_reason"] = None
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
expect_rejected property-status-upgrade 'required property kind/status/order drifted' \
    "$binder" --manifest-check "$upgrade"

invalid=$(manifest_copy property-invalid-evidence)
python3 -I - "$invalid/proofs/M0_PROPERTIES.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
record = next(item for item in value["properties"] if item["name"] == "m0.request_generation")
record["required_mutations"] = []
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
expect_rejected property-invalid-evidence 'invalid Proved evidence declaration' \
    "$binder" --manifest-check "$invalid"

documentation=$(manifest_copy property-documentation-drift)
python3 -I - "$documentation/proofs/M0_PROPERTIES.json" <<'PY'
import hashlib
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
record = value["properties"][0]
record["statement"] += " Altered."
record["statement_sha256"] = hashlib.sha256(record["statement"].encode()).hexdigest()
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
expect_rejected property-documentation-drift 'manifest and documentation disagree' \
    "$binder" --manifest-check "$documentation"

fixture="$scratch/closed-fixture"
mkdir -p "$fixture/docs" "$fixture/proofs/source-gate" \
    "$fixture/proofs/property-binder" "$fixture/proofs/negative/components" \
    "$fixture/proofs/verus" "$fixture/evidence" "$fixture/verus" "$fixture/artifacts" \
    "$fixture/crates/fixture"
cp "$repo/docs/M0_PROPERTY_CONTRACT.md" "$repo/docs/ASSURANCE.md" "$fixture/docs/"
cp "$repo/Cargo.lock" "$fixture/"
cp "$repo/proofs/M0_PROPERTIES.json" "$repo/proofs/UNVERIFIED_BODIES" "$fixture/proofs/"
cp "$repo/proofs/source-gate/Cargo.lock" "$repo/proofs/source-gate/DEPENDENCY_TCB" \
    "$fixture/proofs/source-gate/"
cp "$repo/proofs/property-binder/Cargo.lock" "$repo/proofs/property-binder/DEPENDENCY_TCB" \
    "$fixture/proofs/property-binder/"
cp "$repo/proofs/verus/VERUS_CLOSURE_MANIFEST" "$repo/proofs/verus/VERUS_VERSION" \
    "$fixture/proofs/verus/"

printf 'fixture source\n' >"$fixture/crates/fixture/source.rs"
python3 -I - "$fixture/proofs/M0_PROPERTIES.json" \
    "$fixture/proofs/VERIFIED_MODULES" \
    "$fixture/proofs/negative/REQUIRED_COMPONENTS" "$fixture/components" \
    "$fixture/proof.counts" <<'PY'
import json
from pathlib import Path
import re
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))

def package_for(mutation):
    if mutation == "identity-trust" or mutation == "request-id-generation" or mutation.startswith("speculation"):
        return "ferric-spec", "ferric_spec"
    return "ferric-engine", "ferric_engine"

def target_for(mutation):
    package, crate = package_for(mutation)
    if mutation == "identity-trust":
        function = "Identity::is_present"
    else:
        function = "Mutation::" + re.sub(r"[^A-Za-z0-9_]", "_", mutation)
    return package, crate, "fixture", function, f"{crate}::fixture::{function}"

for record in value["properties"]:
    if record["required_status"] == "Proved":
        record["compiler_paths_resolved"] = True
        record["compiler_path_prefixes"] = sorted({
            target_for(mutation)[4]
            for mutation in record["required_mutations"]
        })
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

manifest = value
names = sorted({
    mutation
    for record in manifest["properties"]
    for mutation in record["required_mutations"]
} | {"identity-trust"})
registry = ["format=FERRIC-NEGATIVE-COMPONENTS-V2"]
components = []
verified = [
    "format=FERRIC-VERIFIED-MODULES-V2",
    "package=ferric-spec|ferric_spec",
    "package=ferric-engine|ferric_engine",
]
targets = {target_for(name)[4] for name in names}
counts = {"ferric-spec": 0, "ferric-engine": 0}
for compiler_path in sorted(targets):
    package = "ferric-spec" if compiler_path.startswith("ferric_spec::") else "ferric-engine"
    verified.append(
        f"verified={package}|crates/fixture/source.rs|{compiler_path}"
    )
    counts[package] += 1
for name in names:
    package, _crate, module, function, _compiler_path = target_for(name)
    marker = "no-cheating" if name == "identity-trust" else "proof"
    registry.append(
        f"always={name}|{package}|{name}.py|{marker}|{module}|{function}"
    )
    components.append(f"{name}|{package}|{marker}|{function}")
Path(sys.argv[2]).write_text("\n".join(verified) + "\n", encoding="utf-8")
Path(sys.argv[3]).write_text("\n".join(registry) + "\n", encoding="utf-8")
Path(sys.argv[4]).write_text("\n".join(components) + "\n", encoding="utf-8")
Path(sys.argv[5]).write_text(
    "".join(f"{package}|{counts[package]}|0|{counts[package]}\n" for package in sorted(counts)),
    encoding="utf-8",
)
PY
while IFS='|' read -r component package marker function; do
    printf 'fixture mutator %s\n' "$component" \
        >"$fixture/proofs/negative/components/$component.py"
    mutator_hash=$(sha256sum "$fixture/proofs/negative/components/$component.py" | awk '{ print $1 }')
    {
        printf 'MUTATED_SOURCE=crates/fixture/source.rs\n'
        printf 'MUTATOR_SHA256=%s\n' "$mutator_hash"
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=fixture\n'
        printf 'VERUS_FUNCTION=%s\n' "$function"
    } >"$fixture/evidence/$component.mutation"
    {
        printf 'VERUS_PACKAGE=%s\n' "$package"
        printf 'VERUS_MODULE=fixture\n'
        printf 'VERUS_FUNCTION=%s\n' "$function"
    } >"$fixture/evidence/$component.transcript"
    if [ "$marker" = no-cheating ]; then
        printf 'error: assume/admit not allowed with --no-cheating\n' \
            >>"$fixture/evidence/$component.transcript"
    else
        printf 'verification results:: 1 verified, 1 errors\n' \
            >>"$fixture/evidence/$component.transcript"
    fi
done <"$fixture/components"
printf 'FIXTURE=parser-transition-allocation\nFAIL: verified engine body violates no-transition-allocation policy\n' \
    >"$fixture/evidence/parser-transition-allocation.transcript"
printf 'source closure fixture\n' >"$fixture/source.records"
printf 'PACKAGE=ferric-spec\nfixture proof output\nPACKAGE=ferric-engine\n' \
    >"$fixture/proof.transcript"
printf 'FERRIC_QUALITY_GATE=fmt:PASS\nFERRIC_QUALITY_GATE=clippy:PASS\nFERRIC_QUALITY_GATE=test-debug:PASS\nFERRIC_QUALITY_GATE=test-release:PASS\ntest transition_paths_preserve_fixed_storage_capacities ... ok\ntest result: ok. 1 passed; 0 failed\n' \
    >"$fixture/runtime-tests.transcript"
for tool in cargo-verus verus rust_verify z3; do
    cp /bin/true "$fixture/verus/$tool"
done
printf 'ferric spec artifact\n' >"$fixture/artifacts/libferric_spec.rlib"
printf 'ferric engine artifact\n' >"$fixture/artifacts/libferric_engine.rlib"

evidence_arguments="$fixture $fixture/source.records $fixture/proof.transcript $fixture/proof.counts $fixture/evidence $fixture/verus $source_gate $fixture/artifacts $fixture/runtime-tests.transcript"
for gate in fmt clippy test-debug test-release; do
    missing="$fixture/runtime-tests-missing-$gate.transcript"
    missing_index="$fixture/evidence-index-missing-$gate"
    grep -F -v "FERRIC_QUALITY_GATE=$gate:PASS" \
        "$fixture/runtime-tests.transcript" >"$missing"
    "$binder" --evidence-index \
        "$fixture" "$fixture/source.records" "$fixture/proof.transcript" \
        "$fixture/proof.counts" "$fixture/evidence" "$fixture/verus" \
        "$source_gate" "$fixture/artifacts" "$missing" "$missing_index"
    expect_rejected "quality-gate-missing-$gate" \
        "quality gate transcript marker is not exact: $gate" \
        "$binder" --bind \
        "$fixture" "$fixture/source.records" "$fixture/proof.transcript" \
        "$fixture/proof.counts" "$fixture/evidence" "$fixture/verus" \
        "$source_gate" "$fixture/artifacts" "$missing" \
        "$missing_index" "$fixture/contract-missing-$gate"
done
# shellcheck disable=SC2086
"$binder" --evidence-index $evidence_arguments "$fixture/evidence-index-a"
# shellcheck disable=SC2086
"$binder" --evidence-index $evidence_arguments "$fixture/evidence-index-b"
cmp -s "$fixture/evidence-index-a" "$fixture/evidence-index-b" || {
    printf 'FAIL: property evidence indexes are nondeterministic\n' >&2
    exit 1
}
# shellcheck disable=SC2086
"$binder" --bind $evidence_arguments "$fixture/evidence-index-a" "$fixture/contract-a"
# shellcheck disable=SC2086
"$binder" --bind $evidence_arguments "$fixture/evidence-index-a" "$fixture/contract-b"
cmp -s "$fixture/contract-a" "$fixture/contract-b" || {
    printf 'FAIL: property contract artifacts are nondeterministic\n' >&2
    exit 1
}
grep -F 'contract-set-validation=validate_closed:PASS' "$fixture/contract-a" >/dev/null || {
    printf 'FAIL: property contract did not record fe2o3 structural validation\n' >&2
    exit 1
}

cp "$fixture/evidence/request-id-generation.mutation" \
    "$fixture/request-id-generation.mutation"
python3 -I - "$fixture/evidence/request-id-generation.mutation" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
old = "VERUS_PACKAGE=ferric-spec"
if text.count(old) != 1:
    raise SystemExit("wrong-package marker fixture anchor drifted")
path.write_text(text.replace(old, "VERUS_PACKAGE=ferric-engine"), encoding="utf-8")
PY
# shellcheck disable=SC2086
expect_rejected property-wrong-mutation-package \
    'mutation marker does not bind its source, mutator, and exact Verus target' \
    "$binder" --evidence-index $evidence_arguments "$fixture/wrong-package-index"
mv "$fixture/request-id-generation.mutation" \
    "$fixture/evidence/request-id-generation.mutation"

cp "$fixture/proofs/negative/REQUIRED_COMPONENTS" "$fixture/required-components"
python3 -I - "$fixture/proofs/negative/REQUIRED_COMPONENTS" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
old = "request-id-generation|ferric-spec"
if text.count(old) != 1:
    raise SystemExit("wrong-package registry fixture anchor drifted")
path.write_text(text.replace(old, "request-id-generation|ferric-engine"), encoding="utf-8")
PY
# shellcheck disable=SC2086
expect_rejected property-wrong-registry-package \
    'mutation target matched no verified compiler path' \
    "$binder" --evidence-index $evidence_arguments "$fixture/wrong-registry-package-index"
mv "$fixture/required-components" "$fixture/proofs/negative/REQUIRED_COMPONENTS"

cp "$fixture/proofs/negative/REQUIRED_COMPONENTS" "$fixture/required-components"
python3 -I - "$fixture/proofs/negative/REQUIRED_COMPONENTS" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
old = "|fixture|Mutation::request_id_generation"
if text.count(old) != 1:
    raise SystemExit("nonexistent-target registry fixture anchor drifted")
path.write_text(text.replace(old, "|fixture|Mutation::missing"), encoding="utf-8")
PY
# shellcheck disable=SC2086
expect_rejected property-nonexistent-mutation-target \
    'mutation target matched no verified compiler path' \
    "$binder" --evidence-index $evidence_arguments "$fixture/nonexistent-target-index"
mv "$fixture/required-components" "$fixture/proofs/negative/REQUIRED_COMPONENTS"

cp "$fixture/proofs/negative/REQUIRED_COMPONENTS" "$fixture/required-components"
cp "$fixture/proofs/VERIFIED_MODULES" "$fixture/verified-modules"
python3 -I - "$fixture/proofs/negative/REQUIRED_COMPONENTS" \
    "$fixture/proofs/VERIFIED_MODULES" <<'PY'
from pathlib import Path
import sys

registry = Path(sys.argv[1])
text = registry.read_text(encoding="utf-8")
old = "|fixture|Identity::is_present"
if text.count(old) != 1:
    raise SystemExit("ambiguous-target registry fixture anchor drifted")
registry.write_text(text.replace(old, "|fixture|is_present"), encoding="utf-8")
verified = Path(sys.argv[2])
with verified.open("a", encoding="utf-8") as output:
    output.write("verified=ferric-spec|crates/fixture/source.rs|ferric_spec::fixture::Other::is_present\n")
PY
# shellcheck disable=SC2086
expect_rejected property-ambiguous-mutation-target \
    'mutation target is ambiguous' \
    "$binder" --evidence-index $evidence_arguments "$fixture/ambiguous-target-index"
mv "$fixture/required-components" "$fixture/proofs/negative/REQUIRED_COMPONENTS"
mv "$fixture/verified-modules" "$fixture/proofs/VERIFIED_MODULES"

cp "$fixture/proofs/M0_PROPERTIES.json" "$fixture/m0-properties.json"
python3 -I - "$fixture/proofs/M0_PROPERTIES.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
record = next(item for item in value["properties"] if item["name"] == "m0.kv_transition")
record["required_mutations"].append("speculation-prefix")
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
# shellcheck disable=SC2086
"$binder" --evidence-index $evidence_arguments "$fixture/outside-target-index"
# shellcheck disable=SC2086
expect_rejected property-mutation-target-outside-paths \
    'property mutation target is outside resolved compiler paths' \
    "$binder" --bind $evidence_arguments "$fixture/outside-target-index" \
    "$fixture/outside-target-contract"
mv "$fixture/m0-properties.json" "$fixture/proofs/M0_PROPERTIES.json"

mv "$fixture/evidence/kv-read-initialized.mutation" \
    "$fixture/kv-read-initialized.mutation"
# shellcheck disable=SC2086
expect_rejected property-missing-mutation-evidence \
    'required mutation evidence missing' \
    "$binder" --evidence-index $evidence_arguments "$fixture/missing-evidence-index"
mv "$fixture/kv-read-initialized.mutation" \
    "$fixture/evidence/kv-read-initialized.mutation"

printf 'UNREGISTERED=1\n' >"$fixture/evidence/unregistered.mutation"
# shellcheck disable=SC2086
expect_rejected property-extra-mutation-evidence \
    'unregistered mutation marker' \
    "$binder" --evidence-index $evidence_arguments "$fixture/extra-evidence-index"
rm "$fixture/evidence/unregistered.mutation"

package=$(awk -F '|' '$1 == "lifecycle" { print $2 }' "$fixture/components")
function=$(awk -F '|' '$1 == "lifecycle" { print $4 }' "$fixture/components")
{
    printf 'VERUS_PACKAGE=%s\n' "$package"
    printf 'VERUS_MODULE=fixture\n'
    printf 'VERUS_FUNCTION=%s\n' "$function"
    printf 'RESULT=rejected-without-authenticated-marker\n'
} >"$fixture/evidence/lifecycle.transcript"
# shellcheck disable=SC2086
expect_rejected property-weak-mutation-evidence \
    'mutation transcript does not contain its required failure marker' \
    "$binder" --evidence-index $evidence_arguments "$fixture/weak-evidence-index"
{
    printf 'VERUS_PACKAGE=%s\n' "$package"
    printf 'VERUS_MODULE=fixture\n'
    printf 'VERUS_FUNCTION=%s\n' "$function"
    printf 'verification results:: 1 verified, 1 errors\n'
} >"$fixture/evidence/lifecycle.transcript"

printf 'changed source closure fixture\n' >"$fixture/source.records"
# shellcheck disable=SC2086
expect_rejected property-stale-hash 'M0 evidence index contains stale or mismatched hashes' \
    "$binder" --bind $evidence_arguments "$fixture/evidence-index-a" "$fixture/stale-contract"

printf 'PASS: M0 property binder rejects omitted, upgraded, inconsistent, invalid, and stale claims and emits deterministic fe2o3 contracts\n'
