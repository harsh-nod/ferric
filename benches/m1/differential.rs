#![forbid(unsafe_code)]

//! Target-only differential run-plan, comparison, and record-ingestion boundary.

use ferric_m1_benchmarks::{
    create_new_output, encode_canonical_document, load_benchmark_plan, load_canonical_document,
    main_for, sha256_identity, BenchResult, Metric, Suite,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

const PAIRS_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-PAIRS-V1";
const OUTPUT_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-OUTPUT-V1";
const RAW_RECORD_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-RAW-RECORD-V1";
const RECORDS_FORMAT: &str = "FERRIC-M1-BENCHMARK-RECORDS-V1";
const PAIRS_AUTHORITY: &str = "externally-collected-differential-pairs-only";
const OUTPUT_AUTHORITY: &str = "externally-collected-model-output-only";
const RAW_AUTHORITY: &str = "computed-differential-comparison-only";
const VOCABULARY_SIZE: u64 = 151_936;
const BF16_BYTES: u64 = 2;
const TOKEN_BYTES: u64 = 4;

const METRICS: &[Metric] = &[
    Metric {
        id: "compared-logits",
        zero_allowed: false,
    },
    Metric {
        id: "compared-tokens",
        zero_allowed: false,
    },
    Metric {
        id: "maximum-logit-ulp-error",
        zero_allowed: true,
    },
    Metric {
        id: "token-mismatches",
        zero_allowed: true,
    },
];

const SUITE: Suite = Suite {
    name: "differential",
    obligation_id: "m1.r29",
    path_id: "differential-bench",
    source_path: "benches/m1/differential.rs",
    case_kinds: &[
        "decode-s1-c8192",
        "decode-s32-c8192",
        "decode-s8-c8192",
        "prefill-s1-t128",
        "prefill-s1-t2048",
        "prefill-s1-t512",
        "prefill-s8-t128",
    ],
    extra_identities: &["reference-implementation", "reference-protocol"],
    metrics: METRICS,
    extra_record_attributes: &["ferric-output-sha256", "reference-output-sha256"],
    minimum_warmups: 0,
    minimum_recorded_samples: 1,
    nonclaim: "Structural acceptance authenticates externally collected target-only differential records only. It does not validate a logit tolerance, prove token equality, establish numerical or hardware correctness, qualify performance, or close m1.r29.",
};

#[derive(Debug)]
struct Payload {
    bytes: u64,
    path: PathBuf,
    sha256: String,
}

#[derive(Debug)]
struct Output {
    manifest_sha256: String,
    logits: Payload,
    tokens: Payload,
}

#[derive(Debug)]
struct Pair {
    case_id: String,
    kind: String,
    ferric: Output,
    reference: Output,
    runner_transcript_sha256: String,
}

#[derive(Debug)]
struct PlanCase {
    input_sha256: String,
    kind: String,
    workload_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Comparison {
    compared_logits: u64,
    compared_tokens: u64,
    maximum_logit_ulp_error: u64,
    token_mismatches: u64,
}

struct CheckedReader<R> {
    inner: R,
    sha256: Sha256,
    bytes: u64,
}

impl<R: Read> CheckedReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            sha256: Sha256::new(),
            bytes: 0,
        }
    }

    fn read_exact(&mut self, buffer: &mut [u8], description: &str) -> BenchResult<()> {
        self.inner
            .read_exact(buffer)
            .map_err(|error| format!("cannot read {description}: {error}"))?;
        self.sha256.update(&*buffer);
        self.bytes = self
            .bytes
            .checked_add(
                u64::try_from(buffer.len())
                    .map_err(|_| format!("{description} read size does not fit u64"))?,
            )
            .ok_or_else(|| format!("{description} read length overflowed"))?;
        Ok(())
    }

    fn finish(mut self, payload: &Payload, description: &str) -> BenchResult<()> {
        let mut trailing = [0_u8; 1];
        let trailing_bytes = self
            .inner
            .read(&mut trailing)
            .map_err(|error| format!("cannot finish {description}: {error}"))?;
        if trailing_bytes != 0 || self.bytes != payload.bytes {
            return Err(format!("{description} length drifted"));
        }
        let actual = hex_digest(self.sha256.finalize().as_slice());
        if actual != payload.sha256 {
            return Err(format!("{description} SHA-256 drifted"));
        }
        Ok(())
    }
}

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|command| command == "produce")
    {
        return match produce_command(&arguments) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("FAIL: {error}");
                ExitCode::FAILURE
            }
        };
    }
    main_for(&SUITE)
}

