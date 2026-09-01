//! Exact durable Worker V3 roster acquisition for the M1 program catalog.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use fe2o3_artifact_transaction::BuildAttempt;
use fe2o3_host::{
    admit_recovered_worker_v3_roster_v1, AuthenticatedWorkerV3RosterV1,
    CompilerGeneratedKernelExpectationRosterV1, RecoveredWorkerV3AdmissionErrorV1,
    RecoveredWorkerV3PinnedRosterV1, WorkerV3ProtectedRosterVerifierAdapterV1,
    WorkerV3ProtectedRosterVerifierBackendV1, WorkerV3RosterVerificationAuthenticationErrorV1,
};
use fe2o3_runtime_protocol::{recover_worker_v3_load_envelope_v2, WorkerV3LoadEnvelopeErrorV2};
use ferric_build::M1KernelArtifactFamilyV1;

use crate::{
    admit_m1_authenticated_worker_v3_programs_v1, M1AuthenticatedProgramSetIntakeFailureV1,
    M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedWorkerV3RostersV1, M1GemmWorkerV3RosterV1,
    M1LogitsWorkerV3RosterV1, M1PagedDecodeWorkerV3RosterV1, M1PrefillWorkerV3RosterV1,
    M1RmsNormWorkerV3RosterV1, M1RopeKvWorkerV3RosterV1, M1SwiGluWorkerV3RosterV1,
};

/// Exact durable publication selected for one M1 kernel family.
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

/// Invalid seven-family selector set.
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

/// Seven exact durable selectors in canonical K1-K7 family order.
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
    /// Names every family selector explicitly so family order cannot be inferred from a directory.
    ///
    /// # Errors
    ///
    /// Rejects one exact durable publication assigned to more than one family.
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

    /// Returns the exact selector bound to one family.
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

/// Seven current, inert host-admitted rosters before protected authentication.
#[must_use = "recovered roster custody must be authenticated or explicitly released"]
pub struct M1RecoveredWorkerV3RostersV1 {
    gemm: RecoveredWorkerV3PinnedRosterV1<M1GemmWorkerV3RosterV1>,
    rmsnorm: RecoveredWorkerV3PinnedRosterV1<M1RmsNormWorkerV3RosterV1>,
    rope_kv: RecoveredWorkerV3PinnedRosterV1<M1RopeKvWorkerV3RosterV1>,
    prefill: RecoveredWorkerV3PinnedRosterV1<M1PrefillWorkerV3RosterV1>,
    paged_decode: RecoveredWorkerV3PinnedRosterV1<M1PagedDecodeWorkerV3RosterV1>,
    swiglu: RecoveredWorkerV3PinnedRosterV1<M1SwiGluWorkerV3RosterV1>,
    logits: RecoveredWorkerV3PinnedRosterV1<M1LogitsWorkerV3RosterV1>,
}

impl fmt::Debug for M1RecoveredWorkerV3RostersV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1RecoveredWorkerV3RostersV1")
            .field("gemm", &self.gemm)
            .field("rmsnorm", &self.rmsnorm)
            .field("rope_kv", &self.rope_kv)
            .field("prefill", &self.prefill)
            .field("paged_decode", &self.paged_decode)
            .field("swiglu", &self.swiglu)
            .field("logits", &self.logits)
            .finish()
    }
}

impl M1RecoveredWorkerV3RostersV1 {
    /// Returns the exact flattened marker count retained across all seven rosters.
    #[must_use]
    pub fn program_count(&self) -> usize {
        self.gemm.entrypoints().len()
            + self.rmsnorm.entrypoints().len()
            + self.rope_kv.entrypoints().len()
            + self.prefill.entrypoints().len()
            + self.paged_decode.entrypoints().len()
            + self.swiglu.entrypoints().len()
            + self.logits.entrypoints().len()
    }

    /// This custody proves host admission only, not verification, load, or launch authority.
    #[must_use]
    pub const fn authenticates_verification_authority(&self) -> bool {
        false
    }
}

