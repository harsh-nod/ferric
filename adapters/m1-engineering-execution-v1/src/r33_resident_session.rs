//! Private custody vault and single-binding shell for a resident R33 session.
//!
//! This is a non-authoritative prerequisite. It does not wire physical
//! execution. The shell consumes and retains the held canonical service bundle,
//! admits exactly one input at a time, and never passes custody or input
//! ownership to executor-controlled code.

use core::fmt;

use ferric_spec::TokenId;

use crate::r33_service::{HeldM1R33ServiceBundleV1, M1R33WorkloadWindowV1};
use crate::r33_wire::M1_R33_WINDOWS_PER_START_V1;

mod vault;

/// Exact number of sequential windows held by one R33 server start.
pub(crate) const M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1: usize = M1_R33_WINDOWS_PER_START_V1;

/// Observable phase of the private custody shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum M1R33ResidentSessionPhaseV1 {
    Ready,
    Outstanding,
    Exhausted,
    Faulted,
    Stopped,
}

/// Stable admission or transition rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum M1R33ResidentSessionErrorV1 {
    InvalidServerStart,
    HeldBundleChanged,
    InvalidWorkload,
    UnsupportedRoster,
    HostAllocation,
    WindowLimit,
    SessionNotReady,
    VaultInvariant,
    Terminal(&'static str),
}

/// Construction rejection retaining both move-only inputs.
#[must_use = "rejected held bundle and custody remain caller-owned"]
pub(crate) struct M1R33ResidentSessionAdmissionFailureV1<C> {
    error: M1R33ResidentSessionErrorV1,
    bundle: HeldM1R33ServiceBundleV1,
    custody: C,
}

impl<C> M1R33ResidentSessionAdmissionFailureV1<C> {
    #[must_use]
    pub(crate) const fn error(&self) -> M1R33ResidentSessionErrorV1 {
        self.error
    }

    #[must_use = "rejected held bundle and custody remain caller-owned"]
    pub(crate) fn into_parts(self) -> (HeldM1R33ServiceBundleV1, C) {
        (self.bundle, self.custody)
    }
}

impl<C> fmt::Debug for M1R33ResidentSessionAdmissionFailureV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33ResidentSessionAdmissionFailureV1")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

/// Rejected bind retaining the exact input value.
#[must_use = "rejected window input remains caller-owned"]
#[derive(Debug)]
pub(crate) struct M1R33ResidentBindFailureV1<I> {
    error: M1R33ResidentSessionErrorV1,
    input: I,
}

impl<I> M1R33ResidentBindFailureV1<I> {
    #[must_use]
    pub(crate) const fn error(&self) -> M1R33ResidentSessionErrorV1 {
        self.error
    }

    #[must_use = "rejected window input remains caller-owned"]
    pub(crate) fn into_input(self) -> I {
        self.input
    }
}

#[derive(Debug)]
struct M1R33ResidentWindowFactsV1 {
    row_id: Box<str>,
    row_ordinal: u64,
    prompt_tokens: Box<[TokenId]>,
    expected_output_tokens: u64,
}

