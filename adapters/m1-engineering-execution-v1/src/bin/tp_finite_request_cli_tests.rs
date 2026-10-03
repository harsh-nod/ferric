fn finite_options(geometry: GraphGeometry) -> Options {
    let mut selected = dependency_options();
    if geometry == GraphGeometry::Long2304 {
        for (flag, value) in [("--context", "2304"), ("--pages", "144")] {
            let index = selected.iter().position(|item| *item == flag).unwrap();
            selected[index + 1] = value;
        }
    }
    selected.extend([
        "--runtime-tp2-queued-graph",
        "finite-request-admission-cache",
        "--runtime-cache-admission",
        "--runtime-tp2-metadata-uploads",
        "batched",
        "--tp2-graph-geometry",
        geometry.argument(),
        "--tp2-graph-kernel-profile",
        "wave-stack-norm-attention-kv-mlp",
        "--tp2-graph-argmax-artifact",
        "/v22",
        "--tp2-graph-norm-artifact",
        "/v15",
        "--max-batches",
        if geometry == GraphGeometry::Short64 {
            "36"
        } else {
            "2303"
        },
    ]);
    args(&selected).unwrap()
}

#[test]
fn finite_cli_exact_single_request_budget_precedes_all_worker_spawns() {
    for geometry in GraphGeometry::ALL {
        let options = finite_options(geometry);
        let (prompt, outputs) = if geometry == GraphGeometry::Short64 {
            (5, 32)
        } else {
            (2048, 256)
        };
        for mutation in 0..7 {
            let mut workload = Workload {
                schema: "FerricQwen3TpBatchWorkloadV1".into(),
                requests: vec![Request {
                    name: "one".into(),
                    prompt: "bound independently".into(),
                    new_tokens: outputs,
                    arrival_tick: 0,
                    cancel_tick: None,
                }],
            };
            let mut prompts = vec![vec![1; prompt]];
            match mutation {
                1 => workload.requests[0].new_tokens -= 1,
                2 => workload.requests[0].arrival_tick = 1,
                3 => workload.requests[0].cancel_tick = Some(1),
                4 => {
                    prompts[0].pop();
                }
                5 => prompts.push(vec![1]),
                6 => workload.requests.clear(),
                _ => {}
            }
            assert_eq!(
                validate_finite_workload(&options, Some(&workload), &prompts).is_ok(),
                mutation == 0
            );
        }
        assert!(validate_finite_workload(&options, None, &[]).is_err());
        let mut wrong = finite_options(geometry);
        wrong.max_batches -= 1;
        let workload = Workload {
            schema: String::new(),
            requests: vec![Request {
                name: "one".into(),
                prompt: String::new(),
                new_tokens: outputs,
                arrival_tick: 0,
                cancel_tick: None,
            }],
        };
        assert!(validate_finite_workload(&wrong, Some(&workload), &[vec![1; prompt]]).is_err());
    }
    let source = include_str!("ferric-qwen3-tp-batch-engineering.rs");
    let run = source.split("fn run_with_timing(").nth(1).unwrap();
    let gate = run.find("validate_finite_workload(options,").unwrap();
    let spawns = run
        .match_indices("::spawn")
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert!(!spawns.is_empty() && spawns.into_iter().all(|spawn| gate < spawn));
}

fn finite_admission(forwards: u64) -> serde_json::Value {
    let fixture: serde_json::Value =
        serde_json::from_slice(include_bytes!("finite-request-2048-fixture.json")).unwrap();
    assert_eq!(fixture["input_token_ids"].as_array().unwrap().len(), 2048);
    let ids = if forwards == 36 {
        serde_json::json!([1, 2, 3, 4, 5])
    } else {
        fixture["input_token_ids"].clone()
    };
    serde_json::json!({"schema":"FerricQwen3TpBatchAdmissionV2", "prompt_tokens":ids})
}

fn finite_batch() -> serde_json::Value {
    serde_json::json!({"schema":"FerricQwen3TpBatchCompletedV2"})
}

