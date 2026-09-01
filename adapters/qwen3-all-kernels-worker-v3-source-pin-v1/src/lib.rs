//! Authority-free extraction of Ferric's aggregate M1 source pin.
//!
//! This crate accepts only a canonical receipt-bearing Worker V3 V2 envelope.
//! It validates the nested compiler handoff, exact AMD target, code-object
//! version, and aggregate kernel symbol set before exposing content identities.
//! The resulting projection authenticates no producer and grants no verifier,
//! publication, load, or launch authority.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use fe2o3_compiler_ffi::{
    CodeObjectVersion, CompilerModuleHandoffV2, CompilerModuleKindV1,
    CompilerModuleSymbolManifestV1, CompilerModuleSymbolRoleV1,
    InertSemanticCompilerModuleHandoffErrorV3, InertSemanticCompilerModuleHandoffV3,
};
use fe2o3_runtime_protocol::{WorkerV3LoadEnvelopeErrorV2, WorkerV3LoadEnvelopeWireV2};

/// Canonical JSON format emitted for one admitted aggregate source pin.
pub const M1_AGGREGATE_SOURCE_PIN_FORMAT_V1: &str = "ferric.m1-all-kernels-worker-v3-source-pin.v1";

/// Exact AMD target admitted by the Ferric M1 aggregate publication.
pub const M1_AGGREGATE_TARGET_V1: &str = "gfx942:xnack-";

/// Exact number of kernel entry symbols in the aggregate publication.
pub const M1_AGGREGATE_PROGRAM_COUNT_V1: usize = 12;

/// Exact Ferric aggregate policy roster.
///
/// This order is Ferric policy metadata. The compiler symbol manifest proves the exact entry and
/// descriptor sets, but its canonical lexical order does not attest descriptor-table order.
pub const M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1: [&str; M1_AGGREGATE_PROGRAM_COUNT_V1] = [
    "ferric_qwen3_lowest_id_argmax_bf16_v1",
    "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
    "qwen3_rope_v1",
    "ferric_qwen3_compact_completion_v1",
    "qwen3_paged_kv_write_v1",
    "qwen3_paged_gqa_decode_bf16_f32_v1",
    "qwen3_swiglu_bf16_f32_v1",
    "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
    "qwen3_rmsnorm_v1",
    "ferric_qwen3_token_embedding_bf16_copy_v1",
    "ferric_qwen3_speculative_token_assembly_v1",
    "qwen3_gqa_prefill_causal_bf16_f32_v1",
];

/// Aggregate envelope field that failed exact source-selection policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AggregateSourcePinFieldV1 {
    /// Neutral compiler-module representation.
    ModuleKind,
    /// Canonical AMD target.
    Target,
    /// AMDGPU HSA code-object version.
    CodeObjectVersion,
    /// Complete set of compiler-observed kernel entry symbols.
    KernelEntries,
    /// Complete set of compiler-observed kernel descriptor symbols.
    KernelDescriptors,
}

/// Failure while decoding or projecting an aggregate Worker V3 V2 envelope.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AggregateSourcePinErrorV1 {
    /// The top-level receipt-bearing V2 envelope is invalid or noncanonical.
    Envelope(Box<WorkerV3LoadEnvelopeErrorV2>),
    /// The nested inert semantic compiler handoff is invalid or noncanonical.
    OuterHandoff(Box<InertSemanticCompilerModuleHandoffErrorV3>),
    /// One exact aggregate selection axis differs from policy.
    Policy {
        /// Rejected policy axis.
        field: M1AggregateSourcePinFieldV1,
        /// Canonical diagnostic rendering of the observed value.
        actual: String,
    },
    /// Canonical JSON serialization failed.
    Json(Box<serde_json::Error>),
}

impl fmt::Display for M1AggregateSourcePinErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Envelope(error) => write!(formatter, "invalid Worker V3 envelope V2: {error}"),
            Self::OuterHandoff(error) => {
                write!(formatter, "invalid nested compiler handoff V3: {error}")
            }
            Self::Policy { field, actual } => {
                write!(
                    formatter,
                    "aggregate source-pin policy rejected {field:?}: {actual}"
                )
            }
            Self::Json(error) => write!(
                formatter,
                "cannot encode canonical source-pin JSON: {error}"
            ),
        }
    }
}