fn produce_command(arguments: &[OsString]) -> BenchResult<()> {
    let [command, plan_path, pairs_path, raw_directory, records_path] = arguments else {
        return Err(
            "usage: ferric-m1-differential produce PLAN PAIRS OUTPUT-RAW-DIR OUTPUT-RECORDS"
                .to_owned(),
        );
    };
    if command != "produce" {
        return Err("differential producer command drifted".to_owned());
    }
    let plan_path = Path::new(plan_path);
    let pairs_path = Path::new(pairs_path);
    let raw_directory = Path::new(raw_directory);
    let records_path = Path::new(records_path);
    require_absent(raw_directory, "raw-record output directory")?;
    require_absent(records_path, "benchmark-record output")?;

    let (plan, plan_bytes) = load_benchmark_plan(&SUITE, plan_path)?;
    let plan_sha256 = sha256_identity(&plan_bytes);
    let plan_cases = exact_plan_cases(&plan)?;
    let identities = plan_identities(&plan)?;
    let (pairs_value, pairs_bytes) =
        load_canonical_document(pairs_path, "differential pairs manifest")?;
    let pairs = parse_pairs(
        &pairs_value,
        pairs_path.parent().unwrap_or_else(|| Path::new(".")),
        &plan_cases,
        &identities,
        &plan_sha256,
    )?;
    let pairs_sha256 = sha256_identity(&pairs_bytes);

    let mut completed = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let comparison = compare_pair(&pair)?;
        let raw = raw_record(&pair, comparison, &plan_sha256, &pairs_sha256)?;
        let raw_bytes = encode_canonical_document(&raw)?;
        completed.push((pair, comparison, raw_bytes));
    }

    fs::create_dir(raw_directory)
        .map_err(|error| format!("cannot create raw-record output directory: {error}"))?;
    let mut observations = Vec::with_capacity(completed.len());
    for (pair, comparison, raw_bytes) in completed {
        let raw_path = raw_directory.join(format!("{}.differential.raw.json", pair.case_id));
        create_new_output(&raw_path, &raw_bytes)?;
        observations.push(observation(&pair, comparison, &raw_bytes));
    }
    let records = json!({
        "format": RECORDS_FORMAT,
        "observations": observations,
        "plan_sha256": plan_sha256,
        "suite": SUITE.name,
    });
    create_new_output(records_path, &encode_canonical_document(&records)?)
}

fn exact_plan_cases(plan: &Value) -> BenchResult<BTreeMap<String, PlanCase>> {
    let plan_object = object(
        plan,
        &[
            "authority",
            "cases",
            "format",
            "identities",
            "input_sha256",
            "milestone",
            "nonclaim",
            "obligation_id",
            "path_id",
            "source_path",
            "suite",
            "target",
        ],
        "benchmark plan",
    )?;
    let cases = field(plan_object, "cases", "benchmark plan")?
        .as_array()
        .ok_or_else(|| "benchmark plan cases must be an array".to_owned())?;
    if cases.len() != SUITE.case_kinds.len() {
        return Err(
            "differential producer requires exactly one case for each of seven kinds".to_owned(),
        );
    }
    let mut by_id = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    for case in cases {
        let case = object(
            case,
            &["id", "input_sha256", "kind", "workload_sha256"],
            "benchmark case",
        )?;
        let id = string(case, "id", "benchmark case")?;
        let kind = string(case, "kind", "benchmark case")?;
        by_id.insert(
            id.to_owned(),
            PlanCase {
                input_sha256: string(case, "input_sha256", "benchmark case")?.to_owned(),
                kind: kind.to_owned(),
                workload_sha256: string(case, "workload_sha256", "benchmark case")?.to_owned(),
            },
        );
        kinds.insert(kind);
    }
    if kinds != SUITE.case_kinds.iter().copied().collect() {
        return Err("differential producer case-kind roster drifted".to_owned());
    }
    Ok(by_id)
}

