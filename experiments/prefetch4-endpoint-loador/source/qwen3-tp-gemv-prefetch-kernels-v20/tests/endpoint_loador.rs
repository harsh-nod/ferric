use quote::ToTokens;
use syn::{Expr, Item, Stmt};

const BEFORE: &str = include_str!("fixtures/prefetch4_before_endpoint.rs.txt");
const AFTER: &str = include_str!("../src/projection.rs");
const K_VALUES: [usize; 6] = [512, 1536, 2048, 4096, 6144, 12288];

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn expected_source() -> syn::File {
    let mut file = syn::parse_file(BEFORE).unwrap();
    let function = file.items.iter_mut().find_map(|item| match item {
        Item::Fn(function)
            if function.sig.ident == "ferric_qwen3_tp_wave_gemv_prefetch4_partial_f32_v20" => Some(function),
        _ => None,
    }).unwrap();
    let loop_index = function.block.stmts.iter().position(|statement| {
        matches!(statement, Stmt::Expr(Expr::While(_), _))
    }).unwrap();
    function.block.stmts.insert(loop_index - 1, syn::parse_quote! {
        if row >= rows || column >= 4096 || row * k + k > a.len()
            || column * k + k > weights.len()
        {
            fe2o3_device::trap();
        }
    });
    let statement = function.block.stmts.iter_mut().find(|statement| {
        matches!(statement, Stmt::Expr(Expr::While(_), _))
    }).unwrap();
    let Stmt::Expr(Expr::While(old), _) = statement else { unreachable!() };
    assert_eq!(old.body.stmts.len(), 13);
    let mut replacement: syn::ExprWhile = syn::parse_quote! {
        while group < 48 {
            let inner_0 = group * 256 + lane;
            let inner_1 = inner_0 + 64;
            let inner_2 = inner_0 + 128;
            let inner_3 = inner_0 + 192;
            if inner_3 < k {
                let left_bits_0 = left_view.load_or(row, inner_0 as u16 as usize, 0);
                let right_bits_0 = right_view.load_or(column, inner_0 as u16 as usize, 0);
                let left_bits_1 = left_view.load_or(row, inner_1 as u16 as usize, 0);
                let right_bits_1 = right_view.load_or(column, inner_1 as u16 as usize, 0);
                let left_bits_2 = left_view.load_or(row, inner_2 as u16 as usize, 0);
                let right_bits_2 = right_view.load_or(column, inner_2 as u16 as usize, 0);
                let left_bits_3 = left_view.load_or(row, inner_3 as u16 as usize, 0);
                let right_bits_3 = right_view.load_or(column, inner_3 as u16 as usize, 0);
            }
            group += 1;
        }
    };
    let Stmt::Expr(Expr::If(active), _) = &mut replacement.body.stmts[4] else {
        unreachable!()
    };
    // Reuse the exact four old arithmetic blocks in order; do not restate their math.
    for statement in &old.body.stmts[8..12] {
        let Stmt::Expr(Expr::If(branch), _) = statement else { unreachable!() };
        assert_eq!(branch.then_branch.stmts.len(), 3);
        active.then_branch.stmts.extend(branch.then_branch.stmts.clone());
    }
    *old = replacement;
    file
}

fn same_source(source: &str) -> bool {
    tokens(&syn::parse_file(source).unwrap()) == tokens(&expected_source())
}

#[test]
fn complete_source_diff_is_only_uniform_endpoints_and_joint_checked_partial_loads() {
    assert!(same_source(AFTER));
    assert!(!same_source(BEFORE));
}

#[test]
fn source_contract_rejects_guard_load_order_math_and_output_drift() {
    for (before, after) in [
        ("if inner_3 < k", "if inner_3 <= k"),
        ("row >= rows", "row > rows"),
        ("column >= 4096", "column > 4096"),
        ("row * k + k > a.len()", "row * k > a.len()"),
        ("column * k + k > weights.len()", "column * k > weights.len()"),
        ("left_view.load_or(row, inner_1 as u16 as usize, 0)", "left_view.load_or(row, inner_2 as u16 as usize, 0)"),
        ("right_view.load_or(column, inner_0 as u16 as usize, 0)", "right_view.load_or(row, inner_0 as u16 as usize, 0)"),
        ("right_view.load_or(column, inner_2 as u16 as usize, 0)", "right_view.load_or(column, inner_2 as u16 as usize, 1)"),
        ("partial += product_2;", "partial -= product_2;"),
        ("finite &= product_3.is_finite() & partial.is_finite();", "finite = partial.is_finite();"),
        ("while group < 48", "while group < 47"),
        ("if !finite || !sum.is_finite()", "if !sum.is_finite()"),
        ("rows * 4096, 1, 1, sum", "rows * 4096, 1, 1, partial"),
    ] {
        assert!(AFTER.contains(before));
        assert!(!same_source(&AFTER.replace(before, after)), "accepted mutation {before}");
    }
}

