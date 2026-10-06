use super::*;
#[test]
fn prefix_model_setup_requires_complete_tail_before_the_opener() {
    require_tail(true, true).unwrap();
    for (tail, images) in [(false, false), (false, true), (true, false)] {
        assert!(require_tail(tail, images).is_err());
    }
}
