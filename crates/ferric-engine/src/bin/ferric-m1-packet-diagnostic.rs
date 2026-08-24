//! One-packet hardware diagnostics for the Ferric M1 service queue path.

use fe2o3_kfd::{
    CheckedGfx942XnackMinusDevice, DeviceSelector, Gfx942BarrierProbeFailureV1,
    Gfx942BarrierProbePollBoundV1, Gfx942DeviceContentDescriptorV1, Gfx942DeviceContentRoleV1,
    OpenedKfd,
};
use fe2o3_service_host::{
    DeviceInputRoleV1, DeviceOutputRoleV1, HostDownloadRoleV1, ServiceAllocationSessionV1,
    ServiceFixedBatchV1, ServiceFixedDispatchBufferV1, ServiceFixedDispatchPacketV1,
    ServiceHostDispatchRangeV1, ServicePublishedQueueSessionV1, ServiceQueuePollWithProgressV1,
    ServiceQueueSessionV1,
};
use ferric_build::{
    decode_bundle_admission_record, reopen_persisted_qwen3_weight_manifest, WeightSection,
    BUNDLE_ADMISSION_RECORD_BYTES, QWEN3_TARGET_PREPACKED_MANIFEST_BYTES,
};
use ferric_engine::{
    m1_k1_target_s1t128_packet_diagnostic_spec_v1, m1_k7_s1k4_packet_diagnostic_spec_v1,
    reopen_persisted_m1_kernel_artifacts_v1, ContentBoundM1ProgramCatalogV1,
    M1PacketDiagnosticSpecV1, M1_PACKET_DIAGNOSTIC_CONTENT_ROLE_IDENTITY_V1,
    M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
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
use std::thread;
use std::time::Duration;

type DiagnosticResult<T> = Result<T, String>;

const DEVICE_ALLOCATION_ALIGNMENT: u64 = 4_096;
const BARRIER_COMPLETION_POLL_LIMIT: u32 = 1_000;
const COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const COMPLETION_POLL_LIMIT: u32 = 6_000;
const K7_ANCHOR: u32 = 10;
const K7_DRAFT_CHOICES: [u32; 4] = [11, 12, 13, 14];
const K7_EXPECTED_TARGET: [u32; 5] = [10, 11, 12, 13, 14];
const K1_INPUT_TOKEN: u32 = 1;
const K1_INPUT_TOKEN_COUNT: usize = 128;

#[derive(Debug)]
enum Command {
    QueueBarrier {
        gpu_unique_id: u64,
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
        _ => Err("usage: ferric-m1-packet-diagnostic queue-barrier GPU-UNIQUE-ID\n       ferric-m1-packet-diagnostic k7-smoke KERNEL-ARTIFACTS GPU-UNIQUE-ID\n       ferric-m1-packet-diagnostic k1-embedding PREPACKED-SNAPSHOT KERNEL-ARTIFACTS GPU-UNIQUE-ID".to_owned()),
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
        Command::QueueBarrier { gpu_unique_id } => {
            println!("mode=queue-barrier");
            run_queue_barrier(bind_device(gpu_unique_id)?)
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
                .with_content_bound_program_catalog_v1(|catalog| run_k7(checked, catalog))
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
                    run_k1(checked, catalog, embedding)
                })
                .map_err(|error| format!("cannot bind content-bound program catalog: {error}"))??;
            Ok(())
        }
    }
}

