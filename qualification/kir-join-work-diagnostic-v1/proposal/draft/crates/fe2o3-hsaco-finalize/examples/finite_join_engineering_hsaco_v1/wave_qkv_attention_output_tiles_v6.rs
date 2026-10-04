// Inert wave-worker custody checks. These do not prove arithmetic, scheduling,
// coherence or safe launch, and never convert a conditional record to a proof.
use super::*;
use fe2o3_compiler_lineage::{
    ConditionalWaveTaskGenericProofStatusV1, ConditionalWaveTaskRuntimeStatusV1,
    InertConditionalWaveTaskAssociationV1,
};
use fe2o3_kernel_descriptor::{
    AccessMode, AliasSemantics, DeviceLayoutDescriptorV1, DeviceLayoutRecordV1, OwnershipSemantics,
    RUSTC_CODEGEN_FE2O3_COMPILER_NAME_V1, RUSTC_CODEGEN_FE2O3_PRODUCTION_V3_PRODUCER_NAME_V1,
    SourceTypeDescriptorV1, SourceTypeRecordV1, WaveQkvAttentionOutputTileRoleV6,
};
use fe2o3_kernel_ir::{
    self as kir, CanonicalKernelIrVerificationResourceBudgetV1 as Budget,
    CanonicalKernelIrWorkBudgetV1 as Work, SemanticWaveTaskProfileV3,
};
use fe2o3_lower_mir_kernel::InertCanonicalWaveQkvAttentionOutputTileFormalMemoryEvidenceV6 as Formal;
use fe2o3_mir_model::semantic_mir_v1::{
    AdmittedInertSemanticMirV1, SemanticLocalRoleV1, SemanticMirLimitsV1,
    SemanticSourceArgumentOwnershipV1, SemanticTypeShapeV1,
};
use fe2o3_pliron::{
    InertProductionMiddleEndWaveTaskEvidenceV1 as Middle,
    ProductionWaveTaskUnresolvedRequirementV1 as Requirement,
};

// Initial fixed Output V6 policy, not a measurement of this new profile.
// The retained handoff must pass the metered exact/one-short diagnostic before
// emission. Legacy profiles and decoder policies stay unchanged; one ledger
// covers archives and every join without waiving or refunding any charge.
const WORK: usize = 1usize << 30;
const STORAGE: usize = 128 * 1024 * 1024;
const PRODUCER_VERSION: &str = "conditional-wave-qkv-attention-output-tile-cov6-v6";
const REQUIRED: [Requirement; 8] = [
    Requirement::BoundsAndAlignment,
    Requirement::AllocationSeparationAndLifetime,
    Requirement::AtomicProtocolAndPublication,
    Requirement::FreshStateAndSingleActivation,
    Requirement::TaskCoverageAndNumericalRefinement,
    Requirement::SchedulerProgressAndRetirement,
    Requirement::PipelineStorageAndConvergence,
    Requirement::TargetAndLaunchContract,
];

fn sum(parts: &[usize]) -> Result<usize> {
    parts.iter().try_fold(0usize, |total, part| {
        total
            .checked_add(*part)
            .ok_or_else(|| io::Error::other("wave bridge size overflow").into())
    })
}

fn equal_bytes(a: &[u8], b: &[u8], budget: &mut Budget<'_>) -> Result<bool> {
    budget.charge_work(sum(&[a.len(), b.len(), 1])?)?;
    Ok(a == b)
}

fn restore_floor(budget: &mut Budget<'_>, floor: usize) -> Result<()> {
    let added = budget
        .storage()
        .checked_sub(floor)
        .ok_or_else(|| io::Error::other("wave bridge resource floor"))?;
    budget.release_storage(added)?;
    Ok(())
}

// The current canonical registries contain 27 float and eight diagnostic
// identities. This conservative lookup allowance covers both registries; the
// scratch reservation covers their bounded scalar declarations and operands.
const INTRINSIC_LOOKUP_ROWS: usize = 64;
const INTRINSIC_DECLARATION_SCRATCH: usize = 16 * 1024;

