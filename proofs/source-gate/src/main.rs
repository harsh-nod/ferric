use proc_macro2::Ident;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use verus_syn::parse::Parser;
use verus_syn::visit::{self, Visit};
use verus_syn::{
    Attribute, Block, Expr, File, FnMode, ImplItem, Item, ItemImpl, Meta, Path as SynPath, Publish,
    Signature, TraitItem, Type,
};

const FORMAT: &str = "FERRIC-VERIFIED-MODULES-V2";
const RUNTIME_TCB_FORMAT: &str = "FERRIC-RUNTIME-DEPENDENCY-TCB-V1";
const RUNTIME_TCB_PATH: &str = "proofs/RUNTIME_DEPENDENCY_TCB";
const CRATES_IO_SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";
const VERUS_SOURCE: &str = "git+https://github.com/verus-lang/verus.git?rev=b677dd5";
const FE2O3_SOURCE: &str =
    "git+https://github.com/harsh-nod/fe2o3.git?rev=73a358f1be5de0d515c888841a87294e2725d634";
const QUALIFIED_BINARIES: &[(&str, &str, &str)] = &[
    (
        "ferric-build",
        "ferric-m1-generate-runner",
        "crates/ferric-build/src/bin/ferric-m1-generate-runner.rs",
    ),
    (
        "ferric-build",
        "ferric-m1-kernel-artifacts",
        "crates/ferric-build/src/bin/ferric-m1-kernel-artifacts.rs",
    ),
    (
        "ferric-build",
        "ferric-m1-prepack",
        "crates/ferric-build/src/bin/ferric-m1-prepack.rs",
    ),
    (
        "ferric-engine",
        "ferric-m1-qualification-capture",
        "crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs",
    ),
    (
        "ferric-m1-benchmarks",
        "ferric-m1-adversarial",
        "benches/m1/adversarial.rs",
    ),
    (
        "ferric-m1-benchmarks",
        "ferric-m1-d10",
        "benches/m1/d10.rs",
    ),
    (
        "ferric-m1-benchmarks",
        "ferric-m1-differential",
        "benches/m1/differential.rs",
    ),
    (
        "ferric-m1-benchmarks",
        "ferric-m1-serving",
        "benches/m1/serving.rs",
    ),
    (
        "ferric-m1-benchmarks",
        "ferric-m1-speculation",
        "benches/m1/speculation.rs",
    ),
];
const RUNTIME_ROOTS: &[(&str, &str, &str, bool, &[&str])] = &[
    ("ferric-build", "onig", "=6.5.3", false, &[]),
    ("ferric-build", "rustix", "=1.1.4", true, &["fs"]),
    ("ferric-build", "sha2", "^0.11.0", true, &[]),
    (
        "ferric-build",
        "unicode-normalization-alignments",
        "=0.1.12",
        true,
        &[],
    ),
    ("ferric-engine", "rustix", "=1.1.4", true, &["fs"]),
    ("ferric-engine", "serde_json", "=1.0.151", true, &[]),
    ("ferric-engine", "sha2", "^0.11.0", true, &[]),
    ("ferric-m1-benchmarks", "num-bigint", "=0.4.8", true, &[]),
    (
        "ferric-m1-benchmarks",
        "rustix",
        "=1.1.4",
        true,
        &["fs", "process"],
    ),
    (
        "ferric-m1-benchmarks",
        "serde_json",
        "=1.0.151",
        true,
        &[],
    ),
    ("ferric-m1-benchmarks", "sha2", "^0.11.0", true, &[]),
    ("ferric-qwen-kernels", "sha2", "^0.11.0", true, &[]),
];
const FE2O3_ROOTS: &[(&str, &str)] = &[
    ("ferric-build", "fe2o3-amdhsa-loader"),
    ("ferric-build", "fe2o3-artifact-transaction"),
    ("ferric-build", "fe2o3-compiler-ffi"),
    ("ferric-build", "fe2o3-hsaco-finalize"),
    ("ferric-build", "fe2o3-llvm-worker-handoff"),
    ("ferric-engine", "fe2o3-amdhsa-loader"),
    ("ferric-engine", "fe2o3-aql"),
    ("ferric-engine", "fe2o3-kfd"),
    ("ferric-engine", "fe2o3-service-host"),
    ("ferric-qwen-kernels", "fe2o3-amdhsa-loader"),
    ("ferric-qwen-kernels", "fe2o3-artifact-transaction"),
    ("ferric-qwen-kernels", "fe2o3-compiler-ffi"),
    ("ferric-qwen-kernels", "fe2o3-hsaco"),
    ("ferric-qwen-kernels", "fe2o3-hsaco-finalize"),
    ("ferric-qwen-kernels", "fe2o3-llvm-handoff"),
    ("ferric-qwen-kernels", "fe2o3-llvm-text"),
    ("ferric-qwen-kernels", "fe2o3-llvm-worker-handoff"),
    ("ferric-qwen-kernels", "reserved-fe2o3-symbols"),
];
const ENGINE_ALLOCATION_CONSTRUCTORS: &[&str] = &[
    "ferric_engine::cache::KvPool::new_bounded",
    "ferric_engine::system::Engine::new",
];

type GateResult<T> = Result<T, String>;
type ModuleMap = BTreeMap<String, (String, String)>;
type WalkOutput = (ModuleMap, BTreeSet<Function>, BTreeSet<PathBuf>);

#[derive(Clone)]
struct PackageTarget {
    crate_name: String,
    root: PathBuf,
}

#[derive(Clone)]
struct Package {
    name: String,
    crate_name: String,
    root: PathBuf,
    dependencies: BTreeSet<String>,
    additional_targets: Vec<PackageTarget>,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct Function {
    package: String,
    source: String,
    compiler_path: String,
    verified: bool,
}

struct PendingInherentMethod {
    source: String,
    source_module: String,
    owner: String,
    method: String,
    verified: bool,
}

#[derive(Default)]
struct Inventory {
    packages: Vec<Package>,
    modules: ModuleMap,
    functions: BTreeSet<Function>,
    runtime_tcb: Vec<String>,
}

struct RuntimeTcb {
    text: String,
    roots: BTreeSet<(String, String)>,
}

#[derive(Default)]
struct LockPackage {
    name: Option<String>,
    version: Option<String>,
    source: Option<String>,
    checksum: Option<String>,
}

struct SourceWalker<'a> {
    repo: &'a Path,
    package: &'a Package,
    source_root: PathBuf,
    module_dir: PathBuf,
    visited: BTreeSet<PathBuf>,
    modules: ModuleMap,
    functions: BTreeSet<Function>,
    type_owners: BTreeMap<String, BTreeSet<String>>,
    inherent_methods: Vec<PendingInherentMethod>,
}

struct SyntaxAudit {
    errors: Vec<String>,
    allow_root_function: bool,
    allow_solver_attributes: bool,
    root_function_seen: bool,
}

#[derive(Default)]
struct AllocationAudit {
    errors: Vec<String>,
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("FAIL: {}", message.as_ref());
    std::process::exit(1);
}

fn read_json(path: &Path) -> GateResult<Value> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

fn canonical(path: &Path) -> GateResult<PathBuf> {
    path.canonicalize()
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn string_field<'a>(value: &'a Value, field: &str) -> GateResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Cargo metadata field {field} is absent or malformed"))
}

fn safe_atom(value: &str, description: &str) -> GateResult<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(format!("unsafe {description}: {value:?}"));
    }
    Ok(())
}

fn safe_tcb_field(value: &str, description: &str) -> GateResult<()> {
    if value.is_empty() || value.contains(['\n', '\r', '|']) {
        return Err(format!(
            "unsafe runtime dependency TCB {description}: {value:?}"
        ));
    }
    Ok(())
}

fn bool_field(value: &Value, field: &str) -> GateResult<bool> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("Cargo metadata field {field} is absent or malformed"))
}

fn is_dev_dependency(dependency: &Value) -> GateResult<bool> {
    match dependency.get("kind") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::String(kind)) if kind == "dev" => Ok(true),
        Some(Value::String(kind)) => Err(format!("unsupported workspace dependency kind: {kind}")),
        Some(_) => Err("workspace dependency kind is malformed".to_owned()),
    }
}

fn string_array(value: &Value, field: &str, description: &str) -> GateResult<Vec<String>> {
    let values = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Cargo metadata field {field} is absent or malformed"))?;
    let mut result = Vec::new();
    for item in values {
        let item = item
            .as_str()
            .ok_or_else(|| format!("Cargo metadata {description} is not a string"))?;
        safe_tcb_field(item, description)?;
        result.push(item.to_owned());
    }
    result.sort();
    if result.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(format!("duplicate Cargo metadata {description}"));
    }
    Ok(result)
}

