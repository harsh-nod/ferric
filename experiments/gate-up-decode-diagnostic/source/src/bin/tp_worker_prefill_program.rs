//! Fixed native prefill shape. No inference policy is added to the runtime.
use super::{
    BTreeMap, BufferAccessV1, CommandV1, EngineeringTpArgumentV1, EngineeringTpDispatchV1,
    LoadedKernel, OrderedBatchDispatchV1, Plan, Slot, TokenProgramDefinitionV1, TpResult, Update,
    pack_dispatch_into, scalar,
};
use ferric_m1_engineering_execution_v1::tp_artifact::{
    ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27, ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14,
    ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15,
};

pub(super) fn require_command_shape(
    index: usize,
    command: &EngineeringTpDispatchV1,
    rows: u32,
) -> TpResult<()> {
    if !matches!(rows, 16 | 32) {
        return Err("prefill row shape is not admitted".into());
    }
    let (kernel, grid, scalar_fields): (&str, u32, Vec<(usize, u32)>) = if index == 0 {
        (
            "ferric_qwen3_tp_batch32_embedding_bf16_v5",
            1024,
            vec![(3, 16)],
        )
    } else {
        match (index - 1) % 17 {
            0 | 11 => (
                ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0],
                16,
                vec![(5, 16), (6, 4096)],
            ),
            1 | 2 | 3 | 12 | 13 => {
                let role = (index - 1) % 17;
                let (n, tag) = match role {
                    1 => (4096, 1),
                    2 => (1024, 2),
                    3 => (1024, 3),
                    12 => (12288, 4),
                    _ => (12288, 5),
                };
                (
                    "ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5",
                    n / 16,
                    vec![(3, 16), (4, n), (5, 4096), (6, 1), (7, tag)],
                )
            }
            4 => ("qwen3_rmsnorm_v1", 512, vec![(5, 512), (6, 128)]),
            5 => ("qwen3_rmsnorm_v1", 128, vec![(5, 128), (6, 128)]),
            6 => ("ferric_qwen3_tp_batch32_rope_v5", 16, vec![(7, 16), (8, 1)]),
            7 => (ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0], 256, vec![]),
            8 => (
                ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0],
                512,
                vec![(6, 16), (7, 1)],
            ),
            9 => (
                "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5",
                256,
                vec![(3, 16), (4, 4096), (5, 4096), (6, 1), (7, 1)],
            ),
            10 | 16 => (
                "ferric_qwen3_tp_batch32_residual_bf16_v5",
                1024,
                vec![(3, 16)],
            ),
            14 => (
                "ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5",
                3072,
                vec![(3, 16), (4, 1)],
            ),
            15 => (
                "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5",
                256,
                vec![(3, 16), (4, 4096), (5, 12288), (6, 1), (7, 2)],
            ),
            _ => return Err("prefill layer role outside fixed graph".into()),
        }
    };
    let copy = kernel == ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
    let grid = if copy { grid } else { grid * (rows / 16) };
    if command.kernel != kernel || command.grid_workgroups != grid || command.workgroup_size != 64 {
        return Err("prefill kernel roster or launch geometry changed".into());
    }
    for (field, expected) in scalar_fields {
        let role = index.checked_sub(1).map(|index| index % 17);
        let row_field = if index == 0 {
            field == 3
        } else {
            match role {
                Some(0 | 4 | 5 | 11) => field == 5,
                Some(6) => field == 7,
                Some(8) => field == 6,
                _ => field == 3,
            }
        };
        let expected = if row_field {
            expected * (rows / 16)
        } else {
            expected
        };
        if scalar(&command.arguments, field)? != expected {
            return Err("prefill static scalar geometry changed".into());
        }
    }
    Ok(())
}

