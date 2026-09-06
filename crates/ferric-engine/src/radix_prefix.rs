//! Bounded live-source radix prefix reuse for logical Engine KV pages.
//!
//! This module indexes authenticated prompt tokens and can share only complete
//! logical pages from a source request that is still live, ready, committed,
//! and initialized. The source registration type is deliberately sealed and
//! is minted only from the completed paired-prefill success owner; callers
//! cannot turn token or KV claims into registration authority.
//!
//! This first slice does not retain KV after source retirement, schedule suffix
//! or chunked prefill, persist physical cache owners, serve Qwen, or establish
//! any timing, allocation, KFD, device-memory, or kernel correctness claim. A
//! logical Engine page is not a claim about the device-cache page domain.

use core::fmt;

use ferric_spec::scheduling::RequestState;
use ferric_spec::{RequestId, TokenId, M1_MAX_CONTEXT_TOKENS};

use crate::{Engine, EngineError, M1AuthenticatedS1T128PrefillExecutionSuccessV1};

const ROOT_NODE: usize = 0;
const NO_PARENT: usize = usize::MAX;

/// One independently bounded radix-index resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RadixPrefixLimitV1 {
    Nodes,
    TokensPerSource,
    Sources,
    StoredTokens,
}

/// Rejection while fixing the index's complete storage envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RadixPrefixLimitsErrorV1 {
    Zero(M1RadixPrefixLimitV1),
    RootRequiresNode,
    TokensExceedM1Context { maximum: usize },
    StorageArithmeticOverflow,
}

/// Fixed resource limits for one prefix index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1RadixPrefixLimitsV1 {
    nodes: usize,
    tokens_per_source: usize,
    sources: usize,
    stored_tokens: usize,
}

impl M1RadixPrefixLimitsV1 {
    /// Validates all independent bounds before any index storage is allocated.
    ///
    /// # Errors
    ///
    /// Rejects zero limits, a missing root slot, a per-source token limit above
    /// the M1 context envelope, or bounds whose maximum logical storage cannot
    /// be represented.
    pub fn new(
        node_limit: usize,
        tokens_per_source_limit: usize,
        source_limit: usize,
        stored_token_limit: usize,
    ) -> Result<Self, M1RadixPrefixLimitsErrorV1> {
        for (kind, value) in [
            (M1RadixPrefixLimitV1::Nodes, node_limit),
            (
                M1RadixPrefixLimitV1::TokensPerSource,
                tokens_per_source_limit,
            ),
            (M1RadixPrefixLimitV1::Sources, source_limit),
            (M1RadixPrefixLimitV1::StoredTokens, stored_token_limit),
        ] {
            if value == 0 {
                return Err(M1RadixPrefixLimitsErrorV1::Zero(kind));
            }
        }
        if node_limit < 2 {
            return Err(M1RadixPrefixLimitsErrorV1::RootRequiresNode);
        }
        if tokens_per_source_limit > M1_MAX_CONTEXT_TOKENS as usize {
            return Err(M1RadixPrefixLimitsErrorV1::TokensExceedM1Context {
                maximum: tokens_per_source_limit,
            });
        }
        let _maximum_logical_tokens = tokens_per_source_limit
            .checked_mul(source_limit)
            .ok_or(M1RadixPrefixLimitsErrorV1::StorageArithmeticOverflow)?;
        Ok(Self {
            nodes: node_limit,
            tokens_per_source: tokens_per_source_limit,
            sources: source_limit,
            stored_tokens: stored_token_limit,
        })
    }

    #[must_use]
    pub const fn node_limit(self) -> usize {
        self.nodes
    }

    #[must_use]
    pub const fn tokens_per_source_limit(self) -> usize {
        self.tokens_per_source
    }

    #[must_use]
    pub const fn source_limit(self) -> usize {
        self.sources
    }

    #[must_use]
    pub const fn stored_token_limit(self) -> usize {
        self.stored_tokens
    }
}

/// Index construction or authenticated-registration rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RadixPrefixIndexErrorV1 {
    Allocation,
    EmptySource,
    ZeroSourceGeneration,
    TokensPerSourceCapacity { limit: usize, requested: usize },
    SourceCapacity { limit: usize },
    StoredTokenCapacity { limit: usize, requested: usize },
    NodeCapacity { limit: usize, requested: usize },
    TicketExhausted,
    InvariantViolation,
}

/// Failure to copy or index one completed authenticated prefill source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedRadixPrefixRegistrationErrorV1 {
    HostAllocation,
    Index(M1RadixPrefixIndexErrorV1),
}

