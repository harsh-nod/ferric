//! Read-only inspection of explicitly pinned pre-ranked semantic captures.
use fe2o3_mir_model::semantic_mir_v1::*;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const MIR_MAX: usize = 16 * 1024 * 1024;
const MAP_MAX: usize = 1024 * 1024;
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceMap {
    schema: String,
    stage: String,
    semantic_bytes: usize,
    semantic_sha256: String,
    semantic_file_sha256: String,
    files: Vec<SourceFile>,
    execution_authority: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    identity: String,
    byte_len: u64,
    display_path: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest(raw: &[u8]) -> String {
    hex(&Sha256::digest(raw))
}

fn check_hash(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err("expected lowercase SHA256")
    }
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    if !path.is_absolute() || path.canonicalize().map_err(|_| "input path unavailable")? != path {
        return Err("input must be canonical and absolute");
    }
    let before = std::fs::symlink_metadata(path).map_err(|_| "input metadata unavailable")?;
    if !before.is_file() || before.len() == 0 || before.len() > maximum as u64 {
        return Err("input extent refused");
    }
    let file = File::open(path).map_err(|_| "input open failed")?;
    let stamp = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.mode(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if stamp(
        &file
            .metadata()
            .map_err(|_| "input descriptor metadata unavailable")?,
    ) != stamp(&before)
    {
        return Err("input substituted");
    }
    let mut raw = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|_| "input read failed")?;
    let after = std::fs::symlink_metadata(path).map_err(|_| "input disappeared")?;
    if raw.len() != before.len() as usize || stamp(&after) != stamp(&before) {
        return Err("input changed");
    }
    Ok(raw)
}

fn map_for(raw: &[u8], bytes: &[u8], mir_hash: &str, map_hash: &str) -> Result<SourceMap> {
    check_hash(mir_hash)?;
    check_hash(map_hash)?;
    if raw.is_empty() || raw.len() > MIR_MAX || bytes.is_empty() || bytes.len() > MAP_MAX {
        return Err("capture byte limit");
    }
    if digest(raw) != mir_hash || digest(bytes) != map_hash {
        return Err("capture digest mismatch");
    }
    let map: SourceMap = serde_json::from_slice(bytes).map_err(|_| "source-map schema rejected")?;
    check_hash(&map.semantic_sha256)?;
    if map.schema != "fe2o3-diagnostic-semantic-source-map-v1"
        || map.stage != "pre-ranked"
        || map.execution_authority
        || map.semantic_bytes != raw.len()
        || map.semantic_file_sha256 != mir_hash
        || map.files.len() > 4096
    {
        return Err("source-map contract rejected");
    }
    for file in &map.files {
        check_hash(&file.identity)?;
        if file.identity.bytes().all(|b| b == b'0')
            || file.byte_len == 0
            || file.display_path.is_empty()
            || file.display_path.len() > 4096
            || file.display_path.contains('\0')
        {
            return Err("source-file record rejected");
        }
    }
    if map
        .files
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err("source-file identities are not sorted and unique");
    }
    Ok(map)
}

fn origin(origin: Option<SemanticSourceOriginV1>, map: &SourceMap) -> Result<Value> {
    let Some(origin) = origin else {
        return Ok(Value::Null);
    };
    let identity = hex(origin.file().as_bytes());
    let file = map
        .files
        .iter()
        .find(|file| file.identity == identity)
        .ok_or("source origin missing from map")?;
    let (start, end) = origin.byte_range();
    if start > end || end > file.byte_len {
        return Err("source origin exceeds recorded file extent");
    }
    Ok(
        json!({"file_identity": identity, "byte_range": [start, end],
        "start": origin.start_coordinate(), "end": origin.end_coordinate(),
        "display_path": file.display_path, "file_bytes": file.byte_len,
        "source_file_contents_verified": false}),
    )
}

