//! Bounded profile/reference contract, separate from the frozen contiguous canary.

use super::tp_worker::RuntimeOptions;
use ferric_build::{DRAFT_REPOSITORY, DRAFT_REVISION};
use ferric_m1_engineering_execution_v1::tp_execution::EngineeringTpProjectionModeV3;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const PROMPT: &str = "The capital of France is";
const LIMIT: u64 = 65_536;
const USAGE: &str = "ferric-qwen3-draft-paged-canary --source DIR --artifact DIR --worker FILE --worker-sha256 HEX --device-unique-id ID --projection baseline|mfma --prefill full|tokenwise --reference FILE --reference-sha256 HEX --allow-unauthenticated-machine-code [--runtime-cache-admission] [--runtime-operational]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Prefill {
    Full,
    Tokenwise,
}
impl Prefill {
    pub const fn steps(self) -> usize {
        match self {
            Self::Full => 2,
            Self::Tokenwise => 6,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Tokenwise => "tokenwise",
        }
    }
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "full" => Ok(Self::Full),
            "tokenwise" => Ok(Self::Tokenwise),
            _ => Err("explicit full or tokenwise prefill required".into()),
        }
    }
}

pub struct Options {
    pub source: PathBuf,
    pub artifact: PathBuf,
    pub worker: PathBuf,
    pub worker_sha256: String,
    pub device: u64,
    pub projection: EngineeringTpProjectionModeV3,
    pub prefill: Prefill,
    pub reference: PathBuf,
    pub reference_sha256: String,
    pub runtime: RuntimeOptions,
}
impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut seen = BTreeSet::new();
        let (mut source, mut artifact, mut worker, mut worker_sha256, mut device) =
            (None, None, None, None, None);
        let (mut projection, mut prefill, mut reference, mut reference_sha256) =
            (None, None, None, None);
        let mut consent = false;
        let mut runtime = RuntimeOptions::default();
        while let Some(flag) = arguments.next() {
            if !seen.insert(flag.clone()) {
                return Err(format!("duplicate option {flag}"));
            }
            match flag.as_str() {
                "--allow-unauthenticated-machine-code" => {
                    consent = true;
                    continue;
                }
                "--runtime-cache-admission" => {
                    runtime.cache_admission = true;
                    continue;
                }
                "--runtime-operational" => {
                    runtime.operational = true;
                    continue;
                }
                "--help" => return Err(USAGE.into()),
                _ => {}
            }
            let value = arguments
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--source" => source = Some(PathBuf::from(value)),
                "--artifact" => artifact = Some(PathBuf::from(value)),
                "--worker" => worker = Some(PathBuf::from(value)),
                "--worker-sha256" => worker_sha256 = Some(checked_sha(&value)?),
                "--device-unique-id" => {
                    device = Some(value.parse::<u64>().map_err(|e| e.to_string())?);
                }
                "--projection" => {
                    projection = Some(match value.as_str() {
                        "baseline" => EngineeringTpProjectionModeV3::Baseline,
                        "mfma" => EngineeringTpProjectionModeV3::Mfma,
                        _ => return Err("only baseline or MFMA projections supported".into()),
                    });
                }
                "--prefill" => prefill = Some(Prefill::parse(&value)?),
                "--reference" => reference = Some(PathBuf::from(value)),
                "--reference-sha256" => reference_sha256 = Some(checked_sha(&value)?),
                _ => return Err(format!("unknown option {flag}")),
            }
        }
        if !consent || device.is_none_or(|value| value == 0) {
            return Err("explicit machine-code consent and nonzero device required".into());
        }
        Ok(Self {
            source: source.ok_or("source required")?,
            artifact: artifact.ok_or("artifact required")?,
            worker: worker.ok_or("worker required")?,
            worker_sha256: worker_sha256.ok_or("worker hash required")?,
            device: device.ok_or("device required")?,
            projection: projection.ok_or("projection required")?,
            prefill: prefill.ok_or("prefill required")?,
            reference: reference.ok_or("reference required")?,
            reference_sha256: reference_sha256.ok_or("reference hash required")?,
            runtime,
        })
    }
}

