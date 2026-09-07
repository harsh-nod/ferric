use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const EXTERNAL_DROP_MARKER_ENV_V1: &str = "FERRIC_R33_CUSTODY_VAULT_EXTERNAL_DROP_MARKER_V1";
const RELEASE_PROBE_ENV_V1: &str = "FERRIC_R33_RELEASE_CUSTODY_VAULT_ABORT_PROBE_V1";

static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ferric-r33-vault-abort-test-{}-{sequence}",
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

#[test]
fn built_probe_is_exact_sigabrt_and_never_runs_external_owner_destructors() {
    let directory = TestDirectory::new();
    let marker = directory.0.join("drop-marker");
    let program = std::env::var_os(RELEASE_PROBE_ENV_V1)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_ferric-r33-custody-vault-abort-probe").into());
    let status = Command::new(program)
        .env(EXTERNAL_DROP_MARKER_ENV_V1, &marker)
        .status()
        .unwrap();
    assert_eq!(status.signal(), Some(6));
    assert_eq!(status.code(), None);
    assert!(!marker.exists());
}
