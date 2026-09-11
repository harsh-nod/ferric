//! Optional exclusive sidecar. The normal correctness stream is unchanged.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use ferric_m1_engineering_execution_v1::host_timing::HostTiming;

pub struct TimingFile {
    pub timing: HostTiming,
    pub setup: Option<serde_json::Value>,
    pub closed: Option<serde_json::Value>,
    pub workload_sha256: Option<String>,
    file: Option<File>,
}

impl TimingFile {
    pub fn create(path: Option<&Path>) -> Result<Self, String> {
        let file = path
            .map(|path| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)
                    .map_err(|error| format!("create host timing sidecar: {error}"))
            })
            .transpose()?;
        Ok(Self {
            timing: if file.is_some() {
                HostTiming::enabled()
            } else {
                HostTiming::default()
            },
            setup: None,
            closed: None,
            workload_sha256: None,
            file,
        })
    }

    pub fn finish(mut self, result: &Result<(), String>) -> Result<(), String> {
        let Some(mut file) = self.file.take() else {
            return Ok(());
        };
        let mut snapshot = self.timing.snapshot();
        snapshot["run_status"] = serde_json::json!(if result.is_ok() {
            "completed"
        } else {
            "failed"
        });
        snapshot["failure"] = serde_json::json!(
            result
                .as_ref()
                .err()
                .map(|s| s.chars().take(1024).collect::<String>())
        );
        snapshot["controller_pid"] = serde_json::json!(std::process::id());
        snapshot["setup"] = serde_json::json!(self.setup);
        snapshot["closed"] = serde_json::json!(self.closed);
        snapshot["workload_sha256"] = serde_json::json!(self.workload_sha256);
        serde_json::to_writer(&mut file, &snapshot)
            .map_err(|e| format!("write host timing: {e}"))?;
        file.write_all(b"\n")
            .and_then(|()| file.flush())
            .and_then(|()| file.sync_all())
            .map_err(|e| format!("flush host timing: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_diagnostics_do_not_open_or_record() {
        let file = TimingFile::create(None).unwrap();
        assert!(!file.timing.is_enabled());
        assert!(file.file.is_none());
        file.finish(&Err("original error".into())).unwrap();
    }

    #[test]
    fn sidecar_is_exclusive_private_and_records_failure() {
        use std::os::unix::fs::PermissionsExt;
        let path =
            std::env::temp_dir().join(format!("ferric-host-timing-{}.json", std::process::id()));
        let file = TimingFile::create(Some(&path)).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(TimingFile::create(Some(&path)).is_err());
        file.finish(&Err("bounded host failure".into())).unwrap();
        let result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(result["run_status"], "failed");
        assert_eq!(result["failure"], "bounded host failure");
        assert_eq!(result["active_records"], 0);
        std::fs::remove_file(path).unwrap();
    }
}