fn lock_string(line: &str, field: &str) -> GateResult<Option<String>> {
    let prefix = format!("{field} = \"");
    let Some(value) = line.strip_prefix(&prefix) else {
        return Ok(None);
    };
    let value = value
        .strip_suffix('"')
        .ok_or_else(|| format!("malformed Cargo.lock {field} string"))?;
    if value.is_empty() || value.contains(['\\', '\n', '\r', '|']) {
        return Err(format!("unsupported Cargo.lock {field} string: {value:?}"));
    }
    Ok(Some(value.to_owned()))
}

fn finish_lock_package(
    package: LockPackage,
    checksums: &mut BTreeMap<(String, String, String), String>,
) -> GateResult<()> {
    let (Some(name), Some(version), Some(source)) = (package.name, package.version, package.source)
    else {
        return Ok(());
    };
    if !source.starts_with("registry+") {
        return Ok(());
    }
    let checksum = package
        .checksum
        .ok_or_else(|| format!("registry Cargo.lock package has no checksum: {name} {version}"))?;
    if checksum.len() != 64
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!(
            "registry Cargo.lock package has malformed checksum: {name} {version}"
        ));
    }
    if checksums
        .insert((name.clone(), version.clone(), source), checksum)
        .is_some()
    {
        return Err(format!(
            "duplicate registry Cargo.lock package identity: {name} {version}"
        ));
    }
    Ok(())
}

// Cargo 1.97 metadata exposes the resolved graph but no registry checksum
// field, so the gate reads only that missing identity from canonical lock V4.
fn runtime_lock_checksums(repo: &Path) -> GateResult<BTreeMap<(String, String, String), String>> {
    let path = repo.join("Cargo.lock");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !text.starts_with("# This file is automatically @generated by Cargo.\n")
        || !text.lines().any(|line| line == "version = 4")
    {
        return Err("workspace Cargo.lock is not canonical format 4".to_owned());
    }
    let mut current = None;
    let mut checksums = BTreeMap::new();
    for line in text.lines() {
        if line == "[[package]]" {
            if let Some(package) = current.take() {
                finish_lock_package(package, &mut checksums)?;
            }
            current = Some(LockPackage::default());
            continue;
        }
        let Some(package) = current.as_mut() else {
            continue;
        };
        for (field, slot) in [
            ("name", &mut package.name),
            ("version", &mut package.version),
            ("source", &mut package.source),
            ("checksum", &mut package.checksum),
        ] {
            if let Some(value) = lock_string(line, field)? {
                if slot.replace(value).is_some() {
                    return Err(format!("duplicate Cargo.lock package field: {field}"));
                }
            }
        }
    }
    if let Some(package) = current {
        finish_lock_package(package, &mut checksums)?;
    }
    Ok(checksums)
}

