#![forbid(unsafe_code)]

use fe2o3_proof_contracts::{
    ArtifactIdentityV1, CheckedEvidenceV1, ContractSetV1, ContractedEvidenceV1,
    CorrespondenceIdentityV1, CorrespondenceKindV1, CorrespondenceReferenceV1, DigestV1,
    EvidenceBindingV1, EvidenceIdentityV1, ExactInputIdentityV1, ExactModelIdentityV1,
    ExactToolIdentityV1, ObligationIdentityV1, ObligationRecordV1, ObligationSatisfactionV1,
    PropertyEvidenceV1, PropertyIdentityV1, PropertyKindV1, PropertyRecordV1, PropertyStatusV1,
    ProvedEvidenceV1, StatementIdentityV1, TcbEntryIdentityV1, TcbEntryKindV1, TcbEntryV1,
    UnsupportedEvidenceV1, UnsupportedReasonV1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORMAT: &str = "ferric.m0-property-manifest.v1";
const FE2O3_COMMIT: &str = "a6fa86b5ccf8f0438925cfec8f48a5d713874da3";
const FE2O3_SOURCE: &str = "git+https://github.com/harsh-nod/fe2o3.git?rev=a6fa86b5ccf8f0438925cfec8f48a5d713874da3#a6fa86b5ccf8f0438925cfec8f48a5d713874da3";
const MACHINE_NAMESPACE: &str = "harsh-nod.ferric.machine_refined.v1";
const MACHINE_CODE: u16 = 1;
const EVIDENCE_FORMAT: &str = "FERRIC-M0-EVIDENCE-INDEX-V1";
const ARTIFACT_FORMAT: &str = "FERRIC-M0-PROPERTY-CONTRACT-V1";

type BinderResult<T> = Result<T, String>;

#[derive(Clone, Copy)]
struct ExpectedProperty {
    name: &'static str,
    kind: &'static str,
    status: &'static str,
}

const EXPECTED_PROPERTIES: &[ExpectedProperty] = &[
    ExpectedProperty {
        name: "m0.request_generation",
        kind: "GenerationSafety",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.greedy_speculation",
        kind: "FunctionalCorrectness",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.scheduler_transition",
        kind: "FunctionalCorrectness",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.scheduler_lifetime",
        kind: "LeaseSafety",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.scheduler_bounds",
        kind: "ResourceBounds",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.kv_transition",
        kind: "FunctionalCorrectness",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.kv_sharing_rollback",
        kind: "FunctionalCorrectness",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.kv_generation",
        kind: "GenerationSafety",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.kv_bounds",
        kind: "ResourceBounds",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.engine_composition",
        kind: "FunctionalCorrectness",
        status: "Proved",
    },
    ExpectedProperty {
        name: "m0.hsa_exact_completion",
        kind: "SynchronizationSafety",
        status: "Contracted",
    },
    ExpectedProperty {
        name: "m0.proof_erasure",
        kind: "ProofErasureCorrespondence",
        status: "Checked",
    },
    ExpectedProperty {
        name: "m0.no_transition_allocation",
        kind: "ResourceBounds",
        status: "Checked",
    },
    ExpectedProperty {
        name: "m0.device_kv_initialization",
        kind: "MemorySafety",
        status: "Unsupported",
    },
    ExpectedProperty {
        name: "m0.machine_refinement",
        kind: "machine_refined",
        status: "Unsupported",
    },
];

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    fe2o3_commit: String,
    machine_extension_namespace: String,
    machine_extension_code: u16,
    properties: Vec<ManifestProperty>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestProperty {
    name: String,
    kind: String,
    required_status: String,
    statement: String,
    statement_sha256: String,
    compiler_paths_resolved: bool,
    compiler_path_prefixes: Vec<String>,
    required_mutations: Vec<String>,
    checked_evidence: Vec<String>,
    unsupported_reason: Option<String>,
}

#[derive(Clone, Copy)]
struct Measurement {
    size: u64,
    digest: [u8; 32],
}

#[derive(Clone)]
struct MutationMeasurement {
    mutator: [u8; 32],
    package: String,
    module: String,
    function: String,
    compiler_path: String,
    marker: Measurement,
    transcript: Measurement,
}

struct RegisteredMutation {
    mutator: PathBuf,
    failure_marker: String,
    package: String,
    module: String,
    function: String,
    compiler_path: String,
}

#[derive(Default)]
struct EvidenceInventory {
    files: BTreeMap<String, Measurement>,
    tools: BTreeMap<String, Measurement>,
    host_tools: BTreeMap<String, Measurement>,
    artifacts: BTreeMap<String, Measurement>,
    negative: BTreeMap<String, Measurement>,
    mutations: BTreeMap<String, MutationMeasurement>,
}

struct EvidencePaths<'a> {
    repo: &'a Path,
    source_records: &'a Path,
    proof_transcript: &'a Path,
    proof_counts: &'a Path,
    negative_dir: &'a Path,
    verus_root: &'a Path,
    source_gate: &'a Path,
    artifact_dir: &'a Path,
    runtime_tests: &'a Path,
}

#[derive(Clone)]
struct VerifiedInventory {
    packages: BTreeMap<String, String>,
    paths: Vec<(String, String)>,
}

struct BuiltProperty {
    record: PropertyRecordV1,
    obligation: ObligationRecordV1,
    correspondence: Option<CorrespondenceReferenceV1>,
    render: String,
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("FAIL: {}", message.as_ref());
    std::process::exit(1);
}

fn safe_name(value: &str, description: &str) -> BinderResult<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!("unsafe {description}: {value:?}"));
    }
    Ok(())
}

fn safe_compiler_prefix(value: &str) -> BinderResult<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
        || !value.contains("::")
    {
        return Err(format!("unsafe compiler path prefix: {value:?}"));
    }
    Ok(())
}

