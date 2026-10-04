use super::*;
#[test]
fn projection_decode_both_residuals_select_candidate_and_keep_original_pair() {
    let original = [10, 11];
    let candidate = [20, 21];
    for first in [true, false] {
        for rank in 0..2 {
            assert_eq!(
                *select_residual(Some(&candidate), rank, |_| panic!("fallback")).unwrap(),
                candidate[rank]
            );
            let (skip, output) = residual_roots(first, 100 + rank, 200 + rank, 300 + rank);
            assert_eq!(
                (skip, output),
                if first {
                    (100 + rank, 200 + rank)
                } else {
                    (200 + rank, 300 + rank)
                }
            );
        }
    }
    assert_eq!(original, [10, 11]);
    assert!(select_residual(Some(&candidate), 2, |_| panic!("fallback")).is_err());
}
#[test]
fn projection_decode_none_selection_preserves_original_residual_kernel() {
    let original = [10, 11];
    for rank in 0..2 {
        assert_eq!(
            *select_residual(None, rank, |r| Ok(&original[r])).unwrap(),
            original[rank]
        );
    }
}
