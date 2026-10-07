//! One warm ledger operation spans prefix, paired MLP, hidden readback and exit.
use super::*;
use fe2o3_kfd::{
    Gfx950EngineeringPeerScopedCurrentnessCountsV1 as CurrentnessCounts,
    Gfx950EngineeringPeerScopedPrefixInputsV1 as PrefixInputs,
};

pub(crate) struct WarmCompletion {
    pub(crate) prefix_states: [[u32; 284]; 2],
    pub(crate) prefix_ns: [u64; 2],
    pub(crate) guarded: Observation,
    pub(crate) hidden: Vec<u8>,
    pub(crate) currentness: CurrentnessCounts,
}

struct Attempt<'a> {
    ledger: &'a mut Ledger,
    committed: bool,
}

impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.ledger.phase = Phase::Terminal;
        }
    }
}

pub(super) fn dispatch_warm_slot<P, C, O>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    layer: usize,
    dispatch: impl FnOnce([&mut P; 2], &mut C, [usize; 2], u64) -> Result<O>,
) -> Result<O> {
    dispatch_warm_slot_for(ledger, slots, layer, Extent::Readiness40, dispatch)
}

fn dispatch_full2303_warm_slot<P, C, O>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    layer: usize,
    dispatch: impl FnOnce([&mut P; 2], &mut C, [usize; 2], u64) -> Result<O>,
) -> Result<O> {
    dispatch_warm_slot_for(ledger, slots, layer, Extent::Full2303, dispatch)
}

pub(super) fn dispatch_warm_slot_for<P, C, O>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    layer: usize,
    extent: Extent,
    dispatch: impl FnOnce([&mut P; 2], &mut C, [usize; 2], u64) -> Result<O>,
) -> Result<O> {
    let mut attempt = Attempt {
        ledger,
        committed: false,
    };
    let ledger = &mut *attempt.ledger;
    let limit = match extent {
        Extent::Readiness40 => 40,
        Extent::Full2303 => 2303,
        Extent::Four => 0,
    };
    if slots.len() != SLOTS
        || ledger.extent != extent
        || ledger.mode != ArenaMode::ReuseRetired
        || ledger.paired_terminal
        || !(2..limit).contains(&ledger.completed)
        || ledger.bank != ledger.completed as usize % BANKS
        || ledger.generation != ledger.completed / 2 + 1
    {
        return Err("scoped warm layer requires genuine warm readiness custody".into());
    }
    let index = ledger.expect(layer, Phase::Prefix)?;
    if ledger.arena_additional(index)? != 0 || ledger.last_generation[index] == 0 {
        return Err("scoped warm layer cannot allocate a fresh arena".into());
    }
    let before = ledger.counts();
    ledger.phase = Phase::ScopedLayerInFlight;
    let Slot {
        prefixes: [left, right],
        pair,
    } = &mut slots[index];
    let observation = dispatch([left, right], pair, before, ledger.generation)?;
    // No externally visible intermediate Prefix commit. The runtime has closed
    // its scope and the caller has checked all returned data before this point.
    if ledger.phase != Phase::ScopedLayerInFlight {
        return Err("scoped warm layer lost in-flight custody".into());
    }
    ledger.phase = Phase::GuardedInFlight;
    ledger.guarded_complete(layer)?;
    attempt.committed = true;
    Ok(observation)
}

