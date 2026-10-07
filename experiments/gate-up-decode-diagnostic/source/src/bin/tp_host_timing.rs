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

    #[allow(dead_code)] // Only the separate ordered64 host diagnostic opts in.
    pub fn create_ordered64(path: &Path) -> Result<Self, String> {
        use rustix::fs::{Mode, OFlags};
        use std::os::unix::fs::MetadataExt;

        let name = path
            .file_name()
            .ok_or("ordered64 host timing requires a file name")?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let directory = File::from(
            rustix::fs::open(
                parent,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| format!("open ordered64 host timing parent: {error}"))?,
        );
        let metadata = directory
            .metadata()
            .map_err(|error| format!("inspect ordered64 host timing parent: {error}"))?;
        if metadata.uid() != rustix::process::geteuid().as_raw() {
            return Err("ordered64 host timing parent must be an owned directory".into());
        }
        // Create relative to the inspected directory, not a re-resolved parent path.
        let file = File::from(
            rustix::fs::openat(
                &directory,
                name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(|error| format!("create ordered64 host timing sidecar: {error}"))?,
        );
        Ok(Self {
            timing: HostTiming::ordered64_diagnostic(),
            setup: None,
            closed: None,
            workload_sha256: None,
            file: Some(file),
        })
    }

    pub fn finish(mut self, result: &Result<(), String>) -> Result<(), String> {
        let Some(mut file) = self.file.take() else {
            return Ok(());
        };
        let mut snapshot = self.timing.snapshot();
        let ordered64 = self.timing.is_ordered64_diagnostic();
        let incomplete = ordered64 && snapshot["incomplete"] != false;
        snapshot["run_status"] = serde_json::json!(if result.is_ok() && !incomplete {
            "completed"
        } else {
            "failed"
        });
        snapshot["failure"] = serde_json::json!(
            result
                .as_ref()
                .err()
                .map(|s| s.chars().take(1024).collect::<String>())
                .or_else(|| incomplete.then(|| "ordered64 host timing incomplete".to_owned()))
        );
        snapshot["controller_pid"] = serde_json::json!(std::process::id());
        snapshot["setup"] = serde_json::json!(self.setup);
        snapshot["closed"] = serde_json::json!(self.closed);
        snapshot["workload_sha256"] = serde_json::json!(self.workload_sha256);
        if ordered64 {
            // Aggregate in memory and write once after execution, never per dispatch.
            let bytes = serde_json::to_vec(&snapshot)
                .map_err(|e| format!("serialize ordered64 host timing: {e}"))?;
            if bytes.len() >= 64 * 1024 * 1024 {
                return Err("ordered64 host timing exceeds 64 MiB output bound".into());
            }
            file.write_all(&bytes)
                .map_err(|e| format!("write host timing: {e}"))?;
        } else {
            serde_json::to_writer(&mut file, &snapshot)
                .map_err(|e| format!("write host timing: {e}"))?;
        }
        file.write_all(b"\n")
            .and_then(|()| file.flush())
            .and_then(|()| file.sync_all())
            .map_err(|e| format!("flush host timing: {e}"))?;
        if incomplete {
            return Err("ordered64 host timing incomplete".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic_directory(label: &str) -> std::path::PathBuf {
        use std::os::unix::fs::DirBuilderExt;
        let directory = std::env::temp_dir().join(format!(
            "ferric-ordered64-host-{label}-{}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        directory
    }

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

    #[test]
    fn ordered64_sidecar_is_explicit_private_bounded_and_uses_a_distinct_schema() {
        use std::os::unix::fs::PermissionsExt;
        let directory = diagnostic_directory("complete");
        let path = directory.join("timing.json");
        let file = TimingFile::create_ordered64(&path).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(TimingFile::create_ordered64(&path).is_err());
        file.timing.ordered_worker_elapsed(0, 64, 123);
        file.finish(&Ok(())).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() <= 64 * 1024 * 1024);
        let result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result["schema"], "FerricOrdered64HostTimingV1");
        assert_eq!(result["run_status"], "completed");
        assert_eq!(result["records"][0]["elapsed_ns"], 123);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn ordered64_incomplete_capture_returns_error_and_retains_failed_sidecar() {
        let directory = diagnostic_directory("incomplete");
        let path = directory.join("timing.json");
        let file = TimingFile::create_ordered64(&path).unwrap();
        let pending = file.timing.span("unfinished", None);
        assert!(file.finish(&Ok(())).is_err());
        drop(pending);
        let result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(result["run_status"], "failed");
        assert_eq!(result["incomplete"], true);
        assert_eq!(result["failure"], "ordered64 host timing incomplete");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn ordered64_parent_must_be_an_existing_real_owned_directory() {
        use std::os::unix::fs::MetadataExt;
        let directory = diagnostic_directory("parents");
        assert!(TimingFile::create_ordered64(&directory.join("missing/timing.json")).is_err());
        let ordinary = directory.join("ordinary");
        std::fs::write(&ordinary, b"retained").unwrap();
        assert!(TimingFile::create_ordered64(&ordinary.join("timing.json")).is_err());
        let linked = directory.join("linked");
        std::os::unix::fs::symlink(&directory, &linked).unwrap();
        assert!(TimingFile::create_ordered64(&linked.join("timing.json")).is_err());
        if std::fs::metadata("/").unwrap().uid() != rustix::process::geteuid().as_raw() {
            let error = TimingFile::create_ordered64(Path::new("/ferric-host-must-not-create"))
                .err()
                .unwrap();
            assert_eq!(
                error,
                "ordered64 host timing parent must be an owned directory"
            );
        }
        assert!(!directory.join("timing.json").exists());
        assert_eq!(std::fs::read(&ordinary).unwrap(), b"retained");
        std::fs::remove_file(linked).unwrap();
        std::fs::remove_file(ordinary).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn ordered64_existing_destination_symlink_is_not_followed() {
        let directory = diagnostic_directory("destination");
        let target = directory.join("retained");
        std::fs::write(&target, b"unchanged").unwrap();
        let link = directory.join("timing.json");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(TimingFile::create_ordered64(&link).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"unchanged");
        std::fs::remove_file(link).unwrap();
        std::fs::remove_file(target).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
