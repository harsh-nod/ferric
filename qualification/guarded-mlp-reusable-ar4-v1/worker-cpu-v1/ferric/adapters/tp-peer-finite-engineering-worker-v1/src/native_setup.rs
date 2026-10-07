//! Private bounded setup, not a production admission or forward execution API.

use crate::finite_composition_wire as source;
use crate::finite_setup_wire_v1 as wire;
use crate::native_catalog::{BindingKey, NativeOwner, SourceScope};
use crate::resident_artifacts::ReviewedImages;
use crate::tail_artifacts::ReviewedTailImages;
use crate::tail_head::{HEAD_BYTES, HeadManifest};
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

type Result<T> = core::result::Result<T, String>;

pub(crate) mod guarded_mlp_decode_v1;
pub(crate) mod prefix_decode_v6;
pub(crate) mod prefix_layer_v6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrefixExecution {
    LayerOnly,
    DecodeFour,
    GuardedDecodeFour,
    GuardedReuseFour,
}
impl PrefixExecution {
    fn validate_images(self, mlp: bool, prefix: bool) -> Result<()> {
        if (prefix && !mlp)
            || (matches!(
                self,
                Self::DecodeFour | Self::GuardedDecodeFour | Self::GuardedReuseFour
            ) && !(prefix && mlp))
        {
            return Err("explicit prefix execution requires both selected images".into());
        }
        Ok(())
    }
}

fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn require(value: bool, message: &str) -> Result<()> {
    if value { Ok(()) } else { Err(message.into()) }
}

fn native_key(key: wire::Key) -> BindingKey {
    match key {
        wire::Key::Source { rank, id } => BindingKey::Source { rank, id },
        wire::Key::Pending { rank, layer, role } => BindingKey::Pending {
            rank,
            layer,
            kind: role,
        },
    }
}

#[derive(Clone)]
struct Planned {
    key: wire::Key,
    bytes: u64,
    // None denotes the existing owner's explicit zero-initialization path.
    digest: Option<[u8; 32]>,
}

fn plan(
    registration: &source::Registration,
    uploads: &wire::UploadManifest,
) -> Result<Vec<Planned>> {
    registration.validate().map_err(|e| e.to_string())?;
    require(
        uploads.version == 1 && uploads.uploads.len() == 939,
        "immutable upload roster version/count",
    )?;
    let mut requirements = Vec::with_capacity(1135);
    let mut add = |key, elements: u64, width: u32, immutable| -> Result<()> {
        let bytes = elements
            .checked_mul(u64::from(width))
            .ok_or("setup extent overflow")?;
        require(
            !requirements.iter().any(|(old, _, _)| *old == key),
            "duplicate setup key",
        )?;
        requirements.push((key, bytes, immutable));
        Ok(())
    };
    let key = |b: source::Buffer| wire::Key::Source {
        rank: b.rank,
        id: b.id,
    };
    for layer in &registration.layers {
        for weight in &layer.weights {
            let b = weight.buffer;
            add(key(b), b.elements, b.element_bytes, true)?;
        }
        for b in layer.caches {
            add(key(b), b.elements, b.element_bytes, false)?;
        }
    }
    for row in &registration.globals {
        let b = row.buffer;
        add(key(b), b.elements, b.element_bytes, true)?;
    }
    for b in registration
        .scratch
        .iter()
        .map(|v| v.buffer)
        .chain(registration.auxiliary.iter().map(|v| v.buffer))
    {
        add(key(b), b.elements, b.element_bytes, false)?;
    }
    for row in &registration.pending_buffers {
        let immutable = matches!(
            row.kind,
            source::PendingKind::PackedQkvWeight | source::PendingKind::PackedHeadNormWeight
        );
        add(
            wire::Key::Pending {
                rank: row.rank,
                layer: row.layer,
                role: row.kind,
            },
            row.elements,
            row.element_bytes,
            immutable,
        )?;
    }
    require(
        requirements.len() == 1135,
        "setup complete catalog cardinality",
    )?;
    let mut result = Vec::with_capacity(requirements.len());
    for (key, bytes, immutable) in requirements {
        let mut matches = uploads.uploads.iter().filter(|row| row.key == key);
        let supplied = matches.next();
        require(matches.next().is_none(), "duplicate immutable upload")?;
        let digest = if immutable {
            let row = supplied.ok_or("missing immutable upload")?;
            require(
                row.bytes == bytes && row.sha256 != [0; 32],
                "immutable upload extent/digest",
            )?;
            Some(row.sha256)
        } else {
            require(
                supplied.is_none(),
                "mutable storage cannot be an immutable upload",
            )?;
            None
        };
        result.push(Planned { key, bytes, digest });
    }
    require(
        result.iter().filter(|v| v.digest.is_some()).count() == uploads.uploads.len(),
        "extra immutable upload",
    )?;
    if let Some(tail) = &uploads.tail {
        let original = registration
            .globals
            .iter()
            .find(|v| v.kind == source::GlobalKind::LanguageModelHead)
            .ok_or("original head missing")?
            .buffer;
        let expected_key = key(original);
        let original_upload = result
            .iter()
            .find(|v| v.key == expected_key)
            .ok_or("head upload missing")?;
        require(
            tail.source == expected_key
                && original.rank == 0
                && tail.bytes == HEAD_BYTES
                && original_upload.bytes == HEAD_BYTES
                && original_upload.digest == Some(tail.source_sha256)
                && tail.sha256 != [0; 32],
            "distinct tail transpose source/output manifest",
        )?;
    }
    Ok(result)
}

