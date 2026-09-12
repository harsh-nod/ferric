//! Closed, opt-in target-only comparison against two unchanged reference files.

use super::paired_paged_canary_contract as frozen;
use super::tp_worker::RuntimeOptions;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const CONTEXT: u32 = 256;
pub const PAGES: u32 = 16;
pub const PROMPT: usize = 128;
pub const CHUNK: usize = 16;
pub const PREFIX_REFERENCE_SHA256: &str =
    "e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgmaxMode {
    Serial,
    WaveV11,
}

impl ArgmaxMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Serial => "serial",
            Self::WaveV11 => "wave-v11",
        }
    }
}

pub struct Options {
    pub source: PathBuf,
    pub target_artifact: PathBuf,
    pub target_head_artifact: PathBuf,
    pub argmax_artifact: PathBuf,
    pub worker: PathBuf,
    pub worker_sha256: String,
    pub device: u64,
    pub reference: PathBuf,
    pub prefix_reference: PathBuf,
    pub host_timing_output: PathBuf,
    pub mode: ArgmaxMode,
    pub outputs: usize,
    pub runtime: RuntimeOptions,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut values = BTreeMap::new();
        while let Some(flag) = arguments.next() {
            if values.contains_key(&flag) {
                return Err(format!("duplicate option {flag}"));
            }
            let value = match flag.as_str() {
                "--allow-unauthenticated-machine-code"
                | "--runtime-cache-admission"
                | "--runtime-operational"
                | "--runtime-rollover" => String::new(),
                "--source"
                | "--target-artifact"
                | "--target-head-artifact"
                | "--argmax-artifact"
                | "--worker"
                | "--worker-sha256"
                | "--device-unique-id"
                | "--reference"
                | "--prefix-reference"
                | "--host-timing-output"
                | "--argmax-mode"
                | "--max-new-tokens" => arguments
                    .next()
                    .ok_or_else(|| format!("missing value for {flag}"))?,
                _ => return Err(format!("unsupported option {flag}")),
            };
            values.insert(flag, value);
        }
        let mut take = |name: &str| {
            values
                .remove(name)
                .ok_or_else(|| format!("required option {name}"))
        };
        for name in [
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--runtime-rollover",
        ] {
            take(name)?;
        }
        let mode = match take("--argmax-mode")?.as_str() {
            "serial" => ArgmaxMode::Serial,
            "wave-v11" => ArgmaxMode::WaveV11,
            _ => return Err("argmax mode must be serial or wave-v11".into()),
        };
        let outputs = match take("--max-new-tokens")?.as_str() {
            "8" => 8,
            "128" => 128,
            _ => return Err("exactly 8 or 128 outputs required".into()),
        };
        let device = take("--device-unique-id")?
            .parse::<u64>()
            .map_err(|e| e.to_string())?;
        if device == 0 {
            return Err("nonzero physical device identity required".into());
        }
        Ok(Self {
            source: take("--source")?.into(),
            target_artifact: take("--target-artifact")?.into(),
            target_head_artifact: take("--target-head-artifact")?.into(),
            argmax_artifact: take("--argmax-artifact")?.into(),
            worker: take("--worker")?.into(),
            worker_sha256: frozen::checked_sha(&take("--worker-sha256")?)?,
            device,
            reference: take("--reference")?.into(),
            prefix_reference: take("--prefix-reference")?.into(),
            host_timing_output: take("--host-timing-output")?.into(),
            mode,
            outputs,
            runtime: RuntimeOptions {
                cache_admission: true,
                operational: true,
                rollover: true,
                ..RuntimeOptions::default()
            },
        })
    }

    pub fn expected_packets(&self) -> Result<u64, String> {
        616_u64
            .checked_mul(u64::try_from(self.outputs).map_err(|_| "output count")?)
            .and_then(|n| n.checked_add(7 * 613))
            .ok_or("packet budget overflow".into())
    }
}

pub struct Reference {
    pub source: frozen::SourceReference,
    pub prefix: frozen::Reference,
}

pub fn workload_sha256(prompt: &[u32], outputs: usize) -> Result<String, String> {
    if prompt.len() != PROMPT || !matches!(outputs, 8 | 128) {
        return Err("fixed workload hash geometry required".into());
    }
    let value = serde_json::json!({"schema":"FerricArgmaxCanaryWorkloadV1","prompt_token_ids":prompt,
        "max_new_tokens":outputs,"prefill_chunk":CHUNK,"context":CONTEXT,"pages":PAGES});
    Ok(frozen::digest(
        &serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    ))
}

