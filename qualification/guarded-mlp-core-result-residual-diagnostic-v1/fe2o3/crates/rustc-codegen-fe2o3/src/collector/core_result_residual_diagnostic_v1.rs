//! Observes already-resolved core residual helpers; grants no admission.

use super::*;
use rustc_hir::def_id::CrateNum;
use std::fmt::Write as _;

const MATCH_CAP: usize = 4;
const SCAN_CAP: usize = fe2o3_rustc_front::MAX_FUNCTIONS_V1;

#[derive(Default)]
struct Census {
    scanned: usize,
    matched: usize,
    scan_truncated: bool,
    match_truncated: bool,
}

impl Census {
    fn scan(&mut self) -> bool {
        if self.scanned == SCAN_CAP {
            self.scan_truncated = true;
            return false;
        }
        self.scanned += 1;
        true
    }

    fn matched(&mut self) -> bool {
        self.matched += 1;
        if self.matched > MATCH_CAP {
            self.match_truncated = true;
            return false;
        }
        true
    }
}

fn result_types<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    core: CrateNum,
) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    let TyKind::Adt(definition, arguments) = ty.kind() else {
        return None;
    };
    let mut variants = definition.variants().iter();
    if !tcx.is_diagnostic_item(sym::Result, definition.did())
        || definition.did().krate != core
        || !definition.is_enum()
        || arguments.len() != 2
        || variants.next().map(|variant| variant.def_id) != tcx.lang_items().result_ok_variant()
        || variants.next().map(|variant| variant.def_id) != tcx.lang_items().result_err_variant()
        || variants.next().is_some()
    {
        return None;
    }
    Some((arguments[0].as_type()?, arguments[1].as_type()?))
}

fn branch_residual<'tcx>(tcx: TyCtxt<'tcx>, branch: Instance<'tcx>) -> Option<Ty<'tcx>> {
    let signature = tcx.instantiate_bound_regions_with_erased(
        tcx.fn_sig(branch.def_id()).instantiate(tcx, branch.args),
    );
    let signature = tcx
        .try_normalize_erasing_regions(TypingEnv::fully_monomorphized(), signature)
        .ok()?;
    let TyKind::Adt(definition, arguments) = signature.output().kind() else {
        return None;
    };
    let mut variants = definition.variants().iter();
    if !tcx.is_diagnostic_item(sym::ControlFlow, definition.did())
        || definition.did().krate != branch.def_id().krate
        || !definition.is_enum()
        || arguments.len() != 2
        || variants.next().map(|variant| variant.def_id) != tcx.lang_items().cf_continue_variant()
        || variants.next().map(|variant| variant.def_id) != tcx.lang_items().cf_break_variant()
        || variants.next().is_some()
        || arguments[1].as_type()? != branch.args[0].as_type()?
    {
        return None;
    }
    let residual = arguments[0].as_type()?;
    let (empty, error) = result_types(tcx, residual, branch.def_id().krate)?;
    let TyKind::Adt(empty_definition, empty_arguments) = empty.kind() else {
        return None;
    };
    if error != branch.args[1].as_type()?
        || empty_definition.did().krate != branch.def_id().krate
        || !empty_definition.is_enum()
        || !empty_definition.variants().is_empty()
        || !empty_arguments.is_empty()
    {
        return None;
    }
    // The exact residual type comes from the observed genuine branch signature.
    // No guessed FromResidual instance or name-based Infallible authority.
    Some(residual)
}

fn selected_residual<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    residual: Ty<'tcx>,
    error: Ty<'tcx>,
) -> bool {
    if !matches!(instance.def, InstanceKind::Item(_))
        || !crate::production_rustc_intrinsic_v1::is_reviewed_core_function_v1(tcx, instance)
        || tcx.def_kind(instance.def_id()) != DefKind::AssocFn
        || !super::super::is_fully_monomorphized(tcx, instance)
        || !tcx.is_mir_available(instance.def_id())
        || instance.args.len() != 3
        || instance.args.iter().any(|argument| argument.as_type().is_none())
        || instance.args[1].as_type() != Some(error)
    {
        return false;
    }
    let Some(method) = tcx.lang_items().from_residual_fn() else {
        return false;
    };
    if tcx.associated_item(instance.def_id()).trait_item_def_id() != Some(method) {
        return false;
    }
    let signature = tcx.instantiate_bound_regions_with_erased(
        tcx.fn_sig(instance.def_id()).instantiate(tcx, instance.args),
    );
    let Ok(signature) =
        tcx.try_normalize_erasing_regions(TypingEnv::fully_monomorphized(), signature)
    else {
        return false;
    };
    signature.safety == Safety::Safe
        && signature.abi == ExternAbi::Rust
        && !signature.c_variadic
        && signature.inputs() == [residual]
        && result_types(tcx, signature.output(), instance.def_id().krate)
            == instance.args[0].as_type().zip(instance.args[2].as_type())
}

fn render_residual<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    index: usize,
    output: &mut BoundedText,
) -> Result<bool, fmt::Error> {
    writeln!(output, "core_helper role=residual index={index} conversion_bodies_dumped=false")?;
    let complete = render_body(tcx, instance, output)?;
    writeln!(output, "core_helper_end role=residual index={index} body_render_complete={complete}")?;
    Ok(complete)
}

pub(super) fn render_residuals<'tcx>(
    tcx: TyCtxt<'tcx>,
    branch: Instance<'tcx>,
    related: impl Iterator<Item = Instance<'tcx>>,
    output: &mut BoundedText,
) -> Result<bool, fmt::Error> {
    writeln!(output, "residual_selection match_cap={MATCH_CAP} scan_cap={SCAN_CAP} authority=none")?;
    let Some(residual) = branch_residual(tcx, branch) else {
        writeln!(output, "residual_binding=unavailable")?;
        return Ok(false);
    };
    let Some(error) = branch.args[1].as_type() else {
        return Ok(false);
    };
    writeln!(output, "residual_binding={residual:?} error_nominal={:?} origin=observed_branch_signature", nominal(error))?;
    let mut census = Census::default();
    let mut complete = true;
    for candidate in related {
        if !census.scan() {
            break;
        }
        if selected_residual(tcx, candidate, residual, error) {
            if !census.matched() {
                break;
            }
            complete &= render_residual(tcx, candidate, census.matched - 1, output)?;
        }
    }
    writeln!(output, "residual_selection_end scanned={} matched={} rendered={} scan_truncated={} match_truncated={} absent={} authority=none",
        census.scanned, census.matched, census.matched.min(MATCH_CAP), census.scan_truncated,
        census.match_truncated, census.matched == 0)?;
    Ok(complete && !census.scan_truncated && !census.match_truncated)
}

#[cfg(test)]
#[path = "core_result_residual_diagnostic_v1_tests.rs"]
mod tests;
