use super::*;
use rustc_driver::{Callbacks, Compilation};
use rustc_hir::def::DefKind;
use rustc_interface::interface::Compiler;
use rustc_middle::ty::{GenericArgs, Ty};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const SOURCE: &str = r#"
#![allow(dead_code, unused_unsafe)]
#[inline(always)]
fn invoke<F: FnMut(u32) -> u32>(mut callback: F, value: u32) -> u32 {
    callback(value)
}
#[inline(never)]
fn safe_target(value: u32) -> u32 { value + 1 }
#[inline(never)]
fn unsafe_body_target(value: u32) -> u32 { unsafe { value + 2 } }
#[inline(never)]
fn unrelated_target(value: u32) -> u32 { value + 3 }
#[inline(never)]
fn generic_target<T>(value: T) -> T { value }
fn call_mut(value: u32) -> u32 { value }
pub fn safe_root(value: u32) -> u32 { invoke(safe_target, value) }
pub fn unsafe_root(value: u32) -> u32 { invoke(unsafe_body_target, value) }
pub fn generic_root(value: u32) -> u32 { invoke(generic_target::<u32>, value) }
pub fn dynamic_root(callback: fn(u32) -> u32, value: u32) -> u32 {
    invoke(callback, value)
}
pub fn closure_root(value: u32) -> u32 { invoke(|input| input + 4, value) }
#[inline(always)]
fn fully_inline_target(value: u32) -> u32 { value + 5 }
#[inline(always)]
fn fully_inline_unsafe_target(value: u32) -> u32 { unsafe { value + 6 } }
pub fn fully_inline_root(value: u32) -> u32 { invoke(fully_inline_target, value) }
pub fn fully_inline_unsafe_root(value: u32) -> u32 { invoke(fully_inline_unsafe_target, value) }
pub fn fully_inline_twice_root(value: u32) -> u32 {
    invoke(fully_inline_target, value) + invoke(fully_inline_target, value + 9)
}
#[inline(always)]
fn invoke_generic<T: Copy, F: FnMut(T) -> T>(mut callback: F, value: T) -> T { callback(value) }
#[inline(always)]
fn fully_inline_generic_target<T: Copy>(value: T) -> T { value }
pub fn fully_inline_generic_root<T: Copy>(value: T) -> T {
    invoke_generic(fully_inline_generic_target::<T>, value)
}
"#;

#[derive(Clone)]
struct Results {
    inlined: InlinedResults,
    safe_audit: Result<(), String>,
    unsafe_error: String,
    missing_target_rejected: bool,
    unrelated_target_rejected: bool,
    wrong_instantiation_rejected: bool,
    malformed: Vec<(&'static str, bool)>,
    dynamic_rejected: bool,
    other_method_rejected: bool,
    closure_shim_rejected: bool,
    closure_audit: Result<(), String>,
}

#[derive(Default)]
struct Capture {
    results: Option<Results>,
}

fn local_function(tcx: TyCtxt<'_>, name: &str) -> DefId {
    tcx.iter_local_def_id()
        .find(|id| {
            tcx.def_kind(id.to_def_id()) == DefKind::Fn
                && tcx.item_name(id.to_def_id()).as_str() == name
        })
        .unwrap_or_else(|| panic!("missing fixture function {name}"))
        .to_def_id()
}

fn origins<'tcx>(tcx: TyCtxt<'tcx>, owner: Instance<'tcx>) -> Vec<Instance<'tcx>> {
    tcx.instance_mir(owner.def)
        .source_scopes
        .iter()
        .filter_map(|scope| scope.inlined.map(|(origin, _)| origin))
        .map(|origin| {
            owner
                .try_instantiate_mir_and_normalize_erasing_regions(
                    tcx,
                    TypingEnv::fully_monomorphized(),
                    EarlyBinder::bind(origin),
                )
                .expect("normalize actual optimized source scope")
        })
        .collect()
}

fn row<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>, root: bool) -> CollectedFunction<'tcx> {
    CollectedFunction {
        instance,
        role: if root {
            CollectedFunctionRole::KernelEntry
        } else {
            CollectedFunctionRole::InternalHelper
        },
        export_name: tcx.def_path_str(instance.def_id()),
        logical_name: root.then(|| tcx.item_name(instance.def_id()).to_string()),
        generated_host_contract_identity: None,
        kernel_binding: None,
        frontend_contract: None,
        reference_effect_binding: None,
        dead_branches: None,
        closure_observation: None,
        kernel_context_contract: None,
    }
}

