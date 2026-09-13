//! Bounded CPU arithmetic models, not execution of device capabilities or GPU math.

use fe2o3_device::{Bf16, StridedReadView2D};

const WIDTH: usize = 4096;
const CAPACITY: usize = 32;
const EPSILON: f32 = 1e-6_f32;
const SENTINEL: u16 = 0x55aa;

#[derive(Clone)]
struct Fixture {
    rows: u32,
    width: u32,
    grid: [u32; 3],
    epsilon: f32,
    behavior: u32,
    input: Vec<u16>,
    residual: Vec<u16>,
    weight: Vec<u16>,
    fused: Vec<u16>,
}

#[derive(Debug, Eq, PartialEq)]
enum Reject {
    Shape,
    Numerical,
}

#[derive(Default, Debug)]
struct Trace {
    // Load attempts include fallbacks, which do not read the backing slice.
    first_reads: usize,
    first_fallbacks: usize,
    second_reads: usize,
    collectives: Vec<(usize, &'static str)>,
    writes: usize,
    sums: Vec<f32>,
}

fn bits(value: f32) -> u16 {
    Bf16::from_f32(value).to_bits()
}

fn value(word: u16) -> f32 {
    Bf16::from_bits(word).to_f32()
}

fn fixture(rows: u32) -> Fixture {
    Fixture {
        rows,
        width: WIDTH as u32,
        grid: [rows, 1, 1],
        epsilon: EPSILON,
        behavior: 0,
        input: vec![bits(1.0); rows as usize * WIDTH],
        residual: Vec::new(),
        weight: vec![bits(1.0); WIDTH],
        fused: Vec::new(),
    }
}

fn wave_sum(mut lanes: [f32; 64]) -> [f32; 64] {
    for offset in [1, 2, 4, 8, 16, 32] {
        let old = lanes;
        for lane in 0..64 {
            lanes[lane] = old[lane] + old[lane ^ offset];
        }
    }
    lanes
}

fn striped_sum(input: &[u16], row: usize, trace: &mut Trace) -> Result<f32, Reject> {
    let view = StridedReadView2D::from_shared_slice(input, 0, input.len() / WIDTH, WIDTH, WIDTH)
        .map_err(|_| Reject::Shape)?;
    striped_sum_view(&view, row, trace)
}

fn striped_sum_view(
    view: &StridedReadView2D<'_, u16>,
    row: usize,
    trace: &mut Trace,
) -> Result<f32, Reject> {
    let mut partial = [0.0_f32; 64];
    let mut finite = [true; 64];
    for lane in 0..64 {
        for component in 0..64 {
            let column = lane + component * 64;
            let x = Bf16::from_bits(view.load_or(row, column, 0x7fc0));
            trace.first_reads += 1;
            trace.first_fallbacks += usize::from(row >= view.rows() || column >= view.columns());
            let square = x.to_f32() * x.to_f32();
            let next = partial[lane] + square;
            finite[lane] &= x.is_finite() & square.is_finite() & next.is_finite();
            partial[lane] = next;
        }
    }
    trace.collectives.push((row, "sum64"));
    let sums = wave_sum(partial);
    trace.collectives.push((row, "invalid_max64"));
    let any_invalid = finite.iter().any(|valid| !valid);
    if any_invalid || sums.iter().any(|sum| !sum.is_finite()) {
        return Err(Reject::Numerical);
    }
    assert!(sums.iter().all(|sum| sum.to_bits() == sums[0].to_bits()));
    Ok(sums[0])
}

#[test]
fn read_view_preserves_every_valid_coordinate_and_rejects_outside_coordinates() {
    for rows in 1..=CAPACITY {
        let input: Vec<u16> = (0..rows * WIDTH).map(|index| (index as u16).wrapping_mul(257)).collect();
        let view = StridedReadView2D::from_shared_slice(&input, 0, rows, WIDTH, WIDTH).unwrap();
        for row in 0..rows {
            for lane in 0..64 {
                for component in 0..64 {
                    let column = lane + component * 64;
                    assert!(column < WIDTH);
                    assert!(row * WIDTH + column < input.len());
                    assert_eq!(view.load_or(row, column, 0x7fc0), input[row * WIDTH + column]);
                }
            }
        }
        for (row, column) in [(rows, 0), (0, WIDTH), (usize::MAX, 0), (0, usize::MAX)] {
            let word = view.load_or(row, column, 0x7fc0);
            assert_eq!(word, 0x7fc0);
            assert!(!Bf16::from_bits(word).is_finite());
        }
    }
}

#[test]
fn out_of_view_nan_fallback_reaches_both_collectives_before_rejection() {
    let input = fixture(1);
    for (columns, row, fallbacks) in [(WIDTH - 1, 0, 1), (WIDTH, 1, WIDTH)] {
        let view = StridedReadView2D::from_shared_slice(&input.input, 0, 1, columns, WIDTH).unwrap();
        let mut trace = Trace::default();
        assert_eq!(striped_sum_view(&view, row, &mut trace), Err(Reject::Numerical));
        assert_eq!(trace.first_reads, WIDTH);
        assert_eq!(trace.first_fallbacks, fallbacks);
        assert_eq!(trace.collectives, [(row, "sum64"), (row, "invalid_max64")]);
        assert_eq!(trace.second_reads, 0);
        assert_eq!(trace.writes, 0);
    }
}

fn sequential_sum(input: &[u16]) -> Result<f32, Reject> {
    let mut sum = 0.0_f32;
    for &word in input {
        let x = value(word);
        let square = x * x;
        let next = sum + square;
        sum = next;
    }
    if !sum.is_finite() {
        return Err(Reject::Numerical);
    }
    Ok(sum)
}

fn finish_value(input: u16, weight: u16, inverse_rms: f32) -> Result<u16, Reject> {
    let input = Bf16::from_bits(input);
    if !input.is_finite() {
        return Err(Reject::Numerical);
    }
    let normalized = input.to_f32() * inverse_rms;
    let weight = Bf16::from_bits(weight);
    if !weight.is_finite() {
        return Err(Reject::Numerical);
    }
    let weighted = normalized * weight.to_f32();
    if !normalized.is_finite() || !weighted.is_finite() {
        return Err(Reject::Numerical);
    }
    let narrowed = Bf16::from_f32(weighted);
    if !narrowed.is_finite() {
        return Err(Reject::Numerical);
    }
    Ok(narrowed.to_bits())
}

fn run(input: &Fixture, output: &mut [u16], trace: &mut Trace) -> Result<(), Reject> {
    if input.rows == 0 || input.rows > 32 || input.width != 4096 || input.behavior != 0 {
        return Err(Reject::Shape);
    }
    let elements = input.rows as usize * WIDTH;
    if input.input.len() != elements
        || input.weight.len() != WIDTH
        || output.len() != elements
        || !input.residual.is_empty()
        || !input.fused.is_empty()
        || input.epsilon != EPSILON
        || input.grid != [input.rows, 1, 1]
    {
        return Err(Reject::Shape);
    }
    for row in 0..input.rows as usize {
        let sum = striped_sum(&input.input, row, trace)?;
        trace.sums.push(sum);
        let mean_square = sum / input.width as f32;
        let stabilized = mean_square + input.epsilon;
        if !mean_square.is_finite() || !stabilized.is_finite() || stabilized <= 0.0 {
            return Err(Reject::Numerical);
        }
        let denominator = stabilized.sqrt();
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(Reject::Numerical);
        }
        let inverse_rms = 1.0_f32 / denominator;
        if !inverse_rms.is_finite() {
            return Err(Reject::Numerical);
        }
        for lane in 0..64 {
            for component in 0..64 {
                let column = lane + component * 64;
                let index = row * WIDTH + column;
                trace.second_reads += 1;
                output[index] =
                    finish_value(input.input[index], input.weight[column], inverse_rms)?;
                trace.writes += 1;
            }
        }
    }
    Ok(())
}

