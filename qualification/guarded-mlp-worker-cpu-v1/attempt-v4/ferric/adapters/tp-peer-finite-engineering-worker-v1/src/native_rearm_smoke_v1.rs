//! Separate four-forward owner sharing only the bounded physical rearm mechanism.

use super::{
    Active, Collector, ForwardInput, ForwardRun, Layer0CaptureV1, NativeOwner, Phase, Result,
    ReusableStateRosterV1, validate_sealed,
};
use crate::finite_rearm_smoke_wire_v1::Bootstrap;
use crate::rearm_smoke_sequence_v1::Sequence;

pub(crate) struct SmokeRun {
    pub(crate) forward: ForwardRun,
    pub(crate) layer0: Option<Layer0CaptureV1>,
}
pub(crate) struct SmokeOwner {
    owner: NativeOwner,
    reuse: ReusableStateRosterV1,
    sequence: Sequence,
    timeout_ms: u32,
    capture_layer0: bool,
}
impl SmokeOwner {
    /// # Safety
    /// Authenticated parent model/setup and reviewed machine code premises match
    /// ForwardOwner, with a distinct declared four-token engineering profile.
    /// The private synchronous stream must retire every old state pointer/command
    /// before rearming. This consumes a fresh owner, never an executed P223 owner.
    pub(crate) unsafe fn from_sealed(
        mut owner: NativeOwner,
        bootstrap: &Bootstrap,
    ) -> Result<Self> {
        bootstrap
            .validate(
                bootstrap.device_ids,
                bootstrap.timeout_ms,
                std::process::id(),
                bootstrap.capture_layer0,
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
            return Err("smoke profile differs from frozen setup scope".into());
        }
        let sequence = Sequence::new(owner.registration_sha256(), &bootstrap.prompt_tokens)?;
        let reuse = ReusableStateRosterV1::from_fresh(
            owner
                .sealed_states
                .take()
                .ok_or("smoke fresh state roster missing")?,
        )?;
        Ok(Self {
            owner,
            reuse,
            sequence,
            timeout_ms: bootstrap.timeout_ms,
            capture_layer0: bootstrap.capture_layer0,
        })
    }
    pub(crate) fn run(&mut self, input: &ForwardInput) -> Result<SmokeRun> {
        let selected = self.capture_layer0 && input.generation == 1;
        let capture = if selected {
            match Collector::new(input.generation, input.cache_metadata[0], 0) {
                Ok(capture) => Some(capture),
                Err(error) => {
                    self.owner.catalog.phase = Phase::Terminal;
                    self.reuse.poison();
                    return Err(error);
                }
            }
        } else {
            None
        };
        let mut active = Active {
            owner: &mut self.owner,
            timeout_ms: self.timeout_ms,
            layer_hidden: Vec::with_capacity(36),
            final_normalized: Vec::new(),
            logits: Vec::new(),
            capture,
            captured_layer0: None,
            reuse: Some((&mut self.reuse, input.generation)),
        };
        let completion = self.sequence.run(&mut active, input)?;
        if active.captured_layer0.is_some() != selected {
            crate::forward_sequence::Backend::poison(&mut active);
            return Err("smoke layer-zero capture presence mismatch".into());
        }
        Ok(SmokeRun {
            forward: ForwardRun {
                completion,
                layer_hidden: active.layer_hidden,
                final_normalized: active.final_normalized,
                logits: active.logits,
            },
            layer0: active.captured_layer0,
        })
    }
    pub(crate) fn close(mut self) -> Result<()> {
        // Sequence commits only after actual reusable-state commit returned Ok.
        // Four-forward completion is deliberately not reuse.exhausted()==2303.
        if !self.sequence.exhausted() {
            self.owner.catalog.phase = Phase::Terminal;
            self.reuse.poison();
            return Err("smoke Close before four committed generations".into());
        }
        self.owner.close_setup()
    }
}
