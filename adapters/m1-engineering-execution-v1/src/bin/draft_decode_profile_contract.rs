//! Frozen bounded repeated-decode contract. References never come from the candidate.

use super::draft_paged_canary_contract::{
    ModelIdentity, Options as CanaryOptions, Prefill, ReferenceStep, Scheduled, checked_sha, digest,
};
use ferric_build::{DRAFT_REPOSITORY, DRAFT_REVISION};
use ferric_m1_engineering_execution_v1::tp_paged::EngineeringTpPagedPoolV1;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

pub const MAX_REFERENCE_BYTES: u64 = 262_144;
pub const PACKETS_PER_FORWARD: u64 = 480;

pub struct Options {
    pub common: CanaryOptions,
    pub new_tokens: usize,
    pub warmups: usize,
    pub samples: usize,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut remaining = Vec::new();
        let mut arguments = arguments;
        let mut seen = BTreeSet::new();
        let (mut new_tokens, mut warmups, mut samples) = (None, None, None);
        while let Some(flag) = arguments.next() {
            let destination = match flag.as_str() {
                "--new-tokens" => Some(&mut new_tokens),
                "--warmups" => Some(&mut warmups),
                "--samples" => Some(&mut samples),
                _ => None,
            };
            if let Some(destination) = destination {
                if !seen.insert(flag.clone()) {
                    return Err(format!("duplicate option {flag}"));
                }
                let value = arguments.next().ok_or_else(|| format!("missing {flag}"))?;
                *destination = Some(value.parse::<usize>().map_err(|e| e.to_string())?);
            } else {
                remaining.push(flag);
            }
        }
        let value = Self {
            common: CanaryOptions::parse(remaining.into_iter())?,
            new_tokens: new_tokens.ok_or("explicit --new-tokens required")?,
            warmups: warmups.ok_or("explicit --warmups required")?,
            samples: samples.ok_or("explicit --samples required")?,
        };
        if !(2..=128).contains(&value.new_tokens)
            || value.warmups > 10
            || !(1..=30).contains(&value.samples)
        {
            return Err("profile bounds: new-tokens 2..128, warmups 0..10, samples 1..30".into());
        }
        if value.total_dispatches() > fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 {
            return Err("combined profile exceeds the existing no-rollover packet budget".into());
        }
        Ok(value)
    }

    pub const fn sampling_class(&self) -> &'static str {
        if self.warmups == 10 && self.samples == 30 {
            "benchmark-sized-unqualified"
        } else {
            "diagnostic-only"
        }
    }

    pub fn physical_pages(&self) -> u32 {
        u32::try_from((self.new_tokens + 4).div_ceil(16)).expect("bounded profile pages")
    }

    pub fn context_tokens(&self) -> u32 {
        self.physical_pages() * 16
    }

    pub fn total_dispatches(&self) -> u64 {
        u64::try_from(
            (self.warmups + self.samples) * forward_count(self.common.prefill, self.new_tokens),
        )
        .expect("bounded profile forward count")
            * PACKETS_PER_FORWARD
    }
}

pub fn require_reusable_pool(pool: &EngineeringTpPagedPoolV1, pages: u32) -> Result<(), String> {
    pool.check_invariants()
        .map_err(|e| format!("pool invariants: {e:?}"))?;
    let stats = pool.stats();
    if stats.sequences != 0
        || stats.free_pages != pages
        || stats.retained_pages != 0
        || stats.cached_pages != 0
        || stats.quarantined_pages != 0
        || stats.prefix_hits != 0
    {
        return Err("profile requires fully retired, uncached and unquarantined storage".into());
    }
    Ok(())
}

pub const fn forward_count(prefill: Prefill, new_tokens: usize) -> usize {
    match prefill {
        Prefill::Full => new_tokens,
        Prefill::Tokenwise => new_tokens + 4,
    }
}

