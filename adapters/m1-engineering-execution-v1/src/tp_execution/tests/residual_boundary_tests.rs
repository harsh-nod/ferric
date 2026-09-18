use super::*;
use crate::tp_execution::residual_boundary::EngineeringTpResidualBoundaryCaptureV1 as Capture;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const FILES: [&str; 4] = [
    "projection-input.bf16",
    "residual-before.bf16",
    "projection-partial.f32le",
    "hidden-after-broadcast.bf16",
];
const WIDTHS: [usize; 4] = [8192, 8192, 16_384, 8192];

struct Output(PathBuf);

impl Output {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ferric-tp-boundary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root.join("capture"))
    }

    fn manifest(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.0.join("manifest.json")).unwrap()).unwrap()
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn setup(positions: &[u32]) -> Value {
    let hash = "a".repeat(64);
    json!({
        "schema":"FerricQwen3TpEngineeringSetupV1", "authority":"none",
        "model":"Qwen/Qwen3-8B", "dtype":"BF16", "target":"gfx950:xnack-",
        "tensor_parallel":1, "repetitions":1, "warmup_runs":0, "new_tokens":2,
        "capacity":2, "prompt_tokens":[7], "device_unique_ids":[u64::MAX],
        "worker_pids":[123], "controller_sha256":hash, "worker_sha256":hash,
        "running_worker_sha256":[hash], "artifact_hsaco_id":hash,
        "artifact_manifest_id":hash, "artifact_handoff_id":hash,
        "model_bundle_id":hash, "executable_identity":"live_proc_exe_sha256",
        "collective":"host_staged_fp32_rank_order_reduce_bf16_residual",
        "residual_boundary_capture":{
            "positions":positions, "layer":0, "operation":"AttentionOutputSum",
            "tensor_parallel_rank":0, "tensor_parallel_world":1,
            "output_projection_weight_tensor":"model.layers.0.self_attn.o_proj.weight",
            "output_projection_weight_shape":[4096,4096],
            "projection_input_elements":4096, "residual_elements":4096,
            "projection_partial_elements":4096, "hidden_after_elements":4096,
            "residual_source":"host_staged_collective_input",
            "output_projection_weight_shard_sha256":"b".repeat(64),
            "benchmark_comparable":false,
        },
    })
}

fn closed() -> Value {
    json!({"schema":"FerricQwen3TpEngineeringClosedV1", "authority":"none",
        "all_workers_exited":true, "worker_pids":[123], "whole_seconds":1.0})
}

fn new_capture(output: &Output, positions: &[u32]) -> Capture {
    Capture::new(&output.0, positions.to_vec(), 2, setup(positions)).unwrap()
}

#[test]
fn legacy_boundary_capture_rejects_explicit_arithmetic_before_dispatch() {
    for mode in [
        EngineeringTpResidualArithmeticV1::Fp32ResidualV1,
        EngineeringTpResidualArithmeticV1::ProjectionBf16V1,
    ] {
        let output = Output::new();
        let mut capture = new_capture(&output, &[0]);
        let mut engine = fixture(1);
        engine.configure_host_residual_arithmetic(mode).unwrap();
        assert!(capture.step(&mut engine, 7).is_err());
        assert!(engine.transports[0].commands.is_empty());
        engine.close().unwrap();
        assert!(capture.finish(&closed()).is_err());
        assert!(!output.0.join("manifest.json").exists());
    }
}

