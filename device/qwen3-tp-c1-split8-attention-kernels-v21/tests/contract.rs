use fe2o3_device::{Index1D, RowStriped2D, WriteOnlyDisjointSlice};
use ferric_qwen3_tp_c1_split8_attention_kernels_device_v21::{
    MERGE_IMPLICIT_OFFSET_V21, MERGE_KERNARG_BYTES_V21, NUMERATOR_ELEMENTS_V21,
    PARTIAL_IMPLICIT_OFFSET_V21, PARTIAL_KERNARG_BYTES_V21, ROOTS_V21, SCRATCH_BYTES_V21,
    STATS_ELEMENTS_V21, compiler_expectation_roster_v21,
};
use quote::ToTokens;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, ExprIf, ExprWhile, Item, ItemFn, Stmt};

#[path = "../build/target_contract.rs"]
mod target_contract;

const SOURCE: &str = include_str!("../src/attention.rs");

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn roots() -> Vec<ItemFn> {
    syn::parse_file(SOURCE)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| {
            if let Item::Fn(function) = item {
                Some(function)
            } else {
                None
            }
        })
        .collect()
}

struct CallPunctuation;

impl VisitMut for CallPunctuation {
    fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
        visit_mut::visit_expr_call_mut(self, call);
        if call.args.trailing_punct() {
            call.args = call.args.iter().cloned().collect();
        }
    }

    fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
        visit_mut::visit_expr_method_call_mut(self, call);
        if call.args.trailing_punct() {
            call.args = call.args.iter().cloned().collect();
        }
    }
}

fn statement_tokens(statement: &Stmt) -> String {
    let mut statement = statement.clone();
    // rustfmt can add only trailing argument commas to multiline calls.
    CallPunctuation.visit_stmt_mut(&mut statement);
    tokens(&statement)
}

fn expression_tokens(expression: &Expr) -> String {
    let mut expression = expression.clone();
    CallPunctuation.visit_expr_mut(&mut expression);
    tokens(&expression)
}

fn contains_statements(block: &syn::Block, expected: &str) -> bool {
    let expected: syn::Block = syn::parse_str(&format!("{{ {expected} }}")).unwrap();
    block.stmts.windows(expected.stmts.len()).any(|window| {
        window
            .iter()
            .zip(&expected.stmts)
            .all(|(actual, expected)| statement_tokens(actual) == statement_tokens(expected))
    })
}