fn validate_scope(
    begin: &wire::Begin,
    registration: &source::Registration,
    expected: &wire::Scope,
    process_id: u32,
) -> Result<()> {
    require(
        begin.scope == *expected
            && expected.child_identity == process_id
            && expected.child_identity != 0
            && expected.bundle_id != [0; 32]
            && expected.model_id != [0; 32]
            && expected.session != [0; 32],
        "setup bootstrap/process scope",
    )?;
    require(
        registration.bundle_id == expected.bundle_id
            && registration.model_id == expected.model_id
            && registration.session == expected.session
            && registration.pool_identity == expected.pool_identity
            && registration.group_id == expected.group_id
            && registration.child_identity == expected.child_identity,
        "setup registration scope",
    )
}

/// Construct this before opening any device. Expected scope must come from the
/// source-owning parent/bootstrap, not from copying the incoming Begin fields.
/// Source-program bytes are digest-bound here; grammar/authentication remain the
/// parent adapter's responsibility, not an authority manufactured by this wire.
pub(super) struct PreparedSetup {
    devices: [u64; 2],
    scope: wire::Scope,
    registration: source::Registration,
    registration_bytes: Vec<u8>,
    program: Vec<u8>,
    plan: Vec<Planned>,
    tail: Option<wire::TailHeadTranspose>,
    images: ReviewedImages,
    tail_images: Option<ReviewedTailImages>,
}

impl PreparedSetup {
    pub(super) fn prepare(
        request: wire::Request,
        payload: Vec<u8>,
        expected_devices: [u64; 2],
        expected_scope: &wire::Scope,
    ) -> Result<Self> {
        require(
            request.payload_bytes().map_err(|e| e.to_string())? == payload.len()
                && request.id == 1
                && request.device_ids == expected_devices,
            "setup Begin envelope/payload",
        )?;
        let wire::Command::Begin(begin) = request.command else {
            return Err("setup requires Begin".into());
        };
        let mut parts = Vec::with_capacity(7);
        let mut offset = 0_usize;
        for part in begin.parts().into_iter().chain(begin.tail_image) {
            let end = offset
                .checked_add(part.bytes as usize)
                .ok_or("setup part overflow")?;
            let bytes = payload.get(offset..end).ok_or("setup truncated part")?;
            require(hash(bytes) == part.sha256, "setup payload part digest")?;
            parts.push(bytes);
            offset = end;
        }
        require(offset == payload.len(), "setup trailing payload")?;
        let registration: source::Registration =
            serde_json::from_slice(parts[0]).map_err(|e| e.to_string())?;
        validate_scope(&begin, &registration, expected_scope, std::process::id())?;
        require(
            registration.source_program_bytes as usize == parts[1].len()
                && registration.source_program_sha256 == hash(parts[1]),
            "source-program exact bytes/digest",
        )?;
        let uploads: wire::UploadManifest =
            serde_json::from_slice(parts[2]).map_err(|e| e.to_string())?;
        let plan = plan(&registration, &uploads)?;
        require(
            uploads.tail.is_some() == begin.tail_image.is_some(),
            "explicit tail/image agreement",
        )?;
        let images = ReviewedImages::new(parts[3].to_vec(), parts[4].to_vec(), parts[5].to_vec())?;
        let tail_images = if uploads.tail.is_some() {
            Some(ReviewedTailImages::new(
                parts[6].to_vec(),
                parts[5].to_vec(),
            )?)
        } else {
            None
        };
        Ok(Self {
            devices: request.device_ids,
            scope: begin.scope,
            registration,
            registration_bytes: parts[0].to_vec(),
            program: parts[1].to_vec(),
            plan,
            tail: uploads.tail,
            images,
            tail_images,
        })
    }

