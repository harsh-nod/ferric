//! Guarded host-visible backing for one opt-in physical K7 completion image.
//!
//! The generic runtime owns allocation identity, queue association, completed
//! generation checks, and copying. This module owns only the Ferric-specific
//! layout and validates two adjacent fixed guard regions before exposing the
//! copied K7 interior to the existing decoder. The result is deliberately a
//! narrow observation of one allocation layout, not a general bounds proof.

use core::fmt;

use fe2o3_service_host::{
    ServiceCompletedReadbackV1, ServiceHostDispatchRangeV1, ServiceHostDispatchSnapshotRangeV1,
};
use sha2::{Digest, Sha256};

use crate::M1CompletionOutputShapeV1;

/// Exact adjacent guard extent on each side of the K7 output interior.
pub const M1_COMPLETION_CANARY_GUARD_BYTES_V1: u64 = 64;
/// Canonical initialized byte for the guard before the K7 output interior.
pub const M1_COMPLETION_CANARY_PREFIX_BYTE_V1: u8 = 0xA5;
/// Canonical initialized byte for the guard after the K7 output interior.
pub const M1_COMPLETION_CANARY_SUFFIX_BYTE_V1: u8 = 0x5A;

/// Checked enclosing and interior geometry for one guarded K7 output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1CompletionCanaryLayoutV1 {
    interior_extent_bytes: u64,
    snapshot_extent_bytes: u64,
}

impl M1CompletionCanaryLayoutV1 {
    /// Derives `[64-byte prefix | exact K7 output | 64-byte suffix]`.
    ///
    /// # Errors
    ///
    /// Returns [`M1CompletionCanaryErrorV1::ExtentOverflow`] if the enclosing
    /// host representation cannot contain the exact layout.
    pub fn for_shape(shape: M1CompletionOutputShapeV1) -> Result<Self, M1CompletionCanaryErrorV1> {
        let snapshot_extent_bytes = M1_COMPLETION_CANARY_GUARD_BYTES_V1
            .checked_add(shape.extent_bytes())
            .and_then(|extent| extent.checked_add(M1_COMPLETION_CANARY_GUARD_BYTES_V1))
            .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
        usize::try_from(snapshot_extent_bytes)
            .map_err(|_| M1CompletionCanaryErrorV1::ExtentOverflow)?;
        Ok(Self {
            interior_extent_bytes: shape.extent_bytes(),
            snapshot_extent_bytes,
        })
    }

    /// Relative offset of the exact K7 writable interior.
    #[must_use]
    pub const fn interior_offset_bytes(self) -> u64 {
        M1_COMPLETION_CANARY_GUARD_BYTES_V1
    }

    /// Exact K7 writable interior extent.
    #[must_use]
    pub const fn interior_extent_bytes(self) -> u64 {
        self.interior_extent_bytes
    }

    /// Exact initialized enclosing snapshot extent.
    #[must_use]
    pub const fn snapshot_extent_bytes(self) -> u64 {
        self.snapshot_extent_bytes
    }

    pub(crate) fn initialized_bytes(self) -> Result<Box<[u8]>, M1CompletionCanaryErrorV1> {
        let snapshot_extent = usize::try_from(self.snapshot_extent_bytes)
            .map_err(|_| M1CompletionCanaryErrorV1::ExtentOverflow)?;
        let interior_start = usize::try_from(self.interior_offset_bytes())
            .map_err(|_| M1CompletionCanaryErrorV1::ExtentOverflow)?;
        let interior_extent = usize::try_from(self.interior_extent_bytes)
            .map_err(|_| M1CompletionCanaryErrorV1::ExtentOverflow)?;
        let interior_end = interior_start
            .checked_add(interior_extent)
            .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
        let mut bytes = vec![M1_COMPLETION_CANARY_PREFIX_BYTE_V1; snapshot_extent];
        bytes
            .get_mut(interior_start..interior_end)
            .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?
            .fill(0);
        bytes
            .get_mut(interior_end..)
            .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?
            .fill(M1_COMPLETION_CANARY_SUFFIX_BYTE_V1);
        Ok(bytes.into_boxed_slice())
    }
}

/// Ferric layout and generic enclosing-snapshot association retained together.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BoundM1CompletionCanaryV1 {
    layout: M1CompletionCanaryLayoutV1,
    snapshot_range: ServiceHostDispatchSnapshotRangeV1,
}

