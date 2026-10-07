//! Host-only load-schedule and arithmetic comparison, not device capability emulation.
//! Both schedules use the same host exp implementation; neither is a GPU math oracle.

use fe2o3_device::Bf16;

#[derive(Clone, Debug, PartialEq)]
struct Fixture {
    rows: usize,
    world: usize,
    pages: usize,
    physical: usize,
    context: usize,
    query: Vec<u16>,
    key: Vec<u16>,
    value: Vec<u16>,
    positions: Vec<u32>,
    table: Vec<u32>,
}

#[derive(Debug, PartialEq)]
enum Reject {
    Shape,
    Position,
    Page,
    Numerical,
}

#[derive(Debug, PartialEq)]
enum Read {
    QueryPair,
    Page(usize),
}

#[derive(Default, Debug)]
struct Trace {
    query_elements: usize,
    reads: Vec<Read>,
}

fn bits(value: f32) -> u16 {
    Bf16::from_f32(value).to_bits()
}

fn value(bits: u16) -> f32 {
    Bf16::from_bits(bits).to_f32()
}

fn fixture(world: usize, rows: usize, context: usize, position: usize) -> Fixture {
    let pages = context.div_ceil(16);
    let physical = (pages + 1).min(512);
    let heads = 32 / world;
    let columns = 8 / world * 128;
    let mut result = Fixture {
        rows,
        world,
        pages,
        physical,
        context,
        query: vec![0; rows * heads * 128],
        key: vec![bits(-2.0); physical * 16 * columns],
        value: vec![bits(-64.0); physical * 16 * columns],
        positions: vec![position as u32; rows],
        table: vec![0; rows * pages],
    };
    for head_row in 0..rows * heads {
        for dimension in 0..128 {
            result.query[head_row * 128 + dimension] =
                bits(if dimension < 64 { 0.25 } else { 0.5 });
        }
    }
    for row in 0..rows {
        for page in 0..pages {
            result.table[row * pages + page] = (pages - 1 - page) as u32;
        }
    }
    for token in 0..pages * 16 {
        let page = pages - 1 - token / 16;
        for column in 0..columns {
            let index = (page * 16 + token % 16) * columns + column;
            result.key[index] = bits(((token % 5) + 1) as f32 * 0.125);
            result.value[index] = bits((token % 16) as f32 * 0.5 + (column % 8) as f32 * 0.125);
        }
    }
    result
}

fn query_pair(input: &Fixture, head_row: usize, trace: &mut Trace) -> [[f32; 2]; 64] {
    trace.reads.push(Read::QueryPair);
    trace.query_elements += 128;
    core::array::from_fn(|lane| {
        [
            value(input.query[head_row * 128 + lane]),
            value(input.query[head_row * 128 + lane + 64]),
        ]
    })
}

fn wave_sum(mut values: [f32; 64]) -> [f32; 64] {
    for offset in [1, 2, 4, 8, 16, 32] {
        let old = values;
        for lane in 0..64 {
            values[lane] = old[lane] + old[lane ^ offset];
        }
    }
    values
}

