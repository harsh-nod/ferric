//! Bounded same-root compiler observations before an unchanged rejection.

use rustc_abi::ExternAbi;
use rustc_hir::{Safety, def::DefKind};
use rustc_middle::ty::{EarlyBinder, Instance, InstanceKind, Ty, TyCtxt, TyKind, TypingEnv};
use rustc_span::sym;
use std::fmt::{self, Write as _};
use std::io::{self, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};

const BYTE_CAP: usize = 64 * 1024;
const TRAILER_RESERVE: usize = 512;
const ITEM_CAP: usize = 128;
const STATEMENT_CAP: usize = 1024;
const SCAN_CAP: usize = 4096;
const BODY_CAP: usize = 24;
static EMITTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Default)]
struct Counts {
    scanned: usize,
    scan_complete: bool,
    matched: usize,
    retained: usize,
    full: usize,
    partial: usize,
}

struct BoundedText {
    text: String,
    limit: usize,
    counts: Counts,
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
        counts: Counts::default(),
    };
    let status = match render(&mut output) {
        Ok(true) => "complete",
        Ok(false) => "partial",
        Err(_) => "truncated_or_render_error",
    };
    let counts = output.counts;
    let omitted = counts.matched.saturating_sub(counts.full + counts.partial);
    output.limit = BYTE_CAP;
    let _ = writeln!(
        output,
        "\ncohort_counts scanned={} scan_complete={} matched={} retained={} full={} partial={} omitted={} scan_cap={SCAN_CAP} body_cap={BODY_CAP}",
        counts.scanned, counts.scan_complete, counts.matched, counts.retained,
        counts.full, counts.partial, omitted,
    );
    let body_bytes = output.text.len();
    let _ = writeln!(
        output,
        "\nFE2O3_CORE_CHECKED_ATTENTION_MIR_END status={status} body_bytes={body_bytes} authority=none"
    );
    output.text
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Family {
    UsizeCheckedAdd,
    Integer,
    OptionOkOr,
    OptionAndThen,
}

fn option_argument<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    core: rustc_hir::def_id::CrateNum,
) -> Option<Ty<'tcx>> {
    let TyKind::Adt(adt, arguments) = *ty.kind() else {
        return None;
    };
    if adt.did().krate != core || !tcx.is_diagnostic_item(sym::Option, adt.did())
        || !adt.is_enum() || arguments.len() != 1
    {
        return None;
    }
    arguments[0].as_type()
}

fn result_arguments<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    core: rustc_hir::def_id::CrateNum,
) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    let TyKind::Adt(adt, arguments) = *ty.kind() else {
        return None;
    };
    if adt.did().krate != core || !tcx.is_diagnostic_item(sym::Result, adt.did())
        || !adt.is_enum() || arguments.len() != 2
    {
        return None;
    }
    Some((arguments[0].as_type()?, arguments[1].as_type()?))
}

fn selected<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> Option<Family> {
    if !matches!(instance.def, InstanceKind::Item(_))
        || !crate::production_rustc_intrinsic_v1::is_reviewed_core_function_v1(tcx, instance)
        || tcx.def_kind(instance.def_id()) != DefKind::AssocFn
        || !super::is_fully_monomorphized(tcx, instance)
        || !tcx.is_mir_available(instance.def_id())
        || instance.args.iter().any(|argument| argument.as_type().is_none())
    {
        return None;
    }
    let core = instance.def_id().krate;
    let impl_id = tcx.impl_of_assoc(instance.def_id())?;
    if impl_id.krate != core || tcx.impl_is_of_trait(impl_id) {
        return None;
    }
    let receiver = tcx.try_normalize_erasing_regions(
        TypingEnv::fully_monomorphized(),
        tcx.type_of(impl_id).instantiate(tcx, instance.args),
    ).ok()?;
    let signature = tcx.instantiate_bound_regions_with_erased(
        tcx.fn_sig(instance.def_id()).instantiate(tcx, instance.args),
    );
    let signature = tcx.try_normalize_erasing_regions(
        TypingEnv::fully_monomorphized(), signature,
    ).ok()?;
    if signature.safety != Safety::Safe || signature.abi != ExternAbi::Rust || signature.c_variadic {
        return None;
    }
    let [input, other] = signature.inputs() else {
        return None;
    };
    if *input != receiver {
        return None;
    }
    let method = tcx.item_name(instance.def_id());
    let method = method.as_str();
    if receiver == tcx.types.usize || receiver == tcx.types.u32 {
        if !instance.args.is_empty() || *other != receiver {
            return None;
        }
        if receiver == tcx.types.u32 && method == "overflowing_mul"
            && matches!(signature.output().kind(), TyKind::Tuple(fields)
                if fields.len() == 2 && fields[0] == tcx.types.u32 && fields[1] == tcx.types.bool)
        {
            return Some(Family::Integer);
        }
        if method == "is_multiple_of" && signature.output() == tcx.types.bool {
            return Some(Family::Integer);
        }
        if matches!(method, "checked_add" | "checked_sub" | "checked_mul" | "checked_div" | "checked_rem")
            && option_argument(tcx, signature.output(), core) == Some(receiver)
        {
            return Some(if receiver == tcx.types.usize && method == "checked_add" {
                Family::UsizeCheckedAdd
            } else {
                Family::Integer
            });
        }
        return None;
    }
    let payload = option_argument(tcx, receiver, core)?;
    match method {
        "ok_or" if instance.args.len() == 2
            && instance.args[0].as_type() == Some(payload)
            && instance.args[1].as_type() == Some(*other)
            && result_arguments(tcx, signature.output(), core) == Some((payload, *other)) => {
            Some(Family::OptionOkOr)
        }
        "and_then" if instance.args.len() == 3
            && instance.args[0].as_type() == Some(payload)
            && instance.args[2].as_type() == Some(*other)
            && option_argument(tcx, signature.output(), core) == instance.args[1].as_type() => {
            Some(Family::OptionAndThen)
        }
        _ => None,
    }
}

