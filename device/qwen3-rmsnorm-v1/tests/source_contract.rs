use quote::ToTokens;
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

fn compact_tokens(tokens: impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
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

#[test]
fn source_has_one_exact_attributed_kernel_and_no_artifact_escape_hatch() {
    let kernel = kernel();
    assert_eq!(kernel.sig.ident, "qwen3_rmsnorm_v1");
    let lowercase = SOURCE.to_ascii_lowercase();
    for forbidden in [
        "compilerhandoff",
        "pinnedworker",
        "std::process",
        "command::new",
        "include_bytes!",
        "llvm assembly",
        "launch authority",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "found forbidden source marker {forbidden}"
        );
    }
}

#[test]
fn signature_retains_exact_slice_order_access_and_scalar_tail() {
    let kernel = kernel();
    assert_eq!(kernel.sig.inputs.len(), 9);
    let arguments = kernel
        .sig
        .inputs
        .iter()
        .map(|argument| match argument {
            FnArg::Typed(argument) => compact_tokens(argument.ty.as_ref()),
            FnArg::Receiver(_) => panic!("kernel must not have a receiver"),
        })
        .collect::<Vec<_>>();
    assert_eq!(arguments[0], "&[u16]");
    assert_eq!(arguments[1], "&[u16]");
    assert_eq!(arguments[2], "&[u16]");
    assert_eq!(
        arguments[3],
        "WriteOnlyDisjointSlice<u16,RowStriped2D<Index1D,64,64>>"
    );
    assert_eq!(arguments[4], arguments[3]);
    assert_eq!(&arguments[5..], ["u32", "u32", "f32", "u32"]);

    for (argument, expected_name) in kernel.sig.inputs.iter().zip([
        "input_bf16",
        "residual_bf16",
        "weight_bf16",
        "fused_residual_bf16",
        "normalized_bf16",
        "rows",
        "width",
        "epsilon",
        "behavior",
    ]) {
        let FnArg::Typed(argument) = argument else {
            panic!("unexpected receiver")
        };
        let syn::Pat::Ident(name) = argument.pat.as_ref() else {
            panic!("argument must retain a simple ABI name")
        };
        assert_eq!(name.ident, expected_name);
    }
}

#[test]
fn attribute_pins_wave64_grid_bound_without_a_dynamic_loop_contract() {
    let kernel = kernel();
    let attribute = kernel
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"))
        .unwrap();
    let Meta::List(arguments) = &attribute.meta else {
        panic!("kernel attribute must carry the typed contract")
    };
    let tokens = compact_tokens(&arguments.tokens);
    for marker in [
        "typed",
        "required=[64,1,1]",
        "max=[64,1,1]",
        "max_grid=[65536,1,1]",
    ] {
        assert!(tokens.contains(marker), "missing attribute marker {marker}");
    }
    assert!(!tokens.contains("control_flow"));
}

#[test]
fn kernel_authenticates_shape_lengths_epsilon_and_exact_grid_before_collective() {
    let body = compact_tokens(&kernel().block);
    for marker in [
        "behavior==QWEN3_RMSNORM_BEHAVIOR_PURE_V1",
        "behavior==QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1",
        "width==128",
        "width==1_024",
        "width==4_096",
        "input_bf16.len()==elements",
        "weight_bf16.len()==widthasusize",
        "normalized_bf16.len()==elements",
        "pure_mode&&residual_bf16.len()==0&&fused_residual_bf16.len()==0",
        "fused_mode&&residual_bf16.len()==elements&&fused_residual_bf16.len()==elements",
        "epsilon.to_bits()!=QWEN3_RMSNORM_EPSILON_BITS_V1",
        "thread::grid_dim_x()==rows",
        "thread::grid_dim_y()==1",
        "thread::grid_dim_z()==1",
    ] {
        assert!(body.contains(marker), "missing admission marker {marker}");
    }
    let validation_trap = body.find("if!shape_valid").unwrap();
    let first_expansion = body
        .find("qwen3_rmsnorm_accumulate_component_v1!(")
        .unwrap();
    let collective = body.find("wave.reduce_sum(").unwrap();
    assert!(validation_trap < first_expansion);
    assert!(validation_trap < collective);
}

