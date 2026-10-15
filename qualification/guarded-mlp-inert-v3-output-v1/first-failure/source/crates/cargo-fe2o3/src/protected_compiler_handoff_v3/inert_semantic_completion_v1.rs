//! Lossless inert-only completion content; not a protected transaction intake.
#![allow(dead_code)] // Staged; the selector stays held until the full child route is qualified.

use std::fmt;
use std::process::ExitStatus;

use fe2o3_amd_target::ProductionAmdTargetProfileV1;
use fe2o3_compiler_ffi::{
    InertSemanticCompilerModuleHandoffErrorV3, InertSemanticCompilerModuleHandoffIdentityV3,
    InertSemanticCompilerModuleHandoffV3,
};
use fe2o3_rustc_invocation::RustcInvocationDescriptorV3;

use super::inert_semantic_extraction_v1::Eligibility;
use super::inert_semantic_output_v1::{CapturedOutput, PinnedOutput};

/// Retains the original eligibility/parent borrow and the COMPLETE outer owner.
/// No conversion to a protected consumed transaction, bare V2 handoff, worker
/// request, artifact or launch authority is provided.
pub(crate) struct Completion<'a> {
    _eligibility: Eligibility<'a>,
    _output: PinnedOutput<'a>,
    content: RetainedContent,
}

impl<'a> Completion<'a> {
    /// Checks content after a reported successful exit. The future caller must
    /// separately own wait/reaping and the provenance of the expected outer
    /// identity. Retained output objects establish file consistency only.
    pub(crate) fn consume_after_exit(
        eligibility: Eligibility<'a>,
        status: ExitStatus,
        captured: CapturedOutput<'a>,
    ) -> Result<Self, Refusal> {
        let selected = eligibility
            .revalidate_for_completion()
            .map_err(|_| Refusal::ParentChanged)?;
        let (output, bytes, expected_outer) = captured
            .into_retained_parts()
            .map_err(|_| Refusal::OutputChanged)?;
        let content = RetainedContent::decode_after_exit(status, bytes, selected, expected_outer)?;
        eligibility
            .revalidate_for_completion()
            .map_err(|_| Refusal::ParentChanged)?;
        output.revalidate().map_err(|_| Refusal::OutputChanged)?;
        Ok(Self {
            _output: output,
            _eligibility: eligibility,
            content,
        })
    }

    pub(crate) fn canonical_outer_bytes(&self) -> &[u8] {
        self.content.outer.canonical_bytes()
    }

    pub(crate) fn formal_memory_preimage(&self) -> &[u8] {
        self.content
            .outer
            .capsule()
            .receipts()
            .formal_memory()
            .canonical_preimage()
    }

    pub(crate) fn proof_binding_preimage(&self) -> &[u8] {
        self.content
            .outer
            .capsule()
            .receipts()
            .proof_binding()
            .canonical_preimage()
    }

    /// Serialization may transfer only the complete outer wire, never just V2.
    pub(crate) fn into_canonical_outer_bytes(self) -> Result<Vec<u8>, Refusal> {
        self._eligibility
            .revalidate_for_completion()
            .map_err(|_| Refusal::ParentChanged)?;
        self._output
            .revalidate()
            .map_err(|_| Refusal::OutputChanged)?;
        self.content
            .outer
            .try_into_canonical_bytes()
            .map_err(|_| Refusal::Encoding)
    }

    pub(crate) const fn grants_execution_or_publication_authority(&self) -> bool {
        false
    }

    pub(crate) fn require_protected_publication_authority(&self) -> Result<(), Refusal> {
        Err(Refusal::NoProtectedPublicationAuthority)
    }
}

/// Structural content consistency only. Canonical receipt preimages remain
/// opaque and retained: no formal or numerical premise is discharged here.
struct RetainedContent {
    outer: InertSemanticCompilerModuleHandoffV3,
}

