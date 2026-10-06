//! Borrow-free paired owners, with short exclusive group custody per operation.
use super::*;

#[derive(Eq, PartialEq)]
struct Binding {
    group: u64,
    kernels: [[(u64, usize, u64, [u8; 32]); 4]; 2],
    roots: [[Gfx950EngineeringPeerBufferV1; 10]; 2],
    partials: [Gfx950EngineeringPeerBufferV1; 2],
    residuals: [Gfx950EngineeringPeerBufferV1; 2],
    outputs: [Gfx950EngineeringPeerBufferV1; 2],
    images: [[u8; 32]; 2],
}

impl Binding {
    fn capture(group: &Gfx950EngineeringPeerGroupV1, inputs: &Inputs<'_>) -> Self {
        Self {
            group: group.incarnation,
            kernels: inputs.ranks.each_ref().map(|rank| {
                rank.kernels.map(|kernel| {
                    (
                        kernel.group,
                        kernel.rank,
                        kernel.id,
                        kernel.metadata.object_sha256,
                    )
                })
            }),
            roots: inputs.ranks.each_ref().map(|rank| rank.mlp_roots),
            partials: inputs.partials,
            residuals: inputs.ranks.each_ref().map(|rank| rank.residual_input),
            outputs: inputs.ranks.each_ref().map(|rank| rank.output),
            images: [inputs.projection_sha256, inputs.mlp_sha256],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Ready,
    Busy,
    Completed,
    Poisoned,
}

struct Completed {
    generation: u64,
    states: [CombinedMlpSnapshotV1; 2],
    proof: arena::Retired,
}

/// No borrowed Group/kernel references, mutable owner views, or caller proofs.
pub(in super::super) struct RetainedPair {
    owners: [CombinedMlpStateV1; 2],
    binding: Binding,
    phase: Phase,
    completed: Option<Completed>,
}

fn poison_group(group: &mut Gfx950EngineeringPeerGroupV1) {
    group.poisoned = true;
    for context in &mut group.contexts {
        context.ordered_batch_poisoned = true;
    }
}

struct Allocation<'a> {
    group: &'a mut Gfx950EngineeringPeerGroupV1,
    owners: Vec<CombinedMlpStateV1>,
    committed: bool,
}

impl Drop for Allocation<'_> {
    fn drop(&mut self) {
        if !self.committed {
            for owner in &mut self.owners {
                owner.poison(self.group);
            }
            poison_group(self.group);
        }
    }
}

struct Operation<'a> {
    group: &'a mut Gfx950EngineeringPeerGroupV1,
    pairs: &'a mut [RetainedPair],
    committed: bool,
    owner_ids: BTreeSet<(u64, u64)>,
    arena_ids: BTreeSet<(u64, u64)>,
}

impl<'a> Operation<'a> {
    fn new(group: &'a mut Gfx950EngineeringPeerGroupV1, pairs: &'a mut [RetainedPair]) -> Self {
        Self {
            group,
            pairs,
            committed: false,
            owner_ids: BTreeSet::new(),
            arena_ids: BTreeSet::new(),
        }
    }

    fn begin(&mut self, expected: Phase) -> Result<()> {
        if self.pairs.iter().any(|pair| pair.phase != expected) {
            return Err("retained paired custody phase mismatch".into());
        }
        for pair in &mut *self.pairs {
            pair.phase = Phase::Busy;
        }
        self.group.require_active()?;
        profiles::validate_policy(self.group)?;
        if self
            .pairs
            .iter()
            .any(|pair| pair.binding.group != self.group.incarnation)
        {
            return Err("retained paired group incarnation mismatch".into());
        }
        Ok(())
    }

    fn quarantine(&mut self) {
        for pair in &mut *self.pairs {
            pair.phase = Phase::Poisoned;
            for owner in &mut pair.owners {
                owner.poison(self.group);
            }
        }
        poison_group(self.group);
    }
}

impl Drop for Operation<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.quarantine();
        }
    }
}

