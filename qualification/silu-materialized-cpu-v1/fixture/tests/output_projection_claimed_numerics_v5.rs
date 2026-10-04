use std::cell::Cell;

#[macro_use]
#[path = "../src/output_projection_numerics_v5.rs"]
mod claimed;

const K: usize = 2048;
const N: usize = 4096;

fn decode(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn bf16(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
}

#[derive(Clone, Copy)]
enum Case {
    Ones,
    Fraction,
    Alternating,
    Basis,
    WaveOrder,
    Cancellation,
    Subnormal,
    Underflow,
    Pattern(usize),
    Nan,
    Infinity,
    ProductOverflow,
    PartialOverflow,
    ReductionOverflow,
}

fn input(case: Case, inner: usize) -> u16 {
    match case {
        Case::Fraction => bf16(0.5),
        Case::Basis => {
            if inner == 2047 {
                bf16(2.0)
            } else {
                0
            }
        }
        Case::Subnormal => 1,
        Case::Underflow => bf16(2.0_f32.powi(-120)),
        Case::Pattern(rank) => bf16(((inner * 13 + rank * 7) % 31) as f32 / 16.0 - 0.875),
        Case::Nan if inner == 63 => 0x7fc0,
        Case::Infinity if inner == 63 => 0x7f80,
        Case::ProductOverflow => 0x7f7f,
        _ => bf16(1.0),
    }
}

fn weight(case: Case, column: usize, inner: usize) -> u16 {
    match case {
        Case::Fraction => bf16(0.25),
        Case::Alternating => bf16(if inner % 2 == 0 { 1.0 } else { -1.0 }),
        Case::Basis => {
            if inner == 2047 {
                bf16(-3.0)
            } else {
                0
            }
        }
        Case::WaveOrder => match inner {
            0 => bf16(16777216.0),
            1 => bf16(1.0),
            64 => bf16(-16777216.0),
            _ => 0,
        },
        Case::Cancellation => match inner {
            0 => bf16(16777216.0),
            64 => bf16(1.0),
            128 => bf16(-16777216.0),
            _ => 0,
        },
        Case::Underflow => bf16(2.0_f32.powi(-30)),
        Case::Pattern(rank) => {
            bf16(((column * 7 + inner * 11 + rank * 17) % 43) as f32 / 32.0 - 0.625)
        }
        Case::ProductOverflow => bf16(2.0),
        Case::PartialOverflow => 0x7f7f,
        Case::ReductionOverflow => {
            if inner < 64 {
                0x7f7f
            } else {
                0
            }
        }
        _ => bf16(1.0),
    }
}

// Independent staged arithmetic model. The real handler below executes the
// production macros, while this model specifies lane-strided addition order.
fn staged(case: Case, column: usize) -> ([f32; 64], [f32; 64]) {
    let partials = std::array::from_fn(|lane| {
        let mut accumulator = 0.0_f32;
        for inner in (lane..K).step_by(64) {
            let product = decode(input(case, inner)) * decode(weight(case, column, inner));
            accumulator += product;
        }
        accumulator
    });
    let mut reduced = partials;
    for offset in [1, 2, 4, 8, 16, 32] {
        let before = reduced;
        reduced = std::array::from_fn(|lane| before[lane] + before[lane ^ offset]);
    }
    (partials, reduced)
}

fn gamma_up(operations: f64, unit: f64) -> f64 {
    let product = operations * unit;
    (product / (1.0 - product)).next_up()
}

// Independent scalar FP64 reference, not the lane tree. Acceptance is fixed
// before GPU evaluation and scales with absolute products, not |dot|, so
// cancellation does not make a relative-error threshold meaningless.
fn reference_bound(case: Case, column: usize) -> (f64, f64) {
    let mut reference = 0.0_f64;
    let mut absolute_sum = 0.0_f64;
    for inner in 0..K {
        let product =
            f64::from(decode(input(case, inner))) * f64::from(decode(weight(case, column, inner)));
        assert!(product.is_finite());
        reference += product;
        absolute_sum += product.abs();
    }
    // Every product-to-output path has at most one product rounding,
    // 32 sequential additions and six XOR additions: gamma39.
    // BF16 products are exactly representable in FP64; its reference sum has
    // at most2048 additions. Outward rounding also bounds summation of |a*w|.
    let g32 = gamma_up(39.0, 2.0_f64.powi(-24));
    let g64 = gamma_up(2048.0, 2.0_f64.powi(-53));
    let absolute_upper = (absolute_sum / (1.0 - g64).next_down()).next_up();
    let relative = ((g32 + g64).next_up() * absolute_upper).next_up();
    // 2048 products +2048 lane additions +63 tree additions, each with
    // at most half of the smallest FP32 subnormal as additive roundoff.
    let underflow =
        (4159.0 * 2.0_f64.powi(-150) / (1.0 - 39.0 * 2.0_f64.powi(-24)).next_down()).next_up();
    (reference, (relative + underflow).next_up())
}

fn within_bound(actual: f32, reference: f64, bound: f64) -> bool {
    if !actual.is_finite() || !reference.is_finite() || !bound.is_finite() || bound < 0.0 {
        return false;
    }
    let difference = (f64::from(actual) - reference).abs();
    (if difference == 0.0 {
        0.0
    } else {
        difference.next_up()
    }) <= bound
}

// CPU stand-in, not a GPU collective or progress proof. Each invocation checks
// its actual local partial against the independent model before returning that
// model's full-wave sum. Invalid lanes may supply NaN after view rejection.
struct Subgroup<'a> {
    lane: usize,
    columns: &'a [([f32; 64], [f32; 64])],
    calls: Cell<usize>,
    allow_rejected: bool,
    poison_reduction: bool,
}

