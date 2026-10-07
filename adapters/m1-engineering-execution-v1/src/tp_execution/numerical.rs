//! Bounded opt-in TP1 readbacks. These artifacts never qualify model timing.

use crate::tp_execution::{
    EngineeringTpArgumentV1 as Argument, EngineeringTpBufferAccessV1 as Access,
    EngineeringTpDispatchV1, EngineeringTpRankTransportV1, TpResult,
};
use crate::tp_paged::EngineeringTpPreparedBatchV1;
use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1};
use rustix::fs::{Mode, OFlags};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{DirBuilder, File};
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

const MAX_BYTES: usize = 224 * 1024 * 1024;
const COPY_BYTES: usize = 4 * 1024 * 1024;
const VOCABULARY: usize = 151_936;
const HIDDEN: usize = 4096;
const WATCH_TOKENS: [u32; 2] = [9856, 17689];

/// A single actual model projection to capture; not an arithmetic selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineeringTpNumericalProjectionV1 {
    Query,
    Key,
    Value,
    AttentionOutput,
    Gate,
    Up,
    Down,
}

impl EngineeringTpNumericalProjectionV1 {
    /// Parses the explicit diagnostic role.
    /// # Errors
    /// Rejects unknown names; no default is inferred.
    pub fn parse(value: &str) -> TpResult<Self> {
        match value {
            "q" => Ok(Self::Query),
            "k" => Ok(Self::Key),
            "v" => Ok(Self::Value),
            "o" => Ok(Self::AttentionOutput),
            "gate" => Ok(Self::Gate),
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            _ => Err("numerical projection must be q/k/v/o/gate/up/down".into()),
        }
    }

    const fn shape(self) -> (u32, u32, u32, bool) {
        match self {
            Self::Query => (4096, 4096, 1, false),
            Self::Key => (1024, 4096, 2, false),
            Self::Value => (1024, 4096, 3, false),
            Self::AttentionOutput => (4096, 4096, 1, true),
            Self::Gate => (12288, 4096, 4, false),
            Self::Up => (12288, 4096, 5, false),
            Self::Down => (4096, 12288, 2, true),
        }
    }
}

#[derive(Clone, Copy)]
struct Buffer {
    id: u64,
    offset: usize,
    elements: usize,
    width: usize,
}

impl Buffer {
    fn parse(argument: &Argument, access: Access, width: usize) -> TpResult<Self> {
        let Argument::Buffer {
            id,
            offset,
            elements,
            element_bytes,
            access: actual_access,
        } = *argument
        else {
            return Err("numerical capture expected a buffer argument".into());
        };
        if id == 0 || actual_access != access || element_bytes as usize != width {
            return Err("numerical buffer identity/access/width".into());
        }
        let result = Self {
            id,
            offset,
            elements,
            width,
        };
        result.extent()?;
        Ok(result)
    }

    fn extent(self) -> TpResult<usize> {
        self.elements
            .checked_mul(self.width)
            .and_then(|bytes| self.offset.checked_add(bytes))
            .ok_or_else(|| "numerical buffer extent overflow".into())
    }
}

struct Projection {
    input: Buffer,
    weight: Buffer,
    output: Buffer,
    rows: usize,
    n: usize,
    k: usize,
    transposed: bool,
}

