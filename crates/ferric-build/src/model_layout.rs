//! Typed, addressless indexing of admitted Qwen3 weight sections.
//!
//! The index consumes and retains an authenticated bundle admission. It only
//! resolves immutable manifest sections; it does not load bytes or confer
//! allocation, device-address, kernel, execution, or qualification authority.

use crate::{
    AuthenticatedBundleAdmission, SafetensorsError, WeightSection, WeightSectionManifest,
    WeightTransform,
};
use ferric_spec::{Qwen3ModelRole, Qwen3TensorKind, Qwen3TensorMetadata};
use std::fmt;

const UNMAPPED_SECTION: usize = usize::MAX;

/// A non-clone authority for complete typed access to both retained manifests.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthenticatedModelWeightLayout {
    admission: AuthenticatedBundleAdmission,
    target: RoleWeightIndex,
    draft: RoleWeightIndex,
}

/// One borrowed, typed section resolved from an authenticated model layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelWeightBinding<'a> {
    section: &'a WeightSection,
    metadata: Qwen3TensorMetadata,
    ordinal: u32,
}

impl ModelWeightBinding<'_> {
    /// Returns the exact section retained by the authenticated admission.
    #[must_use]
    pub const fn section(&self) -> &WeightSection {
        self.section
    }

    /// Returns the schema-checked role, tensor kind, layer, dtype, and shape.
    #[must_use]
    pub const fn metadata(&self) -> Qwen3TensorMetadata {
        self.metadata
    }

    /// Returns the canonical role-local tensor ordinal.
    #[must_use]
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    /// Returns the retained destination offset and length.
    #[must_use]
    pub const fn destination_range(&self) -> (u64, u64) {
        self.section.destination_range()
    }

    /// Returns the retained digest of the exact emitted section bytes.
    #[must_use]
    pub const fn sha256(&self) -> [u8; 32] {
        self.section.sha256()
    }
}

/// Fail-closed authenticated model-layout construction or lookup error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelWeightLayoutError {
    /// The manifest is retained in the wrong role position.
    ManifestRole {
        /// Role required by the layout position.
        expected: Qwen3ModelRole,
        /// Role carried by the retained manifest.
        actual: Qwen3ModelRole,
    },
    /// The role roster is incomplete or contains extra sections.
    SectionCount {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Exact canonical section count.
        expected: u32,
        /// Observed section count, saturated at `u32::MAX`.
        actual: u32,
    },
    /// A section carries a role other than its containing manifest.
    SectionRole {
        /// Expected manifest role.
        expected: Qwen3ModelRole,
        /// Section role that was rejected.
        actual: Qwen3ModelRole,
        /// Source-order manifest section index.
        section: usize,
    },
    /// A retained tensor name or shape is outside the canonical Qwen3 schema.
    TensorSchema {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Source-order manifest section index.
        section: usize,
        /// Exact classifier or schema failure.
        error: SafetensorsError,
    },
    /// A classified ordinal exceeds the exact role-local map bound.
    OrdinalOutOfRange {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Rejected ordinal.
        ordinal: u32,
        /// Source-order manifest section index.
        section: usize,
    },
    /// Two retained sections resolve to the same canonical ordinal.
    DuplicateOrdinal {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Repeated ordinal.
        ordinal: u32,
        /// First source-order section with this ordinal.
        first_section: usize,
        /// Later source-order section with this ordinal.
        duplicate_section: usize,
    },
    /// A canonical role-local ordinal has no retained section.
    MissingOrdinal {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Missing ordinal.
        ordinal: u32,
    },
    /// A published map slot no longer resolves to its canonical ordinal.
    OrdinalDrift {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Ordinal required by the map slot.
        expected: u32,
        /// Ordinal produced by the canonical classifier.
        actual: u32,
        /// Source-order manifest section index.
        section: usize,
    },
    /// A section does not continue exact, gap-free destination coverage.
    DestinationCoverage {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Source-order manifest section index.
        section: usize,
        /// Required destination offset.
        expected: u64,
        /// Retained destination offset.
        actual: u64,
    },
    /// A retained destination section differs from its source or schema length.
    SectionLength {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Source-order manifest section index.
        section: usize,
    },
    /// Destination arithmetic overflowed before a complete map was published.
    DestinationOverflow {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Source-order manifest section index.
        section: usize,
    },
    /// Final destination coverage differs from the retained manifest bound.
    OutputCoverage {
        /// Model role being indexed.
        role: Qwen3ModelRole,
        /// Required exact output bytes.
        expected: u64,
        /// Bytes covered by retained sections.
        actual: u64,
    },
    /// A typed coordinate is not a member of the selected Qwen3 role schema.
    UnknownCoordinate {
        /// Selected model role.
        role: Qwen3ModelRole,
        /// Requested tensor kind.
        kind: Qwen3TensorKind,
        /// Requested layer or [`ferric_spec::QWEN3_NO_LAYER`].
        layer: u32,
    },
}