impl BoundM1CompletionCanaryV1 {
    pub(crate) const fn new(
        layout: M1CompletionCanaryLayoutV1,
        snapshot_range: ServiceHostDispatchSnapshotRangeV1,
    ) -> Self {
        Self {
            layout,
            snapshot_range,
        }
    }

    /// Exact Ferric guard and K7 interior layout.
    #[must_use]
    pub const fn layout(self) -> M1CompletionCanaryLayoutV1 {
        self.layout
    }

    /// Generic initialized enclosing range retained before owner transfer.
    #[must_use]
    pub const fn snapshot_range(self) -> ServiceHostDispatchSnapshotRangeV1 {
        self.snapshot_range
    }
}

/// Immutable coordinates and digests from one validated enclosing readback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1ObservedCompletionCanarySummaryV1 {
    dispatch_generation: u64,
    data_index: usize,
    snapshot_offset_bytes: u64,
    snapshot_extent_bytes: u64,
    interior_offset_bytes: u64,
    interior_extent_bytes: u64,
    prefix_sha256: [u8; 32],
    snapshot_sha256: [u8; 32],
    interior_sha256: [u8; 32],
    suffix_sha256: [u8; 32],
}

impl M1ObservedCompletionCanarySummaryV1 {
    /// Completed queue generation authorizing the one enclosing copy.
    #[must_use]
    pub const fn dispatch_generation(self) -> u64 {
        self.dispatch_generation
    }

    /// Addressless allocation ordinal returned by the completed copy.
    #[must_use]
    pub const fn data_index(self) -> usize {
        self.data_index
    }

    /// Exact enclosing copied offset.
    #[must_use]
    pub const fn snapshot_offset_bytes(self) -> u64 {
        self.snapshot_offset_bytes
    }

    /// Exact enclosing copied extent.
    #[must_use]
    pub const fn snapshot_extent_bytes(self) -> u64 {
        self.snapshot_extent_bytes
    }

    /// Exact absolute K7 interior offset within the same allocation.
    #[must_use]
    pub const fn interior_offset_bytes(self) -> u64 {
        self.interior_offset_bytes
    }

    /// Exact K7 interior extent exposed to existing decoding.
    #[must_use]
    pub const fn interior_extent_bytes(self) -> u64 {
        self.interior_extent_bytes
    }

    /// SHA-256 of the exact checked prefix guard.
    #[must_use]
    pub const fn prefix_sha256(self) -> [u8; 32] {
        self.prefix_sha256
    }

    /// SHA-256 of the exact enclosing copied bytes, including both guards.
    #[must_use]
    pub const fn snapshot_sha256(self) -> [u8; 32] {
        self.snapshot_sha256
    }

    /// SHA-256 of only the exact K7 writable interior.
    #[must_use]
    pub const fn interior_sha256(self) -> [u8; 32] {
        self.interior_sha256
    }

    /// SHA-256 of the exact checked suffix guard.
    #[must_use]
    pub const fn suffix_sha256(self) -> [u8; 32] {
        self.suffix_sha256
    }
}

/// Owned enclosing copy after exact coordinate and adjacent-guard validation.
#[derive(Debug)]
pub(crate) struct M1ValidatedCompletionCanaryReadbackV1 {
    readback: ServiceCompletedReadbackV1,
    interior_start: usize,
    interior_end: usize,
    summary: M1ObservedCompletionCanarySummaryV1,
}

impl M1ValidatedCompletionCanaryReadbackV1 {
    pub(crate) const fn dispatch_generation(&self) -> u64 {
        self.summary.dispatch_generation
    }

    pub(crate) const fn data_index(&self) -> usize {
        self.summary.data_index
    }

    pub(crate) const fn interior_offset_bytes(&self) -> u64 {
        self.summary.interior_offset_bytes
    }

    pub(crate) fn interior_bytes(&self) -> &[u8] {
        &self.readback.bytes()[self.interior_start..self.interior_end]
    }

    pub(crate) const fn summary(&self) -> M1ObservedCompletionCanarySummaryV1 {
        self.summary
    }

    pub(crate) fn into_readback(self) -> ServiceCompletedReadbackV1 {
        self.readback
    }
}