/// Registration rejection retaining the exact rollover-ready prefill success.
#[must_use = "rollover-ready authenticated prefill custody remains retained"]
pub struct M1AuthenticatedRadixPrefixRegistrationFailureV1<const C: usize> {
    error: M1AuthenticatedRadixPrefixRegistrationErrorV1,
    success: M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedRadixPrefixRegistrationFailureV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedRadixPrefixRegistrationFailureV1")
            .field("error", &self.error)
            .field("success", &self.success)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedRadixPrefixRegistrationFailureV1<C> {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedRadixPrefixRegistrationErrorV1 {
        self.error
    }

    #[must_use = "the same rollover-ready success remains retained"]
    pub const fn success(&self) -> &M1AuthenticatedS1T128PrefillExecutionSuccessV1<C> {
        &self.success
    }

    /// Recovers the unchanged rollover-ready owner without exposing prefix
    /// registration authority.
    #[must_use = "rollover-ready authenticated prefill custody remains linear"]
    pub fn into_success(self) -> M1AuthenticatedS1T128PrefillExecutionSuccessV1<C> {
        self.success
    }
}

/// Move-only proof that one live request's token sequence came from completed
/// authenticated prefill custody.
///
/// This type intentionally has no public constructor or decomposition method.
/// The integrated paired-prefill executor is the only production minting
/// route, so callers cannot register token or KV claims.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedRadixPrefixSourceV1;
/// use ferric_spec::RequestId;
/// let _forged = M1AuthenticatedRadixPrefixSourceV1 {
///     source: RequestId::new(0, 1),
///     tokens: vec![1; 16],
/// };
/// ```
#[must_use = "authenticated live-source prefix authority remains linear"]
pub struct M1AuthenticatedRadixPrefixSourceV1 {
    source: RequestId,
    tokens: Vec<TokenId>,
}

impl fmt::Debug for M1AuthenticatedRadixPrefixSourceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedRadixPrefixSourceV1")
            .field("source", &self.source)
            .field("token_count", &self.tokens.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug)]
struct RadixNodeV1 {
    token: TokenId,
    parent: usize,
    first_child: Option<usize>,
    next_sibling: Option<usize>,
    depth: usize,
}

impl RadixNodeV1 {
    const ROOT: Self = Self {
        token: 0,
        parent: NO_PARENT,
        first_child: None,
        next_sibling: None,
        depth: 0,
    };
}

#[derive(Clone, Copy, Debug)]
struct RadixSourceV1 {
    request: RequestId,
    endpoint: usize,
    token_count: usize,
    ticket: u64,
}

/// A deterministic full-page match. The generational source identity remains
/// subject to Engine validation immediately before sharing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1RadixPrefixMatchV1 {
    source: RequestId,
    token_count: u32,
}

impl M1RadixPrefixMatchV1 {
    #[must_use]
    pub const fn source(self) -> RequestId {
        self.source
    }

    #[must_use]
    pub const fn token_count(self) -> u32 {
        self.token_count
    }
}

/// Fixed-capacity token trie plus fixed-capacity source records.
///
/// Nodes are never allocated after construction. Source replacement,
/// invalidation, and eviction prune unreferenced leaf paths for deterministic
/// reuse of the lowest available node and source slots.
#[derive(Debug)]
pub struct M1RadixPrefixIndexV1 {
    limits: M1RadixPrefixLimitsV1,
    nodes: Vec<Option<RadixNodeV1>>,
    sources: Vec<Option<RadixSourceV1>>,
    node_count: usize,
    source_count: usize,
    stored_tokens: usize,
    next_ticket: u64,
}

impl M1RadixPrefixIndexV1 {
    /// Allocates the index's complete node and source arrays once.
    ///
    /// # Errors
    ///
    /// Returns [`M1RadixPrefixIndexErrorV1::Allocation`] if either exact
    /// fixed-capacity reservation fails.
    pub fn new(limits: M1RadixPrefixLimitsV1) -> Result<Self, M1RadixPrefixIndexErrorV1> {
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(limits.nodes)
            .map_err(|_| M1RadixPrefixIndexErrorV1::Allocation)?;
        for _ in 0..limits.nodes {
            nodes.push(None);
        }
        nodes[ROOT_NODE] = Some(RadixNodeV1::ROOT);

        let mut sources = Vec::new();
        sources
            .try_reserve_exact(limits.sources)
            .map_err(|_| M1RadixPrefixIndexErrorV1::Allocation)?;
        for _ in 0..limits.sources {
            sources.push(None);
        }

        Ok(Self {
            limits,
            nodes,
            sources,
            node_count: 1,
            source_count: 0,
            stored_tokens: 0,
            next_ticket: 1,
        })
    }

    #[must_use]
    pub const fn limits(&self) -> M1RadixPrefixLimitsV1 {
        self.limits
    }

    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    #[must_use]
    pub const fn source_count(&self) -> usize {
        self.source_count
    }

    #[must_use]
    pub const fn stored_tokens(&self) -> usize {
        self.stored_tokens
    }

