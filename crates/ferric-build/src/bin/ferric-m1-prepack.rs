#![forbid(unsafe_code)]

use ferric_build::{
    authenticate_qwen3_tokenizer, build_preliminary_deployment_bundle,
    build_prepacked_deployment_bundle, decode_bundle_admission_record,
    encode_canonical_deployment_bundle, prepack_qwen3_draft_weights, prepack_qwen3_target_weights,
    reopen_persisted_qwen3_weights, seal_authenticated_bundle, ArtifactDigest,
    AuthenticatedDeploymentAssets, AuthenticatedModelAssets, DeploymentAssets, ModelAssets,
    SafetensorsSource, WeightDescriptor, BUNDLE_ADMISSION_RECORD_BYTES, DRAFT_REPOSITORY,
    DRAFT_REVISION, QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
    QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
    QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_BYTES, QWEN3_TOKENIZER_SHA256,
    TARGET_REPOSITORY, TARGET_REVISION,
};
use ferric_spec::{EngineLimits, Qwen3ModelRole};
use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const METADATA_INPUT_LIMIT: u64 = 64 * 1_024;
const TARGET_SHARD_NAMES: [&str; 5] = [
    "model-00001-of-00005.safetensors",
    "model-00002-of-00005.safetensors",
    "model-00003-of-00005.safetensors",
    "model-00004-of-00005.safetensors",
    "model-00005-of-00005.safetensors",
];

struct StagingDirectory {
    path: PathBuf,
    armed: bool,
}

impl StagingDirectory {
    fn create(output: &Path) -> io::Result<Self> {
        if output.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("output path already exists: {}", output.display()),
            ));
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
        let source = arguments.next().ok_or_else(|| usage_error(&executable))?;
        let prepacked = arguments.next().ok_or_else(|| usage_error(&executable))?;
        if arguments.next().is_some() {
            return Err(usage_error(&executable).into());
        }
        return verify_prepacked(Path::new(&source), Path::new(&prepacked));
    }
    let output = arguments.next().ok_or_else(|| usage_error(&executable))?;
    if arguments.next().is_some() {
        return Err(usage_error(&executable).into());
    }

    admit_and_prepack(Path::new(&first), Path::new(&output))
}

fn read_model_metadata(root: &Path) -> io::Result<(Vec<u8>, Vec<u8>)> {
    Ok((
        read_bounded(&root.join("config.json"), METADATA_INPUT_LIMIT)?,
        read_bounded(&root.join("tokenizer_config.json"), METADATA_INPUT_LIMIT)?,
    ))
}

