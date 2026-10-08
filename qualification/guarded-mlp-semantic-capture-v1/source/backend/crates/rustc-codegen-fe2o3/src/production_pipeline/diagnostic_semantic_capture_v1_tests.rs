use super::*;
use fe2o3_mir_model::semantic_mir_v1::*;
use std::path::PathBuf;

fn source_map_files() -> Vec<fe2o3_kernel_ir::DebugSourceMapFileV1> {
    vec![fe2o3_kernel_ir::DebugSourceMapFileV1::new(
        [42; 32],
        123,
        "/diagnostic/display-label.rs".to_owned(),
    )
    .unwrap()]
}

#[test]
fn source_map_capture_absent_failed_or_mismatched_semantic_is_inert() {
    let scratch = CaptureScratch::new();
    let semantic = admitted_capture_fixture();
    let path = scratch.0.join("sources.json");
    let files = source_map_files();
    let mut outcome =
        capture_requested_semantic_v1(Some(&scratch.0.join("semantic.mir")), &semantic).unwrap();
    assert!(capture_requested_source_map_v1(None, &semantic, &files, &outcome).is_none());
    outcome.result = Err(io::ErrorKind::PermissionDenied);
    assert!(capture_requested_source_map_v1(Some(&path), &semantic, &files, &outcome).is_none());
    outcome.result = Ok(());
    outcome.semantic_sha256 = [0; 32];
    assert_eq!(
        capture_requested_source_map_v1(Some(&path), &semantic, &files, &outcome)
            .unwrap()
            .result,
        Err(io::ErrorKind::InvalidData)
    );
    outcome.semantic_sha256 = *semantic.semantic_sha256().as_bytes();
    outcome.bytes += 1;
    assert_eq!(
        capture_requested_source_map_v1(Some(&path), &semantic, &files, &outcome)
            .unwrap()
            .result,
        Err(io::ErrorKind::InvalidData)
    );
    assert!(!path.exists());
}

#[test]
fn source_map_capture_binds_exact_files_and_both_semantic_identities() {
    use sha2::{Digest, Sha256};
    let scratch = CaptureScratch::new();
    let semantic = admitted_capture_fixture();
    let bytes = semantic.canonical_encoding().to_vec();
    let files = source_map_files();
    let outcome =
        capture_requested_semantic_v1(Some(&scratch.0.join("semantic.mir")), &semantic).unwrap();
    let path = scratch.0.join("sources.json");
    let map = capture_requested_source_map_v1(Some(&path), &semantic, &files, &outcome).unwrap();
    assert_eq!(map.result, Ok(()));
    let raw = std::fs::read(path).unwrap();
    assert_eq!(raw.len(), map.bytes);
    assert_eq!(raw.last(), Some(&b'\n'));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&raw).unwrap(),
        serde_json::json!({
            "schema": "fe2o3-diagnostic-semantic-source-map-v1",
            "stage": "pre-ranked",
            "semantic_bytes": bytes.len(),
            "semantic_sha256": crate::encode_hex(semantic.semantic_sha256().as_bytes()),
            "semantic_file_sha256": crate::encode_hex(&Sha256::digest(&bytes)),
            "files": files,
            "execution_authority": false,
        })
    );
    assert_eq!(semantic.canonical_encoding(), bytes);
}

