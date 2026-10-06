//! Distinct same-input Q/K/V/O comparison, not a finite-prefix replacement.

use super::{Allocation, LayerBindings};
use fe2o3_kfd::{
    Gfx950EngineeringPeerDispatchV1 as Dispatch, Gfx950EngineeringPeerGroupV1 as Group,
    engineering_wire::BufferAccessV1 as Access,
};
use sha2::{Digest, Sha256};

mod artifacts;
pub(crate) use artifacts::{LoadedArtifacts, ReviewedImage, load};
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComparisonProfile {
    FiniteThenQueuedProjectionsLayerZeroV1,
}
impl ComparisonProfile {
    pub(super) fn validate(self, layer: usize) -> Result<()> {
        if layer != 0 {
            return Err("queued projection comparison requires layer zero".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Query,
    Key,
    Value,
    Output,
}
impl Stage {
    pub(crate) const ALL: [Self; 4] = [Self::Query, Self::Key, Self::Value, Self::Output];
    fn index(self) -> usize {
        match self {
            Self::Query => 0,
            Self::Key => 1,
            Self::Value => 2,
            Self::Output => 3,
        }
    }
    fn output(self) -> View {
        match self {
            Self::Query => View {
                root: 8,
                offset: 0,
                bytes: 4096,
            },
            Self::Key => View {
                root: 8,
                offset: 4096,
                bytes: 1024,
            },
            Self::Value => View {
                root: 8,
                offset: 5120,
                bytes: 1024,
            },
            Self::Output => View {
                root: 13,
                offset: 0,
                bytes: 16384,
            },
        }
    }
    fn width(self) -> u64 {
        if self == Self::Output { 4 } else { 2 }
    }
    fn scalars(self) -> [u32; 5] {
        match self {
            Self::Query => [1, 2048, 4096, 2, 1],
            Self::Key => [1, 512, 4096, 2, 2],
            Self::Value => [1, 512, 4096, 2, 3],
            Self::Output => [1, 4096, 2048, 2, 1],
        }
    }
    fn grid(self) -> [u32; 3] {
        [self.scalars()[1] * 64, 1, 1]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct View {
    root: usize,
    offset: u64,
    bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Binding {
    view: View,
    kernarg_offset: u32,
    access: Access,
}
fn bindings(stage: Stage) -> [Binding; 3] {
    let (input, weight) = match stage {
        Stage::Query => (
            View {
                root: 7,
                offset: 0,
                bytes: 8192,
            },
            View {
                root: 2,
                offset: 0,
                bytes: 16777216,
            },
        ),
        Stage::Key => (
            View {
                root: 7,
                offset: 0,
                bytes: 8192,
            },
            View {
                root: 2,
                offset: 16777216,
                bytes: 4194304,
            },
        ),
        Stage::Value => (
            View {
                root: 7,
                offset: 0,
                bytes: 8192,
            },
            View {
                root: 2,
                offset: 20971520,
                bytes: 4194304,
            },
        ),
        Stage::Output => (
            View {
                root: 12,
                offset: 0,
                bytes: 4096,
            },
            View {
                root: 6,
                offset: 0,
                bytes: 16777216,
            },
        ),
    };
    [
        Binding {
            view: input,
            kernarg_offset: 0,
            access: Access::Read,
        },
        Binding {
            view: weight,
            kernarg_offset: 16,
            access: Access::Read,
        },
        Binding {
            view: stage.output(),
            kernarg_offset: 32,
            access: Access::Write,
        },
    ]
}
fn stage_bytes(stage: Stage) -> Vec<u8> {
    let mut bytes = vec![0; 328];
    for (slot, binding) in bindings(stage).iter().enumerate() {
        let width = if slot == 2 { stage.width() } else { 2 };
        bytes[slot * 16 + 8..slot * 16 + 16]
            .copy_from_slice(&(binding.view.bytes / width).to_le_bytes());
    }
    for (index, value) in stage.scalars().iter().enumerate() {
        bytes[48 + index * 4..52 + index * 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}
fn sentinel(stage: Stage) -> Vec<u8> {
    let word: &[u8] = if stage == Stage::Output {
        &[1, 0, 192, 127]
    } else {
        &[193, 127]
    };
    word.repeat((stage.output().bytes / stage.width()) as usize)
}
fn finite(stage: Stage, bytes: &[u8]) -> Result<()> {
    if bytes.len() != stage.output().bytes as usize {
        return Err("queued projection output extent".into());
    }
    for word in bytes.chunks_exact(stage.width() as usize) {
        let valid = if stage == Stage::Output {
            f32::from_bits(u32::from_le_bytes(word.try_into().unwrap())).is_finite()
        } else {
            u16::from_le_bytes(word.try_into().unwrap()) & 0x7f80 != 0x7f80
        };
        if !valid {
            return Err(format!("queued projection {stage:?} nonfinite output"));
        }
    }
    Ok(())
}
fn terminal(states: &[[u32; 22]; 2]) -> Result<()> {
    for words in states {
        if words[0] != 1
            || words[1] != 0
            || words[2] != 65535
            || words[3] != 65535
            || words[5] != 0
            || words[6..] != [64; 16]
            || (0..16).any(|task| !matches!((words[4] >> (task * 2)) & 3, 1 | 2))
        {
            return Err("queued projections require acquired finite prefix terminal states".into());
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutputEquality {
    pub(crate) stage: Stage,
    pub(crate) rank: usize,
    pub(crate) bytes: usize,
    pub(crate) words: usize,
    pub(crate) sha256: [u8; 32],
}
/// Interim same-owner evidence; whole-forward completion and close remain pending.
#[derive(Debug)]
pub(crate) struct Comparison {
    pub(crate) profile: ComparisonProfile,
    pub(crate) finite_states: [[u32; 22]; 2],
    /// Whole finite prefix host duration, not a per-projection measurement.
    pub(crate) finite_prefix_host_ns: [u64; 2],
    /// Paired queued host durations exclude sentinel writes and comparison reads.
    pub(crate) queued_stage_host_ns: [[u64; 2]; 4],
    pub(crate) equality: [[OutputEquality; 2]; 4],
}
trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn read(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>>;
    fn write(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()>;
    fn stage(&mut self, stage: Stage) -> Result<[u64; 2]>;
}
fn coordinate(
    backend: &mut impl Backend,
    profile: ComparisonProfile,
    finite_states: [[u32; 22]; 2],
    finite_prefix_host_ns: [u64; 2],
) -> Result<Comparison> {
    terminal(&finite_states)?;
    backend.validate()?;
    // Snapshot all four disjoint views before touching shared raw-QKV storage.
    // Expected data is bounded to 45,056 bytes across both ranks.
    let mut expected = Vec::with_capacity(4);
    for stage in Stage::ALL {
        let pair = [backend.read(stage, 0)?, backend.read(stage, 1)?];
        for bytes in &pair {
            finite(stage, bytes)?;
        }
        expected.push(pair);
    }
    let mut timings = [[0; 2]; 4];
    let mut equality = Vec::with_capacity(4);
    for stage in Stage::ALL {
        let poison = sentinel(stage);
        for rank in 0..2 {
            backend.write(stage, rank, &poison)?;
        }
        timings[stage.index()] = backend.stage(stage)?;
        // A Q/K overrun must not be hidden by later K/V recomputation. Validate
        // every previously computed and not-yet-selected view after each stage.
        for checked in Stage::ALL {
            for rank in 0..2 {
                let actual = backend.read(checked, rank)?;
                finite(checked, &actual)?;
                if actual != expected[checked.index()][rank] {
                    return Err(format!(
                        "queued projection {stage:?} changed {checked:?} rank {rank} from finite output"
                    ));
                }
            }
        }
        equality.push(std::array::from_fn(|rank| {
            let bytes = &expected[stage.index()][rank];
            OutputEquality {
                stage,
                rank,
                bytes: bytes.len(),
                words: bytes.len() / stage.width() as usize,
                sha256: Sha256::digest(bytes).into(),
            }
        }));
    }
    Ok(Comparison {
        profile,
        finite_states,
        finite_prefix_host_ns,
        queued_stage_host_ns: timings,
        equality: equality
            .try_into()
            .map_err(|_| "queued projection equality census")?,
    })
}

struct Native<'a> {
    group: &'a mut Group,
    artifacts: &'a LoadedArtifacts,
    roots: &'a LayerBindings,
    timeout_ms: u32,
}
impl Backend for Native<'_> {
    fn validate(&mut self) -> Result<()> {
        self.roots.validate()?;
        if !(1..=10000).contains(&self.timeout_ms) {
            return Err("queued projection deadline".into());
        }
        for rank in 0..2 {
            for stage in Stage::ALL {
                if self.artifacts.kernel(rank, stage)?.rank() != rank {
                    return Err("queued projection artifact owner".into());
                }
                for binding in bindings(stage) {
                    let view = binding.view;
                    let buffer = self.roots.prefix[rank][view.root];
                    let end = view
                        .offset
                        .checked_add(view.bytes)
                        .ok_or("queued projection view overflow")?;
                    if buffer.owner() != rank || end > buffer.bytes() {
                        return Err("queued projection root owner/subview extent".into());
                    }
                }
            }
        }
        Ok(())
    }
    fn read(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>> {
        let view = stage.output();
        self.group.read(
            self.roots.prefix[rank][view.root],
            view.offset,
            view.bytes as u32,
        )
    }
    fn write(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()> {
        let view = stage.output();
        self.group
            .write(self.roots.prefix[rank][view.root], view.offset, bytes)
    }
    fn stage(&mut self, stage: Stage) -> Result<[u64; 2]> {
        let commands = (0..2)
            .map(|rank| {
                Ok(Dispatch {
                    kernel: self.artifacts.kernel(rank, stage)?,
                    bytes: stage_bytes(stage),
                    workgroup: [64, 1, 1],
                    grid: stage.grid(),
                    pointers: bindings(stage)
                        .into_iter()
                        .map(|binding| {
                            let view = binding.view;
                            self.roots.prefix[rank][view.root].pointer(
                                binding.kernarg_offset,
                                view.offset,
                                view.bytes,
                                binding.access,
                            )
                        })
                        .collect(),
                    timeout_ms: self.timeout_ms,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        // SAFETY: exact closed entries and rank-local subviews; both outputs
        // are disjoint and the actual finite prefix has fully completed.
        unsafe { self.group.dispatch_round_unchecked(commands)? }
            .try_into()
            .map_err(|_| "queued projection paired completion count".into())
    }
}

/// # Safety
/// The private caller must retain the same authenticated prefix roots and exact
/// reviewed image in its exclusive group, after real prefix state acquire and
/// before first residual. Every error poisons that finite roster and owner.
pub(super) unsafe fn compare(
    group: &mut Group,
    artifacts: &LoadedArtifacts,
    roots: &LayerBindings,
    timeout_ms: u32,
    profile: ComparisonProfile,
    finite_states: [[u32; 22]; 2],
    finite_prefix_host_ns: [u64; 2],
) -> Result<Comparison> {
    coordinate(
        &mut Native {
            group,
            artifacts,
            roots,
            timeout_ms,
        },
        profile,
        finite_states,
        finite_prefix_host_ns,
    )
}

#[cfg(test)]
mod tests;
