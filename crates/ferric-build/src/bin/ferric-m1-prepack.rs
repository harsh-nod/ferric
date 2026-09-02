#![forbid(unsafe_code)]

//! Offline publication and verification of the canonical M1 prepacked snapshot.
//!
//! The exact regular-file roster and semantic intake checks do not provide a
//! signature or independent-validation authority. As recorded in `docs/M1_TCB.md`,
//! external filesystem behavior, path stability during intake, synchronization,
//! rename, and publication durability remain contracted host assumptions.

use ferric_build::{
    authenticate_qwen3_tokenizer, build_preliminary_deployment_bundle,
    build_prepacked_deployment_bundle, decode_bundle_admission_record,
    encode_canonical_deployment_bundle, open_canonical_qwen3_source_bundle,
    prepack_qwen3_draft_weights, prepack_qwen3_target_weights, reopen_persisted_qwen3_weights,
    seal_authenticated_bundle, ArtifactDigest, AuthenticatedDeploymentAssets,
    AuthenticatedModelAssets, AuthenticatedTokenizer, CanonicalQwen3SourceBundle, DeploymentAssets,
    ModelAssets, WeightDescriptor, BUNDLE_ADMISSION_RECORD_BYTES,
    CANONICAL_DEPLOYMENT_BUNDLE_BYTES, DRAFT_REPOSITORY, DRAFT_REVISION, QWEN3_DRAFT_CONFIG_BYTES,
    QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES, QWEN3_DRAFT_TENSOR_DATA_BYTES,
    QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_CONFIG_BYTES,
    QWEN3_TARGET_PREPACKED_MANIFEST_BYTES, QWEN3_TARGET_TENSOR_DATA_BYTES,
    QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_BYTES,
    QWEN3_TOKENIZER_METADATA_BYTES, QWEN3_TOKENIZER_SHA256, TARGET_REPOSITORY, TARGET_REVISION,
};
use ferric_spec::{EngineLimits, Qwen3ModelRole};
use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const TARGET_CONFIG_NAME: &str = "target.config.json";
const DRAFT_CONFIG_NAME: &str = "draft.config.json";
const TARGET_TOKENIZER_METADATA_NAME: &str = "target.tokenizer_config.json";
const DRAFT_TOKENIZER_METADATA_NAME: &str = "draft.tokenizer_config.json";
const TOKENIZER_NAME: &str = "tokenizer.json";
const TARGET_WEIGHTS_NAME: &str = "target.weights.bin";
const DRAFT_WEIGHTS_NAME: &str = "draft.weights.bin";
const TARGET_MANIFEST_NAME: &str = "target.weights.manifest.bin";
const DRAFT_MANIFEST_NAME: &str = "draft.weights.manifest.bin";
const DEPLOYMENT_BUNDLE_NAME: &str = "deployment.bundle.bin";
const BUNDLE_ADMISSION_NAME: &str = "bundle.admission.bin";

#[derive(Clone, Copy)]
struct SnapshotFile {
    name: &'static str,
    bytes: u64,
}

const SNAPSHOT_FILES: [SnapshotFile; 11] = [
    SnapshotFile {
        name: BUNDLE_ADMISSION_NAME,
        bytes: BUNDLE_ADMISSION_RECORD_BYTES as u64,
    },
    SnapshotFile {
        name: DEPLOYMENT_BUNDLE_NAME,
        bytes: CANONICAL_DEPLOYMENT_BUNDLE_BYTES as u64,
    },
    SnapshotFile {
        name: DRAFT_CONFIG_NAME,
        bytes: QWEN3_DRAFT_CONFIG_BYTES,
    },
    SnapshotFile {
        name: DRAFT_TOKENIZER_METADATA_NAME,
        bytes: QWEN3_TOKENIZER_METADATA_BYTES,
    },
    SnapshotFile {
        name: DRAFT_WEIGHTS_NAME,
        bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
    },
    SnapshotFile {
        name: DRAFT_MANIFEST_NAME,
        bytes: QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES as u64,
    },
    SnapshotFile {
        name: TARGET_CONFIG_NAME,
        bytes: QWEN3_TARGET_CONFIG_BYTES,
    },
    SnapshotFile {
        name: TARGET_TOKENIZER_METADATA_NAME,
        bytes: QWEN3_TOKENIZER_METADATA_BYTES,
    },
    SnapshotFile {
        name: TARGET_WEIGHTS_NAME,
        bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
    },
    SnapshotFile {
        name: TARGET_MANIFEST_NAME,
        bytes: QWEN3_TARGET_PREPACKED_MANIFEST_BYTES as u64,
    },
    SnapshotFile {
        name: TOKENIZER_NAME,
        bytes: QWEN3_TOKENIZER_BYTES,
    },
];

