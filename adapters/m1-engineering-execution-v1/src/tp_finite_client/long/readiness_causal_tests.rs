use super::*;
use crate::tp_finite_client::long::FilePin;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    directory: PathBuf,
    header: Value,
    payload: Vec<u8>,
    bootstrap: ready::Bootstrap,
    files: retained::Files,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "ferric-causal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let directory = directory.canonicalize().unwrap();
        let bootstrap = ready::Bootstrap {
            schema: ready::POSITION5_SCHEMA.into(),
            child_deadline_ms: 60000,
            sequence: crate::finite_guarded_mlp_long_wire_v2::tests::bootstrap(
                long::Profile::Readiness40Position5,
            ),
        };
        let mut captures = vec![];
        let mut payload = vec![];
        // Independent flattened ABI census, intentionally not stages().
        let groups = [
            (
                "before_prefix",
                vec![
                    ("input", "bf16", 8192),
                    ("cache_metadata", "u32", 580),
                    ("rotary", "f32", 512),
                ],
            ),
            (
                "after_prefix",
                vec![
                    ("input_normalized", "bf16", 8192),
                    ("raw_qkv", "bf16", 6144),
                    ("query", "bf16", 4096),
                    ("used_key", "bf16", 1024),
                    ("used_value", "bf16", 1024),
                    ("attention", "bf16", 4096),
                    ("output_partial", "f32", 16384),
                ],
            ),
            (
                "after_first_residual",
                vec![("first_residual", "bf16", 8192)],
            ),
            (
                "after_mlp",
                vec![
                    ("post_normalized", "bf16", 8192),
                    ("gate", "bf16", 12288),
                    ("up", "bf16", 12288),
                    ("activation", "bf16", 12288),
                    ("down_partial", "f32", 16384),
                ],
            ),
            ("after_final_residual", vec![("final_hidden", "bf16", 8192)]),
        ];
        for position in 0..6u32 {
            let mut data = vec![];
            let mut parts = vec![];
            for (boundary, rows) in &groups {
                for rank in 0..2 {
                    for &(role, scalar, base) in rows {
                        let cache = role == "used_key" || role == "used_value";
                        let count = if cache { base * (position + 1) } else { base };
                        let bytes = if role == "cache_metadata" {
                            std::iter::once(position)
                                .chain(0..144)
                                .flat_map(u32::to_le_bytes)
                                .collect::<Vec<_>>()
                        } else {
                            vec![0; count as usize]
                        };
                        parts.push(json!({"boundary":boundary,"rank":rank,"role":role,"scalar":scalar,
                            "elements":count/if scalar=="bf16" {2} else {4},"offset":data.len(),"bytes":count,
                            "source_byte_offset":0,"sha256":hash(&bytes)}));
                        data.extend(bytes);
                    }
                }
            }
            captures.push(
                json!({"generation":position+1,"position":position,"layer":0,"parts":parts,
                "payload_bytes":data.len(),"payload_sha256":hash(&data)}),
            );
            payload.extend(data);
        }
        let header = json!({"schema":"FerricReadiness40CausalLayerZeroV1","bootstrap":bootstrap,
            "transcript_sha256":vec![2u8;32],"captures":captures,"payload_bytes":TOTAL,"payload_sha256":hash(&payload),
            "cache_layout":"used-prefix-token-head-channel-bf16","native_close_confirmed":true,
            "numerical_acceptance":false,"performance_claim":false,"production_authority":false});
        let mut selected = vec![];
        for position in [0, 5] {
            let raw = vec![0; crate::finite_guarded_mlp_decode_wire_v1::CONTROL_BYTES + 606976];
            let path = directory.join(format!("capture-{position}.bin"));
            std::fs::write(&path, &raw).unwrap();
            selected.push(retained::Capture {
                position,
                file: FilePin {
                    path,
                    bytes: raw.len() as u64,
                    sha256: hash(&raw),
                },
            });
        }
        let empty = FilePin {
            path: directory.join("unused"),
            bytes: 0,
            sha256: hash(&[]),
        };
        let files = retained::Files {
            frames: empty.clone(),
            captures: selected,
            child_stderr: empty,
            rows: 40,
            bytes_before_summary: 0,
            summary_bytes: 0,
            total_bytes: 0,
            supervisor_metadata_allowance: 0,
        };
        Self {
            directory,
            header,
            payload,
            bootstrap,
            files,
        }
    }
    fn raw(&self) -> Vec<u8> {
        let head = serde_json::to_vec(&self.header).unwrap();
        let mut out = MAGIC.to_vec();
        out.extend((head.len() as u32).to_le_bytes());
        out.extend((self.payload.len() as u32).to_le_bytes());
        out.extend(head);
        out.extend(&self.payload);
        out
    }
    fn check(&self) -> Result<Summary> {
        validate(
            &self.raw(),
            &self.bootstrap,
            [2; 32],
            &(0..144).collect::<Vec<_>>(),
            &self.files,
        )
    }
}

