use super::*;
fn config() -> OrderedDecodeConfig {
    let mut decode = super::super::tests::config();
    decode.mode = wire::InputMode::Autoregressive;
    OrderedDecodeConfig {
        schema: REQUEST_SCHEMA.into(),
        decode,
        projection_residual_image: FilePin {
            path: "/task/ordered-projection.hsaco".into(),
            bytes: 1,
            sha256: hash(&[10]),
        },
    }
}
#[test]
fn ordered_parent_config_requires_distinct_schema_ar_and_projection_pin() {
    let c = config();
    c.validate().unwrap();
    OrderedDecodeConfig::parse(&serde_json::to_vec(&c).unwrap()).unwrap();
    let mut bad = c.clone();
    bad.schema = "FerricFiniteProjectionResidualDecodeRequestV1".into();
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.decode.mode = wire::InputMode::TeacherForced;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.projection_residual_image = bad.decode.images.residual.clone();
    assert!(bad.validate().is_err());
    let mut bad = c;
    bad.projection_residual_image.bytes = u64::MAX;
    assert!(bad.validate().is_err());
}
#[test]
fn ordered_parent_bootstrap_is_not_plain_projection_profile() {
    let c = config();
    let base = wire::tests::bootstrap(wire::InputMode::Autoregressive);
    let old =
        super::super::projection::bootstrap(base.clone(), &c.projection_residual_image).unwrap();
    let new = bootstrap(base, &c.projection_residual_image).unwrap();
    assert_eq!(selected(&old), new);
    assert_ne!(old.sha256().unwrap(), new.sha256().unwrap());
}
#[test]
fn ordered_parent_worker_selector_has_no_legacy_fallback() {
    let c = config();
    let d = HostDiagnostic::Ordered(
        "/task/sidecar.json".into(),
        c.projection_residual_image.clone(),
    );
    assert_eq!(
        worker_flag(Some(&d), true).unwrap(),
        "--engineering-native-projection-residual-mlp-ordered-v1"
    );
    assert!(worker_flag(Some(&d), false).is_err());
    let old = HostDiagnostic::Device("/task/device.json".into());
    assert!(worker_flag(Some(&old), true).is_err());
    assert_eq!(
        worker_flag(None, true).unwrap(),
        "--engineering-native-projection-residual-decode-v1"
    );
}
#[test]
fn ordered_parent_checks_real_new_control_bytes_chain_and_old_reader_refusal() {
    let b = crate::finite_projection_residual_mlp_ordered_wire_v1::tests::ar_bootstrap();
    let mut request = wire::tests::request(&b.decode, 0, None);
    request.profile_sha256 = b.sha256().unwrap();
    let mut producer = wire::Chain::new(b.decode.registration, request.profile_sha256);
    let (mut response, _, payload) = wire::tests::completed(&request, &mut producer, 7);
    let control = crate::finite_projection_residual_mlp_ordered_wire_v1::tests::control();
    let mut producer = wire::Chain::new(b.decode.registration, request.profile_sha256);
    let wire::Event::Completed(value) = &mut response.event else {
        panic!()
    };
    value.control = old::part(&control.encode());
    value.chain = producer.advance(value);
    let mut checked = wire::Chain::new(b.decode.registration, request.profile_sha256);
    validate_completion_bytes(
        &request,
        &response,
        &control.encode(),
        &payload,
        &mut checked,
    )
    .unwrap();
    let mut raw = Vec::new();
    projection_wire::write_response(
        &mut raw,
        &mut wire::FrameBudget::new(),
        &response,
        Some(&control),
        &payload,
    )
    .unwrap();
    assert!(wire::read_response(&mut &raw[..], &mut wire::FrameBudget::new()).is_err());
    let mut bad = control.encode();
    bad[0] ^= 1;
    let mut checked = wire::Chain::new(b.decode.registration, request.profile_sha256);
    assert!(validate_completion_bytes(&request, &response, &bad, &payload, &mut checked).is_err());
}
