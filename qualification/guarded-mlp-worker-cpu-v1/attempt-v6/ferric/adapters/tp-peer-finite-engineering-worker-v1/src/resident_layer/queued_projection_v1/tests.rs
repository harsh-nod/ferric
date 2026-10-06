use super::*;

const PROFILE: ComparisonProfile = ComparisonProfile::FiniteThenQueuedProjectionsLayerZeroV1;
fn states() -> [[u32; 22]; 2] {
    let mut row = [64; 22];
    row[..6].copy_from_slice(&[1, 0, 65535, 65535, 0x55555555, 0]);
    [row; 2]
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn exact_gemv_carrier_lengths_scalars_grid_and_zero_padding() {
    let lengths = [
        [4096, 8388608, 2048],
        [4096, 2097152, 512],
        [4096, 2097152, 512],
        [2048, 8388608, 4096],
    ];
    let scalars = [
        [1, 2048, 4096, 2, 1],
        [1, 512, 4096, 2, 2],
        [1, 512, 4096, 2, 3],
        [1, 4096, 2048, 2, 1],
    ];
    for (index, stage) in Stage::ALL.into_iter().enumerate() {
        let bytes = stage_bytes(stage);
        assert_eq!(bytes.len(), 328);
        for slot in 0..3 {
            assert_eq!(u64_at(&bytes, slot * 16), 0);
            assert_eq!(u64_at(&bytes, slot * 16 + 8), lengths[index][slot]);
        }
        for field in 0..5 {
            assert_eq!(u32_at(&bytes, 48 + field * 4), scalars[index][field]);
        }
        assert!(bytes[68..].iter().all(|v| *v == 0));
        assert_eq!(
            stage.grid(),
            [[131072, 1, 1], [32768, 1, 1], [32768, 1, 1], [262144, 1, 1]][index]
        );
    }
}

#[test]
fn packed_qkv_subviews_cover_exact_rows_and_never_write_query_or_kv() {
    let weights = [(0, 16777216), (16777216, 4194304), (20971520, 4194304)];
    let outputs = [(0, 4096), (4096, 1024), (5120, 1024)];
    for (index, stage) in Stage::ALL.into_iter().enumerate() {
        let plan = bindings(stage);
        assert_eq!(plan.map(|v| v.kernarg_offset), [0, 16, 32]);
        assert_eq!(
            plan.map(|v| v.access),
            [Access::Read, Access::Read, Access::Write]
        );
        if index < 3 {
            assert_eq!(
                plan[0].view,
                View {
                    root: 7,
                    offset: 0,
                    bytes: 8192
                }
            );
            assert_eq!(
                plan[1].view,
                View {
                    root: 2,
                    offset: weights[index].0,
                    bytes: weights[index].1
                }
            );
            assert_eq!(
                plan[2].view,
                View {
                    root: 8,
                    offset: outputs[index].0,
                    bytes: outputs[index].1
                }
            );
        } else {
            assert_eq!(
                plan[0].view,
                View {
                    root: 12,
                    offset: 0,
                    bytes: 4096
                }
            );
            assert_eq!(
                plan[1].view,
                View {
                    root: 6,
                    offset: 0,
                    bytes: 16777216
                }
            );
            assert_eq!(
                plan[2].view,
                View {
                    root: 13,
                    offset: 0,
                    bytes: 16384
                }
            );
        }
        assert!(![9, 10, 11].contains(&plan[2].view.root));
    }
    assert_eq!(
        weights.last().unwrap().0 + weights.last().unwrap().1,
        25165824
    );
    assert_eq!(outputs.last().unwrap().0 + outputs.last().unwrap().1, 6144);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Validate,
    Read(Stage, usize),
    Write(Stage, usize),
    Dispatch(Stage),
}
struct Fake {
    qkv: [Vec<u8>; 2],
    partial: [Vec<u8>; 2],
    original_qkv: [Vec<u8>; 2],
    original_partial: [Vec<u8>; 2],
    events: Vec<Event>,
    fail: Option<usize>,
    skip: Option<Stage>,
    corruption: Option<(Stage, Stage, usize)>,
    negative_zero: bool,
}
impl Fake {
    fn new() -> Self {
        let mut qkv = std::array::from_fn(|_| vec![0; 6144]);
        let mut partial = std::array::from_fn(|_| vec![0; 16384]);
        for rank in 0..2 {
            for stage in Stage::ALL {
                let view = stage.output();
                let target = if stage == Stage::Output {
                    &mut partial[rank]
                } else {
                    &mut qkv[rank]
                };
                let word = if stage == Stage::Output {
                    (0x3f800001u32 + rank as u32).to_le_bytes().to_vec()
                } else {
                    (0x3f80u16 + (stage.index() * 128 + rank) as u16)
                        .to_le_bytes()
                        .to_vec()
                };
                target[view.offset as usize..(view.offset + view.bytes) as usize]
                    .copy_from_slice(&word.repeat((view.bytes / stage.width()) as usize));
            }
        }
        Self {
            original_qkv: qkv.clone(),
            original_partial: partial.clone(),
            qkv,
            partial,
            events: vec![],
            fail: None,
            skip: None,
            corruption: None,
            negative_zero: false,
        }
    }
    fn step(&mut self, event: Event) -> Result<()> {
        let index = self.events.len();
        self.events.push(event);
        if self.fail == Some(index) {
            Err("injected comparison operation failure".into())
        } else {
            Ok(())
        }
    }
    fn slice(&self, stage: Stage, rank: usize) -> &[u8] {
        let view = stage.output();
        let root = if stage == Stage::Output {
            &self.partial[rank]
        } else {
            &self.qkv[rank]
        };
        &root[view.offset as usize..(view.offset + view.bytes) as usize]
    }
    fn overwrite(&mut self, stage: Stage, rank: usize, bytes: &[u8]) {
        let view = stage.output();
        let root = if stage == Stage::Output {
            &mut self.partial[rank]
        } else {
            &mut self.qkv[rank]
        };
        root[view.offset as usize..(view.offset + view.bytes) as usize].copy_from_slice(bytes);
    }
}
impl Backend for Fake {
    fn validate(&mut self) -> Result<()> {
        self.step(Event::Validate)
    }
    fn read(&mut self, stage: Stage, rank: usize) -> Result<Vec<u8>> {
        self.step(Event::Read(stage, rank))?;
        Ok(self.slice(stage, rank).to_vec())
    }
    fn write(&mut self, stage: Stage, rank: usize, bytes: &[u8]) -> Result<()> {
        self.step(Event::Write(stage, rank))?;
        assert_eq!(bytes, sentinel(stage));
        self.overwrite(stage, rank, bytes);
        Ok(())
    }
    fn stage(&mut self, stage: Stage) -> Result<[u64; 2]> {
        self.step(Event::Dispatch(stage))?;
        if self.skip != Some(stage) {
            for rank in 0..2 {
                let view = stage.output();
                let root = if stage == Stage::Output {
                    &self.original_partial[rank]
                } else {
                    &self.original_qkv[rank]
                };
                let bytes =
                    root[view.offset as usize..(view.offset + view.bytes) as usize].to_vec();
                self.overwrite(stage, rank, &bytes);
            }
        }
        if let Some((at, victim, rank)) = self.corruption {
            if at == stage {
                let mut bytes = self.slice(victim, rank).to_vec();
                bytes[0] ^= 1;
                self.overwrite(victim, rank, &bytes);
            }
        }
        if self.negative_zero && stage == Stage::Query {
            let mut bytes = self.slice(stage, 0).to_vec();
            bytes[..2].copy_from_slice(&0x8000u16.to_le_bytes());
            self.overwrite(stage, 0, &bytes);
        }
        Ok([101 + stage.index() as u64, 201 + stage.index() as u64])
    }
}

#[test]
fn all_snapshots_precede_writes_and_all_views_rechecked_after_every_dispatch() {
    let mut fake = Fake::new();
    let result = coordinate(&mut fake, PROFILE, states(), [7, 8]).unwrap();
    assert_eq!(fake.events.len(), 53);
    assert_eq!(fake.events[0], Event::Validate);
    assert_eq!(
        fake.events[1..9],
        Stage::ALL
            .into_iter()
            .flat_map(|stage| [Event::Read(stage, 0), Event::Read(stage, 1)])
            .collect::<Vec<_>>()
    );
    for (index, stage) in Stage::ALL.into_iter().enumerate() {
        let offset = 9 + index * 11;
        assert_eq!(
            fake.events[offset..offset + 3],
            [
                Event::Write(stage, 0),
                Event::Write(stage, 1),
                Event::Dispatch(stage)
            ]
        );
        assert_eq!(fake.events[offset + 3..offset + 11], fake.events[1..9]);
    }
    assert_eq!(result.profile, PROFILE);
    assert_eq!(result.finite_states, states());
    assert_eq!(result.finite_prefix_host_ns, [7, 8]);
    assert_eq!(
        result.queued_stage_host_ns,
        [[101, 201], [102, 202], [103, 203], [104, 204]]
    );
    for row in result.equality.iter().flatten() {
        assert_eq!(
            row.sha256,
            <[u8; 32]>::from(Sha256::digest(fake.slice(row.stage, row.rank)))
        );
        assert_eq!(
            row.bytes,
            Stage::ALL[row.stage.index()].output().bytes as usize
        );
    }
    assert_eq!(
        result
            .equality
            .iter()
            .flatten()
            .map(|v| v.bytes)
            .sum::<usize>(),
        45056
    );
    assert_eq!(
        result
            .equality
            .iter()
            .flatten()
            .map(|v| v.words)
            .sum::<usize>(),
        14336
    );
    assert_eq!(fake.qkv, fake.original_qkv);
    assert_eq!(fake.partial, fake.original_partial);
}

#[test]
fn invalid_prefix_state_never_touches_backend() {
    for rank in 0..2 {
        for change in 0..8 {
            let mut state = states();
            match change {
                0 => state[rank][0] = 2,
                1 => state[rank][1] = 1,
                2 => state[rank][2] = 65534,
                3 => state[rank][3] = 65534,
                4 => state[rank][4] &= !3,
                5 => state[rank][4] |= 3 << 30,
                6 => state[rank][5] = 1,
                _ => state[rank][21] = 63,
            }
            let mut fake = Fake::new();
            assert!(coordinate(&mut fake, PROFILE, state, [1, 2]).is_err());
            assert!(fake.events.is_empty());
        }
    }
    let mut mixed = states();
    mixed[1][4] = 0xaaaaaaaa;
    terminal(&mixed).unwrap();
}

#[test]
fn nonfinite_finite_snapshot_refuses_before_first_sentinel() {
    for stage in Stage::ALL {
        for rank in 0..2 {
            let mut fake = Fake::new();
            fake.overwrite(stage, rank, &sentinel(stage));
            assert!(coordinate(&mut fake, PROFILE, states(), [1, 2]).is_err());
            assert!(
                !fake
                    .events
                    .iter()
                    .any(|e| matches!(e, Event::Write(..) | Event::Dispatch(..)))
            );
        }
    }
}

#[test]
fn every_read_write_and_dispatch_failure_stops_without_a_later_operation() {
    for index in 0..53 {
        let mut fake = Fake::new();
        fake.fail = Some(index);
        assert!(coordinate(&mut fake, PROFILE, states(), [1, 2]).is_err());
        assert_eq!(fake.events.len(), index + 1);
    }
}

#[test]
fn missing_output_write_and_signed_zero_difference_are_not_tolerated() {
    for stage in Stage::ALL {
        let mut fake = Fake::new();
        fake.skip = Some(stage);
        assert!(coordinate(&mut fake, PROFILE, states(), [1, 2]).is_err());
        assert_eq!(
            fake.events
                .iter()
                .filter(|e| matches!(e, Event::Dispatch(_)))
                .count(),
            stage.index() + 1
        );
    }
    let mut fake = Fake::new();
    fake.qkv[0][..2].fill(0);
    fake.original_qkv[0][..2].fill(0);
    fake.negative_zero = true;
    assert!(coordinate(&mut fake, PROFILE, states(), [1, 2]).is_err());
}

#[test]
fn adjacent_qkv_or_partial_damage_cannot_be_hidden_by_later_recomputation() {
    for stage in Stage::ALL {
        for victim in Stage::ALL {
            for rank in 0..2 {
                let mut fake = Fake::new();
                fake.corruption = Some((stage, victim, rank));
                assert!(coordinate(&mut fake, PROFILE, states(), [1, 2]).is_err());
                assert_eq!(
                    fake.events
                        .iter()
                        .filter(|e| matches!(e, Event::Dispatch(_)))
                        .count(),
                    stage.index() + 1
                );
                assert_eq!(
                    fake.events
                        .iter()
                        .filter(|e| matches!(e, Event::Write(..)))
                        .count(),
                    2 * (stage.index() + 1)
                );
            }
        }
    }
}
