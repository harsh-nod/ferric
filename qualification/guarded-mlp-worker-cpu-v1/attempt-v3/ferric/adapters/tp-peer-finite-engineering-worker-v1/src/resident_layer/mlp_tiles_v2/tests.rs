use super::*;
use crate::finite_mlp_tiles_comparison_wire_v1::tests as fixture;

struct Fake {
    data: [[Vec<u8>; 2]; 5],
    expected: [[Vec<u8>; 2]; 5],
    events: Vec<String>,
    fail: Option<usize>,
    bad_state: Option<(usize, usize)>,
    bad_output: Option<(usize, usize)>,
    no_write: bool,
}
impl Fake {
    fn new() -> Self {
        let data = std::array::from_fn(|i| {
            std::array::from_fn(|rank| {
                let word: &[u8] = if i == 4 {
                    &[0, 0, 128, 63]
                } else if rank == 0 {
                    &[128, 63]
                } else {
                    &[0, 64]
                };
                word.repeat(extent(Stage::ALL[i]) / word.len())
            })
        });
        Self {
            expected: data.clone(),
            data,
            events: vec![],
            fail: None,
            bad_state: None,
            bad_output: None,
            no_write: false,
        }
    }
    fn step(&mut self, event: String) -> Result<()> {
        let n = self.events.len();
        self.events.push(event);
        if self.fail == Some(n) {
            Err("injected failure".into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Fake {
    fn validate(&mut self) -> Result<()> {
        self.step("validate".into())
    }
    fn read(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>> {
        self.step(format!("read{}:{rank}", index(stage)))?;
        Ok(self.data[index(stage)][rank].clone())
    }
    fn write(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()> {
        self.step(format!("write{}:{rank}", index(stage)))?;
        assert_eq!(bytes, sentinel(stage));
        self.data[index(stage)][rank] = bytes.to_vec();
        Ok(())
    }
    fn dispatch(&mut self) -> Result<Round> {
        self.step("dispatch".into())?;
        for stage in Stage::ALL {
            for rank in 0..2 {
                assert_eq!(self.data[index(stage)][rank], sentinel(stage));
            }
        }
        if !self.no_write {
            self.data = self.expected.clone();
        }
        if let Some((stage, rank)) = self.bad_output {
            self.data[stage][rank][0] ^= 1;
        }
        let mut states: [[u32; 548]; 2] = [fixture::terminal_tiles().try_into().unwrap(); 2];
        if let Some((rank, word)) = self.bad_state {
            states[rank][word] = 0;
        }
        Ok(Round {
            final_states: states,
            dispatch_elapsed_ns: [7, 9],
        })
    }
}
fn run(fake: &mut Fake) -> Result<Comparison> {
    coordinate(fake, fixture::control().layers[0].mlp_states, [3, 5])
}

#[test]
fn all_ten_snapshots_precede_all_ten_poison_writes_and_one_paired_dispatch() {
    let mut fake = Fake::new();
    let result = run(&mut fake).unwrap();
    assert_eq!(fake.events.len(), 32);
    assert!(fake.events[1..11].iter().all(|e| e.starts_with("read")));
    assert!(fake.events[11..21].iter().all(|e| e.starts_with("write")));
    assert_eq!(fake.events[21], "dispatch");
    assert_eq!(result.tiles_queue_host_ns, [7, 9]);
    assert_eq!(result.finite_queue_host_ns, [3, 5]);
    for i in 0..5 {
        for rank in 0..2 {
            assert_eq!(
                result.equality[i][rank].sha256,
                <[u8; 32]>::from(Sha256::digest(&fake.expected[i][rank]))
            );
        }
    }
}
#[test]
fn every_backend_failure_stops_without_reading_later_outputs_or_retrying() {
    for fail in 0..32 {
        let mut fake = Fake::new();
        fake.fail = Some(fail);
        assert!(run(&mut fake).is_err());
        assert_eq!(fake.events.len(), fail + 1);
    }
}
#[test]
fn either_rank_every_output_and_untouched_sentinel_refuses_parity() {
    for stage in 0..5 {
        for rank in 0..2 {
            let mut fake = Fake::new();
            fake.bad_output = Some((stage, rank));
            assert!(run(&mut fake).is_err());
        }
    }
    let mut fake = Fake::new();
    fake.no_write = true;
    assert!(run(&mut fake).is_err());
}
#[test]
fn malformed_v1_state_refuses_before_snapshot_or_dispatch_and_v2_before_readback() {
    let mut words = fixture::control().layers[0].mlp_states;
    words[1][6] = 63;
    let mut fake = Fake::new();
    assert!(coordinate(&mut fake, words, [0; 2]).is_err());
    assert!(fake.events.is_empty());
    for rank in 0..2 {
        let mut fake = Fake::new();
        fake.bad_state = Some((rank, 290));
        assert!(run(&mut fake).is_err());
        assert_eq!(fake.events.last().unwrap(), "dispatch");
    }
}
#[test]
fn bad_snapshot_length_or_nonfinite_data_prevents_all_writes() {
    for stage in 0..5 {
        for rank in 0..2 {
            for nonfinite in [false, true] {
                let mut fake = Fake::new();
                fake.data[stage][rank] = if nonfinite {
                    sentinel(Stage::ALL[stage])
                } else {
                    vec![]
                };
                assert!(run(&mut fake).is_err());
                assert!(!fake.events.iter().any(|e| e.starts_with("write")));
            }
        }
    }
}

struct LayerFake {
    events: Vec<&'static str>,
    fail: Option<usize>,
    poisoned: bool,
}
impl LayerFake {
    fn step(&mut self, event: &'static str) -> Result<()> {
        let n = self.events.len();
        self.events.push(event);
        if self.fail == Some(n) {
            Err("layer failure".into())
        } else {
            Ok(())
        }
    }
}
impl LayerBackend for LayerFake {
    fn validate(&mut self) -> Result<()> {
        self.step("validate")
    }
    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])> {
        self.step("prefix")?;
        Ok((fixture::control().layers[0].prefix_states, [1, 2]))
    }
    fn first_residual(&mut self) -> Result<[u64; 2]> {
        self.step("first")?;
        Ok([3, 4])
    }
    fn mlp(&mut self) -> Result<([[u32; 11]; 2], [u64; 2])> {
        self.step("v1")?;
        Ok((fixture::control().layers[0].mlp_states, [5, 6]))
    }
    fn final_residual(&mut self) -> Result<[u64; 2]> {
        self.step("final")?;
        Ok([7, 8])
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
impl ComparedLayer for LayerFake {
    fn compare(&mut self, states: [[u32; 11]; 2], elapsed: [u64; 2]) -> Result<Comparison> {
        self.step("v2-all-parity")?;
        coordinate(&mut Fake::new(), states, elapsed)
    }
}
#[test]
fn actual_layer_coordinator_preserves_v1_control_and_compares_before_residual() {
    let mut fake = LayerFake {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    let (finite, comparison) = coordinate_layer(&mut fake, 0).unwrap();
    assert_eq!(
        fake.events,
        [
            "validate",
            "prefix",
            "first",
            "v1",
            "v2-all-parity",
            "final"
        ]
    );
    assert_eq!(finite.mlp_states, comparison.finite_states);
    assert_eq!(finite.paired_ns[2], comparison.finite_queue_host_ns);
    assert!(!fake.poisoned);
}
#[test]
fn all_layer_failures_poison_and_no_comparison_failure_reaches_final_residual() {
    for fail in 0..6 {
        let mut fake = LayerFake {
            events: vec![],
            fail: Some(fail),
            poisoned: false,
        };
        assert!(coordinate_layer(&mut fake, 0).is_err());
        assert!(fake.poisoned);
        assert_eq!(fake.events.len(), fail + 1);
    }
    let mut fake = LayerFake {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    assert!(coordinate_layer(&mut fake, 1).is_err());
    assert!(fake.poisoned);
    assert!(fake.events.is_empty());
}
