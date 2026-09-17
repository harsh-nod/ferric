//! Emit reference-binding identities after the existing CPU model admission.
//! This example never opens a GPU device or creates a worker.

use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::io::{Read, Write as _};
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}

fn checkpoint_file(path: &Path) -> Result<serde_json::Value, String> {
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 16384];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        total += read as u64;
    }
    Ok(json!({"bytes": total, "sha256": hex(&hash.finalize())}))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("usage: draft_reference_identity CANONICAL_PAIRED_SOURCE".into());
    }
    let source = Path::new(&args[0]);
    let model = EngineeringQwenModelV1::open_with_draft(source)?;
    let draft = model
        .draft()
        .ok_or("authenticated draft was not retained")?;
    let mut checkpoint = serde_json::Map::new();
    for name in [
        "config.json",
        "model.safetensors",
        "tokenizer.json",
        "tokenizer_config.json",
    ] {
        checkpoint.insert(
            name.into(),
            checkpoint_file(&source.join("draft").join(name))?,
        );
    }
    let result = json!({
        "schema": "FerricDraftReferenceIdentityV1",
        "checkpoint": checkpoint,
        "identity": {
            "model_bundle_id": hex(model.bundle_id().as_bytes()),
            "draft_model_id": hex(draft.config().model_id.as_bytes()),
            "draft_config_id": hex(draft.config().config_id.as_bytes()),
            "draft_weights_sha256": hex(&Sha256::digest(draft.weights())),
        },
    });
    let mut output = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, &result).map_err(|error| error.to_string())?;
    writeln!(output).map_err(|error| error.to_string())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("model identity admission failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