/// Exact pre-authentication stage that rejected one family selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1WorkerV3RosterAcquisitionStageV1 {
    RecoverEnvelope,
    HostRosterAdmission,
}

/// Typed failure for exact V2 recovery or compiler-generated roster admission.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1WorkerV3RosterAcquisitionFailureV1 {
    RecoverEnvelope {
        family: M1KernelArtifactFamilyV1,
        source: Box<WorkerV3LoadEnvelopeErrorV2>,
    },
    HostRosterAdmission {
        family: M1KernelArtifactFamilyV1,
        source: Box<RecoveredWorkerV3AdmissionErrorV1>,
    },
}

impl M1WorkerV3RosterAcquisitionFailureV1 {
    /// Returns the exact family whose selector failed.
    #[must_use]
    pub const fn family(&self) -> M1KernelArtifactFamilyV1 {
        match self {
            Self::RecoverEnvelope { family, .. } | Self::HostRosterAdmission { family, .. } => {
                *family
            }
        }
    }

    /// Returns the exact rejected pre-authentication stage.
    #[must_use]
    pub const fn stage(&self) -> M1WorkerV3RosterAcquisitionStageV1 {
        match self {
            Self::RecoverEnvelope { .. } => M1WorkerV3RosterAcquisitionStageV1::RecoverEnvelope,
            Self::HostRosterAdmission { .. } => {
                M1WorkerV3RosterAcquisitionStageV1::HostRosterAdmission
            }
        }
    }
}

impl fmt::Display for M1WorkerV3RosterAcquisitionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecoverEnvelope { family, source } => {
                write!(
                    formatter,
                    "M1 {family:?} Worker V3 envelope recovery failed: {source}"
                )
            }
            Self::HostRosterAdmission { family, source } => write!(
                formatter,
                "M1 {family:?} Worker V3 host roster admission failed: {source}"
            ),
        }
    }
}

impl Error for M1WorkerV3RosterAcquisitionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
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
#[derive(Debug)]
#[non_exhaustive]
pub enum M1WorkerV3AuthenticationFailureV1<E> {
    ProtectedAuthentication {
        family: M1KernelArtifactFamilyV1,
        source: Box<WorkerV3RosterVerificationAuthenticationErrorV1<E>>,
    },
    ProgramSetComposition(Box<M1AuthenticatedProgramSetIntakeFailureV1>),
}

impl<E> M1WorkerV3AuthenticationFailureV1<E> {
    /// Returns the family rejected by protected authentication, when applicable.
    #[must_use]
    pub const fn family(&self) -> Option<M1KernelArtifactFamilyV1> {
        match self {
            Self::ProtectedAuthentication { family, .. } => Some(*family),
            Self::ProgramSetComposition(_) => None,
        }
    }

    /// Returns the exact rejected authenticated-acquisition stage.
    #[must_use]
    pub const fn stage(&self) -> M1WorkerV3AuthenticationStageV1 {
        match self {
            Self::ProtectedAuthentication { .. } => {
                M1WorkerV3AuthenticationStageV1::ProtectedAuthentication
            }
            Self::ProgramSetComposition(_) => {
                M1WorkerV3AuthenticationStageV1::ProgramSetComposition
            }
        }
    }
}

impl<E: fmt::Display> fmt::Display for M1WorkerV3AuthenticationFailureV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectedAuthentication { family, source } => write!(
                formatter,
                "M1 {family:?} Worker V3 protected authentication failed: {source}"
            ),
            Self::ProgramSetComposition(failure) => write!(
                formatter,
                "M1 Worker V3 program-set composition failed at {:?}: {}",
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
            Self::ProgramSetComposition(_) => None,
        }
    }
}

/// End-to-end exact-selector acquisition failure.
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