pub fn scheduled(
    prefill: Prefill,
    prompt: &[u32; 5],
    new_tokens: usize,
    ordinal: usize,
    previous: Option<u32>,
) -> Result<Scheduled, String> {
    if !(2..=128).contains(&new_tokens)
        || ordinal >= forward_count(prefill, new_tokens)
        || prompt.iter().any(|&token| token >= 151_936)
        || previous.is_some_and(|token| token >= 151_936)
    {
        return Err("profile schedule bound".into());
    }
    if prefill == Prefill::Full && ordinal == 0 {
        return Ok(Scheduled {
            inputs: prompt.to_vec(),
            positions: (0..5).collect(),
            selected_row: 4,
        });
    }
    let position = if prefill == Prefill::Full {
        ordinal + 4
    } else {
        ordinal
    };
    let token = if position < 5 {
        prompt[position]
    } else {
        previous.ok_or("decode requires the previous completed choice")?
    };
    Ok(Scheduled {
        inputs: vec![token],
        positions: vec![u32::try_from(position).map_err(|_| "position overflow")?],
        selected_row: 0,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub schema: String,
    pub model: String,
    pub model_revision: String,
    pub identity: ModelIdentity,
    pub head_precision: String,
    pub prefill: String,
    pub prompt_tokens: [u32; 5],
    pub steps: Vec<ReferenceStep>,
    pub generated_tokens: Vec<u32>,
    pub generated_utf8_bytes: Vec<u8>,
    pub producer: String,
}

impl Reference {
    pub fn parse(bytes: &[u8], expected: &str, options: &Options) -> Result<Self, String> {
        if bytes.is_empty()
            || bytes.len() > usize::try_from(MAX_REFERENCE_BYTES).map_err(|_| "reference cap")?
            || digest(bytes) != checked_sha(expected)?
        {
            return Err("profile reference bytes/hash bound".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let schema = value.schema == "FerricDraftDecodeProfileReferenceV1"
            || (options.new_tokens == 2 && value.schema == "FerricDraftPagedCanaryReferenceV10");
        if !schema
            || value.model != DRAFT_REPOSITORY
            || value.model_revision != DRAFT_REVISION
            || value.head_precision != "fp32-v10"
            || value.prefill != options.common.prefill.label()
            || value.producer.is_empty()
            || value.producer.len() > 1024
            || value.generated_utf8_bytes.len() > 16_384
            || value.generated_tokens.len() != options.new_tokens
            || value.steps.len() != forward_count(options.common.prefill, options.new_tokens)
        {
            return Err("profile reference identity/shape/policy bound".into());
        }
        for hash in [
            &value.identity.model_bundle_id,
            &value.identity.draft_model_id,
            &value.identity.draft_config_id,
            &value.identity.draft_weights_sha256,
        ] {
            checked_sha(hash)?;
        }
        let mut generated = Vec::new();
        let mut previous = None;
        for (ordinal, step) in value.steps.iter().enumerate() {
            let plan = scheduled(
                options.common.prefill,
                &value.prompt_tokens,
                options.new_tokens,
                ordinal,
                previous,
            )?;
            if step.inputs != plan.inputs
                || step.positions != plan.positions
                || step.selected_row != plan.selected_row
                || step.choice >= 151_936
                || step.cache_tokens != plan.positions.last().ok_or("empty reference plan")? + 1
            {
                return Err("profile reference completed-input schedule".into());
            }
            if step.cache_tokens >= 5 {
                generated.push(step.choice);
            }
            previous = Some(step.choice);
        }
        if generated != value.generated_tokens {
            return Err("profile reference output schedule".into());
        }
        Ok(value)
    }

    pub fn open(options: &Options) -> Result<Self, String> {
        let file = std::fs::File::open(&options.common.reference).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_REFERENCE_BYTES {
            return Err("profile reference file bound".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_REFERENCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if u64::try_from(bytes.len()).map_err(|_| "reference length")? != metadata.len() {
            return Err("profile reference length changed".into());
        }
        Self::parse(&bytes, &options.common.reference_sha256, options)
    }

    pub fn bind(&self, identity: &ModelIdentity, prompt: &[u32; 5]) -> Result<(), String> {
        if &self.identity != identity || &self.prompt_tokens != prompt {
            return Err("profile reference actual model/prompt mismatch".into());
        }
        Ok(())
    }
}

pub fn raw_ns() -> Result<u64, String> {
    let time = rustix::time::clock_gettime(rustix::time::ClockId::MonotonicRaw);
    u64::try_from(time.tv_sec)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .and_then(|base| {
            u64::try_from(time.tv_nsec)
                .ok()
                .and_then(|nanos| base.checked_add(nanos))
        })
        .ok_or_else(|| "CLOCK_MONOTONIC_RAW overflow".into())
}

pub fn offset(origin: u64) -> Result<u64, String> {
    raw_ns()?
        .checked_sub(origin)
        .ok_or_else(|| "monotonic clock moved backwards".into())
}

pub fn identity_unchanged(path: &Path, expected: &str) -> Result<(), String> {
    if super::draft_paged_canary_contract::hash_file(path)? != expected {
        return Err(format!("executable identity changed: {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
#[path = "draft_decode_profile_tests.rs"]
mod tests;
