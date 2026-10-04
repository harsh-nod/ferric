//! Closed four-forward all36-layer Prefix284 + MLP548 owner, not the layer0 route.
use super::{Active, AllocationProfile, ForwardInput, NativeOwner, Phase, Result};
use crate::finite_setup_wire_v1::Scope;
use crate::forward_sequence::Backend as OriginalBackend;
use crate::native_prefix_device_recorder_v1::Recorder;
use crate::resident_artifacts::ResidentKind;
use crate::resident_layer::prefix_tiles_decode_v6 as layer;
use crate::resident_layer::prefix_tiles_v6::projection_residual;
use crate::state_roster::prefix_tiles_decode_v6::{COUNTS, Roster};
use crate::tail_artifacts::TailKind;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    TeacherForced([u32; 4]),
    Autoregressive { first: u32 },
}
pub(crate) struct Profile {
    scope: Scope,
    registration: [u8; 32],
    prefix: [u8; 32],
    mlp: [u8; 32],
    mode: Mode,
    timeout_ms: u32,
    sha256: [u8; 32],
    projection: Option<[u8; 32]>,
}
impl Profile {
    pub(crate) fn new(
        scope: &Scope,
        registration: [u8; 32],
        prefix: [u8; 32],
        mlp: [u8; 32],
        mode: Mode,
        timeout_ms: u32,
        devices: [u64; 2],
    ) -> Result<Self> {
        let (tag, tokens) = match mode {
            Mode::TeacherForced(tokens) => (0, tokens),
            Mode::Autoregressive { first } => (1, [first, 0, 0, 0]),
        };
        if scope.bundle_id == [0; 32]
            || scope.model_id == [0; 32]
            || scope.session == [0; 32]
            || scope.pool_identity == 0
            || scope.child_identity == 0
            || registration == [0; 32]
            || prefix == [0; 32]
            || mlp == [0; 32]
            || devices[0] == 0
            || devices[1] == 0
            || devices[0] == devices[1]
            || !(1..=10000).contains(&timeout_ms)
            || tokens.iter().any(|t| *t >= 151936)
        {
            return Err("prefix decode profile scope/images/mode/deadline".into());
        }
        let mut h = Sha256::new();
        h.update(b"ferric-prefix284-mlp548-four-decode-v6\0");
        h.update(scope.bundle_id);
        h.update(scope.model_id);
        h.update(scope.session);
        h.update(scope.pool_identity.to_le_bytes());
        h.update(scope.group_id.to_le_bytes());
        h.update(scope.child_identity.to_le_bytes());
        h.update(registration);
        h.update(prefix);
        h.update(mlp);
        h.update(timeout_ms.to_le_bytes());
        h.update([tag]);
        for device in devices {
            h.update(device.to_le_bytes());
        }
        for token in tokens {
            h.update(token.to_le_bytes());
        }
        Ok(Self {
            scope: scope.clone(),
            registration,
            prefix,
            mlp,
            mode,
            timeout_ms,
            sha256: h.finalize().into(),
            projection: None,
        })
    }
    pub(crate) fn with_projection(mut self, image: [u8; 32]) -> Result<Self> {
        if self.projection.is_some() || !matches!(self.mode, Mode::TeacherForced(_)) {
            return Err("projection decode requires a fresh teacher-forced profile".into());
        }
        self.sha256 =
            crate::finite_projection_residual_decode_wire_v1::profile_sha256(self.sha256, image)
                .map_err(|error| error.to_string())?;
        self.projection = Some(image);
        Ok(self)
    }
    pub(crate) fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}
