//! One-packet hardware diagnostics for the Ferric M1 service queue path.

use fe2o3_kfd::{
    CheckedGfx942XnackMinusDevice, DeviceSelector, Gfx942BarrierProbeFailureV1,
    Gfx942BarrierProbePollBoundV1, OpenedKfd,
};
use ferric_build::{
    decode_bundle_admission_record, reopen_persisted_qwen3_weight_manifest, WeightSection,
    BUNDLE_ADMISSION_RECORD_BYTES, QWEN3_TARGET_PREPACKED_MANIFEST_BYTES,
};
use ferric_engine::{
    execute_m1_k1_target_s1t128_packet_v1, execute_m1_k7_s1k4_packet_v1,
    reopen_persisted_m1_kernel_artifacts_v1, M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
};
use ferric_spec::{
    Qwen3ModelRole, Qwen3TensorKind, QWEN3_NO_LAYER, QWEN3_TARGET_TENSOR_DATA_BYTES,
};
use rustix::fd::OwnedFd;
use rustix::fs::{fstat, openat2, FileType, Mode, OFlags, ResolveFlags, Stat, CWD};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::ExitCode;

type DiagnosticResult<T> = Result<T, String>;

const BARRIER_COMPLETION_POLL_LIMIT: u32 = 1_000;
const USAGE: &str = "usage: ferric-m1-packet-diagnostic queue-barrier GPU-UNIQUE-ID\n       ferric-m1-packet-diagnostic queue-barrier-executable GPU-UNIQUE-ID\n       ferric-m1-packet-diagnostic queue-barrier-userptr GPU-UNIQUE-ID\n       ferric-m1-packet-diagnostic k7-smoke KERNEL-ARTIFACTS GPU-UNIQUE-ID\n       ferric-m1-packet-diagnostic k1-embedding PREPACKED-SNAPSHOT KERNEL-ARTIFACTS GPU-UNIQUE-ID";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueueBarrierRing {
    AqlSpecial,
    Executable,
    Userptr,
}

#[derive(Debug)]
enum Command {
    QueueBarrier {
        gpu_unique_id: u64,
        ring: QueueBarrierRing,
    },
    K7Smoke {
        artifact_root: OsString,
        gpu_unique_id: u64,
    },
    K1Embedding {
        prepacked_root: OsString,
        artifact_root: OsString,
        gpu_unique_id: u64,
    },
}

#[derive(Debug)]
struct SecureDirectory {
    descriptor: OwnedFd,
}

#[derive(Debug)]
struct SecureFile {
    file: File,
    initial: Stat,
}

fn main() -> ExitCode {
    match parse_command(std::env::args_os().skip(1).collect()).and_then(execute) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ferric-m1-packet-diagnostic: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_command(arguments: Vec<OsString>) -> DiagnosticResult<Command> {
    match arguments.as_slice() {
        [mode, gpu_unique_id] if mode == "queue-barrier" => Ok(Command::QueueBarrier {
            gpu_unique_id: parse_gpu_unique_id(gpu_unique_id)?,
            ring: QueueBarrierRing::AqlSpecial,
        }),
        [mode, gpu_unique_id] if mode == "queue-barrier-executable" => Ok(Command::QueueBarrier {
            gpu_unique_id: parse_gpu_unique_id(gpu_unique_id)?,
            ring: QueueBarrierRing::Executable,
        }),
        [mode, gpu_unique_id] if mode == "queue-barrier-userptr" => Ok(Command::QueueBarrier {
            gpu_unique_id: parse_gpu_unique_id(gpu_unique_id)?,
            ring: QueueBarrierRing::Userptr,
        }),
        [mode, artifact_root, gpu_unique_id] if mode == "k7-smoke" => Ok(Command::K7Smoke {
            artifact_root: artifact_root.clone(),
            gpu_unique_id: parse_gpu_unique_id(gpu_unique_id)?,
        }),
        [mode, prepacked_root, artifact_root, gpu_unique_id] if mode == "k1-embedding" => {
            Ok(Command::K1Embedding {
                prepacked_root: prepacked_root.clone(),
                artifact_root: artifact_root.clone(),
                gpu_unique_id: parse_gpu_unique_id(gpu_unique_id)?,
            })
        }
        _ => Err(USAGE.to_owned()),
    }
}

fn parse_gpu_unique_id(value: &OsString) -> DiagnosticResult<u64> {
    value
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())
}

