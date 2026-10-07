//! Opt-in full-page prefill copy; prepared pool/COW ownership remains authoritative.

use super::{
    EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2, EngineeringTpDispatchV1,
    EngineeringTpRankTransportV1, EngineeringTpWaveTargetArtifactsV17,
    EngineeringTpWaveTargetModeV17, PAGE_TOKENS, Rank, Tensor, TpResult, dispatch,
};
use crate::tp_artifact::{
    ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27, EngineeringTpArtifactV1, PrefillKvCopyBindingV27,
};
use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};
use crate::tp_paged::{EngineeringTpPagedPoolV1, EngineeringTpPreparedBatchV1};
use ferric_build::AuthenticatedModelWeightLayout;
use ferric_spec::ModelConfig;

const ELEMENTS: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PrefillPageV28 {
    first: u32,
    page: u32,
    pages: u32,
    byte_offset: usize,
    rotation: u32,
}

impl PrefillPageV28 {
    pub(super) fn select(
        batch: &EngineeringTpPreparedBatchV1,
        order: &[usize],
        pages: u32,
        context: u32,
    ) -> TpResult<Option<Self>> {
        if !(1..=512).contains(&pages) || !(1..=8192).contains(&context) {
            return Err("prefill copy storage bounds differ".into());
        }
        let rows = batch.rows();
        if rows.len() != 16 {
            return Ok(None);
        }
        if order.len() != 16
            || order.iter().any(|&index| index >= 16)
            || order
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != 16
        {
            return Err("prefill copy requires a valid complete execution permutation".into());
        }
        let first = rows[0].position();
        if first > 8176
            || !first.is_multiple_of(PAGE_TOKENS)
            || rows.iter().zip(first..first + 16).any(|(row, position)| {
                row.sequence() != rows[0].sequence() || row.position() != position
            })
        {
            return Ok(None);
        }
        let page = rows[0].writable_physical_page();
        if rows.iter().any(|row| row.writable_physical_page() != page) {
            return Ok(None);
        }
        if page >= pages
            || first + 15 >= context
            || rows.iter().any(|row| {
                row.writable_token_offset() != row.position() % PAGE_TOKENS
                    || row
                        .physical_pages()
                        .get((row.position() / PAGE_TOKENS) as usize)
                        != Some(&page)
            })
        {
            return Err("prefill copy requires each exact prepared writable slot".into());
        }
        let natural = order.iter().copied().eq(0..16);
        let rotated = order.iter().copied().eq(std::iter::once(15).chain(0..15));
        if !natural && !rotated {
            return Ok(None);
        }
        let byte_offset = usize::try_from(page)
            .map_err(|_| "prefill page conversion")?
            .checked_mul(ELEMENTS * 2)
            .ok_or("prefill page offset overflow")?;
        let end = byte_offset
            .checked_add(ELEMENTS * 2)
            .ok_or("prefill page end overflow")?;
        let total = usize::try_from(pages)
            .map_err(|_| "prefill pool conversion")?
            .checked_mul(ELEMENTS * 2)
            .ok_or("prefill pool extent overflow")?;
        if end > total {
            return Err("prefill page exceeds the pool".into());
        }
        Ok(Some(Self {
            first,
            page,
            pages,
            byte_offset,
            rotation: u32::from(rotated),
        }))
    }