/// Fail-closed guarded layout, completed-coordinate, or guard rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1CompletionCanaryErrorV1 {
    /// Enclosing geometry overflowed its host representation.
    ExtentOverflow,
    /// A zero dispatch generation cannot identify completed queue work.
    ZeroDispatchGeneration,
    /// Revalidated enclosing allocation extent differs from the retained layout.
    SnapshotExtentDrift { expected: u64, actual: u64 },
    /// Revalidated K7 interior extent differs from the retained layout.
    InteriorExtentDrift { expected: u64, actual: u64 },
    /// The K7 interior no longer starts exactly after the prefix guard.
    InteriorOffsetDrift { expected: u64, actual: u64 },
    /// The completed copy no longer begins at the retained snapshot offset.
    SnapshotOffsetDrift { expected: u64, actual: u64 },
    /// The completed copy length differs from the retained snapshot extent.
    ReadbackExtentDrift { expected: u64, actual: u64 },
    /// One prefix guard byte changed.
    PrefixGuardMismatch { offset: usize, actual: u8 },
    /// One suffix guard byte changed.
    SuffixGuardMismatch { offset: usize, actual: u8 },
}

impl fmt::Display for M1CompletionCanaryErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 completion canary rejected: {self:?}")
    }
}

impl std::error::Error for M1CompletionCanaryErrorV1 {}

/// Validates one completed enclosing copy and preserves it on rejection.
pub(crate) fn validate_m1_completion_canary_readback_v1(
    binding: BoundM1CompletionCanaryV1,
    interior_range: ServiceHostDispatchRangeV1,
    readback: ServiceCompletedReadbackV1,
) -> Result<
    M1ValidatedCompletionCanaryReadbackV1,
    (M1CompletionCanaryErrorV1, ServiceCompletedReadbackV1),
> {
    let result = validate_readback_facts(
        binding.layout,
        binding.snapshot_range.offset_bytes(),
        binding.snapshot_range.extent_bytes(),
        interior_range.offset_bytes(),
        interior_range.extent_bytes(),
        readback.dispatch_generation(),
        readback.data_index(),
        readback.offset_bytes(),
        readback.bytes(),
    );
    let (interior_start, interior_end, summary) = match result {
        Ok(result) => result,
        Err(error) => return Err((error, readback)),
    };
    Ok(M1ValidatedCompletionCanaryReadbackV1 {
        readback,
        interior_start,
        interior_end,
        summary,
    })
}

/// Revalidates retained Ferric geometry before attempting the one snapshot copy.
pub(crate) fn preflight_m1_completion_canary_v1(
    binding: BoundM1CompletionCanaryV1,
    interior_range: ServiceHostDispatchRangeV1,
) -> Result<(), M1CompletionCanaryErrorV1> {
    validate_coordinates(
        binding.layout,
        binding.snapshot_range.offset_bytes(),
        binding.snapshot_range.extent_bytes(),
        interior_range.offset_bytes(),
        interior_range.extent_bytes(),
    )
}

