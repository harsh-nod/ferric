#!/bin/sh
set -eu

usage() {
    printf 'usage: %s REPO METADATA_JSON OUTPUT_DIR SOURCE_GATE\n' "$0" >&2
    exit 2
}

[ "$#" -eq 4 ] || usage
repo=$(CDPATH='' cd -- "$1" && pwd)
metadata=$2
output=$3
source_gate=$4
[ -f "$metadata" ] || {
    printf 'FAIL: Cargo metadata is unavailable: %s\n' "$metadata" >&2
    exit 1
}
[ -x "$source_gate" ] || {
    printf 'FAIL: source gate is unavailable: %s\n' "$source_gate" >&2
    exit 1
}
mkdir -p "$output"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-policy-negative.XXXXXX")
trap 'chmod -R u+w "$scratch" 2>/dev/null || true; rm -rf "$scratch"' EXIT HUP INT TERM
registry_checker="$repo/proofs/negative/check-registry.py"
[ -f "$registry_checker" ] || {
    printf 'FAIL: negative registry checker is unavailable\n' >&2
    exit 1
}

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

new_copy() {
    name=$1
    destination="$scratch/$name"
    mkdir -p "$destination"
    cp -a "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/rust-toolchain.toml" "$destination/"
    cp -a "$repo/benches" "$destination/"
    cp -a "$repo/crates" "$destination/"
    mkdir -p "$destination/proofs"
    cp -a "$repo/proofs/m1" "$destination/proofs/"
    cp -a "$repo/proofs/UNVERIFIED_BODIES" \
        "$repo/proofs/RUNTIME_DEPENDENCY_TCB" "$destination/proofs/"
    chmod -R u+w "$destination"
    printf '%s\n' "$destination"
}

write_metadata() {
    source_repo=$1
    destination=$2
    (
        cd "$source_repo"
        cargo metadata --locked --format-version 1
    ) >"$destination"
}

runtime_tcb_hostile() {
    name=$1
    mutation=$2
    fixture=$(new_copy "runtime-tcb-$name")
    fixture_metadata="$scratch/runtime-tcb-$name.metadata"
    write_metadata "$fixture" "$fixture_metadata"
    python3 -I - "$fixture/proofs/RUNTIME_DEPENDENCY_TCB" \
        "$fixture/Cargo.lock" "$mutation" <<'PY'
from pathlib import Path
import sys

manifest = Path(sys.argv[1])
lock = Path(sys.argv[2])
mutation = sys.argv[3]
lines = manifest.read_text(encoding="utf-8").splitlines()
onig_package = next(
    index
    for index, line in enumerate(lines)
    if line.startswith("package=") and "#onig@6.5.3|onig|6.5.3|" in line
)
roots = [index for index, line in enumerate(lines) if line.startswith("root=")]
if mutation == "missing":
    del lines[onig_package]
elif mutation == "extra":
    lines.insert(onig_package, lines[onig_package].replace("|onig|", "|onig-extra|", 1))
elif mutation == "reordered":
    lines[roots[0]], lines[roots[1]] = lines[roots[1]], lines[roots[0]]
elif mutation == "duplicate":
    lines.insert(roots[0], lines[roots[0]])
elif mutation == "version":
    lines[onig_package] = lines[onig_package].replace("|onig|6.5.3|", "|onig|6.5.4|", 1)
elif mutation == "source":
    fields = lines[onig_package].split("|")
    fields[3] = "registry+https://example.invalid/index"
    lines[onig_package] = "|".join(fields)
elif mutation == "checksum":
    fields = lines[onig_package].split("|")
    fields[4] = ("1" if fields[4][0] != "1" else "2") + fields[4][1:]
    lines[onig_package] = "|".join(fields)
elif mutation == "root":
    lines[roots[0]] = lines[roots[0]].replace("root=ferric-build|", "root=ferric-engine|", 1)
elif mutation == "lock-checksum":
    text = lock.read_text(encoding="utf-8")
    old = "0cc3cbf698f9438986c11a880c90a6d04b9de27575afd28bbf45b154b6c709e2"
    if text.count(old) != 1:
        raise SystemExit("onig Cargo.lock checksum anchor drifted")
    lock.write_text(text.replace(old, "1" + old[1:]), encoding="utf-8")
else:
    raise SystemExit(f"unknown runtime TCB mutation: {mutation}")
if mutation != "lock-checksum":
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
    expect_rejected "runtime-tcb-$name" 'workspace runtime dependency TCB drifted' \
        "$source_gate" "$fixture" "$repo/proofs/VERIFIED_MODULES" \
        "$fixture_metadata"
}

