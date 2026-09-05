//! Authority-free wire and collector types for the Ferric R33 service.
//!
//! The collector frontend is killed after every action. These types therefore
//! carry one action to an independently supervised Unix-domain service. They
//! bind identities and observations only; they grant no artifact, publication,
//! model-memory, queue, or launch authority.

use std::collections::BTreeSet;
use std::env::VarError;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, Read, Write};

use ferric_spec::{M1_MAX_CONTEXT_TOKENS, QWEN3_VOCABULARY_SIZE};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Environment variable containing the canonical supervised-service plan path.
pub const M1_R33_SERVICE_PLAN_ENV_V1: &str = "FERRIC_R33_SERVICE_PLAN_PATH_V1";
/// Environment variable binding the exact canonical service-plan bytes.
pub const M1_R33_SERVICE_PLAN_SHA256_ENV_V1: &str = "FERRIC_R33_SERVICE_PLAN_SHA256_V1";
/// Exact collector result schema consumed by the R33 V3 collector.
pub const M1_R33_COLLECTOR_RESULT_FORMAT_V1: &str = "FERRIC-M1-R33-SERVING-ADAPTER-RESULT-V2";
/// Authority label required by the external collector.
pub const M1_R33_COLLECTOR_RESULT_AUTHORITY_V1: &str = "external-r33-serving-adapter-report-only";
/// Exact service request schema.
pub const M1_R33_WIRE_REQUEST_FORMAT_V1: &str = "FERRIC-M1-R33-SERVICE-REQUEST-V1";
/// Exact service response schema.
pub const M1_R33_WIRE_RESPONSE_FORMAT_V1: &str = "FERRIC-M1-R33-SERVICE-RESPONSE-V1";
/// Exact response-delivery acknowledgement schema.
pub const M1_R33_WIRE_ACK_FORMAT_V1: &str = "FERRIC-M1-R33-SERVICE-ACK-V1";
/// Exact fixed M1 target admitted by this adapter.
pub const M1_R33_TARGET_V1: &str = "gfx942:xnack-";
/// R33's exact timing clock.
pub const M1_R33_CLOCK_V1: &str = "monotonic-raw-nanoseconds";
/// R33's exact measured window boundary.
pub const M1_R33_DURATION_BOUNDARY_V1: &str = "declared-window-start-to-declared-window-end";
/// R33's exact per-request timing boundaries.
pub const M1_R33_TIMING_BOUNDARIES_V1: &str =
    "request-arrival-to-first-output-token-observed-to-terminal-event";
/// Maximum successful requests in one bounded M1 window.
pub const M1_R33_MAX_REQUESTS_PER_WINDOW_V1: usize = 32;
/// Exact number of windows served by one backend instance.
pub const M1_R33_WINDOWS_PER_START_V1: usize = 20;
/// Maximum canonical JSON payload accepted by the local wire.
pub const M1_R33_MAX_WIRE_PAYLOAD_BYTES_V1: usize = 8 * 1024 * 1024;

const FRAME_MAGIC_V1: [u8; 8] = *b"FRR33V1\0";
const FRAME_VERSION_V1: u16 = 1;
const FRAME_HEADER_BYTES_V1: usize = 48;

/// One R33 lifecycle action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum M1R33ActionV1 {
    Start,
    Ready,
    Measure,
    Stop,
}

impl M1R33ActionV1 {
    /// Exact lowercase collector spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Ready => "ready",
            Self::Measure => "measure",
            Self::Stop => "stop",
        }
    }

    fn parse(value: &str) -> Result<Self, M1R33WireErrorV1> {
        match value {
            "start" => Ok(Self::Start),
            "ready" => Ok(Self::Ready),
            "measure" => Ok(Self::Measure),
            "stop" => Ok(Self::Stop),
            _ => Err(M1R33WireErrorV1::Context("action")),
        }
    }
}

/// Exact aggregate work declared for one collector row.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33WorkV1 {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub successful_requests: u64,
    pub total_tokens: u64,
}

impl M1R33WorkV1 {
    pub(crate) fn validate(self) -> Result<(), M1R33WireErrorV1> {
        let request_count = usize::try_from(self.successful_requests)
            .map_err(|_| M1R33WireErrorV1::Context("successful requests"))?;
        if self.input_tokens == 0
            || self.output_tokens == 0
            || request_count == 0
            || request_count > M1_R33_MAX_REQUESTS_PER_WINDOW_V1
            || self.input_tokens.checked_add(self.output_tokens) != Some(self.total_tokens)
        {
            return Err(M1R33WireErrorV1::Context("expected work"));
        }
        Ok(())
    }
}

