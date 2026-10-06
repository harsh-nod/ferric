use super::*;
use serde_json::{Value, json};

fn fixture(mode: gwire::InputMode) -> (gwire::Bootstrap, gwire::Request, Vec<u8>, Value) {
    let mut b = gwire::tests::bootstrap(mode);
    b.decode.device_ids[0] = u64::from(u32::MAX) + 7;
    let request = gwire::tests::request(&b, 0, None);
    let gwire::Command::Forward {
        cache_metadata,
        rotary_bits,
        ..
    } = &request.command
    else {
        panic!("fixture");
    };
    let mut payload = Vec::new();
    let mut parts = Vec::new();
    for (boundary, specs) in STAGES {
        for rank in 0..2 {
            for &(role, scalar, bytes) in specs {
                let data = match role {
                    "cache_metadata" => cache_metadata
                        .iter()
                        .flat_map(|w| w.to_le_bytes())
                        .collect::<Vec<_>>(),
                    "rotary" => rotary_bits.iter().flat_map(|w| w.to_le_bytes()).collect(),
                    _ => vec![0; bytes as usize],
                };
                assert_eq!(data.len(), bytes as usize);
                parts.push(json!({
                    "boundary":boundary, "rank":rank, "role":role, "scalar":scalar,
                    "elements":bytes / if scalar == "bf16" { 2 } else { 4 },
                    "offset":payload.len(), "bytes":bytes,
                    "source_byte_offset":if matches!(role,"current_key"|"current_value") { u64::from(cache_metadata[1])*16384 } else { 0 },
                    "sha256":hash(&data),
                }));
                payload.extend_from_slice(&data);
            }
        }
    }
    assert_eq!(parts.len(), 34);
    assert_eq!(payload.len(), 256136);
    let value = json!({
        "schema":"FerricFiniteGuardedMlpLayerZeroCaptureV1", "profile_sha256":b.sha256().unwrap(),
        "registration_sha256":b.decode.registration, "session":b.decode.scope.session,
        "device_ids":b.decode.device_ids, "completed_forwards":4, "native_closed":true,
        "sampling":"prefix-boundaries-and-post-paired-retained",
        "numerical_acceptance":false,"performance_claim":false,"production_authority":false,
        "capture":{
            "schema":"FerricFiniteLayerZeroCaptureV1","generation":1,"position":0,"layer":0,
            "parts":parts,"payload_bytes":256136,"payload_sha256":hash(&payload),"payload":payload,
            "full_cache_capture":false,"native_close_confirmed":true,
            "numerical_acceptance":false,"performance_claim":false,"production_authority":false,
        },
    });
    (b, request, vec![0; old::OBSERVATION_BYTES], value)
}

fn accept(
    b: &gwire::Bootstrap,
    r: &gwire::Request,
    observation: &[u8],
    value: &Value,
) -> Result<()> {
    validate(&serde_json::to_vec(value).unwrap(), b, r, observation)
}

#[test]
fn guarded_capture_parent_accepts_only_closed_same_run_layout_in_both_modes() {
    assert_eq!(
        worker_flag(false),
        "--engineering-native-guarded-mlp-decode-v1"
    );
    assert_eq!(
        worker_flag(true),
        "--engineering-native-guarded-mlp-stage-capture-v1"
    );
    for mode in [
        gwire::InputMode::TeacherForced,
        gwire::InputMode::Autoregressive,
    ] {
        let (b, r, observed, value) = fixture(mode);
        accept(&b, &r, &observed, &value).unwrap();
        assert!(serde_json::to_vec(&value).unwrap().len() < LIMIT);
    }
}