for mutation in missing extra reordered duplicate version source checksum root lock-checksum; do
    runtime_tcb_hostile "$mutation" "$mutation"
done

python3 -I - "$metadata" "$scratch" <<'PY'
import copy
import json
from pathlib import Path
import sys

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
scratch = Path(sys.argv[2])


def package(name):
    return next(value for value in metadata["packages"] if value["name"] == name)


def node(name):
    identity = package(name)["id"]
    return next(value for value in metadata["resolve"]["nodes"] if value["id"] == identity)


feature = copy.deepcopy(metadata)
feature_node = next(
    value
    for value in feature["resolve"]["nodes"]
    if value["id"] == next(p for p in feature["packages"] if p["name"] == "onig")["id"]
)
feature_node["features"].append("hostile-feature")
(scratch / "runtime-feature.metadata").write_text(json.dumps(feature), encoding="utf-8")

build = copy.deepcopy(metadata)
build_package = next(p for p in build["packages"] if p["name"] == "onig_sys")
build_package["targets"] = [
    target for target in build_package["targets"] if "custom-build" not in target["kind"]
]
(scratch / "runtime-build-script.metadata").write_text(json.dumps(build), encoding="utf-8")

proc_macro = copy.deepcopy(metadata)
proc_package = next(p for p in proc_macro["packages"] if p["name"] == "onig")
proc_target = next(target for target in proc_package["targets"] if "lib" in target["kind"])
proc_target["kind"] = ["proc-macro"]
proc_target["crate_types"] = ["proc-macro"]
(scratch / "runtime-proc-macro.metadata").write_text(
    json.dumps(proc_macro), encoding="utf-8"
)

root = copy.deepcopy(metadata)
root_package = next(p for p in root["packages"] if p["name"] == "ferric-build")
hostile = copy.deepcopy(next(d for d in root_package["dependencies"] if d["name"] == "onig"))
hostile["name"] = "bitflags"
hostile["req"] = "=2.13.1"
root_package["dependencies"].append(hostile)
(scratch / "runtime-extra-root.metadata").write_text(json.dumps(root), encoding="utf-8")

dev_promoted = copy.deepcopy(metadata)
qwen_package = next(
    p for p in dev_promoted["packages"] if p["name"] == "ferric-qwen-kernels"
)
trybuild = next(d for d in qwen_package["dependencies"] if d["name"] == "trybuild")
if trybuild["kind"] != "dev":
    raise SystemExit("qwen trybuild dev-dependency anchor drifted")
trybuild["kind"] = None
(scratch / "runtime-dev-promoted.metadata").write_text(
    json.dumps(dev_promoted), encoding="utf-8"
)

fe2o3 = copy.deepcopy(metadata)
qwen_package = next(p for p in fe2o3["packages"] if p["name"] == "ferric-qwen-kernels")
compiler_dependency = next(
    d for d in qwen_package["dependencies"] if d["name"] == "fe2o3-compiler-ffi"
)
compiler_dependency["source"] = compiler_dependency["source"].replace(
    "dfc1eef4786383eb754e1f8770322d31db396cf9", "0" * 40
)
(scratch / "fe2o3-source.metadata").write_text(json.dumps(fe2o3), encoding="utf-8")

