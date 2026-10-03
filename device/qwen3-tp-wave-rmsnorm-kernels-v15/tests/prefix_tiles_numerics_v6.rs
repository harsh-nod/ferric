//! Production handler macros with checked CPU storage/collective doubles.
//! No device worker, GPU collective, OCML equivalence or launch proof is made.
use fe2o3_device::Bf16;
use std::cell::Cell;

#[macro_use]
#[path = "../src/wave_numerics_v1.rs"]
mod wave;
#[macro_use]
#[path = "../src/output_projection_numerics_v5.rs"]
mod output;
#[macro_use]
#[path = "../src/attention_online.rs"]
mod online;
#[macro_use]
#[path = "../src/attention_numerics_v4.rs"]
mod old_attention;
#[macro_use]
#[path = "../src/prefix_tiles_numerics_v6.rs"]
mod tiled;

const POISON: u16 = 0x7fc0;
fn decode(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}
fn bf16(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
}
fn same_float(actual: f32, expected: f32) {
    assert!(actual.to_bits() == expected.to_bits() || (actual.is_nan() && expected.is_nan()));
}
fn xor_tree(mut values: [f32; 64]) -> [f32; 64] {
    for offset in [1, 2, 4, 8, 16, 32] {
        let old = values;
        for lane in 0..64 {
            values[lane] = old[lane] + old[lane ^ offset];
        }
    }
    values
}

struct DotData {
    rank: usize,
    inner: usize,
    special: bool,
    exceptional: Option<u16>,
}
impl DotData {
    fn input(&self, k: usize) -> u16 {
        assert!(k < self.inner);
        if let Some(bits) = self.exceptional {
            return bits;
        }
        if self.special {
            0x3f80
        } else {
            bf16(((k * 13 + self.rank * 5) % 31) as f32 / 16.0 - 0.9375)
        }
    }
    fn weight(&self, row: usize, k: usize) -> u16 {
        assert!(k < self.inner);
        if self.exceptional.is_some() {
            return 0x7f7f;
        }
        if self.special {
            return match (row % 4, k) {
                (0, 0) => bf16(1.0),
                (0, 64) => bf16(1.0 / 256.0),
                (1, 0) => bf16(1.0),
                (1, 64) => bf16(1.0 / 256.0),
                (1, 128) => bf16(1.0 / 512.0),
                (2, 0) => bf16(33554432.0),
                (2, 1) => bf16(1.0),
                (2, 2) => bf16(-33554432.0),
                (2, 3) => bf16(1.0),
                (3, 0) => bf16(-0.0),
                _ => 0,
            };
        }
        bf16(((row * 19 + k * 7 + self.rank * 11) % 43) as f32 / 32.0 - 0.625)
    }
    fn partials(&self, row: usize) -> [f32; 64] {
        std::array::from_fn(|lane| {
            let mut sum = 0.0_f32;
            for k in (lane..self.inner).step_by(64) {
                let product = decode(self.input(k)) * decode(self.weight(row, k));
                sum += product;
            }
            sum
        })
    }
}
struct DotTask<'a> {
    data: &'a DotData,
    base: usize,
    rows: usize,
    lane: usize,
    valid: &'a Cell<bool>,
    result: Vec<u32>,
    writes: usize,
    fail_row: Option<usize>,
}
impl DotTask<'_> {
    fn lane(&self) -> usize {
        self.lane
    }
    fn input(&self, k: usize) -> Option<u16> {
        self.valid.get().then(|| self.data.input(k))
    }
    fn weight(&self, row: usize, k: usize) -> Option<u16> {
        assert!(row < self.rows);
        self.valid
            .get()
            .then(|| self.data.weight(self.base + row, k))
    }
    fn reject(&mut self) {
        self.valid.set(false);
    }
    fn store(&mut self, row: usize, bits: u32) -> bool {
        assert_eq!(self.lane, 0);
        if !self.valid.get() || self.fail_row == Some(row) || row != self.writes {
            return false;
        }
        self.result[row] = bits;
        self.writes += 1;
        true
    }
    fn write_column(&mut self, row: usize, bits: u16) -> bool {
        self.store(row, u32::from(bits))
    }
    fn write_output(&mut self, row: usize, value: f32) -> bool {
        self.store(row, value.to_bits())
    }
}
struct DotWave<'a> {
    data: &'a DotData,
    base: usize,
    lane: usize,
    valid: &'a Cell<bool>,
    calls: Cell<usize>,
    broadcasts: Cell<usize>,
}
impl DotWave<'_> {
    fn reduce_sum_f32<const LANES: usize>(&self, partial: f32) -> f32 {
        assert_eq!(LANES, 64);
        let row = self.base + self.calls.get();
        self.calls.set(self.calls.get() + 1);
        if !self.valid.get() {
            assert!(partial.is_nan());
            return f32::NAN;
        }
        let all = self.data.partials(row);
        same_float(partial, all[self.lane]);
        xor_tree(all)[self.lane]
    }
    fn broadcast_f32<const LANES: usize>(&self, _value: f32, source: usize) -> f32 {
        assert_eq!((LANES, source), (64, 0));
        assert_eq!(self.broadcasts.get() + 1, self.calls.get());
        self.broadcasts.set(self.broadcasts.get() + 1);
        if !self.valid.get() {
            return f32::NAN;
        }
        xor_tree(self.data.partials(self.base + self.calls.get() - 1))[0]
    }
}
fn dot_tile(
    data: &DotData,
    base: usize,
    lane: usize,
    qkv: bool,
    fail_row: Option<usize>,
) -> (Vec<u32>, usize) {
    let valid = Cell::new(true);
    let mut task = DotTask {
        data,
        base,
        rows: 64,
        lane,
        valid: &valid,
        result: vec![0xdead_beef; 64],
        writes: 0,
        fail_row,
    };
    let subgroup = DotWave {
        data,
        base,
        lane,
        valid: &valid,
        calls: Cell::new(0),
        broadcasts: Cell::new(0),
    };
    if qkv {
        qwen_claimed_projection_tile_v6!(task, subgroup);
    } else {
        qwen_claimed_output_projection_tile_v6!(task, subgroup);
    }
    assert_eq!(subgroup.calls.get(), 64);
    assert_eq!(subgroup.broadcasts.get(), if qkv { 64 } else { 0 });
    (task.result, task.writes)
}

