use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ferric_qwen3_tp_batch32_kernels_device_v5 as device;

    let roster = device::compiler_expectation_roster_v5();
    let names = roster
        .iter()
        .map(|entry| (entry.logical_name(), entry.export_name()))
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 15);
    assert!(
        names
            .iter()
            .all(|(logical, export)| !logical.is_empty() && !export.is_empty())
    );
    let logical = names
        .iter()
        .map(|entry| entry.0)
        .collect::<std::collections::BTreeSet<_>>();
    let exports = names
        .iter()
        .map(|entry| entry.1)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(logical.len(), names.len());
    assert_eq!(exports.len(), names.len());
    let expected = ["qwen3_rmsnorm_v1"]
        .into_iter()
        .chain(device::baseline_contract::NEW_ROOTS)
        .chain(device::contract::PERFORMANCE_ROOTS)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(exports, expected);

    // This unit owns the migrated SDK; the library publishes only marker names.
    let mut generated = String::from(
        "/// Marker-derived V5 names; no image, execution or proof authority.\n\
         pub const COMPILER_NAMES: [(&str, &str); 15] = [\n",
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
