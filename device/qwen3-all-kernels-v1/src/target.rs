//! Target selection for the same twelve aggregate kernel bodies.
//!
//! These are engineering source requirements, not artifact or launch authority.

#[cfg(all(feature = "gfx942", feature = "gfx950"))]
compile_error!("select exactly one Qwen3 device target: gfx942 or gfx950");
#[cfg(not(any(feature = "gfx942", feature = "gfx950")))]
compile_error!("select exactly one Qwen3 device target: gfx942 or gfx950");

/// Exact AMD processor selected for all aggregate entrypoints.
pub const QWEN3_DEVICE_CPU_V1: &str = if cfg!(feature = "gfx950") {
    "gfx950"
} else {
    "gfx942"
};

/// Exact target ID selected for all aggregate entrypoints.
pub const QWEN3_DEVICE_TARGET_V1: &str = if cfg!(feature = "gfx950") {
    "gfx950:xnack-"
} else {
    "gfx942:xnack-"
};

/// Required rustc target features shared by both Wave64 implementations.
pub const QWEN3_DEVICE_RUSTC_FEATURES_V1: &str = "-wavefrontsize32,+wavefrontsize64,-xnack";

/// The aggregate ABI retains the COV6 hidden argument tail on both targets.
pub const QWEN3_DEVICE_CODE_OBJECT_VERSION_V1: u8 = 6;
