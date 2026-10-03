//! Supplemental AST/source contracts, not semantic-KIR extraction evidence.
//! Numerical acceptance in the companion test comes from executed arithmetic.
use fe2o3_device::finite_join::wave_qkv_attention_output_tiles_v6 as profile;
use quote::ToTokens;
use syn::{
    Expr, FnArg, Item, ItemFn, Lit, Meta, Token, Type, parse::Parser, punctuated::Punctuated,
};

const SOURCE: &str = include_str!("../src/finite_qkv_attention_output_tiles_v6.rs");
const OLD: &str = include_str!("../src/finite_qkv_attention_output_tasks.rs");
const HELPERS: &str = include_str!("../src/prefix_tiles_numerics_v6.rs");
const ENTRY: &str = "ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6";

fn function<'a>(file: &'a syn::File, name: &str) -> &'a ItemFn {
    file.items
        .iter()
        .find_map(|item| match item {
            Item::Fn(f) if f.sig.ident == name => Some(f),
            _ => None,
        })
        .unwrap()
}
fn integer(e: &Expr) -> u32 {
    let Expr::Lit(e) = e else {
        panic!("literal required")
    };
    let Lit::Int(n) = &e.lit else {
        panic!("integer required")
    };
    n.base10_parse().unwrap()
}
fn vector(e: &Expr) -> Vec<u32> {
    let Expr::Array(a) = e else {
        panic!("literal array required")
    };
    a.elems.iter().map(integer).collect()
}

#[test]
fn source_entry_is_distinct_typed_storage_and_exact_64_group_geometry() {
    let file = syn::parse_file(SOURCE).unwrap();
    let entry = function(&file, ENTRY);
    assert!(
        entry.sig.unsafety.is_none() && entry.sig.abi.is_none() && entry.sig.variadic.is_none()
    );
    assert_eq!(entry.sig.inputs.len(), 1);
    let FnArg::Typed(arg) = entry.sig.inputs.first().unwrap() else {
        panic!("typed storage")
    };
    let Type::Path(path) = arg.ty.as_ref() else {
        panic!("nominal storage")
    };
    assert_eq!(
        path.path.segments.last().unwrap().ident,
        "WaveQkvAttentionOutputTileStorageV6"
    );
    let attribute = entry
        .attrs
        .iter()
        .find(|a| a.path().segments.last().unwrap().ident == "kernel")
        .unwrap();
    let args = attribute
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .unwrap();
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0],Meta::Path(p) if p.is_ident("typed")));
    let Meta::List(launch) = &args[1] else {
        panic!("closed launch")
    };
    assert!(launch.path.is_ident("launch"));
    let values = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(launch.tokens.clone())
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        let Meta::NameValue(n) = value else {
            panic!("launch field")
        };
        let name = n.path.get_ident().unwrap().to_string();
        assert!(seen.insert(name.clone()));
        match name.as_str() {
            "required" | "max" | "max_grid" => assert_eq!(vector(&n.value), vec![64, 1, 1]),
            "static_shared_memory_bytes" => assert_eq!(integer(&n.value), 512),
            _ => panic!("unexpected launch field"),
        }
    }
    assert_eq!(seen.len(), 4);
    assert_eq!(profile::WORKGROUPS as usize * profile::WAVE_LANES, 4096);
}

