use quote::ToTokens;
use syn::{Item, ItemFn};

const SOURCE: &str = include_str!("../src/projection.rs");
const CONTROL: &str = include_str!("fixtures/control.rs.txt");
const PAIRED: &str = include_str!("fixtures/paired.rs.txt");
const ROOTS: [&str; 2] = [
    "ferric_qwen3_prefill_k16_control_bf16_r1",
    "ferric_qwen3_prefill_k16_paired_bf16_r1",
];

fn functions(source: &str) -> Vec<ItemFn> {
    syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| {
            if let Item::Fn(function) = item {
                Some(function)
            } else {
                None
            }
        })
        .collect()
}

fn matches_fixture(source: &str, fixture: &str, root: &str) -> bool {
    let originals = functions(fixture);
    assert_eq!(originals.len(), 1);
    let mut expected = originals[0].clone();
    expected.sig.ident = syn::parse_str(root).unwrap();
    expected
        .attrs
        .retain(|attribute| !attribute.path().is_ident("cfg"));
    let actual = functions(source);
    let Some(actual) = actual.iter().find(|function| function.sig.ident == root) else {
        return false;
    };
    actual.to_token_stream().to_string() == expected.to_token_stream().to_string()
}

#[test]
fn both_roots_exactly_match_retained_functions_except_name_and_feature_placement() {
    let actual = functions(SOURCE);
    assert_eq!(actual.len(), 2);
    assert!(matches_fixture(SOURCE, CONTROL, ROOTS[0]));
    assert!(matches_fixture(SOURCE, PAIRED, ROOTS[1]));
    for function in actual {
        assert_eq!(
            function
                .attrs
                .iter()
                .filter(|attribute| attribute.path().is_ident("kernel"))
                .count(),
            1
        );
    }
}

#[test]
fn exact_source_contract_rejects_changed_bounds_guards_slots_and_operands() {
    for (from, to) in [
        ("world_size: u32", "world_size: u64"),
        ("rows > 32", "rows > 64"),
        (
            "weights_kn.len() != n * 4096",
            "weights_kn.len() < n * 4096",
        ),
        ("thread::block_dim_x() != 64", "thread::block_dim_x() != 32"),
        ("row_base + 1 < rows", "row_base + 1 <= rows"),
        ("Bf16::from_f32(value_0)", "Bf16::from_f32(value_1)"),
        ("!value_2.is_finite()", "!value_3.is_finite()"),
        ("while pair < 128", "while pair < 127"),
        (
            "multiply_accumulate(next_a_fragment, next_b_fragment, accumulator)",
            "multiply_accumulate(a_fragment, next_b_fragment, accumulator)",
        ),
    ] {
        assert!(SOURCE.contains(from), "missing mutation target: {from}");
        assert!(!matches_fixture(
            &SOURCE.replace(from, to),
            PAIRED,
            ROOTS[1]
        ));
    }
}

#[test]
fn experimental_feature_is_default_off_and_does_not_import_other_kernel_roots() {
    let manifest = include_str!("../Cargo.toml");
    let library = include_str!("../src/lib.rs");
    assert!(manifest.contains("default = []"));
    assert!(manifest.contains("paired-prefill-k16-r1 = [\"gfx950\"]"));
    assert!(library.contains("#[cfg(feature = \"paired-prefill-k16-r1\")]\npub mod projection;"));
    assert!(!SOURCE.contains("mod "));
    assert!(!SOURCE.contains("pub use"));
    assert_eq!(
        ferric_qwen3_tp_prefill_k2_kernels_device_v1::EXPERIMENT_ENABLED,
        cfg!(feature = "paired-prefill-k16-r1")
    );
}

#[test]
fn paired_k16_visits_match_the_complete_ascending_control_sequence() {
    let control = (0..256).map(|step| step * 16).collect::<Vec<_>>();
    let paired = (0..128)
        .flat_map(|pair| [pair * 32, pair * 32 + 16])
        .collect::<Vec<_>>();
    assert_eq!(control, paired);
    assert_eq!(paired.first(), Some(&0));
    assert_eq!(paired.last(), Some(&4080));
    assert_eq!(paired.len(), 256);
}

#[test]
fn tiled_output_mapping_writes_each_active_element_once() {
    for rows in [1usize, 15, 16, 17, 31, 32] {
        let columns = 12_288;
        let groups = rows.div_ceil(16) * (columns / 16);
        let mut visits = vec![0u8; 32 * columns];
        for group in 0..groups {
            for lane in 0..64 {
                for component in 0..4 {
                    let row = 16 * (group / 768) + 4 * (lane / 16) + component;
                    let column = 16 * (group % 768) + lane % 16;
                    if row < rows {
                        visits[row * columns + column] += 1;
                    }
                }
            }
        }
        assert!(visits[..rows * columns].iter().all(|count| *count == 1));
        assert!(visits[rows * columns..].iter().all(|count| *count == 0));
    }
}

#[test]
fn host_order_model_preserves_f32_updates_and_bf16_rounding() {
    // This is an ordering model, not an MFMA emulator or native parity result.
    for offset in 0..19 {
        let contribution = |step: usize| ((step * 7 + offset) % 31) as f32 / 32.0 - 0.5;
        let mut control = 0.0_f32;
        for step in 0..256 {
            control += contribution(step);
        }
        let mut paired = 0.0_f32;
        for pair in 0..128 {
            paired += contribution(pair * 2);
            paired += contribution(pair * 2 + 1);
        }
        assert_eq!(control.to_bits(), paired.to_bits());
        assert_eq!(
            fe2o3_device::Bf16::from_f32(control).to_bits(),
            fe2o3_device::Bf16::from_f32(paired).to_bits()
        );
    }
}

#[test]
fn checked_matrix_constructors_reject_short_stride_extent_and_overflow() {
    use fe2o3_device::{Bf16MfmaAMatrix, Bf16MfmaBMatrix};
    let bits = [0_u16; 32];
    assert!(Bf16MfmaAMatrix::row_major(&bits, 0, 2, 16, 16).is_ok());
    assert!(Bf16MfmaBMatrix::row_major(&bits, 0, 16, 2, 2).is_ok());
    assert!(Bf16MfmaAMatrix::row_major(&bits, 0, 2, 16, 15).is_err());
    assert!(Bf16MfmaBMatrix::row_major(&bits, 0, 16, 2, 1).is_err());
    assert!(Bf16MfmaAMatrix::row_major(&bits[..31], 0, 2, 16, 16).is_err());
    assert!(Bf16MfmaBMatrix::row_major(&bits[..31], 0, 16, 2, 2).is_err());
    assert!(Bf16MfmaAMatrix::row_major(&bits, 1, 2, 1, usize::MAX).is_err());
    assert!(Bf16MfmaBMatrix::row_major(&bits, 1, 2, 1, usize::MAX).is_err());
}

#[cfg(feature = "paired-prefill-k16-r1")]
#[test]
fn enabled_host_roster_has_two_distinct_kernel_bindings() {
    let entries = ferric_qwen3_tp_prefill_k2_kernels_device_v1::compiler_expectation_roster();
    assert_eq!(entries.len(), 2);
    assert_ne!(
        entries[0].kernel_binding_id(),
        entries[1].kernel_binding_id()
    );
}
