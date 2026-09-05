//! Canonical pre-observation admission for an externally supplied D10 policy.
//!
//! The non-atomic `mkdirat`/open boundary adopts an exact empty owner-controlled
//! directory. Failure cleanup never claims or removes that directory; it removes
//! only names still bound to file inodes created and held by this transaction.

use ferric_m1_benchmarks::{encode_canonical_document, sha256_identity, BenchResult};
use rustix::fd::OwnedFd;
use rustix::fs::{
    fstat, fsync, mkdirat, openat2, renameat_with, unlinkat, AtFlags, Dir, FileType, Mode, OFlags,
    RenameFlags, ResolveFlags, Stat, CWD,
};
use rustix::process::{getegid, geteuid};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

const POLICY_FORMAT: &str = "FERRIC-M1-D10-EXPERIMENT-POLICY-INPUT-V2";
const ADMISSION_FORMAT: &str = "FERRIC-M1-D10-EXPERIMENT-POLICY-ADMISSION-V2";
const POLICY_AUTHORITY: &str = "externally-supplied-pre-observation-policy-only";
const COMPANION_AUTHORITY: &str = "externally-supplied-pre-observation-companion-only";
const ADMISSION_AUTHORITY: &str = "checked-pre-observation-policy-structure-only";
const PARTIAL_STATUS: &str = "PARTIAL_NON_EVIDENCE";
const TARGET: &str = "gfx942:xnack-";
const WARMUPS: u64 = 10;
const RECORDED_SAMPLES: u64 = 30;
const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
const MAX_POLICY_INTEGER: u64 = 1_000_000_000_000_000;
const POLICY_NONCLAIM: &str = "This input freezes externally supplied D10 experiment-policy values and source, compiler-worker, runtime, and KFD closure identities before observation. It does not claim that the values or identities are reviewed, truthful, fair, sufficient, measured, passing, or able to close m1.r31.";
const ADMISSION_NONCLAIM: &str = "This admission authenticates only canonical structure and frozen identity for an externally supplied pre-observation D10 experiment policy and its Ferric, pinned-fe2o3, compiler-worker, runtime, and KFD bindings. Legacy plan and validate commands do not consume this policy, so its 10 warmups and 30 recorded samples are not execution-enforced. The separate policy-SHA-256-bound D10 observation validator does not retroactively bind those legacy commands or validate the supplied policy values or closure truth. This admission does not endorse thresholds, weights, work units, vendor mappings, tuning budgets, observations, performance, hardware correctness, qualification, or close m1.r31.";
const FUTURE_OBSERVATION_BINDING: &str = "policy-sha256-bound-d10-observation-validator";
pub(super) const PINNED_FE2O3_SOURCE_COMMIT: &str = "d8fa0835c64d6574c8589ac3e69e3c34b0350758";
pub(super) const TOOLCHAIN_BINDING_SCHEMA: &str = "canonical-external-ferric-source-commit-and-closure-pinned-fe2o3-source-commit-and-closure-compiler-configuration-and-worker-closure-runtime-and-kfd-closure-v1";
const PROTOCOL_SHA256: &str = "d3563541f74b9506c22743398c1a38055dc600053d76e78fc30160513504284f";

const CASE_ROSTER: &[(&str, &str)] = &[
    ("flash-attention-prefill", "k4-gqa-prefill"),
    ("gemm-gemv", "k1-gemm-gemv"),
    ("gqa-paged-decode", "k5-paged-gqa-decode"),
    ("logits-argmax", "k7-logits-compact"),
    ("rmsnorm-residual", "k2-rmsnorm-residual"),
    ("rope-paged-kv", "k3-rope-paged-kv"),
    ("swiglu-projection", "k6-swiglu"),
];

const COMPANIONS: &[(&str, &str)] = &[
    ("calibration", "calibration.json"),
    ("execution-order", "execution-order.json"),
    ("holdout", "holdout.json"),
    ("regression-reference", "regression-reference.json"),
    ("resource-inspection", "resource-inspection.json"),
    ("telemetry", "telemetry.json"),
    ("timing", "timing.json"),
    ("tuning", "tuning.json"),
];

const INPUT_FILES: &[&str] = &[
    "calibration.json",
    "execution-order.json",
    "holdout.json",
    "policy.json",
    "protocol.json",
    "regression-reference.json",
    "resource-inspection.json",
    "telemetry.json",
    "timing.json",
    "tuning.json",
];

const OUTPUT_FILES: &[&str] = &["admission.json", "protocol.json"];

/// Descriptor-held view of one policy root after the existing admission checks.
pub(super) struct HeldValidatedPolicy {
    admission: Value,
    admission_bytes: Vec<u8>,
    root: PolicyRoot,
}

impl HeldValidatedPolicy {
    pub(super) fn admission(&self) -> &Value {
        &self.admission
    }

    pub(super) fn admission_bytes(&self) -> &[u8] {
        &self.admission_bytes
    }

    pub(super) fn document_bytes(&self, path: &str) -> BenchResult<&[u8]> {
        Ok(&self.root.document(path)?.bytes)
    }

    pub(super) fn document_value(&self, path: &str) -> BenchResult<&Value> {
        Ok(&self.root.document(path)?.value)
    }

    pub(super) fn toolchain(&self) -> BenchResult<&Value> {
        let policy = self
            .document_value("policy.json")?
            .as_object()
            .ok_or_else(|| "held D10 policy must be an object".to_owned())?;
        get(policy, "toolchain", "held D10 policy")
    }

    pub(super) fn toolchain_sha256(&self) -> BenchResult<String> {
        validate_toolchain(self.toolchain()?)
    }

    pub(super) fn revalidate(&mut self) -> BenchResult<()> {
        self.root.revalidate()
    }
}

/// Holds and validates the exact original policy root without publishing admission.
pub(super) fn hold_validated_policy(path: &Path) -> BenchResult<HeldValidatedPolicy> {
    let root = PolicyRoot::open(path)?;
    let admission = validate_policy(&root)?;
    let admission_bytes = encode_canonical_document(&admission)?;
    Ok(HeldValidatedPolicy {
        admission,
        admission_bytes,
        root,
    })
}

struct HeldDocument {
    bytes: Vec<u8>,
    file: File,
    initial: Stat,
    path: &'static str,
    value: Value,
}

