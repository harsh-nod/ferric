//! Selected engineering diagnostics, deliberately outside qualification ingestion.

use super::{
    admit_input_identity, compare_pair, directory_roster, duplicate_secure_input_directory,
    encode_canonical_document, exact_plan_cases, expect, field, fstat, identity, json,
    load_benchmark_plan, object, open_pairs_parent, parse_output, parse_output_for_purpose,
    plan_identities, require_sha256, sha256_identity, string, unsigned, validate_capture_execution,
    BTreeMap, BTreeSet, BenchResult, OsString, Output, OutputContext, OutputPurpose, OwnedFd, Pair,
    Path, PathBuf, PlanCase, SecureInputDirectory, SecureInputFile, TranscriptBinding, Value,
    Write, BF16_BYTES, SUITE, TARGET, TOKEN_BYTES, VOCABULARY_SIZE,
};

pub(super) const CASE_ID: &str = "prefill-s1-t128.001";
const KIND: &str = "prefill-s1-t128";
pub(super) const REFERENCE_FORMAT: &str = "FERRIC-M1-ENGINEERING-REFERENCE-OUTPUT-V1";
pub(super) const REFERENCE_NONCLAIM: &str = "Selected-case engineering reference bytes only. Authority is none; this is not a qualification result, reviewed tolerance, benchmark, protected publication, full seven-case R29 comparison, or M1 gate closure.";
const ENGINEERING_CAPTURE_FORMAT: &str = "FERRIC-M1-TECHNICAL-PREQUALIFICATION-CAPTURE-V1";
const ENGINEERING_CAPTURE_NONCLAIM: &str = "Authority-none aggregate engineering observation only. This transcript authenticates no compiler origin or Worker V3 publication, selects no current protected publication, establishes no reference comparison, tolerance, numerical correctness, hardware correctness, performance, qualification, or m1.r29 closure.";
const REPORT_FORMAT: &str = "FERRIC-M1-ENGINEERING-SELECTED-COMPARISON-V1";
const REPORT_NONCLAIM: &str = "Numerical diagnostics for one engineering case only. Authority is none; no threshold acceptance, benchmark, protected publication, full seven-case R29 comparison, qualification, or M1 gate closure is produced.";
const FILES: [&str; 4] = [
    "logits.bf16le",
    "output.json",
    "runner.json",
    "tokens.u32le",
];

pub(super) fn require_selected_case(case_id: &str, kind: &str) -> BenchResult<()> {
    if case_id != CASE_ID || kind != KIND {
        return Err("engineering comparison supports only prefill-s1-t128.001".to_owned());
    }
    Ok(())
}

