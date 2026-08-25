//! Non-qualification K7 probes for ordered and multi-packet queue publication.

use fe2o3_aql::AqlDispatchOrderingV1;
use fe2o3_kfd::{
    CheckedGfx942XnackMinusDevice, Gfx942DeviceContentDescriptorV1, Gfx942DeviceContentRoleV1,
};
use fe2o3_service_host::{
    DeviceInputRoleV1, HostDownloadRoleV1, ServiceAllocationSessionV1, ServiceFixedBatchV1,
    ServiceFixedDispatchBufferV1, ServiceFixedDispatchPacketV1, ServiceHostDispatchRangeV1,
    ServicePublishedQueueSessionV1, ServiceQueuePollWithProgressV1, ServiceQueueSessionV1,
};
use ferric_engine::{
    m1_k7_s1k4_packet_diagnostic_spec_v1, ContentBoundM1ProgramCatalogV1,
    M1PacketDiagnosticBufferV1, M1PacketDiagnosticSpecV1,
    M1_PACKET_DIAGNOSTIC_CONTENT_ROLE_IDENTITY_V1, M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
};
use std::thread;
use std::time::Duration;

type DiagnosticResult<T> = Result<T, String>;

const COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const COMPLETION_POLL_LIMIT: u32 = 6_000;
const FIRST_ANCHOR: u32 = 10;
const FIRST_DRAFT: [u32; 4] = [11, 12, 13, 14];
const FIRST_EXPECTED: [u32; 5] = [10, 11, 12, 13, 14];
const SECOND_ANCHOR: u32 = 20;
const SECOND_DRAFT: [u32; 4] = [21, 22, 23, 24];
const SECOND_EXPECTED: [u32; 5] = [20, 21, 22, 23, 24];

pub(super) fn execute_ordered_single(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<()> {
    let spec = k7_spec()?;
    report_spec(&spec, report);
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    report("phase=allocate".to_owned());
    let (packet, output) = match prepare_k7(&mut allocations, spec, FIRST_ANCHOR, &FIRST_DRAFT, 0) {
        Ok(prepared) => prepared,
        Err(error) => return release_unpublished_after_error(allocations, error),
    };
    let completed = publish_and_complete(allocations, catalog, [packet], report)?;
    let mut recycled = completed
        .recycle()
        .map_err(queue_operation_error("cannot recycle ordered K7 packet"))?;
    if let Err(error) = verify_output(&mut recycled, output, &FIRST_EXPECTED, "ordered K7") {
        return destroy_recycled_after_error(recycled, error);
    }
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy ordered K7 queue and release allocations: {failure:?}")
    })?;
    report(format!(
        "dispatch_generation={}",
        released.dispatch_generation()
    ));
    report("packet_count=1".to_owned());
    report("ordering=wait-for-prior".to_owned());
    report("output_verified=true".to_owned());
    report("status=completed-non-qualification".to_owned());
    Ok(())
}

pub(super) fn execute_ordered_pair(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<()> {
    let first_spec = k7_spec()?;
    let second_spec = k7_spec()?;
    report_spec(&first_spec, report);
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    report("phase=allocate".to_owned());
    let ([first_packet, second_packet], [first_output, second_output]) = match prepare_k7_pair(
        &mut allocations,
        first_spec,
        second_spec,
        AqlDispatchOrderingV1::WaitForPrior,
    ) {
        Ok(prepared) => prepared,
        Err(error) => return release_unpublished_after_error(allocations, error),
    };
    let completed =
        publish_and_complete(allocations, catalog, [first_packet, second_packet], report)?;
    let mut recycled = completed
        .recycle()
        .map_err(queue_operation_error("cannot recycle ordered K7 pair"))?;
    if let Err(error) = verify_output(
        &mut recycled,
        first_output,
        &FIRST_EXPECTED,
        "first ordered K7",
    ) {
        return destroy_recycled_after_error(recycled, error);
    }
    if let Err(error) = verify_output(
        &mut recycled,
        second_output,
        &SECOND_EXPECTED,
        "second ordered K7",
    ) {
        return destroy_recycled_after_error(recycled, error);
    }
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy ordered K7 pair and release allocations: {failure:?}")
    })?;
    report(format!(
        "dispatch_generation={}",
        released.dispatch_generation()
    ));
    report("packet_count=2".to_owned());
    report("ordering=wait-for-prior".to_owned());
    report("outputs_verified=2".to_owned());
    report("status=completed-non-qualification".to_owned());
    Ok(())
}

