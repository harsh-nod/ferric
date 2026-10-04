//! Shared-source CPU arithmetic versus the separately generated Python oracle.
//! This deliberately uses host sqrt, never the inert GPU Math fallback.
use fe2o3_device::Bf16;
use std::path::{Path, PathBuf};

include!("../src/head_rope_numerics_v3.rs");

struct CpuTask<'a> {
    raw: &'a [u16],
    weights: &'a [u16],
    rotary: &'a [f32],
    query: &'a mut [u16],
    key: &'a mut [u16],
    value: &'a mut [u16],
    lane: usize,
    slot: usize,
    written: usize,
    valid: bool,
}
impl CpuTask<'_> {
    fn lane(&self) -> usize {
        self.lane
    }
    fn reject(&mut self) {
        self.valid = false;
    }
    fn head_input(&self, h: usize, c: usize) -> Option<u16> {
        (self.valid && h < 20 && c < 128).then(|| self.raw[h * 128 + c])
    }
    fn head_weight(&self, h: usize, c: usize) -> Option<u16> {
        (self.valid && h < 20 && c < 128).then(|| self.weights[if h < 16 { c } else { 128 + c }])
    }
    fn rotary(&self, half: usize) -> Option<f32> {
        (self.valid && half < 2).then(|| self.rotary[half * 64 + self.lane])
    }
    fn value(&self, h: usize, half: usize) -> Option<u16> {
        (self.valid && h < 4 && half < 2).then(|| self.raw[2560 + h * 128 + self.lane + half * 64])
    }
    fn write_query_head(&mut self, h: usize, lo: u16, hi: u16) -> bool {
        if !self.valid || h >= 16 || self.written != 2 * h {
            return false;
        }
        self.query[h * 128 + self.lane] = lo;
        self.query[h * 128 + self.lane + 64] = hi;
        self.written += 2;
        true
    }
    fn write_key_value_head(&mut self, h: usize, kl: u16, kh: u16, vl: u16, vh: u16) -> bool {
        if !self.valid || h >= 4 || self.written != 32 + 4 * h {
            return false;
        }
        let i = self.slot * 512 + h * 128 + self.lane;
        self.key[i] = kl;
        self.key[i + 64] = kh;
        self.value[i] = vl;
        self.value[i + 64] = vh;
        self.written += 4;
        true
    }
}

fn directory() -> PathBuf {
    PathBuf::from(
        std::env::var("FE2O3_QKV_POST_REFERENCE_V3").expect(
            "FE2O3_QKV_POST_REFERENCE_V3 must name the independently generated real fixture",
        ),
    )
}
fn u16s(dir: &Path, name: &str, count: usize) -> Vec<u16> {
    let raw = std::fs::read(dir.join(name)).unwrap();
    assert_eq!(raw.len(), count * 2);
    raw.chunks_exact(2)
        .map(|x| u16::from_le_bytes(x.try_into().unwrap()))
        .collect()
}
fn u32s(dir: &Path, name: &str, count: usize) -> Vec<u32> {
    let raw = std::fs::read(dir.join(name)).unwrap();
    assert_eq!(raw.len(), count * 4);
    raw.chunks_exact(4)
        .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
        .collect()
}

#[test]
fn both_ranks_all_positions_match_independent_staged_reference_and_preserve_cache() {
    let dir = directory();
    let weights = u16s(&dir, "head-weights.bf16", 256);
    for rank in 0..2 {
        let raw = u16s(&dir, &format!("raw-rank{rank}.bf16"), 3072);
        let sums = u32s(&dir, &format!("sum-rank{rank}.u32"), 20);
        let norm = u16s(&dir, &format!("head-normalized-rank{rank}.bf16"), 2560);
        for position in [0, 15, 16, 2047, 2048, 2303] {
            let rotary = u32s(&dir, &format!("rotary-{position}.f32"), 128)
                .into_iter()
                .map(f32::from_bits)
                .collect::<Vec<_>>();
            // This is the existing Rust host trig policy, checked against the
            // independently frozen Python FP64-to-FP32 values at every pair.
            for pair in 0..64 {
                let angle = f64::from(position) * 1000000_f64.powf(-(pair as f64) / 64.0);
                assert_eq!(rotary[pair].to_bits(), (angle.cos() as f32).to_bits());
                assert_eq!(rotary[pair + 64].to_bits(), (angle.sin() as f32).to_bits());
            }
            let metadata = u32s(&dir, &format!("metadata-{position}.u32"), 145);
            assert_eq!(metadata[0], position);
            let slot = (metadata[1 + position as usize / 16] * 16 + position % 16) as usize;
            let expected_q = u16s(&dir, &format!("rank{rank}-pos{position}-query.bf16"), 2048);
            let expected_k = u16s(&dir, &format!("rank{rank}-pos{position}-key.bf16"), 512);
            let expected_v = u16s(&dir, &format!("rank{rank}-pos{position}-value.bf16"), 512);
            let mut query = vec![0x7fc1; 2048];
            let mut key = (0..2304 * 512)
                .map(|i| 0x7f81 + (i % 63) as u16)
                .collect::<Vec<_>>();
            let mut value = key.clone();
            for lane in 0..64 {
                let mut task = CpuTask {
                    raw: &raw,
                    weights: &weights,
                    rotary: &rotary,
                    query: &mut query,
                    key: &mut key,
                    value: &mut value,
                    lane,
                    slot,
                    written: 0,
                    valid: true,
                };
                for head in 0..20 {
                    let (sum, inverse, valid) = qwen_head_inverse_v3!(&task, head, x, x.sqrt());
                    assert!(valid);
                    assert_eq!(sum.to_bits(), sums[head]);
                    let (a, b, valid) = qwen_head_weighted_pair_v3!(&task, head, inverse);
                    assert!(valid);
                    assert_eq!(
                        (a.to_bits(), b.to_bits()),
                        (norm[head * 128 + lane], norm[head * 128 + lane + 64])
                    );
                }
                qwen_head_rope_post_v3!(&mut task, x, x.sqrt());
                assert!(task.valid);
                assert_eq!(task.written, 48);
            }
            assert_eq!(query, expected_q);
            assert_eq!(&key[slot * 512..(slot + 1) * 512], expected_k);
            assert_eq!(&value[slot * 512..(slot + 1) * 512], expected_v);
            assert_eq!(expected_v, &raw[2560..]);
            for i in 0..2304 * 512 {
                if i / 512 != slot {
                    assert_eq!(key[i], 0x7f81 + (i % 63) as u16);
                    assert_eq!(value[i], key[i]);
                }
            }
        }
    }
}