/// Recovers and host-admits all seven exact selectors without creating authority.
///
/// # Errors
///
/// Returns the first exact family and recovery/admission stage that rejects.
pub fn recover_m1_worker_v3_rosters_v1(
    selectors: M1WorkerV3ArtifactSelectorsV1,
) -> Result<M1RecoveredWorkerV3RostersV1, M1WorkerV3RosterAcquisitionFailureV1> {
    let M1WorkerV3ArtifactSelectorsV1 {
        gemm,
        rmsnorm,
        rope_kv,
        prefill,
        paged_decode,
        swiglu,
        logits,
    } = selectors;
    Ok(M1RecoveredWorkerV3RostersV1 {
        gemm: recover_roster(M1KernelArtifactFamilyV1::Gemm, gemm)?,
        rmsnorm: recover_roster(M1KernelArtifactFamilyV1::RmsNorm, rmsnorm)?,
        rope_kv: recover_roster(M1KernelArtifactFamilyV1::RopeKv, rope_kv)?,
        prefill: recover_roster(M1KernelArtifactFamilyV1::Prefill, prefill)?,
        paged_decode: recover_roster(M1KernelArtifactFamilyV1::PagedDecode, paged_decode)?,
        swiglu: recover_roster(M1KernelArtifactFamilyV1::SwiGlu, swiglu)?,
        logits: recover_roster(M1KernelArtifactFamilyV1::Logits, logits)?,
    })
}

