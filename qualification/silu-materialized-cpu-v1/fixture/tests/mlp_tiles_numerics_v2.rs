use fe2o3_device::Bf16;
use std::cell::Cell;

#[macro_use]
#[path = "../src/wave_numerics_v1.rs"]
mod wave;
#[macro_use]
#[path = "../src/mlp_numerics_v1.rs"]
mod mlp;
#[macro_use]
#[path = "../src/mlp_tile_numerics_v2.rs"]
mod tiled;

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
    NegativeZero,
}
fn input(case: Case, inner: usize) -> u16 {
    match case {
        Case::Pattern => narrow(((inner * 13) % 31) as f32 / 128.0 - 0.125),
        Case::Tiny => narrow(2.0_f32.powi(-120)),
        Case::Nan if inner == 63 => 0x7fc0,
        Case::ProductOverflow => 0x7f7f,
        Case::NegativeZero => 0x8000,
        _ => narrow(1.0),
    }
}
fn weight(case: Case, row: usize, inner: usize) -> u16 {
    match case {
        Case::Pattern => narrow(((inner * 11 + row * 7) % 43) as f32 / 128.0 - 0.125),
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
    fn new(
        case: Case,
        width: usize,
        rows: usize,
        row_base: usize,
        lane: usize,
        allow_rejected: bool,
    ) -> Self {
        Self {
            lane,
            rows,
            expected: (0..rows)
                .map(|row| staged(case, width, row_base + row))
                .collect(),
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
            same(value, self.expected[row].0[self.lane]);
        }
        self.expected[row].1[self.lane]
    }
    fn broadcast_f32<const WIDTH: usize>(&self, value: f32, source: usize) -> f32 {
        assert_eq!((WIDTH, source), (64, 0));
        let row = self.broadcasts.get();
        assert_eq!(self.sums.get(), row + 1);
        same(value, self.expected[row].1[self.lane]);
        self.broadcasts.set(row + 1);
        self.expected[row].1[0]
    }
}
struct DotTask {
    lane: usize,
    width: usize,
    rows: usize,
    row_base: usize,
    case: Case,
    valid: bool,
    inputs: Cell<usize>,
    weights: Cell<usize>,
    output: Vec<u32>,
    missing_row: Option<usize>,
    fail_write_row: Option<usize>,
}
impl DotTask {
    fn new(case: Case, width: usize, rows: usize, row_base: usize, lane: usize) -> Self {
        Self {
            lane,
            width,
            rows,
            row_base,
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
            Some(weight(self.case, self.row_base + row, inner))
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
    qwen_claimed_mlp_projection_tile_v2!(task, subgroup);
}
fn down(task: &mut DotTask, subgroup: &DotWave) {
    qwen_claimed_mlp_down_tile_v2!(task, subgroup);
}

#[test]
fn every_tile_retains_lane_product_order_and_exact_narrowing_boundaries() {
    for width in [4096, 6144] {
        let tiles = if width == 4096 { 96 } else { 64 };
        for tile in 0..tiles {
            // All tile endpoints with a row-sensitive original NxK source.
            let lane = tile % 64;
            let mut task = DotTask::new(Case::Pattern, width, 64, tile * 64, lane);
            let subgroup = DotWave::new(Case::Pattern, width, 64, tile * 64, lane, false);
            if width == 4096 {
                project(&mut task, &subgroup);
            } else {
                down(&mut task, &subgroup);
            }
            assert!(task.valid);
            assert_eq!(subgroup.sums.get(), 64);
            assert_eq!(
                subgroup.broadcasts.get(),
                if width == 4096 { 64 } else { 0 }
            );
            assert_eq!(task.inputs.get(), width);
            assert_eq!(task.weights.get(), width);
            assert_eq!(task.output.len(), if lane == 0 { 64 } else { 0 });
            for (row, actual) in task.output.iter().enumerate() {
                let sum = subgroup.expected[row].1[0];
                let expected = if width == 4096 {
                    u32::from(narrow(sum))
                } else {
                    sum.to_bits()
                };
                assert_eq!(*actual, expected);
            }
        }
    }
}

#[test]
fn tile_arithmetic_matches_v1_inner_macros_for_adversarial_inputs() {
    for case in [
        Case::Cancellation,
        Case::Basis,
        Case::Tiny,
        Case::NegativeZero,
    ] {
        for width in [4096, 6144] {
            for lane in [0, 1, 63] {
                let mut task = DotTask::new(case, width, 64, 4032, lane);
                let subgroup = DotWave::new(case, width, 64, 4032, lane, false);
                if width == 4096 {
                    project(&mut task, &subgroup);
                } else {
                    down(&mut task, &subgroup);
                }
                assert!(task.valid);
                assert_eq!(subgroup.sums.get(), 64);
                for (row, actual) in task.output.iter().enumerate() {
                    let sum = staged(case, width, 4032 + row).1[0];
                    assert_eq!(
                        *actual,
                        if width == 4096 {
                            u32::from(narrow(sum))
                        } else {
                            sum.to_bits()
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn rejected_tiles_still_execute_every_remaining_row_collective() {
    for width in [4096, 6144] {
        for lane in [0, 63] {
            for failure in 0..4 {
                let case = match failure {
                    0 => Case::Nan,
                    1 => Case::ProductOverflow,
                    _ => Case::Pattern,
                };
                let mut task = DotTask::new(case, width, 64, 0, lane);
                if failure == 2 {
                    task.missing_row = Some(1);
                }
                if failure == 3 {
                    task.fail_write_row = Some(1);
                }
                let subgroup = DotWave::new(case, width, 64, 0, lane, true);
                if width == 4096 {
                    project(&mut task, &subgroup);
                } else {
                    down(&mut task, &subgroup);
                }
                if failure != 3 || lane == 0 {
                    assert!(!task.valid);
                }
                assert_eq!(subgroup.sums.get(), 64);
                assert_eq!(
                    subgroup.broadcasts.get(),
                    if width == 4096 { 64 } else { 0 }
                );
                assert_eq!(task.inputs.get(), width);
                assert_eq!(task.weights.get(), width);
            }
        }
    }
}

#[test]
fn source_changes_only_outer_row_bounds_and_reuses_old_norm_swiglu() {
    let old = include_str!("../src/mlp_numerics_v1.rs");
    let tiled = include_str!("../src/mlp_tile_numerics_v2.rs");
    for (name, new_name, bound) in [
        (
            "qwen_claimed_mlp_projection_v1",
            "qwen_claimed_mlp_projection_tile_v2",
            "6144",
        ),
        (
            "qwen_claimed_mlp_down_v1",
            "qwen_claimed_mlp_down_tile_v2",
            "4096",
        ),
    ] {
        fn macro_body<'a>(s: &'a str, name: &str) -> &'a str {
            let start = s.find(&format!("macro_rules! {name}")).unwrap();
            let tail = &s[start..];
            &tail[..tail.find("\n#[allow(unused_macros)]").unwrap_or(tail.len())]
        }
        let expected = macro_body(old, name)
            .replace(name, new_name)
            .replace(&format!("while row < {bound}"), "while row < 64");
        let tokens = |s: &str| {
            syn::parse_str::<syn::ItemMacro>(s)
                .map(|m| quote::quote!(#m).to_string())
                .unwrap()
        };
        assert_eq!(tokens(macro_body(tiled, new_name)), tokens(&expected));
    }
    let source = include_str!("../src/finite_mlp_tiles_v2.rs");
    let syntax = syn::parse_file(source).unwrap();
    let text = quote::quote!(#syntax).to_string();
    assert!(text.contains("qwen_claimed_mlp_norm_v1"));
    assert!(text.contains("qwen_claimed_mlp_swiglu_v1"));
    assert!(text.contains("WaveMlpTileStorageV2"));
    assert!(text.contains("max_grid = [64 , 1 , 1]"));
    assert!(!text.contains("WaveMlpTaskStorageV1"));
}
