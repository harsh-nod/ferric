//! Explicit Full2303 entry. No readiness fallback or completion-feasibility grant.
use crate::finite_guarded_mlp_decode_wire_v1 as four;
use crate::finite_guarded_mlp_full2303_bank_scoped_census_v1 as bank_census;
use crate::finite_guarded_mlp_full2303_scoped_v1 as scoped;
use crate::finite_guarded_mlp_full2303_wire_v1 as wire;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::guarded_mlp_long_sequence_v2::Produced;
use crate::native_catalog::forward::guarded_mlp_decode_v1::{Mode, full2303::Owner};
pub use crate::native_guarded_mlp_decode_cli_v1::NativeOptions;
use crate::native_setup::{PreparedSetup, guarded_mlp_decode_v1::PreparedGuardedDecodeSetup};
use crate::resident_layer::{
    guarded_mlp_decode_v1::{Image, Images},
    mlp_tiles_v2::Image as MlpImage,
    prefix_tiles_v6::{
        artifacts::Image as PrefixImage, projection_residual::Image as ProjectionImage,
    },
};
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    time::{Duration, Instant},
};
pub const FLAG: &str = wire::WORKER_FLAG;
pub const SCOPED_FLAG: &str = scoped::WORKER_FLAG;
pub const BANK_CENSUS_FLAG: &str = bank_census::WORKER_FLAG;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativePolicy {
    Full,
    ScopedWarm,
    BankCensus,
}

pub fn parse_bank_scoped_census_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(BANK_CENSUS_FLAG) {
        return Err(io::Error::other("explicit full2303 bank census invocation"));
    }
    let mut delegated = args.to_vec();
    delegated[0] = FLAG.into();
    parse_args(&delegated)
}

pub fn parse_scoped_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(SCOPED_FLAG) {
        return Err(io::Error::other("explicit full2303 scoped invocation"));
    }
    let mut delegated = args.to_vec();
    delegated[0] = FLAG.into();
    parse_args(&delegated)
}

pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(FLAG) {
        return Err(io::Error::other("explicit full2303 invocation"));
    }
    let mut delegated = args.to_vec();
    delegated[0] = crate::native_guarded_mlp_decode_cli_v1::REUSE_FLAG.into();
    crate::native_guarded_mlp_decode_cli_v1::parse_reuse_args(&delegated)
}

trait Backend {
    fn run(&mut self, request: &long::Request) -> io::Result<Produced>;
    fn digest(&self) -> [u8; 32];
    fn output_tokens(&self) -> &[u32];
    fn close(&mut self, request: &long::Request, digest: [u8; 32]) -> io::Result<()>;
    fn cancel(&mut self);
}
struct Native {
    owner: Option<Owner>,
    policy: NativePolicy,
    scoped_counts: Option<scoped::Counts>,
    bank_census_counts: Option<bank_census::Counts>,
}
impl Backend for Native {
    fn run(&mut self, request: &long::Request) -> io::Result<Produced> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("full2303 owner absent"))?
            .run(request)
    }
    fn digest(&self) -> [u8; 32] {
        self.owner.as_ref().map_or([0; 32], Owner::digest)
    }
    fn output_tokens(&self) -> &[u32] {
        self.owner.as_ref().map_or(&[], Owner::output_tokens)
    }
    fn close(&mut self, request: &long::Request, digest: [u8; 32]) -> io::Result<()> {
        let owner = self
            .owner
            .take()
            .ok_or_else(|| io::Error::other("full2303 owner absent at Close"))?;
        match self.policy {
            NativePolicy::Full => owner.close(request, digest),
            NativePolicy::ScopedWarm => {
                self.scoped_counts = Some(owner.close_with_scoped(request, digest)?);
                Ok(())
            }
            NativePolicy::BankCensus => {
                self.bank_census_counts = Some(owner.close_with_bank_census(request, digest)?);
                Ok(())
            }
        }
    }

    fn cancel(&mut self) {
        if let Some(mut owner) = self.owner.take() {
            owner.cancel();
        }
    }
}
struct Publication<'a, B: Backend> {
    backend: &'a mut B,
    complete: bool,
}
impl<B: Backend> Drop for Publication<'_, B> {
    fn drop(&mut self) {
        if !self.complete {
            self.backend.cancel();
        }
    }
}
fn join_completed(backend: &impl Backend, transcript: &long::Transcript) -> io::Result<()> {
    if !transcript.is_closed()
        || transcript.profile() != long::Profile::Full2303
        || transcript.output_tokens().len() != long::OUTPUT_TOKENS
        || backend.digest() != transcript.digest()
        || backend.output_tokens() != transcript.output_tokens()
    {
        return Err(io::Error::other("full2303 owner transcript join"));
    }
    Ok(())
}
fn serve(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &wire::Bootstrap,
    incoming: &mut long::FrameBudget,
) -> io::Result<()> {
    serve_closed(backend, r, w, b, incoming).map(|_| ())
}
fn serve_closed(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &wire::Bootstrap,
    incoming: &mut long::FrameBudget,
) -> io::Result<wire::Closed> {
    let mut operation = Publication {
        backend,
        complete: false,
    };
    b.validate(
        b.sequence.device_ids,
        b.sequence.timeout_ms,
        b.sequence.scope.child_identity,
    )?;
    let mut outgoing = long::FrameBudget::new();
    let mut transcript = long::Transcript::new(b.sequence.clone())?;
    for _ in 0..long::FORWARDS {
        let request = long::read_record::<long::Request>(r, incoming)?
            .ok_or_else(|| io::Error::other("full2303 EOF before 2303 forwards"))?;
        transcript.begin(&request)?;
        let produced = operation.backend.run(&request)?;
        if produced.frame.request != request {
            return Err(io::Error::other("full2303 native request join"));
        }
        transcript.advance(&produced.frame)?;
        long::write_frame(
            w,
            &mut outgoing,
            &produced.frame,
            &produced.control,
            &produced.observation,
        )?;
        w.flush()?;
    }
    let request = long::read_record::<long::Request>(r, incoming)?
        .ok_or_else(|| io::Error::other("full2303 EOF before Close"))?;
    let digest = transcript.digest();
    transcript.close(&request, digest)?;
    join_completed(operation.backend, &transcript)?;
    operation.backend.close(&request, digest)?;
    let closed = wire::Closed::from_transcript(request, &transcript)?;
    long::write_record(w, &mut outgoing, &closed)?;
    w.flush()?;
    operation.complete = true;
    Ok(closed)
}