#[test]
fn selected_boundary_retains_four_exact_payloads_and_weight_binding_after_close() {
    let output = Output::new();
    let mut capture = new_capture(&output, &[0]);
    let mut engine = fixture(1);
    let attention = engine.ranks[0].attention.id;
    assert_eq!(capture.step(&mut engine, 7).unwrap(), 42);
    assert_eq!(capture.step(&mut engine, 42).unwrap(), 42);
    assert!(
        engine.transports[0]
            .reads
            .iter()
            .any(|read| read.0 == attention)
    );
    engine.close().unwrap();
    capture.finish(&closed()).unwrap();

    let manifest = output.manifest();
    assert_eq!(manifest["positions"], json!([0]));
    assert_eq!(manifest["worker_close_confirmed"], true);
    assert_eq!(manifest["numerical_pass_claimed"], false);
    assert_eq!(
        manifest["setup"]["residual_boundary_capture"]["output_projection_weight_shard_sha256"],
        "b".repeat(64)
    );
    assert_eq!(
        manifest["setup"]["residual_boundary_capture"]["output_projection_weight_tensor"],
        "model.layers.0.self_attn.o_proj.weight"
    );
    assert_eq!(
        manifest["setup"]["residual_boundary_capture"]["output_projection_weight_shape"],
        json!([4096, 4096])
    );
    assert_eq!(manifest["rows"][0]["layer"], 0);
    assert_eq!(manifest["rows"][0]["operation"], "AttentionOutputSum");
    assert_eq!(
        manifest["rows"][0]["gpu_hidden_matches_host_broadcast"],
        true
    );
    for (name, bytes) in FILES.iter().zip(WIDTHS) {
        let payload = std::fs::read(output.0.join(name)).unwrap();
        assert_eq!(payload.len(), bytes);
    }
    assert!(
        std::fs::read(output.0.join("projection-input.bf16"))
            .unwrap()
            .chunks_exact(2)
            .all(|bits| bits == [0, 0])
    );
    assert!(
        std::fs::read(output.0.join("residual-before.bf16"))
            .unwrap()
            .chunks_exact(2)
            .all(|bits| bits == 0x3f80_u16.to_le_bytes())
    );
    assert!(
        std::fs::read(output.0.join("projection-partial.f32le"))
            .unwrap()
            .chunks_exact(4)
            .all(|bits| bits == (1.0_f32 / 1024.0).to_le_bytes())
    );
    assert!(
        std::fs::read(output.0.join("hidden-after-broadcast.bf16"))
            .unwrap()
            .chunks_exact(2)
            .all(|bits| bits == 0x3f80_u16.to_le_bytes())
    );
    for (index, descriptor) in manifest["payloads"].as_array().unwrap().iter().enumerate() {
        let bytes = std::fs::read(output.0.join(FILES[index])).unwrap();
        assert_eq!(descriptor["sha256"], crate::hex(&Sha256::digest(&bytes)));
    }
}

#[test]
fn unselected_and_default_steps_add_no_boundary_reads() {
    let output = Output::new();
    let mut capture = new_capture(&output, &[1]);
    let mut engine = fixture(1);
    let attention = engine.ranks[0].attention.id;
    capture.step(&mut engine, 7).unwrap();
    assert!(
        engine.transports[0]
            .reads
            .iter()
            .all(|read| read.0 != attention)
    );
    capture.step(&mut engine, 42).unwrap();
    assert!(
        engine.transports[0]
            .reads
            .iter()
            .any(|read| read.0 == attention)
    );
    engine.close().unwrap();
    capture.finish(&closed()).unwrap();

    let mut ordinary = fixture(1);
    let ordinary_attention = ordinary.ranks[0].attention.id;
    ordinary.step(7).unwrap();
    assert!(
        ordinary.transports[0]
            .reads
            .iter()
            .all(|read| read.0 != ordinary_attention)
    );
    ordinary.close().unwrap();
}

#[test]
fn gpu_hidden_mismatch_is_retained_raw_without_acceptance_claim() {
    let output = Output::new();
    let mut capture = new_capture(&output, &[0]);
    let mut engine = fixture(1);
    let hidden = engine.ranks[0].hidden.id;
    engine.transports[0]
        .read_overrides_after_partial
        .push((hidden, 0, 0x4000));
    capture.step(&mut engine, 7).unwrap();
    capture.step(&mut engine, 42).unwrap();
    engine.close().unwrap();
    capture.finish(&closed()).unwrap();
    let manifest = output.manifest();
    assert_eq!(
        manifest["rows"][0]["gpu_hidden_matches_host_broadcast"],
        false
    );
    assert_eq!(manifest["numerical_pass_claimed"], false);
    assert_eq!(
        &std::fs::read(output.0.join("hidden-after-broadcast.bf16")).unwrap()[..2],
        &0x4000_u16.to_le_bytes()
    );
}

