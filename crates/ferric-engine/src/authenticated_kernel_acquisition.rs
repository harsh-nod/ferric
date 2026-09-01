//! Exact durable Worker V3 aggregate-roster acquisition for the M1 program catalog.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use fe2o3_artifact_transaction::BuildAttempt;
use fe2o3_host::{
    admit_recovered_worker_v3_roster_v1, AuthenticatedWorkerV3RosterV1,
    RecoveredWorkerV3AdmissionErrorV1, RecoveredWorkerV3PinnedRosterV1,
    WorkerV3ProtectedRosterVerifierAdapterV1, WorkerV3ProtectedRosterVerifierBackendV1,
    WorkerV3RosterVerificationAuthenticationErrorV1,
};
use fe2o3_runtime_protocol::{recover_worker_v3_load_envelope_v2, WorkerV3LoadEnvelopeErrorV2};
use ferric_build::M1KernelArtifactFamilyV1;
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;

use crate::{
    admit_m1_authenticated_worker_v3_programs_v1, M1AuthenticatedProgramSetIntakeFailureV1,
    M1AuthenticatedWorkerV3ProgramSetV1,
};

/// Exact durable publication selected for the aggregate M1 compiler unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1WorkerV3ArtifactSelectorV1 {
    output_dir: PathBuf,
    attempt: BuildAttempt,
}

impl M1WorkerV3ArtifactSelectorV1 {
    /// Selects one exact build attempt in one exact durable output directory.
    #[must_use]
    pub fn new(output_dir: PathBuf, attempt: BuildAttempt) -> Self {
        Self {
            output_dir,
            attempt,
        }
    }

    /// Returns the durable output directory without inferring a latest attempt.
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Returns the exact durable build attempt.
    #[must_use]
    pub const fn attempt(&self) -> BuildAttempt {
        self.attempt
    }
}

/// Invalid legacy seven-family selector set.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1WorkerV3ArtifactSelectorsErrorV1 {
    DuplicatePublication {
        first: M1KernelArtifactFamilyV1,
        second: M1KernelArtifactFamilyV1,
    },
}

impl fmt::Display for M1WorkerV3ArtifactSelectorsErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePublication { first, second } => write!(
                formatter,
                "M1 {first:?} and {second:?} select the same exact Worker V3 publication"
            ),
        }
    }
}

impl Error for M1WorkerV3ArtifactSelectorsErrorV1 {}

/// Compatibility-only seven-selector container for the legacy manifest decoder.
///
/// This type cannot be converted to an aggregate selector because no member is authoritative for
/// the new all-kernels compiler unit. Aggregate recovery rejects it explicitly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1WorkerV3ArtifactSelectorsV1 {
    gemm: M1WorkerV3ArtifactSelectorV1,
    rmsnorm: M1WorkerV3ArtifactSelectorV1,
    rope_kv: M1WorkerV3ArtifactSelectorV1,
    prefill: M1WorkerV3ArtifactSelectorV1,
    paged_decode: M1WorkerV3ArtifactSelectorV1,
    swiglu: M1WorkerV3ArtifactSelectorV1,
    logits: M1WorkerV3ArtifactSelectorV1,
}

