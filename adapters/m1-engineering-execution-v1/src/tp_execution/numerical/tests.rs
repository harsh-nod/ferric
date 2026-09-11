use super::*;
use crate::tp_paged::{
    EngineeringTpPageRowV1, EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1,
    EngineeringTpPoolScopeV1,
};
use crate::tp_scheduler::TpRequestIdV1;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SERIAL: AtomicU64 = AtomicU64::new(0);

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "ferric-numerical-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn identity(mfma: bool) -> Value {
    let mut value = json!({"tensor_parallel":1,"projection":if mfma {"mfma"} else {"baseline"},"output_head_pruning":false,"device_unique_id":1});
    for key in [
        "controller_sha256",
        "worker_sha256",
        "artifact_hsaco_id",
        "artifact_manifest_id",
        "artifact_handoff_id",
        "model_bundle_id",
        "requests_sha256",
        "session_id",
    ] {
        value[key] = json!("ab".repeat(32));
    }
    value
}

fn capture(path: &Path, mfma: bool) -> EngineeringTpNumericalCaptureV1 {
    EngineeringTpNumericalCaptureV1::new(
        path,
        2,
        0,
        EngineeringTpNumericalProjectionV1::Key,
        identity(mfma),
    )
    .unwrap()
}

fn plan() -> Vec<TpBatchRowV1> {
    vec![TpBatchRowV1 {
        request: TpRequestIdV1 {
            slot: 3,
            generation: 2,
        },
        token_id: 42,
        absolute_position: 0,
        kind: TpBatchRowKindV1::PrefillFinal,
    }]
}

fn prepared() -> EngineeringTpPreparedBatchV1 {
    let scope = EngineeringTpPoolScopeV1 {
        model: [1; 32],
        session: [2; 32],
    };
    let mut pool = EngineeringTpPagedPoolV1::new(
        scope,
        EngineeringTpPagedLimitsV1::new(32, 2, 4, 100).unwrap(),
    )
    .unwrap();
    let hit = pool.open_sequence(scope, &[42], 0).unwrap();
    pool.reserve_batch(&[EngineeringTpPageRowV1 {
        sequence: hit.sequence(),
        token: 42,
        position: 0,
    }])
    .unwrap()
}

fn buffer(id: u64, elements: usize, width: u32, access: Access) -> Argument {
    Argument::Buffer {
        id,
        offset: 0,
        elements,
        element_bytes: width,
        access,
    }
}

fn command(n: u32, tag: u32, mfma: bool) -> EngineeringTpDispatchV1 {
    EngineeringTpDispatchV1 {
        kernel: if mfma {
            "ferric_qwen3_tp_mfma_gemm_bf16_v3"
        } else {
            "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2"
        },
        grid_workgroups: n / 16,
        workgroup_size: 64,
        arguments: vec![
            buffer(1, 4096, 2, Access::Read),
            buffer(if mfma { 4 } else { 2 }, n as usize * 4096, 2, Access::Read),
            buffer(3, n as usize, 2, Access::Write),
            Argument::U32(1),
            Argument::U32(n),
            Argument::U32(4096),
            Argument::U32(1),
            Argument::U32(tag),
        ],
    }
}

#[derive(Default)]
struct Zeros {
    reads: usize,
    fail_after: Option<usize>,
}
impl EngineeringTpRankTransportV1 for Zeros {
    fn allocate(&mut self, _: usize) -> TpResult<u64> {
        Err("unexpected allocation".into())
    }
    fn write(&mut self, _: u64, _: usize, _: &[u8]) -> TpResult<()> {
        Err("unexpected write".into())
    }
    fn read(&mut self, _: u64, _: usize, bytes: &mut [u8]) -> TpResult<()> {
        self.reads += 1;
        if self.fail_after == Some(self.reads) {
            return Err("injected read failure".into());
        }
        bytes.fill(0);
        Ok(())
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        Err("unexpected dispatch".into())
    }
    fn wait(&mut self) -> TpResult<()> {
        Err("unexpected wait".into())
    }
    fn close(&mut self) -> TpResult<()> {
        Ok(())
    }
}

