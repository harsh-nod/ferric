//! Private four-forward real-model route; no production or performance grant.
pub(crate) use super::prefix_tiles_decode_v6::Mode;
use super::{Active, AllocationProfile, ForwardInput, NativeOwner, Phase, Result};
use crate::finite_setup_wire_v1::Scope;
use crate::forward_sequence::Backend as OriginalBackend;
use crate::resident_layer::guarded_mlp_decode_v1 as layer;
use crate::resident_layer::capture_v1::{Collector, Layer0CaptureV1};
use crate::state_roster::guarded_mlp_decode_v1::Roster;

pub(crate) struct Profile {
    scope: Scope,
    registration: [u8; 32],
    prefix: [u8; 32],
    mlp: [u8; 32],
    projection: [u8; 32],
    mode: Mode,
    timeout_ms: u32,
    sha256: [u8; 32],
}
impl Profile {
    pub(crate) fn new(
        scope: &Scope,
        registration: [u8; 32],
        prefix: [u8; 32],
        mlp: [u8; 32],
        projection: [u8; 32],
        mode: Mode,
        timeout_ms: u32,
        devices: [u64; 2],
    ) -> Result<Self> {
        let (tag, tokens) = match mode {
            Mode::TeacherForced(t) => (0, t),
            Mode::Autoregressive { first } => (1, [first, 0, 0, 0]),
        };
        if scope.bundle_id == [0; 32]
            || scope.model_id == [0; 32]
            || scope.session == [0; 32]
            || scope.pool_identity == 0
            || scope.child_identity == 0
            || [registration, prefix, mlp, projection].contains(&[0; 32])
            || devices.contains(&0)
            || devices[0] == devices[1]
            || !(1..=10_000).contains(&timeout_ms)
            || tokens.iter().any(|t| *t >= 151936)
        {
            return Err("guarded model profile scope/images/mode/deadline".into());
        }
        let sha256 = crate::finite_guarded_mlp_decode_wire_v1::profile_sha256(
            scope,
            registration,
            prefix,
            mlp,
            projection,
            tag == 1,
            tokens,
            timeout_ms,
            devices,
        );
        Ok(Self {
            scope: scope.clone(),
            registration,
            prefix,
            mlp,
            projection,
            mode,
            timeout_ms,
            sha256,
        })
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
    capture_enabled: bool,
    captured_layer0: Option<Layer0CaptureV1>,
}
impl Owner {
    /// # Safety
    /// The caller authenticates the actual model/images and engineering TP2
    /// coherence premises; this constructor issues no protected worker authority.
    pub(crate) unsafe fn from_sealed(mut owner: NativeOwner, profile: Profile) -> Result<Self> {
        let s = &owner.catalog.scope;
        if owner.catalog.profile != super::super::ExecutionProfile::GuardedMlpDecodeV1
            || owner.catalog.phase != Phase::LayersSealed
            || owner.layer_bindings.len() != 36
            || owner.sealed_states.is_some()
            || owner.tiles_states.is_some()
            || owner.prefix_states.is_some()
            || owner.prefix_decode_states.is_some()
            || owner.guarded_states.is_none()
            || owner.guarded_down.is_none()
            || owner.guarded_images.is_some()
            || owner.tiles_image.is_some()
            || owner.prefix_image.is_some()
            || owner.tail_artifacts.is_none()
            || owner.tail_bindings.is_none()
            || owner.tail_head.is_none()
            || owner
                .prefix_artifacts
                .as_ref()
                .is_none_or(|v| v.sha256 != profile.prefix)
            || owner
                .tiles_artifacts
                .as_ref()
                .is_none_or(|v| v.sha256 != profile.mlp)
            || owner
                .guarded_loaded
                .as_ref()
                .is_none_or(|v| v.projection.sha256 != profile.projection)
            || owner.registration_sha256() != profile.registration
            || s.bundle_id != profile.scope.bundle_id
            || s.model_id != profile.scope.model_id
            || s.session != profile.scope.session
            || s.pool_identity != profile.scope.pool_identity
            || s.group_id != profile.scope.group_id
            || s.child_identity != profile.scope.child_identity
        {
            owner.catalog.phase = Phase::Terminal;
            return Err("guarded model sealed scope/profile/roster/images".into());
        }
        let states = owner.guarded_states.take().ok_or("guarded roster absent")?;
        let sequence = sequence::Sequence::new(&profile);
        Ok(Self {
            owner,
            states,
            sequence,
            profile,
            capture_enabled: false,
            captured_layer0: None,
        })
    }
    pub(crate) fn enable_layer0_capture(&mut self) -> Result<()> {
        if self.capture_enabled || self.captured_layer0.is_some()
            || !self.sequence.pristine() || !self.states.between() || self.states.completed() != 0 {
            self.sequence.poison();
            self.states.poison();
            self.owner.catalog.phase = Phase::Terminal;
            return Err("guarded capture must be selected once before the first forward".into());
        }
        self.capture_enabled = true;
        Ok(())
    }
    pub(crate) fn run(&mut self, profile: [u8; 32], input: &ForwardInput) -> Result<Run> {
        let capture = if self.capture_enabled && self.sequence.pristine() {
            match Collector::new(input.generation, input.cache_metadata[0], 0) {
                Ok(capture) => Some(capture),
                Err(error) => {
                    self.sequence.poison();
                    self.states.poison();
                    self.owner.catalog.phase = Phase::Terminal;
                    return Err(error);
                }
            }
        } else {
            None
        };
        let mut active = Native {
            base: Active {
                owner: &mut self.owner,
                timeout_ms: self.profile.timeout_ms,
                layer_hidden: Vec::with_capacity(36),
                final_normalized: Vec::new(),
                logits: Vec::new(),
                capture,
                captured_layer0: None,
                reuse: None,
            },
            states: &mut self.states,
            generation: input.generation,
        };
        let completion = self.sequence.run(&mut active, profile, input)?;
        if let Some(capture) = active.base.captured_layer0.take() {
            self.captured_layer0 = Some(capture);
        }
        Ok(Run {
            completion,
            layer_hidden: active.base.layer_hidden,
            final_normalized: active.base.final_normalized,
            logits: active.base.logits,
        })
    }
    pub(crate) fn close_with_capture(mut self) -> Result<Layer0CaptureV1> {
        if !self.capture_enabled {
            return Err("guarded capture was not selected".into());
        }
        let capture = self.captured_layer0.take().ok_or("guarded capture was not completed")?;
        capture.after_close(|| self.close())
    }
    pub(crate) fn close(mut self) -> Result<()> {
        let result = (|| {
            if !self.sequence.exhausted()
                || !self.states.between()
                || self.states.completed() != 4
                || self
                    .owner
                    .catalog
                    .backend
                    .preflight_additional_allocations_v1(&[0, 0])?
                    != self.states.counts()
            {
                return Err("guarded model Close before complete four-forward fence".into());
            }
            self.owner.close_setup()
        })();
        if result.is_err() {
            self.states.poison();
            self.owner.catalog.phase = Phase::Terminal;
        }
        result
    }
}
struct Native<'a> {
    base: Active<'a>,
    states: &'a mut Roster,
    generation: u64,
}
impl sequence::Backend for Native<'_> {
    fn metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.base.upload_metadata_for(
            input,
            AllocationProfile::GuardedMlpDecodeV1(self.states.counts()),
        )
    }
    fn embedding(&mut self, token: u32) -> Result<[u64; 2]> {
        self.base.embedding(token)
    }
    fn begin(&mut self, input: &ForwardInput) -> Result<()> {
        let owner = &mut self.base.owner;
        self.states.begin(
            &mut owner.catalog.backend,
            input.registration,
            owner.catalog.scope.model_id,
            input.generation,
            input.cache_metadata[0],
            self.base.timeout_ms,
        )
    }
    fn layer(&mut self, index: usize) -> Result<layer::Completion> {
        let owner = &mut self.base.owner;
        let roots = owner
            .layer_bindings
            .get(index)
            .ok_or("guarded model layer bounds")?;
        let hidden = roots.final_hidden;
        let prefix = owner.prefix_artifacts.as_ref().ok_or("guarded Prefix image missing")?;
        let mlp = owner.tiles_artifacts.as_ref().ok_or("guarded MLP image missing")?;
        let images = owner.guarded_loaded.as_ref().ok_or("guarded kernels missing")?;
        let down = owner.guarded_down.ok_or("guarded Down scratch missing")?;
        // SAFETY: sealed exact model/image/pair bindings and mixed-bank gate.
        let result = unsafe {
            match self.base.capture.as_mut() {
                Some(capture) => layer::execute_with_capture(
                    &mut owner.catalog.backend, self.states, prefix, mlp, images,
                    roots, down, index, self.base.timeout_ms, capture),
                None => layer::execute(
                    &mut owner.catalog.backend, self.states, prefix, mlp, images,
                    roots, down, index, self.base.timeout_ms),
            }
        }?;
        if let Some(capture) = self.base.capture.take() {
            self.base.captured_layer0 = Some(capture.finish()?);
        }
        self.base.retain_layer_hidden(index, hidden)?;
        Ok(result)
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.base.tail()
    }
    fn fence(&mut self) -> Result<()> {
        self.base
            .idle_fence_for(AllocationProfile::GuardedMlpDecodeV1(self.states.counts()))
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

// Reuse the unchanged bounded token/page/rotary and four-forward sequencing,
// instantiated over this route's distinct completion type and paired backend.
#[path = "native_prefix_tiles_decode_v6/sequence.rs"]
mod sequence;

#[cfg(test)]
#[path = "native_guarded_mlp_decode_v1/tests.rs"]
mod tests;
