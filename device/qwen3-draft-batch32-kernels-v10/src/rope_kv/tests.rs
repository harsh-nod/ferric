use super::*;

#[test]
fn page_boundary_last_page_and_unique_append_slots() {
    let positions = [15_u32, 16, 8191];
    let mut tables = std::vec![u32::MAX;3*512];
    tables[0] = 0;
    tables[512 + 1] = 1;
    tables[2 * 512 + 511] = 511;
    for (row, expected) in [15_usize, 16, 8191].into_iter().enumerate() {
        assert_eq!(
            batch_paged_slot_v10!(&positions, &tables, row, 512, 512),
            expected
        );
    }
    batch_distinct_slots_v10!(&positions, &tables, 3, 512, 512);
    let positions = [16_u32, 16];
    let mut tables = std::vec![u32::MAX;1024];
    tables[1] = 1;
    tables[513] = 1;
    assert!(
        std::panic::catch_unwind(|| batch_distinct_slots_v10!(&positions, &tables, 2, 512, 512))
            .is_err()
    );
}

#[test]
fn invalid_page_positions_and_physical_indices_trap_on_host() {
    for (position, page, stride, pages) in
        [(8192, 0, 512, 512), (16, 0, 1, 512), (0, 512, 512, 512)]
    {
        let positions = [position];
        let tables = std::vec![page;512];
        assert!(
            std::panic::catch_unwind(|| batch_paged_slot_v10!(
                &positions, &tables, 0, stride, pages
            ))
            .is_err()
        );
    }
}

#[test]
fn rope_pairs_retain_signs_rounding_and_finite_guards() {
    for (first, second, cos, sin) in [
        (1.0, -2.0, 1.0, 0.0),
        (1.0, -2.0, 0.0, 1.0),
        (0.5, 0.25, 0.5, -0.5),
    ] {
        let (a, b) = batch_rope_pair_v10!(
            Bf16::from_f32(first).to_bits(),
            Bf16::from_f32(second).to_bits(),
            cos,
            sin
        );
        assert_eq!(a, Bf16::from_f32(first * cos - second * sin).to_bits());
        assert_eq!(b, Bf16::from_f32(second * cos + first * sin).to_bits());
    }
    for (first, cos) in [(f32::NAN, 1.0), (1.0, f32::INFINITY)] {
        assert!(
            std::panic::catch_unwind(|| batch_rope_pair_v10!(
                Bf16::from_f32(first).to_bits(),
                0,
                cos,
                0.0
            ))
            .is_err()
        );
    }
}
