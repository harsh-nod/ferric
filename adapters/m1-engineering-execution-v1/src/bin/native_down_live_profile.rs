//! Exact-image native32 down selection, kept separate from ordinary packed paths.
use super::{artifact_identity, hex, packed_digest_argument};
use ferric_m1_engineering_execution_v1::tp_artifact::{
    ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS, EngineeringTpSplitKDownArtifactR1,
    EngineeringTpSplitKDownImageIdsR1,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

const MAX_ROSTER_BYTES: u64 = 16 * 1024;

#[derive(Clone)]
pub(crate) struct NativeDown {
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
type Roster = [Root; 2];

pub(super) struct AdmittedNativeDown {
    pub image: EngineeringTpSplitKDownArtifactR1,
    roster: Roster,
}

impl NativeDown {
    pub(super) fn validate(&self) -> Result<(), String> {
        if !cfg!(all(
            feature = "c1-token-program",
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
        {
            return Err(
                "native down requires native32 and explicit nonzero image/roster pins".into(),
            );
        }
        Ok(())
    }

    pub(super) fn open(&self) -> Result<AdmittedNativeDown, String> {
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
            return Err("native down roster must be a canonical bounded regular file".into());
        }
        let mut file = std::fs::File::from(
            rustix::fs::open(
                &self.roster,
                rustix::fs::OFlags::RDONLY
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::NONBLOCK
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::empty(),
            )
            .map_err(|error| error.to_string())?,
        );
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
            return Err("native down roster changed while reopening".into());
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
        let image = EngineeringTpSplitKDownArtifactR1::open(&self.artifact, &names, self.image_ids)
            .map_err(|error| error.to_string())?;
        Ok(AdmittedNativeDown { image, roster })
    }

    pub(super) fn metadata(
        &self,
        admitted: &AdmittedNativeDown,
        actual: Option<(bool, u64)>,
    ) -> Result<Value, String> {
        if actual != Some((self.enabled, 131_072)) {
            return Err("native down actual selection or scratch extent differs".into());
        }
        Ok(json!({
            "schema":"FerricNativeDownSelectionR1","authority":"none",
            "enabled":self.enabled,"artifact_path":self.artifact,
            "artifact":artifact_identity(admitted.image.artifact()),
            "compiler_roster":{"path":self.roster,"sha256":hex(&self.roster_sha256),"value":admitted.roster},
            "loaded_image_count":10,"additional_weight_bytes":0,"scratch_bytes":131_072,
            "weights":"existing authenticated KN BF16 down maps",
            "scratch_layout":"eight FP32 partial rows of 4096 elements",
            "prefill_unchanged":true,"prefill_rows":32,"prefill_dispatches":649,
            "decode_dispatches":if self.enabled {688} else {652},
            "decode_dynamic_slots":180,"decode_command_family":"legacy256-v1",
            "decode_positions":[128,255],"roles":[2],"selected_rows":1,
            "same_image_set_in_both_arms":true,"same_scratch_in_both_arms":true,
            "native_qualified":false,"performance_qualified":false,"serving_qualified":false
        }))
    }
}

pub(super) const fn profile(enabled: bool, counters: bool) -> &'static str {
    match (enabled, counters) {
        (false, false) => "prefill32-native649-decode652-down-control-r1",
        (true, false) => "prefill32-native649-decode688-down-splitk8-r1",
        (false, true) => "prefill32-native649-decode652-down-control-counters-r1",
        (true, true) => "prefill32-native649-decode688-down-splitk8-counters-r1",
    }
}

pub(super) fn annotate(
    value: &mut Value,
    selection: &NativeDown,
    metadata: &Value,
    counters: bool,
) {
    value["live_profile"] = json!(profile(selection.enabled, counters));
    value["native_down"] = metadata.clone();
    value["split_attention_policy"]["packet_counts_scope"] =
        json!("baseline FFN only; selected composition counts are in native_down");
    value["token_program"]["c1_dispatches"] = json!(if selection.enabled { 688 } else { 652 });
    value["token_program"]["ordinary_fallback"] = json!(false);
    value["token_program"]["prefill_unchanged"] = json!(true);
    if counters {
        value["token_program"]["counter_schema"] = json!("FerricNativeDownProgramCountersR1");
    }
    value["prefill_program"]["fallback"] = json!("reject ineligible batches before submission");
    value["prefill_program"]["decode_unchanged"] = json!(!selection.enabled);
    value["prefill_program"]["same_kernel_images"] = json!(true);
}

#[allow(dead_code)]
pub(crate) fn parse(
    arguments: impl IntoIterator<Item = String>,
) -> Result<(Option<NativeDown>, Vec<String>), String> {
    const FLAGS: [&str; 7] = [
        "--native-down",
        "--down-artifact",
        "--down-roster",
        "--down-roster-sha256",
        "--down-hsaco-sha256",
        "--down-manifest-sha256",
        "--down-handoff-sha256",
    ];
    let mut arguments = arguments.into_iter();
    let mut values = BTreeMap::new();
    let mut rest = Vec::new();
    while let Some(argument) = arguments.next() {
        if FLAGS.contains(&argument.as_str()) {
            let value = arguments.next().ok_or("native down option value missing")?;
            if values.insert(argument, value).is_some() {
                return Err("duplicate native down option".into());
            }
        } else {
            rest.push(argument);
        }
    }
    if values.is_empty() {
        return Ok((None, rest));
    }
    if values.len() != FLAGS.len() {
        return Err("native down requires its complete explicit pin set".into());
    }
    let enabled = match values["--native-down"].as_str() {
        "control" => false,
        "splitk8" => true,
        _ => return Err("native down must be control or splitk8".into()),
    };
    let selection = NativeDown {
        artifact: PathBuf::from(&values["--down-artifact"]),
        roster: PathBuf::from(&values["--down-roster"]),
        roster_sha256: packed_digest_argument(&values["--down-roster-sha256"])?,
        image_ids: EngineeringTpSplitKDownImageIdsR1 {
            hsaco: packed_digest_argument(&values["--down-hsaco-sha256"])?,
            manifest: packed_digest_argument(&values["--down-manifest-sha256"])?,
            handoff: packed_digest_argument(&values["--down-handoff-sha256"])?,
        },
        enabled,
    };
    selection.validate()?;
    Ok((Some(selection), rest))
}

fn decode_roster(bytes: &[u8], expected: [u8; 32]) -> Result<Roster, String> {
    if bytes.is_empty()
        || bytes.len() as u64 > MAX_ROSTER_BYTES
        || expected == [0; 32]
        || Sha256::digest(bytes).as_slice() != expected
    {
        return Err("native down roster byte identity differs".into());
    }
    let roster: Roster = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
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
        return Err("native down requires the exact ordered partial/merge compiler roster".into());
    }
    Ok(roster)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(arm: &str) -> Vec<String> {
        [
            "--native-down",
            arm,
            "--down-artifact",
            "/unused/image",
            "--down-roster",
            "/unused/roster.json",
            "--down-roster-sha256",
            &"1".repeat(64),
            "--down-hsaco-sha256",
            &"2".repeat(64),
            "--down-manifest-sha256",
            &"3".repeat(64),
            "--down-handoff-sha256",
            &"4".repeat(64),
        ]
        .map(String::from)
        .to_vec()
    }

    #[test]
    fn native_down_missing_selection_preserves_original_arguments() {
        let args = ["--native-prefill-rows", "32", "--model", "m"]
            .map(String::from)
            .to_vec();
        let (selection, rest) = parse(args.clone()).unwrap();
        assert!(selection.is_none());
        assert_eq!(rest, args);
    }

    #[test]
    fn native_down_parser_requires_closed_explicit_nonzero_pins() {
        let supported = cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        ));
        for (arm, enabled) in [("control", false), ("splitk8", true)] {
            let mut args = arguments(arm);
            args.extend(["--native-prefill-rows".into(), "32".into()]);
            let result = parse(args);
            assert_eq!(result.is_ok(), supported);
            if supported {
                let (selection, rest) = result.unwrap();
                assert_eq!(selection.unwrap().enabled, enabled);
                assert_eq!(rest, ["--native-prefill-rows", "32"]);
            }
        }
        for mutation in 0..8 {
            let mut args = arguments("control");
            match mutation {
                0 => {
                    args.pop();
                }
                1 => {
                    args.drain(2..4);
                }
                2 => args.extend(["--native-down".into(), "splitk8".into()]),
                3 => args[1] = "auto".into(),
                4 => args[3] = "relative/image".into(),
                5 => args[7] = "0".repeat(64),
                6 => args[9] = "A".repeat(64),
                7 => args[11] = "a".repeat(63),
                _ => unreachable!(),
            }
            assert!(parse(args).is_err(), "{mutation}");
        }
    }

    #[test]
    fn native_down_roster_is_exact_bounded_and_hash_bound() {
        let roster = json!([
            {"logical_name":"partial","export_name":ROOTS[0]},
            {"logical_name":"merge","export_name":ROOTS[1]},
        ]);
        let bytes = serde_json::to_vec(&roster).unwrap();
        decode_roster(&bytes, Sha256::digest(&bytes).into()).unwrap();
        assert!(decode_roster(&bytes, [7; 32]).is_err());
        for mutation in 0..7 {
            let mut value = roster.clone();
            match mutation {
                0 => value.as_array_mut().unwrap().reverse(),
                1 => value[0]["logical_name"] = json!("merge"),
                2 => value[0]["extra"] = json!(true),
                3 => value[0]["logical_name"] = json!("line\nbreak"),
                4 => value[0]["export_name"] = json!("other"),
                5 => value.as_array_mut().unwrap().push(json!({})),
                6 => value[0]["logical_name"] = json!("x".repeat(513)),
                _ => unreachable!(),
            }
            let bytes = serde_json::to_vec(&value).unwrap();
            assert!(decode_roster(&bytes, Sha256::digest(&bytes).into()).is_err());
        }
        let bytes = vec![b' '; MAX_ROSTER_BYTES as usize + 1];
        assert!(decode_roster(&bytes, Sha256::digest(&bytes).into()).is_err());
    }

    #[test]
    #[ignore = "requires retained actual image and exact generated roster; CPU only"]
    fn native_down_actual_image_and_roster_open_for_both_arms() {
        let artifact = PathBuf::from(
            std::env::var_os("FERRIC_TEST_SPLITK_DOWN_R1_ARTIFACT").expect("exact retained image"),
        );
        let roster = PathBuf::from(
            std::env::var_os("FERRIC_TEST_NATIVE_DOWN_ROSTER")
                .expect("exact actual-image compiler roster"),
        );
        let bytes = std::fs::read(&roster).unwrap();
        let hash = |value: &str| {
            std::array::from_fn(|i| u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).unwrap())
        };
        for enabled in [false, true] {
            let selection = NativeDown {
                artifact: artifact.clone(),
                roster: roster.clone(),
                roster_sha256: Sha256::digest(&bytes).into(),
                enabled,
                image_ids: EngineeringTpSplitKDownImageIdsR1 {
                    hsaco: hash("1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae"),
                    manifest: hash(
                        "2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54",
                    ),
                    handoff: hash(
                        "cd23060449750a973f05742c0d5a043cb637491743e2ac56b56e847cc2c4dbdf",
                    ),
                },
            };
            selection.validate().unwrap();
            let admitted = selection.open().unwrap();
            let metadata = selection
                .metadata(&admitted, Some((enabled, 131_072)))
                .unwrap();
            assert_eq!(metadata["loaded_image_count"], 10);
            assert_eq!(
                metadata["decode_dispatches"],
                if enabled { 688 } else { 652 }
            );
            assert_eq!(
                metadata["compiler_roster"]["value"],
                serde_json::from_slice::<Value>(&bytes).unwrap()
            );
            assert!(
                selection
                    .metadata(&admitted, Some((!enabled, 131_072)))
                    .is_err()
            );
            assert!(
                selection
                    .metadata(&admitted, Some((enabled, 131_071)))
                    .is_err()
            );
            assert!(selection.metadata(&admitted, None).is_err());
        }
    }

    #[test]
    fn native_down_annotation_overrides_all_inherited_decode_claims() {
        for enabled in [false, true] {
            for counters in [false, true] {
                let selection = NativeDown {
                    artifact: "/image".into(),
                    roster: "/roster".into(),
                    roster_sha256: [1; 32],
                    image_ids: EngineeringTpSplitKDownImageIdsR1 {
                        hsaco: [2; 32],
                        manifest: [3; 32],
                        handoff: [4; 32],
                    },
                    enabled,
                };
                let mut value = json!({});
                super::super::annotate_token_program(
                    &mut value,
                    crate::tp_worker::TokenProgramBackend::NativeWholeProgramSlots512V1,
                    counters,
                );
                super::super::annotate_prefill_width(&mut value, 32, counters);
                let metadata = json!({"enabled":enabled,"scratch_bytes":131072});
                annotate(&mut value, &selection, &metadata, counters);
                assert_eq!(value["live_profile"], profile(enabled, counters));
                assert_eq!(value["native_down"], metadata);
                assert_eq!(
                    value["split_attention_policy"]["packet_counts_scope"],
                    "baseline FFN only; selected composition counts are in native_down"
                );
                assert_eq!(
                    value["token_program"]["c1_dispatches"],
                    if enabled { 688 } else { 652 }
                );
                assert_eq!(value["token_program"]["ordinary_fallback"], false);
                assert_eq!(value["token_program"]["prefill_unchanged"], true);
                assert_eq!(value["prefill_program"]["decode_unchanged"], !enabled);
                assert_eq!(
                    value["prefill_program"]["fallback"],
                    "reject ineligible batches before submission"
                );
                if counters {
                    assert_eq!(
                        value["token_program"]["counter_schema"],
                        "FerricNativeDownProgramCountersR1"
                    );
                }
            }
        }
    }
}