impl HeldDocument {
    fn open(root: &OwnedFd, path: &'static str) -> BenchResult<Self> {
        let descriptor = openat2(
            root,
            Path::new(path),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open D10 policy input {path}: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect D10 policy input {path}: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
            || initial.st_nlink != 1
        {
            return Err(format!(
                "D10 policy input {path} must be a one-link regular file"
            ));
        }
        let length = usize::try_from(initial.st_size)
            .map_err(|_| format!("D10 policy input {path} length is invalid"))?;
        if length == 0 || length > MAX_DOCUMENT_BYTES {
            return Err(format!(
                "D10 policy input {path} length is outside the admitted bound"
            ));
        }
        let mut file = File::from(descriptor);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| format!("cannot reserve D10 policy input {path} buffer"))?;
        Read::by_ref(&mut file)
            .take(MAX_DOCUMENT_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read D10 policy input {path}: {error}"))?;
        let final_stat = fstat(&file)
            .map_err(|error| format!("cannot reinspect D10 policy input {path}: {error}"))?;
        if bytes.len() != length || !same_snapshot(&initial, &final_stat) {
            return Err(format!("D10 policy input {path} changed while being read"));
        }
        if !bytes.is_ascii() {
            return Err(format!("D10 policy input {path} must be ASCII JSON"));
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot parse D10 policy input {path}: {error}"))?;
        if encode_canonical_document(&value)? != bytes {
            return Err(format!("D10 policy input {path} is not canonical JSON"));
        }
        Ok(Self {
            bytes,
            file,
            initial,
            path,
            value,
        })
    }

    fn revalidate(&mut self, root: &OwnedFd) -> BenchResult<()> {
        let held = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect held input {}: {error}", self.path))?;
        if !same_snapshot(&self.initial, &held) {
            return Err(format!("held D10 policy input {} changed", self.path));
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("cannot rewind held input {}: {error}", self.path))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.bytes.len().saturating_add(1))
            .map_err(|_| format!("cannot reserve held input {} buffer", self.path))?;
        Read::by_ref(&mut self.file)
            .take(MAX_DOCUMENT_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot reread held input {}: {error}", self.path))?;
        let reread = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect reread input {}: {error}", self.path))?;
        if bytes != self.bytes || !same_snapshot(&self.initial, &reread) {
            return Err(format!("held D10 policy input {} bytes changed", self.path));
        }
        let named = openat2(
            root,
            Path::new(self.path),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind D10 policy input {}: {error}", self.path))?;
        let named = fstat(&named)
            .map_err(|error| format!("cannot inspect rebound input {}: {error}", self.path))?;
        if !same_snapshot(&self.initial, &named) {
            return Err(format!(
                "D10 policy input name {} no longer identifies the held file",
                self.path
            ));
        }
        Ok(())
    }
}

struct PolicyRoot {
    descriptor: OwnedFd,
    documents: BTreeMap<&'static str, HeldDocument>,
    initial: Stat,
}

struct PartitionRoster {
    digest: String,
    identities: BTreeSet<String>,
    member_ids: BTreeSet<String>,
}

impl PolicyRoot {
    fn open(path: &Path) -> BenchResult<Self> {
        validate_root_path(path)?;
        let descriptor = openat2(
            CWD,
            path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open D10 policy root: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect D10 policy root: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::Directory {
            return Err("D10 policy root must be a directory".to_owned());
        }
        validate_roster(&descriptor, INPUT_FILES, "D10 policy root")?;
        let mut documents = BTreeMap::new();
        let mut identities = BTreeSet::new();
        for path in INPUT_FILES {
            let document = HeldDocument::open(&descriptor, path)?;
            if !identities.insert((document.initial.st_dev, document.initial.st_ino)) {
                return Err("D10 policy inputs must not alias one another".to_owned());
            }
            documents.insert(*path, document);
        }
        let current = fstat(&descriptor)
            .map_err(|error| format!("cannot reinspect D10 policy root: {error}"))?;
        if !same_snapshot(&initial, &current) {
            return Err("D10 policy root changed while inputs were opened".to_owned());
        }
        Ok(Self {
            descriptor,
            documents,
            initial,
        })
    }

    fn document(&self, path: &str) -> BenchResult<&HeldDocument> {
        self.documents
            .get(path)
            .ok_or_else(|| format!("missing held D10 policy input: {path}"))
    }

    fn revalidate(&mut self) -> BenchResult<()> {
        validate_roster(&self.descriptor, INPUT_FILES, "D10 policy root")?;
        for document in self.documents.values_mut() {
            document.revalidate(&self.descriptor)?;
        }
        let current = fstat(&self.descriptor)
            .map_err(|error| format!("cannot reinspect D10 policy root: {error}"))?;
        if !same_snapshot(&self.initial, &current) {
            return Err("D10 policy root changed after validation".to_owned());
        }
        Ok(())
    }
}

struct CreatedOutput {
    bytes: Vec<u8>,
    file: File,
    initial: Stat,
    name: &'static str,
}

impl CreatedOutput {
    fn create(
        parent: &OwnedFd,
        name: &'static str,
        bytes: Vec<u8>,
        description: &str,
    ) -> BenchResult<Self> {
        let descriptor = openat2(
            parent,
            Path::new(name),
            OFlags::RDWR
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot create staged {description}: {error}"))?;
        let created = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect created {description}: {error}"))?;
        if let Err(error) = validate_created_file(&created, 0, description) {
            cleanup_created_name(parent, OsStr::new(name), &created);
            return Err(error);
        }
        let mut file = File::from(descriptor);
        if let Err(error) = file.write_all(&bytes) {
            drop(file);
            cleanup_created_name(parent, OsStr::new(name), &created);
            return Err(format!("cannot write staged {description}: {error}"));
        }
        if let Err(error) = file.sync_all() {
            drop(file);
            cleanup_created_name(parent, OsStr::new(name), &created);
            return Err(format!("cannot sync staged {description}: {error}"));
        }
        let initial = match fstat(&file) {
            Ok(initial) => initial,
            Err(error) => {
                drop(file);
                cleanup_created_name(parent, OsStr::new(name), &created);
                return Err(format!("cannot reinspect staged {description}: {error}"));
            }
        };
        let expected_size = u64::try_from(bytes.len())
            .map_err(|_| format!("staged {description} length does not fit u64"))?;
        if let Err(error) = validate_created_file(&initial, expected_size, description) {
            drop(file);
            cleanup_created_name(parent, OsStr::new(name), &created);
            return Err(error);
        }
        Ok(Self {
            bytes,
            file,
            initial,
            name,
        })
    }

    fn validate_held(&mut self, description: &str) -> BenchResult<()> {
        let held = fstat(&self.file)
            .map_err(|error| format!("cannot inspect held {description}: {error}"))?;
        if !same_snapshot(&self.initial, &held) {
            return Err(format!("held {description} metadata changed"));
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("cannot rewind held {description}: {error}"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.bytes.len().saturating_add(1))
            .map_err(|_| format!("cannot reserve held {description} verification buffer"))?;
        Read::by_ref(&mut self.file)
            .take(self.bytes.len().saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot reread held {description}: {error}"))?;
        let final_stat = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect held {description}: {error}"))?;
        if bytes != self.bytes || !same_snapshot(&self.initial, &final_stat) {
            return Err(format!("held {description} bytes or metadata changed"));
        }
        Ok(())
    }

    fn rebind_and_reread(&mut self, parent: &OwnedFd, description: &str) -> BenchResult<()> {
        self.validate_held(description)?;
        let descriptor = openat2(
            parent,
            Path::new(self.name),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind {description} name: {error}"))?;
        let named = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect rebound {description}: {error}"))?;
        if !same_snapshot(&self.initial, &named) {
            return Err(format!(
                "{description} name no longer identifies its created file"
            ));
        }
        let mut file = File::from(descriptor);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.bytes.len().saturating_add(1))
            .map_err(|_| format!("cannot reserve rebound {description} buffer"))?;
        Read::by_ref(&mut file)
            .take(self.bytes.len().saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot reread rebound {description}: {error}"))?;
        let final_stat = fstat(&file)
            .map_err(|error| format!("cannot reinspect rebound {description}: {error}"))?;
        if bytes != self.bytes || !same_snapshot(&self.initial, &final_stat) {
            return Err(format!("rebound {description} bytes or metadata changed"));
        }
        Ok(())
    }

    fn name_has_created_identity(&self, parent: &OwnedFd) -> bool {
        name_has_identity(parent, OsStr::new(self.name), &self.initial)
    }
}

struct StagedAdmission {
    admission: Option<CreatedOutput>,
    armed: bool,
    initial: Stat,
    output_name: OsString,
    output_path: PathBuf,
    parent: OwnedFd,
    protocol: Option<CreatedOutput>,
    staging: OwnedFd,
    staging_name: OsString,
    staging_path: PathBuf,
}

impl StagedAdmission {
    fn create<F>(
        output: &Path,
        admission: Vec<u8>,
        protocol: Vec<u8>,
        after_mkdir: F,
    ) -> BenchResult<Self>
    where
        F: FnOnce(&Path) -> BenchResult<()>,
    {
        let output_name = safe_output_name(output)?;
        let parent_path = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        validate_parent_path(parent_path)?;
        let parent = openat2(
            CWD,
            parent_path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open D10 admission parent: {error}"))?;
        let parent_stat = fstat(&parent)
            .map_err(|error| format!("cannot inspect D10 admission parent: {error}"))?;
        if FileType::from_raw_mode(parent_stat.st_mode) != FileType::Directory
            || parent_stat.st_uid != geteuid().as_raw()
            || parent_stat.st_gid != getegid().as_raw()
            || parent_stat.st_mode & 0o022 != 0
        {
            return Err(
                "D10 admission parent must be owner-controlled without group/other writes"
                    .to_owned(),
            );
        }
        require_absent(&parent, &output_name, "D10 admission output")?;
        let mut after_mkdir = Some(after_mkdir);
        for nonce in 0..1_024_u16 {
            let mut staging_name = OsString::from(".");
            staging_name.push(&output_name);
            staging_name.push(format!(".staging.{}.{nonce}", std::process::id()));
            match mkdirat(
                &parent,
                staging_name.as_os_str(),
                Mode::RUSR | Mode::WUSR | Mode::XUSR,
            ) {
                Ok(()) => {
                    let staging_path = parent_path.join(&staging_name);
                    after_mkdir
                        .take()
                        .ok_or_else(|| "D10 mkdir hook was already consumed".to_owned())?(
                        &staging_path,
                    )?;
                    let staging = open_directory_at(
                        &parent,
                        Path::new(&staging_name),
                        "D10 admission staging directory",
                    )?;
                    let initial = fstat(&staging).map_err(|error| {
                        format!("cannot inspect D10 admission staging directory: {error}")
                    })?;
                    validate_adopted_directory(&initial, "D10 admission staging directory")?;
                    validate_roster(&staging, &[], "newly adopted D10 staging directory")?;
                    let mut staged = Self {
                        admission: None,
                        armed: true,
                        initial,
                        output_name: output_name.clone(),
                        output_path: output.to_path_buf(),
                        parent,
                        protocol: None,
                        staging,
                        staging_name: staging_name.clone(),
                        staging_path,
                    };
                    staged.write_files(admission, protocol)?;
                    return Ok(staged);
                }
                Err(error) if error == rustix::io::Errno::EXIST => {}
                Err(error) => {
                    return Err(format!(
                        "cannot create D10 admission staging directory: {error}"
                    ));
                }
            }
        }
        Err("D10 admission staging namespace was exhausted".to_owned())
    }

    fn write_files(&mut self, admission: Vec<u8>, protocol: Vec<u8>) -> BenchResult<()> {
        self.admission = Some(CreatedOutput::create(
            &self.staging,
            "admission.json",
            admission,
            "D10 admission document",
        )?);
        self.protocol = Some(CreatedOutput::create(
            &self.staging,
            "protocol.json",
            protocol,
            "D10 admission protocol",
        )?);
        fsync(&self.staging)
            .map_err(|error| format!("cannot sync D10 admission staging directory: {error}"))?;
        self.initial = fstat(&self.staging)
            .map_err(|error| format!("cannot reinspect staged D10 admission: {error}"))?;
        validate_adopted_directory(&self.initial, "D10 admission staging directory")
    }

    fn publish<F>(mut self, inputs: &mut PolicyRoot, after_rename: F) -> BenchResult<()>
    where
        F: FnOnce(&Path) -> BenchResult<()>,
    {
        validate_roster(
            &self.staging,
            OUTPUT_FILES,
            "D10 admission staging directory",
        )?;
        Self::revalidate_files(
            &mut self.admission,
            &mut self.protocol,
            &self.staging,
            "staged",
        )?;
        let namespace = open_directory_at(
            &self.parent,
            Path::new(&self.staging_name),
            "D10 admission staging namespace",
        )?;
        let named = fstat(&namespace)
            .map_err(|error| format!("cannot inspect D10 admission staging namespace: {error}"))?;
        if !same_snapshot(&self.initial, &named) {
            return Err(
                "D10 admission staging name no longer identifies the held directory".to_owned(),
            );
        }
        inputs.revalidate()?;
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::EXIST {
                "D10 admission output appeared before no-replace publication".to_owned()
            } else {
                format!("cannot publish D10 admission without replacement: {error}")
            }
        })?;
        self.armed = false;
        fsync(&self.parent)
            .map_err(|error| format!("cannot sync published D10 admission parent: {error}"))?;
        let published = open_directory_at(
            &self.parent,
            Path::new(&self.output_name),
            "published D10 admission",
        )?;
        let published_baseline = fstat(&published)
            .map_err(|error| format!("cannot inspect published D10 admission: {error}"))?;
        if !same_directory_publication_transition(&self.initial, &published_baseline) {
            return Err("published D10 admission identity changed".to_owned());
        }
        after_rename(&self.output_path)?;
        validate_roster(&published, OUTPUT_FILES, "published D10 admission")?;
        Self::revalidate_files(
            &mut self.admission,
            &mut self.protocol,
            &published,
            "published",
        )?;
        inputs.revalidate()?;
        let held_final = fstat(&published)
            .map_err(|error| format!("cannot reinspect published D10 admission: {error}"))?;
        if !same_snapshot(&published_baseline, &held_final) {
            return Err("published D10 admission directory changed during verification".to_owned());
        }
        let final_name = open_directory_at(
            &self.parent,
            Path::new(&self.output_name),
            "final D10 admission namespace",
        )?;
        let final_stat = fstat(&final_name)
            .map_err(|error| format!("cannot inspect final D10 admission namespace: {error}"))?;
        if !same_snapshot(&published_baseline, &final_stat) {
            return Err("published D10 admission name changed during verification".to_owned());
        }
        Ok(())
    }

    fn revalidate_files(
        admission: &mut Option<CreatedOutput>,
        protocol: &mut Option<CreatedOutput>,
        parent: &OwnedFd,
        phase: &str,
    ) -> BenchResult<()> {
        admission
            .as_mut()
            .ok_or_else(|| "D10 admission output was not created".to_owned())?
            .rebind_and_reread(parent, &format!("{phase} D10 admission document"))?;
        protocol
            .as_mut()
            .ok_or_else(|| "D10 admission protocol was not created".to_owned())?
            .rebind_and_reread(parent, &format!("{phase} D10 admission protocol"))
    }
}

impl Drop for StagedAdmission {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for output in [&self.admission, &self.protocol].into_iter().flatten() {
            if output.name_has_created_identity(&self.staging) {
                let _ = unlinkat(&self.staging, output.name, AtFlags::empty());
            }
        }
    }
}

/// Admits one exact external D10 experiment policy without granting evidence authority.
pub(super) fn admit_experiment_policy(arguments: &[OsString]) -> BenchResult<()> {
    admit_experiment_policy_with_hooks(arguments, |_| Ok(()), |_| Ok(()), |_| Ok(()), |_| Ok(()))
}

fn admit_experiment_policy_with_hooks<F, G, H, I>(
    arguments: &[OsString],
    after_input: F,
    after_mkdir: G,
    after_staging: H,
    after_rename: I,
) -> BenchResult<()>
where
    F: FnOnce(&Path) -> BenchResult<()>,
    G: FnOnce(&Path) -> BenchResult<()>,
    H: FnOnce(&Path) -> BenchResult<()>,
    I: FnOnce(&Path) -> BenchResult<()>,
{
    let [command, policy_root, output] = arguments else {
        return Err(
            "usage: ferric-m1-d10 admit-experiment-policy POLICY-ROOT OUTPUT-BUNDLE".to_owned(),
        );
    };
    if command != "admit-experiment-policy" {
        return Err("D10 experiment-policy command drifted".to_owned());
    }
    let root_path = Path::new(policy_root);
    let mut root = PolicyRoot::open(root_path)?;
    let admission = validate_policy(&root)?;
    let admission = encode_canonical_document(&admission)?;
    let protocol = root.document("protocol.json")?.bytes.clone();
    after_input(root_path)?;
    let staged = StagedAdmission::create(Path::new(output), admission, protocol, after_mkdir)?;
    after_staging(&staged.staging_path)?;
    staged.publish(&mut root, after_rename)
}

fn validate_policy(root: &PolicyRoot) -> BenchResult<Value> {
    let protocol = root.document("protocol.json")?;
    if sha256_identity(&protocol.bytes) != PROTOCOL_SHA256 {
        return Err("D10 policy protocol was substituted".to_owned());
    }
    validate_protocol(&protocol.value)?;

    let policy_document = root.document("policy.json")?;
    let policy = exact_object(
        &policy_document.value,
        &[
            "authority",
            "cases",
            "companions",
            "format",
            "milestone",
            "nonclaim",
            "obligation_id",
            "path_id",
            "protocol_sha256",
            "sample_protocol",
            "status",
            "suite",
            "target",
            "thresholds",
            "toolchain",
        ],
        "D10 experiment policy",
    )?;
    expect_string(
        policy,
        "authority",
        POLICY_AUTHORITY,
        "D10 policy authority",
    )?;
    expect_string(policy, "format", POLICY_FORMAT, "D10 policy format")?;
    expect_string(policy, "milestone", "M1", "D10 policy milestone")?;
    expect_string(policy, "nonclaim", POLICY_NONCLAIM, "D10 policy nonclaim")?;
    expect_string(policy, "obligation_id", "m1.r31", "D10 policy obligation")?;
    expect_string(policy, "path_id", "d10-bench", "D10 policy path")?;
    expect_string(policy, "status", "pre-observation", "D10 policy status")?;
    expect_string(policy, "suite", "d10", "D10 policy suite")?;
    expect_string(policy, "target", TARGET, "D10 policy target")?;
    expect_string(
        policy,
        "protocol_sha256",
        PROTOCOL_SHA256,
        "D10 policy protocol identity",
    )?;
    validate_samples(get(policy, "sample_protocol", "D10 experiment policy")?)?;
    let cases = validate_cases(get(policy, "cases", "D10 experiment policy")?)?;
    validate_thresholds(get(policy, "thresholds", "D10 experiment policy")?)?;
    let toolchain = get(policy, "toolchain", "D10 experiment policy")?;
    let toolchain_sha256 = validate_toolchain(toolchain)?;
    let companion_bindings =
        validate_companion_bindings(root, get(policy, "companions", "D10 experiment policy")?)?;
    validate_timing(&root.document("timing.json")?.value)?;
    let tuning_calibration = validate_tuning(&root.document("tuning.json")?.value)?;
    validate_execution_order(&root.document("execution-order.json")?.value)?;
    validate_telemetry(&root.document("telemetry.json")?.value)?;
    validate_resources(&root.document("resource-inspection.json")?.value)?;
    let calibration = validate_partition(
        &root.document("calibration.json")?.value,
        "FERRIC-M1-D10-CALIBRATION-POLICY-V1",
        "D10 calibration policy",
    )?;
    let holdout = validate_partition(
        &root.document("holdout.json")?.value,
        "FERRIC-M1-D10-HOLDOUT-POLICY-V1",
        "D10 holdout policy",
    )?;
    if !calibration.member_ids.is_disjoint(&holdout.member_ids)
        || !calibration.identities.is_disjoint(&holdout.identities)
    {
        return Err("D10 calibration and holdout memberships must be disjoint".to_owned());
    }
    if tuning_calibration != calibration.digest {
        return Err("D10 tuning policy does not bind the calibration roster".to_owned());
    }
    validate_regression_reference(&root.document("regression-reference.json")?.value)?;

    let mut inputs = Map::new();
    for (path, document) in &root.documents {
        inputs.insert(
            (*path).to_owned(),
            json!({
                "bytes": document.bytes.len(),
                "path": path,
                "sha256": sha256_identity(&document.bytes),
            }),
        );
    }
    Ok(json!({
        "authority": ADMISSION_AUTHORITY,
        "cases": cases,
        "closes": [],
        "companion_bindings": companion_bindings,
        "format": ADMISSION_FORMAT,
        "future_required_binding": FUTURE_OBSERVATION_BINDING,
        "input_roster": inputs,
        "legacy_plan_validate_policy_bound": false,
        "milestone": "M1",
        "nonclaim": ADMISSION_NONCLAIM,
        "obligation_id": "m1.r31",
        "observation_counts_enforced": false,
        "observations_admitted": false,
        "path_id": "d10-bench",
        "policy_sha256": sha256_identity(&policy_document.bytes),
        "protocol_sha256": PROTOCOL_SHA256,
        "r31_closed": false,
        "sample_protocol": get(policy, "sample_protocol", "D10 experiment policy")?,
        "status": PARTIAL_STATUS,
        "suite": "d10",
        "target": TARGET,
        "thresholds": get(policy, "thresholds", "D10 experiment policy")?,
        "toolchain": toolchain,
        "toolchain_sha256": toolchain_sha256,
    }))
}

fn validate_protocol(value: &Value) -> BenchResult<()> {
    let protocol = exact_object(
        value,
        &[
            "authority",
            "case_roster",
            "companion_roster",
            "format",
            "future_required_binding",
            "input_format",
            "legacy_plan_validate_policy_bound",
            "nonclaim",
            "obligation_id",
            "observation_counts_enforced",
            "output_format",
            "publication_roster",
            "recorded_samples",
            "status",
            "suite",
            "target",
            "toolchain_binding_schema",
            "warmups",
        ],
        "D10 policy protocol",
    )?;
    expect_string(
        protocol,
        "authority",
        "source-controlled-pre-observation-protocol-only",
        "D10 protocol authority",
    )?;
    expect_string(
        protocol,
        "format",
        "FERRIC-M1-D10-EXPERIMENT-POLICY-PROTOCOL-V2",
        "D10 protocol format",
    )?;
    expect_string(
        protocol,
        "future_required_binding",
        FUTURE_OBSERVATION_BINDING,
        "D10 protocol future observation binding",
    )?;
    expect_string(
        protocol,
        "input_format",
        POLICY_FORMAT,
        "D10 protocol input format",
    )?;
    expect_string(
        protocol,
        "output_format",
        ADMISSION_FORMAT,
        "D10 protocol output format",
    )?;
    expect_string(
        protocol,
        "obligation_id",
        "m1.r31",
        "D10 protocol obligation",
    )?;
    expect_bool(
        protocol,
        "legacy_plan_validate_policy_bound",
        false,
        "D10 protocol legacy binding",
    )?;
    expect_bool(
        protocol,
        "observation_counts_enforced",
        false,
        "D10 protocol count enforcement",
    )?;
    expect_string(protocol, "status", PARTIAL_STATUS, "D10 protocol status")?;
    expect_string(protocol, "suite", "d10", "D10 protocol suite")?;
    expect_string(protocol, "target", TARGET, "D10 protocol target")?;
    expect_string(
        protocol,
        "toolchain_binding_schema",
        TOOLCHAIN_BINDING_SCHEMA,
        "D10 protocol toolchain binding schema",
    )?;
    expect_u64(protocol, "warmups", WARMUPS, "D10 protocol warmups")?;
    expect_u64(
        protocol,
        "recorded_samples",
        RECORDED_SAMPLES,
        "D10 protocol recorded samples",
    )?;
    validate_string_array(
        get(protocol, "companion_roster", "D10 policy protocol")?,
        &COMPANIONS.iter().map(|(_, path)| *path).collect::<Vec<_>>(),
        "D10 protocol companion roster",
    )?;
    validate_string_array(
        get(protocol, "publication_roster", "D10 policy protocol")?,
        OUTPUT_FILES,
        "D10 protocol publication roster",
    )?;
    let cases = get(protocol, "case_roster", "D10 policy protocol")?
        .as_array()
        .ok_or_else(|| "D10 protocol case roster must be an array".to_owned())?;
    if cases.len() != CASE_ROSTER.len() {
        return Err("D10 protocol case roster length drifted".to_owned());
    }
    for (case, (expected_id, expected_family)) in cases.iter().zip(CASE_ROSTER) {
        let case = exact_object(case, &["case_id", "kernel_family"], "D10 protocol case")?;
        expect_string(case, "case_id", expected_id, "D10 protocol case id")?;
        expect_string(
            case,
            "kernel_family",
            expected_family,
            "D10 protocol kernel family",
        )?;
    }
    Ok(())
}

fn validate_samples(value: &Value) -> BenchResult<()> {
    let samples = exact_object(
        value,
        &["recorded_samples", "warmups"],
        "D10 sample protocol",
    )?;
    expect_u64(samples, "warmups", WARMUPS, "D10 policy warmups")?;
    expect_u64(
        samples,
        "recorded_samples",
        RECORDED_SAMPLES,
        "D10 policy recorded samples",
    )
}

fn validate_cases(value: &Value) -> BenchResult<Value> {
    let cases = value
        .as_array()
        .ok_or_else(|| "D10 policy cases must be an array".to_owned())?;
    if cases.len() != CASE_ROSTER.len() {
        return Err("D10 policy case roster length drifted".to_owned());
    }
    let mut profile_ids = BTreeSet::new();
    let mut profile_identities = BTreeSet::new();
    for (case, (expected_id, expected_family)) in cases.iter().zip(CASE_ROSTER) {
        let case = exact_object(
            case,
            &[
                "case_id",
                "ferric_implementation_sha256",
                "kernel_family",
                "profile",
                "vendor",
                "weight",
                "work_unit",
            ],
            "D10 policy case",
        )?;
        expect_string(case, "case_id", expected_id, "D10 policy case id")?;
        expect_string(
            case,
            "kernel_family",
            expected_family,
            "D10 policy kernel family",
        )?;
        require_sha256(get_string(
            case,
            "ferric_implementation_sha256",
            "D10 Ferric implementation identity",
        )?)?;
        let weight = get_u64(case, "weight", "D10 policy case weight")?;
        require_bounded_positive(weight, "D10 policy case weight")?;
        let profile = exact_object(
            get(case, "profile", "D10 policy case")?,
            &["id", "sha256"],
            "D10 policy profile",
        )?;
        let profile_id = get_string(profile, "id", "D10 policy profile id")?;
        require_safe_id(profile_id, "D10 policy profile id")?;
        let profile_sha256 = get_string(profile, "sha256", "D10 policy profile identity")?;
        require_sha256(profile_sha256)?;
        if !profile_ids.insert(profile_id) || !profile_identities.insert(profile_sha256) {
            return Err("D10 policy profiles must be unique".to_owned());
        }
        validate_vendor(get(case, "vendor", "D10 policy case")?)?;
        validate_work_unit(get(case, "work_unit", "D10 policy case")?)?;
    }
    Ok(value.clone())
}

fn validate_vendor(value: &Value) -> BenchResult<()> {
    let vendor = exact_object(
        value,
        &["applicable", "config_sha256", "implementation_sha256"],
        "D10 vendor mapping",
    )?;
    let applicable = get(vendor, "applicable", "D10 vendor mapping")?
        .as_bool()
        .ok_or_else(|| "D10 vendor applicability must be boolean".to_owned())?;
    for field in ["config_sha256", "implementation_sha256"] {
        match (applicable, get(vendor, field, "D10 vendor mapping")?) {
            (true, Value::String(identity)) => require_sha256(identity)?,
            (false, Value::Null) => {}
            _ => {
                return Err(format!(
                    "D10 vendor {field} must be a SHA-256 exactly when applicable"
                ));
            }
        }
    }
    Ok(())
}

fn validate_work_unit(value: &Value) -> BenchResult<()> {
    let unit = exact_object(
        value,
        &["count_per_iteration", "name", "semantics_sha256"],
        "D10 work unit",
    )?;
    require_bounded_positive(
        get_u64(unit, "count_per_iteration", "D10 work-unit count")?,
        "D10 work-unit count",
    )?;
    require_safe_id(
        get_string(unit, "name", "D10 work-unit name")?,
        "D10 work-unit name",
    )?;
    require_sha256(get_string(
        unit,
        "semantics_sha256",
        "D10 work-unit semantics identity",
    )?)
}

fn validate_thresholds(value: &Value) -> BenchResult<()> {
    let thresholds = exact_object(
        value,
        &[
            "maximum_regression_ppm",
            "minimum_per_case_vendor_ratio_ppm",
            "minimum_weighted_vendor_ratio_ppm",
        ],
        "D10 policy thresholds",
    )?;
    for field in [
        "maximum_regression_ppm",
        "minimum_per_case_vendor_ratio_ppm",
        "minimum_weighted_vendor_ratio_ppm",
    ] {
        require_bounded_positive(
            get_u64(thresholds, field, "D10 policy threshold")?,
            "D10 policy threshold",
        )?;
    }
    Ok(())
}

pub(super) fn validate_toolchain(value: &Value) -> BenchResult<String> {
    let toolchain = exact_object(
        value,
        &[
            "compiler_configuration_sha256",
            "compiler_worker_closure_sha256",
            "fe2o3_source_closure_sha256",
            "fe2o3_source_commit",
            "ferric_source_closure_sha256",
            "ferric_source_commit",
            "kfd_runtime_closure_sha256",
            "runtime_closure_sha256",
        ],
        "D10 toolchain binding",
    )?;
    for field in ["ferric_source_commit", "fe2o3_source_commit"] {
        require_commit(
            get_string(toolchain, field, "D10 toolchain binding")?,
            field,
        )?;
    }
    expect_string(
        toolchain,
        "fe2o3_source_commit",
        PINNED_FE2O3_SOURCE_COMMIT,
        "D10 pinned fe2o3 source commit",
    )?;
    for field in [
        "compiler_configuration_sha256",
        "compiler_worker_closure_sha256",
        "fe2o3_source_closure_sha256",
        "ferric_source_closure_sha256",
        "kfd_runtime_closure_sha256",
        "runtime_closure_sha256",
    ] {
        require_sha256(get_string(toolchain, field, "D10 toolchain binding")?)?;
    }
    Ok(sha256_identity(&encode_canonical_document(value)?))
}

fn validate_companion_bindings(root: &PolicyRoot, value: &Value) -> BenchResult<Value> {
    let bindings = value
        .as_object()
        .ok_or_else(|| "D10 policy companion bindings must be an object".to_owned())?;
    exact_keys(
        bindings,
        &COMPANIONS.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "D10 policy companion bindings",
    )?;
    for (name, expected_path) in COMPANIONS {
        let binding = exact_object(
            get(bindings, name, "D10 policy companion bindings")?,
            &["bytes", "path", "sha256"],
            "D10 policy companion binding",
        )?;
        expect_string(binding, "path", expected_path, "D10 policy companion path")?;
        let document = root.document(expected_path)?;
        expect_u64(
            binding,
            "bytes",
            u64::try_from(document.bytes.len())
                .map_err(|_| "D10 companion length does not fit u64".to_owned())?,
            "D10 policy companion length",
        )?;
        expect_string(
            binding,
            "sha256",
            &sha256_identity(&document.bytes),
            "D10 policy companion identity",
        )?;
    }
    Ok(value.clone())
}

fn validate_timing(value: &Value) -> BenchResult<()> {
    validate_identity_companion(
        value,
        "FERRIC-M1-D10-TIMING-POLICY-V1",
        &[
            "clock_source_sha256",
            "iteration_boundary_sha256",
            "synchronization_sha256",
            "timer_overhead_sha256",
        ],
        "D10 timing policy",
    )
}

fn validate_tuning(value: &Value) -> BenchResult<String> {
    let tuning = exact_object(
        value,
        &[
            "authority",
            "budget_unit",
            "calibration_roster_sha256",
            "ferric_budget",
            "format",
            "search_protocol_sha256",
            "vendor_budget",
        ],
        "D10 tuning policy",
    )?;
    expect_companion_envelope(
        tuning,
        "FERRIC-M1-D10-TUNING-POLICY-V1",
        "D10 tuning policy",
    )?;
    require_safe_id(
        get_string(tuning, "budget_unit", "D10 tuning budget unit")?,
        "D10 tuning budget unit",
    )?;
    let ferric_budget = get_u64(tuning, "ferric_budget", "D10 Ferric tuning budget")?;
    let vendor_budget = get_u64(tuning, "vendor_budget", "D10 vendor tuning budget")?;
    require_bounded_positive(ferric_budget, "D10 Ferric tuning budget")?;
    require_bounded_positive(vendor_budget, "D10 vendor tuning budget")?;
    if ferric_budget != vendor_budget {
        return Err("D10 Ferric and vendor tuning budgets must be equal".to_owned());
    }
    let calibration = get_string(tuning, "calibration_roster_sha256", "D10 tuning identity")?;
    require_sha256(calibration)?;
    require_sha256(get_string(
        tuning,
        "search_protocol_sha256",
        "D10 tuning identity",
    )?)?;
    Ok(calibration.to_owned())
}

fn validate_execution_order(value: &Value) -> BenchResult<()> {
    let order = exact_object(
        value,
        &["authority", "cases", "format", "sample_id_protocol_sha256"],
        "D10 execution-order policy",
    )?;
    expect_companion_envelope(
        order,
        "FERRIC-M1-D10-EXECUTION-ORDER-POLICY-V1",
        "D10 execution-order policy",
    )?;
    require_sha256(get_string(
        order,
        "sample_id_protocol_sha256",
        "D10 sample-id protocol identity",
    )?)?;
    let cases = get(order, "cases", "D10 execution-order policy")?
        .as_array()
        .ok_or_else(|| "D10 execution-order cases must be an array".to_owned())?;
    if cases.len() != CASE_ROSTER.len() {
        return Err("D10 execution-order case count drifted".to_owned());
    }
    for (case, (case_id, _)) in cases.iter().zip(CASE_ROSTER) {
        let case = exact_object(
            case,
            &["case_id", "recorded_order_sha256", "warmup_order_sha256"],
            "D10 execution-order case",
        )?;
        expect_string(case, "case_id", case_id, "D10 execution-order case id")?;
        require_sha256(get_string(
            case,
            "recorded_order_sha256",
            "D10 recorded order identity",
        )?)?;
        require_sha256(get_string(
            case,
            "warmup_order_sha256",
            "D10 warmup order identity",
        )?)?;
    }
    Ok(())
}

fn validate_telemetry(value: &Value) -> BenchResult<()> {
    validate_identity_companion(
        value,
        "FERRIC-M1-D10-TELEMETRY-POLICY-V1",
        &[
            "clock_trace_sha256",
            "environment_snapshot_sha256",
            "error_trace_sha256",
            "temperature_trace_sha256",
        ],
        "D10 telemetry policy",
    )
}

fn validate_resources(value: &Value) -> BenchResult<()> {
    let resources = exact_object(
        value,
        &["authority", "cases", "format", "rejection_protocol_sha256"],
        "D10 resource-inspection policy",
    )?;
    expect_companion_envelope(
        resources,
        "FERRIC-M1-D10-RESOURCE-INSPECTION-POLICY-V1",
        "D10 resource-inspection policy",
    )?;
    require_sha256(get_string(
        resources,
        "rejection_protocol_sha256",
        "D10 resource rejection identity",
    )?)?;
    let cases = get(resources, "cases", "D10 resource-inspection policy")?
        .as_array()
        .ok_or_else(|| "D10 resource-inspection cases must be an array".to_owned())?;
    if cases.len() != CASE_ROSTER.len() {
        return Err("D10 resource-inspection case count drifted".to_owned());
    }
    for (case, (case_id, _)) in cases.iter().zip(CASE_ROSTER) {
        let case = exact_object(
            case,
            &[
                "artifact_manifest_sha256",
                "case_id",
                "expected_resources_sha256",
                "inspection_protocol_sha256",
            ],
            "D10 resource-inspection case",
        )?;
        expect_string(case, "case_id", case_id, "D10 resource-inspection case id")?;
        for field in [
            "artifact_manifest_sha256",
            "expected_resources_sha256",
            "inspection_protocol_sha256",
        ] {
            require_sha256(get_string(case, field, "D10 resource-inspection identity")?)?;
        }
    }
    Ok(())
}

fn validate_partition(
    value: &Value,
    format: &str,
    description: &str,
) -> BenchResult<PartitionRoster> {
    let partition = exact_object(
        value,
        &[
            "authority",
            "format",
            "members",
            "roster_sha256",
            "selection_protocol_sha256",
        ],
        description,
    )?;
    expect_companion_envelope(partition, format, description)?;
    let roster = get_string(partition, "roster_sha256", description)?;
    require_sha256(roster)?;
    let members_value = get(partition, "members", description)?;
    let members = members_value
        .as_array()
        .ok_or_else(|| format!("{description} members must be an array"))?;
    if members.is_empty() || members.len() > 512 {
        return Err(format!(
            "{description} member count is outside the admitted bound"
        ));
    }
    let actual_roster = sha256_identity(&encode_canonical_document(members_value)?);
    if roster != actual_roster {
        return Err(format!("{description} member roster identity drifted"));
    }
    let mut identities = BTreeSet::new();
    let mut member_ids = BTreeSet::new();
    let mut prior: Option<&str> = None;
    for member in members {
        let member = exact_object(member, &["id", "sha256"], description)?;
        let member_id = get_string(member, "id", description)?;
        require_safe_id(member_id, description)?;
        if prior.is_some_and(|prior| prior >= member_id) {
            return Err(format!(
                "{description} members must be uniquely sorted by id"
            ));
        }
        prior = Some(member_id);
        let identity = get_string(member, "sha256", description)?;
        require_sha256(identity)?;
        if !member_ids.insert(member_id.to_owned()) || !identities.insert(identity.to_owned()) {
            return Err(format!("{description} members must have unique identities"));
        }
    }
    require_sha256(get_string(
        partition,
        "selection_protocol_sha256",
        description,
    )?)?;
    Ok(PartitionRoster {
        digest: actual_roster,
        identities,
        member_ids,
    })
}

fn validate_regression_reference(value: &Value) -> BenchResult<()> {
    validate_identity_companion(
        value,
        "FERRIC-M1-D10-REGRESSION-REFERENCE-POLICY-V1",
        &[
            "artifact_sha256",
            "config_sha256",
            "implementation_sha256",
            "measurement_protocol_sha256",
            "measurement_roster_sha256",
        ],
        "D10 regression-reference policy",
    )
}

fn validate_identity_companion(
    value: &Value,
    format: &str,
    identity_fields: &[&str],
    description: &str,
) -> BenchResult<()> {
    let mut expected = vec!["authority", "format"];
    expected.extend_from_slice(identity_fields);
    let companion = exact_object(value, &expected, description)?;
    expect_companion_envelope(companion, format, description)?;
    for field in identity_fields {
        require_sha256(get_string(companion, field, description)?)?;
    }
    Ok(())
}

fn expect_companion_envelope(
    object: &Map<String, Value>,
    format: &str,
    description: &str,
) -> BenchResult<()> {
    expect_string(object, "authority", COMPANION_AUTHORITY, description)?;
    expect_string(object, "format", format, description)
}

fn exact_object<'a>(
    value: &'a Value,
    expected: &[&str],
    description: &str,
) -> BenchResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    exact_keys(object, expected, description)?;
    Ok(object)
}

fn exact_keys(
    object: &Map<String, Value>,
    expected: &[&str],
    description: &str,
) -> BenchResult<()> {
    if object.len() != expected.len() || !expected.iter().all(|field| object.contains_key(*field)) {
        return Err(format!("{description} fields drifted"));
    }
    Ok(())
}

fn get<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    description: &str,
) -> BenchResult<&'a Value> {
    object
        .get(field)
        .ok_or_else(|| format!("{description} is missing {field}"))
}

fn get_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    description: &str,
) -> BenchResult<&'a str> {
    get(object, field, description)?
        .as_str()
        .ok_or_else(|| format!("{description} field {field} must be a string"))
}

