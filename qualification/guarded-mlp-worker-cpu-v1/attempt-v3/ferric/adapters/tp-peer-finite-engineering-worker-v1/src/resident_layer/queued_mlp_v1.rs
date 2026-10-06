//! Opt-in same-input comparison, not a replacement finite-worker contract.

use super::{Allocation, LayerBindings};
use fe2o3_kfd::{
    Gfx950EngineeringPeerDispatchV1 as Dispatch, Gfx950EngineeringPeerGroupV1 as Group,
    engineering_wire::BufferAccessV1 as Access,
};
use sha2::{Digest, Sha256};

mod artifacts;
pub(crate) use artifacts::{LoadedArtifacts, ReviewedImages, load};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComparisonProfile {
    FiniteThenQueuedLayerZeroV1,
}

impl ComparisonProfile {
    pub(super) fn validate(self, layer: usize) -> Result<()> {
        if layer != 0 {
            return Err("queued MLP comparison is restricted to layer zero".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Norm,
    Gate,
    Up,
    Activation,
    Down,
}

impl Stage {
    pub(crate) const ALL: [Self; 5] = [
        Self::Norm,
        Self::Gate,
        Self::Up,
        Self::Activation,
        Self::Down,
    ];

    fn index(self) -> usize {
        match self {
            Self::Norm => 0,
            Self::Gate => 1,
            Self::Up => 2,
            Self::Activation => 3,
            Self::Down => 4,
        }
    }

    fn output(self) -> usize {
        self.index() + 5
    }

    fn width(self) -> usize {
        if self == Self::Down { 4 } else { 2 }
    }

    fn output_bytes(self) -> usize {
        match self {
            Self::Norm => 8192,
            Self::Gate | Self::Up | Self::Activation => 12288,
            Self::Down => 16384,
        }
    }

    fn grid(self) -> [u32; 3] {
        [
            match self {
                Self::Norm => 64,
                Self::Gate | Self::Up => 393216,
                Self::Activation => 6144,
                Self::Down => 262144,
            },
            1,
            1,
        ]
    }

    fn total(self) -> usize {
        match self {
            Self::Norm => 352,
            Self::Activation => 312,
            _ => 328,
        }
    }

    fn hidden(self) -> u32 {
        match self {
            Self::Norm => 96,
            Self::Activation => 56,
            _ => 72,
        }
    }

    fn slices(self) -> usize {
        if self == Self::Norm { 5 } else { 3 }
    }

    fn scalars(self) -> &'static [u32] {
        match self {
            Self::Norm => &[1, 4096, 0x358637bd, 0],
            Self::Gate => &[1, 6144, 4096, 2, 4],
            Self::Up => &[1, 6144, 4096, 2, 5],
            Self::Activation => &[1, 2],
            Self::Down => &[1, 4096, 6144, 2, 2],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Binding {
    root: usize,
    offset: u32,
    bytes: u64,
    access: Access,
}

fn binding(root: usize, slot: u32, bytes: u64, access: Access) -> Binding {
    Binding {
        root,
        offset: slot * 16,
        bytes,
        access,
    }
}

fn bindings(stage: Stage) -> Vec<Binding> {
    match stage {
        Stage::Norm => vec![
            binding(0, 0, 8192, Access::Read),
            binding(0, 1, 0, Access::Read),
            binding(1, 2, 8192, Access::Read),
            binding(5, 3, 0, Access::Write),
            binding(5, 4, 8192, Access::Write),
        ],
        Stage::Gate | Stage::Up => vec![
            binding(5, 0, 8192, Access::Read),
            binding(
                if stage == Stage::Gate { 2 } else { 3 },
                1,
                50_331_648,
                Access::Read,
            ),
            binding(stage.output(), 2, 12288, Access::Write),
        ],
        Stage::Activation => vec![
            binding(6, 0, 12288, Access::Read),
            binding(7, 1, 12288, Access::Read),
            binding(8, 2, 12288, Access::Write),
        ],
        Stage::Down => vec![
            binding(8, 0, 12288, Access::Read),
            binding(4, 1, 50_331_648, Access::Read),
            binding(9, 2, 16384, Access::Write),
        ],
    }
}

fn stage_bytes(stage: Stage) -> Vec<u8> {
    let mut bytes = vec![0; stage.total()];
    for (slot, root) in bindings(stage).iter().enumerate() {
        let width = if stage == Stage::Down && slot == 2 {
            4
        } else {
            2
        };
        bytes[slot * 16 + 8..slot * 16 + 16].copy_from_slice(&(root.bytes / width).to_le_bytes());
    }
    let start = stage.slices() * 16;
    for (i, value) in stage.scalars().iter().enumerate() {
        bytes[start + i * 4..start + i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn sentinel(stage: Stage) -> Vec<u8> {
    let word: &[u8] = if stage == Stage::Down {
        &[1, 0, 192, 127]
    } else {
        &[193, 127]
    };
    word.repeat(stage.output_bytes() / stage.width())
}

fn finite_output(stage: Stage, bytes: &[u8]) -> Result<()> {
    if bytes.len() != stage.output_bytes() {
        return Err(format!("queued MLP {stage:?} output extent"));
    }
    for word in bytes.chunks_exact(stage.width()) {
        let finite = if stage == Stage::Down {
            f32::from_bits(u32::from_le_bytes(word.try_into().unwrap())).is_finite()
        } else {
            u16::from_le_bytes(word.try_into().unwrap()) & 0x7f80 != 0x7f80
        };
        if !finite {
            return Err(format!("queued MLP {stage:?} nonfinite output"));
        }
    }
    Ok(())
}

fn terminal(states: &[[u32; 11]; 2]) -> Result<()> {
    for words in states {
        if words[0] != 1
            || words[1] != 0
            || words[2] != 31
            || words[3] != 31
            || words[4] >> 10 != 0
            || words[5] != 0
            || words[6..] != [64; 5]
            || (0..5).any(|task| !matches!((words[4] >> (2 * task)) & 3, 1 | 2))
        {
            return Err("queued comparison requires actual finite MLP terminal states".into());
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

/// Private, interim evidence. The owning forward still has to complete and close.
/// Queue host durations exclude sentinel writes/readbacks and are not GPU times.
#[derive(Debug)]
pub(crate) struct Comparison {
    pub(crate) profile: ComparisonProfile,
    pub(crate) finite_states: [[u32; 11]; 2],
    pub(crate) finite_queue_host_ns: [u64; 2],
    pub(crate) queued_stage_host_ns: [[u64; 2]; 5],
    pub(crate) equality: [[OutputEquality; 2]; 5],
}

trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn read_output(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>>;
    fn write_output(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()>;
    fn stage(&mut self, stage: Stage) -> Result<[u64; 2]>;
}

fn coordinate(
    backend: &mut impl Backend,
    profile: ComparisonProfile,
    finite_states: [[u32; 11]; 2],
    finite_queue_host_ns: [u64; 2],
) -> Result<Comparison> {
    terminal(&finite_states)?;
    backend.validate()?;
    // Capture every finite-worker output before any queued stage overwrites it.
    // At most 122,880 bytes of expected tensor data are retained for this pair.
    let mut expected = Vec::with_capacity(5);
    for stage in Stage::ALL {
        let pair = [
            backend.read_output(stage, 0)?,
            backend.read_output(stage, 1)?,
        ];
        for bytes in &pair {
            finite_output(stage, bytes)?;
        }
        expected.push(pair);
    }
    let mut queued_stage_host_ns = [[0; 2]; 5];
    let mut equality = Vec::with_capacity(5);
    for stage in Stage::ALL {
        let poison = sentinel(stage);
        for rank in 0..2 {
            backend.write_output(stage, rank, &poison)?;
        }
        queued_stage_host_ns[stage.index()] = backend.stage(stage)?;
        let mut pair = Vec::with_capacity(2);
        for rank in 0..2 {
            let actual = backend.read_output(stage, rank)?;
            finite_output(stage, &actual)?;
            if actual != expected[stage.index()][rank] {
                return Err(format!(
                    "queued MLP {stage:?} rank {rank} differs from actual finite output"
                ));
            }
            pair.push(OutputEquality {
                stage,
                rank,
                bytes: actual.len(),
                words: actual.len() / stage.width(),
                sha256: Sha256::digest(&actual).into(),
            });
        }
        equality.push(
            pair.try_into()
                .map_err(|_| "queued MLP equality rank count")?,
        );
    }
    Ok(Comparison {
        profile,
        finite_states,
        finite_queue_host_ns,
        queued_stage_host_ns,
        equality: equality
            .try_into()
            .map_err(|_| "queued MLP equality stage count")?,
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
        if !(1..=10_000).contains(&self.timeout_ms) {
            return Err("queued MLP deadline".into());
        }
        for rank in 0..2 {
            for stage in Stage::ALL {
                if self.artifacts.kernel(rank, stage)?.rank() != rank {
                    return Err("queued MLP artifact owner".into());
                }
                for binding in bindings(stage) {
                    let buffer = self.roots.mlp[rank][binding.root];
                    if buffer.owner() != rank || buffer.bytes() < binding.bytes {
                        return Err("queued MLP root owner/extent".into());
                    }
                }
            }
        }
        Ok(())
    }
    fn read_output(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>> {
        self.group.read(
            self.roots.mlp[rank][stage.output()],
            0,
            stage.output_bytes() as u32,
        )
    }
    fn write_output(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()> {
        self.group
            .write(self.roots.mlp[rank][stage.output()], 0, bytes)
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
                            self.roots.mlp[rank][binding.root].pointer(
                                binding.offset,
                                0,
                                binding.bytes,
                                binding.access,
                            )
                        })
                        .collect(),
                    timeout_ms: self.timeout_ms,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        // SAFETY: only the exact retained entry/ABI/grid is selected. Both rank
        // outputs are disjoint, all inputs completed, and no peer writes exist.
        unsafe { self.group.dispatch_round_unchecked(commands)? }
            .try_into()
            .map_err(|_| "queued MLP paired completion count".into())
    }
}

/// The private layer caller supplies the just-acquired finite state, never a
/// queued substitute. Any error must poison that layer's state and owner.
/// # Safety
/// The caller must preserve the same authenticated roots, exact reviewed images
/// and exclusive group lifetime as the finite MLP, after its actual completion
/// and before final residual. This comparison is not production admission.
pub(super) unsafe fn compare(
    group: &mut Group,
    artifacts: &LoadedArtifacts,
    roots: &LayerBindings,
    timeout_ms: u32,
    profile: ComparisonProfile,
    finite_states: [[u32; 11]; 2],
    finite_queue_host_ns: [u64; 2],
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
        finite_queue_host_ns,
    )
}

#[cfg(test)]
mod tests;
