//! Transport identities for the opt-in native whole-program validation scope.

use super::{Input, Program, TpResult, dep, dependency_wire, prepared_forward_wire, protocol};
use std::io::{self, Read};

#[path = "../../../tp-peer-engineering-worker-v4/src/prepared_program_wire.rs"]
#[allow(dead_code)]
mod native;

#[path = "../../../tp-peer-engineering-worker-v4/src/prepared_graph_wire.rs"]
#[allow(dead_code)]
pub(super) mod graph;

#[path = "tp_prepared_graph.rs"]
mod graph_transport;
pub(super) use graph_transport::validate_geometry_graph_receipt;
#[cfg(test)]
pub(super) use graph_transport::validate_graph_receipt;

pub(super) use native::NativeProgramCompletion;

#[cfg(test)]
pub(super) fn graph_ready_fixture(
    ids: [u64; 2],
    pid: u32,
    execution_mode: graph::ExecutionMode,
) -> graph::Response {
    graph_ready_fixture_with_profile(ids, pid, execution_mode, graph::KernelProfile::Baseline)
}

#[cfg(test)]
pub(super) fn graph_ready_fixture_with_profile(
    ids: [u64; 2],
    pid: u32,
    execution_mode: graph::ExecutionMode,
    kernel_profile: graph::KernelProfile,
) -> graph::Response {
    graph_ready_fixture_with_options(
        ids,
        pid,
        execution_mode,
        kernel_profile,
        graph::MetadataUploadMode::SeparateWrites,
    )
}

#[cfg(test)]
pub(super) fn graph_ready_fixture_with_options(
    ids: [u64; 2],
    pid: u32,
    execution_mode: graph::ExecutionMode,
    kernel_profile: graph::KernelProfile,
    metadata_upload_mode: graph::MetadataUploadMode,
) -> graph::Response {
    graph::Response::Ready {
        protocol: graph::PROTOCOL,
        mode: graph::MODE.into(),
        execution_mode,
        kernel_profile,
        metadata_upload_mode,
        profile: kernel_profile.program_profile().into(),
        unique_ids: ids,
        process_id: pid,
        authority: "none".into(),
        currentness: execution_mode.currentness().into(),
        control_allocation_flags: dep::CONTROL_ALLOCATION_FLAGS,
        native_source_sha256: graph::NATIVE_SOURCE.into(),
        native_program_deadline_ms: graph::NATIVE_PROGRAM_DEADLINE_MS,
        native_queued_deadline_ms: graph::NATIVE_QUEUED_DEADLINE_MS,
    }
}

#[cfg(test)]
pub(super) fn long_graph_ready_fixture(
    ids: [u64; 2],
    pid: u32,
    execution: graph::ExecutionMode,
    profile: graph::KernelProfile,
    metadata: graph::MetadataUploadMode,
) -> graph::LongResponse {
    let mut response = graph_ready_fixture_with_options(ids, pid, execution, profile, metadata);
    let graph::Response::Ready {
        protocol,
        mode,
        profile: program_profile,
        ..
    } = &mut response
    else {
        unreachable!()
    };
    *protocol = graph::LONG_PROTOCOL;
    *mode = graph::LONG_MODE.into();
    *program_profile = graph::GraphGeometry::Long2304.program_profile(profile);
    graph::LongResponse {
        geometry: graph::GraphGeometry::Long2304,
        response,
    }
}

#[cfg(test)]
pub(super) fn ready_fixture(ids: [u64; 2], pid: u32) -> serde_json::Value {
    serde_json::to_value(native::Response::Ready {
        protocol: native::PROTOCOL,
        mode: native::MODE.into(),
        profile: protocol::PROFILE.into(),
        unique_ids: ids,
        process_id: pid,
        authority: "none".into(),
        currentness: native::CURRENTNESS.into(),
        control_allocation_flags: dep::CONTROL_ALLOCATION_FLAGS,
        native_source_sha256: native::NATIVE_SOURCE.into(),
        native_program_deadline_ms: native::NATIVE_PROGRAM_DEADLINE_MS,
    })
    .unwrap()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Interpreter,
    NativeProgram,
    Graph(graph::ExecutionMode),
    GraphOptions(
        graph::ExecutionMode,
        graph::KernelProfile,
        graph::MetadataUploadMode,
    ),
    LongGraph(
        graph::ExecutionMode,
        graph::KernelProfile,
        graph::MetadataUploadMode,
    ),
}