#[test]
fn source_map_encoding_has_inclusive_byte_and_file_bounds() {
    let semantic = admitted_capture_fixture();
    let files = source_map_files();
    let raw = encode_source_map_v1(&semantic, &files, MAX_SOURCE_MAP_BYTES_V1).unwrap();
    assert_eq!(
        encode_source_map_v1(&semantic, &files, raw.len()).unwrap(),
        raw
    );
    assert!(encode_source_map_v1(&semantic, &files, raw.len() - 1).is_err());
    assert!(encode_source_map_v1(&semantic, &files, 0).is_err());
    let second =
        fe2o3_kernel_ir::DebugSourceMapFileV1::new([43; 32], 12, "/second.rs".to_owned()).unwrap();
    assert!(encode_source_map_v1(
        &semantic,
        &[files[0].clone(), files[0].clone()],
        MAX_SOURCE_MAP_BYTES_V1
    )
    .is_err());
    assert!(encode_source_map_v1(
        &semantic,
        &[second, files[0].clone()],
        MAX_SOURCE_MAP_BYTES_V1
    )
    .is_err());
    let many: Vec<_> = (0..=MAX_SOURCE_MAP_FILES_V1)
        .map(|index| {
            let mut id = [1; 32];
            id[..8].copy_from_slice(&(index as u64).to_be_bytes());
            fe2o3_kernel_ir::DebugSourceMapFileV1::new(id, 1, "/file".to_owned()).unwrap()
        })
        .collect();
    assert!(encode_source_map_v1(
        &semantic,
        &many[..MAX_SOURCE_MAP_FILES_V1],
        MAX_SOURCE_MAP_BYTES_V1
    )
    .is_ok());
    assert!(encode_source_map_v1(&semantic, &many, MAX_SOURCE_MAP_BYTES_V1).is_err());
    let escaped =
        fe2o3_kernel_ir::DebugSourceMapFileV1::new([44; 32], 1, "/escaped-\"\\\n.rs".to_owned())
            .unwrap();
    let escaped = vec![escaped];
    let raw = encode_source_map_v1(&semantic, &escaped, MAX_SOURCE_MAP_BYTES_V1).unwrap();
    assert_eq!(
        encode_source_map_v1(&semantic, &escaped, raw.len()).unwrap(),
        raw
    );
    assert!(encode_source_map_v1(&semantic, &escaped, raw.len() - 1).is_err());

    let scratch = CaptureScratch::new();
    let outcome =
        capture_requested_semantic_v1(Some(&scratch.0.join("semantic.mir")), &semantic).unwrap();
    let oversized: Vec<_> = many[..1024]
        .iter()
        .map(|file| {
            fe2o3_kernel_ir::DebugSourceMapFileV1::new(file.identity(), 1, "x".repeat(1024))
                .unwrap()
        })
        .collect();
    let destination = scratch.0.join("oversized.json");
    let refused =
        capture_requested_source_map_v1(Some(&destination), &semantic, &oversized, &outcome)
            .unwrap();
    assert!(refused.result.is_err());
    assert_eq!(refused.bytes, 0);
    assert!(!destination.exists());
}

#[test]
fn source_map_publication_failure_keeps_semantic_and_existing_sidecar() {
    let scratch = CaptureScratch::new();
    let semantic = admitted_capture_fixture();
    let semantic_path = scratch.0.join("semantic.mir");
    let outcome = capture_requested_semantic_v1(Some(&semantic_path), &semantic).unwrap();
    let path = scratch.0.join("sources.json");
    std::fs::write(&path, b"retained partial sidecar").unwrap();
    let failed =
        capture_requested_source_map_v1(Some(&path), &semantic, &source_map_files(), &outcome)
            .unwrap();
    assert_eq!(failed.result, Err(io::ErrorKind::AlreadyExists));
    assert_eq!(std::fs::read(&path).unwrap(), b"retained partial sidecar");
    assert_eq!(
        std::fs::read(&semantic_path).unwrap(),
        semantic.canonical_encoding()
    );
    assert_eq!(outcome.result, Ok(()));
    let missing = scratch.0.join("missing").join("path\nnot-for-stderr.json");
    let failed =
        capture_requested_source_map_v1(Some(&missing), &semantic, &source_map_files(), &outcome)
            .unwrap();
    assert_eq!(failed.result, Err(io::ErrorKind::NotFound));
    let mut receipt = Vec::new();
    write_source_map_receipt_v1(&mut receipt, &failed).unwrap();
    let receipt = String::from_utf8(receipt).unwrap();
    assert_eq!(receipt.lines().count(), 1);
    assert!(receipt.contains("status=failed") && receipt.len() < 256);
    assert!(!receipt.contains("not-for-stderr"));
    assert!(!missing.parent().unwrap().exists());
}

#[test]
fn source_map_receipt_write_failure_does_not_change_capture_outcome() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let outcome = SemanticCaptureOutcomeV1 {
        bytes: 17,
        semantic_sha256: [3; 32],
        result: Ok(()),
    };
    assert_eq!(
        write_source_map_receipt_v1(&mut Broken, &outcome)
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(outcome.result, Ok(()));
}

struct CaptureScratch(PathBuf);

