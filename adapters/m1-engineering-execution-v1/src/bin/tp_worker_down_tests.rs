//! Synthetic route and protocol tests; not actual-image or GPU evidence.
use super::*;
use ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS;

fn selected(worker: &mut Worker, enabled: bool) {
    worker.token_program.as_mut().unwrap().backend =
        TokenProgramBackend::NativeWholeProgramSlots512V1;
    worker.configure_down_shape(enabled).unwrap();
}

fn decode_fixture(enabled: bool) -> Plan {
    let mut plan = fixture();
    let packets = if enabled { 688 } else { 652 };
    plan.definition
        .dispatches
        .resize(packets, plan.definition.dispatches[0].clone());
    plan.kernargs.resize(packets * 4, 0);
    plan
}

fn submit_decode(worker: &mut Worker, enabled: bool) -> TpResult<()> {
    let plan = decode_fixture(enabled);
    let immutable = plan.immutable()?;
    worker.submit_shape(
        plan,
        immutable,
        if enabled {
            Shape::DecodeDown688
        } else {
            Shape::Decode
        },
    )
}

fn submit_prefill(worker: &mut Worker) -> TpResult<()> {
    let plan = prefill32_fixture();
    let immutable = plan.immutable()?;
    worker.submit_shape(plan, immutable, Shape::Prefill32)
}

