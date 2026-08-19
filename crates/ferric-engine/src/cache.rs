use ferric_spec::RequestId;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId {
    index: u32,
    generation: u32,
}

impl PageId {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Debug)]
struct PageSlot {
    generation: u32,
    owner: Option<RequestId>,
    initialized_tokens: u32,
}

#[derive(Clone, Debug, Default)]
struct KvChain {
    pages: Vec<PageId>,
    committed_tokens: u32,
    resident_tokens: u32,
}

/// Fixed-capacity metadata for an exclusive paged KV cache.
///
/// Prefix sharing is deliberately not represented yet. Adding it requires an
/// immutable sealed-page state and a separate refinement proof.
#[derive(Clone, Debug)]
pub struct KvPool {
    page_tokens: u32,
    max_context_tokens: u32,
    pages: Vec<PageSlot>,
    free: BTreeSet<u32>,
    chains: BTreeMap<RequestId, KvChain>,
}

impl KvPool {
    /// Creates an empty fixed-capacity pool.
    ///
    /// # Errors
    ///
    /// Returns [`KvError`] when any capacity is zero or a page is larger than
    /// the maximum context.
    pub fn new(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
    ) -> Result<Self, KvError> {
        if page_count == 0 {
            return Err(KvError::ZeroCapacity("page_count"));
        }
        if page_tokens == 0 {
            return Err(KvError::ZeroCapacity("page_tokens"));
        }
        if max_context_tokens == 0 {
            return Err(KvError::ZeroCapacity("max_context_tokens"));
        }
        if page_tokens > max_context_tokens {
            return Err(KvError::PageExceedsContext);
        }

        let pages = (0..page_count)
            .map(|_| PageSlot {
                generation: 1,
                owner: None,
                initialized_tokens: 0,
            })
            .collect();
        let free = (0..page_count).collect();

        Ok(Self {
            page_tokens,
            max_context_tokens,
            pages,
            free,
            chains: BTreeMap::new(),
        })
    }

    /// Adds an empty request-owned KV chain.
    ///
    /// # Errors
    ///
    /// Returns [`KvError::DuplicateRequest`] when the identity is already live.
    pub fn create_request(&mut self, request: RequestId) -> Result<(), KvError> {
        if self.chains.contains_key(&request) {
            return Err(KvError::DuplicateRequest(request));
        }
        self.chains.insert(request, KvChain::default());
        Ok(())
    }

    /// Materializes tentative KV positions without publishing them.
    ///
    /// The allocation check is performed before mutation, so an out-of-pages
    /// result leaves the pool unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`KvError`] for an unknown request, insufficient capacity,
    /// context overflow, or an internal invariant violation.
    pub fn append_tentative(
        &mut self,
        request: RequestId,
        token_count: u32,
    ) -> Result<(), KvError> {
        let chain = self
            .chains
            .get(&request)
            .ok_or(KvError::UnknownRequest(request))?;
        let new_resident = chain
            .resident_tokens
            .checked_add(token_count)
            .ok_or(KvError::ContextExceeded)?;
        if new_resident > self.max_context_tokens {
            return Err(KvError::ContextExceeded);
        }
        if token_count == 0 {
            return Ok(());
        }

        let tail_capacity = chain.pages.last().map_or(0, |page| {
            self.page_tokens - self.pages[page.index as usize].initialized_tokens
        });
        let after_tail = token_count.saturating_sub(tail_capacity);
        let required_pages = after_tail.div_ceil(self.page_tokens) as usize;
        if required_pages > self.free.len() {
            return Err(KvError::OutOfPages);
        }

        let mut remaining = token_count;
        while remaining > 0 {
            let tail = self.chains[&request].pages.last().copied();
            let page = match tail {
                Some(page)
                    if self.pages[page.index as usize].initialized_tokens < self.page_tokens =>
                {
                    page
                }
                _ => self.allocate_page(request)?,
            };

            let slot = &mut self.pages[page.index as usize];
            let available = self.page_tokens - slot.initialized_tokens;
            let written = available.min(remaining);
            slot.initialized_tokens += written;
            self.chains
                .get_mut(&request)
                .ok_or(KvError::UnknownRequest(request))?
                .resident_tokens += written;
            remaining -= written;
        }

        self.validate_invariants()
    }

    /// Publishes a prefix of already materialized tentative KV positions.
    ///
    /// # Errors
    ///
    /// Returns [`KvError`] for an unknown request or when the commit would
    /// publish KV positions that have not been materialized.
    pub fn commit(&mut self, request: RequestId, token_count: u32) -> Result<(), KvError> {
        let chain = self
            .chains
            .get_mut(&request)
            .ok_or(KvError::UnknownRequest(request))?;
        let committed = chain
            .committed_tokens
            .checked_add(token_count)
            .ok_or(KvError::CommitExceedsResident)?;
        if committed > chain.resident_tokens {
            return Err(KvError::CommitExceedsResident);
        }
        chain.committed_tokens = committed;
        self.validate_invariants()
    }

