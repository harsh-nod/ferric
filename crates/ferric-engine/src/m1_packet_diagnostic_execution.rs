//! Physical execution for the two closed M1 packet-diagnostic shapes.

use fe2o3_kfd::{
    CheckedGfx942XnackMinusDevice, Gfx942DeviceContentDescriptorV1, Gfx942DeviceContentRoleV1,
};
use fe2o3_service_host::{
    DeviceInputRoleV1, DeviceOutputRoleV1, HostDownloadRoleV1, ServiceAllocationSessionV1,
    ServiceFixedBatchV1, ServiceFixedDispatchBufferV1, ServiceFixedDispatchPacketV1,
    ServiceHostDispatchRangeV1, ServicePublishedQueueSessionV1, ServiceQueuePollWithProgressV1,
    ServiceQueueSessionV1,
};
use std::thread;
use std::time::Duration;

use crate::{
    m1_k1_target_s1t128_packet_diagnostic_spec_v1, m1_k7_s1k4_packet_diagnostic_spec_v1,
    ContentBoundM1ProgramCatalogV1, M1PacketDiagnosticBufferV1, M1PacketDiagnosticSpecV1,
    M1_PACKET_DIAGNOSTIC_CONTENT_ROLE_IDENTITY_V1, M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
};

type DiagnosticResult<T> = Result<T, String>;

const DEVICE_ALLOCATION_ALIGNMENT: u64 = 4_096;
const COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const COMPLETION_POLL_LIMIT: u32 = 6_000;
const K7_ANCHOR: u32 = 10;
const K7_DRAFT_CHOICES: [u32; 4] = [11, 12, 13, 14];
const K7_EXPECTED_TARGET: [u32; 5] = [10, 11, 12, 13, 14];
const K1_INPUT_TOKEN: u32 = 1;
const K1_INPUT_TOKEN_COUNT: usize = 128;

/// Completed exact K7 S1K4 packet observation after queue teardown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1K7S1K4PacketObservationV1 {
    dispatch_generation: u64,
    output_tokens: [u32; 5],
}

impl M1K7S1K4PacketObservationV1 {
    /// Dispatch generation retained by the released queue resources.
    #[must_use]
    pub const fn dispatch_generation(self) -> u64 {
        self.dispatch_generation
    }

    /// Exact copied and checked K7 output tokens.
    #[must_use]
    pub const fn output_tokens(self) -> [u32; 5] {
        self.output_tokens
    }
}

/// Completed exact K1 S1T128 packet observation after queue teardown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1K1S1T128PacketObservationV1 {
    dispatch_generation: u64,
}

impl M1K1S1T128PacketObservationV1 {
    /// Dispatch generation retained by the released queue resources.
    #[must_use]
    pub const fn dispatch_generation(self) -> u64 {
        self.dispatch_generation
    }
}

/// Executes and checks one exact K7 S1K4 packet on the selected physical device.
///
/// The reporter receives the diagnostic binary's stable line-oriented progress
/// records. A structured caller may supply a no-op reporter.
///
/// # Errors
///
/// Returns a diagnostic if allocation, queue creation, publication, completion,
/// readback, semantic checking, or teardown does not finish exactly.
pub fn execute_m1_k7_s1k4_packet_v1(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<M1K7S1K4PacketObservationV1> {
    let spec = m1_k7_s1k4_packet_diagnostic_spec_v1()
        .map_err(|error| format!("cannot build exact K7 packet: {error}"))?;
    report(format!("program_index={}", spec.program().program_index()));
    report(format!("grid={:?}", spec.geometry().grid()));
    report(format!("workgroup={:?}", spec.geometry().workgroup()));
    let anchor = K7_ANCHOR.to_le_bytes().to_vec().into_boxed_slice();
    let draft = u32_bytes(&K7_DRAFT_CHOICES);
    let output = vec![0_u8; K7_EXPECTED_TARGET.len() * size_of::<u32>()].into_boxed_slice();
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    report("phase=allocate".to_owned());
    let prepared = prepare_k7(&mut allocations, spec, anchor, draft, output);
    let (packet, output_range) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return release_unpublished_after_error(allocations, error),
    };
    run_readback_packet(allocations, catalog, packet, output_range, report)
}

