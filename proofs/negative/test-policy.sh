#!/bin/sh
set -eu

usage() {
    printf 'usage: %s REPO METADATA_JSON VERIFIER_METADATA_JSON OUTPUT_DIR SOURCE_GATE\n' "$0" >&2
    exit 2
}

[ "$#" -eq 5 ] || usage
repo=$(CDPATH='' cd -- "$1" && pwd)
metadata=$2
verifier_metadata=$3
output=$4
source_gate=$5
[ -f "$metadata" ] || {
    printf 'FAIL: Cargo metadata is unavailable: %s\n' "$metadata" >&2
    exit 1
}
[ -f "$verifier_metadata" ] || {
    printf 'FAIL: standalone verifier Cargo metadata is unavailable: %s\n' \
        "$verifier_metadata" >&2
    exit 1
}
[ -x "$source_gate" ] || {
    printf 'FAIL: source gate is unavailable: %s\n' "$source_gate" >&2
    exit 1
}
mkdir -p "$output"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ferric-policy-negative.XXXXXX")
trap 'chmod -R u+w "$scratch" 2>/dev/null || true; rm -rf "$scratch"' EXIT HUP INT TERM

verifier_metadata_for() {
    candidate_repo=$(CDPATH='' cd -- "$1" && pwd)
    if [ "$candidate_repo" = "$repo" ]; then
        printf '%s\n' "$verifier_metadata"
        return
    fi
    key=$(printf '%s' "$candidate_repo" | sha256sum | awk '{ print $1 }')
    candidate_metadata="$scratch/verifier-$key.metadata"
    if [ ! -f "$candidate_metadata" ]; then
        (
            cd "$candidate_repo"
            cargo metadata \
                --manifest-path adapters/qwen3-all-kernels-worker-v3-verifier-v1/Cargo.toml \
                --locked --all-features --format-version 1
        ) >"$candidate_metadata"
    fi
    printf '%s\n' "$candidate_metadata"
}

invoke_source_gate() {
    case "$1" in
        --generate|--unverified-inventory|--runtime-dependency-tcb)
            candidate_repo=$2
            candidate_verifier_metadata=$(verifier_metadata_for "$candidate_repo")
            "$source_gate" "$1" "$2" "$3" "$candidate_verifier_metadata" "$4"
            ;;
        *)
            candidate_repo=$1
            candidate_verifier_metadata=$(verifier_metadata_for "$candidate_repo")
            "$source_gate" "$1" "$2" "$3" "$candidate_verifier_metadata"
            ;;
    esac
}
registry_checker="$repo/proofs/negative/check-registry.py"
[ -f "$registry_checker" ] || {
    printf 'FAIL: negative registry checker is unavailable\n' >&2
    exit 1
}
mutation_checker="$repo/proofs/negative/check-mutation.py"
[ -f "$mutation_checker" ] || {
    printf 'FAIL: negative mutation checker is unavailable\n' >&2
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
    cp -a "$repo/adapters" "$destination/"
    cp -a "$repo/benches" "$destination/"
    cp -a "$repo/crates" "$destination/"
    cp -a "$repo/device" "$destination/"
    mkdir -p "$destination/proofs"
    cp -a "$repo/proofs/m1" "$destination/proofs/"
    cp -a "$repo/proofs/UNVERIFIED_BODIES" \
        "$repo/proofs/RUNTIME_DEPENDENCY_TCB" "$destination/proofs/"
    mkdir -p "$destination/proofs/source-gate"
    cp -a "$repo/proofs/source-gate/VERIFIER_PRODUCTION_DEPENDENCY_TCB" \
        "$repo/proofs/source-gate/VERIFIER_DEV_DEPENDENCY_TCB" \
        "$destination/proofs/source-gate/"
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
        invoke_source_gate "$fixture" "$repo/proofs/VERIFIED_MODULES" \
        "$fixture_metadata"
}

for mutation in missing extra reordered duplicate version source checksum root lock-checksum; do
    runtime_tcb_hostile "$mutation" "$mutation"
done

for scope in PRODUCTION DEV; do
    lower=$(printf '%s' "$scope" | tr 'A-Z' 'a-z')
    fixture=$(new_copy "verifier-$lower-tcb-missing")
    fixture_metadata="$scratch/verifier-$lower-tcb-missing.metadata"
    write_metadata "$fixture" "$fixture_metadata"
    tcb="$fixture/proofs/source-gate/VERIFIER_${scope}_DEPENDENCY_TCB"
    awk 'BEGIN { removed = 0 } /^package=/ && !removed { removed = 1; next } { print }' \
        "$tcb" >"$tcb.mutated"
    cp "$tcb.mutated" "$tcb"
    rm "$tcb.mutated"
    expect_rejected "verifier-$lower-tcb-missing" \
        'verifier dependency TCB drifted' \
        invoke_source_gate "$fixture" "$repo/proofs/VERIFIED_MODULES" \
        "$fixture_metadata"
done

python3 -I - "$verifier_metadata" "$scratch" <<'PY'
import copy
import json
from pathlib import Path
import sys

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
scratch = Path(sys.argv[2])
root = metadata["resolve"]["root"]

feature = copy.deepcopy(metadata)
next(node for node in feature["resolve"]["nodes"] if node["id"] == root)["features"] = ["escape"]
(scratch / "verifier-dev-feature.metadata").write_text(json.dumps(feature), encoding="utf-8")

target = copy.deepcopy(metadata)
package = next(package for package in target["packages"] if package["id"] != root)
package["targets"][0]["doc"] = not package["targets"][0]["doc"]
(scratch / "verifier-dev-target.metadata").write_text(json.dumps(target), encoding="utf-8")

edge = copy.deepcopy(metadata)
node = next(node for node in edge["resolve"]["nodes"] if node["id"] != root and node["deps"])
node["deps"][0]["dep_kinds"][0]["target"] = "cfg(ferric_escape)"
(scratch / "verifier-dev-edge.metadata").write_text(json.dumps(edge), encoding="utf-8")