impl Mode {
    pub(crate) fn launch_flag(self) -> &'static str {
        match self {
            Self::Interpreter => "--tp2-prepared-forward-interpreter",
            Self::NativeProgram => native::LAUNCH_FLAG,
            Self::Graph(_) | Self::GraphOptions(_, _, _) | Self::LongGraph(_, _, _) => {
                graph::LAUNCH_FLAG
            }
        }
    }

    pub(crate) fn source(self) -> &'static str {
        match self {
            Self::Interpreter => protocol::NATIVE_SOURCE,
            Self::NativeProgram => native::NATIVE_SOURCE,
            Self::Graph(_) | Self::GraphOptions(_, _, _) | Self::LongGraph(_, _, _) => {
                graph::NATIVE_SOURCE
            }
        }
    }

    pub(crate) fn kernel_profile(self) -> graph::KernelProfile {
        match self {
            Self::GraphOptions(_, profile, _) | Self::LongGraph(_, profile, _) => profile,
            Self::Interpreter | Self::NativeProgram | Self::Graph(_) => {
                graph::KernelProfile::Baseline
            }
        }
    }

    pub(crate) fn metadata_upload_mode(self) -> graph::MetadataUploadMode {
        match self {
            Self::GraphOptions(_, _, metadata) | Self::LongGraph(_, _, metadata) => metadata,
            Self::Interpreter | Self::NativeProgram | Self::Graph(_) => {
                graph::MetadataUploadMode::SeparateWrites
            }
        }
    }

    pub(crate) fn geometry(self) -> graph::GraphGeometry {
        match self {
            Self::LongGraph(_, _, _) => graph::GraphGeometry::Long2304,
            _ => graph::GraphGeometry::Short64,
        }
    }

    pub(crate) fn launch_arguments(self) -> Vec<&'static str> {
        let mut arguments = vec![self.launch_flag()];
        if let Some(policy) = self.graph_execution() {
            arguments.push(policy.argument());
        }
        if let Self::GraphOptions(_, profile, metadata) | Self::LongGraph(_, profile, metadata) =
            self
        {
            arguments.extend([
                graph::KERNEL_PROFILE_FLAG,
                profile.argument(),
                graph::METADATA_UPLOAD_FLAG,
                metadata.argument(),
            ]);
        }
        if self.geometry() == graph::GraphGeometry::Long2304 {
            arguments.extend([graph::GEOMETRY_FLAG, self.geometry().argument()]);
        }
        arguments
    }

    pub(crate) fn graph_execution(self) -> Option<graph::ExecutionMode> {
        match self {
            Self::Graph(mode) | Self::GraphOptions(mode, _, _) | Self::LongGraph(mode, _, _) => {
                Some(mode)
            }
            Self::Interpreter | Self::NativeProgram => None,
        }
    }

    pub(crate) fn graph_policy(self) -> Option<super::GraphPolicy> {
        match self {
            Self::Graph(graph::ExecutionMode::QueuedBaseline)
            | Self::GraphOptions(graph::ExecutionMode::QueuedBaseline, _, _) => {
                Some(super::GraphPolicy::QueuedBaseline)
            }
            Self::Graph(graph::ExecutionMode::TransactionFences)
            | Self::GraphOptions(graph::ExecutionMode::TransactionFences, _, _) => {
                Some(super::GraphPolicy::TransactionFences)
            }
            Self::Graph(graph::ExecutionMode::TransactionFencesAdmissionCache)
            | Self::GraphOptions(graph::ExecutionMode::TransactionFencesAdmissionCache, _, _) => {
                Some(super::GraphPolicy::TransactionFencesAdmissionCache)
            }
            Self::Graph(
                graph::ExecutionMode::TransactionFencesAdmissionCacheScopedObservations,
            )
            | Self::GraphOptions(
                graph::ExecutionMode::TransactionFencesAdmissionCacheScopedObservations,
                _,
                _,
            ) => Some(super::GraphPolicy::TransactionFencesAdmissionCacheScopedObservations),
            Self::Graph(graph::ExecutionMode::ClosedTokenAdmissionCache)
            | Self::GraphOptions(graph::ExecutionMode::ClosedTokenAdmissionCache, _, _) => {
                Some(super::GraphPolicy::ClosedTokenAdmissionCache)
            }
            Self::Graph(graph::ExecutionMode::FiniteRequestAdmissionCache)
            | Self::GraphOptions(graph::ExecutionMode::FiniteRequestAdmissionCache, _, _) => {
                Some(super::GraphPolicy::FiniteRequestAdmissionCache)
            }
            Self::Interpreter | Self::NativeProgram => None,
            Self::LongGraph(mode, _, _) => Self::Graph(mode).graph_policy(),
        }
    }
}

