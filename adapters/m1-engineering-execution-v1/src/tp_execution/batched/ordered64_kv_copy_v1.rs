//! Explicit V19 control/candidate over the fixed V27/split8/ordered64 composition.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpBatchExecutionV2, EngineeringTpPagedPoolV1,
    EngineeringTpRankTransportV1, EngineeringTpWaveTargetArtifactsV17,
    EngineeringTpWaveTargetModeV17, ModelConfig, OrderedBatchWidth, TpResult,
};
use crate::tp_artifact::{
    C1KvCopyBindingV19, EngineeringTpArtifactV1, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20,
    PrefillKvCopyBindingV27, QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
};

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: C1KvCopyBindingV19,
) -> TpResult<()> {
    if !cfg!(feature = "c1-ordered64")
        || cfg!(feature = "model-timestamps")
        || transports.len() != 1
        || !transports[0].supports_ordered_batches64()
    {
        return Err("KV composition requires a dedicated non-timestamp ordered64 transport".into());
    }
    super::c1_kv_copy_v19::admit(transports, binding)
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits the ninth image before the unchanged eight-image allocation path.
    /// Both control and candidate use this constructor and identical storage.
    /// # Errors
    /// Rejects unsupported builds/transports or images before allocation; closes owned transports.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_ordered64_kv_copy_v1(
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
                .ok_or("KV composition requires its exact separate V19 image")?;
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

    /// Atomically selects V27 prefill, split8 attention, baseline GEMV and ordered64.
    /// Only the C1 copy policy varies; historical selectors retain their exclusions.
    /// # Errors
    /// Rejects wrong bindings, unsupported builds, stale storage or incompatible execution.
    #[allow(clippy::too_many_arguments)]
    pub fn configure_ordered64_kv_copy_v1(
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
            .ok_or("KV composition requires all historical V17 images")?;
        self.configure_ordered64_kv_copy_bindings_v1(
            argmax,
            attention,
            rmsnorm,
            prefill
                .prefill_kv_copy_binding_v27()
                .ok_or("KV composition requires V27")?,
            split
                .split_attention_binding_v21()
                .ok_or("KV composition requires V21")?,
            gemv.gemv_prefetch_binding_v20()
                .ok_or("KV composition requires V20")?,
            copy.c1_kv_copy_binding_v19()
                .ok_or("KV composition requires V19")?,
            enabled,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn configure_ordered64_kv_copy_bindings_v1(
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
        if !cfg!(feature = "c1-ordered64")
            || cfg!(feature = "model-timestamps")
            || self.inner.ordered_batch_width.is_wide()
            || self.inner.transports.len() != 1
            || !self.inner.transports[0].supports_ordered_batches64()
        {
            return Err(
                "KV composition requires its fresh non-timestamp ordered64 transport".into(),
            );
        }
        self.configure_ordered64_kv_copy_checked_bindings_v1(
            argmax, attention, rmsnorm, prefill, split, gemv, copy, enabled,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn configure_ordered64_kv_copy_checked_bindings_v1(
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
        self.validate_copy_storage_v19(copy)?;
        self.validate_prefill_storage_with_c1_v1(prefill, true, Some(copy))?;
        self.validate_split_storage_with_c1_v1(split, true, Some(copy))?;
        self.validate_partial_gemv_v28(gemv)?;
        self.validate_c1_packet_packing_idle()?;
        self.configure_ordered_c1_wave_target_bindings_v17(
            argmax,
            attention,
            rmsnorm,
            EngineeringTpWaveTargetModeV17::Combined,
        )?;
        // No fallible operation follows the first selection mutation.
        self.prefill_kv_copy_v28 = Some(true);
        self.c1_split_attention_v25 = Some(true);
        self.partial_gemv_v28 = Some(false);
        self.c1_packet_packing_v22 = Some(true);
        self.c1_kv_copy_v19 = enabled.then_some(copy);
        self.inner.ordered_batch_width = OrderedBatchWidth::Packets64;
        Ok(())
    }
}
