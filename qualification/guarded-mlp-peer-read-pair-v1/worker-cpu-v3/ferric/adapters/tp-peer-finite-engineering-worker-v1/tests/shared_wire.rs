use ferric_tp_peer_finite_engineering_worker_v1::finite_composition_wire;
use ferric_tp_peer_finite_engineering_worker_v1::native_guarded_mlp_decode_cli_v1 as guarded;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

struct OwnedCliChild(Option<Child>);

impl Drop for OwnedCliChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn worker_eof(args: &[&str]) -> Output {
    // EOF stops at bootstrap admission, before any device can be opened.
    let child = Command::new(env!(
        "CARGO_BIN_EXE_ferric-tp-peer-finite-engineering-worker-v1"
    ))
    .args(args)
    .env("HIP_VISIBLE_DEVICES", "")
    .env("ROCR_VISIBLE_DEVICES", "")
    .env("CUDA_VISIBLE_DEVICES", "")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let mut child = OwnedCliChild(Some(child));
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "worker EOF probe exceeded deadline"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        Instant::now() < deadline,
        "worker EOF probe exceeded deadline"
    );
    let output = child.0.take().unwrap().wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.len() < 4096);
    output
}

fn guarded_args(flag: &str) -> [&str; 8] {
    [
        flag,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,12",
        "--timeout-ms",
        "100",
        "--mode",
        "autoregressive",
    ]
}

#[test]
fn guarded_executable_routes_reach_bootstrap_without_legacy_fallback() {
    for flag in [
        guarded::FLAG,
        guarded::CAPTURE_FLAG,
        guarded::HOST_FLAG,
        guarded::HOST_SHARED_FLAG,
        guarded::REUSE_FLAG,
        guarded::HOST_PAIRED_READ_FLAG,
    ] {
        let output = worker_eof(&guarded_args(flag));
        assert_eq!(
            output.stderr, b"finite engineering worker: guarded bootstrap absent\n",
            "{flag}"
        );
    }
}

#[test]
fn paired_executable_route_preserves_mode_and_opt_in_refusals() {
    let mut args = guarded_args(guarded::HOST_PAIRED_READ_FLAG).to_vec();
    args[7] = "teacher-forced";
    assert_eq!(
        worker_eof(&args).stderr,
        b"finite engineering worker: paired hidden-read host mode requires fresh AR4\n"
    );
    args[7] = "autoregressive";
    args.remove(1);
    assert_eq!(
        worker_eof(&args).stderr,
        b"finite engineering worker: exact plain projection decode invocation\n"
    );
    let mut args = guarded_args(guarded::HOST_PAIRED_READ_FLAG).to_vec();
    args.push("--unexpected");
    assert_eq!(
        worker_eof(&args).stderr,
        b"finite engineering worker: exact plain projection decode invocation\n"
    );
}

#[path = "../src/wire_golden_tests.rs"]
mod golden;

#[test]
fn current_kfd_typed_state_exports_are_linked_without_legacy_graph_group() {
    assert!(std::mem::size_of::<fe2o3_kfd::Gfx950EngineeringPeerWaveOutputStateV5>() > 0);
    assert!(std::mem::size_of::<fe2o3_kfd::Gfx950EngineeringPeerWaveMlpStateV1>() > 0);
}
