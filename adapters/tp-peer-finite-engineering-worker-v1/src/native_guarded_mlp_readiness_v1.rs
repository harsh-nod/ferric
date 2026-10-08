//! Private forty-forward owner. Full2303 and public transport are not enabled.
use super::sequence::Backend as NativeBackend;
use super::{Active, ForwardInput, Mode, Native, Phase};
use crate::finite_guarded_mlp_decode_wire_v1::LayerObservation;
use crate::finite_guarded_mlp_long_wire_v2::{BankStep, Bootstrap, Command, Profile, Request};
use crate::guarded_mlp_long_sequence_v2::{Backend, Produced, Sequence, Tail};
use crate::state_roster::guarded_mlp_decode_v1::{BOUND_COUNTS, REUSE_MAX_COUNTS};
use std::{
    io,
    time::{Duration, Instant},
};

#[path = "native_guarded_mlp_readiness_bank_scoped_v2.rs"]
mod bank_scoped;
#[path = "native_guarded_mlp_readiness_census_v3.rs"]
mod census_scoped;
#[path = "native_guarded_mlp_readiness_scoped_v1.rs"]
mod scoped;
#[path = "native_guarded_mlp_readiness_tail_v4.rs"]
mod tail_scoped;

const CACHE_BYTES: u64 = 2304 * 512 * 2;
const WHOLE_LIMIT: Duration = Duration::from_secs(3600);

struct CensusLayerResult {
    completion: crate::resident_layer::guarded_mlp_decode_v1::Completion,
    hidden: Vec<u8>,
    counters: Option<(
        fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1,
        fe2o3_kfd::Gfx950EngineeringPeerScopedCapacityCensusObservationV1,
    )>,
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    durations: Option<fe2o3_kfd::Gfx950EngineeringCurrentnessDurationsV1>,
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    layer_durations: Option<fe2o3_kfd::Gfx950EngineeringPeerScopedLayerDurationsV1>,
}
struct TailResult {
    value: (u32, [u64; 3], Vec<u8>, Vec<u8>),
    counters: Option<fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1>,
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    durations: Option<fe2o3_kfd::Gfx950EngineeringCurrentnessDurationsV1>,
}

fn require(ok: bool, why: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}
fn remaining(now: Instant, deadline: Instant) -> io::Result<()> {
    require(now < deadline, "readiness fixed outer deadline")
}
fn admit_deadline(now: Instant, deadline: Instant) -> io::Result<()> {
    remaining(now, deadline)?;
    require(
        deadline.duration_since(now) <= WHOLE_LIMIT,
        "readiness refuses an enlarged outer deadline",
    )
}
fn input(request: &Request) -> io::Result<ForwardInput> {
    input_for(request, Profile::Readiness40)
}
fn input_for(request: &Request, profile: Profile) -> io::Result<ForwardInput> {
    request.validate(profile)?;
    let Command::Forward {
        generation,
        token,
        cache_metadata,
        rotary_bits,
    } = &request.command
    else {
        return Err(io::Error::other("readiness native input requires Forward"));
    };
    Ok(ForwardInput {
        registration: request.registration,
        generation: *generation,
        token: *token,
        cache_metadata: cache_metadata
            .as_slice()
            .try_into()
            .map_err(io::Error::other)?,
        rotary_bits: rotary_bits
            .as_slice()
            .try_into()
            .map_err(io::Error::other)?,
    })
}
fn profile_join(b: &Bootstrap, profile: &super::Profile) -> io::Result<()> {
    profile_join_for(b, profile, Profile::Readiness40)
}
fn profile_join_for(b: &Bootstrap, profile: &super::Profile, expected: Profile) -> io::Result<()> {
    crate::finite_guarded_mlp_readiness_wire_v1::schemas(expected)?;
    b.validate(b.device_ids, b.timeout_ms, std::process::id())?;
    require(
        b.profile == expected
            && profile.reusable_arenas
            && matches!(profile.mode, Mode::Autoregressive { first } if first == b.prompt_tokens[0])
            && profile.scope == b.scope
            && profile.registration == b.registration
            && profile.prefix == b.prefix_image.sha256
            && profile.mlp == b.mlp_image.sha256
            && profile.projection == b.projection_image.sha256
            && profile.timeout_ms == b.timeout_ms,
        "readiness actual sealed profile/scope/images",
    )?;
    let original = crate::finite_guarded_mlp_decode_wire_v1::profile_sha256(
        &b.scope,
        b.registration,
        b.prefix_image.sha256,
        b.mlp_image.sha256,
        b.projection_image.sha256,
        true,
        [b.prompt_tokens[0], 0, 0, 0],
        b.timeout_ms,
        b.device_ids,
    );
    require(
        profile.sha256
            == crate::finite_guarded_mlp_decode_wire_v1::reusable_profile_sha256(original),
        "readiness exact original rank/device profile binding",
    )
}