fn branch<'a>(block: &'a syn::Block, condition: &str) -> &'a ExprIf {
    let condition: Expr = syn::parse_str(condition).unwrap();
    let matches: Vec<_> = block
        .stmts
        .iter()
        .filter_map(|statement| {
            if let Stmt::Expr(Expr::If(value), _) = statement
                && expression_tokens(&value.cond) == expression_tokens(&condition)
            {
                Some(value)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}

fn loop_body<'a>(function: &'a ItemFn, condition: &str) -> &'a ExprWhile {
    let condition: Expr = syn::parse_str(condition).unwrap();
    let loops: Vec<_> = function
        .block
        .stmts
        .iter()
        .filter_map(|statement| {
            if let Stmt::Expr(Expr::While(value), _) = statement {
                Some(value)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(loops.len(), 1);
    assert_eq!(
        expression_tokens(&loops[0].cond),
        expression_tokens(&condition)
    );
    loops[0]
}

#[test]
fn independent_typed_roots_have_the_expected_host_layout_and_bounded_launches() {
    let roots = roots();
    assert_eq!(roots.len(), 2);
    let signatures = [
        "fn ferric_qwen3_tp_c1_split8_attention_partial_f32_v21(
            query: &[u16], key_cache: &[u16], value_cache: &[u16], positions: &[u32], page_table: &[u32],
            mut stats: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
            mut numerators: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 2>>,
            rows: u32, world_size: u32, max_pages_per_sequence: u32, physical_pages: u32,
            max_context_tokens: u32) {}",
        "fn ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21(
            stats: &[f32], numerators: &[f32],
            mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>) {}",
    ];
    for (index, root) in roots.iter().enumerate() {
        assert_eq!(root.sig.ident, ROOTS_V21[index]);
        let expected: ItemFn = syn::parse_str(signatures[index]).unwrap();
        assert_eq!(root.sig.inputs.len(), expected.sig.inputs.len());
        for (actual, expected) in root.sig.inputs.iter().zip(&expected.sig.inputs) {
            assert_eq!(tokens(actual), tokens(expected));
        }
        assert_eq!(tokens(&root.sig.output), tokens(&expected.sig.output));
        let grid = if index == 0 { 256 } else { 32 };
        let bound = if index == 0 { 32 } else { 8 };
        let expected: ItemFn = syn::parse_str(&format!(
            "#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [{grid}, 1, 1]), control_flow(loop_bounds({bound})))] fn expected() {{}}"
        )).unwrap();
        let attribute = root
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("kernel"))
            .unwrap();
        assert_eq!(tokens(attribute), tokens(&expected.attrs[0]));
    }
    let roster = compiler_expectation_roster_v21();
    assert_eq!(roster.len(), 2);
    for (entry, name) in roster.iter().zip(ROOTS_V21) {
        assert_eq!(entry.export_name(), name);
        assert!(!name.ends_with("_v14") && !name.ends_with("_v17"));
    }
    assert_ne!(roster[0].kernel_binding_id(), roster[1].kernel_binding_id());
    assert_eq!(core::mem::size_of::<&[u16]>(), 16);
    assert_eq!(core::mem::size_of::<&[u32]>(), 16);
    assert_eq!(core::mem::size_of::<&[f32]>(), 16);
    assert_eq!(
        WriteOnlyDisjointSlice::<f32, RowStriped2D<Index1D, 64, 1>>::__fe2o3_rust_layout_v1(),
        (16, 8, 0, 8)
    );
    assert_eq!(
        WriteOnlyDisjointSlice::<f32, RowStriped2D<Index1D, 64, 2>>::__fe2o3_rust_layout_v1(),
        (16, 8, 0, 8)
    );
    assert_eq!(
        WriteOnlyDisjointSlice::<u16, RowStriped2D<Index1D, 64, 2>>::__fe2o3_rust_layout_v1(),
        (16, 8, 0, 8)
    );
    assert_eq!(
        PARTIAL_IMPLICIT_OFFSET_V21,
        (7_usize * 16 + 5 * 4).next_multiple_of(8)
    );
    assert_eq!(PARTIAL_KERNARG_BYTES_V21, 392);
    assert_eq!(MERGE_IMPLICIT_OFFSET_V21, 3 * 16);
    assert_eq!(MERGE_KERNARG_BYTES_V21, 304);
    assert_eq!(
        (
            STATS_ELEMENTS_V21,
            NUMERATOR_ELEMENTS_V21,
            SCRATCH_BYTES_V21
        ),
        (512, 32768, 133120)
    );
}

