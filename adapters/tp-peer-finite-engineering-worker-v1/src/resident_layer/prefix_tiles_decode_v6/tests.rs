use super::*;
struct Mock {
    events: Vec<&'static str>,
    fail: Option<usize>,
    poisoned: bool,
}
impl Mock {
    fn call(&mut self, name: &'static str) -> Result<()> {
        let at = self.events.len();
        self.events.push(name);
        if self.fail == Some(at) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Mock {
    fn validate(&mut self) -> Result<()> {
        self.call("validate")
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])> {
        self.call("prefix_pair_complete")?;
        Ok(([[1; 284]; 2], [10, 11]))
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        self.call(if first {
            "first_residual_pair_complete"
        } else {
            "last_residual_pair_complete"
        })?;
        Ok(if first { [20, 21] } else { [40, 41] })
    }
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])> {
        self.call("mlp_pair_complete")?;
        Ok(([[2; 548]; 2], [30, 31]))
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
#[test]
fn prefix_decode_layer_keeps_both_paired_residual_fences_and_no_stage_capture() {
    let mut b = Mock {
        events: Vec::new(),
        fail: None,
        poisoned: false,
    };
    let c = coordinate(&mut b).unwrap();
    assert_eq!(
        b.events,
        [
            "validate",
            "prefix_pair_complete",
            "first_residual_pair_complete",
            "mlp_pair_complete",
            "last_residual_pair_complete"
        ]
    );
    assert_eq!(c.paired_ns, [[10, 11], [20, 21], [30, 31], [40, 41]]);
    assert_eq!(c.prefix_states, [[1; 284]; 2]);
    assert_eq!(c.mlp_states, [[2; 548]; 2]);
    assert!(!b.poisoned);
}
#[test]
fn prefix_decode_layer_every_failure_stops_before_scratch_reuse() {
    for fail in 0..5 {
        let mut b = Mock {
            events: Vec::new(),
            fail: Some(fail),
            poisoned: false,
        };
        assert!(coordinate(&mut b).is_err());
        assert_eq!(b.events.len(), fail + 1);
        assert!(b.poisoned);
    }
}