fn exact_roster_v1(
    bundle: &HeldM1R33ServiceBundleV1,
    server_start: u64,
) -> Result<Box<[M1R33ResidentWindowFactsV1]>, M1R33ResidentSessionErrorV1> {
    bundle
        .revalidate()
        .map_err(|_| M1R33ResidentSessionErrorV1::HeldBundleChanged)?;
    bundle
        .workload()
        .validate()
        .map_err(|_| M1R33ResidentSessionErrorV1::InvalidWorkload)?;
    if server_start >= 3 {
        return Err(M1R33ResidentSessionErrorV1::InvalidServerStart);
    }
    let start = usize::try_from(server_start)
        .ok()
        .and_then(|value| value.checked_mul(M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1))
        .ok_or(M1R33ResidentSessionErrorV1::InvalidServerStart)?;
    let end = start
        .checked_add(M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1)
        .ok_or(M1R33ResidentSessionErrorV1::InvalidServerStart)?;
    let selected = bundle
        .workload()
        .rows
        .get(start..end)
        .ok_or(M1R33ResidentSessionErrorV1::InvalidWorkload)?;
    let expected_start = server_start
        .checked_mul(M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 as u64)
        .ok_or(M1R33ResidentSessionErrorV1::InvalidServerStart)?;
    let mut roster = Vec::new();
    roster
        .try_reserve_exact(M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1)
        .map_err(|_| M1R33ResidentSessionErrorV1::HostAllocation)?;
    for (sequence, window) in selected.iter().enumerate() {
        let Some(request) = exact_single_request_v1(window) else {
            return Err(M1R33ResidentSessionErrorV1::UnsupportedRoster);
        };
        if window.row.server_start != server_start
            || window.row.ordinal != expected_start + sequence as u64
        {
            return Err(M1R33ResidentSessionErrorV1::InvalidWorkload);
        }
        roster.push(M1R33ResidentWindowFactsV1 {
            row_id: window.row.id.clone().into_boxed_str(),
            row_ordinal: window.row.ordinal,
            prompt_tokens: request.prompt_tokens.clone().into_boxed_slice(),
            expected_output_tokens: request.expected_output_tokens,
        });
    }
    Ok(roster.into_boxed_slice())
}

fn exact_single_request_v1(
    window: &M1R33WorkloadWindowV1,
) -> Option<&crate::r33_wire::M1R33WorkloadRequestV1> {
    window
        .requests
        .first()
        .filter(|_| window.requests.len() == 1)
}

/// Non-authoritative disposition produced without receiving either owner.
enum M1R33ResidentExecutionDispositionV1<R> {
    Complete(R),
    Terminal(&'static str),
}

/// Private execution boundary. Implementations receive only an opaque,
/// non-owning capability whose API has no generic callbacks and exposes no
/// custody or input reference.
trait M1R33ResidentWindowExecutorV1<C, I> {
    type Report;

    fn execute(
        &mut self,
        capability: &mut vault::ExecutionCapability<'_, C, I>,
    ) -> M1R33ResidentExecutionDispositionV1<Self::Report>;
}

/// Resident holder of one held workload identity and private owner vault.
pub(crate) struct M1R33ResidentSessionV1<C, I> {
    bundle: HeldM1R33ServiceBundleV1,
    server_start: u64,
    completed: usize,
    roster: Box<[M1R33ResidentWindowFactsV1]>,
    state: M1R33ResidentSessionPhaseV1,
    vault: vault::Vault<C, I>,
}

impl<C, I> fmt::Debug for M1R33ResidentSessionV1<C, I> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33ResidentSessionV1")
            .field("service_plan_sha256", &self.bundle.plan_sha256())
            .field("server_start", &self.server_start)
            .field("completed", &self.completed)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<C, I> M1R33ResidentSessionV1<C, I> {
    /// Consumes the held canonical bundle and custody owner.
    pub(crate) fn new(
        bundle: HeldM1R33ServiceBundleV1,
        server_start: u64,
        custody: C,
    ) -> Result<Self, M1R33ResidentSessionAdmissionFailureV1<C>> {
        let mut vault = vault::Vault::new(custody);
        let roster = match exact_roster_v1(&bundle, server_start) {
            Ok(roster) => roster,
            Err(error) => {
                let custody = vault
                    .recover_custody()
                    .expect("new vault must retain exact construction custody");
                return Err(M1R33ResidentSessionAdmissionFailureV1 {
                    error,
                    bundle,
                    custody,
                });
            }
        };
        Ok(Self {
            bundle,
            server_start,
            completed: 0,
            roster,
            state: M1R33ResidentSessionPhaseV1::Ready,
            vault,
        })
    }

    #[must_use]
    pub(crate) const fn phase(&self) -> M1R33ResidentSessionPhaseV1 {
        self.state
    }

    #[must_use]
    pub(crate) const fn completed_windows(&self) -> usize {
        self.completed
    }

    #[must_use]
    pub(crate) fn service_plan_sha256(&self) -> &str {
        self.bundle.plan_sha256()
    }

    /// Installs exactly one input and returns an exclusive-borrow binding.
    pub(crate) fn bind_next(
        &mut self,
        input: I,
    ) -> Result<M1R33OutstandingWindowV1<'_, C, I>, M1R33ResidentBindFailureV1<I>> {
        let reject = |error, input| M1R33ResidentBindFailureV1 { error, input };
        if self.completed >= M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 {
            return Err(reject(M1R33ResidentSessionErrorV1::WindowLimit, input));
        }
        if self.state != M1R33ResidentSessionPhaseV1::Ready {
            return Err(reject(M1R33ResidentSessionErrorV1::SessionNotReady, input));
        }
        if let Err(input) = self.vault.install_input(input) {
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            return Err(reject(M1R33ResidentSessionErrorV1::VaultInvariant, input));
        }
        if self.bundle.revalidate().is_err() {
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            let input = self
                .vault
                .recover_input()
                .expect("accepted bind must retain its exact input");
            return Err(reject(
                M1R33ResidentSessionErrorV1::HeldBundleChanged,
                input,
            ));
        }
        self.state = M1R33ResidentSessionPhaseV1::Outstanding;
        Ok(M1R33OutstandingWindowV1 {
            session: self,
            active: true,
        })
    }

