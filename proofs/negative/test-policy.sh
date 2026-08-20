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
    cp -a "$repo/crates" "$destination/"
    mkdir -p "$destination/proofs"
    cp -a "$repo/proofs/UNVERIFIED_BODIES" "$destination/proofs/"
    chmod -R u+w "$destination"
    printf '%s\n' "$destination"
}

write_metadata() {
    source_repo=$1
    destination=$2
    (
        cd "$source_repo"
        cargo metadata --locked --no-deps --format-version 1
    ) >"$destination"
}

cp "$repo/proofs/negative/REQUIRED_COMPONENTS" "$scratch/unsafe-target.registry"
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