pub fn checked_sha(value: &str) -> Result<String, String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("exact lowercase SHA-256 required".into());
    }
    Ok(value.to_owned())
}
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(DIGITS[usize::from(byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    result
}
pub fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
pub fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    if !before.is_file() || before.len() == 0 || before.len() > 512 * 1024 * 1024 {
        return Err("executable file bound".into());
    }
    let mut hasher = Sha256::new();
    let mut bytes = vec![0_u8; 65_536];
    let mut total = 0_u64;
    loop {
        let count = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(count).map_err(|_| "read count")?)
            .ok_or("read overflow")?;
        if total > before.len() {
            return Err("executable grew".into());
        }
        hasher.update(&bytes[..count]);
    }
    if total != before.len() || file.metadata().map_err(|e| e.to_string())?.len() != total {
        return Err("executable changed length".into());
    }
    Ok(hex(&hasher.finalize()))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelIdentity {
    pub model_bundle_id: String,
    pub draft_model_id: String,
    pub draft_config_id: String,
    pub draft_weights_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scheduled {
    pub inputs: Vec<u32>,
    pub positions: Vec<u32>,
    pub selected_row: usize,
}

pub fn scheduled(
    prefill: Prefill,
    prompt: &[u32; 5],
    ordinal: usize,
    previous: Option<u32>,
) -> Result<Scheduled, String> {
    if ordinal >= prefill.steps()
        || prompt.iter().any(|&t| t >= 151_936)
        || previous.is_some_and(|t| t >= 151_936)
    {
        return Err("schedule token/ordinal bound".into());
    }
    if prefill == Prefill::Full && ordinal == 0 {
        return Ok(Scheduled {
            inputs: prompt.to_vec(),
            positions: (0..5).collect(),
            selected_row: 4,
        });
    }
    let position = if prefill == Prefill::Full { 5 } else { ordinal };
    let token = if position < 5 {
        prompt[position]
    } else {
        previous.ok_or("decode needs completed prior choice")?
    };
    Ok(Scheduled {
        inputs: vec![token],
        positions: vec![u32::try_from(position).map_err(|_| "position")?],
        selected_row: 0,
    })
}

pub fn checked_choice(choices: &[u32]) -> Result<u32, String> {
    match choices {
        [value] if *value < 151_936 => Ok(*value),
        _ => Err("one in-vocabulary completed choice required".into()),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ObservedStep {
    pub inputs: Vec<u32>,
    pub positions: Vec<u32>,
    pub selected_row: usize,
    pub choice: u32,
    pub cache_tokens: u32,
    pub completed_dispatches: u64,
}
#[derive(Debug)]
pub struct Observation {
    pub steps: Vec<ObservedStep>,
    pub generated_tokens: [u32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceStep {
    pub inputs: Vec<u32>,
    pub positions: Vec<u32>,
    pub selected_row: usize,
    pub choice: u32,
    pub cache_tokens: u32,
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
    pub generated_tokens: [u32; 2],
    pub generated_utf8_bytes: Vec<u8>,
    pub producer: String,
}
impl Reference {
    pub fn parse(bytes: &[u8], expected: &str) -> Result<Self, String> {
        if bytes.is_empty()
            || bytes.len() > usize::try_from(LIMIT).map_err(|e| e.to_string())?
            || digest(bytes) != checked_sha(expected)?
        {
            return Err("reference bytes/hash bound".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if value.schema != "FerricDraftPagedCanaryReferenceV10"
            || value.model != DRAFT_REPOSITORY
            || value.model_revision != DRAFT_REVISION
            || value.head_precision != "fp32-v10"
            || value.producer.is_empty()
            || value.producer.len() > 1024
            || value.generated_utf8_bytes.len() > 16_384
        {
            return Err("reference identity/profile bound".into());
        }
        for hash in [
            &value.identity.model_bundle_id,
            &value.identity.draft_model_id,
            &value.identity.draft_config_id,
            &value.identity.draft_weights_sha256,
        ] {
            checked_sha(hash)?;
        }
        let prefill = Prefill::parse(&value.prefill)?;
        if value.steps.len() != prefill.steps() {
            return Err("reference forward count".into());
        }
        let mut previous = None;
        let mut generated = Vec::new();
        for (ordinal, step) in value.steps.iter().enumerate() {
            let plan = scheduled(prefill, &value.prompt_tokens, ordinal, previous)?;
            if step.inputs != plan.inputs
                || step.positions != plan.positions
                || step.selected_row != plan.selected_row
                || step.choice >= 151_936
                || step.cache_tokens
                    != plan
                        .positions
                        .last()
                        .copied()
                        .ok_or("empty reference step")?
                        + 1
            {
                return Err("reference completed input schedule".into());
            }
            if step.cache_tokens >= 5 {
                generated.push(step.choice);
            }
            previous = Some(step.choice);
        }
        if generated != value.generated_tokens {
            return Err("reference output schedule".into());
        }
        Ok(value)
    }
    pub fn open(path: &Path, expected: &str) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > LIMIT {
            return Err("reference file bound".into());
        }
        let mut bytes = Vec::new();
        file.take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if u64::try_from(bytes.len()).map_err(|_| "reference length")? != metadata.len() {
            return Err("reference length changed".into());
        }
        Self::parse(&bytes, expected)
    }
    pub fn bind(&self, identity: &ModelIdentity, prompt: &[u32; 5]) -> Result<(), String> {
        if &self.identity != identity || &self.prompt_tokens != prompt {
            return Err("reference model/prompt binding".into());
        }
        Ok(())
    }
    pub fn matches(&self, observation: &Observation, bytes: &[u8]) -> bool {
        self.generated_tokens == observation.generated_tokens
            && self.generated_utf8_bytes == bytes
            && self.steps.len() == observation.steps.len()
            && self.steps.iter().zip(&observation.steps).all(|(a, b)| {
                a.inputs == b.inputs
                    && a.positions == b.positions
                    && a.selected_row == b.selected_row
                    && a.choice == b.choice
                    && a.cache_tokens == b.cache_tokens
            })
    }
}

pub fn finish_run(
    observed: Result<bool, String>,
    closed: Result<(), String>,
) -> Result<(), String> {
    match (observed, closed) {
        (Ok(true), Ok(())) => Ok(()),
        (Ok(false), Ok(())) => Err("paged draft differs from the pinned FP32 reference".into()),
        (Err(error), Ok(())) => Err(error),
        (observed, Err(close)) => Err(format!(
            "execution: {observed:?}; worker close failed: {close}"
        )),
    }
}

#[cfg(test)]
#[path = "draft_paged_canary_tests.rs"]
mod tests;
