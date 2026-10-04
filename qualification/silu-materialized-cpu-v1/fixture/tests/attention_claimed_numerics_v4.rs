use std::cell::Cell;

#[macro_use]
#[path = "../src/attention_online.rs"]
mod online;
#[macro_use]
#[path = "../src/attention_numerics_v4.rs"]
mod claimed;

const CAPACITY: usize = 2304;
const SCALE: f32 = f32::from_bits(0x3db5_04f3);
const POISON: u16 = 0x7fc0;

// Host exp is not OCML. This test executes the production Rust handler but
// explicitly simulates checked storage and Wave64 operations on the CPU.
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

// Independent ties-to-even conversion, not fe2o3's BF16 implementation.
fn bf16(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
}

fn decode(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn ordered(bits: u16) -> i32 {
    if bits & 0x8000 == 0 {
        i32::from(bits)
    } else {
        -i32::from(bits & 0x7fff)
    }
}

struct Fixture {
    position: usize,
    query: Vec<u16>,
    keys: Vec<u16>,
    values: Vec<u16>,
    logical_keys: Vec<u16>,
    logical_values: Vec<u16>,
    pages: [u32; 144],
}

impl Fixture {
    fn new(rank: usize, position: usize) -> Self {
        assert!(rank < 2 && position < CAPACITY);
        // Multiplication by five permutes all 144 pages. Neither rank is an
        // identity layout, and the second rank has a different physical order.
        let pages = std::array::from_fn(|page| ((5 * page + 11 + rank * 17) % 144) as u32);
        let query = (0..2048)
            .map(|i| bf16(((i * 13 + rank * 7) % 29) as f32 / 32.0 - 0.4375))
            .collect();
        let mut fixture = Self {
            position,
            query,
            keys: vec![POISON; CAPACITY * 512],
            values: vec![POISON; CAPACITY * 512],
            logical_keys: vec![POISON; CAPACITY * 512],
            logical_values: vec![POISON; CAPACITY * 512],
            pages,
        };
        for token in 0..=position {
            for head in 0..4 {
                for dimension in 0..128 {
                    let logical = token * 512 + head * 128 + dimension;
                    let physical = (fixture.pages[token / 16] as usize * 16 + token % 16) * 512
                        + head * 128
                        + dimension;
                    let key = bf16(
                        ((token * 7 + head * 11 + dimension * 3 + rank * 5) % 31) as f32 / 64.0
                            - 0.25,
                    );
                    let value = bf16(
                        ((token * 17 + head * 5 + dimension * 13 + rank * 19) % 47) as f32 / 16.0
                            - 1.5,
                    );
                    fixture.keys[physical] = key;
                    fixture.values[physical] = value;
                    fixture.logical_keys[logical] = key;
                    fixture.logical_values[logical] = value;
                }
            }
        }
        fixture
    }

    fn checked_index(
        &self,
        head: usize,
        token: usize,
        page: u32,
        lane: usize,
        half: usize,
    ) -> Option<usize> {
        assert!(
            token <= self.position,
            "the handler accessed a future token"
        );
        if head >= 16
            || token >= CAPACITY
            || page >= 144
            || lane >= 64
            || half >= 2
            || self.pages[token / 16] != page
        {
            return None;
        }
        Some((page as usize * 16 + token % 16) * 512 + (head / 4) * 128 + lane + half * 64)
    }

    fn partial(&self, head: usize, token: usize, lane: usize) -> f32 {
        let product = |half| {
            let key = self
                .checked_index(head, token, self.pages[token / 16], lane, half)
                .map(|index| self.keys[index])
                .unwrap_or(POISON);
            decode(self.query[head * 128 + lane + half * 64]) * decode(key)
        };
        product(0) + product(1)
    }

    fn wave_dots(&self) -> Vec<[f32; 64]> {
        let mut dots = Vec::with_capacity(16 * (self.position + 1));
        for head in 0..16 {
            for token in 0..=self.position {
                let mut lanes = std::array::from_fn(|lane| self.partial(head, token, lane));
                for offset in [1, 2, 4, 8, 16, 32] {
                    let previous = lanes;
                    for lane in 0..64 {
                        lanes[lane] = previous[lane] + previous[lane ^ offset];
                    }
                }
                dots.push(lanes);
            }
        }
        dots
    }

    // Independent dense FP64 dot and two-pass softmax over the original
    // unpaged data. Neither the page mapper nor online recurrence is reused.
    fn reference(&self, head: usize) -> [f64; 128] {
        let kv_head = head * 4 / 16;
        let scores: Vec<f64> = (0..=self.position)
            .map(|token| {
                (0..128)
                    .map(|dimension| {
                        f64::from(decode(self.query[head * 128 + dimension]))
                            * f64::from(decode(
                                self.logical_keys[token * 512 + kv_head * 128 + dimension],
                            ))
                    })
                    .sum::<f64>()
                    * f64::from(SCALE)
            })
            .collect();
        let maximum = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let weights: Vec<_> = scores.iter().map(|score| (score - maximum).exp()).collect();
        let denominator: f64 = weights.iter().sum();
        std::array::from_fn(|dimension| {
            weights
                .iter()
                .enumerate()
                .map(|(token, weight)| {
                    weight
                        * f64::from(decode(
                            self.logical_values[token * 512 + kv_head * 128 + dimension],
                        ))
                })
                .sum::<f64>()
                / denominator
        })
    }
}

// API-shaped storage double. The provider's separate unit tests exercise real
// claimed views and unsafe-construction constraints; this tests handler control
// flow and arithmetic without fabricating a GPU worker or collective token.
struct SimTask<'a> {
    fixture: &'a Fixture,
    lane: usize,
    valid: &'a Cell<bool>,
    output: Vec<u16>,
    writes: usize,
    rejects: usize,
    fail_write_head: Option<usize>,
}

