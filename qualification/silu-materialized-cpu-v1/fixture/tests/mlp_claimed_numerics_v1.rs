use fe2o3_device::Bf16;
use std::cell::Cell;

#[macro_use]
#[path = "../src/wave_numerics_v1.rs"]
mod wave;
#[macro_use]
#[path = "../src/mlp_numerics_v1.rs"]
mod mlp;

fn decode(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}
fn narrow(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
}
fn same(actual: f32, expected: f32) {
    if expected.is_nan() {
        assert!(actual.is_nan());
    } else {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}
fn xor_tree(partial: [f32; 64]) -> [f32; 64] {
    let mut values = partial;
    for offset in [1, 2, 4, 8, 16, 32] {
        let previous = values;
        values = std::array::from_fn(|lane| previous[lane] + previous[lane ^ offset]);
    }
    values
}

#[derive(Clone, Copy)]
enum Case {
    Pattern,
    Cancellation,
    Basis,
    Tiny,
    Nan,
    ProductOverflow,
}
fn input(case: Case, inner: usize) -> u16 {
    match case {
        Case::Pattern => narrow(((inner * 13) % 31) as f32 / 128.0 - 0.125),
        Case::Tiny => narrow(2.0_f32.powi(-120)),
        Case::Nan if inner == 63 => 0x7fc0,
        Case::ProductOverflow => 0x7f7f,
        _ => narrow(1.0),
    }
}
fn weight(case: Case, row: usize, inner: usize) -> u16 {
    match case {
        Case::Pattern => narrow(((inner * 11 + (row % 8) * 7) % 43) as f32 / 128.0 - 0.125),
        Case::Cancellation => match inner {
            0 => narrow(16777216.0),
            64 => narrow(1.0),
            128 => narrow(-16777216.0),
            1 => narrow(0.5),
            _ => 0,
        },
        Case::Basis => {
            if inner == 6143 {
                narrow(-3.0)
            } else {
                0
            }
        }
        Case::Tiny => narrow(2.0_f32.powi(-30)),
        Case::ProductOverflow => narrow(2.0),
        _ => narrow(1.0),
    }
}
fn staged(case: Case, width: usize, row: usize) -> ([f32; 64], [f32; 64]) {
    let partial = std::array::from_fn(|lane| {
        let mut sum = 0.0_f32;
        for inner in (lane..width).step_by(64) {
            sum += decode(input(case, inner)) * decode(weight(case, row, inner));
        }
        sum
    });
    (partial, xor_tree(partial))
}

// An API-shaped stand-in checks the real macro's local arithmetic and returns
// an independent simulated full-wave reduction. It is not GPU collective,
// divergence, compiler-lowering or scheduling evidence.
struct DotWave {
    lane: usize,
    rows: usize,
    expected: Vec<([f32; 64], [f32; 64])>,
    sums: Cell<usize>,
    broadcasts: Cell<usize>,
    allow_rejected: bool,
}
impl DotWave {
    fn new(case: Case, width: usize, rows: usize, lane: usize, allow_rejected: bool) -> Self {
        Self {
            lane,
            rows,
            expected: (0..8).map(|row| staged(case, width, row)).collect(),
            sums: Cell::new(0),
            broadcasts: Cell::new(0),
            allow_rejected,
        }
    }
    fn reduce_sum_f32<const WIDTH: usize>(&self, value: f32) -> f32 {
        assert_eq!(WIDTH, 64);
        let row = self.sums.get();
        assert!(row < self.rows);
        self.sums.set(row + 1);
        if !(self.allow_rejected && value.is_nan()) {
            same(value, self.expected[row % 8].0[self.lane]);
        }
        self.expected[row % 8].1[self.lane]
    }
    fn broadcast_f32<const WIDTH: usize>(&self, value: f32, source: usize) -> f32 {
        assert_eq!((WIDTH, source), (64, 0));
        let row = self.broadcasts.get();
        assert_eq!(self.sums.get(), row + 1);
        same(value, self.expected[row % 8].1[self.lane]);
        self.broadcasts.set(row + 1);
        self.expected[row % 8].1[0]
    }
}
struct DotTask {
    lane: usize,
    width: usize,
    rows: usize,
    case: Case,
    valid: bool,
    inputs: Cell<usize>,
    weights: Cell<usize>,
    output: Vec<u32>,
    missing_row: Option<usize>,
    fail_write_row: Option<usize>,
}
impl DotTask {
    fn new(case: Case, width: usize, rows: usize, lane: usize) -> Self {
        Self {
            lane,
            width,
            rows,
            case,
            valid: true,
            inputs: Cell::new(0),
            weights: Cell::new(0),
            output: Vec::new(),
            missing_row: None,
            fail_write_row: None,
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
        assert_eq!(inner, self.lane + (call % (self.width / 64)) * 64);
        assert!(inner < self.width && call < self.rows * (self.width / 64));
        self.inputs.set(call + 1);
        if self.valid {
            Some(input(self.case, inner))
        } else {
            None
        }
    }
    fn weight(&self, row: usize, inner: usize) -> Option<u16> {
        let call = self.weights.get();
        assert_eq!(row, call / (self.width / 64));
        assert_eq!(inner, self.lane + (call % (self.width / 64)) * 64);
        assert!(row < self.rows && inner < self.width);
        self.weights.set(call + 1);
        if self.valid && self.missing_row != Some(row) {
            Some(weight(self.case, row, inner))
        } else {
            None
        }
    }
    fn write_column(&mut self, row: usize, value: u16) -> bool {
        self.store(row, u32::from(value))
    }
    fn write_output(&mut self, row: usize, value: f32) -> bool {
        assert!(value.is_finite());
        self.store(row, value.to_bits())
    }
    fn store(&mut self, row: usize, value: u32) -> bool {
        assert_eq!(self.lane, 0);
        assert_eq!(row, self.output.len());
        if !self.valid || self.fail_write_row == Some(row) {
            return false;
        }
        self.output.push(value);
        true
    }
}
fn project(task: &mut DotTask, subgroup: &DotWave) {
    qwen_claimed_mlp_projection_v1!(task, subgroup);
}
fn down(task: &mut DotTask, subgroup: &DotWave) {
    qwen_claimed_mlp_down_v1!(task, subgroup);
}

#[test]
fn both_projection_handlers_preserve_all_rows_and_only_lane_zero_writes() {
    // Gate and Up share the exact numerical body; actual distinct root binding
    // and all64-lane write coverage are tested in the provider unit tests.
    for _projection in ["gate", "up"] {
        for lane in [0, 1, 63] {
            let mut task = DotTask::new(Case::Pattern, 4096, 6144, lane);
            let subgroup = DotWave::new(Case::Pattern, 4096, 6144, lane, false);
            project(&mut task, &subgroup);
            assert!(task.valid);
            assert_eq!(
                (subgroup.sums.get(), subgroup.broadcasts.get()),
                (6144, 6144)
            );
            assert_eq!(
                (task.inputs.get(), task.weights.get()),
                (6144 * 64, 6144 * 64)
            );
            assert_eq!(task.output.len(), if lane == 0 { 6144 } else { 0 });
            for (row, bits) in task.output.iter().enumerate() {
                assert_eq!(*bits, u32::from(narrow(subgroup.expected[row % 8].1[0])));
            }
        }
    }
}

#[test]
fn rejected_projection_and_down_still_execute_every_collective() {
    for down_mode in [false, true] {
        let (width, rows) = if down_mode {
            (6144, 4096)
        } else {
            (4096, 6144)
        };
        for failure in 0..4 {
            let lane = if failure == 3 { 63 } else { 0 };
            let case = if failure == 3 {
                Case::Nan
            } else {
                Case::Pattern
            };
            let mut task = DotTask::new(case, width, rows, lane);
            match failure {
                0 => task.missing_row = Some(0),
                1 => task.missing_row = Some(7),
                2 => task.fail_write_row = Some(3),
                _ => (),
            }
            let subgroup = DotWave::new(case, width, rows, lane, true);
            if down_mode {
                down(&mut task, &subgroup);
            } else {
                project(&mut task, &subgroup);
            }
            assert!(!task.valid);
            assert_eq!(subgroup.sums.get(), rows);
            assert_eq!(subgroup.broadcasts.get(), if down_mode { 0 } else { rows });
            assert_eq!(task.inputs.get(), rows * (width / 64));
            assert_eq!(task.weights.get(), rows * (width / 64));
            assert_eq!(
                task.output.len(),
                match failure {
                    1 => 7,
                    2 => 3,
                    _ => 0,
                }
            );
        }
    }
}

fn gamma_up(count: f64, unit: f64) -> f64 {
    (count * unit / (1.0 - count * unit)).next_up()
}
fn down_reference(case: Case, row: usize) -> (f64, f64) {
    let mut reference = 0.0_f64;
    let mut absolute = 0.0_f64;
    for inner in 0..6144 {
        let product =
            f64::from(decode(input(case, inner))) * f64::from(decode(weight(case, row, inner)));
        reference += product;
        absolute += product.abs();
    }
    // One product,96 lane adds,six XOR levels. Independent scalar reference
    // sums6144 exact BF16 products. Gradual IEEE F32 is an artifact prerequisite.
    let g32 = gamma_up(103.0, 2.0_f64.powi(-24));
    let g64 = gamma_up(6144.0, 2.0_f64.powi(-53));
    let upper = (absolute / (1.0 - g64).next_down()).next_up();
    let relative = ((g32 + g64).next_up() * upper).next_up();
    let underflow =
        (12351.0 * 2.0_f64.powi(-150) / (1.0 - 103.0 * 2.0_f64.powi(-24)).next_down()).next_up();
    (reference, (relative + underflow).next_up())
}

#[test]
fn down_keeps_fp32_low_bits_and_meets_independent_fp64_bound() {
    for case in [Case::Pattern, Case::Cancellation, Case::Basis, Case::Tiny] {
        let mut task = DotTask::new(case, 6144, 4096, 0);
        let subgroup = DotWave::new(case, 6144, 4096, 0, false);
        down(&mut task, &subgroup);
        assert!(task.valid);
        assert_eq!(task.output.len(), 4096);
        assert_eq!((subgroup.sums.get(), subgroup.broadcasts.get()), (4096, 0));
        for (row, bits) in task.output.iter().enumerate() {
            same(f32::from_bits(*bits), subgroup.expected[row % 8].1[0]);
        }
        for row in 0..8 {
            let actual = f64::from(f32::from_bits(task.output[row]));
            let (reference, bound) = down_reference(case, row);
            let error = (actual - reference).abs();
            assert!((if error == 0.0 { 0.0 } else { error.next_up() }) <= bound);
        }
        if matches!(case, Case::Pattern) {
            assert!(task.output.iter().any(|bits| bits & 0xffff != 0));
        }
    }
    for lane in [1, 63] {
        let mut task = DotTask::new(Case::Pattern, 6144, 4096, lane);
        let subgroup = DotWave::new(Case::Pattern, 6144, 4096, lane, false);
        down(&mut task, &subgroup);
        assert!(task.valid && task.output.is_empty());
        assert_eq!(subgroup.sums.get(), 4096);
    }
}

struct IdentityWave {
    expected: f32,
    calls: Cell<usize>,
}
impl IdentityWave {
    fn reduce_sum_f32<const WIDTH: usize>(&self, partial: f32) -> f32 {
        assert_eq!(WIDTH, 64);
        same(partial, self.expected);
        self.calls.set(self.calls.get() + 1);
        partial
    }
}
#[test]
fn specialized_down_matches_every_active_queued_iteration_and_overflow_status() {
    for case in [
        Case::Pattern,
        Case::Cancellation,
        Case::Tiny,
        Case::Nan,
        Case::ProductOverflow,
    ] {
        for lane in 0..64 {
            let mut expected = 0.0_f32;
            let mut expected_finite = true;
            let mut active = Vec::new();
            for step in 0..192 {
                let inner = step * 64 + lane;
                if inner < 6144 {
                    active.push(inner);
                    let product = decode(input(case, inner)) * decode(weight(case, 3, inner));
                    expected += product;
                    expected_finite &= product.is_finite() & expected.is_finite();
                }
            }
            let subgroup = IdentityWave {
                expected,
                calls: Cell::new(0),
            };
            let mut observed = Vec::new();
            let (actual, finite) = qwen_wave_mlp_down_partial_v1!(
                lane,
                inner,
                {
                    observed.push(inner);
                    decode(input(case, inner))
                },
                decode(weight(case, 3, inner)),
                subgroup
            );
            same(actual, expected);
            assert_eq!(finite, expected_finite);
            assert_eq!(observed, active);
            assert_eq!(subgroup.calls.get(), 1);
        }
    }
}

fn norm_input(column: usize) -> u16 {
    narrow(((column % 13) as f32 - 6.0) / 8.0)
}
fn norm_weight(column: usize) -> u16 {
    narrow(((column % 7) as f32 + 1.0) / 4.0)
}
struct NormTask {
    lane: usize,
    valid: bool,
    output: Vec<u16>,
    missing_weight: bool,
    fail_write: bool,
}
impl NormTask {
    fn lane(&self) -> usize {
        self.lane
    }
    fn reject(&mut self) {
        self.valid = false;
    }
    fn input(&self, column: usize) -> Option<u16> {
        assert!(column < 4096);
        if self.valid {
            Some(norm_input(column))
        } else {
            None
        }
    }
    fn weight(&self, column: usize) -> Option<u16> {
        assert_eq!(column % 64, self.lane);
        if self.valid && !self.missing_weight {
            Some(norm_weight(column))
        } else {
            None
        }
    }
    fn write_component(&mut self, component: usize, value: u16) -> bool {
        assert_eq!(component, self.output.len());
        if !self.valid || self.fail_write {
            return false;
        }
        self.output.push(value);
        true
    }
}
struct NormWave {
    lane: usize,
    partial: [f32; 64],
    sums: [f32; 64],
    invalid: bool,
    phase: Cell<usize>,
}
impl NormWave {
    fn new(lane: usize, invalid: bool) -> Self {
        let partial = std::array::from_fn(|lane| {
            (lane..4096).step_by(64).fold(0.0_f32, |sum, column| {
                let value = decode(norm_input(column));
                sum + value * value
            })
        });
        Self {
            lane,
            partial,
            sums: xor_tree(partial),
            invalid,
            phase: Cell::new(0),
        }
    }
    fn reduce_sum_f32<const WIDTH: usize>(&self, value: f32) -> f32 {
        assert_eq!((WIDTH, self.phase.get()), (64, 0));
        same(value, self.partial[self.lane]);
        self.phase.set(1);
        self.sums[self.lane]
    }
    fn broadcast_f32<const WIDTH: usize>(&self, value: f32, source: usize) -> f32 {
        assert_eq!((WIDTH, source), (64, 0));
        match self.phase.get() {
            1 => {
                same(value, self.sums[self.lane]);
                self.phase.set(2);
                self.sums[0]
            }
            3 => {
                same(value, if self.invalid { 1.0 } else { 0.0 });
                self.phase.set(4);
                value
            }
            _ => panic!("unexpected norm broadcast"),
        }
    }
    fn reduce_max_f32<const WIDTH: usize>(&self, value: f32) -> f32 {
        assert_eq!((WIDTH, self.phase.get()), (64, 2));
        same(value, 0.0);
        self.phase.set(3);
        if self.invalid { 1.0 } else { 0.0 }
    }
}
fn norm(task: &mut NormTask, subgroup: &NormWave) {
    qwen_claimed_mlp_norm_v1!(task, subgroup, stabilized, stabilized.sqrt());
}
#[test]
fn norm_preserves_two_bf16_rounds_and_all_lane_coverage() {
    let mut dual_differs_from_single = false;
    for lane in 0..64 {
        let mut task = NormTask {
            lane,
            valid: true,
            output: Vec::new(),
            missing_weight: false,
            fail_write: false,
        };
        let subgroup = NormWave::new(lane, false);
        norm(&mut task, &subgroup);
        assert!(task.valid);
        assert_eq!((task.output.len(), subgroup.phase.get()), (64, 4));
        let inverse = 1.0_f32 / (subgroup.sums[0] / 4096.0 + 1e-6).sqrt();
        for (component, actual) in task.output.iter().enumerate() {
            let column = lane + 64 * component;
            let value = decode(norm_input(column)) * inverse;
            let expected = narrow(decode(narrow(value)) * decode(norm_weight(column)));
            assert_eq!(*actual, expected);
            dual_differs_from_single |= expected != narrow(value * decode(norm_weight(column)));
        }
    }
    assert!(dual_differs_from_single);
}
#[test]
fn norm_rejects_collective_invalid_math_and_local_write_failure() {
    for case in 0..4 {
        let mut task = NormTask {
            lane: 0,
            valid: true,
            output: Vec::new(),
            missing_weight: case == 2,
            fail_write: case == 3,
        };
        let subgroup = NormWave::new(0, case == 0);
        if case == 1 {
            qwen_claimed_mlp_norm_v1!(task, subgroup, stabilized, {
                let _ = stabilized;
                f32::NAN
            });
        } else {
            norm(&mut task, &subgroup);
        }
        assert!(!task.valid && task.output.is_empty());
        assert_eq!(subgroup.phase.get(), 4);
    }
}

fn activation_gate(column: usize) -> u16 {
    match column % 8 {
        0 => 0,
        1 => 0x8000,
        2 => narrow(-100.0),
        3 => narrow(100.0),
        4 => narrow(-3.0),
        5 => narrow(3.0),
        6 => narrow(-0.5),
        _ => narrow(0.5),
    }
}
fn activation_up(column: usize) -> u16 {
    narrow(if column % 3 == 0 { -0.75 } else { 1.25 })
}
struct ActivationTask {
    lane: usize,
    valid: bool,
    output: Vec<u16>,
    bad: bool,
    fail_write: bool,
    gate_calls: Cell<usize>,
    up_calls: Cell<usize>,
}
impl ActivationTask {
    fn lane(&self) -> usize {
        self.lane
    }
    fn reject(&mut self) {
        self.valid = false;
    }
    fn gate(&self, column: usize) -> Option<u16> {
        assert_eq!(column, self.lane + self.gate_calls.get() * 64);
        self.gate_calls.set(self.gate_calls.get() + 1);
        if self.valid {
            Some(if self.bad {
                0x7fc0
            } else {
                activation_gate(column)
            })
        } else {
            None
        }
    }
    fn up(&self, column: usize) -> Option<u16> {
        assert_eq!(column, self.lane + self.up_calls.get() * 64);
        self.up_calls.set(self.up_calls.get() + 1);
        if self.valid {
            Some(activation_up(column))
        } else {
            None
        }
    }
    fn write_component(&mut self, component: usize, value: u16) -> bool {
        assert_eq!(component, self.output.len());
        if !self.valid || self.fail_write {
            return false;
        }
        self.output.push(value);
        true
    }
}
struct HostMath {
    calls: Cell<usize>,
    poison: bool,
}
impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        self.calls.set(self.calls.get() + 1);
        if self.poison { f32::NAN } else { value.exp() }
    }
}
fn ordered(bits: u16) -> i32 {
    if bits & 0x8000 != 0 {
        i32::from(!bits)
    } else {
        i32::from(bits | 0x8000)
    }
}
#[test]
fn swiglu_full_coverage_stable_expression_and_zero_signs() {
    for lane in 0..64 {
        let mut task = ActivationTask {
            lane,
            valid: true,
            output: Vec::new(),
            bad: false,
            fail_write: false,
            gate_calls: Cell::new(0),
            up_calls: Cell::new(0),
        };
        let math = HostMath {
            calls: Cell::new(0),
            poison: false,
        };
        qwen_claimed_mlp_swiglu_v1!(task, math);
        assert!(task.valid);
        assert_eq!((task.output.len(), math.calls.get()), (96, 96));
        for (component, actual) in task.output.iter().enumerate() {
            let column = lane + component * 64;
            let g = f64::from(decode(activation_gate(column)));
            let u = f64::from(decode(activation_up(column)));
            let e = (-g.abs()).exp();
            let expected = narrow(((g * (if g >= 0.0 { 1.0 } else { e } / (1.0 + e))) * u) as f32);
            if g == 0.0 || u == 0.0 {
                assert_eq!(*actual, expected);
            } else {
                assert!((ordered(*actual) - ordered(expected)).abs() <= 1);
            }
        }
    }
}
#[test]
fn swiglu_invalid_inputs_exp_and_writes_finish_all_components() {
    for failure in 0..3 {
        let mut task = ActivationTask {
            lane: 0,
            valid: true,
            output: Vec::new(),
            bad: failure == 0,
            fail_write: failure == 2,
            gate_calls: Cell::new(0),
            up_calls: Cell::new(0),
        };
        let math = HostMath {
            calls: Cell::new(0),
            poison: failure == 1,
        };
        qwen_claimed_mlp_swiglu_v1!(task, math);
        assert!(!task.valid && task.output.is_empty());
        assert_eq!(
            (task.gate_calls.get(), task.up_calls.get(), math.calls.get()),
            (96, 96, 96)
        );
    }
}
