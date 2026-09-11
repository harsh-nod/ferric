//! Bounded standalone Draft06B diagnostic. No target CLI or fast profile changes.

#![recursion_limit = "256"]

mod tp_worker;

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use ferric_build::{DRAFT_REPOSITORY, DRAFT_REVISION, ModelWeightBinding};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpExecutionV1, EngineeringTpRankTransportV1,
};
use ferric_m1_engineering_execution_v1::tp_model::{
    EngineeringQwenDraftModelV1, EngineeringQwenModelV1,
};
use ferric_spec::{
    ModelConfig, QWEN3_DRAFT_TENSOR_COUNT, QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_NO_LAYER,
    Qwen3ModelRole, Qwen3TensorKind,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tp_worker::Worker;

const CAPACITY: u32 = 32;
const PROMPT_TOKENS: usize = 5;
const OUTPUT_TOKENS: usize = 2;
const STEPS: u32 = 6;
const PACKETS: u64 = 2544;
const MAX_REFERENCE_BYTES: u64 = 65_536;
const USAGE: &str = "ferric-qwen3-draft-canary --source DIR --artifact DIR --worker FILE --worker-sha256 HEX --device-unique-id ID --allow-unauthenticated-machine-code [--prompt TEXT] [--reference FILE --reference-sha256 HEX]";

struct Options {
    source: PathBuf,
    artifact: PathBuf,
    worker: PathBuf,
    worker_sha256: String,
    device: u64,
    prompt: String,
    reference: Option<(PathBuf, String)>,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.peekable();
        let mut seen = BTreeSet::new();
        let (mut source, mut artifact, mut worker, mut worker_sha256) = (None, None, None, None);
        let (mut device, mut reference, mut reference_sha256) = (None, None, None);
        let mut prompt = "The capital of France is".to_owned();
        let mut consent = false;
        while let Some(flag) = arguments.next() {
            if !seen.insert(flag.clone()) {
                return Err(format!("duplicate option {flag}"));
            }
            if flag == "--allow-unauthenticated-machine-code" {
                consent = true;
                continue;
            }
            if flag == "--help" {
                return Err(USAGE.into());
            }
            let value = arguments
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--source" => source = Some(PathBuf::from(value)),
                "--artifact" => artifact = Some(PathBuf::from(value)),
                "--worker" => worker = Some(PathBuf::from(value)),
                "--worker-sha256" => worker_sha256 = Some(checked_sha256(&value)?),
                "--device-unique-id" => {
                    device = Some(
                        value
                            .parse::<u64>()
                            .map_err(|e| format!("device ID: {e}"))?,
                    );
                }
                "--prompt" => prompt = value,
                "--reference" => reference = Some(PathBuf::from(value)),
                "--reference-sha256" => reference_sha256 = Some(checked_sha256(&value)?),
                _ => return Err(format!("unknown option {flag}")),
            }
        }
        if !consent {
            return Err("explicit --allow-unauthenticated-machine-code is required".into());
        }
        let device = device.ok_or("--device-unique-id is required")?;
        if device == 0 || prompt.is_empty() || prompt.len() > 16_384 {
            return Err("draft canary device or prompt bound".into());
        }
        let reference = match (reference, reference_sha256) {
            (Some(path), Some(hash)) => Some((path, hash)),
            (None, None) => None,
            _ => return Err("--reference and --reference-sha256 must be paired".into()),
        };
        Ok(Self {
            source: source.ok_or("--source is required")?,
            artifact: artifact.ok_or("--artifact is required")?,
            worker: worker.ok_or("--worker is required")?,
            worker_sha256: worker_sha256.ok_or("--worker-sha256 is required")?,
            device,
            prompt,
            reference,
        })
    }
}

fn checked_sha256(value: &str) -> Result<String, String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("SHA-256 must be exactly 64 lowercase hexadecimal characters".into());
    }
    Ok(value.to_owned())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 512 * 1024 * 1024 {
        return Err("executable file bound".into());
    }
    let mut hasher = Sha256::new();
    let mut bytes = [0; 65_536];
    let mut total = 0_u64;
    loop {
        let count = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(count).map_err(|_| "read count")?)
            .ok_or("executable size overflow")?;
        if total > metadata.len() {
            return Err("executable grew during hashing".into());
        }
        hasher.update(&bytes[..count]);
    }
    if total != metadata.len() {
        return Err("executable shortened during hashing".into());
    }
    Ok(hex(&hasher.finalize()))
}

