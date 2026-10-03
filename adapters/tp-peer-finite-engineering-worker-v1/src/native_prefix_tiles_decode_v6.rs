//! Closed four-forward all36-layer Prefix284 + MLP548 owner, not the layer0 route.
use super::{Active, AllocationProfile, ForwardInput, NativeOwner, Phase, Result};
use crate::finite_setup_wire_v1::Scope;
use crate::forward_sequence::Backend as OriginalBackend;
use crate::resident_layer::prefix_tiles_decode_v6 as layer;
use crate::state_roster::prefix_tiles_decode_v6::{COUNTS, Roster};
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
}
fn close_ready(exhausted: bool, between: bool, completed: u64) -> Result<()> {
    if !exhausted || !between || completed != 4 {
        return Err("prefix decode Close before four complete forwards".into());
    }
    Ok(())
}
impl Owner {
    pub(crate) fn host_observation(
        &mut self,
    ) -> Result<fe2o3_kfd::Gfx950EngineeringPeerHostObservationV1> {
        self.owner.catalog.backend.host_observation_v1()
    }
    /// # Safety
    /// Actual source/model/image provenance and TP2 runtime premises require
    /// independent review. A setup/profile digest is not a proof issuer.
    pub(crate) unsafe fn from_sealed(mut owner: NativeOwner, profile: Profile) -> Result<Self> {
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
        let states = owner
            .prefix_decode_states
            .take()
            .ok_or("prefix decode roster missing")?;
        let sequence = sequence::Sequence::new(&profile);
        Ok(Self {
            owner,
            states,
            sequence,
            profile,
        })
    }
    pub(crate) fn run(&mut self, profile: [u8; 32], input: &ForwardInput) -> Result<Run> {
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
    generation: u64,
}
impl sequence::Backend for Native<'_> {
    fn metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.base
            .upload_metadata_for(input, AllocationProfile::PrefixTilesDecodeV6)
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
        let roots = owner
            .layer_bindings
            .get(index)
            .ok_or("prefix decode layer bounds")?;
        let hidden = roots.final_hidden;
        // SAFETY: the constructor sealed this distinct profile/images/typed roster;
        // sequence and roster enforce all36 layers before tail and commitment.
        let result = unsafe {
            layer::execute(
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
            )
        }?;
        self.base.retain_layer_hidden(index, hidden)?;
        Ok(result)
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.base.tail()
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
