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

fn decode_capture(
    raw: &[u8],
    map_raw: &[u8],
    mir_hash: &str,
    map_hash: &str,
) -> Result<(AdmittedInertSemanticMirV1, SourceMap)> {
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
    Ok((semantic, map))
}

fn function_record(
    semantic: &AdmittedInertSemanticMirV1,
    map: &SourceMap,
    index: usize,
) -> Result<Value> {
    let function = semantic.functions().get(index).ok_or("function absent")?;
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
    Ok(
        json!({"function_index": index, "identity": hex(function.identity().as_bytes()),
            "item_definition_identity": hex(function.item_definition_identity().as_bytes()),
            "monomorphization_identity": hex(function.monomorphization_identity().as_bytes()),
            "generic_type_arguments_identity": hex(function.generic_type_arguments_identity().as_bytes()),
            "const_generic_arguments_identity": hex(function.const_generic_arguments_identity().as_bytes()),
            "role": format!("{:?}", function.role()), "arguments": arguments,
            "call_site": origin(function.source().call_site(), &map)?,
            "expansion": origin(function.source().expansion(), &map)?}),
    )
}

fn inspection_record(
    semantic: &AdmittedInertSemanticMirV1,
    bytes: usize,
    mir_hash: &str,
    map_hash: &str,
    schema: &str,
) -> Value {
    json!({"schema": schema, "semantic_file_sha256": mir_hash,
        "source_map_file_sha256": map_hash,
        "semantic_sha256": hex(semantic.semantic_sha256().as_bytes()), "semantic_bytes": bytes,
        "canonical_decode_verified": true, "execution_authority": false, "production_admission": false})
}

fn inspect(
    raw: &[u8],
    map_raw: &[u8],
    mir_hash: &str,
    map_hash: &str,
    helper: &str,
) -> Result<Value> {
    check_hash(helper)?;
    let (semantic, map) = decode_capture(raw, map_raw, mir_hash, map_hash)?;
    let matches: Vec<_> = semantic
        .functions()
        .iter()
        .enumerate()
        .filter(|(_, function)| hex(function.identity().as_bytes()) == helper)
        .map(|(index, _)| index)
        .collect();
    if matches.len() != 1 {
        return Err("helper identity not unique or absent");
    }
    let mut result = inspection_record(
        &semantic,
        raw.len(),
        mir_hash,
        map_hash,
        "fe2o3-semantic-capture-inspection-v1",
    );
    result["helper"] = function_record(&semantic, &map, matches[0])?;
    Ok(result)
}

fn call_type_record(semantic: &AdmittedInertSemanticMirV1, id: SemanticTypeIdV1) -> Result<Value> {
    let mut value = type_record(semantic, id)?;
    let ty = semantic
        .types()
        .get(id.index() as usize)
        .ok_or("type absent")?;
    if let SemanticTypeShapeV1::Scalar(scalar) = ty.shape() {
        value["scalar"] = match scalar {
            SemanticScalarTypeV1::Bool => json!({"kind": "Bool"}),
            SemanticScalarTypeV1::Char => json!({"kind": "Char"}),
            SemanticScalarTypeV1::Integer { signed, bits } => {
                json!({"kind": "Integer", "signed": signed, "bits": bits})
            }
            SemanticScalarTypeV1::Float { bits } => json!({"kind": "Float", "bits": bits}),
        };
    }
    Ok(value)
}