fn plan_identities(plan: &Value) -> BenchResult<BTreeMap<String, String>> {
    let plan = plan
        .as_object()
        .ok_or_else(|| "benchmark plan must be an object".to_owned())?;
    let identities = field(plan, "identities", "benchmark plan")?
        .as_object()
        .ok_or_else(|| "benchmark identities must be an object".to_owned())?;
    identities
        .iter()
        .map(|(name, identity)| {
            identity
                .as_str()
                .map(|identity| (name.clone(), identity.to_owned()))
                .ok_or_else(|| format!("benchmark identity must be a string: {name}"))
        })
        .collect()
}

fn parse_pairs(
    value: &Value,
    base: &Path,
    plan_cases: &BTreeMap<String, PlanCase>,
    identities: &BTreeMap<String, String>,
    plan_sha256: &str,
) -> BenchResult<Vec<Pair>> {
    let document = object(
        value,
        &["authority", "format", "pairs", "plan_sha256", "suite"],
        "differential pairs manifest",
    )?;
    expect(document, "authority", PAIRS_AUTHORITY, "pairs authority")?;
    expect(document, "format", PAIRS_FORMAT, "pairs format")?;
    expect(document, "plan_sha256", plan_sha256, "pairs plan identity")?;
    expect(document, "suite", SUITE.name, "pairs suite")?;
    let values = field(document, "pairs", "differential pairs manifest")?
        .as_array()
        .ok_or_else(|| "differential pairs must be an array".to_owned())?;
    if values.len() != plan_cases.len() {
        return Err("differential pairs do not cover the exact seven-case plan".to_owned());
    }
    let mut prior: Option<&str> = None;
    let mut seen = BTreeSet::new();
    let mut manifest_paths = BTreeSet::new();
    let mut pairs = Vec::with_capacity(values.len());
    for value in values {
        let pair = object(
            value,
            &[
                "case_id",
                "ferric_output_manifest",
                "kind",
                "reference_output_manifest",
                "runner_transcript",
            ],
            "differential pair",
        )?;
        let case_id = string(pair, "case_id", "differential pair")?;
        if prior.is_some_and(|previous| previous >= case_id) {
            return Err("differential pairs must be uniquely sorted by case id".to_owned());
        }
        prior = Some(case_id);
        let kind = string(pair, "kind", "differential pair")?;
        let plan_case = plan_cases
            .get(case_id)
            .ok_or_else(|| format!("differential pair names an unknown case: {case_id}"))?;
        if plan_case.kind != kind {
            return Err(format!("differential pair drifted from plan: {case_id}"));
        }
        let runner_transcript_sha256 = parse_companion(
            field(pair, "runner_transcript", "differential pair")?,
            base,
            "runner transcript",
        )?;
        let ferric_path = relative_path(
            base,
            string(pair, "ferric_output_manifest", "differential pair")?,
            "Ferric output manifest",
        )?;
        let reference_path = relative_path(
            base,
            string(pair, "reference_output_manifest", "differential pair")?,
            "reference output manifest",
        )?;
        if !manifest_paths.insert(ferric_path.clone())
            || !manifest_paths.insert(reference_path.clone())
        {
            return Err("differential output manifest was reused".to_owned());
        }
        let ferric = parse_output(
            &ferric_path,
            case_id,
            plan_case,
            "ferric",
            identities,
            plan_sha256,
            &runner_transcript_sha256,
        )?;
        let reference = parse_output(
            &reference_path,
            case_id,
            plan_case,
            "reference",
            identities,
            plan_sha256,
            &runner_transcript_sha256,
        )?;
        seen.insert(case_id);
        pairs.push(Pair {
            case_id: case_id.to_owned(),
            kind: kind.to_owned(),
            ferric,
            reference,
            runner_transcript_sha256,
        });
    }
    if seen != plan_cases.keys().map(String::as_str).collect() {
        return Err("differential pair roster drifted from the plan".to_owned());
    }
    Ok(pairs)
}