fn publication_deadline(now: Instant, deadline: Instant) -> io::Result<()> {
    if now >= deadline {
        return Err(io::Error::other("full2303 scoped publication deadline"));
    }
    Ok(())
}

fn publish_policy(
    output: &mut (impl Write + ?Sized),
    b: &wire::Bootstrap,
    closed: &wire::Closed,
    counts: scoped::Counts,
    worker: [u8; 32],
    deadline: Instant,
    rehash: impl FnOnce() -> io::Result<[u8; 32]>,
) -> io::Result<()> {
    publication_deadline(Instant::now(), deadline)?;
    let current = rehash()?;
    if worker != current {
        return Err(io::Error::other("full2303 scoped worker changed"));
    }
    closed.validate(
        &closed.request,
        closed.transcript_sha256,
        &closed.generated_tokens,
    )?;
    scoped::PolicyRecord::new(
        b,
        closed.transcript_sha256,
        &closed.generated_tokens,
        current,
        counts,
    )?
    .write_to(output)?;
    publication_deadline(Instant::now(), deadline)
}

fn publish_bank_census_policy(
    output: &mut (impl Write + ?Sized),
    b: &wire::Bootstrap,
    closed: &wire::Closed,
    counts: bank_census::Counts,
    worker: [u8; 32],
    deadline: Instant,
    rehash: impl FnOnce() -> io::Result<[u8; 32]>,
) -> io::Result<()> {
    publication_deadline(Instant::now(), deadline)?;
    let current = rehash()?;
    if worker != current {
        return Err(io::Error::other("full2303 scoped worker changed"));
    }
    closed.validate(
        &closed.request,
        closed.transcript_sha256,
        &closed.generated_tokens,
    )?;
    bank_census::PolicyRecord::new(
        b,
        closed.transcript_sha256,
        &closed.generated_tokens,
        current,
        counts,
    )?
    .write_to(output)?;
    publication_deadline(Instant::now(), deadline)
}

/// # Safety
/// The trusted parent authenticates original model/images and owns a separately
/// bounded child, process group and retirement reserve. This entry does not admit
/// that the full workload can complete within its fixed one-hour source cap.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    unsafe { run_native_selected(options, r, w, None, NativePolicy::Full) }
}

/// # Safety
/// Same authenticated inputs and separately bounded owned-child obligations as
/// run_native. The scoped windows are temporally distinct from ordinary Full.
#[allow(unsafe_code)]
pub unsafe fn run_native_scoped(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    policy: &mut impl Write,
) -> io::Result<()> {
    unsafe { run_native_selected(options, r, w, Some(policy), NativePolicy::ScopedWarm) }
}