/// Exact collector row binding, without physical prompt custody.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33CollectorRowV1 {
    pub expected_work: M1R33WorkV1,
    pub id: String,
    pub ordinal: u64,
    pub phase: String,
    pub server_start: u64,
    pub window: u64,
}

impl M1R33CollectorRowV1 {
    pub(crate) fn validate(&self) -> Result<(), M1R33WireErrorV1> {
        self.expected_work.validate()?;
        require_ascii_token(&self.id, 256, "row id")?;
        if !matches!(self.phase.as_str(), "warmup" | "recorded")
            || self.server_start >= 3
            || self.window >= 10
        {
            return Err(M1R33WireErrorV1::Context("row coordinates"));
        }
        let local = self
            .ordinal
            .checked_sub(self.server_start * M1_R33_WINDOWS_PER_START_V1 as u64)
            .ok_or(M1R33WireErrorV1::Context("row ordinal"))?;
        let expected_phase = if local < 10 { "warmup" } else { "recorded" };
        if local >= M1_R33_WINDOWS_PER_START_V1 as u64
            || self.phase != expected_phase
            || self.window != local % 10
        {
            return Err(M1R33WireErrorV1::Context("row sequence"));
        }
        Ok(())
    }
}

/// Exact hardware-slot binding echoed to the collector and daemon.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33SlotV1 {
    pub hardware_configuration_sha256: String,
    pub hardware_sha256: String,
    pub id: String,
    pub target: String,
}

impl M1R33SlotV1 {
    pub(crate) fn validate(&self) -> Result<(), M1R33WireErrorV1> {
        require_sha256(&self.hardware_configuration_sha256, "slot configuration")?;
        require_sha256(&self.hardware_sha256, "slot hardware")?;
        require_ascii_token(&self.id, 128, "slot id")?;
        if self.target != M1_R33_TARGET_V1 {
            return Err(M1R33WireErrorV1::Context("slot target"));
        }
        Ok(())
    }
}

/// Collector invocation captured from its reserved environment contract.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33CollectorContextV1 {
    pub action: M1R33ActionV1,
    pub command_sha256: String,
    pub engine: String,
    pub engine_order: Vec<String>,
    pub implementation: Value,
    pub policy_sha256: String,
    pub row: Option<M1R33CollectorRowV1>,
    pub server_instance_sha256: Option<String>,
    pub server_start: u64,
    pub slot: M1R33SlotV1,
    pub target: String,
}

impl M1R33CollectorContextV1 {
    /// Reads and validates the exact reserved environment emitted by the R33 collector.
    ///
    /// # Errors
    ///
    /// Rejects missing, non-Unicode, malformed, cross-engine, cross-slot, or
    /// action-inconsistent values.
    pub fn from_current_environment() -> Result<Self, M1R33WireErrorV1> {
        Self::from_environment(|name| std::env::var_os(name))
    }

    pub(crate) fn from_environment(
        mut read: impl FnMut(&str) -> Option<OsString>,
    ) -> Result<Self, M1R33WireErrorV1> {
        let action = M1R33ActionV1::parse(&required_unicode(&mut read, "FERRIC_M1_R33_ACTION")?)?;
        let command_sha256 = required_unicode(&mut read, "FERRIC_M1_R33_COMMAND_SHA256")?;
        let engine = required_unicode(&mut read, "FERRIC_M1_R33_ENGINE")?;
        let engine_order = required_unicode(&mut read, "FERRIC_M1_R33_ENGINE_ORDER")?
            .split(',')
            .map(str::to_owned)
            .collect();
        let implementation_source =
            required_unicode(&mut read, "FERRIC_M1_R33_IMPLEMENTATION_JSON")?;
        let implementation: Value = serde_json::from_str(&implementation_source)
            .map_err(|_| M1R33WireErrorV1::Context("implementation JSON"))?;
        if serde_json::to_string(&implementation)
            .map_err(|_| M1R33WireErrorV1::Context("implementation JSON"))?
            != implementation_source
        {
            return Err(M1R33WireErrorV1::Context("implementation canonical JSON"));
        }
        let policy_sha256 = required_unicode(&mut read, "FERRIC_M1_R33_POLICY_SHA256")?;
        let server_start = parse_u64(
            &required_unicode(&mut read, "FERRIC_M1_R33_SERVER_START")?,
            "server start",
        )?;
        let slot = M1R33SlotV1 {
            hardware_configuration_sha256: required_unicode(
                &mut read,
                "FERRIC_M1_R33_SLOT_CONFIGURATION_SHA256",
            )?,
            hardware_sha256: required_unicode(&mut read, "FERRIC_M1_R33_SLOT_SHA256")?,
            id: required_unicode(&mut read, "FERRIC_M1_R33_SLOT_ID")?,
            target: required_unicode(&mut read, "FERRIC_M1_R33_TARGET")?,
        };
        let target = slot.target.clone();

        let server_instance_sha256 = optional_unicode(
            read("FERRIC_M1_R33_SERVER_INSTANCE_SHA256"),
            "FERRIC_M1_R33_SERVER_INSTANCE_SHA256",
        )?;
        let row = read_optional_row(&mut read, server_start)?;
        let context = Self {
            action,
            command_sha256,
            engine,
            engine_order,
            implementation,
            policy_sha256,
            row,
            server_instance_sha256,
            server_start,
            slot,
            target,
        };
        context.validate()?;
        Ok(context)
    }