    fn execute_bound<E>(
        &mut self,
        executor: &mut E,
    ) -> Result<E::Report, M1R33ResidentSessionErrorV1>
    where
        E: M1R33ResidentWindowExecutorV1<C, I>,
    {
        if self.state != M1R33ResidentSessionPhaseV1::Outstanding {
            return Err(M1R33ResidentSessionErrorV1::SessionNotReady);
        }
        if self.bundle.revalidate().is_err() {
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            return Err(M1R33ResidentSessionErrorV1::HeldBundleChanged);
        }
        let Some(facts) = self.roster.get(self.completed) else {
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            return Err(M1R33ResidentSessionErrorV1::VaultInvariant);
        };
        let view = vault::WindowView {
            sequence: self.completed,
            row_ordinal: facts.row_ordinal,
            prompt_tokens: &facts.prompt_tokens,
            expected_output_tokens: facts.expected_output_tokens,
        };
        let mut abort_on_unwind = vault::AbortOnUnwind::armed();
        let Some(mut capability) = self.vault.execution_capability(view) else {
            abort_on_unwind.disarm();
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            return Err(M1R33ResidentSessionErrorV1::VaultInvariant);
        };
        let disposition = executor.execute(&mut capability);
        drop(capability);
        match disposition {
            M1R33ResidentExecutionDispositionV1::Complete(report) => {
                if !self.vault.complete_input() {
                    abort_on_unwind.disarm();
                    self.state = M1R33ResidentSessionPhaseV1::Faulted;
                    return Err(M1R33ResidentSessionErrorV1::VaultInvariant);
                }
                self.completed += 1;
                self.state = if self.completed == M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 {
                    M1R33ResidentSessionPhaseV1::Exhausted
                } else {
                    M1R33ResidentSessionPhaseV1::Ready
                };
                abort_on_unwind.disarm();
                Ok(report)
            }
            M1R33ResidentExecutionDispositionV1::Terminal(code) => {
                self.state = M1R33ResidentSessionPhaseV1::Faulted;
                abort_on_unwind.disarm();
                Err(M1R33ResidentSessionErrorV1::Terminal(code))
            }
        }
    }