// Pure shape checks are not authority; the caller derives every row from owned
// source records and actual opaque tokens before reaching this helper.
fn cache_shapes(rows: &[(u32, u32, usize, u64, u64, u64)]) -> io::Result<()> {
    require(rows.len() == 144, "readiness complete two-rank K/V roster")?;
    let mut ids = std::collections::BTreeSet::new();
    for (index, &(rank, layer, side, source_id, token_bytes, allocation_bytes)) in
        rows.iter().enumerate()
    {
        require(
            rank as usize == index / 72
                && layer as usize == index / 2 % 36
                && side == index % 2
                && source_id != 0
                && ids.insert((rank, source_id))
                && token_bytes == CACHE_BYTES
                && allocation_bytes == CACHE_BYTES,
            "readiness actual 144-page cache role/extent/alias",
        )?;
    }
    Ok(())
}
pub(super) fn capacity(base: &mut super::Owner) -> io::Result<()> {
    let owner = &mut base.owner;
    owner.catalog.source.validate()?;
    require(
        owner.catalog.phase == Phase::LayersSealed && owner.layer_bindings.len() == 36,
        "readiness sealed complete layer bindings",
    )?;
    let mut rows = Vec::with_capacity(144);
    let mut tokens = Vec::with_capacity(144);
    for source in &owner.catalog.source.layers {
        let roots = &owner.layer_bindings[source.layer as usize];
        let rank = source.rank as usize;
        require(
            roots.prefix[rank][4].bytes() == 128 * 4 && roots.prefix[rank][5].bytes() == 145 * 4,
            "readiness actual rotary and complete page-map allocations",
        )?;
        for (side, cache) in source.caches.iter().enumerate() {
            let key = crate::native_catalog::BindingKey::Source {
                rank: cache.rank,
                id: cache.id,
            };
            let record = owner
                .catalog
                .records
                .iter()
                .find(|r| r.facts.key == key)
                .ok_or_else(|| io::Error::other("readiness retained KV record missing"))?;
            let token = roots.prefix[rank][10 + side];
            require(
                record.token == token
                    && token.owner_rank() == rank
                    && record.facts.bytes == CACHE_BYTES
                    && !record.facts.immutable
                    && record.upload.is_none()
                    && !tokens.contains(&token),
                "readiness actual KV source/token custody",
            )?;
            rows.push((
                source.rank,
                source.layer,
                side,
                cache.id,
                token.bytes(),
                record.facts.allocation_bytes,
            ));
            tokens.push(token);
        }
    }
    cache_shapes(&rows)?;
    require(
        owner
            .catalog
            .backend
            .preflight_additional_allocations_v1(&[0, 0])
            .map_err(io::Error::other)?
            == BOUND_COUNTS,
        "readiness actual unused reusable allocation census",
    )
}
pub(super) fn poison(base: &mut super::Owner) {
    base.sequence.poison();
    base.states.poison();
    base.owner.catalog.phase = Phase::Terminal;
}
struct Admission<'a> {
    base: &'a mut super::Owner,
    committed: bool,
}
impl Drop for Admission<'_> {
    fn drop(&mut self) {
        if !self.committed {
            poison(self.base);
        }
    }
}

