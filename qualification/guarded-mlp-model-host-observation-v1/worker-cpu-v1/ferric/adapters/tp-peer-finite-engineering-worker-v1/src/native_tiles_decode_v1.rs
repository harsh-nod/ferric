//! Private four-forward all36-layer V2 backend; no wire or public CLI admission.
use super::{Active, AllocationProfile, ForwardInput, NativeOwner, Phase, Result};
use crate::finite_setup_wire_v1::Scope;
use crate::finite_tiles_decode_wire_v1::{AdmissionReceipt, KernelAdmission};
use crate::forward_sequence::Backend as OriginalBackend;
use crate::resident_layer::tiles_decode_v1 as layer;
use crate::state_roster::tiles_decode_v1::Roster;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    TeacherForced([u32; 4]),
    Autoregressive { first: u32 },
}
pub(crate) struct Profile {
    scope: Scope,
    registration: [u8; 32],
    image: [u8; 32],
    mode: Mode,
    timeout_ms: u32,
    sha256: [u8; 32],
    admission: KernelAdmission,
    configuration_attempted: bool,
    applied_admission: Option<AdmissionReceipt>,
}
impl Profile {
    pub(crate) fn new(
        scope: &Scope,
        registration: [u8; 32],
        image: [u8; 32],
        mode: Mode,
        timeout_ms: u32,
        devices: [u64; 2],
    ) -> Result<Self> {
        Self::new_with_admission(
            scope,
            registration,
            image,
            mode,
            timeout_ms,
            devices,
            KernelAdmission::Baseline,
        )
    }
    pub(crate) fn new_with_admission(
        scope: &Scope,
        registration: [u8; 32],
        image: [u8; 32],
        mode: Mode,
        timeout_ms: u32,
        devices: [u64; 2],
        admission: KernelAdmission,
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
            || registration == [0; 32]
            || image == [0; 32]
            || devices[0] == 0
            || devices[1] == 0
            || devices[0] == devices[1]
            || !(1..=10000).contains(&timeout_ms)
            || tokens.iter().any(|t| *t >= 151936)
        {
            return Err("tiles decode profile identity/mode/deadline".into());
        }
        let mut hash = Sha256::new();
        hash.update(admission.profile().domain());
        hash.update(scope.bundle_id);
        hash.update(scope.model_id);
        hash.update(scope.session);
        hash.update(scope.pool_identity.to_le_bytes());
        hash.update(scope.group_id.to_le_bytes());
        hash.update(scope.child_identity.to_le_bytes());
        hash.update(registration);
        hash.update(image);
        hash.update(timeout_ms.to_le_bytes());
        hash.update([tag]);
        for device in devices {
            hash.update(device.to_le_bytes());
        }
        for token in tokens {
            hash.update(token.to_le_bytes());
        }
        Ok(Self {
            scope: scope.clone(),
            registration,
            image,
            mode,
            timeout_ms,
            sha256: hash.finalize().into(),
            admission,
            configuration_attempted: false,
            applied_admission: None,
        })
    }
    pub(crate) fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
    // Called on the fresh group before any setup allocation. An error cannot
    // mint a configuration witness or permit a second attempt.
    pub(crate) fn configure_admission(
        &mut self,
        configure: impl FnOnce(bool, bool) -> Result<()>,
    ) -> Result<()> {
        if self.configuration_attempted {
            return Err("tiles admission configuration already attempted".into());
        }
        self.configuration_attempted = true;
        if self.admission == KernelAdmission::CachedImmutable {
            configure(true, false)?;
            self.applied_admission = self.admission.receipt();
        }
        Ok(())
    }
    pub(crate) fn applied_admission(&self) -> Result<Option<AdmissionReceipt>> {
        if !self.configuration_attempted || self.applied_admission != self.admission.receipt() {
            return Err("tiles admission configuration not applied".into());
        }
        Ok(self.applied_admission)
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
}
impl Owner {
    /// # Safety
    /// The supplied profile/image and original catalog must be authenticated and
    /// reviewed by the owning parent. This private type is not production proof.
    pub(crate) unsafe fn from_sealed(mut owner: NativeOwner, profile: Profile) -> Result<Self> {
        profile.applied_admission()?;
        let s = &owner.catalog.scope;
        if owner.catalog.profile != super::super::ExecutionProfile::TilesDecodeV1
            || owner.catalog.phase != Phase::LayersSealed
            || owner.layer_bindings.len() != 36
            || owner.sealed_states.is_some()
            || owner.tiles_states.is_none()
            || owner.artifacts.is_none()
            || owner.tail_manifest.is_none()
            || owner.tail_head.is_none()
            || owner.tail_artifacts.is_none()
            || owner.tail_bindings.is_none()
            || owner.tiles_image.is_some()
            || owner
                .tiles_artifacts
                .as_ref()
                .is_none_or(|a| a.sha256 != profile.image)
            || owner.registration_sha256() != profile.registration
            || s.bundle_id != profile.scope.bundle_id
            || s.model_id != profile.scope.model_id
            || s.session != profile.scope.session
            || s.pool_identity != profile.scope.pool_identity
            || s.group_id != profile.scope.group_id
            || s.child_identity != profile.scope.child_identity
        {
            return Err("tiles decode requires its own sealed profile/roster/image".into());
        }
        let states = owner
            .tiles_states
            .take()
            .ok_or("tiles state owner missing")?;
        let sequence = sequence::Sequence::new(&profile);
        Ok(Self {
            owner,
            states,
            sequence,
            profile,
        })
    }
    pub(crate) fn run(&mut self, profile_sha256: [u8; 32], input: &ForwardInput) -> Result<Run> {
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
            generation: input.generation,
        };
        let completion = self.sequence.run(&mut active, profile_sha256, input)?;
        Ok(Run {
            completion,
            layer_hidden: active.base.layer_hidden,
            final_normalized: active.base.final_normalized,
            logits: active.base.logits,
        })
    }
    pub(crate) fn close(mut self) -> Result<()> {
        if !self.sequence.exhausted() || !self.states.between() || self.states.completed() != 4 {
            self.owner.catalog.phase = Phase::Terminal;
            self.states.poison();
            return Err("tiles Close before four committed forwards".into());
        }
        let census = self
            .owner
            .catalog
            .backend
            .preflight_additional_allocations_v1(&[0, 0]);
        if !matches!(&census,Ok(v) if v.as_slice()==crate::state_roster::tiles_decode_v1::COUNTS) {
            self.owner.catalog.phase = Phase::Terminal;
            self.states.poison();
            return Err("tiles Close allocation census".into());
        }
        self.owner.close_setup()
    }
}
struct Native<'a> {
    base: Active<'a>,
    states: &'a mut Roster,
    generation: u64,
}
impl sequence::Backend for Native<'_> {
    fn metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.base
            .upload_metadata_for(input, AllocationProfile::TilesDecodeV1)
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
        )
    }
    fn layer(&mut self, index: usize) -> Result<layer::Completion> {
        let owner = &mut self.base.owner;
        let roots = owner.layer_bindings.get(index).ok_or("tiles layer bound")?;
        let hidden = roots.final_hidden;
        // SAFETY: constructor bound the separately selected typed roster/image;
        // the sequence and roster enforce all36 layers and bank generation.
        let result = unsafe {
            layer::execute(
                &mut owner.catalog.backend,
                self.states,
                owner
                    .artifacts
                    .as_ref()
                    .ok_or("tiles prefix/residual images")?,
                owner.tiles_artifacts.as_ref().ok_or("tiles image")?,
                roots,
                index,
                self.base.timeout_ms,
            )
        }?;
        self.base.retain_layer_hidden(index, hidden)?;
        Ok(result)
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.base.tail()
    }
    fn fence(&mut self) -> Result<()> {
        self.base.idle_fence_for(AllocationProfile::TilesDecodeV1)
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
#[path = "native_tiles_decode_v1/sequence.rs"]
mod sequence;
#[cfg(test)]
#[path = "native_tiles_decode_v1/tests.rs"]
mod tests;
