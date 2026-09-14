//! Engineering diagnostics, deliberately outside qualification ingestion.

use super::{
    admit_input_identity, compare_pair, directory_roster, duplicate_secure_input_directory,
    encode_canonical_document, exact_plan_cases, expect, field, fstat, identity, json,
    load_benchmark_plan, mode_for_kind, object, open_pairs_parent, parse_output,
    parse_output_for_purpose, plan_identities, require_sha256, rows_for_kind, sha256_identity,
    string, unsigned, validate_capture_execution, BTreeMap, BTreeSet, BenchResult, Comparison,
    OsString, Output, OutputContext, OutputPurpose, OwnedFd, Pair, Path, PathBuf, PlanCase,
    SecureInputDirectory, SecureInputFile, TranscriptBinding, Value, Write, BF16_BYTES, SUITE,
    TARGET, TOKEN_BYTES, VOCABULARY_SIZE,
};
use std::io::Read;

pub(super) const REFERENCE_FORMAT: &str = "FERRIC-M1-ENGINEERING-REFERENCE-OUTPUT-V2";
pub(super) const REFERENCE_NONCLAIM: &str = "Engineering reference bytes only. Authority is none; this is not a qualification result, reviewed tolerance, benchmark, protected publication, or M1 gate closure.";
const ENGINEERING_CAPTURE_FORMAT: &str = "FERRIC-M1-TECHNICAL-PREQUALIFICATION-CAPTURE-V1";
const ENGINEERING_CAPTURE_NONCLAIM: &str = "Authority-none aggregate engineering observation only. This transcript authenticates no compiler origin or Worker V3 publication, selects no current protected publication, establishes no reference comparison, tolerance, numerical correctness, hardware correctness, performance, qualification, or m1.r29 closure.";
const REPORT_FORMAT: &str = "FERRIC-M1-ENGINEERING-SELECTED-COMPARISON-V2";
const REPORT_NONCLAIM: &str = "Numerical diagnostics for one engineering case only. Authority is none; no threshold acceptance, benchmark, protected publication, full seven-case R29 comparison, qualification, or M1 gate closure is produced.";
const SUITE_REPORT_FORMAT: &str = "FERRIC-M1-ENGINEERING-SUITE-COMPARISON-V1";
const SUITE_REPORT_NONCLAIM: &str = "Numerical diagnostics for seven engineering cases only. Authority is none; no threshold acceptance, reviewed tolerance, benchmark, protected publication, qualification, or M1 gate closure is produced.";
const FILES: [&str; 4] = [
    "logits.bf16le",
    "output.json",
    "runner.json",
    "tokens.u32le",
];

pub(super) fn require_canonical_case(case_id: &str, kind: &str) -> BenchResult<()> {
    if !SUITE.case_kinds.contains(&kind) || case_id != format!("{kind}.001") {
        return Err("engineering comparison requires a canonical seven-case identity".to_owned());
    }
    Ok(())
}

pub(super) fn command(arguments: &[OsString]) -> BenchResult<()> {
    let report = match arguments {
        [command, plan, case, capture, reference] if command == "compare-engineering-selected" => {
            let case_id = case.to_str().ok_or_else(|| "engineering case must be UTF-8".to_owned())?;
            compare_selected(Path::new(plan), case_id, Path::new(capture), Path::new(reference))?
        }
        [command, plan, captures, references] if command == "compare-engineering-suite" => {
            compare_suite(Path::new(plan), Path::new(captures), Path::new(references))?
        }
        _ => return Err("usage: ferric-m1-differential compare-engineering-selected PLAN CASE-ID CAPTURE-BUNDLE REFERENCE-BUNDLE | compare-engineering-suite PLAN CAPTURE-ROOT REFERENCE-ROOT".to_owned()),
    };
    std::io::stdout()
        .write_all(&encode_canonical_document(&report)?)
        .map_err(|error| format!("cannot write engineering comparison: {error}"))
}

struct HeldBundle {
    path: PathBuf,
    descriptor: OwnedFd,
    root: SecureInputDirectory,
    files: [SecureInputFile; 4],
    runner: Value,
    runner_bytes: Vec<u8>,
    manifest_bytes: Vec<u8>,
}

impl HeldBundle {
    fn open(path: &Path, rows: u64) -> BenchResult<Self> {
        let descriptor = open_pairs_parent(path)?;
        require_bundle_roster(&descriptor)?;
        let root = duplicate_secure_input_directory(&descriptor, "engineering bundle")?;
        let logits = root.open_exact(
            Path::new(FILES[0]),
            rows * VOCABULARY_SIZE * BF16_BYTES,
            "engineering logits",
        )?;
        let (_, manifest_bytes, manifest) =
            root.read_canonical_held(Path::new(FILES[1]), "engineering output manifest")?;
        let (runner, runner_bytes, transcript) =
            root.read_canonical_held(Path::new(FILES[2]), "engineering runner transcript")?;
        let tokens =
            root.open_exact(Path::new(FILES[3]), rows * TOKEN_BYTES, "engineering token")?;
        Ok(Self {
            path: path.to_owned(),
            descriptor,
            root,
            files: [logits, manifest, transcript, tokens],
            runner,
            runner_bytes,
            manifest_bytes,
        })
    }

    fn validate(&self) -> BenchResult<()> {
        let rebound = open_pairs_parent(&self.path)?;
        let before = fstat(&self.descriptor)
            .map_err(|error| format!("cannot inspect held engineering bundle: {error}"))?;
        let after = fstat(&rebound)
            .map_err(|error| format!("cannot inspect rebound engineering bundle: {error}"))?;
        if before.st_dev != after.st_dev || before.st_ino != after.st_ino {
            return Err("engineering bundle directory binding changed".to_owned());
        }
        require_bundle_roster(&self.descriptor)?;
        for (name, held) in FILES.iter().zip(&self.files) {
            self.root
                .validate_binding(Path::new(name), held, "engineering bundle input")?;
        }
        Ok(())
    }

