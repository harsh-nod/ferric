//! New warm-only route; the expected counts are an assertion, never authority.
#[cfg(test)]
use super::scoped_currentness_v1::dispatch_warm_slot;
use super::scoped_currentness_v1::{WarmCompletion, dispatch_warm_slot_for};
use super::*;
use fe2o3_kfd::{
    Gfx950EngineeringPeerScopedCapacityCensusObservationV1 as Census,
    Gfx950EngineeringPeerScopedCurrentnessCountsV1 as Currentness,
    Gfx950EngineeringPeerScopedPrefixInputsV1 as PrefixInputs,
};
pub(crate) struct WarmCensusCompletion {
    pub(crate) layer: WarmCompletion,
    pub(crate) census: Census,
}
fn validate_census(c: &Census, current: Currentness, expected: [usize; 2]) -> Result<()> {
    let expected = [
        u64::try_from(expected[0]).map_err(|_| "census owner count conversion")?,
        u64::try_from(expected[1]).map_err(|_| "census owner count conversion")?,
    ];
    let local = current.local_checkpoints.checked_sub(16);
    let before = current.before_calls.checked_sub(16);
    let after = current.after_calls.checked_sub(16);
    let probes = current.generation_probes.checked_sub(32);
    let valid = match (local, before, after, probes) {
        (Some(l), Some(b), Some(a), Some(p)) => {
            l > 0
                && b == a
                && l.checked_add(4).is_some_and(|n| b >= n)
                && l.checked_mul(2)
                    .and_then(|n| n.checked_add(4))
                    .is_some_and(|n| b <= n)
                && l.checked_mul(2).and_then(|n| n.checked_add(3)) == Some(p)
        }
        _ => false,
    };
    if c.preflights != 2
        || c.rank_checkpoints != 16
        || c.owner_counts != expected
        || current.full_discoveries != 2
        || !valid
    {
        return Err("scoped census returned owner/subset/counter mismatch".into());
    }
    Ok(())
}
impl Roster {
    /// # Safety
    /// Exact authenticated images and actual same-slot mixed-bank owners are
    /// required. Only the explicit census Position5 selector may call this.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn dispatch_warm_scoped_census(
        &mut self,
        group: &mut Group,
        layer: usize,
        prefix_inputs: [PrefixInputs<'_>; 2],
        mlp_inputs: Inputs<'_>,
        timeout_ms: u32,
    ) -> Result<WarmCensusCompletion> {
        unsafe {
            self.dispatch_warm_scoped_census_for(
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
    /// Same exact images and owners, only for the separately selected warm Full roster.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn dispatch_full2303_warm_scoped_census(
        &mut self,
        group: &mut Group,
        layer: usize,
        prefix_inputs: [PrefixInputs<'_>; 2],
        mlp_inputs: Inputs<'_>,
        timeout_ms: u32,
    ) -> Result<WarmCensusCompletion> {
        unsafe {
            self.dispatch_warm_scoped_census_for(
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
    unsafe fn dispatch_warm_scoped_census_for(
        &mut self,
        group: &mut Group,
        layer: usize,
        prefix_inputs: [PrefixInputs<'_>; 2],
        mlp_inputs: Inputs<'_>,
        timeout_ms: u32,
        extent: Extent,
    ) -> Result<WarmCensusCompletion> {
        dispatch_warm_slot_for(
            &mut self.ledger,
            &mut self.slots,
            layer,
            extent,
            |prefixes, pair, expected, generation| {
                if !(1..=10_000).contains(&timeout_ms) {
                    return Err("scoped census layer deadline".into());
                }
                // SAFETY: dispatch_warm_slot derives expected solely from the
                // genuine Ledger and retains exact same-slot owners. Runtime
                // derives and validates complete actual accounting itself.
                let observed = unsafe {
                    group.dispatch_warm_layer_scoped_census_unchecked_v1(
                        prefixes,
                        pair,
                        prefix_inputs,
                        &mlp_inputs,
                        expected,
                        timeout_ms,
                    )
                }?;
                validate_census(&observed.census, observed.layer.currentness, expected)?;
                let done = observed.layer;
                if !done
                    .prefix
                    .final_states
                    .iter()
                    .all(super::super::prefix_tiles_v6::terminal)
                    || done.mlp.guards != [[generation as u32, 0, 1, 0]; 2]
                    || !done
                        .mlp
                        .prefixes
                        .iter()
                        .all(super::super::tiles_decode_v1::terminal)
                {
                    return Err("scoped census completion/generation mismatch".into());
                }
                let hidden = crate::native_catalog::forward::checked_layer_hidden_pair(
                    layer,
                    Ok(done.hidden),
                )?;
                Ok(WarmCensusCompletion {
                    layer: WarmCompletion {
                        prefix_states: done.prefix.final_states,
                        prefix_ns: done.prefix.dispatch_elapsed_ns,
                        guarded: done.mlp,
                        hidden,
                        currentness: done.currentness,
                    },
                    census: observed.census,
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
    fn counters() -> Currentness {
        Currentness {
            full_discoveries: 2,
            local_checkpoints: 21,
            before_calls: 27,
            after_calls: 27,
            generation_probes: 45,
        }
    }
    fn census(expected: [usize; 2]) -> Census {
        Census {
            owner_counts: expected.map(|n| n as u64),
            preflights: 2,
            rank_checkpoints: 16,
        }
    }
    #[test]
    fn census_roster_assertion_is_derived_from_real_ledger_and_same_slot() {
        let (mut ledger, mut slots) = warm();
        dispatch_warm_slot(
            &mut ledger,
            &mut slots,
            0,
            |[left, right], pair, expected, generation| {
                assert_eq!((*left, *right, *pair), (0, 1, 0));
                assert_eq!(expected, BOUND_COUNTS.map(|n| n + SLOTS));
                assert_eq!(generation, 2);
                validate_census(&census(expected), counters(), expected)
            },
        )
        .unwrap();
        assert_eq!(ledger.layer, 1);
        assert_eq!(ledger.phase, Phase::Prefix);
    }
    #[test]
    fn census_roster_post_runtime_data_refusal_is_terminal_without_retry() {
        for mutation in 0..8 {
            let (mut ledger, mut slots) = warm();
            let mut returned = false;
            assert!(
                dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, expected, _| {
                    returned = true;
                    let mut c = census(expected);
                    let mut v = counters();
                    match mutation {
                        0 => c.owner_counts.swap(0, 1),
                        1 => c.preflights = 1,
                        2 => c.rank_checkpoints = 15,
                        3 => v.full_discoveries = 1,
                        4 => v.local_checkpoints = 16,
                        5 => v.before_calls = 15,
                        6 => v.generation_probes = 31,
                        _ => v.after_calls += 1,
                    }
                    validate_census(&c, v, expected)
                })
                .is_err()
            );
            assert!(returned && ledger.phase == Phase::Terminal);
            assert!(
                dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| -> Result<()> {
                    panic!("no runtime retry")
                })
                .is_err()
            );
        }
    }
    #[test]
    fn census_roster_keeps_first_use_full_extent_and_unwind_refusals() {
        for variant in 0..3 {
            let (mut ledger, mut slots) = warm();
            match variant {
                0 => ledger.completed = 1,
                1 => ledger.extent = Extent::Full2303,
                _ => ledger.mode = ArenaMode::Fresh,
            }
            assert!(
                dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| -> Result<()> {
                    panic!("invalid custody reached runtime")
                })
                .is_err()
            );
        }
        let (mut ledger, mut slots) = warm();
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ =
                    dispatch_warm_slot(&mut ledger, &mut slots, 0, |_, _, _, _| -> Result<()> {
                        panic!("runtime unwind")
                    });
            }))
            .is_err()
        );
        assert_eq!(ledger.phase, Phase::Terminal);
    }
    #[test]
    fn full_bank_census_slot_preserves_late_generation_owner_assertion_and_fatal_refusal() {
        for reject in [false, true] {
            let (mut ledger, mut slots) = warm();
            ledger.extent = Extent::Full2303;
            ledger.completed = 2302;
            ledger.generation = 1152;
            ledger.dispatches = 2302 * LAYERS;
            ledger.last_generation = [1151; SLOTS];
            let before = ledger.counts();
            let mut returned = false;
            let result = dispatch_warm_slot_for(
                &mut ledger,
                &mut slots,
                0,
                Extent::Full2303,
                |prefixes, pair, expected, generation| {
                    assert_eq!((*prefixes[0], *prefixes[1], *pair), (0, 1, 0));
                    assert_eq!(generation, 1152);
                    assert_eq!(expected, before);
                    let mut c = census(expected);
                    if reject {
                        c.owner_counts.swap(0, 1);
                    }
                    returned = true;
                    validate_census(&c, counters(), expected)
                },
            );
            assert!(returned);
            if reject {
                assert!(result.is_err());
                assert_eq!(ledger.phase, Phase::Terminal);
                assert_eq!(ledger.last_generation[0], 1151);
            } else {
                result.unwrap();
                assert_eq!(ledger.phase, Phase::Prefix);
                assert_eq!(ledger.layer, 1);
                assert_eq!(ledger.last_generation[0], 1152);
                assert_eq!(ledger.counts(), before);
            }
        }
    }
}