    pub(crate) fn validate(&self) -> Result<(), M1R33WireErrorV1> {
        require_sha256(&self.command_sha256, "command")?;
        require_sha256(&self.policy_sha256, "policy")?;
        self.slot.validate()?;
        if self.engine != "ferric" || self.target != M1_R33_TARGET_V1 || self.server_start >= 3 {
            return Err(M1R33WireErrorV1::Context("collector binding"));
        }
        let expected_order: BTreeSet<&str> = ["ferric", "vllm", "sglang"].into_iter().collect();
        let actual_order: BTreeSet<&str> = self.engine_order.iter().map(String::as_str).collect();
        if self.engine_order.len() != 3 || actual_order != expected_order {
            return Err(M1R33WireErrorV1::Context("engine order"));
        }
        if !canonical_json_bytes(&self.implementation)?.is_ascii() {
            return Err(M1R33WireErrorV1::Context("implementation ASCII"));
        }
        match self.action {
            M1R33ActionV1::Start => {
                if self.server_instance_sha256.is_some() || self.row.is_some() {
                    return Err(M1R33WireErrorV1::Context("start shape"));
                }
            }
            M1R33ActionV1::Measure => {
                require_sha256(
                    self.server_instance_sha256
                        .as_deref()
                        .ok_or(M1R33WireErrorV1::Context("measure instance"))?,
                    "server instance",
                )?;
                let row = self
                    .row
                    .as_ref()
                    .ok_or(M1R33WireErrorV1::Context("measure row"))?;
                row.validate()?;
                if row.server_start != self.server_start {
                    return Err(M1R33WireErrorV1::Context("measure server start"));
                }
            }
            M1R33ActionV1::Ready | M1R33ActionV1::Stop => {
                require_sha256(
                    self.server_instance_sha256
                        .as_deref()
                        .ok_or(M1R33WireErrorV1::Context("lifecycle instance"))?,
                    "server instance",
                )?;
                if self.row.is_some() {
                    return Err(M1R33WireErrorV1::Context("lifecycle row"));
                }
            }
        }
        Ok(())
    }

    /// Builds the exact common collector result around a service observation.
    ///
    /// # Errors
    ///
    /// Rejects a response that is not bound to this invocation or reports
    /// invalid work/timing data.
    pub fn collector_result(
        &self,
        response: &M1R33WireResponseV1,
    ) -> Result<Value, M1R33WireErrorV1> {
        response.validate()?;
        if response.status != M1R33WireStatusV1::Passed
            || response.action != self.action
            || response.server_start != self.server_start
            || response.policy_sha256 != self.policy_sha256
            || response.slot_id != self.slot.id
        {
            return Err(M1R33WireErrorV1::ResponseBinding);
        }
        let instance = response
            .server_instance_sha256
            .as_deref()
            .ok_or(M1R33WireErrorV1::ResponseBinding)?;
        if self
            .server_instance_sha256
            .as_deref()
            .is_some_and(|expected| expected != instance)
        {
            return Err(M1R33WireErrorV1::ResponseBinding);
        }
        let reported = match (&self.action, &response.reported) {
            (M1R33ActionV1::Measure, Some(M1R33WireReportV1::Measurement(report))) => {
                report.validate_against(
                    &self
                        .row
                        .as_ref()
                        .ok_or(M1R33WireErrorV1::ResponseBinding)?
                        .expected_work,
                )?;
                serde_json::to_value(report).map_err(M1R33WireErrorV1::Json)?
            }
            (
                M1R33ActionV1::Start | M1R33ActionV1::Ready | M1R33ActionV1::Stop,
                Some(M1R33WireReportV1::Lifecycle),
            ) => json!({"kind": "lifecycle"}),
            _ => return Err(M1R33WireErrorV1::ResponseBinding),
        };
        Ok(json!({
            "action": self.action.as_str(),
            "authority": M1_R33_COLLECTOR_RESULT_AUTHORITY_V1,
            "command_sha256": self.command_sha256,
            "engine": self.engine,
            "engine_order": self.engine_order,
            "format": M1_R33_COLLECTOR_RESULT_FORMAT_V1,
            "implementation": self.implementation,
            "policy_sha256": self.policy_sha256,
            "reported": reported,
            "row": self.row,
            "server_instance_sha256": instance,
            "server_start": self.server_start,
            "slot": self.slot,
            "status": "passed",
            "target": self.target,
        }))
    }
}

