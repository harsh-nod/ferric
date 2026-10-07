// Shared source instantiated in each transport's own wire namespace.
// No conversion between the historical peer wire and diagnostic worker wire.
const DISPATCH_TIMEOUT_MS: u32 = 60_000;

fn wire_access(access: ArgumentAccess) -> BufferAccessV1 {
    match access {
        ArgumentAccess::ReadOnly => BufferAccessV1::Read,
        ArgumentAccess::WriteOnly => BufferAccessV1::Write,
        ArgumentAccess::ReadWrite => BufferAccessV1::ReadWrite,
    }
}

pub(super) fn metadata_matches(
    expected: &InspectedKernel,
    actual: &KernelMetadataV1,
    hash: [u8; 32],
) -> bool {
    if expected.name() != actual.symbol
        || hash != actual.object_sha256
        || expected.kernarg_segment_size() != u64::from(actual.kernarg_bytes)
        || expected.kernarg_segment_alignment() != u64::from(actual.kernarg_alignment)
        || expected.group_segment_fixed_size() != u64::from(actual.group_segment_bytes)
        || expected.private_segment_fixed_size() != u64::from(actual.private_segment_bytes)
        || expected.wavefront_size() != actual.wavefront_size
        || expected.implicit_argument_offset() != actual.implicit_argument_offset.map(u64::from)
        || expected.implicit_argument_size() != u64::from(actual.implicit_argument_bytes)
        || expected.explicit_arguments().len() != actual.explicit_arguments.len()
    {
        return false;
    }
    expected
        .explicit_arguments()
        .iter()
        .zip(&actual.explicit_arguments)
        .all(|(left, right)| {
            left.offset() == u64::from(right.offset)
                && left.size() == u64::from(right.bytes)
                && (left.value_kind() == ExplicitValueKind::GlobalBuffer) == right.global_buffer
                && left.pointee_alignment() == right.pointee_alignment.map(u64::from)
                && left.access().map(wire_access) == right.access
        })
}

fn pointee_size_matches(value_type: Option<ExplicitValueType>, element_bytes: u32) -> bool {
    use ExplicitValueType::{F16, F32, F64, I8, I16, I32, I64, Struct, U8, U16, U32, U64};
    match value_type {
        None => matches!(element_bytes, 2 | 4),
        Some(I8 | U8) => element_bytes == 1,
        Some(I16 | U16 | F16) => element_bytes == 2,
        Some(I32 | U32 | F32) => element_bytes == 4,
        Some(I64 | U64 | F64) => element_bytes == 8,
        Some(Struct) => false,
    }
}

pub(super) fn pack_dispatch(
    loaded: &LoadedKernel,
    dispatch: &EngineeringTpDispatchV1,
    buffers: &BTreeMap<u64, usize>,
) -> TpResult<(CommandV1, Vec<u8>)> {
    let mut bytes = Vec::new();
    let header = pack_dispatch_into(loaded, dispatch, buffers, &mut bytes)?;
    Ok((header, bytes))
}

fn pack_dispatch_into(
    loaded: &LoadedKernel,
    dispatch: &EngineeringTpDispatchV1,
    buffers: &BTreeMap<u64, usize>,
    payload: &mut Vec<u8>,
) -> TpResult<CommandV1> {
    let metadata = &loaded.metadata;
    let workgroup = [dispatch.workgroup_size, 1, 1];
    let grid_x = dispatch
        .grid_workgroups
        .checked_mul(u32::from(dispatch.workgroup_size))
        .ok_or("grid overflow")?;
    if grid_x == 0
        || dispatch.workgroup_size == 0
        || u32::from(dispatch.workgroup_size) > metadata.max_flat_workgroup_size()
        || metadata
            .required_workgroup_size()
            .is_some_and(|required| required != workgroup.map(u32::from))
        || metadata
            .max_workgroups()
            .iter()
            .zip([dispatch.grid_workgroups, 1, 1])
            .any(|(limit, actual)| limit.is_some_and(|limit| actual > limit))
    {
        return Err("dispatch geometry contradicts compiler metadata".into());
    }
    let length =
        u32::try_from(metadata.kernarg_segment_size()).map_err(|_| "kernarg length overflow")?;
    append_kernarg_payload(payload, length, |bytes| {
        pack_dispatch_arguments(loaded, dispatch, buffers, bytes, workgroup, grid_x, length)
    })
}