/// Executes one exact K1 target S1T128 token-embedding packet.
///
/// # Errors
///
/// Returns a diagnostic if allocation, queue creation, publication, completion,
/// or teardown does not finish exactly.
pub fn execute_m1_k1_target_s1t128_packet_v1(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    embedding: Box<[u8]>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<M1K1S1T128PacketObservationV1> {
    let spec = m1_k1_target_s1t128_packet_diagnostic_spec_v1()
        .map_err(|error| format!("cannot build exact K1 packet: {error}"))?;
    report(format!("program_index={}", spec.program().program_index()));
    report(format!("grid={:?}", spec.geometry().grid()));
    report(format!("workgroup={:?}", spec.geometry().workgroup()));
    let tokens = u32_bytes(&[K1_INPUT_TOKEN; K1_INPUT_TOKEN_COUNT]);
    let output_length = usize::try_from(spec.buffers()[2].byte_len())
        .map_err(|_| "K1 output length does not fit this host".to_owned())?;
    let output = vec![0_u8; output_length].into_boxed_slice();
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    report("phase=allocate".to_owned());
    let packet = match prepare_k1(&mut allocations, spec, tokens, embedding, output) {
        Ok(packet) => packet,
        Err(error) => return release_unpublished_after_error(allocations, error),
    };
    run_completion_only_packet(allocations, catalog, packet, report)
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
    contract: M1PacketDiagnosticBufferV1,
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
    report: &mut impl FnMut(String),
) -> DiagnosticResult<M1K7S1K4PacketObservationV1> {
    let completed = publish_and_complete(allocations, catalog, packet, report)?;
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
    let output_tokens: [u32; 5] = match actual.as_slice().try_into() {
        Ok(output_tokens) => output_tokens,
        Err(_) => {
            return destroy_recycled_after_error(
                recycled,
                "completed K7 output length drifted".to_owned(),
            )
        }
    };
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy K7 queue and release allocations: {failure:?}")
    })?;
    let dispatch_generation = released.dispatch_generation();
    report(format!("dispatch_generation={dispatch_generation}"));
    report(format!("output_tokens={actual:?}"));
    report("output_verified=true".to_owned());
    report("status=completed-non-qualification".to_owned());
    Ok(M1K7S1K4PacketObservationV1 {
        dispatch_generation,
        output_tokens,
    })
}

fn run_completion_only_packet(
    allocations: ServiceAllocationSessionV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    packet: ServiceFixedDispatchPacketV1,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<M1K1S1T128PacketObservationV1> {
    let completed = publish_and_complete(allocations, catalog, packet, report)?;
    let recycled = completed
        .recycle()
        .map_err(queue_operation_error("cannot recycle completed K1 packet"))?;
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy K1 queue and release allocations: {failure:?}")
    })?;
    let dispatch_generation = released.dispatch_generation();
    report(format!("dispatch_generation={dispatch_generation}"));
    report("completion_observed=true".to_owned());
    report("output_observed=false".to_owned());
    report("status=completed-non-qualification".to_owned());
    Ok(M1K1S1T128PacketObservationV1 {
        dispatch_generation,
    })
}

fn publish_and_complete(
    allocations: ServiceAllocationSessionV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    packet: ServiceFixedDispatchPacketV1,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<fe2o3_service_host::ServiceCompletedQueueSessionV1<1>> {
    let batch = ServiceFixedBatchV1::new(catalog.into_programs(), [packet]);
    report("phase=create-queue".to_owned());
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
    report("phase=submit".to_owned());
    let published = queue
        .submit()
        .map_err(queue_operation_error("cannot submit diagnostic packet"))?;
    wait_with_pacing(published, report)
}

fn wait_with_pacing(
    mut published: ServicePublishedQueueSessionV1<1>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<fe2o3_service_host::ServiceCompletedQueueSessionV1<1>> {
    report("phase=wait".to_owned());
    for scan in 1..=COMPLETION_POLL_LIMIT {
        match published
            .poll_with_progress()
            .map_err(queue_operation_error("diagnostic completion poll failed"))?
        {
            ServiceQueuePollWithProgressV1::Pending { session, progress } => {
                if scan == 1 || scan.is_multiple_of(100) {
                    report(format!(
                        "completion_scan={scan} completed={} pending={} first_pending={:?}",
                        progress.completed_count(),
                        progress.pending_count(),
                        progress.first_pending_batch_index()
                    ));
                }
                published = session;
                thread::sleep(COMPLETION_POLL_INTERVAL);
            }
            ServiceQueuePollWithProgressV1::Ready { session, progress } => {
                report(format!(
                    "completion_scan={scan} completed={} pending={}",
                    progress.completed_count(),
                    progress.pending_count()
                ));
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