#[test]
fn down_selection_is_fresh_explicit_and_single_use() {
    for mutation in 0..8 {
        let (mut worker, path) = worker("normal");
        worker.token_program.as_mut().unwrap().backend =
            TokenProgramBackend::NativeWholeProgramSlots512V1;
        match mutation {
            0 => worker.queue_epoch = 1,
            1 => worker.queue_packets = 1,
            2 => {
                worker.buffers.insert(1, 4);
            }
            3 => worker.token_program.as_mut().unwrap().counter_snapshots = 1,
            4 => {
                worker.token_program.as_mut().unwrap().backend =
                    TokenProgramBackend::NativeWholeProgramV1
            }
            5 => worker.token_program.as_mut().unwrap().completed_prefills = 1,
            6 => worker.token_program.as_mut().unwrap().releases = 1,
            7 => worker.configure_down_shape(false).unwrap(),
            _ => unreachable!(),
        }
        assert!(worker.configure_down_shape(true).is_err(), "{mutation}");
        assert!(worker.failed && worker.exited);
        assert!(std::fs::read_to_string(&path).unwrap().is_empty());
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn down_program_counts_and_profiles_follow_explicit_arm() {
    for enabled in [false, true] {
        let (mut worker, path) = worker("counter_prefill_width");
        selected(&mut worker, enabled);
        worker.options.profile = true;
        worker.options.ordered64_runtime_counters = true;
        let initial = worker.token_program_counter_snapshot().unwrap();
        assert_eq!(initial["schema"], "FerricNativeDownProgramCountersR1");
        assert_eq!(initial["live_profile"], down::profile(enabled, true));
        assert_eq!(initial["down_splitk8"]["enabled"], enabled);
        for _ in 0..4 {
            submit_prefill(&mut worker).unwrap();
            worker.wait_fixed_token(649).unwrap();
        }
        let packets = if enabled { 688 } else { 652 };
        for _ in 0..127 {
            submit_decode(&mut worker, enabled).unwrap();
            worker.wait_fixed_token(packets).unwrap();
        }
        worker.release_registered_token().unwrap();
        let final_snapshot = worker.token_program_counter_snapshot().unwrap();
        let total = if enabled { 89_972u64 } else { 85_400 };
        assert_eq!(final_snapshot["counters"]["dispatches"], total);
        assert_eq!(final_snapshot["counters"]["executions"], 131);
        assert_eq!(final_snapshot["counters"]["publications"], 131);
        assert_eq!(final_snapshot["counters"]["final_waits"], 131);
        assert_eq!(
            final_snapshot["program_phases"]["decode_c1"]["dispatches_per_execution"],
            packets
        );
        assert_eq!(
            final_snapshot["program_phases"]["decode_c1"]["dynamic_slots"],
            180
        );
        assert_eq!(
            final_snapshot["program_phases"]["decode_c1"]["command_family"],
            "legacy256-v1"
        );
        assert_eq!(
            final_snapshot["program_phases"]["prefill"]["dispatches_per_execution"],
            649
        );
        assert_eq!(final_snapshot["program_phases"]["registrations"], 2);
        assert_eq!(final_snapshot["program_phases"]["releases"], 2);
        assert_eq!(worker.queue_packets, total);
        worker.options.profile = false;
        worker.options.ordered64_runtime_counters = false;
        worker.close().unwrap();
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn down_program_releases_before_each_phase_change_and_reuses_decode() {
    let (mut worker, path) = worker("normal");
    selected(&mut worker, true);
    submit_prefill(&mut worker).unwrap();
    worker.wait_fixed_token(649).unwrap();
    for _ in 0..2 {
        submit_decode(&mut worker, true).unwrap();
        worker.wait_fixed_token(688).unwrap();
    }
    submit_prefill(&mut worker).unwrap();
    worker.wait_fixed_token(649).unwrap();
    let state = worker.token_program.as_ref().unwrap();
    assert_eq!(
        (state.completed_prefills, state.completed_executions),
        (2, 2)
    );
    assert_eq!((state.registrations, state.releases), (3, 2));
    worker.close().unwrap();
    assert_eq!(
        std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        [
            "register_token_program_slots512_v1",
            "execute_token_program_slots512_v1",
            "release_token_program",
            "register_token_program",
            "execute_token_program",
            "execute_token_program",
            "release_token_program",
            "register_token_program_slots512_v1",
            "execute_token_program_slots512_v1",
            "release_token_program",
            "close",
        ]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn down_late_immutable_and_wrong_arm_refuse_without_ipc() {
    for mutation in 0..4 {
        let (mut worker, path) = worker("normal");
        selected(&mut worker, true);
        submit_decode(&mut worker, true).unwrap();
        worker.wait_fixed_token(688).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        let mut plan = decode_fixture(true);
        match mutation {
            0 => plan.definition.dispatches[687].grid[0] += 64,
            1 => *plan.kernargs.last_mut().unwrap() = 1,
            2 => {
                plan.definition.slots.pop();
            }
            3 => plan = decode_fixture(false),
            _ => unreachable!(),
        }
        let immutable = plan.immutable().unwrap();
        assert!(
            worker
                .submit_shape(plan, immutable, Shape::DecodeDown688)
                .is_err()
        );
        assert!(worker.failed && worker.exited);
        assert_eq!(worker.queue_packets, 688);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        std::fs::remove_file(path).unwrap();
    }
    for enabled in [false, true] {
        let (mut worker, path) = worker("normal");
        selected(&mut worker, enabled);
        assert!(submit_decode(&mut worker, !enabled).is_err());
        assert_eq!(worker.queue_packets, 0);
        assert!(std::fs::read_to_string(&path).unwrap().is_empty());
        std::fs::remove_file(path).unwrap();
    }
}

fn view(
    id: u64,
    elements: usize,
    element_bytes: u32,
    access: EngineeringTpBufferAccessV1,
) -> EngineeringTpArgumentV1 {
    EngineeringTpArgumentV1::Buffer {
        id,
        offset: 0,
        elements,
        element_bytes,
        access,
    }
}

fn route() -> Vec<EngineeringTpDispatchV1> {
    use EngineeringTpBufferAccessV1::{Read, Write};
    let empty = EngineeringTpDispatchV1 {
        kernel: "synthetic_other_role",
        grid_workgroups: 1,
        workgroup_size: 64,
        arguments: vec![],
    };
    let mut commands = vec![empty; 688];
    for layer in 0..36 {
        let base = 1 + layer * 19;
        commands[base + 7].kernel = ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
        commands[base + 8].kernel = ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0];
        let weight = 100 + layer as u64;
        let mut arguments = vec![
            view(1, 12_288, 2, Read),
            view(weight, 4096 * 12_288, 2, Read),
            view(2, 8 * 4096, 4, Write),
        ];
        arguments.extend([1, 4096, 12_288, 1, 2].map(EngineeringTpArgumentV1::U32));
        commands[base + 16] = EngineeringTpDispatchV1 {
            kernel: ROOTS[0],
            grid_workgroups: 2048,
            workgroup_size: 64,
            arguments,
        };
        commands[base + 17] = EngineeringTpDispatchV1 {
            kernel: ROOTS[1],
            grid_workgroups: 64,
            workgroup_size: 64,
            arguments: vec![view(2, 8 * 4096, 4, Read), view(3, 4096, 4, Write)],
        };
    }
    commands
}

#[test]
fn down_route_rejects_wrong_layer_role_identity_or_extent() {
    use EngineeringTpBufferAccessV1::{Read, Write};
    let valid = route();
    down::validate_route(&valid).unwrap();
    for layer in 0..36 {
        for offset in [16, 17] {
            let mut invalid = valid.clone();
            invalid[1 + layer * 19 + offset].grid_workgroups += 1;
            assert!(down::validate_route(&invalid).is_err());
        }
    }
    for mutation in 0..12 {
        let mut invalid = valid.clone();
        match mutation {
            0 => {
                invalid.pop();
            }
            1 => invalid[17].arguments[7] = EngineeringTpArgumentV1::U32(5),
            2 => invalid.swap(17, 18),
            3 => invalid[17].arguments[1] = view(1, 4096 * 12_288, 2, Read),
            4 => invalid[36].arguments[1] = invalid[17].arguments[1].clone(),
            5 => invalid[18].arguments[0] = view(9, 8 * 4096, 4, Read),
            6 => invalid[36].arguments[0] = view(3, 12_288, 2, Read),
            7 => invalid[18].arguments[1] = view(2, 4096, 4, Write),
            8 => invalid[17].arguments[2] = view(2, 8 * 4096 - 1, 4, Write),
            9 => invalid[36].arguments[1] = view(3, 4096 * 12_288, 2, Read),
            10 => invalid[687] = invalid[18].clone(),
            11 => invalid[17].arguments[0] = view(1, 12_288, 2, Write),
            _ => unreachable!(),
        }
        assert!(down::validate_route(&invalid).is_err(), "{mutation}");
    }
}

#[test]
fn down_roots_cannot_enter_a_control_graph() {
    for root in ROOTS {
        let mut commands = route();
        commands.truncate(652);
        commands[651].kernel = root;
        let error = plan(&commands, &BTreeMap::new(), &BTreeMap::new())
            .err()
            .unwrap();
        assert!(error.contains("excluded from the control"));
    }
    for root in [
        "ferric_qwen3_c1_gate_up_splitk4_mfma_partial_f32_r1",
        "ferric_qwen3_c1_gate_up_splitk4_merge_bf16_r1",
    ] {
        let mut commands = route();
        commands[687].kernel = root;
        let error = plan_for_shape(
            &commands,
            &BTreeMap::new(),
            &BTreeMap::new(),
            Shape::DecodeDown688,
        )
        .err()
        .unwrap();
        assert!(error.contains("gate/up experiment roots are excluded"));
    }
}
