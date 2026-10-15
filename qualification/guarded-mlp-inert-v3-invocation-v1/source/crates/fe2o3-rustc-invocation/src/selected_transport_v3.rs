//! Exact selected-process joins for inert V3 transport, not execution authority.

use std::{error::Error, fmt};

use fe2o3_build_authority::CompilerClosureV2;

use crate::{
    RustcInvocationDescriptorV2, RustcInvocationDescriptorV3, decode_descriptor_v3,
    encode_descriptor_v3,
};

/// Canonical transport bound to an independently supplied selected process.
///
/// Callers must capture the final selected argv, cwd and complete intended
/// environment, and obtain all six role pins from the actual retained compiler
/// path. This value does not authenticate those inputs, grant execution or
/// publication authority, or discharge conditional runtime premises. In
/// particular, an in-process extraction callback must not invent a backend
/// argument or substitute its own executable for absent wrapper roles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedInvocationTransportV3 {
    descriptor: RustcInvocationDescriptorV3,
    bytes: Vec<u8>,
}

impl SelectedInvocationTransportV3 {
    /// Produces canonical bytes from exact selected inputs and all closure roles.
    ///
    /// An absent closure is an error, never an implicit V2 downgrade. V3's
    /// existing constructor checks the rustc/backend role pins against V2.
    pub fn capture(
        selected: RustcInvocationDescriptorV2,
        closure: Option<CompilerClosureV2>,
    ) -> Result<Self, SelectedInvocationTransportErrorV3> {
        let closure = closure.ok_or(SelectedInvocationTransportErrorV3::MissingCompilerClosure)?;
        let descriptor = RustcInvocationDescriptorV3::new(selected, closure)
            .map_err(|_| SelectedInvocationTransportErrorV3::CompilerRoleMismatch)?;
        let bytes = encode_descriptor_v3(&descriptor)
            .map_err(|_| SelectedInvocationTransportErrorV3::InvalidEncoding)?;
        Ok(Self { descriptor, bytes })
    }

    /// Joins received bytes to independently captured final process and role pins.
    ///
    /// Equality includes every argv element, cwd, environment entry (including
    /// target and normalization flags), and each of the six closure roles.
    /// Merely decoding a self-consistent descriptor is not this custody join.
    pub fn verify(
        bytes: &[u8],
        selected: &RustcInvocationDescriptorV2,
        closure: Option<&CompilerClosureV2>,
    ) -> Result<Self, SelectedInvocationTransportErrorV3> {
        let closure = closure.ok_or(SelectedInvocationTransportErrorV3::MissingCompilerClosure)?;
        let descriptor = decode_descriptor_v3(bytes)
            .map_err(|_| SelectedInvocationTransportErrorV3::InvalidEncoding)?;
        let canonical = encode_descriptor_v3(&descriptor)
            .map_err(|_| SelectedInvocationTransportErrorV3::InvalidEncoding)?;
        if canonical != bytes {
            return Err(SelectedInvocationTransportErrorV3::InvalidEncoding);
        }
        if descriptor.descriptor_v2() != selected {
            return Err(SelectedInvocationTransportErrorV3::SelectedProcessMismatch);
        }
        if descriptor.compiler_closure() != closure {
            return Err(SelectedInvocationTransportErrorV3::CompilerRoleMismatch);
        }
        Ok(Self {
            descriptor,
            bytes: canonical,
        })
    }

    /// Returns canonical V3 bytes; these may contain private environment values.
    ///
    /// Transport them only over the selected private channel, not logs or a
    /// durable public receipt. The caller owns channel selection and lifetime.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the exact joined descriptor, without creating new authority.
    pub const fn descriptor(&self) -> &RustcInvocationDescriptorV3 {
        &self.descriptor
    }
}

/// A closed refusal at the selected-process transport boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedInvocationTransportErrorV3 {
    /// No complete six-role compiler closure was supplied.
    MissingCompilerClosure,
    /// Canonical V3 decoding or encoding failed.
    InvalidEncoding,
    /// The received descriptor does not describe the selected process inputs.
    SelectedProcessMismatch,
    /// A compiler role differs from the independently supplied role pins.
    CompilerRoleMismatch,
}

impl fmt::Display for SelectedInvocationTransportErrorV3 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingCompilerClosure => {
                "selected V3 transport requires all compiler closure roles"
            }
            Self::InvalidEncoding => "selected V3 transport requires canonical V3 encoding",
            Self::SelectedProcessMismatch => {
                "selected V3 transport differs from final selected process"
            }
            Self::CompilerRoleMismatch => {
                "selected V3 transport differs from compiler closure roles"
            }
        })
    }
}