target = copy.deepcopy(metadata)
qwen_package = next(p for p in target["packages"] if p["name"] == "ferric-qwen-kernels")
test_target = next(t for t in qwen_package["targets"] if t["kind"] == ["test"])
test_target["kind"] = ["bin"]
test_target["crate_types"] = ["bin"]
(scratch / "non-library-target.metadata").write_text(json.dumps(target), encoding="utf-8")

binary_name = copy.deepcopy(metadata)
build_package = next(p for p in binary_name["packages"] if p["name"] == "ferric-build")
prepack_target = next(t for t in build_package["targets"] if t["kind"] == ["bin"])
prepack_target["name"] = "unadmitted-prepack"
(scratch / "binary-name.metadata").write_text(json.dumps(binary_name), encoding="utf-8")

binary_path = copy.deepcopy(metadata)
build_package = next(p for p in binary_path["packages"] if p["name"] == "ferric-build")
prepack_target = next(t for t in build_package["targets"] if t["kind"] == ["bin"])
prepack_target["src_path"] = next(
    t for t in build_package["targets"] if t["kind"] == ["lib"]
)["src_path"]
(scratch / "binary-path.metadata").write_text(json.dumps(binary_path), encoding="utf-8")

test_fixture_runtime = copy.deepcopy(metadata)
engine_package = next(
    p for p in test_fixture_runtime["packages"] if p["name"] == "ferric-engine"
)
runtime_build = next(
    d
    for d in engine_package["dependencies"]
    if d["name"] == "ferric-build" and d["kind"] is None
)
runtime_build["features"].append("test-fixtures")
(scratch / "test-fixture-runtime.metadata").write_text(
    json.dumps(test_fixture_runtime), encoding="utf-8"
)
PY

expect_rejected runtime-tcb-feature 'workspace runtime dependency TCB drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-feature.metadata"
expect_rejected runtime-tcb-build-script 'workspace runtime dependency TCB drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-build-script.metadata"
expect_rejected runtime-tcb-proc-macro 'workspace runtime dependency TCB drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-proc-macro.metadata"
expect_rejected runtime-tcb-extra-root 'unadmitted registry runtime root' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-extra-root.metadata"
expect_rejected runtime-dev-promoted 'unadmitted registry runtime root' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-dev-promoted.metadata"
expect_rejected fe2o3-source-drift 'workspace fe2o3 dependency declaration drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/fe2o3-source.metadata"
expect_rejected non-library-target 'unsupported non-library target' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/non-library-target.metadata"
expect_rejected binary-name-drift 'unsupported non-library target' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/binary-name.metadata"
expect_rejected binary-path-drift 'qualified binary source path drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/binary-path.metadata"
expect_rejected test-fixture-runtime-activation \
    'activates the test-fixtures feature outside its admitted dev edge' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/test-fixture-runtime.metadata"

cp "$repo/proofs/negative/REQUIRED_COMPONENTS" "$scratch/unsafe-target.registry"
chmod u+w "$scratch/unsafe-target.registry"
python3 -I - "$scratch/unsafe-target.registry" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
old = "|identity|RequestId::new"
if text.count(old) != 1:
    raise SystemExit("registry target fixture anchor drifted")
path.write_text(text.replace(old, "|identity*|RequestId::new"), encoding="utf-8")
PY
expect_rejected negative-registry-unsafe-target 'unsafe Verus module target' \
    python3 -I "$registry_checker" "$repo" "$scratch/unsafe-target.registry" \
    "$scratch/unsafe-target.active"

cp "$repo/proofs/negative/REQUIRED_COMPONENTS" "$scratch/duplicate.registry"
chmod u+w "$scratch/duplicate.registry"
sed -n '2p' "$repo/proofs/negative/REQUIRED_COMPONENTS" \
    >>"$scratch/duplicate.registry"
expect_rejected negative-registry-duplicate 'duplicate negative component' \
    python3 -I "$registry_checker" "$repo" "$scratch/duplicate.registry" \
    "$scratch/duplicate.active"

python3 -I - "$repo/proofs/negative/REQUIRED_COMPONENTS" \
    "$scratch/missing-target.registry" <<'PY'