impl SimTask<'_> {
    fn position(&self) -> usize {
        self.fixture.position
    }

    fn reject(&mut self) {
        self.valid.set(false);
        self.rejects += 1;
    }

    fn page(&self, token: usize) -> Option<u32> {
        assert!(token <= self.fixture.position);
        self.valid.get().then(|| self.fixture.pages[token / 16])
    }

    fn query(&self, head: usize, half: usize) -> Option<u16> {
        assert!(head < 16 && half < 2);
        self.valid
            .get()
            .then(|| self.fixture.query[head * 128 + self.lane + half * 64])
    }

    fn key(&self, head: usize, token: usize, page: u32, half: usize) -> Option<u16> {
        if !self.valid.get() {
            return None;
        }
        self.fixture
            .checked_index(head, token, page, self.lane, half)
            .map(|index| self.fixture.keys[index])
    }

    fn value(&self, head: usize, token: usize, page: u32, half: usize) -> Option<u16> {
        if !self.valid.get() {
            return None;
        }
        self.fixture
            .checked_index(head, token, page, self.lane, half)
            .map(|index| self.fixture.values[index])
    }

    fn write_head(&mut self, head: usize, low: u16, high: u16) -> bool {
        assert!(head < 16);
        if !self.valid.get() || self.fail_write_head == Some(head) || self.writes != 2 * head {
            self.valid.set(false);
            return false;
        }
        self.output[head * 128 + self.lane] = low;
        self.output[head * 128 + self.lane + 64] = high;
        self.writes += 2;
        true
    }
}

struct SimWave<'a> {
    fixture: &'a Fixture,
    dots: &'a [[f32; 64]],
    lane: usize,
    valid: &'a Cell<bool>,
    broadcasts: Cell<usize>,
    reductions: Cell<usize>,
}

impl SimWave<'_> {
    fn broadcast_f32<const LANES: usize>(&self, input: f32, source: usize) -> f32 {
        assert_eq!(LANES, 64);
        assert_eq!(source, 0);
        let call = self.broadcasts.get();
        assert_eq!(
            call,
            self.reductions.get(),
            "broadcast precedes each reduction exactly once"
        );
        let token = call % (self.fixture.position + 1);
        let expected = if self.valid.get() {
            self.fixture.pages[token / 16]
        } else {
            u32::MAX
        };
        assert_eq!(input.to_bits(), expected);
        self.broadcasts.set(call + 1);
        // Fixtures use identical metadata in all lanes; invalid fixtures reject
        // symmetrically. This is an explicit CPU model, not a host GPU intrinsic.
        f32::from_bits(expected)
    }

    fn reduce_sum_f32<const LANES: usize>(&self, input: f32) -> f32 {
        assert_eq!(LANES, 64);
        let call = self.reductions.get();
        assert_eq!(self.broadcasts.get(), call + 1);
        let head = call / (self.fixture.position + 1);
        let token = call % (self.fixture.position + 1);
        let expected = if self.valid.get() {
            self.fixture.partial(head, token, self.lane)
        } else {
            f32::NAN
        };
        assert!(input.to_bits() == expected.to_bits() || (input.is_nan() && expected.is_nan()));
        self.reductions.set(call + 1);
        if self.valid.get() {
            self.dots[call][self.lane]
        } else {
            f32::NAN
        }
    }
}

fn run_lane(
    fixture: &Fixture,
    dots: &[[f32; 64]],
    lane: usize,
    poison_exp: bool,
    fail_write_head: Option<usize>,
) -> (Vec<u16>, usize, usize) {
    let valid = Cell::new(true);
    let mut task = SimTask {
        fixture,
        lane,
        valid: &valid,
        output: vec![0xdead; 2048],
        writes: 0,
        rejects: 0,
        fail_write_head,
    };
    let subgroup = SimWave {
        fixture,
        dots,
        lane,
        valid: &valid,
        broadcasts: Cell::new(0),
        reductions: Cell::new(0),
    };
    let math = HostMath {
        calls: Cell::new(0),
        poison: poison_exp,
    };
    qwen_claimed_attention_v4!(task, subgroup, math);
    assert_eq!(subgroup.broadcasts.get(), 16 * (fixture.position + 1));
    assert_eq!(subgroup.reductions.get(), 16 * (fixture.position + 1));
    assert_eq!(math.calls.get(), 32 * fixture.position);
    (task.output, task.writes, task.rejects)
}