    /// Consumes authenticated prompt/source authority into the bounded index.
    ///
    /// The same request slot replaces any older generation. The same exact
    /// token sequence replaces its prior source. All capacity checks complete
    /// before either record is removed, so every error leaves the index
    /// unchanged. This method is usable only after a trusted executor mints its
    /// sealed argument.
    ///
    /// # Errors
    ///
    /// Rejects empty or oversized tokens, invalid generation zero, exhausted
    /// node/source/logical-storage capacity, ticket exhaustion, or an internal
    /// structural inconsistency.
    ///
    /// # Panics
    ///
    /// Panics only if this type's private fixed-array and radix-link invariants
    /// disagree after the complete mutation preflight succeeds.
    pub fn register_authenticated_source(
        &mut self,
        authority: M1AuthenticatedRadixPrefixSourceV1,
    ) -> Result<(), M1RadixPrefixIndexErrorV1> {
        let M1AuthenticatedRadixPrefixSourceV1 { source, tokens } = authority;
        if tokens.is_empty() {
            return Err(M1RadixPrefixIndexErrorV1::EmptySource);
        }
        if source.generation() == 0 {
            return Err(M1RadixPrefixIndexErrorV1::ZeroSourceGeneration);
        }
        if tokens.len() > self.limits.tokens_per_source {
            return Err(M1RadixPrefixIndexErrorV1::TokensPerSourceCapacity {
                limit: self.limits.tokens_per_source,
                requested: tokens.len(),
            });
        }
        let next_ticket = self
            .next_ticket
            .checked_add(1)
            .ok_or(M1RadixPrefixIndexErrorV1::TicketExhausted)?;

        let same_slot = self
            .sources
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.request.slot() == source.slot()));
        let duplicate_prefix = self
            .sources
            .iter()
            .position(|entry| entry.is_some_and(|entry| self.source_tokens_equal(entry, &tokens)));
        let victims = [
            same_slot,
            duplicate_prefix.filter(|slot| Some(*slot) != same_slot),
        ];

        let mut victim_count = 0usize;
        let mut victim_tokens = 0usize;
        for slot in victims.into_iter().flatten() {
            let Some(entry) = self.sources[slot] else {
                return Err(M1RadixPrefixIndexErrorV1::InvariantViolation);
            };
            victim_count = victim_count
                .checked_add(1)
                .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
            victim_tokens = victim_tokens
                .checked_add(entry.token_count)
                .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        }
        let retained_sources = self
            .source_count
            .checked_sub(victim_count)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        if retained_sources >= self.limits.sources {
            return Err(M1RadixPrefixIndexErrorV1::SourceCapacity {
                limit: self.limits.sources,
            });
        }
        let retained_tokens = self
            .stored_tokens
            .checked_sub(victim_tokens)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        let requested_tokens = retained_tokens.checked_add(tokens.len()).ok_or(
            M1RadixPrefixIndexErrorV1::StoredTokenCapacity {
                limit: self.limits.stored_tokens,
                requested: usize::MAX,
            },
        )?;
        if requested_tokens > self.limits.stored_tokens {
            return Err(M1RadixPrefixIndexErrorV1::StoredTokenCapacity {
                limit: self.limits.stored_tokens,
                requested: requested_tokens,
            });
        }

        let (existing_parent, existing_depth) = self.existing_prefix(&tokens)?;
        let missing_nodes = tokens.len() - existing_depth;
        let free_nodes = self.limits.nodes - self.node_count;
        if missing_nodes > free_nodes {
            return Err(M1RadixPrefixIndexErrorV1::NodeCapacity {
                limit: self.limits.nodes,
                requested: self.node_count.saturating_add(missing_nodes),
            });
        }

        let mut victim_endpoints = [None, None];
        for (position, slot) in victims.into_iter().flatten().enumerate() {
            let entry = self.sources[slot]
                .take()
                .expect("private source preflight retained every replacement victim");
            victim_endpoints[position] = Some(entry.endpoint);
        }
        self.source_count = retained_sources;
        self.stored_tokens = retained_tokens;

        let mut endpoint = existing_parent;
        for &token in &tokens[existing_depth..] {
            endpoint = self
                .insert_child(endpoint, token)
                .expect("private node preflight reserved every missing radix node");
        }
        let source_slot = self
            .sources
            .iter()
            .position(Option::is_none)
            .expect("private source preflight reserved one registration slot");
        self.sources[source_slot] = Some(RadixSourceV1 {
            request: source,
            endpoint,
            token_count: tokens.len(),
            ticket: self.next_ticket,
        });
        self.source_count = retained_sources + 1;
        self.stored_tokens = requested_tokens;
        self.next_ticket = next_ticket;

        for endpoint in victim_endpoints.into_iter().flatten() {
            self.prune_from(endpoint)
                .expect("private radix links remain valid while pruning replaced paths");
        }
        Ok(())
    }

    /// Returns the deterministic longest match rounded down to complete pages
    /// of the supplied logical Engine width. At an equal page depth, the
    /// smallest generational `RequestId` wins.
    #[must_use]
    pub fn longest_page_aligned_match(
        &self,
        tokens: &[TokenId],
        logical_page_tokens: u32,
    ) -> Option<M1RadixPrefixMatchV1> {
        if logical_page_tokens == 0 {
            return None;
        }
        let page_tokens = logical_page_tokens as usize;
        let mut node = ROOT_NODE;
        let mut best = None;
        for &token in tokens {
            let Some(child) = self.find_child(node, token) else {
                break;
            };
            node = child;
            let depth = self.nodes[node]?.depth;
            if depth.is_multiple_of(page_tokens) {
                if let Some(source) = self.best_source_below(node) {
                    best = Some(M1RadixPrefixMatchV1 {
                        source,
                        token_count: u32::try_from(depth).ok()?,
                    });
                }
            }
        }
        best
    }

    /// Removes only the exact generational source identity.
    ///
    /// A stale retirement notification cannot remove a newer generation in the
    /// same scheduler slot.
    ///
    /// # Errors
    ///
    /// Returns an invariant error only if retained counters disagree with the
    /// fixed source or node arrays.
    pub fn invalidate_source(
        &mut self,
        source: RequestId,
    ) -> Result<bool, M1RadixPrefixIndexErrorV1> {
        let Some(slot) = self
            .sources
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.request == source))
        else {
            return Ok(false);
        };
        self.remove_source_slot(slot)?;
        Ok(true)
    }

    /// Evicts the oldest registration, using `RequestId` as a stable tie-break.
    ///
    /// # Errors
    ///
    /// Returns an invariant error only if retained counters disagree with the
    /// fixed source array.
    pub fn evict_oldest(&mut self) -> Result<Option<RequestId>, M1RadixPrefixIndexErrorV1> {
        let oldest = self
            .sources
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| entry.map(|entry| (slot, entry)))
            .min_by_key(|(_, entry)| (entry.ticket, entry.request));
        let Some((slot, entry)) = oldest else {
            if self.source_count != 0 {
                return Err(M1RadixPrefixIndexErrorV1::InvariantViolation);
            }
            return Ok(None);
        };
        self.remove_source_slot(slot)?;
        Ok(Some(entry.request))
    }

    fn existing_prefix(
        &self,
        tokens: &[TokenId],
    ) -> Result<(usize, usize), M1RadixPrefixIndexErrorV1> {
        let mut node = ROOT_NODE;
        let mut depth = 0usize;
        for &token in tokens {
            let Some(child) = self.find_child(node, token) else {
                break;
            };
            node = child;
            depth = depth
                .checked_add(1)
                .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        }
        Ok((node, depth))
    }

    fn find_child(&self, parent: usize, token: TokenId) -> Option<usize> {
        let mut child = self.nodes.get(parent)?.as_ref()?.first_child;
        while let Some(index) = child {
            let node = self.nodes.get(index)?.as_ref()?;
            if node.token == token {
                return Some(index);
            }
            child = node.next_sibling;
        }
        None
    }

    fn insert_child(
        &mut self,
        parent: usize,
        token: TokenId,
    ) -> Result<usize, M1RadixPrefixIndexErrorV1> {
        let slot = self
            .nodes
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, node)| node.is_none().then_some(index))
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        let parent_node = self
            .nodes
            .get(parent)
            .and_then(Option::as_ref)
            .copied()
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        let depth = parent_node
            .depth
            .checked_add(1)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        self.nodes[slot] = Some(RadixNodeV1 {
            token,
            parent,
            first_child: None,
            next_sibling: parent_node.first_child,
            depth,
        });
        self.nodes[parent]
            .as_mut()
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?
            .first_child = Some(slot);
        self.node_count = self
            .node_count
            .checked_add(1)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        Ok(slot)
    }

    fn source_tokens_equal(&self, source: RadixSourceV1, tokens: &[TokenId]) -> bool {
        if source.token_count != tokens.len() {
            return false;
        }
        let mut node = source.endpoint;
        for &token in tokens.iter().rev() {
            let Some(entry) = self.nodes.get(node).and_then(Option::as_ref) else {
                return false;
            };
            if entry.token != token {
                return false;
            }
            node = entry.parent;
        }
        node == ROOT_NODE
    }

    fn best_source_below(&self, ancestor: usize) -> Option<RequestId> {
        self.sources
            .iter()
            .filter_map(Option::as_ref)
            .filter(|source| self.node_is_ancestor(ancestor, source.endpoint))
            .map(|source| source.request)
            .min()
    }

    fn node_is_ancestor(&self, ancestor: usize, mut node: usize) -> bool {
        let Some(ancestor_depth) = self
            .nodes
            .get(ancestor)
            .and_then(Option::as_ref)
            .map(|node| node.depth)
        else {
            return false;
        };
        loop {
            let Some(entry) = self.nodes.get(node).and_then(Option::as_ref) else {
                return false;
            };
            if entry.depth < ancestor_depth {
                return false;
            }
            if node == ancestor {
                return true;
            }
            node = entry.parent;
        }
    }

    fn remove_source_slot(&mut self, slot: usize) -> Result<(), M1RadixPrefixIndexErrorV1> {
        let source = self
            .sources
            .get_mut(slot)
            .and_then(Option::take)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        self.source_count = self
            .source_count
            .checked_sub(1)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        self.stored_tokens = self
            .stored_tokens
            .checked_sub(source.token_count)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
        self.prune_from(source.endpoint)
    }

    fn prune_from(&mut self, mut node: usize) -> Result<(), M1RadixPrefixIndexErrorV1> {
        while node != ROOT_NODE {
            let entry = self
                .nodes
                .get(node)
                .and_then(Option::as_ref)
                .copied()
                .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
            if entry.first_child.is_some()
                || self
                    .sources
                    .iter()
                    .flatten()
                    .any(|source| source.endpoint == node)
            {
                break;
            }
            self.unlink_child(entry.parent, node)?;
            self.nodes[node] = None;
            self.node_count = self
                .node_count
                .checked_sub(1)
                .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?;
            node = entry.parent;
        }
        Ok(())
    }

    fn unlink_child(
        &mut self,
        parent: usize,
        child: usize,
    ) -> Result<(), M1RadixPrefixIndexErrorV1> {
        let replacement = self
            .nodes
            .get(child)
            .and_then(Option::as_ref)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?
            .next_sibling;
        let first = self
            .nodes
            .get(parent)
            .and_then(Option::as_ref)
            .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?
            .first_child;
        if first == Some(child) {
            self.nodes[parent]
                .as_mut()
                .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?
                .first_child = replacement;
            return Ok(());
        }
        let mut sibling = first;
        while let Some(index) = sibling {
            let next = self
                .nodes
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?
                .next_sibling;
            if next == Some(child) {
                self.nodes[index]
                    .as_mut()
                    .ok_or(M1RadixPrefixIndexErrorV1::InvariantViolation)?
                    .next_sibling = replacement;
                return Ok(());
            }
            sibling = next;
        }
        Err(M1RadixPrefixIndexErrorV1::InvariantViolation)
    }
}

