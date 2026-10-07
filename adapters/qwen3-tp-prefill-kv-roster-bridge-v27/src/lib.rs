#![no_std]
#![forbid(unsafe_code)]

//! Strings generated in a separate build unit; no device SDK in this library.

include!(concat!(env!("OUT_DIR"), "/compiler_names.rs"));

#[cfg(test)]
mod tests {
    use super::COMPILER_NAMES;

    #[test]
    fn generated_roster_retains_one_marker_name_and_the_original_export() {
        assert_eq!(COMPILER_NAMES.len(), 1);
        assert!(!COMPILER_NAMES[0].0.is_empty());
        assert_eq!(
            COMPILER_NAMES[0].1,
            "ferric_qwen3_tp_prefill16_kv_copy_bf16_v27"
        );
    }
}
