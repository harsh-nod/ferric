use super::*;
use crate::contract::geometry;
use std::vec;
use std::vec::Vec;

struct HostMath;
impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        value.exp()
    }
}
fn values(length: usize, seed: usize) -> Vec<u16> {
    (0..length)
        .map(|i| Bf16::from_f32(((i * 13 + seed) % 29) as f32 / 32.0 - 0.4375).to_bits())
        .collect()
}

#[test]
fn all_rank_local_gqa_heads_match_full_same_source_attention() {
    assert_eq!(ATTENTION_SCALE.to_bits(), 0x3db5_04f3);
    let math = HostMath;
    for role in [1, 2] {
        let full = geometry(role, 1).unwrap();
        let query = values(full.query_heads as usize * 128, 3);
        for count in [1_usize, 3, 17] {
            let keys = values(count * 8 * 128, 5);
            let vals = values(count * 8 * 128, 7);
            for world in [1_u32, 2, 8] {
                let local = geometry(role, world).unwrap();
                let qheads = local.query_heads as usize;
                let kheads = local.kv_heads as usize;
                let ratio = if role == 1 { 4 } else { 2 };
                for rank in 0..world as usize {
                    let qstart = rank * qheads * 128;
                    let local_q = &query[qstart..qstart + qheads * 128];
                    let mut local_k = Vec::new();
                    let mut local_v = Vec::new();
                    for token in 0..count {
                        let start = (token * 8 + rank * kheads) * 128;
                        local_k.extend_from_slice(&keys[start..start + kheads * 128]);
                        local_v.extend_from_slice(&vals[start..start + kheads * 128]);
                    }
                    for head in 0..qheads {
                        let global_head = rank * qheads + head;
                        assert_eq!(global_head / ratio, rank * kheads + head / ratio);
                        for lane in [0_usize, 31, 63] {
                            let expected = tp_attention_pair_v1!(
                                &query,
                                &keys,
                                &vals,
                                count,
                                1_024,
                                global_head,
                                global_head / ratio,
                                lane,
                                math
                            );
                            let actual = tp_attention_pair_v1!(
                                local_q,
                                &local_k,
                                &local_v,
                                count,
                                kheads * 128,
                                head,
                                head / ratio,
                                lane,
                                math
                            );
                            assert_eq!(actual, expected);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn attention_prefix_does_not_read_stale_trailing_capacity() {
    let math = HostMath;
    let query = vec![0_u16; 128];
    let mut keys = vec![Bf16::from_f32(f32::NAN).to_bits(); 256];
    let mut vals = keys.clone();
    keys[..128].fill(0);
    vals[..128].fill(Bf16::from_f32(0.375).to_bits());
    assert_eq!(
        tp_attention_pair_v1!(&query, &keys, &vals, 1, 128, 0, 0, 0, math),
        (
            Bf16::from_f32(0.375).to_bits(),
            Bf16::from_f32(0.375).to_bits()
        )
    );
}

#[test]
#[should_panic]
fn nonfinite_initialized_attention_prefix_traps() {
    let math = HostMath;
    let query = vec![0_u16; 128];
    let keys = vec![Bf16::from_f32(f32::NAN).to_bits(); 128];
    let vals = vec![0_u16; 128];
    let _ = tp_attention_pair_v1!(&query, &keys, &vals, 1, 128, 0, 0, 0, math);
}
