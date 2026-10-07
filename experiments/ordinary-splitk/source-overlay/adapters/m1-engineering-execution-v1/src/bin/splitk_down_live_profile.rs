//! Separate same-image down-only experiment, not a kernel or model speed claim.

use super::{DecodeComposition, Options, Ordered64KvCopy, Variant, artifact_identity, hex};
use ferric_m1_engineering_execution_v1::tp_artifact::{
    ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS, EngineeringTpSplitKDownArtifactR1,
    EngineeringTpSplitKDownImageIdsR1,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(super) const PROFILE: &str = "prefill16-decode-ordered64-v19-splitk-down-r1-live-v1";
const MAX_ROSTER_BYTES: u64 = 16 * 1024;

#[derive(Clone)]
pub(crate) struct SplitKDown {
    pub artifact: PathBuf,
    pub roster: PathBuf,
    pub roster_sha256: [u8; 32],
    pub image_ids: EngineeringTpSplitKDownImageIdsR1,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Root {
    logical_name: String,
    export_name: String,
}

pub(super) struct AdmittedSplitKDown {
    pub image: EngineeringTpSplitKDownArtifactR1,
    roster: [Root; 2],
}

impl SplitKDown {
    pub(super) fn mode(&self) -> &'static str {
        if self.enabled {
            "splitk8-down-mfma-r1"
        } else {
            "baseline"
        }
    }

    pub(crate) fn validate(
        &self,
        options: &Options,
        variant: Variant<'_>,
        copy: Ordered64KvCopy<'_>,
    ) -> Result<(), String> {
        variant.validate(options)?;
        copy.validate(options, variant)?;
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
            || !copy.enabled
            || copy.prefill32_pages.is_some()
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
            return Err("split-K down requires ordinary V19/V27/split8/packed64, baseline GEMV, exact image pins and no diagnostics/programs".into());
        }
        Ok(())
    }

    pub(super) fn open(&self) -> Result<AdmittedSplitKDown, String> {
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
            return Err("split-K roster must be canonical bounded regular bytes".into());
        }
        let bytes = read_roster(&self.roster, &before)?;
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
        let image = EngineeringTpSplitKDownArtifactR1::open(&self.artifact, &names, self.image_ids)
            .map_err(|error| error.to_string())?;
        Ok(AdmittedSplitKDown { image, roster })
    }

    pub(super) fn metadata(
        &self,
        admitted: &AdmittedSplitKDown,
        mode: &str,
        scratch: u64,
        resident: u64,
    ) -> Result<Value, String> {
        let mut value = self.storage_metadata(mode, scratch, resident)?;
        value["artifact_path"] = json!(self.artifact);
        value["artifact"] = artifact_identity(admitted.image.artifact());
        value["compiler_roster"] =
            json!({"path":self.roster,"sha256":hex(&self.roster_sha256),"value":admitted.roster});
        Ok(value)
    }

    fn storage_metadata(&self, mode: &str, scratch: u64, resident: u64) -> Result<Value, String> {
        if mode != self.mode() || scratch != 131_072 || resident != 3_623_878_656 {
            return Err("split-K actual selection or resident/scratch ownership differs".into());
        }
        Ok(json!({
            "schema":"FerricSplitKDownLiveSelectionR1", "authority":"none",
            "requested_mode":self.mode(), "actual_mode":mode,
            "additional_weight_bytes":0, "resident_transposed_down_bytes":resident,
            "activation_scratch_bytes":scratch, "loaded_image_count":10,
            "composition":"V5/V8/V11/V14/V15/V19/V20/V21/V27 plus unpaired split-K down; V20 unselected",
            "weight_source":"existing authenticated resident KxN down tensors, no new transpose or upload",
            "worker_backend":"ordinary-ordered64", "selected_phase":"decode", "selected_rows":1,
            "selected_published_rows":1, "roles":[2], "c1_only":true,
            "max_simultaneous_selected_requests":1, "prefill_unchanged":true,
            "same_image_set_in_both_arms":true, "same_scratch_in_both_arms":true,
            "extra_packets_per_selected_forward":if self.enabled {36} else {0},
            "decode_packets":if self.enabled {688} else {652},
            "model_batches_128_128":135, "model_dispatches_128_128":if self.enabled {92283} else {87711},
            "source_schedule_only":true, "native_qualified":false, "performance_qualified":false,
            "serving_qualified":false, "numerical_caveat":"changed split reduction grouping requires full-model token parity",
        }))
    }
}

