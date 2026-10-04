use std::cell::Cell;

#[macro_use]
#[path = "../src/mlp_numerics_v1.rs"]
mod original;
#[macro_use]
#[path = "../src/mlp_tile_numerics_v2.rs"]
mod tiled;

const LANES: usize = 64;
const ROWS: usize = 64;
const STEPS: usize = 96;
const WIDTH: usize = LANES * STEPS;

#[derive(Clone, Copy, Debug)]
enum Case {
    Pattern,
    Cancellation,
    NegativeZero,
    Tiny,
    TinyTie,
    Nan,
    Infinity,
    ProductOverflow,
    PartialOverflow,
    ReductionOverflow,
}

#[derive(Clone, Copy, Debug)]
struct Scenario {
    case: Case,
    missing_input: Option<usize>,
    missing_weight: Option<(usize, usize)>,
    failed_write: Option<usize>,
}

impl Scenario {
    fn new(case: Case) -> Self {
        Self {
            case,
            missing_input: None,
            missing_weight: None,
            failed_write: None,
        }
    }
}

fn decode(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn encode(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
}

fn input(case: Case, inner: usize) -> u16 {
    match case {
        Case::Pattern => encode(((inner * 13) % 31) as f32 / 128.0 - 0.125),
        Case::NegativeZero => 0x8000,
        Case::Tiny | Case::TinyTie => 0x0001,
        Case::Nan if inner == 63 => 0x7fc0,
        Case::Infinity if inner == 63 => 0x7f80,
        Case::ProductOverflow | Case::PartialOverflow | Case::ReductionOverflow => 0x7f7f,
        _ => 0x3f80,
    }
}

fn weight(case: Case, row: usize, inner: usize) -> u16 {
    match case {
        Case::Pattern => encode(((inner * 11 + row * 7) % 43) as f32 / 128.0 - 0.125),
        Case::Cancellation => match inner {
            0 => encode(16777216.0),
            64 => encode(1.0),
            128 => encode(-16777216.0),
            1 => encode(0.5),
            _ => 0,
        },
        Case::Tiny => 0x3780, // 2^-16 gives an exact minimum FP32 subnormal product.
        Case::TinyTie => 0x3700, // 2^-17 gives a halfway product that rounds to zero.
        Case::ProductOverflow => 0x4000,
        Case::ReductionOverflow if inner >= LANES => 0,
        _ => 0x3f80,
    }
}

fn same(actual: f32, expected: f32) {
    if expected.is_nan() {
        assert!(
            actual.is_nan(),
            "expected a rejected NaN partial, got {actual:?}"
        );
    } else {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

fn xor_tree(mut values: [f32; LANES]) -> [f32; LANES] {
    for offset in [1, 2, 4, 8, 16, 32] {
        let before = values;
        values = std::array::from_fn(|lane| before[lane] + before[lane ^ offset]);
    }
    values
}

// This reference evaluates complete rows before the next row, independently
// of both macros. A failed lane sees the original provider's None input on
// every subsequent row. The stand-in below checks each actual macro partial
// against this full-wave model; it does not emulate GPU execution or barriers.
struct Reference {
    partials: Vec<[f32; LANES]>,
    sums: Vec<[f32; LANES]>,
    valid: [bool; LANES],
    output: Vec<u32>,
}

impl Reference {
    fn new(scenario: Scenario, row_base: usize) -> Self {
        let mut result = Self {
            partials: Vec::new(),
            sums: Vec::new(),
            valid: [true; LANES],
            output: Vec::new(),
        };
        for row in 0..ROWS {
            let mut partial = [0.0_f32; LANES];
            let mut finite = [true; LANES];
            for lane in 0..LANES {
                for inner in (lane..WIDTH).step_by(LANES) {
                    let left = if result.valid[lane] && scenario.missing_input != Some(inner) {
                        input(scenario.case, inner)
                    } else {
                        0x7fc0
                    };
                    let right = if scenario.missing_weight == Some((row, inner)) {
                        0x7fc0
                    } else {
                        weight(scenario.case, row_base + row, inner)
                    };
                    let product = decode(left) * decode(right);
                    partial[lane] += product;
                    finite[lane] &= product.is_finite() & partial[lane].is_finite();
                }
            }
            let sums = xor_tree(partial);
            for lane in 0..LANES {
                if !finite[lane] || !sums[lane].is_finite() {
                    result.valid[lane] = false;
                } else if lane == 0 {
                    if !result.valid[lane] || scenario.failed_write == Some(row) {
                        result.valid[lane] = false;
                    } else {
                        assert_eq!(result.output.len(), row);
                        result.output.push(sums[lane].to_bits());
                    }
                }
            }
            result.partials.push(partial);
            result.sums.push(sums);
        }
        result
    }
}

struct Wave<'a> {
    lane: usize,
    reference: &'a Reference,
    reductions: Cell<usize>,
}

impl Wave<'_> {
    fn reduce_sum_f32<const N: usize>(&self, partial: f32) -> f32 {
        assert_eq!(N, LANES);
        let row = self.reductions.get();
        assert!(row < ROWS);
        same(partial, self.reference.partials[row][self.lane]);
        self.reductions.set(row + 1);
        self.reference.sums[row][self.lane]
    }
}

#[derive(Clone, Copy)]
enum Order {
    Original,
    Paired,
}

struct Task {
    lane: usize,
    row_base: usize,
    scenario: Scenario,
    order: Order,
    valid: bool,
    inputs: Cell<usize>,
    weights: Cell<usize>,
    output: Vec<u32>,
    writes: Vec<(usize, u32)>,
    rejects: usize,
}

impl Task {
    fn new(lane: usize, row_base: usize, scenario: Scenario, order: Order) -> Self {
        assert!(lane < LANES && row_base % ROWS == 0 && row_base + ROWS <= 4096);
        Self {
            lane,
            row_base,
            scenario,
            order,
            valid: true,
            inputs: Cell::new(0),
            weights: Cell::new(0),
            output: Vec::new(),
            writes: Vec::new(),
            rejects: 0,
        }
    }

    fn lane(&self) -> usize {
        self.lane
    }

    fn reject(&mut self) {
        self.valid = false;
        self.rejects += 1;
    }

    fn input(&self, inner: usize) -> Option<u16> {
        let call = self.inputs.get();
        let count = match self.order {
            Order::Original => ROWS * STEPS,
            Order::Paired => ROWS / 2 * STEPS,
        };
        assert!(call < count);
        assert_eq!(inner, self.lane + (call % STEPS) * LANES);
        self.inputs.set(call + 1);
        (self.valid && self.scenario.missing_input != Some(inner))
            .then(|| input(self.scenario.case, inner))
    }

    fn weight(&self, row: usize, inner: usize) -> Option<u16> {
        let call = self.weights.get();
        assert!(call < ROWS * STEPS);
        let (expected_row, step) = match self.order {
            Order::Original => (call / STEPS, call % STEPS),
            Order::Paired => (2 * (call / (2 * STEPS)) + call % 2, (call / 2) % STEPS),
        };
        assert_eq!((row, inner), (expected_row, self.lane + step * LANES));
        assert!(row < ROWS && inner < WIDTH);
        self.weights.set(call + 1);
        // The actual provider's immutable weight read does not inspect valid.
        (self.scenario.missing_weight != Some((row, inner)))
            .then(|| weight(self.scenario.case, self.row_base + row, inner))
    }

    fn write_output(&mut self, row: usize, value: f32) -> bool {
        assert!(value.is_finite());
        assert_eq!((self.lane, row), (0, self.output.len()));
        self.writes.push((row, value.to_bits()));
        if !self.valid || self.scenario.failed_write == Some(row) {
            self.valid = false;
            return false;
        }
        self.output.push(value.to_bits());
        true
    }
}

fn baseline(task: &mut Task, subgroup: &Wave<'_>) {
    qwen_claimed_mlp_down_tile_v2!(task, subgroup);
}

fn candidate(task: &mut Task, subgroup: &Wave<'_>) {
    qwen_claimed_mlp_down_tile2_v1!(task, subgroup);
}

fn compare_lane(scenario: Scenario, row_base: usize, lane: usize, reference: &Reference) -> Task {
    let mut old = Task::new(lane, row_base, scenario, Order::Original);
    let old_wave = Wave {
        lane,
        reference,
        reductions: Cell::new(0),
    };
    baseline(&mut old, &old_wave);
    let mut new = Task::new(lane, row_base, scenario, Order::Paired);
    let new_wave = Wave {
        lane,
        reference,
        reductions: Cell::new(0),
    };
    candidate(&mut new, &new_wave);
    assert_eq!(old_wave.reductions.get(), ROWS);
    assert_eq!(new_wave.reductions.get(), ROWS);
    assert_eq!((old.inputs.get(), new.inputs.get()), (6144, 3072));
    assert_eq!((old.weights.get(), new.weights.get()), (6144, 6144));
    assert_eq!(old.output, new.output);
    assert_eq!(old.writes, new.writes);
    assert_eq!(old.rejects, new.rejects);
    assert_eq!(old.valid, new.valid);
    assert_eq!(new.valid, reference.valid[lane]);
    if lane == 0 {
        assert_eq!(new.output, reference.output);
    } else {
        assert!(new.output.is_empty() && new.writes.is_empty());
    }
    new
}

#[test]
fn all_lanes_match_original_partial_bits_reductions_and_ordered_writes() {
    let scenario = Scenario::new(Case::Pattern);
    for row_base in [0, 4032] {
        let reference = Reference::new(scenario, row_base);
        for lane in 0..LANES {
            let task = compare_lane(scenario, row_base, lane, &reference);
            assert!(task.valid);
            assert_eq!(task.output.len(), if lane == 0 { ROWS } else { 0 });
        }
    }
}

#[test]
fn every_down_tile_preserves_first_last_rows_and_original_weight_layout() {
    let scenario = Scenario::new(Case::Pattern);
    for tile in 0..64 {
        let row_base = tile * ROWS;
        let reference = Reference::new(scenario, row_base);
        for lane in [0, 1, 63] {
            assert!(compare_lane(scenario, row_base, lane, &reference).valid);
        }
    }
}

#[test]
fn cancellation_signed_zero_and_subnormal_rounding_are_bitwise_unchanged() {
    for case in [
        Case::Cancellation,
        Case::NegativeZero,
        Case::Tiny,
        Case::TinyTie,
    ] {
        let scenario = Scenario::new(case);
        let reference = Reference::new(scenario, 0);
        for lane in 0..LANES {
            assert!(compare_lane(scenario, 0, lane, &reference).valid);
        }
        if matches!(case, Case::Tiny) {
            assert_eq!(reference.output, vec![6144; ROWS]);
        }
        if matches!(case, Case::NegativeZero | Case::TinyTie) {
            assert_eq!(reference.output, vec![0; ROWS]);
        }
    }
}

#[test]
fn nonfinite_product_partial_and_reduction_overflow_keep_all_collectives() {
    for case in [
        Case::Nan,
        Case::Infinity,
        Case::ProductOverflow,
        Case::PartialOverflow,
        Case::ReductionOverflow,
    ] {
        let scenario = Scenario::new(case);
        let reference = Reference::new(scenario, 0);
        for lane in 0..LANES {
            let task = compare_lane(scenario, 0, lane, &reference);
            assert!(!task.valid && task.output.is_empty());
        }
    }
}

#[test]
fn first_row_failure_poisons_cached_second_partial_before_its_reduction() {
    for row in [0, 62] {
        let mut scenario = Scenario::new(Case::Pattern);
        scenario.missing_weight = Some((row, 63));
        let reference = Reference::new(scenario, 0);
        for lane in 0..LANES {
            let task = compare_lane(scenario, 0, lane, &reference);
            assert!(!task.valid);
        }
        assert_eq!(reference.output.len(), row);
        assert!(
            reference.partials[row + 1]
                .iter()
                .all(|value| value.is_nan())
        );
    }
}

#[test]
fn second_row_failure_does_not_prematurely_reject_the_first_row() {
    for row in [1, 63] {
        let mut scenario = Scenario::new(Case::Pattern);
        scenario.missing_weight = Some((row, 6143));
        let reference = Reference::new(scenario, 4032);
        for lane in 0..LANES {
            assert!(!compare_lane(scenario, 4032, lane, &reference).valid);
        }
        assert_eq!(reference.output.len(), row);
    }
}

#[test]
fn failed_even_and_odd_writes_preserve_sticky_rejection_and_census() {
    for row in [0, 1, 62, 63] {
        let mut scenario = Scenario::new(Case::Pattern);
        scenario.failed_write = Some(row);
        let reference = Reference::new(scenario, 0);
        for lane in 0..LANES {
            let task = compare_lane(scenario, 0, lane, &reference);
            if lane == 0 {
                assert!(!task.valid);
                assert_eq!(task.writes.len(), row + 1);
            }
        }
        assert_eq!(reference.output.len(), row);
        if row % 2 == 0 {
            assert!(reference.partials[row + 1][0].is_nan());
        }
    }
}

#[test]
fn missing_acquired_input_still_reads_all_weights_and_reduces_all_rows() {
    for inner in [0, 63, 6080, 6143] {
        let mut scenario = Scenario::new(Case::Pattern);
        scenario.missing_input = Some(inner);
        let reference = Reference::new(scenario, 0);
        for lane in 0..LANES {
            assert!(!compare_lane(scenario, 0, lane, &reference).valid);
        }
        assert!(reference.output.is_empty());
    }
}

fn function<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemFn {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .unwrap()
}

#[test]
fn source_selects_only_down2_and_preserves_fixed_rounds_geometry_and_storage() {
    let source = syn::parse_file(include_str!("../src/finite_mlp_tiles_v2.rs")).unwrap();
    let down = &function(&source, "down").block;
    let expected: syn::Block =
        syn::parse_str("{ qwen_claimed_mlp_down_tile2_v1!(task, subgroup); }").unwrap();
    assert_eq!(
        quote::quote!(#down).to_string(),
        quote::quote!(#expected).to_string()
    );
    for name in ["gate", "up"] {
        let body = &function(&source, name).block;
        assert!(
            quote::quote!(#body)
                .to_string()
                .contains("qwen_claimed_mlp_projection_tile_v2")
        );
    }
    let engineering = &function(&source, "engineering_mlp_tiles_v2").block;
    assert!(
        quote::quote!(#engineering)
            .to_string()
            .contains("run_fixed_rounds")
    );
    let entry = function(&source, "ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2");
    let text = quote::quote!(#entry).to_string();
    assert!(text.contains("WaveMlpTileStorageV2"));
    for geometry in [
        "required = [64 , 1 , 1]",
        "max = [64 , 1 , 1]",
        "max_grid = [64 , 1 , 1]",
        "static_shared_memory_bytes = 512",
    ] {
        assert!(text.contains(geometry));
    }
    let norm = &function(&source, "norm").block;
    let swiglu = &function(&source, "swiglu").block;
    assert!(
        quote::quote!(#norm)
            .to_string()
            .contains("qwen_claimed_mlp_norm_v1")
    );
    assert!(
        quote::quote!(#swiglu)
            .to_string()
            .contains("qwen_claimed_mlp_swiglu_v1")
    );
}

#[test]
fn closed_macro_has_two_ordered_scalar_accumulators_without_new_control_exits() {
    let source = syn::parse_file(include_str!("../src/mlp_tile_numerics_v2.rs")).unwrap();
    let macro_item = source
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Macro(item)
                if item
                    .ident
                    .as_ref()
                    .is_some_and(|name| name == "qwen_claimed_mlp_down_tile2_v1") =>
            {
                Some(item)
            }
            _ => None,
        })
        .unwrap();
    let tokens = &macro_item.mac.tokens;
    let text = quote::quote!(#tokens).to_string();
    for shape in [
        "while pair < 32",
        "while step < 96",
        "pair += 1",
        "step += 1",
        "pair * 2",
        "row0 + 1",
        "partial0 += product0",
        "partial1 += product1",
    ] {
        assert!(text.contains(shape), "missing {shape}");
    }
    assert_eq!(text.matches("reduce_sum_f32").count(), 2);
    assert_eq!(text.matches("write_output").count(), 2);
    assert_eq!(text.matches(". input").count(), 1);
    assert_eq!(text.matches(". weight").count(), 2);
    for forbidden in [
        "unsafe",
        "return",
        "break",
        "continue",
        "mul_add",
        "broadcast_f32",
        "write_column",
    ] {
        assert!(
            !text
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .any(|word| word == forbidden)
        );
    }
    assert!(text.contains("if rejected0"));
    assert!(text.contains("partial1 = f32 :: from_bits (0x7fc0_0000)"));
}