    fn cancel_bound(&mut self) -> Result<I, M1R33ResidentSessionErrorV1> {
        if self.state != M1R33ResidentSessionPhaseV1::Outstanding {
            return Err(M1R33ResidentSessionErrorV1::SessionNotReady);
        }
        let Some(input) = self.vault.recover_input() else {
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            return Err(M1R33ResidentSessionErrorV1::VaultInvariant);
        };
        self.state = M1R33ResidentSessionPhaseV1::Ready;
        Ok(input)
    }

    fn abandon_bound(&mut self) {
        if self.state == M1R33ResidentSessionPhaseV1::Outstanding {
            self.vault.quarantine_input();
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
        }
    }

    /// Releases retained owners only after the held bundle still validates.
    pub(crate) fn stop(&mut self) -> Result<(), M1R33ResidentSessionErrorV1> {
        if self.state == M1R33ResidentSessionPhaseV1::Stopped {
            return Ok(());
        }
        if self.bundle.revalidate().is_err() {
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            return Err(M1R33ResidentSessionErrorV1::HeldBundleChanged);
        }
        if !self.vault.release_all() {
            self.state = M1R33ResidentSessionPhaseV1::Faulted;
            return Err(M1R33ResidentSessionErrorV1::VaultInvariant);
        }
        self.state = M1R33ResidentSessionPhaseV1::Stopped;
        Ok(())
    }
}

impl<C, I> Drop for M1R33ResidentSessionV1<C, I> {
    fn drop(&mut self) {
        if self.state != M1R33ResidentSessionPhaseV1::Stopped {
            // A direct drop is fail-closed quarantine, never implicit release.
            self.vault.quarantine_all();
        }
    }
}

/// Exclusive-borrow proof that this session has exactly one installed input.
#[must_use = "cancel, execute, or drop to quarantine the outstanding input"]
pub(crate) struct M1R33OutstandingWindowV1<'a, C, I> {
    session: &'a mut M1R33ResidentSessionV1<C, I>,
    active: bool,
}

impl<C, I> M1R33OutstandingWindowV1<'_, C, I> {
    #[must_use]
    pub(crate) fn row_id(&self) -> &str {
        self.session.roster[self.session.completed].row_id.as_ref()
    }

    #[must_use]
    pub(crate) const fn sequence(&self) -> usize {
        self.session.completed
    }

    #[must_use]
    pub(crate) fn prompt_tokens(&self) -> &[TokenId] {
        &self.session.roster[self.session.completed].prompt_tokens
    }

    #[must_use]
    pub(crate) fn expected_output_tokens(&self) -> u64 {
        self.session.roster[self.session.completed].expected_output_tokens
    }

    #[must_use = "cancel explicitly recovers the exact installed input"]
    pub(crate) fn cancel(mut self) -> Result<I, M1R33ResidentSessionErrorV1> {
        let result = self.session.cancel_bound();
        self.active = false;
        result
    }

    fn execute<E>(mut self, executor: &mut E) -> Result<E::Report, M1R33ResidentSessionErrorV1>
    where
        E: M1R33ResidentWindowExecutorV1<C, I>,
    {
        let result = self.session.execute_bound(executor);
        self.active = false;
        result
    }
}

