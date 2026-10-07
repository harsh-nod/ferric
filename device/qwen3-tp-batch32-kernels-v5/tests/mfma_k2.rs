use fe2o3_device::{Bf16MatrixViewError, Bf16MfmaAMatrix, Bf16MfmaBMatrix};
use syn::{BinOp, Expr, Item, Lit, Pat, Stmt, visit::Visit};

const PROJECTION: &str = include_str!("../src/projection_k2.rs");

fn path(expr: &Expr) -> String {
    let Expr::Path(path) = expr else {
        panic!("expected a named value")
    };
    path.path.get_ident().unwrap().to_string()
}

fn integer(expr: &Expr) -> usize {
    let Expr::Lit(value) = expr else {
        panic!("expected an integer")
    };
    let Lit::Int(value) = &value.lit else {
        panic!("expected an integer literal")
    };
    value.base10_parse().unwrap()
}

fn initializer<'a>(body: &'a syn::Block, name: &str) -> &'a Expr {
    body.stmts
        .iter()
        .find_map(|statement| {
            let Stmt::Local(local) = statement else {
                return None;
            };
            let Pat::Ident(ident) = &local.pat else {
                return None;
            };
            (ident.ident == name).then(|| local.init.as_ref().unwrap().expr.as_ref())
        })
        .unwrap()
}

#[derive(Default)]
struct Loops<'a>(Vec<&'a syn::ExprWhile>);

impl<'a> Visit<'a> for Loops<'a> {
    fn visit_expr_while(&mut self, expression: &'a syn::ExprWhile) {
        self.0.push(expression);
        syn::visit::visit_expr_while(self, expression);
    }
}

#[derive(Default)]
struct FragmentEvents(Vec<String>);

impl<'a> Visit<'a> for FragmentEvents {
    fn visit_expr_method_call(&mut self, call: &'a syn::ExprMethodCall) {
        match call.method.to_string().as_str() {
            "load_m16k16" => {
                assert_eq!(path(&call.receiver), "left");
                self.0.push(format!("A:{}", path(&call.args[2])));
            }
            "load_k16n16" => {
                assert_eq!(path(&call.receiver), "right");
                self.0.push(format!("B:{}", path(&call.args[1])));
            }
            "multiply_accumulate" => {
                assert_eq!(path(&call.receiver), "matrix");
                self.0.push(format!(
                    "MFMA:{}:{}:{}",
                    path(&call.args[0]),
                    path(&call.args[1]),
                    path(&call.args[2])
                ));
            }
            _ => {}
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

#[test]
fn actual_mfma_loops_buffer_two_fragments_then_consume_in_original_order() {
    let parsed = syn::parse_file(PROJECTION).unwrap();
    let mut bounds = Vec::new();
    for item in &parsed.items {
        let Item::Fn(function) = item else {
            continue;
        };
        if !function.sig.ident.to_string().contains("batch32_mfma_gemm_") {
            continue;
        }
        let mut loops = Loops::default();
        loops.visit_block(&function.block);
        for repeated in loops.0 {
            let Expr::Binary(condition) = repeated.cond.as_ref() else {
                panic!("expected a bounded pair loop")
            };
            assert!(matches!(condition.op, BinOp::Lt(_)));
            assert_eq!(path(&condition.left), "pair");
            bounds.push(integer(&condition.right));
            for (name, operand, factor, multiply) in [
                ("reduction_base", "pair", 32, true),
                ("next_reduction_base", "reduction_base", 16, false),
            ] {
                let Expr::Binary(value) = initializer(&repeated.body, name) else {
                    panic!("expected the exact adjacent K16 coordinate")
                };
                assert_eq!(path(&value.left), operand);
                assert_eq!(integer(&value.right), factor);
                assert!(if multiply {
                    matches!(value.op, BinOp::Mul(_))
                } else {
                    matches!(value.op, BinOp::Add(_))
                });
            }
            let Some(Stmt::Expr(Expr::Binary(increment), _)) = repeated.body.stmts.last() else {
                panic!("expected one pair increment")
            };
            assert!(matches!(increment.op, BinOp::AddAssign(_)));
            assert_eq!(path(&increment.left), "pair");
            assert_eq!(integer(&increment.right), 1);
            let mut events = FragmentEvents::default();
            events.visit_block(&repeated.body);
            assert_eq!(
                events.0,
                [
                    "A:reduction_base",
                    "B:reduction_base",
                    "A:next_reduction_base",
                    "B:next_reduction_base",
                    "MFMA:a_fragment:b_fragment:accumulator",
                    "MFMA:next_a_fragment:next_b_fragment:accumulator",
                ]
            );
        }
    }
    assert_eq!(bounds, [128, 16, 48, 64, 128, 192, 384]);
    assert!(!PROJECTION.contains("WorkgroupPipeline"));
    assert!(!PROJECTION.contains("write_mfma_fragment"));
    assert_eq!(PROJECTION.matches("Bf16MfmaAMatrix::row_major(").count(), 2);
    assert_eq!(PROJECTION.matches("Bf16MfmaBMatrix::row_major(").count(), 2);
}

#[test]
fn actual_checked_matrix_constructors_reject_short_stride_extent_and_overflow() {
    let bits = [0_u16; 32];
    assert!(Bf16MfmaAMatrix::row_major(&bits, 0, 2, 16, 16).is_ok());
    assert!(Bf16MfmaBMatrix::row_major(&bits, 0, 16, 2, 2).is_ok());
    for error in [
        Bf16MfmaAMatrix::row_major(&bits, 0, 2, 16, 15).err(),
        Bf16MfmaBMatrix::row_major(&bits, 0, 16, 2, 1).err(),
    ] {
        assert_eq!(error, Some(Bf16MatrixViewError::InvalidStride));
    }
    for error in [
        Bf16MfmaAMatrix::row_major(&bits[..31], 0, 2, 16, 16).err(),
        Bf16MfmaBMatrix::row_major(&bits[..31], 0, 16, 2, 2).err(),
    ] {
        assert_eq!(
            error,
            Some(Bf16MatrixViewError::OutOfBounds {
                required: 32,
                actual: 31
            })
        );
    }
    assert_eq!(
        Bf16MfmaAMatrix::row_major(&bits, 1, 2, 1, usize::MAX).err(),
        Some(Bf16MatrixViewError::ExtentOverflow)
    );
    assert_eq!(
        Bf16MfmaBMatrix::row_major(&bits, 1, 2, 1, usize::MAX).err(),
        Some(Bf16MatrixViewError::ExtentOverflow)
    );
}