impl fmt::Display for ModelWeightLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "authenticated model weight layout rejected: {self:?}"
        )
    }
}

impl std::error::Error for ModelWeightLayoutError {}

#[derive(Debug, PartialEq, Eq)]
struct RoleWeightIndex {
    section_by_ordinal: Box<[usize]>,
}

impl AuthenticatedModelWeightLayout {
    /// Returns the exact authenticated admission retained by this authority.
    #[must_use]
    pub const fn admission(&self) -> &AuthenticatedBundleAdmission {
        &self.admission
    }

    /// Returns the exact canonical number of indexed sections for `role`.
    #[must_use]
    pub const fn section_count(&self, role: Qwen3ModelRole) -> u32 {
        role.tensor_count()
    }

    /// Resolves one exact role-local ordinal to its retained manifest section.
    ///
    /// # Errors
    ///
    /// Returns [`ModelWeightLayoutError`] if `ordinal` is outside the exact
    /// role bound or if the retained classifier result no longer matches its
    /// immutable map slot.
    pub fn by_ordinal(
        &self,
        role: Qwen3ModelRole,
        ordinal: u32,
    ) -> Result<ModelWeightBinding<'_>, ModelWeightLayoutError> {
        let index = self.index(role);
        let ordinal_index =
            usize::try_from(ordinal).map_err(|_| ModelWeightLayoutError::OrdinalOutOfRange {
                role,
                ordinal,
                section: UNMAPPED_SECTION,
            })?;
        let Some(&section_index) = index.section_by_ordinal.get(ordinal_index) else {
            return Err(ModelWeightLayoutError::OrdinalOutOfRange {
                role,
                ordinal,
                section: UNMAPPED_SECTION,
            });
        };
        let section = self
            .manifest(role)
            .sections()
            .get(section_index)
            .ok_or(ModelWeightLayoutError::MissingOrdinal { role, ordinal })?;
        binding_for_slot(role, ordinal, section_index, section)
    }

    /// Resolves an exact typed Qwen3 coordinate to its retained section.
    ///
    /// Global tensors must use [`ferric_spec::QWEN3_NO_LAYER`]. Per-layer tensors must use
    /// an in-range layer for the selected role.
    ///
    /// # Errors
    ///
    /// Returns [`ModelWeightLayoutError`] if the coordinate is outside the
    /// canonical role schema or an immutable map consistency check fails.
    pub fn lookup(
        &self,
        role: Qwen3ModelRole,
        kind: Qwen3TensorKind,
        layer: u32,
    ) -> Result<ModelWeightBinding<'_>, ModelWeightLayoutError> {
        for ordinal in 0..self.section_count(role) {
            let binding = self.by_ordinal(role, ordinal)?;
            let metadata = binding.metadata();
            if metadata.kind == kind && metadata.layer == layer {
                return Ok(binding);
            }
        }
        Err(ModelWeightLayoutError::UnknownCoordinate { role, kind, layer })
    }

    const fn index(&self, role: Qwen3ModelRole) -> &RoleWeightIndex {
        match role {
            Qwen3ModelRole::Target8B => &self.target,
            Qwen3ModelRole::Draft06B => &self.draft,
        }
    }

    const fn manifest(&self, role: Qwen3ModelRole) -> &WeightSectionManifest {
        match role {
            Qwen3ModelRole::Target8B => self.admission.prepacked().target_manifest(),
            Qwen3ModelRole::Draft06B => self.admission.prepacked().draft_manifest(),
        }
    }
}

