//! Closed policy identity; this record does not authorize execution by itself.
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::finite_guarded_mlp_readiness_wire_v1 as ready;
use serde::{Deserialize, Serialize};
use std::io;

pub const WORKER_FLAG: &str =
    "--engineering-native-guarded-mlp-readiness40-position5-shared-full-v1";
pub const SCHEMA: &str = "FerricReadiness40Position5SharedFullPolicyV1";
pub const MAX_BYTES: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRecord {
    pub schema: String,
    pub session: [u8; 32],
    pub device_ids: [u64; 2],
    pub child_pid: u32,
    pub worker_sha256: [u8; 32],
    pub profile_sha256: [u8; 32],
    pub registration_sha256: [u8; 32],
    pub transcript_sha256: [u8; 32],
    pub completed_forwards: u32,
    pub generated_tokens: Vec<u32>,
    pub capture_positions: [u32; 4],
    pub shared_full_currentness: bool,
    pub cache_kernel_admission: bool,
    pub operational_currentness: bool,
    pub legacy_profile: bool,
    pub host_observer: bool,
    pub paired_hidden_reads: bool,
    pub paired_terminal: bool,
    pub native_closed: bool,
    pub full_long_workload: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
impl PolicyRecord {
    pub fn new(b: &ready::Bootstrap, transcript: [u8; 32], worker: [u8; 32]) -> io::Result<Self> {
        b.validate(
            b.sequence.device_ids,
            b.sequence.timeout_ms,
            b.sequence.scope.child_identity,
        )?;
        if b.sequence.profile != long::Profile::Readiness40Position5
            || transcript == [0; 32]
            || worker == [0; 32]
        {
            return Err(io::Error::other(
                "shared full requires closed Position5 identity",
            ));
        }
        Ok(Self {
            schema: SCHEMA.into(),
            session: b.sequence.scope.session,
            device_ids: b.sequence.device_ids,
            child_pid: b.sequence.scope.child_identity,
            worker_sha256: worker,
            profile_sha256: b.sequence.sha256()?,
            registration_sha256: b.sequence.registration,
            transcript_sha256: transcript,
            completed_forwards: 40,
            generated_tokens: Vec::new(),
            capture_positions: [0, 5, 16, 39],
            shared_full_currentness: true,
            cache_kernel_admission: false,
            operational_currentness: false,
            legacy_profile: false,
            host_observer: false,
            paired_hidden_reads: false,
            paired_terminal: false,
            native_closed: true,
            full_long_workload: false,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        })
    }
    pub fn validate(
        &self,
        b: &ready::Bootstrap,
        transcript: [u8; 32],
        worker: [u8; 32],
    ) -> io::Result<()> {
        if self != &Self::new(b, transcript, worker)? {
            return Err(io::Error::other(
                "shared full exact policy and Close identity",
            ));
        }
        Ok(())
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut raw = serde_json::to_vec(self).map_err(io::Error::other)?;
        raw.push(b'\n');
        if raw.len() > MAX_BYTES {
            return Err(io::Error::other("shared full record bound"));
        }
        Ok(raw)
    }
    pub fn write_to(&self, w: &mut (impl io::Write + ?Sized)) -> io::Result<()> {
        w.write_all(&self.encode()?)?;
        w.flush()
    }
    pub fn decode(
        raw: &[u8],
        b: &ready::Bootstrap,
        transcript: [u8; 32],
        worker: [u8; 32],
    ) -> io::Result<Self> {
        if raw.is_empty() || raw.len() > MAX_BYTES || raw.last() != Some(&b'\n') {
            return Err(io::Error::other("shared full single bounded policy record"));
        }
        let value: Self = serde_json::from_slice(raw).map_err(io::Error::other)?;
        value.validate(b, transcript, worker)?;
        if value.encode()? != raw {
            return Err(io::Error::other("shared full exact record framing"));
        }
        Ok(value)
    }
}

#[cfg(test)]
#[path = "finite_guarded_mlp_readiness_shared_v1_tests.rs"]
mod tests;
