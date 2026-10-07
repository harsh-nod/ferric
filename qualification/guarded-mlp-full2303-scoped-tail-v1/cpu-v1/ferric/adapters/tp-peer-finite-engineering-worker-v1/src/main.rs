//! Closed engineering CLI. Legacy wire-check mode remains refusal-only.

use ferric_tp_peer_finite_engineering_worker_v1::finite_composition_wire as wire;
use ferric_tp_peer_finite_engineering_worker_v1::native_cli_v1::{self as native, Invocation};
use ferric_tp_peer_finite_engineering_worker_v1::native_guarded_mlp_decode_cli_v1 as guarded_decode;
use ferric_tp_peer_finite_engineering_worker_v1::native_guarded_mlp_full2303_cli_v1 as full2303;
use ferric_tp_peer_finite_engineering_worker_v1::native_guarded_mlp_readiness_cli_v1 as readiness;
use ferric_tp_peer_finite_engineering_worker_v1::native_long_cli_v1 as long;
use ferric_tp_peer_finite_engineering_worker_v1::native_mlp_tiles_comparison_cli_v1 as tiles;
use ferric_tp_peer_finite_engineering_worker_v1::native_prefix_decode_cli_v1 as prefix_decode;
use ferric_tp_peer_finite_engineering_worker_v1::native_prefix_decode_device_clock_v2 as prefix_clocks;
use ferric_tp_peer_finite_engineering_worker_v1::native_prefix_decode_device_v1 as prefix_device;
use ferric_tp_peer_finite_engineering_worker_v1::native_prefix_decode_host_v1 as prefix_host;
use ferric_tp_peer_finite_engineering_worker_v1::native_prefix_decode_host_v2 as prefix_host_policy;
use ferric_tp_peer_finite_engineering_worker_v1::native_prefix_layer_cli_v1 as layer;
use ferric_tp_peer_finite_engineering_worker_v1::native_projection_residual_decode_cli_v1 as projection_decode;
use ferric_tp_peer_finite_engineering_worker_v1::native_projection_residual_decode_host_v1 as projection_host;
use ferric_tp_peer_finite_engineering_worker_v1::native_projection_residual_layer_cli_v1 as projection_layer;
use ferric_tp_peer_finite_engineering_worker_v1::native_projection_residual_mlp_ordered_v1 as ordered;
use ferric_tp_peer_finite_engineering_worker_v1::native_queued_mlp_comparison_cli_v1 as comparison;
use ferric_tp_peer_finite_engineering_worker_v1::native_queued_projection_comparison_cli_v1 as projection;
use ferric_tp_peer_finite_engineering_worker_v1::native_rearm_smoke_cli_v1 as smoke;
use ferric_tp_peer_finite_engineering_worker_v1::native_tiles_decode_cli_v1 as decode;
use std::io::{self, IsTerminal};