fn closed_archived_entry<'a>(
    module: &'a kir::Module,
    ordinal: usize,
    entry: &kir::FunctionId,
    budget: &mut Budget<'_>,
) -> Result<&'a kir::Function> {
    budget.charge_work(1)?;
    let selected = module
        .functions
        .get(ordinal)
        .ok_or_else(|| io::Error::other("wave archived entry ordinal is absent"))?;
    require(
        equal_bytes(
            selected.id.as_str().as_bytes(),
            entry.as_str().as_bytes(),
            budget,
        )?,
        "wave archived entry ordinal does not select the kernel entry",
    )?;
    budget.charge_work(module.functions.len())?;
    for (index, function) in module.functions.iter().enumerate() {
        if index == ordinal {
            require(
                function.role == kir::FunctionRole::KernelEntry && function.body.is_some(),
                "wave archived entry is not a kernel definition",
            )?;
        } else {
            require(
                function.role == kir::FunctionRole::ExternalImport && function.body.is_none(),
                "wave archived non-entry function is not a declaration",
            )?;
            // Reject recursive or oversized impostor signatures before full
            // equality. Every registered declaration has only scalar types,
            // at most three inputs, one result and one bounded capability.
            require(
                function.signature.parameters.len() <= 3
                    && function.signature.results.len() <= 1
                    && function.required_capabilities.len() <= 1,
                "wave archived intrinsic declaration exceeds its scalar bounds",
            )?;
            budget.charge_work(16)?;
            require(
                function
                    .signature
                    .parameters
                    .iter()
                    .chain(&function.signature.results)
                    .all(|ty| matches!(ty, kir::Type::Scalar(_)))
                    && function
                        .required_capabilities
                        .iter()
                        .all(|capability| match capability {
                            kir::TargetCapability::Float16 | kir::TargetCapability::BFloat16 => {
                                true
                            }
                            kir::TargetCapability::Extension { namespace, name } => {
                                namespace.len() <= 128 && name.len() <= 128
                            }
                            _ => false,
                        }),
                "wave archived intrinsic declaration has unsupported fields",
            )?;
        }

        // Comparing earlier borrowed IDs needs no additional owned index.
        // Charge every comparison, including rejected duplicate identities.
        for previous in &module.functions[..index] {
            require(
                !equal_bytes(
                    previous.id.as_str().as_bytes(),
                    function.id.as_str().as_bytes(),
                    budget,
                )?,
                "wave archived function identity is duplicated",
            )?;
        }
        let lookup_work = sum(&[function.id.as_str().len(), 2])?
            .checked_mul(INTRINSIC_LOOKUP_ROWS)
            .ok_or_else(|| io::Error::other("wave intrinsic lookup work overflow"))?;
        budget.charge_work(sum(&[lookup_work, function.id.as_str().len(), 512])?)?;
        budget.reserve_storage(INTRINSIC_DECLARATION_SCRATCH)?;
        let canonical_result = (|| {
            let canonical = kir::FloatOperation::from_intrinsic_id(&function.id)
                .map(|operation| operation.declaration())
                .or_else(|| {
                    kir::AmdGpuDiagnosticOperation::from_intrinsic_id(&function.id)
                        .map(|operation| operation.declaration())
                });
            if index == ordinal {
                require(
                    canonical.is_none(),
                    "wave entry uses a reserved intrinsic identity",
                )
            } else {
                require(
                    canonical.as_ref() == Some(function),
                    "wave archived function is not an exact canonical intrinsic declaration",
                )
            }
        })();
        // The temporary canonical declaration is dropped before its reservation.
        let released = budget.release_storage(INTRINSIC_DECLARATION_SCRATCH);
        canonical_result?;
        released?;
    }
    Ok(selected)
}