fn parse_companion(value: &Value, base: &Path, description: &str) -> BenchResult<String> {
    let companion = object(value, &["bytes", "path", "sha256"], description)?;
    let bytes = field(companion, "bytes", description)?
        .as_u64()
        .ok_or_else(|| format!("{description} length must be an unsigned integer"))?;
    if bytes == 0 {
        return Err(format!("{description} must not be empty"));
    }
    let expected_sha256 = string(companion, "sha256", description)?;
    require_sha256(expected_sha256, &format!("{description} identity"))?;
    let path = relative_path(base, string(companion, "path", description)?, description)?;
    let (_, actual) = load_canonical_document(&path, description)?;
    if u64::try_from(actual.len()) != Ok(bytes) {
        return Err(format!("{description} length drifted"));
    }
    if sha256_identity(&actual) != expected_sha256 {
        return Err(format!("{description} SHA-256 drifted"));
    }
    Ok(expected_sha256.to_owned())
}

fn parse_output(
    path: &Path,
    case_id: &str,
    case: &PlanCase,
    producer: &str,
    identities: &BTreeMap<String, String>,
    plan_sha256: &str,
    runner_transcript_sha256: &str,
) -> BenchResult<Output> {
    let description = format!("{producer} output manifest");
    let (value, bytes) = load_canonical_document(path, &description)?;
    let output = object(
        &value,
        &[
            "authority",
            "case_id",
            "environment_sha256",
            "format",
            "input_sha256",
            "kind",
            "logits",
            "plan_sha256",
            "producer",
            "producer_sha256",
            "protocol_sha256",
            "runner_transcript_sha256",
            "shape",
            "tokens",
            "workload_sha256",
        ],
        &description,
    )?;
    expect(output, "authority", OUTPUT_AUTHORITY, "output authority")?;
    expect(output, "case_id", case_id, "output case id")?;
    expect(
        output,
        "environment_sha256",
        identity(identities, "environment")?,
        "output environment identity",
    )?;
    expect(output, "format", OUTPUT_FORMAT, "output format")?;
    expect(
        output,
        "input_sha256",
        &case.input_sha256,
        "output input identity",
    )?;
    expect(output, "kind", &case.kind, "output kind")?;
    expect(output, "plan_sha256", plan_sha256, "output plan identity")?;
    expect(output, "producer", producer, "output producer")?;

    let (producer_identity, protocol_identity) = if producer == "ferric" {
        ("benchmark-executable", "benchmark-protocol")
    } else {
        ("reference-implementation", "reference-protocol")
    };
    expect(
        output,
        "producer_sha256",
        identity(identities, producer_identity)?,
        "output producer identity",
    )?;
    expect(
        output,
        "protocol_sha256",
        identity(identities, protocol_identity)?,
        "output protocol identity",
    )?;
    expect(
        output,
        "runner_transcript_sha256",
        runner_transcript_sha256,
        "output runner transcript identity",
    )?;
    expect(
        output,
        "workload_sha256",
        &case.workload_sha256,
        "output workload identity",
    )?;

    let rows = rows_for_kind(&case.kind)?;
    let shape = object(
        field(output, "shape", &description)?,
        &["rows", "vocabulary_size"],
        "differential output shape",
    )?;
    expect_u64(shape, "rows", rows, "output row count")?;
    expect_u64(
        shape,
        "vocabulary_size",
        VOCABULARY_SIZE,
        "output vocabulary size",
    )?;
    let logits_bytes = rows
        .checked_mul(VOCABULARY_SIZE)
        .and_then(|elements| elements.checked_mul(BF16_BYTES))
        .ok_or_else(|| "logit payload length overflowed".to_owned())?;
    let token_bytes = rows
        .checked_mul(TOKEN_BYTES)
        .ok_or_else(|| "token payload length overflowed".to_owned())?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let logits = parse_payload(
        field(output, "logits", &description)?,
        base,
        "bf16-le",
        logits_bytes,
        "logit payload",
    )?;
    let tokens = parse_payload(
        field(output, "tokens", &description)?,
        base,
        "u32-le",
        token_bytes,
        "token payload",
    )?;
    if logits.path == tokens.path {
        return Err(format!("{producer} output reuses one payload path"));
    }
    Ok(Output {
        manifest_sha256: sha256_identity(&bytes),
        logits,
        tokens,
    })
}

