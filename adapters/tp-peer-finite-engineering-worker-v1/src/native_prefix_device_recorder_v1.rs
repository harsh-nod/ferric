//! Private recorder: only native observation getters feed the production path.
use crate::finite_prefix_decode_wire_v1::{Bootstrap, Completion, Control};
use crate::native_catalog::forward::prefix_tiles_decode_v6::Owner;
use crate::prefix_decode_device_observation_v1::{
    self as data, Images, Rank, Report, Row, Stage, require,
};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerHostObservationV1 as Snapshot,
    Gfx950EngineeringPeerKernelV1 as Kernel, Gfx950EngineeringRawTimestampObservationV1 as Raw,
};
use std::io;

pub(crate) struct Recorder {
    report: Report,
    bound: bool,
    failed: bool,
    packets: [u64; 2],
    pending_control: Option<[u8; 32]>,
}
/// No constructor outside this module: available only after consuming Close.
pub(crate) struct ClosedReport(Report);
impl ClosedReport {
    pub(crate) fn encode(&self) -> io::Result<Vec<u8>> {
        self.0.encode()
    }
    pub(crate) fn worker_sha256(&self) -> [u8; 32] {
        self.0.worker_sha256
    }
}
fn identity(raw: &Snapshot, b: &Bootstrap) -> io::Result<[Rank; 2]> {
    require(
        raw.group_incarnation() != 0
            && raw.shared_full_currentness()
            && raw.participants().len() == 2,
        "raw recorder group/policy",
    )?;
    let mut ranks = [Rank {
        rank: 0,
        unique_id: 0,
        queue_epoch: 0,
    }; 2];
    for (i, r) in raw.participants().iter().enumerate() {
        require(
            r.rank() == i
                && r.unique_id() == b.device_ids[i]
                && r.raw_timestamp_queue()
                && !r.cache_kernel_admission()
                && r.counters().operational_currentness_checks == 0
                && r.counters().operational_currentness_ns == 0,
            "raw recorder rank/policy",
        )?;
        ranks[i] = Rank {
            rank: i as u32,
            unique_id: r.unique_id(),
            queue_epoch: r.queue_epoch(),
        };
    }
    Ok(ranks)
}
impl Recorder {
    /// Caller must already have opened a fresh raw-enabled group. No mode toggle.
    pub(crate) fn enable(group: &mut Group, b: &Bootstrap, worker: [u8; 32]) -> io::Result<Self> {
        b.validate(b.device_ids, b.timeout_ms, std::process::id(), b.mode)?;
        require(worker != [0; 32], "raw recorder executable digest")?;
        group
            .configure_performance_v2(false, false, true)
            .map_err(io::Error::other)?;
        group
            .enable_host_observation_v1()
            .map_err(io::Error::other)?;
        let raw = group.host_observation_v1().map_err(io::Error::other)?;
        let ranks = identity(&raw, b)?;
        require(
            raw.shared_counters() == &Default::default()
                && raw
                    .participants()
                    .iter()
                    .all(|r| r.counters() == &Default::default()),
            "raw recorder must precede allocation/setup",
        )?;
        Ok(Self {
            report: Report {
                schema: data::SCHEMA.into(),
                bootstrap: b.clone(),
                worker_sha256: worker,
                child_pid: std::process::id(),
                profile_sha256: b.sha256()?,
                group_incarnation: raw.group_incarnation(),
                ranks,
                images: Images {
                    prefix: [0; 32],
                    mlp: [0; 32],
                    residual: [0; 32],
                    tail: [0; 32],
                    copy: [0; 32],
                },
                rows: Vec::with_capacity(data::MAX_ROWS),
                final_dispatches: [0; 2],
                completions: Vec::with_capacity(4),
                transcript_sha256: [0; 32],
                raw_timestamp_queue: true,
                shared_full_currentness: true,
                cache_kernel_admission: false,
                operational_currentness: false,
                native_closed: false,
                raw_completion_ticks: true,
                calibrated_nanoseconds: false,
                cross_device_clock_alignment: false,
                overlap_claim: false,
                numerical_acceptance: false,
                performance_claim: false,
                production_authority: false,
                full_model_acceptance: false,
            },
            bound: false,
            failed: false,
            packets: [0; 2],
            pending_control: None,
        })
    }
    fn active(&self) -> io::Result<()> {
        require(
            !self.failed && !self.report.native_closed,
            "raw recorder terminal",
        )
    }
    fn finish<T>(&mut self, result: io::Result<T>) -> io::Result<T> {
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    /// Actual retained kernels in order: prefix, MLP, residual, embedding, copy.
    /// This records metadata identity; it does not authenticate model/source roles.
    pub(crate) fn bind_images(&mut self, kernels: [&Kernel; 5]) -> io::Result<()> {
        let result = (|| {
            self.active()?;
            require(
                !self.bound && self.report.rows.is_empty(),
                "raw recorder image binding order",
            )?;
            let stages = [
                Stage::Prefix,
                Stage::Mlp,
                Stage::PostAttentionResidual,
                Stage::Embedding,
                Stage::Copy,
            ];
            for (i, (kernel, stage)) in kernels.iter().zip(stages).enumerate() {
                require(
                    kernel.rank() == usize::from(i == 4)
                        && kernel.metadata().symbol == stage.entry(),
                    "raw recorder loaded kernel identity",
                )?;
            }
            let images = Images {
                prefix: kernels[0].metadata().object_sha256,
                mlp: kernels[1].metadata().object_sha256,
                residual: kernels[2].metadata().object_sha256,
                tail: kernels[3].metadata().object_sha256,
                copy: kernels[4].metadata().object_sha256,
            };
            images.validate(&self.report.bootstrap)?;
            self.report.images = images;
            self.bound = true;
            Ok(())
        })();
        self.finish(result)
    }
    pub(crate) fn record(
        &mut self,
        generation: u64,
        position: u32,
        stage: Stage,
        layer: Option<u32>,
        kernel: &Kernel,
        raw: &Raw,
    ) -> io::Result<()> {
        let result = (|| {
            require(
                kernel.rank() == raw.rank(),
                "raw recorder selected kernel rank",
            )?;
            self.record_row(Row {
                generation,
                position,
                stage,
                layer,
                rank: raw.rank() as u32,
                entry: kernel.metadata().symbol.clone(),
                image_sha256: kernel.metadata().object_sha256,
                group_incarnation: raw.group_incarnation(),
                unique_id: raw.unique_id(),
                queue_epoch: raw.queue_epoch(),
                packet_id: raw.packet_id(),
                signal_generation: raw.signal_generation(),
                start_tick: raw.start_tick(),
                end_tick: raw.end_tick(),
                host_elapsed_ns: raw.host_elapsed_ns(),
            })
        })();
        self.finish(result)
    }
    // Private synthetic tests reach this seam; production callers cannot inject rows.
    fn record_row(&mut self, row: Row) -> io::Result<()> {
        let result = (|| {
            self.active()?;
            require(
                self.bound
                    && self.report.completions.len() == self.report.rows.len() / data::PER_FORWARD,
                "raw recorder row/completion order",
            )?;
            data::validate_row(
                &row,
                self.report.rows.len(),
                self.report.group_incarnation,
                &self.report.ranks,
                self.report.images,
                &mut self.packets,
            )?;
            self.report.rows.push(row);
            Ok(())
        })();
        self.finish(result)
    }
    pub(crate) fn control(&mut self, control: &Control) -> io::Result<()> {
        let result = (|| {
            self.active()?;
            let n = self.report.completions.len();
            require(
                n < 4
                    && self.pending_control.is_none()
                    && self.report.rows.len() == (n + 1) * data::PER_FORWARD,
                "raw recorder control order",
            )?;
            self.pending_control = Some(data::join_control(
                &self.report.rows[n * data::PER_FORWARD..],
                control,
            )?);
            Ok(())
        })();
        self.finish(result)
    }
    pub(crate) fn completed(&mut self, completion: &Completion) -> io::Result<()> {
        let result = (|| {
            self.active()?;
            let n = self.report.completions.len();
            require(
                n < 4 && self.report.rows.len() == (n + 1) * data::PER_FORWARD,
                "raw recorder incomplete forward",
            )?;
            require(
                self.pending_control == Some(completion.control.sha256),
                "raw recorder control digest join",
            )?;
            self.report.completions.push(completion.clone());
            self.report.transcript_sha256 = data::completions(
                &self.report.bootstrap,
                self.report.profile_sha256,
                &self.report.completions,
            )?;
            self.pending_control = None;
            Ok(())
        })();
        self.finish(result)
    }
    /// Consumes the real owner. No caller-supplied `closed: bool` can publish.
    pub(crate) fn close(self, mut owner: Owner) -> io::Result<ClosedReport> {
        let raw = owner.host_observation().map_err(io::Error::other)?;
        require(
            identity(&raw, &self.report.bootstrap)? == self.report.ranks
                && raw.group_incarnation() == self.report.group_incarnation,
            "raw recorder pre-Close queue identity",
        )?;
        let dispatches = [
            raw.participants()[0].counters().dispatches,
            raw.participants()[1].counters().dispatches,
        ];
        self.close_with(dispatches, || owner.close().map_err(io::Error::other))
    }
    fn close_with(
        mut self,
        dispatches: [u64; 2],
        close: impl FnOnce() -> io::Result<()>,
    ) -> io::Result<ClosedReport> {
        self.active()?;
        require(
            self.bound
                && self.report.rows.len() == data::MAX_ROWS
                && self.report.completions.len() == 4
                && self.pending_control.is_none()
                && self.packets == data::RANK_PACKETS
                && dispatches == self.packets,
            "raw recorder incomplete Close",
        )?;
        close()?;
        self.report.final_dispatches = dispatches;
        self.report.native_closed = true;
        self.report.validate()?;
        Ok(ClosedReport(self.report))
    }
}

#[cfg(test)]
#[path = "native_prefix_device_recorder_v1_tests.rs"]
mod tests;
