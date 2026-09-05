use quote::ToTokens as _;
use syn::{FnArg, Item, ItemFn};

const SOURCE: &str = include_str!("../../qwen3-all-kernels-v1/src/gemm.rs");

fn kernels() -> Vec<ItemFn> {
    syn::parse_file(SOURCE)
        .expect("device source parses as ordinary Rust")
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Fn(function)
                if function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("kernel")) =>
            {
                Some(function)
            }
            _ => None,
        })
        .collect()
}

fn compact_type(argument: &FnArg) -> String {
    let FnArg::Typed(argument) = argument else {
        panic!("device roots cannot have a receiver");
    };
    argument
        .ty
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn compact_tokens(tokens: impl quote::ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

#[test]
fn source_has_exact_three_root_roster_and_no_worker_escape_hatch() {
    let kernels = kernels();
    assert_eq!(kernels.len(), 3);
    assert_eq!(
        kernels
            .iter()
            .map(|function| function.sig.ident.to_string())
            .collect::<Vec<_>>(),
        vec![
            String::from("ferric_qwen3_gemm_reference_bf16_f32_bf16_v1"),
            String::from("ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1"),
            String::from("ferric_qwen3_token_embedding_bf16_copy_v1"),
        ]
    );

    let lowercase = SOURCE.to_ascii_lowercase();
    for forbidden in [
        "compilerhandoff",
        "pinnedworker",
        "std::process",
        "command::new",
        "include_bytes!",
        "llvm assembly",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "found forbidden marker {forbidden}"
        );
    }
}

#[test]
fn matrix_roots_are_read_read_readwrite_and_embedding_is_write_only() {
    let kernels = kernels();
    for matrix in &kernels[..2] {
        assert_eq!(matrix.sig.inputs.len(), 7);
        assert_eq!(compact_type(&matrix.sig.inputs[0]), "&[u16]");
        assert_eq!(compact_type(&matrix.sig.inputs[1]), "&[u16]");
        assert_eq!(
            compact_type(&matrix.sig.inputs[2]),
            "DisjointSlice<u16,Tiled2D<Index1D,64,16,16,4>>"
        );
        for scalar in matrix.sig.inputs.iter().skip(3) {
            assert_eq!(compact_type(scalar), "u32");
        }
    }

    let embedding = &kernels[2];
    assert_eq!(embedding.sig.inputs.len(), 6);
    assert_eq!(compact_type(&embedding.sig.inputs[0]), "&[u32]");
    assert_eq!(compact_type(&embedding.sig.inputs[1]), "&[u16]");
    assert_eq!(
        compact_type(&embedding.sig.inputs[2]),
        "WriteOnlyDisjointSlice<u16>"
    );
    assert!(matches!(&embedding.sig.output, syn::ReturnType::Default));
}

#[test]
fn generated_host_adapter_types_preserve_exact_kfd_effects() {
    use fe2o3_host::__generated::{
        CompilerGeneratedKernelExpectationV1, CompilerGeneratedKfdArguments, GeneratedKfdReadSlice,
        GeneratedKfdReadWriteSlice, GeneratedKfdWriteSlice,
    };
    use ferric_qwen3_gemm_device_v1::{
        ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu as reference,
        ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu as vectorized,
        ferric_qwen3_token_embedding_bf16_copy_v1_gpu as embedding,
    };

    fn assert_kfd_adapter<'allocation, K, A>()
    where
        K: CompilerGeneratedKernelExpectationV1,
        A: CompilerGeneratedKfdArguments<'allocation, K>,
    {
    }

    type ReadU16 = GeneratedKfdReadSlice<'static, u16>;
    type ReadWriteU16 = GeneratedKfdReadWriteSlice<'static, u16>;
    type ReadU32 = GeneratedKfdReadSlice<'static, u32>;
    type WriteU16 = GeneratedKfdWriteSlice<'static, u16>;

    assert_kfd_adapter::<
        reference::Marker,
        reference::Arguments<'static, ReadU16, ReadU16, ReadWriteU16>,
    >();
    assert_kfd_adapter::<
        vectorized::Marker,
        vectorized::Arguments<'static, ReadU16, ReadU16, ReadWriteU16>,
    >();
    assert_kfd_adapter::<
        embedding::Marker,
        embedding::Arguments<'static, ReadU32, ReadU16, WriteU16>,
    >();

    let a = [0_u16; 1];
    let b = [0_u16; 1];
    let mut reference_c = [0_u16; 1];
    let _reference_arguments = reference::Arguments::new(
        GeneratedKfdReadSlice::new(&a),
        GeneratedKfdReadSlice::new(&b),
        GeneratedKfdReadWriteSlice::new(&mut reference_c),
        1,
        4_096,
        4_096,
        0,
    );

    let mut vectorized_c = [0_u16; 1];
    let _vectorized_arguments = vectorized::Arguments::new(
        GeneratedKfdReadSlice::new(&a),
        GeneratedKfdReadSlice::new(&b),
        GeneratedKfdReadWriteSlice::new(&mut vectorized_c),
        16,
        2_048,
        1_024,
        0,
    );

    let tokens = [0_u32; 1];
    let weight = [0_u16; 1];
    let mut output = [0_u16; 1];
    let _embedding_arguments = embedding::Arguments::new(
        GeneratedKfdReadSlice::new(&tokens),
        GeneratedKfdReadSlice::new(&weight),
        GeneratedKfdWriteSlice::new(&mut output),
        1,
        4_096,
        151_936,
    );
}

#[test]
fn source_keeps_separate_reference_and_a4_reduction_steps() {
    let kernels = kernels();
    let reference = kernels[0].block.to_token_stream().to_string();
    let vectorized = kernels[1].block.to_token_stream().to_string();
    assert!(reference.contains("reduction += 1"));
    assert!(!reference.contains("reduction += 4"));
    assert!(vectorized.contains("reduction_wide += 4"));
    assert!(!vectorized.contains("reduction += 1"));
    assert!(vectorized.contains("if reduction + 3 >= k"));
    for removed_macro in [
        "reference_profile_is_admitted_v1 !",
        "vectorized_profile_is_admitted_v1 !",
        "reference_component_v1 !",
        "vectorized_component_v1 !",
        "accumulate_one_v1 !",
    ] {
        assert!(!reference.contains(removed_macro));
        assert!(!vectorized.contains(removed_macro));
    }
}

#[test]
fn matrix_roots_use_no_call_sequential_indices_after_finite_caps() {
    let kernels = kernels();
    for (index, matrix) in kernels[..2].iter().enumerate() {
        let body = compact_tokens(&kernels[index].block);
        let profile_guard = body
            .find("if!profile_is_admitted{fe2o3_device::trap();}")
            .expect("matrix rejects profiles before index arithmetic");
        let mut previous_cap = profile_guard;
        for cap in [
            "ifm<2_049{}else{fe2o3_device::trap();}",
            "ifn<151_937{}else{fe2o3_device::trap();}",
            "ifk<12_289{}else{fe2o3_device::trap();}",
        ] {
            let position = body.find(cap).expect("matrix pins every finite dimension cap");
            assert!(previous_cap < position);
            previous_cap = position;
        }
        let column_guard = body
            .find("ifcolumn<n{}else{fe2o3_device::trap();}")
            .expect("matrix authenticates columns before index arithmetic");
        let loop_source = if index == 0 {
            "whilereduction<k{"
        } else {
            "whilereduction_wide<reduction_bound{"
        };
        let loop_start = body
            .find(loop_source)
            .expect("matrix has its bounded reduction loop");
        let root_row_cap = body
            .find("ifrow_base<2_045{}else{fe2o3_device::trap();}")
            .expect("matrix caps the row base before component arithmetic");
        assert!(profile_guard < column_guard);
        assert!(column_guard < loop_start);
        assert!(!body.contains("qwen3_matrix_"));
        let mut previous_row = root_row_cap;
        for component in 0..4 {
            let source = if component == 0 {
                String::from("letrow_0=row_base;")
            } else {
                format!("letrow_{component}=row_{}+1;", component - 1)
            };
            let row = body.find(&source).unwrap_or_else(|| {
                panic!("missing sequential bounded row component {component}")
            });
            let active = body
                .find(&format!("letactive_{component}=row_{component}<m&&column<n;"))
                .unwrap_or_else(|| panic!("missing access gate for row component {component}"));
            assert!(previous_row < row);
            assert!(row < active);
            assert!(active < loop_start);
            previous_row = row;
        }
        if index == 0 {
            assert_eq!(body.matches("letright_index=reduction*n+column;").count(), 1);
            assert_eq!(body.matches("letleft_index=row_").count(), 4);
        } else {
            let nonzero_guard = body
                .find("ifm==0||n==0||k==0{fe2o3_device::trap();}")
                .expect("A4 matrix rejects zero dimensions");
            let k_cap = body
                .find("ifk<12_289{}else{fe2o3_device::trap();}")
                .expect("A4 matrix pins its maximum admitted k");
            let divisible_by_four = body
                .find("ifk%4!=0{fe2o3_device::trap();}")
                .expect("A4 matrix rejects partial reduction groups");
            let widened_bound = body
                .find("letreduction_bound=kasu64;")
                .expect("A4 matrix widens its u32 bound before the usize conversion");
            let body_index = body
                .find("letreduction=reduction_wideasusize;")
                .expect("A4 matrix narrows its proved body index for address arithmetic");
            let reduction_guard = body
                .find("ifreduction+3>=k{fe2o3_device::trap();}")
                .expect("A4 matrix authenticates its full reduction group");
            let first_index = body
                .find("letright_index_0=reduction*n+column;")
                .expect("A4 matrix starts B indexing from one bounded base");
            let backedge = body
                .find("reduction_wide+=4;")
                .expect("A4 matrix has its non-unit backedge");
            assert!(profile_guard < widened_bound);
            assert!(widened_bound < nonzero_guard);
            assert!(nonzero_guard < k_cap);
            assert!(k_cap < divisible_by_four);
            assert!(divisible_by_four < loop_start);
            assert!(loop_start < body_index);
            assert!(body_index < reduction_guard);
            assert!(reduction_guard < first_index);
            assert!(first_index < backedge);
            assert_eq!(body.matches("letleft_index_0=row_").count(), 4);
            assert_eq!(body.matches("letleft_index_1=left_index_0+1;").count(), 4);
            assert_eq!(body.matches("letleft_index_2=left_index_1+1;").count(), 4);
            assert_eq!(body.matches("letleft_index_3=left_index_2+1;").count(), 4);
        }
        assert_eq!(compact_tokens(matrix).matches("qwen3_matrix_").count(), 0);
    }
}

#[test]
fn reduction_ast_pins_each_load_pair_and_ascending_accumulation() {
    let kernels = kernels();
    let reference = compact_tokens(&kernels[0].block);
    let vector = compact_tokens(&kernels[1].block);

    fn reduction_loop<'a>(body: &'a str, step: &str) -> &'a str {
        let start = body
            .find("while")
            .expect("kernel has a reduction loop");
        let end = body[start..]
            .find(step)
            .map(|offset| start + offset)
            .expect("reduction loop has the expected step");
        &body[start..end]
    }

    fn active_branch<'a>(body: &'a str, component: usize) -> &'a str {
        let marker = format!("ifactive_{component}{{");
        let start = body
            .find(&marker)
            .unwrap_or_else(|| panic!("missing {marker}"));
        let end = if component == 3 {
            body.len()
        } else {
            let next = format!("ifactive_{}{{", component + 1);
            body[start..]
                .find(&next)
                .map(|offset| start + offset)
                .unwrap_or_else(|| panic!("missing {next}"))
        };
        &body[start..end]
    }

    let reference_loop = reduction_loop(&reference, "reduction+=1;");
    let right_index = reference_loop
        .find("letright_index=reduction*n+column;")
        .expect("reference computes one B index");
    let right_load = reference_loop
        .find("volatile_load(b,right_index)")
        .expect("reference loads B through its bounded index");
    assert!(right_index < right_load);
    for component in 0..4 {
        let branch = active_branch(reference_loop, component);
        let index = format!("letleft_index=row_{component}*k+reduction;");
        let load = "volatile_load(a,left_index)";
        let accumulation = format!("accumulator_{component}=accumulator_{component}+left*right;");
        assert_eq!(branch.matches(&index).count(), 1);
        assert_eq!(branch.matches(&load).count(), 1);
        assert_eq!(branch.matches(&accumulation).count(), 1);
        assert!(branch.find(&index).unwrap() < branch.find(load).unwrap());
        assert!(branch.find(&load).unwrap() < branch.find(&accumulation).unwrap());
    }

    let vector_loop = reduction_loop(&vector, "reduction_wide+=4;");
    let mut previous_right = 0;
    for offset in 0..4 {
        let index = if offset == 0 {
            String::from("letright_index_0=reduction*n+column;")
        } else {
            format!("letright_index_{offset}=right_index_{}+n;", offset - 1)
        };
        let load = format!("volatile_load(b,right_index_{offset})");
        let index_position = vector_loop
            .find(&index)
            .unwrap_or_else(|| panic!("missing ordered B index {index}"));
        let position = vector_loop
            .find(&load)
            .unwrap_or_else(|| panic!("missing ordered B load {load}"));
        assert!(index_position < position);
        assert!(offset == 0 || previous_right < position);
        previous_right = position;
    }
    for component in 0..4 {
        let branch = active_branch(vector_loop, component);
        let mut previous_accumulation = 0;
        for offset in 0..4 {
            let index = if offset == 0 {
                format!("letleft_index_0=row_{component}*k+reduction;")
            } else {
                format!("letleft_index_{offset}=left_index_{}+1;", offset - 1)
            };
            let load = format!("volatile_load(a,left_index_{offset})");
            let accumulation = format!(
                "accumulator_{component}=accumulator_{component}+left_{offset}*right_{offset};"
            );
            assert_eq!(branch.matches(&index).count(), 1);
            assert_eq!(branch.matches(&load).count(), 1);
            assert_eq!(branch.matches(&accumulation).count(), 1);
            let index_position = branch.find(&index).unwrap();
            let load_position = branch.find(&load).unwrap();
            let accumulation_position = branch.find(&accumulation).unwrap();
            assert!(index_position < load_position);
            assert!(load_position < accumulation_position);
            assert!(offset == 0 || previous_accumulation < accumulation_position);
            previous_accumulation = accumulation_position;
        }
    }
}

