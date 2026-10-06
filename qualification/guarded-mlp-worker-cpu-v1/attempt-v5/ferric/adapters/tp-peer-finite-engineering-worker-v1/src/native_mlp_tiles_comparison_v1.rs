//! Distinct one-forward owner. V1 remains the full model's state contract.
use super::{
    Active, AllocationProfile, ForwardInput, ForwardRun, NativeOwner, Phase, Result,
    validate_sealed,
};
use crate::finite_mlp_tiles_comparison_wire_v1::{Bootstrap, TOKEN};
use crate::finite_setup_wire_v1::Scope;
use crate::forward_sequence::{Backend, InputMode, Sequence};
use crate::native_catalog::SourceScope;
use crate::resident_layer::{self, LayerCompletion, mlp_tiles_v2 as tiles};

pub(crate) struct ComparisonRun {
    pub(crate) forward: ForwardRun,
    pub(crate) comparison: tiles::Comparison,
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
            return Err("V1/V2 comparison permits only its first forward".into());
        }
        *self = Self::Running;
        Ok(())
    }
    fn commit(&mut self) -> Result<()> {
        if *self != Self::Running {
            *self = Self::Failed;
            return Err("V1/V2 comparison commit phase".into());
        }
        *self = Self::Completed;
        Ok(())
    }
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
        return Err("V1/V2 comparison differs from authenticated setup scope".into());
    }
    Ok(())
}
pub(crate) struct ComparisonOwner {
    owner: NativeOwner,
    tiles: tiles::LoadedArtifacts,
    sequence: Sequence,
    gate: Gate,
    timeout_ms: u32,
}
impl ComparisonOwner {
    /// # Safety
    /// The owning parent supplies authenticated original model roles and the
    /// actual reviewed V2 image. Pinning arbitrary bytes is not artifact admission.
    pub(crate) unsafe fn from_sealed(
        mut owner: NativeOwner,
        bootstrap: &Bootstrap,
        image: tiles::Image,
    ) -> Result<Self> {
        let result = (|| {
            bootstrap
                .validate(
                    bootstrap.device_ids,
                    bootstrap.timeout_ms,
                    std::process::id(),
                )
                .map_err(|e| e.to_string())?;
            validate_sealed(&owner, bootstrap.timeout_ms)?;
            require_scope(&owner.catalog.scope, &bootstrap.scope)?;
            let sequence = Sequence::new(owner.registration_sha256(), InputMode::TeacherForced)?;
            let tiles = tiles::load(&mut owner.catalog.backend, image)?;
            Ok((sequence, tiles))
        })();
        let (sequence, tiles) = match result {
            Ok(value) => value,
            Err(error) => {
                poison(&mut owner);
                return Err(error);
            }
        };
        Ok(Self {
            owner,
            tiles,
            sequence,
            gate: Gate::Fresh,
            timeout_ms: bootstrap.timeout_ms,
        })
    }
    pub(crate) fn run(&mut self, input: &ForwardInput) -> Result<ComparisonRun> {
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
            tiles: &mut self.tiles,
            comparison: None,
        };
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
            return Err("V2 comparison missing".into());
        };
        if let Err(error) = self.gate.commit() {
            active.poison();
            return Err(error);
        }
        Ok(ComparisonRun {
            forward: ForwardRun {
                completion,
                layer_hidden: active.active.layer_hidden,
                final_normalized: active.active.final_normalized,
                logits: active.active.logits,
            },
            comparison,
        })
    }
    pub(crate) fn close(mut self) -> Result<()> {
        let result = (|| {
            if self.gate != Gate::Completed {
                return Err("V2 comparison Close before whole forward".into());
            }
            AllocationProfile::TilesComparisonV1.validate(
                &self
                    .owner
                    .catalog
                    .backend
                    .preflight_additional_allocations_v1(&[0, 0])?,
            )?;
            self.owner.close_setup()
        })();
        if result.is_err() {
            self.gate = Gate::Failed;
            poison(&mut self.owner);
        }
        result
    }
}
struct Compared<'a> {
    active: Active<'a>,
    tiles: &'a mut tiles::LoadedArtifacts,
    comparison: Option<tiles::Comparison>,
}
impl Backend for Compared<'_> {
    fn upload_metadata(&mut self, input: &ForwardInput) -> Result<()> {
        self.active
            .upload_metadata_for(input, AllocationProfile::TilesComparisonV1)
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
            return Err("V2 comparison layer repeated".into());
        }
        let owner = &mut *self.active.owner;
        let roots = owner
            .layer_bindings
            .get(layer)
            .ok_or("V2 comparison layer bounds")?;
        let artifacts = owner.artifacts.as_ref().ok_or("V1 artifacts missing")?;
        let states = owner
            .sealed_states
            .as_mut()
            .ok_or("V1 state roster missing")?;
        // SAFETY: catalog-derived original roles, same exclusive group and fresh
        // V2 states; the closed layer hook compares before the first residual use.
        let (finite, comparison) = unsafe {
            resident_layer::execute_layer_with_tiles_comparison(
                &mut owner.catalog.backend,
                states,
                artifacts,
                self.tiles,
                layer,
                roots,
                self.active.timeout_ms,
            )
        }?;
        let hidden = roots.final_hidden;
        self.active.retain_layer_hidden(layer, hidden)?;
        self.comparison = Some(comparison);
        Ok(finite)
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.active.tail()
    }
    fn idle_fence(&mut self) -> Result<()> {
        self.active
            .idle_fence_for(AllocationProfile::TilesComparisonV1)
    }
    fn commit_states(&mut self) -> Result<()> {
        self.active.commit_states()
    }
    fn poison(&mut self) {
        self.active.poison()
    }
}

#[cfg(test)]
#[path = "native_mlp_tiles_comparison_v1_tests.rs"]
mod tests;
