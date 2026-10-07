//! Separate Full2303 transport. Its fixed abort bound is not launch feasibility.
use crate::finite_guarded_mlp_decode_wire_v1 as four;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::finite_long_wire_v1::{read_header, write_header};
use crate::finite_prefix_decode_wire_v1 as prefix;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const WORKER_FLAG: &str = "--engineering-native-guarded-mlp-full2303-v1";
pub const SCHEMA: &str = "FerricGuardedMlpFull2303BootstrapV1";
pub const CLOSE_SCHEMA: &str = "FerricGuardedMlpFull2303ClosedV1";
// An independently named hard abort cap, not permission to launch or a speed estimate.
pub const MAX_DEADLINE_MS: u64 = 3_600_000;
pub const MAX_IMAGE_BYTES: usize = 32 << 20;

fn require(ok: bool, why: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub schema: String,
    pub sequence: long::Bootstrap,
    pub child_deadline_ms: u64,
}
impl Bootstrap {
    pub fn validate(&self, devices: [u64; 2], timeout: u32, pid: u32) -> io::Result<()> {
        require(
            self.schema == SCHEMA
                && self.sequence.profile == long::Profile::Full2303
                && (1_000..=MAX_DEADLINE_MS).contains(&self.child_deadline_ms),
            "full2303 closed native profile/deadline",
        )?;
        self.sequence.validate(devices, timeout, pid)
    }
    /// This is only the pre-existing setup profile. Native forward framing uses
    /// the distinct long profile hash and can never be sent through AR4 serve.
    pub fn setup(&self) -> io::Result<four::Bootstrap> {
        let b = &self.sequence;
        self.validate(b.device_ids, b.timeout_ms, b.scope.child_identity)?;
        Ok(four::Bootstrap {
            schema: four::REUSE_SCHEMA.into(),
            decode: prefix::Bootstrap {
                protocol: prefix::PROTOCOL,
                profile: prefix::Profile::Prefix284Mlp548FourForwardV1,
                device_ids: b.device_ids,
                scope: b.scope.clone(),
                registration: b.registration,
                begin: b.begin.clone(),
                timeout_ms: b.timeout_ms,
                mode: prefix::InputMode::Autoregressive,
                input_tokens: vec![b.prompt_tokens[0]],
                tiles_image: b.mlp_image.clone(),
                prefix_image: b.prefix_image.clone(),
            },
            projection_image: b.projection_image.clone(),
            guarded_image: b.guarded_image.clone(),
        })
    }
}

pub fn write_bootstrap(
    w: &mut impl Write,
    budget: &mut long::FrameBudget,
    b: &Bootstrap,
    images: [&[u8]; 4],
) -> io::Result<()> {
    b.validate(
        b.sequence.device_ids,
        b.sequence.timeout_ms,
        b.sequence.scope.child_identity,
    )?;
    let pins = [
        &b.sequence.mlp_image,
        &b.sequence.prefix_image,
        &b.sequence.projection_image,
        &b.sequence.guarded_image,
    ];
    for (image, pin) in images.iter().zip(pins) {
        require(
            !image.is_empty()
                && image.len() <= MAX_IMAGE_BYTES
                && crate::finite_forward_wire_v1::part(image) == *pin,
            "full2303 exact image body",
        )?;
    }
    budget.charge(images.iter().map(|image| image.len()).sum())?;
    write_header(w, budget, b)?;
    for image in images {
        w.write_all(image)?;
    }
    w.flush()
}
pub fn read_bootstrap(
    r: &mut impl Read,
    budget: &mut long::FrameBudget,
) -> io::Result<Option<(Bootstrap, [Vec<u8>; 4])>> {
    let Some(b) = read_header::<Bootstrap>(r, budget)? else {
        return Ok(None);
    };
    b.validate(
        b.sequence.device_ids,
        b.sequence.timeout_ms,
        b.sequence.scope.child_identity,
    )?;
    require(
        b.sequence.profile == long::Profile::Full2303,
        "full2303 explicit entry/profile mismatch",
    )?;
    let pins = [
        &b.sequence.mlp_image,
        &b.sequence.prefix_image,
        &b.sequence.projection_image,
        &b.sequence.guarded_image,
    ];
    let mut images = std::array::from_fn(|_| Vec::new());
    for (image, pin) in images.iter_mut().zip(pins) {
        let n = usize::try_from(pin.bytes).map_err(io::Error::other)?;
        require(n != 0 && n <= MAX_IMAGE_BYTES, "full2303 image bound")?;
        budget.charge(n)?;
        image.resize(n, 0);
        r.read_exact(image)?;
        require(
            crate::finite_forward_wire_v1::part(image) == *pin,
            "full2303 image digest",
        )?;
    }
    Ok(Some((b, images)))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Closed {
    pub schema: String,
    pub request: long::Request,
    pub completed_forwards: u32,
    pub generated_tokens: Vec<u32>,
    pub transcript_sha256: [u8; 32],
    pub native_closed: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
impl Closed {
    /// Native publication obtains tokens from its already closed local transcript,
    /// after separately joining the genuine owner's committed output history.
    pub fn from_transcript(
        request: long::Request,
        transcript: &long::Transcript,
    ) -> io::Result<Self> {
        require(
            transcript.profile() == long::Profile::Full2303
                && transcript.is_closed()
                && transcript.completed() == long::FORWARDS
                && request.profile_sha256 == transcript.profile_sha256(),
            "full2303 Closed requires an actual closed full transcript",
        )?;
        let value = Self {
            schema: CLOSE_SCHEMA.into(),
            request,
            completed_forwards: long::FORWARDS,
            generated_tokens: transcript.output_tokens().to_vec(),
            transcript_sha256: transcript.digest(),
            native_closed: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        };
        value.validate(
            &value.request,
            transcript.digest(),
            transcript.output_tokens(),
        )?;
        Ok(value)
    }
    /// The parent supplies only its own independently reconstructed, committed
    /// record history. A worker-provided list alone is not an acceptance oracle.
    pub fn validate(
        &self,
        request: &long::Request,
        digest: [u8; 32],
        generated_tokens: &[u32],
    ) -> io::Result<()> {
        request.validate(long::Profile::Full2303)?;
        require(
            self.schema == CLOSE_SCHEMA
                && self.request == *request
                && matches!(request.command, long::Command::Close)
                && self.completed_forwards == long::FORWARDS
                && generated_tokens.len() == long::OUTPUT_TOKENS
                && generated_tokens.iter().all(|token| *token < 151_936)
                && self.generated_tokens == generated_tokens
                && self.transcript_sha256 == digest
                && self.native_closed
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "full2303 exact healthy Close and committed output history",
        )
    }
}

#[cfg(test)]
#[path = "finite_guarded_mlp_full2303_wire_v1_tests.rs"]
mod tests;