impl RetainedContent {
    fn decode_after_exit(
        status: ExitStatus,
        bytes: Box<[u8]>,
        selected: &RustcInvocationDescriptorV3,
        expected_outer: InertSemanticCompilerModuleHandoffIdentityV3,
    ) -> Result<Self, Refusal> {
        if !status.success() {
            return Err(Refusal::ChildUnsuccessful);
        }
        // The strict existing decoder enforces all byte bounds and canonical
        // pair/inner identities. Owned decode preserves the outer allocation.
        let outer =
            InertSemanticCompilerModuleHandoffV3::decode_owned(bytes).map_err(
                |error| match error {
                    InertSemanticCompilerModuleHandoffErrorV3::FinalCommitment(_)
                    | InertSemanticCompilerModuleHandoffErrorV3::FinalCommitmentMismatch => {
                        Refusal::FinalCommitmentMismatch
                    }
                    _ => Refusal::Encoding,
                },
            )?;
        if outer.identity() != expected_outer {
            return Err(Refusal::UnexpectedOuterIdentity);
        }
        let capsule = outer.capsule();
        let target = ProductionAmdTargetProfileV1::Gfx950.device_target();
        if selected.amd_target() != target
            || capsule.target().to_string() != target
            || outer.module_handoff().target().to_string() != target
        {
            return Err(Refusal::TargetMismatch);
        }
        // Full descriptor equality includes the six closure roles, every argv
        // token, cwd and environment; a self-consistent foreign capsule is not
        // evidence for this parent's selected invocation.
        if capsule.invocation() != selected {
            return Err(Refusal::InvocationMismatch);
        }
        Ok(Self { outer })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    ParentChanged,
    OutputChanged,
    ChildUnsuccessful,
    Encoding,
    UnexpectedOuterIdentity,
    TargetMismatch,
    InvocationMismatch,
    FinalCommitmentMismatch,
    NoProtectedPublicationAuthority,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ParentChanged => "inert completion retained parent invocation changed",
            Self::OutputChanged => "inert completion retained output object changed",
            Self::ChildUnsuccessful => "inert completion requires a successful child exit status",
            Self::Encoding => "inert completion requires a complete canonical outer V3 handoff",
            Self::UnexpectedOuterIdentity => {
                "inert completion differs from the independently expected outer identity"
            }
            Self::TargetMismatch => "inert completion requires the selected exact gfx950 target",
            Self::InvocationMismatch => {
                "inert completion capsule differs from the selected invocation and closure"
            }
            Self::FinalCommitmentMismatch => {
                "inert completion final commitment differs from the retained module"
            }
            Self::NoProtectedPublicationAuthority => {
                "inert completion cannot adopt protected publication authority"
            }
        })
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use fe2o3_build_authority::CompilerClosureV2;
    use fe2o3_compiler_ffi::InertFinalCompilerModuleCommitmentV3;
    use fe2o3_compiler_ffi::{
        CodeObjectVersion, CompilerFfiEnvelopeV1, CompilerModuleHandoffV2, CompilerModuleKindV1,
        CompilerModuleSymbolManifestV1, CompilerModuleSymbolRoleV1, DeviceTargetV1,
    };
    use fe2o3_compiler_lineage::*;
    use fe2o3_rustc_invocation::{CompileEnvironmentV2, RustcInvocationDescriptorV2, RustcUnitV2};
    use std::ffi::OsString;
    use std::os::unix::process::ExitStatusExt;

    fn exit(raw: i32) -> ExitStatus {
        ExitStatus::from_raw(raw)
    }

    // All fixtures below are inert schema data, NOT actual compiler execution,
    // formal archives, conditional proof admissions or runtime evidence.
    fn invocation(target: &str, role_seed: u8) -> RustcInvocationDescriptorV3 {
        let closure =
            CompilerClosureV2::new([role_seed; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32])
                .unwrap();
        let args = RustcUnitV2::new(
            "/workspace",
            vec![
                "/tools/rustc".into(),
                "--crate-name=kernel".into(),
                "kernel.rs".into(),
                "-Zcodegen-backend=/tools/backend.so".into(),
            ],
        )
        .unwrap();
        let env = CompileEnvironmentV2::from_child_environment([
            (OsString::from("FE2O3_TARGET"), OsString::from(target)),
            ("FE2O3_HSACO_DIR".into(), "/private/artifacts".into()),
        ])
        .unwrap();
        RustcInvocationDescriptorV3::new(
            RustcInvocationDescriptorV2::new([4; 32], [6; 32], args, env).unwrap(),
            closure,
        )
        .unwrap()
    }

    fn module(target: &str, payload: &[u8]) -> CompilerModuleHandoffV2 {
        let target = DeviceTargetV1::parse(target).unwrap();
        CompilerModuleHandoffV2::new(
            CompilerModuleKindV1::LlvmTextIr,
            target,
            CodeObjectVersion::V6,
            CompilerFfiEnvelopeV1::for_module_without_device_ffi(target, CodeObjectVersion::V6)
                .unwrap(),
            CompilerModuleSymbolManifestV1::new([
                (CompilerModuleSymbolRoleV1::KernelEntry, "kernel"),
                (CompilerModuleSymbolRoleV1::KernelDescriptor, "kernel.kd"),
            ])
            .unwrap(),
            payload,
        )
        .unwrap()
    }

    fn outer(
        selected: RustcInvocationDescriptorV3,
        formal: &[u8],
        proof: &[u8],
    ) -> InertSemanticCompilerModuleHandoffV3 {
        let module = module(
            selected.amd_target(),
            b"define amdgpu_kernel void @kernel() { ret void }\n",
        );
        let commitment = InertFinalCompilerModuleCommitmentV3::from_handoff(&module).unwrap();
        let receipts = OrderedInertSemanticLineageReceiptsV3::new(
            InertRustcIdentityInventoryReceiptV3::from_canonical_preimage(
                b"fixture-inventory".to_vec(),
            )
            .unwrap(),
            InertRustcPreflightPlanReceiptV3::from_canonical_preimage(
                b"fixture-preflight".to_vec(),
            )
            .unwrap(),
            InertCanonicalSemanticMirReceiptV3::from_canonical_preimage(
                b"fixture-semantic".to_vec(),
            )
            .unwrap(),
            InertMiddleEndReceiptV3::from_canonical_preimage(b"fixture-middle".to_vec()).unwrap(),
            InertKernelIrReceiptV3::from_canonical_preimage(b"fixture-kir".to_vec()).unwrap(),
            InertMirToKirCorrespondenceReceiptV3::from_canonical_preimage(
                b"fixture-correspondence".to_vec(),
            )
            .unwrap(),
            InertFormalMemoryReceiptV3::from_canonical_preimage(formal.to_vec()).unwrap(),
            InertProofBindingReceiptV3::from_canonical_preimage(proof.to_vec()).unwrap(),
            InertTargetBindingReceiptV3::from_canonical_preimage(b"fixture-target".to_vec())
                .unwrap(),
            InertDataLayoutReceiptV3::from_canonical_preimage(b"fixture-layout".to_vec()).unwrap(),
            InertAbiReceiptV3::from_canonical_preimage(b"fixture-abi".to_vec()).unwrap(),
            InertExportManifestReceiptV3::from_canonical_preimage(b"fixture-exports".to_vec())
                .unwrap(),
            InertAmdgpuLoweringReceiptV3::from_canonical_preimage(b"fixture-lowering".to_vec())
                .unwrap(),
            InertSemanticToLlvmReceiptV3::from_canonical_preimage(b"fixture-llvm".to_vec())
                .unwrap(),
            InertFinalCompilerModuleCommitmentReceiptV3::from_canonical_preimage(
                commitment.canonical_bytes().to_vec(),
            )
            .unwrap(),
        );
        let target = module.target();
        InertSemanticCompilerModuleHandoffV3::new(
            InertProductionSemanticCapsuleV3::new(selected, target, receipts).unwrap(),
            module,
        )
        .unwrap()
    }

    fn module_handoff_alternate(target: &str) -> CompilerModuleHandoffV2 {
        module(
            target,
            b"; alternate\ndefine amdgpu_kernel void @kernel() { ret void }\n",
        )
    }

    pub(in crate::protected_compiler_handoff_v3) fn baseline()
    -> InertSemanticCompilerModuleHandoffV3 {
        outer(
            invocation("gfx950:xnack-", 1),
            b"OPAQUE-FIXTURE: runtime-premise-A unresolved\0runtime-premise-B unresolved",
            b"OPAQUE-FIXTURE: no signed proof",
        )
    }

    fn decode(
        wire: &[u8],
        selected: &RustcInvocationDescriptorV3,
        expected: InertSemanticCompilerModuleHandoffIdentityV3,
    ) -> Result<RetainedContent, Refusal> {
        RetainedContent::decode_after_exit(
            exit(0),
            wire.to_vec().into_boxed_slice(),
            selected,
            expected,
        )
    }

    #[test]
    fn owned_decode_retains_exact_outer_allocation_and_opaque_premises() {
        let original = baseline();
        let selected = original.capsule().invocation().clone();
        let wire = original.canonical_bytes().to_vec().into_boxed_slice();
        let ptr = wire.as_ptr();
        let retained =
            RetainedContent::decode_after_exit(exit(0), wire, &selected, original.identity())
                .unwrap();
        assert_eq!(retained.outer.canonical_bytes(), original.canonical_bytes());
        assert_eq!(retained.outer.canonical_bytes().as_ptr(), ptr);
        assert_eq!(
            retained
                .outer
                .capsule()
                .receipts()
                .formal_memory()
                .canonical_preimage(),
            original
                .capsule()
                .receipts()
                .formal_memory()
                .canonical_preimage()
        );
        assert_eq!(
            retained
                .outer
                .capsule()
                .receipts()
                .proof_binding()
                .canonical_preimage(),
            original
                .capsule()
                .receipts()
                .proof_binding()
                .canonical_preimage()
        );
        assert!(!retained.outer.authenticates_compiler_origin());
        assert!(!retained.outer.grants_launch_authority());
        assert_eq!(
            retained.outer.try_into_canonical_bytes().unwrap(),
            original.canonical_bytes()
        );
    }

    pub(in crate::protected_compiler_handoff_v3) fn assert_retained_wire(
        bytes: &[u8],
        expected: &InertSemanticCompilerModuleHandoffV3,
    ) {
        let content = decode(bytes, expected.capsule().invocation(), expected.identity()).unwrap();
        assert_eq!(content.outer.canonical_bytes(), expected.canonical_bytes());
        assert_eq!(
            content
                .outer
                .capsule()
                .receipts()
                .formal_memory()
                .canonical_preimage(),
            expected
                .capsule()
                .receipts()
                .formal_memory()
                .canonical_preimage()
        );
    }

    #[test]
    fn ordinary_v2_is_not_a_completion_and_malformed_outer_never_falls_back() {
        let original = baseline();
        let selected = original.capsule().invocation();
        let mut trailing = original.canonical_bytes().to_vec();
        trailing.push(0);
        for wire in [
            original.module_handoff().canonical_bytes(),
            &original.canonical_bytes()[..20],
            &trailing,
        ] {
            assert!(matches!(
                decode(wire, selected, original.identity()),
                Err(Refusal::Encoding)
            ));
        }
    }

    #[test]
    fn nonzero_and_signalled_child_status_never_consume_even_valid_content() {
        let original = baseline();
        for status in [exit(1 << 8), exit(9)] {
            assert!(matches!(
                RetainedContent::decode_after_exit(
                    status,
                    original.canonical_bytes().to_vec().into_boxed_slice(),
                    original.capsule().invocation(),
                    original.identity()
                ),
                Err(Refusal::ChildUnsuccessful)
            ));
        }
    }

    #[test]
    fn foreign_invocation_or_any_closure_role_is_not_the_selected_parent() {
        let original = baseline();
        let d = original.capsule().invocation();
        for role in 0..6 {
            let mut pins = [[1; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32]];
            pins[role][0] ^= 0x80;
            let closure =
                CompilerClosureV2::new(pins[0], pins[1], pins[2], pins[3], pins[4], pins[5])
                    .unwrap();
            let changed = RustcInvocationDescriptorV3::new(
                RustcInvocationDescriptorV2::new(
                    pins[3],
                    pins[5],
                    d.rustc().clone(),
                    d.compile_environment().clone(),
                )
                .unwrap(),
                closure,
            )
            .unwrap();
            assert!(
                matches!(
                    decode(original.canonical_bytes(), &changed, original.identity()),
                    Err(Refusal::InvocationMismatch)
                ),
                "role {role}"
            );
        }
        let args =
            RustcUnitV2::new("/other", d.rustc().argv().map(str::to_owned).collect()).unwrap();
        let other = RustcInvocationDescriptorV3::new(
            RustcInvocationDescriptorV2::new(
                *d.rustc_executable_sha256(),
                *d.codegen_backend_sha256(),
                args,
                d.compile_environment().clone(),
            )
            .unwrap(),
            *d.compiler_closure(),
        )
        .unwrap();
        assert!(matches!(
            decode(original.canonical_bytes(), &other, original.identity()),
            Err(Refusal::InvocationMismatch)
        ));
    }

    #[test]
    fn canonical_other_target_is_refused_even_when_self_consistent() {
        let other = outer(invocation("gfx942:xnack-", 1), b"formal", b"proof");
        assert!(matches!(
            decode(
                other.canonical_bytes(),
                other.capsule().invocation(),
                other.identity()
            ),
            Err(Refusal::TargetMismatch)
        ));
    }

    #[test]
    fn recomputed_capsule_mutation_cannot_replace_expected_outer_identity() {
        let original = baseline();
        let selected = original.capsule().invocation();
        for changed in [
            outer(
                selected.clone(),
                b"changed-formal",
                b"OPAQUE-FIXTURE: no signed proof",
            ),
            outer(selected.clone(), b"changed-formal", b"changed-proof"),
        ] {
            assert!(matches!(
                decode(changed.canonical_bytes(), selected, original.identity()),
                Err(Refusal::UnexpectedOuterIdentity)
            ));
        }
    }

    #[test]
    fn inherited_codec_refuses_a_mismatched_final_commitment() {
        let original = baseline();
        let (capsule, _) = original.into_capsule_and_module_handoff();
        let alternate = module_handoff_alternate("gfx950:xnack-");
        assert!(matches!(
            InertSemanticCompilerModuleHandoffV3::new(capsule, alternate),
            Err(InertSemanticCompilerModuleHandoffErrorV3::FinalCommitmentMismatch)
        ));
    }

    #[test]
    fn raw_capsule_tampering_is_rejected_by_existing_strict_codec() {
        let original = baseline();
        let mut wire = original.canonical_bytes().to_vec();
        let marker = b"OPAQUE-FIXTURE";
        let offset = wire
            .windows(marker.len())
            .position(|x| x == marker)
            .unwrap();
        wire[offset] ^= 1;
        assert!(matches!(
            decode(&wire, original.capsule().invocation(), original.identity()),
            Err(Refusal::Encoding)
        ));
    }

    #[test]
    fn production_api_retains_parent_and_full_outer_without_publication_transition() {
        let source = include_str!("inert_semantic_completion_v1.rs")
            .split_once("#[cfg(test)]")
            .unwrap()
            .0;
        assert!(source.contains("_eligibility: Eligibility<'a>"));
        assert_eq!(source.matches(".revalidate_for_completion()").count(), 3);
        for forbidden in [
            "ProductionCompilerModuleHandoffIntake",
            "publish_worker_handoff",
            "into_capsule_and_module_handoff",
            "command.spawn",
            "into_parts(",
        ] {
            assert!(!source.contains(forbidden));
        }
        assert!(source.contains("Err(Refusal::NoProtectedPublicationAuthority)"));
        let preflight = include_str!("inert_semantic_extraction_v1.rs");
        assert!(preflight.contains("Err(Refusal::ExtractionOnlyConsumerUnqualified)"));
    }
}
