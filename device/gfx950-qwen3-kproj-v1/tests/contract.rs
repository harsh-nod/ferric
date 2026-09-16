use ferric_gfx950_qwen3_kproj_v1::*;

#[test]
fn checkpoint_projection_and_launch_are_fixed() {
    assert_eq!((INPUTS, OUTPUTS, WEIGHTS), (1024, 1024, 1_048_576));
    assert_eq!(WORKGROUP, [128, 1, 1]);
    assert_eq!(AQL_GRID, [1024, 1, 1]);
    assert_eq!(AQL_GRID[0] / WORKGROUP[0], 8);
    assert_eq!(EXPLICIT_KERNARG_BYTES, 3 * 16);
    assert_eq!(core::mem::size_of::<u16>(), 2);
    assert_eq!(core::mem::size_of::<f32>(), 4);
}