impl Error for M1AggregateSourcePinErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Envelope(error) => Some(error.as_ref()),
            Self::OuterHandoff(error) => Some(error.as_ref()),
            Self::Json(error) => Some(error.as_ref()),
            Self::Policy { .. } => None,
        }
    }
}

/// Six exact compiler source coordinates derived from one canonical V2 envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AggregateSourcePinV1 {
    compiler_module_sha256: [u8; 32],
    compiler_module_length: u64,
    compiler_handoff_sha256: [u8; 32],
    compiler_handoff_length: u64,
    symbol_manifest_sha256: [u8; 32],
    symbol_manifest_length: u64,
}

impl M1AggregateSourcePinV1 {
    /// Returns the exact neutral LLVM module digest.
    #[must_use]
    pub const fn compiler_module_sha256(self) -> [u8; 32] {
        self.compiler_module_sha256
    }

    /// Returns the exact neutral LLVM module byte length.
    #[must_use]
    pub const fn compiler_module_length(self) -> u64 {
        self.compiler_module_length
    }

    /// Returns the exact nested V2 compiler-handoff digest.
    #[must_use]
    pub const fn compiler_handoff_sha256(self) -> [u8; 32] {
        self.compiler_handoff_sha256
    }

    /// Returns the exact nested V2 compiler-handoff byte length.
    #[must_use]
    pub const fn compiler_handoff_length(self) -> u64 {
        self.compiler_handoff_length
    }

    /// Returns the exact compiler symbol-manifest digest.
    #[must_use]
    pub const fn symbol_manifest_sha256(self) -> [u8; 32] {
        self.symbol_manifest_sha256
    }

    /// Returns the exact compiler symbol-manifest byte length.
    #[must_use]
    pub const fn symbol_manifest_length(self) -> u64 {
        self.symbol_manifest_length
    }
}

/// Authority-free projection of one exact aggregate source pin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AggregateSourcePinProjectionV1 {
    source_pin: M1AggregateSourcePinV1,
}

impl M1AggregateSourcePinProjectionV1 {
    /// Returns all six exact source-pin coordinates.
    #[must_use]
    pub const fn source_pin(self) -> M1AggregateSourcePinV1 {
        self.source_pin
    }

    /// Returns the deterministic, pretty-printed ASCII JSON document with a final newline.
    ///
    /// # Errors
    ///
    /// Returns an error if `serde_json` cannot serialize the fixed projection.
    pub fn to_canonical_json(self) -> Result<Vec<u8>, M1AggregateSourcePinErrorV1> {
        let source_pin = self.source_pin;
        let document = serde_json::json!({
            "authority": "identity-observation-only",
            "authenticates_compiler_origin": false,
            "code_object_version": 6,
            "format": M1_AGGREGATE_SOURCE_PIN_FORMAT_V1,
            "grants_launch_authority": false,
            "grants_load_authority": false,
            "grants_publication_authority": false,
            "grants_verifier_authority": false,
            "policy_kernel_symbols": M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1,
            "program_count": M1_AGGREGATE_PROGRAM_COUNT_V1,
            "source_pin": {
                "compiler_handoff_length": source_pin.compiler_handoff_length,
                "compiler_handoff_sha256": hex32(&source_pin.compiler_handoff_sha256),
                "compiler_module_length": source_pin.compiler_module_length,
                "compiler_module_sha256": hex32(&source_pin.compiler_module_sha256),
                "symbol_manifest_length": source_pin.symbol_manifest_length,
                "symbol_manifest_sha256": hex32(&source_pin.symbol_manifest_sha256),
            },
            "target": M1_AGGREGATE_TARGET_V1,
        });
        let mut bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| M1AggregateSourcePinErrorV1::Json(Box::new(error)))?;
        bytes.push(b'\n');
        debug_assert!(bytes.is_ascii());
        Ok(bytes)
    }

    /// Identity observation alone never authenticates compiler process origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(self) -> bool {
        false
    }

    /// Identity observation grants no protected-verifier authority.
    #[must_use]
    pub const fn grants_verifier_authority(self) -> bool {
        false
    }

    /// Identity observation grants no publication authority.
    #[must_use]
    pub const fn grants_publication_authority(self) -> bool {
        false
    }

    /// Identity observation grants no GPU load authority.
    #[must_use]
    pub const fn grants_load_authority(self) -> bool {
        false
    }

    /// Identity observation grants no GPU launch authority.
    #[must_use]
    pub const fn grants_launch_authority(self) -> bool {
        false
    }
}

