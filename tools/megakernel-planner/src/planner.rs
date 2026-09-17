use core::fmt;

/// Structural declaration wire version. It is not a device task ABI.
pub const DECLARATION_VERSION: u32 = 1;
/// Exact processor required by this engineering planning envelope.
pub const DECLARED_PROCESSOR: &str = "gfx950";
/// Wave size required by the future target-specific lowering.
pub const DECLARED_WAVE_SIZE: u32 = 64;
/// BF16 Qwen3 vocabulary shared by both models.
pub const VOCABULARY_SIZE: u32 = 151_936;
const ALIGNMENT: u64 = 256;
const MAX_TASKS: u32 = 1_024;
const MAX_BUFFERS: u32 = 4_096;

/// Supported *declared* model geometry, not authenticated model admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3Model {
    /// Qwen3-0.6B, used independently rather than as a speculative draft.
    Qwen3_06B,
    /// Qwen3-8B.
    Qwen3_8B,
}

/// Exact dense model dimensions used by the operation graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelGeometry {
    /// Decoder layer count.
    pub layers: u32,
    /// Residual/embedding width.
    pub hidden: u32,
    /// MLP intermediate width.
    pub intermediate: u32,
    /// Query head count.
    pub query_heads: u32,
    /// Key/value head count.
    pub kv_heads: u32,
    /// Per-head width.
    pub head_dim: u32,
    /// Exact IEEE-754 binary32 bits of the model `RMSNorm` epsilon (1e-6).
    pub rms_norm_epsilon_bits: u32,
    /// Integer rotary-embedding base from the pinned Qwen3 configurations.
    pub rope_theta: u32,
    /// Whether the output projection shares the token embedding weights.
    pub tied_embeddings: bool,
}

impl Qwen3Model {
    /// Geometry is independent of the residual width: Qwen3-0.6B has
    /// 16 * 128 query channels but only 1,024 residual channels.
    #[must_use]
    pub const fn geometry(self) -> ModelGeometry {
        match self {
            Self::Qwen3_06B => ModelGeometry {
                layers: 28,
                hidden: 1_024,
                intermediate: 3_072,
                query_heads: 16,
                kv_heads: 8,
                head_dim: 128,
                rms_norm_epsilon_bits: 1e-6_f32.to_bits(),
                rope_theta: 1_000_000,
                tied_embeddings: true,
            },
            Self::Qwen3_8B => ModelGeometry {
                layers: 36,
                hidden: 4_096,
                intermediate: 12_288,
                query_heads: 32,
                kv_heads: 8,
                head_dim: 128,
                rms_norm_epsilon_bits: 1e-6_f32.to_bits(),
                rope_theta: 1_000_000,
                tied_embeddings: false,
            },
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::Qwen3_06B => 1,
            Self::Qwen3_8B => 2,
        }
    }
}

/// The initial numerical contract is only a declared policy, not evidence that
/// a handler implements it. Norm/softmax reductions and accumulators are FP32;
/// stored activations, KV and weights are BF16 with ties-to-even conversion.
/// The fused `AttentionOutputResidual` and `DownResidual` tags preserve two
/// separate rounding boundaries: accumulate the projection in FP32, round the
/// projection to BF16, add that rounded value to the BF16 residual in FP32,
/// then round the sum to BF16. Adding the unrounded projection to the residual
/// and casting only once is a different numerical policy.
/// Vocabulary logits remain FP32 to avoid an additional argmax rounding step.
/// This is a candidate handler contract, not established equivalence to an
/// existing reference implementation or numerical evidence for model outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclaredNumericalPolicy {
    /// BF16 activations/weights/KV, FP32 accumulation/logits, stable argmax.
    Bf16Fp32LogitsStableArgmax,
}

/// A request-independent shape bucket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeShape {
    /// Exact declared model.
    pub model: Qwen3Model,
    /// Number of active independent sequences: 1, 2, 4, or 8.
    pub batch: u32,
    /// Capacity including the newly appended token: 1K, 4K, or 8K.
    pub context_capacity: u32,
    /// Power-of-two KV page size, at most 256 tokens.
    pub kv_page_tokens: u32,
}

/// Caller-supplied commitments compared with an independently supplied
/// expectation. Nonzero bytes do not establish provenance or authenticity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredIdentityBindings {
    /// Authenticated-bundle commitment expected by a future custody bridge.
    pub bundle: [u8; 32],
    /// Exact model configuration commitment.
    pub config: [u8; 32],
    /// Weight contents/layout commitment.
    pub weights: [u8; 32],
    /// Tokenizer commitment.
    pub tokenizer: [u8; 32],
    /// Sequential graph commitment.
    pub graph: [u8; 32],
    /// Numerical-policy commitment.
    pub numerical_policy: [u8; 32],
    /// Bytes of an upstream-owned task schema identity declaration.
    pub task_schema: [u8; 32],
    /// Bytes of an upstream-owned scheduler model identity declaration.
    pub scheduler_model: [u8; 32],
    /// Bytes of an upstream-owned fusion plan identity declaration.
    pub fusion_plan: [u8; 32],
    /// Bytes of an upstream-owned persistent plan identity declaration.
    pub persistent_plan: [u8; 32],
}