fn reference(input: &Fixture, row: usize, column: usize) -> f64 {
    let mut sum = 0.0_f64;
    for &word in &input.input[row * WIDTH..(row + 1) * WIDTH] {
        let x = f64::from(value(word));
        sum += x * x;
    }
    let denominator = (sum / WIDTH as f64 + f64::from(EPSILON)).sqrt();
    f64::from(value(input.input[row * WIDTH + column])) / denominator
        * f64::from(value(input.weight[column]))
}

fn compare_reference(input: &Fixture) {
    let before = input.input.clone();
    let weights = input.weight.clone();
    let mut output = vec![SENTINEL; input.input.len()];
    run(input, &mut output, &mut Trace::default()).unwrap();
    for row in 0..input.rows as usize {
        // Bounded sampled coordinates include every lane and both row ends.
        for column in (0..64).chain([64, 127, 1023, 2048, 4095]) {
            let expected = reference(input, row, column);
            let actual = f64::from(value(output[row * WIDTH + column]));
            // Host sanity bound for these fixtures, not a native admission tolerance.
            assert!((actual - expected).abs() <= expected.abs() * 0.0041 + 1e-30);
        }
    }
    assert_eq!(input.input, before);
    assert_eq!(input.weight, weights);
}

#[test]
fn uniform_rows_have_exact_bf16_identity() {
    for rows in [1, 16, 17, 32] {
        let input = fixture(rows);
        let mut output = vec![SENTINEL; input.input.len()];
        run(&input, &mut output, &mut Trace::default()).unwrap();
        assert_eq!(output, input.input);
    }
}

