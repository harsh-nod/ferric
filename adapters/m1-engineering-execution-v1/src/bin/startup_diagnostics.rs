//! Opt-in, non-authoritative startup phase diagnostics for engineering smoke tools.

use std::ffi::OsStr;
use std::io::Write;
use std::time::Instant;

pub(crate) const STARTUP_PHASE_DIAGNOSTICS_ENV_V1: &str =
    "FERRIC_M1_ENGINEERING_STARTUP_PHASE_DIAGNOSTICS_V1";
const STARTUP_PHASE_DIAGNOSTICS_OPT_IN_V1: &str = "1";
const STARTUP_PHASE_RECORD_PREFIX_V1: &str = "FERRIC_M1_ENGINEERING_STARTUP_PHASE_V1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EngineeringStartupPhaseV1 {
    ArtifactAdmission,
    CpuModelBootstrapPreparation,
    RunnerBind,
    KfdBind,
    InitializeMemoryAllocationUpload,
    ControllerExecution,
}

impl EngineeringStartupPhaseV1 {
    const fn id(self) -> &'static str {
        match self {
            Self::ArtifactAdmission => "artifact-admission",
            Self::CpuModelBootstrapPreparation => "cpu-model-bootstrap-preparation",
            Self::RunnerBind => "runner-bind",
            Self::KfdBind => "kfd-bind",
            Self::InitializeMemoryAllocationUpload => "initialize-memory-allocation-upload",
            Self::ControllerExecution => "controller-execution",
        }
    }
}

pub(crate) struct EngineeringStartupDiagnosticsV1 {
    started: Option<Instant>,
}

impl EngineeringStartupDiagnosticsV1 {
    pub(crate) fn from_process_environment() -> Self {
        let enabled =
            is_explicitly_enabled(std::env::var_os(STARTUP_PHASE_DIAGNOSTICS_ENV_V1).as_deref());
        Self {
            started: enabled.then(Instant::now),
        }
    }

    pub(crate) fn completed(&self, phase: EngineeringStartupPhaseV1) {
        let Some(started) = self.started else {
            return;
        };
        let elapsed_ns = started.elapsed().as_nanos();
        emit_completion(&mut std::io::stderr().lock(), phase, elapsed_ns);
    }
}

fn is_explicitly_enabled(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new(STARTUP_PHASE_DIAGNOSTICS_OPT_IN_V1))
}

fn emit_completion(writer: &mut impl Write, phase: EngineeringStartupPhaseV1, elapsed_ns: u128) {
    let _ = writeln!(
        writer,
        "{STARTUP_PHASE_RECORD_PREFIX_V1} scope=engineering-only authority=none evidence=false \
         benchmark_comparable=false clock=std-instant-cumulative phase={} elapsed_ns={elapsed_ns}",
        phase.id(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    const PHASES: [EngineeringStartupPhaseV1; 6] = [
        EngineeringStartupPhaseV1::ArtifactAdmission,
        EngineeringStartupPhaseV1::CpuModelBootstrapPreparation,
        EngineeringStartupPhaseV1::RunnerBind,
        EngineeringStartupPhaseV1::KfdBind,
        EngineeringStartupPhaseV1::InitializeMemoryAllocationUpload,
        EngineeringStartupPhaseV1::ControllerExecution,
    ];

    #[test]
    fn only_exact_explicit_value_enables_diagnostics() {
        assert!(!is_explicitly_enabled(None));
        assert!(!is_explicitly_enabled(Some(OsStr::new(""))));
        assert!(!is_explicitly_enabled(Some(OsStr::new("0"))));
        assert!(!is_explicitly_enabled(Some(OsStr::new("true"))));
        assert!(!is_explicitly_enabled(Some(OsStr::new("01"))));
        assert!(is_explicitly_enabled(Some(OsStr::new("1"))));
    }

    #[test]
    fn phase_roster_and_record_format_are_fixed_and_data_free() {
        let expected_ids = [
            "artifact-admission",
            "cpu-model-bootstrap-preparation",
            "runner-bind",
            "kfd-bind",
            "initialize-memory-allocation-upload",
            "controller-execution",
        ];
        let actual_ids = PHASES.map(EngineeringStartupPhaseV1::id);
        assert_eq!(actual_ids, expected_ids);

        let mut output = Vec::new();
        emit_completion(&mut output, EngineeringStartupPhaseV1::RunnerBind, 17);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "FERRIC_M1_ENGINEERING_STARTUP_PHASE_V1 scope=engineering-only authority=none \
             evidence=false benchmark_comparable=false clock=std-instant-cumulative \
             phase=runner-bind elapsed_ns=17\n"
        );
    }

    #[test]
    fn injected_completion_records_preserve_monotonic_boundaries() {
        let mut output = Vec::new();
        emit_completion(
            &mut output,
            EngineeringStartupPhaseV1::ArtifactAdmission,
            10,
        );
        emit_completion(
            &mut output,
            EngineeringStartupPhaseV1::CpuModelBootstrapPreparation,
            25,
        );
        let text = String::from_utf8(output).unwrap();
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("phase=artifact-admission elapsed_ns=10"));
        assert!(lines[1].ends_with("phase=cpu-model-bootstrap-preparation elapsed_ns=25"));
    }

    struct RejectingWriter;

    impl Write for RejectingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("diagnostic sink rejected write"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("diagnostic sink rejected flush"))
        }
    }

    #[test]
    fn diagnostic_write_failure_is_ignored() {
        emit_completion(
            &mut RejectingWriter,
            EngineeringStartupPhaseV1::ControllerExecution,
            99,
        );
    }
}