fn nominal(ty: Ty<'_>) -> Option<rustc_hir::def_id::DefId> {
    match ty.kind() {
        TyKind::Adt(definition, _) => Some(definition.did()),
        _ => None,
    }
}

fn render_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    output: &mut BoundedText,
) -> Result<bool, fmt::Error> {
    writeln!(
        output,
        "instance={instance:?}\ndefinition={:?}\nstructural_path={}\narguments={:?}",
        instance.def_id(), tcx.def_path(instance.def_id()).to_string_no_crate_verbose(), instance.args,
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
    writeln!(output, "body arg_count={} locals={} blocks={} scopes={}",
        body.arg_count, body.local_decls.len(), body.basic_blocks.len(), body.source_scopes.len())?;
    if body.local_decls.len() > ITEM_CAP || body.basic_blocks.len() > ITEM_CAP
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
            tcx, TypingEnv::fully_monomorphized(), EarlyBinder::bind(local.ty),
        );
        complete &= normalized.is_ok();
        let nominal_type = normalized.as_ref().ok().copied().and_then(nominal);
        writeln!(output, "local[{index:?}] raw={:?} normalized={normalized:?} nominal={nominal_type:?} scope={:?}",
            local.ty, local.source_info.scope)?;
    }
    for (index, scope) in body.source_scopes.iter_enumerated() {
        let normalized = scope.inlined.map(|(raw, _)| {
            instance.try_instantiate_mir_and_normalize_erasing_regions(
                tcx, TypingEnv::fully_monomorphized(), EarlyBinder::bind(raw),
            )
        });
        complete &= normalized.as_ref().is_none_or(|result| result.is_ok());
        writeln!(output, "scope[{index:?}] data={scope:?} normalized_inline={normalized:?}")?;
    }
    for (index, block) in body.basic_blocks.iter_enumerated() {
        writeln!(output, "block[{index:?}] cleanup={} statements={}", block.is_cleanup, block.statements.len())?;
        for (statement_index, statement) in block.statements.iter().enumerate() {
            writeln!(output, "  statement[{statement_index}] scope={:?} kind={:?}",
                statement.source_info.scope, statement.kind)?;
        }
        if let Some(terminator) = &block.terminator {
            writeln!(output, "  terminator scope={:?} kind={:?}", terminator.source_info.scope, terminator.kind)?;
        } else {
            complete = false;
            writeln!(output, "  terminator missing")?;
        }
    }
    Ok(complete)
}

fn render_cohort<'tcx>(
    tcx: TyCtxt<'tcx>,
    rejected: Instance<'tcx>,
    root_name: &str,
    same_root: impl Iterator<Item = Instance<'tcx>>,
    output: &mut BoundedText,
) -> Result<bool, fmt::Error> {
    let mut retained = vec![(rejected, Family::UsizeCheckedAdd)];
    output.counts.matched = 1;
    output.counts.scan_complete = true;
    for (index, instance) in same_root.take(SCAN_CAP + 1).enumerate() {
        if index == SCAN_CAP {
            output.counts.scan_complete = false;
            break;
        }
        output.counts.scanned += 1;
        if instance == rejected {
            continue;
        }
        let Some(family) = selected(tcx, instance) else {
            continue;
        };
        output.counts.matched += 1;
        if retained.len() < BODY_CAP {
            retained.push((instance, family));
        }
    }
    output.counts.retained = retained.len();
    writeln!(output, "FE2O3_CORE_CHECKED_ATTENTION_MIR_BEGIN version=1 authority=none byte_cap={BYTE_CAP}")?;
    writeln!(output, "root={root_name:?} rejected_first=true scan_cap={SCAN_CAP} body_cap={BODY_CAP}")?;
    for (index, (instance, family)) in retained.into_iter().enumerate() {
        // An attempted body remains partial if any write reaches the shared cap.
        output.counts.partial += 1;
        writeln!(output, "BODY_BEGIN index={index} family={family:?} rejected={}", index == 0)?;
        let full = render_body(tcx, instance, output)?;
        writeln!(output, "BODY_END index={index} complete={full}")?;
        if full {
            output.counts.partial -= 1;
            output.counts.full += 1;
        }
    }
    Ok(output.counts.scan_complete && output.counts.partial == 0
        && output.counts.full == output.counts.matched)
}

pub(super) fn emit_core_checked_attention_diagnostic_v1<'tcx>(
    tcx: TyCtxt<'tcx>,
    rejected: Instance<'tcx>,
    root_name: &str,
    same_root: impl Iterator<Item = Instance<'tcx>>,
) {
    if std::env::var_os("FE2O3_DIAG_CORE_CHECKED_ATTENTION_MIR_V1").as_deref()
        != Some(std::ffi::OsStr::new("1"))
        || selected(tcx, rejected) != Some(Family::UsizeCheckedAdd)
        || EMITTED.swap(true, Ordering::Relaxed)
    {
        return;
    }
    let text = diagnostic_text(|output| render_cohort(tcx, rejected, root_name, same_root, output));
    // A diagnostic write failure cannot replace the original refusal.
    let _ = io::stderr().lock().write_all(text.as_bytes());
}

#[cfg(test)]
#[path = "core_checked_attention_diagnostic_v1_tests.rs"]
mod tests;
