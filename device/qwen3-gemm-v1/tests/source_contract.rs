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
    assert!(vectorized.contains("reduction += 4"));
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
fn reduction_ast_pins_each_load_pair_and_ascending_accumulation() {
    let kernels = kernels();
    let reference = compact_tokens(&kernels[0].block);
    let vector = compact_tokens(&kernels[1].block);

    fn reduction_loop<'a>(body: &'a str, step: &str) -> &'a str {
        let start = body
            .find("whilereduction<k{")
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
    for component in 0..4 {
        let branch = active_branch(reference_loop, component);
        let load = format!("volatile_load(a,row_{component}*k+reduction)");
        let accumulation = format!("accumulator_{component}=accumulator_{component}+left*right;");
        assert_eq!(branch.matches(&load).count(), 1);
        assert_eq!(branch.matches(&accumulation).count(), 1);
        assert!(branch.find(&load).unwrap() < branch.find(&accumulation).unwrap());
    }

    let vector_loop = reduction_loop(&vector, "reduction+=4;");
    let mut previous_right = 0;
    for offset in 0..4 {
        let index = if offset == 0 {
            "reduction".to_owned()
        } else {
            format!("(reduction+{offset})")
        };
        let load = format!("volatile_load(b,{index}*n+column)");
        let position = vector_loop
            .find(&load)
            .unwrap_or_else(|| panic!("missing ordered B load {load}"));
        assert!(offset == 0 || previous_right < position);
        previous_right = position;
    }
    for component in 0..4 {
        let branch = active_branch(vector_loop, component);
        let mut previous_accumulation = 0;
        for offset in 0..4 {
            let reduction = if offset == 0 {
                "reduction".to_owned()
            } else {
                format!("reduction+{offset}")
            };
            let load = format!("volatile_load(a,row_{component}*k+{reduction})");
            let accumulation = format!(
                "accumulator_{component}=accumulator_{component}+left_{offset}*right_{offset};"
            );
            assert_eq!(branch.matches(&load).count(), 1);
            assert_eq!(branch.matches(&accumulation).count(), 1);
            let load_position = branch.find(&load).unwrap();
            let accumulation_position = branch.find(&accumulation).unwrap();
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
            .find("while reduction < k")
            .expect("matrix has the ascending reduction loop");
        let loop_end = body[loop_start..]
            .find(if loads_per_active_row == 1 {
                "reduction += 1"
            } else {
                "reduction += 4"
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
        let column = body
            .find("letcolumn=tile_column*16+lane%16;")
            .expect("matrix computes its lane column");
        let column_guard = body
            .find("ifcolumn<n{}else{fe2o3_device::trap();}")
            .expect("matrix authenticates the exact B column");
        let first_b_read = body
            .find("volatile_load(b,")
            .expect("matrix reads its B input");

        assert!(tile_guard < column);
        assert!(column < column_guard);
        assert!(column_guard < first_b_read);
        assert_eq!(
            body.matches("iftile_column<tiles_per_row{}else{fe2o3_device::trap();}")
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
    if tile_column >= tiles_per_row {
        return None;
    }
    let column = tile_column.checked_mul(16)?.checked_add(lane % 16)?;
    (column < n).then_some(column)
}

#[test]
fn admitted_tile_column_endpoints_are_in_bounds_and_hostile_columns_fail_closed() {
    for n in [1_024_usize, 2_048, 3_072, 4_096, 12_288, 151_936] {
        let tiles_per_row = n.div_ceil(16);
        for tile_column in [0, tiles_per_row - 1] {
            for lane in [0, 63] {
                let column = checked_tile_column(n, tile_column, lane)
                    .expect("admitted endpoint maps to an in-bounds column");
                assert_eq!(column, tile_column * 16 + lane % 16);
                assert!(column < n);
            }
        }
        assert_eq!(checked_tile_column(n, tiles_per_row, 0), None);
        assert_eq!(checked_tile_column(n, usize::MAX, 63), None);
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
