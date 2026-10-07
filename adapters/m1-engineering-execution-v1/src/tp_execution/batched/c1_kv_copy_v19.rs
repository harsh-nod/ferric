//! Fixed TP1/C1 copy selection and checked subviews of the prepared writable slot.

use super::{
    EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2, EngineeringTpDispatchV1,
    EngineeringTpRankTransportV1, EngineeringTpWaveTargetArtifactsV17,
    EngineeringTpWaveTargetModeV17, PAGE_TOKENS, Rank, Tensor, TpResult, dispatch,
};
use crate::tp_artifact::{
    C1KvCopyBindingV19, ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19, EngineeringTpArtifactV1,
};
use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};
use crate::tp_paged::{
    EngineeringTpPagedPoolV1, EngineeringTpPreparedBatchV1, EngineeringTpPreparedRowV1,
};
use ferric_build::AuthenticatedModelWeightLayout;
use ferric_spec::ModelConfig;

const COLUMNS: usize = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CopySlotV19 {
    position: u32,
    page: u32,
    pages: u32,
    byte_offset: usize,
}

impl CopySlotV19 {
    pub(super) fn new(
        row: &EngineeringTpPreparedRowV1,
        pages: u32,
        context: u32,
    ) -> TpResult<Self> {
        let position = row.position();
        let page = row.writable_physical_page();
        if !(1..=512).contains(&pages)
            || !(1..=8192).contains(&context)
            || position >= context
            || page >= pages
            || row.writable_token_offset() != position % PAGE_TOKENS
            || row.physical_pages().get((position / PAGE_TOKENS) as usize) != Some(&page)
        {
            return Err("C1 copy requires the prepared row's exact bounded writable slot".into());
        }
        let slot = usize::try_from(page)
            .map_err(|_| "copy page conversion")?
            .checked_mul(PAGE_TOKENS as usize)
            .and_then(|base| base.checked_add((position % PAGE_TOKENS) as usize))
            .ok_or("copy slot overflow")?;
        let byte_offset = slot
            .checked_mul(COLUMNS)
            .and_then(|offset| offset.checked_mul(2))
            .ok_or("copy byte offset overflow")?;
        let end = byte_offset
            .checked_add(COLUMNS * 2)
            .ok_or("copy end overflow")?;
        let total = usize::try_from(pages)
            .map_err(|_| "copy pages conversion")?
            .checked_mul(PAGE_TOKENS as usize * COLUMNS * 2)
            .ok_or("copy pool extent overflow")?;
        if end > total {
            return Err("copy slot exceeds the physical pool".into());
        }
        Ok(Self {
            position,
            page,
            pages,
            byte_offset,
        })
    }

