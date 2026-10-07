//! Exercise the actual binary selector without a bootstrap or native device access.
#[test]
fn shared_full_executable_routes_selector_without_fallback_or_stdin_relock() {
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args([
        "--engineering-native-guarded-mlp-readiness40-position5-shared-full-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ])
    .env_clear()
    .env("PATH", "/usr/bin:/bin")
    .env("LANG", "C")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = Owned(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "readiness empty-input executable deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        Instant::now() < deadline,
        "readiness exit exceeded executable deadline"
    );
    assert_eq!(status.code(), Some(2));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        b"finite engineering worker: readiness bootstrap absent\n"
    );
}
#[test]
fn scoped_warm_executable_routes_selector_without_fallback_or_stdin_relock() {
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args([
        "--engineering-native-guarded-mlp-readiness40-position5-scoped-warm-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ])
    .env_clear()
    .env("PATH", "/usr/bin:/bin")
    .env("LANG", "C")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = Owned(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "readiness empty-input executable deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        Instant::now() < deadline,
        "readiness exit exceeded executable deadline"
    );
    assert_eq!(status.code(), Some(2));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        b"finite engineering worker: readiness bootstrap absent\n"
    );
}
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn readiness_executable_routes_valid_selector_to_bootstrap_not_legacy_parser() {
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args([
        "--engineering-native-guarded-mlp-readiness40-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ])
    .env_clear()
    .env("PATH", "/usr/bin:/bin")
    .env("LANG", "C")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = Owned(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "readiness empty-input executable deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        Instant::now() < deadline,
        "readiness exit exceeded executable deadline"
    );
    assert_eq!(status.code(), Some(2));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        b"finite engineering worker: readiness bootstrap absent\n"
    );
}

#[test]
fn position5_executable_routes_explicit_selector_to_bootstrap() {
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args([
        "--engineering-native-guarded-mlp-readiness40-position5-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ])
    .env_clear()
    .env("PATH", "/usr/bin:/bin")
    .env("LANG", "C")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = Owned(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "readiness empty-input executable deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        Instant::now() < deadline,
        "readiness exit exceeded executable deadline"
    );
    assert_eq!(status.code(), Some(2));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        b"finite engineering worker: readiness bootstrap absent\n"
    );
}

#[test]
fn causal_executable_routes_explicit_selector_without_nested_stdin_lock() {
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args([
        "--engineering-native-guarded-mlp-readiness40-causal-layer0-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ])
    .env_clear()
    .env("PATH", "/usr/bin:/bin")
    .env("LANG", "C")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = Owned(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "readiness empty-input executable deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        Instant::now() < deadline,
        "readiness exit exceeded executable deadline"
    );
    assert_eq!(status.code(), Some(2));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        b"finite engineering worker: readiness bootstrap absent\n"
    );
}

#[test]
fn full2303_executable_routes_explicit_selector_without_legacy_fallback_or_stdin_relock() {
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args([
        "--engineering-native-guarded-mlp-full2303-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ])
    .env_clear()
    .env("PATH", "/usr/bin:/bin")
    .env("LANG", "C")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = Owned(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "full2303 empty-input executable deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        Instant::now() < deadline,
        "full2303 exit exceeded executable deadline"
    );
    assert_eq!(status.code(), Some(2));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        b"finite engineering worker: full2303 bootstrap absent\n"
    );
}

#[test]
fn full2303_scoped_executable_routes_explicit_selector_without_fallback_or_stdin_relock() {
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args([
        "--engineering-native-guarded-mlp-full2303-scoped-warm-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ])
    .env_clear()
    .env("PATH", "/usr/bin:/bin")
    .env("LANG", "C")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = Owned(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "full scoped empty-input deadline"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(Instant::now() < deadline);
    assert_eq!(status.code(), Some(2));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .0
        .stdout
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stdout)
        .unwrap();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(4097)
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        b"finite engineering worker: full2303 bootstrap absent\n"
    );
}
