//! Fresh Prefix22/284 owners with the same MLP548 and one authentic layer-zero input.
use super::{FilePin, ImagePins, Images, Result, hash, process, require, setup};
use crate::tp_execution::batched::EngineeringTp2FiniteSourceRecorderV1;
use crate::tp_execution::{
    EngineeringTp2Finite2304MetadataV1, EngineeringTp2GraphGeometryV1, EngineeringTp2GraphInputV1,
};
use crate::tp_model::EngineeringQwenModelV1;
use crate::tp_paged::{
    EngineeringTpPageRowV1, EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1,
    EngineeringTpPoolScopeV1, EngineeringTpPreparedBatchV1,
};
use crate::{finite_prefix_layer_wire_v1 as wire, finite_setup_wire_v1 as setup_wire};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};
mod capture;
mod evidence;
mod provenance;
pub use capture::{CaptureConfig, CaptureObservation, run_capture};

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for b in bytes {
        write!(&mut out, "{b:02x}").expect("String formatting");
    }
    out
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptPins {
    pub manifest: FilePin,
    pub text: FilePin,
    pub tokens: FilePin,
}
impl PromptPins {
    fn validate(&self) -> Result<()> {
        require(
            self.manifest.bytes == 21318
                && hex(&self.manifest.sha256)
                    == "30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600"
                && self.text.bytes == 11224
                && hex(&self.text.sha256)
                    == "a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a"
                && self.tokens.bytes == 8192
                && hex(&self.tokens.sha256)
                    == "2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02",
            "layer exact authentic raw prompt pins",
        )
    }
    fn read(&self) -> Result<(String, Vec<u32>)> {
        self.validate()?;
        let manifest = self.manifest.read(32 << 10, true)?;
        let text = String::from_utf8(self.text.read(16 << 10, true)?).map_err(|e| e.to_string())?;
        let tokens = self
            .tokens
            .read(8192, true)?
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>();
        let m: serde_json::Value = serde_json::from_slice(&manifest).map_err(|e| e.to_string())?;
        require(
            m["schema"] == "FerricQwen3LongPromptV1"
                && m["revision"] == "b968826d9c46dd6066d109eabc6255188de91218"
                && m["input_tokens"] == 2048
                && m["output_tokens"] == 256
                && m["add_special_tokens"] == false
                && m["chat_template"].is_null()
                && m["round_trip_verified"] == true
                && m["input_token_ids"]
                    == serde_json::to_value(&tokens).map_err(|e| e.to_string())?
                && tokens.len() == 2048
                && tokens[0] == 9112
                && tokens.iter().all(|v| *v < 151936),
            "layer prompt manifest/tokens",
        )?;
        Ok((text, tokens))
    }
    fn recheck(&self) -> Result<()> {
        self.manifest.read(32 << 10, false)?;
        self.text.read(16 << 10, false)?;
        self.tokens.read(8192, false)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub source: PathBuf,
    pub worker: FilePin,
    pub images: ImagePins,
    pub expected_bundle_id: [u8; 32],
    pub expected_model_id: [u8; 32],
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub prompt: PromptPins,
    pub prefix_tiles_image: FilePin,
    pub mlp_tiles_image: FilePin,
    pub evidence_directory: PathBuf,
    pub dispatch_timeout_ms: u32,
    pub child_deadline_ms: u64,
}
impl Config {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65536,
            "layer request bound",
        )?;
        let c: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        c.validate()?;
        Ok(c)
    }
    fn validate(&self) -> Result<()> {
        self.validate_schema("FerricFinitePrefixLayerComparisonRequestV1")
    }
    fn validate_schema(&self, schema: &str) -> Result<()> {
        self.prompt.validate()?;
        require(
            self.schema == schema
                && self.source.is_absolute()
                && self.expected_bundle_id != [0; 32]
                && self.expected_model_id != [0; 32]
                && self.session != [0; 32]
                && self.device_ids[0] != 0
                && self.device_ids[1] != 0
                && self.device_ids[0] != self.device_ids[1]
                && (1..=10000).contains(&self.dispatch_timeout_ms)
                && (1000..=3600000).contains(&self.child_deadline_ms)
                && self.evidence_directory.is_absolute()
                && self.evidence_directory.file_name().is_some()
                && serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 16384,
            "layer request identity/bounds",
        )?;
        for p in [&self.prefix_tiles_image, &self.mlp_tiles_image] {
            require(
                p.path.is_absolute() && p.bytes > 0 && p.bytes <= 32 << 20 && p.sha256 != [0; 32],
                "layer actual image pin",
            )?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct StageMatch {
    pub rank: u32,
    pub stage: &'static str,
    pub elements: usize,
    pub element_bytes: usize,
    pub baseline_sha256: [u8; 32],
    pub candidate_sha256: [u8; 32],
    pub bit_mismatches: usize,
}
#[derive(Serialize)]
pub struct RunRecord {
    pub child_pid: u32,
    pub bootstrap: wire::Bootstrap,
    pub profile_sha256: [u8; 32],
    pub setup_commands: u64,
    pub close: wire::Response,
    pub child_exit_zero: bool,
    pub process_group_absent: bool,
}
#[derive(Serialize)]
pub struct Observation {
    pub schema: &'static str,
    pub request: Config,
    pub runs: [RunRecord; 2],
    pub stages: Vec<StageMatch>,
    pub files: Vec<evidence::File>,
    pub bitwise_equal: bool,
    pub completed_layers_per_run: u32,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
    pub full_forward: bool,
}
struct Retained {
    source: provenance::Source,
    record: RunRecord,
    capture: Vec<u8>,
}

fn metadata(
    batch: &EngineeringTpPreparedBatchV1,
    program: [u8; 32],
    theta: u32,
) -> Result<wire::Input> {
    require(
        batch.rows().len() == 1
            && theta == 1000000
            && batch.context_tokens() == 2304
            && batch.physical_page_count() == 144
            && batch.page_table_stride() == 144,
        "layer authentic pool geometry",
    )?;
    let row = &batch.rows()[0];
    require(
        row.position() == 0 && row.physical_pages().len() == 1,
        "layer exact fresh first page",
    )?;
    let mut pages = vec![u32::MAX; 144];
    pages[0] = row.physical_pages()[0];
    let (mut rotary, sin) = crate::tp_execution::rope_bytes(0, theta);
    rotary.extend(sin);
    let m = EngineeringTp2Finite2304MetadataV1::prepare(&EngineeringTp2GraphInputV1 {
        geometry: EngineeringTp2GraphGeometryV1::Long2304,
        plan_sha256: program,
        generation: 1,
        epoch: 0,
        token: row.token(),
        position: 0,
        page_table: pages,
        cos_sin: rotary,
    })?;
    let input = wire::Input {
        generation: 1,
        token: row.token(),
        cache_metadata: m.cache_metadata().to_vec(),
        rotary_bits: m.rotary().iter().map(|v| v.to_bits()).collect(),
    };
    input.validate().map_err(|e| e.to_string())?;
    Ok(input)
}
fn untouched_kv(input: &wire::Input, capture: &[u8]) -> Result<()> {
    input.validate().map_err(|e| e.to_string())?;
    let rows = wire::capture_rows(capture).map_err(|e| e.to_string())?;
    let start = input.cache_metadata[1] as usize * 16 * 512 * 2;
    let end = start + 512 * 2;
    for rank in 0..2 {
        for stage in [3, 4] {
            let row = rows[rank * 14 + stage];
            require(
                row[..start].iter().chain(&row[end..]).all(|v| *v == 0),
                "layer wrote outside current KV slot",
            )?;
        }
    }
    Ok(())
}
fn compare(a: &[u8], b: &[u8]) -> Result<Vec<StageMatch>> {
    let a = wire::capture_rows(a).map_err(|e| e.to_string())?;
    let b = wire::capture_rows(b).map_err(|e| e.to_string())?;
    Ok(a.iter()
        .zip(b)
        .enumerate()
        .map(|(i, (a, b))| {
            let (stage, bytes, width) = wire::STAGES[i % 14];
            StageMatch {
                rank: (i / 14) as u32,
                stage,
                elements: bytes / width,
                element_bytes: width,
                baseline_sha256: hash(a),
                candidate_sha256: hash(b),
                bit_mismatches: a
                    .chunks_exact(width)
                    .zip(b.chunks_exact(width))
                    .filter(|(a, b)| a != b)
                    .count(),
            }
        })
        .collect())
}
fn response(request: &wire::Request, b: &wire::Bootstrap, r: &wire::Response) -> Result<()> {
    r.validate().map_err(|e| e.to_string())?;
    require(
        request.id == r.id && request.profile_sha256 == r.profile_sha256 && r.profile == b.profile,
        "layer response actual request/profile",
    )
}

fn run_one(
    config: &Config,
    model: &EngineeringQwenModelV1,
    images: &Images,
    mlp: &[u8],
    prefix: Option<&[u8]>,
    token: u32,
    deadline: Instant,
    previous: Option<&Retained>,
    evidence: &mut evidence::Evidence,
) -> Result<Retained> {
    let profile = if prefix.is_some() {
        wire::Profile::Prefix284Mlp548
    } else {
        wire::Profile::Baseline22Mlp548
    };
    let label = if prefix.is_some() {
        "candidate"
    } else {
        "baseline"
    };
    let scope = EngineeringTpPoolScopeV1 {
        model: config.expected_model_id,
        session: config.session,
    };
    let limits =
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).map_err(|e| format!("{e:?}"))?;
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).map_err(|e| format!("{e:?}"))?;
    let mut command = Command::new(&config.worker.path);
    command
        .args([
            "--engineering-native-prefix-layer-v1",
            "--allow-unauthenticated-machine-code",
            "--devices",
        ])
        .arg(format!("{},{}", config.device_ids[0], config.device_ids[1]))
        .arg("--timeout-ms")
        .arg(config.dispatch_timeout_ms.to_string())
        .arg("--profile")
        .arg(if prefix.is_some() {
            "prefix284-mlp548"
        } else {
            "baseline22-mlp548"
        })
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C");
    let mut child = process::OwnedChild::spawn_command(command, deadline)?;
    let pid = child.id();
    eprintln!("finite prefix layer {label} child pid={pid} pgid={pid}; setup not acknowledged");
    config.worker.read(512 << 20, false)?;
    // Recorder authentication requires an empty pool; the first reservation is
    // made afterwards, before the child opens a native Group.
    let recorder = EngineeringTp2FiniteSourceRecorderV1::new(model, &pool, pid)?;
    let sequence = pool
        .open_sequence(scope, &[token], 0)
        .map_err(|e| format!("{e:?}"))?;
    require(
        sequence.hit_tokens() == 0 && sequence.physical_pages().is_empty(),
        "layer fresh source history",
    )?;
    let batch = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence: sequence.sequence(),
            token,
            position: 0,
        }])
        .map_err(|e| format!("{e:?}"))?;
    pool.begin_submission(&batch)
        .map_err(|e| format!("{e:?}"))?;
    let mut sent = wire::Budget::new();
    let mut received = wire::Budget::new();
    let outcome = recorder.with_plan(|composition, source, uploads| {
        let registration = composition.wire_registration_v1()?;
        let program = composition.source_program_snapshot_v1()?;
        let head = source.head_transpose_upload()?;
        let manifest = setup::manifest(
            uploads,
            setup_wire::TailHeadTranspose {
                source: setup_wire::Key::Source {
                    rank: 0,
                    id: head.source_id(),
                },
                source_sha256: head.source_sha256(),
                bytes: head.bytes() as u64,
                sha256: head.sha256(),
            },
        )?;
        let source = provenance::Source::new(registration, program, manifest, pid)?;
        require(
            source.registration.bundle_id == config.expected_bundle_id
                && source.registration.model_id == config.expected_model_id
                && source.registration.session == config.session,
            "layer source requested scope",
        )?;
        if let Some(old) = previous {
            old.source.same_input_source(&source)?;
        }
        let input = metadata(&batch, hash(&source.raw_program), model.config().rope_theta)?;
        if let Some(old) = previous {
            require(
                old.record.bootstrap.input == input,
                "layer authentic input/pages/rotary differ",
            )?;
        }
        let reg = serde_json::to_vec(&source.registration).map_err(|e| e.to_string())?;
        let uploads_raw = serde_json::to_vec(&source.manifest).map_err(|e| e.to_string())?;
        let begin = setup_wire::Begin {
            scope: setup::scope(&source.registration),
            registration: wire::part(&reg),
            source_program: wire::part(&source.raw_program),
            uploads: wire::part(&uploads_raw),
            prefix_image: wire::part(&images.prefix),
            mlp_image: wire::part(&images.mlp),
            residual_image: wire::part(&images.residual),
            tail_image: Some(wire::part(&images.tail)),
        };
        let b = wire::Bootstrap {
            protocol: wire::PROTOCOL,
            profile,
            device_ids: config.device_ids,
            timeout_ms: config.dispatch_timeout_ms,
            begin,
            input,
            mlp_image: wire::part(mlp),
            prefix_image: prefix.map(wire::part),
        };
        let profile_sha256 = b.sha256().map_err(|e| e.to_string())?;
        evidence.append(&format!("{label}-registration.json"), &reg, 4 << 20)?;
        evidence.append(
            &format!("{label}-program.json"),
            &source.raw_program,
            4 << 20,
        )?;
        evidence.append(&format!("{label}-uploads.json"), &uploads_raw, 4 << 20)?;
        evidence.json(&format!("{label}-bootstrap.json"), &b, wire::HEADER_LIMIT)?;
        child.check_deadline()?;
        wire::write_bootstrap(
            child.input.as_mut().ok_or("closed layer stdin")?,
            &mut sent,
            &b,
            mlp,
            prefix,
        )
        .map_err(|e| e.to_string())?;
        let mut stream = setup::Stream::begin(
            &mut child,
            config.device_ids,
            &source.registration,
            &source.raw_program,
            &source.manifest,
            images,
        )?;
        for upload in uploads {
            stream.upload(upload)?;
        }
        for key in setup::mutable_keys(&source.registration)? {
            stream.zero(key)?;
        }
        stream.allocate_tail()?;
        let mut total = 0;
        let mut digest = Sha256::new();
        head.visit_chunks(
            crate::finite_composition_wire::MAX_TRANSFER,
            |offset, bytes| {
                require(offset == total, "layer head contiguous recipe")?;
                stream.write_tail(offset, bytes)?;
                total = total
                    .checked_add(bytes.len())
                    .ok_or("layer head extent overflow")?;
                digest.update(bytes);
                Ok(())
            },
        )?;
        let actual: [u8; 32] = digest.finalize().into();
        require(
            total == head.bytes() && actual == head.sha256(),
            "layer original head recipe changed",
        )?;
        let setup_commands = stream.seal()?;
        let mut close = None;
        let mut capture = Vec::new();
        for (id, command) in [(1, wire::Command::Run), (2, wire::Command::Close)] {
            child.check_deadline()?;
            let request = wire::Request {
                protocol: wire::PROTOCOL,
                id,
                profile_sha256,
                command,
            };
            evidence.json(
                &format!("{label}-request-{id}.json"),
                &request,
                wire::HEADER_LIMIT,
            )?;
            wire::write_request(
                child.input.as_mut().ok_or("closed layer request stdin")?,
                &mut sent,
                &request,
            )
            .map_err(|e| e.to_string())?;
            let (r, body) = wire::read_response(
                child.output.as_mut().ok_or("closed layer stdout")?,
                &mut received,
            )
            .map_err(|e| e.to_string())?;
            response(&request, &b, &r)?;
            child.check_deadline()?;
            evidence.json(
                &format!("{label}-response-{id}.json"),
                &r,
                wire::HEADER_LIMIT,
            )?;
            if id == 2 {
                close = Some(r);
                capture = body;
            }
        }
        Ok((
            source,
            b,
            profile_sha256,
            setup_commands,
            close.ok_or("layer Close missing")?,
            capture,
        ))
    });
    let (source, bootstrap, profile_sha256, setup_commands, close, capture) = match outcome {
        Ok(v) => v,
        Err(e) => {
            pool.quarantine_batch(&batch)
                .map_err(|q| format!("layer quarantine {q:?}; {e}"))?;
            return Err(e);
        }
    };
    // finish consumes the child only after its checked Close, and refuses any
    // residual group, nonzero exit, trailing stdout or failed deadline/cleanup.
    let stderr = child.finish()?;
    untouched_kv(&bootstrap.input, &capture)?;
    // This is one layer, not a whole-token cache commit. The host reservation is
    // never published as reusable model history, even after both ranks close.
    pool.quarantine_batch(&batch)
        .map_err(|e| format!("{e:?}"))?;
    evidence.append(
        &format!("{label}-capture.bin"),
        &capture,
        wire::CAPTURE_BYTES,
    )?;
    evidence.append(&format!("{label}-stderr.bin"), &stderr, 2 << 20)?;
    Ok(Retained {
        source,
        capture,
        record: RunRecord {
            child_pid: pid,
            bootstrap,
            profile_sha256,
            setup_commands,
            close,
            child_exit_zero: true,
            process_group_absent: true,
        },
    })
}

