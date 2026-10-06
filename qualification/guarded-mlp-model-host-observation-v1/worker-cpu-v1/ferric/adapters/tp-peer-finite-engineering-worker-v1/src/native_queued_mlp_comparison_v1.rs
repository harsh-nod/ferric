//! One complete forward, with an explicit same-owner layer-zero comparison.

use super::{Active, ForwardInput, ForwardRun, NativeOwner, Phase, Result, validate_sealed};
use crate::finite_queued_mlp_comparison_wire_v1::{Bootstrap, TOKEN};
use crate::finite_queued_projection_comparison_wire_v1::Bootstrap as ProjectionBootstrap;
use crate::finite_setup_wire_v1::Scope;
use crate::forward_sequence::{Backend, InputMode, Sequence};
use crate::native_catalog::SourceScope;
use crate::resident_layer::queued_projection_v1 as projection;
use crate::resident_layer::{self, LayerCompletion, queued_mlp_v1 as queued};

pub(crate) struct ComparisonRun {
    pub(crate) forward: ForwardRun,
    pub(crate) comparison: queued::Comparison,
}
pub(crate) struct ProjectionRun {
    pub(crate) forward: ForwardRun,
    pub(crate) comparison: projection::Comparison,
}

enum Images {
    Mlp(queued::ReviewedImages),
    Projection(projection::ReviewedImage),
}
enum Loaded {
    Mlp(queued::LoadedArtifacts),
    Projection(projection::LoadedArtifacts),
}
enum Report {
    Mlp(queued::Comparison),
    Projection(projection::Comparison),
}
struct ComparedRun {
    forward: ForwardRun,
    comparison: Report,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gate {
    Fresh,
    Running,
    Completed,
    Failed,
}
impl Gate {
    fn begin(&mut self, input: &ForwardInput) -> Result<()> {
        if *self != Self::Fresh
            || input.generation != 1
            || input.cache_metadata[0] != 0
            || input.token != TOKEN
        {
            *self = Self::Failed;
            return Err("queued comparison permits only its declared first forward".into());
        }
        *self = Self::Running;
        Ok(())
    }
    fn commit(&mut self) -> Result<()> {
        if *self != Self::Running {
            *self = Self::Failed;
            return Err("queued comparison commit phase".into());
        }
        *self = Self::Completed;
        Ok(())
    }
}

struct SharedOwner {
    owner: NativeOwner,
    queued: Loaded,
    sequence: Sequence,
    gate: Gate,
    timeout_ms: u32,
}
pub(crate) struct ComparisonOwner {
    shared: SharedOwner,
}
pub(crate) struct ProjectionOwner {
    shared: SharedOwner,
}
fn poison(owner: &mut NativeOwner) {
    owner.catalog.phase = Phase::Terminal;
    if let Some(states) = owner.sealed_states.as_mut() {
        states.poison();
    }
}
fn require_scope(scope: &SourceScope, expected: &Scope) -> Result<()> {
    if scope.bundle_id != expected.bundle_id
        || scope.model_id != expected.model_id
        || scope.session != expected.session
        || scope.pool_identity != expected.pool_identity
        || scope.group_id != expected.group_id
        || scope.child_identity != expected.child_identity
    {
        return Err("queued comparison differs from frozen setup scope".into());
    }
    Ok(())
}
impl SharedOwner {
    /// # Safety
    /// Same authenticated parent/model and exact machine-code premises as the
    /// existing finite owner. Images were pinned before open and are loaded only
    /// into this consumed owner. This profile is diagnostic dual execution,
    /// never a queued substitution for finite task ownership or production.
    unsafe fn from_sealed(
        mut owner: NativeOwner,
        scope: &Scope,
        timeout_ms: u32,
        bootstrap_validation: Result<()>,
        images: Images,
    ) -> Result<Self> {
        let result = (|| {
            bootstrap_validation?;
            validate_sealed(&owner, timeout_ms)?;
            require_scope(&owner.catalog.scope, scope)?;
            let sequence = Sequence::new(owner.registration_sha256(), InputMode::TeacherForced)?;
            let queued = match images {
                Images::Mlp(images) => {
                    Loaded::Mlp(queued::load(&mut owner.catalog.backend, images)?)
                }
                Images::Projection(image) => {
                    Loaded::Projection(projection::load(&mut owner.catalog.backend, image)?)
                }
            };
            Ok((sequence, queued))
        })();
        let (sequence, queued) = match result {
            Ok(value) => value,
            Err(error) => {
                poison(&mut owner);
                return Err(error);
            }
        };
        Ok(Self {
            owner,
            queued,
            sequence,
            gate: Gate::Fresh,
            timeout_ms,
        })
    }
    fn run(&mut self, input: &ForwardInput) -> Result<ComparedRun> {
        if let Err(error) = self.gate.begin(input) {
            poison(&mut self.owner);
            return Err(error);
        }
        let mut active = Compared {
            active: Active {
                owner: &mut self.owner,
                timeout_ms: self.timeout_ms,
                layer_hidden: Vec::with_capacity(36),
                final_normalized: Vec::new(),
                logits: Vec::new(),
                capture: None,
                captured_layer0: None,
                reuse: None,
            },
            queued: &self.queued,
            comparison: None,
        };
        // The unchanged sequencer checks metadata, all36 layers, tail, fence
        // and actual finite-state commit. This wrapper permits only its first run.
        let completion = match self.sequence.run(&mut active, input) {
            Ok(value) => value,
            Err(error) => {
                self.gate = Gate::Failed;
                return Err(error);
            }
        };
        let Some(comparison) = active.comparison.take() else {
            active.poison();
            self.gate = Gate::Failed;
            return Err("queued comparison was not performed".into());
        };
        if let Err(error) = self.gate.commit() {
            active.poison();
            return Err(error);
        }
        Ok(ComparedRun {
            forward: ForwardRun {
                completion,
                layer_hidden: active.active.layer_hidden,
                final_normalized: active.active.final_normalized,
                logits: active.active.logits,
            },
            comparison,
        })
    }
    fn close(mut self) -> Result<()> {
        if self.gate != Gate::Completed {
            poison(&mut self.owner);
            return Err("queued comparison Close before committed forward".into());
        }
        self.owner.close_setup()
    }
}

impl ComparisonOwner {
    /// # Safety
    /// Unchanged authentic-parent, reviewed-image and exclusive-owner contract.
    pub(crate) unsafe fn from_sealed(
        owner: NativeOwner,
        bootstrap: &Bootstrap,
        images: queued::ReviewedImages,
    ) -> Result<Self> {
        let validation = bootstrap
            .validate(
                bootstrap.device_ids,
                bootstrap.timeout_ms,
                std::process::id(),
            )
            .map_err(|e| e.to_string());
        // SAFETY: the unchanged MLP wrapper selects only its closed image kind.
        Ok(Self {
            shared: unsafe {
                SharedOwner::from_sealed(
                    owner,
                    &bootstrap.scope,
                    bootstrap.timeout_ms,
                    validation,
                    Images::Mlp(images),
                )
            }?,
        })
    }
    pub(crate) fn run(&mut self, input: &ForwardInput) -> Result<ComparisonRun> {
        let run = self.shared.run(input)?;
        match run.comparison {
            Report::Mlp(comparison) => Ok(ComparisonRun {
                forward: run.forward,
                comparison,
            }),
            Report::Projection(_) => {
                self.shared.gate = Gate::Failed;
                poison(&mut self.shared.owner);
                Err("MLP wrapper received projection report".into())
            }
        }
    }
    pub(crate) fn close(self) -> Result<()> {
        self.shared.close()
    }
}
impl ProjectionOwner {
    /// # Safety
    /// The distinct projection bootstrap retains authentic first-token custody;
    /// the exact V3 image is loaded in this consumed exclusive owner only.
    pub(crate) unsafe fn from_sealed(
        owner: NativeOwner,
        bootstrap: &ProjectionBootstrap,
        image: projection::ReviewedImage,
    ) -> Result<Self> {
        let validation = bootstrap
            .validate(
                bootstrap.device_ids,
                bootstrap.timeout_ms,
                std::process::id(),
            )
            .map_err(|e| e.to_string());
        // SAFETY: only this explicit wrapper can select projection artifacts.
        Ok(Self {
            shared: unsafe {
                SharedOwner::from_sealed(
                    owner,
                    &bootstrap.scope,
                    bootstrap.timeout_ms,
                    validation,
                    Images::Projection(image),
                )
            }?,
        })
    }
    pub(crate) fn run(&mut self, input: &ForwardInput) -> Result<ProjectionRun> {
        let run = self.shared.run(input)?;
        match run.comparison {
            Report::Projection(comparison) => Ok(ProjectionRun {
                forward: run.forward,
                comparison,
            }),
            Report::Mlp(_) => {
                self.shared.gate = Gate::Failed;
                poison(&mut self.shared.owner);
                Err("projection wrapper received MLP report".into())
            }
        }
    }
    pub(crate) fn close(self) -> Result<()> {
        self.shared.close()
    }
}

struct Compared<'a> {
    active: Active<'a>,
    queued: &'a Loaded,
    comparison: Option<Report>,
}
impl Backend for Compared<'_> {
    fn upload_metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.active.upload_metadata(input)
    }
    fn embedding(&mut self, token: u32) -> Result<[u64; 2]> {
        self.active.embedding(token)
    }
    fn begin_states(&mut self, input: &ForwardInput) -> Result<()> {
        self.active.begin_states(input)
    }
    fn layer(&mut self, layer: usize) -> Result<LayerCompletion> {
        if layer != 0 {
            return self.active.layer(layer);
        }
        if self.comparison.is_some() {
            return Err("queued comparison layer zero repeated".into());
        }
        let owner = &mut *self.active.owner;
        let roots = owner
            .layer_bindings
            .get(layer)
            .ok_or("queued comparison layer bounds")?;
        let artifacts = owner
            .artifacts
            .as_ref()
            .ok_or("queued comparison resident artifacts missing")?;
        let states = owner
            .sealed_states
            .as_mut()
            .ok_or("queued comparison finite states missing")?;
        // SAFETY: the same retained owner/roots/state as the normal layer, and
        // additional exact objects loaded by ComparisonOwner in that same group.
        let (finite, comparison) = match self.queued {
            Loaded::Mlp(queued) => {
                let result = unsafe {
                    resident_layer::execute_layer_with_mlp_comparison(
                        &mut owner.catalog.backend,
                        states,
                        artifacts,
                        queued,
                        layer,
                        roots,
                        self.active.timeout_ms,
                        queued::ComparisonProfile::FiniteThenQueuedLayerZeroV1,
                    )
                }?;
                (result.finite, Report::Mlp(result.queued))
            }
            Loaded::Projection(queued) => {
                // SAFETY: same retained group/roots/state, separately selected
                // exact projection image and closed after-prefix coordinator.
                let result = unsafe {
                    resident_layer::execute_layer_with_projection_comparison(
                        &mut owner.catalog.backend,
                        states,
                        artifacts,
                        queued,
                        layer,
                        roots,
                        self.active.timeout_ms,
                        projection::ComparisonProfile::FiniteThenQueuedProjectionsLayerZeroV1,
                    )
                }?;
                (result.finite, Report::Projection(result.queued))
            }
        };
        let hidden = roots.final_hidden;
        self.active.retain_layer_hidden(layer, hidden)?;
        self.comparison = Some(comparison);
        Ok(finite)
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.active.tail()
    }
    fn idle_fence(&mut self) -> Result<()> {
        self.active.idle_fence()
    }
    fn commit_states(&mut self) -> Result<()> {
        self.active.commit_states()
    }
    fn poison(&mut self) {
        self.active.poison()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> ForwardInput {
        ForwardInput {
            registration: [1; 32],
            generation: 1,
            token: TOKEN,
            cache_metadata: std::array::from_fn(|i| if i == 0 { 0 } else { (i - 1) as u32 }),
            rotary_bits: [0; 128],
        }
    }
    #[test]
    fn one_committed_forward_cannot_be_repeated_or_relabelled() {
        let mut gate = Gate::Fresh;
        gate.begin(&input()).unwrap();
        gate.commit().unwrap();
        assert_eq!(gate, Gate::Completed);
        assert!(gate.begin(&input()).is_err());
        assert_eq!(gate, Gate::Failed);
        for mutation in 0..3 {
            let mut value = input();
            match mutation {
                0 => value.generation = 2,
                1 => value.cache_metadata[0] = 1,
                _ => value.token += 1,
            }
            let mut gate = Gate::Fresh;
            assert!(gate.begin(&value).is_err());
            assert_eq!(gate, Gate::Failed);
            assert!(gate.commit().is_err());
        }
    }
    #[test]
    fn no_commit_before_run_or_after_terminal_failure() {
        for phase in [Gate::Fresh, Gate::Completed, Gate::Failed] {
            let mut gate = phase;
            assert!(gate.commit().is_err());
            assert_eq!(gate, Gate::Failed);
        }
        let mut gate = Gate::Fresh;
        gate.begin(&input()).unwrap();
        assert!(gate.begin(&input()).is_err());
        assert_eq!(gate, Gate::Failed);
    }

    #[test]
    fn shared_owner_preserves_every_frozen_scope_field_and_zero_group_id() {
        let expected = Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 4,
            group_id: 0,
            child_identity: 5,
        };
        let actual = SourceScope {
            bundle_id: expected.bundle_id,
            model_id: expected.model_id,
            session: expected.session,
            pool_identity: expected.pool_identity,
            group_id: expected.group_id,
            child_identity: expected.child_identity,
        };
        require_scope(&actual, &expected).unwrap();
        for field in 0..6 {
            let mut changed = actual.clone();
            match field {
                0 => changed.bundle_id[0] ^= 1,
                1 => changed.model_id[0] ^= 1,
                2 => changed.session[0] ^= 1,
                3 => changed.pool_identity += 1,
                4 => changed.group_id += 1,
                _ => changed.child_identity += 1,
            }
            assert!(require_scope(&changed, &expected).is_err());
        }
    }
}