#[test]
fn nonuniform_signed_rows_match_bounded_f64_reference() {
    for rows in [1, 16, 17, 32] {
        let mut input = fixture(rows);
        let pattern = [-8.0, -1.25, -0.125, 0.0, 0.25, 1.5, 4.0];
        for (index, word) in input.input.iter_mut().enumerate() {
            *word = bits(pattern[(index + index / WIDTH) % pattern.len()]);
        }
        for (column, weight) in input.weight.iter_mut().enumerate() {
            *weight = bits([0.5, -1.0, 1.5, 2.0][column % 4]);
        }
        compare_reference(&input);
    }
}

#[test]
fn exponent_mixtures_cover_each_lane_and_component() {
    let mut input = fixture(2);
    for (index, word) in input.input.iter_mut().enumerate() {
        let exponent = (index % 25) as i32 - 12;
        let sign = if index % 3 == 0 { -1.0 } else { 1.0 };
        *word = bits(sign * 2.0_f32.powi(exponent));
    }
    compare_reference(&input);
}

#[test]
fn striped_association_is_not_the_old_left_fold() {
    let mut input = fixture(1);
    input.input.fill(bits(0.03125));
    input.input[0] = bits(256.0);
    let old = sequential_sum(&input.input).unwrap();
    let new = striped_sum(&input.input, 0, &mut Trace::default()).unwrap();
    assert_eq!(old, 65536.0);
    assert!(new > old);
    assert_ne!(old.to_bits(), new.to_bits());
    compare_reference(&input);
}

#[test]
fn output_rounding_keeps_even_ties_and_rejects_narrowing_overflow() {
    for (midpoint, expected) in [
        (0x3f80_8000, 0x3f80),
        (0x3f81_8000, 0x3f82),
        (0xbf80_8000, 0xbf80),
        (0xbf81_8000, 0xbf82),
    ] {
        assert_eq!(
            finish_value(bits(1.0), bits(1.0), f32::from_bits(midpoint)),
            Ok(expected)
        );
    }
    assert_eq!(
        finish_value(bits(1.0), bits(1.0), f32::MAX),
        Err(Reject::Numerical)
    );
}

#[test]
fn zero_signs_and_tiny_finite_inputs_remain_finite() {
    let mut input = fixture(1);
    for (index, word) in input.input.iter_mut().enumerate() {
        *word = [0x0000, 0x8000, 0x0001, 0x8001, 0x0080, 0x8080][index % 6];
    }
    let mut output = vec![SENTINEL; WIDTH];
    run(&input, &mut output, &mut Trace::default()).unwrap();
    assert_eq!(output[0], 0x0000);
    assert_eq!(output[1], 0x8000);
    assert!(output.iter().all(|&word| Bf16::from_bits(word).is_finite()));
}

#[test]
fn invalid_shape_and_lengths_reject_before_reads_or_writes() {
    for mutation in 0..12 {
        let mut input = fixture(1);
        let mut output = vec![SENTINEL; WIDTH];
        match mutation {
            0 => input.rows = 0,
            1 => input.rows = 33,
            2 => input.rows = u32::MAX,
            3 => input.width = 128,
            4 => input.width = 1024,
            5 => input.width = u32::MAX,
            6 => {
                input.input.pop();
            }
            7 => input.input.push(bits(1.0)),
            8 => {
                input.weight.pop();
            }
            9 => input.weight.push(bits(1.0)),
            10 => {
                output.pop();
            }
            11 => output.push(SENTINEL),
            _ => unreachable!(),
        }
        let before = output.clone();
        let mut trace = Trace::default();
        assert_eq!(run(&input, &mut output, &mut trace), Err(Reject::Shape));
        assert_eq!(output, before);
        assert_eq!(trace.first_reads, 0);
        assert!(trace.collectives.is_empty());
    }
}

#[test]
fn exact_epsilon_and_grid_are_required() {
    for epsilon in [
        0.0,
        -EPSILON,
        f32::NAN,
        f32::INFINITY,
        f32::from_bits(EPSILON.to_bits() + 1),
    ] {
        let mut input = fixture(1);
        input.epsilon = epsilon;
        assert_eq!(
            run(&input, &mut vec![SENTINEL; WIDTH], &mut Trace::default()),
            Err(Reject::Shape)
        );
    }
    for grid in [[0, 1, 1], [2, 1, 1], [1, 2, 1], [1, 1, 2]] {
        let mut input = fixture(1);
        input.grid = grid;
        assert_eq!(
            run(&input, &mut vec![SENTINEL; WIDTH], &mut Trace::default()),
            Err(Reject::Shape)
        );
    }
}

