//! Opt-in TP1 final-stage bytes. Transport completion is not numerical parity.

use super::{EngineeringTpExecutionV1, EngineeringTpRankTransportV1, TpResult};
use crate::hex;
use ferric_spec::{FiniteBf16ArgmaxError, Qwen3ModelRole, select_lowest_finite_bf16_argmax};
use rustix::fs::{Mode, OFlags, RenameFlags};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{DirBuilder, File};
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

const WIDTHS: [usize; 3] = [4096, 4096, 151_936];
const FILES: [&str; 3] = ["residual.bf16", "normalized.bf16", "logits.bf16"];
const MAX_BYTES: usize = 4 * 1024 * 1024;

pub(super) struct Readback {
    pub(super) epoch: u64,
    pub(super) position: u32,
    pub(super) token: u32,
    pub(super) choice: u32,
    pub(super) payloads: [Vec<u8>; 3],
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    pub(super) fn final_stage_step(&mut self, token: u32) -> TpResult<Readback> {
        let model = self.plan.model();
        if self.closed
            || self.plan.world_size() != 1
            || self.ranks.len() != 1
            || self.transports.len() != 1
            || self.row_capacity != 1
            || model.role != Qwen3ModelRole::Target8B
            || model.hidden_size != 4096
            || model.vocabulary_size != 151_936
            || self.sequences.is_some()
            || self.ordered_batches.is_some()
        {
            return Err("final-stage readback requires open single-row target TP1".into());
        }
        let tensors = [
            self.ranks[0].hidden,
            self.ranks[0].normalized,
            self.ranks[0].logits,
        ];
        if tensors.iter().zip(WIDTHS).any(|(tensor, width)| {
            tensor.id == 0 || tensor.element_bytes != 2 || tensor.elements != width
        }) {
            return Err("final-stage tensor extent drift".into());
        }
        let epoch = self.sequence.epoch();
        let position = self.position();
        let choice = self.step(token)?;
        let mut payloads = WIDTHS.map(|width| vec![0; width * 2]);
        for (tensor, bytes) in tensors.iter().zip(&mut payloads) {
            if let Err(error) = self.transports[0].read(tensor.id, 0, bytes) {
                self.sequence.poison();
                return Err(format!("completed final-stage readback: {error}"));
            }
        }
        Ok(Readback {
            epoch,
            position,
            token,
            choice,
            payloads,
        })
    }
}

/// Owns a fresh directory and a bounded, predeclared sequence capture.
/// All captured values are Contracted observations, not qualified arithmetic.
pub struct EngineeringTpFinalStageCaptureV1 {
    directory: File,
    files: [File; 3],
    hashes: [Sha256; 3],
    setup: Value,
    positions: Vec<u32>,
    required: u32,
    epoch: Option<u64>,
    tokens: Vec<u32>,
    choices: Vec<u32>,
    rows: Vec<Value>,
    bytes: usize,
    failed: bool,
    finished: bool,
}

impl EngineeringTpFinalStageCaptureV1 {
    /// Validates selection before model execution or output creation.
    /// # Errors
    /// Rejects empty, duplicate, unordered or unreachable positions.
    pub fn validate_positions(positions: &[u32], required: u32) -> TpResult<()> {
        if positions.is_empty()
            || positions.len() > 8
            || !(1..=8192).contains(&required)
            || positions.windows(2).any(|pair| pair[0] >= pair[1])
            || positions.iter().any(|&position| position >= required)
        {
            return Err("capture needs 1..8 increasing unique reachable KV positions".into());
        }
        Ok(())
    }

