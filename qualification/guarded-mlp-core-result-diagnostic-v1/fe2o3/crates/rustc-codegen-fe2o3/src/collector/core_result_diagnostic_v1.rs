//! Bounded compiler-only evidence immediately before an unchanged rejection.

use rustc_abi::ExternAbi;
use rustc_hir::{Safety, def::DefKind};
use rustc_middle::ty::{EarlyBinder, Instance, InstanceKind, Ty, TyCtxt, TyKind, TypingEnv};
use rustc_span::sym;
use std::fmt::{self, Write as _};
use std::io::{self, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};

const BYTE_CAP: usize = 64 * 1024;
const TRAILER_RESERVE: usize = 192;
const ITEM_CAP: usize = 128;
const STATEMENT_CAP: usize = 1024;
static EMITTED: AtomicBool = AtomicBool::new(false);

struct BoundedText {
    text: String,
    limit: usize,
}

impl fmt::Write for BoundedText {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > self.limit.saturating_sub(self.text.len()) {
            return Err(fmt::Error);
        }
        self.text.try_reserve_exact(text.len()).map_err(|_| fmt::Error)?;
        self.text.push_str(text);
        Ok(())
    }
}

fn diagnostic_text(render: impl FnOnce(&mut BoundedText) -> Result<bool, fmt::Error>) -> String {
    let mut output = BoundedText {
        text: String::new(),
        limit: BYTE_CAP - TRAILER_RESERVE,
    };
    let status = match render(&mut output) {
        Ok(true) => "complete",
        Ok(false) => "partial",
        Err(_) => "truncated_or_render_error",
    };
    let body_bytes = output.text.len();
    output.limit = BYTE_CAP;
    let _ = writeln!(
        output,
        "\nFE2O3_CORE_RESULT_MIR_END status={status} body_bytes={body_bytes} authority=none"
    );
    output.text
}

fn selected_result_branch<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
    if !matches!(instance.def, InstanceKind::Item(_))
        || !crate::production_rustc_intrinsic_v1::is_reviewed_core_function_v1(tcx, instance)
        || tcx.def_kind(instance.def_id()) != DefKind::AssocFn
        || !super::is_fully_monomorphized(tcx, instance)
        || !tcx.is_mir_available(instance.def_id())
        || instance.args.len() != 2
        || instance.args.iter().any(|argument| argument.as_type().is_none())
    {
        return false;
    }
    let Some(branch) = tcx.lang_items().branch_fn() else {
        return false;
    };
    if tcx.associated_item(instance.def_id()).trait_item_def_id() != Some(branch) {
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
    if signature.safety != Safety::Safe || signature.abi != ExternAbi::Rust || signature.c_variadic {
        return false;
    }
    let [input] = signature.inputs() else {
        return false;
    };
    let TyKind::Adt(definition, arguments) = input.kind() else {
        return false;
    };
    let mut variants = definition.variants().iter();
    tcx.is_diagnostic_item(sym::Result, definition.did())
        && definition.did().krate == instance.def_id().krate
        && definition.is_enum()
        && *arguments == instance.args
        && variants.next().map(|variant| variant.def_id) == tcx.lang_items().result_ok_variant()
        && variants.next().map(|variant| variant.def_id) == tcx.lang_items().result_err_variant()
        && variants.next().is_none()
}

fn nominal(ty: Ty<'_>) -> Option<rustc_hir::def_id::DefId> {
    match ty.kind() {
        TyKind::Adt(definition, _) => Some(definition.did()),
        _ => None,
    }
}