fn admit_and_prepack(source: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let target_root = source.join("target");
    let draft_root = source.join("draft");
    let (target_config, target_tokenizer_metadata) = read_model_metadata(&target_root)?;
    let (draft_config, draft_tokenizer_metadata) = read_model_metadata(&draft_root)?;
    let target_index = read_bounded(
        &target_root.join("model.safetensors.index.json"),
        METADATA_INPUT_LIMIT,
    )?;

    let limits = EngineLimits {
        max_context_tokens: 8_192,
        max_active_sequences: 32,
        kv_page_tokens: 256,
        max_draft_tokens: 16,
    };
    let vocabulary = ArtifactDigest {
        sha256: QWEN3_TOKENIZER_SHA256,
        byte_len: QWEN3_TOKENIZER_BYTES,
    };
    build_preliminary_deployment_bundle(DeploymentAssets {
        target: ModelAssets {
            repository: TARGET_REPOSITORY,
            revision: TARGET_REVISION,
            config_json: &target_config,
            tokenizer_metadata_json: &target_tokenizer_metadata,
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
            config_json: &draft_config,
            tokenizer_metadata_json: &draft_tokenizer_metadata,
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

    let target_tokenizer = authenticate_qwen3_tokenizer(
        Qwen3ModelRole::Target8B,
        File::open(target_root.join("tokenizer.json"))?,
    )?;
    let draft_tokenizer = authenticate_qwen3_tokenizer(
        Qwen3ModelRole::Draft06B,
        File::open(draft_root.join("tokenizer.json"))?,
    )?;

    let staging = StagingDirectory::create(output)?;
    let mut target_output = create_new_file(&staging.path.join("target.weights.bin"))?;
    let target_weights = prepack_qwen3_target_weights(
        &target_index,
        open_target_shards(&target_root)?,
        &mut target_output,
    )?;
    target_output.sync_all()?;

    let mut draft_output = create_new_file(&staging.path.join("draft.weights.bin"))?;
    let draft_weights = prepack_qwen3_draft_weights(
        SafetensorsSource {
            name: "model.safetensors",
            reader: File::open(draft_root.join("model.safetensors"))?,
        },
        &mut draft_output,
    )?;
    draft_output.sync_all()?;

    let prepacked = build_prepacked_deployment_bundle(
        AuthenticatedDeploymentAssets {
            target: AuthenticatedModelAssets {
                repository: TARGET_REPOSITORY,
                revision: TARGET_REVISION,
                config_json: &target_config,
                tokenizer_metadata_json: &target_tokenizer_metadata,
            },
            draft: AuthenticatedModelAssets {
                repository: DRAFT_REPOSITORY,
                revision: DRAFT_REVISION,
                config_json: &draft_config,
                tokenizer_metadata_json: &draft_tokenizer_metadata,
            },
            limits,
        },
        target_tokenizer,
        draft_tokenizer,
        target_weights,
        draft_weights,
    )?;
    let bundle_bytes = encode_canonical_deployment_bundle(prepacked.deployment())?;
    write_new_file(
        &staging.path.join("target.weights.manifest.bin"),
        prepacked.target_manifest().canonical_bytes(),
    )?;
    write_new_file(
        &staging.path.join("draft.weights.manifest.bin"),
        prepacked.draft_manifest().canonical_bytes(),
    )?;
    write_new_file(
        &staging.path.join("deployment.bundle.bin"),
        bundle_bytes.as_bytes(),
    )?;

    let admission = seal_authenticated_bundle(prepacked)?;
    let record_id = admission.record().record_id();
    let bundle_id = admission.prepacked().deployment().bundle_id;
    let target_manifest_id = admission.prepacked().target_manifest().aggregate_id();
    let draft_manifest_id = admission.prepacked().draft_manifest().aggregate_id();
    write_new_file(
        &staging.path.join("bundle.admission.bin"),
        admission.record().as_bytes(),
    )?;
    sync_directory(&staging.path)?;
    staging.publish(output)?;

    println!("output={}", output.display());
    println!("bundle_id={}", hex(bundle_id.as_bytes()));
    println!("target_manifest_id={}", hex(&target_manifest_id));
    println!("draft_manifest_id={}", hex(&draft_manifest_id));
    println!("admission_record_id={}", hex(record_id.as_bytes()));
    Ok(())
}

fn verify_prepacked(source: &Path, prepacked_root: &Path) -> Result<(), Box<dyn Error>> {
    let target_root = source.join("target");
    let draft_root = source.join("draft");
    let (target_config, target_tokenizer_metadata) = read_model_metadata(&target_root)?;
    let (draft_config, draft_tokenizer_metadata) = read_model_metadata(&draft_root)?;
    let record_bytes = read_bounded(
        &prepacked_root.join("bundle.admission.bin"),
        BUNDLE_ADMISSION_RECORD_BYTES as u64,
    )?;
    let descriptor = decode_bundle_admission_record(&record_bytes)?;
    let target_manifest = read_bounded(
        &prepacked_root.join("target.weights.manifest.bin"),
        METADATA_INPUT_LIMIT * 4,
    )?;
    let draft_manifest = read_bounded(
        &prepacked_root.join("draft.weights.manifest.bin"),
        METADATA_INPUT_LIMIT * 4,
    )?;
    let target_weights = reopen_persisted_qwen3_weights(
        Qwen3ModelRole::Target8B,
        descriptor.target_manifest,
        &target_manifest,
        File::open(prepacked_root.join("target.weights.bin"))?,
    )?;
    let draft_weights = reopen_persisted_qwen3_weights(
        Qwen3ModelRole::Draft06B,
        descriptor.draft_manifest,
        &draft_manifest,
        File::open(prepacked_root.join("draft.weights.bin"))?,
    )?;
    let target_tokenizer = authenticate_qwen3_tokenizer(
        Qwen3ModelRole::Target8B,
        File::open(target_root.join("tokenizer.json"))?,
    )?;
    let draft_tokenizer = authenticate_qwen3_tokenizer(
        Qwen3ModelRole::Draft06B,
        File::open(draft_root.join("tokenizer.json"))?,
    )?;
    let limits = EngineLimits {
        max_context_tokens: 8_192,
        max_active_sequences: 32,
        kv_page_tokens: 256,
        max_draft_tokens: 16,
    };
    let prepacked = build_prepacked_deployment_bundle(
        AuthenticatedDeploymentAssets {
            target: AuthenticatedModelAssets {
                repository: TARGET_REPOSITORY,
                revision: TARGET_REVISION,
                config_json: &target_config,
                tokenizer_metadata_json: &target_tokenizer_metadata,
            },
            draft: AuthenticatedModelAssets {
                repository: DRAFT_REPOSITORY,
                revision: DRAFT_REVISION,
                config_json: &draft_config,
                tokenizer_metadata_json: &draft_tokenizer_metadata,
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
        &prepacked_root.join("deployment.bundle.bin"),
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

fn open_target_shards(
    root: &Path,
) -> io::Result<[SafetensorsSource<'static, File>; TARGET_SHARD_NAMES.len()]> {
    Ok([
        SafetensorsSource {
            name: TARGET_SHARD_NAMES[0],
            reader: File::open(root.join(TARGET_SHARD_NAMES[0]))?,
        },
        SafetensorsSource {
            name: TARGET_SHARD_NAMES[1],
            reader: File::open(root.join(TARGET_SHARD_NAMES[1]))?,
        },
        SafetensorsSource {
            name: TARGET_SHARD_NAMES[2],
            reader: File::open(root.join(TARGET_SHARD_NAMES[2]))?,
        },
        SafetensorsSource {
            name: TARGET_SHARD_NAMES[3],
            reader: File::open(root.join(TARGET_SHARD_NAMES[3]))?,
        },
        SafetensorsSource {
            name: TARGET_SHARD_NAMES[4],
            reader: File::open(root.join(TARGET_SHARD_NAMES[4]))?,
        },
    ])
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
            "usage: {} <download-root> <output-directory> | --verify <download-root> <prepacked-directory>",
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
    use super::{hex, read_bounded};
    use std::fs;

    #[test]
    fn bounded_read_rejects_trailing_input() {
        let root = std::env::temp_dir().join(format!(
            "ferric-m1-prepack-test-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
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
}
