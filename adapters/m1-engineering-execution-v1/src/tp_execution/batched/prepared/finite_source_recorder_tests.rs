use super::*;

fn ranks() -> (Rc<RefCell<Catalog>>, Vec<RecordingRank>) {
    recording_ranks(19).unwrap()
}
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[test]
fn source_identity_never_enables_execution_capabilities() {
    assert!(recording_ranks(0).is_err());
    let (_, mut ranks) = ranks();
    for (rank, transport) in ranks.iter_mut().enumerate() {
        assert_eq!(transport.peer_group_rank(), Some((19, rank as u32, 2)));
        assert!(!transport.supports_peer_dependency_collectives());
        assert!(!transport.supports_concurrent_rounds());
        assert!(!transport.supports_sequences());
        assert!(!transport.supports_queue_rollover());
        assert!(!transport.supports_prepared_peer());
        assert!(
            transport
                .require_loaded_image([1; 32], &["not_admitted"])
                .is_err()
        );
    }
}

#[test]
fn complete_original_upload_binds_exact_rank_id_extent_and_hash() {
    let (catalog, mut ranks) = ranks();
    let first = ranks[0].allocate(7).unwrap();
    let second = ranks[1].allocate(7).unwrap();
    assert_ne!(first, second);
    ranks[0].write(first, 0, b"abc").unwrap();
    ranks[0].write(first, 3, b"defg").unwrap();
    ranks[1].write(second, 0, b"7654321").unwrap();
    let catalog = catalog.borrow();
    assert!(
        catalog
            .validate_upload(0, first, 7, digest(b"abcdefg"))
            .is_ok()
    );
    assert!(
        catalog
            .validate_upload(1, second, 7, digest(b"7654321"))
            .is_ok()
    );
    assert!(
        catalog
            .validate_upload(1, first, 7, digest(b"abcdefg"))
            .is_err()
    );
    assert!(
        catalog
            .validate_upload(0, first, 6, digest(b"abcdefg"))
            .is_err()
    );
    assert!(
        catalog
            .validate_upload(0, first, 7, digest(b"abcdefh"))
            .is_err()
    );
    assert!(
        catalog
            .validate_upload(0, 99, 7, digest(b"abcdefg"))
            .is_err()
    );
}

#[test]
fn partial_or_peer_visible_bytes_cannot_be_immutable_weight_intake() {
    let (catalog, mut ranks) = ranks();
    let partial = ranks[0].allocate(4).unwrap();
    ranks[0].write(partial, 0, b"ab").unwrap();
    assert!(
        catalog
            .borrow()
            .validate_upload(0, partial, 4, digest(b"ab"))
            .is_err()
    );
    let peer = ranks[0].allocate_peer_readable(2).unwrap();
    ranks[0].write(peer, 0, b"cd").unwrap();
    assert!(
        catalog
            .borrow()
            .validate_upload(0, peer, 2, digest(b"cd"))
            .is_err()
    );
}

#[test]
fn write_mutations_poison_both_logical_ranks_without_recording_bad_bytes() {
    for case in 0..7 {
        let (catalog, mut ranks) = ranks();
        let id = ranks[0].allocate(4).unwrap();
        ranks[0].write(id, 0, b"ab").unwrap();
        let result = match case {
            0 => ranks[0].write(id, 1, b"c"),
            1 => ranks[0].write(id, 3, b"c"),
            2 => ranks[0].write(id, 2, b"cde"),
            3 => ranks[0].write(id, usize::MAX, b"c"),
            4 => ranks[1].write(id, 2, b"cd"),
            5 => ranks[0].write(99, 0, b"cd"),
            6 => ranks[0].write(id, 2, b""),
            _ => unreachable!(),
        };
        assert!(result.is_err(), "case {case}");
        assert_eq!(catalog.borrow().allocations[&id].written, 2);
        assert!(ranks[0].allocate(2).is_err());
        assert!(ranks[1].allocate(2).is_err());
    }
}

