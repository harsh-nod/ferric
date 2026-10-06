//! CPU framing and private state ownership for a distinct finite-worker route.
//! Native execution is a separate explicit trusted-parent engineering mode.

pub mod finite_composition_wire;
pub mod finite_forward_wire_v1;
pub mod finite_guarded_mlp_decode_wire_v1;
pub mod finite_long_wire_v1;
pub mod finite_mlp_tiles_comparison_wire_v1;
pub mod finite_prefix_decode_wire_v1;
pub mod finite_prefix_layer_wire_v1;
pub mod finite_projection_residual_decode_wire_v1;
pub mod finite_projection_residual_layer_wire_v1;
pub mod finite_queued_mlp_comparison_wire_v1;
pub mod finite_queued_projection_comparison_wire_v1;
pub mod finite_rearm_smoke_wire_v1;
pub mod finite_setup_wire_v1;
pub mod finite_tiles_decode_wire_v1;
#[allow(dead_code)]
mod forward_sequence;
mod long_forward_sequence_v1;
pub mod native_cli_v1;
pub mod native_guarded_mlp_decode_cli_v1;
pub mod native_long_cli_v1;
pub mod native_mlp_tiles_comparison_cli_v1;
pub mod native_prefix_decode_cli_v1;
pub mod native_prefix_decode_device_clock_v2;
pub mod native_prefix_decode_device_v1;
pub mod native_prefix_decode_host_v1;
pub mod native_prefix_decode_host_v2;
mod native_prefix_device_recorder_v1;
pub mod native_prefix_layer_cli_v1;
pub mod native_projection_residual_decode_cli_v1;
pub mod native_projection_residual_decode_host_v1;
pub mod native_projection_residual_layer_cli_v1;
pub mod native_queued_mlp_comparison_cli_v1;
pub mod native_queued_projection_comparison_cli_v1;
pub mod native_rearm_smoke_cli_v1;
pub mod native_tiles_decode_cli_v1;
pub mod prefix_decode_device_clock_observation_v2;
pub mod prefix_decode_device_observation_v1;
pub mod prefix_decode_host_observation_v1;
pub mod prefix_decode_host_observation_v2;
pub mod projection_residual_decode_host_observation_v1;
mod rearm_smoke_sequence_v1;

// Private ownership: no raw Group or token is exposed by the CLI.
#[allow(dead_code)]
mod native_catalog;
#[allow(dead_code)]
mod native_setup;
#[allow(dead_code)]
mod resident_artifacts;
#[allow(dead_code)]
mod state_roster;
// The three dispatch boundaries carry explicit native ABI safety contracts.
#[allow(dead_code, unsafe_code)]
mod resident_layer;
#[allow(dead_code)]
mod tail_artifacts;
#[allow(dead_code, unsafe_code)]
mod tail_bindings;
#[allow(dead_code)]
mod tail_head;

pub mod finite_projection_residual_mlp_ordered_wire_v1;
pub mod native_projection_residual_mlp_ordered_v1;
pub mod projection_residual_mlp_ordered_observation_v1;