fn validate_draft_shape(
    model: ModelConfig,
    weight_bytes: u64,
    section_count: u32,
) -> Result<(), String> {
    model
        .validate()
        .map_err(|e| format!("draft geometry: {e:?}"))?;
    if model.role != Qwen3ModelRole::Draft06B
        || !model.tie_word_embeddings
        || weight_bytes != QWEN3_DRAFT_TENSOR_DATA_BYTES
        || section_count != QWEN3_DRAFT_TENSOR_COUNT
        || u64::from(model.layers) * 15 + 4 != PACKETS / u64::from(STEPS)
    {
        return Err(
            "canonical draft role, tied weights, payload or packet geometry drifted".into(),
        );
    }
    Ok(())
}

fn validate_tied_pair(
    embedding: &[u8],
    head: &[u8],
    embedding_sha: [u8; 32],
    head_sha: [u8; 32],
) -> Result<(), String> {
    if embedding.len() != head.len()
        || embedding_sha != head_sha
        || embedding != head
        || Sha256::digest(embedding).as_slice() != embedding_sha
        || Sha256::digest(head).as_slice() != head_sha
    {
        return Err("authenticated tied embedding/head bytes or digests differ".into());
    }
    Ok(())
}

fn bound_section<'a>(
    weights: &'a [u8],
    binding: ModelWeightBinding<'_>,
) -> Result<&'a [u8], String> {
    let (offset, length) = binding.destination_range();
    let end = offset
        .checked_add(length)
        .ok_or("weight section overflow")?;
    let start = usize::try_from(offset).map_err(|_| "weight offset")?;
    let end = usize::try_from(end).map_err(|_| "weight end")?;
    weights
        .get(start..end)
        .ok_or_else(|| "weight section outside retained payload".into())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelIdentity {
    model_bundle_id: String,
    draft_model_id: String,
    draft_config_id: String,
    draft_weights_sha256: String,
}