unreachable = copy.deepcopy(metadata)
extra = copy.deepcopy(unreachable["packages"][0])
extra["id"] = "registry+https://example.invalid#index@1.0.0"
extra["name"] = "unreachable-index"
extra["version"] = "1.0.0"
extra["source"] = "registry+https://example.invalid"
unreachable["packages"].append(extra)
(scratch / "verifier-dev-unreachable.metadata").write_text(
    json.dumps(unreachable), encoding="utf-8"
)
PY

expect_rejected verifier-dev-feature 'protected verifier resolved features drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" "$metadata" \
    "$scratch/verifier-dev-feature.metadata"
expect_rejected verifier-dev-target 'verifier dependency TCB drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" "$metadata" \
    "$scratch/verifier-dev-target.metadata"
expect_rejected verifier-dev-edge 'verifier resolved edge has no matching declaration' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" "$metadata" \
    "$scratch/verifier-dev-edge.metadata"
expect_rejected verifier-dev-unreachable \
    'standalone verifier metadata and Cargo.lock package rosters drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" "$metadata" \
    "$scratch/verifier-dev-unreachable.metadata"

python3 -I - "$metadata" "$scratch" <<'PY'
import copy
import json
from pathlib import Path
import sys

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
scratch = Path(sys.argv[2])


def package(document, name):
    matches = [value for value in document["packages"] if value["name"] == name]
    if len(matches) != 1:
        raise SystemExit(f"package fixture anchor drifted: {name}")
    return matches[0]


def node(document, name):
    identity = package(document, name)["id"]
    matches = [value for value in document["resolve"]["nodes"] if value["id"] == identity]
    if len(matches) != 1:
        raise SystemExit(f"resolve fixture anchor drifted: {name}")
    return matches[0]


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
    "b2cce9c271e85a97c35ce7a1ccffe17bb330f07c", "0" * 40
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

device_aggregate = "ferric-qwen3-all-kernels-device-v1"
device_compatibility = "ferric-qwen3-gemm-device-v1"
compatibility_crate = "ferric_qwen3_gemm_device_v1"


def dependency(document, owner, name):
    matches = [
        value
        for value in package(document, owner)["dependencies"]
        if value["name"] == name and value["kind"] is None
    ]
    if len(matches) != 1:
        raise SystemExit(f"dependency fixture anchor drifted: {owner}::{name}")
    return matches[0]


def write_hostile(name, document):
    (scratch / f"local-runtime-{name}.metadata").write_text(
        json.dumps(document), encoding="utf-8"
    )


def add_compatibility_package(document, include_resolve=False):
    aggregate = package(document, device_aggregate)
    aggregate_root = Path(aggregate["manifest_path"]).parent
    compatibility_root = aggregate_root.parent / "qwen3-gemm-v1"
    compatibility = copy.deepcopy(aggregate)
    compatibility_id = (
        f"path+file://{compatibility_root}#{device_compatibility}@0.1.0"
    )
    compatibility["name"] = device_compatibility
    compatibility["id"] = compatibility_id
    compatibility["manifest_path"] = str(compatibility_root / "Cargo.toml")
    for target in compatibility["targets"]:
        source = Path(target["src_path"])
        target["src_path"] = str(compatibility_root / source.relative_to(aggregate_root))
        if target["kind"] == ["lib"]:
            target["name"] = compatibility_crate
    document["packages"].append(compatibility)
    if include_resolve:
        aggregate_node = next(
            value
            for value in document["resolve"]["nodes"]
            if value["id"] == aggregate["id"]
        )
        compatibility_node = copy.deepcopy(aggregate_node)
        compatibility_node["id"] = compatibility_id
        document["resolve"]["nodes"].append(compatibility_node)
    return compatibility, compatibility_root


local_missing = copy.deepcopy(metadata)
engine_dependencies = package(local_missing, "ferric-engine")["dependencies"]
aggregate_dependency = dependency(
    local_missing, "ferric-engine", device_aggregate
)
engine_dependencies.remove(aggregate_dependency)
write_hostile("missing", local_missing)

local_extra = copy.deepcopy(metadata)
_, compatibility_root = add_compatibility_package(local_extra)
hostile_dependency = copy.deepcopy(
    dependency(local_extra, "ferric-engine", device_aggregate)
)
hostile_dependency["name"] = device_compatibility
hostile_dependency["path"] = str(compatibility_root)
package(local_extra, "ferric-engine")["dependencies"].append(hostile_dependency)
write_hostile("extra", local_extra)

local_wrong_owner = copy.deepcopy(metadata)
wrong_owner_dependency = copy.deepcopy(
    dependency(local_wrong_owner, "ferric-engine", device_aggregate)
)
package(local_wrong_owner, "ferric-qwen-kernels")["dependencies"].append(
    wrong_owner_dependency
)
write_hostile("wrong-owner", local_wrong_owner)

local_path = copy.deepcopy(metadata)
aggregate_root = Path(
    dependency(local_path, "ferric-engine", device_aggregate)["path"]
)
dependency(local_path, "ferric-engine", device_aggregate)["path"] = str(
    aggregate_root.parent / "qwen3-gemm-v1"
)
write_hostile("path", local_path)

local_workspace = copy.deepcopy(metadata)
local_workspace["workspace_members"].append(
    package(local_workspace, device_aggregate)["id"]
)
write_hostile("workspace", local_workspace)

local_verus = copy.deepcopy(metadata)
package(local_verus, device_aggregate)["metadata"] = {
    "verus": {"verify": True}
}
write_hostile("verus", local_verus)

local_manifest = copy.deepcopy(metadata)
aggregate_manifest = Path(
    package(local_manifest, device_aggregate)["manifest_path"]
)
package(local_manifest, device_aggregate)["manifest_path"] = str(
    aggregate_manifest.parent.parent / "qwen3-gemm-v1" / "Cargo.toml"
)
write_hostile("manifest", local_manifest)

local_target = copy.deepcopy(metadata)
aggregate_library = next(
    value
    for value in package(local_target, device_aggregate)["targets"]
    if value["kind"] == ["lib"]
)
aggregate_library["name"] = f"{aggregate_library['name']}_hostile"
write_hostile("target", local_target)