fn parse_payload(
    value: &Value,
    base: &Path,
    encoding: &str,
    expected_bytes: u64,
    description: &str,
) -> BenchResult<Payload> {
    let payload = object(value, &["bytes", "encoding", "path", "sha256"], description)?;
    expect(payload, "encoding", encoding, "payload encoding")?;
    expect_u64(payload, "bytes", expected_bytes, "payload length")?;
    let sha256 = string(payload, "sha256", description)?;
    require_sha256(sha256, "payload identity")?;
    Ok(Payload {
        bytes: expected_bytes,
        path: relative_path(base, string(payload, "path", description)?, description)?,
        sha256: sha256.to_owned(),
    })
}

fn compare_pair(pair: &Pair) -> BenchResult<Comparison> {
    let rows = rows_for_kind(&pair.kind)?;
    let ferric_logits = open_payload(&pair.ferric.logits, "Ferric logit payload")?;
    let reference_logits = open_payload(&pair.reference.logits, "reference logit payload")?;
    let (maximum_logit_ulp_error, ferric_argmax, reference_argmax) = compare_logits(
        ferric_logits,
        reference_logits,
        rows,
        VOCABULARY_SIZE,
        &pair.ferric.logits,
        &pair.reference.logits,
    )?;
    let ferric_tokens = read_tokens(
        open_payload(&pair.ferric.tokens, "Ferric token payload")?,
        &pair.ferric.tokens,
        rows,
        "Ferric token payload",
    )?;
    let reference_tokens = read_tokens(
        open_payload(&pair.reference.tokens, "reference token payload")?,
        &pair.reference.tokens,
        rows,
        "reference token payload",
    )?;
    validate_argmax(&ferric_tokens, &ferric_argmax, "Ferric", &pair.case_id)?;
    validate_argmax(
        &reference_tokens,
        &reference_argmax,
        "reference",
        &pair.case_id,
    )?;
    let token_mismatches = ferric_tokens
        .iter()
        .zip(&reference_tokens)
        .filter(|(ferric, reference)| ferric != reference)
        .count();
    Ok(Comparison {
        compared_logits: rows
            .checked_mul(VOCABULARY_SIZE)
            .ok_or_else(|| "compared logit count overflowed".to_owned())?,
        compared_tokens: rows,
        maximum_logit_ulp_error,
        token_mismatches: u64::try_from(token_mismatches)
            .map_err(|_| "token mismatch count does not fit u64".to_owned())?,
    })
}

fn compare_logits<FR: Read, RR: Read>(
    mut ferric: CheckedReader<FR>,
    mut reference: CheckedReader<RR>,
    rows: u64,
    vocabulary: u64,
    ferric_payload: &Payload,
    reference_payload: &Payload,
) -> BenchResult<(u64, Vec<u32>, Vec<u32>)> {
    let row_capacity = usize::try_from(rows)
        .map_err(|_| "differential row count does not fit this host".to_owned())?;
    let mut ferric_argmax = Vec::with_capacity(row_capacity);
    let mut reference_argmax = Vec::with_capacity(row_capacity);
    let mut maximum_ulp = 0_u64;
    for row in 0..rows {
        let mut ferric_best = None;
        let mut reference_best = None;
        for token in 0..vocabulary {
            let ferric_bits = read_bf16(&mut ferric, "Ferric logit payload")?;
            let reference_bits = read_bf16(&mut reference, "reference logit payload")?;
            require_finite(ferric_bits, "Ferric", row, token)?;
            require_finite(reference_bits, "reference", row, token)?;
            maximum_ulp = maximum_ulp.max(u64::from(bf16_ulp(ferric_bits, reference_bits)));
            update_argmax(&mut ferric_best, ferric_bits, token)?;
            update_argmax(&mut reference_best, reference_bits, token)?;
        }
        ferric_argmax.push(
            ferric_best
                .map(|(_, token)| token)
                .ok_or_else(|| "Ferric logit row was empty".to_owned())?,
        );
        reference_argmax.push(
            reference_best
                .map(|(_, token)| token)
                .ok_or_else(|| "reference logit row was empty".to_owned())?,
        );
    }
    ferric.finish(ferric_payload, "Ferric logit payload")?;
    reference.finish(reference_payload, "reference logit payload")?;
    Ok((maximum_ulp, ferric_argmax, reference_argmax))
}