pub(crate) struct Owner {
    base: super::Owner,
    sequence: Sequence,
    profile: Profile,
    deadline: Instant,
    causal: Option<crate::resident_layer::capture_v1::causal_v1::Series>,
    scoped: Option<scoped::State>,
    bank_scoped: Option<bank_scoped::State>,
    census_scoped: Option<census_scoped::State>,
    tail_scoped: Option<tail_scoped::State>,
}
impl Owner {
    /// Consumes a pristine genuine reusable owner, without resetting or replacing
    /// any state. The separate transport must authenticate the bootstrap before
    /// setup and cancel this owner if publication fails. No Full2303 constructor.
    pub(crate) fn from_owner(
        base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        Self::from_owner_for(base, b, deadline, Profile::Readiness40)
    }
    pub(crate) fn from_owner_position5(
        base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        Self::from_owner_for(base, b, deadline, Profile::Readiness40Position5)
    }
    pub(crate) fn from_owner_scoped(
        base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        let mut value = Self::from_owner_for(base, b, deadline, Profile::Readiness40Position5)?;
        value.scoped = Some(scoped::State::new(value.profile, false)?);
        Ok(value)
    }
    pub(crate) fn from_owner_bank_scoped(
        base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        let mut value = Self::from_owner_for(base, b, deadline, Profile::Readiness40Position5)?;
        value.bank_scoped = Some(bank_scoped::State::new(value.profile)?);
        Ok(value)
    }
    pub(crate) fn from_owner_census_scoped(
        base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        let mut value = Self::from_owner_for(base, b, deadline, Profile::Readiness40Position5)?;
        value.census_scoped = Some(census_scoped::State::new(value.profile)?);
        Ok(value)
    }
    pub(crate) fn from_owner_tail_scoped(
        base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        let mut value = Self::from_owner_for(base, b, deadline, Profile::Readiness40Position5)?;
        #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
        {
            value.tail_scoped = Some(tail_scoped::State::new(value.profile)?);
        }
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        {
            value.tail_scoped = Some(tail_scoped::State::new_diagnostic(value.profile)?);
        }
        Ok(value)
    }
    pub(crate) fn from_owner_causal(
        base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
    ) -> io::Result<Self> {
        let mut value = Self::from_owner_for(base, b, deadline, Profile::Readiness40Position5)?;
        value.causal = Some(crate::resident_layer::capture_v1::causal_v1::Series::new());
        Ok(value)
    }
    fn from_owner_for(
        mut base: super::Owner,
        b: Bootstrap,
        deadline: Instant,
        profile: Profile,
    ) -> io::Result<Self> {
        let mut admission = Admission {
            base: &mut base,
            committed: false,
        };
        let now = Instant::now();
        admit_deadline(now, deadline)?;
        profile_join_for(&b, &admission.base.profile, profile)?;
        require(
            admission.base.sequence.pristine()
                && admission.base.states.completed() == 0
                && admission.base.states.between()
                && !admission.base.states.readiness40()
                && !admission.base.states.full2303()
                && !admission.base.capture_enabled
                && admission.base.captured_layer0.is_none()
                && admission.base.observer.is_none(),
            "readiness consumes pristine owner without diagnostics",
        )?;
        require(
            b.begin.source_program.bytes
                == admission.base.owner.catalog.source.source_program_bytes
                && b.begin.source_program.sha256
                    == admission.base.owner.catalog.source.source_program_sha256,
            "readiness original source program custody",
        )?;
        capacity(admission.base)?;
        remaining(Instant::now(), deadline)?;
        let sequence = Sequence::new(b)?;
        admission
            .base
            .states
            .select_readiness40()
            .map_err(io::Error::other)?;
        remaining(Instant::now(), deadline)?;
        admission.committed = true;
        drop(admission);
        Ok(Self {
            base,
            sequence,
            profile,
            deadline,
            causal: None,
            scoped: None,
            bank_scoped: None,
            census_scoped: None,
            tail_scoped: None,
        })
    }
    pub(crate) fn completed(&self) -> u32 {
        self.sequence.completed()
    }
    pub(crate) fn digest(&self) -> [u8; 32] {
        self.sequence.digest()
    }
    pub(crate) fn run(&mut self, request: &Request) -> io::Result<Produced> {
        let mut backend = Driver {
            base: &mut self.base,
            deadline: self.deadline,
            profile: self.profile,
            input: None,
            hidden: Vec::with_capacity(36),
            causal: self.causal.as_mut(),
            scoped: self.scoped.as_mut(),
            bank_scoped: self.bank_scoped.as_mut(),
            census_scoped: self.census_scoped.as_mut(),
            tail_scoped: self.tail_scoped.as_mut(),
            full_scoped: None,
            full_bank_census: None,
            full_bank_census_tail: None,
        };
        self.sequence.run(&mut backend, request)
    }
    pub(crate) fn close(mut self, request: &Request, digest: [u8; 32]) -> io::Result<()> {
        require(
            self.causal.is_none()
                && self.scoped.is_none()
                && self.bank_scoped.is_none()
                && self.census_scoped.is_none()
                && self.tail_scoped.is_none(),
            "selected diagnostics/policy require separate Close publication",
        )?;
        let mut backend = Driver {
            base: &mut self.base,
            deadline: self.deadline,
            profile: self.profile,
            input: None,
            hidden: Vec::new(),
            causal: None,
            scoped: None,
            bank_scoped: None,
            census_scoped: None,
            tail_scoped: None,
            full_scoped: None,
            full_bank_census: None,
            full_bank_census_tail: None,
        };
        self.sequence.close(&mut backend, request, digest)
    }
    pub(crate) fn close_with_scoped(
        mut self,
        request: &Request,
        digest: [u8; 32],
    ) -> io::Result<crate::finite_guarded_mlp_readiness_scoped_v1::Counts> {
        let counts = self
            .scoped
            .take()
            .ok_or_else(|| io::Error::other("scoped warm route not selected"))?
            .closed_counts()?;
        self.close(request, digest)?;
        Ok(counts)
    }
    pub(crate) fn close_with_bank_scoped(
        mut self,
        request: &Request,
        digest: [u8; 32],
    ) -> io::Result<crate::finite_guarded_mlp_readiness_bank_scoped_v2::Counts> {
        let counts = self
            .bank_scoped
            .take()
            .ok_or_else(|| io::Error::other("bank scoped route not selected"))?
            .closed_counts()?;
        self.close(request, digest)?;
        Ok(counts)
    }
    pub(crate) fn close_with_census_scoped(
        mut self,
        request: &Request,
        digest: [u8; 32],
    ) -> io::Result<crate::finite_guarded_mlp_readiness_bank_scoped_census_v3::Counts> {
        let counts = self
            .census_scoped
            .take()
            .ok_or_else(|| io::Error::other("census scoped route not selected"))?
            .closed_counts()?;
        self.close(request, digest)?;
        Ok(counts)
    }
    #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
    pub(crate) fn close_with_tail_scoped(
        mut self,
        request: &Request,
        digest: [u8; 32],
    ) -> io::Result<crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::Counts> {
        let counts = self
            .tail_scoped
            .take()
            .ok_or_else(|| io::Error::other("tail scoped route not selected"))?
            .closed_counts()?;
        self.close(request, digest)?;
        Ok(counts)
    }
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    pub(crate) fn close_with_tail_diagnostic(
        mut self,
        request: &Request,
        digest: [u8; 32],
    ) -> io::Result<(
        crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::Counts,
        Vec<crate::finite_guarded_mlp_readiness_currentness_durations_v1::ForwardRow>,
        Vec<crate::finite_guarded_mlp_readiness_layer_durations_v2::ForwardRow>,
    )> {
        let mut admission = Admission {
            base: &mut self.base,
            committed: false,
        };
        let state = self
            .tail_scoped
            .take()
            .ok_or_else(|| io::Error::other("diagnostic Tail route not selected"))?;
        let counts = state.closed_counts()?;
        let rows = state.diagnostic_rows(&counts)?;
        let forward_rows = state.forward_rows()?;
        remaining(Instant::now(), self.deadline)?;
        admission.committed = true;
        drop(admission);
        self.close(request, digest)?;
        Ok((counts, rows, forward_rows))
    }
    pub(crate) fn close_with_causal(
        mut self,
        request: &Request,
        digest: [u8; 32],
        bootstrap: &crate::finite_guarded_mlp_readiness_wire_v1::Bootstrap,
    ) -> io::Result<Vec<u8>> {
        let series = self
            .causal
            .take()
            .ok_or_else(|| io::Error::other("causal diagnostic not enabled"))?;
        series
            .encode_after_close(bootstrap, digest, || {
                self.close(request, digest).map_err(|e| e.to_string())
            })
            .map_err(io::Error::other)
    }
    pub(crate) fn cancel(&mut self) {
        let mut backend = Driver {
            base: &mut self.base,
            deadline: self.deadline,
            profile: self.profile,
            input: None,
            hidden: Vec::new(),
            causal: None,
            scoped: None,
            bank_scoped: None,
            census_scoped: None,
            tail_scoped: None,
            full_scoped: None,
            full_bank_census: None,
            full_bank_census_tail: None,
        };
        self.sequence.cancel(&mut backend);
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if !self.sequence.is_closed() {
            poison(&mut self.base);
        }
    }
}