local_fe2o3 = copy.deepcopy(metadata)
device_dependency = dependency(local_fe2o3, device_aggregate, "fe2o3-device")
device_dependency["source"] = device_dependency["source"].replace(
    "b2cce9c271e85a97c35ce7a1ccffe17bb330f07c", "0" * 40
)
write_hostile("fe2o3", local_fe2o3)

local_resolve = copy.deepcopy(metadata)
aggregate_id = package(local_resolve, device_aggregate)["id"]
compatibility, _ = add_compatibility_package(local_resolve, include_resolve=True)
resolved_aggregate = [
    value
    for value in node(local_resolve, "ferric-engine")["deps"]
    if value["pkg"] == aggregate_id
]
if len(resolved_aggregate) != 1:
    raise SystemExit("local runtime resolve edge fixture anchor drifted")
resolved_aggregate[0]["pkg"] = compatibility["id"]
owner_dependencies = node(local_resolve, "ferric-engine")["dependencies"]
if owner_dependencies.count(aggregate_id) != 1:
    raise SystemExit("local runtime resolve dependency fixture anchor drifted")
owner_dependencies[owner_dependencies.index(aggregate_id)] = compatibility["id"]
write_hostile("resolve", local_resolve)
PY

python3 -I - "$metadata" "$scratch" <<'PY'
import copy
import json
from pathlib import Path
import sys

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
scratch = Path(sys.argv[2])
packages = {package["id"]: package for package in metadata["packages"]}
nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
root = next(
    package["id"]
    for package in metadata["packages"]
    if package["name"] == "ferric-qwen3-all-kernels-worker-v3-verifier-v1"
)

def edge_kinds(edge):
    return [(kind["kind"] or "normal", kind["target"]) for kind in edge["dep_kinds"]]

reachable = set()
stack = [root]
while stack:
    identity = stack.pop()
    if identity in reachable:
        continue
    reachable.add(identity)
    for edge in nodes[identity]["deps"]:
        if any(kind != "dev" for kind, _ in edge_kinds(edge)):
            stack.append(edge["pkg"])

selected = None
for identity in sorted(reachable):
    package = packages[identity]
    for edge_index, edge in enumerate(nodes[identity]["deps"]):
        for kind_index, (kind, target) in enumerate(edge_kinds(edge)):
            if kind != "normal":
                continue
            for declaration_index, declaration in enumerate(package["dependencies"]):
                declared_name = (declaration["rename"] or declaration["name"]).replace("-", "_")
                declared_kind = declaration["kind"] or "normal"
                if (declared_name, declared_kind, declaration["target"]) == (
                    edge["name"], kind, target
                ):
                    selected = (identity, edge_index, kind_index, declaration_index)
                    break
            if selected:
                break
        if selected:
            break
    if selected:
        break
if selected is None:
    raise SystemExit("production verifier edge/declaration fixture anchor drifted")
identity, edge_index, kind_index, declaration_index = selected

package = next(package for package in metadata["packages"] if package["name"] == "curve25519-dalek")
package["targets"][0]["doc"] = not package["targets"][0]["doc"]
(scratch / "verifier-production-target.metadata").write_text(
    json.dumps(metadata), encoding="utf-8"
)

empty = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
next(node for node in empty["resolve"]["nodes"] if node["id"] == identity)["deps"][edge_index][
    "dep_kinds"
] = []
(scratch / "verifier-production-empty-kind.metadata").write_text(
    json.dumps(empty), encoding="utf-8"
)

reclassified = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
next(node for node in reclassified["resolve"]["nodes"] if node["id"] == identity)["deps"][
    edge_index
]["dep_kinds"][kind_index]["kind"] = "dev"
next(package for package in reclassified["packages"] if package["id"] == identity)[
    "dependencies"
][declaration_index]["kind"] = "dev"
(scratch / "verifier-production-reclassified.metadata").write_text(
    json.dumps(reclassified), encoding="utf-8"
)

extra_dev = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
extra_packages = {package["id"]: package for package in extra_dev["packages"]}
owner = extra_packages[identity]
owner_node = next(node for node in extra_dev["resolve"]["nodes"] if node["id"] == identity)
existing_ids = {edge["pkg"] for edge in owner_node["deps"]}
extra = next(
    package
    for package in extra_dev["packages"]
    if package["source"] == "registry+https://github.com/rust-lang/crates.io-index"
    and package["id"] not in existing_ids
    and package["id"] != identity
)
edge_name = extra["name"].replace("-", "_") + "_filtered_probe"
owner["dependencies"].append(
    {
        "name": extra["name"],
        "source": extra["source"],
        "req": "=" + extra["version"],
        "kind": "dev",
        "rename": edge_name,
        "optional": False,
        "uses_default_features": True,
        "features": [],
        "target": None,
        "registry": None,
    }
)
owner_node["deps"].append(
    {
        "name": edge_name,
        "pkg": extra["id"],
        "dep_kinds": [{"kind": "dev", "target": None}],
    }
)
owner_node["dependencies"].append(extra["id"])
(scratch / "verifier-production-extra-dev.metadata").write_text(
    json.dumps(extra_dev), encoding="utf-8"
)

workspace_root = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
workspace_root["workspace_root"] = str(Path(workspace_root["workspace_root"]) / "adapters")
(scratch / "verifier-production-workspace-root.metadata").write_text(
    json.dumps(workspace_root), encoding="utf-8"
)

remote_manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
remote_package = next(
    package for package in remote_manifest["packages"] if package["name"] == "curve25519-dalek"
)
remote_package["manifest_path"] = str(
    Path(remote_manifest["workspace_root"])
    / "adapters/qwen3-all-kernels-worker-v3-verifier-v1/Cargo.toml"
)
(scratch / "verifier-production-remote-manifest.metadata").write_text(
    json.dumps(remote_manifest), encoding="utf-8"
)
PY

