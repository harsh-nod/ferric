//! Owned process group and bounded streams. No detached worker or retry path.
use super::{Config, Result, require};
use rustix::process::{Pid, Signal, kill_process_group, test_kill_process_group};
use std::io::{BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const STDERR_LIMIT: usize = 2 << 20;
const FAILURE_DIAGNOSTIC_BYTES: usize = 64 << 10;
struct Stop {
    done: Mutex<bool>,
    wake: Condvar,
    expired: AtomicBool,
}

struct StderrCapture {
    bytes: Vec<u8>,
    failure: Option<String>,
}

fn failure_diagnostic(pid: u32, capture: &StderrCapture) -> serde_json::Value {
    let length = capture.bytes.len();
    let (prefix, suffix) = if length <= FAILURE_DIAGNOSTIC_BYTES {
        (capture.bytes.as_slice(), &[][..])
    } else {
        (
            &capture.bytes[..FAILURE_DIAGNOSTIC_BYTES / 2],
            &capture.bytes[length - FAILURE_DIAGNOSTIC_BYTES / 2..],
        )
    };
    serde_json::json!({
        "schema": "FerricFiniteChildFailureDiagnosticV1",
        "child_pid": pid,
        "captured_stderr_bytes": length,
        "stderr_prefix": prefix,
        "stderr_suffix": suffix,
        "omitted_middle_bytes": length - prefix.len() - suffix.len(),
        "capture_failure": capture.failure,
        "successful_observation": false,
        "native_close_confirmed": false,
        "gpu_execution_determined": false,
        "numerical_acceptance": false,
        "production_authority": false
    })
}

fn write_failure_diagnostic(
    writer: &mut impl Write,
    pid: u32,
    capture: &StderrCapture,
) -> std::io::Result<()> {
    writer.write_all(b"finite engineering failed child diagnostic ")?;
    serde_json::to_writer(&mut *writer, &failure_diagnostic(pid, capture))
        .map_err(std::io::Error::other)?;
    writer.write_all(b"\n")
}

pub(super) struct OwnedChild {
    child: Option<Child>,
    pid: Pid,
    pub(super) input: Option<ChildStdin>,
    pub(super) output: Option<BufReader<ChildStdout>>,
    stop: Arc<Stop>,
    watcher: Option<JoinHandle<()>>,
    stderr: Option<JoinHandle<StderrCapture>>,
    failure_stderr: Option<StderrCapture>,
    deadline: Instant,
}

impl OwnedChild {
    pub(super) fn spawn(config: &Config, deadline: Instant) -> Result<Self> {
        require(
            deadline > Instant::now(),
            "deadline expired before child spawn",
        )?;
        let mut command = Command::new(&config.worker.path);
        command
            .args([
                "--engineering-native",
                "--allow-unauthenticated-machine-code",
                "--devices",
            ])
            .arg(format!("{},{}", config.device_ids[0], config.device_ids[1]))
            .args([
                "--input-mode",
                super::mode_label(config.mode),
                "--timeout-ms",
            ])
            .arg(config.dispatch_timeout_ms.to_string())
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C");
        Self::spawn_command(command, deadline)
    }

    pub(super) fn spawn_command(mut command: Command, deadline: Instant) -> Result<Self> {
        require(
            deadline > Instant::now(),
            "deadline expired before child spawn",
        )?;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .map_err(|e| format!("finite child spawn: {e}"))?;
        let Some(pid) = i32::try_from(child.id()).ok().and_then(Pid::from_raw) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("invalid child PID".into());
        };
        // Construct the guard before creating threads or taking any fallible step.
        let stop = Arc::new(Stop {
            done: Mutex::new(false),
            wake: Condvar::new(),
            expired: AtomicBool::new(false),
        });
        let mut owned = Self {
            child: Some(child),
            pid,
            input: None,
            output: None,
            stop,
            watcher: None,
            stderr: None,
            failure_stderr: None,
            deadline,
        };
        let child = owned.child.as_mut().ok_or("missing owned child")?;
        owned.input = Some(child.stdin.take().ok_or("missing child stdin")?);
        owned.output = Some(BufReader::new(
            child.stdout.take().ok_or("missing child stdout")?,
        ));
        let mut stderr = child.stderr.take().ok_or("missing child stderr")?;
        let stop = Arc::clone(&owned.stop);
        owned.stderr = Some(
            thread::Builder::new()
                .name("finite-child-stderr".into())
                .spawn(move || {
                    let mut captured = Vec::new();
                    let mut block = [0; 8192];
                    loop {
                        let count = match stderr.read(&mut block) {
                            Ok(value) => value,
                            Err(error) => {
                                return StderrCapture {
                                    bytes: captured,
                                    failure: Some(error.to_string()),
                                };
                            }
                        };
                        if count == 0 {
                            return StderrCapture {
                                bytes: captured,
                                failure: None,
                            };
                        }
                        if captured
                            .len()
                            .checked_add(count)
                            .is_none_or(|n| n > STDERR_LIMIT)
                        {
                            captured.extend_from_slice(&block[..STDERR_LIMIT - captured.len()]);
                            if let Ok(done) = stop.done.lock() {
                                if !*done {
                                    let _ = kill_process_group(pid, Signal::KILL);
                                }
                            }
                            return StderrCapture {
                                bytes: captured,
                                failure: Some("finite child stderr limit".into()),
                            };
                        }
                        captured.extend_from_slice(&block[..count]);
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        let stop = Arc::clone(&owned.stop);
        owned.watcher = Some(
            thread::Builder::new()
                .name("finite-child-deadline".into())
                .spawn(move || {
                    let Ok(mut done) = stop.done.lock() else {
                        let _ = kill_process_group(pid, Signal::KILL);
                        return;
                    };
                    while !*done {
                        let left = deadline.saturating_duration_since(Instant::now());
                        if left.is_zero() {
                            stop.expired.store(true, Ordering::Release);
                            let _ = kill_process_group(pid, Signal::KILL);
                            return;
                        }
                        match stop.wake.wait_timeout(done, left) {
                            Ok((guard, _)) => done = guard,
                            Err(_) => {
                                let _ = kill_process_group(pid, Signal::KILL);
                                return;
                            }
                        }
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        Ok(owned)
    }

    pub(super) fn id(&self) -> u32 {
        self.pid.as_raw_nonzero().get() as u32
    }
    pub(super) fn check_deadline(&self) -> Result<()> {
        require(
            Instant::now() < self.deadline && !self.stop.expired.load(Ordering::Acquire),
            "finite parent/child deadline expired",
        )
    }

    fn collect_stderr(&mut self) -> Result<()> {
        if self.failure_stderr.is_none() {
            while !self
                .stderr
                .as_ref()
                .ok_or("missing stderr reader")?
                .is_finished()
            {
                self.check_deadline()?;
                thread::sleep(Duration::from_millis(10));
            }
            self.failure_stderr = Some(
                self.stderr
                    .take()
                    .ok_or("missing stderr reader")?
                    .join()
                    .map_err(|_| "stderr thread panicked")?,
            );
        }
        let capture = self
            .failure_stderr
            .as_ref()
            .ok_or("missing retained stderr")?;
        match &capture.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    /// Called only after a verified native Closed reply. Trailing stdout, failed
    /// exit, remaining group members or bounded diagnostic failure reject success.
    pub(super) fn finish(mut self) -> Result<Vec<u8>> {
        self.input.take();
        let mut extra = [0];
        require(
            self.output
                .as_mut()
                .ok_or("missing child stdout")?
                .read(&mut extra)
                .map_err(|e| e.to_string())?
                == 0,
            "trailing native stdout after Close",
        )?;
        self.output.take();
        // Wait for/drain stderr while the leader is still owned and unreaped.
        // Deadline or overflow signals therefore cannot target a reused PID.
        self.collect_stderr()?;
        let status = loop {
            self.check_deadline()?;
            // Serialize reaping with watchdog/group signalling, avoiding a stale
            // PID signal after this child has been reaped and its ID reused.
            let mut done = self
                .stop
                .done
                .lock()
                .map_err(|_| "child stop lock poisoned")?;
            if let Some(status) = self
                .child
                .as_mut()
                .ok_or("missing owned child")?
                .try_wait()
                .map_err(|e| e.to_string())?
            {
                *done = true;
                self.stop.wake.notify_all();
                break status;
            }
            drop(done);
            thread::sleep(Duration::from_millis(10));
        };
        // Reaping releases the PID. Disarm all Drop signalling before any later
        // failure; a post-reap group observation never authorizes a new signal.
        self.child.take();
        if let Some(watcher) = self.watcher.take() {
            watcher.join().map_err(|_| "deadline thread panicked")?;
        }
        require(status.success(), "finite child nonzero exit")?;
        require(
            test_kill_process_group(self.pid) == Err(rustix::io::Errno::SRCH),
            "owned child process group remains or cannot be inspected",
        )?;
        // Only verified healthy finish consumes the diagnostic without logging.
        Ok(self
            .failure_stderr
            .take()
            .ok_or("missing healthy stderr")?
            .bytes)
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Ok(mut done) = self.stop.done.lock() {
            *done = true;
            self.stop.wake.notify_all();
        }
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
        self.input.take();
        self.output.take();
        if let Some(mut child) = self.child.take() {
            let _ = kill_process_group(self.pid, Signal::KILL);
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(stderr) = self.stderr.take() {
            if let Ok(capture) = stderr.join() {
                self.failure_stderr = Some(capture);
            }
        }
        if let Some(capture) = self.failure_stderr.take() {
            // This is failure evidence after owned teardown, never a Closed or
            // no-GPU claim. Arbitrary bytes remain JSON byte arrays, not log text.
            let _ = write_failure_diagnostic(&mut std::io::stderr().lock(), self.id(), &capture);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn shell(script: &str, milliseconds: u64) -> OwnedChild {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        OwnedChild::spawn_command(
            command,
            Instant::now() + Duration::from_millis(milliseconds),
        )
        .unwrap()
    }
    fn absent(pid: Pid) {
        assert_eq!(test_kill_process_group(pid), Err(rustix::io::Errno::SRCH));
    }

    #[test]
    fn healthy_owned_cpu_child_is_reaped_and_threads_joined() {
        let child = shell("exit 0", 2000);
        let pid = child.pid;
        assert!(child.finish().unwrap().is_empty());
        absent(pid);
    }
    #[test]
    fn cpu_child_nonzero_or_trailing_stdout_cannot_finish_successfully() {
        for script in ["exit 7", "printf extra"] {
            let child = shell(script, 2000);
            let pid = child.pid;
            assert!(child.finish().is_err());
            absent(pid);
        }
    }
    #[test]
    fn deadline_breaks_blocked_read_and_reaps_owned_group() {
        let child = shell("exec /bin/sleep 30", 100);
        let pid = child.pid;
        assert!(child.finish().is_err());
        absent(pid);
    }
    #[test]
    fn deadline_breaks_blocked_write_and_drop_reaps_owned_group() {
        let mut child = shell("exec /bin/sleep 30", 100);
        let pid = child.pid;
        assert!(
            child
                .input
                .as_mut()
                .unwrap()
                .write_all(&vec![0; 1 << 20])
                .is_err()
        );
        drop(child);
        absent(pid);
    }
    #[test]
    fn bounded_stderr_overflow_kills_and_reaps_owned_group() {
        let child = shell("exec /usr/bin/head -c 2097153 /dev/zero >&2", 2000);
        let pid = child.pid;
        assert!(child.finish().is_err());
        absent(pid);
    }

    #[test]
    fn failed_cpu_child_stderr_is_retained_before_rejected_finish() {
        let mut child = shell(
            "printf 'native setup refused: fixture detail' >&2; exit 7",
            2000,
        );
        let pid = child.pid;
        child.collect_stderr().unwrap();
        let capture = child.failure_stderr.as_ref().unwrap();
        assert_eq!(capture.bytes, b"native setup refused: fixture detail");
        let mut log = Vec::new();
        write_failure_diagnostic(&mut log, child.id(), capture).unwrap();
        let prefix = b"finite engineering failed child diagnostic ";
        let value: serde_json::Value = serde_json::from_slice(&log[prefix.len()..]).unwrap();
        assert_eq!(value["schema"], "FerricFiniteChildFailureDiagnosticV1");
        assert_eq!(value["stderr_prefix"], serde_json::json!(capture.bytes));
        assert_eq!(value["successful_observation"], false);
        assert_eq!(value["native_close_confirmed"], false);
        assert!(child.finish().is_err());
        absent(pid);
    }

    #[test]
    fn failure_diagnostic_caps_prefix_suffix_without_lossy_text_or_false_close() {
        let bytes: Vec<_> = (0..STDERR_LIMIT).map(|i| (i % 256) as u8).collect();
        let capture = StderrCapture {
            bytes,
            failure: Some("finite child stderr limit".into()),
        };
        let value = failure_diagnostic(7, &capture);
        let first = value["stderr_prefix"].as_array().unwrap();
        let last = value["stderr_suffix"].as_array().unwrap();
        assert_eq!(first.len() + last.len(), FAILURE_DIAGNOSTIC_BYTES);
        assert_eq!(value["captured_stderr_bytes"], STDERR_LIMIT);
        assert_eq!(
            value["omitted_middle_bytes"],
            STDERR_LIMIT - FAILURE_DIAGNOSTIC_BYTES
        );
        assert_eq!(first[255], 255);
        assert_eq!(last[last.len() - 1], 255);
        assert_eq!(value["gpu_execution_determined"], false);
        let mut log = Vec::new();
        write_failure_diagnostic(&mut log, 7, &capture).unwrap();
        assert!(log.len() < 4 * FAILURE_DIAGNOSTIC_BYTES + 2048);
        assert_eq!(log.iter().filter(|b| **b == b'\n').count(), 1);
    }
}