fn read_tokens<R: Read>(
    mut reader: CheckedReader<R>,
    payload: &Payload,
    rows: u64,
    description: &str,
) -> BenchResult<Vec<u32>> {
    let capacity = usize::try_from(rows)
        .map_err(|_| "differential row count does not fit this host".to_owned())?;
    let mut tokens = Vec::with_capacity(capacity);
    for _ in 0..rows {
        let mut bytes = [0_u8; 4];
        reader.read_exact(&mut bytes, description)?;
        tokens.push(u32::from_le_bytes(bytes));
    }
    reader.finish(payload, description)?;
    Ok(tokens)
}

fn read_bf16<R: Read>(reader: &mut CheckedReader<R>, description: &str) -> BenchResult<u16> {
    let mut bytes = [0_u8; 2];
    reader.read_exact(&mut bytes, description)?;
    Ok(u16::from_le_bytes(bytes))
}

fn require_finite(bits: u16, producer: &str, row: u64, token: u64) -> BenchResult<()> {
    if bits & 0x7f80 == 0x7f80 {
        return Err(format!(
            "{producer} output contains a nonfinite BF16 logit at row {row}, token {token}"
        ));
    }
    Ok(())
}

fn update_argmax(best: &mut Option<(f32, u32)>, bits: u16, token: u64) -> BenchResult<()> {
    let token = u32::try_from(token).map_err(|_| "token id does not fit u32".to_owned())?;
    let value = f32::from_bits(u32::from(bits) << 16);
    if best.is_none_or(|(prior, _)| value > prior) {
        *best = Some((value, token));
    }
    Ok(())
}

fn bf16_ulp(left: u16, right: u16) -> u32 {
    ordered_bf16(left).abs_diff(ordered_bf16(right))
}

fn ordered_bf16(bits: u16) -> u32 {
    let magnitude = u32::from(bits & 0x7fff);
    if bits & 0x8000 == 0 {
        0x8000 + magnitude
    } else {
        0x8000 - magnitude
    }
}

fn validate_argmax(
    tokens: &[u32],
    argmax: &[u32],
    producer: &str,
    case_id: &str,
) -> BenchResult<()> {
    for (row, (actual, expected)) in tokens.iter().zip(argmax).enumerate() {
        if actual != expected {
            return Err(format!(
                "{producer} token is not the lowest-ID BF16 argmax for {case_id} row {row}"
            ));
        }
    }
    Ok(())
}

fn open_payload(
    payload: &Payload,
    description: &str,
) -> BenchResult<CheckedReader<BufReader<File>>> {
    let metadata = fs::symlink_metadata(&payload.path)
        .map_err(|error| format!("cannot inspect {description}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{description} must be a regular nonsymlink file"));
    }
    if metadata.len() != payload.bytes {
        return Err(format!("{description} length drifted"));
    }
    let file =
        File::open(&payload.path).map_err(|error| format!("cannot open {description}: {error}"))?;
    if file
        .metadata()
        .map_err(|error| format!("cannot inspect opened {description}: {error}"))?
        .len()
        != payload.bytes
    {
        return Err(format!("opened {description} length drifted"));
    }
    Ok(CheckedReader::new(BufReader::new(file)))
}