fn read_reference(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 65_536 {
        return Err("reference must be a bounded nonempty regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if u64::try_from(bytes.len()).map_err(|_| "reference size")? != metadata.len() {
        return Err("reference changed size".into());
    }
    Ok(bytes)
}

impl Reference {
    pub fn open(options: &Options) -> Result<Self, String> {
        let original = read_reference(&options.reference)?;
        let prefix = read_reference(&options.prefix_reference)?;
        let prefix = frozen::Reference::parse(&original, &prefix, PREFIX_REFERENCE_SHA256)?;
        let source = serde_json::from_slice(&original).map_err(|e| e.to_string())?;
        Ok(Self { source, prefix })
    }

    pub fn expected_utf8(&self, outputs: usize) -> Result<&str, String> {
        match outputs {
            8 => self
                .prefix
                .prefix_utf8_hex
                .get(7)
                .map(String::as_str)
                .ok_or("missing independent 8-token decode".into()),
            128 => Ok(&self.source.generated_utf8_hex),
            _ => Err("unsupported output extent".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(mode: &str, outputs: &str) -> Vec<String> {
        let mut result = Vec::new();
        for (flag, value) in [
            ("--source", "source"),
            ("--target-artifact", "target"),
            ("--target-head-artifact", "head"),
            ("--argmax-artifact", "argmax"),
            ("--worker", "worker"),
            (
                "--worker-sha256",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            ("--device-unique-id", "1"),
            ("--reference", "reference"),
            ("--prefix-reference", "prefix"),
            ("--host-timing-output", "timing"),
            ("--argmax-mode", mode),
            ("--max-new-tokens", outputs),
        ] {
            result.extend([flag.to_owned(), value.to_owned()]);
        }
        result.extend(
            [
                "--allow-unauthenticated-machine-code",
                "--runtime-cache-admission",
                "--runtime-operational",
                "--runtime-rollover",
            ]
            .map(str::to_owned),
        );
        result
    }

    #[test]
    fn explicit_modes_keep_identical_runtime_and_exact_packet_bounds() {
        for mode in ["serial", "wave-v11"] {
            for (outputs, packets) in [("8", 9_219), ("128", 83_139)] {
                let options = Options::parse(arguments(mode, outputs).into_iter()).unwrap();
                assert_eq!(options.mode.label(), mode);
                assert_eq!(options.expected_packets().unwrap(), packets);
                assert!(
                    options.runtime.operational
                        && options.runtime.cache_admission
                        && options.runtime.rollover
                );
                assert!(
                    !options.runtime.sequences
                        && !options.runtime.ordered_batches
                        && !options.runtime.profile
                );
            }
        }
    }

    #[test]
    fn every_flag_is_required_and_duplicates_or_extra_modes_are_rejected() {
        let args = arguments("serial", "8");
        for index in 0..args.len() {
            if !args[index].starts_with("--") {
                continue;
            }
            let mut missing = args.clone();
            missing.remove(index);
            assert!(Options::parse(missing.into_iter()).is_err());
            let mut duplicate = args.clone();
            duplicate.push(args[index].clone());
            assert!(Options::parse(duplicate.into_iter()).is_err());
        }
        for (mode, outputs) in [("wave", "8"), ("serial", "9"), ("serial", "0")] {
            assert!(Options::parse(arguments(mode, outputs).into_iter()).is_err());
        }
        let mut unsupported = args;
        unsupported.push("--runtime-ordered-batches".into());
        assert!(Options::parse(unsupported.into_iter()).is_err());
    }

    #[test]
    fn frozen_reference_parser_rejects_unpinned_bytes() {
        assert!(frozen::Reference::parse(b"{}", b"{}", PREFIX_REFERENCE_SHA256).is_err());
        assert!(frozen::Reference::parse(b"", b"{}", PREFIX_REFERENCE_SHA256).is_err());
    }

    #[test]
    #[ignore = "requires frozen target and prefix reference files; host-only serde golden check"]
    fn argmax_workload_hash_matches_independently_frozen_python_goldens() {
        let mut options = Options::parse(arguments("serial", "8").into_iter()).unwrap();
        options.reference = std::env::var_os("FERRIC_ARGMAX_TARGET_REFERENCE")
            .expect("target reference")
            .into();
        options.prefix_reference = std::env::var_os("FERRIC_ARGMAX_PREFIX_REFERENCE")
            .expect("prefix reference")
            .into();
        let reference = Reference::open(&options).unwrap();
        for (outputs, expected) in [
            (
                8,
                "de7f971aa48ff2e08d58e94fbcacb73d11b8aac52ad9fdeb769d836aafdfa914",
            ),
            (
                128,
                "c5b11256de5c789f7703cbc0ea757cd2ffe111b02dc6574a4e747ffcb054c9b8",
            ),
        ] {
            assert_eq!(
                workload_sha256(&reference.source.prompt_token_ids, outputs).unwrap(),
                expected
            );
            assert!(!reference.expected_utf8(outputs).unwrap().is_empty());
        }
    }
}
