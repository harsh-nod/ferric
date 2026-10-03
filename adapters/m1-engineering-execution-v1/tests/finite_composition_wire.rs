// Compile the exact same source against the unchanged parent's dependencies.
// The shared module intentionally contains no native-generation type imports.
#[path = "../../tp-peer-finite-engineering-worker-v1/src/finite_composition_wire.rs"]
mod finite_composition_wire;

#[path = "../../tp-peer-finite-engineering-worker-v1/src/wire_golden_tests.rs"]
mod golden;