fn get_u64(object: &Map<String, Value>, field: &str, description: &str) -> BenchResult<u64> {
    get(object, field, description)?
        .as_u64()
        .ok_or_else(|| format!("{description} field {field} must be an unsigned integer"))
}

fn expect_string(
    object: &Map<String, Value>,
    field: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    if get_string(object, field, description)? != expected {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn expect_u64(
    object: &Map<String, Value>,
    field: &str,
    expected: u64,
    description: &str,
) -> BenchResult<()> {
    if get_u64(object, field, description)? != expected {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn expect_bool(
    object: &Map<String, Value>,
    field: &str,
    expected: bool,
    description: &str,
) -> BenchResult<()> {
    if get(object, field, description)?.as_bool() != Some(expected) {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn validate_string_array(value: &Value, expected: &[&str], description: &str) -> BenchResult<()> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{description} must be an array"))?;
    if values.len() != expected.len()
        || !values
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.as_str() == Some(*expected))
    {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn require_safe_id(value: &str, description: &str) -> BenchResult<()> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
    {
        return Err(format!("invalid {description}: {value}"));
    }
    Ok(())
}

fn require_sha256(value: &str) -> BenchResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == value.as_bytes()[0])
    {
        return Err("invalid SHA-256 identity".to_owned());
    }
    Ok(())
}

fn require_commit(value: &str, description: &str) -> BenchResult<()> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == value.as_bytes()[0])
    {
        return Err(format!("invalid {description}"));
    }
    Ok(())
}

fn require_bounded_positive(value: u64, description: &str) -> BenchResult<()> {
    if value == 0 || value > MAX_POLICY_INTEGER {
        return Err(format!("{description} is outside the structural bound"));
    }
    Ok(())
}

fn validate_root_path(path: &Path) -> BenchResult<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err("D10 policy root path is not admitted".to_owned());
    }
    Ok(())
}

