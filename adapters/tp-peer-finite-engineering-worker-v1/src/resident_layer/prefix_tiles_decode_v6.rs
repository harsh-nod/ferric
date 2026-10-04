//! All-layer typed284/548 dispatch, with both paired scratch-consumer fences.
use super::mlp_tiles_v2::artifacts::LoadedKernels;
use super::prefix_tiles_v6::artifacts::Loaded as LoadedPrefix;
use super::prefix_tiles_v6::projection_residual::Loaded as LoadedProjection;
use super::prefix_tiles_v6::select_residual;
use super::{
    Access, Dispatch, Group, LayerBindings, LoadedResidentArtifacts, ResidentKind, Result,
    consumer_bytes,
};
use crate::native_prefix_device_recorder_v1::Recorder;
use crate::prefix_decode_device_observation_v1::Stage;
use crate::state_roster::prefix_tiles_decode_v6::Roster;
use fe2o3_kfd::{
    Gfx950EngineeringPeerKernelV1 as Kernel,
    Gfx950EngineeringPeerWaveMlpTilesDispatchV2 as MlpDispatch,
    Gfx950EngineeringPeerWaveQkvAttentionOutputTilesDispatchV6 as PrefixDispatch,
    Gfx950EngineeringRawTimestampObservationV1 as Raw,
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
    projection: Option<&'a LoadedProjection>,
    prefix: &'a LoadedPrefix,
    mlp: &'a LoadedKernels,
    roots: &'a LayerBindings,
    layer: usize,
    timeout_ms: u32,
    recording: Option<(&'a mut Recorder, u64, u32)>,
}
fn residual_kernel<'a>(
    original: &'a LoadedResidentArtifacts,
    projection: Option<&'a LoadedProjection>,
    rank: usize,
) -> Result<&'a Kernel> {
    select_residual(projection.map(|p| &p.kernels), rank, |r| {
        original.kernel(r, ResidentKind::Residual)
    })
}
fn residual_roots<T: Copy>(first: bool, prefix: T, mlp: T, final_hidden: T) -> (T, T) {
    if first {
        (prefix, mlp)
    } else {
        (mlp, final_hidden)
    }
}
fn record_pair(
    recording: &mut Option<(&mut Recorder, u64, u32)>,
    stage: Stage,
    layer: usize,
    kernels: [&Kernel; 2],
    raw: [Raw; 2],
) -> Result<()> {
    let (recorder, generation, position) =
        recording.as_mut().ok_or("missing layer device recorder")?;
    for rank in 0..2 {
        recorder
            .record(
                *generation,
                *position,
                stage,
                Some(layer as u32),
                kernels[rank],
                &raw[rank],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
impl Backend for Native<'_> {
    fn validate(&mut self) -> Result<()> {
        if self.layer >= 36 || !(1..=10_000).contains(&self.timeout_ms) {
            return Err("prefix decode layer/deadline".into());
        }
        if let Some((_, generation, position)) = self.recording.as_ref() {
            if *position >= 4 || *generation != u64::from(*position) + 1 {
                return Err("prefix decode raw recording generation".into());
            }
        }
        if self.projection.is_some() && self.recording.is_some() {
            return Err("projection decode and legacy device recording are distinct".into());
        }
        self.roots.validate()?;
        for rank in 0..2 {
            if self.prefix.kernels[rank].rank() != rank
                || self.mlp.kernels[rank].rank() != rank
                || residual_kernel(self.original, self.projection, rank)?.rank() != rank
            {
                return Err("prefix decode artifact owner".into());
            }
        }
        Ok(())
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])> {
        let mut raw = None;
        let completion = (|| {
            let [left, right] = self.states.take_prefix(self.layer)?;
            // SAFETY: sealed original role order, image identity and private
            // same-owner typed284 pair; no command/reference escapes this call.
            let commands = [
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
            ];
            unsafe {
                if self.recording.is_some() {
                    self.group.dispatch_wave_qkv_attention_output_tiles_round_with_raw_timestamps_unchecked_v1(commands)
                        .map(|(completion, observations)| { raw = Some(observations); completion })
                } else {
                    self.group
                        .dispatch_wave_qkv_attention_output_tiles_round_v6(commands)
                }
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
        let completion = completion?;
        if let Some(raw) = raw {
            record_pair(
                &mut self.recording,
                Stage::Prefix,
                self.layer,
                [&self.prefix.kernels[0], &self.prefix.kernels[1]],
                raw,
            )?;
        }
        Ok((words, completion.dispatch_elapsed_ns))
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        self.states.begin_residual(self.layer, first)?;
        let mut raw = None;
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
                let (original, output) = residual_roots(
                    first,
                    self.roots.prefix[rank][0],
                    self.roots.mlp[rank][0],
                    self.roots.final_hidden[rank],
                );
                pointers.push(original.pointer(128, 0, 8192, Access::Read));
                pointers.push(output.pointer(144, 0, 8192, Access::Write));
                commands.push(Dispatch {
                    kernel: residual_kernel(self.original, self.projection, rank)?,
                    bytes: consumer_bytes(),
                    workgroup: [64, 1, 1],
                    grid: [4096, 1, 1],
                    pointers,
                    timeout_ms: self.timeout_ms,
                });
            }
            // SAFETY: both partial producers completed before either residual
            // consumer; both consumers finish before shared scratch is reused.
            let elapsed = if self.recording.is_some() {
                let observations: [Raw; 2] = unsafe {
                    self.group
                        .dispatch_round_with_raw_timestamps_unchecked(commands)?
                }
                .try_into()
                .map_err(|_| "prefix decode raw residual pair")?;
                let elapsed = observations
                    .iter()
                    .map(Raw::host_elapsed_ns)
                    .collect::<Vec<_>>();
                raw = Some(observations);
                elapsed
            } else {
                unsafe { self.group.dispatch_round_unchecked(commands)? }
            };
            elapsed
                .try_into()
                .map_err(|_| "prefix decode residual pair".into())
        })();
        self.states
            .finish_residual(self.layer, first, result.clone().map(|_| ()))?;
        let result = result?;
        if let Some(raw) = raw {
            record_pair(
                &mut self.recording,
                if first {
                    Stage::PostAttentionResidual
                } else {
                    Stage::PostMlpResidual
                },
                self.layer,
                [
                    self.original.kernel(0, ResidentKind::Residual)?,
                    self.original.kernel(1, ResidentKind::Residual)?,
                ],
                raw,
            )?;
        }
        Ok(result)
    }
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])> {
        let mut raw = None;
        let completion = (|| {
            let [left, right] = self.states.take_mlp(self.layer)?;
            // SAFETY: unchanged typed548 dispatch and exact original model roots.
            let commands = [
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
            ];
            unsafe {
                if self.recording.is_some() {
                    self.group
                        .dispatch_wave_mlp_tiles_round_with_raw_timestamps_unchecked_v1(commands)
                        .map(|(completion, observations)| {
                            raw = Some(observations);
                            completion
                        })
                } else {
                    self.group.dispatch_wave_mlp_tiles_round_v2(commands)
                }
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
        let completion = completion?;
        if let Some(raw) = raw {
            record_pair(
                &mut self.recording,
                Stage::Mlp,
                self.layer,
                [&self.mlp.kernels[0], &self.mlp.kernels[1]],
                raw,
            )?;
        }
        Ok((words, completion.dispatch_elapsed_ns))
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
    unsafe {
        execute_selected(
            group, states, original, prefix, mlp, roots, layer, timeout_ms, None,
        )
    }
}

/// # Safety
/// Same paired producer/consumer and owner obligations as execute. The optional
/// pair belongs to this sealed group and authenticates the separate arithmetic.
pub(crate) unsafe fn execute_selected(
    group: &mut Group,
    states: &mut Roster,
    original: &LoadedResidentArtifacts,
    prefix: &LoadedPrefix,
    mlp: &LoadedKernels,
    roots: &LayerBindings,
    layer: usize,
    timeout_ms: u32,
    projection: Option<&LoadedProjection>,
) -> Result<Completion> {
    coordinate(&mut Native {
        group,
        states,
        original,
        projection,
        prefix,
        mlp,
        roots,
        layer,
        timeout_ms,
        recording: None,
    })
}

/// # Safety
/// Same exact owner/image/ABI obligations as execute, on a fresh raw-enabled
/// group. Every packet in this forward must use raw dispatch; no fallback.
pub(crate) unsafe fn execute_recorded(
    group: &mut Group,
    states: &mut Roster,
    original: &LoadedResidentArtifacts,
    prefix: &LoadedPrefix,
    mlp: &LoadedKernels,
    roots: &LayerBindings,
    layer: usize,
    timeout_ms: u32,
    recorder: &mut Recorder,
    generation: u64,
    position: u32,
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
        projection: None,
        recording: Some((recorder, generation, position)),
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "prefix_tiles_decode_v6/projection_tests.rs"]
mod projection_tests;
