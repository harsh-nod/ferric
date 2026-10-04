//! CPU arithmetic checks of the actual device macro, not GPU/Math evidence.
use fe2o3_device::Bf16;

include!("../src/head_rope_numerics_v3.rs");
include!("../src/prefix_reciprocal_numerics_v1.rs");

const LOW: u32 = 0x3a80_0000;
const HIGH: u32 = 0x5f80_0000;

// Independent u64 div/mod replaces the device's 24-step u32 restoring loop.
// A separately generated Fraction fixture also checks exponent/result packing.
fn divmod_reference(bits: u32) -> Option<u32> {
    if !(LOW..=HIGH).contains(&bits) {
        return None;
    }
    let exponent = bits >> 23;
    let mantissa = u64::from((bits & 0x7f_ffff) | (1 << 23));
    if mantissa == 1 << 23 {
        return Some((254 - exponent) << 23);
    }
    let numerator = 1_u64 << 47;
    let quotient = numerator / mantissa;
    let remainder = numerator % mantissa;
    let rounded = quotient
        + u64::from(2 * remainder > mantissa || (2 * remainder == mantissa && quotient & 1 != 0));
    Some(((253 - exponent) << 23) + (rounded as u32 - (1 << 23)))
}

fn actual(bits: u32) -> f32 {
    qwen_prefix_reciprocal_rn_v1!(f32::from_bits(bits))
}

#[test]
fn all_power_of_two_denominators_and_adjacent_words() {
    for exponent in 117..=191 {
        let center = exponent << 23;
        for bits in [center - 1, center, center + 1] {
            if let Some(expected) = divmod_reference(bits) {
                assert_eq!(actual(bits).to_bits(), expected, "{bits:08x}");
            } else {
                assert!(actual(bits).is_nan());
            }
        }
    }
    assert_eq!(actual(LOW).to_bits(), 0x4480_0000);
    assert_eq!(actual(HIGH).to_bits(), 0x1f80_0000);
}

#[test]
fn fixed_mantissas_and_4096_stratified_samples_at_every_admitted_exponent() {
    let edges = [
        0, 1, 2, 3, 0x2a_aaaa, 0x3f_ffff, 0x40_0000, 0x55_5555, 0x7f_fffd, 0x7f_fffe, 0x7f_ffff,
    ];
    for exponent in 117..=191 {
        for mantissa in edges
            .into_iter()
            .chain((0..4096).map(|i| (i * 2053) & 0x7f_ffff))
        {
            let bits = (exponent << 23) | mantissa;
            if let Some(expected) = divmod_reference(bits) {
                assert_eq!(actual(bits).to_bits(), expected, "{bits:08x}");
            }
        }
    }
}

#[test]
fn independent_fraction_nearest_neighbor_vectors_match_the_actual_macro() {
    let fixture = include_str!("fixtures/prefix_reciprocal_fraction_v1.tsv");
    let mut rows = 0;
    for line in fixture.lines().filter(|line| !line.starts_with('#')) {
        let (input, output) = line.split_once('\t').unwrap();
        let bits = u32::from_str_radix(input, 16).unwrap();
        let expected = u32::from_str_radix(output, 16).unwrap();
        assert_eq!(actual(bits).to_bits(), expected, "{bits:08x}");
        assert_eq!(divmod_reference(bits), Some(expected), "{bits:08x}");
        rows += 1;
    }
    assert!(rows > 900);
}

#[test]
fn six_permitted_seed_counterexamples_now_round_above_the_power_of_two() {
    for exponent in [126, 127, 128, 150, 190, 191] {
        let denominator = (exponent << 23) - 1;
        let old_seed_result = (254 - exponent) << 23;
        assert_eq!(actual(denominator).to_bits(), old_seed_result + 1);
        assert_ne!(actual(denominator).to_bits(), old_seed_result);
    }
}

