//! Explicit host recorders with closed conservative or shared-full policy.
use crate::finite_guarded_mlp_decode_wire_v1::{Bootstrap, Completion};
use crate::guarded_mlp_host_observation_v1::{self as data, Policy, Report};
use crate::native_prefix_decode_host_v1::{ns, snapshot_shared_policy};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerHostObservationV1 as NativeSnapshot,
};
use std::{io, time::Duration};

trait ObserverSetup {
    fn shared_full(&mut self) -> io::Result<()>;
    fn enable(&mut self) -> io::Result<()>;
}

impl ObserverSetup for Group {
    fn shared_full(&mut self) -> io::Result<()> {
        self.configure_performance_v2(false, false, true)
            .map_err(io::Error::other)
    }

    fn enable(&mut self) -> io::Result<()> {
        self.enable_host_observation_v1().map_err(io::Error::other)
    }
}

fn configure_observer(group: &mut impl ObserverSetup, policy: Policy) -> io::Result<()> {
    if policy.shared() {
        group.shared_full()?;
    }
    group.enable()
}

pub(crate) struct Recorder {
    previous: NativeSnapshot,
    report: Report,
    terminal: bool,
    policy: Policy,
}
impl Recorder {
    pub(crate) fn enable(group: &mut Group, b: &Bootstrap) -> io::Result<Self> {
        Self::enable_policy(group, b, Policy::DefaultFull)
    }

    pub(crate) fn enable_shared(group: &mut Group, b: &Bootstrap) -> io::Result<Self> {
        Self::enable_policy(group, b, Policy::SharedFull)
    }

    pub(crate) fn enable_paired_read(group: &mut Group, b: &Bootstrap) -> io::Result<Self> {
        if b.schema != crate::finite_guarded_mlp_decode_wire_v1::SCHEMA
            || b.decode.mode != crate::finite_guarded_mlp_decode_wire_v1::InputMode::Autoregressive
        {
            return Err(io::Error::other("paired hidden reads require fresh AR4"));
        }
        Self::enable_policy(group, b, Policy::SharedFullPairedRead)
    }

    pub(crate) fn paired_read(&self) -> bool {
        self.policy.paired_read()
    }