#[test]
fn all_admitted_k_lane_and_group_predicates_are_equivalent() {
    for k in K_VALUES {
        assert_eq!(k % 256, 0);
        for lane in 0..64 {
            for group in 0..48 {
                let active = group * 256 + lane + 192 < k;
                for offset in [0, 64, 128, 192] {
                    assert_eq!(group * 256 + lane + offset < k, active);
                }
                if active {
                    assert!(group * 256 + lane + 192 < k);
                }
            }
        }
    }
}

#[test]
fn endpoints_cover_every_admitted_row_column_and_offset_without_overflow() {
    for k in K_VALUES {
        for rows in 1..33 {
            for row in 0..rows {
                let end = row * k + k;
                assert!(end <= rows * k);
                for lane in 0..64 {
                    let greatest = row * k + (k / 256 - 1) * 256 + lane + 192;
                    assert!(greatest < end);
                    assert!(greatest < rows * k);
                    let inner = (k / 256 - 1) * 256 + lane + 192;
                    assert_eq!(inner as u16 as usize, inner);
                }
            }
        }
        for column in 0..4096 {
            let greatest = column * k + (k / 256 - 1) * 256 + 63 + 192;
            assert_eq!(greatest, (column + 1) * k - 1);
            assert!(greatest < 4096 * k);
            assert!(greatest < u32::MAX as usize);
        }
    }
}

fn value(index: usize, case: usize, weight: bool) -> f32 {
    let ordinary = [0x3f80_u16, 0xbf80, 0x3e80, 0xbe80, 0x0000, 0x8000, 0x3fc0, 0xbfc0];
    let special = [0x0001_u16, 0x007f, 0x0080, 0x7f7f, 0xff7f, 0x7f80, 0xff80, 0x7fc1];
    let bits = if case == 0 {
        ordinary[(index + usize::from(weight) * 3) % ordinary.len()]
    } else if index.is_multiple_of(257) {
        special[case - 1]
    } else {
        ordinary[(index + usize::from(weight)) % ordinary.len()]
    };
    f32::from_bits(u32::from(bits) << 16)
}

fn lane_steps(k: usize, lane: usize, case: usize, grouped: bool) -> Vec<(u32, bool)> {
    let mut partial = 0.0_f32;
    let mut finite = true;
    let mut steps = Vec::new();
    let read = |index: usize, weight: bool| {
        // A poisoned inactive tail is not a legal fallback/dummy read.
        assert!(index < k, "inactive-tail read");
        value(index, case, weight)
    };
    for group in 0..48 {
        let inner = [group * 256 + lane, group * 256 + lane + 64,
            group * 256 + lane + 128, group * 256 + lane + 192];
        let mut pairs = [(0.0_f32, 0.0_f32); 4];
        for (index, pair) in inner.into_iter().zip(&mut pairs) {
            if if grouped { inner[3] < k } else { index < k } {
                *pair = (read(index, false), read(index, true));
            }
        }
        for (index, (left, right)) in inner.into_iter().zip(pairs) {
            if if grouped { inner[3] < k } else { index < k } {
                let product = left * right;
                partial += product;
                finite &= product.is_finite() & partial.is_finite();
                steps.push((partial.to_bits(), finite));
            }
        }
    }
    steps
}

#[test]
fn every_active_step_and_finite_flag_match_with_exceptional_bf16_values() {
    for k in K_VALUES {
        for lane in 0..64 {
            for case in 0..9 {
                let old = lane_steps(k, lane, case, false);
                let new = lane_steps(k, lane, case, true);
                assert_eq!(old, new, "k={k} lane={lane} case={case}");
                assert_eq!(new.len(), k / 64);
            }
        }
    }
}

#[test]
fn inactive_groups_never_access_poisoned_input_tails() {
    for k in K_VALUES {
        for lane in 0..64 {
            let steps = lane_steps(k, lane, 0, true);
            assert_eq!(steps.len(), k / 64);
        }
    }
}