impl DeclaredIdentityBindings {
    fn fields(&self) -> [[u8; 32]; 10] {
        [
            self.bundle,
            self.config,
            self.weights,
            self.tokenizer,
            self.graph,
            self.numerical_policy,
            self.task_schema,
            self.scheduler_model,
            self.fusion_plan,
            self.persistent_plan,
        ]
    }

    fn validate(self) -> Result<(), PlanError> {
        for (index, identity) in self.fields().iter().enumerate() {
            if identity.iter().all(|byte| *byte == 0) {
                return Err(PlanError::MissingIdentity(index));
            }
        }
        Ok(())
    }
}

/// Host planner resource limits. These are not hardware admission evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanResourceLimits {
    /// Maximum operation records.
    pub tasks: u32,
    /// Maximum tensor records.
    pub buffers: u32,
    /// Maximum separately allocated temporary bytes.
    pub workspace_bytes: u64,
    /// Conservative ready-storage capacity, one slot per operation.
    pub ready_tasks: u32,
}

impl Default for PlanResourceLimits {
    fn default() -> Self {
        Self {
            tasks: MAX_TASKS,
            buffers: MAX_BUFFERS,
            workspace_bytes: 256 * 1_024 * 1_024,
            ready_tasks: MAX_TASKS,
        }
    }
}

/// Tensor scalar representation in the addressless declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    /// Stored weights, activations, and KV.
    Bf16,
    /// Token IDs, positions, and compact greedy output IDs.
    U32,
    /// Vocabulary logits retained without a BF16 narrowing conversion.
    F32,
}

impl ScalarType {
    const fn bytes(self) -> u64 {
        match self {
            Self::Bf16 => 2,
            Self::U32 | Self::F32 => 4,
        }
    }
    const fn tag(self) -> u8 {
        match self {
            Self::Bf16 => 1,
            Self::U32 => 2,
            Self::F32 => 3,
        }
    }
}

/// Closed tensor role roster. Layer roles carry their layer separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TensorRole {
    /// Per-request input token IDs.
    TokenIds = 1,
    /// Absolute decode positions, equal to committed KV lengths here.
    Positions,
    /// Token embedding table, also the tied language model head in 0.6B.
    EmbeddingWeight,
    /// Untied output language model head in 8B.
    LanguageHeadWeight,
    /// Final `RMSNorm` scale.
    FinalNormWeight,
    /// Input `RMSNorm` scale.
    InputNormWeight,
    /// Post-attention `RMSNorm` scale.
    PostNormWeight,
    /// Query head `RMSNorm` scale.
    QueryNormWeight,
    /// Key head `RMSNorm` scale.
    KeyNormWeight,
    /// Query projection weight.
    QueryWeight,
    /// Key projection weight.
    KeyWeight,
    /// Value projection weight.
    ValueWeight,
    /// Attention output projection weight.
    OutputWeight,
    /// MLP gate projection weight.
    GateWeight,
    /// MLP up projection weight.
    UpWeight,
    /// MLP down projection weight.
    DownWeight,
    /// Previously committed paged keys, read-only for this step.
    KeyPrefix,
    /// Previously committed paged values, read-only for this step.
    ValuePrefix,
    /// Reserved key append slice, distinct from committed history.
    AppendedKey,
    /// Reserved value append slice, distinct from committed history.
    AppendedValue,
    /// Residual stream.
    Hidden,
    /// Normalized residual stream.
    NormalizedHidden,
    /// Query projection result.
    Query,
    /// Key projection result.
    Key,
    /// Value projection result.
    Value,
    /// Normalized query heads.
    NormalizedQuery,
    /// Normalized key heads.
    NormalizedKey,
    /// Position-rotated queries.
    RotatedQuery,
    /// Position-rotated keys.
    RotatedKey,
    /// Attention result before output projection.
    Attention,
    /// Attention residual sum.
    AttentionResidual,
    /// Post-attention normalized stream.
    PostNormalized,
    /// MLP gate activation.
    Gate,
    /// MLP up activation.
    Up,
    /// `SwiGLU` activation.
    Activated,
    /// Final normalized residual stream.
    FinalNormalized,
    /// Dense vocabulary logits.
    Logits,
    /// Lowest-index maximum token per sequence.
    GreedyTokens,
}

/// Storage ownership class; no addresses or live allocation handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    /// Read-only input, weight, or committed KV declaration. KV dimensions
    /// describe capacity, not a lease over initialized data: the future
    /// runner must bind only each request's committed prefix and prove that
    /// its append reservation is disjoint from that initialized range.
    ExternalReadOnly,
    /// A future request-owned append reservation, not the committed KV prefix.
    KvAppendReservation,
    /// A dedicated non-overlapping range retained until dispatch quiescence.
    Workspace {
        /// Aligned byte offset relative to a future workspace allocation.
        offset: u64,
    },
}

