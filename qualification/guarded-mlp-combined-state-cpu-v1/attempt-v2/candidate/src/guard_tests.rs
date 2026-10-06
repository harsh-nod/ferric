use core::sync::atomic::{AtomicU32, Ordering};
use fe2o3_device::Bf16;
use std::cell::{Cell, RefCell};
use std::vec::Vec;

macro_rules! plain_word {
    ($words:expr, $index:expr) => {
        $words[$index]
    };
}

macro_rules! traced_word {
    ($words:expr, $index:expr) => {
        $words.load($index)
    };
}

macro_rules! traced_store {
    ($words:expr, $index:expr, $value:expr, $ordering:expr) => {
        $words.store($index, $value, $ordering)
    };
}

macro_rules! atomic_word {
    ($words:expr, $index:expr) => {
        $words[$index].load(Ordering::Acquire)
    };
}

macro_rules! atomic_store {
    ($words:expr, $index:expr, $value:expr, $ordering:expr) => {
        $words[$index].store($value, $ordering)
    };
}

fn valid_state() -> [u32; 548] {
    let mut words = [0; 548];
    words[..4].copy_from_slice(&[1, 0, 0, 31]);
    words[4..14].copy_from_slice(&[1, 96, 96, 1, 64, 1, 96, 96, 1, 64]);
    words[14..22].fill(u32::MAX);
    words[22] = 3;
    words[23..31].fill(u32::MAX);
    words[31] = 3;
    for (index, word) in words[32..290].iter_mut().enumerate() {
        *word = (index % 64) as u32 + 1;
    }
    words[290..].fill(64);
    words
}

// Independent range/slice formulation of RT validate_final_state. This does
// not use the production macro's index lists or expected-value grouping.
fn host_oracle(words: &[u32]) -> bool {
    words.len() == 548
        && words[0] == 1
        && words[1] == 0
        && words[2] == 0
        && words[3] == 31
        && words[4..9] == [1, 96, 96, 1, 64]
        && words[9..14] == [1, 96, 96, 1, 64]
        && words[14..22] == [u32::MAX; 8]
        && words[22] == 3
        && words[23..31] == [u32::MAX; 8]
        && words[31] == 3
        && words[32..290].iter().all(|owner| (1..=64).contains(owner))
        && words[290..548] == [64; 258]
}

struct TracedWords {
    words: Vec<u32>,
    reads: RefCell<Vec<usize>>,
}

impl TracedWords {
    fn new(words: &[u32]) -> Self {
        Self {
            words: words.to_vec(),
            reads: RefCell::new(Vec::new()),
        }
    }
    fn len(&self) -> usize {
        self.words.len()
    }
    fn load(&self, index: usize) -> u32 {
        self.reads.borrow_mut().push(index);
        self.words[index]
    }
}

struct GuardWrites {
    words: RefCell<[u32; 4]>,
    writes: RefCell<Vec<(usize, u32, Ordering)>>,
}

impl GuardWrites {
    fn new() -> Self {
        Self {
            words: RefCell::new([0; 4]),
            writes: RefCell::new(Vec::new()),
        }
    }
    fn len(&self) -> usize {
        4
    }
    fn store(&self, index: usize, value: u32, order: Ordering) {
        self.writes.borrow_mut().push((index, value, order));
        self.words.borrow_mut()[index] = value;
    }
}

#[test]
fn full_state_predicate_matches_independent_host_ranges() {
    let words = valid_state();
    assert!(host_oracle(&words));
    assert!(mlp_state_valid_v1!(&words, plain_word));
}

#[test]
fn every_state_word_mutation_matches_independent_host_oracle() {
    let original = valid_state();
    for index in 0..548 {
        for value in [
            0,
            1,
            2,
            3,
            31,
            63,
            64,
            65,
            96,
            u32::MAX,
            original[index] ^ 1,
        ] {
            let mut words = original;
            words[index] = value;
            assert_eq!(
                mlp_state_valid_v1!(&words, plain_word),
                host_oracle(&words),
                "word {index} value {value}"
            );
        }
    }
}

#[test]
fn all_owner_values_one_through_sixty_four_are_valid() {
    for owner in 1..=64 {
        let mut words = valid_state();
        words[32..290].fill(owner);
        assert!(host_oracle(&words));
        assert!(mlp_state_valid_v1!(&words, plain_word));
    }
}