fn deadline(start: Instant, timeout_ms: u32) -> Result<Instant> {
    if !(1..=10_000).contains(&timeout_ms) {
        return Err("retained paired timeout outside 1..10000 ms".into());
    }
    start
        .checked_add(Duration::from_millis(u64::from(timeout_ms)))
        .ok_or_else(|| "retained paired deadline overflow".into())
}

impl RetainedPair {
    /// # Safety
    /// Dispatch safety obligations hold for the entire pair lifetime, including
    /// every future payload update/run: dedicated disposable process, reviewed
    /// exact images and model-role joins, completed coherent attention producers,
    /// and no other dispatch accessing the private owner storage. Failures are
    /// terminal. Whole-model bank/forward ordering is the Ferric caller's job.
    pub(in super::super) unsafe fn allocate(
        group: &mut Gfx950EngineeringPeerGroupV1,
        inputs: &Inputs<'_>,
        timeout_ms: u32,
    ) -> Result<Self> {
        let mut custody = Allocation {
            group,
            owners: Vec::with_capacity(2),
            committed: false,
        };
        let until = deadline(Instant::now(), timeout_ms)?;
        custody.group.require_active()?;
        profiles::validate_policy(custody.group)?;
        check_contexts(
            &mut custody.group.contexts,
            custody.group.shared_full_currentness,
        )?;
        for rank in 0..2 {
            custody
                .owners
                .push(super::super::allocate(custody.group, rank)?);
            deadline_check(Instant::now(), until)?;
        }
        let owners: &mut [CombinedMlpStateV1; 2] = custody
            .owners
            .as_mut_slice()
            .try_into()
            .map_err(|_| "retained paired owner count")?;
        profiles::prepare(custody.group, owners, inputs, timeout_ms)?;
        check_contexts(
            &mut custody.group.contexts,
            custody.group.shared_full_currentness,
        )?;
        deadline_check(Instant::now(), until)?;
        let binding = Binding::capture(custody.group, inputs);
        let owners = std::mem::take(&mut custody.owners)
            .try_into()
            .map_err(|_| "retained paired allocated owner count")?;
        let pair = Self {
            owners,
            binding,
            phase: Phase::Ready,
            completed: None,
        };
        deadline_check(Instant::now(), until)?;
        custody.committed = true;
        Ok(pair)
    }

    pub(in super::super) fn run(
        &mut self,
        group: &mut Gfx950EngineeringPeerGroupV1,
        inputs: Inputs<'_>,
        timeout_ms: u32,
    ) -> Result<Completion> {
        let mut op = Operation::new(group, std::slice::from_mut(self));
        let until = deadline(Instant::now(), timeout_ms)?;
        op.begin(Phase::Ready)?;
        let pair = &mut op.pairs[0];
        if pair.completed.is_some() || pair.binding != Binding::capture(op.group, &inputs) {
            return Err("retained paired input/kernel role binding changed".into());
        }
        let mut native = Native {
            group: op.group,
            owners: &mut pair.owners,
            inputs,
            timeout_ms,
            generation: 0,
            staged: None,
            validated_terminal: None,
        };
        // Keep the unchanged coordinator's strict in-flight reservation checks.
        let result = coordinate_until(&mut native, timeout_ms, Some(until))?;
        deadline_check(Instant::now(), until)?;
        let proof = native
            .staged
            .take()
            .ok_or("retained paired completed arena absent")?
            .seal_retired(native.group, until)?;
        let generation = native.generation;
        pair.completed = Some(Completed {
            generation,
            states: result.states.clone(),
            proof,
        });
        deadline_check(Instant::now(), until)?;
        pair.phase = Phase::Completed;
        op.committed = true;
        Ok(result)
    }
}

trait RearmBackend {
    fn now(&mut self) -> Instant;
    fn validate(&mut self, pair: usize, deadline: Instant) -> Result<u64>;
    fn fence(&mut self) -> Result<()>;
    fn reset(&mut self, pair: usize, rank: usize, next: u64) -> Result<()>;
    fn initial(&mut self, pair: usize, next: u64) -> Result<()>;
    fn quarantine(&mut self);
}