expect_rejected verifier-production-target 'verifier dependency TCB drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/verifier-production-target.metadata" "$verifier_metadata"
expect_rejected verifier-production-empty-kind 'verifier dependency edge has no kind' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/verifier-production-empty-kind.metadata" "$verifier_metadata"
expect_rejected verifier-production-reclassified 'verifier dependency TCB drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/verifier-production-reclassified.metadata" "$verifier_metadata"
expect_rejected verifier-production-extra-dev 'verifier dependency TCB drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/verifier-production-extra-dev.metadata" "$verifier_metadata"
expect_rejected verifier-production-workspace-root \
    'verifier Cargo metadata workspace root drifted' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/verifier-production-workspace-root.metadata" "$verifier_metadata"
expect_rejected verifier-production-remote-manifest \
    'verifier dependency target escapes its package root' \
    "$source_gate" "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/verifier-production-remote-manifest.metadata" "$verifier_metadata"

expect_rejected runtime-tcb-feature 'workspace runtime dependency TCB drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-feature.metadata"
expect_rejected runtime-tcb-build-script 'workspace runtime dependency TCB drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-build-script.metadata"
expect_rejected runtime-tcb-proc-macro 'workspace runtime dependency TCB drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-proc-macro.metadata"
expect_rejected runtime-tcb-extra-root 'unadmitted registry runtime root' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-extra-root.metadata"
expect_rejected runtime-dev-promoted 'unadmitted registry runtime root' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/runtime-dev-promoted.metadata"
expect_rejected fe2o3-source-drift 'workspace fe2o3 dependency declaration drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/fe2o3-source.metadata"
expect_rejected non-library-target 'unsupported non-library target' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/non-library-target.metadata"
expect_rejected binary-name-drift 'unsupported non-library target' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/binary-name.metadata"
expect_rejected binary-path-drift 'qualified binary source path drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/binary-path.metadata"
expect_rejected test-fixture-runtime-activation \
    'activates the test-fixtures feature outside its admitted dev edge' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/test-fixture-runtime.metadata"
expect_rejected local-runtime-missing 'local runtime roots drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-missing.metadata"
expect_rejected local-runtime-extra 'unadmitted path dependency' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-extra.metadata"
expect_rejected local-runtime-wrong-owner 'unadmitted path dependency' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-wrong-owner.metadata"
expect_rejected local-runtime-path 'local runtime dependency declaration drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-path.metadata"
expect_rejected local-runtime-workspace \
    'local runtime package may not become a workspace member' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-workspace.metadata"
expect_rejected local-runtime-verus 'local runtime package may not claim Verus authority' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-verus.metadata"
expect_rejected local-runtime-manifest 'protected verifier resolved dependency identity drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-manifest.metadata"
expect_rejected local-runtime-target 'verifier dependency TCB drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-target.metadata"
expect_rejected local-runtime-fe2o3 'verifier dependency TCB drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-fe2o3.metadata"
expect_rejected local-runtime-resolve 'local runtime owner resolve edge drifted' \
    invoke_source_gate "$repo" "$repo/proofs/VERIFIED_MODULES" \
    "$scratch/local-runtime-resolve.metadata"

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

mutation_escape=$(new_copy mutation-closure-escape)
python3 -I "$repo/proofs/negative/components/identity-trust.py" "$mutation_escape" \
    >"$scratch/mutation-closure-escape.marker"
printf '\n// hostile second mutation\n' \
    >>"$mutation_escape/crates/ferric-spec/src/configuration.rs"
expect_rejected mutation-closure-escape \
    'mutator changed source outside its exact attestation' \
    python3 -I "$mutation_checker" "$repo" "$mutation_escape" \
    "$scratch/mutation-closure-escape.marker" ferric-spec

device_escape=$(new_copy mutation-device-closure-escape)
python3 -I "$repo/proofs/negative/components/identity-trust.py" "$device_escape" \
    >"$scratch/mutation-device-closure-escape.marker"
printf '\n// hostile device mutation\n' \
    >>"$device_escape/device/qwen3-all-kernels-v1/src/gemm.rs"
expect_rejected mutation-device-closure-escape \
    'mutator changed source outside its exact attestation' \
    python3 -I "$mutation_checker" "$repo" "$device_escape" \
    "$scratch/mutation-device-closure-escape.marker" ferric-spec

lane_fixture=$(new_copy negative-cache-lanes)
mkdir -p "$lane_fixture/proofs/negative/components" "$scratch/fake-bin" \
    "$scratch/fake-verus" "$scratch/fake-state"
cp "$registry_checker" "$mutation_checker" "$lane_fixture/proofs/negative/"
cat >"$lane_fixture/proofs/negative/REQUIRED_COMPONENTS" <<'REGISTRY'
format=FERRIC-NEGATIVE-COMPONENTS-V2
always=engine-one|ferric-engine|engine-one.py|proof|system|Engine::admit
always=spec-one|ferric-spec|spec-one.py|proof|identity|RequestId::new
always=spec-two|ferric-spec|spec-two.py|proof|identity|RequestId::new
REGISTRY
cat >"$lane_fixture/proofs/negative/components/engine-one.py" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1]) / "crates/ferric-engine/src/system.rs"
path.write_text(path.read_text(encoding="utf-8") + "\n// engine mutation\n", encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
PY
cat >"$lane_fixture/proofs/negative/components/spec-one.py" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1]) / "crates/ferric-spec/src/configuration.rs"
path.write_text(path.read_text(encoding="utf-8") + "\n// spec mutation one\n", encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/configuration.rs")
PY
cat >"$lane_fixture/proofs/negative/components/spec-two.py" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1]) / "crates/ferric-spec/src/configuration.rs"
path.write_text(path.read_text(encoding="utf-8") + "\n// spec mutation two\n", encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/configuration.rs")
PY
cat >"$scratch/fake-bin/cargo" <<'SH'
#!/bin/sh
set -eu
package=
target=
manifest=
previous=
for argument in "$@"; do
    case "$previous" in
        -p) package=$argument ;;
        --target-dir) target=$argument ;;
        --manifest-path) manifest=$argument ;;
    esac
    previous=$argument
done
[ -n "$package" ] && [ -n "$target" ] && [ -n "$manifest" ] || exit 8
printf 'cargo|%s|%s|%s|%s\n' "$PWD" "$package" "$target" "$manifest" \
    >>"$FERRIC_FAKE_TOOL_LOG"
