use quote::ToTokens;
use std::cell::{Cell, RefCell};

#[macro_use]
#[path = "../src/mlp_numerics_v1.rs"]
mod original;
#[macro_use]
#[path = "../src/mlp_silu_materialized_numerics_v1.rs"]
mod candidate;

fn widen(word: u16) -> f32 {
    f32::from_bits(u32::from(word) << 16)
}

fn narrow(value: f32) -> u16 {
    assert!(value.is_finite());
    let bits = value.to_bits();
    let upper = bits >> 16;
    let lower = bits & 0xffff;
    let result = upper + u32::from(lower > 0x8000 || (lower == 0x8000 && upper & 1 != 0));
    assert_ne!(result & 0x7f80, 0x7f80);
    result as u16
}

fn staged(gate: u16, up: u16, exponential: f32) -> (f32, u16, u16) {
    let g = widen(gate);
    let numerator = if g >= 0.0 { 1.0_f32 } else { exponential };
    let sigmoid = numerator / (1.0_f32 + exponential);
    let silu = g * sigmoid;
    let materialized = narrow(silu);
    (silu, materialized, narrow(widen(materialized) * widen(up)))
}

// API-shaped CPU stand-ins exercise the actual macro, not GPU exp, collectives
// or provider admission. Explicit exp return bits isolate the rounding change.
struct Math {
    exponential: f32,
    inputs: RefCell<Vec<u32>>,
}
impl Math {
    fn new(exponential: f32) -> Self {
        Self { exponential, inputs: RefCell::new(Vec::new()) }
    }
    fn exp_f32(&self, input: f32) -> f32 {
        self.inputs.borrow_mut().push(input.to_bits());
        self.exponential
    }
}

struct Task {
    lane: usize,
    gate: u16,
    up: u16,
    valid: bool,
    gate_calls: Cell<usize>,
    up_calls: Cell<usize>,
    reject_calls: usize,
    write_attempts: usize,
    output: Vec<u16>,
    missing_gate: bool,
    missing_up: bool,
    fail_write: Option<usize>,
}
impl Task {
    fn new(gate: u16, up: u16, lane: usize) -> Self {
        Self { lane, gate, up, valid: true, gate_calls: Cell::new(0),
            up_calls: Cell::new(0), reject_calls: 0, write_attempts: 0,
            output: Vec::new(), missing_gate: false, missing_up: false, fail_write: None }
    }
    fn lane(&self) -> usize { self.lane }
    fn gate(&self, column: usize) -> Option<u16> {
        assert_eq!(column, self.lane + 64 * self.gate_calls.get());
        assert!(column < 6144);
        self.gate_calls.set(self.gate_calls.get() + 1);
        (self.valid && !self.missing_gate).then_some(self.gate)
    }
    fn up(&self, column: usize) -> Option<u16> {
        assert_eq!(column, self.lane + 64 * self.up_calls.get());
        assert!(column < 6144);
        self.up_calls.set(self.up_calls.get() + 1);
        (self.valid && !self.missing_up).then_some(self.up)
    }
    fn reject(&mut self) {
        self.valid = false;
        self.reject_calls += 1;
    }
    fn write_component(&mut self, component: usize, value: u16) -> bool {
        assert_eq!(component, self.output.len());
        self.write_attempts += 1;
        if !self.valid || self.fail_write == Some(component) {
            return false;
        }
        self.output.push(value);
        true
    }
}

fn run(task: &mut Task, math: &Math) {
    qwen_claimed_mlp_swiglu_materialized_v1!(task, math);
}

fn census(task: &Task, math: &Math) {
    assert_eq!((task.gate_calls.get(), task.up_calls.get(), math.inputs.borrow().len()), (96, 96, 96));
}

fn rejected(gate: u16, up: u16, exponential: f32) {
    let mut task = Task::new(gate, up, 63);
    let math = Math::new(exponential);
    run(&mut task, &math);
    census(&task, &math);
    assert!(!task.valid && task.output.is_empty());
    assert_eq!(task.write_attempts, 0);
    assert_eq!(task.reject_calls, 96);
}

