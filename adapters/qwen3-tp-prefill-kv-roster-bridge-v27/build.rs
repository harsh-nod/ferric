use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let roster =
        ferric_qwen3_tp_prefill_kv_copy_kernels_device_v27::compiler_expectation_roster_v27();
    let names = roster
        .iter()
        .map(|entry| (entry.logical_name(), entry.export_name()))
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 1);
    assert!(
        names
            .iter()
            .all(|(logical, export)| !logical.is_empty() && !export.is_empty())
    );
    assert_eq!(
        names.iter().map(|(_, export)| *export).collect::<Vec<_>>(),
        ferric_qwen3_tp_prefill_kv_copy_kernels_device_v27::ROOTS_V27,
    );
    // This unit links d10. Its library publishes strings, never SDK types.
    let mut generated = String::from(
        "/// Marker-derived V27 names; no image, execution or proof authority.\n\
         pub const COMPILER_NAMES: [(&str, &str); 1] = [\n",
    );
    for (logical, export) in names {
        writeln!(generated, "    ({logical:?}, {export:?}),")?;
    }
    generated.push_str("];\n");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    std::fs::write(output.join("compiler_names.rs"), generated)?;
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
