//! Closed two-page prefill graph; the 512-slot wire family is explicit.
use super::{
    BTreeMap, BufferAccessV1, CommandV1, EngineeringTpArgumentV1, EngineeringTpDispatchV1,
    LoadedKernel, OrderedBatchDispatchV1, Plan, Slot, TokenProgramDefinitionV1, TpResult, Update,
    pack_dispatch_into, scalar,
};

pub(super) fn plan_prefill32(
    dispatches: &[EngineeringTpDispatchV1],
    kernels: &BTreeMap<String, LoadedKernel>,
    buffers: &BTreeMap<u64, usize>,
) -> TpResult<Plan> {
    if dispatches.len() != 649 {
        return Err("prefill32 requires the complete 649-command graph".into());
    }
    let mut result = Plan {
        definition: TokenProgramDefinitionV1 {
            dispatches: Vec::with_capacity(649),
            slots: Vec::with_capacity(396),
        },
        kernargs: Vec::new(),
        updates: Vec::with_capacity(396),
    };
    let mut page_shapes = [None; 2];
    let mut layer_buffers: Option<Vec<u64>> = None;
    let mut context = None;
    for (index, dispatch) in dispatches.iter().enumerate() {
        let dispatch_index = u16::try_from(index).map_err(|_| "prefill32 dispatch index")?;
        let role = index.checked_sub(1).map(|value| value % 18);
        let old_index = if index == 0 {
            0
        } else {
            let role = role.expect("layer role");
            1 + if role >= 8 { role - 1 } else { role }
        };
        super::prefill::require_command_shape(old_index, dispatch, 32)?;
        let copy_half = match role {
            Some(7) => Some(0usize),
            Some(8) => Some(1usize),
            _ => None,
        };
        let attention = role == Some(9);
        if let Some(half) = copy_half {
            let first = scalar(&dispatch.arguments, 4)?;
            let page = scalar(&dispatch.arguments, 5)?;
            let pages = scalar(&dispatch.arguments, 6)?;
            let rotation = scalar(&dispatch.arguments, 7)?;
            let shape = (first, page, pages, rotation);
            if dispatch.arguments.len() != 8
                || first > 8176
                || first % 16 != 0
                || !(2..=512).contains(&pages)
                || page >= pages
                || rotation > u32::from(half == 0)
                || page_shapes[half].is_some_and(|previous| previous != shape)
            {
                return Err("prefill32 page geometry differs across the complete graph".into());
            }
            page_shapes[half] = Some(shape);
            if half == 1 {
                let first_half = page_shapes[0].ok_or("prefill32 missing first half")?;
                let terminal = first_half.3 == 1;
                let low = if terminal { first } else { first_half.0 };
                if low > 8160
                    || first_half.2 != pages
                    || first_half.1 == page
                    || (if terminal {
                        first.checked_add(16) != Some(first_half.0)
                    } else {
                        first_half.0.checked_add(16) != Some(first)
                    })
                {
                    return Err("prefill32 halves are not distinct contiguous logical pages".into());
                }
            }
        }
        if attention {
            let current = scalar(&dispatch.arguments, 10)?;
            let left = page_shapes[0].ok_or("prefill32 first copy missing")?;
            let right = page_shapes[1].ok_or("prefill32 second copy missing")?;
            let stride = scalar(&dispatch.arguments, 8)?;
            if dispatch.arguments.len() != 11
                || scalar(&dispatch.arguments, 9)? != left.2
                || !(2..=512).contains(&stride)
                || current > stride * 16
                || current != left.0.min(right.0) + 32
                || !(32..=8192).contains(&current)
                || context.is_some_and(|previous| previous != current)
            {
                return Err("prefill32 attention context/page-table geometry changed".into());
            }
            context = Some(current);
        }
        let loaded = kernels
            .get(dispatch.kernel)
            .ok_or("unloaded prefill32 kernel")?;
        let CommandV1::Dispatch {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
            ..
        } = pack_dispatch_into(loaded, dispatch, buffers, &mut result.kernargs)?
        else {
            return Err("prefill32 packing contract changed".into());
        };
        let entry = OrderedBatchDispatchV1 {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
        };
        if attention {
            let ids = layer_buffers
                .as_ref()
                .ok_or("prefill32 copy ownership missing")?;
            if entry
                .pointers
                .get(1)
                .is_none_or(|pointer| pointer.buffer != ids[2])
                || entry
                    .pointers
                    .get(2)
                    .is_none_or(|pointer| pointer.buffer != ids[3])
            {
                return Err("prefill32 attention must read the copied layer pages".into());
            }
            let pages = scalar(&dispatch.arguments, 9)?;
            let stride = scalar(&dispatch.arguments, 8)?;
            for (pointer, bytes) in [
                (1, u64::from(pages) * 32768),
                (2, u64::from(pages) * 32768),
                (3, 32 * 4),
                (4, 32 * u64::from(stride) * 4),
            ] {
                if entry.pointers.get(pointer).is_none_or(|fixup| {
                    fixup.buffer_offset != 0
                        || fixup.extent_bytes != bytes
                        || fixup.access != BufferAccessV1::Read
                }) {
                    return Err("prefill32 attention page/position/table extent differs".into());
                }
            }
        }
        if let Some(half) = copy_half {
            let page = page_shapes[half].expect("validated page").1;
            if entry.pointers.len() != 4 {
                return Err("prefill32 pointer roster".into());
            }
            let ids = entry
                .pointers
                .iter()
                .map(|pointer| pointer.buffer)
                .collect::<Vec<_>>();
            let base = 1 + (index - 1) / 18 * 18;
            for (source, command, argument) in [(0, base + 6, 6), (1, base + 3, 2)] {
                if !matches!(dispatches[command].arguments.get(argument), Some(EngineeringTpArgumentV1::Buffer { id, .. }) if *id == ids[source])
                {
                    return Err("prefill32 copy source must be its layer RoPE/value output".into());
                }
            }
            if ids
                .iter()
                .enumerate()
                .any(|(index, id)| *id == 0 || ids[index + 1..].contains(id))
                || (half == 1 && layer_buffers.as_ref() != Some(&ids))
            {
                return Err("prefill32 half allocation ownership differs".into());
            }
            if half == 0 {
                layer_buffers = Some(ids);
            }
            for (pointer, fixup) in entry.pointers.iter().enumerate() {
                let source = pointer < 2;
                if fixup.buffer_offset
                    != if source {
                        half as u64 * 32768
                    } else {
                        u64::from(page) * 32768
                    }
                    || fixup.extent_bytes != 32768
                    || fixup.access
                        != if source {
                            BufferAccessV1::Read
                        } else {
                            BufferAccessV1::Write
                        }
                {
                    return Err(
                        "prefill32 pointer must bind its exact source half or owned page".into(),
                    );
                }
            }
        }
        let mut fields = loaded.metadata.explicit_arguments().iter();
        let mut pointer_index = 0u16;
        for (argument, value) in dispatch.arguments.iter().enumerate() {
            let field = fields.next().ok_or("prefill32 explicit ABI roster")?;
            match value {
                EngineeringTpArgumentV1::Buffer { id, .. } => {
                    fields.next().ok_or("prefill32 slice ABI")?;
                    if copy_half.is_some() && matches!(argument, 2 | 3) {
                        let fixup = &entry.pointers[usize::from(pointer_index)];
                        let maximum_offset =
                            (*buffers.get(id).ok_or("prefill32 buffer ownership")? as u64)
                                .checked_sub(fixup.extent_bytes)
                                .ok_or("prefill32 pointer extent")?;
                        result.definition.slots.push(Slot::Pointer {
                            dispatch: dispatch_index,
                            pointer: pointer_index,
                            buffers: vec![*id],
                            maximum_offset,
                        });
                        result.updates.push(Update::Pointer {
                            buffer: *id,
                            offset: fixup.buffer_offset,
                        });
                    }
                    pointer_index += 1;
                }
                EngineeringTpArgumentV1::U32(value)
                    if (copy_half.is_some() && matches!(argument, 4 | 5 | 7))
                        || (attention && argument == 10) =>
                {
                    let (minimum, maximum) = if attention {
                        (32, 8192)
                    } else if argument == 4 {
                        (0, 8176)
                    } else if argument == 5 {
                        (0, scalar(&dispatch.arguments, 6)? - 1)
                    } else {
                        (0, u32::from(copy_half == Some(0)))
                    };
                    result.definition.slots.push(Slot::ScalarU32 {
                        dispatch: dispatch_index,
                        offset: u32::try_from(field.offset())
                            .map_err(|_| "prefill32 scalar offset")?,
                        minimum,
                        maximum,
                    });
                    result.updates.push(Update::ScalarU32 { value: *value });
                }
                _ => {}
            }
        }
        result.definition.dispatches.push(entry);
    }
    if result.definition.slots.len() != 396 || context.is_none() {
        return Err("prefill32 dynamic slot roster changed".into());
    }
    Ok(result)
}