    pub(super) fn device_ids(&self) -> [u64; 2] {
        self.devices
    }

    /// The audited opener must open exactly device_ids() in rank order. No raw
    /// Group or token is ever returned by this processor.
    pub(super) fn into_processor(self, group: Group) -> Result<NativeSetup> {
        self.into_processor_selected(group, None)
    }

    fn into_processor_selected(
        self,
        group: Group,
        tiles: Option<crate::resident_layer::mlp_tiles_v2::Image>,
    ) -> Result<NativeSetup> {
        self.into_processor_profiles(group, tiles, None)
    }

    fn into_processor_profiles(
        self,
        group: Group,
        tiles: Option<crate::resident_layer::mlp_tiles_v2::Image>,
        prefix: Option<crate::resident_layer::prefix_tiles_v6::artifacts::Image>,
    ) -> Result<NativeSetup> {
        self.into_processor_execution(group, tiles, prefix, PrefixExecution::LayerOnly)
    }

    fn into_processor_execution(
        self,
        group: Group,
        tiles: Option<crate::resident_layer::mlp_tiles_v2::Image>,
        prefix: Option<crate::resident_layer::prefix_tiles_v6::artifacts::Image>,
        execution: PrefixExecution,
    ) -> Result<NativeSetup> {
        self.into_processor_with_guarded_images(group, tiles, prefix, execution, None)
    }

    fn into_processor_with_guarded_images(
        self,
        group: Group,
        tiles: Option<crate::resident_layer::mlp_tiles_v2::Image>,
        prefix: Option<crate::resident_layer::prefix_tiles_v6::artifacts::Image>,
        execution: PrefixExecution,
        guarded: Option<crate::resident_layer::guarded_mlp_decode_v1::Images>,
    ) -> Result<NativeSetup> {
        execution.validate_images(tiles.is_some(), prefix.is_some())?;
        if matches!(
            execution,
            PrefixExecution::GuardedDecodeFour | PrefixExecution::GuardedReuseFour
        ) != guarded.is_some()
        {
            return Err("guarded setup requires exact distinct image selection".into());
        }
        let scope = SourceScope {
            bundle_id: self.scope.bundle_id,
            model_id: self.scope.model_id,
            session: self.scope.session,
            pool_identity: self.scope.pool_identity,
            group_id: self.scope.group_id,
            child_identity: self.scope.child_identity,
        };
        let mut owner = NativeOwner::from_reviewed_group(
            group,
            scope,
            self.registration,
            &self.registration_bytes,
            &self.program,
        )?;
        match (tiles, prefix) {
            (Some(mlp), Some(prefix)) => match execution {
                PrefixExecution::LayerOnly => owner.select_prefix_tiles_layer(prefix, mlp)?,
                PrefixExecution::DecodeFour => owner.select_prefix_tiles_decode(prefix, mlp)?,
                PrefixExecution::GuardedDecodeFour | PrefixExecution::GuardedReuseFour => owner
                    .select_guarded_mlp_decode(
                        prefix,
                        mlp,
                        guarded.ok_or("guarded setup images missing")?,
                        execution == PrefixExecution::GuardedReuseFour,
                    )?,
            },
            (Some(mlp), None) => owner.select_tiles_decode(mlp)?,
            (None, None) => (),
            (None, Some(_)) => return Err("prefix layer requires distinct MLP548 image".into()),
        }
        if let Some(tail) = &self.tail {
            let wire::Key::Source { rank: 0, id } = tail.source else {
                return Err("tail original source key".into());
            };
            owner.prepare_tail_setup(HeadManifest {
                source_id: id,
                source_sha256: tail.source_sha256,
                output_sha256: tail.sha256,
            })?;
        }
        Ok(NativeSetup {
            core: Core::new(
                Native {
                    owner,
                    images: Some(self.images),
                    tail_images: self.tail_images,
                },
                self.devices,
                self.scope.session,
                self.plan,
                self.tail,
            ),
        })
    }
}