pub(super) fn command(arguments: &[OsString]) -> BenchResult<()> {
    let [command, plan, case, capture, reference] = arguments else {
        return Err("usage: ferric-m1-differential compare-engineering-selected PLAN CASE-ID CAPTURE-BUNDLE REFERENCE-BUNDLE".to_owned());
    };
    if command != "compare-engineering-selected" || case != CASE_ID {
        return Err("engineering selected command or case drifted".to_owned());
    }
    let report = compare_selected(Path::new(plan), Path::new(capture), Path::new(reference))?;
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
    fn open(path: &Path) -> BenchResult<Self> {
        let descriptor = open_pairs_parent(path)?;
        require_bundle_roster(&descriptor)?;
        let root = duplicate_secure_input_directory(&descriptor, "engineering bundle")?;
        let logits = root.open_exact(
            Path::new(FILES[0]),
            VOCABULARY_SIZE * BF16_BYTES,
            "engineering logits",
        )?;
        let (_, manifest_bytes, manifest) =
            root.read_canonical_held(Path::new(FILES[1]), "engineering output manifest")?;
        let (runner, runner_bytes, transcript) =
            root.read_canonical_held(Path::new(FILES[2]), "engineering runner transcript")?;
        let tokens = root.open_exact(Path::new(FILES[3]), TOKEN_BYTES, "engineering token")?;
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
}

fn require_bundle_roster(descriptor: &OwnedFd) -> BenchResult<()> {
    if directory_roster(descriptor, "engineering selected bundle")?
        != FILES.into_iter().map(str::to_owned).collect()
    {
        return Err("engineering selected bundle file roster drifted".to_owned());
    }
    Ok(())
}

fn compare_selected(
    plan_path: &Path,
    capture_path: &Path,
    reference_path: &Path,
) -> BenchResult<Value> {
    let plan_parent = open_pairs_parent(plan_path.parent().unwrap_or_else(|| Path::new(".")))?;
    let plan_root = duplicate_secure_input_directory(&plan_parent, "engineering plan parent")?;
    let plan_name = Path::new(
        plan_path
            .file_name()
            .ok_or_else(|| "plan path has no filename".to_owned())?,
    );
    let (_, held_plan_bytes, held_plan) =
        plan_root.read_canonical_held(plan_name, "engineering plan")?;
    let (plan, plan_bytes) = load_benchmark_plan(&SUITE, plan_path)?;
    if held_plan_bytes != plan_bytes {
        return Err("engineering plan changed during admission".to_owned());
    }
    let plan_sha256 = sha256_identity(&plan_bytes);
    let cases = exact_plan_cases(&plan)?;
    let case = cases
        .get(CASE_ID)
        .ok_or_else(|| "plan lacks selected engineering case".to_owned())?;
    require_selected_case(CASE_ID, &case.kind)?;
    let identities = plan_identities(&plan)?;
    let capture = HeldBundle::open(capture_path)?;
    let reference = HeldBundle::open(reference_path)?;
    let mut seen = BTreeSet::new();
    admit_input_identity(&mut seen, held_plan.identity(), "engineering plan")?;
    for bundle in [&capture, &reference] {
        for held in &bundle.files {
            admit_input_identity(&mut seen, held.identity(), "engineering bundle input")?;
        }
    }
    if capture.runner_bytes != reference.runner_bytes {
        return Err("engineering reference retained a different capture transcript".to_owned());
    }
    let transcript =
        validate_engineering_capture(&capture.runner, case, &identities, &plan_sha256)?;
    let runner_sha256 = sha256_identity(&capture.runner_bytes);
    let context = OutputContext {
        case_id: CASE_ID,
        case,
        identities: &identities,
        plan_sha256: &plan_sha256,
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
        case_id: CASE_ID.to_owned(),
        kind: KIND.to_owned(),
        ferric,
        reference: reference_output,
        runner_transcript_sha256: runner_sha256,
    };
    let comparison = compare_pair(&mut pair)?;
    capture.validate()?;
    reference.validate()?;
    plan_root.validate_binding(plan_name, &held_plan, "engineering plan")?;
    let (_, final_plan_bytes) = load_benchmark_plan(&SUITE, plan_path)?;
    if final_plan_bytes != plan_bytes {
        return Err("engineering plan changed during comparison".to_owned());
    }
    Ok(json!({
        "authority": "none",
        "case_id": CASE_ID,
        "case_scope": "selected-case-only",
        "ferric_output_sha256": pair.ferric.manifest_sha256,
        "format": REPORT_FORMAT,
        "identities": identities,
        "kind": KIND,
        "metrics": {
            "compared_logits": comparison.compared_logits,
            "compared_tokens": comparison.compared_tokens,
            "maximum_logit_ulp_error": comparison.maximum_logit_ulp_error,
            "token_mismatches": comparison.token_mismatches,
        },
        "nonclaim": REPORT_NONCLAIM,
        "plan_sha256": plan_sha256,
        "qualification": false,
        "reference_output_sha256": pair.reference.manifest_sha256,
        "runner_transcript_sha256": pair.runner_transcript_sha256,
        "target": TARGET,
    }))
}

fn validate_engineering_capture(
    value: &Value,
    case: &PlanCase,
    identities: &BTreeMap<String, String>,
    plan_sha256: &str,
) -> BenchResult<TranscriptBinding> {
    require_selected_case(CASE_ID, &case.kind)?;
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
        ("case_id", CASE_ID),
        ("environment_sha256", identity(identities, "environment")?),
        ("format", ENGINEERING_CAPTURE_FORMAT),
        ("input_sha256", &case.input_sha256),
        ("kind", KIND),
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
        KIND,
        "prefill",
        1,
        generation,
    )?;
    let selection = object(
        field(transcript, "selection", "engineering capture transcript")?,
        &["bucket", "mode", "role"],
        "engineering capture selection",
    )?;
    for (key, expected) in [("bucket", KIND), ("mode", "prefill"), ("role", "target-8b")] {
        expect(selection, key, expected, "engineering capture selection")?;
    }
    let logits_sha256 = string(
        transcript,
        "logits_sha256",
        "engineering capture transcript",
    )?;
    if field(
        transcript,
        "logits_row_sha256",
        "engineering capture transcript",
    )? != &json!([logits_sha256])
    {
        return Err(
            "engineering selected row identity differs from the full single-row payload".to_owned(),
        );
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

    struct Fixture {
        _temporary: super::super::tests::TestDirectory,
        plan: PathBuf,
        capture: PathBuf,
        reference: PathBuf,
    }

    fn fixture() -> Fixture {
        let all = super::super::tests::pairs_fixture();
        let capture = all.captures.join(format!("{KIND}.capture.bundle"));
        let reference = all.references.join(format!("{KIND}.reference.bundle"));
        let mut plan: Value = serde_json::from_slice(&fs::read(&all.plan).unwrap()).unwrap();
        let environment = encode_canonical_document(&json!({
            "format": "FERRIC-M1-QUALIFICATION-ENVIRONMENT-V1", "gpu_unique_id": 23, "target": TARGET,
        })).unwrap();
        plan["identities"]["environment"] = json!(sha256_identity(&environment));
        let plan_bytes = encode_canonical_document(&plan).unwrap();
        fs::write(&all.plan, &plan_bytes).unwrap();
        let plan_sha256 = sha256_identity(&plan_bytes);
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
        let runner_bytes = encode_canonical_document(&runner).unwrap();
        for bundle in [&capture, &reference] {
            fs::write(bundle.join("runner.json"), &runner_bytes).unwrap();
            let mut manifest: Value =
                serde_json::from_slice(&fs::read(bundle.join("output.json")).unwrap()).unwrap();
            manifest["plan_sha256"] = json!(plan_sha256);
            manifest["environment_sha256"] = plan["identities"]["environment"].clone();
            manifest["runner_transcript_sha256"] = json!(sha256_identity(&runner_bytes));
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
        Fixture {
            _temporary: all.temporary,
            plan: all.plan,
            capture,
            reference,
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
        compare_selected(&fixture.plan, &fixture.capture, &fixture.reference)
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
        logits[2..4].copy_from_slice(&0x3f80_u16.to_le_bytes());
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
        let held = HeldBundle::open(&fixture.reference).unwrap();
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
        assert!(require_selected_case("prefill-s1-t512.001", "prefill-s1-t512").is_err());
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
                            0x3f80_u16
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
        let held = HeldBundle::open(&fixture.reference).unwrap();
        let path = fixture.reference.join("tokens.u32le");
        fs::remove_file(&path).unwrap();
        fs::write(&path, [0_u8; 4]).unwrap();
        assert!(held.validate().is_err());
        let next = self::fixture();
        let held = HeldBundle::open(&next.reference).unwrap();
        let moved = next.reference.with_extension("moved");
        fs::rename(&next.reference, &moved).unwrap();
        fs::create_dir(&next.reference).unwrap();
        assert!(held.validate().is_err());
    }
}
