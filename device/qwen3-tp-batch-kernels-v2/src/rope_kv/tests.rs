use super::*;

#[test]
fn page_slot_mapping_handles_noncontiguous_pages_and_token_boundaries() {
    let positions = [0_u32, 15, 16, 31];
    let tables = [3_u32, 0, 2, 1, 1, 0, 0, 2];
    let expected = [48, 47, 0, 47];
    for (row, slot) in expected.into_iter().enumerate() {
        assert_eq!(batch_paged_slot_v2!(&positions, &tables, row, 2, 4), slot);
    }
}

#[test]
fn selected_distinct_rows_can_share_one_physical_page_at_different_offsets() {
    let positions = [14_u32, 15, 16];
    let tables = [1_u32, u32::MAX, 1, u32::MAX, 1, 2];
    batch_distinct_slots_v2!(&positions, &tables, 3, 2, 3);
}

#[test]
#[should_panic]
fn duplicate_selected_physical_slot_traps_before_append() {
    let positions = [2_u32, 18];
    let tables = [1_u32, 0, 0, 1];
    batch_distinct_slots_v2!(&positions, &tables, 2, 2, 2);
}

#[test]
#[should_panic]
fn selected_unmapped_page_traps() {
    let _ = batch_paged_slot_v2!(&[16_u32], &[0_u32, u32::MAX], 0, 2, 2);
}

#[test]
#[should_panic]
fn context_past_page_table_extent_traps() {
    let _ = batch_paged_slot_v2!(&[16_u32], &[0_u32], 0, 1, 2);
}

#[test]
fn independent_row_rope_pairs_preserve_the_original_fp32_order() {
    for row in 0..16 {
        for lane in 0..64 {
            let first = Bf16::from_f32((row * 64 + lane) as f32 / 128.0).to_bits();
            let second = Bf16::from_f32(-((lane + 1) as f32) / 32.0).to_bits();
            let cos = ((row * 5 + lane) as f32 / 64.0).cos();
            let sin = ((row * 5 + lane) as f32 / 64.0).sin();
            let (a, b) = batch_rope_pair_v2!(first, second, cos, sin);
            let x = Bf16::from_bits(first).to_f32();
            let y = Bf16::from_bits(second).to_f32();
            assert_eq!(a, Bf16::from_f32(x * cos - y * sin).to_bits());
            assert_eq!(b, Bf16::from_f32(y * cos + x * sin).to_bits());
        }
    }
}

#[test]
#[should_panic]
fn nonfinite_row_rope_table_traps() {
    let _ = batch_rope_pair_v2!(0, 0, f32::NAN, 0.0);
}
