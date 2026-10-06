use core::sync::atomic::{AtomicU32, Ordering};
use fe2o3_device::{Bf16, KernelMarkerV1, WriteOnlyDisjointSlice};
use std::cell::{Cell, RefCell};
use std::vec::Vec;

macro_rules! plain_word {
    ($words:expr, $index:expr) => {
        $words[$index]
    };
}

macro_rules! traced_word {
    ($words:expr, $index:expr) => {
        $words.load($index, Ordering::Acquire)
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

fn valid_combined() -> Vec<u32> {
    let mut words = std::vec![0; 552];
    words[..4].copy_from_slice(&[1, 0, 0, 31]);
    words[4..9].copy_from_slice(&[1, 96, 96, 1, 64]);
    words[9..14].copy_from_slice(&[1, 96, 96, 1, 64]);
    words[14..22].fill(u32::MAX);
    words[22] = 3;
    words[23..31].fill(u32::MAX);
    words[31] = 3;
    for (index, word) in words[32..290].iter_mut().enumerate() {
        *word = (index % 64) as u32 + 1;
    }
    words[290..548].fill(64);
    words[548..].copy_from_slice(&[0x12345678, 0x87654321, u32::MAX, 31]);
    words
}

// Independent range-based reference: no production macro or index list is used.
// The four publication words deliberately do not enter the state predicate.
fn combined_oracle(words: &[u32]) -> bool {
    words.len() == 552
        && words[..4] == [1, 0, 0, 31]
        && words[4..9] == [1, 96, 96, 1, 64]
        && words[9..14] == [1, 96, 96, 1, 64]
        && words[14..22].iter().all(|word| *word == u32::MAX)
        && words[22] == 3
        && words[23..31].iter().all(|word| *word == u32::MAX)
        && words[31] == 3
        && words[32..290].iter().all(|word| (1..=64).contains(word))
        && words[290..548].iter().all(|word| *word == 64)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Load(usize, Ordering),
    Store(usize, u32, Ordering),
}

struct CombinedWords {
    words: RefCell<Vec<u32>>,
    events: RefCell<Vec<Event>>,
}

impl CombinedWords {
    fn new(words: Vec<u32>) -> Self {
        Self {
            words: RefCell::new(words),
            events: RefCell::new(Vec::new()),
        }
    }

    fn len(&self) -> usize {
        self.words.borrow().len()
    }

    fn load(&self, index: usize, ordering: Ordering) -> u32 {
        self.events.borrow_mut().push(Event::Load(index, ordering));
        self.words.borrow()[index]
    }

    fn store(&self, index: usize, value: u32, ordering: Ordering) {
        self.events
            .borrow_mut()
            .push(Event::Store(index, value, ordering));
        self.words.borrow_mut()[index] = value;
    }

    fn snapshot(&self) -> Vec<u32> {
        self.words.borrow().clone()
    }
}

#[test]
fn combined_layout_has_exact_prefix_suffix_and_atomic_storage() {
    assert_eq!(crate::MLP_STATE_WORDS_V2, 548);
    assert_eq!(crate::MLP_GUARD_OFFSET_WORDS_V2, 548);
    assert_eq!(crate::MLP_GUARD_WORDS_V2, 4);
    assert_eq!(crate::MLP_COMBINED_WORDS_V2, 552);
    assert_eq!(
        crate::MLP_GUARD_OFFSET_WORDS_V2 + crate::MLP_GUARD_WORDS_V2,
        crate::MLP_COMBINED_WORDS_V2
    );
    assert_eq!(core::mem::size_of::<AtomicU32>(), 4);
    assert_eq!(core::mem::align_of::<AtomicU32>(), 4);
    assert_eq!(core::mem::size_of::<[AtomicU32; 548]>(), 2192);
    assert_eq!(core::mem::size_of::<[AtomicU32; 552]>(), 2208);
}

#[test]
fn combined_predicate_matches_independent_oracle_for_every_word() {
    let original = valid_combined();
    assert!(combined_oracle(&original));
    for index in 0..552 {
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
            let mut words = original.clone();
            words[index] = value;
            assert_eq!(
                mlp_combined_state_valid_v2!(&words, plain_word),
                combined_oracle(&words),
                "word {index}, value {value}"
            );
        }
    }
}

#[test]
fn combined_predicate_reads_only_all_548_state_words_even_on_failure() {
    for bad in [None, Some(0), Some(31), Some(289), Some(547)] {
        let mut initial = valid_combined();
        if let Some(index) = bad {
            initial[index] = 0;
        }
        let words = CombinedWords::new(initial);
        assert_eq!(
            mlp_combined_state_valid_v2!(&words, traced_word),
            bad.is_none()
        );
        let events = words.events.borrow();
        assert_eq!(events.len(), 548);
        let mut indices = events
            .iter()
            .map(|event| match event {
                Event::Load(index, Ordering::Acquire) => *index,
                other => panic!("unexpected state event: {other:?}"),
            })
            .collect::<Vec<_>>();
        indices.sort_unstable();
        assert_eq!(indices, (0..548).collect::<Vec<_>>());
    }
}

#[test]
fn combined_wrong_extents_refuse_before_any_state_load() {
    for len in [0, 1, 4, 547, 548, 549, 550, 551, 553, 1024] {
        let words = CombinedWords::new(std::vec![0; len]);
        assert!(!mlp_combined_state_valid_v2!(&words, traced_word));
        assert!(words.events.borrow().is_empty());
        assert!(!combined_oracle(&words.snapshot()));
    }
}

#[test]
fn combined_publication_preserves_prefix_and_releases_verdict_last() {
    for valid in [false, true] {
        let original = valid_combined();
        let words = CombinedWords::new(original.clone());
        publish_combined_guard_v2!(&words, 0xfedcba98, 0x87654321, valid, traced_store);
        let verdict = if valid { 1 } else { 2 };
        assert_eq!(&words.snapshot()[..548], &original[..548]);
        assert_eq!(
            &words.snapshot()[548..],
            &[0xfedcba98, 0x87654321, verdict, 0]
        );
        assert_eq!(
            *words.events.borrow(),
            [
                Event::Store(548, 0xfedcba98, Ordering::Relaxed),
                Event::Store(549, 0x87654321, Ordering::Relaxed),
                Event::Store(551, 0, Ordering::Relaxed),
                Event::Store(550, verdict, Ordering::Release),
            ]
        );
    }
}

#[test]
fn combined_invalid_generation_or_extent_publishes_nothing() {
    for (len, lo, hi) in [
        (552, 0, 0),
        (0, 1, 0),
        (4, 1, 0),
        (548, 1, 0),
        (551, 1, 0),
        (553, 1, 0),
    ] {
        let original = std::vec![0xdeadbeef; len];
        let words = CombinedWords::new(original.clone());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            publish_combined_guard_v2!(&words, lo, hi, true, traced_store);
        }));
        assert!(result.is_err());
        assert!(words.events.borrow().is_empty());
        assert_eq!(words.snapshot(), original);
    }
}