#[test]
fn all_lanes_keep_full_component_coverage_and_exp_arguments() {
    for lane in 0..64 {
        let gate = if lane % 2 == 0 { 0x3f81 } else { 0xbf81 };
        let up = if lane % 3 == 0 { 0xbfc0 } else { 0x3fc0 };
        let mut task = Task::new(gate, up, lane);
        let math = Math::new(f32::from_bits(0x3eaaaaab));
        run(&mut task, &math);
        census(&task, &math);
        assert!(task.valid);
        assert_eq!(task.output, vec![staged(gate, up, math.exponential).2; 96]);
        assert_eq!(task.write_attempts, 96);
        assert!(math.inputs.borrow().iter().all(|bits| *bits == (-widen(gate).abs()).to_bits()));
    }
}

#[test]
fn negative_gate_preserves_numerator_and_division_before_rounding() {
    let mut task = Task::new(0xbf81, 0x3fc0, 0);
    let math = Math::new(0.25);
    run(&mut task, &math);
    assert_eq!(task.output, vec![staged(0xbf81, 0x3fc0, 0.25).2; 96]);
    let wrong = narrow(widen(narrow(widen(0xbf81) / 1.25)) * widen(0x3fc0));
    assert_ne!(task.output[0], wrong);
}

#[test]
fn first_halfway_rounds_up_to_even_and_differs_from_unchanged_fused_macro() {
    let mut task = Task::new(0x3f81, 0x3fc0, 0);
    let math = Math::new(f32::from_bits(0x3eaaaaab));
    run(&mut task, &math);
    let (_, first, result) = staged(task.gate, task.up, math.exponential);
    assert_eq!((first, result), (0x3f42, 0x3f92));
    let mut old = Task::new(task.gate, task.up, 0);
    let old_math = Math::new(math.exponential);
    qwen_claimed_mlp_swiglu_v1!(old, old_math);
    assert!(old.valid);
    assert_eq!(old.output, vec![0x3f91; 96]);
    assert_eq!(task.output, vec![0x3f92; 96]);
    assert_eq!(*math.inputs.borrow(), *old_math.inputs.borrow());
}

#[test]
fn first_halfway_rounds_down_to_even() {
    let mut task = Task::new(0x3f83, 0x3f80, 1);
    let math = Math::new(f32::from_bits(0x3eaaaaab));
    run(&mut task, &math);
    assert_eq!(staged(task.gate, task.up, math.exponential).1, 0x3f44);
    assert_eq!(task.output, vec![0x3f44; 96]);
}

#[test]
fn final_product_keeps_its_own_bf16_ties_even_rounding() {
    for (gate, expected) in [(0x3f81, 0x3fc2), (0x3f83, 0x3fc4)] {
        let mut task = Task::new(gate, 0x3fc0, 0);
        let math = Math::new(0.0);
        run(&mut task, &math);
        assert_eq!(task.output, vec![expected; 96]);
    }
}

#[test]
fn signed_zero_survives_both_materializations() {
    for gate in [0, 0x8000] {
        for up in [0, 0x8000, 0x3f80, 0xbf80] {
            let mut task = Task::new(gate, up, 0);
            let math = Math::new(1.0);
            run(&mut task, &math);
            assert_eq!(task.output, vec![(gate ^ up) & 0x8000; 96]);
        }
    }
}

#[test]
fn tiny_silu_rounds_at_bf16_boundary_before_amplifying_up() {
    for (gate, expected) in [(1, 0), (3, 4), (0x8001, 0x8000)] {
        let mut task = Task::new(gate, 0x4000, 0);
        let math = Math::new(1.0);
        run(&mut task, &math);
        assert_eq!(task.output, vec![expected; 96]);
    }
}

#[test]
fn nonfinite_gate_rejects_without_shortening_components() {
    for gate in [0x7f80, 0xff80, 0x7fc0] { rejected(gate, 0x3f80, 1.0); }
}