#[test]
fn every_shared_matrix_and_embedding_read_uses_a_bounded_volatile_load() {
    assert_eq!(SOURCE.matches("memory::volatile_load").count(), 27);
    assert_eq!(
        SOURCE.matches("memory::volatile_load(tokens, row)").count(),
        1
    );
    assert_eq!(
        SOURCE
            .matches("memory::volatile_load(weight, weight_index)")
            .count(),
        1
    );
    for forbidden in ["StridedReadView2D", "a[", "b[", "tokens[", "weight["] {
        assert!(
            !SOURCE.contains(forbidden),
            "shared input bypasses bounded volatile load with {forbidden}"
        );
    }

    let kernels = kernels();
    for (matrix, loads_per_active_row) in kernels[..2].iter().zip([1_usize, 4]) {
        let body = matrix.block.to_token_stream().to_string();
        let loop_start = body
            .find(if loads_per_active_row == 1 {
                "while reduction < k"
            } else {
                "while reduction_wide < reduction_bound"
            })
            .expect("matrix has the ascending reduction loop");
        let loop_end = body[loop_start..]
            .find(if loads_per_active_row == 1 {
                "reduction += 1"
            } else {
                "reduction_wide += 4"
            })
            .map(|offset| loop_start + offset)
            .expect("matrix loop has the expected step");
        let reduction_loop = &body[loop_start..loop_end];

        let mut active_offsets = Vec::new();
        for component in 0..4 {
            let marker = format!("if active_{component}");
            active_offsets.push(
                reduction_loop
                    .find(&marker)
                    .unwrap_or_else(|| panic!("missing {marker} in reduction loop")),
            );
        }
        assert!(
            active_offsets
                .windows(2)
                .all(|window| window[0] < window[1])
        );
        assert!(!reduction_loop[..active_offsets[0]].contains("volatile_load (a"));

        for component in 0..4 {
            let end = active_offsets
                .get(component + 1)
                .copied()
                .unwrap_or(reduction_loop.len());
            let active_body = &reduction_loop[active_offsets[component]..end];
            assert_eq!(
                active_body.matches("volatile_load (a").count(),
                loads_per_active_row,
                "active row {component} has the wrong A-load count"
            );
        }
    }
}