#[test]
fn combined_generation_halves_and_stale_suffix_tags_remain_exact() {
    for (lo, hi) in [
        (1, 0),
        (0, 1),
        (u32::MAX, u32::MAX),
        (0x80000000, 0x80000000),
        (u32::MAX, 0),
    ] {
        let words = CombinedWords::new(valid_combined());
        publish_combined_guard_v2!(&words, lo, hi, true, traced_store);
        let snapshot = words.snapshot();
        // This CPU-only suffix view models R2's unchanged four-word argument.
        let guard = &snapshot[548..552];
        assert!(guard_current_v1!(guard, lo, hi, plain_word));
        assert!(!guard_current_v1!(guard, lo ^ 1, hi, plain_word));
        assert!(!guard_current_v1!(guard, lo, hi ^ 1, plain_word));
    }
}

#[test]
fn combined_revalidation_replaces_every_suffix_word_after_all_loads() {
    let words = CombinedWords::new(valid_combined());
    let valid = mlp_combined_state_valid_v2!(&words, traced_word);
    publish_combined_guard_v2!(&words, 7, 9, valid, traced_store);
    {
        let events = words.events.borrow();
        assert_eq!(events.len(), 552);
        assert!(
            events[..548]
                .iter()
                .all(|event| matches!(event, Event::Load(_, Ordering::Acquire)))
        );
        assert_eq!(
            &events[548..],
            &[
                Event::Store(548, 7, Ordering::Relaxed),
                Event::Store(549, 9, Ordering::Relaxed),
                Event::Store(551, 0, Ordering::Relaxed),
                Event::Store(550, 1, Ordering::Release),
            ]
        );
    }
    words.words.borrow_mut()[547] = 63;
    words.events.borrow_mut().clear();
    let valid = mlp_combined_state_valid_v2!(&words, traced_word);
    publish_combined_guard_v2!(&words, 8, 9, valid, traced_store);
    let snapshot = words.snapshot();
    assert_eq!(&snapshot[548..], &[8, 9, 2, 0]);
    assert!(!guard_current_v1!(&snapshot[548..], 7, 9, plain_word));
    assert!(!guard_current_v1!(&snapshot[548..], 8, 9, plain_word));
    assert_eq!(
        words.events.borrow().last(),
        Some(&Event::Store(550, 2, Ordering::Release))
    );
}

#[test]
fn combined_real_atomics_reject_each_corrupted_prefix_word() {
    let original = valid_combined();
    for bad in 0..548 {
        let mut values = original.clone();
        values[bad] = if (32..290).contains(&bad) {
            65
        } else {
            original[bad] ^ 1
        };
        assert!(!combined_oracle(&values));
        let words: [AtomicU32; 552] = std::array::from_fn(|index| AtomicU32::new(values[index]));
        let valid = mlp_combined_state_valid_v2!(&words, atomic_word);
        publish_combined_guard_v2!(&words, 17, 19, valid, atomic_store);
        assert!(!valid);
        for index in 0..548 {
            assert_eq!(words[index].load(Ordering::Acquire), values[index]);
        }
        let guard: [u32; 4] =
            std::array::from_fn(|index| words[548 + index].load(Ordering::Acquire));
        assert_eq!(guard, [17, 19, 2, 0]);
        assert!(!guard_current_v1!(&guard, 17, 19, plain_word));
    }
}