pub(super) enum Reply {
    Interpreter(protocol::Response),
    NativeProgram(native::Response),
    Graph(graph::Response),
    LongGraph(graph::LongResponse),
}

impl Reply {
    pub(super) fn valid_ready(&self, mode: Mode, ids: [u64; 2], pid: u32) -> bool {
        match (mode, self) {
            (Mode::LongGraph(expected, profile, metadata), Self::LongGraph(response)) => {
                response.geometry == graph::GraphGeometry::Long2304
                    && graph_transport::valid_ready_geometry(
                        &response.response,
                        expected,
                        profile,
                        metadata,
                        graph::GraphGeometry::Long2304,
                        ids,
                        pid,
                    )
            }
            (Mode::Graph(expected) | Mode::GraphOptions(expected, _, _), Self::Graph(response)) => {
                graph_transport::valid_ready(
                    response,
                    expected,
                    mode.kernel_profile(),
                    mode.metadata_upload_mode(),
                    ids,
                    pid,
                )
            }
            (Mode::Interpreter, Self::Interpreter(response)) => {
                super::valid_ready(response, ids, pid)
            }
            (
                Mode::NativeProgram,
                Self::NativeProgram(native::Response::Ready {
                    protocol: version,
                    mode,
                    profile,
                    unique_ids,
                    process_id,
                    authority,
                    currentness,
                    control_allocation_flags,
                    native_source_sha256,
                    native_program_deadline_ms,
                }),
            ) => {
                *version == native::PROTOCOL
                    && mode == native::MODE
                    && profile == protocol::PROFILE
                    && *unique_ids == ids
                    && *process_id == pid
                    && authority == "none"
                    && currentness == native::CURRENTNESS
                    && *control_allocation_flags == dep::CONTROL_ALLOCATION_FLAGS
                    && native_source_sha256 == native::NATIVE_SOURCE
                    && *native_program_deadline_ms == native::NATIVE_PROGRAM_DEADLINE_MS
            }
            _ => false,
        }
    }

    // Common response fields retain their actual wire values. Native completion
    // metadata is returned separately and must be validated before host commit.
    pub(super) fn into_response(
        self,
    ) -> TpResult<(protocol::Response, Option<NativeProgramCompletion>)> {
        let response = match self {
            Self::Interpreter(value) => return Ok((value, None)),
            Self::NativeProgram(value) => value,
            Self::Graph(value) => {
                return graph_transport::common_response(value).map(|response| (response, None));
            }
            Self::LongGraph(value) => {
                if value.geometry != graph::GraphGeometry::Long2304 {
                    return Err("long graph response geometry mismatch".into());
                }
                return graph_transport::common_response(value.response)
                    .map(|response| (response, None));
            }
        };
        let response = match response {
            native::Response::Setup { id, response } => protocol::Response::Setup { id, response },
            native::Response::Registered {
                id,
                plan_sha256,
                catalog_sha256,
                steps,
                kernel_counts,
            } => protocol::Response::Registered {
                id,
                plan_sha256,
                catalog_sha256,
                steps,
                kernel_counts,
            },
            native::Response::Executed {
                receipt,
                native_program,
            } => {
                return Ok((
                    protocol::Response::Executed { receipt },
                    Some(native_program),
                ));
            }
            native::Response::Closed { id } => protocol::Response::Closed { id },
            native::Response::Fatal { id, message } => protocol::Response::Fatal { id, message },
            native::Response::Ready { .. } => return Err("unexpected second scoped Ready".into()),
        };
        Ok((response, None))
    }
}

