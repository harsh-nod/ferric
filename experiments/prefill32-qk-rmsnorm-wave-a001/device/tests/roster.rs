use ferric_prefill32_qk_rmsnorm_wave_device_r1::{ROOTS_R1, compiler_expectation_roster_r1};

#[test]
fn generated_roster_has_the_distinct_kernel_identity() {
    let roster = compiler_expectation_roster_r1();
    assert_eq!(roster.len(), 1);
    assert_eq!(ROOTS_R1, ["ferric_qwen3_prefill32_qk_wave_rmsnorm_bf16_r1"]);
    assert_eq!(roster[0].export_name(), ROOTS_R1[0]);
    assert_ne!(roster[0].generated_host_contract_identity(), [0; 32]);
}