#[test]
fn source_pins_finite_divisors_and_a4_reduction_guards() {
    for (extent_source, extent, divisor_source, divisor) in [
        ("1_024", 1_024_usize, "64", 64_usize),
        ("2_048", 2_048, "128", 128),
        ("3_072", 3_072, "192", 192),
        ("4_096", 4_096, "256", 256),
        ("12_288", 12_288, "768", 768),
        ("151_936", 151_936, "9_496", 9_496),
    ] {
        assert_eq!(divisor, extent.div_ceil(16));
        let branch = format!(
            "n == {extent_source} {{\n        (tile_index / {divisor_source}, tile_index % {divisor_source})"
        );
        assert_eq!(
            SOURCE.matches(&branch).count(),
            2,
            "missing finite {extent_source} divisor"
        );
    }
    assert!(SOURCE.contains("if k % 4 != 0"));
    assert!(SOURCE.contains("if reduction + 3 >= k"));
    assert_eq!(SOURCE.matches("output_index / 4_096").count(), 1);
    assert_eq!(SOURCE.matches("output_index / 1_024").count(), 1);
}

#[test]
fn matrix_roots_authenticate_columns_before_arithmetic_and_b_reads() {
    let kernels = kernels();
    for matrix in &kernels[..2] {
        let body = compact_tokens(&matrix.block);
        let tile_guard = body
            .find("iftile_column<tiles_per_row{}else{fe2o3_device::trap();}")
            .expect("matrix authenticates the finite tile-column remainder");
        let tile_cap = body
            .find("iftile_column<9_496{}else{fe2o3_device::trap();}")
            .expect("matrix caps tile columns before bounded arithmetic");
        let tile_row_cap = body
            .find("iftile_row<128{}else{fe2o3_device::trap();}")
            .expect("matrix caps tile rows before bounded arithmetic");
        let row_base = body
            .find("letrow_base=tile_row*16+(lane/16)*4;")
            .expect("matrix computes its lane row base");
        let row_cap = body
            .find("ifrow_base<2_045{}else{fe2o3_device::trap();}")
            .expect("matrix caps its row base before component offsets");
        let column = body
            .find("letcolumn=tile_column*16+lane%16;")
            .expect("matrix computes its lane column");
        let column_guard = body
            .find("ifcolumn<n{}else{fe2o3_device::trap();}")
            .expect("matrix authenticates the exact B column");
        let first_b_read = body
            .find("volatile_load(b,")
            .expect("matrix reads its B input");

        assert!(tile_guard < tile_cap);
        assert!(tile_guard < tile_row_cap);
        assert!(tile_row_cap < row_base);
        assert!(tile_cap < row_base);
        assert!(row_base < row_cap);
        assert!(row_cap < column);
        assert!(column < column_guard);
        assert!(column_guard < first_b_read);
        assert_eq!(
            body.matches("iftile_column<tiles_per_row{}else{fe2o3_device::trap();}")
                .count(),
            1
        );
        assert_eq!(
            body.matches("iftile_column<9_496{}else{fe2o3_device::trap();}")
                .count(),
            1
        );
        assert_eq!(
            body.matches("iftile_row<128{}else{fe2o3_device::trap();}")
                .count(),
            1
        );
        assert_eq!(
            body.matches("ifrow_base<2_045{}else{fe2o3_device::trap();}")
                .count(),
            1
        );
        assert_eq!(
            body.matches("ifcolumn<n{}else{fe2o3_device::trap();}")
                .count(),
            1
        );
    }
}