workspace=${manifest%/Cargo.toml}
[ -f "$workspace/adapters/qwen3-all-kernels-worker-v3-verifier-v1/Cargo.toml" ] || exit 8
for device in qwen3-all-kernels-v1 qwen3-gemm-v1 qwen3-logits-v1 qwen3-paged-decode-v1 qwen3-prefill-v1 \
    qwen3-rmsnorm-v1 qwen3-rope-kv-v1 qwen3-swiglu-v1; do
    [ -f "$workspace/device/$device/Cargo.toml" ] || exit 8
done
if [ -d "$target" ] && [ ! -f "$target/CACHEDIR.TAG" ]; then
    exit 8
fi
if [ "${FERRIC_FAKE_FAIL_POST_CLEAN:-false}" = true ]; then
    flag="$FERRIC_FAKE_TOOL_STATE/$package.pre-clean-seen"
    if [ -f "$flag" ]; then
        exit 9
    fi
    : >"$flag"
fi
SH
cat >"$scratch/fake-verus/cargo-verus" <<'SH'
#!/bin/sh
set -eu
package=
target=
previous=
for argument in "$@"; do
    case "$previous" in
        -p) package=$argument ;;
        --target-dir) target=$argument ;;
    esac
    previous=$argument
done
printf 'verus|%s|%s|%s\n' "$PWD" "$package" "$target" >>"$FERRIC_FAKE_TOOL_LOG"
mkdir -p "$target"
printf 'Signature: 8a477f597d28d172789f06886806bc55\n' >"$target/CACHEDIR.TAG"
printf 'Compiling %s v0.0.0\n' "$package"
printf 'verification results:: 0 verified, 1 errors\n'
exit "${FERRIC_FAKE_VERUS_STATUS:-101}"
SH
cat >"$scratch/fake-verus/z3" <<'SH'
#!/bin/sh
exit 0
SH
chmod +x "$scratch/fake-bin/cargo" "$scratch/fake-verus/cargo-verus" \
    "$scratch/fake-verus/z3"

: >"$scratch/fake-tools.log"
PATH="$scratch/fake-bin:$PATH" \
FERRIC_FAKE_TOOL_LOG="$scratch/fake-tools.log" \
FERRIC_FAKE_TOOL_STATE="$scratch/fake-state" \
FERRIC_NEGATIVE_TIMEOUT_SECONDS=30 FERRIC_NEGATIVE_JOBS=2 \
    "$repo/proofs/negative/run-same-source.sh" \
    "$lane_fixture" "$scratch/fake-verus" "$scratch/cache-lane-output" \
    >"$scratch/cache-lane.stdout"
python3 -I - "$scratch/fake-tools.log" <<'PY'
from pathlib import Path
import sys

lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
clean = [line.split("|", 4) for line in lines if line.startswith("cargo|")]
verus = [line.split("|", 3) for line in lines if line.startswith("verus|")]
if len(clean) != 6:
    raise SystemExit(f"negative cache lanes performed {len(clean)} cleans, expected 6")
if len(verus) != 3:
    raise SystemExit(f"negative cache lanes invoked {len(verus)} roots, expected 3")
spec = [fields for fields in verus if fields[2] == "ferric-spec"]
engine = [fields for fields in verus if fields[2] == "ferric-engine"]
if len(spec) != 2 or len(engine) != 1:
    raise SystemExit("negative cache lanes did not preserve package assignment")
if len({(fields[1], fields[3]) for fields in spec}) != 1:
    raise SystemExit("ferric-spec mutations did not reuse one stable isolated lane")
if (engine[0][1], engine[0][3]) == (spec[0][1], spec[0][3]):
    raise SystemExit("negative package cache lanes were not isolated")
for package, expected in (("ferric-spec", 4), ("ferric-engine", 2)):
    actual = sum(fields[2] == package for fields in clean)
    if actual != expected:
        raise SystemExit(f"{package} performed {actual} package cleans, expected {expected}")
copies_by_target = {
    Path(fields[3]): (Path(fields[1]), fields[2])
    for fields in verus
}
for _kind, pwd, package, target, manifest in clean:
    source = Path(manifest).parent
    expected = copies_by_target.get(Path(target))
    if expected != (source, package) or Path(pwd) != source:
        raise SystemExit("package clean escaped its isolated mutation source and target lane")
PY

cat >"$lane_fixture/proofs/negative/REQUIRED_COMPONENTS" <<'REGISTRY'
format=FERRIC-NEGATIVE-COMPONENTS-V2
always=spec-one|ferric-spec|spec-one.py|proof|identity|RequestId::new
REGISTRY
for hostile_status in 1 137; do
    : >"$scratch/fake-tools-status-$hostile_status.log"
    set +e
    PATH="$scratch/fake-bin:$PATH" \
    FERRIC_FAKE_TOOL_LOG="$scratch/fake-tools-status-$hostile_status.log" \
    FERRIC_FAKE_TOOL_STATE="$scratch/fake-state" \
    FERRIC_FAKE_VERUS_STATUS="$hostile_status" \
    FERRIC_NEGATIVE_TIMEOUT_SECONDS=30 FERRIC_NEGATIVE_JOBS=1 \
        "$repo/proofs/negative/run-same-source.sh" \
        "$lane_fixture" "$scratch/fake-verus" \
        "$scratch/cache-lane-status-$hostile_status-output" \
        >"$scratch/cache-lane-status-$hostile_status.stdout" \
        2>"$scratch/cache-lane-status-$hostile_status.stderr"
    lane_status=$?
    set -e
    [ "$lane_status" -ne 0 ] && \
        grep -F "mutation returned unexpected status $hostile_status" \
            "$scratch/cache-lane-status-$hostile_status.stderr" >/dev/null || {
        printf 'FAIL: negative cache lane accepted hostile status %s\n' \
            "$hostile_status" >&2
        exit 1
    }
done