    fn binds_output(&self, output: &Output) -> BenchResult<()> {
        if output.manifest_identity != self.files[1].identity()
            || output.manifest_sha256 != sha256_identity(&self.manifest_bytes)
            || output.logits.path != Path::new(FILES[0])
            || output.tokens.path != Path::new(FILES[3])
            || output.logits.input.as_ref().map(SecureInputFile::identity)
                != Some(self.files[0].identity())
            || output.tokens.input.as_ref().map(SecureInputFile::identity)
                != Some(self.files[3].identity())
        {
            return Err("engineering output does not bind the held bundle files".to_owned());
        }
        self.validate()
    }

    fn validate_ordered_rows(&mut self, rows: u64) -> BenchResult<()> {
        let hashes = self.runner["logits_row_sha256"]
            .as_array()
            .ok_or_else(|| "engineering row identities must be an array".to_owned())?;
        if u64::try_from(hashes.len()) != Ok(rows) {
            return Err("engineering row identity roster drifted".to_owned());
        }
        let mut row = vec![
            0_u8;
            usize::try_from(VOCABULARY_SIZE * BF16_BYTES)
                .map_err(|_| "engineering row size does not fit usize".to_owned())?
        ];
        for expected in hashes {
            self.files[0]
                .read_exact(&mut row)
                .map_err(|error| format!("cannot read held engineering logits row: {error}"))?;
            if expected.as_str() != Some(sha256_identity(&row).as_str()) {
                return Err(
                    "engineering ordered row identity differs from capture bytes".to_owned(),
                );
            }
        }
        if self.files[0]
            .read(&mut [0_u8; 1])
            .map_err(|error| format!("cannot finish held engineering logits: {error}"))?
            != 0
        {
            return Err("engineering logits contain trailing row bytes".to_owned());
        }
        self.validate()
    }
}

fn require_bundle_roster(descriptor: &OwnedFd) -> BenchResult<()> {
    if directory_roster(descriptor, "engineering bundle")?
        != FILES.into_iter().map(str::to_owned).collect()
    {
        return Err("engineering bundle file roster drifted".to_owned());
    }
    Ok(())
}

struct HeldPlan {
    path: PathBuf,
    name: PathBuf,
    root: SecureInputDirectory,
    input: SecureInputFile,
    bytes: Vec<u8>,
    sha256: String,
    cases: BTreeMap<String, PlanCase>,
    identities: BTreeMap<String, String>,
}

impl HeldPlan {
    fn open(path: &Path) -> BenchResult<Self> {
        let parent = open_pairs_parent(path.parent().unwrap_or_else(|| Path::new(".")))?;
        let root = duplicate_secure_input_directory(&parent, "engineering plan parent")?;
        let name = PathBuf::from(
            path.file_name()
                .ok_or_else(|| "plan path has no filename".to_owned())?,
        );
        let (_, bytes, input) = root.read_canonical_held(&name, "engineering plan")?;
        let (plan, loaded_bytes) = load_benchmark_plan(&SUITE, path)?;
        if bytes != loaded_bytes {
            return Err("engineering plan changed during admission".to_owned());
        }
        let cases = exact_plan_cases(&plan)?;
        for (case_id, case) in &cases {
            require_canonical_case(case_id, &case.kind)?;
        }
        Ok(Self {
            path: path.to_owned(),
            name,
            root,
            input,
            sha256: sha256_identity(&bytes),
            bytes,
            cases,
            identities: plan_identities(&plan)?,
        })
    }

    fn validate(&self) -> BenchResult<()> {
        self.root
            .validate_binding(&self.name, &self.input, "engineering plan")?;
        let parent = open_pairs_parent(self.path.parent().unwrap_or_else(|| Path::new(".")))?;
        let rebound = duplicate_secure_input_directory(&parent, "engineering rebound plan parent")?;
        rebound.validate_binding(&self.name, &self.input, "engineering rebound plan")?;
        let (_, final_bytes) = load_benchmark_plan(&SUITE, &self.path)?;
        if final_bytes != self.bytes {
            return Err("engineering plan changed during comparison".to_owned());
        }
        Ok(())
    }
}

struct HeldSuiteRoot {
    path: PathBuf,
    descriptor: OwnedFd,
    roster: BTreeSet<String>,
}

impl HeldSuiteRoot {
    fn open(path: &Path, producer: &str) -> BenchResult<Self> {
        let root = Self {
            path: path.to_owned(),
            descriptor: open_pairs_parent(path)?,
            roster: SUITE
                .case_kinds
                .iter()
                .map(|kind| format!("{kind}.{producer}.bundle"))
                .collect(),
        };
        root.validate()?;
        Ok(root)
    }

    fn validate(&self) -> BenchResult<()> {
        let rebound = open_pairs_parent(&self.path)?;
        let before = fstat(&self.descriptor)
            .map_err(|error| format!("cannot inspect held engineering root: {error}"))?;
        let after = fstat(&rebound)
            .map_err(|error| format!("cannot inspect rebound engineering root: {error}"))?;
        if before.st_dev != after.st_dev
            || before.st_ino != after.st_ino
            || directory_roster(&self.descriptor, "engineering suite root")? != self.roster
        {
            return Err("engineering suite root binding or exact roster changed".to_owned());
        }
        Ok(())
    }
}

fn compare_selected(
    plan_path: &Path,
    case_id: &str,
    capture_path: &Path,
    reference_path: &Path,
) -> BenchResult<Value> {
    let plan = HeldPlan::open(plan_path)?;
    let case = plan
        .cases
        .get(case_id)
        .ok_or_else(|| "plan lacks selected engineering case".to_owned())?;
    let rows = rows_for_kind(&case.kind)?;
    let mut capture = HeldBundle::open(capture_path, rows)?;
    let reference = HeldBundle::open(reference_path, rows)?;
    let mut seen = BTreeSet::new();
    admit_input_identity(&mut seen, plan.input.identity(), "engineering plan")?;
    for bundle in [&capture, &reference] {
        for held in &bundle.files {
            admit_input_identity(&mut seen, held.identity(), "engineering bundle input")?;
        }
    }
    let (report, _) = compare_case(&plan, case_id, case, &mut capture, &reference)?;
    capture.validate()?;
    reference.validate()?;
    plan.validate()?;
    Ok(report)
}