#[test]
fn fresh_private_identity_bound_directory_and_exact_cap() {
    let dir = Directory::new();
    let mut owner = capture(&dir.0, false);
    assert_eq!(
        std::fs::metadata(&dir.0).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(
        EngineeringTpNumericalCaptureV1::new(
            &dir.0,
            2,
            0,
            EngineeringTpNumericalProjectionV1::Key,
            identity(false)
        )
        .is_err()
    );
    assert!(owner.finish().is_err());
    owner.bytes = MAX_BYTES - 1;
    assert_eq!(owner.reserve(1).unwrap(), MAX_BYTES);
    assert!(owner.reserve(2).is_err());
    assert!(owner.reserve(usize::MAX).is_err());
    for key in ["controller_sha256", "requests_sha256", "session_id"] {
        let mut value = identity(false);
        value[key] = json!("00".repeat(32));
        assert!(
            EngineeringTpNumericalCaptureV1::new(
                &Directory::new().0,
                2,
                0,
                EngineeringTpNumericalProjectionV1::Key,
                value
            )
            .is_err()
        );
    }
}

#[test]
fn exact_scheduler_binding_rejects_rebind_and_row_drift() {
    let dir = Directory::new();
    let mut owner = capture(&dir.0, false);
    owner.bind_rows(1, 0, &[]).unwrap();
    assert!(owner.plan.is_none());
    owner.bind_rows(2, 19, &plan()).unwrap();
    assert!(owner.begin_batch(2, &prepared(), &[0], &[]).is_err());
    assert!(owner.finish().is_err());
    let dir = Directory::new();
    let mut owner = capture(&dir.0, false);
    owner.bind_rows(2, 19, &plan()).unwrap();
    assert!(owner.bind_rows(2, 20, &plan()).is_err());
    let dir = Directory::new();
    let mut owner = capture(&dir.0, false);
    owner.bind_rows(2, 19, &plan()).unwrap();
    assert!(owner.begin_batch(2, &prepared(), &[1], &[0]).is_err());
}

#[test]
fn pruned_execution_rows_preserve_two_request_generations_and_positions() {
    let scope = EngineeringTpPoolScopeV1 {
        model: [1; 32],
        session: [2; 32],
    };
    let mut pool = EngineeringTpPagedPoolV1::new(
        scope,
        EngineeringTpPagedLimitsV1::new(32, 2, 4, 100).unwrap(),
    )
    .unwrap();
    let mut pages = Vec::new();
    let mut planned = Vec::new();
    for slot in 0..2u8 {
        let token = 40 + u32::from(slot);
        let hit = pool.open_sequence(scope, &[token, token + 1], 0).unwrap();
        for position in 0..2 {
            pages.push(EngineeringTpPageRowV1 {
                sequence: hit.sequence(),
                token: token + position,
                position,
            });
            planned.push(TpBatchRowV1 {
                request: TpRequestIdV1 {
                    slot,
                    generation: u64::from(slot) + 3,
                },
                token_id: token + position,
                absolute_position: position,
                kind: if position == 0 {
                    TpBatchRowKindV1::PrefillIntermediate
                } else {
                    TpBatchRowKindV1::PrefillFinal
                },
            });
        }
    }
    let batch = pool.reserve_batch(&pages).unwrap();
    let dir = Directory::new();
    let mut owner = capture(&dir.0, false);
    owner.bind_rows(2, 19, &planned).unwrap();
    owner
        .begin_batch(2, &batch, &[1, 3, 0, 2], &[1, 3])
        .unwrap();
    for (physical, source) in [1usize, 3, 0, 2].into_iter().enumerate() {
        assert_eq!(owner.row_map[physical]["source_row"], source);
        assert_eq!(
            owner.row_map[physical]["slot"],
            planned[source].request.slot
        );
        assert_eq!(
            owner.row_map[physical]["generation"],
            planned[source].request.generation
        );
        assert_eq!(
            owner.row_map[physical]["position"],
            planned[source].absolute_position
        );
    }
}

#[test]
fn exact_projection_shapes_layout_and_active_bounds() {
    for mfma in [false, true] {
        let mut cmd = command(1024, 2, mfma);
        let parsed = Projection::parse(&cmd, (1024, 4096, 2, false)).unwrap();
        assert_eq!(parsed.transposed, mfma);
        cmd.grid_workgroups += 1;
        assert!(Projection::parse(&cmd, (1024, 4096, 2, false)).is_err());
        cmd.grid_workgroups -= 1;
        cmd.arguments[6] = Argument::U32(2);
        assert!(Projection::parse(&cmd, (1024, 4096, 2, false)).is_err());
    }
    assert!(Buffer::parse(&buffer(1, usize::MAX, 2, Access::Read), Access::Read, 2).is_err());
    assert!(Buffer::parse(&buffer(0, 1, 2, Access::Read), Access::Read, 2).is_err());
    assert!(Buffer::parse(&buffer(1, 1, 2, Access::Write), Access::Read, 2).is_err());
}

#[test]
fn successful_capture_streams_both_layouts_and_hashes_every_file() {
    for mfma in [false, true] {
        let dir = Directory::new();
        let mut owner = capture(&dir.0, mfma);
        let mut transport = Zeros::default();
        owner.bind_rows(2, 19, &plan()).unwrap();
        owner.begin_batch(2, &prepared(), &[0], &[0]).unwrap();
        owner
            .capture_projection(
                1,
                0,
                EngineeringTpNumericalProjectionV1::Key,
                &command(1024, 2, mfma),
                buffer(2, 1024 * 4096, 2, Access::Read),
                &mut transport,
            )
            .unwrap();
        assert_eq!(transport.reads, 0);
        owner
            .capture_projection(
                2,
                0,
                EngineeringTpNumericalProjectionV1::Key,
                &command(1024, 2, mfma),
                buffer(2, 1024 * 4096, 2, Access::Read),
                &mut transport,
            )
            .unwrap();
        owner
            .capture_head(
                2,
                &command(151_936, 6, mfma),
                buffer(2, VOCABULARY * HIDDEN, 2, Access::Read),
                &[0],
                &mut transport,
            )
            .unwrap();
        assert!(!dir.0.join("manifest.json").exists());
        let receipt = owner.finish().unwrap();
        assert!(owner.finish().is_err());
        let bytes = std::fs::read(dir.0.join("manifest.json")).unwrap();
        assert_eq!(receipt["manifest"]["sha256"], hex(&Sha256::digest(&bytes)));
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest["execution_rows"][0]["slot"], 3);
        assert_eq!(manifest["execution_rows"][0]["generation"], 2);
        assert_eq!(manifest["performance_qualified"], false);
        let mut total = 0;
        for file in std::fs::read_dir(&dir.0).unwrap() {
            let file = file.unwrap();
            total += file.metadata().unwrap().len();
            assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert_eq!(receipt["total_bytes"].as_u64(), Some(total));
        assert_eq!(dir.0.join("projection-weights-kn.bf16").exists(), mfma);
    }
}

#[test]
fn failed_read_and_argmax_never_publish_success_manifest() {
    for read_failure in [true, false] {
        let dir = Directory::new();
        let mut owner = capture(&dir.0, false);
        owner.bind_rows(2, 19, &plan()).unwrap();
        owner.begin_batch(2, &prepared(), &[0], &[0]).unwrap();
        let mut transport = Zeros {
            fail_after: read_failure.then_some(2),
            ..Zeros::default()
        };
        assert!(
            owner
                .capture_head(
                    2,
                    &command(151_936, 6, false),
                    buffer(2, VOCABULARY * HIDDEN, 2, Access::Read),
                    &[1],
                    &mut transport
                )
                .is_err()
        );
        assert!(owner.finish().is_err());
        assert!(!dir.0.join("manifest.json").exists());
    }
}

#[test]
fn full_logits_lowest_id_ties_watch_bits_and_nonfinite() {
    let mut data = vec![0; VOCABULARY * 2];
    for token in [9856usize, 17689] {
        data[token * 2..token * 2 + 2].copy_from_slice(&0x3f80u16.to_le_bytes());
    }
    let (summary, selected) = summarize_logits(&data, &[9856], &[json!({"slot":1})]).unwrap();
    assert_eq!(summary[0]["top_two_tied"], true);
    assert_eq!(summary[0]["top16"][1]["token"], 17689);
    assert!(selected.contains(&9856) && selected.contains(&17689));
    assert!(summarize_logits(&data, &[17689], &[json!({})]).is_err());
    data[..2].copy_from_slice(&0x7f80u16.to_le_bytes());
    assert!(summarize_logits(&data, &[9856], &[json!({})]).is_err());
}