#[test]
fn zero_state_and_wrong_extents_are_invalid_without_out_of_bounds_reads() {
    assert!(!mlp_state_valid_v1!(&[0_u32; 548], plain_word));
    for len in [0, 1, 32, 547, 549] {
        let words = TracedWords::new(&std::vec![0; len]);
        assert!(!mlp_state_valid_v1!(&words, traced_word));
        assert!(words.reads.borrow().is_empty());
        assert!(!host_oracle(&words.words));
    }
}

#[test]
fn predicate_loads_all_548_distinct_words_even_after_an_early_failure() {
    for bad in [None, Some(0), Some(31), Some(289), Some(547)] {
        let mut original = valid_state();
        if let Some(index) = bad {
            original[index] = 0;
        }
        let words = TracedWords::new(&original);
        assert_eq!(mlp_state_valid_v1!(&words, traced_word), bad.is_none());
        let mut indices = words.reads.borrow().clone();
        assert_eq!(indices.len(), 548);
        indices.sort_unstable();
        assert_eq!(indices, (0..548).collect::<Vec<_>>());
    }
}

#[test]
fn guard_tags_preserve_both_full_u32_halves_and_refuse_generation_zero() {
    for (lo, hi) in [
        (1, 0),
        (0, 1),
        (u32::MAX, u32::MAX),
        (0x80000000, 0x80000000),
    ] {
        let words = [lo, hi, 1, 0];
        assert!(guard_current_v1!(&words, lo, hi, plain_word));
        assert!(!guard_current_v1!(&words, lo ^ 1, hi, plain_word));
        assert!(!guard_current_v1!(&words, lo, hi ^ 1, plain_word));
    }
    assert!(!guard_current_v1!(&[0, 0, 1, 0], 0, 0, plain_word));
    assert!(!guard_current_v1!(&[0_u32; 4], 1, 0, plain_word));
}

#[test]
fn all_four_guard_words_are_checked_including_invalid_nonboolean_verdicts() {
    let original = [7, 9, 1, 0];
    for index in 0..4 {
        for value in [0, 1, 2, 3, 7, 9, u32::MAX] {
            let mut guard = original;
            guard[index] = value;
            assert_eq!(
                guard_current_v1!(&guard, 7, 9, plain_word),
                guard == original
            );
        }
    }
}

#[test]
fn guard_wrong_extent_reads_nothing_and_valid_shape_acquires_verdict_first() {
    for words in [&[][..], &[7][..], &[7, 9, 1][..], &[7, 9, 1, 0, 0][..]] {
        let guard = TracedWords::new(words);
        assert!(!guard_current_v1!(&guard, 7, 9, traced_word));
        assert!(guard.reads.borrow().is_empty());
    }
    let guard = TracedWords::new(&[7, 9, 1, 0]);
    assert!(guard_current_v1!(&guard, 7, 9, traced_word));
    assert_eq!(*guard.reads.borrow(), [2, 0, 1, 3]);
}

#[test]
fn valid_guard_publishes_tag_and_reserved_before_release_verdict() {
    let guard = GuardWrites::new();
    publish_guard_v1!(&guard, 0xfedcba98, 0x87654321, true, traced_store);
    assert_eq!(*guard.words.borrow(), [0xfedcba98, 0x87654321, 1, 0]);
    assert_eq!(
        *guard.writes.borrow(),
        [
            (0, 0xfedcba98, Ordering::Relaxed),
            (1, 0x87654321, Ordering::Relaxed),
            (3, 0, Ordering::Relaxed),
            (2, 1, Ordering::Release),
        ]
    );
}

#[test]
fn invalid_state_publishes_current_generation_with_invalid_verdict() {
    let mut words = valid_state();
    words[547] = 63;
    let guard = GuardWrites::new();
    publish_guard_v1!(
        &guard,
        7,
        9,
        mlp_state_valid_v1!(&words, plain_word),
        traced_store
    );
    assert_eq!(*guard.words.borrow(), [7, 9, 2, 0]);
    assert_eq!(
        guard.writes.borrow().last(),
        Some(&(2, 2, Ordering::Release))
    );
    assert!(!guard_current_v1!(&*guard.words.borrow(), 7, 9, plain_word));
}

