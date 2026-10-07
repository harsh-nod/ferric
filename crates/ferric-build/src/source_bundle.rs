//! Descriptor-held intake for the exact upstream Qwen3 M1 source bundle.
//!
//! This boundary admits only the target Qwen3-8B and draft Qwen3-0.6B files
//! consumed by Ferric's byte authenticators. It rejects traversal through
//! symlinks, nonregular and multiply linked files, incomplete or extra roster
//! entries, wrong file lengths, duplicate file identities, and metadata drift
//! during bounded reads. Content authentication remains the responsibility of
//! the existing config, tokenizer, and safetensors admission paths.

use crate::safetensors::{DRAFT_FILE_PIN, TARGET_INDEX_BYTES, TARGET_SHARD_PINS};
use crate::{
    SafetensorsSource, QWEN3_DRAFT_CONFIG_BYTES, QWEN3_TARGET_CONFIG_BYTES, QWEN3_TOKENIZER_BYTES,
    QWEN3_TOKENIZER_METADATA_BYTES,
};
use rustix::fd::OwnedFd;
use rustix::fs::{fstat, openat2, Dir, FileType, Mode, OFlags, ResolveFlags, Stat, CWD};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const CONFIG_NAME: &str = "config.json";
const TOKENIZER_METADATA_NAME: &str = "tokenizer_config.json";
const TOKENIZER_NAME: &str = "tokenizer.json";
const TARGET_INDEX_NAME: &str = "model.safetensors.index.json";

#[derive(Clone, Copy)]
struct ExpectedFile {
    name: &'static str,
    bytes: u64,
}

const TARGET_FILES: [ExpectedFile; 9] = [
    ExpectedFile {
        name: CONFIG_NAME,
        bytes: QWEN3_TARGET_CONFIG_BYTES,
    },
    ExpectedFile {
        name: "model-00001-of-00005.safetensors",
        bytes: TARGET_SHARD_PINS[0].file_bytes,
    },
    ExpectedFile {
        name: "model-00002-of-00005.safetensors",
        bytes: TARGET_SHARD_PINS[1].file_bytes,
    },
    ExpectedFile {
        name: "model-00003-of-00005.safetensors",
        bytes: TARGET_SHARD_PINS[2].file_bytes,
    },
    ExpectedFile {
        name: "model-00004-of-00005.safetensors",
        bytes: TARGET_SHARD_PINS[3].file_bytes,
    },
    ExpectedFile {
        name: "model-00005-of-00005.safetensors",
        bytes: TARGET_SHARD_PINS[4].file_bytes,
    },
    ExpectedFile {
        name: TARGET_INDEX_NAME,
        bytes: TARGET_INDEX_BYTES as u64,
    },
    ExpectedFile {
        name: TOKENIZER_NAME,
        bytes: QWEN3_TOKENIZER_BYTES,
    },
    ExpectedFile {
        name: TOKENIZER_METADATA_NAME,
        bytes: QWEN3_TOKENIZER_METADATA_BYTES,
    },
];

const DRAFT_FILES: [ExpectedFile; 4] = [
    ExpectedFile {
        name: CONFIG_NAME,
        bytes: QWEN3_DRAFT_CONFIG_BYTES,
    },
    ExpectedFile {
        name: "model.safetensors",
        bytes: DRAFT_FILE_PIN.file_bytes,
    },
    ExpectedFile {
        name: TOKENIZER_NAME,
        bytes: QWEN3_TOKENIZER_BYTES,
    },
    ExpectedFile {
        name: TOKENIZER_METADATA_NAME,
        bytes: QWEN3_TOKENIZER_METADATA_BYTES,
    },
];

