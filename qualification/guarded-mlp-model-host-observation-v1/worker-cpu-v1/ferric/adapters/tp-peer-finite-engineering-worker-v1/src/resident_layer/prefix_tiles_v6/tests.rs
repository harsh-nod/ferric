use super::*;
#[derive(Default)]
struct Fake {
    events: Vec<&'static str>,
    fail: Option<usize>,
    poison: bool,
    tiles: bool,
}
impl Fake {
    fn step(&mut self, event: &'static str) -> Result<()> {
        let n = self.events.len();
        self.events.push(event);
        if self.fail == Some(n) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Fake {
    fn validate(&mut self) -> Result<()> {
        self.step("validate")
    }
    fn prefix(&mut self) -> Result<(PrefixObservation, [u64; 2])> {
        self.step("prefix")?;
        Ok((
            if self.tiles {
                PrefixObservation::Tiles284([[284; 284]; 2])
            } else {
                PrefixObservation::Baseline22([[22; 22]; 2])
            },
            [1, 2],
        ))
    }
    fn prefix_capture(&mut self) -> Result<[[Vec<u8>; 7]; 2]> {
        self.step("prefix capture")?;
        Ok(core::array::from_fn(|_| {
            core::array::from_fn(|_| vec![0; 2])
        }))
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        self.step(if first {
            "first residual"
        } else {
            "last residual"
        })?;
        Ok([3, 4])
    }
    fn residual_capture(&mut self, first: bool) -> Result<[Vec<u8>; 2]> {
        self.step(if first {
            "first capture"
        } else {
            "last capture"
        })?;
        Ok([vec![0; 2], vec![0; 2]])
    }
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])> {
        self.step("mlp")?;
        Ok(([[548; 548]; 2], [5, 6]))
    }
    fn mlp_capture(&mut self) -> Result<[[Vec<u8>; 5]; 2]> {
        self.step("mlp capture")?;
        Ok(core::array::from_fn(|_| {
            core::array::from_fn(|_| vec![0; 2])
        }))
    }
    fn poison(&mut self) {
        self.poison = true;
    }
}
#[test]
fn prefix_layer_captures_before_scratch_reuse_and_keeps_both_residual_fences() {
    for tiles in [false, true] {
        let mut f = Fake {
            tiles,
            ..Default::default()
        };
        let r = coordinate(&mut f).unwrap();
        assert_eq!(
            f.events,
            [
                "validate",
                "prefix",
                "prefix capture",
                "first residual",
                "first capture",
                "mlp",
                "mlp capture",
                "last residual",
                "last capture"
            ]
        );
        assert!(!f.poison);
        assert_eq!(
            matches!(r.completion.prefix, PrefixObservation::Tiles284(_)),
            tiles
        );
        assert_eq!(r.completion.mlp, [[548; 548]; 2]);
    }
}
#[test]
fn prefix_layer_every_failure_prevents_all_later_phases() {
    for fail in 0..9 {
        let mut f = Fake {
            fail: Some(fail),
            tiles: true,
            ..Default::default()
        };
        assert!(coordinate(&mut f).is_err());
        assert!(f.poison);
        assert_eq!(f.events.len(), fail + 1);
    }
}
#[test]
fn prefix_layer_capture_is_bounded_and_finite_without_loosening_signed_zero() {
    assert_eq!(
        CAPTURE_BYTES,
        2 * (8192
            + 6144
            + 4096
            + 2359296
            + 2359296
            + 4096
            + 16384
            + 8192
            + 8192
            + 12288 * 3
            + 16384
            + 8192)
    );
    assert!(CAPTURE_BYTES < 10 << 20);
    for bits in [0u32, 0x80000000, 1, 0x007fffff, 0x7f7fffff] {
        finite(&bits.to_le_bytes(), true).unwrap();
    }
    for bits in [0x7f800000u32, 0xff800000, 0x7fc00001] {
        assert!(finite(&bits.to_le_bytes(), true).is_err());
    }
    for bits in [0x7f80u16, 0xff80, 0x7fc1] {
        assert!(finite(&bits.to_le_bytes(), false).is_err());
    }
    assert!(finite(&[], false).is_err());
    assert!(finite(&[0], false).is_err());
    assert_ne!(0u32.to_le_bytes(), 0x80000000u32.to_le_bytes());
}