/// A tensor with exact size and producer/consumer ownership.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferDeclaration {
    /// Dense zero-based tensor record position.
    pub id: u32,
    /// Tensor's semantic role.
    pub role: TensorRole,
    /// Decoder layer, or absent for global tensors.
    pub layer: Option<u32>,
    /// Exact scalar representation.
    pub scalar: ScalarType,
    /// Row-major logical dimensions; physical KV paging is bound later.
    pub dimensions: Vec<u32>,
    /// Checked product of dimensions and scalar size.
    pub byte_len: u64,
    /// Storage ownership declaration.
    pub storage: Storage,
    /// Unique producing operation, absent only for external read-only tensors.
    pub producer: Option<u32>,
    /// Complete sorted set of consuming operations.
    pub consumers: Vec<u32>,
}

/// Closed operation family. This is a host graph tag, not a device opcode ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Operation {
    /// Token lookup.
    Embedding = 1,
    /// Pre-attention normalization.
    InputRmsNorm,
    /// Query projection.
    QueryProjection,
    /// Key projection.
    KeyProjection,
    /// Value projection.
    ValueProjection,
    /// Per-query-head normalization.
    QueryRmsNorm,
    /// Per-key-head normalization.
    KeyRmsNorm,
    /// Position rotation of Q and K.
    Rope,
    /// Write to reserved K/V append slices.
    KvAppend,
    /// Paged grouped-query decode attention.
    Attention,
    /// Attention output projection plus residual.
    AttentionOutputResidual,
    /// Pre-MLP normalization.
    PostAttentionRmsNorm,
    /// MLP gate projection.
    GateProjection,
    /// MLP up projection.
    UpProjection,
    /// SiLU(gate) multiplied by up.
    SwiGlu,
    /// MLP down projection plus residual.
    DownResidual,
    /// Final normalization.
    FinalRmsNorm,
    /// Dense language model projection.
    LogitsProjection,
    /// Greedy lowest-index argmax.
    Argmax,
}

/// One complete operation in an addressless finite DAG.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskDeclaration {
    /// Dense zero-based graph node.
    pub id: u32,
    /// Closed operation kind.
    pub operation: Operation,
    /// Decoder layer, absent for global operations.
    pub layer: Option<u32>,
    /// Ordered input tensor record IDs.
    pub inputs: Vec<u32>,
    /// Ordered output tensor record IDs.
    pub outputs: Vec<u32>,
    /// Sorted unique producer task IDs derived from the inputs.
    pub predecessors: Vec<u32>,
}

/// Untrusted structural input. Public fields intentionally allow inspection and
/// adversarial mutation; successful validation does not create GPU authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodePlanDeclaration {
    /// Structural format version.
    pub version: u32,
    /// Exact processor string.
    pub processor: String,
    /// Required wavefront width.
    pub wave_size: u32,
    /// Numerical semantics declaration.
    pub numerical_policy: DeclaredNumericalPolicy,
    /// Exact model and shape bucket.
    pub shape: DecodeShape,
    /// Exact model geometry and normalization/position constants.
    pub geometry: ModelGeometry,
    /// Unauthenticated caller commitments.
    pub identities: DeclaredIdentityBindings,
    /// Complete operation DAG in canonical topological order.
    pub tasks: Vec<TaskDeclaration>,
    /// Complete tensor graph.
    pub buffers: Vec<BufferDeclaration>,
    /// End of the final allocated temporary range.
    pub workspace_bytes: u64,
    /// Worst-case ready storage, conservatively all task records.
    pub ready_capacity: u32,
}

/// A checked structural declaration. Intentionally exposes no loading,
/// submission, or model-admission method and is not a production capability.
#[derive(Debug, PartialEq, Eq)]
pub struct DeclaredDecodePlan {
    declaration: DecodePlanDeclaration,
}

impl DeclaredDecodePlan {
    /// Read the checked addressless graph.
    #[must_use]
    pub const fn declaration(&self) -> &DecodePlanDeclaration {
        &self.declaration
    }

