use super::*;
struct Fake {
    events: Vec<&'static str>,
    fail: Option<usize>,
    poisoned: bool,
}
impl Fake {
    fn step(&mut self, s: &'static str) -> Result<()> {
        let n = self.events.len();
        self.events.push(s);
        if self.fail == Some(n) {
            Err(s.into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Fake {
    fn validate(&mut self) -> Result<()> {
        self.step("validate")
    }
    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])> {
        self.step("prefix22")?;
        Ok(([[22; 22]; 2], [1, 2]))
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        self.step(if first { "residual1" } else { "residual2" })?;
        Ok([3, 4])
    }
    fn tiles(&mut self) -> Result<([[u32; WORDS]; 2], [u64; 2])> {
        self.step("tiles548")?;
        Ok(([[548; WORDS]; 2], [5, 6]))
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
#[test]
fn all_layer_coordinator_uses_only_one_real_tiles_phase_between_residuals() {
    for _layer in 0..36 {
        let mut f = Fake {
            events: vec![],
            fail: None,
            poisoned: false,
        };
        let result = coordinate(&mut f).unwrap();
        assert_eq!(
            f.events,
            ["validate", "prefix22", "residual1", "tiles548", "residual2"]
        );
        assert_eq!(result.tiles_states, [[548; 548]; 2]);
        assert_eq!(result.paired_ns[2], [5, 6]);
        assert!(!f.poisoned);
    }
}
#[test]
fn every_phase_error_stops_later_dispatches_and_poison_is_mandatory() {
    for n in 0..5 {
        let mut f = Fake {
            events: vec![],
            fail: Some(n),
            poisoned: false,
        };
        assert!(coordinate(&mut f).is_err());
        assert_eq!(f.events.len(), n + 1);
        assert!(f.poisoned);
    }
}
