//! Ordered pure driver for a future native adapter. No native adapter is installed here.
use crate::finite_guarded_mlp_decode_wire_v1::{Control, LayerObservation};
use crate::finite_guarded_mlp_long_wire_v2::{
    BankStep, Bootstrap, Command, Completion, Frame, RESPONSE_SCHEMA, Request, Transcript,
};
use std::io;

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[path = "guarded_mlp_long_sequence_v2_timing.rs"]
mod timing;

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
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    fn forward_timing_selected(&self) -> bool {
        false
    }
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    fn record_forward_timing(
        &mut self,
        _row: crate::finite_guarded_mlp_readiness_forward_durations_v1::ForwardRow,
    ) -> io::Result<()> {
        Err(io::Error::other("forward timing sink not selected"))
    }
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
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        {
            self.run_inner(backend, request, &mut timing::NativeClock::default())
        }
        #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
        {
            self.run_inner(backend, request)
        }
    }
    fn run_inner(
        &mut self,
        backend: &mut impl Backend,
        request: &Request,
        #[cfg(feature = "engineering-currentness-duration-diagnostics")] clock: &mut impl timing::Clock,
    ) -> io::Result<Produced> {
        let mut attempt = Attempt {
            transcript: &mut self.transcript,
            backend,
            finished: false,
        };
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        let mut timing = timing::Timing::new(
            attempt.backend.forward_timing_selected(),
            attempt.transcript.completed(),
        );
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        attempt.backend.check_deadline()?;
        let step = attempt.transcript.begin(request)?;
        let Command::Forward {
            generation, token, ..
        } = &request.command
        else {
            unreachable!()
        };
        attempt.backend.check_deadline()?;
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        attempt.backend.metadata(request)?;
        attempt.backend.check_deadline()?;
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        let embedding_ns = attempt.backend.embedding(*token)?;
        attempt.backend.check_deadline()?;
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        attempt.backend.begin(step)?;
        attempt.backend.check_deadline()?;
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        let mut layers = Vec::with_capacity(36);
        for index in 0..36 {
            layers.push(attempt.backend.layer(index)?);
            attempt.backend.check_deadline()?;
        }
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        let tail = attempt.backend.tail()?;
        attempt.backend.check_deadline()?;
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
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
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        attempt.backend.fence()?;
        attempt.backend.check_deadline()?;
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        timing.mark(clock)?;
        attempt.backend.commit(*generation)?;
        attempt.backend.check_deadline()?;
        attempt.transcript.advance(&frame)?;
        attempt.backend.check_deadline()?;
        // State may already be committed. Keep custody armed through row acceptance:
        // refusal poisons and prevents publication; it does not roll state back.
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        {
            timing.mark(clock)?;
            if let Some(row) = timing.finish()? {
                attempt.backend.record_forward_timing(row)?;
            }
        }
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