    /// Canonical little-endian bytes for an external commitment. This format
    /// is distinct from the future upstream GPU task descriptor ABI.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let d = &self.declaration;
        let mut bytes = b"ferric.declared-qwen3-operation-dag.v1\0".to_vec();
        put_u32(&mut bytes, d.version);
        bytes.push(d.shape.model.tag());
        for number in [
            d.shape.batch,
            d.shape.context_capacity,
            d.shape.kv_page_tokens,
            d.wave_size,
            d.ready_capacity,
        ] {
            put_u32(&mut bytes, number);
        }
        for number in [
            d.geometry.layers,
            d.geometry.hidden,
            d.geometry.intermediate,
            d.geometry.query_heads,
            d.geometry.kv_heads,
            d.geometry.head_dim,
            d.geometry.rms_norm_epsilon_bits,
            d.geometry.rope_theta,
        ] {
            put_u32(&mut bytes, number);
        }
        bytes.push(u8::from(d.geometry.tied_embeddings));
        bytes.extend_from_slice(DECLARED_PROCESSOR.as_bytes());
        bytes.push(1); // Fixed BF16/FP32 policy tag, not the Rust enum layout.
        for identity in d.identities.fields() {
            bytes.extend_from_slice(&identity);
        }
        put_u64(&mut bytes, d.workspace_bytes);
        put_len(&mut bytes, d.tasks.len());
        for task in &d.tasks {
            put_u32(&mut bytes, task.id);
            bytes.push(task.operation as u8);
            put_u32(&mut bytes, task.layer.unwrap_or(u32::MAX));
            for ids in [&task.inputs, &task.outputs, &task.predecessors] {
                put_len(&mut bytes, ids.len());
                for id in ids {
                    put_u32(&mut bytes, *id);
                }
            }
        }
        put_len(&mut bytes, d.buffers.len());
        for buffer in &d.buffers {
            put_u32(&mut bytes, buffer.id);
            bytes.push(buffer.role as u8);
            bytes.push(buffer.scalar.tag());
            put_u32(&mut bytes, buffer.layer.unwrap_or(u32::MAX));
            put_len(&mut bytes, buffer.dimensions.len());
            for dimension in &buffer.dimensions {
                put_u32(&mut bytes, *dimension);
            }
            put_u64(&mut bytes, buffer.byte_len);
            let (tag, offset) = match buffer.storage {
                Storage::ExternalReadOnly => (1, 0),
                Storage::KvAppendReservation => (2, 0),
                Storage::Workspace { offset } => (3, offset),
            };
            bytes.push(tag);
            put_u64(&mut bytes, offset);
            put_u32(&mut bytes, buffer.producer.unwrap_or(u32::MAX));
            put_len(&mut bytes, buffer.consumers.len());
            for consumer in &buffer.consumers {
                put_u32(&mut bytes, *consumer);
            }
        }
        bytes
    }

    /// Validate addressless per-step patch values without allocating.
    ///
    /// The caller supplies an epoch expectation; comparing it is not
    /// authentication, stale-run detection, or a KV lease. Physical custody and
    /// quiescent epoch advancement must still be provided by Ferric's runner.
    ///
    /// # Errors
    /// Rejects an unsupported epoch, request count, duplicate request slot,
    /// missing generation, invalid token, or position outside the capacity.
    pub fn bind_step<'a>(
        &'a self,
        expected_epoch: u64,
        declared_epoch: u64,
        requests: &'a [RequestDeclaration],
    ) -> Result<DeclaredStepBinding<'a>, PlanError> {
        if expected_epoch == 0 || expected_epoch == u64::MAX || declared_epoch != expected_epoch {
            return Err(PlanError::Epoch);
        }
        if requests.len() != self.declaration.shape.batch as usize {
            return Err(PlanError::RequestCount);
        }
        for (index, request) in requests.iter().enumerate() {
            if request.generation == 0 || request.kv_generation == 0 {
                return Err(PlanError::RequestGeneration(index));
            }
            if request.token_id >= VOCABULARY_SIZE {
                return Err(PlanError::Token(index));
            }
            if request.committed_context >= self.declaration.shape.context_capacity
                || request.position != request.committed_context
            {
                return Err(PlanError::Position(index));
            }
            if requests[..index]
                .iter()
                .any(|prior| prior.slot == request.slot)
            {
                return Err(PlanError::DuplicateRequestSlot(request.slot));
            }
        }
        Ok(DeclaredStepBinding {
            plan: self,
            epoch: declared_epoch,
            requests,
        })
    }
}

/// Addressless request patch data. Generations are declared values, not leases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestDeclaration {
    /// Request slot, unique inside one batch.
    pub slot: u32,
    /// Nonzero declared request generation.
    pub generation: u32,
    /// Nonzero declared KV reservation generation.
    pub kv_generation: u32,
    /// Input token to consume.
    pub token_id: u32,
    /// Committed KV length before this token is appended.
    pub committed_context: u32,
    /// Absolute position, equal to the committed length in this envelope.
    pub position: u32,
}

/// Borrowed checked patch data. No heap allocation or device action occurs.
/// Private fields preserve the structural checks for the borrow's lifetime.
///
/// ```compile_fail
/// use ferric_megakernel_planner::{
///     DeclaredDecodePlan, DeclaredStepBinding, RequestDeclaration,
/// };
/// fn forge<'a>(
///     plan: &'a DeclaredDecodePlan,
///     requests: &'a [RequestDeclaration],
/// ) -> DeclaredStepBinding<'a> {
///     DeclaredStepBinding { plan, epoch: 0, requests }
/// }
/// ```
#[derive(Debug)]
pub struct DeclaredStepBinding<'a> {
    plan: &'a DeclaredDecodePlan,
    epoch: u64,
    requests: &'a [RequestDeclaration],
}

impl<'a> DeclaredStepBinding<'a> {
    /// Addressless checked plan retained by this binding.
    #[must_use]
    pub const fn plan(&self) -> &'a DeclaredDecodePlan {
        self.plan
    }

    /// Compared caller-declared epoch; still not live runtime custody.
    #[must_use]
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Borrowed checked per-request declarations, exposed read-only.
    #[must_use]
    pub const fn requests(&self) -> &'a [RequestDeclaration] {
        self.requests
    }
}