/// # Safety
/// Same owned Full inputs and deadline. Explicitly accepts both bank and census
/// scoped temporal sampling; no workload feasibility or numerical admission.
#[allow(unsafe_code)]
pub unsafe fn run_native_bank_scoped_census(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    policy: &mut impl Write,
) -> io::Result<()> {
    unsafe { run_native_selected(options, r, w, Some(policy), NativePolicy::BankCensus) }
}

#[allow(unsafe_code)]
unsafe fn run_native_selected(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    policy: Option<&mut dyn Write>,
    selected: NativePolicy,
) -> io::Result<()> {
    if (selected == NativePolicy::Full) != policy.is_none() {
        return Err(io::Error::other("full2303 exact selected policy output"));
    }
    let started = Instant::now();
    let mut incoming = long::FrameBudget::new();
    let (b, [mlp, prefix, projection, guarded]) = wire::read_bootstrap(r, &mut incoming)?
        .ok_or_else(|| io::Error::other("full2303 bootstrap absent"))?;
    b.validate(options.devices, options.timeout_ms, std::process::id())?;
    if options.mode != four::InputMode::Autoregressive {
        return Err(io::Error::other("full2303 explicit invocation mode"));
    }
    let deadline = started
        .checked_add(Duration::from_millis(b.child_deadline_ms))
        .ok_or_else(|| io::Error::other("full2303 deadline overflow"))?;
    let setup_profile = b.setup()?;
    let (request, payload) = four::read_begin(r, &mut incoming, &setup_profile)?;
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.sequence.scope)
        .map_err(io::Error::other)?;
    let prepared = PreparedGuardedDecodeSetup::new(
        setup,
        PrefixImage::new(prefix, &b.sequence.prefix_image).map_err(io::Error::other)?,
        MlpImage::new(mlp, &b.sequence.mlp_image).map_err(io::Error::other)?,
        Images {
            projection: ProjectionImage::new(projection, &b.sequence.projection_image)
                .map_err(io::Error::other)?,
            guarded: Image::new(guarded, &b.sequence.guarded_image).map_err(io::Error::other)?,
        },
        Mode::Autoregressive {
            first: b.sequence.prompt_tokens[0],
        },
        options.timeout_ms,
    )
    .map_err(io::Error::other)?
    .with_reusable_arenas()
    .map_err(io::Error::other)?;
    if prepared.profile_sha256() != setup_profile.sha256()? || Instant::now() >= deadline {
        return Err(io::Error::other("full2303 setup profile/deadline"));
    }
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let worker = if policy.is_some() {
        Some(crate::native_prefix_decode_host_v1::executable_sha()?)
    } else {
        None
    };
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() || Instant::now() >= deadline {
        return Err(io::Error::other(
            "full2303 setup closed or deadline elapsed",
        ));
    }
    let base = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    let owner = match selected {
        NativePolicy::Full => Owner::from_owner(base, b.sequence.clone(), deadline)?,
        NativePolicy::ScopedWarm => Owner::from_owner_scoped(base, b.sequence.clone(), deadline)?,
        NativePolicy::BankCensus => {
            Owner::from_owner_bank_census(base, b.sequence.clone(), deadline)?
        }
    };
    let mut native = Native {
        owner: Some(owner),
        policy: selected,
        scoped_counts: None,
        bank_census_counts: None,
    };
    match policy {
        Some(output) => {
            let closed = serve_closed(&mut native, r, w, &b, &mut incoming)?;
            if selected == NativePolicy::BankCensus {
                let counts = native
                    .bank_census_counts
                    .take()
                    .ok_or_else(|| io::Error::other("full2303 bank census Close counts absent"))?;
                return publish_bank_census_policy(
                    output,
                    &b,
                    &closed,
                    counts,
                    worker.ok_or_else(|| io::Error::other("full2303 bank census worker absent"))?,
                    deadline,
                    crate::native_prefix_decode_host_v1::executable_sha,
                );
            }
            let counts = native
                .scoped_counts
                .take()
                .ok_or_else(|| io::Error::other("full2303 scoped healthy Close counts absent"))?;
            publish_policy(
                output,
                &b,
                &closed,
                counts,
                worker.ok_or_else(|| io::Error::other("full2303 scoped worker absent"))?,
                deadline,
                crate::native_prefix_decode_host_v1::executable_sha,
            )
        }
        None => serve(&mut native, r, w, &b, &mut incoming),
    }
}

#[cfg(test)]
#[path = "native_guarded_mlp_full2303_cli_v1_tests.rs"]
mod tests;
