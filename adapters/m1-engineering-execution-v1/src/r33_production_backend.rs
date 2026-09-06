//! Fail-closed ownership join for a future authenticated R33 backend.
//!
//! Construction consumes an already-authenticated Worker V3 runner and an
//! already-initialized physical model/KV pool. This module never obtains those
//! capabilities from engineering artifacts. `start` creates and binds the real
//! Ferric Engine, while `stop` deterministically drops unpublished ownership.
//!
//! The opt-in S1/T128 constructor can join one exact canonical R33 row through
//! authenticated paired-prefill prepublication. The default constructor and
//! every unsupported row remain fail-closed. Queue execution, completion
//! observation, timing, and measurement reports are still unavailable.

use core::fmt;

use ferric_engine::{
    prepare_m1_authenticated_s1_t128_prefill_prepublication_v1, Engine,
    M1AuthenticatedPhysicalRunnerV1, M1AuthenticatedS1T128PrefillBootstrapFailureV1,
    M1AuthenticatedS1T128PrefillBootstrapInputV1,
    M1AuthenticatedS1T128PrefillPrepublicationV1, M1PartitionedModelMemoryKvPoolV1,
};
use ferric_spec::{M1_MAX_ACTIVE_SEQUENCES, M1_MAX_CONTEXT_TOKENS, M1_MAX_KV_PAGE_TOKENS};

use crate::r33_service::{
    M1R33AuthorityFreeBackendV1, M1R33BackendFaultV1, M1R33OperationDeadlineV1,
    M1R33WorkloadDocumentV1, M1R33WorkloadWindowV1,
};
use crate::r33_wire::M1R33MeasurementReportV1;

const M1_R33_ENGINE_CAPACITY_V1: usize = M1_MAX_ACTIVE_SEQUENCES as usize;
const M1_R33_ENGINE_PAGE_COUNT_V1: u32 = 512;
const M1_R33_ENGINE_PAGE_TOKENS_V1: u32 = M1_MAX_KV_PAGE_TOKENS;

type M1R33EngineV1 = Engine<M1_R33_ENGINE_CAPACITY_V1>;

const FAULT_DEADLINE_EXPIRED: &str = "backend-deadline-expired";
const FAULT_ENGINE_CONSTRUCTION: &str = "engine-construction-failed";
const FAULT_IDENTITY: &str = "backend-instance-mismatch";
const FAULT_MISSING_BOOTSTRAP: &str = "authenticated-window-bootstrap-unavailable";
const FAULT_BOOTSTRAP_BINDING: &str = "authenticated-window-bootstrap-binding-rejected";
const FAULT_BOOTSTRAP_REJECTED: &str = "authenticated-window-bootstrap-rejected";
const FAULT_EXECUTION_UNAVAILABLE: &str = "authenticated-window-execution-unavailable";
const FAULT_NOT_ACTIVE: &str = "backend-not-active";
const FAULT_STOPPED: &str = "backend-stopped";
const FAULT_UNHEALTHY_ENGINE: &str = "backend-engine-not-ready";
const FAULT_WORKLOAD: &str = "backend-workload-rejected";
const FAULT_WINDOW: &str = "backend-window-binding-rejected";

/// Observable phase of the authenticated R33 ownership join.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33AuthenticatedProductionBackendPhaseV1 {
    /// Authenticated runner and initialized model/KV ownership are unpublished.
    Dormant,
    /// One exact instance owns a fresh Ferric Engine and unpublished resources.
    Active,
    /// The exact instance is terminal except for an exact stop.
    Faulted,
    /// Unpublished physical ownership and Engine state have been dropped.
    Stopped,
}

#[derive(Debug)]
struct InstanceBindingV1 {
    instance_sha256: Box<str>,
    server_start: u64,
}

impl InstanceBindingV1 {
    fn new(instance_sha256: &str, server_start: u64) -> Self {
        Self {
            instance_sha256: instance_sha256.into(),
            server_start,
        }
    }

    fn matches(&self, instance_sha256: &str) -> bool {
        self.instance_sha256.as_ref() == instance_sha256
    }
}

struct ActiveCustodyV1<R, M, E> {
    runner: R,
    model_memory: M,
    engine: E,
}

enum FaultedCustodyV1<R, M, E> {
    Prepublication { _runner: R, _model_memory: M },
    Active(ActiveCustodyV1<R, M, E>),
}

enum BackendStateV1<R, M, E> {
    Dormant {
        runner: R,
        model_memory: M,
    },
    Active {
        binding: InstanceBindingV1,
        custody: ActiveCustodyV1<R, M, E>,
    },
    Faulted {
        binding: InstanceBindingV1,
        custody: FaultedCustodyV1<R, M, E>,
    },
    Stopped {
        binding: InstanceBindingV1,
    },
}

