// Standalone host model screen; this does not emulate MFMA or qualify device code.
const PROJECTION: &str = include_str!("../src/projection_k2.rs");
const BASELINE: &str = include_str!("../src/projection.rs");
const LIBRARY: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn candidate_requires_explicit_feature_and_baseline_stays_unpaired() {
    assert!(LIBRARY.contains(concat!(
        "#[cfg(not(feature = \"prefill-mfma-k2\"))]\n",
        "pub mod projection;\n",
        "#[cfg(feature = \"prefill-mfma-k2\")]\n",
        "#[path = \"projection_k2.rs\"]\n",
        "pub mod projection;"
    )));
    assert_eq!(LIBRARY.matches("pub mod projection;").count(), 2);
    assert!(MANIFEST.contains("default = [\"gfx950\", \"mfma\"]"));
    assert!(MANIFEST.contains("prefill-mfma-k2 = [\"mfma\"]"));
    let mfma_start = "/// One Wave64 owns a 16x16 tile, with zero-filled inactive activation rows.";
    let (baseline_wave, baseline_mfma) = BASELINE.split_once(mfma_start).unwrap();
    assert!(!baseline_mfma.contains("while pair <"));
    assert_eq!(baseline_mfma.matches("let mut step = 0_usize;").count(), 7);
    assert!(baseline_mfma.contains("control_flow(loop_bounds(256))"));
    assert!(baseline_mfma.contains("control_flow(loop_bounds(32, 96, 128, 256, 384, 768))"));
    assert_eq!(
        baseline_wave,
        PROJECTION.split_once(mfma_start).unwrap().0
    );
}

fn paired_loop_source_matches(source: &str) -> bool {
    let normalized = source.lines().map(str::trim).collect::<Vec<_>>().join("\n");
    let body = [
        "let reduction_base = pair * 32;",
        "let next_reduction_base = reduction_base + 16;",
        "let a_fragment = left.load_m16k16(&lane, tile_row * 16, reduction_base);",
        "let b_fragment = right.load_k16n16(&lane, reduction_base, tile_column * 16);",
        "let next_a_fragment = left.load_m16k16(&lane, tile_row * 16, next_reduction_base);",
        "let next_b_fragment = right.load_k16n16(&lane, next_reduction_base, tile_column * 16);",
        "accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);",
        "accumulator = matrix.multiply_accumulate(next_a_fragment, next_b_fragment, accumulator);",
        "pair += 1;",
        "}",
    ]
    .join("\n");
    [16, 48, 64, 128, 192, 384].into_iter().all(|bound| {
        let expected = format!("let mut pair = 0_usize;\nwhile pair < {bound} {{\n{body}");
        normalized.matches(&expected).count() == if bound == 128 { 2 } else { 1 }
    }) && normalized.matches("while pair <").count() == 7
        && normalized.matches("control_flow(loop_bounds(128))").count() == 1
        && normalized
            .matches("control_flow(loop_bounds(16, 48, 64, 128, 192, 384))")
            .count()
            == 1
}

#[test]
fn source_retains_seven_exact_paired_steps_and_declared_trip_counts() {
    assert!(paired_loop_source_matches(PROJECTION));
}

#[test]
fn source_screen_rejects_reordered_updates_missing_loads_and_wrong_bounds() {
    for (before, after) in [
        (
            "multiply_accumulate(a_fragment, b_fragment, accumulator)",
            "multiply_accumulate(next_a_fragment, next_b_fragment, accumulator)",
        ),
        (
            "let next_a_fragment = left.load_m16k16",
            "let next_a_fragment = left.load_other",
        ),
        ("while pair < 128", "while pair < 127"),
        (
            "loop_bounds(16, 48, 64, 128, 192, 384)",
            "loop_bounds(16, 48, 64, 128, 192, 383)",
        ),
    ] {
        assert!(!paired_loop_source_matches(
            &PROJECTION.replacen(before, after, 1)
        ));
    }
}

fn paired_bases(k: usize) -> Vec<usize> {
    let mut bases = Vec::new();
    for pair in 0..k.div_ceil(32) {
        bases.push(pair * 32);
        if pair * 32 + 16 < k {
            bases.push(pair * 32 + 16);
        }
    }
    bases
}

