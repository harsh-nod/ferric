//! Layer0 capture with distinct prefix states and unchanged paired residuals.
use super::mlp_tiles_v2::artifacts::LoadedKernels;
use super::{
    Access, Dispatch, Group, LayerBindings, LoadedResidentArtifacts, PREFIX_BYTES, ResidentKind,
    Result, consumer_bytes,
};
use crate::state_roster::{prefix_tiles_v6 as prefix, tiles_decode_v1 as old};
use fe2o3_kfd::{
    Gfx950EngineeringPeerBufferV1 as Buffer,
    Gfx950EngineeringPeerWaveMlpTilesDispatchV2 as MlpDispatch,
    Gfx950EngineeringPeerWaveQkvAttentionOutputTilesDispatchV6 as PrefixDispatch,
};

pub(crate) mod artifacts;
pub(crate) mod projection_residual;
pub(crate) enum States<'a> {
    Baseline(&'a mut old::Roster),
    Tiles(&'a mut prefix::Roster),
}
#[derive(Debug)]
pub(crate) enum PrefixObservation {
    Baseline22([[u32; 22]; 2]),
    Tiles284([[u32; 284]; 2]),
}
#[derive(Debug)]
pub(crate) struct Completion {
    pub(crate) prefix: PrefixObservation,
    pub(crate) mlp: [[u32; 548]; 2],
    pub(crate) paired_ns: [[u64; 2]; 4],
}
#[derive(Debug)]
pub(crate) struct Capture {
    /// Rank then normalized, QKV, query, key cache, value cache, attention,
    /// and F32 output partial. Acquired before the first residual/MLP reuse.
    pub(crate) prefix: [[Vec<u8>; 7]; 2],
    pub(crate) first_residual: [Vec<u8>; 2],
    /// Rank then normalized, gate, up, activation, and F32 down partial.
    pub(crate) mlp: [[Vec<u8>; 5]; 2],
    pub(crate) final_hidden: [Vec<u8>; 2],
}
pub(crate) const CAPTURE_BYTES: usize = 9_670_656;
pub(crate) struct Run {
    pub(crate) completion: Completion,
    pub(crate) capture: Capture,
}

trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn prefix(&mut self) -> Result<(PrefixObservation, [u64; 2])>;
    fn prefix_capture(&mut self) -> Result<[[Vec<u8>; 7]; 2]>;
    fn residual(&mut self, first: bool) -> Result<[u64; 2]>;
    fn residual_capture(&mut self, first: bool) -> Result<[Vec<u8>; 2]>;
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])>;
    fn mlp_capture(&mut self) -> Result<[[Vec<u8>; 5]; 2]>;
    fn poison(&mut self);
}
fn coordinate(b: &mut impl Backend) -> Result<Run> {
    let result = (|| {
        b.validate()?;
        let (prefix, prefix_ns) = b.prefix()?;
        let prefix_capture = b.prefix_capture()?;
        let first = b.residual(true)?;
        let first_residual = b.residual_capture(true)?;
        let (mlp, mlp_ns) = b.mlp()?;
        let mlp_capture = b.mlp_capture()?;
        let last = b.residual(false)?;
        let final_hidden = b.residual_capture(false)?;
        Ok(Run {
            completion: Completion {
                prefix,
                mlp,
                paired_ns: [prefix_ns, first, mlp_ns, last],
            },
            capture: Capture {
                prefix: prefix_capture,
                first_residual,
                mlp: mlp_capture,
                final_hidden,
            },
        })
    })();
    if result.is_err() {
        b.poison();
    }
    result
}
fn finite(bytes: &[u8], f32: bool) -> Result<()> {
    let width = if f32 { 4 } else { 2 };
    if bytes.is_empty()
        || bytes.len() % width != 0
        || bytes.chunks_exact(width).any(|b| {
            if f32 {
                u32::from_le_bytes(b.try_into().unwrap()) & 0x7f800000 == 0x7f800000
            } else {
                u16::from_le_bytes(b.try_into().unwrap()) & 0x7f80 == 0x7f80
            }
        })
    {
        return Err("nonfinite layer capture".into());
    }
    Ok(())
}
struct Native<'a> {
    group: &'a mut Group,
    states: States<'a>,
    original: &'a LoadedResidentArtifacts,
    prefix: Option<&'a artifacts::Loaded>,
    projection: Option<&'a projection_residual::Loaded>,
    mlp: &'a LoadedKernels,
    roots: &'a LayerBindings,
    timeout_ms: u32,
}
fn select_residual<'a, T>(
    projection: Option<&'a [T; 2]>,
    rank: usize,
    original: impl FnOnce(usize) -> Result<&'a T>,
) -> Result<&'a T> {
    if rank >= 2 {
        return Err("projection residual rank".into());
    }
    match projection {
        Some(pair) => Ok(&pair[rank]),
        None => original(rank),
    }
}
impl Native<'_> {
    fn read<const N: usize>(
        &mut self,
        roots: [[Buffer; N]; 2],
        extents: [u64; N],
        last_f32: bool,
    ) -> Result<[[Vec<u8>; N]; 2]> {
        let mut ranks = Vec::with_capacity(2);
        for rank in 0..2 {
            let mut rows = Vec::with_capacity(N);
            for i in 0..N {
                let bytes = self.group.read(roots[rank][i], 0, extents[i] as u32)?;
                if bytes.len() as u64 != extents[i] {
                    return Err("layer capture exact extent".into());
                }
                finite(&bytes, last_f32 && i + 1 == N)?;
                rows.push(bytes);
            }
            ranks.push(rows.try_into().map_err(|_| "capture row count")?);
        }
        ranks.try_into().map_err(|_| "capture rank count".into())
    }
}
impl Backend for Native<'_> {
    fn validate(&mut self) -> Result<()> {
        self.roots.validate()?;
        if !(1..=10000).contains(&self.timeout_ms)
            || matches!(&self.states, States::Tiles(_)) != self.prefix.is_some()
            || (self.projection.is_some() && self.prefix.is_none())
        {
            return Err("prefix layer image/state profile".into());
        }
        for rank in 0..2 {
            if select_residual(self.projection.map(|p| &p.kernels), rank, |r| {
                self.original.kernel(r, ResidentKind::Residual)
            })?
            .rank()
                != rank
                || self.mlp.kernels[rank].rank() != rank
            {
                return Err("prefix layer artifact owner".into());
            }
        }
        Ok(())
    }
    fn prefix(&mut self) -> Result<(PrefixObservation, [u64; 2])> {
        match &mut self.states {
            States::Baseline(states) => {
                let completion: Result<[u64; 2]> = (|| {
                    let pair = states.take_prefix(0)?;
                    let mut commands = Vec::with_capacity(2);
                    for rank in 0..2 {
                        let mut pointers = self.roots.prefix[rank]
                            .iter()
                            .zip(PREFIX_BYTES)
                            .enumerate()
                            .map(|(i, (b, n))| {
                                b.pointer(
                                    i as u32 * 8,
                                    0,
                                    n,
                                    if i < 7 {
                                        Access::Read
                                    } else {
                                        Access::ReadWrite
                                    },
                                )
                            })
                            .collect::<Vec<_>>();
                        pointers.push(pair[rank].pointer_v5());
                        commands.push(Dispatch {
                            kernel: self.original.kernel(rank, ResidentKind::Prefix)?,
                            bytes: vec![0; 376],
                            workgroup: [64, 1, 1],
                            grid: [128, 1, 1],
                            pointers,
                            timeout_ms: self.timeout_ms,
                        });
                    }
                    // SAFETY: unchanged V5 source roles, exact baseline profile and private typed22 pair.
                    unsafe { self.group.dispatch_round_unchecked(commands)? }
                        .try_into()
                        .map_err(|_| "baseline prefix pair".into())
                })();
                let words = states.finish_prefix(self.group, 0, completion.clone().map(|_| ()))?;
                Ok((PrefixObservation::Baseline22(words), completion?))
            }
            States::Tiles(states) => {
                let image = self.prefix.ok_or("missing prefix284 image")?;
                let completion = (|| {
                    let [left, right] = states.take_prefix()?;
                    // SAFETY: authenticated original catalog roots and closed typed284 pair; no raw state pointer.
                    unsafe {
                        self.group
                            .dispatch_wave_qkv_attention_output_tiles_round_v6([
                                PrefixDispatch {
                                    kernel: &image.kernels[0],
                                    object_sha256: image.sha256,
                                    roots: self.roots.prefix[0],
                                    state: left,
                                    timeout_ms: self.timeout_ms,
                                },
                                PrefixDispatch {
                                    kernel: &image.kernels[1],
                                    object_sha256: image.sha256,
                                    roots: self.roots.prefix[1],
                                    state: right,
                                    timeout_ms: self.timeout_ms,
                                },
                            ])
                    }
                })();
                let words = states.finish_prefix(
                    self.group,
                    completion
                        .as_ref()
                        .map(|v| v.final_states)
                        .map_err(Clone::clone),
                )?;
                Ok((
                    PrefixObservation::Tiles284(words),
                    completion?.dispatch_elapsed_ns,
                ))
            }
        }
    }
    fn prefix_capture(&mut self) -> Result<[[Vec<u8>; 7]; 2]> {
        self.read(
            self.roots
                .prefix
                .map(|r| core::array::from_fn(|i| r[i + 7])),
            [8192, 6144, 4096, 2359296, 2359296, 4096, 16384],
            true,
        )
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        match &mut self.states {
            States::Baseline(s) => s.begin_residual(0, first)?,
            States::Tiles(s) => s.begin_residual(first)?,
        }
        let result: Result<[u64; 2]> = (|| {
            let mut commands = Vec::with_capacity(2);
            for rank in 0..2 {
                let mut pointers = (0..8)
                    .map(|slot| {
                        self.roots.prefix[if slot < 2 { slot } else { 0 }][13].pointer(
                            slot as u32 * 16,
                            0,
                            if slot < 2 { 16384 } else { 0 },
                            Access::Read,
                        )
                    })
                    .collect::<Vec<_>>();
                let (original, output) = if first {
                    (self.roots.prefix[rank][0], self.roots.mlp[rank][0])
                } else {
                    (self.roots.mlp[rank][0], self.roots.final_hidden[rank])
                };
                pointers.push(original.pointer(128, 0, 8192, Access::Read));
                pointers.push(output.pointer(144, 0, 8192, Access::Write));
                commands.push(Dispatch {
                    kernel: select_residual(self.projection.map(|p| &p.kernels), rank, |r| {
                        self.original.kernel(r, ResidentKind::Residual)
                    })?,
                    bytes: consumer_bytes(),
                    workgroup: [64, 1, 1],
                    grid: [4096, 1, 1],
                    pointers,
                    timeout_ms: self.timeout_ms,
                });
            }
            // SAFETY: the selected image has the same reviewed TP2 ABI; both
            // partial producers completed before either rank reads.
            unsafe { self.group.dispatch_round_unchecked(commands)? }
                .try_into()
                .map_err(|_| "residual pair count".into())
        })();
        match &mut self.states {
            States::Baseline(s) => s.finish_residual(0, first, result.clone().map(|_| ()))?,
            States::Tiles(s) => s.finish_residual(first, result.clone().map(|_| ()))?,
        }
        result
    }
    fn residual_capture(&mut self, first: bool) -> Result<[Vec<u8>; 2]> {
        let buffers = if first {
            self.roots.mlp.map(|r| [r[0]])
        } else {
            self.roots.final_hidden.map(|b| [b])
        };
        let pair = self.read(buffers, [8192], false)?.map(|[v]| v);
        if pair[0] != pair[1] {
            return Err("paired residual hidden disagreement".into());
        }
        Ok(pair)
    }
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])> {
        let result = (|| {
            let [left, right] = match &mut self.states {
                States::Baseline(s) => s.take_tiles(0)?,
                States::Tiles(s) => s.take_mlp()?,
            };
            // SAFETY: unchanged typed548 API, catalog role order and same-owner pair.
            unsafe {
                self.group.dispatch_wave_mlp_tiles_round_v2([
                    MlpDispatch {
                        kernel: &self.mlp.kernels[0],
                        object_sha256: self.mlp.sha256,
                        roots: self.roots.mlp[0],
                        state: left,
                        timeout_ms: self.timeout_ms,
                    },
                    MlpDispatch {
                        kernel: &self.mlp.kernels[1],
                        object_sha256: self.mlp.sha256,
                        roots: self.roots.mlp[1],
                        state: right,
                        timeout_ms: self.timeout_ms,
                    },
                ])
            }
        })();
        let observed = result
            .as_ref()
            .map(|v| v.final_states)
            .map_err(Clone::clone);
        let words = match &mut self.states {
            States::Baseline(s) => s.finish_tiles(self.group, 0, observed)?,
            States::Tiles(s) => s.finish_mlp(self.group, observed)?,
        };
        Ok((words, result?.dispatch_elapsed_ns))
    }
    fn mlp_capture(&mut self) -> Result<[[Vec<u8>; 5]; 2]> {
        self.read(
            self.roots.mlp.map(|r| core::array::from_fn(|i| r[i + 5])),
            [8192, 12288, 12288, 12288, 16384],
            true,
        )
    }
    fn poison(&mut self) {
        match &mut self.states {
            States::Baseline(s) => s.poison(),
            States::Tiles(s) => s.poison(),
        }
    }
}
/// # Safety
/// The consuming model owner binds actual source, images, roles and numerical/runtime premises.
pub(crate) unsafe fn execute(
    group: &mut Group,
    states: States<'_>,
    original: &LoadedResidentArtifacts,
    prefix: Option<&artifacts::Loaded>,
    mlp: &LoadedKernels,
    roots: &LayerBindings,
    timeout_ms: u32,
) -> Result<Run> {
    unsafe {
        execute_selected(
            group, states, original, prefix, None, mlp, roots, timeout_ms,
        )
    }
}
/// # Safety
/// The selected projection image additionally requires independent source and
/// numerical review. No selection changes the original image or copy kernel.
pub(crate) unsafe fn execute_selected(
    group: &mut Group,
    states: States<'_>,
    original: &LoadedResidentArtifacts,
    prefix: Option<&artifacts::Loaded>,
    projection: Option<&projection_residual::Loaded>,
    mlp: &LoadedKernels,
    roots: &LayerBindings,
    timeout_ms: u32,
) -> Result<Run> {
    coordinate(&mut Native {
        group,
        states,
        original,
        prefix,
        projection,
        mlp,
        roots,
        timeout_ms,
    })
}

#[cfg(test)]
mod tests;