impl<R, M, E> BackendStateV1<R, M, E> {
    const fn phase(&self) -> M1R33AuthenticatedProductionBackendPhaseV1 {
        match self {
            Self::Dormant { .. } => M1R33AuthenticatedProductionBackendPhaseV1::Dormant,
            Self::Active { .. } => M1R33AuthenticatedProductionBackendPhaseV1::Active,
            Self::Faulted { .. } => M1R33AuthenticatedProductionBackendPhaseV1::Faulted,
            Self::Stopped { .. } => M1R33AuthenticatedProductionBackendPhaseV1::Stopped,
        }
    }

    const fn binding(&self) -> Option<&InstanceBindingV1> {
        match self {
            Self::Dormant { .. } => None,
            Self::Active { binding, .. }
            | Self::Faulted { binding, .. }
            | Self::Stopped { binding } => Some(binding),
        }
    }
}

struct BackendStateMachineV1<R, M, E> {
    state: Option<BackendStateV1<R, M, E>>,
}

impl<R, M, E> BackendStateMachineV1<R, M, E> {
    const fn new(runner: R, model_memory: M) -> Self {
        Self {
            state: Some(BackendStateV1::Dormant {
                runner,
                model_memory,
            }),
        }
    }

    fn state(&self) -> &BackendStateV1<R, M, E> {
        self.state
            .as_ref()
            .expect("R33 backend state is restored before every return")
    }

    fn phase(&self) -> M1R33AuthenticatedProductionBackendPhaseV1 {
        self.state().phase()
    }

    fn bound_instance_sha256(&self) -> Option<&str> {
        self.state()
            .binding()
            .map(|binding| binding.instance_sha256.as_ref())
    }

    fn start<BuildError>(
        &mut self,
        instance_sha256: &str,
        server_start: u64,
        workload_valid: bool,
        deadline_expired: bool,
        build_engine: impl FnOnce() -> Result<E, BuildError>,
    ) -> Result<(), M1R33BackendFaultV1> {
        if !valid_sha256(instance_sha256) {
            return Err(fault(FAULT_IDENTITY));
        }
        match self.state() {
            BackendStateV1::Dormant { .. } => {}
            BackendStateV1::Stopped { .. } => return Err(fault(FAULT_STOPPED)),
            BackendStateV1::Active { .. } | BackendStateV1::Faulted { .. } => {
                return Err(fault(FAULT_NOT_ACTIVE));
            }
        }
        let Some(BackendStateV1::Dormant {
            runner,
            model_memory,
        }) = self.state.take()
        else {
            return Err(fault(FAULT_NOT_ACTIVE));
        };
        let binding = InstanceBindingV1::new(instance_sha256, server_start);
        if deadline_expired || !workload_valid {
            self.state = Some(BackendStateV1::Faulted {
                binding,
                custody: FaultedCustodyV1::Prepublication {
                    _runner: runner,
                    _model_memory: model_memory,
                },
            });
            return Err(fault(if deadline_expired {
                FAULT_DEADLINE_EXPIRED
            } else {
                FAULT_WORKLOAD
            }));
        }
        if let Ok(engine) = build_engine() {
            self.state = Some(BackendStateV1::Active {
                binding,
                custody: ActiveCustodyV1 {
                    runner,
                    model_memory,
                    engine,
                },
            });
            Ok(())
        } else {
            self.state = Some(BackendStateV1::Faulted {
                binding,
                custody: FaultedCustodyV1::Prepublication {
                    _runner: runner,
                    _model_memory: model_memory,
                },
            });
            Err(fault(FAULT_ENGINE_CONSTRUCTION))
        }
    }

    fn ready(
        &mut self,
        instance_sha256: &str,
        deadline_expired: bool,
        healthy: impl FnOnce(&E) -> bool,
    ) -> Result<(), M1R33BackendFaultV1> {
        if deadline_expired {
            return Err(fault(FAULT_DEADLINE_EXPIRED));
        }
        let BackendStateV1::Active { binding, custody } = self.state() else {
            return Err(fault_for_inactive(self.state()));
        };
        if !binding.matches(instance_sha256) {
            return Err(fault(FAULT_IDENTITY));
        }
        if healthy(&custody.engine) {
            return Ok(());
        }
        let Some(BackendStateV1::Active { binding, custody }) = self.state.take() else {
            return Err(fault(FAULT_NOT_ACTIVE));
        };
        self.state = Some(BackendStateV1::Faulted {
            binding,
            custody: FaultedCustodyV1::Active(custody),
        });
        Err(fault(FAULT_UNHEALTHY_ENGINE))
    }

