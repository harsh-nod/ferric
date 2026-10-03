//! Two complete forwards. A token is publishable only after the tail and fence.

use crate::resident_layer::LayerCompletion;

type Result<T> = core::result::Result<T, String>;
const LAYERS: usize = 36;
const PAGES: usize = 144;
const VOCABULARY: u32 = 151_936;

#[derive(Clone, Copy)]
pub(super) enum InputMode {
    /// Two supplied prompt tokens, explicitly not autoregressive generation.
    TeacherForced,
    /// The second forward must consume the first forward's checked output.
    Autoregressive,
}

pub(super) struct ForwardInput {
    pub(super) registration: [u8; 32],
    pub(super) generation: u64,
    pub(super) token: u32,
    pub(super) cache_metadata: [u32; PAGES + 1],
    /// Exact source-owned F32 bits; this coordinator does not recompute RoPE.
    pub(super) rotary_bits: [u32; 128],
}

impl ForwardInput {
    fn validate(
        &self,
        registration: [u8; 32],
        completed: u32,
        previous_pages: Option<&[u32; PAGES]>,
    ) -> Result<()> {
        if registration == [0; 32]
            || self.registration != registration
            || completed >= 2
            || self.generation != u64::from(completed) + 1
            || self.cache_metadata[0] != completed
            || self.token >= VOCABULARY
            || self
                .rotary_bits
                .iter()
                .any(|bits| !f32::from_bits(*bits).is_finite())
        {
            return Err("finite forward identity, token, position or rotary bounds".into());
        }
        let mut seen = [false; PAGES];
        for &page in &self.cache_metadata[1..] {
            let slot = seen.get_mut(page as usize).ok_or("finite page bounds")?;
            if *slot {
                return Err("finite page mapping aliases".into());
            }
            *slot = true;
        }
        if previous_pages.is_some_and(|pages| pages.as_slice() != &self.cache_metadata[1..]) {
            return Err("finite second forward changed retained KV page mapping".into());
        }
        Ok(())
    }
}

pub(super) struct ForwardCompletion {
    pub(super) generation: u64,
    pub(super) position: u32,
    pub(super) input_token: u32,
    pub(super) output_token: u32,
    pub(super) embedding_ns: [u64; 2],
    pub(super) layers: Vec<LayerCompletion>,
    pub(super) tail_ns: [u64; 3],
}

/// Private native implementation owns all retained tokens. Fake implementations
/// exist only to verify sequencing and failure behavior without a GPU.
pub(super) trait Backend {
    fn upload_metadata(&mut self, input: &ForwardInput) -> Result<()>;
    fn embedding(&mut self, token: u32) -> Result<[u64; 2]>;
    fn begin_states(&mut self, input: &ForwardInput) -> Result<()>;
    fn layer(&mut self, layer: usize) -> Result<LayerCompletion>;
    fn tail(&mut self) -> Result<(u32, [u64; 3])>;
    fn idle_fence(&mut self) -> Result<()>;
    fn commit_states(&mut self) -> Result<()>;
    fn poison(&mut self);
}

pub(super) struct Sequence {
    registration: [u8; 32],
    completed: u32,
    pages: Option<[u32; PAGES]>,
    mode: InputMode,
    last_output: Option<u32>,
    terminal: bool,
}

impl Sequence {
    pub(super) fn new(registration: [u8; 32], mode: InputMode) -> Result<Self> {
        if registration == [0; 32] {
            return Err("empty finite registration".into());
        }
        Ok(Self {
            registration,
            completed: 0,
            pages: None,
            mode,
            last_output: None,
            terminal: false,
        })
    }

    pub(super) fn run(
        &mut self,
        backend: &mut impl Backend,
        input: &ForwardInput,
    ) -> Result<ForwardCompletion> {
        let result = (|| {
            if self.terminal {
                return Err("finite forward owner is terminal".into());
            }
            input.validate(self.registration, self.completed, self.pages.as_ref())?;
            if matches!(self.mode, InputMode::Autoregressive)
                && self.last_output.is_some_and(|token| input.token != token)
            {
                return Err("finite autoregressive input differs from committed output".into());
            }
            backend.upload_metadata(input)?;
            let embedding_ns = backend.embedding(input.token)?;
            backend.begin_states(input)?;
            let mut layers = Vec::with_capacity(LAYERS);
            for layer in 0..LAYERS {
                layers.push(backend.layer(layer)?);
            }
            let (output_token, tail_ns) = backend.tail()?;
            if output_token >= VOCABULARY {
                return Err("finite tail returned an out-of-vocabulary token".into());
            }
            backend.idle_fence()?;
            backend.commit_states()?;
            let mut pages = [0; PAGES];
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
#[path = "forward_sequence_tests.rs"]
mod tests;
