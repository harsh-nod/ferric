mod ferric_qwen3_logits_device_v1 {
    pub use ferric_qwen3_all_kernels_device_v1::logits::{
        QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1, QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1,
        QWEN3_LOGITS_VOCABULARY_V1,
    };
}

include!("../../qwen3-logits-v1/tests/reference.rs");