struct SnapshotMetadata {
    target_config: Vec<u8>,
    target_tokenizer_metadata: Vec<u8>,
    draft_config: Vec<u8>,
    draft_tokenizer_metadata: Vec<u8>,
    tokenizer: Vec<u8>,
}
#[derive(Debug)]
struct StagingDirectory {
    path: PathBuf,
    armed: bool,
}

impl StagingDirectory {
    fn create(output: &Path) -> io::Result<Self> {
        match fs::symlink_metadata(output) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("output path already exists: {}", output.display()),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let parent = parent_directory(output);
        let name = output.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "output path has no final component",
            )
        })?;
        let mut staging_name = OsString::from(".");
        staging_name.push(name);
        staging_name.push(format!(".staging.{}", std::process::id()));
        let path = parent.join(staging_name);
        fs::create_dir(&path)?;
        Ok(Self { path, armed: true })
    }

    fn publish(mut self, output: &Path) -> io::Result<()> {
        fs::rename(&self.path, output)?;
        sync_directory(parent_directory(output))?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ferric-m1-prepack: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os();
    let executable = arguments
        .next()
        .unwrap_or_else(|| OsString::from("ferric-m1-prepack"));
    let first = arguments.next().ok_or_else(|| usage_error(&executable))?;
    if first == "--verify" {
        let snapshot = arguments.next().ok_or_else(|| usage_error(&executable))?;
        if arguments.next().is_some() {
            return Err(usage_error(&executable).into());
        }
        return verify_prepacked(Path::new(&snapshot));
    }
    let output = arguments.next().ok_or_else(|| usage_error(&executable))?;
    if arguments.next().is_some() {
        return Err(usage_error(&executable).into());
    }

    admit_and_prepack(Path::new(&first), Path::new(&output))
}

fn read_snapshot_metadata(root: &Path) -> io::Result<SnapshotMetadata> {
    Ok(SnapshotMetadata {
        target_config: read_bounded(&root.join(TARGET_CONFIG_NAME), QWEN3_TARGET_CONFIG_BYTES)?,
        target_tokenizer_metadata: read_bounded(
            &root.join(TARGET_TOKENIZER_METADATA_NAME),
            QWEN3_TOKENIZER_METADATA_BYTES,
        )?,
        draft_config: read_bounded(&root.join(DRAFT_CONFIG_NAME), QWEN3_DRAFT_CONFIG_BYTES)?,
        draft_tokenizer_metadata: read_bounded(
            &root.join(DRAFT_TOKENIZER_METADATA_NAME),
            QWEN3_TOKENIZER_METADATA_BYTES,
        )?,
        tokenizer: read_bounded(&root.join(TOKENIZER_NAME), QWEN3_TOKENIZER_BYTES)?,
    })
}

fn authenticate_snapshot_metadata(
    metadata: &SnapshotMetadata,
    limits: EngineLimits,
) -> Result<(AuthenticatedTokenizer, AuthenticatedTokenizer), Box<dyn Error>> {
    let vocabulary = ArtifactDigest {
        sha256: QWEN3_TOKENIZER_SHA256,
        byte_len: QWEN3_TOKENIZER_BYTES,
    };
    build_preliminary_deployment_bundle(DeploymentAssets {
        target: ModelAssets {
            repository: TARGET_REPOSITORY,
            revision: TARGET_REVISION,
            config_json: &metadata.target_config,
            tokenizer_metadata_json: &metadata.target_tokenizer_metadata,
            vocabulary,
            weights: WeightDescriptor {
                weights_id: QWEN3_TARGET_WEIGHT_SET_SHA256,
                artifact_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
                tensor_data_bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
                sections: 5,
            },
        },
        draft: ModelAssets {
            repository: DRAFT_REPOSITORY,
            revision: DRAFT_REVISION,
            config_json: &metadata.draft_config,
            tokenizer_metadata_json: &metadata.draft_tokenizer_metadata,
            vocabulary,
            weights: WeightDescriptor {
                weights_id: QWEN3_DRAFT_WEIGHT_SHA256,
                artifact_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
                tensor_data_bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
                sections: 1,
            },
        },
        limits,
    })?;
    let target = authenticate_qwen3_tokenizer(
        Qwen3ModelRole::Target8B,
        Cursor::new(metadata.tokenizer.as_slice()),
    )?;
    let draft = authenticate_qwen3_tokenizer(
        Qwen3ModelRole::Draft06B,
        Cursor::new(metadata.tokenizer.as_slice()),
    )?;
    Ok((target, draft))
}