fn execute(command: Command) -> DiagnosticResult<()> {
    match command {
        Command::QueueBarrier {
            gpu_unique_id,
            ring,
        } => {
            let mode = match ring {
                QueueBarrierRing::AqlSpecial => "queue-barrier",
                QueueBarrierRing::Executable => "queue-barrier-executable",
                QueueBarrierRing::Userptr => "queue-barrier-userptr",
            };
            println!("mode={mode}");
            run_queue_barrier(bind_device(gpu_unique_id)?, ring)
        }
        Command::K7Smoke {
            artifact_root,
            gpu_unique_id,
        } => {
            println!("mode=k7-smoke");
            let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(&artifact_root))
                .map_err(|error| {
                    format!("cannot authenticate persisted kernel artifacts: {error}")
                })?;
            let checked = bind_device(gpu_unique_id)?;
            artifacts
                .with_content_bound_program_catalog_v1(|catalog| {
                    let mut report = |line| println!("{line}");
                    execute_m1_k7_s1k4_packet_v1(checked, catalog, &mut report)
                })
                .map_err(|error| format!("cannot bind content-bound program catalog: {error}"))??;
            Ok(())
        }
        Command::K1Embedding {
            prepacked_root,
            artifact_root,
            gpu_unique_id,
        } => {
            println!("mode=k1-embedding");
            println!("phase=authenticate-token-embedding");
            let embedding = load_target_token_embedding(Path::new(&prepacked_root))?;
            println!("token_embedding_bytes={}", embedding.len());
            let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(&artifact_root))
                .map_err(|error| {
                    format!("cannot authenticate persisted kernel artifacts: {error}")
                })?;
            let checked = bind_device(gpu_unique_id)?;
            artifacts
                .with_content_bound_program_catalog_v1(|catalog| {
                    let mut report = |line| println!("{line}");
                    execute_m1_k1_target_s1t128_packet_v1(checked, catalog, embedding, &mut report)
                })
                .map_err(|error| format!("cannot bind content-bound program catalog: {error}"))??;
            Ok(())
        }
    }
}

fn run_queue_barrier(
    checked: CheckedGfx942XnackMinusDevice,
    ring: QueueBarrierRing,
) -> DiagnosticResult<()> {
    let poll_bound = Gfx942BarrierProbePollBoundV1::new(BARRIER_COMPLETION_POLL_LIMIT)
        .map_err(|error| format!("cannot construct barrier poll bound: {error:?}"))?;
    println!("phase=queue-barrier");
    let result = match ring {
        QueueBarrierRing::AqlSpecial => {
            checked.run_compute_aql_barrier_probe(M1_PACKET_DIAGNOSTIC_RING_BYTES_V1, poll_bound)
        }
        QueueBarrierRing::Executable => checked.run_compute_aql_executable_ring_barrier_probe(
            M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
            poll_bound,
        ),
        QueueBarrierRing::Userptr => checked.run_compute_aql_userptr_ring_barrier_probe(
            M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
            poll_bound,
        ),
    };
    match result {
        Ok(success) => {
            let execution = success.execution();
            println!("ring_backing={:?}", success.backing());
            println!("poll_bound={}", success.poll_bound());
            println!("packet_count={}", execution.packet_count());
            println!("write_counter={}", execution.write_counter());
            println!("read_counter={}", execution.read_counter());
            println!("packet_header=0x{:04x}", execution.packet_header());
            println!("packet_setup={}", execution.packet_setup());
            println!("signal_kind={}", execution.signal_kind());
            println!("signal={:?}", execution.signal());
            println!(
                "queue_exception_reason_mask=0x{:x}",
                execution.queue_exception_reason_mask()
            );
            println!(
                "currentness_confirmed={}",
                execution.currentness_confirmed()
            );
            println!("recycled_signal_count={}", success.recycled_signal_count());
            println!(
                "released_queue_resources={}",
                success.destroyed().released_resources()
            );
            println!("status=completed-non-qualification");
            Ok(())
        }
        Err(failure) => barrier_probe_failure(failure),
    }
}

