use super::*;

fn states() -> [[u32; 11]; 2] {
    [
        [1, 0, 31, 31, 0x155, 0, 64, 64, 64, 64, 64],
        [1, 0, 31, 31, 0x2aa, 0, 64, 64, 64, 64, 64],
    ]
}

fn output(stage: Stage, rank: usize) -> Vec<u8> {
    let mut bytes = if stage == Stage::Down {
        (1.25f32 + rank as f32)
            .to_bits()
            .to_le_bytes()
            .repeat(stage.output_bytes() / 4)
    } else {
        (0x3f80u16 + rank as u16)
            .to_le_bytes()
            .repeat(stage.output_bytes() / 2)
    };
    // Bitwise parity must distinguish signed zero, not just numerical equality.
    if stage == Stage::Down {
        bytes[..4].copy_from_slice(&0x8000_0000u32.to_le_bytes());
    } else {
        bytes[..2].copy_from_slice(&0x8000u16.to_le_bytes());
    }
    bytes
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Validate,
    Read(Stage, usize),
    Write(Stage, usize),
    Dispatch(Stage),
}

struct Fake {
    expected: [[Vec<u8>; 2]; 5],
    current: [[Vec<u8>; 2]; 5],
    events: Vec<Event>,
    fail: Option<usize>,
    skip: Option<(Stage, usize)>,
    changed: Option<(Stage, usize)>,
}

impl Fake {
    fn new() -> Self {
        let expected = Stage::ALL.map(|stage| std::array::from_fn(|rank| output(stage, rank)));
        Self {
            current: expected.clone(),
            expected,
            events: vec![],
            fail: None,
            skip: None,
            changed: None,
        }
    }
    fn event(&mut self, event: Event) -> Result<()> {
        let index = self.events.len();
        self.events.push(event);
        if self.fail == Some(index) {
            Err("injected comparison failure".into())
        } else {
            Ok(())
        }
    }
    fn run(&mut self) -> Result<Comparison> {
        coordinate(
            self,
            ComparisonProfile::FiniteThenQueuedLayerZeroV1,
            states(),
            [71, 79],
        )
    }
}

