use fe2o3_device::Bf16 as SdkBf16;
use ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1::reference_bf16::Bf16 as ReferenceBf16;

#[test]
fn host_model_matches_sdk_for_every_bf16_and_rounding_boundary() {
    for high in 0..=u16::MAX {
        let reference = ReferenceBf16::from_bits(high);
        let sdk = SdkBf16::from_bits(high);
        assert_eq!(reference.to_f32().to_bits(), sdk.to_f32().to_bits());
        assert_eq!(reference.is_finite(), sdk.is_finite());
        for low in [0, 1, 0x7fff, 0x8000, 0x8001, 0xffff] {
            let value = f32::from_bits((u32::from(high) << 16) | low);
            assert_eq!(
                ReferenceBf16::from_f32(value).to_bits(),
                SdkBf16::from_f32(value).to_bits(),
                "FP32 bits {:08x}",
                value.to_bits()
            );
        }
    }
}

#[test]
fn selection_requires_the_explicit_feature() {
    assert_eq!(
        ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1::EXPERIMENT_ENABLED,
        cfg!(feature = "fixed-safe-gate-up-r1")
    );
}