impl Projection {
    fn parse(command: &EngineeringTpDispatchV1, shape: (u32, u32, u32, bool)) -> TpResult<Self> {
        let (n, k, tag, partial) = shape;
        let [
            input,
            weight,
            output,
            Argument::U32(rows),
            Argument::U32(actual_n),
            Argument::U32(actual_k),
            Argument::U32(world),
            Argument::U32(actual_tag),
        ] = command.arguments.as_slice()
        else {
            return Err("numerical projection argument roster".into());
        };
        let transposed = match (command.kernel, partial) {
            ("ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2", false)
            | ("ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2", true) => false,
            ("ferric_qwen3_tp_mfma_gemm_bf16_v3", false)
            | ("ferric_qwen3_tp_mfma_gemm_partial_f32_v3", true) => true,
            _ => return Err("numerical capture requires baseline or MFMA v3 arithmetic".into()),
        };
        if !matches!(rows, 1..=16)
            || (*actual_n, *actual_k, *actual_tag, *world) != (n, k, tag, 1)
            || command.workgroup_size != 64
            || command.grid_workgroups != n / 16
        {
            return Err("numerical projection shape/launch".into());
        }
        let input = Buffer::parse(input, Access::Read, 2)?;
        let weight = Buffer::parse(weight, Access::Read, 2)?;
        let output = Buffer::parse(output, Access::Write, if partial { 4 } else { 2 })?;
        let (rows, n, k) = (*rows as usize, n as usize, k as usize);
        if !(rows * k..=16 * k).contains(&input.elements)
            || weight.elements != n * k
            || !(rows * n..=16 * n).contains(&output.elements)
        {
            return Err("numerical projection active slice extent".into());
        }
        Ok(Self {
            input,
            weight,
            output,
            rows,
            n,
            k,
            transposed,
        })
    }
}

/// One private readback owner. Construction itself is an explicit opt-in.
pub struct EngineeringTpNumericalCaptureV1 {
    directory: File,
    identity: Value,
    ordinal: u64,
    layer: u32,
    role: EngineeringTpNumericalProjectionV1,
    plan: Option<(u64, Vec<TpBatchRowV1>)>,
    row_map: Vec<Value>,
    pool_batch: Option<u64>,
    projection: Option<Value>,
    head: Option<Value>,
    bytes: usize,
    failed: bool,
    finished: bool,
}