fn trace_join_work_v1(stage: &str, budget: &Budget<'_>, charge: usize) {
    if std::env::var_os("FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1").as_deref()
        == Some(std::ffi::OsStr::new("1"))
    {
        eprintln!(
            "kir-join-work-v1 stage={} work={} remaining={} storage={} peak={} charge={}",
            stage,
            budget.work(),
            budget.remaining_work(),
            budget.storage(),
            budget.peak_storage(),
            charge
        );
    }
}

pub(super) fn conditional_descriptor(
    handoff: &InertSemanticCompilerModuleHandoffV3,
) -> Result<CompilerDescriptorSourceV1> {
    let mut work = Work::new(WORK);
    let mut budget = Budget::new(&mut work, STORAGE);
    // Outer/semantic/V6 decoding retain their existing bounded policies. The
    // outer decoder shares one immutable allocation for its nested receipts.
    budget.reserve_storage(handoff.canonical_bytes().len())?;
    conditional_descriptor_with_budget(handoff, &mut budget)
}

fn conditional_descriptor_with_budget(
    handoff: &InertSemanticCompilerModuleHandoffV3,
    budget: &mut Budget<'_>,
) -> Result<CompilerDescriptorSourceV1> {
    let floor = budget.storage();
    require(
        floor >= handoff.canonical_bytes().len(),
        "unreserved wave handoff input",
    )?;
    let result = (|| {
        let capsule = handoff.capsule();
        require(
            capsule.target() == DeviceTargetV1::parse("gfx950:xnack-")?,
            "wave bridge requires exact gfx950:xnack-",
        )?;
        require(
            !handoff.authenticates_producer()
                && !handoff.grants_artifact_authority()
                && !handoff.grants_publication_authority()
                && !handoff.grants_load_authority()
                && !handoff.grants_launch_authority(),
            "inert wave outer policy",
        )?;
        let receipts = capsule.receipts();
        let association = InertConditionalWaveTaskAssociationV1::decode(
            receipts.proof_binding().canonical_preimage(),
        )?;
        require(
            association.generic_proof_status()
                == ConditionalWaveTaskGenericProofStatusV1::Unsupported
                && association.runtime_status()
                    == ConditionalWaveTaskRuntimeStatusV1::RequiredUndischarged
                && !association.grants_authority(),
            "inert wave association policy",
        )?;
        let axes = association.inputs();
        for (input, digest, length) in [
            (
                axes.semantic_mir(),
                *receipts.semantic_mir().identity().sha256(),
                receipts.semantic_mir().identity().byte_len(),
            ),
            (
                axes.middle_end(),
                *receipts.middle_end().identity().sha256(),
                receipts.middle_end().identity().byte_len(),
            ),
            (
                axes.kernel_ir(),
                *receipts.kernel_ir().identity().sha256(),
                receipts.kernel_ir().identity().byte_len(),
            ),
            (
                axes.mir_to_kir_correspondence(),
                *receipts.mir_to_kir_correspondence().identity().sha256(),
                receipts.mir_to_kir_correspondence().identity().byte_len(),
            ),
            (
                axes.formal_memory(),
                *receipts.formal_memory().identity().sha256(),
                receipts.formal_memory().identity().byte_len(),
            ),
        ] {
            budget.charge_work(40)?;
            require(
                input.sha256() == digest && input.byte_len() == length,
                "wave five-stage association changed",
            )?;
        }

        let semantic = AdmittedInertSemanticMirV1::decode_exact_v41_canonical(
            receipts.semantic_mir().canonical_preimage(),
            SemanticMirLimitsV1::default(),
        )?;
        trace_join_work_v1("outer-middle-before", budget, 0);
        let middle =
            Middle::decode_with_budget(receipts.middle_end().canonical_preimage(), budget)?;
        trace_join_work_v1("outer-middle-after", budget, 0);
        trace_join_work_v1("outer-formal-before", budget, 0);
        let formal =
            Formal::decode_with_budget(receipts.formal_memory().canonical_preimage(), budget)?;
        trace_join_work_v1("outer-formal-after", budget, 0);
        let correspondence = InertCanonicalMirToKirCorrespondenceEvidenceV5::decode(
            receipts.mir_to_kir_correspondence().canonical_preimage(),
        )?;
        let structural = correspondence.nested_v4();
        require(
            equal_bytes(
                formal.middle_end_evidence().canonical_bytes(),
                middle.canonical_bytes(),
                budget,
            )? && equal_bytes(
                formal.canonical_kernel_ir_bytes(),
                receipts.kernel_ir().canonical_preimage(),
                budget,
            )?,
            "wave exact parent or published KIR preimage changed",
        )?;
        budget.charge_work(256)?;
        require(
            middle.source_semantic_identity() == semantic.semantic_sha256().as_bytes()
                && structural.semantic_sha256() == semantic.semantic_sha256().as_bytes()
                && structural.semantic_u32_induction().semantic_mir_sha256()
                    == semantic.semantic_sha256().as_bytes()
                && formal.canonical_kernel_ir_identity()
                    == structural.canonical_kernel_ir_identity()
                && !correspondence.grants_authority()
                && !structural.semantic_u32_induction().grants_authority()
                && !formal.grants_authority()
                && !middle.claims_verus_verification()
                && !middle.claims_full_arithmetic_correctness()
                && !middle.grants_artifact_or_launch_authority(),
            "wave source/neutral/correspondence policy changed",
        )?;
        require(
            formal.unresolved_requirements() == &REQUIRED
                && middle.unresolved_requirements() == &REQUIRED,
            "wave unresolved requirement roster changed",
        )?;

        let [root] = semantic.roots() else {
            return Err(io::Error::other("one wave semantic root required").into());
        };
        let source = &semantic.functions()[root.index() as usize];
        let selected = semantic
            .select_kernel_body_for_root_v1(*root)
            .ok_or_else(|| io::Error::other("wave selected body missing"))?;
        let source_body = &semantic.functions()[selected.body().index() as usize];
        let entry = source
            .kernel_entry()
            .ok_or_else(|| io::Error::other("wave entry missing"))?;
        let source_launch = entry
            .source_contract()
            .launch()
            .ok_or_else(|| io::Error::other("wave source launch missing"))?;
        require(
            source.abi().source_argument_ownership()
                == [SemanticSourceArgumentOwnershipV1::WaveQkvAttentionOutputTileStorage64OwnerV6]
                && source_body.abi().source_argument_ownership()
                    == source.abi().source_argument_ownership()
                && structural.semantic_u32_induction().function() == selected.body().index()
                && source_launch
                    .required()
                    .is_some_and(|size| size.as_array() == [64, 1, 1])
                && source_launch
                    .maximum()
                    .is_some_and(|size| size.as_array() == [64, 1, 1])
                && entry
                    .source_contract()
                    .resources()
                    .is_some_and(|resources| {
                        resources.static_shared_memory_bytes() == 512
                            && resources.max_dynamic_shared_memory_bytes() == 0
                    }),
            "wave source ownership or selected body changed",
        )?;

        let [kernel] = formal.module().kernels.as_slice() else {
            return Err(io::Error::other("one wave archived kernel required").into());
        };
        let function = closed_archived_entry(
            formal.module(),
            middle.function_ordinal() as usize,
            &kernel.entry,
            budget,
        )?;
        let body = function
            .body
            .as_ref()
            .ok_or_else(|| io::Error::other("wave archived definition missing"))?;
        validate_prefix_tile_lds(function, budget)?;
        let [function_binding] = correspondence.functions() else {
            return Err(io::Error::other("one wave correspondence function required").into());
        };
        // Charge each compared string's full length before any equality scan.
        budget.charge_work(sum(&[
            entry.export_symbol().as_bytes().len(),
            function_binding.kernel_ir_function().len(),
            function.id.as_str().len(),
            function.id.as_str().len(),
            kernel.entry.as_str().len(),
            kernel.entry.as_str().len(),
            kernel.id.as_str().len(),
            kernel.id.as_str().len(),
            96,
        ])?)?;
        require(
            middle.function_ordinal() == function_binding.kernel_ir_function_ordinal()
                && function_binding.correspondence_owner() == root.index()
                && function_binding.semantic_function() == selected.body().index()
                && function_binding.kernel_ir_function() == function.id.as_str()
                && function.role == kir::FunctionRole::KernelEntry
                && kernel.entry == function.id
                && kernel.id.as_str().as_bytes() == entry.export_symbol().as_bytes()
                && kernel.entry.as_str() == kernel.id.as_str()
                && formal.rank() == 1
                && formal.witness_extents() == [4096, 1, 1]
                && formal.workgroup_extents() == [64, 1, 1]
                && kernel.domain.rank() == 1
                && kernel
                    .workgroup_size
                    .is_some_and(|size| [size.x, size.y, size.z] == [64, 1, 1])
                && formal.roots().len() == 15
                && body.parameters.len() == 15
                && function.signature.parameters.len() == 15,
            "wave decoded source/KIR/function/geometry changed",
        )?;

        validate_attention_ocml_contract(
            function,
            handoff.module_handoff().envelope(),
            handoff.module_handoff().module_bytes(),
            formal.canonical_kernel_ir_identity().digest(),
            budget,
        )?;

        let [carrier_type] = source_body.abi().source_input_types() else {
            return Err(io::Error::other("one wave source carrier ABI type required").into());
        };
        require(
            source.abi().source_input_types() == source_body.abi().source_input_types(),
            "wave entry/body carrier ABI type changed",
        )?;
        // V41 admission checks the exact seventeen-field carrier (fifteen roots
        // plus two ZST markers), pointer kind/mutability/address space/width,
        // fixed extents, U16/F32/ordinary U32 and nominal CoreAtomicU32 layouts.
        // This is structural
        // custody, not producer/provider authentication by an inert decoder.
        for (index, role) in SemanticWaveTaskProfileV3::QkvAttentionOutputTilesV6
            .roles()
            .iter()
            .copied()
            .enumerate()
        {
            budget.charge_work(64)?;
            let physical = &formal.roots()[index];
            let local = source_body
                .locals()
                .get(physical.source_local as usize)
                .ok_or_else(|| io::Error::other("wave source local missing"))?;
            let carrier = semantic
                .types()
                .get(local.ty().index() as usize)
                .ok_or_else(|| io::Error::other("wave source carrier missing"))?;
            let SemanticTypeShapeV1::Aggregate(fields) = carrier.shape() else {
                return Err(io::Error::other("wave source carrier shape changed").into());
            };
            let scalar = role.scalar();
            let access = if role.is_read_only() {
                kir::AccessMode::ReadOnly
            } else {
                kir::AccessMode::ReadWrite
            };
            let pointer_matches = matches!(&function.signature.parameters[index],
                kir::Type::Pointer(pointer) if pointer.address_space == kir::AddressSpace::Global
                    && pointer.pointee.as_ref() == &kir::Type::Scalar(scalar)
                    && pointer.access == access);
            require(
                physical.role == role
                    && physical.source_argument == 0
                    && physical.parameter_index == index as u32
                    && physical.parameter == body.parameters[index]
                    && physical.allocation_origin == role.logical_allocation_origin(0)
                    && local.role() == SemanticLocalRoleV1::Argument(0)
                    && local.ty() == *carrier_type
                    && fields.fields().len() == 17
                    && fields.fields()[index].index() == physical.component_type
                    && pointer_matches,
                "wave original source role/type/entry parameter changed",
            )?;
        }

        // The formal decoder has already replayed the entire actual operation
        // census, including full atomics, Private/LDS and synchronization. Rejoin
        // each non-private source site to the decoded source and V4 span, not a
        // role inferred from the pointer type or a status label.
        for row in formal.memory_census() {
            budget.charge_work(1)?;
            let Some((block, statement, _ordinal)) = row.source_site else {
                require(
                    row.memory_space_tag == 2 && row.ranked_site.is_none(),
                    "only compiler-private memory may omit source/ranked sites",
                )?;
                continue;
            };
            require(
                row.memory_space_tag != 2 && row.ranked_site.is_some(),
                "wave external memory source/ranked site changed",
            )?;
            let source_block = source_body
                .blocks()
                .get(block as usize)
                .ok_or_else(|| io::Error::other("wave memory source block missing"))?;
            if let Some(statement) = statement {
                require(
                    (statement as usize) < source_block.statements().len(),
                    "wave memory source statement missing",
                )?;
            }
            budget.charge_work(body.blocks.len())?;
            let ir_block = body
                .blocks
                .iter()
                .find(|candidate| candidate.id == row.location.block)
                .ok_or_else(|| io::Error::other("wave memory KIR block missing"))?;
            let mut matches = 0usize;
            if let Some(statement) = statement {
                for span in structural.statement_spans() {
                    budget.charge_work(1)?;
                    if span.semantic_function() == selected.body().index()
                        && span.semantic_block() == block
                        && span.statement() == statement
                        && span.kernel_ir_block() == ir_block.id.0
                        && covers(
                            span.first_operation(),
                            span.operation_count(),
                            row.location.operation_index,
                        )?
                    {
                        matches += 1;
                    }
                }
            } else {
                for span in structural.terminator_spans() {
                    budget.charge_work(1)?;
                    if span.semantic_function() == selected.body().index()
                        && span.semantic_block() == block
                        && span.kernel_ir_block() == ir_block.id.0
                        && covers(
                            span.first_operation(),
                            span.operation_count(),
                            row.location.operation_index,
                        )?
                    {
                        matches += 1;
                    }
                }
            }
            require(
                matches == 1,
                "wave source effect does not have one exact correspondence span",
            )?;
        }

        let descriptor = CompilerDescriptorSourceV1::decode(receipts.abi().canonical_preimage())?;
        validate_table(descriptor.table())?;
        let described = &descriptor.table().kernels()[0];
        budget.charge_work(sum(&[
            described.entry_name().as_str().len(),
            described.entry_name().as_str().len(),
            entry.export_symbol().as_bytes().len(),
            described.descriptor_symbol().as_str().len(),
            34,
        ])?)?;
        require(
            described.kernel_id().as_bytes() == entry.kernel_binding_identity().as_bytes()
                && described.entry_name().as_str().as_bytes() == entry.export_symbol().as_bytes()
                && described.descriptor_symbol().as_str().strip_suffix(".kd")
                    == Some(described.entry_name().as_str()),
            "wave source entry/descriptor binding changed",
        )?;
        Ok(descriptor)
    })();
    // Decoded records drop on either branch before the caller's floor returns.
    restore_floor(budget, floor)?;
    result
}