/// Structural validation diagnostic; none implies a device action occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// Batch or context is outside the fixed product envelope.
    Shape,
    /// KV page length is invalid.
    PageSize,
    /// An identity declaration was absent.
    MissingIdentity(usize),
    /// Caller declarations differ from the independent expectation.
    IdentityMismatch,
    /// Processor, wavefront, or format does not match this declaration.
    TargetOrVersion,
    /// Host planning bounds were exceeded.
    ResourceLimit,
    /// A tensor size or range arithmetic overflowed.
    ArithmeticOverflow,
    /// Tensor shape or declared byte length is malformed.
    BufferShape(u32),
    /// A graph record or edge is inconsistent, cyclic, or dangling.
    Graph(u32),
    /// Two live workspace buffers overlap.
    WorkspaceOverlap(u32, u32),
    /// Workspace extent or alignment is invalid.
    WorkspaceRange(u32),
    /// The structurally valid graph differs from the exact model graph.
    NonCanonicalGraph,
    /// Epoch is absent, would wrap, or differs from the expected declaration.
    Epoch,
    /// Request count differs from the fixed batch.
    RequestCount,
    /// A declared generation is absent.
    RequestGeneration(usize),
    /// A request token lies outside the vocabulary.
    Token(usize),
    /// A context/position exceeds or disagrees with the admitted bucket.
    Position(usize),
    /// One request slot is present twice, even with distinct generations.
    DuplicateRequestSlot(u32),
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid addressless decode declaration: {self:?}"
        )
    }
}

impl std::error::Error for PlanError {}

/// Build the complete, addressless Qwen3 operation DAG.
///
/// # Errors
/// Rejects missing identity declarations, unsupported shapes, arithmetic
/// overflow, or a graph/workspace that exceeds the caller's host limits.
pub fn declare_decode_plan(
    shape: DecodeShape,
    identities: DeclaredIdentityBindings,
    limits: PlanResourceLimits,
) -> Result<DeclaredDecodePlan, PlanError> {
    validate_shape(shape)?;
    identities.validate()?;
    let declaration = build_graph(shape, &identities)?;
    validate_structure(&declaration, limits)?;
    Ok(DeclaredDecodePlan { declaration })
}

/// Validate hostile declaration data against the exact canonical graph and an
/// independent identity expectation. Does not authenticate either input.
///
/// # Errors
/// Rejects malformed resources/edges, stale identity declarations, changed
/// model operators/shapes/lifetimes, or noncanonical target/format fields.
pub fn validate_declaration(
    declaration: DecodePlanDeclaration,
    expected_shape: DecodeShape,
    expected_identities: DeclaredIdentityBindings,
    limits: PlanResourceLimits,
) -> Result<DeclaredDecodePlan, PlanError> {
    validate_shape(expected_shape)?;
    validate_shape(declaration.shape)?;
    if declaration.shape != expected_shape {
        return Err(PlanError::Shape);
    }
    expected_identities.validate()?;
    declaration.identities.validate()?;
    if declaration.identities != expected_identities {
        return Err(PlanError::IdentityMismatch);
    }
    if declaration.version != DECLARATION_VERSION
        || declaration.processor != DECLARED_PROCESSOR
        || declaration.wave_size != DECLARED_WAVE_SIZE
    {
        return Err(PlanError::TargetOrVersion);
    }
    validate_structure(&declaration, limits)?;
    if declaration != build_graph(declaration.shape, &expected_identities)? {
        return Err(PlanError::NonCanonicalGraph);
    }
    Ok(DeclaredDecodePlan { declaration })
}

fn validate_shape(shape: DecodeShape) -> Result<(), PlanError> {
    if !matches!(shape.batch, 1 | 2 | 4 | 8)
        || !matches!(shape.context_capacity, 1_024 | 4_096 | 8_192)
    {
        return Err(PlanError::Shape);
    }
    if !shape.kv_page_tokens.is_power_of_two()
        || shape.kv_page_tokens > 256
        || !shape.context_capacity.is_multiple_of(shape.kv_page_tokens)
    {
        return Err(PlanError::PageSize);
    }
    Ok(())
}

fn tensor_bytes(scalar: ScalarType, dimensions: &[u32]) -> Result<u64, PlanError> {
    if dimensions.is_empty() || dimensions.len() > 4 || dimensions.contains(&0) {
        return Err(PlanError::BufferShape(u32::MAX));
    }
    dimensions
        .iter()
        .try_fold(scalar.bytes(), |size, dimension| {
            size.checked_mul(u64::from(*dimension))
                .ok_or(PlanError::ArithmeticOverflow)
        })
}

