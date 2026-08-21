use super::{
    bundle_identity, encode_u32_big_endian, encode_u64_big_endian, model_identity,
    DRAFT_CONFIG_SHA256, DRAFT_REPOSITORY, DRAFT_REVISION, QWEN3_DRAFT_MODEL_ID,
    QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256,
    QWEN3_TARGET_MODEL_ID, QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
    QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_SHA256, TARGET_CONFIG_SHA256,
    TARGET_REPOSITORY, TARGET_REVISION, TOKENIZER_METADATA_SHA256,
};
use ferric_spec::{
    DeploymentBundle, EngineLimits, Identity, ModelArtifact, ModelConfig, NumericalPolicy,
    Qwen3ModelRole, SpecError, Target, TokenizerConfig, WeightManifest, QWEN3_END_OF_TEXT_TOKEN,
    QWEN3_IM_END_TOKEN, QWEN3_IM_START_TOKEN, QWEN3_VOCABULARY_SIZE,
};
use std::fmt;
use vstd::prelude::*;
#[allow(unused_imports)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

const MAGIC: [u8; 16] = [70, 69, 82, 82, 73, 67, 45, 77, 49, 45, 66, 85, 78, 68, 76, 69];
const TARGET_GFX942_XNACK_MINUS: u8 = 1;
const NUMERICAL_BF16_FP32: u8 = 1;
const ROLE_TARGET_8B: u8 = 1;
const ROLE_DRAFT_06B: u8 = 2;

/// Version of the fixed-width canonical M1 deployment-bundle record.
pub const CANONICAL_DEPLOYMENT_BUNDLE_VERSION: u32 = 1;
/// Exact byte length of a canonical M1 deployment-bundle record.
pub const CANONICAL_DEPLOYMENT_BUNDLE_BYTES: usize = 522;

closed spec fn byte_wire(value: u8) -> Seq<u8> {
    seq![value]
}

closed spec fn boolean_wire(value: bool) -> Seq<u8> {
    seq![if value { 1u8 } else { 0u8 }]
}

closed spec fn fixed_tokenizer_spec() -> TokenizerConfig {
    TokenizerConfig {
        tokenizer_id: Identity::from_bytes_spec(TOKENIZER_METADATA_SHA256@),
        vocabulary_id: Identity::from_bytes_spec(QWEN3_TOKENIZER_SHA256@),
        vocabulary_size: QWEN3_VOCABULARY_SIZE,
        end_of_text_token: QWEN3_END_OF_TEXT_TOKEN,
        im_start_token: QWEN3_IM_START_TOKEN,
        im_end_token: QWEN3_IM_END_TOKEN,
    }
}

closed spec fn fixed_model_spec(role: Qwen3ModelRole) -> ModelArtifact {
    match role {
        Qwen3ModelRole::Target8B => ModelArtifact {
            config: ModelConfig {
                role,
                model_id: Identity::from_bytes_spec(QWEN3_TARGET_MODEL_ID@),
                config_id: Identity::from_bytes_spec(TARGET_CONFIG_SHA256@),
                vocabulary_size: QWEN3_VOCABULARY_SIZE,
                layers: 36,
                hidden_size: 4_096,
                intermediate_size: 12_288,
                query_heads: 32,
                kv_heads: 8,
                head_dim: 128,
                max_position_embeddings: 40_960,
                rope_theta: 1_000_000,
                tie_word_embeddings: false,
            },
            tokenizer: fixed_tokenizer_spec(),
            weights: WeightManifest {
                weights_id: Identity::from_bytes_spec(QWEN3_TARGET_WEIGHT_SET_SHA256@),
                total_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
                sections: 5,
            },
        },
        Qwen3ModelRole::Draft06B => ModelArtifact {
            config: ModelConfig {
                role,
                model_id: Identity::from_bytes_spec(QWEN3_DRAFT_MODEL_ID@),
                config_id: Identity::from_bytes_spec(DRAFT_CONFIG_SHA256@),
                vocabulary_size: QWEN3_VOCABULARY_SIZE,
                layers: 28,
                hidden_size: 1_024,
                intermediate_size: 3_072,
                query_heads: 16,
                kv_heads: 8,
                head_dim: 128,
                max_position_embeddings: 40_960,
                rope_theta: 1_000_000,
                tie_word_embeddings: true,
            },
            tokenizer: fixed_tokenizer_spec(),
            weights: WeightManifest {
                weights_id: Identity::from_bytes_spec(QWEN3_DRAFT_WEIGHT_SHA256@),
                total_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
                sections: 1,
            },
        },
    }
}

closed spec fn model_config_wire(model: ModelArtifact) -> Seq<u8> {
    byte_wire(match model.config.role {
        Qwen3ModelRole::Target8B => ROLE_TARGET_8B,
        Qwen3ModelRole::Draft06B => ROLE_DRAFT_06B,
    })
        + model.config.model_id.bytes_spec()
        + model.config.config_id.bytes_spec()
        + super::u32_big_endian(model.config.vocabulary_size)
        + super::u32_big_endian(model.config.layers)
        + super::u32_big_endian(model.config.hidden_size)
        + super::u32_big_endian(model.config.intermediate_size)
        + super::u32_big_endian(model.config.query_heads)
        + super::u32_big_endian(model.config.kv_heads)
        + super::u32_big_endian(model.config.head_dim)
        + super::u32_big_endian(model.config.max_position_embeddings)
        + super::u32_big_endian(model.config.rope_theta)
        + boolean_wire(model.config.tie_word_embeddings)
}

closed spec fn model_tokenizer_weights_wire(model: ModelArtifact) -> Seq<u8> {
    model.tokenizer.tokenizer_id.bytes_spec()
        + model.tokenizer.vocabulary_id.bytes_spec()
        + super::u32_big_endian(model.tokenizer.vocabulary_size)
        + super::u32_big_endian(model.tokenizer.end_of_text_token)
        + super::u32_big_endian(model.tokenizer.im_start_token)
        + super::u32_big_endian(model.tokenizer.im_end_token)
        + model.weights.weights_id.bytes_spec()
        + super::u64_big_endian(model.weights.total_bytes)
        + super::u32_big_endian(model.weights.sections)
}

