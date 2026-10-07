//! Explicit forty-position readiness entry; never a Full2303 or AR4 fallback.
use crate::finite_guarded_mlp_decode_wire_v1 as four;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::finite_guarded_mlp_readiness_wire_v1 as wire;
use crate::guarded_mlp_long_sequence_v2::Produced;
use crate::native_catalog::forward::guarded_mlp_decode_v1::{Mode, readiness::Owner};
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
pub const POSITION5_FLAG: &str = wire::POSITION5_WORKER_FLAG;

pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, FLAG)
}
pub fn parse_position5_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, POSITION5_FLAG)
}
fn parse_args_for(args: &[OsString], flag: &str) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(flag) {
        return Err(io::Error::other("explicit readiness40 invocation"));
    }
    let mut delegated = args.to_vec();
    delegated[0] = crate::native_guarded_mlp_decode_cli_v1::REUSE_FLAG.into();
    crate::native_guarded_mlp_decode_cli_v1::parse_reuse_args(&delegated)
}

trait Backend {
    fn run(&mut self, request: &long::Request) -> io::Result<Produced>;
    fn digest(&self) -> [u8; 32];
    fn close(&mut self, request: &long::Request, digest: [u8; 32]) -> io::Result<()>;
    fn cancel(&mut self);
}
struct Native(Option<Owner>);
impl Backend for Native {
    fn run(&mut self, request: &long::Request) -> io::Result<Produced> {
        self.0
            .as_mut()
            .ok_or_else(|| io::Error::other("readiness owner absent"))?
            .run(request)
    }
    fn digest(&self) -> [u8; 32] {
        self.0.as_ref().map_or([0; 32], Owner::digest)
    }
    fn close(&mut self, request: &long::Request, digest: [u8; 32]) -> io::Result<()> {
        self.0
            .take()
            .ok_or_else(|| io::Error::other("readiness owner absent at Close"))?
            .close(request, digest)
    }
    fn cancel(&mut self) {
        if let Some(mut owner) = self.0.take() {
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
fn serve(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &wire::Bootstrap,
    incoming: &mut long::FrameBudget,
) -> io::Result<()> {
    let mut operation = Publication {
        backend,
        complete: false,
    };
    let mut outgoing = long::FrameBudget::new();
    let mut transcript = long::Transcript::new(b.sequence.clone())?;
    for _ in 0..long::READINESS_FORWARDS {
        let request = long::read_record::<long::Request>(r, incoming)?
            .ok_or_else(|| io::Error::other("readiness EOF before forty forwards"))?;
        transcript.begin(&request)?;
        let produced = operation.backend.run(&request)?;
        if produced.frame.request != request {
            return Err(io::Error::other("readiness native request join"));
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
        .ok_or_else(|| io::Error::other("readiness EOF before Close"))?;
    let digest = transcript.digest();
    transcript.close(&request, digest)?;
    if operation.backend.digest() != digest {
        return Err(io::Error::other("readiness owner transcript join"));
    }
    operation.backend.close(&request, digest)?;
    let closed = wire::Closed::new_for(b.sequence.profile, request, digest)?;
    long::write_record(w, &mut outgoing, &closed)?;
    w.flush()?;
    operation.complete = true;
    Ok(())
}

/// # Safety
/// The trusted parent must authenticate all source/model/images, own bounded
/// child retirement, and establish reviewed peer/coherence premises. This entry
/// grants no production authority and admits only forty prompt positions.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    unsafe { run_native_for(options, r, w, long::Profile::Readiness40) }
}

/// # Safety
/// The same entire-lifetime obligations as run_native apply; only the four
/// selected observations differ, not execution scope, policy or authority.
#[allow(unsafe_code)]
pub unsafe fn run_native_position5(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    unsafe { run_native_for(options, r, w, long::Profile::Readiness40Position5) }
}
#[allow(unsafe_code)]
unsafe fn run_native_for(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    profile: long::Profile,
) -> io::Result<()> {
    let started = Instant::now();
    let mut incoming = long::FrameBudget::new();
    let (b, [mlp, prefix, projection, guarded]) =
        wire::read_bootstrap_for(r, &mut incoming, profile)?
            .ok_or_else(|| io::Error::other("readiness bootstrap absent"))?;
    b.validate(options.devices, options.timeout_ms, std::process::id())?;
    if options.mode != four::InputMode::Autoregressive {
        return Err(io::Error::other("readiness explicit invocation mode"));
    }
    let deadline = started
        .checked_add(Duration::from_millis(b.child_deadline_ms))
        .ok_or_else(|| io::Error::other("readiness deadline overflow"))?;
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
        return Err(io::Error::other("readiness setup profile/deadline"));
    }
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() || Instant::now() >= deadline {
        return Err(io::Error::other(
            "readiness setup closed or deadline elapsed",
        ));
    }
    let base = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    let owner = match profile {
        long::Profile::Readiness40 => Owner::from_owner(base, b.sequence.clone(), deadline)?,
        long::Profile::Readiness40Position5 => {
            Owner::from_owner_position5(base, b.sequence.clone(), deadline)?
        }
        long::Profile::Full2303 => return Err(io::Error::other("readiness refuses Full2303")),
    };
    serve(&mut Native(Some(owner)), r, w, &b, &mut incoming)
}

#[cfg(test)]
#[path = "native_guarded_mlp_readiness_cli_v1_tests.rs"]
mod tests;