fn relative_source(repo: &Path, path: &Path) -> GateResult<String> {
    let relative = path
        .strip_prefix(repo)
        .map_err(|_| format!("source escapes repository: {}", path.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    if relative.is_empty()
        || relative.contains("..")
        || !relative
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'))
    {
        return Err(format!("unsupported source path: {relative:?}"));
    }
    Ok(relative)
}

fn is_opted(package: &Value) -> bool {
    package
        .pointer("/metadata/verus/verify")
        .and_then(Value::as_bool)
        == Some(true)
}

fn package_map(metadata: &Value) -> GateResult<BTreeMap<&str, &Value>> {
    let mut packages = BTreeMap::new();
    for package in metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo metadata has no packages array".to_owned())?
    {
        let id = string_field(package, "id")?;
        safe_tcb_field(id, "package ID")?;
        if packages.insert(id, package).is_some() {
            return Err(format!("duplicate Cargo metadata package ID: {id}"));
        }
    }
    Ok(packages)
}

fn resolve_map(metadata: &Value) -> GateResult<BTreeMap<&str, &Value>> {
    let mut nodes = BTreeMap::new();
    for node in metadata
        .pointer("/resolve/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo metadata has no complete resolve graph".to_owned())?
    {
        let id = string_field(node, "id")?;
        safe_tcb_field(id, "resolve node ID")?;
        if nodes.insert(id, node).is_some() {
            return Err(format!("duplicate Cargo metadata resolve node ID: {id}"));
        }
    }
    Ok(nodes)
}

fn workspace_packages<'a>(
    metadata: &'a Value,
    packages: &BTreeMap<&'a str, &'a Value>,
) -> GateResult<BTreeMap<&'a str, &'a Value>> {
    let mut workspace = BTreeMap::new();
    for member in metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo metadata has no workspace_members array".to_owned())?
    {
        let id = member
            .as_str()
            .ok_or_else(|| "workspace member identity is not a string".to_owned())?;
        let package = packages
            .get(id)
            .ok_or_else(|| format!("workspace package is absent from metadata: {id}"))?;
        let name = string_field(package, "name")?;
        if workspace.insert(name, *package).is_some() {
            return Err(format!("duplicate workspace package name: {name}"));
        }
    }
    if workspace.is_empty() {
        return Err("Cargo metadata workspace package closure is empty".to_owned());
    }
    Ok(workspace)
}

fn validate_root_declaration(
    owner: &str,
    dependency: &Value,
) -> GateResult<Option<(String, String)>> {
    if is_dev_dependency(dependency)? {
        return Ok(None);
    }
    let name = string_field(dependency, "name")?;
    let source = dependency.get("source").and_then(Value::as_str);
    if source != Some(CRATES_IO_SOURCE) {
        return Ok(None);
    }
    let expected = RUNTIME_ROOTS
        .iter()
        .find(|(expected_owner, expected_name, _, _, _)| {
            owner == *expected_owner && name == *expected_name
        })
        .ok_or_else(|| {
            format!("package {owner} has an unadmitted registry runtime root: {name}")
        })?;
    let (_, _, requirement, uses_default_features, expected_features) = *expected;
    let features = string_array(dependency, "features", "root feature")?;
    if string_field(dependency, "req")? != requirement
        || bool_field(dependency, "uses_default_features")? != uses_default_features
        || bool_field(dependency, "optional")?
        || dependency
            .get("rename")
            .is_some_and(|value| !value.is_null())
        || dependency
            .get("target")
            .is_some_and(|value| !value.is_null())
        || dependency
            .get("registry")
            .is_some_and(|value| !value.is_null())
        || features != expected_features
    {
        return Err(format!(
            "workspace registry runtime root declaration drifted: {owner}::{name}"
        ));
    }
    Ok(Some((owner.to_owned(), name.to_owned())))
}

fn validate_fe2o3_root_declaration(owner: &str, dependency: &Value) -> GateResult<bool> {
    let name = string_field(dependency, "name")?;
    let expected = FE2O3_ROOTS
        .iter()
        .any(|(expected_owner, expected_name)| owner == *expected_owner && name == *expected_name);
    if !expected {
        return Ok(false);
    }
    if dependency.get("source").and_then(Value::as_str) != Some(FE2O3_SOURCE)
        || string_field(dependency, "req")? != "*"
        || !bool_field(dependency, "uses_default_features")?
        || bool_field(dependency, "optional")?
        || dependency
            .get("rename")
            .is_some_and(|value| !value.is_null())
        || dependency
            .get("target")
            .is_some_and(|value| !value.is_null())
        || dependency
            .get("registry")
            .is_some_and(|value| !value.is_null())
        || !string_array(dependency, "features", "fe2o3 root feature")?.is_empty()
    {
        return Err(format!(
            "workspace fe2o3 dependency declaration drifted: {owner}::{name}"
        ));
    }
    Ok(true)
}

fn render_runtime_dependency_tcb(repo: &Path, metadata: &Value) -> GateResult<RuntimeTcb> {
    let packages = package_map(metadata)?;
    let nodes = resolve_map(metadata)?;
    let workspace = workspace_packages(metadata, &packages)?;
    let checksums = runtime_lock_checksums(repo)?;
    let mut root_records = Vec::new();
    let mut roots = BTreeSet::new();
    let mut root_ids = Vec::new();

    for (owner, package) in &workspace {
        let node = nodes
            .get(string_field(package, "id")?)
            .ok_or_else(|| format!("workspace package has no resolve node: {owner}"))?;
        let resolved_dependencies =
            node.get("deps").and_then(Value::as_array).ok_or_else(|| {
                format!("workspace package resolve dependencies are malformed: {owner}")
            })?;
        for dependency in package
            .get("dependencies")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("package {owner} dependencies are malformed"))?
        {
            let Some(root) = validate_root_declaration(owner, dependency)? else {
                continue;
            };
            if !roots.insert(root.clone()) {
                return Err(format!(
                    "duplicate workspace registry runtime root: {}::{}",
                    root.0, root.1
                ));
            }
            let dependency_name = string_field(dependency, "name")?;
            let candidates: Vec<&Value> = resolved_dependencies
                .iter()
                .filter(|resolved| {
                    resolved
                        .get("pkg")
                        .and_then(Value::as_str)
                        .and_then(|id| packages.get(id))
                        .and_then(|package| package.get("name"))
                        .and_then(Value::as_str)
                        == Some(dependency_name)
                })
                .collect();
            let [resolved] = candidates.as_slice() else {
                return Err(format!(
                    "workspace registry runtime root does not resolve uniquely: {owner}::{dependency_name}"
                ));
            };
            let resolved_id = string_field(resolved, "pkg")?;
            root_ids.push(resolved_id.to_owned());
            let features = string_array(dependency, "features", "root feature")?;
            let features = if features.is_empty() {
                "none".to_owned()
            } else {
                features.join(",")
            };
            root_records.push(format!(
                "root={owner}|{dependency_name}|{}|{}|{}|{}|features={features}|{resolved_id}",
                string_field(dependency, "req")?,
                string_field(dependency, "source")?,
                bool_field(dependency, "uses_default_features")?,
                bool_field(dependency, "optional")?,
            ));
        }
    }
    let expected_roots: BTreeSet<(String, String)> = RUNTIME_ROOTS
        .iter()
        .map(|(owner, name, _, _, _)| ((*owner).to_owned(), (*name).to_owned()))
        .collect();
    if roots != expected_roots {
        return Err(format!(
            "workspace registry runtime roots drifted (expected={expected_roots:?}, actual={roots:?})"
        ));
    }

    let mut closure = BTreeSet::new();
    let mut stack = root_ids;
    while let Some(id) = stack.pop() {
        if !closure.insert(id.clone()) {
            continue;
        }
        let package = packages
            .get(id.as_str())
            .ok_or_else(|| format!("runtime dependency package is absent: {id}"))?;
        if package.get("source").and_then(Value::as_str) != Some(CRATES_IO_SOURCE) {
            return Err(format!("runtime dependency is not from crates.io: {id}"));
        }
        let node = nodes
            .get(id.as_str())
            .ok_or_else(|| format!("runtime dependency has no resolve node: {id}"))?;
        for dependency in node
            .get("deps")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("runtime dependency resolve edges are malformed: {id}"))?
        {
            stack.push(string_field(dependency, "pkg")?.to_owned());
        }
    }

    let mut package_records = Vec::new();
    let mut feature_records = Vec::new();
    let mut target_records = Vec::new();
    let mut build_script_records = Vec::new();
    let mut proc_macro_records = Vec::new();
    let mut edge_records = Vec::new();
    for id in &closure {
        let package = packages
            .get(id.as_str())
            .ok_or_else(|| format!("runtime dependency package is absent: {id}"))?;
        let name = string_field(package, "name")?;
        let version = string_field(package, "version")?;
        let source = string_field(package, "source")?;
        for (value, description) in [
            (name, "package name"),
            (version, "package version"),
            (source, "package source"),
        ] {
            safe_tcb_field(value, description)?;
        }
        let checksum = checksums
            .get(&(name.to_owned(), version.to_owned(), source.to_owned()))
            .ok_or_else(|| {
                format!("runtime dependency checksum is absent from Cargo.lock: {id}")
            })?;
        package_records.push(format!("package={id}|{name}|{version}|{source}|{checksum}"));
        let node = nodes
            .get(id.as_str())
            .ok_or_else(|| format!("runtime dependency has no resolve node: {id}"))?;
        let features = string_array(node, "features", "resolved feature")?;
        feature_records.push(format!(
            "features={id}|{}",
            if features.is_empty() {
                "none".to_owned()
            } else {
                features.join(",")
            }
        ));

        let mut build_scripts = Vec::new();
        let mut proc_macros = Vec::new();
        for target in package
            .get("targets")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("runtime dependency targets are malformed: {id}"))?
        {
            let target_name = string_field(target, "name")?;
            safe_tcb_field(target_name, "target name")?;
            let kinds = string_array(target, "kind", "target kind")?;
            let crate_types = string_array(target, "crate_types", "target crate type")?;
            if kinds.is_empty() || crate_types.is_empty() {
                return Err(format!(
                    "runtime dependency target is incomplete: {id}::{target_name}"
                ));
            }
            target_records.push(format!(
                "target={id}|{target_name}|{}|{}",
                kinds.join(","),
                crate_types.join(",")
            ));
            if kinds.iter().any(|kind| kind == "custom-build") {
                build_scripts.push(target_name.to_owned());
            }
            if kinds.iter().any(|kind| kind == "proc-macro") {
                proc_macros.push(target_name.to_owned());
            }
        }
        build_scripts.sort();
        proc_macros.sort();
        build_script_records.push(format!(
            "build-scripts={id}|{}",
            if build_scripts.is_empty() {
                "none".to_owned()
            } else {
                build_scripts.join(",")
            }
        ));
        proc_macro_records.push(format!(
            "proc-macros={id}|{}",
            if proc_macros.is_empty() {
                "none".to_owned()
            } else {
                proc_macros.join(",")
            }
        ));

        for dependency in node
            .get("deps")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("runtime dependency resolve edges are malformed: {id}"))?
        {
            let dependency_name = string_field(dependency, "name")?;
            safe_tcb_field(dependency_name, "resolved dependency name")?;
            let dependency_id = string_field(dependency, "pkg")?;
            if !closure.contains(dependency_id) {
                return Err(format!(
                    "runtime dependency edge escapes resolved closure: {id}::{dependency_name}"
                ));
            }
            for kind in dependency
                .get("dep_kinds")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("runtime dependency edge kinds are malformed: {id}"))?
            {
                let kind_name = kind.get("kind").and_then(Value::as_str).unwrap_or("normal");
                let target = kind.get("target").and_then(Value::as_str).unwrap_or("none");
                safe_tcb_field(kind_name, "dependency kind")?;
                safe_tcb_field(target, "dependency target")?;
                edge_records.push(format!(
                    "edge={id}|{dependency_name}|{dependency_id}|{kind_name}|{target}"
                ));
            }
        }
    }
    for records in [
        &mut root_records,
        &mut package_records,
        &mut feature_records,
        &mut target_records,
        &mut build_script_records,
        &mut proc_macro_records,
        &mut edge_records,
    ] {
        records.sort();
        if records.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("duplicate canonical runtime dependency TCB record".to_owned());
        }
    }
    let mut lines = vec![format!("format={RUNTIME_TCB_FORMAT}")];
    lines.extend(root_records);
    lines.extend(package_records);
    lines.extend(feature_records);
    lines.extend(target_records);
    lines.extend(build_script_records);
    lines.extend(proc_macro_records);
    lines.extend(edge_records);
    Ok(RuntimeTcb {
        text: lines.join("\n") + "\n",
        roots,
    })
}

fn validate_runtime_dependency_tcb(repo: &Path, metadata: &Value) -> GateResult<RuntimeTcb> {
    let runtime_tcb = render_runtime_dependency_tcb(repo, metadata)?;
    let path = repo.join(RUNTIME_TCB_PATH);
    let admitted =
        fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    if admitted != runtime_tcb.text {
        return Err("workspace runtime dependency TCB drifted".to_owned());
    }
    Ok(runtime_tcb)
}