/// Successful or empty result of one validated live-source share attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RadixPrefixShareOutcomeV1 {
    NoMatch {
        invalidated_sources: usize,
    },
    Shared {
        source: RequestId,
        target: RequestId,
        token_count: u32,
        invalidated_sources: usize,
    },
}

/// Rejection before or during one live-source share.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RadixPrefixShareErrorV1 {
    TargetNotReady {
        target: RequestId,
        state: Option<RequestState>,
    },
    TargetNotEmpty {
        target: RequestId,
        resident_tokens: Option<u32>,
        committed_tokens: Option<u32>,
    },
    SourceEqualsTarget(RequestId),
    Index(M1RadixPrefixIndexErrorV1),
    Engine(EngineError),
}

/// Registers tokens only from a completed authenticated paired-prefill owner.
///
/// The prompt is copied into temporary sealed authority with fallible
/// allocation. Both success and every error return the exact same
/// rollover-ready success owner; registration never decomposes it. Capacity and
/// ticket rejections leave the index unchanged.
///
/// This only records live-source metadata. With the current S1/T128 prompt and
/// 256-token logical Engine pages, registration succeeds but lookup admits no
/// reusable full logical page.
///
/// # Errors
///
/// Returns allocation or bounded-index rejection with the unchanged completed
/// prefill success in explicit failure custody.
pub fn register_m1_authenticated_prefill_radix_source_v1<const C: usize>(
    index: &mut M1RadixPrefixIndexV1,
    success: M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
) -> Result<
    M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
    Box<M1AuthenticatedRadixPrefixRegistrationFailureV1<C>>,