pub(super) fn execute_independent_pair(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<()> {
    let first_spec = k7_spec()?;
    let second_spec = k7_spec()?;
    report_spec(&first_spec, report);
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    report("phase=allocate".to_owned());
    let ([first_packet, second_packet], [first_output, second_output]) = match prepare_k7_pair(
        &mut allocations,
        first_spec,
        second_spec,
        AqlDispatchOrderingV1::Independent,
    ) {
        Ok(prepared) => prepared,
        Err(error) => return release_unpublished_after_error(allocations, error),
    };
    let completed =
        publish_and_complete(allocations, catalog, [first_packet, second_packet], report)?;
    let mut recycled = completed
        .recycle()
        .map_err(queue_operation_error("cannot recycle independent K7 pair"))?;
    if let Err(error) = verify_output(
        &mut recycled,
        first_output,
        &FIRST_EXPECTED,
        "first independent K7",
    ) {
        return destroy_recycled_after_error(recycled, error);
    }
    if let Err(error) = verify_output(
        &mut recycled,
        second_output,
        &SECOND_EXPECTED,
        "second independent K7",
    ) {
        return destroy_recycled_after_error(recycled, error);
    }
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy independent K7 pair and release allocations: {failure:?}")
    })?;
    report(format!(
        "dispatch_generation={}",
        released.dispatch_generation()
    ));
    report("packet_count=2".to_owned());
    report("ordering=independent".to_owned());
    report("outputs_verified=2".to_owned());
    report("status=completed-non-qualification".to_owned());
    Ok(())
}