fn run(
    input: &Fixture,
    head_row: usize,
    hoist: bool,
    output: &mut [u16],
    trace: &mut Trace,
) -> Result<(), Reject> {
    if input.rows == 0
        || input.rows > 32
        || ![1, 2, 8].contains(&input.world)
        || input.pages == 0
        || input.pages > 512
        || input.physical == 0
        || input.physical > 512
        || input.context == 0
        || input.context > 8192
    {
        return Err(Reject::Shape);
    }
    let heads = 32 / input.world;
    let columns = 8 / input.world * 128;
    let head_rows = input.rows * heads;
    if input.query.len() < head_rows * 128
        || input.query.len() > 32 * heads * 128
        || output.len() < head_rows * 128
        || output.len() > 32 * heads * 128
        || input.key.len() != input.physical * 16 * columns
        || input.value.len() != input.physical * 16 * columns
        || input.positions.len() < input.rows
        || input.positions.len() > 32
        || input.table.len() < input.rows * input.pages
        || input.table.len() > 32 * input.pages
        || input.context > input.pages * 16
        || head_row >= head_rows
    {
        return Err(Reject::Shape);
    }
    let row = head_row / heads;
    let position = (input.positions[row] as f32) as usize;
    if position >= input.context {
        return Err(Reject::Position);
    }
    let kv_head = (head_row % heads) / 4;
    let mut query = if hoist {
        query_pair(input, head_row, trace)
    } else {
        [[0.0; 2]; 64]
    };
    let mut maximum = [0.0_f32; 64];
    let mut denominator = [0.0_f32; 64];
    let mut numerator = [[0.0_f32; 2]; 64];
    let mut finite = [true; 64];
    for token in 0..input.context {
        if token > position {
            continue;
        }
        let table_index = row * input.pages + token / 16;
        trace.reads.push(Read::Page(table_index));
        let physical_page = (input.table[table_index] as f32) as usize;
        if physical_page >= input.physical || physical_page >= 512 {
            return Err(Reject::Page);
        }
        if !hoist {
            query = query_pair(input, head_row, trace);
        }
        let base = (physical_page * 16 + token % 16) * columns + kv_head * 128;
        let partial = core::array::from_fn(|lane| {
            let product_0 = query[lane][0] * value(input.key[base + lane]);
            let product_1 = query[lane][1] * value(input.key[base + lane + 64]);
            let partial = product_0 + product_1;
            finite[lane] &= product_0.is_finite() & product_1.is_finite() & partial.is_finite();
            partial
        });
        let dots = wave_sum(partial);
        if dots.iter().any(|dot| !dot.is_finite()) {
            return Err(Reject::Numerical);
        }
        for lane in 0..64 {
            let score = dots[lane] * f32::from_bits(0x3db5_04f3);
            let values = [
                value(input.value[base + lane]),
                value(input.value[base + lane + 64]),
            ];
            finite[lane] &= score.is_finite() & values[0].is_finite() & values[1].is_finite();
            if token == 0 {
                maximum[lane] = score;
                denominator[lane] = 1.0;
                numerator[lane] = values;
            } else {
                let next = if score > maximum[lane] {
                    score
                } else {
                    maximum[lane]
                };
                let previous_weight = (maximum[lane] - next).exp();
                let current_weight = (score - next).exp();
                denominator[lane] = denominator[lane] * previous_weight + current_weight;
                numerator[lane][0] =
                    numerator[lane][0] * previous_weight + values[0] * current_weight;
                numerator[lane][1] =
                    numerator[lane][1] * previous_weight + values[1] * current_weight;
                finite[lane] &= previous_weight.is_finite()
                    & (previous_weight >= 0.0)
                    & current_weight.is_finite()
                    & (current_weight >= 0.0)
                    & denominator[lane].is_finite()
                    & (denominator[lane] > 0.0)
                    & numerator[lane][0].is_finite()
                    & numerator[lane][1].is_finite();
                maximum[lane] = next;
            }
        }
    }
    let mut result = [0_u16; 128];
    for lane in 0..64 {
        let values = [
            numerator[lane][0] / denominator[lane],
            numerator[lane][1] / denominator[lane],
        ];
        let narrowed = [bits(values[0]), bits(values[1])];
        if !finite[lane]
            || values.iter().any(|x| !x.is_finite())
            || narrowed.iter().any(|&x| !value(x).is_finite())
        {
            return Err(Reject::Numerical);
        }
        result[lane] = narrowed[0];
        result[lane + 64] = narrowed[1];
    }
    output[head_row * 128..(head_row + 1) * 128].copy_from_slice(&result);
    Ok(())
}

fn compare(input: &Fixture, head_row: usize) -> (Vec<u16>, Trace, Trace) {
    let before = input.clone();
    let mut baseline = vec![0x55aa; 32 * (32 / input.world) * 128];
    let mut candidate = baseline.clone();
    let mut a = Trace::default();
    let mut b = Trace::default();
    assert_eq!(run(input, head_row, false, &mut baseline, &mut a), Ok(()));
    assert_eq!(run(input, head_row, true, &mut candidate, &mut b), Ok(()));
    assert_eq!(baseline, candidate);
    assert_eq!(*input, before);
    assert!(candidate[..head_row * 128].iter().all(|&x| x == 0x55aa));
    assert!(
        candidate[(head_row + 1) * 128..]
            .iter()
            .all(|&x| x == 0x55aa)
    );
    assert_eq!(
        a.query_elements,
        128 * (input.positions[head_row / (32 / input.world)] as usize + 1)
    );
    assert_eq!(b.query_elements, 128);
    (candidate, a, b)
}