fn validate_parent_path(path: &Path) -> BenchResult<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err("D10 admission parent path is not admitted".to_owned());
    }
    Ok(())
}

fn safe_output_name(path: &Path) -> BenchResult<OsString> {
    let name = path
        .file_name()
        .ok_or_else(|| "D10 admission output has no final component".to_owned())?;
    let bytes = name.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 255
        || !bytes.is_ascii()
        || matches!(bytes, b"." | b"..")
        || Path::new(name).components().count() != 1
    {
        return Err("invalid D10 admission output name".to_owned());
    }
    Ok(name.to_os_string())
}

fn validate_roster(root: &OwnedFd, expected: &[&str], description: &str) -> BenchResult<()> {
    let expected = expected
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    let mut directory =
        Dir::read_from(root).map_err(|error| format!("cannot enumerate {description}: {error}"))?;
    let mut actual = BTreeSet::new();
    while let Some(entry) = directory.read() {
        let entry = entry.map_err(|error| format!("cannot enumerate {description}: {error}"))?;
        let name = entry.file_name().to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        if !name.is_ascii() {
            return Err(format!("{description} filename must be ASCII"));
        }
        let name = std::str::from_utf8(name)
            .map_err(|_| format!("{description} filename must be UTF-8"))?;
        if !actual.insert(name.to_owned()) {
            return Err(format!("{description} contains a duplicate name"));
        }
    }
    if actual != expected {
        return Err(format!("{description} exact file roster drifted"));
    }
    Ok(())
}