fn safe_verus_target(value: &str, description: &str) -> BinderResult<()> {
    let mut components = value.split("::");
    let valid = !value.is_empty()
        && components.all(|component| {
            let mut bytes = component.bytes();
            bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        });
    if !valid {
        return Err(format!("unsafe {description}: {value:?}"));
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn domain_digest(domain: &str, parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain.as_bytes());
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(64);
    for byte in bytes {
        result.push(char::from(DIGITS[usize::from(byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    result
}

fn digest_v1(bytes: [u8; 32]) -> DigestV1 {
    DigestV1::from_untrusted_bytes(bytes)
}

fn measure(path: &Path, allow_symlink: bool) -> BinderResult<Measurement> {
    let link_metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if link_metadata.file_type().is_symlink() && !allow_symlink {
        return Err(format!("evidence symlink is forbidden: {}", path.display()));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", path.display()))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("cannot inspect {}: {error}", canonical.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(format!(
            "evidence must be a nonempty regular file: {}",
            canonical.display()
        ));
    }
    let bytes = fs::read(&canonical)
        .map_err(|error| format!("cannot read {}: {error}", canonical.display()))?;
    Ok(Measurement {
        size: metadata.len(),
        digest: sha256_bytes(&bytes),
    })
}

fn canonical_json(path: &Path) -> BinderResult<Manifest> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let mut canonical = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("cannot canonicalize manifest: {error}"))?;
    canonical.push(b'\n');
    if bytes != canonical {
        return Err("M0 property manifest is not canonical JSON".to_owned());
    }
    Ok(manifest)
}

fn strip_ticks(value: &str) -> String {
    value
        .strip_prefix('`')
        .and_then(|inner| inner.strip_suffix('`'))
        .unwrap_or(value)
        .to_owned()
}

fn doc_properties(path: &Path) -> BinderResult<BTreeMap<String, (String, String, String)>> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut properties = BTreeMap::new();
    for line in text.lines().filter(|line| line.starts_with("| `m0.")) {
        let fields: Vec<&str> = line.split('|').map(str::trim).collect();
        if fields.len() != 6 {
            return Err(format!("malformed M0 property documentation row: {line}"));
        }
        let name = strip_ticks(fields[1]);
        let kind = if fields[2].contains("machine_refined") {
            "machine_refined".to_owned()
        } else {
            strip_ticks(fields[2])
        };
        let status = strip_ticks(fields[3]);
        let statement = fields[4].to_owned();
        if properties
            .insert(name.clone(), (kind, status, statement))
            .is_some()
        {
            return Err(format!("duplicate documented property: {name}"));
        }
    }
    Ok(properties)
}

fn unique(values: &[String], description: &str) -> BinderResult<()> {
    let set: BTreeSet<&String> = values.iter().collect();
    if set.len() != values.len() {
        return Err(format!("duplicate {description}"));
    }
    Ok(())
}

fn validate_manifest(repo: &Path, allow_unresolved: bool) -> BinderResult<Manifest> {
    let manifest_path = repo.join("proofs/M0_PROPERTIES.json");
    let docs_path = repo.join("docs/M0_PROPERTY_CONTRACT.md");
    let manifest = canonical_json(&manifest_path)?;
    if manifest.format != FORMAT
        || manifest.fe2o3_commit != FE2O3_COMMIT
        || manifest.machine_extension_namespace != MACHINE_NAMESPACE
        || manifest.machine_extension_code != MACHINE_CODE
    {
        return Err("M0 property manifest authority identity drifted".to_owned());
    }
    if manifest.properties.len() != EXPECTED_PROPERTIES.len() {
        return Err(format!(
            "M0 property roster has {} records, expected {}",
            manifest.properties.len(),
            EXPECTED_PROPERTIES.len()
        ));
    }
    let documented = doc_properties(&docs_path)?;
    if documented.len() != EXPECTED_PROPERTIES.len() {
        return Err("documented M0 property roster is incomplete".to_owned());
    }
    for (property, expected) in manifest.properties.iter().zip(EXPECTED_PROPERTIES) {
        safe_name(&property.name, "property name")?;
        if property.name != expected.name
            || property.kind != expected.kind
            || property.required_status != expected.status
        {
            return Err(format!(
                "required property kind/status/order drifted at {}",
                property.name
            ));
        }
        let Some((doc_kind, doc_status, doc_statement)) = documented.get(&property.name) else {
            return Err(format!(
                "property is absent from M0 documentation: {}",
                property.name
            ));
        };
        if doc_kind != &property.kind
            || doc_status != &property.required_status
            || doc_statement != &property.statement
        {
            return Err(format!(
                "manifest and documentation disagree for {}",
                property.name
            ));
        }
        if property.statement_sha256 != hex(&sha256_bytes(property.statement.as_bytes())) {
            return Err(format!("statement hash drifted for {}", property.name));
        }
        unique(&property.compiler_path_prefixes, "compiler path prefix")?;
        unique(&property.required_mutations, "required mutation")?;
        unique(&property.checked_evidence, "checked evidence")?;
        for prefix in &property.compiler_path_prefixes {
            safe_compiler_prefix(prefix)?;
        }
        for mutation in &property.required_mutations {
            safe_name(mutation, "mutation name")?;
        }
        match property.required_status.as_str() {
            "Proved" => {
                if property.required_mutations.is_empty()
                    || !property.checked_evidence.is_empty()
                    || property.unsupported_reason.is_some()
                {
                    return Err(format!(
                        "invalid Proved evidence declaration: {}",
                        property.name
                    ));
                }
                if property.compiler_paths_resolved {
                    if property.compiler_path_prefixes.is_empty() {
                        return Err(format!(
                            "resolved Proved property has no path: {}",
                            property.name
                        ));
                    }
                } else if !allow_unresolved {
                    return Err(format!(
                        "Proved compiler paths remain unresolved: {}",
                        property.name
                    ));
                } else if !property.compiler_path_prefixes.is_empty() {
                    return Err(format!(
                        "unresolved Proved property contains compiler prefixes: {}",
                        property.name
                    ));
                }
            }
            "Checked" => {
                if property.checked_evidence.is_empty()
                    || property.compiler_paths_resolved
                    || !property.compiler_path_prefixes.is_empty()
                    || !property.required_mutations.is_empty()
                    || property.unsupported_reason.is_some()
                {
                    return Err(format!(
                        "invalid Checked evidence declaration: {}",
                        property.name
                    ));
                }
            }
            "Contracted" => {
                if property.compiler_paths_resolved
                    || !property.compiler_path_prefixes.is_empty()
                    || !property.required_mutations.is_empty()
                    || !property.checked_evidence.is_empty()
                    || property.unsupported_reason.is_some()
                {
                    return Err(format!(
                        "invalid Contracted evidence declaration: {}",
                        property.name
                    ));
                }
            }
            "Unsupported" => {
                if property.compiler_paths_resolved
                    || !property.compiler_path_prefixes.is_empty()
                    || !property.required_mutations.is_empty()
                    || !property.checked_evidence.is_empty()
                    || property.unsupported_reason.is_none()
                {
                    return Err(format!(
                        "invalid Unsupported evidence declaration: {}",
                        property.name
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "unknown property status: {}",
                    property.required_status
                ));
            }
        }
    }
    Ok(manifest)
}

fn parse_verified(path: &Path) -> BinderResult<VerifiedInventory> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut lines = text.lines();
    if lines.next() != Some("format=FERRIC-VERIFIED-MODULES-V2") {
        return Err("unsupported verified compiler-path manifest".to_owned());
    }
    let mut packages = BTreeMap::new();
    let mut paths = Vec::new();
    for line in lines {
        if let Some(record) = line.strip_prefix("package=") {
            let fields: Vec<&str> = record.split('|').collect();
            let [package, crate_name] = fields.as_slice() else {
                return Err(format!("malformed package record: {line}"));
            };
            safe_name(package, "package name")?;
            safe_name(crate_name, "crate name")?;
            if packages
                .insert((*package).to_owned(), (*crate_name).to_owned())
                .is_some()
            {
                return Err(format!("duplicate package record: {package}"));
            }
        } else if let Some(record) = line.strip_prefix("verified=") {
            let fields: Vec<&str> = record.split('|').collect();
            let [package, _source, compiler_path] = fields.as_slice() else {
                return Err(format!("malformed verified path record: {line}"));
            };
            safe_name(package, "verified package name")?;
            safe_compiler_prefix(compiler_path)?;
            paths.push(((*package).to_owned(), (*compiler_path).to_owned()));
        }
    }
    if packages.is_empty() || paths.is_empty() {
        return Err("verified compiler-path manifest is empty".to_owned());
    }
    paths.sort();
    if paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("verified compiler-path manifest contains duplicates".to_owned());
    }
    if paths
        .iter()
        .any(|(package, _)| !packages.contains_key(package))
    {
        return Err("verified compiler path cites an unknown package".to_owned());
    }
    Ok(VerifiedInventory { packages, paths })
}

fn executable(path: &Path) -> BinderResult<Measurement> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("cannot resolve tool {}: {error}", path.display()))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("cannot inspect tool {}: {error}", canonical.display()))?;
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(format!("tool is not executable: {}", canonical.display()));
    }
    measure(&canonical, true)
}

fn command_stdout(program: &str, arguments: &[&str]) -> BinderResult<String> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot execute {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{program} failed with {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("{program} emitted invalid UTF-8: {error}"))
}

fn find_tool(name: &str) -> BinderResult<PathBuf> {
    let path = env::var_os("PATH").ok_or_else(|| "PATH is unavailable".to_owned())?;
    env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| format!("qualification host tool is unavailable: {name}"))
}

fn fixed_files(paths: &EvidencePaths<'_>) -> Vec<(String, PathBuf)> {
    vec![
        (
            "m0-property-doc".to_owned(),
            paths.repo.join("docs/M0_PROPERTY_CONTRACT.md"),
        ),
        (
            "assurance-doc".to_owned(),
            paths.repo.join("docs/ASSURANCE.md"),
        ),
        (
            "property-manifest".to_owned(),
            paths.repo.join("proofs/M0_PROPERTIES.json"),
        ),
        (
            "verified-modules".to_owned(),
            paths.repo.join("proofs/VERIFIED_MODULES"),
        ),
        (
            "unverified-bodies".to_owned(),
            paths.repo.join("proofs/UNVERIFIED_BODIES"),
        ),
        (
            "runtime-cargo-lock".to_owned(),
            paths.repo.join("Cargo.lock"),
        ),
        (
            "source-gate-lock".to_owned(),
            paths.repo.join("proofs/source-gate/Cargo.lock"),
        ),
        (
            "source-gate-tcb".to_owned(),
            paths.repo.join("proofs/source-gate/DEPENDENCY_TCB"),
        ),
        (
            "property-binder-lock".to_owned(),
            paths.repo.join("proofs/property-binder/Cargo.lock"),
        ),
        (
            "property-binder-tcb".to_owned(),
            paths.repo.join("proofs/property-binder/DEPENDENCY_TCB"),
        ),
        (
            "verus-closure".to_owned(),
            paths.repo.join("proofs/verus/VERUS_CLOSURE_MANIFEST"),
        ),
        (
            "verus-version".to_owned(),
            paths.repo.join("proofs/verus/VERUS_VERSION"),
        ),
        (
            "negative-registry".to_owned(),
            paths.repo.join("proofs/negative/REQUIRED_COMPONENTS"),
        ),
        ("source-closure".to_owned(), paths.source_records.to_owned()),
        (
            "proof-transcript".to_owned(),
            paths.proof_transcript.to_owned(),
        ),
        ("proof-counts".to_owned(), paths.proof_counts.to_owned()),
        ("runtime-tests".to_owned(), paths.runtime_tests.to_owned()),
    ]
}

