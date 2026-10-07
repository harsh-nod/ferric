//! Private two-forward GPU bring-up with bounded numerical observations.

#[path = "native_prefix_tiles_decode_v6.rs"]
pub(crate) mod prefix_tiles_decode_v6;
#[path = "native_prefix_tiles_layer_v6.rs"]
pub(crate) mod prefix_tiles_layer_v6;
#[path = "native_tiles_decode_v1.rs"]
pub(crate) mod tiles_decode_v1;

use super::{BindingKey, NativeOwner, Phase, Result, source_key, wire};
use crate::forward_sequence::{self, ForwardCompletion, ForwardInput, InputMode, Sequence};
use crate::resident_layer::capture_v1::{Collector, Layer0CaptureV1};
use crate::resident_layer::{self, LayerCompletion};
use crate::state_roster::state_reuse_v1::ReusableStateRosterV1;
use fe2o3_kfd::Gfx950EngineeringPeerBufferV1 as Buffer;

pub(crate) struct ForwardRun {
    pub(crate) completion: ForwardCompletion,
    /// Each row was read only after both ranks completed the final residual.
    pub(crate) layer_hidden: Vec<Vec<u8>>,
    pub(crate) final_normalized: Vec<u8>,
    pub(crate) logits: Vec<u8>,
}

pub(crate) struct ForwardOwner {
    owner: NativeOwner,
    sequence: Sequence,
    timeout_ms: u32,
}

impl ForwardOwner {
    /// # Safety
    /// The caller must bind the authenticated parent model/recipe manifest and
    /// reviewed source numerical/runtime premises to this exclusively retained
    /// Group. This finite engineering path grants no production admission.
    pub(crate) unsafe fn from_sealed(
        owner: NativeOwner,
        mode: InputMode,
        timeout_ms: u32,
    ) -> Result<Self> {
        validate_sealed(&owner, timeout_ms)?;
        let sequence = Sequence::new(owner.registration_sha256(), mode)?;
        Ok(Self {
            owner,
            sequence,
            timeout_ms,
        })
    }

    pub(crate) fn run(&mut self, input: &ForwardInput) -> Result<ForwardRun> {
        self.run_inner(input, false).map(|(run, _)| run)
    }

    /// Separate opt-in diagnostic. Default run and all existing IPC stay intact.
    /// Capture is returned only after the entire existing forward succeeds; the
    /// caller must retain the usual healthy Close gate before final publication.
    pub(crate) fn run_with_layer0_capture(
        &mut self,
        input: &ForwardInput,
    ) -> Result<(ForwardRun, Layer0CaptureV1)> {
        let (run, capture) = self.run_inner(input, true)?;
        match capture {
            Some(capture) => Ok((run, capture)),
            None => {
                self.owner.catalog.phase = Phase::Terminal;
                if let Some(states) = self.owner.sealed_states.as_mut() {
                    states.poison();
                }
                Err("requested layer-zero capture missing".into())
            }
        }
    }

    fn run_inner(
        &mut self,
        input: &ForwardInput,
        capture_enabled: bool,
    ) -> Result<(ForwardRun, Option<Layer0CaptureV1>)> {
        let capture = if capture_enabled {
            match Collector::new(input.generation, input.cache_metadata[0], 0) {
                Ok(capture) => Some(capture),
                Err(error) => {
                    self.owner.catalog.phase = Phase::Terminal;
                    if let Some(states) = self.owner.sealed_states.as_mut() {
                        states.poison();
                    }
                    return Err(error);
                }
            }
        } else {
            None
        };
        let mut active = Active {
            owner: &mut self.owner,
            timeout_ms: self.timeout_ms,
            layer_hidden: Vec::with_capacity(36),
            final_normalized: Vec::new(),
            logits: Vec::new(),
            capture,
            captured_layer0: None,
            reuse: None,
        };
        let completion = self.sequence.run(&mut active, input)?;
        Ok((
            ForwardRun {
                completion,
                layer_hidden: active.layer_hidden,
                final_normalized: active.final_normalized,
                logits: active.logits,
            },
            active.captured_layer0,
        ))
    }

    pub(crate) fn close(mut self) -> Result<()> {
        self.owner.close_setup()
    }
}

struct Active<'a> {
    owner: &'a mut NativeOwner,
    timeout_ms: u32,
    layer_hidden: Vec<Vec<u8>>,
    final_normalized: Vec<u8>,
    logits: Vec<u8>,
    capture: Option<Collector>,
    captured_layer0: Option<Layer0CaptureV1>,
    reuse: Option<(&'a mut ReusableStateRosterV1, u64)>,
}

