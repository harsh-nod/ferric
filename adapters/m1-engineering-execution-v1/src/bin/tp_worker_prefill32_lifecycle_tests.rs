//! Protocol lifecycle fixtures only; actual-image tests own graph and ABI checks.
use super::*;

fn submit_width(worker: &mut Worker, rows: u32) -> TpResult<()> {
    let (plan, shape) = match rows {
        16 => (prefill_fixture(), Shape::Prefill16),
        32 => (prefill32_fixture(), Shape::Prefill32),
        _ => panic!("closed fixture width"),
    };
    let immutable = plan.immutable()?;
    worker.submit_shape(plan, immutable, shape)
}

#[test]
fn width_programs_reuse_within_phase_and_release_across_legacy_decode() {
    for rows in [16, 32] {
        let (mut worker, path) = worker("normal");
        select_native_width(&mut worker, rows);
        let packets = if rows == 32 { 649 } else { 613 };
        for _ in 0..2 {
            submit_width(&mut worker, rows).unwrap();
            worker.wait_fixed_token(packets).unwrap();
        }
        submit(&mut worker, fixture()).unwrap();
        worker.wait_fixed_token(652).unwrap();
        submit_width(&mut worker, rows).unwrap();
        worker.wait_fixed_token(packets).unwrap();
        let state = worker.token_program.as_ref().unwrap();
        assert_eq!(
            (state.completed_prefills, state.completed_executions),
            (3, 1)
        );
        assert_eq!((state.registrations, state.releases), (3, 2));
        assert_eq!(
            worker.queue_packets,
            u64::try_from(3 * packets + 652).unwrap()
        );
        worker.close().unwrap();
        let register = if rows == 32 {
            "register_token_program_slots512_v1"
        } else {
            "register_token_program"
        };
        let execute = if rows == 32 {
            "execute_token_program_slots512_v1"
        } else {
            "execute_token_program"
        };
        assert_eq!(
            std::fs::read_to_string(&path)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            [
                register,
                execute,
                execute,
                "release_token_program",
                "register_token_program",
                "execute_token_program",
                "release_token_program",
                register,
                execute,
                "release_token_program",
                "close",
            ]
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn width_32_never_falls_back_to_a_default_or_legacy_selected_worker() {
    for selection in 0..4 {
        let (mut worker, path) = worker("normal");
        match selection {
            0 => {}
            1 => select_native_prefill(&mut worker),
            2 => select_native_width(&mut worker, 16),
            3 => select_native_width(&mut worker, 32),
            _ => unreachable!(),
        }
        assert!(submit_width(&mut worker, if selection == 3 { 16 } else { 32 }).is_err());
        assert!(worker.failed && worker.exited);
        assert_eq!(worker.queue_packets, 0);
        assert!(std::fs::read_to_string(&path).unwrap().is_empty());
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn registered_width_32_rejects_pending_immutable_and_epoch_drift_without_ipc() {
    for mutation in 0..7 {
        let (mut worker, path) = worker("normal");
        select_native_width(&mut worker, 32);
        submit_width(&mut worker, 32).unwrap();
        worker.wait_fixed_token(649).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        let mut plan = prefill32_fixture();
        match mutation {
            0 => {
                worker.pending = Some(PendingRequest::TokenExecute {
                    program: 1,
                    epoch: 0,
                    next: 1298,
                })
            }
            1 => {
                plan.definition.slots.pop();
            }
            2 => {
                plan.updates.pop();
            }
            3 => plan.definition.dispatches[648].grid[0] += 64,
            4 => *plan.kernargs.last_mut().unwrap() = 1,
            5 => {
                let Slot::ScalarU32 { maximum, .. } = &mut plan.definition.slots[395] else {
                    panic!("scalar fixture")
                };
                *maximum -= 1;
            }
            6 => worker.queue_epoch += 1,
            _ => unreachable!(),
        }
        let immutable = plan.immutable().unwrap();
        assert!(
            worker
                .submit_shape(plan, immutable, Shape::Prefill32)
                .is_err(),
            "{mutation}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "{mutation}"
        );
        assert_eq!(worker.queue_packets, 649);
        assert!(worker.failed && worker.exited);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn extended_and_legacy_execute_families_cannot_cross_registered_shapes() {
    for extended in [false, true] {
        let (mut worker, path) = worker("normal");
        select_native_width(&mut worker, 32);
        if extended {
            submit_width(&mut worker, 32).unwrap();
            worker.wait_fixed_token(649).unwrap();
        } else {
            submit(&mut worker, fixture()).unwrap();
            worker.wait_fixed_token(652).unwrap();
        }
        let registered = worker
            .token_program
            .as_ref()
            .unwrap()
            .registered
            .as_ref()
            .unwrap();
        let updates = vec![Update::ScalarU32 { value: 1 }; if extended { 396 } else { 180 }];
        let command = if extended {
            CommandV1::ExecuteTokenProgram {
                program: registered.program,
                expected_epoch: registered.epoch,
                expected_completed_packets: worker.queue_packets,
                timeout_ms: DISPATCH_TIMEOUT_MS,
                updates,
            }
        } else {
            CommandV1::ExecuteTokenProgramSlots512V1 {
                program: registered.program,
                expected_epoch: registered.epoch,
                expected_completed_packets: worker.queue_packets,
                timeout_ms: DISPATCH_TIMEOUT_MS,
                updates,
            }
        };
        let before = std::fs::read_to_string(&path).unwrap();
        assert!(worker.send(command, Vec::new()).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        assert!(worker.failed && worker.exited);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn failed_legacy_release_never_registers_the_next_extended_prefill() {
    let (mut worker, path) = worker("release");
    select_native_width(&mut worker, 32);
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(652).unwrap();
    assert!(submit_width(&mut worker, 32).is_err());
    assert_eq!(worker.queue_packets, 652);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "register_token_program\nexecute_token_program\nrelease_token_program\n"
    );
    assert!(worker.failed && worker.exited);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn invalid_complete_graph_entry_preserves_registered_decode_before_any_release() {
    let (mut worker, path) = worker("normal");
    select_native_width(&mut worker, 32);
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(652).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    assert!(worker.submit_fixed_prefill(&[]).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    assert_eq!(worker.queue_packets, 652);
    assert!(worker.failed && worker.exited);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn width_program_counter_request_totals_exclude_three_head_singletons() {
    for (rows, prefills, packets, slots, executions, dispatches) in [
        (16, 8, 613, 216, 135, 87_708),
        (32, 4, 649, 396, 131, 85_400),
    ] {
        let (mut worker, path) = worker("counter_prefill_width");
        select_native_width(&mut worker, rows);
        worker.options.profile = true;
        worker.options.ordered64_runtime_counters = true;
        let initial = worker.token_program_counter_snapshot().unwrap();
        assert_eq!(initial["schema"], "FerricPrefillWidthProgramCountersV1");
        assert_eq!(initial["backend"], "native-whole-program-slots512-v1");
        assert_eq!(initial["counters"]["executions"], 0);
        assert_eq!(initial["program_phases"]["prefill"]["rows"], rows);
        for _ in 0..prefills {
            submit_width(&mut worker, rows).unwrap();
            worker.wait_fixed_token(packets).unwrap();
        }
        for _ in 0..127 {
            submit(&mut worker, fixture()).unwrap();
            worker.wait_fixed_token(652).unwrap();
        }
        worker.release_registered_token().unwrap();
        let final_snapshot = worker.token_program_counter_snapshot().unwrap();
        let counters = &final_snapshot["counters"];
        assert_eq!(counters["executions"], executions);
        assert_eq!(counters["dispatches"], dispatches);
        assert_eq!(counters["publications"], executions);
        assert_eq!(counters["final_waits"], executions);
        assert_eq!(counters["retirement_signals"], dispatches);
        let phases = &final_snapshot["program_phases"];
        assert_eq!(phases["prefill"]["rows"], rows);
        assert_eq!(phases["prefill"]["executions"], prefills);
        assert_eq!(phases["prefill"]["dispatches_per_execution"], packets);
        assert_eq!(phases["prefill"]["dynamic_slots"], slots);
        assert_eq!(
            phases["prefill"]["command_family"],
            if rows == 32 {
                "slots512-v1"
            } else {
                "legacy256-v1"
            }
        );
        assert_eq!(phases["decode_c1"]["executions"], 127);
        assert_eq!(phases["decode_c1"]["dispatches_per_execution"], 652);
        assert_eq!(phases["decode_c1"]["dynamic_slots"], 180);
        assert_eq!(phases["decode_c1"]["command_family"], "legacy256-v1");
        assert_eq!(phases["registrations"], 2);
        assert_eq!(phases["releases"], 2);
        assert_eq!(worker.queue_packets, u64::try_from(dispatches).unwrap());
        worker.options.ordered64_runtime_counters = false;
        worker.options.profile = false;
        worker.close().unwrap();
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn width_counter_dispatch_drift_is_not_accepted_as_another_phase() {
    let (mut worker, path) = worker("counter_prefill_width_bad");
    select_native_width(&mut worker, 32);
    worker.options.profile = true;
    worker.options.ordered64_runtime_counters = true;
    worker.token_program_counter_snapshot().unwrap();
    submit_width(&mut worker, 32).unwrap();
    worker.wait_fixed_token(649).unwrap();
    worker.release_registered_token().unwrap();
    assert!(worker.token_program_counter_snapshot().is_err());
    assert!(worker.failed && worker.exited);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn width_backend_identity_is_positive_before_any_model_command() {
    let (mut worker, path) = worker("backend_slots512");
    worker.token_program = None;
    worker
        .verify_token_program_backend(TokenProgramBackend::NativeWholeProgramSlots512V1)
        .unwrap();
    assert_eq!(
        worker.token_program_backend(),
        Some(TokenProgramBackend::NativeWholeProgramSlots512V1)
    );
    assert!(worker.kernels.is_empty() && worker.buffers.is_empty());
    worker.close().unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "describe_token_program_backend_v1\nclose\n"
    );
    std::fs::remove_file(path).unwrap();
}