/// Authenticates seven recovered rosters through one protected backend and composes 12 programs.
///
/// # Errors
///
/// Returns the exact family rejected by protected authentication or the existing ownership-
/// retaining program-set composition failure.
pub fn authenticate_m1_worker_v3_rosters_v1<B, E>(
    rosters: M1RecoveredWorkerV3RostersV1,
    verifier: &mut WorkerV3ProtectedRosterVerifierAdapterV1<B>,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1WorkerV3AuthenticationFailureV1<E>>
where
    B: WorkerV3ProtectedRosterVerifierBackendV1<M1GemmWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1RmsNormWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1RopeKvWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1PrefillWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1PagedDecodeWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1SwiGluWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1LogitsWorkerV3RosterV1, Error = E>,
{
    let M1RecoveredWorkerV3RostersV1 {
        gemm,
        rmsnorm,
        rope_kv,
        prefill,
        paged_decode,
        swiglu,
        logits,
    } = rosters;
    let rosters = M1AuthenticatedWorkerV3RostersV1::new(
        authenticate_roster(M1KernelArtifactFamilyV1::Gemm, gemm, verifier)?,
        authenticate_roster(M1KernelArtifactFamilyV1::RmsNorm, rmsnorm, verifier)?,
        authenticate_roster(M1KernelArtifactFamilyV1::RopeKv, rope_kv, verifier)?,
        authenticate_roster(M1KernelArtifactFamilyV1::Prefill, prefill, verifier)?,
        authenticate_roster(
            M1KernelArtifactFamilyV1::PagedDecode,
            paged_decode,
            verifier,
        )?,
        authenticate_roster(M1KernelArtifactFamilyV1::SwiGlu, swiglu, verifier)?,
        authenticate_roster(M1KernelArtifactFamilyV1::Logits, logits, verifier)?,
    );
    admit_m1_authenticated_worker_v3_programs_v1(rosters).map_err(|failure| {
        M1WorkerV3AuthenticationFailureV1::ProgramSetComposition(Box::new(failure))
    })
}

/// Runs exact V2 recovery, host admission, protected authentication, and program composition.
///
/// # Errors
///
/// Returns the exact recovery, admission, protected-verification, or composition failure.
pub fn acquire_m1_authenticated_worker_v3_programs_v1<B, E>(
    selectors: M1WorkerV3ArtifactSelectorsV1,
    verifier: &mut WorkerV3ProtectedRosterVerifierAdapterV1<B>,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedWorkerV3AcquisitionFailureV1<E>>
where
    B: WorkerV3ProtectedRosterVerifierBackendV1<M1GemmWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1RmsNormWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1RopeKvWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1PrefillWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1PagedDecodeWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1SwiGluWorkerV3RosterV1, Error = E>
        + WorkerV3ProtectedRosterVerifierBackendV1<M1LogitsWorkerV3RosterV1, Error = E>,
{
    let rosters = recover_m1_worker_v3_rosters_v1(selectors).map_err(|source| {
        M1AuthenticatedWorkerV3AcquisitionFailureV1::RosterAcquisition(Box::new(source))
    })?;
    authenticate_m1_worker_v3_rosters_v1(rosters, verifier).map_err(|source| {
        M1AuthenticatedWorkerV3AcquisitionFailureV1::Authentication(Box::new(source))
    })
}

fn recover_roster<R: CompilerGeneratedKernelExpectationRosterV1>(
    family: M1KernelArtifactFamilyV1,
    selector: M1WorkerV3ArtifactSelectorV1,
) -> Result<RecoveredWorkerV3PinnedRosterV1<R>, M1WorkerV3RosterAcquisitionFailureV1> {
    let envelope = recover_worker_v3_load_envelope_v2(selector.output_dir(), selector.attempt())
        .map_err(
            |source| M1WorkerV3RosterAcquisitionFailureV1::RecoverEnvelope {
                family,
                source: Box::new(source),
            },
        )?;
    admit_recovered_worker_v3_roster_v1::<R>(envelope).map_err(|source| {
        M1WorkerV3RosterAcquisitionFailureV1::HostRosterAdmission {
            family,
            source: Box::new(source),
        }
    })
}

fn authenticate_roster<R, B, E>(
    family: M1KernelArtifactFamilyV1,
    roster: RecoveredWorkerV3PinnedRosterV1<R>,
    verifier: &mut WorkerV3ProtectedRosterVerifierAdapterV1<B>,
) -> Result<AuthenticatedWorkerV3RosterV1<R>, M1WorkerV3AuthenticationFailureV1<E>>
where
    R: CompilerGeneratedKernelExpectationRosterV1,
    B: WorkerV3ProtectedRosterVerifierBackendV1<R, Error = E>,
{
    AuthenticatedWorkerV3RosterV1::authenticate(roster, verifier).map_err(|source| {
        M1WorkerV3AuthenticationFailureV1::ProtectedAuthentication {
            family,
            source: Box::new(source),
        }
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

    fn selectors(root: &Path) -> M1WorkerV3ArtifactSelectorsV1 {
        M1WorkerV3ArtifactSelectorsV1::new(
            M1WorkerV3ArtifactSelectorV1::new(root.join("k1"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k2"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k3"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k4"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k5"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k6"), attempt()),
            M1WorkerV3ArtifactSelectorV1::new(root.join("k7"), attempt()),
        )
        .expect("seven distinct exact selectors")
    }

    #[test]
    fn selectors_preserve_exact_named_family_paths_and_attempts() {
        let root = Path::new("/durable-worker-v3");
        let selectors = selectors(root);
        for (family, suffix) in M1KernelArtifactFamilyV1::ALL
            .into_iter()
            .zip(["k1", "k2", "k3", "k4", "k5", "k6", "k7"])
        {
            let selector = selectors.selector(family);
            assert_eq!(selector.output_dir(), root.join(suffix));
            assert_eq!(selector.attempt(), attempt());
        }
    }

    #[test]
    fn missing_exact_attempt_fails_at_k1_recovery_without_trying_later_families() {
        let directory = TestDirectory::new("missing-attempt");
        let error = recover_m1_worker_v3_rosters_v1(selectors(&directory.0))
            .expect_err("no durable publication exists");
        assert_eq!(error.family(), M1KernelArtifactFamilyV1::Gemm);
        assert_eq!(
            error.stage(),
            M1WorkerV3RosterAcquisitionStageV1::RecoverEnvelope
        );
        assert!(matches!(
            error,
            M1WorkerV3RosterAcquisitionFailureV1::RecoverEnvelope { .. }
        ));
    }

    #[test]
    fn duplicate_exact_publication_is_rejected_before_recovery() {
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
        .expect_err("one exact publication cannot serve two families");
        assert_eq!(
            error,
            M1WorkerV3ArtifactSelectorsErrorV1::DuplicatePublication {
                first: M1KernelArtifactFamilyV1::Gemm,
                second: M1KernelArtifactFamilyV1::RopeKv,
            }
        );
    }
}