#[test]
fn invalid_numeric_inputs_do_not_complete_the_post_task() {
    let dir = directory();
    for case in 0..6 {
        let mut raw = u16s(&dir, "raw-rank0.bf16", 3072);
        let mut weights = u16s(&dir, "head-weights.bf16", 256);
        let mut rotary = u32s(&dir, "rotary-2048.f32", 128)
            .into_iter()
            .map(f32::from_bits)
            .collect::<Vec<_>>();
        match case {
            0 => raw[0] = 0x7fc0,
            1 => raw[0] = 0x7f7f,
            2 => weights[0] = 0x7f80,
            3 => rotary[0] = f32::NAN,
            4 => rotary[64] = f32::INFINITY,
            _ => raw[2560] = 0x7fc0,
        }
        let mut query = vec![0x7fc1; 2048];
        let mut key = vec![0x7fc1; 512];
        let mut value = vec![0x7fc1; 512];
        let mut task = CpuTask {
            raw: &raw,
            weights: &weights,
            rotary: &rotary,
            query: &mut query,
            key: &mut key,
            value: &mut value,
            lane: 0,
            slot: 0,
            written: 0,
            valid: true,
        };
        qwen_head_rope_post_v3!(&mut task, x, x.sqrt());
        assert!(!task.valid, "case {case}");
        assert!(task.written < 48, "case {case}");
    }
}

#[test]
fn serial_128_sum_rejects_the_known_xor_reassociation() {
    let row = [
        16295, 16734, 15610, 16917, 16782, 17267, 16001, 15365, 17397, 15627, 16562, 15898, 15809,
        16186, 17041, 16392, 16277, 16725, 15587, 16964, 16853, 17263, 16043, 15361, 17318, 15708,
        16557, 15923, 15850, 16176, 17133, 16399, 16327, 16704, 15535, 16903, 16800, 17182, 16120,
        15441, 17306, 15648, 16523, 15968, 15767, 16218, 17132, 16489, 16317, 16670, 15584, 16991,
        16879, 17154, 16106, 15478, 17362, 15705, 16590, 15906, 15816, 16184, 17040, 16407, 16375,
        16753, 15605, 16970, 16834, 17178, 16127, 15470, 17358, 15622, 16628, 15991, 15869, 16201,
        17112, 16410, 16373, 16696, 15597, 16969, 16793, 17254, 16057, 15418, 17294, 15655, 16639,
        15968, 15798, 16143, 17091, 16496, 16311, 16755, 15562, 16989, 16884, 17254, 16022, 15451,
        17298, 15676, 16621, 15966, 15860, 16137, 17107, 16410, 16317, 16674, 15498, 16900, 16852,
        17177, 16024, 15439, 17370, 15685, 16576, 15983, 15797, 16183, 17030, 16408,
    ];
    struct Row([u16; 128]);
    impl Row {
        fn head_input(&self, _: usize, column: usize) -> Option<u16> {
            Some(self.0[column])
        }
    }
    let (sum, _, valid) = qwen_head_inverse_v3!(Row(row), 0, x, x.sqrt());
    assert!(valid);
    assert_eq!(sum.to_bits(), 0x49be_1c17);
    assert_ne!(sum.to_bits(), 0x49be_1c1a);
}
