use super::*;

fn request(command: Command) -> Request {
    Request {
        protocol: PROTOCOL,
        id: 2,
        device_ids: [7, 9],
        session: [3; 32],
        command,
    }
}
fn part(bytes: u32) -> Part {
    Part {
        bytes,
        sha256: [1; 32],
    }
}

#[test]
fn setup_command_roundtrip_preserves_semantic_keys_and_chunk_bytes() {
    let requests = [
        request(Command::Allocate {
            key: Key::Source { rank: 1, id: 71 },
        }),
        request(Command::Allocate {
            key: Key::Pending {
                rank: 0,
                layer: Some(35),
                role: PendingKind::PackedQkvWeight,
            },
        }),
        request(Command::Write {
            key: Key::Source { rank: 0, id: 5 },
            offset: 8,
            part: part(2),
        }),
        request(Command::AllocateTailHead),
        request(Command::WriteTailHead {
            offset: 0,
            part: part(2),
        }),
        request(Command::LoadArtifacts),
        request(Command::BindAndSeal),
        request(Command::AbortClose),
    ];
    for value in requests {
        let payload = vec![6; value.payload_bytes().unwrap()];
        let mut frame = Vec::new();
        write_request(&mut frame, &value, &payload).unwrap();
        let mut input = frame.as_slice();
        assert_eq!(read_request(&mut input).unwrap(), Some((value, payload)));
        assert!(read_request(&mut input).unwrap().is_none());
    }
}

#[test]
fn begin_tail_is_an_explicit_additional_payload_part() {
    let mut value = request(Command::Begin(Begin {
        scope: Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 4,
            group_id: 0,
            child_identity: 6,
        },
        registration: part(1),
        source_program: part(2),
        uploads: part(3),
        prefix_image: part(4),
        mlp_image: part(5),
        residual_image: part(6),
        tail_image: Some(part(7)),
    }));
    value.id = 1;
    assert_eq!(value.payload_bytes().unwrap(), 28);
    let mut bytes = Vec::new();
    write_request(&mut bytes, &value, &[8; 28]).unwrap();
    assert_eq!(
        read_request(&mut bytes.as_slice()).unwrap(),
        Some((value.clone(), vec![8; 28]))
    );
    assert!(write_request(&mut Vec::new(), &value, &[8; 27]).is_err());
    value.id = 2;
    assert!(value.payload_bytes().is_err());
}

#[test]
fn framing_bounds_truncation_unknown_fields_and_overflow_fail_closed() {
    let key = Key::Source { rank: 0, id: 1 };
    assert!(
        request(Command::Write {
            key,
            offset: 0,
            part: part(source::MAX_TRANSFER as u32 + 1)
        })
        .payload_bytes()
        .is_err()
    );
    assert!(
        request(Command::Write {
            key,
            offset: u64::MAX,
            part: part(1)
        })
        .payload_bytes()
        .is_err()
    );
    assert!(
        request(Command::Allocate {
            key: Key::Pending {
                rank: 0,
                layer: None,
                role: PendingKind::PackedQkvWeight
            }
        })
        .payload_bytes()
        .is_err()
    );
    assert!(read_request(&mut &(source::MAX_HEADER as u32 + 1).to_le_bytes()[..]).is_err());
    assert!(read_request(&mut &[1, 0][..]).is_err());
    let mut value = serde_json::to_value(request(Command::BindAndSeal)).unwrap();
    value["approval"] = serde_json::json!(true);
    let data = serde_json::to_vec(&value).unwrap();
    let mut frame = (data.len() as u32).to_le_bytes().to_vec();
    frame.extend(data);
    assert!(read_request(&mut frame.as_slice()).is_err());
    let mut command = request(Command::BindAndSeal);
    command.id = MAX_COMMANDS + 1;
    assert!(command.payload_bytes().is_err());
}

#[test]
fn setup_responses_never_claim_ready_forward_gpu_or_production_authority() {
    let mut response = Response {
        protocol: PROTOCOL,
        id: 7,
        status: Status::LayersAndTailSealed,
        catalog_id: None,
        native_opened: true,
        gpu_execution: false,
        forward_started: false,
        production_authority: false,
    };
    let mut frame = Vec::new();
    write_response(&mut frame, &response).unwrap();
    assert_eq!(
        read_response(&mut frame.as_slice()).unwrap(),
        Some(response.clone())
    );
    for field in ["gpu_execution", "forward_started", "production_authority"] {
        let mut value = serde_json::to_value(&response).unwrap();
        value[field] = serde_json::json!(true);
        let changed: Response = serde_json::from_value(value).unwrap();
        assert!(write_response(&mut Vec::new(), &changed).is_err());
    }
    response.status = Status::OwnerCreated;
    assert!(write_response(&mut Vec::new(), &response).is_err());
    response.id = 1;
    write_response(&mut Vec::new(), &response).unwrap();
    response.catalog_id = Some(1);
    assert!(write_response(&mut Vec::new(), &response).is_err());
    assert!(serde_json::from_str::<Status>("\"ready\"").is_err());
}
