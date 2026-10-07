//! Actual retained images plus recording graphs; no GPU execution or numerical claim.
use super::*;

pub(super) fn run(
    mut worker: Worker,
    trace: std::path::PathBuf,
    snapshot: &abi_fixture::Snapshot,
    raw: &[u8],
    images_raw: &[u8],
    artifacts: &[(&str, EngineeringTpArtifactV1)],
) {
    select_native_width(&mut worker, 32);
    let commands = snapshot
        .graphs
        .iter()
        .map(|graph| abi_commands_count(graph, 649))
        .collect::<Vec<_>>();
    let mut reference = None;
    let mut sizes = Vec::new();
    for (index, graph) in commands.iter().enumerate() {
        let plan = prefill32::plan_prefill32(graph, &worker.kernels, &worker.buffers).unwrap();
        assert_eq!(plan.definition.slots.len(), 396);
        assert_eq!(
            plan.definition
                .slots
                .iter()
                .filter(|slot| matches!(slot, Slot::Pointer { .. }))
                .count(),
            144
        );
        assert_eq!(
            plan.definition
                .slots
                .iter()
                .filter(|slot| matches!(slot, Slot::ScalarU32 { .. }))
                .count(),
            252
        );
        assert!(
            wire::validate_token_program_encoding_v1(&plan.definition, &plan.kernargs).is_err()
        );
        wire::validate_token_program_slots512_encoding_v1(&plan.definition, &plan.kernargs)
            .unwrap();
        let (header, payload) =
            wire::encode_token_program_slots512_v1(&plan.definition, &plan.kernargs).unwrap();
        let CommandV1::RegisterTokenProgramSlots512V1 {
            definition_bytes,
            kernarg_bytes,
        } = header
        else {
            panic!("512 family");
        };
        assert!(definition_bytes <= 524_288);
        assert_eq!(kernarg_bytes, 218_616);
        let execute = CommandV1::ExecuteTokenProgramSlots512V1 {
            program: 1,
            expected_epoch: 0,
            expected_completed_packets: u64::try_from(index).unwrap() * 649,
            timeout_ms: DISPATCH_TIMEOUT_MS,
            updates: plan.updates.clone(),
        };
        let execute_header_bytes = serde_json::to_vec(&execute).unwrap().len();
        assert!(execute_header_bytes <= 65536);
        sizes.push(serde_json::json!({"definition_bytes":definition_bytes,"kernarg_bytes":kernarg_bytes,"registration_header_bytes":serde_json::to_vec(&header).unwrap().len(),"execute_header_bytes":execute_header_bytes,"payload_bytes":payload.len()}));
        let immutable = plan.immutable().unwrap();
        if let Some(expected) = &reference {
            assert_eq!(&immutable, expected);
        } else {
            reference = Some(immutable);
        }
    }
    // Relocate the two owned destinations inside the same allocations. This is
    // packing/slot evidence only; no prepared page table or GPU result is claimed.
    for (graph, terminal) in [(&commands[0], false), (&commands[2], true)] {
        let mut noncontiguous = graph.clone();
        for layer in 0..36 {
            for half in 0..2 {
                let logical = if terminal { 1 - half } else { half };
                let page = [7u32, 3][logical];
                let copy = &mut noncontiguous[1 + layer * 18 + 7 + half];
                copy.arguments[5] = EngineeringTpArgumentV1::U32(page);
                for argument in [2, 3] {
                    let EngineeringTpArgumentV1::Buffer { ref mut offset, .. } =
                        copy.arguments[argument]
                    else {
                        panic!("destination");
                    };
                    *offset = usize::try_from(page).unwrap() * 32768;
                }
            }
        }
        let plan =
            prefill32::plan_prefill32(&noncontiguous, &worker.kernels, &worker.buffers).unwrap();
        assert_eq!(plan.immutable().unwrap(), *reference.as_ref().unwrap());
        for layer in 0..36 {
            for half in 0..2 {
                let logical = if terminal { 1 - half } else { half };
                let page = [7u32, 3][logical];
                let updates = &plan.updates[layer * 11 + half * 5..layer * 11 + half * 5 + 5];
                assert!(
                    matches!(updates[0], Update::Pointer { offset, .. } if offset == u64::from(page) * 32768)
                );
                assert!(
                    matches!(updates[1], Update::Pointer { offset, .. } if offset == u64::from(page) * 32768)
                );
                assert_eq!(updates[3], Update::ScalarU32 { value: page });
                assert_eq!(
                    updates[4],
                    Update::ScalarU32 {
                        value: u32::from(terminal && half == 0)
                    }
                );
            }
        }
    }
    // Every role, including the final layer, is checked before the first IPC.
    for index in 0..649 {
        let mut invalid = commands[0].clone();
        invalid[index].grid_workgroups += 1;
        assert!(
            prefill32::plan_prefill32(&invalid, &worker.kernels, &worker.buffers).is_err(),
            "grid {index}"
        );
    }
    for mutation in 0..15 {
        let mut invalid = commands[0].clone();
        let second = 9;
        match mutation {
            0 => {
                invalid.pop();
            }
            1 => invalid[648].kernel = "unloaded_last_layer",
            2 => invalid[second].arguments[4] = EngineeringTpArgumentV1::U32(32),
            3 => invalid[second].arguments[5] = invalid[8].arguments[5].clone(),
            4 => invalid[second].arguments[7] = EngineeringTpArgumentV1::U32(1),
            5 => invalid[10].arguments[10] = EngineeringTpArgumentV1::U32(16),
            6 => invalid[10].arguments[9] = EngineeringTpArgumentV1::U32(1),
            7 => invalid[8 + 35 * 18].arguments[4] = EngineeringTpArgumentV1::U32(16),
            8 => invalid[10 + 35 * 18].arguments[10] = EngineeringTpArgumentV1::U32(64),
            9 => invalid[648].arguments[3] = EngineeringTpArgumentV1::U32(16),
            10 => invalid.swap(8, second),
            11 => invalid[second].arguments[2] = invalid[second].arguments[3].clone(),
            12 => {
                let EngineeringTpArgumentV1::Buffer { ref mut offset, .. } =
                    invalid[second].arguments[0]
                else {
                    panic!("source");
                };
                *offset = 0;
            }
            13 => {
                let EngineeringTpArgumentV1::Buffer { ref mut offset, .. } =
                    invalid[second].arguments[2]
                else {
                    panic!("destination");
                };
                *offset += 2;
            }
            _ => invalid[10].arguments[8] = EngineeringTpArgumentV1::U32(512),
        }
        assert!(
            prefill32::plan_prefill32(&invalid, &worker.kernels, &worker.buffers).is_err(),
            "mutation {mutation}"
        );
    }
    assert!(std::fs::read_to_string(&trace).unwrap().is_empty());
    for graph in &commands {
        worker.submit_fixed_prefill(graph).unwrap();
        worker.wait_prefill_program(649).unwrap();
    }
    assert_eq!(worker.queue_packets, 3 * 649);
    let state = worker.token_program.as_ref().unwrap();
    assert_eq!(
        (
            state.registrations,
            state.releases,
            state.completed_prefills
        ),
        (1, 0, 3)
    );
    let before = std::fs::read_to_string(&trace).unwrap();
    let mut invalid = commands[0].clone();
    invalid[648].arguments[3] = EngineeringTpArgumentV1::U32(16);
    assert!(worker.submit_fixed_prefill(&invalid).is_err());
    assert_eq!(std::fs::read_to_string(&trace).unwrap(), before);
    worker.close().unwrap();
    assert!(worker.exited);
    std::fs::remove_file(trace).unwrap();
    println!(
        "{}",
        serde_json::json!({
            "schema":"FerricPrefill32ProgramActualAbiCpuGateV1", "graph_sha256":abi_sha256_hex(raw),
            "image_map_sha256":abi_sha256_hex(images_raw), "images":artifacts.iter().map(|(role, artifact)| (role, abi_sha256_hex(artifact.bytes()))).collect::<BTreeMap<_,_>>(),
            "graphs":3,"positions":[0,32,96],"dispatches_per_graph":649,"dynamic_slots":396,
            "pointer_slots":144,"scalar_slots":252,"encoding_bytes":sizes,"grid_negative_cases":649,"binding_negative_cases":15,"noncontiguous_page_cases":2,
            "late_invalid_before_ipc":true,"same_shape_registration_reused":true,"native_executed":false,"model_parity":false
        })
    );
}
