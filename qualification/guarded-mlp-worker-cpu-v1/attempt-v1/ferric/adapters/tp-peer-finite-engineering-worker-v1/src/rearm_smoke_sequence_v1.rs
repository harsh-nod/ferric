//! Exactly four declared teacher-forced forwards; no long-workload completion.

use crate::forward_sequence::{Backend, ForwardCompletion, ForwardInput};
type Result<T> = core::result::Result<T, String>;

pub(super) struct Sequence {
    registration: [u8; 32],
    prompt: [u32; 4],
    completed: u32,
    pages: Option<[u32; 144]>,
    terminal: bool,
}
impl Sequence {
    pub(super) fn new(registration: [u8; 32], prompt: &[u32]) -> Result<Self> {
        if registration == [0; 32]
            || prompt.len() != 4
            || prompt.iter().any(|token| *token >= 151_936)
        {
            return Err("smoke registration/prompt".into());
        }
        Ok(Self {
            registration,
            prompt: prompt.try_into().map_err(|_| "smoke prompt extent")?,
            completed: 0,
            pages: None,
            terminal: false,
        })
    }
    pub(super) fn exhausted(&self) -> bool {
        !self.terminal && self.completed == 4
    }
    fn validate(&self, input: &ForwardInput) -> Result<()> {
        if self.terminal
            || self.completed >= 4
            || input.registration != self.registration
            || input.generation != u64::from(self.completed) + 1
            || input.cache_metadata[0] != self.completed
            || input.token != self.prompt[self.completed as usize]
            || input
                .rotary_bits
                .iter()
                .any(|word| !f32::from_bits(*word).is_finite())
        {
            return Err("smoke identity/teacher token/generation/phase".into());
        }
        let mut seen = [false; 144];
        for &page in &input.cache_metadata[1..] {
            let slot = seen.get_mut(page as usize).ok_or("smoke page bounds")?;
            if *slot {
                return Err("smoke page alias".into());
            }
            *slot = true;
        }
        if self
            .pages
            .as_ref()
            .is_some_and(|pages| pages.as_slice() != &input.cache_metadata[1..])
        {
            return Err("smoke retained page mapping changed".into());
        }
        Ok(())
    }
    pub(super) fn run(
        &mut self,
        backend: &mut impl Backend,
        input: &ForwardInput,
    ) -> Result<ForwardCompletion> {
        let result = (|| {
            self.validate(input)?;
            backend.upload_metadata(input)?;
            let embedding_ns = backend.embedding(input.token)?;
            backend.begin_states(input)?;
            let mut layers = Vec::with_capacity(36);
            for layer in 0..36 {
                layers.push(backend.layer(layer)?);
            }
            let (output_token, tail_ns) = backend.tail()?;
            if output_token >= 151_936 {
                return Err("smoke tail token bounds".into());
            }
            backend.idle_fence()?;
            // This succeeds only after the actual reuse ledger commits this
            // generation. No progress/count publication precedes it.
            backend.commit_states()?;
            let mut pages = [0; 144];
            pages.copy_from_slice(&input.cache_metadata[1..]);
            self.pages = Some(pages);
            self.completed += 1;
            Ok(ForwardCompletion {
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token,
                embedding_ns,
                layers,
                tail_ns,
            })
        })();
        if result.is_err() {
            self.terminal = true;
            backend.poison();
        }
        result
    }
}

#[cfg(test)]
#[path = "rearm_smoke_sequence_v1_tests.rs"]
mod tests;
