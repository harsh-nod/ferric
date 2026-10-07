//! Actual ten-image ABI packing with synthetic capacities; no GPU or model parity.
use super::*;
use ferric_m1_engineering_execution_v1::tp_artifact::{
    ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS, EngineeringTpSplitKDownArtifactR1,
    EngineeringTpSplitKDownImageIdsR1,
};

fn symbol(name: &str) -> &'static str {
    ROOTS
        .into_iter()
        .find(|root| *root == name)
        .unwrap_or_else(|| abi_symbol(name))
}

fn open_candidate() -> EngineeringTpSplitKDownArtifactR1 {
    let root = std::path::PathBuf::from(
        std::env::var_os("FERRIC_TEST_SPLITK_DOWN_R1_ARTIFACT").expect("exact tenth image"),
    );
    assert!(root.is_absolute() && root.canonicalize().unwrap() == root);
    let hash = |value: &str| {
        std::array::from_fn(|i| u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).unwrap())
    };
    let ids = EngineeringTpSplitKDownImageIdsR1 {
        hsaco: hash("1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae"),
        manifest: hash("2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54"),
        handoff: hash("cd23060449750a973f05742c0d5a043cb637491743e2ac56b56e847cc2c4dbdf"),
    };
    let bytes = abi_read(&root.join("observation.hsaco"), 12_832);
    assert_eq!(bytes.len(), 12_832);
    assert_eq!(Sha256::digest(&bytes).as_slice(), ids.hsaco);
    let inspection = fe2o3_hsaco_finalize::inspect_finalized(&bytes).unwrap();
    let names = ROOTS.map(|export| {
        let descriptor = inspection
            .descriptor_table()
            .kernels()
            .iter()
            .find(|entry| entry.entry_name().as_str() == export)
            .unwrap();
        (descriptor.logical_name().as_str(), export)
    });
    EngineeringTpSplitKDownArtifactR1::open(&root, &names, ids).unwrap()
}

#[test]
#[ignore = "requires actual retained ten images and native688 capture; CPU only"]
fn actual_images_pack_down_native688_and_reuse180_slots() {
    run(true);
}

#[test]
#[ignore = "requires same ten images and control652 capture; CPU only"]
fn actual_images_pack_down_control652_and_reuse180_slots() {
    run(false);
}