from pathlib import Path
import sys

lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
fields = lines[1].split("|")
lines[1] = "|".join(fields[:-1])
Path(sys.argv[2]).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
expect_rejected negative-registry-missing-target 'malformed negative component record' \
    python3 -I "$registry_checker" "$repo" "$scratch/missing-target.registry" \
    "$scratch/missing-target.active"

identity_anchor=$(new_copy identity-trust-mutator-anchor)
python3 -I "$repo/proofs/negative/components/identity-trust.py" "$identity_anchor" \
    >"$scratch/identity-trust-mutator.marker"
grep -F 'MUTATED_SOURCE=crates/ferric-spec/src/identity.rs' \
    "$scratch/identity-trust-mutator.marker" >/dev/null || {
    printf 'FAIL: identity-trust mutator did not attest its exact source\n' >&2
    exit 1
}
python3 -I - "$identity_anchor/crates/ferric-spec/src/identity.rs" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8")
is_present = source.index("    pub fn is_present(&self)")
equals = source.index("    pub fn equals(&self, other: &Self)")
if source[is_present:equals].count("        assume(false);\n") != 1:
    raise SystemExit("identity-trust mutation did not land exactly in is_present")
if "assume(false)" in source[equals:]:
    raise SystemExit("identity-trust mutation escaped into equals or later source")
PY

python3 -I - "$repo/proofs/VERIFIED_MODULES" "$scratch/missing-record.manifest" <<'PY'
from pathlib import Path
import sys

lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
removed = False
output = []
for line in lines:
    if not removed and line.startswith(("module=", "verified=", "unverified=")):
        removed = True
        continue
    output.append(line)
if not removed:
    raise SystemExit("coverage manifest contains no removable record")
Path(sys.argv[2]).write_text("\n".join(output) + "\n", encoding="utf-8")
PY
expect_rejected coverage-missing-record 'compiler-rooted proof coverage manifest drifted' \
    "$source_gate" "$repo" "$scratch/missing-record.manifest" "$metadata"

sed '/^verified=ferric-engine|/d' "$repo/proofs/VERIFIED_MODULES" \
    >"$scratch/zero-direct.manifest"
: >"$scratch/empty-verus.transcript"
expect_rejected transcript-zero-direct-coverage 'has no admitted directly verified executable bodies' \
    python3 -I "$repo/proofs/check-transcript.py" ferric-engine ferric_engine \
    "$scratch/zero-direct.manifest" "$scratch/empty-verus.transcript" \
    "$scratch/zero-direct.counts" "$(sed -n '1p' "$repo/proofs/verus/VERUS_VERSION")"

# Complex Rust syntax must be classified structurally. A function pointer is a type,
# not an executable body, while generic free and trait-default functions are bodies.
complex=$(new_copy complex-syntax)
cat >>"$complex/crates/ferric-spec/src/configuration.rs" <<'RS'

pub type CallbackProbe = fn(u32) -> u32;

pub fn generic_probe<T, U, const N: usize>(value: T) -> Option<U>
where
    T: Into<U>,
    U: Copy,
{
    let _ = N;
    Some(value.into())
}

pub trait DefaultProbe {
    fn default_probe<T>(&self, callback: fn(T) -> T, value: T) -> T
    where
        T: Copy,
    {
        callback(value)
    }
}
RS
cat >>"$complex/proofs/UNVERIFIED_BODIES" <<'ADMISSIONS'
unverified=ferric-spec|crates/ferric-spec/src/configuration.rs|ferric_spec::configuration::DefaultProbe::default_probe|pending-verus|hostile-parser-generic-fixture
unverified=ferric-spec|crates/ferric-spec/src/configuration.rs|ferric_spec::configuration::generic_probe|pending-verus|hostile-parser-generic-fixture
ADMISSIONS
write_metadata "$complex" "$scratch/complex.metadata"
"$source_gate" --generate "$complex" "$scratch/complex.metadata" "$scratch/complex.manifest"
grep -F 'ferric_spec::configuration::generic_probe' "$scratch/complex.manifest" >/dev/null || {
    printf 'FAIL: generic executable body was not classified\n' >&2
    exit 1
}
grep -F 'ferric_spec::configuration::DefaultProbe::default_probe' \
    "$scratch/complex.manifest" >/dev/null || {
    printf 'FAIL: trait-default executable body was not classified\n' >&2
    exit 1
}
if grep -F 'CallbackProbe' "$scratch/complex.manifest" >/dev/null; then
    printf 'FAIL: function pointer type was misclassified as an executable body\n' >&2
    exit 1
