#![allow(unused_macros)]

use fe2o3_device::Bf16;
use std::cell::Cell;

include!("../src/head_rope_numerics_v3.rs");
include!("../src/prefix_reciprocal_numerics_v1.rs");
include!("../src/prefix_rope_materialized_numerics_v1.rs");

fn pair(a: u16, b: u16, cosine: f32, sine: f32) -> (u16, u16, bool) {
    let (low, high, valid) = qwen_prefix_rope_materialized_pair_v1!(
        Bf16::from_bits(a), Bf16::from_bits(b), cosine, sine
    );
    (low.to_bits(), high.to_bits(), valid)
}

// The constants below are dyadic rational calculations, not Math/OCML
// predictions. This local bit operation independently checks the provider's
// finite FP32-to-BF16 conversion when exposing intermediate boundary examples.
fn rne_bits(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
}

#[test]
fn position_zero_is_identity_for_nonzero_finite_pairs() {
    for (a, b) in [(0x3fc0, 0xbd00), (0xbf80, 0x4000), (1, 0x8001), (0x7f7f, 0xff7f)] {
        assert_eq!(pair(a, b, 1.0, 0.0), (a, b, true));
    }
}

#[test]
fn quarter_turn_uses_split_half_negative_high_then_low() {
    assert_eq!(pair(0x3fc0, 0xbd00, 0.0, 1.0), (0x3d00, 0x3fc0, true));
    assert_eq!(pair(0x3fc0, 0xbd00, 0.0, -1.0), (0xbd00, 0xbfc0, true));
}

#[test]
fn coefficient_halfway_rounds_even_before_multiplication() {
    // 1+1/256 -> 1; 1+3/256 -> 1+1/64.
    assert_eq!(pair(0x3f80, 0, f32::from_bits(0x3f80_8000), 0.0), (0x3f80, 0, true));
    assert_eq!(pair(0x3f80, 0, f32::from_bits(0x3f81_8000), 0.0), (0x3f82, 0, true));
}

#[test]
fn sine_coefficient_has_its_own_bf16_boundary() {
    assert_eq!(pair(0x3f80, 0x3f80, 0.0, f32::from_bits(0x3f80_8000)), (0xbf80, 0x3f80, true));
}

#[test]
fn product_halfway_rounds_before_opposite_sign_addend() {
    // (3/2)*(127/128)=381/256, halfway at 0x3fbe/0x3fbf.
    // The +1/256 addend makes the old fused-final result 0x3fbf;
    // narrowing the first product makes the final halfway round to 0x3fbe.
    let ac = 1.5_f32 * (127.0_f32 / 128.0_f32);
    assert_eq!(rne_bits(ac), 0x3fbe);
    assert_eq!(rne_bits(ac + 1.0 / 256.0), 0x3fbf);
    let (low, _, valid) = pair(0x3fc0, 0xbd00, 127.0 / 128.0, 0.125);
    assert!(valid);
    assert_eq!(low, 0x3fbe);
}

#[test]
fn high_product_has_independent_materialization() {
    // b*c is the same halfway product, followed by a*s=1/256.
    let (_, high, valid) = pair(0x3d00, 0x3fc0, 127.0 / 128.0, 0.125);
    assert!(valid);
    assert_eq!(high, 0x3fbe);
}

#[test]
fn final_addition_still_rounds_bf16_ties_even() {
    assert_eq!(pair(0x3f80, 0xbb80, 1.0, 1.0).0, 0x3f80);
    assert_eq!(pair(0x3f80, 0xbc40, 1.0, 1.0).0, 0x3f82);
}

#[test]
fn negate_before_product_preserves_framework_signed_zero() {
    assert_eq!(pair(0x8000, 0, 1.0, 0.0), (0x8000, 0, true));
    assert_eq!(pair(0, 0x8000, 1.0, 0.0), (0, 0, true));
    assert_eq!(pair(0x8000, 0x8000, 1.0, 0.0), (0, 0x8000, true));
    // Nonzero sine distinguishes negate-then-add from fusing a product sum.
    assert_eq!(pair(0x8000, 0, -0.0, 1.0), (0, 0x8000, true));
}

#[test]
fn exact_cancellation_is_positive_zero() {
    assert_eq!(pair(0x3f80, 0x3f80, 1.0, 1.0), (0, 0x4000, true));
}

