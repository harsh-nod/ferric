use ferric_gfx950_decoder_layer_f32_v1::{
    AQL_GRID, CHECKPOINT_OFFSETS, CHECKPOINTS_PER_REQUEST, EXPLICIT_KERNARG_BYTES, HEAD_DIM,
    HIDDEN, INPUT_FLOATS, INPUTS_PER_REQUEST, INTERMEDIATE, KV_HEADS, OUTPUT_FLOATS, PREFIX_TOKENS,
    QUERY_HEADS, REQUESTS, WEIGHT_FLOATS, WEIGHT_OFFSETS, WORKGROUP,
};

#[test]
fn fixed_shape_abi_and_two_multiwave_workgroups_are_exact() {
    assert_eq!(
        (HIDDEN, QUERY_HEADS, KV_HEADS, HEAD_DIM, INTERMEDIATE),
        (4, 2, 1, 2, 4)
    );
    assert_eq!(PREFIX_TOKENS, 2);
    assert_eq!(
        (REQUESTS, INPUTS_PER_REQUEST, CHECKPOINTS_PER_REQUEST),
        (256, 12, 40)
    );
    assert_eq!(
        (INPUT_FLOATS, WEIGHT_FLOATS, OUTPUT_FLOATS),
        (3072, 110, 10240)
    );
    assert_eq!(EXPLICIT_KERNARG_BYTES, 48);
    assert_eq!(AQL_GRID, [256, 1, 1]);
    assert_eq!(WORKGROUP, [128, 1, 1]);
    assert_eq!(AQL_GRID[0] / WORKGROUP[0], 2);
    assert_eq!(WORKGROUP[0] / 64, 2);
}

#[test]
fn packed_records_have_complete_nonoverlapping_stage_boundaries() {
    assert_eq!(
        WEIGHT_OFFSETS,
        [0, 4, 20, 28, 36, 38, 40, 56, 60, 76, 92, 108, 109, 110]
    );
    assert_eq!(
        CHECKPOINT_OFFSETS,
        [0, 4, 8, 10, 12, 16, 20, 24, 28, 32, 36, 40]
    );
    assert!(
        WEIGHT_OFFSETS
            .windows(2)
            .all(|offsets| offsets[0] < offsets[1])
    );
    assert!(
        CHECKPOINT_OFFSETS
            .windows(2)
            .all(|offsets| offsets[0] < offsets[1])
    );
}