#[test]
fn chunk_cap_is_exact_and_one_more_is_terminal() {
    let (catalog, mut ranks) = ranks();
    let bytes = vec![0xa5; MAX_CHUNK];
    let id = ranks[0].allocate(MAX_CHUNK + 2).unwrap();
    ranks[0].write(id, 0, &bytes).unwrap();
    ranks[0].write(id, MAX_CHUNK, &[1, 2]).unwrap();
    assert_eq!(catalog.borrow().allocations[&id].written, MAX_CHUNK + 2);
    let (_, mut ranks) = self::ranks();
    let id = ranks[0].allocate(MAX_CHUNK + 2).unwrap();
    assert!(ranks[0].write(id, 0, &vec![0; MAX_CHUNK + 1]).is_err());
    assert!(ranks[1].allocate(1).is_err());
}

#[test]
fn exact_extent_is_metadata_only_and_invalid_allocation_poisoned() {
    let (catalog, mut ranks) = ranks();
    let id = ranks[0].allocate(MAX_BUFFER).unwrap();
    assert_eq!(catalog.borrow().allocations[&id].bytes, MAX_BUFFER);
    assert_eq!(catalog.borrow().allocations[&id].written, 0);
    for bytes in [0, MAX_BUFFER + 1] {
        let (catalog, mut ranks) = self::ranks();
        assert!(ranks[0].allocate(bytes).is_err());
        assert!(catalog.borrow().allocations.is_empty());
        assert!(ranks[1].allocate(1).is_err());
    }
}

#[test]
fn allocation_count_cap_is_bounded_without_native_tokens() {
    let (catalog, mut ranks) = ranks();
    for i in 0..MAX_ALLOCATIONS {
        assert_eq!(ranks[i % 2].allocate(1).unwrap(), i as u64 + 1);
    }
    assert!(ranks[0].allocate(1).is_err());
    assert_eq!(catalog.borrow().allocations.len(), MAX_ALLOCATIONS);
}

#[test]
fn transpose_census_requires_every_real_write_and_exact_count() {
    let (catalog, mut ranks) = ranks();
    ranks[0].allocate(8).unwrap(); // Earlier uninitialized scratch is not a transpose.
    let first = ranks[0].allocate(3).unwrap();
    let second = ranks[1].allocate(2).unwrap();
    ranks[0].write(first, 0, b"abc").unwrap();
    assert!(catalog.borrow().complete_uploads_since(first, 2).is_err());
    ranks[1].write(second, 0, b"de").unwrap();
    assert_eq!(
        catalog.borrow().complete_uploads_since(first, 2).unwrap(),
        5
    );
    assert!(catalog.borrow().complete_uploads_since(first, 1).is_err());
    assert!(catalog.borrow().complete_uploads_since(1, 3).is_err());
}

#[test]
fn runtime_methods_never_succeed_and_poison_the_recorder() {
    for case in 0..3 {
        let (_, mut ranks) = ranks();
        let result = match case {
            0 => ranks[0].read(1, 0, &mut [0]),
            1 => ranks[0].wait(),
            2 => ranks[0].submit(&EngineeringTpDispatchV1 {
                kernel: "not_admitted",
                grid_workgroups: 1,
                workgroup_size: 64,
                arguments: Vec::new(),
            }),
            _ => unreachable!(),
        };
        assert!(result.is_err());
        assert!(ranks[1].allocate(1).is_err());
    }
}

#[test]
fn source_close_is_idempotent_and_prevents_both_rank_mutations() {
    let (catalog, mut ranks) = ranks();
    ranks[0].close().unwrap();
    ranks[0].close().unwrap();
    assert!(ranks[0].allocate(1).is_err());
    assert!(ranks[1].allocate(1).is_err());
    ranks[1].close().unwrap();
    assert_eq!(catalog.borrow().closed, [true; 2]);
}