#[test]
fn finite_buffered_metrics_cannot_be_confused_with_streaming_acceptance() {
    let mut records = FiniteRecords::new(36);
    records.push(finite_admission(36)).unwrap();
    for _ in 0..36 {
        records.push(finite_batch()).unwrap();
    }
    records
        .push(serde_json::json!({"schema":"FerricQwen3TpBatchRequestV2",
        "ttft_ns":10,"tpot_ns":2,"decode_intervals_ns":[2],"output_timestamps_ns":[10,12]}))
        .unwrap();
    let record = records.values.last().unwrap();
    assert_eq!(record["schema"], "FerricQwen3TpFiniteBufferedRequestV1");
    assert_eq!(
        record["metric_basis"],
        "internal-generation-nonstreaming-unaccepted-until-clean-close"
    );
    for field in [
        "ttft_ns",
        "tpot_ns",
        "decode_intervals_ns",
        "output_timestamps_ns",
    ] {
        assert!(record.get(field).is_none());
        assert!(record.get(format!("internal_{field}")).is_some());
    }
    assert!(
        records
            .push(serde_json::json!({"schema":"FerricQwen3TpBatchRequestV2"}),)
            .is_err()
    );
    let accepted = finite_acceptance_record(&records.finish().unwrap(), 100, 200, 3, 303);
    assert_eq!(accepted["client_visible_streaming_qualified"], false);
    assert_eq!(accepted["accepted_after_clean_close"], true);
    assert_eq!(accepted["setup_elapsed_ns"], 100);
    assert_eq!(accepted["request_elapsed_ns"], 200);
    assert_eq!(accepted["close_elapsed_ns"], 3);
    assert_eq!(accepted["acceptance_elapsed_ns"], 303);
    let source = include_str!("ferric-qwen3-tp-batch-engineering.rs");
    let run = source.split("fn run_with_timing(").nth(1).unwrap();
    assert!(
        run.find("runtime.close()").unwrap() < run.find("publish_finite_after_close(").unwrap()
    );
    for work_ok in [false, true] {
        for close_ok in [false, true] {
            let work = if work_ok {
                Ok(())
            } else {
                Err("native request incomplete".into())
            };
            let close = if close_ok {
                Ok(())
            } else {
                Err("close/EOF/reap failed".into())
            };
            let mut calls = 0;
            publish_finite_after_close(&work, &close, &accepted, |_| {
                calls += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(calls, usize::from(work_ok && close_ok));
        }
    }
}

#[test]
fn finite_real2048_admission_record_bounds_order_and_exact_budget() {
    let admission = finite_admission(2303);
    assert!(serde_json::to_vec(&admission).unwrap().len() > 4096);
    let request = serde_json::json!({"schema":"FerricQwen3TpBatchRequestV2"});
    let unknown = serde_json::json!({"schema":"unexpected"});
    let mut records = FiniteRecords::new(2303);
    assert!(records.push(finite_batch()).is_err());
    assert!(records.push(request.clone()).is_err());
    assert!(records.push(unknown.clone()).is_err());
    records.push(admission.clone()).unwrap();
    assert!(records.push(admission.clone()).is_err());
    assert!(records.push(request.clone()).is_err());
    for _ in 0..2303 {
        records.push(finite_batch()).unwrap();
    }
    assert!(records.push(finite_batch()).is_err());
    records.push(request.clone()).unwrap();
    for wrong in [admission.clone(), request.clone(), finite_batch(), unknown] {
        assert!(records.push(wrong).is_err());
    }
    assert!(records.bytes < 10 << 20);
    assert_eq!(records.finish().unwrap().len(), 2305);
    for (mut oversized, initial) in [(admission.clone(), false), (request, true)] {
        oversized["oversized"] = serde_json::json!("x".repeat(65_536));
        let mut records = FiniteRecords::new(2303);
        if initial {
            records.push(admission.clone()).unwrap();
            for _ in 0..2303 {
                records.push(finite_batch()).unwrap();
            }
        }
        let before = (records.values.len(), records.bytes, records.batches);
        assert!(records.push(oversized).is_err());
        assert_eq!(
            (records.values.len(), records.bytes, records.batches),
            before
        );
    }
    let mut records = FiniteRecords::new(2303);
    records.push(admission.clone()).unwrap();
    let mut batch = finite_batch();
    batch["oversized"] = serde_json::json!("x".repeat(4096));
    assert!(records.push(batch).is_err());
    records.bytes = 10 << 20;
    assert!(records.push(finite_batch()).is_err());
    assert!(records.finish().is_err());
    assert!(FiniteRecords::new(0).push(admission).is_err());
}

#[test]
fn finite_setup_declares_non_equivalent_observation_and_provisional_outputs() {
    let mut setup = serde_json::json!({"runtime_tp2_prepared":{},"performance_profile":{}});
    record_closed_token_policy(&mut setup, GraphPolicy::FiniteRequestAdmissionCache);
    let prepared = &setup["runtime_tp2_prepared"];
    assert_eq!(prepared["observation_policy"], "FiniteRequestV1");
    assert_eq!(prepared["provisional_outputs"], true);
    assert_eq!(prepared["transient_currentness_equivalence_claim"], false);
}