impl<C, I> Drop for M1R33OutstandingWindowV1<'_, C, I> {
    fn drop(&mut self) {
        if self.active {
            self.session.abandon_bound();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::os::unix::process::ExitStatusExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use rustix::process::geteuid;

    use crate::r33_service::{
        HeldM1R33ServiceBundleV1, M1_R33_SERVICE_AUTHORITY_V1, M1_R33_SERVICE_PLAN_FORMAT_V1,
        M1_R33_WORKLOAD_FORMAT_V1, M1R33CommandIdentitiesV1, M1R33ServicePlanDocumentV1,
        M1R33WorkloadDocumentV1, M1R33WorkloadWindowV1,
    };
    use crate::r33_wire::{
        M1_R33_TARGET_V1, M1R33CollectorRowV1, M1R33SlotV1, M1R33WorkV1, M1R33WorkloadRequestV1,
        encode_canonical_json_v1, sha256_hex,
    };

    use super::*;

    static NEXT_TEST: AtomicU64 = AtomicU64::new(0);
    const SESSION_ABORT_ROOT_ENV_V1: &str = "FERRIC_R33_SESSION_ABORT_ROOT_V1";
    const SESSION_DROP_MARKER_ENV_V1: &str = "FERRIC_R33_SESSION_DROP_MARKER_V1";

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-r33-vault-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Debug)]
    struct CountedDrop(Rc<Cell<usize>>);

    impl Drop for CountedDrop {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[derive(Debug)]
    struct FileDropMarker(PathBuf);

    impl Drop for FileDropMarker {
        fn drop(&mut self) {
            let _ = fs::write(&self.0, b"dropped");
        }
    }

    fn digest(label: &str) -> String {
        sha256_hex(label.as_bytes())
    }

    fn workload(service_id: &str, policy: &str) -> M1R33WorkloadDocumentV1 {
        let rows = (0..3 * M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1)
            .map(|ordinal| {
                let within = ordinal % M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1;
                let server_start = ordinal / M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1;
                let phase = if within < 10 { "warmup" } else { "recorded" };
                M1R33WorkloadWindowV1 {
                    requests: vec![M1R33WorkloadRequestV1 {
                        expected_output_tokens: 2,
                        prompt_tokens: vec![1, 2],
                        request_ordinal: 0,
                    }],
                    row: M1R33CollectorRowV1 {
                        expected_work: M1R33WorkV1 {
                            input_tokens: 2,
                            output_tokens: 2,
                            successful_requests: 1,
                            total_tokens: 4,
                        },
                        id: format!("start-{server_start}.{phase}-{:02}", within % 10),
                        ordinal: ordinal as u64,
                        phase: phase.to_owned(),
                        server_start: server_start as u64,
                        window: (within % 10) as u64,
                    },
                }
            })
            .collect();
        M1R33WorkloadDocumentV1 {
            authority: M1_R33_SERVICE_AUTHORITY_V1.to_owned(),
            format: M1_R33_WORKLOAD_FORMAT_V1.to_owned(),
            policy_sha256: policy.to_owned(),
            rows,
            service_id: service_id.to_owned(),
            target: M1_R33_TARGET_V1.to_owned(),
        }
    }

    fn write_bundle(root: &Path) -> HeldM1R33ServiceBundleV1 {
        let service_id = digest("service");
        let policy = digest("policy");
        let workload_path = root.join("workload.json");
        let workload_bytes = encode_canonical_json_v1(&workload(&service_id, &policy)).unwrap();
        fs::write(&workload_path, &workload_bytes).unwrap();
        let plan = M1R33ServicePlanDocumentV1 {
            authority: M1_R33_SERVICE_AUTHORITY_V1.to_owned(),
            commands: M1R33CommandIdentitiesV1 {
                measure: digest("measure"),
                ready: digest("ready"),
                start: digest("start"),
                stop: digest("stop"),
            },
            expected_client_uid: geteuid().as_raw(),
            expected_daemon_uid: geteuid().as_raw(),
            format: M1_R33_SERVICE_PLAN_FORMAT_V1.to_owned(),
            implementation: serde_json::json!({"id": "ferric"}),
            io_timeout_ms: 100,
            policy_sha256: policy,
            service_id,
            slot: M1R33SlotV1 {
                hardware_configuration_sha256: digest("configuration"),
                hardware_sha256: digest("hardware"),
                id: "slot-0".to_owned(),
                target: M1_R33_TARGET_V1.to_owned(),
            },
            slot_gpu_ids: vec![0],
            socket_path: root.join("service.sock").to_str().unwrap().to_owned(),
            target: M1_R33_TARGET_V1.to_owned(),
            workload_path: workload_path.to_str().unwrap().to_owned(),
            workload_sha256: sha256_hex(&workload_bytes),
        };
        let plan_path = root.join("plan.json");
        fs::write(&plan_path, encode_canonical_json_v1(&plan).unwrap()).unwrap();
        HeldM1R33ServiceBundleV1::open(plan_path).unwrap()
    }

    fn fixture() -> (TestDirectory, HeldM1R33ServiceBundleV1) {
        let directory = TestDirectory::new();
        let bundle = write_bundle(&directory.0);
        (directory, bundle)
    }

    struct CompleteExecutor;

    impl<C, I> M1R33ResidentWindowExecutorV1<C, I> for CompleteExecutor {
        type Report = usize;

        fn execute(
            &mut self,
            capability: &mut vault::ExecutionCapability<'_, C, I>,
        ) -> M1R33ResidentExecutionDispositionV1<Self::Report> {
            assert_eq!(capability.row_ordinal(), 20 + capability.sequence() as u64);
            assert_eq!(capability.prompt_tokens(), &[1, 2]);
            assert_eq!(capability.expected_output_tokens(), 2);
            assert_eq!(
                capability.entered_windows(),
                capability.sequence() as u64 + 1
            );
            M1R33ResidentExecutionDispositionV1::Complete(capability.sequence())
        }
    }

    #[test]
    fn held_bundle_roster_executes_twenty_windows_without_releasing_custody() {
        let (_directory, bundle) = fixture();
        let custody_drops = Rc::new(Cell::new(0));
        let input_drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentSessionV1::new(bundle, 1, CountedDrop(Rc::clone(&custody_drops))).unwrap();
        assert_eq!(session.service_plan_sha256().len(), 64);
        for sequence in 0..M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 {
            let binding = session
                .bind_next(CountedDrop(Rc::clone(&input_drops)))
                .unwrap();
            assert_eq!(binding.sequence(), sequence);
            assert!(!binding.row_id().is_empty());
            assert_eq!(binding.prompt_tokens(), &[1, 2]);
            assert_eq!(binding.expected_output_tokens(), 2);
            assert_eq!(binding.execute(&mut CompleteExecutor).unwrap(), sequence);
        }
        assert_eq!(session.completed_windows(), 20);
        assert_eq!(session.phase(), M1R33ResidentSessionPhaseV1::Exhausted);
        assert_eq!(input_drops.get(), 20);
        assert_eq!(custody_drops.get(), 0);
        session.stop().unwrap();
        assert_eq!(custody_drops.get(), 1);
    }

    #[test]
    fn cancellation_recovers_exact_input_and_dropped_binding_quarantines_it() {
        let (_directory, bundle) = fixture();
        let custody_drops = Rc::new(Cell::new(0));
        let recovered_drops = Rc::new(Cell::new(0));
        let quarantined_drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentSessionV1::new(bundle, 0, CountedDrop(Rc::clone(&custody_drops))).unwrap();
        let recovered = session
            .bind_next(CountedDrop(Rc::clone(&recovered_drops)))
            .unwrap()
            .cancel()
            .unwrap();
        assert_eq!(session.phase(), M1R33ResidentSessionPhaseV1::Ready);
        assert_eq!(recovered_drops.get(), 0);
        drop(recovered);
        assert_eq!(recovered_drops.get(), 1);

        let binding = session
            .bind_next(CountedDrop(Rc::clone(&quarantined_drops)))
            .unwrap();
        drop(binding);
        assert_eq!(session.phase(), M1R33ResidentSessionPhaseV1::Faulted);
        assert_eq!(quarantined_drops.get(), 0);
        let rejected = match session.bind_next(CountedDrop(Rc::clone(&recovered_drops))) {
            Ok(_) => panic!("faulted session admitted a second input"),
            Err(rejected) => rejected,
        };
        assert_eq!(
            rejected.error(),
            M1R33ResidentSessionErrorV1::SessionNotReady
        );
        drop(rejected.into_input());
        assert_eq!(recovered_drops.get(), 2);
        session.stop().unwrap();
        assert_eq!(custody_drops.get(), 1);
        assert_eq!(quarantined_drops.get(), 0);
    }

    #[test]
    fn forgotten_outstanding_handle_cannot_install_a_second_input() {
        let (_directory, bundle) = fixture();
        let custody_drops = Rc::new(Cell::new(0));
        let installed_drops = Rc::new(Cell::new(0));
        let rejected_drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentSessionV1::new(bundle, 0, CountedDrop(Rc::clone(&custody_drops))).unwrap();
        let outstanding = session
            .bind_next(CountedDrop(Rc::clone(&installed_drops)))
            .unwrap();
        core::mem::forget(outstanding);
        let rejected = match session.bind_next(CountedDrop(Rc::clone(&rejected_drops))) {
            Ok(_) => panic!("outstanding session admitted a second input"),
            Err(rejected) => rejected,
        };
        assert_eq!(
            rejected.error(),
            M1R33ResidentSessionErrorV1::SessionNotReady
        );
        drop(rejected.into_input());
        assert_eq!(rejected_drops.get(), 1);
        assert_eq!(installed_drops.get(), 0);
        session.stop().unwrap();
        assert_eq!(installed_drops.get(), 1);
        assert_eq!(custody_drops.get(), 1);
    }

    #[test]
    fn held_workload_mutation_returns_rejected_input_and_quarantines_custody() {
        let (directory, bundle) = fixture();
        let custody_drops = Rc::new(Cell::new(0));
        let input_drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentSessionV1::new(bundle, 0, CountedDrop(Rc::clone(&custody_drops))).unwrap();
        fs::write(directory.0.join("workload.json"), b"replaced").unwrap();
        let rejected = match session.bind_next(CountedDrop(Rc::clone(&input_drops))) {
            Ok(_) => panic!("mutated held workload admitted an input"),
            Err(rejected) => rejected,
        };
        assert_eq!(
            rejected.error(),
            M1R33ResidentSessionErrorV1::HeldBundleChanged
        );
        drop(rejected.into_input());
        assert_eq!(input_drops.get(), 1);
        assert_eq!(
            session.stop(),
            Err(M1R33ResidentSessionErrorV1::HeldBundleChanged)
        );
        drop(session);
        assert_eq!(custody_drops.get(), 0);
    }

    struct TerminalExecutor;

    impl<C, I> M1R33ResidentWindowExecutorV1<C, I> for TerminalExecutor {
        type Report = ();

        fn execute(
            &mut self,
            _capability: &mut vault::ExecutionCapability<'_, C, I>,
        ) -> M1R33ResidentExecutionDispositionV1<Self::Report> {
            M1R33ResidentExecutionDispositionV1::Terminal("terminal")
        }
    }

    #[test]
    fn terminal_retains_both_owners_until_exact_stop() {
        let (_directory, bundle) = fixture();
        let custody_drops = Rc::new(Cell::new(0));
        let input_drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentSessionV1::new(bundle, 0, CountedDrop(Rc::clone(&custody_drops))).unwrap();
        let failure = session
            .bind_next(CountedDrop(Rc::clone(&input_drops)))
            .unwrap()
            .execute(&mut TerminalExecutor)
            .unwrap_err();
        assert_eq!(failure, M1R33ResidentSessionErrorV1::Terminal("terminal"));
        assert_eq!(session.phase(), M1R33ResidentSessionPhaseV1::Faulted);
        assert_eq!((custody_drops.get(), input_drops.get()), (0, 0));
        session.stop().unwrap();
        assert_eq!((custody_drops.get(), input_drops.get()), (1, 1));
    }

    struct CapturedSameTypeExecutor {
        captured: Option<CountedDrop>,
    }

    impl M1R33ResidentWindowExecutorV1<CountedDrop, CountedDrop> for CapturedSameTypeExecutor {
        type Report = ();

        fn execute(
            &mut self,
            capability: &mut vault::ExecutionCapability<'_, CountedDrop, CountedDrop>,
        ) -> M1R33ResidentExecutionDispositionV1<Self::Report> {
            assert_eq!(capability.sequence(), 0);
            drop(self.captured.take());
            M1R33ResidentExecutionDispositionV1::Complete(())
        }
    }

    #[test]
    fn captured_same_type_value_cannot_substitute_private_vault_owner() {
        let (_directory, bundle) = fixture();
        let resident_drops = Rc::new(Cell::new(0));
        let input_drops = Rc::new(Cell::new(0));
        let captured_drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentSessionV1::new(bundle, 0, CountedDrop(Rc::clone(&resident_drops)))
                .unwrap();
        let mut executor = CapturedSameTypeExecutor {
            captured: Some(CountedDrop(Rc::clone(&captured_drops))),
        };
        session
            .bind_next(CountedDrop(Rc::clone(&input_drops)))
            .unwrap()
            .execute(&mut executor)
            .unwrap();
        assert_eq!(captured_drops.get(), 1);
        assert_eq!(input_drops.get(), 1);
        assert_eq!(resident_drops.get(), 0);
        session.stop().unwrap();
        assert_eq!(resident_drops.get(), 1);
    }

    #[test]
    fn invalid_start_returns_held_bundle_and_custody() {
        let (_directory, bundle) = fixture();
        let drops = Rc::new(Cell::new(0));
        let failure =
            M1R33ResidentSessionV1::<_, ()>::new(bundle, 3, CountedDrop(Rc::clone(&drops)))
                .unwrap_err();
        assert_eq!(
            failure.error(),
            M1R33ResidentSessionErrorV1::InvalidServerStart
        );
        let (bundle, custody) = failure.into_parts();
        assert_eq!(bundle.workload().rows.len(), 60);
        drop(custody);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn direct_session_drop_quarantines_custody() {
        let (_directory, bundle) = fixture();
        let drops = Rc::new(Cell::new(0));
        let session =
            M1R33ResidentSessionV1::<_, ()>::new(bundle, 0, CountedDrop(Rc::clone(&drops)))
                .unwrap();
        drop(session);
        assert_eq!(drops.get(), 0);
    }

    struct PanicExecutor;

    impl M1R33ResidentWindowExecutorV1<FileDropMarker, FileDropMarker> for PanicExecutor {
        type Report = ();

        fn execute(
            &mut self,
            _capability: &mut vault::ExecutionCapability<'_, FileDropMarker, FileDropMarker>,
        ) -> M1R33ResidentExecutionDispositionV1<Self::Report> {
            panic!("intentional resident execution panic")
        }
    }

    #[test]
    fn resident_executor_unwind_is_exact_sigabrt_without_owner_destructors() {
        if std::env::var_os(SESSION_ABORT_ROOT_ENV_V1).is_some() {
            return;
        }
        let directory = TestDirectory::new();
        let marker = directory.0.join("drop-marker");
        let status = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("r33_resident_session::tests::resident_abort_child_entry")
            .arg("--nocapture")
            .env(SESSION_ABORT_ROOT_ENV_V1, &directory.0)
            .env(SESSION_DROP_MARKER_ENV_V1, &marker)
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(6));
        assert_eq!(status.code(), None);
        assert!(!marker.exists());
    }

    #[test]
    fn resident_abort_child_entry() {
        let Some(root) = std::env::var_os(SESSION_ABORT_ROOT_ENV_V1) else {
            return;
        };
        let marker = PathBuf::from(std::env::var_os(SESSION_DROP_MARKER_ENV_V1).unwrap());
        let bundle = write_bundle(Path::new(&root));
        let mut session =
            M1R33ResidentSessionV1::new(bundle, 0, FileDropMarker(marker.clone())).unwrap();
        let _ = session
            .bind_next(FileDropMarker(marker))
            .unwrap()
            .execute(&mut PanicExecutor);
        panic!("resident panic child escaped abort guard")
    }
}