fn packages(
    repo: &Path,
    metadata: &Value,
    runtime_roots: &BTreeSet<(String, String)>,
) -> GateResult<Vec<Package>> {
    let members: BTreeSet<&str> = metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo metadata has no workspace_members array".to_owned())?
        .iter()
        .map(|member| {
            member
                .as_str()
                .ok_or_else(|| "workspace member identity is not a string".to_owned())
        })
        .collect::<GateResult<_>>()?;
    let package_values = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo metadata has no packages array".to_owned())?;
    let workspace: Vec<&Value> = package_values
        .iter()
        .filter(|package| {
            package
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| members.contains(id))
        })
        .collect();
    if workspace.len() != members.len() || workspace.is_empty() {
        return Err("Cargo metadata workspace package closure is incomplete".to_owned());
    }

    let mut manifest_to_name = BTreeMap::new();
    for package in &workspace {
        let name = string_field(package, "name")?;
        safe_atom(name, "workspace package name")?;
        if !is_opted(package) {
            return Err(format!(
                "first-party workspace package is not opted into strict Verus: {name}"
            ));
        }
        if name == "ferric-build" {
            let features = package
                .get("features")
                .and_then(Value::as_object)
                .ok_or_else(|| "ferric-build feature declarations are malformed".to_owned())?;
            let Some(test_fixtures) = features.get("test-fixtures").and_then(Value::as_array)
            else {
                return Err("ferric-build test-fixtures feature is absent or malformed".to_owned());
            };
            if features.len() != 1 || !test_fixtures.is_empty() {
                return Err("ferric-build test-fixtures feature declaration drifted".to_owned());
            }
        }
        let manifest = canonical(
            Path::new(string_field(package, "manifest_path")?)
                .parent()
                .ok_or_else(|| format!("package {name} manifest has no parent"))?,
        )?;
        if !manifest.starts_with(repo) {
            return Err(format!("workspace package escapes repository: {name}"));
        }
        if manifest_to_name.insert(manifest, name.to_owned()).is_some() {
            return Err("workspace packages share a manifest directory".to_owned());
        }
    }

    let mut result = Vec::new();
    let mut crate_names = BTreeSet::new();
    for package in workspace {
        let name = string_field(package, "name")?.to_owned();
        let targets = package
            .get("targets")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("package {name} has no targets array"))?;
        let mut library_targets = Vec::new();
        let mut additional_targets = Vec::new();
        for target in targets {
            let target = target
                .as_object()
                .ok_or_else(|| format!("package {name} target is malformed"))?;
            let kinds = target
                .get("kind")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("package {name} target has no kind"))?;
            if kinds.len() == 1 && kinds[0].as_str() == Some("lib") {
                library_targets.push(target);
            } else if kinds.len() == 1 && kinds[0].as_str() == Some("test") {
            } else if kinds.len() == 1
                && kinds[0].as_str() == Some("bin")
                && target
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|target_name| {
                        QUALIFIED_BINARIES
                            .iter()
                            .any(|(owner, expected_name, _)| {
                                name == *owner && target_name == *expected_name
                            })
                    })
            {
                let target_name = target
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "qualified binary has no name".to_owned())?;
                let crate_types = target
                    .get("crate_types")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "qualified binary has no crate types".to_owned())?;
                if crate_types.len() != 1 || crate_types[0].as_str() != Some("bin") {
                    return Err("qualified binary crate type drifted".to_owned());
                }
                let root = canonical(Path::new(
                    target
                        .get("src_path")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "qualified binary has no source path".to_owned())?,
                ))?;
                let expected_path = QUALIFIED_BINARIES
                    .iter()
                    .find_map(|(owner, expected_name, expected_path)| {
                        (name == *owner && target_name == *expected_name)
                            .then_some(*expected_path)
                    })
                    .ok_or_else(|| "unsupported qualified binary".to_owned())?;
                let expected_root = canonical(&repo.join(expected_path))?;
                if root != expected_root {
                    return Err("qualified binary source path drifted".to_owned());
                }
                let crate_name = target_name.replace('-', "_");
                safe_atom(&crate_name, "binary crate name")?;
                additional_targets.push(PackageTarget { crate_name, root });
            } else {
                return Err(format!(
                    "qualified package {name} has an unsupported non-library target"
                ));
            }
        }
        let [target] = library_targets.as_slice() else {
            return Err(format!(
                "qualified package {name} must contain exactly one library target"
            ));
        };
        let crate_name = target
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("package {name} library has no crate name"))?
            .to_owned();
        safe_atom(&crate_name, "crate name")?;
        if !crate_names.insert(crate_name.clone()) {
            return Err(format!("qualified packages share crate name: {crate_name}"));
        }
        for target in &additional_targets {
            if !crate_names.insert(target.crate_name.clone()) {
                return Err(format!(
                    "qualified targets share crate name: {}",
                    target.crate_name
                ));
            }
        }
        let root = canonical(Path::new(
            target
                .get("src_path")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("package {name} library has no source path"))?,
        ))?;
        if !root.starts_with(repo) {
            return Err(format!("package {name} library root escapes repository"));
        }

        let mut dependencies = BTreeSet::new();
        for dependency in package
            .get("dependencies")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("package {name} dependencies are malformed"))?
        {
            let dependency_name = string_field(dependency, "name")?;
            let is_dev = is_dev_dependency(dependency)?;
            let requested_features = string_array(dependency, "features", "workspace feature")?;
            if requested_features
                .iter()
                .any(|feature| feature == "test-fixtures")
                && !(name == "ferric-engine" && dependency_name == "ferric-build" && is_dev)
            {
                return Err(format!(
                    "package {name} activates the test-fixtures feature outside its admitted dev edge"
                ));
            }
            if is_dev {
                continue;
            }
            if let Some(path) = dependency.get("path").and_then(Value::as_str) {
                let dependency_path = canonical(Path::new(path))?;
                let admitted = manifest_to_name.get(&dependency_path).ok_or_else(|| {
                    format!(
                        "package {name} has an unadmitted path dependency: {}",
                        dependency_path.display()
                    )
                })?;
                if admitted != dependency_name {
                    return Err(format!(
                        "package {name} path dependency identity drifted: {dependency_name}"
                    ));
                }
                dependencies.insert(admitted.clone());
            } else {
                let source = dependency.get("source").and_then(Value::as_str);
                let admitted_runtime_root = source == Some(CRATES_IO_SOURCE)
                    && runtime_roots.contains(&(name.clone(), dependency_name.to_owned()));
                let admitted_vstd = dependency_name == "vstd" && source == Some(VERUS_SOURCE);
                let admitted_fe2o3 = validate_fe2o3_root_declaration(&name, dependency)?;
                if !admitted_runtime_root && !admitted_vstd && !admitted_fe2o3 {
                    return Err(format!(
                        "package {name} has an unadmitted external dependency: {dependency_name}"
                    ));
                }
            }
        }
        result.push(Package {
            name,
            crate_name,
            root,
            dependencies,
            additional_targets,
        });
    }
    topological_packages(result)
}

fn topological_packages(packages: Vec<Package>) -> GateResult<Vec<Package>> {
    let mut pending: BTreeMap<String, Package> = packages
        .into_iter()
        .map(|package| (package.name.clone(), package))
        .collect();
    let mut ordered = Vec::new();
    let mut emitted = BTreeSet::new();
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .find(|(_, package)| package.dependencies.is_subset(&emitted))
            .map(|(name, _)| name.clone())
            .ok_or_else(|| "workspace path dependency graph contains a cycle".to_owned())?;
        let package = pending.remove(&ready).expect("selected package exists");
        emitted.insert(ready);
        ordered.push(package);
    }
    Ok(ordered)
}

fn mode_is_exec(mode: &FnMode) -> GateResult<bool> {
    match mode {
        FnMode::Default | FnMode::Exec(_) => Ok(true),
        FnMode::Spec(_) | FnMode::SpecChecked(_) | FnMode::Proof(_) => Ok(false),
        FnMode::ProofAxiom(_) => Err("axiom proof functions are forbidden".to_owned()),
    }
}

fn validate_signature(signature: &Signature) -> GateResult<bool> {
    if matches!(signature.publish, Publish::Uninterp(_)) {
        return Err(format!(
            "uninterpreted function is forbidden: {}",
            signature.ident
        ));
    }
    mode_is_exec(&signature.mode)
}