// This is an audit unit graph derived from real optimized direct calls, not a
// sealed production owner or proof that the actual GPU worker is accepted.
fn audit_collector<'tcx>(tcx: TyCtxt<'tcx>, root: Instance<'tcx>) -> DeviceCollector<'tcx> {
    let contexts = capture_context_producers_v1(tcx).expect("empty context registrations");
    let mut collector = DeviceCollector::new(tcx, false, Vec::new(), String::new(), contexts);
    collector.result.push(row(tcx, root, true));
    let body = tcx.instance_mir(root.def);
    for block in body.basic_blocks.iter() {
        let TerminatorKind::Call { func, .. } = &block.terminator().kind else {
            continue;
        };
        let callee = kernel_context_auth_v1::resolve_call(tcx, root, body, func)
            .expect("fixture has only exact direct calls");
        assert!(matches!(callee.def, InstanceKind::Item(_)));
        if !collector
            .result
            .iter()
            .any(|entry| entry.instance == callee)
        {
            collector.result.push(row(tcx, callee, false));
        }
        let caller_identity = collector.instance_identity(root);
        let callee_identity = collector.instance_identity(callee);
        collector
            .call_edges
            .entry(caller_identity)
            .or_default()
            .insert(callee_identity);
    }
    collector
}

impl Callbacks for Capture {
    fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        let safe = Instance::mono(tcx, local_function(tcx, "safe_root"));
        let target = Instance::mono(tcx, local_function(tcx, "safe_target"));
        let original = audit_collector(tcx, safe);
        assert_eq!(
            original.result.len(),
            2,
            "root plus real direct-call target"
        );
        assert_eq!(original.result[1].instance, target);
        let observed = origins(tcx, safe);
        let shims = observed.iter().copied().filter(|origin| {
            matches!(origin.def, InstanceKind::FnPtrShim(_, ty) if matches!(ty.kind(), TyKind::FnDef(..)))
        }).collect::<Vec<_>>();
        assert_eq!(
            shims.len(),
            1,
            "actual optimized FnDef source-scope shim: {observed:?}"
        );
        let shim = shims[0];
        let InstanceKind::FnPtrShim(method, self_ty) = shim.def else {
            unreachable!()
        };
        assert_eq!(tcx.parent(method), tcx.lang_items().fn_mut_trait().unwrap());
        assert_eq!(tcx.item_name(method).as_str(), "call_mut");
        assert_eq!(
            static_fndef_source_origin_v1(tcx, shim, &original.result),
            Some(target),
            "exact observed shim must map to the original executable target",
        );
        let safe_audit = original
            .authenticate_production_kernel_source_safety()
            .map_err(|e| e.to_string());
        let missing_target_rejected =
            static_fndef_source_origin_v1(tcx, shim, &original.result[..1]).is_none();
        let unrelated = row(
            tcx,
            Instance::mono(tcx, local_function(tcx, "unrelated_target")),
            false,
        );
        let unrelated_target_rejected =
            static_fndef_source_origin_v1(tcx, shim, &[original.result[0].clone(), unrelated])
                .is_none();

