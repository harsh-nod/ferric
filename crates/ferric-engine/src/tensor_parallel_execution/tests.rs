use super::{TensorParallelSequenceErrorV1 as Error, TensorParallelSequenceV1 as Sequence};

#[test]
fn sequence_bounds_and_reset_reuse_only_new_prefix() {
    let mut state = Sequence::new(2, 100).unwrap();
    assert_eq!(state.begin(100), Err(Error::InvalidToken));
    assert_eq!(state.begin(99), Ok(0));
    assert_eq!(state.begin(0), Err(Error::Busy));
    assert_eq!(state.reset(), Err(Error::Busy));
    assert_eq!(state.complete(), Ok(()));
    assert_eq!(state.begin(0), Ok(1));
    assert_eq!(state.complete(), Ok(()));
    assert_eq!(state.begin(0), Err(Error::Exhausted));
    assert_eq!(state.reset(), Ok(()));
    assert_eq!(state.epoch(), 1);
    assert_eq!(state.begin(0), Ok(0));
}

#[test]
fn failed_execution_cannot_be_reused_or_reset() {
    let mut state = Sequence::new(8192, 100).unwrap();
    assert_eq!(state.complete(), Err(Error::NotRunning));
    assert_eq!(state.begin(0), Ok(0));
    state.poison();
    assert_eq!(state.complete(), Err(Error::Poisoned));
    assert_eq!(state.begin(0), Err(Error::Poisoned));
    assert_eq!(state.reset(), Err(Error::Poisoned));
    assert_eq!(state.position(), 0);
}

#[test]
fn invalid_capacity_and_epoch_overflow_are_rejected() {
    assert!(Sequence::new(0, 1).is_err());
    assert!(Sequence::new(8193, 1).is_err());
    assert!(Sequence::new(1, 0).is_err());
    let mut state = Sequence::new(1, 1).unwrap();
    state.epoch = u64::MAX;
    assert_eq!(state.reset(), Err(Error::EpochOverflow));
    assert_eq!(state.epoch(), u64::MAX);
}