    fn reject_measure(
        &mut self,
        instance_sha256: &str,
        server_start: u64,
        deadline_expired: bool,
    ) -> Result<(), M1R33BackendFaultV1> {
        self.reject_measure_with(
            instance_sha256,
            server_start,
            deadline_expired,
            FAULT_MISSING_BOOTSTRAP,
        )
    }

    fn reject_measure_with(
        &mut self,
        instance_sha256: &str,
        server_start: u64,
        deadline_expired: bool,
        code: &'static str,
    ) -> Result<(), M1R33BackendFaultV1> {
        if deadline_expired {
            return Err(fault(FAULT_DEADLINE_EXPIRED));
        }
        let BackendStateV1::Active { binding, .. } = self.state() else {
            return Err(fault_for_inactive(self.state()));
        };
        if !binding.matches(instance_sha256) || binding.server_start != server_start {
            return Err(fault(FAULT_IDENTITY));
        }
        let Some(BackendStateV1::Active { binding, custody }) = self.state.take() else {
            return Err(fault(FAULT_NOT_ACTIVE));
        };
        self.state = Some(BackendStateV1::Faulted {
            binding,
            custody: FaultedCustodyV1::Active(custody),
        });
        Err(fault(code))
    }

    fn stop(
        &mut self,
        instance_sha256: &str,
        deadline_expired: bool,
    ) -> Result<(), M1R33BackendFaultV1> {
        if deadline_expired {
            return Err(fault(FAULT_DEADLINE_EXPIRED));
        }
        let Some(binding) = self.state().binding() else {
            return Err(fault(FAULT_NOT_ACTIVE));
        };
        if !binding.matches(instance_sha256) {
            return Err(fault(FAULT_IDENTITY));
        }
        if matches!(self.state(), BackendStateV1::Stopped { .. }) {
            return Ok(());
        }
        let Some(state) = self.state.take() else {
            return Err(fault(FAULT_NOT_ACTIVE));
        };
        let (binding, custody) = match state {
            BackendStateV1::Active { binding, custody } => {
                (binding, FaultedCustodyV1::Active(custody))
            }
            BackendStateV1::Faulted { binding, custody } => (binding, custody),
            BackendStateV1::Dormant {
                runner,
                model_memory,
            } => {
                self.state = Some(BackendStateV1::Dormant {
                    runner,
                    model_memory,
                });
                return Err(fault(FAULT_NOT_ACTIVE));
            }
            BackendStateV1::Stopped { binding } => {
                self.state = Some(BackendStateV1::Stopped { binding });
                return Ok(());
            }
        };
        drop(custody);
        self.state = Some(BackendStateV1::Stopped { binding });
        Ok(())
    }
}

fn fault_for_inactive<R, M, E>(state: &BackendStateV1<R, M, E>) -> M1R33BackendFaultV1 {
    match state {
        BackendStateV1::Stopped { .. } => fault(FAULT_STOPPED),
        BackendStateV1::Dormant { .. }
        | BackendStateV1::Active { .. }
        | BackendStateV1::Faulted { .. } => fault(FAULT_NOT_ACTIVE),
    }
}

const fn fault(code: &'static str) -> M1R33BackendFaultV1 {
    M1R33BackendFaultV1::new(code)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Stable rejection while binding one canonical R33 row to S1/T128 inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33AuthenticatedS1T128BootstrapBindingErrorV1 {
    InvalidWindow,
    UnsupportedRoster,
    PretokenizedRoster,
    OutputPolicy,
}

/// Binding rejection retaining the exact authenticated bootstrap input.
#[must_use = "rejected authenticated bootstrap input remains linearly owned"]
#[derive(Debug)]
pub struct M1R33AuthenticatedS1T128BootstrapBindingFailureV1 {
    error: M1R33AuthenticatedS1T128BootstrapBindingErrorV1,
    input: M1AuthenticatedS1T128PrefillBootstrapInputV1,
}

impl M1R33AuthenticatedS1T128BootstrapBindingFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1R33AuthenticatedS1T128BootstrapBindingErrorV1 {
        self.error
    }

    #[must_use = "the rejected authenticated bootstrap input remains linear"]
    pub fn into_input(self) -> M1AuthenticatedS1T128PrefillBootstrapInputV1 {
        self.input
    }
}