// The V5 projection body is copied verbatim; its AST is checked separately.
fn old_projection(task: &mut DotTask<'_>, subgroup: &DotWave<'_>) {
    let lane = task.lane();
    let mut column = 0;
    while column < 256 {
        // Invalid lanes continue all256 collectives; rejection is gathered
        // uniformly afterward, so no lane exits around a subgroup operation.
        let (sum, narrowed, finite) = qwen_wave_dot_v1!(
            lane,
            inner,
            Bf16::from_bits(match task.input(inner) {
                Some(value) => value,
                None => 0x7fc0,
            })
            .to_f32(),
            Bf16::from_bits(match task.weight(column, inner) {
                Some(value) => value,
                None => 0x7fc0,
            })
            .to_f32(),
            subgroup
        );
        if !finite || !sum.is_finite() || !narrowed.is_finite() {
            task.reject();
        } else if lane == 0 && !task.write_column(column, narrowed.to_bits()) {
            task.reject();
        }
        column += 1;
    }
}

#[test]
fn tiled_qkv_boundary_rows_match_independent_lane_xor_and_bf16_rne() {
    for rank in 0..2 {
        let data = DotData {
            rank,
            inner: 4096,
            special: false,
            exceptional: None,
        };
        for base in [0, 1984, 2048, 2496, 2560, 3008] {
            let old_base = base / 256 * 256;
            let valid = Cell::new(true);
            let mut old = DotTask {
                data: &data,
                base: old_base,
                rows: 256,
                lane: 0,
                valid: &valid,
                result: vec![0xdead_beef; 256],
                writes: 0,
                fail_row: None,
            };
            let old_wave = DotWave {
                data: &data,
                base: old_base,
                lane: 0,
                valid: &valid,
                calls: Cell::new(0),
                broadcasts: Cell::new(0),
            };
            old_projection(&mut old, &old_wave);
            assert_eq!(
                (old.writes, old_wave.calls.get(), old_wave.broadcasts.get()),
                (256, 256, 256)
            );
            let (actual, writes) = dot_tile(&data, base, 0, true, None);
            assert_eq!(writes, 64);
            assert_eq!(&actual, &old.result[base - old_base..base - old_base + 64]);
            for row in 0..64 {
                assert_eq!(
                    actual[row],
                    u32::from(bf16(xor_tree(data.partials(base + row))[0]))
                );
            }
            let (other, writes) = dot_tile(&data, base, 63, true, None);
            assert_eq!(writes, 0);
            assert!(other.iter().all(|&x| x == 0xdead_beef));
        }
    }
}

