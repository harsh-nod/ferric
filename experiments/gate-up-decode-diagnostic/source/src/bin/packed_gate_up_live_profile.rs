//! Closed ten-image V19 plus packed gate/up screen; future image identities stay explicit.

use super::{DecodeComposition, Options, Variant, artifact_identity, hex};
use ferric_m1_engineering_execution_v1::tp_artifact::{
    ENGINEERING_TP_PACKED_BF16_EXPORTS_R2, EngineeringTpPackedBf16ArtifactR2,
    EngineeringTpPackedBf16ImageIdsR2,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

pub(super) const PROFILE: &str = "prefill16-decode-ordered64-kv-packed-gate-up-r2-live-v1";
pub(super) const HOST_DIAGNOSTIC_PROFILE: &str =
    "prefill16-decode-ordered64-kv-packed-gate-up-r2-host-diagnostic-v1";
const ROSTER_SCHEMA: &str = "FerricPackedBf16CompilerRosterR2";
const MAX_ROSTER_BYTES: u64 = 16 * 1024;
const WEIGHT_BYTES: u64 = 7_247_757_312;
const SCRATCH_BYTES: u64 = 8192;

#[derive(Clone)]
pub(crate) struct PackedGateUp {
    pub artifact: PathBuf,
    pub roster: PathBuf,
    pub roster_sha256: [u8; 32],
    pub image_ids: EngineeringTpPackedBf16ImageIdsR2,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Root {
    logical_name: String,
    export_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Roster {
    schema: String,
    roots: [Root; 2],
}

pub(super) struct AdmittedPacked {
    pub image: EngineeringTpPackedBf16ArtifactR2,
    roster: Roster,
}

impl PackedGateUp {
    pub(super) fn mode(&self) -> &'static str {
        if self.enabled {
            "packed-gate-up-u32-r2"
        } else {
            "baseline"
        }
    }

    pub(crate) fn validate(&self, options: &Options, variant: Variant<'_>) -> Result<(), String> {
        variant.validate(options)?;
        if !cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
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
            return Err("packed gate/up requires explicit image/roster pins, V27/split8/packed64/baseline-GEMV with V19, and no diagnostics".into());
        }
        Ok(())
    }

    pub(super) fn open(&self) -> Result<AdmittedPacked, String> {
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
            return Err("packed gate/up roster must be a canonical bounded regular file".into());
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
            return Err("packed gate/up roster changed while reopening".into());
        }
        let roster = decode_roster(&bytes, self.roster_sha256)?;
        let names = [
            (
                roster.roots[0].logical_name.as_str(),
                roster.roots[0].export_name.as_str(),
            ),
            (
                roster.roots[1].logical_name.as_str(),
                roster.roots[1].export_name.as_str(),
            ),
        ];
        let image = EngineeringTpPackedBf16ArtifactR2::open(&self.artifact, &names, self.image_ids)
            .map_err(|error| error.to_string())?;
        Ok(AdmittedPacked { image, roster })
    }

    pub(super) fn metadata(
        &self,
        admitted: &AdmittedPacked,
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
            "source":"caller-supplied actual compiler roster; independently reviewed before launch",
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
            || sources.len() != 72
            || sources
                .iter()
                .enumerate()
                .any(|(ordinal, &(layer, role, _))| {
                    layer as usize != ordinal / 2
                        || role != if ordinal.is_multiple_of(2) { 4 } else { 5 }
                })
        {
            return Err(
                "packed gate/up actual mode, storage or authenticated source roster differs".into(),
            );
        }
        let sources = sources
            .iter()
            .map(|(layer, role, digest)| {
                json!({
                    "layer":layer, "role":role, "source_sha256":hex(digest), "shape":[12288,4096],
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "schema":"FerricPackedGateUpKvLiveSelectionR2", "authority":"none",
            "requested_mode":self.mode(), "actual_mode":actual_mode,
            "additional_weight_bytes":weight_bytes, "activation_scratch_bytes":scratch_bytes,
            "authenticated_weight_sources":sources, "loaded_image_count":10,
            "composition":"V5/V8/V11/V14/V15/V19/V20/V21/V27 plus packed-BF16-r2; V20 loaded but unselected",
            "selected_rows":1, "selected_published_rows":1, "roles":[4,5],
            "activation_pack":"one fresh ordered pack after each selected post-attention RMSNorm",
            "extra_packets_per_selected_forward":if self.enabled { 36 } else { 0 },
            "ffn_producer_packets_per_selected_layer":if self.enabled { 6 } else { 5 },
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
        return Err("packed gate/up roster byte identity differs".into());
    }
    let roster: Roster = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if roster.schema != ROSTER_SCHEMA
        || roster
            .roots
            .iter()
            .zip(ENGINEERING_TP_PACKED_BF16_EXPORTS_R2)
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
        || roster.roots[0].logical_name == roster.roots[1].logical_name
    {
        return Err(
            "packed gate/up requires the exact ordered projection/pack compiler roster".into(),
        );
    }
    Ok(roster)
}

#[allow(dead_code)] // Parsed only by the dedicated packed executable.
pub(crate) fn digest_argument(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("packed gate/up identities must be 64 lowercase hexadecimal digits".into());
    }
    let mut result = [0; 32];
    for (index, output) in result.iter_mut().enumerate() {
        *output = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|error| error.to_string())?;
    }
    if result == [0; 32] {
        return Err("packed gate/up identity cannot be a zero placeholder".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection() -> PackedGateUp {
        PackedGateUp {
            artifact: "/unused/packed".into(),
            roster: "/unused/roster.json".into(),
            roster_sha256: [1; 32],
            image_ids: EngineeringTpPackedBf16ImageIdsR2 {
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
        json!({"schema":ROSTER_SCHEMA,"roots":[
            {"logical_name":"synthetic_projection", "export_name":ENGINEERING_TP_PACKED_BF16_EXPORTS_R2[0]},
            {"logical_name":"synthetic_pack", "export_name":ENGINEERING_TP_PACKED_BF16_EXPORTS_R2[1]},
        ]})
    }

    #[test]
    fn packed_live_identity_parser_has_no_placeholder_or_normalization_path() {
        assert_eq!(digest_argument(&"ab".repeat(32)).unwrap(), [0xab; 32]);
        for value in [
            "0".repeat(64),
            "AB".repeat(32),
            "ab".repeat(31),
            "x".repeat(64),
            " ".repeat(64),
        ] {
            assert!(digest_argument(&value).is_err());
        }
    }

    #[test]
    fn packed_live_roster_is_hash_bound_ordered_closed_and_not_a_manifest_derived_guess() {
        let value = roster();
        let bytes = serde_json::to_vec(&value).unwrap();
        let digest = Sha256::digest(&bytes).into();
        decode_roster(&bytes, digest).unwrap();
        assert!(decode_roster(&bytes, [7; 32]).is_err());
        for mutation in 0..6 {
            let mut value = roster();
            match mutation {
                0 => value["schema"] = json!("old"),
                1 => value["roots"].as_array_mut().unwrap().reverse(),
                2 => value["roots"][0]["logical_name"] = value["roots"][1]["logical_name"].clone(),
                3 => value["roots"][0]["logical_name"] = json!(""),
                4 => value["roots"][0]["extra"] = json!(true),
                _ => value["extra"] = json!(true),
            }
            let bytes = serde_json::to_vec(&value).unwrap();
            assert!(decode_roster(&bytes, Sha256::digest(&bytes).into()).is_err());
        }
        let duplicate = br#"{"schema":"FerricPackedBf16CompilerRosterR2","schema":"FerricPackedBf16CompilerRosterR2","roots":[]}"#;
        assert!(decode_roster(duplicate, Sha256::digest(duplicate).into()).is_err());
    }

    #[test]
    fn packed_live_feature_and_composition_matrix_requires_current_kv_profile() {
        let options = super::super::super::wave_target_v17_live_contract::fixture(
            super::super::super::wave_target_v17_live_contract::Mode::Combined,
        );
        let input = selection();
        assert_eq!(
            input.validate(&options, variant()).is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps")
            ))
        );
        assert!(input.validate(&options, Variant::V17).is_err());
        for mutation in 0..6 {
            let mut variant = variant();
            let Variant::PrefillKvCopyV28 {
                enabled,
                decode: Some(decode),
                ..
            } = &mut variant
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
            assert!(input.validate(&options, variant).is_err());
        }
        assert_ne!(PROFILE, super::super::ORDERED64_KV_COPY_PROFILE);
        assert_ne!(HOST_DIAGNOSTIC_PROFILE, PROFILE);
        assert_ne!(
            HOST_DIAGNOSTIC_PROFILE,
            super::super::ORDERED64_HOST_DIAGNOSTIC_PROFILE
        );
    }

    #[test]
    fn packed_host_diagnostic_admission_keeps_v19_and_excludes_other_profiles() {
        use super::super::{Ordered64KvCopy, validate_packed_host_diagnostic};
        use std::path::Path;

        let mut options = super::super::super::wave_target_v17_live_contract::fixture(
            super::super::super::wave_target_v17_live_contract::Mode::Combined,
        );
        options.live.max_batches = 135;
        let copy = Ordered64KvCopy {
            artifact: Path::new("copy"),
            enabled: true,
            prefill32_pages: None,
        };
        let output = Path::new("/unused/packed-host.json");
        let input = selection();
        assert_eq!(
            validate_packed_host_diagnostic(&options, variant(), copy, &input, output).is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps")
            ))
        );
        for invalid_copy in [
            Ordered64KvCopy {
                enabled: false,
                ..copy
            },
            Ordered64KvCopy {
                prefill32_pages: Some(false),
                ..copy
            },
            Ordered64KvCopy {
                prefill32_pages: Some(true),
                ..copy
            },
        ] {
            assert!(
                validate_packed_host_diagnostic(&options, variant(), invalid_copy, &input, output)
                    .is_err()
            );
        }
        for flag in 0..4 {
            let mut altered = options.clone();
            match flag {
                0 => altered.live.host_timing = Some(output.into()),
                1 => altered.live.runtime.profile = true,
                2 => altered.live.runtime.ordered64_runtime_counters = true,
                _ => altered.live.runtime.ordered64_packet_ticks = true,
            }
            assert!(
                validate_packed_host_diagnostic(&altered, variant(), copy, &input, output).is_err()
            );
        }
    }

    #[test]
    fn packed_live_storage_metadata_checks_actual_mode_bytes_and_all_72_sources() {
        let sources = (0..36)
            .flat_map(|layer| [(layer, 4, [5; 32]), (layer, 5, [6; 32])])
            .collect::<Vec<_>>();
        for enabled in [false, true] {
            let input = PackedGateUp {
                enabled,
                ..selection()
            };
            let value = input
                .storage_metadata(input.mode(), WEIGHT_BYTES, SCRATCH_BYTES, &sources)
                .unwrap();
            assert_eq!(value["loaded_image_count"], 10);
            assert_eq!(
                value["extra_packets_per_selected_forward"],
                if enabled { 36 } else { 0 }
            );
            assert_eq!(
                value["authenticated_weight_sources"]
                    .as_array()
                    .unwrap()
                    .len(),
                72
            );
            assert!(
                input
                    .storage_metadata("wrong", WEIGHT_BYTES, SCRATCH_BYTES, &sources)
                    .is_err()
            );
            assert!(
                input
                    .storage_metadata(input.mode(), WEIGHT_BYTES - 1, SCRATCH_BYTES, &sources)
                    .is_err()
            );
            assert!(
                input
                    .storage_metadata(input.mode(), WEIGHT_BYTES, SCRATCH_BYTES - 1, &sources)
                    .is_err()
            );
            assert!(
                input
                    .storage_metadata(input.mode(), WEIGHT_BYTES, SCRATCH_BYTES, &sources[..71])
                    .is_err()
            );
            let mut wrong = sources.clone();
            wrong.swap(0, 1);
            assert!(
                input
                    .storage_metadata(input.mode(), WEIGHT_BYTES, SCRATCH_BYTES, &wrong)
                    .is_err()
            );
        }
    }

    #[test]
    fn packed_live_explicit_pins_and_diagnostic_exclusions_are_checked_before_open() {
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
        let mut timed = options.clone();
        timed.live.host_timing = Some("unused-sidecar".into());
        assert!(selection().validate(&timed, variant()).is_err());
        let mut profiled = options;
        profiled.live.runtime.profile = true;
        assert!(selection().validate(&profiled, variant()).is_err());
    }

    #[test]
    #[ignore = "requires independently qualified actual full-N packed R2 image and reviewed roster pins"]
    fn packed_live_actual_image_reopens_only_with_explicit_future_qualification_inputs() {
        let value = |key: &str| std::env::var(key).expect("explicit actual packed R2 test input");
        let input = PackedGateUp {
            artifact: value("FERRIC_TEST_PACKED_R2_ARTIFACT").into(),
            roster: value("FERRIC_TEST_PACKED_R2_ROSTER").into(),
            roster_sha256: digest_argument(&value("FERRIC_TEST_PACKED_R2_ROSTER_SHA256")).unwrap(),
            image_ids: EngineeringTpPackedBf16ImageIdsR2 {
                hsaco: digest_argument(&value("FERRIC_TEST_PACKED_R2_HSACO_SHA256")).unwrap(),
                manifest: digest_argument(&value("FERRIC_TEST_PACKED_R2_MANIFEST_SHA256")).unwrap(),
                handoff: digest_argument(&value("FERRIC_TEST_PACKED_R2_HANDOFF_SHA256")).unwrap(),
            },
            enabled: true,
        };
        let actual = input.open().unwrap();
        assert_eq!(
            artifact_identity(actual.image.artifact()),
            json!({
                "artifact_hsaco_id":hex(&input.image_ids.hsaco),
                "artifact_manifest_id":hex(&input.image_ids.manifest),
                "artifact_handoff_id":hex(&input.image_ids.handoff),
            })
        );
        let wrong = PackedGateUp {
            roster_sha256: [0; 32],
            ..input
        };
        assert!(wrong.open().is_err());
    }
}
