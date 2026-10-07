//! Explicit nine-image decode-only packed-down screen; no performance or serving grant.

use super::{DecodeComposition, Options, Variant, artifact_identity, hex};
use ferric_m1_engineering_execution_v1::tp_artifact::{
    ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1, EngineeringTpPackedDownArtifactR1,
    EngineeringTpPackedDownImageIdsR1,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

pub(super) const PROFILE: &str = "prefill16-decode-ordered64-packed-down-r1-live-v1";
const MAX_ROSTER_BYTES: u64 = 16 * 1024;
const WEIGHT_BYTES: u64 = 3_623_878_656;
const SCRATCH_BYTES: u64 = 24_576;

#[derive(Clone)]
pub(crate) struct PackedDown {
    pub artifact: PathBuf,
    pub roster: PathBuf,
    pub roster_sha256: [u8; 32],
    pub image_ids: EngineeringTpPackedDownImageIdsR1,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Root {
    logical_name: String,
    export_name: String,
}

type Roster = [Root; 2];

pub(super) struct AdmittedPackedDown {
    pub image: EngineeringTpPackedDownArtifactR1,
    roster: Roster,
}

impl PackedDown {
    pub(super) fn mode(&self) -> &'static str {
        if self.enabled {
            "packed-down-u32-r1"
        } else {
            "baseline"
        }
    }

    pub(crate) fn validate(&self, options: &Options, variant: Variant<'_>) -> Result<(), String> {
        variant.validate(options)?;
        if !cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps"),
            not(feature = "c1-token-program")
        )) || !self.artifact.is_absolute()
            || !self.roster.is_absolute()
            || [
                self.roster_sha256,
                self.image_ids.hsaco,
                self.image_ids.manifest,
                self.image_ids.handoff,
            ]
            .contains(&[0; 32])
            || options.live.host_timing.is_some()
            || options.live.runtime.profile
            || options.live.runtime.ordered64_runtime_counters
            || options.live.runtime.ordered64_packet_ticks
            || !matches!(
                variant,
                Variant::PrefillKvCopyV28 {
                    enabled: true,
                    decode: Some(DecodeComposition {
                        split: true,
                        packed: true,
                        ordered64: true,
                        gemv: Some((_, false)),
                        ..
                    }),
                    ..
                }
            )
        {
            return Err("packed down requires explicit image/roster pins, ordinary V27/split8/packed64/baseline-GEMV, and no diagnostics or token-program feature".into());
        }
        Ok(())
    }

    pub(super) fn open(&self) -> Result<AdmittedPackedDown, String> {
        let before = std::fs::symlink_metadata(&self.roster).map_err(|error| error.to_string())?;
        if !before.is_file()
            || before.len() == 0
            || before.len() > MAX_ROSTER_BYTES
            || self
                .roster
                .canonicalize()
                .map_err(|error| error.to_string())?
                != self.roster
        {
            return Err("packed down roster must be a canonical bounded regular file".into());
        }
        let mut file = std::fs::File::open(&self.roster).map_err(|error| error.to_string())?;
        let opened = file.metadata().map_err(|error| error.to_string())?;
        let mut bytes = Vec::new();
        (&mut file)
            .take(MAX_ROSTER_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        let after = std::fs::symlink_metadata(&self.roster).map_err(|error| error.to_string())?;
        let stable = |other: &std::fs::Metadata| {
            other.is_file()
                && (
                    before.dev(),
                    before.ino(),
                    before.len(),
                    before.mtime(),
                    before.mtime_nsec(),
                    before.ctime(),
                    before.ctime_nsec(),
                ) == (
                    other.dev(),
                    other.ino(),
                    other.len(),
                    other.mtime(),
                    other.mtime_nsec(),
                    other.ctime(),
                    other.ctime_nsec(),
                )
        };
        if !stable(&opened) || !stable(&after) || bytes.len() as u64 != before.len() {
            return Err("packed down roster changed while reopening".into());
        }
        let roster = decode_roster(&bytes, self.roster_sha256)?;
        let names = [
            (
                roster[0].logical_name.as_str(),
                roster[0].export_name.as_str(),
            ),
            (
                roster[1].logical_name.as_str(),
                roster[1].export_name.as_str(),
            ),
        ];
        let image = EngineeringTpPackedDownArtifactR1::open(&self.artifact, &names, self.image_ids)
            .map_err(|error| error.to_string())?;
        Ok(AdmittedPackedDown { image, roster })
    }

    pub(super) fn metadata(
        &self,
        admitted: &AdmittedPackedDown,
        actual_mode: &str,
        weight_bytes: u64,
        scratch_bytes: u64,
        sources: &[(u32, u32, [u8; 32])],
    ) -> Result<Value, String> {
        let mut value = self.storage_metadata(actual_mode, weight_bytes, scratch_bytes, sources)?;
        value["artifact_path"] = json!(self.artifact);
        value["artifact"] = artifact_identity(admitted.image.artifact());
        value["compiler_roster"] = json!({
            "path":self.roster, "sha256":hex(&self.roster_sha256), "value":admitted.roster,
            "source":"caller-supplied exact producer roster bytes; independently reviewed before launch",
        });
        Ok(value)
    }

    fn storage_metadata(
        &self,
        actual_mode: &str,
        weight_bytes: u64,
        scratch_bytes: u64,
        sources: &[(u32, u32, [u8; 32])],
    ) -> Result<Value, String> {
        if actual_mode != self.mode()
            || weight_bytes != WEIGHT_BYTES
            || scratch_bytes != SCRATCH_BYTES
            || sources.len() != 36
            || sources
                .iter()
                .enumerate()
                .any(|(ordinal, &(layer, role, digest))| {
                    layer as usize != ordinal || role != 2 || digest == [0; 32]
                })
        {
            return Err(
                "packed down actual mode, storage or authenticated source roster differs".into(),
            );
        }
        let sources = sources
            .iter()
            .map(|(layer, role, digest)| {
                json!({
                    "layer":layer, "role":role, "source_sha256":hex(digest), "shape":[4096,12288],
                    "role_namespace":"fp32-partial-projection", "tensor_kind":"down_projection",
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "schema":"FerricPackedDownLiveSelectionR1", "authority":"none",
            "requested_mode":self.mode(), "actual_mode":actual_mode,
            "additional_weight_bytes":weight_bytes, "activation_scratch_bytes":scratch_bytes,
            "authenticated_weight_sources":sources, "loaded_image_count":9,
            "composition":"V5/V8/V11/V14/V15/V20/V21/V27 plus packed-down-r1; V20 loaded but unselected; no V19",
            "selected_phase":"decode", "selected_rows":1, "selected_published_rows":1, "roles":[2],
            "c1_only":true, "max_simultaneous_selected_requests":1,
            "mixed_request_batches":"rejected_before_submission",
            "multirow_decode_batches":"rejected_before_submission",
            "prefill_unchanged":true, "same_image_set_in_both_arms":true,
            "activation_pack":"one fresh ordered pack after each selected decode SwiGLU",
            "extra_packets_per_selected_forward":if self.enabled { 36 } else { 0 },
            "source_schedule_only":true, "native_qualified":false, "performance_qualified":false,
            "serving_qualified":false,
        }))
    }
}

fn decode_roster(bytes: &[u8], expected: [u8; 32]) -> Result<Roster, String> {
    if bytes.is_empty()
        || bytes.len() as u64 > MAX_ROSTER_BYTES
        || expected == [0; 32]
        || Sha256::digest(bytes).as_slice() != expected
    {
        return Err("packed down roster byte identity differs".into());
    }
    let roster: Roster = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if roster
        .iter()
        .zip(ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1)
        .any(|(root, export)| {
            root.export_name != export
                || root.logical_name.is_empty()
                || root.logical_name.len() > 512
                || !root.logical_name.is_ascii()
                || root
                    .logical_name
                    .bytes()
                    .any(|byte| byte.is_ascii_control())
        })
        || roster[0].logical_name == roster[1].logical_name
    {
        return Err("packed down requires the exact ordered projection/pack compiler roster".into());
    }
    Ok(roster)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection() -> PackedDown {
        PackedDown {
            artifact: "/unused/packed-down".into(),
            roster: "/unused/roster.json".into(),
            roster_sha256: [1; 32],
            image_ids: EngineeringTpPackedDownImageIdsR1 {
                hsaco: [2; 32],
                manifest: [3; 32],
                handoff: [4; 32],
            },
            enabled: true,
        }
    }

    fn variant() -> Variant<'static> {
        Variant::PrefillKvCopyV28 {
            artifact: std::path::Path::new("prefill"),
            enabled: true,
            decode: Some(DecodeComposition {
                artifact: std::path::Path::new("split"),
                split: true,
                packed: true,
                ordered64: true,
                gemv: Some((std::path::Path::new("gemv"), false)),
            }),
        }
    }

    fn roster() -> Value {
        json!([
            {"logical_name":"synthetic_down", "export_name":ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1[0]},
            {"logical_name":"synthetic_pack", "export_name":ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1[1]},
        ])
    }

    #[test]
    fn packed_down_live_roster_is_hash_bound_ordered_bounded_and_closed() {
        let bytes = serde_json::to_vec(&roster()).unwrap();
        decode_roster(&bytes, Sha256::digest(&bytes).into()).unwrap();
        assert!(decode_roster(&bytes, [7; 32]).is_err());
        assert!(decode_roster(&[], [7; 32]).is_err());
        let oversized = vec![b' '; MAX_ROSTER_BYTES as usize + 1];
        assert!(decode_roster(&oversized, Sha256::digest(&oversized).into()).is_err());
        for mutation in 0..10 {
            let mut value = roster();
            match mutation {
                0 => value = json!({"roots":value}),
                1 => value.as_array_mut().unwrap().reverse(),
                2 => value[0]["logical_name"] = value[1]["logical_name"].clone(),
                3 => value[0]["logical_name"] = json!(""),
                4 => value[0]["extra"] = json!(true),
                5 => value.as_array_mut().unwrap().push(json!({})),
                6 => value[0]["logical_name"] = json!("line\nbreak"),
                7 => value[0]["logical_name"] = json!("x".repeat(513)),
                8 => value = json!([]),
                _ => value[0]["export_name"] = json!("wrong"),
            }
            let bytes = serde_json::to_vec(&value).unwrap();
            assert!(decode_roster(&bytes, Sha256::digest(&bytes).into()).is_err());
        }
        let duplicate = br#"[{"logical_name":"x","logical_name":"y","export_name":"unused"},{}]"#;
        assert!(decode_roster(duplicate, Sha256::digest(duplicate).into()).is_err());
    }

    #[test]
    fn packed_down_live_requires_ordinary_ordered64_and_closed_composition() {
        let options = super::super::super::wave_target_v17_live_contract::fixture(
            super::super::super::wave_target_v17_live_contract::Mode::Combined,
        );
        assert_eq!(
            selection().validate(&options, variant()).is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps"),
                not(feature = "c1-token-program")
            ))
        );
        assert!(selection().validate(&options, Variant::V17).is_err());
        for mutation in 0..6 {
            let mut changed = variant();
            let Variant::PrefillKvCopyV28 {
                enabled,
                decode: Some(decode),
                ..
            } = &mut changed
            else {
                unreachable!()
            };
            match mutation {
                0 => *enabled = false,
                1 => decode.split = false,
                2 => decode.packed = false,
                3 => decode.ordered64 = false,
                4 => decode.gemv = None,
                _ => decode.gemv = Some((std::path::Path::new("gemv"), true)),
            }
            assert!(selection().validate(&options, changed).is_err());
        }
        assert_ne!(PROFILE, variant().live_profile());
        assert_ne!(PROFILE, super::super::ORDERED64_KV_COPY_PROFILE);
        assert_ne!(PROFILE, super::super::packed_gate_up_live_profile::PROFILE);
    }

    #[test]
    fn packed_down_live_metadata_keeps_same_images_storage_and_36_source_order_in_both_arms() {
        let sources = (0..36)
            .map(|layer| (layer, 2, [5; 32]))
            .collect::<Vec<_>>();
        for enabled in [false, true] {
            let input = PackedDown {
                enabled,
                ..selection()
            };
            let value = input
                .storage_metadata(input.mode(), WEIGHT_BYTES, SCRATCH_BYTES, &sources)
                .unwrap();
            assert_eq!(value["loaded_image_count"], 9);
            assert_eq!(value["selected_phase"], "decode");
            assert_eq!(value["c1_only"], true);
            assert_eq!(value["max_simultaneous_selected_requests"], 1);
            assert_eq!(value["mixed_request_batches"], "rejected_before_submission");
            assert_eq!(value["multirow_decode_batches"], "rejected_before_submission");
            assert_eq!(value["prefill_unchanged"], true);
            assert_eq!(value["same_image_set_in_both_arms"], true);
            assert_eq!(value["additional_weight_bytes"], 3_623_878_656_u64);
            assert_eq!(value["activation_scratch_bytes"], 24_576);
            assert_eq!(
                value["extra_packets_per_selected_forward"],
                if enabled { 36 } else { 0 }
            );
            let rows = value["authenticated_weight_sources"].as_array().unwrap();
            assert_eq!(rows.len(), 36);
            for (layer, row) in rows.iter().enumerate() {
                assert_eq!(row["layer"], layer);
                assert_eq!(row["role"], 2);
                assert_eq!(row["role_namespace"], "fp32-partial-projection");
                assert_eq!(row["tensor_kind"], "down_projection");
                assert_eq!(row["shape"], json!([4096, 12288]));
            }
            for (mode, weights, scratch, count) in [
                ("wrong", WEIGHT_BYTES, SCRATCH_BYTES, 36),
                (input.mode(), WEIGHT_BYTES - 1, SCRATCH_BYTES, 36),
                (input.mode(), WEIGHT_BYTES, SCRATCH_BYTES - 1, 36),
                (input.mode(), WEIGHT_BYTES, SCRATCH_BYTES, 35),
            ] {
                assert!(
                    input
                        .storage_metadata(mode, weights, scratch, &sources[..count])
                        .is_err()
                );
            }
            for mutation in 0..4 {
                let mut wrong = sources.clone();
                match mutation {
                    0 => wrong.swap(0, 1),
                    1 => wrong[0].1 = 4,
                    2 => wrong[35].0 = 36,
                    _ => wrong[0].2 = [0; 32],
                }
                assert!(
                    input
                        .storage_metadata(input.mode(), WEIGHT_BYTES, SCRATCH_BYTES, &wrong)
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn packed_down_live_rejects_bad_pins_and_every_instrumentation_flag_before_open() {
        let options = super::super::super::wave_target_v17_live_contract::fixture(
            super::super::super::wave_target_v17_live_contract::Mode::Combined,
        );
        for mutation in 0..6 {
            let mut input = selection();
            match mutation {
                0 => input.roster_sha256 = [0; 32],
                1 => input.image_ids.hsaco = [0; 32],
                2 => input.image_ids.manifest = [0; 32],
                3 => input.image_ids.handoff = [0; 32],
                4 => input.artifact = "relative".into(),
                _ => input.roster = "relative".into(),
            }
            assert!(input.validate(&options, variant()).is_err());
        }
        for mutation in 0..4 {
            let mut altered = options.clone();
            match mutation {
                0 => altered.live.host_timing = Some("unused".into()),
                1 => altered.live.runtime.profile = true,
                2 => altered.live.runtime.ordered64_runtime_counters = true,
                _ => altered.live.runtime.ordered64_packet_ticks = true,
            }
            assert!(selection().validate(&altered, variant()).is_err());
        }
    }

    #[test]
    #[cfg(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps"),
        not(feature = "c1-token-program")
    ))]
    fn packed_down_live_both_arms_keep_uninstrumented_ordinary_runtime() {
        let options = super::super::super::wave_target_v17_live_contract::fixture(
            super::super::super::wave_target_v17_live_contract::Mode::Combined,
        );
        for enabled in [false, true] {
            PackedDown {
                enabled,
                ..selection()
            }
            .validate(&options, variant())
            .unwrap();
            let runtime = variant().worker_runtime(&options).unwrap();
            assert!(runtime.ordered64 && runtime.ordered_batches);
            assert!(!runtime.profile);
            assert!(!runtime.ordered64_runtime_counters);
            assert!(!runtime.ordered64_packet_ticks);
            assert!(!runtime.sequences);
            assert!(!runtime.shared_full_currentness);
        }
    }

    #[test]
    #[cfg(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps"),
        not(feature = "c1-token-program")
    ))]
    fn packed_down_live_runner_rejects_mixed_modes_before_opening_worker_or_model() {
        use super::super::{ExecutionMode, Ordered64KvCopy, TimingFile, run_with_timing};
        use std::path::Path;

        let options = super::super::super::wave_target_v17_live_contract::fixture(
            super::super::super::wave_target_v17_live_contract::Mode::Combined,
        );
        let input = selection();
        for case in 0..4 {
            let mut timing = TimingFile::create(None).unwrap();
            let copy = (case < 2).then_some(Ordered64KvCopy {
                artifact: Path::new("unused-copy"),
                enabled: case == 1,
                prefill32_pages: None,
            });
            let error = run_with_timing(
                &options,
                &mut timing,
                variant(),
                (case == 2).then_some(Path::new("unused-timestamps")),
                case == 3,
                copy,
                ExecutionMode::PackedDown(&input),
            )
            .unwrap_err();
            assert_eq!(error, "packed down excludes V19, prefill32 and all instrumentation");
        }
    }

    #[test]
    #[ignore = "requires qualified packed-down R1 image and independently reviewed producer-roster pins"]
    fn packed_down_live_actual_image_reopens_with_exact_profile_identity_in_both_arms() {
        let value = |key: &str| std::env::var(key).expect("explicit actual packed-down R1 test input");
        let digest = |key: &str| super::super::packed_digest_argument(&value(key)).unwrap();
        let input = PackedDown {
            artifact: value("FERRIC_TEST_PACKED_DOWN_R1_ARTIFACT").into(),
            roster: value("FERRIC_TEST_PACKED_DOWN_R1_ROSTER").into(),
            roster_sha256: digest("FERRIC_TEST_PACKED_DOWN_R1_ROSTER_SHA256"),
            image_ids: EngineeringTpPackedDownImageIdsR1 {
                hsaco: digest("FERRIC_TEST_PACKED_DOWN_R1_HSACO_SHA256"),
                manifest: digest("FERRIC_TEST_PACKED_DOWN_R1_MANIFEST_SHA256"),
                handoff: digest("FERRIC_TEST_PACKED_DOWN_R1_HANDOFF_SHA256"),
            },
            enabled: true,
        };
        let actual = input.open().unwrap();
        let expected = json!({
            "artifact_hsaco_id":hex(&input.image_ids.hsaco),
            "artifact_manifest_id":hex(&input.image_ids.manifest),
            "artifact_handoff_id":hex(&input.image_ids.handoff),
        });
        let sources = (0..36)
            .map(|layer| (layer, 2, [5; 32]))
            .collect::<Vec<_>>();
        for enabled in [false, true] {
            let selection = PackedDown {
                enabled,
                ..input.clone()
            };
            let observed = selection
                .metadata(
                    &actual,
                    selection.mode(),
                    WEIGHT_BYTES,
                    SCRATCH_BYTES,
                    &sources,
                )
                .unwrap();
            assert_eq!(observed["artifact"], expected);
            assert_eq!(
                observed["compiler_roster"]["sha256"],
                hex(&input.roster_sha256)
            );
            assert_eq!(observed["loaded_image_count"], 9);
        }
        let wrong = PackedDown {
            roster_sha256: [0; 32],
            ..input
        };
        assert!(wrong.open().is_err());
    }
}
