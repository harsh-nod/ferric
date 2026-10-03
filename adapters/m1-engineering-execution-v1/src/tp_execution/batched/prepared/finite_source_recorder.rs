//! Authenticated CPU source recording. No native allocation or execution capability.

use super::finite_composition::EngineeringTp2FiniteCompositionV1;
use super::finite_packing::{
    EngineeringTp2FiniteUploadKeyV1, EngineeringTp2FiniteUploadV1,
    EngineeringTp2FiniteWeightSourceV1,
};
use super::{EngineeringTpBatchExecutionV2, EngineeringTpRankTransportV1, TpResult};
use crate::tp_execution::projection::ProjectionPolicy;
use crate::tp_execution::{
    EngineeringTpDispatchV1, EngineeringTpProjectionModeV3, EngineeringTpReductionModeV3,
};
use crate::tp_model::EngineeringQwenModelV1;
use crate::tp_paged::EngineeringTpPagedPoolV1;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

const MAX_CHUNK: usize = 4 << 20;
const MAX_ALLOCATIONS: usize = 2048;
const MAX_BUFFER: usize = 151_936 * 4096 * 2;
const TRANSPOSES: usize = 36 * 7 * 2 + 1;

struct Allocation {
    rank: u32,
    bytes: usize,
    peer_readable: bool,
    written: usize,
    digest: Sha256,
}

#[derive(Default)]
struct Catalog {
    allocations: BTreeMap<u64, Allocation>,
    closed: [bool; 2],
    terminal: bool,
}

impl Catalog {
    fn check(&self, rank: u32) -> TpResult<()> {
        if rank >= 2 || self.terminal || self.closed.iter().any(|value| *value) {
            return Err("finite CPU source recorder is terminal".into());
        }
        Ok(())
    }

    fn complete_uploads_since(&self, first: u64, expected: usize) -> TpResult<u64> {
        if self.terminal || self.closed.iter().any(|value| *value) {
            return Err("finite CPU source recorder upload census is terminal".into());
        }
        let mut count = 0_usize;
        let mut total = 0_u64;
        for (_, entry) in self.allocations.range(first..) {
            if entry.written != entry.bytes || entry.peer_readable {
                return Err("finite CPU source transpose upload is incomplete or shared".into());
            }
            count += 1;
            total = total
                .checked_add(entry.bytes as u64)
                .ok_or("finite CPU source transpose byte overflow")?;
        }
        if count != expected {
            return Err("finite CPU source transpose upload cardinality".into());
        }
        Ok(total)
    }

    fn validate_upload(&self, rank: u32, id: u64, bytes: usize, digest: [u8; 32]) -> TpResult<()> {
        self.check(rank)?;
        let entry = self
            .allocations
            .get(&id)
            .ok_or("finite CPU source intake ID missing")?;
        let recorded: [u8; 32] = entry.digest.clone().finalize().into();
        if entry.rank != rank
            || entry.peer_readable
            || entry.bytes != bytes
            || entry.written != entry.bytes
            || recorded != digest
        {
            return Err("finite CPU source packing differs from authenticated intake".into());
        }
        Ok(())
    }
}

/// CPU-only implementation of the existing intake sink. Its logical IDs are
/// source grammar identities, not handles accepted by the native KFD catalog.
struct RecordingRank {
    child_identity: u32,
    rank: u32,
    catalog: Rc<RefCell<Catalog>>,
}

fn recording_ranks(child_identity: u32) -> TpResult<(Rc<RefCell<Catalog>>, Vec<RecordingRank>)> {
    if child_identity == 0 {
        return Err("finite CPU source recorder needs an explicit child identity".into());
    }
    let catalog = Rc::new(RefCell::new(Catalog::default()));
    let ranks = (0..2)
        .map(|rank| RecordingRank {
            child_identity,
            rank,
            catalog: Rc::clone(&catalog),
        })
        .collect();
    Ok((catalog, ranks))
}