fn validate_draft(
    model: &EngineeringQwenModelV1,
    draft: &EngineeringQwenDraftModelV1<'_>,
) -> Result<ModelIdentity, String> {
    let config = draft.config();
    validate_draft_shape(
        config,
        u64::try_from(draft.weights().len()).map_err(|_| "draft length")?,
        draft.layout().section_count(config.role),
    )?;
    let embedding = draft
        .layout()
        .lookup(config.role, Qwen3TensorKind::TokenEmbedding, QWEN3_NO_LAYER)
        .map_err(|e| e.to_string())?;
    let head = draft
        .layout()
        .lookup(
            config.role,
            Qwen3TensorKind::LanguageModelHead,
            QWEN3_NO_LAYER,
        )
        .map_err(|e| e.to_string())?;
    validate_tied_pair(
        bound_section(draft.weights(), embedding)?,
        bound_section(draft.weights(), head)?,
        embedding.sha256(),
        head.sha256(),
    )?;
    Ok(ModelIdentity {
        model_bundle_id: hex(model.bundle_id().as_bytes()),
        draft_model_id: hex(config.model_id.as_bytes()),
        draft_config_id: hex(config.config_id.as_bytes()),
        draft_weights_sha256: digest(draft.weights()),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    schema: String,
    model: String,
    model_revision: String,
    identity: ModelIdentity,
    prompt_tokens: [u32; PROMPT_TOKENS],
    generated_tokens: [u32; OUTPUT_TOKENS],
    generated_utf8_bytes: Vec<u8>,
    producer: String,
}

impl Reference {
    fn parse(bytes: &[u8], expected_sha256: &str) -> Result<Self, String> {
        if bytes.is_empty()
            || bytes.len() > MAX_REFERENCE_BYTES as usize
            || digest(bytes) != expected_sha256
        {
            return Err("reference bytes exceed bounds or differ from external SHA-256".into());
        }
        let value: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("reference schema: {e}"))?;
        if value.schema != "FerricDraftCanaryReferenceV1"
            || value.model != DRAFT_REPOSITORY
            || value.model_revision != DRAFT_REVISION
            || value.producer.is_empty()
            || value.producer.len() > 1024
            || value.generated_utf8_bytes.len() > 16_384
            || value
                .prompt_tokens
                .iter()
                .chain(&value.generated_tokens)
                .any(|&token| token >= 151_936)
        {
            return Err("reference canonical model, token or producer bound".into());
        }
        for hash in [
            &value.identity.model_bundle_id,
            &value.identity.draft_model_id,
            &value.identity.draft_config_id,
            &value.identity.draft_weights_sha256,
        ] {
            checked_sha256(hash)?;
        }
        Ok(value)
    }

    fn open(path: &Path, expected_sha256: &str) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("open reference: {e}"))?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_REFERENCE_BYTES {
            return Err("reference regular-file bound".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_REFERENCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if u64::try_from(bytes.len()).map_err(|_| "reference length")? != metadata.len() {
            return Err("reference file length changed".into());
        }
        Self::parse(&bytes, expected_sha256)
    }

    fn bind(&self, identity: &ModelIdentity, prompt: &[u32]) -> Result<(), String> {
        if self.identity != *identity || self.prompt_tokens != prompt {
            return Err("reference does not bind this authenticated draft and exact prompt".into());
        }
        Ok(())
    }

    fn matches(&self, tokens: &[u32; OUTPUT_TOKENS], bytes: &[u8]) -> bool {
        self.generated_tokens == *tokens && self.generated_utf8_bytes == bytes
    }
}

#[derive(Serialize)]
struct TokenStep {
    position: u32,
    input_token: u32,
    next_token: u32,
    completed_dispatches: u64,
}

fn validate_prompt(prompt: &[u32]) -> Result<(), String> {
    if prompt.len() != PROMPT_TOKENS || prompt.iter().any(|&token| token >= 151_936) {
        return Err("draft canary requires exactly five in-vocabulary prompt tokens".into());
    }
    Ok(())
}

fn run_steps<R: EngineeringTpRankTransportV1>(
    engine: &mut EngineeringTpExecutionV1<R>,
    prompt: &[u32],
) -> Result<([u32; OUTPUT_TOKENS], Vec<TokenStep>), String> {
    validate_prompt(prompt)?;
    if engine.position() != 0 || engine.dispatch_counts() != [0] {
        return Err("draft canary requires a fresh TP1 driver".into());
    }
    let mut rows = Vec::with_capacity(STEPS as usize);
    let mut next = 0;
    let mut generated = [0; OUTPUT_TOKENS];
    for ordinal in 0..STEPS {
        let input_token = if ordinal < PROMPT_TOKENS as u32 {
            prompt[ordinal as usize]
        } else {
            next
        };
        next = engine.step(input_token)?;
        let expected = u64::from(ordinal + 1) * (PACKETS / u64::from(STEPS));
        if engine.position() != ordinal + 1
            || engine.dispatch_counts() != [expected]
            || next >= 151_936
        {
            return Err("draft token/position or exact per-layer dispatch count drifted".into());
        }
        rows.push(TokenStep {
            position: ordinal,
            input_token,
            next_token: next,
            completed_dispatches: expected,
        });
        if ordinal >= PROMPT_TOKENS as u32 - 1 {
            generated[ordinal as usize + 1 - PROMPT_TOKENS] = next;
        }
    }
    Ok((generated, rows))
}

fn emit(value: &impl Serialize) -> Result<(), String> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    output.write_all(b"\n").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

fn finish_run<T>(observed: Result<T, String>, closed: Result<(), String>) -> Result<T, String> {
    match (observed, closed) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(format!("worker teardown failed: {error}")),
        (Err(error), Err(close)) => Err(format!("{error}; worker teardown failed: {close}")),
    }
}