impl M1WorkerV3ArtifactSelectorsV1 {
    /// Retains the legacy manifest shape without granting aggregate publication authority.
    ///
    /// # Errors
    ///
    /// Returns an error when two families select the same exact publication.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gemm: M1WorkerV3ArtifactSelectorV1,
        rmsnorm: M1WorkerV3ArtifactSelectorV1,
        rope_kv: M1WorkerV3ArtifactSelectorV1,
        prefill: M1WorkerV3ArtifactSelectorV1,
        paged_decode: M1WorkerV3ArtifactSelectorV1,
        swiglu: M1WorkerV3ArtifactSelectorV1,
        logits: M1WorkerV3ArtifactSelectorV1,
    ) -> Result<Self, M1WorkerV3ArtifactSelectorsErrorV1> {
        let selectors = Self {
            gemm,
            rmsnorm,
            rope_kv,
            prefill,
            paged_decode,
            swiglu,
            logits,
        };
        let ordered = [
            (M1KernelArtifactFamilyV1::Gemm, &selectors.gemm),
            (M1KernelArtifactFamilyV1::RmsNorm, &selectors.rmsnorm),
            (M1KernelArtifactFamilyV1::RopeKv, &selectors.rope_kv),
            (M1KernelArtifactFamilyV1::Prefill, &selectors.prefill),
            (
                M1KernelArtifactFamilyV1::PagedDecode,
                &selectors.paged_decode,
            ),
            (M1KernelArtifactFamilyV1::SwiGlu, &selectors.swiglu),
            (M1KernelArtifactFamilyV1::Logits, &selectors.logits),
        ];
        for (index, (first_family, first)) in ordered.iter().enumerate() {
            for (second_family, second) in ordered.iter().skip(index + 1) {
                if first.output_dir == second.output_dir && first.attempt == second.attempt {
                    return Err(M1WorkerV3ArtifactSelectorsErrorV1::DuplicatePublication {
                        first: *first_family,
                        second: *second_family,
                    });
                }
            }
        }
        Ok(selectors)
    }

    /// Returns one legacy family selector for manifest diagnostics only.
    #[must_use]
    pub const fn selector(
        &self,
        family: M1KernelArtifactFamilyV1,
    ) -> &M1WorkerV3ArtifactSelectorV1 {
        match family {
            M1KernelArtifactFamilyV1::Gemm => &self.gemm,
            M1KernelArtifactFamilyV1::RmsNorm => &self.rmsnorm,
            M1KernelArtifactFamilyV1::RopeKv => &self.rope_kv,
            M1KernelArtifactFamilyV1::Prefill => &self.prefill,
            M1KernelArtifactFamilyV1::PagedDecode => &self.paged_decode,
            M1KernelArtifactFamilyV1::SwiGlu => &self.swiglu,
            M1KernelArtifactFamilyV1::Logits => &self.logits,
        }
    }
}

/// One current, inert host-admitted aggregate roster before protected authentication.
#[must_use = "recovered roster custody must be authenticated or explicitly released"]
pub struct M1RecoveredWorkerV3RosterV1 {
    selector: M1WorkerV3ArtifactSelectorV1,
    roster: RecoveredWorkerV3PinnedRosterV1<M1AllKernelsWorkerV3RosterV1>,
}

impl fmt::Debug for M1RecoveredWorkerV3RosterV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1RecoveredWorkerV3RosterV1")
            .field("selector", &self.selector)
            .field("roster", &self.roster)
            .finish()
    }
}

impl M1RecoveredWorkerV3RosterV1 {
    /// Returns the exact durable publication selector retained with recovered custody.
    #[must_use]
    pub const fn selector(&self) -> &M1WorkerV3ArtifactSelectorV1 {
        &self.selector
    }

    /// Returns the exact aggregate marker count.
    #[must_use]
    pub fn program_count(&self) -> usize {
        self.roster.entrypoints().len()
    }

    /// This custody proves host admission only, not verification, load, or launch authority.
    #[must_use]
    pub const fn authenticates_verification_authority(&self) -> bool {
        false
    }

    fn into_parts(
        self,
    ) -> (
        M1WorkerV3ArtifactSelectorV1,
        RecoveredWorkerV3PinnedRosterV1<M1AllKernelsWorkerV3RosterV1>,
    ) {
        (self.selector, self.roster)
    }
}

/// Compatibility alias retained for legacy selector-set callers.
pub type M1RecoveredWorkerV3RostersV1 = M1RecoveredWorkerV3RosterV1;