fn rearm_all(
    backend: &mut impl RearmBackend,
    count: usize,
    timeout_ms: u32,
    outer_deadline: Option<Instant>,
) -> Result<u64> {
    let result = (|| {
        let until = deadline(backend.now(), timeout_ms)?;
        let until = outer_deadline.map_or(until, |outer| outer.min(until));
        deadline_check(backend.now(), until)?;
        if !(1..=72).contains(&count) {
            return Err("retained paired batch count outside 1..72".into());
        }
        let mut next = None;
        // Validate the whole selected set before any atomic reset. No external
        // snapshots are accepted and every member must advance the same local generation.
        for pair in 0..count {
            let old = backend.validate(pair, until)?;
            let candidate = old
                .checked_add(1)
                .filter(|_| old != 0)
                .ok_or("retained paired generation exhausted or zero")?;
            if next.is_some_and(|value| value != candidate) {
                return Err("retained paired batch generations differ".into());
            }
            next = Some(candidate);
            deadline_check(backend.now(), until)?;
        }
        let next = next.ok_or("retained paired batch empty")?;
        backend.fence()?;
        deadline_check(backend.now(), until)?;
        for pair in 0..count {
            for rank in 0..2 {
                backend.reset(pair, rank, next)?;
                deadline_check(backend.now(), until)?;
            }
        }
        for pair in 0..count {
            backend.initial(pair, next)?;
            deadline_check(backend.now(), until)?;
        }
        backend.fence()?;
        deadline_check(backend.now(), until)?;
        Ok(next)
    })();
    if result.is_err() {
        backend.quarantine();
    }
    result
}

impl RearmBackend for Operation<'_> {
    fn now(&mut self) -> Instant {
        Instant::now()
    }

    fn validate(&mut self, index: usize, until: Instant) -> Result<u64> {
        let pair = &mut self.pairs[index];
        if pair.phase != Phase::Busy || pair.binding.group != self.group.incarnation {
            return Err("retained paired rearm custody/group mismatch".into());
        }
        let old = pair
            .completed
            .as_mut()
            .ok_or("retained paired terminal proof missing")?;
        old.proof.recheck(self.group, until)?;
        for id in old.proof.identities() {
            if !self.arena_ids.insert(id) {
                return Err("retained paired duplicate arena".into());
            }
        }
        for rank in 0..2 {
            let owner = &pair.owners[rank];
            if owner.buffer.owner != rank
                || !self.owner_ids.insert((owner.buffer.group, owner.buffer.id))
            {
                return Err("retained paired duplicate or misranked owner".into());
            }
            require_activation(owner.activation, Activation::Completed)?;
            if owner.generation != old.generation || owner.observe(self.group)? != old.states[rank]
            {
                return Err("retained paired terminal generation/state drift".into());
            }
            require_terminal(&old.states[rank], old.generation)?;
            deadline_check(Instant::now(), until)?;
        }
        Ok(old.generation)
    }

    fn fence(&mut self) -> Result<()> {
        check_contexts(&mut self.group.contexts, self.group.shared_full_currentness)
    }

    fn reset(&mut self, index: usize, rank: usize, next: u64) -> Result<()> {
        let pair = &mut self.pairs[index];
        let old = pair
            .completed
            .as_ref()
            .ok_or("retained paired reset lost proof")?;
        // SAFETY: all selected sealed batches/signals/terminal owners and the
        // current idle queues were validated before the first store. Exclusive
        // Group custody prevents intervening publication. No arenas are reset.
        unsafe { pair.owners[rank].rearm_quiescent(self.group, &old.states[rank], next) }
    }

    fn initial(&mut self, index: usize, next: u64) -> Result<()> {
        if profiles::validate_owners(self.group, &self.pairs[index].owners)? != next {
            return Err("retained paired batch initial readback/generation mismatch".into());
        }
        Ok(())
    }

    fn quarantine(&mut self) {
        Operation::quarantine(self);
    }
}