/// Explicit engineering comparison only. Each route closes and is reaped before
/// the next route starts. Uncertain errors are fatal, with no Close retry.
pub fn run(config: Config, allow_unauthenticated_machine_code: bool) -> Result<Observation> {
    require(
        allow_unauthenticated_machine_code,
        "layer explicit engineering opt-in required",
    )?;
    config.validate()?;
    config.worker.read(512 << 20, false)?;
    let images = config.images.read()?;
    let prefix = config.prefix_tiles_image.read(32 << 20, true)?;
    let mlp = config.mlp_tiles_image.read(32 << 20, true)?;
    let (text, tokens) = config.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&config.source)?;
    require(
        *model.bundle_id().as_bytes() == config.expected_bundle_id
            && *model.config().model_id.as_bytes() == config.expected_model_id,
        "layer original model/bundle mismatch",
    )?;
    require(
        model.encode_with_limits(
            &text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == tokens,
        "layer original tokenizer mismatch",
    )?;
    let mut evidence = evidence::Evidence::create(&config.evidence_directory)?;
    evidence.json("request.json", &config, 16384)?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(config.child_deadline_ms))
        .ok_or("layer deadline overflow")?;
    let baseline = run_one(
        &config,
        &model,
        &images,
        &mlp,
        None,
        tokens[0],
        deadline,
        None,
        &mut evidence,
    )?;
    let candidate = run_one(
        &config,
        &model,
        &images,
        &mlp,
        Some(&prefix),
        tokens[0],
        deadline,
        Some(&baseline),
        &mut evidence,
    )?;
    let stages = compare(&baseline.capture, &candidate.capture)?;
    evidence.json("parity.json", &stages, wire::HEADER_LIMIT)?;
    let equal = stages.iter().all(|v| v.bit_mismatches == 0);
    config.worker.read(512 << 20, false)?;
    config.images.read()?;
    config.prefix_tiles_image.read(32 << 20, false)?;
    config.mlp_tiles_image.read(32 << 20, false)?;
    config.prompt.recheck()?;
    let observation = Observation {
        schema: "FerricFinitePrefixLayerComparisonObservationV1",
        request: config,
        runs: [baseline.record, candidate.record],
        stages,
        files: evidence.files(),
        bitwise_equal: equal,
        completed_layers_per_run: 1,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_forward: false,
    };
    evidence.finish(&observation)?;
    require(
        equal,
        "layer exact baseline/candidate parity failed; mismatch rows and closed captures retained",
    )?;
    Ok(observation)
}
#[cfg(test)]
#[path = "prefix_layer/tests.rs"]
mod tests;