/// Strictly decodes one canonical Worker V3 V2 envelope and extracts its aggregate source pin.
///
/// The top-level V2 decoder validates the complete receipt carriage and nested replay. The nested
/// compiler-FFI decoder then validates the exact outer V3 handoff and native V2 module handoff.
/// This function additionally requires LLVM text IR, `gfx942:xnack-`, code-object V6, and the
/// exact aggregate sets of 12 kernel-entry and 12 matching kernel-descriptor symbols.
///
/// # Errors
///
/// Returns a typed decoding or exact-policy error. No partial projection is returned.
pub fn extract_m1_aggregate_source_pin_v1(
    envelope_bytes: &[u8],
) -> Result<M1AggregateSourcePinProjectionV1, M1AggregateSourcePinErrorV1> {
    let wire = WorkerV3LoadEnvelopeWireV2::decode_canonical(envelope_bytes)
        .map_err(|error| M1AggregateSourcePinErrorV1::Envelope(Box::new(error)))?;
    let outer = InertSemanticCompilerModuleHandoffV3::decode(wire.replay().outer_handoff())
        .map_err(|error| M1AggregateSourcePinErrorV1::OuterHandoff(Box::new(error)))?;
    project_module_handoff(outer.module_handoff())
}

fn project_module_handoff(
    handoff: &CompilerModuleHandoffV2,
) -> Result<M1AggregateSourcePinProjectionV1, M1AggregateSourcePinErrorV1> {
    if handoff.kind() != CompilerModuleKindV1::LlvmTextIr {
        return Err(policy(
            M1AggregateSourcePinFieldV1::ModuleKind,
            format!("{:?}", handoff.kind()),
        ));
    }
    let target = handoff.target().to_string();
    if target != M1_AGGREGATE_TARGET_V1 {
        return Err(policy(M1AggregateSourcePinFieldV1::Target, target));
    }
    if handoff.code_object_version() != CodeObjectVersion::V6 {
        return Err(policy(
            M1AggregateSourcePinFieldV1::CodeObjectVersion,
            handoff.code_object_version().number().to_string(),
        ));
    }
    validate_kernel_symbols(handoff.symbol_manifest())?;

    let module = handoff.module_identity();
    let compiler_handoff = handoff.identity();
    let manifest = handoff.symbol_manifest().identity();
    Ok(M1AggregateSourcePinProjectionV1 {
        source_pin: M1AggregateSourcePinV1 {
            compiler_module_sha256: *module.sha256(),
            compiler_module_length: module.byte_len(),
            compiler_handoff_sha256: *compiler_handoff.sha256(),
            compiler_handoff_length: compiler_handoff.byte_len(),
            symbol_manifest_sha256: *manifest.sha256(),
            symbol_manifest_length: manifest.byte_len(),
        },
    })
}

fn validate_kernel_symbols(
    manifest: &CompilerModuleSymbolManifestV1,
) -> Result<(), M1AggregateSourcePinErrorV1> {
    let entries = manifest
        .symbols(CompilerModuleSymbolRoleV1::KernelEntry)
        .collect::<Vec<_>>();
    let mut expected_entries = M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1;
    expected_entries.sort_unstable();
    if entries != expected_entries {
        return Err(policy(
            M1AggregateSourcePinFieldV1::KernelEntries,
            render_symbols(&entries),
        ));
    }

    let descriptors = manifest
        .symbols(CompilerModuleSymbolRoleV1::KernelDescriptor)
        .collect::<Vec<_>>();
    let mut expected_descriptors = M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1
        .iter()
        .map(|symbol| format!("{symbol}.kd"))
        .collect::<Vec<_>>();
    expected_descriptors.sort_unstable();
    if descriptors.len() != expected_descriptors.len()
        || descriptors
            .iter()
            .zip(&expected_descriptors)
            .any(|(actual, expected)| *actual != expected)
    {
        return Err(policy(
            M1AggregateSourcePinFieldV1::KernelDescriptors,
            render_symbols(&descriptors),
        ));
    }
    Ok(())
}