fn compare_suite(
    plan_path: &Path,
    capture_path: &Path,
    reference_path: &Path,
) -> BenchResult<Value> {
    let plan = HeldPlan::open(plan_path)?;
    let captures = HeldSuiteRoot::open(capture_path, "capture")?;
    let references = HeldSuiteRoot::open(reference_path, "reference")?;
    let mut seen = BTreeSet::new();
    admit_input_identity(&mut seen, plan.input.identity(), "engineering plan")?;
    let mut bundles = Vec::new();
    for kind in SUITE.case_kinds {
        let case_id = format!("{kind}.001");
        let case = plan
            .cases
            .get(&case_id)
            .ok_or_else(|| "plan lacks canonical engineering case".to_owned())?;
        let rows = rows_for_kind(&case.kind)?;
        let capture = HeldBundle::open(&capture_path.join(format!("{kind}.capture.bundle")), rows)?;
        let reference = HeldBundle::open(
            &reference_path.join(format!("{kind}.reference.bundle")),
            rows,
        )?;
        for bundle in [&capture, &reference] {
            for held in &bundle.files {
                admit_input_identity(&mut seen, held.identity(), "engineering suite bundle input")?;
            }
        }
        bundles.push((case_id, capture, reference));
    }
    let mut reports = Vec::new();
    let mut total = Comparison {
        compared_logits: 0,
        compared_tokens: 0,
        maximum_logit_ulp_error: 0,
        token_mismatches: 0,
    };
    for (case_id, capture, reference) in &mut bundles {
        let (report, comparison) = compare_case(
            &plan,
            case_id,
            &plan.cases[case_id.as_str()],
            capture,
            reference,
        )?;
        total.compared_logits = total
            .compared_logits
            .checked_add(comparison.compared_logits)
            .ok_or_else(|| "engineering suite logit count overflowed".to_owned())?;
        total.compared_tokens = total
            .compared_tokens
            .checked_add(comparison.compared_tokens)
            .ok_or_else(|| "engineering suite token count overflowed".to_owned())?;
        total.token_mismatches = total
            .token_mismatches
            .checked_add(comparison.token_mismatches)
            .ok_or_else(|| "engineering suite mismatch count overflowed".to_owned())?;
        total.maximum_logit_ulp_error = total
            .maximum_logit_ulp_error
            .max(comparison.maximum_logit_ulp_error);
        reports.push(report);
    }
    // Keep every earlier input alive until the complete suite has been checked.
    for (_, capture, reference) in &bundles {
        capture.validate()?;
        reference.validate()?;
    }
    captures.validate()?;
    references.validate()?;
    plan.validate()?;
    if reports.len() != 7
        || total.compared_tokens != 52
        || total.compared_logits != 52 * VOCABULARY_SIZE
    {
        return Err("engineering suite comparison geometry drifted".to_owned());
    }
    Ok(json!({
        "authority": "none", "case_count": 7, "case_scope": "seven-canonical-cases",
        "cases": reports, "format": SUITE_REPORT_FORMAT, "identities": plan.identities,
        "metrics": comparison_metrics(&total), "nonclaim": SUITE_REPORT_NONCLAIM,
        "plan_sha256": plan.sha256, "qualification": false, "target": TARGET,
    }))
}

fn comparison_metrics(comparison: &Comparison) -> Value {
    json!({
        "compared_logits": comparison.compared_logits,
        "compared_tokens": comparison.compared_tokens,
        "maximum_logit_ulp_error": comparison.maximum_logit_ulp_error,
        "token_mismatches": comparison.token_mismatches,
    })
}

fn compare_case(
    plan: &HeldPlan,
    case_id: &str,
    case: &PlanCase,
    capture: &mut HeldBundle,
    reference: &HeldBundle,
) -> BenchResult<(Value, Comparison)> {
    if capture.runner_bytes != reference.runner_bytes {
        return Err("engineering reference retained a different capture transcript".to_owned());
    }
    let transcript = validate_engineering_capture(
        &capture.runner,
        case_id,
        case,
        &plan.identities,
        &plan.sha256,
    )?;
    capture.validate_ordered_rows(rows_for_kind(&case.kind)?)?;
    let runner_sha256 = sha256_identity(&capture.runner_bytes);
    let context = OutputContext {
        case_id,
        case,
        identities: &plan.identities,
        plan_sha256: &plan.sha256,
        runner_transcript_sha256: &runner_sha256,
    };
    let ferric = parse_output(&capture.root, Path::new("output.json"), "ferric", &context)?;
    let reference_output = parse_output_for_purpose(
        &reference.root,
        Path::new("output.json"),
        "reference",
        &context,
        OutputPurpose::EngineeringReference,
    )?;
    capture.binds_output(&ferric)?;
    reference.binds_output(&reference_output)?;
    if ferric.logits.sha256 != transcript.logits_sha256
        || ferric.tokens.sha256 != transcript.tokens_sha256
    {
        return Err("engineering Ferric payload identities differ from the transcript".to_owned());
    }
    let mut pair = Pair {
        case_id: case_id.to_owned(),
        kind: case.kind.to_owned(),
        ferric,
        reference: reference_output,
        runner_transcript_sha256: runner_sha256,
    };
    let comparison = compare_pair(&mut pair)?;
    Ok((
        json!({
            "authority": "none",
            "case_id": case_id,
            "case_scope": "selected-case-only",
            "ferric_output_sha256": pair.ferric.manifest_sha256,
            "format": REPORT_FORMAT,
            "identities": plan.identities,
            "kind": case.kind,
            "metrics": comparison_metrics(&comparison),
            "nonclaim": REPORT_NONCLAIM,
            "plan_sha256": plan.sha256,
            "qualification": false,
            "reference_output_sha256": pair.reference.manifest_sha256,
            "runner_transcript_sha256": pair.runner_transcript_sha256,
            "target": TARGET,
        }),
        comparison,
    ))
}