impl CaptureScratch {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fe2o3-semantic-capture-{}-{nonce}",
            std::process::id(),
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for CaptureScratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn admitted_capture_fixture() -> AdmittedInertSemanticMirV1 {
    // The canonical decoder's minimal request recipe, admitted through the
    // current production schema. This is not rustc-source or launch evidence.
    let ty = SemanticTypeIdV1::from_index(0);
    let source = SemanticSourceProvenanceV1::unavailable();
    let declaration = SemanticTypeDeclV1::new(
        SemanticTypeIdentityV1::from_sha256([1; 32]),
        SemanticLayoutIdentityV1::from_sha256([2; 32]),
        SemanticTypeLayoutV1::new_with_backend_repr(
            Some(4),
            4,
            SemanticBackendReprV1::scalar(SemanticBackendScalarV1::initialized(
                SemanticBackendPrimitiveV1::integer(false, 32, 4),
                SemanticScalarValidityRangeV1::new(0, u128::from(u32::MAX)),
            )),
            false,
        )
        .unwrap(),
        SemanticTypeShapeV1::Scalar(SemanticScalarTypeV1::Integer {
            signed: false,
            bits: 32,
        }),
    );
    let mode = SemanticAbiPassModeV1::Direct(
        SemanticAbiValueAttributesV1::new(
            SemanticAbiRegularAttributesV1::new(false, None, false, false, false, true),
            SemanticAbiExtensionV1::None,
            0,
            None,
        )
        .unwrap(),
    );
    let abi = SemanticFunctionAbiV1::new(
        SemanticAbiIdentityV1::from_sha256([3; 32]),
        SemanticLayoutIdentityV1::from_sha256([4; 32]),
        SemanticCanonAbiV1::Rust,
        false,
        false,
        vec![SemanticAbiValueV1::new(ty, mode.clone())],
        SemanticAbiValueV1::new(ty, mode),
    )
    .unwrap();
    let locals = [
        SemanticLocalRoleV1::Return,
        SemanticLocalRoleV1::Argument(0),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, role)| {
        SemanticLocalDeclV1::new(
            SemanticLocalIdentityV1::from_sha256([5 + index as u8; 32]),
            ty,
            role,
            source,
        )
    })
    .collect();
    let block = SemanticBasicBlockV1::new(
        SemanticBlockIdentityV1::from_sha256([7; 32]),
        source,
        vec![],
        SemanticTerminatorV1::new(source, SemanticTerminatorKindV1::Return),
    )
    .unwrap();
    let function = SemanticFunctionDeclV1::new(
        SemanticFunctionIdentityV1::from_sha256([8; 32]),
        SemanticFunctionRoleV1::KernelRoot,
        SemanticItemDefinitionIdentityV1::from_sha256([9; 32]),
        SemanticMonomorphizationIdentityV1::from_sha256([10; 32]),
        SemanticGenericTypeArgumentsIdentityV1::from_sha256([11; 32]),
        SemanticConstGenericArgumentsIdentityV1::from_sha256([12; 32]),
        source,
        abi,
        locals,
        SemanticBlockIdV1::from_index(0),
        vec![block],
    )
    .unwrap();
    InertSemanticMirRequestV1::new(
        SemanticTargetDataLayoutV1::gfx942(SemanticLayoutIdentityV1::from_sha256([13; 32])),
        vec![declaration],
        vec![],
        vec![],
        vec![],
        vec![function],
        vec![SemanticFunctionIdV1::from_index(0)],
    )
    .unwrap()
    .admit_current_production(SemanticMirLimitsV1::default())
    .unwrap()
}

#[test]
fn semantic_capture_absent_opt_in_is_inert() {
    let scratch = CaptureScratch::new();
    let semantic = admitted_capture_fixture();
    let bytes = semantic.canonical_encoding().as_ptr();
    let identity = *semantic.semantic_sha256().as_bytes();
    assert!(capture_requested_semantic_v1(None, &semantic).is_none());
    assert_eq!(semantic.canonical_encoding().as_ptr(), bytes);
    assert_eq!(semantic.semantic_sha256().as_bytes(), &identity);
    assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(), 0);
}

