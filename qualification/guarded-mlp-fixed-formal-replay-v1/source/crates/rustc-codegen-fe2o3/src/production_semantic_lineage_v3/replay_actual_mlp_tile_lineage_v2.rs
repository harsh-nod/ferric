// Descriptor construction precedes final outer replay in the extraction path.
// Exercise conditional replay independently of the descriptor factory.
#[cfg(test)]
pub(crate) fn replay_actual_mlp_tile_lineage_v2(
    owner: &fe2o3_lower_mir_kernel::ProductionConditionalWaveMlpTileFormalMemoryOwnerV2,
    ranked: AuthenticatedRankedVerificationRosterV1,
) -> Result<(), ProductionSemanticLineageErrorV3> {
    let semantic = owner.semantic_kir().semantic().semantic();
    assert_eq!(
        crate::production_pipeline::wave_source_profile_v3(semantic),
        Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::MlpTilesV2)
    );
    let source =
        InertCanonicalSemanticMirReceiptV3::from_canonical_preimage(semantic.canonical_encoding())?;
    let neutral = owner.semantic_kir().canonical_kernel_ir_identity();
    let (target, _) = dialect_amdgcn::bind_production_target_v1(
        owner.semantic_kir().module(),
        fe2o3_amd_target::ProductionAmdTargetProfileV1::Gfx950,
    )
    .map_err(|error| ProductionSemanticLineageErrorV3::LiveOwner(error.to_string()))?
    .into_parts();
    let incoming = wave_lineage_storage_sum_v1(&[
        source.canonical_preimage().len(),
        std::mem::size_of::<InertCanonicalSemanticMirReceiptV3>(),
    ])?;
    let prepared =
        prepare_wave_mlp_tile_lineage_evidence_v2(ranked, owner, &target, neutral, incoming)?;
    assert!(matches!(
        &prepared.roster_custody,
        PreparedLineageRosterCustodyV1::ConditionalWaveTasks
    ));
    assert!(prepared.proof_custody.signed_verus().is_err());
    validate_wave_task_lineage_receipts_v1(
        &source,
        &prepared.middle_end,
        &prepared.formal_memory,
        &prepared.mir_to_kir_correspondence,
        neutral,
        0,
    )?;
    eprintln!(
        "stage=outer-lineage-replay status=complete formal_bytes={}",
        prepared.formal_memory.canonical_preimage().len()
    );
    Ok(())
}