fn validate_sealed(owner: &NativeOwner, timeout_ms: u32) -> Result<()> {
    if owner.catalog.phase != Phase::LayersSealed
        || owner.layer_bindings.len() != 36
        || owner.artifacts.is_none()
        || owner.sealed_states.is_none()
        || owner.tail_manifest.is_none()
        || owner.tail_head.is_none()
        || owner.tail_artifacts.is_none()
        || owner.tail_bindings.is_none()
        || !(1..=10_000).contains(&timeout_ms)
    {
        return Err("finite forward requires fully sealed layers and tail".into());
    }
    Ok(())
}

fn finite_bf16(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty()
        || bytes.len() % 2 != 0
        || bytes
            .chunks_exact(2)
            .any(|word| u16::from_le_bytes([word[0], word[1]]) & 0x7f80 == 0x7f80)
    {
        Err("nonfinite or malformed BF16 observation".into())
    } else {
        Ok(())
    }
}

fn checked_argmax(bytes: &[u8], token: u32) -> Result<()> {
    if bytes.len() != 151_936 * 2 || token >= 151_936 {
        return Err("finite logits or token extent".into());
    }
    finite_bf16(bytes)?;
    let mut best = f32::NEG_INFINITY;
    let mut winner = 0;
    for (index, word) in bytes.chunks_exact(2).enumerate() {
        let value = f32::from_bits(u32::from(u16::from_le_bytes([word[0], word[1]])) << 16);
        if value > best {
            best = value;
            winner = index as u32;
        }
    }
    if winner != token {
        return Err("device argmax differs from retained BF16 logits".into());
    }
    Ok(())
}

fn checked_layer_hidden_pair(layer: usize, result: Result<[Vec<u8>; 2]>) -> Result<Vec<u8>> {
    let [hidden0, hidden1] = result?;
    if hidden0.len() != 8192 || hidden0 != hidden1 {
        return Err(format!("finite layer {layer} rank residual outputs differ"));
    }
    finite_bf16(&hidden0).map_err(|error| format!("finite layer {layer}: {error}"))?;
    Ok(hidden0)
}

fn scalar_layer_hidden_pair(mut read_rank: impl FnMut(usize) -> Result<Vec<u8>>) -> Result<[Vec<u8>; 2]> {
    Ok([read_rank(0)?, read_rank(1)?])
}

impl Active<'_> {
    fn retain_tail_result(&mut self, result: (u32, [u64; 3])) -> Result<(u32, [u64; 3])> {
        let source = self
            .owner
            .catalog
            .source
            .scratch
            .iter()
            .find(|row| row.buffer.rank == 0 && row.kind == wire::ScratchKind::Normalized)
            .ok_or("finite normalization root missing")?;
        let normalized = self.buffer(source_key(source.buffer))?;
        let logits = self.auxiliary(wire::AuxiliaryKind::Logits)?;
        self.final_normalized = self.owner.catalog.backend.read(normalized, 0, 8192)?;
        self.logits = self.owner.catalog.backend.read(logits, 0, 151_936 * 2)?;
        if self.final_normalized.len() != 8192 {
            return Err("final normalization extent".into());
        }
        finite_bf16(&self.final_normalized)?;
        checked_argmax(&self.logits, result.0)?;
        Ok(result)
    }
    fn retain_layer_hidden(&mut self, layer: usize, hidden: [Buffer; 2]) -> Result<()> {
        let observed = scalar_layer_hidden_pair(|rank| self.owner.catalog.backend.read(hidden[rank], 0, 8192));
        self.layer_hidden.push(checked_layer_hidden_pair(layer, observed)?);
        Ok(())
    }

    fn retain_layer_hidden_pair(&mut self, layer: usize, hidden: [Buffer; 2]) -> Result<()> {
        let observed = self.owner.catalog.backend.read_pair_v1(hidden, 0, 8192);
        self.layer_hidden.push(checked_layer_hidden_pair(layer, observed)?);
        Ok(())
    }

    fn buffer(&self, key: BindingKey) -> Result<Buffer> {
        self.owner
            .catalog
            .records
            .iter()
            .find(|record| record.facts.key == key)
            .filter(|record| !record.facts.immutable && record.upload.is_none())
            .map(|record| record.token)
            .ok_or_else(|| "finite mutable role missing".into())
    }

    fn auxiliary(&self, kind: wire::AuxiliaryKind) -> Result<Buffer> {
        let source = self
            .owner
            .catalog
            .source
            .auxiliary
            .iter()
            .find(|row| row.buffer.rank == 0 && row.kind == kind)
            .ok_or("finite rank-zero auxiliary role missing")?;
        self.buffer(source_key(source.buffer))
    }
}