fn write_snapshot_metadata(root: &Path, metadata: &SnapshotMetadata) -> io::Result<()> {
    for (name, bytes) in [
        (TARGET_CONFIG_NAME, metadata.target_config.as_slice()),
        (
            TARGET_TOKENIZER_METADATA_NAME,
            metadata.target_tokenizer_metadata.as_slice(),
        ),
        (DRAFT_CONFIG_NAME, metadata.draft_config.as_slice()),
        (
            DRAFT_TOKENIZER_METADATA_NAME,
            metadata.draft_tokenizer_metadata.as_slice(),
        ),
        (TOKENIZER_NAME, metadata.tokenizer.as_slice()),
    ] {
        write_new_file(&root.join(name), bytes)?;
    }
    Ok(())
}

fn admit_and_prepack(source: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    validate_source_output_separation(source, output)?;
    let CanonicalQwen3SourceBundle {
        target_config,
        target_tokenizer_metadata,
        draft_config,
        draft_tokenizer_metadata,
        tokenizer,
        target_index,
        target_shards,
        draft_weights,
    } = open_canonical_qwen3_source_bundle(source)?;
    let metadata = SnapshotMetadata {
        target_config,
        target_tokenizer_metadata,
        draft_config,
        draft_tokenizer_metadata,
        tokenizer,
    };

    let limits = EngineLimits {
        max_context_tokens: 8_192,
        max_active_sequences: 32,
        kv_page_tokens: 256,
        max_draft_tokens: 16,
    };
    let (target_tokenizer, draft_tokenizer) = authenticate_snapshot_metadata(&metadata, limits)?;

    let staging = StagingDirectory::create(output)?;
    let mut target_output = create_new_file(&staging.path.join(TARGET_WEIGHTS_NAME))?;
    let target_weights =
        prepack_qwen3_target_weights(&target_index, target_shards, &mut target_output)?;
    target_output.sync_all()?;

    let mut draft_output = create_new_file(&staging.path.join(DRAFT_WEIGHTS_NAME))?;
    let draft_weights = prepack_qwen3_draft_weights(draft_weights, &mut draft_output)?;
    draft_output.sync_all()?;

    let prepacked = build_prepacked_deployment_bundle(
        AuthenticatedDeploymentAssets {
            target: AuthenticatedModelAssets {
                repository: TARGET_REPOSITORY,
                revision: TARGET_REVISION,
                config_json: &metadata.target_config,
                tokenizer_metadata_json: &metadata.target_tokenizer_metadata,
            },
            draft: AuthenticatedModelAssets {
                repository: DRAFT_REPOSITORY,
                revision: DRAFT_REVISION,
                config_json: &metadata.draft_config,
                tokenizer_metadata_json: &metadata.draft_tokenizer_metadata,
            },
            limits,
        },
        target_tokenizer,
        draft_tokenizer,
        target_weights,
        draft_weights,
    )?;
    let bundle_bytes = encode_canonical_deployment_bundle(prepacked.deployment())?;
    write_snapshot_metadata(&staging.path, &metadata)?;
    write_new_file(
        &staging.path.join(TARGET_MANIFEST_NAME),
        prepacked.target_manifest().canonical_bytes(),
    )?;
    write_new_file(
        &staging.path.join(DRAFT_MANIFEST_NAME),
        prepacked.draft_manifest().canonical_bytes(),
    )?;
    write_new_file(
        &staging.path.join(DEPLOYMENT_BUNDLE_NAME),
        bundle_bytes.as_bytes(),
    )?;

    let admission = seal_authenticated_bundle(prepacked)?;
    let record_id = admission.record().record_id();
    let bundle_id = admission.prepacked().deployment().bundle_id;
    let target_manifest_id = admission.prepacked().target_manifest().aggregate_id();
    let draft_manifest_id = admission.prepacked().draft_manifest().aggregate_id();
    write_new_file(
        &staging.path.join(BUNDLE_ADMISSION_NAME),
        admission.record().as_bytes(),
    )?;
    validate_snapshot_roster(&staging.path)?;
    sync_directory(&staging.path)?;
    staging.publish(output)?;

    println!("output={}", output.display());
    println!("bundle_id={}", hex(bundle_id.as_bytes()));
    println!("target_manifest_id={}", hex(&target_manifest_id));
    println!("draft_manifest_id={}", hex(&draft_manifest_id));
    println!("admission_record_id={}", hex(record_id.as_bytes()));
    Ok(())
}

