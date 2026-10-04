use super::*;

pub(crate) fn bootstrap() -> Bootstrap {
    Bootstrap {
        schema: SCHEMA.into(),
        decode: base::tests::bootstrap(InputMode::TeacherForced),
        projection_residual_image: part(&[10]),
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
fn projection_decode_wire_refuses_non_tf_missing_image_and_unknown_fields() {
    for change in 0..5 {
        let mut b = bootstrap();
        match change {
            0 => b.schema = "FerricProjectionResidualLayerBootstrapV1".into(),
            1 => {
                b.decode.mode = InputMode::Autoregressive;
                b.decode.input_tokens.truncate(1);
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