// Failed packing rolls back only the new suffix; no partial batch is published.
fn append_kernarg_payload<T>(
    payload: &mut Vec<u8>,
    length: u32,
    pack: impl FnOnce(&mut [u8]) -> TpResult<T>,
) -> TpResult<T> {
    if length > wire::MAX_KERNARG_BYTES_V1 {
        return Err("kernarg bound exceeded".into());
    }
    let start = payload.len();
    let end = start
        .checked_add(length as usize)
        .ok_or("kernarg payload extent overflow")?;
    payload.resize(end, 0);
    let result = pack(&mut payload[start..]);
    if result.is_err() {
        payload.truncate(start);
    }
    result
}

fn pack_dispatch_arguments(
    loaded: &LoadedKernel,
    dispatch: &EngineeringTpDispatchV1,
    buffers: &BTreeMap<u64, usize>,
    bytes: &mut [u8],
    workgroup: [u16; 3],
    grid_x: u32,
    length: u32,
) -> TpResult<CommandV1> {
    let metadata = &loaded.metadata;
    let mut explicit = metadata.explicit_arguments().iter();
    let mut pointers = Vec::new();
    for argument in &dispatch.arguments {
        let field = explicit
            .next()
            .ok_or("missing explicit argument metadata")?;
        match *argument {
            EngineeringTpArgumentV1::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                access,
            } => {
                let wanted_access = match access {
                    EngineeringTpBufferAccessV1::Read => BufferAccessV1::Read,
                    EngineeringTpBufferAccessV1::Write => BufferAccessV1::Write,
                    EngineeringTpBufferAccessV1::ReadWrite => BufferAccessV1::ReadWrite,
                };
                let extent = elements
                    .checked_mul(element_bytes as usize)
                    .ok_or("slice extent overflow")?;
                let end = offset.checked_add(extent).ok_or("slice offset overflow")?;
                if element_bytes == 0
                    || !offset.is_multiple_of(element_bytes as usize)
                    || !buffers
                        .get(&id)
                        .is_some_and(|capacity| offset <= *capacity && end <= *capacity)
                    || field.value_kind() != ExplicitValueKind::GlobalBuffer
                    || field.size() != 8
                    || !pointee_size_matches(field.value_type(), element_bytes)
                    || field
                        .access()
                        .map(wire_access)
                        .is_some_and(|actual| actual != wanted_access)
                    || field.pointee_alignment().is_some_and(|alignment| {
                        alignment == 0 || !(offset as u64).is_multiple_of(alignment)
                    })
                {
                    return Err("slice ownership, access or physical ABI mismatch".into());
                }
                pointers.push(PointerFixupV1 {
                    kernarg_offset: u32::try_from(field.offset())
                        .map_err(|_| "pointer offset overflow")?,
                    buffer: id,
                    buffer_offset: offset as u64,
                    extent_bytes: extent as u64,
                    access: wanted_access,
                });
                let count = explicit.next().ok_or("missing slice length ABI field")?;
                put_scalar(
                    bytes,
                    count,
                    &(elements as u64).to_le_bytes(),
                    ExplicitValueType::U64,
                )?;
            }
            EngineeringTpArgumentV1::U32(value) => {
                put_scalar(bytes, field, &value.to_le_bytes(), ExplicitValueType::U32)?;
            }
            EngineeringTpArgumentV1::F32(value) => {
                if !value.is_finite() {
                    return Err("nonfinite dispatch scalar".into());
                }
                put_scalar(bytes, field, &value.to_le_bytes(), ExplicitValueType::F32)?;
            }
        }
    }
    if explicit.next().is_some() {
        return Err("extra explicit compiler arguments".into());
    }
    let header = CommandV1::Dispatch {
        kernel: loaded.id,
        payload_bytes: length,
        workgroup,
        grid: [grid_x, 1, 1],
        pointers,
        timeout_ms: DISPATCH_TIMEOUT_MS,
    };
    header.payload_bytes().map_err(|error| error.to_string())?;
    Ok(header)
}

fn put_scalar(
    bytes: &mut [u8],
    field: &ExplicitArgument,
    value: &[u8],
    expected_type: ExplicitValueType,
) -> TpResult<()> {
    if field.value_kind() != ExplicitValueKind::ByValue
        || field.size() != value.len() as u64
        || field.value_type().is_some_and(|kind| kind != expected_type)
    {
        return Err("scalar physical ABI mismatch".into());
    }
    let start = usize::try_from(field.offset()).map_err(|_| "scalar offset overflow")?;
    let end = start
        .checked_add(value.len())
        .ok_or("scalar end overflow")?;
    bytes
        .get_mut(start..end)
        .ok_or("scalar exceeds kernarg extent")?
        .copy_from_slice(value);
    Ok(())
}
