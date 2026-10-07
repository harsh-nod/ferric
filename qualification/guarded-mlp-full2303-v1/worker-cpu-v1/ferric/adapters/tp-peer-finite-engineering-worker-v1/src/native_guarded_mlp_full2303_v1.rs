//! Explicit full-workload custody; no launch feasibility or numerical authority.
use super::{
    Mode,
    readiness::{Driver, capacity, poison},
};
use crate::finite_guarded_mlp_full2303_wire_v1::MAX_DEADLINE_MS;
use crate::finite_guarded_mlp_long_wire_v2::{
    Bootstrap, FORWARDS, OUTPUT_TOKENS, Profile, Request,
};
use crate::guarded_mlp_long_sequence_v2::{Produced, Sequence};
use std::{
    io,
    time::{Duration, Instant},
};

const WHOLE_LIMIT: Duration = Duration::from_millis(MAX_DEADLINE_MS);
fn require(ok: bool, why: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}
fn admit_deadline(now: Instant, deadline: Instant) -> io::Result<()> {
    require(
        now < deadline && deadline.duration_since(now) <= WHOLE_LIMIT,
        "full2303 separately bounded outer deadline",
    )
}
fn profile_join(b: &Bootstrap, profile: &super::Profile) -> io::Result<()> {
    b.validate(b.device_ids, b.timeout_ms, std::process::id())?;
    require(
        b.profile == Profile::Full2303
            && profile.reusable_arenas
            && !profile.paired_terminal
            && matches!(profile.mode, Mode::Autoregressive { first } if first == b.prompt_tokens[0])
            && profile.scope == b.scope
            && profile.registration == b.registration
            && profile.prefix == b.prefix_image.sha256
            && profile.mlp == b.mlp_image.sha256
            && profile.projection == b.projection_image.sha256
            && profile.timeout_ms == b.timeout_ms,
        "full2303 exact sealed reusable profile and images",
    )?;
    let original = crate::finite_guarded_mlp_decode_wire_v1::profile_sha256(
        &b.scope,
        b.registration,
        b.prefix_image.sha256,
        b.mlp_image.sha256,
        b.projection_image.sha256,
        true,
        [b.prompt_tokens[0], 0, 0, 0],
        b.timeout_ms,
        b.device_ids,
    );
    require(
        profile.sha256
            == crate::finite_guarded_mlp_decode_wire_v1::reusable_profile_sha256(original),
        "full2303 original device/rank setup profile",
    )
}
struct Admission<'a> {
    base: &'a mut super::Owner,
    committed: bool,
}
impl Drop for Admission<'_> {
    fn drop(&mut self) {
        if !self.committed {
            poison(self.base);
        }
    }
}
pub(crate) struct Owner {
    base: super::Owner,
    sequence: Sequence,
    deadline: Instant,
}
impl Owner {
    pub(crate) fn from_owner(
        mut base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        let mut admission = Admission {
            base: &mut base,
            committed: false,
        };
        admit_deadline(Instant::now(), deadline)?;
        profile_join(&b, &admission.base.profile)?;
        require(
            admission.base.sequence.pristine()
                && admission.base.states.completed() == 0
                && admission.base.states.between()
                && !admission.base.states.readiness40()
                && !admission.base.states.full2303()
                && !admission.base.capture_enabled
                && admission.base.captured_layer0.is_none()
                && admission.base.observer.is_none(),
            "full2303 consumes pristine reusable owner without diagnostics",
        )?;
        require(
            b.begin.source_program.bytes
                == admission.base.owner.catalog.source.source_program_bytes
                && b.begin.source_program.sha256
                    == admission.base.owner.catalog.source.source_program_sha256,
            "full2303 actual source-program custody",
        )?;
        capacity(admission.base)?;
        let sequence = Sequence::new(b)?;
        admission
            .base
            .states
            .select_full2303()
            .map_err(io::Error::other)?;
        admit_deadline(Instant::now(), deadline)?;
        admission.committed = true;
        drop(admission);
        Ok(Self {
            base,
            sequence,
            deadline,
        })
    }
    pub(crate) fn digest(&self) -> [u8; 32] {
        self.sequence.digest()
    }
    pub(crate) fn output_tokens(&self) -> &[u32] {
        self.sequence.output_tokens()
    }
    pub(crate) fn run(&mut self, request: &Request) -> io::Result<Produced> {
        let mut driver = Driver::full2303(&mut self.base, self.deadline)?;
        self.sequence.run(&mut driver, request)
    }
    pub(crate) fn close(mut self, request: &Request, digest: [u8; 32]) -> io::Result<()> {
        require(
            self.sequence.completed() == FORWARDS
                && self.sequence.output_tokens().len() == OUTPUT_TOKENS,
            "full2303 Close requires actual complete own-output history",
        )?;
        let mut driver = Driver::full2303(&mut self.base, self.deadline)?;
        self.sequence.close(&mut driver, request, digest)
    }
    pub(crate) fn cancel(&mut self) {
        match Driver::full2303(&mut self.base, self.deadline) {
            Ok(mut driver) => self.sequence.cancel(&mut driver),
            Err(_) => poison(&mut self.base),
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if !self.sequence.is_closed() {
            poison(&mut self.base);
        }
    }
}
#[cfg(test)]
#[path = "native_guarded_mlp_full2303_v1_tests.rs"]
mod tests;
