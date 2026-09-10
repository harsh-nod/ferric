use ferric_qwen3_tp_kernels_device_v1::contract;
use syn::{FnArg, GenericArgument, Item, Pat, PathArguments, Type};

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Reference(reference) => {
            assert!(reference.mutability.is_none());
            let Type::Slice(slice) = reference.elem.as_ref() else {
                panic!("not a slice")
            };
            format!("ro:{}", type_name(&slice.elem))
        }
        Type::Path(path) => {
            let segment = path.path.segments.last().unwrap();
            if segment.ident == "WriteOnlyDisjointSlice" {
                let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                    panic!("no output type")
                };
                let GenericArgument::Type(element) = arguments.args.first().unwrap() else {
                    panic!("bad output element")
                };
                format!("wo:{}", type_name(element))
            } else {
                assert!(matches!(segment.arguments, PathArguments::None));
                segment.ident.to_string()
            }
        }
        _ => panic!("unsupported kernel ABI type"),
    }
}

fn new_signatures() -> Vec<(String, Vec<(String, String)>)> {
    let mut result = Vec::new();
    for source in [
        include_str!("../src/projection.rs"),
        include_str!("../src/activation.rs"),
        include_str!("../src/rope_kv.rs"),
        include_str!("../src/attention.rs"),
    ] {
        for item in syn::parse_file(source).unwrap().items {
            let Item::Fn(function) = item else { continue };
            if !function
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("kernel"))
            {
                continue;
            }
            assert!(
                !function
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("cfg"))
            );
            let arguments = function
                .sig
                .inputs
                .iter()
                .map(|argument| {
                    let FnArg::Typed(argument) = argument else {
                        panic!("receiver")
                    };
                    let Pat::Ident(name) = argument.pat.as_ref() else {
                        panic!("pattern")
                    };
                    (name.ident.to_string(), type_name(&argument.ty))
                })
                .collect();
            result.push((function.sig.ident.to_string(), arguments));
        }
    }
    result
}

#[test]
fn six_new_roots_have_exact_element_units_and_scalar_order() {
    type ExpectedAbi = (&'static str, &'static [(&'static str, &'static str)], u32);
    let expected: [ExpectedAbi; 6] = [
        (
            "ferric_qwen3_tp_gemv_bf16_f32_bf16_v1",
            &[
                ("a", "ro:u16"),
                ("weights", "ro:u16"),
                ("output", "wo:u16"),
                ("n", "u32"),
                ("k", "u32"),
                ("model_role", "u32"),
                ("world_size", "u32"),
                ("projection", "u32"),
            ],
            contract::GEMV_EXPLICIT_BYTES,
        ),
        (
            "ferric_qwen3_tp_gemv_partial_bf16_f32_v1",
            &[
                ("a", "ro:u16"),
                ("weights", "ro:u16"),
                ("output", "wo:f32"),
                ("n", "u32"),
                ("k", "u32"),
                ("model_role", "u32"),
                ("world_size", "u32"),
                ("projection", "u32"),
            ],
            contract::GEMV_EXPLICIT_BYTES,
        ),
        (
            "ferric_qwen3_tp_swiglu_bf16_f32_v1",
            &[
                ("gate", "ro:u16"),
                ("up", "ro:u16"),
                ("output", "wo:u16"),
                ("model_role", "u32"),
                ("world_size", "u32"),
            ],
            contract::SWIGLU_EXPLICIT_BYTES,
        ),
        (
            "ferric_qwen3_tp_rope_v1",
            &[
                ("query", "ro:u16"),
                ("key", "ro:u16"),
                ("cos", "ro:f32"),
                ("sin", "ro:f32"),
                ("rotated_query", "wo:u16"),
                ("rotated_key", "wo:u16"),
                ("position", "u32"),
                ("model_role", "u32"),
                ("world_size", "u32"),
            ],
            contract::ROPE_EXPLICIT_BYTES,
        ),
        (
            "ferric_qwen3_tp_kv_append_v1",
            &[
                ("key", "ro:u16"),
                ("value", "ro:u16"),
                ("key_cache", "wo:u16"),
                ("value_cache", "wo:u16"),
                ("position", "u32"),
                ("capacity", "u32"),
                ("model_role", "u32"),
                ("world_size", "u32"),
            ],
            contract::KV_APPEND_EXPLICIT_BYTES,
        ),
        (
            "ferric_qwen3_tp_gqa_decode_bf16_f32_v1",
            &[
                ("query", "ro:u16"),
                ("key_cache", "ro:u16"),
                ("value_cache", "ro:u16"),
                ("output", "wo:u16"),
                ("count", "u32"),
                ("capacity", "u32"),
                ("model_role", "u32"),
                ("world_size", "u32"),
            ],
            contract::ATTENTION_EXPLICIT_BYTES,
        ),
    ];
    let signatures = new_signatures();
    assert_eq!(signatures.len(), expected.len());
    for (name, arguments, bytes) in expected {
        let actual = &signatures.iter().find(|entry| entry.0 == name).unwrap().1;
        assert_eq!(
            actual
                .iter()
                .map(|(name, ty)| (name.as_str(), ty.as_str()))
                .collect::<Vec<_>>(),
            arguments
        );
        assert_eq!(
            actual
                .iter()
                .map(|(_, ty)| if ty.contains(':') { 16 } else { 4 })
                .sum::<u32>(),
            bytes
        );
    }
}

#[test]
fn imported_roots_and_new_roots_match_exact_thirteen_symbol_contract() {
    let mut roots: Vec<_> = new_signatures().into_iter().map(|entry| entry.0).collect();
    for source in [
        include_str!("../../qwen3-all-kernels-v1/src/gemm.rs"),
        include_str!("../../qwen3-all-kernels-v1/src/logits.rs"),
        include_str!("../../qwen3-all-kernels-v1/src/rmsnorm.rs"),
    ] {
        roots.extend(
            syn::parse_file(source)
                .unwrap()
                .items
                .into_iter()
                .filter_map(|item| {
                    let Item::Fn(function) = item else {
                        return None;
                    };
                    function
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("kernel"))
                        .then(|| function.sig.ident.to_string())
                }),
        );
    }
    roots.sort();
    assert_eq!(roots, contract::KERNEL_SYMBOLS);
}