fn operand_record(
    semantic: &AdmittedInertSemanticMirV1,
    map: &SourceMap,
    caller: &SemanticFunctionDeclV1,
    operand: &SemanticOperandV1,
) -> Result<Value> {
    let mut result = json!({"type": call_type_record(semantic, operand.ty())?});
    let place = match operand {
        SemanticOperandV1::Copy(place) => {
            result["kind"] = json!("Copy");
            place
        }
        SemanticOperandV1::Move(place) => {
            result["kind"] = json!("Move");
            place
        }
        SemanticOperandV1::Constant(constant) => {
            result["kind"] = json!("Constant");
            result["value_kind"] = json!(match constant.value() {
                SemanticConstantValueV1::ZeroSized => "ZeroSized",
                SemanticConstantValueV1::Scalar(_) => "Scalar",
                SemanticConstantValueV1::Bytes(_) => "Bytes",
                SemanticConstantValueV1::Pointer(_) => "Pointer",
                SemanticConstantValueV1::Callable(_) => "Callable",
            });
            return Ok(result);
        }
    };
    if place.projections().len() > 64 {
        return Err("call projection diagnostic limit");
    }
    let local = caller
        .locals()
        .get(place.local().index() as usize)
        .ok_or("operand local absent")?;
    let projections = place
        .projections()
        .iter()
        .map(|projection| {
            Ok(json!({"kind": format!("{:?}", projection.kind()),
                  "result_type": call_type_record(semantic, projection.result_type())?}))
        })
        .collect::<Result<Vec<_>>>()?;
    result["local"] = json!({"index": place.local().index(), "identity": hex(local.identity().as_bytes()),
        "type": call_type_record(semantic, local.ty())?,
        "call_site": origin(local.source().call_site(), map)?,
        "expansion": origin(local.source().expansion(), map)?});
    result["projections"] = json!(projections);
    Ok(result)
}

fn inspect_call(
    raw: &[u8],
    map_raw: &[u8],
    mir_hash: &str,
    map_hash: &str,
    function_index: u32,
    block_index: u32,
) -> Result<Value> {
    let (semantic, map) = decode_capture(raw, map_raw, mir_hash, map_hash)?;
    let caller = semantic
        .functions()
        .get(function_index as usize)
        .ok_or("caller absent")?;
    let block = caller
        .blocks()
        .get(block_index as usize)
        .ok_or("call block absent")?;
    let SemanticTerminatorKindV1::Call(call) = block.terminator().kind() else {
        return Err("selected terminator is not a call");
    };
    let callable = semantic
        .callables()
        .get(call.callee().index() as usize)
        .ok_or("callable absent")?;
    let SemanticCallableDeclV1::Defined {
        function: callee_id,
    } = callable
    else {
        return Err("selected call is not a defined function");
    };
    let callee = semantic
        .functions()
        .get(callee_id.index() as usize)
        .ok_or("callee absent")?;
    let expected = callee.abi().source_input_types();
    if call.arguments().len() > 256 {
        return Err("call argument diagnostic limit");
    }
    if call.arguments().len() != expected.len() || !call.variadic_argument_abis().is_empty() {
        return Err("defined call source argument count mismatch");
    }
    let mut arguments = Vec::new();
    for (index, (operand, expected)) in call.arguments().iter().zip(expected).enumerate() {
        arguments.push(json!({"source_argument": index,
            "operand": operand_record(&semantic, &map, caller, operand)?,
            "expected_source_type": call_type_record(&semantic, *expected)?,
            "semantic_type_matches": operand.ty() == *expected}));
    }
    let mut result = inspection_record(
        &semantic,
        raw.len(),
        mir_hash,
        map_hash,
        "fe2o3-semantic-call-inspection-v1",
    );
    // These records precede lowering; they cannot establish transient KIR binding types.
    result["kir_binding_types_observed"] = json!(false);
    result["caller"] = function_record(&semantic, &map, function_index as usize)?;
    result["callee"] = function_record(&semantic, &map, callee_id.index() as usize)?;
    result["call"] = json!({"function_index": function_index, "block_index": block_index,
        "block_identity": hex(block.identity().as_bytes()), "callable_index": call.callee().index(),
        "callee_function_index": callee_id.index(), "arguments": arguments,
        "call_site": origin(block.terminator().source().call_site(), &map)?,
        "expansion": origin(block.terminator().source().expansion(), &map)?});
    Ok(result)
}

fn selector_index(value: &str) -> Result<u32> {
    if value.is_empty()
        || value.len() > 10
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err("expected canonical u32 index");
    }
    value.parse().map_err(|_| "expected canonical u32 index")
}

