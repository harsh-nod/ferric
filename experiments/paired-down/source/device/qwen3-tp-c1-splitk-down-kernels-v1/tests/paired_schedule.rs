use quote::ToTokens;
use syn::{Expr, ExprWhile, Item, Pat, Stmt, visit::Visit};

const SOURCE: &str = include_str!("../src/projection.rs");
const BEFORE: &str = include_str!("fixtures/projection_unpaired.rs.txt");
const EXISTING_K2: &str = include_str!("fixtures/projection_k2.rs.txt");

#[derive(Default)]
struct Loops(Vec<ExprWhile>);

impl<'ast> Visit<'ast> for Loops {
    fn visit_expr_while(&mut self, expression: &'ast ExprWhile) {
        self.0.push(expression.clone());
        syn::visit::visit_expr_while(self, expression);
    }
}

fn existing_k1536_loop() -> ExprWhile {
    let parsed = syn::parse_file(EXISTING_K2).unwrap();
    let function = parsed
        .items
        .iter()
        .find_map(|item| match item {
            Item::Fn(function)
                if function.sig.ident == "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5" =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let mut loops = Loops::default();
    loops.visit_block(&function.block);
    let mut matched = loops
        .0
        .into_iter()
        .filter(|repeated| repeated.cond.to_token_stream().to_string() == "pair < 48");
    let result = matched.next().unwrap();
    assert!(matched.next().is_none());
    result
}

fn adapted_existing_loop() -> ExprWhile {
    let mut repeated = existing_k1536_loop();
    let Stmt::Local(base) = &mut repeated.body.stmts[0] else {
        panic!("expected reduction base declaration")
    };
    let Pat::Ident(name) = &base.pat else {
        panic!("expected named reduction base")
    };
    assert_eq!(name.ident, "reduction_base");
    let value = base.init.as_mut().unwrap();
    assert_eq!(value.expr.to_token_stream().to_string(), "pair * 32");
    *value.expr = syn::parse_quote!(partition * 1536 + pair * 32);
    for index in [2, 4] {
        let Stmt::Local(fragment) = &mut repeated.body.stmts[index] else {
            panic!("expected A fragment declaration")
        };
        let Expr::MethodCall(load) = fragment.init.as_mut().unwrap().expr.as_mut() else {
            panic!("expected checked fragment load")
        };
        assert_eq!(load.method, "load_m16k16");
        assert_eq!(load.args[1].to_token_stream().to_string(), "tile_row * 16");
        load.args[1] = syn::parse_quote!(0);
    }
    repeated
}

fn expected_source() -> syn::File {
    let mut expected = syn::parse_file(BEFORE).unwrap();
    let function = expected
        .items
        .iter_mut()
        .find_map(|item| match item {
            Item::Fn(function)
                if function.sig.ident == "ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1" =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let kernel = function
        .attrs
        .iter_mut()
        .find(|attr| attr.path().is_ident("kernel"))
        .unwrap();
    assert!(
        kernel
            .to_token_stream()
            .to_string()
            .contains("loop_bounds (96)")
    );
    *kernel = syn::parse_quote! {
        #[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2048, 1, 1]), control_flow(loop_bounds(48)))]
    };
    let loop_index = function
        .block
        .stmts
        .iter()
        .position(|statement| matches!(statement, Stmt::Expr(Expr::While(_), _)))
        .unwrap();
    let Stmt::Expr(Expr::While(original), _) = &function.block.stmts[loop_index] else {
        unreachable!()
    };
    assert_eq!(original.cond.to_token_stream().to_string(), "step < 96");
    assert_eq!(
        function.block.stmts[loop_index - 1]
            .to_token_stream()
            .to_string(),
        "let mut step = 0_usize ;"
    );
    function.block.stmts[loop_index - 1] = syn::parse_quote!(let mut pair = 0_usize;);
    function.block.stmts[loop_index] = Stmt::Expr(Expr::While(adapted_existing_loop()), None);
    expected
}

fn exact_declared_change(source: &str) -> bool {
    syn::parse_file(source).is_ok_and(|parsed| {
        parsed.to_token_stream().to_string() == expected_source().to_token_stream().to_string()
    })
}

#[test]
fn whole_source_changes_only_existing_paired_schedule_and_trip_bound() {
    assert!(exact_declared_change(SOURCE));
}

#[test]
fn original_k1536_pattern_needs_only_partition_offset_and_row_zero() {
    let original = existing_k1536_loop().to_token_stream().to_string();
    let adapted = adapted_existing_loop().to_token_stream().to_string();
    assert_eq!(original.matches("tile_row * 16").count(), 2);
    assert_eq!(
        adapted,
        original
            .replacen("pair * 32", "partition * 1536 + pair * 32", 1)
            .replace("tile_row * 16", "0")
    );
    assert_eq!(adapted.matches("multiply_accumulate (").count(), 2);
    assert!(!adapted.contains("WorkgroupPipeline"));
}

#[test]
fn source_equivalence_rejects_math_geometry_guards_and_order_mutations() {
    for (before, after) in [
        ("pair < 48", "pair < 47"),
        ("loop_bounds(48)", "loop_bounds(49)"),
        (
            "partition * 1536 + pair * 32",
            "partition * 1536 + pair * 16",
        ),
        ("reduction_base + 16", "reduction_base + 32"),
        (
            "left.load_m16k16(&lane, 0, next_reduction_base)",
            "left.load_m16k16(&lane, 1, next_reduction_base)",
        ),
        (
            "multiply_accumulate(a_fragment, b_fragment, accumulator)",
            "multiply_accumulate(next_a_fragment, next_b_fragment, accumulator)",
        ),
        (
            "multiply_accumulate(next_a_fragment, next_b_fragment, accumulator)",
            "multiply_accumulate(a_fragment, b_fragment, accumulator)",
        ),
        ("pair += 1", "pair += 2"),
        ("raw % 64 < 16", "raw % 64 <= 16"),
        ("!value_0.is_finite()", "false"),
        ("2048, 16, 16, value_0", "2048, 16, 17, value_0"),
        ("world_size != 1", "world_size != 2"),
        ("partials.len() != 8 * 4096", "partials.len() < 8 * 4096"),
        ("sum += value", "sum = value + sum"),
    ] {
        assert!(
            SOURCE.contains(before),
            "missing mutation preimage: {before}"
        );
        assert!(
            !exact_declared_change(&SOURCE.replacen(before, after, 1)),
            "accepted {after}"
        );
    }
}

#[test]
fn paired_fragment_coordinates_cover_exact_original_partitions() {
    for partition in 0_usize..8 {
        let paired: Vec<_> = (0..48)
            .flat_map(|pair| {
                [
                    partition * 1536 + pair * 32,
                    partition * 1536 + pair * 32 + 16,
                ]
            })
            .collect();
        let original: Vec<_> = (0..96).map(|step| partition * 1536 + step * 16).collect();
        assert_eq!(paired, original);
        assert_eq!(paired.len(), 96);
        assert_eq!(paired[95] + 15, (partition + 1) * 1536 - 1);
        for base in paired {
            for lane in 0..64 {
                for component in 0..4 {
                    let reduction = base + (lane / 16) * 4 + component;
                    assert!((partition * 1536..(partition + 1) * 1536).contains(&reduction));
                    assert!(reduction < 12288);
                    let a_row = lane & 15;
                    assert_eq!(a_row < 1, lane.is_multiple_of(16));
                    for tile_column in [0, 255] {
                        let column = tile_column * 16 + (lane & 15);
                        assert!(column < 4096);
                        assert!(reduction * 4096 + column < 12288 * 4096);
                    }
                }
            }
        }
    }
}

fn ordered_additions(values: &[f32; 96], paired: bool) -> f32 {
    let mut accumulator = 0.0_f32;
    if paired {
        for pair in 0..48 {
            let first = values[pair * 2];
            let second = values[pair * 2 + 1];
            accumulator += first;
            accumulator += second;
        }
    } else {
        for value in values {
            accumulator += value;
        }
    }
    accumulator
}

#[test]
fn ordered_host_model_preserves_sensitive_addition_sequence() {
    // This screens update order; it does not emulate an MFMA instruction.
    let mut values = [0.0_f32; 96];
    for (index, value) in values.iter_mut().enumerate() {
        *value = [33_554_432.0_f32, -33_554_432.0, 1.0][index % 3];
    }
    assert_eq!(
        ordered_additions(&values, false).to_bits(),
        ordered_additions(&values, true).to_bits()
    );
    assert_ne!(
        (values[0] + values[1]) + values[2],
        values[0] + (values[1] + values[2])
    );
}

#[test]
fn ordered_host_model_keeps_exceptional_classification_and_signed_zero() {
    for value in [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        -0.0,
        f32::from_bits(1),
    ] {
        for index in [0, 1, 47, 48, 94, 95] {
            let mut values = [0.0_f32; 96];
            values[index] = value;
            let original = ordered_additions(&values, false);
            let paired = ordered_additions(&values, true);
            assert_eq!(original.is_finite(), paired.is_finite());
            if original.is_nan() {
                assert!(paired.is_nan());
            } else {
                assert_eq!(original.to_bits(), paired.to_bits());
            }
        }
    }
}