> {
    let mut tokens = Vec::new();
    if tokens
        .try_reserve_exact(success.prompt_tokens().len())
        .is_err()
    {
        return Err(Box::new(M1AuthenticatedRadixPrefixRegistrationFailureV1 {
            error: M1AuthenticatedRadixPrefixRegistrationErrorV1::HostAllocation,
            success,
        }));
    }
    tokens.extend_from_slice(success.prompt_tokens());
    let authority = M1AuthenticatedRadixPrefixSourceV1 {
        source: success.request(),
        tokens,
    };
    match index.register_authenticated_source(authority) {
        Ok(()) => Ok(success),
        Err(error) => Err(Box::new(M1AuthenticatedRadixPrefixRegistrationFailureV1 {
            error: M1AuthenticatedRadixPrefixRegistrationErrorV1::Index(error),
            success,
        })),
    }
}

/// Finds and shares the longest currently valid complete logical-page prefix.
///
/// Source readiness, generational identity, committed length, resident length,
/// and initialization are revalidated against the Engine in the same mutable
/// call that performs the COW share. Invalid source records are removed and the
/// next deterministic candidate is tried. The target must be a distinct empty
/// live Ready request. Stale retries are bounded by the source count observed
/// on entry.
///
/// # Errors
///
/// Rejects a faulted Engine, non-ready or nonempty target, self-share, index
/// invalidation failure, or lower Engine share failure.
pub fn share_m1_radix_prefix_from_live_source_v1<const C: usize>(
    index: &mut M1RadixPrefixIndexV1,
    engine: &mut Engine<C>,
    target: RequestId,
    tokens: &[TokenId],
) -> Result<M1RadixPrefixShareOutcomeV1, M1RadixPrefixShareErrorV1> {
    if engine.is_faulted() {
        return Err(M1RadixPrefixShareErrorV1::Engine(EngineError::Faulted));
    }
    let logical_page_tokens = engine.page_tokens();
    let target_state = engine.state(target);
    if target_state != Some(RequestState::Ready) {
        return Err(M1RadixPrefixShareErrorV1::TargetNotReady {
            target,
            state: target_state,
        });
    }
    let target_resident = engine.resident_tokens(target);
    let target_committed = engine.committed_tokens(target);
    if target_resident != Some(0) || target_committed != Some(0) {
        return Err(M1RadixPrefixShareErrorV1::TargetNotEmpty {
            target,
            resident_tokens: target_resident,
            committed_tokens: target_committed,
        });
    }

    let maximum_attempts = index.source_count();
    let mut invalidated_sources = 0usize;
    for _ in 0..maximum_attempts {
        let Some(prefix) = index.longest_page_aligned_match(tokens, logical_page_tokens) else {
            return Ok(M1RadixPrefixShareOutcomeV1::NoMatch {
                invalidated_sources,
            });
        };
        let source = prefix.source();
        if source == target {
            return Err(M1RadixPrefixShareErrorV1::SourceEqualsTarget(source));
        }
        let token_count = prefix.token_count();
        let source_ready = engine.state(source) == Some(RequestState::Ready);
        let source_committed = engine.committed_tokens(source);
        let source_resident = engine.resident_tokens(source);
        let source_has_range = source_ready
            && source_committed.is_some_and(|count| count >= token_count)
            && source_resident.is_some_and(|count| count >= token_count)
            && match engine.validate_read(source, 0, token_count) {
                Ok(()) => true,
                Err(EngineError::Faulted) => {
                    return Err(M1RadixPrefixShareErrorV1::Engine(EngineError::Faulted));
                }
                Err(_) => false,
            };
        if !source_has_range {
            let removed = index
                .invalidate_source(source)
                .map_err(M1RadixPrefixShareErrorV1::Index)?;
            if !removed {
                return Err(M1RadixPrefixShareErrorV1::Index(
                    M1RadixPrefixIndexErrorV1::InvariantViolation,
                ));
            }
            invalidated_sources =
                invalidated_sources
                    .checked_add(1)
                    .ok_or(M1RadixPrefixShareErrorV1::Index(
                        M1RadixPrefixIndexErrorV1::InvariantViolation,
                    ))?;
            continue;
        }

        engine
            .share_committed_prefix(source, target, token_count)
            .map_err(M1RadixPrefixShareErrorV1::Engine)?;
        return Ok(M1RadixPrefixShareOutcomeV1::Shared {
            source,
            target,
            token_count,
            invalidated_sources,
        });
    }
    Ok(M1RadixPrefixShareOutcomeV1::NoMatch {
        invalidated_sources,
    })
}