fn validate_engineering_capture(
    value: &Value,
    case_id: &str,
    case: &PlanCase,
    identities: &BTreeMap<String, String>,
    plan_sha256: &str,
) -> BenchResult<TranscriptBinding> {
    require_canonical_case(case_id, &case.kind)?;
    let mode = mode_for_kind(&case.kind)?;
    let rows = rows_for_kind(&case.kind)?;
    let transcript = object(
        value,
        &[
            "artifact_authority",
            "authority",
            "benchmark_executable_sha256",
            "benchmark_protocol_sha256",
            "case_id",
            "compact_sha256",
            "device_identity_sha256",
            "dispatch_generation",
            "environment_sha256",
            "execution",
            "format",
            "gpu_unique_id",
            "input_sha256",
            "kernel_artifact_manifest_sha256",
            "kind",
            "logits_row_sha256",
            "logits_sha256",
            "nonclaim",
            "plan_sha256",
            "program_catalog_sha256",
            "runner_declaration_sha256",
            "selection",
            "status",
            "target",
            "tokens_sha256",
            "workload_sha256",
        ],
        "engineering capture transcript",
    )?;
    for (key, expected) in [
        ("artifact_authority", "none"),
        ("authority", "aggregate-engineering-observation-only"),
        (
            "benchmark_executable_sha256",
            identity(identities, "benchmark-executable")?,
        ),
        (
            "benchmark_protocol_sha256",
            identity(identities, "benchmark-protocol")?,
        ),
        ("case_id", case_id),
        ("environment_sha256", identity(identities, "environment")?),
        ("format", ENGINEERING_CAPTURE_FORMAT),
        ("input_sha256", &case.input_sha256),
        ("kind", &case.kind),
        ("nonclaim", ENGINEERING_CAPTURE_NONCLAIM),
        ("plan_sha256", plan_sha256),
        (
            "runner_declaration_sha256",
            identity(identities, "generated-plan")?,
        ),
        ("status", "OBSERVED-NON-AUTHORITATIVE"),
        ("target", TARGET),
        ("workload_sha256", &case.workload_sha256),
    ] {
        expect(transcript, key, expected, "engineering capture transcript")?;
    }
    for key in [
        "compact_sha256",
        "device_identity_sha256",
        "kernel_artifact_manifest_sha256",
        "logits_sha256",
        "program_catalog_sha256",
        "tokens_sha256",
    ] {
        require_sha256(
            string(transcript, key, "engineering capture transcript")?,
            key,
        )?;
    }
    let gpu = unsigned(
        transcript,
        "gpu_unique_id",
        "engineering capture transcript",
    )?;
    let environment = encode_canonical_document(&json!({
        "format": "FERRIC-M1-QUALIFICATION-ENVIRONMENT-V1", "gpu_unique_id": gpu, "target": TARGET,
    }))?;
    if gpu == 0 || sha256_identity(&environment) != identity(identities, "environment")? {
        return Err("engineering capture GPU does not bind the plan environment".to_owned());
    }
    let generation = unsigned(
        transcript,
        "dispatch_generation",
        "engineering capture transcript",
    )?;
    if generation == 0 {
        return Err("engineering capture generation must be nonzero".to_owned());
    }
    validate_capture_execution(
        field(transcript, "execution", "engineering capture transcript")?,
        &case.kind,
        mode,
        rows,
        generation,
    )?;
    if mode == "decode"
        && value["execution"]["context_plan_sha256"].as_str()
            != Some(identity(
                identities,
                &format!("dispatch-graph-{}", case.kind),
            )?)
    {
        return Err(
            "engineering decode context plan differs from the selected plan identity".to_owned(),
        );
    }
    let selection = object(
        field(transcript, "selection", "engineering capture transcript")?,
        &["bucket", "mode", "role"],
        "engineering capture selection",
    )?;
    for (key, expected) in [
        ("bucket", case.kind.as_str()),
        ("mode", mode),
        ("role", "target-8b"),
    ] {
        expect(selection, key, expected, "engineering capture selection")?;
    }
    let logits_sha256 = string(
        transcript,
        "logits_sha256",
        "engineering capture transcript",
    )?;
    let row_hashes = field(
        transcript,
        "logits_row_sha256",
        "engineering capture transcript",
    )?
    .as_array()
    .ok_or_else(|| "engineering row identities must be an array".to_owned())?;
    if u64::try_from(row_hashes.len()) != Ok(rows) {
        return Err("engineering row identity roster drifted".to_owned());
    }
    for hash in row_hashes {
        require_sha256(
            hash.as_str()
                .ok_or_else(|| "engineering row identity must be a string".to_owned())?,
            "engineering row identity",
        )?;
    }
    Ok(TranscriptBinding {
        logits_sha256: logits_sha256.to_owned(),
        tokens_sha256: string(
            transcript,
            "tokens_sha256",
            "engineering capture transcript",
        )?
        .to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::{
        validate_capture_transcript, ACCEPTANCE_RESULT_FORMAT, CAPTURE_FORMAT, CAPTURE_NONCLAIM,
        OUTPUT_AUTHORITY, OUTPUT_FORMAT, PAIRS_FORMAT, RAW_RECORD_FORMAT, RECORDS_FORMAT,
    };
    use super::*;
    use std::fs;

    const CASE_ID: &str = "prefill-s1-t128.001";
    const KIND: &str = "prefill-s1-t128";

    struct Fixture {
        _temporary: super::super::tests::TestDirectory,
        plan: PathBuf,
        capture: PathBuf,
        reference: PathBuf,
        captures: PathBuf,
        references: PathBuf,
        case_id: String,
    }

    fn fixture() -> Fixture {
        fixture_for_kind(KIND)
    }

    fn fixture_for_kind(selected_kind: &str) -> Fixture {
        let all = super::super::tests::pairs_fixture();
        let mut plan: Value = serde_json::from_slice(&fs::read(&all.plan).unwrap()).unwrap();
        let environment = encode_canonical_document(&json!({
            "format": "FERRIC-M1-QUALIFICATION-ENVIRONMENT-V1", "gpu_unique_id": 23, "target": TARGET,
        })).unwrap();
        plan["identities"]["environment"] = json!(sha256_identity(&environment));
        let plan_bytes = encode_canonical_document(&plan).unwrap();
        fs::write(&all.plan, &plan_bytes).unwrap();
        let plan_sha256 = sha256_identity(&plan_bytes);
        for kind in SUITE.case_kinds {
            let capture = all.captures.join(format!("{kind}.capture.bundle"));
            let reference = all.references.join(format!("{kind}.reference.bundle"));
            let rows = rows_for_kind(kind).unwrap();
            let row_bytes = usize::try_from(VOCABULARY_SIZE * BF16_BYTES).unwrap();
            let mut logits = vec![0_u8; usize::try_from(rows).unwrap() * row_bytes];
            let mut tokens = Vec::new();
            for lane in 0..usize::try_from(rows).unwrap() {
                let offset = lane * row_bytes + lane * 2;
                logits[offset..offset + 2].copy_from_slice(&0x3f80_u16.to_le_bytes());
                tokens.extend_from_slice(&u32::try_from(lane).unwrap().to_le_bytes());
            }
            let mut runner: Value =
                serde_json::from_slice(&fs::read(capture.join("runner.json")).unwrap()).unwrap();
            runner["artifact_authority"] = json!("none");
            runner["authority"] = json!("aggregate-engineering-observation-only");
            runner["format"] = json!(ENGINEERING_CAPTURE_FORMAT);
            runner["nonclaim"] = json!(ENGINEERING_CAPTURE_NONCLAIM);
            runner["status"] = json!("OBSERVED-NON-AUTHORITATIVE");
            runner["plan_sha256"] = json!(plan_sha256);
            runner["environment_sha256"] = plan["identities"]["environment"].clone();
            runner["runner_declaration_sha256"] = plan["identities"]["generated-plan"].clone();
            runner["logits_sha256"] = json!(sha256_identity(&logits));
            runner["logits_row_sha256"] = json!(logits
                .chunks_exact(row_bytes)
                .map(sha256_identity)
                .collect::<Vec<_>>());
            runner["tokens_sha256"] = json!(sha256_identity(&tokens));
            if mode_for_kind(kind).unwrap() == "decode" {
                runner["execution"]["context_plan_sha256"] =
                    plan["identities"][format!("dispatch-graph-{kind}")].clone();
            }
            let runner_bytes = encode_canonical_document(&runner).unwrap();
            for bundle in [&capture, &reference] {
                fs::write(bundle.join("logits.bf16le"), &logits).unwrap();
                fs::write(bundle.join("tokens.u32le"), &tokens).unwrap();
                fs::write(bundle.join("runner.json"), &runner_bytes).unwrap();
                let mut manifest: Value =
                    serde_json::from_slice(&fs::read(bundle.join("output.json")).unwrap()).unwrap();
                manifest["plan_sha256"] = json!(plan_sha256);
                manifest["environment_sha256"] = plan["identities"]["environment"].clone();
                manifest["runner_transcript_sha256"] = json!(sha256_identity(&runner_bytes));
                manifest["logits"]["sha256"] = json!(sha256_identity(&logits));
                manifest["tokens"]["sha256"] = json!(sha256_identity(&tokens));
                if bundle == &reference {
                    manifest["authority"] = json!("none");
                    manifest["format"] = json!(REFERENCE_FORMAT);
                    manifest["nonclaim"] = json!(REFERENCE_NONCLAIM);
                    manifest["qualification"] = json!(false);
                }
                fs::write(
                    bundle.join("output.json"),
                    encode_canonical_document(&manifest).unwrap(),
                )
                .unwrap();
            }
        }
        Fixture {
            _temporary: all.temporary,
            plan: all.plan,
            capture: all.captures.join(format!("{selected_kind}.capture.bundle")),
            reference: all
                .references
                .join(format!("{selected_kind}.reference.bundle")),
            captures: all.captures,
            references: all.references,
            case_id: format!("{selected_kind}.001"),
        }
    }

    fn rewrite_runner(fixture: &Fixture, change: impl FnOnce(&mut Value)) {
        let mut runner: Value =
            serde_json::from_slice(&fs::read(fixture.capture.join("runner.json")).unwrap())
                .unwrap();
        change(&mut runner);
        let bytes = encode_canonical_document(&runner).unwrap();
        for bundle in [&fixture.capture, &fixture.reference] {
            fs::write(bundle.join("runner.json"), &bytes).unwrap();
            let mut output: Value =
                serde_json::from_slice(&fs::read(bundle.join("output.json")).unwrap()).unwrap();
            output["runner_transcript_sha256"] = json!(sha256_identity(&bytes));
            fs::write(
                bundle.join("output.json"),
                encode_canonical_document(&output).unwrap(),
            )
            .unwrap();
        }
    }

    fn compare(fixture: &Fixture) -> BenchResult<Value> {
        compare_selected(
            &fixture.plan,
            &fixture.case_id,
            &fixture.capture,
            &fixture.reference,
        )
    }

    #[test]
    fn engineering_selected_runs_real_numerical_core_without_qualification_result() {
        let fixture = fixture();
        let report = compare(&fixture).unwrap();
        assert_eq!(report["format"], REPORT_FORMAT);
        assert_eq!(report["authority"], "none");
        assert_eq!(report["qualification"], false);
        assert_eq!(
            report["metrics"],
            json!({
                "compared_logits": VOCABULARY_SIZE, "compared_tokens": 1,
                "maximum_logit_ulp_error": 0, "token_mismatches": 0,
            })
        );
        for format in [
            PAIRS_FORMAT,
            RAW_RECORD_FORMAT,
            ACCEPTANCE_RESULT_FORMAT,
            RECORDS_FORMAT,
        ] {
            assert_ne!(report["format"], format);
        }
    }

    #[test]
    fn engineering_selected_reports_real_difference_not_threshold_acceptance() {
        let fixture = fixture();
        let mut logits = fs::read(fixture.reference.join("logits.bf16le")).unwrap();
        logits[2..4].copy_from_slice(&0x4000_u16.to_le_bytes());
        let tokens = 1_u32.to_le_bytes();
        fs::write(fixture.reference.join("logits.bf16le"), &logits).unwrap();
        fs::write(fixture.reference.join("tokens.u32le"), tokens).unwrap();
        let path = fixture.reference.join("output.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["logits"]["sha256"] = json!(sha256_identity(&logits));
        manifest["tokens"]["sha256"] = json!(sha256_identity(&tokens));
        fs::write(path, encode_canonical_document(&manifest).unwrap()).unwrap();
        let report = compare(&fixture).unwrap();
        assert_eq!(report["metrics"]["token_mismatches"], 1);
        assert!(
            report["metrics"]["maximum_logit_ulp_error"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(report.get("accepted").is_none());
    }

    #[test]
    fn engineering_selected_canonical_transcript_substitutions_fail_semantic_checks() {
        let substitutions = [
            ("artifact_authority", json!("qualified")),
            (
                "authority",
                json!("observed-target-only-qualification-capture"),
            ),
            ("format", json!(CAPTURE_FORMAT)),
            ("status", json!("OBSERVED")),
            ("case_id", json!("prefill-s1-t512.001")),
            ("gpu_unique_id", json!(24)),
            ("dispatch_generation", json!(0)),
            ("runner_declaration_sha256", json!("f".repeat(64))),
            ("plan_sha256", json!("f".repeat(64))),
            ("benchmark_executable_sha256", json!("f".repeat(64))),
            ("benchmark_protocol_sha256", json!("f".repeat(64))),
            ("environment_sha256", json!("f".repeat(64))),
            ("input_sha256", json!("f".repeat(64))),
            ("workload_sha256", json!("f".repeat(64))),
            ("logits_row_sha256", json!(["f".repeat(64)])),
            (
                "selection",
                json!({"bucket": KIND, "mode": "prefill", "role": "draft-06b"}),
            ),
            ("extra", json!(true)),
        ];
        for (key, value) in substitutions {
            let fixture = fixture();
            rewrite_runner(&fixture, |runner| runner[key] = value);
            assert!(
                compare(&fixture).is_err(),
                "accepted canonical substitution {key}"
            );
        }
    }

    #[test]
    fn engineering_selected_and_qualification_parsers_remain_disjoint() {
        let fixture = fixture();
        let (plan, bytes) = load_benchmark_plan(&SUITE, &fixture.plan).unwrap();
        let cases = exact_plan_cases(&plan).unwrap();
        let identities = plan_identities(&plan).unwrap();
        let runner: Value =
            serde_json::from_slice(&fs::read(fixture.capture.join("runner.json")).unwrap())
                .unwrap();
        assert!(validate_capture_transcript(
            &runner,
            CASE_ID,
            &cases[CASE_ID],
            &identities,
            &sha256_identity(&bytes)
        )
        .is_err());
        let held = HeldBundle::open(&fixture.reference, 1).unwrap();
        let context = OutputContext {
            case_id: CASE_ID,
            case: &cases[CASE_ID],
            identities: &identities,
            plan_sha256: &sha256_identity(&bytes),
            runner_transcript_sha256: &sha256_identity(&held.runner_bytes),
        };
        assert!(parse_output(&held.root, Path::new("output.json"), "reference", &context).is_err());
        rewrite_runner(&fixture, |runner| {
            runner.as_object_mut().unwrap().remove("artifact_authority");
            runner["format"] = json!(CAPTURE_FORMAT);
            runner["authority"] = json!("observed-target-only-qualification-capture");
            runner["status"] = json!("OBSERVED");
            runner["nonclaim"] = json!(CAPTURE_NONCLAIM);
        });
        assert!(compare(&fixture).is_err());
        assert!(require_canonical_case("prefill-s1-t128.001", "prefill-s1-t512").is_err());
        assert!(require_canonical_case("prefill-s1-t512.002", "prefill-s1-t512").is_err());
    }

    #[test]
    fn engineering_selected_rejects_finite_argmax_hash_and_file_custody_failures() {
        for failure in [
            "nonfinite",
            "argmax",
            "hash",
            "trailing",
            "extra",
            "alias",
            "symlink",
            "reference-format",
        ] {
            let fixture = fixture();
            let path = fixture.reference.join("logits.bf16le");
            let manifest_path = fixture.reference.join("output.json");
            let mut manifest: Value =
                serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
            match failure {
                "nonfinite" | "argmax" | "hash" => {
                    let mut logits = fs::read(&path).unwrap();
                    logits[2..4].copy_from_slice(
                        &(if failure == "nonfinite" {
                            0x7f80_u16
                        } else {
                            0x4000_u16
                        })
                        .to_le_bytes(),
                    );
                    fs::write(&path, &logits).unwrap();
                    if failure != "hash" {
                        manifest["logits"]["sha256"] = json!(sha256_identity(&logits));
                    }
                }
                "trailing" => fs::write(
                    &path,
                    vec![0_u8; usize::try_from(VOCABULARY_SIZE * BF16_BYTES + 2).unwrap()],
                )
                .unwrap(),
                "extra" => fs::write(fixture.reference.join("extra"), b"extra").unwrap(),
                "alias" | "symlink" => {
                    fs::remove_file(&path).unwrap();
                    let source = fixture.capture.join("logits.bf16le");
                    if failure == "alias" {
                        fs::hard_link(source, &path).unwrap();
                    } else {
                        std::os::unix::fs::symlink(source, &path).unwrap();
                    }
                }
                "reference-format" => {
                    manifest["format"] = json!(OUTPUT_FORMAT);
                    manifest["authority"] = json!(OUTPUT_AUTHORITY);
                    manifest.as_object_mut().unwrap().remove("nonclaim");
                    manifest.as_object_mut().unwrap().remove("qualification");
                }
                _ => unreachable!(),
            }
            fs::write(manifest_path, encode_canonical_document(&manifest).unwrap()).unwrap();
            assert!(compare(&fixture).is_err(), "accepted {failure}");
        }
    }

    #[test]
    fn engineering_selected_retained_files_reject_name_or_directory_replacement() {
        let fixture = fixture();
        let held = HeldBundle::open(&fixture.reference, 1).unwrap();
        let path = fixture.reference.join("tokens.u32le");
        fs::remove_file(&path).unwrap();
        fs::write(&path, [0_u8; 4]).unwrap();
        assert!(held.validate().is_err());
        let next = self::fixture();
        let held = HeldBundle::open(&next.reference, 1).unwrap();
        let moved = next.reference.with_extension("moved");
        fs::rename(&next.reference, &moved).unwrap();
        fs::create_dir(&next.reference).unwrap();
        assert!(held.validate().is_err());
    }

    fn compare_all(fixture: &Fixture) -> BenchResult<Value> {
        compare_suite(&fixture.plan, &fixture.captures, &fixture.references)
    }

    fn rewrite_output(path: &Path, change: impl FnOnce(&mut Value)) {
        let mut output: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        change(&mut output);
        fs::write(path, encode_canonical_document(&output).unwrap()).unwrap();
    }

    #[test]
    fn engineering_suite_and_each_selected_case_compare_all_seven_kinds_and_52_rows() {
        let fixture = fixture();
        let suite = compare_all(&fixture).unwrap();
        assert_eq!(suite["format"], SUITE_REPORT_FORMAT);
        assert_eq!(suite["authority"], "none");
        assert_eq!(suite["qualification"], false);
        assert_eq!(suite["case_count"], 7);
        assert_eq!(
            suite["metrics"],
            json!({
                "compared_logits": 52 * VOCABULARY_SIZE, "compared_tokens": 52,
                "maximum_logit_ulp_error": 0, "token_mismatches": 0,
            })
        );
        assert!(suite.get("accepted").is_none());
        let reports = suite["cases"].as_array().unwrap();
        assert_eq!(reports.len(), 7);
        for (ordinal, kind) in SUITE.case_kinds.iter().enumerate() {
            let case_id = format!("{kind}.001");
            let selected = compare_selected(
                &fixture.plan,
                &case_id,
                &fixture.captures.join(format!("{kind}.capture.bundle")),
                &fixture.references.join(format!("{kind}.reference.bundle")),
            )
            .unwrap();
            assert_eq!(selected, reports[ordinal]);
            assert_eq!(selected["format"], REPORT_FORMAT);
            assert_eq!(selected["case_id"], case_id);
            assert_eq!(
                selected["metrics"]["compared_tokens"],
                rows_for_kind(kind).unwrap()
            );
            assert!(selected.get("accepted").is_none());
        }
    }

    #[test]
    fn engineering_multilane_last_row_difference_is_diagnostic_not_acceptance() {
        let fixture = fixture_for_kind("decode-s8-c8192");
        let mut logits = fs::read(fixture.reference.join(FILES[0])).unwrap();
        let offset = usize::try_from(7 * VOCABULARY_SIZE * BF16_BYTES).unwrap();
        logits[offset..offset + 2].copy_from_slice(&0x4000_u16.to_le_bytes());
        let mut tokens = fs::read(fixture.reference.join(FILES[3])).unwrap();
        tokens[28..32].copy_from_slice(&0_u32.to_le_bytes());
        fs::write(fixture.reference.join(FILES[0]), &logits).unwrap();
        fs::write(fixture.reference.join(FILES[3]), &tokens).unwrap();
        rewrite_output(&fixture.reference.join(FILES[1]), |output| {
            output["logits"]["sha256"] = json!(sha256_identity(&logits));
            output["tokens"]["sha256"] = json!(sha256_identity(&tokens));
        });
        let selected = compare(&fixture).unwrap();
        assert_eq!(selected["metrics"]["compared_tokens"], 8);
        assert_eq!(selected["metrics"]["token_mismatches"], 1);
        let suite = compare_all(&fixture).unwrap();
        assert_eq!(suite["metrics"]["token_mismatches"], 1);
        assert!(
            suite["metrics"]["maximum_logit_ulp_error"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(suite.get("accepted").is_none());
    }

    #[test]
    fn engineering_ordered_rows_reject_missing_extra_reordered_and_substituted_hashes() {
        for failure in [
            "missing",
            "extra",
            "reordered",
            "substituted",
            "whole-payload",
            "non-string",
        ] {
            let fixture = fixture_for_kind("decode-s32-c8192");
            rewrite_runner(&fixture, |runner| {
                let whole = runner["logits_sha256"].clone();
                let rows = runner["logits_row_sha256"].as_array_mut().unwrap();
                match failure {
                    "missing" => {
                        rows.pop();
                    }
                    "extra" => rows.push(rows[0].clone()),
                    "reordered" => rows.swap(0, 31),
                    "substituted" => {
                        rows[31] = json!(sha256_identity(b"different valid row digest"))
                    }
                    "whole-payload" => rows[0] = whole,
                    "non-string" => rows[0] = json!(7),
                    _ => unreachable!(),
                }
            });
            assert!(compare(&fixture).is_err(), "accepted row failure {failure}");
            assert!(
                compare_all(&fixture).is_err(),
                "accepted suite row failure {failure}"
            );
        }
    }

    #[test]
    fn engineering_decode_requires_exact_c8192_execution_and_ordered_lane_bindings() {
        for failure in [
            "prefill",
            "short-rounds",
            "terminal-ordinal",
            "terminal-generation",
            "history",
            "context",
            "context-substitution",
            "workload",
            "missing-lane",
            "extra-lane",
            "lane-order",
            "lane-digest",
            "token-sequence",
            "lane-extra-field",
            "execution-extra-field",
        ] {
            let fixture = fixture_for_kind("decode-s8-c8192");
            rewrite_runner(&fixture, |runner| {
                let execution = &mut runner["execution"];
                match failure {
                    "prefill" => {
                        *execution = json!({"dispatch_generation": 8202, "epoch": 8208, "mode": "one-shot-prefill", "round_count": 1})
                    }
                    "short-rounds" => execution["round_count"] = json!(8191),
                    "terminal-ordinal" => execution["terminal_ordinal"] = json!(8190),
                    "terminal-generation" => {
                        execution["terminal_dispatch_generation"] = json!(8203)
                    }
                    "history" => execution["round_history_sha256"] = json!("invalid"),
                    "context" => execution["context_plan_sha256"] = json!("invalid"),
                    "context-substitution" => {
                        execution["context_plan_sha256"] =
                            json!(sha256_identity(b"different context plan"))
                    }
                    "workload" => execution["declared_workload_binding_sha256"] = json!("invalid"),
                    "missing-lane" => {
                        execution["ordered_lane_bindings"]
                            .as_array_mut()
                            .unwrap()
                            .pop();
                    }
                    "extra-lane" => {
                        let lane = execution["ordered_lane_bindings"][0].clone();
                        execution["ordered_lane_bindings"]
                            .as_array_mut()
                            .unwrap()
                            .push(lane);
                    }
                    "lane-order" => execution["ordered_lane_bindings"]
                        .as_array_mut()
                        .unwrap()
                        .swap(0, 7),
                    "lane-digest" => {
                        execution["ordered_lane_bindings"][7]["lane_identity_sha256"] =
                            json!("invalid")
                    }
                    "token-sequence" => {
                        execution["ordered_lane_bindings"][7]["token_sequence_identity_sha256"] =
                            json!("invalid")
                    }
                    "lane-extra-field" => {
                        execution["ordered_lane_bindings"][7]["extra"] = json!(true)
                    }
                    "execution-extra-field" => execution["extra"] = json!(true),
                    _ => unreachable!(),
                }
            });
            assert!(
                compare(&fixture).is_err(),
                "accepted c8192 failure {failure}"
            );
        }
    }

    #[test]
    fn engineering_suite_rejects_shape_identity_roster_alias_and_late_case_failures() {
        for failure in [
            "shape-rows",
            "shape-vocabulary",
            "payload-size",
            "last-case-identity",
            "environment",
            "plan-case-id",
            "plan-kind-id",
            "missing-capture",
            "extra-capture",
            "missing-reference",
            "extra-reference",
            "renamed-bundle",
            "cross-case-alias",
            "root-symlink",
            "bundle-symlink",
            "old-engineering-reference",
            "unmatched-runner",
        ] {
            let fixture = fixture_for_kind("prefill-s8-t128");
            let output = fixture.reference.join(FILES[1]);
            match failure {
                "shape-rows" => rewrite_output(&output, |v| v["shape"]["rows"] = json!(1)),
                "shape-vocabulary" => rewrite_output(&output, |v| {
                    v["shape"]["vocabulary_size"] = json!(VOCABULARY_SIZE - 1)
                }),
                "payload-size" => rewrite_output(&output, |v| v["tokens"]["bytes"] = json!(4)),
                "last-case-identity" => rewrite_output(&output, |v| v["case_id"] = json!(CASE_ID)),
                "environment" => rewrite_output(&output, |v| {
                    v["environment_sha256"] = json!(sha256_identity(b"another environment"))
                }),
                "plan-case-id" => rewrite_output(&fixture.plan, |v| {
                    v["cases"][6]["id"] = json!("prefill-s8-t128.002")
                }),
                "plan-kind-id" => rewrite_output(&fixture.plan, |v| {
                    let first = v["cases"][0]["id"].clone();
                    v["cases"][0]["id"] = v["cases"][6]["id"].clone();
                    v["cases"][6]["id"] = first;
                }),
                "missing-capture" => fs::remove_dir_all(&fixture.capture).unwrap(),
                "extra-capture" => fs::write(fixture.captures.join("extra"), b"extra").unwrap(),
                "missing-reference" => fs::remove_dir_all(&fixture.reference).unwrap(),
                "extra-reference" => {
                    fs::create_dir(fixture.references.join("extra.reference.bundle")).unwrap()
                }
                "renamed-bundle" => fs::rename(
                    &fixture.reference,
                    fixture.references.join("unknown.reference.bundle"),
                )
                .unwrap(),
                "cross-case-alias" => {
                    let destination = fixture.reference.join(FILES[0]);
                    fs::remove_file(&destination).unwrap();
                    fs::hard_link(
                        fixture
                            .references
                            .join("decode-s8-c8192.reference.bundle")
                            .join(FILES[0]),
                        destination,
                    )
                    .unwrap();
                }
                "root-symlink" => {
                    let moved = fixture.references.with_extension("moved");
                    fs::rename(&fixture.references, &moved).unwrap();
                    std::os::unix::fs::symlink(moved, &fixture.references).unwrap();
                }
                "bundle-symlink" => {
                    let moved = fixture.reference.with_extension("moved");
                    fs::rename(&fixture.reference, &moved).unwrap();
                    std::os::unix::fs::symlink(moved, &fixture.reference).unwrap();
                }
                "old-engineering-reference" => rewrite_output(&output, |v| {
                    v["format"] = json!("FERRIC-M1-ENGINEERING-REFERENCE-OUTPUT-V1")
                }),
                "unmatched-runner" => rewrite_output(&fixture.reference.join(FILES[2]), |v| {
                    v["dispatch_generation"] = json!(99)
                }),
                _ => unreachable!(),
            }
            assert!(
                compare_all(&fixture).is_err(),
                "accepted suite failure {failure}"
            );
        }
    }

    #[test]
    fn engineering_suite_retains_earlier_sources_and_plan_until_final_revalidation() {
        let fixture = fixture();
        let plan = HeldPlan::open(&fixture.plan).unwrap();
        let root = HeldSuiteRoot::open(&fixture.references, "reference").unwrap();
        let earlier = fixture.references.join("decode-s1-c8192.reference.bundle");
        let held = HeldBundle::open(&earlier, 1).unwrap();
        compare(&fixture).unwrap();
        let name = earlier.join(FILES[3]);
        let bytes = fs::read(&name).unwrap();
        fs::remove_file(&name).unwrap();
        fs::write(&name, bytes).unwrap();
        assert!(held.validate().is_err());
        fs::remove_file(&fixture.plan).unwrap();
        fs::write(&fixture.plan, &plan.bytes).unwrap();
        assert!(plan.validate().is_err());
        let moved = fixture.references.with_extension("moved");
        fs::rename(&fixture.references, &moved).unwrap();
        fs::create_dir(&fixture.references).unwrap();
        assert!(root.validate().is_err());
    }

    #[test]
    fn engineering_suite_rejects_last_lane_nonfinite_or_argmax_without_partial_report() {
        for nonfinite in [false, true] {
            let fixture = fixture_for_kind("prefill-s8-t128");
            let mut logits = fs::read(fixture.reference.join(FILES[0])).unwrap();
            let offset = usize::try_from(7 * VOCABULARY_SIZE * BF16_BYTES).unwrap();
            logits[offset..offset + 2]
                .copy_from_slice(&(if nonfinite { 0x7f80_u16 } else { 0x4000_u16 }).to_le_bytes());
            fs::write(fixture.reference.join(FILES[0]), &logits).unwrap();
            rewrite_output(&fixture.reference.join(FILES[1]), |output| {
                output["logits"]["sha256"] = json!(sha256_identity(&logits));
            });
            compare_selected(
                &fixture.plan,
                "decode-s1-c8192.001",
                &fixture.captures.join("decode-s1-c8192.capture.bundle"),
                &fixture.references.join("decode-s1-c8192.reference.bundle"),
            )
            .unwrap();
            assert!(compare_all(&fixture).is_err());
        }
    }
}