impl EngineeringTpNumericalCaptureV1 {
    /// Opens a fresh 0700 directory and binds one batch/layer/role selection.
    /// Identity fields must come from the same retained CLI inputs and workers.
    /// # Errors
    /// Rejects invalid selection, identity, noncanonical parent or existing output.
    pub fn new(
        directory: &Path,
        ordinal: u64,
        layer: u32,
        role: EngineeringTpNumericalProjectionV1,
        identity: Value,
    ) -> TpResult<Self> {
        if !(1..=64).contains(&ordinal) || layer >= 36 {
            return Err("numerical capture requires batch1..64 and layer0..35".into());
        }
        for key in [
            "controller_sha256",
            "worker_sha256",
            "artifact_hsaco_id",
            "artifact_manifest_id",
            "artifact_handoff_id",
            "model_bundle_id",
            "requests_sha256",
            "session_id",
        ] {
            let value = identity[key].as_str().ok_or("numerical identity field")?;
            if value.len() != 64
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || value.bytes().all(|byte| byte == b'0')
            {
                return Err("numerical identity must be a nonzero SHA-256".into());
            }
        }
        if identity["tensor_parallel"].as_u64() != Some(1)
            || !matches!(identity["projection"].as_str(), Some("baseline" | "mfma"))
            || identity["output_head_pruning"].as_bool().is_none()
            || identity["device_unique_id"]
                .as_u64()
                .is_none_or(|id| id == 0)
            || serde_json::to_vec(&identity)
                .map_err(|error| error.to_string())?
                .len()
                > 65_536
        {
            return Err("numerical identity/profile is outside TP1 capture scope".into());
        }
        let parent = directory.parent().ok_or("numerical directory parent")?;
        if !directory.is_absolute()
            || directory.file_name().is_none()
            || parent.canonicalize().map_err(|error| error.to_string())? != parent
        {
            return Err("numerical directory needs an absolute canonical parent".into());
        }
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|error| error.to_string())?;
        let fd = rustix::fs::open(
            directory,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            directory: File::from(fd),
            identity,
            ordinal,
            layer,
            role,
            plan: None,
            row_map: Vec::new(),
            pool_batch: None,
            projection: None,
            head: None,
            bytes: 0,
            failed: false,
            finished: false,
        })
    }

    /// Whether the next completed-batch ordinal is selected; no I/O or clock read.
    #[must_use]
    pub const fn selects(&self, ordinal: u64) -> bool {
        ordinal == self.ordinal
    }

    /// Checks the selection without reading clocks, allocating or issuing I/O.
    #[must_use]
    pub fn selects_projection(
        &self,
        ordinal: u64,
        layer: u32,
        role: EngineeringTpNumericalProjectionV1,
    ) -> bool {
        self.selects(ordinal) && self.layer == layer && self.role == role
    }

    pub(super) fn matches_profile(&self, projection: &str, pruning: bool) -> bool {
        self.identity["projection"].as_str() == Some(projection)
            && self.identity["output_head_pruning"].as_bool() == Some(pruning)
    }

    /// Binds the scheduler's actual immutable row identities, only on selection.
    /// # Errors
    /// Rejects duplicate binding, exhausted/stale identity or out-of-range rows.
    pub fn bind_rows(
        &mut self,
        ordinal: u64,
        scheduler_batch: u64,
        rows: &[TpBatchRowV1],
    ) -> TpResult<()> {
        if !self.selects(ordinal) {
            return Ok(());
        }
        if self.failed
            || self.finished
            || self.plan.is_some()
            || scheduler_batch == 0
            || rows.is_empty()
            || rows.len() > 16
            || rows.iter().any(|row| {
                row.request.generation == 0
                    || row.token_id >= 151_936
                    || row.absolute_position >= 8192
            })
        {
            self.failed = true;
            return Err("numerical scheduler row binding is invalid or repeated".into());
        }
        self.plan = Some((scheduler_batch, rows.to_vec()));
        Ok(())
    }

    /// Checks source/execution row order against the actual prepared pool batch.
    /// # Errors
    /// Rejects stale binding, any row/permutation/publishing mismatch or reuse.
    pub fn begin_batch(
        &mut self,
        ordinal: u64,
        batch: &EngineeringTpPreparedBatchV1,
        order: &[usize],
        published: &[usize],
    ) -> TpResult<()> {
        if !self.selects(ordinal) {
            return Ok(());
        }
        let result = (|| {
            if self.failed || self.finished || self.pool_batch.is_some() {
                return Err("numerical batch already used".into());
            }
            let (_, plan) = self
                .plan
                .as_ref()
                .ok_or("numerical scheduler row identity is missing")?;
            if plan.len() != batch.rows().len()
                || order.len() != plan.len()
                || order.iter().copied().collect::<BTreeSet<_>>() != (0..plan.len()).collect()
            {
                return Err("numerical execution row permutation".into());
            }
            for (source, (row, planned)) in batch.rows().iter().zip(plan).enumerate() {
                if row.token() != planned.token_id
                    || row.position() != planned.absolute_position
                    || published.contains(&source)
                        != (planned.kind != TpBatchRowKindV1::PrefillIntermediate)
                {
                    return Err("numerical source/published row identity mismatch".into());
                }
            }
            self.row_map = order.iter().enumerate().map(|(physical, &source)| {
                let row = plan[source];
                json!({"execution_row":physical,"source_row":source,"slot":row.request.slot,
                    "generation":row.request.generation,"token":row.token_id,"position":row.absolute_position,
                    "kind":format!("{:?}",row.kind),"publishable":published.contains(&source)})
            }).collect();
            self.pool_batch = Some(batch.id());
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn file(&self, name: &str) -> TpResult<File> {
        let fd = rustix::fs::openat(
            &self.directory,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|error| error.to_string())?;
        Ok(File::from(fd))
    }

    fn reserve(&self, bytes: usize) -> TpResult<usize> {
        self.bytes
            .checked_add(bytes)
            .filter(|total| *total <= MAX_BYTES)
            .ok_or_else(|| "numerical capture exceeds the 224 MiB total payload bound".into())
    }

    fn read<R: EngineeringTpRankTransportV1>(
        &mut self,
        transport: &mut R,
        buffer: Buffer,
        bytes: usize,
        name: &str,
        retain: bool,
    ) -> TpResult<(Value, Vec<u8>)> {
        if bytes == 0 || bytes > buffer.elements * buffer.width {
            return Err("numerical read active extent".into());
        }
        let total = self.reserve(bytes)?;
        let mut file = self.file(name)?;
        let mut hash = Sha256::new();
        let mut kept = Vec::new();
        let mut scratch = vec![0; bytes.min(COPY_BYTES)];
        for offset in (0..bytes).step_by(COPY_BYTES) {
            let count = (bytes - offset).min(COPY_BYTES);
            let part = &mut scratch[..count];
            transport.read(buffer.id, buffer.offset + offset, part)?;
            file.write_all(part).map_err(|error| error.to_string())?;
            hash.update(&*part);
            if retain {
                kept.extend_from_slice(part);
            }
        }
        file.sync_all().map_err(|error| error.to_string())?;
        self.bytes = total;
        Ok((
            json!({"file":name,"bytes":bytes,"sha256":hex(&hash.finalize()),
            "buffer_id":buffer.id,"buffer_offset":buffer.offset,"element_bytes":buffer.width}),
            kept,
        ))
    }

    fn write_bytes(&mut self, name: &str, bytes: &[u8]) -> TpResult<Value> {
        let total = self.reserve(bytes.len())?;
        let mut file = self.file(name)?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        self.bytes = total;
        Ok(json!({"file":name,"bytes":bytes.len(),"sha256":hex(&Sha256::digest(bytes))}))
    }

    fn validate_projection(
        &self,
        projection: &Projection,
        original: &Argument,
    ) -> TpResult<Buffer> {
        if self.failed
            || self.finished
            || self.pool_batch.is_none()
            || projection.transposed != (self.identity["projection"].as_str() == Some("mfma"))
        {
            return Err("numerical projection lifecycle/profile mismatch".into());
        }
        let original = Buffer::parse(original, Access::Read, 2)?;
        if original.elements != projection.n * projection.k
            || (!projection.transposed
                && (original.id, original.offset)
                    != (projection.weight.id, projection.weight.offset))
        {
            return Err("numerical original weight identity/extent".into());
        }
        Ok(original)
    }

    /// Captures one already-completed selected projection without dispatching work.
    /// # Errors
    /// Rejects binding/shape drift, read or file failure, duplicate capture or limit excess.
    pub fn capture_projection<R: EngineeringTpRankTransportV1>(
        &mut self,
        ordinal: u64,
        layer: u32,
        role: EngineeringTpNumericalProjectionV1,
        command: &EngineeringTpDispatchV1,
        original: Argument,
        transport: &mut R,
    ) -> TpResult<()> {
        if !self.selects(ordinal) || layer != self.layer || role != self.role {
            return Ok(());
        }
        let result = (|| {
            if self.projection.is_some() {
                return Err("numerical projection captured twice".into());
            }
            let projection = Projection::parse(command, role.shape())?;
            let original = self.validate_projection(&projection, &original)?;
            if projection.rows != self.row_map.len() {
                return Err("numerical projection row count".into());
            }
            let input = self
                .read(
                    transport,
                    projection.input,
                    projection.rows * projection.k * 2,
                    "projection-input.bf16",
                    false,
                )?
                .0;
            let weights_nk = self
                .read(
                    transport,
                    original,
                    projection.n * projection.k * 2,
                    "projection-weights-nk.bf16",
                    false,
                )?
                .0;
            let actual_weights = if projection.transposed {
                self.read(
                    transport,
                    projection.weight,
                    projection.n * projection.k * 2,
                    "projection-weights-kn.bf16",
                    false,
                )?
                .0
            } else {
                weights_nk.clone()
            };
            let output = self
                .read(
                    transport,
                    projection.output,
                    projection.rows * projection.n * projection.output.width,
                    if role.shape().3 {
                        "projection-output.f32"
                    } else {
                        "projection-output.bf16"
                    },
                    false,
                )?
                .0;
            self.projection = Some(
                json!({"kernel":command.kernel,"role":format!("{role:?}"),"layer":layer,
                "rows":projection.rows,"n":projection.n,"k":projection.k,"world":1,"tag":role.shape().2,
                "input":input,"weights_nk":weights_nk,"actual_weights":actual_weights,
                "actual_weight_layout":if projection.transposed {"kn"} else {"nk"},"output":output}),
            );
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Captures final logits/margins and actual LM operands after greedy choice readback.
    /// # Errors
    /// Rejects nonfinite logits, CPU/GPU argmax disagreement, bad rows or read/file errors.
    pub fn capture_head<R: EngineeringTpRankTransportV1>(
        &mut self,
        ordinal: u64,
        command: &EngineeringTpDispatchV1,
        original: Argument,
        choices: &[u32],
        transport: &mut R,
    ) -> TpResult<()> {
        if !self.selects(ordinal) {
            return Ok(());
        }
        let result = (|| {
            if self.head.is_some() {
                return Err("numerical head captured twice".into());
            }
            let projection = Projection::parse(command, (151_936, 4096, 6, false))?;
            let original = self.validate_projection(&projection, &original)?;
            let expected_rows = if self.identity["output_head_pruning"] == true {
                self.row_map
                    .iter()
                    .filter(|row| row["publishable"] == true)
                    .count()
            } else {
                self.row_map.len()
            };
            if projection.rows != expected_rows || choices.len() != expected_rows {
                return Err("numerical final head rows".into());
            }
            let input = self
                .read(
                    transport,
                    projection.input,
                    projection.rows * HIDDEN * 2,
                    "head-input.bf16",
                    false,
                )?
                .0;
            let (logits, data) = self.read(
                transport,
                projection.output,
                projection.rows * VOCABULARY * 2,
                "head-logits.bf16",
                true,
            )?;
            let (summaries, selected) =
                summarize_logits(&data, choices, &self.row_map[..projection.rows])?;
            let mut selected_weights = vec![0; selected.len() * HIDDEN * 2];
            for (index, &token) in selected.iter().enumerate() {
                let offset = original.offset + token as usize * HIDDEN * 2;
                let bytes = &mut selected_weights[index * HIDDEN * 2..(index + 1) * HIDDEN * 2];
                transport.read(original.id, offset, bytes)?;
            }
            let weights = self.write_bytes("head-selected-weights-nk.bf16", &selected_weights)?;
            self.head = Some(
                json!({"kernel":command.kernel,"rows":projection.rows,"n":VOCABULARY,"k":HIDDEN,
                "input":input,"logits":logits,"request_rows":summaries,
                "selected_weight_tokens":selected,"selected_weights_nk":weights,
                "original_weight_buffer_id":original.id,"original_weight_buffer_offset":original.offset,
                "watch_tokens":WATCH_TOKENS,"watch_label_scope":"token IDs only; no tokenizer decoding assumed"}),
            );
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Publishes the manifest only after both exact selected readbacks completed.
    /// The CLI must call this only after successful model work and owned-worker close.
    /// # Errors
    /// Rejects an incomplete/failed/repeated capture; partial files are not success.
    pub fn finish(&mut self) -> TpResult<Value> {
        if self.failed
            || self.finished
            || self.projection.is_none()
            || self.head.is_none()
            || self.pool_batch.is_none()
        {
            return Err("numerical capture is incomplete, failed, or already finished".into());
        }
        let (scheduler_batch, _) = self
            .plan
            .as_ref()
            .ok_or("numerical plan identity missing")?;
        let manifest = json!({"schema":"FerricTpNumericalCaptureV1","authority":"none","complete":true,
            "model_parity_qualified":false,"performance_qualified":false,
            "timing_scope":"diagnostic readbacks invalidate all performance measurements for this run",
            "identity":self.identity,"batch_ordinal":self.ordinal,"scheduler_batch_id":scheduler_batch,
            "pool_batch_id":self.pool_batch,"execution_rows":self.row_map,
            "projection":self.projection,"head":self.head,"payload_bytes_before_manifest":self.bytes,
            "maximum_total_bytes":MAX_BYTES});
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
        let result = self.write_bytes("manifest.incomplete.json", &bytes);
        if result.is_err() {
            self.failed = true;
        }
        let mut receipt = result?;
        let publication = (|| {
            self.directory
                .sync_all()
                .map_err(|error| error.to_string())?;
            rustix::fs::renameat_with(
                &self.directory,
                "manifest.incomplete.json",
                &self.directory,
                "manifest.json",
                rustix::fs::RenameFlags::NOREPLACE,
            )
            .map_err(|error| error.to_string())?;
            self.directory.sync_all().map_err(|error| error.to_string())
        })();
        if let Err(error) = publication {
            self.failed = true;
            return Err(format!(
                "numerical manifest publication failed; receipt absent: {error}"
            ));
        }
        receipt["file"] = json!("manifest.json");
        self.finished = true;
        Ok(
            json!({"schema":"FerricTpNumericalCaptureReceiptV1","authority":"none",
            "performance_qualified":false,"manifest":receipt,"total_bytes":self.bytes}),
        )
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[(byte >> 4) as usize]),
                char::from(DIGITS[(byte & 15) as usize]),
            ]
        })
        .collect()
}

// Exact finite BF16 ties, including signed zero, must follow the GPU token-ID policy.
#[allow(clippy::float_cmp)]
fn summarize_logits(
    data: &[u8],
    choices: &[u32],
    rows: &[Value],
) -> TpResult<(Vec<Value>, Vec<u32>)> {
    if data.len() != choices.len() * VOCABULARY * 2
        || choices.len() != rows.len()
        || choices.is_empty()
        || choices.len() > 16
    {
        return Err("numerical logits byte/row extent".into());
    }
    let mut summaries = Vec::new();
    let mut selected = WATCH_TOKENS.into_iter().collect::<BTreeSet<_>>();
    for (row, logits) in data.chunks_exact(VOCABULARY * 2).enumerate() {
        let mut top: Vec<(u32, u16, f32)> = Vec::new();
        let mut watch = Vec::new();
        for (token, bytes) in logits.chunks_exact(2).enumerate() {
            let bits = u16::from_le_bytes([bytes[0], bytes[1]]);
            let value = f32::from_bits(u32::from(bits) << 16);
            if !value.is_finite() {
                return Err("numerical final logits contain nonfinite values".into());
            }
            let token = u32::try_from(token).map_err(|_| "numerical vocabulary conversion")?;
            if WATCH_TOKENS.contains(&token) {
                watch.push(json!({"token":token,"bf16_bits":bits,"value":value}));
            }
            let index = top
                .iter()
                .position(|entry| value > entry.2)
                .unwrap_or(top.len());
            if index < 16 {
                top.insert(index, (token, bits, value));
                top.truncate(16);
            }
        }
        if top[0].0 != choices[row] {
            return Err("numerical CPU/GPU argmax disagreement".into());
        }
        selected.extend(top.iter().map(|entry| entry.0));
        let top_json = top
            .iter()
            .map(|&(token, bits, value)| json!({"token":token,"bf16_bits":bits,"value":value}))
            .collect::<Vec<_>>();
        summaries.push(json!({"row_identity":rows[row],"gpu_choice":choices[row],"top16":top_json,
            "top1_minus_top2":f64::from(top[0].2)-f64::from(top[1].2),"top_two_tied":top[0].2==top[1].2,
            "tie_policy":"lowest token ID among equal finite BF16 logits","watch":watch}));
    }
    if selected.len() > 258 {
        return Err("numerical selected head weight row bound".into());
    }
    Ok((summaries, selected.into_iter().collect()))
}

#[cfg(test)]
#[path = "numerical/tests.rs"]
mod tests;