cat >"$lane_fixture/proofs/negative/REQUIRED_COMPONENTS" <<'REGISTRY'
format=FERRIC-NEGATIVE-COMPONENTS-V2
always=spec-one|ferric-spec|spec-one.py|proof|identity|RequestId::new
always=spec-two|ferric-spec|spec-two.py|proof|identity|RequestId::new
REGISTRY
rm -rf "$scratch/fake-state"
mkdir -p "$scratch/fake-state"
: >"$scratch/fake-tools-failure.log"
set +e
PATH="$scratch/fake-bin:$PATH" \
FERRIC_FAKE_TOOL_LOG="$scratch/fake-tools-failure.log" \
FERRIC_FAKE_TOOL_STATE="$scratch/fake-state" \
FERRIC_FAKE_FAIL_POST_CLEAN=true \
FERRIC_NEGATIVE_TIMEOUT_SECONDS=30 FERRIC_NEGATIVE_JOBS=1 \
    "$repo/proofs/negative/run-same-source.sh" \
    "$lane_fixture" "$scratch/fake-verus" "$scratch/cache-lane-failure-output" \
    >"$scratch/cache-lane-failure.stdout" 2>"$scratch/cache-lane-failure.stderr"
lane_failure_status=$?
set -e
[ "$lane_failure_status" -ne 0 ] || {
    printf 'FAIL: negative cache lane accepted a failed post-clean\n' >&2
    exit 1
}
[ "$(grep -c '^verus|' "$scratch/fake-tools-failure.log")" -eq 1 ] || {
    printf 'FAIL: negative cache lane continued after its first failed component\n' >&2
    exit 1
}
printf 'PASS: negative cache lanes isolate packages, invalidate roots, and stop on failure\n'

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
    invoke_source_gate "$repo" "$scratch/missing-record.manifest" "$metadata"

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
invoke_source_gate --generate "$complex" "$scratch/complex.metadata" "$scratch/complex.manifest"
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
    invoke_source_gate "$complex" "$repo/proofs/VERIFIED_MODULES" "$scratch/complex.metadata"

outside=$(new_copy unadmitted-outside-verus)
printf '\npub fn unadmitted_production_body() {}\n' \
    >>"$outside/crates/ferric-spec/src/configuration.rs"
write_metadata "$outside" "$scratch/outside.metadata"
expect_rejected parser-unadmitted-outside-verus 'unverified executable body admission drifted' \
    invoke_source_gate --generate "$outside" "$scratch/outside.metadata" "$scratch/outside.manifest"

stale=$(new_copy stale-unverified-body)
cat >>"$stale/proofs/UNVERIFIED_BODIES" <<'ADMISSIONS'
unverified=ferric-spec|crates/ferric-spec/src/configuration.rs|ferric_spec::configuration::stale_admission_probe|pending-verus|hostile-stale-admission-fixture
ADMISSIONS
write_metadata "$stale" "$scratch/stale.metadata"
expect_rejected parser-stale-unverified-body 'unverified executable body admission drifted' \
    invoke_source_gate --generate "$stale" "$scratch/stale.metadata" "$scratch/stale.manifest"

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

#[verifier::rlimit(20)]
proof fn bounded_resource_attribute_probe()
{}
}
RS
write_metadata "$solver" "$scratch/solver.metadata"
invoke_source_gate "$solver" "$repo/proofs/VERIFIED_MODULES" "$scratch/solver.metadata"

resource_limit=$(new_copy unsupported-resource-limit)
cat >>"$resource_limit/crates/ferric-spec/src/identity.rs" <<'RS'

verus! {
#[verifier::rlimit(101)]
proof fn unsupported_resource_limit_probe()
{}
}
RS
write_metadata "$resource_limit" "$scratch/resource-limit.metadata"
expect_rejected parser-unsupported-resource-limit 'unsupported verifier resource limit: 101' \
    invoke_source_gate --generate "$resource_limit" "$scratch/resource-limit.metadata" \
    "$scratch/resource-limit.manifest"

constructor=$(new_copy admitted-allocation-constructor)
write_metadata "$constructor" "$scratch/constructor.metadata"
invoke_source_gate --generate "$constructor" "$scratch/constructor.metadata" \
    "$scratch/constructor.manifest"
grep -F 'ferric_engine::cache::KvPool::new_bounded' \
    "$scratch/constructor.manifest" >/dev/null || {
    printf 'FAIL: exact allocation constructor was not classified\n' >&2
    exit 1
}

engine_constructor=$(new_copy admitted-engine-allocation-constructor)
write_metadata "$engine_constructor" "$scratch/engine-constructor.metadata"
invoke_source_gate --generate "$engine_constructor" \
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
    invoke_source_gate --generate "$engine_transition" \
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
invoke_source_gate --generate "$ghost_allocation" \
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
    invoke_source_gate --generate "$allocation" "$scratch/allocation.metadata" \
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
    invoke_source_gate --generate "$raw" "$scratch/raw.metadata" "$scratch/raw.manifest"

macro=$(new_copy item-macro)
cat >>"$macro/crates/ferric-spec/src/configuration.rs" <<'RS'

macro_rules! generated_body {
    () => { pub fn macro_generated_probe() {} };
}
generated_body!();
RS
write_metadata "$macro" "$scratch/macro.metadata"
expect_rejected parser-item-macro 'item macro invocation is forbidden' \
    invoke_source_gate --generate "$macro" "$scratch/macro.metadata" "$scratch/macro.manifest"

orphan=$(new_copy orphan-source)
printf 'pub fn orphan_probe() {}\n' >"$orphan/crates/ferric-spec/src/orphan_probe.rs"
write_metadata "$orphan" "$scratch/orphan.metadata"
expect_rejected parser-orphan-source 'contains unreachable Rust source' \
    invoke_source_gate --generate "$orphan" "$scratch/orphan.metadata" "$scratch/orphan.manifest"

test_module_nested=$(new_copy cfg-test-external-module-nested)
mkdir -p "$test_module_nested/crates/ferric-engine/src/authenticated_test_runtime"
mv "$test_module_nested/crates/ferric-engine/src/authenticated_test_runtime.rs" \
    "$test_module_nested/crates/ferric-engine/src/authenticated_test_runtime/mod.rs"