fn shape(value: &SemanticTypeShapeV1) -> &'static str {
    match value {
        SemanticTypeShapeV1::Unit => "Unit",
        SemanticTypeShapeV1::Never => "Never",
        SemanticTypeShapeV1::Scalar(_) => "Scalar",
        SemanticTypeShapeV1::ValidityScalar(_) => "ValidityScalar",
        SemanticTypeShapeV1::Pointer(_) => "Pointer",
        SemanticTypeShapeV1::Array { .. } => "Array",
        SemanticTypeShapeV1::Slice { .. } => "Slice",
        SemanticTypeShapeV1::Tuple(_) => "Tuple",
        SemanticTypeShapeV1::Aggregate(_) => "Aggregate",
        SemanticTypeShapeV1::Union(_) => "Union",
        SemanticTypeShapeV1::Enum { .. } => "Enum",
        SemanticTypeShapeV1::FunctionPointer { .. } => "FunctionPointer",
        SemanticTypeShapeV1::Opaque => "Opaque",
    }
}

fn mode(value: &SemanticAbiPassModeV1) -> &'static str {
    match value {
        SemanticAbiPassModeV1::Ignore => "Ignore",
        SemanticAbiPassModeV1::Direct(_) => "Direct",
        SemanticAbiPassModeV1::Pair { .. } => "Pair",
        SemanticAbiPassModeV1::Cast { .. } => "Cast",
        SemanticAbiPassModeV1::Indirect { .. } => "Indirect",
    }
}

fn type_record(semantic: &AdmittedInertSemanticMirV1, id: SemanticTypeIdV1) -> Result<Value> {
    let ty = semantic
        .types()
        .get(id.index() as usize)
        .ok_or("type absent")?;
    let mut value = json!({"type": id.index(), "shape": shape(ty.shape()),
        "rust_type_kind": format!("{:?}", ty.rust_type_kind()),
        "identity": hex(ty.identity().as_bytes()), "layout_identity": hex(ty.layout_identity().as_bytes())});
    if let SemanticTypeShapeV1::Pointer(pointer) = ty.shape() {
        let target = semantic
            .types()
            .get(pointer.pointee().index() as usize)
            .ok_or("pointee absent")?;
        value["pointer"] = json!({"kind": format!("{:?}", pointer.kind()),
            "mutability": format!("{:?}", pointer.mutability()), "address_space": pointer.address_space(),
            "pointer_width_bits": pointer.pointer_width_bits(), "metadata": format!("{:?}", pointer.metadata()),
            "pointee": {"type": pointer.pointee().index(), "shape": shape(target.shape()),
                "rust_type_kind": format!("{:?}", target.rust_type_kind()),
                "identity": hex(target.identity().as_bytes())}});
    }
    Ok(value)
}

fn inspect(
    raw: &[u8],
    map_raw: &[u8],
    mir_hash: &str,
    map_hash: &str,
    helper: &str,
) -> Result<Value> {
    check_hash(helper)?;
    let map = map_for(raw, map_raw, mir_hash, map_hash)?;
    let limits = SemanticMirLimitsV1::default()
        .with_limit(SemanticMirResourceV1::CanonicalBytes, MIR_MAX as u64)
        .map_err(|_| "semantic limits rejected")?;
    let semantic = AdmittedInertSemanticMirV1::decode_current_production_canonical(raw, limits)
        .map_err(|_| "canonical semantic decode rejected")?;
    if semantic.canonical_encoding() != raw
        || hex(semantic.semantic_sha256().as_bytes()) != map.semantic_sha256
    {
        return Err("decoded semantic identity mismatch");
    }
    let matches: Vec<_> = semantic
        .functions()
        .iter()
        .enumerate()
        .filter(|(_, function)| hex(function.identity().as_bytes()) == helper)
        .collect();
    if matches.len() != 1 {
        return Err("helper identity not unique or absent");
    }
    let (index, function) = matches[0];
    let id = SemanticFunctionIdV1::from_index(
        u32::try_from(index).map_err(|_| "function index overflow")?,
    );
    let logical = semantic
        .logical_arguments_v1(id)
        .map_err(|_| "logical argument mapping rejected")?;
    let mut arguments = Vec::new();
    for argument in logical.adjusted_arguments() {
        arguments.push(json!({"adjusted_argument": argument.ordinal(), "source_argument": argument.source_argument(),
            "tuple_field": argument.tuple_field(), "local": argument.local().index(), "local_field": argument.local_field(),
            "ownership": format!("{:?}", argument.source_ownership()), "abi_mode": mode(argument.abi().mode()),
            "type": type_record(&semantic, argument.abi().ty())?}));
    }
    Ok(json!({"schema": "fe2o3-semantic-capture-inspection-v1",
        "semantic_file_sha256": mir_hash, "source_map_file_sha256": map_hash,
        "semantic_sha256": hex(semantic.semantic_sha256().as_bytes()), "semantic_bytes": raw.len(),
        "canonical_decode_verified": true, "execution_authority": false, "production_admission": false,
        "helper": {"function_index": index, "identity": helper,
            "item_definition_identity": hex(function.item_definition_identity().as_bytes()),
            "monomorphization_identity": hex(function.monomorphization_identity().as_bytes()),
            "generic_type_arguments_identity": hex(function.generic_type_arguments_identity().as_bytes()),
            "const_generic_arguments_identity": hex(function.const_generic_arguments_identity().as_bytes()),
            "role": format!("{:?}", function.role()), "arguments": arguments,
            "call_site": origin(function.source().call_site(), &map)?,
            "expansion": origin(function.source().expansion(), &map)?}}))
}