/// Consumes an authenticated bundle admission and constructs complete typed
/// ordinal maps for the exact target and draft manifests.
///
/// This preserves the admitted bundle record and manifest identities exactly.
/// It does not independently authenticate source or prepacked bytes and grants
/// no storage, device, kernel, dispatch, inference, or qualification authority.
///
/// # Errors
///
/// Returns [`ModelWeightLayoutError`] unless both retained manifests are
/// role-correct, schema-complete, ordinal-complete, and destination-contiguous.
pub fn build_authenticated_model_weight_layout(
    admission: AuthenticatedBundleAdmission,
) -> Result<AuthenticatedModelWeightLayout, ModelWeightLayoutError> {
    let target = build_role_index(
        admission.prepacked().target_manifest(),
        Qwen3ModelRole::Target8B,
    )?;
    let draft = build_role_index(
        admission.prepacked().draft_manifest(),
        Qwen3ModelRole::Draft06B,
    )?;
    Ok(AuthenticatedModelWeightLayout {
        admission,
        target,
        draft,
    })
}

fn build_role_index(
    manifest: &WeightSectionManifest,
    role: Qwen3ModelRole,
) -> Result<RoleWeightIndex, ModelWeightLayoutError> {
    if manifest.role() != role {
        return Err(ModelWeightLayoutError::ManifestRole {
            expected: role,
            actual: manifest.role(),
        });
    }
    let expected_count = role.tensor_count();
    let actual_count = u32::try_from(manifest.sections().len()).unwrap_or(u32::MAX);
    if actual_count != expected_count {
        return Err(ModelWeightLayoutError::SectionCount {
            role,
            expected: expected_count,
            actual: actual_count,
        });
    }
    if manifest.output_bytes() != role.tensor_data_bytes() {
        return Err(ModelWeightLayoutError::OutputCoverage {
            role,
            expected: role.tensor_data_bytes(),
            actual: manifest.output_bytes(),
        });
    }

    let mut section_by_ordinal = vec![UNMAPPED_SECTION; expected_count as usize];
    let mut expected_destination = 0_u64;
    for (section_index, section) in manifest.sections().iter().enumerate() {
        if section.role() != role {
            return Err(ModelWeightLayoutError::SectionRole {
                expected: role,
                actual: section.role(),
                section: section_index,
            });
        }
        let (metadata, ordinal) =
            section
                .qwen3_metadata()
                .map_err(|error| ModelWeightLayoutError::TensorSchema {
                    role,
                    section: section_index,
                    error,
                })?;
        let ordinal_index =
            usize::try_from(ordinal).map_err(|_| ModelWeightLayoutError::OrdinalOutOfRange {
                role,
                ordinal,
                section: section_index,
            })?;
        let Some(slot) = section_by_ordinal.get_mut(ordinal_index) else {
            return Err(ModelWeightLayoutError::OrdinalOutOfRange {
                role,
                ordinal,
                section: section_index,
            });
        };
        if *slot != UNMAPPED_SECTION {
            return Err(ModelWeightLayoutError::DuplicateOrdinal {
                role,
                ordinal,
                first_section: *slot,
                duplicate_section: section_index,
            });
        }

        let (destination_offset, destination_length) = section.destination_range();
        if destination_offset != expected_destination {
            return Err(ModelWeightLayoutError::DestinationCoverage {
                role,
                section: section_index,
                expected: expected_destination,
                actual: destination_offset,
            });
        }
        let expected_length =
            tensor_bytes(metadata).ok_or(ModelWeightLayoutError::SectionLength {
                role,
                section: section_index,
            })?;
        if destination_length != section.source_range().1
            || destination_length != expected_length
            || destination_length == 0
            || destination_length % 2 != 0
            || destination_offset % 2 != 0
            || section.alignment() != 2
            || section.transform() != WeightTransform::Bf16RowMajorIdentityV1
        {
            return Err(ModelWeightLayoutError::SectionLength {
                role,
                section: section_index,
            });
        }
        expected_destination = expected_destination.checked_add(destination_length).ok_or(
            ModelWeightLayoutError::DestinationOverflow {
                role,
                section: section_index,
            },
        )?;
        *slot = section_index;
    }

    if expected_destination != manifest.output_bytes() {
        return Err(ModelWeightLayoutError::OutputCoverage {
            role,
            expected: manifest.output_bytes(),
            actual: expected_destination,
        });
    }
    validate_slot_ordinals(manifest, role, &section_by_ordinal)?;
    Ok(RoleWeightIndex {
        section_by_ordinal: section_by_ordinal.into_boxed_slice(),
    })
}

