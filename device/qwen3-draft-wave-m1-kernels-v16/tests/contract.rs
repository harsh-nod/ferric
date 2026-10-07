use ferric_qwen3_draft_wave_m1_kernels_device_v16::{
    compiler_expectation_roster_v16,
    contract::{self, OutputCarrier, ProjectionRole, ROOTS_V16},
};
use quote::{ToTokens, quote};
use syn::{Block, Expr, FnArg, Item, ItemFn, Pat, Stmt};

const SOURCE: &str = include_str!("../src/projection.rs");
const ROLES: [ProjectionRole; 8] = [
    ProjectionRole::Query,
    ProjectionRole::Key,
    ProjectionRole::Value,
    ProjectionRole::Gate,
    ProjectionRole::Up,
    ProjectionRole::AttentionOutput,
    ProjectionRole::Down,
    ProjectionRole::Head,
];

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn functions(source: &str) -> Vec<ItemFn> {
    syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Fn(function) => Some(function),
            _ => None,
        })
        .collect()
}

fn binding(statement: &Stmt, name: &str) -> bool {
    matches!(statement, Stmt::Local(local) if matches!(&local.pat,
        Pat::Ident(pattern) if pattern.ident == name))
}

fn verify(source: &str) -> Result<(), &'static str> {
    let functions = functions(source);
    if functions.len() != 2 {
        return Err("root count");
    }
    for (index, function) in functions.iter().enumerate() {
        if function.sig.ident != ROOTS_V16[index] || function.sig.unsafety.is_some() {
            return Err("root identity");
        }
        let expected_entry: Stmt = if index == 0 {
            syn::parse_quote! {
                if rows != 1 || world_size != 1 || k != 1024
                    || !((projection == 1 && n == 2048)
                        || ((projection == 2 || projection == 3) && n == 1024)
                        || ((projection == 4 || projection == 5) && n == 3072)) {
                    fe2o3_device::trap();
                }
            }
        } else {
            syn::parse_quote! {
                if rows != 1 || world_size != 1
                    || !((n == 1024 && ((projection == 1 && k == 2048)
                        || (projection == 2 && k == 3072)))
                        || (n == 151936 && projection == 6 && k == 1024)) {
                    fe2o3_device::trap();
                }
            }
        };
        if tokens(&function.block.stmts[0]) != tokens(&expected_entry) {
            return Err("shape");
        }
        let width: Expr = if index == 0 {
            syn::parse_quote!(1024)
        } else {
            syn::parse_quote!(k)
        };
        let lengths: Stmt = syn::parse_quote! {
            if a.len() < #width || a.len() > 32 * #width
                || weights.len() != n * #width || output.len() < n || output.len() > 32 * n
                || thread::grid_dim_x() as usize != n
                || thread::grid_dim_y() != 1 || thread::grid_dim_z() != 1
                || thread::block_dim_x() != 64 {
                fe2o3_device::trap();
            }
        };
        if !function
            .block
            .stmts
            .iter()
            .any(|s| tokens(s) == tokens(&lengths))
        {
            return Err("length or grid");
        }
        for expected in [
            quote!(let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, 1, #width, #width)
                else { fe2o3_device::trap(); };),
            quote!(let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, n, #width, #width)
                else { fe2o3_device::trap(); };),
        ] {
            if !function
                .block
                .stmts
                .iter()
                .any(|s| tokens(s) == expected.to_string())
            {
                return Err("guarded view");
            }
        }
        let begin = function
            .block
            .stmts
            .iter()
            .position(|s| binding(s, "invocation"))
            .ok_or("invocation")?;
        let end = function
            .block
            .stmts
            .iter()
            .position(|s| binding(s, "sum"))
            .ok_or("sum")?;
        let product: Block = syn::parse_quote!({
            let left = Bf16::from_bits(left_view.load_or(0, inner, 0x7fc0)).to_f32();
            let right = Bf16::from_bits(right_view.load_or(column, inner, 0x7fc0)).to_f32();
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
        });
        let product = product.stmts;
        let (bound, body) = if index == 0 {
            (quote!(16), quote!(#(#product)*))
        } else {
            (quote!(48), quote!(if inner < k { #(#product)* }))
        };
        let expected: Block = syn::parse2(quote!({
            let invocation = thread::index_1d();
            let lane = invocation.get() % 64;
            let column = thread::block_idx_x() as usize;
            let subgroup = Gfx950Subgroup::current();
            let mut partial = 0.0_f32;
            let mut finite = true;
            let mut step = 0_usize;
            while step < #bound {
                let inner = step * 64 + lane;
                #body
                step += 1;
            }
        }))
        .unwrap();
        let actual = &function.block.stmts[begin..end];
        let expected = expected.stmts;
        if quote!(#(#actual)*).to_string() != quote!(#(#expected)*).to_string() {
            return Err("convergent lane arithmetic");
        }
        let (narrowing, rejection, stored) = if index == 0 {
            (
                quote!(let narrowed = Bf16::from_f32(sum);),
                quote!(any_invalid != 0.0 || !sum.is_finite() || !narrowed.is_finite()),
                quote!(narrowed.to_bits()),
            )
        } else {
            (
                quote!(),
                quote!(any_invalid != 0.0 || !sum.is_finite()),
                quote!(sum),
            )
        };
        let expected: Block = syn::parse2(quote!({
            let sum = subgroup.reduce_sum_f32::<64>(partial);
            let invalid = if finite { 0.0_f32 } else { 1.0_f32 };
            let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
            #narrowing
            if #rejection { fe2o3_device::trap(); }
            if lane == 0 {
                let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
                    fe2o3_device::trap();
                };
                if !output.write_row_striped_2d(&stripe, 0, n, 1, 1, #stored) {
                    fe2o3_device::trap();
                }
            }
        }))
        .unwrap();
        let actual = &function.block.stmts[end..];
        let expected = expected.stmts;
        if quote!(#(#actual)*).to_string() != quote!(#(#expected)*).to_string() {
            return Err("collectives or store");
        }
    }
    Ok(())
}

#[test]
fn two_distinct_roots_preserve_typed_abi_and_exact_launch_bounds() {
    verify(SOURCE).unwrap();
    let roster = compiler_expectation_roster_v16();
    assert_eq!(roster.len(), 2);
    assert!(roster[0].kernel_binding_id() < roster[1].kernel_binding_id());
    let mut exports: Vec<_> = roster.iter().map(|entry| entry.export_name()).collect();
    exports.sort_unstable();
    let mut expected = ROOTS_V16;
    expected.sort_unstable();
    assert_eq!(exports, expected);
    for (index, function) in functions(SOURCE).iter().enumerate() {
        let output = if index == 0 { quote!(u16) } else { quote!(f32) };
        let expected: ItemFn = syn::parse2(quote! {
            fn expected(a: &[u16], weights: &[u16],
                mut output: WriteOnlyDisjointSlice<#output, RowStriped2D<Index1D, 64, 1>>,
                rows: u32, n: u32, k: u32, world_size: u32, projection: u32) {}
        })
        .unwrap();
        assert_eq!(
            function.sig.inputs.iter().map(tokens).collect::<Vec<_>>(),
            expected.sig.inputs.iter().map(tokens).collect::<Vec<_>>()
        );
        assert!(
            function
                .sig
                .inputs
                .iter()
                .all(|arg| matches!(arg, FnArg::Typed(_)))
        );
        let grid = contract::MAX_GRID_WORKGROUPS[index];
        let bound = if index == 0 { 16_u32 } else { 48_u32 };
        let actual = function
            .attrs
            .iter()
            .find(|a| a.path().is_ident("kernel"))
            .unwrap();
        let actual = tokens(actual).replace(' ', "");
        assert_eq!(
            actual,
            format!(
                "#[kernel(typed,launch(required=[64,1,1],max=[64,1,1],max_grid=[{grid},1,1]),control_flow(loop_bounds({bound})))]"
            )
        );
    }
    assert_eq!(contract::EXPLICIT_ARGUMENT_BYTES, 3 * 16 + 5 * 4);
    assert_eq!(contract::TARGET, "gfx950:xnack-");
    assert_eq!(contract::WORKGROUP, [64, 1, 1]);
}

#[test]
fn descriptors_admit_only_single_row_draft_tp1_with_native_weights() {
    let shapes = [
        (2048, 1024, 1),
        (1024, 1024, 2),
        (1024, 1024, 3),
        (3072, 1024, 4),
        (3072, 1024, 5),
        (1024, 2048, 1),
        (1024, 3072, 2),
        (151936, 1024, 6),
    ];
    for (index, role) in ROLES.into_iter().enumerate() {
        let descriptor = role.descriptor(1, 1).unwrap();
        let (n, k, tag) = shapes[index];
        assert_eq!(descriptor.scalars(), [1, n, k, 1, tag]);
        assert_eq!(descriptor.grid(), [n, 1, 1]);
        assert_eq!(descriptor.kernel(), ROOTS_V16[usize::from(index >= 5)]);
        assert_eq!(
            descriptor.output(),
            if index < 5 {
                OutputCarrier::Bf16
            } else {
                OutputCarrier::F32
            }
        );
        assert_eq!(
            descriptor.output().element_bytes(),
            if index < 5 { 2 } else { 4 }
        );
        for rows in [0, 2, 4, 5, 8, 16, 32, 33, 128, u32::MAX] {
            assert!(role.descriptor(rows, 1).is_none());
        }
        for world in [0, 2, 8, u32::MAX] {
            assert!(role.descriptor(1, world).is_none());
        }
        let (n, k) = (u64::from(n), u64::from(k));
        assert!(descriptor.accepts_lengths(k, n * k, n));
        assert!(descriptor.accepts_lengths(32 * k, n * k, 32 * n));
        for (a, weights, output) in [
            (k - 1, n * k, n),
            (32 * k + 1, n * k, n),
            (k, n * k - 1, n),
            (k, n * k + 1, n),
            (k, n * k, n - 1),
            (k, n * k, 32 * n + 1),
            (u64::MAX, u64::MAX, u64::MAX),
        ] {
            assert!(!descriptor.accepts_lengths(a, weights, output));
        }
    }
}

#[test]
fn descriptors_have_no_public_fields_or_external_constructors() {
    let source = syn::parse_file(include_str!("../src/contract.rs")).unwrap();
    let descriptor = source
        .items
        .iter()
        .find_map(|item| match item {
            Item::Struct(item) if item.ident == "ProjectionDescriptor" => Some(item),
            _ => None,
        })
        .unwrap();
    assert!(
        descriptor
            .fields
            .iter()
            .all(|field| matches!(field.vis, syn::Visibility::Inherited))
    );
    let methods: Vec<_> = source
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(item) if tokens(&item.self_ty) == "ProjectionDescriptor" => Some(item),
            _ => None,
        })
        .flat_map(|item| &item.items)
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) if matches!(method.vis, syn::Visibility::Public(_)) => {
                Some(method.sig.ident.to_string())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        methods,
        ["kernel", "output", "grid", "scalars", "accepts_lengths"]
    );
}