fn verify_prepacked(prepacked_root: &Path) -> Result<(), Box<dyn Error>> {
    validate_snapshot_roster(prepacked_root)?;
    let metadata = read_snapshot_metadata(prepacked_root)?;
    let record_bytes = read_bounded(
        &prepacked_root.join(BUNDLE_ADMISSION_NAME),
        BUNDLE_ADMISSION_RECORD_BYTES as u64,
    )?;
    let descriptor = decode_bundle_admission_record(&record_bytes)?;
    let target_manifest = read_bounded(
        &prepacked_root.join(TARGET_MANIFEST_NAME),
        u64::from(QWEN3_TARGET_PREPACKED_MANIFEST_BYTES),
    )?;
    let draft_manifest = read_bounded(
        &prepacked_root.join(DRAFT_MANIFEST_NAME),
        u64::from(QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES),
    )?;
    let target_weights = reopen_persisted_qwen3_weights(
        Qwen3ModelRole::Target8B,
        descriptor.target_manifest,
        &target_manifest,
        File::open(prepacked_root.join(TARGET_WEIGHTS_NAME))?,
    )?;
    let draft_weights = reopen_persisted_qwen3_weights(
        Qwen3ModelRole::Draft06B,
        descriptor.draft_manifest,
        &draft_manifest,
        File::open(prepacked_root.join(DRAFT_WEIGHTS_NAME))?,
    )?;
    let limits = EngineLimits {
        max_context_tokens: 8_192,
        max_active_sequences: 32,
        kv_page_tokens: 256,
        max_draft_tokens: 16,
    };
    let (target_tokenizer, draft_tokenizer) = authenticate_snapshot_metadata(&metadata, limits)?;
    let prepacked = build_prepacked_deployment_bundle(
        AuthenticatedDeploymentAssets {
            target: AuthenticatedModelAssets {
                repository: TARGET_REPOSITORY,
                revision: TARGET_REVISION,
                config_json: &metadata.target_config,
                tokenizer_metadata_json: &metadata.target_tokenizer_metadata,
            },
            draft: AuthenticatedModelAssets {
                repository: DRAFT_REPOSITORY,
                revision: DRAFT_REVISION,
                config_json: &metadata.draft_config,
                tokenizer_metadata_json: &metadata.draft_tokenizer_metadata,
            },
            limits,
        },
        target_tokenizer,
        draft_tokenizer,
        target_weights,
        draft_weights,
    )?;
    if prepacked.deployment() != &descriptor.deployment {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "persisted deployment differs from the admission record",
        )
        .into());
    }
    let bundle = encode_canonical_deployment_bundle(prepacked.deployment())?;
    let persisted_bundle = read_bounded(
        &prepacked_root.join(DEPLOYMENT_BUNDLE_NAME),
        bundle.as_bytes().len() as u64,
    )?;
    if bundle.as_bytes().as_slice() != persisted_bundle {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "persisted canonical deployment bytes differ",
        )
        .into());
    }
    let admission = seal_authenticated_bundle(prepacked)?;
    if admission.record().as_bytes().as_slice() != record_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "persisted admission record does not re-seal exactly",
        )
        .into());
    }

    println!("verified_snapshot={}", prepacked_root.display());
    println!(
        "bundle_id={}",
        hex(admission.prepacked().deployment().bundle_id.as_bytes())
    );
    println!(
        "target_manifest_id={}",
        hex(&admission.prepacked().target_manifest().aggregate_id())
    );
    println!(
        "draft_manifest_id={}",
        hex(&admission.prepacked().draft_manifest().aggregate_id())
    );
    println!(
        "admission_record_id={}",
        hex(admission.record().record_id().as_bytes())
    );
    Ok(())
}