#[test]
fn tiled_o_boundary_rows_are_bitwise_equal_to_whole_stage_and_independent_fp32() {
    for rank in 0..2 {
        let data = DotData {
            rank,
            inner: 2048,
            special: false,
            exceptional: None,
        };
        let valid = Cell::new(true);
        let mut task = DotTask {
            data: &data,
            base: 0,
            rows: 4096,
            lane: 0,
            valid: &valid,
            result: vec![0xdead_beef; 4096],
            writes: 0,
            fail_row: None,
        };
        let subgroup = DotWave {
            data: &data,
            base: 0,
            lane: 0,
            valid: &valid,
            calls: Cell::new(0),
            broadcasts: Cell::new(0),
        };
        qwen_claimed_output_projection_v5!(task, subgroup);
        assert_eq!(
            (task.writes, subgroup.calls.get(), subgroup.broadcasts.get()),
            (4096, 4096, 0)
        );
        for base in [0, 1984, 4032] {
            let (actual, writes) = dot_tile(&data, base, 0, false, None);
            assert_eq!(writes, 64);
            assert_eq!(&actual, &task.result[base..base + 64]);
            for row in 0..64 {
                assert_eq!(
                    actual[row],
                    xor_tree(data.partials(base + row))[0].to_bits()
                );
            }
            assert_eq!(dot_tile(&data, base, 63, false, None).1, 0);
        }
    }
}

#[test]
fn exact_rounding_ties_cancellation_and_fp32_o_are_not_reassociated_or_narrowed() {
    let qkv = DotData {
        rank: 0,
        inner: 4096,
        special: true,
        exceptional: None,
    };
    let o = DotData {
        rank: 0,
        inner: 2048,
        special: true,
        exceptional: None,
    };
    let (q, _) = dot_tile(&qkv, 0, 0, true, None);
    let (f, _) = dot_tile(&o, 0, 0, false, None);
    assert_eq!((q[0], q[1]), (0x3f80, 0x3f81));
    assert_eq!(f[0], (1.0_f32 + 1.0 / 256.0).to_bits());
    assert_ne!(f[0], decode(bf16(f32::from_bits(f[0]))).to_bits());
    let mut ascending = 0.0_f32;
    for k in 0..2048 {
        ascending += decode(o.input(k)) * decode(o.weight(2, k));
    }
    assert_ne!(
        f[2],
        ascending.to_bits(),
        "fixture must distinguish an ascending scalar reduction"
    );
}

#[test]
fn rejected_row_still_executes_all_64_reductions_without_later_writes() {
    for qkv in [false, true] {
        let data = DotData {
            rank: 0,
            inner: if qkv { 4096 } else { 2048 },
            special: false,
            exceptional: None,
        };
        for failure in [0, 31, 63] {
            let (actual, writes) = dot_tile(&data, 0, 0, qkv, Some(failure));
            assert_eq!(writes, failure);
            assert!(actual[failure..].iter().all(|&x| x == 0xdead_beef));
        }
    }
}

#[test]
fn nonfinite_inputs_and_finite_product_overflow_reject_without_skipping_reductions() {
    for qkv in [false, true] {
        for bits in [0x7fc0, 0x7f80, 0x7f7f] {
            let data = DotData {
                rank: 0,
                inner: if qkv { 4096 } else { 2048 },
                special: false,
                exceptional: Some(bits),
            };
            for lane in [0, 63] {
                let (actual, writes) = dot_tile(&data, 0, lane, qkv, None);
                assert_eq!(writes, 0);
                assert!(actual.iter().all(|&x| x == 0xdead_beef));
            }
        }
    }
}