fn validate_structure(
    declaration: &DecodePlanDeclaration,
    limits: PlanResourceLimits,
) -> Result<(), PlanError> {
    let task_count = declaration.tasks.len();
    if task_count == 0
        || task_count > MAX_TASKS as usize
        || task_count > limits.tasks as usize
        || declaration.buffers.len() > MAX_BUFFERS as usize
        || declaration.buffers.len() > limits.buffers as usize
        || declaration.workspace_bytes > limits.workspace_bytes
        || declaration.ready_capacity as usize != task_count
        || declaration.ready_capacity > limits.ready_tasks
    {
        return Err(PlanError::ResourceLimit);
    }
    // Bound all edge vectors before any cross-record search. Checking this
    // only in the task walk lets hostile vectors amplify the buffer walk.
    for (index, task) in declaration.tasks.iter().enumerate() {
        if task.id as usize != index
            || task.outputs.is_empty()
            || task.inputs.len() > 8
            || task.outputs.len() > 2
            || task.predecessors.len() > 8
        {
            return Err(PlanError::Graph(task.id));
        }
    }
    for (index, buffer) in declaration.buffers.iter().enumerate() {
        if buffer.id as usize != index
            || tensor_bytes(buffer.scalar, &buffer.dimensions)? != buffer.byte_len
        {
            return Err(PlanError::BufferShape(buffer.id));
        }
        let expected_external = matches!(buffer.storage, Storage::ExternalReadOnly);
        if expected_external != buffer.producer.is_none() {
            return Err(PlanError::Graph(buffer.id));
        }
        if let Some(producer) = buffer.producer {
            if producer as usize >= task_count
                || !declaration.tasks[producer as usize]
                    .outputs
                    .contains(&buffer.id)
            {
                return Err(PlanError::Graph(buffer.id));
            }
        }
        if buffer.consumers.len() > task_count
            || buffer.consumers.windows(2).any(|pair| pair[0] >= pair[1])
            || buffer.consumers.iter().any(|consumer| {
                *consumer as usize >= task_count
                    || !declaration.tasks[*consumer as usize]
                        .inputs
                        .contains(&buffer.id)
            })
        {
            return Err(PlanError::Graph(buffer.id));
        }
        if let Storage::Workspace { offset } = buffer.storage {
            let end = offset
                .checked_add(buffer.byte_len)
                .ok_or(PlanError::ArithmeticOverflow)?;
            if !offset.is_multiple_of(ALIGNMENT) || end > declaration.workspace_bytes {
                return Err(PlanError::WorkspaceRange(buffer.id));
            }
            // No interval reuse is admitted yet: independent DAG branches can
            // run out of canonical order, so ordinal intervals are not enough.
            for prior in &declaration.buffers[..index] {
                if let Storage::Workspace {
                    offset: prior_offset,
                } = prior.storage
                {
                    let prior_end = prior_offset
                        .checked_add(prior.byte_len)
                        .ok_or(PlanError::ArithmeticOverflow)?;
                    if offset < prior_end && prior_offset < end {
                        return Err(PlanError::WorkspaceOverlap(prior.id, buffer.id));
                    }
                }
            }
        }
    }
    for task in &declaration.tasks {
        let mut predecessors = Vec::new();
        for (input_index, input) in task.inputs.iter().enumerate() {
            let buffer = declaration
                .buffers
                .get(*input as usize)
                .ok_or(PlanError::Graph(task.id))?;
            if task.inputs[..input_index].contains(input) || !buffer.consumers.contains(&task.id) {
                return Err(PlanError::Graph(task.id));
            }
            if let Some(producer) = buffer.producer {
                if producer >= task.id {
                    return Err(PlanError::Graph(task.id));
                }
                predecessors.push(producer);
            }
        }
        predecessors.sort_unstable();
        predecessors.dedup();
        if predecessors != task.predecessors {
            return Err(PlanError::Graph(task.id));
        }
        for (output_index, output) in task.outputs.iter().enumerate() {
            let buffer = declaration
                .buffers
                .get(*output as usize)
                .ok_or(PlanError::Graph(task.id))?;
            if task.outputs[..output_index].contains(output) || buffer.producer != Some(task.id) {
                return Err(PlanError::Graph(task.id));
            }
        }
    }
    Ok(())
}

struct Builder {
    declaration: DecodePlanDeclaration,
}

impl Builder {
    fn buffer(
        &mut self,
        role: TensorRole,
        layer: Option<u32>,
        scalar: ScalarType,
        dimensions: Vec<u32>,
        producer: Option<u32>,
        append: bool,
    ) -> Result<u32, PlanError> {
        let id = u32::try_from(self.declaration.buffers.len())
            .map_err(|_| PlanError::ArithmeticOverflow)?;
        let byte_len = tensor_bytes(scalar, &dimensions)?;
        let storage = if producer.is_none() {
            Storage::ExternalReadOnly
        } else if append {
            Storage::KvAppendReservation
        } else {
            let offset = self
                .declaration
                .workspace_bytes
                .checked_add(ALIGNMENT - 1)
                .ok_or(PlanError::ArithmeticOverflow)?
                / ALIGNMENT
                * ALIGNMENT;
            self.declaration.workspace_bytes = offset
                .checked_add(byte_len)
                .ok_or(PlanError::ArithmeticOverflow)?;
            Storage::Workspace { offset }
        };
        self.declaration.buffers.push(BufferDeclaration {
            id,
            role,
            layer,
            scalar,
            dimensions,
            byte_len,
            storage,
            producer,
            consumers: Vec::new(),
        });
        Ok(id)
    }