/// Exact byte-backed metadata and descriptor-held weight streams from one
/// canonical target/draft source bundle.
///
/// The fields are bytes and forward-only readers, not model-admission
/// authority. Callers must pass them through Ferric's existing authenticators.
pub struct CanonicalQwen3SourceBundle {
    /// Complete exact target `config.json` bytes.
    pub target_config: Vec<u8>,
    /// Complete exact target `tokenizer_config.json` bytes.
    pub target_tokenizer_metadata: Vec<u8>,
    /// Complete exact draft `config.json` bytes.
    pub draft_config: Vec<u8>,
    /// Complete exact draft `tokenizer_config.json` bytes.
    pub draft_tokenizer_metadata: Vec<u8>,
    /// Shared exact target/draft `tokenizer.json` bytes.
    pub tokenizer: Vec<u8>,
    /// Complete exact target safetensors index bytes.
    pub target_index: Vec<u8>,
    /// Five held target safetensors streams in canonical shard order.
    pub target_shards: [SafetensorsSource<'static, CanonicalQwen3SourceFile>; 5],
    /// One held draft safetensors stream.
    pub draft_weights: SafetensorsSource<'static, CanonicalQwen3SourceFile>,
}

/// A no-follow, singly linked regular file held from roster admission through
/// the safetensors authenticator's final EOF observation.
pub struct CanonicalQwen3SourceFile {
    file: File,
    initial: Stat,
    description: &'static str,
    eof_validated: bool,
}

impl std::fmt::Debug for CanonicalQwen3SourceFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CanonicalQwen3SourceFile")
            .field("description", &self.description)
            .field("bytes", &self.initial.st_size)
            .field("eof_validated", &self.eof_validated)
            .finish_non_exhaustive()
    }
}

impl Read for CanonicalQwen3SourceFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.file.read(buffer)?;
        if read == 0 && !buffer.is_empty() && !self.eof_validated {
            validate_snapshot(&self.file, &self.initial, self.description)?;
            self.eof_validated = true;
        }
        Ok(read)
    }
}

/// Opens and admits the exact two-directory Qwen3 M1 source bundle.
///
/// The root must contain only `target` and `draft`. Each model directory must
/// contain exactly the files Ferric consumes, with the pinned byte lengths.
/// Every path component and child is opened without symlink traversal. Weight
/// content is deliberately not trusted here; the returned readers must still
/// pass the existing full-file SHA-256 and safetensors schema admission.
///
/// # Errors
///
/// Returns an I/O error for path traversal, unsafe file types or links,
/// missing, extra, aliased, or wrong-sized entries, concurrent metadata drift,
/// incomplete bounded reads, or unequal target/draft tokenizer bytes.
pub fn open_canonical_qwen3_source_bundle(root: &Path) -> io::Result<CanonicalQwen3SourceBundle> {
    let root = open_root(root)?;
    let root_initial = directory_snapshot(&root, "Qwen source root")?;
    let target = open_directory(&root, "target", "target source directory")?;
    let draft = open_directory(&root, "draft", "draft source directory")?;
    let target_initial = directory_snapshot(&target, "target source directory")?;
    let draft_initial = directory_snapshot(&draft, "draft source directory")?;

    validate_directory_roster(&root, &["draft", "target"], "Qwen source root")?;
    validate_directory_roster(
        &target,
        &TARGET_FILES.map(|expected| expected.name),
        "target source directory",
    )?;
    validate_directory_roster(
        &draft,
        &DRAFT_FILES.map(|expected| expected.name),
        "draft source directory",
    )?;

    let mut target_files = open_expected_files(&target, &TARGET_FILES, "target")?;
    let mut draft_files = open_expected_files(&draft, &DRAFT_FILES, "draft")?;
    validate_distinct_files(&target_files, &draft_files)?;

    validate_directory_snapshot(&root, &root_initial, "Qwen source root")?;
    validate_directory_snapshot(&target, &target_initial, "target source directory")?;
    validate_directory_snapshot(&draft, &draft_initial, "draft source directory")?;

    let target_config = read_metadata(take_file(&mut target_files, CONFIG_NAME)?)?;
    let target_tokenizer_metadata =
        read_metadata(take_file(&mut target_files, TOKENIZER_METADATA_NAME)?)?;
    let target_tokenizer = read_metadata(take_file(&mut target_files, TOKENIZER_NAME)?)?;
    let target_index = read_metadata(take_file(&mut target_files, TARGET_INDEX_NAME)?)?;
    let draft_config = read_metadata(take_file(&mut draft_files, CONFIG_NAME)?)?;
    let draft_tokenizer_metadata =
        read_metadata(take_file(&mut draft_files, TOKENIZER_METADATA_NAME)?)?;
    let draft_tokenizer = read_metadata(take_file(&mut draft_files, TOKENIZER_NAME)?)?;
    if target_tokenizer != draft_tokenizer {
        return Err(invalid_data(
            "target and draft tokenizer bytes are not identical",
        ));
    }

    let mut target_sources = Vec::with_capacity(TARGET_SHARD_PINS.len());
    for pin in TARGET_SHARD_PINS {
        target_sources.push(SafetensorsSource {
            name: pin.name,
            reader: take_file(&mut target_files, pin.name)?,
        });
    }
    let target_shards = target_sources
        .try_into()
        .map_err(|_| invalid_data("target shard custody count drifted"))?;
    let draft_weight = take_file(&mut draft_files, DRAFT_FILE_PIN.name)?;
    if target_files.iter().any(Option::is_some) || draft_files.iter().any(Option::is_some) {
        return Err(invalid_data("source file custody roster was not consumed"));
    }

    validate_directory_snapshot(&root, &root_initial, "Qwen source root")?;
    validate_directory_snapshot(&target, &target_initial, "target source directory")?;
    validate_directory_snapshot(&draft, &draft_initial, "draft source directory")?;

    Ok(CanonicalQwen3SourceBundle {
        target_config,
        target_tokenizer_metadata,
        draft_config,
        draft_tokenizer_metadata,
        tokenizer: target_tokenizer,
        target_index,
        target_shards,
        draft_weights: SafetensorsSource {
            name: DRAFT_FILE_PIN.name,
            reader: draft_weight,
        },
    })
}

