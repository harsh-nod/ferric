use ferric_qwen3_tp_parallel_kv_kernels_device_v16::{ROOTS_V16, compiler_expectation_roster_v16};
use quote::ToTokens;
use syn::{Block, Expr, FnArg, Item, ItemFn, Pat, Stmt};

const SOURCE: &str = include_str!("../src/append.rs");
const BASELINE: &str = include_str!("../../qwen3-tp-batch-kernels-v2/src/rope_kv.rs");
const PROFILE: &str = r"{
    if rows != 1 || world_size != 1 || max_pages_per_sequence != 4 || physical_pages != 4
        || thread::grid_dim_x() != 64 || thread::grid_dim_y() != 1 || thread::grid_dim_z() != 1
    { fe2o3_device::trap(); }
}";
const OWNERSHIP: &str = r"{
    let output_slot = thread::block_idx_x() as usize;
    let lane = thread::thread_idx_x() as usize;
    if lane < 64 {} else { fe2o3_device::trap(); }
    let Some(output_row) = thread::index_1d().checked_row_striped_2d::<64, 16>() else {
        fe2o3_device::trap();
    };
    let physical_slots = physical_pages * 16;
    let chunks = columns / 64;
}";
const COPY: &str = r"{
    if slot == output_slot {
        let mut chunk = 0_usize;
        while chunk < 16 {
            if chunk < chunks {
                let component = lane + chunk * 64;
                let source_index = row * columns + component;
                let key_value = memory::volatile_load(key, source_index);
                let value_value = memory::volatile_load(value, source_index);
                if !key_cache.write_row_striped_2d(
                    &output_row, chunk, physical_slots, columns, columns, key_value,
                ) || !value_cache.write_row_striped_2d(
                    &output_row, chunk, physical_slots, columns, columns, value_value,
                ) { fe2o3_device::trap(); }
            }
            chunk += 1;
        }
    }
    row += 1;
}";

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn function(source: &str, name: &str) -> Result<ItemFn, &'static str> {
    syn::parse_file(source)
        .map_err(|_| "parse")?
        .items
        .into_iter()
        .find_map(|item| match item {
            Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .ok_or("missing function")
}

fn verify(source: &str) -> Result<(), &'static str> {
    let root = function(source, ROOTS_V16[0])?;
    let baseline = function(BASELINE, "ferric_qwen3_tp_batch_paged_kv_append_v2")?;
    let parsed = syn::parse_file(source).map_err(|_| "parse")?;
    if parsed
        .items
        .iter()
        .filter(|item| matches!(item, Item::Fn(_)))
        .count()
        != 1
        || root.sig.unsafety.is_some()
        || root.sig.inputs.len() != 10
    {
        return Err("closed safe root");
    }
    for (index, (actual, old)) in root.sig.inputs.iter().zip(&baseline.sig.inputs).enumerate() {
        let (FnArg::Typed(actual), FnArg::Typed(old)) = (actual, old) else {
            return Err("receiver");
        };
        let (Pat::Ident(a), Pat::Ident(b)) = (actual.pat.as_ref(), old.pat.as_ref()) else {
            return Err("argument name");
        };
        if a.ident != b.ident {
            return Err("argument name");
        }
        let expected = if index == 4 || index == 5 {
            let ty: syn::Type =
                syn::parse_quote!(WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 16>>);
            tokens(&ty)
        } else {
            tokens(&old.ty)
        };
        if tokens(&actual.ty) != expected {
            return Err("typed argument ABI");
        }
    }
    let expected: syn::Attribute = syn::parse_quote!(#[kernel(typed,
        launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]),
        control_flow(loop_bounds(16, 16, 16, 16)))]);
    let attribute = root
        .attrs
        .iter()
        .find(|a| a.path().is_ident("kernel"))
        .ok_or("attribute")?;
    if tokens(attribute) != tokens(&expected) {
        return Err("launch or loop bound");
    }
    let mut expected = baseline.block.as_ref().clone();
    for statement in &mut expected.stmts {
        if let Stmt::Expr(Expr::If(branch), _) = statement {
            replace_launch_extent(&mut branch.cond);
        }
    }
    let leader = expected
        .stmts
        .iter()
        .position(|s| tokens(s).contains("grid_leader"))
        .ok_or("old leader")?;
    expected.stmts.remove(leader);
    let ownership: Block = syn::parse_str(OWNERSHIP).map_err(|_| "ownership model")?;
    expected
        .stmts
        .splice(leader + 1..leader + 1, ownership.stmts);
    let copy = expected
        .stmts
        .iter_mut()
        .rev()
        .find_map(|s| match s {
            Stmt::Expr(Expr::While(value), _) => Some(value),
            _ => None,
        })
        .ok_or("copy loop")?;
    // Keep the original dynamic slot computation and its explicit slot bound.
    copy.body.stmts.truncate(2);
    copy.body.stmts.extend(
        syn::parse_str::<Block>(COPY)
            .map_err(|_| "copy model")?
            .stmts,
    );
    expected.stmts.splice(
        0..0,
        syn::parse_str::<Block>(PROFILE)
            .map_err(|_| "narrow profile")?
            .stmts,
    );
    if tokens(root.block.as_ref()) != tokens(&expected) {
        return Err("original guards, all-slot prepass, or exact lane copy changed");
    }
    Ok(())
}

