use ferric_tp_peer_finite_engineering_worker_v1::finite_composition_wire;

#[path = "../src/wire_golden_tests.rs"]
mod golden;

#[test]
fn current_kfd_typed_state_exports_are_linked_without_legacy_graph_group() {
    assert!(std::mem::size_of::<fe2o3_kfd::Gfx950EngineeringPeerWaveOutputStateV5>() > 0);
    assert!(std::mem::size_of::<fe2o3_kfd::Gfx950EngineeringPeerWaveMlpStateV1>() > 0);
}
