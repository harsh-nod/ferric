use ferric_qwen3_draft_batch32_kernels_device_v10::{
    compiler_expectation_roster_v10,
    contract::{self, OutputCarrier, ProjectionMath, ProjectionRole, ROOTS_V10},
};
use syn::{FnArg, Item, Type};

const SOURCES: [&str; 10] = [
    include_str!("../src/rmsnorm.rs"),
    include_str!("../src/embedding.rs"),
    include_str!("../src/projection.rs"),
    include_str!("../src/mfma.rs"),
    include_str!("../src/activation.rs"),
    include_str!("../src/rope_kv.rs"),
    include_str!("../src/attention.rs"),
    include_str!("../src/collective.rs"),
    include_str!("../src/head.rs"),
    include_str!("../src/logits.rs"),
];
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

#[test]
fn closed_roster_exact_typed_abi_and_launch_bounds() {
    let roster = compiler_expectation_roster_v10();
    assert_eq!(roster.len(), 14);
    assert!(
        roster
            .windows(2)
            .all(|w| w[0].kernel_binding_id() < w[1].kernel_binding_id())
    );
    let mut exports: Vec<_> = roster.iter().map(|entry| entry.export_name()).collect();
    exports.sort_unstable();
    let mut expected = ROOTS_V10;
    expected.sort_unstable();
    assert_eq!(exports, expected);
    let functions: Vec<_> = SOURCES
        .iter()
        .flat_map(|source| {
            syn::parse_file(source)
                .unwrap()
                .items
                .into_iter()
                .filter_map(|item| match item {
                    Item::Fn(f) if f.attrs.iter().any(|a| a.path().is_ident("kernel")) => Some(f),
                    _ => None,
                })
        })
        .collect();
    assert_eq!(functions.len(), ROOTS_V10.len());
    for (index, name) in ROOTS_V10.iter().enumerate() {
        let function = functions.iter().find(|f| f.sig.ident == name).unwrap();
        assert!(function.sig.unsafety.is_none());
        let bytes: u32 = function
            .sig
            .inputs
            .iter()
            .map(|arg| {
                let FnArg::Typed(arg) = arg else {
                    panic!("receiver")
                };
                match arg.ty.as_ref() {
                    Type::Reference(r) => {
                        assert!(r.mutability.is_none());
                        assert!(matches!(r.elem.as_ref(), Type::Slice(_)));
                        16
                    }
                    Type::Path(p)
                        if p.path.segments.last().unwrap().ident == "WriteOnlyDisjointSlice" =>
                    {
                        16
                    }
                    Type::Path(p) if p.path.is_ident("u32") || p.path.is_ident("f32") => 4,
                    _ => panic!("unexpected ABI type"),
                }
            })
            .sum();
        assert_eq!(bytes, contract::EXPLICIT_ARGUMENT_BYTES[index], "{name}");
        let kernel = function
            .attrs
            .iter()
            .find(|a| a.path().is_ident("kernel"))
            .unwrap();
        let syn::Meta::List(meta) = &kernel.meta else {
            panic!("kernel metadata")
        };
        let tokens = meta.tokens.to_string();
        assert!(tokens.contains("required = [64 , 1 , 1]"), "{tokens}");
        assert!(tokens.contains("max = [64 , 1 , 1]"), "{tokens}");
        assert!(
            tokens.contains(&format!(
                "max_grid = [{} , 1 , 1]",
                contract::MAX_GRID_WORKGROUPS[index]
            )),
            "{name}: {tokens}"
        );
    }
}