    fn enable_policy(group: &mut Group, b: &Bootstrap, policy: Policy) -> io::Result<Self> {
        // Configure before enabling timers or allocating any owner/kernel. No
        // currentness snapshot or runtime authorization is cached by this mode.
        configure_observer(group, policy)?;
        let fresh = group.host_observation_v1().map_err(io::Error::other)?;
        let first = snapshot_shared_policy(
            &fresh,
            &data::phase(0)?,
            b.decode.device_ids,
            policy.shared(),
        )?;
        if first.shared != [0; 4] || first.ranks.iter().any(|r| r.counters != [0; 19]) {
            return Err(io::Error::other("guarded host baseline is not fresh"));
        }
        Ok(Self {
            previous: fresh,
            terminal: false,
            policy,
            report: Report {
                schema: policy.schema().into(),
                bootstrap: b.clone(),
                worker_sha256: crate::native_prefix_decode_host_v1::executable_sha()?,
                child_pid: std::process::id(),
                profile_sha256: b.sha256()?,
                snapshots: vec![first],
                intervals: Vec::with_capacity(data::SNAPSHOTS - 1),
                forward_host_ns: [0; 4],
                close_host_ns: 0,
                completions: Vec::with_capacity(4),
                native_closed: false,
                inclusive_nested_host_scopes: true,
                paired_generic_dispatch_timers_complete: false,
                tensor_stage_capture: false,
                gpu_time: false,
                gpu_overlap: false,
                numerical_acceptance: false,
                full_model_acceptance: false,
                performance_claim: false,
                production_authority: false,
            },
        })
    }
    pub(crate) fn mark(&mut self, group: &mut Group, phase: &str) -> io::Result<()> {
        let result = (|| {
            if self.terminal || data::phase(self.report.snapshots.len())? != phase {
                return Err(io::Error::other("guarded host record order or terminal"));
            }
            let raw = group.host_observation_v1().map_err(io::Error::other)?;
            let delta = raw
                .checked_delta(&self.previous)
                .map_err(io::Error::other)?;
            let current = snapshot_shared_policy(
                &raw,
                phase,
                self.report.bootstrap.decode.device_ids,
                self.policy.shared(),
            )?;
            let previous = self
                .report
                .snapshots
                .last()
                .ok_or_else(|| io::Error::other("guarded host baseline absent"))?;
            let ranks = [[0; 19]; 2];
            let mut interval = data::Interval {
                host_elapsed_ns: delta.host_elapsed_ns(),
                ranks,
                shared: [0; 4],
            };
            // The native checked_delta validates identity/epochs; subtract the
            // same serialized cumulative counters without assuming disjoint scopes.
            interval.shared = crate::prefix_decode_host_observation_v1::difference(
                &current.shared,
                &previous.shared,
            )?;
            for rank in 0..2 {
                interval.ranks[rank] = crate::prefix_decode_host_observation_v1::difference(
                    &current.ranks[rank].counters,
                    &previous.ranks[rank].counters,
                )?;
            }
            self.report.intervals.push(interval);
            self.report.snapshots.push(current);
            self.previous = raw;
            Ok(())
        })();
        if result.is_err() {
            self.terminal = true;
        }
        result
    }
    pub(crate) fn forward_done(
        &mut self,
        group: &mut Group,
        position: u32,
        elapsed: Duration,
    ) -> io::Result<()> {
        if position >= 4 || self.report.completions.len() != position as usize {
            self.terminal = true;
            return Err(io::Error::other("guarded host forward order"));
        }
        self.mark(group, &format!("forward_{position}/done"))?;
        self.report.forward_host_ns[position as usize] = ns(elapsed)?;
        Ok(())
    }
    pub(crate) fn completed(&mut self, done: &Completion) -> io::Result<()> {
        let position = self.report.completions.len();
        if self.terminal
            || position >= 4
            || done.position as usize != position
            || done.generation != position as u64 + 1
            || self.report.snapshots.len() != 2 + (position + 1) * data::FORWARD_POINTS
        {
            self.terminal = true;
            return Err(io::Error::other("guarded host completed order"));
        }
        self.report.completions.push(done.clone());
        Ok(())
    }
    pub(crate) fn after_close(
        self,
        close: impl FnOnce() -> Result<(), String>,
    ) -> Result<Report, String> {
        if self.terminal {
            return Err("guarded host terminal report before Close".into());
        }
        match self.policy {
            Policy::DefaultFull => finish_report(self.report, close),
            Policy::SharedFull => finish_report_policy(self.report, close, Policy::SharedFull),
            Policy::SharedFullPairedRead => finish_report_policy(self.report, close, Policy::SharedFullPairedRead),
        }
    }
}

fn finish_report(
    report: Report,
    close: impl FnOnce() -> Result<(), String>,
) -> Result<Report, String> {
    finish_report_policy(report, close, Policy::DefaultFull)
}

