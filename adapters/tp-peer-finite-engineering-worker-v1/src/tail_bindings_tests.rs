use super::*;

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn tail_plans_have_zero_pointers_padding_and_hidden_arguments() {
    for kind in TailKind::ALL {
        let plan = plan(kind);
        assert_eq!(plan.bytes.len(), kind.explicit_bytes() as usize + 256);
        assert!(
            plan.bytes[kind.explicit_bytes() as usize..]
                .iter()
                .all(|&b| b == 0)
        );
        for slice in &plan.slices {
            assert_eq!(u64_at(&plan.bytes, slice.offset as usize), 0);
            assert_eq!(
                u64_at(&plan.bytes, slice.offset as usize + 8),
                slice.elements
            );
            assert!(slice.elements * slice.width <= slice.root.bytes());
        }
    }
    assert_eq!(&plan(TailKind::Head).bytes[68..72], &[0; 4]);
    assert_eq!(&plan(TailKind::Embedding).bytes[52..56], &[0; 4]);
}

#[test]
fn tail_plans_preserve_queued_arithmetic_and_distinct_transposed_head() {
    let norm = plan(TailKind::FinalNorm);
    assert_eq!(
        norm.slices
            .iter()
            .map(|s| (s.root, s.elements, s.access))
            .collect::<Vec<_>>(),
        [
            (Root::Hidden0, 4096, Access::Read),
            (Root::Empty, 0, Access::Read),
            (Root::FinalNorm, 4096, Access::Read),
            (Root::Empty, 0, Access::Write),
            (Root::Normalized, 4096, Access::Write)
        ]
    );
    assert_eq!(
        [80, 84, 88, 92].map(|at| u32_at(&norm.bytes, at)),
        [1, 4096, EPSILON_BITS, 0]
    );
    let head = plan(TailKind::Head);
    assert_eq!(
        head.slices.iter().map(|s| s.root).collect::<Vec<_>>(),
        [Root::Normalized, Root::Head, Root::Logits]
    );
    assert_eq!(
        [48, 52, 56, 60, 64].map(|at| u32_at(&head.bytes, at)),
        [1, VOCAB, 4096, 2, 6]
    );
    assert_eq!(head.slices[1].elements * head.slices[1].width, HEAD_BYTES);
    assert_eq!(head.slices[2].elements, u64::from(VOCAB));
    let copy = plan(TailKind::Copy);
    assert_eq!(
        copy.slices
            .iter()
            .map(|s| (s.root, s.access))
            .collect::<Vec<_>>(),
        [
            (Root::Hidden0, Access::Read),
            (Root::Hidden1, Access::Write)
        ]
    );
    let argmax = plan(TailKind::Argmax);
    assert_eq!(argmax.slices[1].elements, 1);
    assert_eq!(argmax.slices[1].width, 4);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Token {
    id: u64,
    rank: usize,
    bytes: u64,
}
fn roots() -> [Token; 10] {
    Root::ALL.map(|r| Token {
        id: r.index() as u64 + 1,
        rank: r.rank(),
        bytes: r.bytes(),
    })
}

#[test]
fn tail_actual_root_fact_checks_reject_every_extent_owner_or_alias_mutation() {
    let check = |roots: &[Token; 10]| validate_roots(roots, |b| (b.rank, b.bytes));
    check(&roots()).unwrap();
    for index in 0..10 {
        let mut bad = roots();
        bad[index].bytes += 2;
        assert!(check(&bad).is_err());
        let mut bad = roots();
        bad[index].rank = 1 - bad[index].rank;
        assert!(check(&bad).is_err());
    }
    let mut bad = roots();
    bad[Root::Head.index()] = bad[Root::Embedding.index()];
    assert!(check(&bad).is_err());
    let mut bad = roots();
    bad[Root::Normalized.index()] = bad[Root::Hidden0.index()];
    assert!(check(&bad).is_err());
    let mut bad = roots();
    bad[Root::Empty.index()].bytes = 0;
    assert!(check(&bad).is_err());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Write(u32),
    Dispatch(TailKind),
    Read,
}
struct Fake {
    events: Vec<Event>,
    fail: Option<usize>,
    choice: Vec<u8>,
}
impl Fake {
    fn event(&mut self, event: Event) -> Result<()> {
        let index = self.events.len();
        self.events.push(event);
        if self.fail == Some(index) {
            Err("injected operation failure".into())
        } else {
            Ok(())
        }
    }
}
impl Operations for Fake {
    fn write_token(&mut self, token: u32) -> Result<()> {
        self.event(Event::Write(token))
    }
    fn dispatch(&mut self, kind: TailKind) -> Result<u64> {
        self.event(Event::Dispatch(kind))?;
        Ok(100 + kind.index() as u64)
    }
    fn read_choice(&mut self) -> Result<Vec<u8>> {
        self.event(Event::Read)?;
        Ok(self.choice.clone())
    }
}
fn fake() -> Fake {
    Fake {
        events: vec![],
        fail: None,
        choice: 151935u32.to_le_bytes().to_vec(),
    }
}

#[test]
fn tail_begin_and_finish_are_fixed_order_and_return_only_checked_choice() {
    let mut ops = fake();
    assert_eq!(begin(&mut ops, 42).unwrap(), [100, 101]);
    assert_eq!(
        ops.events,
        [
            Event::Write(42),
            Event::Dispatch(TailKind::Embedding),
            Event::Dispatch(TailKind::Copy)
        ]
    );
    let mut ops = fake();
    let result = finish(&mut ops).unwrap();
    assert_eq!(result.token, 151935);
    assert_eq!(result.timing_ns, [102, 103, 104]);
    assert_eq!(
        ops.events,
        [
            Event::Dispatch(TailKind::FinalNorm),
            Event::Dispatch(TailKind::Head),
            Event::Dispatch(TailKind::Argmax),
            Event::Read
        ]
    );
}

#[test]
fn tail_first_failure_stops_without_retry_dependent_dispatch_or_read() {
    for index in 0..3 {
        let mut ops = fake();
        ops.fail = Some(index);
        assert!(begin(&mut ops, 42).is_err());
        assert_eq!(ops.events.len(), index + 1);
    }
    for index in 0..4 {
        let mut ops = fake();
        ops.fail = Some(index);
        assert!(finish(&mut ops).is_err());
        assert_eq!(ops.events.len(), index + 1);
    }
    let mut ops = fake();
    assert!(begin(&mut ops, VOCAB).is_err());
    assert!(ops.events.is_empty());
    for choice in [
        vec![],
        vec![0; 3],
        vec![0; 5],
        VOCAB.to_le_bytes().to_vec(),
        u32::MAX.to_le_bytes().to_vec(),
    ] {
        let mut ops = fake();
        ops.choice = choice;
        assert!(finish(&mut ops).is_err());
    }
}