#[test]
fn subnormal_product_rounds_before_small_addend() {
    assert_eq!(pair(1, 0, 0.5, 0.0), (0, 0, true));
    assert_eq!(pair(3, 0, 0.5, 0.0).0, 2);
    // 2^-134 rounds to zero before adding 2^-133; one final rounding
    // instead would round 1.5 * 2^-133 up to BF16 bit pattern 2.
    let sine = f32::from_bits(0x0001_0000);
    assert_eq!(pair(1, 0xbf80, 0.5, sine).0, 1);
    assert_eq!(rne_bits(f32::from_bits(0x0000_8000) + sine), 2);
}

#[test]
fn all_nonfinite_inputs_are_rejected() {
    for bits in [0x7f80, 0xff80, 0x7fc0] {
        assert!(!pair(bits, 0, 1.0, 0.0).2);
        assert!(!pair(0, bits, 1.0, 0.0).2);
    }
    for value in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        assert!(!pair(0, 0, value, 0.0).2);
        assert!(!pair(0, 0, 1.0, value).2);
    }
}

#[test]
fn finite_coefficient_that_overflows_bf16_is_rejected() {
    assert!(f32::MAX.is_finite());
    assert_eq!(rne_bits(f32::MAX), 0x7f80);
    assert!(!pair(0, 0, f32::MAX, 0.0).2);
    assert!(!pair(0, 0, 1.0, f32::MAX).2);
}

#[test]
fn finite_product_that_overflows_bf16_is_not_hidden_by_cancellation() {
    // (129/128 * 2^64)*(254/128 * 2^63)
    // = (2-2^-13)*2^127, finite FP32 but beyond the BF16 RNE threshold.
    let a = Bf16::from_bits(0x5f81).to_f32();
    let c = Bf16::from_bits(0x5f7e).to_f32();
    let product = a * c;
    assert!(product.is_finite());
    assert_eq!(rne_bits(product), 0x7f80);
    assert_eq!(product + (-a) * c, 0.0);
    assert!(!pair(0x5f81, 0x5f81, c, c).2);
}

#[test]
fn finite_bf16_products_with_overflowing_sum_are_rejected() {
    assert!(!pair(0x7f7f, 0xff7f, 1.0, 1.0).2);
}

struct Task {
    lane: usize,
    a: u16,
    b: u16,
    cosine: Option<f32>,
    sine: Option<f32>,
    valid: bool,
    bad_value: bool,
    fail_query: Option<usize>,
    reads: Cell<usize>,
    rotary_reads: Cell<usize>,
    rejects: usize,
    queries: [Option<(u16, u16)>; 16],
    kv: [Option<(u16, u16, u16, u16)>; 4],
}

impl Task {
    fn new(lane: usize) -> Self {
        Self { lane, a: 0x3fc0, b: 0xbd00, cosine: Some(1.0), sine: Some(0.0),
            valid: true, bad_value: false, fail_query: None, reads: Cell::new(0),
            rotary_reads: Cell::new(0), rejects: 0, queries: [None; 16], kv: [None; 4] }
    }
    fn lane(&self) -> usize { self.lane }
    fn head_input(&self, head: usize, column: usize) -> Option<u16> {
        self.reads.set(self.reads.get() + 1);
        assert!(head < 20 && column < 128);
        self.valid.then_some(if column < 64 { self.a } else { self.b })
    }
    fn head_weight(&self, head: usize, column: usize) -> Option<u16> {
        assert!(head < 20 && column < 128);
        self.valid.then_some(0x3f80)
    }
    fn rotary(&self, part: usize) -> Option<f32> {
        self.rotary_reads.set(self.rotary_reads.get() + 1);
        if !self.valid { return None; }
        match part { 0 => self.cosine, 1 => self.sine, _ => panic!("rotary part") }
    }
    fn value(&self, head: usize, half: usize) -> Option<u16> {
        assert!(head < 4 && half < 2);
        if !self.valid { None }
        else if self.bad_value { Some(0x7fc0) }
        else { Some(if half == 0 { 0x3f01 + head as u16 } else { 0xbf21 + head as u16 }) }
    }
    fn reject(&mut self) { self.valid = false; self.rejects += 1; }
    fn write_query_head(&mut self, head: usize, low: u16, high: u16) -> bool {
        if !self.valid || self.fail_query == Some(head) { return false; }
        assert!(self.queries[head].is_none());
        self.queries[head] = Some((low, high)); true
    }
    fn write_key_value_head(&mut self, head: usize, low: u16, high: u16, vl: u16, vh: u16) -> bool {
        if !self.valid { return false; }
        assert!(self.kv[head].is_none());
        self.kv[head] = Some((low, high, vl, vh)); true
    }
}