impl Subgroup<'_> {
    fn reduce_sum_f32<const WIDTH: usize>(&self, partial: f32) -> f32 {
        assert_eq!(WIDTH, 64);
        let column = self.calls.get();
        assert!(column < N);
        self.calls.set(column + 1);
        let (partials, sums) = &self.columns[column % self.columns.len()];
        if !(self.allow_rejected && partial.is_nan()) {
            if partial.is_nan() {
                assert!(partials[self.lane].is_nan());
            } else {
                assert_eq!(partial.to_bits(), partials[self.lane].to_bits());
            }
        }
        if self.poison_reduction {
            f32::NAN
        } else {
            sums[self.lane]
        }
    }
}

// API-shaped test double, not a forged provider view. The provider unit tests
// separately cover real claims, lifetimes, disjoint roots and ordered writes.
struct Task {
    case: Case,
    lane: usize,
    valid: bool,
    inputs: Cell<usize>,
    weights: Cell<usize>,
    output: Vec<u32>,
    missing_input: bool,
    missing_weight: bool,
    fail_write: bool,
    missing_weight_at: Option<usize>,
    fail_write_at: Option<usize>,
}

impl Task {
    fn new(case: Case, lane: usize) -> Self {
        Self {
            case,
            lane,
            valid: true,
            inputs: Cell::new(0),
            weights: Cell::new(0),
            output: Vec::new(),
            missing_input: false,
            missing_weight: false,
            fail_write: false,
            missing_weight_at: None,
            fail_write_at: None,
        }
    }
    fn lane(&self) -> usize {
        self.lane
    }
    fn reject(&mut self) {
        self.valid = false;
    }
    fn input(&self, inner: usize) -> Option<u16> {
        let call = self.inputs.get();
        assert_eq!(inner, self.lane + 64 * (call % 32));
        assert!(inner < K && call < N * 32);
        self.inputs.set(call + 1);
        if !self.valid || self.missing_input {
            None
        } else {
            Some(input(self.case, inner))
        }
    }
    fn weight(&self, column: usize, inner: usize) -> Option<u16> {
        let call = self.weights.get();
        assert_eq!(inner, self.lane + 64 * (call % 32));
        assert_eq!(column, call / 32);
        assert!(column < N && inner < K);
        self.weights.set(call + 1);
        if !self.valid || self.missing_weight || self.missing_weight_at == Some(column) {
            None
        } else {
            Some(weight(self.case, column, inner))
        }
    }
    fn write_output(&mut self, column: usize, value: f32) -> bool {
        assert_eq!(self.lane, 0);
        assert_eq!(column, self.output.len());
        assert!(column < N && value.is_finite());
        if !self.valid || self.fail_write || self.fail_write_at == Some(column) {
            return false;
        }
        self.output.push(value.to_bits());
        true
    }
}