/// Exact pre-authentication stage that rejected aggregate acquisition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1WorkerV3RosterAcquisitionStageV1 {
    SelectAggregatePublication,
    RecoverEnvelope,
    HostRosterAdmission,
}

/// Typed failure for aggregate selection, exact V2 recovery, or roster admission.
#[must_use = "roster acquisition failure retains the exact rejected selector"]
#[derive(Debug)]
#[non_exhaustive]
pub enum M1WorkerV3RosterAcquisitionFailureV1 {
    LegacySelectorSetUnsupported {
        selectors: Box<M1WorkerV3ArtifactSelectorsV1>,
    },
    RecoverEnvelope {
        selector: M1WorkerV3ArtifactSelectorV1,
        source: Box<WorkerV3LoadEnvelopeErrorV2>,
    },
    HostRosterAdmission {
        selector: M1WorkerV3ArtifactSelectorV1,
        source: Box<RecoveredWorkerV3AdmissionErrorV1>,
    },
}

impl M1WorkerV3RosterAcquisitionFailureV1 {
    /// Returns the exact rejected pre-authentication stage.
    #[must_use]
    pub const fn stage(&self) -> M1WorkerV3RosterAcquisitionStageV1 {
        match self {
            Self::LegacySelectorSetUnsupported { .. } => {
                M1WorkerV3RosterAcquisitionStageV1::SelectAggregatePublication
            }
            Self::RecoverEnvelope { .. } => M1WorkerV3RosterAcquisitionStageV1::RecoverEnvelope,
            Self::HostRosterAdmission { .. } => {
                M1WorkerV3RosterAcquisitionStageV1::HostRosterAdmission
            }
        }
    }

    /// Returns the retained aggregate selector when recovery had selected one.
    #[must_use]
    pub const fn selector(&self) -> Option<&M1WorkerV3ArtifactSelectorV1> {
        match self {
            Self::LegacySelectorSetUnsupported { .. } => None,
            Self::RecoverEnvelope { selector, .. } | Self::HostRosterAdmission { selector, .. } => {
                Some(selector)
            }
        }
    }
}

impl fmt::Display for M1WorkerV3RosterAcquisitionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LegacySelectorSetUnsupported { .. } => write!(
                formatter,
                "legacy seven-family Worker V3 selectors cannot identify the aggregate M1 publication"
            ),
            Self::RecoverEnvelope { source, .. } => {
                write!(
                    formatter,
                    "M1 aggregate Worker V3 envelope recovery failed: {source}"
                )
            }
            Self::HostRosterAdmission { source, .. } => write!(
                formatter,
                "M1 aggregate Worker V3 host roster admission failed: {source}"
            ),
        }
    }
}

impl Error for M1WorkerV3RosterAcquisitionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LegacySelectorSetUnsupported { .. } => None,
            Self::RecoverEnvelope { source, .. } => Some(source),
            Self::HostRosterAdmission { source, .. } => Some(source),
        }
    }
}

/// Exact authenticated-acquisition stage that rejected retained custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1WorkerV3AuthenticationStageV1 {
    ProtectedAuthentication,
    ProgramSetComposition,
}

/// Typed protected-authentication or exact-program-set composition failure.
#[must_use = "authentication failure retains the exact selector and Worker V3 custody"]
#[derive(Debug)]
#[non_exhaustive]
pub enum M1WorkerV3AuthenticationFailureV1<E> {
    ProtectedAuthentication {
        source: Box<WorkerV3RosterVerificationAuthenticationErrorV1<E>>,
        recovered: Box<M1RecoveredWorkerV3RosterV1>,
    },
    ProgramSetComposition {
        selector: Box<M1WorkerV3ArtifactSelectorV1>,
        source: Box<M1AuthenticatedProgramSetIntakeFailureV1>,
    },
}

