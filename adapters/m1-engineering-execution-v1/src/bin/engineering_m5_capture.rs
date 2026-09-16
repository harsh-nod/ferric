//! Separate engineering M=5 transcript; never an R29 qualification bundle.

use ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1;
use ferric_spec::QWEN3_VOCABULARY_SIZE;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use super::{SmokeResult, bytes_hex};

const FORMAT: &str = "FERRIC-ENGINEERING-S1-K4-ALL-LOGITS-V1";

fn tokens(value: &Value, count: usize, label: &str) -> SmokeResult<Vec<u32>> {
    let values = value
        .as_array()
        .filter(|values| values.len() == count)
        .ok_or_else(|| format!("{label} must contain exactly {count} tokens"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|token| u32::try_from(token).ok())
                .filter(|token| *token < QWEN3_VOCABULARY_SIZE)
                .ok_or_else(|| format!("{label} contains an invalid token"))
        })
        .collect()
}

fn sequence(report: &Value) -> SmokeResult<(Vec<u32>, Vec<u32>)> {
    let prefix = tokens(
        &report["prompt"]["physical_token_ids"],
        128,
        "physical prefix",
    )?;
    let anchor = tokens(
        &json!([report["paired_prefill"]["first_token_id"]]),
        1,
        "anchor",
    )?[0];
    let mut target = vec![anchor];
    target.extend(tokens(
        &report["speculative_k4"]["draft_choices"],
        4,
        "actual draft proposals",
    )?);
    Ok((prefix, target))
}

pub(super) fn publish(
    directory: &Path,
    report: &Value,
    choices: &M1ObservedSpeculativeDiagnosticChoicesV1,
) -> SmokeResult<()> {
    let logits = choices
        .engineering_s1_k4_logits()
        .ok_or_else(|| "requested engineering M=5 logits were not captured".to_owned())?;
    if logits.dispatch_generation() != choices.dispatch_generation()
        || logits.choices() != choices.target_choices()
        || logits.raw_bytes().len() != 5 * QWEN3_VOCABULARY_SIZE as usize * 2
    {
        return Err(
            "engineering logits generation, shape, or argmax differs from copied target choices"
                .to_owned(),
        );
    }
    let (prefix, target) = sequence(report)?;
    if tokens(
        &report["speculative_k4"]["target_choices"],
        5,
        "reported target choices",
    )? != choices.target_choices()
    {
        return Err("engineering report differs from actual target choices".to_owned());
    }
    let raw_hash = bytes_hex(logits.raw_sha256());
    let rows = (0..5).map(|index| {
        let row = logits.row_bytes(index).expect("validated five-row geometry");
        json!({
            "active_index": index,
            "position": 128 + index,
            "bytes": row.len(),
            "offset_bytes": logits.offset_bytes() + u64::try_from(index).unwrap() * logits.shape().row_bytes(),
            "sha256": bytes_hex(&Sha256::digest(row)),
            "argmax_token": logits.choices()[index],
        })
    }).collect::<Vec<_>>();
    let document = json!({
        "format": FORMAT,
        "authority": "none",
        "qualification": false,
        "benchmark_comparable": false,
        "numerical_comparison_performed": false,
        "scope": "single-sequence-k4-five-target-positions-with-real-paired-prefill",
        "reference_sequence_semantics": "full-128-token-prefix-plus-anchor-plus-four-actual-draft-proposals",
        "prefix_token_ids": prefix,
        "target_token_ids": target,
        "positions": [128, 129, 130, 131, 132],
        "shape": [1, 5, QWEN3_VOCABULARY_SIZE],
        "dtype": "bf16-little-endian",
        "dispatch_generation": logits.dispatch_generation(),
        "logits": {"path": "target-logits.bf16", "bytes": logits.raw_bytes().len(), "sha256": raw_hash},
        "rows": rows,
        "smoke": report,
        "nonclaim": "Engineering differential input only. Not protected R29, general K8/K16 coverage, qualification, production admission, or a performance result.",
    });
    let parent = directory
        .parent()
        .ok_or_else(|| "capture output must have a parent".to_owned())?;
    if !directory.is_absolute()
        || parent.canonicalize().map_err(|error| error.to_string())? != parent
    {
        return Err("capture output requires an absolute canonical parent".to_owned());
    }
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(directory)
        .map_err(|error| format!("cannot create fresh M=5 capture directory: {error}"))?;
    let write = |name: &str, bytes: &[u8]| -> SmokeResult<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(directory.join(name))
            .map_err(|error| format!("cannot create {name}: {error}"))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("cannot publish {name}: {error}"))
    };
    write("target-logits.bf16", logits.raw_bytes())?;
    let mut encoded = serde_json::to_vec(&document).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    write("capture.json", &encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m5_reference_sequence_uses_real_prefix_anchor_and_proposals() {
        let mut prefix = vec![7; 128];
        prefix[127] = 19;
        let report = json!({
            "prompt": {"physical_token_ids": prefix},
            "paired_prefill": {"first_token_id": 31},
            "speculative_k4": {"draft_choices": [41, 43, 47, 53]},
        });
        let (actual_prefix, target) = sequence(&report).unwrap();
        assert_eq!(actual_prefix, prefix);
        assert_eq!(target, [31, 41, 43, 47, 53]);
        let mut invalid = report.clone();
        invalid["prompt"]["physical_token_ids"] = json!([7]);
        assert!(sequence(&invalid).is_err());
        let mut invalid = report.clone();
        invalid["speculative_k4"]["draft_choices"][3] = json!(QWEN3_VOCABULARY_SIZE);
        assert!(sequence(&invalid).is_err());
        let mut invalid = report;
        invalid["paired_prefill"]["first_token_id"] = json!(-1);
        assert!(sequence(&invalid).is_err());
    }
}