closed spec fn model_wire(model: ModelArtifact) -> Seq<u8> {
    model_config_wire(model) + model_tokenizer_weights_wire(model)
}

/// Complete verifier-visible wire image for one production bundle value.
pub closed spec fn canonical_deployment_bundle_wire(bundle: DeploymentBundle) -> Seq<u8> {
    MAGIC@
        + super::u32_big_endian(CANONICAL_DEPLOYMENT_BUNDLE_VERSION)
        + bundle.bundle_id.bytes_spec()
        + byte_wire(TARGET_GFX942_XNACK_MINUS)
        + byte_wire(NUMERICAL_BF16_FP32)
        + super::u32_big_endian(bundle.limits.max_context_tokens)
        + super::u32_big_endian(bundle.limits.max_active_sequences)
        + super::u32_big_endian(bundle.limits.kv_page_tokens)
        + super::u32_big_endian(bundle.limits.max_draft_tokens)
        + model_wire(bundle.target_model)
        + model_wire(bundle.draft_model)
}

closed spec fn model_matches_pins(model: ModelArtifact, role: Qwen3ModelRole) -> bool {
    match role {
        Qwen3ModelRole::Target8B => {
            &&& TARGET_REPOSITORY.spec_bytes().len() <= 16
            &&& TARGET_REVISION.spec_bytes().len() == 40
            &&& model.config.config_id.bytes_spec() == TARGET_CONFIG_SHA256@
            &&& model.config.model_id.bytes_spec() == QWEN3_TARGET_MODEL_ID@
            &&& model.tokenizer.tokenizer_id.bytes_spec() == TOKENIZER_METADATA_SHA256@
            &&& model.tokenizer.vocabulary_id.bytes_spec() == QWEN3_TOKENIZER_SHA256@
            &&& model.weights.weights_id.bytes_spec() == QWEN3_TARGET_WEIGHT_SET_SHA256@
            &&& model.weights.total_bytes == QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES
            &&& model.weights.sections == 5
            &&& model.config.model_id.bytes_spec() == super::sha256::digest_spec(
                super::model_identity_preimage(
                    role,
                    TARGET_REPOSITORY,
                    TARGET_REVISION,
                    model.config.config_id,
                    model.tokenizer,
                    model.weights,
                    QWEN3_TARGET_TENSOR_DATA_BYTES,
                ),
            )
        },
        Qwen3ModelRole::Draft06B => {
            &&& DRAFT_REPOSITORY.spec_bytes().len() <= 16
            &&& DRAFT_REVISION.spec_bytes().len() == 40
            &&& model.config.config_id.bytes_spec() == DRAFT_CONFIG_SHA256@
            &&& model.config.model_id.bytes_spec() == QWEN3_DRAFT_MODEL_ID@
            &&& model.tokenizer.tokenizer_id.bytes_spec() == TOKENIZER_METADATA_SHA256@
            &&& model.tokenizer.vocabulary_id.bytes_spec() == QWEN3_TOKENIZER_SHA256@
            &&& model.weights.weights_id.bytes_spec() == QWEN3_DRAFT_WEIGHT_SHA256@
            &&& model.weights.total_bytes == QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES
            &&& model.weights.sections == 1
            &&& model.config.model_id.bytes_spec() == super::sha256::digest_spec(
                super::model_identity_preimage(
                    role,
                    DRAFT_REPOSITORY,
                    DRAFT_REVISION,
                    model.config.config_id,
                    model.tokenizer,
                    model.weights,
                    QWEN3_DRAFT_TENSOR_DATA_BYTES,
                ),
            )
        },
    }
}

/// Exact executable acceptance relation used by the production encoder.
///
/// The SHA clauses are computation equalities. They do not assert collision
/// resistance, provenance, signature validity, or authentication of files.
pub closed spec fn canonical_deployment_bundle_spec(bundle: DeploymentBundle) -> bool {
    &&& bundle.valid()
    &&& bundle.target == Target::Gfx942XnackMinus
    &&& bundle.numerical_policy == NumericalPolicy::Bf16ParametersFp32Accumulation
    &&& model_matches_pins(bundle.target_model, Qwen3ModelRole::Target8B)
    &&& model_matches_pins(bundle.draft_model, Qwen3ModelRole::Draft06B)
    &&& bundle.bundle_id.bytes_spec() == super::sha256::digest_spec(
        super::bundle_identity_preimage(bundle.limits, bundle.target_model, bundle.draft_model),
    )
}

closed spec fn u32_at(bytes: Seq<u8>, offset: int) -> u32
    recommends 0 <= offset, offset + 4 <= bytes.len(),
{
    ((bytes[offset] as u32) << 24)
        | ((bytes[offset + 1] as u32) << 16)
        | ((bytes[offset + 2] as u32) << 8)
        | (bytes[offset + 3] as u32)
}

/// Exact bundle value retained by the production fixed-width reader.
pub closed spec fn parsed_bundle_spec(bytes: Seq<u8>) -> DeploymentBundle
    recommends bytes.len() == CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
{
    DeploymentBundle {
        bundle_id: Identity::from_bytes_spec(bytes.subrange(20, 52)),
        target: Target::Gfx942XnackMinus,
        numerical_policy: NumericalPolicy::Bf16ParametersFp32Accumulation,
        limits: EngineLimits {
            max_context_tokens: u32_at(bytes, 54),
            max_active_sequences: u32_at(bytes, 58),
            kv_page_tokens: u32_at(bytes, 62),
            max_draft_tokens: u32_at(bytes, 66),
        },
        target_model: fixed_model_spec(Qwen3ModelRole::Target8B),
        draft_model: fixed_model_spec(Qwen3ModelRole::Draft06B),
    }
}

closed spec fn canonical_deployment_bundle_syntax(bytes: Seq<u8>) -> bool {
    if bytes.len() != CANONICAL_DEPLOYMENT_BUNDLE_BYTES {
        false
    } else {
        bytes.subrange(0, 16) == MAGIC@
            && u32_at(bytes, 16) == CANONICAL_DEPLOYMENT_BUNDLE_VERSION
            && bytes[52] == TARGET_GFX942_XNACK_MINUS
            && bytes[53] == NUMERICAL_BF16_FP32
            && bytes[70] == ROLE_TARGET_8B
            && bytes[171] <= 1
            && bytes[296] == ROLE_DRAFT_06B
            && bytes[397] <= 1
    }
}