impl<E> M1WorkerV3AuthenticationFailureV1<E> {
    /// Returns the exact rejected authenticated-acquisition stage.
    #[must_use]
    pub const fn stage(&self) -> M1WorkerV3AuthenticationStageV1 {
        match self {
            Self::ProtectedAuthentication { .. } => {
                M1WorkerV3AuthenticationStageV1::ProtectedAuthentication
            }
            Self::ProgramSetComposition { .. } => {
                M1WorkerV3AuthenticationStageV1::ProgramSetComposition
            }
        }
    }

    /// Returns the exact aggregate publication selector retained at either rejection stage.
    #[must_use]
    pub fn selector(&self) -> &M1WorkerV3ArtifactSelectorV1 {
        match self {
            Self::ProtectedAuthentication { recovered, .. } => recovered.selector(),
            Self::ProgramSetComposition { selector, .. } => selector,
        }
    }
}

impl<E: fmt::Display> fmt::Display for M1WorkerV3AuthenticationFailureV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectedAuthentication { source, .. } => write!(
                formatter,
                "M1 aggregate Worker V3 protected authentication failed: {source}"
            ),
            Self::ProgramSetComposition {
                source: failure, ..
            } => write!(
                formatter,
                "M1 aggregate Worker V3 program-set composition failed at {:?}: {}",
                failure.phase(),
                failure.error()
            ),
        }
    }
}

impl<E> Error for M1WorkerV3AuthenticationFailureV1<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ProtectedAuthentication { source, .. } => Some(source),
            Self::ProgramSetComposition { .. } => None,
        }
    }
}

/// End-to-end exact-selector acquisition failure.
#[must_use = "acquisition failure retains the exact selector and available Worker V3 custody"]
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AuthenticatedWorkerV3AcquisitionFailureV1<E> {
    RosterAcquisition(Box<M1WorkerV3RosterAcquisitionFailureV1>),
    Authentication(Box<M1WorkerV3AuthenticationFailureV1<E>>),
}

impl<E: fmt::Display> fmt::Display for M1AuthenticatedWorkerV3AcquisitionFailureV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RosterAcquisition(source) => source.fmt(formatter),
            Self::Authentication(source) => source.fmt(formatter),
        }
    }
}

impl<E> Error for M1AuthenticatedWorkerV3AcquisitionFailureV1<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RosterAcquisition(source) => Some(source),
            Self::Authentication(source) => Some(source),
        }
    }
}

/// Recovers and host-admits one exact aggregate publication without creating authority.
///
/// # Errors
///
/// Returns the rejected recovery or host-admission stage while retaining the exact selector.
pub fn recover_m1_all_kernels_worker_v3_roster_v1(
    selector: M1WorkerV3ArtifactSelectorV1,
) -> Result<M1RecoveredWorkerV3RosterV1, M1WorkerV3RosterAcquisitionFailureV1> {
    let envelope = recover_worker_v3_load_envelope_v2(selector.output_dir(), selector.attempt())
        .map_err(
            |source| M1WorkerV3RosterAcquisitionFailureV1::RecoverEnvelope {
                selector: selector.clone(),
                source: Box::new(source),
            },
        )?;
    let roster = admit_recovered_worker_v3_roster_v1::<M1AllKernelsWorkerV3RosterV1>(envelope)
        .map_err(
            |source| M1WorkerV3RosterAcquisitionFailureV1::HostRosterAdmission {
                selector: selector.clone(),
                source: Box::new(source),
            },
        )?;
    Ok(M1RecoveredWorkerV3RosterV1 { selector, roster })
}

/// Compatibility-only legacy entry point retained for V1 selector-manifest callers.
///
/// It rejects before reading any of the seven publications because none identifies the aggregate
/// compiler unit.
///
/// # Errors
///
/// Always returns a typed rejection retaining the legacy selector set.
pub fn recover_m1_worker_v3_rosters_v1(
    selectors: M1WorkerV3ArtifactSelectorsV1,
) -> Result<M1RecoveredWorkerV3RostersV1, M1WorkerV3RosterAcquisitionFailureV1> {
    Err(
        M1WorkerV3RosterAcquisitionFailureV1::LegacySelectorSetUnsupported {
            selectors: Box::new(selectors),
        },
    )
}