    /// Creates exclusive 0600 payload files in a fresh 0700 directory.
    /// `setup` must be the actual same-run emitted setup and live-worker identity.
    /// # Errors
    /// Rejects incompatible setup, invalid selection, existing output or I/O failure.
    pub fn new(
        directory: &Path,
        positions: Vec<u32>,
        required: u32,
        setup: Value,
    ) -> TpResult<Self> {
        Self::validate_positions(&positions, required)?;
        validate_setup(&setup, required)?;
        if setup["final_stage_capture"]["positions"] != json!(positions)
            || setup["final_stage_capture"]["benchmark_comparable"] != false
        {
            return Err("capture selection differs from setup".into());
        }
        let parent = directory.parent().ok_or("capture directory parent")?;
        if !directory.is_absolute()
            || directory.file_name().is_none()
            || parent.canonicalize().map_err(|error| error.to_string())? != parent
        {
            return Err("capture needs an absolute canonical parent".into());
        }
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|error| error.to_string())?;
        let directory = File::from(
            rustix::fs::open(
                directory,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| error.to_string())?,
        );
        let files = [
            new_file(&directory, FILES[0])?,
            new_file(&directory, FILES[1])?,
            new_file(&directory, FILES[2])?,
        ];
        let intent = serde_json::to_vec_pretty(&json!({
            "schema":"FerricTpFinalStageIntentV1", "authority":"none", "complete":false,
            "setup":setup, "positions":positions, "required_steps":required,
            "benchmark_comparable":false, "qualification":false,
        }))
        .map_err(|error| error.to_string())?;
        write_new(&directory, "intent.json", &intent)?;
        directory.sync_all().map_err(|error| error.to_string())?;
        Ok(Self {
            directory,
            files,
            hashes: std::array::from_fn(|_| Sha256::new()),
            setup,
            positions,
            required,
            epoch: None,
            tokens: Vec::new(),
            choices: Vec::new(),
            rows: Vec::new(),
            bytes: intent.len(),
            failed: false,
            finished: false,
        })
    }

    /// Executes the ordinary step, reading three completed tensors only on selection.
    /// The returned token is always the actual GPU choice, even if logits disagree.
    /// # Errors
    /// Rejects sequence/epoch drift, failed transport or output writes; poisons on failure.
    pub fn step<R: EngineeringTpRankTransportV1>(
        &mut self,
        engine: &mut EngineeringTpExecutionV1<R>,
        token: u32,
    ) -> TpResult<u32> {
        let result = (|| {
            let position = engine.position();
            let epoch = engine.sequence.epoch();
            if self.failed
                || self.finished
                || position as usize != self.tokens.len()
                || position >= self.required
                || self.epoch.is_some_and(|seen| seen != epoch)
                || engine.plan.world_size() != 1
                || engine.row_capacity != 1
                || engine.plan.model().role != Qwen3ModelRole::Target8B
            {
                return Err("capture sequence state drift".into());
            }
            let prompt = self.setup["prompt_tokens"]
                .as_array()
                .ok_or("capture prompt")?;
            let expected = if (position as usize) < prompt.len() {
                prompt[position as usize]
                    .as_u64()
                    .ok_or("capture prompt token")?
            } else {
                u64::from(*self.choices.last().ok_or("capture preceding GPU choice")?)
            };
            if u64::from(token) != expected {
                return Err("capture consumed sequence differs from prompt/GPU choices".into());
            }
            self.epoch = Some(epoch);
            let choice = if self.positions.contains(&position) {
                let row = engine.final_stage_step(token)?;
                let choice = row.choice;
                self.retain(&row)?;
                choice
            } else {
                engine.step(token)?
            };
            self.tokens.push(token);
            self.choices.push(choice);
            Ok(choice)
        })();
        if result.is_err() {
            self.failed = true;
            engine.sequence.poison();
        }
        result
    }

    fn retain(&mut self, row: &Readback) -> TpResult<()> {
        if self.positions.get(self.rows.len()) != Some(&row.position)
            || self.epoch != Some(row.epoch)
            || row
                .payloads
                .iter()
                .zip(WIDTHS)
                .any(|(bytes, width)| bytes.len() != width * 2)
        {
            return Err("capture readback roster/extent drift".into());
        }
        let mut descriptors = Vec::new();
        for (index, bytes) in row.payloads.iter().enumerate() {
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .filter(|&n| n <= MAX_BYTES)
                .ok_or("capture exceeds 4 MiB")?;
            self.files[index]
                .write_all(bytes)
                .map_err(|error| error.to_string())?;
            self.files[index]
                .sync_all()
                .map_err(|error| error.to_string())?;
            self.hashes[index].update(bytes);
            descriptors.push(
                json!({"file":FILES[index], "offset_bytes":self.rows.len()*WIDTHS[index]*2,
                "bytes":bytes.len(), "sha256":hex(&Sha256::digest(bytes))}),
            );
        }
        // Classification follows durable raw retention and cannot replace the GPU result.
        let diagnostics = logits_diagnostics(&row.payloads[2], row.choice);
        self.rows.push(
            json!({"position":row.position,"input_token":row.token,"gpu_choice":row.choice,
            "residual":descriptors[0],"normalized":descriptors[1],"logits":descriptors[2],
            "logits_diagnostics":diagnostics}),
        );
        Ok(())
    }

    /// Publishes only after the caller's model work and real worker close succeeded.
    /// `closed` must be the actual same-run Closed record, not inferred PID absence.
    /// # Errors
    /// Rejects incomplete/failed/repeated capture or missing/mismatched successful close.
    pub fn finish(&mut self, closed: &Value) -> TpResult<Value> {
        let result = (|| {
            if self.failed
                || self.finished
                || self.tokens.len() != self.required as usize
                || self.rows.len() != self.positions.len()
                || self.epoch.is_none()
                || closed["schema"] != "FerricQwen3TpEngineeringClosedV1"
                || closed["authority"] != "none"
                || closed["all_workers_exited"] != true
                || closed["worker_pids"] != self.setup["worker_pids"]
                || closed["whole_seconds"]
                    .as_f64()
                    .is_none_or(|v| !v.is_finite() || v < 0.0)
            {
                return Err(
                    "capture incomplete, failed, repeated, or without confirmed worker close"
                        .into(),
                );
            }
            let payloads = (0..3)
                .map(|index| {
                    json!({
                        "file":FILES[index], "shape":[self.rows.len(),WIDTHS[index]],
                        "dtype":"bf16-little-endian", "bytes":self.rows.len()*WIDTHS[index]*2,
                        "sha256":hex(&self.hashes[index].clone().finalize()),
                    })
                })
                .collect::<Vec<_>>();
            let manifest = json!({"schema":"FerricTpFinalStageCaptureV1", "authority":"none",
                "complete":true, "worker_close_confirmed":true, "qualification":false,
                "benchmark_comparable":false, "numerical_pass_claimed":false,
                "setup":self.setup,"closed":closed,"positions":self.positions,"epoch":self.epoch,
                "input_tokens":self.tokens,"gpu_choices":self.choices,"rows":self.rows,
                "payloads":payloads,"maximum_total_bytes":MAX_BYTES,
                "nonclaim":"Completed diagnostic readbacks only; no numerical parity, tolerance acceptance, protected proof or performance claim."});
            let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
            self.bytes
                .checked_add(bytes.len())
                .filter(|&n| n <= MAX_BYTES)
                .ok_or("capture manifest exceeds 4 MiB")?;
            write_new(&self.directory, "manifest.incomplete.json", &bytes)?;
            self.directory
                .sync_all()
                .map_err(|error| error.to_string())?;
            rustix::fs::renameat_with(
                &self.directory,
                "manifest.incomplete.json",
                &self.directory,
                "manifest.json",
                RenameFlags::NOREPLACE,
            )
            .map_err(|error| error.to_string())?;
            self.directory
                .sync_all()
                .map_err(|error| error.to_string())?;
            self.finished = true;
            Ok(
                json!({"schema":"FerricTpFinalStageCaptureReceiptV1", "authority":"none",
                "qualification":false,"benchmark_comparable":false,
                "manifest_sha256":hex(&Sha256::digest(&bytes)),"manifest_bytes":bytes.len()}),
            )
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

fn validate_setup(setup: &Value, required: u32) -> TpResult<()> {
    for name in [
        "controller_sha256",
        "worker_sha256",
        "artifact_hsaco_id",
        "artifact_manifest_id",
        "artifact_handoff_id",
        "model_bundle_id",
    ] {
        let value = setup[name].as_str().ok_or("capture identity field")?;
        if value.len() != 64
            || value.bytes().all(|b| b == b'0')
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("capture identity requires nonzero SHA-256".into());
        }
    }
    let prompt = setup["prompt_tokens"]
        .as_array()
        .ok_or("capture prompt tokens")?;
    let devices = setup["device_unique_ids"]
        .as_array()
        .ok_or("capture devices")?;
    let pids = setup["worker_pids"]
        .as_array()
        .ok_or("capture worker pids")?;
    if setup["schema"] != "FerricQwen3TpEngineeringSetupV1"
        || setup["authority"] != "none"
        || setup["model"] != "Qwen/Qwen3-8B"
        || setup["dtype"] != "BF16"
        || setup["target"] != "gfx950:xnack-"
        || setup["tensor_parallel"].as_u64() != Some(1)
        || setup["repetitions"].as_u64() != Some(1)
        || setup["warmup_runs"].as_u64() != Some(0)
        || setup["executable_identity"] != "live_proc_exe_sha256"
        || setup["running_worker_sha256"] != json!([setup["worker_sha256"]])
        || devices.len() != 1
        || devices[0].as_u64().is_none_or(|id| id == 0)
        || pids.len() != 1
        || pids[0]
            .as_u64()
            .is_none_or(|id| !(2..=u64::from(u32::MAX)).contains(&id))
        || prompt.is_empty()
        || prompt.len() > 8192
        || prompt
            .iter()
            .any(|token| token.as_u64().is_none_or(|n| n >= 151_936))
        || setup["new_tokens"].as_u64().is_none_or(|n| {
            !(2..=256).contains(&n) || prompt.len() as u64 + n - 1 != u64::from(required)
        })
        || setup["capacity"]
            .as_u64()
            .is_none_or(|n| n < u64::from(required) || n > 8192)
        || serde_json::to_vec(setup)
            .map_err(|error| error.to_string())?
            .len()
            > 65_536
    {
        return Err("capture setup is outside the single-run TP1 diagnostic scope".into());
    }
    Ok(())
}

fn new_file(directory: &File, name: &str) -> TpResult<File> {
    rustix::fs::openat(
        directory,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map(File::from)
    .map_err(|error| error.to_string())
}

fn write_new(directory: &File, name: &str, bytes: &[u8]) -> TpResult<()> {
    let mut file = new_file(directory, name)?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn logits_diagnostics(bytes: &[u8], choice: u32) -> Value {
    match select_lowest_finite_bf16_argmax(bytes) {
        Ok(cpu) => json!({"finite":true,"cpu_choice":cpu,"gpu_choice_matches":cpu==choice}),
        Err(FiniteBf16ArgmaxError::NonFinite { token }) => json!({
            "finite":false,"first_nonfinite_token":token,"cpu_choice":null,"gpu_choice_matches":null}),
        Err(FiniteBf16ArgmaxError::RowExtent) => json!({"extent_valid":false,
            "cpu_choice":null,"gpu_choice_matches":null}),
    }
}