/// Exact verifier-visible byte acceptance relation for the production decoder.
pub closed spec fn canonical_deployment_bundle_bytes(bytes: Seq<u8>) -> bool {
    canonical_deployment_bundle_syntax(bytes)
        && canonical_deployment_bundle_spec(parsed_bundle_spec(bytes))
        && bytes == canonical_deployment_bundle_wire(parsed_bundle_spec(bytes))
}

/// An accepted record is exactly the canonical re-encoding of every retained field.
pub proof fn accepted_record_reencodes(bytes: Seq<u8>)
    requires canonical_deployment_bundle_bytes(bytes),
    ensures
        bytes.len() == CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
        canonical_deployment_bundle_spec(parsed_bundle_spec(bytes)),
        bytes == canonical_deployment_bundle_wire(parsed_bundle_spec(bytes)),
{
}

/// The retained production bundle uniquely identifies an accepted record.
pub proof fn accepted_record_injective(left: Seq<u8>, right: Seq<u8>)
    requires
        canonical_deployment_bundle_bytes(left),
        canonical_deployment_bundle_bytes(right),
        parsed_bundle_spec(left) == parsed_bundle_spec(right),
    ensures left == right,
{
}

proof fn model_wire_len(model: ModelArtifact)
    ensures model_wire(model).len() == 226,
{
    model.config.model_id.bytes_spec_len();
    model.config.config_id.bytes_spec_len();
    model.tokenizer.tokenizer_id.bytes_spec_len();
    model.tokenizer.vocabulary_id.bytes_spec_len();
    model.weights.weights_id.bytes_spec_len();
}

/// Establishes the exact production record width from its field layout.
pub proof fn canonical_deployment_bundle_wire_len(bundle: DeploymentBundle)
    ensures canonical_deployment_bundle_wire(bundle).len() == CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
{
    bundle.bundle_id.bytes_spec_len();
    model_wire_len(bundle.target_model);
    model_wire_len(bundle.draft_model);
}

#[verifier::bit_vector]
proof fn u32_big_endian_parts_round_trip(value: u32)
    ensures
        ((((value >> 24) % 256) as u8 as u32) << 24)
            | ((((value >> 16) % 256) as u8 as u32) << 16)
            | ((((value >> 8) % 256) as u8 as u32) << 8)
            | (((value % 256) as u8) as u32)
            == value,
{
}

proof fn u32_big_endian_round_trip(value: u32)
    ensures u32_at(super::u32_big_endian(value), 0) == value,
{
    u32_big_endian_parts_round_trip(value);
}

/// A fixed-width canonical record for the exact first M1 model pair.
///
/// This value binds already-admitted identities and geometry. It does not
/// authenticate the external files named by those identities and is not a
/// signature or a device-load authority.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalDeploymentBundle {
    bytes: [u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES],
}

impl CanonicalDeploymentBundle {
    /// Verifier view of the complete fixed-width record.
    pub closed spec fn bytes_spec(&self) -> Seq<u8> {
        self.bytes@
    }

    /// Returns the complete canonical record bytes.
    #[must_use]
    pub fn as_bytes(&self) -> (bytes: &[u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES])
        ensures bytes@ == self.bytes_spec(),
    {
        &self.bytes
    }
}