#[test]
fn causal_parent_admits_only_complete_same_run_closed_sidecar() {
    let f = Fixture::new();
    let summary = f.check().unwrap();
    assert_eq!(summary.positions, [0, 1, 2, 3, 4, 5]);
    assert_eq!(summary.parts, 204);
    assert_eq!(summary.payload_bytes, 1598256);
    assert!(!summary.numerical_acceptance && !summary.performance_claim);
    assert!(summary.sidecar_bytes < 2 << 20);
}
#[test]
fn causal_parent_rejects_identity_claim_roster_and_boundary_changes() {
    for field in [
        "native_close_confirmed",
        "numerical_acceptance",
        "performance_claim",
        "production_authority",
    ] {
        let mut f = Fixture::new();
        f.header[field] = (!f.header[field].as_bool().unwrap()).into();
        assert!(f.check().is_err());
    }
    let mut f = Fixture::new();
    f.header["captures"][1]["position"] = 5.into();
    assert!(f.check().is_err());
    let mut f = Fixture::new();
    f.header["captures"][0]["parts"][0]["role"] = "attention".into();
    assert!(f.check().is_err());
    let mut f = Fixture::new();
    f.header["bootstrap"]["sequence"]["scope"]["session"][0] = 99.into();
    assert!(f.check().is_err());
    let mut f = Fixture::new();
    f.header["unknown_claim"] = true.into();
    assert!(f.check().is_err());
}
#[test]
fn causal_parent_rejects_truncation_extra_bytes_nonfinite_and_selected_output_drift() {
    let f = Fixture::new();
    let mut bytes = f.raw();
    bytes.pop();
    assert!(
        validate(
            &bytes,
            &f.bootstrap,
            [2; 32],
            &(0..144).collect::<Vec<_>>(),
            &f.files
        )
        .is_err()
    );
    let mut bytes = f.raw();
    bytes.push(0);
    assert!(
        validate(
            &bytes,
            &f.bootstrap,
            [2; 32],
            &(0..144).collect::<Vec<_>>(),
            &f.files
        )
        .is_err()
    );
    let mut f = Fixture::new();
    f.payload[..2].copy_from_slice(&0x7fc1u16.to_le_bytes());
    f.header["captures"][0]["parts"][0]["sha256"] = json!(hash(&f.payload[..8192]));
    f.header["captures"][0]["payload_sha256"] = json!(hash(&f.payload[..256136]));
    f.header["payload_sha256"] = json!(hash(&f.payload));
    assert!(f.check().is_err());
    let mut f = Fixture::new();
    let raw = vec![1; crate::finite_guarded_mlp_decode_wire_v1::CONTROL_BYTES + 606976];
    std::fs::write(&f.files.captures[0].file.path, &raw).unwrap();
    f.files.captures[0].file.sha256 = hash(&raw);
    assert!(f.check().is_err());
}

#[test]
fn causal_parent_file_backed_sidecar_reads_and_rechecks_pinned_bytes() {
    let mut f = Fixture::new();
    let raw = f.raw();
    let path = f.directory.join("child-stderr.bin");
    std::fs::write(&path, &raw).unwrap();
    f.files.child_stderr = FilePin {
        path: path.clone(),
        bytes: raw.len() as u64,
        sha256: hash(&raw),
    };
    let pages: Vec<u32> = (0..144).collect();
    let expected = f.check().unwrap();
    assert_eq!(
        validate_file(&f.bootstrap, [2; 32], &pages, &f.files).unwrap(),
        expected
    );

    let mut changed = raw.clone();
    *changed.last_mut().unwrap() ^= 1;
    std::fs::write(&path, &changed).unwrap();
    assert!(validate_file(&f.bootstrap, [2; 32], &pages, &f.files).is_err());

    std::fs::write(&path, &raw).unwrap();
    f.files.child_stderr.sha256[0] ^= 1;
    assert!(validate_file(&f.bootstrap, [2; 32], &pages, &f.files).is_err());
    f.files.child_stderr.sha256 = hash(&raw);
    std::fs::remove_file(&path).unwrap();
    assert!(validate_file(&f.bootstrap, [2; 32], &pages, &f.files).is_err());
}
