//! Optional single-row partial GEMV over the unchanged composed execution schedule.

use super::{AuthenticatedModelWeightLayout, EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2,
    EngineeringTpDispatchV1, EngineeringTpPagedPoolV1, EngineeringTpRankTransportV1,
    EngineeringTpWaveTargetArtifactsV17, ModelConfig, TpResult};
use crate::tp_artifact::{ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20, EngineeringTpArtifactV1,
    GemvPrefetchBindingV20};

/// Explicit single-row partial policy for the separately admitted V20 image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineeringTpPartialGemvModeV28 {
    /// Preserve the existing partial projection kernel.
    Baseline,
    /// Select the V20 four-step-prefetch partial kernel.
    Prefetch4,
}

impl From<bool> for EngineeringTpPartialGemvModeV28 {
    fn from(enabled: bool) -> Self {
        if enabled { Self::Prefetch4 } else { Self::Baseline }
    }
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    image: GemvPrefetchBindingV20,
) -> TpResult<()> {
    if transports.len() != 1 || transports[0].peer_group_rank().is_some() {
        return Err("partial GEMV requires one nonpeer TP1 transport".into());
    }
    transports[0].require_loaded_image(image.hsaco, &ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20)
}

pub(super) fn select_partial(
    enabled: bool,
    mut command: EngineeringTpDispatchV1,
) -> EngineeringTpDispatchV1 {
    if enabled
        && command.kernel == "ferric_qwen3_tp_wave_gemv_partial_f32_v3"
        && command.arguments.get(3) == Some(&EngineeringTpArgumentV1::U32(1))
    {
        command.kernel = ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20[1];
    }
    command
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits V20 in both arms before the existing seven-image allocation path.
    /// # Errors
    /// Rejects missing/wrong loaded images before allocation and closes owned transports.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_prefill_decode_gemv_v28(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        let admitted = gemv.gemv_prefetch_binding_v20()
            .ok_or_else(|| String::from("partial GEMV requires its exact V20 image"))
            .and_then(|binding| admit(&mut transports, binding).map(|()| binding));
        let binding = match admitted {
            Ok(binding) => binding,
            Err(error) => {
                for transport in &mut transports {
                    let _ = transport.close();
                }
                return Err(error);
            }
        };
        let mut driver = Self::new_wide32_with_prefill_decode_v28(
            transports, model, weights, layout, pool, artifacts, copy, split,
        )?;
        driver.admitted_partial_gemv_v20 = Some(binding);
        Ok(driver)
    }

    /// Selects the existing composition plus one explicit single-row partial policy.
    /// # Errors
    /// Rejects missing images, repeated selection and the existing composition exclusions.
    #[allow(clippy::too_many_arguments)]
    pub fn configure_ordered_prefill_decode_gemv_v28(
        &mut self,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
        prefill_enabled: bool,
        split_enabled: bool,
        packed: bool,
        mode: EngineeringTpPartialGemvModeV28,
    ) -> TpResult<()> {
        let binding = gemv.gemv_prefetch_binding_v20()
            .ok_or("partial GEMV requires its exact V20 image")?;
        let copy = copy.prefill_kv_copy_binding_v27().ok_or("composition requires the exact V27 image")?;
        let split = split.split_attention_binding_v21().ok_or("composition requires the exact V21 image")?;
        let ((argmax, attention), rmsnorm) = artifacts.argmax.fp32_argmax_binding_v11()
            .zip(artifacts.attention.query_hoist_binding_v14())
            .zip(artifacts.rmsnorm.wave_rmsnorm_binding_v15())
            .ok_or("composition requires all historical V17 images")?;
        self.configure_partial_gemv_bindings_v28(argmax, attention, rmsnorm, copy, split,
            binding, prefill_enabled, split_enabled, packed, mode)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn configure_partial_gemv_bindings_v28(
        &mut self,
        argmax: crate::tp_artifact::Fp32ArgmaxBindingV11,
        attention: crate::tp_artifact::QueryHoistBindingV14,
        rmsnorm: crate::tp_artifact::WaveRmsNormBindingV15,
        copy: crate::tp_artifact::PrefillKvCopyBindingV27,
        split: crate::tp_artifact::SplitAttentionBindingV21,
        binding: GemvPrefetchBindingV20,
        prefill_enabled: bool,
        split_enabled: bool,
        packed: bool,
        mode: EngineeringTpPartialGemvModeV28,
    ) -> TpResult<()> {
        self.validate_partial_gemv_v28(binding)?;
        self.configure_prefill_decode_bindings_v28(
            argmax, attention, rmsnorm, copy, split, prefill_enabled, split_enabled, packed,
        )?;
        let enabled = mode == EngineeringTpPartialGemvModeV28::Prefetch4;
        self.partial_gemv_v28 = Some(enabled);
        Ok(())
    }

    pub(super) fn validate_partial_gemv_v28(&self, binding: GemvPrefetchBindingV20) -> TpResult<()> {
        if self.admitted_partial_gemv_v20 != Some(binding) || self.partial_gemv_v28.is_some() {
            return Err("partial GEMV requires its admitted image and fresh selection".into());
        }
        Ok(())
    }

    /// Actual selected policy. Single-row prefill tails share the same exact partial arithmetic.
    #[must_use]
    pub const fn partial_gemv_mode(&self) -> &'static str {
        if matches!(self.partial_gemv_v28, Some(true)) {
            "partial-prefetch4-v20"
        } else {
            "baseline"
        }
    }
}
