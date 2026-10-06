//! Closed one-token layer0 route. Captures escape only after Group close.
use super::{Active, AllocationProfile, ForwardInput, NativeOwner, Phase, Result};
use crate::finite_setup_wire_v1::Scope;
use crate::forward_sequence::Backend as OriginalBackend;
use crate::resident_layer::prefix_tiles_v6 as layer;
use crate::state_roster::{prefix_tiles_v6, tiles_decode_v1};
use sha2::{Digest, Sha256};

pub(crate) struct Profile {
    scope: Scope,
    registration: [u8; 32],
    prefix: Option<[u8; 32]>,
    projection: Option<[u8; 32]>,
    mlp: [u8; 32],
    input: [u8; 32],
    timeout_ms: u32,
    sha256: [u8; 32],
}
fn input_digest(input: &ForwardInput, registration: [u8; 32]) -> Result<[u8; 32]> {
    if registration == [0; 32]
        || input.registration != registration
        || input.generation != 1
        || input.cache_metadata[0] != 0
        || input.token >= 151936
        || input
            .rotary_bits
            .iter()
            .any(|v| !f32::from_bits(*v).is_finite())
    {
        return Err("one-layer exact first token/generation/position/rotary".into());
    }
    let mut seen = [false; 144];
    for &page in &input.cache_metadata[1..] {
        let slot = seen
            .get_mut(page as usize)
            .ok_or("one-layer physical page bounds")?;
        if *slot {
            return Err("one-layer physical page alias".into());
        }
        *slot = true;
    }
    let mut h = Sha256::new();
    h.update(b"ferric-prefix-layer-input-v6\0");
    h.update(registration);
    h.update(input.generation.to_le_bytes());
    h.update(input.token.to_le_bytes());
    for v in input.cache_metadata {
        h.update(v.to_le_bytes());
    }
    for v in input.rotary_bits {
        h.update(v.to_le_bytes());
    }
    Ok(h.finalize().into())
}
impl Profile {
    pub(crate) fn new(
        scope: &Scope,
        registration: [u8; 32],
        prefix: Option<[u8; 32]>,
        mlp: [u8; 32],
        input: &ForwardInput,
        timeout_ms: u32,
        devices: [u64; 2],
    ) -> Result<Self> {
        if scope.bundle_id == [0; 32]
            || scope.model_id == [0; 32]
            || scope.session == [0; 32]
            || scope.pool_identity == 0
            || scope.child_identity == 0
            || registration == [0; 32]
            || prefix == Some([0; 32])
            || mlp == [0; 32]
            || devices[0] == 0
            || devices[1] == 0
            || devices[0] == devices[1]
            || !(1..=10000).contains(&timeout_ms)
        {
            return Err("one-layer profile scope/images/devices/deadline".into());
        }
        let input = input_digest(input, registration)?;
        let mut h = Sha256::new();
        h.update(b"ferric-prefix-layer-closed-v6\0");
        h.update(scope.bundle_id);
        h.update(scope.model_id);
        h.update(scope.session);
        h.update(scope.pool_identity.to_le_bytes());
        h.update(scope.group_id.to_le_bytes());
        h.update(scope.child_identity.to_le_bytes());
        h.update(registration);
        h.update([u8::from(prefix.is_some())]);
        h.update(prefix.unwrap_or([0; 32]));
        h.update(mlp);
        h.update(input);
        h.update(timeout_ms.to_le_bytes());
        for device in devices {
            h.update(device.to_le_bytes());
        }
        Ok(Self {
            scope: scope.clone(),
            registration,
            prefix,
            projection: None,
            mlp,
            input,
            timeout_ms,
            sha256: h.finalize().into(),
        })
    }
    pub(crate) fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
    pub(crate) fn with_projection(mut self, image: [u8; 32]) -> Result<Self> {
        if self.prefix.is_none() || self.projection.is_some() {
            return Err("projection residual requires a fresh prefix284 profile".into());
        }
        self.sha256 =
            crate::finite_projection_residual_layer_wire_v1::profile_sha256(self.sha256, image)
                .map_err(|e| e.to_string())?;
        self.projection = Some(image);
        Ok(self)
    }
    fn validate_input(&self, supplied: [u8; 32], input: &ForwardInput) -> Result<()> {
        if supplied != self.sha256 || input_digest(input, self.registration)? != self.input {
            return Err("one-layer supplied input differs from pre-open profile".into());
        }
        Ok(())
    }
    fn allocation(&self) -> AllocationProfile {
        if self.prefix.is_some() {
            AllocationProfile::PrefixTilesLayerV6
        } else {
            AllocationProfile::TilesDecodeV1
        }
    }
}
enum States {
    Baseline(tiles_decode_v1::Roster),
    Tiles(prefix_tiles_v6::Roster),
}
impl States {
    fn poison(&mut self) {
        match self {
            Self::Baseline(s) => s.poison(),
            Self::Tiles(s) => s.poison(),
        }
    }
}
pub(crate) struct ClosedRun {
    pub(crate) profile_sha256: [u8; 32],
    pub(crate) generation: u64,
    pub(crate) position: u32,
    pub(crate) input_token: u32,
    pub(crate) embedding_ns: [u64; 2],
    pub(crate) layer: layer::Run,
}
trait Backend {
    fn metadata(&mut self, input: &ForwardInput) -> Result<()>;
    fn embedding(&mut self, token: u32) -> Result<[u64; 2]>;
    fn begin(&mut self, input: &ForwardInput) -> Result<()>;
    fn layer(&mut self) -> Result<layer::Run>;
    fn fence(&mut self) -> Result<()>;
    fn poison(&mut self);
}
#[derive(Default)]
struct Gate {
    attempted: bool,
    complete: bool,
}
fn close_pending<T>(
    complete: bool,
    pending: Option<T>,
    close: impl FnOnce() -> Result<()>,
) -> Result<T> {
    if !complete {
        return Err("one-layer Close before completion".into());
    }
    let pending = pending.ok_or("one-layer capture missing")?;
    close()?;
    Ok(pending)
}
impl Gate {
    fn run(
        &mut self,
        profile: &Profile,
        sha: [u8; 32],
        input: &ForwardInput,
        b: &mut impl Backend,
    ) -> Result<ClosedRun> {
        let result = (|| {
            if self.attempted {
                return Err("one-layer run cannot repeat".into());
            }
            self.attempted = true;
            profile.validate_input(sha, input)?;
            b.metadata(input)?;
            let embedding_ns = b.embedding(input.token)?;
            b.begin(input)?;
            let layer = b.layer()?;
            if matches!(
                &layer.completion.prefix,
                layer::PrefixObservation::Tiles284(_)
            ) != profile.prefix.is_some()
            {
                return Err("one-layer completion profile mismatch".into());
            }
            b.fence()?;
            self.complete = true;
            Ok(ClosedRun {
                profile_sha256: sha,
                generation: 1,
                position: 0,
                input_token: input.token,
                embedding_ns,
                layer,
            })
        })();
        if result.is_err() {
            self.attempted = true;
            self.complete = false;
            b.poison();
        }
        result
    }
}
pub(crate) struct Owner {
    owner: NativeOwner,
    states: States,
    profile: Profile,
    gate: Gate,
    pending: Option<ClosedRun>,
    projection: Option<layer::projection_residual::Loaded>,
}
impl Owner {
    /// # Safety
    /// Source/model, actual image provenance and runtime premises must be reviewed
    /// independently. The original setup and this profile are not proof issuers.
    pub(crate) unsafe fn from_sealed(owner: NativeOwner, profile: Profile) -> Result<Self> {
        unsafe { Self::from_sealed_selected(owner, profile, None) }
    }
    /// # Safety
    /// In addition to the original premises, the caller reviewed the supplied
    /// projection materialization image for both residual stages.
    pub(crate) unsafe fn from_sealed_projection(
        owner: NativeOwner,
        profile: Profile,
        image: layer::projection_residual::Image,
    ) -> Result<Self> {
        unsafe { Self::from_sealed_selected(owner, profile, Some(image)) }
    }
    unsafe fn from_sealed_selected(
        mut owner: NativeOwner,
        profile: Profile,
        image: Option<layer::projection_residual::Image>,
    ) -> Result<Self> {
        if image.as_ref().map(|v| v.sha256()) != profile.projection
            || (image.is_some() && profile.prefix.is_none())
        {
            return Err("one-layer projection image/profile mismatch".into());
        }
        let expected = if profile.prefix.is_some() {
            super::super::ExecutionProfile::PrefixTilesLayerV6
        } else {
            super::super::ExecutionProfile::TilesDecodeV1
        };
        let s = &owner.catalog.scope;
        if owner.catalog.profile != expected
            || owner.catalog.phase != Phase::LayersSealed
            || owner.layer_bindings.len() != 36
            || owner.sealed_states.is_some()
            || owner.artifacts.is_none()
            || owner.tail_bindings.is_none()
            || owner.tail_artifacts.is_none()
            || owner.tail_head.is_none()
            || owner.tail_manifest.is_none()
            || owner.tiles_image.is_some()
            || owner.prefix_image.is_some()
            || owner
                .tiles_artifacts
                .as_ref()
                .is_none_or(|a| a.sha256 != profile.mlp)
            || owner.prefix_artifacts.as_ref().map(|a| a.sha256) != profile.prefix
            || owner.registration_sha256() != profile.registration
            || s.bundle_id != profile.scope.bundle_id
            || s.model_id != profile.scope.model_id
            || s.session != profile.scope.session
            || s.pool_identity != profile.scope.pool_identity
            || s.group_id != profile.scope.group_id
            || s.child_identity != profile.scope.child_identity
        {
            return Err("one-layer sealed catalog/profile/image mismatch".into());
        }
        let mut states = if profile.prefix.is_some() {
            if owner.tiles_states.is_some() {
                return Err("one-layer candidate has old state roster".into());
            }
            States::Tiles(
                owner
                    .prefix_states
                    .take()
                    .ok_or("prefix284 roster missing")?,
            )
        } else {
            if owner.prefix_states.is_some() {
                return Err("one-layer baseline has candidate states".into());
            }
            States::Baseline(
                owner
                    .tiles_states
                    .take()
                    .ok_or("baseline22/548 roster missing")?,
            )
        };
        let projection = match image {
            Some(image) => {
                match layer::projection_residual::load(&mut owner.catalog.backend, image) {
                    Ok(loaded) => Some(loaded),
                    Err(error) => {
                        owner.catalog.phase = Phase::Terminal;
                        states.poison();
                        return Err(error);
                    }
                }
            }
            None => None,
        };
        Ok(Self {
            owner,
            states,
            profile,
            gate: Gate::default(),
            pending: None,
            projection,
        })
    }
    /// Successful run retains all captures internally. Only close returns them.
    pub(crate) fn run(&mut self, profile_sha256: [u8; 32], input: &ForwardInput) -> Result<()> {
        let mut native = Native {
            base: Active {
                owner: &mut self.owner,
                timeout_ms: self.profile.timeout_ms,
                layer_hidden: Vec::new(),
                final_normalized: Vec::new(),
                logits: Vec::new(),
                capture: None,
                captured_layer0: None,
                reuse: None,
            },
            states: &mut self.states,
            allocation: self.profile.allocation(),
            projection: self.projection.as_ref(),
        };
        self.pending = Some(
            self.gate
                .run(&self.profile, profile_sha256, input, &mut native)?,
        );
        Ok(())
    }
    pub(crate) fn close(mut self) -> Result<ClosedRun> {
        if !self.gate.complete || self.pending.is_none() {
            self.owner.catalog.phase = Phase::Terminal;
            self.states.poison();
            return Err("one-layer Close requires one completed capture".into());
        }
        let pending = self.pending.take();
        let result = close_pending(self.gate.complete, pending, || {
            self.profile.allocation().validate(
                &self
                    .owner
                    .catalog
                    .backend
                    .preflight_additional_allocations_v1(&[0, 0])?,
            )?;
            self.owner.close_setup()?;
            Ok(())
        });
        if result.is_err() {
            self.owner.catalog.phase = Phase::Terminal;
            self.states.poison();
        }
        result
    }
}
struct Native<'a> {
    base: Active<'a>,
    states: &'a mut States,
    allocation: AllocationProfile,
    projection: Option<&'a layer::projection_residual::Loaded>,
}
impl Backend for Native<'_> {
    fn metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.base.upload_metadata_for(input, self.allocation)
    }
    fn embedding(&mut self, token: u32) -> Result<[u64; 2]> {
        self.base.embedding(token)
    }
    fn begin(&mut self, input: &ForwardInput) -> Result<()> {
        let owner = &mut self.base.owner;
        match self.states {
            States::Baseline(s) => s.begin(
                &mut owner.catalog.backend,
                input.registration,
                owner.catalog.scope.model_id,
                1,
                0,
            ),
            States::Tiles(s) => s.begin(
                &mut owner.catalog.backend,
                input.registration,
                owner.catalog.scope.model_id,
                1,
                0,
            ),
        }
    }
    fn layer(&mut self) -> Result<layer::Run> {
        let owner = &mut self.base.owner;
        let states = match self.states {
            States::Baseline(s) => layer::States::Baseline(s),
            States::Tiles(s) => layer::States::Tiles(s),
        };
        // SAFETY: sealed constructor and pre-open profile bind the exact original
        // model roles and image pair; this owner admits only one layer0 call.
        unsafe {
            layer::execute_selected(
                &mut owner.catalog.backend,
                states,
                owner
                    .artifacts
                    .as_ref()
                    .ok_or("one-layer resident artifacts")?,
                owner.prefix_artifacts.as_ref(),
                self.projection,
                owner
                    .tiles_artifacts
                    .as_ref()
                    .ok_or("one-layer MLP image")?,
                &owner.layer_bindings[0],
                self.base.timeout_ms,
            )
        }
    }
    fn fence(&mut self) -> Result<()> {
        self.base.idle_fence_for(self.allocation)
    }
    fn poison(&mut self) {
        self.base.owner.catalog.phase = Phase::Terminal;
        self.states.poison();
    }
}

#[cfg(test)]
#[path = "native_prefix_tiles_layer_v6/tests.rs"]
mod tests;