fn encode_output(value: &Value) -> Result<Vec<u8>> {
    let output = serde_json::to_vec(value).map_err(|_| "JSON output failed")?;
    if output.len() >= MAP_MAX {
        return Err("diagnostic output limit");
    }
    Ok(output)
}

fn run(args: &[String]) -> Result<()> {
    let call = match args.len() {
        6 => None,
        8 if args[5] == "--call" => Some((selector_index(&args[6])?, selector_index(&args[7])?)),
        _ => return Err("usage: MIR MAP MIR_SHA256 MAP_SHA256 HELPER_ID | --call FUNCTION BLOCK"),
    };
    let raw = read_bounded(Path::new(&args[1]), MIR_MAX)?;
    let map = read_bounded(Path::new(&args[2]), MAP_MAX)?;
    let value = match call {
        Some((function, block)) => inspect_call(&raw, &map, &args[3], &args[4], function, block)?,
        None => inspect(&raw, &map, &args[3], &args[4], &args[5])?,
    };
    let output = encode_output(&value)?;
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

    #[derive(Clone, Copy)]
    enum CallFixture {
        Copy,
        Move,
        Constant,
        Field,
        Intrinsic,
    }

    fn call_fixture(
        kind: CallFixture,
        source: SemanticSourceProvenanceV1,
    ) -> AdmittedInertSemanticMirV1 {
        let base = fixture();
        let old = &base.functions()[0];
        let ty = SemanticTypeIdV1::from_index(0);
        let place =
            |local| SemanticPlaceV1::new(SemanticLocalIdV1::from_index(local), vec![], ty).unwrap();
        let mut types = base.types().to_vec();
        let mut locals = old.locals().to_vec();
        let mut statements = Vec::new();
        let operand = match kind {
            CallFixture::Copy | CallFixture::Intrinsic => SemanticOperandV1::Copy(place(1)),
            CallFixture::Move => SemanticOperandV1::Move(place(1)),
            CallFixture::Constant => SemanticOperandV1::Constant(SemanticConstantV1::new(
                ty,
                SemanticConstantValueV1::Scalar(SemanticScalarValueV1::new(7, 4).unwrap()),
            )),
            CallFixture::Field => {
                let tuple = SemanticTypeIdV1::from_index(1);
                types.push(SemanticTypeDeclV1::new(
                    SemanticTypeIdentityV1::from_sha256([2; 32]),
                    SemanticLayoutIdentityV1::from_sha256([3; 32]),
                    SemanticTypeLayoutV1::aggregate(
                        Some(4),
                        4,
                        SemanticAggregateLayoutV1::new(vec![0], vec![]).unwrap(),
                    )
                    .unwrap(),
                    SemanticTypeShapeV1::Tuple(SemanticAggregateTypeV1::new(vec![ty]).unwrap()),
                ));
                locals.push(SemanticLocalDeclV1::new(
                    SemanticLocalIdentityV1::from_sha256([7; 32]),
                    tuple,
                    SemanticLocalRoleV1::Temporary,
                    source,
                ));
                let tuple_place =
                    SemanticPlaceV1::new(SemanticLocalIdV1::from_index(2), vec![], tuple).unwrap();
                statements.push(SemanticStatementV1::new(
                    source,
                    SemanticStatementKindV1::Assign(SemanticAssignmentV1::new(
                        tuple_place,
                        SemanticRvalueV1::new(
                            tuple,
                            SemanticRvalueKindV1::Aggregate(
                                SemanticAggregateRvalueV1::new(
                                    SemanticAggregateKindV1::Tuple,
                                    vec![SemanticOperandV1::Copy(place(1))],
                                )
                                .unwrap(),
                            ),
                        ),
                    )),
                ));
                SemanticOperandV1::Copy(
                    SemanticPlaceV1::new(
                        SemanticLocalIdV1::from_index(2),
                        vec![
                            SemanticProjectionV1::new(SemanticProjectionKindV1::Field(0), ty)
                                .unwrap(),
                        ],
                        ty,
                    )
                    .unwrap(),
                )
            }
        };
        let intrinsic = matches!(kind, CallFixture::Intrinsic);
        let call = SemanticDirectCallV1::new_callable(
            SemanticCallableIdV1::from_index(1),
            if intrinsic { vec![] } else { vec![operand] },
            Some(SemanticCallDestinationV1::new(
                place(0),
                SemanticControlFlowEdgeV1::new(
                    SemanticEdgeRoleV1::CallReturn,
                    SemanticBlockIdV1::from_index(1),
                ),
            )),
            SemanticUnwindActionV1::Unreachable,
        )
        .unwrap();
        let blocks = vec![
            SemanticBasicBlockV1::new(
                SemanticBlockIdentityV1::from_sha256([7; 32]),
                source,
                statements,
                SemanticTerminatorV1::new(source, SemanticTerminatorKindV1::Call(call)),
            )
            .unwrap(),
            SemanticBasicBlockV1::new(
                SemanticBlockIdentityV1::from_sha256([8; 32]),
                source,
                vec![],
                SemanticTerminatorV1::new(source, SemanticTerminatorKindV1::Return),
            )
            .unwrap(),
        ];
        let caller = SemanticFunctionDeclV1::new(
            SemanticFunctionIdentityV1::from_sha256([7; 32]),
            SemanticFunctionRoleV1::KernelRoot,
            SemanticItemDefinitionIdentityV1::from_sha256([7; 32]),
            SemanticMonomorphizationIdentityV1::from_sha256([7; 32]),
            SemanticGenericTypeArgumentsIdentityV1::from_sha256([7; 32]),
            SemanticConstGenericArgumentsIdentityV1::from_sha256([7; 32]),
            source,
            old.abi().clone(),
            locals,
            SemanticBlockIdV1::from_index(0),
            blocks,
        )
        .unwrap();
        let request = if intrinsic {
            let abi = SemanticFunctionAbiV1::new(
                SemanticAbiIdentityV1::from_sha256([20; 32]),
                SemanticLayoutIdentityV1::from_sha256([20; 32]),
                SemanticCanonAbiV1::Rust,
                false,
                false,
                vec![],
                old.abi().return_value().clone(),
            )
            .unwrap();
            let binding = SemanticNonBodyCallableBindingV1::new(
                SemanticFunctionIdentityV1::from_sha256([20; 32]),
                SemanticItemDefinitionIdentityV1::from_sha256([20; 32]),
                SemanticMonomorphizationIdentityV1::from_sha256([20; 32]),
                SemanticGenericTypeArgumentsIdentityV1::from_sha256([20; 32]),
                SemanticConstGenericArgumentsIdentityV1::from_sha256([20; 32]),
                source,
                abi,
            );
            InertSemanticMirRequestV1::new_with_callables(
                base.target(),
                types,
                base.allocations().to_vec(),
                base.statics().to_vec(),
                base.vtables().to_vec(),
                vec![caller],
                vec![
                    SemanticCallableDeclV1::defined(SemanticFunctionIdV1::from_index(0)),
                    SemanticCallableDeclV1::CompilerIntrinsic {
                        binding,
                        operation: SemanticCompilerIntrinsicOperationV1::ThreadIndex(
                            SemanticAxisV1::X,
                        ),
                        operation_identity: SemanticCompilerIntrinsicIdentityV1::from_sha256(
                            [21; 32],
                        ),
                    },
                ],
                vec![SemanticFunctionIdV1::from_index(0)],
            )
            .unwrap()
        } else {
            InertSemanticMirRequestV1::new(
                base.target(),
                types,
                base.allocations().to_vec(),
                base.statics().to_vec(),
                base.vtables().to_vec(),
                vec![
                    caller,
                    old.clone()
                        .with_role(SemanticFunctionRoleV1::InternalHelper),
                ],
                vec![SemanticFunctionIdV1::from_index(0)],
            )
            .unwrap()
        };
        request
            .admit_current_production(SemanticMirLimitsV1::default())
            .unwrap()
    }

    fn inspect_call_fixture(raw: &[u8], map: &Value, function: u32, block: u32) -> Result<Value> {
        let bytes = serde_json::to_vec(map).unwrap();
        inspect_call(raw, &bytes, &digest(raw), &digest(&bytes), function, block)
    }

    #[test]
    fn defined_call_reports_copy_move_constant_and_exact_identities() {
        for (kind, label) in [
            (CallFixture::Copy, "Copy"),
            (CallFixture::Move, "Move"),
            (CallFixture::Constant, "Constant"),
        ] {
            let fixture = call_fixture(kind, SemanticSourceProvenanceV1::unavailable());
            let raw = fixture.canonical_encoding();
            let value = inspect_call_fixture(raw, &mapping(raw), 0, 0).unwrap();
            assert_eq!(value["schema"], "fe2o3-semantic-call-inspection-v1");
            assert_eq!(value["caller"]["identity"], hex(&[7; 32]));
            assert_eq!(value["callee"]["identity"], hex(&[8; 32]));
            assert_eq!(value["call"]["block_identity"], hex(&[7; 32]));
            assert_eq!(value["call"]["callable_index"], 1);
            assert_eq!(value["call"]["callee_function_index"], 1);
            let argument = &value["call"]["arguments"][0];
            assert_eq!(argument["source_argument"], 0);
            assert_eq!(argument["operand"]["kind"], label);
            assert_eq!(
                argument["operand"]["type"],
                argument["expected_source_type"]
            );
            assert_eq!(argument["semantic_type_matches"], true);
            assert_eq!(
                argument["operand"]["type"]["scalar"],
                json!({"kind": "Integer", "signed": false, "bits": 32})
            );
            assert_eq!(value["callee"]["arguments"][0]["source_argument"], 0);
            assert_eq!(value["canonical_decode_verified"], true);
            for name in [
                "execution_authority",
                "production_admission",
                "kir_binding_types_observed",
            ] {
                assert_eq!(value[name], false);
            }
        }
    }

    #[test]
    fn defined_call_reports_initialized_tuple_projection() {
        let fixture = call_fixture(
            CallFixture::Field,
            SemanticSourceProvenanceV1::unavailable(),
        );
        let raw = fixture.canonical_encoding();
        let value = inspect_call_fixture(raw, &mapping(raw), 0, 0).unwrap();
        let operand = &value["call"]["arguments"][0]["operand"];
        assert_eq!(operand["local"]["index"], 2);
        assert_eq!(operand["local"]["identity"], hex(&[7; 32]));
        assert_eq!(operand["local"]["type"]["shape"], "Tuple");
        assert_eq!(operand["projections"].as_array().unwrap().len(), 1);
        assert_eq!(operand["projections"][0]["kind"], "Field(0)");
        assert_eq!(operand["projections"][0]["result_type"], operand["type"]);
    }

    #[test]
    fn call_selector_rejects_out_of_range_and_noncall_blocks() {
        let fixture = call_fixture(CallFixture::Copy, SemanticSourceProvenanceV1::unavailable());
        let raw = fixture.canonical_encoding();
        for (function, block, error) in [
            (2, 0, "caller absent"),
            (u32::MAX, 0, "caller absent"),
            (0, 2, "call block absent"),
            (0, u32::MAX, "call block absent"),
            (0, 1, "selected terminator is not a call"),
            (1, 0, "selected terminator is not a call"),
        ] {
            assert_eq!(
                inspect_call_fixture(raw, &mapping(raw), function, block).unwrap_err(),
                error
            );
        }
    }

    #[test]
    fn call_selector_indices_are_canonical_u32() {
        assert_eq!(selector_index("0"), Ok(0));
        assert_eq!(selector_index("4294967295"), Ok(u32::MAX));
        for value in [
            "",
            "00",
            "01",
            "+1",
            "-1",
            " 1",
            "1 ",
            "1.0",
            "0x1",
            "4294967296",
            "10000000000",
        ] {
            assert!(selector_index(value).is_err(), "{value}");
        }
        let args = [
            "reader", "/unused", "/unused", "x", "y", "--other", "0", "0",
        ]
        .map(str::to_owned);
        assert!(run(&args).is_err());
        let mut args = args;
        args[5] = "--call".into();
        args[6] = "01".into();
        assert_eq!(run(&args).unwrap_err(), "expected canonical u32 index");
    }

    #[test]
    fn call_selector_refuses_admitted_nonbody_callable() {
        let fixture = call_fixture(
            CallFixture::Intrinsic,
            SemanticSourceProvenanceV1::unavailable(),
        );
        let raw = fixture.canonical_encoding();
        assert_eq!(
            inspect_call_fixture(raw, &mapping(raw), 0, 0).unwrap_err(),
            "selected call is not a defined function"
        );
    }

    #[test]
    fn call_capture_joins_sources_and_rejects_changed_inputs() {
        let file = [0x42; 32];
        let span = SemanticSourceOriginV1::new(
            SemanticSourceFileIdentityV1::from_sha256(file),
            3,
            9,
            2,
            4,
            2,
            10,
        )
        .unwrap();
        let fixture = call_fixture(
            CallFixture::Copy,
            SemanticSourceProvenanceV1::new(None, Some(span)),
        );
        let raw = fixture.canonical_encoding();
        let mut map = mapping(raw);
        assert!(inspect_call_fixture(raw, &map, 0, 0).is_err());
        map["files"] =
            json!([{"identity": hex(&file), "byte_len": 9, "display_path": "/display-only.rs"}]);
        let value = inspect_call_fixture(raw, &map, 0, 0).unwrap();
        assert_eq!(value["call"]["call_site"]["file_identity"], hex(&file));
        assert_eq!(value["call"]["call_site"]["byte_range"], json!([3, 9]));
        assert_eq!(
            value["call"]["call_site"]["source_file_contents_verified"],
            false
        );
        let bytes = serde_json::to_vec(&map).unwrap();
        assert!(inspect_call(raw, &bytes, &hex(&[0; 32]), &digest(&bytes), 0, 0).is_err());
        assert!(inspect_call(raw, &bytes, &digest(raw), &hex(&[0; 32]), 0, 0).is_err());
        map["files"][0]["identity"] = json!(hex(&[0x43; 32]));
        assert!(inspect_call_fixture(raw, &map, 0, 0).is_err());
        map["files"][0]["identity"] = json!(hex(&file));
        map["files"][0]["byte_len"] = json!(8);
        assert!(inspect_call_fixture(raw, &map, 0, 0).is_err());
        for changed in [raw[..raw.len() - 1].to_vec(), [raw, &[0]].concat()] {
            assert!(inspect_call_fixture(&changed, &mapping(&changed), 0, 0).is_err());
        }
    }

    #[test]
    fn call_diagnostic_output_and_projection_limits_are_exact() {
        assert_eq!(
            encode_output(&json!("x".repeat(MAP_MAX - 3)))
                .unwrap()
                .len(),
            MAP_MAX - 1
        );
        assert_eq!(
            encode_output(&json!("x".repeat(MAP_MAX - 2))).unwrap_err(),
            "diagnostic output limit"
        );
        let fixture = fixture();
        let map: SourceMap = serde_json::from_value(mapping(fixture.canonical_encoding())).unwrap();
        for (count, accepted) in [(64, true), (65, false)] {
            let ty = SemanticTypeIdV1::from_index(0);
            let projection =
                SemanticProjectionV1::new(SemanticProjectionKindV1::OpaqueCast, ty).unwrap();
            let place = SemanticPlaceV1::new(
                SemanticLocalIdV1::from_index(1),
                vec![projection; count],
                ty,
            )
            .unwrap();
            let result = operand_record(
                &fixture,
                &map,
                &fixture.functions()[0],
                &SemanticOperandV1::Copy(place),
            );
            assert_eq!(result.is_ok(), accepted);
        }
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
