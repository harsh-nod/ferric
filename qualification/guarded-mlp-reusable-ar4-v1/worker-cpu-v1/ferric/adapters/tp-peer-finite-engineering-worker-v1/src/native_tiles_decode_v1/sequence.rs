use super::{Completion, ForwardInput, Mode, Profile, Result, layer};
pub(super) trait Backend {
    fn metadata(&mut self, input: &ForwardInput) -> Result<()>;
    fn embedding(&mut self, token: u32) -> Result<[u64; 2]>;
    fn begin(&mut self, input: &ForwardInput) -> Result<()>;
    fn layer(&mut self, index: usize) -> Result<layer::Completion>;
    fn tail(&mut self) -> Result<(u32, [u64; 3])>;
    fn fence(&mut self) -> Result<()>;
    fn commit(&mut self) -> Result<()>;
    fn poison(&mut self);
}
pub(super) struct Sequence {
    registration: [u8; 32],
    profile: [u8; 32],
    mode: Mode,
    completed: u32,
    pages: Option<[u32; 144]>,
    last: Option<u32>,
    terminal: bool,
}
impl Sequence {
    pub(super) fn new(profile: &Profile) -> Self {
        Self {
            registration: profile.registration,
            profile: profile.sha256,
            mode: profile.mode,
            completed: 0,
            pages: None,
            last: None,
            terminal: false,
        }
    }
    pub(super) fn exhausted(&self) -> bool {
        self.completed == 4 && !self.terminal
    }
    pub(super) fn run(
        &mut self,
        b: &mut impl Backend,
        profile: [u8; 32],
        input: &ForwardInput,
    ) -> Result<Completion> {
        let result = (|| {
            if self.terminal
                || self.completed >= 4
                || profile != self.profile
                || input.registration != self.registration
                || input.generation != u64::from(self.completed) + 1
                || input.cache_metadata[0] != self.completed
                || input.token >= 151936
                || input
                    .rotary_bits
                    .iter()
                    .any(|x| !f32::from_bits(*x).is_finite())
            {
                return Err("tiles forward identity/order/profile/rotary".into());
            }
            let expected = match self.mode {
                Mode::TeacherForced(tokens) => tokens[self.completed as usize],
                Mode::Autoregressive { first } => {
                    if self.completed == 0 {
                        first
                    } else {
                        self.last.ok_or("tiles prior output")?
                    }
                }
            };
            if input.token != expected {
                return Err("tiles input is not selected prompt/own previous output".into());
            }
            let mut seen = [false; 144];
            for &page in &input.cache_metadata[1..] {
                let slot = seen.get_mut(page as usize).ok_or("tiles page bounds")?;
                if *slot {
                    return Err("tiles page alias".into());
                }
                *slot = true;
            }
            if self
                .pages
                .as_ref()
                .is_some_and(|p| p.as_slice() != &input.cache_metadata[1..])
            {
                return Err("tiles retained KV page mapping changed".into());
            }
            b.metadata(input)?;
            let embedding_ns = b.embedding(input.token)?;
            b.begin(input)?;
            let mut layers = Vec::with_capacity(36);
            for index in 0..36 {
                layers.push(b.layer(index)?);
            }
            let (output_token, tail_ns) = b.tail()?;
            if output_token >= 151936 {
                return Err("tiles output token bounds".into());
            }
            b.fence()?;
            b.commit()?;
            let mut pages = [0; 144];
            pages.copy_from_slice(&input.cache_metadata[1..]);
            self.pages = Some(pages);
            self.last = Some(output_token);
            self.completed += 1;
            Ok(Completion {
                profile_sha256: profile,
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
            b.poison();
        }
        result
    }
}