fn run(options: &Options) -> Result<(), String> {
    let reference = options
        .reference
        .as_ref()
        .map(|(path, hash)| Reference::open(path, hash))
        .transpose()?;
    let controller_sha256 = hash_file(Path::new("/proc/self/exe"))?;
    if hash_file(&options.worker)? != options.worker_sha256 {
        return Err("worker differs from external SHA-256".into());
    }
    let roster = ferric_qwen3_tp_kernels_device_v1::compiler_expectation_roster_v1();
    let artifact =
        EngineeringTpArtifactV1::open(&options.artifact, &roster).map_err(|e| e.to_string())?;
    eprintln!("TP-v1 artifact admitted; authenticating and retaining canonical draft payload");
    let model = EngineeringQwenModelV1::open_with_draft(&options.source)?;
    let draft = model
        .draft()
        .ok_or("opt-in intake returned no draft payload")?;
    let identity = validate_draft(&model, &draft)?;
    let prompt = model.encode(&options.prompt)?;
    validate_prompt(&prompt)?;
    if let Some(expected) = &reference {
        expected.bind(&identity, &prompt)?;
    }
    let worker = Worker::spawn(&options.worker, options.device, &artifact)?;
    let pid = worker.pid();
    let running_worker_sha256 = hash_file(&PathBuf::from(format!("/proc/{pid}/exe")))?;
    if running_worker_sha256 != options.worker_sha256 {
        return Err("running worker differs from external SHA-256".into());
    }
    let mut engine = EngineeringTpExecutionV1::new(
        vec![worker],
        draft.config(),
        draft.weights(),
        draft.layout(),
        CAPACITY,
    )?;
    let observed = (|| {
        emit(&serde_json::json!({
            "schema": "FerricDraftCanarySetupV1", "authority": "none", "performance_qualified": false,
            "model": DRAFT_REPOSITORY, "model_revision": DRAFT_REVISION, "model_role": "Draft06B",
            "identity": identity, "dtype": "BF16", "head_precision": "bf16", "target": "gfx950:xnack-",
            "layers": 28, "hidden": 1024, "intermediate": 3072, "query_heads": 16, "kv_heads": 8,
            "head_dimension": 128, "vocabulary": 151936, "tie_word_embeddings": true,
            "tensor_parallel": 1, "capacity": CAPACITY, "prompt": options.prompt, "prompt_tokens": prompt,
            "new_tokens": OUTPUT_TOKENS, "expected_steps": STEPS, "expected_dispatches": PACKETS,
            "draft_payload_bytes": draft.weights().len(), "retained_target_payload_bytes": model.target_weights().len(),
            "draft_kv_payload_bytes": 28_u64 * 2 * u64::from(CAPACITY) * 1024 * 2,
            "worker_pid": pid, "device_unique_id": options.device, "worker_sha256": options.worker_sha256,
            "running_worker_sha256": running_worker_sha256, "controller_sha256": controller_sha256,
            "artifact_hsaco_id": hex(artifact.hsaco_id().as_bytes()),
            "artifact_manifest_id": hex(artifact.manifest_id().as_bytes()),
            "artifact_handoff_id": hex(artifact.handoff_id().as_bytes()),
            "reference_sha256": options.reference.as_ref().map(|(_, hash)| hash),
            "reference_producer": reference.as_ref().map(|value| &value.producer),
            "prefill": "token_at_a_time_m1", "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
            "decoding": "greedy_lowest_id_fixed_length", "speculative_execution": false,
            "numerical_status": "diagnostic_only; reference checked only when externally pinned"
        }))?;
        let (tokens, steps) = run_steps(&mut engine, &prompt)?;
        let bytes = model.decode(&tokens)?;
        let reference_passed = reference
            .as_ref()
            .map(|expected| expected.matches(&tokens, &bytes));
        emit(&serde_json::json!({
            "schema": "FerricDraftCanaryObservationV1", "authority": "none", "performance_qualified": false,
            "steps": steps, "generated_tokens": tokens, "generated_utf8_bytes": bytes,
            "generated_text": String::from_utf8(bytes.clone()).ok(), "reference_passed": reference_passed,
            "rank_dispatch_counts": engine.dispatch_counts(), "kv_tokens_processed": engine.position()
        }))?;
        if reference_passed == Some(false) {
            return Err(
                "generated draft tokens or UTF-8 bytes differ from pinned reference".into(),
            );
        }
        Ok(reference_passed)
    })();
    let closed = engine.close();
    let reference_passed = finish_run(observed, closed)?;
    emit(&serde_json::json!({
        "schema": "FerricDraftCanaryClosedV1", "authority": "none", "performance_qualified": false,
        "execution_completed": true, "reference_passed": reference_passed, "worker_pid": pid,
        "all_workers_exited": true, "rank_dispatch_counts": engine.dispatch_counts(), "kv_tokens_processed": engine.position()
    }))
}

fn main() -> std::process::ExitCode {
    match Options::parse(std::env::args().skip(1)).and_then(|options| run(&options)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Draft canary failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
#[path = "draft_canary_tests.rs"]
mod tests;