/// CPU-only profile preparation must finish before the caller opens the Group.
/// It does not grant source, image-review, runtime, or production authority.
pub(super) struct PreparedTilesSetup {
    setup: PreparedSetup,
    image: crate::resident_layer::mlp_tiles_v2::Image,
    profile: crate::native_catalog::forward::tiles_decode_v1::Profile,
}
impl PreparedTilesSetup {
    pub(super) fn new(
        setup: PreparedSetup,
        image: crate::resident_layer::mlp_tiles_v2::Image,
        mode: crate::native_catalog::forward::tiles_decode_v1::Mode,
        timeout_ms: u32,
    ) -> Result<Self> {
        Self::new_with_admission(
            setup,
            image,
            mode,
            timeout_ms,
            crate::finite_tiles_decode_wire_v1::KernelAdmission::Baseline,
        )
    }
    pub(super) fn new_with_admission(
        setup: PreparedSetup,
        image: crate::resident_layer::mlp_tiles_v2::Image,
        mode: crate::native_catalog::forward::tiles_decode_v1::Mode,
        timeout_ms: u32,
        admission: crate::finite_tiles_decode_wire_v1::KernelAdmission,
    ) -> Result<Self> {
        if setup.tail.is_none() || setup.tail_images.is_none() {
            return Err("tiles decode requires complete tail before native open".into());
        }
        let profile = crate::native_catalog::forward::tiles_decode_v1::Profile::new_with_admission(
            &setup.scope,
            hash(&setup.registration_bytes),
            image.sha256(),
            mode,
            timeout_ms,
            setup.devices,
            admission,
        )?;
        Ok(Self {
            setup,
            image,
            profile,
        })
    }
    pub(super) fn device_ids(&self) -> [u64; 2] {
        self.setup.devices
    }
    pub(super) fn profile_sha256(&self) -> [u8; 32] {
        self.profile.sha256()
    }
    pub(super) fn into_processor(mut self, mut group: Group) -> Result<TilesSetup> {
        self.profile.configure_admission(|cache, operational| {
            group.configure_performance(cache, operational)
        })?;
        Ok(TilesSetup {
            inner: self
                .setup
                .into_processor_selected(group, Some(self.image))?,
            profile: self.profile,
        })
    }
}
pub(super) struct TilesSetup {
    inner: NativeSetup,
    profile: crate::native_catalog::forward::tiles_decode_v1::Profile,
}
impl TilesSetup {
    pub(super) fn applied_admission(
        &self,
    ) -> Result<Option<crate::finite_tiles_decode_wire_v1::AdmissionReceipt>> {
        self.profile.applied_admission()
    }
    pub(super) fn serve(&mut self, r: &mut impl Read, w: &mut impl Write) -> Result<()> {
        self.inner.serve(r, w)
    }
    pub(super) fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
    /// # Safety
    /// Actual parent source/model and supplied V2 image review premises must
    /// hold for this exact retained owner; the profile digest is not authority.
    #[allow(unsafe_code)]
    pub(super) unsafe fn into_decode(
        self,
    ) -> Result<crate::native_catalog::forward::tiles_decode_v1::Owner> {
        unsafe {
            crate::native_catalog::forward::tiles_decode_v1::Owner::from_sealed(
                self.inner.into_forward_owner()?,
                self.profile,
            )
        }
    }
}

