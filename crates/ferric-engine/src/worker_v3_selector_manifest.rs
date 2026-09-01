//! Canonical deployment input for seven exact M1 Worker V3 publications.

use std::error::Error;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use fe2o3_artifact_transaction::BuildAttempt;
use ferric_build::M1KernelArtifactFamilyV1;
use serde_json::Value;

use crate::{
    M1WorkerV3ArtifactSelectorV1, M1WorkerV3ArtifactSelectorsErrorV1, M1WorkerV3ArtifactSelectorsV1,
};

/// Exact format label for the canonical seven-family selector document.
pub const M1_WORKER_V3_SELECTOR_MANIFEST_FORMAT_V1: &str =
    "ferric.m1-worker-v3-selector-manifest.v1";

/// Maximum admitted selector-document size.
pub const M1_WORKER_V3_SELECTOR_MANIFEST_MAX_BYTES_V1: usize = 64 * 1_024;

/// Why a canonical selector document could not name seven exact publications.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1WorkerV3SelectorManifestErrorV1 {
    Size { actual: usize },
    Json(String),
    NonCanonicalJson,
    Schema(String),
    InvalidOutputDirectory { family: M1KernelArtifactFamilyV1 },
    InvalidBuildAttempt { family: M1KernelArtifactFamilyV1 },
    Selectors(M1WorkerV3ArtifactSelectorsErrorV1),
}

impl fmt::Display for M1WorkerV3SelectorManifestErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size { actual } => write!(
                formatter,
                "M1 Worker V3 selector manifest size {actual} is outside 1..={M1_WORKER_V3_SELECTOR_MANIFEST_MAX_BYTES_V1}"
            ),
            Self::Json(error) => write!(
                formatter,
                "M1 Worker V3 selector manifest JSON is invalid: {error}"
            ),
            Self::NonCanonicalJson => formatter.write_str(
                "M1 Worker V3 selector manifest must be canonical pretty ASCII JSON with one trailing newline",
            ),
            Self::Schema(error) => {
                write!(formatter, "M1 Worker V3 selector manifest schema is invalid: {error}")
            }
            Self::InvalidOutputDirectory { family } => write!(
                formatter,
                "M1 {family:?} Worker V3 output directory is not an exact canonical absolute path"
            ),
            Self::InvalidBuildAttempt { family } => write!(
                formatter,
                "M1 {family:?} Worker V3 build attempt is not canonical"
            ),
            Self::Selectors(error) => error.fmt(formatter),
        }
    }
}

impl Error for M1WorkerV3SelectorManifestErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Selectors(error) => Some(error),
            _ => None,
        }
    }
}