fn checked_tile_column(n: usize, tile_column: usize, lane: usize) -> Option<usize> {
    let tiles_per_row = n.checked_add(15)? / 16;
    if tile_column >= tiles_per_row || tile_column >= 9_496 {
        return None;
    }
    let column = tile_column.checked_mul(16)?.checked_add(lane % 16)?;
    (column < n).then_some(column)
}

#[test]
fn admitted_tile_column_endpoints_are_in_bounds_and_hostile_columns_fail_closed() {
    for n in [1_024_usize, 2_048, 3_072, 4_096, 12_288, 151_936] {
        let tiles_per_row = n.div_ceil(16);
        assert!(tiles_per_row <= 9_496);
        for tile_column in 0..tiles_per_row {
            for lane in 0..64 {
                let column = checked_tile_column(n, tile_column, lane)
                    .expect("every admitted tile lane maps to an in-bounds column");
                assert_eq!(column, tile_column * 16 + lane % 16);
                assert!(column < n);
            }
        }
        assert_eq!(checked_tile_column(n, tiles_per_row, 0), None);
        assert_eq!(checked_tile_column(n, usize::MAX, 63), None);
    }
}

#[test]
fn admitted_tile_rows_and_component_offsets_fit_checked_arithmetic() {
    for m in [
        1_usize, 4, 5, 8, 9, 16, 17, 32, 40, 128, 512, 1_024, 2_048,
    ] {
        let tile_rows = m.div_ceil(16);
        assert!(tile_rows <= 128);
        for tile_row in 0..tile_rows {
            for lane in 0_usize..64 {
                let row_base = tile_row
                    .checked_mul(16)
                    .and_then(|value| {
                        (lane / 16)
                            .checked_mul(4)
                            .and_then(|part| value.checked_add(part))
                    })
                    .expect("every admitted tile row base must fit usize");
                assert!(row_base < 2_045);
                assert_eq!(row_base, tile_row * 16 + (lane / 16) * 4);
                for component in 0..4 {
                    let row = row_base
                        .checked_add(component)
                        .expect("every admitted component row must fit usize");
                    assert_eq!(row, row_base + component);
                }
            }
        }
    }
}