pub(crate) struct Completion {
    pub(crate) profile_sha256: [u8; 32],
    pub(crate) generation: u64,
    pub(crate) position: u32,
    pub(crate) input_token: u32,
    pub(crate) output_token: u32,
    pub(crate) embedding_ns: [u64; 2],
    pub(crate) layers: Vec<layer::Completion>,
    pub(crate) tail_ns: [u64; 3],
}
pub(crate) struct Run {
    pub(crate) completion: Completion,
    pub(crate) layer_hidden: Vec<Vec<u8>>,
    pub(crate) final_normalized: Vec<u8>,
    pub(crate) logits: Vec<u8>,
}
pub(crate) struct Owner {
    owner: NativeOwner,
    states: Roster,
    sequence: sequence::Sequence,
    profile: Profile,
    projection: Option<projection_residual::Loaded>,
}
fn finish_projection_load<T>(
    result: Result<T>,
    phase: &mut Phase,
    poison: impl FnOnce(),
) -> Result<T> {
    if result.is_err() {
        *phase = Phase::Terminal;
        poison();
    }
    result
}
fn close_ready(exhausted: bool, between: bool, completed: u64) -> Result<()> {
    if !exhausted || !between || completed != 4 {
        return Err("prefix decode Close before four complete forwards".into());
    }
    Ok(())
}
impl Owner {
    pub(crate) fn poison_device_recording(&mut self) {
        self.owner.catalog.phase = Phase::Terminal;
        self.states.poison();
        self.sequence.poison();
    }
    /// Raw observations only; every call takes fresh runtime currentness fences.
    pub(crate) fn sample_device_clocks(
        &mut self,
        completed: u64,
    ) -> Result<Vec<fe2o3_kfd::Gfx950EngineeringPeerClockObservationV1>> {
        let result = (|| {
            if self.owner.catalog.phase != Phase::LayersSealed
                || !self.states.between()
                || completed > 4
                || self.states.completed() != completed
            {
                return Err("prefix clock sample requires a matching idle owner".into());
            }
            self.owner.catalog.backend.observe_clock_correlation_v1()
        })();
        if result.is_err() {
            self.poison_device_recording();
        }
        result
    }
    pub(crate) fn bind_device_images(&mut self, recorder: &mut Recorder) -> Result<()> {
        let result = (|| {
            if self.owner.catalog.phase != Phase::LayersSealed
                || self.projection.is_some()
                || !self.sequence.pristine()
                || !self.states.between()
                || self.states.completed() != 0
            {
                return Err("prefix device images require pristine sealed owner".into());
            }
            let prefix = self
                .owner
                .prefix_artifacts
                .as_ref()
                .ok_or("prefix device image missing")?;
            let mlp = self
                .owner
                .tiles_artifacts
                .as_ref()
                .ok_or("MLP device image missing")?;
            let original = self
                .owner
                .artifacts
                .as_ref()
                .ok_or("residual device image missing")?;
            let tail = self
                .owner
                .tail_artifacts
                .as_ref()
                .ok_or("tail device image missing")?;
            recorder
                .bind_images([
                    &prefix.kernels[0],
                    &mlp.kernels[0],
                    original.kernel(0, ResidentKind::Residual)?,
                    tail.kernel(TailKind::Embedding),
                    tail.kernel(TailKind::Copy),
                ])
                .map_err(|error| error.to_string())
        })();
        if result.is_err() {
            self.owner.catalog.phase = Phase::Terminal;
            self.states.poison();
            self.sequence.poison();
        }
        result
    }
    pub(crate) fn host_observation(
        &mut self,
    ) -> Result<fe2o3_kfd::Gfx950EngineeringPeerHostObservationV1> {
        self.owner.catalog.backend.host_observation_v1()
    }
    /// # Safety
    /// Actual source/model/image provenance and TP2 runtime premises require
    /// independent review. A setup/profile digest is not a proof issuer.
    pub(crate) unsafe fn from_sealed(owner: NativeOwner, profile: Profile) -> Result<Self> {
        unsafe { Self::from_sealed_selected(owner, profile, None) }
    }
    /// # Safety
    /// In addition to the ordinary owner premises, the actual candidate is
    /// independently reviewed for both O and Down residuals of every layer.
    pub(crate) unsafe fn from_sealed_projection(
        owner: NativeOwner,
        profile: Profile,
        image: projection_residual::Image,
    ) -> Result<Self> {
        unsafe { Self::from_sealed_selected(owner, profile, Some(image)) }
    }
    unsafe fn from_sealed_selected(
        mut owner: NativeOwner,
        profile: Profile,
        image: Option<projection_residual::Image>,
    ) -> Result<Self> {
        if image.as_ref().map(projection_residual::Image::sha256) != profile.projection {
            return Err("projection decode image/profile mismatch".into());
        }
        let s = &owner.catalog.scope;
        if owner.catalog.profile != super::super::ExecutionProfile::PrefixTilesDecodeV6
            || owner.catalog.phase != Phase::LayersSealed
            || owner.layer_bindings.len() != 36
            || owner.sealed_states.is_some()
            || owner.tiles_states.is_some()
            || owner.prefix_states.is_some()
            || owner.prefix_decode_states.is_none()
            || owner.artifacts.is_none()
            || owner.tail_manifest.is_none()
            || owner.tail_head.is_none()
            || owner.tail_artifacts.is_none()
            || owner.tail_bindings.is_none()
            || owner.tiles_image.is_some()
            || owner.prefix_image.is_some()
            || owner
                .tiles_artifacts
                .as_ref()
                .is_none_or(|a| a.sha256 != profile.mlp)
            || owner
                .prefix_artifacts
                .as_ref()
                .is_none_or(|a| a.sha256 != profile.prefix)
            || owner.registration_sha256() != profile.registration
            || s.bundle_id != profile.scope.bundle_id
            || s.model_id != profile.scope.model_id
            || s.session != profile.scope.session
            || s.pool_identity != profile.scope.pool_identity
            || s.group_id != profile.scope.group_id
            || s.child_identity != profile.scope.child_identity
        {
            return Err("prefix decode sealed scope/profile/roster/images/tail".into());
        }
        let mut states = owner
            .prefix_decode_states
            .take()
            .ok_or("prefix decode roster missing")?;
        let projection = match image {
            Some(image) => Some(finish_projection_load(
                projection_residual::load(&mut owner.catalog.backend, image),
                &mut owner.catalog.phase,
                || states.poison(),
            )?),
            None => None,
        };
        let sequence = sequence::Sequence::new(&profile);
        Ok(Self {
            owner,
            states,
            sequence,
            profile,
            projection,
        })
    }
    pub(crate) fn run(&mut self, profile: [u8; 32], input: &ForwardInput) -> Result<Run> {
        self.run_with_recorder(profile, input, None)
    }
    pub(crate) fn run_recorded(
        &mut self,
        profile: [u8; 32],
        input: &ForwardInput,
        recorder: &mut Recorder,
    ) -> Result<Run> {
        self.run_with_recorder(profile, input, Some(recorder))
    }
    fn run_with_recorder(
        &mut self,
        profile: [u8; 32],
        input: &ForwardInput,
        recorder: Option<&mut Recorder>,
    ) -> Result<Run> {
        if recorder.is_some() && self.projection.is_some() {
            self.poison_device_recording();
            return Err("projection decode does not use legacy device reports".into());
        }
        let mut active = Native {
            base: Active {
                owner: &mut self.owner,
                timeout_ms: self.profile.timeout_ms,
                layer_hidden: Vec::with_capacity(36),
                final_normalized: Vec::new(),
                logits: Vec::new(),
                capture: None,
                captured_layer0: None,
                reuse: None,
            },
            states: &mut self.states,
            projection: self.projection.as_ref(),
            generation: input.generation,
            position: input.cache_metadata[0],
            recorder,
        };
        let completion = self.sequence.run(&mut active, profile, input)?;
        Ok(Run {
            completion,
            layer_hidden: active.base.layer_hidden,
            final_normalized: active.base.final_normalized,
            logits: active.base.logits,
        })
    }
    pub(crate) fn close(mut self) -> Result<()> {
        let result = (|| {
            close_ready(
                self.sequence.exhausted(),
                self.states.between(),
                self.states.completed(),
            )?;
            if self
                .owner
                .catalog
                .backend
                .preflight_additional_allocations_v1(&[0, 0])?
                != COUNTS
            {
                return Err("prefix decode Close allocation census".into());
            }
            self.owner.close_setup()
        })();
        if result.is_err() {
            self.owner.catalog.phase = Phase::Terminal;
            self.states.poison();
        }
        result
    }
}
struct Native<'a> {
    base: Active<'a>,
    states: &'a mut Roster,
    projection: Option<&'a projection_residual::Loaded>,
    generation: u64,
    position: u32,
    recorder: Option<&'a mut Recorder>,
}
impl sequence::Backend for Native<'_> {
    fn metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.base
            .upload_metadata_for(input, AllocationProfile::PrefixTilesDecodeV6)
    }
    fn embedding(&mut self, token: u32) -> Result<[u64; 2]> {
        if let Some(recorder) = self.recorder.as_deref_mut() {
            // SAFETY: the unchanged Sequence admitted this token/position;
            // catalog sealing, error poisoning and singleton dependencies remain.
            unsafe {
                self.base.owner.tail_embedding_with_recording(
                    token,
                    self.base.timeout_ms,
                    Some((recorder, self.generation, self.position)),
                )
            }
        } else {
            self.base.embedding(token)
        }
    }
    fn begin(&mut self, input: &ForwardInput) -> Result<()> {
        let owner = &mut self.base.owner;
        self.states.begin(
            &mut owner.catalog.backend,
            input.registration,
            owner.catalog.scope.model_id,
            input.generation,
            input.cache_metadata[0],
        )
    }
    fn layer(&mut self, index: usize) -> Result<layer::Completion> {
        let owner = &mut self.base.owner;
        let roots = owner
            .layer_bindings
            .get(index)
            .ok_or("prefix decode layer bounds")?;
        let hidden = roots.final_hidden;
        // SAFETY: the constructor sealed this distinct profile/images/typed roster;
        // sequence and roster enforce all36 layers before tail and commitment.
        let result = unsafe {
            if let Some(recorder) = self.recorder.as_deref_mut() {
                layer::execute_recorded(
                    &mut owner.catalog.backend,
                    self.states,
                    owner
                        .artifacts
                        .as_ref()
                        .ok_or("prefix decode residual image")?,
                    owner
                        .prefix_artifacts
                        .as_ref()
                        .ok_or("prefix decode284 image")?,
                    owner
                        .tiles_artifacts
                        .as_ref()
                        .ok_or("prefix decode548 image")?,
                    roots,
                    index,
                    self.base.timeout_ms,
                    recorder,
                    self.generation,
                    self.position,
                )
            } else {
                layer::execute_selected(
                    &mut owner.catalog.backend,
                    self.states,
                    owner
                        .artifacts
                        .as_ref()
                        .ok_or("prefix decode residual image")?,
                    owner
                        .prefix_artifacts
                        .as_ref()
                        .ok_or("prefix decode284 image")?,
                    owner
                        .tiles_artifacts
                        .as_ref()
                        .ok_or("prefix decode548 image")?,
                    roots,
                    index,
                    self.base.timeout_ms,
                    self.projection,
                )
            }
        }?;
        self.base.retain_layer_hidden(index, hidden)?;
        Ok(result)
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        if let Some(recorder) = self.recorder.as_deref_mut() {
            // SAFETY: unchanged Sequence completed all36 paired layers and
            // consumers. Reuse the ordinary finite/readback/argmax checks below.
            let result = unsafe {
                self.base.owner.tail_finish_with_recording(
                    self.base.timeout_ms,
                    Some((recorder, self.generation, self.position)),
                )
            }?;
            self.base.retain_tail_result(result)
        } else {
            self.base.tail()
        }
    }
    fn fence(&mut self) -> Result<()> {
        self.base
            .idle_fence_for(AllocationProfile::PrefixTilesDecodeV6)
    }
    fn commit(&mut self) -> Result<()> {
        self.states
            .commit(&mut self.base.owner.catalog.backend, self.generation)
    }
    fn poison(&mut self) {
        self.states.poison();
        self.base.owner.catalog.phase = Phase::Terminal;
    }
}

#[path = "native_prefix_tiles_decode_v6/sequence.rs"]
mod sequence;
#[cfg(test)]
#[path = "native_prefix_tiles_decode_v6/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_prefix_tiles_decode_v6/projection_tests.rs"]
mod projection_tests;
