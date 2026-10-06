use super::*;
use crate::resident_layer::LayerCompletion;

const REG: [u8; 32] = [1; 32];
const PROMPT: [u32; 4] = [9112, 2190, 3772, 220];
#[derive(Default)]
struct Fake {
    calls: Vec<&'static str>,
    generations: Vec<u64>,
    layers: Vec<usize>,
    fail: Option<&'static str>,
    poisoned: bool,
}
impl Fake {
    fn step(&mut self, name: &'static str) -> Result<()> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err("injected".into())
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
    fn begin_states(&mut self, input: &ForwardInput) -> Result<()> {
        self.step("begin")?;
        self.generations.push(input.generation);
        Ok(())
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
        Ok((67, [1; 3]))
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
fn input(position: u32) -> ForwardInput {
    ForwardInput {
        registration: REG,
        generation: u64::from(position) + 1,
        token: PROMPT[(position as usize).min(3)],
        cache_metadata: core::array::from_fn(|i| if i == 0 { position } else { i as u32 - 1 }),
        rotary_bits: [0; 128],
    }
}

#[test]
fn four_teacher_inputs_commit_generations_one_through_four_then_exhaust() {
    let mut sequence = Sequence::new(REG, &PROMPT).unwrap();
    let mut backend = Fake::default();
    for position in 0..4 {
        let result = sequence.run(&mut backend, &input(position)).unwrap();
        assert_eq!(result.input_token, PROMPT[position as usize]);
        assert_eq!(result.output_token, 67);
        assert_eq!(&backend.calls[..3], &["metadata", "embedding", "begin"]);
        assert_eq!(
            &backend.calls[backend.calls.len() - 3..],
            &["tail", "fence", "commit"]
        );
        assert_eq!(backend.layers, (0..36).collect::<Vec<_>>());
        backend.calls.clear();
        backend.layers.clear();
        assert_eq!(sequence.exhausted(), position == 3);
    }
    assert_eq!(backend.generations, [1, 2, 3, 4]);
    assert!(sequence.run(&mut backend, &input(4)).is_err());
    assert!(backend.calls.is_empty());
    assert!(backend.poisoned);
}

#[test]
fn autoregressive_substitution_is_not_a_teacher_forced_smoke() {
    let mut sequence = Sequence::new(REG, &PROMPT).unwrap();
    let mut backend = Fake::default();
    sequence.run(&mut backend, &input(0)).unwrap();
    backend.calls.clear();
    let mut next = input(1);
    next.token = 67;
    assert!(sequence.run(&mut backend, &next).is_err());
    assert!(backend.calls.is_empty());
}

#[test]
fn stale_skipped_wrongmodel_metadata_or_alias_fails_before_native_work() {
    for mutation in 0..8 {
        let mut sequence = Sequence::new(REG, &PROMPT).unwrap();
        let mut backend = Fake::default();
        sequence.run(&mut backend, &input(0)).unwrap();
        backend.calls.clear();
        let mut next = input(1);
        match mutation {
            0 => next.registration = [2; 32],
            1 => next.generation = 1,
            2 => next.generation = 3,
            3 => next.cache_metadata[0] = 2,
            4 => next.cache_metadata.swap(1, 2),
            5 => next.cache_metadata[1] = 144,
            6 => next.cache_metadata[1] = next.cache_metadata[2],
            _ => next.rotary_bits[0] = f32::INFINITY.to_bits(),
        }
        assert!(sequence.run(&mut backend, &next).is_err());
        assert!(backend.calls.is_empty());
        assert!(sequence.run(&mut backend, &input(1)).is_err());
    }
}

#[test]
fn failed_state_commit_or_other_phase_never_advances_or_retries() {
    for failure in [
        "metadata",
        "embedding",
        "begin",
        "layer",
        "tail",
        "fence",
        "commit",
    ] {
        let mut sequence = Sequence::new(REG, &PROMPT).unwrap();
        let mut backend = Fake {
            fail: Some(failure),
            ..Fake::default()
        };
        assert!(sequence.run(&mut backend, &input(0)).is_err());
        assert_eq!(sequence.completed, 0);
        assert!(!sequence.exhausted());
        assert!(backend.poisoned);
        let calls = backend.calls.len();
        backend.fail = None;
        assert!(sequence.run(&mut backend, &input(0)).is_err());
        assert_eq!(backend.calls.len(), calls);
    }
}

#[test]
fn profile_does_not_accept_old_two_or_full_long_prompt_shape() {
    assert!(Sequence::new([0; 32], &PROMPT).is_err());
    assert!(Sequence::new(REG, &PROMPT[..2]).is_err());
    assert!(Sequence::new(REG, &[1; 2048]).is_err());
    assert!(Sequence::new(REG, &[1, 2, 3, 151_936]).is_err());
}