#[test]
fn auxiliary_buffers_and_behavior_cannot_enable_fusion() {
    for mutation in 0..4 {
        let mut input = fixture(1);
        match mutation {
            0 => input.behavior = 1,
            1 => input.behavior = u32::MAX,
            2 => input.residual.push(bits(1.0)),
            3 => input.fused.push(SENTINEL),
            _ => unreachable!(),
        }
        let residual = input.residual.clone();
        let fused = input.fused.clone();
        let mut output = vec![SENTINEL; WIDTH];
        assert_eq!(
            run(&input, &mut output, &mut Trace::default()),
            Err(Reject::Shape)
        );
        assert_eq!(output, vec![SENTINEL; WIDTH]);
        assert_eq!(input.residual, residual);
        assert_eq!(input.fused, fused);
    }
}

#[test]
fn invalid_input_in_each_lane_reaches_both_collectives_before_rejection() {
    for lane in 0..64 {
        for word in [0x7f80, 0xff80, 0x7fc1, 0x7f81] {
            let mut input = fixture(1);
            input.input[lane + 63 * 64] = word;
            let mut output = vec![SENTINEL; WIDTH];
            let mut trace = Trace::default();
            assert_eq!(run(&input, &mut output, &mut trace), Err(Reject::Numerical));
            assert_eq!(trace.first_reads, WIDTH);
            assert_eq!(trace.collectives, [(0, "sum64"), (0, "invalid_max64")]);
            assert_eq!(trace.writes, 0);
            assert_eq!(output, vec![SENTINEL; WIDTH]);
        }
    }
}

#[test]
fn finite_square_and_sum_overflow_reject_without_early_lane_exit() {
    for word in [Bf16::MAX.to_bits(), bits(1e19)] {
        let mut input = fixture(1);
        input.input.fill(word);
        let mut trace = Trace::default();
        assert_eq!(
            run(&input, &mut vec![SENTINEL; WIDTH], &mut trace),
            Err(Reject::Numerical)
        );
        assert_eq!(trace.collectives.len(), 2);
        assert_eq!(trace.first_reads, WIDTH);
        assert_eq!(trace.second_reads, 0);
    }
}

#[test]
fn invalid_weights_and_weighted_overflow_are_not_successful_outputs() {
    for weight in [0x7f80, 0xff80, 0x7fc1, 0x7f81] {
        let mut input = fixture(1);
        input.weight[4095] = weight;
        let mut trace = Trace::default();
        assert_eq!(
            run(&input, &mut vec![SENTINEL; WIDTH], &mut trace),
            Err(Reject::Numerical)
        );
        assert_eq!(trace.collectives.len(), 2);
    }
    let mut input = fixture(1);
    input.input.fill(0);
    input.input[0] = bits(1.0);
    input.weight[0] = Bf16::MAX.to_bits();
    assert_eq!(
        run(&input, &mut vec![SENTINEL; WIDTH], &mut Trace::default()),
        Err(Reject::Numerical)
    );
}

#[test]
fn second_pass_and_guarded_inactive_capacity_are_preserved() {
    for rows in [1, 16, 17, 32] {
        let input = fixture(rows);
        let active = rows as usize * WIDTH;
        let mut storage = vec![SENTINEL; CAPACITY * WIDTH + 16];
        let mut trace = Trace::default();
        run(&input, &mut storage[8..8 + active], &mut trace).unwrap();
        assert_eq!(trace.first_reads, active);
        assert_eq!(trace.first_fallbacks, 0);
        assert_eq!(trace.second_reads, active);
        assert_eq!(trace.writes, active);
        assert_eq!(trace.collectives.len(), rows as usize * 2);
        assert!(storage[..8].iter().all(|&word| word == SENTINEL));
        assert!(storage[8 + active..].iter().all(|&word| word == SENTINEL));
    }
}

#[test]
fn changed_inputs_after_rejection_have_no_retained_model_state() {
    let mut input = fixture(1);
    let mut output = vec![SENTINEL; WIDTH];
    run(&input, &mut output, &mut Trace::default()).unwrap();
    input.input[63] = Bf16::NAN.to_bits();
    assert_eq!(
        run(&input, &mut output, &mut Trace::default()),
        Err(Reject::Numerical)
    );
    input.input.fill(bits(-2.0));
    run(&input, &mut output, &mut Trace::default()).unwrap();
    assert_eq!(output, vec![bits(-1.0); WIDTH]);
}