fn read_optional_row(
    read: &mut impl FnMut(&str) -> Option<OsString>,
    server_start: u64,
) -> Result<Option<M1R33CollectorRowV1>, M1R33WireErrorV1> {
    const ROW_NAMES: [&str; 8] = [
        "FERRIC_M1_R33_ROW_ID",
        "FERRIC_M1_R33_ORDINAL",
        "FERRIC_M1_R33_PHASE",
        "FERRIC_M1_R33_WINDOW",
        "FERRIC_M1_R33_EXPECTED_SUCCESSFUL_REQUESTS",
        "FERRIC_M1_R33_EXPECTED_INPUT_TOKENS",
        "FERRIC_M1_R33_EXPECTED_OUTPUT_TOKENS",
        "FERRIC_M1_R33_EXPECTED_TOTAL_TOKENS",
    ];
    let values = ROW_NAMES
        .map(|name| optional_unicode(read(name), name))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    if values.iter().any(Option::is_none) {
        return Err(M1R33WireErrorV1::Context("partial row environment"));
    }
    let values = values
        .into_iter()
        .map(|value| value.expect("all optional row values were checked"))
        .collect::<Vec<_>>();
    Ok(Some(M1R33CollectorRowV1 {
        expected_work: M1R33WorkV1 {
            successful_requests: parse_u64(&values[4], "successful requests")?,
            input_tokens: parse_u64(&values[5], "input tokens")?,
            output_tokens: parse_u64(&values[6], "output tokens")?,
            total_tokens: parse_u64(&values[7], "total tokens")?,
        },
        id: values[0].clone(),
        ordinal: parse_u64(&values[1], "ordinal")?,
        phase: values[2].clone(),
        server_start,
        window: parse_u64(&values[3], "window")?,
    }))
}

/// One canonical pretokenized request held by the supervised service.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33WorkloadRequestV1 {
    pub expected_output_tokens: u64,
    pub prompt_tokens: Vec<u32>,
    pub request_ordinal: u64,
}

impl M1R33WorkloadRequestV1 {
    pub(crate) fn validate(&self, ordinal: usize) -> Result<(), M1R33WireErrorV1> {
        if self.request_ordinal != ordinal as u64
            || self.prompt_tokens.is_empty()
            || self
                .prompt_tokens
                .iter()
                .any(|token| *token >= QWEN3_VOCABULARY_SIZE)
            || self.expected_output_tokens < 2
            || u64::try_from(self.prompt_tokens.len())
                .ok()
                .and_then(|input| input.checked_add(self.expected_output_tokens))
                .is_none_or(|total| total > u64::from(M1_MAX_CONTEXT_TOKENS))
        {
            return Err(M1R33WireErrorV1::Context("pretokenized request"));
        }
        Ok(())
    }
}

/// Per-request event returned by the physical backend.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33RequestEventWireV1 {
    pub arrival_offset_ns: u64,
    pub first_token_offset_ns: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub request_ordinal: u64,
    pub terminal_offset_ns: u64,
}

/// Exact measurement report accepted by the external collector.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33MeasurementReportV1 {
    pub clock: String,
    pub duration_boundary: String,
    pub duration_ns: u64,
    pub failed_requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub request_events: Vec<M1R33RequestEventWireV1>,
    pub request_timing_boundaries: String,
    pub successful_requests: u64,
    pub total_tokens: u64,
}

impl M1R33MeasurementReportV1 {
    pub(crate) fn validate_against(&self, expected: &M1R33WorkV1) -> Result<(), M1R33WireErrorV1> {
        if self.clock != M1_R33_CLOCK_V1
            || self.duration_boundary != M1_R33_DURATION_BOUNDARY_V1
            || self.request_timing_boundaries != M1_R33_TIMING_BOUNDARIES_V1
            || self.duration_ns == 0
            || self.failed_requests != 0
            || self.input_tokens != expected.input_tokens
            || self.output_tokens != expected.output_tokens
            || self.successful_requests != expected.successful_requests
            || self.total_tokens != expected.total_tokens
            || usize::try_from(expected.successful_requests) != Ok(self.request_events.len())
        {
            return Err(M1R33WireErrorV1::Measurement);
        }
        let mut input = 0_u64;
        let mut output = 0_u64;
        for (ordinal, event) in self.request_events.iter().enumerate() {
            if event.request_ordinal != ordinal as u64
                || event.input_tokens == 0
                || event.output_tokens < 2
                || !(event.arrival_offset_ns < event.first_token_offset_ns
                    && event.first_token_offset_ns < event.terminal_offset_ns
                    && event.terminal_offset_ns <= self.duration_ns)
                || (event.terminal_offset_ns - event.first_token_offset_ns)
                    / (event.output_tokens - 1)
                    == 0
            {
                return Err(M1R33WireErrorV1::Measurement);
            }
            input = input
                .checked_add(event.input_tokens)
                .ok_or(M1R33WireErrorV1::Measurement)?;
            output = output
                .checked_add(event.output_tokens)
                .ok_or(M1R33WireErrorV1::Measurement)?;
        }
        if input != self.input_tokens || output != self.output_tokens {
            return Err(M1R33WireErrorV1::Measurement);
        }
        Ok(())
    }
}