#[test]
fn admitted_a4_reduction_backedges_fit_checked_arithmetic() {
    assert_eq!(
        SOURCE
            .matches("let reduction_bound = k as u64;")
            .count(),
        1
    );
    assert_eq!(
        SOURCE
            .matches("while reduction_wide < reduction_bound {\n        let reduction = reduction_wide as usize;")
            .count(),
        1
    );
    for k in [1_024_usize, 2_048, 3_072, 4_096, 12_288] {
        assert_ne!(k, 0);
        assert_eq!(k % 4, 0);
        assert!(k < 12_289);
        let reduction_bound = (k as u32) as u64;
        let mut reduction_wide = 0_u64;
        let mut observed = Vec::new();
        while reduction_wide < reduction_bound {
            let reduction = reduction_wide as usize;
            assert_eq!(reduction as u64, reduction_wide);
            let final_component = reduction
                .checked_add(3)
                .expect("admitted A4 final component must fit usize");
            assert!(final_component < k);
            observed.push(reduction);
            reduction_wide = reduction_wide
                .checked_add(4)
                .expect("admitted A4 backedge must fit u64");
        }
        assert_eq!(reduction_wide, reduction_bound);
        assert_eq!(observed, (0..k).step_by(4).collect::<Vec<_>>());
        assert_eq!(observed.last(), Some(&(k - 4)));
        assert_eq!(
            (observed.last().copied().unwrap() as u64).checked_add(4),
            Some(reduction_bound)
        );
    }
}

