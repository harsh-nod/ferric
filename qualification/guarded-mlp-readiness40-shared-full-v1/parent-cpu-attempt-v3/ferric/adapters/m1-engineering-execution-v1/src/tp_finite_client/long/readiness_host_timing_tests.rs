use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn trace(count: usize) -> Trace {
    let mut value = Trace::new();
    for i in 0..count {
        value
            .at(expected_event(i).unwrap(), (i as u64 + 1) * 10)
            .unwrap();
    }
    value
}
fn report(path: PathBuf, original: &[u8]) -> Report {
    Report {
        schema: SCHEMA.into(),
        ordinary_complete: FilePin {
            path,
            bytes: original.len() as u64,
            sha256: hash(original),
        },
        profile_sha256: [1; 32],
        transcript_sha256: [2; 32],
        completed_forwards: 40,
        generated_tokens: 0,
        capture_positions: [0, 5, 16, 39],
        timeline: trace(EVENTS).finish().unwrap(),
        native_closed: true,
        child_exit_zero: true,
        process_group_absent: true,
        parent_host_measurement: true,
        gpu_timing: false,
        nested_control_timers_included: false,
        sidecar_publication_timed: false,
        full_long_workload: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferric-readiness-host-timing-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn readiness_host_timing_reconciles_exact_forty_disjoint_rows() {
    let value = trace(EVENTS).finish().unwrap();
    value.validate().unwrap();
    assert_eq!(value.total_ns, 1240);
    assert_eq!(value.source_preparation.elapsed_ns, 10);
    assert_eq!(value.spawn_to_setup_seal.elapsed_ns, 10);
    assert_eq!(value.close_and_retirement.elapsed_ns, 10);
    assert_eq!(value.postcheck_and_ordinary_publication.elapsed_ns, 10);
    assert_eq!(value.forwards.len(), 40);
    for (i, row) in value.forwards.iter().enumerate() {
        assert_eq!(
            (row.position, row.generation, row.elapsed_ns),
            (i as u32, i as u64 + 1, 30)
        );
        assert_eq!(row.flush_to_frame_read.elapsed_ns, 10);
        assert_eq!(row.validate_retain_commit.elapsed_ns, 10);
    }
    let encoded = serde_json::to_vec(&value).unwrap();
    assert_eq!(serde_json::from_slice::<Timeline>(&encoded).unwrap(), value);
}

#[test]
fn readiness_host_timing_refuses_incomplete_duplicate_and_failed_prefixes() {
    for n in 0..EVENTS {
        assert!(trace(n).finish().is_err(), "prefix {n}");
    }
    let mut duplicate = trace(5);
    assert!(duplicate.at(Event::Committed(0), 60).is_err());
    assert!(duplicate.at(Event::Flushed(1), 70).is_err());
    assert!(duplicate.finish().is_err());
    let mut late = trace(EVENTS);
    assert!(late.at(Event::Finished, 2000).is_err());
    assert!(late.finish().is_err());
}

#[test]
fn readiness_host_timing_clock_regression_is_terminal_and_zero_spans_are_valid() {
    assert!(Span::new(2, 1).is_err());
    let mut value = trace(2);
    assert!(value.at(Event::Flushed(0), 19).is_err());
    assert!(value.at(Event::Flushed(0), 21).is_err());
    assert!(value.finish().is_err());
    let mut zero = Trace::new();
    for i in 0..EVENTS {
        zero.at(expected_event(i).unwrap(), 0).unwrap();
    }
    assert_eq!(zero.finish().unwrap().total_ns, 0);
}

#[test]
fn readiness_host_timing_rejects_wrong_rows_overlap_gap_and_totals() {
    let original = trace(EVENTS).finish().unwrap();
    for change in 0..8 {
        let mut value = original.clone();
        match change {
            0 => {
                value.forwards.pop();
            }
            1 => value.forwards[1].position = 0,
            2 => value.forwards[2].generation = 2,
            3 => value.forwards[0].flush_to_frame_read.start_ns -= 1,
            4 => value.forwards[0].validate_retain_commit.start_ns += 1,
            5 => value.forwards[0].elapsed_ns += 1,
            6 => value.total_ns += 1,
            _ => value.spawn_to_setup_seal.elapsed_ns = u64::MAX,
        }
        assert!(value.validate().is_err(), "mutation {change}");
    }
}

#[test]
fn readiness_host_timing_rejects_negative_and_nested_timer_fields() {
    assert!(serde_json::from_str::<Span>(r#"{"start_ns":0,"end_ns":-1,"elapsed_ns":1}"#).is_err());
    let value = trace(EVENTS).finish().unwrap();
    let mut json = serde_json::to_value(&value).unwrap();
    json["segment_host_ns"] = serde_json::json!(5000);
    assert!(serde_json::from_value::<Timeline>(json).is_err());
    let mut json = serde_json::to_value(&value).unwrap();
    json["forwards"][0]["prefix_host_ns"] = serde_json::json!(5000);
    assert!(serde_json::from_value::<Timeline>(json).is_err());
}

#[test]
fn readiness_host_timing_keeps_existing_retention_cap_and_overflow_checks() {
    let reserve = 512 << 10;
    let limit = long::EVIDENCE_BYTES as u64;
    assert_eq!(
        retained_total(limit - reserve - FILE_BYTES, FILE_BYTES, reserve).unwrap(),
        limit - reserve
    );
    assert!(retained_total(limit - reserve - FILE_BYTES + 1, FILE_BYTES, reserve).is_err());
    assert!(retained_total(1, FILE_BYTES + 1, reserve).is_err());
    assert!(retained_total(1, 0, reserve).is_err());
    assert!(retained_total(u64::MAX, 1, reserve).is_err());
    assert!(retained_total(1, 1, u64::MAX).is_err());
}

#[test]
fn readiness_host_timing_requires_healthy_close_and_no_authority() {
    for change in 0..10 {
        let mut value = report(PathBuf::from("/task/complete.json"), b"{}\n");
        match change {
            0 => value.native_closed = false,
            1 => value.child_exit_zero = false,
            2 => value.process_group_absent = false,
            3 => value.generated_tokens = 1,
            4 => value.capture_positions[1] = 15,
            5 => value.gpu_timing = true,
            6 => value.nested_control_timers_included = true,
            7 => value.numerical_acceptance = true,
            8 => value.performance_claim = true,
            _ => value.full_long_workload = true,
        }
        assert!(value.validate().is_err(), "mutation {change}");
    }
}

#[test]
fn readiness_host_timing_file_publication_is_pinned_exclusive_and_bounded() {
    let temp = Temp::new();
    let original = b"synthetic ordinary completion\n";
    let path = temp.0.join("complete.json");
    std::fs::write(&path, original).unwrap();
    let value = report(path.clone(), original);
    let status = publish(&value, original.len() as u64, 512 << 10).unwrap();
    assert!(status.complete && status.parent_host_measurement);
    assert!(!status.gpu_timing && !status.numerical_acceptance && !status.performance_claim);
    assert_eq!(
        status.retained_bytes_with_timing,
        status.ordinary_retained_bytes + status.file.bytes
    );
    let raw = status.file.read(FILE_BYTES, true).unwrap();
    let parsed: Report = serde_json::from_slice(&raw).unwrap();
    parsed.validate().unwrap();
    assert_eq!(parsed.timeline, value.timeline);
    assert_eq!(std::fs::read(path).unwrap(), original);
    assert!(!temp.0.join("host-timing.pending").exists());
    assert!(publish(&value, original.len() as u64, 512 << 10).is_err());
    assert_eq!(status.file.read(FILE_BYTES, true).unwrap(), raw);
}

#[test]
fn readiness_host_timing_file_publication_refuses_changed_missing_or_unclosed_inputs() {
    for mode in 0..3 {
        let temp = Temp::new();
        let path = temp.0.join("complete.json");
        let mut value = report(path.clone(), b"original\n");
        match mode {
            0 => std::fs::write(&path, b"modified\n").unwrap(),
            1 => {}
            _ => {
                std::fs::write(&path, b"original\n").unwrap();
                value.native_closed = false;
            }
        }
        assert!(publish(&value, 9, 512 << 10).is_err());
        assert!(!temp.0.join("host-timing.pending").exists());
        assert!(!temp.0.join("host-timing.json").exists());
    }
}

#[test]
fn readiness_host_timing_preserves_default_entry_signatures_and_disabled_recorder() {
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run;
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run_position5;
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run_causal;
    let mut disabled = None;
    mark(&mut disabled, Event::Finished).unwrap();
    assert!(disabled.is_none());
    admit_entry(super::super::POSITION5_REQUEST_SCHEMA, true).unwrap();
    assert!(admit_entry(super::super::POSITION5_REQUEST_SCHEMA, false).is_err());
    for schema in [
        super::super::REQUEST_SCHEMA,
        super::super::CAUSAL_REQUEST_SCHEMA,
        "FerricGuardedMlpFull2303RequestV1",
        "",
    ] {
        assert!(admit_entry(schema, true).is_err());
    }
}

#[test]
fn readiness_shared_full_host_timing_keeps_all_existing_entry_signatures() {
    let _: fn(ReadinessConfig, bool) -> Result<SharedFullTimedObservation> =
        super::super::run_position5_shared_full_host_timing;
    let _: fn(ReadinessConfig, bool) -> Result<TimedObservation> =
        super::super::run_position5_host_timing;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::SharedFullObservation> =
        super::super::run_position5_shared_full;
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run_position5;
    assert_ne!(SHARED_WRAPPER_SCHEMA, WRAPPER_SCHEMA);
    assert_ne!(SHARED_WRAPPER_SCHEMA, super::super::shared_full::SCHEMA);
    assert_eq!(EVENTS, 124);
    assert_eq!((FILE_BYTES, SUMMARY_BYTES), (64 << 10, 128 << 10));
    let mut disabled = None;
    mark(&mut disabled, Event::Finished).unwrap();
    assert!(disabled.is_none());
}

#[test]
fn readiness_shared_full_host_timing_refuses_mode_and_opt_in_before_io() {
    let pin = FilePin {
        path: PathBuf::from("relative-and-not-opened"),
        bytes: 0,
        sha256: [0; 32],
    };
    // Deliberately invalid sources distinguish entry refusal from later source admission.
    let config = ReadinessConfig {
        schema: super::super::POSITION5_REQUEST_SCHEMA.into(),
        base: super::super::super::Config {
            schema: String::new(),
            source: pin.path.clone(),
            worker: pin.clone(),
            images: super::super::super::ImagePins {
                prefix: pin.clone(),
                mlp: pin.clone(),
                residual: pin.clone(),
                tail: pin.clone(),
            },
            expected_bundle_id: [0; 32],
            expected_model_id: [0; 32],
            device_ids: [0; 2],
            session: [0; 32],
            prompt: super::super::super::PromptPins {
                manifest: pin.clone(),
                text: pin.clone(),
                tokens: pin.clone(),
            },
            evidence_directory: pin.path.clone(),
            dispatch_timeout_ms: 0,
            child_deadline_ms: 0,
        },
        tiles_image: pin.clone(),
        prefix_image: pin.clone(),
        projection_image: pin.clone(),
        guarded_image: pin,
    };
    let expected = "host timing explicit Position5 entry and machine-code opt-in";
    assert_eq!(
        run_position5_shared_full_host_timing(config.clone(), false)
            .err()
            .unwrap(),
        expected,
    );
    for schema in [
        super::super::REQUEST_SCHEMA,
        super::super::CAUSAL_REQUEST_SCHEMA,
        "FerricGuardedMlpFull2303RequestV1",
        "",
    ] {
        let mut bad = config.clone();
        bad.schema = schema.into();
        assert_eq!(
            run_position5_shared_full_host_timing(bad, true)
                .err()
                .unwrap(),
            expected,
        );
    }
    admit_entry(&config.schema, true).unwrap();
    super::super::shared_full::admit_entry(&config.schema, true).unwrap();
    // The actual combined entry must admit Some(Recorder) and reach pure source validation.
    assert_eq!(
        run_position5_shared_full_host_timing(config.clone(), true)
            .err()
            .unwrap(),
        "long profile requires the exact retained raw 2048-token prompt",
    );
    for (profile, causal) in [
        (long::Profile::Readiness40Position5, true),
        (long::Profile::Readiness40, false),
        (long::Profile::Full2303, false),
    ] {
        let mut timing = Some(Recorder::new());
        assert_eq!(
            super::super::run_inner_policy(
                config.clone(),
                true,
                profile,
                causal,
                &mut timing,
                true
            )
            .err()
            .unwrap(),
            "shared full cannot combine causal or another profile",
        );
    }
}
