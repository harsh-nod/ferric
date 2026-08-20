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

python3 -I - "$fixture/proofs/M0_PROPERTIES.json" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
for record in value["properties"]:
    if record["required_status"] == "Proved" and record["name"] not in {
        "m0.request_generation",
        "m0.greedy_speculation",
    }:
        token = record["name"].removeprefix("m0.")
        crate = "ferric_engine" if token.startswith("kv_") or token == "engine_composition" else "ferric_spec"
        record["compiler_paths_resolved"] = True
        record["compiler_path_prefixes"] = [f"{crate}::fixture::{token}"]
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY

cat >"$fixture/proofs/VERIFIED_MODULES" <<'EOF'
format=FERRIC-VERIFIED-MODULES-V2
package=ferric-spec|ferric_spec
package=ferric-engine|ferric_engine
verified=ferric-spec|crates/ferric-spec/src/identity.rs|ferric_spec::identity::RequestId::new
verified=ferric-spec|crates/ferric-spec/src/speculation.rs|ferric_spec::speculation::verify_greedy_round
verified=ferric-spec|crates/ferric-spec/src/speculation.rs|ferric_spec::speculation::GreedyCommit::emitted_tokens
verified=ferric-spec|crates/ferric-spec/src/fixture.rs|ferric_spec::fixture::scheduler_transition
verified=ferric-spec|crates/ferric-spec/src/fixture.rs|ferric_spec::fixture::scheduler_lifetime
verified=ferric-spec|crates/ferric-spec/src/fixture.rs|ferric_spec::fixture::scheduler_bounds
verified=ferric-engine|crates/ferric-engine/src/fixture.rs|ferric_engine::fixture::kv_transition
verified=ferric-engine|crates/ferric-engine/src/fixture.rs|ferric_engine::fixture::kv_sharing_rollback
verified=ferric-engine|crates/ferric-engine/src/fixture.rs|ferric_engine::fixture::kv_generation
verified=ferric-engine|crates/ferric-engine/src/fixture.rs|ferric_engine::fixture::kv_bounds
verified=ferric-engine|crates/ferric-engine/src/fixture.rs|ferric_engine::fixture::engine_composition
EOF
cat >"$fixture/proofs/negative/REQUIRED_COMPONENTS" <<'EOF'
format=FERRIC-NEGATIVE-COMPONENTS-V1
always=identity-trust|ferric-spec|identity-trust.py|no-cheating
always=speculation|ferric-spec|speculation.py|proof
always=lifecycle|ferric-engine|lifecycle.py|proof
always=kv|ferric-engine|kv.py|proof
always=system|ferric-engine|system.py|proof
EOF
for component in identity-trust speculation lifecycle kv system; do
    printf 'fixture source %s\n' "$component" >"$fixture/crates/fixture/$component.rs"
    printf 'fixture mutator %s\n' "$component" \
        >"$fixture/proofs/negative/components/$component.py"
    mutator_hash=$(sha256sum "$fixture/proofs/negative/components/$component.py" | awk '{ print $1 }')
    {
        printf 'MUTATED_SOURCE=crates/fixture/%s.rs\n' "$component"
        printf 'MUTATOR_SHA256=%s\n' "$mutator_hash"
    } >"$fixture/evidence/$component.mutation"
    if [ "$component" = identity-trust ]; then
        printf 'error: assume/admit not allowed with --no-cheating\n' \
            >"$fixture/evidence/$component.transcript"
    else
        printf 'verification results:: 1 verified, 1 errors\n' \
            >"$fixture/evidence/$component.transcript"
    fi
done
printf 'FIXTURE=parser-transition-allocation\nFAIL: verified engine body violates no-transition-allocation policy\n' \
    >"$fixture/evidence/parser-transition-allocation.transcript"
printf 'source closure fixture\n' >"$fixture/source.records"
printf 'PACKAGE=ferric-spec\nfixture proof output\nPACKAGE=ferric-engine\n' \
    >"$fixture/proof.transcript"
printf 'ferric-spec|6|0|6\nferric-engine|5|0|5\n' >"$fixture/proof.counts"
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

printf 'RESULT=rejected-without-authenticated-marker\n' \
    >"$fixture/evidence/lifecycle.transcript"
# shellcheck disable=SC2086
expect_rejected property-weak-mutation-evidence \
    'mutation transcript does not contain its required failure marker' \
    "$binder" --evidence-index $evidence_arguments "$fixture/weak-evidence-index"
printf 'verification results:: 1 verified, 1 errors\n' \
    >"$fixture/evidence/lifecycle.transcript"

printf 'changed source closure fixture\n' >"$fixture/source.records"
# shellcheck disable=SC2086
expect_rejected property-stale-hash 'M0 evidence index contains stale or mismatched hashes' \
    "$binder" --bind $evidence_arguments "$fixture/evidence-index-a" "$fixture/stale-contract"

printf 'PASS: M0 property binder rejects omitted, upgraded, inconsistent, invalid, and stale claims and emits deterministic fe2o3 contracts\n'