fn open_directory_at(parent: &OwnedFd, path: &Path, description: &str) -> BenchResult<OwnedFd> {
    openat2(
        parent,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot securely open {description}: {error}"))
}

fn require_absent(parent: &OwnedFd, name: &OsStr, description: &str) -> BenchResult<()> {
    match openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) {
        Ok(_) => Err(format!("{description} already exists")),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(()),
        Err(error) => Err(format!("cannot safely inspect {description}: {error}")),
    }
}

fn validate_created_file(stat: &Stat, expected_size: u64, description: &str) -> BenchResult<()> {
    let actual_size = u64::try_from(stat.st_size)
        .map_err(|_| format!("created {description} size is invalid"))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_mode & 0o7777 != 0o600
        || stat.st_nlink != 1
        || stat.st_uid != geteuid().as_raw()
        || stat.st_gid != getegid().as_raw()
        || actual_size != expected_size
    {
        return Err(format!(
            "created {description} must retain exact 0600 owner/group/link/size custody"
        ));
    }
    Ok(())
}

// `mkdirat` and the descriptor open are not atomic. This validates the exact
// empty directory adopted after that boundary without claiming inode provenance.
fn validate_adopted_directory(stat: &Stat, description: &str) -> BenchResult<()> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || stat.st_mode & 0o7777 != 0o700
        || stat.st_uid != geteuid().as_raw()
        || stat.st_gid != getegid().as_raw()
    {
        return Err(format!(
            "adopted {description} must retain exact 0700 owner/group custody"
        ));
    }
    Ok(())
}