impl RecordingRank {
    fn allocate_record(&mut self, bytes: usize, peer_readable: bool) -> TpResult<u64> {
        let mut catalog = self.catalog.borrow_mut();
        let result = (|| {
            catalog.check(self.rank)?;
            if bytes == 0 || bytes > MAX_BUFFER || catalog.allocations.len() >= MAX_ALLOCATIONS {
                return Err("finite CPU source allocation bound".into());
            }
            let id = u64::try_from(catalog.allocations.len())
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or("finite CPU source identity overflow")?;
            if catalog
                .allocations
                .insert(
                    id,
                    Allocation {
                        rank: self.rank,
                        bytes,
                        peer_readable,
                        written: 0,
                        digest: Sha256::new(),
                    },
                )
                .is_some()
            {
                return Err("finite CPU source identity collision".into());
            }
            Ok(id)
        })();
        if result.is_err() {
            catalog.terminal = true;
        }
        result
    }

    fn reject<T>(&self) -> TpResult<T> {
        self.catalog.borrow_mut().terminal = true;
        Err("finite CPU source recorder cannot execute, register, or read native memory".into())
    }
}

impl EngineeringTpRankTransportV1 for RecordingRank {
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        Some((self.child_identity, self.rank, 2))
    }

    fn setup_upload_chunk_bytes(&self) -> usize {
        MAX_CHUNK
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocate_record(bytes, false)
    }
    fn allocate_peer_readable(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocate_record(bytes, true)
    }

    fn write(&mut self, id: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        let mut catalog = self.catalog.borrow_mut();
        let result = (|| {
            catalog.check(self.rank)?;
            if bytes.is_empty() || bytes.len() > MAX_CHUNK {
                return Err("finite CPU source upload chunk bound".into());
            }
            let entry = catalog
                .allocations
                .get_mut(&id)
                .ok_or("finite CPU source upload identity")?;
            let end = offset
                .checked_add(bytes.len())
                .ok_or("finite CPU source upload overflow")?;
            if entry.rank != self.rank || offset != entry.written || end > entry.bytes {
                return Err("finite CPU source upload owner, order, or extent".into());
            }
            entry.digest.update(bytes);
            entry.written = end;
            Ok(())
        })();
        if result.is_err() {
            catalog.terminal = true;
        }
        result
    }

    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        self.reject()
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        self.reject()
    }
    fn wait(&mut self) -> TpResult<()> {
        self.reject()
    }
    fn close(&mut self) -> TpResult<()> {
        self.catalog.borrow_mut().closed[self.rank as usize] = true;
        Ok(())
    }
    // All supports_* defaults remain false. Registration, image admission, and
    // execution defaults reject. No old GraphGroup capability is asserted.
}

/// Owns a private, non-executable intake recorder over an authenticated model.
///
/// No native child is opened here. `child_identity` is a binding selected by the
/// parent; the eventual setup endpoint must independently match its actual PID.
/// Real old-layout transposes are computed and hashed, then discarded. Their
/// source-only IDs never become native allocation or image admission claims.
pub struct EngineeringTp2FiniteSourceRecorderV1<'model> {
    model: &'model EngineeringQwenModelV1,
    driver: EngineeringTpBatchExecutionV2<RecordingRank>,
    catalog: Rc<RefCell<Catalog>>,
}