fn validate_snapshot_roster(root: &Path) -> io::Result<()> {
    let root_metadata = fs::symlink_metadata(root)?;
    if !root_metadata.file_type().is_dir() {
        return Err(invalid_snapshot("snapshot root is not a directory"));
    }

    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(expected) = SNAPSHOT_FILES
            .iter()
            .find(|expected| name == OsStr::new(expected.name))
        else {
            return Err(invalid_snapshot(&format!(
                "unexpected snapshot entry {}",
                name.display()
            )));
        };
        if !seen.insert(expected.name) {
            return Err(invalid_snapshot(&format!(
                "duplicate snapshot entry {:?}",
                expected.name
            )));
        }
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.file_type().is_file() {
            return Err(invalid_snapshot(&format!(
                "snapshot entry {:?} is not a regular file",
                expected.name
            )));
        }
        if metadata.len() != expected.bytes {
            return Err(invalid_snapshot(&format!(
                "snapshot entry {:?} has {} bytes, expected {}",
                expected.name,
                metadata.len(),
                expected.bytes
            )));
        }
    }
    for expected in SNAPSHOT_FILES {
        if !seen.contains(expected.name) {
            return Err(invalid_snapshot(&format!(
                "missing snapshot entry {:?}",
                expected.name
            )));
        }
    }
    Ok(())
}

fn invalid_snapshot(reason: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid canonical prepacked snapshot: {reason}"),
    )
}

fn validate_source_output_separation(source: &Path, output: &Path) -> io::Result<()> {
    let source = fs::canonicalize(source)?;
    let output_name = output.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "output path has no final component",
        )
    })?;
    let output_parent = fs::canonicalize(parent_directory(output))?;
    let output = output_parent.join(output_name);
    if output == source || output.starts_with(&source) || source.starts_with(&output) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source and canonical snapshot paths overlap",
        ));
    }
    Ok(())
}

fn read_bounded(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("input exceeds {limit} bytes: {}", path.display()),
        ));
    }
    Ok(bytes)
}