fn raw_record(
    pair: &Pair,
    comparison: Comparison,
    plan_sha256: &str,
    pairs_sha256: &str,
) -> BenchResult<Value> {
    Ok(json!({
        "authority": RAW_AUTHORITY,
        "case_id": pair.case_id,
        "comparison": {
            "compared_logits": comparison.compared_logits,
            "compared_tokens": comparison.compared_tokens,
            "maximum_logit_ulp_error": comparison.maximum_logit_ulp_error,
            "token_mismatches": comparison.token_mismatches,
        },
        "ferric_output": output_record(&pair.ferric),
        "format": RAW_RECORD_FORMAT,
        "kind": pair.kind,
        "nonclaim": SUITE.nonclaim,
        "pairs_sha256": pairs_sha256,
        "plan_sha256": plan_sha256,
        "reference_output": output_record(&pair.reference),
        "runner_transcript_sha256": pair.runner_transcript_sha256,
        "shape": {
            "rows": rows_for_kind(&pair.kind)?,
            "vocabulary_size": VOCABULARY_SIZE,
        },
        "status": "compared",
    }))
}

fn output_record(output: &Output) -> Value {
    json!({
        "logits_bytes": output.logits.bytes,
        "logits_sha256": output.logits.sha256,
        "manifest_sha256": output.manifest_sha256,
        "tokens_bytes": output.tokens.bytes,
        "tokens_sha256": output.tokens.sha256,
    })
}

fn observation(pair: &Pair, comparison: Comparison, raw_bytes: &[u8]) -> Value {
    json!({
        "attributes": {
            "ferric-output-sha256": pair.ferric.manifest_sha256,
            "raw-record-sha256": sha256_identity(raw_bytes),
            "reference-output-sha256": pair.reference.manifest_sha256,
            "runner-transcript-sha256": pair.runner_transcript_sha256,
        },
        "case_id": pair.case_id,
        "kind": pair.kind,
        "measurements": {
            "compared-logits": [comparison.compared_logits],
            "compared-tokens": [comparison.compared_tokens],
            "maximum-logit-ulp-error": [comparison.maximum_logit_ulp_error],
            "token-mismatches": [comparison.token_mismatches],
        },
        "recorded_samples": 1,
        "status": "completed",
        "warmups": 0,
    })
}

fn rows_for_kind(kind: &str) -> BenchResult<u64> {
    match kind {
        "decode-s1-c8192" | "prefill-s1-t128" | "prefill-s1-t2048" | "prefill-s1-t512" => Ok(1),
        "decode-s8-c8192" | "prefill-s8-t128" => Ok(8),
        "decode-s32-c8192" => Ok(32),
        _ => Err(format!("unknown differential case kind: {kind}")),
    }
}

fn relative_path(base: &Path, value: &str, description: &str) -> BenchResult<PathBuf> {
    if value.is_empty() || value.len() > 1024 || !value.is_ascii() {
        return Err(format!("invalid {description} path"));
    }
    let relative = Path::new(value);
    if relative.is_absolute()
        || !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("{description} path must be a safe relative path"));
    }
    Ok(base.join(relative))
}

fn require_absent(path: &Path, description: &str) -> BenchResult<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(format!("{description} already exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot inspect {description}: {error}")),
    }
}

fn object<'a>(
    value: &'a Value,
    expected: &[&str],
    description: &str,
) -> BenchResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    if object.len() != expected.len() || !expected.iter().all(|key| object.contains_key(*key)) {
        return Err(format!("{description} fields drifted"));
    }
    Ok(object)
}

fn field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    description: &str,
) -> BenchResult<&'a Value> {
    object
        .get(key)
        .ok_or_else(|| format!("{description} is missing {key}"))
}

fn string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    description: &str,
) -> BenchResult<&'a str> {
    field(object, key, description)?
        .as_str()
        .ok_or_else(|| format!("{description} field must be a string: {key}"))
}