impl<'model> EngineeringTp2FiniteSourceRecorderV1<'model> {
    /// Record fresh fixed8B/TP2/2304 geometry using existing authenticated intake.
    ///
    /// This performs CPU work over the full model, including real queued MFMA
    /// transposes for the retained source grammar. No tensor payload is retained
    /// in the recording catalog; its largest temporary transpose is the head.
    /// # Errors
    /// Rejects unsupported model/pool, missing child identity, malformed uploads,
    /// incomplete transpose recording, or any source profile mismatch.
    pub fn new(
        model: &'model EngineeringQwenModelV1,
        pool: &EngineeringTpPagedPoolV1,
        child_identity: u32,
    ) -> TpResult<Self> {
        let (catalog, transports) = recording_ranks(child_identity)?;
        let config = model.config();
        if config.role != ferric_spec::Qwen3ModelRole::Target8B
            || config.layers != 36
            || config.hidden_size != 4096
            || config.intermediate_size != 12_288
            || config.query_heads != 32
            || config.kv_heads != 8
            || config.head_dim != 128
            || config.rope_theta != 1_000_000
            || config.tie_word_embeddings
            || config.max_position_embeddings != 40_960
            || pool.scope().model != *config.model_id.as_bytes()
            || pool.scope().session == [0; 32]
            || !pool.is_empty()
            || pool.row_capacity() != 16
            || pool.limits().context_tokens() != 2304
            || pool.limits().physical_page_count() != 144
            || pool.limits().page_table_stride() != 144
        {
            return Err("finite CPU source recorder supports only authentic Qwen3-8B TP2".into());
        }
        let mut driver = EngineeringTpBatchExecutionV2::new(
            transports,
            config,
            model.target_weights(),
            model.layout(),
            pool,
        )?;
        let first_transpose = u64::try_from(catalog.borrow().allocations.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or("finite CPU source transpose identity overflow")?;
        // This is the real authenticated bit transpose, not the test-only
        // synthetic_mfma_ranks_for_recording helper or an admitted GPU mode.
        let projection = ProjectionPolicy::prepare(
            EngineeringTpProjectionModeV3::Mfma,
            &mut driver.inner,
            model.target_weights(),
            model.layout(),
        )?;
        if catalog
            .borrow()
            .complete_uploads_since(first_transpose, TRANSPOSES)?
            != projection.bytes
        {
            return Err("finite CPU source transpose byte census".into());
        }
        driver.projection = projection;
        driver.projection_configured = true;
        driver.inner.check_peer_dependency_source_profile()?;
        // Reuse only source-owned scratch allocation/dataflow. The public
        // executable configure_reduction path retains its capability checks.
        driver
            .inner
            .configure_device_peer(EngineeringTpReductionModeV3::DevicePeerDependencyV1)?;
        driver.prepare_finite_two_forward_composition_v1()?;
        Ok(Self {
            model,
            driver,
            catalog,
        })
    }

    /// Borrow an inert authenticated composition and its exact packing factory.
    /// The callback cannot move a native owner or executable driver out of this
    /// wrapper. Owned manifest/source-program bytes may be returned by it.
    /// # Errors
    /// Rejects changed source ownership/geometry or model-section digest drift.
    pub fn with_plan<T>(
        &self,
        action: impl for<'plan> FnOnce(
            &'plan EngineeringTp2FiniteCompositionV1<'plan>,
            &'plan EngineeringTp2FiniteWeightSourceV1<'plan>,
            &'plan [EngineeringTp2FiniteUploadV1<'plan>],
        ) -> TpResult<T>,
    ) -> TpResult<T> {
        let composition = self.driver.prepare_finite_two_forward_composition_v1()?;
        let source = EngineeringTp2FiniteWeightSourceV1::new(
            self.model.layout(),
            self.model.target_weights(),
            &composition,
        )?;
        let mut uploads = Vec::with_capacity(939);
        for rank in 0..2 {
            for layer in 0..36 {
                uploads.extend(source.layer_uploads(rank, layer)?);
            }
        }
        uploads.extend(source.global_uploads()?);
        validate_intake(&self.catalog.borrow(), &uploads)?;
        action(&composition, &source, &uploads)
    }

    /// CPU-recorded old-layout bytes only, never a native memory claim.
    #[must_use]
    pub const fn recorded_transpose_bytes(&self) -> u64 {
        self.driver.projection.bytes
    }
}

fn validate_intake(
    catalog: &Catalog,
    uploads: &[EngineeringTp2FiniteUploadV1<'_>],
) -> TpResult<()> {
    catalog.check(0)?;
    if uploads.len() != 939 {
        return Err("finite CPU source packing roster cardinality".into());
    }
    let mut originals = 0;
    let mut seen = std::collections::BTreeSet::new();
    for upload in uploads {
        if let EngineeringTp2FiniteUploadKeyV1::Source { rank, id, .. } = upload.key() {
            if !seen.insert((rank, id)) {
                return Err("finite CPU source duplicate original intake".into());
            }
            catalog.validate_upload(rank, id, upload.bytes(), upload.sha256())?;
            originals += 1;
        }
    }
    if originals != 795 {
        return Err("finite CPU source original intake cardinality".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "finite_source_recorder_tests.rs"]
mod tests;