    fn external(
        &mut self,
        role: TensorRole,
        layer: Option<u32>,
        dimensions: Vec<u32>,
    ) -> Result<u32, PlanError> {
        self.buffer(role, layer, ScalarType::Bf16, dimensions, None, false)
    }

    fn operation(
        &mut self,
        operation: Operation,
        layer: Option<u32>,
        inputs: &[u32],
        outputs: &[(TensorRole, Vec<u32>)],
    ) -> Result<Vec<u32>, PlanError> {
        let id = u32::try_from(self.declaration.tasks.len())
            .map_err(|_| PlanError::ArithmeticOverflow)?;
        let mut predecessors = Vec::new();
        for input in inputs {
            let buffer = &mut self.declaration.buffers[*input as usize];
            if let Some(producer) = buffer.producer {
                predecessors.push(producer);
            }
            buffer.consumers.push(id);
        }
        predecessors.sort_unstable();
        predecessors.dedup();
        let mut output_ids = Vec::new();
        for (role, dimensions) in outputs {
            let scalar = match role {
                TensorRole::GreedyTokens => ScalarType::U32,
                TensorRole::Logits => ScalarType::F32,
                _ => ScalarType::Bf16,
            };
            output_ids.push(self.buffer(
                *role,
                layer,
                scalar,
                dimensions.clone(),
                Some(id),
                matches!(role, TensorRole::AppendedKey | TensorRole::AppendedValue),
            )?);
        }
        self.declaration.tasks.push(TaskDeclaration {
            id,
            operation,
            layer,
            inputs: inputs.to_vec(),
            outputs: output_ids.clone(),
            predecessors,
        });
        Ok(output_ids)
    }
}

