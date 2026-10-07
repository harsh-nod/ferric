//! Diagnostic-only admission for the measured nine-image, prefill16 V19 route.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpBatchExecutionV2, EngineeringTpPagedPoolV1,
    EngineeringTpRankTransportV1, EngineeringTpWaveTargetArtifactsV17, ModelConfig, TpResult,
};
use crate::tp_artifact::{
    C1KvCopyBindingV19, EngineeringTpArtifactV1, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20,
    PrefillKvCopyBindingV27, QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
};

fn require_transport<R: EngineeringTpRankTransportV1>(transports: &[R]) -> TpResult<()> {
    if transports.len() != 1
        || !transports[0].supports_ordered_batches64()
        || !transports[0].model_timestamps_enabled()
        || !transports[0].model_timestamp_ordered64_enabled()
        || transports[0].peer_group_rank().is_some()
    {
        return Err(
            "V19 packet ticks require the explicitly marked nonpeer ordered64 timestamp transport"
                .into(),
        );
    }
    Ok(())
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: C1KvCopyBindingV19,
) -> TpResult<()> {
    require_transport(transports)?;
    super::c1_kv_copy_v19::admit(transports, binding)
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits the same ninth image before the unchanged eight-image allocation path.
    /// # Errors
    /// Rejects a missing explicit timestamp marker or image and closes owned transports.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_ordered64_kv_copy_packet_ticks_v1(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        prefill: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
        copy: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        let admitted = (|| {
            let binding = copy
                .c1_kv_copy_binding_v19()
                .ok_or("V19 packet ticks require the actual separate copy image")?;
            admit(&mut transports, binding)?;
            Ok::<_, String>(binding)
        })();
        let binding = match admitted {
            Ok(binding) => binding,
            Err(error) => {
                for transport in &mut transports {
                    let _ = transport.close();
                }
                return Err(error);
            }
        };
        let mut driver = Self::new_wide32_with_prefill_decode_gemv_v28(
            transports, model, weights, layout, pool, artifacts, prefill, split, gemv,
        )?;
        driver.admitted_c1_kv_copy_v19 = Some(binding);
        Ok(driver)
    }

    /// Selects the measured V19/V27/split8/packed64/baseline-GEMV route for raw ticks.
    /// # Errors
    /// Rejects the baseline arm, unadmitted bindings, marker drift or incompatible state.
    #[allow(clippy::too_many_arguments)]
    pub fn configure_ordered64_kv_copy_packet_ticks_v1(
        &mut self,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        prefill: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
        copy: &EngineeringTpArtifactV1,
        enabled: bool,
    ) -> TpResult<()> {
        let ((argmax, attention), rmsnorm) = artifacts
            .argmax
            .fp32_argmax_binding_v11()
            .zip(artifacts.attention.query_hoist_binding_v14())
            .zip(artifacts.rmsnorm.wave_rmsnorm_binding_v15())
            .ok_or("V19 packet ticks require all historical V17 images")?;
        self.configure_ordered64_kv_copy_packet_tick_bindings_v1(
            argmax,
            attention,
            rmsnorm,
            prefill
                .prefill_kv_copy_binding_v27()
                .ok_or("V19 packet ticks require V27")?,
            split
                .split_attention_binding_v21()
                .ok_or("V19 packet ticks require V21")?,
            gemv.gemv_prefetch_binding_v20()
                .ok_or("V19 packet ticks require baseline V20")?,
            copy.c1_kv_copy_binding_v19()
                .ok_or("V19 packet ticks require V19")?,
            enabled,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn configure_ordered64_kv_copy_packet_tick_bindings_v1(
        &mut self,
        argmax: Fp32ArgmaxBindingV11,
        attention: QueryHoistBindingV14,
        rmsnorm: WaveRmsNormBindingV15,
        prefill: PrefillKvCopyBindingV27,
        split: SplitAttentionBindingV21,
        gemv: GemvPrefetchBindingV20,
        copy: C1KvCopyBindingV19,
        enabled: bool,
    ) -> TpResult<()> {
        require_transport(&self.inner.transports)?;
        if !enabled || self.inner.ordered_batch_width.is_wide() || self.prefill32_pages_v1.is_some()
        {
            return Err("V19 packet ticks require fresh prefill16 candidate selection".into());
        }
        self.configure_ordered64_kv_copy_checked_bindings_v1(
            argmax, attention, rmsnorm, prefill, split, gemv, copy, true,
        )
    }
}
