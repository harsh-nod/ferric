//! Explicit forty-position readiness entry; never a Full2303 or AR4 fallback.
use crate::finite_guarded_mlp_decode_wire_v1 as four;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::finite_guarded_mlp_readiness_bank_scoped_v2 as bank_scoped;
use crate::finite_guarded_mlp_readiness_scoped_v1 as scoped;
use crate::finite_guarded_mlp_readiness_shared_v1 as shared;
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
pub const CAUSAL_FLAG: &str = "--engineering-native-guarded-mlp-readiness40-causal-layer0-v1";
pub const SHARED_FLAG: &str = shared::WORKER_FLAG;
pub const SCOPED_FLAG: &str = scoped::WORKER_FLAG;
pub const BANK_SCOPED_FLAG: &str = bank_scoped::WORKER_FLAG;

pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, FLAG)
}
pub fn parse_position5_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, POSITION5_FLAG)
}
pub fn parse_causal_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, CAUSAL_FLAG)
}
pub fn parse_shared_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, SHARED_FLAG)
}
pub fn parse_scoped_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, SCOPED_FLAG)
}
pub fn parse_bank_scoped_args(args: &[OsString]) -> io::Result<NativeOptions> {
    parse_args_for(args, BANK_SCOPED_FLAG)
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
trait FreshPolicy {
    fn configure(&mut self, cache: bool, operational: bool, shared: bool) -> io::Result<()>;
}
impl FreshPolicy for Group {
    fn configure(&mut self, cache: bool, operational: bool, shared: bool) -> io::Result<()> {
        self.configure_performance_v2(cache, operational, shared)
            .map_err(io::Error::other)
    }
}
struct SharedApplied;
fn shared_deadline(now: Instant, deadline: Instant) -> io::Result<()> {
    if now >= deadline {
        return Err(io::Error::other("shared full publication deadline"));
    }
    Ok(())
}
fn admit_policy(profile: long::Profile, causal: bool, selected: bool) -> io::Result<()> {
    if selected && (profile != long::Profile::Readiness40Position5 || causal) {
        return Err(io::Error::other("shared full only ordinary Position5"));
    }
    Ok(())
}
fn install_policy<G: FreshPolicy, T>(
    mut group: G,
    selected: bool,
    install: impl FnOnce(G) -> io::Result<T>,
) -> io::Result<(T, Option<SharedApplied>)> {
    let applied = if selected {
        group.configure(false, false, true)?;
        Some(SharedApplied)
    } else {
        None
    };
    Ok((install(group)?, applied))
}
impl SharedApplied {
    fn publish(
        self,
        b: &wire::Bootstrap,
        digest: [u8; 32],
        worker: [u8; 32],
        w: &mut dyn Write,
    ) -> io::Result<()> {
        let current = crate::native_prefix_decode_host_v1::executable_sha()?;
        if current != worker {
            return Err(io::Error::other("shared full worker changed"));
        }
        shared::PolicyRecord::new(b, digest, current)?.write_to(w)
    }
}
fn admit_scoped(
    profile: long::Profile,
    causal: bool,
    shared: bool,
    selected: bool,
) -> io::Result<()> {
    if selected && (profile != long::Profile::Readiness40Position5 || causal || shared) {
        return Err(io::Error::other(
            "scoped warm only separate ordinary Position5",
        ));
    }
    Ok(())
}
fn scoped_deadline(now: Instant, deadline: Instant) -> io::Result<()> {
    if now >= deadline {
        return Err(io::Error::other("scoped warm publication deadline"));
    }
    Ok(())
}
fn admit_bank_scoped(
    profile: long::Profile,
    causal: bool,
    shared: bool,
    scoped: bool,
    selected: bool,
) -> io::Result<()> {
    if selected && (profile != long::Profile::Readiness40Position5 || causal || shared || scoped) {
        return Err(io::Error::other(
            "bank scoped only separate ordinary Position5",
        ));
    }
    Ok(())
}
struct Native {
    owner: Option<Owner>,
    causal: bool,
    bootstrap: wire::Bootstrap,
    sidecar: Option<Vec<u8>>,
    scoped: bool,
    scoped_counts: Option<scoped::Counts>,
    bank_scoped: bool,
    bank_counts: Option<bank_scoped::Counts>,
}
impl Backend for Native {
    fn run(&mut self, request: &long::Request) -> io::Result<Produced> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("readiness owner absent"))?
            .run(request)
    }
    fn digest(&self) -> [u8; 32] {
        self.owner.as_ref().map_or([0; 32], Owner::digest)
    }
    fn close(&mut self, request: &long::Request, digest: [u8; 32]) -> io::Result<()> {
        let owner = self
            .owner
            .take()
            .ok_or_else(|| io::Error::other("readiness owner absent at Close"))?;
        if self.causal {
            self.sidecar = Some(owner.close_with_causal(request, digest, &self.bootstrap)?);
            Ok(())
        } else if self.bank_scoped {
            self.bank_counts = Some(owner.close_with_bank_scoped(request, digest)?);
            Ok(())
        } else if self.scoped {
            self.scoped_counts = Some(owner.close_with_scoped(request, digest)?);
            Ok(())
        } else {
            owner.close(request, digest)
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
fn serve(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &wire::Bootstrap,
    incoming: &mut long::FrameBudget,
) -> io::Result<[u8; 32]> {
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
    Ok(digest)
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
    unsafe { run_native_for(options, r, w, long::Profile::Readiness40, None) }
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
    unsafe { run_native_for(options, r, w, long::Profile::Readiness40Position5, None) }
}
/// # Safety
/// Same readiness lifetime. The separate sidecar is published only after healthy Close.
#[allow(unsafe_code)]
pub unsafe fn run_native_causal(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: &mut impl Write,
) -> io::Result<()> {
    unsafe {
        run_native_for(
            options,
            r,
            w,
            long::Profile::Readiness40Position5,
            Some(diagnostic),
        )
    }
}
#[allow(unsafe_code)]
unsafe fn run_native_for(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    profile: long::Profile,
    diagnostic: Option<&mut dyn Write>,
) -> io::Result<()> {
    unsafe { run_native_policy(options, r, w, profile, diagnostic, None) }
}
/// # Safety
/// The ordinary Position5 lifetime obligations apply. Only fresh full group
/// currentness is shared; no operational checks, paired reads or terminals.
#[allow(unsafe_code)]
pub unsafe fn run_native_shared(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    policy: &mut impl Write,
) -> io::Result<()> {
    unsafe {
        run_native_policy(
            options,
            r,
            w,
            long::Profile::Readiness40Position5,
            None,
            Some(policy),
        )
    }
}
/// # Safety
/// Same sealed model/image and whole-lifetime obligations as run_native.
/// Warm layers select a temporally distinct closed currentness transaction;
/// neither SharedFull nor operational/cached currentness is enabled.
#[allow(unsafe_code)]
pub unsafe fn run_native_scoped(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    policy: &mut impl Write,
) -> io::Result<()> {
    unsafe {
        run_native_selected(
            options,
            r,
            w,
            long::Profile::Readiness40Position5,
            None,
            None,
            Some(policy),
        )
    }
}
#[allow(unsafe_code)]
unsafe fn run_native_policy(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    profile: long::Profile,
    diagnostic: Option<&mut dyn Write>,
    policy: Option<&mut dyn Write>,
) -> io::Result<()> {
    unsafe { run_native_selected(options, r, w, profile, diagnostic, policy, None) }
}
#[allow(unsafe_code)]
unsafe fn run_native_selected(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    profile: long::Profile,
    diagnostic: Option<&mut dyn Write>,
    policy: Option<&mut dyn Write>,
    scoped_policy: Option<&mut dyn Write>,
) -> io::Result<()> {
    unsafe {
        run_native_bank_selected(
            options,
            r,
            w,
            profile,
            diagnostic,
            policy,
            scoped_policy,
            None,
        )
    }
}
/// # Safety
/// Same authenticated owned Position5 lifetime as run_native_scoped. Warm bank
/// rearm adds its own temporally distinct window, separate from each layer;
/// allocation preflights and every ordinary/default selector stay unchanged.
#[allow(unsafe_code)]
pub unsafe fn run_native_bank_scoped(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    policy: &mut impl Write,
) -> io::Result<()> {
    unsafe {
        run_native_bank_selected(
            options,
            r,
            w,
            long::Profile::Readiness40Position5,
            None,
            None,
            None,
            Some(policy),
        )
    }
}
#[allow(unsafe_code)]
unsafe fn run_native_bank_selected(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    profile: long::Profile,
    diagnostic: Option<&mut dyn Write>,
    policy: Option<&mut dyn Write>,
    scoped_policy: Option<&mut dyn Write>,
    bank_policy: Option<&mut dyn Write>,
) -> io::Result<()> {
    admit_bank_scoped(
        profile,
        diagnostic.is_some(),
        policy.is_some(),
        scoped_policy.is_some(),
        bank_policy.is_some(),
    )?;
    admit_scoped(
        profile,
        diagnostic.is_some(),
        policy.is_some(),
        scoped_policy.is_some(),
    )?;
    admit_policy(profile, diagnostic.is_some(), policy.is_some())?;
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
    let worker = if policy.is_some() || scoped_policy.is_some() || bank_policy.is_some() {
        Some(crate::native_prefix_decode_host_v1::executable_sha()?)
    } else {
        None
    };
    let (mut setup, applied) = install_policy(group, policy.is_some(), |group| {
        prepared.into_processor(group).map_err(io::Error::other)
    })?;
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
            if bank_policy.is_some() {
                Owner::from_owner_bank_scoped(base, b.sequence.clone(), deadline)?
            } else if scoped_policy.is_some() {
                Owner::from_owner_scoped(base, b.sequence.clone(), deadline)?
            } else if diagnostic.is_some() {
                Owner::from_owner_causal(base, b.sequence.clone(), deadline)?
            } else {
                Owner::from_owner_position5(base, b.sequence.clone(), deadline)?
            }
        }
        long::Profile::Full2303 => return Err(io::Error::other("readiness refuses Full2303")),
    };
    let mut native = Native {
        owner: Some(owner),
        causal: diagnostic.is_some(),
        bootstrap: b.clone(),
        sidecar: None,
        scoped: scoped_policy.is_some(),
        scoped_counts: None,
        bank_scoped: bank_policy.is_some(),
        bank_counts: None,
    };
    let digest = serve(&mut native, r, w, &b, &mut incoming)?;
    if let Some(output) = policy {
        shared_deadline(Instant::now(), deadline)?;
        applied
            .ok_or_else(|| io::Error::other("shared full policy not applied"))?
            .publish(
                &b,
                digest,
                worker.ok_or_else(|| io::Error::other("shared full worker absent"))?,
                output,
            )?;
        shared_deadline(Instant::now(), deadline)?;
    }
    if let Some(output) = scoped_policy {
        scoped_deadline(Instant::now(), deadline)?;
        let current = crate::native_prefix_decode_host_v1::executable_sha()?;
        if Some(current) != worker {
            return Err(io::Error::other("scoped warm worker changed"));
        }
        let counts = native
            .scoped_counts
            .take()
            .ok_or_else(|| io::Error::other("scoped warm healthy Close counts absent"))?;
        scoped::PolicyRecord::new(&b, digest, current, counts)?.write_to(output)?;
        scoped_deadline(Instant::now(), deadline)?;
    } else if native.scoped_counts.is_some() {
        return Err(io::Error::other("unexpected scoped warm policy counts"));
    }
    if let Some(output) = bank_policy {
        scoped_deadline(Instant::now(), deadline)?;
        let current = crate::native_prefix_decode_host_v1::executable_sha()?;
        if Some(current) != worker {
            return Err(io::Error::other("bank scoped worker changed"));
        }
        let counts = native
            .bank_counts
            .take()
            .ok_or_else(|| io::Error::other("bank scoped healthy Close counts absent"))?;
        bank_scoped::PolicyRecord::new(&b, digest, current, counts)?.write_to(output)?;
        scoped_deadline(Instant::now(), deadline)?;
    } else if native.bank_counts.is_some() {
        return Err(io::Error::other("unexpected bank scoped policy counts"));
    }
    if let Some(output) = diagnostic {
        if Instant::now() >= deadline {
            return Err(io::Error::other("causal sidecar deadline"));
        }
        output.write_all(
            &native
                .sidecar
                .take()
                .ok_or_else(|| io::Error::other("causal Close sidecar missing"))?,
        )?;
        output.flush()?;
        if Instant::now() >= deadline {
            return Err(io::Error::other("causal sidecar deadline"));
        }
    } else if native.sidecar.is_some() {
        return Err(io::Error::other("unexpected causal sidecar"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "native_guarded_mlp_readiness_cli_v1_tests.rs"]
mod tests;

#[cfg(test)]
mod causal_selector_tests {
    use super::*;
    #[test]
    fn causal_selector_is_distinct_and_keeps_machine_code_and_ar_mode_gates() {
        let values = [
            CAUSAL_FLAG,
            "--allow-unauthenticated-machine-code",
            "--devices",
            "7,9",
            "--timeout-ms",
            "10000",
            "--mode",
            "autoregressive",
        ];
        let good = values.map(OsString::from);
        assert!(parse_causal_args(&good).is_ok());
        assert!(parse_args(&good).is_err());
        assert!(parse_position5_args(&good).is_err());
        let mut bad = good.clone();
        bad[7] = "teacher-forced".into();
        assert!(parse_causal_args(&bad).is_err());
        assert!(parse_causal_args(&good[1..]).is_err());
        let mut bad = good.to_vec();
        bad.push("--full2303".into());
        assert!(parse_causal_args(&bad).is_err());
    }
}