impl Error for SelectedInvocationTransportErrorV3 {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompileEnvironmentV2, RustcUnitV2};

    // Schema-only test data. No executable custody or GPU authority is claimed.
    fn closure(pins: [[u8; 32]; 6]) -> CompilerClosureV2 {
        CompilerClosureV2::new(pins[0], pins[1], pins[2], pins[3], pins[4], pins[5]).unwrap()
    }

    const PINS: [[u8; 32]; 6] = [[1; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32]];

    fn selected(cwd: &str, target: &str, hint: &str) -> RustcInvocationDescriptorV2 {
        let rustc = RustcUnitV2::new(
            cwd,
            vec![
                "/tools/rustc".into(),
                "--crate-name=kernel".into(),
                "kernel.rs".into(),
                "-Ctarget-cpu=gfx950".into(),
                "-Copt-level=3".into(),
                "-Zinline-mir=yes".into(),
                hint.into(),
                "-Zcodegen-backend=/tools/backend.so".into(),
            ],
        )
        .unwrap();
        let env = CompileEnvironmentV2::from_entries_for_test([
            ("FE2O3_TARGET", target),
            ("FE2O3_HSACO_DIR", "/private/artifacts"),
        ])
        .unwrap();
        RustcInvocationDescriptorV2::new(PINS[3], PINS[5], rustc, env).unwrap()
    }

    fn baseline() -> RustcInvocationDescriptorV2 {
        selected(
            "/workspace",
            "gfx950:xnack-",
            "-Zinline-mir-hint-threshold=16384",
        )
    }

    #[test]
    fn canonical_selected_transport_round_trip_preserves_exact_inputs() {
        let process = baseline();
        let roles = closure(PINS);
        let transport =
            SelectedInvocationTransportV3::capture(process.clone(), Some(roles)).unwrap();
        let verified = SelectedInvocationTransportV3::verify(
            transport.canonical_bytes(),
            &process,
            Some(&roles),
        )
        .unwrap();
        assert_eq!(verified, transport);
        assert_eq!(verified.descriptor().descriptor_v2(), &process);
        assert_eq!(verified.descriptor().compiler_closure(), &roles);
    }

    #[test]
    fn missing_closure_never_downgrades_to_v2() {
        assert_eq!(
            SelectedInvocationTransportV3::capture(baseline(), None),
            Err(SelectedInvocationTransportErrorV3::MissingCompilerClosure)
        );
        assert_eq!(
            SelectedInvocationTransportV3::verify(&[], &baseline(), None),
            Err(SelectedInvocationTransportErrorV3::MissingCompilerClosure)
        );
    }

    #[test]
    fn every_compiler_role_is_joined_not_just_rustc_and_backend() {
        let process = baseline();
        let transport =
            SelectedInvocationTransportV3::capture(process.clone(), Some(closure(PINS))).unwrap();
        for index in 0..6 {
            let mut pins = PINS;
            pins[index][0] ^= 0x80;
            assert_eq!(
                SelectedInvocationTransportV3::verify(
                    transport.canonical_bytes(),
                    &process,
                    Some(&closure(pins))
                ),
                Err(SelectedInvocationTransportErrorV3::CompilerRoleMismatch),
                "role {index}"
            );
            if index == 3 || index == 5 {
                assert_eq!(
                    SelectedInvocationTransportV3::capture(process.clone(), Some(closure(pins))),
                    Err(SelectedInvocationTransportErrorV3::CompilerRoleMismatch)
                );
            }
        }
    }

    #[test]
    fn altered_cwd_target_or_normalization_is_not_the_selected_process() {
        let roles = closure(PINS);
        let transport = SelectedInvocationTransportV3::capture(baseline(), Some(roles)).unwrap();
        for process in [
            selected(
                "/other",
                "gfx950:xnack-",
                "-Zinline-mir-hint-threshold=16384",
            ),
            selected(
                "/workspace",
                "gfx942:xnack-",
                "-Zinline-mir-hint-threshold=16384",
            ),
            selected(
                "/workspace",
                "gfx950:xnack-",
                "-Zinline-mir-hint-threshold=4096",
            ),
        ] {
            assert_eq!(
                SelectedInvocationTransportV3::verify(
                    transport.canonical_bytes(),
                    &process,
                    Some(&roles)
                ),
                Err(SelectedInvocationTransportErrorV3::SelectedProcessMismatch)
            );
        }
    }

    #[test]
    fn transport_refuses_v2_truncation_and_trailing_data() {
        let process = baseline();
        let roles = closure(PINS);
        let transport =
            SelectedInvocationTransportV3::capture(process.clone(), Some(roles)).unwrap();
        let mut trailing = transport.canonical_bytes().to_vec();
        trailing.push(0);
        for bytes in [
            crate::encode_descriptor_v2(&process).unwrap(),
            transport.canonical_bytes()[..20].to_vec(),
            trailing,
        ] {
            assert_eq!(
                SelectedInvocationTransportV3::verify(&bytes, &process, Some(&roles)),
                Err(SelectedInvocationTransportErrorV3::InvalidEncoding)
            );
        }
    }

    #[test]
    fn in_process_extractor_cannot_fabricate_a_child_backend_selector() {
        let process = baseline();
        let mut argv = process
            .rustc()
            .argv()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        argv.pop();
        let exact_extractor_inputs = RustcUnitV2::new("/workspace", argv).unwrap();
        assert!(
            RustcInvocationDescriptorV2::new(
                PINS[3],
                PINS[5],
                exact_extractor_inputs,
                process.compile_environment().clone()
            )
            .is_err()
        );
    }
}