fn validate_slot_ordinals(
    manifest: &WeightSectionManifest,
    role: Qwen3ModelRole,
    section_by_ordinal: &[usize],
) -> Result<(), ModelWeightLayoutError> {
    for (expected_index, &section_index) in section_by_ordinal.iter().enumerate() {
        let expected = u32::try_from(expected_index).expect("Qwen3 ordinal fits u32");
        if section_index == UNMAPPED_SECTION {
            return Err(ModelWeightLayoutError::MissingOrdinal {
                role,
                ordinal: expected,
            });
        }
        let section = manifest.sections().get(section_index).ok_or(
            ModelWeightLayoutError::MissingOrdinal {
                role,
                ordinal: expected,
            },
        )?;
        let (_, actual) =
            section
                .qwen3_metadata()
                .map_err(|error| ModelWeightLayoutError::TensorSchema {
                    role,
                    section: section_index,
                    error,
                })?;
        if actual != expected {
            return Err(ModelWeightLayoutError::OrdinalDrift {
                role,
                expected,
                actual,
                section: section_index,
            });
        }
    }
    Ok(())
}

fn binding_for_slot(
    role: Qwen3ModelRole,
    expected: u32,
    section_index: usize,
    section: &WeightSection,
) -> Result<ModelWeightBinding<'_>, ModelWeightLayoutError> {
    let (metadata, actual) =
        section
            .qwen3_metadata()
            .map_err(|error| ModelWeightLayoutError::TensorSchema {
                role,
                section: section_index,
                error,
            })?;
    if actual != expected {
        return Err(ModelWeightLayoutError::OrdinalDrift {
            role,
            expected,
            actual,
            section: section_index,
        });
    }
    Ok(ModelWeightBinding {
        section,
        metadata,
        ordinal: expected,
    })
}

fn tensor_bytes(metadata: Qwen3TensorMetadata) -> Option<u64> {
    u64::from(metadata.dimension_0)
        .checked_mul(u64::from(metadata.dimension_1))?
        .checked_mul(2)
}

#[cfg(test)]
mod tests {
    use super::{
        build_authenticated_model_weight_layout, build_role_index, validate_slot_ordinals,
        ModelWeightLayoutError, UNMAPPED_SECTION,
    };
    use crate::{
        seal_authenticated_bundle,
        tokenizer::tests::{authenticated_assets, test_tokenizer},
        weight_stream::tests::test_prepacked,
    };
    use ferric_spec::{
        Qwen3ModelRole, Qwen3TensorKind, QWEN3_DRAFT_TENSOR_COUNT, QWEN3_NO_LAYER,
        QWEN3_TARGET_TENSOR_COUNT,
    };

