pub fn validate_device_build(
    target_arch: &str,
    encoded_rustflags: &str,
) -> Result<(), &'static str> {
    if target_arch != "amdgpu" {
        return Ok(());
    }
    let flags = encoded_rustflags.split('\u{1f}').collect::<Vec<_>>();
    if codegen_values(&flags, "target-cpu=") != ["gfx950"] {
        return Err("exactly one gfx950 target CPU is required");
    }
    if codegen_values(&flags, "target-feature=") != ["-wavefrontsize32,+wavefrontsize64,-xnack"] {
        return Err("exact Wave64, xnack-disabled features are required");
    }
    Ok(())
}

fn codegen_values<'a>(flags: &[&'a str], key: &str) -> Vec<&'a str> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < flags.len() {
        let argument = flags[index];
        let codegen = if argument == "-C" || argument == "--codegen" {
            index += 1;
            flags.get(index).copied()
        } else {
            argument
                .strip_prefix("-C")
                .or_else(|| argument.strip_prefix("--codegen="))
        };
        if let Some(value) = codegen.and_then(|argument| argument.strip_prefix(key)) {
            values.push(value);
        }
        index += 1;
    }
    values
}