impl Roster {
    /// # Safety
    /// The caller must select the explicit scoped profile and retain the exact
    /// admitted images, buffer roles and mixed-bank association. This method is
    /// warm-only; the first two forwards use the unchanged ordinary path.
    pub(crate) unsafe fn dispatch_warm_scoped(
        &mut self,
        group: &mut Group,
        layer: usize,
        prefix_inputs: [PrefixInputs<'_>; 2],
        mlp_inputs: Inputs<'_>,
        timeout_ms: u32,
    ) -> Result<WarmCompletion> {
        unsafe {
            self.dispatch_warm_scoped_for(
                group,
                layer,
                prefix_inputs,
                mlp_inputs,
                timeout_ms,
                Extent::Readiness40,
            )
        }
    }

    /// # Safety
    /// Same image/owner obligations as dispatch_warm_scoped, but only for the
    /// separately selected Full2303 roster after both banks' ordinary first use.
    pub(crate) unsafe fn dispatch_full2303_warm_scoped(
        &mut self,
        group: &mut Group,
        layer: usize,
        prefix_inputs: [PrefixInputs<'_>; 2],
        mlp_inputs: Inputs<'_>,
        timeout_ms: u32,
    ) -> Result<WarmCompletion> {
        unsafe {
            self.dispatch_warm_scoped_for(
                group,
                layer,
                prefix_inputs,
                mlp_inputs,
                timeout_ms,
                Extent::Full2303,
            )
        }
    }

    #[allow(unsafe_code)]
    unsafe fn dispatch_warm_scoped_for(
        &mut self,
        group: &mut Group,
        layer: usize,
        prefix_inputs: [PrefixInputs<'_>; 2],
        mlp_inputs: Inputs<'_>,
        timeout_ms: u32,
        extent: Extent,
    ) -> Result<WarmCompletion> {
        dispatch_warm_slot_for(
            &mut self.ledger,
            &mut self.slots,
            layer,
            extent,
            |prefixes, pair, before, generation| {
                if !(1..=10_000).contains(&timeout_ms)
                    || group.preflight_additional_allocations_v1(&[0, 0])? != before
                {
                    return Err("scoped warm pre-dispatch deadline/allocation census".into());
                }
                // SAFETY: the ledger selects the actual same Slot and previous
                // generation, never caller-generated Prefix or retirement proof.
                // Runtime retains all three owners through its checked exit.
                let observed = unsafe {
                    group.dispatch_warm_layer_scoped_currentness_unchecked_v1(
                        prefixes,
                        pair,
                        prefix_inputs,
                        &mlp_inputs,
                        timeout_ms,
                    )
                }?;
                if !observed
                    .prefix
                    .final_states
                    .iter()
                    .all(super::super::prefix_tiles_v6::terminal)
                    || observed.mlp.guards != [[generation as u32, 0, 1, 0]; 2]
                    || !observed
                        .mlp
                        .prefixes
                        .iter()
                        .all(super::super::tiles_decode_v1::terminal)
                    || observed.currentness.full_discoveries != 2
                    || observed.currentness.local_checkpoints == 0
                    || group.preflight_additional_allocations_v1(&[0, 0])? != before
                {
                    return Err("scoped warm completion/generation/allocation census".into());
                }
                let hidden = crate::native_catalog::forward::checked_layer_hidden_pair(
                    layer,
                    Ok(observed.hidden),
                )?;
                Ok(WarmCompletion {
                    prefix_states: observed.prefix.final_states,
                    prefix_ns: observed.prefix.dispatch_elapsed_ns,
                    guarded: observed.mlp,
                    hidden,
                    currentness: observed.currentness,
                })
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn warm() -> (Ledger, Vec<Slot<u32, u32>>) {
        let mut ledger = Ledger::with_mode([1; 32], [2; 32], ArenaMode::ReuseRetired).unwrap();
        ledger.extent = Extent::Readiness40;
        ledger.completed = 2;
        ledger.bank = 0;
        ledger.generation = 2;
        ledger.phase = Phase::Prefix;
        ledger.arena_allocations = SLOTS;
        ledger.dispatches = SLOTS;
        ledger.last_generation = [1; SLOTS];
        let slots = (0..SLOTS)
            .map(|i| Slot {
                prefixes: [i as u32 * 2, i as u32 * 2 + 1],
                pair: i as u32,
            })
            .collect();
        (ledger, slots)
    }

    #[test]
    fn full_scoped_roster_keeps_real_slot_generations_through_all_2303_forwards() {
        let mut ledger = Ledger::with_mode([1; 32], [2; 32], ArenaMode::ReuseRetired).unwrap();
        ledger.select_full2303().unwrap();
        let (_, mut slots) = warm();
        let mut generations = [0; SLOTS];
        let mut ordinary = 0;
        let mut scoped = 0;
        for position in 0..2303u64 {
            ledger
                .begin(
                    [1; 32],
                    [2; 32],
                    position + 1,
                    position as u32,
                    |bank, rearm| {
                        assert_eq!(bank, position as usize % 2);
                        assert_eq!(rearm, position >= 2);
                        Ok(position / 2 + 1)
                    },
                )
                .unwrap();
            for layer in 0..36 {
                let index = position as usize % 2 * 36 + layer;
                if position < 2 {
                    ledger.phase = Phase::Guarded;
                    dispatch_slot(
                        &mut ledger,
                        &mut slots,
                        layer,
                        |pair, additional, _, generation| {
                            assert_eq!(*pair as usize, index);
                            assert_eq!(additional, 1);
                            generations[index] = generation;
                            ordinary += 1;
                            Ok(())
                        },
                    )
                    .unwrap();
                } else {
                    dispatch_full2303_warm_slot(
                        &mut ledger,
                        &mut slots,
                        layer,
                        |prefixes, pair, before, generation| {
                            assert_eq!(
                                (*prefixes[0], *prefixes[1]),
                                (index as u32 * 2, index as u32 * 2 + 1)
                            );
                            assert_eq!(*pair as usize, index);
                            assert_eq!(before, REUSE_MAX_COUNTS);
                            assert_eq!(generations[index] + 1, generation);
                            generations[index] = generation;
                            scoped += 1;
                            Ok(())
                        },
                    )
                    .unwrap();
                }
            }
            ledger.commit(position + 1, Ok(())).unwrap();
            if position >= 1 {
                assert_eq!(ledger.counts(), REUSE_MAX_COUNTS);
            }
            if [39, 40, 2047, 2048, 2302].contains(&position) {
                assert_eq!(ledger.completed, position + 1);
                assert_eq!(ledger.arena_allocations, 72);
            }
        }
        assert_eq!((ordinary, scoped), (72, 82_836));
        assert_eq!(ledger.phase, Phase::Exhausted);
        assert_eq!(&generations[..36], &[1152; 36]);
        assert_eq!(&generations[36..], &[1151; 36]);
        assert_eq!(ledger.dispatches, 2303 * 36);
        assert!(
            ledger
                .begin([1; 32], [2; 32], 2304, 2303, |_, _| panic!("after extent"))
                .is_err()
        );
    }

    #[test]
    fn full_scoped_roster_refuses_other_extents_stale_late_generations_and_unwind() {
        for mutation in 0..7 {
            let (mut ledger, mut slots) = warm();
            ledger.extent = Extent::Full2303;
            ledger.completed = 2302;
            ledger.generation = 1152;
            ledger.last_generation = [1151; SLOTS];
            match mutation {
                0 => ledger.extent = Extent::Readiness40,
                1 => ledger.extent = Extent::Four,
                2 => ledger.completed = 1,
                3 => ledger.completed = 2303,
                4 => ledger.last_generation[0] -= 1,
                5 => ledger.paired_terminal = true,
                _ => ledger.mode = ArenaMode::Fresh,
            }
            assert!(
                dispatch_full2303_warm_slot(
                    &mut ledger,
                    &mut slots,
                    0,
                    |_, _, _, _| -> Result<()> {
                        panic!("invalid Full scoped dispatch");
                    }
                )
                .is_err()
            );
            assert_eq!(ledger.phase, Phase::Terminal);
        }
        for unwind in [false, true] {
            let (mut ledger, mut slots) = warm();
            ledger.extent = Extent::Full2303;
            let result = catch_unwind(AssertUnwindSafe(|| {
                dispatch_full2303_warm_slot(
                    &mut ledger,
                    &mut slots,
                    0,
                    |_, _, _, _| -> Result<()> {
                        assert!(!unwind, "late checked exit panic");
                        Err("late checked exit error".into())
                    },
                )
            }));
            if unwind {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            assert_eq!(ledger.phase, Phase::Terminal);
            assert_eq!(ledger.dispatches, 72);
            assert_eq!(ledger.last_generation, [1; SLOTS]);
        }
    }

    #[test]
    fn scoped_warm_slot_commits_only_after_closed_call_and_keeps_allocation_plateau() {
        let (mut ledger, mut slots) = warm();
        let value = dispatch_warm_slot(
            &mut ledger,
            &mut slots,
            0,
            |prefixes, pair, counts, generation| {
                assert_eq!((*prefixes[0], *prefixes[1], *pair), (0, 1, 0));
                assert_eq!(generation, 2);
                assert_eq!(counts, REUSE_MAX_COUNTS);
                Ok(19)
            },
        )
        .unwrap();
        assert_eq!(value, 19);
        assert_eq!(ledger.phase, Phase::Prefix);
        assert_eq!(ledger.layer, 1);
        assert_eq!(ledger.dispatches, SLOTS + 1);
        assert_eq!(ledger.last_generation[0], 2);
        assert_eq!(ledger.counts(), REUSE_MAX_COUNTS);
    }

    #[test]
    fn scoped_warm_slot_refuses_first_use_fresh_extent_and_paired_mode_before_dispatch() {
        for mutation in 0..5 {
            let (mut ledger, mut slots) = warm();
            match mutation {
                0 => ledger.completed = 0,
                1 => ledger.completed = 1,
                2 => ledger.mode = ArenaMode::Fresh,
                3 => ledger.extent = Extent::Full2303,
                _ => ledger.paired_terminal = true,
            }
            assert!(
                dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| -> Result<()> {
                    panic!("inadmissible warm callback");
                })
                .is_err()
            );
            assert_eq!(ledger.phase, Phase::Terminal);
        }
    }

    #[test]
    fn scoped_warm_slot_refuses_stale_bank_generation_and_wrong_layer() {
        for mutation in 0..5 {
            let (mut ledger, mut slots) = warm();
            match mutation {
                0 => ledger.bank = 1,
                1 => ledger.generation = 3,
                2 => ledger.last_generation[0] = 0,
                3 => ledger.phase = Phase::Guarded,
                _ => ledger.layer = 1,
            }
            assert!(
                dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| -> Result<()> {
                    panic!("stale warm callback");
                })
                .is_err()
            );
            assert_eq!(ledger.phase, Phase::Terminal);
        }
    }

    #[test]
    fn scoped_warm_slot_missing_owner_roster_refuses_without_execution() {
        let (mut ledger, mut slots) = warm();
        slots.pop();
        assert!(
            dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| -> Result<()> {
                panic!("incomplete owner roster");
            })
            .is_err()
        );
        assert_eq!(ledger.phase, Phase::Terminal);
    }

    #[test]
    fn scoped_warm_slot_error_leaves_no_commit_or_reusable_ledger() {
        let (mut ledger, mut slots) = warm();
        assert!(
            dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| -> Result<()> {
                Err("injected checked-exit failure".into())
            })
            .is_err()
        );
        assert_eq!(ledger.phase, Phase::Terminal);
        assert_eq!(ledger.dispatches, SLOTS);
        assert_eq!(ledger.last_generation, [1; SLOTS]);
    }

    #[test]
    fn scoped_warm_slot_unwind_quarantines_ledger_without_advancing_generation() {
        let (mut ledger, mut slots) = warm();
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _: Result<()> = dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| {
                    panic!("injected native unwind");
                });
            }))
            .is_err()
        );
        assert_eq!(ledger.phase, Phase::Terminal);
        assert_eq!(ledger.dispatches, SLOTS);
        assert_eq!(ledger.last_generation, [1; SLOTS]);
    }

    #[test]
    fn scoped_warm_last_layer_advances_to_tail_not_the_next_forward() {
        let (mut ledger, mut slots) = warm();
        ledger.layer = LAYERS - 1;
        dispatch_warm_slot(
            &mut ledger,
            &mut slots,
            LAYERS - 1,
            |prefixes, pair, _, _| {
                assert_eq!((*prefixes[0], *prefixes[1], *pair), (70, 71, 35));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(ledger.phase, Phase::Tail);
        assert_eq!(ledger.completed, 2);
    }
}