#[test]
fn wave_math_and_row_striped_writes_retain_the_exact_formula_boundaries() {
    let body = compact_tokens(&kernel().block);
    let accumulation = compact_tokens(named_macro("qwen3_rmsnorm_accumulate_component_v1"));
    let write = compact_tokens(named_macro("qwen3_rmsnorm_write_component_v1"));
    for marker in [
        "WaveLane::<Wave64>::current()",
        "SubgroupTile::<64>::from_wave64_snapshot(&lane)",
        "Gfx942Collectives::current()",
        "letsum=wave.reduce_sum(&collectives,local_sum)",
        "letmean_square=sum/widthasf32",
        "1.0_f32/Math::current().sqrt_f32(mean_square+epsilon)",
        "checked_row_striped_2d::<64,64>()",
    ] {
        assert!(body.contains(marker), "missing numerical marker {marker}");
    }
    for marker in [
        "letcolumn=$lane_index+$component*64",
        "$local_sum+=normalized_input*normalized_input",
    ] {
        assert!(
            accumulation.contains(marker),
            "missing accumulation marker {marker}"
        );
    }
    for marker in [
        "letcolumn=$lane_index+$component*64",
        "Bf16::from_f32(fused).to_bits()",
        "letnormalized=normalized_input*$inverse_rms",
        "letweighted=normalized*Bf16::from_bits($weight[column]).to_f32()",
        "Bf16::from_f32(weighted).to_bits()",
    ] {
        assert!(write.contains(marker), "missing write marker {marker}");
    }
    let roster = (0..64)
        .map(|component| component.to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        body.matches("qwen3_rmsnorm_accumulate_component_v1!(")
            .count(),
        1
    );
    assert_eq!(
        body.matches("qwen3_rmsnorm_write_component_v1!(").count(),
        1
    );
    assert!(body.contains(&format!("local_sum;{roster},)")));
    assert!(body.contains(&format!("normalized_bf16;{roster},)")));
    assert_eq!(body.matches("wave.reduce_sum(").count(), 1);
    assert_eq!(write.matches("write_row_striped_2d(").count(), 2);
}

#[test]
fn host_build_exposes_exact_write_only_adapter_and_both_behavior_shapes() {
    use fe2o3_host::__generated::{
        CompilerGeneratedKernelExpectationV1, CompilerGeneratedKfdArguments, GeneratedKfdReadSlice,
        GeneratedKfdWriteSlice,
    };
    use ferric_qwen3_rmsnorm_device_v1::qwen3_rmsnorm_v1_gpu::{Arguments, Marker};

    fn assert_kfd_adapter<'allocation, K, A>()
    where
        K: CompilerGeneratedKernelExpectationV1,
        A: CompilerGeneratedKfdArguments<'allocation, K>,
    {
    }

    assert_kfd_adapter::<
        Marker,
        Arguments<
            'static,
            GeneratedKfdReadSlice<'static, u16>,
            GeneratedKfdReadSlice<'static, u16>,
            GeneratedKfdReadSlice<'static, u16>,
            GeneratedKfdWriteSlice<'static, u16>,
            GeneratedKfdWriteSlice<'static, u16>,
        >,
    >();

    let pure_input = [0_u16; 128];
    let pure_residual: [u16; 0] = [];
    let pure_weight = [0_u16; 128];
    let mut pure_fused_output: [u16; 0] = [];
    let mut pure_normalized_output = [0_u16; 128];
    let pure_residual = GeneratedKfdReadSlice::new(&pure_residual);
    let pure_fused_output = GeneratedKfdWriteSlice::new(&mut pure_fused_output);
    assert!(pure_residual.is_empty());
    assert!(pure_fused_output.is_empty());
    let _pure = Arguments::new(
        GeneratedKfdReadSlice::new(&pure_input),
        pure_residual,
        GeneratedKfdReadSlice::new(&pure_weight),
        pure_fused_output,
        GeneratedKfdWriteSlice::new(&mut pure_normalized_output),
        1,
        128,
        f32::from_bits(ferric_qwen3_rmsnorm_device_v1::QWEN3_RMSNORM_EPSILON_BITS_V1),
        ferric_qwen3_rmsnorm_device_v1::QWEN3_RMSNORM_BEHAVIOR_PURE_V1,
    );

    let fused_input = [0_u16; 1_024];
    let fused_residual = [0_u16; 1_024];
    let fused_weight = [0_u16; 1_024];
    let mut fused_output = [0_u16; 1_024];
    let mut fused_normalized = [0_u16; 1_024];
    let _fused = Arguments::new(
        GeneratedKfdReadSlice::new(&fused_input),
        GeneratedKfdReadSlice::new(&fused_residual),
        GeneratedKfdReadSlice::new(&fused_weight),
        GeneratedKfdWriteSlice::new(&mut fused_output),
        GeneratedKfdWriteSlice::new(&mut fused_normalized),
        1,
        1_024,
        f32::from_bits(ferric_qwen3_rmsnorm_device_v1::QWEN3_RMSNORM_EPSILON_BITS_V1),
        ferric_qwen3_rmsnorm_device_v1::QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1,
    );
}

#[test]
fn output_arguments_are_write_only_types_not_mutable_slice_substitutes() {
    let kernel = kernel();
    for argument in kernel.sig.inputs.iter().skip(3).take(2) {
        let FnArg::Typed(argument) = argument else {
            panic!("unexpected receiver")
        };
        assert!(matches!(argument.ty.as_ref(), Type::Path(_)));
        let text = compact_tokens(argument.ty.as_ref());
        assert!(text.starts_with("WriteOnlyDisjointSlice<"));
        assert!(!text.starts_with("DisjointSlice<"));
        assert!(!text.starts_with("&mut"));
    }
}
