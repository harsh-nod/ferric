use super::*;
fn fixture(group: u64, pid: u32) -> Source {
    // Deliberately inert minimal records for the structural comparator, not a
    // stand-in for Source::new's independently validated full catalog.
    let registration = Registration {
        profile: "fixture".into(),
        bundle_id: [1; 32],
        model_id: [2; 32],
        session: [3; 32],
        pool_identity: group + 1,
        group_id: group,
        child_identity: pid,
        layers: vec![],
        globals: vec![],
        auxiliary: vec![],
        scratch: vec![],
        pending_buffers: vec![],
        state_slots: vec![],
        source_program_bytes: 1,
        source_program_sha256: [4; 32],
    };
    let command = Dispatch {
        symbol: "source_kernel".into(),
        grid_workgroups: 2,
        workgroup_size: 64,
        arguments: vec![
            Argument::Buffer {
                source_id: 7,
                offset: 0,
                elements: 128,
                element_bytes: 4,
                access: Access::Read,
            },
            Argument::U32 { value: 3 },
        ],
    };
    let mut steps = vec![
        Step::Rank {
            rank: 0,
            dispatch: command.clone()
        };
        1012
    ];
    steps.push(Step::Collective {
        rows: 1,
        group_id: group,
        epoch: 0,
        layer: 0,
        model: Model::Target8b,
        operation: Operation::AttentionOutputSum,
        producers: vec![command.clone(); 2],
        consumers: vec![command; 2],
    });
    let program = Program {
        schema: "ferric-finite-source-grammar-v1".into(),
        profile: "fixture".into(),
        group_id: group,
        token_buffer: 1,
        result_buffer: 2,
        metadata: core::array::from_fn(|_| Metadata {
            positions: 3,
            page_table: 4,
            cos: 5,
            sin: 6,
        }),
        steps,
    };
    let raw_program = serde_json::to_vec(&program).unwrap();
    Source {
        registration,
        raw_program,
        manifest: UploadManifest {
            version: 1,
            uploads: vec![],
            tail: None,
        },
        program,
    }
}
#[test]
fn separate_authenticated_groups_and_child_scopes_do_not_require_byte_equality() {
    let a = fixture(0, 17);
    let b = fixture(2, 19);
    assert_ne!(a.raw_program, b.raw_program);
    a.same_input_source(&b).unwrap();
    Program::checked(&a.raw_program, &a.registration).unwrap();
    Program::checked(&b.raw_program, &b.registration).unwrap();
    assert!(Program::checked(&a.raw_program, &b.registration).is_err());
    // The inert catalog cannot pass the real source admission constructor.
    assert!(Source::new(a.registration, a.raw_program, a.manifest, 17).is_err());
}
#[test]
fn epochs_operations_order_and_argument_identity_remain_exact() {
    let a = fixture(0, 17);
    for which in 0..9 {
        let mut b = fixture(2, 19);
        match which {
            0 => {
                if let Step::Collective { epoch, .. } = &mut b.program.steps[1012] {
                    *epoch = 1;
                }
            }
            1 => {
                if let Step::Collective { operation, .. } = &mut b.program.steps[1012] {
                    *operation = Operation::FeedForwardDownSum;
                }
            }
            2 => b.program.steps.swap(0, 1012),
            3 => {
                if let Step::Rank { dispatch, .. } = &mut b.program.steps[0] {
                    dispatch.arguments.swap(0, 1);
                }
            }
            4 => {
                if let Step::Rank { dispatch, .. } = &mut b.program.steps[0] {
                    if let Argument::Buffer { source_id, .. } = &mut dispatch.arguments[0] {
                        *source_id = 8;
                    }
                }
            }
            5 => {
                if let Step::Rank { dispatch, .. } = &mut b.program.steps[0] {
                    dispatch.grid_workgroups = 64;
                }
            }
            6 => b.program.metadata[1].cos += 1,
            7 => b.registration.model_id = [8; 32],
            _ => b.manifest.version = 2,
        }
        assert!(a.same_input_source(&b).is_err(), "mutation {which}");
    }
}
#[test]
fn source_parser_is_closed_and_all_collectives_bind_their_own_group() {
    let a = fixture(0, 17);
    for which in 0..6 {
        let mut value = serde_json::to_value(&a.program).unwrap();
        match which {
            0 => value["unknown"] = true.into(),
            1 => value["steps"][0]["dispatch"]["arguments"][0]["unknown"] = true.into(),
            2 => value["steps"][1012]["group_id"] = 7.into(),
            3 => value["steps"][1012]["model"] = "draft06b".into(),
            4 => value["metadata"][0]["positions"] = "3".into(),
            _ => value["steps"]
                .as_array_mut()
                .unwrap()
                .pop()
                .map(|_| ())
                .unwrap(),
        }
        assert!(Program::checked(&serde_json::to_vec(&value).unwrap(), &a.registration).is_err());
    }
}
