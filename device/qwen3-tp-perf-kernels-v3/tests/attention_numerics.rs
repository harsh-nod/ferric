use fe2o3_device::Bf16;
use std::cell::Cell;

#[macro_use]
#[path = "../src/attention_online.rs"]
mod online;

const SCALE: f32 = f32::from_bits(0x3db5_04f3);

#[derive(Default)]
struct HostMath(Cell<usize>);

impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        self.0.set(self.0.get() + 1);
        value.exp()
    }
}

// Independent ties-to-even conversion; never call the device narrowing helper.
fn bf16(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
}

fn decode(bits: u16) -> f64 {
    f64::from(f32::from_bits(u32::from(bits) << 16))
}

fn ordered(bits: u16) -> i32 {
    if bits & 0x8000 == 0 {
        i32::from(bits)
    } else {
        -i32::from(bits & 0x7fff)
    }
}

fn pair(scores: &[f32], values: &[[f32; 2]], context: usize) -> (u16, u16, bool) {
    let math = HostMath::default();
    let mut visited = Vec::new();
    let result = qwen_attention_online_pair_v1!(
        qwen_attention_emit_pair_v1,
        (),
        context,
        scores.len() - 1,
        token,
        {
            visited.push(token);
            (scores[token], values[token][0], values[token][1], true)
        },
        math
    );
    assert_eq!(visited, (0..scores.len()).collect::<Vec<_>>());
    assert_eq!(math.0.get(), 2 * (scores.len() - 1));
    result
}

fn dense_pair_reference(scores: &[f32], values: &[[f32; 2]]) -> [f64; 2] {
    let maximum = scores
        .iter()
        .copied()
        .map(f64::from)
        .fold(f64::NEG_INFINITY, f64::max);
    let weights: Vec<_> = scores
        .iter()
        .map(|&x| (f64::from(x) - maximum).exp())
        .collect();
    let denominator: f64 = weights.iter().sum();
    std::array::from_fn(|dimension| {
        weights
            .iter()
            .zip(values)
            .map(|(w, v)| w * f64::from(v[dimension]))
            .sum::<f64>()
            / denominator
    })
}

#[test]
fn single_token_preserves_both_halves_and_ties_to_even() {
    for value in [
        0.0,
        -0.0,
        1.0,
        -7.25,
        f32::from_bits(0x3f80_8000),
        f32::from_bits(0x3f81_8000),
    ] {
        let (first, second, finite) = pair(&[-17.0], &[[value, -value]], 2304);
        assert!(finite);
        assert_eq!(first, bf16(value));
        assert_eq!(second, bf16(-value));
    }
}

#[test]
fn equal_scores_have_exact_dyadic_averages() {
    for length in [2, 16, 128, 2048] {
        let values: Vec<_> = (0..length)
            .map(|t| [(t % 2) as f32, -2.0 * (t % 2) as f32])
            .collect();
        let result = pair(&vec![-3.0; length], &values, 8192);
        assert_eq!(result, (bf16(0.5), bf16(-1.0), true));
    }
}

#[test]
fn repeated_maximum_updates_and_exponential_underflow_are_stable() {
    for scores in [
        vec![-3.0, -7.0, -1.0, 2.0, -4.0],
        vec![1000.0, -1000.0, 999.0],
    ] {
        let values: Vec<_> = (0..scores.len())
            .map(|t| [t as f32 - 1.0, 3.0 - t as f32 * 2.0])
            .collect();
        let expected = dense_pair_reference(&scores, &values);
        let (first, second, finite) = pair(&scores, &values, scores.len() + 16);
        assert!(finite);
        for (actual, reference) in [first, second].into_iter().zip(expected) {
            assert!((ordered(actual) - ordered(bf16(reference as f32))).abs() <= 1);
        }
    }
}

struct PagedFixture {
    query: Vec<u16>,
    keys: Vec<u16>,
    values: Vec<u16>,
    logical_keys: Vec<u16>,
    logical_values: Vec<u16>,
    table: Vec<usize>,
    kv_heads: usize,
    query_heads: usize,
}

