use super::*;
use crate::contract::geometry;

#[test]
fn rope_pairs_preserve_every_local_to_global_head_mapping() {
    for role in [1, 2] {
        for world in [1, 2, 8] {
            let full = geometry(role, 1).unwrap();
            let local = geometry(role, world).unwrap();
            for (full_heads, local_heads) in [
                (full.query_heads, local.query_heads),
                (full.kv_heads, local.kv_heads),
            ] {
                for rank in 0..world {
                    for head in 0..local_heads {
                        let global = rank * local_heads + head;
                        assert!(global < full_heads);
                        for lane in 0..64 {
                            let first =
                                Bf16::from_f32((global * 128 + lane) as f32 / 128.0).to_bits();
                            let second =
                                Bf16::from_f32((global * 128 + lane + 64) as f32 / 128.0).to_bits();
                            let cos = (lane as f32 / 64.0).cos();
                            let sin = (lane as f32 / 64.0).sin();
                            let (a, b) = tp_rope_pair_v1!(first, second, cos, sin);
                            let x = Bf16::from_bits(first).to_f32();
                            let y = Bf16::from_bits(second).to_f32();
                            assert_eq!(a, Bf16::from_f32(x * cos - y * sin).to_bits());
                            assert_eq!(b, Bf16::from_f32(y * cos + x * sin).to_bits());
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[should_panic]
fn nonfinite_rope_table_traps() {
    let _ = tp_rope_pair_v1!(0, 0, f32::NAN, 0.0);
}
