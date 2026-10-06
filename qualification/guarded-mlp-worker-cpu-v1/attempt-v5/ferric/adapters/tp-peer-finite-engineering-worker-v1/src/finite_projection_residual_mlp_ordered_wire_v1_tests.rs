use super::*;
pub(crate) fn ar_bootstrap() -> Bootstrap {
    let old = crate::finite_projection_residual_decode_wire_v1::tests::ar_bootstrap();
    Bootstrap {
        schema: SCHEMA.into(),
        decode: old.decode,
        projection_residual_image: old.projection_residual_image,
    }
}
pub(crate) fn control() -> Control {
    let old = base::tests::control();
    Control {
        embedding_ns: old.embedding_ns,
        tail_ns: old.tail_ns,
        layers: old.layers.map(|r| LayerObservation {
            prefix_states: r.prefix_states,
            tiles_states: r.tiles_states,
            prefix_ns: [11, 12],
            segment_host_ns: 23,
            final_residual_ns: [31, 32],
        }),
    }
}
#[test]
fn ordered_profile_separates_projection_and_mode_domains() {
    let b = ar_bootstrap();
    let old = crate::finite_projection_residual_decode_wire_v1::tests::ar_bootstrap();
    assert_ne!(b.sha256().unwrap(), old.sha256().unwrap());
    let mut wrong = b.clone();
    wrong.schema = old.schema;
    assert!(wrong.sha256().is_err());
    let mut wrong = b;
    wrong.decode = base::tests::bootstrap(InputMode::TeacherForced);
    assert!(wrong.sha256().is_err());
}
#[test]
fn ordered_control_has_one_segment_and_rejects_legacy_extent() {
    let c = control();
    let raw = c.encode();
    assert_eq!(raw.len(), CONTROL_BYTES);
    assert_eq!(Control::decode(&raw).unwrap(), c);
    assert!(base::Control::decode(&raw).is_err());
    assert!(Control::decode(&base::tests::control().encode()).is_err());
    assert_eq!(c.layers[0].segment_host_ns, 23);
}
#[test]
fn ordered_image_identity_and_stream_bounds_fail_closed() {
    let mut b = ar_bootstrap();
    b.projection_residual_image.sha256 = [0; 32];
    assert!(b.sha256().is_err());
    let mut b = ar_bootstrap();
    b.projection_residual_image.bytes = u32::MAX;
    assert!(b.sha256().is_err());
    assert!(profile_sha256([0; 32]).is_err());
    assert!(
        read_bootstrap(&mut &[][..], &mut FrameBudget::new())
            .unwrap()
            .is_none()
    );
}
#[test]
fn ordered_control_requires_both_rank_terminal_states() {
    for rank in 0..2 {
        let mut c = control();
        c.layers[35].tiles_states[rank][0] = 0;
        assert!(c.validate().is_err());
        let mut c = control();
        c.layers[0].prefix_states[rank][154] = 63;
        assert!(c.validate().is_err());
    }
}