fn path_name(path: &SynPath) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn validate_attributes(attributes: &[Attribute], allow_solver_attributes: bool) -> GateResult<()> {
    const DERIVES: &[&str] = &[
        "Clone",
        "Copy",
        "Debug",
        "Default",
        "Eq",
        "Hash",
        "Ord",
        "PartialEq",
        "PartialOrd",
    ];
    for attribute in attributes {
        let name = path_name(attribute.path());
        match name.as_str() {
            "cfg" | "cfg_attr" => {
                let detail = match &attribute.meta {
                    Meta::List(list) => format!("({})", list.tokens),
                    _ => String::new(),
                };
                return Err(format!("conditional source is forbidden: {name}{detail}"));
            }
            "path" => return Err("#[path] module redirection is forbidden".to_owned()),
            "doc" | "must_use" | "inline" | "cold" | "non_exhaustive" | "deprecated" => {}
            "allow" => {
                let Meta::List(list) = &attribute.meta else {
                    return Err("malformed allow attribute".to_owned());
                };
                if !matches!(
                    list.tokens.to_string().as_str(),
                    "dead_code"
                        | "unused_imports"
                        | "clippy :: cast_possible_truncation"
                        | "clippy :: large_enum_variant"
                        | "clippy :: too_many_arguments"
                        | "clippy :: type_complexity"
                ) {
                    return Err(format!("unsupported allow attribute: {}", list.tokens));
                }
            }
            "forbid" => {
                let Meta::List(list) = &attribute.meta else {
                    return Err("malformed forbid attribute".to_owned());
                };
                if list.tokens.to_string() != "unsafe_code" {
                    return Err(format!("unsupported forbid attribute: {}", list.tokens));
                }
            }
            "deny" => {
                let Meta::List(list) = &attribute.meta else {
                    return Err("malformed deny attribute".to_owned());
                };
                if list.tokens.to_string() != "missing_docs" {
                    return Err(format!("unsupported deny attribute: {}", list.tokens));
                }
            }
            "repr" => {
                let Meta::List(list) = &attribute.meta else {
                    return Err("malformed repr attribute".to_owned());
                };
                if !matches!(
                    list.tokens.to_string().as_str(),
                    "u8" | "u32" | "transparent"
                ) {
                    return Err(format!("unsupported repr attribute: {}", list.tokens));
                }
            }
            "derive" => {
                let Meta::List(list) = &attribute.meta else {
                    return Err("malformed derive attribute".to_owned());
                };
                let derives = list
                    .parse_args_with(
                        verus_syn::punctuated::Punctuated::<SynPath, verus_syn::Token![,]>::parse_terminated,
                    )
                    .map_err(|error| format!("cannot parse derive attribute: {error}"))?;
                for derive in derives {
                    let derive_name = path_name(&derive);
                    if !DERIVES.contains(&derive_name.as_str()) {
                        return Err(format!("unsupported derive macro: {derive_name}"));
                    }
                }
            }
            "verifier::allow" => {
                let Meta::List(list) = &attribute.meta else {
                    return Err("malformed verifier allow attribute".to_owned());
                };
                if list.tokens.to_string() != "autoderive_clone_without_spec" {
                    return Err(format!("unsupported verifier allowance: {}", list.tokens));
                }
            }
            "trigger" | "verifier::bit_vector" if allow_solver_attributes => {}
            name if name.starts_with("verifier::") || name == "verus_verify" => {
                return Err(format!(
                    "trust-expanding or unsupported verifier attribute: {name}"
                ));
            }
            _ => return Err(format!("unsupported source attribute: {name}")),
        }
    }
    Ok(())
}

impl SyntaxAudit {
    fn new(allow_root_function: bool, allow_solver_attributes: bool) -> Self {
        Self {
            errors: Vec::new(),
            allow_root_function,
            allow_solver_attributes,
            root_function_seen: false,
        }
    }

    fn finish(self) -> GateResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.join("; "))
        }
    }

    fn reject_macro(&mut self, path: &SynPath, context: &str) {
        self.errors.push(format!(
            "{context} macro invocation is forbidden: {}!",
            path_name(path)
        ));
    }

    fn visit_expression_macro_arguments(&mut self, tokens: proc_macro2::TokenStream, name: &str) {
        let parser =
            verus_syn::punctuated::Punctuated::<Expr, verus_syn::Token![,]>::parse_terminated;
        match parser.parse2(tokens) {
            Ok(expressions) => {
                for expression in &expressions {
                    self.visit_expr(expression);
                }
            }
            Err(error) => self
                .errors
                .push(format!("unsupported {name}! invocation: {error}")),
        }
    }
}

