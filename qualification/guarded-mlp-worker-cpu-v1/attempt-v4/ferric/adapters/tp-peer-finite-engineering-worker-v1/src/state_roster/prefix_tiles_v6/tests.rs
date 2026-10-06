use super::*;
struct Fake {
    calls: usize,
    fail: Option<usize>,
    counts: [usize; 2],
}
impl Fake {
    fn step(&mut self) -> Result<()> {
        let n = self.calls;
        self.calls += 1;
        if self.fail == Some(n) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
impl Allocator for Fake {
    type Prefix = (u8, usize);
    type Mlp = (u8, usize);
    fn preflight(&mut self, n: &[usize]) -> Result<Vec<usize>> {
        self.step()?;
        assert!(n == [144, 144] || n == [0, 0]);
        Ok(self.counts.to_vec())
    }
    fn prefix(&mut self, r: usize) -> Result<Self::Prefix> {
        self.step()?;
        self.counts[r] += 1;
        Ok((6, r))
    }
    fn mlp(&mut self, r: usize) -> Result<Self::Mlp> {
        self.step()?;
        self.counts[r] += 1;
        Ok((2, r))
    }
}
fn fake() -> Fake {
    Fake {
        calls: 0,
        fail: None,
        counts: [570, 566],
    }
}
#[test]
fn prefix_layer_reservations_keep_full_logical_catalog_but_distinct_native_types() {
    let mut f = fake();
    let (states, ids) = allocate(&mut f, [7; 32]).unwrap();
    assert_eq!(states.len(), 72);
    assert_eq!(ids.len(), 288);
    assert_eq!(f.counts, COUNTS);
    for (i, row) in ids.iter().enumerate() {
        assert_eq!(row.reservation_id, i as u64 + 1);
        assert_eq!(row.model, [7; 32]);
        assert_eq!(row.forward, (i / 144) as u32);
        assert_eq!(row.layer, ((i % 144) / 4) as u32);
        assert_eq!(row.rank, (i % 2) as u32);
        assert_eq!(
            row.kind,
            if i % 4 < 2 {
                WorkerKind::PrefixTilesV6
            } else {
                WorkerKind::MlpTilesV2
            }
        );
    }
    assert_eq!(states[71].prefix, [(6, 0), (6, 1)]);
    assert_eq!(states[71].mlp, [(2, 0), (2, 1)]);
}
#[test]
fn prefix_layer_allocation_stops_at_every_cutoff_and_refuses_wrong_census() {
    for fail in 0..290 {
        let mut f = fake();
        f.fail = Some(fail);
        assert!(allocate(&mut f, [7; 32]).is_err());
        assert_eq!(f.calls, fail + 1);
    }
    let mut f = fake();
    f.counts[0] += 1;
    assert!(allocate(&mut f, [7; 32]).is_err());
    assert_eq!(f.calls, 1);
    let mut f = fake();
    assert!(allocate(&mut f, [0; 32]).is_err());
    assert_eq!(f.calls, 0);
}
#[test]
fn prefix_layer_terminal_decoder_covers_all284_words_without_full_workgroup_claim() {
    let mut good = [0; 284];
    good[..4].copy_from_slice(&[1, 0, 0, 31]);
    good[4..9].copy_from_slice(&[1, 48, 1, 16, 64]);
    good[9..14].copy_from_slice(&[1, 48, 1, 16, 64]);
    good[14..19].copy_from_slice(&[u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]);
    good[19..24].copy_from_slice(&[u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]);
    good[24..154].fill(1);
    good[154..].fill(64);
    assert!(terminal(&good));
    let mut other = good;
    other[24..154].fill(64);
    assert!(terminal(&other));
    for i in 0..284 {
        let mut bad = good;
        bad[i] = if (24..154).contains(&i) {
            65
        } else {
            bad[i] ^ 1
        };
        assert!(!terminal(&bad), "word {i}");
    }
}
#[test]
fn prefix_layer_phase_gate_is_one_shot_and_wrong_phase_is_permanent() {
    let steps = [
        Step::Fresh,
        Step::Prefix,
        Step::PrefixPending,
        Step::FirstResidual,
        Step::FirstPending,
        Step::Mlp,
        Step::MlpPending,
        Step::LastResidual,
        Step::LastPending,
        Step::Complete,
    ];
    let mut actual = Step::Fresh;
    for pair in steps.windows(2) {
        actual.advance(pair[0], pair[1]).unwrap();
    }
    assert!(actual.advance(Step::Fresh, Step::Prefix).is_err());
    assert_eq!(actual, Step::Terminal);
    for phase in steps {
        let mut actual = phase;
        assert!(actual.advance(Step::Terminal, Step::Fresh).is_err());
        assert_eq!(actual, Step::Terminal);
    }
}