/// One exact canonical R33 row joined to authenticated S1/T128 bootstrap inputs.
///
/// Construction copies only immutable row identity. The move-only engine input
/// remains the sole workspace-plan owner and is rechecked against the borrowed
/// canonical row immediately before Engine admission.
#[must_use = "R33 row identity and authenticated bootstrap input remain joined"]
#[derive(Debug)]
pub struct M1R33AuthenticatedS1T128BootstrapBindingV1 {
    row_id: Box<str>,
    row_ordinal: u64,
    server_start: u64,
    window: u64,
    input: M1AuthenticatedS1T128PrefillBootstrapInputV1,
}

impl M1R33AuthenticatedS1T128BootstrapBindingV1 {
    /// Binds exactly one 128-token R33 request and its successor output limit.
    ///
    /// R33 counts the direct paired-prefill token in `expected_output_tokens`;
    /// the engine input counts only later speculative publications. Therefore
    /// this join requires `expected_output_tokens == successor_limit + 1`.
    ///
    /// # Errors
    ///
    /// Rejects an invalid window, any roster other than one ordinal-zero
    /// request, a prompt mismatch, or an output-limit mismatch. The returned
    /// failure retains the unchanged move-only bootstrap input.
    pub fn bind(
        window: &M1R33WorkloadWindowV1,
        input: M1AuthenticatedS1T128PrefillBootstrapInputV1,
    ) -> Result<Self, M1R33AuthenticatedS1T128BootstrapBindingFailureV1> {
        let reject = |error, input| M1R33AuthenticatedS1T128BootstrapBindingFailureV1 {
            error,
            input,
        };
        if window.validate().is_err() {
            return Err(reject(
                M1R33AuthenticatedS1T128BootstrapBindingErrorV1::InvalidWindow,
                input,
            ));
        }
        let Some(request) = window.requests.first().filter(|_| window.requests.len() == 1) else {
            return Err(reject(
                M1R33AuthenticatedS1T128BootstrapBindingErrorV1::UnsupportedRoster,
                input,
            ));
        };
        if request.request_ordinal != 0 || request.prompt_tokens != input.prompt_tokens() {
            return Err(reject(
                M1R33AuthenticatedS1T128BootstrapBindingErrorV1::PretokenizedRoster,
                input,
            ));
        }
        if u64::from(input.maximum_successor_output_tokens())
            .checked_add(1)
            != Some(request.expected_output_tokens)
        {
            return Err(reject(
                M1R33AuthenticatedS1T128BootstrapBindingErrorV1::OutputPolicy,
                input,
            ));
        }
        Ok(Self {
            row_id: window.row.id.clone().into_boxed_str(),
            row_ordinal: window.row.ordinal,
            server_start: window.row.server_start,
            window: window.row.window,
            input,
        })
    }

    fn matches(&self, window: &M1R33WorkloadWindowV1) -> bool {
        let Some(request) = window.requests.first().filter(|_| window.requests.len() == 1) else {
            return false;
        };
        window.validate().is_ok()
            && window.row.id == self.row_id.as_ref()
            && window.row.ordinal == self.row_ordinal
            && window.row.server_start == self.server_start
            && window.row.window == self.window
            && request.request_ordinal == 0
            && request.prompt_tokens == self.input.prompt_tokens()
            && u64::from(self.input.maximum_successor_output_tokens())
                .checked_add(1)
                == Some(request.expected_output_tokens)
    }
}

enum M1R33AuthenticatedRunnerCustodyV1 {
    MissingBootstrap {
        _runner: M1AuthenticatedPhysicalRunnerV1,
    },
    Pending {
        runner: M1AuthenticatedPhysicalRunnerV1,
        bootstrap: M1R33AuthenticatedS1T128BootstrapBindingV1,
    },
    Prepared {
        _custody: Box<
            M1AuthenticatedS1T128PrefillPrepublicationV1<M1_R33_ENGINE_CAPACITY_V1>,
        >,
    },
    Rejected {
        _custody:
            Box<M1AuthenticatedS1T128PrefillBootstrapFailureV1<M1_R33_ENGINE_CAPACITY_V1>>,
    },
}

enum M1R33AuthenticatedMemoryCustodyV1 {
    Initialized(Box<M1PartitionedModelMemoryKvPoolV1>),
    Joined,
}

enum M1R33EngineCustodyV1 {
    Fresh(Box<M1R33EngineV1>),
    Joined,
}