#[test]
fn guarded_capture_parent_rejects_namespace_scope_close_and_authority_drift() {
    let (b, r, observed, value) = fixture(gwire::InputMode::TeacherForced);
    for (field, replacement) in [
        ("schema", json!("old")),
        ("profile_sha256", json!(vec![0u8; 32])),
        ("registration_sha256", json!(vec![0u8; 32])),
        ("session", json!(vec![0u8; 32])),
        ("device_ids", json!([0, 1])),
        ("completed_forwards", json!(3)),
        ("native_closed", json!(false)),
        ("sampling", json!("interleaved-packets")),
        ("numerical_acceptance", json!(true)),
        ("performance_claim", json!(true)),
        ("production_authority", json!(true)),
        ("unexpected", json!(true)),
    ] {
        let mut v = value.clone();
        v[field] = replacement;
        assert!(accept(&b, &r, &observed, &v).is_err(), "{field}");
    }
    for (field, replacement) in [
        ("schema", json!("old")),
        ("generation", json!(2)),
        ("position", json!(1)),
        ("layer", json!(1)),
        ("payload_bytes", json!(0)),
        ("full_cache_capture", json!(true)),
        ("native_close_confirmed", json!(false)),
        ("numerical_acceptance", json!(true)),
        ("performance_claim", json!(true)),
        ("production_authority", json!(true)),
        ("unexpected", json!(true)),
    ] {
        let mut v = value.clone();
        v["capture"][field] = replacement;
        assert!(accept(&b, &r, &observed, &v).is_err(), "{field}");
    }
    assert!(validate(&[], &b, &r, &observed).is_err());
    assert!(validate(&vec![b' '; LIMIT + 1], &b, &r, &observed).is_err());
    let mut two = serde_json::to_vec(&value).unwrap();
    two.extend_from_slice(b"{}");
    assert!(validate(&two, &b, &r, &observed).is_err());
}

#[test]
fn guarded_capture_parent_rejects_reordered_parts_offsets_widths_and_extra_payload() {
    let (b, r, observed, value) = fixture(gwire::InputMode::TeacherForced);
    for (field, replacement) in [
        ("boundary", json!("after_mlp")),
        ("rank", json!(2)),
        ("role", json!("down_partial")),
        ("scalar", json!("u32")),
        ("elements", json!(1)),
        ("offset", json!(1)),
        ("bytes", json!(4)),
        ("source_byte_offset", json!(4)),
        ("unexpected", json!(0)),
    ] {
        let mut v = value.clone();
        v["capture"]["parts"][0][field] = replacement;
        assert!(accept(&b, &r, &observed, &v).is_err(), "{field}");
    }
    let mut v = value.clone();
    v["capture"]["parts"].as_array_mut().unwrap().swap(0, 1);
    assert!(accept(&b, &r, &observed, &v).is_err());
    let mut v = value.clone();
    v["capture"]["parts"].as_array_mut().unwrap().pop();
    assert!(accept(&b, &r, &observed, &v).is_err());
    let mut v = value;
    v["capture"]["payload"]
        .as_array_mut()
        .unwrap()
        .push(json!(0));
    assert!(accept(&b, &r, &observed, &v).is_err());
}

fn changed_payload(value: &mut Value, part: usize, word: &[u8]) {
    let offset = value["capture"]["parts"][part]["offset"].as_u64().unwrap() as usize;
    let mut data: Vec<u8> = serde_json::from_value(value["capture"]["payload"].clone()).unwrap();
    data[offset..offset + word.len()].copy_from_slice(word);
    let bytes = value["capture"]["parts"][part]["bytes"].as_u64().unwrap() as usize;
    value["capture"]["parts"][part]["sha256"] = json!(hash(&data[offset..offset + bytes]));
    value["capture"]["payload_sha256"] = json!(hash(&data));
    value["capture"]["payload"] = json!(data);
}

#[test]
fn guarded_capture_parent_rejects_digest_nonfinite_and_same_run_content_mismatch() {
    let (b, r, observed, value) = fixture(gwire::InputMode::TeacherForced);
    let mut v = value.clone();
    v["capture"]["payload"][0] = json!(1);
    assert!(accept(&b, &r, &observed, &v).is_err());
    let mut v = value.clone();
    v["capture"]["parts"][0]["sha256"] = json!(vec![0u8; 32]);
    assert!(accept(&b, &r, &observed, &v).is_err());
    for (part, word) in [
        (0, 0x7fc0u16.to_le_bytes().to_vec()),
        (2, f32::INFINITY.to_le_bytes().to_vec()),
        (1, 1u32.to_le_bytes().to_vec()),
        (2, 1.25f32.to_le_bytes().to_vec()),
        (32, 0x3f80u16.to_le_bytes().to_vec()),
        (33, 0x3f80u16.to_le_bytes().to_vec()),
    ] {
        let mut v = value.clone();
        changed_payload(&mut v, part, &word);
        assert!(accept(&b, &r, &observed, &v).is_err(), "part {part}");
    }
    let mut changed = observed.clone();
    changed[0] = 1;
    assert!(accept(&b, &r, &changed, &value).is_err());
    let mut r = r;
    r.id = 2;
    assert!(accept(&b, &r, &observed, &value).is_err());
}