struct AttentionData {
    rank: usize,
    position: usize,
    pages: [u32; 144],
}
impl AttentionData {
    fn new(rank: usize, position: usize) -> Self {
        Self {
            rank,
            position,
            pages: std::array::from_fn(|p| ((5 * p + 11 + rank * 17) % 144) as u32),
        }
    }
    fn query(&self, head: usize, dimension: usize) -> u16 {
        bf16(((head * 128 + dimension) * 13 + self.rank * 7).rem_euclid(29) as f32 / 32.0 - 0.4375)
    }
    fn key(&self, head: usize, token: usize, dimension: usize) -> u16 {
        bf16(((token * 7 + head * 11 + dimension * 3 + self.rank * 5) % 31) as f32 / 64.0 - 0.25)
    }
    fn value(&self, head: usize, token: usize, dimension: usize) -> u16 {
        bf16(((token * 17 + head * 5 + dimension * 13 + self.rank * 19) % 47) as f32 / 16.0 - 1.5)
    }
    fn partials(&self, head: usize, token: usize) -> [f32; 64] {
        std::array::from_fn(|lane| {
            let p0 = decode(self.query(head, lane)) * decode(self.key(head / 4, token, lane));
            let p1 =
                decode(self.query(head, lane + 64)) * decode(self.key(head / 4, token, lane + 64));
            p0 + p1
        })
    }
    // Independently written scalar recurrence, with explicit f32 operations and
    // independent BF16 conversion. Host exp is shared only as a math primitive.
    fn reference(&self, dots: &[[f32; 64]], head: usize, lane: usize) -> [u16; 2] {
        let score =
            |t: usize| dots[head * (self.position + 1) + t][lane] * f32::from_bits(0x3db5_04f3);
        let mut maximum = score(0);
        let mut denominator = 1.0_f32;
        let mut numerator = [
            decode(self.value(head / 4, 0, lane)),
            decode(self.value(head / 4, 0, lane + 64)),
        ];
        for token in 1..=self.position {
            let s = score(token);
            let next = if s > maximum { s } else { maximum };
            let prior = (maximum - next).exp();
            let weight = (s - next).exp();
            denominator = denominator * prior + weight;
            for half in 0..2 {
                numerator[half] = numerator[half] * prior
                    + decode(self.value(head / 4, token, lane + half * 64)) * weight;
            }
            maximum = next;
        }
        numerator.map(|x| bf16(x / denominator))
    }
}
struct AttentionTask<'a> {
    data: &'a AttentionData,
    lane: usize,
    selected: Option<usize>,
    valid: &'a Cell<bool>,
    out: Vec<u16>,
    writes: usize,
}
impl AttentionTask<'_> {
    fn head(&self) -> usize {
        self.selected.unwrap()
    }
    fn position(&self) -> usize {
        self.data.position
    }
    fn reject(&mut self) {
        self.valid.set(false);
    }
    fn check(&self, head: usize) {
        assert!(head < 16 && self.selected.is_none_or(|h| h == head));
    }
    fn page(&self, token: usize) -> Option<u32> {
        assert!(token <= self.data.position);
        self.valid.get().then(|| self.data.pages[token / 16])
    }
    fn query(&self, head: usize, half: usize) -> Option<u16> {
        self.check(head);
        assert!(half < 2);
        self.valid
            .get()
            .then(|| self.data.query(head, self.lane + half * 64))
    }
    fn logical(
        &self,
        head: usize,
        token: usize,
        page: u32,
        half: usize,
    ) -> Option<(usize, usize, usize)> {
        self.check(head);
        assert!(token <= self.data.position && half < 2);
        if !self.valid.get() {
            return None;
        }
        let logical_page = self
            .data
            .pages
            .iter()
            .position(|&p| p == page)
            .expect("checked page");
        let logical_token = logical_page * 16 + token % 16;
        assert_eq!(logical_token, token);
        Some((head / 4, logical_token, self.lane + half * 64))
    }
    fn key(&self, h: usize, t: usize, p: u32, half: usize) -> Option<u16> {
        self.logical(h, t, p, half)
            .map(|(h, t, d)| self.data.key(h, t, d))
    }
    fn value(&self, h: usize, t: usize, p: u32, half: usize) -> Option<u16> {
        self.logical(h, t, p, half)
            .map(|(h, t, d)| self.data.value(h, t, d))
    }
    fn write_head(&mut self, h: usize, a: u16, b: u16) -> bool {
        self.check(h);
        assert_eq!(self.writes, if self.selected.is_some() { 0 } else { 2 * h });
        if !self.valid.get() {
            return false;
        }
        self.out[h * 128 + self.lane] = a;
        self.out[h * 128 + self.lane + 64] = b;
        self.writes += 2;
        true
    }
}
struct AttentionWave<'a> {
    data: &'a AttentionData,
    dots: &'a [[f32; 64]],
    lane: usize,
    head: Option<usize>,
    valid: &'a Cell<bool>,
    calls: Cell<usize>,
    broadcasts: Cell<usize>,
}
impl AttentionWave<'_> {
    fn broadcast_f32<const N: usize>(&self, input: f32, source: usize) -> f32 {
        assert_eq!((N, source), (64, 0));
        assert_eq!(self.calls.get(), self.broadcasts.get());
        let token = self.calls.get() % (self.data.position + 1);
        let expected = if self.valid.get() {
            self.data.pages[token / 16]
        } else {
            u32::MAX
        };
        assert_eq!(input.to_bits(), expected);
        self.broadcasts.set(self.broadcasts.get() + 1);
        f32::from_bits(expected)
    }
    fn reduce_sum_f32<const N: usize>(&self, input: f32) -> f32 {
        assert_eq!(N, 64);
        let call = self.calls.get();
        assert_eq!(self.broadcasts.get(), call + 1);
        self.calls.set(call + 1);
        let token = call % (self.data.position + 1);
        let head = self.head.unwrap_or(call / (self.data.position + 1));
        if !self.valid.get() {
            assert!(input.is_nan());
            return f32::NAN;
        }
        same_float(input, self.data.partials(head, token)[self.lane]);
        self.dots[head * (self.data.position + 1) + token][self.lane]
    }
}
struct HostMath {
    calls: Cell<usize>,
    poison: bool,
}
impl HostMath {
    fn exp_f32(&self, x: f32) -> f32 {
        self.calls.set(self.calls.get() + 1);
        if self.poison { f32::NAN } else { x.exp() }
    }
}
fn attention_lane(
    data: &AttentionData,
    dots: &[[f32; 64]],
    lane: usize,
    head: Option<usize>,
    poison: bool,
) -> (Vec<u16>, usize) {
    let valid = Cell::new(true);
    let mut task = AttentionTask {
        data,
        lane,
        selected: head,
        valid: &valid,
        out: vec![0xdead; 2048],
        writes: 0,
    };
    let subgroup = AttentionWave {
        data,
        dots,
        lane,
        head,
        valid: &valid,
        calls: Cell::new(0),
        broadcasts: Cell::new(0),
    };
    let math = HostMath {
        calls: Cell::new(0),
        poison,
    };
    if head.is_some() {
        qwen_claimed_attention_head_v6!(task, subgroup, math);
    } else {
        qwen_claimed_attention_v4!(task, subgroup, math);
    }
    let heads = if head.is_some() { 1 } else { 16 };
    assert_eq!(subgroup.calls.get(), heads * (data.position + 1));
    assert_eq!(subgroup.broadcasts.get(), heads * (data.position + 1));
    assert_eq!(math.calls.get(), heads * 2 * data.position);
    (task.out, task.writes)
}

