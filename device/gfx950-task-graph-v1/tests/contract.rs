use ferric_gfx950_task_graph_v1::*;

#[test]
fn launch_and_atomic_abi_are_fixed() {
    assert_eq!(WORKGROUP, [128, 1, 1]);
    assert_eq!(AQL_GRID, [256, 1, 1]);
    assert_eq!((TASKS, LANES, ITERATIONS), (7, 128, 16));
    assert_eq!(EMPTY_PROBES, 8);
    assert_eq!(INPUT_WORDS, 896);
    assert_eq!(EXPLICIT_KERNARG_BYTES, 2 * 16 + 13 * 8);
    assert_eq!(LDS_BYTES, 2 * 128 * 4);
    assert_eq!(ALL_TASKS, (1 << TASKS) - 1);
}

#[test]
fn valid_arithmetic_and_owner_packing_fit() {
    assert_eq!(13 * LANES as u32 * MAX_INPUT, 1_703_936);
    assert_eq!((1u32 << (2 * TASKS)) - 1, 16_383);
    assert_eq!(ERROR_STALE_EPOCH | ERROR_DUPLICATE | ERROR_INVALID, 7);
    assert!(TASKS > AQL_GRID[0] as usize / WORKGROUP[0] as usize);
}

#[test]
fn widened_checked_sums_match_the_host_intrinsic_at_boundaries() {
    // Check the arithmetic contract independently of GPU execution. Every pair
    // fits in u64, and the guarded cast agrees with checked u32 arithmetic.
    for left in [0u32, 1, MAX_INPUT, 1_703_936, u32::MAX - 1, u32::MAX] {
        for right in [0u32, 1, 2, MAX_INPUT, u32::MAX - 1, u32::MAX] {
            let wide = left as u64 + right as u64;
            let result = if wide <= u32::MAX as u64 {
                Some(wide as u32)
            } else {
                None
            };
            assert_eq!(result, left.checked_add(right));
        }
    }
}