/// Decodes one canonical, explicitly ordered K1-K7 selector document.
///
/// The document names exact `BuildAttempt` values. It never scans a directory or infers a latest
/// publication, and constructing the result performs no artifact recovery or authentication.
///
/// # Errors
///
/// Rejects noncanonical JSON, schema drift, family-order drift, aliased paths, noncanonical build
/// attempts, and one exact publication assigned to multiple families.
pub fn decode_m1_worker_v3_selector_manifest_v1(
    bytes: &[u8],
) -> Result<M1WorkerV3ArtifactSelectorsV1, M1WorkerV3SelectorManifestErrorV1> {
    if bytes.is_empty() || bytes.len() > M1_WORKER_V3_SELECTOR_MANIFEST_MAX_BYTES_V1 {
        return Err(M1WorkerV3SelectorManifestErrorV1::Size {
            actual: bytes.len(),
        });
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| M1WorkerV3SelectorManifestErrorV1::Json(error.to_string()))?;
    if canonical_json(&value)? != bytes {
        return Err(M1WorkerV3SelectorManifestErrorV1::NonCanonicalJson);
    }
    let document = value
        .as_object()
        .ok_or_else(|| schema("root must be an object"))?;
    if document.len() != 2 {
        return Err(schema("root must contain exactly format and selectors"));
    }
    if document.get("format").and_then(Value::as_str)
        != Some(M1_WORKER_V3_SELECTOR_MANIFEST_FORMAT_V1)
    {
        return Err(schema("format is missing or unsupported"));
    }
    let entries = document
        .get("selectors")
        .and_then(Value::as_array)
        .ok_or_else(|| schema("selectors must be an array"))?;
    if entries.len() != M1KernelArtifactFamilyV1::ALL.len() {
        return Err(schema("selectors must contain exactly seven entries"));
    }

    let mut selectors = Vec::with_capacity(entries.len());
    for (ordinal, (expected_family, entry)) in M1KernelArtifactFamilyV1::ALL
        .into_iter()
        .zip(entries)
        .enumerate()
    {
        let entry = entry
            .as_object()
            .ok_or_else(|| schema(format!("selector {ordinal} must be an object")))?;
        if entry.len() != 3 {
            return Err(schema(format!(
                "selector {ordinal} must contain exactly build_attempt, family, and output_directory"
            )));
        }
        if entry.get("family").and_then(Value::as_str) != Some(family_name(expected_family)) {
            return Err(schema(format!(
                "selector {ordinal} must name family {}",
                family_name(expected_family)
            )));
        }
        let output = entry
            .get("output_directory")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                schema(format!(
                    "selector {ordinal} output_directory must be a string"
                ))
            })?;
        let output_dir = canonical_absolute_path(output).ok_or(
            M1WorkerV3SelectorManifestErrorV1::InvalidOutputDirectory {
                family: expected_family,
            },
        )?;
        let attempt = entry
            .get("build_attempt")
            .and_then(Value::as_str)
            .ok_or_else(|| schema(format!("selector {ordinal} build_attempt must be a string")))?;
        let attempt = BuildAttempt::from_env_value(attempt).map_err(|_| {
            M1WorkerV3SelectorManifestErrorV1::InvalidBuildAttempt {
                family: expected_family,
            }
        })?;
        selectors.push(M1WorkerV3ArtifactSelectorV1::new(output_dir, attempt));
    }
    let [gemm, rmsnorm, rope_kv, prefill, paged_decode, swiglu, logits] =
        selectors.try_into().map_err(|_| {
            schema("internal selector count differed from the admitted seven-family bound")
        })?;
    M1WorkerV3ArtifactSelectorsV1::new(
        gemm,
        rmsnorm,
        rope_kv,
        prefill,
        paged_decode,
        swiglu,
        logits,
    )
    .map_err(M1WorkerV3SelectorManifestErrorV1::Selectors)
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, M1WorkerV3SelectorManifestErrorV1> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| M1WorkerV3SelectorManifestErrorV1::Json(error.to_string()))?;
    if !bytes.is_ascii() {
        return Err(M1WorkerV3SelectorManifestErrorV1::NonCanonicalJson);
    }
    bytes.push(b'\n');
    Ok(bytes)
}

fn canonical_absolute_path(value: &str) -> Option<PathBuf> {
    if value.as_bytes().contains(&0) {
        return None;
    }
    let path = Path::new(value);
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::from("/");
    let mut saw_normal = false;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(component) => {
                normalized.push(component);
                saw_normal = true;
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    if saw_normal && normalized.as_os_str() == path.as_os_str() {
        Some(normalized)
    } else {
        None
    }
}

const fn family_name(family: M1KernelArtifactFamilyV1) -> &'static str {
    match family {
        M1KernelArtifactFamilyV1::Gemm => "gemm",
        M1KernelArtifactFamilyV1::RmsNorm => "rmsnorm",
        M1KernelArtifactFamilyV1::RopeKv => "rope-kv",
        M1KernelArtifactFamilyV1::Prefill => "prefill",
        M1KernelArtifactFamilyV1::PagedDecode => "paged-decode",
        M1KernelArtifactFamilyV1::SwiGlu => "swiglu",
        M1KernelArtifactFamilyV1::Logits => "logits",
    }
}

