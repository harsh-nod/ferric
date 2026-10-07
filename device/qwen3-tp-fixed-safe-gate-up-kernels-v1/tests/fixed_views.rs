use fe2o3_device::StridedReadView2D;
use ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1::fixed_host;

#[test]
fn sdk_fixed_views_match_direct_reads_and_exclude_capacity_tails() {
    let columns = [0, 1, 6143, 12_287];
    let mut weights = vec![0_u32; 12_288 * 2048];
    for column in columns {
        for inner in 0..2048 {
            weights[column * 2048 + inner] = (column * 2048 + inner) as u32;
        }
    }
    let right = StridedReadView2D::from_shared_slice(&weights, 0, 12_288, 2048, 2048).unwrap();
    for capacity in [2048, 32 * 2048] {
        let mut activation = vec![u32::MAX; capacity];
        for (index, value) in activation[..2048].iter_mut().enumerate() {
            *value = index as u32;
        }
        let left = StridedReadView2D::from_shared_slice(&activation, 0, 1, 2048, 2048).unwrap();
        for column in columns {
            for group in 0..32 {
                for lane in 0..64 {
                    let [left_index, right_index] = fixed_host::read_indices(column, group, lane).unwrap();
                    let inner = group * 64 + lane;
                    assert_eq!(left.load_or(0, inner, u32::MAX), activation[left_index]);
                    assert_eq!(right.load_or(column, inner, u32::MAX), weights[right_index]);
                }
            }
        }
        assert_eq!(left.load_or(0, 2048, 17), 17);
        assert_eq!(left.load_or(1, 0, 19), 19);
        assert!(activation[2048..].iter().all(|value| *value == u32::MAX));
    }
    assert_eq!(right.load_or(12_288, 0, 23), 23);
    assert_eq!(right.load_or(0, 2048, 29), 29);
}

#[test]
fn sdk_fixed_views_reject_short_storage_before_reads() {
    let activation = [0_u32; 2047];
    let weights = [0_u32; 2048];
    assert!(StridedReadView2D::from_shared_slice(&activation, 0, 1, 2048, 2048).is_err());
    assert!(StridedReadView2D::from_shared_slice(&weights, 0, 12_288, 2048, 2048).is_err());
}