#[test]
fn nonfinite_up_rejects_even_when_gate_is_zero() {
    for up in [0x7f80, 0xff80, 0x7fc0] { rejected(0, up, 1.0); }
}

#[test]
fn nonfinite_exp_rejects_without_committing_zero() {
    for exponential in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        rejected(0, 0, exponential);
    }
}

#[test]
fn fp32_silu_overflow_stays_rejected() {
    rejected(0x7f7f, 0, -0.5);
}

#[test]
fn materialized_silu_overflow_rejects_before_a_small_up_can_hide_it() {
    // Adversarial finite Math return, not a claim about valid OCML exp outputs.
    let g = 0x7f7e;
    let e = -1.0_f32 / 129.0;
    let silu = widen(g) * (1.0 / (1.0 + e));
    assert!(silu.is_finite());
    assert_eq!(fe2o3_device::Bf16::from_f32(silu).to_bits(), 0x7f80);
    let mut old = Task::new(g, 0x0d80, 0);
    let old_math = Math::new(e);
    qwen_claimed_mlp_swiglu_v1!(old, old_math);
    assert!(old.valid && old.output.len() == 96);
    rejected(g, 0x0d80, e);
}

#[test]
fn final_product_overflow_remains_rejected() {
    rejected(0x7f7f, 0x4000, 0.0);
}

#[test]
fn missing_inputs_keep_sticky_rejection_and_complete_read_census() {
    for gate in [false, true] {
        let mut task = Task::new(0x3f80, 0x3f80, 31);
        task.missing_gate = gate;
        task.missing_up = !gate;
        let math = Math::new(1.0);
        run(&mut task, &math);
        census(&task, &math);
        assert!(!task.valid && task.output.is_empty());
        assert_eq!(task.write_attempts, 0);
    }
}

#[test]
fn failed_writes_preserve_prefix_and_poison_all_remaining_components() {
    for component in [0, 1, 95] {
        let mut task = Task::new(0x3f80, 0x3f80, 7);
        task.fail_write = Some(component);
        let math = Math::new(1.0);
        run(&mut task, &math);
        census(&task, &math);
        assert!(!task.valid);
        assert_eq!(task.output, vec![0x3f00; component]);
        assert_eq!(task.write_attempts, component + 1);
        assert_eq!(task.reject_calls, 96 - component);
    }
}

#[test]
fn complete_candidate_entry_changes_only_selected_swiglu_and_one_include() {
    let candidate = syn::parse_file(include_str!("../src/finite_mlp_tiles_silu_materialized_v1.rs")).unwrap();
    let baseline = std::env::var("FE2O3_SILU_BASELINE_ENTRY")
        .expect("root must supply the pinned actual Down2 fixture/src/lib.rs");
    let baseline = syn::parse_file(&std::fs::read_to_string(baseline).unwrap()).unwrap();
    let functions = |source: &syn::File| source.items.iter().filter_map(|item| {
        if let syn::Item::Fn(value) = item {
            Some((value.sig.ident.to_string(), value.to_token_stream().to_string()))
        } else { None }
    }).collect::<std::collections::BTreeMap<_, _>>();
    let mut old = functions(&baseline);
    let mut new = functions(&candidate);
    assert_eq!(old.len(), 9);
    assert!(old.remove("swiglu").unwrap().contains("qwen_claimed_mlp_swiglu_v1"));
    assert!(new.remove("swiglu").unwrap().contains("qwen_claimed_mlp_swiglu_materialized_v1"));
    assert_eq!(old, new);
    let includes = |source: &syn::File| source.items.iter().filter_map(|item| {
        if let syn::Item::Macro(value) = item {
            assert!(value.mac.path.is_ident("include"));
            Some(syn::parse2::<syn::LitStr>(value.mac.tokens.clone()).unwrap().value())
        } else { None }
    }).collect::<Vec<_>>();
    let mut expected = includes(&baseline);
    expected.push("mlp_silu_materialized_numerics_v1.rs".into());
    assert_eq!(includes(&candidate), expected);
}
