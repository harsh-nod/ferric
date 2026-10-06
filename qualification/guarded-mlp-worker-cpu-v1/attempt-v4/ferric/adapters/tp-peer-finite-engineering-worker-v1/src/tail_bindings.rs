//! Fixed one-token ordinary tail. Root owns forward/layer/state sequencing.

use crate::native_prefix_device_recorder_v1::Recorder;
use crate::prefix_decode_device_observation_v1::Stage;
use crate::tail_artifacts::{LoadedTailArtifacts, TailKind};
use crate::tail_head::{HEAD_BYTES, HeadTranspose};
use fe2o3_kfd::{
    Gfx950EngineeringPeerBufferV1 as Buffer, Gfx950EngineeringPeerDispatchV1 as Dispatch,
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerPointerV1 as Pointer,
    Gfx950EngineeringRawTimestampObservationV1 as Raw, engineering_wire::BufferAccessV1 as Access,
};

type Result<T> = std::result::Result<T, String>;
const VOCAB: u32 = 151_936;
const HIDDEN: u64 = 4096;
const EPSILON_BITS: u32 = 897_988_541;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Root {
    Token,
    Embedding,
    Hidden0,
    Hidden1,
    FinalNorm,
    Empty,
    Normalized,
    Head,
    Logits,
    Choice,
}
impl Root {
    const ALL: [Self; 10] = [
        Self::Token,
        Self::Embedding,
        Self::Hidden0,
        Self::Hidden1,
        Self::FinalNorm,
        Self::Empty,
        Self::Normalized,
        Self::Head,
        Self::Logits,
        Self::Choice,
    ];
    const fn index(self) -> usize {
        self as usize
    }
    const fn bytes(self) -> u64 {
        match self {
            Self::Token | Self::Choice => 16 * 4,
            Self::Embedding | Self::Head => HEAD_BYTES,
            Self::Hidden0 | Self::Hidden1 | Self::FinalNorm | Self::Normalized => HIDDEN * 2,
            Self::Empty => 2,
            Self::Logits => 16 * VOCAB as u64 * 2,
        }
    }
    const fn rank(self) -> usize {
        if matches!(self, Self::Hidden1) { 1 } else { 0 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slice {
    root: Root,
    offset: u32,
    elements: u64,
    width: u64,
    access: Access,
}
struct Plan {
    bytes: Vec<u8>,
    slices: Vec<Slice>,
}
fn validate_roots<B: Copy + Eq>(roots: &[B; 10], facts: impl Fn(B) -> (usize, u64)) -> Result<()> {
    for root in Root::ALL {
        let buffer = roots[root.index()];
        if facts(buffer) != (root.rank(), root.bytes()) || roots[..root.index()].contains(&buffer) {
            return Err("tail native root extent, owner or alias mismatch".into());
        }
    }
    Ok(())
}
fn plan(kind: TailKind) -> Plan {
    use Root as R;
    let (slices, scalars): (Vec<(Root, u64, u64, Access)>, &[u32]) = match kind {
        TailKind::Embedding => (
            vec![
                (R::Token, 1, 4, Access::Read),
                (R::Embedding, 622_329_856, 2, Access::Read),
                (R::Hidden0, HIDDEN, 2, Access::Write),
            ],
            &[1],
        ),
        TailKind::Copy => (
            vec![
                (R::Hidden0, HIDDEN, 2, Access::Read),
                (R::Hidden1, HIDDEN, 2, Access::Write),
            ],
            &[1],
        ),
        TailKind::FinalNorm => (
            vec![
                (R::Hidden0, HIDDEN, 2, Access::Read),
                (R::Empty, 0, 2, Access::Read),
                (R::FinalNorm, HIDDEN, 2, Access::Read),
                (R::Empty, 0, 2, Access::Write),
                (R::Normalized, HIDDEN, 2, Access::Write),
            ],
            &[1, 4096, EPSILON_BITS, 0],
        ),
        TailKind::Head => (
            vec![
                (R::Normalized, HIDDEN, 2, Access::Read),
                (R::Head, 622_329_856, 2, Access::Read),
                (R::Logits, VOCAB as u64, 2, Access::Write),
            ],
            &[1, VOCAB, 4096, 2, 6],
        ),
        TailKind::Argmax => (
            vec![
                (R::Logits, VOCAB as u64, 2, Access::Read),
                (R::Choice, 1, 4, Access::Write),
            ],
            &[1],
        ),
    };
    // Pointers are zero until Group's checked fixup; hidden bytes/padding zero.
    let mut bytes = vec![0; kind.explicit_bytes() as usize + 256];
    for (index, (_, elements, _, _)) in slices.iter().enumerate() {
        bytes[index * 16 + 8..index * 16 + 16].copy_from_slice(&elements.to_le_bytes());
    }
    for (index, scalar) in scalars.iter().enumerate() {
        let offset = slices.len() * 16 + index * 4;
        bytes[offset..offset + 4].copy_from_slice(&scalar.to_le_bytes());
    }
    Plan {
        bytes,
        slices: slices
            .into_iter()
            .enumerate()
            .map(|(index, (root, elements, width, access))| Slice {
                root,
                offset: index as u32 * 16,
                elements,
                width,
                access,
            })
            .collect(),
    }
}

/// Actual source/catalog tokens in closed semantic order. The separate completed
/// head type prevents original NxK storage from satisfying the MFMA binding.
pub(super) struct TailBindings {
    roots: [Buffer; 10],
}
impl TailBindings {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        token: Buffer,
        embedding: Buffer,
        hidden: [Buffer; 2],
        final_norm: Buffer,
        empty: Buffer,
        normalized: Buffer,
        head: &HeadTranspose,
        logits: Buffer,
        choice: Buffer,
    ) -> Result<Self> {
        let result = Self {
            roots: [
                token,
                embedding,
                hidden[0],
                hidden[1],
                final_norm,
                empty,
                normalized,
                head.buffer()?,
                logits,
                choice,
            ],
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<()> {
        validate_roots(&self.roots, |buffer| (buffer.owner_rank(), buffer.bytes()))
    }
    fn pointers(&self, plan: &Plan) -> Vec<Pointer> {
        plan.slices
            .iter()
            .map(|slice| {
                self.roots[slice.root.index()].pointer(
                    slice.offset,
                    0,
                    slice.elements * slice.width,
                    slice.access,
                )
            })
            .collect()
    }
}

trait Operations {
    fn write_token(&mut self, token: u32) -> Result<()>;
    fn dispatch(&mut self, kind: TailKind) -> Result<u64>;
    fn read_choice(&mut self) -> Result<Vec<u8>>;
}
struct Native<'a> {
    group: &'a mut Group,
    artifacts: &'a LoadedTailArtifacts,
    bindings: &'a TailBindings,
    timeout_ms: u32,
    recording: Option<(&'a mut Recorder, u64, u32)>,
}
fn device_stage(kind: TailKind) -> Stage {
    match kind {
        TailKind::Embedding => Stage::Embedding,
        TailKind::Copy => Stage::Copy,
        TailKind::FinalNorm => Stage::FinalNorm,
        TailKind::Head => Stage::Head,
        TailKind::Argmax => Stage::Argmax,
    }
}
impl Operations for Native<'_> {
    fn write_token(&mut self, token: u32) -> Result<()> {
        self.group.write(
            self.bindings.roots[Root::Token.index()],
            0,
            &token.to_le_bytes(),
        )
    }
    fn dispatch(&mut self, kind: TailKind) -> Result<u64> {
        let plan = plan(kind);
        let pointers = self.bindings.pointers(&plan);
        if let Some((recorder, generation, position)) = self.recording.as_mut() {
            let kernel = self.artifacts.kernel(kind);
            // SAFETY: identical singleton arguments/roots/deadline; each round
            // finishes before the next dependent singleton can be published.
            let [raw]: [Raw; 1] = unsafe {
                self.group
                    .dispatch_round_with_raw_timestamps_unchecked(vec![Dispatch {
                        kernel,
                        bytes: plan.bytes,
                        workgroup: [64, 1, 1],
                        grid: kind.grid(),
                        pointers,
                        timeout_ms: self.timeout_ms,
                    }])
            }?
            .try_into()
            .map_err(|_| "tail raw singleton census")?;
            recorder
                .record(
                    *generation,
                    *position,
                    device_stage(kind),
                    None,
                    kernel,
                    &raw,
                )
                .map_err(|error| error.to_string())?;
            return Ok(raw.host_elapsed_ns());
        }
        // SAFETY: only the exact pinned entry and fixed bounds/roots are used.
        // Outer unsafe entry requires the owner to enforce phase, currentness
        // and no intervening writes; Group retains and checks every token.
        unsafe {
            self.group.dispatch_unchecked(
                self.artifacts.kernel(kind),
                plan.bytes,
                [64, 1, 1],
                kind.grid(),
                &pointers,
                self.timeout_ms,
            )
        }
    }
    fn read_choice(&mut self) -> Result<Vec<u8>> {
        self.group
            .read(self.bindings.roots[Root::Choice.index()], 0, 4)
    }
}

fn check_call(
    bindings: &TailBindings,
    artifacts: &LoadedTailArtifacts,
    timeout_ms: u32,
) -> Result<()> {
    bindings.validate()?;
    if timeout_ms == 0 || timeout_ms > 600_000 {
        return Err("tail timeout bound".into());
    }
    for kind in TailKind::ALL {
        if artifacts.kernel(kind).rank() != kind.rank() {
            return Err("tail kernel rank mismatch".into());
        }
    }
    Ok(())
}
fn begin<O: Operations>(ops: &mut O, token: u32) -> Result<[u64; 2]> {
    if token >= VOCAB {
        return Err("tail input token outside vocabulary".into());
    }
    ops.write_token(token)?;
    Ok([
        ops.dispatch(TailKind::Embedding)?,
        ops.dispatch(TailKind::Copy)?,
    ])
}
fn finish<O: Operations>(ops: &mut O) -> Result<TailResult> {
    let timing_ns = [
        ops.dispatch(TailKind::FinalNorm)?,
        ops.dispatch(TailKind::Head)?,
        ops.dispatch(TailKind::Argmax)?,
    ];
    let bytes: [u8; 4] = ops
        .read_choice()?
        .try_into()
        .map_err(|_| "tail choice byte extent")?;
    let token = u32::from_le_bytes(bytes);
    if token >= VOCAB {
        return Err("tail output token outside vocabulary".into());
    }
    Ok(TailResult { token, timing_ns })
}

/// Still-retained internal observation, not output publication or owner closure.
/// Durations are existing queue host measurements, not device-only kernel time.
pub(super) struct TailResult {
    pub(super) token: u32,
    pub(super) timing_ns: [u64; 3],
}

/// Initialize both rank hidden rows. No other source/state transitions occur.
/// # Safety
/// Root owner must have authenticated source/program/manifest and all actual
/// bindings, made setup immutable, and admitted this exact forward transition.
/// On any error it must enter Terminal and quarantine rather than retry/free.
pub(super) unsafe fn begin_embedding(
    group: &mut Group,
    artifacts: &LoadedTailArtifacts,
    bindings: &TailBindings,
    token: u32,
    timeout_ms: u32,
) -> Result<[u64; 2]> {
    check_call(bindings, artifacts, timeout_ms)?;
    begin(
        &mut Native {
            group,
            artifacts,
            bindings,
            timeout_ms,
            recording: None,
        },
        token,
    )
}

/// Execute exactly finalnorm -> MFMA head -> argmax after all 36 layers.
/// # Safety
/// Same owner contract as `begin_embedding`; all layer/TP completion observations
/// must have succeeded and Hidden0 must be this forward's final residual. The
/// returned token stays internal until the owner publication/close gate accepts.
pub(super) unsafe fn finish_tail(
    group: &mut Group,
    artifacts: &LoadedTailArtifacts,
    bindings: &TailBindings,
    timeout_ms: u32,
) -> Result<TailResult> {
    check_call(bindings, artifacts, timeout_ms)?;
    finish(&mut Native {
        group,
        artifacts,
        bindings,
        timeout_ms,
        recording: None,
    })
}

fn check_recording_position(generation: u64, position: u32) -> Result<()> {
    if position >= 4 || generation != u64::from(position) + 1 {
        return Err("tail raw recording generation".into());
    }
    Ok(())
}
/// # Safety
/// Same owner/phase contract as begin_embedding; the group is freshly raw-enabled.
pub(super) unsafe fn begin_embedding_recorded(
    group: &mut Group,
    artifacts: &LoadedTailArtifacts,
    bindings: &TailBindings,
    token: u32,
    timeout_ms: u32,
    recorder: &mut Recorder,
    generation: u64,
    position: u32,
) -> Result<[u64; 2]> {
    check_recording_position(generation, position)?;
    check_call(bindings, artifacts, timeout_ms)?;
    begin(
        &mut Native {
            group,
            artifacts,
            bindings,
            timeout_ms,
            recording: Some((recorder, generation, position)),
        },
        token,
    )
}
/// # Safety
/// Same completed-layer contract as finish_tail; no partial or retried capture.
pub(super) unsafe fn finish_tail_recorded(
    group: &mut Group,
    artifacts: &LoadedTailArtifacts,
    bindings: &TailBindings,
    timeout_ms: u32,
    recorder: &mut Recorder,
    generation: u64,
    position: u32,
) -> Result<TailResult> {
    check_recording_position(generation, position)?;
    check_call(bindings, artifacts, timeout_ms)?;
    finish(&mut Native {
        group,
        artifacts,
        bindings,
        timeout_ms,
        recording: Some((recorder, generation, position)),
    })
}

#[cfg(test)]
#[path = "tail_bindings_tests.rs"]
mod tests;