    /// Removes every tentative KV position and invalidates released page IDs.
    ///
    /// # Errors
    ///
    /// Returns [`KvError`] for an unknown request, stale internal page state,
    /// or an invariant violation.
    pub fn rollback(&mut self, request: RequestId) -> Result<(), KvError> {
        let committed = self
            .chains
            .get(&request)
            .ok_or(KvError::UnknownRequest(request))?
            .committed_tokens;
        let retained_page_count = if committed == 0 {
            0
        } else {
            committed.div_ceil(self.page_tokens)
        };
        let retained_pages = usize::try_from(retained_page_count)
            .map_err(|_| KvError::InvariantViolation("page count does not fit usize"))?;

        let released = {
            let chain = self
                .chains
                .get_mut(&request)
                .ok_or(KvError::UnknownRequest(request))?;
            let released = chain.pages.split_off(retained_pages);
            chain.resident_tokens = committed;
            released
        };

        for page in released {
            self.release_page(page, request)?;
        }

        if retained_pages > 0 {
            let retained_tail = self.chains[&request].pages[retained_pages - 1];
            let preceding = (retained_page_count - 1) * self.page_tokens;
            self.pages[retained_tail.index as usize].initialized_tokens = committed - preceding;
        }

        self.validate_invariants()
    }

    /// Releases all pages owned by a request and invalidates their handles.
    ///
    /// # Errors
    ///
    /// Returns [`KvError`] for an unknown request or invalid page ownership.
    pub fn release_request(&mut self, request: RequestId) -> Result<(), KvError> {
        let chain = self
            .chains
            .remove(&request)
            .ok_or(KvError::UnknownRequest(request))?;
        for page in chain.pages {
            self.release_page(page, request)?;
        }
        self.validate_invariants()
    }

    #[must_use]
    pub fn resident_tokens(&self, request: RequestId) -> Option<u32> {
        self.chains.get(&request).map(|chain| chain.resident_tokens)
    }

    #[must_use]
    pub fn committed_tokens(&self, request: RequestId) -> Option<u32> {
        self.chains
            .get(&request)
            .map(|chain| chain.committed_tokens)
    }

    #[must_use]
    pub fn pages(&self, request: RequestId) -> Option<&[PageId]> {
        self.chains
            .get(&request)
            .map(|chain| chain.pages.as_slice())
    }

    /// Checks that a page handle is current and exclusively owned by a request.
    ///
    /// # Errors
    ///
    /// Returns [`KvError`] for an invalid index, stale generation, or wrong
    /// owner.
    pub fn validate_page(&self, page: PageId, owner: RequestId) -> Result<(), KvError> {
        let slot = self
            .pages
            .get(page.index as usize)
            .ok_or(KvError::InvalidPage(page))?;
        if slot.generation != page.generation {
            return Err(KvError::StalePage(page));
        }
        if slot.owner != Some(owner) {
            return Err(KvError::WrongOwner { page, owner });
        }
        Ok(())
    }

    /// Validates ownership, reachability, initialization, and token accounting.
    ///
    /// # Errors
    ///
    /// Returns [`KvError`] on the first violated pool invariant.
    pub fn validate_invariants(&self) -> Result<(), KvError> {
        let mut referenced = BTreeSet::new();

        for (request, chain) in &self.chains {
            if chain.committed_tokens > chain.resident_tokens
                || chain.resident_tokens > self.max_context_tokens
            {
                return Err(KvError::InvariantViolation("invalid token counts"));
            }

            let mut observed_tokens = 0_u32;
            for (position, page) in chain.pages.iter().enumerate() {
                self.validate_page(*page, *request)?;
                if !referenced.insert(page.index) {
                    return Err(KvError::InvariantViolation("page is referenced twice"));
                }
                let initialized = self.pages[page.index as usize].initialized_tokens;
                if initialized == 0 || initialized > self.page_tokens {
                    return Err(KvError::InvariantViolation("invalid initialized range"));
                }
                if position + 1 < chain.pages.len() && initialized != self.page_tokens {
                    return Err(KvError::InvariantViolation("non-tail page is partial"));
                }
                observed_tokens = observed_tokens
                    .checked_add(initialized)
                    .ok_or(KvError::InvariantViolation("token count overflow"))?;
            }
            if observed_tokens != chain.resident_tokens {
                return Err(KvError::InvariantViolation("resident token mismatch"));
            }
        }

        for (index, slot) in self.pages.iter().enumerate() {
            let index = u32::try_from(index)
                .map_err(|_| KvError::InvariantViolation("page index does not fit u32"))?;
            let is_free = self.free.contains(&index);
            if is_free != slot.owner.is_none() {
                return Err(KvError::InvariantViolation("free ownership mismatch"));
            }
            if is_free && slot.initialized_tokens != 0 {
                return Err(KvError::InvariantViolation("free page remains initialized"));
            }
            if !is_free && !referenced.contains(&index) {
                return Err(KvError::InvariantViolation("owned page is unreachable"));
            }
        }

        Ok(())
    }

