use super::*;
use crate::resident_layer::{MLP_BYTES, PREFIX_BYTES};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Token {
    id: usize,
    rank: usize,
    bytes: u64,
}
impl Allocation for Token {
    fn owner(self) -> usize {
        self.rank
    }
    fn bytes(self) -> u64 {
        self.bytes
    }
}
fn roots() -> LayerBindings<Token> {
    let prefix: [[Token; 14]; 2] = std::array::from_fn(|rank| {
        std::array::from_fn(|i| Token {
            id: rank * 100 + i,
            rank,
            bytes: PREFIX_BYTES[i],
        })
    });
    let mlp = std::array::from_fn(|rank| {
        std::array::from_fn(|i| match i {
            5 => prefix[rank][7],
            9 => prefix[rank][13],
            _ => Token {
                id: rank * 100 + 20 + i,
                rank,
                bytes: MLP_BYTES[i],
            },
        })
    });
    LayerBindings {
        prefix,
        mlp,
        final_hidden: [prefix[0][0], prefix[1][0]],
    }
}
fn down() -> [Token; 2] {
    std::array::from_fn(|rank| Token {
        id: 300 + rank * 100,
        rank,
        bytes: 16384,
    })
}
struct Reads {
    position: u32,
    calls: Vec<(usize, u64, u32)>,
    fail: Option<usize>,
    short: Option<usize>,
    nonfinite: bool,
}
impl Reads {
    fn new(position: u32) -> Self {
        Self {
            position,
            calls: vec![],
            fail: None,
            short: None,
            nonfinite: false,
        }
    }
}
impl Reader<Token> for Reads {
    fn read(&mut self, root: Token, offset: u64, count: u32) -> Result<Vec<u8>> {
        let index = self.calls.len();
        self.calls.push((root.id, offset, count));
        if self.fail == Some(index) {
            return Err("injected causal read".into());
        }
        let mut data = if root.id < 200 && root.id % 100 == 5 {
            std::iter::once(self.position)
                .chain((0..144).map(|v| (v + 17) % 144))
                .flat_map(u32::to_le_bytes)
                .collect()
        } else {
            vec![0; count as usize]
        };
        if self.nonfinite && root.id == 7 {
            data[..2].copy_from_slice(&0x7fc1u16.to_le_bytes());
        }
        if self.short == Some(index) {
            data.pop();
        }
        Ok(data)
    }
}
fn capture(position: u32, reads: &mut Reads) -> Result<Snapshot> {
    let mut c = Collector::new(position as u64 + 1, position, 0)?;
    for b in BOUNDARIES {
        c.observe_guarded(b, reads, &roots(), down())?;
    }
    c.finish()
}
fn series() -> Series {
    let mut s = Series::new();
    for p in 0..6 {
        s.push(capture(p, &mut Reads::new(p)).unwrap(), &[0; 8192])
            .unwrap();
    }
    s
}
fn bootstrap() -> Bootstrap {
    Bootstrap {
        schema: crate::finite_guarded_mlp_readiness_wire_v1::POSITION5_SCHEMA.into(),
        child_deadline_ms: 60000,
        sequence: crate::finite_guarded_mlp_long_wire_v2::tests::bootstrap(
            crate::finite_guarded_mlp_long_wire_v2::Profile::Readiness40Position5,
        ),
    }
}

#[test]
fn causal_six_actual_prefixes_have_exact_lengths_physical_offsets_and_down_roots() {
    for p in 0..6 {
        let mut r = Reads::new(p);
        let v = capture(p, &mut r).unwrap();
        assert_eq!(v.parts.len(), 34);
        assert_eq!(v.payload.len(), 256136 + 4096 * p as usize);
        for rank in 0..2 {
            for id in [10, 11] {
                assert!(
                    r.calls
                        .contains(&(rank * 100 + id, 17 * 16 * 1024, 1024 * (p + 1)))
                );
            }
            assert_eq!(r.calls.iter().filter(|c| c.0 == rank * 100 + 13).count(), 1);
            assert!(r.calls.contains(&(down()[rank].id, 0, 16384)));
        }
    }
    assert!(Collector::new(7, 6, 0).is_err());
    assert!(Collector::new(1, 1, 0).is_err());
    assert!(Collector::new(1, 0, 1).is_err());
}

#[test]
fn causal_every_read_failure_and_short_read_prevents_completion() {
    for index in 0..34 {
        for short in [false, true] {
            let mut r = Reads::new(5);
            if short {
                r.short = Some(index);
            } else {
                r.fail = Some(index);
            }
            assert!(capture(5, &mut r).is_err());
            assert_eq!(r.calls.len(), index + 1);
        }
    }
    let mut r = Reads::new(0);
    r.nonfinite = true;
    assert!(capture(0, &mut r).is_err());
}

#[test]
fn causal_completed_cache_mutation_rank_disagreement_and_order_poison_series() {
    for role in [
        Role::UsedKey,
        Role::UsedValue,
        Role::CacheMetadata,
        Role::FinalHidden,
    ] {
        let mut s = Series::new();
        s.push(capture(0, &mut Reads::new(0)).unwrap(), &[0; 8192])
            .unwrap();
        let mut v = capture(1, &mut Reads::new(1)).unwrap();
        let p = v
            .parts
            .iter()
            .find(|p| p.role == role && p.rank == 1)
            .unwrap();
        v.payload[p.offset as usize + if role == Role::CacheMetadata { 4 } else { 0 }] ^= 1;
        assert!(s.push(v, &[0; 8192]).is_err());
        assert!(
            s.push(capture(1, &mut Reads::new(1)).unwrap(), &[0; 8192])
                .is_err()
        );
    }
    assert!(
        Series::new()
            .push(capture(1, &mut Reads::new(1)).unwrap(), &[0; 8192])
            .is_err()
    );
}

#[test]
fn causal_close_failure_incomplete_and_wrong_profile_publish_nothing() {
    let mut called = false;
    assert!(
        Series::new()
            .encode_after_close(&bootstrap(), [2; 32], || {
                called = true;
                Ok(())
            })
            .is_err()
    );
    assert!(!called);
    assert!(
        series()
            .encode_after_close(&bootstrap(), [2; 32], || Err("injected close".into()))
            .is_err()
    );
    let mut b = bootstrap();
    b.sequence.profile = crate::finite_guarded_mlp_long_wire_v2::Profile::Readiness40;
    assert!(
        series()
            .encode_after_close(&b, [2; 32], || {
                called = true;
                Ok(())
            })
            .is_err()
    );
    assert!(!called);
}

#[test]
fn causal_closed_binary_envelope_fits_existing_stderr_without_json_tensor_expansion() {
    let bytes = series()
        .encode_after_close(&bootstrap(), [2; 32], || Ok(()))
        .unwrap();
    assert_eq!(&bytes[..8], MAGIC);
    let head = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert!(head <= META_BYTES);
    assert_eq!(bytes.len(), 16 + head + TOTAL_BYTES);
    assert!(bytes.len() <= 2 << 20);
    let header: serde_json::Value = serde_json::from_slice(&bytes[16..16 + head]).unwrap();
    assert_eq!(header["captures"].as_array().unwrap().len(), 6);
    assert_eq!(header["native_close_confirmed"], true);
    for capture in header["captures"].as_array().unwrap() {
        assert!(capture.get("payload").is_none());
    }
}