fn validate_coordinates(
    layout: M1CompletionCanaryLayoutV1,
    expected_snapshot_offset: u64,
    expected_snapshot_extent: u64,
    interior_offset: u64,
    interior_extent: u64,
) -> Result<(), M1CompletionCanaryErrorV1> {
    if expected_snapshot_extent != layout.snapshot_extent_bytes {
        return Err(M1CompletionCanaryErrorV1::SnapshotExtentDrift {
            expected: layout.snapshot_extent_bytes,
            actual: expected_snapshot_extent,
        });
    }
    if interior_extent != layout.interior_extent_bytes {
        return Err(M1CompletionCanaryErrorV1::InteriorExtentDrift {
            expected: layout.interior_extent_bytes,
            actual: interior_extent,
        });
    }
    let expected_interior_offset = expected_snapshot_offset
        .checked_add(layout.interior_offset_bytes())
        .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
    if interior_offset != expected_interior_offset {
        return Err(M1CompletionCanaryErrorV1::InteriorOffsetDrift {
            expected: expected_interior_offset,
            actual: interior_offset,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_readback_facts(
    layout: M1CompletionCanaryLayoutV1,
    expected_snapshot_offset: u64,
    expected_snapshot_extent: u64,
    interior_offset: u64,
    interior_extent: u64,
    dispatch_generation: u64,
    data_index: usize,
    readback_offset: u64,
    bytes: &[u8],
) -> Result<(usize, usize, M1ObservedCompletionCanarySummaryV1), M1CompletionCanaryErrorV1> {
    if dispatch_generation == 0 {
        return Err(M1CompletionCanaryErrorV1::ZeroDispatchGeneration);
    }
    validate_coordinates(
        layout,
        expected_snapshot_offset,
        expected_snapshot_extent,
        interior_offset,
        interior_extent,
    )?;
    let expected_interior_offset = expected_snapshot_offset
        .checked_add(layout.interior_offset_bytes())
        .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
    if readback_offset != expected_snapshot_offset {
        return Err(M1CompletionCanaryErrorV1::SnapshotOffsetDrift {
            expected: expected_snapshot_offset,
            actual: readback_offset,
        });
    }
    let actual_extent = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if actual_extent != expected_snapshot_extent {
        return Err(M1CompletionCanaryErrorV1::ReadbackExtentDrift {
            expected: expected_snapshot_extent,
            actual: actual_extent,
        });
    }

    let interior_start = usize::try_from(layout.interior_offset_bytes())
        .map_err(|_| M1CompletionCanaryErrorV1::ExtentOverflow)?;
    let interior_extent = usize::try_from(layout.interior_extent_bytes)
        .map_err(|_| M1CompletionCanaryErrorV1::ExtentOverflow)?;
    let interior_end = interior_start
        .checked_add(interior_extent)
        .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
    let prefix = bytes
        .get(..interior_start)
        .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
    let interior = bytes
        .get(interior_start..interior_end)
        .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
    let suffix = bytes
        .get(interior_end..)
        .ok_or(M1CompletionCanaryErrorV1::ExtentOverflow)?;
    if let Some((offset, actual)) = prefix
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| *byte != M1_COMPLETION_CANARY_PREFIX_BYTE_V1)
    {
        return Err(M1CompletionCanaryErrorV1::PrefixGuardMismatch { offset, actual });
    }
    if let Some((offset, actual)) = suffix
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| *byte != M1_COMPLETION_CANARY_SUFFIX_BYTE_V1)
    {
        return Err(M1CompletionCanaryErrorV1::SuffixGuardMismatch { offset, actual });
    }

    let prefix_sha256: [u8; 32] = Sha256::digest(prefix).into();
    let snapshot_sha256: [u8; 32] = Sha256::digest(bytes).into();
    let interior_sha256: [u8; 32] = Sha256::digest(interior).into();
    let suffix_sha256: [u8; 32] = Sha256::digest(suffix).into();
    Ok((
        interior_start,
        interior_end,
        M1ObservedCompletionCanarySummaryV1 {
            dispatch_generation,
            data_index,
            snapshot_offset_bytes: expected_snapshot_offset,
            snapshot_extent_bytes: expected_snapshot_extent,
            interior_offset_bytes: expected_interior_offset,
            interior_extent_bytes: layout.interior_extent_bytes,
            prefix_sha256,
            snapshot_sha256,
            interior_sha256,
            suffix_sha256,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};

    fn layout() -> M1CompletionCanaryLayoutV1 {
        M1CompletionCanaryLayoutV1::for_shape(
            crate::m1_completion_output_shape_v1(Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Prefill,
                bucket: Qwen3PlanBucket::PrefillS1T128,
            })
            .unwrap(),
        )
        .unwrap()
    }

    fn validate(
        layout: M1CompletionCanaryLayoutV1,
        snapshot_offset: u64,
        snapshot_extent: u64,
        interior_offset: u64,
        interior_extent: u64,
        readback_offset: u64,
        bytes: &[u8],
    ) -> Result<(usize, usize, M1ObservedCompletionCanarySummaryV1), M1CompletionCanaryErrorV1>
    {
        validate_readback_facts(
            layout,
            snapshot_offset,
            snapshot_extent,
            interior_offset,
            interior_extent,
            17,
            3,
            readback_offset,
            bytes,
        )
    }

    #[test]
    fn exact_layout_initializes_distinct_guards_and_zero_interior() {
        let layout = layout();
        let bytes = layout.initialized_bytes().unwrap();
        assert_eq!(layout.interior_extent_bytes(), 120);
        assert_eq!(layout.snapshot_extent_bytes(), 248);
        assert!(bytes[..64]
            .iter()
            .all(|byte| *byte == M1_COMPLETION_CANARY_PREFIX_BYTE_V1));
        assert!(bytes[64..184].iter().all(|byte| *byte == 0));
        assert!(bytes[184..]
            .iter()
            .all(|byte| *byte == M1_COMPLETION_CANARY_SUFFIX_BYTE_V1));
    }

    #[test]
    fn exact_guards_expose_only_the_k7_interior() {
        let layout = layout();
        let mut bytes = layout.initialized_bytes().unwrap();
        bytes[64] = 9;
        bytes[183] = 7;
        let (start, end, summary) = validate(layout, 32, 248, 96, 120, 32, &bytes).unwrap();
        assert_eq!((start, end), (64, 184));
        assert_eq!(&bytes[start..end][..1], &[9]);
        assert_eq!(summary.dispatch_generation(), 17);
        assert_eq!(summary.data_index(), 3);
        assert_eq!(summary.snapshot_offset_bytes(), 32);
        assert_eq!(summary.interior_offset_bytes(), 96);
        assert_eq!(summary.interior_extent_bytes(), 120);
    }

    #[test]
    fn shifted_short_and_corrupt_guards_fail_closed() {
        let layout = layout();
        let bytes = layout.initialized_bytes().unwrap();
        assert!(matches!(
            validate(layout, 32, 247, 96, 120, 32, &bytes),
            Err(M1CompletionCanaryErrorV1::SnapshotExtentDrift { .. })
        ));
        assert!(matches!(
            validate(layout, 32, 248, 95, 120, 32, &bytes),
            Err(M1CompletionCanaryErrorV1::InteriorOffsetDrift { .. })
        ));
        assert!(matches!(
            validate(layout, 32, 248, 96, 119, 32, &bytes),
            Err(M1CompletionCanaryErrorV1::InteriorExtentDrift { .. })
        ));
        assert!(matches!(
            validate(layout, 32, 248, 96, 120, 31, &bytes),
            Err(M1CompletionCanaryErrorV1::SnapshotOffsetDrift { .. })
        ));
        assert!(matches!(
            validate(layout, 32, 248, 96, 120, 32, &bytes[..247]),
            Err(M1CompletionCanaryErrorV1::ReadbackExtentDrift { .. })
        ));

        let mut prefix = bytes.clone();
        prefix[63] = 0;
        assert_eq!(
            validate(layout, 32, 248, 96, 120, 32, &prefix).unwrap_err(),
            M1CompletionCanaryErrorV1::PrefixGuardMismatch {
                offset: 63,
                actual: 0
            }
        );
        let mut suffix = bytes;
        suffix[184] = 0;
        assert_eq!(
            validate(layout, 32, 248, 96, 120, 32, &suffix).unwrap_err(),
            M1CompletionCanaryErrorV1::SuffixGuardMismatch {
                offset: 0,
                actual: 0
            }
        );
    }

    #[test]
    fn zero_dispatch_generation_fails_closed() {
        let layout = layout();
        let bytes = layout.initialized_bytes().unwrap();
        assert_eq!(
            validate_readback_facts(layout, 32, 248, 96, 120, 0, 3, 32, &bytes).unwrap_err(),
            M1CompletionCanaryErrorV1::ZeroDispatchGeneration
        );
    }

    #[test]
    fn copied_guard_rejection_retains_the_whole_snapshot() {
        let layout = layout();
        let mut bytes = layout.initialized_bytes().unwrap();
        bytes[1] = 0;
        let original_pointer = bytes.as_ptr();
        let rejected = match validate_readback_facts(layout, 32, 248, 96, 120, 17, 3, 32, &bytes) {
            Ok(_) => panic!("corrupt copied guard unexpectedly validated"),
            Err(error) => (error, bytes),
        };
        assert!(matches!(
            rejected.0,
            M1CompletionCanaryErrorV1::PrefixGuardMismatch {
                offset: 1,
                actual: 0
            }
        ));
        assert_eq!(rejected.1.len(), 248);
        assert_eq!(rejected.1.as_ptr(), original_pointer);
        assert_eq!(rejected.1[1], 0);
        assert_eq!(rejected.1[64..184].len(), 120);
    }
}