fn mutation_registry(
    repo: &Path,
    verified: &VerifiedInventory,
) -> BinderResult<BTreeMap<String, RegisteredMutation>> {
    let path = repo.join("proofs/negative/REQUIRED_COMPONENTS");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut lines = text.lines();
    if lines.next() != Some("format=FERRIC-NEGATIVE-COMPONENTS-V2") {
        return Err("unsupported negative component registry".to_owned());
    }
    let mut registry = BTreeMap::new();
    let mut names = BTreeSet::new();
    for line in lines {
        let (payload, enabled) = if let Some(payload) = line.strip_prefix("always=") {
            (payload, true)
        } else if let Some(payload) = line.strip_prefix("when-verus=") {
            let fields: Vec<&str> = payload.split('|').collect();
            let [source, ..] = fields.as_slice() else {
                return Err(format!("malformed conditional negative component: {line}"));
            };
            let source_path = repo.join(source);
            if source.is_empty()
                || source.starts_with('/')
                || source.split('/').any(|component| component == "..")
                || !source_path.is_file()
            {
                return Err(format!("unsafe conditional mutation source: {source}"));
            }
            let source_text = fs::read_to_string(&source_path)
                .map_err(|error| format!("{}: {error}", source_path.display()))?;
            let compact: String = source_text
                .chars()
                .filter(|char| !char.is_whitespace())
                .collect();
            (payload, compact.contains("verus!{"))
        } else {
            return Err(format!("malformed negative component: {line}"));
        };
        let fields: Vec<&str> = payload.split('|').collect();
        let (name, package, mutator, marker, module, function) = match fields.as_slice() {
            [name, package, mutator, marker, module, function] => {
                (*name, *package, *mutator, *marker, *module, *function)
            }
            [source, name, package, mutator, marker, module, function] => {
                if source.is_empty()
                    || source.starts_with('/')
                    || source.split('/').any(|component| component == "..")
                    || !repo.join(source).is_file()
                {
                    return Err(format!("unsafe conditional mutation source: {source}"));
                }
                (*name, *package, *mutator, *marker, *module, *function)
            }
            _ => return Err(format!("malformed negative component: {line}")),
        };
        safe_name(name, "negative component name")?;
        safe_name(package, "negative component package")?;
        safe_name(mutator, "negative mutator name")?;
        if !matches!(marker, "proof" | "no-cheating") {
            return Err(format!("unknown negative failure marker: {marker}"));
        }
        safe_verus_target(module, "Verus module target")?;
        safe_verus_target(function, "Verus function target")?;
        if !names.insert(name.to_owned()) {
            return Err(format!("duplicate negative component: {name}"));
        }
        if enabled {
            let crate_name = verified.packages.get(package).ok_or_else(|| {
                format!("mutation target package is not verified: {name}|{package}")
            })?;
            let module_prefix = format!("{crate_name}::{module}::");
            let candidates: Vec<&String> = verified
                .paths
                .iter()
                .filter_map(|(owner, path)| {
                    if owner != package {
                        return None;
                    }
                    let tail = path.strip_prefix(&module_prefix)?;
                    let exact = tail == function;
                    let unique_suffix =
                        !function.contains("::") && tail.rsplit("::").next() == Some(function);
                    (exact || unique_suffix).then_some(path)
                })
                .collect();
            let compiler_path = match candidates.as_slice() {
                [path] => (*path).clone(),
                [] => {
                    return Err(format!(
                        "mutation target matched no verified compiler path: {name}|{package}|{module}|{function}"
                    ));
                }
                _ => {
                    return Err(format!(
                        "mutation target is ambiguous: {name}|{package}|{module}|{function}"
                    ));
                }
            };
            registry.insert(
                name.to_owned(),
                RegisteredMutation {
                    mutator: repo.join("proofs/negative/components").join(mutator),
                    failure_marker: marker.to_owned(),
                    package: package.to_owned(),
                    module: module.to_owned(),
                    function: function.to_owned(),
                    compiler_path,
                },
            );
        }
    }
    if registry.is_empty() {
        return Err("negative component registry selected no mutations".to_owned());
    }
    Ok(registry)
}

fn exact_marker_field<'a>(text: &'a str, prefix: &str) -> BinderResult<&'a str> {
    let values: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .collect();
    match values.as_slice() {
        [value] if !value.is_empty() => Ok(value),
        _ => Err(format!("mutation evidence field is not exact: {prefix}")),
    }
}

fn marker_hash(
    repo: &Path,
    marker: &Path,
    expected: [u8; 32],
    registered: &RegisteredMutation,
) -> BinderResult<()> {
    let text = fs::read_to_string(marker)
        .map_err(|error| format!("cannot read mutation marker {}: {error}", marker.display()))?;
    let source = exact_marker_field(&text, "MUTATED_SOURCE=")?;
    let hash = exact_marker_field(&text, "MUTATOR_SHA256=")?;
    let package = exact_marker_field(&text, "VERUS_PACKAGE=")?;
    let module = exact_marker_field(&text, "VERUS_MODULE=")?;
    let function = exact_marker_field(&text, "VERUS_FUNCTION=")?;
    if source.starts_with('/')
        || source.split('/').any(|component| component == "..")
        || !repo.join(source).is_file()
        || hash != hex(&expected)
        || package != registered.package
        || module != registered.module
        || function != registered.function
    {
        return Err(format!(
            "mutation marker does not bind its source, mutator, and exact Verus target: {}",
            marker.display()
        ));
    }
    Ok(())
}

fn proof_transcript_has_error(text: &str) -> bool {
    text.contains("postcondition not satisfied")
        || text.contains("assertion failed")
        || text.lines().any(|line| {
            let Some((_, tail)) = line.split_once("verification results::") else {
                return false;
            };
            let Some((_, errors)) = tail.split_once(" verified,") else {
                return false;
            };
            let mut fields = errors.split_whitespace();
            fields
                .next()
                .and_then(|count| count.parse::<u64>().ok())
                .is_some_and(|count| count > 0)
                && fields.next() == Some("errors")
        })
}

fn mutation_transcript(transcript: &Path, registered: &RegisteredMutation) -> BinderResult<()> {
    let text = fs::read_to_string(transcript).map_err(|error| {
        format!(
            "cannot read mutation transcript {}: {error}",
            transcript.display()
        )
    })?;
    if exact_marker_field(&text, "VERUS_MODULE=")? != registered.module
        || exact_marker_field(&text, "VERUS_PACKAGE=")? != registered.package
        || exact_marker_field(&text, "VERUS_FUNCTION=")? != registered.function
    {
        return Err(format!(
            "mutation transcript does not bind its exact Verus target: {}",
            transcript.display()
        ));
    }
    let matched = match registered.failure_marker.as_str() {
        "proof" => proof_transcript_has_error(&text),
        "no-cheating" => text.contains("assume/admit not allowed with --no-cheating"),
        _ => false,
    };
    if !matched {
        return Err(format!(
            "mutation transcript does not contain its required failure marker: {}",
            transcript.display()
        ));
    }
    Ok(())
}

