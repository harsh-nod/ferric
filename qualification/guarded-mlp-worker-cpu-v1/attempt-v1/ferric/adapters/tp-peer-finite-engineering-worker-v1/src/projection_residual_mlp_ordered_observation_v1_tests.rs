use super::*;
pub(crate) fn report() -> SharedReport {
    let old = crate::projection_residual_decode_host_observation_v1::tests::shared_report();
    let mut v = serde_json::to_value(old).unwrap();
    v["schema"] = SHARED_ENVELOPE.into();
    v["policy"] = "ordered-shared-full".into();
    v["observation"]["schema"] = SCHEMA.into();
    v["observation"]["bootstrap"]["schema"] =
        crate::finite_projection_residual_mlp_ordered_wire_v1::SCHEMA.into();
    let mut r: SharedReport = serde_json::from_value(v).unwrap();
    let o = &mut r.observation;
    o.profile_sha256 = o.bootstrap.sha256().unwrap();
    let mut chain = crate::finite_prefix_decode_wire_v1::Chain::new(
        o.bootstrap.decode.registration,
        o.profile_sha256,
    );
    for c in &mut o.completions {
        c.control.bytes =
            crate::finite_projection_residual_mlp_ordered_wire_v1::CONTROL_BYTES as u32;
        c.chain = chain.advance(c);
    }
    o.transcript_sha256 = chain.digest();
    r
}
#[test]
fn ordered_report_is_distinct_from_default_and_shared_legacy_decoders() {
    let r = report();
    r.validate().unwrap();
    let raw = serde_json::to_vec(&r).unwrap();
    SharedReport::decode(&raw).unwrap();
    assert!(
        crate::projection_residual_decode_host_observation_v1::SharedReport::decode(&raw).is_err()
    );
    assert!(crate::projection_residual_decode_host_observation_v1::Report::decode(&raw).is_err());
}
#[test]
fn ordered_report_requires_shared_policy_on_all_seven_snapshots() {
    for i in 0..7 {
        let mut v = serde_json::to_value(report()).unwrap();
        v["observation"]["snapshots"][i]["shared_full_currentness"] = false.into();
        assert!(SharedReport::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    }
}
#[test]
fn ordered_report_profile_control_extent_and_chain_are_bound() {
    let mut r = report();
    r.observation.profile_sha256 = [9; 32];
    assert!(r.validate().is_err());
    let mut r = report();
    r.observation.completions[0].control.bytes = 241960;
    assert!(r.validate().is_err());
    let mut r = report();
    r.observation.transcript_sha256 = [9; 32];
    assert!(r.validate().is_err());
}
#[test]
fn ordered_configuration_is_outside_snapshot_intervals_and_no_accuracy_authority() {
    let mut r = report();
    r.configuration_host_ns = u64::MAX;
    r.validate().unwrap();
    let mut r = report();
    r.observation.performance_claim = true;
    assert!(r.validate().is_err());
    let mut r = report();
    r.observation.gpu_time = true;
    assert!(r.validate().is_err());
}