// Static CPU-test seam only. All actual buffers/states remain in NativeOwner.
trait Backend {
    fn allocate(&mut self, row: &Planned) -> Result<u64>;
    fn write(&mut self, id: u64, offset: u64, bytes: &[u8]) -> Result<()>;
    fn allocate_tail(&mut self) -> Result<()>;
    fn write_tail(&mut self, offset: u64, bytes: &[u8]) -> Result<()>;
    fn load(&mut self) -> Result<()>;
    fn bind_seal(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
}

struct Native {
    owner: NativeOwner,
    images: Option<ReviewedImages>,
    tail_images: Option<ReviewedTailImages>,
}
impl Backend for Native {
    fn allocate(&mut self, row: &Planned) -> Result<u64> {
        match row.digest {
            Some(digest) => self.owner.allocate_upload(native_key(row.key), digest),
            None => self.owner.allocate_zeroed(native_key(row.key)),
        }
    }
    fn write(&mut self, id: u64, offset: u64, bytes: &[u8]) -> Result<()> {
        self.owner.write_upload(id, offset, bytes)
    }
    fn allocate_tail(&mut self) -> Result<()> {
        self.owner.allocate_tail_head()
    }
    fn write_tail(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        self.owner.write_tail_head(offset, bytes)
    }
    fn load(&mut self) -> Result<()> {
        self.owner.load_resident_artifacts(
            self.images
                .take()
                .ok_or("resident images already consumed")?,
        )?;
        if let Some(images) = self.tail_images.take() {
            self.owner.load_tail_artifacts(images)?;
        }
        Ok(())
    }
    fn bind_seal(&mut self) -> Result<()> {
        self.owner.bind_catalog()?;
        self.owner.seal_layer_bindings_after_source_intake()
    }
    fn close(&mut self) -> Result<()> {
        self.owner.close_setup()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Uploading,
    Sealed,
    Closed,
    Terminal,
}

struct Core<B> {
    backend: B,
    devices: [u64; 2],
    session: [u8; 32],
    plan: Vec<Planned>,
    // Wire keys map only to IDs minted by this owner, never to imported tokens.
    allocated: Vec<(wire::Key, u64)>,
    tail: Option<wire::TailHeadTranspose>,
    tail_allocated: bool,
    loaded: bool,
    next: u64,
    phase: Phase,
}

impl<B: Backend> Core<B> {
    fn new(
        backend: B,
        devices: [u64; 2],
        session: [u8; 32],
        plan: Vec<Planned>,
        tail: Option<wire::TailHeadTranspose>,
    ) -> Self {
        Self {
            backend,
            devices,
            session,
            plan,
            allocated: Vec::new(),
            tail,
            tail_allocated: false,
            loaded: false,
            next: 2,
            phase: Phase::Uploading,
        }
    }
    fn poison(&mut self) {
        self.phase = Phase::Terminal;
    }
    fn forward_extractable(&self) -> bool {
        self.phase == Phase::Sealed && self.tail.is_some()
    }
    fn response(id: u64, status: wire::Status, catalog_id: Option<u64>) -> wire::Response {
        wire::Response {
            protocol: wire::PROTOCOL,
            id,
            status,
            catalog_id,
            native_opened: true,
            gpu_execution: false,
            forward_started: false,
            production_authority: false,
        }
    }
    fn handle(&mut self, request: wire::Request, payload: &[u8]) -> Result<wire::Response> {
        let result = self.handle_inner(request, payload);
        if result.is_err() {
            self.poison();
        }
        result
    }
    fn handle_inner(&mut self, request: wire::Request, payload: &[u8]) -> Result<wire::Response> {
        require(
            self.phase == Phase::Uploading
                && request.id == self.next
                && request.device_ids == self.devices
                && request.session == self.session,
            "setup phase/sequence/scope",
        )?;
        require(
            request.payload_bytes().map_err(|e| e.to_string())? == payload.len(),
            "setup command payload extent",
        )?;
        let mut catalog_id = None;
        let status = match request.command {
            wire::Command::Allocate { key } => {
                require(
                    !self.allocated.iter().any(|(old, _)| *old == key),
                    "setup repeated allocation",
                )?;
                let row = self
                    .plan
                    .iter()
                    .find(|row| row.key == key)
                    .ok_or("setup key outside source plan")?;
                let id = self.backend.allocate(row)?;
                require(
                    id != 0 && !self.allocated.iter().any(|(_, old)| *old == id),
                    "setup native catalog identity",
                )?;
                self.allocated.push((key, id));
                catalog_id = Some(id);
                wire::Status::Allocated
            }
            wire::Command::Write { key, offset, part } => {
                let row = self
                    .plan
                    .iter()
                    .find(|row| row.key == key)
                    .ok_or("setup unknown upload key")?;
                require(
                    row.digest.is_some()
                        && hash(payload) == part.sha256
                        && offset
                            .checked_add(payload.len() as u64)
                            .is_some_and(|end| end <= row.bytes),
                    "setup immutable chunk/hash/extent",
                )?;
                let id = self
                    .allocated
                    .iter()
                    .find(|(old, _)| *old == key)
                    .ok_or("setup upload before allocation")?
                    .1;
                self.backend.write(id, offset, payload)?;
                wire::Status::Uploaded
            }
            wire::Command::AllocateTailHead => {
                require(
                    self.tail.is_some() && !self.tail_allocated,
                    "setup undeclared/repeated tail",
                )?;
                self.backend.allocate_tail()?;
                self.tail_allocated = true;
                wire::Status::TailHeadAllocated
            }
            wire::Command::WriteTailHead { offset, part } => {
                let tail = self.tail.as_ref().ok_or("setup undeclared tail upload")?;
                require(
                    self.tail_allocated
                        && hash(payload) == part.sha256
                        && offset
                            .checked_add(payload.len() as u64)
                            .is_some_and(|end| end <= tail.bytes),
                    "setup tail chunk/hash/extent",
                )?;
                self.backend.write_tail(offset, payload)?;
                wire::Status::TailHeadUploaded
            }
            wire::Command::LoadArtifacts => {
                require(!self.loaded, "setup repeated image load")?;
                self.backend.load()?;
                self.loaded = true;
                wire::Status::ArtifactsLoaded
            }
            wire::Command::BindAndSeal => {
                require(
                    self.allocated.len() == self.plan.len()
                        && self.loaded
                        && (self.tail.is_none() || self.tail_allocated),
                    "setup incomplete allocation/image roster",
                )?;
                // The owner checks all final digests, typed state initialization,
                // actual artifact/root joins and tail completion before sealing.
                self.backend.bind_seal()?;
                self.phase = Phase::Sealed;
                if self.tail.is_some() {
                    wire::Status::LayersAndTailSealed
                } else {
                    wire::Status::LayersSealed
                }
            }
            wire::Command::AbortClose => {
                self.backend.close()?;
                self.phase = Phase::Closed;
                wire::Status::Closed
            }
            wire::Command::Begin(_) => return Err("setup repeated Begin".into()),
        };
        self.next = self.next.checked_add(1).ok_or("setup sequence overflow")?;
        Ok(Self::response(request.id, status, catalog_id))
    }
    fn serve(&mut self, reader: &mut impl Read, writer: &mut impl Write) -> Result<()> {
        let result = (|| {
            require(
                self.phase == Phase::Uploading && self.next == 2,
                "setup stream already started",
            )?;
            wire::write_response(writer, &Self::response(1, wire::Status::OwnerCreated, None))
                .map_err(|e| e.to_string())?;
            loop {
                let (request, payload) = wire::read_request(reader)
                    .map_err(|e| e.to_string())?
                    .ok_or("setup EOF before seal/close")?;
                let response = self.handle(request, &payload)?;
                wire::write_response(writer, &response).map_err(|e| e.to_string())?;
                if self.phase != Phase::Uploading {
                    return Ok(());
                }
            }
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
}

pub(super) struct NativeSetup {
    core: Core<Native>,
}
impl NativeSetup {
    /// Reader starts after the fully validated Begin frame. Stops at the first
    /// seal/close; it never reads or interprets a forward command.
    pub(super) fn serve(&mut self, reader: &mut impl Read, writer: &mut impl Write) -> Result<()> {
        self.core.serve(reader, writer)
    }
    /// True only after successful native close and its flushed acknowledgement.
    pub(super) fn is_closed(&self) -> bool {
        self.core.phase == Phase::Closed
    }
    /// Consuming handoff only; a layer-only setup cannot become a full-forward
    /// owner. No successful setup response is a Ready or production claim.
    pub(super) fn into_forward_owner(self) -> Result<NativeOwner> {
        require(
            self.core.forward_extractable(),
            "full forward requires acknowledged layers-and-tail seal",
        )?;
        Ok(self.core.backend.owner)
    }
}

#[cfg(test)]
#[path = "native_setup_tests.rs"]
mod tests;