/// Authenticated, initialized ownership for one fail-closed R33 instance.
///
/// This is not a serving-complete backend. The constructor requires production
/// capabilities that can only be obtained through Ferric's authenticated
/// Worker V3 and checked-device model-memory paths. No artifact path, verifier,
/// structural runner, raw KFD handle, or report callback is accepted here.
#[must_use = "authenticated runner and initialized model memory remain linearly owned"]
pub struct M1R33AuthenticatedProductionBackendV1 {
    state: BackendStateMachineV1<
        M1R33AuthenticatedRunnerCustodyV1,
        M1R33AuthenticatedMemoryCustodyV1,
        M1R33EngineCustodyV1,
    >,
}

impl fmt::Debug for M1R33AuthenticatedProductionBackendV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33AuthenticatedProductionBackendV1")
            .field("phase", &self.phase())
            .field("bound_instance_sha256", &self.bound_instance_sha256())
            .finish_non_exhaustive()
    }
}

impl M1R33AuthenticatedProductionBackendV1 {
    /// Consumes already-authenticated program and initialized physical-memory owners.
    pub fn new(
        runner: M1AuthenticatedPhysicalRunnerV1,
        model_memory: M1PartitionedModelMemoryKvPoolV1,
    ) -> Self {
        Self {
            state: BackendStateMachineV1::new(
                M1R33AuthenticatedRunnerCustodyV1::MissingBootstrap { _runner: runner },
                M1R33AuthenticatedMemoryCustodyV1::Initialized(Box::new(model_memory)),
            ),
        }
    }

    /// Consumes authenticated physical owners and one exact canonical first-window input.
    ///
    /// This enables only paired-prefill prepublication. Queue execution,
    /// completion observation, publication accounting, timing, and reports
    /// remain unavailable and fail closed.
    pub fn new_with_s1_t128_prefill_bootstrap(
        runner: M1AuthenticatedPhysicalRunnerV1,
        model_memory: M1PartitionedModelMemoryKvPoolV1,
        bootstrap: M1R33AuthenticatedS1T128BootstrapBindingV1,
    ) -> Self {
        Self {
            state: BackendStateMachineV1::new(
                M1R33AuthenticatedRunnerCustodyV1::Pending { runner, bootstrap },
                M1R33AuthenticatedMemoryCustodyV1::Initialized(Box::new(model_memory)),
            ),
        }
    }

    /// Current explicit ownership phase.
    #[must_use]
    pub fn phase(&self) -> M1R33AuthenticatedProductionBackendPhaseV1 {
        self.state.phase()
    }

    /// Exact instance identity after a successful or attempted bound start.
    #[must_use]
    pub fn bound_instance_sha256(&self) -> Option<&str> {
        self.state.bound_instance_sha256()
    }
}

impl crate::r33_service::sealed::Sealed for M1R33AuthenticatedProductionBackendV1 {}