fn create_new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = create_new_file(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn usage_error(executable: &std::ffi::OsStr) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "usage: {} <download-root> <output-directory> | --verify <prepacked-directory>",
            Path::new(executable).display()
        ),
    )
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("writing into String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        authenticate_snapshot_metadata, hex, read_bounded, read_snapshot_metadata,
        validate_snapshot_roster, validate_source_output_separation, SnapshotFile,
        StagingDirectory, BUNDLE_ADMISSION_NAME, DRAFT_CONFIG_NAME, DRAFT_TOKENIZER_METADATA_NAME,
        QWEN3_DRAFT_CONFIG_BYTES, QWEN3_TARGET_CONFIG_BYTES, QWEN3_TOKENIZER_BYTES,
        QWEN3_TOKENIZER_METADATA_BYTES, SNAPSHOT_FILES, TARGET_CONFIG_NAME,
        TARGET_TOKENIZER_METADATA_NAME, TOKENIZER_NAME,
    };
    use ferric_spec::EngineLimits;
    use std::fs::{self, File};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    const TARGET_CONFIG_FIXTURE: &[u8] = include_bytes!("../fixtures/qwen3-8b-config.json");
    const DRAFT_CONFIG_FIXTURE: &[u8] = include_bytes!("../fixtures/qwen3-06b-config.json");
    const TOKENIZER_METADATA: &[u8] = include_bytes!("../fixtures/qwen3-tokenizer-config.json");
    const TOKENIZER: &[u8] = include_bytes!("../fixtures/tokenizer/qwen3-tokenizer.json");
    static TEST_ORDINAL: AtomicU64 = AtomicU64::new(0);

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ferric-m1-prepack-{label}-{}-{}",
            std::process::id(),
            TEST_ORDINAL.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn create_sparse_file(path: &Path, bytes: u64) {
        let file = File::create(path).expect("create sparse snapshot file");
        file.set_len(bytes).expect("set exact snapshot file length");
    }

    fn create_snapshot_shape(root: &Path) {
        fs::create_dir(root).expect("create snapshot root");
        for SnapshotFile { name, bytes } in SNAPSHOT_FILES {
            create_sparse_file(&root.join(name), bytes);
        }
    }

    fn pinned_config(fixture: &'static [u8], expected_bytes: u64) -> &'static [u8] {
        let expected_bytes = usize::try_from(expected_bytes).expect("fixture length fits usize");
        assert_eq!(fixture.len(), expected_bytes + 1);
        assert_eq!(fixture.last(), Some(&b'\n'));
        &fixture[..expected_bytes]
    }

    fn write_exact_metadata(root: &Path) {
        let target_config = pinned_config(TARGET_CONFIG_FIXTURE, QWEN3_TARGET_CONFIG_BYTES);
        let draft_config = pinned_config(DRAFT_CONFIG_FIXTURE, QWEN3_DRAFT_CONFIG_BYTES);
        for (name, bytes) in [
            (TARGET_CONFIG_NAME, target_config),
            (TARGET_TOKENIZER_METADATA_NAME, TOKENIZER_METADATA),
            (DRAFT_CONFIG_NAME, draft_config),
            (DRAFT_TOKENIZER_METADATA_NAME, TOKENIZER_METADATA),
            (TOKENIZER_NAME, TOKENIZER),
        ] {
            fs::write(root.join(name), bytes).expect("write exact snapshot metadata");
        }
    }

    const fn limits() -> EngineLimits {
        EngineLimits {
            max_context_tokens: 8_192,
            max_active_sequences: 32,
            kv_page_tokens: 256,
            max_draft_tokens: 16,
        }
    }

    #[test]
    fn bounded_read_rejects_trailing_input() {
        let root = test_root("bounded-read");
        fs::create_dir(&root).expect("create isolated test directory");
        let input = root.join("input");
        fs::write(&input, b"12345").expect("write test input");
        assert_eq!(read_bounded(&input, 5).expect("exact bound"), b"12345");
        assert_eq!(
            read_bounded(&input, 4)
                .expect_err("trailing byte must fail")
                .kind(),
            std::io::ErrorKind::InvalidData
        );
        fs::remove_dir_all(root).expect("remove isolated test directory");
    }

    #[test]
    fn identity_hex_is_fixed_width() {
        assert_eq!(hex(&[0, 1, 15, 16, 255]), "00010f10ff");
    }

    #[test]
    fn fixture_boundaries_match_the_exact_pinned_snapshot_lengths() {
        assert_eq!(
            pinned_config(TARGET_CONFIG_FIXTURE, QWEN3_TARGET_CONFIG_BYTES).len() as u64,
            QWEN3_TARGET_CONFIG_BYTES
        );
        assert_eq!(
            pinned_config(DRAFT_CONFIG_FIXTURE, QWEN3_DRAFT_CONFIG_BYTES).len() as u64,
            QWEN3_DRAFT_CONFIG_BYTES
        );
        assert_eq!(
            TOKENIZER_METADATA.len() as u64,
            QWEN3_TOKENIZER_METADATA_BYTES
        );
        assert_eq!(TOKENIZER.len() as u64, QWEN3_TOKENIZER_BYTES);
    }

    #[test]
    fn snapshot_metadata_authenticates_after_source_tree_is_deleted() {
        let root = test_root("self-contained");
        let source = root.join("source");
        let snapshot = root.join("snapshot");
        fs::create_dir(&root).expect("create isolated test root");
        fs::create_dir(&source).expect("create disposable source tree");
        fs::write(source.join("download.marker"), b"source-only")
            .expect("write disposable source marker");
        create_snapshot_shape(&snapshot);
        write_exact_metadata(&snapshot);

        // Full weight-image verification requires the canonical external Qwen
        // assets. This focused test establishes only that all model metadata and
        // tokenizer authority now come from the snapshot after source removal.
        fs::remove_dir_all(&source).expect("delete original source tree");
        assert!(!source.exists());
        validate_snapshot_roster(&snapshot).expect("exact snapshot roster");
        let metadata = read_snapshot_metadata(&snapshot).expect("snapshot-owned metadata");
        authenticate_snapshot_metadata(&metadata, limits())
            .expect("snapshot-owned metadata and tokenizer authenticate");

        fs::remove_dir_all(root).expect("remove isolated test root");
    }

    #[test]
    fn every_missing_or_wrong_length_snapshot_file_fails_closed() {
        let root = test_root("roster-shape");
        create_snapshot_shape(&root);
        validate_snapshot_roster(&root).expect("exact sparse roster");

        for expected in SNAPSHOT_FILES {
            let path = root.join(expected.name);
            fs::remove_file(&path).expect("remove required snapshot file");
            assert!(
                validate_snapshot_roster(&root).is_err(),
                "missing {} was accepted",
                expected.name
            );
            create_sparse_file(&path, expected.bytes);

            File::options()
                .write(true)
                .open(&path)
                .expect("open required snapshot file")
                .set_len(expected.bytes - 1)
                .expect("truncate required snapshot file");
            assert!(
                validate_snapshot_roster(&root).is_err(),
                "truncated {} was accepted",
                expected.name
            );
            File::options()
                .write(true)
                .open(&path)
                .expect("reopen required snapshot file")
                .set_len(expected.bytes + 1)
                .expect("extend required snapshot file");
            assert!(
                validate_snapshot_roster(&root).is_err(),
                "trailing byte in {} was accepted",
                expected.name
            );
            File::options()
                .write(true)
                .open(&path)
                .expect("restore required snapshot file")
                .set_len(expected.bytes)
                .expect("restore exact snapshot file length");
        }

        fs::remove_dir_all(root).expect("remove isolated test root");
    }

    #[test]
    fn canonical_snapshot_roster_is_strictly_ordered_and_unique() {
        for adjacent in SNAPSHOT_FILES.windows(2) {
            assert!(
                adjacent[0].name < adjacent[1].name,
                "snapshot roster is not strictly ordered at {:?} and {:?}",
                adjacent[0].name,
                adjacent[1].name
            );
        }
    }

    #[test]
    fn unexpected_and_nonregular_snapshot_entries_fail_closed() {
        let root = test_root("roster-types");
        create_snapshot_shape(&root);

        fs::write(root.join("unexpected"), b"extra").expect("write unexpected entry");
        assert!(validate_snapshot_roster(&root).is_err());
        fs::remove_file(root.join("unexpected")).expect("remove unexpected entry");

        let nested = root.join("unexpected-directory");
        fs::create_dir(&nested).expect("create unexpected directory");
        fs::write(nested.join("nested"), b"extra").expect("write nested unexpected entry");
        assert!(validate_snapshot_roster(&root).is_err());
        fs::remove_dir_all(&nested).expect("remove unexpected directory");

        let admission = root.join(BUNDLE_ADMISSION_NAME);
        fs::remove_file(&admission).expect("remove regular admission record");
        fs::create_dir(&admission).expect("replace admission record with directory");
        assert!(validate_snapshot_roster(&root).is_err());
        fs::remove_dir(&admission).expect("remove hostile directory");
        let expected = SNAPSHOT_FILES
            .iter()
            .find(|entry| entry.name == BUNDLE_ADMISSION_NAME)
            .expect("admission roster entry");
        create_sparse_file(&admission, expected.bytes);

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            use std::os::unix::net::UnixListener;

            fs::remove_file(&admission).expect("remove restored admission record");
            symlink(TARGET_CONFIG_NAME, &admission).expect("create hostile snapshot symlink");
            assert!(validate_snapshot_roster(&root).is_err());
            fs::remove_file(&admission).expect("remove hostile snapshot symlink");

            let socket = UnixListener::bind(&admission).expect("create hostile snapshot socket");
            assert!(validate_snapshot_roster(&root).is_err());
            drop(socket);
            fs::remove_file(&admission).expect("remove hostile snapshot socket");

            rustix::fs::mkfifoat(
                rustix::fs::CWD,
                &admission,
                rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
            )
            .expect("create hostile snapshot FIFO");
            assert!(validate_snapshot_roster(&root).is_err());
        }

        fs::remove_dir_all(root).expect("remove isolated test root");
    }

    #[test]
    fn config_metadata_and_tokenizer_mutations_fail_closed() {
        let root = test_root("metadata-hostiles");
        create_snapshot_shape(&root);
        write_exact_metadata(&root);
        validate_snapshot_roster(&root).expect("exact snapshot roster");

        for name in [
            TARGET_CONFIG_NAME,
            TARGET_TOKENIZER_METADATA_NAME,
            DRAFT_CONFIG_NAME,
            DRAFT_TOKENIZER_METADATA_NAME,
            TOKENIZER_NAME,
        ] {
            let path = root.join(name);
            let exact = fs::read(&path).expect("read exact metadata bytes");
            let mut changed = exact.clone();
            changed[0] ^= 1;
            fs::write(&path, &changed).expect("write same-length metadata mutation");
            validate_snapshot_roster(&root).expect("mutation retains exact file shape");
            let metadata = read_snapshot_metadata(&root).expect("read mutated metadata");
            assert!(
                authenticate_snapshot_metadata(&metadata, limits()).is_err(),
                "same-length mutation of {name} was accepted"
            );
            fs::write(path, exact).expect("restore exact metadata bytes");
        }

        let target = fs::read(root.join(TARGET_CONFIG_NAME)).expect("read target config");
        let draft = fs::read(root.join(DRAFT_CONFIG_NAME)).expect("read draft config");
        fs::write(root.join(TARGET_CONFIG_NAME), &draft).expect("swap target config");
        fs::write(root.join(DRAFT_CONFIG_NAME), &target).expect("swap draft config");
        assert!(validate_snapshot_roster(&root).is_err());

        fs::remove_dir_all(root).expect("remove isolated test root");
    }

    #[test]
    fn failed_directory_publication_preserves_destination_and_cleans_staging() {
        let root = test_root("publication-failure");
        fs::create_dir(&root).expect("create isolated publication root");
        let output = root.join("snapshot");
        let staging = StagingDirectory::create(&output).expect("create staging directory");
        let staging_path = staging.path.clone();
        fs::write(staging_path.join("partial"), b"partial").expect("write partial staging file");

        fs::create_dir(&output).expect("create competing destination");
        fs::write(output.join("sentinel"), b"keep").expect("write destination sentinel");
        assert!(staging.publish(&output).is_err());
        assert_eq!(
            fs::read(output.join("sentinel")).expect("read destination sentinel"),
            b"keep"
        );
        assert!(!staging_path.exists());

        fs::remove_dir_all(root).expect("remove isolated publication root");
    }

    #[test]
    fn source_snapshot_aliases_and_preexisting_outputs_fail_closed() {
        let root = test_root("path-aliases");
        let source = root.join("source");
        fs::create_dir_all(&source).expect("create source tree");

        assert!(validate_source_output_separation(&source, &source).is_err());
        assert!(validate_source_output_separation(&source, &source.join("snapshot")).is_err());
        assert!(validate_source_output_separation(&source, &root).is_err());
        let sibling = root.join("snapshot");
        validate_source_output_separation(&source, &sibling)
            .expect("disjoint sibling snapshot path");

        fs::create_dir(&sibling).expect("create preexisting snapshot output");
        fs::write(sibling.join("sentinel"), b"keep").expect("write output sentinel");
        assert_eq!(
            StagingDirectory::create(&sibling)
                .expect_err("preexisting output must fail")
                .kind(),
            std::io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read(sibling.join("sentinel")).expect("read preserved output sentinel"),
            b"keep"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let dangling = root.join("dangling-output");
            symlink("missing-target", &dangling).expect("create dangling output symlink");
            assert_eq!(
                StagingDirectory::create(&dangling)
                    .expect_err("dangling output symlink must fail")
                    .kind(),
                std::io::ErrorKind::AlreadyExists
            );
            assert_eq!(
                fs::read_link(&dangling).expect("read preserved dangling output symlink"),
                Path::new("missing-target")
            );
        }

        fs::remove_dir_all(root).expect("remove isolated alias test root");
    }
}