#[test]
fn finite_outputs_and_read_counts_match_across_tp_rows_and_context_boundaries() {
    for world in [1, 2, 8] {
        for rows in [1, 17, 32] {
            for (context, position) in [(1, 0), (16, 15), (17, 16), (256, 255)] {
                let input = fixture(world, rows, context, position);
                compare(&input, rows * (32 / world) - 1);
            }
        }
    }
    for position in [0, 15, 16, 8191] {
        compare(&fixture(8, 1, 8192, position), 3);
    }
}

#[test]
fn causal_mask_and_page_permutation_preserve_exact_host_output() {
    let input = fixture(1, 1, 33, 16);
    let (expected, _, _) = compare(&input, 31);
    let mut future = input.clone();
    let columns = 8 * 128;
    for token in 17..48 {
        let physical = future.table[token / 16] as usize;
        let base = (physical * 16 + token % 16) * columns;
        future.key[base..base + columns].fill(bits(64.0));
        future.value[base..base + columns].fill(bits(224.0));
    }
    assert_eq!(compare(&future, 31).0, expected);
    let mut permuted = input.clone();
    let page_elements = 16 * columns;
    for page in 0..input.physical {
        let target = input.physical - 1 - page;
        permuted.key[target * page_elements..(target + 1) * page_elements]
            .copy_from_slice(&input.key[page * page_elements..(page + 1) * page_elements]);
        permuted.value[target * page_elements..(target + 1) * page_elements]
            .copy_from_slice(&input.value[page * page_elements..(page + 1) * page_elements]);
    }
    for page in &mut permuted.table {
        *page = (input.physical - 1 - *page as usize) as u32;
    }
    assert_eq!(compare(&permuted, 31).0, expected);
    let mut wrong_page = input.clone();
    wrong_page.table[0] = (input.physical - 1) as u32;
    assert_ne!(compare(&wrong_page, 31).0, expected);
}

#[test]
fn invalid_shape_or_position_rejects_before_any_query_read() {
    let original = fixture(2, 1, 17, 16);
    for mutation in 0..15 {
        let mut input = original.clone();
        match mutation {
            0 => input.rows = 0,
            1 => input.rows = 33,
            2 => input.world = 3,
            3 => input.pages = 0,
            4 => input.pages = 513,
            5 => input.physical = 0,
            6 => input.physical = 513,
            7 => input.context = 0,
            8 => input.context = 8193,
            9 => {
                input.query.pop();
            }
            10 => {
                input.key.pop();
            }
            11 => {
                input.value.pop();
            }
            12 => input.positions.clear(),
            13 => {
                input.table.pop();
            }
            14 => input.positions[0] = u32::MAX,
            _ => unreachable!(),
        }
        for hoist in [false, true] {
            let mut output = vec![0x55aa; 32 * 16 * 128];
            let before = output.clone();
            let mut trace = Trace::default();
            assert!(matches!(
                run(&input, 0, hoist, &mut output, &mut trace),
                Err(Reject::Shape | Reject::Position)
            ));
            assert_eq!(trace.query_elements, 0);
            assert_eq!(output, before);
        }
    }
}

#[test]
fn invalid_first_and_later_pages_preserve_rejection_but_change_read_order() {
    for (logical, expected_baseline_pairs) in [(0, 0), (1, 16)] {
        let mut input = fixture(1, 1, 17, 16);
        input.table[logical] = u32::MAX;
        let mut traces = Vec::new();
        for hoist in [false, true] {
            let mut output = vec![0x55aa; 32 * 32 * 128];
            let before = output.clone();
            let mut trace = Trace::default();
            assert_eq!(
                run(&input, 0, hoist, &mut output, &mut trace),
                Err(Reject::Page)
            );
            assert_eq!(output, before);
            traces.push(trace);
        }
        assert_eq!(traces[0].query_elements, expected_baseline_pairs * 128);
        assert_eq!(traces[1].query_elements, 128);
        assert_eq!(traces[0].reads.first(), Some(&Read::Page(0)));
        assert_eq!(traces[1].reads.first(), Some(&Read::QueryPair));
    }
}