    pub(super) fn command(self, rank: &Rank, layer: usize) -> EngineeringTpDispatchV1 {
        let buffer = |tensor: Tensor, offset, access| EngineeringTpArgumentV1::Buffer {
            id: tensor.id,
            offset,
            elements: COLUMNS,
            element_bytes: 2,
            access,
        };
        dispatch(
            ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0],
            16,
            vec![
                buffer(rank.k_rotated, 0, Read),
                buffer(rank.v, 0, Read),
                buffer(rank.layers[layer].k_cache, self.byte_offset, Write),
                buffer(rank.layers[layer].v_cache, self.byte_offset, Write),
                EngineeringTpArgumentV1::U32(self.position),
                EngineeringTpArgumentV1::U32(self.page),
                EngineeringTpArgumentV1::U32(self.pages),
            ],
        )
    }
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    image: C1KvCopyBindingV19,
) -> TpResult<()> {
    if transports.len() != 1 || transports[0].peer_group_rank().is_some() {
        return Err("C1 copy requires one nonpeer TP1 transport".into());
    }
    transports[0].require_loaded_image(image.hsaco, &ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19)
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits V19 before any allocation; all historical V17 images remain separately required.
    /// # Errors
    /// Rejects missing/wrong images, unsupported transport or V17 geometry, or allocation failure.
    pub fn new_wide32_with_c1_kv_copy_v19(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        let admitted = copy
            .c1_kv_copy_binding_v19()
            .ok_or_else(|| String::from("C1 copy requires its exact separate V19 image"))
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
        driver.admitted_c1_kv_copy_v19 = Some(binding);
        Ok(driver)
    }

    /// Selects a V19 control/candidate atop combined V17; multi-row append stays unchanged.
    /// # Errors
    /// Rejects unadmitted images, incompatible buffers, or repeated/late profile selection.
    pub fn configure_ordered_c1_kv_copy_v19(
        &mut self,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        copy: &EngineeringTpArtifactV1,
        enabled: bool,
    ) -> TpResult<()> {
        let binding = copy
            .c1_kv_copy_binding_v19()
            .ok_or("C1 copy requires its exact separate V19 image")?;
        let ((argmax, attention), rmsnorm) = artifacts
            .argmax
            .fp32_argmax_binding_v11()
            .zip(artifacts.attention.query_hoist_binding_v14())
            .zip(artifacts.rmsnorm.wave_rmsnorm_binding_v15())
            .ok_or("C1 copy requires the exact historical V17 images")?;
        self.configure_c1_copy_bindings_v19(argmax, attention, rmsnorm, binding, enabled)
    }

    pub(super) fn configure_c1_copy_bindings_v19(
        &mut self,
        argmax: crate::tp_artifact::Fp32ArgmaxBindingV11,
        attention: crate::tp_artifact::QueryHoistBindingV14,
        rmsnorm: crate::tp_artifact::WaveRmsNormBindingV15,
        copy: C1KvCopyBindingV19,
        enabled: bool,
    ) -> TpResult<()> {
        self.validate_copy_storage_v19(copy)?;
        self.configure_ordered_c1_wave_target_bindings_v17(
            argmax,
            attention,
            rmsnorm,
            EngineeringTpWaveTargetModeV17::Combined,
        )?;
        self.c1_kv_copy_v19 = enabled.then_some(copy);
        Ok(())
    }

    pub(super) fn validate_copy_storage_v19(&self, binding: C1KvCopyBindingV19) -> TpResult<()> {
        if self.admitted_c1_kv_copy_v19 != Some(binding)
            || self.c1_kv_copy_v19.is_some()
            || self.inner.plan.world_size() != 1
            || self.inner.ranks.len() != 1
            || self.row_capacity != 32
            || self.inner.large_kv
            || !(1..=512).contains(&self.physical_pages)
            || !(1..=8192).contains(&self.context_tokens)
        {
            return Err(
                "C1 copy requires preallocated TP1/32-row legacy storage and its exact image"
                    .into(),
            );
        }
        let rank = &self.inner.ranks[0];
        let pool_elements = self.physical_pages as usize * PAGE_TOKENS as usize * COLUMNS;
        if rank.geometry.kv_channels.count
            != u32::try_from(COLUMNS).map_err(|_| "copy width conversion")?
            || [rank.k_rotated, rank.v].iter().any(|tensor| {
                tensor.element_bytes != 2 || tensor.elements != self.row_capacity * COLUMNS
            })
            || rank.layers.iter().any(|layer| {
                [layer.k_cache, layer.v_cache]
                    .iter()
                    .any(|tensor| tensor.element_bytes != 2 || tensor.elements != pool_elements)
            })
        {
            return Err("C1 copy buffer extents differ from the fixed TP1 geometry".into());
        }
        Ok(())
    }

    pub(super) fn prepare_c1_copy_v19(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
    ) -> TpResult<Option<CopySlotV19>> {
        if self.c1_kv_copy_v19.is_none() || batch.rows().len() != 1 {
            return Ok(None);
        }
        // execute_selected has already checked pool/scope/freshness and COW write authority.
        CopySlotV19::new(&batch.rows()[0], self.physical_pages, self.context_tokens).map(Some)
    }

    /// Actual single-row append route; multi-row append always uses the resident V5 root.
    #[must_use]
    pub const fn kv_append_mode(&self) -> &'static str {
        if self.c1_kv_copy_v19.is_some() {
            "parallel-c1-v19"
        } else {
            "baseline"
        }
    }
}
