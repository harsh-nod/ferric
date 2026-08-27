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
fn attribute_pins_launch_grid_and_loop_bound() {
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
    assert!(tokens.contains("loop_bounds (8)"));
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
fn body_retains_stable_sigmoid_and_eight_contiguous_stores() {
    for marker in [
        "workitem.checked_block::<1, 8>()",
        "while component < QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1",
        "Bf16::from_bits(gate[index])",
        "Bf16::from_bits(up[index])",
        "math.exp_f32(exponent_argument)",
        "let denominator = 1.0 + exponent",
        "let silu = gate_f32 * sigmoid",
        "let product = silu * up_f32",
        "Bf16::from_f32(product)",
        "output.get_block_mut(&output_block, component)",
    ] {
        assert!(
            SOURCE.contains(marker),
            "missing source-contract marker {marker}"
        );
    }
}