write_metadata "$test_module_nested" "$scratch/test-module-nested.metadata"
invoke_source_gate --generate "$test_module_nested" \
    "$scratch/test-module-nested.metadata" "$scratch/test-module-nested.manifest"
if grep -F 'authenticated_test_runtime' "$scratch/test-module-nested.manifest" >/dev/null; then
    printf 'FAIL: nested cfg(test) external module entered the release manifest\n' >&2
    exit 1
fi

test_module_missing=$(new_copy cfg-test-external-module-missing)
rm "$test_module_missing/crates/ferric-engine/src/authenticated_test_runtime.rs"
write_metadata "$test_module_missing" "$scratch/test-module-missing.metadata"
expect_rejected parser-cfg-test-external-module-missing \
    'test-only module ferric_engine::authenticated_test_runtime must resolve to exactly one source file' \
    invoke_source_gate --generate "$test_module_missing" \
    "$scratch/test-module-missing.metadata" "$scratch/test-module-missing.manifest"

test_module_ambiguous=$(new_copy cfg-test-external-module-ambiguous)
mkdir -p "$test_module_ambiguous/crates/ferric-engine/src/authenticated_test_runtime"
cp "$test_module_ambiguous/crates/ferric-engine/src/authenticated_test_runtime.rs" \
    "$test_module_ambiguous/crates/ferric-engine/src/authenticated_test_runtime/mod.rs"
write_metadata "$test_module_ambiguous" "$scratch/test-module-ambiguous.metadata"
expect_rejected parser-cfg-test-external-module-ambiguous \
    'test-only module ferric_engine::authenticated_test_runtime must resolve to exactly one source file' \
    invoke_source_gate --generate "$test_module_ambiguous" \
    "$scratch/test-module-ambiguous.metadata" "$scratch/test-module-ambiguous.manifest"

test_module_duplicate=$(new_copy cfg-test-external-module-duplicate)
cat >>"$test_module_duplicate/crates/ferric-engine/src/lib.rs" <<'RS'

#[cfg(test)]
mod authenticated_test_runtime;
RS
write_metadata "$test_module_duplicate" "$scratch/test-module-duplicate.metadata"
expect_rejected parser-cfg-test-external-module-duplicate \
    'module source is included more than once' \
    invoke_source_gate --generate "$test_module_duplicate" \
    "$scratch/test-module-duplicate.metadata" "$scratch/test-module-duplicate.manifest"

test_module_child=$(new_copy cfg-test-external-module-child)
mkdir -p "$test_module_child/crates/ferric-engine/src/authenticated_test_runtime"
cat >>"$test_module_child/crates/ferric-engine/src/authenticated_test_runtime.rs" <<'RS'

mod nested_probe;
RS
printf 'pub fn nested_probe() {}\n' \
    >"$test_module_child/crates/ferric-engine/src/authenticated_test_runtime/nested_probe.rs"
write_metadata "$test_module_child" "$scratch/test-module-child.metadata"
expect_rejected parser-cfg-test-external-module-child 'contains unreachable Rust source' \
    invoke_source_gate --generate "$test_module_child" \
    "$scratch/test-module-child.metadata" "$scratch/test-module-child.manifest"

conditional=$(new_copy conditional-source)
printf '\n#[cfg(any())]\npub fn conditionally_absent_probe() {}\n' \
    >>"$conditional/crates/ferric-spec/src/configuration.rs"
write_metadata "$conditional" "$scratch/conditional.metadata"
expect_rejected parser-cfg 'conditional source is forbidden' \
    invoke_source_gate --generate "$conditional" "$scratch/conditional.metadata" \
    "$scratch/conditional.manifest"

deny=$(new_copy unsupported-deny)
sed -i 's/^#!\[deny(missing_docs)\]$/#![deny(dead_code)]/' \
    "$deny/crates/ferric-qwen-kernels/src/lib.rs"
write_metadata "$deny" "$scratch/deny.metadata"
expect_rejected parser-unsupported-deny 'unsupported deny attribute: dead_code' \
    invoke_source_gate --generate "$deny" "$scratch/deny.metadata" "$scratch/deny.manifest"

repr=$(new_copy unsupported-repr)
sed -i '0,/^#\[repr(u8)\]$/s//#[repr(C)]/' \
    "$repr/crates/ferric-qwen-kernels/src/gemm.rs"
write_metadata "$repr" "$scratch/repr.metadata"
expect_rejected parser-unsupported-repr 'unsupported repr attribute: C' \
    invoke_source_gate --generate "$repr" "$scratch/repr.metadata" "$scratch/repr.manifest"

assert_include=$(new_copy assert-source-inclusion)
cat >>"$assert_include/crates/ferric-spec/src/configuration.rs" <<'RS'

const _: () = {
    assert!(include_str!("configuration.rs").is_empty());
};
RS
write_metadata "$assert_include" "$scratch/assert-include.metadata"
expect_rejected parser-assert-source-inclusion 'source inclusion macro is forbidden: include_str!' \
    invoke_source_gate --generate "$assert_include" "$scratch/assert-include.metadata" \
    "$scratch/assert-include.manifest"

trust=$(new_copy trust-attribute)
cat >>"$trust/crates/ferric-spec/src/configuration.rs" <<'RS'

#[verus_verify(external_body)]
pub fn alternate_trust_attribute_probe() {}
RS
write_metadata "$trust" "$scratch/trust.metadata"
expect_rejected parser-alternate-trust-attribute 'trust-expanding or unsupported verifier attribute: verus_verify' \
    invoke_source_gate --generate "$trust" "$scratch/trust.metadata" "$scratch/trust.manifest"

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
    invoke_source_gate --generate "$assume" "$scratch/assume.metadata" "$scratch/assume.manifest"

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
    invoke_source_gate --generate "$optout" "$scratch/optout.metadata" "$scratch/optout.manifest"

foreign_source_edge=$(new_copy non-authoritative-source-foreign-edge)
python3 -I - "$foreign_source_edge/crates/ferric-build/Cargo.toml" \
    "$foreign_source_edge/Cargo.lock" <<'PY'
from pathlib import Path
import sys

