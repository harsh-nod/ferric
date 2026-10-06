use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FakeBuffer {
    id: usize,
    rank: usize,
    bytes: u64,
}
impl Allocation for FakeBuffer {
    fn owner(self) -> usize {
        self.rank
    }
    fn bytes(self) -> u64 {
        self.bytes
    }
}
fn roots() -> LayerBindings<FakeBuffer> {
    let prefix = std::array::from_fn(|rank| {
        std::array::from_fn(|i| FakeBuffer {
            id: rank * 100 + i,
            rank,
            bytes: PREFIX_BYTES[i],
        })
    });
    let mlp = std::array::from_fn(|rank| {
        std::array::from_fn(|i| match i {
            5 => prefix[rank][7],
            9 => prefix[rank][13],
            _ => FakeBuffer {
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

#[test]
fn exact_retained_rows_share_only_sequential_scratch() {
    let mut rows = roots();
    rows.validate().unwrap();
    for rank in 0..2 {
        // Allocations may hold row-capacity16 while binding only a row1 view.
        rows.prefix[rank][0].bytes *= 16;
        rows.final_hidden[rank] = rows.prefix[rank][0];
    }
    rows.validate().unwrap();
}

#[test]
fn every_prefix_and_mlp_extent_and_owner_is_checked() {
    for rank in 0..2 {
        for mlp in [false, true] {
            for index in 0..if mlp { 10 } else { 14 } {
                for owner in [false, true] {
                    let mut rows = roots();
                    let root = if mlp {
                        &mut rows.mlp[rank][index]
                    } else {
                        &mut rows.prefix[rank][index]
                    };
                    if owner {
                        root.rank = 1 - rank;
                    } else {
                        root.bytes -= 1;
                    }
                    assert!(rows.validate().is_err());
                }
            }
        }
    }
}

#[test]
fn within_phase_aliases_and_wrong_dataflow_are_rejected() {
    for rank in 0..2 {
        for change in 0..6 {
            let mut rows = roots();
            match change {
                0 => rows.prefix[rank][1] = rows.prefix[rank][0],
                1 => rows.mlp[rank][1] = rows.mlp[rank][0],
                2 => rows.mlp[rank][5] = rows.prefix[rank][0],
                3 => rows.mlp[rank][9].id += 1000,
                4 => rows.final_hidden[rank] = rows.mlp[rank][0],
                _ => rows.mlp[rank][0] = rows.prefix[rank][0],
            }
            assert!(rows.validate().is_err());
        }
    }
}

#[test]
fn residual_physical_lengths_scalars_and_hidden_tail_are_exact() {
    let bytes = consumer_bytes();
    assert_eq!(bytes.len(), 424);
    for slot in 0..10 {
        assert_eq!(&bytes[slot * 16..slot * 16 + 8], &[0; 8]);
        let length = u64::from_le_bytes(bytes[slot * 16 + 8..slot * 16 + 16].try_into().unwrap());
        assert_eq!(
            length,
            if [0, 1, 8, 9].contains(&slot) {
                4096
            } else {
                0
            }
        );
    }
    assert_eq!(&bytes[160..168], &[1, 0, 0, 0, 2, 0, 0, 0]);
    assert!(bytes[168..].iter().all(|byte| *byte == 0));
}

struct Fake {
    events: Vec<&'static str>,
    fail: Option<usize>,
    poisoned: bool,
}
impl Fake {
    fn step(&mut self, name: &'static str) -> Result<()> {
        let index = self.events.len();
        self.events.push(name);
        if self.fail == Some(index) {
            Err("injected layer failure".into())
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
        self.step("prefix-completed-and-acquired")?;
        Ok(([[7; 22]; 2], [1, 2]))
    }
    fn first_residual(&mut self) -> Result<[u64; 2]> {
        self.step("first-residual-completed")?;
        Ok([3, 4])
    }
    fn compare_projections(
        &mut self,
        finite_states: [[u32; 22]; 2],
        finite_prefix_host_ns: [u64; 2],
    ) -> Result<queued_projection_v1::Comparison> {
        self.step("queued-projections-all-outputs-equal")?;
        // Control-flow fixture only; concrete comparison checks real states,
        // exact subviews and every output after every queued dispatch.
        Ok(queued_projection_v1::Comparison {
            profile:
                queued_projection_v1::ComparisonProfile::FiniteThenQueuedProjectionsLayerZeroV1,
            finite_states,
            finite_prefix_host_ns,
            queued_stage_host_ns: [[31, 32]; 4],
            equality: std::array::from_fn(|index| {
                std::array::from_fn(|rank| queued_projection_v1::OutputEquality {
                    stage: queued_projection_v1::Stage::ALL[index],
                    rank,
                    bytes: [4096, 1024, 1024, 16384][index],
                    words: [2048, 512, 512, 4096][index],
                    sha256: [0; 32],
                })
            }),
        })
    }
    fn mlp(&mut self) -> Result<([[u32; 11]; 2], [u64; 2])> {
        self.step("mlp-completed-and-acquired")?;
        Ok(([[8; 11]; 2], [5, 6]))
    }
    fn compare_mlp(
        &mut self,
        finite_states: [[u32; 11]; 2],
        finite_queue_host_ns: [u64; 2],
    ) -> Result<queued_mlp_v1::Comparison> {
        self.step("queued-mlp-all-outputs-equal")?;
        // Control-flow fixture only; native comparison separately validates
        // actual acquired states and all five output tensors.
        Ok(queued_mlp_v1::Comparison {
            profile: queued_mlp_v1::ComparisonProfile::FiniteThenQueuedLayerZeroV1,
            finite_states,
            finite_queue_host_ns,
            queued_stage_host_ns: [[21, 22]; 5],
            equality: std::array::from_fn(|index| {
                std::array::from_fn(|rank| queued_mlp_v1::OutputEquality {
                    stage: queued_mlp_v1::Stage::ALL[index],
                    rank,
                    bytes: [8192, 12288, 12288, 12288, 16384][index],
                    words: [4096, 6144, 6144, 6144, 4096][index],
                    sha256: [0; 32],
                })
            }),
        })
    }
    fn final_residual(&mut self) -> Result<[u64; 2]> {
        self.step("final-residual-completed")?;
        Ok([7, 8])
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}

#[test]
fn only_completed_dependencies_enable_the_next_phase() {
    let mut fake = Fake {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    let result = coordinate(&mut fake).unwrap();
    assert_eq!(
        fake.events,
        [
            "validate",
            "prefix-completed-and-acquired",
            "first-residual-completed",
            "mlp-completed-and-acquired",
            "final-residual-completed"
        ]
    );
    assert!(!fake.poisoned);
    assert_eq!(result.paired_ns, [[1, 2], [3, 4], [5, 6], [7, 8]]);
    assert_eq!(result.prefix_states, [[7; 22]; 2]);
    assert_eq!(result.mlp_states, [[8; 11]; 2]);
}

#[test]
fn every_phase_failure_prevents_later_operations_and_poisons_the_owner_state() {
    for index in 0..5 {
        let mut fake = Fake {
            events: vec![],
            fail: Some(index),
            poisoned: false,
        };
        assert!(coordinate(&mut fake).is_err());
        assert_eq!(fake.events.len(), index + 1);
        assert!(fake.poisoned);
    }
}

#[test]
fn distinct_comparison_retains_finite_states_and_waits_before_final_residual() {
    let mut fake = Fake {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    let result = coordinate_comparison(
        &mut fake,
        queued_mlp_v1::ComparisonProfile::FiniteThenQueuedLayerZeroV1,
        0,
    )
    .unwrap();
    assert_eq!(
        fake.events,
        [
            "validate",
            "prefix-completed-and-acquired",
            "first-residual-completed",
            "mlp-completed-and-acquired",
            "queued-mlp-all-outputs-equal",
            "final-residual-completed"
        ]
    );
    assert_eq!(result.finite.mlp_states, [[8; 11]; 2]);
    assert_eq!(result.queued.finite_states, result.finite.mlp_states);
    assert_eq!(result.finite.paired_ns, [[1, 2], [3, 4], [5, 6], [7, 8]]);
    assert_eq!(result.queued.finite_queue_host_ns, [5, 6]);
    assert_eq!(result.queued.queued_stage_host_ns, [[21, 22]; 5]);
    assert!(!fake.poisoned);
}

#[test]
fn comparison_failure_or_wrong_scope_is_terminal_and_cannot_fall_back() {
    for index in 0..6 {
        let mut fake = Fake {
            events: vec![],
            fail: Some(index),
            poisoned: false,
        };
        assert!(
            coordinate_comparison(
                &mut fake,
                queued_mlp_v1::ComparisonProfile::FiniteThenQueuedLayerZeroV1,
                0
            )
            .is_err()
        );
        assert_eq!(fake.events.len(), index + 1);
        assert!(fake.poisoned);
        if index == 4 {
            assert!(!fake.events.contains(&"final-residual-completed"));
        }
    }
    let mut fake = Fake {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    assert!(
        coordinate_comparison(
            &mut fake,
            queued_mlp_v1::ComparisonProfile::FiniteThenQueuedLayerZeroV1,
            1
        )
        .is_err()
    );
    assert!(fake.events.is_empty());
    assert!(fake.poisoned);
}

#[test]
fn projection_comparison_waits_for_prefix_acquire_and_precedes_first_residual() {
    let mut fake = Fake {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    let result = coordinate_projection_comparison(
        &mut fake,
        queued_projection_v1::ComparisonProfile::FiniteThenQueuedProjectionsLayerZeroV1,
        0,
    )
    .unwrap();
    assert_eq!(
        fake.events,
        [
            "validate",
            "prefix-completed-and-acquired",
            "queued-projections-all-outputs-equal",
            "first-residual-completed",
            "mlp-completed-and-acquired",
            "final-residual-completed"
        ]
    );
    assert_eq!(result.finite.prefix_states, [[7; 22]; 2]);
    assert_eq!(result.queued.finite_states, result.finite.prefix_states);
    assert_eq!(result.queued.finite_prefix_host_ns, [1, 2]);
    assert_eq!(result.queued.queued_stage_host_ns, [[31, 32]; 4]);
    assert_eq!(result.finite.paired_ns, [[1, 2], [3, 4], [5, 6], [7, 8]]);
    assert!(!fake.poisoned);
    assert!(!fake.events.contains(&"queued-mlp-all-outputs-equal"));
}

#[test]
fn projection_mismatch_or_scope_failure_poisons_without_residual_or_fallback() {
    let profile = queued_projection_v1::ComparisonProfile::FiniteThenQueuedProjectionsLayerZeroV1;
    for index in 0..6 {
        let mut fake = Fake {
            events: vec![],
            fail: Some(index),
            poisoned: false,
        };
        assert!(coordinate_projection_comparison(&mut fake, profile, 0).is_err());
        assert_eq!(fake.events.len(), index + 1);
        assert!(fake.poisoned);
        if index == 2 {
            assert!(!fake.events.contains(&"first-residual-completed"));
            assert!(!fake.events.contains(&"mlp-completed-and-acquired"));
        }
    }
    let mut fake = Fake {
        events: vec![],
        fail: None,
        poisoned: false,
    };
    assert!(coordinate_projection_comparison(&mut fake, profile, 1).is_err());
    assert!(fake.events.is_empty() && fake.poisoned);
}
