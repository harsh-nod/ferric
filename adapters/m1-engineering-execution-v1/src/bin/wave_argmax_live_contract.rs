//! Closed live-only profile; no legacy CLI or default policy is widened.

use super::tp_worker::RuntimeOptions;
use ferric_m1_engineering_execution_v1::tp_paged::EngineeringTpPagedLimitsV1;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const ROWS: usize = 32;
pub const CHUNK: usize = 16;
pub const CACHE_TTL: u64 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Submission {
    Synchronous,
    Ordered,
}

impl Submission {
    pub const fn ordered(self) -> bool {
        matches!(self, Self::Ordered)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Synchronous => "synchronous",
            Self::Ordered => "ordered",
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub source: PathBuf,
    pub target_artifact: PathBuf,
    pub target_head_artifact: PathBuf,
    pub argmax_artifact: PathBuf,
    pub worker: PathBuf,
    pub worker_sha256: String,
    pub device: u64,
    pub submission: Submission,
    pub context: u32,
    pub pages: u32,
    pub max_batches: u64,
    pub host_timing: Option<PathBuf>,
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
                "--live-stdin"
                | "--allow-unauthenticated-machine-code"
                | "--runtime-cache-admission"
                | "--runtime-operational"
                | "--queue-rollover"
                | "--disable-prefix-cache"
                | "--prune-output-head" => String::new(),
                "--source"
                | "--target-artifact"
                | "--target-head-artifact"
                | "--argmax-artifact"
                | "--worker"
                | "--worker-sha256"
                | "--device-unique-id"
                | "--submission"
                | "--context"
                | "--pages"
                | "--max-batches"
                | "--host-timing" => arguments
                    .next()
                    .ok_or_else(|| format!("missing value for {flag}"))?,
                _ => return Err(format!("unsupported live profile option {flag}")),
            };
            values.insert(flag, value);
        }
        let host_timing = values.remove("--host-timing").map(PathBuf::from);
        let mut take = |name: &str| {
            values
                .remove(name)
                .ok_or_else(|| format!("required option {name}"))
        };
        for flag in [
            "--live-stdin",
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--queue-rollover",
            "--disable-prefix-cache",
            "--prune-output-head",
        ] {
            take(flag)?;
        }
        let submission = match take("--submission")?.as_str() {
            "synchronous" => Submission::Synchronous,
            "ordered" => Submission::Ordered,
            _ => return Err("submission must be synchronous or ordered".into()),
        };
        let options = Self {
            source: take("--source")?.into(),
            target_artifact: take("--target-artifact")?.into(),
            target_head_artifact: take("--target-head-artifact")?.into(),
            argmax_artifact: take("--argmax-artifact")?.into(),
            worker: take("--worker")?.into(),
            worker_sha256: take("--worker-sha256")?,
            device: take("--device-unique-id")?
                .parse::<u64>()
                .map_err(|e| e.to_string())?,
            submission,
            context: take("--context")?
                .parse::<u32>()
                .map_err(|e| e.to_string())?,
            pages: take("--pages")?.parse::<u32>().map_err(|e| e.to_string())?,
            max_batches: take("--max-batches")?
                .parse::<u64>()
                .map_err(|e| e.to_string())?,
            host_timing,
            runtime: RuntimeOptions {
                cache_admission: true,
                operational: true,
                rollover: true,
                ordered_batches: submission.ordered(),
                ..RuntimeOptions::default()
            },
        };
        options.validate()?;
        Ok(options)
    }

    pub fn limits(&self) -> Result<EngineeringTpPagedLimitsV1, String> {
        EngineeringTpPagedLimitsV1::new(self.context, 32, self.pages, CACHE_TTL)
            .map_err(|e| format!("legacy live page limits: {e:?}"))
    }

    pub fn validate(&self) -> Result<(), String> {
        self.limits()?;
        if self.device == 0
            || !(1..=1_000_000).contains(&self.max_batches)
            || self.worker_sha256.len() != 64
            || !self
                .worker_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || self.worker_sha256.bytes().all(|byte| byte == b'0')
            || !self.runtime.cache_admission
            || !self.runtime.operational
            || !self.runtime.rollover
            || self.runtime.ordered_batches != self.submission.ordered()
            || self.runtime.sequences
            || self.runtime.profile
            || self.runtime.shared_full_currentness
        {
            return Err("live wave/v8/v11 requires nonzero device and worker hash, bounded batches, matching submission, cache admission, operational currentness and rollover; sequences, profiling and shared currentness are excluded".into());
        }
        for path in [
            &self.source,
            &self.target_artifact,
            &self.target_head_artifact,
            &self.argmax_artifact,
            &self.worker,
        ]
        .into_iter()
        .chain(self.host_timing.as_ref())
        {
            if path.as_os_str().is_empty() {
                return Err("live profile paths must be nonempty".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn fixture(submission: Submission) -> Options {
    Options {
        source: "source".into(),
        target_artifact: "target".into(),
        target_head_artifact: "head".into(),
        argmax_artifact: "argmax".into(),
        worker: "worker".into(),
        worker_sha256: "a".repeat(64),
        device: 1,
        submission,
        context: 8192,
        pages: 512,
        max_batches: 1_000_000,
        host_timing: None,
        runtime: RuntimeOptions {
            cache_admission: true,
            operational: true,
            rollover: true,
            ordered_batches: submission.ordered(),
            ..RuntimeOptions::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(submission: &str) -> Vec<String> {
        let mut arguments = Vec::new();
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
            ("--submission", submission),
            ("--context", "8192"),
            ("--pages", "512"),
            ("--max-batches", "1000000"),
        ] {
            arguments.extend([flag.to_owned(), value.to_owned()]);
        }
        arguments.extend(
            [
                "--live-stdin",
                "--allow-unauthenticated-machine-code",
                "--runtime-cache-admission",
                "--runtime-operational",
                "--queue-rollover",
                "--disable-prefix-cache",
                "--prune-output-head",
            ]
            .map(str::to_owned),
        );
        arguments
    }

    #[test]
    fn live_submission_modes_preserve_the_matched_8192_envelope() {
        for submission in ["synchronous", "ordered"] {
            let options = Options::parse(arguments(submission).into_iter()).unwrap();
            assert_eq!(options.submission.label(), submission);
            assert_eq!(options.runtime.ordered_batches, submission == "ordered");
            assert_eq!(
                (ROWS, CHUNK, options.context, options.pages),
                (32, 16, 8192, 512)
            );
            assert_eq!(options.limits().unwrap().page_table_stride(), 512);
            assert_eq!(
                options.limits().unwrap().physical_token_capacity().unwrap(),
                8192
            );
            assert_eq!((128_u32 + 128 - 1).div_ceil(16), 16);
            assert!(options.host_timing.is_none());
        }
    }

    #[test]
    fn live_parser_requires_every_profile_flag_and_rejects_duplicates() {
        let original = arguments("ordered");
        for (index, flag) in original
            .iter()
            .enumerate()
            .filter(|(_, value)| value.starts_with("--"))
        {
            let mut missing = original.clone();
            missing.remove(index);
            assert!(Options::parse(missing.into_iter()).is_err(), "{flag}");
            let mut duplicate = original.clone();
            duplicate.push(flag.clone());
            assert!(Options::parse(duplicate.into_iter()).is_err(), "{flag}");
        }
    }

    #[test]
    fn live_parser_rejects_other_execution_and_storage_profiles() {
        for flag in [
            "--requests",
            "--benchmark-control",
            "--numerical-capture",
            "--runtime-profile",
            "--runtime-ordered-batches",
            "--dispatch-sequences",
            "--runtime-sequences",
            "--peer-artifact",
            "--peer-shared-full-currentness",
            "--devices",
            "--speculative-k",
            "--draft-artifact",
            "--kv-pool-profile",
            "--large-kv-artifact",
            "--attention",
            "--projection",
            "--head-precision",
            "--argmax-mode",
            "--batch-tokens",
            "--prefill-chunk",
        ] {
            let mut changed = arguments("ordered");
            changed.extend([flag.to_owned(), "unreviewed".to_owned()]);
            assert!(Options::parse(changed.into_iter()).is_err(), "{flag}");
        }
        for submission in ["", "auto", "serial", "Ordered", "ordered=synchronous"] {
            assert!(Options::parse(arguments(submission).into_iter()).is_err());
        }
    }

    #[test]
    fn live_limits_keep_logical_context_and_aggregate_pages_distinct() {
        for (context, pages) in [(1, 1), (256, 16), (8192, 16), (8192, 512)] {
            let mut options = fixture(Submission::Synchronous);
            options.context = context;
            options.pages = pages;
            options.validate().unwrap();
            assert_eq!(
                options.limits().unwrap().physical_token_capacity().unwrap(),
                pages * 16
            );
        }
        for (context, pages) in [(0, 16), (8193, 512), (256, 0), (8192, 513), (8192, 16384)] {
            let mut options = fixture(Submission::Ordered);
            options.context = context;
            options.pages = pages;
            assert!(options.validate().is_err());
        }
    }

    #[test]
    fn direct_live_options_reject_runtime_identity_and_budget_drift() {
        for submission in [Submission::Synchronous, Submission::Ordered] {
            for mutation in 0..15 {
                let mut options = fixture(submission);
                match mutation {
                    0 => options.runtime.cache_admission = false,
                    1 => options.runtime.operational = false,
                    2 => options.runtime.rollover = false,
                    3 => options.runtime.ordered_batches = !submission.ordered(),
                    4 => options.runtime.sequences = true,
                    5 => options.runtime.profile = true,
                    6 => options.runtime.shared_full_currentness = true,
                    7 => options.device = 0,
                    8 => options.worker_sha256 = "0".repeat(64),
                    9 => options.worker_sha256 = "A".repeat(64),
                    10 => options.worker_sha256 = "a".repeat(63),
                    11 => options.max_batches = 0,
                    12 => options.max_batches = 1_000_001,
                    13 => options.source = PathBuf::new(),
                    14 => options.host_timing = Some(PathBuf::new()),
                    _ => unreachable!(),
                }
                assert!(options.validate().is_err(), "{submission:?}/{mutation}");
            }
        }
    }

    #[test]
    fn live_parser_preserves_option_shaped_paths_and_exclusive_timing_choice() {
        let mut original = arguments("ordered");
        let source = original.iter().position(|arg| arg == "--source").unwrap() + 1;
        original[source] = "--runtime-profile".into();
        original.extend(["--host-timing".into(), "--requests".into()]);
        let options = Options::parse(original.clone().into_iter()).unwrap();
        assert_eq!(options.source, PathBuf::from("--runtime-profile"));
        assert_eq!(options.host_timing, Some(PathBuf::from("--requests")));
        assert!(!options.runtime.profile);
        original.extend(["--host-timing".into(), "other".into()]);
        assert!(Options::parse(original.into_iter()).is_err());
    }
}