impl forward_sequence::Backend for Active<'_> {
    fn upload_metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.upload_metadata_for(input, AllocationProfile::BaseV1)
    }

    fn embedding(&mut self, token: u32) -> Result<[u64; 2]> {
        // SAFETY: construction fixes the authenticated owner; Sequence has
        // admitted this exact token and immutable generation/position order.
        unsafe { self.owner.tail_embedding(token, self.timeout_ms) }
    }

    fn begin_states(&mut self, input: &ForwardInput) -> Result<()> {
        let registration = self.owner.registration_sha256();
        let model = self.owner.catalog.scope.model_id;
        if let Some((reuse, generation)) = self.reuse.as_mut() {
            if *generation != input.generation {
                return Err("long active generation mismatch".into());
            }
            return reuse.begin_forward(
                &mut self.owner.catalog.backend,
                registration,
                model,
                input.generation,
                input.cache_metadata[0],
            );
        }
        self.owner
            .sealed_states
            .as_mut()
            .ok_or("finite state roster missing")?
            .begin_forward(
                registration,
                model,
                input.generation,
                input.cache_metadata[0],
            )
    }

    fn layer(&mut self, layer: usize) -> Result<LayerCompletion> {
        let roots = self
            .owner
            .layer_bindings
            .get(layer)
            .ok_or("finite layer bounds")?;
        let artifacts = self
            .owner
            .artifacts
            .as_ref()
            .ok_or("finite artifacts missing")?;
        let states = if let Some((reuse, generation)) = self.reuse.as_mut() {
            reuse.active_states(*generation)?
        } else {
            self.owner
                .sealed_states
                .as_mut()
                .ok_or("finite state roster missing")?
        };
        // SAFETY: exact model roots/images are sealed; embedding and metadata
        // completed, typed states advance once, and this owner is exclusive.
        let result = unsafe {
            if layer == 0 && self.capture.is_some() {
                resident_layer::execute_layer_with_capture(
                    &mut self.owner.catalog.backend,
                    states,
                    artifacts,
                    layer,
                    roots,
                    self.timeout_ms,
                    self.capture
                        .as_mut()
                        .ok_or("missing active diagnostic collector")?,
                )
            } else {
                resident_layer::execute_layer(
                    &mut self.owner.catalog.backend,
                    states,
                    artifacts,
                    layer,
                    roots,
                    self.timeout_ms,
                )
            }
        }?;
        if layer == 0 {
            if let Some(capture) = self.capture.take() {
                self.captured_layer0 = Some(capture.finish()?);
            }
        }
        self.retain_layer_hidden(layer, roots.final_hidden)?;
        Ok(result)
    }

    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        // SAFETY: the roster has completed all36 layers and both final residual
        // queues; the separate completed transpose and exact tail are retained.
        let result = unsafe { self.owner.tail_finish(self.timeout_ms) }?;
        self.retain_tail_result(result)
    }

    fn idle_fence(&mut self) -> Result<()> {
        self.idle_fence_for(AllocationProfile::BaseV1)
    }

    fn commit_states(&mut self) -> Result<()> {
        if let Some((reuse, generation)) = self.reuse.as_mut() {
            return reuse.commit_forward(&mut self.owner.catalog.backend, *generation, Ok(()));
        }
        self.owner
            .sealed_states
            .as_mut()
            .ok_or("finite state roster missing")?
            .commit_forward(Ok(()))
    }

    fn poison(&mut self) {
        self.owner.catalog.phase = Phase::Terminal;
        if let Some((reuse, _)) = self.reuse.as_mut() {
            reuse.poison();
        }
        if let Some(states) = self.owner.sealed_states.as_mut() {
            states.poison();
        }
    }
}

#[cfg(test)]
#[path = "native_forward_tests.rs"]
mod tests;

#[path = "native_long_forward_v1.rs"]
pub(crate) mod long_v1;

#[path = "native_rearm_smoke_v1.rs"]
pub(crate) mod rearm_smoke_v1;