impl M1R33AuthorityFreeBackendV1 for M1R33AuthenticatedProductionBackendV1 {
    fn start(
        &mut self,
        instance_sha256: &str,
        server_start: u64,
        workload: &M1R33WorkloadDocumentV1,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<(), M1R33BackendFaultV1> {
        self.state.start(
            instance_sha256,
            server_start,
            workload.validate().is_ok(),
            deadline.expired(),
            || {
                M1R33EngineV1::new(
                    M1_R33_ENGINE_PAGE_COUNT_V1,
                    M1_R33_ENGINE_PAGE_TOKENS_V1,
                    M1_MAX_CONTEXT_TOKENS,
                )
                .map(|engine| M1R33EngineCustodyV1::Fresh(Box::new(engine)))
            },
        )
    }

    fn ready(
        &mut self,
        instance_sha256: &str,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<(), M1R33BackendFaultV1> {
        self.state
            .ready(instance_sha256, deadline.expired(), |engine| {
                matches!(
                    engine,
                    M1R33EngineCustodyV1::Fresh(engine)
                        if !engine.is_faulted() && engine.live_count() == 0
                )
            })
    }

    fn measure(
        &mut self,
        instance_sha256: &str,
        window: &M1R33WorkloadWindowV1,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<M1R33MeasurementReportV1, M1R33BackendFaultV1> {
        if window.validate().is_err() {
            return Err(fault(FAULT_WINDOW));
        }
        if deadline.expired() {
            return Err(fault(FAULT_DEADLINE_EXPIRED));
        }
        let BackendStateV1::Active { binding, custody } = self.state.state() else {
            return Err(fault_for_inactive(self.state.state()));
        };
        if !binding.matches(instance_sha256) || binding.server_start != window.row.server_start {
            return Err(fault(FAULT_IDENTITY));
        }
        match &custody.runner {
            M1R33AuthenticatedRunnerCustodyV1::MissingBootstrap { .. } => {
                let result = self
                    .state
                    .reject_measure(instance_sha256, window.row.server_start, false);
                return match result {
                    Err(error) => Err(error),
                    Ok(()) => Err(fault(FAULT_NOT_ACTIVE)),
                };
            }
            M1R33AuthenticatedRunnerCustodyV1::Pending { bootstrap, .. }
                if !bootstrap.matches(window) =>
            {
                let result = self
                    .state
                    .reject_measure_with(
                        instance_sha256,
                        window.row.server_start,
                        false,
                        FAULT_BOOTSTRAP_BINDING,
                    );
                return match result {
                    Err(error) => Err(error),
                    Ok(()) => Err(fault(FAULT_NOT_ACTIVE)),
                };
            }
            M1R33AuthenticatedRunnerCustodyV1::Pending { .. } => {}
            M1R33AuthenticatedRunnerCustodyV1::Prepared { .. }
            | M1R33AuthenticatedRunnerCustodyV1::Rejected { .. } => {
                return Err(fault(FAULT_NOT_ACTIVE));
            }
        }
        let Some(BackendStateV1::Active { binding, custody }) = self.state.state.take() else {
            return Err(fault(FAULT_NOT_ACTIVE));
        };
        let ActiveCustodyV1 {
            runner,
            model_memory,
            engine,
        } = custody;
        let (runner, bootstrap, memory, engine) = match (runner, model_memory, engine) {
            (
                M1R33AuthenticatedRunnerCustodyV1::Pending { runner, bootstrap },
                M1R33AuthenticatedMemoryCustodyV1::Initialized(memory),
                M1R33EngineCustodyV1::Fresh(engine),
            ) => (runner, bootstrap, *memory, *engine),
            (runner, memory, engine) => {
                self.state.state = Some(BackendStateV1::Faulted {
                    binding,
                    custody: FaultedCustodyV1::Active(ActiveCustodyV1 {
                        runner,
                        model_memory: memory,
                        engine,
                    }),
                });
                return Err(fault(FAULT_BOOTSTRAP_REJECTED));
            }
        };
        let M1R33AuthenticatedS1T128BootstrapBindingV1 { input, .. } = bootstrap;
        let result = prepare_m1_authenticated_s1_t128_prefill_prepublication_v1(
            engine, runner, memory, input,
        );
        let (runner, code) = match result {
            Ok(prepared) => (
                M1R33AuthenticatedRunnerCustodyV1::Prepared {
                    _custody: Box::new(prepared),
                },
                FAULT_EXECUTION_UNAVAILABLE,
            ),
            Err(rejected) => (
                M1R33AuthenticatedRunnerCustodyV1::Rejected {
                    _custody: rejected,
                },
                FAULT_BOOTSTRAP_REJECTED,
            ),
        };
        self.state.state = Some(BackendStateV1::Faulted {
            binding,
            custody: FaultedCustodyV1::Active(ActiveCustodyV1 {
                runner,
                model_memory: M1R33AuthenticatedMemoryCustodyV1::Joined,
                engine: M1R33EngineCustodyV1::Joined,
            }),
        });
        Err(fault(code))
    }

    fn stop(
        &mut self,
        instance_sha256: &str,
        deadline: M1R33OperationDeadlineV1,
    ) -> Result<(), M1R33BackendFaultV1> {
        self.state.stop(instance_sha256, deadline.expired())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r33_wire::{M1R33CollectorRowV1, M1R33WorkV1, M1R33WorkloadRequestV1};
    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
        DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration,
        M1StepWorkspacePlanOutcome,
    };
    use ferric_engine::M1FullStepWorkspacePlans;
    use ferric_spec::{Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};
    use std::cell::Cell;
    use std::rc::Rc;

    const INSTANCE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const WRONG_INSTANCE: &str = "1123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    const TARGET_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        mode: Qwen3ExecutionMode::Prefill,
        bucket: Qwen3PlanBucket::PrefillS1T128,
    };
    const DRAFT_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Prefill,
        bucket: Qwen3PlanBucket::PrefillS1T128,
    };

    #[derive(Debug)]
    struct DropWitness(Rc<Cell<usize>>);

    impl Drop for DropWitness {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    fn backend() -> (
        BackendStateMachineV1<DropWitness, DropWitness, usize>,
        Rc<Cell<usize>>,
    ) {
        let drops = Rc::new(Cell::new(0));
        (
            BackendStateMachineV1::new(
                DropWitness(Rc::clone(&drops)),
                DropWitness(Rc::clone(&drops)),
            ),
            drops,
        )
    }

    fn workspace_plan(
        selection: Qwen3PlanSelection,
        identity_byte: u8,
    ) -> AddresslessM1StepWorkspacePlan {
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                Identity::new([identity_byte; 32]),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        ));
        match plan_addressless_m1_step_workspace(selection, available) {
            M1StepWorkspacePlanOutcome::Planned(plan) => plan,
            M1StepWorkspacePlanOutcome::Rejected(_) => panic!("test workspace rejected"),
        }
    }

