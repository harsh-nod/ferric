use super::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Token {
    id: usize,
    rank: usize,
    bytes: u64,
}
impl super::super::super::Allocation for Token {
    fn owner(self) -> usize {
        self.rank
    }
    fn bytes(self) -> u64 {
        self.bytes
    }
}
fn roots() -> LayerBindings<Token> {
    let prefix = core::array::from_fn(|rank| {
        core::array::from_fn(|i| Token {
            id: rank * 100 + i,
            rank,
            bytes: super::super::super::PREFIX_BYTES[i],
        })
    });
    let mut mlp = core::array::from_fn(|rank| {
        core::array::from_fn(|i| Token {
            id: rank * 100 + 30 + i,
            rank,
            bytes: super::super::super::MLP_BYTES[i],
        })
    });
    for rank in 0..2 {
        mlp[rank][5] = prefix[rank][7];
        mlp[rank][9] = prefix[rank][13];
    }
    LayerBindings {
        prefix,
        mlp,
        final_hidden: [prefix[0][0], prefix[1][0]],
    }
}
fn down() -> [Token; 2] {
    core::array::from_fn(|rank| Token {
        id: 300 + rank,
        rank,
        bytes: 16384,
    })
}
#[test]
fn ordered_down_roots_keep_old_alias_contract_and_replace_only_output() {
    let r = roots();
    r.validate().unwrap();
    let d = down();
    let selected = selected_mlp(&r, d).unwrap();
    for rank in 0..2 {
        assert_eq!(&selected[rank][..9], &r.mlp[rank][..9]);
        assert_eq!(selected[rank][9], d[rank]);
        assert_eq!(r.mlp[rank][9], r.prefix[rank][13]);
    }
}
#[test]
fn ordered_rejects_global_partial_fallback_and_every_old_root_alias() {
    let r = roots();
    for root in r.prefix.iter().flatten().chain(r.mlp.iter().flatten()) {
        let mut d = down();
        d[root.rank] = *root;
        assert!(validate_down(&r, d).is_err());
    }
    let mut d = down();
    d.swap(0, 1);
    assert!(validate_down(&r, d).is_err());
    let mut d = down();
    d[0].bytes = 16383;
    assert!(validate_down(&r, d).is_err());
}
struct Alloc {
    calls: Vec<usize>,
    drift: bool,
}
impl ScratchAllocator for Alloc {
    type Buffer = Token;
    fn preflight(&mut self, extra: &[usize]) -> Result<Vec<usize>> {
        if self.drift {
            return Ok(vec![715, 710]);
        }
        match (extra, self.calls.len()) {
            ([1, 1], 0) => Ok(vec![714, 710]),
            ([0, 0], 2) => Ok(vec![715, 711]),
            _ => Err("unexpected allocation/reuse".into()),
        }
    }
    fn allocate(&mut self, rank: usize) -> Result<Token> {
        self.calls.push(rank);
        Ok(down()[rank])
    }
}
#[test]
fn ordered_setup_allocates_fixed_pair_once_and_refuses_census_drift() {
    let layers = (0..36).map(|_| roots()).collect::<Vec<_>>();
    let mut a = Alloc {
        calls: vec![],
        drift: false,
    };
    let d = allocate(&mut a, &layers).unwrap();
    for _ in 0..(36 * 2303) {
        selected_mlp(&layers[0], d).unwrap();
    }
    assert_eq!(a.calls, [0, 1]);
    assert!(allocate(&mut a, &layers).is_err());
    let mut a = Alloc {
        calls: vec![],
        drift: true,
    };
    assert!(allocate(&mut a, &layers).is_err());
    assert!(a.calls.is_empty());
    let mut a = Alloc {
        calls: vec![],
        drift: false,
    };
    assert!(allocate(&mut a, &layers[..35]).is_err());
    assert!(a.calls.is_empty());
}
struct Trace {
    calls: Vec<&'static str>,
    fail: Option<usize>,
    poison: bool,
}
impl Trace {
    fn hit(&mut self, s: &'static str) -> Result<()> {
        if self.poison {
            return Err("terminal".into());
        }
        self.calls.push(s);
        if self.fail == Some(self.calls.len()) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Trace {
    fn validate(&mut self) -> Result<()> {
        self.hit("validate")
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])> {
        self.hit("prefix-both")?;
        Ok(([[1; 284]; 2], [1, 2]))
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        assert!(!first);
        self.hit("final-both")?;
        Ok([4, 5])
    }
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])> {
        panic!("separate MLP forbidden")
    }
    fn poison(&mut self) {
        self.poison = true;
    }
}
impl CompoundBackend for Trace {
    fn compound(&mut self) -> Result<([[u32; 548]; 2], u64)> {
        self.hit("compound-both")?;
        Ok(([[2; 548]; 2], 3))
    }
}
#[test]
fn ordered_trace_has_global_prefix_compound_final_boundaries_and_one_duration() {
    let mut t = Trace {
        calls: vec![],
        fail: None,
        poison: false,
    };
    let value = coordinate_ordered(&mut t).unwrap();
    assert_eq!(
        t.calls,
        ["validate", "prefix-both", "compound-both", "final-both"]
    );
    assert_eq!(
        value.timing,
        Timing::Ordered {
            prefix_ns: [1, 2],
            segment_host_ns: 3,
            final_residual_ns: [4, 5]
        }
    );
    assert!(value.timing.paired().is_err());
}
#[test]
fn ordered_any_boundary_failure_poison_prevents_later_or_repeated_work() {
    for fail in 1..=4 {
        let mut t = Trace {
            calls: vec![],
            fail: Some(fail),
            poison: false,
        };
        assert!(coordinate_ordered(&mut t).is_err());
        assert!(t.poison);
        assert_eq!(t.calls.len(), fail);
        assert!(coordinate_ordered(&mut t).is_err());
        assert_eq!(t.calls.len(), fail);
    }
}