fn finish_report_policy(
    mut report: Report,
    close: impl FnOnce() -> Result<(), String>,
    policy: Policy,
) -> Result<Report, String> {
    if report.native_closed
        || report.snapshots.len() != data::SNAPSHOTS
        || report.completions.len() != 4
    {
        return Err("guarded host incomplete report before Close".into());
    }
    let started = std::time::Instant::now();
    close()?;
    report.close_host_ns = ns(started.elapsed()).map_err(|e| e.to_string())?;
    report.native_closed = true;
    report.validate_policy(policy).map_err(|e| e.to_string())?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_paired_read_report_waits_for_close_without_control_policy_fallback() {
        let mut report = data::tests::shared_fixture(
            crate::finite_guarded_mlp_decode_wire_v1::InputMode::Autoregressive);
        report.schema = data::PAIRED_READ_SCHEMA.into();
        report.native_closed = false;
        let mut calls = 0;
        assert!(finish_report_policy(report.clone(), || {
            calls += 1;
            Err("Close failed".into())
        }, Policy::SharedFullPairedRead).is_err());
        assert_eq!(calls, 1);
        let complete = finish_report_policy(report, || {
            calls += 1;
            Ok(())
        }, Policy::SharedFullPairedRead).unwrap();
        assert_eq!(calls, 2);
        complete.validate_policy(Policy::SharedFullPairedRead).unwrap();
        assert!(complete.validate_policy(Policy::SharedFull).is_err());
        assert!(complete.validate().is_err());
    }
    #[test]
    fn guarded_shared_host_configures_once_before_observer_and_stops_on_failure() {
        struct Setup {
            events: Vec<&'static str>,
            fail: Option<&'static str>,
        }
        impl Setup {
            fn event(&mut self, event: &'static str) -> io::Result<()> {
                self.events.push(event);
                if self.fail == Some(event) {
                    Err(io::Error::other("injected setup failure"))
                } else {
                    Ok(())
                }
            }
        }
        impl ObserverSetup for Setup {
            fn shared_full(&mut self) -> io::Result<()> {
                self.event("shared-full-only")
            }
            fn enable(&mut self) -> io::Result<()> {
                self.event("observer")
            }
        }
        for policy in [Policy::DefaultFull, Policy::SharedFull, Policy::SharedFullPairedRead] {
            let expected = if policy.shared() {
                vec!["shared-full-only", "observer"]
            } else {
                vec!["observer"]
            };
            let mut setup = Setup {
                events: Vec::new(),
                fail: None,
            };
            configure_observer(&mut setup, policy).unwrap();
            assert_eq!(setup.events, expected);
            for (index, event) in expected.iter().enumerate() {
                let mut setup = Setup {
                    events: Vec::new(),
                    fail: Some(*event),
                };
                assert!(configure_observer(&mut setup, policy).is_err());
                assert_eq!(setup.events, expected[..=index]);
            }
        }
    }

    #[test]
    fn guarded_shared_host_report_requires_close_and_never_enters_default_parser() {
        let mut report = data::tests::shared_fixture(
            crate::finite_guarded_mlp_decode_wire_v1::InputMode::Autoregressive,
        );
        report.native_closed = false;
        let mut calls = 0;
        assert!(
            finish_report_policy(
                report.clone(),
                || {
                    calls += 1;
                    Err("Close failed".into())
                },
                Policy::SharedFull
            )
            .is_err()
        );
        assert_eq!(calls, 1);
        let complete = finish_report_policy(
            report.clone(),
            || {
                calls += 1;
                Ok(())
            },
            Policy::SharedFull,
        )
        .unwrap();
        assert_eq!(calls, 2);
        complete.validate_policy(Policy::SharedFull).unwrap();
        assert!(complete.validate().is_err());
        assert!(
            std::panic::catch_unwind(|| finish_report_policy(
                report,
                || panic!("Close unwind"),
                Policy::SharedFull
            ))
            .is_err()
        );
    }

    fn pending() -> Report {
        let mut value = crate::guarded_mlp_host_observation_v1::tests::fixture(
            crate::finite_guarded_mlp_decode_wire_v1::InputMode::TeacherForced,
        );
        value.native_closed = false;
        value
    }
    #[test]
    fn guarded_host_report_publication_requires_actual_successful_close_once() {
        let mut calls = 0;
        let result = finish_report(pending(), || {
            calls += 1;
            Err("injected Close failure".into())
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
        let result = finish_report(pending(), || {
            calls += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert!(result.native_closed);
        result.validate().unwrap();
        assert!(
            finish_report(result, || {
                calls += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(calls, 2);
    }
    #[test]
    fn guarded_host_partial_or_panicking_close_never_publishes() {
        let mut calls = 0;
        let mut value = pending();
        value.snapshots.pop();
        assert!(
            finish_report(value, || {
                calls += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(calls, 0);
        assert!(
            std::panic::catch_unwind(|| finish_report(pending(), || panic!(
                "injected Close unwind"
            )))
            .is_err()
        );
    }
}