fn validate_prefix_tile_lds(entry: &kir::Function, budget: &mut Budget<'_>) -> Result<()> {
    let body = entry
        .body
        .as_ref()
        .ok_or_else(|| io::Error::other("prefix tile body missing"))?;
    let mut bytes = 0u64;
    for block in &body.blocks {
        budget.charge_work(1)?;
        for operation in &block.operations {
            budget.charge_work(1)?;
            if let kir::OperationKind::WorkgroupMemory(memory) = &operation.kind {
                require(
                    memory.element == kir::Type::Scalar(kir::ScalarType::U32),
                    "prefix tile LDS requires its actual U32 exchange storage",
                )?;
                let kir::WorkgroupMemoryExtent::Static(elements) = &memory.extent else {
                    return Err(io::Error::other("prefix tile dynamic LDS is unsupported").into());
                };
                bytes = bytes
                    .checked_add(
                        u64::from(*elements)
                            .checked_mul(4)
                            .ok_or_else(|| io::Error::other("prefix tile LDS extent overflow"))?,
                    )
                    .ok_or_else(|| io::Error::other("prefix tile LDS sum overflow"))?;
            }
        }
    }
    require(
        bytes == 512,
        "prefix tile source and actual LDS census must both be 512",
    )
}

// Exact exp-only linkage is source-derived, not an arbitrary external provider.
// The archive still carries the ordinary source body and its checked call site.
fn validate_attention_ocml_contract(
    entry: &kir::Function,
    envelope: &fe2o3_compiler_ffi::CompilerFfiEnvelopeV1,
    llvm_bytes: &[u8],
    neutral_identity: &[u8; 32],
    budget: &mut Budget<'_>,
) -> Result<()> {
    use fe2o3_compiler_ffi::{
        ProductionGfx950CompilerFfiEnvelopeKindV1,
        inspect_production_gfx950_compiler_ffi_envelope_v1,
    };
    let inspection_work = envelope
        .canonical_bytes()
        .len()
        .checked_mul(64)
        .ok_or_else(|| io::Error::other("attention envelope work overflow"))?;
    let llvm_work = llvm_bytes
        .len()
        .checked_mul(8)
        .ok_or_else(|| io::Error::other("attention LLVM scan work overflow"))?;
    let charge = sum(&[inspection_work, llvm_work, 256])?;
    trace_join_work_v1("ocml-before", budget, charge);
    budget.charge_work(charge)?;
    trace_join_work_v1("ocml-after", budget, 0);
    require(
        matches!(inspect_production_gfx950_compiler_ffi_envelope_v1(envelope),
            Some(ProductionGfx950CompilerFfiEnvelopeKindV1::OcmlExpF32 {
                canonical_kernel_ir_identity,
            }) if canonical_kernel_ir_identity == *neutral_identity),
        "attention requires the exact source-bound gfx950 OCML exp envelope",
    )?;
    let llvm = std::str::from_utf8(llvm_bytes)?;
    require(
        llvm.matches("declare float @__ocml_exp_f32(float)").count() == 1
            && llvm.matches("call float @__ocml_exp_f32(float ").count() >= 1
            && llvm
                .split("@__ocml_")
                .skip(1)
                .all(|suffix| suffix.starts_with("exp_f32(")),
        "attention LLVM does not retain the closed OCML exp-only shape",
    )?;
    let body = entry
        .body
        .as_ref()
        .ok_or_else(|| io::Error::other("attention entry definition missing"))?;
    let mut found = false;
    for block in &body.blocks {
        budget.charge_work(1)?;
        for operation in &block.operations {
            budget.charge_work(1)?;
            let kir::OperationKind::Call { callee, .. } = &operation.kind else {
                continue;
            };
            let lookup_work = sum(&[callee.as_str().len(), 2])?
                .checked_mul(INTRINSIC_LOOKUP_ROWS)
                .ok_or_else(|| io::Error::other("attention intrinsic work overflow"))?;
            budget.charge_work(lookup_work)?;
            found |= matches!(
                kir::FloatOperation::from_intrinsic_id(callee),
                Some(kir::FloatOperation::F32Math {
                    function: kir::F32MathFunction::Exp,
                    implementation: kir::F32MathImplementation::OcmlAbiV1,
                    ..
                })
            );
        }
    }
    require(
        found,
        "attention archive has no retained authenticated exp call",
    )
}