/// Success report transported from daemon to frontend.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "kind", content = "value")]
pub enum M1R33WireReportV1 {
    Lifecycle,
    Measurement(M1R33MeasurementReportV1),
}

/// Wire response status. Fault responses are never converted into collector results.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum M1R33WireStatusV1 {
    Passed,
    Fault,
}

/// Canonical request sent by one short-lived collector frontend.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33WireRequestV1 {
    pub context: M1R33CollectorContextV1,
    pub format: String,
    pub request_sha256: String,
    pub service_id: String,
    pub service_plan_sha256: String,
}

impl M1R33WireRequestV1 {
    pub(crate) fn validate(&self) -> Result<(), M1R33WireErrorV1> {
        self.context.validate()?;
        if self.format != M1_R33_WIRE_REQUEST_FORMAT_V1 {
            return Err(M1R33WireErrorV1::Protocol("request format"));
        }
        for (value, name) in [
            (&self.request_sha256, "request"),
            (&self.service_id, "service"),
            (&self.service_plan_sha256, "service plan"),
        ] {
            require_sha256(value, name)?;
        }
        Ok(())
    }
}

/// Canonical response returned by the independently supervised service.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33WireResponseV1 {
    pub action: M1R33ActionV1,
    pub error_code: Option<String>,
    pub format: String,
    pub policy_sha256: String,
    pub reported: Option<M1R33WireReportV1>,
    pub request_sha256: String,
    pub server_instance_sha256: Option<String>,
    pub server_start: u64,
    pub service_id: String,
    pub service_plan_sha256: String,
    pub slot_id: String,
    pub status: M1R33WireStatusV1,
}

/// Canonical frontend acknowledgement after full response validation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct M1R33WireAckV1 {
    pub format: String,
    pub request_sha256: String,
    pub response_sha256: String,
    pub service_id: String,
    pub service_plan_sha256: String,
}

impl M1R33WireAckV1 {
    pub(crate) fn validate(&self) -> Result<(), M1R33WireErrorV1> {
        if self.format != M1_R33_WIRE_ACK_FORMAT_V1 {
            return Err(M1R33WireErrorV1::Protocol("ack format"));
        }
        for (value, name) in [
            (&self.request_sha256, "ack request"),
            (&self.response_sha256, "ack response"),
            (&self.service_id, "ack service"),
            (&self.service_plan_sha256, "ack service plan"),
        ] {
            require_sha256(value, name)?;
        }
        Ok(())
    }
}

impl M1R33WireResponseV1 {
    pub(crate) fn validate(&self) -> Result<(), M1R33WireErrorV1> {
        if self.format != M1_R33_WIRE_RESPONSE_FORMAT_V1 || self.server_start >= 3 {
            return Err(M1R33WireErrorV1::Protocol("response format"));
        }
        for (value, name) in [
            (&self.policy_sha256, "policy"),
            (&self.request_sha256, "request"),
            (&self.service_id, "service"),
            (&self.service_plan_sha256, "service plan"),
        ] {
            require_sha256(value, name)?;
        }
        require_ascii_token(&self.slot_id, 128, "slot id")?;
        if let Some(instance) = &self.server_instance_sha256 {
            require_sha256(instance, "server instance")?;
        }
        match self.status {
            M1R33WireStatusV1::Passed => {
                if self.error_code.is_some()
                    || self.reported.is_none()
                    || self.server_instance_sha256.is_none()
                {
                    return Err(M1R33WireErrorV1::Protocol("passed response shape"));
                }
            }
            M1R33WireStatusV1::Fault => {
                let code = self
                    .error_code
                    .as_deref()
                    .ok_or(M1R33WireErrorV1::Protocol("fault response code"))?;
                require_ascii_token(code, 128, "fault code")?;
                if self.reported.is_some() {
                    return Err(M1R33WireErrorV1::Protocol("fault response report"));
                }
            }
        }
        Ok(())
    }
}

