use std::collections::BTreeSet;

use ferric_qwen3_gemm_device_v1::{
    QWEN3_VOCABULARY_SIZE_V1, qwen3_gemm_reference_profile_is_admitted_v1,
    qwen3_gemm_vectorized_profile_is_admitted_v1, qwen3_token_embedding_profile_is_admitted_v1,
};
use syn::{BinOp, Expr, Item, ItemFn, Lit, Pat, Stmt};

const BETA_ZERO: u32 = 0.0_f32.to_bits();
const BETA_ONE: u32 = 1.0_f32.to_bits();
const MODEL_VOCABULARY_SIZE: u32 = 151_936;
const SOURCE: &str = include_str!("../src/lib.rs");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PredicateValue {
    Boolean(bool),
    Integer(u32),
}

#[derive(Clone, Copy, Debug)]
struct PredicateInputs {
    m: u32,
    n: u32,
    k: u32,
    beta_bits: u32,
    rows: u32,
    hidden: u32,
    vocabulary: u32,
    target_shape_is_admitted: Option<bool>,
    draft_shape_is_admitted: Option<bool>,
}

fn evaluate_predicate(expression: &Expr, inputs: PredicateInputs) -> PredicateValue {
    match expression {
        Expr::Binary(binary) => {
            let left = evaluate_predicate(&binary.left, inputs);
            let right = evaluate_predicate(&binary.right, inputs);
            match binary.op {
                BinOp::Eq(_) => PredicateValue::Boolean(left == right),
                BinOp::And(_) => {
                    let (PredicateValue::Boolean(left), PredicateValue::Boolean(right)) =
                        (left, right)
                    else {
                        panic!("logical and requires boolean operands")
                    };
                    PredicateValue::Boolean(left && right)
                }
                BinOp::Or(_) => {
                    let (PredicateValue::Boolean(left), PredicateValue::Boolean(right)) =
                        (left, right)
                    else {
                        panic!("logical or requires boolean operands")
                    };
                    PredicateValue::Boolean(left || right)
                }
                _ => panic!("unsupported inline predicate operator"),
            }
        }
        Expr::Group(group) => evaluate_predicate(&group.expr, inputs),
        Expr::Paren(parenthesized) => evaluate_predicate(&parenthesized.expr, inputs),
        Expr::Lit(literal) => match &literal.lit {
            Lit::Int(integer) => PredicateValue::Integer(
                integer
                    .base10_parse()
                    .expect("predicate integer is an exact u32"),
            ),
            Lit::Bool(boolean) => PredicateValue::Boolean(boolean.value),
            _ => panic!("unsupported inline predicate literal"),
        },
        Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            let identifier = path.path.segments[0].ident.to_string();
            match identifier.as_str() {
                "m" => PredicateValue::Integer(inputs.m),
                "n" => PredicateValue::Integer(inputs.n),
                "k" => PredicateValue::Integer(inputs.k),
                "beta_bits" => PredicateValue::Integer(inputs.beta_bits),
                "rows" => PredicateValue::Integer(inputs.rows),
                "hidden" => PredicateValue::Integer(inputs.hidden),
                "vocabulary" => PredicateValue::Integer(inputs.vocabulary),
                "QWEN3_VOCABULARY_SIZE_V1" => PredicateValue::Integer(MODEL_VOCABULARY_SIZE),
                "target_shape_is_admitted" => PredicateValue::Boolean(
                    inputs
                        .target_shape_is_admitted
                        .expect("target shape predicate is evaluated first"),
                ),
                "draft_shape_is_admitted" => PredicateValue::Boolean(
                    inputs
                        .draft_shape_is_admitted
                        .expect("draft shape predicate is evaluated first"),
                ),
                unsupported => panic!("unsupported inline predicate path {unsupported}"),
            }
        }
        _ => panic!("unsupported inline predicate expression"),
    }
}