fi
expect_rejected coverage-generic-drift 'compiler-rooted proof coverage manifest drifted' \
    "$source_gate" "$complex" "$repo/proofs/VERIFIED_MODULES" "$scratch/complex.metadata"

outside=$(new_copy unadmitted-outside-verus)
printf '\npub fn unadmitted_production_body() {}\n' \
    >>"$outside/crates/ferric-spec/src/configuration.rs"
write_metadata "$outside" "$scratch/outside.metadata"
expect_rejected parser-unadmitted-outside-verus 'unverified executable body admission drifted' \
    "$source_gate" --generate "$outside" "$scratch/outside.metadata" "$scratch/outside.manifest"

solver=$(new_copy solver-attributes)
cat >>"$solver/crates/ferric-spec/src/identity.rs" <<'RS'

verus! {
closed spec fn trigger_attribute_probe(values: Seq<u8>, position: int) -> bool {
    #[trigger] values[position] == values[position]
}

#[verifier::bit_vector]
proof fn bit_vector_attribute_probe(value: u8)
    ensures value & 0_u8 == 0_u8,
{}
}
RS
write_metadata "$solver" "$scratch/solver.metadata"
"$source_gate" "$solver" "$repo/proofs/VERIFIED_MODULES" "$scratch/solver.metadata"

constructor=$(new_copy admitted-allocation-constructor)
write_metadata "$constructor" "$scratch/constructor.metadata"
"$source_gate" --generate "$constructor" "$scratch/constructor.metadata" \
    "$scratch/constructor.manifest"
grep -F 'ferric_engine::cache::KvPool::new_bounded' \
    "$scratch/constructor.manifest" >/dev/null || {
    printf 'FAIL: exact allocation constructor was not classified\n' >&2
    exit 1
}

engine_constructor=$(new_copy admitted-engine-allocation-constructor)
write_metadata "$engine_constructor" "$scratch/engine-constructor.metadata"
"$source_gate" --generate "$engine_constructor" \
    "$scratch/engine-constructor.metadata" "$scratch/engine-constructor.manifest"
grep -F 'ferric_engine::system::Engine::new' \
    "$scratch/engine-constructor.manifest" >/dev/null || {
    printf 'FAIL: exact engine allocation constructor was not classified\n' >&2
    exit 1
}

engine_transition=$(new_copy rejected-engine-allocation-transition)
cat >>"$engine_transition/crates/ferric-engine/src/system.rs" <<'RS'
verus! {
impl Engine {
    pub fn transition() {
        let mut permits: Vec<Option<u8>> = Vec::with_capacity(1);
        permits.push(None);
    }
}
}
RS
write_metadata "$engine_transition" "$scratch/engine-transition.metadata"
expect_rejected parser-engine-transition-allocation \
    'verified engine body ferric_engine::system::Engine::transition violates no-transition-allocation policy' \
    "$source_gate" --generate "$engine_transition" \
    "$scratch/engine-transition.metadata" "$scratch/engine-transition.manifest"

ghost_allocation=$(new_copy admitted-engine-ghost-allocation)
cat >>"$ghost_allocation/crates/ferric-engine/src/system.rs" <<'RS'
verus! {
impl Engine {
    pub fn ghost_allocation_probe() {
        let ghost _erased_values: Vec<u8> = Vec::new();
        let ghost values = Seq::<u8>::empty();
        proof {
            let _extended = values.push(1);
        }
    }
}
}
RS
write_metadata "$ghost_allocation" "$scratch/ghost-allocation.metadata"
"$source_gate" --generate "$ghost_allocation" \
    "$scratch/ghost-allocation.metadata" "$scratch/ghost-allocation.manifest"
