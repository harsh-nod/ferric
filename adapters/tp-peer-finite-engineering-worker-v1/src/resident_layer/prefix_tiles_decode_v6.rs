//! All-layer typed284/548 dispatch, with both paired scratch-consumer fences.
use super::mlp_tiles_v2::artifacts::LoadedKernels;
use super::prefix_tiles_v6::artifacts::Loaded as LoadedPrefix;
use super::{
    Access, Dispatch, Group, LayerBindings, LoadedResidentArtifacts, ResidentKind, Result,
    consumer_bytes,
};
use crate::state_roster::prefix_tiles_decode_v6::Roster;
use fe2o3_kfd::{
    Gfx950EngineeringPeerWaveMlpTilesDispatchV2 as MlpDispatch,
    Gfx950EngineeringPeerWaveQkvAttentionOutputTilesDispatchV6 as PrefixDispatch,
};

pub(crate) struct Completion {
    pub(crate) prefix_states: [[u32; 284]; 2],
    pub(crate) mlp_states: [[u32; 548]; 2],
    pub(crate) paired_ns: [[u64; 2]; 4],
}
trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])>;
    fn residual(&mut self, first: bool) -> Result<[u64; 2]>;
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])>;
    fn poison(&mut self);
}
fn coordinate(b: &mut impl Backend) -> Result<Completion> {
    let result = (|| {
        b.validate()?;
        let (prefix_states, prefix) = b.prefix()?;
        let first = b.residual(true)?;
        let (mlp_states, mlp) = b.mlp()?;
        let last = b.residual(false)?;
        Ok(Completion {
            prefix_states,
            mlp_states,
            paired_ns: [prefix, first, mlp, last],
        })
    })();
    if result.is_err() {
        b.poison();
    }
    result
}
struct Native<'a> {
    group: &'a mut Group,
    states: &'a mut Roster,
    original: &'a LoadedResidentArtifacts,
    prefix: &'a LoadedPrefix,
    mlp: &'a LoadedKernels,
    roots: &'a LayerBindings,
    layer: usize,
    timeout_ms: u32,
}
impl Backend for Native<'_> {
    fn validate(&mut self) -> Result<()> {
        if self.layer >= 36 || !(1..=10_000).contains(&self.timeout_ms) {
            return Err("prefix decode layer/deadline".into());
        }
        self.roots.validate()?;
        for rank in 0..2 {
            if self.prefix.kernels[rank].rank() != rank
                || self.mlp.kernels[rank].rank() != rank
                || self.original.kernel(rank, ResidentKind::Residual)?.rank() != rank
            {
                return Err("prefix decode artifact owner".into());
            }
        }
        Ok(())
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])> {
        let completion = (|| {
            let [left, right] = self.states.take_prefix(self.layer)?;
            // SAFETY: sealed original role order, image identity and private
            // same-owner typed284 pair; no command/reference escapes this call.
            unsafe {
                self.group
                    .dispatch_wave_qkv_attention_output_tiles_round_v6([
                        PrefixDispatch {
                            kernel: &self.prefix.kernels[0],
                            object_sha256: self.prefix.sha256,
                            roots: self.roots.prefix[0],
                            state: left,
                            timeout_ms: self.timeout_ms,
                        },
                        PrefixDispatch {
                            kernel: &self.prefix.kernels[1],
                            object_sha256: self.prefix.sha256,
                            roots: self.roots.prefix[1],
                            state: right,
                            timeout_ms: self.timeout_ms,
                        },
                    ])
            }
        })();
        let words = self.states.finish_prefix(
            self.group,
            self.layer,
            completion
                .as_ref()
                .map(|v| v.final_states)
                .map_err(Clone::clone),
        )?;
        Ok((words, completion?.dispatch_elapsed_ns))
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        self.states.begin_residual(self.layer, first)?;
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
                    kernel: self.original.kernel(rank, ResidentKind::Residual)?,
                    bytes: consumer_bytes(),
                    workgroup: [64, 1, 1],
                    grid: [4096, 1, 1],
                    pointers,
                    timeout_ms: self.timeout_ms,
                });
            }
            // SAFETY: both partial producers completed before either residual
            // consumer; both consumers finish before shared scratch is reused.
            unsafe { self.group.dispatch_round_unchecked(commands)? }
                .try_into()
                .map_err(|_| "prefix decode residual pair".into())
        })();
        self.states
            .finish_residual(self.layer, first, result.clone().map(|_| ()))?;
        result
    }
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])> {
        let completion = (|| {
            let [left, right] = self.states.take_mlp(self.layer)?;
            // SAFETY: unchanged typed548 dispatch and exact original model roots.
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
        let words = self.states.finish_mlp(
            self.group,
            self.layer,
            completion
                .as_ref()
                .map(|v| v.final_states)
                .map_err(Clone::clone),
        )?;
        Ok((words, completion?.dispatch_elapsed_ns))
    }
    fn poison(&mut self) {
        self.states.poison();
    }
}
/// # Safety
/// The exclusive model owner binds actual reviewed source/images and roots;
/// the private roster binds the layer/bank generation and retired commands.
pub(crate) unsafe fn execute(
    group: &mut Group,
    states: &mut Roster,
    original: &LoadedResidentArtifacts,
    prefix: &LoadedPrefix,
    mlp: &LoadedKernels,
    roots: &LayerBindings,
    layer: usize,
    timeout_ms: u32,
) -> Result<Completion> {
    coordinate(&mut Native {
        group,
        states,
        original,
        prefix,
        mlp,
        roots,
        layer,
        timeout_ms,
    })
}

#[cfg(test)]
mod tests;
