use quote::ToTokens as _;
use syn::{FnArg, Item, ItemFn, Meta, Type};

const SOURCE: &str = include_str!("../src/lib.rs");

fn kernel() -> ItemFn {
    let kernels: Vec<_> = syn::parse_file(SOURCE)
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
        .collect();
    assert_eq!(kernels.len(), 1);
    kernels.into_iter().next().unwrap()
}

fn named_macro(name: &str) -> syn::ItemMacro {
    syn::parse_file(SOURCE)
        .expect("device source parses as ordinary Rust")
        .items
        .into_iter()
        .find_map(|item| match item {
            Item::Macro(item)
                if item
                    .ident
                    .as_ref()
                    .is_some_and(|identifier| identifier == name) =>
            {
                Some(item)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing macro {name}"))
}

fn compact_function_body(function: ItemFn) -> String {
    function
        .block
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn compact_macro_body(item: syn::ItemMacro) -> String {
    item.mac
        .tokens
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

#[test]
fn source_has_one_exact_attributed_kernel_and_no_worker_escape_hatch() {
    let kernel = kernel();
    assert_eq!(kernel.sig.ident, "qwen3_swiglu_bf16_f32_v1");
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
fn attribute_pins_launch_grid_without_dynamic_control_contracts() {
    let kernel = kernel();
    let attribute = kernel
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"))
        .unwrap();
    let Meta::List(arguments) = &attribute.meta else {
        panic!("kernel attribute must carry the typed contract");
    };
    let tokens = arguments.tokens.to_string();
    assert!(tokens.contains("typed"));
    assert!(tokens.contains("required = [256 , 1 , 1]"));
    assert!(tokens.contains("max = [256 , 1 , 1]"));
    assert!(tokens.contains("max_grid = [12288 , 1 , 1]"));
    assert!(!tokens.contains("control_flow"));
}

#[test]
fn signature_retains_three_slice_records_and_physical_bf16_carriers() {
    let kernel = kernel();
    assert_eq!(kernel.sig.inputs.len(), 3);
    let arguments: Vec<String> = kernel
        .sig
        .inputs
        .iter()
        .map(|argument| match argument {
            FnArg::Typed(argument) => match argument.ty.as_ref() {
                Type::Reference(reference) => format!("&{}", quote_type(reference.elem.as_ref())),
                ty => quote_type(ty),
            },
            FnArg::Receiver(_) => panic!("kernel must not have a receiver"),
        })
        .collect();
    assert_eq!(arguments[0], "&[u16]");
    assert_eq!(arguments[1], "&[u16]");
    assert_eq!(arguments[2], "DisjointSlice<u16,Blocked<Index1D,1,8>>");
}

fn quote_type(ty: &Type) -> String {
    let mut text = String::new();
    match ty {
        Type::Path(path) => {
            for segment in &path.path.segments {
                text.push_str(&segment.ident.to_string());
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    text.push('<');
                    let mut first = true;
                    for argument in &arguments.args {
                        if !first {
                            text.push(',');
                        }
                        first = false;
                        match argument {
                            syn::GenericArgument::Type(ty) => text.push_str(&quote_type(ty)),
                            syn::GenericArgument::Const(syn::Expr::Lit(literal)) => {
                                let syn::Lit::Int(value) = &literal.lit else {
                                    panic!("const generic must be an integer literal");
                                };
                                text.push_str(value.base10_digits());
                            }
                            _ => panic!("unexpected generic argument"),
                        }
                    }
                    text.push('>');
                }
            }
        }
        Type::Slice(slice) => {
            text.push('[');
            text.push_str(&quote_type(slice.elem.as_ref()));
            text.push(']');
        }
        _ => panic!("unexpected kernel argument type"),
    }
    text
}

#[test]
fn element_expansion_retains_stable_sigmoid_and_fail_closed_narrowing() {
    let body = compact_macro_body(named_macro("qwen3_swiglu_element_v1"));
    for marker in [
        "Bf16::from_bits($gate_bits)",
        "Bf16::from_bits($up_bits)",
        "if!gate_value.is_finite()||!up_value.is_finite()",
        "letexponent_argument=ifnonnegative{-gate_f32}else{gate_f32}",
        "Math::current().exp_f32(exponent_argument)",
        "!(exponent>=f32::MIN&&exponent<=f32::MAX)||exponent<0.0",
        "letdenominator=1.0+exponent",
        "letnumerator=ifnonnegative{1.0}else{exponent}",
        "letsilu=gate_f32*sigmoid",
        "letproduct=silu*up_f32",
        "!(sigmoid>=f32::MIN&&sigmoid<=f32::MAX)",
        "!(silu>=f32::MIN&&silu<=f32::MAX)",
        "!(product>=f32::MIN&&product<=f32::MAX)",
        "Bf16::from_f32(product)",
        "if!narrowed.is_finite()",
        "narrowed.to_bits()",
    ] {
        assert!(
            body.contains(marker),
            "missing source-contract marker {marker}"
        );
    }
    assert_eq!(body.matches("Math::current().exp_f32(").count(), 1);
    assert_eq!(body.matches("Bf16::from_f32(product)").count(), 1);
    assert_eq!(body.matches("fe2o3_device::trap()").count(), 4);
    for forbidden in ["return", "break", "continue", "f32_is_finite_v1"] {
        assert!(
            !body.contains(forbidden),
            "found forbidden body marker {forbidden}"
        );
    }
}

#[test]
fn kernel_extent_expansion_is_closed_and_call_free() {
    let body = compact_macro_body(named_macro("qwen3_swiglu_extent_is_admitted_expr_v1"));
    for extent in [
        "3_072",
        "12_288",
        "24_576",
        "49_152",
        "61_440",
        "98_304",
        "110_592",
        "208_896",
        "393_216",
        "491_520",
        "1_572_864",
        "3_145_728",
        "6_291_456",
        "12_582_912",
        "25_165_824",
    ] {
        assert!(body.contains(&format!("$elements=={extent}")));
    }
    assert_eq!(body.matches("$elements==").count(), 15);
}

#[test]
fn kernel_uses_eight_constant_blocked_stores() {
    let body = compact_function_body(kernel());
    assert!(body.contains("qwen3_swiglu_extent_is_admitted_expr_v1!(elements)"));
    assert!(!body.contains("qwen3_swiglu_extent_is_admitted_v1("));
    assert!(body.contains("workitem.checked_block::<1,8>()"));
    for component in 0..8 {
        let index = format!("index_{component}");
        assert!(body.contains(&format!("if{index}<elements")));
        assert!(body.contains(&format!(
            "qwen3_swiglu_element_v1!(gate[{index}],up[{index}])"
        )));
        assert!(body.contains(&format!("output.get_block_mut(&output_block,{component})")));
    }
    assert_eq!(body.matches("qwen3_swiglu_element_v1!(").count(), 8);
    assert_eq!(body.matches("output.get_block_mut(").count(), 8);
    assert_eq!(body.matches("*slot=value;").count(), 8);
    assert_eq!(body.matches("fe2o3_device::trap()").count(), 10);
    for forbidden in ["while", "loop", "break", "continue", "macro_rules!"] {
        assert!(
            !body.contains(forbidden),
            "found forbidden body marker {forbidden}"
        );
    }
}