fn collect_negative(
    paths: &EvidencePaths<'_>,
    inventory: &mut EvidenceInventory,
    verified: &VerifiedInventory,
) -> BinderResult<()> {
    let registry = mutation_registry(paths.repo, verified)?;
    for entry in fs::read_dir(paths.negative_dir)
        .map_err(|error| format!("{}: {error}", paths.negative_dir.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(format!(
                "negative evidence must be a regular file: {}",
                entry.path().display()
            ));
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        safe_name(&name, "negative evidence filename")?;
        inventory
            .negative
            .insert(name, measure(&entry.path(), false)?);
    }
    for filename in inventory.negative.keys() {
        if let Some(name) = filename.strip_suffix(".mutation")
            && !registry.contains_key(name)
        {
            return Err(format!("unregistered mutation marker: {filename}"));
        }
    }
    for (name, registered) in registry {
        let marker = paths.negative_dir.join(format!("{name}.mutation"));
        if !marker.is_file() {
            return Err(format!(
                "required mutation evidence missing: {}",
                marker.display()
            ));
        }
        let transcript = paths.negative_dir.join(format!("{name}.transcript"));
        if !transcript.is_file() {
            return Err(format!(
                "required mutation evidence missing: {}",
                transcript.display()
            ));
        }
        let mutator_measurement = measure(&registered.mutator, false)?;
        marker_hash(paths.repo, &marker, mutator_measurement.digest, &registered)?;
        mutation_transcript(&transcript, &registered)?;
        let marker_measurement = measure(&marker, false)?;
        let transcript_measurement = measure(&transcript, false)?;
        inventory.mutations.insert(
            name,
            MutationMeasurement {
                mutator: mutator_measurement.digest,
                package: registered.package,
                module: registered.module,
                function: registered.function,
                compiler_path: registered.compiler_path,
                marker: marker_measurement,
                transcript: transcript_measurement,
            },
        );
    }
    Ok(())
}

fn collect_evidence(paths: &EvidencePaths<'_>) -> BinderResult<EvidenceInventory> {
    let mut inventory = EvidenceInventory::default();
    for (name, path) in fixed_files(paths) {
        inventory.files.insert(name, measure(&path, false)?);
    }
    for name in ["cargo-verus", "verus", "rust_verify", "z3"] {
        inventory
            .tools
            .insert(name.to_owned(), executable(&paths.verus_root.join(name))?);
    }
    inventory.tools.insert(
        "ferric-source-gate".to_owned(),
        executable(paths.source_gate)?,
    );
    let current_exe = env::current_exe().map_err(|error| error.to_string())?;
    inventory.tools.insert(
        "ferric-property-binder".to_owned(),
        executable(&current_exe)?,
    );
    let sysroot = command_stdout("rustc", &["--print", "sysroot"])?;
    for name in [
        "rustc",
        "cargo",
        "rustfmt",
        "cargo-fmt",
        "cargo-clippy",
        "clippy-driver",
    ] {
        inventory.tools.insert(
            name.to_owned(),
            executable(&Path::new(&sysroot).join("bin").join(name))?,
        );
    }
    for name in [
        "sh",
        "awk",
        "cat",
        "chmod",
        "cmp",
        "cp",
        "dirname",
        "grep",
        "mkdir",
        "mktemp",
        "python3",
        "rm",
        "sed",
        "sha256sum",
        "sort",
        "timeout",
        "tr",
        "uname",
    ] {
        inventory
            .host_tools
            .insert(name.to_owned(), executable(&find_tool(name)?)?);
    }
    let verified = parse_verified(&paths.repo.join("proofs/VERIFIED_MODULES"))?;
    let expected_artifacts: BTreeSet<String> = verified
        .packages
        .values()
        .map(|crate_name| format!("lib{}.rlib", crate_name.replace('-', "_")))
        .collect();
    for entry in fs::read_dir(paths.artifact_dir)
        .map_err(|error| format!("{}: {error}", paths.artifact_dir.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_symlink()
            || !entry.path().is_file()
            || !expected_artifacts.contains(&name)
        {
            return Err(format!(
                "unexpected qualified artifact: {}",
                entry.path().display()
            ));
        }
        inventory
            .artifacts
            .insert(name, measure(&entry.path(), false)?);
    }
    if inventory.artifacts.keys().cloned().collect::<BTreeSet<_>>() != expected_artifacts {
        return Err("qualified release artifact set is incomplete".to_owned());
    }
    let verified = parse_verified(&paths.repo.join("proofs/VERIFIED_MODULES"))?;
    collect_negative(paths, &mut inventory, &verified)?;
    Ok(inventory)
}

fn render_measurement(section: &str, name: &str, value: Measurement) -> String {
    format!("{section}={name}|{}|{}", value.size, hex(&value.digest))
}

fn render_evidence_index(inventory: &EvidenceInventory) -> String {
    let mut lines = vec![format!("format={EVIDENCE_FORMAT}")];
    lines.extend(
        inventory
            .files
            .iter()
            .map(|(name, value)| render_measurement("file", name, *value)),
    );
    lines.extend(
        inventory
            .tools
            .iter()
            .map(|(name, value)| render_measurement("tool", name, *value)),
    );
    lines.extend(
        inventory
            .host_tools
            .iter()
            .map(|(name, value)| render_measurement("host-tool", name, *value)),
    );
    lines.extend(
        inventory
            .artifacts
            .iter()
            .map(|(name, value)| render_measurement("artifact", name, *value)),
    );
    lines.extend(
        inventory
            .negative
            .iter()
            .map(|(name, value)| render_measurement("negative", name, *value)),
    );
    lines.extend(inventory.mutations.iter().map(|(name, value)| {
        format!(
            "mutation={name}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            hex(&value.mutator),
            value.package,
            value.module,
            value.function,
            value.compiler_path,
            value.marker.size,
            hex(&value.marker.digest),
            value.transcript.size,
            hex(&value.transcript.digest)
        )
    }));
    lines.join("\n") + "\n"
}

fn property_kind(name: &str) -> BinderResult<PropertyKindV1> {
    Ok(match name {
        "MemorySafety" => PropertyKindV1::MemorySafety,
        "SynchronizationSafety" => PropertyKindV1::SynchronizationSafety,
        "FunctionalCorrectness" => PropertyKindV1::FunctionalCorrectness,
        "ResourceBounds" => PropertyKindV1::ResourceBounds,
        "LeaseSafety" => PropertyKindV1::LeaseSafety,
        "GenerationSafety" => PropertyKindV1::GenerationSafety,
        "ProofErasureCorrespondence" => PropertyKindV1::ProofErasureCorrespondence,
        "machine_refined" => PropertyKindV1::Extension {
            namespace: digest_v1(domain_digest(
                "ferric.property-kind.extension.v1",
                &[MACHINE_NAMESPACE.as_bytes()],
            )),
            code: MACHINE_CODE,
        },
        _ => return Err(format!("unknown fe2o3 property kind: {name}")),
    })
}

fn property_status(name: &str) -> BinderResult<PropertyStatusV1> {
    Ok(match name {
        "Proved" => PropertyStatusV1::Proved,
        "Contracted" => PropertyStatusV1::Contracted,
        "Checked" => PropertyStatusV1::Checked,
        "Unsupported" => PropertyStatusV1::Unsupported,
        _ => return Err(format!("unknown fe2o3 property status: {name}")),
    })
}

fn unsupported_reason(name: &str) -> BinderResult<UnsupportedReasonV1> {
    Ok(match name {
        "OutsideDeclaredScope" => UnsupportedReasonV1::OutsideDeclaredScope,
        "UnresolvedCorrespondence" => UnsupportedReasonV1::UnresolvedCorrespondence,
        "machine_refined" => UnsupportedReasonV1::Extension {
            namespace: digest_v1(domain_digest(
                "ferric.unsupported-reason.extension.v1",
                &[MACHINE_NAMESPACE.as_bytes()],
            )),
            code: MACHINE_CODE,
        },
        _ => return Err(format!("unknown unsupported reason: {name}")),
    })
}

fn aggregate(domain: &str, values: impl IntoIterator<Item = [u8; 32]>) -> [u8; 32] {
    let values: Vec<[u8; 32]> = values.into_iter().collect();
    let references: Vec<&[u8]> = values.iter().map(<[u8; 32]>::as_slice).collect();
    domain_digest(domain, &references)
}

fn file_measurement(inventory: &EvidenceInventory, name: &str) -> BinderResult<Measurement> {
    inventory
        .files
        .get(name)
        .copied()
        .ok_or_else(|| format!("missing indexed file evidence: {name}"))
}

fn tool_measurement(inventory: &EvidenceInventory, name: &str) -> BinderResult<Measurement> {
    inventory
        .tools
        .get(name)
        .copied()
        .ok_or_else(|| format!("missing indexed tool evidence: {name}"))
}

fn artifact_identity(measurement: Measurement, format: &str) -> ArtifactIdentityV1 {
    ArtifactIdentityV1::new(
        digest_v1(measurement.digest),
        digest_v1(domain_digest(
            "ferric.artifact-format.v1",
            &[format.as_bytes()],
        )),
    )
}

fn aggregate_artifact(
    domain: &str,
    digests: impl IntoIterator<Item = [u8; 32]>,
) -> ArtifactIdentityV1 {
    ArtifactIdentityV1::new(
        digest_v1(aggregate(domain, digests)),
        digest_v1(domain_digest(
            "ferric.artifact-format.v1",
            &[domain.as_bytes()],
        )),
    )
}

fn exact_tool(
    inventory: &EvidenceInventory,
    executable: &str,
    configuration_domain: &str,
    configuration: impl IntoIterator<Item = [u8; 32]>,
) -> BinderResult<ExactToolIdentityV1> {
    Ok(ExactToolIdentityV1::new(
        digest_v1(tool_measurement(inventory, executable)?.digest),
        digest_v1(aggregate(configuration_domain, configuration)),
    ))
}

fn identity_hex(digest: DigestV1) -> String {
    hex(digest.as_bytes())
}

fn validate_counts(paths: &EvidencePaths<'_>, verified: &VerifiedInventory) -> BinderResult<()> {
    let counts = fs::read_to_string(paths.proof_counts)
        .map_err(|error| format!("{}: {error}", paths.proof_counts.display()))?;
    let transcript = fs::read_to_string(paths.proof_transcript)
        .map_err(|error| format!("{}: {error}", paths.proof_transcript.display()))?;
    let transcript_packages: Vec<&str> = transcript
        .lines()
        .filter_map(|line| line.strip_prefix("PACKAGE="))
        .collect();
    let transcript_package_set: BTreeSet<&str> = transcript_packages.iter().copied().collect();
    if transcript_packages.len() != transcript_package_set.len()
        || transcript_package_set != verified.packages.keys().map(String::as_str).collect()
    {
        return Err("proof transcript package attribution is not exact".to_owned());
    }
    let mut seen = BTreeSet::new();
    for line in counts.lines() {
        let fields: Vec<&str> = line.split('|').collect();
        let [package, queries, errors, direct] = fields.as_slice() else {
            return Err(format!("malformed proof count record: {line}"));
        };
        let queries: usize = queries
            .parse()
            .map_err(|_| format!("malformed verification query count: {line}"))?;
        let direct: usize = direct
            .parse()
            .map_err(|_| format!("malformed direct body count: {line}"))?;
        let expected = verified
            .paths
            .iter()
            .filter(|(owner, _)| owner == package)
            .count();
        if *errors != "0" || queries == 0 || direct == 0 || direct != expected || queries < direct {
            return Err(format!("weak proof count evidence: {line}"));
        }
        if !transcript_package_set.contains(package) || !seen.insert(*package) {
            return Err(format!(
                "proof transcript/package attribution drifted: {package}"
            ));
        }
    }
    if seen != verified.packages.keys().map(String::as_str).collect() {
        return Err("proof counts do not cover the exact verified package set".to_owned());
    }
    Ok(())
}

fn validate_checked_evidence(
    property: &ManifestProperty,
    paths: &EvidencePaths<'_>,
    inventory: &EvidenceInventory,
    verified: &VerifiedInventory,
) -> BinderResult<Vec<[u8; 32]>> {
    let runtime = fs::read_to_string(paths.runtime_tests)
        .map_err(|error| format!("{}: {error}", paths.runtime_tests.display()))?;
    if !runtime.contains("test result: ok.") || runtime.contains("FAILED") {
        return Err("runtime test transcript is not an all-pass result".to_owned());
    }
    for gate in ["fmt", "clippy", "test-debug", "test-release"] {
        let marker = format!("FERRIC_QUALITY_GATE={gate}:PASS");
        if runtime.lines().filter(|line| *line == marker).count() != 1 {
            return Err(format!(
                "quality gate transcript marker is not exact: {gate}"
            ));
        }
    }
    let mut digests = Vec::new();
    for evidence in &property.checked_evidence {
        if evidence == "proof-transcript" {
            digests.push(file_measurement(inventory, "proof-transcript")?.digest);
        } else if let Some(package) = evidence.strip_prefix("artifact:") {
            let crate_name = verified
                .packages
                .get(package)
                .ok_or_else(|| format!("checked evidence cites unknown package: {package}"))?;
            let name = format!("lib{}.rlib", crate_name.replace('-', "_"));
            digests.push(
                inventory
                    .artifacts
                    .get(&name)
                    .ok_or_else(|| format!("checked evidence artifact is absent: {name}"))?
                    .digest,
            );
        } else if let Some(name) = evidence.strip_prefix("policy:") {
            safe_name(name, "policy evidence name")?;
            let filename = format!("{name}.transcript");
            let measurement = inventory
                .negative
                .get(&filename)
                .ok_or_else(|| format!("checked policy evidence is absent: {filename}"))?;
            let transcript_path = paths.negative_dir.join(&filename);
            let transcript = fs::read_to_string(&transcript_path)
                .map_err(|error| format!("{}: {error}", transcript_path.display()))?;
            if transcript
                .lines()
                .filter(|line| *line == format!("FIXTURE={name}"))
                .count()
                != 1
                || !transcript.lines().any(|line| line.starts_with("FAIL:"))
            {
                return Err(format!("checked policy transcript is weak: {filename}"));
            }
            digests.push(measurement.digest);
        } else if let Some(name) = evidence.strip_prefix("runtime-test:") {
            safe_name(name, "runtime test evidence name")?;
            let matched = runtime.lines().any(|line| {
                line.trim()
                    .strip_prefix("test ")
                    .and_then(|value| value.strip_suffix(" ... ok"))
                    .and_then(|path| path.rsplit("::").next())
                    == Some(name)
            });
            if !matched {
                return Err(format!("required runtime test did not pass: {name}"));
            }
            digests.push(file_measurement(inventory, "runtime-tests")?.digest);
        } else {
            return Err(format!("unknown checked evidence selector: {evidence}"));
        }
    }
    Ok(digests)
}

fn property_paths(
    property: &ManifestProperty,
    verified: &VerifiedInventory,
) -> BinderResult<Vec<(String, String)>> {
    let mut matched = BTreeSet::new();
    for prefix in &property.compiler_path_prefixes {
        let prefix_matches: Vec<(String, String)> = verified
            .paths
            .iter()
            .filter(|(_, path)| {
                if prefix.ends_with("::") {
                    path.starts_with(prefix)
                } else {
                    path == prefix
                }
            })
            .cloned()
            .collect();
        if prefix_matches.is_empty() {
            return Err(format!(
                "compiler prefix for {} matched no admitted path: {prefix}",
                property.name
            ));
        }
        matched.extend(prefix_matches);
    }
    if matched.is_empty() {
        return Err(format!(
            "Proved property has no admitted path: {}",
            property.name
        ));
    }
    Ok(matched.into_iter().collect())
}

fn relevant_artifacts(
    property_paths: &[(String, String)],
    verified: &VerifiedInventory,
    inventory: &EvidenceInventory,
) -> BinderResult<Vec<[u8; 32]>> {
    let packages: BTreeSet<&String> = property_paths.iter().map(|(package, _)| package).collect();
    let mut digests = Vec::new();
    for package in packages {
        let crate_name = verified
            .packages
            .get(package)
            .ok_or_else(|| format!("verified path has unknown package: {package}"))?;
        let artifact = format!("lib{}.rlib", crate_name.replace('-', "_"));
        digests.push(
            inventory
                .artifacts
                .get(&artifact)
                .ok_or_else(|| format!("release artifact missing for {package}: {artifact}"))?
                .digest,
        );
    }
    Ok(digests)
}

fn sorted_references(mut references: Vec<TcbEntryIdentityV1>) -> Vec<TcbEntryIdentityV1> {
    references.sort();
    references
}

fn build_contract(
    repo: &Path,
    manifest: &Manifest,
    paths: &EvidencePaths<'_>,
    inventory: &EvidenceInventory,
    evidence_index: &[u8],
) -> BinderResult<String> {
    let verified = parse_verified(&repo.join("proofs/VERIFIED_MODULES"))?;
    validate_counts(paths, &verified)?;
    let proof_config = vec![
        tool_measurement(inventory, "verus")?.digest,
        tool_measurement(inventory, "rust_verify")?.digest,
        tool_measurement(inventory, "z3")?.digest,
        tool_measurement(inventory, "rustc")?.digest,
        tool_measurement(inventory, "cargo")?.digest,
        file_measurement(inventory, "runtime-cargo-lock")?.digest,
        file_measurement(inventory, "verus-closure")?.digest,
        file_measurement(inventory, "verus-version")?.digest,
        sha256_bytes(b"--locked --release --lib --no-cheating --output-json"),
    ];
    let proof_tool = exact_tool(
        inventory,
        "cargo-verus",
        "ferric.proof-tool-configuration.v1",
        proof_config,
    )?;
    let source_gate_tool = exact_tool(
        inventory,
        "ferric-source-gate",
        "ferric.source-gate-configuration.v1",
        [
            file_measurement(inventory, "source-gate-lock")?.digest,
            file_measurement(inventory, "source-gate-tcb")?.digest,
            file_measurement(inventory, "verified-modules")?.digest,
        ],
    )?;
    let binder_tool = exact_tool(
        inventory,
        "ferric-property-binder",
        "ferric.property-binder-configuration.v1",
        [
            file_measurement(inventory, "property-binder-lock")?.digest,
            file_measurement(inventory, "property-binder-tcb")?.digest,
            file_measurement(inventory, "property-manifest")?.digest,
        ],
    )?;

    let assurance = artifact_identity(
        file_measurement(inventory, "assurance-doc")?,
        "Ferric assurance policy Markdown",
    );
    let tcb_ids = BTreeMap::from([
        (
            "proof-tool",
            TcbEntryIdentityV1::from_untrusted_digest(digest_v1(domain_digest(
                "ferric.tcb-entry.v1",
                &[b"proof-tool"],
            ))),
        ),
        (
            "source-gate",
            TcbEntryIdentityV1::from_untrusted_digest(digest_v1(domain_digest(
                "ferric.tcb-entry.v1",
                &[b"source-gate"],
            ))),
        ),
        (
            "property-binder",
            TcbEntryIdentityV1::from_untrusted_digest(digest_v1(domain_digest(
                "ferric.tcb-entry.v1",
                &[b"property-binder"],
            ))),
        ),
        (
            "rust-toolchain",
            TcbEntryIdentityV1::from_untrusted_digest(digest_v1(domain_digest(
                "ferric.tcb-entry.v1",
                &[b"rust-toolchain"],
            ))),
        ),
        (
            "qualification-host",
            TcbEntryIdentityV1::from_untrusted_digest(digest_v1(domain_digest(
                "ferric.tcb-entry.v1",
                &[b"qualification-host"],
            ))),
        ),
    ]);
    let host_digest = aggregate(
        "ferric.qualification-host-tools.v1",
        inventory.host_tools.values().map(|value| value.digest),
    );
    let mut tcb = vec![
        TcbEntryV1 {
            identity: tcb_ids["proof-tool"],
            kind: TcbEntryKindV1::Tool,
            component: artifact_identity(tool_measurement(inventory, "cargo-verus")?, "executable"),
            exact_tool: Some(proof_tool),
            rationale: assurance,
        },
        TcbEntryV1 {
            identity: tcb_ids["source-gate"],
            kind: TcbEntryKindV1::Tool,
            component: artifact_identity(
                tool_measurement(inventory, "ferric-source-gate")?,
                "executable",
            ),
            exact_tool: Some(source_gate_tool),
            rationale: assurance,
        },
        TcbEntryV1 {
            identity: tcb_ids["property-binder"],
            kind: TcbEntryKindV1::Tool,
            component: artifact_identity(
                tool_measurement(inventory, "ferric-property-binder")?,
                "executable",
            ),
            exact_tool: Some(binder_tool),
            rationale: assurance,
        },
        TcbEntryV1 {
            identity: tcb_ids["rust-toolchain"],
            kind: TcbEntryKindV1::CompilerAssumption,
            component: aggregate_artifact(
                "ferric.rust-toolchain.v1",
                [
                    tool_measurement(inventory, "rustc")?.digest,
                    tool_measurement(inventory, "cargo")?.digest,
                ],
            ),
            exact_tool: None,
            rationale: assurance,
        },
        TcbEntryV1 {
            identity: tcb_ids["qualification-host"],
            kind: TcbEntryKindV1::RuntimeAssumption,
            component: aggregate_artifact("ferric.qualification-host.v1", [host_digest]),
            exact_tool: None,
            rationale: assurance,
        },
    ];
    tcb.sort_by_key(|entry| entry.identity);
    let proof_tcb = sorted_references(vec![
        tcb_ids["proof-tool"],
        tcb_ids["property-binder"],
        tcb_ids["rust-toolchain"],
        tcb_ids["qualification-host"],
    ]);
    let source_gate_tcb = sorted_references(vec![
        tcb_ids["source-gate"],
        tcb_ids["property-binder"],
        tcb_ids["rust-toolchain"],
        tcb_ids["qualification-host"],
    ]);

    let docs_artifact = artifact_identity(
        file_measurement(inventory, "m0-property-doc")?,
        "Ferric M0 property contract Markdown",
    );
    let global_input = vec![
        file_measurement(inventory, "source-closure")?.digest,
        file_measurement(inventory, "proof-transcript")?.digest,
        file_measurement(inventory, "proof-counts")?.digest,
        file_measurement(inventory, "verified-modules")?.digest,
        file_measurement(inventory, "property-manifest")?.digest,
        file_measurement(inventory, "m0-property-doc")?.digest,
        sha256_bytes(evidence_index),
    ];
    let model_assumptions = aggregate(
        "ferric.m0-model-assumptions.v1",
        [
            file_measurement(inventory, "assurance-doc")?.digest,
            file_measurement(inventory, "runtime-cargo-lock")?.digest,
            file_measurement(inventory, "unverified-bodies")?.digest,
        ],
    );
    let proof_artifact = artifact_identity(
        file_measurement(inventory, "proof-transcript")?,
        "Verus output-json qualification transcript",
    );
    let witness = artifact_identity(
        file_measurement(inventory, "verified-modules")?,
        "Ferric compiler-path manifest V2",
    );
    let mut built = Vec::new();
    for property in &manifest.properties {
        let property_digest =
            domain_digest("ferric.property-identity.v1", &[property.name.as_bytes()]);
        let statement_digest = sha256_bytes(property.statement.as_bytes());
        let property_id = PropertyIdentityV1::from_untrusted_digest(digest_v1(property_digest));
        let statement_id = StatementIdentityV1::from_untrusted_digest(digest_v1(statement_digest));
        let evidence_digest = domain_digest(
            "ferric.evidence-identity.v1",
            &[
                property.name.as_bytes(),
                property.required_status.as_bytes(),
                &sha256_bytes(evidence_index),
            ],
        );
        let evidence_id = EvidenceIdentityV1::from_untrusted_digest(digest_v1(evidence_digest));
        let binding = EvidenceBindingV1 {
            identity: evidence_id,
            property: property_id,
            statement: statement_id,
        };
        let mut correspondence = None;
        let mut exact_paths = Vec::new();
        let evidence = match property.required_status.as_str() {
            "Proved" => {
                let matched = property_paths(property, &verified)?;
                let artifact_digests = relevant_artifacts(&matched, &verified, inventory)?;
                let mut input_digests = global_input.clone();
                for (_, path) in &matched {
                    input_digests.push(sha256_bytes(path.as_bytes()));
                    exact_paths.push(path.clone());
                }
                for mutation in &property.required_mutations {
                    let evidence = inventory.mutations.get(mutation).ok_or_else(|| {
                        format!(
                            "required actual-body mutation is absent for {}: {mutation}",
                            property.name
                        )
                    })?;
                    if !matched.iter().any(|(package, path)| {
                        package == &evidence.package && path == &evidence.compiler_path
                    }) {
                        return Err(format!(
                            "property mutation target is outside resolved compiler paths: {}|{}|{}",
                            property.name, mutation, evidence.compiler_path
                        ));
                    }
                    input_digests.extend([
                        evidence.mutator,
                        sha256_bytes(evidence.package.as_bytes()),
                        sha256_bytes(evidence.module.as_bytes()),
                        sha256_bytes(evidence.function.as_bytes()),
                        sha256_bytes(evidence.compiler_path.as_bytes()),
                        evidence.marker.digest,
                        evidence.transcript.digest,
                    ]);
                }
                input_digests.extend(artifact_digests.iter().copied());
                let input = ExactInputIdentityV1::new(
                    digest_v1(aggregate("ferric.proved-input.v1", input_digests)),
                    digest_v1(domain_digest(
                        "ferric.input-interpretation.v1",
                        &[b"source+paths+transcript+mutations+release-artifacts"],
                    )),
                );
                let model = ExactModelIdentityV1::new(
                    digest_v1(statement_digest),
                    digest_v1(model_assumptions),
                );
                let correspondence_digest = domain_digest(
                    "ferric.correspondence-identity.v1",
                    &[property.name.as_bytes()],
                );
                let correspondence_id = CorrespondenceIdentityV1::from_untrusted_digest(digest_v1(
                    correspondence_digest,
                ));
                let model_input = ExactInputIdentityV1::new(
                    digest_v1(aggregate(
                        "ferric.source-model.v1",
                        std::iter::once(statement_digest).chain(
                            matched
                                .iter()
                                .map(|(_, path)| sha256_bytes(path.as_bytes())),
                        ),
                    )),
                    digest_v1(model_assumptions),
                );
                correspondence = Some(CorrespondenceReferenceV1 {
                    identity: correspondence_id,
                    kind: CorrespondenceKindV1::SourceToModel,
                    property: property_id,
                    statement: statement_id,
                    from: input,
                    to: model_input,
                    witness_artifact: witness,
                });
                PropertyEvidenceV1::Proved(ProvedEvidenceV1 {
                    binding,
                    input,
                    model,
                    tool: proof_tool,
                    proof_artifact,
                    correspondence: correspondence_id,
                    trusted_computing_base: proof_tcb.clone(),
                })
            }
            "Checked" => {
                let checked = validate_checked_evidence(property, paths, inventory, &verified)?;
                let mut input_digests = global_input.clone();
                input_digests.extend(checked.iter().copied());
                let input = ExactInputIdentityV1::new(
                    digest_v1(aggregate("ferric.checked-input.v1", input_digests)),
                    digest_v1(domain_digest(
                        "ferric.input-interpretation.v1",
                        &[property.checked_evidence.join(",").as_bytes()],
                    )),
                );
                let allocation = property.name == "m0.no_transition_allocation";
                PropertyEvidenceV1::Checked(CheckedEvidenceV1 {
                    binding,
                    input,
                    tool: if allocation {
                        source_gate_tool
                    } else {
                        proof_tool
                    },
                    check_artifact: aggregate_artifact("ferric.checked-evidence.v1", checked),
                    trusted_computing_base: if allocation {
                        source_gate_tcb.clone()
                    } else {
                        proof_tcb.clone()
                    },
                })
            }
            "Contracted" => PropertyEvidenceV1::Contracted(ContractedEvidenceV1 {
                binding,
                contract_artifact: docs_artifact,
            }),
            "Unsupported" => PropertyEvidenceV1::Unsupported(UnsupportedEvidenceV1 {
                binding,
                reason: unsupported_reason(
                    property.unsupported_reason.as_deref().ok_or_else(|| {
                        format!("unsupported rationale missing: {}", property.name)
                    })?,
                )?,
                rationale_artifact: docs_artifact,
            }),
            status => return Err(format!("unimplemented property status: {status}")),
        };
        let status = property_status(&property.required_status)?;
        let record = PropertyRecordV1 {
            identity: property_id,
            kind: property_kind(&property.kind)?,
            statement: statement_id,
            status,
            evidence,
        };
        let obligation_id = ObligationIdentityV1::from_untrusted_digest(digest_v1(domain_digest(
            "ferric.obligation-identity.v1",
            &[property.name.as_bytes()],
        )));
        let obligation = ObligationRecordV1 {
            identity: obligation_id,
            property: property_id,
            statement: statement_id,
            required_status: status,
            satisfaction: Some(ObligationSatisfactionV1 {
                evidence: evidence_id,
                property: property_id,
                statement: statement_id,
                status,
            }),
        };
        let render = format!(
            "property={}|{}|{}|{}|{}|{}|statement-sha256={}|paths={}|mutations={}|checked={}",
            property.name,
            property.kind,
            property.required_status,
            hex(&property_digest),
            hex(&statement_digest),
            hex(&evidence_digest),
            property.statement_sha256,
            exact_paths.join(","),
            property.required_mutations.join(","),
            property.checked_evidence.join(",")
        );
        built.push(BuiltProperty {
            record,
            obligation,
            correspondence,
            render,
        });
    }
    built.sort_by_key(|property| property.record.identity);
    let mut obligations: Vec<ObligationRecordV1> =
        built.iter().map(|property| property.obligation).collect();
    obligations.sort_by_key(|obligation| obligation.identity);
    let mut correspondences: Vec<CorrespondenceReferenceV1> = built
        .iter()
        .filter_map(|property| property.correspondence)
        .collect();
    correspondences.sort_by_key(|reference| reference.identity);
    let contract = ContractSetV1 {
        properties: built
            .iter()
            .map(|property| property.record.clone())
            .collect(),
        obligations: obligations.clone(),
        trusted_computing_base: tcb.clone(),
        correspondences: correspondences.clone(),
    };
    contract
        .validate_closed()
        .map_err(|error| format!("fe2o3 ContractSetV1::validate_closed rejected M0: {error:?}"))?;

    let mut lines = vec![
        format!("format={ARTIFACT_FORMAT}"),
        format!("fe2o3-proof-contracts-commit={FE2O3_COMMIT}"),
        "validator-authority=structural-only".to_owned(),
        "qualification-authority=identity-and-reconciliation".to_owned(),
        "nonclaim=validate_closed-does-not-authenticate-digests-or-prove-semantics".to_owned(),
        format!(
            "manifest-sha256={}",
            hex(&file_measurement(inventory, "property-manifest")?.digest)
        ),
        format!(
            "documentation-sha256={}",
            hex(&file_measurement(inventory, "m0-property-doc")?.digest)
        ),
        format!(
            "evidence-index-sha256={}",
            hex(&sha256_bytes(evidence_index))
        ),
    ];
    lines.extend(built.iter().map(|property| property.render.clone()));
    for obligation in obligations {
        let satisfaction = obligation
            .satisfaction
            .ok_or_else(|| "closed obligation unexpectedly has no satisfaction".to_owned())?;
        lines.push(format!(
            "obligation={}|{}|{}|{}|{}",
            identity_hex(obligation.identity.digest()),
            identity_hex(obligation.property.digest()),
            identity_hex(obligation.statement.digest()),
            identity_hex(satisfaction.evidence.digest()),
            format_status(obligation.required_status)
        ));
    }
    for entry in tcb {
        lines.push(format!(
            "tcb={}|{}|{}|{}|{}",
            identity_hex(entry.identity.digest()),
            format_tcb_kind(entry.kind),
            identity_hex(entry.component.bytes),
            identity_hex(entry.component.format),
            entry.exact_tool.map_or_else(
                || "none".to_owned(),
                |tool| identity_hex(tool.configuration)
            )
        ));
    }
    for reference in correspondences {
        lines.push(format!(
            "correspondence={}|{}|{}|{}|{}|{}",
            identity_hex(reference.identity.digest()),
            format_correspondence_kind(reference.kind),
            identity_hex(reference.property.digest()),
            identity_hex(reference.statement.digest()),
            identity_hex(reference.from.bytes),
            identity_hex(reference.to.bytes)
        ));
    }
    lines.push("contract-set-validation=validate_closed:PASS".to_owned());
    Ok(lines.join("\n") + "\n")
}

fn format_status(status: PropertyStatusV1) -> &'static str {
    match status {
        PropertyStatusV1::Proved => "Proved",
        PropertyStatusV1::Validated => "Validated",
        PropertyStatusV1::Contracted => "Contracted",
        PropertyStatusV1::Checked => "Checked",
        PropertyStatusV1::Unsupported => "Unsupported",
    }
}

fn format_tcb_kind(kind: TcbEntryKindV1) -> &'static str {
    match kind {
        TcbEntryKindV1::Tool => "Tool",
        TcbEntryKindV1::ModelAssumption => "ModelAssumption",
        TcbEntryKindV1::CompilerAssumption => "CompilerAssumption",
        TcbEntryKindV1::RuntimeAssumption => "RuntimeAssumption",
        TcbEntryKindV1::HardwareAssumption => "HardwareAssumption",
        TcbEntryKindV1::HumanReview => "HumanReview",
        TcbEntryKindV1::Extension { .. } => "Extension",
    }
}

