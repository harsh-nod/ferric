#![cfg(feature = "tp-batch-engineering")]
#![allow(dead_code)]

#[path = "../src/bin/tp_peer_worker.rs"]
mod tp_peer_worker;
#[path = "../src/bin/tp_rank_worker.rs"]
mod tp_rank_worker;
#[path = "../src/bin/tp_worker.rs"]
#[allow(clippy::struct_excessive_bools)]
mod tp_worker;

#[test]
fn native_peer_entry_remains_outside_the_safe_adapter() {
    assert!(include_str!("../Cargo.toml").contains("unsafe_code = \"forbid\""));
    let child = include_str!("../../tp-peer-engineering-worker-v4/Cargo.toml");
    assert!(child.contains("[workspace]"));
    assert!(child.contains("unsafe_op_in_unsafe_fn = \"forbid\""));
}