fn open_root(path: &Path) -> io::Result<OwnedFd> {
    if path.as_os_str().is_empty() {
        return Err(invalid_input("Qwen source root path is empty"));
    }
    openat2(
        CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(io::Error::from)
}

fn open_directory(
    parent: &OwnedFd,
    name: &'static str,
    description: &'static str,
) -> io::Result<OwnedFd> {
    let directory = openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(io::Error::from)?;
    directory_snapshot(&directory, description)?;
    Ok(directory)
}

fn open_expected_files<const N: usize>(
    directory: &OwnedFd,
    expected: &[ExpectedFile; N],
    role: &'static str,
) -> io::Result<[Option<CanonicalQwen3SourceFile>; N]> {
    let mut files: [Option<CanonicalQwen3SourceFile>; N] = std::array::from_fn(|_| None);
    for (slot, expected) in files.iter_mut().zip(expected) {
        let description = match (role, expected.name) {
            ("target", CONFIG_NAME) => "target config",
            ("target", TOKENIZER_METADATA_NAME) => "target tokenizer metadata",
            ("target", TOKENIZER_NAME) => "target tokenizer",
            ("target", TARGET_INDEX_NAME) => "target safetensors index",
            ("target", "model-00001-of-00005.safetensors") => "target safetensors shard 1",
            ("target", "model-00002-of-00005.safetensors") => "target safetensors shard 2",
            ("target", "model-00003-of-00005.safetensors") => "target safetensors shard 3",
            ("target", "model-00004-of-00005.safetensors") => "target safetensors shard 4",
            ("target", "model-00005-of-00005.safetensors") => "target safetensors shard 5",
            ("draft", CONFIG_NAME) => "draft config",
            ("draft", TOKENIZER_METADATA_NAME) => "draft tokenizer metadata",
            ("draft", TOKENIZER_NAME) => "draft tokenizer",
            ("draft", "model.safetensors") => "draft safetensors",
            _ => return Err(invalid_data("unknown canonical Qwen source file")),
        };
        *slot = Some(open_exact_file(
            directory,
            expected.name,
            expected.bytes,
            description,
        )?);
    }
    Ok(files)
}

fn open_exact_file(
    directory: &OwnedFd,
    name: &'static str,
    expected_bytes: u64,
    description: &'static str,
) -> io::Result<CanonicalQwen3SourceFile> {
    let descriptor = openat2(
        directory,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(io::Error::from)?;
    let initial = fstat(&descriptor).map_err(io::Error::from)?;
    if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
        || initial.st_nlink != 1
        || u64::try_from(initial.st_size).ok() != Some(expected_bytes)
    {
        return Err(invalid_data(&format!(
            "{description} is not one singly linked regular file of {expected_bytes} bytes"
        )));
    }
    Ok(CanonicalQwen3SourceFile {
        file: File::from(descriptor),
        initial,
        description,
        eof_validated: false,
    })
}

fn validate_directory_roster(
    directory: &OwnedFd,
    expected: &[&str],
    description: &str,
) -> io::Result<()> {
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut entries = Dir::read_from(directory).map_err(io::Error::from)?;
    while let Some(entry) = entries.read() {
        let entry = entry.map_err(io::Error::from)?;
        let name = entry.file_name();
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        let Ok(name) = name.to_str() else {
            return Err(invalid_data(&format!(
                "{description} contains a non-UTF-8 name"
            )));
        };
        if !expected.contains(name) || !seen.insert(name.to_owned()) {
            return Err(invalid_data(&format!(
                "{description} contains unexpected or repeated entry {name:?}"
            )));
        }
    }
    if seen.len() != expected.len() || expected.iter().any(|name| !seen.contains(*name)) {
        return Err(invalid_data(&format!(
            "{description} is missing a canonical entry"
        )));
    }
    Ok(())
}

fn validate_distinct_files<const T: usize, const D: usize>(
    target: &[Option<CanonicalQwen3SourceFile>; T],
    draft: &[Option<CanonicalQwen3SourceFile>; D],
) -> io::Result<()> {
    let mut identities = BTreeSet::new();
    for file in target.iter().chain(draft).flatten() {
        if !identities.insert((file.initial.st_dev, file.initial.st_ino)) {
            return Err(invalid_data(
                "two canonical source names alias one filesystem object",
            ));
        }
    }
    Ok(())
}

fn take_file<const N: usize>(
    files: &mut [Option<CanonicalQwen3SourceFile>; N],
    name: &str,
) -> io::Result<CanonicalQwen3SourceFile> {
    let expectations: &[ExpectedFile] = if N == TARGET_FILES.len() {
        &TARGET_FILES
    } else if N == DRAFT_FILES.len() {
        &DRAFT_FILES
    } else {
        return Err(invalid_data("unknown source custody roster"));
    };
    let index = expectations
        .iter()
        .position(|expected| expected.name == name)
        .ok_or_else(|| invalid_data("source custody name is absent"))?;
    files[index]
        .take()
        .ok_or_else(|| invalid_data("source custody name was consumed twice"))
}

fn read_metadata(mut file: CanonicalQwen3SourceFile) -> io::Result<Vec<u8>> {
    let length = usize::try_from(file.initial.st_size)
        .map_err(|_| invalid_data("metadata length does not fit this host"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| invalid_data("metadata allocation failed"))?;
    file.read_to_end(&mut bytes)?;
    if bytes.len() != length || !file.eof_validated {
        return Err(invalid_data(
            "metadata read did not reach the exact stable EOF",
        ));
    }
    Ok(bytes)
}

fn directory_snapshot(directory: &OwnedFd, description: &str) -> io::Result<Stat> {
    let snapshot = fstat(directory).map_err(io::Error::from)?;
    if FileType::from_raw_mode(snapshot.st_mode) != FileType::Directory {
        return Err(invalid_data(&format!("{description} is not a directory")));
    }
    Ok(snapshot)
}

fn validate_directory_snapshot(
    directory: &OwnedFd,
    initial: &Stat,
    description: &str,
) -> io::Result<()> {
    let current = directory_snapshot(directory, description)?;
    if !same_snapshot(initial, &current) {
        return Err(invalid_data(&format!(
            "{description} changed during source intake"
        )));
    }
    Ok(())
}

fn validate_snapshot(file: &File, initial: &Stat, description: &str) -> io::Result<()> {
    let current = fstat(file).map_err(io::Error::from)?;
    if FileType::from_raw_mode(current.st_mode) != FileType::RegularFile
        || current.st_nlink != 1
        || !same_snapshot(initial, &current)
    {
        return Err(invalid_data(&format!(
            "{description} changed during source authentication"
        )));
    }
    Ok(())
}

fn same_snapshot(initial: &Stat, current: &Stat) -> bool {
    initial.st_dev == current.st_dev
        && initial.st_ino == current.st_ino
        && initial.st_mode == current.st_mode
        && initial.st_nlink == current.st_nlink
        && initial.st_size == current.st_size
        && initial.st_mtime == current.st_mtime
        && initial.st_mtime_nsec == current.st_mtime_nsec
        && initial.st_ctime == current.st_ctime
        && initial.st_ctime_nsec == current.st_ctime_nsec
}

fn invalid_data(reason: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid canonical Qwen3 source bundle: {reason}"),
    )
}

fn invalid_input(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, reason)
}

#[cfg(test)]
mod tests {
    use super::{
        directory_snapshot, open_canonical_qwen3_source_bundle, open_exact_file, same_snapshot,
        validate_directory_roster, validate_snapshot,
    };
    use rustix::fs::{openat2, Mode, OFlags, ResolveFlags, CWD};
    use std::fs;
    use std::io::{Read, Write as _};
    use std::os::unix::ffi::OsStringExt as _;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ORDINAL: AtomicU64 = AtomicU64::new(0);

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ferric-qwen-source-{label}-{}-{}",
            std::process::id(),
            TEST_ORDINAL.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn open_directory(path: &Path) -> rustix::fd::OwnedFd {
        openat2(
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
        .unwrap()
    }

    fn create_sparse_file(path: &Path, bytes: u64) {
        let file = fs::File::create(path).unwrap();
        file.set_len(bytes).unwrap();
    }

    fn create_source_shape(root: &Path) {
        let target = root.join("target");
        let draft = root.join("draft");
        fs::create_dir(root).unwrap();
        fs::create_dir(&target).unwrap();
        fs::create_dir(&draft).unwrap();
        for expected in super::TARGET_FILES {
            create_sparse_file(&target.join(expected.name), expected.bytes);
        }
        for expected in super::DRAFT_FILES {
            create_sparse_file(&draft.join(expected.name), expected.bytes);
        }
    }

    #[test]
    fn complete_source_shape_is_held_and_outer_hostiles_fail_closed() {
        let root = test_root("complete");
        create_source_shape(&root);
        let bundle = open_canonical_qwen3_source_bundle(&root).unwrap();
        assert_eq!(
            bundle.target_config.len(),
            usize::try_from(super::QWEN3_TARGET_CONFIG_BYTES).unwrap()
        );
        assert_eq!(bundle.target_shards.len(), 5);
        assert_eq!(
            bundle.tokenizer.len(),
            usize::try_from(super::QWEN3_TOKENIZER_BYTES).unwrap()
        );
        drop(bundle);

        fs::write(root.join("ignored-file"), b"not canonical").unwrap();
        assert!(open_canonical_qwen3_source_bundle(&root).is_err());
        fs::remove_file(root.join("ignored-file")).unwrap();

        let alias = root.with_extension("symlink");
        symlink(&root, &alias).unwrap();
        assert!(open_canonical_qwen3_source_bundle(&alias).is_err());
        fs::remove_file(alias).unwrap();

        let draft_tokenizer = root.join("draft/tokenizer.json");
        let mut changed = fs::OpenOptions::new()
            .write(true)
            .open(&draft_tokenizer)
            .unwrap();
        changed.write_all(&[1]).unwrap();
        drop(changed);
        assert!(open_canonical_qwen3_source_bundle(&root).is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_roster_rejects_missing_extra_and_non_utf8_names() {
        let root = test_root("roster");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a"), b"a").unwrap();
        fs::write(root.join("b"), b"b").unwrap();
        let directory = open_directory(&root);
        validate_directory_roster(&directory, &["a", "b"], "test roster").unwrap();
        fs::write(root.join("extra"), b"x").unwrap();
        assert!(validate_directory_roster(&open_directory(&root), &["a", "b"], "test").is_err());
        fs::remove_file(root.join("extra")).unwrap();
        let non_utf8 = std::ffi::OsString::from_vec(vec![0xff]);
        fs::write(root.join(&non_utf8), b"x").unwrap();
        assert!(validate_directory_roster(&open_directory(&root), &["a", "b"], "test").is_err());
        fs::remove_file(root.join(non_utf8)).unwrap();
        fs::remove_file(root.join("b")).unwrap();
        assert!(validate_directory_roster(&open_directory(&root), &["a", "b"], "test").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_file_rejects_symlink_hardlink_directory_fifo_and_wrong_size() {
        let root = test_root("types");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("exact"), b"1234").unwrap();
        let directory = open_directory(&root);
        assert!(open_exact_file(&directory, "exact", 4, "exact").is_ok());
        assert!(open_exact_file(&directory, "exact", 3, "exact").is_err());

        symlink("exact", root.join("link")).unwrap();
        assert!(open_exact_file(&directory, "link", 4, "link").is_err());
        fs::hard_link(root.join("exact"), root.join("hard")).unwrap();
        assert!(open_exact_file(&directory, "exact", 4, "exact").is_err());
        fs::create_dir(root.join("dir")).unwrap();
        assert!(open_exact_file(&directory, "dir", 0, "dir").is_err());
        rustix::fs::mkfifoat(&directory, Path::new("fifo"), Mode::RUSR | Mode::WUSR).unwrap();
        assert!(open_exact_file(&directory, "fifo", 0, "fifo").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn held_reader_rejects_metadata_drift_at_eof() {
        let root = test_root("drift");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("input"), b"1234").unwrap();
        let directory = open_directory(&root);
        let mut held = open_exact_file(&directory, "input", 4, "input").unwrap();
        assert_eq!(held.read(&mut []).unwrap(), 0);
        assert!(!held.eof_validated);
        let mut bytes = [0; 4];
        held.read_exact(&mut bytes).unwrap();
        fs::hard_link(root.join("input"), root.join("hostile-alias")).unwrap();
        assert!(held.read(&mut [0; 1]).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unchanged_file_and_directory_snapshots_are_stable() {
        let root = test_root("stable");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("input"), b"1234").unwrap();
        let directory = open_directory(&root);
        let initial_directory = directory_snapshot(&directory, "test directory").unwrap();
        let mut held = open_exact_file(&directory, "input", 4, "input").unwrap();
        let mut bytes = Vec::new();
        held.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"1234");
        validate_snapshot(&held.file, &held.initial, "input").unwrap();
        let current_directory = directory_snapshot(&directory, "test directory").unwrap();
        assert!(same_snapshot(&initial_directory, &current_directory));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_expectation_tables_are_strictly_ordered() {
        for table in [&super::TARGET_FILES[..], &super::DRAFT_FILES[..]] {
            for pair in table.windows(2) {
                assert!(
                    pair[0].name < pair[1].name,
                    "canonical roster is not ordered: {} >= {}",
                    pair[0].name,
                    pair[1].name
                );
            }
        }
    }
}