#[test]
fn semantic_capture_preserves_exact_canonical_bytes_identity_and_admission() {
    let scratch = CaptureScratch::new();
    let path = scratch.0.join("semantic.mir");
    let semantic = admitted_capture_fixture();
    let bytes = semantic.canonical_encoding().as_ptr();
    let outcome = capture_requested_semantic_v1(Some(&path), &semantic).unwrap();
    assert_eq!(outcome.result, Ok(()));
    assert_eq!(outcome.bytes, semantic.canonical_encoding().len());
    assert_eq!(
        outcome.semantic_sha256,
        *semantic.semantic_sha256().as_bytes()
    );
    assert_eq!(semantic.canonical_encoding().as_ptr(), bytes);
    let captured = std::fs::read(&path).unwrap();
    assert_eq!(captured, semantic.canonical_encoding());
    let replay = AdmittedInertSemanticMirV1::decode_current_production_canonical(
        &captured,
        SemanticMirLimitsV1::default(),
    )
    .unwrap();
    assert_eq!(replay.canonical_encoding(), semantic.canonical_encoding());
    assert_eq!(replay.semantic_sha256(), semantic.semantic_sha256());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let mut receipt = Vec::new();
    write_capture_receipt_v1(&mut receipt, &outcome).unwrap();
    assert_eq!(
        String::from_utf8(receipt).unwrap(),
        format!(
            "fe2o3 diagnostic semantic MIR: stage=pre-ranked status=complete bytes={} semantic_sha256={} error_kind=None\n",
            captured.len(),
            crate::encode_hex(semantic.semantic_sha256().as_bytes()),
        ),
    );
}

#[test]
fn semantic_capture_byte_cap_is_inclusive_and_rejects_before_creation() {
    let scratch = CaptureScratch::new();
    // Short arbitrary bytes test only publication bounds, not MIR admission.
    let bytes = b"bounded bytes";
    let exact = scratch.0.join("exact");
    write_new_capture_bytes_v1(&exact, bytes, bytes.len()).unwrap();
    assert_eq!(std::fs::read(&exact).unwrap(), bytes);
    let over = scratch.0.join("over");
    assert_eq!(
        write_new_capture_bytes_v1(&over, bytes, bytes.len() - 1)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput,
    );
    assert!(!over.exists());
    let empty = scratch.0.join("empty");
    assert_eq!(
        write_new_capture_bytes_v1(&empty, b"", bytes.len())
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput,
    );
    assert!(!empty.exists());
}

#[test]
fn semantic_capture_refuses_overwrite_and_symlink_without_changing_old_bytes() {
    let scratch = CaptureScratch::new();
    let path = scratch.0.join("existing");
    write_new_capture_bytes_v1(&path, b"original", 8).unwrap();
    assert_eq!(
        write_new_capture_bytes_v1(&path, b"replaced", 8)
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists,
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    #[cfg(unix)]
    {
        let link = scratch.0.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert_eq!(
            write_new_capture_bytes_v1(&link, b"replaced", 8)
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists,
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
    }
}

#[test]
fn semantic_capture_path_failures_are_bounded_and_do_not_create_parents() {
    assert_eq!(
        write_new_capture_bytes_v1(Path::new("relative/semantic.mir"), b"bytes", 5)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput,
    );
    let scratch = CaptureScratch::new();
    let parent = scratch.0.join("missing");
    let path = parent.join("untrusted-path\nmust-not-appear.mir");
    let semantic = admitted_capture_fixture();
    let outcome = capture_requested_semantic_v1(Some(&path), &semantic).unwrap();
    assert_eq!(outcome.result, Err(io::ErrorKind::NotFound));
    assert!(!parent.exists());
    let mut receipt = Vec::new();
    write_capture_receipt_v1(&mut receipt, &outcome).unwrap();
    let receipt = String::from_utf8(receipt).unwrap();
    assert_eq!(
        receipt,
        format!(
            "fe2o3 diagnostic semantic MIR: stage=pre-ranked status=failed bytes={} semantic_sha256={} error_kind=Some(NotFound)\n",
            semantic.canonical_encoding().len(),
            crate::encode_hex(semantic.semantic_sha256().as_bytes()),
        ),
    );
    assert_eq!(receipt.lines().count(), 1);
    assert!(receipt.len() < 256);
    assert!(!receipt.contains("untrusted-path"));
}

#[test]
fn semantic_capture_receipt_write_failure_returns_without_panicking() {
    struct UnavailableOutput;
    impl Write for UnavailableOutput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let outcome = SemanticCaptureOutcomeV1 {
        bytes: 1,
        semantic_sha256: [7; 32],
        result: Err(io::ErrorKind::PermissionDenied),
    };
    assert_eq!(
        write_capture_receipt_v1(&mut UnavailableOutput, &outcome)
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe,
    );
}