#[test]
fn unsupported_engine_profiles_reject_before_dispatch() {
    for mut engine in [fixture(2), fixture_model(1, draft(), 2)] {
        let output = Output::new();
        let mut capture = new_capture(&output, &[1]);
        assert!(capture.step(&mut engine, 7).is_err());
        assert!(
            engine
                .transports
                .iter()
                .all(|transport| transport.commands.is_empty())
        );
        assert!(
            engine
                .transports
                .iter()
                .all(|transport| transport.reads.is_empty())
        );
        engine.close().unwrap();
        assert!(capture.finish(&closed()).is_err());
        assert!(!output.0.join("manifest.json").exists());
    }
}

#[test]
fn each_boundary_read_failure_poisons_and_never_publishes() {
    for tensor in 0..3 {
        let output = Output::new();
        let mut capture = new_capture(&output, &[0]);
        let mut engine = fixture(1);
        let id = match tensor {
            0 => engine.ranks[0].attention.id,
            1 => engine.ranks[0].partial.id,
            _ => engine.ranks[0].hidden.id,
        };
        engine.transports[0].fail_boundary_read = Some(id);
        assert!(capture.step(&mut engine, 7).is_err());
        assert!(engine.step(7).is_err());
        assert!(engine.reset_sequence().is_err());
        engine.close().unwrap();
        assert!(capture.finish(&closed()).is_err());
        assert!(!output.0.join("manifest.json").exists());
    }
}

#[test]
fn selection_setup_close_and_publication_are_fail_closed() {
    for positions in [vec![], vec![1, 1], vec![1, 0], vec![2], (0..9).collect()] {
        let output = Output::new();
        assert!(Capture::new(&output.0, positions.clone(), 2, setup(&positions)).is_err());
        assert!(!output.0.exists());
    }
    for (field, replacement) in [
        ("tensor_parallel_world", json!(2)),
        ("layer", json!(1)),
        ("operation", json!("FeedForwardDownSum")),
        (
            "output_projection_weight_tensor",
            json!("layers.1.self_attn.o_proj.weight"),
        ),
        ("output_projection_weight_shape", json!([4096, 4095])),
        ("projection_input_elements", json!(4095)),
        (
            "output_projection_weight_shard_sha256",
            json!("0".repeat(64)),
        ),
        ("benchmark_comparable", json!(true)),
    ] {
        let output = Output::new();
        let mut value = setup(&[0]);
        value["residual_boundary_capture"][field] = replacement;
        assert!(
            Capture::new(&output.0, vec![0], 2, value).is_err(),
            "{field}"
        );
        assert!(!output.0.exists());
    }

    let output = Output::new();
    let mut capture = new_capture(&output, &[0]);
    let mut engine = fixture(1);
    capture.step(&mut engine, 7).unwrap();
    capture.step(&mut engine, 42).unwrap();
    engine.close().unwrap();
    let mut bad_close = closed();
    bad_close["all_workers_exited"] = json!(false);
    assert!(capture.finish(&bad_close).is_err());
    assert!(!output.0.join("manifest.json").exists());

    let output = Output::new();
    let mut capture = new_capture(&output, &[0]);
    let mut engine = fixture(1);
    capture.step(&mut engine, 7).unwrap();
    capture.step(&mut engine, 42).unwrap();
    engine.close().unwrap();
    std::fs::write(output.0.join("manifest.json"), b"unrelated").unwrap();
    assert!(capture.finish(&closed()).is_err());
    assert_eq!(
        std::fs::read(output.0.join("manifest.json")).unwrap(),
        b"unrelated"
    );
}