#[test]
fn combined_suffix_guards_gate_both_r2_payloads_without_changing_arithmetic() {
    for bad_rank in [None, Some(0), Some(1)] {
        let first = CombinedWords::new(valid_combined());
        let second = CombinedWords::new(valid_combined());
        publish_combined_guard_v2!(&first, 7, 9, bad_rank != Some(0), traced_store);
        publish_combined_guard_v2!(&second, 7, 9, bad_rank != Some(1), traced_store);
        let first_snapshot = first.snapshot();
        let second_snapshot = second.snapshot();
        let reads = Cell::new(0);
        let output = Cell::new(None);
        with_current_guards_v1!(
            &first_snapshot[548..], &second_snapshot[548..], 7, 9, plain_word;
            {
                let value = projection_residual_v1!(
                    { reads.set(reads.get() + 1); 0.5 },
                    { reads.set(reads.get() + 1); 0.25 },
                    { reads.set(reads.get() + 1); 0x3f80 }
                );
                output.set(Some(value));
            }
        );
        assert_eq!(reads.get(), 3 * usize::from(bad_rank.is_none()));
        assert_eq!(output.get(), bad_rank.is_none().then_some(0x3fe0));
    }
}

#[test]
fn combined_generated_markers_keep_one_atomic_root_and_readonly_r2_guards() {
    type Guard = crate::kernels::ferric_qwen3_mlp_state_guard_v2_gpu::Marker;
    type R2 = crate::kernels::ferric_qwen3_tp2_guarded_projection_residual_bf16_v2_gpu::Marker;
    let _: fn(&[AtomicU32], u32, u32) = <Guard as KernelMarkerV1>::FUNCTION;
    let _: fn(
        &[f32],
        &[f32],
        &[u16],
        WriteOnlyDisjointSlice<u16>,
        &[AtomicU32],
        &[AtomicU32],
        u32,
        u32,
    ) = <R2 as KernelMarkerV1>::FUNCTION;
    assert_eq!(<Guard as KernelMarkerV1>::LOGICAL_NAME, crate::ROOTS_V2[0]);
    assert_eq!(<Guard as KernelMarkerV1>::EXPORT_NAME, crate::ROOTS_V2[0]);
    assert_eq!(<R2 as KernelMarkerV1>::LOGICAL_NAME, crate::ROOTS_V2[1]);
    assert_eq!(<R2 as KernelMarkerV1>::EXPORT_NAME, crate::ROOTS_V2[1]);
    assert_ne!(crate::ROOTS_V2[0], crate::ROOTS_V2[1]);
    assert_eq!(
        crate::ROOTS_V2,
        [
            "ferric_qwen3_mlp_state_guard_v2",
            "ferric_qwen3_tp2_guarded_projection_residual_bf16_v2",
        ]
    );
}

#[test]
fn combined_source_contract_has_one_unsplit_argument_and_literal_store_offsets() {
    let kernels = include_str!("kernels.rs");
    let guard = kernels
        .split("pub fn ferric_qwen3_mlp_state_guard_v2(")
        .nth(1)
        .unwrap()
        .split("#[kernel")
        .next()
        .unwrap();
    let signature = guard
        .split(") {")
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        signature,
        "state_guard: &[AtomicU32], generation_lo: u32, generation_hi: u32,"
    );
    assert!(guard.contains("state_guard.len() != 552"));
    assert!(guard.contains("(generation_lo | generation_hi) == 0"));
    assert!(guard.contains("thread::launch_extent_1d() != 64"));
    assert!(guard.contains("thread::index_1d().get() == 0"));
    assert!(guard.contains("mlp_combined_state_valid_v2!(state_guard, load_atomic_word_v1)"));
    assert!(!guard.contains("split_at"));
    assert!(!guard.contains("state_guard["));
    assert!(!guard.contains("as_ptr"));
    assert!(!guard.contains("unsafe"));
    let macros = include_str!("guard.rs");
    for expected in [
        "$store!(state_guard, 548, lo, core::sync::atomic::Ordering::Relaxed);",
        "$store!(state_guard, 549, hi, core::sync::atomic::Ordering::Relaxed);",
        "$store!(state_guard, 551, 0, core::sync::atomic::Ordering::Relaxed);",
        "$store!(state_guard, 550, verdict, core::sync::atomic::Ordering::Release);",
    ] {
        assert!(macros.contains(expected));
    }
    assert_eq!(kernels.matches("pub fn ").count(), 2);
    assert!(!kernels.contains("ferric_qwen3_mlp_state_guard_v1"));
    assert!(!kernels.contains("ferric_qwen3_tp2_guarded_projection_residual_bf16_v1"));
}