fn check_fixture(rank: usize, position: usize, lanes: &[usize]) {
    let fixture = Fixture::new(rank, position);
    let dots = fixture.wave_dots();
    let expected: Vec<_> = (0..16).map(|head| fixture.reference(head)).collect();
    let max_value = fixture
        .logical_values
        .iter()
        .copied()
        .map(decode)
        .filter(|value| value.is_finite())
        .map(|value| f64::from(value.abs()))
        .fold(0.0_f64, f64::max);
    let mut outputs_written = vec![false; 2048];
    for &lane in lanes {
        let (output, written, rejects) = run_lane(&fixture, &dots, lane, false, None);
        assert_eq!((written, rejects), (32, 0));
        for head in 0..16 {
            for dimension in [lane, lane + 64] {
                let index = head * 128 + dimension;
                let reference = expected[head][dimension];
                let expected_bits = bf16(reference as f32);
                let steps = (ordered(output[index]) - ordered(expected_bits)).abs();
                let absolute = (f64::from(decode(output[index])) - reference).abs();
                assert!(
                    steps <= 1 || absolute <= 5e-5 * max_value,
                    "rank={rank} position={position} head={head} dimension={dimension} actual={:04x} expected={expected_bits:04x} steps={steps} abs={absolute}",
                    output[index]
                );
                if position == 0 {
                    // Single-token attention must exactly preserve V's BF16 bits.
                    assert_eq!(
                        output[index],
                        fixture.logical_values[(head / 4) * 128 + dimension]
                    );
                }
                assert!(!outputs_written[index]);
                outputs_written[index] = true;
            }
        }
        for (index, bits) in output.into_iter().enumerate() {
            if index % 128 != lane && index % 128 != lane + 64 {
                assert_eq!(bits, 0xdead, "handler wrote another lane's output");
            }
        }
    }
    assert_eq!(
        outputs_written.iter().filter(|&&written| written).count(),
        lanes.len() * 32
    );
}

#[test]
fn actual_handler_all_heads_and_lanes_match_unpaged_fp64_on_both_ranks() {
    let lanes: Vec<_> = (0..64).collect();
    for rank in 0..2 {
        for position in [0, 16] {
            check_fixture(rank, position, &lanes);
        }
    }
}

#[test]
fn actual_handler_causal_page_boundaries_and_long_context_match_unpaged_fp64() {
    for rank in 0..2 {
        for position in [0, 15, 16, 2047, 2048, 2303] {
            check_fixture(rank, position, &[0, 7, 31, 63]);
        }
    }
}

#[test]
fn actual_handler_nan_query_keeps_every_collective_after_rejection() {
    let mut fixture = Fixture::new(0, 16);
    fixture.query[..128].fill(POISON);
    let dots = fixture.wave_dots();
    for lane in [0, 7, 63] {
        let (output, written, rejects) = run_lane(&fixture, &dots, lane, false, None);
        assert_eq!((written, rejects), (0, 16));
        assert!(output.iter().all(|&bits| bits == 0xdead));
    }
}

#[test]
fn actual_handler_invalid_raw_page_bits_do_not_skip_collectives_or_publish() {
    for invalid in [144, u32::MAX, 0x7fc0_0001, 0xff80_0001] {
        let mut fixture = Fixture::new(0, 16);
        fixture.pages[0] = invalid;
        let dots = fixture.wave_dots();
        for lane in [0, 63] {
            let (output, written, rejects) = run_lane(&fixture, &dots, lane, false, None);
            assert_eq!((written, rejects), (0, 16));
            assert!(output.iter().all(|&bits| bits == 0xdead));
        }
    }
}

#[test]
fn actual_handler_nonfinite_math_keeps_later_heads_collective() {
    let fixture = Fixture::new(1, 16);
    let dots = fixture.wave_dots();
    let (output, written, rejects) = run_lane(&fixture, &dots, 0, true, None);
    assert_eq!((written, rejects), (0, 16));
    assert!(output.iter().all(|&bits| bits == 0xdead));
}

#[test]
fn actual_handler_failed_store_does_not_skip_later_collectives() {
    let fixture = Fixture::new(0, 16);
    let dots = fixture.wave_dots();
    let (output, written, rejects) = run_lane(&fixture, &dots, 0, false, Some(3));
    assert_eq!((written, rejects), (6, 13));
    assert!(output[3 * 128..].iter().all(|&bits| bits == 0xdead));
}