fn run(enabled: bool) {
    let path = std::env::var_os("FERRIC_TOKEN_ABI_GRAPH").expect("explicit driver capture");
    let raw = abi_read(Path::new(&path), abi_fixture::MAX_BYTES);
    let snapshot: abi_fixture::Snapshot = serde_json::from_slice(&raw).unwrap();
    assert_eq!(snapshot.schema, abi_fixture::SCHEMA);
    assert_eq!(
        snapshot
            .graphs
            .iter()
            .map(|graph| graph.position)
            .collect::<Vec<_>>(),
        [143, 144]
    );
    assert!(!snapshot.buffers.is_empty() && snapshot.buffers.len() <= 2048);
    assert!(
        snapshot
            .buffers
            .iter()
            .all(|&(id, bytes)| id != 0 && bytes > 0 && bytes <= 1024 * 1024 * 1024)
    );
    let images_path = std::env::var_os("FERRIC_TOKEN_ABI_IMAGES")
        .expect("closed nine-image map plus separate tenth image");
    let images_raw = abi_read(Path::new(&images_path), 16 * 1024);
    let images: AbiImages = serde_json::from_slice(&images_raw).unwrap();
    let artifacts = abi_open_images(&images);
    let candidate = open_candidate();
    let (mut worker, trace) = worker("normal");
    worker.token_program.as_mut().unwrap().backend =
        TokenProgramBackend::NativeWholeProgramSlots512V1;
    worker.configure_down_shape(enabled).unwrap();
    for artifact in artifacts
        .iter()
        .map(|(_, a)| a)
        .chain(std::iter::once(candidate.artifact()))
    {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        assert_eq!(hash, *artifact.hsaco_id().as_bytes());
        for metadata in artifact.inspection().hsaco().kernels() {
            let id = u64::try_from(worker.kernels.len() + 1).unwrap();
            assert!(
                worker
                    .kernels
                    .insert(
                        metadata.name().into(),
                        LoadedKernel {
                            id,
                            image: hash,
                            metadata: metadata.clone(),
                        }
                    )
                    .is_none()
            );
        }
    }
    worker.buffers = snapshot.buffers.iter().copied().collect();
    assert_eq!(worker.buffers.len(), snapshot.buffers.len());
    let shape = if enabled {
        Shape::DecodeDown688
    } else {
        Shape::Decode
    };
    let commands = snapshot
        .graphs
        .iter()
        .map(|graph| abi_commands_with_symbol(graph, shape.packets(), symbol))
        .collect::<Vec<_>>();
    let plans = commands
        .iter()
        .map(|graph| plan_for_shape(graph, &worker.kernels, &worker.buffers, shape).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(plans[0].immutable().unwrap(), plans[1].immutable().unwrap());
    assert!(
        plans[0]
            .updates
            .iter()
            .zip(&plans[1].updates)
            .all(|(a, b)| a != b)
    );
    let mut measurements = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        let (slots, updates) = abi_expected_slots(&commands[index], &worker);
        assert_eq!(plan.definition.slots, slots);
        assert_eq!(plan.updates, updates);
        assert_eq!(slots.len(), 180);
        assert_eq!(
            slots
                .iter()
                .filter(|slot| matches!(slot, Slot::Pointer { .. }))
                .count(),
            72
        );
        let mut roster = abi_pointer_roster();
        if enabled {
            roster
                .get_mut("ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5")
                .unwrap()
                .0 = 36;
            roster.insert(ROOTS[0], (36, 3, 0));
            roster.insert(ROOTS[1], (36, 2, 0));
        }
        let pointers = abi_pointer_counts_for_roster(
            &commands[index],
            &worker,
            &plan.definition,
            shape.packets(),
            roster,
        );
        assert_eq!(
            pointers,
            if enabled {
                (2641, 2351, 290)
            } else {
                (2569, 2279, 290)
            }
        );
        wire::validate_token_program_encoding_v1(&plan.definition, &plan.kernargs).unwrap();
        let (header, payload) =
            wire::encode_token_program_v1(&plan.definition, &plan.kernargs).unwrap();
        let CommandV1::RegisterTokenProgram {
            definition_bytes,
            kernarg_bytes,
        } = header
        else {
            panic!("legacy decode family")
        };
        assert!(definition_bytes <= wire::MAX_TOKEN_PROGRAM_DEFINITION_BYTES_V1);
        assert_eq!(kernarg_bytes as usize, plan.kernargs.len());
        assert!(payload.len() <= wire::MAX_TRANSFER_BYTES_V1 as usize);
        let execute = CommandV1::ExecuteTokenProgram {
            program: 1,
            expected_epoch: 0,
            expected_completed_packets: (index * shape.packets()) as u64,
            timeout_ms: DISPATCH_TIMEOUT_MS,
            updates: plan.updates.clone(),
        };
        let header_bytes = serde_json::to_vec(&execute).unwrap().len();
        assert!(header_bytes <= wire::MAX_HEADER_BYTES_V1);
        measurements.push(serde_json::json!({"position":snapshot.graphs[index].position,
            "definition_bytes":definition_bytes,"kernarg_bytes":kernarg_bytes,
            "execute_header_bytes":header_bytes,"payload_bytes":payload.len(),"pointer_fixups":pointers}));
    }
    if enabled {
        for layer in 0..36 {
            for offset in [16, 17] {
                let mut invalid = commands[0].clone();
                invalid[1 + layer * 19 + offset].grid_workgroups += 1;
                assert!(plan_for_shape(&invalid, &worker.kernels, &worker.buffers, shape).is_err());
            }
        }
        let mut missing = worker.buffers.clone();
        let EngineeringTpArgumentV1::Buffer { id, .. } = commands[0][17].arguments[2] else {
            panic!("scratch")
        };
        missing.insert(id, 131071);
        assert!(plan_for_shape(&commands[0], &worker.kernels, &missing, shape).is_err());
    }
    let mut late = commands[0].clone();
    late.last_mut().unwrap().kernel = "unloaded_final_kernel";
    assert!(plan_for_shape(&late, &worker.kernels, &worker.buffers, shape).is_err());
    assert!(std::fs::read_to_string(&trace).unwrap().is_empty());
    for graph in &commands {
        worker.submit_fixed_token(graph).unwrap();
        worker.wait_fixed_token(shape.packets()).unwrap();
    }
    assert_eq!(worker.queue_packets, (2 * shape.packets()) as u64);
    assert_eq!(worker.token_program.as_ref().unwrap().registrations, 1);
    let before = std::fs::read_to_string(&trace).unwrap();
    if enabled {
        let mut changed = commands[1].clone();
        let first = changed[17].arguments[1].clone();
        changed[17].arguments[1] = changed[36].arguments[1].clone();
        changed[36].arguments[1] = first;
        assert!(plan_for_shape(&changed, &worker.kernels, &worker.buffers, shape).is_ok());
        assert!(worker.submit_fixed_token(&changed).is_err());
    } else {
        assert!(worker.submit_fixed_token(&late).is_err());
    }
    assert!(worker.failed && worker.exited);
    assert_eq!(std::fs::read_to_string(&trace).unwrap(), before);
    assert_eq!(worker.queue_packets, (2 * shape.packets()) as u64);
    std::fs::remove_file(trace).unwrap();
    println!(
        "{}",
        serde_json::json!({"schema":"FerricNativeDownActualAbiCpuGateR1",
        "enabled":enabled,"graph_sha256":abi_sha256_hex(&raw),"image_map_sha256":abi_sha256_hex(&images_raw),
        "candidate_hsaco_sha256":abi_sha256_hex(candidate.artifact().bytes()),
        "images":artifacts.iter().map(|(role,a)| (role,abi_sha256_hex(a.bytes()))).collect::<BTreeMap<_,_>>(),
        "graphs":2,"dispatches_per_graph":shape.packets(),"dynamic_slots":180,
        "encoding_bytes":measurements,"same_shape_reused":true,"late_invalid_before_further_ipc":true,
        "weight_contents":"synthetic capacity-only identities","native_executed":false,"model_parity":false})
    );
}