fn replace_launch_extent(expression: &mut Expr) {
    if let Expr::Binary(binary) = expression {
        if tokens(&binary.left) == "thread :: launch_extent_1d ()" {
            assert_eq!(tokens(&binary.right), "64");
            *binary.right = syn::parse_quote!(4096);
        } else {
            replace_launch_extent(&mut binary.left);
            replace_launch_extent(&mut binary.right);
        }
    }
}

#[test]
fn one_root_retains_six_slices_four_scalars_and_112_explicit_bytes() {
    verify(SOURCE).unwrap();
    let roster = compiler_expectation_roster_v16();
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].export_name(), ROOTS_V16[0]);
    assert_ne!(roster[0].generated_host_contract_identity(), [0; 32]);
    let root = function(SOURCE, ROOTS_V16[0]).unwrap();
    let sizes: Vec<_> = root
        .sig
        .inputs
        .iter()
        .map(|arg| {
            let FnArg::Typed(arg) = arg else {
                panic!("receiver")
            };
            if tokens(&arg.ty) == "u32" { 4 } else { 16 }
        })
        .collect();
    assert_eq!(sizes, [16, 16, 16, 16, 16, 16, 4, 4, 4, 4]);
    assert_eq!(sizes.iter().sum::<usize>(), 112);
}

#[test]
fn baseline_guards_and_complete_unique_slot_prepass_are_unchanged() {
    verify(SOURCE).unwrap();
    assert!(!SOURCE.contains("grid_leader"));
    assert!(!SOURCE.contains("write_exclusive"));
    assert_eq!(SOURCE.matches(".write_row_striped_2d(").count(), 2);
    assert_eq!(
        SOURCE
            .matches("memory::volatile_load(key, source_index)")
            .count(),
        1
    );
    assert_eq!(
        SOURCE
            .matches("memory::volatile_load(value, source_index)")
            .count(),
        1
    );
    for forbidden in [
        "unsafe",
        "from_raw_parts",
        "atomic",
        "Bf16",
        "f32",
        "f64",
        "barrier",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "unexpected source operation: {forbidden}"
        );
    }
}

#[test]
fn weakening_any_guard_or_mapping_rejects_the_source_contract() {
    for (old, new) in [
        ("rows > 16", "rows > 32"),
        ("physical_pages > 512", "physical_pages > 1024"),
        ("slot == previous", "slot != previous"),
        (
            "thread::launch_extent_1d() != 4096",
            "thread::launch_extent_1d() != 128",
        ),
        ("slot == output_slot", "slot != output_slot"),
        ("lane + chunk * 64", "lane + chunk * 32"),
        (
            "memory::volatile_load(value, source_index)",
            "memory::volatile_load(key, source_index)",
        ),
        ("value_value,", "key_value,"),
        (
            "checked_row_striped_2d::<64, 16>()",
            "checked_row_striped_2d::<64, 17>()",
        ),
        ("rows != 1", "rows != 2"),
        ("world_size != 1", "world_size != 2"),
        ("max_pages_per_sequence != 4", "max_pages_per_sequence != 8"),
        ("physical_pages != 4", "physical_pages != 8"),
        ("thread::grid_dim_x() != 64", "thread::grid_dim_x() != 1"),
    ] {
        assert!(SOURCE.contains(old));
        assert!(
            verify(&SOURCE.replacen(old, new, 1)).is_err(),
            "accepted mutation {old}"
        );
    }
}

#[test]
fn physical_slot_row_ownership_is_injective_over_the_entire_narrow_cache() {
    let mut all = std::collections::BTreeSet::new();
    for slot in 0..64 {
        for lane in 0..64 {
            let invocation = slot * 64 + lane;
            for chunk in 0..16 {
                let destination = (invocation / 64) * 1024 + invocation % 64 + chunk * 64;
                assert_eq!(destination, slot * 1024 + chunk * 64 + lane);
                assert!(all.insert(destination));
            }
        }
    }
    assert_eq!(
        all.into_iter().collect::<Vec<_>>(),
        (0..65536).collect::<Vec<_>>()
    );
}