/// Authenticates one recovered aggregate roster and attempts exact 12-program composition.
///
/// Protected-authentication failures retain the exact aggregate selector and recovered roster.
/// Composition failures retain the selector plus the authenticated roster or composed program set
/// in their residue.
///
/// # Errors
///
/// Returns a protected-authentication error or an ownership-retaining composition failure.
pub fn authenticate_m1_all_kernels_worker_v3_roster_v1<B, E>(
    roster: M1RecoveredWorkerV3RosterV1,
    verifier: &mut WorkerV3ProtectedRosterVerifierAdapterV1<B>,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1WorkerV3AuthenticationFailureV1<E>>
where
    B: WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1, Error = E>,
{
    let (selector, roster) = roster.into_parts();
    let roster = match AuthenticatedWorkerV3RosterV1::authenticate(roster, verifier) {
        Ok(roster) => roster,
        Err(failure) => {
            let (source, roster) = failure.into_parts();
            return Err(M1WorkerV3AuthenticationFailureV1::ProtectedAuthentication {
                source: Box::new(source),
                recovered: Box::new(M1RecoveredWorkerV3RosterV1 { selector, roster }),
            });
        }
    };
    match admit_m1_authenticated_worker_v3_programs_v1(roster) {
        Ok(programs) => Ok(programs),
        Err(source) => Err(M1WorkerV3AuthenticationFailureV1::ProgramSetComposition {
            selector: Box::new(selector),
            source: Box::new(source),
        }),
    }
}

/// Compatibility alias for callers already holding the now-singular recovered owner.
///
/// # Errors
///
/// Returns the same authentication or composition failure as the aggregate entry point.
pub fn authenticate_m1_worker_v3_rosters_v1<B, E>(
    roster: M1RecoveredWorkerV3RostersV1,
    verifier: &mut WorkerV3ProtectedRosterVerifierAdapterV1<B>,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1WorkerV3AuthenticationFailureV1<E>>
where
    B: WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1, Error = E>,
{
    authenticate_m1_all_kernels_worker_v3_roster_v1(roster, verifier)
}

/// Runs aggregate V2 recovery, host admission, authentication, and program composition.
///
/// # Errors
///
/// Returns the exact acquisition, authentication, or composition failure.
pub fn acquire_m1_all_kernels_authenticated_worker_v3_programs_v1<B, E>(
    selector: M1WorkerV3ArtifactSelectorV1,
    verifier: &mut WorkerV3ProtectedRosterVerifierAdapterV1<B>,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedWorkerV3AcquisitionFailureV1<E>>
where
    B: WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1, Error = E>,
{
    let roster = recover_m1_all_kernels_worker_v3_roster_v1(selector).map_err(|source| {
        M1AuthenticatedWorkerV3AcquisitionFailureV1::RosterAcquisition(Box::new(source))
    })?;
    authenticate_m1_all_kernels_worker_v3_roster_v1(roster, verifier).map_err(|source| {
        M1AuthenticatedWorkerV3AcquisitionFailureV1::Authentication(Box::new(source))
    })
}

/// Compatibility-only legacy end-to-end entry point.
///
/// This always rejects the seven-selector container before protected authentication.
///
/// # Errors
///
/// Always returns a roster-acquisition rejection retaining the legacy selector set.
pub fn acquire_m1_authenticated_worker_v3_programs_v1<B, E>(
    selectors: M1WorkerV3ArtifactSelectorsV1,
    verifier: &mut WorkerV3ProtectedRosterVerifierAdapterV1<B>,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedWorkerV3AcquisitionFailureV1<E>>
where
    B: WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1, Error = E>,
{
    let roster = recover_m1_worker_v3_rosters_v1(selectors).map_err(|source| {
        M1AuthenticatedWorkerV3AcquisitionFailureV1::RosterAcquisition(Box::new(source))
    })?;
    authenticate_m1_all_kernels_worker_v3_roster_v1(roster, verifier).map_err(|source| {
        M1AuthenticatedWorkerV3AcquisitionFailureV1::Authentication(Box::new(source))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    const DIRECT_ATTEMPT: &str = concat!(
        "1:",
        "00000000000000000000000000000000:",
        "0000000000000000000000000000000000000000000000000000000000000000"
    );

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-worker-v3-acquisition-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn attempt() -> BuildAttempt {
        BuildAttempt::from_env_value(DIRECT_ATTEMPT).expect("canonical direct attempt")
    }

    fn legacy_selectors(root: &Path) -> M1WorkerV3ArtifactSelectorsV1 {
        M1WorkerV3ArtifactSelectorsV1::new(
            M1WorkerV3ArtifactSelectorV1::new(root.join("k1"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k2"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k3"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k4"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k5"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k6"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k7"), attempt()),
        )
        .expect("seven distinct legacy selectors")
    }

    #[test]
    fn singular_selector_preserves_exact_aggregate_publication() {
        let selector = M1WorkerV3ArtifactSelectorV1::new(
            PathBuf::from("/durable-worker-v3/all-kernels"),
            attempt(),
        );
        assert_eq!(
            selector.output_dir(),
            Path::new("/durable-worker-v3/all-kernels")
        );
        assert_eq!(selector.attempt(), attempt());
    }

    #[test]
    fn missing_exact_attempt_fails_at_aggregate_recovery_and_retains_selector() {
        let directory = TestDirectory::new("missing-attempt");
        let selector = M1WorkerV3ArtifactSelectorV1::new(directory.0.clone(), attempt());
        let error = recover_m1_all_kernels_worker_v3_roster_v1(selector.clone())
            .expect_err("no durable publication exists");
        assert_eq!(
            error.stage(),
            M1WorkerV3RosterAcquisitionStageV1::RecoverEnvelope
        );
        assert_eq!(error.selector(), Some(&selector));
        assert!(matches!(
            error,
            M1WorkerV3RosterAcquisitionFailureV1::RecoverEnvelope { .. }
        ));
    }

    #[test]
    fn legacy_selector_set_is_rejected_before_any_publication_is_read() {
        let selectors = legacy_selectors(Path::new("/durable-worker-v3"));
        let error = recover_m1_worker_v3_rosters_v1(selectors.clone())
            .expect_err("legacy family publications cannot identify the aggregate unit");
        assert_eq!(
            error.stage(),
            M1WorkerV3RosterAcquisitionStageV1::SelectAggregatePublication
        );
        match error {
            M1WorkerV3RosterAcquisitionFailureV1::LegacySelectorSetUnsupported {
                selectors: retained,
            } => assert_eq!(*retained, selectors),
            other => panic!("unexpected compatibility rejection: {other:?}"),
        }
    }

    #[test]
    fn duplicate_legacy_publication_remains_rejected_by_manifest_container() {
        let root = Path::new("/durable-worker-v3");
        let duplicate = M1WorkerV3ArtifactSelectorV1::new(root.join("shared"), attempt());
        let error = M1WorkerV3ArtifactSelectorsV1::new(
            duplicate.clone(),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k2"), attempt()),
            duplicate,
            M1WorkerV3ArtifactSelectorV1::new(root.join("k4"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k5"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k6"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k7"), attempt()),
        )
        .expect_err("one exact publication cannot serve two legacy families");
        assert_eq!(
            error,
            M1WorkerV3ArtifactSelectorsErrorV1::DuplicatePublication {
                first: M1KernelArtifactFamilyV1::Gemm,
                second: M1KernelArtifactFamilyV1::RopeKv,
            }
        );
    }
}