#[test]
#[ignore = "Authentic model CPU intake and real transposes; explicit bundle path and bounded >20GiB process required"]
fn authentic_source_recorder_939_recipes_and_real_transposes() {
    use crate::tp_paged::{EngineeringTpPagedLimitsV1, EngineeringTpPoolScopeV1};
    let path = std::path::PathBuf::from(
        std::env::var_os("FERRIC_P222_SOURCE_BUNDLE")
            .expect("explicit canonical source bundle required"),
    );
    assert!(path.is_absolute());
    assert_eq!(std::fs::canonicalize(&path).unwrap(), path);
    let model = EngineeringQwenModelV1::open(&path).unwrap();
    let pool = EngineeringTpPagedPoolV1::new(
        EngineeringTpPoolScopeV1 {
            model: *model.config().model_id.as_bytes(),
            session: [0x72; 32],
        },
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 100).unwrap(),
    )
    .unwrap();
    // This process is only the CPU test namespace, never a spawned GPU child.
    let recorder =
        EngineeringTp2FiniteSourceRecorderV1::new(&model, &pool, std::process::id()).unwrap();
    assert!(recorder.recorded_transpose_bytes() > 0);
    for rank in &recorder.driver.inner.transports {
        assert!(!rank.supports_peer_dependency_collectives());
        assert!(!rank.supports_prepared_peer());
    }
    // The unchanged executable admission still refuses this CPU-only driver.
    assert!(
        recorder
            .driver
            .inner
            .check_peer_dependency_profile()
            .is_err()
    );
    let (bytes, source_sha) = recorder.with_plan(|composition, factory, uploads| {
        assert_eq!(uploads.len(), 939);
        assert_eq!(composition.layers().len(), 72);
        assert_eq!(composition.state_slots().len(), 288);
        let source = composition.source_program_snapshot_v1()?;
        assert!(!source.is_empty() && source.len() <= 4 << 20);
        let head = factory.head_transpose_upload()?;
        assert_eq!(head.bytes(), 1_244_659_712);
        assert_eq!(head.dimensions(), [4096, 151_936]);
        let binding = model.layout().lookup(ferric_spec::Qwen3ModelRole::Target8B,
            ferric_spec::Qwen3TensorKind::LanguageModelHead, ferric_spec::QWEN3_NO_LAYER).unwrap();
        assert_eq!(head.source_sha256(), binding.sha256());
        let original = crate::tp_execution::section_bytes(model.target_weights(), binding.destination_range())?;
        let mut streamed = Sha256::new();
        let mut written = 0;
        let mut chunks = 0;
        head.visit_chunks(MAX_CHUNK, |offset, bytes| {
            assert_eq!(offset, written);
            // Independent scalar NxK->KxN indexing, not the production packer.
            for (index, word) in bytes.chunks_exact(2).enumerate() {
                let output_element = offset / 2 + index;
                let source_row = output_element % 151_936;
                let source_column = output_element / 151_936;
                let from = (source_row * 4096 + source_column) * 2;
                assert_eq!(word, &original[from..from + 2]);
            }
            streamed.update(bytes);
            written += bytes.len();
            chunks += 1;
            Ok(())
        })?;
        assert_eq!(written, head.bytes());
        assert_eq!(chunks, 316);
        assert_eq!(<[u8; 32]>::from(streamed.finalize()), head.sha256());
        let grammar: serde_json::Value = serde_json::from_slice(&source).unwrap();
        let queued_head = grammar["steps"].as_array().unwrap().iter().rev().find(|step|
            step["kind"] == "rank" && step["rank"] == 0
                && step["dispatch"]["symbol"] == "ferric_qwen3_tp_mfma_gemm_bf16_v3").unwrap();
        let arguments = queued_head["dispatch"]["arguments"].as_array().unwrap();
        assert_eq!(arguments.len(), 8);
        assert_eq!(arguments[4]["value"], 151_936);
        assert_eq!(arguments[5]["value"], 4096);
        assert_eq!(arguments[7]["value"], 6);
        let transpose_id = arguments[1]["source_id"].as_u64().unwrap();
        assert_ne!(transpose_id, head.source_id());
        recorder.catalog.borrow().validate_upload(0, transpose_id, head.bytes(), head.sha256())?;
        println!("P222_HEAD_TRANSPOSE bytes={written} chunks={chunks} original_scalar_index_equal=true queued_transpose_digest_equal=true sha256={:02x?}", head.sha256());
        Ok((uploads.iter().map(EngineeringTp2FiniteUploadV1::bytes).sum::<usize>(), digest(&source)))
    }).unwrap();
    assert_eq!(bytes, 18_194_055_168);
    assert_ne!(source_sha, [0; 32]);
    println!(
        "P222_SOURCE_RECORDER uploads=939 real_transposes={TRANSPOSES} bytes={bytes} native_opened=false gpu_execution=false"
    );
}