#[test]
fn partial_bounds_preserve_capacity_views_and_mask_every_inactive_token_load() {
    let roots = roots();
    let root = &roots[0];
    for condition in [
        "rows != 1 || world_size != 1 || max_pages_per_sequence == 0 || max_pages_per_sequence > 512 || physical_pages == 0 || physical_pages > 512 || max_context_tokens < 128 || max_context_tokens > 256",
        "query.len() < 4096 || query.len() > 32 * 4096 || key_cache.len() != physical_pages * 16 * 1024 || value_cache.len() != physical_pages * 16 * 1024 || positions.len() < 1 || positions.len() > 32 || page_table.len() < max_pages_per_sequence || page_table.len() > 32 * max_pages_per_sequence || max_context_tokens > max_pages_per_sequence * 16 || stats.len() != 512 || numerators.len() != 32768 || thread::launch_extent_1d() != 256 * 64",
    ] {
        assert!(contains_statements(
            &branch(&root.block, condition).then_branch,
            "fe2o3_device::trap();"
        ));
    }
    for condition in [
        "max_pages_per_sequence < 513 && physical_pages < 513 && max_context_tokens < 257",
        "partition_row < 256",
        "query_head < 32 && kv_head < 8",
        "position < 256 && position + 1 == max_context_tokens",
        "span > 0 && span < 33",
        "begin < max_context_tokens",
    ] {
        let guard = branch(&root.block, condition);
        assert!(guard.then_branch.stmts.is_empty());
        let Some((_, alternative)) = &guard.else_branch else {
            panic!("missing rejection");
        };
        let Expr::Block(block) = alternative.as_ref() else {
            panic!("missing rejection block");
        };
        assert!(contains_statements(&block.block, "fe2o3_device::trap();"));
    }
    assert!(contains_statements(
        &root.block,
        "let query_head = partition_row / 8; let partition = partition_row % 8; let kv_head = query_head / 4;"
    ));
    assert!(contains_statements(
        &root.block,
        "let span = (max_context_tokens + 7) / 8;"
    ));
    assert!(contains_statements(
        &root.block,
        "let begin = partition * span;"
    ));
    let body = loop_body(root, "offset < span");
    assert_eq!(body.body.stmts.len(), 3);
    assert!(contains_statements(
        &body.body,
        "let token = begin + offset;"
    ));
    assert!(contains_statements(&body.body, "offset += 1;"));
    let active = &branch(&body.body, "token < max_context_tokens").then_branch;
    assert!(contains_statements(
        active,
        "let physical_page = subgroup.broadcast_f32::<64>(table_view.load_or(0, token / 16, u32::MAX) as f32, 0) as usize;"
    ));
    assert!(contains_statements(
        active,
        "let cache_row = physical_page * 16 + token % 16; let cache_column = kv_head * 128 + lane;"
    ));
    assert!(contains_statements(
        active,
        "let key_0 = Bf16::from_bits(key_view.load_or(cache_row, cache_column, 0)).to_f32(); let key_1 = Bf16::from_bits(key_view.load_or(cache_row, cache_column + 64, 0)).to_f32();"
    ));
    assert!(contains_statements(
        active,
        "let value_0 = Bf16::from_bits(value_view.load_or(cache_row, cache_column, 0)).to_f32(); let value_1 = Bf16::from_bits(value_view.load_or(cache_row, cache_column + 64, 0)).to_f32();"
    ));
    assert_eq!(SOURCE.matches("key_view.load_or(").count(), 2);
    assert_eq!(SOURCE.matches("value_view.load_or(").count(), 2);
    assert_eq!(SOURCE.matches("table_view.load_or(").count(), 1);
}