        let other_ty = Ty::new_fn_def(tcx, local_function(tcx, "unrelated_target"), target.args);
        let generic = local_function(tcx, "generic_target");
        let generic_root = Instance::mono(tcx, local_function(tcx, "generic_root"));
        let generic_collector = audit_collector(tcx, generic_root);
        let generic_shim = origins(tcx, generic_root).into_iter().find(|origin| {
            matches!(origin.def, InstanceKind::FnPtrShim(_, ty) if matches!(ty.kind(), TyKind::FnDef(def, _) if *def == generic))
        }).expect("actual generic function-item shim");
        assert_eq!(generic_collector.result.len(), 2);
        let generic_target = generic_collector.result[1].instance;
        assert_eq!(generic_target.def_id(), generic);
        assert_eq!(
            static_fndef_source_origin_v1(tcx, generic_shim, &generic_collector.result),
            Some(generic_target)
        );
        let other_instantiation = Instance::try_resolve(
            tcx,
            TypingEnv::fully_monomorphized(),
            generic,
            tcx.mk_args(&[tcx.types.u64.into()]),
        )
        .expect("resolve other generic instantiation")
        .expect("concrete other instantiation");
        assert_eq!(generic_target.def_id(), other_instantiation.def_id());
        assert_ne!(generic_target.args, other_instantiation.args);
        let wrong_instantiation_rejected = static_fndef_source_origin_v1(
            tcx,
            generic_shim,
            &[
                generic_collector.result[0].clone(),
                row(tcx, other_instantiation, false),
            ],
        )
        .is_none();
        let generic_ty = Ty::new_fn_def(tcx, generic, GenericArgs::identity_for_item(tcx, generic));
        let mutations = [
            (
                "empty arguments",
                Instance {
                    args: target.args,
                    ..shim
                },
            ),
            (
                "non-type Self",
                Instance {
                    args: tcx.mk_args(&[tcx.lifetimes.re_erased.into(), shim.args[1]]),
                    ..shim
                },
            ),
            (
                "non-type Args",
                Instance {
                    args: tcx.mk_args(&[self_ty.into(), tcx.lifetimes.re_erased.into()]),
                    ..shim
                },
            ),
            (
                "non-tuple argument",
                Instance {
                    args: tcx.mk_args(&[self_ty.into(), tcx.types.u32.into()]),
                    ..shim
                },
            ),
            (
                "wrong tuple arity",
                Instance {
                    args: tcx.mk_args(&[self_ty.into(), tcx.types.unit.into()]),
                    ..shim
                },
            ),
            (
                "wrong tuple element",
                Instance {
                    args: tcx.mk_args(&[self_ty.into(), Ty::new_tup(tcx, &[tcx.types.u64]).into()]),
                    ..shim
                },
            ),
            (
                "mismatched Self",
                Instance {
                    def: InstanceKind::FnPtrShim(method, other_ty),
                    ..shim
                },
            ),
            (
                "nonmonomorphic Self",
                Instance {
                    def: InstanceKind::FnPtrShim(method, generic_ty),
                    args: tcx.mk_args(&[generic_ty.into(), shim.args[1]]),
                },
            ),
            (
                "same-name ordinary method",
                Instance {
                    def: InstanceKind::FnPtrShim(local_function(tcx, "call_mut"), self_ty),
                    ..shim
                },
            ),
            ("ordinary Item is not a shim", target),
        ];
        let malformed = mutations
            .into_iter()
            .map(|(name, instance)| {
                (
                    name,
                    static_fndef_source_origin_v1(tcx, instance, &original.result).is_none(),
                )
            })
            .collect();

        let dynamic = Instance::mono(tcx, local_function(tcx, "dynamic_root"));
        let dynamic_origins = origins(tcx, dynamic);
        let dynamic_shims = dynamic_origins.iter().copied().filter(|origin| {
            matches!(origin.def, InstanceKind::FnPtrShim(_, ty) if matches!(ty.kind(), TyKind::FnPtr(..)))
        }).collect::<Vec<_>>();
        assert_eq!(
            dynamic_shims.len(),
            1,
            "actual runtime-pointer shim: {dynamic_origins:?}"
        );
        let dynamic_rejected =
            static_fndef_source_origin_v1(tcx, dynamic_shims[0], &original.result).is_none();
        let fn_trait = tcx.lang_items().fn_trait().unwrap();
        let fn_call = tcx
            .associated_items(fn_trait)
            .in_definition_order()
            .find(|item| tcx.item_name(item.def_id).as_str() == "call")
            .expect("Fn::call lang-item method")
            .def_id;
        let other_method =
            Instance::try_resolve(tcx, TypingEnv::fully_monomorphized(), fn_call, shim.args)
                .expect("resolve genuine Fn call")
                .expect("concrete Fn call");
        assert!(matches!(other_method.def, InstanceKind::FnPtrShim(..)));
        let other_method_rejected =
            static_fndef_source_origin_v1(tcx, other_method, &original.result).is_none();

