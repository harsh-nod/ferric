//! Default-off selection and inert eligibility, never a publication transition.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;

use fe2o3_amd_target::ProductionAmdTargetProfileV1;
use fe2o3_rustc_invocation::{
    RUSTC_SEPARATE_VALUE_OPTIONS_V2, RustcInvocationDescriptorV3, SelectedInvocationTransportV3,
};

use super::ParentRustcInvocationCustody;

pub(crate) const SELECTOR_ENV: &str = "FE2O3_INERT_SEMANTIC_EXTRACTION_V1";
const SELECTOR: &str = "semantic-v3-gfx950-inline16384-v1";
const CALLER_DESCRIPTOR: &str = "FE2O3_EXTRACT_INERT_RUSTC_INVOCATION_V3_HEX";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Request {
    Disabled,
    Gfx950Inline16384,
}

impl Request {
    fn parse(value: Option<&OsStr>) -> Result<Self, Refusal> {
        match value {
            None => Ok(Self::Disabled),
            Some(value) if value == OsStr::new(SELECTOR) => Ok(Self::Gfx950Inline16384),
            Some(_) => Err(Refusal::InvalidSelector),
        }
    }

    /// Staged for a separate extraction-only child route. The caller must
    /// retain the real sealed capability, not construct custody from raw bytes.
    #[allow(dead_code)]
    pub(crate) fn prepare<'a>(
        self,
        parent: Option<&'a ParentRustcInvocationCustody>,
    ) -> Result<Option<Eligibility<'a>>, Refusal> {
        if self == Self::Disabled {
            return Ok(None);
        }
        let parent = parent.ok_or(Refusal::MissingRetainedCustody)?;
        parent
            .revalidate()
            .map_err(|_| Refusal::RetainedCustodyChanged)?;
        let transport = prepare_descriptor(parent.descriptor())?;
        Ok(Some(Eligibility {
            _parent: parent,
            transport,
        }))
    }
}

/// Keeps the original parent alive; has no conversion to protected publication
/// or execution. A future extraction-only consumer must revalidate it again.
#[allow(dead_code)]
pub(crate) struct Eligibility<'a> {
    _parent: &'a ParentRustcInvocationCustody,
    transport: SelectedInvocationTransportV3,
}

impl Eligibility<'_> {
    /// Rejoin the retained parent image and exact prepared transport. This is
    /// input consistency, not proof of child execution or compiler authorship.
    pub(super) fn revalidate_for_completion(
        &self,
    ) -> Result<&RustcInvocationDescriptorV3, Refusal> {
        self._parent
            .revalidate()
            .map_err(|_| Refusal::RetainedCustodyChanged)?;
        let selected = self._parent.descriptor();
        SelectedInvocationTransportV3::verify(
            self.transport.canonical_bytes(),
            selected.descriptor_v2(),
            Some(selected.compiler_closure()),
        )
        .map_err(|_| Refusal::RetainedCustodyChanged)?;
        Ok(selected)
    }
}

/// Runs before any managed attempt, capability admission or rustc spawn.
/// Nothing is forwarded to a child until a lossless extraction-only consumer
/// exists. In particular, eligibility must not enable the ordinary protected
/// completion path, which requires a publication transaction.
pub(crate) fn preflight_before_attempt(
    environment: impl IntoIterator<Item = (OsString, OsString)>,
) -> Result<(), Refusal> {
    let mut selector = None;
    let mut descriptor_present = false;
    let mut incompatible = false;
    for (key, value) in environment {
        if key == OsStr::new(SELECTOR_ENV) {
            if selector.replace(value).is_some() {
                return Err(Refusal::InvalidSelector);
            }
        } else if key == OsStr::new(CALLER_DESCRIPTOR) {
            descriptor_present = true;
        } else if key.as_encoded_bytes().starts_with(b"FE2O3_EXTRACT_") {
            incompatible = true;
        }
    }
    if descriptor_present {
        return Err(Refusal::CallerDescriptorForbidden);
    }
    let request = Request::parse(selector.as_deref())?;
    if request == Request::Disabled {
        return Ok(());
    }
    if incompatible {
        return Err(Refusal::IncompatibleExtractionMode);
    }
    // Deliberately no environment mutation, descriptor injection, attempt,
    // spawn, or downgrade to the protected publication transaction.
    Err(Refusal::ExtractionOnlyConsumerUnqualified)
}