#[test]
fn invalid_and_out_of_domain_words_fail_closed() {
    for bits in [
        0,
        1,
        0x007f_ffff,
        0x0080_0000,
        LOW - 1,
        HIGH + 1,
        0x7f7f_ffff,
        0x7f80_0000,
        0x7f80_0001,
        0x7fc0_0000,
        0x8000_0000,
        0x8000_0001,
        0xbf80_0000,
        0xff80_0000,
        0xffff_ffff,
    ] {
        assert!(actual(bits).is_nan(), "{bits:08x}");
        assert_eq!(divmod_reference(bits), None);
    }
}

#[test]
fn macro_evaluates_its_denominator_once() {
    for bits in [3.0_f32.to_bits(), LOW, HIGH, 0, LOW - 1, HIGH + 1, 0xffff_ffff] {
        let mut calls = 0;
        let result = qwen_prefix_reciprocal_rn_v1!({
            calls += 1;
            f32::from_bits(bits)
        });
        assert_eq!(calls, 1);
        let expected = divmod_reference(bits).unwrap_or(f32::NAN.to_bits());
        assert_eq!(result.to_bits(), expected, "{bits:08x}");
    }
}

#[test]
fn rejected_words_at_every_exponent_return_the_exact_nan_sentinel() {
    for exponent in 0..=255_u32 {
        for sign in [0, 0x8000_0000_u32] {
            for mantissa in [0, 1, 0x3f_ffff, 0x40_0000, 0x7f_fffe, 0x7f_ffff] {
                let bits = sign | (exponent << 23) | mantissa;
                if !(LOW..=HIGH).contains(&bits) {
                    assert_eq!(actual(bits).to_bits(), f32::NAN.to_bits(), "{bits:08x}");
                }
            }
        }
    }
}

#[test]
#[ignore = "bounded exhaustive mantissa qualification; run explicitly with optimized test profile"]
fn exhaustive_all_significands_in_one_binade() {
    for bits in 0x3f80_0000..0x4000_0000 {
        assert_eq!(
            actual(bits).to_bits(),
            divmod_reference(bits).unwrap(),
            "{bits:08x}"
        );
    }
}

struct Task {
    raw: [u16; 3072],
    weights: [u16; 256],
    rotary: [f32; 2],
    output: Vec<u16>,
    lane: usize,
    valid: bool,
}
impl Task {
    fn fixture(lane: usize) -> Self {
        Self {
            raw: std::array::from_fn(|i| {
                Bf16::from_f32(((i * 19 % 73) as f32 - 36.0) / 16.0).to_bits()
            }),
            weights: std::array::from_fn(|i| {
                Bf16::from_f32(0.5 + (i % 31) as f32 / 32.0).to_bits()
            }),
            rotary: [0.75, 0.25],
            output: Vec::new(),
            lane,
            valid: true,
        }
    }
    fn lane(&self) -> usize {
        self.lane
    }
    fn reject(&mut self) {
        self.valid = false;
    }
    fn head_input(&self, head: usize, column: usize) -> Option<u16> {
        (self.valid && head < 20 && column < 128).then(|| self.raw[head * 128 + column])
    }
    fn head_weight(&self, head: usize, column: usize) -> Option<u16> {
        (self.valid && head < 20 && column < 128)
            .then(|| self.weights[usize::from(head >= 16) * 128 + column])
    }
    fn rotary(&self, half: usize) -> Option<f32> {
        (self.valid && half < 2).then(|| self.rotary[half])
    }
    fn value(&self, head: usize, half: usize) -> Option<u16> {
        (self.valid && head < 4 && half < 2)
            .then(|| self.raw[2560 + head * 128 + self.lane + half * 64])
    }
    fn write_query_head(&mut self, head: usize, low: u16, high: u16) -> bool {
        if !self.valid || self.output.len() != head * 2 || head >= 16 {
            return false;
        }
        self.output.extend([low, high]);
        true
    }
    fn write_key_value_head(&mut self, head: usize, kl: u16, kh: u16, vl: u16, vh: u16) -> bool {
        if !self.valid || self.output.len() != 32 + head * 4 || head >= 4 {
            return false;
        }
        self.output.extend([kl, kh, vl, vh]);
        true
    }
}