        let unsafe_root = Instance::mono(tcx, local_function(tcx, "unsafe_root"));
        let unsafe_target = Instance::mono(tcx, local_function(tcx, "unsafe_body_target"));
        let unsafe_collector = audit_collector(tcx, unsafe_root);
        assert!(
            unsafe_collector
                .result
                .iter()
                .any(|entry| entry.instance == unsafe_target)
        );
        assert!(
            origins(tcx, unsafe_root).into_iter().any(|origin| {
                static_fndef_source_origin_v1(tcx, origin, &unsafe_collector.result)
                    == Some(unsafe_target)
            }),
            "unsafe-body fixture must reach the new normalized shim route"
        );
        let unsafe_error = unsafe_collector
            .authenticate_production_kernel_source_safety()
            .expect_err("safe signature does not exempt an unsafe body")
            .to_string();

        let closure_root = Instance::mono(tcx, local_function(tcx, "closure_root"));
        let closure_item = origins(tcx, closure_root)
            .into_iter()
            .find(|origin| {
                matches!(origin.def, InstanceKind::Item(_))
                    && tcx.def_kind(origin.def_id()) == DefKind::Closure
            })
            .expect("actual local closure Item origin remains represented");
        let closure_ty = Ty::new_closure(tcx, closure_item.def_id(), closure_item.args);
        let once_trait = tcx.lang_items().fn_once_trait().unwrap();
        let once_method = tcx
            .associated_items(once_trait)
            .in_definition_order()
            .find(|item| tcx.item_name(item.def_id).as_str() == "call_once")
            .expect("FnOnce::call_once lang-item method")
            .def_id;
        let once_shim = Instance::try_resolve(
            tcx,
            TypingEnv::fully_monomorphized(),
            once_method,
            tcx.mk_args(&[closure_ty.into(), shim.args[1]]),
        )
        .expect("resolve actual closure FnOnce adapter")
        .expect("concrete closure adapter");
        assert!(matches!(
            once_shim.def,
            InstanceKind::ClosureOnceShim { .. }
        ));
        let closure_shim_rejected =
            static_fndef_source_origin_v1(tcx, once_shim, &original.result).is_none();
        let closure_audit = audit_collector(tcx, closure_root)
            .authenticate_production_kernel_source_safety()
            .map_err(|e| e.to_string());
        self.results = Some(Results {
            inlined: inlined_results(tcx),
            safe_audit,
            unsafe_error,
            missing_target_rejected,
            unrelated_target_rejected,
            wrong_instantiation_rejected,
            malformed,
            dynamic_rejected,
            other_method_rejected,
            closure_shim_rejected,
            closure_audit,
        });
        Compilation::Stop
    }
}