fn run_queue_barrier(checked: CheckedGfx942XnackMinusDevice) -> DiagnosticResult<()> {
    let poll_bound = Gfx942BarrierProbePollBoundV1::new(BARRIER_COMPLETION_POLL_LIMIT)
        .map_err(|error| format!("cannot construct barrier poll bound: {error:?}"))?;
    println!("phase=queue-barrier");
    let result =
        checked.run_compute_aql_barrier_probe(M1_PACKET_DIAGNOSTIC_RING_BYTES_V1, poll_bound);
    match result {
        Ok(success) => {
            let execution = success.execution();
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
    let timeout = failure
        .timeout_observation()
        .map(|observation| format!("; timeout_observation={observation:?}"))
        .unwrap_or_default();
    match failure {
        Gfx942BarrierProbeFailureV1::Creation { error } => Err(format!(
            "queue barrier creation failed: {error}{timeout}; no live queue was returned"
        )),
        Gfx942BarrierProbeFailureV1::QuarantinedExecution { error, retained } => {
            std::mem::forget(retained);
            Err(format!(
                "queue barrier execution failed: {error}{timeout}; queue quarantined until process teardown"
            ))
        }
        Gfx942BarrierProbeFailureV1::TerminalTeardown { error } => Err(format!(
            "queue barrier teardown failed: {error}{timeout}; no authority was recovered and process termination is required"
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

fn run_k7(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
) -> DiagnosticResult<()> {
    let spec = m1_k7_s1k4_packet_diagnostic_spec_v1()
        .map_err(|error| format!("cannot build exact K7 packet: {error}"))?;
    println!("program_index={}", spec.program().program_index());
    println!("grid={:?}", spec.geometry().grid());
    println!("workgroup={:?}", spec.geometry().workgroup());
    let anchor = K7_ANCHOR.to_le_bytes().to_vec().into_boxed_slice();
    let draft = u32_bytes(&K7_DRAFT_CHOICES);
    let output = vec![0_u8; K7_EXPECTED_TARGET.len() * size_of::<u32>()].into_boxed_slice();
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    println!("phase=allocate");
    let prepared = prepare_k7(&mut allocations, spec, anchor, draft, output);
    let (packet, output_range) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return release_unpublished_after_error(allocations, error),
    };
    run_readback_packet(allocations, catalog, packet, output_range)
}

fn prepare_k7(
    allocations: &mut ServiceAllocationSessionV1,
    spec: M1PacketDiagnosticSpecV1,
    anchor: Box<[u8]>,
    draft: Box<[u8]>,
    output: Box<[u8]>,
) -> DiagnosticResult<(ServiceFixedDispatchPacketV1, ServiceHostDispatchRangeV1)> {
    let anchor_range = allocate_device_input(allocations, anchor, 0, spec.buffers()[0])?;
    let draft_range = allocate_device_input(allocations, draft, 1, spec.buffers()[1])?;
    let output_contract = spec.buffers()[2];
    require_extent(output.len(), output_contract.byte_len(), "K7 output")?;
    let output_key = allocations
        .allocate_initialized_host_visible::<HostDownloadRoleV1>(output)
        .map_err(|error| format!("cannot allocate K7 host-visible output: {error}"))?;
    let output_range = allocations
        .range(
            output_key,
            0,
            output_contract.byte_len(),
            output_contract.alignment(),
        )
        .and_then(|range| allocations.host_dispatch_range(range))
        .map_err(|error| format!("cannot bind K7 host-visible output: {error}"))?;
    let (program, geometry, dynamic_group_bytes, kernarg, buffers) = spec.into_packet_parts();
    let bindings = vec![
        ServiceFixedDispatchBufferV1::new(buffers[0].explicit_argument_index(), anchor_range),
        ServiceFixedDispatchBufferV1::new(buffers[1].explicit_argument_index(), draft_range),
        ServiceFixedDispatchBufferV1::new_host_visible(
            buffers[2].explicit_argument_index(),
            output_range,
        ),
    ]
    .into_boxed_slice();
    Ok((
        ServiceFixedDispatchPacketV1::new_independent(
            program.program_index(),
            geometry,
            dynamic_group_bytes,
            kernarg,
            bindings,
        ),
        output_range,
    ))
}

fn run_k1(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    embedding: Box<[u8]>,
) -> DiagnosticResult<()> {
    let spec = m1_k1_target_s1t128_packet_diagnostic_spec_v1()
        .map_err(|error| format!("cannot build exact K1 packet: {error}"))?;
    println!("program_index={}", spec.program().program_index());
    println!("grid={:?}", spec.geometry().grid());
    println!("workgroup={:?}", spec.geometry().workgroup());
    let tokens = u32_bytes(&[K1_INPUT_TOKEN; K1_INPUT_TOKEN_COUNT]);
    let output_length = usize::try_from(spec.buffers()[2].byte_len())
        .map_err(|_| "K1 output length does not fit this host".to_owned())?;
    let output = vec![0_u8; output_length].into_boxed_slice();
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    println!("phase=allocate");
    let packet = match prepare_k1(&mut allocations, spec, tokens, embedding, output) {
        Ok(packet) => packet,
        Err(error) => return release_unpublished_after_error(allocations, error),
    };
    run_completion_only_packet(allocations, catalog, packet)
}

fn prepare_k1(
    allocations: &mut ServiceAllocationSessionV1,
    spec: M1PacketDiagnosticSpecV1,
    tokens: Box<[u8]>,
    embedding: Box<[u8]>,
    output: Box<[u8]>,
) -> DiagnosticResult<ServiceFixedDispatchPacketV1> {
    let token_range = allocate_device_input(allocations, tokens, 2, spec.buffers()[0])?;
    let embedding_range = allocate_device_input(allocations, embedding, 3, spec.buffers()[1])?;
    let output_contract = spec.buffers()[2];
    require_extent(output.len(), output_contract.byte_len(), "K1 output")?;
    let descriptor = content_descriptor(4, &output)?;
    let output_key = allocations
        .allocate_initialized_device_local::<DeviceOutputRoleV1>(
            output,
            DEVICE_ALLOCATION_ALIGNMENT,
            descriptor,
        )
        .map_err(|error| format!("cannot allocate K1 device output: {error}"))?;
    let output_range = allocations
        .range(
            output_key,
            0,
            output_contract.byte_len(),
            output_contract.alignment(),
        )
        .and_then(|range| allocations.device_dispatch_range(range))
        .map_err(|error| format!("cannot bind K1 device output: {error}"))?;
    let (program, geometry, dynamic_group_bytes, kernarg, buffers) = spec.into_packet_parts();
    let bindings = vec![
        ServiceFixedDispatchBufferV1::new(buffers[0].explicit_argument_index(), token_range),
        ServiceFixedDispatchBufferV1::new(buffers[1].explicit_argument_index(), embedding_range),
        ServiceFixedDispatchBufferV1::new(buffers[2].explicit_argument_index(), output_range),
    ]
    .into_boxed_slice();
    Ok(ServiceFixedDispatchPacketV1::new_independent(
        program.program_index(),
        geometry,
        dynamic_group_bytes,
        kernarg,
        bindings,
    ))
}

fn allocate_device_input(
    allocations: &mut ServiceAllocationSessionV1,
    bytes: Box<[u8]>,
    content_ordinal: u32,
    contract: ferric_engine::M1PacketDiagnosticBufferV1,
) -> DiagnosticResult<fe2o3_service_host::ServiceDeviceDispatchRangeV1> {
    require_extent(bytes.len(), contract.byte_len(), "device input")?;
    let descriptor = content_descriptor(content_ordinal, &bytes)?;
    let key = allocations
        .allocate_initialized_device_local::<DeviceInputRoleV1>(
            bytes,
            DEVICE_ALLOCATION_ALIGNMENT,
            descriptor,
        )
        .map_err(|error| format!("cannot allocate initialized device input: {error}"))?;
    allocations
        .range(key, 0, contract.byte_len(), contract.alignment())
        .and_then(|range| allocations.device_dispatch_range(range))
        .map_err(|error| format!("cannot bind initialized device input: {error}"))
}

fn content_descriptor(
    ordinal: u32,
    bytes: &[u8],
) -> DiagnosticResult<Gfx942DeviceContentDescriptorV1> {
    let role =
        Gfx942DeviceContentRoleV1::new(M1_PACKET_DIAGNOSTIC_CONTENT_ROLE_IDENTITY_V1, ordinal)
            .map_err(|error| format!("cannot construct diagnostic content role: {error}"))?;
    Gfx942DeviceContentDescriptorV1::from_bytes(role, bytes)
        .map_err(|error| format!("cannot describe diagnostic content: {error}"))
}

fn require_extent(actual: usize, expected: u64, description: &str) -> DiagnosticResult<()> {
    if u64::try_from(actual).ok() != Some(expected) {
        return Err(format!(
            "{description} extent drifted: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

fn run_readback_packet(
    allocations: ServiceAllocationSessionV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    packet: ServiceFixedDispatchPacketV1,
    output_range: ServiceHostDispatchRangeV1,
) -> DiagnosticResult<()> {
    let completed = publish_and_complete(allocations, catalog, packet)?;
    let mut recycled = completed
        .recycle()
        .map_err(queue_operation_error("cannot recycle completed K7 packet"))?;
    let request = recycled.completed_read_request(output_range);
    let readback = match recycled.read_completed(request) {
        Ok(readback) => readback,
        Err(error) => {
            return destroy_recycled_after_error(
                recycled,
                format!("cannot read completed K7 output: {error}"),
            )
        }
    };
    let actual = match decode_u32s(readback.bytes()) {
        Ok(actual) => actual,
        Err(error) => return destroy_recycled_after_error(recycled, error),
    };
    if actual != K7_EXPECTED_TARGET {
        return destroy_recycled_after_error(
            recycled,
            format!("K7 output mismatch: expected {K7_EXPECTED_TARGET:?}, got {actual:?}"),
        );
    }
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy K7 queue and release allocations: {failure:?}")
    })?;
    println!("dispatch_generation={}", released.dispatch_generation());
    println!("output_tokens={actual:?}");
    println!("output_verified=true");
    println!("status=completed-non-qualification");
    Ok(())
}

fn run_completion_only_packet(
    allocations: ServiceAllocationSessionV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    packet: ServiceFixedDispatchPacketV1,
) -> DiagnosticResult<()> {
    let completed = publish_and_complete(allocations, catalog, packet)?;
    let recycled = completed
        .recycle()
        .map_err(queue_operation_error("cannot recycle completed K1 packet"))?;
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy K1 queue and release allocations: {failure:?}")
    })?;
    println!("dispatch_generation={}", released.dispatch_generation());
    println!("completion_observed=true");
    println!("output_observed=false");
    println!("status=completed-non-qualification");
    Ok(())
}

fn publish_and_complete(
    allocations: ServiceAllocationSessionV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    packet: ServiceFixedDispatchPacketV1,
) -> DiagnosticResult<fe2o3_service_host::ServiceCompletedQueueSessionV1<1>> {
    let batch = ServiceFixedBatchV1::new(catalog.into_programs(), [packet]);
    println!("phase=create-queue");
    let queue = match ServiceQueueSessionV1::<1>::create(
        allocations,
        M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
        batch,
    ) {
        Ok(queue) => queue,
        Err(failure) => {
            let diagnostic = format!(
                "cannot create diagnostic service queue: {}",
                failure.error()
            );
            if let Some((allocations, _batch)) = failure.into_rejected_inputs() {
                return release_unpublished_after_error(allocations, diagnostic);
            }
            return Err(format!("{diagnostic}; creation failure is terminal"));
        }
    };
    println!("phase=submit");
    let published = queue
        .submit()
        .map_err(queue_operation_error("cannot submit diagnostic packet"))?;
    wait_with_pacing(published)
}

fn wait_with_pacing(
    mut published: ServicePublishedQueueSessionV1<1>,
) -> DiagnosticResult<fe2o3_service_host::ServiceCompletedQueueSessionV1<1>> {
    println!("phase=wait");
    for scan in 1..=COMPLETION_POLL_LIMIT {
        match published
            .poll_with_progress()
            .map_err(queue_operation_error("diagnostic completion poll failed"))?
        {
            ServiceQueuePollWithProgressV1::Pending { session, progress } => {
                if scan == 1 || scan.is_multiple_of(100) {
                    println!(
                        "completion_scan={scan} completed={} pending={} first_pending={:?}",
                        progress.completed_count(),
                        progress.pending_count(),
                        progress.first_pending_batch_index()
                    );
                }
                published = session;
                thread::sleep(COMPLETION_POLL_INTERVAL);
            }
            ServiceQueuePollWithProgressV1::Ready { session, progress } => {
                println!(
                    "completion_scan={scan} completed={} pending={}",
                    progress.completed_count(),
                    progress.pending_count()
                );
                return Ok(session);
            }
        }
    }
    published.wait(0).map_err(queue_operation_error(
        "diagnostic completion deadline expired",
    ))
}

fn queue_operation_error(
    context: &'static str,
) -> impl FnOnce(fe2o3_service_host::ServiceQueueOperationFailureV1) -> String {
    move |failure| {
        let error = format!("{}", failure.error());
        let timeout = failure
            .timeout_observation()
            .map(|observation| format!("{observation:?}"));
        let _quarantined = failure.into_quarantined();
        match timeout {
            Some(observation) => format!(
                "{context}: {error}; timeout_observation={observation}; queue quarantined until process teardown"
            ),
            None => format!("{context}: {error}; queue quarantined until process teardown"),
        }
    }
}

fn release_unpublished_after_error<T>(
    allocations: ServiceAllocationSessionV1,
    error: String,
) -> DiagnosticResult<T> {
    match allocations.release_unpublished() {
        Ok(_) => Err(error),
        Err(failure) => {
            let release_error = format!("{:?}", failure.error());
            let _quarantined = failure.into_retained();
            Err(format!(
                "{error}; unpublished allocation release failed: {release_error}"
            ))
        }
    }
}

fn destroy_recycled_after_error<T>(
    recycled: fe2o3_service_host::ServiceRecycledQueueSessionV1<1>,
    error: String,
) -> DiagnosticResult<T> {
    match recycled.destroy_and_release() {
        Ok(_) => Err(error),
        Err(failure) => Err(format!(
            "{error}; completed queue teardown also failed: {failure:?}"
        )),
    }
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

fn u32_bytes(values: &[u32]) -> Box<[u8]> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn decode_u32s(bytes: &[u8]) -> DiagnosticResult<Vec<u32>> {
    if !bytes.len().is_multiple_of(size_of::<u32>()) {
        return Err("completed K7 output is not an integral u32 array".to_owned());
    }
    bytes
        .chunks_exact(size_of::<u32>())
        .map(|chunk| {
            let encoded: [u8; 4] = chunk
                .try_into()
                .map_err(|_| "completed K7 output chunk drifted".to_owned())?;
            Ok(u32::from_le_bytes(encoded))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_modes_are_explicit_and_reject_extra_inputs() {
        assert!(matches!(
            parse_command(vec!["queue-barrier".into(), "5".into()]),
            Ok(Command::QueueBarrier { gpu_unique_id: 5 })
        ));
        assert!(parse_command(vec!["queue-barrier".into(), "not-decimal".into()]).is_err());
        assert!(
            parse_command(vec!["queue-barrier".into(), "18446744073709551616".into(),]).is_err()
        );
        assert!(parse_command(vec!["queue-barrier".into(), "5".into(), "extra".into(),]).is_err());
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

    #[test]
    fn fixed_packet_inputs_are_exact_little_endian_images() {
        assert_eq!(u32_bytes(&K7_DRAFT_CHOICES).len(), 16);
        assert_eq!(
            decode_u32s(&u32_bytes(&K7_EXPECTED_TARGET)).unwrap(),
            K7_EXPECTED_TARGET
        );
        let tokens = u32_bytes(&[K1_INPUT_TOKEN; K1_INPUT_TOKEN_COUNT]);
        assert_eq!(tokens.len(), 512);
        assert!(decode_u32s(&tokens)
            .unwrap()
            .into_iter()
            .all(|token| token == K1_INPUT_TOKEN));
    }
}