pub(super) fn plan_prefill(
    dispatches: &[EngineeringTpDispatchV1],
    kernels: &BTreeMap<String, LoadedKernel>,
    buffers: &BTreeMap<u64, usize>,
) -> TpResult<Plan> {
    if dispatches.len() != 613 {
        return Err("prefill program requires exactly 613 commands".into());
    }
    let mut result = Plan {
        definition: TokenProgramDefinitionV1 {
            dispatches: Vec::new(),
            slots: Vec::new(),
        },
        kernargs: Vec::new(),
        updates: Vec::new(),
    };
    let mut page_shape = None;
    let mut context = None;
    for (index, dispatch) in dispatches.iter().enumerate() {
        require_command_shape(index, dispatch, 16)?;
        let dispatch_index = u16::try_from(index).map_err(|_| "prefill dispatch index")?;
        let copy = dispatch.kernel == ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
        let attention = dispatch.kernel == ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0];
        if copy != (index > 0 && (index - 1) % 17 == 7)
            || attention != (index > 0 && (index - 1) % 17 == 8)
        {
            return Err("prefill copy/attention layer roster changed".into());
        }
        if copy {
            let first = scalar(&dispatch.arguments, 4)?;
            let page = scalar(&dispatch.arguments, 5)?;
            let pages = scalar(&dispatch.arguments, 6)?;
            let rotation = scalar(&dispatch.arguments, 7)?;
            let shape = (first, page, pages, rotation);
            if dispatch.arguments.len() != 8
                || first > 8176
                || first % 16 != 0
                || !(1..=512).contains(&pages)
                || page >= pages
                || rotation > 1
                || dispatch.grid_workgroups != 256
                || dispatch.workgroup_size != 64
                || page_shape.is_some_and(|previous| previous != shape)
            {
                return Err("prefill complete-page geometry changed".into());
            }
            page_shape = Some(shape);
        }
        if attention {
            let current = scalar(&dispatch.arguments, 10)?;
            if dispatch.arguments.len() != 11
                || scalar(&dispatch.arguments, 6)? != 16
                || scalar(&dispatch.arguments, 7)? != 1
                || scalar(&dispatch.arguments, 9)?
                    != page_shape.ok_or("prefill page precedes attention")?.2
                || !(1..=512).contains(&scalar(&dispatch.arguments, 8)?)
                || current > scalar(&dispatch.arguments, 8)? * 16
                || !(16..=8192).contains(&current)
                || context.is_some_and(|previous| previous != current)
            {
                return Err("prefill attention geometry changed".into());
            }
            context = Some(current);
        }
        let loaded = kernels
            .get(dispatch.kernel)
            .ok_or("unloaded prefill kernel")?;
        let header = pack_dispatch_into(loaded, dispatch, buffers, &mut result.kernargs)?;
        let CommandV1::Dispatch {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
            ..
        } = header
        else {
            return Err("prefill dispatch packing changed".into());
        };
        let entry = OrderedBatchDispatchV1 {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
        };
        if copy {
            let (_, page, _, _) = page_shape.expect("copy shape");
            for pointer in [2usize, 3] {
                let fixup = entry.pointers.get(pointer).ok_or("prefill copy pointers")?;
                if fixup.buffer_offset != u64::from(page) * 32768
                    || fixup.extent_bytes != 32768
                    || fixup.access != BufferAccessV1::Write
                {
                    return Err("prefill pointer does not match its owned complete page".into());
                }
            }
        }
        let mut fields = loaded.metadata.explicit_arguments().iter();
        let mut pointer_index = 0u16;
        for (argument, value) in dispatch.arguments.iter().enumerate() {
            let field = fields.next().ok_or("prefill explicit ABI roster")?;
            match value {
                EngineeringTpArgumentV1::Buffer { id, .. } => {
                    fields.next().ok_or("prefill slice ABI")?;
                    if copy && matches!(argument, 2 | 3) {
                        let fixup = &entry.pointers[usize::from(pointer_index)];
                        let maximum_offset = (*buffers.get(id).ok_or("prefill buffer ownership")?
                            as u64)
                            .checked_sub(fixup.extent_bytes)
                            .ok_or("prefill pointer extent")?;
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
                    if (copy && matches!(argument, 4 | 5 | 7)) || (attention && argument == 10) =>
                {
                    let (minimum, maximum) = if attention {
                        (16, 8192)
                    } else if argument == 4 {
                        (0, 8176)
                    } else if argument == 5 {
                        (0, scalar(&dispatch.arguments, 6)? - 1)
                    } else {
                        (0, 1)
                    };
                    result.definition.slots.push(Slot::ScalarU32 {
                        dispatch: dispatch_index,
                        offset: u32::try_from(field.offset())
                            .map_err(|_| "prefill scalar offset")?,
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
    if result.definition.slots.len() != 216 || page_shape.map(|shape| shape.0 + 16) != context {
        return Err("prefill slot roster or page/context agreement changed".into());
    }
    Ok(result)
}
