//! Closed graph completion evidence, separate from serial collective receipts.

use super::dependency_wire::Identity;
pub use super::prepared_forward_wire::{GraphGeometry, KernelProfile};
use fe2o3_kfd::engineering_wire::{self as base, ResponseV1};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const MODE: &str = "device-peer-tp2-prepared-queued-graph-v1";
pub const LAUNCH_FLAG: &str = "--tp2-prepared-queued-graph";
// CPU-qualified native overlay; GPU model qualification remains separate.
pub const NATIVE_SOURCE: &str = "721420a02d222b4e45c7257dfbf1dfbaecc0ca7134c3073a306c01ace901c592";
pub const NATIVE_PROGRAM_DEADLINE_MS: u32 = 30_000;
pub const NATIVE_QUEUED_DEADLINE_MS: u32 = 2_000;
pub const PACKET_COUNTS: [u32; 2] = [759, 757];
pub const BARRIER_COUNTS: [u32; 2] = [143, 144];
pub const METADATA_UPLOAD_FLAG: &str = "--metadata-uploads";
pub const KERNEL_PROFILE_FLAG: &str = "--kernel-profile";
pub const GEOMETRY_FLAG: &str = "--graph-geometry";
pub const LONG_PROTOCOL: u32 = 2;
pub const LONG_MODE: &str = "device-peer-tp2-prepared-queued-graph-context2304-v2";

/// A single fresh request. Its lifetime starts at native begin, never at each token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteBudget {
    pub forwards: u32,
    pub lifetime_ms: u32,
}

impl FiniteBudget {
    pub const fn for_geometry(geometry: GraphGeometry) -> Self {
        Self {
            forwards: match geometry {
                GraphGeometry::Short64 => 36,
                GraphGeometry::Long2304 => 2303,
            },
            lifetime_ms: 1_800_000,
        }
    }