#[allow(unsafe_code)]
fn run() -> io::Result<i32> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let ordered_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-projection-residual-mlp-ordered-v1")
    {
        Some(ordered::parse_args(&args)?)
    } else {
        None
    };
    let projection_shared_host_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-projection-residual-decode-shared-host-v1")
    {
        Some(projection_host::parse_shared_args(&args)?)
    } else {
        None
    };
    let projection_host_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-projection-residual-decode-host-v1")
    {
        Some(projection_host::parse_args(&args)?)
    } else {
        None
    };
    let full2303_bank_census_tail_options =
        if args.first().and_then(|s| s.to_str()) == Some(full2303::BANK_CENSUS_TAIL_FLAG) {
            Some(full2303::parse_bank_scoped_census_tail_args(&args)?)
        } else {
            None
        };
    let full2303_bank_census_options =
        if args.first().and_then(|s| s.to_str()) == Some(full2303::BANK_CENSUS_FLAG) {
            Some(full2303::parse_bank_scoped_census_args(&args)?)
        } else {
            None
        };
    let full2303_scoped_options =
        if args.first().and_then(|s| s.to_str()) == Some(full2303::SCOPED_FLAG) {
            Some(full2303::parse_scoped_args(&args)?)
        } else {
            None
        };
    let full2303_options = if args.first().and_then(|s| s.to_str()) == Some(full2303::FLAG) {
        Some(full2303::parse_args(&args)?)
    } else {
        None
    };
    let readiness_tail_scoped_options =
        if args.first().and_then(|s| s.to_str()) == Some(readiness::TAIL_SCOPED_FLAG) {
            Some(readiness::parse_tail_scoped_args(&args)?)
        } else {
            None
        };
    let readiness_census_scoped_options =
        if args.first().and_then(|s| s.to_str()) == Some(readiness::CENSUS_SCOPED_FLAG) {
            Some(readiness::parse_census_scoped_args(&args)?)
        } else {
            None
        };
    let readiness_bank_scoped_options =
        if args.first().and_then(|s| s.to_str()) == Some(readiness::BANK_SCOPED_FLAG) {
            Some(readiness::parse_bank_scoped_args(&args)?)
        } else {
            None
        };
    let readiness_scoped_options =
        if args.first().and_then(|s| s.to_str()) == Some(readiness::SCOPED_FLAG) {
            Some(readiness::parse_scoped_args(&args)?)
        } else {
            None
        };
    let readiness_shared_options =
        if args.first().and_then(|s| s.to_str()) == Some(readiness::SHARED_FLAG) {
            Some(readiness::parse_shared_args(&args)?)
        } else {
            None
        };
    let readiness_causal_options =
        if args.first().and_then(|s| s.to_str()) == Some(readiness::CAUSAL_FLAG) {
            Some(readiness::parse_causal_args(&args)?)
        } else {
            None
        };
    let readiness_position5_options =
        if args.first().and_then(|a| a.to_str()) == Some(readiness::POSITION5_FLAG) {
            Some(readiness::parse_position5_args(&args)?)
        } else {
            None
        };
    let readiness_options = if args.first().and_then(|a| a.to_str()) == Some(readiness::FLAG) {
        Some(readiness::parse_args(&args)?)
    } else {
        None
    };
    let guarded_decode_options =
        if args.first().and_then(|a| a.to_str()) == Some(guarded_decode::FLAG) {
            Some(guarded_decode::parse_args(&args)?)
        } else {
            None
        };
    let guarded_capture_options =
        if args.first().and_then(|a| a.to_str()) == Some(guarded_decode::CAPTURE_FLAG) {
            Some(guarded_decode::parse_capture_args(&args)?)
        } else {
            None
        };
    let guarded_host_options =
        if args.first().and_then(|a| a.to_str()) == Some(guarded_decode::HOST_FLAG) {
            Some(guarded_decode::parse_host_args(&args)?)
        } else {
            None
        };
    let guarded_host_shared_options =
        if args.first().and_then(|a| a.to_str()) == Some(guarded_decode::HOST_SHARED_FLAG) {
            Some(guarded_decode::parse_host_shared_args(&args)?)
        } else {
            None
        };
    let guarded_paired_terminal_options =
        if args.first().and_then(|a| a.to_str()) == Some(guarded_decode::PAIRED_TERMINAL_FLAG) {
            Some(guarded_decode::parse_paired_terminal_args(&args)?)
        } else {
            None
        };
    let guarded_reuse_options =
        if args.first().and_then(|a| a.to_str()) == Some(guarded_decode::REUSE_FLAG) {
            Some(guarded_decode::parse_reuse_args(&args)?)
        } else {
            None
        };
    let guarded_host_paired_read_options =
        if args.first().and_then(|a| a.to_str()) == Some(guarded_decode::HOST_PAIRED_READ_FLAG) {
            Some(guarded_decode::parse_host_paired_read_args(&args)?)
        } else {
            None
        };
    let projection_decode_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-projection-residual-decode-v1")
    {
        Some(projection_decode::parse_args(&args)?)
    } else {
        None
    };
    let projection_layer_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-projection-residual-layer-v1")
    {
        Some(projection_layer::parse_args(&args)?)
    } else {
        None
    };
    let clock_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-prefix-decode-device-clock-v2")
    {
        Some(prefix_clocks::parse_args(&args)?)
    } else {
        None
    };
    let device_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-prefix-decode-device-v1")
    {
        Some(prefix_device::parse_args(&args)?)
    } else {
        None
    };
    let host_policy_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-prefix-decode-host-v2")
    {
        Some(prefix_host_policy::parse_args(&args)?)
    } else {
        None
    };
    let host_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-prefix-decode-host-v1")
    {
        Some(prefix_host::parse_args(&args)?)
    } else {
        None
    };
    let prefix_decode_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-prefix-decode-v1")
    {
        Some(prefix_decode::parse_args(&args)?)
    } else {
        None
    };
    let layer_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-prefix-layer-v1")
    {
        Some(layer::parse_args(&args)?)
    } else {
        None
    };
    let decode_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-tiles-decode-v1")
    {
        Some(decode::parse_args(&args)?)
    } else {
        None
    };
    let tiles_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-mlp-tiles-comparison-v1")
    {
        Some(tiles::parse_args(&args)?)
    } else {
        None
    };
    let projection_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-queued-projection-comparison-v1")
    {
        Some(projection::parse_args(&args)?)
    } else {
        None
    };
    let comparison_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-queued-mlp-comparison-v1")
    {
        Some(comparison::parse_args(&args)?)
    } else {
        None
    };
    let smoke_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-rearm-smoke-v1")
    {
        Some(smoke::parse_args(&args)?)
    } else {
        None
    };
    let long_options = if args
        .first()
        .is_some_and(|arg| arg == "--engineering-native-long-v1")
    {
        Some(long::parse_args(&args)?)
    } else {
        None
    };
    let invocation = if full2303_options.is_none()
        && full2303_scoped_options.is_none()
        && full2303_bank_census_options.is_none()
        && full2303_bank_census_tail_options.is_none()
        && readiness_scoped_options.is_none()
        && readiness_bank_scoped_options.is_none()
        && readiness_census_scoped_options.is_none()
        && readiness_tail_scoped_options.is_none()
        && readiness_shared_options.is_none()
        && readiness_causal_options.is_none()
        && readiness_position5_options.is_none()
        && readiness_options.is_none()
        && long_options.is_none()
        && projection_layer_options.is_none()
        && guarded_decode_options.is_none()
        && guarded_capture_options.is_none()
        && guarded_host_options.is_none()
        && guarded_host_shared_options.is_none()
        && guarded_host_paired_read_options.is_none()
        && guarded_reuse_options.is_none()
        && guarded_paired_terminal_options.is_none()
        && projection_decode_options.is_none()
        && projection_host_options.is_none()
        && projection_shared_host_options.is_none()
        && ordered_options.is_none()
        && device_options.is_none()
        && clock_options.is_none()
        && host_policy_options.is_none()
        && host_options.is_none()
        && smoke_options.is_none()
        && comparison_options.is_none()
        && projection_options.is_none()
        && tiles_options.is_none()
        && decode_options.is_none()
        && layer_options.is_none()
        && prefix_decode_options.is_none()
    {
        Some(native::parse_args(&args)?)
    } else {
        None
    };
    if io::stdout().is_terminal() || std::fs::read_dir("/proc/self/task")?.count() != 1 {
        return Err(io::Error::other(
            "wire checker requires captured stdout and a single-threaded process",
        ));
    }
    let mut input = io::stdin().lock();
    if let Some(options) = ordered_options {
        // SAFETY: distinct opt-in authenticated ordered bootstrap; no fallback.
        unsafe { ordered::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = projection_shared_host_options {
        // SAFETY: explicit shared-full route, unchanged owned AR4 lifetime.
        unsafe { projection_host::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = projection_host_options {
        // SAFETY: explicit observational route, unchanged owned AR4 lifetime.
        unsafe { projection_host::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = full2303_bank_census_tail_options {
        // SAFETY: this explicit mode requires the trusted parent's original
        // authenticated Full request, owned process group and fixed deadline.
        unsafe {
            full2303::run_native_bank_scoped_census_tail(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = full2303_bank_census_options {
        // SAFETY: this explicit mode requires the trusted parent's original
        // authenticated Full request, owned process group and fixed deadline.
        unsafe {
            full2303::run_native_bank_scoped_census(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = full2303_scoped_options {
        // SAFETY: explicit separately owned Full route; no policy activation on defaults.
        unsafe {
            full2303::run_native_scoped(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = full2303_options {
        // SAFETY: explicit full profile; the authenticated parent owns bounded retirement.
        unsafe { full2303::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = readiness_tail_scoped_options {
        // SAFETY: separately authenticated bank/layer/census/tail scoped Position5 route.
        unsafe {
            readiness::run_native_tail_scoped(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = readiness_census_scoped_options {
        // SAFETY: separately authenticated bank-and-layer scoped Position5 route.
        unsafe {
            readiness::run_native_census_scoped(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = readiness_bank_scoped_options {
        // SAFETY: separately authenticated bank-and-layer scoped Position5 route.
        unsafe {
            readiness::run_native_bank_scoped(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = readiness_scoped_options {
        // SAFETY: explicit scoped-warm route retains the owned readiness lifetime.
        unsafe {
            readiness::run_native_scoped(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = readiness_shared_options {
        // SAFETY: explicit shared-full policy, unchanged owned Position5 lifetime.
        unsafe {
            readiness::run_native_shared(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = readiness_causal_options {
        // SAFETY: diagnostic-only explicit flag; unchanged owned readiness lifetime.
        unsafe {
            readiness::run_native_causal(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = readiness_position5_options {
        // SAFETY: separate closed diagnostic profile, unchanged owned lifetime.
        unsafe { readiness::run_native_position5(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = readiness_options {
        // The distinct entry consumes only the closed Readiness40 owner.
        unsafe { readiness::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = guarded_decode_options {
        unsafe { guarded_decode::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = guarded_capture_options {
        // SAFETY: explicit diagnostic selection preserves the same owned GPU lifetime.
        unsafe {
            guarded_decode::run_native_capture(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = guarded_host_options {
        // SAFETY: same explicit owned diagnostic lifetime, original runtime policy.
        unsafe {
            guarded_decode::run_native_host(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = guarded_host_shared_options {
        // SAFETY: explicit shared-full mode preserves the owned GPU lifetime.
        unsafe {
            guarded_decode::run_native_host_shared(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = guarded_host_paired_read_options {
        // SAFETY: explicit fresh AR4 shared-full route, authenticated inputs and
        // the same owned child lifetime; runtime pair reads retain both fences.
        unsafe {
            guarded_decode::run_native_host_paired_read(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = guarded_paired_terminal_options {
        // SAFETY: explicit authenticated reusable AR4 selection; the runtime
        // retains actual retired-proof, ownership and terminal validation.
        unsafe {
            guarded_decode::run_native_paired_terminal(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = guarded_reuse_options {
        // SAFETY: explicit authenticated AR4 reuse profile, owned child and
        // private runtime retirement checks. No policy or fresh fallback.
        unsafe {
            guarded_decode::run_native_reuse(
                options,
                &mut input,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            )
        }?;
        return Ok(0);
    }
    if let Some(options) = projection_decode_options {
        // SAFETY: separate reviewed TF4 arithmetic route and owned disposable child.
        unsafe { projection_decode::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = projection_layer_options {
        // SAFETY: separate reviewed candidate with the same disposable-child lifetime.
        unsafe { projection_layer::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = clock_options {
        // SAFETY: distinct trusted-parent raw-clock diagnostic and owned child.
        unsafe { prefix_clocks::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = device_options {
        // SAFETY: explicit trusted-parent diagnostic with fresh raw-enabled queues.
        unsafe { prefix_device::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = host_policy_options {
        // SAFETY: closed explicit host policy; same owned Four native lifetime.
        unsafe { prefix_host_policy::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = host_options {
        // SAFETY: explicit trusted-parent diagnostic; policy and Four wire unchanged.
        unsafe { prefix_host::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = prefix_decode_options {
        // SAFETY: distinct trusted-parent all-layer profile. A native failure
        // exits this disposable child; the owner must reap and audit afterward.
        unsafe { prefix_decode::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = layer_options {
        // SAFETY: distinct one-layer trusted-parent mode; errors exit this
        // disposable process and require owned-parent reap and post-idle audit.
        unsafe { layer::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = decode_options {
        // SAFETY: distinct trusted-parent all-layer typed548 engineering profile.
        unsafe { decode::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = tiles_options {
        // SAFETY: distinct explicit trusted-parent V1/V2 diagnostic profile.
        unsafe { tiles::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = projection_options {
        // SAFETY: separate explicit same-owner projection comparison with
        // captured single-threaded trusted-parent lifetime and fatal teardown.
        unsafe { projection::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = comparison_options {
        // SAFETY: distinct explicit diagnostic profile, captured single-threaded
        // trusted-parent custody and unchanged owned deadline/teardown contract.
        unsafe { comparison::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = smoke_options {
        // SAFETY: distinct explicit four-forward trusted-parent engineering
        // profile with captured single-threaded ownership and fatal teardown.
        unsafe { smoke::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(options) = long_options {
        // SAFETY: separate explicit profile, same captured single-threaded
        // trusted-parent lifetime contract; never selected by the old CLI.
        unsafe { long::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    if let Some(Invocation::Native(options)) = invocation {
        // SAFETY: explicit opt-in, captured single-threaded child and trusted
        // owning-parent custody. Parent enforces deadline and kill/reap. This
        // boundary is engineering native code, not production admission.
        unsafe { native::run_native(options, &mut input, &mut io::stdout().lock()) }?;
        return Ok(0);
    }
    let Some((request, payload)) = wire::read_request(&mut input)? else {
        return Err(io::Error::other("missing finite request"));
    };
    let response = wire::refuse_unbound_request(&request, &payload)?;
    wire::write_response(&mut io::stdout().lock(), &response)?;
    Ok(2)
}

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("finite engineering worker: {error}");
            2
        }
    };
    std::process::exit(code);
}
