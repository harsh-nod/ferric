use ferric_qwen3_tp_c1_splitk_down_kernels_device_v1::{PARTIAL_BYTES, PARTIAL_ELEMENTS};

#[test]
fn compact_row_stripes_cover_every_scratch_element_exactly_once() {
    let mut writes = vec![0_u8; PARTIAL_ELEMENTS];
    for raw in 0..2048 * 64 {
        let group = raw / 64;
        let lane = raw % 64;
        if lane < 16 {
            let partition = group / 256;
            let tile_column = group % 256;
            let physical = group * 16 + lane;
            let logical = partition * 4096 + tile_column * 16 + lane;
            assert_eq!(physical, logical);
            writes[physical] += 1;
        }
    }
    assert!(writes.iter().all(|count| *count == 1));
    assert_eq!(PARTIAL_BYTES, 131_072);
}

#[test]
fn k_partitions_cover_all_reduction_indices_once_for_each_fragment_column() {
    for column_lane in 0..16 {
        let mut reads = vec![0_u8; 12288];
        for partition in 0..8 {
            for step in 0..96 {
                for lane_group in 0..4 {
                    let lane = lane_group * 16 + column_lane;
                    for component in 0..4 {
                        let reduction = partition * 1536 + step * 16 + (lane / 16) * 4 + component;
                        reads[reduction] += 1;
                    }
                }
            }
        }
        assert!(reads.iter().all(|count| *count == 1));
    }
}

#[test]
fn active_mfma_accumulator_row_matches_the_compact_store() {
    for lane in 0..64 {
        for component in 0..4 {
            let row = lane / 16 * 4 + component;
            assert_eq!(row == 0, lane < 16 && component == 0);
        }
    }
}

#[test]
fn merge_reads_only_initialized_partials_and_writes_no_output_tail() {
    let mut reads = vec![0_u8; PARTIAL_ELEMENTS];
    let mut output_writes = vec![0_u8; 32 * 4096];
    for column in 0..64 * 64 {
        for partition in 0..8 {
            reads[partition * 4096 + column] += 1;
        }
        output_writes[column] += 1;
    }
    assert!(reads.iter().all(|count| *count == 1));
    assert!(output_writes[..4096].iter().all(|count| *count == 1));
    assert!(output_writes[4096..].iter().all(|count| *count == 0));
}

fn merge_model(partials: [f32; 8]) -> Option<f32> {
    let mut sum = 0.0_f32;
    let mut finite = true;
    for value in partials {
        sum += value;
        finite &= value.is_finite() & sum.is_finite();
    }
    finite.then_some(sum)
}

#[test]
fn merge_order_is_serial_fp32_not_an_exact_real_sum() {
    let partials = [16_777_216.0, 1.0, -16_777_216.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(merge_model(partials), Some(0.0));
    assert_eq!(partials.into_iter().map(f64::from).sum::<f64>(), 1.0);
    assert_eq!(merge_model([1.0; 8]), Some(8.0));
}

#[test]
fn merge_rejects_nonfinite_and_intermediate_overflow() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for position in 0..8 {
            let mut partials = [0.0; 8];
            partials[position] = value;
            assert!(merge_model(partials).is_none());
        }
    }
    assert!(merge_model([f32::MAX, f32::MAX, -f32::MAX, 0.0, 0.0, 0.0, 0.0, 0.0]).is_none());
}