fn format_correspondence_kind(kind: CorrespondenceKindV1) -> &'static str {
    match kind {
        CorrespondenceKindV1::ProofErasure => "ProofErasure",
        CorrespondenceKindV1::SourceToModel => "SourceToModel",
        CorrespondenceKindV1::ModelToExecutable => "ModelToExecutable",
        CorrespondenceKindV1::SourceToExecutable => "SourceToExecutable",
        CorrespondenceKindV1::Extension { .. } => "Extension",
    }
}

fn metadata_string<'a>(value: &'a serde_json::Value, field: &str) -> BinderResult<&'a str> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("Cargo metadata field is absent or malformed: {field}"))
}

fn render_dependency_tcb(metadata_path: &Path) -> BinderResult<String> {
    let bytes =
        fs::read(metadata_path).map_err(|error| format!("{}: {error}", metadata_path.display()))?;
    let metadata: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid Cargo metadata: {error}"))?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "property-binder Cargo metadata has no packages".to_owned())?;
    let mut records = BTreeSet::new();
    let mut first_party = BTreeSet::new();
    let mut fe2o3_contracts = 0_u8;
    for package in packages {
        let name = metadata_string(package, "name")?;
        let version = metadata_string(package, "version")?;
        safe_name(name, "dependency name")?;
        safe_name(version, "dependency version")?;
        let source = package
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("first-party");
        if source == "first-party" {
            first_party.insert(name);
        }
        if name == "fe2o3-proof-contracts" {
            if source != FE2O3_SOURCE {
                return Err("fe2o3-proof-contracts source identity drifted".to_owned());
            }
            fe2o3_contracts = fe2o3_contracts
                .checked_add(1)
                .ok_or_else(|| "too many fe2o3-proof-contracts packages".to_owned())?;
        }
        if source.contains(['\n', '\r', '|']) {
            return Err(format!("unsafe dependency source: {source:?}"));
        }
        let mut build_scripts = Vec::new();
        for target in package
            .get("targets")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("dependency has no target list: {name}"))?
        {
            let is_build = target
                .get("kind")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|kinds| {
                    kinds
                        .iter()
                        .any(|kind| kind.as_str() == Some("custom-build"))
                });
            if is_build {
                let target_name = metadata_string(target, "name")?;
                safe_name(target_name, "build-script target")?;
                build_scripts.push(target_name);
            }
        }
        build_scripts.sort_unstable();
        if !records.insert(format!(
            "package={name}|{version}|{source}|{}",
            if build_scripts.is_empty() {
                "none".to_owned()
            } else {
                build_scripts.join(",")
            }
        )) {
            return Err(format!("duplicate dependency identity: {name}"));
        }
    }
    if first_party != BTreeSet::from(["ferric-property-binder"]) || fe2o3_contracts != 1 {
        return Err(format!(
            "property-binder first-party or fe2o3 dependency closure drifted: first-party={first_party:?}, fe2o3-count={fe2o3_contracts}"
        ));
    }
    let mut lines = vec!["format=FERRIC-PROPERTY-BINDER-TCB-V1".to_owned()];
    lines.extend(records);
    Ok(lines.join("\n") + "\n")
}