impl Backend for Fake {
    fn validate(&mut self) -> Result<()> {
        self.event(Event::Validate)
    }
    fn read_output(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>> {
        self.event(Event::Read(stage, rank))?;
        Ok(self.current[stage.index()][rank].clone())
    }
    fn write_output(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()> {
        self.event(Event::Write(stage, rank))?;
        assert_eq!(bytes, sentinel(stage));
        assert!(finite_output(stage, bytes).is_err());
        self.current[stage.index()][rank] = bytes.to_vec();
        Ok(())
    }
    fn stage(&mut self, stage: Stage) -> Result<[u64; 2]> {
        self.event(Event::Dispatch(stage))?;
        let dependencies: &[Stage] = match stage {
            Stage::Norm => &[],
            Stage::Gate | Stage::Up => &[Stage::Norm],
            Stage::Activation => &[Stage::Gate, Stage::Up],
            Stage::Down => &[Stage::Activation],
        };
        for dependency in dependencies {
            assert_eq!(
                self.current[dependency.index()],
                self.expected[dependency.index()]
            );
        }
        for rank in 0..2 {
            if self.skip != Some((stage, rank)) {
                self.current[stage.index()][rank] = self.expected[stage.index()][rank].clone();
            }
            if self.changed == Some((stage, rank)) {
                // Change -0 to +0; a numerical-equality-only comparison is insufficient.
                self.current[stage.index()][rank][stage.width() - 1] &= 0x7f;
            }
        }
        Ok([stage.index() as u64 + 1, stage.index() as u64 + 11])
    }
}

#[test]
fn exact_stage_lengths_scalars_hidden_tail_and_row_grids() {
    let expected_lengths: [&[u64]; 5] = [
        &[4096, 0, 4096, 0, 4096],
        &[4096, 25_165_824, 6144],
        &[4096, 25_165_824, 6144],
        &[6144, 6144, 6144],
        &[6144, 25_165_824, 4096],
    ];
    let scalars: [&[u32]; 5] = [
        &[1, 4096, 0x358637bd, 0],
        &[1, 6144, 4096, 2, 4],
        &[1, 6144, 4096, 2, 5],
        &[1, 2],
        &[1, 4096, 6144, 2, 2],
    ];
    for (index, stage) in Stage::ALL.into_iter().enumerate() {
        let bytes = stage_bytes(stage);
        assert_eq!(bytes.len(), [352, 328, 328, 312, 328][index]);
        assert_eq!(
            stage.grid(),
            [
                [64, 1, 1],
                [393216, 1, 1],
                [393216, 1, 1],
                [6144, 1, 1],
                [262144, 1, 1]
            ][index]
        );
        for (slot, length) in expected_lengths[index].iter().enumerate() {
            assert_eq!(&bytes[slot * 16..slot * 16 + 8], &[0; 8]);
            assert_eq!(
                u64::from_le_bytes(bytes[slot * 16 + 8..slot * 16 + 16].try_into().unwrap()),
                *length
            );
        }
        let start = stage.slices() * 16;
        for (i, value) in scalars[index].iter().enumerate() {
            assert_eq!(
                u32::from_le_bytes(bytes[start + i * 4..start + i * 4 + 4].try_into().unwrap()),
                *value
            );
        }
        assert!(
            bytes[start + scalars[index].len() * 4..]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(bytes.len() - stage.hidden() as usize, 256);
    }
}

#[test]
fn closed_root_roster_uses_only_original_readonly_weights_and_owned_outputs() {
    let expected: [&[(usize, u64, Access)]; 5] = [
        &[
            (0, 8192, Access::Read),
            (0, 0, Access::Read),
            (1, 8192, Access::Read),
            (5, 0, Access::Write),
            (5, 8192, Access::Write),
        ],
        &[
            (5, 8192, Access::Read),
            (2, 50_331_648, Access::Read),
            (6, 12288, Access::Write),
        ],
        &[
            (5, 8192, Access::Read),
            (3, 50_331_648, Access::Read),
            (7, 12288, Access::Write),
        ],
        &[
            (6, 12288, Access::Read),
            (7, 12288, Access::Read),
            (8, 12288, Access::Write),
        ],
        &[
            (8, 12288, Access::Read),
            (4, 50_331_648, Access::Read),
            (9, 16384, Access::Write),
        ],
    ];
    for (stage, expected) in Stage::ALL.into_iter().zip(expected) {
        let actual = bindings(stage);
        assert_eq!(actual.len(), expected.len());
        for (slot, (actual, &(root, bytes, access))) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                *actual,
                Binding {
                    root,
                    bytes,
                    access,
                    offset: slot as u32 * 16
                }
            );
        }
    }
}

#[test]
fn all_five_outputs_both_ranks_are_captured_before_poison_then_compared_exactly() {
    let mut fake = Fake::new();
    let completion = fake.run().unwrap();
    assert_eq!(
        completion.profile,
        ComparisonProfile::FiniteThenQueuedLayerZeroV1
    );
    assert_eq!(completion.finite_states, states());
    assert_eq!(completion.finite_queue_host_ns, [71, 79]);
    assert_eq!(
        completion
            .equality
            .iter()
            .flatten()
            .map(|item| item.words)
            .sum::<usize>(),
        53_248
    );
    assert_eq!(
        completion
            .equality
            .iter()
            .flatten()
            .map(|item| item.bytes)
            .sum::<usize>(),
        122_880
    );
    assert_eq!(
        &fake.events[..11],
        std::iter::once(Event::Validate)
            .chain(
                Stage::ALL
                    .into_iter()
                    .flat_map(|stage| [Event::Read(stage, 0), Event::Read(stage, 1)])
            )
            .collect::<Vec<_>>()
    );
    for stage in Stage::ALL {
        assert_eq!(
            completion.queued_stage_host_ns[stage.index()],
            [stage.index() as u64 + 1, stage.index() as u64 + 11]
        );
        for rank in 0..2 {
            let item = &completion.equality[stage.index()][rank];
            assert_eq!(
                (item.stage, item.rank, item.words),
                (stage, rank, stage.output_bytes() / stage.width())
            );
            assert_eq!(
                item.sha256,
                <[u8; 32]>::from(Sha256::digest(&fake.expected[stage.index()][rank]))
            );
        }
    }
}

#[test]
fn stale_initial_outputs_cannot_hide_skipped_queued_writes_or_signed_zero_changes() {
    for stage in Stage::ALL {
        for rank in 0..2 {
            for missing in [false, true] {
                let mut fake = Fake::new();
                if missing {
                    fake.skip = Some((stage, rank));
                } else {
                    fake.changed = Some((stage, rank));
                }
                assert!(fake.run().is_err());
                let dispatches = fake
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Dispatch(stage) => Some(*stage),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(dispatches, Stage::ALL[..=stage.index()]);
            }
        }
    }
}

#[test]
fn every_read_write_dispatch_and_validation_error_stops_without_retry() {
    let mut success = Fake::new();
    success.run().unwrap();
    assert_eq!(success.events.len(), 36);
    for fail in 0..success.events.len() {
        let mut fake = Fake::new();
        fake.fail = Some(fail);
        assert!(fake.run().is_err());
        assert_eq!(fake.events, success.events[..=fail]);
    }
}

#[test]
fn nonfinite_or_wrong_extent_finite_baseline_never_starts_queued_execution() {
    for stage in Stage::ALL {
        for rank in 0..2 {
            for short in [false, true] {
                let mut fake = Fake::new();
                if short {
                    fake.current[stage.index()][rank].pop();
                } else {
                    fake.current[stage.index()][rank] = sentinel(stage);
                }
                assert!(fake.run().is_err());
                assert!(
                    !fake
                        .events
                        .iter()
                        .any(|event| matches!(event, Event::Write(..) | Event::Dispatch(..)))
                );
            }
        }
    }
}

#[test]
fn terminal_state_is_real_finite_contract_not_a_queued_success_placeholder() {
    terminal(&states()).unwrap();
    for rank in 0..2 {
        for index in 0..11 {
            let mut bad = states();
            if index == 4 {
                bad[rank][index] &= !3;
            } else {
                bad[rank][index] ^= 1;
            }
            assert!(terminal(&bad).is_err());
            let mut fake = Fake::new();
            assert!(
                coordinate(
                    &mut fake,
                    ComparisonProfile::FiniteThenQueuedLayerZeroV1,
                    bad,
                    [1, 1]
                )
                .is_err()
            );
            assert!(fake.events.is_empty());
        }
    }
    let mut bad = states();
    bad[0][4] |= 1 << 10;
    assert!(terminal(&bad).is_err());
    assert!(terminal(&[[0; 11]; 2]).is_err());
}

#[test]
fn comparison_profile_cannot_silently_enable_other_layers() {
    ComparisonProfile::FiniteThenQueuedLayerZeroV1
        .validate(0)
        .unwrap();
    for layer in [1, 35, 36, usize::MAX] {
        assert!(
            ComparisonProfile::FiniteThenQueuedLayerZeroV1
                .validate(layer)
                .is_err()
        );
    }
}
