//! Opt-in TP1 layer-zero attention-output boundary bytes.
//! These are raw Contracted observations, never numerical qualification.

use super::final_stage::{new_file, validate_setup, write_new};
use super::{
    EngineeringTpExecutionV1, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3, TpResult,
};
use crate::hex;
use ferric_engine::tensor_parallel::Qwen3TensorParallelCollectiveV1;
use ferric_spec::Qwen3ModelRole;
use rustix::fs::{Mode, OFlags, RenameFlags};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{DirBuilder, File};
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

const ELEMENTS: usize = 4096;
const WIDTHS: [usize; 4] = [ELEMENTS * 2, ELEMENTS * 2, ELEMENTS * 4, ELEMENTS * 2];
const FILES: [&str; 4] = [
    "projection-input.bf16",
    "residual-before.bf16",
    "projection-partial.f32le",
    "hidden-after-broadcast.bf16",
];
const MAX_BYTES: usize = 4 * 1024 * 1024;

pub(super) struct LayerBoundaryReadbackV1 {
    pub(super) epoch: u64,
    pub(super) position: u32,
    pub(super) token: u32,
    pub(super) layer: u32,
    pub(super) operation: Qwen3TensorParallelCollectiveV1,
    pub(super) projection_input: Vec<u8>,
    pub(super) residual_before: Vec<u8>,
    pub(super) projection_partial: Vec<u8>,
    pub(super) hidden_after: Vec<u8>,
    pub(super) hidden_after_matches_host_broadcast: bool,
}