fn render_symbols(symbols: &[&str]) -> String {
    symbols.join(",")
}

fn policy(field: M1AggregateSourcePinFieldV1, actual: String) -> M1AggregateSourcePinErrorV1 {
    M1AggregateSourcePinErrorV1::Policy { field, actual }
}

fn hex32(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe2o3_compiler_ffi::{CompilerFfiEnvelopeV1, DeviceTargetV1};

    const LLVM_IR: &[u8] = b"; exact aggregate source-pin fixture\n";

    fn manifest(entries: &[&str], descriptors: &[String]) -> CompilerModuleSymbolManifestV1 {
        let mut entries = entries
            .iter()
            .map(|symbol| (*symbol).to_owned())
            .collect::<Vec<_>>();
        entries.sort_unstable();
        let mut descriptors = descriptors.to_vec();
        descriptors.sort_unstable();
        CompilerModuleSymbolManifestV1::new(
            entries
                .into_iter()
                .map(|symbol| (CompilerModuleSymbolRoleV1::KernelEntry, symbol))
                .chain(
                    descriptors
                        .into_iter()
                        .map(|symbol| (CompilerModuleSymbolRoleV1::KernelDescriptor, symbol)),
                ),
        )
        .expect("canonical fixture manifest")
    }

    fn exact_descriptors() -> Vec<String> {
        M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1
            .iter()
            .map(|symbol| format!("{symbol}.kd"))
            .collect()
    }

    fn handoff_with_kind(
        kind: CompilerModuleKindV1,
        target: &str,
        code_object_version: CodeObjectVersion,
        entries: &[&str],
        descriptors: &[String],
    ) -> CompilerModuleHandoffV2 {
        let target = DeviceTargetV1::parse(target).expect("canonical fixture target");
        let envelope =
            CompilerFfiEnvelopeV1::for_module_without_device_ffi(target, code_object_version)
                .expect("fixture FFI envelope");
        CompilerModuleHandoffV2::new(
            kind,
            target,
            code_object_version,
            envelope,
            manifest(entries, descriptors),
            LLVM_IR,
        )
        .expect("fixture module handoff")
    }

    fn handoff(
        target: &str,
        code_object_version: CodeObjectVersion,
        entries: &[&str],
        descriptors: &[String],
    ) -> CompilerModuleHandoffV2 {
        handoff_with_kind(
            CompilerModuleKindV1::LlvmTextIr,
            target,
            code_object_version,
            entries,
            descriptors,
        )
    }

    #[test]
    fn exact_typed_handoff_projects_all_six_source_coordinates() {
        let handoff = handoff(
            M1_AGGREGATE_TARGET_V1,
            CodeObjectVersion::V6,
            &M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1,
            &exact_descriptors(),
        );
        let projection = project_module_handoff(&handoff).expect("exact aggregate handoff");
        let pin = projection.source_pin();
        assert_eq!(
            pin.compiler_module_sha256(),
            *handoff.module_identity().sha256()
        );
        assert_eq!(
            pin.compiler_module_length(),
            handoff.module_identity().byte_len()
        );
        assert_eq!(pin.compiler_handoff_sha256(), *handoff.identity().sha256());
        assert_eq!(pin.compiler_handoff_length(), handoff.identity().byte_len());
        assert_eq!(
            pin.symbol_manifest_sha256(),
            *handoff.symbol_manifest().identity().sha256()
        );
        assert_eq!(
            pin.symbol_manifest_length(),
            handoff.symbol_manifest().identity().byte_len()
        );
        assert!(!projection.authenticates_compiler_origin());
        assert!(!projection.grants_verifier_authority());
        assert!(!projection.grants_publication_authority());
        assert!(!projection.grants_load_authority());
        assert!(!projection.grants_launch_authority());
    }

    #[test]
    fn canonical_json_is_ascii_newline_terminated_and_scope_limited() {
        let handoff = handoff(
            M1_AGGREGATE_TARGET_V1,
            CodeObjectVersion::V6,
            &M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1,
            &exact_descriptors(),
        );
        let bytes = project_module_handoff(&handoff)
            .unwrap()
            .to_canonical_json()
            .unwrap();
        assert!(bytes.is_ascii() && bytes.ends_with(b"\n"));
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["format"], M1_AGGREGATE_SOURCE_PIN_FORMAT_V1);
        assert_eq!(value["authority"], "identity-observation-only");
        assert_eq!(value["program_count"], M1_AGGREGATE_PROGRAM_COUNT_V1);
        assert_eq!(value["target"], M1_AGGREGATE_TARGET_V1);
        for field in [
            "authenticates_compiler_origin",
            "grants_verifier_authority",
            "grants_publication_authority",
            "grants_load_authority",
            "grants_launch_authority",
        ] {
            assert_eq!(value[field], false, "{field}");
        }
        assert_eq!(value["source_pin"].as_object().unwrap().len(), 6);
    }

    #[test]
    fn malformed_top_level_v2_envelope_is_rejected_before_projection() {
        assert!(matches!(
            extract_m1_aggregate_source_pin_v1(b"not a Worker V3 V2 envelope"),
            Err(M1AggregateSourcePinErrorV1::Envelope(_))
        ));
    }

    #[test]
    fn wrong_target_and_code_object_version_are_rejected() {
        let wrong_target = handoff(
            "gfx942:xnack+",
            CodeObjectVersion::V6,
            &M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1,
            &exact_descriptors(),
        );
        assert!(matches!(
            project_module_handoff(&wrong_target),
            Err(M1AggregateSourcePinErrorV1::Policy {
                field: M1AggregateSourcePinFieldV1::Target,
                ..
            })
        ));

        let wrong_version = handoff(
            M1_AGGREGATE_TARGET_V1,
            CodeObjectVersion::V5,
            &M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1,
            &exact_descriptors(),
        );
        assert!(matches!(
            project_module_handoff(&wrong_version),
            Err(M1AggregateSourcePinErrorV1::Policy {
                field: M1AggregateSourcePinFieldV1::CodeObjectVersion,
                ..
            })
        ));
    }

    #[test]
    fn llvm_bitcode_is_rejected_before_source_pin_projection() {
        let bitcode = handoff_with_kind(
            CompilerModuleKindV1::LlvmBitcode,
            M1_AGGREGATE_TARGET_V1,
            CodeObjectVersion::V6,
            &M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1,
            &exact_descriptors(),
        );
        assert!(matches!(
            project_module_handoff(&bitcode),
            Err(M1AggregateSourcePinErrorV1::Policy {
                field: M1AggregateSourcePinFieldV1::ModuleKind,
                ..
            })
        ));
    }

    #[test]
    fn missing_or_extra_kernel_entry_is_rejected() {
        let missing = handoff(
            M1_AGGREGATE_TARGET_V1,
            CodeObjectVersion::V6,
            &M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1[..11],
            &exact_descriptors()[..11],
        );
        assert!(matches!(
            project_module_handoff(&missing),
            Err(M1AggregateSourcePinErrorV1::Policy {
                field: M1AggregateSourcePinFieldV1::KernelEntries,
                ..
            })
        ));

        let mut extra_entries = M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1.to_vec();
        extra_entries.push("substituted_extra_kernel_v1");
        let mut extra_descriptors = exact_descriptors();
        extra_descriptors.push("substituted_extra_kernel_v1.kd".to_owned());
        let extra = handoff(
            M1_AGGREGATE_TARGET_V1,
            CodeObjectVersion::V6,
            &extra_entries,
            &extra_descriptors,
        );
        assert!(matches!(
            project_module_handoff(&extra),
            Err(M1AggregateSourcePinErrorV1::Policy {
                field: M1AggregateSourcePinFieldV1::KernelEntries,
                ..
            })
        ));
    }

    #[test]
    fn substituted_kernel_descriptor_is_rejected() {
        let mut descriptors = exact_descriptors();
        descriptors[0] = "substituted_kernel_descriptor_v1.kd".to_owned();
        let substituted = handoff(
            M1_AGGREGATE_TARGET_V1,
            CodeObjectVersion::V6,
            &M1_AGGREGATE_POLICY_KERNEL_SYMBOLS_V1,
            &descriptors,
        );
        assert!(matches!(
            project_module_handoff(&substituted),
            Err(M1AggregateSourcePinErrorV1::Policy {
                field: M1AggregateSourcePinFieldV1::KernelDescriptors,
                ..
            })
        ));
    }
}
