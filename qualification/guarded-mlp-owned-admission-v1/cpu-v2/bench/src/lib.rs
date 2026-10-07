#![forbid(unsafe_code)]

use fe2o3_amdhsa_loader::{
    AdmittedProfile, OwnedValidatedKernelEnvelope, ValidatedKernelEnvelope, validate,
    validate_owned,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File, Metadata};
use std::hint::black_box;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{Duration, Instant};

pub type Result<T> = std::result::Result<T, String>;
pub const LAYERS: usize = 36;
pub const WARMUP_PAIRS: usize = 2;
pub const SAMPLE_PAIRS: usize = 12;
pub const WALL_SECONDS: u64 = 60;
pub const REQUIRED_CPU_SECONDS: u64 = 45;
pub const REQUIRED_ADDRESS_SPACE_BYTES: u64 = 256 << 20;
pub const MAX_OUTPUT_BYTES: usize = 128 << 10;
const PROFILE: AdmittedProfile = AdmittedProfile::Gfx950XnackOffCov6;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ImagePin {
    pub file: &'static str,
    pub bytes: usize,
    pub sha256: &'static str,
}

pub const IMAGES: [ImagePin; 4] = [
    ImagePin {
        file: "prefix.hsaco",
        bytes: 54344,
        sha256: "29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8",
    },
    ImagePin {
        file: "projection.hsaco",
        bytes: 10864,
        sha256: "25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25",
    },
    ImagePin {
        file: "mlp.hsaco",
        bytes: 33320,
        sha256: "b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589",
    },
    ImagePin {
        file: "guarded.hsaco",
        bytes: 28440,
        sha256: "de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Role {
    pub rank: u8,
    pub stage: &'static str,
    pub image: usize,
    pub symbol: &'static str,
}

pub const ROLES: [Role; 10] = [
    Role {
        rank: 0,
        stage: "prefix",
        image: 0,
        symbol: "ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6",
    },
    Role {
        rank: 1,
        stage: "prefix",
        image: 0,
        symbol: "ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6",
    },
    Role {
        rank: 0,
        stage: "r1",
        image: 1,
        symbol: "ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1",
    },
    Role {
        rank: 0,
        stage: "mlp",
        image: 2,
        symbol: "ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2",
    },
    Role {
        rank: 0,
        stage: "guard",
        image: 3,
        symbol: "ferric_qwen3_mlp_state_guard_v2",
    },
    Role {
        rank: 0,
        stage: "r2",
        image: 3,
        symbol: "ferric_qwen3_tp2_guarded_projection_residual_bf16_v2",
    },
    Role {
        rank: 1,
        stage: "r1",
        image: 1,
        symbol: "ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1",
    },
    Role {
        rank: 1,
        stage: "mlp",
        image: 2,
        symbol: "ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2",
    },
    Role {
        rank: 1,
        stage: "guard",
        image: 3,
        symbol: "ferric_qwen3_mlp_state_guard_v2",
    },
    Role {
        rank: 1,
        stage: "r2",
        image: 3,
        symbol: "ferric_qwen3_tp2_guarded_projection_residual_bf16_v2",
    },
];

fn check(ok: bool, what: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(what.to_owned()) }
}

fn deadline(now: Instant, until: Instant) -> Result<()> {
    check(now < until, "fixed benchmark wall deadline exceeded")
}