fn kernel(name: &str) -> ItemFn {
    syn::parse_file(SOURCE)
        .expect("device source parses as Rust")
        .items
        .into_iter()
        .find_map(|item| match item {
            Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing kernel {name}"))
}

fn local_initializer(function: &ItemFn, name: &str) -> Expr {
    function
        .block
        .stmts
        .iter()
        .find_map(|statement| match statement {
            Stmt::Local(local) => {
                let Pat::Ident(pattern) = &local.pat else {
                    return None;
                };
                (pattern.ident == name).then(|| {
                    local
                        .init
                        .as_ref()
                        .expect("predicate local has an initializer")
                        .expr
                        .as_ref()
                        .clone()
                })
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing local predicate {name}"))
}

struct InlineGemmPredicate {
    target: Expr,
    draft: Expr,
    profile: Expr,
}

impl InlineGemmPredicate {
    fn from_kernel(name: &str) -> Self {
        let function = kernel(name);
        Self {
            target: local_initializer(&function, "target_shape_is_admitted"),
            draft: local_initializer(&function, "draft_shape_is_admitted"),
            profile: local_initializer(&function, "profile_is_admitted"),
        }
    }

    fn admits(&self, tuple: GemmTuple) -> bool {
        let mut inputs = PredicateInputs {
            m: tuple.m,
            n: tuple.n,
            k: tuple.k,
            beta_bits: tuple.beta_bits,
            rows: 0,
            hidden: 0,
            vocabulary: 0,
            target_shape_is_admitted: None,
            draft_shape_is_admitted: None,
        };
        let PredicateValue::Boolean(target) = evaluate_predicate(&self.target, inputs) else {
            panic!("target shape predicate did not produce bool")
        };
        inputs.target_shape_is_admitted = Some(target);
        let PredicateValue::Boolean(draft) = evaluate_predicate(&self.draft, inputs) else {
            panic!("draft shape predicate did not produce bool")
        };
        inputs.draft_shape_is_admitted = Some(draft);
        let PredicateValue::Boolean(profile) = evaluate_predicate(&self.profile, inputs) else {
            panic!("profile predicate did not produce bool")
        };
        profile
    }
}

struct InlineEmbeddingPredicate {
    profile: Expr,
}

impl InlineEmbeddingPredicate {
    fn current() -> Self {
        let function = kernel("ferric_qwen3_token_embedding_bf16_copy_v1");
        Self {
            profile: local_initializer(&function, "profile_is_admitted"),
        }
    }

    fn admits(&self, tuple: EmbeddingTuple) -> bool {
        let inputs = PredicateInputs {
            m: 0,
            n: 0,
            k: 0,
            beta_bits: 0,
            rows: tuple.rows,
            hidden: tuple.hidden,
            vocabulary: tuple.vocabulary,
            target_shape_is_admitted: None,
            draft_shape_is_admitted: None,
        };
        let PredicateValue::Boolean(profile) = evaluate_predicate(&self.profile, inputs) else {
            panic!("embedding profile predicate did not produce bool")
        };
        profile
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Role {
    Target8B,
    Draft06B,
}

impl Role {
    const ALL: [Self; 2] = [Self::Target8B, Self::Draft06B];

    const fn hidden(self) -> u32 {
        match self {
            Self::Target8B => 4_096,
            Self::Draft06B => 1_024,
        }
    }

    const fn intermediate(self) -> u32 {
        match self {
            Self::Target8B => 12_288,
            Self::Draft06B => 3_072,
        }
    }

    const fn query_width(self) -> u32 {
        match self {
            Self::Target8B => 4_096,
            Self::Draft06B => 2_048,
        }
    }

    const fn kv_width(self) -> u32 {
        1_024
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Bucket {
    PrefillS1T128,
    PrefillS8T128,
    PrefillS1T512,
    PrefillS1T2048,
    DecodeS1C8192,
    DecodeS8C8192,
    DecodeS32C8192,
    SpeculativeS1K4C8192,
    SpeculativeS8K4C8192,
    SpeculativeS1K8C8192,
    SpeculativeS1K16C8192,
}

impl Bucket {
    const ALL: [Self; 11] = [
        Self::PrefillS1T128,
        Self::PrefillS8T128,
        Self::PrefillS1T512,
        Self::PrefillS1T2048,
        Self::DecodeS1C8192,
        Self::DecodeS8C8192,
        Self::DecodeS32C8192,
        Self::SpeculativeS1K4C8192,
        Self::SpeculativeS8K4C8192,
        Self::SpeculativeS1K8C8192,
        Self::SpeculativeS1K16C8192,
    ];

    const fn sequence_and_active_tokens(self, role: Role) -> [u32; 2] {
        match self {
            Self::PrefillS1T128 => [1, 128],
            Self::PrefillS8T128 => [8, 128],
            Self::PrefillS1T512 => [1, 512],
            Self::PrefillS1T2048 => [1, 2_048],
            Self::DecodeS1C8192 => [1, 1],
            Self::DecodeS8C8192 => [8, 1],
            Self::DecodeS32C8192 => [32, 1],
            Self::SpeculativeS1K4C8192 => match role {
                Role::Target8B => [1, 5],
                Role::Draft06B => [1, 4],
            },
            Self::SpeculativeS8K4C8192 => match role {
                Role::Target8B => [8, 5],
                Role::Draft06B => [8, 4],
            },
            Self::SpeculativeS1K8C8192 => match role {
                Role::Target8B => [1, 9],
                Role::Draft06B => [1, 8],
            },
            Self::SpeculativeS1K16C8192 => match role {
                Role::Target8B => [1, 17],
                Role::Draft06B => [1, 16],
            },
        }
    }

    const fn flattened_rows(self, role: Role) -> u32 {
        let [sequences, active_tokens] = self.sequence_and_active_tokens(role);
        sequences * active_tokens
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Operation {
    QueryProjection,
    KeyProjection,
    ValueProjection,
    AttentionOutputResidual,
    GateProjection,
    UpProjection,
    DownResidual,
    LogitsProjection,
}

impl Operation {
    const ALL: [Self; 8] = [
        Self::QueryProjection,
        Self::KeyProjection,
        Self::ValueProjection,
        Self::AttentionOutputResidual,
        Self::GateProjection,
        Self::UpProjection,
        Self::DownResidual,
        Self::LogitsProjection,
    ];

    const fn dimensions(self, role: Role, m: u32) -> [u32; 3] {
        let hidden = role.hidden();
        match self {
            Self::QueryProjection => [m, role.query_width(), hidden],
            Self::KeyProjection | Self::ValueProjection => [m, role.kv_width(), hidden],
            Self::AttentionOutputResidual => [m, hidden, role.query_width()],
            Self::GateProjection | Self::UpProjection => [m, role.intermediate(), hidden],
            Self::DownResidual => [m, hidden, role.intermediate()],
            Self::LogitsProjection => [m, MODEL_VOCABULARY_SIZE, hidden],
        }
    }

    const fn beta_bits(self) -> u32 {
        match self {
            Self::AttentionOutputResidual | Self::DownResidual => BETA_ONE,
            _ => BETA_ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct GemmIdentity {
    role: Role,
    bucket: Bucket,
    operation: Operation,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct GemmTuple {
    m: u32,
    n: u32,
    k: u32,
    beta_bits: u32,
}

impl GemmIdentity {
    const fn runtime_tuple(self) -> GemmTuple {
        let m = self.bucket.flattened_rows(self.role);
        let [m, n, k] = self.operation.dimensions(self.role, m);
        GemmTuple {
            m,
            n,
            k,
            beta_bits: self.operation.beta_bits(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EmbeddingIdentity {
    role: Role,
    bucket: Bucket,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EmbeddingTuple {
    rows: u32,
    hidden: u32,
    vocabulary: u32,
}

impl EmbeddingIdentity {
    const fn runtime_tuple(self) -> EmbeddingTuple {
        EmbeddingTuple {
            rows: self.bucket.flattened_rows(self.role),
            hidden: self.role.hidden(),
            vocabulary: MODEL_VOCABULARY_SIZE,
        }
    }
}

fn gemm_identities() -> Vec<GemmIdentity> {
    let mut identities = Vec::new();
    for role in Role::ALL {
        for bucket in Bucket::ALL {
            for operation in Operation::ALL {
                identities.push(GemmIdentity {
                    role,
                    bucket,
                    operation,
                });
            }
        }
    }
    identities
}

fn embedding_identities() -> Vec<EmbeddingIdentity> {
    let mut identities = Vec::new();
    for role in Role::ALL {
        for bucket in Bucket::ALL {
            identities.push(EmbeddingIdentity { role, bucket });
        }
    }
    identities
}

fn reference_admits(tuple: GemmTuple) -> bool {
    qwen3_gemm_reference_profile_is_admitted_v1(tuple.m, tuple.n, tuple.k, tuple.beta_bits)
}

fn vector_admits(tuple: GemmTuple) -> bool {
    qwen3_gemm_vectorized_profile_is_admitted_v1(tuple.m, tuple.n, tuple.k, tuple.beta_bits)
}

fn embedding_admits(tuple: EmbeddingTuple) -> bool {
    qwen3_token_embedding_profile_is_admitted_v1(tuple.rows, tuple.hidden, tuple.vocabulary)
}

#[test]
fn all_176_gemm_identities_project_to_the_exact_schedule_partition() {
    assert_eq!(QWEN3_VOCABULARY_SIZE_V1, MODEL_VOCABULARY_SIZE);
    let inline_reference =
        InlineGemmPredicate::from_kernel("ferric_qwen3_gemm_reference_bf16_f32_bf16_v1");
    let inline_vector =
        InlineGemmPredicate::from_kernel("ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1");
    let identities = gemm_identities();
    assert_eq!(identities.len(), 2 * 11 * 8);
    assert_eq!(
        identities.iter().copied().collect::<BTreeSet<_>>().len(),
        176
    );
    assert_eq!(
        identities
            .iter()
            .filter(|identity| identity.runtime_tuple().m < 16)
            .count(),
        64
    );
    assert_eq!(
        identities
            .iter()
            .filter(|identity| identity.runtime_tuple().m >= 16)
            .count(),
        112
    );

    let runtime_tuples = identities
        .iter()
        .map(|identity| identity.runtime_tuple())
        .collect::<BTreeSet<_>>();
    let reference = runtime_tuples
        .iter()
        .copied()
        .filter(|tuple| tuple.m < 16)
        .collect::<BTreeSet<_>>();
    let vector = runtime_tuples
        .iter()
        .copied()
        .filter(|tuple| tuple.m >= 16)
        .collect::<BTreeSet<_>>();

    assert_eq!(runtime_tuples.len(), 120);
    assert_eq!(reference.len(), 42);
    assert_eq!(vector.len(), 78);
    assert!(reference.is_disjoint(&vector));

    for identity in identities {
        let tuple = identity.runtime_tuple();
        let expected_reference = tuple.m < 16;
        assert_eq!(
            reference_admits(tuple),
            expected_reference,
            "reference classifier drifted for {identity:?} -> {tuple:?}"
        );
        assert_eq!(
            vector_admits(tuple),
            !expected_reference,
            "A4 classifier drifted for {identity:?} -> {tuple:?}"
        );
        assert_eq!(
            inline_reference.admits(tuple),
            expected_reference,
            "inline reference predicate drifted for {identity:?} -> {tuple:?}"
        );
        assert_eq!(
            inline_vector.admits(tuple),
            !expected_reference,
            "inline A4 predicate drifted for {identity:?} -> {tuple:?}"
        );
    }
}

#[test]
fn all_22_embedding_identities_project_to_20_exact_runtime_tuples() {
    let inline_embedding = InlineEmbeddingPredicate::current();
    let identities = embedding_identities();
    assert_eq!(identities.len(), 2 * 11);
    assert_eq!(
        identities.iter().copied().collect::<BTreeSet<_>>().len(),
        22
    );

    let runtime_tuples = identities
        .iter()
        .map(|identity| identity.runtime_tuple())
        .collect::<BTreeSet<_>>();
    assert_eq!(runtime_tuples.len(), 20);

    for identity in identities {
        let tuple = identity.runtime_tuple();
        assert!(
            embedding_admits(tuple),
            "embedding classifier drifted for {identity:?} -> {tuple:?}"
        );
        assert!(
            inline_embedding.admits(tuple),
            "inline embedding predicate drifted for {identity:?} -> {tuple:?}"
        );
    }
}

fn with_adjacent_candidates(values: impl IntoIterator<Item = u32>) -> BTreeSet<u32> {
    let mut candidates = BTreeSet::from([0, 2, 3, u32::MAX]);
    for value in values {
        candidates.insert(value);
        if let Some(previous) = value.checked_sub(1) {
            candidates.insert(previous);
        }
        if let Some(next) = value.checked_add(1) {
            candidates.insert(next);
        }
    }
    candidates
}

#[test]
fn source_classifiers_equal_the_authoritative_model_over_a_finite_candidate_grid() {
    let inline_reference =
        InlineGemmPredicate::from_kernel("ferric_qwen3_gemm_reference_bf16_f32_bf16_v1");
    let inline_vector =
        InlineGemmPredicate::from_kernel("ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1");
    let inline_embedding = InlineEmbeddingPredicate::current();
    let gemm_tuples = gemm_identities()
        .into_iter()
        .map(GemmIdentity::runtime_tuple)
        .collect::<BTreeSet<_>>();
    let expected_reference = gemm_tuples
        .iter()
        .copied()
        .filter(|tuple| tuple.m < 16)
        .collect::<BTreeSet<_>>();
    let expected_vector = gemm_tuples
        .iter()
        .copied()
        .filter(|tuple| tuple.m >= 16)
        .collect::<BTreeSet<_>>();
    let m_candidates = with_adjacent_candidates(gemm_tuples.iter().map(|tuple| tuple.m));
    let n_candidates = with_adjacent_candidates(gemm_tuples.iter().map(|tuple| tuple.n));
    let k_candidates = with_adjacent_candidates(gemm_tuples.iter().map(|tuple| tuple.k));
    let beta_candidates = BTreeSet::from([BETA_ZERO, BETA_ONE, 1, u32::MAX]);

    for m in m_candidates {
        for n in &n_candidates {
            for k in &k_candidates {
                for beta_bits in &beta_candidates {
                    let tuple = GemmTuple {
                        m,
                        n: *n,
                        k: *k,
                        beta_bits: *beta_bits,
                    };
                    assert_eq!(
                        reference_admits(tuple),
                        expected_reference.contains(&tuple),
                        "reference candidate-grid mismatch for {tuple:?}"
                    );
                    assert_eq!(
                        vector_admits(tuple),
                        expected_vector.contains(&tuple),
                        "A4 candidate-grid mismatch for {tuple:?}"
                    );
                    assert_eq!(
                        inline_reference.admits(tuple),
                        expected_reference.contains(&tuple),
                        "inline reference candidate-grid mismatch for {tuple:?}"
                    );
                    assert_eq!(
                        inline_vector.admits(tuple),
                        expected_vector.contains(&tuple),
                        "inline A4 candidate-grid mismatch for {tuple:?}"
                    );
                }
            }
        }
    }

    let embedding_tuples = embedding_identities()
        .into_iter()
        .map(EmbeddingIdentity::runtime_tuple)
        .collect::<BTreeSet<_>>();
    let rows_candidates = with_adjacent_candidates(embedding_tuples.iter().map(|tuple| tuple.rows));
    let hidden_candidates =
        with_adjacent_candidates(embedding_tuples.iter().map(|tuple| tuple.hidden));
    let vocabulary_candidates =
        with_adjacent_candidates(embedding_tuples.iter().map(|tuple| tuple.vocabulary));
    for rows in rows_candidates {
        for hidden in &hidden_candidates {
            for vocabulary in &vocabulary_candidates {
                let tuple = EmbeddingTuple {
                    rows,
                    hidden: *hidden,
                    vocabulary: *vocabulary,
                };
                assert_eq!(
                    embedding_admits(tuple),
                    embedding_tuples.contains(&tuple),
                    "embedding candidate-grid mismatch for {tuple:?}"
                );
                assert_eq!(
                    inline_embedding.admits(tuple),
                    embedding_tuples.contains(&tuple),
                    "inline embedding candidate-grid mismatch for {tuple:?}"
                );
            }
        }
    }
}

#[test]
fn one_field_substitutions_and_cross_role_tuples_fail_closed() {
    let gemm_tuples = gemm_identities()
        .into_iter()
        .map(GemmIdentity::runtime_tuple)
        .collect::<BTreeSet<_>>();
    for tuple in gemm_tuples {
        let substitutions = [
            GemmTuple {
                m: u32::MAX,
                ..tuple
            },
            GemmTuple {
                n: u32::MAX,
                ..tuple
            },
            GemmTuple {
                k: u32::MAX,
                ..tuple
            },
            GemmTuple {
                beta_bits: u32::MAX,
                ..tuple
            },
        ];
        for hostile in substitutions {
            assert!(!reference_admits(hostile), "accepted hostile {hostile:?}");
            assert!(!vector_admits(hostile), "accepted hostile {hostile:?}");
        }
    }

    for hostile in [
        GemmTuple {
            m: 5,
            n: 2_048,
            k: 1_024,
            beta_bits: BETA_ZERO,
        },
        GemmTuple {
            m: 4,
            n: 4_096,
            k: 4_096,
            beta_bits: BETA_ZERO,
        },
        GemmTuple {
            m: 17,
            n: 2_048,
            k: 1_024,
            beta_bits: BETA_ZERO,
        },
        GemmTuple {
            m: 16,
            n: 4_096,
            k: 4_096,
            beta_bits: BETA_ZERO,
        },
    ] {
        assert!(
            !reference_admits(hostile),
            "accepted cross-role {hostile:?}"
        );
        assert!(!vector_admits(hostile), "accepted cross-role {hostile:?}");
    }

    let embedding_tuples = embedding_identities()
        .into_iter()
        .map(EmbeddingIdentity::runtime_tuple)
        .collect::<BTreeSet<_>>();
    for tuple in embedding_tuples {
        for hostile in [
            EmbeddingTuple {
                rows: u32::MAX,
                ..tuple
            },
            EmbeddingTuple {
                hidden: u32::MAX,
                ..tuple
            },
            EmbeddingTuple {
                vocabulary: u32::MAX,
                ..tuple
            },
        ] {
            assert!(!embedding_admits(hostile), "accepted hostile {hostile:?}");
        }
    }

    for hostile in [
        EmbeddingTuple {
            rows: 5,
            hidden: 1_024,
            vocabulary: MODEL_VOCABULARY_SIZE,
        },
        EmbeddingTuple {
            rows: 4,
            hidden: 4_096,
            vocabulary: MODEL_VOCABULARY_SIZE,
        },
    ] {
        assert!(
            !embedding_admits(hostile),
            "accepted cross-role {hostile:?}"
        );
    }
}