#[test]
fn nonfinite_query_key_value_and_overflow_keep_host_rejection() {
    for field in 0..3 {
        for dimension in [0, 64] {
            for invalid in [0x7fc0, 0x7f80, 0xff80] {
                let mut input = fixture(8, 1, 16, 15);
                let base = input.table[0] as usize * 16 * 128;
                match field {
                    0 => input.query[dimension] = invalid,
                    1 => input.key[base + dimension] = invalid,
                    2 => input.value[base + dimension] = invalid,
                    _ => unreachable!(),
                }
                for hoist in [false, true] {
                    let mut output = vec![0x55aa; 32 * 4 * 128];
                    let before = output.clone();
                    assert_eq!(
                        run(&input, 0, hoist, &mut output, &mut Trace::default()),
                        Err(Reject::Numerical)
                    );
                    assert_eq!(output, before);
                }
            }
        }
    }
    let mut input = fixture(8, 1, 1, 0);
    input.query.fill(0x7f7f);
    input.key.fill(0x7f7f);
    for hoist in [false, true] {
        let mut output = vec![0x55aa; 4 * 128];
        assert_eq!(
            run(&input, 0, hoist, &mut output, &mut Trace::default()),
            Err(Reject::Numerical)
        );
        assert!(output.iter().all(|&x| x == 0x55aa));
    }
}

#[test]
fn minimum_and_full_carriers_preserve_inputs_and_inactive_output() {
    let input = fixture(2, 17, 17, 16);
    let head_row = 17 * 16 - 1;
    let (expected, _, _) = compare(&input, head_row);
    let mut full = input.clone();
    full.query.resize(32 * 16 * 128, 0x7fc0);
    full.positions.resize(32, u32::MAX);
    full.table.resize(32 * full.pages, u32::MAX);
    assert_eq!(compare(&full, head_row).0, expected);
    for hoist in [false, true] {
        let mut minimum = vec![0x55aa; 17 * 16 * 128];
        assert_eq!(
            run(&input, head_row, hoist, &mut minimum, &mut Trace::default()),
            Ok(())
        );
        assert_eq!(minimum, expected[..minimum.len()]);
        minimum.pop();
        let before = minimum.clone();
        assert_eq!(
            run(&input, head_row, hoist, &mut minimum, &mut Trace::default()),
            Err(Reject::Shape)
        );
        assert_eq!(minimum, before);
    }
}

#[test]
fn two_query_halves_and_single_token_value_have_independent_checks() {
    let input = fixture(8, 1, 2, 1);
    let (expected, _, _) = compare(&input, 0);
    let mut wrong = input.clone();
    wrong.query[64..128].fill(0);
    assert_ne!(compare(&wrong, 0).0, expected);
    let single = fixture(8, 1, 1, 0);
    let (actual, _, _) = compare(&single, 0);
    let base = single.table[0] as usize * 16 * 128;
    assert_eq!(&actual[..128], &single.value[base..base + 128]);
}

#[test]
fn repeated_calls_do_not_reuse_stale_query_values_after_rejection() {
    for hoist in [false, true] {
        let mut output = vec![0x55aa; 32 * 4 * 128];
        for (rows, query_scale) in [(32, 0.25), (1, 0.75), (17, -0.25)] {
            let mut input = fixture(8, rows, 17, 16);
            input.query.fill(bits(query_scale));
            let head_row = rows * 4 - 1;
            let expected = compare(&input, head_row).0;
            assert_eq!(
                run(&input, head_row, hoist, &mut output, &mut Trace::default()),
                Ok(())
            );
            assert_eq!(
                &output[head_row * 128..(head_row + 1) * 128],
                &expected[head_row * 128..(head_row + 1) * 128]
            );
            let before = output.clone();
            input.table[0] = u32::MAX;
            assert_eq!(
                run(&input, 0, hoist, &mut output, &mut Trace::default()),
                Err(Reject::Page)
            );
            assert_eq!(output, before);
        }
    }
}