    fn allocate_page(&mut self, request: RequestId) -> Result<PageId, KvError> {
        let index = self.free.pop_first().ok_or(KvError::OutOfPages)?;
        let slot = &mut self.pages[index as usize];
        if slot.owner.is_some() || slot.initialized_tokens != 0 {
            return Err(KvError::InvariantViolation("allocated page was not free"));
        }
        slot.owner = Some(request);
        let page = PageId {
            index,
            generation: slot.generation,
        };
        self.chains
            .get_mut(&request)
            .ok_or(KvError::UnknownRequest(request))?
            .pages
            .push(page);
        Ok(page)
    }

    fn release_page(&mut self, page: PageId, owner: RequestId) -> Result<(), KvError> {
        self.validate_page(page, owner)?;
        let slot = &mut self.pages[page.index as usize];
        slot.owner = None;
        slot.initialized_tokens = 0;
        slot.generation = slot
            .generation
            .checked_add(1)
            .ok_or(KvError::GenerationExhausted)?;
        if !self.free.insert(page.index) {
            return Err(KvError::InvariantViolation("page was released twice"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KvError {
    ZeroCapacity(&'static str),
    PageExceedsContext,
    DuplicateRequest(RequestId),
    UnknownRequest(RequestId),
    ContextExceeded,
    CommitExceedsResident,
    OutOfPages,
    InvalidPage(PageId),
    StalePage(PageId),
    WrongOwner { page: PageId, owner: RequestId },
    GenerationExhausted,
    InvariantViolation(&'static str),
}

impl fmt::Display for KvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for KvError {}

#[cfg(test)]
mod tests {
    use super::{KvError, KvPool};
    use ferric_spec::RequestId;

    fn request(slot: u32) -> RequestId {
        RequestId::new(slot, 1)
    }

    #[test]
    fn append_commit_and_rollback_preserve_the_committed_prefix() {
        let mut pool = KvPool::new(8, 4, 16).unwrap();
        pool.create_request(request(1)).unwrap();
        pool.append_tentative(request(1), 6).unwrap();
        pool.commit(request(1), 5).unwrap();
        let released_page = pool.pages(request(1)).unwrap()[1];

        pool.rollback(request(1)).unwrap();

        assert_eq!(pool.committed_tokens(request(1)), Some(5));
        assert_eq!(pool.resident_tokens(request(1)), Some(5));
        assert_eq!(pool.pages(request(1)).unwrap().len(), 2);
        assert_eq!(pool.pages(request(1)).unwrap()[1], released_page);
        pool.validate_invariants().unwrap();
    }

    #[test]
    fn rollback_releases_wholly_tentative_pages_and_rejects_stale_handles() {
        let mut pool = KvPool::new(8, 4, 16).unwrap();
        pool.create_request(request(1)).unwrap();
        pool.append_tentative(request(1), 3).unwrap();
        pool.commit(request(1), 3).unwrap();
        pool.append_tentative(request(1), 5).unwrap();
        let stale_page = pool.pages(request(1)).unwrap()[1];

        pool.rollback(request(1)).unwrap();
        pool.create_request(request(2)).unwrap();
        pool.append_tentative(request(2), 1).unwrap();

        assert_eq!(
            pool.validate_page(stale_page, request(1)),
            Err(KvError::StalePage(stale_page))
        );
    }

    #[test]
    fn out_of_pages_is_transactional() {
        let mut pool = KvPool::new(1, 4, 16).unwrap();
        pool.create_request(request(1)).unwrap();
        pool.append_tentative(request(1), 3).unwrap();

        assert_eq!(
            pool.append_tentative(request(1), 2),
            Err(KvError::OutOfPages)
        );
        assert_eq!(pool.resident_tokens(request(1)), Some(3));
        assert_eq!(pool.pages(request(1)).unwrap().len(), 1);
        pool.validate_invariants().unwrap();
    }

    #[test]
    fn request_release_invalidates_every_page() {
        let mut pool = KvPool::new(4, 4, 16).unwrap();
        pool.create_request(request(1)).unwrap();
        pool.append_tentative(request(1), 8).unwrap();
        let pages = pool.pages(request(1)).unwrap().to_vec();
        pool.release_request(request(1)).unwrap();

        for page in pages {
            assert_eq!(
                pool.validate_page(page, request(1)),
                Err(KvError::StalePage(page))
            );
        }
    }

    #[test]
    fn commits_cannot_publish_unmaterialized_tokens() {
        let mut pool = KvPool::new(4, 4, 16).unwrap();
        pool.create_request(request(1)).unwrap();
        pool.append_tentative(request(1), 2).unwrap();
        assert_eq!(
            pool.commit(request(1), 3),
            Err(KvError::CommitExceedsResident)
        );
    }
}
