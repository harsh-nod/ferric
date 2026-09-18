use super::*;
use crate::tp_execution::final_stage::EngineeringTpFinalStageCaptureV1 as Capture;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Output(PathBuf);

impl Output {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = std::env::temp_dir().canonicalize().unwrap();
        Self(parent.join(format!(
            "fc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn manifest(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.0.join("manifest.json")).unwrap()).unwrap()
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        if self.0.exists() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

fn setup(positions: &[u32]) -> Value {
    let hash = "a".repeat(64);
    json!({
        "schema":"FerricQwen3TpEngineeringSetupV1", "authority":"none",
        "model":"Qwen/Qwen3-8B", "dtype":"BF16", "target":"gfx950:xnack-",
        "tensor_parallel":1, "repetitions":1, "warmup_runs":0, "new_tokens":2,
        "capacity":2, "prompt_tokens":[7], "device_unique_ids":[u64::MAX], "worker_pids":[123],
        "controller_sha256":hash, "worker_sha256":hash, "running_worker_sha256":[hash],
        "artifact_hsaco_id":hash, "artifact_manifest_id":hash, "artifact_handoff_id":hash,
        "model_bundle_id":hash, "executable_identity":"live_proc_exe_sha256",
        "final_stage_capture":{"positions":positions,"benchmark_comparable":false},
    })
}

fn closed() -> Value {
    json!({"schema":"FerricQwen3TpEngineeringClosedV1","authority":"none",
        "all_workers_exited":true,"worker_pids":[123],"whole_seconds":1.0})
}

fn capture(output: &Output, positions: &[u32]) -> Capture {
    Capture::new(&output.0, positions.to_vec(), 2, setup(positions)).unwrap()
}

#[test]
fn final_stage_reads_follow_argmax_and_preserve_exact_extents_and_choice() {
    let mut engine = fixture(1);
    engine.reset_sequence().unwrap();
    let epoch = engine.sequence.epoch();
    let row = engine.final_stage_step(7).unwrap();
    assert_eq!(
        (row.epoch, row.position, row.token, row.choice),
        (epoch, 0, 7, 42)
    );
    let tensors = [
        engine.ranks[0].hidden,
        engine.ranks[0].normalized,
        engine.ranks[0].logits,
    ];
    let transport = &engine.transports[0];
    let reads = &transport.reads[transport.reads.len() - 3..];
    for ((tensor, bytes), read) in tensors.iter().zip(&row.payloads).zip(reads) {
        assert_eq!(bytes.len(), tensor.elements * 2);
        assert_eq!(bytes, &transport.buffers[&tensor.id]);
        assert_eq!(*read, (tensor.id, 0, bytes.len(), Some(ARGMAX)));
    }
    assert_eq!(engine.dispatch_counts(), [544]);
    engine.close().unwrap();
}

#[test]
fn final_stage_rejects_wrong_shape_world_role_and_closed_engine_before_dispatch() {
    for mut engine in [fixture(2), fixture_model(1, draft(), 2)] {
        assert!(engine.final_stage_step(7).is_err());
        assert!(
            engine
                .transports
                .iter()
                .all(|transport| transport.commands.is_empty())
        );
        engine.close().unwrap();
    }
    let mut engine = fixture(1);
    engine.ranks[0].logits.elements -= 1;
    assert!(engine.final_stage_step(7).is_err());
    assert!(engine.transports[0].commands.is_empty());
    engine.close().unwrap();
    assert!(engine.final_stage_step(7).is_err());
}

#[test]
fn capture_failure_on_each_read_poisons_sequence_and_never_publishes() {
    for selected in 0..3 {
        let output = Output::new();
        let mut capture = capture(&output, &[0, 1]);
        let mut engine = fixture(1);
        let ids = [
            engine.ranks[0].hidden.id,
            engine.ranks[0].normalized.id,
            engine.ranks[0].logits.id,
        ];
        engine.transports[0].fail_final_read = Some(ids[selected]);
        assert!(capture.step(&mut engine, 7).is_err());
        assert!(engine.step(42).is_err());
        assert!(engine.reset_sequence().is_err());
        engine.close().unwrap();
        assert!(capture.finish(&closed()).is_err());
        assert!(!output.0.join("manifest.json").exists());
    }
}

#[test]
fn capture_records_consumed_sequence_not_last_generated_token_and_keeps_disagreement() {
    let output = Output::new();
    let mut capture = capture(&output, &[0, 1]);
    let mut engine = fixture(1);
    engine.reset_sequence().unwrap();
    assert_eq!(capture.step(&mut engine, 7).unwrap(), 42);
    assert_eq!(capture.step(&mut engine, 42).unwrap(), 42);
    engine.close().unwrap();
    let receipt = capture.finish(&closed()).unwrap();
    let manifest = output.manifest();
    assert_eq!(manifest["input_tokens"], json!([7, 42]));
    assert_eq!(manifest["gpu_choices"], json!([42, 42]));
    assert_eq!(manifest["setup"]["device_unique_ids"], json!([u64::MAX]));
    assert_eq!(
        manifest["rows"][0]["logits_diagnostics"],
        json!({"finite":true,"cpu_choice":0,"gpu_choice_matches":false})
    );
    assert_eq!(manifest["worker_close_confirmed"], true);
    for name in [
        "qualification",
        "benchmark_comparable",
        "numerical_pass_claimed",
    ] {
        assert_eq!(manifest[name], false);
    }
    let raw_manifest = std::fs::read(output.0.join("manifest.json")).unwrap();
    assert_eq!(
        receipt["manifest_sha256"],
        crate::hex(&Sha256::digest(&raw_manifest))
    );
    for payload in manifest["payloads"].as_array().unwrap() {
        let bytes = std::fs::read(output.0.join(payload["file"].as_str().unwrap())).unwrap();
        assert_eq!(payload["bytes"].as_u64().unwrap(), bytes.len() as u64);
        assert_eq!(payload["sha256"], crate::hex(&Sha256::digest(&bytes)));
        for (index, row) in manifest["rows"].as_array().unwrap().iter().enumerate() {
            let descriptor = &row[match payload["file"].as_str().unwrap() {
                "residual.bf16" => "residual",
                "normalized.bf16" => "normalized",
                _ => "logits",
            }];
            let width = bytes.len() / 2;
            assert_eq!(descriptor["offset_bytes"], index * width);
            assert_eq!(
                descriptor["sha256"],
                crate::hex(&Sha256::digest(&bytes[index * width..(index + 1) * width]))
            );
        }
    }
    assert!(capture.finish(&closed()).is_err());
}

#[test]
fn capture_retains_nonfinite_at_either_end_and_never_replaces_gpu_choice() {
    for index in [0, 151_935] {
        let output = Output::new();
        let mut capture = capture(&output, &[1]);
        let mut engine = fixture(1);
        engine.transports[0].logit_overrides = vec![(index, 0x7fc1)];
        assert_eq!(capture.step(&mut engine, 7).unwrap(), 42);
        assert_eq!(capture.step(&mut engine, 42).unwrap(), 42);
        engine.close().unwrap();
        capture.finish(&closed()).unwrap();
        let manifest = output.manifest();
        assert_eq!(
            manifest["rows"][0]["logits_diagnostics"],
            json!({
            "finite":false,"first_nonfinite_token":index,"cpu_choice":null,"gpu_choice_matches":null})
        );
        let bytes = std::fs::read(output.0.join("logits.bf16")).unwrap();
        assert_eq!(&bytes[index * 2..index * 2 + 2], &0x7fc1_u16.to_le_bytes());
        assert_eq!(bytes.len(), 151_936 * 2);
    }
}

#[test]
fn capture_lowest_ties_and_signed_zero_are_diagnostics_only() {
    for (overrides, cpu) in [(vec![(5, 0x3f80), (42, 0x3f80)], 5), (vec![(0, 0x8000)], 0)] {
        let output = Output::new();
        let mut capture = capture(&output, &[1]);
        let mut engine = fixture(1);
        engine.transports[0].logit_overrides = overrides;
        capture.step(&mut engine, 7).unwrap();
        assert_eq!(capture.step(&mut engine, 42).unwrap(), 42);
        engine.close().unwrap();
        capture.finish(&closed()).unwrap();
        assert_eq!(
            output.manifest()["rows"][0]["logits_diagnostics"]["cpu_choice"],
            cpu
        );
    }
}

#[test]
fn capture_sequence_prompt_decode_and_epoch_drift_fail_closed() {
    for mode in 0..3 {
        let output = Output::new();
        let mut capture = capture(&output, &[1]);
        let mut engine = fixture(1);
        if mode != 0 {
            capture.step(&mut engine, 7).unwrap();
        }
        if mode == 2 {
            engine.reset_sequence().unwrap();
            engine.step(7).unwrap();
        }
        let token = if mode == 2 { 42 } else { 8 };
        assert!(capture.step(&mut engine, token).is_err());
        engine.close().unwrap();
        assert!(capture.finish(&closed()).is_err());
        assert!(!output.0.join("manifest.json").exists());
    }
}

#[test]
fn capture_rejects_incomplete_and_unconfirmed_or_mismatched_close() {
    for mode in 0..5 {
        let output = Output::new();
        let mut capture = capture(&output, &[1]);
        let mut engine = fixture(1);
        capture.step(&mut engine, 7).unwrap();
        let mut close = closed();
        if mode != 0 {
            capture.step(&mut engine, 42).unwrap();
        }
        match mode {
            1 => close["all_workers_exited"] = json!(false),
            2 => close["worker_pids"] = json!([124]),
            3 => close["whole_seconds"] = json!(-1),
            4 => close["authority"] = json!("qualified"),
            _ => (),
        }
        engine.close().unwrap();
        assert!(capture.finish(&close).is_err());
        assert!(!output.0.join("manifest.json").exists());
    }
}

#[test]
fn capture_validates_identity_selection_and_output_before_creation() {
    for (field, value) in [
        ("worker_sha256", json!("0".repeat(64))),
        ("running_worker_sha256", json!(["b".repeat(64)])),
        ("device_unique_ids", json!([0])),
        ("worker_pids", json!([1])),
        ("prompt_tokens", json!([true])),
        ("new_tokens", json!(3)),
        ("capacity", json!(1)),
        ("target", json!("gfx942:xnack-")),
        (
            "final_stage_capture",
            json!({"positions":[0],"benchmark_comparable":false}),
        ),
        (
            "final_stage_capture",
            json!({"positions":[1],"benchmark_comparable":true}),
        ),
    ] {
        let output = Output::new();
        let mut value_setup = setup(&[1]);
        value_setup[field] = value;
        assert!(
            Capture::new(&output.0, vec![1], 2, value_setup).is_err(),
            "{field}"
        );
        assert!(!output.0.exists());
    }
    for positions in [vec![], vec![1, 1], vec![1, 0], vec![2], (0..9).collect()] {
        let output = Output::new();
        assert!(Capture::new(&output.0, positions.clone(), 2, setup(&positions)).is_err());
        assert!(!output.0.exists());
    }
    let output = Output::new();
    let _capture = capture(&output, &[1]);
    assert!(Capture::new(&output.0, vec![1], 2, setup(&[1])).is_err());
}

#[test]
fn capture_publication_never_overwrites_existing_manifest() {
    let output = Output::new();
    let mut capture = capture(&output, &[1]);
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
    assert!(capture.finish(&closed()).is_err());
}