    pub fn validate(self, geometry: GraphGeometry) -> io::Result<()> {
        if self == Self::for_geometry(geometry) {
            Ok(())
        } else {
            Err(io::Error::other("finite request exact budget/lifetime"))
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteProgram {
    pub geometry: GraphGeometry,
    pub budget: FiniteBudget,
    pub program: super::prepared_forward_wire::Program,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteCompleted {
    pub group_id: u64,
    pub forwards: u32,
    pub final_labels: [u64; 3],
    pub next_packets: [u64; 2],
    pub full_boundaries: u32,
}

/// Distinct envelopes prevent a long owner from accepting the short fixed-table wire.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LongProgram {
    pub geometry: GraphGeometry,
    pub program: super::prepared_forward_wire::Program,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LongExecute {
    pub id: u64,
    pub plan_sha256: [u8; 32],
    pub generation: u64,
    pub epoch: u64,
    pub token: u32,
    pub position: u32,
    pub page_table: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "prepared_long_graph_op",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LongRequestBody {
    Setup {
        id: u64,
        rank: u32,
        peer_readable: bool,
        command: base::CommandV1,
    },
    Register {
        id: u64,
        program_sha256: [u8; 32],
        program_bytes: u32,
    },
    Execute {
        execution: LongExecute,
    },
    Close {
        id: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LongRequest {
    pub geometry: GraphGeometry,
    pub request: LongRequestBody,
}

impl LongRequest {
    pub fn id(&self) -> u64 {
        match &self.request {
            LongRequestBody::Setup { id, .. }
            | LongRequestBody::Register { id, .. }
            | LongRequestBody::Close { id } => *id,
            LongRequestBody::Execute { execution } => execution.id,
        }
    }

    pub fn payload_bytes(&self) -> io::Result<usize> {
        use super::prepared_forward_wire::Request;
        if self.geometry != GraphGeometry::Long2304 || self.id() == 0 {
            return Err(io::Error::other("long graph request geometry or ID"));
        }
        // Reuse the unchanged setup/program payload allowlist, not a broader command API.
        match &self.request {
            LongRequestBody::Setup {
                id,
                rank,
                peer_readable,
                command,
            } => Request::Setup {
                id: *id,
                rank: *rank,
                peer_readable: *peer_readable,
                command: command.clone(),
            }
            .payload_bytes(),
            LongRequestBody::Register {
                id,
                program_sha256,
                program_bytes,
            } => Request::Register {
                id: *id,
                program_sha256: *program_sha256,
                program_bytes: *program_bytes,
            }
            .payload_bytes(),
            LongRequestBody::Execute { execution } if execution.page_table.len() == 144 => Ok(512),
            LongRequestBody::Execute { .. } => {
                Err(io::Error::other("long graph exact page table length"))
            }
            LongRequestBody::Close { .. } => Ok(0),
        }
    }
}

pub fn write_long_request(
    output: &mut impl Write,
    request: &LongRequest,
    payload: &[u8],
) -> io::Result<()> {
    if request.payload_bytes()? != payload.len() {
        return Err(io::Error::other("long graph payload extent"));
    }
    base::write_header_v1(output, request)?;
    output.write_all(payload)?;
    output.flush()
}

pub fn read_long_request(input: &mut impl Read) -> io::Result<Option<(LongRequest, Vec<u8>)>> {
    let Some(request): Option<LongRequest> = base::read_header_v1(input)? else {
        return Ok(None);
    };
    let mut payload = vec![0; request.payload_bytes()?];
    input.read_exact(&mut payload)?;
    Ok(Some((request, payload)))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LongResponse {
    pub geometry: GraphGeometry,
    pub response: Response,
}

pub fn write_long_response(output: &mut impl Write, response: &LongResponse) -> io::Result<()> {
    if response.geometry != GraphGeometry::Long2304 {
        return Err(io::Error::other("long graph response geometry"));
    }
    base::write_header_v1(output, response)?;
    output.flush()
}

pub fn read_long_response(input: &mut impl Read) -> io::Result<Option<LongResponse>> {
    let response: Option<LongResponse> = base::read_header_v1(input)?;
    if response
        .as_ref()
        .is_some_and(|value| value.geometry != GraphGeometry::Long2304)
    {
        return Err(io::Error::other("long graph response geometry"));
    }
    Ok(response)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetadataUploadMode {
    #[default]
    SeparateWrites,
    Batched,
}

impl MetadataUploadMode {
    pub const ALL: [Self; 2] = [Self::SeparateWrites, Self::Batched];

    pub const fn argument(self) -> &'static str {
        match self {
            Self::SeparateWrites => "separate-writes",
            Self::Batched => "batched",
        }
    }

    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "separate-writes" => Ok(Self::SeparateWrites),
            "batched" => Ok(Self::Batched),
            _ => Err(io::Error::other("unknown graph metadata upload mode")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionMode {
    QueuedBaseline,
    TransactionFences,
    TransactionFencesAdmissionCache,
    TransactionFencesAdmissionCacheScopedObservations,
    ClosedTokenAdmissionCache,
    FiniteRequestAdmissionCache,
}

impl ExecutionMode {
    #[cfg(test)]
    pub const PRE_FINITE: [Self; 5] = [
        Self::QueuedBaseline,
        Self::TransactionFences,
        Self::TransactionFencesAdmissionCache,
        Self::TransactionFencesAdmissionCacheScopedObservations,
        Self::ClosedTokenAdmissionCache,
    ];
    #[cfg(test)]
    pub const LEGACY: [Self; 4] = [
        Self::QueuedBaseline,
        Self::TransactionFences,
        Self::TransactionFencesAdmissionCache,
        Self::TransactionFencesAdmissionCacheScopedObservations,
    ];

    pub const ALL: [Self; 6] = [
        Self::QueuedBaseline,
        Self::TransactionFences,
        Self::TransactionFencesAdmissionCache,
        Self::TransactionFencesAdmissionCacheScopedObservations,
        Self::ClosedTokenAdmissionCache,
        Self::FiniteRequestAdmissionCache,
    ];

    pub const fn closed_token(self) -> bool {
        matches!(self, Self::ClosedTokenAdmissionCache)
    }

    pub const fn finite_request(self) -> bool {
        matches!(self, Self::FiniteRequestAdmissionCache)
    }

    pub const fn decode_token(self) -> bool {
        self.closed_token() || self.finite_request()
    }

    pub const fn admission_cache(self) -> bool {
        matches!(
            self,
            Self::TransactionFencesAdmissionCache
                | Self::TransactionFencesAdmissionCacheScopedObservations
                | Self::ClosedTokenAdmissionCache
                | Self::FiniteRequestAdmissionCache
        )
    }

    pub const fn scoped_operation_observations(self) -> bool {
        matches!(
            self,
            Self::TransactionFencesAdmissionCacheScopedObservations
        )
    }

    pub const fn argument(self) -> &'static str {
        match self {
            Self::QueuedBaseline => "queued-baseline",
            Self::TransactionFences => "transaction-fences",
            Self::TransactionFencesAdmissionCache => "transaction-fences-admission-cache",
            Self::TransactionFencesAdmissionCacheScopedObservations => {
                "transaction-fences-admission-cache-scoped-observations"
            }
            Self::ClosedTokenAdmissionCache => "closed-token-admission-cache",
            Self::FiniteRequestAdmissionCache => "finite-request-admission-cache",
        }
    }

    pub const fn currentness(self) -> &'static str {
        match self {
            Self::QueuedBaseline => "queued-baseline-per-packet-validation",
            Self::TransactionFences => "transaction-boundaries-paced-reset-observation",
            Self::TransactionFencesAdmissionCache => {
                "transaction-boundaries-paced-reset-observation-immutable-admission-cache"
            }
            Self::TransactionFencesAdmissionCacheScopedObservations => {
                "scoped-operation-v1-full-entry-exit-operational-inner-immutable-admission-cache"
            }
            Self::ClosedTokenAdmissionCache => {
                "closed-token-v1-full-entry-exit-operational-inner-immutable-admission-cache"
            }
            Self::FiniteRequestAdmissionCache => {
                "finite-request-v1-full-begin-completion-operational-inner-provisional"
            }
        }
    }

    pub fn parse(value: &str) -> io::Result<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.argument() == value)
            .ok_or_else(|| io::Error::other("unknown queued graph execution mode"))
    }
}

/// Actual API counters only. The baseline method does not return these counters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "execution_mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PolicyEvidence {
    FiniteRequestAdmissionCache {
        full_boundaries: u32,
        graph_operational_boundaries: u32,
        token_boundaries: u32,
        reset_only_rounds: u32,
        loop_checks: u64,
        token_ordinal: u64,
        provisional: bool,
        completed: Option<FiniteCompleted>,
    },
    QueuedBaseline {},
    TransactionFences {
        full_boundaries: u32,
        operational_boundaries: u32,
        reset_only_rounds: u32,
        loop_checks: u64,
    },
    TransactionFencesAdmissionCache {
        full_boundaries: u32,
        operational_boundaries: u32,
        reset_only_rounds: u32,
        loop_checks: u64,
    },
    TransactionFencesAdmissionCacheScopedObservations {
        full_boundaries: u32,
        operational_boundaries: u32,
        reset_only_rounds: u32,
        loop_checks: u64,
    },
    ClosedTokenAdmissionCache {
        full_boundaries: u32,
        graph_operational_boundaries: u32,
        token_boundaries: u32,
        reset_only_rounds: u32,
        loop_checks: u64,
    },
}

impl PolicyEvidence {
    pub fn validate_for(&self, mode: ExecutionMode) -> io::Result<()> {
        self.validate_for_profile(mode, KernelProfile::Baseline)
    }

    pub fn validate_for_profile(
        &self,
        mode: ExecutionMode,
        profile: KernelProfile,
    ) -> io::Result<()> {
        let (minimum, scan_stride) = if profile.split_attention() {
            (9_543, 1_592)
        } else {
            (9_111, 1_520)
        };
        let valid = match (mode, self) {
            (
                ExecutionMode::FiniteRequestAdmissionCache,
                Self::FiniteRequestAdmissionCache {
                    full_boundaries,
                    graph_operational_boundaries,
                    token_boundaries,
                    reset_only_rounds,
                    loop_checks,
                    token_ordinal,
                    provisional,
                    completed,
                },
            ) => {
                *token_ordinal > 0
                    && *full_boundaries == u32::from(!provisional)
                    && completed.is_some() != *provisional
                    && *graph_operational_boundaries == 9
                    && *token_boundaries == 4
                    && loop_checks
                        .checked_sub(minimum)
                        .is_some_and(|extra| extra.is_multiple_of(scan_stride))
                    && u64::from(*reset_only_rounds) <= *loop_checks
            }
            (ExecutionMode::QueuedBaseline, Self::QueuedBaseline {}) => true,
            (
                ExecutionMode::ClosedTokenAdmissionCache,
                Self::ClosedTokenAdmissionCache {
                    full_boundaries,
                    graph_operational_boundaries,
                    token_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            ) => {
                *full_boundaries == 2
                    && *graph_operational_boundaries == 9
                    && *token_boundaries == 4
                    && loop_checks
                        .checked_sub(minimum)
                        .is_some_and(|extra| extra.is_multiple_of(scan_stride))
                    && u64::from(*reset_only_rounds) <= *loop_checks
            }
            (
                ExecutionMode::TransactionFences,
                Self::TransactionFences {
                    full_boundaries,
                    operational_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            )
            | (
                ExecutionMode::TransactionFencesAdmissionCache,
                Self::TransactionFencesAdmissionCache {
                    full_boundaries,
                    operational_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            )
            | (
                ExecutionMode::TransactionFencesAdmissionCacheScopedObservations,
                Self::TransactionFencesAdmissionCacheScopedObservations {
                    full_boundaries,
                    operational_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            ) => {
                // Four pre-drain scans, two write advances, nine boundaries,
                // one acquisition per slot, then N+4 checks per drain scan.
                // N is selected by the owner-held profile, never by the receipt.
                *full_boundaries == 2
                    && *operational_boundaries == 9
                    && loop_checks
                        .checked_sub(minimum)
                        .is_some_and(|extra| extra.is_multiple_of(scan_stride))
                    && u64::from(*reset_only_rounds) <= *loop_checks
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(io::Error::other(
                "queued graph actual policy evidence mismatch",
            ))
        }
    }
}

#[cfg(test)]
mod scoped_observation_tests {
    use super::*;

    #[test]
    fn prior_mode_and_policy_json_bytes_are_unchanged() {
        for (mode, expected) in [
            (ExecutionMode::QueuedBaseline, "\"queued-baseline\""),
            (ExecutionMode::TransactionFences, "\"transaction-fences\""),
            (
                ExecutionMode::TransactionFencesAdmissionCache,
                "\"transaction-fences-admission-cache\"",
            ),
        ] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), expected);
            assert!(!mode.scoped_operation_observations());
        }
        assert_eq!(
            serde_json::to_string(&PolicyEvidence::QueuedBaseline {}).unwrap(),
            "{\"execution_mode\":\"queued-baseline\"}"
        );
        for (evidence, expected) in [
            (
                PolicyEvidence::TransactionFences {
                    full_boundaries: 2,
                    operational_boundaries: 9,
                    reset_only_rounds: 1,
                    loop_checks: 9_111,
                },
                "{\"execution_mode\":\"transaction-fences\",\"full_boundaries\":2,\"operational_boundaries\":9,\"reset_only_rounds\":1,\"loop_checks\":9111}",
            ),
            (
                PolicyEvidence::TransactionFencesAdmissionCache {
                    full_boundaries: 2,
                    operational_boundaries: 9,
                    reset_only_rounds: 1,
                    loop_checks: 9_111,
                },
                "{\"execution_mode\":\"transaction-fences-admission-cache\",\"full_boundaries\":2,\"operational_boundaries\":9,\"reset_only_rounds\":1,\"loop_checks\":9111}",
            ),
        ] {
            assert_eq!(serde_json::to_string(&evidence).unwrap(), expected);
        }
    }

    #[test]
    fn scoped_mode_and_evidence_are_explicit_and_not_cached_mode_aliases() {
        let selected = ExecutionMode::TransactionFencesAdmissionCacheScopedObservations;
        let evidence = PolicyEvidence::TransactionFencesAdmissionCacheScopedObservations {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: 9_111,
        };
        for mode in ExecutionMode::PRE_FINITE {
            assert_eq!(mode.scoped_operation_observations(), mode == selected);
            assert_eq!(evidence.validate_for(mode).is_ok(), mode == selected);
            assert_eq!(ExecutionMode::parse(mode.argument()).unwrap(), mode);
            assert_eq!(
                serde_json::from_slice::<ExecutionMode>(&serde_json::to_vec(&mode).unwrap())
                    .unwrap(),
                mode
            );
            if mode != selected {
                assert_ne!(mode.currentness(), selected.currentness());
            }
        }
        assert!(selected.admission_cache());
        assert_eq!(
            selected.currentness(),
            "scoped-operation-v1-full-entry-exit-operational-inner-immutable-admission-cache"
        );
        let bytes = serde_json::to_vec(&evidence).unwrap();
        let roundtrip: PolicyEvidence = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(roundtrip, evidence);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["execution_mode"], selected.argument());
        for field in ["full_boundaries", "operational_boundaries", "loop_checks"] {
            let mut invalid = value.clone();
            invalid[field] = 0.into();
            let parsed: PolicyEvidence = serde_json::from_value(invalid).unwrap();
            assert!(parsed.validate_for(selected).is_err());
        }
        let mut invalid = value;
        invalid["shared_full_currentness"] = true.into();
        assert!(serde_json::from_value::<PolicyEvidence>(invalid).is_err());
    }
}

/// Final observed frontiers and each actual acquired completion slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankCompletion {
    pub unique_id: u64,
    pub queue_epoch: u64,
    pub first_packet: u64,
    pub next_packet: u64,
    pub final_write: u64,
    pub final_read: u64,
    pub completion_values: Vec<i64>,
}

/// Source-order packet bindings, never an independently drained collective.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectiveBinding {
    pub identity: Identity,
    pub before_producers: Option<[u64; 2]>,
    pub producers: [u64; 2],
    pub after_producers: [u64; 2],
    pub consumers: [u64; 2],
}

/// Field-for-field copy of the actual native queued graph receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeGraphCompletion {
    pub program_id: u64,
    pub group_id: u64,
    pub epoch: u64,
    pub unique_ids: [u64; 2],
    pub graph_api_calls: u32,
    pub logical_steps: u32,
    pub rank_dispatches: u32,
    pub kernel_counts: [u32; 2],
    pub barrier_counts: [u32; 2],
    pub packet_counts: [u32; 2],
    pub ranks: [RankCompletion; 2],
    pub embedding_copy_packets: [u64; 3],
    pub collectives: Vec<CollectiveBinding>,
}

pub fn validate_finite_completion(
    policy: &PolicyEvidence,
    graph: &NativeGraphCompletion,
    budget: FiniteBudget,
) -> io::Result<()> {
    let PolicyEvidence::FiniteRequestAdmissionCache {
        token_ordinal,
        provisional,
        completed,
        ..
    } = policy
    else {
        return Err(io::Error::other("finite request policy substitution"));
    };
    let final_token = *token_ordinal == u64::from(budget.forwards);
    let last_collective = token_ordinal
        .checked_mul(72)
        .ok_or_else(|| io::Error::other("finite collective overflow"))?;
    let next_generation = last_collective
        .checked_add(1)
        .ok_or_else(|| io::Error::other("finite generation overflow"))?;
    if *token_ordinal == 0
        || *token_ordinal > u64::from(budget.forwards)
        || *token_ordinal != graph.program_id
        || graph.epoch.checked_add(1) != Some(*token_ordinal)
        || *provisional == final_token
        || completed.is_some() != final_token
    {
        return Err(io::Error::other(
            "finite ordinal/provisional/completion mismatch",
        ));
    }
    if let Some(done) = completed
        && (done.group_id != graph.group_id
            || done.forwards != budget.forwards
            || done.full_boundaries != 2
            || done.final_labels != [graph.program_id, next_generation, last_collective]
            || done.next_packets != graph.ranks.each_ref().map(|rank| rank.next_packet))
    {
        return Err(io::Error::other("finite exact closing receipt mismatch"));
    }
    Ok(())
}

/// Transport identity, actual final argmax readback and actual native completion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphExecutionReceipt {
    pub id: u64,
    pub plan_sha256: [u8; 32],
    pub generation: u64,
    pub epoch: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub graph: NativeGraphCompletion,
    pub policy: PolicyEvidence,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "prepared_graph_op",
    rename_all = "snake_case",
    deny_unknown_fields
)]
// Keep bounded synchronous responses inline without a per-token allocation.
#[allow(clippy::large_enum_variant)]
pub enum Response {
    RegisteredFinite {
        id: u64,
        plan_sha256: [u8; 32],
        catalog_sha256: [u8; 32],
        group_id: u64,
        geometry: GraphGeometry,
        budget: FiniteBudget,
        full_boundaries: u32,
        steps: u32,
        kernel_counts: [u32; 2],
    },
    Ready {
        protocol: u32,
        mode: String,
        execution_mode: ExecutionMode,
        metadata_upload_mode: MetadataUploadMode,
        kernel_profile: KernelProfile,
        profile: String,
        unique_ids: [u64; 2],
        process_id: u32,
        authority: String,
        currentness: String,
        control_allocation_flags: u32,
        native_source_sha256: String,
        native_program_deadline_ms: u32,
        native_queued_deadline_ms: u32,
    },
    Setup {
        id: u64,
        response: ResponseV1,
    },
    Registered {
        id: u64,
        plan_sha256: [u8; 32],
        catalog_sha256: [u8; 32],
        steps: u32,
        kernel_counts: [u32; 2],
    },
    Executed {
        receipt: GraphExecutionReceipt,
    },
    Closed {
        id: u64,
    },
    Fatal {
        id: u64,
        message: String,
    },
}

pub fn write_response(output: &mut impl Write, response: &Response) -> io::Result<()> {
    base::write_header_v1(output, response)?;
    output.flush()
}

pub fn read_response(input: &mut impl Read) -> io::Result<Option<Response>> {
    base::read_header_v1(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_wire_requires_both_immutable_selections_under_all_modes() {
        for execution_mode in ExecutionMode::PRE_FINITE {
            for kernel_profile in KernelProfile::ALL {
                for metadata_upload_mode in MetadataUploadMode::ALL {
                    let value = serde_json::to_value(Response::Ready {
                        protocol: PROTOCOL,
                        mode: MODE.into(),
                        execution_mode,
                        metadata_upload_mode,
                        kernel_profile,
                        profile: kernel_profile.program_profile().into(),
                        unique_ids: [11, 22],
                        process_id: 30,
                        authority: "none".into(),
                        currentness: execution_mode.currentness().into(),
                        control_allocation_flags:
                            super::super::dependency_wire::CONTROL_ALLOCATION_FLAGS,
                        native_source_sha256: NATIVE_SOURCE.into(),
                        native_program_deadline_ms: NATIVE_PROGRAM_DEADLINE_MS,
                        native_queued_deadline_ms: NATIVE_QUEUED_DEADLINE_MS,
                    })
                    .unwrap();
                    assert!(serde_json::from_value::<Response>(value.clone()).is_ok());
                    for field in ["kernel_profile", "metadata_upload_mode"] {
                        let mut missing = value.clone();
                        missing.as_object_mut().unwrap().remove(field);
                        assert!(serde_json::from_value::<Response>(missing).is_err());
                        let mut unknown = value.clone();
                        unknown[field] = "automatic".into();
                        assert!(serde_json::from_value::<Response>(unknown).is_err());
                    }
                    let mut extra = value;
                    extra["ignored_profile"] = kernel_profile.argument().into();
                    assert!(serde_json::from_value::<Response>(extra).is_err());
                }
            }
        }
    }

    #[test]
    fn cached_mode_is_closed_and_does_not_alias_uncached_transaction_evidence() {
        let evidence = PolicyEvidence::TransactionFencesAdmissionCache {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: 9_111,
        };
        for mode in ExecutionMode::PRE_FINITE {
            assert_eq!(ExecutionMode::parse(mode.argument()).unwrap(), mode);
            assert_eq!(
                evidence.validate_for(mode).is_ok(),
                mode == ExecutionMode::TransactionFencesAdmissionCache
            );
        }
        let raw = serde_json::to_value(&evidence).unwrap();
        assert_eq!(raw["execution_mode"], "transaction-fences-admission-cache");
        assert_eq!(
            serde_json::from_value::<PolicyEvidence>(raw.clone()).unwrap(),
            evidence
        );
        let mut unknown = raw.clone();
        unknown["cache_currentness"] = true.into();
        assert!(serde_json::from_value::<PolicyEvidence>(unknown).is_err());
        for field in ["full_boundaries", "operational_boundaries", "loop_checks"] {
            let mut invalid = raw.clone();
            invalid[field] = 0.into();
            let decoded = serde_json::from_value::<PolicyEvidence>(invalid).unwrap();
            assert!(
                decoded
                    .validate_for(ExecutionMode::TransactionFencesAdmissionCache)
                    .is_err()
            );
        }
        let mut invalid_resets = raw;
        invalid_resets["reset_only_rounds"] = 9_112.into();
        let decoded = serde_json::from_value::<PolicyEvidence>(invalid_resets).unwrap();
        assert!(
            decoded
                .validate_for(ExecutionMode::TransactionFencesAdmissionCache)
                .is_err()
        );
        let uncached = PolicyEvidence::TransactionFences {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: 9_111,
        };
        assert!(
            uncached
                .validate_for(ExecutionMode::TransactionFencesAdmissionCache)
                .is_err()
        );
    }

    #[test]
    fn metadata_upload_modes_are_closed_and_default_to_separate_writes() {
        assert_eq!(
            MetadataUploadMode::default(),
            MetadataUploadMode::SeparateWrites
        );
        for mode in [
            MetadataUploadMode::SeparateWrites,
            MetadataUploadMode::Batched,
        ] {
            assert_eq!(MetadataUploadMode::parse(mode.argument()).unwrap(), mode);
            assert_eq!(serde_json::to_value(mode).unwrap(), mode.argument());
        }
        for unknown in ["", "batch", "default", "Batched", "batched ", "cached"] {
            assert!(MetadataUploadMode::parse(unknown).is_err());
            assert!(serde_json::from_value::<MetadataUploadMode>(unknown.into()).is_err());
        }
    }

    #[test]
    fn actual_transaction_counter_congruence_and_policy_are_closed() {
        for loops in [9_111, 10_631, 12_151] {
            PolicyEvidence::TransactionFences {
                full_boundaries: 2,
                operational_boundaries: 9,
                reset_only_rounds: 1,
                loop_checks: loops,
            }
            .validate_for(ExecutionMode::TransactionFences)
            .unwrap();
        }
        for loops in [0, 9_110, 9_112, 10_630, 10_632] {
            assert!(
                PolicyEvidence::TransactionFences {
                    full_boundaries: 2,
                    operational_boundaries: 9,
                    reset_only_rounds: 1,
                    loop_checks: loops,
                }
                .validate_for(ExecutionMode::TransactionFences)
                .is_err()
            );
        }
        assert!(
            PolicyEvidence::QueuedBaseline {}
                .validate_for(ExecutionMode::TransactionFences)
                .is_err()
        );
        let mut value = serde_json::to_value(PolicyEvidence::QueuedBaseline {}).unwrap();
        value["loop_checks"] = 9_111.into();
        assert!(serde_json::from_value::<PolicyEvidence>(value).is_err());
    }

    #[test]
    fn policy_evidence_round_trips_without_accepting_unknown_fields() {
        let baseline = PolicyEvidence::QueuedBaseline {};
        let baseline_json = serde_json::json!({"execution_mode": "queued-baseline"});
        assert_eq!(serde_json::to_value(&baseline).unwrap(), baseline_json);
        assert_eq!(
            serde_json::from_value::<PolicyEvidence>(baseline_json.clone()).unwrap(),
            baseline,
        );
        for field in [
            "full_boundaries",
            "operational_boundaries",
            "reset_only_rounds",
            "loop_checks",
            "future_field",
        ] {
            let mut unknown = baseline_json.clone();
            unknown[field] = 1.into();
            assert!(serde_json::from_value::<PolicyEvidence>(unknown).is_err());
        }
        let transaction = PolicyEvidence::TransactionFences {
            full_boundaries: 2,
            operational_boundaries: 9,
            reset_only_rounds: 1,
            loop_checks: 9_111,
        };
        let mut transaction_json = serde_json::to_value(&transaction).unwrap();
        assert_eq!(
            serde_json::from_value::<PolicyEvidence>(transaction_json.clone()).unwrap(),
            transaction,
        );
        transaction_json["future_field"] = 1.into();
        assert!(serde_json::from_value::<PolicyEvidence>(transaction_json).is_err());
    }
}