impl PagedFixture {
    fn new(world_size: usize, context: usize) -> Self {
        let kv_heads = 8 / world_size;
        let query_heads = 32 / world_size;
        let pages = context.div_ceil(16);
        // Reversed physical pages catch accidental use of logical cache offsets.
        let table: Vec<_> = (0..pages).rev().collect();
        let query = (0..query_heads * 128)
            .map(|i| bf16(((i * 13 % 29) as f32 - 14.0) / 32.0))
            .collect();
        let mut keys = vec![0x7fc0; pages * 16 * kv_heads * 128];
        let mut values = keys.clone();
        let mut logical_keys = vec![0; context * kv_heads * 128];
        let mut logical_values = logical_keys.clone();
        for token in 0..context {
            for head in 0..kv_heads {
                for dim in 0..128 {
                    let index =
                        ((table[token / 16] * 16 + token % 16) * kv_heads + head) * 128 + dim;
                    keys[index] =
                        bf16(((token * 7 + head * 11 + dim * 3) % 31) as f32 / 64.0 - 0.25);
                    values[index] =
                        bf16(((token * 17 + head * 5 + dim * 13) % 47) as f32 / 16.0 - 1.5);
                    let logical = (token * kv_heads + head) * 128 + dim;
                    logical_keys[logical] = keys[index];
                    logical_values[logical] = values[index];
                }
            }
        }
        Self {
            query,
            keys,
            values,
            logical_keys,
            logical_values,
            table,
            kv_heads,
            query_heads,
        }
    }

    fn index(&self, token: usize, head: usize, dimension: usize) -> usize {
        ((self.table[token / 16] * 16 + token % 16) * self.kv_heads + head) * 128 + dimension
    }

    fn poison_future(&mut self, position: usize, context: usize) {
        for token in position + 1..context {
            for head in 0..self.kv_heads {
                for dim in 0..128 {
                    let index = self.index(token, head, dim);
                    self.keys[index] = 0x7fc0;
                    self.values[index] = 0x7fc0;
                }
            }
        }
        // Unused whole pages are invalid metadata, not merely zero-valued data.
        for page in position / 16 + 1..self.table.len() {
            self.table[page] = usize::MAX;
        }
    }

    // Independent dense FP64 dot and two-pass softmax, not the online recurrence.
    fn reference(&self, head: usize, position: usize) -> [f64; 128] {
        // The oracle uses original unpaged data and a separate head mapping.
        let kv_head = head * self.kv_heads / self.query_heads;
        let scores: Vec<_> = (0..=position)
            .map(|token| {
                (0..128)
                    .map(|d| {
                        decode(self.query[head * 128 + d])
                            * decode(
                                self.logical_keys[token * self.kv_heads * 128 + kv_head * 128 + d],
                            )
                    })
                    .sum::<f64>()
                    * f64::from(SCALE)
            })
            .collect();
        let maximum = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let weights: Vec<_> = scores.iter().map(|score| (score - maximum).exp()).collect();
        let denominator: f64 = weights.iter().sum();
        std::array::from_fn(|d| {
            weights
                .iter()
                .enumerate()
                .map(|(token, w)| {
                    w * decode(self.logical_values[token * self.kv_heads * 128 + kv_head * 128 + d])
                })
                .sum::<f64>()
                / denominator
        })
    }

    // Model the existing XOR tree in each lane, without assuming identical lane bits.
    fn scores(&self, head: usize, position: usize) -> Vec<[f32; 64]> {
        (0..=position)
            .map(|token| {
                let mut partials: [f32; 64] = std::array::from_fn(|lane| {
                    let product = |d| {
                        Bf16::from_bits(self.query[head * 128 + d]).to_f32()
                            * Bf16::from_bits(self.keys[self.index(token, head / 4, d)]).to_f32()
                    };
                    product(lane) + product(lane + 64)
                });
                for offset in [1, 2, 4, 8, 16, 32] {
                    let before = partials;
                    for lane in 0..64 {
                        partials[lane] = before[lane] + before[lane ^ offset];
                    }
                }
                partials.map(|dot| dot * SCALE)
            })
            .collect()
    }
}