grep -F 'ferric_engine::system::Engine::ghost_allocation_probe' \
    "$scratch/ghost-allocation.manifest" >/dev/null || {
    printf 'FAIL: erased ghost-allocation probe was not classified\n' >&2
    exit 1
}

allocation=$(new_copy transition-allocation)
cat >>"$allocation/crates/ferric-engine/src/cache.rs" <<'RS'

verus! {
pub fn transition_allocation_probe() {
    let mut values: Vec<u8> = Vec::new();
    values.push(1);
    values.reserve(1);
    values.resize(2, 0);
    values.extend([3_u8]);
    let _cloned = values.clone();
    let _copied = values.to_vec();
    let _collected: Vec<u8> = values.collect();
    let _boxed: Box<u8> = Box::new(1);
    let _macro_values = vec![1_u8];
}
}
RS
write_metadata "$allocation" "$scratch/allocation.metadata"
expect_rejected parser-transition-allocation 'violates no-transition-allocation policy' \
    "$source_gate" --generate "$allocation" "$scratch/allocation.metadata" \
    "$scratch/allocation.manifest"
for marker in 'Vec::new' 'Box::new' 'method is forbidden: push' \
    'method is forbidden: reserve' 'method is forbidden: resize' \
    'method is forbidden: extend' 'method is forbidden: clone' \
    'method is forbidden: to_vec' 'method is forbidden: collect' \
    'vec! allocation is forbidden' 'Box type is forbidden'; do
    grep -F "$marker" "$output/parser-transition-allocation.transcript" >/dev/null || {
        printf 'FAIL: transition allocation fixture did not exercise %s\n' "$marker" >&2
        exit 1
    }
done

raw=$(new_copy raw-identifier)
printf '\npub fn r#match() {}\n' >>"$raw/crates/ferric-spec/src/configuration.rs"
write_metadata "$raw" "$scratch/raw.metadata"
expect_rejected parser-raw-identifier 'raw identifier is forbidden' \
    "$source_gate" --generate "$raw" "$scratch/raw.metadata" "$scratch/raw.manifest"

macro=$(new_copy item-macro)
cat >>"$macro/crates/ferric-spec/src/configuration.rs" <<'RS'

macro_rules! generated_body {
    () => { pub fn macro_generated_probe() {} };
}
generated_body!();
RS
write_metadata "$macro" "$scratch/macro.metadata"
expect_rejected parser-item-macro 'item macro invocation is forbidden' \
    "$source_gate" --generate "$macro" "$scratch/macro.metadata" "$scratch/macro.manifest"

orphan=$(new_copy orphan-source)
printf 'pub fn orphan_probe() {}\n' >"$orphan/crates/ferric-spec/src/orphan_probe.rs"
write_metadata "$orphan" "$scratch/orphan.metadata"
expect_rejected parser-orphan-source 'contains unreachable Rust source' \
    "$source_gate" --generate "$orphan" "$scratch/orphan.metadata" "$scratch/orphan.manifest"

conditional=$(new_copy conditional-source)
printf '\n#[cfg(any())]\npub fn conditionally_absent_probe() {}\n' \
    >>"$conditional/crates/ferric-spec/src/configuration.rs"
write_metadata "$conditional" "$scratch/conditional.metadata"
expect_rejected parser-cfg 'conditional source is forbidden' \
    "$source_gate" --generate "$conditional" "$scratch/conditional.metadata" \
    "$scratch/conditional.manifest"

deny=$(new_copy unsupported-deny)
sed -i 's/^#!\[deny(missing_docs)\]$/#![deny(dead_code)]/' \
    "$deny/crates/ferric-qwen-kernels/src/lib.rs"
