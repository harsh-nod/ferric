#![forbid(unsafe_code)]
use fe2o3_compiler_ffi::{CompilerModuleHandoffV2, CompilerModuleKindV1, MAX_COMPILER_MODULE_HANDOFF_BYTES_V2};
use std::{env, fs::{File, OpenOptions}, io::{Read, Write}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 3 { return Err("usage: unpack-handoff INPUT OUTPUT".into()); }
    let mut bytes = Vec::new();
    File::open(&args[1])?.take(MAX_COMPILER_MODULE_HANDOFF_BYTES_V2 as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_COMPILER_MODULE_HANDOFF_BYTES_V2 { return Err("oversized handoff".into()); }
    let handoff = CompilerModuleHandoffV2::decode(&bytes)?;
    if handoff.kind() != CompilerModuleKindV1::LlvmTextIr { return Err("expected LLVM text".into()); }
    println!("target={:?} code_object={:?} module={:?} compiler_origin_authority={}",
        handoff.target(), handoff.code_object_version(), handoff.module_identity(),
        handoff.authenticates_compiler_origin());
    OpenOptions::new().write(true).create_new(true).open(&args[2])?.write_all(handoff.module_bytes())?;
    Ok(())
}
