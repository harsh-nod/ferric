//! Explicit bank-only ablation; layer dispatch and allocation preflights stay separate.
use super::*;
use fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1 as Currentness;

pub(crate) struct BankCompletion {
    pub(crate) generation: u64,
    pub(crate) currentness: Option<Currentness>,
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
fn valid_window(c: &Currentness) -> bool {
    let calls = c
        .local_checkpoints
        .checked_mul(2)
        .and_then(|n| n.checked_add(4));
    let probes = c
        .local_checkpoints
        .checked_mul(2)
        .and_then(|n| n.checked_add(3));
    c.full_discoveries == 2
        && c.local_checkpoints > 0
        && calls == Some(c.before_calls)
        && c.before_calls == c.after_calls
        && probes == Some(c.generation_probes)
}
fn begin_bank<P, C>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    registration: [u8; 32],
    model: [u8; 32],
    forward: u64,
    position: u32,
    timeout_ms: u32,
    action: impl FnMut(&mut [Slot<P, C>], bool) -> Result<BankCompletion>,
) -> Result<BankCompletion> {
    begin_bank_for(
        ledger,
        slots,
        registration,
        model,
        forward,
        position,
        timeout_ms,
        Extent::Readiness40,
        action,
    )
}
fn begin_full2303_bank<P, C>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    registration: [u8; 32],
    model: [u8; 32],
    forward: u64,
    position: u32,
    timeout_ms: u32,
    action: impl FnMut(&mut [Slot<P, C>], bool) -> Result<BankCompletion>,
) -> Result<BankCompletion> {
    begin_bank_for(
        ledger,
        slots,
        registration,
        model,
        forward,
        position,
        timeout_ms,
        Extent::Full2303,
        action,
    )
}
fn begin_bank_for<P, C>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    registration: [u8; 32],
    model: [u8; 32],
    forward: u64,
    position: u32,
    timeout_ms: u32,
    extent: Extent,
    mut action: impl FnMut(&mut [Slot<P, C>], bool) -> Result<BankCompletion>,
) -> Result<BankCompletion> {
    let mut attempt = Attempt {
        ledger,
        committed: false,
    };
    let limit = match extent {
        Extent::Readiness40 => 40,
        Extent::Full2303 => 2303,
        Extent::Four => 0,
    };
    if slots.len() != SLOTS
        || attempt.ledger.extent != extent
        || attempt.ledger.mode != ArenaMode::ReuseRetired
        || attempt.ledger.paired_terminal
        || !(1..=10_000).contains(&timeout_ms)
        || attempt.ledger.completed >= limit
        || attempt.ledger.dispatches != attempt.ledger.completed as usize * LAYERS
        || attempt.ledger.arena_allocations != (attempt.ledger.completed.min(2) as usize) * LAYERS
    {
        return Err("bank scoped requires exact reusable readiness custody".into());
    }
    let selected = attempt.ledger.completed as usize % BANKS;
    let previous = attempt.ledger.completed / 2;
    if !attempt.ledger.last_generation[selected * LAYERS..(selected + 1) * LAYERS]
        .iter()
        .all(|g| *g == previous)
    {
        return Err("bank scoped requires every layer's actual retired generation".into());
    }
    let mut completed = None;
    attempt
        .ledger
        .begin(registration, model, forward, position, |bank, rearm| {
            let output = action(&mut slots[bank * LAYERS..(bank + 1) * LAYERS], rearm)?;
            match (rearm, output.currentness.as_ref()) {
                (false, None) => (),
                (true, Some(c)) if valid_window(c) => (),
                _ => return Err("bank scoped cannot replace first use or fall back".into()),
            }
            let generation = output.generation;
            completed = Some(output);
            Ok(generation)
        })?;
    let output = completed.ok_or("bank scoped missing completed action")?;
    attempt.committed = true;
    Ok(output)
}
impl Roster {
    /// # Safety
    /// Only the separately selected Position5 bank-scoped route may call this.
    /// Genuine slots retain exact mixed-bank/layer association and the previous
    /// fully retired generation; runtime validates all owners before any reset.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn begin_bank_scoped(
        &mut self,
        group: &mut Group,
        registration: [u8; 32],
        model: [u8; 32],
        forward: u64,
        position: u32,
        timeout_ms: u32,
    ) -> Result<BankCompletion> {
        unsafe {
            self.begin_bank_scoped_for(
                group,
                registration,
                model,
                forward,
                position,
                timeout_ms,
                Extent::Readiness40,
                #[cfg(feature = "engineering-currentness-duration-diagnostics")]
                &mut None,
            )
        }
    }
    /// # Safety
    /// Same genuine retired-bank obligations, only for the explicitly selected Full roster.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn begin_full2303_bank_scoped(
        &mut self,
        group: &mut Group,
        registration: [u8; 32],
        model: [u8; 32],
        forward: u64,
        position: u32,
        timeout_ms: u32,
    ) -> Result<BankCompletion> {
        unsafe {
            self.begin_bank_scoped_for(
                group,
                registration,
                model,
                forward,
                position,
                timeout_ms,
                Extent::Full2303,
                #[cfg(feature = "engineering-currentness-duration-diagnostics")]
                &mut None,
            )
        }
    }
    /// Returns diagnostics from the same exclusively owned Readiness40 bank operation.
    ///
    /// # Safety
    /// The caller must satisfy the existing `begin_bank_scoped` custody contract.
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    #[allow(unsafe_code)]
    pub(crate) unsafe fn begin_bank_scoped_diagnostic(
        &mut self,
        group: &mut Group,
        registration: [u8; 32],
        model: [u8; 32],
        forward: u64,
        position: u32,
        timeout_ms: u32,
    ) -> Result<(
        BankCompletion,
        Option<(fe2o3_kfd::Gfx950EngineeringCurrentnessDurationsV1, u64)>,
    )> {
        let mut metrics = None;
        let result = unsafe {
            self.begin_bank_scoped_for(
                group,
                registration,
                model,
                forward,
                position,
                timeout_ms,
                Extent::Readiness40,
                &mut metrics,
            )
        }?;
        Ok((result, metrics))
    }
    #[allow(unsafe_code)]
    unsafe fn begin_bank_scoped_for(
        &mut self,
        group: &mut Group,
        registration: [u8; 32],
        model: [u8; 32],
        forward: u64,
        position: u32,
        timeout_ms: u32,
        extent: Extent,
        #[cfg(feature = "engineering-currentness-duration-diagnostics")] metrics: &mut Option<(
            fe2o3_kfd::Gfx950EngineeringCurrentnessDurationsV1,
            u64,
        )>,
    ) -> Result<BankCompletion> {
        begin_bank_for(
            &mut self.ledger,
            &mut self.slots,
            registration,
            model,
            forward,
            position,
            timeout_ms,
            extent,
            |slots, rearm| {
                let mut entries = Vec::with_capacity(LAYERS);
                for Slot {
                    prefixes: [left, right],
                    pair,
                } in slots
                {
                    entries.push(BankEntry {
                        prefixes: [left, right],
                        pair,
                    });
                }
                if rearm {
                    // Runtime retains and quarantines the exact 36 exclusive entries.
                    let observed = unsafe {
                        group.rearm_guarded_mlp_bank_scoped_currentness_unchecked_v1(
                            &mut entries,
                            timeout_ms,
                        )
                    }?;
                    if observed.entries != LAYERS as u32 {
                        return Err("bank scoped returned wrong entry census".into());
                    }
                    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
                    {
                        *metrics = Some((
                            observed.currentness_durations,
                            observed.bank_guarded_body_ns,
                        ));
                    }
                    Ok(BankCompletion {
                        generation: observed.generation,
                        currentness: Some(observed.currentness),
                    })
                } else {
                    Ok(BankCompletion {
                        generation: group
                            .validate_guarded_mlp_initial_bank_v1(&mut entries, timeout_ms)?,
                        currentness: None,
                    })
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    fn fixture() -> (Ledger, Vec<Slot<u32, u32>>) {
        let mut ledger = Ledger::with_mode([1; 32], [2; 32], ArenaMode::ReuseRetired).unwrap();
        ledger.select_readiness40().unwrap();
        let slots = (0..SLOTS)
            .map(|i| Slot {
                prefixes: [i as u32 * 2, i as u32 * 2 + 1],
                pair: i as u32,
            })
            .collect();
        (ledger, slots)
    }
    fn currentness() -> Currentness {
        Currentness {
            full_discoveries: 2,
            local_checkpoints: 5,
            before_calls: 14,
            after_calls: 14,
            generation_probes: 13,
        }
    }
    fn finish(ledger: &mut Ledger, forward: u64) {
        for layer in 0..LAYERS {
            ledger.phase = Phase::GuardedInFlight;
            ledger.guarded_complete(layer).unwrap();
        }
        ledger.commit(forward, Ok(())).unwrap();
    }
    #[test]
    fn bank_scoped_roster_joins_all_actual_bank_generations_and_preserves_slots() {
        let (mut ledger, mut slots) = fixture();
        let mut ordinary = 0;
        let mut scoped = 0;
        for position in 0..40 {
            let completed = begin_bank(
                &mut ledger,
                &mut slots,
                [1; 32],
                [2; 32],
                position as u64 + 1,
                position,
                10_000,
                |bank, warm| {
                    assert_eq!(warm, position >= 2);
                    assert_eq!(bank.len(), 36);
                    for (layer, slot) in bank.iter().enumerate() {
                        let index = (position as usize % 2) * 36 + layer;
                        assert_eq!(slot.pair, index as u32);
                        assert_eq!(slot.prefixes, [index as u32 * 2, index as u32 * 2 + 1]);
                    }
                    if warm {
                        scoped += 1;
                    } else {
                        ordinary += 1;
                    }
                    Ok(BankCompletion {
                        generation: position as u64 / 2 + 1,
                        currentness: warm.then(currentness),
                    })
                },
            )
            .unwrap();
            assert_eq!(completed.generation, position as u64 / 2 + 1);
            assert_eq!(ledger.phase, Phase::Prefix);
            finish(&mut ledger, position as u64 + 1);
        }
        assert_eq!((ordinary, scoped), (2, 38));
        assert_eq!(ledger.last_generation, [20; SLOTS]);
        assert_eq!(ledger.arena_allocations, SLOTS);
        assert_eq!(ledger.phase, Phase::Exhausted);
    }
    #[test]
    fn bank_scoped_roster_refuses_wrong_domain_or_order_before_action() {
        for mutation in 0..10 {
            let (mut ledger, mut slots) = fixture();
            let mut timeout = 10_000;
            match mutation {
                0 => ledger.extent = Extent::Four,
                1 => ledger.extent = Extent::Full2303,
                2 => ledger.mode = ArenaMode::Fresh,
                3 => ledger.paired_terminal = true,
                4 => timeout = 0,
                5 => timeout = 10_001,
                6 => {
                    slots.pop();
                }
                7 => ledger.last_generation[0] = 1,
                8 => ledger.dispatches = 1,
                _ => ledger.phase = Phase::Prefix,
            }
            assert!(
                begin_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    1,
                    0,
                    timeout,
                    |_, _| panic!("invalid custody executed")
                )
                .is_err()
            );
            assert_eq!(ledger.phase, Phase::Terminal);
        }
        let (mut ledger, mut slots) = fixture();
        assert!(
            begin_bank(
                &mut ledger,
                &mut slots,
                [9; 32],
                [2; 32],
                1,
                0,
                10_000,
                |_, _| panic!("wrong identity executed")
            )
            .is_err()
        );
        assert_eq!(ledger.phase, Phase::Terminal);
    }
    #[test]
    fn bank_scoped_roster_warm_runtime_return_must_join_before_any_later_layer() {
        for mutation in 0..8 {
            let (mut ledger, mut slots) = fixture();
            for position in 0..2 {
                begin_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    position + 1,
                    position as u32,
                    10_000,
                    |_, warm| {
                        assert!(!warm);
                        Ok(BankCompletion {
                            generation: 1,
                            currentness: None,
                        })
                    },
                )
                .unwrap();
                finish(&mut ledger, position + 1);
            }
            assert_eq!(ledger.completed, 2);
            let mut returned = false;
            assert!(
                begin_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    3,
                    2,
                    10_000,
                    |_, warm| {
                        assert!(warm);
                        returned = true;
                        let mut c = currentness();
                        match mutation {
                            0 => c.full_discoveries = 1,
                            1 => c.local_checkpoints = 0,
                            2 => c.before_calls += 1,
                            3 => c.after_calls += 1,
                            4 => c.generation_probes += 1,
                            5 => c.local_checkpoints = u64::MAX,
                            _ => (),
                        }
                        Ok(BankCompletion {
                            generation: if mutation == 7 { 3 } else { 2 },
                            currentness: (mutation != 6).then_some(c),
                        })
                    }
                )
                .is_err()
            );
            assert!(returned);
            assert_eq!(ledger.phase, Phase::Terminal);
            assert_eq!(ledger.completed, 2);
            assert_eq!(ledger.dispatches, 72);
            assert_eq!(ledger.last_generation, [1; SLOTS]);
            assert!(ledger.expect(0, Phase::Prefix).is_err());
            assert!(
                begin_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    3,
                    2,
                    10_000,
                    |_, _| panic!("warm failure retried")
                )
                .is_err()
            );
        }
    }
    #[test]
    fn bank_scoped_roster_runtime_refusal_unwind_and_late_result_are_terminal() {
        for mutation in 0..4 {
            let (mut ledger, mut slots) = fixture();
            let result = catch_unwind(AssertUnwindSafe(|| {
                begin_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    1,
                    0,
                    10_000,
                    |_, _| match mutation {
                        0 => Err("runtime rejected".into()),
                        1 => panic!("runtime unwind"),
                        2 => Ok(BankCompletion {
                            generation: 2,
                            currentness: None,
                        }),
                        _ => Ok(BankCompletion {
                            generation: 1,
                            currentness: Some(currentness()),
                        }),
                    },
                )
            }));
            assert!(result.is_err() || result.unwrap().is_err());
            assert_eq!(ledger.phase, Phase::Terminal);
            assert_eq!(ledger.completed, 0);
            assert!(
                begin_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    1,
                    0,
                    10_000,
                    |_, _| panic!("failed bank retried")
                )
                .is_err()
            );
        }
    }
    #[test]
    fn full_bank_census_roster_joins_all_2303_retired_bank_generations() {
        let (mut ledger, mut slots) = fixture();
        ledger.extent = Extent::Full2303;
        let mut ordinary = 0;
        let mut scoped = 0;
        for position in 0..2303 {
            let completed = begin_full2303_bank(
                &mut ledger,
                &mut slots,
                [1; 32],
                [2; 32],
                position as u64 + 1,
                position,
                10_000,
                |bank, warm| {
                    assert_eq!(warm, position >= 2);
                    assert_eq!(bank.len(), 36);
                    for (layer, slot) in bank.iter().enumerate() {
                        let index = (position as usize % 2) * 36 + layer;
                        assert_eq!(slot.pair, index as u32);
                        assert_eq!(slot.prefixes, [index as u32 * 2, index as u32 * 2 + 1]);
                    }
                    if warm {
                        scoped += 1;
                    } else {
                        ordinary += 1;
                    }
                    Ok(BankCompletion {
                        generation: position as u64 / 2 + 1,
                        currentness: warm.then(currentness),
                    })
                },
            )
            .unwrap();
            assert_eq!(completed.generation, position as u64 / 2 + 1);
            assert_eq!(ledger.phase, Phase::Prefix);
            finish(&mut ledger, position as u64 + 1);
        }
        assert_eq!((ordinary, scoped), (2, 2301));
        assert_eq!(&ledger.last_generation[..36], &[1152; 36]);
        assert_eq!(&ledger.last_generation[36..], &[1151; 36]);
        assert_eq!(ledger.arena_allocations, SLOTS);
        assert_eq!(ledger.phase, Phase::Exhausted);
    }
    #[test]
    fn full_bank_census_roster_refuses_readiness_stale_late_banks_and_unwind() {
        for mutation in 0..6 {
            let (mut ledger, mut slots) = fixture();
            ledger.extent = Extent::Full2303;
            ledger.completed = 2302;
            ledger.dispatches = 2302 * LAYERS;
            ledger.arena_allocations = SLOTS;
            ledger.last_generation = [1151; SLOTS];
            match mutation {
                0 => ledger.extent = Extent::Readiness40,
                1 => ledger.extent = Extent::Four,
                2 => ledger.last_generation[35] -= 1,
                3 => ledger.completed = 2303,
                4 => ledger.paired_terminal = true,
                _ => ledger.mode = ArenaMode::Fresh,
            }
            assert!(
                begin_full2303_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    2303,
                    2302,
                    10_000,
                    |_, _| panic!("invalid Full bank")
                )
                .is_err()
            );
            assert_eq!(ledger.phase, Phase::Terminal);
        }
        for unwind in [false, true] {
            let (mut ledger, mut slots) = fixture();
            ledger.extent = Extent::Full2303;
            let result = catch_unwind(AssertUnwindSafe(|| {
                begin_full2303_bank(
                    &mut ledger,
                    &mut slots,
                    [1; 32],
                    [2; 32],
                    1,
                    0,
                    10_000,
                    |_, _| {
                        assert!(!unwind, "Full bank unwind");
                        Err("Full bank runtime refusal".into())
                    },
                )
            }));
            assert!(if unwind {
                result.is_err()
            } else {
                result.unwrap().is_err()
            });
            assert_eq!(ledger.phase, Phase::Terminal);
            assert_eq!(ledger.completed, 0);
        }
    }
}
