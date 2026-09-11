//! Compare parsed bodies, including helper macros, against explicit source deltas.
//! These checks preserve arithmetic; they are not a numerical qualification.
use syn::{Item, ItemFn, Stmt};

fn rename(source: &str, version: &str) -> String {
    source
        .replace("ferric_qwen3_tp_batch32_", "ferric_qwen3_draft_batch32_")
        .replace(version, "_v10")
}

fn replace(source: String, from: &str, to: &str) -> String {
    assert!(
        source.contains(from),
        "missing expected original source: {from}"
    );
    source.replace(from, to)
}

fn functions(source: &str) -> Vec<ItemFn> {
    syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| {
            let Item::Fn(mut f) = item else { return None };
            if !f.attrs.iter().any(|a| a.path().is_ident("kernel")) {
                return None;
            }
            f.attrs
                .retain(|a| !a.path().is_ident("doc") && !a.path().is_ident("cfg"));
            Some(f)
        })
        .collect()
}

fn equal_functions(expected: &str, actual: &str) {
    assert_eq!(functions(expected), functions(actual));
}

fn equal_macros(expected: &str, actual: &str) {
    let collect = |source: &str| {
        syn::parse_file(source)
            .unwrap()
            .items
            .into_iter()
            .filter_map(|item| match item {
                Item::Macro(m) => Some(m),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let originals = collect(expected);
    let copies = collect(actual);
    for copy in copies {
        let original = originals.iter().find(|m| m.ident == copy.ident).unwrap();
        assert_eq!(original, &copy);
    }
}

const WORLD: &str = "!(world_size == 1 || world_size == 2 || world_size == 8)";
const QUERY_HEADS: &str = "let query_heads = if world_size == 1 {\n        32_u32\n    } else if world_size == 2 {\n        16\n    } else {\n        4\n    };";
const KV_HEADS: &str = "let kv_heads = if world_size == 1 {\n        8_u32\n    } else if world_size == 2 {\n        4\n    } else {\n        1\n    };";

#[test]
fn embedding_and_residual_are_exact_width_only_copies() {
    for (old, new) in [
        (
            include_str!("../../qwen3-tp-batch32-kernels-v5/src/embedding.rs"),
            include_str!("../src/embedding.rs"),
        ),
        (
            include_str!("../../qwen3-tp-batch32-kernels-v5/src/collective.rs"),
            include_str!("../src/collective.rs"),
        ),
    ] {
        let expected = replace(
            replace(rename(old, "_v5"), "4096", "1024"),
            "max_grid = [2048,",
            "max_grid = [512,",
        );
        equal_functions(&expected, new);
        equal_macros(&expected, new);
    }
}

#[test]
fn activation_is_exact_width_and_tp1_guard_copy() {
    let old = include_str!("../../qwen3-tp-batch32-kernels-v5/src/activation.rs");
    let new = include_str!("../src/activation.rs");
    let expected = replace(rename(old, "_v5"), WORLD, "world_size != 1");
    let expected = replace(
        expected,
        "let columns = if world_size == 1 {\n        12288_u32\n    } else if world_size == 2 {\n        6144\n    } else {\n        1536\n    };",
        "let columns = 3072_u32;",
    );
    let expected = replace(
        replace(expected, "max_grid = [6144,", "max_grid = [1536,"),
        "columns < 12289",
        "columns < 3073",
    );
    equal_functions(&expected, new);
    equal_macros(&expected, new);
}

#[test]
fn rope_append_and_all_helpers_preserve_arithmetic_and_page_guards() {
    let old = include_str!("../../qwen3-tp-batch32-kernels-v5/src/rope_kv.rs");
    let new = include_str!("../src/rope_kv.rs");
    let mut expected = rename(old, "_v5");
    for (from, to) in [
        (WORLD, "world_size != 1"),
        (QUERY_HEADS, "let query_heads = 16_u32;"),
        (KV_HEADS, "let kv_heads = 8_u32;"),
        ("query_heads < 33", "query_heads < 17"),
        ("head < 32", "head < 16"),
        ("index < 131072", "index < 65536"),
        ("loop_bounds(32, 8)", "loop_bounds(16, 8)"),
    ] {
        expected = replace(expected, from, to);
    }
    equal_functions(&expected, new);
    equal_macros(&expected, new);
}

#[test]
fn attention_changes_only_head_geometry_and_tp1_admission() {
    let old = include_str!("../../qwen3-tp-batch32-kernels-v5/src/baseline_attention.rs");
    let new = include_str!("../src/attention.rs");
    let mut expected = rename(old, "_v5");
    for (from, to) in [
        (WORLD, "world_size != 1"),
        (QUERY_HEADS, "let query_heads = 16_u32;"),
        (KV_HEADS, "let kv_heads = 8_u32;"),
        ("query_heads < 33", "query_heads < 17"),
        ("max_grid = [1024,", "max_grid = [512,"),
        ("query_base < 131072", "query_base < 65536"),
        (
            "let row = if world_size == 1 {\n        head_row / 32\n    } else if world_size == 2 {\n        head_row / 16\n    } else {\n        head_row / 4\n    };",
            "let row = head_row / 16;",
        ),
        (
            "let query_head = if world_size == 1 {\n        head_row % 32\n    } else if world_size == 2 {\n        head_row % 16\n    } else {\n        head_row % 4\n    };",
            "let query_head = head_row % 16;",
        ),
        (
            "let kv_head = query_head / 4;",
            "let kv_head = query_head / 2;",
        ),
    ] {
        expected = replace(expected, from, to);
    }
    equal_functions(&expected, new);
    equal_macros(&expected, new);
}

#[test]
fn head_and_argmax_preserve_fp32_arithmetic_and_tie_rules() {
    let old = include_str!("../../qwen3-tp-fp32-head32-kernels-v8/src/projection.rs");
    let new = include_str!("../src/head.rs");
    let expected = replace(
        replace(
            replace(rename(old, "_v8"), "4096", "1024"),
            "step < 256",
            "step < 64",
        ),
        "loop_bounds(256)",
        "loop_bounds(64)",
    );
    equal_functions(&expected, new);
    equal_macros(&expected, new);
    let old = rename(
        include_str!("../../qwen3-tp-fp32-head32-kernels-v8/src/logits.rs"),
        "_v8",
    );
    let new = include_str!("../src/logits.rs");
    equal_functions(&old, new);
    equal_macros(&old, new);
}

#[test]
fn rmsnorm_has_only_stronger_entry_admission_and_symbol_grid_changes() {
    let old = include_str!("../../qwen3-all-kernels-v1/src/rmsnorm.rs")
        .replace(
            "qwen3_rmsnorm_v1(",
            "ferric_qwen3_draft_batch32_rmsnorm_v10(",
        )
        .replace("max_grid = [65536,", "max_grid = [512,");
    let mut expected = functions(&old).pop().unwrap();
    expected.block.stmts.insert(0,syn::parse_str::<Stmt>(
        "if behavior != 0 || !((width == 1024 && rows <= 32) || (width == 128 && rows <= 512)) { fe2o3_device::trap(); }").unwrap());
    let actual = functions(include_str!("../src/rmsnorm.rs")).pop().unwrap();
    assert_eq!(expected, actual);
    let source = include_str!("../src/rmsnorm.rs");
    assert!(source.contains("QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1: u32 = 512;"));
    assert!(source.contains("QWEN3_RMSNORM_EPSILON_V1: f32 = 1e-6_f32;"));
}

const COLUMN_SHAPE: &str = "k == 1024 && world_size == 1 && ((projection == 1 && n == 2048) || ((projection == 2 || projection == 3) && n == 1024) || ((projection == 4 || projection == 5) && n == 3072))";
const PARTIAL_SHAPE: &str = "n == 1024 && world_size == 1 && ((projection == 1 && k == 2048) || (projection == 2 && k == 3072))";
const TILES: &str = "let (tile_row, tile_column) = if n == 1024 { (tile_index / 64, tile_index % 64) } else if n == 2048 { (tile_index / 128, tile_index % 128) } else if n == 3072 { (tile_index / 192, tile_index % 192) } else { fe2o3_device::trap() };";

fn replace_tiles(function: &mut ItemFn) {
    let matches: Vec<_> = function
        .block
        .stmts
        .iter()
        .enumerate()
        .filter_map(|(i, stmt)| {
            let Stmt::Local(local) = stmt else {
                return None;
            };
            if matches!(local.pat, syn::Pat::Tuple(_)) {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    let index = matches[0];
    let Stmt::Local(local) = &function.block.stmts[index] else {
        panic!("tile")
    };
    let expected: syn::Pat = syn::parse_str::<Stmt>(TILES)
        .ok()
        .and_then(|s| match s {
            Stmt::Local(l) => Some(l.pat),
            _ => None,
        })
        .unwrap();
    assert_eq!(local.pat, expected);
    function.block.stmts[index] = syn::parse_str(TILES).unwrap();
}

#[test]
fn scalar_projection_bodies_change_only_exact_shapes_and_bounded_coordinates() {
    let mut old = rename(
        include_str!("../../qwen3-tp-batch32-kernels-v5/src/baseline_projection.rs"),
        "_v5",
    );
    for (from, to) in [
        ("loop_bounds(12288)", "loop_bounds(3072)"),
        ("max_grid = [18992,", "max_grid = [384,"),
        ("max_grid = [512,", "max_grid = [128,"),
        ("n < 151937", "n < 3073"),
        ("n < 4097", "n < 1025"),
        ("k < 12289", "k < 3073"),
        ("tile_column < 256", "tile_column < 64"),
    ] {
        old = replace(old, from, to);
    }
    let mut expected = functions(&old);
    for (function, shape) in expected.iter_mut().zip([COLUMN_SHAPE, PARTIAL_SHAPE]) {
        function.block.stmts[0] = syn::parse_str(&format!(
            "if rows == 0 || rows > 32 || !{{ {shape} }} {{ fe2o3_device::trap(); }}"
        ))
        .unwrap();
        replace_tiles(function);
    }
    let actual = include_str!("../src/projection.rs");
    assert_eq!(expected, functions(actual));
    equal_macros(&old, actual);
}

#[test]
fn mfma_projection_keeps_exact_fragments_and_masked_stores() {
    let original = rename(
        include_str!("../../qwen3-tp-batch32-kernels-v5/src/projection.rs"),
        "_v5",
    );
    let mut column = functions(&original)
        .into_iter()
        .find(|f| f.sig.ident == "ferric_qwen3_draft_batch32_mfma_gemm_bf16_v10")
        .unwrap();
    // Transform typed literals through parsed source, then assert the complete body.
    let mut source = original
        .replace("4096", "1024")
        .replace("n < 151937", "n < 3073")
        .replace("step < 256", "step < 64")
        .replace("loop_bounds(256)", "loop_bounds(64)")
        .replace("max_grid = [18992,", "max_grid = [384,");
    column = functions(&source)
        .into_iter()
        .find(|f| f.sig.ident == column.sig.ident)
        .unwrap();
    column.block.stmts[0] = syn::parse_str(&format!(
        "if rows == 0 || rows > 32 || !({COLUMN_SHAPE}) {{ fe2o3_device::trap(); }}"
    ))
    .unwrap();
    replace_tiles(&mut column);
    source = original
        .replace("4096", "1024")
        .replace("k < 12289", "k < 3073")
        .replace("tile_column < 256", "tile_column < 64")
        .replace("max_grid = [512,", "max_grid = [128,")
        .replace(
            "loop_bounds(32, 96, 128, 256, 384, 768)",
            "loop_bounds(128, 192)",
        )
        .replace("((rows + 15) / 16) * 256", "((rows + 15) / 16) * 64");
    let mut partial = functions(&source)
        .into_iter()
        .find(|f| f.sig.ident == "ferric_qwen3_draft_batch32_mfma_gemm_partial_f32_v10")
        .unwrap();
    partial.block.stmts[0] = syn::parse_str(&format!(
        "if rows == 0 || rows > 32 || !({PARTIAL_SHAPE}) {{ fe2o3_device::trap(); }}"
    ))
    .unwrap();
    replace_tiles(&mut partial);
    let loops = partial
        .block
        .stmts
        .iter()
        .position(|stmt| {
            let Stmt::Expr(syn::Expr::If(expr), _) = stmt else {
                return false;
            };
            *expr.cond == syn::parse_str::<syn::Expr>("k == 512").unwrap()
        })
        .unwrap();
    let body = "let a_fragment = left.load_m16k16(&lane, tile_row * 16, step * 16); let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16); accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator); step += 1;";
    // Every original branch uses this same fragment body; only K/16 changes.
    struct Loops<'a>(&'a syn::Block, usize);
    impl<'ast> syn::visit::Visit<'ast> for Loops<'_> {
        fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
            assert_eq!(&node.body, self.0);
            self.1 += 1;
        }
    }
    let body_block = syn::parse_str::<syn::Block>(&format!("{{ {body} }}")).unwrap();
    let mut visitor = Loops(&body_block, 0);
    syn::visit::Visit::visit_stmt(&mut visitor, &partial.block.stmts[loops]);
    assert_eq!(visitor.1, 6);
    partial.block.stmts[loops]=syn::parse_str(&format!("if k == 2048 {{ let mut step=0_usize; while step < 128 {{ {body} }} }} else {{ let mut step=0_usize; while step < 192 {{ {body} }} }}")).unwrap();
    assert_eq!(
        vec![column, partial],
        functions(include_str!("../src/mfma.rs"))
    );
}