/// Only the guarded MLP owners are covered here. Ferric must also fence and
/// validate its Prefix284 bank and global forward ledger before whole-bank reuse.
pub(in super::super) fn rearm_pairs(
    group: &mut Gfx950EngineeringPeerGroupV1,
    pairs: &mut [RetainedPair],
    timeout_ms: u32,
) -> Result<u64> {
    let mut op = Operation::new(group, pairs);
    let until = deadline(Instant::now(), timeout_ms)?;
    if !(1..=72).contains(&op.pairs.len()) {
        return Err("retained paired batch count outside 1..72".into());
    }
    op.begin(Phase::Completed)?;
    let count = op.pairs.len();
    let next = rearm_all(&mut op, count, timeout_ms, Some(until))?;
    for pair in &mut *op.pairs {
        pair.completed = None; // Host metadata only; Group retains every arena backing.
        pair.phase = Phase::Ready;
    }
    deadline_check(Instant::now(), until)?;
    op.committed = true;
    Ok(next)
}

#[cfg(test)]
pub(in super::super) struct DiagnosticPair {
    pub owners: [[u64; 4]; 2],
    pub generation: u64,
    pub completed: bool,
    pub states: [CombinedMlpSnapshotV1; 2],
    pub frontiers: [(u64, u64); 2],
}

#[cfg(test)]
impl RetainedPair {
    /// Observation only: no token, mutable owner, or transferable retired proof.
    pub(in super::super) fn observe_for_test(
        &mut self,
        group: &mut Gfx950EngineeringPeerGroupV1,
        timeout_ms: u32,
    ) -> Result<DiagnosticPair> {
        let mut op = Operation::new(group, std::slice::from_mut(self));
        let until = deadline(Instant::now(), timeout_ms)?;
        let phase = op.pairs[0].phase;
        if !matches!(phase, Phase::Ready | Phase::Completed) {
            return Err("retained diagnostic requires quiescent custody".into());
        }
        op.begin(phase)?;
        check_contexts(&mut op.group.contexts, op.group.shared_full_currentness)?;
        let generation = if phase == Phase::Completed {
            let pair = &op.pairs[0];
            let old = pair
                .completed
                .as_ref()
                .ok_or("retained diagnostic terminal proof absent")?;
            if pair.owners[0].buffer.id == pair.owners[1].buffer.id {
                return Err("retained diagnostic duplicate owners".into());
            }
            for rank in 0..2 {
                require_activation(pair.owners[rank].activation, Activation::Completed)?;
                if pair.owners[rank].owner_rank() != rank
                    || pair.owners[rank].generation != old.generation
                {
                    return Err("retained diagnostic terminal owner generation/rank".into());
                }
            }
            old.generation
        } else {
            if op.pairs[0].completed.is_some() {
                return Err("retained diagnostic Ready with old proof".into());
            }
            profiles::validate_owners(op.group, &op.pairs[0].owners)?
        };
        let pair = &mut op.pairs[0];
        let states = [
            pair.owners[0].observe(op.group)?,
            pair.owners[1].observe(op.group)?,
        ];
        for state in &states {
            if phase == Phase::Ready {
                require_initial(state, generation)?;
            } else {
                require_terminal(state, generation)?;
            }
        }
        if phase == Phase::Completed
            && states
                != pair
                    .completed
                    .as_ref()
                    .ok_or("retained diagnostic completed proof missing")?
                    .states
        {
            return Err("retained diagnostic terminal readback changed".into());
        }
        check_contexts(&mut op.group.contexts, op.group.shared_full_currentness)?;
        let value = DiagnosticPair {
            owners: pair.owners.each_ref().map(|owner| {
                let token = owner.buffer;
                [token.group, token.id, token.owner as u64, token.bytes]
            }),
            generation,
            completed: phase == Phase::Completed,
            states,
            frontiers: std::array::from_fn(|rank| {
                let context = &op.group.contexts[rank];
                (context.ring.write(), context.last_observed_read)
            }),
        };
        deadline_check(Instant::now(), until)?;
        pair.phase = phase;
        op.committed = true;
        Ok(value)
    }
}

#[cfg(test)]
#[path = "engineering_gfx950_peer_combined_mlp_paired_retained_v1_tests.rs"]
mod tests;