pub(super) fn execute_independent_pair_shared_inputs(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<()> {
    execute_independent_pair_shared_inputs_impl(checked, catalog, 0, report)
}

pub(super) fn execute_independent_pair_shared_inputs_with_one_unreferenced(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<()> {
    execute_independent_pair_shared_inputs_impl(checked, catalog, 1, report)
}

pub(super) fn execute_independent_pair_shared_inputs_with_unreferenced(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<()> {
    execute_independent_pair_shared_inputs_impl(checked, catalog, 2, report)
}

fn execute_independent_pair_shared_inputs_impl(
    checked: CheckedGfx942XnackMinusDevice,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    unreferenced_input_count: u8,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<()> {
    let first_spec = k7_spec()?;
    let second_spec = k7_spec()?;
    report_spec(&first_spec, report);
    let mut allocations = ServiceAllocationSessionV1::acquire(checked)
        .map_err(|error| format!("cannot acquire service allocation session: {error:?}"))?;
    report("phase=allocate".to_owned());
    let anchor = allocate_device_input(
        &mut allocations,
        FIRST_ANCHOR.to_le_bytes().to_vec().into_boxed_slice(),
        0,
        first_spec.buffers()[0],
    )?;
    let draft = allocate_device_input(
        &mut allocations,
        u32_bytes(&FIRST_DRAFT),
        1,
        first_spec.buffers()[1],
    )?;
    if unreferenced_input_count >= 1 {
        let _second_anchor = allocate_device_input(
            &mut allocations,
            SECOND_ANCHOR.to_le_bytes().to_vec().into_boxed_slice(),
            2,
            second_spec.buffers()[0],
        )?;
    }
    if unreferenced_input_count >= 2 {
        let _second_draft = allocate_device_input(
            &mut allocations,
            u32_bytes(&SECOND_DRAFT),
            3,
            second_spec.buffers()[1],
        )?;
    }
    let first_output = allocate_host_output(&mut allocations, first_spec.buffers()[2])?;
    let second_output = allocate_host_output(&mut allocations, second_spec.buffers()[2])?;
    let first_packet = build_k7_packet(
        first_spec,
        anchor,
        draft,
        first_output,
        AqlDispatchOrderingV1::Independent,
    );
    let second_packet = build_k7_packet(
        second_spec,
        anchor,
        draft,
        second_output,
        AqlDispatchOrderingV1::Independent,
    );
    let completed =
        publish_and_complete(allocations, catalog, [first_packet, second_packet], report)?;
    let mut recycled = completed
        .recycle()
        .map_err(queue_operation_error("cannot recycle shared-input K7 pair"))?;
    if let Err(error) = verify_output(
        &mut recycled,
        first_output,
        &FIRST_EXPECTED,
        "first shared-input K7",
    ) {
        return destroy_recycled_after_error(recycled, error);
    }
    if let Err(error) = verify_output(
        &mut recycled,
        second_output,
        &FIRST_EXPECTED,
        "second shared-input K7",
    ) {
        return destroy_recycled_after_error(recycled, error);
    }
    let released = recycled.destroy_and_release().map_err(|failure| {
        format!("cannot destroy shared-input K7 pair and release allocations: {failure:?}")
    })?;
    report(format!(
        "dispatch_generation={}",
        released.dispatch_generation()
    ));
    report("packet_count=2".to_owned());
    report("ordering=independent".to_owned());
    report(
        match unreferenced_input_count {
            0 => "data_roster=shared-singleton-inputs",
            1 => "data_roster=shared-inputs-plus-one-unreferenced-device",
            _ => "data_roster=shared-inputs-plus-two-unreferenced-device",
        }
        .to_owned(),
    );
    report("outputs_verified=2".to_owned());
    report("status=completed-non-qualification".to_owned());
    Ok(())
}

fn k7_spec() -> DiagnosticResult<M1PacketDiagnosticSpecV1> {
    m1_k7_s1k4_packet_diagnostic_spec_v1()
        .map_err(|error| format!("cannot build exact K7 packet: {error}"))
}

fn report_spec(spec: &M1PacketDiagnosticSpecV1, report: &mut impl FnMut(String)) {
    report(format!("program_index={}", spec.program().program_index()));
    report(format!("grid={:?}", spec.geometry().grid()));
    report(format!("workgroup={:?}", spec.geometry().workgroup()));
}

fn prepare_k7(
    allocations: &mut ServiceAllocationSessionV1,
    spec: M1PacketDiagnosticSpecV1,
    anchor: u32,
    draft: &[u32; 4],
    content_ordinal_base: u32,
) -> DiagnosticResult<(ServiceFixedDispatchPacketV1, ServiceHostDispatchRangeV1)> {
    let anchor_range = allocate_device_input(
        allocations,
        anchor.to_le_bytes().to_vec().into_boxed_slice(),
        content_ordinal_base,
        spec.buffers()[0],
    )?;
    let draft_range = allocate_device_input(
        allocations,
        u32_bytes(draft),
        content_ordinal_base + 1,
        spec.buffers()[1],
    )?;
    let output_contract = spec.buffers()[2];
    let output = vec![0_u8; 5 * size_of::<u32>()].into_boxed_slice();
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
        ServiceFixedDispatchPacketV1::new(
            program.program_index(),
            geometry,
            dynamic_group_bytes,
            kernarg,
            bindings,
        ),
        output_range,
    ))
}

fn prepare_k7_pair(
    allocations: &mut ServiceAllocationSessionV1,
    first_spec: M1PacketDiagnosticSpecV1,
    second_spec: M1PacketDiagnosticSpecV1,
    ordering: AqlDispatchOrderingV1,
) -> DiagnosticResult<(
    [ServiceFixedDispatchPacketV1; 2],
    [ServiceHostDispatchRangeV1; 2],
)> {
    let first_anchor = allocate_device_input(
        allocations,
        FIRST_ANCHOR.to_le_bytes().to_vec().into_boxed_slice(),
        0,
        first_spec.buffers()[0],
    )?;
    let first_draft = allocate_device_input(
        allocations,
        u32_bytes(&FIRST_DRAFT),
        1,
        first_spec.buffers()[1],
    )?;
    let second_anchor = allocate_device_input(
        allocations,
        SECOND_ANCHOR.to_le_bytes().to_vec().into_boxed_slice(),
        2,
        second_spec.buffers()[0],
    )?;
    let second_draft = allocate_device_input(
        allocations,
        u32_bytes(&SECOND_DRAFT),
        3,
        second_spec.buffers()[1],
    )?;

    // Host range ordinals depend on the final device-allocation count.
    let first_output = allocate_host_output(allocations, first_spec.buffers()[2])?;
    let second_output = allocate_host_output(allocations, second_spec.buffers()[2])?;
    let first_packet = build_k7_packet(
        first_spec,
        first_anchor,
        first_draft,
        first_output,
        ordering,
    );
    let second_packet = build_k7_packet(
        second_spec,
        second_anchor,
        second_draft,
        second_output,
        ordering,
    );
    Ok(([first_packet, second_packet], [first_output, second_output]))
}

fn allocate_host_output(
    allocations: &mut ServiceAllocationSessionV1,
    contract: M1PacketDiagnosticBufferV1,
) -> DiagnosticResult<ServiceHostDispatchRangeV1> {
    let output = vec![0_u8; 5 * size_of::<u32>()].into_boxed_slice();
    require_extent(output.len(), contract.byte_len(), "K7 output")?;
    let key = allocations
        .allocate_initialized_host_visible::<HostDownloadRoleV1>(output)
        .map_err(|error| format!("cannot allocate K7 host-visible output: {error}"))?;
    allocations
        .range(key, 0, contract.byte_len(), contract.alignment())
        .and_then(|range| allocations.host_dispatch_range(range))
        .map_err(|error| format!("cannot bind K7 host-visible output: {error}"))
}

fn build_k7_packet(
    spec: M1PacketDiagnosticSpecV1,
    anchor: fe2o3_service_host::ServiceDeviceDispatchRangeV1,
    draft: fe2o3_service_host::ServiceDeviceDispatchRangeV1,
    output: ServiceHostDispatchRangeV1,
    ordering: AqlDispatchOrderingV1,
) -> ServiceFixedDispatchPacketV1 {
    let (program, geometry, dynamic_group_bytes, kernarg, buffers) = spec.into_packet_parts();
    let bindings = vec![
        ServiceFixedDispatchBufferV1::new(buffers[0].explicit_argument_index(), anchor),
        ServiceFixedDispatchBufferV1::new(buffers[1].explicit_argument_index(), draft),
        ServiceFixedDispatchBufferV1::new_host_visible(
            buffers[2].explicit_argument_index(),
            output,
        ),
    ]
    .into_boxed_slice();
    match ordering {
        AqlDispatchOrderingV1::WaitForPrior => ServiceFixedDispatchPacketV1::new(
            program.program_index(),
            geometry,
            dynamic_group_bytes,
            kernarg,
            bindings,
        ),
        AqlDispatchOrderingV1::Independent => ServiceFixedDispatchPacketV1::new_independent(
            program.program_index(),
            geometry,
            dynamic_group_bytes,
            kernarg,
            bindings,
        ),
    }
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
        .allocate_initialized_device_local::<DeviceInputRoleV1>(bytes, 4_096, descriptor)
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

fn publish_and_complete<const N: usize>(
    allocations: ServiceAllocationSessionV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    packets: [ServiceFixedDispatchPacketV1; N],
    report: &mut impl FnMut(String),
) -> DiagnosticResult<fe2o3_service_host::ServiceCompletedQueueSessionV1<N>> {
    let batch = ServiceFixedBatchV1::new(catalog.into_programs(), packets);
    report("phase=create-queue".to_owned());
    let queue = match ServiceQueueSessionV1::<N>::create(
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
        .map_err(queue_operation_error("cannot submit diagnostic batch"))?;
    wait_with_pacing(published, report)
}

fn wait_with_pacing<const N: usize>(
    mut published: ServicePublishedQueueSessionV1<N>,
    report: &mut impl FnMut(String),
) -> DiagnosticResult<fe2o3_service_host::ServiceCompletedQueueSessionV1<N>> {
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

fn verify_output<const N: usize>(
    recycled: &mut fe2o3_service_host::ServiceRecycledQueueSessionV1<N>,
    output: ServiceHostDispatchRangeV1,
    expected: &[u32; 5],
    description: &str,
) -> DiagnosticResult<()> {
    let request = recycled.completed_read_request(output);
    let readback = recycled
        .read_completed(request)
        .map_err(|error| format!("cannot read completed {description} output: {error}"))?;
    let actual = decode_u32s(readback.bytes())?;
    if actual.as_slice() != expected {
        return Err(format!(
            "{description} output mismatch: expected {expected:?}, got {actual:?}"
        ));
    }
    Ok(())
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

fn destroy_recycled_after_error<T, const N: usize>(
    recycled: fe2o3_service_host::ServiceRecycledQueueSessionV1<N>,
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
    use fe2o3_aql::{AqlDispatchGeometryV1, AqlDispatchOrderingV1};

    use super::*;

    fn inert_ordered_packet() -> ServiceFixedDispatchPacketV1 {
        ServiceFixedDispatchPacketV1::new(
            0,
            AqlDispatchGeometryV1::new([64, 1, 1], [64, 1, 1]).unwrap(),
            0,
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        )
    }

    #[test]
    fn ordered_pair_fixture_is_distinct_and_uses_two_packet_batch() {
        assert_eq!(FIRST_EXPECTED, [FIRST_ANCHOR, 11, 12, 13, 14]);
        assert_eq!(SECOND_EXPECTED, [SECOND_ANCHOR, 21, 22, 23, 24]);
        assert_ne!(FIRST_EXPECTED, SECOND_EXPECTED);

        let first = inert_ordered_packet();
        assert_eq!(first.ordering(), AqlDispatchOrderingV1::WaitForPrior);
        let second = inert_ordered_packet();
        assert_eq!(second.ordering(), AqlDispatchOrderingV1::WaitForPrior);
        let batch = ServiceFixedBatchV1::new(Vec::new(), [first, second]);
        assert_eq!(batch.packet_count(), 2);
    }
}