/// One frame direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33FrameKindV1 {
    Request,
    Response,
    Ack,
}

impl M1R33FrameKindV1 {
    const fn code(self) -> u16 {
        match self {
            Self::Request => 1,
            Self::Response => 2,
            Self::Ack => 3,
        }
    }
}

/// Fail-closed protocol, context, framing, or measurement error.
#[derive(Debug)]
pub enum M1R33WireErrorV1 {
    Environment(&'static str),
    Context(&'static str),
    Protocol(&'static str),
    Frame(&'static str),
    PayloadTooLarge,
    NonCanonicalPayload,
    ResponseBinding,
    Measurement,
    Io(io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for M1R33WireErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Ferric M1 R33 wire rejected: {self:?}")
    }
}

impl std::error::Error for M1R33WireErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Json(source) => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for M1R33WireErrorV1 {
    fn from(source: io::Error) -> Self {
        Self::Io(source)
    }
}

/// Encodes exact pretty canonical JSON with one trailing newline.
///
/// # Errors
///
/// Rejects non-ASCII or oversized values and serialization failures.
pub fn encode_canonical_json_v1<T: Serialize>(value: &T) -> Result<Vec<u8>, M1R33WireErrorV1> {
    let value = serde_json::to_value(value).map_err(M1R33WireErrorV1::Json)?;
    canonical_json_bytes(&value)
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, M1R33WireErrorV1> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(M1R33WireErrorV1::Json)?;
    bytes.push(b'\n');
    if !bytes.is_ascii() {
        return Err(M1R33WireErrorV1::NonCanonicalPayload);
    }
    if bytes.len() > M1_R33_MAX_WIRE_PAYLOAD_BYTES_V1 {
        return Err(M1R33WireErrorV1::PayloadTooLarge);
    }
    Ok(bytes)
}

/// Decodes one exact canonical JSON payload.
///
/// # Errors
///
/// Rejects empty, non-ASCII, oversized, noncanonical, unknown-field, or
/// schema-invalid payloads.
pub fn decode_canonical_json_v1<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, M1R33WireErrorV1> {
    if bytes.is_empty() || bytes.len() > M1_R33_MAX_WIRE_PAYLOAD_BYTES_V1 {
        return Err(M1R33WireErrorV1::PayloadTooLarge);
    }
    if !bytes.is_ascii() {
        return Err(M1R33WireErrorV1::NonCanonicalPayload);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(M1R33WireErrorV1::Json)?;
    if canonical_json_bytes(&value)? != bytes {
        return Err(M1R33WireErrorV1::NonCanonicalPayload);
    }
    serde_json::from_value(value).map_err(M1R33WireErrorV1::Json)
}

/// Writes one bounded digest-bound canonical frame.
///
/// # Errors
///
/// Returns serialization or I/O failure without retrying partial output.
pub fn write_frame_v1<T: Serialize>(
    writer: &mut impl Write,
    kind: M1R33FrameKindV1,
    value: &T,
) -> Result<(), M1R33WireErrorV1> {
    let payload = encode_canonical_json_v1(value)?;
    let length = u32::try_from(payload.len()).map_err(|_| M1R33WireErrorV1::PayloadTooLarge)?;
    let mut header = [0_u8; FRAME_HEADER_BYTES_V1];
    header[..8].copy_from_slice(&FRAME_MAGIC_V1);
    header[8..10].copy_from_slice(&FRAME_VERSION_V1.to_be_bytes());
    header[10..12].copy_from_slice(&kind.code().to_be_bytes());
    header[12..16].copy_from_slice(&length.to_be_bytes());
    header[16..].copy_from_slice(&Sha256::digest(&payload));
    writer.write_all(&header)?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

/// Reads one bounded digest-bound canonical frame and requires exact EOF.
///
/// # Errors
///
/// Rejects wrong magic/version/kind, truncation, trailing bytes, digest drift,
/// oversize payloads, or noncanonical JSON.
pub fn read_frame_v1<T: DeserializeOwned>(
    reader: &mut impl Read,
    expected_kind: M1R33FrameKindV1,
) -> Result<T, M1R33WireErrorV1> {
    let value = read_frame_open_v1(reader, expected_kind)?;
    require_exact_eof_v1(reader)?;
    Ok(value)
}

pub(crate) fn read_frame_open_v1<T: DeserializeOwned>(
    reader: &mut impl Read,
    expected_kind: M1R33FrameKindV1,
) -> Result<T, M1R33WireErrorV1> {
    let mut header = [0_u8; FRAME_HEADER_BYTES_V1];
    reader.read_exact(&mut header)?;
    if header[..8] != FRAME_MAGIC_V1
        || u16::from_be_bytes([header[8], header[9]]) != FRAME_VERSION_V1
        || u16::from_be_bytes([header[10], header[11]]) != expected_kind.code()
    {
        return Err(M1R33WireErrorV1::Frame("header"));
    }
    let length = u32::from_be_bytes([header[12], header[13], header[14], header[15]]) as usize;
    if length == 0 || length > M1_R33_MAX_WIRE_PAYLOAD_BYTES_V1 {
        return Err(M1R33WireErrorV1::PayloadTooLarge);
    }
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(length)
        .map_err(|_| M1R33WireErrorV1::PayloadTooLarge)?;
    payload.resize(length, 0);
    reader.read_exact(&mut payload)?;
    if header[16..] != Sha256::digest(&payload)[..] {
        return Err(M1R33WireErrorV1::Frame("payload digest"));
    }
    decode_canonical_json_v1(&payload)
}

pub(crate) fn require_exact_eof_v1(reader: &mut impl Read) -> Result<(), M1R33WireErrorV1> {
    let mut trailing = [0_u8; 1];
    if reader.read(&mut trailing)? != 0 {
        return Err(M1R33WireErrorV1::Frame("trailing bytes"));
    }
    Ok(())
}

pub(crate) fn require_sha256(value: &str, name: &'static str) -> Result<(), M1R33WireErrorV1> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(M1R33WireErrorV1::Context(name));
    }
    Ok(())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn require_ascii_token(
    value: &str,
    maximum: usize,
    name: &'static str,
) -> Result<(), M1R33WireErrorV1> {
    if value.is_empty()
        || value.len() > maximum
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'/' | b'\\'))
    {
        return Err(M1R33WireErrorV1::Context(name));
    }
    Ok(())
}

fn parse_u64(value: &str, name: &'static str) -> Result<u64, M1R33WireErrorV1> {
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return Err(M1R33WireErrorV1::Context(name));
    }
    value.parse().map_err(|_| M1R33WireErrorV1::Context(name))
}

fn optional_unicode(
    value: Option<OsString>,
    name: &'static str,
) -> Result<Option<String>, M1R33WireErrorV1> {
    value
        .map(|value| {
            value
                .into_string()
                .map_err(|_| M1R33WireErrorV1::Environment(name))
        })
        .transpose()
}

fn required_unicode(
    read: &mut impl FnMut(&str) -> Option<OsString>,
    name: &'static str,
) -> Result<String, M1R33WireErrorV1> {
    read(name)
        .ok_or(M1R33WireErrorV1::Environment(name))?
        .into_string()
        .map_err(|_| M1R33WireErrorV1::Environment(name))
}

impl From<VarError> for M1R33WireErrorV1 {
    fn from(_: VarError) -> Self {
        Self::Environment(M1_R33_SERVICE_PLAN_ENV_V1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::Cursor;

    fn digest(label: &str) -> String {
        sha256_hex(label.as_bytes())
    }

    fn context_environment(action: &str) -> BTreeMap<String, OsString> {
        let mut values = BTreeMap::new();
        for (name, value) in [
            ("FERRIC_M1_R33_ACTION", action.to_owned()),
            ("FERRIC_M1_R33_COMMAND_SHA256", digest(action)),
            ("FERRIC_M1_R33_ENGINE", "ferric".to_owned()),
            (
                "FERRIC_M1_R33_ENGINE_ORDER",
                "ferric,vllm,sglang".to_owned(),
            ),
            (
                "FERRIC_M1_R33_IMPLEMENTATION_JSON",
                "{\"id\":\"ferric\"}".to_owned(),
            ),
            ("FERRIC_M1_R33_POLICY_SHA256", digest("policy")),
            ("FERRIC_M1_R33_SERVER_START", "0".to_owned()),
            (
                "FERRIC_M1_R33_SLOT_CONFIGURATION_SHA256",
                digest("configuration"),
            ),
            ("FERRIC_M1_R33_SLOT_ID", "slot-0".to_owned()),
            ("FERRIC_M1_R33_SLOT_SHA256", digest("hardware")),
            ("FERRIC_M1_R33_TARGET", M1_R33_TARGET_V1.to_owned()),
        ] {
            values.insert(name.to_owned(), OsString::from(value));
        }
        if action != "start" {
            values.insert(
                "FERRIC_M1_R33_SERVER_INSTANCE_SHA256".to_owned(),
                OsString::from(digest("instance")),
            );
        }
        if action == "measure" {
            for (name, value) in [
                ("FERRIC_M1_R33_ROW_ID", "start-0.warmup-00"),
                ("FERRIC_M1_R33_ORDINAL", "0"),
                ("FERRIC_M1_R33_PHASE", "warmup"),
                ("FERRIC_M1_R33_WINDOW", "0"),
                ("FERRIC_M1_R33_EXPECTED_SUCCESSFUL_REQUESTS", "1"),
                ("FERRIC_M1_R33_EXPECTED_INPUT_TOKENS", "2"),
                ("FERRIC_M1_R33_EXPECTED_OUTPUT_TOKENS", "2"),
                ("FERRIC_M1_R33_EXPECTED_TOTAL_TOKENS", "4"),
            ] {
                values.insert(name.to_owned(), OsString::from(value));
            }
        }
        values
    }

    fn parse_context(
        values: &BTreeMap<String, OsString>,
    ) -> Result<M1R33CollectorContextV1, M1R33WireErrorV1> {
        M1R33CollectorContextV1::from_environment(|name| values.get(name).cloned())
    }

    #[test]
    fn collector_environment_is_action_exact() {
        for action in ["start", "ready", "measure", "stop"] {
            assert_eq!(
                parse_context(&context_environment(action))
                    .unwrap()
                    .action
                    .as_str(),
                action
            );
        }
        let mut values = context_environment("measure");
        values.remove("FERRIC_M1_R33_EXPECTED_TOTAL_TOKENS");
        assert!(parse_context(&values).is_err());
        let mut values = context_environment("ready");
        values.insert(
            "FERRIC_M1_R33_ROW_ID".to_owned(),
            OsString::from("substitution"),
        );
        assert!(parse_context(&values).is_err());
    }

    #[test]
    fn frame_rejects_header_digest_truncation_and_trailing_data() {
        let value = json!({"a": 1});
        let mut bytes = Vec::new();
        write_frame_v1(&mut bytes, M1R33FrameKindV1::Request, &value).unwrap();
        assert_eq!(
            read_frame_v1::<Value>(&mut Cursor::new(&bytes), M1R33FrameKindV1::Request).unwrap(),
            value
        );

        let mut wrong_kind = bytes.clone();
        wrong_kind[11] = 2;
        assert!(
            read_frame_v1::<Value>(&mut Cursor::new(wrong_kind), M1R33FrameKindV1::Request)
                .is_err()
        );
        let mut digest_drift = bytes.clone();
        digest_drift[16] ^= 1;
        assert!(
            read_frame_v1::<Value>(&mut Cursor::new(digest_drift), M1R33FrameKindV1::Request)
                .is_err()
        );
        assert!(
            read_frame_v1::<Value>(
                &mut Cursor::new(&bytes[..bytes.len() - 1]),
                M1R33FrameKindV1::Request
            )
            .is_err()
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert!(
            read_frame_v1::<Value>(&mut Cursor::new(trailing), M1R33FrameKindV1::Request).is_err()
        );
    }

    #[test]
    fn canonical_codec_rejects_whitespace_unknown_fields_and_non_ascii() {
        assert!(decode_canonical_json_v1::<Value>(b"{\"a\":1}\n").is_err());
        assert!(decode_canonical_json_v1::<Value>("{\n  \"a\": \"é\"\n}\n".as_bytes()).is_err());
        let context = parse_context(&context_environment("start")).unwrap();
        let mut value = serde_json::to_value(&context).unwrap();
        value["unexpected"] = Value::Bool(true);
        let bytes = encode_canonical_json_v1(&value).unwrap();
        assert!(decode_canonical_json_v1::<M1R33CollectorContextV1>(&bytes).is_err());
    }

    #[test]
    fn measurement_rejects_bad_order_work_and_zero_tpot() {
        let expected = M1R33WorkV1 {
            input_tokens: 2,
            output_tokens: 2,
            successful_requests: 1,
            total_tokens: 4,
        };
        let mut report = M1R33MeasurementReportV1 {
            clock: M1_R33_CLOCK_V1.to_owned(),
            duration_boundary: M1_R33_DURATION_BOUNDARY_V1.to_owned(),
            duration_ns: 10,
            failed_requests: 0,
            input_tokens: 2,
            output_tokens: 2,
            request_events: vec![M1R33RequestEventWireV1 {
                arrival_offset_ns: 0,
                first_token_offset_ns: 2,
                input_tokens: 2,
                output_tokens: 2,
                request_ordinal: 0,
                terminal_offset_ns: 4,
            }],
            request_timing_boundaries: M1_R33_TIMING_BOUNDARIES_V1.to_owned(),
            successful_requests: 1,
            total_tokens: 4,
        };
        assert!(report.validate_against(&expected).is_ok());
        report.request_events[0].first_token_offset_ns = 4;
        assert!(report.validate_against(&expected).is_err());
        report.request_events[0].first_token_offset_ns = 2;
        report.request_events[0].terminal_offset_ns = 3;
        report.request_events[0].output_tokens = 3;
        assert!(report.validate_against(&expected).is_err());
    }
}
