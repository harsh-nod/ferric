//! Private collection wrappers keep the existing Tail operation guard armed.
use super::*;
use crate::finite_guarded_mlp_readiness_currentness_durations_v1 as wire;
use crate::finite_guarded_mlp_readiness_forward_durations_v1 as forward;
use fe2o3_kfd::Gfx950EngineeringCurrentnessDurationsV1 as RuntimeDurations;

#[derive(Default)]
pub(super) struct Trace {
    rows: Vec<wire::ForwardRow>,
    forward_rows: Vec<forward::ForwardRow>,
    forward_body_ns: u64,
    pending: Option<Pending>,
}
struct Pending {
    bank: Option<(wire::Durations, u64)>,
    layers: wire::Durations,
    next_layer: usize,
}
fn convert(value: RuntimeDurations, counts: Currentness) -> io::Result<wire::Durations> {
    let pair = |p: fe2o3_kfd::Gfx950EngineeringCurrentnessCallDurationV1| wire::CallDuration {
        calls: p.calls,
        elapsed_ns: p.elapsed_ns,
    };
    let output = wire::Durations {
        before: pair(value.before),
        discover: pair(value.discover),
        after: pair(value.after),
        root_generation: pair(value.root_generation),
    };
    output.validate_calls(
        counts.full_discoveries,
        counts.before_calls,
        counts.after_calls,
        counts.generation_probes,
    )?;
    Ok(output)
}
fn observed(
    position: u32,
    counts: Option<Currentness>,
    value: Option<RuntimeDurations>,
) -> io::Result<Option<wire::Durations>> {
    match (position < 2, counts, value) {
        (true, None, None) => Ok(None),
        (false, Some(counts), Some(value)) => convert(value, counts).map(Some),
        _ => Err(io::Error::other(
            "diagnostic first-use/warm runtime duration mismatch",
        )),
    }
}
impl Trace {
    fn begin(
        &mut self,
        position: u32,
        counts: Option<Currentness>,
        value: Option<(RuntimeDurations, u64)>,
    ) -> io::Result<()> {
        require(
            self.pending.is_none()
                && self.rows.len() == position as usize
                && self.forward_rows.len() == position as usize
                && position < 40,
            "diagnostic bank ordering",
        )?;
        let duration = observed(position, counts, value.map(|v| v.0))?;
        let bank = match (duration, value) {
            (None, None) => None,
            (Some(duration), Some((_, body))) => {
                duration.validate_calls(2, 726, 726, 725)?;
                require(
                    duration.elapsed_subtotal()? <= body && body <= 3_600_000_000_000,
                    "diagnostic bank callback containment",
                )?;
                Some((duration, body))
            }
            _ => return Err(io::Error::other("diagnostic bank first-use duration")),
        };
        self.pending = Some(Pending {
            bank,
            layers: wire::Durations::default(),
            next_layer: 0,
        });
        Ok(())
    }
    fn layer(
        &mut self,
        position: u32,
        layer: usize,
        counts: Option<Currentness>,
        value: Option<RuntimeDurations>,
    ) -> io::Result<()> {
        require(
            self.rows.len() == position as usize,
            "diagnostic layer position",
        )?;
        let pending = self
            .pending
            .as_mut()
            .ok_or_else(|| io::Error::other("diagnostic absent bank"))?;
        require(
            layer == pending.next_layer && layer < 36,
            "diagnostic fixed layer order",
        )?;
        if let Some(value) = observed(position, counts, value)? {
            require(value.discover.calls == 2, "diagnostic two layer boundaries")?;
            pending.layers = pending.layers.checked_add(value)?;
        }
        pending.next_layer += 1;
        Ok(())
    }
    fn tail(
        &mut self,
        position: u32,
        counts: Option<Currentness>,
        value: Option<RuntimeDurations>,
    ) -> io::Result<()> {
        require(
            self.rows.len() == position as usize,
            "diagnostic tail position",
        )?;
        let pending = self
            .pending
            .as_ref()
            .ok_or_else(|| io::Error::other("diagnostic absent forward"))?;
        require(pending.next_layer == 36, "diagnostic all thirty-six layers")?;
        let measured = match (pending.bank, observed(position, counts, value)?) {
            (None, None) => {
                require(
                    pending.layers == wire::Durations::default(),
                    "diagnostic ordinary layers unmeasured",
                )?;
                None
            }
            (Some((bank, bank_guarded_body_ns)), Some(tail)) => {
                let value = wire::MeasuredForward {
                    bank,
                    layers: pending.layers,
                    tail,
                    bank_guarded_body_ns,
                };
                value.validate()?;
                Some(value)
            }
            _ => {
                return Err(io::Error::other(
                    "diagnostic full forward measurement consistency",
                ));
            }
        };
        self.rows.push(wire::ForwardRow { position, measured });
        self.pending = None;
        Ok(())
    }
    fn forward(&mut self, row: forward::ForwardRow) -> io::Result<()> {
        require(
            self.pending.is_none()
                && self.forward_rows.len() == row.position as usize
                && self.rows.len() == row.position as usize + 1,
            "diagnostic forward phase ordering",
        )?;
        let callbacks = self
            .rows
            .get(row.position as usize)
            .ok_or_else(|| io::Error::other("diagnostic forward callbacks absent"))?;
        row.validate_against(callbacks)?;
        let total = self
            .forward_body_ns
            .checked_add(row.forward_body_ns)
            .ok_or_else(|| io::Error::other("diagnostic forward total overflow"))?;
        require(
            total <= 3_600_000_000_000,
            "diagnostic forward total extent",
        )?;
        self.forward_rows.push(row);
        self.forward_body_ns = total;
        Ok(())
    }
}
impl State {
    pub(in super::super) fn forward_timing_selected(&self) -> bool {
        self.diagnostic.is_some()
    }
    pub(in super::super) fn record_forward_timing(
        &mut self,
        row: forward::ForwardRow,
    ) -> io::Result<()> {
        let mut a = Attempt {
            state: self,
            committed: false,
        };
        require(
            !a.state.terminal
                && !a.state.active
                && row.position.checked_add(1) == Some(a.state.next),
            "diagnostic forward requires completed Tail operation",
        )?;
        a.state
            .diagnostic
            .as_mut()
            .ok_or_else(|| io::Error::other("diagnostic forward trace absent"))?
            .forward(row)?;
        a.committed = true;
        Ok(())
    }
    pub(in super::super) fn new_diagnostic(profile: Profile) -> io::Result<Self> {
        let mut state = Self::new(profile)?;
        state.diagnostic = Some(Trace::default());
        Ok(state)
    }
    pub(in super::super) fn begin_diagnostic(
        &mut self,
        position: u32,
        call: impl FnOnce() -> io::Result<(BankCompletion, Option<(RuntimeDurations, u64)>)>,
    ) -> io::Result<()> {
        let mut a = Attempt {
            state: self,
            committed: false,
        };
        require(
            a.state.diagnostic.is_some(),
            "diagnostic Tail state not selected",
        )?;
        let mut observed_counts = None;
        let mut observed_time = None;
        let trace = a.state.diagnostic.as_ref().unwrap();
        require(
            trace.rows.len() == trace.forward_rows.len(),
            "diagnostic previous forward phases absent",
        )?;
        a.state.begin(position, || {
            let (result, duration) = call()?;
            observed_counts = result.currentness;
            observed_time = duration;
            Ok(result)
        })?;
        a.state
            .diagnostic
            .as_mut()
            .unwrap()
            .begin(position, observed_counts, observed_time)?;
        a.committed = true;
        Ok(())
    }
    pub(in super::super) fn dispatch_diagnostic<O>(
        &mut self,
        position: u32,
        layer: usize,
        call: impl FnOnce(
            bool,
        )
            -> io::Result<(O, Option<(Currentness, Census)>, Option<RuntimeDurations>)>,
    ) -> io::Result<O> {
        let mut a = Attempt {
            state: self,
            committed: false,
        };
        require(
            a.state.diagnostic.is_some(),
            "diagnostic Tail state not selected",
        )?;
        let mut observed_counts = None;
        let mut observed_time = None;
        let result = a.state.dispatch(position, layer, |warm| {
            let (result, counts, duration) = call(warm)?;
            observed_counts = counts.map(|v| v.0);
            observed_time = duration;
            Ok((result, counts))
        })?;
        a.state.diagnostic.as_mut().unwrap().layer(
            position,
            layer,
            observed_counts,
            observed_time,
        )?;
        a.committed = true;
        Ok(result)
    }
    pub(in super::super) fn tail_diagnostic<O>(
        &mut self,
        position: u32,
        call: impl FnOnce(bool) -> io::Result<(O, Option<Currentness>, Option<RuntimeDurations>)>,
    ) -> io::Result<O> {
        let mut a = Attempt {
            state: self,
            committed: false,
        };
        require(
            a.state.diagnostic.is_some(),
            "diagnostic Tail state not selected",
        )?;
        let mut observed_counts = None;
        let mut observed_time = None;
        let result = a.state.tail(position, |warm| {
            let (result, counts, duration) = call(warm)?;
            observed_counts = counts;
            observed_time = duration;
            Ok((result, counts))
        })?;
        a.state
            .diagnostic
            .as_mut()
            .unwrap()
            .tail(position, observed_counts, observed_time)?;
        a.committed = true;
        Ok(result)
    }
    pub(in super::super) fn diagnostic_rows(
        &self,
        counts: &Counts,
    ) -> io::Result<Vec<wire::ForwardRow>> {
        require(
            !self.terminal && !self.active && self.next == 40,
            "diagnostic healthy complete Tail state",
        )?;
        let trace = self
            .diagnostic
            .as_ref()
            .ok_or_else(|| io::Error::other("diagnostic absent trace"))?;
        require(trace.pending.is_none(), "diagnostic forward still active")?;
        wire::validate_rows(&trace.rows, counts)?;
        require(
            serde_json::to_vec(&trace.rows)
                .map_err(io::Error::other)?
                .len()
                <= wire::MAX_BYTES - 4096,
            "diagnostic bounded rows before Close",
        )?;
        Ok(trace.rows.clone())
    }
    pub(in super::super) fn forward_rows(&self) -> io::Result<Vec<forward::ForwardRow>> {
        require(
            !self.terminal && !self.active && self.next == 40,
            "diagnostic healthy complete forward phases",
        )?;
        let trace = self
            .diagnostic
            .as_ref()
            .ok_or_else(|| io::Error::other("diagnostic forward trace absent"))?;
        require(
            trace.pending.is_none() && trace.forward_rows.len() == 40 && trace.rows.len() == 40,
            "diagnostic all forty phase rows before Close",
        )?;
        for (row, callbacks) in trace.forward_rows.iter().zip(&trace.rows) {
            row.validate_against(callbacks)?;
        }
        Ok(trace.forward_rows.clone())
    }
}

#[cfg(test)]
#[path = "native_guarded_mlp_readiness_duration_v1_tests.rs"]
mod tests;