pub(super) fn read_response(input: &mut impl Read, mode: Mode) -> io::Result<Option<Reply>> {
    match mode {
        Mode::Interpreter => {
            protocol::read_response(input).map(|value| value.map(Reply::Interpreter))
        }
        Mode::NativeProgram => {
            native::read_response(input).map(|value| value.map(Reply::NativeProgram))
        }
        Mode::Graph(_) | Mode::GraphOptions(_, _, _) => {
            graph::read_response(input).map(|value| value.map(Reply::Graph))
        }
        Mode::LongGraph(_, _, _) => {
            graph::read_long_response(input).map(|value| value.map(Reply::LongGraph))
        }
    }
}

pub(super) enum Request {
    Short(protocol::Request),
    Long(graph::LongRequest),
}

impl Request {
    pub(super) fn for_mode(mode: Mode, request: protocol::Request) -> TpResult<Self> {
        if mode.geometry() == graph::GraphGeometry::Short64 {
            return Ok(Self::Short(request));
        }
        let request = match request {
            protocol::Request::Setup {
                id,
                rank,
                peer_readable,
                command,
            } => graph::LongRequestBody::Setup {
                id,
                rank,
                peer_readable,
                command,
            },
            protocol::Request::Register {
                id,
                program_sha256,
                program_bytes,
            } => graph::LongRequestBody::Register {
                id,
                program_sha256,
                program_bytes,
            },
            protocol::Request::Close { id } => graph::LongRequestBody::Close { id },
            protocol::Request::Execute { .. } => {
                return Err("short Execute cannot enter long graph wire".into());
            }
        };
        Ok(Self::Long(graph::LongRequest {
            geometry: graph::GraphGeometry::Long2304,
            request,
        }))
    }
    pub(super) fn geometry(&self) -> graph::GraphGeometry {
        match self {
            Self::Short(_) => graph::GraphGeometry::Short64,
            Self::Long(value) => value.geometry,
        }
    }
    pub(super) fn id(&self) -> u64 {
        match self {
            Self::Short(value) => value.id(),
            Self::Long(value) => value.id(),
        }
    }
    pub(super) fn is_close(&self) -> bool {
        matches!(
            self,
            Self::Short(protocol::Request::Close { .. })
                | Self::Long(graph::LongRequest {
                    request: graph::LongRequestBody::Close { .. },
                    ..
                })
        )
    }
    pub(super) fn payload_bytes(&self) -> io::Result<usize> {
        match self {
            Self::Short(value) => value.payload_bytes(),
            Self::Long(value) => value.payload_bytes(),
        }
    }
    pub(super) fn write(&self, output: &mut impl std::io::Write, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::Short(value) => protocol::write_request(output, value, bytes),
            Self::Long(value) => graph::write_long_request(output, value, bytes),
        }
    }
}