fn write_output(path: &Path, bytes: &[u8]) -> BinderResult<()> {
    if path.exists() {
        return Err(format!("output already exists: {}", path.display()));
    }
    fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn evidence_paths(arguments: &[String]) -> BinderResult<EvidencePaths<'_>> {
    let [
        repo,
        source_records,
        proof_transcript,
        proof_counts,
        negative_dir,
        verus_root,
        source_gate,
        artifact_dir,
        runtime_tests,
    ] = arguments
    else {
        return Err("wrong number of evidence path arguments".to_owned());
    };
    Ok(EvidencePaths {
        repo: Path::new(repo),
        source_records: Path::new(source_records),
        proof_transcript: Path::new(proof_transcript),
        proof_counts: Path::new(proof_counts),
        negative_dir: Path::new(negative_dir),
        verus_root: Path::new(verus_root),
        source_gate: Path::new(source_gate),
        artifact_dir: Path::new(artifact_dir),
        runtime_tests: Path::new(runtime_tests),
    })
}

fn run() -> BinderResult<()> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match arguments.as_slice() {
        [flag, repo] if flag == "--manifest-check" => {
            let repo = Path::new(repo)
                .canonicalize()
                .map_err(|error| format!("{repo}: {error}"))?;
            let manifest = validate_manifest(&repo, true)?;
            let unresolved = manifest
                .properties
                .iter()
                .filter(|property| {
                    property.required_status == "Proved" && !property.compiler_paths_resolved
                })
                .count();
            println!(
                "PASS: M0 property manifest/docs agree ({} records, {unresolved} unresolved Proved mappings)",
                manifest.properties.len()
            );
            Ok(())
        }
        [flag, metadata, output] if flag == "--dependency-tcb" => {
            let rendered = render_dependency_tcb(Path::new(metadata))?;
            write_output(Path::new(output), rendered.as_bytes())?;
            println!("PASS: generated property-binder dependency TCB");
            Ok(())
        }
        [flag, evidence @ .., output] if flag == "--evidence-index" && evidence.len() == 9 => {
            let paths = evidence_paths(evidence)?;
            let repo = paths
                .repo
                .canonicalize()
                .map_err(|error| format!("{}: {error}", paths.repo.display()))?;
            validate_manifest(&repo, true)?;
            let paths = EvidencePaths {
                repo: &repo,
                ..paths
            };
            let inventory = collect_evidence(&paths)?;
            let rendered = render_evidence_index(&inventory);
            write_output(Path::new(output), rendered.as_bytes())?;
            println!("PASS: generated M0 evidence index");
            Ok(())
        }
        [flag, evidence @ .., index, output] if flag == "--bind" && evidence.len() == 9 => {
            let paths = evidence_paths(evidence)?;
            let repo = paths
                .repo
                .canonicalize()
                .map_err(|error| format!("{}: {error}", paths.repo.display()))?;
            let manifest = validate_manifest(&repo, false)?;
            let paths = EvidencePaths {
                repo: &repo,
                ..paths
            };
            let inventory = collect_evidence(&paths)?;
            let rendered_index = render_evidence_index(&inventory);
            let admitted_index = fs::read(index).map_err(|error| format!("{index}: {error}"))?;
            if admitted_index != rendered_index.as_bytes() {
                return Err("M0 evidence index contains stale or mismatched hashes".to_owned());
            }
            let artifact = build_contract(&repo, &manifest, &paths, &inventory, &admitted_index)?;
            write_output(Path::new(output), artifact.as_bytes())?;
            println!("PASS: fe2o3 ContractSetV1::validate_closed accepted M0");
            Ok(())
        }
        _ => Err(format!(
            "usage:\n  {} --manifest-check REPO\n  {} --dependency-tcb METADATA OUTPUT\n  {} --evidence-index REPO SOURCE_RECORDS PROOF_TRANSCRIPT COUNTS NEGATIVE_DIR VERUS_ROOT SOURCE_GATE ARTIFACT_DIR RUNTIME_TESTS OUTPUT\n  {} --bind REPO SOURCE_RECORDS PROOF_TRANSCRIPT COUNTS NEGATIVE_DIR VERUS_ROOT SOURCE_GATE ARTIFACT_DIR RUNTIME_TESTS EVIDENCE_INDEX OUTPUT",
            env::args()
                .next()
                .unwrap_or_else(|| "ferric-property-binder".to_owned()),
            env::args()
                .next()
                .unwrap_or_else(|| "ferric-property-binder".to_owned()),
            env::args()
                .next()
                .unwrap_or_else(|| "ferric-property-binder".to_owned()),
            env::args()
                .next()
                .unwrap_or_else(|| "ferric-property-binder".to_owned())
        )),
    }
}

fn main() {
    if let Err(error) = run() {
        fail(error);
    }
}
