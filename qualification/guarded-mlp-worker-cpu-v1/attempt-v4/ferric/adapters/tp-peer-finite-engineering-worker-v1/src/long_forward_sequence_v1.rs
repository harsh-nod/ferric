//! Separate2048-prompt/256-output engineering sequence, never an old mode alias.

use crate::finite_long_wire_v1::{FORWARDS, PROMPT_TOKENS};
use crate::forward_sequence::{Backend, ForwardCompletion, ForwardInput};

type Result<T> = core::result::Result<T, String>;

pub(super) struct Sequence {
    registration: [u8; 32],
    prompt: [u32; PROMPT_TOKENS],
    completed: u32,
    pages: Option<[u32; 144]>,
    last_output: Option<u32>,
    terminal: bool,
}

impl Sequence {
    pub(super) fn new(registration: [u8; 32], prompt: &[u32]) -> Result<Self> {
        if registration == [0; 32]
            || prompt.len() != PROMPT_TOKENS
            || prompt.iter().any(|token| *token >= 151_936)
        {
            return Err("long sequence registration or prompt".into());
        }
        Ok(Self {
            registration,
            prompt: prompt.try_into().map_err(|_| "long prompt extent")?,
            completed: 0,
            pages: None,
            last_output: None,
            terminal: false,
        })
    }

    pub(super) fn exhausted(&self) -> bool {
        !self.terminal && self.completed == FORWARDS
    }

    fn validate(&self, input: &ForwardInput) -> Result<()> {
        if self.terminal
            || self.completed >= FORWARDS
            || input.registration != self.registration
            || input.generation != u64::from(self.completed) + 1
            || input.cache_metadata[0] != self.completed
            || input.token >= 151_936
            || input
                .rotary_bits
                .iter()
                .any(|word| !f32::from_bits(*word).is_finite())
        {
            return Err("long forward identity, position, rotary or exhausted phase".into());
        }
        let expected = if (self.completed as usize) < PROMPT_TOKENS {
            self.prompt[self.completed as usize]
        } else {
            self.last_output
                .ok_or("long decode without committed predecessor")?
        };
        if input.token != expected {
            return Err("long input differs from frozen prompt or committed output".into());
        }
        let mut seen = [false; 144];
        for &page in &input.cache_metadata[1..] {
            let slot = seen.get_mut(page as usize).ok_or("long page bounds")?;
            if *slot {
                return Err("long page alias".into());
            }
            *slot = true;
        }
        if self
            .pages
            .as_ref()
            .is_some_and(|pages| pages.as_slice() != &input.cache_metadata[1..])
        {
            return Err("long retained KV mapping changed".into());
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
                return Err("long tail token bounds".into());
            }
            backend.idle_fence()?;
            backend.commit_states()?;
            let mut pages = [0; 144];
            pages.copy_from_slice(&input.cache_metadata[1..]);
            self.pages = Some(pages);
            self.last_output = Some(output_token);
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
#[path = "long_forward_sequence_v1_tests.rs"]
mod tests;
