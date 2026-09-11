use super::*;

#[test]
fn complete_physical_page_range_preserves_logical_slot_mapping() {
    let positions = [0_u32, 15, 16, 8191];
    let mut tables = std::vec![u32::MAX; positions.len() * 512];
    for page in 0..16384_u32 {
        for (row, position) in positions.iter().copied().enumerate() {
            tables[row * 512 + position as usize / 16] = page;
            assert_eq!(
                batch_paged_slot_v9!(&positions, &tables, row, 512, 16384),
                page as usize * 16 + position as usize % 16
            );
        }
    }
    assert_eq!(batch_paged_slot_v9!(&positions, &tables, 3, 512, 16384), 262143);
}

#[test]
fn distinct_rows_share_pages_only_at_distinct_offsets() {
    for rows in [1_usize, 16, 17, 32] {
        let positions: std::vec::Vec<_> = (0..rows as u32).collect();
        let mut tables = std::vec![u32::MAX; rows * 2];
        for row in 0..rows {
            tables[row * 2] = 511;
            tables[row * 2 + 1] = 16383;
        }
        batch_distinct_slots_v9!(&positions, &tables, rows, 2, 16384);
    }
}

#[test]
fn invalid_mapping_and_duplicate_final_slot_trap_before_append() {
    for (position, page, stride, pages) in [
        (8192, 0, 512, 16384),
        (8191, 16384, 512, 16384),
        (16, 0, 1, 16384),
        (0, u32::MAX, 1, 16384),
        (0, 512, 1, 512),
    ] {
        let table = std::vec![page; stride];
        assert!(std::panic::catch_unwind(|| {
            batch_paged_slot_v9!(&[position], &table, 0, stride, pages)
        }).is_err());
    }
    let positions = [15_u32, 8191];
    let mut tables = std::vec![u32::MAX; 1024];
    tables[0] = 16383;
    tables[1023] = 16383;
    assert!(std::panic::catch_unwind(|| {
        batch_distinct_slots_v9!(&positions, &tables, 2, 512, 16384);
    }).is_err());
}