    fn bootstrap_input(
        prompt: Vec<u32>,
        successor_output: u32,
    ) -> M1AuthenticatedS1T128PrefillBootstrapInputV1 {
        let plans = || {
            M1FullStepWorkspacePlans::paired_prefill(
                workspace_plan(DRAFT_PREFILL, 1),
                workspace_plan(TARGET_PREFILL, 2),
            )
        };
        M1AuthenticatedS1T128PrefillBootstrapInputV1::new(
            prompt,
            successor_output,
            plans(),
            plans(),
        )
        .unwrap()
    }

    fn r33_window(prompt: Vec<u32>, expected_output: u64) -> M1R33WorkloadWindowV1 {
        let input_tokens = prompt.len() as u64;
        M1R33WorkloadWindowV1 {
            requests: vec![M1R33WorkloadRequestV1 {
                expected_output_tokens: expected_output,
                prompt_tokens: prompt,
                request_ordinal: 0,
            }],
            row: M1R33CollectorRowV1 {
                expected_work: M1R33WorkV1 {
                    input_tokens,
                    output_tokens: expected_output,
                    successful_requests: 1,
                    total_tokens: input_tokens + expected_output,
                },
                id: "start-0.warmup-00".to_owned(),
                ordinal: 0,
                phase: "warmup".to_owned(),
                server_start: 0,
                window: 0,
            },
        }
    }

    #[test]
    fn bootstrap_binding_pins_exact_row_prompt_and_output_identity() {
        let window = r33_window(vec![1; 128], 33);
        let binding = M1R33AuthenticatedS1T128BootstrapBindingV1::bind(
            &window,
            bootstrap_input(vec![1; 128], 32),
        )
        .unwrap();
        assert!(binding.matches(&window));

        let mut wrong_row = window.clone();
        wrong_row.row.id.push_str("-other");
        assert!(!binding.matches(&wrong_row));
        let mut wrong_prompt = window.clone();
        wrong_prompt.requests[0].prompt_tokens[127] = 2;
        assert!(!binding.matches(&wrong_prompt));
        let mut wrong_output = window.clone();
        wrong_output.requests[0].expected_output_tokens = 34;
        wrong_output.row.expected_work.output_tokens = 34;
        wrong_output.row.expected_work.total_tokens = 162;
        assert!(!binding.matches(&wrong_output));
    }

    #[test]
    fn bootstrap_binding_rejections_retain_move_only_input() {
        let window = r33_window(vec![1; 128], 33);
        let failure = M1R33AuthenticatedS1T128BootstrapBindingV1::bind(
            &window,
            bootstrap_input(vec![2; 128], 32),
        )
        .unwrap_err();
        assert_eq!(
            failure.error(),
            M1R33AuthenticatedS1T128BootstrapBindingErrorV1::PretokenizedRoster
        );
        assert_eq!(failure.into_input().prompt_tokens(), [2; 128]);

        let failure = M1R33AuthenticatedS1T128BootstrapBindingV1::bind(
            &window,
            bootstrap_input(vec![1; 128], 31),
        )
        .unwrap_err();
        assert_eq!(
            failure.error(),
            M1R33AuthenticatedS1T128BootstrapBindingErrorV1::OutputPolicy
        );
        assert_eq!(failure.into_input().maximum_successor_output_tokens(), 31);
    }

