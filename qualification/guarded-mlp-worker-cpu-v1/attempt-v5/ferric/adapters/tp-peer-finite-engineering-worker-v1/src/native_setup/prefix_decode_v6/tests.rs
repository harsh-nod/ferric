use super::*;
#[test]
fn prefix_decode_setup_refuses_missing_tail_before_the_opener() {
    for (tail, images) in [(false, false), (false, true), (true, false)] {
        assert!(require_tail(tail, images).is_err());
    }
    require_tail(true, true).unwrap();
}
#[test]
fn prefix_decode_setup_requires_explicit_selection_not_image_presence() {
    for (mlp, prefix) in [(false, false), (true, false), (false, true)] {
        assert!(
            PrefixExecution::DecodeFour
                .validate_images(mlp, prefix)
                .is_err()
        );
    }
    PrefixExecution::DecodeFour
        .validate_images(true, true)
        .unwrap();
    for (mlp, prefix) in [(false, false), (true, false), (true, true)] {
        PrefixExecution::LayerOnly
            .validate_images(mlp, prefix)
            .unwrap();
    }
    assert!(
        PrefixExecution::LayerOnly
            .validate_images(false, true)
            .is_err()
    );
    assert_ne!(PrefixExecution::LayerOnly, PrefixExecution::DecodeFour);
}
