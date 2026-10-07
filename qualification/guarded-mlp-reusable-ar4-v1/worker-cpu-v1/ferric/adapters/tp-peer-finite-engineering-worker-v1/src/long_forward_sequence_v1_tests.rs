use super::*;
use crate::resident_layer::LayerCompletion;

const REG: [u8; 32] = [3; 32];
struct Fake {
    calls: Vec<&'static str>,
    layers: Vec<usize>,
    output: u32,
    fail: Option<&'static str>,
    poisoned: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: vec![],
            layers: vec![],
            output: 7,
            fail: None,
            poisoned: false,
        }
    }
    fn step(&mut self, name: &'static str) -> Result<()> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err("injected failure".into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Fake {
    fn upload_metadata(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("metadata")
    }
    fn embedding(&mut self, _: u32) -> Result<[u64; 2]> {
        self.step("embedding")?;
        Ok([1; 2])
    }
    fn begin_states(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("begin")
    }
    fn layer(&mut self, layer: usize) -> Result<LayerCompletion> {
        self.step("layer")?;
        self.layers.push(layer);
        Ok(LayerCompletion {
            prefix_states: [[0; 22]; 2],
            mlp_states: [[0; 11]; 2],
            paired_ns: [[1; 2]; 4],
        })
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.step("tail")?;
        Ok((self.output, [1; 3]))
    }
    fn idle_fence(&mut self) -> Result<()> {
        self.step("fence")
    }
    fn commit_states(&mut self) -> Result<()> {
        self.step("commit")
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
fn input(generation: u64, token: u32) -> ForwardInput {
    ForwardInput {
        registration: REG,
        generation,
        token,
        cache_metadata: core::array::from_fn(|i| {
            if i == 0 {
                generation as u32 - 1
            } else {
                i as u32 - 1
            }
        }),
        rotary_bits: [0; 128],
    }
}

#[test]
fn exact_prompt_then_255_committed_outputs_covers_2303_forwards() {
    let prompt: Vec<_> = (0..2048).collect();
    let mut sequence = Sequence::new(REG, &prompt).unwrap();
    let mut backend = Fake::new();
    let mut outputs = Vec::new();
    for position in 0..FORWARDS {
        let token = if position < 2048 {
            prompt[position as usize]
        } else {
            backend.output
        };
        backend.output = (position + 3100) % 151_936;
        let result = sequence
            .run(&mut backend, &input(u64::from(position) + 1, token))
            .unwrap();
        if position >= 2047 {
            outputs.push(result.output_token);
        }
        assert_eq!(backend.layers, (0..36).collect::<Vec<_>>());
        assert_eq!(&backend.calls[..3], &["metadata", "embedding", "begin"]);
        assert_eq!(
            &backend.calls[backend.calls.len() - 3..],
            &["tail", "fence", "commit"]
        );
        backend.calls.clear();
        backend.layers.clear();
    }
    assert_eq!(outputs.len(), 256);
    assert!(sequence.exhausted());
    let last = backend.output;
    assert!(sequence.run(&mut backend, &input(2304, last)).is_err());
    assert!(backend.poisoned);
}

#[test]
fn teacher_forcing_decode_or_wrong_prompt_is_fatal_before_native_work() {
    for completed in [0, 2047, 2048, 2302] {
        let mut sequence = Sequence::new(REG, &[3; 2048]).unwrap();
        sequence.completed = completed;
        sequence.last_output = Some(5);
        let mut backend = Fake::new();
        let wrong = if completed < 2048 { 5 } else { 3 };
        assert!(
            sequence
                .run(&mut backend, &input(u64::from(completed) + 1, wrong))
                .is_err()
        );
        assert!(backend.calls.is_empty());
        assert!(backend.poisoned);
    }
}

#[test]
fn metadata_position_mapping_rotary_and_generation_remain_closed() {
    for mutation in 0..7 {
        let mut sequence = Sequence::new(REG, &[3; 2048]).unwrap();
        let mut backend = Fake::new();
        sequence.run(&mut backend, &input(1, 3)).unwrap();
        backend.calls.clear();
        let mut next = input(2, 3);
        match mutation {
            0 => next.registration = [4; 32],
            1 => next.generation = 3,
            2 => next.cache_metadata[0] = 2,
            3 => next.cache_metadata.swap(1, 2),
            4 => next.cache_metadata[1] = 144,
            5 => next.cache_metadata[1] = next.cache_metadata[2],
            _ => next.rotary_bits[127] = f32::NAN.to_bits(),
        }
        assert!(sequence.run(&mut backend, &next).is_err());
        assert!(backend.calls.is_empty());
        assert!(sequence.run(&mut backend, &input(2, 3)).is_err());
    }
}

#[test]
fn every_native_failure_prevents_commit_and_retry() {
    for failure in [
        "metadata",
        "embedding",
        "begin",
        "layer",
        "tail",
        "fence",
        "commit",
    ] {
        let mut sequence = Sequence::new(REG, &[3; 2048]).unwrap();
        let mut backend = Fake::new();
        backend.fail = Some(failure);
        assert!(sequence.run(&mut backend, &input(1, 3)).is_err());
        assert_eq!(sequence.completed, 0);
        assert!(backend.poisoned);
        let count = backend.calls.len();
        backend.fail = None;
        assert!(sequence.run(&mut backend, &input(1, 3)).is_err());
        assert_eq!(backend.calls.len(), count);
    }
}

#[test]
fn malformed_profile_and_out_of_range_tail_are_rejected() {
    assert!(Sequence::new([0; 32], &[1; 2048]).is_err());
    assert!(Sequence::new(REG, &[1; 2047]).is_err());
    assert!(Sequence::new(REG, &[151_936; 2048]).is_err());
    let mut sequence = Sequence::new(REG, &[1; 2048]).unwrap();
    let mut backend = Fake::new();
    backend.output = 151_936;
    assert!(sequence.run(&mut backend, &input(1, 1)).is_err());
    assert!(!backend.calls.contains(&"fence"));
    assert!(!backend.calls.contains(&"commit"));
}