fn render<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    output: &mut BoundedText,
) -> Result<bool, fmt::Error> {
    writeln!(output, "FE2O3_CORE_RESULT_MIR_BEGIN version=1 authority=none byte_cap={BYTE_CAP}")?;
    writeln!(
        output,
        "instance={instance:?}\ndefinition={:?}\ntrait_method={:?}\narguments={:?}",
        instance.def_id(),
        tcx.associated_item(instance.def_id()).trait_item_def_id(),
        instance.args
    )?;
    let signature = tcx.instantiate_bound_regions_with_erased(
        tcx.fn_sig(instance.def_id()).instantiate(tcx, instance.args),
    );
    let signature = tcx.try_normalize_erasing_regions(TypingEnv::fully_monomorphized(), signature);
    writeln!(output, "normalized_signature={signature:?}")?;
    for (index, argument) in instance.args.iter().enumerate() {
        writeln!(output, "argument_nominal[{index}]={:?}", argument.as_type().and_then(nominal))?;
    }
    let mut complete = signature.is_ok();
    if let Ok(signature) = signature {
        for (index, ty) in signature.inputs().iter().enumerate() {
            writeln!(output, "input_nominal[{index}]={:?}", nominal(*ty))?;
        }
        writeln!(output, "output_nominal={:?}", nominal(signature.output()))?;
    }
    let body = tcx.instance_mir(instance.def);
    writeln!(
        output,
        "body arg_count={} locals={} blocks={} scopes={}",
        body.arg_count,
        body.local_decls.len(),
        body.basic_blocks.len(),
        body.source_scopes.len()
    )?;
    if body.local_decls.len() > ITEM_CAP
        || body.basic_blocks.len() > ITEM_CAP
        || body.source_scopes.len() > ITEM_CAP
    {
        return Ok(false);
    }
    let mut statements = 0_usize;
    for block in body.basic_blocks.iter() {
        if block.statements.len() > STATEMENT_CAP - statements {
            return Ok(false);
        }
        statements += block.statements.len();
    }
    writeln!(output, "statements={statements} item_cap={ITEM_CAP} statement_cap={STATEMENT_CAP}")?;
    for (index, local) in body.local_decls.iter_enumerated() {
        let normalized = instance.try_instantiate_mir_and_normalize_erasing_regions(
            tcx,
            TypingEnv::fully_monomorphized(),
            EarlyBinder::bind(local.ty),
        );
        complete &= normalized.is_ok();
        let nominal_type = normalized.as_ref().ok().copied().and_then(nominal);
        writeln!(
            output,
            "local[{index:?}] raw={:?} normalized={normalized:?} nominal={nominal_type:?} scope={:?}",
            local.ty,
            local.source_info.scope
        )?;
    }
    for (index, scope) in body.source_scopes.iter_enumerated() {
        let normalized = scope.inlined.map(|(raw, _)| {
            instance.try_instantiate_mir_and_normalize_erasing_regions(
                tcx,
                TypingEnv::fully_monomorphized(),
                EarlyBinder::bind(raw),
            )
        });
        complete &= normalized.as_ref().is_none_or(|result| result.is_ok());
        writeln!(output, "scope[{index:?}] data={scope:?} normalized_inline={normalized:?}")?;
    }
    for (index, block) in body.basic_blocks.iter_enumerated() {
        writeln!(
            output,
            "block[{index:?}] cleanup={} statements={}",
            block.is_cleanup,
            block.statements.len()
        )?;
        for (statement_index, statement) in block.statements.iter().enumerate() {
            writeln!(
                output,
                "  statement[{statement_index}] scope={:?} kind={:?}",
                statement.source_info.scope,
                statement.kind
            )?;
        }
        if let Some(terminator) = &block.terminator {
            writeln!(
                output,
                "  terminator scope={:?} kind={:?}",
                terminator.source_info.scope,
                terminator.kind
            )?;
        } else {
            complete = false;
            writeln!(output, "  terminator missing")?;
        }
    }
    Ok(complete)
}

pub(super) fn emit_core_result_diagnostic_v1<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) {
    if std::env::var_os("FE2O3_DIAG_CORE_RESULT_MIR_V1").as_deref()
        != Some(std::ffi::OsStr::new("1"))
        || !selected_result_branch(tcx, instance)
        || EMITTED.swap(true, Ordering::Relaxed)
    {
        return;
    }
    let text = diagnostic_text(|output| render(tcx, instance, output));
    // Stderr failure cannot replace the original refusal with acceptance.
    let _ = io::stderr().lock().write_all(text.as_bytes());
}

#[cfg(test)]
#[path = "core_result_diagnostic_v1_tests.rs"]
mod tests;
