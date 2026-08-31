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
fn attribute_pins_wave64_grid_and_serial_reduction_bound() {
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
        "control_flow(loop_bounds(4096,64))",
    ] {
        assert!(tokens.contains(marker), "missing attribute marker {marker}");
    }
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
        "rows<=QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1",
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
    let first_serial_load = body
        .find("memory::volatile_load(input_bf16,index)")
        .unwrap();
    let collective = body
        .find("collectives.subgroup_reduce_sum_f32::<64>(local_sum)")
        .unwrap();
    assert!(validation_trap < first_serial_load);
    assert!(validation_trap < collective);
}

#[test]
fn wave_math_and_row_striped_writes_retain_the_exact_formula_boundaries() {
    let body = compact_tokens(&kernel().block);
    for marker in [
        "WaveLane::<Wave64>::current()",
        "Gfx942Collectives::current()",
        "iflane_index==0",
        "letmutcolumn=0_usize",
        "whilecolumn<widthasusize",
        "letsquare=normalized_input*normalized_input",
        "letnext_sum=local_sum+square",
        "local_sum=next_sum",
        "column+=1",
        "letsum=collectives.subgroup_reduce_sum_f32::<64>(local_sum)",
        "letmean_square=sum/widthasf32",
        "letstabilized=mean_square+epsilon",
        "letdenominator=Math::current().sqrt_f32(stabilized)",
        "letinverse_rms=1.0_f32/denominator",
        "checked_row_striped_2d::<64,64>()",
        "letmutcomponent=0_usize",
        "whilecomponent<64",
        "letcolumn=lane_index+component*64",
        "memory::volatile_load(input_bf16,index)",
        "memory::volatile_load(residual_bf16,index)",
        "memory::volatile_load(weight_bf16,column)",
        "letnarrowed_fused=Bf16::from_f32(fused)",
        "letnormalized=normalized_input*inverse_rms",
        "letnarrowed_weighted=Bf16::from_f32(weighted)",
    ] {
        assert!(body.contains(marker), "missing numerical marker {marker}");
    }
    assert_eq!(
        body.matches("collectives.subgroup_reduce_sum_f32::<64>(local_sum)")
            .count(),
        1
    );
    assert_eq!(body.matches("whilecolumn<widthasusize").count(), 1);
    assert_eq!(body.matches("whilecomponent<64").count(), 1);
    assert_eq!(body.matches("component+=1").count(), 1);
    assert_eq!(body.matches("write_row_striped_2d(").count(), 2);
}

#[test]
fn numerical_path_traps_nonfinite_inputs_intermediates_and_bf16_outputs() {
    let body = compact_tokens(&kernel().block);
    for marker in [
        "if!input.is_finite()",
        "if!residual.is_finite()",
        "if!fused.is_finite()",
        "if!square.is_finite()||!next_sum.is_finite()",
        "if!sum.is_finite()",
        "if!mean_square.is_finite()||!stabilized.is_finite()||stabilized<=0.0",
        "if!denominator.is_finite()||denominator<=0.0",
        "if!inverse_rms.is_finite()",
        "if!narrowed_fused.is_finite()",
        "if!weight.is_finite()",
        "if!normalized.is_finite()||!weighted.is_finite()",
        "if!narrowed_weighted.is_finite()",
    ] {
        assert!(body.contains(marker), "missing finite trap marker {marker}");
    }
    assert!(body.matches("fe2o3_device::trap()").count() >= 15);
}

#[test]
fn every_shared_observation_uses_the_bounded_volatile_terminal() {
    let body = compact_tokens(&kernel().block);
    assert_eq!(body.matches("memory::volatile_load(").count(), 5);
    for forbidden in [
        "input_bf16[",
        "residual_bf16[",
        "weight_bf16[",
        "SubgroupTile",
        "wave.reduce_sum(",
        "qwen3_rmsnorm_accumulate_component_v1",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "shared-memory or reduction custody regressed through {forbidden}"
        );
    }
    let lane_zero = body.find("iflane_index==0").unwrap();
    let serial_loop = body.find("whilecolumn<widthasusize").unwrap();
    let collective = body
        .find("collectives.subgroup_reduce_sum_f32::<64>(local_sum)")
        .unwrap();
    let write_loop = body.find("whilecomponent<64").unwrap();
    assert!(lane_zero < serial_loop && serial_loop < collective && collective < write_loop);
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