pub(super) struct Driver<'a> {
    base: &'a mut super::Owner,
    deadline: Instant,
    profile: Profile,
    input: Option<ForwardInput>,
    hidden: Vec<Vec<u8>>,
    causal: Option<&'a mut crate::resident_layer::capture_v1::causal_v1::Series>,
    scoped: Option<&'a mut scoped::State>,
    bank_scoped: Option<&'a mut bank_scoped::State>,
    census_scoped: Option<&'a mut census_scoped::State>,
    tail_scoped: Option<&'a mut tail_scoped::State>,
    full_scoped: Option<&'a mut super::full2303::scoped::State>,
    full_bank_census: Option<&'a mut super::full2303::bank_census::State>,
    full_bank_census_tail: Option<&'a mut super::full2303::bank_census_tail::State>,
}
impl<'a> Driver<'a> {
    // The sibling Full2303 owner has already consumed and admitted genuine custody.
    pub(super) fn full2303(base: &'a mut super::Owner, deadline: Instant) -> io::Result<Self> {
        require(
            base.states.full2303(),
            "full2303 driver requires selected actual roster",
        )?;
        Ok(Self {
            base,
            deadline,
            profile: Profile::Full2303,
            input: None,
            hidden: Vec::with_capacity(36),
            causal: None,
            scoped: None,
            bank_scoped: None,
            census_scoped: None,
            tail_scoped: None,
            full_scoped: None,
            full_bank_census: None,
            full_bank_census_tail: None,
        })
    }
    pub(super) fn full2303_scoped(
        base: &'a mut super::Owner,
        deadline: Instant,
        state: &'a mut super::full2303::scoped::State,
    ) -> io::Result<Self> {
        let mut driver = Self::full2303(base, deadline)?;
        driver.full_scoped = Some(state);
        Ok(driver)
    }
    pub(super) fn full2303_bank_census(
        base: &'a mut super::Owner,
        deadline: Instant,
        state: &'a mut super::full2303::bank_census::State,
    ) -> io::Result<Self> {
        let mut driver = Self::full2303(base, deadline)?;
        driver.full_bank_census = Some(state);
        Ok(driver)
    }
    pub(super) fn full2303_bank_census_tail(
        base: &'a mut super::Owner,
        deadline: Instant,
        state: &'a mut super::full2303::bank_census_tail::State,
    ) -> io::Result<Self> {
        let mut driver = Self::full2303(base, deadline)?;
        driver.full_bank_census_tail = Some(state);
        Ok(driver)
    }
    fn execute_layer(
        &mut self,
        index: usize,
        capture: Option<&mut crate::resident_layer::capture_v1::causal_v1::Collector>,
        warm: bool,
    ) -> io::Result<(
        crate::resident_layer::guarded_mlp_decode_v1::Completion,
        Vec<u8>,
        Option<fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1>,
    )> {
        use crate::resident_layer::guarded_mlp_decode_v1 as layer;
        let full_extent = self.profile == Profile::Full2303;
        let mut native = self.native()?;
        if warm {
            require(
                capture.is_none() && native.observer.is_none(),
                "scoped warm refuses capture/observer combination",
            )?;
            let owner = &mut native.base.owner;
            let roots = owner
                .layer_bindings
                .get(index)
                .ok_or_else(|| io::Error::other("scoped readiness layer binding"))?;
            let prefix = owner
                .prefix_artifacts
                .as_ref()
                .ok_or_else(|| io::Error::other("scoped readiness Prefix image"))?;
            let mlp = owner
                .tiles_artifacts
                .as_ref()
                .ok_or_else(|| io::Error::other("scoped readiness MLP image"))?;
            let images = owner
                .guarded_loaded
                .as_ref()
                .ok_or_else(|| io::Error::other("scoped readiness guarded image"))?;
            let down = owner
                .guarded_down
                .ok_or_else(|| io::Error::other("scoped readiness Down scratch"))?;
            // SAFETY: this private owner selected genuine warm extent-specific
            // custody; sealed images/roles and the same-slot mixed-bank gate
            // remain required. No fallback or extra hidden read is permitted.
            let observed = unsafe {
                let execute = if full_extent {
                    layer::execute_full2303_warm_scoped
                } else {
                    layer::execute_warm_scoped
                };
                execute(
                    &mut owner.catalog.backend,
                    native.states,
                    prefix,
                    mlp,
                    images,
                    roots,
                    down,
                    index,
                    native.base.timeout_ms,
                )
            }
            .map_err(io::Error::other)?;
            return Ok((
                layer::Completion {
                    prefix_states: observed.prefix_states,
                    prefix_ns: observed.prefix_ns,
                    guarded: observed.guarded,
                },
                observed.hidden,
                Some(observed.currentness),
            ));
        }
        let done = match capture {
            Some(value) => native.layer_with_causal_capture(index, value),
            None => native.layer(index),
        }
        .map_err(io::Error::other)?;
        let hidden = native
            .base
            .layer_hidden
            .pop()
            .ok_or_else(|| io::Error::other("readiness checked hidden missing"))?;
        Ok((done, hidden, None))
    }
    fn execute_census_layer(&mut self, index: usize, warm: bool) -> io::Result<CensusLayerResult> {
        require(
            self.profile == Profile::Readiness40Position5 && self.causal.is_none(),
            "census only ordinary Position5",
        )?;
        self.execute_census_layer_for(index, warm, false)
    }
    fn execute_full2303_census_layer(
        &mut self,
        index: usize,
        warm: bool,
    ) -> io::Result<CensusLayerResult> {
        require(
            self.profile == Profile::Full2303 && self.causal.is_none(),
            "census only separately selected Full2303",
        )?;
        self.execute_census_layer_for(index, warm, true)
    }
    fn execute_census_layer_for(
        &mut self,
        index: usize,
        warm: bool,
        full_extent: bool,
    ) -> io::Result<CensusLayerResult> {
        use crate::resident_layer::guarded_mlp_decode_v1 as layer;
        if !warm {
            let (done, hidden, currentness) = self.execute_layer(index, None, false)?;
            require(currentness.is_none(), "census first-use remains ordinary")?;
            return Ok(CensusLayerResult {
                completion: done,
                hidden,
                counters: None,
                #[cfg(feature = "engineering-currentness-duration-diagnostics")]
                durations: None,
                #[cfg(feature = "engineering-currentness-duration-diagnostics")]
                layer_durations: None,
            });
        }
        let mut native = self.native()?;
        require(native.observer.is_none(), "census refuses observer")?;
        let owner = &mut native.base.owner;
        let roots = owner
            .layer_bindings
            .get(index)
            .ok_or_else(|| io::Error::other("census readiness layer binding"))?;
        let prefix = owner
            .prefix_artifacts
            .as_ref()
            .ok_or_else(|| io::Error::other("census readiness Prefix image"))?;
        let mlp = owner
            .tiles_artifacts
            .as_ref()
            .ok_or_else(|| io::Error::other("census readiness MLP image"))?;
        let images = owner
            .guarded_loaded
            .as_ref()
            .ok_or_else(|| io::Error::other("census readiness guarded image"))?;
        let down = owner
            .guarded_down
            .ok_or_else(|| io::Error::other("census readiness Down scratch"))?;
        // SAFETY: the private extent-specific state and Roster retain exact warm
        // custody. The new facade derives its own accounting; no fallback.
        let observed = unsafe {
            let execute = if full_extent {
                layer::execute_full2303_warm_scoped_census
            } else {
                layer::execute_warm_scoped_census
            };
            execute(
                &mut owner.catalog.backend,
                native.states,
                prefix,
                mlp,
                images,
                roots,
                down,
                index,
                native.base.timeout_ms,
            )
        }
        .map_err(io::Error::other)?;
        Ok(CensusLayerResult {
            completion: layer::Completion {
                prefix_states: observed.layer.prefix_states,
                prefix_ns: observed.layer.prefix_ns,
                guarded: observed.layer.guarded,
            },
            hidden: observed.layer.hidden,
            counters: Some((observed.layer.currentness, observed.census)),
            #[cfg(feature = "engineering-currentness-duration-diagnostics")]
            durations: Some(observed.currentness_durations),
            #[cfg(feature = "engineering-currentness-duration-diagnostics")]
            layer_durations: Some(observed.layer_durations),
        })
    }
    fn execute_tail(&mut self, warm: bool) -> io::Result<TailResult> {
        if warm {
            require(
                self.profile == Profile::Readiness40Position5 && self.causal.is_none(),
                "tail scoped only non-diagnostic Position5",
            )?;
        }
        self.execute_tail_admitted(warm)
    }
    fn execute_full2303_tail(&mut self, warm: bool) -> io::Result<TailResult> {
        require(
            self.profile == Profile::Full2303 && self.causal.is_none(),
            "tail scoped only separately selected Full2303",
        )?;
        self.execute_tail_admitted(warm)
    }
    fn execute_tail_admitted(&mut self, warm: bool) -> io::Result<TailResult> {
        let deadline = self.deadline;
        let mut native = self.native()?;
        #[cfg(feature = "engineering-currentness-duration-diagnostics")]
        let mut durations = None;
        let (token, host_ns, counters) = if warm {
            require(native.observer.is_none(), "tail scoped refuses observer")?;
            let observed = native
                .base
                .tail_scoped(deadline)
                .map_err(io::Error::other)?;
            #[cfg(feature = "engineering-currentness-duration-diagnostics")]
            {
                durations = Some(observed.currentness_durations);
            }
            (observed.token, observed.host_ns, Some(observed.currentness))
        } else {
            let (token, host_ns) = native.tail().map_err(io::Error::other)?;
            (token, host_ns, None)
        };
        let normalized = std::mem::take(&mut native.base.final_normalized);
        let logits = std::mem::take(&mut native.base.logits);
        Ok(TailResult {
            value: (token, host_ns, normalized, logits),
            counters,
            #[cfg(feature = "engineering-currentness-duration-diagnostics")]
            durations,
        })
    }
    fn native(&mut self) -> io::Result<Native<'_>> {
        let input = self
            .input
            .as_ref()
            .ok_or_else(|| io::Error::other("readiness metadata not admitted"))?;
        Ok(Native {
            base: Active {
                owner: &mut self.base.owner,
                timeout_ms: self.base.profile.timeout_ms,
                layer_hidden: Vec::new(),
                final_normalized: Vec::new(),
                logits: Vec::new(),
                capture: None,
                captured_layer0: None,
                reuse: None,
            },
            states: &mut self.base.states,
            generation: input.generation,
            position: input.cache_metadata[0],
            observer: None,
        })
    }
}
impl Backend for Driver<'_> {
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    fn forward_timing_selected(&self) -> bool {
        self.profile == Profile::Readiness40Position5
            && self
                .tail_scoped
                .as_deref()
                .is_some_and(tail_scoped::State::forward_timing_selected)
    }
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    fn record_forward_timing(
        &mut self,
        row: crate::finite_guarded_mlp_readiness_forward_durations_v1::ForwardRow,
    ) -> io::Result<()> {
        require(
            self.forward_timing_selected(),
            "diagnostic forward route not selected",
        )?;
        self.tail_scoped
            .as_deref_mut()
            .ok_or_else(|| io::Error::other("diagnostic forward Tail state absent"))?
            .record_forward_timing(row)
    }
    fn check_deadline(&mut self) -> io::Result<()> {
        remaining(Instant::now(), self.deadline)
    }
    fn metadata(&mut self, request: &Request) -> io::Result<()> {
        let input = input_for(request, self.profile)?;
        require(
            self.base.owner.catalog.phase == Phase::LayersSealed
                && self.input.is_none()
                && self.hidden.is_empty(),
            "readiness terminal or reused native driver",
        )?;
        self.input = Some(ForwardInput {
            registration: input.registration,
            generation: input.generation,
            token: input.token,
            cache_metadata: input.cache_metadata,
            rotary_bits: input.rotary_bits,
        });
        self.native()?.metadata(&input).map_err(io::Error::other)
    }
    fn embedding(&mut self, token: u32) -> io::Result<[u64; 2]> {
        self.native()?.embedding(token).map_err(io::Error::other)
    }
    fn begin(&mut self, step: BankStep) -> io::Result<()> {
        let i = self
            .input
            .as_ref()
            .ok_or_else(|| io::Error::other("readiness input missing"))?;
        require(
            step == BankStep::at(self.profile, i.cache_metadata[0])?,
            "readiness exact bank step",
        )?;
        if let Some(state) = self.full_bank_census_tail.take() {
            let result = state.begin(i.cache_metadata[0], || {
                // SAFETY: this owner selected only the separate bank-scoped
                // Full2303 route; Roster and runtime retain actual bank proofs.
                unsafe {
                    self.base.states.begin_full2303_bank_scoped(
                        &mut self.base.owner.catalog.backend,
                        i.registration,
                        self.base.owner.catalog.scope.model_id,
                        i.generation,
                        i.cache_metadata[0],
                        self.base.profile.timeout_ms,
                    )
                }
                .map_err(io::Error::other)
            });
            self.full_bank_census_tail = Some(state);
            return result;
        }
        if let Some(state) = self.full_bank_census.take() {
            let result = state.begin(i.cache_metadata[0], || {
                // SAFETY: this owner selected only the separate bank-scoped
                // Full2303 route; Roster and runtime retain actual bank proofs.
                unsafe {
                    self.base.states.begin_full2303_bank_scoped(
                        &mut self.base.owner.catalog.backend,
                        i.registration,
                        self.base.owner.catalog.scope.model_id,
                        i.generation,
                        i.cache_metadata[0],
                        self.base.profile.timeout_ms,
                    )
                }
                .map_err(io::Error::other)
            });
            self.full_bank_census = Some(state);
            return result;
        }
        if let Some(state) = self.tail_scoped.take() {
            #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
            let result = state.begin(i.cache_metadata[0], || {
                // SAFETY: this owner selected only the separate bank-scoped
                // Position5 route; Roster and runtime retain actual bank proofs.
                unsafe {
                    self.base.states.begin_bank_scoped(
                        &mut self.base.owner.catalog.backend,
                        i.registration,
                        self.base.owner.catalog.scope.model_id,
                        i.generation,
                        i.cache_metadata[0],
                        self.base.profile.timeout_ms,
                    )
                }
                .map_err(io::Error::other)
            });
            #[cfg(feature = "engineering-currentness-duration-diagnostics")]
            let result = state.begin_diagnostic(i.cache_metadata[0], || {
                // SAFETY: identical authenticated Tail bank custody; only the returned diagnostics differ.
                unsafe {
                    self.base.states.begin_bank_scoped_diagnostic(
                        &mut self.base.owner.catalog.backend,
                        i.registration,
                        self.base.owner.catalog.scope.model_id,
                        i.generation,
                        i.cache_metadata[0],
                        self.base.profile.timeout_ms,
                    )
                }
                .map_err(io::Error::other)
            });
            self.tail_scoped = Some(state);
            return result;
        }
        if let Some(state) = self.census_scoped.take() {
            let result = state.begin(i.cache_metadata[0], || {
                // SAFETY: this owner selected only the separate bank-scoped
                // Position5 route; Roster and runtime retain actual bank proofs.
                unsafe {
                    self.base.states.begin_bank_scoped(
                        &mut self.base.owner.catalog.backend,
                        i.registration,
                        self.base.owner.catalog.scope.model_id,
                        i.generation,
                        i.cache_metadata[0],
                        self.base.profile.timeout_ms,
                    )
                }
                .map_err(io::Error::other)
            });
            self.census_scoped = Some(state);
            return result;
        }
        if let Some(state) = self.bank_scoped.take() {
            let result = state.begin(i.cache_metadata[0], || {
                // SAFETY: this owner selected only the separate bank-scoped
                // Position5 route; Roster and runtime retain actual bank proofs.
                unsafe {
                    self.base.states.begin_bank_scoped(
                        &mut self.base.owner.catalog.backend,
                        i.registration,
                        self.base.owner.catalog.scope.model_id,
                        i.generation,
                        i.cache_metadata[0],
                        self.base.profile.timeout_ms,
                    )
                }
                .map_err(io::Error::other)
            });
            self.bank_scoped = Some(state);
            return result;
        }
        self.base
            .states
            .begin(
                &mut self.base.owner.catalog.backend,
                i.registration,
                self.base.owner.catalog.scope.model_id,
                i.generation,
                i.cache_metadata[0],
                self.base.profile.timeout_ms,
            )
            .map_err(io::Error::other)
    }
    fn layer(&mut self, index: usize) -> io::Result<LayerObservation> {
        require(
            index == self.hidden.len() && index < 36,
            "readiness exact hidden layer order",
        )?;
        let input = self
            .input
            .as_ref()
            .ok_or_else(|| io::Error::other("causal metadata missing"))?;
        let position = input.cache_metadata[0];
        let mut capture = if self.causal.is_some() && index == 0 && position < 6 {
            Some(
                crate::resident_layer::capture_v1::causal_v1::Collector::new(
                    input.generation,
                    position,
                    index,
                )
                .map_err(io::Error::other)?,
            )
        } else {
            None
        };
        let executed = if let Some(state) = self.full_bank_census_tail.take() {
            let result = state.dispatch(position, index, |warm| {
                let value = self.execute_full2303_census_layer(index, warm)?;
                Ok(((value.completion, value.hidden), value.counters))
            });
            self.full_bank_census_tail = Some(state);
            result
        } else if let Some(state) = self.full_bank_census.take() {
            let result = state.dispatch(position, index, |warm| {
                let value = self.execute_full2303_census_layer(index, warm)?;
                Ok(((value.completion, value.hidden), value.counters))
            });
            self.full_bank_census = Some(state);
            result
        } else if let Some(state) = self.tail_scoped.take() {
            #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
            let result = state.dispatch(position, index, |warm| {
                let value = self.execute_census_layer(index, warm)?;
                Ok(((value.completion, value.hidden), value.counters))
            });
            #[cfg(feature = "engineering-currentness-duration-diagnostics")]
            let result = state.dispatch_diagnostic(position, index, |warm| {
                let value = self.execute_census_layer(index, warm)?;
                Ok((
                    (value.completion, value.hidden),
                    value.counters,
                    value.durations,
                    value.layer_durations,
                ))
            });
            self.tail_scoped = Some(state);
            result
        } else if let Some(state) = self.census_scoped.take() {
            let result = state.dispatch(position, index, |warm| {
                let value = self.execute_census_layer(index, warm)?;
                Ok(((value.completion, value.hidden), value.counters))
            });
            self.census_scoped = Some(state);
            result
        } else if let Some(state) = self.bank_scoped.take() {
            let result = state.dispatch(position, index, |warm| {
                let (done, hidden, counters) = self.execute_layer(index, capture.as_mut(), warm)?;
                Ok(((done, hidden), counters))
            });
            self.bank_scoped = Some(state);
            result
        } else {
            let selected = self.scoped.take();
            match selected {
                Some(state) => {
                    let result = state.dispatch(position, index, |warm| {
                        let (done, hidden, counters) =
                            self.execute_layer(index, capture.as_mut(), warm)?;
                        Ok(((done, hidden), counters))
                    });
                    self.scoped = Some(state);
                    result
                }
                None => match self.full_scoped.take() {
                    Some(state) => {
                        let result = state.dispatch(position, index, |warm| {
                            let (done, hidden, counters) =
                                self.execute_layer(index, capture.as_mut(), warm)?;
                            Ok(((done, hidden), counters))
                        });
                        self.full_scoped = Some(state);
                        result
                    }
                    None => self
                        .execute_layer(index, capture.as_mut(), false)
                        .map(|(done, hidden, _)| (done, hidden)),
                },
            }
        };
        let (done, hidden) = executed?;
        if let Some(capture) = capture {
            self.causal
                .as_mut()
                .ok_or_else(|| io::Error::other("causal collector disappeared"))?
                .push(capture.finish().map_err(io::Error::other)?, &hidden)
                .map_err(io::Error::other)?;
        }
        self.hidden.push(hidden);
        Ok(LayerObservation {
            prefix_states: done.prefix_states,
            mlp_prefixes: done.guarded.prefixes,
            guards: done.guarded.guards,
            prefix_host_ns: done.prefix_ns,
            segment_host_ns: done.guarded.segment_host_ns,
            observed_queue_frontiers: done.guarded.observed_queue_frontiers,
        })
    }
    fn tail(&mut self) -> io::Result<Tail> {
        require(
            self.hidden.len() == 36 && self.hidden.iter().all(|row| row.len() == 8192),
            "readiness complete hidden output roster",
        )?;
        let position = self
            .input
            .as_ref()
            .ok_or_else(|| io::Error::other("tail metadata missing"))?
            .cache_metadata[0];
        let result = if let Some(state) = self.full_bank_census_tail.take() {
            let result = state.tail(position, |warm| {
                self.execute_full2303_tail(warm)
                    .map(|v| (v.value, v.counters))
            });
            self.full_bank_census_tail = Some(state);
            result
        } else if let Some(state) = self.tail_scoped.take() {
            #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
            let result = state.tail(position, |warm| {
                self.execute_tail(warm).map(|v| (v.value, v.counters))
            });
            #[cfg(feature = "engineering-currentness-duration-diagnostics")]
            let result = state.tail_diagnostic(position, |warm| {
                self.execute_tail(warm)
                    .map(|v| (v.value, v.counters, v.durations))
            });
            self.tail_scoped = Some(state);
            result
        } else {
            self.execute_tail(false).map(|v| v.value)
        };
        let (output_token, host_ns, normalized, logits) = result?;
        require(
            normalized.len() == 8192 && logits.len() == 303872,
            "readiness actual tail readback extents",
        )?;
        let mut observation = Vec::with_capacity(crate::finite_forward_wire_v1::OBSERVATION_BYTES);
        for row in self.hidden.drain(..) {
            observation.extend_from_slice(&row);
        }
        observation.extend_from_slice(&normalized);
        observation.extend_from_slice(&logits);
        Ok(Tail {
            output_token,
            host_ns,
            observation,
        })
    }
    fn fence(&mut self) -> io::Result<()> {
        self.native()?.fence().map_err(io::Error::other)
    }
    fn commit(&mut self, generation: u64) -> io::Result<()> {
        require(
            self.input
                .as_ref()
                .is_some_and(|i| i.generation == generation),
            "readiness exact commit generation",
        )?;
        self.native()?.commit().map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<()> {
        require(
            match self.profile {
                Profile::Full2303 => {
                    self.base.states.full2303()
                        && self.base.states.between()
                        && self.base.states.completed() == 2303
                        && self.base.states.counts() == REUSE_MAX_COUNTS
                }
                Profile::Readiness40 | Profile::Readiness40Position5 => {
                    self.base.states.readiness40()
                        && self.base.states.between()
                        && self.base.states.completed() == 40
                        && self.base.states.counts() == REUSE_MAX_COUNTS
                }
            },
            "closed native extent requires every committed forward",
        )?;
        require(
            self.base
                .owner
                .catalog
                .backend
                .preflight_additional_allocations_v1(&[0, 0])
                .map_err(io::Error::other)?
                == REUSE_MAX_COUNTS,
            "readiness Close allocation census",
        )?;
        self.check_deadline()?;
        self.base.owner.close_setup().map_err(io::Error::other)
    }
    fn poison(&mut self) {
        poison(self.base);
    }
}

#[cfg(test)]
#[path = "native_guarded_mlp_readiness_v1_tests.rs"]
mod tests;
