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
    final_rms_requested: bool,
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
    if logits.final_rms_row_bytes().is_some() != final_rms_requested
        || logits.final_rms_row_offsets().is_some() != final_rms_requested
    {
        return Err("final RMS capture differs from the explicit request".to_owned());
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
    write("capture.json", &encoded)?;
    if let (Some([input, output]), Some(offsets)) =
        (logits.final_rms_row_bytes(), logits.final_rms_row_offsets())
    {
        let document = final_rms_document(
            &encoded,
            &target,
            logits.dispatch_generation(),
            logits.offset_bytes(),
            logits.completed_readback_extent_bytes(),
            logits.completed_readback_sha256(),
            [(input, offsets[0]), (output, offsets[1])],
        )?;
        write("target-final-rms-input.bf16", input)?;
        write("target-final-rms-output.bf16", output)?;
        let mut encoded = serde_json::to_vec(&document).map_err(|error| error.to_string())?;
        encoded.push(b'\n');
        write("final-rms-capture.json", &encoded)?;
    }
    Ok(())
}

fn final_rms_document(
    capture: &[u8],
    target: &[u32],
    generation: u64,
    base: u64,
    extent: usize,
    readback_sha256: &[u8; 32],
    rows: [(&[u8], u64); 2],
) -> SmokeResult<Value> {
    let offsets = rows.map(|(_, offset)| offset);
    let rows = rows.map(|(bytes, _)| bytes);
    let logits_bytes = 5 * u64::from(QWEN3_VOCABULARY_SIZE) * 2;
    let expected_offsets = [
        base.checked_add(logits_bytes + 3 * 8192),
        base.checked_add(logits_bytes + 5 * 8192 + 3 * 8192),
    ];
    if target.len() != 5
        || u64::try_from(extent).ok() != Some(logits_bytes + 2 * 5 * 8192)
        || expected_offsets != offsets.map(Some)
        || rows.iter().any(|row| {
            row.len() != 8192
                || row
                    .chunks_exact(2)
                    .any(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) & 0x7f80 == 0x7f80)
        })
    {
        return Err("final RMS completed-copy extent, offsets, or finite rows rejected".to_owned());
    }
    let row = |index: usize, path: &str| {
        json!({
            "path": path, "shape": [4096], "dtype": "bf16-little-endian",
            "bytes": rows[index].len(), "sha256": bytes_hex(&Sha256::digest(rows[index])),
            "offset_bytes": offsets[index],
        })
    };
    Ok(json!({
        "format": "FERRIC-ENGINEERING-S1-K4-FINAL-RMS-V1",
        "authority": "none", "qualification": false, "benchmark_comparable": false,
        "numerical_comparison_performed": false, "target_segment": 4, "active_index": 3,
        "position": 131, "input_token_id": target[3], "dispatch_generation": generation,
        "capture_sha256": bytes_hex(&Sha256::digest(capture)),
        "epsilon_f32_bits": 1e-6_f32.to_bits(), "weight_tensor": "model.norm.weight",
        "retained_payload_scope": "all-target-logits-and-final-rms-row3-only",
        "completed_readback": {"offset_bytes": base, "bytes": extent, "sha256": bytes_hex(readback_sha256)},
        "input": row(0, "target-final-rms-input.bf16"),
        "output": row(1, "target-final-rms-output.bf16"),
        "scope": "Opt-in target segment4 ResidualHidden and FinalNormalized host-visible workspace substitution; one owned completed readback before teardown, publishing row3 only.",
        "nonclaim": "Other RMS rows in the completed-copy digest are not exported. No independently replayable full-copy hash, numerical tolerance, device causality, qualification, production admission, or performance claim.",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_rms_sidecar_binds_capture_copy_and_only_row_three() {
        let input = vec![0; 8192];
        let output = vec![0; 8192];
        let offsets = [64 + 1_543_936, 64 + 1_584_896];
        let capture = b"actual-capture-fixture\n";
        let record = final_rms_document(
            capture,
            &[12, 220, 17, 13, 198],
            9,
            64,
            1_601_280,
            &[7; 32],
            [(&input, offsets[0]), (&output, offsets[1])],
        )
        .unwrap();
        assert_eq!(
            record["capture_sha256"],
            bytes_hex(&Sha256::digest(capture))
        );
        assert_eq!(record["target_segment"], 4);
        assert_eq!(record["active_index"], 3);
        assert_eq!(record["position"], 131);
        assert_eq!(record["input_token_id"], 13);
        assert_eq!(record["dispatch_generation"], 9);
        assert_eq!(record["epsilon_f32_bits"], 1e-6_f32.to_bits());
        assert_eq!(
            record["retained_payload_scope"],
            "all-target-logits-and-final-rms-row3-only"
        );
        assert_eq!(record["completed_readback"]["bytes"], 1_601_280);
        for (index, (role, path)) in [
            ("input", "target-final-rms-input.bf16"),
            ("output", "target-final-rms-output.bf16"),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(record[role]["path"], path);
            assert_eq!(record[role]["bytes"], 8192);
            assert_eq!(record[role]["shape"], json!([4096]));
            assert_eq!(record[role]["offset_bytes"], offsets[index]);
        }
        for name in [
            "qualification",
            "benchmark_comparable",
            "numerical_comparison_performed",
        ] {
            assert_eq!(record[name], false);
        }
        let actual = record
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = [
            "format",
            "authority",
            "qualification",
            "benchmark_comparable",
            "numerical_comparison_performed",
            "target_segment",
            "active_index",
            "position",
            "input_token_id",
            "dispatch_generation",
            "capture_sha256",
            "epsilon_f32_bits",
            "weight_tensor",
            "retained_payload_scope",
            "completed_readback",
            "input",
            "output",
            "scope",
            "nonclaim",
        ]
        .into_iter()
        .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn final_rms_sidecar_rejects_bad_extent_offsets_and_nonfinite_payloads() {
        let input = vec![0; 8192];
        let output = vec![0; 8192];
        let record = |base, extent, rows| {
            final_rms_document(
                b"capture",
                &[12, 220, 17, 13, 198],
                9,
                base,
                extent,
                &[7; 32],
                rows,
            )
        };
        assert!(record(0, 1_519_360, [(&input, 1_543_936), (&output, 1_584_896)]).is_err());
        assert!(record(0, 1_601_280, [(&input, 1_543_938), (&output, 1_584_896)]).is_err());
        assert!(
            record(
                0,
                1_601_280,
                [(&input[..8190], 1_543_936), (&output, 1_584_896)]
            )
            .is_err()
        );
        assert!(
            record(
                u64::MAX,
                1_601_280,
                [(&input, 1_543_936), (&output, 1_584_896)]
            )
            .is_err()
        );
        let mut nonfinite = input.clone();
        nonfinite[0..2].copy_from_slice(&0x7fc0_u16.to_le_bytes());
        assert!(
            record(
                0,
                1_601_280,
                [(&nonfinite, 1_543_936), (&output, 1_584_896)]
            )
            .is_err()
        );
        assert!(record(0, 1_601_280, [(&input, 1_543_936), (&nonfinite, 1_584_896)]).is_err());
    }

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