#[derive(Clone)]
struct InlinedResults {
    safe_audit: Result<(), String>,
    unsafe_error: String,
    generic_audit: Result<(), String>,
    rejected: Vec<(&'static str, bool)>,
    exact_budget: bool,
    shared_budget: bool,
}

fn inlined_results<'tcx>(tcx: TyCtxt<'tcx>) -> InlinedResults {
    let owner = Instance::mono(tcx, local_function(tcx, "fully_inline_root"));
    let target = Instance::mono(tcx, local_function(tcx, "fully_inline_target"));
    let collector = audit_collector(tcx, owner);
    assert_eq!(collector.result.len(), 1, "target must be fully inlined");
    assert!(collector.call_edges.values().all(BTreeSet::is_empty));
    let body = tcx.instance_mir(owner.def);
    let normalize = |owner: Instance<'tcx>, origin| {
        owner
            .try_instantiate_mir_and_normalize_erasing_regions(
                tcx,
                TypingEnv::fully_monomorphized(),
                EarlyBinder::bind(origin),
            )
            .expect("normalize real fixture scope")
    };
    let shims = body
        .source_scopes
        .iter_enumerated()
        .filter_map(|(index, scope)| {
            let origin = normalize(owner, scope.inlined?.0);
            matches!(origin.def, InstanceKind::FnPtrShim(..)).then_some((index, origin))
        })
        .collect::<Vec<_>>();
    assert_eq!(shims.len(), 1, "one actual fully inlined FnDef shim");
    let (shim_scope, shim) = shims[0];
    assert_eq!(
        static_fndef_source_origin_v1(tcx, shim, &collector.result),
        None
    );
    let targets = body
        .source_scopes
        .iter_enumerated()
        .filter_map(|(index, scope)| {
            (normalize(owner, scope.inlined?.0) == target).then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 1, "one actual target Item scope");
    let target_scope = targets[0];
    let mut budget = fe2o3_rustc_front::MAX_TOTAL_BLOCKS_V1;
    let allowed =
        inlined_static_fndef_source_origin_v1(tcx, owner, body, shim_scope, shim, &mut budget);
    assert_eq!(allowed, Some(target));
    let cost = fe2o3_rustc_front::MAX_TOTAL_BLOCKS_V1 - budget;
    assert!(cost > 2);
    let mut exact = cost;
    let exact_budget =
        inlined_static_fndef_source_origin_v1(tcx, owner, body, shim_scope, shim, &mut exact)
            == Some(target)
            && exact == 0;
    let shared_budget =
        inlined_static_fndef_source_origin_v1(tcx, owner, body, shim_scope, shim, &mut exact)
            .is_none()
            && exact == 0;
    let mut rejected = Vec::new();
    rejected.push((
        "every insufficient scan/walk budget",
        (0..cost).all(|limit| {
            let mut remaining = limit;
            inlined_static_fndef_source_origin_v1(
                tcx,
                owner,
                body,
                shim_scope,
                shim,
                &mut remaining,
            )
            .is_none()
        }),
    ));
    let refuses = |candidate: &Body<'tcx>, candidate_owner, scope, origin| {
        let mut remaining = fe2o3_rustc_front::MAX_TOTAL_BLOCKS_V1;
        inlined_static_fndef_source_origin_v1(
            tcx,
            candidate_owner,
            candidate,
            scope,
            origin,
            &mut remaining,
        )
        .is_none()
    };
    let absent_index = SourceScope::from_usize(body.source_scopes.len());
    rejected.push((
        "out-of-range shim",
        refuses(body, owner, absent_index, shim),
    ));
    rejected.push((
        "Item scope is not shim",
        refuses(body, owner, target_scope, shim),
    ));
    rejected.push((
        "normalized shim mismatch",
        refuses(body, owner, shim_scope, target),
    ));
    let other_owner = Instance::mono(tcx, local_function(tcx, "safe_root"));
    rejected.push((
        "wrong body owner",
        refuses(body, other_owner, shim_scope, shim),
    ));
    let mut missing = body.clone();
    missing.source_scopes[target_scope].inlined = None;
    rejected.push((
        "missing Item origin",
        refuses(&missing, owner, shim_scope, shim),
    ));

    for (name, parent) in [
        ("missing lexical parent", None),
        ("out-of-range lexical parent", Some(absent_index)),
        ("self-cycle lexical parent", Some(target_scope)),
    ] {
        let mut changed = body.clone();
        changed.source_scopes[target_scope].parent_scope = parent;
        rejected.push((name, refuses(&changed, owner, shim_scope, shim)));
    }
    for (name, parent) in [
        ("missing cached parent", None),
        ("out-of-range cached parent", Some(absent_index)),
        ("self-cycle cached parent", Some(target_scope)),
    ] {
        let mut changed = body.clone();
        changed.source_scopes[target_scope].inlined_parent_scope = parent;
        rejected.push((name, refuses(&changed, owner, shim_scope, shim)));
    }
    let mut cycle = body.clone();
    cycle.source_scopes[target_scope].parent_scope = Some(shim_scope);
    cycle.source_scopes[shim_scope].parent_scope = Some(target_scope);
    rejected.push((
        "two-node lexical cycle",
        refuses(&cycle, owner, shim_scope, shim),
    ));
    let mut cached_cycle = body.clone();
    let mut detached = body.clone();
    detached.source_scopes[shim_scope].parent_scope = None;
    detached.source_scopes[shim_scope].inlined_parent_scope = None;
    rejected.push((
        "detached shim with matching lexical and cached chains",
        refuses(&detached, owner, shim_scope, shim),
    ));
    cached_cycle.source_scopes[shim_scope].inlined_parent_scope = Some(target_scope);
    rejected.push((
        "cached cycle above shim",
        refuses(&cached_cycle, owner, shim_scope, shim),
    ));
    let mut cached_invalid = body.clone();
    cached_invalid.source_scopes[shim_scope].inlined_parent_scope = Some(absent_index);
    rejected.push((
        "cached invalid ancestor",
        refuses(&cached_invalid, owner, shim_scope, shim),
    ));

    let sibling_owner = Instance::mono(tcx, local_function(tcx, "fully_inline_twice_root"));
    let sibling_body = tcx.instance_mir(sibling_owner.def);
    let sibling_shims = sibling_body
        .source_scopes
        .iter_enumerated()
        .filter_map(|(index, scope)| {
            let origin = normalize(sibling_owner, scope.inlined?.0);
            matches!(origin.def, InstanceKind::FnPtrShim(..)).then_some((index, origin))
        })
        .collect::<Vec<_>>();
    assert_eq!(sibling_shims.len(), 2);
    let (first_scope, first_shim) = sibling_shims[0];
    let (second_scope, _) = sibling_shims[1];
    let mut moved = sibling_body.clone();
    let mut moved_count = 0;
    for scope in moved.source_scopes.iter_mut() {
        if scope
            .inlined
            .is_some_and(|(origin, _)| normalize(sibling_owner, origin) == target)
            && scope.inlined_parent_scope == Some(first_scope)
        {
            scope.parent_scope = Some(second_scope);
            scope.inlined_parent_scope = Some(second_scope);
            moved_count += 1;
        }
    }
    assert_eq!(moved_count, 1);
    rejected.push((
        "same target only under sibling shim",
        refuses(&moved, sibling_owner, first_scope, first_shim),
    ));

    let generic_def = local_function(tcx, "fully_inline_generic_root");
    let generic_owner = Instance::try_resolve(
        tcx,
        TypingEnv::fully_monomorphized(),
        generic_def,
        tcx.mk_args(&[tcx.types.u32.into()]),
    )
    .unwrap()
    .unwrap();
    let generic_collector = audit_collector(tcx, generic_owner);
    assert_eq!(generic_collector.result.len(), 1);
    let generic_body = tcx.instance_mir(generic_owner.def);
    let (generic_scope, generic_shim) = generic_body
        .source_scopes
        .iter_enumerated()
        .find_map(|(index, scope)| {
            let origin = normalize(generic_owner, scope.inlined?.0);
            matches!(origin.def, InstanceKind::FnPtrShim(..)).then_some((index, origin))
        })
        .expect("generic owner contains actual normalized FnDef shim");
    let generic_target = static_fndef_target_v1(tcx, generic_shim).unwrap();
    let mut generic_budget = fe2o3_rustc_front::MAX_TOTAL_BLOCKS_V1;
    assert_eq!(
        inlined_static_fndef_source_origin_v1(
            tcx,
            generic_owner,
            generic_body,
            generic_scope,
            generic_shim,
            &mut generic_budget,
        ),
        Some(generic_target)
    );
    let other_instantiation = Instance::try_resolve(
        tcx,
        TypingEnv::fully_monomorphized(),
        generic_target.def_id(),
        tcx.mk_args(&[tcx.types.u64.into()]),
    )
    .unwrap()
    .unwrap();
    let mut wrong_generic = generic_body.clone();
    let mut substitutions = 0;
    for scope in wrong_generic.source_scopes.iter_mut() {
        if let Some((origin, callsite)) = scope.inlined
            && normalize(generic_owner, origin) == generic_target
        {
            scope.inlined = Some((other_instantiation, callsite));
            substitutions += 1;
        }
    }
    assert_eq!(substitutions, 1);
    rejected.push((
        "same DefId wrong target instantiation",
        refuses(&wrong_generic, generic_owner, generic_scope, generic_shim),
    ));
    let other_generic_owner = Instance::try_resolve(
        tcx,
        TypingEnv::fully_monomorphized(),
        generic_def,
        tcx.mk_args(&[tcx.types.u64.into()]),
    )
    .unwrap()
    .unwrap();
    rejected.push((
        "wrong owner instantiation",
        refuses(
            generic_body,
            other_generic_owner,
            generic_scope,
            generic_shim,
        ),
    ));

    let before_instances = collector
        .result
        .iter()
        .map(|r| r.instance)
        .collect::<Vec<_>>();
    let before_edges = collector.call_edges.clone();
    let safe_audit = collector
        .authenticate_production_kernel_source_safety()
        .map_err(|e| e.to_string());
    assert_eq!(
        collector
            .result
            .iter()
            .map(|r| r.instance)
            .collect::<Vec<_>>(),
        before_instances
    );
    assert_eq!(collector.call_edges, before_edges);
    let generic_audit = generic_collector
        .authenticate_production_kernel_source_safety()
        .map_err(|e| e.to_string());
    let unsafe_owner = Instance::mono(tcx, local_function(tcx, "fully_inline_unsafe_root"));
    let unsafe_collector = audit_collector(tcx, unsafe_owner);
    assert_eq!(
        unsafe_collector.result.len(),
        1,
        "unsafe target also fully inlined"
    );
    let unsafe_error = unsafe_collector
        .authenticate_production_kernel_source_safety()
        .expect_err("inlining cannot exempt unsafe HIR")
        .to_string();
    InlinedResults {
        safe_audit,
        unsafe_error,
        generic_audit,
        rejected,
        exact_budget,
        shared_budget,
    }
}

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn results() -> Results {
    static RESULTS: OnceLock<Results> = OnceLock::new();
    RESULTS
        .get_or_init(|| {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let fixture = Fixture(std::env::temp_dir().join(format!(
                "fe2o3-static-origin-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )));
            fs::create_dir(&fixture.0).expect("create optimized source fixture");
            let source = fixture.0.join("fixture.rs");
            fs::write(&source, SOURCE).expect("write optimized source fixture");
            let mut command = Command::new("rustc");
            command.args(["--print", "sysroot"]);
            let sysroot =
                crate::process_execution::capture_output(&mut command).expect("rustc sysroot");
            assert!(sysroot.status.success());
            let args = vec![
                "rustc".to_owned(),
                "--crate-name".to_owned(),
                "fe2o3_static_origin_fixture".to_owned(),
                "--crate-type=lib".to_owned(),
                "--edition=2024".to_owned(),
                "--emit=metadata".to_owned(),
                "-Copt-level=3".to_owned(),
                "-Zinline-mir=yes".to_owned(),
                "-Zmir-enable-passes=-JumpThreading".to_owned(),
                "-Coverflow-checks=off".to_owned(),
                "-Cpanic=abort".to_owned(),
                "--sysroot".to_owned(),
                String::from_utf8(sysroot.stdout)
                    .expect("UTF-8 sysroot")
                    .trim()
                    .to_owned(),
                "-o".to_owned(),
                fixture.0.join("fixture.rmeta").display().to_string(),
                source.display().to_string(),
            ];
            static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
            let _guard = LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut callbacks = Capture::default();
            rustc_driver::run_compiler(&args, &mut callbacks);
            callbacks.results.expect("optimized callback ran")
        })
        .clone()
}

#[test]
fn static_fndef_origin_observed_optimized_shim_audits_original_target() {
    assert_eq!(results().safe_audit, Ok(()));
}

#[test]
fn fully_inlined_fndef_origin_audits_real_descendant_without_executable_node() {
    assert_eq!(results().inlined.safe_audit, Ok(()));
}

#[test]
fn fully_inlined_fndef_origin_preserves_unsafe_body_rejection() {
    let error = results().inlined.unsafe_error;
    for expected in [
        "ordinary production kernel `fully_inline_unsafe_root`",
        "safe-signature local helper containing a user-provided unsafe block",
        "reachable call chain:",
        "fully_inline_unsafe_target",
    ] {
        assert!(error.contains(expected), "missing {expected:?}: {error}");
    }
    assert!(!error.contains("not a traversable monomorphic function instance"));
}

#[test]
fn fully_inlined_fndef_origin_normalizes_generic_owner_and_target() {
    assert_eq!(results().inlined.generic_audit, Ok(()));
}

#[test]
fn fully_inlined_fndef_origin_rejects_wrong_scope_identity_and_ancestry() {
    let rejected = results().inlined.rejected;
    assert_eq!(rejected.len(), 19);
    for (name, refused) in rejected {
        assert!(refused, "accepted {name}");
    }
}

#[test]
fn fully_inlined_fndef_origin_has_exact_and_shared_work_bounds() {
    let inlined = results().inlined;
    assert!(inlined.exact_budget);
    assert!(inlined.shared_budget);
}

#[test]
fn static_fndef_origin_unsafe_callback_body_keeps_rooted_rejection() {
    let error = results().unsafe_error;
    for expected in [
        "ordinary production kernel `unsafe_root`",
        "safe-signature local helper containing a user-provided unsafe block",
        "reachable call chain:",
        "unsafe_root",
        "unsafe_body_target",
        "fixture.rs:",
    ] {
        assert!(error.contains(expected), "missing {expected:?}: {error}");
    }
    assert!(!error.contains("not a traversable monomorphic function instance"));
}

#[test]
fn static_fndef_origin_requires_exact_original_roster_target() {
    let results = results();
    assert!(results.missing_target_rejected);
    assert!(results.unrelated_target_rejected);
    assert!(results.wrong_instantiation_rejected);
}

#[test]
fn static_fndef_origin_rejects_malformed_identity_and_arguments() {
    let malformed = results().malformed;
    assert_eq!(malformed.len(), 10);
    for (name, rejected) in malformed {
        assert!(rejected, "accepted {name}");
    }
}

#[test]
fn static_fndef_origin_rejects_dynamic_and_other_call_trait_shims() {
    let results = results();
    assert!(results.dynamic_rejected);
    assert!(results.other_method_rejected);
    assert!(results.closure_shim_rejected);
}

#[test]
fn static_fndef_origin_preserves_ordinary_item_closure_audit() {
    assert_eq!(results().closure_audit, Ok(()));
}

#[test]
fn static_fndef_origin_rooted_cycle_visits_each_origin_once() {
    let edges = BTreeMap::from([
        (0, BTreeSet::from([2])),
        (1, BTreeSet::from([2])),
        (2, BTreeSet::from([3])),
        (3, BTreeSet::from([2])),
    ]);
    let labels = BTreeMap::from([
        (0, "root_a".to_owned()),
        (1, "root_b".to_owned()),
        (2, "callback".to_owned()),
        (3, "leaf".to_owned()),
    ]);
    for root in [0, 1] {
        let (links, order) = root_scoped_call_chains(&edges, &labels, &root);
        assert_eq!(order, [root, 2, 3]);
        assert_eq!(
            reconstruct_call_chain(&links, &3),
            [
                labels[&root].clone(),
                "callback".to_owned(),
                "leaf".to_owned()
            ]
        );
    }
}