#[path = "native_queued_mlp_comparison_v1.rs"]
pub(crate) mod queued_mlp_comparison_v1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AllocationProfile {
    BaseV1,
    TilesComparisonV1,
    TilesDecodeV1,
    PrefixTilesLayerV6,
    PrefixTilesDecodeV6,
    ProjectionResidualMlpOrderedV1,
    GuardedMlpDecodeV1([usize; 2]),
}
impl AllocationProfile {
    fn counts(self) -> [usize; 2] {
        match self {
            Self::BaseV1 => [714, 710],
            Self::TilesDecodeV1 => [714, 710],
            Self::PrefixTilesLayerV6 => [714, 710],
            Self::PrefixTilesDecodeV6 => [714, 710],
            Self::ProjectionResidualMlpOrderedV1 => [715, 711],
            Self::TilesComparisonV1 => [715, 711],
            Self::GuardedMlpDecodeV1(counts) => counts,
        }
    }
    fn validate(self, actual: &[usize]) -> Result<()> {
        if actual != self.counts() {
            return Err("finite retained allocation census changed".into());
        }
        Ok(())
    }
}

#[path = "native_guarded_mlp_decode_v1.rs"]
pub(crate) mod guarded_mlp_decode_v1;
impl Active<'_> {
    fn upload_metadata_for(
        &mut self,
        input: &ForwardInput,
        profile: AllocationProfile,
    ) -> Result<()> {
        if self.owner.catalog.phase != Phase::LayersSealed {
            return Err("finite owner is not executable".into());
        }
        let mut roots = Vec::with_capacity(4);
        for rank in 0..2 {
            for kind in [wire::PendingKind::CacheMetadata, wire::PendingKind::Rotary] {
                roots.push(self.buffer(BindingKey::Pending {
                    rank,
                    layer: None,
                    kind,
                })?);
            }
        }
        // Resolve every role before writes; no caller-supplied native IDs exist.
        let metadata = input
            .cache_metadata
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>();
        let rotary = input
            .rotary_bits
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>();
        write_metadata(
            &mut self.owner.catalog.backend,
            profile,
            roots.try_into().map_err(|_| "finite metadata role count")?,
            &metadata,
            &rotary,
        )
    }

    fn idle_fence_for(&mut self, profile: AllocationProfile) -> Result<()> {
        profile.validate(
            &self
                .owner
                .catalog
                .backend
                .preflight_additional_allocations_v1(&[0, 0])?,
        )
    }
}
#[path = "native_mlp_tiles_comparison_v1.rs"]
pub(crate) mod mlp_tiles_comparison_v1;

trait MetadataWriter {
    type Root: Copy;
    fn counts(&mut self) -> Result<Vec<usize>>;
    fn write_metadata_root(&mut self, root: Self::Root, bytes: &[u8]) -> Result<()>;
}
impl MetadataWriter for super::Group {
    type Root = Buffer;
    fn counts(&mut self) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(&[0, 0])
    }
    fn write_metadata_root(&mut self, root: Buffer, bytes: &[u8]) -> Result<()> {
        self.write(root, 0, bytes)
    }
}
fn write_metadata<W: MetadataWriter>(
    writer: &mut W,
    profile: AllocationProfile,
    roots: [W::Root; 4],
    metadata: &[u8],
    rotary: &[u8],
) -> Result<()> {
    profile.validate(&writer.counts()?)?;
    for rank in 0..2 {
        writer.write_metadata_root(roots[rank * 2], metadata)?;
        writer.write_metadata_root(roots[rank * 2 + 1], rotary)?;
    }
    Ok(())
}

#[cfg(test)]
mod ordered_allocation_tests {
    use super::*;
    struct Writer {
        counts: Vec<usize>,
        writes: usize,
    }
    impl MetadataWriter for Writer {
        type Root = u8;
        fn counts(&mut self) -> Result<Vec<usize>> {
            Ok(self.counts.clone())
        }
        fn write_metadata_root(&mut self, _: u8, _: &[u8]) -> Result<()> {
            self.writes += 1;
            Ok(())
        }
    }
    #[test]
    fn ordered_allocation_profile_refuses_old_census_before_any_metadata_write() {
        let mut w = Writer {
            counts: vec![714, 710],
            writes: 0,
        };
        assert!(
            write_metadata(
                &mut w,
                AllocationProfile::ProjectionResidualMlpOrderedV1,
                [0, 1, 2, 3],
                &[],
                &[]
            )
            .is_err()
        );
        assert_eq!(w.writes, 0);
        w.counts = vec![715, 711];
        write_metadata(
            &mut w,
            AllocationProfile::ProjectionResidualMlpOrderedV1,
            [0, 1, 2, 3],
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(w.writes, 4);
        assert!(
            AllocationProfile::PrefixTilesDecodeV6
                .validate(&w.counts)
                .is_err()
        );
    }
}
