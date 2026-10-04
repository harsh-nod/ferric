use super::*;

pub(crate) fn bootstrap() -> Bootstrap {
    Bootstrap {
        schema: SCHEMA.into(),
        decode: base::tests::bootstrap(InputMode::TeacherForced),
        projection_residual_image: part(&[10]),
    }
}
pub(crate) fn ar_bootstrap() -> Bootstrap {
    Bootstrap {
        decode: base::tests::bootstrap(InputMode::Autoregressive),
        ..bootstrap()
    }
}
fn stream(b: &Bootstrap) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_bootstrap(&mut bytes, &mut FrameBudget::new(), b, &[8], &[9], &[10]).unwrap();
    bytes
}
#[test]
fn projection_decode_wire_roundtrip_keeps_old_begin_and_distinct_profile() {
    let b = bootstrap();
    let bytes = stream(&b);
    let (actual, mlp, prefix, projection) =
        read_bootstrap(&mut &bytes[..], &mut FrameBudget::new())
            .unwrap()
            .unwrap();
    assert_eq!(actual, b);
    assert_eq!((mlp, prefix, projection), (vec![8], vec![9], vec![10]));
    assert_ne!(b.sha256().unwrap(), b.decode.sha256().unwrap());
    assert_eq!(b.decode.begin.residual_image, part(&[7]));
    assert_eq!(
        b.total_input_bytes().unwrap(),
        b.decode.total_input_bytes().unwrap() + 1
    );
    assert!(base::read_bootstrap(&mut &bytes[..], &mut FrameBudget::new()).is_err());
    let mut old = Vec::new();
    base::write_bootstrap(&mut old, &mut FrameBudget::new(), &b.decode, &[8], &[9]).unwrap();
    assert!(read_bootstrap(&mut &old[..], &mut FrameBudget::new()).is_err());
}
#[test]
fn projection_decode_wire_refuses_invalid_mode_shape_missing_image_and_unknown_fields() {
    for change in 0..5 {
        let mut b = bootstrap();
        match change {
            0 => b.schema = "FerricProjectionResidualLayerBootstrapV1".into(),
            1 => {
                b.decode.mode = InputMode::Autoregressive;
            }
            2 => b.projection_residual_image.bytes = 0,
            3 => b.projection_residual_image.sha256 = [0; 32],
            _ => b.projection_residual_image.bytes = MAX_IMAGE_BYTES as u32 + 1,
        }
        assert!(b.sha256().is_err());
    }
    let mut value = serde_json::to_value(bootstrap()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("clock".into(), true.into());
    assert!(serde_json::from_value::<Bootstrap>(value).is_err());
}

#[test]
fn projection_ar4_wire_authenticates_mode_seed_and_own_output_recurrence() {
    let b = ar_bootstrap();
    let serialized = stream(&b);
    let mut bytes = &serialized[..];
    let (seen, _, _, _) = read_bootstrap(&mut bytes, &mut FrameBudget::new())
        .unwrap()
        .unwrap();
    assert_eq!(seen, b);
    assert!(bytes.is_empty());
    assert_ne!(b.sha256().unwrap(), bootstrap().sha256().unwrap());
    assert_eq!(b.input(0, None).unwrap(), 9112);
    assert!(b.input(1, None).is_err());
    assert_eq!(b.input(1, Some(17)).unwrap(), 17);
    assert_eq!(b.input(3, Some(19)).unwrap(), 19);
    assert!(b.input(4, Some(20)).is_err());
    assert!(
        b.validate(
            b.decode.device_ids,
            b.decode.timeout_ms,
            b.decode.scope.child_identity,
            InputMode::TeacherForced
        )
        .is_err()
    );
    let mut bad = b.clone();
    bad.decode.input_tokens.push(7);
    assert!(bad.sha256().is_err());
    let mut changed = b.clone();
    changed.projection_residual_image.sha256[0] ^= 1;
    assert_ne!(b.sha256().unwrap(), changed.sha256().unwrap());
    let mut output = Vec::new();
    assert!(write_bootstrap(&mut output, &mut FrameBudget::new(), &b, &[8], &[9], &[11]).is_err());
    assert!(output.is_empty());
}

#[test]
fn projection_tf4_wire_profile_formula_and_serialized_fields_are_unchanged() {
    let b = bootstrap();
    let mut h = Sha256::new();
    h.update(b"ferric-projection-residual-four-decode-v1\0");
    h.update(b"ordered-fp32-tp2-bf16-projection-then-bf16-residual-v1");
    h.update(b.decode.sha256().unwrap());
    h.update(b.projection_residual_image.sha256);
    let expected: [u8; 32] = h.finalize().into();
    assert_eq!(b.sha256().unwrap(), expected);
    let expected = format!(
        "{{\"schema\":\"FerricProjectionResidualDecodeBootstrapV1\",\"decode\":{},\"projection_residual_image\":{}}}",
        serde_json::to_string(&b.decode).unwrap(),
        serde_json::to_string(&b.projection_residual_image).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&b).unwrap().as_slice(),
        expected.as_bytes()
    );
    assert_eq!(b.decode.mode, InputMode::TeacherForced);
    assert_eq!(b.decode.input_tokens, [9112, 2190, 3772, 220]);
}
#[test]
fn projection_decode_wire_checks_all_actual_image_bytes_before_write() {
    let b = bootstrap();
    for bodies in [
        (&[0][..], &[9][..], &[10][..]),
        (&[8][..], &[0][..], &[10][..]),
        (&[8][..], &[9][..], &[0][..]),
    ] {
        let mut out = Vec::new();
        assert!(
            write_bootstrap(
                &mut out,
                &mut FrameBudget::new(),
                &b,
                bodies.0,
                bodies.1,
                bodies.2
            )
            .is_err()
        );
        assert!(out.is_empty());
    }
    let bytes = stream(&b);
    for length in [1, bytes.len() - 1] {
        assert!(read_bootstrap(&mut &bytes[..length], &mut FrameBudget::new()).is_err());
    }
    let mut changed = bytes;
    *changed.last_mut().unwrap() ^= 1;
    assert!(read_bootstrap(&mut &changed[..], &mut FrameBudget::new()).is_err());
}
#[test]
fn projection_decode_wire_additional_image_cannot_exceed_original_stream_budget() {
    let mut b = bootstrap();
    b.decode.prefix_image.bytes = MAX_IMAGE_BYTES as u32;
    b.decode.tiles_image.bytes = (MAX_IMAGE_BYTES - 8 * HEADER_BYTES) as u32;
    b.decode.sha256().unwrap();
    b.projection_residual_image.bytes = MAX_IMAGE_BYTES as u32;
    assert!(b.sha256().is_err());
    assert!(b.total_input_bytes().is_err());
}
#[test]
fn projection_decode_wire_profile_binds_base_and_candidate_without_layer_alias() {
    let b = bootstrap();
    let expected = b.sha256().unwrap();
    let mut changed = b.clone();
    changed.projection_residual_image.sha256[0] ^= 1;
    assert_ne!(expected, changed.sha256().unwrap());
    changed = b.clone();
    changed.decode.input_tokens[3] ^= 1;
    assert_ne!(expected, changed.sha256().unwrap());
    assert!(profile_sha256([0; 32], [1; 32]).is_err());
    assert!(profile_sha256([1; 32], [0; 32]).is_err());
    assert_ne!(
        expected,
        crate::finite_projection_residual_layer_wire_v1::profile_sha256(
            b.decode.sha256().unwrap(),
            b.projection_residual_image.sha256
        )
        .unwrap()
    );
}