#[test]
fn paged_all_tp_heads_and_both_halves_match_independent_fp64() {
    let mut exact = 0;
    let mut tolerated = 0;
    let mut max_abs_error = 0.0_f64;
    let mut max_bf16_steps = 0;
    for world_size in [1, 2, 8] {
        for (context, position) in [
            (32, 0),
            (32, 15),
            (32, 16),
            (2048, 2047),
            (2304, 2047),
            (2304, 2303),
        ] {
            let mut fixture = PagedFixture::new(world_size, context);
            fixture.poison_future(position, context);
            for head in 0..fixture.query_heads {
                let reference = fixture.reference(head, position);
                let scores = fixture.scores(head, position);
                for lane in 0..64 {
                    let math = HostMath::default();
                    let mut loads = 0;
                    let (first, second, finite) = qwen_attention_online_pair_v1!(
                        qwen_attention_emit_pair_v1,
                        (),
                        context,
                        position,
                        token,
                        {
                            assert!(token <= position);
                            loads += 1;
                            let value = |d| {
                                Bf16::from_bits(fixture.values[fixture.index(token, head / 4, d)])
                                    .to_f32()
                            };
                            (scores[token][lane], value(lane), value(lane + 64), true)
                        },
                        math
                    );
                    assert!(finite);
                    assert_eq!(loads, position + 1);
                    assert_eq!(math.0.get(), 2 * position);
                    for (bits, dim) in [(first, lane), (second, lane + 64)] {
                        let expected = bf16(reference[dim] as f32);
                        let error = (decode(bits) - reference[dim]).abs();
                        let steps = (ordered(bits) - ordered(expected)).abs();
                        max_abs_error = max_abs_error.max(error);
                        max_bf16_steps = max_bf16_steps.max(steps);
                        // Fixed before execution: one BF16 step, with a cancellation
                        // floor of 5e-5 * max_abs_V (max_abs_V is exactly 1.5 here).
                        assert!(
                            steps <= 1 || error <= 5e-5 * 1.5,
                            "tp={world_size} context={context} position={position} head={head} dim={dim} steps={steps} error={error}"
                        );
                        if position == 0 {
                            assert_eq!(bits, expected);
                        }
                        if bits == expected {
                            exact += 1;
                        } else {
                            tolerated += 1;
                        }
                    }
                }
            }
        }
    }
    println!(
        "attention_fp64: exact={exact} tolerated={tolerated} max_abs_error={max_abs_error:e} max_bf16_steps={max_bf16_steps}"
    );
    assert_eq!(exact + tolerated, (32 + 16 + 4) * 6 * 128);
}

#[test]
fn every_lane_rejects_its_nonfinite_score_value_or_product_status() {
    for lane in 0..64 {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for field in 0..3 {
                let math = HostMath::default();
                let mut loads = 0;
                let (_, _, finite) = qwen_attention_online_pair_v1!(
                    qwen_attention_emit_pair_v1,
                    (),
                    4,
                    3,
                    token,
                    {
                        loads += 1;
                        let mut values = [0.0_f32, lane as f32, -(lane as f32)];
                        if token == 1 {
                            values[field] = invalid;
                        }
                        (values[0], values[1], values[2], true)
                    },
                    math
                );
                assert!(!finite);
                assert_eq!(loads, 4);
            }
        }
        let math = HostMath::default();
        let (_, _, finite) = qwen_attention_online_pair_v1!(
            qwen_attention_emit_pair_v1,
            (),
            4,
            3,
            token,
            { (0.0_f32, 1.0_f32, -1.0_f32, token != lane % 4) },
            math
        );
        assert!(!finite);
    }
}

#[test]
fn empty_recurrence_and_overflow_do_not_report_finite_outputs() {
    let math = HostMath::default();
    let (_, _, finite) = qwen_attention_online_pair_v1!(
        qwen_attention_emit_pair_v1,
        (),
        0,
        0,
        token,
        {
            let _ = token;
            (0.0_f32, 0.0_f32, 0.0_f32, true)
        },
        math
    );
    assert!(!finite);
    assert!(!pair(&[0.0, 0.0], &[[f32::MAX; 2]; 2], 2).2);
    // Finite f32 can overflow only at the final BF16 narrowing.
    assert!(!pair(&[0.0], &[[f32::MAX; 2]], 1).2);
}