    pub(super) fn command(self, rank: &Rank, layer: usize) -> EngineeringTpDispatchV1 {
        let buffer = |tensor: Tensor, offset, access| EngineeringTpArgumentV1::Buffer {
            id: tensor.id,
            offset,
            elements: ELEMENTS,
            element_bytes: 2,
            access,
        };
        dispatch(
            ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0],
            256,
            vec![
                buffer(rank.k_rotated, 0, Read),
                buffer(rank.v, 0, Read),
                buffer(rank.layers[layer].k_cache, self.byte_offset, Write),
                buffer(rank.layers[layer].v_cache, self.byte_offset, Write),
                EngineeringTpArgumentV1::U32(self.first),
                EngineeringTpArgumentV1::U32(self.page),
                EngineeringTpArgumentV1::U32(self.pages),
                EngineeringTpArgumentV1::U32(self.rotation),
            ],
        )
    }
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    image: PrefillKvCopyBindingV27,
) -> TpResult<()> {
    if transports.len() != 1 || transports[0].peer_group_rank().is_some() {
        return Err("prefill copy requires one nonpeer TP1 transport".into());
    }
    transports[0].require_loaded_image(image.hsaco, &ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27)
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits both separate images before allocating the same V21 workspace in every arm.
    /// # Errors
    /// Rejects either image or transport before allocation, closing owned transports once.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_prefill_decode_v28(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        let admitted = (|| {
            let copy = copy
                .prefill_kv_copy_binding_v27()
                .ok_or("composition requires the exact V27 image")?;
            let split = split
                .split_attention_binding_v21()
                .ok_or("composition requires the exact V21 image")?;
            admit(&mut transports, copy)?;
            super::c1_split_attention_v25::admit(&mut transports, split)?;
            Ok::<_, String>((copy, split))
        })();
        let (copy, split) = match admitted {
            Ok(images) => images,
            Err(error) => {
                for transport in &mut transports {
                    let _ = transport.close();
                }
                return Err(error);
            }
        };
        let mut driver = Self::new_wide32_with_wave_target_v17(
            transports, model, weights, layout, pool, artifacts,
        )?;
        driver.admitted_prefill_kv_copy_v27 = Some(copy);
        driver.allocate_split_workspace_v25(split)?;
        Ok(driver)
    }

    /// Selects all three independent policies atomically, including explicit baseline modes.
    /// # Errors
    /// Rejects wrong images, nonfresh storage, capture, diagnostics or incompatible execution.
    pub fn configure_ordered_prefill_decode_v28(
        &mut self,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        prefill_enabled: bool,
        split_enabled: bool,
        packed: bool,
    ) -> TpResult<()> {
        let copy = copy
            .prefill_kv_copy_binding_v27()
            .ok_or("composition requires the exact V27 image")?;
        let split = split
            .split_attention_binding_v21()
            .ok_or("composition requires the exact V21 image")?;
        let ((argmax, attention), rmsnorm) = artifacts
            .argmax
            .fp32_argmax_binding_v11()
            .zip(artifacts.attention.query_hoist_binding_v14())
            .zip(artifacts.rmsnorm.wave_rmsnorm_binding_v15())
            .ok_or("composition requires all historical V17 images")?;
        self.configure_prefill_decode_bindings_v28(
            argmax,
            attention,
            rmsnorm,
            copy,
            split,
            prefill_enabled,
            split_enabled,
            packed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn configure_prefill_decode_bindings_v28(
        &mut self,
        argmax: crate::tp_artifact::Fp32ArgmaxBindingV11,
        attention: crate::tp_artifact::QueryHoistBindingV14,
        rmsnorm: crate::tp_artifact::WaveRmsNormBindingV15,
        copy: PrefillKvCopyBindingV27,
        split: crate::tp_artifact::SplitAttentionBindingV21,
        prefill_enabled: bool,
        split_enabled: bool,
        packed: bool,
    ) -> TpResult<()> {
        self.validate_prefill_storage_v28(copy, true)?;
        self.validate_split_storage_v25(split, true)?;
        self.validate_c1_packet_packing_idle()?;
        // V17 checks freshness, admitted bindings and the complete ordered execution
        // contract before mutation. No fallible operation follows that selection.
        self.configure_ordered_c1_wave_target_bindings_v17(
            argmax,
            attention,
            rmsnorm,
            EngineeringTpWaveTargetModeV17::Combined,
        )?;
        self.prefill_kv_copy_v28 = Some(prefill_enabled);
        self.c1_split_attention_v25 = Some(split_enabled);
        self.c1_packet_packing_v22 = Some(packed);
        Ok(())
    }

    /// Admits the separate V27 image before allocation; historical V17 images remain required.
    /// # Errors
    /// Rejects image/transport/geometry drift or allocation failure and closes admitted transports.
    pub fn new_wide32_with_prefill_kv_copy_v28(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        let admitted = copy
            .prefill_kv_copy_binding_v27()
            .ok_or_else(|| String::from("prefill copy requires its exact separate V27 image"))
            .and_then(|image| admit(&mut transports, image).map(|()| image));
        let binding = match admitted {
            Ok(image) => image,
            Err(error) => {
                for transport in &mut transports {
                    let _ = transport.close();
                }
                return Err(error);
            }
        };
        let mut driver = Self::new_wide32_with_wave_target_v17(
            transports, model, weights, layout, pool, artifacts,
        )?;
        driver.admitted_prefill_kv_copy_v27 = Some(binding);
        Ok(driver)
    }

    /// Selects a separate V28 prefill-copy control/candidate atop combined V17.
    /// # Errors
    /// Rejects unadmitted images, incompatible buffers, other candidates or repeated/late selection.
    pub fn configure_ordered_prefill_kv_copy_v28(
        &mut self,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
        enabled: bool,
    ) -> TpResult<()> {
        let binding = copy
            .prefill_kv_copy_binding_v27()
            .ok_or("prefill copy requires its exact V27 image")?;
        let ((argmax, attention), rmsnorm) = artifacts
            .argmax
            .fp32_argmax_binding_v11()
            .zip(artifacts.attention.query_hoist_binding_v14())
            .zip(artifacts.rmsnorm.wave_rmsnorm_binding_v15())
            .ok_or("prefill copy requires exact historical V17 images")?;
        self.configure_prefill_copy_bindings_v28(argmax, attention, rmsnorm, binding, enabled)
    }

    pub(super) fn configure_prefill_copy_bindings_v28(
        &mut self,
        argmax: crate::tp_artifact::Fp32ArgmaxBindingV11,
        attention: crate::tp_artifact::QueryHoistBindingV14,
        rmsnorm: crate::tp_artifact::WaveRmsNormBindingV15,
        copy: PrefillKvCopyBindingV27,
        enabled: bool,
    ) -> TpResult<()> {
        self.validate_prefill_storage_v28(copy, false)?;
        self.configure_ordered_c1_wave_target_bindings_v17(
            argmax,
            attention,
            rmsnorm,
            EngineeringTpWaveTargetModeV17::Combined,
        )?;
        self.prefill_kv_copy_v28 = Some(enabled);
        Ok(())
    }

    fn validate_prefill_storage_v28(
        &self,
        binding: PrefillKvCopyBindingV27,
        split_admitted: bool,
    ) -> TpResult<()> {
        self.validate_prefill_storage_with_c1_v1(binding, split_admitted, None)
    }

    pub(super) fn validate_prefill_storage_with_c1_v1(
        &self,
        binding: PrefillKvCopyBindingV27,
        split_admitted: bool,
        c1: Option<crate::tp_artifact::C1KvCopyBindingV19>,
    ) -> TpResult<()> {
        if self.admitted_prefill_kv_copy_v27 != Some(binding)
            || self.prefill_kv_copy_v28.is_some()
            || self.c1_split_attention_v25.is_some()
            || (!split_admitted && self.split_attention_workspace_v25.is_some())
            || self.c1_kv_copy_v19.is_some()
            || self.admitted_c1_kv_copy_v19 != c1
            || self.c1_packet_packing_v22.is_some()
            || self.inner.plan.world_size() != 1
            || self.inner.ranks.len() != 1
            || self.row_capacity != 32
            || self.inner.large_kv
            || !(1..=512).contains(&self.physical_pages)
            || !(1..=8192).contains(&self.context_tokens)
        {
            return Err(
                "prefill copy requires fresh TP1/32-row legacy storage and no other candidate"
                    .into(),
            );
        }
        let rank = &self.inner.ranks[0];
        let total = self.physical_pages as usize * ELEMENTS;
        if rank.geometry.kv_channels.count != 1024
            || [rank.k_rotated, rank.v]
                .iter()
                .any(|tensor| tensor.element_bytes != 2 || tensor.elements != 32 * 1024)
            || rank.layers.iter().any(|layer| {
                [layer.k_cache, layer.v_cache]
                    .iter()
                    .any(|tensor| tensor.element_bytes != 2 || tensor.elements != total)
            })
        {
            return Err("prefill copy buffer extents differ from fixed TP1 geometry".into());
        }
        Ok(())
    }

    pub(super) fn prepare_prefill_copy_v28(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        order: &[usize],
    ) -> TpResult<Option<PrefillPageV28>> {
        if self.prefill_kv_copy_v28 != Some(true) {
            return Ok(None);
        }
        // execute_selected already validated pool/scope/freshness and prepared writable-page ownership.
        PrefillPageV28::select(batch, order, self.physical_pages, self.context_tokens)
    }

    /// Selected prefill policy; unsupported valid batches retain the V5 append root.
    #[must_use]
    pub const fn prefill_kv_mode(&self) -> &'static str {
        if matches!(self.prefill_kv_copy_v28, Some(true)) {
            "parallel-prefill16-v27"
        } else {
            "baseline"
        }
    }
}