#[test]
fn every_head_matches_old_whole_stage_and_independent_online_recurrence_at_causal_boundaries() {
    for rank in 0..2 {
        for position in [0, 15, 16, 17, 2047, 2048, 2302] {
            let data = AttentionData::new(rank, position);
            let dots: Vec<_> = (0..16)
                .flat_map(|h| (0..=position).map(move |t| (h, t)))
                .map(|(h, t)| xor_tree(data.partials(h, t)))
                .collect();
            for lane in [0, 31, 63] {
                let (old, writes) = attention_lane(&data, &dots, lane, None, false);
                assert_eq!(writes, 32);
                for head in 0..16 {
                    let (new, writes) = attention_lane(&data, &dots, lane, Some(head), false);
                    assert_eq!(writes, 2);
                    let reference = data.reference(&dots, head, lane);
                    for half in 0..2 {
                        let i = head * 128 + lane + half * 64;
                        assert_eq!(new[i], old[i]);
                        assert_eq!(new[i], reference[half]);
                    }
                    for (i, &bits) in new.iter().enumerate() {
                        if i != head * 128 + lane && i != head * 128 + lane + 64 {
                            assert_eq!(bits, 0xdead);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn rejected_head_keeps_all_causal_collectives_and_never_writes_other_heads() {
    let data = AttentionData::new(1, 17);
    let dots: Vec<_> = (0..16)
        .flat_map(|h| (0..=17).map(move |t| (h, t)))
        .map(|(h, t)| xor_tree(data.partials(h, t)))
        .collect();
    for head in [0, 3, 4, 15] {
        for lane in [0, 63] {
            let (new, writes) = attention_lane(&data, &dots, lane, Some(head), true);
            assert_eq!(writes, 0);
            assert!(new.iter().all(|&x| x == 0xdead));
        }
    }
}