    fn admission() -> crate::AuthenticatedBundleAdmission {
        let prepacked = crate::build_prepacked_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_prepacked(Qwen3ModelRole::Target8B),
            test_prepacked(Qwen3ModelRole::Draft06B),
        )
        .expect("official manifest fixtures build a complete deployment");
        seal_authenticated_bundle(prepacked).expect("official fixtures seal")
    }

    #[test]
    fn complete_official_rosters_resolve_every_ordinal_to_retained_sections() {
        let authority = admission();
        let record_id = authority.record().record_id();
        let target_manifest_id = authority.prepacked().target_manifest().aggregate_id();
        let draft_manifest_id = authority.prepacked().draft_manifest().aggregate_id();
        let layout = build_authenticated_model_weight_layout(authority)
            .expect("official admitted manifests index exactly");

        assert_eq!(layout.section_count(Qwen3ModelRole::Target8B), 399);
        assert_eq!(layout.section_count(Qwen3ModelRole::Draft06B), 311);
        assert_eq!(layout.admission().record().record_id(), record_id);
        assert_eq!(
            layout
                .admission()
                .prepacked()
                .target_manifest()
                .aggregate_id(),
            target_manifest_id
        );
        assert_eq!(
            layout
                .admission()
                .prepacked()
                .draft_manifest()
                .aggregate_id(),
            draft_manifest_id
        );

        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            let manifest = match role {
                Qwen3ModelRole::Target8B => layout.admission().prepacked().target_manifest(),
                Qwen3ModelRole::Draft06B => layout.admission().prepacked().draft_manifest(),
            };
            for ordinal in 0..role.tensor_count() {
                let binding = layout.by_ordinal(role, ordinal).expect("complete ordinal");
                assert_eq!(binding.ordinal(), ordinal);
                assert_eq!(binding.metadata().role, role);
                assert_eq!(
                    binding.section().qwen3_metadata().expect("retained schema"),
                    (binding.metadata(), ordinal)
                );
                assert_eq!(
                    binding.destination_range(),
                    binding.section().destination_range()
                );
                assert_eq!(binding.sha256(), binding.section().sha256());
                assert!(
                    manifest
                        .sections()
                        .iter()
                        .any(|section| std::ptr::eq(section, binding.section())),
                    "binding must borrow the retained manifest section"
                );
            }
        }
    }

    #[test]
    fn typed_coordinate_lookup_is_role_layer_and_global_exact() {
        let layout = build_authenticated_model_weight_layout(admission()).expect("exact layout");
        let target_embedding = layout
            .lookup(
                Qwen3ModelRole::Target8B,
                Qwen3TensorKind::TokenEmbedding,
                QWEN3_NO_LAYER,
            )
            .expect("target embedding");
        assert_eq!(target_embedding.ordinal(), 0);
        assert_eq!(
            target_embedding.section().tensor_name(),
            "model.embed_tokens.weight"
        );

        let draft_query = layout
            .lookup(
                Qwen3ModelRole::Draft06B,
                Qwen3TensorKind::QueryProjection,
                27,
            )
            .expect("last draft query projection");
        assert_eq!(draft_query.ordinal(), 2 + 27 * 11 + 9);
        assert_eq!(
            draft_query.section().tensor_name(),
            "model.layers.27.self_attn.q_proj.weight"
        );

        assert!(matches!(
            layout.lookup(
                Qwen3ModelRole::Draft06B,
                Qwen3TensorKind::QueryProjection,
                28,
            ),
            Err(ModelWeightLayoutError::UnknownCoordinate { .. })
        ));
        assert!(matches!(
            layout.lookup(Qwen3ModelRole::Target8B, Qwen3TensorKind::FinalNorm, 0,),
            Err(ModelWeightLayoutError::UnknownCoordinate { .. })
        ));
        assert!(matches!(
            layout.by_ordinal(Qwen3ModelRole::Target8B, QWEN3_TARGET_TENSOR_COUNT),
            Err(ModelWeightLayoutError::OrdinalOutOfRange { .. })
        ));
        assert!(matches!(
            layout.by_ordinal(Qwen3ModelRole::Draft06B, QWEN3_DRAFT_TENSOR_COUNT),
            Err(ModelWeightLayoutError::OrdinalOutOfRange { .. })
        ));
    }

    #[test]
    fn incomplete_role_schema_and_destination_mutations_fail_closed() {
        let (_, mut missing) = test_prepacked(Qwen3ModelRole::Draft06B).into_parts();
        missing.test_sections_mut().pop();
        assert!(matches!(
            build_role_index(&missing, Qwen3ModelRole::Draft06B),
            Err(ModelWeightLayoutError::SectionCount { .. })
        ));

        let (_, mut wrong_role) = test_prepacked(Qwen3ModelRole::Draft06B).into_parts();
        wrong_role.test_sections_mut()[0].test_set_role(Qwen3ModelRole::Target8B);
        assert!(matches!(
            build_role_index(&wrong_role, Qwen3ModelRole::Draft06B),
            Err(ModelWeightLayoutError::SectionRole { .. })
        ));

        let (_, mut wrong_schema) = test_prepacked(Qwen3ModelRole::Draft06B).into_parts();
        wrong_schema.test_sections_mut()[0].test_increment_dimension_0();
        assert!(matches!(
            build_role_index(&wrong_schema, Qwen3ModelRole::Draft06B),
            Err(ModelWeightLayoutError::TensorSchema { .. })
        ));

        let (_, mut gap) = test_prepacked(Qwen3ModelRole::Draft06B).into_parts();
        gap.test_sections_mut()[1].test_increment_destination_offset();
        assert!(matches!(
            build_role_index(&gap, Qwen3ModelRole::Draft06B),
            Err(ModelWeightLayoutError::DestinationCoverage { .. })
        ));

        let (_, mut wrong_length) = test_prepacked(Qwen3ModelRole::Draft06B).into_parts();
        wrong_length.test_sections_mut()[0].test_decrement_destination_length();
        assert!(matches!(
            build_role_index(&wrong_length, Qwen3ModelRole::Draft06B),
            Err(ModelWeightLayoutError::SectionLength { .. })
        ));
    }

    #[test]
    fn duplicate_missing_and_ordinal_map_drift_are_rejected() {
        let (_, mut duplicate) = test_prepacked(Qwen3ModelRole::Draft06B).into_parts();
        let sections = duplicate.test_sections_mut();
        let first = sections
            .iter()
            .position(|section| section.tensor_name() == "model.layers.0.input_layernorm.weight")
            .expect("layer zero input norm");
        let second = sections
            .iter()
            .position(|section| section.tensor_name() == "model.layers.1.input_layernorm.weight")
            .expect("layer one input norm");
        sections[second].test_set_tensor_name("model.layers.0.input_layernorm.weight");
        assert!(matches!(
            build_role_index(&duplicate, Qwen3ModelRole::Draft06B),
            Err(ModelWeightLayoutError::DuplicateOrdinal {
                first_section,
                duplicate_section,
                ..
            }) if first_section == first && duplicate_section == second
        ));

        let (_, manifest) = test_prepacked(Qwen3ModelRole::Draft06B).into_parts();
        let mut missing_slots = vec![UNMAPPED_SECTION; QWEN3_DRAFT_TENSOR_COUNT as usize];
        assert_eq!(
            validate_slot_ordinals(&manifest, Qwen3ModelRole::Draft06B, &missing_slots),
            Err(ModelWeightLayoutError::MissingOrdinal {
                role: Qwen3ModelRole::Draft06B,
                ordinal: 0,
            })
        );

        let index = build_role_index(&manifest, Qwen3ModelRole::Draft06B)
            .expect("official draft role index");
        missing_slots.copy_from_slice(&index.section_by_ordinal);
        missing_slots.swap(0, 1);
        assert!(matches!(
            validate_slot_ordinals(&manifest, Qwen3ModelRole::Draft06B, &missing_slots),
            Err(ModelWeightLayoutError::OrdinalDrift {
                expected: 0,
                actual: 1,
                ..
            })
        ));
    }
}