fn covers(first: u32, count: u32, operation: usize) -> Result<bool> {
    let end = first
        .checked_add(count)
        .ok_or_else(|| io::Error::other("wave span overflow"))?;
    Ok(first as usize <= operation && operation < end as usize)
}

type Component = (
    PhysicalAbiComponentKind,
    u32,
    u16,
    u16,
    AccessMode,
    AliasSemantics,
);
fn validate_components(components: impl ExactSizeIterator<Item = Component>) -> Result<()> {
    require(components.len() == 15, "wave physical component count")?;
    for (actual, role) in components.zip(WaveQkvAttentionOutputTileRoleV6::ALL) {
        require(
            actual
                == (
                    PhysicalAbiComponentKind::WaveQkvAttentionOutputTilePointer(role),
                    u32::from(role.field_index()) * 8,
                    8,
                    8,
                    role.access(),
                    role.alias(),
                ),
            "wave ordered role/access/alias/offset changed",
        )?;
    }
    Ok(())
}

fn validate_table(table: &DeviceDescriptorTableV1) -> Result<()> {
    let source = SourceTypeRecordV1::new(
        SourceTypeDescriptorV1::wave_qkv_attention_output_tile_storage_64_v6(),
    );
    let layout = DeviceLayoutRecordV1::new(
        DeviceLayoutDescriptorV1::wave_qkv_attention_output_tile_storage_64_v6(),
    );
    let [kernel] = table.kernels() else {
        return Err(io::Error::other("one wave descriptor kernel required").into());
    };
    require(
        table.device_target() == DeviceTargetV1::parse("gfx950:xnack-")?
            && table.code_object_version() == CodeObjectVersion::V6
            && table.compiler().name().as_str() == RUSTC_CODEGEN_FE2O3_COMPILER_NAME_V1
            && table.producer().name().as_str()
                == RUSTC_CODEGEN_FE2O3_PRODUCTION_V3_PRODUCER_NAME_V1
            && table.producer().version().as_str() == PRODUCER_VERSION
            && table.type_records() == [source.clone()]
            && table.layout_records() == [layout.clone()],
        "wave descriptor target/type/layout changed",
    )?;
    let [argument] = kernel.arguments() else {
        return Err(io::Error::other("one wave logical argument required").into());
    };
    require(
        argument.source_index() == 0
            && argument.source_type() == source.identity()
            && argument.device_layout() == layout.identity()
            && argument.ownership()
                == OwnershipSemantics::WaveQkvAttentionOutputTileStorage64OwnerV6
            && argument.access() == AccessMode::ComponentWise
            && argument.alias() == AliasSemantics::ComponentWise
            && kernel.abi_layout().explicit_argument_size() == 120
            && kernel.abi_layout().kernarg_segment_size() == 376
            && kernel.abi_layout().kernarg_segment_alignment() == 8,
        "wave logical ownership or 376-byte ABI changed",
    )?;
    validate_components(argument.physical_component_contracts())?;
    let launch = kernel.launch();
    require(
        launch.rank() == 1
            && launch.block_size() == BlockSizeV1::Exact(DimensionsV1::new(64, 1, 1)?)
            && launch.max_grid() == DimensionsV1::new(64, 1, 1)?
            && launch.max_flat_workgroup_size() == 64
            && launch.static_shared_memory_bytes() == 512
            && launch.max_dynamic_shared_memory_bytes() == 0,
        "wave source launch or LDS bounds changed",
    )
}

pub(super) fn validate_finalized(
    inspection: &fe2o3_hsaco_finalize::FinalizedDescriptorInspection,
) -> Result<()> {
    validate_table(inspection.descriptor_table())?;
    let [metadata] = inspection.hsaco().kernels() else {
        return Err(io::Error::other("one wave metadata kernel required").into());
    };
    require(
        metadata.wavefront_size() == 64
            && metadata.private_segment_fixed_size() == 0
            && metadata.group_segment_fixed_size()
                == u64::from(
                    inspection.descriptor_table().kernels()[0]
                        .launch()
                        .static_shared_memory_bytes(),
                )
            && metadata.kernarg_segment_size() == 376
            && metadata.kernarg_segment_alignment() == 8
            && metadata.explicit_arguments().len() == 15
            && metadata.implicit_argument_offset() == Some(120)
            && metadata.implicit_argument_size() == 256,
        "wave final physical resources differ from the native engineering runner",
    )
}

#[cfg(test)]
mod tests {
    include!("wave_qkv_attention_output_tiles_v6_tests.rs");
}
