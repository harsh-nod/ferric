#[cfg(feature = "tp-batch-engineering")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;

    let roster =
        ferric_qwen3_tp_c1_split8_attention_kernels_device_v21::compiler_expectation_roster_v21();
    let names = roster
        .iter()
        .map(|entry| (entry.logical_name(), entry.export_name()))
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 2);
    assert!(
        names
            .iter()
            .all(|(logical, export)| !logical.is_empty() && !export.is_empty())
    );
    assert_ne!(names[0].0, names[1].0);
    assert_ne!(names[0].1, names[1].1);
    assert_eq!(
        names.iter().map(|(_, export)| *export).collect::<Vec<_>>(),
        ferric_qwen3_tp_c1_split8_attention_kernels_device_v21::ROOTS_V21,
    );

    // Only escaped string literals cross from the c4c build unit to the 5a adapter.
    let mut generated = String::from("const V21_COMPILER_NAMES: [(&str, &str); 2] = [\n");
    for (logical, export) in names {
        writeln!(generated, "    ({logical:?}, {export:?}),")?;
    }
    generated.push_str("];\n");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    std::fs::write(output.join("v21_compiler_names.rs"), generated)?;
    let roster = ferric_qwen3_tp_gemv_prefetch_kernels_device_v20::compiler_expectation_roster_v20();
    let names = roster.iter().map(|entry| (entry.logical_name(), entry.export_name())).collect::<Vec<_>>();
    assert_eq!(names.len(), 2);
    assert!(names.iter().all(|(logical, export)| !logical.is_empty() && !export.is_empty()));
    assert_ne!(names[0].0, names[1].0);
    assert_ne!(names[0].1, names[1].1);
    assert_eq!(names.iter().map(|(_, export)| *export).collect::<Vec<_>>(),
        ferric_qwen3_tp_gemv_prefetch_kernels_device_v20::ROOTS_V20);
    let mut generated = String::from("const V20_COMPILER_NAMES: [(&str, &str); 2] = [\n");
    for (logical, export) in names {
        writeln!(generated, "    ({logical:?}, {export:?}),")?;
    }
    generated.push_str("];\n");
    std::fs::write(output.join("v20_compiler_names.rs"), generated)?;
    let names = ferric_qwen3_tp_prefill_kv_roster_bridge_v27::COMPILER_NAMES;
    assert_eq!(names.len(), 1);
    assert!(
        names
            .iter()
            .all(|(logical, export)| !logical.is_empty() && !export.is_empty())
    );
    // The bridge's separate build script owns d10; this unit keeps only c4c.
    // Only escaped marker-derived strings reach the 5a adapter.
    let mut generated = String::from("const V27_COMPILER_NAMES: [(&str, &str); 1] = [\n");
    for (logical, export) in names {
        writeln!(generated, "    ({logical:?}, {export:?}),")?;
    }
    generated.push_str("];\n");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    std::fs::write(output.join("v27_compiler_names.rs"), generated)?;
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

#[cfg(not(feature = "tp-batch-engineering"))]
fn main() {}