fn run(args: &[String]) -> Result<()> {
    if args.len() != 6 {
        return Err("usage: MIR MAP MIR_SHA256 MAP_SHA256 HELPER_ID");
    }
    let raw = read_bounded(Path::new(&args[1]), MIR_MAX)?;
    let map = read_bounded(Path::new(&args[2]), MAP_MAX)?;
    let value = inspect(&raw, &map, &args[3], &args[4], &args[5])?;
    let output = serde_json::to_vec(&value).map_err(|_| "JSON output failed")?;
    if output.len() >= MAP_MAX {
        return Err("diagnostic output limit");
    }
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&output)
        .and_then(|_| stdout.write_all(b"\n"))
        .map_err(|_| "output write failed")
}

fn main() {
    if let Err(error) = run(&std::env::args().collect::<Vec<_>>()) {
        eprintln!("semantic capture inspection: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> AdmittedInertSemanticMirV1 {
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

    fn mapping(raw: &[u8]) -> Value {
        json!({"schema":"fe2o3-diagnostic-semantic-source-map-v1", "stage":"pre-ranked",
            "semantic_bytes":raw.len(), "semantic_sha256":digest(raw), "semantic_file_sha256":digest(raw),
            "files":[], "execution_authority":false})
    }

    fn inspect_fixture(raw: &[u8], map: &Value, helper: &str) -> Result<Value> {
        let bytes = serde_json::to_vec(map).unwrap();
        inspect(raw, &bytes, &digest(raw), &digest(&bytes), helper)
    }

    #[test]
    fn exact_canonical_roundtrip_reports_one_helper_without_authority() {
        let fixture = fixture();
        let raw = fixture.canonical_encoding();
        let result = inspect_fixture(raw, &mapping(raw), &hex(&[8; 32])).unwrap();
        assert_eq!(result["helper"]["identity"], hex(&[8; 32]));
        assert_eq!(result["helper"]["arguments"][0]["abi_mode"], "Direct");
        assert_eq!(result["helper"]["call_site"], Value::Null);
        assert_eq!(result["canonical_decode_verified"], true);
        assert_eq!(result["execution_authority"], false);
        assert_eq!(result["production_admission"], false);
    }

    #[test]
    fn truncated_trailing_and_noncanonical_mir_are_rejected() {
        let f = fixture();
        let raw = f.canonical_encoding();
        for mut changed in [raw[..raw.len() - 1].to_vec(), raw.to_vec()] {
            if changed.len() == raw.len() {
                changed.push(0);
            }
            assert!(inspect_fixture(&changed, &mapping(&changed), &hex(&[8; 32])).is_err());
        }
        let mut changed = raw.to_vec();
        changed[0] ^= 1;
        assert!(inspect_fixture(&changed, &mapping(&changed), &hex(&[8; 32])).is_err());
    }

    #[test]
    fn digest_extent_identity_and_schema_mismatches_are_rejected() {
        let f = fixture();
        let raw = f.canonical_encoding();
        let map = mapping(raw);
        let bytes = serde_json::to_vec(&map).unwrap();
        assert!(inspect(raw, &bytes, &hex(&[0; 32]), &digest(&bytes), &hex(&[8; 32])).is_err());
        assert!(inspect(raw, &bytes, &digest(raw), &hex(&[0; 32]), &hex(&[8; 32])).is_err());
        assert!(inspect_fixture(raw, &map, &hex(&[9; 32])).is_err());
        assert!(inspect_fixture(raw, &map, "not-an-id").is_err());
        for (key, value) in [
            ("semantic_bytes", json!(raw.len() + 1)),
            ("semantic_sha256", json!(hex(&[0; 32]))),
            ("semantic_file_sha256", json!(hex(&[0; 32]))),
            ("execution_authority", json!(true)),
            ("stage", json!("other")),
            ("schema", json!("other")),
            ("unknown", json!(1)),
        ] {
            let mut changed = map.clone();
            changed[key] = value;
            assert!(
                inspect_fixture(raw, &changed, &hex(&[8; 32])).is_err(),
                "{key}"
            );
        }
    }

    #[test]
    fn map_files_are_bounded_sorted_unique_and_strict() {
        let f = fixture();
        let raw = f.canonical_encoding();
        let mut map = mapping(raw);
        let file = json!({"identity":hex(&[1;32]),"byte_len":1,"display_path":"/label.rs"});
        map["files"] = json!([file.clone()]);
        assert!(inspect_fixture(raw, &map, &hex(&[8; 32])).is_ok());
        for files in [
            json!([file.clone(), file.clone()]),
            json!([{"identity":hex(&[2;32]),"byte_len":1,"display_path":"b"},file.clone()]),
            json!([{"identity":hex(&[0;32]),"byte_len":1,"display_path":"a"}]),
            json!([{"identity":hex(&[1;32]),"byte_len":0,"display_path":"a"}]),
            json!([{"identity":hex(&[1;32]),"byte_len":1,"display_path":"a","unknown":true}]),
            json!(vec![file.clone(); 4097]),
        ] {
            map["files"] = files;
            assert!(inspect_fixture(raw, &map, &hex(&[8; 32])).is_err());
        }
    }

    #[test]
    fn oversized_inputs_are_rejected_before_decode() {
        let raw = vec![0; MIR_MAX + 1];
        let map = b"{}";
        assert_eq!(
            inspect(&raw, map, &digest(&raw), &digest(map), &hex(&[8; 32])).unwrap_err(),
            "capture byte limit"
        );
        let raw = b"x";
        let map = vec![0; MAP_MAX + 1];
        assert_eq!(
            inspect(raw, &map, &digest(raw), &digest(&map), &hex(&[8; 32])).unwrap_err(),
            "capture byte limit"
        );
    }

    #[test]
    fn source_origin_requires_full_identity_and_bounded_extent() {
        let id = [0x42; 32];
        let span = SemanticSourceOriginV1::new(
            SemanticSourceFileIdentityV1::from_sha256(id),
            3,
            9,
            2,
            4,
            2,
            10,
        )
        .unwrap();
        let mut map: SourceMap = serde_json::from_value(mapping(b"x")).unwrap();
        assert_eq!(origin(None, &map).unwrap(), Value::Null);
        assert_eq!(
            origin(Some(span), &map).unwrap_err(),
            "source origin missing from map"
        );
        let mut other = id;
        other[31] ^= 1;
        map.files.push(SourceFile {
            identity: hex(&other),
            byte_len: 9,
            display_path: "/label.rs".into(),
        });
        assert_eq!(
            origin(Some(span), &map).unwrap_err(),
            "source origin missing from map"
        );
        map.files[0].identity = hex(&id);
        let value = origin(Some(span), &map).unwrap();
        assert_eq!(value["file_identity"], hex(&id));
        assert_eq!(value["byte_range"], json!([3, 9]));
        assert_eq!(value["start"], json!([2, 4]));
        assert_eq!(value["end"], json!([2, 10]));
        assert_eq!(value["source_file_contents_verified"], false);
        map.files[0].byte_len = 8;
        assert_eq!(
            origin(Some(span), &map).unwrap_err(),
            "source origin exceeds recorded file extent"
        );
        assert!(
            SemanticSourceOriginV1::new(
                SemanticSourceFileIdentityV1::from_sha256(id),
                9,
                3,
                2,
                4,
                2,
                10,
            )
            .is_err()
        );
    }

    #[test]
    fn relative_file_and_argument_count_are_refused() {
        assert!(read_bounded(Path::new("relative.bin"), MIR_MAX).is_err());
        assert!(run(&[]).is_err());
        assert!(check_hash(&"A".repeat(64)).is_err());
    }
}