#[cfg(test)]
impl M1AuthenticatedRadixPrefixSourceV1 {
    fn for_test(source: RequestId, tokens: Vec<TokenId>) -> Self {
        Self { source, tokens }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch::ExactCompletion;
    use ferric_spec::completion::CompletionEpoch;

    const TEST_PAGE_TOKENS: u32 = 16;

    fn limits(nodes: usize, tokens: usize, sources: usize, stored: usize) -> M1RadixPrefixLimitsV1 {
        M1RadixPrefixLimitsV1::new(nodes, tokens, sources, stored).unwrap()
    }

    fn tokens(seed: u32, count: usize) -> Vec<TokenId> {
        (0..count)
            .map(|offset| seed.wrapping_add(u32::try_from(offset).unwrap()))
            .collect()
    }

    fn register(
        index: &mut M1RadixPrefixIndexV1,
        source: RequestId,
        tokens: Vec<TokenId>,
    ) -> Result<(), M1RadixPrefixIndexErrorV1> {
        index.register_authenticated_source(M1AuthenticatedRadixPrefixSourceV1::for_test(
            source, tokens,
        ))
    }

    fn ready_source<const C: usize>(accepted_tokens: u32) -> (Engine<C>, RequestId) {
        let mut engine = Engine::<C>::new(64, TEST_PAGE_TOKENS, 256).unwrap();
        let source = engine.admit().unwrap();
        engine.append_tentative(source, accepted_tokens).unwrap();
        let mut members = [RequestId::new(0, 0); C];
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members[0], source);
        engine
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &[accepted_tokens],
            )
            .unwrap();
        (engine, source)
    }

    #[test]
    fn lookup_rounds_down_using_the_supplied_logical_page_width() {
        let source = RequestId::new(1, 1);
        let source_tokens = tokens(10, 48);
        let mut index = M1RadixPrefixIndexV1::new(limits(128, 128, 4, 512)).unwrap();
        register(&mut index, source, source_tokens.clone()).unwrap();

        assert_eq!(
            index.longest_page_aligned_match(&source_tokens[..15], TEST_PAGE_TOKENS),
            None
        );
        assert_eq!(
            index.longest_page_aligned_match(&source_tokens[..31], TEST_PAGE_TOKENS),
            Some(M1RadixPrefixMatchV1 {
                source,
                token_count: 16,
            })
        );
        assert_eq!(
            index.longest_page_aligned_match(&source_tokens, 32),
            Some(M1RadixPrefixMatchV1 {
                source,
                token_count: 32,
            })
        );
        assert_eq!(index.longest_page_aligned_match(&source_tokens, 256), None);
    }

    #[test]
    fn authenticated_s1_t128_metadata_has_no_256_token_logical_page() {
        let source = RequestId::new(0, 1);
        let source_tokens = tokens(10, 128);
        let mut index = M1RadixPrefixIndexV1::new(limits(256, 128, 2, 256)).unwrap();
        register(&mut index, source, source_tokens.clone()).unwrap();

        assert_eq!(index.source_count(), 1);
        assert_eq!(index.stored_tokens(), 128);
        assert_eq!(index.longest_page_aligned_match(&source_tokens, 256), None);
    }

    #[test]
    fn longest_match_and_equal_depth_tie_break_are_deterministic() {
        let lower = RequestId::new(2, 1);
        let higher = RequestId::new(7, 1);
        let common = tokens(100, 32);
        let mut lower_tokens = common.clone();
        lower_tokens.extend(tokens(1_000, 16));
        let mut higher_tokens = common.clone();
        higher_tokens.extend(tokens(2_000, 32));
        let mut index = M1RadixPrefixIndexV1::new(limits(256, 128, 4, 512)).unwrap();
        register(&mut index, higher, higher_tokens.clone()).unwrap();
        register(&mut index, lower, lower_tokens).unwrap();

        assert_eq!(
            index.longest_page_aligned_match(&common, TEST_PAGE_TOKENS),
            Some(M1RadixPrefixMatchV1 {
                source: lower,
                token_count: 32,
            })
        );
        assert_eq!(
            index.longest_page_aligned_match(&higher_tokens, TEST_PAGE_TOKENS),
            Some(M1RadixPrefixMatchV1 {
                source: higher,
                token_count: 64,
            })
        );
        assert_eq!(
            index.longest_page_aligned_match(&tokens(9_000, 64), TEST_PAGE_TOKENS),
            None
        );
    }

    #[test]
    fn duplicate_prefix_and_request_slot_registration_replace_exactly_once() {
        let first = RequestId::new(1, 1);
        let second = RequestId::new(2, 1);
        let replacement = RequestId::new(2, 2);
        let shared = tokens(10, 32);
        let other = tokens(500, 32);
        let mut index = M1RadixPrefixIndexV1::new(limits(128, 64, 4, 256)).unwrap();

        register(&mut index, first, shared.clone()).unwrap();
        register(&mut index, second, shared.clone()).unwrap();
        assert_eq!(index.source_count(), 1);
        assert_eq!(
            index
                .longest_page_aligned_match(&shared, TEST_PAGE_TOKENS)
                .unwrap()
                .source(),
            second
        );

        register(&mut index, replacement, other.clone()).unwrap();
        assert_eq!(index.source_count(), 1);
        assert_eq!(
            index.longest_page_aligned_match(&shared, TEST_PAGE_TOKENS),
            None
        );
        assert_eq!(
            index
                .longest_page_aligned_match(&other, TEST_PAGE_TOKENS)
                .unwrap()
                .source(),
            replacement
        );
        assert!(!index.invalidate_source(second).unwrap());
        assert!(index.invalidate_source(replacement).unwrap());
        assert_eq!(index.node_count(), 1);
    }

    #[test]
    fn capacity_and_ticket_rejection_are_atomic_and_eviction_is_explicit() {
        let first = RequestId::new(1, 1);
        let second = RequestId::new(2, 1);
        let first_tokens = tokens(10, 16);
        let second_tokens = tokens(1_000, 16);
        let mut index = M1RadixPrefixIndexV1::new(limits(17, 16, 2, 32)).unwrap();
        register(&mut index, first, first_tokens.clone()).unwrap();

        assert_eq!(
            register(&mut index, second, second_tokens.clone()),
            Err(M1RadixPrefixIndexErrorV1::NodeCapacity {
                limit: 17,
                requested: 33,
            })
        );
        assert_eq!(index.source_count(), 1);
        assert_eq!(index.stored_tokens(), 16);
        assert_eq!(
            index
                .longest_page_aligned_match(&first_tokens, TEST_PAGE_TOKENS)
                .unwrap()
                .source(),
            first
        );

        index.next_ticket = u64::MAX;
        assert_eq!(
            register(&mut index, first, first_tokens.clone()),
            Err(M1RadixPrefixIndexErrorV1::TicketExhausted)
        );
        assert_eq!(index.source_count(), 1);
        index.next_ticket = 2;
        assert_eq!(index.evict_oldest().unwrap(), Some(first));
        register(&mut index, second, second_tokens).unwrap();
        assert_eq!(index.evict_oldest().unwrap(), Some(second));
        assert_eq!(index.evict_oldest().unwrap(), None);
    }

    #[test]
    fn independent_token_source_and_storage_limits_fail_closed() {
        assert_eq!(
            M1RadixPrefixLimitsV1::new(2, 0, 1, 1),
            Err(M1RadixPrefixLimitsErrorV1::Zero(
                M1RadixPrefixLimitV1::TokensPerSource
            ))
        );
        let source = RequestId::new(0, 1);
        let mut token_limited = M1RadixPrefixIndexV1::new(limits(64, 16, 2, 64)).unwrap();
        assert!(matches!(
            register(&mut token_limited, source, tokens(0, 32)),
            Err(M1RadixPrefixIndexErrorV1::TokensPerSourceCapacity { .. })
        ));
        let mut storage_limited = M1RadixPrefixIndexV1::new(limits(64, 32, 2, 16)).unwrap();
        assert!(matches!(
            register(&mut storage_limited, source, tokens(0, 32)),
            Err(M1RadixPrefixIndexErrorV1::StoredTokenCapacity { .. })
        ));
        let mut source_limited = M1RadixPrefixIndexV1::new(limits(128, 32, 1, 64)).unwrap();
        register(&mut source_limited, source, tokens(0, 16)).unwrap();
        assert!(matches!(
            register(&mut source_limited, RequestId::new(1, 1), tokens(100, 16)),
            Err(M1RadixPrefixIndexErrorV1::SourceCapacity { limit: 1 })
        ));
        assert_eq!(token_limited.source_count(), 0);
        assert_eq!(storage_limited.source_count(), 0);
        assert_eq!(source_limited.source_count(), 1);
    }

    #[test]
    fn bridge_shares_longest_initialized_logical_page_prefix() {
        let (mut engine, source) = ready_source::<2>(32);
        assert_eq!(engine.page_tokens(), TEST_PAGE_TOKENS);
        let target = engine.admit().unwrap();
        let source_tokens = tokens(10, 32);
        let mut query = source_tokens.clone();
        query[31] = 99_999;
        let mut index = M1RadixPrefixIndexV1::new(limits(128, 128, 4, 512)).unwrap();
        register(&mut index, source, source_tokens).unwrap();
        let free_before = engine.free_pages();

        assert_eq!(
            share_m1_radix_prefix_from_live_source_v1(&mut index, &mut engine, target, &query,),
            Ok(M1RadixPrefixShareOutcomeV1::Shared {
                source,
                target,
                token_count: 16,
                invalidated_sources: 0,
            })
        );
        assert_eq!(engine.committed_tokens(source), Some(32));
        assert_eq!(engine.committed_tokens(target), Some(16));
        assert_eq!(engine.resident_tokens(target), Some(16));
        assert_eq!(engine.free_pages(), free_before);
    }

    #[test]
    fn stale_generation_is_invalidated_before_any_share() {
        let (mut engine, source) = ready_source::<2>(32);
        let target = engine.admit().unwrap();
        let source_tokens = tokens(10, 32);
        let mut index = M1RadixPrefixIndexV1::new(limits(128, 128, 4, 512)).unwrap();
        register(&mut index, source, source_tokens.clone()).unwrap();
        engine.retire(source).unwrap();
        assert_eq!(engine.reclaim_one().unwrap(), Some(source));
        let replacement = engine.admit().unwrap();
        assert_eq!(replacement.slot(), source.slot());
        assert_ne!(replacement.generation(), source.generation());

        assert_eq!(
            share_m1_radix_prefix_from_live_source_v1(
                &mut index,
                &mut engine,
                target,
                &source_tokens,
            ),
            Ok(M1RadixPrefixShareOutcomeV1::NoMatch {
                invalidated_sources: 1,
            })
        );
        assert_eq!(index.source_count(), 0);
        assert_eq!(engine.committed_tokens(target), Some(0));
    }

    #[test]
    fn stale_deepest_match_falls_back_once_to_a_shorter_live_source() {
        let mut engine = Engine::<2>::new(64, TEST_PAGE_TOKENS, 256).unwrap();
        let live = engine.admit().unwrap();
        engine.append_tentative(live, 16).unwrap();
        let mut members = [RequestId::new(0, 0); 2];
        let live_batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        engine
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(live_batch.epoch()),
                &[16],
            )
            .unwrap();

        let stale = engine.admit().unwrap();
        engine.append_tentative(stale, 32).unwrap();
        let stale_batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members, [live, stale]);
        engine
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(stale_batch.epoch()),
                &[0, 32],
            )
            .unwrap();

        let query = tokens(10, 32);
        let mut index = M1RadixPrefixIndexV1::new(limits(128, 128, 4, 512)).unwrap();
        register(&mut index, live, query[..16].to_vec()).unwrap();
        register(&mut index, stale, query.clone()).unwrap();
        engine.retire(stale).unwrap();
        assert_eq!(engine.reclaim_one().unwrap(), Some(stale));
        let target = engine.admit().unwrap();
        assert_eq!(target.slot(), stale.slot());
        assert_ne!(target.generation(), stale.generation());

        assert_eq!(
            share_m1_radix_prefix_from_live_source_v1(&mut index, &mut engine, target, &query,),
            Ok(M1RadixPrefixShareOutcomeV1::Shared {
                source: live,
                target,
                token_count: 16,
                invalidated_sources: 1,
            })
        );
        assert_eq!(index.source_count(), 1);
        assert_eq!(engine.committed_tokens(target), Some(16));
    }

    #[test]
    fn bridge_rejects_nonempty_target_and_self_share() {
        let (mut engine, source) = ready_source::<2>(32);
        let target = engine.admit().unwrap();
        let source_tokens = tokens(10, 32);
        let mut index = M1RadixPrefixIndexV1::new(limits(128, 128, 4, 512)).unwrap();
        register(&mut index, source, source_tokens.clone()).unwrap();
        engine.append_tentative(target, 1).unwrap();
        assert!(matches!(
            share_m1_radix_prefix_from_live_source_v1(
                &mut index,
                &mut engine,
                target,
                &source_tokens,
            ),
            Err(M1RadixPrefixShareErrorV1::TargetNotEmpty { .. })
        ));

        let mut empty_engine = Engine::<1>::new(8, TEST_PAGE_TOKENS, 64).unwrap();
        let empty = empty_engine.admit().unwrap();
        let mut self_index = M1RadixPrefixIndexV1::new(limits(64, 32, 2, 64)).unwrap();
        register(&mut self_index, empty, source_tokens.clone()).unwrap();
        assert_eq!(
            share_m1_radix_prefix_from_live_source_v1(
                &mut self_index,
                &mut empty_engine,
                empty,
                &source_tokens,
            ),
            Err(M1RadixPrefixShareErrorV1::SourceEqualsTarget(empty))
        );
    }

    #[test]
    fn explicit_source_retirement_and_no_match_leave_target_unchanged() {
        let source = RequestId::new(1, 1);
        let source_tokens = tokens(10, 32);
        let mut index = M1RadixPrefixIndexV1::new(limits(128, 128, 4, 512)).unwrap();
        register(&mut index, source, source_tokens.clone()).unwrap();
        assert!(index.invalidate_source(source).unwrap());
        assert_eq!(
            index.longest_page_aligned_match(&source_tokens, TEST_PAGE_TOKENS),
            None
        );

        let mut engine = Engine::<1>::new(16, TEST_PAGE_TOKENS, 64).unwrap();
        let target = engine.admit().unwrap();
        assert_eq!(
            share_m1_radix_prefix_from_live_source_v1(
                &mut index,
                &mut engine,
                target,
                &source_tokens,
            ),
            Ok(M1RadixPrefixShareOutcomeV1::NoMatch {
                invalidated_sources: 0,
            })
        );
        assert_eq!(engine.committed_tokens(target), Some(0));
        assert_eq!(engine.resident_tokens(target), Some(0));
        assert_eq!(engine.completed_epoch(), CompletionEpoch::new(0));
    }
}
