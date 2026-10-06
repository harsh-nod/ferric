use super::*;

struct Fake {
    calls: Vec<String>,
    fail_at: Option<usize>,
    output: u32,
    poisoned: bool,
}

impl Fake {
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            fail_at: None,
            output: 42,
            poisoned: false,
        }
    }
    fn call(&mut self, name: impl Into<String>) -> Result<()> {
        self.calls.push(name.into());
        if self.fail_at == Some(self.calls.len() - 1) {
            Err("injected failure".into())
        } else {
            Ok(())
        }
    }
}

impl Backend for Fake {
    fn upload_metadata(&mut self, _: &ForwardInput) -> Result<()> {
        self.call("metadata")
    }
    fn embedding(&mut self, _: u32) -> Result<[u64; 2]> {
        self.call("embedding")?;
        Ok([1, 2])
    }
    fn begin_states(&mut self, _: &ForwardInput) -> Result<()> {
        self.call("begin")
    }
    fn layer(&mut self, layer: usize) -> Result<LayerCompletion> {
        self.call(format!("layer{layer}"))?;
        Ok(LayerCompletion {
            prefix_states: [[0; 22]; 2],
            mlp_states: [[0; 11]; 2],
            paired_ns: [[0; 2]; 4],
        })
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.call("tail")?;
        Ok((self.output, [3, 4, 5]))
    }
    fn idle_fence(&mut self) -> Result<()> {
        self.call("fence")
    }
    fn commit_states(&mut self) -> Result<()> {
        self.call("commit")
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}

fn input(position: u32) -> ForwardInput {
    let mut cache_metadata = [0; PAGES + 1];
    cache_metadata[0] = position;
    for (index, word) in cache_metadata[1..].iter_mut().enumerate() {
        *word = index as u32;
    }
    ForwardInput {
        registration: [7; 32],
        generation: u64::from(position) + 1,
        token: 13,
        cache_metadata,
        rotary_bits: [0; 128],
    }
}

fn sequence() -> Sequence {
    Sequence::new([7; 32], InputMode::TeacherForced).unwrap()
}

#[test]
fn two_forwards_complete_every_stage_before_token_publication() {
    let mut sequence = sequence();
    let mut fake = Fake::new();
    for position in 0..2 {
        let result = sequence.run(&mut fake, &input(position)).unwrap();
        assert_eq!(result.position, position);
        assert_eq!(result.generation, u64::from(position) + 1);
        assert_eq!(result.input_token, 13);
        assert_eq!(result.output_token, 42);
        assert_eq!(result.layers.len(), 36);
        assert_eq!(result.embedding_ns, [1, 2]);
        assert_eq!(result.tail_ns, [3, 4, 5]);
    }
    let expected = [
        vec!["metadata".into(), "embedding".into(), "begin".into()],
        (0..36).map(|i| format!("layer{i}")).collect(),
        vec!["tail".into(), "fence".into(), "commit".into()],
    ]
    .concat();
    assert_eq!(fake.calls, [expected.clone(), expected].concat());
    assert!(!fake.poisoned);
    let calls = fake.calls.len();
    assert!(sequence.run(&mut fake, &input(2)).is_err());
    assert_eq!(fake.calls.len(), calls);
    assert!(fake.poisoned);
}

#[test]
fn every_backend_failure_stops_and_permanently_poison_owner() {
    for fail_at in 0..42 {
        let mut sequence = sequence();
        let mut fake = Fake::new();
        fake.fail_at = Some(fail_at);
        assert!(
            sequence.run(&mut fake, &input(0)).is_err(),
            "failure {fail_at}"
        );
        assert_eq!(fake.calls.len(), fail_at + 1);
        assert!(fake.poisoned);
        assert_eq!(sequence.completed, 0);
        fake.fail_at = None;
        assert!(sequence.run(&mut fake, &input(0)).is_err());
        assert_eq!(fake.calls.len(), fail_at + 1);
    }
}

#[test]
fn malformed_input_is_rejected_before_metadata_or_embedding() {
    for case in 0..8 {
        let mut request = input(0);
        match case {
            0 => request.registration = [8; 32],
            1 => request.generation = 2,
            2 => request.cache_metadata[0] = 1,
            3 => request.token = VOCABULARY,
            4 => request.rotary_bits[3] = f32::NAN.to_bits(),
            5 => request.rotary_bits[4] = f32::INFINITY.to_bits(),
            6 => request.cache_metadata[1] = 144,
            7 => request.cache_metadata[2] = request.cache_metadata[1],
            _ => unreachable!(),
        }
        let mut sequence = sequence();
        let mut fake = Fake::new();
        assert!(sequence.run(&mut fake, &request).is_err());
        assert!(fake.calls.is_empty() && fake.poisoned);
    }
    assert!(Sequence::new([0; 32], InputMode::TeacherForced).is_err());
}

#[test]
fn invalid_tail_token_cannot_cross_fence_or_commit() {
    let mut sequence = sequence();
    let mut fake = Fake::new();
    fake.output = VOCABULARY;
    assert!(sequence.run(&mut fake, &input(0)).is_err());
    assert_eq!(fake.calls.last().unwrap(), "tail");
    assert!(fake.poisoned && sequence.pages.is_none());
}

#[test]
fn second_forward_must_preserve_first_forward_cache_page_mapping() {
    let mut sequence = sequence();
    let mut fake = Fake::new();
    sequence.run(&mut fake, &input(0)).unwrap();
    let before = fake.calls.len();
    let mut next = input(1);
    next.cache_metadata.swap(1, 2);
    assert!(sequence.run(&mut fake, &next).is_err());
    assert_eq!(fake.calls.len(), before);
    assert_eq!(sequence.completed, 1);
}

#[test]
fn arbitrary_complete_page_permutation_is_retained_between_two_forwards() {
    let mut sequence = sequence();
    let mut fake = Fake::new();
    for position in 0..2 {
        let mut request = input(position);
        request.cache_metadata[1..].reverse();
        request.rotary_bits[0] = (-0.0f32).to_bits();
        assert!(sequence.run(&mut fake, &request).is_ok());
    }
    assert!(!fake.poisoned);
}

#[test]
fn second_forward_failure_keeps_one_committed_result_and_refuses_retry() {
    for fail_at in 0..42 {
        let mut sequence = sequence();
        let mut fake = Fake::new();
        sequence.run(&mut fake, &input(0)).unwrap();
        fake.fail_at = Some(42 + fail_at);
        assert!(sequence.run(&mut fake, &input(1)).is_err());
        assert_eq!(sequence.completed, 1);
        assert_eq!(sequence.last_output, Some(42));
        let calls = fake.calls.len();
        fake.fail_at = None;
        assert!(sequence.run(&mut fake, &input(1)).is_err());
        assert_eq!(fake.calls.len(), calls);
    }
}

#[test]
fn autoregressive_mode_binds_next_input_to_previous_checked_output() {
    for valid in [false, true] {
        let mut sequence = Sequence::new([7; 32], InputMode::Autoregressive).unwrap();
        let mut fake = Fake::new();
        sequence.run(&mut fake, &input(0)).unwrap();
        let mut next = input(1);
        if valid {
            next.token = 42;
        }
        assert_eq!(sequence.run(&mut fake, &next).is_ok(), valid);
        assert_eq!(fake.calls.len(), if valid { 84 } else { 42 });
        assert_eq!(fake.poisoned, !valid);
    }
}
