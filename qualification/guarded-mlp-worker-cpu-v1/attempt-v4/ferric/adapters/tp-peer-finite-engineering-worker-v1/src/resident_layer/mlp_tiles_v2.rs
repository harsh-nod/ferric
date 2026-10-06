//! One-shot V1 then typed548 V2 comparison; not a full-model substitution.

use super::{Backend as LayerBackend, Boundary, LayerBindings, LayerCompletion, Native};
use crate::finite_mlp_tiles_comparison_wire_v1::{Stage, validate_tiles_state};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerWaveMlpTilesDispatchV2 as Dispatch,
    Gfx950EngineeringPeerWaveMlpTilesRoundV2 as Round,
};
use sha2::{Digest, Sha256};

pub(crate) mod artifacts;
pub(crate) use artifacts::{Image, LoadedArtifacts, load};
type Result<T> = std::result::Result<T, String>;

fn index(stage: Stage) -> usize {
    match stage {
        Stage::Norm => 0,
        Stage::Gate => 1,
        Stage::Up => 2,
        Stage::Activation => 3,
        Stage::Down => 4,
    }
}
fn width(stage: Stage) -> usize {
    if stage == Stage::Down { 4 } else { 2 }
}
fn extent(stage: Stage) -> usize {
    [8192, 12288, 12288, 12288, 16384][index(stage)]
}
fn sentinel(stage: Stage) -> Vec<u8> {
    let word: &[u8] = if stage == Stage::Down {
        &[1, 0, 192, 127]
    } else {
        &[193, 127]
    };
    word.repeat(extent(stage) / width(stage))
}
fn finite(stage: Stage, bytes: &[u8]) -> Result<()> {
    if bytes.len() != extent(stage) {
        return Err("V2 comparison output extent".into());
    }
    if bytes.chunks_exact(width(stage)).any(|word| {
        if stage == Stage::Down {
            !f32::from_bits(u32::from_le_bytes(word.try_into().unwrap())).is_finite()
        } else {
            u16::from_le_bytes(word.try_into().unwrap()) & 0x7f80 == 0x7f80
        }
    }) {
        return Err("V2 comparison nonfinite output".into());
    }
    Ok(())
}
fn finite_terminal(states: &[[u32; 11]; 2]) -> Result<()> {
    for words in states {
        if words[0..4] != [1, 0, 31, 31]
            || words[4] >> 10 != 0
            || words[5] != 0
            || words[6..] != [64; 5]
            || (0..5).any(|i| !matches!((words[4] >> (2 * i)) & 3, 1 | 2))
        {
            return Err("V2 comparison requires acquired V1 terminal states".into());
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct Comparison {
    pub(crate) finite_states: [[u32; 11]; 2],
    pub(crate) finite_queue_host_ns: [u64; 2],
    pub(crate) tiles_states: [[u32; 548]; 2],
    pub(crate) tiles_queue_host_ns: [u64; 2],
    pub(crate) equality: [[crate::finite_mlp_tiles_comparison_wire_v1::OutputEquality; 2]; 5],
}

trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn read(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>>;
    fn write(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()>;
    fn dispatch(&mut self) -> Result<Round>;
}

fn coordinate(
    backend: &mut impl Backend,
    finite_states: [[u32; 11]; 2],
    finite_queue_host_ns: [u64; 2],
) -> Result<Comparison> {
    finite_terminal(&finite_states)?;
    backend.validate()?;
    let mut expected = Vec::with_capacity(5);
    // Snapshot all ten outputs before any sentinel write or V2 publication.
    for stage in Stage::ALL {
        let pair = [backend.read(stage, 0)?, backend.read(stage, 1)?];
        for value in &pair {
            finite(stage, value)?;
        }
        expected.push(pair);
    }
    for stage in Stage::ALL {
        for rank in 0..2 {
            backend.write(stage, rank, &sentinel(stage))?;
        }
    }
    let completed = backend.dispatch()?;
    for words in &completed.final_states {
        validate_tiles_state(words).map_err(|e| e.to_string())?;
    }
    let mut equality = Vec::with_capacity(5);
    for stage in Stage::ALL {
        let mut pair = Vec::with_capacity(2);
        for rank in 0..2 {
            let actual = backend.read(stage, rank)?;
            finite(stage, &actual)?;
            if actual != expected[index(stage)][rank] {
                return Err(format!("V2 differs from V1: {stage:?} rank {rank}"));
            }
            pair.push(crate::finite_mlp_tiles_comparison_wire_v1::OutputEquality {
                stage,
                rank: rank as u32,
                bytes: actual.len() as u32,
                words: (actual.len() / width(stage)) as u32,
                sha256: Sha256::digest(&actual).into(),
            });
        }
        equality.push(pair.try_into().map_err(|_| "V2 comparison rank count")?);
    }
    Ok(Comparison {
        finite_states,
        finite_queue_host_ns,
        tiles_states: completed.final_states,
        tiles_queue_host_ns: completed.dispatch_elapsed_ns,
        equality: equality
            .try_into()
            .map_err(|_| "V2 comparison stage count")?,
    })
}

struct Pair<'a> {
    group: &'a mut Group,
    loaded: &'a mut LoadedArtifacts,
    roots: &'a LayerBindings,
    timeout_ms: u32,
}
impl Backend for Pair<'_> {
    fn validate(&mut self) -> Result<()> {
        self.roots.validate()?;
        if !(1..=10_000).contains(&self.timeout_ms) {
            return Err("V2 comparison timeout".into());
        }
        for rank in 0..2 {
            if self.loaded.kernels[rank].rank() != rank
                || self.loaded.states[rank].owner_rank() != rank
            {
                return Err("V2 comparison owner rank".into());
            }
            let initial = self
                .group
                .observe_wave_mlp_tiles_state_v2(&self.loaded.states[rank])?;
            if initial
                .iter()
                .enumerate()
                .any(|(i, &v)| v != u32::from(i == 0 || i == 2))
            {
                return Err("V2 comparison requires fresh typed548 state".into());
            }
        }
        Ok(())
    }
    fn read(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>> {
        self.group.read(
            self.roots.mlp[rank][index(stage) + 5],
            0,
            extent(stage) as u32,
        )
    }
    fn write(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()> {
        self.group
            .write(self.roots.mlp[rank][index(stage) + 5], 0, bytes)
    }
    fn dispatch(&mut self) -> Result<Round> {
        let [left, right] = &mut self.loaded.states;
        // SAFETY: catalog-derived exact semantic roles, same owner, fresh typed
        // states, and the actual reviewed image bound before opening this group.
        unsafe {
            self.group.dispatch_wave_mlp_tiles_round_v2([
                Dispatch {
                    kernel: &self.loaded.kernels[0],
                    object_sha256: self.loaded.sha256,
                    roots: self.roots.mlp[0],
                    state: left,
                    timeout_ms: self.timeout_ms,
                },
                Dispatch {
                    kernel: &self.loaded.kernels[1],
                    object_sha256: self.loaded.sha256,
                    roots: self.roots.mlp[1],
                    state: right,
                    timeout_ms: self.timeout_ms,
                },
            ])
        }
    }
}

trait ComparedLayer: LayerBackend {
    fn compare(&mut self, states: [[u32; 11]; 2], elapsed: [u64; 2]) -> Result<Comparison>;
}
fn coordinate_layer(
    backend: &mut impl ComparedLayer,
    layer: usize,
) -> Result<(LayerCompletion, Comparison)> {
    let result = (|| {
        if layer != 0 {
            return Err("V1/V2 comparison is layer zero only".into());
        }
        backend.validate()?;
        backend.capture(Boundary::BeforePrefix)?;
        let (prefix_states, prefix) = backend.prefix()?;
        backend.capture(Boundary::AfterPrefix)?;
        let first = backend.first_residual()?;
        backend.capture(Boundary::AfterFirstResidual)?;
        let (mlp_states, mlp) = backend.mlp()?;
        backend.capture(Boundary::AfterMlp)?;
        let comparison = backend.compare(mlp_states, mlp)?;
        let last = backend.final_residual()?;
        backend.capture(Boundary::AfterFinalResidual)?;
        Ok((
            LayerCompletion {
                prefix_states,
                mlp_states,
                paired_ns: [prefix, first, mlp, last],
            },
            comparison,
        ))
    })();
    if result.is_err() {
        backend.poison();
    }
    result
}
struct Layer<'a> {
    base: Native<'a>,
    loaded: &'a mut LoadedArtifacts,
}
impl LayerBackend for Layer<'_> {
    fn validate(&mut self) -> Result<()> {
        self.base.validate()
    }
    fn capture(&mut self, boundary: Boundary) -> Result<()> {
        self.base.capture(boundary)
    }
    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])> {
        self.base.prefix()
    }
    fn first_residual(&mut self) -> Result<[u64; 2]> {
        self.base.first_residual()
    }
    fn mlp(&mut self) -> Result<([[u32; 11]; 2], [u64; 2])> {
        self.base.mlp()
    }
    fn final_residual(&mut self) -> Result<[u64; 2]> {
        self.base.final_residual()
    }
    fn poison(&mut self) {
        self.base.poison()
    }
}
impl ComparedLayer for Layer<'_> {
    fn compare(&mut self, states: [[u32; 11]; 2], elapsed: [u64; 2]) -> Result<Comparison> {
        coordinate(
            &mut Pair {
                group: self.base.group,
                loaded: self.loaded,
                roots: self.base.roots,
                timeout_ms: self.base.timeout_ms,
            },
            states,
            elapsed,
        )
    }
}

/// The private caller must retain the same authenticated roots and reviewed
/// source/image contract. Errors poison the original finite state and owner.
pub(super) fn execute<'a>(
    base: Native<'a>,
    loaded: &'a mut LoadedArtifacts,
) -> Result<(LayerCompletion, Comparison)> {
    let layer = base.layer;
    coordinate_layer(&mut Layer { base, loaded }, layer)
}

#[cfg(test)]
mod tests;
