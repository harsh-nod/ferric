//! Explicit long owner. The original source registration remains a setup record.

use super::{
    Active, ForwardInput, ForwardRun, NativeOwner, Phase, Result, ReusableStateRosterV1,
    validate_sealed,
};
use crate::finite_long_wire_v1::Bootstrap;
use crate::long_forward_sequence_v1::Sequence;

pub(crate) struct LongForwardOwnerV1 {
    owner: NativeOwner,
    reuse: ReusableStateRosterV1,
    sequence: Sequence,
    timeout_ms: u32,
}

impl LongForwardOwnerV1 {
    /// # Safety
    /// All ordinary retained image/model premises of ForwardOwner apply. In
    /// addition the trusted parent must explicitly select the distinct closed
    /// long profile, supply authentic prompt/metadata, and own this synchronous
    /// command stream until healthy Close or fatal owned-process teardown.
    /// No saved state pointer or command may escape and re-enter a generation.
    /// The old two-slot source record is not evidence of long execution.
    pub(crate) unsafe fn from_sealed(
        mut owner: NativeOwner,
        bootstrap: &Bootstrap,
    ) -> Result<Self> {
        bootstrap
            .validate(
                bootstrap.device_ids,
                bootstrap.timeout_ms,
                std::process::id(),
            )
            .map_err(|error| error.to_string())?;
        validate_sealed(&owner, bootstrap.timeout_ms)?;
        let scope = &owner.catalog.scope;
        if scope.bundle_id != bootstrap.scope.bundle_id
            || scope.model_id != bootstrap.scope.model_id
            || scope.session != bootstrap.scope.session
            || scope.pool_identity != bootstrap.scope.pool_identity
            || scope.group_id != bootstrap.scope.group_id
            || scope.child_identity != bootstrap.scope.child_identity
        {
            return Err("long profile differs from frozen setup scope".into());
        }
        let sequence = Sequence::new(owner.registration_sha256(), &bootstrap.prompt_tokens)?;
        let reuse = ReusableStateRosterV1::from_fresh(
            owner
                .sealed_states
                .take()
                .ok_or("long fresh state owner missing")?,
        )?;
        Ok(Self {
            owner,
            reuse,
            sequence,
            timeout_ms: bootstrap.timeout_ms,
        })
    }

    pub(crate) fn run(&mut self, input: &ForwardInput) -> Result<ForwardRun> {
        let mut active = Active {
            owner: &mut self.owner,
            timeout_ms: self.timeout_ms,
            layer_hidden: Vec::with_capacity(36),
            final_normalized: Vec::new(),
            logits: Vec::new(),
            capture: None,
            captured_layer0: None,
            reuse: Some((&mut self.reuse, input.generation)),
        };
        let completion = self.sequence.run(&mut active, input)?;
        Ok(ForwardRun {
            completion,
            layer_hidden: active.layer_hidden,
            final_normalized: active.final_normalized,
            logits: active.logits,
        })
    }

    pub(crate) fn close(mut self) -> Result<()> {
        if !self.sequence.exhausted() || !self.reuse.exhausted() {
            self.owner.catalog.phase = Phase::Terminal;
            self.reuse.poison();
            return Err("long close before all2303 committed forwards".into());
        }
        self.owner.close_setup()
    }
}
