#![no_std]
#![forbid(unsafe_code)]

//! Strings generated in a separate build unit; no device SDK in this library.

include!(concat!(env!("OUT_DIR"), "/compiler_names.rs"));

#[cfg(test)]
mod tests {
    use super::COMPILER_NAMES;

    #[test]
    fn generated_roster_retains_full_and_wave_profile_names() {
        assert_eq!(COMPILER_NAMES.len(), 15);
        assert!(
            COMPILER_NAMES
                .iter()
                .all(|(logical, export)| !logical.is_empty() && !export.is_empty())
        );
        assert_eq!(
            COMPILER_NAMES
                .iter()
                .filter(|(_, export)| !export.contains("_mfma_"))
                .count(),
            13
        );
        assert_eq!(
            COMPILER_NAMES
                .iter()
                .filter(|(_, export)| *export == "qwen3_rmsnorm_v1")
                .count(),
            1
        );
    }
}