fn barrier_probe_failure(failure: Gfx942BarrierProbeFailureV1) -> DiagnosticResult<()> {
    let backing = failure.backing();
    let timeout = failure
        .timeout_observation()
        .map(|observation| format!("; timeout_observation={observation:?}"))
        .unwrap_or_default();
    match failure {
        Gfx942BarrierProbeFailureV1::Creation { error, .. } => Err(format!(
            "queue barrier creation failed with {backing:?}: {error}{timeout}; no live queue was returned"
        )),
        Gfx942BarrierProbeFailureV1::TerminalCreation { error, .. } => Err(format!(
            "queue barrier creation became terminal with {backing:?}: {error}{timeout}; no authority was recovered and process termination is required"
        )),
        Gfx942BarrierProbeFailureV1::QuarantinedExecution {
            error, retained, ..
        } => {
            std::mem::forget(retained);
            Err(format!(
                "queue barrier execution failed with {backing:?}: {error}{timeout}; queue quarantined until process teardown"
            ))
        }
        Gfx942BarrierProbeFailureV1::TerminalTeardown { error, .. } => Err(format!(
            "queue barrier teardown failed with {backing:?}: {error}{timeout}; no authority was recovered and process termination is required"
        )),
    }
}

fn bind_device(gpu_unique_id: u64) -> DiagnosticResult<CheckedGfx942XnackMinusDevice> {
    println!("phase=bind-device");
    OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))
}

fn load_target_token_embedding(root: &Path) -> DiagnosticResult<Box<[u8]>> {
    let directory = SecureDirectory::open(root)?;
    let admission = directory.read_exact(
        Path::new("bundle.admission.bin"),
        BUNDLE_ADMISSION_RECORD_BYTES as u64,
        "bundle admission record",
    )?;
    let descriptor = decode_bundle_admission_record(&admission)
        .map_err(|error| format!("cannot decode bundle admission record: {error}"))?;
    let manifest_bytes = directory.read_exact(
        Path::new("target.weights.manifest.bin"),
        u64::from(QWEN3_TARGET_PREPACKED_MANIFEST_BYTES),
        "target weight manifest",
    )?;
    let manifest = reopen_persisted_qwen3_weight_manifest(
        Qwen3ModelRole::Target8B,
        descriptor.target_manifest,
        &manifest_bytes,
    )
    .map_err(|error| format!("cannot authenticate target weight manifest: {error}"))?;
    let section = find_target_embedding(manifest.sections())?;
    let (offset, length) = section.destination_range();
    let bytes = directory.read_range(
        Path::new("target.weights.bin"),
        QWEN3_TARGET_TENSOR_DATA_BYTES,
        offset,
        length,
        "target token embedding",
    )?;
    let actual: [u8; 32] = Sha256::digest(&bytes).into();
    if actual != section.sha256() {
        return Err("target token embedding digest differs from its canonical manifest".to_owned());
    }
    Ok(bytes.into_boxed_slice())
}

fn find_target_embedding(sections: &[WeightSection]) -> DiagnosticResult<&WeightSection> {
    let mut found = None;
    for section in sections {
        let (metadata, ordinal) = section
            .qwen3_metadata()
            .map_err(|error| format!("cannot resolve target weight section: {error}"))?;
        if metadata.kind == Qwen3TensorKind::TokenEmbedding
            && metadata.layer == QWEN3_NO_LAYER
            && (ordinal != 0 || found.replace(section).is_some())
        {
            return Err("target manifest has a noncanonical token-embedding roster".to_owned());
        }
    }
    found.ok_or_else(|| "target manifest has no token-embedding section".to_owned())
}