#[test]
fn norm_and_post_ast_bodies_and_epsilon_are_unchanged_from_v5() {
    let old = syn::parse_file(OLD).unwrap();
    let new = syn::parse_file(SOURCE).unwrap();
    for name in ["norm", "postprocess"] {
        // AST-token equality is a source-change tripwire, not a numerical test.
        assert_eq!(
            function(&old, name).block.to_token_stream().to_string(),
            function(&new, name).block.to_token_stream().to_string()
        );
    }
    let host = syn::parse_file(include_str!("prefix_tiles_numerics_v6.rs")).unwrap();
    assert_eq!(
        function(&old, "projection")
            .block
            .to_token_stream()
            .to_string(),
        function(&host, "old_projection")
            .block
            .to_token_stream()
            .to_string()
    );
    let epsilon = |file: &syn::File| {
        file.items
            .iter()
            .find_map(|i| match i {
                Item::Const(c) if c.ident == "EPSILON" => {
                    Some(c.expr.to_token_stream().to_string())
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(epsilon(&old), epsilon(&new));
}

#[test]
fn unchanged_arithmetic_files_are_included_and_only_three_sliced_handlers_are_added() {
    let file = syn::parse_file(SOURCE).unwrap();
    let mut includes = std::collections::BTreeSet::new();
    for item in &file.items {
        if let Item::Macro(m) = item {
            if m.mac.path.is_ident("include") {
                let name: syn::LitStr = syn::parse2(m.mac.tokens.clone()).unwrap();
                assert!(includes.insert(name.value()));
            }
        }
    }
    assert_eq!(
        includes.into_iter().collect::<Vec<_>>(),
        vec![
            "attention_online.rs",
            "head_rope_numerics_v3.rs",
            "output_projection_numerics_v5.rs",
            "prefix_tiles_numerics_v6.rs",
            "wave_numerics_v1.rs"
        ]
    );
    let helper = syn::parse_file(HELPERS).unwrap();
    let names: Vec<_> = helper
        .items
        .iter()
        .map(|i| match i {
            Item::Macro(m) => m.ident.as_ref().unwrap().to_string(),
            _ => panic!("only additive handler macros"),
        })
        .collect();
    assert_eq!(
        names,
        vec![
            "qwen_claimed_projection_tile_v6",
            "qwen_claimed_output_projection_tile_v6",
            "qwen_claimed_attention_head_v6"
        ]
    );
    for (name, expected) in [
        ("projection", "qwen_claimed_projection_tile_v6"),
        ("attention", "qwen_claimed_attention_head_v6"),
        (
            "output_projection",
            "qwen_claimed_output_projection_tile_v6",
        ),
    ] {
        let macros: Vec<_> = function(&file, name)
            .block
            .stmts
            .iter()
            .filter_map(|s| match s {
                syn::Stmt::Macro(m) => Some(m.mac.path.get_ident().unwrap().to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(macros, vec![expected]);
    }
}

#[test]
fn all_five_claim_variants_and_consuming_provider_entry_remain_explicit() {
    let file = syn::parse_file(SOURCE).unwrap();
    let execute = function(&file, "execute_task");
    let syn::Stmt::Expr(Expr::Match(m), _) = execute.block.stmts.last().unwrap() else {
        panic!("typed task match")
    };
    let variants: Vec<_> = m
        .arms
        .iter()
        .map(|a| match &a.pat {
            syn::Pat::TupleStruct(p) => p.path.segments.last().unwrap().ident.to_string(),
            _ => panic!("no wildcard task"),
        })
        .collect();
    assert_eq!(
        variants,
        vec![
            "Norm",
            "Projection",
            "Post",
            "Attention",
            "OutputProjection"
        ]
    );
    let entry = function(&file, "engineering_rmsnorm_qkv_attention_output_tiles_v6");
    let syn::Stmt::Expr(Expr::Call(call), _) = entry.block.stmts.last().unwrap() else {
        panic!("consuming run")
    };
    let Expr::Path(p) = call.func.as_ref() else {
        panic!("closed worker")
    };
    assert_eq!(
        p.path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>(),
        vec!["WaveQkvAttentionOutputTileWorkerV6", "run"]
    );
    assert_eq!(call.args.len(), 3);
    assert!(matches!(call.args.last().unwrap(), Expr::Closure(_)));
}

#[test]
fn stage_counts_and_all_fifteen_root_extents_are_generation_specific() {
    assert_eq!(profile::STAGE_COUNTS, [1, 48, 1, 16, 64]);
    assert_eq!(profile::STAGE_STARTS, [0, 1, 49, 50, 66]);
    assert_eq!(
        (
            profile::TASK_COUNT,
            profile::WAVE_STATE_WORDS,
            profile::MAX_ROUNDS
        ),
        (130, 284, 256)
    );
    let extents = [
        profile::NORM_ELEMENTS * 2,
        profile::NORM_ELEMENTS * 2,
        profile::QKV_ELEMENTS * 2,
        profile::HEAD_WEIGHT_ELEMENTS * 2,
        profile::ROTARY_ELEMENTS * 4,
        profile::CACHE_METADATA_WORDS * 4,
        profile::OUTPUT_WEIGHT_ELEMENTS * 2,
        profile::NORM_ELEMENTS * 2,
        profile::QKV_COLUMNS * 2,
        profile::QUERY_ELEMENTS * 2,
        profile::CACHE_ELEMENTS * 2,
        profile::CACHE_ELEMENTS * 2,
        profile::ATTENTION_ELEMENTS * 2,
        profile::OUTPUT_ELEMENTS * 4,
        profile::WAVE_STATE_WORDS * 4,
    ];
    assert_eq!(
        extents,
        [
            8192, 8192, 25165824, 512, 512, 580, 16777216, 8192, 6144, 4096, 2359296, 2359296,
            4096, 16384, 1136
        ]
    );
    assert_eq!(
        core::mem::size_of::<profile::WaveQkvAttentionOutputTileStorageV6<'_>>(),
        120
    );
    assert!(!profile::terminal_snapshot(&profile::initial_state_words()));
}