fn post(task: &mut Task) {
    // A controlled positive sqrt result isolates RoPE; the unchanged inverse
    // and weighted-pair macros still execute. This is not an OCML sqrt test.
    qwen_prefix_head_rope_materialized_post_v1!(task, stabilized, { let _ = stabilized; 1.0_f32 });
}

#[test]
fn all_lanes_keep_twenty_head_routes_and_value_bits() {
    for lane in 0..64 {
        let mut task = Task::new(lane);
        post(&mut task);
        assert!(task.valid);
        assert_eq!(task.rejects, 0);
        assert_eq!(task.reads.get(), 20 * (128 + 2));
        assert_eq!(task.rotary_reads.get(), 40);
        assert_eq!(task.queries, [Some((task.a, task.b)); 16]);
        for head in 0..4 { assert_eq!(task.kv[head], Some((task.a, task.b, 0x3f01 + head as u16, 0xbf21 + head as u16))); }
    }
}

#[test]
fn post_uses_materialized_arithmetic_and_preserves_raw_values() {
    let mut task = Task::new(63);
    task.cosine = Some(127.0 / 128.0);
    task.sine = Some(0.125);
    post(&mut task);
    assert!(task.valid);
    assert_eq!(task.queries[0].unwrap().0, 0x3fbe);
    assert_eq!(task.kv[3].unwrap().0, 0x3fbe);
    assert_eq!(task.kv[3].unwrap().2, 0x3f04);
    assert_eq!(task.kv[3].unwrap().3, 0xbf24);
}

#[test]
fn missing_coefficient_poison_is_sticky_without_shortening_head_loop() {
    let mut task = Task::new(0);
    task.sine = None;
    post(&mut task);
    assert!(!task.valid);
    assert_eq!(task.rejects, 20);
    assert_eq!(task.reads.get(), 2600);
    assert_eq!(task.rotary_reads.get(), 40);
    assert_eq!(task.queries, [None; 16]);
    assert_eq!(task.kv, [None; 4]);
}

#[test]
fn failed_write_preserves_prefix_and_rejects_remaining_heads() {
    let mut task = Task::new(7);
    task.fail_query = Some(1);
    post(&mut task);
    assert!(!task.valid);
    assert_eq!(task.rejects, 19);
    assert_eq!(task.reads.get(), 2600);
    assert_eq!(task.queries[0], Some((task.a, task.b)));
    assert!(task.queries[1..].iter().all(Option::is_none));
    assert_eq!(task.kv, [None; 4]);
}

#[test]
fn nonfinite_value_is_rejected_without_changing_completed_query_writes() {
    let mut task = Task::new(0);
    task.bad_value = true;
    post(&mut task);
    assert!(!task.valid);
    assert_eq!(task.rejects, 4);
    assert_eq!(task.queries, [Some((task.a, task.b)); 16]);
    assert_eq!(task.kv, [None; 4]);
    assert_eq!(task.reads.get(), 2600);
}

#[test]
fn selected_entry_changes_only_include_and_post_selector() {
    let original = include_str!("fixtures/v7_lib.rs");
    let candidate = include_str!("../src/lib.rs");
    let include = "include!(\"prefix_reciprocal_numerics_v1.rs\");";
    let selector = "qwen_prefix_head_rope_post_v6!";
    assert_eq!(original.matches(include).count(), 1);
    assert_eq!(original.matches(selector).count(), 1);
    let expected = original.replace(include, &format!("{include}\ninclude!(\"prefix_rope_materialized_numerics_v1.rs\");"))
        .replace(selector, "qwen_prefix_head_rope_materialized_post_v1!");
    // Formatting copies may change whitespace, never the token sequence.
    let compact = |text: &str| text.split_whitespace().collect::<String>();
    assert_eq!(compact(candidate), compact(&expected));
}