impl SecureDirectory {
    fn open(path: &Path) -> DiagnosticResult<Self> {
        let descriptor = openat2(
            CWD,
            path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open prepacked snapshot root: {error}"))?;
        Ok(Self { descriptor })
    }

    fn open_file(&self, relative: &Path, description: &str) -> DiagnosticResult<SecureFile> {
        let descriptor = openat2(
            &self.descriptor,
            relative,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect opened {description}: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
            || initial.st_nlink != 1
        {
            return Err(format!(
                "{description} must be a regular file with exactly one filesystem link"
            ));
        }
        Ok(SecureFile {
            file: File::from(descriptor),
            initial,
        })
    }

    fn read_exact(
        &self,
        relative: &Path,
        expected_length: u64,
        description: &str,
    ) -> DiagnosticResult<Vec<u8>> {
        let mut input = self.open_file(relative, description)?;
        input.require_length(expected_length, description)?;
        input.read_range(0, expected_length, description)
    }

    fn read_range(
        &self,
        relative: &Path,
        expected_file_length: u64,
        offset: u64,
        length: u64,
        description: &str,
    ) -> DiagnosticResult<Vec<u8>> {
        let mut input = self.open_file(relative, description)?;
        input.require_length(expected_file_length, description)?;
        input.read_range(offset, length, description)
    }
}

impl SecureFile {
    fn require_length(&self, expected: u64, description: &str) -> DiagnosticResult<()> {
        if u64::try_from(self.initial.st_size).ok() != Some(expected) {
            return Err(format!("{description} length drifted"));
        }
        Ok(())
    }

    fn read_range(
        &mut self,
        offset: u64,
        length: u64,
        description: &str,
    ) -> DiagnosticResult<Vec<u8>> {
        let end = offset
            .checked_add(length)
            .ok_or_else(|| format!("{description} range overflowed"))?;
        if u64::try_from(self.initial.st_size)
            .ok()
            .is_none_or(|size| end > size)
        {
            return Err(format!("{description} range exceeds the opened file"));
        }
        let length = usize::try_from(length)
            .map_err(|_| format!("{description} length does not fit this host"))?;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|error| format!("cannot seek {description}: {error}"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| format!("cannot reserve {description} read buffer"))?;
        let read = (&mut self.file)
            .take(u64::try_from(length).unwrap_or(u64::MAX))
            .read_to_end(&mut bytes);
        let snapshot = self.validate_snapshot(description);
        if let Err(error) = read {
            snapshot?;
            return Err(format!("cannot read {description}: {error}"));
        }
        snapshot?;
        if bytes.len() != length {
            return Err(format!("{description} changed during the exact read"));
        }
        Ok(bytes)
    }

    fn validate_snapshot(&self, description: &str) -> DiagnosticResult<()> {
        let final_stat = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        if !same_file_snapshot(&self.initial, &final_stat) {
            return Err(format!("{description} changed while being read"));
        }
        Ok(())
    }
}

fn same_file_snapshot(initial: &Stat, final_stat: &Stat) -> bool {
    initial.st_dev == final_stat.st_dev
        && initial.st_ino == final_stat.st_ino
        && initial.st_mode == final_stat.st_mode
        && initial.st_nlink == final_stat.st_nlink
        && initial.st_size == final_stat.st_size
        && initial.st_mtime == final_stat.st_mtime
        && initial.st_mtime_nsec == final_stat.st_mtime_nsec
        && initial.st_ctime == final_stat.st_ctime
        && initial.st_ctime_nsec == final_stat.st_ctime_nsec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_modes_are_explicit_and_reject_extra_inputs() {
        assert!(matches!(
            parse_command(vec!["queue-barrier".into(), "5".into()]),
            Ok(Command::QueueBarrier {
                gpu_unique_id: 5,
                ring: QueueBarrierRing::AqlSpecial
            })
        ));
        assert!(matches!(
            parse_command(vec!["queue-barrier-executable".into(), "6".into()]),
            Ok(Command::QueueBarrier {
                gpu_unique_id: 6,
                ring: QueueBarrierRing::Executable
            })
        ));
        assert!(matches!(
            parse_command(vec!["queue-barrier-userptr".into(), "7".into()]),
            Ok(Command::QueueBarrier {
                gpu_unique_id: 7,
                ring: QueueBarrierRing::Userptr
            })
        ));
        assert!(parse_command(vec!["queue-barrier".into(), "not-decimal".into()]).is_err());
        assert!(
            parse_command(vec!["queue-barrier".into(), "18446744073709551616".into(),]).is_err()
        );
        assert!(parse_command(vec!["queue-barrier".into(), "5".into(), "extra".into(),]).is_err());
        assert!(parse_command(vec![
            "queue-barrier-userptr".into(),
            "7".into(),
            "extra".into(),
        ])
        .is_err());
        assert!(parse_command(Vec::new())
            .unwrap_err()
            .contains("ferric-m1-packet-diagnostic queue-barrier-userptr GPU-UNIQUE-ID"));
        assert!(matches!(
            parse_command(vec!["k7-smoke".into(), "artifacts".into(), "7".into()]),
            Ok(Command::K7Smoke {
                gpu_unique_id: 7,
                ..
            })
        ));
        assert!(matches!(
            parse_command(vec![
                "k1-embedding".into(),
                "snapshot".into(),
                "artifacts".into(),
                "9".into(),
            ]),
            Ok(Command::K1Embedding {
                gpu_unique_id: 9,
                ..
            })
        ));
        assert!(parse_command(vec!["k7-smoke".into()]).is_err());
        let barrier_bound = Gfx942BarrierProbePollBoundV1::new(BARRIER_COMPLETION_POLL_LIMIT)
            .expect("diagnostic poll count must remain within FE2O3's typed bound");
        assert_eq!(barrier_bound.get(), BARRIER_COMPLETION_POLL_LIMIT);
        assert!(BARRIER_COMPLETION_POLL_LIMIT < Gfx942BarrierProbePollBoundV1::maximum());
    }
}