fn schema(error: impl Into<String>) -> M1WorkerV3SelectorManifestErrorV1 {
    M1WorkerV3SelectorManifestErrorV1::Schema(error.into())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const ATTEMPT: &str = concat!(
        "1:",
        "00000000000000000000000000000000:",
        "0000000000000000000000000000000000000000000000000000000000000000"
    );

    fn manifest() -> Value {
        let selectors = M1KernelArtifactFamilyV1::ALL
            .into_iter()
            .enumerate()
            .map(|(ordinal, family)| {
                json!({
                    "build_attempt": ATTEMPT,
                    "family": family_name(family),
                    "output_directory": format!("/worker-v3/k{}", ordinal + 1),
                })
            })
            .collect::<Vec<_>>();
        json!({
            "format": M1_WORKER_V3_SELECTOR_MANIFEST_FORMAT_V1,
            "selectors": selectors,
        })
    }

    fn encode(value: &Value) -> Vec<u8> {
        canonical_json(value).expect("canonical JSON")
    }

    #[test]
    fn canonical_manifest_preserves_all_exact_family_selectors() {
        let selectors = decode_m1_worker_v3_selector_manifest_v1(&encode(&manifest()))
            .expect("canonical selector manifest");
        for (ordinal, family) in M1KernelArtifactFamilyV1::ALL.into_iter().enumerate() {
            let selector = selectors.selector(family);
            assert_eq!(
                selector.output_dir(),
                Path::new(&format!("/worker-v3/k{}", ordinal + 1))
            );
            assert_eq!(selector.attempt().to_env_value(), ATTEMPT);
        }
    }

    #[test]
    fn family_order_and_schema_drift_are_rejected() {
        let mut value = manifest();
        let entries = value["selectors"].as_array_mut().expect("selector array");
        entries.swap(0, 1);
        assert!(matches!(
            decode_m1_worker_v3_selector_manifest_v1(&encode(&value)),
            Err(M1WorkerV3SelectorManifestErrorV1::Schema(_))
        ));

        let mut value = manifest();
        value["extra"] = Value::Bool(false);
        assert!(matches!(
            decode_m1_worker_v3_selector_manifest_v1(&encode(&value)),
            Err(M1WorkerV3SelectorManifestErrorV1::Schema(_))
        ));
    }

    #[test]
    fn noncanonical_json_attempts_and_paths_are_rejected() {
        let compact = serde_json::to_vec(&manifest()).expect("compact JSON");
        assert_eq!(
            decode_m1_worker_v3_selector_manifest_v1(&compact),
            Err(M1WorkerV3SelectorManifestErrorV1::NonCanonicalJson)
        );

        let mut value = manifest();
        value["selectors"][0]["build_attempt"] = Value::String(format!("0:{ATTEMPT}"));
        assert_eq!(
            decode_m1_worker_v3_selector_manifest_v1(&encode(&value)),
            Err(M1WorkerV3SelectorManifestErrorV1::InvalidBuildAttempt {
                family: M1KernelArtifactFamilyV1::Gemm,
            })
        );

        let mut value = manifest();
        value["selectors"][2]["output_directory"] =
            Value::String("/worker-v3/../worker-v3/k3".to_owned());
        assert_eq!(
            decode_m1_worker_v3_selector_manifest_v1(&encode(&value)),
            Err(M1WorkerV3SelectorManifestErrorV1::InvalidOutputDirectory {
                family: M1KernelArtifactFamilyV1::RopeKv,
            })
        );
    }

    #[test]
    fn duplicate_exact_publication_is_rejected_after_manifest_decoding() {
        let mut value = manifest();
        value["selectors"][2]["output_directory"] = Value::String("/worker-v3/k1".to_owned());
        let error = decode_m1_worker_v3_selector_manifest_v1(&encode(&value))
            .expect_err("K1 and K3 cannot select one publication");
        assert_eq!(
            error,
            M1WorkerV3SelectorManifestErrorV1::Selectors(
                M1WorkerV3ArtifactSelectorsErrorV1::DuplicatePublication {
                    first: M1KernelArtifactFamilyV1::Gemm,
                    second: M1KernelArtifactFamilyV1::RopeKv,
                }
            )
        );
    }
}