impl<'ast> Visit<'ast> for SyntaxAudit {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if let Err(error) = validate_attributes(
            std::slice::from_ref(attribute),
            self.allow_solver_attributes,
        ) {
            self.errors.push(error);
        }
        visit::visit_attribute(self, attribute);
    }

    fn visit_ident(&mut self, ident: &'ast Ident) {
        if ident.to_string().starts_with("r#") {
            self.errors
                .push(format!("raw identifier is forbidden: {ident}"));
        }
    }

    fn visit_item_fn(&mut self, function: &'ast verus_syn::ItemFn) {
        if self.allow_root_function && !self.root_function_seen {
            self.root_function_seen = true;
            visit::visit_item_fn(self, function);
        } else {
            self.errors.push(format!(
                "nested executable item is forbidden: {}",
                function.sig.ident
            ));
        }
    }

    fn visit_item_macro(&mut self, item: &'ast verus_syn::ItemMacro) {
        self.reject_macro(&item.mac.path, "item");
    }

    fn visit_impl_item_macro(&mut self, item: &'ast verus_syn::ImplItemMacro) {
        self.reject_macro(&item.mac.path, "impl item");
    }

    fn visit_trait_item_macro(&mut self, item: &'ast verus_syn::TraitItemMacro) {
        self.reject_macro(&item.mac.path, "trait item");
    }

    fn visit_foreign_item_macro(&mut self, item: &'ast verus_syn::ForeignItemMacro) {
        self.reject_macro(&item.mac.path, "foreign item");
    }

    fn visit_stmt_macro(&mut self, statement: &'ast verus_syn::StmtMacro) {
        let name = path_name(&statement.mac.path);
        if matches!(name.as_str(), "assert" | "debug_assert") {
            match verus_syn::parse2::<Expr>(statement.mac.tokens.clone()) {
                Ok(expression) => self.visit_expr(&expression),
                Err(error) => self
                    .errors
                    .push(format!("unsupported assertion invocation: {error}")),
            }
        } else if matches!(name.as_str(), "debug_assert_eq" | "eprintln" | "println") {
            self.visit_expression_macro_arguments(statement.mac.tokens.clone(), &name);
        } else {
            self.reject_macro(&statement.mac.path, "statement");
        }
    }

    fn visit_expr_macro(&mut self, expression: &'ast verus_syn::ExprMacro) {
        let name = path_name(&expression.mac.path);
        if matches!(name.as_str(), "include" | "include_bytes" | "include_str") {
            self.errors
                .push(format!("source inclusion macro is forbidden: {name}!"));
        }
        visit::visit_expr_macro(self, expression);
    }

    fn visit_expr_call(&mut self, call: &'ast verus_syn::ExprCall) {
        if let Expr::Path(path) = call.func.as_ref() {
            if let Some(last) = path.path.segments.last() {
                if matches!(last.ident.to_string().as_str(), "assume" | "admit") {
                    self.errors
                        .push(format!("forbidden trust call: {}", path_name(&path.path)));
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_assume_specification(&mut self, _item: &'ast verus_syn::AssumeSpecification) {
        self.errors
            .push("assume_specification is forbidden".to_owned());
    }
}

impl<'ast> Visit<'ast> for AllocationAudit {
    fn visit_expr_unary(&mut self, expression: &'ast verus_syn::ExprUnary) {
        if matches!(expression.op, verus_syn::UnOp::Proof(_)) {
            return;
        }
        visit::visit_expr_unary(self, expression);
    }

    fn visit_local(&mut self, local: &'ast verus_syn::Local) {
        if local.ghost.is_some() || local.tracked.is_some() {
            return;
        }
        visit::visit_local(self, local);
    }

    fn visit_expr_macro(&mut self, expression: &'ast verus_syn::ExprMacro) {
        if expression
            .mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "vec")
        {
            self.errors.push("vec! allocation is forbidden".to_owned());
        }
        visit::visit_expr_macro(self, expression);
    }

    fn visit_expr_call(&mut self, call: &'ast verus_syn::ExprCall) {
        if let Expr::Path(path) = call.func.as_ref() {
            let segments: Vec<String> = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let vec_constructor = segments.len() >= 2
                && segments[segments.len() - 2] == "Vec"
                && matches!(
                    segments.last().map(String::as_str),
                    Some("new" | "with_capacity")
                );
            let box_constructor = segments.iter().any(|segment| segment == "Box");
            if vec_constructor || box_constructor {
                self.errors.push(format!(
                    "allocation constructor is forbidden: {}",
                    path_name(&path.path)
                ));
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast verus_syn::ExprMethodCall) {
        if matches!(
            call.method.to_string().as_str(),
            "push"
                | "reserve"
                | "reserve_exact"
                | "resize"
                | "resize_with"
                | "extend"
                | "extend_from_slice"
                | "clone"
                | "to_vec"
                | "collect"
        ) {
            self.errors.push(format!(
                "allocation or growth method is forbidden: {}",
                call.method
            ));
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_type_path(&mut self, path: &'ast verus_syn::TypePath) {
        if path
            .path
            .segments
            .iter()
            .any(|segment| segment.ident == "Box")
        {
            self.errors.push("Box type is forbidden".to_owned());
        }
        visit::visit_type_path(self, path);
    }
}

fn audit_engine_allocation(block: &Block, compiler_path: &str) -> GateResult<()> {
    if ENGINE_ALLOCATION_CONSTRUCTORS.contains(&compiler_path) {
        return Ok(());
    }
    let mut audit = AllocationAudit::default();
    audit.visit_block(block);
    if audit.errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "verified engine body {compiler_path} violates no-transition-allocation policy: {}",
            audit.errors.join("; ")
        ))
    }
}

fn audit_item(item: &Item, in_verus: bool, source: &str) -> GateResult<()> {
    let allow_root_function = matches!(item, Item::Fn(_));
    let item_name = match item {
        Item::Const(item) => format!("const {}", item.ident),
        Item::Enum(item) => format!("enum {}", item.ident),
        Item::Fn(item) => format!("fn {}", item.sig.ident),
        Item::Impl(item) => format!(
            "impl {}",
            impl_owner(item).unwrap_or_else(|_| "<unsupported-owner>".to_owned())
        ),
        Item::Static(item) => format!("static {}", item.ident),
        Item::Struct(item) => format!("struct {}", item.ident),
        Item::Trait(item) => format!("trait {}", item.ident),
        Item::Type(item) => format!("type {}", item.ident),
        _ => "item".to_owned(),
    };
    let mut audit = SyntaxAudit::new(allow_root_function, in_verus);
    audit.visit_item(item);
    audit
        .finish()
        .map_err(|error| format!("{source} ({item_name}): {error}"))
}

fn cfg_test_attributes(attributes: &[Attribute]) -> bool {
    let non_doc: Vec<&Attribute> = attributes
        .iter()
        .filter(|attribute| path_name(attribute.path()) != "doc")
        .collect();
    non_doc.len() == 1
        && path_name(non_doc[0].path()) == "cfg"
        && matches!(&non_doc[0].meta, Meta::List(list) if list.tokens.to_string() == "test")
}

fn cfg_test_item(item: &Item) -> bool {
    match item {
        Item::Impl(item) => cfg_test_attributes(&item.attrs),
        Item::Mod(item) => cfg_test_attributes(&item.attrs),
        _ => false,
    }
}

fn cfg_test_fixture_item(item: &Item) -> GateResult<bool> {
    let attributes = match item {
        Item::Fn(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => return Ok(false),
    };
    let cfg_attributes: Vec<&Attribute> = attributes
        .iter()
        .filter(|attribute| path_name(attribute.path()) == "cfg")
        .collect();
    if cfg_attributes.is_empty() {
        return Ok(false);
    }
    if cfg_attributes.len() != 1
        || !matches!(
            &cfg_attributes[0].meta,
            Meta::List(list) if list.tokens.to_string() == "feature = \"test-fixtures\""
        )
    {
        return Ok(false);
    }
    let remaining: Vec<Attribute> = attributes
        .iter()
        .filter(|attribute| path_name(attribute.path()) != "cfg")
        .cloned()
        .collect();
    validate_attributes(&remaining, false)?;
    Ok(true)
}

fn impl_owner(item: &ItemImpl) -> GateResult<String> {
    let Type::Path(path) = item.self_ty.as_ref() else {
        return Err("unsupported impl self type".to_owned());
    };
    if path.qself.is_some() {
        return Err("qualified impl self types are unsupported".to_owned());
    }
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .ok_or_else(|| "impl self type has no identity".to_owned())
}

fn inherent_owner_module(
    type_owners: &BTreeMap<String, BTreeSet<String>>,
    source_module: &str,
    owner: &str,
) -> GateResult<String> {
    let owners = type_owners.get(owner).ok_or_else(|| {
        format!("inherent impl owner has no admitted nominal type: {source_module}::{owner}")
    })?;
    if owners.contains(source_module) {
        return Ok(source_module.to_owned());
    }
    if owners.len() == 1 {
        return owners
            .iter()
            .next()
            .cloned()
            .ok_or_else(|| format!("inherent impl owner set is empty: {owner}"));
    }
    Err(format!(
        "cross-module inherent impl owner is ambiguous: {source_module}::{owner} candidates={owners:?}"
    ))
}

impl SourceWalker<'_> {
    fn walk(mut self) -> GateResult<WalkOutput> {
        let root = self.package.root.clone();
        let module_dir = self.module_dir.clone();
        let module_path = self.package.crate_name.clone();
        self.walk_file(&root, &module_dir, &module_path)?;
        self.resolve_inherent_methods()?;
        Ok((self.modules, self.functions, self.visited))
    }

    fn add_type_owner(&mut self, owner: &Ident, module_path: &str) -> GateResult<()> {
        let owner = owner.to_string();
        let owners = self.type_owners.entry(owner.clone()).or_default();
        if !owners.insert(module_path.to_owned()) {
            return Err(format!(
                "duplicate nominal type definition: {module_path}::{owner}"
            ));
        }
        Ok(())
    }

    fn resolve_inherent_methods(&mut self) -> GateResult<()> {
        for method in std::mem::take(&mut self.inherent_methods) {
            let owner_module =
                inherent_owner_module(&self.type_owners, &method.source_module, &method.owner)?;
            self.add_function(
                &method.source,
                &format!("{owner_module}::{}::{}", method.owner, method.method),
                method.verified,
            )?;
        }
        Ok(())
    }

    fn walk_file(&mut self, path: &Path, module_dir: &Path, module_path: &str) -> GateResult<()> {
        let path = canonical(path)?;
        if !path.starts_with(self.repo) || !path.starts_with(&self.source_root) {
            return Err(format!(
                "module source escapes admitted root: {}",
                path.display()
            ));
        }
        if !self.visited.insert(path.clone()) {
            return Err(format!(
                "module source is included more than once: {}",
                path.display()
            ));
        }
        let relative = relative_source(self.repo, &path)?;
        self.modules.insert(
            relative.clone(),
            (self.package.name.clone(), module_path.to_owned()),
        );
        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let file = verus_syn::parse_file(&source).map_err(|error| {
            format!(
                "cannot parse {} with pinned verus_syn: {error}",
                path.display()
            )
        })?;
        validate_attributes(&file.attrs, false)?;
        self.walk_items(&file.items, &relative, module_dir, module_path, false)
    }

    fn walk_items(
        &mut self,
        items: &[Item],
        source: &str,
        module_dir: &Path,
        module_path: &str,
        in_verus: bool,
    ) -> GateResult<()> {
        for item in items {
            if cfg_test_item(item) {
                continue;
            }
            if self.package.name == "ferric-build" && cfg_test_fixture_item(item)? {
                continue;
            }
            match item {
                Item::Macro(item_macro)
                    if item_macro.ident.is_none()
                        && item_macro.mac.path.segments.len() == 1
                        && item_macro.mac.path.is_ident("verus") =>
                {
                    if in_verus {
                        return Err("nested verus! blocks are forbidden".to_owned());
                    }
                    validate_attributes(&item_macro.attrs, false)?;
                    let inner: File = verus_syn::parse2(item_macro.mac.tokens.clone())
                        .map_err(|error| format!("cannot parse verus! body: {error}"))?;
                    validate_attributes(&inner.attrs, true)?;
                    self.walk_items(&inner.items, source, module_dir, module_path, true)?;
                }
                Item::Macro(item_macro) => {
                    return Err(format!(
                        "item macro invocation is forbidden: {}!",
                        path_name(&item_macro.mac.path)
                    ));
                }
                Item::Mod(module) => {
                    if in_verus {
                        return Err("module declarations inside verus! are forbidden".to_owned());
                    }
                    validate_attributes(&module.attrs, false)?;
                    let child_name = module.ident.to_string();
                    if child_name.starts_with("r#") {
                        return Err(format!("raw module identifier is forbidden: {child_name}"));
                    }
                    let child_path = format!("{module_path}::{child_name}");
                    let child_dir = module_dir.join(&child_name);
                    if let Some((_, items)) = &module.content {
                        self.walk_items(items, source, &child_dir, &child_path, false)?;
                    } else {
                        let flat = module_dir.join(format!("{child_name}.rs"));
                        let nested = child_dir.join("mod.rs");
                        let flat_exists = flat.is_file();
                        let nested_exists = nested.is_file();
                        if flat_exists == nested_exists {
                            return Err(format!(
                                "module {child_path} must resolve to exactly one source file"
                            ));
                        }
                        self.walk_file(
                            if flat_exists { &flat } else { &nested },
                            &child_dir,
                            &child_path,
                        )?;
                    }
                }
                Item::Fn(function) => {
                    audit_item(item, in_verus, source)?;
                    let executable = validate_signature(&function.sig)?;
                    if executable {
                        let compiler_path = format!("{module_path}::{}", function.sig.ident);
                        if in_verus && self.package.name == "ferric-engine" {
                            audit_engine_allocation(&function.block, &compiler_path)?;
                        }
                        self.add_function(source, &compiler_path, in_verus)?;
                    }
                }
                Item::Enum(item_enum) => {
                    audit_item(item, in_verus, source)?;
                    self.add_type_owner(&item_enum.ident, module_path)?;
                }
                Item::Impl(item_impl) => {
                    audit_item(item, in_verus, source)?;
                    let owner = impl_owner(item_impl)?;
                    let trait_name = item_impl
                        .trait_
                        .as_ref()
                        .and_then(|(_, path, _)| path.segments.last())
                        .map(|segment| segment.ident.to_string());
                    for impl_item in &item_impl.items {
                        match impl_item {
                            ImplItem::Fn(function) => {
                                let executable = validate_signature(&function.sig)?;
                                if executable {
                                    if in_verus && trait_name.is_some() {
                                        return Err(format!(
                                            "verified trait implementation methods are unsupported: {owner}::{}",
                                            function.sig.ident
                                        ));
                                    }
                                    let method = function.sig.ident.to_string();
                                    let qualifier = trait_name.as_ref().map_or_else(
                                        || owner.clone(),
                                        |name| format!("{owner}::{name}"),
                                    );
                                    let compiler_path =
                                        format!("{module_path}::{qualifier}::{method}");
                                    if in_verus && self.package.name == "ferric-engine" {
                                        audit_engine_allocation(&function.block, &compiler_path)?;
                                    }
                                    if trait_name.is_none() {
                                        self.inherent_methods.push(PendingInherentMethod {
                                            source: source.to_owned(),
                                            source_module: module_path.to_owned(),
                                            owner: owner.clone(),
                                            method,
                                            verified: in_verus,
                                        });
                                    } else {
                                        self.add_function(source, &compiler_path, in_verus)?;
                                    }
                                }
                            }
                            ImplItem::Macro(macro_item) => {
                                return Err(format!(
                                    "impl item macro is forbidden: {}!",
                                    path_name(&macro_item.mac.path)
                                ));
                            }
                            ImplItem::Verbatim(_) => {
                                return Err("unparsed impl item syntax is forbidden".to_owned());
                            }
                            _ => {}
                        }
                    }
                }
                Item::Struct(item_struct) => {
                    audit_item(item, in_verus, source)?;
                    self.add_type_owner(&item_struct.ident, module_path)?;
                }
                Item::Trait(item_trait) => {
                    audit_item(item, in_verus, source)?;
                    for trait_item in &item_trait.items {
                        match trait_item {
                            TraitItem::Fn(function) => {
                                let executable = validate_signature(&function.sig)?;
                                if executable && function.default.is_some() {
                                    if in_verus {
                                        return Err(format!(
                                            "verified trait default methods are unsupported: {}::{}",
                                            item_trait.ident, function.sig.ident
                                        ));
                                    }
                                    self.add_function(
                                        source,
                                        &format!(
                                            "{module_path}::{}::{}",
                                            item_trait.ident, function.sig.ident
                                        ),
                                        false,
                                    )?;
                                }
                            }
                            TraitItem::Macro(macro_item) => {
                                return Err(format!(
                                    "trait item macro is forbidden: {}!",
                                    path_name(&macro_item.mac.path)
                                ));
                            }
                            TraitItem::Verbatim(_) => {
                                return Err("unparsed trait item syntax is forbidden".to_owned());
                            }
                            _ => {}
                        }
                    }
                }
                Item::Union(item_union) => {
                    audit_item(item, in_verus, source)?;
                    self.add_type_owner(&item_union.ident, module_path)?;
                }
                Item::ForeignMod(_) => return Err("foreign modules are forbidden".to_owned()),
                Item::AssumeSpecification(_) => {
                    return Err("assume_specification is forbidden".to_owned());
                }
                Item::Verbatim(_) => return Err("unparsed item syntax is forbidden".to_owned()),
                _ => audit_item(item, in_verus, source)?,
            }
        }
        Ok(())
    }

    fn add_function(
        &mut self,
        source: &str,
        compiler_path: &str,
        verified: bool,
    ) -> GateResult<()> {
        if self.functions.iter().any(|function| {
            function.package == self.package.name && function.compiler_path == compiler_path
        }) {
            return Err(format!(
                "duplicate executable compiler path in package {}: {compiler_path}",
                self.package.name
            ));
        }
        if !self.functions.insert(Function {
            package: self.package.name.clone(),
            source: source.to_owned(),
            compiler_path: compiler_path.to_owned(),
            verified,
        }) {
            return Err(format!(
                "duplicate executable function identity: {compiler_path}"
            ));
        }
        Ok(())
    }
}

fn collect_rs_files(root: &Path, output: &mut BTreeSet<PathBuf>) -> GateResult<()> {
    for entry in fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))? {
        let entry = entry.map_err(|error| format!("{}: {error}", root.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "source symlink is forbidden: {}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            collect_rs_files(&entry.path(), output)?;
        } else if file_type.is_file() && entry.path().extension().is_some_and(|ext| ext == "rs") {
            output.insert(canonical(&entry.path())?);
        } else if !file_type.is_file() {
            return Err(format!(
                "special source entry is forbidden: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn target_module_dir(root: &Path) -> GateResult<PathBuf> {
    root.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("target root has no parent: {}", root.display()))
}

fn inventory(repo: &Path, metadata: &Value) -> GateResult<Inventory> {
    let runtime_tcb = validate_runtime_dependency_tcb(repo, metadata)?;
    let packages = packages(repo, metadata, &runtime_tcb.roots)?;
    let mut inventory = Inventory {
        packages: packages.clone(),
        runtime_tcb: runtime_tcb.text.lines().map(str::to_owned).collect(),
        ..Inventory::default()
    };
    for package in &packages {
        let source_root = package
            .root
            .parent()
            .ok_or_else(|| format!("package {} source root has no parent", package.name))?
            .to_owned();
        let mut targets = vec![PackageTarget {
            crate_name: package.crate_name.clone(),
            root: package.root.clone(),
        }];
        targets.extend(package.additional_targets.iter().cloned());
        let mut visited = BTreeSet::new();
        for target in targets {
            let module_dir = target_module_dir(&target.root)?;
            let target_package = Package {
                name: package.name.clone(),
                crate_name: target.crate_name,
                root: target.root,
                dependencies: package.dependencies.clone(),
                additional_targets: Vec::new(),
            };
            let walker = SourceWalker {
                repo,
                package: &target_package,
                source_root: source_root.clone(),
                module_dir,
                visited: BTreeSet::new(),
                modules: BTreeMap::new(),
                functions: BTreeSet::new(),
                type_owners: BTreeMap::new(),
                inherent_methods: Vec::new(),
            };
            let (modules, functions, target_visited) = walker.walk()?;
            for (source, owner) in modules {
                if inventory.modules.insert(source.clone(), owner).is_some() {
                    return Err(format!("source belongs to multiple packages: {source}"));
                }
            }
            for function in functions {
                if !inventory.functions.insert(function.clone()) {
                    return Err(format!(
                        "executable compiler path belongs to multiple targets: {}",
                        function.compiler_path
                    ));
                }
            }
            visited.extend(target_visited);
        }
        let mut all_rs = BTreeSet::new();
        collect_rs_files(&source_root, &mut all_rs)?;
        let orphaned: Vec<String> = all_rs
            .difference(&visited)
            .map(|path| relative_source(repo, path))
            .collect::<GateResult<_>>()?;
        if !orphaned.is_empty() {
            return Err(format!(
                "package {} contains unreachable Rust source: {orphaned:?}",
                package.name
            ));
        }
    }
    Ok(inventory)
}

fn validate_unverified_admissions(repo: &Path, inventory: &Inventory) -> GateResult<()> {
    let path = repo.join("proofs/UNVERIFIED_BODIES");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut lines = text.lines();
    if lines.next() != Some("format=FERRIC-UNVERIFIED-BODIES-V1") {
        return Err("unsupported unverified executable body admission format".to_owned());
    }
    let mut admitted = BTreeSet::new();
    for line in lines {
        let fields: Vec<&str> = line
            .strip_prefix("unverified=")
            .ok_or_else(|| format!("malformed unverified body admission: {line:?}"))?
            .split('|')
            .collect();
        let [package, source, compiler_path, status, rationale] = fields.as_slice() else {
            return Err(format!("malformed unverified body admission: {line:?}"));
        };
        safe_atom(status, "unverified body status")?;
        safe_atom(rationale, "unverified body rationale")?;
        if !matches!(*status, "pending-verus" | "excluded-presentation") {
            return Err(format!("unsupported unverified body status: {status}"));
        }
        let identity = (
            (*package).to_owned(),
            (*source).to_owned(),
            (*compiler_path).to_owned(),
        );
        if !admitted.insert(identity) {
            return Err(format!(
                "duplicate unverified executable body admission: {compiler_path}"
            ));
        }
    }
    let discovered: BTreeSet<(String, String, String)> = inventory
        .functions
        .iter()
        .filter(|function| !function.verified)
        .map(|function| {
            (
                function.package.clone(),
                function.source.clone(),
                function.compiler_path.clone(),
            )
        })
        .collect();
    if admitted != discovered {
        let unadmitted: Vec<_> = discovered.difference(&admitted).cloned().collect();
        let stale: Vec<_> = admitted.difference(&discovered).cloned().collect();
        return Err(format!(
            "unverified executable body admission drifted (unadmitted={unadmitted:?}, stale={stale:?})"
        ));
    }
    Ok(())
}

fn render(inventory: &Inventory) -> String {
    let mut lines = vec![format!("format={FORMAT}")];
    for line in &inventory.runtime_tcb {
        lines.push(format!("runtime-tcb={line}"));
    }
    for package in &inventory.packages {
        lines.push(format!("package={}|{}", package.name, package.crate_name));
    }
    for (source, (package, module_path)) in &inventory.modules {
        lines.push(format!("module={package}|{source}|{module_path}"));
        for function in inventory
            .functions
            .iter()
            .filter(|function| function.source == *source)
        {
            let status = if function.verified {
                "verified"
            } else {
                "unverified"
            };
            lines.push(format!(
                "{status}={}|{}|{}",
                function.package, function.source, function.compiler_path
            ));
        }
    }
    lines.join("\n") + "\n"
}

fn render_dependency_tcb(repo: &Path, metadata: &Value) -> GateResult<String> {
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "source-gate Cargo metadata has no packages array".to_owned())?;
    if packages.is_empty() {
        return Err("source-gate dependency closure is empty".to_owned());
    }
    let mut records = BTreeSet::new();
    for package in packages {
        let name = string_field(package, "name")?;
        let version = string_field(package, "version")?;
        safe_atom(name, "source-gate dependency name")?;
        safe_atom(version, "source-gate dependency version")?;
        let source = package
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("first-party");
        if source.contains(['\n', '\r', '|']) {
            return Err(format!("unsafe source-gate dependency source: {source:?}"));
        }
        let mut build_scripts = Vec::new();
        for target in package
            .get("targets")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("source-gate dependency {name} has no targets array"))?
        {
            let is_build_script =
                target
                    .get("kind")
                    .and_then(Value::as_array)
                    .is_some_and(|kinds| {
                        kinds
                            .iter()
                            .any(|kind| kind.as_str() == Some("custom-build"))
                    });
            if is_build_script {
                let target_name = string_field(target, "name")?;
                safe_atom(target_name, "source-gate build script target")?;
                build_scripts.push(target_name);
            }
        }
        build_scripts.sort_unstable();
        let scripts = if build_scripts.is_empty() {
            "none".to_owned()
        } else {
            build_scripts.join(",")
        };
        records.insert(format!("package={name}|{version}|{source}|{scripts}"));
    }
    let mut lines = vec!["format=FERRIC-SOURCE-GATE-TCB-V1".to_owned()];
    lines.extend(records);
    let runtime_path = repo.join(RUNTIME_TCB_PATH);
    let runtime = fs::read_to_string(&runtime_path)
        .map_err(|error| format!("{}: {error}", runtime_path.display()))?;
    if !runtime.starts_with(&format!("format={RUNTIME_TCB_FORMAT}\n")) {
        return Err("unsupported workspace runtime dependency TCB format".to_owned());
    }
    lines.extend(runtime.lines().map(|line| format!("runtime-tcb={line}")));
    Ok(lines.join("\n") + "\n")
}

fn run() -> GateResult<()> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if let [flag, repo_arg, metadata_arg, output_arg] = arguments.as_slice() {
        if flag == "--dependency-tcb" {
            let repo = canonical(Path::new(repo_arg))?;
            let rendered = render_dependency_tcb(&repo, &read_json(Path::new(metadata_arg))?)?;
            fs::write(output_arg, rendered).map_err(|error| format!("{output_arg}: {error}"))?;
            println!("PASS: generated source-gate dependency TCB at {output_arg}");
            return Ok(());
        }
        if flag == "--runtime-dependency-tcb" {
            let repo = canonical(Path::new(repo_arg))?;
            let rendered =
                render_runtime_dependency_tcb(&repo, &read_json(Path::new(metadata_arg))?)?;
            fs::write(output_arg, rendered.text)
                .map_err(|error| format!("{output_arg}: {error}"))?;
            println!("PASS: generated workspace runtime dependency TCB at {output_arg}");
            return Ok(());
        }
    }
    let (generate, repo_arg, manifest_arg, metadata_arg) = match arguments.as_slice() {
        [flag, repo, metadata, output] if flag == "--generate" => {
            (true, repo.as_str(), output.as_str(), metadata.as_str())
        }
        [repo, manifest, metadata] => (false, repo.as_str(), manifest.as_str(), metadata.as_str()),
        _ => {
            return Err(
                "usage: ferric-source-gate REPO MANIFEST METADATA\n       ferric-source-gate --generate REPO METADATA OUTPUT\n       ferric-source-gate --runtime-dependency-tcb REPO METADATA OUTPUT\n       ferric-source-gate --dependency-tcb REPO METADATA OUTPUT"
                    .to_owned(),
            );
        }
    };
    let repo = canonical(Path::new(repo_arg))?;
    let metadata = read_json(Path::new(metadata_arg))?;
    let inventory = inventory(&repo, &metadata)?;
    validate_unverified_admissions(&repo, &inventory)?;
    let rendered = render(&inventory);
    let manifest = Path::new(manifest_arg);
    if generate {
        fs::write(manifest, rendered)
            .map_err(|error| format!("{}: {error}", manifest.display()))?;
        println!(
            "PASS: generated compiler-rooted coverage manifest at {}",
            manifest.display()
        );
    } else {
        let admitted = fs::read_to_string(manifest)
            .map_err(|error| format!("{}: {error}", manifest.display()))?;
        if admitted != rendered {
            return Err("compiler-rooted proof coverage manifest drifted".to_owned());
        }
        let module_count = rendered
            .lines()
            .filter(|line| line.starts_with("module="))
            .count();
        let function_count = rendered
            .lines()
            .filter(|line| line.starts_with("verified=") || line.starts_with("unverified="))
            .count();
        println!(
            "PASS: compiler-rooted source coverage matched ({module_count} modules, {function_count} executable bodies)"
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        fail(error);
    }
}

#[cfg(test)]
mod tests {
    use super::{inherent_owner_module, target_module_dir};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    fn owners(records: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
        records
            .iter()
            .map(|(owner, modules)| {
                (
                    (*owner).to_owned(),
                    modules.iter().map(|module| (*module).to_owned()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn inherent_owner_uses_the_defining_module() {
        let type_owners = owners(&[("Role", &["crate_name::configuration"])]);
        assert_eq!(
            inherent_owner_module(&type_owners, "crate_name::qwen3", "Role"),
            Ok("crate_name::configuration".to_owned())
        );
        assert_eq!(
            inherent_owner_module(&type_owners, "crate_name::configuration", "Role"),
            Ok("crate_name::configuration".to_owned())
        );
    }

    #[test]
    fn inherent_owner_rejects_missing_or_ambiguous_types() {
        let type_owners = owners(&[("Role", &["crate_name::a", "crate_name::b"])]);
        assert!(inherent_owner_module(&type_owners, "crate_name::c", "Role").is_err());
        assert!(inherent_owner_module(&type_owners, "crate_name::c", "Missing").is_err());
    }

    #[test]
    fn target_modules_resolve_from_each_cargo_target_root() {
        assert_eq!(
            target_module_dir(Path::new("crate/src/lib.rs")),
            Ok(PathBuf::from("crate/src"))
        );
        assert_eq!(
            target_module_dir(Path::new("crate/src/bin/tool.rs")),
            Ok(PathBuf::from("crate/src/bin"))
        );
    }
}