#[test]
fn actual_dot_macro_matches_wave_order_and_independent_bound() {
    for (case, expected_bits) in [
        (Case::Ones, 0x4500_0000),
        (Case::Fraction, 0x4380_0000),
        (Case::Alternating, 0),
        (Case::Basis, 0xc0c0_0000),
        (Case::WaveOrder, 0x3f80_0000),
        (Case::Cancellation, 0),
        (Case::Subnormal, 0x0280_0000),
        (Case::Underflow, 0),
    ] {
        let columns = [staged(case, 4095)];
        let (reference, bound) = reference_bound(case, 4095);
        for lane in 0..64 {
            let subgroup = Subgroup {
                lane,
                columns: &columns,
                calls: Cell::new(0),
                allow_rejected: false,
                poison_reduction: false,
            };
            let (sum, finite) = qwen_wave_output_partial_f32_v1!(
                lane,
                inner,
                decode(input(case, inner)),
                decode(weight(case, 4095, inner)),
                subgroup
            );
            assert!(finite);
            assert_eq!(sum.to_bits(), expected_bits);
            assert!(within_bound(sum, reference, bound));
            assert_eq!(subgroup.calls.get(), 1);
        }
    }
}

#[test]
fn all_64_lanes_execute_all_4096_collectives_but_only_lane_zero_writes() {
    let columns = [staged(Case::Ones, 0)];
    for lane in 0..64 {
        let subgroup = Subgroup {
            lane,
            columns: &columns,
            calls: Cell::new(0),
            allow_rejected: false,
            poison_reduction: false,
        };
        let mut task = Task::new(Case::Ones, lane);
        qwen_claimed_output_projection_v5!(task, subgroup);
        assert!(task.valid);
        assert_eq!(task.inputs.get(), N * 32);
        assert_eq!(task.weights.get(), N * 32);
        assert_eq!(subgroup.calls.get(), N);
        assert_eq!(task.output.len(), if lane == 0 { N } else { 0 });
        assert!(task.output.iter().all(|&bits| bits == 0x4500_0000));
    }
}

#[test]
fn actual_full_handler_matches_rank_distinct_fp64_reference_without_narrowing() {
    for rank in 0..2 {
        let case = Case::Pattern(rank);
        let columns: Vec<_> = (0..N).map(|column| staged(case, column)).collect();
        for lane in [0, 63] {
            let subgroup = Subgroup {
                lane,
                columns: &columns,
                calls: Cell::new(0),
                allow_rejected: false,
                poison_reduction: false,
            };
            let mut task = Task::new(case, lane);
            qwen_claimed_output_projection_v5!(task, subgroup);
            assert!(task.valid);
            assert_eq!(subgroup.calls.get(), N);
            assert_eq!(task.inputs.get(), N * 32);
            assert_eq!(task.weights.get(), N * 32);
            if lane == 0 {
                assert_eq!(task.output.len(), N);
                assert!(task.output.iter().any(|bits| bits & 0xffff != 0));
                for column in 0..N {
                    let actual = f32::from_bits(task.output[column]);
                    assert_eq!(actual.to_bits(), columns[column].1[0].to_bits());
                    let (reference, bound) = reference_bound(case, column);
                    assert!(
                        within_bound(actual, reference, bound),
                        "rank={rank} column={column}"
                    );
                }
            } else {
                assert!(task.output.is_empty());
            }
        }
    }
}

#[test]
fn missing_load_or_failed_store_never_skips_later_collectives() {
    for failure in 0..4 {
        for lane in [0, 63] {
            let columns = [staged(Case::Ones, 0)];
            let subgroup = Subgroup {
                lane,
                columns: &columns,
                calls: Cell::new(0),
                allow_rejected: true,
                poison_reduction: failure == 3,
            };
            let mut task = Task::new(Case::Ones, lane);
            task.missing_input = failure == 0;
            task.missing_weight = failure == 1;
            task.fail_write = failure == 2;
            qwen_claimed_output_projection_v5!(task, subgroup);
            assert_eq!(task.valid, failure == 2 && lane != 0);
            assert_eq!(subgroup.calls.get(), N);
            assert_eq!(task.inputs.get(), N * 32);
            assert_eq!(task.weights.get(), N * 32);
            assert!(task.output.is_empty());
        }
    }
}