#[test]
fn all_projection_roles_use_draft_shapes_and_distinct_fp32_head() {
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
    for (index, role) in ROLES.iter().copied().enumerate() {
        let shape = role.shape();
        assert_eq!((shape.n, shape.k, shape.tag), shapes[index]);
        assert_eq!(
            shape.output,
            if index < 5 {
                OutputCarrier::Bf16
            } else {
                OutputCarrier::F32
            }
        );
        for rows in 1..=32 {
            assert!(role.accepts(rows, 1, shape.n, shape.k, shape.tag));
            assert_eq!(
                role.grid(rows, 1),
                Some([rows.div_ceil(16) * (shape.n / 16), 1, 1])
            );
            for mode in [ProjectionMath::Scalar, ProjectionMath::Mfma] {
                let name = role.kernel(mode);
                assert!(ROOTS_V10.contains(&name));
                assert!(name.starts_with("ferric_qwen3_draft_batch32_"));
            }
            for world in [0, 2, 8, u32::MAX] {
                assert!(!role.accepts(rows, world, shape.n, shape.k, shape.tag));
                assert_eq!(role.grid(rows, world), None);
            }
            for (n, k, tag) in [
                (shape.n + 1, shape.k, shape.tag),
                (shape.n, shape.k + 1, shape.tag),
                (shape.n, shape.k, 0),
            ] {
                assert!(!role.accepts(rows, 1, n, k, tag));
            }
        }
        for rows in [0, 33, u32::MAX] {
            assert!(!role.accepts(rows, 1, shape.n, shape.k, shape.tag));
            assert_eq!(role.grid(rows, 1), None);
        }
    }
    assert_eq!(
        ProjectionRole::Head.kernel(ProjectionMath::Scalar),
        ROOTS_V10[11]
    );
    assert_eq!(
        ProjectionRole::Head.kernel(ProjectionMath::Mfma),
        ROOTS_V10[12]
    );
}

#[test]
fn every_active_projection_element_has_one_owner_and_tail_rows_have_none() {
    for n in [1024_usize, 2048, 3072, 151936] {
        for rows in [1_usize, 3, 5, 16, 17, 31, 32] {
            let mut owners = vec![0_u8; 32 * n];
            for group in 0..rows.div_ceil(16) * (n / 16) {
                for lane in 0..64 {
                    let column = group % (n / 16) * 16 + lane % 16;
                    for component in 0..4 {
                        let row = group / (n / 16) * 16 + lane / 16 * 4 + component;
                        if row < rows {
                            owners[row * n + column] += 1;
                        }
                    }
                }
            }
            assert!(owners[..rows * n].iter().all(|&v| v == 1));
            assert!(owners[rows * n..].iter().all(|&v| v == 0));
        }
    }
}

#[test]
fn draft_norm_gqa_and_payload_envelopes_are_bounded() {
    assert!(contract::norm_shape_is_supported(32, 1024, 0));
    assert!(contract::norm_shape_is_supported(512, 128, 0));
    for (rows, width, mode) in [
        (0, 128, 0),
        (33, 1024, 0),
        (513, 128, 0),
        (1, 4096, 0),
        (1, 1024, 1),
    ] {
        assert!(!contract::norm_shape_is_supported(rows, width, mode));
    }
    for query in 0..16 {
        assert_eq!(contract::query_to_kv_head(query), Some(query / 2));
    }
    assert_eq!(contract::query_to_kv_head(16), None);
    assert_eq!(contract::query_to_kv_head(u32::MAX), None);
    for rows in 1..=32 {
        assert!(contract::paged_limits_are_supported(rows, 1, 512, 512));
    }
    for (rows, world, pages, stride) in [
        (0, 1, 512, 512),
        (33, 1, 512, 512),
        (1, 2, 512, 512),
        (1, 1, 0, 512),
        (1, 1, 513, 512),
        (1, 1, 512, 0),
        (1, 1, 512, 513),
    ] {
        assert!(!contract::paged_limits_are_supported(
            rows, world, pages, stride
        ));
    }
    assert_eq!(contract::KV_PAYLOAD_BYTES, 28_u64 * 2 * 512 * 16 * 1024 * 2);
    assert_eq!(contract::FP32_LOGITS_BYTES, 32_u64 * 151936 * 4);
    assert_eq!(
        (512_usize * 16 - 1) * 1024 + 7 * 128 + 127,
        512 * 16 * 1024 - 1
    );
}
