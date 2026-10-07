//! Ordered pure driver for a future native adapter. No native adapter is installed here.
use crate::finite_guarded_mlp_decode_wire_v1::{Control, LayerObservation};
use crate::finite_guarded_mlp_long_wire_v2::{
    BankStep, Bootstrap, Command, Completion, Frame, RESPONSE_SCHEMA, Request, Transcript,
};
use std::io;

pub struct Tail {
    pub output_token: u32,
    pub host_ns: [u64; 3],
    pub observation: Vec<u8>,
}
/// Implementations retain actual owner/currentness/retirement checks and finite,
/// equal-rank hidden readbacks. `check_deadline` checks one unchanged outer deadline.
/// `poison` must not panic; it quarantines partial native work, never retries it.
pub trait Backend {
    fn check_deadline(&mut self) -> io::Result<()>;
    fn metadata(&mut self, request: &Request) -> io::Result<()>;
    fn embedding(&mut self, token: u32) -> io::Result<[u64; 2]>;
    fn begin(&mut self, step: BankStep) -> io::Result<()>;
    fn layer(&mut self, index: usize) -> io::Result<LayerObservation>;
    fn tail(&mut self) -> io::Result<Tail>;
    fn fence(&mut self) -> io::Result<()>;
    fn commit(&mut self, generation: u64) -> io::Result<()>;
    fn close(&mut self) -> io::Result<()>;
    fn poison(&mut self);
}
pub struct Produced {
    pub frame: Frame,
    pub control: Control,
    pub observation: Vec<u8>,
}
pub struct Sequence {
    transcript: Transcript,
}
struct Attempt<'a, B: Backend> {
    transcript: &'a mut Transcript,
    backend: &'a mut B,
    finished: bool,
}
impl<B: Backend> Drop for Attempt<'_, B> {
    fn drop(&mut self) {
        if !self.finished {
            self.transcript.cancel();
            self.backend.poison();
        }
    }
}
impl Sequence {
    pub fn new(bootstrap: Bootstrap) -> io::Result<Self> {
        Ok(Self {
            transcript: Transcript::new(bootstrap)?,
        })
    }
    pub fn completed(&self) -> u32 {
        self.transcript.completed()
    }
    pub fn output_tokens(&self) -> &[u32] {
        self.transcript.output_tokens()
    }
    pub fn digest(&self) -> [u8; 32] {
        self.transcript.digest()
    }
    pub fn is_closed(&self) -> bool {
        self.transcript.is_closed()
    }
    /// The caller cancels on publication/transport failure; no response retry is permitted.
    pub fn cancel(&mut self, backend: &mut impl Backend) {
        self.transcript.cancel();
        backend.poison();
    }
    pub fn run(&mut self, backend: &mut impl Backend, request: &Request) -> io::Result<Produced> {
        let mut attempt = Attempt {
            transcript: &mut self.transcript,
            backend,
            finished: false,
        };
        attempt.backend.check_deadline()?;
        let step = attempt.transcript.begin(request)?;
        let Command::Forward {
            generation, token, ..
        } = &request.command
        else {
            unreachable!()
        };
        attempt.backend.check_deadline()?;
        attempt.backend.metadata(request)?;
        attempt.backend.check_deadline()?;
        let embedding_ns = attempt.backend.embedding(*token)?;
        attempt.backend.check_deadline()?;
        attempt.backend.begin(step)?;
        attempt.backend.check_deadline()?;
        let mut layers = Vec::with_capacity(36);
        for index in 0..36 {
            layers.push(attempt.backend.layer(index)?);
            attempt.backend.check_deadline()?;
        }
        let tail = attempt.backend.tail()?;
        attempt.backend.check_deadline()?;
        let control = Control {
            embedding_ns,
            layers,
            tail_ns: tail.host_ns,
        };
        let profile = attempt.transcript.profile();
        let completion = Completion::from_observation(
            profile,
            request,
            &control,
            &tail.observation,
            tail.output_token,
        )?;
        let mut frame = Frame {
            schema: RESPONSE_SCHEMA.into(),
            profile,
            request: request.clone(),
            completion,
        };
        frame.completion.chain = attempt.transcript.next_chain(request, &frame.completion)?;
        frame.validate(profile)?;
        attempt.backend.check_deadline()?;
        attempt.backend.fence()?;
        attempt.backend.check_deadline()?;
        attempt.backend.commit(*generation)?;
        attempt.backend.check_deadline()?;
        attempt.transcript.advance(&frame)?;
        attempt.backend.check_deadline()?;
        attempt.finished = true;
        Ok(Produced {
            frame,
            control,
            observation: tail.observation,
        })
    }
    pub fn close(
        &mut self,
        backend: &mut impl Backend,
        request: &Request,
        digest: [u8; 32],
    ) -> io::Result<()> {
        let mut attempt = Attempt {
            transcript: &mut self.transcript,
            backend,
            finished: false,
        };
        attempt.backend.check_deadline()?;
        attempt.transcript.close(request, digest)?;
        attempt.backend.check_deadline()?;
        attempt.backend.close()?;
        attempt.backend.check_deadline()?;
        attempt.finished = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "guarded_mlp_long_sequence_v2_tests.rs"]
mod tests;