#[test]
fn one_wave_leader_owns_each_active_output_and_no_capacity_tail() {
    for n in [1024_usize, 2048, 3072, 151936] {
        let mut owners = vec![0_u8; 32 * n];
        for raw in 0..n * 64 {
            let column = raw / 64;
            let lane = raw % 64;
            if lane == 0 {
                owners[column] += 1;
            }
        }
        assert!(owners[..n].iter().all(|&count| count == 1));
        assert!(owners[n..].iter().all(|&count| count == 0));
    }
}

#[test]
fn mutations_cannot_relax_shape_finiteness_convergence_or_output_carrier() {
    verify(SOURCE).unwrap();
    for (from, to) in [
        ("rows != 1", "rows > 32"),
        ("world_size != 1", "world_size != 2"),
        ("k != 1024", "k != 4096"),
        ("projection == 6", "projection == 5"),
        ("output.len() > 32 * n", "output.len() > 33 * n"),
        (
            "thread::grid_dim_x() as usize != n",
            "thread::grid_dim_x() as usize != n / 16",
        ),
        ("thread::grid_dim_y() != 1", "thread::grid_dim_y() != 2"),
        (
            "from_shared_slice(a, 0, 1, k, k)",
            "from_shared_slice(a, 0, 32, k, k)",
        ),
        ("load_or(0, inner, 0x7fc0)", "load_or(0, inner, 0)"),
        ("partial += product", "partial = product"),
        ("while step < 16", "while step < 15"),
        ("if inner < k", "if inner <= k"),
        (
            "finite &= product.is_finite() & partial.is_finite()",
            "finite = true",
        ),
        (
            "step += 1;",
            "if !finite { fe2o3_device::trap(); } step += 1;",
        ),
        ("reduce_max_f32::<64>(invalid)", "reduce_max_f32::<64>(0.0)"),
        ("if lane == 0", "if lane == 1"),
        ("&stripe, 0, n, 1, 1, sum", "&stripe, 0, n, 1, 1, 0.0"),
    ] {
        assert!(SOURCE.contains(from), "mutation anchor: {from}");
        assert!(
            verify(&SOURCE.replacen(from, to, 1)).is_err(),
            "accepted mutation: {from}"
        );
    }
}
