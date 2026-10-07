//! Closed two-round contract; references never supply draft choices or acceptance.

use super::tp_worker::RuntimeOptions;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{
    CorrectionBonusKvDisposition, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, SpeculativeKvInterval, SpeculativeKvRoundIndex,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const K: usize = 4;
pub const ROUNDS: usize = 2;
pub const PROMPT_TOKENS: usize = 128;
pub const PREFIX_TOKENS: usize = PROMPT_TOKENS - 1;
pub const CHUNK: usize = 16;
pub const PREFILL_BATCHES: usize = PREFIX_TOKENS.div_ceil(CHUNK);
pub const CONTEXT: u32 = 160;
pub const PAGES: u32 = 10;
pub const SOURCE_REFERENCE_SHA256: &str =
    "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b";
const FILE_LIMIT: u64 = 65_536;
const VOCABULARY: u32 = 151_936;
const USAGE: &str = "ferric-qwen3-paired-paged-canary --source DIR --target-artifact DIR --target-head-artifact DIR --draft-artifact DIR --worker FILE --worker-sha256 HEX --device-unique-id ID --reference FILE --reference-sha256 HEX --target-reference FILE --allow-unauthenticated-machine-code --runtime-cache-admission --runtime-operational --runtime-rollover";

pub struct Options {
    pub source: PathBuf,
    pub target_artifact: PathBuf,
    pub target_head_artifact: PathBuf,
    pub draft_artifact: PathBuf,
    pub worker: PathBuf,
    pub worker_sha256: String,
    pub device: u64,
    pub reference: PathBuf,
    pub reference_sha256: String,
    pub target_reference: PathBuf,
    pub runtime: RuntimeOptions,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut seen = BTreeSet::new();
        let (mut source, mut target_artifact, mut target_head_artifact, mut draft_artifact) =
            (None, None, None, None);
        let (mut worker, mut worker_sha256, mut device) = (None, None, None);
        let (mut reference, mut reference_sha256, mut target_reference) = (None, None, None);
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
                "--runtime-rollover" => {
                    runtime.rollover = true;
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
                "--target-artifact" => target_artifact = Some(PathBuf::from(value)),
                "--target-head-artifact" => target_head_artifact = Some(PathBuf::from(value)),
                "--draft-artifact" => draft_artifact = Some(PathBuf::from(value)),
                "--worker" => worker = Some(PathBuf::from(value)),
                "--worker-sha256" => worker_sha256 = Some(checked_sha(&value)?),
                "--device-unique-id" => {
                    device = Some(value.parse::<u64>().map_err(|e| e.to_string())?);
                }
                "--reference" => reference = Some(PathBuf::from(value)),
                "--reference-sha256" => reference_sha256 = Some(checked_sha(&value)?),
                "--target-reference" => target_reference = Some(PathBuf::from(value)),
                _ => return Err(format!("unknown option {flag}")),
            }
        }
        if !consent
            || !runtime.cache_admission
            || !runtime.operational
            || !runtime.rollover
            || device.is_none_or(|value| value == 0)
        {
            return Err(
                "explicit consent/device and all three fixed runtime options required".into(),
            );
        }
        Ok(Self {
            source: source.ok_or("source required")?,
            target_artifact: target_artifact.ok_or("target artifact required")?,
            target_head_artifact: target_head_artifact.ok_or("target head required")?,
            draft_artifact: draft_artifact.ok_or("draft artifact required")?,
            worker: worker.ok_or("worker required")?,
            worker_sha256: worker_sha256.ok_or("worker hash required")?,
            device: device.ok_or("device required")?,
            reference: reference.ok_or("reference required")?,
            reference_sha256: reference_sha256.ok_or("reference hash required")?,
            target_reference: target_reference.ok_or("immutable target reference required")?,
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

fn read_bound(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > FILE_LIMIT {
        return Err("reference file bound".into());
    }
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if u64::try_from(bytes.len()).map_err(|_| "reference length")? != metadata.len() {
        return Err("reference length changed".into());
    }
    Ok(bytes)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceReference {
    pub schema: String,
    pub generated_token_ids: Vec<u32>,
    pub generated_utf8_hex: String,
    pub independent_producer: String,
    pub policy: String,
    pub producer_evidence_sha256: String,
    pub producer_source_sha256: String,
    pub prompt_token_ids: Vec<u32>,
    pub target_manifest_sha256: String,
    pub workload_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub schema: String,
    pub source_reference_sha256: String,
    pub prompt_token_ids: Vec<u32>,
    pub generated_token_ids: Vec<u32>,
    pub prefix_utf8_hex: Vec<String>,
    pub tokenizer_sha256: String,
    pub producer_source_sha256: String,
}

impl Reference {
    pub fn open(options: &Options) -> Result<Self, String> {
        let original = read_bound(&options.target_reference)?;
        let adapted = read_bound(&options.reference)?;
        Self::parse(&original, &adapted, &options.reference_sha256)
    }

    pub fn parse(original: &[u8], adapted: &[u8], expected: &str) -> Result<Self, String> {
        let limit = usize::try_from(FILE_LIMIT).map_err(|_| "reference limit")?;
        if original.is_empty()
            || adapted.is_empty()
            || original.len() > limit
            || adapted.len() > limit
            || digest(original) != SOURCE_REFERENCE_SHA256
            || digest(adapted) != checked_sha(expected)?
        {
            return Err("reference byte/hash bound".into());
        }
        let source: SourceReference =
            serde_json::from_slice(original).map_err(|e| e.to_string())?;
        let value: Self = serde_json::from_slice(adapted).map_err(|e| e.to_string())?;
        value.validate_source(&source)?;
        Ok(value)
    }

    pub fn validate_source(&self, source: &SourceReference) -> Result<(), String> {
        if source.schema != "FerricMatched128ReferenceV1"
            || source.policy != "exact-greedy-token-ids-and-decoded-utf8-v1"
            || source.independent_producer
                != "offline-stock-HF-Qwen3-BF16-eager-decoder-explicit-FP32-F.linear-head"
            || self.schema != "FerricPairedPagedK4ReferenceV1"
            || self.source_reference_sha256 != SOURCE_REFERENCE_SHA256
            || self.prompt_token_ids.len() != PROMPT_TOKENS
            || self.generated_token_ids.len() != PROMPT_TOKENS
            || self.prompt_token_ids != source.prompt_token_ids
            || self.generated_token_ids != source.generated_token_ids
            || self
                .prompt_token_ids
                .iter()
                .chain(&self.generated_token_ids)
                .any(|&t| t >= VOCABULARY)
            || self.prefix_utf8_hex.len() != ROUNDS * (K + 1)
        {
            return Err("closed reference identity/token contract".into());
        }
        for hash in [
            &self.tokenizer_sha256,
            &self.producer_source_sha256,
            &source.producer_evidence_sha256,
            &source.producer_source_sha256,
            &source.target_manifest_sha256,
            &source.workload_sha256,
        ] {
            checked_sha(hash)?;
        }
        let mut previous = "";
        for prefix in &self.prefix_utf8_hex {
            if prefix.is_empty()
                || prefix.len() > 32_768
                || !prefix.len().is_multiple_of(2)
                || !prefix
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || !prefix.starts_with(previous)
                || !source.generated_utf8_hex.starts_with(prefix)
            {
                return Err("independent UTF-8 prefix extent/chain".into());
            }
            previous = prefix;
        }
        Ok(())
    }

    pub fn matches(&self, tokens: &[u32], bytes: &[u8]) -> bool {
        (ROUNDS..=ROUNDS * (K + 1)).contains(&tokens.len())
            && self.generated_token_ids.get(..tokens.len()) == Some(tokens)
            && self
                .prefix_utf8_hex
                .get(tokens.len() - 1)
                .is_some_and(|expected| *expected == hex(bytes))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct PacketBudget {
    pub target_prefill: u64,
    pub target_verify: u64,
    pub draft_prefill: u64,
    pub draft_proposal: u64,
    pub draft_catch_up: u64,
    pub target_total: u64,
    pub draft_total_max: u64,
}

impl PacketBudget {
    pub fn new(
        target_empty: &[u64],
        target_choices: &[u64],
        draft_empty: &[u64],
        draft_choice: &[u64],
    ) -> Result<Self, String> {
        if target_empty != [613]
            || target_choices != [616]
            || draft_empty != [477]
            || draft_choice != [480]
        {
            return Err("fixed role/profile packet counts differ".into());
        }
        let limit = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1;
        if limit < target_choices[0] || limit < draft_choice[0] {
            return Err("one complete forward exceeds queue budget".into());
        }
        let prefills = u64::try_from(PREFILL_BATCHES).map_err(|_| "prefill count")?;
        let rounds = u64::try_from(ROUNDS).map_err(|_| "round count")?;
        let proposals = u64::try_from(ROUNDS * K).map_err(|_| "proposal count")?;
        let target_total = prefills
            .checked_mul(target_empty[0])
            .and_then(|n| n.checked_add(rounds * target_choices[0]))
            .ok_or("target packet overflow")?;
        let draft_total_max = prefills
            .checked_add(rounds)
            .and_then(|n| n.checked_mul(draft_empty[0]))
            .and_then(|n| n.checked_add(proposals * draft_choice[0]))
            .ok_or("draft packet overflow")?;
        Ok(Self {
            target_prefill: target_empty[0],
            target_verify: target_choices[0],
            draft_prefill: draft_empty[0],
            draft_proposal: draft_choice[0],
            draft_catch_up: draft_empty[0],
            target_total,
            draft_total_max,
        })
    }
}

pub fn bootstrap_index(anchor: u32, plan_id: Identity) -> Result<SpeculativeKvRoundIndex, String> {
    let cursor = u32::try_from(PREFIX_TOKENS).map_err(|_| "prefix bound")?;
    let width = u8::try_from(K).map_err(|_| "K bound")?;
    let selection = |role| Qwen3PlanSelection {
        role,
        mode: Qwen3ExecutionMode::Speculative,
        bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
    };
    let mut target_commit_ends = [0; 17];
    let mut draft_commit_ends = [0; 17];
    for accepted in 0..=width {
        target_commit_ends[usize::from(accepted)] = cursor + u32::from(accepted) + 1;
        draft_commit_ends[usize::from(accepted)] = cursor + u32::from((accepted + 1).min(width));
    }
    let index = SpeculativeKvRoundIndex {
        request: RequestId::new(1, 1),
        completion_epoch: CompletionEpoch::new(1),
        plan_id,
        target_selection: selection(Qwen3ModelRole::Target8B),
        draft_selection: selection(Qwen3ModelRole::Draft06B),
        draft_token_count: width,
        round_anchor: anchor,
        // Constructor-only untrusted slots; reserve_proposals replaces every live slot.
        // These values never enter either driver or the evidence stream.
        draft_tokens: [0; 16],
        target_pre_committed: cursor,
        draft_pre_committed: cursor,
        target_tentative: SpeculativeKvInterval {
            start: cursor,
            end: cursor + u32::from(width) + 1,
        },
        draft_tentative: SpeculativeKvInterval {
            start: cursor,
            end: cursor + u32::from(width),
        },
        target_commit_ends,
        draft_commit_ends,
        correction_bonus: CorrectionBonusKvDisposition::DeferredUntilNextStep,
    };
    index
        .validate()
        .map_err(|e| format!("initial index: {e:?}"))?;
    Ok(index)
}

pub fn finish_pair(
    observed: Result<bool, String>,
    target_close: Result<(), String>,
    draft_close: Result<(), String>,
) -> Result<(), String> {
    match (observed, target_close, draft_close) {
        (Ok(true), Ok(()), Ok(())) => Ok(()),
        (Ok(false), Ok(()), Ok(())) => {
            Err("paired output differs from the frozen target prefix".into())
        }
        (Err(error), Ok(()), Ok(())) => Err(error),
        (observed, target, draft) => Err(format!(
            "execution: {observed:?}; target close: {target:?}; draft close: {draft:?}"
        )),
    }
}

#[cfg(test)]
#[path = "paired_paged_canary_tests.rs"]
mod tests;