/// Owns a fresh directory and selected TP1 layer-zero attention-output rows.
pub struct EngineeringTpResidualBoundaryCaptureV1 {
    directory: File,
    files: [File; 4],
    hashes: [Sha256; 4],
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

impl EngineeringTpResidualBoundaryCaptureV1 {
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
            return Err("boundary capture needs 1..8 increasing unique reachable positions".into());
        }
        Ok(())
    }

    /// Creates exclusive payload files in a fresh directory.
    /// # Errors
    /// Rejects non-TP1 setup, unsupported boundary selection or existing output.
    pub fn new(
        directory: &Path,
        positions: Vec<u32>,
        required: u32,
        setup: Value,
    ) -> TpResult<Self> {
        Self::validate_positions(&positions, required)?;
        validate_setup(&setup, required)?;
        let selection = &setup["residual_boundary_capture"];
        let weight_sha = selection["output_projection_weight_shard_sha256"]
            .as_str()
            .ok_or("boundary capture weight identity")?;
        if setup.get("final_stage_capture").is_some()
            || setup["collective"] != "host_staged_fp32_rank_order_reduce_bf16_residual"
            || selection["positions"] != json!(positions)
            || selection["layer"].as_u64() != Some(0)
            || selection["operation"] != "AttentionOutputSum"
            || selection["tensor_parallel_rank"].as_u64() != Some(0)
            || selection["tensor_parallel_world"].as_u64() != Some(1)
            || selection["output_projection_weight_tensor"]
                != "model.layers.0.self_attn.o_proj.weight"
            || selection["output_projection_weight_shape"] != json!([4096, 4096])
            || selection["projection_input_elements"].as_u64() != Some(ELEMENTS as u64)
            || selection["residual_elements"].as_u64() != Some(ELEMENTS as u64)
            || selection["projection_partial_elements"].as_u64() != Some(ELEMENTS as u64)
            || selection["hidden_after_elements"].as_u64() != Some(ELEMENTS as u64)
            || selection["residual_source"] != "host_staged_collective_input"
            || selection["benchmark_comparable"] != false
            || !valid_sha256(weight_sha)
        {
            return Err("boundary selection differs from exact TP1 layer-zero setup".into());
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
            new_file(&directory, FILES[3])?,
        ];
        let intent = serde_json::to_vec_pretty(&json!({
            "schema":"FerricTpResidualBoundaryIntentV1", "authority":"none",
            "complete":false, "setup":setup, "positions":positions,
            "required_steps":required, "benchmark_comparable":false, "qualification":false,
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

    /// Executes an ordinary token step, adding boundary reads only on selection.
    /// # Errors
    /// Rejects sequence, engine-profile or transport drift and poisons the sequence.
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
            {
                return Err("boundary capture sequence state drift".into());
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
            engine.require_residual_boundary_scope()?;
            self.epoch = Some(epoch);
            let selected = self.positions.binary_search(&position).is_ok();
            let before = self.rows.len();
            let choice = if selected {
                engine.step_with_residual_boundary(token, self)?
            } else {
                engine.step(token)?
            };
            if selected && self.rows.len() != before + 1 {
                return Err("selected boundary row was not retained".into());
            }
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

    pub(super) fn retain(&mut self, row: LayerBoundaryReadbackV1) -> TpResult<()> {
        let payloads = [
            row.projection_input,
            row.residual_before,
            row.projection_partial,
            row.hidden_after,
        ];
        if self.positions.get(self.rows.len()) != Some(&row.position)
            || self.epoch != Some(row.epoch)
            || row.layer != 0
            || row.operation != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            || payloads
                .iter()
                .zip(WIDTHS)
                .any(|(bytes, width)| bytes.len() != width)
        {
            return Err("boundary readback roster or extent drift".into());
        }
        let mut descriptors = Vec::new();
        for (index, bytes) in payloads.iter().enumerate() {
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .filter(|&n| n <= MAX_BYTES)
                .ok_or("boundary capture exceeds 4 MiB")?;
            self.files[index]
                .write_all(bytes)
                .map_err(|error| error.to_string())?;
            self.files[index]
                .sync_all()
                .map_err(|error| error.to_string())?;
            self.hashes[index].update(bytes);
            descriptors.push(json!({
                "file":FILES[index], "offset_bytes":self.rows.len()*WIDTHS[index],
                "bytes":bytes.len(), "sha256":hex(&Sha256::digest(bytes)),
            }));
        }
        self.rows.push(json!({
            "position":row.position, "input_token":row.token, "layer":row.layer,
            "operation":"AttentionOutputSum", "projection_input":descriptors[0],
            "residual_before":descriptors[1], "projection_partial":descriptors[2],
            "hidden_after_broadcast":descriptors[3],
            "gpu_hidden_matches_host_broadcast":row.hidden_after_matches_host_broadcast,
        }));
        Ok(())
    }

    /// Publishes a manifest only after the same-run worker-close record succeeds.
    /// # Errors
    /// Rejects incomplete, failed or repeated capture and any close mismatch.
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
                    .is_none_or(|value| !value.is_finite() || value < 0.0)
            {
                return Err(
                    "boundary capture incomplete, failed, repeated, or without confirmed worker close"
                        .into(),
                );
            }
            let dtypes = [
                "bf16-little-endian",
                "bf16-little-endian",
                "f32-little-endian",
                "bf16-little-endian",
            ];
            let element_bytes = [2, 2, 4, 2];
            let payloads = (0..4)
                .map(|index| {
                    json!({
                        "file":FILES[index], "shape":[self.rows.len(),ELEMENTS],
                        "dtype":dtypes[index], "element_bytes":element_bytes[index],
                        "bytes":self.rows.len()*WIDTHS[index],
                        "sha256":hex(&self.hashes[index].clone().finalize()),
                    })
                })
                .collect::<Vec<_>>();
            let manifest = json!({
                "schema":"FerricTpResidualBoundaryCaptureV1", "authority":"none",
                "complete":true, "worker_close_confirmed":true, "qualification":false,
                "benchmark_comparable":false, "numerical_pass_claimed":false,
                "setup":self.setup, "closed":closed, "positions":self.positions,
                "epoch":self.epoch, "input_tokens":self.tokens, "gpu_choices":self.choices,
                "rows":self.rows, "payloads":payloads, "maximum_total_bytes":MAX_BYTES,
                "nonclaim":"Raw selected layer-zero TP1 boundary readbacks only; mismatches are retained and no parity, tolerance, protected-proof or performance claim is made.",
            });
            let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
            self.bytes
                .checked_add(bytes.len())
                .filter(|&n| n <= MAX_BYTES)
                .ok_or("boundary capture manifest exceeds 4 MiB")?;
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
            Ok(json!({
                "schema":"FerricTpResidualBoundaryCaptureReceiptV1", "authority":"none",
                "qualification":false, "benchmark_comparable":false,
                "manifest_sha256":hex(&Sha256::digest(&bytes)), "manifest_bytes":bytes.len(),
            }))
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    fn require_residual_boundary_scope(&self) -> TpResult<()> {
        let model = self.plan.model();
        if self.closed
            || self.plan.world_size() != 1
            || self.ranks.len() != 1
            || self.transports.len() != 1
            || self.row_capacity != 1
            || self.large_kv
            || self.draft_v10
            || model.role != Qwen3ModelRole::Target8B
            || model.hidden_size as usize != ELEMENTS
            || self.ranks[0].geometry.query_channels.count as usize != ELEMENTS
            || self.reduction.mode() != EngineeringTpReductionModeV3::HostStagedV1
            || self.residual_arithmetic.is_some()
            || self.sequences.is_some()
            || self.ordered_batches.is_some()
        {
            return Err(
                "boundary capture requires open target TP1 single-row default host-staged execution"
                    .into(),
            );
        }
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && !value.bytes().all(|byte| byte == b'0')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