#[test]
fn generation_zero_cannot_publish_any_guard_word() {
    let guard = GuardWrites::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        publish_guard_v1!(&guard, 0, 0, true, traced_store);
    }));
    assert!(result.is_err());
    assert!(guard.writes.borrow().is_empty());
    assert_eq!(*guard.words.borrow(), [0; 4]);
}

#[test]
fn either_bad_guard_prevents_all_payload_reads_and_output_writes() {
    for bad_rank in 0..2 {
        for field in 0..4 {
            let mut guards = [[7_u32, 9, 1, 0]; 2];
            guards[bad_rank][field] ^= 1;
            let guard0 = TracedWords::new(&guards[0]);
            let guard1 = TracedWords::new(&guards[1]);
            let payload_reads = Cell::new(0);
            let output = Cell::new(None);
            with_current_guards_v1!(&guard0, &guard1, 7, 9, traced_word; {
                let value = projection_residual_v1!(
                    { payload_reads.set(payload_reads.get() + 1); 0.5 },
                    { payload_reads.set(payload_reads.get() + 1); 0.25 },
                    { payload_reads.set(payload_reads.get() + 1); 0x3f80 }
                );
                output.set(Some(value));
            });
            assert_eq!(payload_reads.get(), 0);
            assert_eq!(output.get(), None);
            assert_eq!(*guard0.reads.borrow(), [2, 0, 1, 3]);
            assert_eq!(*guard1.reads.borrow(), [2, 0, 1, 3]);
        }
    }
}

#[test]
fn both_guards_finish_before_rank_ordered_payload_evaluation() {
    let guard0 = TracedWords::new(&[7, 9, 1, 0]);
    let guard1 = TracedWords::new(&[7, 9, 1, 0]);
    let payload_order = RefCell::new(Vec::new());
    let output = Cell::new(None);
    with_current_guards_v1!(&guard0, &guard1, 7, 9, traced_word; {
        let value = projection_residual_v1!(
            {
            assert_eq!(guard0.reads.borrow().len(), 4);
            assert_eq!(guard1.reads.borrow().len(), 4);
            payload_order.borrow_mut().push(0); 0.5
            },
            { payload_order.borrow_mut().push(1); 0.25 },
            { payload_order.borrow_mut().push(2); 0x3f80 }
        );
        output.set(Some(value));
    });
    assert_eq!(output.get(), Some(0x3fe0));
    assert_eq!(*payload_order.borrow(), [0, 1, 2]);
}

#[test]
fn matching_guards_preserve_original_r2_arithmetic() {
    let guard0 = [7_u32, 9, 1, 0];
    let guard1 = [7_u32, 9, 1, 0];
    for p0 in [0.0, -0.0, 0.5, 1.0, 16_777_216.0] {
        for p1 in [0.0, -0.0, 0.25, 1.0 / 256.0, -16_777_216.0] {
            for residual in [0, 0x8000, 0x3f80, 0xbf80, 0x0080] {
                let output = Cell::new(None);
                with_current_guards_v1!(&guard0, &guard1, 7, 9, plain_word; {
                    output.set(Some(projection_residual_v1!(p0, p1, residual)));
                });
                assert_eq!(
                    output.get(),
                    Some(projection_residual_v1!(p0, p1, residual))
                );
            }
        }
    }
}

#[test]
fn real_atomic_state_and_guard_storage_follow_the_same_predicate() {
    let words = valid_state();
    let state: [AtomicU32; 548] = std::array::from_fn(|index| AtomicU32::new(words[index]));
    let guard: [AtomicU32; 4] = std::array::from_fn(|_| AtomicU32::new(0));
    publish_guard_v1!(
        &guard,
        7,
        9,
        mlp_state_valid_v1!(&state, atomic_word),
        atomic_store
    );
    assert!(guard_current_v1!(&guard, 7, 9, atomic_word));
    state[547].store(0, Ordering::Release);
    publish_guard_v1!(
        &guard,
        8,
        9,
        mlp_state_valid_v1!(&state, atomic_word),
        atomic_store
    );
    assert!(!guard_current_v1!(&guard, 7, 9, atomic_word));
    assert!(!guard_current_v1!(&guard, 8, 9, atomic_word));
    assert_eq!(guard[2].load(Ordering::Acquire), 2);
}