#[test]
fn typed_tile_arithmetic_matches_the_witness_component_mapping() {
    assert!(SOURCE.contains("let row_base = tile_row * 16 + (lane / 16) * 4;"));
    for tiles_per_row in [1_usize, 2, 9_496] {
        for tile_index in 0..(tiles_per_row * 3) {
            let tile_row = tile_index / tiles_per_row;
            let tile_column = tile_index % tiles_per_row;
            let mut owned = std::collections::BTreeSet::new();
            for lane in 0..64 {
                for component in 0..4 {
                    let row = tile_row * 16 + (lane / 16) * 4 + component;
                    let column = tile_column * 16 + lane % 16;
                    assert!(owned.insert((row, column)));
                }
            }
            assert_eq!(owned.len(), 16 * 16);
            assert_eq!(owned.first(), Some(&(tile_row * 16, tile_column * 16)));
            assert_eq!(
                owned.last(),
                Some(&(tile_row * 16 + 15, tile_column * 16 + 15))
            );
        }
    }
    assert!(!SOURCE.contains("row_base + 4"));
    assert!(!SOURCE.contains("row_base + 8"));
    assert!(!SOURCE.contains("row_base + 12"));
}

#[test]
fn source_requires_a_flat_one_dimensional_grid() {
    let compact: String = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    assert_eq!(compact.matches("max_grid=[1215488,1,1]").count(), 2);
    assert_eq!(compact.matches("max_grid=[131072,1,1]").count(), 1);
    assert!(SOURCE.contains("let tile_index = raw / 64;"));
    assert_eq!(SOURCE.matches("thread::grid_dim_x() as usize").count(), 3);
    assert_eq!(SOURCE.matches("thread::block_dim_x() as usize").count(), 3);
    assert_eq!(
        SOURCE
            .matches("if launch_extent != expected_extent")
            .count(),
        2
    );
    assert!(SOURCE.contains("if launch_extent != output.len()"));
}