fn build_graph(
    shape: DecodeShape,
    identities: &DeclaredIdentityBindings,
) -> Result<DecodePlanDeclaration, PlanError> {
    let geometry = shape.model.geometry();
    let mut builder = Builder {
        declaration: DecodePlanDeclaration {
            version: DECLARATION_VERSION,
            processor: DECLARED_PROCESSOR.into(),
            wave_size: DECLARED_WAVE_SIZE,
            numerical_policy: DeclaredNumericalPolicy::Bf16Fp32LogitsStableArgmax,
            shape,
            geometry,
            identities: *identities,
            tasks: Vec::new(),
            buffers: Vec::new(),
            workspace_bytes: 0,
            ready_capacity: 0,
        },
    };
    let batch = shape.batch;
    let query_width = geometry
        .query_heads
        .checked_mul(geometry.head_dim)
        .ok_or(PlanError::ArithmeticOverflow)?;
    let kv_width = geometry
        .kv_heads
        .checked_mul(geometry.head_dim)
        .ok_or(PlanError::ArithmeticOverflow)?;
    let token_ids = builder.buffer(
        TensorRole::TokenIds,
        None,
        ScalarType::U32,
        vec![batch],
        None,
        false,
    )?;
    let positions = builder.buffer(
        TensorRole::Positions,
        None,
        ScalarType::U32,
        vec![batch],
        None,
        false,
    )?;
    let embedding = builder.external(
        TensorRole::EmbeddingWeight,
        None,
        vec![VOCABULARY_SIZE, geometry.hidden],
    )?;
    let language_head = if geometry.tied_embeddings {
        embedding
    } else {
        builder.external(
            TensorRole::LanguageHeadWeight,
            None,
            vec![VOCABULARY_SIZE, geometry.hidden],
        )?
    };
    let final_norm = builder.external(TensorRole::FinalNormWeight, None, vec![geometry.hidden])?;
    let mut hidden = builder.operation(
        Operation::Embedding,
        None,
        &[token_ids, embedding],
        &[(TensorRole::Hidden, vec![batch, geometry.hidden])],
    )?[0];

    for layer in 0..geometry.layers {
        let at = Some(layer);
        let input_norm =
            builder.external(TensorRole::InputNormWeight, at, vec![geometry.hidden])?;
        let post_norm = builder.external(TensorRole::PostNormWeight, at, vec![geometry.hidden])?;
        let query_norm =
            builder.external(TensorRole::QueryNormWeight, at, vec![geometry.head_dim])?;
        let key_norm = builder.external(TensorRole::KeyNormWeight, at, vec![geometry.head_dim])?;
        let wq = builder.external(
            TensorRole::QueryWeight,
            at,
            vec![query_width, geometry.hidden],
        )?;
        let wk = builder.external(TensorRole::KeyWeight, at, vec![kv_width, geometry.hidden])?;
        let wv = builder.external(TensorRole::ValueWeight, at, vec![kv_width, geometry.hidden])?;
        let wo = builder.external(
            TensorRole::OutputWeight,
            at,
            vec![geometry.hidden, query_width],
        )?;
        let wg = builder.external(
            TensorRole::GateWeight,
            at,
            vec![geometry.intermediate, geometry.hidden],
        )?;
        let wu = builder.external(
            TensorRole::UpWeight,
            at,
            vec![geometry.intermediate, geometry.hidden],
        )?;
        let wd = builder.external(
            TensorRole::DownWeight,
            at,
            vec![geometry.hidden, geometry.intermediate],
        )?;
        let prefix_shape = vec![
            batch,
            geometry.kv_heads,
            shape.context_capacity,
            geometry.head_dim,
        ];
        let key_prefix = builder.external(TensorRole::KeyPrefix, at, prefix_shape.clone())?;
        let value_prefix = builder.external(TensorRole::ValuePrefix, at, prefix_shape)?;
        let normalized = builder.operation(
            Operation::InputRmsNorm,
            at,
            &[hidden, input_norm],
            &[(TensorRole::NormalizedHidden, vec![batch, geometry.hidden])],
        )?[0];
        let q = builder.operation(
            Operation::QueryProjection,
            at,
            &[normalized, wq],
            &[(
                TensorRole::Query,
                vec![batch, geometry.query_heads, geometry.head_dim],
            )],
        )?[0];
        let k = builder.operation(
            Operation::KeyProjection,
            at,
            &[normalized, wk],
            &[(
                TensorRole::Key,
                vec![batch, geometry.kv_heads, geometry.head_dim],
            )],
        )?[0];
        let v = builder.operation(
            Operation::ValueProjection,
            at,
            &[normalized, wv],
            &[(
                TensorRole::Value,
                vec![batch, geometry.kv_heads, geometry.head_dim],
            )],
        )?[0];
        let qn = builder.operation(
            Operation::QueryRmsNorm,
            at,
            &[q, query_norm],
            &[(
                TensorRole::NormalizedQuery,
                vec![batch, geometry.query_heads, geometry.head_dim],
            )],
        )?[0];
        let kn = builder.operation(
            Operation::KeyRmsNorm,
            at,
            &[k, key_norm],
            &[(
                TensorRole::NormalizedKey,
                vec![batch, geometry.kv_heads, geometry.head_dim],
            )],
        )?[0];
        let rotated = builder.operation(
            Operation::Rope,
            at,
            &[qn, kn, positions],
            &[
                (
                    TensorRole::RotatedQuery,
                    vec![batch, geometry.query_heads, geometry.head_dim],
                ),
                (
                    TensorRole::RotatedKey,
                    vec![batch, geometry.kv_heads, geometry.head_dim],
                ),
            ],
        )?;
        let appended = builder.operation(
            Operation::KvAppend,
            at,
            &[rotated[1], v, positions],
            &[
                (
                    TensorRole::AppendedKey,
                    vec![batch, geometry.kv_heads, geometry.head_dim],
                ),
                (
                    TensorRole::AppendedValue,
                    vec![batch, geometry.kv_heads, geometry.head_dim],
                ),
            ],
        )?;
        let attention = builder.operation(
            Operation::Attention,
            at,
            &[
                rotated[0],
                key_prefix,
                value_prefix,
                appended[0],
                appended[1],
                positions,
            ],
            &[(
                TensorRole::Attention,
                vec![batch, geometry.query_heads, geometry.head_dim],
            )],
        )?[0];
        let residual = builder.operation(
            Operation::AttentionOutputResidual,
            at,
            &[attention, wo, hidden],
            &[(TensorRole::AttentionResidual, vec![batch, geometry.hidden])],
        )?[0];
        let post = builder.operation(
            Operation::PostAttentionRmsNorm,
            at,
            &[residual, post_norm],
            &[(TensorRole::PostNormalized, vec![batch, geometry.hidden])],
        )?[0];
        let gate = builder.operation(
            Operation::GateProjection,
            at,
            &[post, wg],
            &[(TensorRole::Gate, vec![batch, geometry.intermediate])],
        )?[0];
        let up = builder.operation(
            Operation::UpProjection,
            at,
            &[post, wu],
            &[(TensorRole::Up, vec![batch, geometry.intermediate])],
        )?[0];
        let activation = builder.operation(
            Operation::SwiGlu,
            at,
            &[gate, up],
            &[(TensorRole::Activated, vec![batch, geometry.intermediate])],
        )?[0];
        hidden = builder.operation(
            Operation::DownResidual,
            at,
            &[activation, wd, residual],
            &[(TensorRole::Hidden, vec![batch, geometry.hidden])],
        )?[0];
    }
    let normalized = builder.operation(
        Operation::FinalRmsNorm,
        None,
        &[hidden, final_norm],
        &[(TensorRole::FinalNormalized, vec![batch, geometry.hidden])],
    )?[0];
    let logits = builder.operation(
        Operation::LogitsProjection,
        None,
        &[normalized, language_head],
        &[(TensorRole::Logits, vec![batch, VOCABULARY_SIZE])],
    )?[0];
    builder.operation(
        Operation::Argmax,
        None,
        &[logits],
        &[(TensorRole::GreedyTokens, vec![batch])],
    )?;
    builder.declaration.ready_capacity = u32::try_from(builder.declaration.tasks.len())
        .map_err(|_| PlanError::ArithmeticOverflow)?;
    Ok(builder.declaration)
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
fn put_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
fn put_len(bytes: &mut Vec<u8>, length: usize) {
    // Records are already bounded by structural validation; a u64 length also
    // keeps serialization independent of the host pointer width.
    put_u64(
        bytes,
        u64::try_from(length).expect("bounded declaration length"),
    );
}