#[test]
fn head_sum_and_inverse_match_the_legacy_cpu_rne_path_for_each_head() {
    let task = Task::fixture(0);
    for head in 0..20 {
        let old = qwen_head_inverse_v3!(&task, head, stabilized, stabilized.sqrt());
        let new = qwen_prefix_head_inverse_v6!(&task, head, stabilized, stabilized.sqrt());
        assert!(old.2 && new.2);
        assert_eq!(old.0.to_bits(), new.0.to_bits());
        assert_eq!(old.1.to_bits(), new.1.to_bits());
    }
}

#[test]
fn post_preserves_both_bf16_narrowings_rotary_and_value_append_for_every_lane() {
    for lane in 0..64 {
        let mut old = Task::fixture(lane);
        let mut new = Task::fixture(lane);
        qwen_head_rope_post_v3!(&mut old, stabilized, stabilized.sqrt());
        qwen_prefix_head_rope_post_v6!(&mut new, stabilized, stabilized.sqrt());
        assert!(old.valid && new.valid);
        assert_eq!(old.output.len(), 48);
        assert_eq!(new.output, old.output);
        for head in 0..4 {
            assert_eq!(new.output[34 + head * 4], new.raw[2560 + head * 128 + lane]);
            assert_eq!(
                new.output[35 + head * 4],
                new.raw[2560 + head * 128 + lane + 64]
            );
        }
    }
}

#[test]
fn rejected_denominator_cannot_publish_a_post_output() {
    for bits in [0, LOW - 1, HIGH + 1, 0x7f80_0000, 0x7fc0_0000, 0xbf80_0000] {
        let mut task = Task::fixture(0);
        qwen_prefix_head_rope_post_v6!(&mut task, stabilized, {
            let _ = stabilized;
            f32::from_bits(bits)
        });
        assert!(!task.valid, "{bits:08x}");
        assert!(task.output.is_empty(), "{bits:08x}");
    }
}

#[test]
fn invalid_input_weight_rotary_and_value_still_reject() {
    for case in 0..6 {
        let mut task = Task::fixture(0);
        match case {
            0 => task.raw[0] = 0x7fc0,
            1 => task.raw[0] = 0x7f7f,
            2 => task.weights[0] = 0x7f80,
            3 => task.rotary[0] = f32::NAN,
            4 => task.rotary[1] = f32::INFINITY,
            _ => task.raw[2560] = 0x7fc0,
        }
        qwen_prefix_head_rope_post_v6!(&mut task, stabilized, stabilized.sqrt());
        assert!(!task.valid, "case {case}");
        assert!(task.output.len() < 48, "case {case}");
    }
}

#[test]
fn prefix_head_macro_changes_are_limited_to_the_reciprocal_operation() {
    fn body(file: &syn::File, name: &str) -> String {
        file.items
            .iter()
            .find_map(|item| {
                let syn::Item::Macro(item) = item else {
                    return None;
                };
                (item.ident.as_ref().is_some_and(|ident| ident == name))
                    .then(|| item.mac.tokens.to_string())
            })
            .unwrap()
    }
    let original = syn::parse_file(include_str!("../src/head_rope_numerics_v3.rs")).unwrap();
    let candidate =
        syn::parse_file(include_str!("../src/prefix_reciprocal_numerics_v1.rs")).unwrap();
    let original_inverse = body(&original, "qwen_head_inverse_v3");
    let candidate_inverse = body(&candidate, "qwen_prefix_head_inverse_v6");
    let replacement = "qwen_prefix_reciprocal_rn_v1 ! (denominator)";
    assert_eq!(candidate_inverse.matches(replacement).count(), 1);
    assert_eq!(
        candidate_inverse.replace(replacement, "1.0_f32 / denominator"),
        original_inverse
    );
    let original_post = body(&original, "qwen_head_rope_post_v3");
    let candidate_post = body(&candidate, "qwen_prefix_head_rope_post_v6");
    assert_eq!(
        candidate_post
            .matches("qwen_prefix_head_inverse_v6")
            .count(),
        1
    );
    assert_eq!(
        candidate_post.replace("qwen_prefix_head_inverse_v6", "qwen_head_inverse_v3"),
        original_post
    );
}