/// Failure while encoding or decoding the fixed M1 bundle record.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalBundleError {
    /// The record is truncated, has trailing bytes, or exhausted a cursor.
    InvalidLength,
    /// The fixed format discriminator is incorrect.
    InvalidMagic,
    /// The fixed format version is unsupported.
    InvalidVersion,
    /// An enum discriminator is not canonical for M1.
    InvalidTag(&'static str),
    /// A boolean byte is not the canonical zero or one representation.
    InvalidBoolean(&'static str),
    /// A field differs from the exact pinned Qwen3 pair.
    PinnedFieldMismatch(&'static str),
    /// The executable sequential bundle contract rejected the decoded value.
    Spec(SpecError),
}

fn exact_identity(
    actual: Identity,
    expected: [u8; 32],
    field: &'static str,
) -> (result: Result<(), CanonicalBundleError>)
    ensures result.is_ok() == (actual.bytes_spec() == expected@),
{
    if !actual.equals(&Identity::new(expected)) {
        return Err(CanonicalBundleError::PinnedFieldMismatch(field));
    }
    Ok(())
}

fn validate_model(
    model: ModelArtifact,
    role: Qwen3ModelRole,
) -> (result: Result<(), CanonicalBundleError>)
    ensures result.is_ok() == model_matches_pins(model, role),
{
    let (
        config_id, model_id, repository, revision, weight_id, artifact_bytes, tensor_bytes,
        sections,
    ) = match role {
        Qwen3ModelRole::Target8B => (
            TARGET_CONFIG_SHA256,
            QWEN3_TARGET_MODEL_ID,
            TARGET_REPOSITORY,
            TARGET_REVISION,
            QWEN3_TARGET_WEIGHT_SET_SHA256,
            QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
            QWEN3_TARGET_TENSOR_DATA_BYTES,
            5,
        ),
        Qwen3ModelRole::Draft06B => (
            DRAFT_CONFIG_SHA256,
            QWEN3_DRAFT_MODEL_ID,
            DRAFT_REPOSITORY,
            DRAFT_REVISION,
            QWEN3_DRAFT_WEIGHT_SHA256,
            QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
            QWEN3_DRAFT_TENSOR_DATA_BYTES,
            1,
        ),
    };
    exact_identity(model.config.config_id, config_id, "config_id")?;
    exact_identity(model.config.model_id, model_id, "model_id")?;
    exact_identity(model.tokenizer.tokenizer_id, TOKENIZER_METADATA_SHA256, "tokenizer_id")?;
    exact_identity(model.tokenizer.vocabulary_id, QWEN3_TOKENIZER_SHA256, "vocabulary_id")?;
    exact_identity(model.weights.weights_id, weight_id, "weights_id")?;
    if model.weights.total_bytes != artifact_bytes {
        return Err(CanonicalBundleError::PinnedFieldMismatch("weight_bytes"));
    }
    if model.weights.sections != sections {
        return Err(CanonicalBundleError::PinnedFieldMismatch("weight_sections"));
    }
    let repository_bytes = repository.as_bytes();
    let revision_bytes = revision.as_bytes();
    let repository_len = repository_bytes.len();
    let revision_len = revision_bytes.len();
    if repository_len > 16 || revision_len != 40 {
        return Err(CanonicalBundleError::PinnedFieldMismatch("identity_source"));
    }
    proof {
        super::model_identity_preimage_len(
            role,
            repository,
            revision,
            model.config.config_id,
            model.tokenizer,
            model.weights,
            tensor_bytes,
        );
        assert(repository_bytes@ == repository.spec_bytes());
        assert(revision_bytes@ == revision.spec_bytes());
        assert(repository.spec_bytes().len() <= 16);
        assert(revision.spec_bytes().len() == 40);
    }
    let expected_model_id = model_identity(
        role, repository, revision, model.config.config_id, model.tokenizer, model.weights,
        tensor_bytes,
    );
    if !model.config.model_id.equals(&expected_model_id) {
        return Err(CanonicalBundleError::PinnedFieldMismatch("derived_model_id"));
    }
    Ok(())
}

fn validate_exact_bundle(
    bundle: &DeploymentBundle,
) -> (result: Result<(), CanonicalBundleError>)
    ensures result.is_ok() == canonical_deployment_bundle_spec(*bundle),
{
    match bundle.validate() {
        Ok(()) => {}
        Err(error) => return Err(CanonicalBundleError::Spec(error)),
    }
    validate_model(bundle.target_model, Qwen3ModelRole::Target8B)?;
    validate_model(bundle.draft_model, Qwen3ModelRole::Draft06B)?;
    let expected_bundle_id = bundle_identity(bundle.limits, bundle.target_model, bundle.draft_model);
    if !bundle.bundle_id.equals(&expected_bundle_id) {
        return Err(CanonicalBundleError::PinnedFieldMismatch("bundle_id"));
    }
    Ok(())
}

struct Writer {
    bytes: [u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES],
    offset: usize,
}

impl Writer {
    closed spec fn valid(&self) -> bool {
        self.offset <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES
    }

    closed spec fn view(&self) -> Seq<u8>
        recommends self.valid(),
    {
        self.bytes@.subrange(0, self.offset as int)
    }

    fn new() -> (writer: Self)
        ensures writer.valid(), writer.offset == 0, writer.view() == Seq::<u8>::empty(),
    {
        Self { bytes: [0; CANONICAL_DEPLOYMENT_BUNDLE_BYTES], offset: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            result.is_ok() == (old(self).offset + value@.len()
                <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES),
            result.is_ok() ==> {
                &&& final(self).valid()
                &&& final(self).offset == old(self).offset + value@.len()
                &&& final(self).view() == old(self).view() + value@
            },
            result.is_err() ==> {
                &&& final(self).offset == old(self).offset
                &&& final(self).bytes@ == old(self).bytes@
            },
    {
        if value.len() > CANONICAL_DEPLOYMENT_BUNDLE_BYTES - self.offset {
            return Err(CanonicalBundleError::InvalidLength);
        }
        let ghost initial_view = self.view();
        let ghost initial_offset = self.offset;
        let mut index = 0;
        while index < value.len()
            invariant
                self.valid(),
                initial_offset + value@.len() <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
                0 <= index <= value@.len(),
                self.offset == initial_offset + index,
                self.view() == initial_view + value@.subrange(0, index as int),
            decreases value@.len() - index,
        {
            let byte = value[index];
            self.byte_with_capacity(byte);
            index += 1;
            assert(value@.subrange(0, index as int)
                == value@.subrange(0, index as int - 1).push(byte)) by {
                assert(value@.subrange(0, index as int) =~=
                    value@.subrange(0, index as int - 1).push(byte)) by {
                    assert forall|position: int| 0 <= position < index implies
                        value@.subrange(0, index as int)[position]
                            == value@.subrange(0, index as int - 1).push(byte)[position] by {
                        if position < index - 1 {} else {}
                    }
                }
            }
        }
        assert(value@.subrange(0, value@.len() as int) == value@);
        Ok(())
    }

    fn u8(&mut self, value: u8) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            result.is_ok() == (old(self).offset + 1 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES),
            result.is_ok() ==> {
                &&& final(self).valid()
                &&& final(self).offset == old(self).offset + 1
                &&& final(self).view() == old(self).view() + byte_wire(value)
            },
    {
        self.bytes(&[value])
    }

    fn boolean(&mut self, value: bool) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            result.is_ok() == (old(self).offset + 1 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES),
            result.is_ok() ==> {
                &&& final(self).valid()
                &&& final(self).offset == old(self).offset + 1
                &&& final(self).view() == old(self).view() + boolean_wire(value)
            },
    {
        if value {
            self.u8(1)
        } else {
            self.u8(0)
        }
    }

    fn u32(&mut self, value: u32) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            result.is_ok() == (old(self).offset + 4 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES),
            result.is_ok() ==> {
                &&& final(self).valid()
                &&& final(self).offset == old(self).offset + 4
                &&& final(self).view() == old(self).view() + super::u32_big_endian(value)
            },
    {
        let encoded = encode_u32_big_endian(value);
        self.bytes(&encoded)
    }

    fn u64(&mut self, value: u64) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            result.is_ok() == (old(self).offset + 8 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES),
            result.is_ok() ==> {
                &&& final(self).valid()
                &&& final(self).offset == old(self).offset + 8
                &&& final(self).view() == old(self).view() + super::u64_big_endian(value)
            },
    {
        let encoded = encode_u64_big_endian(value);
        self.bytes(&encoded)
    }

    fn identity(&mut self, value: Identity) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            result.is_ok() == (old(self).offset + 32 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES),
            result.is_ok() ==> {
                &&& final(self).valid()
                &&& final(self).offset == old(self).offset + 32
                &&& final(self).view() == old(self).view() + value.bytes_spec()
            },
    {
        proof { value.bytes_spec_len(); }
        self.bytes(value.as_bytes())
    }

    fn byte_with_capacity(&mut self, value: u8)
        requires
            old(self).valid(),
            old(self).offset < CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
        ensures
            final(self).valid(),
            final(self).offset == old(self).offset + 1,
            final(self).view() == old(self).view().push(value),
    {
        let offset = self.offset;
        self.bytes[offset] = value;
        self.offset += 1;
        assert(self.view() =~= old(self).view().push(value)) by {
            assert forall|position: int| 0 <= position < self.view().len() implies
                self.view()[position] == old(self).view().push(value)[position] by {
                if position < offset {
                } else {
                    assert(position == offset);
                }
            }
        }
    }

    fn limits(&mut self, value: EngineLimits) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(), old(self).offset + 16 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
        ensures
            result.is_ok(),
            final(self).valid(),
            final(self).offset == old(self).offset + 16,
            final(self).view() == old(self).view()
                + super::u32_big_endian(value.max_context_tokens)
                + super::u32_big_endian(value.max_active_sequences)
                + super::u32_big_endian(value.kv_page_tokens)
                + super::u32_big_endian(value.max_draft_tokens),
    {
        self.u32(value.max_context_tokens)?;
        self.u32(value.max_active_sequences)?;
        self.u32(value.kv_page_tokens)?;
        self.u32(value.max_draft_tokens)?;
        Ok(())
    }

    fn model_config(&mut self, value: ModelArtifact) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(), old(self).offset + 102 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
        ensures
            result.is_ok(),
            final(self).valid(),
            final(self).offset == old(self).offset + 102,
            final(self).view() == old(self).view() + model_config_wire(value),
    {
        self.u8(match value.config.role {
            Qwen3ModelRole::Target8B => ROLE_TARGET_8B,
            Qwen3ModelRole::Draft06B => ROLE_DRAFT_06B,
        })?;
        self.identity(value.config.model_id)?;
        self.identity(value.config.config_id)?;
        self.u32(value.config.vocabulary_size)?;
        self.u32(value.config.layers)?;
        self.u32(value.config.hidden_size)?;
        self.u32(value.config.intermediate_size)?;
        self.u32(value.config.query_heads)?;
        self.u32(value.config.kv_heads)?;
        self.u32(value.config.head_dim)?;
        self.u32(value.config.max_position_embeddings)?;
        self.u32(value.config.rope_theta)?;
        self.boolean(value.config.tie_word_embeddings)?;
        Ok(())
    }

    fn model_tokenizer_weights(
        &mut self,
        value: ModelArtifact,
    ) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(), old(self).offset + 124 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
        ensures
            result.is_ok(),
            final(self).valid(),
            final(self).offset == old(self).offset + 124,
            final(self).view() == old(self).view() + model_tokenizer_weights_wire(value),
    {
        self.identity(value.tokenizer.tokenizer_id)?;
        self.identity(value.tokenizer.vocabulary_id)?;
        self.u32(value.tokenizer.vocabulary_size)?;
        self.u32(value.tokenizer.end_of_text_token)?;
        self.u32(value.tokenizer.im_start_token)?;
        self.u32(value.tokenizer.im_end_token)?;
        self.identity(value.weights.weights_id)?;
        self.u64(value.weights.total_bytes)?;
        self.u32(value.weights.sections)?;
        Ok(())
    }

    fn model(&mut self, value: ModelArtifact) -> (result: Result<(), CanonicalBundleError>)
        requires old(self).valid(), old(self).offset + 226 <= CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
        ensures
            result.is_ok(),
            final(self).valid(),
            final(self).offset == old(self).offset + 226,
            final(self).view() == old(self).view() + model_wire(value),
    {
        self.model_config(value)?;
        self.model_tokenizer_weights(value)?;
        proof { model_wire_len(value); }
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    closed spec fn valid(&self) -> bool {
        self.offset <= self.bytes@.len()
    }

    fn new(bytes: &'a [u8]) -> (reader: Self)
        ensures reader.valid(), reader.offset == 0, reader.bytes@ == bytes@,
    {
        Self { bytes, offset: 0 }
    }

    fn array<const N: usize>(&mut self) -> (result: Result<[u8; N], CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (N <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + N
                &&& result.get_Ok_0()@ == old(self).bytes@.subrange(
                    old(self).offset as int, (old(self).offset + N) as int,
                )
            },
            result.is_err() ==> final(self).offset == old(self).offset,
    {
        if N > self.bytes.len() - self.offset {
            return Err(CanonicalBundleError::InvalidLength);
        }
        let start = self.offset;
        let mut value = [0; N];
        let mut index = 0;
        while index < N
            invariant
                self.offset == start,
                start + N <= self.bytes@.len(),
                self.bytes@.len() <= usize::MAX as nat,
                0 <= index <= N,
                value@.len() == N,
                forall|prior: int| 0 <= prior < index ==>
                    value@[prior] == self.bytes@[start as int + prior],
            decreases N - index,
        {
            proof {
                assert(start as nat + index as nat <= self.bytes@.len());
                assert(start as nat + index as nat <= usize::MAX as nat);
            }
            let position = match start.checked_add(index) {
                Some(position) => position,
                None => return Err(CanonicalBundleError::InvalidLength),
            };
            value[index] = self.bytes[position];
            index += 1;
        }
        assert(value@ =~= self.bytes@.subrange(start as int, (start + N) as int)) by {
            assert forall|position: int| 0 <= position < N implies
                value@[position] == self.bytes@.subrange(start as int, (start + N) as int)[position]
                by {}
        }
        self.offset += N;
        Ok(value)
    }

    fn u8(&mut self) -> (result: Result<u8, CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (1 <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + 1
                &&& result.get_Ok_0() == old(self).bytes@[old(self).offset as int]
            },
    {
        match self.array::<1>() {
            Ok(value) => Ok(value[0]),
            Err(error) => Err(error),
        }
    }

    fn u32(&mut self) -> (result: Result<u32, CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (4 <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + 4
                &&& result.get_Ok_0() == u32_at(old(self).bytes@, old(self).offset as int)
            },
    {
        match self.array::<4>() {
            Ok(value) => Ok(
                (u32::from(value[0]) << 24)
                    | (u32::from(value[1]) << 16)
                    | (u32::from(value[2]) << 8)
                    | u32::from(value[3]),
            ),
            Err(error) => Err(error),
        }
    }

    fn identity(&mut self) -> (result: Result<Identity, CanonicalBundleError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (32 <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + 32
                &&& result.get_Ok_0().bytes_spec() == old(self).bytes@.subrange(
                    old(self).offset as int, (old(self).offset + 32) as int,
                )
            },
    {
        match self.array::<32>() {
            Ok(value) => Ok(Identity::new(value)),
            Err(error) => Err(error),
        }
    }
}

fn fixed_model(role: Qwen3ModelRole) -> (model: ModelArtifact)
    ensures model == fixed_model_spec(role),
{
    let tokenizer = TokenizerConfig {
        tokenizer_id: Identity::new(TOKENIZER_METADATA_SHA256),
        vocabulary_id: Identity::new(QWEN3_TOKENIZER_SHA256),
        vocabulary_size: QWEN3_VOCABULARY_SIZE,
        end_of_text_token: QWEN3_END_OF_TEXT_TOKEN,
        im_start_token: QWEN3_IM_START_TOKEN,
        im_end_token: QWEN3_IM_END_TOKEN,
    };
    let model = match role {
        Qwen3ModelRole::Target8B => ModelArtifact {
            config: ModelConfig {
                role,
                model_id: Identity::new(QWEN3_TARGET_MODEL_ID),
                config_id: Identity::new(TARGET_CONFIG_SHA256),
                vocabulary_size: QWEN3_VOCABULARY_SIZE,
                layers: 36,
                hidden_size: 4_096,
                intermediate_size: 12_288,
                query_heads: 32,
                kv_heads: 8,
                head_dim: 128,
                max_position_embeddings: 40_960,
                rope_theta: 1_000_000,
                tie_word_embeddings: false,
            },
            tokenizer,
            weights: WeightManifest {
                weights_id: Identity::new(QWEN3_TARGET_WEIGHT_SET_SHA256),
                total_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
                sections: 5,
            },
        },
        Qwen3ModelRole::Draft06B => ModelArtifact {
            config: ModelConfig {
                role,
                model_id: Identity::new(QWEN3_DRAFT_MODEL_ID),
                config_id: Identity::new(DRAFT_CONFIG_SHA256),
                vocabulary_size: QWEN3_VOCABULARY_SIZE,
                layers: 28,
                hidden_size: 1_024,
                intermediate_size: 3_072,
                query_heads: 16,
                kv_heads: 8,
                head_dim: 128,
                max_position_embeddings: 40_960,
                rope_theta: 1_000_000,
                tie_word_embeddings: true,
            },
            tokenizer,
            weights: WeightManifest {
                weights_id: Identity::new(QWEN3_DRAFT_WEIGHT_SHA256),
                total_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
                sections: 1,
            },
        },
    };
    proof {
        Identity::from_bytes_spec_view(TOKENIZER_METADATA_SHA256@);
        Identity::from_bytes_spec_view(QWEN3_TOKENIZER_SHA256@);
        Identity::from_bytes_spec_view(TARGET_CONFIG_SHA256@);
        Identity::from_bytes_spec_view(DRAFT_CONFIG_SHA256@);
        Identity::from_bytes_spec_view(QWEN3_TARGET_MODEL_ID@);
        Identity::from_bytes_spec_view(QWEN3_DRAFT_MODEL_ID@);
        Identity::from_bytes_spec_view(QWEN3_TARGET_WEIGHT_SET_SHA256@);
        Identity::from_bytes_spec_view(QWEN3_DRAFT_WEIGHT_SHA256@);
        Identity::extensional(&model.config.model_id, &fixed_model_spec(role).config.model_id);
        Identity::extensional(&model.config.config_id, &fixed_model_spec(role).config.config_id);
        Identity::extensional(
            &model.tokenizer.tokenizer_id,
            &fixed_model_spec(role).tokenizer.tokenizer_id,
        );
        Identity::extensional(
            &model.tokenizer.vocabulary_id,
            &fixed_model_spec(role).tokenizer.vocabulary_id,
        );
        Identity::extensional(&model.weights.weights_id, &fixed_model_spec(role).weights.weights_id);
    }
    model
}

fn bytes_equal(left: &[u8], right: &[u8]) -> (equal: bool)
    ensures equal == (left@ == right@),
{
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len()
        invariant
            left@.len() == right@.len(),
            0 <= index <= left@.len(),
            forall|prior: int| 0 <= prior < index ==> left@[prior] == right@[prior],
        decreases left@.len() - index,
    {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    assert(left@ =~= right@) by {
        assert forall|position: int| 0 <= position < left@.len() implies
            left@[position] == right@[position] by {}
    }
    true
}

/// Encodes one exact admitted Qwen3 target/draft bundle.
///
/// # Errors
///
/// Returns [`CanonicalBundleError`] when any identity, geometry, tokenizer,
/// weight descriptor, limit, or derived model/bundle identity is not exact.
pub fn encode_canonical_deployment_bundle(
    bundle: &DeploymentBundle,
) -> (result: Result<CanonicalDeploymentBundle, CanonicalBundleError>)
    ensures
        result.is_ok() == canonical_deployment_bundle_spec(*bundle),
        result.is_ok() ==> result.get_Ok_0().bytes_spec()
            == canonical_deployment_bundle_wire(*bundle),
{
    validate_exact_bundle(bundle)?;
    let mut writer = Writer::new();
    writer.bytes(&MAGIC)?;
    writer.u32(CANONICAL_DEPLOYMENT_BUNDLE_VERSION)?;
    writer.identity(bundle.bundle_id)?;
    writer.u8(TARGET_GFX942_XNACK_MINUS)?;
    writer.u8(NUMERICAL_BF16_FP32)?;
    writer.limits(bundle.limits)?;
    writer.model(bundle.target_model)?;
    writer.model(bundle.draft_model)?;
    proof { canonical_deployment_bundle_wire_len(*bundle); }
    assert(writer.offset == CANONICAL_DEPLOYMENT_BUNDLE_BYTES);
    assert(writer.view() == writer.bytes@);
    Ok(CanonicalDeploymentBundle { bytes: writer.bytes })
}

fn parse_canonical_deployment_bundle(
    bytes: &[u8],
) -> (result: Result<DeploymentBundle, CanonicalBundleError>)
    ensures
        result.is_ok() == canonical_deployment_bundle_syntax(bytes@),
        result.is_ok() ==> result.get_Ok_0() == parsed_bundle_spec(bytes@),
{
    if bytes.len() != CANONICAL_DEPLOYMENT_BUNDLE_BYTES {
        return Err(CanonicalBundleError::InvalidLength);
    }
    let mut reader = Reader::new(bytes);
    let magic = reader.array::<16>()?;
    if !bytes_equal(&magic, &MAGIC) {
        return Err(CanonicalBundleError::InvalidMagic);
    }
    if reader.u32()? != CANONICAL_DEPLOYMENT_BUNDLE_VERSION {
        return Err(CanonicalBundleError::InvalidVersion);
    }
    let bundle_id = reader.identity()?;
    if reader.u8()? != TARGET_GFX942_XNACK_MINUS {
        return Err(CanonicalBundleError::InvalidTag("target"));
    }
    if reader.u8()? != NUMERICAL_BF16_FP32 {
        return Err(CanonicalBundleError::InvalidTag("numerical_policy"));
    }
    let limits = EngineLimits {
        max_context_tokens: reader.u32()?,
        max_active_sequences: reader.u32()?,
        kv_page_tokens: reader.u32()?,
        max_draft_tokens: reader.u32()?,
    };
    if bytes[70] != ROLE_TARGET_8B {
        return Err(CanonicalBundleError::InvalidTag("target_model"));
    }
    if bytes[171] > 1 {
        return Err(CanonicalBundleError::InvalidBoolean("tie_word_embeddings"));
    }
    if bytes[296] != ROLE_DRAFT_06B {
        return Err(CanonicalBundleError::InvalidTag("draft_model"));
    }
    if bytes[397] > 1 {
        return Err(CanonicalBundleError::InvalidBoolean("tie_word_embeddings"));
    }
    let bundle = DeploymentBundle {
        bundle_id,
        target: Target::Gfx942XnackMinus,
        numerical_policy: NumericalPolicy::Bf16ParametersFp32Accumulation,
        limits,
        target_model: fixed_model(Qwen3ModelRole::Target8B),
        draft_model: fixed_model(Qwen3ModelRole::Draft06B),
    };
    proof {
        Identity::from_bytes_spec_view(bytes@.subrange(20, 52));
        Identity::extensional(&bundle.bundle_id, &parsed_bundle_spec(bytes@).bundle_id);
        assert(bundle == parsed_bundle_spec(bytes@));
    }
    Ok(bundle)
}

/// Decodes and revalidates one exact fixed-width Qwen3 bundle record.
///
/// # Errors
///
/// Returns [`CanonicalBundleError`] for truncation, trailing bytes,
/// noncanonical scalar encodings, pin drift, or derived-identity mismatch.
pub fn decode_canonical_deployment_bundle(
    bytes: &[u8],
) -> (result: Result<DeploymentBundle, CanonicalBundleError>)
    ensures
        result.is_ok() == canonical_deployment_bundle_bytes(bytes@),
        result.is_ok() ==> {
            &&& canonical_deployment_bundle_spec(result.get_Ok_0())
            &&& bytes@ == canonical_deployment_bundle_wire(result.get_Ok_0())
            &&& result.get_Ok_0() == parsed_bundle_spec(bytes@)
        },
{
    let bundle = parse_canonical_deployment_bundle(bytes)?;
    validate_exact_bundle(&bundle)?;
    let canonical = encode_canonical_deployment_bundle(&bundle)?;
    if !bytes_equal(canonical.as_bytes(), bytes) {
        return Err(CanonicalBundleError::PinnedFieldMismatch("canonical_encoding"));
    }
    Ok(bundle)
}

} // verus!

impl fmt::Display for CanonicalBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => formatter.write_str("canonical bundle has the wrong length"),
            Self::InvalidMagic => formatter.write_str("canonical bundle magic is invalid"),
            Self::InvalidVersion => formatter.write_str("canonical bundle version is unsupported"),
            Self::InvalidTag(field) => write!(formatter, "canonical bundle tag {field} is invalid"),
            Self::InvalidBoolean(field) => {
                write!(formatter, "canonical bundle boolean {field} is invalid")
            }
            Self::PinnedFieldMismatch(field) => {
                write!(formatter, "canonical bundle field {field} is not pinned")
            }
            Self::Spec(error) => write!(formatter, "canonical bundle is invalid: {error}"),
        }
    }
}

impl std::error::Error for CanonicalBundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spec(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        decode_canonical_deployment_bundle, encode_canonical_deployment_bundle,
        CanonicalBundleError, CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
        CANONICAL_DEPLOYMENT_BUNDLE_VERSION,
    };
    use crate::{
        build_preliminary_deployment_bundle, ArtifactDigest, DeploymentAssets, ModelAssets,
        WeightDescriptor, DRAFT_REPOSITORY, DRAFT_REVISION, QWEN3_DRAFT_TENSOR_DATA_BYTES,
        QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256,
        QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
        QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_BYTES, QWEN3_TOKENIZER_SHA256,
        TARGET_REPOSITORY, TARGET_REVISION,
    };
    use ferric_spec::EngineLimits;

    const TARGET_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-8b-config.json");
    const DRAFT_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-06b-config.json");
    const TOKENIZER_METADATA: &[u8] = include_bytes!("fixtures/qwen3-tokenizer-config.json");

    fn pinned_config(bytes: &'static [u8]) -> &'static [u8] {
        assert_eq!(bytes.last(), Some(&b'\n'));
        &bytes[..bytes.len() - 1]
    }

    fn model_assets(
        repository: &'static str,
        revision: &'static str,
        config_json: &'static [u8],
        weights: WeightDescriptor,
    ) -> ModelAssets<'static> {
        ModelAssets {
            repository,
            revision,
            config_json,
            tokenizer_metadata_json: TOKENIZER_METADATA,
            vocabulary: ArtifactDigest {
                sha256: QWEN3_TOKENIZER_SHA256,
                byte_len: QWEN3_TOKENIZER_BYTES,
            },
            weights,
        }
    }

    pub(crate) fn exact_bundle() -> ferric_spec::DeploymentBundle {
        build_preliminary_deployment_bundle(DeploymentAssets {
            target: model_assets(
                TARGET_REPOSITORY,
                TARGET_REVISION,
                pinned_config(TARGET_CONFIG),
                WeightDescriptor {
                    weights_id: QWEN3_TARGET_WEIGHT_SET_SHA256,
                    artifact_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
                    tensor_data_bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
                    sections: 5,
                },
            ),
            draft: model_assets(
                DRAFT_REPOSITORY,
                DRAFT_REVISION,
                pinned_config(DRAFT_CONFIG),
                WeightDescriptor {
                    weights_id: QWEN3_DRAFT_WEIGHT_SHA256,
                    artifact_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
                    tensor_data_bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
                    sections: 1,
                },
            ),
            limits: EngineLimits {
                max_context_tokens: 8_192,
                max_active_sequences: 32,
                kv_page_tokens: 16,
                max_draft_tokens: 8,
            },
        })
        .expect("exact preliminary bundle")
    }

    fn canonical_bytes() -> [u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES] {
        encode_canonical_deployment_bundle(&exact_bundle())
            .expect("canonical encoding")
            .as_bytes()
            .to_owned()
    }

    #[test]
    fn exact_bundle_round_trips_canonically() {
        let bundle = exact_bundle();
        let encoded = encode_canonical_deployment_bundle(&bundle).expect("canonical encoding");
        assert_eq!(encoded.as_bytes().len(), CANONICAL_DEPLOYMENT_BUNDLE_BYTES);
        assert_eq!(
            decode_canonical_deployment_bundle(encoded.as_bytes()),
            Ok(bundle)
        );
        assert_eq!(
            encode_canonical_deployment_bundle(
                &decode_canonical_deployment_bundle(encoded.as_bytes()).expect("canonical decode")
            )
            .expect("canonical re-encode"),
            encoded
        );
    }

    #[test]
    fn exact_offsets_and_big_endian_scalars_are_stable() {
        let bytes = canonical_bytes();
        assert_eq!(&bytes[0..16], b"FERRIC-M1-BUNDLE");
        assert_eq!(
            &bytes[16..20],
            &CANONICAL_DEPLOYMENT_BUNDLE_VERSION.to_be_bytes()
        );
        assert_eq!(&bytes[54..58], &8_192u32.to_be_bytes());
        assert_eq!(&bytes[58..62], &32u32.to_be_bytes());
        assert_eq!(&bytes[62..66], &16u32.to_be_bytes());
        assert_eq!(&bytes[66..70], &8u32.to_be_bytes());
        assert_eq!(bytes[70], 1);
        assert_eq!(bytes[171], 0);
        assert_eq!(bytes[296], 2);
        assert_eq!(bytes[397], 1);
        assert_eq!(
            &bytes[284..292],
            &QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES.to_be_bytes()
        );
        assert_eq!(
            &bytes[510..518],
            &QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES.to_be_bytes()
        );
    }

    #[test]
    fn truncation_trailing_and_arbitrary_inputs_never_panic() {
        let bytes = canonical_bytes();
        for length in 0..CANONICAL_DEPLOYMENT_BUNDLE_BYTES {
            assert!(decode_canonical_deployment_bundle(&bytes[..length]).is_err());
        }
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert_eq!(
            decode_canonical_deployment_bundle(&trailing),
            Err(CanonicalBundleError::InvalidLength)
        );
        for seed in 0u8..=255 {
            let hostile = [seed; CANONICAL_DEPLOYMENT_BUNDLE_BYTES];
            assert!(decode_canonical_deployment_bundle(&hostile).is_err());
        }
    }

    #[test]
    fn hostile_tags_booleans_and_endianness_are_rejected() {
        let bytes = canonical_bytes();
        for (offset, expected) in [
            (52, CanonicalBundleError::InvalidTag("target")),
            (53, CanonicalBundleError::InvalidTag("numerical_policy")),
            (70, CanonicalBundleError::InvalidTag("target_model")),
            (296, CanonicalBundleError::InvalidTag("draft_model")),
        ] {
            let mut changed = bytes;
            changed[offset] = 9;
            assert_eq!(decode_canonical_deployment_bundle(&changed), Err(expected));
        }
        for offset in [171, 397] {
            let mut changed = bytes;
            changed[offset] = 2;
            assert_eq!(
                decode_canonical_deployment_bundle(&changed),
                Err(CanonicalBundleError::InvalidBoolean("tie_word_embeddings"))
            );
        }
        for start in [54, 58, 62, 66, 135, 292, 361, 518] {
            let mut changed = bytes;
            changed[start..start + 4].reverse();
            assert!(decode_canonical_deployment_bundle(&changed).is_err());
        }
    }

    #[test]
    fn every_single_byte_drift_is_rejected() {
        let encoded = canonical_bytes();
        for index in 0..CANONICAL_DEPLOYMENT_BUNDLE_BYTES {
            let mut changed = encoded;
            changed[index] ^= 1;
            assert!(
                decode_canonical_deployment_bundle(&changed).is_err(),
                "byte {index} was not identity-sensitive"
            );
        }
    }

    #[test]
    fn descriptor_values_cannot_claim_canonical_authority() {
        let mut bundle = exact_bundle();
        bundle.target_model.weights.total_bytes -= 1;
        assert_eq!(
            encode_canonical_deployment_bundle(&bundle),
            Err(CanonicalBundleError::PinnedFieldMismatch("weight_bytes"))
        );
    }
}