#[test]
fn supported_reduction_widths_have_no_extra_loads_or_missing_k16_tiles() {
    for k in [512, 1536, 2048, 4096, 6144, 12288] {
        let actual: Vec<_> = (0..k / 32)
            .flat_map(|pair| [pair * 32, pair * 32 + 16])
            .collect();
        assert_eq!(actual, (0..k).step_by(16).collect::<Vec<_>>());
        assert_eq!(actual.len(), k / 16);
        assert_eq!(actual.last(), Some(&(k - 16)));
    }
}

#[test]
fn k16_k32_k48_order_model_keeps_one_accumulator_and_odd_tile_tail() {
    // K16/K48 are model probes, not newly admitted production shapes.
    let contributions = [33_554_432_f32, -33_554_432.0, 1.0];
    for k in [16, 32, 48] {
        assert_eq!(paired_bases(k), (0..k).step_by(16).collect::<Vec<_>>());
        let mut expected = 0_f32;
        for base in (0..k).step_by(16) {
            expected += contributions[base / 16];
        }
        let mut paired = 0_f32;
        for base in paired_bases(k) {
            paired += contributions[base / 16];
        }
        assert_eq!(paired.to_bits(), expected.to_bits());
    }
    assert_eq!((contributions[0] + contributions[1]) + contributions[2], 1.0);
    assert_eq!(contributions[0] + (contributions[1] + contributions[2]), 0.0);
}

fn checked_value(
    bits: &[u16],
    shape: [usize; 3],
    row: Option<usize>,
    column: Option<usize>,
) -> u16 {
    let [rows, columns, stride] = shape;
    let (Some(row), Some(column)) = (row, column) else {
        return 0;
    };
    if row >= rows || column >= columns {
        return 0;
    }
    row.checked_mul(stride)
        .and_then(|base| base.checked_add(column))
        .and_then(|index| bits.get(index))
        .copied()
        .unwrap_or(0)
}

#[test]
fn checked_fragment_model_preserves_row_column_and_reduction_tails() {
    // Model the documented four-value distribution; host code cannot forge a WaveLane.
    for rows in [1_usize, 15, 16, 17, 31, 32] {
        for k in [1_usize, 15, 16, 17, 31, 32, 33, 48] {
            let a: Vec<_> = (0..rows * k).map(|value| value as u16 + 1).collect();
            let n = 19_usize;
            let b: Vec<_> = (0..k * n).map(|value| value as u16 + 1).collect();
            for tile_row in 0..rows.div_ceil(16) {
                for tile_column in 0..n.div_ceil(16) {
                    for base in paired_bases(k) {
                        for lane in 0..64_usize {
                            let row = tile_row * 16 + (lane & 15);
                            let column = tile_column * 16 + (lane & 15);
                            for component in 0..4 {
                                let reduction = base + (lane / 16) * 4 + component;
                                let left =
                                    checked_value(&a, [rows, k, k], Some(row), Some(reduction));
                                let right =
                                    checked_value(&b, [k, n, n], Some(reduction), Some(column));
                                assert_eq!(
                                    left,
                                    if row < rows && reduction < k {
                                        a[row * k + reduction]
                                    } else {
                                        0
                                    }
                                );
                                assert_eq!(
                                    right,
                                    if reduction < k && column < n {
                                        b[reduction * n + column]
                                    } else {
                                        0
                                    }
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn checked_fragment_model_zero_fills_coordinate_and_address_overflow() {
    let bits = [7_u16; 16];
    assert_eq!(
        checked_value(&bits, [1, 16, 16], usize::MAX.checked_add(1), Some(0)),
        0
    );
    assert_eq!(
        checked_value(&bits, [1, 16, 16], Some(0), usize::MAX.checked_add(1)),
        0
    );
    assert_eq!(
        checked_value(&bits, [3, 16, usize::MAX], Some(2), Some(0)),
        0
    );
    assert_eq!(
        checked_value(&bits, [3, 16, usize::MAX], Some(1), Some(1)),
        0
    );
}