write_metadata "$deny" "$scratch/deny.metadata"
expect_rejected parser-unsupported-deny 'unsupported deny attribute: dead_code' \
    "$source_gate" --generate "$deny" "$scratch/deny.metadata" "$scratch/deny.manifest"

repr=$(new_copy unsupported-repr)
sed -i '0,/^#\[repr(u8)\]$/s//#[repr(C)]/' \
    "$repr/crates/ferric-qwen-kernels/src/gemm.rs"
write_metadata "$repr" "$scratch/repr.metadata"
expect_rejected parser-unsupported-repr 'unsupported repr attribute: C' \
    "$source_gate" --generate "$repr" "$scratch/repr.metadata" "$scratch/repr.manifest"

assert_include=$(new_copy assert-source-inclusion)
cat >>"$assert_include/crates/ferric-spec/src/configuration.rs" <<'RS'

const _: () = {
    assert!(include_str!("configuration.rs").is_empty());
};
RS
write_metadata "$assert_include" "$scratch/assert-include.metadata"
expect_rejected parser-assert-source-inclusion 'source inclusion macro is forbidden: include_str!' \
    "$source_gate" --generate "$assert_include" "$scratch/assert-include.metadata" \
    "$scratch/assert-include.manifest"

trust=$(new_copy trust-attribute)
cat >>"$trust/crates/ferric-spec/src/configuration.rs" <<'RS'

#[verus_verify(external_body)]
pub fn alternate_trust_attribute_probe() {}
RS
write_metadata "$trust" "$scratch/trust.metadata"
expect_rejected parser-alternate-trust-attribute 'trust-expanding or unsupported verifier attribute: verus_verify' \
    "$source_gate" --generate "$trust" "$scratch/trust.metadata" "$scratch/trust.manifest"

assume=$(new_copy qualified-assume)
cat >>"$assume/crates/ferric-spec/src/identity.rs" <<'RS'

verus! {
pub fn qualified_assume_probe() {
    vstd::pervasive::assume(false);
}
}
RS
write_metadata "$assume" "$scratch/assume.metadata"
expect_rejected parser-qualified-assume 'forbidden trust call' \
    "$source_gate" --generate "$assume" "$scratch/assume.metadata" "$scratch/assume.manifest"

cat >"$scratch/trust-call-method.rs" <<'RS'
verus! {
fn admit(&self) {}

fn production_method_call(&self) {
    self.scheduler.admit();
}
}
RS
python3 -I "$repo/proofs/check-source.py" --verus-blocks \
    "$scratch/trust-call-method.rs"

cat >"$scratch/trust-call-bare.rs" <<'RS'
verus! {
proof fn bare_trust_call() {
    assume(false);
}
}
RS
expect_rejected scanner-bare-trust-call 'forbidden trust call' \
    python3 -I "$repo/proofs/check-source.py" --verus-blocks \
    "$scratch/trust-call-bare.rs"

cat >"$scratch/trust-call-qualified.rs" <<'RS'
verus! {
proof fn qualified_trust_call() {
    vstd::pervasive::assume(false);
}
}
RS
expect_rejected scanner-qualified-trust-call 'forbidden trust call' \
    python3 -I "$repo/proofs/check-source.py" --verus-blocks \
    "$scratch/trust-call-qualified.rs"

optout=$(new_copy dependency-opt-out)
python3 -I - "$optout/crates/ferric-spec/Cargo.toml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
source = path.read_text(encoding="utf-8")
needle = "[package.metadata.verus]\nverify = true"
if source.count(needle) != 1:
    raise SystemExit("dependency opt-out anchor drifted")
path.write_text(source.replace(needle, "[package.metadata.verus]\nverify = false"), encoding="utf-8")
PY
write_metadata "$optout" "$scratch/optout.metadata"
expect_rejected dependency-opt-out 'first-party workspace package is not opted into strict Verus' \
    "$source_gate" --generate "$optout" "$scratch/optout.metadata" "$scratch/optout.manifest"

printf 'PASS: compiler-rooted admission rejects stale, unparsed, conditional, trust-expanded, and opted-out source\n'
