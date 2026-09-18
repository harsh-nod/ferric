//! Optional, fail-closed reference gate for the existing BF16 target-only runner.

use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const REFERENCE_SHA256: &str =
    "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094";
pub const TARGET_REVISION: &str = "b968826d9c46dd6066d109eabc6255188de91218";
pub const PROMPT: &str = "The capital of France is";
const PROMPT_TOKENS: [u32; 5] = [785, 6722, 315, 9625, 374];
const REFERENCE_BYTES_LIMIT: u64 = 131_072;
const PREFIX_TOKENS: [u32; 2] = [12095, 13];
const PREFIX_BYTES: &[u8] = b" Paris.";

#[derive(Deserialize)]
struct Document {
    format: String,
    authority: String,
    model: Model,
    prompt: Prompt,
    execution: Execution,
}

#[derive(Deserialize)]
struct Model {
    repository: String,
    revision: String,
    deployment_bundle_identity: String,
    class: String,
    config: Config,
}

#[derive(Deserialize)]
struct Config {
    hidden_size: u32,
    model_type: String,
    num_attention_heads: u32,
    num_hidden_layers: u32,
    num_key_value_heads: u32,
    torch_dtype: String,
    vocab_size: u32,
}

#[derive(Deserialize)]
struct Prompt {
    add_special_tokens: bool,
    text: String,
    token_ids: Vec<u32>,
}

#[derive(Deserialize)]
struct Execution {
    attention_implementation: String,
    device_dtype: String,
    do_sample: bool,
    max_new_tokens: u32,
    model_forward: String,
    network: String,
    num_beams: u32,
    output_loading_info: LoadingInfo,
    passes: Vec<Pass>,
    use_cache: bool,
}

#[derive(Deserialize)]
struct LoadingInfo {
    error_msgs: Vec<serde_json::Value>,
    mismatched_keys: Vec<serde_json::Value>,
    missing_keys: Vec<serde_json::Value>,
    unexpected_keys: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct Pass {
    argmax_token_ids: Vec<u32>,
    token_ids: Vec<u32>,
    decoded_new_tokens: String,
}

pub struct Reference {
    bundle_id: String,
    pub tokens: Vec<u32>,
    pub bytes: Vec<u8>,
}

/// Full decoded bytes are frozen only for all 32 tokens; the existing smoke
/// comparator separately fixes the two-token prefix. Do not invent other byte
/// prefixes by decoding the candidate's output with its own tokenizer.
pub fn validate_profile_options(
    reference: Option<&Path>,
    digest: Option<&str>,
    world: usize,
    prompt: &str,
    new_tokens: u32,
    runtime_enabled: bool,
) -> Result<bool, String> {
    match (reference, digest) {
        (None, None) if !runtime_enabled => Ok(false),
        (Some(_), Some(REFERENCE_SHA256))
            if world == 1 && prompt == PROMPT && matches!(new_tokens, 2 | 32) =>
        {
            Ok(true)
        }
        _ => Err("target profile requires the pinned --reference/--reference-sha256 pair, one GPU, canonical prompt, and exactly 2 or 32 output tokens; runtime ablations require that profile".into()),
    }
}

impl Reference {
    pub fn open(path: &Path, expected_digest: &str, new_tokens: u32) -> Result<Self, String> {
        if expected_digest != REFERENCE_SHA256 {
            return Err(
                "target reference digest must equal the independently pinned identity".into(),
            );
        }
        let file = std::fs::File::open(path).map_err(|e| format!("reference open: {e}"))?;
        let metadata = file
            .metadata()
            .map_err(|e| format!("reference stat: {e}"))?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > REFERENCE_BYTES_LIMIT {
            return Err("target reference extent or file type".into());
        }
        let mut bytes = Vec::new();
        file.take(REFERENCE_BYTES_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("reference read: {e}"))?;
        if bytes.len() as u64 != metadata.len()
            || super::hex(&Sha256::digest(&bytes)) != expected_digest
        {
            return Err("target reference content identity mismatch".into());
        }
        Self::from_document(
            serde_json::from_slice(&bytes).map_err(|e| format!("reference JSON: {e}"))?,
            new_tokens,
        )
    }

    fn from_document(document: Document, new_tokens: u32) -> Result<Self, String> {
        let model = document.model;
        let config = model.config;
        let execution = document.execution;
        let loading = execution.output_loading_info;
        if document.format != "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1"
            || document.authority != "independent-offline-reference-only"
            || model.repository != "Qwen/Qwen3-8B"
            || model.revision != TARGET_REVISION
            || model.class != "Qwen3ForCausalLM"
            || config.model_type != "qwen3"
            || config.torch_dtype != "torch.bfloat16"
            || (
                config.hidden_size,
                config.num_attention_heads,
                config.num_hidden_layers,
                config.num_key_value_heads,
                config.vocab_size,
            ) != (4096, 32, 36, 8, 151_936)
            || document.prompt.text != PROMPT
            || document.prompt.token_ids != PROMPT_TOKENS
            || document.prompt.add_special_tokens
            || execution.attention_implementation != "sdpa"
            || execution.device_dtype != "bfloat16"
            || execution.model_forward != "transformers.generate"
            || execution.network != "offline"
            || execution.do_sample
            || execution.num_beams != 1
            || execution.max_new_tokens != 32
            || !execution.use_cache
            || !loading.error_msgs.is_empty()
            || !loading.mismatched_keys.is_empty()
            || !loading.missing_keys.is_empty()
            || !loading.unexpected_keys.is_empty()
            || execution.passes.len() != 2
            || !matches!(new_tokens, 2 | 32)
        {
            return Err("target reference workload contract mismatch".into());
        }
        let first = &execution.passes[0];
        if first.token_ids.len() != 32
            || first.token_ids.iter().any(|&id| id >= config.vocab_size)
            || first.token_ids[..2] != PREFIX_TOKENS
            || !first
                .decoded_new_tokens
                .as_bytes()
                .starts_with(PREFIX_BYTES)
            || execution.passes.iter().any(|pass| {
                pass.token_ids != first.token_ids
                    || pass.argmax_token_ids != first.token_ids
                    || pass.decoded_new_tokens != first.decoded_new_tokens
            })
        {
            return Err("independent target reference passes disagree".into());
        }
        Ok(Self {
            bundle_id: model.deployment_bundle_identity,
            tokens: first.token_ids[..new_tokens as usize].to_vec(),
            bytes: if new_tokens == 2 {
                PREFIX_BYTES.to_vec()
            } else {
                first.decoded_new_tokens.as_bytes().to_vec()
            },
        })
    }

    pub fn bind(&self, bundle_id: &str, prompt: &[u32]) -> Result<(), String> {
        if bundle_id != self.bundle_id || prompt != PROMPT_TOKENS {
            return Err(
                "admitted target model or prompt differs from independent reference".into(),
            );
        }
        Ok(())
    }

    pub fn check_token(&self, ordinal: usize, token: u32) -> Result<(), String> {
        if self.tokens.get(ordinal) != Some(&token) {
            return Err(format!(
                "target output {ordinal} differs from independent reference"
            ));
        }
        Ok(())
    }

    pub fn check_output(&self, tokens: &[u32], bytes: &[u8]) -> Result<(), String> {
        if tokens != self.tokens || bytes != self.bytes {
            return Err("target generated tokens/bytes differ from independent reference".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "target_decode_profile_tests.rs"]
mod tests;