fn expect(
    object: &Map<String, Value>,
    key: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    if string(object, key, description)? != expected {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn expect_u64(
    object: &Map<String, Value>,
    key: &str,
    expected: u64,
    description: &str,
) -> BenchResult<()> {
    if field(object, key, description)?.as_u64() != Some(expected) {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn identity<'a>(identities: &'a BTreeMap<String, String>, name: &str) -> BenchResult<&'a str> {
    identities
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("benchmark plan is missing identity: {name}"))
}

fn require_sha256(value: &str, description: &str) -> BenchResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == value.as_bytes()[0])
    {
        return Err(format!("invalid {description}"));
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn bf16(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn payload(bytes: &[u8]) -> Payload {
        Payload {
            bytes: u64::try_from(bytes.len()).unwrap(),
            path: PathBuf::new(),
            sha256: sha256_identity(bytes),
        }
    }

    fn compare(
        ferric: &[u16],
        reference: &[u16],
        rows: u64,
        vocabulary: u64,
    ) -> BenchResult<(u64, Vec<u32>, Vec<u32>)> {
        let ferric = bf16(ferric);
        let reference = bf16(reference);
        compare_logits(
            CheckedReader::new(Cursor::new(&ferric)),
            CheckedReader::new(Cursor::new(&reference)),
            rows,
            vocabulary,
            &payload(&ferric),
            &payload(&reference),
        )
    }

    #[test]
    fn streaming_comparison_uses_monotonic_bf16_ulp_and_signed_zero_equality() {
        let (maximum, ferric, reference) = compare(
            &[0xbf80, 0x8000, 0x3f80, 0x4000],
            &[0xbf81, 0x0000, 0x3f81, 0x4000],
            1,
            4,
        )
        .unwrap();
        assert_eq!(maximum, 1);
        assert_eq!(ferric, vec![3]);
        assert_eq!(reference, vec![3]);
        assert_eq!(bf16_ulp(0x8000, 0x0000), 0);
        assert_eq!(bf16_ulp(0x8001, 0x0001), 2);
        assert_eq!(bf16_ulp(0xbf80, 0xbf81), 1);
    }

    #[test]
    fn lowest_id_argmax_is_required_for_each_output() {
        let (_, ferric, reference) =
            compare(&[0x3f80, 0x4000, 0x4000], &[0x4000, 0x4000, 0x3f80], 1, 3).unwrap();
        assert_eq!(ferric, vec![1]);
        assert_eq!(reference, vec![0]);
        assert!(validate_argmax(&[2], &ferric, "Ferric", "case.001").is_err());
    }

    #[test]
    fn nonfinite_truncated_and_substituted_payloads_fail_closed() {
        assert!(compare(&[0x7f80], &[0x3f80], 1, 1).is_err());
        assert!(compare(&[0x7fc1], &[0x3f80], 1, 1).is_err());
        assert!(compare(&[], &[0x3f80], 1, 1).is_err());

        let ferric = bf16(&[0x3f80]);
        let reference = bf16(&[0x3f80]);
        let substituted = Payload {
            bytes: 2,
            path: PathBuf::new(),
            sha256: sha256_identity(b"substituted"),
        };
        assert!(compare_logits(
            CheckedReader::new(Cursor::new(&ferric)),
            CheckedReader::new(Cursor::new(&reference)),
            1,
            1,
            &substituted,
            &payload(&reference),
        )
        .is_err());
    }

    #[test]
    fn seven_case_roster_and_shapes_are_fixed() {
        assert_eq!(SUITE.case_kinds.len(), 7);
        assert_eq!(rows_for_kind("decode-s1-c8192").unwrap(), 1);
        assert_eq!(rows_for_kind("decode-s32-c8192").unwrap(), 32);
        assert_eq!(rows_for_kind("decode-s8-c8192").unwrap(), 8);
        assert_eq!(rows_for_kind("prefill-s8-t128").unwrap(), 8);
        assert!(rows_for_kind("decode-s2-c8192").is_err());
    }

    #[test]
    fn paths_reject_absolute_parent_and_non_ascii_components() {
        let base = Path::new("/tmp/base");
        assert!(relative_path(base, "case/output.json", "test").is_ok());
        assert!(relative_path(base, "/tmp/output.json", "test").is_err());
        assert!(relative_path(base, "../output.json", "test").is_err());
        assert!(relative_path(base, "case/\u{2603}.json", "test").is_err());
    }
}