#[test]
fn score_order_and_online_state_recurrences_are_explicit_and_narrow_only_after_merge() {
    let plain: Stmt = syn::parse_quote! { let value = view.load_or(row, column, 0); };
    let formatted: Stmt = syn::parse_quote! { let value = view.load_or(row, column, 0,); };
    assert_eq!(statement_tokens(&plain), statement_tokens(&formatted));
    for mutation in [
        "let value = view.load_or(column, row, 0,);",
        "let value = view.load_or(row, column, 1,);",
        "let value = view.load_or(row, column, 0, extra,);",
    ] {
        let mutation: Stmt = syn::parse_str(mutation).unwrap();
        assert_ne!(statement_tokens(&plain), statement_tokens(&mutation));
    }
    let nested_plain: Stmt = syn::parse_quote! {
        let value = Bf16::from_bits(view.load_or(row, column, 0)).to_f32();
    };
    let nested_formatted: Stmt = syn::parse_quote! {
        let value = Bf16::from_bits(view.load_or(row, column, 0,),).to_f32();
    };
    assert_eq!(
        statement_tokens(&nested_plain),
        statement_tokens(&nested_formatted)
    );
    let broadcast_plain: Stmt = syn::parse_quote! {
        let page = subgroup.broadcast_f32::<64>(view.load_or(0, token / 16, u32::MAX) as f32, 0) as usize;
    };
    let broadcast_formatted: Stmt = syn::parse_quote! {
        let page = subgroup.broadcast_f32::<64>(view.load_or(0, token / 16, u32::MAX,) as f32, 0,) as usize;
    };
    assert_eq!(
        statement_tokens(&broadcast_plain),
        statement_tokens(&broadcast_formatted)
    );
    let guarded_calls: syn::Block = syn::parse_quote! { {
        if token < active && ready(view.load_or(row, column, 0,),) { checked(); }
    } };
    assert!(contains_statements(
        &branch(
            &guarded_calls,
            "token < active && ready(view.load_or(row, column, 0))"
        )
        .then_branch,
        "checked();"
    ));
    let guarded_update = "if token < active { let product = left * right; sum += product; finite &= product.is_finite() & sum.is_finite(); }";
    for mutation in [
        "if token <= active { let product = left * right; sum += product; finite &= product.is_finite() & sum.is_finite(); }",
        "if token < active { let product = left + right; sum += product; finite &= product.is_finite() & sum.is_finite(); }",
        "if token < active { let product = left * right; finite &= product.is_finite() & sum.is_finite(); sum += product; }",
        "if token < active { let product = left * right; sum += product; finite &= product.is_finite(); }",
    ] {
        let changed: syn::Block = syn::parse_str(&format!("{{ {mutation} }}")).unwrap();
        assert!(!contains_statements(&changed, guarded_update));
    }
    let roots = roots();
    let partial = &branch(
        &loop_body(&roots[0], "offset < span").body,
        "token < max_context_tokens",
    )
    .then_branch;
    assert!(contains_statements(partial,
        "let product_0 = query_0 * key_0; let product_1 = query_1 * key_1; let partial = product_0 + product_1;
        finite &= product_0.is_finite() & product_1.is_finite() & partial.is_finite();
        let dot = subgroup.reduce_sum_f32::<64>(partial);"));
    assert!(contains_statements(
        partial,
        "let score = dot * ATTENTION_SCALE;"
    ));
    let initialization = branch(partial, "offset == 0");
    assert!(contains_statements(
        &initialization.then_branch,
        "maximum = score; denominator = 1.0; numerator_0 = value_0; numerator_1 = value_1;"
    ));
    let Expr::Block(update) = initialization.else_branch.as_ref().unwrap().1.as_ref() else {
        panic!("missing partial update");
    };
    assert!(contains_statements(
        &update.block,
        "let next_maximum = if score > maximum { score } else { maximum };
        let previous_weight = math.exp_f32(maximum - next_maximum);
        let current_weight = math.exp_f32(score - next_maximum);
        denominator = denominator * previous_weight + current_weight;
        numerator_0 = numerator_0 * previous_weight + value_0 * current_weight;
        numerator_1 = numerator_1 * previous_weight + value_1 * current_weight;"
    ));
    assert!(contains_statements(&update.block,
        "finite &= previous_weight.is_finite() & (previous_weight >= 0.0) & current_weight.is_finite()
        & (current_weight >= 0.0) & denominator.is_finite() & (denominator > 0.0)
        & numerator_0.is_finite() & numerator_1.is_finite(); maximum = next_maximum;"));
    let merge = loop_body(&roots[1], "partition < 8");
    assert!(contains_statements(
        &merge.body,
        "let partition_row = query_head * 8 + partition;
        let part_maximum = stats_view.load_or(partition_row, 0, f32::INFINITY);
        let part_denominator = stats_view.load_or(partition_row, 1, f32::INFINITY);
        let part_numerator_0 = numerator_view.load_or(partition_row, lane, f32::INFINITY);
        let part_numerator_1 = numerator_view.load_or(partition_row, lane + 64, f32::INFINITY);"
    ));
    assert!(contains_statements(&merge.body,
        "finite &= part_maximum.is_finite() & part_denominator.is_finite() & (part_denominator > 0.0)
        & part_numerator_0.is_finite() & part_numerator_1.is_finite();"));
    let initialization = branch(&merge.body, "partition == 0");
    assert!(contains_statements(
        &initialization.then_branch,
        "maximum = part_maximum; denominator = part_denominator; numerator_0 = part_numerator_0; numerator_1 = part_numerator_1;"
    ));
    let Expr::Block(update) = initialization.else_branch.as_ref().unwrap().1.as_ref() else {
        panic!("missing merge update");
    };
    assert!(contains_statements(
        &update.block,
        "let next_maximum = if part_maximum > maximum { part_maximum } else { maximum };
        let previous_weight = math.exp_f32(maximum - next_maximum);
        let current_weight = math.exp_f32(part_maximum - next_maximum);
        denominator = denominator * previous_weight + part_denominator * current_weight;
        numerator_0 = numerator_0 * previous_weight + part_numerator_0 * current_weight;
        numerator_1 = numerator_1 * previous_weight + part_numerator_1 * current_weight;"
    ));
    assert!(contains_statements(
        &roots[1].block,
        "let result_0 = numerator_0 / denominator; let result_1 = numerator_1 / denominator;
        let narrowed_0 = Bf16::from_f32(result_0); let narrowed_1 = Bf16::from_f32(result_1);"
    ));
    assert!(!tokens(&roots[0]).contains("Bf16 :: from_f32"));
    assert_eq!(SOURCE.matches("Bf16::from_f32(").count(), 2);
    assert!(SOURCE.contains("f32::from_bits(0x3db5_04f3)"));
    let deliberately_changed: syn::Block = syn::parse_quote! { {
        denominator = denominator * current_weight + part_denominator * previous_weight;
    } };
    assert!(!contains_statements(
        &deliberately_changed,
        "denominator = denominator * previous_weight + part_denominator * current_weight;"
    ));
}

#[test]
fn scratch_store_mapping_and_merge_output_extent_remain_separate_and_checked() {
    let roots = roots();
    assert!(contains_statements(
        &roots[0].block,
        "if !numerators.write_row_striped_2d(&numerator_stripe, 0, 256, 128, 128, numerator_0)
            || !numerators.write_row_striped_2d(&numerator_stripe, 1, 256, 128, 128, numerator_1)
        { fe2o3_device::trap(); }"
    ));
    let scalar_store = &branch(&roots[0].block, "lane < 2").then_branch;
    assert!(contains_statements(
        scalar_store,
        "let value = if lane == 0 { maximum } else { denominator };"
    ));
    assert!(contains_statements(
        scalar_store,
        "if !stats.write_row_striped_2d(&stats_stripe, 0, 256, 2, 2, value) { fe2o3_device::trap(); }"
    ));
    assert!(contains_statements(&branch(&roots[1].block,
        "stats.len() != 512 || numerators.len() != 32768 || output.len() < 4096 || output.len() > 32 * 4096 || thread::launch_extent_1d() != 32 * 64").then_branch,
        "fe2o3_device::trap();"));
    assert!(contains_statements(
        &roots[1].block,
        "if !output.write_row_striped_2d(&stripe, 0, 32, 128, 128, narrowed_0.to_bits())
            || !output.write_row_striped_2d(&stripe, 1, 32, 128, 128, narrowed_1.to_bits())
        { fe2o3_device::trap(); }"
    ));
    assert_eq!(SOURCE.matches("write_row_striped_2d(").count(), 5);
    assert_eq!(
        SOURCE.matches("checked_row_striped_2d::<64, 2>()").count(),
        2
    );
    assert_eq!(
        SOURCE.matches("checked_row_striped_2d::<64, 1>()").count(),
        1
    );
}

#[test]
fn producer_and_target_are_closed_without_unsafe_or_alternate_math_paths() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(
        manifest
            .matches("rev = \"c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b\"")
            .count(),
        2
    );
    assert!(manifest.contains("default = [\"gfx950\"]"));
    for forbidden in [
        "unsafe",
        "mul_add",
        "Mfma",
        "memory::",
        "GridExclusive",
        "atomic::",
        "lds::",
    ] {
        assert!(!SOURCE.contains(forbidden));
    }
    let cpu = "gfx950";
    let features = "-wavefrontsize32,+wavefrontsize64,-xnack";
    let flags = format!("-Ctarget-cpu={cpu}\u{1f}-Ctarget-feature={features}");
    assert!(target_contract::validate_device_build("amdgpu", &flags, cpu, features).is_ok());
    for flags in [
        String::new(),
        flags.replace(cpu, "gfx942"),
        flags.replace("-xnack", "+xnack"),
        format!("{flags}\u{1f}-Ctarget-cpu=gfx950"),
    ] {
        assert!(target_contract::validate_device_build("amdgpu", &flags, cpu, features).is_err());
    }
}