fn name_has_identity(parent: &OwnedFd, name: &OsStr, identity: &Stat) -> bool {
    let Ok(descriptor) = openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) else {
        return false;
    };
    fstat(&descriptor)
        .is_ok_and(|current| current.st_dev == identity.st_dev && current.st_ino == identity.st_ino)
}

fn cleanup_created_name(parent: &OwnedFd, name: &OsStr, identity: &Stat) {
    if name_has_identity(parent, name, identity) {
        let _ = unlinkat(parent, name, AtFlags::empty());
    }
}

fn same_snapshot(initial: &Stat, final_stat: &Stat) -> bool {
    initial.st_dev == final_stat.st_dev
        && initial.st_ino == final_stat.st_ino
        && initial.st_mode == final_stat.st_mode
        && initial.st_nlink == final_stat.st_nlink
        && initial.st_uid == final_stat.st_uid
        && initial.st_gid == final_stat.st_gid
        && initial.st_size == final_stat.st_size
        && initial.st_mtime == final_stat.st_mtime
        && initial.st_mtime_nsec == final_stat.st_mtime_nsec
        && initial.st_ctime == final_stat.st_ctime
        && initial.st_ctime_nsec == final_stat.st_ctime_nsec
}

fn same_directory_publication_transition(initial: &Stat, published: &Stat) -> bool {
    initial.st_dev == published.st_dev
        && initial.st_ino == published.st_ino
        && initial.st_mode == published.st_mode
        && initial.st_nlink == published.st_nlink
        && initial.st_uid == published.st_uid
        && initial.st_gid == published.st_gid
        && initial.st_size == published.st_size
        && initial.st_mtime == published.st_mtime
        && initial.st_mtime_nsec == published.st_mtime_nsec
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NONCE: AtomicU64 = AtomicU64::new(0);

    pub(crate) struct TestDirectory(pub(crate) PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-d10-policy-test.{}.{nonce}",
                std::process::id()
            ));
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn digest(label: &str) -> String {
        sha256_identity(label.as_bytes())
    }

    fn commit(label: &str) -> String {
        digest(label)[..40].to_owned()
    }

    fn protocol_bytes() -> Vec<u8> {
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("d10_policy_protocol.json")).unwrap()
    }

    fn identity_companion(format: &str, fields: &[&str]) -> Value {
        let mut value = Map::new();
        value.insert(
            "authority".to_owned(),
            Value::String(COMPANION_AUTHORITY.to_owned()),
        );
        value.insert("format".to_owned(), Value::String(format.to_owned()));
        for field in fields {
            value.insert((*field).to_owned(), Value::String(digest(field)));
        }
        Value::Object(value)
    }

    fn write_canonical(path: &Path, value: &Value) {
        fs::write(path, encode_canonical_document(value).unwrap()).unwrap();
    }

    fn member_roster_digest(members: &Value) -> String {
        sha256_identity(&encode_canonical_document(members).unwrap())
    }

    pub(crate) fn fixture() -> (TestDirectory, PathBuf, PathBuf) {
        let temporary = TestDirectory::new();
        let root = temporary.0.join("policy");
        fs::create_dir(&root).unwrap();
        let protocol = protocol_bytes();
        assert_eq!(sha256_identity(&protocol), PROTOCOL_SHA256);
        fs::write(root.join("protocol.json"), protocol).unwrap();

        let timing = identity_companion(
            "FERRIC-M1-D10-TIMING-POLICY-V1",
            &[
                "clock_source_sha256",
                "iteration_boundary_sha256",
                "synchronization_sha256",
                "timer_overhead_sha256",
            ],
        );
        let telemetry = identity_companion(
            "FERRIC-M1-D10-TELEMETRY-POLICY-V1",
            &[
                "clock_trace_sha256",
                "environment_snapshot_sha256",
                "error_trace_sha256",
                "temperature_trace_sha256",
            ],
        );
        let regression = identity_companion(
            "FERRIC-M1-D10-REGRESSION-REFERENCE-POLICY-V1",
            &[
                "artifact_sha256",
                "config_sha256",
                "implementation_sha256",
                "measurement_protocol_sha256",
                "measurement_roster_sha256",
            ],
        );
        let calibration_members = json!([
            {
                "id": "calibration-a",
                "sha256": digest("calibration-a"),
            },
            {
                "id": "calibration-b",
                "sha256": digest("calibration-b"),
            }
        ]);
        let holdout_members = json!([
            {
                "id": "holdout-a",
                "sha256": digest("holdout-a"),
            },
            {
                "id": "holdout-b",
                "sha256": digest("holdout-b"),
            }
        ]);
        let calibration_digest = member_roster_digest(&calibration_members);
        let holdout_digest = member_roster_digest(&holdout_members);
        let tuning = json!({
            "authority": COMPANION_AUTHORITY,
            "budget_unit": "candidate-builds",
            "calibration_roster_sha256": calibration_digest,
            "ferric_budget": 17,
            "format": "FERRIC-M1-D10-TUNING-POLICY-V1",
            "search_protocol_sha256": digest("search-protocol"),
            "vendor_budget": 17,
        });
        let order_cases = CASE_ROSTER
            .iter()
            .map(|(case_id, _)| {
                json!({
                    "case_id": case_id,
                    "recorded_order_sha256": digest(&format!("{case_id}-recorded")),
                    "warmup_order_sha256": digest(&format!("{case_id}-warmup")),
                })
            })
            .collect::<Vec<_>>();
        let execution_order = json!({
            "authority": COMPANION_AUTHORITY,
            "cases": order_cases,
            "format": "FERRIC-M1-D10-EXECUTION-ORDER-POLICY-V1",
            "sample_id_protocol_sha256": digest("sample-id-protocol"),
        });
        let resource_cases = CASE_ROSTER
            .iter()
            .map(|(case_id, _)| {
                json!({
                    "artifact_manifest_sha256": digest(&format!("{case_id}-artifact")),
                    "case_id": case_id,
                    "expected_resources_sha256": digest(&format!("{case_id}-resources")),
                    "inspection_protocol_sha256": digest(&format!("{case_id}-inspection")),
                })
            })
            .collect::<Vec<_>>();
        let resources = json!({
            "authority": COMPANION_AUTHORITY,
            "cases": resource_cases,
            "format": "FERRIC-M1-D10-RESOURCE-INSPECTION-POLICY-V1",
            "rejection_protocol_sha256": digest("resource-rejection"),
        });
        let calibration = json!({
            "authority": COMPANION_AUTHORITY,
            "format": "FERRIC-M1-D10-CALIBRATION-POLICY-V1",
            "members": calibration_members,
            "roster_sha256": calibration_digest,
            "selection_protocol_sha256": digest("calibration-selection"),
        });
        let holdout = json!({
            "authority": COMPANION_AUTHORITY,
            "format": "FERRIC-M1-D10-HOLDOUT-POLICY-V1",
            "members": holdout_members,
            "roster_sha256": holdout_digest,
            "selection_protocol_sha256": digest("holdout-selection"),
        });
        let documents = [
            ("timing.json", timing),
            ("telemetry.json", telemetry),
            ("regression-reference.json", regression),
            ("tuning.json", tuning),
            ("execution-order.json", execution_order),
            ("resource-inspection.json", resources),
            ("calibration.json", calibration),
            ("holdout.json", holdout),
        ];
        for (path, value) in &documents {
            write_canonical(&root.join(path), value);
        }
        let companions = COMPANIONS
            .iter()
            .map(|(name, path)| {
                let bytes = fs::read(root.join(path)).unwrap();
                (
                    (*name).to_owned(),
                    json!({
                        "bytes": bytes.len(),
                        "path": path,
                        "sha256": sha256_identity(&bytes),
                    }),
                )
            })
            .collect::<Map<_, _>>();
        let cases = CASE_ROSTER
            .iter()
            .enumerate()
            .map(|(index, (case_id, family))| {
                json!({
                    "case_id": case_id,
                    "ferric_implementation_sha256": digest(&format!("{case_id}-ferric")),
                    "kernel_family": family,
                    "profile": {
                        "id": format!("profile-{}", index + 1),
                        "sha256": digest(&format!("{case_id}-profile")),
                    },
                    "vendor": {
                        "applicable": index % 2 == 0,
                        "config_sha256": if index % 2 == 0 { Value::String(digest(&format!("{case_id}-vendor-config"))) } else { Value::Null },
                        "implementation_sha256": if index % 2 == 0 { Value::String(digest(&format!("{case_id}-vendor"))) } else { Value::Null },
                    },
                    "weight": u64::try_from(index + 1).unwrap(),
                    "work_unit": {
                        "count_per_iteration": u64::try_from(index + 11).unwrap(),
                        "name": format!("{}-operations", case_id),
                        "semantics_sha256": digest(&format!("{case_id}-work-unit")),
                    },
                })
            })
            .collect::<Vec<_>>();
        let policy = json!({
            "authority": POLICY_AUTHORITY,
            "cases": cases,
            "companions": companions,
            "format": POLICY_FORMAT,
            "milestone": "M1",
            "nonclaim": POLICY_NONCLAIM,
            "obligation_id": "m1.r31",
            "path_id": "d10-bench",
            "protocol_sha256": PROTOCOL_SHA256,
            "sample_protocol": {
                "recorded_samples": RECORDED_SAMPLES,
                "warmups": WARMUPS,
            },
            "status": "pre-observation",
            "suite": "d10",
            "target": TARGET,
            "thresholds": {
                "maximum_regression_ppm": 7,
                "minimum_per_case_vendor_ratio_ppm": 11,
                "minimum_weighted_vendor_ratio_ppm": 13,
            },
            "toolchain": {
                "compiler_configuration_sha256": digest("compiler-configuration"),
                "compiler_worker_closure_sha256": digest("compiler-worker-closure"),
                "fe2o3_source_closure_sha256": digest("fe2o3-source-closure"),
                "fe2o3_source_commit": PINNED_FE2O3_SOURCE_COMMIT,
                "ferric_source_closure_sha256": digest("ferric-source-closure"),
                "ferric_source_commit": commit("ferric-source-commit"),
                "kfd_runtime_closure_sha256": digest("kfd-runtime-closure"),
                "runtime_closure_sha256": digest("runtime-closure"),
            },
        });
        write_canonical(&root.join("policy.json"), &policy);
        let output = temporary.0.join("admitted");
        (temporary, root, output)
    }

    fn arguments(root: &Path, output: &Path) -> Vec<OsString> {
        vec![
            OsString::from("admit-experiment-policy"),
            root.as_os_str().to_os_string(),
            output.as_os_str().to_os_string(),
        ]
    }

    fn mutate_policy(root: &Path, mutation: impl FnOnce(&mut Value)) {
        let path = root.join("policy.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        mutation(&mut value);
        write_canonical(&path, &value);
    }

    fn rewrite_companion(root: &Path, name: &str, path: &str, value: &Value) {
        write_canonical(&root.join(path), value);
        let bytes = fs::read(root.join(path)).unwrap();
        mutate_policy(root, |policy| {
            policy["companions"][name]["bytes"] = json!(bytes.len());
            policy["companions"][name]["sha256"] = json!(sha256_identity(&bytes));
        });
    }

    #[test]
    fn canonical_policy_publishes_partial_non_evidence_bundle() {
        let (_temporary, root, output) = fixture();
        admit_experiment_policy(&arguments(&root, &output)).unwrap();
        let admission: Value =
            serde_json::from_slice(&fs::read(output.join("admission.json")).unwrap()).unwrap();
        assert_eq!(admission["authority"], ADMISSION_AUTHORITY);
        assert_eq!(admission["status"], PARTIAL_STATUS);
        assert_eq!(admission["r31_closed"], false);
        assert_eq!(
            admission["toolchain_sha256"],
            validate_toolchain(&admission["toolchain"]).unwrap()
        );
        assert_eq!(
            admission["future_required_binding"],
            FUTURE_OBSERVATION_BINDING
        );
        assert_eq!(admission["legacy_plan_validate_policy_bound"], false);
        assert_eq!(admission["observation_counts_enforced"], false);
        assert_eq!(admission["observations_admitted"], false);
        assert_eq!(admission["r31_closed"], false);
        assert_eq!(admission["closes"], json!([]));
        assert_eq!(
            fs::read(output.join("protocol.json")).unwrap(),
            protocol_bytes()
        );
    }

    #[test]
    fn malformed_toolchain_commit_and_closure_values_fail_closed() {
        for field in [
            "compiler_configuration_sha256",
            "compiler_worker_closure_sha256",
            "fe2o3_source_closure_sha256",
            "ferric_source_closure_sha256",
            "kfd_runtime_closure_sha256",
            "runtime_closure_sha256",
        ] {
            let (_temporary, root, output) = fixture();
            mutate_policy(&root, |policy| {
                policy["toolchain"][field] = json!("not-a-sha256");
            });
            assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
        }

        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["toolchain"]["fe2o3_source_commit"] = json!(commit("wrong-fe2o3"));
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["toolchain"]["ferric_source_commit"] = json!("not-a-commit");
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }

    #[test]
    fn pinned_fe2o3_toolchain_commit_matches_every_workspace_dependency() {
        let workspace = include_str!("../../Cargo.toml");
        let dependencies = workspace
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                line.starts_with("fe2o3-") || line.starts_with("reserved-fe2o3-symbols")
            })
            .collect::<Vec<_>>();
        assert_eq!(dependencies.len(), 14);
        let revision = format!("rev = \"{PINNED_FE2O3_SOURCE_COMMIT}\"");
        assert!(dependencies.iter().all(|line| line.contains(&revision)));
    }

    #[test]
    fn case_profile_substitution_and_order_drift_fail_closed() {
        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["cases"][0]["kernel_family"] = json!("k1-gemm-gemv");
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["cases"].as_array_mut().unwrap().swap(0, 1);
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }

    #[test]
    fn exact_sample_counts_and_vendor_mapping_fail_closed() {
        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["sample_protocol"]["warmups"] = json!(9)
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["sample_protocol"]["recorded_samples"] = json!(31);
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["cases"][0]["vendor"]["applicable"] = json!(false);
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }

    #[test]
    fn unequal_tuning_budgets_fail_closed() {
        let (_temporary, root, output) = fixture();
        let path = root.join("tuning.json");
        let mut tuning: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        tuning["vendor_budget"] = json!(18);
        rewrite_companion(&root, "tuning", "tuning.json", &tuning);
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }

    #[test]
    fn companion_path_hash_and_extra_entry_fail_closed() {
        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["companions"]["timing"]["path"] = json!("../timing.json");
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        mutate_policy(&root, |policy| {
            policy["companions"]["timing"]["sha256"] = json!(digest("substitution"));
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        fs::write(root.join("extra.json"), b"{}\n").unwrap();
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }

    #[test]
    fn noncanonical_protocol_substitution_and_existing_output_fail_closed() {
        let (_temporary, root, output) = fixture();
        let policy: Value =
            serde_json::from_slice(&fs::read(root.join("policy.json")).unwrap()).unwrap();
        fs::write(
            root.join("policy.json"),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        let mut protocol: Value = serde_json::from_slice(&protocol_bytes()).unwrap();
        protocol["warmups"] = json!(11);
        write_canonical(&root.join("protocol.json"), &protocol);
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        fs::create_dir(&output).unwrap();
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }

    #[test]
    fn held_input_mutation_before_publication_fails_closed() {
        let (_temporary, root, output) = fixture();
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |root| {
                let path = root.join("timing.json");
                let mut timing: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                timing["clock_source_sha256"] = json!(digest("mutated-clock"));
                write_canonical(&path, &timing);
                Ok(())
            },
            |_| Ok(()),
            |_| Ok(()),
            |_| Ok(()),
        );
        assert!(result.is_err());
        assert!(!output.exists());
    }

    #[test]
    fn mkdir_to_open_substitutions_are_retained_without_unbound_cleanup() {
        let (_temporary, root, output) = fixture();
        let replacement = RefCell::new(None);
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |staging| {
                fs::remove_dir(staging).unwrap();
                fs::write(staging, b"caller-owned-replacement").unwrap();
                fs::set_permissions(staging, fs::Permissions::from_mode(0o600)).unwrap();
                replacement.replace(Some(staging.to_path_buf()));
                Ok(())
            },
            |_| Ok(()),
            |_| Ok(()),
        );
        assert!(result.is_err());
        let replacement = replacement.into_inner().unwrap();
        assert_eq!(fs::read(replacement).unwrap(), b"caller-owned-replacement");
        assert!(!output.exists());

        let (_temporary, root, output) = fixture();
        let replacement = RefCell::new(None);
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |staging| {
                fs::remove_dir(staging).unwrap();
                fs::create_dir(staging).unwrap();
                fs::set_permissions(staging, fs::Permissions::from_mode(0o700)).unwrap();
                replacement.replace(Some(staging.to_path_buf()));
                Ok(())
            },
            |_| Err("stop after adopted staging replacement".to_owned()),
            |_| Ok(()),
        );
        assert!(result.is_err());
        let replacement = replacement.into_inner().unwrap();
        assert!(replacement.is_dir());
        assert_eq!(fs::read_dir(replacement).unwrap().count(), 0);
        assert!(!output.exists());
    }

    #[test]
    fn staged_content_and_mode_mutation_fail_before_publication() {
        let (_temporary, root, output) = fixture();
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |staging| {
                fs::write(staging.join("admission.json"), b"{}\n").unwrap();
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(result.is_err());
        assert!(!output.exists());

        let (_temporary, root, output) = fixture();
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |staging| {
                fs::set_permissions(
                    staging.join("admission.json"),
                    fs::Permissions::from_mode(0o400),
                )
                .unwrap();
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(result.is_err());
        assert!(!output.exists());
    }

    #[test]
    fn identical_byte_name_replacement_is_rejected_and_not_deleted() {
        let (_temporary, root, output) = fixture();
        let retained_staging = RefCell::new(None);
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |staging| {
                let target = staging.join("admission.json");
                let bytes = fs::read(&target).unwrap();
                let original = staging.parent().unwrap().join("retained-original.json");
                fs::rename(&target, original).unwrap();
                fs::write(&target, bytes).unwrap();
                fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
                retained_staging.replace(Some(staging.to_path_buf()));
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(result.is_err());
        assert!(!output.exists());
        let staging = retained_staging.into_inner().unwrap();
        assert!(staging.join("admission.json").exists());
    }

    #[test]
    fn staged_extra_entry_name_move_and_directory_mode_drift_fail_closed() {
        let (_temporary, root, output) = fixture();
        let retained_staging = RefCell::new(None);
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |staging| {
                fs::write(staging.join("extra.json"), b"{}\n").unwrap();
                retained_staging.replace(Some(staging.to_path_buf()));
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(result.is_err());
        let staging = retained_staging.into_inner().unwrap();
        assert!(staging.join("extra.json").exists());

        let (_temporary, root, output) = fixture();
        let moved = RefCell::new(None);
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |staging| {
                let destination = staging.parent().unwrap().join("moved-staging");
                fs::rename(staging, &destination).unwrap();
                moved.replace(Some(destination));
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(result.is_err());
        assert!(moved.into_inner().unwrap().exists());

        let (_temporary, root, output) = fixture();
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |staging| {
                fs::set_permissions(staging, fs::Permissions::from_mode(0o750)).unwrap();
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(result.is_err());
        assert!(!output.exists());
    }

    #[test]
    fn post_rename_content_and_directory_ctime_drift_are_retained_as_failures() {
        let (_temporary, root, output) = fixture();
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |_| Ok(()),
            |published| {
                fs::write(published.join("protocol.json"), b"{}\n").unwrap();
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(output.exists());

        let (_temporary, root, output) = fixture();
        let result = admit_experiment_policy_with_hooks(
            &arguments(&root, &output),
            |_| Ok(()),
            |_| Ok(()),
            |_| Ok(()),
            |published| {
                let transient = published.join("transient.json");
                let baseline = fs::metadata(published).unwrap();
                for _ in 0..100 {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    fs::write(&transient, b"{}\n").unwrap();
                    fs::remove_file(&transient).unwrap();
                    let current = fs::metadata(published).unwrap();
                    if current.ctime() != baseline.ctime()
                        || current.ctime_nsec() != baseline.ctime_nsec()
                    {
                        return Ok(());
                    }
                }
                panic!("test could not induce published directory ctime drift")
            },
        );
        assert!(result.is_err());
        assert!(output.exists());
    }

    #[test]
    fn partition_membership_reorder_overlap_and_substitution_fail_closed() {
        let (_temporary, root, output) = fixture();
        let path = root.join("calibration.json");
        let mut calibration: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        calibration["members"].as_array_mut().unwrap().swap(0, 1);
        calibration["roster_sha256"] = json!(member_roster_digest(&calibration["members"]));
        rewrite_companion(&root, "calibration", "calibration.json", &calibration);
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        let calibration: Value =
            serde_json::from_slice(&fs::read(root.join("calibration.json")).unwrap()).unwrap();
        let path = root.join("holdout.json");
        let mut holdout: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        holdout["members"][0] = calibration["members"][0].clone();
        holdout["roster_sha256"] = json!(member_roster_digest(&holdout["members"]));
        rewrite_companion(&root, "holdout", "holdout.json", &holdout);
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        let path = root.join("holdout.json");
        let mut holdout: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        holdout["members"][0]["sha256"] = json!(digest("substituted-holdout-member"));
        rewrite_companion(&root, "holdout", "holdout.json", &holdout);
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        let path = root.join("holdout.json");
        let mut holdout: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        holdout["members"] = json!([]);
        holdout["roster_sha256"] = json!(member_roster_digest(&holdout["members"]));
        rewrite_companion(&root, "holdout", "holdout.json", &holdout);
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }

    #[test]
    fn calibration_holdout_alias_and_companion_order_fail_closed() {
        let (_temporary, root, output) = fixture();
        let calibration: Value =
            serde_json::from_slice(&fs::read(root.join("calibration.json")).unwrap()).unwrap();
        let mut holdout = calibration;
        holdout["format"] = json!("FERRIC-M1-D10-HOLDOUT-POLICY-V1");
        write_canonical(&root.join("holdout.json"), &holdout);
        mutate_policy(&root, |policy| {
            let bytes = fs::read(root.join("holdout.json")).unwrap();
            policy["companions"]["holdout"]["bytes"] = json!(bytes.len());
            policy["companions"]["holdout"]["sha256"] = json!(sha256_identity(&bytes));
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());

        let (_temporary, root, output) = fixture();
        let path = root.join("execution-order.json");
        let mut order: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        order["cases"].as_array_mut().unwrap().swap(0, 1);
        write_canonical(&path, &order);
        mutate_policy(&root, |policy| {
            let bytes = fs::read(&path).unwrap();
            policy["companions"]["execution-order"]["bytes"] = json!(bytes.len());
            policy["companions"]["execution-order"]["sha256"] = json!(sha256_identity(&bytes));
        });
        assert!(admit_experiment_policy(&arguments(&root, &output)).is_err());
    }
}