pub(super) fn validate_completion(
    actual: &NativeProgramCompletion,
    program: &Program,
    input: &Input,
    ids: [u64; 2],
) -> TpResult<()> {
    let next = input.epoch.checked_add(1).ok_or("scoped epoch overflow")?;
    let first = [688_u64, 685].map(|count| input.epoch.checked_mul(count));
    let last = [688_u64, 685].map(|count| next.checked_mul(count));
    if actual.program_id != input.generation
        || input.generation != next
        || actual.group_id != program.group_id
        || actual.epoch != input.epoch
        || actual.unique_ids != ids
        || actual.program_api_calls != 1
        || actual.logical_steps != 1013
        || actual.rank_dispatches != 941
        || actual.kernel_counts != [616, 613]
        || actual.barrier_counts != [72; 2]
        || actual.packet_counts != [688, 685]
        || first.iter().any(Option::is_none)
        || last.iter().any(Option::is_none)
        || actual.first_frontiers.map(Some) != first
        || actual.final_frontiers.map(|value| value.map(Some)) != last.map(|value| [value; 2])
    {
        return Err("scoped actual native program identity/count/frontier mismatch".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_context2304_ready_and_launch_are_bound_across_every_closed_axis() {
        for execution in graph::ExecutionMode::PRE_FINITE {
            for profile in graph::KernelProfile::ALL {
                for metadata in graph::MetadataUploadMode::ALL {
                    let selected = Mode::LongGraph(execution, profile, metadata);
                    assert_eq!(
                        selected.launch_arguments(),
                        [
                            graph::LAUNCH_FLAG,
                            execution.argument(),
                            graph::KERNEL_PROFILE_FLAG,
                            profile.argument(),
                            graph::METADATA_UPLOAD_FLAG,
                            metadata.argument(),
                            graph::GEOMETRY_FLAG,
                            "long2304"
                        ]
                    );
                    let ready =
                        long_graph_ready_fixture([11, 22], 30, execution, profile, metadata);
                    if execution.closed_token() && metadata != graph::MetadataUploadMode::Batched {
                        assert!(!Reply::LongGraph(ready).valid_ready(selected, [11, 22], 30));
                        continue;
                    }
                    assert!(Reply::LongGraph(ready.clone()).valid_ready(selected, [11, 22], 30));
                    assert!(!Reply::LongGraph(ready.clone()).valid_ready(
                        Mode::GraphOptions(execution, profile, metadata),
                        [11, 22],
                        30
                    ));
                    assert!(!Reply::Graph(ready.response.clone()).valid_ready(
                        selected,
                        [11, 22],
                        30
                    ));
                    for other in graph::KernelProfile::ALL {
                        assert_eq!(
                            Reply::LongGraph(ready.clone()).valid_ready(
                                Mode::LongGraph(execution, other, metadata),
                                [11, 22],
                                30
                            ),
                            other == profile
                        );
                    }
                    for other in graph::ExecutionMode::PRE_FINITE {
                        assert_eq!(
                            Reply::LongGraph(ready.clone()).valid_ready(
                                Mode::LongGraph(other, profile, metadata),
                                [11, 22],
                                30
                            ),
                            other == execution
                        );
                    }
                    for other in graph::MetadataUploadMode::ALL {
                        assert_eq!(
                            Reply::LongGraph(ready.clone()).valid_ready(
                                Mode::LongGraph(execution, profile, other),
                                [11, 22],
                                30
                            ),
                            other == metadata
                        );
                    }
                    let mut substituted = ready.clone();
                    substituted.geometry = graph::GraphGeometry::Short64;
                    assert!(!Reply::LongGraph(substituted).valid_ready(selected, [11, 22], 30));
                    let mut substituted = ready.clone();
                    let graph::Response::Ready {
                        protocol,
                        mode,
                        profile: program,
                        ..
                    } = &mut substituted.response
                    else {
                        panic!()
                    };
                    *protocol = graph::PROTOCOL;
                    *mode = graph::MODE.into();
                    *program = profile.program_profile().into();
                    assert!(!Reply::LongGraph(substituted).valid_ready(selected, [11, 22], 30));
                    let mut bytes = Vec::new();
                    graph::write_long_response(&mut bytes, &ready).unwrap();
                    assert!(
                        read_response(&mut bytes.as_slice(), selected)
                            .unwrap()
                            .is_some()
                    );
                    assert!(read_response(&mut bytes.as_slice(), Mode::Graph(execution)).is_err());
                }
            }
        }
    }

    #[test]
    fn graph_context2304_request_adapter_preserves_short_bytes_and_rejects_short_execute() {
        let short = Mode::Graph(graph::ExecutionMode::TransactionFences);
        let long = Mode::LongGraph(
            graph::ExecutionMode::TransactionFences,
            graph::KernelProfile::Baseline,
            graph::MetadataUploadMode::SeparateWrites,
        );
        let request = protocol::Request::Close { id: 1 };
        let mut expected = Vec::new();
        protocol::write_request(&mut expected, &request, &[]).unwrap();
        let mut actual = Vec::new();
        Request::for_mode(short, request)
            .unwrap()
            .write(&mut actual, &[])
            .unwrap();
        assert_eq!(actual, expected);
        let request = Request::for_mode(long, protocol::Request::Close { id: 1 }).unwrap();
        let mut bytes = Vec::new();
        request.write(&mut bytes, &[]).unwrap();
        assert!(
            graph::read_long_request(&mut bytes.as_slice())
                .unwrap()
                .is_some()
        );
        assert!(protocol::read_request(&mut bytes.as_slice()).is_err());
        assert!(
            Request::for_mode(
                long,
                protocol::Request::Execute {
                    execution: protocol::Execute {
                        id: 1,
                        plan_sha256: [1; 32],
                        generation: 1,
                        epoch: 0,
                        token: 42,
                        position: 0,
                        page_table: [0, u32::MAX, u32::MAX, u32::MAX]
                    }
                }
            )
            .is_err()
        );
    }

    fn ready() -> native::Response {
        native::Response::Ready {
            protocol: native::PROTOCOL,
            mode: native::MODE.into(),
            profile: protocol::PROFILE.into(),
            unique_ids: [11, 22],
            process_id: 30,
            authority: "none".into(),
            currentness: native::CURRENTNESS.into(),
            control_allocation_flags: dep::CONTROL_ALLOCATION_FLAGS,
            native_source_sha256: native::NATIVE_SOURCE.into(),
            native_program_deadline_ms: 30_000,
        }
    }

    #[test]
    fn graph_launch_options_keep_default_and_canonical_argument_order() {
        for execution in graph::ExecutionMode::PRE_FINITE {
            let default = Mode::Graph(execution);
            assert_eq!(default.kernel_profile(), graph::KernelProfile::Baseline);
            assert_eq!(
                default.metadata_upload_mode(),
                graph::MetadataUploadMode::SeparateWrites
            );
            assert_eq!(
                default.launch_arguments(),
                [graph::LAUNCH_FLAG, execution.argument()]
            );
            for profile in graph::KernelProfile::ALL {
                for metadata in graph::MetadataUploadMode::ALL {
                    let selected = Mode::GraphOptions(execution, profile, metadata);
                    assert_eq!(selected.graph_policy(), default.graph_policy());
                    assert_eq!(
                        selected.launch_arguments(),
                        [
                            graph::LAUNCH_FLAG,
                            execution.argument(),
                            graph::KERNEL_PROFILE_FLAG,
                            profile.argument(),
                            graph::METADATA_UPLOAD_FLAG,
                            metadata.argument(),
                        ]
                    );
                }
            }
        }
        for mode in [Mode::Interpreter, Mode::NativeProgram] {
            assert_eq!(mode.launch_arguments(), [mode.launch_flag()]);
        }
    }

    #[test]
    fn graph_ready_binds_metadata_selection_and_requires_both_option_fields() {
        for execution in graph::ExecutionMode::PRE_FINITE {
            for profile in graph::KernelProfile::ALL {
                for metadata in graph::MetadataUploadMode::ALL {
                    let selected = Mode::GraphOptions(execution, profile, metadata);
                    let ready = graph_ready_fixture_with_options(
                        [11, 22],
                        30,
                        execution,
                        profile,
                        metadata,
                    );
                    if execution.closed_token() && metadata != graph::MetadataUploadMode::Batched {
                        assert!(!Reply::Graph(ready).valid_ready(selected, [11, 22], 30));
                        continue;
                    }
                    assert!(Reply::Graph(ready.clone()).valid_ready(selected, [11, 22], 30));
                    for other in graph::MetadataUploadMode::ALL {
                        let actual = graph_ready_fixture_with_options(
                            [11, 22],
                            30,
                            execution,
                            profile,
                            other,
                        );
                        assert_eq!(
                            Reply::Graph(actual).valid_ready(selected, [11, 22], 30),
                            other == metadata
                        );
                    }
                    for other in graph::KernelProfile::ALL {
                        let actual = graph_ready_fixture_with_options(
                            [11, 22],
                            30,
                            execution,
                            other,
                            metadata,
                        );
                        assert_eq!(
                            Reply::Graph(actual).valid_ready(selected, [11, 22], 30),
                            other == profile
                        );
                    }
                    for other in graph::ExecutionMode::PRE_FINITE {
                        let actual = graph_ready_fixture_with_options(
                            [11, 22],
                            30,
                            other,
                            profile,
                            metadata,
                        );
                        assert_eq!(
                            Reply::Graph(actual).valid_ready(selected, [11, 22], 30),
                            other == execution
                        );
                    }
                    for field in ["metadata_upload_mode", "kernel_profile"] {
                        let mut missing = serde_json::to_value(&ready).unwrap();
                        missing.as_object_mut().unwrap().remove(field);
                        assert!(serde_json::from_value::<graph::Response>(missing).is_err());
                    }
                    let mut unknown = serde_json::to_value(&ready).unwrap();
                    unknown["metadata_upload_mode"] = "batch".into();
                    assert!(serde_json::from_value::<graph::Response>(unknown).is_err());
                }
            }
        }
    }

    #[test]
    fn graph_ready_binds_every_kernel_profile_and_observation_mode() {
        for execution in graph::ExecutionMode::PRE_FINITE {
            for expected in graph::KernelProfile::ALL {
                let metadata = if execution.closed_token() {
                    graph::MetadataUploadMode::Batched
                } else {
                    graph::MetadataUploadMode::SeparateWrites
                };
                let selected = Mode::GraphOptions(execution, expected, metadata);
                let ready =
                    graph_ready_fixture_with_options([11, 22], 30, execution, expected, metadata);
                assert!(Reply::Graph(ready.clone()).valid_ready(selected, [11, 22], 30));
                for other in graph::KernelProfile::ALL {
                    if other == expected {
                        continue;
                    }
                    let substituted =
                        graph_ready_fixture_with_options([11, 22], 30, execution, other, metadata);
                    assert!(!Reply::Graph(substituted).valid_ready(selected, [11, 22], 30));
                }
                let mut wrong_program_profile = ready.clone();
                let graph::Response::Ready { profile, .. } = &mut wrong_program_profile else {
                    unreachable!()
                };
                profile.push('x');
                assert!(!Reply::Graph(wrong_program_profile).valid_ready(selected, [11, 22], 30));
                let mut wrong_source = ready.clone();
                let graph::Response::Ready {
                    native_source_sha256,
                    ..
                } = &mut wrong_source
                else {
                    unreachable!()
                };
                *native_source_sha256 = protocol::NATIVE_SOURCE.into();
                assert!(!Reply::Graph(wrong_source).valid_ready(selected, [11, 22], 30));
                assert!(!Reply::Graph(ready.clone()).valid_ready(
                    Mode::NativeProgram,
                    [11, 22],
                    30
                ));
                assert!(!Reply::Graph(ready).valid_ready(Mode::Interpreter, [11, 22], 30));
            }
        }
    }

    #[test]
    fn native_program_ready_checks_scope_source_deadline_and_wire_discriminator() {
        let valid = ready();
        assert!(Reply::NativeProgram(valid.clone()).valid_ready(Mode::NativeProgram, [11, 22], 30));
        assert!(!Reply::NativeProgram(valid.clone()).valid_ready(Mode::Interpreter, [11, 22], 30));
        assert!(!Reply::NativeProgram(valid.clone()).valid_ready(
            Mode::NativeProgram,
            [22, 11],
            30
        ));
        assert!(!Reply::NativeProgram(valid.clone()).valid_ready(
            Mode::NativeProgram,
            [11, 22],
            31
        ));
        for index in 0..8 {
            let mut changed = valid.clone();
            let native::Response::Ready {
                protocol,
                mode,
                profile,
                authority,
                currentness,
                control_allocation_flags,
                native_source_sha256,
                native_program_deadline_ms,
                ..
            } = &mut changed
            else {
                panic!("ready fixture");
            };
            match index {
                0 => *protocol += 1,
                1 => mode.push('x'),
                2 => profile.push('x'),
                3 => authority.push('x'),
                4 => *currentness = "full".into(),
                5 => *control_allocation_flags = 0,
                6 => native_source_sha256.push('x'),
                7 => *native_program_deadline_ms += 1,
                _ => unreachable!(),
            }
            assert!(!Reply::NativeProgram(changed).valid_ready(Mode::NativeProgram, [11, 22], 30));
        }
        let mut bytes = Vec::new();
        native::write_response(&mut bytes, &valid).unwrap();
        assert!(
            read_response(&mut bytes.as_slice(), Mode::NativeProgram)
                .unwrap()
                .is_some()
        );
        assert!(read_response(&mut bytes.as_slice(), Mode::Interpreter).is_err());
        assert!(Reply::NativeProgram(valid).into_response().is_err());
    }
}