#[test]
fn nonfinite_and_overflow_are_rejected_without_abandoning_collectives() {
    for case in [
        Case::Nan,
        Case::Infinity,
        Case::ProductOverflow,
        Case::PartialOverflow,
        Case::ReductionOverflow,
    ] {
        let columns = [staged(case, 0)];
        for lane in [0, 63] {
            let subgroup = Subgroup {
                lane,
                columns: &columns,
                calls: Cell::new(0),
                allow_rejected: true,
                poison_reduction: false,
            };
            let mut task = Task::new(case, lane);
            qwen_claimed_output_projection_v5!(task, subgroup);
            assert!(!task.valid);
            assert!(task.output.is_empty());
            assert_eq!(subgroup.calls.get(), N);
            assert_eq!(task.inputs.get(), N * 32);
            assert_eq!(task.weights.get(), N * 32);
        }
    }
}

#[test]
fn predeclared_bound_handles_cancellation_underflow_and_rejects_bad_results() {
    let (reference, bound) = reference_bound(Case::Cancellation, 0);
    assert_eq!(reference, 1.0);
    assert!(within_bound(0.0, reference, bound));
    assert!(!within_bound(1000.0, reference, bound));
    let (reference, bound) = reference_bound(Case::Underflow, 0);
    assert_eq!(reference, 2.0_f64.powi(-139));
    assert!(within_bound(0.0, reference, bound));
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(!within_bound(bad, reference, bound));
    }
    let (reference, bound) = reference_bound(Case::Ones, 0);
    assert!(!within_bound(2049.0, reference, bound));
    assert!(!within_bound(2048.0, reference, -bound));
    assert!(!within_bound(2048.0, f64::NAN, bound));
}

#[test]
fn middle_and_final_column_rejection_preserve_prefix_and_all_collectives() {
    let columns = [staged(Case::Ones, 0)];
    for column in [2048, 4095] {
        for failed_store in [false, true] {
            for lane in [0, 63] {
                let subgroup = Subgroup {
                    lane,
                    columns: &columns,
                    calls: Cell::new(0),
                    allow_rejected: true,
                    poison_reduction: false,
                };
                let mut task = Task::new(Case::Ones, lane);
                if failed_store {
                    task.fail_write_at = Some(column);
                } else {
                    task.missing_weight_at = Some(column);
                }
                qwen_claimed_output_projection_v5!(task, subgroup);
                assert_eq!(task.valid, failed_store && lane != 0);
                assert_eq!(task.output.len(), if lane == 0 { column } else { 0 });
                assert!(task.output.iter().all(|&bits| bits == 0x4500_0000));
                assert_eq!(subgroup.calls.get(), N);
                assert_eq!(task.inputs.get(), N * 32);
                assert_eq!(task.weights.get(), N * 32);
            }
        }
    }
}

#[test]
fn specialized_dot_preserves_original_192_step_queued_partial_arithmetic() {
    for case in [
        Case::Pattern(0),
        Case::Pattern(1),
        Case::WaveOrder,
        Case::Cancellation,
    ] {
        for column in [0, 2048, 4095] {
            let columns = [staged(case, column)];
            for lane in 0..64 {
                let mut queued_partial = 0.0_f32;
                let mut queued_finite = true;
                for step in 0..192 {
                    let inner = step * 64 + lane;
                    if inner < 2048 {
                        let product =
                            decode(input(case, inner)) * decode(weight(case, column, inner));
                        queued_partial += product;
                        queued_finite &= product.is_finite() & queued_partial.is_finite();
                    }
                }
                assert_eq!(queued_partial.to_bits(), columns[0].0[lane].to_bits());
                let subgroup = Subgroup {
                    lane,
                    columns: &columns,
                    calls: Cell::new(0),
                    allow_rejected: false,
                    poison_reduction: false,
                };
                let (sum, finite) = qwen_wave_output_partial_f32_v1!(
                    lane,
                    inner,
                    decode(input(case, inner)),
                    decode(weight(case, column, inner)),
                    subgroup
                );
                assert_eq!(finite, queued_finite);
                assert_eq!(sum.to_bits(), columns[0].1[lane].to_bits());
            }
        }
    }
}
