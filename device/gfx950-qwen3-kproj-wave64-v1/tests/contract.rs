use fe2o3_device::{Index1D, RowStriped2D, WriteOnlyDisjointSlice};
use ferric_gfx950_qwen3_kproj_wave64_v1::*;

#[test]
fn two_complete_waves_per_group_and_one_wave_per_row() {
    assert_eq!((INPUTS, OUTPUTS, WEIGHTS), (1024, 1024, 1_048_576));
    assert_eq!((LANES_PER_ROW, TERMS_PER_LANE), (64, 16));
    assert_eq!(WORKGROUP, [128, 1, 1]);
    assert_eq!(AQL_GRID, [65536, 1, 1]);
    assert_eq!(AQL_GRID[0] / WORKGROUP[0], 512);
    assert_eq!(AQL_GRID[0] as usize / LANES_PER_ROW, OUTPUTS);
    assert_eq!(EXPLICIT_KERNARG_BYTES, 48);
    assert_eq!(
        core::mem::size_of::<WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>>(),
        16
    );
}

#[test]
fn coalesced_lane_partition_visits_every_column_exactly_once() {
    let mut visits = [0u8; INPUTS];
    for term in 0..TERMS_PER_LANE {
        for lane in 0..LANES_PER_ROW {
            let column = lane + term * LANES_PER_ROW;
            visits[column] += 1;
            if lane != 0 {
                assert_eq!(column - (lane - 1 + term * LANES_PER_ROW), 1);
            }
        }
    }
    assert!(visits.iter().all(|&count| count == 1));
}