fn nanos(duration: Duration) -> Result<u64> {
    u64::try_from(duration.as_nanos()).map_err(|_| "nanosecond overflow".into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn authenticate_bytes(bytes: &[u8], pin: ImagePin) -> Result<()> {
    check(bytes.len() == pin.bytes, "image exact extent")?;
    check(
        hex(&Sha256::digest(bytes)) == pin.sha256,
        "image original SHA256",
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FileIdentity {
    device: u64,
    inode: u64,
    bytes: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}

fn identity(metadata: &Metadata) -> Result<FileIdentity> {
    check(
        metadata.is_file() && metadata.nlink() == 1,
        "regular single-link image required",
    )?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        bytes: metadata.len(),
        mode: metadata.mode(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        links: metadata.nlink(),
        mtime: (metadata.mtime(), metadata.mtime_nsec()),
        ctime: (metadata.ctime(), metadata.ctime_nsec()),
    })
}

fn read_image(root: &Path, pin: ImagePin) -> Result<(Vec<u8>, FileIdentity)> {
    let path = root.join(pin.file);
    let before = identity(&fs::symlink_metadata(&path).map_err(|e| e.to_string())?)?;
    check(before.bytes == pin.bytes as u64, "image metadata extent")?;
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    check(
        identity(&file.metadata().map_err(|e| e.to_string())?)? == before,
        "image open identity drift",
    )?;
    let mut bytes = Vec::with_capacity(pin.bytes);
    (&mut file)
        .take(pin.bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    authenticate_bytes(&bytes, pin)?;
    check(
        identity(&file.metadata().map_err(|e| e.to_string())?)? == before,
        "image fd drift",
    )?;
    check(
        identity(&fs::symlink_metadata(&path).map_err(|e| e.to_string())?)? == before,
        "image path drift",
    )?;
    Ok((bytes, before))
}

fn fresh<'a>(bytes: &'a [u8], role: Role) -> Result<ValidatedKernelEnvelope<'a>> {
    validate(black_box(bytes), PROFILE)
        .map_err(|e| format!("loader: {e:?}"))?
        .bind_kernel(black_box(role.symbol))
        .map_err(|e| format!("binding: {e:?}"))
}

fn equivalent(a: &ValidatedKernelEnvelope<'_>, b: &ValidatedKernelEnvelope<'_>) -> Result<()> {
    check(
        a.envelope().plan() == b.envelope().plan(),
        "load plan mismatch",
    )?;
    check(
        a.selected_kernel() == b.selected_kernel(),
        "selected metadata mismatch",
    )?;
    check(
        a.selected_kernel_index() == b.selected_kernel_index(),
        "selected index mismatch",
    )?;
    check(
        a.selected_binding() == b.selected_binding(),
        "descriptor binding mismatch",
    )?;
    check(a.resources() == b.resources(), "resources mismatch")?;
    check(
        a.descriptor_bytes() == b.descriptor_bytes(),
        "descriptor bytes mismatch",
    )?;
    check(a.entry_bytes() == b.entry_bytes(), "entry bytes mismatch")?;
    check(
        a.identity_inputs() == b.identity_inputs(),
        "closure identity mismatch",
    )?;
    check(
        a.relocation_evidence() == b.relocation_evidence(),
        "relocation evidence mismatch",
    )
}

// Identical descriptor access in both arms; no clone, formatting or byte hash.
fn consume(value: &ValidatedKernelEnvelope<'_>) -> [u8; 32] {
    black_box((
        value.envelope().plan(),
        value.selected_kernel(),
        value.selected_kernel_index(),
        value.selected_binding(),
        value.resources(),
        value.descriptor_bytes(),
        value.entry_bytes(),
        value.identity_inputs(),
        value.relocation_evidence(),
    ));
    black_box(value.identity_inputs().closure_sha256())
}

fn accumulate(sum: &mut u64, digest: [u8; 32]) -> Result<()> {
    // Every consumed digest contributes; the bounded byte sum cannot wrap.
    for byte in digest {
        *sum = sum
            .checked_add(u64::from(byte))
            .ok_or("consumption counter overflow")?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Arm {
    Fresh,
    Owned,
}

fn order(pair: usize) -> [Arm; 2] {
    if pair.is_multiple_of(2) {
        [Arm::Fresh, Arm::Owned]
    } else {
        [Arm::Owned, Arm::Fresh]
    }
}

#[derive(Debug, Serialize)]
struct ArmSample {
    elapsed_ns: u64,
    preparations: usize,
    consumption_sum: u64,
}

#[derive(Debug, Serialize)]
struct PairSample {
    index: usize,
    order: [Arm; 2],
    fresh: ArmSample,
    owned: ArmSample,
}

fn sample(
    arm: Arm,
    images: &[Vec<u8>],
    owned: &[OwnedValidatedKernelEnvelope],
    until: Instant,
) -> Result<ArmSample> {
    check(
        images.len() == IMAGES.len() && owned.len() == ROLES.len(),
        "closed benchmark roster",
    )?;
    let started = Instant::now();
    deadline(started, until)?;
    let mut sum = 0;
    for _ in 0..LAYERS {
        for (index, role) in ROLES.iter().copied().enumerate() {
            deadline(Instant::now(), until)?;
            let value = match arm {
                Arm::Fresh => fresh(&images[role.image], role)?,
                Arm::Owned => black_box(&owned[index]).validated(),
            };
            accumulate(&mut sum, consume(&value))?;
            drop(value);
            deadline(Instant::now(), until)?;
        }
    }
    let finished = Instant::now();
    deadline(finished, until)?;
    Ok(ArmSample {
        elapsed_ns: nanos(finished.duration_since(started))?,
        preparations: LAYERS * ROLES.len(),
        consumption_sum: sum,
    })
}

fn pair(
    index: usize,
    images: &[Vec<u8>],
    owned: &[OwnedValidatedKernelEnvelope],
    until: Instant,
) -> Result<PairSample> {
    let ordered = order(index);
    let first = sample(ordered[0], images, owned, until)?;
    let second = sample(ordered[1], images, owned, until)?;
    let (fresh, owned) = if ordered[0] == Arm::Fresh {
        (first, second)
    } else {
        (second, first)
    };
    check(
        fresh.preparations == owned.preparations && fresh.consumption_sum == owned.consumption_sum,
        "timed-arm result consumption mismatch",
    )?;
    Ok(PairSample {
        index,
        order: ordered,
        fresh,
        owned,
    })
}

#[derive(Debug, Serialize)]
struct RoleIdentity {
    role: Role,
    object_sha256: String,
    metadata_sha256: String,
    descriptor_sha256: String,
    entry_sha256: String,
    closure_sha256: String,
}

fn check_roles(
    images: &[Vec<u8>],
    owned: &[OwnedValidatedKernelEnvelope],
    until: Instant,
) -> Result<Vec<RoleIdentity>> {
    check(
        images.len() == IMAGES.len() && owned.len() == ROLES.len(),
        "closed equality roster",
    )?;
    let mut identities = Vec::with_capacity(ROLES.len());
    for (index, role) in ROLES.iter().copied().enumerate() {
        deadline(Instant::now(), until)?;
        let a = fresh(&images[role.image], role)?;
        let b = owned[index].validated();
        equivalent(&a, &b)?;
        check(
            owned[index].semantic_binding_passes() == 1,
            "owned closure binding count",
        )?;
        let id = a.identity_inputs();
        check(
            hex(&id.object_sha256()) == IMAGES[role.image].sha256,
            "selected closure original object identity",
        )?;
        identities.push(RoleIdentity {
            role,
            object_sha256: hex(&id.object_sha256()),
            metadata_sha256: hex(&id.metadata_sha256()),
            descriptor_sha256: hex(&id.descriptor_sha256()),
            entry_sha256: hex(&id.entry_sha256()),
            closure_sha256: hex(&id.closure_sha256()),
        });
        deadline(Instant::now(), until)?;
    }
    Ok(identities)
}

#[derive(Debug, Serialize)]
struct Limits {
    wall_seconds: u64,
    required_external_cpu_seconds: u64,
    required_external_address_space_bytes: u64,
    maximum_stdout_bytes: usize,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    passed: bool,
    profile: &'static str,
    images: [ImagePin; 4],
    image_identities: Vec<FileIdentity>,
    role_identities: Vec<RoleIdentity>,
    layers_per_sample: usize,
    preparations_per_sample: usize,
    owned_creation_ns: u64,
    warmup_pairs: Vec<PairSample>,
    sample_pairs: Vec<PairSample>,
    wall_elapsed_ns: u64,
    limits: Limits,
    pre_and_post_equivalence_checked: bool,
    original_images_rehashed_after_samples: bool,
    external_resource_limits_verified_by_program: bool,
    dynamic_dispatch_validation_measured: bool,
    native_or_model_execution: bool,
    end_to_end_gain: bool,
    full2303_feasibility: bool,
    numerical_acceptance: bool,
}

pub fn run(root: &Path) -> Result<Vec<u8>> {
    let started = Instant::now();
    let until = started
        .checked_add(Duration::from_secs(WALL_SECONDS))
        .ok_or("deadline overflow")?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    check(root.is_dir(), "image root must be a directory")?;
    let mut images = Vec::with_capacity(IMAGES.len());
    let mut ids = Vec::with_capacity(IMAGES.len());
    for pin in IMAGES {
        deadline(Instant::now(), until)?;
        let (bytes, id) = read_image(&root, pin)?;
        images.push(bytes);
        ids.push(id);
    }
    let cold = Instant::now();
    let mut owned = Vec::with_capacity(ROLES.len());
    for role in ROLES {
        deadline(Instant::now(), until)?;
        owned.push(
            validate_owned(images[role.image].clone(), PROFILE)
                .map_err(|e| format!("owned loader: {e:?}"))?
                .bind_kernel(role.symbol)
                .map_err(|e| format!("owned binding: {e:?}"))?,
        );
    }
    let owned_creation_ns = nanos(cold.elapsed())?;
    let role_identities = check_roles(&images, &owned, until)?;
    let mut warmup_pairs = Vec::with_capacity(WARMUP_PAIRS);
    for index in 0..WARMUP_PAIRS {
        warmup_pairs.push(pair(index, &images, &owned, until)?);
    }
    let mut sample_pairs = Vec::with_capacity(SAMPLE_PAIRS);
    for index in 0..SAMPLE_PAIRS {
        sample_pairs.push(pair(index, &images, &owned, until)?);
    }
    check_roles(&images, &owned, until)?;
    for (index, pin) in IMAGES.into_iter().enumerate() {
        deadline(Instant::now(), until)?;
        let (bytes, id) = read_image(&root, pin)?;
        check(
            id == ids[index] && bytes == images[index],
            "original image drift after samples",
        )?;
    }
    deadline(Instant::now(), until)?;
    let report = Report {
        schema: "ferric-owned-kernel-admission-opportunity-v1",
        passed: true,
        profile: "gfx950-xnack-off-cov6",
        images: IMAGES,
        image_identities: ids,
        role_identities,
        layers_per_sample: LAYERS,
        preparations_per_sample: LAYERS * ROLES.len(),
        owned_creation_ns,
        warmup_pairs,
        sample_pairs,
        wall_elapsed_ns: nanos(started.elapsed())?,
        limits: Limits {
            wall_seconds: WALL_SECONDS,
            required_external_cpu_seconds: REQUIRED_CPU_SECONDS,
            required_external_address_space_bytes: REQUIRED_ADDRESS_SPACE_BYTES,
            maximum_stdout_bytes: MAX_OUTPUT_BYTES,
        },
        pre_and_post_equivalence_checked: true,
        original_images_rehashed_after_samples: true,
        external_resource_limits_verified_by_program: false,
        dynamic_dispatch_validation_measured: false,
        native_or_model_execution: false,
        end_to_end_gain: false,
        full2303_feasibility: false,
        numerical_acceptance: false,
    };
    let mut raw = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    raw.push(b'\n');
    check(raw.len() <= MAX_OUTPUT_BYTES, "report output cap")?;
    deadline(Instant::now(), until)?;
    Ok(raw)
}

#[cfg(test)]
mod tests;