fn prepare_descriptor(
    descriptor: &RustcInvocationDescriptorV3,
) -> Result<SelectedInvocationTransportV3, Refusal> {
    if descriptor.amd_target() != ProductionAmdTargetProfileV1::Gfx950.device_target() {
        return Err(Refusal::TargetOrNormalizationMismatch);
    }
    require_profile(descriptor)?;
    SelectedInvocationTransportV3::capture(
        descriptor.descriptor_v2().clone(),
        Some(*descriptor.compiler_closure()),
    )
    .map_err(|_| Refusal::RetainedCustodyChanged)
}

/// Walk rustc's frozen option grammar so option values cannot masquerade as
/// profile flags. Controlled duplicate/alias settings are rejected, not last-wins.
fn require_profile(descriptor: &RustcInvocationDescriptorV3) -> Result<(), Refusal> {
    let profile = ProductionAmdTargetProfileV1::Gfx950;
    let required = BTreeMap::from([
        ("target".to_owned(), profile.rustc_target()),
        ("Ctarget-cpu".to_owned(), profile.cpu()),
        ("Ctarget-feature".to_owned(), profile.rustc_features()),
        ("Copt-level".to_owned(), "3"),
        ("Zalways-encode-mir".to_owned(), "yes"),
        ("Zinline-mir".to_owned(), "yes"),
        ("Zinline-mir-hint-threshold".to_owned(), "16384"),
        ("Zmir-enable-passes".to_owned(), "-JumpThreading"),
    ]);
    let args = descriptor.rustc().argv().collect::<Vec<_>>();
    let mut found = BTreeMap::new();
    let mut index = 1;
    while index < args.len() {
        let arg = args[index];
        let mut consumed = 1;
        let option = if arg == "--target" {
            consumed = 2;
            args.get(index + 1).map(|v| ("target".to_owned(), *v))
        } else if let Some(value) = arg.strip_prefix("--target=") {
            Some(("target".to_owned(), value))
        } else {
            let option = if matches!(arg, "-C" | "--codegen" | "-Z") {
                consumed = 2;
                args.get(index + 1)
                    .map(|v| (if arg == "-Z" { "Z" } else { "C" }, *v))
            } else {
                arg.strip_prefix("--codegen=")
                    .map(|v| ("C", v))
                    .or_else(|| arg.strip_prefix("-C").map(|v| ("C", v)))
                    .or_else(|| arg.strip_prefix("-Z").map(|v| ("Z", v)))
            };
            option.map(|(family, value)| {
                let value = value.strip_prefix('=').unwrap_or(value);
                let (name, value) = value.split_once('=').unwrap_or((value, "yes"));
                (format!("{family}{}", name.replace('_', "-")), value)
            })
        };
        if let Some((key, value)) = option {
            if let Some(expected) = required.get(&key) {
                if value != *expected || found.insert(key, value).is_some() {
                    return Err(Refusal::TargetOrNormalizationMismatch);
                }
            } else if key.starts_with("Zinline-mir") || key.starts_with("Zmir-") {
                return Err(Refusal::TargetOrNormalizationMismatch);
            }
        } else if RUSTC_SEPARATE_VALUE_OPTIONS_V2.contains(&arg) {
            consumed = 2;
        }
        if index + consumed > args.len() {
            return Err(Refusal::TargetOrNormalizationMismatch);
        }
        index += consumed;
    }
    if found.len() != required.len() {
        return Err(Refusal::TargetOrNormalizationMismatch);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    InvalidSelector,
    CallerDescriptorForbidden,
    IncompatibleExtractionMode,
    MissingRetainedCustody,
    RetainedCustodyChanged,
    TargetOrNormalizationMismatch,
    ExtractionOnlyConsumerUnqualified,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidSelector => "inert semantic extraction selector must be semantic-v3-gfx950-inline16384-v1",
            Self::CallerDescriptorForbidden => "inert semantic extraction forbids caller-supplied invocation descriptors",
            Self::IncompatibleExtractionMode => "inert semantic extraction conflicts with an existing extraction mode",
            Self::MissingRetainedCustody => "inert semantic extraction requires retained sealed parent invocation custody",
            Self::RetainedCustodyChanged => "inert semantic extraction retained invocation custody changed",
            Self::TargetOrNormalizationMismatch => "inert semantic extraction requires exact gfx950 optimized-inline-hint-16384 inputs",
            Self::ExtractionOnlyConsumerUnqualified => "inert semantic extraction remains disabled: lossless extraction-only child consumer is not qualified; protected publication fallback is forbidden",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe2o3_build_authority::CompilerClosureV2;
    use fe2o3_rustc_invocation::{CompileEnvironmentV2, RustcInvocationDescriptorV2, RustcUnitV2};

    fn env(values: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        values
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }

    // Schema fixtures exercise parsing, not actual retained compiler custody.
    fn descriptor(target: &str, flags: &[String]) -> RustcInvocationDescriptorV3 {
        let mut args = vec![
            "/tools/rustc".into(),
            "--crate-name=kernel".into(),
            "kernel.rs".into(),
        ];
        args.extend(flags.iter().cloned());
        args.push("-Zcodegen-backend=/tools/backend.so".into());
        let environment = CompileEnvironmentV2::from_child_environment(env(&[
            ("FE2O3_TARGET", target),
            ("FE2O3_HSACO_DIR", "/private/artifacts"),
        ]))
        .unwrap();
        let v2 = RustcInvocationDescriptorV2::new(
            [4; 32],
            [6; 32],
            RustcUnitV2::new("/workspace", args).unwrap(),
            environment,
        )
        .unwrap();
        RustcInvocationDescriptorV3::new(
            v2,
            CompilerClosureV2::new([1; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32]).unwrap(),
        )
        .unwrap()
    }

    fn flags() -> Vec<String> {
        let p = ProductionAmdTargetProfileV1::Gfx950;
        vec![
            format!("--target={}", p.rustc_target()),
            format!("-Ctarget-cpu={}", p.cpu()),
            format!("-Ctarget-feature={}", p.rustc_features()),
            "-Copt-level=3".into(),
            "-Zalways-encode-mir".into(),
            "-Zinline-mir=yes".into(),
            "-Zinline-mir-hint-threshold=16384".into(),
            "-Zmir-enable-passes=-JumpThreading".into(),
        ]
    }

    #[test]
    fn absent_selector_preserves_ordinary_preflight() {
        assert_eq!(
            preflight_before_attempt(env(&[("PATH", "/usr/bin")])),
            Ok(())
        );
        assert_eq!(Request::Disabled.prepare(None).unwrap().is_none(), true);
    }

    #[test]
    fn exact_opt_in_is_held_before_attempt_not_publication_fallback() {
        assert_eq!(
            Request::parse(Some(OsStr::new(SELECTOR))),
            Ok(Request::Gfx950Inline16384)
        );
        assert_eq!(
            preflight_before_attempt(env(&[(SELECTOR_ENV, SELECTOR)])),
            Err(Refusal::ExtractionOnlyConsumerUnqualified)
        );
    }

    #[test]
    fn invalid_empty_duplicate_and_non_utf8_selectors_refuse() {
        use std::os::unix::ffi::OsStringExt;
        for value in [
            OsString::from(""),
            "1".into(),
            "semantic-v3".into(),
            OsString::from_vec(vec![255]),
        ] {
            assert_eq!(Request::parse(Some(&value)), Err(Refusal::InvalidSelector));
        }
        assert_eq!(
            preflight_before_attempt(env(&[(SELECTOR_ENV, SELECTOR), (SELECTOR_ENV, SELECTOR)])),
            Err(Refusal::InvalidSelector)
        );
    }

    #[test]
    fn caller_descriptor_is_never_adopted_even_without_selector() {
        for values in [
            env(&[(CALLER_DESCRIPTOR, "00")]),
            env(&[(SELECTOR_ENV, SELECTOR), (CALLER_DESCRIPTOR, "")]),
        ] {
            assert_eq!(
                preflight_before_attempt(values),
                Err(Refusal::CallerDescriptorForbidden)
            );
        }
    }

    #[test]
    fn existing_extraction_modes_are_incompatible_with_opt_in() {
        for mode in [
            "FE2O3_EXTRACT_RANKED_MEMORY_V1",
            "FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1",
            "FE2O3_EXTRACT_SIMULATION_BUNDLE_PATH_V6",
            "FE2O3_EXTRACT_WORKER_LAYOUT_V1",
        ] {
            assert_eq!(
                preflight_before_attempt(env(&[(SELECTOR_ENV, SELECTOR), (mode, "1")])),
                Err(Refusal::IncompatibleExtractionMode)
            );
            assert_eq!(preflight_before_attempt(env(&[(mode, "1")])), Ok(()));
        }
    }

    #[test]
    fn requested_eligibility_requires_live_parent_not_schema_fixture() {
        assert!(matches!(
            Request::Gfx950Inline16384.prepare(None),
            Err(Refusal::MissingRetainedCustody)
        ));
    }

    #[test]
    fn exact_gfx950_profile_prepares_only_inert_canonical_transport() {
        let d = descriptor("gfx950:xnack-", &flags());
        let transport = prepare_descriptor(&d).unwrap();
        assert_eq!(transport.descriptor(), &d);
        assert_eq!(
            SelectedInvocationTransportV3::verify(
                transport.canonical_bytes(),
                d.descriptor_v2(),
                Some(d.compiler_closure())
            )
            .unwrap(),
            transport
        );
    }

    #[test]
    fn wrong_target_missing_flags_changed_hint_and_duplicate_alias_refuse() {
        assert_eq!(
            prepare_descriptor(&descriptor("gfx942:xnack-", &flags())).unwrap_err(),
            Refusal::TargetOrNormalizationMismatch
        );
        for index in 0..flags().len() {
            let mut missing = flags();
            missing.remove(index);
            assert_eq!(
                prepare_descriptor(&descriptor("gfx950:xnack-", &missing)).unwrap_err(),
                Refusal::TargetOrNormalizationMismatch
            );
        }
        for extra in [
            "-Zinline_mir_hint_threshold=4096",
            "-Zinline_mir_hint_threshold=16384",
            "-Ctarget-cpu=gfx942",
            "-Ctarget-feature=+wavefrontsize32",
            "-Z=inline-mir=no",
            "-Zmir-opt-level=0",
            "-Zinline-mir-threshold=999999",
        ] {
            let mut changed = flags();
            changed.push(extra.into());
            assert_eq!(
                prepare_descriptor(&descriptor("gfx950:xnack-", &changed)).unwrap_err(),
                Refusal::TargetOrNormalizationMismatch
            );
        }
    }

    #[test]
    fn split_options_work_but_cfg_values_cannot_supply_missing_flags() {
        let mut split = flags();
        split[0] = "--target".into();
        split.insert(
            1,
            ProductionAmdTargetProfileV1::Gfx950.rustc_target().into(),
        );
        assert!(prepare_descriptor(&descriptor("gfx950:xnack-", &split)).is_ok());
        let mut disguised = flags();
        let hint = disguised.remove(6);
        disguised.push("--cfg".into());
        disguised.push(hint);
        assert_eq!(
            prepare_descriptor(&descriptor("gfx950:xnack-", &disguised)).unwrap_err(),
            Refusal::TargetOrNormalizationMismatch
        );
    }

    #[test]
    fn source_keeps_selector_before_attempt_and_spawn_and_holds_consumer() {
        let wrapper = include_str!("../binding_wrapper.rs");
        let body = wrapper.split_once("pub(crate) fn run(mut argv:").unwrap().1;
        assert!(
            body.find("preflight_before_attempt(").unwrap()
                < body.find("expected_rustc_sha256()?").unwrap()
        );
        let preflight = include_str!("inert_semantic_extraction_v1.rs")
            .split_once("pub(crate) fn preflight_before_attempt(")
            .unwrap()
            .1
            .split_once("fn prepare_descriptor(")
            .unwrap()
            .0;
        assert!(preflight.contains("Err(Refusal::ExtractionOnlyConsumerUnqualified)"));
        assert!(!preflight.contains("command.spawn"));
    }
}
