//! Explicit Readiness40 transport. Full2303 has no native admission here.
use crate::finite_guarded_mlp_decode_wire_v1 as four;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::finite_long_wire_v1::{read_header, write_header};
use crate::finite_prefix_decode_wire_v1 as prefix;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const WORKER_FLAG: &str = "--engineering-native-guarded-mlp-readiness40-v1";
pub const SCHEMA: &str = "FerricGuardedMlpReadiness40BootstrapV1";
pub const POSITION5_WORKER_FLAG: &str = "--engineering-native-guarded-mlp-readiness40-position5-v1";
pub const POSITION5_SCHEMA: &str = "FerricGuardedMlpReadiness40Position5BootstrapV1";
pub const POSITION5_CLOSE_SCHEMA: &str = "FerricGuardedMlpReadiness40Position5ClosedV1";
pub const CLOSE_SCHEMA: &str = "FerricGuardedMlpReadiness40ClosedV1";
pub const MAX_DEADLINE_MS: u64 = 3_600_000;
pub const MAX_IMAGE_BYTES: usize = 32 << 20;

fn require(ok: bool, why: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}

pub fn schemas(profile: long::Profile) -> io::Result<(&'static str, &'static str, &'static str)> {
    match profile {
        long::Profile::Readiness40 => Ok((SCHEMA, CLOSE_SCHEMA, WORKER_FLAG)),
        long::Profile::Readiness40Position5 => Ok((
            POSITION5_SCHEMA,
            POSITION5_CLOSE_SCHEMA,
            POSITION5_WORKER_FLAG,
        )),
        long::Profile::Full2303 => Err(io::Error::other("readiness refuses Full2303")),
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
            self.schema == schemas(self.sequence.profile)?.0
                && (1_000..=MAX_DEADLINE_MS).contains(&self.child_deadline_ms),
            "readiness closed native profile/deadline",
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
            "readiness exact image body",
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
    read_bootstrap_for(r, budget, long::Profile::Readiness40)
}
pub fn read_bootstrap_for(
    r: &mut impl Read,
    budget: &mut long::FrameBudget,
    expected: long::Profile,
) -> io::Result<Option<(Bootstrap, [Vec<u8>; 4])>> {
    schemas(expected)?;
    let Some(b) = read_header::<Bootstrap>(r, budget)? else {
        return Ok(None);
    };
    b.validate(
        b.sequence.device_ids,
        b.sequence.timeout_ms,
        b.sequence.scope.child_identity,
    )?;
    require(
        b.sequence.profile == expected,
        "readiness explicit entry/profile mismatch",
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
        require(n != 0 && n <= MAX_IMAGE_BYTES, "readiness image bound")?;
        budget.charge(n)?;
        image.resize(n, 0);
        r.read_exact(image)?;
        require(
            crate::finite_forward_wire_v1::part(image) == *pin,
            "readiness image digest",
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
    pub fn new(request: long::Request, digest: [u8; 32]) -> io::Result<Self> {
        Self::new_for(long::Profile::Readiness40, request, digest)
    }
    pub fn new_for(
        profile: long::Profile,
        request: long::Request,
        digest: [u8; 32],
    ) -> io::Result<Self> {
        let value = Self {
            schema: schemas(profile)?.1.into(),
            request,
            completed_forwards: long::READINESS_FORWARDS,
            generated_tokens: Vec::new(),
            transcript_sha256: digest,
            native_closed: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        };
        value.validate_for(profile, &value.request, digest)?;
        Ok(value)
    }
    pub fn validate(&self, request: &long::Request, digest: [u8; 32]) -> io::Result<()> {
        self.validate_for(long::Profile::Readiness40, request, digest)
    }
    pub fn validate_for(
        &self,
        profile: long::Profile,
        request: &long::Request,
        digest: [u8; 32],
    ) -> io::Result<()> {
        request.validate(profile)?;
        require(
            self.schema == schemas(profile)?.1
                && self.request == *request
                && matches!(request.command, long::Command::Close)
                && self.completed_forwards == long::READINESS_FORWARDS
                && self.generated_tokens.is_empty()
                && self.transcript_sha256 == digest
                && self.native_closed
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "readiness exact healthy Close",
        )
    }
}

#[cfg(test)]
#[path = "finite_guarded_mlp_readiness_wire_v1_tests.rs"]
mod tests;