    #[test]
    fn exact_start_binds_real_state_shape_once() {
        let (mut backend, drops) = backend();
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Dormant
        );
        backend
            .start(INSTANCE, 7, true, false, || Ok::<_, ()>(11))
            .unwrap();
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Active
        );
        assert_eq!(backend.bound_instance_sha256(), Some(INSTANCE));
        assert_eq!(drops.get(), 0);
        assert_eq!(
            backend
                .start(INSTANCE, 7, true, false, || Ok::<_, ()>(12))
                .unwrap_err()
                .code(),
            FAULT_NOT_ACTIVE
        );
    }

    #[test]
    fn malformed_direct_start_does_not_mutate_or_drop_owners() {
        let (mut backend, drops) = backend();
        assert_eq!(
            backend
                .start("ABC", 0, true, false, || Ok::<_, ()>(1))
                .unwrap_err()
                .code(),
            FAULT_IDENTITY
        );
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Dormant
        );
        assert_eq!(drops.get(), 0);
    }

    #[test]
    fn failed_valid_start_binds_faulted_instance_for_exact_stop() {
        for (workload_valid, deadline_expired, expected) in [
            (true, true, FAULT_DEADLINE_EXPIRED),
            (false, false, FAULT_WORKLOAD),
        ] {
            let (mut backend, drops) = backend();
            assert_eq!(
                backend
                    .start(INSTANCE, 0, workload_valid, deadline_expired, || {
                        Ok::<_, ()>(1)
                    })
                    .unwrap_err()
                    .code(),
                expected
            );
            assert_eq!(
                backend.phase(),
                M1R33AuthenticatedProductionBackendPhaseV1::Faulted
            );
            assert_eq!(backend.bound_instance_sha256(), Some(INSTANCE));
            assert_eq!(drops.get(), 0);
            backend.stop(INSTANCE, false).unwrap();
            assert_eq!(drops.get(), 2);
        }
    }

    #[test]
    fn active_identity_and_deadline_rejections_do_not_drop_owners() {
        let (mut backend, drops) = backend();
        backend
            .start(INSTANCE, 0, true, false, || Ok::<_, ()>(1))
            .unwrap();
        for fault in [
            backend.ready(WRONG_INSTANCE, false, |_| true).unwrap_err(),
            backend.ready(INSTANCE, true, |_| true).unwrap_err(),
        ] {
            assert!(matches!(
                fault.code(),
                FAULT_IDENTITY | FAULT_DEADLINE_EXPIRED
            ));
        }
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Active
        );
        assert_eq!(drops.get(), 0);
    }

    #[test]
    fn missing_bootstrap_faults_without_report_or_owner_loss() {
        let (mut backend, drops) = backend();
        backend
            .start(INSTANCE, 3, true, false, || Ok::<_, ()>(1))
            .unwrap();
        assert_eq!(
            backend
                .reject_measure(WRONG_INSTANCE, 3, false)
                .unwrap_err()
                .code(),
            FAULT_IDENTITY
        );
        assert_eq!(
            backend
                .reject_measure(INSTANCE, 4, false)
                .unwrap_err()
                .code(),
            FAULT_IDENTITY
        );
        assert_eq!(
            backend
                .reject_measure(INSTANCE, 3, true)
                .unwrap_err()
                .code(),
            FAULT_DEADLINE_EXPIRED
        );
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Active
        );
        assert_eq!(
            backend
                .reject_measure(INSTANCE, 3, false)
                .unwrap_err()
                .code(),
            FAULT_MISSING_BOOTSTRAP
        );
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Faulted
        );
        assert_eq!(drops.get(), 0);
        assert_eq!(
            backend
                .reject_measure(INSTANCE, 3, false)
                .unwrap_err()
                .code(),
            FAULT_NOT_ACTIVE
        );
    }

    #[test]
    fn unhealthy_ready_faults_and_only_exact_stop_releases_custody() {
        let (mut backend, drops) = backend();
        backend
            .start(INSTANCE, 1, true, false, || Ok::<_, ()>(1))
            .unwrap();
        assert_eq!(
            backend
                .ready(INSTANCE, false, |_| false)
                .unwrap_err()
                .code(),
            FAULT_UNHEALTHY_ENGINE
        );
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Faulted
        );
        assert_eq!(
            backend.stop(WRONG_INSTANCE, false).unwrap_err().code(),
            FAULT_IDENTITY
        );
        assert_eq!(
            backend.stop(INSTANCE, true).unwrap_err().code(),
            FAULT_DEADLINE_EXPIRED
        );
        assert_eq!(drops.get(), 0);
        backend.stop(INSTANCE, false).unwrap();
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Stopped
        );
        assert_eq!(drops.get(), 2);
        backend.stop(INSTANCE, false).unwrap();
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn engine_construction_failure_retains_custody_for_exact_stop() {
        let (mut backend, drops) = backend();
        assert_eq!(
            backend
                .start(INSTANCE, 5, true, false, || Err::<usize, _>(()))
                .unwrap_err()
                .code(),
            FAULT_ENGINE_CONSTRUCTION
        );
        assert_eq!(
            backend.phase(),
            M1R33AuthenticatedProductionBackendPhaseV1::Faulted
        );
        assert_eq!(backend.bound_instance_sha256(), Some(INSTANCE));
        assert_eq!(drops.get(), 0);
        backend.stop(INSTANCE, false).unwrap();
        assert_eq!(drops.get(), 2);
    }
}