fn read_roster(path: &Path, before: &std::fs::Metadata) -> Result<Vec<u8>, String> {
    let flags = i32::try_from((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits())
        .map_err(|error| error.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(|error| error.to_string())?;
    let identity = |value: &std::fs::Metadata| {
        (
            value.dev(),
            value.ino(),
            value.len(),
            value.mtime(),
            value.mtime_nsec(),
            value.ctime(),
            value.ctime_nsec(),
        )
    };
    let opened = file.metadata().map_err(|error| error.to_string())?;
    if !before.is_file()
        || before.len() == 0
        || before.len() > MAX_ROSTER_BYTES
        || !opened.is_file()
        || identity(before) != identity(&opened)
    {
        return Err("split-K roster changed before reading".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_ROSTER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let after = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    let fd_after = file.metadata().map_err(|error| error.to_string())?;
    if !after.is_file()
        || identity(before) != identity(&after)
        || identity(before) != identity(&fd_after)
        || bytes.len() as u64 != before.len()
    {
        return Err("split-K roster changed while reopening".into());
    }
    Ok(bytes)
}

fn decode_roster(bytes: &[u8], expected: [u8; 32]) -> Result<[Root; 2], String> {
    if bytes.is_empty()
        || bytes.len() as u64 > MAX_ROSTER_BYTES
        || expected == [0; 32]
        || Sha256::digest(bytes).as_slice() != expected
    {
        return Err("split-K roster byte identity differs".into());
    }
    let roster: [Root; 2] = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if roster.iter().zip(ROOTS).any(|(root, export)| {
        root.export_name != export
            || root.logical_name.is_empty()
            || root.logical_name.len() > 512
            || !root.logical_name.is_ascii()
            || root
                .logical_name
                .bytes()
                .any(|byte| byte.is_ascii_control())
    }) || roster[0].logical_name == roster[1].logical_name
    {
        return Err("split-K requires exact ordered partial/merge compiler roots".into());
    }
    Ok(roster)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitk_cli_roster_replacement_refuses_without_blocking() {
        let directory = std::env::temp_dir().join(format!(
            "ferric-splitk-roster-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("roster.json");
        std::fs::write(&path, b"[]").unwrap();
        let before = std::fs::symlink_metadata(&path).unwrap();
        assert_eq!(read_roster(&path, &before).unwrap(), b"[]");
        let saved = directory.join("saved.json");
        std::fs::rename(&path, &saved).unwrap();
        std::os::unix::fs::symlink(&saved, &path).unwrap();
        assert!(read_roster(&path, &before).is_err());
        std::fs::remove_file(&path).unwrap();
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &path,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .unwrap();
        assert!(read_roster(&path, &before).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"[]").unwrap();
        assert!(read_roster(&path, &before).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&saved).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn splitk_cli_roster_is_closed_ordered_and_hash_bound() {
        let original = json!([{"logical_name":"partial", "export_name":ROOTS[0]},
            {"logical_name":"merge", "export_name":ROOTS[1]}]);
        let raw = serde_json::to_vec(&original).unwrap();
        decode_roster(&raw, Sha256::digest(&raw).into()).unwrap();
        assert!(decode_roster(&raw, [0; 32]).is_err());
        for mutation in 0..7 {
            let mut value = original.clone();
            match mutation {
                0 => value.as_array_mut().unwrap().reverse(),
                1 => value[0]["logical_name"] = json!("merge"),
                2 => value[0]["logical_name"] = json!("x".repeat(513)),
                3 => value[0]["logical_name"] = json!("bad\nname"),
                4 => value[0]["extra"] = json!(true),
                5 => value.as_array_mut().unwrap().push(json!({})),
                _ => value[1]["export_name"] = json!(ROOTS[0]),
            }
            let raw = serde_json::to_vec(&value).unwrap();
            assert!(decode_roster(&raw, Sha256::digest(&raw).into()).is_err());
        }
    }

    #[test]
    fn splitk_cli_metadata_keeps_assets_equal_and_counts_distinct() {
        for enabled in [false, true] {
            let selection = SplitKDown {
                artifact: "/unused/image".into(),
                roster: "/unused/roster".into(),
                roster_sha256: [1; 32],
                image_ids: EngineeringTpSplitKDownImageIdsR1 {
                    hsaco: [2; 32],
                    manifest: [3; 32],
                    handoff: [4; 32],
                },
                enabled,
            };
            let value = selection
                .storage_metadata(selection.mode(), 131_072, 3_623_878_656)
                .unwrap();
            assert_eq!(value["additional_weight_bytes"], 0);
            assert_eq!(value["loaded_image_count"], 10);
            assert_eq!(value["decode_packets"], if enabled { 688 } else { 652 });
            assert_eq!(
                value["model_dispatches_128_128"],
                if enabled { 92283 } else { 87711 }
            );
            assert!(
                selection
                    .storage_metadata("other", 131_072, 3_623_878_656)
                    .is_err()
            );
            assert!(
                selection
                    .storage_metadata(selection.mode(), 0, 3_623_878_656)
                    .is_err()
            );
            assert!(
                selection
                    .storage_metadata(selection.mode(), 131_072, 0)
                    .is_err()
            );
        }
    }
}