manifest = Path(sys.argv[1])
lock = Path(sys.argv[2])
source = manifest.read_text(encoding="utf-8")
needle = "[dependencies]\n"
replacement = (
    "[dependencies]\n"
    "ferric-non-authoritative-program-source-v1 = "
    "{ path = \"../ferric-non-authoritative-program-source-v1\" }\n"
)
if source.count(needle) != 1:
    raise SystemExit("foreign source dependency manifest anchor drifted")
manifest.write_text(source.replace(needle, replacement), encoding="utf-8")
source = lock.read_text(encoding="utf-8")
needle = 'name = "ferric-build"\nversion = "0.1.0"\ndependencies = [\n'
replacement = needle + ' "ferric-non-authoritative-program-source-v1",\n'
if source.count(needle) != 1:
    raise SystemExit("foreign source dependency lock anchor drifted")
lock.write_text(source.replace(needle, replacement), encoding="utf-8")
PY
write_metadata "$foreign_source_edge" "$scratch/foreign-source-edge.metadata"
expect_rejected dependency-non-authoritative-source-foreign-edge \
    'package ferric-build cannot directly construct non-authoritative program source custody' \
    invoke_source_gate --generate "$foreign_source_edge" \
    "$scratch/foreign-source-edge.metadata" "$scratch/foreign-source-edge.manifest"

foreign_source_dev_edge=$(new_copy non-authoritative-source-foreign-dev-edge)
python3 -I - "$foreign_source_dev_edge/crates/ferric-build/Cargo.toml" \
    "$foreign_source_dev_edge/Cargo.lock" <<'PY'
from pathlib import Path
import sys

manifest = Path(sys.argv[1])
lock = Path(sys.argv[2])
source = manifest.read_text(encoding="utf-8")
needle = "\n[package.metadata.verus]\n"
replacement = (
    "\n[dev-dependencies]\n"
    "ferric-non-authoritative-program-source-v1 = "
    "{ path = \"../ferric-non-authoritative-program-source-v1\" }\n"
    "\n[package.metadata.verus]\n"
)
if source.count(needle) != 1:
    raise SystemExit("foreign source dev-dependency manifest anchor drifted")
manifest.write_text(source.replace(needle, replacement), encoding="utf-8")
source = lock.read_text(encoding="utf-8")
needle = 'name = "ferric-build"\nversion = "0.1.0"\ndependencies = [\n'
replacement = needle + ' "ferric-non-authoritative-program-source-v1",\n'
if source.count(needle) != 1:
    raise SystemExit("foreign source dev-dependency lock anchor drifted")
lock.write_text(source.replace(needle, replacement), encoding="utf-8")
PY
write_metadata "$foreign_source_dev_edge" "$scratch/foreign-source-dev-edge.metadata"
expect_rejected dependency-non-authoritative-source-foreign-dev-edge \
    'package ferric-build cannot directly construct non-authoritative program source custody' \
    invoke_source_gate --generate "$foreign_source_dev_edge" \
    "$scratch/foreign-source-dev-edge.metadata" \
    "$scratch/foreign-source-dev-edge.manifest"

missing_engine_source_edge=$(new_copy non-authoritative-source-missing-engine-edge)
python3 -I - "$missing_engine_source_edge/crates/ferric-engine/Cargo.toml" \
    "$missing_engine_source_edge/Cargo.lock" <<'PY'
from pathlib import Path
import sys

manifest = Path(sys.argv[1])
lock = Path(sys.argv[2])
dependency = (
    "ferric-non-authoritative-program-source-v1 = "
    "{ path = \"../ferric-non-authoritative-program-source-v1\" }\n"
)
source = manifest.read_text(encoding="utf-8")
if source.count(dependency) != 1:
    raise SystemExit("engine source dependency manifest anchor drifted")
manifest.write_text(source.replace(dependency, ""), encoding="utf-8")
source = lock.read_text(encoding="utf-8")
dependency = ' "ferric-non-authoritative-program-source-v1",\n'
engine_start = source.index('name = "ferric-engine"')
engine_end = source.index("\n[[package]]", engine_start)
engine = source[engine_start:engine_end]
if engine.count(dependency) != 1:
    raise SystemExit("engine source dependency lock anchor drifted")
source = source[:engine_start] + engine.replace(dependency, "") + source[engine_end:]
lock.write_text(source, encoding="utf-8")
PY
write_metadata "$missing_engine_source_edge" "$scratch/missing-engine-source-edge.metadata"
expect_rejected dependency-non-authoritative-source-missing-engine-edge \
    'ferric-engine no longer retains the internal non-authoritative source boundary' \
    invoke_source_gate --generate "$missing_engine_source_edge" \
    "$scratch/missing-engine-source-edge.metadata" \
    "$scratch/missing-engine-source-edge.manifest"

demoted_engine_source_edge=$(new_copy non-authoritative-source-demoted-engine-edge)
python3 -I - "$demoted_engine_source_edge/crates/ferric-engine/Cargo.toml" <<'PY'
from pathlib import Path
import sys

manifest = Path(sys.argv[1])
source = manifest.read_text(encoding="utf-8")
dependency = (
    "ferric-non-authoritative-program-source-v1 = "
    "{ path = \"../ferric-non-authoritative-program-source-v1\" }\n"
)
if source.count(dependency) != 1:
    raise SystemExit("engine source dependency demotion anchor drifted")
source = source.replace(dependency, "")
needle = "\n[dev-dependencies]\n"
if source.count(needle) != 1:
    raise SystemExit("engine dev-dependency anchor drifted")
replacement = needle + dependency
manifest.write_text(source.replace(needle, replacement), encoding="utf-8")
PY
write_metadata "$demoted_engine_source_edge" "$scratch/demoted-engine-source-edge.metadata"
expect_rejected dependency-non-authoritative-source-demoted-engine-edge \
    'ferric-engine no longer retains the internal non-authoritative source boundary' \
    invoke_source_gate --generate "$demoted_engine_source_edge" \
    "$scratch/demoted-engine-source-edge.metadata" \
    "$scratch/demoted-engine-source-edge.manifest"

printf 'PASS: compiler-rooted admission rejects stale, unparsed, conditional, trust-expanded, and opted-out source\n'
